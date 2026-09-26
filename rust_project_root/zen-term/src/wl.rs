//! Pure-Wayland event loop (smithay-client-toolkit, the wfm convention).
//!
//! The loop blocks on the wayland socket (calloop/epoll) with **no timers and
//! no file watches**: at rest the process uses 0 CPU. Redraws happen only when
//! something actually changed (pty output, input, resize, reload) and are
//! paced by `wl_surface.frame` callbacks.

use std::ffi::c_void;
use std::path::PathBuf;
use std::sync::Arc;

use alacritty_terminal::event::WindowSize;
use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::index::{Column, Point, Side};
use alacritty_terminal::selection::{Selection, SelectionType};
use alacritty_terminal::term::TermMode;
use alacritty_terminal::vte::ansi::Rgb;
use calloop::channel::{Channel, Event};
use smithay_client_toolkit::compositor::{CompositorHandler, CompositorState, FrameCallbackData};
use smithay_client_toolkit::data_device_manager::data_device::{DataDevice, DataDeviceHandler};
use smithay_client_toolkit::data_device_manager::data_offer::{DataOfferHandler, DragOffer, SelectionOffer};
use smithay_client_toolkit::data_device_manager::data_source::{CopyPasteSource, DataSourceHandler};
use smithay_client_toolkit::data_device_manager::{DataDeviceManagerState, WritePipe};
use smithay_client_toolkit::output::{OutputHandler, OutputState};
use smithay_client_toolkit::reexports::calloop::{EventLoop, LoopHandle, PostAction};
use smithay_client_toolkit::reexports::calloop_wayland_source::WaylandSource;
use smithay_client_toolkit::registry::{ProvidesRegistryState, RegistryState};
use smithay_client_toolkit::seat::keyboard::{KeyEvent, KeyboardHandler, Modifiers, RawModifiers};
use smithay_client_toolkit::seat::pointer::{AxisScroll, PointerEventKind, PointerHandler};
use smithay_client_toolkit::seat::{Capability, SeatHandler, SeatState};
use smithay_client_toolkit::shell::xdg::window::{Window, WindowConfigure, WindowDecorations, WindowHandler};
use smithay_client_toolkit::shell::WaylandSurface;
use smithay_client_toolkit::shm::{Shm, ShmHandler};
use smithay_client_toolkit::{delegate_dispatch2, delegate_registry, registry_handlers};
use wayland_client::globals::registry_queue_init;
use wayland_client::protocol::{wl_data_device, wl_data_device_manager::DndAction, wl_data_source, wl_keyboard, wl_output, wl_pointer, wl_seat, wl_surface};
use wayland_client::{Connection, Proxy, QueueHandle};
use wayland_egl::WlEglSurface;

use crate::config::{self, Config};
use crate::input;
use crate::render::Renderer;
use crate::render_gl;
use crate::term::{Terminal, UiMsg};

const APP_ID: &str = "zen-term";

pub struct Wl {
    registry_state: RegistryState,
    seat_state: SeatState,
    output_state: OutputState,
    shm: Shm,
    data_device_manager: Option<DataDeviceManagerState>,
    window: Window,
    renderer: Option<Renderer>,
    terminal: Terminal,
    keyboard: Option<wl_keyboard::WlKeyboard>,
    pointer: Option<wl_pointer::WlPointer>,
    data_device: Option<DataDevice>,
    copy_source: Option<CopyPasteSource>,
    clipboard_text: Option<String>,
    clipboard_offer: Option<SelectionOffer>,
    pointer_pos: (f64, f64),
    mouse_down: bool,
    mods_ctrl: bool,
    mods_shift: bool,
    mods_alt: bool,
    scale: f64,
    configured: (i32, i32),
    dirty: bool,
    frame_pending: bool,
    running: bool,
    qh: QueueHandle<Wl>,
    loop_handle: LoopHandle<'static, Wl>,
    cfg: Config,
    cfg_path: PathBuf,
}

pub fn run(
    cfg: Config,
    cfg_path: PathBuf,
    terminal: Terminal,
    ui_rx: Channel<UiMsg>,
) -> Result<(), Box<dyn std::error::Error>> {
    crate::install_reload_handler();
    let conn = Connection::connect_to_env()?;
    let (globals, mut event_queue) = registry_queue_init(&conn).expect("failed to init registry");
    let qh = event_queue.handle();
    let mut event_loop: EventLoop<Wl> = EventLoop::try_new()?;
    let loop_handle = event_loop.handle();

    let compositor = CompositorState::bind(&globals, &qh).expect("wl_compositor not available");
    let xdg_shell =
        smithay_client_toolkit::shell::xdg::XdgShell::bind(&globals, &qh).expect("xdg shell not available");
    let shm = Shm::bind(&globals, &qh).expect("wl_shm not available");
    let data_device_manager = DataDeviceManagerState::bind(&globals, &qh).ok();

    let surface = compositor.create_surface(&qh);
    let window = xdg_shell.create_window(surface, WindowDecorations::RequestServer, &qh);
    window.set_title("zen-term");
    window.set_app_id(APP_ID);
    window.set_min_size(Some((160, 40)));
    window.commit();

    let mut wl = Wl {
        registry_state: RegistryState::new(&globals),
        seat_state: SeatState::new(&globals, &qh),
        output_state: OutputState::new(&globals, &qh),
        shm,
        data_device_manager,
        window,
        renderer: None,
        terminal,
        keyboard: None,
        pointer: None,
        data_device: None,
        copy_source: None,
        clipboard_text: None,
        clipboard_offer: None,
        pointer_pos: (0.0, 0.0),
        mouse_down: false,
        mods_ctrl: false,
        mods_shift: false,
        mods_alt: false,
        scale: 1.0,
        configured: (800, 600),
        dirty: true,
        frame_pending: false,
        running: true,
        qh: qh.clone(),
        loop_handle: loop_handle.clone(),
        cfg,
        cfg_path,
    };

    // First roundtrip: deliver the initial configure + scale factor (before
    // the event source is installed, so the queue can be pumped directly).
    let _ = event_queue.roundtrip(&mut wl);
    WaylandSource::new(conn.clone(), event_queue).insert(loop_handle.clone())?;

    // --- GPU init (GL/EGL; Vulkan slots in behind the same renderer) ---
    let (w, h) = wl.buffer_size();
    let display_ptr = conn.backend().display_ptr() as *mut c_void;
    let egl_window = WlEglSurface::new(wl.window.wl_surface().id(), w, h)?;
    let mut gl = unsafe { render_gl::Gl::init(display_ptr, egl_window.ptr(), egl_window, w, h) }
        .map_err(|e| format!("GL init: {e}"))?;
    gl.set_vsync(wl.cfg.vsync);
    let mut renderer = Renderer::new(Box::new(gl), &wl.cfg);
    renderer.size = (w, h);
    let scale = wl.scale.max(1.0);
    renderer.set_scale(scale as f32);
    let (cols, rows) = renderer.grid_size(w, h);
    wl.terminal.resize(cols, rows, (renderer.cell_w / scale as f32, renderer.cell_h / scale as f32));
    wl.renderer = Some(renderer);
    wl.request_redraw(); // first frame (the initial configure predates the renderer)

    // UI channel: pty thread → event loop.
    loop_handle.insert_source(ui_rx, |event, _, wl: &mut Wl| {
        if let Event::Msg(msg) = event {
            wl.on_ui_msg(msg);
        }
    })?;

    // Main loop: blocks in epoll_wait until a wayland event or a signal.
    // No timers, no file watches — an idle terminal costs 0 CPU.
    loop {
        event_loop.dispatch(None, &mut wl)?;
        if !wl.running {
            break;
        }
        // Signal-driven hot reload (`zen-term --reload` → SIGUSR1): the
        // signal interrupts epoll_wait with EINTR and the flag is checked
        // here, so idle cost stays 0.
        if crate::reload_requested() {
            wl.reload_config();
        }
    }

    Ok(())
}

impl Wl {
    fn request_redraw(&mut self) {
        if self.frame_pending || self.renderer.is_none() {
            return;
        }
        self.frame_pending = true;
        let surface = self.window.wl_surface();
        surface.frame(&self.qh, FrameCallbackData(surface.clone()));
        self.draw();
    }

    fn draw(&mut self) {
        let Some(renderer) = &mut self.renderer else {
            self.frame_pending = false;
            return;
        };
        let (w, h) = renderer.size;
        if w <= 0 || h <= 0 {
            self.frame_pending = false;
            return;
        }
        let term = self.terminal.term.lock();
        let rc = term.renderable_content();
        let offset = rc.display_offset;
        renderer.draw(rc, offset);
        drop(term);
        self.dirty = false;
    }

    fn on_ui_msg(&mut self, msg: UiMsg) {
        match msg {
            UiMsg::Redraw => {
                self.dirty = true;
                self.request_redraw();
            }
            UiMsg::Title(title) => {
                let title = title.unwrap_or_else(|| "zen-term".to_string());
                self.window.set_title(title);
            }
            UiMsg::Copy(text) => self.copy_to_clipboard(text, 0),
            UiMsg::Paste(formatter) => self.paste_clipboard(Some(formatter)),
            UiMsg::ColorRequest(index, formatter) => {
                let rgb = self.terminal.term.lock().colors()[index].unwrap_or(Rgb { r: 0, g: 0, b: 0 });
                let resp = formatter(rgb);
                self.terminal.write(resp.as_bytes());
            }
            UiMsg::TextAreaSize(formatter) => {
                let (cell_w, cell_h) = self.renderer.as_ref().map_or((0, 0), |r| {
                    let s = self.scale.max(1.0) as f32;
                    ((r.cell_w / s) as u16, (r.cell_h / s) as u16)
                });
                let term = self.terminal.term.lock();
                let resp = formatter(WindowSize {
                    num_lines: term.screen_lines() as u16,
                    num_cols: term.columns() as u16,
                    cell_width: cell_w,
                    cell_height: cell_h,
                });
                drop(term);
                self.terminal.write(resp.as_bytes());
            }
            UiMsg::PtyWrite(text) => self.terminal.write(text.as_bytes()),
            UiMsg::Bell => {
                // v1: no audible/visual bell.
            }
            UiMsg::Exit => {
                self.running = false;
            }
        }
    }

    /// Signal-driven config reload (`zen-term --reload`).
    fn reload_config(&mut self) {
        let mut cfg = Config::default();
        config::load(&mut cfg, &self.cfg_path);
        let font_changed = match &mut self.renderer {
            Some(renderer) => renderer.apply_config(&cfg),
            None => false,
        };
        if let Some(renderer) = &mut self.renderer {
            let (bw, bh) = renderer.size;
            let (cols, rows) = renderer.grid_size(bw, bh);
            let s = self.scale.max(1.0) as f32;
            self.terminal.resize(cols, rows, (renderer.cell_w / s, renderer.cell_h / s));
            let _ = font_changed;
        }
        self.terminal.reload_config(&cfg);
        self.cfg = cfg;
        self.dirty = true;
        self.request_redraw();
    }

    // ── input ────────────────────────────────────────────────────────────

    fn on_key(&mut self, event: KeyEvent, serial: u32) {
        let mode = *self.terminal.term.lock().mode();
        let outcome = input::translate(
            &event,
            self.mods_ctrl,
            self.mods_alt,
            self.mods_shift,
            &mode,
        );
        if outcome.copy {
            let text = self.terminal.term.lock().selection_to_string().unwrap_or_default();
            if !text.is_empty() {
                self.copy_to_clipboard(text, serial);
            }
            return;
        }
        if outcome.paste {
            self.paste_clipboard(None);
            return;
        }
        if !outcome.bytes.is_empty() {
            self.terminal.write(&outcome.bytes);
        }
    }

    /// Pointer position → grid point (viewport row/col → terminal line/col).
    fn pointer_point(&self) -> Option<Point> {
        let renderer = self.renderer.as_ref()?;
        let s = self.scale.max(1.0);
        let (px, py) = self.pointer_pos;
        let bx = px * s;
        let by = py * s;
        let pad = renderer.padding as f64;
        let col = ((bx - pad) / renderer.cell_w as f64).floor().max(0.0) as usize;
        let row = ((by - pad) / renderer.cell_h as f64).floor().max(0.0) as usize;
        let col = col.min(renderer.cols.saturating_sub(1));
        let row = row.min(renderer.rows.saturating_sub(1));
        let offset = self.terminal.term.lock().grid().display_offset();
        Some(alacritty_terminal::term::viewport_to_point(offset, Point::new(row, Column(col))))
    }

    fn start_selection(&mut self, point: Point) {
        self.mouse_down = true;
        self.terminal.term.lock().selection =
            Some(Selection::new(SelectionType::Simple, point, Side::Left));
        self.dirty = true;
        self.request_redraw();
    }

    fn update_selection(&mut self, point: Point) {
        let mut term = self.terminal.term.lock();
        if let Some(sel) = &mut term.selection {
            sel.update(point, Side::Right);
        }
        drop(term);
        self.dirty = true;
        self.request_redraw();
    }

    fn finish_selection(&mut self) {
        let mut term = self.terminal.term.lock();
        if let Some(sel) = &term.selection {
            if sel.is_empty() {
                term.selection = None;
            }
        }
        drop(term);
        self.dirty = true;
        self.request_redraw();
    }

    fn on_scroll(&mut self, axis: &AxisScroll) {
        let steps = if axis.value120 != 0 {
            axis.value120 as f32 / 120.0
        } else if axis.discrete != 0 {
            axis.discrete as f32
        } else {
            (axis.absolute / 15.0) as f32
        };
        let steps = steps.round() as i32;
        if steps == 0 {
            return;
        }
        let mode = *self.terminal.term.lock().mode();
        if mode.contains(TermMode::ALT_SCREEN) && mode.contains(TermMode::ALTERNATE_SCROLL) {
            // Alternate scroll: translate wheel into arrow keys for the app.
            let key: &[u8] = if steps > 0 { b"\x1b[A" } else { b"\x1b[B" };
            let mut bytes = Vec::with_capacity(key.len() * steps.unsigned_abs() as usize);
            for _ in 0..steps.unsigned_abs() {
                bytes.extend_from_slice(key);
            }
            self.terminal.write(&bytes);
        } else {
            self.terminal.scroll(steps);
            self.dirty = true;
            self.request_redraw();
        }
    }

    // ── clipboard (wl_data_device) ───────────────────────────────────────

    fn copy_to_clipboard(&mut self, text: String, serial: u32) {
        let (Some(mgr), Some(device)) = (&self.data_device_manager, &self.data_device) else {
            return;
        };
        self.clipboard_text = Some(text);
        let source = mgr.create_copy_paste_source(
            &self.qh,
            ["text/plain;charset=utf-8", "text/plain"],
        );
        source.set_selection(device, serial);
        self.copy_source = Some(source);
    }

    fn paste_clipboard(&mut self, formatter: Option<Arc<dyn Fn(&str) -> String + Send + Sync>>) {
        let Some(device) = &self.data_device else {
            return;
        };
        let Some(offer) = device.data().selection_offer() else {
            return;
        };
        let mime = offer.with_mime_types(|mimes: &[String]| {
            mimes
                .iter()
                .find(|m| m.as_str() == "text/plain;charset=utf-8" || m.as_str() == "text/plain")
                .cloned()
        });
        let Some(mime) = mime else {
            return;
        };
        let pipe = match offer.receive(mime) {
            Ok(pipe) => pipe,
            Err(_) => return,
        };        self.clipboard_offer = Some(offer);
        let handle = self.loop_handle.clone();
        let _ = handle.insert_source(
            pipe,
            move |_, f: &mut calloop::generic::NoIoDrop<std::fs::File>, wl: &mut Wl| {
                let mut data = Vec::new();
                use std::io::Read;
                let file: &mut std::fs::File = unsafe { f.get_mut() };
                let mut buf = [0u8; 8192];
                loop {
                    match file.read(&mut buf) {
                        Ok(0) => break,
                        Ok(n) => data.extend_from_slice(&buf[..n]),
                        Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                        Err(_) => break,
                    }
                }
                let text = String::from_utf8_lossy(&data);
                match &formatter {
                    // Terminal-requested paste (OSC52 load): the formatter
                    // already produced the exact bytes to write.
                    Some(fmt) => {
                        let formatted = fmt(&text);
                        wl.terminal.write(formatted.as_bytes());
                    }
                    // App shortcut (Ctrl+Shift+V): handle bracketed paste.
                    None => wl.terminal.paste(&text),
                }
                PostAction::Continue
            },
        );
    }
}

// ── SCTK handlers ───────────────────────────────────────────────────────

impl CompositorHandler for Wl {
    fn scale_factor_changed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, scale: i32) {
        let s = scale.max(1) as f64;
        if (self.scale - s).abs() < 0.001 {
            return;
        }
        self.scale = s;
        let (bw, bh) = self.buffer_size();
        if let Some(renderer) = &mut self.renderer {
            renderer.set_scale(s as f32);
            renderer.size = (bw, bh);
            renderer.backend.resize(bw, bh);
        }
        self.apply_grid();
        self.dirty = true;
        self.request_redraw();
    }

    fn transform_changed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: wl_output::Transform) {}

    fn frame(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: u32) {
        self.frame_pending = false;
        if self.dirty {
            self.request_redraw();
        }
    }

    fn surface_enter(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: &wl_output::WlOutput) {}
    fn surface_leave(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: &wl_output::WlOutput) {}
}

impl OutputHandler for Wl {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }
    fn new_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
    fn update_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
    fn output_destroyed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
}

impl WindowHandler for Wl {
    fn request_close(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &Window) {
        self.running = false;
    }

    fn configure(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &Window, configure: WindowConfigure, _serial: u32) {
        let w = configure.new_size.0.map(|v| v.get() as i32).unwrap_or(self.configured.0);
        let h = configure.new_size.1.map(|v| v.get() as i32).unwrap_or(self.configured.1);
        // Skip degenerate 0×0 configures (initial unmapped state): keep the
        // previous size instead of collapsing to a 1×1 window.
        if w > 0 && h > 0 {
            self.configured = (w, h);
        }
        let (bw, bh) = self.buffer_size();
        if let Some(renderer) = &mut self.renderer {
            renderer.size = (bw, bh);
            renderer.backend.resize(bw, bh);
        }
        self.apply_grid();
        self.dirty = true;
        // Draw immediately so a buffer attaches on this very commit (an empty
        // surface never gets presented, so the frame callback never fires).
        self.request_redraw();
    }
}

impl SeatHandler for Wl {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seat_state
    }
    fn new_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}

    fn new_capability(
        &mut self,
        _: &Connection,
        qh: &QueueHandle<Self>,
        seat: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Keyboard && self.keyboard.is_none() {
            let keyboard = self
                .seat_state
                .get_keyboard_with_repeat(
                    qh,
                    &seat,
                    None,
                    self.loop_handle.clone(),
                    Box::new(|state: &mut Wl, _kbd: &wl_keyboard::WlKeyboard, event: KeyEvent| {
                        state.on_key(event, 0);
                    }),
                )
                .expect("failed to create keyboard");
            self.keyboard = Some(keyboard);
        }
        if capability == Capability::Pointer && self.pointer.is_none() {
            if let Ok(pointer) = self.seat_state.get_pointer(qh, &seat) {
                self.pointer = Some(pointer);
            }
        }
        if self.data_device.is_none() {
            if let Some(mgr) = &self.data_device_manager {
                self.data_device = Some(mgr.get_data_device(qh, &seat));
            }
        }
    }

    fn remove_capability(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat, capability: Capability) {
        if capability == Capability::Keyboard && self.keyboard.is_some() {
            self.keyboard.take().unwrap().release();
        }
        if capability == Capability::Pointer && self.pointer.is_some() {
            self.pointer.take();
        }
    }

    fn remove_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}
}

impl KeyboardHandler for Wl {
    fn enter(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_keyboard::WlKeyboard, _: &wl_surface::WlSurface, _: u32, _: &[u32], _: &[smithay_client_toolkit::seat::keyboard::Keysym]) {
        self.terminal.set_focused(true);
        self.dirty = true;
        self.request_redraw();
    }
    fn leave(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_keyboard::WlKeyboard, _: &wl_surface::WlSurface, _: u32) {
        self.terminal.set_focused(false);
        self.dirty = true;
        self.request_redraw();
    }
    fn press_key(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_keyboard::WlKeyboard, serial: u32, event: KeyEvent) {
        self.on_key(event, serial);
    }
    fn repeat_key(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_keyboard::WlKeyboard, _serial: u32, event: KeyEvent) {
        self.on_key(event, 0);
    }
    fn release_key(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_keyboard::WlKeyboard, _serial: u32, _event: KeyEvent) {}
    fn update_modifiers(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_keyboard::WlKeyboard, _serial: u32, modifiers: Modifiers, _raw: RawModifiers, _layout: u32) {
        self.mods_ctrl = modifiers.ctrl;
        self.mods_shift = modifiers.shift;
        self.mods_alt = modifiers.alt;
    }
}

impl PointerHandler for Wl {
    fn pointer_frame(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_pointer::WlPointer, events: &[smithay_client_toolkit::seat::pointer::PointerEvent]) {
        for e in events {
            self.pointer_pos = e.position;
            match &e.kind {
                PointerEventKind::Enter { .. } => {}
                PointerEventKind::Leave { .. } => {
                    self.mouse_down = false;
                }
                PointerEventKind::Motion { .. } => {
                    if self.mouse_down {
                        if let Some(p) = self.pointer_point() {
                            self.update_selection(p);
                        }
                    }
                }
                PointerEventKind::Press { button, serial, .. } => match *button {
                    0x110 => {
                        if let Some(p) = self.pointer_point() {
                            self.start_selection(p);
                            let _ = serial;
                        }
                    }
                    0x112 => {
                        // Middle click: paste the selection.
                        self.paste_clipboard(None);
                    }
                    _ => {}
                },
                PointerEventKind::Release { button, .. } => {
                    if *button == 0x110 {
                        self.mouse_down = false;
                        self.finish_selection();
                    }
                }
                PointerEventKind::Axis { vertical, .. } => {
                    self.on_scroll(vertical);
                }
            }
        }
    }
}

impl ShmHandler for Wl {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}

impl DataDeviceHandler for Wl {
    fn enter(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_data_device::WlDataDevice,
        _: f64,
        _: f64,
        _: &wl_surface::WlSurface,
    ) {
    }
    fn leave(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_data_device::WlDataDevice) {}
    fn motion(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_data_device::WlDataDevice, _: f64, _: f64) {}
    fn drop_performed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_data_device::WlDataDevice) {}
    fn selection(&mut self, _: &Connection, _: &QueueHandle<Self>, data_device: &wl_data_device::WlDataDevice) {
        if let Some(device) = &self.data_device {
            if device.inner() == data_device {
                self.clipboard_offer = device.data().selection_offer();
            }
        }
    }
}

impl DataOfferHandler for Wl {
    fn source_actions(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &mut DragOffer,
        _actions: DndAction,
    ) {
    }

    fn selected_action(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &mut DragOffer,
        _action: DndAction,
    ) {
    }
}

impl DataSourceHandler for Wl {
    fn accept_mime(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_data_source::WlDataSource, _: Option<String>) {}
    fn send_request(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_data_source::WlDataSource, mime: String, write_pipe: WritePipe) {
        if !mime.starts_with("text/plain") {
            return;
        }
        let Some(text) = self.clipboard_text.clone() else {
            return;
        };
        use std::io::Write;
        let mut f = std::fs::File::from(std::os::unix::io::OwnedFd::from(write_pipe));
        let _ = f.write_all(text.as_bytes());
    }
    fn cancelled(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_data_source::WlDataSource) {
        self.copy_source = None;
    }
    fn dnd_dropped(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_data_source::WlDataSource) {}
    fn dnd_finished(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_data_source::WlDataSource) {}
    fn action(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_data_source::WlDataSource, _: DndAction) {}
}

impl Wl {
    /// Configured logical size × scale → buffer pixels.
    fn buffer_size(&self) -> (i32, i32) {
        let s = self.scale.max(1.0);
        (
            ((self.configured.0 as f64 * s) as i32).max(1),
            ((self.configured.1 as f64 * s) as i32).max(1),
        )
    }

    /// Recompute the grid and resize the terminal after a size/scale change.
    fn apply_grid(&mut self) {
        let Some(renderer) = &mut self.renderer else {
            return;
        };
        let (bw, bh) = renderer.size;
        let (cols, rows) = renderer.grid_size(bw, bh);
        let s = self.scale.max(1.0) as f32;
        self.terminal.resize(cols, rows, (renderer.cell_w / s, renderer.cell_h / s));
    }
}

delegate_registry!(Wl);

impl ProvidesRegistryState for Wl {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![OutputState, SeatState,];
}

delegate_dispatch2!(Wl);
