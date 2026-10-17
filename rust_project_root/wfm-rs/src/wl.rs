use std::time::Duration;

use smithay_client_toolkit::activation::{ActivationHandler, ActivationState, RequestData};
use smithay_client_toolkit::compositor::{CompositorHandler, CompositorState, FrameCallbackData};
use smithay_client_toolkit::data_device_manager::{
    data_device::{DataDevice, DataDeviceHandler},
    data_offer::{DataOfferHandler, DragOffer},
    data_source::{DataSourceHandler, DragSource},
    DataDeviceManagerState, WritePipe,
};
use smithay_client_toolkit::output::{OutputHandler, OutputState};
use smithay_client_toolkit::reexports::calloop::channel::Event as ChannelEvent;
use smithay_client_toolkit::reexports::calloop::{PostAction, RegistrationToken};
use smithay_client_toolkit::registry::{ProvidesRegistryState, RegistryState};
use smithay_client_toolkit::reexports::calloop::EventLoop;
use smithay_client_toolkit::reexports::calloop_wayland_source::WaylandSource;
use smithay_client_toolkit::seat::keyboard::{KeyEvent, KeyboardHandler, Modifiers, RawModifiers};
use smithay_client_toolkit::seat::pointer::{
    CursorIcon, PointerEvent, PointerEventKind, PointerHandler, ThemeSpec, ThemedPointer,
};
use smithay_client_toolkit::seat::{Capability, SeatHandler, SeatState};
use smithay_client_toolkit::shell::xdg::window::{Window, WindowConfigure, WindowDecorations, WindowHandler};
use smithay_client_toolkit::shell::WaylandSurface;
use smithay_client_toolkit::shm::slot::{Buffer, SlotPool};
use smithay_client_toolkit::shm::{Shm, ShmHandler};
use smithay_client_toolkit::{delegate_dispatch2, delegate_registry, registry_handlers};
use wayland_client::globals::registry_queue_init;
use wayland_client::protocol::{
    wl_data_device::WlDataDevice, wl_data_device_manager::DndAction, wl_data_source::WlDataSource,
    wl_keyboard, wl_output, wl_pointer, wl_seat, wl_shm, wl_surface,
};
use wayland_client::{Connection, QueueHandle};

use crate::app::App;
use crate::layout;
use crate::render;

pub use smithay_client_toolkit::seat::keyboard::Keysym;

/// A pressed left button, tracked so a small drag threshold can start a DnD.
#[derive(Clone, Copy)]
struct PressState {
    x: i32,
    y: i32,
    serial: u32,
    on_entry: bool,
}

fn pick_dnd_mime(mimes: &[String]) -> Option<String> {
    for m in mimes {
        if m == "text/uri-list" || m == "x-special/gnome-copied-files" {
            return Some(m.clone());
        }
    }
    None
}

/// Run the event loop; returns when the app requests exit.
pub fn run(app: App, title: &str) -> Result<(), Box<dyn std::error::Error>> {
    crate::install_reload_handler();
    let conn = Connection::connect_to_env()?;
    let (globals, event_queue) = registry_queue_init(&conn).expect("Failed to init registry");
    let qh = event_queue.handle();
    let event_loop: EventLoop<Wl> = EventLoop::try_new()?;
    let mut event_loop = event_loop;
    let loop_handle = event_loop.handle();
    WaylandSource::new(conn.clone(), event_queue).insert(loop_handle)?;

    let compositor = CompositorState::bind(&globals, &qh).expect("wl_compositor not available");
    let xdg_shell =
        smithay_client_toolkit::shell::xdg::XdgShell::bind(&globals, &qh).expect("xdg shell not available");
    let shm = Shm::bind(&globals, &qh).expect("wl_shm not available");
    let xdg_activation = ActivationState::bind(&globals, &qh).ok();

    let surface = compositor.create_surface(&qh);
    let window = xdg_shell.create_window(surface, WindowDecorations::RequestServer, &qh);
    window.set_title(title.to_string());
    window.set_app_id("wfm.wfm");
    window.set_min_size(Some((320, 240)));
    window.commit();

    let pool = SlotPool::new(1024 * 1024 * 4, &shm).expect("Failed to create pool");
    let data_device_manager = DataDeviceManagerState::bind(&globals, &qh).ok();

    let (worker, job_tx) = crate::worker::Worker::new();
    let mut app = app;
    app.worker = Some(job_tx);

    let mut wl = Wl {
        registry_state: RegistryState::new(&globals),
        seat_state: SeatState::new(&globals, &qh),
        output_state: OutputState::new(&globals, &qh),
        compositor,
        shm,
        xdg_activation,
        window,
        pool,
        buffer: None,
        keyboard: None,
        pointer: None,
        cursor_icon: CursorIcon::Default,
        cursor_applied: false,
        loop_handle: event_loop.handle(),
        frame_pending: false,
        buffer_scale: 1,
        conn: conn.clone(),
        qh: qh.clone(),
        app,
        data_device_manager,
        data_device: None,
        drag_sources: Vec::new(),
        drag_surface: None,
        drag_pool: None,
        dnd_offers: Vec::new(),
        press: None,
    };

    event_loop.handle().insert_source(worker.res_rx, |event, _, state| {
        if let ChannelEvent::Msg(res) = event {
            state.app.apply_worker_result(res);
            if state.app.dirty {
                state.request_redraw();
            }
        }
    })?;

    if let Some(chk_rx) = wl.app.checksum_rx.take() {
        event_loop.handle().insert_source(chk_rx, |event, _, state| {
            if let ChannelEvent::Msg(res) = event {
                state.app.apply_checksum_result(res);
                if state.app.dirty {
                    state.request_redraw();
                }
            }
        })?;
    }

    loop {
        event_loop.dispatch(Duration::from_millis(16), &mut wl)?;
        if !wl.app.running {
            break;
        }
        // Hot reload is signal-driven only (`wfm --reload` → SIGUSR1): no
        // config polling, so an idle window costs nothing.
        if crate::reload_requested() {
            wl.app.reload_config();
            wl.request_redraw();
        }
    }
    wl.app.save_state();
    Ok(())
}

struct Wl {
    registry_state: RegistryState,
    seat_state: SeatState,
    output_state: OutputState,
    compositor: CompositorState,
    shm: Shm,
    xdg_activation: Option<ActivationState>,
    window: Window,
    pool: SlotPool,
    buffer: Option<Buffer>,
    keyboard: Option<wl_keyboard::WlKeyboard>,
    pointer: Option<ThemedPointer>,
    cursor_icon: CursorIcon,
    cursor_applied: bool,
    loop_handle: smithay_client_toolkit::reexports::calloop::LoopHandle<'static, Wl>,
    frame_pending: bool,
    buffer_scale: i32,
    conn: Connection,
    qh: QueueHandle<Wl>,
    app: App,
    data_device_manager: Option<DataDeviceManagerState>,
    data_device: Option<DataDevice>,
    drag_sources: Vec<DragSource>,
    drag_surface: Option<wl_surface::WlSurface>,
    drag_pool: Option<SlotPool>,
    dnd_offers: Vec<(DragOffer, Vec<u8>, Option<RegistrationToken>)>,
    press: Option<PressState>,
}

impl Wl {
    /// Mark the app dirty and request a frame from the compositor; the frame
    /// callback triggers the actual redraw in `CompositorHandler::frame`.
    fn request_redraw(&mut self) {
        if self.frame_pending {
            return;
        }
        self.frame_pending = true;
        let surface = self.window.wl_surface();
        surface.frame(&self.qh, FrameCallbackData(surface.clone()));
        surface.damage_buffer(0, 0, self.app.width as i32, self.app.height as i32);
        self.window.commit();
    }

    fn draw(&mut self) {
        let scale = self.buffer_scale.max(1);
        // app.width/height are in surface (logical) coordinates from xdg configure.
        let logical_w = self.app.width as i32;
        let logical_h = self.app.height as i32;
        if logical_w <= 0 || logical_h <= 0 {
            return;
        }
        let width = logical_w * scale;
        let height = logical_h * scale;
        let stride = width * 4;

        // Render at buffer resolution so text is sharp on HiDPI.
        let saved_w = self.app.width;
        let saved_h = self.app.height;
        self.app.width = width as u32;
        self.app.height = height as u32;
        // Scale UI metrics that are in pixels (font, padding, sidebar).
        let saved_font = self.app.cfg.font_size;
        let saved_pad = self.app.cfg.padding;
        let saved_side = self.app.cfg.sidebar_width;
        let saved_grid = self.app.cfg.grid_cell;
        let saved_thumb = self.app.cfg.thumb_size;
        if scale > 1 {
            self.app.cfg.font_size = (saved_font * scale).max(8);
            self.app.cfg.padding = (saved_pad * scale).max(1);
            self.app.cfg.sidebar_width = saved_side * scale;
            self.app.cfg.grid_cell = saved_grid * scale;
            self.app.cfg.thumb_size = saved_thumb * scale;
            if let Some(ref mut text) = self.app.text {
                text.font_size = self.app.cfg.font_size as f32;
            }
        }

        let buffer = match &mut self.buffer {
            Some(b) => {
                let (bw, bh) = (b.stride() / 4, b.height());
                if bw != width || bh != height {
                    *b = self
                        .pool
                        .create_buffer(width, height, stride, wl_shm::Format::Argb8888)
                        .unwrap()
                        .0;
                }
                b
            }
            None => {
                self.buffer = Some(
                    self.pool
                        .create_buffer(width, height, stride, wl_shm::Format::Argb8888)
                        .unwrap()
                        .0,
                );
                self.buffer.as_mut().unwrap()
            }
        };

        let canvas = match self.pool.canvas(buffer) {
            Some(c) => c,
            None => {
                let (second, c) = self
                    .pool
                    .create_buffer(width, height, stride, wl_shm::Format::Argb8888)
                    .unwrap();
                *buffer = second;
                c
            }
        };

        let pixels = render::render(&self.app);
        // Restore logical metrics immediately after rasterize.
        self.app.width = saved_w;
        self.app.height = saved_h;
        self.app.cfg.font_size = saved_font;
        self.app.cfg.padding = saved_pad;
        self.app.cfg.sidebar_width = saved_side;
        self.app.cfg.grid_cell = saved_grid;
        self.app.cfg.thumb_size = saved_thumb;
        if let Some(ref mut text) = self.app.text {
            text.font_size = saved_font as f32;
        }

        let need = (height as usize) * (stride as usize);
        if canvas.len() < need || pixels.len() < (width as usize * height as usize) {
            eprintln!(
                "wfm: draw size mismatch canvas={} need={} pixels={} {}x{}",
                canvas.len(),
                need,
                pixels.len(),
                width,
                height
            );
            return;
        }
        for y in 0..height as usize {
            let row = y * width as usize;
            let dst_row = y * stride as usize;
            for x in 0..width as usize {
                let px = pixels[row + x];
                let o = dst_row + x * 4;
                // wl_shm Argb8888 little-endian: B, G, R, A
                canvas[o] = (px & 0xFF) as u8;
                canvas[o + 1] = ((px >> 8) & 0xFF) as u8;
                canvas[o + 2] = ((px >> 16) & 0xFF) as u8;
                canvas[o + 3] = 0xFF;
            }
        }

        let surface = self.window.wl_surface();
        surface.set_buffer_scale(scale);
        surface.damage_buffer(0, 0, width, height);
        surface.frame(&self.qh, FrameCallbackData(surface.clone()));
        self.frame_pending = true;
        buffer.attach_to(surface).expect("buffer attach");
        self.window.commit();
        self.app.dirty = false;
    }
}

impl CompositorHandler for Wl {
    fn scale_factor_changed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        scale: i32,
    ) {
        if scale >= 1 && scale != self.buffer_scale {
            self.buffer_scale = scale;
            self.buffer = None;
            self.app.dirty = true;
            self.request_redraw();
        }
    }

    fn transform_changed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: wl_output::Transform) {}

    fn frame(&mut self, conn: &Connection, qh: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: u32) {
        let _ = (conn, qh);
        self.frame_pending = false;
        if self.app.dirty {
            self.draw();
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
        self.app.request_close();
    }

    fn configure(&mut self, conn: &Connection, qh: &QueueHandle<Self>, _: &Window, configure: WindowConfigure, _serial: u32) {
        let _ = (conn, qh);
        self.buffer = None;
        self.app.width = configure.new_size.0.map(|v| v.get()).unwrap_or(self.app.width);
        self.app.height = configure.new_size.1.map(|v| v.get()).unwrap_or(self.app.height);
        if self.app.height < 1 {
            self.app.height = 1;
        }
        if self.app.width < 1 {
            self.app.width = 1;
        }
        self.app.apply_responsive_layout();
        self.app.dirty = true;
        // Draw immediately so a buffer is attached on this very commit;
        // waiting for the frame callback first would deadlock (an empty
        // surface never gets presented, so `frame` never fires).
        self.draw();
    }
}

impl ActivationHandler for Wl {
    type RequestUdata = ();
    fn new_token(&mut self, token: String, _: &RequestData<()>) {
        if let Some(a) = &self.xdg_activation {
            a.activate::<Wl>(self.window.wl_surface(), token);
        }
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
                        state.on_key_event(event);
                    }),
                )
                .expect("Failed to create keyboard");
            self.keyboard = Some(keyboard);
        }

        if capability == Capability::Pointer && self.pointer.is_none() {
            let surface = self.compositor.create_surface(qh);
            let pointer = self
                .seat_state
                .get_pointer_with_theme::<Wl, ()>(
                    qh,
                    &seat,
                    self.shm.wl_shm(),
                    surface,
                    ThemeSpec::System,
                )
                .expect("Failed to create pointer");
            // No enter serial yet, so this fails silently (MissingEnterSerial);
            // the next enter event re-applies it via sync_cursor.
            let _ = pointer.set_cursor(&self.conn, CursorIcon::Default);
            self.pointer = Some(pointer);
            self.cursor_icon = CursorIcon::Default;
            self.cursor_applied = false;
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
            self.cursor_applied = false;
        }
    }

    fn remove_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}
}

impl KeyboardHandler for Wl {
    fn enter(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_keyboard::WlKeyboard, _: &wl_surface::WlSurface, _: u32, _: &[u32], _: &[Keysym]) {}

    fn leave(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_keyboard::WlKeyboard, _: &wl_surface::WlSurface, _: u32) {}

    fn press_key(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_keyboard::WlKeyboard, _: u32, event: KeyEvent) {
        self.on_key_event(event);
    }

    fn repeat_key(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_keyboard::WlKeyboard, _: u32, event: KeyEvent) {
        self.on_key_event(event);
    }

    fn release_key(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_keyboard::WlKeyboard, _: u32, _event: KeyEvent) {}

    fn update_modifiers(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_keyboard::WlKeyboard, _serial: u32, modifiers: Modifiers, _raw: RawModifiers, _layout: u32) {
        self.app.ctrl = modifiers.ctrl;
        self.app.shift = modifiers.shift;
        self.app.alt = modifiers.alt;
    }
}


fn axis_steps(a: &smithay_client_toolkit::seat::pointer::AxisScroll) -> Option<i32> {
    if a.discrete != 0 {
        return Some(a.discrete);
    }
    if a.value120 != 0 {
        let mut s = a.value120 / 120;
        if s == 0 {
            s = a.value120.signum();
        }
        return Some(s);
    }
    if a.absolute.abs() >= 1.0 {
        return Some(if a.absolute > 0.0 { 1 } else { -1 });
    }
    None
}

impl PointerHandler for Wl {
    fn pointer_frame(&mut self, conn: &Connection, qh: &QueueHandle<Self>, _pointer: &wl_pointer::WlPointer, events: &[PointerEvent]) {
        let _ = qh;
        let mut need_redraw = false;
        for event in events {
            if &event.surface != self.window.wl_surface() {
                continue;
            }
            let (x, y) = event.position;
            let (x, y) = (x as i32, y as i32);
            match event.kind {
                PointerEventKind::Enter { .. } => {
                    self.app.on_pointer_motion(x, y);
                    self.sync_cursor(conn);
                    need_redraw = true;
                }
                PointerEventKind::Leave { .. } => {
                    self.app.on_pointer_leave();
                    need_redraw = true;
                }
                PointerEventKind::Motion { .. } => {
                    self.app.on_pointer_motion(x, y);
                    self.maybe_start_drag(x, y);
                    self.sync_cursor(conn);
                    need_redraw = true;
                }
                PointerEventKind::Press { button, serial, time } => {
                    if button == 272 {
                        self.press = Some(PressState {
                            x,
                            y,
                            serial,
                            on_entry: self.app.on_entry(x, y),
                        });
                    }
                    self.app.on_pointer_button(x, y, time, button, true);
                    self.sync_cursor(conn);
                    need_redraw = true;
                }
                PointerEventKind::Release { button, time, .. } => {
                    if button == 272 {
                        self.press = None;
                        if !self.app.dnd_active && self.app.dnd_hover {
                            // A drag offer was finished over us without a drop_performed; clear.
                            self.app.dnd_clear();
                        }
                    }
                    self.app.on_pointer_button(x, y, time, button, false);
                    self.sync_cursor(conn);
                    need_redraw = true;
                }
                PointerEventKind::Axis { vertical, horizontal, .. } => {
                    // Wayland: positive vertical = scroll down (increase row offset).
                    let dy = axis_steps(&vertical).or_else(|| axis_steps(&horizontal)).unwrap_or(0);
                    if dy != 0 {
                        self.app.on_scroll(dy);
                        need_redraw = true;
                    }
                }
            }
        }
        if need_redraw {
            self.request_redraw();
        }
    }
}

impl ShmHandler for Wl {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}

impl Wl {
    fn desired_cursor(&self) -> CursorIcon {
        if self.app.side_resize
            || self.app.side_resize_hover
            || self.app.preview_resize
            || self.app.preview_resize_hover
            || self.app.split_resize
            || self.app.split_resize_hover
        {
            // Horizontal double-arrow: <->
            CursorIcon::ColResize
        } else {
            CursorIcon::Default
        }
    }

    fn sync_cursor(&mut self, conn: &Connection) {
        let icon = self.desired_cursor();
        if self.cursor_applied && icon == self.cursor_icon {
            return;
        }
        if let Some(ptr) = self.pointer.as_ref() {
            if ptr.set_cursor(conn, icon).is_ok() {
                self.cursor_icon = icon;
                self.cursor_applied = true;
            }
        }
    }

    fn on_key_event(&mut self, event: KeyEvent) {
        let sym = event.keysym;
        let utf8 = event.utf8.clone();
        self.app.on_key(sym, utf8, self.app.ctrl, self.app.shift);
        if self.app.dirty {
            self.request_redraw();
        }
    }

    /// After a left press on an entry, start a file drag once the pointer
    /// moves past a small threshold.
    fn maybe_start_drag(&mut self, x: i32, y: i32) {
        if self.app.dnd_active || self.app.menu.is_some() {
            return;
        }
        let Some(p) = self.press else { return };
        if !p.on_entry || self.app.ctrl {
            return;
        }
        let (dx, dy) = (x - p.x, y - p.y);
        if dx * dx + dy * dy < 36 {
            return;
        }
        self.start_file_drag(p.serial);
    }

    fn start_file_drag(&mut self, serial: u32) {
        let paths = self.app.dnd_paths();
        if paths.is_empty() {
            return;
        }
        let icon = self.make_drag_icon();
        let (Some(mgr), Some(device)) = (&self.data_device_manager, &self.data_device) else {
            return;
        };
        let source = mgr.create_drag_and_drop_source(
            &self.qh,
            vec!["text/uri-list".to_string(), "x-special/gnome-copied-files".to_string()],
            DndAction::Copy | DndAction::Move | DndAction::Ask,
        );
        source.start_drag(device, self.window.wl_surface(), icon.as_ref(), serial);
        self.app.dnd_active = true;
        self.app.dnd_hover = false;
        self.app.dnd_drop_dir = None;
        self.app.dirty = true;
        self.app.set_status(format!("Dragging {} item(s)", paths.len()));
        self.drag_sources.push(source);
        self.drag_surface = icon;
    }

    /// Small folder icon used as the drag-and-drop cursor.
    fn make_drag_icon(&mut self) -> Option<wl_surface::WlSurface> {
        let surface = self.compositor.create_surface(&self.qh);
        let s = 48i32;
        let mut pool = SlotPool::new(s as usize * s as usize * 4, &self.shm).ok()?;
        let (buffer, canvas) = pool.create_buffer(s, s, s * 4, wl_shm::Format::Argb8888).ok()?;
        canvas.fill(0);
        for yy in 0..s {
            for xx in 0..s {
                let i = (yy * s + xx) as usize * 4;
                let body = xx >= 8 && xx < 40 && yy >= 20 && yy < 42;
                let tab = xx >= 8 && xx < 28 && yy >= 12 && yy < 20;
                let edge = xx >= 7 && xx <= 40 && yy >= 11 && yy <= 42
                    && (xx == 7 || xx == 40 || yy == 11 || yy == 42);
                if body || tab {
                    let c = [0xfd, 0xb2, 0x8f, 0xff]; // BGRA: blue-ish folder
                    canvas[i..i + 4].copy_from_slice(&c);
                }
                if edge {
                    let c = [0xff, 0xff, 0xff, 0xff];
                    canvas[i..i + 4].copy_from_slice(&c);
                }
            }
        }
        buffer.attach_to(&surface).ok()?;
        surface.damage(0, 0, s, s);
        surface.commit();
        self.drag_pool = Some(pool);
        Some(surface)
    }
}

impl DataDeviceHandler for Wl {
    fn enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _device: &WlDataDevice,
        x: f64,
        y: f64,
        _surface: &wl_surface::WlSurface,
    ) {
        let Some(data_device) = &self.data_device else { return };
        let Some(offer) = data_device.data().drag_offer() else {
            // Internal drag (our own source): no drop highlight.
            self.app.dnd_hover = false;
            return;
        };
        if let Some(mime) = offer.with_mime_types(|mimes| pick_dnd_mime(mimes)) {
            offer.accept_mime_type(0, Some(mime));
        }
        offer.set_actions(DndAction::Copy | DndAction::Move | DndAction::Ask, DndAction::Copy);
        self.app.dnd_hover = true;
        self.app.dnd_x = x.max(0.0) as i32;
        self.app.dnd_y = y.max(0.0) as i32;
        self.app.dnd_side_hover =
            layout::sidebar_area_at(&self.app, self.app.dnd_x, self.app.dnd_y);
        self.app.dnd_update_target(self.app.dnd_x, self.app.dnd_y);
    }

    fn leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _device: &WlDataDevice,
    ) {
        self.app.dnd_hover = false;
        self.app.dnd_side_hover = false;
        self.app.dnd_drop_dir = None;
        self.app.dnd_hover_row = -1;
        self.app.dirty = true;
    }

    fn motion(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _device: &WlDataDevice,
        x: f64,
        y: f64,
    ) {
        self.app.dnd_x = x.max(0.0) as i32;
        self.app.dnd_y = y.max(0.0) as i32;
        self.app.dnd_side_hover =
            layout::sidebar_area_at(&self.app, self.app.dnd_x, self.app.dnd_y);
        self.app.dnd_update_target(self.app.dnd_x, self.app.dnd_y);
    }

    fn selection(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _device: &WlDataDevice,
    ) {
    }

    fn drop_performed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _device: &WlDataDevice,
    ) {
        let Some(data_device) = &self.data_device else { return };
        let Some(offer) = data_device.data().drag_offer() else { return };

        // A drop landing back in the window that started the drag is a
        // same-window drag (each wfm window is its own process, so
        // `dnd_active` is only set while *this* window owns the drag).
        // It is handled exactly like an external drop: on the file pane it
        // offers the Copy/Move/Link popup (dropping on a folder drops *into*
        // that folder), on the sidebar it adds shortcuts, anywhere else it is
        // cancelled.
        if self.app.dnd_active {
            let own = offer.clone();
            own.finish();
            own.destroy();
            let x = self.app.dnd_x.max(0);
            let y = self.app.dnd_y.max(0);
            let paths = self.app.dnd_paths();
            if layout::sidebar_area_at(&self.app, x, y) {
                self.app.add_shortcuts(paths);
                self.app.dnd_clear();
            } else if layout::dnd_popup_area_at(&self.app, x, y) {
                self.app.open_dnd_menu(x, y);
            } else {
                self.app.dnd_clear();
            }
            self.app.reload_status();
            self.app.dirty = true;
            self.request_redraw();
            return;
        }

        let Some(mime) = offer.with_mime_types(|mimes| pick_dnd_mime(mimes)) else {
            return;
        };
        let read_pipe = match offer.receive(mime.clone()) {
            Ok(pipe) => pipe,
            Err(_) => return,
        };
        offer.accept_mime_type(0, Some(mime.clone()));
        offer.set_actions(DndAction::Copy | DndAction::Move | DndAction::Ask, DndAction::Copy);
        let offer_id = offer.clone();
        self.app.dnd_uri_paths = Vec::new();
        self.dnd_offers.push((offer.clone(), Vec::new(), None));
        let idx = self.dnd_offers.len() - 1;
        let token = self.loop_handle.insert_source(read_pipe, move |_, f, state| {
            let Some(pos) = state.dnd_offers.iter().position(|(o, _, _)| *o == offer_id) else {
                return PostAction::Continue;
            };
            let (_, data, _) = state.dnd_offers.remove(pos);
            let mut data = data;
            // SAFETY: we only read from the pipe until it reaches EOF.
            let file: &mut std::fs::File = unsafe { f.get_mut() };
            use std::io::Read;
            let mut buf = [0u8; 8192];
            loop {
                match file.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => data.extend_from_slice(&buf[..n]),
                    Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                    Err(_) => break,
                }
            }
            let text = String::from_utf8_lossy(&data).to_string();
            let mut paths = Vec::new();
            for line in text.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                if let Some(p) = crate::fs::uri_to_path(line) {
                    paths.push(p);
                }
            }
            let x = state.app.dnd_x.max(0);
            let y = state.app.dnd_y.max(0);
            state.app.dnd_uri_paths = paths.clone();
            state.app.dnd_hover = false;
            state.app.dnd_side_hover = false;
            // The drop popup is only offered on the left (focused) file
            // pane. A drop on the sidebar (Places) adds the paths as
            // shortcuts — `add_shortcuts` keeps only valid directories /
            // symlinks-to-directories and cancels everything else in the
            // drop. Any drop on the right split pane, the preview pane or
            // window chrome is cancelled outright: nothing happens.
            if paths.is_empty() {
                state.app.set_status("no files in drop".into());
                state.app.dnd_clear();
            } else if layout::sidebar_area_at(&state.app, x, y) {
                state.app.add_shortcuts(paths);
                state.app.dnd_clear();
            } else if layout::dnd_popup_area_at(&state.app, x, y) {
                state.app.open_dnd_menu(x, y);
            } else {
                // right split pane / preview / chrome: cancelled
                state.app.dnd_clear();
            }
            offer_id.finish();
            offer_id.destroy();
            state.app.dirty = true;
            state.request_redraw();
            PostAction::Remove
        });
        if let Ok(token) = token {
            self.dnd_offers[idx].2 = Some(token);
        }
    }
}

impl DataOfferHandler for Wl {
    fn source_actions(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        offer: &mut DragOffer,
        _actions: DndAction,
    ) {
        offer.set_actions(DndAction::Copy | DndAction::Move | DndAction::Ask, DndAction::Copy);
    }

    fn selected_action(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _offer: &mut DragOffer,
        _action: DndAction,
    ) {
    }
}

impl DataSourceHandler for Wl {
    fn accept_mime(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _source: &WlDataSource,
        _mime: Option<String>,
    ) {
    }

    fn send_request(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        source: &WlDataSource,
        mime: String,
        write_pipe: WritePipe,
    ) {
        if !self.drag_sources.iter().any(|s| s.inner() == source) {
            return;
        }
        use std::io::Write;
        let mut f = std::fs::File::from(std::os::unix::io::OwnedFd::from(write_pipe));
        let paths = self.app.dnd_paths();
        if mime == "text/uri-list" {
            let mut out = String::new();
            for p in &paths {
                out.push_str(&crate::fs::path_to_uri(&p.display().to_string()));
                out.push_str("\r\n");
            }
            let _ = f.write_all(out.as_bytes());
        } else if mime == "x-special/gnome-copied-files" {
            let mut out = String::from("copy\n");
            for p in &paths {
                out.push_str(&crate::fs::path_to_uri(&p.display().to_string()));
                out.push('\n');
            }
            let _ = f.write_all(out.as_bytes());
        }
    }

    fn cancelled(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        source: &WlDataSource,
    ) {
        self.drag_sources.retain(|s| s.inner() != source);
        self.drag_surface = None;
        self.app.dnd_active = false;
        self.app.dnd_clear();
        source.destroy();
    }

    fn dnd_dropped(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _source: &WlDataSource,
    ) {
    }

    fn dnd_finished(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        source: &WlDataSource,
    ) {
        self.drag_sources.retain(|s| s.inner() != source);
        self.drag_surface = None;
        self.press = None;
        self.app.dnd_active = false;
        self.app.dnd_hover = false;
        // keep `dnd_drop_dir` alive: a same-window drop may have opened the
        // Copy/Move/Link popup, which still needs the drop target.
        // clear the lingering "Dragging N item(s)" status
        self.app.reload_status();
        self.app.dirty = true;
        source.destroy();
    }

    fn action(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _source: &WlDataSource,
        _action: DndAction,
    ) {
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
