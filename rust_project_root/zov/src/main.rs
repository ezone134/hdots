//! zov — niri-like scrollable workspace overview for Hyprland.
//!
//! A full-size layer-shell window (keyboard/pointer grabbed) that draws a
//! vertical tape of workspaces, each workspace as a row of its window tiles.
//! Standalone: the keybind runs `zov open` to show the overview, and Escape
//! closes it in-process (the overlay holds an exclusive keyboard grab, so
//! Hyprland never sees that key).

mod config;
mod hypr;
mod render;

use config::{is_super_key, is_super_keycode, Config};
use hypr::{Hypr, KeyAction, Snapshot};
use render::{Renderer, Theme};
use smithay_client_toolkit::compositor::{CompositorHandler, CompositorState, FrameCallbackData};
use smithay_client_toolkit::delegate_registry;
use smithay_client_toolkit::output::{OutputHandler, OutputState};
use smithay_client_toolkit::registry::{ProvidesRegistryState, RegistryState};
use smithay_client_toolkit::reexports::calloop;
use smithay_client_toolkit::reexports::calloop::generic::Generic;
use smithay_client_toolkit::reexports::calloop::LoopHandle;
use smithay_client_toolkit::reexports::calloop_wayland_source::WaylandSource;
use smithay_client_toolkit::seat::keyboard::{KeyEvent, KeyboardHandler, Keysym, Modifiers, RawModifiers};
use smithay_client_toolkit::seat::pointer::{PointerEvent, PointerEventKind, PointerHandler};
use smithay_client_toolkit::seat::{Capability, SeatHandler, SeatState};
use smithay_client_toolkit::shell::wlr_layer::{
    Anchor, KeyboardInteractivity, Layer, LayerShell, LayerShellHandler, LayerSurface,
    LayerSurfaceConfigure,
};
use smithay_client_toolkit::shell::WaylandSurface;
use smithay_client_toolkit::shm::{slot::SlotPool, Shm, ShmHandler};

use wayland_client::globals::registry_queue_init;
use wayland_client::protocol::{wl_keyboard, wl_output, wl_pointer, wl_seat, wl_shm, wl_surface};
use wayland_client::{Connection, QueueHandle};

use std::io::{Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::process::{Command, Stdio};

const NS: &str = "zov";

struct App {
    conn: Connection,
    qh: QueueHandle<App>,
    loop_handle: Option<LoopHandle<'static, App>>,
    quit: Option<calloop::LoopSignal>,
    /// Set by `quit()`; the IPC handler exits the process after replying.
    quit_requested: bool,
    registry_state: RegistryState,
    compositor: CompositorState,
    layer_shell: LayerShell,
    output_state: OutputState,
    seat_state: SeatState,
    shm: Shm,

    layer: Option<LayerSurface>,
    pool: Option<SlotPool>,
    configured: (i32, i32),
    visible: bool,
    /// Overlay was asked to open but is waiting for a keyboard to map.
    want_visible: bool,

    pointer: Option<wl_pointer::WlPointer>,
    keyboard: Option<wl_keyboard::WlKeyboard>,

    hypr: Hypr,
    snap: Snapshot,
    scroll: f32,
    /// Keyboard selection cursor, as a (workspace id, tile index) pair.
    /// Pre-selected on open to the currently focused window, so the active
    /// card is highlighted right away and arrows walk from there.
    sel: Option<(i64, usize)>,
    dirty: bool,
    theme: Theme,
    renderer: Renderer,
    /// Key bindings from `~/.config/zov/config` (`[keys]`), seeded with the
    /// built-in rofi-style set for keys the user did not configure.
    keys: Config,

    ipc_listener: Option<UnixListener>,
    debug: bool,
    /// Last keyboard modifiers state. The overlay holds the keyboard grab,
    /// so it must track SUPER itself: SUPER+Left/Right = jump to the
    /// neighboring window and activate it immediately (no Enter).
    mods: Modifiers,
    /// Raw Super hold state from key press/release events. Belt-and-suspenders
    /// for `mods.logo`: some builds deliver the Modifiers event late (or not at
    /// all mid-grab), so Super+arrow must not depend on it alone.
    super_held: bool,
}

fn zlog(debug: bool, msg: &str) {
    if debug {
        eprintln!("[zov] {msg}");
    }
}

impl App {
    fn new(
        conn: Connection,
        qh: QueueHandle<App>,
        globals: wayland_client::globals::GlobalList,
    ) -> Result<Self, String> {
        let compositor = CompositorState::bind(&globals, &qh)
            .map_err(|e| format!("wl_compositor: {e}"))?;
        let layer_shell = LayerShell::bind(&globals, &qh)
            .map_err(|e| format!("wlr-layer-shell: {e}"))?;
        let shm = Shm::bind(&globals, &qh)
            .map_err(|e| format!("wl_shm: {e}"))?;
        let output_state = OutputState::new(&globals, &qh);
        let seat_state = SeatState::new(&globals, &qh);
        // Config file (expanded by hdots rconfig_gen into ~/.config/zov/config):
        // theme colors, layout geometry and key bindings. Missing file or
        // entries fall back to the built-in Theme/geometry/keymap.
        let mut cfg = Config::load();
        let mut theme = Theme::default();
        cfg.apply_theme(&mut theme);
        let renderer = Renderer::with_layout(
            cfg.font_family(),
            cfg.f32_or("gap", 14.0),
            cfg.f32_or("tile_radius", 8.0),
        );
        cfg.seed_builtins();
        let hypr = Hypr::new();

        let snap = hypr.snapshot().unwrap_or_default();

        let ipc_listener = bind_ipc().ok();

        Ok(App {
            conn,
            qh,
            loop_handle: None,
            quit: None,
            quit_requested: false,
            registry_state: RegistryState::new(&globals),
            compositor,
            layer_shell,
            output_state,
            seat_state,
            shm,
            layer: None,
            pool: None,
            configured: (0, 0),
            visible: false,
            want_visible: false,
            pointer: None,
            keyboard: None,
            hypr,
            snap,
            scroll: 0.0,
            sel: None,
            dirty: true,
            theme,
            renderer,
            keys: cfg,
            ipc_listener,
            debug: std::env::var("ZOV_DEBUG").ok().as_deref() == Some("1"),
            mods: Modifiers::default(),
            super_held: false,
        })
    }

    fn create_layer(&mut self) {
        if self.layer.is_some() {
            return;
        }
        let surface = self.compositor.create_surface(&self.qh);
        let layer = self
            .layer_shell
            .create_layer_surface(&self.qh, surface, Layer::Overlay, Some(NS), None);
        layer.set_anchor(Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT);
        layer.set_keyboard_interactivity(KeyboardInteractivity::Exclusive);
        layer.commit();
        self.layer = Some(layer);
        self.visible = true;
        let _ = self.conn.flush();
    }

    /// Destroy the layer surface. The compositor drops the exclusive keyboard
    /// grab with it, so input goes back to the focused client.
    fn destroy_layer(&mut self) {
        self.layer = None;
        self.visible = false;
        self.dirty = false;
        let _ = self.conn.flush();
    }

    fn show(&mut self) {
        if self.visible {
            return;
        }
        // Startup race guard: occasionally the first IPC query times out and
        // returns nothing; retry briefly so the tape (and the pre-selection
        // below) don't come up empty.
        for _ in 0..3 {
            if let Some(s) = self.hypr.snapshot() {
                if !s.workspaces.is_empty() {
                    self.snap = s;
                    break;
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(60));
        }
        self.scroll = 0.0;
        self.sel = None;
        // Open with the focused window's card highlighted; set_sel also pans
        // its row into view. Retry briefly: focus state can lag the snapshot
        // by a frame or two right after a switch.
        for _ in 0..4 {
            if let Some(card) = self.focused_card() {
                self.set_sel(card.0, card.1);
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(40));
        }
        // Defer mapping until the seat has a keyboard. The compositor grants a
        // layer surface keyboard focus at map time; if the surface maps before
        // our wl_keyboard exists, the enter event is dropped and never resent,
        // leaving the overlay permanently inert. See `maybe_create_layer`.
        self.want_visible = true;
        self.maybe_create_layer();
    }

    /// Create the overlay surface, but only once a keyboard exists so the
    /// compositor can hand it focus the moment it maps.
    fn maybe_create_layer(&mut self) {
        if !self.want_visible || self.visible || self.layer.is_some() {
            return;
        }
        if self.keyboard.is_none() {
            zlog(self.debug, "defer layer creation until keyboard arrives");
            return;
        }
        self.create_layer();
        self.dirty = true;
    }

    /// Tear the whole daemon down. Called when Escape dismisses the overlay so
    /// the process exits instead of lingering hidden and holding the lock.
    ///
    /// This only *requests* the exit: any in-flight IPC reply is still written
    /// by the handler before the process actually leaves (see `quit_requested`).
    fn quit(&mut self) {
        zlog(self.debug, "quit()");
        self.want_visible = false;
        self.quit_requested = true;
        self.destroy_layer();
        if let Some(sig) = self.quit.as_ref() {
            sig.stop();
            sig.wakeup();
        }
    }

    /// Actually leave the process. Called once no reply is owed to anybody.
    fn exit_now(&self) -> ! {
        let _ = self.conn.flush();
        let _ = std::fs::remove_file(ipc_socket_path());
        std::process::exit(0);
    }

    fn hide(&mut self) {
        zlog(self.debug, &format!("hide() visible={}", self.visible));
        self.want_visible = false;
        if !self.visible {
            return;
        }
        self.destroy_layer();
    }

    fn draw(&mut self, qh: &QueueHandle<Self>) {
        if !self.visible {
            self.dirty = false;
            return;
        }
        let (w, h) = self.configured;
        if w <= 0 || h <= 0 || self.layer.is_none() {
            zlog(self.debug, &format!("draw skipped: {w}x{h} layer={}", self.layer.is_some()));
            return;
        }

        let stride = w * 4;

        // Ensure pool is large enough
        let needed = (stride * h) as usize;
        if self.pool.is_none() || self.pool.as_ref().map_or(0, |p| p.len()) < needed {
            self.pool = Some(SlotPool::new(needed.max(256 * 256 * 4), &self.shm).unwrap());
        }

        let pool = match self.pool.as_mut() {
            Some(p) => p,
            None => return,
        };

        let (buffer, canvas) = pool
            .create_buffer(w, h, stride, wl_shm::Format::Argb8888)
            .expect("failed to create buffer");

        // Render the overview onto the canvas via tiny-skia
        {
            let mut px = tiny_skia::Pixmap::new(w as u32, h as u32).unwrap();
            self.renderer
                .draw(&mut px, &self.snap, &self.theme, self.scroll, self.sel);
            // Copy premultiplied RGBA from pixmap into SHM canvas (both are ARGB8888)
            canvas.copy_from_slice(px.data());
        }

        // Damage the entire surface
        let layer = self.layer.as_ref().unwrap();
        layer.wl_surface().damage_buffer(0, 0, w, h);

        // Request next frame callback
        layer.wl_surface().frame(qh, FrameCallbackData(layer.wl_surface().clone()));

        // Attach and commit
        buffer.attach_to(layer.wl_surface()).expect("buffer attach");
        layer.commit();
        let _ = self.conn.flush();
        self.dirty = false;
    }

    fn hit_test(&self, x: f64, y: f64) -> Option<(i64, usize)> {
        let (vw, vh) = (self.configured.0 as f32, self.configured.1 as f32);
        if vw <= 0.0 || vh <= 0.0 {
            return None;
        }
        let lay = self.renderer.layout(&self.snap, vw, vh);
        let scroll = self.scroll.clamp(0.0, (lay.content_h - vh + 60.0).max(0.0));
        let offset = if lay.content_h <= vh {
            (vh - lay.content_h) / 2.0
        } else {
            0.0
        };
        for row in &lay.rows {
            let ry = row.y + offset - scroll;
            let row_has_wins = self
                .snap
                .workspaces
                .iter()
                .find(|w| w.id == row.ws_id)
                .map(|w| !w.wins.is_empty())
                .unwrap_or(false);
            if !row_has_wins {
                continue;
            }
            for (idx, tile) in row.tiles.iter().enumerate() {
                if x >= tile.x as f64
                    && x <= (tile.x + tile.w) as f64
                    && y >= ry as f64
                    && y <= (ry + tile.h) as f64
                {
                    return Some((row.ws_id, idx));
                }
            }
        }
        None
    }

    /// Rows that can be selected, in visual order: only workspaces that
    /// actually have windows, each paired with its window count.
    ///
    /// Empty workspaces are skipped so the cursor never lands on a placeholder
    /// tile (those have no window behind them and cannot be activated).
    fn selectable_rows(&self) -> Vec<(i64, usize)> {
        self.snap
            .workspaces
            .iter()
            .filter(|w| !w.wins.is_empty())
            .map(|w| (w.id, w.wins.len()))
            .collect()
    }

    /// Card (workspace id, tile index) of the currently focused window.
    ///
    /// Resolved by address via a direct `activewindow` query — focusHistoryID
    /// is a global MRU rank and transiently has no zero entry mid-switch, so
    /// it is only a fallback. Retries briefly because the compositor can
    /// report no active window for a few frames during a workspace switch.
    /// Card (workspace id, tile index) of the currently focused window.
    ///
    /// Resolved by address via a direct `activewindow` query, matched only on
    /// the snapshot's active workspace; falls back to the workspace's MRU
    /// (focusHistoryID == 0) window. Single attempt — callers retry, because
    /// both the IPC queries and the compositor's own focus state can lag a
    /// frame or two during a workspace switch.
    fn focused_card(&self) -> Option<(i64, usize)> {
        if let Some(addr) = self.hypr.active_window_addr() {
            for ws in &self.snap.workspaces {
                if ws.id == self.snap.active_id {
                    if let Some(i) = ws.wins.iter().position(|w| w.addr == addr) {
                        return Some((ws.id, i));
                    }
                }
            }
        }
        // Fallback: active workspace's window with MRU rank 0.
        let ws = self.snap.workspaces.iter().find(|w| w.active)?;
        let idx = ws.wins.iter().position(|w| w.focus_history == 0)?;
        Some((ws.id, idx))
    }

    /// Move the selection by `delta` tiles within its row, clamped at both
    /// ends. Left/Right never cross workspace rows: Right stops at the last
    /// card of the same row, Left at the first. Row changes are Up/Down's job
    /// (which also clamp, never cycle).
    fn move_sel(&mut self, delta: i32) {
        let rows = self.selectable_rows();
        if rows.is_empty() {
            return;
        }
        if self.sel.is_none() {
            // Seed at an end of the first row: Right picks its first card,
            // Left its last.
            let c = if delta < 0 { rows[0].1 - 1 } else { 0 };
            self.set_sel(rows[0].0, c);
            return;
        }
        let (r, c) = self.sel_position(&rows);
        let count = rows[r].1 as i32;
        let next = (c as i32 + delta).clamp(0, count - 1);
        self.set_sel(rows[r].0, next as usize);
    }

    /// Move the selection by `delta` rows, keeping the column where possible.
    /// Clamped at the first/last row — never cycles around the tape.
    fn move_row(&mut self, delta: i32) {
        let rows = self.selectable_rows();
        if rows.is_empty() {
            return;
        }
        let (r, c) = self.sel_position(&rows);
        let next = (r as i32 + delta).clamp(0, rows.len() as i32 - 1) as usize;
        // Keep the column if the target row is wide enough, else clamp to its
        // last window so the selection is always valid.
        let col = c.min(rows[next].1.saturating_sub(1));
        self.set_sel(rows[next].0, col);
    }

    /// Current (row, column) for the selection, repairing it if the selected
    /// window has since closed.
    fn sel_position(&self, rows: &[(i64, usize)]) -> (usize, usize) {
        let Some((ws_id, idx)) = self.sel else {
            return (0, 0);
        };
        match rows.iter().position(|(id, count)| *id == ws_id && *count > idx) {
            Some(r) => {
                let c = rows[r].1;
                (r, idx.min(c.saturating_sub(1)))
            }
            // Row vanished or the window closed: fall back to the first tile.
            None => (0, 0),
        }
    }

    fn set_sel(&mut self, ws_id: i64, idx: usize) {
        self.sel = Some((ws_id, idx));
        zlog(
            self.debug,
            &format!("sel -> ws={ws_id} idx={idx}"),
        );
        self.scroll_sel_into_view();
        self.dirty = true;
    }

    /// Pan the tape so the selected row is on screen.
    fn scroll_sel_into_view(&mut self) {
        let Some((ws_id, _)) = self.sel else { return };
        let (vw, vh) = (self.configured.0 as f32, self.configured.1 as f32);
        if vw <= 0.0 || vh <= 0.0 {
            return;
        }
        let lay = self.renderer.layout(&self.snap, vw, vh);
        let Some(row) = lay.rows.iter().find(|r| r.ws_id == ws_id) else { return };
        let top = row.y;
        let bottom = row.y + lay.row_h;
        let max_scroll = (lay.content_h - vh + 60.0).max(0.0);
        if top < self.scroll {
            self.scroll = top;
        } else if bottom > self.scroll + vh {
            self.scroll = bottom - vh;
        }
        self.scroll = self.scroll.clamp(0.0, max_scroll);
    }

    /// Focus the selected window and dismiss. No-op if nothing is selected.
    fn activate(&mut self) {
        let Some(sel) = self.sel else {
            return;
        };
        self.select(sel.0, sel.1);
    }

    fn select(&mut self, ws_id: i64, idx: usize) {
        let win = self
            .snap
            .workspaces
            .iter()
            .find(|w| w.id == ws_id)
            .and_then(|w| w.wins.get(idx))
            .map(|w| w.addr.clone());
        // Hide BEFORE focusing: destroying the layer surface makes Hyprland
        // revert keyboard focus to the previously focused window. That revert
        // races our focus dispatch — last one wins — so the real focus is
        // dispatched from a short timer, after the grab is fully gone and the
        // revert has happened. The timer also stops the event loop, which is
        // the daemon's exit for activation paths.
        self.hide();
        let Some(addr) = win else { return };
        let Some(lh) = self.loop_handle.clone() else {
            // No loop available (shouldn't happen): best effort, focus now.
            self.hypr.switch_workspace(ws_id);
            self.hypr.focus_window(&addr);
            self.quit();
            return;
        };
        if let Err(e) = lh.insert_source(
            calloop::timer::Timer::from_duration(std::time::Duration::from_millis(120)),
            move |_, _, app: &mut App| {
                app.hypr.switch_workspace(ws_id);
                app.hypr.focus_window(&addr);
                // Force exit: the IPC handler's accept loop can block the
                // event loop from unwinding, so quit()'s graceful stop is
                // not guaranteed to end the process.
                app.exit_now();
            },
        ) {
            zlog(self.debug, &format!("activation timer insert failed: {e}"));
        }
    }

    fn maybe_render(&mut self) {
        if self.dirty {
            self.draw(&self.qh.clone());
        }
    }

    /// Config-driven key resolution with the builtin fallback.
    ///
    /// Super+Left/Right ALWAYS keep their jump semantics (move, then activate
    /// in `press_key`): with Super held they resolve as the plain movement,
    /// regardless of what `[keys]` maps them to — a config entry, or its
    /// absence, cannot break the jump.
    fn resolve_key(&self, keysym: Keysym) -> KeyAction {
        if !self.visible {
            return KeyAction::Ignore;
        }
        if self.super_active() && matches!(keysym, Keysym::Left | Keysym::Right) {
            return self
                .keys
                .resolve(keysym, false)
                .map(Into::into)
                .unwrap_or_else(|| KeyAction::resolve(keysym, true));
        }
        if let Some(a) = self.keys.resolve(keysym, self.mods.logo) {
            return a.into();
        }
        KeyAction::resolve(keysym, true)
    }

    /// Super state from the compositor's Modifiers event OR the raw Super
    /// key press/release tracked in `press_key`/`release_key` — some builds
    /// deliver the Modifiers event late (or not at all) mid-grab.
    fn super_active(&self) -> bool {
        self.mods.logo || self.super_held
    }
}

// ---- Wayland delegate macros ----

delegate_registry!(App);

impl ProvidesRegistryState for App {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    smithay_client_toolkit::registry_handlers![OutputState, SeatState];
}

smithay_client_toolkit::delegate_dispatch2!(App);

// ---- Wayland trait impls ----

impl CompositorHandler for App {
    fn scale_factor_changed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: i32,
    ) {
    }
    fn transform_changed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: wl_output::Transform,
    ) {
    }
    fn frame(
        &mut self,
        _: &Connection,
        _qh: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: u32,
    ) {
        self.maybe_render();
    }
    fn surface_enter(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: &wl_output::WlOutput,
    ) {
    }
    fn surface_leave(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: &wl_output::WlOutput,
    ) {
    }
}

impl OutputHandler for App {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }
    fn new_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
    fn update_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
    fn output_destroyed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: wl_output::WlOutput,
    ) {
    }
}

impl SeatHandler for App {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seat_state
    }
    fn new_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {
        zlog(self.debug, "new_seat");
    }
    fn new_capability(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        seat: wl_seat::WlSeat,
        capability: Capability,
    ) {
        zlog(self.debug, &format!("new_capability: {capability:?}"));
        if capability == Capability::Pointer && self.pointer.is_none() {
            if let Ok(ptr) = self.seat_state.get_pointer(qh, &seat) {
                self.pointer = Some(ptr);
                zlog(self.debug, "pointer bound");
            }
        }
        if capability == Capability::Keyboard && self.keyboard.is_none() {
            if let Some(handle) = self.loop_handle.clone() {
                let cb = Box::new(
                    |app: &mut App, kbd: &wl_keyboard::WlKeyboard, event: KeyEvent| {
                        let conn = app.conn.clone();
                        let qh = app.qh.clone();
                        app.repeat_key(&conn, &qh, kbd, 0, event);
                    },
                );
                if let Ok(kbd) =
                    self.seat_state.get_keyboard_with_repeat(qh, &seat, None, handle, cb)
                {
                    self.keyboard = Some(kbd);
                    zlog(self.debug, "keyboard bound (with repeat)");
                }
            }
            if self.keyboard.is_none() {
                if let Ok(kbd) = self.seat_state.get_keyboard(qh, &seat, None) {
                    self.keyboard = Some(kbd);
                    zlog(self.debug, "keyboard bound (no repeat)");
                }
            }
            if self.keyboard.is_none() {
                zlog(self.debug, "KEYBOARD BIND FAILED");
            }
        }
        // Requests issued from inside a dispatch stay buffered until flushed. Without
        // this, the compositor never sees get_keyboard, never sends the keymap, and
        // SCTK silently drops every key event because its xkb state is never set.
        let _ = self.conn.flush();
        // The keyboard now exists, so an overlay waiting to open can finally map
        // and receive focus.
        self.maybe_create_layer();
    }
    fn remove_capability(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Pointer {
            if let Some(p) = self.pointer.take() {
                p.release();
            }
        }
        if capability == Capability::Keyboard {
            if let Some(k) = self.keyboard.take() {
                k.release();
            }
        }
    }
    fn remove_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}
}

impl LayerShellHandler for App {
    fn closed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &LayerSurface) {
        self.destroy_layer();
    }
    fn configure(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _layer: &LayerSurface,
        configure: LayerSurfaceConfigure,
        _serial: u32,
    ) {
        let (w, h) = configure.new_size;
        let w = if w > 0 { w as i32 } else { self.configured.0 };
        let h = if h > 0 { h as i32 } else { self.configured.1 };
        zlog(self.debug, &format!("configure {w}x{h}"));
        self.configured = (w, h);
        // Keep the pre-selected card's row on screen once we know the real
        // viewport size (at first open `show()` ran before any configure).
        if self.sel.is_some() {
            self.scroll_sel_into_view();
        }
        self.dirty = true;
        self.maybe_render();
    }
}

impl PointerHandler for App {
    fn pointer_frame(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _pointer: &wl_pointer::WlPointer,
        events: &[PointerEvent],
    ) {
        for e in events {
            // Focus is the discriminator here: if neither Enter ever arrives, the
            // compositor is not treating the layer surface as interactive at all,
            // and the cause is not on the key-handling side.
            match e.kind {
                PointerEventKind::Enter { .. } => {
                    zlog(self.debug, "POINTER ENTER (surface interactive)")
                }
                PointerEventKind::Leave { .. } => zlog(self.debug, "POINTER LEAVE"),
                _ => {}
            }
            match e.kind {
                PointerEventKind::Axis { vertical, .. } => {
                    if !self.visible {
                        return;
                    }
                    let delta = if vertical.discrete != 0 {
                        -vertical.discrete as f32
                    } else if vertical.value120 != 0 {
                        -vertical.value120 as f32 / 120.0
                    } else if vertical.absolute != 0.0 {
                        vertical.absolute as f32 / 30.0
                    } else {
                        0.0
                    };
                    let (vw, vh) = (self.configured.0 as f32, self.configured.1 as f32);
                    if vh > 0.0 {
                        let lay = self.renderer.layout(&self.snap, vw, vh);
                        let max = (lay.content_h - vh + 60.0).max(0.0);
                        self.scroll = (self.scroll + delta * 40.0).clamp(0.0, max);
                        self.dirty = true;
                        self.maybe_render();
                    }
                }
                PointerEventKind::Press { button, .. } if button == 272 => {
                    if !self.visible {
                        return;
                    }
                    if let Some((ws, idx)) = self.hit_test(e.position.0, e.position.1) {
                        self.select(ws, idx);
                    }
                }
                PointerEventKind::Motion { .. } => {}
                _ => {}
            }
        }
    }
}

impl KeyboardHandler for App {
    fn update_keymap(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _km: smithay_client_toolkit::seat::keyboard::Keymap<'_>,
    ) {
        zlog(self.debug, "KEYMAP RECEIVED (xkb state installed)");
    }
    fn update_repeat_info(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        info: smithay_client_toolkit::seat::keyboard::RepeatInfo,
    ) {
        zlog(self.debug, &format!("repeat info: {info:?}"));
    }
    fn enter(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: &wl_surface::WlSurface,
        _: u32,
        keys: &[u32],
        keysyms: &[Keysym],
    ) {
        // The enter event lists the keys ALREADY held at grab time. The
        // SUPER+Tab open means Super went down BEFORE this surface got the
        // keyboard — its press was never delivered to us, and a late/absent
        // Modifiers event leaves mods.logo false. Scan the list: if Super is
        // in there, the hold-from-open is real and Super+arrows must jump.
        self.super_held = keys.iter().any(|&code| is_super_keycode(code))
            || keysyms.iter().any(|&ks| is_super_key(ks));
        zlog(
            self.debug,
            &format!(
                "KEYBOARD FOCUS ENTER ({} keys held, super_held={})",
                keys.len(),
                self.super_held
            ),
        );
    }
    fn leave(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: &wl_surface::WlSurface,
        _: u32,
    ) {
        // Keyboard focus gone: the held-at-grab state is void.
        self.super_held = false;
        zlog(self.debug, "KEYBOARD FOCUS LEAVE");
    }
    fn press_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _kbd: &wl_keyboard::WlKeyboard,
        _serial: u32,
        event: KeyEvent,
    ) {
        // Track Super by raw key state too: the compositor's Modifiers event
        // can arrive late mid-grab, so mods.logo alone misses the jump binds.
        if is_super_key(event.keysym) {
            self.super_held = true;
        }
        let action = self.resolve_key(event.keysym);
        zlog(
            self.debug,
            &format!(
                "press_key: keysym={:?} visible={} logo={} super_held={} -> {action:?}",
                event.keysym, self.visible, self.mods.logo, self.super_held
            ),
        );
        match action {
            KeyAction::Ignore => {}
            KeyAction::Hide => {
                self.quit();
                // No IPC client is owed a reply here, so leave immediately.
                self.exit_now();
            }
            KeyAction::MoveSel(d) => {
                self.move_sel(d);
                if self.super_active() {
                    // SUPER+Left/Right: jump straight to the neighboring
                    // window — focus it and close the overview, no Enter
                    // needed (mirrors the normal SUPER+arrow focus binds).
                    self.activate();
                    return;
                }
                // Repaint now: the frame-callback chain is dormant between
                // redraws, so a dirty flag alone would leave the ring frozen.
                self.maybe_render();
            }
            KeyAction::MoveRow(d) => {
                self.move_row(d);
                self.maybe_render();
            }
            KeyAction::Activate => {
                // select() hides the overlay now and schedules the focus +
                // shutdown on a short timer, so the focus lands after the
                // keyboard grab is released (see select()).
                self.activate();
            }
            KeyAction::ScrollToWs(n) => {
                let (vw, vh) = (self.configured.0 as f32, self.configured.1 as f32);
                let lay = self.renderer.layout(&self.snap, vw, vh);
                if let Some(row) = lay.rows.iter().find(|r| r.ws_id == n) {
                    let max = (lay.content_h - vh + 60.0).max(0.0);
                    self.scroll = (row.y - 20.0).clamp(0.0, max);
                    self.dirty = true;
                    self.maybe_render();
                }
            }
        }
    }
    fn repeat_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _kbd: &wl_keyboard::WlKeyboard,
        _serial: u32,
        event: KeyEvent,
    ) {
        // Held keys repeat movement only (rofi-style). Everything else
        // (Escape, Enter, digits) stays single-shot so holding it can't
        // strobe the overlay or close it.
        match self.resolve_key(event.keysym) {
            KeyAction::MoveSel(d) => {
                self.move_sel(d);
                self.maybe_render();
            }
            KeyAction::MoveRow(d) => {
                self.move_row(d);
                self.maybe_render();
            }
            _ => {}
        }
    }
    fn release_key(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        event: KeyEvent,
    ) {
        if is_super_key(event.keysym) {
            self.super_held = false;
        }
    }
    fn update_modifiers(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _serial: u32,
        mods: Modifiers,
        _: RawModifiers,
        _: u32,
    ) {
        self.mods = mods;
    }
}

impl ShmHandler for App {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}

// ---- IPC ----

fn ipc_socket_path() -> PathBuf {
    std::env::var("XDG_RUNTIME_DIR")
        .map(|d| PathBuf::from(d).join("zov.sock"))
        .unwrap_or_else(|_| PathBuf::from("/tmp/zov.sock"))
}

fn bind_ipc() -> std::io::Result<UnixListener> {
    let path = ipc_socket_path();
    let _ = std::fs::remove_file(&path);
    let l = UnixListener::bind(&path)?;
    // Nonblocking: the calloop handler drains pending connections and
    // returns instead of parking in accept(). With a blocking listener the
    // handler could sit in accept() and starve the activation timer — the
    // overlay hid but focus/exit never ran (the exact bug seen in testing).
    l.set_nonblocking(true)?;
    Ok(l)
}

/// Send one IPC command to the running daemon. Returns Some(reply) on success.
fn ipc_client(cmd: &str) -> Option<String> {
    let path = ipc_socket_path();
    let mut sock = UnixStream::connect(&path).ok()?;
    sock.write_all(cmd.as_bytes()).ok()?;
    // The server reads to EOF, so half-close before waiting on the reply —
    // otherwise both sides block and the handshake deadlocks.
    sock.shutdown(std::net::Shutdown::Write).ok()?;
    let mut reply = String::new();
    sock.read_to_string(&mut reply).ok()?;
    Some(reply)
}

/// Spawn the daemon detached, so the compositor-owned bind returns immediately.
fn spawn_daemon() -> std::io::Result<()> {
    let mut cmd = Command::new(std::env::current_exe()?);
    if std::env::var("ZOV_DEBUG").ok().as_deref() == Some("1") {
        cmd.stdin(Stdio::null());
    } else {
        cmd.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    }
    cmd.spawn()?;
    Ok(())
}

// ---- main ----

struct StderrLogger;

impl log::Log for StderrLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.target().starts_with("sctk") || metadata.level() <= log::Level::Warn
    }
    fn log(&self, record: &log::Record) {
        eprintln!("[{}] {}", record.target(), record.args());
    }
    fn flush(&self) {}
}

static LOGGER: StderrLogger = StderrLogger;

fn init_debug_logging() {
    if std::env::var("ZOV_DEBUG").ok().as_deref() == Some("1") {
        let _ = log::set_logger(&LOGGER);
        log::set_max_level(log::LevelFilter::Debug);
    }
}

fn main() {
    init_debug_logging();
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Some(cmd) = args.first().map(|s| s.as_str()) {
        match cmd {
            "open" | "close" => {
                if ipc_client(cmd).is_none() && cmd == "open" {
                    // Not running: start the daemon, which shows itself on boot.
                    if let Err(e) = spawn_daemon() {
                        eprintln!("zov: failed to start daemon: {e}");
                        std::process::exit(1);
                    }
                    for _ in 0..100 {
                        if ipc_client("open").is_some() {
                            break;
                        }
                        std::thread::sleep(std::time::Duration::from_millis(20));
                    }
                }
                std::process::exit(0);
            }
            // Toggle for the keybind: close via IPC when running (so the daemon
            // can clean up its submap on the way out), else start the daemon.
            "toggle" => {
                if ipc_client("close").is_none() {
                    if let Err(e) = spawn_daemon() {
                        eprintln!("zov: failed to start daemon: {e}");
                        std::process::exit(1);
                    }
                }
                std::process::exit(0);
            }
            // Selection subcommands are relayed to the running daemon. They are
            // used by the Hyprland submap, because the compositor never grants
            // this surface keyboard focus and so arrow keys cannot arrive here.
            "sel-left" | "sel-right" | "sel-up" | "sel-down" | "activate" => {
                let _ = ipc_client(cmd);
                std::process::exit(0);
            }
            _ => {}
        }
    }

    // refuse to run twice
    let lock_path = std::env::var("XDG_RUNTIME_DIR")
        .map(|d| PathBuf::from(d).join("zov.lock"))
        .unwrap_or_else(|_| PathBuf::from("/tmp/zov.lock"));
    if let Ok(file) = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .open(&lock_path)
    {
        use std::os::fd::AsRawFd;
        if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            eprintln!("zov: already running");
            std::process::exit(0);
        }
        std::mem::forget(file);
    }

    let conn = match Connection::connect_to_env() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("zov: cannot connect to wayland: {e}");
            std::process::exit(1);
        }
    };
    let (globals, mut event_queue) = match registry_queue_init(&conn) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("zov: registry init failed: {e}");
            std::process::exit(1);
        }
    };
    let qh = event_queue.handle();
    let mut app = match App::new(conn, qh, globals) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("zov: {e}");
            std::process::exit(1);
        }
    };

    let mut event_loop: calloop::EventLoop<App> =
        calloop::EventLoop::try_new().expect("failed to create event loop");
    let handle = event_loop.handle();
    app.loop_handle = Some(handle.clone());
    let quit = event_loop.get_signal();
    app.quit = Some(quit.clone());

    // wayland source
    WaylandSource::new(app.conn.clone(), event_queue)
        .insert(handle.clone())
        .expect("failed to insert wayland source");

    // hypr event socket (nonblocking) → redraw + refresh on change
    if let Some(ev) = app.hypr.event.as_ref().and_then(|s| s.try_clone().ok()) {
        use calloop::Interest;
        let _ = handle.insert_source(
            Generic::new(ev, Interest::READ, calloop::Mode::Level),
            |_, _, app: &mut App| {
                let mut sink = Vec::new();
                app.hypr.drain_events(&mut sink);
                if !sink.is_empty() {
                    zlog(app.debug, &format!("events: {sink:?}"));
                }
                // Always drain so the socket cannot back up, but only pay for a
                // snapshot while the overview is actually on screen.
                if !sink.is_empty() && app.visible {
                    // Refresh the tape first so new rows/windows are visible.
                    if let Some(s) = app.hypr.snapshot() {
                        if !s.workspaces.is_empty() {
                            app.snap = s;
                        }
                    }
                    // Workspace switches and focus changes made while the
                    // overview is up move the highlight to the newly active
                    // window card — the tape mirrors live focus.
                    //
                    // Event payloads are used directly because the IPC queries
                    // (activewindow/activeworkspace) can lag a frame behind
                    // the switch on this build; the events themselves are the
                    // compositor's word on what just changed.
                    let ws_evt = sink.iter().rev().find(|(e, _)| e == "workspace");
                    let win_evt = sink.iter().rev().find(|(e, _)| e == "activewindowv2");
                    if let Some((_, payload)) = ws_evt {
                        // `workspace>>N` names the target row. Pick its MRU
                        // window (rank 0), else the first window it has.
                        if let Ok(n) = payload.trim().parse::<i64>() {
                            if let Some(ws) = app.snap.workspaces.iter().find(|w| w.id == n) {
                                let idx = ws
                                    .wins
                                    .iter()
                                    .position(|w| w.focus_history == 0)
                                    .or(if ws.wins.is_empty() { None } else { Some(0) });
                                if let Some(i) = idx {
                                    app.set_sel(ws.id, i);
                                }
                            }
                        }
                    } else if let Some((_, addr)) = win_evt {
                        // `activewindowv2>>0x…` — the exact window focused.
                        let addr = addr.trim().trim_start_matches("0x");
                        for ws in &app.snap.workspaces {
                            if let Some(i) = ws.wins.iter().position(|w| w.addr == addr) {
                                app.set_sel(ws.id, i);
                                break;
                            }
                        }
                    }
                    app.dirty = true;
                    app.maybe_render();
                }
                Ok(calloop::PostAction::Continue)
            },
        );
    }

    // IPC listener → open/toggle/close
    if let Some(l) = app.ipc_listener.as_ref().and_then(|s| s.try_clone().ok()) {
        use calloop::Interest;
        let _ = handle.insert_source(
            Generic::new(l, Interest::READ, calloop::Mode::Level),
            |_, _, app: &mut App| loop {
                let (mut stream, _) = match app.ipc_listener.as_ref().unwrap().accept() {
                    Ok(x) => x,
                    Err(_) => break Ok(calloop::PostAction::Continue),
                };
                let mut line = String::new();
                let _ = stream.read_to_string(&mut line);
                let v = line.trim().to_string();
                match v.as_str() {
                    "open" | "show" => app.show(),
                    "close" | "hide" => app.quit(),
                    // Selection driven from Hyprland keybinds. The compositor
                    // never grants this surface keyboard focus, so the arrow keys
                    // cannot reach us directly; the submap relays them here.
                    "sel-left" => app.move_sel(-1),
                    "sel-right" => app.move_sel(1),
                    "sel-up" => app.move_row(-1),
                    "sel-down" => app.move_row(1),
                    // Activating focuses the window, then the daemon leaves:
                    // the overlay is not needed once focus has moved. The
                    // timer inside select() drives the shutdown.
                    "activate" => app.activate(),
                    "ping" => {}
                    _ => {}
                }
                // Selection moves need a repaint so the highlight follows.
                if v.starts_with("sel-") {
                    app.maybe_render();
                }
                let state = if app.visible { "ok:visible" } else { "ok:hidden" };
                let _ = stream.write_all(state.as_bytes());
                let _ = stream.flush();
                // Reply is on the wire; now it is safe to tear the process down.
                if app.quit_requested {
                    app.exit_now();
                }
            },
        );
    }

    if let Some(s) = app.hypr.snapshot() {
        app.snap = s;
    }
    app.show();
    let _ = app.conn.flush();
    app.maybe_render();

    event_loop
        .run(None, &mut app, |_| {})
        .expect("event loop failed");

    // The daemon is one-shot: drop the socket so the next `zov open` does not
    // connect to a stale path and think a daemon is already alive.
    let _ = std::fs::remove_file(ipc_socket_path());
}
