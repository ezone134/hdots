//! zen-overview — niri-like scrollable workspace overview for Hyprland.
//!
//! A full-size layer-shell window (keyboard/pointer grabbed) that draws a
//! vertical tape of workspaces, each workspace as a row of its window tiles.
//! Orphaned by the shell, standalone: `zen-overview` shows the overview,
//! `zen-overview toggle` flips it, `zen-overview close` hides it.

mod hypr;
mod render;

use hypr::{Hypr, Snapshot};
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
use std::os::fd::AsRawFd;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

const NS: &str = "zen-overview";

struct App {
    running: AtomicBool,
    conn: Connection,
    qh: QueueHandle<App>,
    loop_handle: Option<LoopHandle<'static, App>>,
    registry_state: RegistryState,
    compositor: CompositorState,
    layer_shell: LayerShell,
    output_state: OutputState,
    seat_state: SeatState,
    shm: Shm,

    layer: Option<LayerSurface>,
    pool: Option<SlotPool>,
    configured: (i32, i32),

    pointer: Option<wl_pointer::WlPointer>,
    keyboard: Option<wl_keyboard::WlKeyboard>,

    hypr: Hypr,
    snap: Snapshot,
    scroll: f32,
    dirty: bool,
    theme: Theme,
    renderer: Renderer,

    ipc_listener: Option<UnixListener>,
    ipc_rx: Vec<String>,
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
        let renderer = Renderer::new("Sans".into());
        let hypr = Hypr::new();

        let snap = hypr.snapshot().unwrap_or_default();

        let ipc_listener = bind_ipc().ok();

        Ok(App {
            running: AtomicBool::new(true),
            conn,
            qh,
            loop_handle: None,
            registry_state: RegistryState::new(&globals),
            compositor,
            layer_shell,
            output_state,
            seat_state,
            shm,
            layer: None,
            pool: None,
            configured: (0, 0),
            pointer: None,
            keyboard: None,
            hypr,
            snap,
            scroll: 0.0,
            dirty: true,
            theme: Theme::default(),
            renderer,
            ipc_listener,
            ipc_rx: Vec::new(),
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
        let _ = self.conn.flush();
    }

    fn draw(&mut self, qh: &QueueHandle<Self>) {
        let (w, h) = self.configured;
        if w <= 0 || h <= 0 {
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
                .draw(&mut px, &self.snap, &self.theme, self.scroll);
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

    fn select(&mut self, ws_id: i64, idx: usize) {
        let win = self
            .snap
            .workspaces
            .iter()
            .find(|w| w.id == ws_id)
            .and_then(|w| w.wins.get(idx));
        if let Some(w) = win {
            self.hypr.switch_workspace(ws_id);
            self.hypr.focus_window(&w.addr);
        }
        self.close();
    }

    fn close(&mut self) {
        std::process::exit(0);
    }

    fn maybe_render(&mut self) {
        if self.dirty {
            self.draw(&self.qh.clone());
        }
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
        qh: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: u32,
    ) {
        // Re-render on frame callback (continuous updates while visible)
        self.dirty = true;
        self.draw(qh);
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
    fn new_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}
    fn new_capability(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        seat: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Pointer && self.pointer.is_none() {
            if let Ok(ptr) = self.seat_state.get_pointer(qh, &seat) {
                self.pointer = Some(ptr);
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
                }
            }
            if self.keyboard.is_none() {
                if let Ok(kbd) = self.seat_state.get_keyboard(qh, &seat, None) {
                    self.keyboard = Some(kbd);
                }
            }
        }
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
        std::process::exit(0);
    }
    fn configure(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        _layer: &LayerSurface,
        configure: LayerSurfaceConfigure,
        _serial: u32,
    ) {
        let (w, h) = configure.new_size;
        let w = if w > 0 { w as i32 } else { self.configured.0 };
        let h = if h > 0 { h as i32 } else { self.configured.1 };
        self.configured = (w, h);
        self.dirty = true;
        // Draw on configure
        self.draw(qh);
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
            match e.kind {
                PointerEventKind::Axis { vertical, .. } => {
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
    fn enter(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: &wl_surface::WlSurface,
        _: u32,
        _: &[u32],
        _: &[Keysym],
    ) {
    }
    fn leave(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: &wl_surface::WlSurface,
        _: u32,
    ) {
    }
    fn press_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _kbd: &wl_keyboard::WlKeyboard,
        _serial: u32,
        event: KeyEvent,
    ) {
        if event.keysym == Keysym::Escape {
            self.close();
        } else if event.keysym >= Keysym::_1 && event.keysym <= Keysym::_9 {
            // number keys 1..9 → jump to workspace n
            let n = (event.keysym.raw() - Keysym::_1.raw() + 1) as i64;
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
    fn repeat_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _kbd: &wl_keyboard::WlKeyboard,
        _serial: u32,
        event: KeyEvent,
    ) {
        // Synthesized key repeat — ignore for now
        let _ = event;
    }
    fn release_key(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        _: KeyEvent,
    ) {
    }
    fn update_modifiers(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _serial: u32,
        _: Modifiers,
        _: RawModifiers,
        _: u32,
    ) {
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
        .map(|d| PathBuf::from(d).join("zen-overview.sock"))
        .unwrap_or_else(|_| PathBuf::from("/tmp/zen-overview.sock"))
}

fn bind_ipc() -> std::io::Result<UnixListener> {
    let path = ipc_socket_path();
    let _ = std::fs::remove_file(&path);
    UnixListener::bind(&path)
}

fn ipc_client(cmd: &str) -> i32 {
    let path = ipc_socket_path();
    let Ok(mut sock) = UnixStream::connect(&path) else {
        eprintln!(
            "zen-overview: not running ({} missing)",
            path.display()
        );
        return 1;
    };
    let _ = sock.write_all(cmd.as_bytes());
    let mut reply = String::new();
    let _ = sock.read_to_string(&mut reply);
    0
}

// ---- main ----

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Some(cmd) = args.first().map(|s| s.as_str()) {
        match cmd {
            "toggle" | "open" => {
                let ret = ipc_client(if cmd == "toggle" { "toggle" } else { "open" });
                std::process::exit(ret);
            }
            "close" => {
                let ret = ipc_client("close");
                std::process::exit(ret);
            }
            _ => {}
        }
    }

    // refuse to run twice
    let lock_path = std::env::var("XDG_RUNTIME_DIR")
        .map(|d| PathBuf::from(d).join("zen-overview.lock"))
        .unwrap_or_else(|_| PathBuf::from("/tmp/zen-overview.lock"));
    if let Ok(file) = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .open(&lock_path)
    {
        use std::os::fd::AsRawFd;
        if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            eprintln!("zen-overview: already running");
            std::process::exit(0);
        }
        std::mem::forget(file);
    }

    let conn = match Connection::connect_to_env() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("zen-overview: cannot connect to wayland: {e}");
            std::process::exit(1);
        }
    };
    let (globals, mut event_queue) = match registry_queue_init(&conn) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("zen-overview: registry init failed: {e}");
            std::process::exit(1);
        }
    };
    let qh = event_queue.handle();
    let mut app = match App::new(conn, qh, globals) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("zen-overview: {e}");
            std::process::exit(1);
        }
    };

    let mut event_loop: calloop::EventLoop<App> =
        calloop::EventLoop::try_new().expect("failed to create event loop");
    let handle = event_loop.handle();
    app.loop_handle = Some(handle.clone());

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
                    if let Some(s) = app.hypr.snapshot() {
                        app.snap = s;
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
                let _ = stream.write_all(b"ok");
                match v.as_str() {
                    "close" => app.close(),
                    "toggle" => {
                        // TODO: show/hide toggle logic
                        app.close();
                    }
                    "open" => {
                        // already shown
                    }
                    _ => {}
                }
            },
        );
    }

    app.create_layer();
    let _ = app.conn.flush();

    // Initial snapshot + render
    if let Some(s) = app.hypr.snapshot() {
        app.snap = s;
    }
    app.dirty = true;
    app.maybe_render();

    event_loop
        .run(None, &mut app, |_| {})
        .expect("event loop failed");
}
