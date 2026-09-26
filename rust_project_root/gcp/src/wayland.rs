use wayland_client::protocol::{
    wl_buffer, wl_callback, wl_compositor, wl_data_device, wl_data_device_manager, wl_data_offer,
    wl_data_source, wl_keyboard, wl_pointer, wl_registry, wl_seat, wl_shm, wl_shm_pool, wl_surface,
};
use wayland_client::protocol::{wl_data_device::WlDataDevice, wl_data_offer::WlDataOffer};
use wayland_client::{event_created_child, Connection, Dispatch, QueueHandle, WEnum};
use wayland_protocols_wlr::layer_shell::v1::client::{
    zwlr_layer_shell_v1, zwlr_layer_surface_v1,
};

use crate::color::{ColorState, ShmPool};

pub const WIDTH: i32 = 360;
pub const HEIGHT: i32 = 500;

pub const SLIDER_H: i32 = 8;
pub const SLIDER_MARGIN: i32 = 24;
pub const SLIDER_Y: i32 = 286;
pub const SLIDER_STRIDE: i32 = 32;

pub const BTN_COPY_LABEL: &str = "Copy";
pub const BTN_OK_LABEL: &str = "OK";
pub const BTN_PASTE_LABEL: &str = "Paste";

pub const BTN_Y: i32 = 448;
pub const BTN_H: i32 = 30;
pub const FIELD_Y: i32 = 404;
pub const FIELD_H: i32 = 26;

pub fn hex_field() -> (i32, i32, i32, i32) {
    (20, FIELD_Y, 146, FIELD_H)
}

pub fn btn_paste() -> (i32, i32, i32, i32) {
    (172, FIELD_Y, 76, FIELD_H)
}

pub fn btn_ok(_show_paste: bool) -> (i32, i32, i32, i32) {
    (20, BTN_Y, 112, BTN_H)
}

pub fn btn_copy(_show_paste: bool) -> (i32, i32, i32, i32) {
    (240, BTN_Y, 100, BTN_H)
}

pub fn btn_close() -> (i32, i32, i32, i32) {
    const CW: i32 = 72;
    (WIDTH - CW - 10, 8, CW, 22)
}

#[derive(Clone)]
pub struct ClipboardState {
    pub color: Option<ColorState>,
    pub revision: u64,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum DragTarget {
    None,
    Wheel,
    Slider(usize),
}

pub struct AppState {
    pub qh: QueueHandle<AppState>,

    pub compositor: Option<wl_compositor::WlCompositor>,
    pub shm: Option<wl_shm::WlShm>,
    pub seat: Option<wl_seat::WlSeat>,
    pub layer_shell: Option<zwlr_layer_shell_v1::ZwlrLayerShellV1>,

    pub surface: Option<wl_surface::WlSurface>,
    pub layer_surface: Option<zwlr_layer_surface_v1::ZwlrLayerSurfaceV1>,
    pub configured: bool,

    pub pool: Option<ShmPool>,
    pub buffers: [Option<wl_buffer::WlBuffer>; 2],
    pub frame_callback: Option<wl_callback::WlCallback>,

    pub color: ColorState,
    pub wheel: Vec<u32>,

    pub theme_fg: u32,
    pub theme_bg: u32,

    pub needs_render: bool,
    pub waiting_for_frame: bool,

    pub hot: bool,

    pub pointer: Option<wl_pointer::WlPointer>,
    pub keyboard: Option<wl_keyboard::WlKeyboard>,
    pub exit: bool,
    pub output_hex: bool,

    pub data_device_manager: Option<wl_data_device_manager::WlDataDeviceManager>,
    pub data_device: Option<wl_data_device::WlDataDevice>,
    pub data_source: Option<wl_data_source::WlDataSource>,
    pub pointer_serial: u32,
    pub keyboard_serial: u32,
    pub pointer_sx: f64,
    pub pointer_sy: f64,
    pub ctrl: bool,
    pub active: DragTarget,

    pub clipboard: std::sync::Arc<std::sync::Mutex<ClipboardState>>,
    pub seen_clip_rev: u64,
    pub show_paste: bool,
    pub paste_requested: bool,
    pub hex_input: String,
    pub backspace_held: bool,
    pub backspace_down: Option<std::time::Instant>,
    pub backspace_last: std::time::Instant,
    pub copy_flash_until: Option<std::time::Instant>,
}

impl AppState {
    pub fn new(_conn: Connection, qh: QueueHandle<AppState>) -> Self {
        Self {
            qh,
            compositor: None,
            shm: None,
            seat: None,
            layer_shell: None,
            surface: None,
            layer_surface: None,
            configured: false,
            pool: None,
            buffers: [None, None],
            frame_callback: None,
            color: ColorState::default(),
            wheel: crate::color::generate_wheel(),
            theme_fg: 0xFFF8F8F2,
            theme_bg: 0xFF282A36,
            needs_render: false,
            waiting_for_frame: false,
            hot: false,
            pointer: None,
            keyboard: None,
            exit: false,
            output_hex: true,
            data_device_manager: None,
            data_device: None,
            data_source: None,
            pointer_serial: 0,
            keyboard_serial: 0,
            pointer_sx: 0.0,
            pointer_sy: 0.0,
            ctrl: false,
            active: DragTarget::None,
            clipboard: std::sync::Arc::new(std::sync::Mutex::new(ClipboardState {
                color: None,
                revision: 0,
            })),
            seen_clip_rev: 0,
            show_paste: false,
            paste_requested: false,
            hex_input: ColorState::default().hex().trim_start_matches('#').to_string(),
            backspace_held: false,
            backspace_down: None,
            backspace_last: std::time::Instant::now(),
            copy_flash_until: None,
        }
    }

    pub fn surface_ready(&self) -> bool {
        self.surface.is_some() && self.layer_surface.is_some() && self.configured
    }

    pub fn copy_flash_active(&self) -> bool {
        matches!(self.copy_flash_until, Some(t) if t > std::time::Instant::now())
    }

    pub fn center(&self) -> (i32, i32) {
        (WIDTH / 2, 140)
    }

    pub fn wheel_radius(&self) -> f32 {
        (WIDTH / 2) as f32 - 30.0
    }

    pub fn slider_rect(&self, i: usize) -> (i32, i32, i32, i32) {
        let w = WIDTH - 30 - SLIDER_MARGIN;
        let y = SLIDER_Y + (i as i32) * SLIDER_STRIDE;
        (30, y, w, SLIDER_H)
    }

    fn btn_hit(&self, x: f64, y: f64, r: (i32, i32, i32, i32)) -> bool {
        let (bx, by, bw, bh) = r;
        x >= bx as f64 && x < (bx + bw) as f64 && y >= by as f64 && y < (by + bh) as f64
    }

    fn target_at(&self, x: f64, y: f64) -> DragTarget {
        if self.btn_hit(x, y, btn_close()) || self.btn_hit(x, y, btn_copy(self.show_paste)) {
            return DragTarget::None;
        }
        if self.btn_hit(x, y, btn_ok(self.show_paste)) {
            return DragTarget::None;
        }
        if self.btn_hit(x, y, btn_paste()) {
            return DragTarget::None;
        }

        let cx = self.center().0 as f64;
        let cy = self.center().1 as f64;
        let radius = self.wheel_radius() as f64;
        let dx = x - cx;
        let dy = y - cy;
        if (dx * dx + dy * dy).sqrt() <= radius + 12.0 {
            return DragTarget::Wheel;
        }

        for i in 0..4 {
            let (sx, sy, sw, sh) = self.slider_rect(i);
            if x >= sx as f64
                && x < (sx + sw) as f64
                && y >= (sy - 4) as f64
                && y < (sy + sh + 4) as f64
            {
                return DragTarget::Slider(i);
            }
        }
        DragTarget::None
    }

    fn apply_wheel(&mut self, surf_x: f64, surf_y: f64) {
        let cx = self.center().0 as f64;
        let cy = self.center().1 as f64;
        let radius = self.wheel_radius() as f64;

        let dx = surf_x - cx;
        let dy = surf_y - cy;
        let hue = (dy.atan2(dx).to_degrees() + 360.0) % 360.0;
        let sat = ((dx * dx + dy * dy).sqrt() / radius).clamp(0.0, 1.0);
        self.color.hue = hue as f32;
        self.color.sat = sat as f32;
        self.sync_hex_input();
        self.needs_render = true;
    }

    fn apply_slider(&mut self, i: usize, surf_x: f64) {
        let (sx, _, sw, _) = self.slider_rect(i);
        let frac = ((surf_x - sx as f64) / sw as f64).clamp(0.0, 1.0);
        match i {
            0 => self.color.val = frac as f32,
            1 => {
                let (_, g, b) = self.color.rgb();
                let r = (frac * 255.0) as u8;
                let (h, s, v) = crate::color::rgb_to_hsv(r, g, b);
                self.color = ColorState { hue: h, sat: s, val: v, alpha: self.color.alpha };
            }
            2 => {
                let (r, _, b) = self.color.rgb();
                let g = (frac * 255.0) as u8;
                let (h, s, v) = crate::color::rgb_to_hsv(r, g, b);
                self.color = ColorState { hue: h, sat: s, val: v, alpha: self.color.alpha };
            }
            _ => {
                let (r, g, _) = self.color.rgb();
                let b = (frac * 255.0) as u8;
                let (h, s, v) = crate::color::rgb_to_hsv(r, g, b);
                self.color = ColorState { hue: h, sat: s, val: v, alpha: self.color.alpha };
            }
        }
        self.needs_render = true;
    }

    pub fn sync_hex_input(&mut self) {
        self.hex_input = self.color.hex().trim_start_matches('#').to_string();
    }

    pub fn type_hex_digit(&mut self, ch: char) {
        if self.hex_input.len() >= 6 {
            return;
        }
        self.hex_input.push(ch);
        self.apply_hex_input();
    }

    pub fn backspace_hex(&mut self) {
        if self.hex_input.pop().is_some() {
            self.apply_hex_input();
        }
    }

    pub fn apply_hex_input(&mut self) {
        self.hex_input = self.hex_input.to_uppercase();
        if let Some(t) = crate::color::parse_hex(&format!("#{}", self.hex_input)) {
            if t.alpha == 255 {
                self.color = ColorState {
                    hue: t.hue,
                    sat: t.sat,
                    val: t.val,
                    alpha: self.color.alpha,
                };
            }
        }
        self.needs_render = true;
    }

    pub fn apply_pointer(&mut self, surf_x: f64, surf_y: f64) {
        match self.active {
            DragTarget::Wheel => self.apply_wheel(surf_x, surf_y),
            DragTarget::Slider(i) => self.apply_slider(i, surf_x),
            DragTarget::None => {}
        }
    }

    pub fn handle_click(&mut self, x: f64, y: f64) {
        let (bx, by, bw, bh) = btn_close();
        if x >= bx as f64 && x < (bx + bw) as f64 && y >= by as f64 && y < (by + bh) as f64 {
            self.exit = true;
            self.output_hex = false;
            return;
        }
        let (bx, by, bw, bh) = btn_copy(self.show_paste);
        if x >= bx as f64 && x < (bx + bw) as f64 && y >= by as f64 && y < (by + bh) as f64 {
            self.copy_hex(self.pointer_serial);
            return;
        }
        let (bx, by, bw, bh) = btn_ok(self.show_paste);
        if x >= bx as f64 && x < (bx + bw) as f64 && y >= by as f64 && y < (by + bh) as f64 {
            self.exit = true;
            return;
        }
        if self.show_paste {
            let (bx, by, bw, bh) = btn_paste();
            if x >= bx as f64 && x < (bx + bw) as f64 && y >= by as f64 && y < (by + bh) as f64 {
                self.paste_request();
            }
        }
    }

    pub fn copy_hex(&mut self, serial: u32) {
        let Some(device) = self.data_device.as_ref() else { return };
        let Some(mgr) = self.data_device_manager.clone() else { return };
        let src = mgr.create_data_source(&self.qh, ());
        src.offer("text/plain;charset=utf-8".to_string());
        src.offer("text/plain".to_string());
        device.set_selection(Some(&src), serial);
        self.data_source = Some(src);
        self.copy_flash_until = Some(std::time::Instant::now() + std::time::Duration::from_secs(3));
        self.needs_render = true;
    }

    pub fn paste_request(&mut self) {
        self.paste_requested = true;
    }

    pub fn sync_clipboard(&mut self) {
        let (color, rev) = {
            let s = self.clipboard.lock().unwrap();
            (s.color.clone(), s.revision)
        };
        let show = color.is_some();
        if rev != self.seen_clip_rev || show != self.show_paste {
            self.seen_clip_rev = rev;
            if show != self.show_paste {
                self.show_paste = show;
                self.needs_render = true;
            }
        }
        if self.paste_requested {
            self.paste_requested = false;
            if let Some(c) = color {
                self.color = c;
                self.sync_hex_input();
                self.needs_render = true;
            }
        }
    }

    pub fn snapshot_clipboard(&mut self) {
        let parsed = read_clipboard_hex();
        {
            let mut s = self.clipboard.lock().unwrap();
            if s.color != parsed {
                s.color = parsed;
                s.revision = s.revision.wrapping_add(1);
            }
        }
        self.sync_clipboard();
    }
}

pub fn clipboard_loop(slot: std::sync::Arc<std::sync::Mutex<ClipboardState>>) {
    loop {
        let parsed = read_clipboard_hex();
        let mut s = slot.lock().unwrap();
        if s.color != parsed {
            s.color = parsed;
            s.revision += 1;
        }
        drop(s);
        std::thread::sleep(std::time::Duration::from_secs(2));
    }
}

pub fn read_clipboard_hex() -> Option<ColorState> {
    let out = std::process::Command::new("wl-paste")
        .args(["--no-newline"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    parse_clipboard_hex(&String::from_utf8_lossy(&out.stdout))
}

fn parse_clipboard_hex(text: &str) -> Option<ColorState> {
    let raw = text.trim();
    let has_hash = raw.starts_with('#');
    let digits = raw.trim_start_matches('#');
    if !has_hash && digits.len() != 6 {
        return None;
    }
    let mut c = crate::color::parse_hex(raw)?;
    c.alpha = 255;
    Some(c)
}

impl Dispatch<wl_registry::WlRegistry, ()> for AppState {
    fn event(
        state: &mut AppState,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _data: &(),
        _conn: &Connection,
        qh: &QueueHandle<AppState>,
    ) {
        if let wl_registry::Event::Global { name, interface, version, .. } = event {
            match interface.as_str() {
                "wl_compositor" => {
                    let v = version.min(4);
                    let c = registry.bind::<wl_compositor::WlCompositor, _, _>(name, v, qh, ());
                    state.compositor = Some(c);
                }
                "wl_shm" => {
                    let v = version.min(1);
                    let s = registry.bind::<wl_shm::WlShm, _, _>(name, v, qh, ());
                    state.shm = Some(s);
                }
                "wl_seat" => {
                    let v = version.min(7);
                    let s = registry.bind::<wl_seat::WlSeat, _, _>(name, v, qh, ());
                    state.seat = Some(s);
                }
                "zwlr_layer_shell_v1" => {
                    let v = version.min(4);
                    let l =
                        registry.bind::<zwlr_layer_shell_v1::ZwlrLayerShellV1, _, _>(name, v, qh, ());
                    state.layer_shell = Some(l);
                }
                "wl_data_device_manager" => {
                    let v = version.min(3);
                    let m = registry.bind::<wl_data_device_manager::WlDataDeviceManager, _, _>(name, v, qh, ());
                    state.data_device_manager = Some(m);
                }
                _ => {}
            }
        }
    }
}

impl Dispatch<zwlr_layer_surface_v1::ZwlrLayerSurfaceV1, ()> for AppState {
    fn event(
        state: &mut AppState,
        layer_surface: &zwlr_layer_surface_v1::ZwlrLayerSurfaceV1,
        event: zwlr_layer_surface_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<AppState>,
    ) {
        match event {
            zwlr_layer_surface_v1::Event::Configure { serial, .. } => {
                layer_surface.ack_configure(serial);
                state.configured = true;
                state.needs_render = true;
            }
            zwlr_layer_surface_v1::Event::Closed => {
                state.exit = true;
                state.output_hex = false;
            }
            _ => {}
        }
    }
}

impl Dispatch<wl_seat::WlSeat, ()> for AppState {
    fn event(
        state: &mut AppState,
        seat: &wl_seat::WlSeat,
        event: wl_seat::Event,
        _data: &(),
        _conn: &Connection,
        qh: &QueueHandle<AppState>,
    ) {
        if let wl_seat::Event::Capabilities { capabilities: WEnum::Value(capabilities) } = event {
            if capabilities.contains(wl_seat::Capability::Pointer)
                && state.pointer.is_none()
            {
                let p = seat.get_pointer(qh, ());
                state.pointer = Some(p);
            }
            if capabilities.contains(wl_seat::Capability::Keyboard)
                && state.keyboard.is_none()
            {
                let k = seat.get_keyboard(qh, ());
                state.keyboard = Some(k);
            }
            if state.data_device.is_none() {
                if let Some(mgr) = state.data_device_manager.as_ref() {
                    let dd = mgr.get_data_device(seat, qh, ());
                    state.data_device = Some(dd);
                }
            }
        }
    }
}

impl Dispatch<wl_pointer::WlPointer, ()> for AppState {
    fn event(
        state: &mut AppState,
        _proxy: &wl_pointer::WlPointer,
        event: wl_pointer::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<AppState>,
    ) {
        match event {
            wl_pointer::Event::Leave { .. } => {
                state.hot = false;
                state.active = DragTarget::None;
            }
            wl_pointer::Event::Motion { surface_x, surface_y, .. } => {
                state.pointer_sx = surface_x;
                state.pointer_sy = surface_y;
                if state.hot && state.active != DragTarget::None {
                    state.apply_pointer(surface_x, surface_y);
                }
            }
            wl_pointer::Event::Button {
                button,
                state: WEnum::Value(bs),
                serial,
                ..
            } => {
                let pressed = bs == wl_pointer::ButtonState::Pressed;
                if pressed && button == 272 {
                    state.pointer_serial = serial;
                    state.hot = true;
                    state.active = state.target_at(state.pointer_sx, state.pointer_sy);
                    if state.active != DragTarget::None {
                        state.apply_pointer(state.pointer_sx, state.pointer_sy);
                    }
                    state.handle_click(state.pointer_sx, state.pointer_sy);
                } else if !pressed && button == 272 {
                    state.hot = false;
                    state.active = DragTarget::None;
                }
            }
            _ => {}
        }
    }
}

impl Dispatch<wl_keyboard::WlKeyboard, ()> for AppState {
    fn event(
        state: &mut AppState,
        _proxy: &wl_keyboard::WlKeyboard,
        event: wl_keyboard::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<AppState>,
    ) {
        match event {
            wl_keyboard::Event::Modifiers {
                mods_depressed, ..
            } => {
                state.ctrl = (mods_depressed & 4) != 0;
            }
            wl_keyboard::Event::Key {
                key,
                state: WEnum::Value(ks),
                serial,
                ..
            } => {
                state.keyboard_serial = serial;
                // Hyprland delivers raw evdev keycodes:
                // Esc=1 Enter=28 Backspace=14 c=54 v=55
                if key == 14 {
                    if ks == wl_keyboard::KeyState::Pressed {
                        state.backspace_hex();
                        state.backspace_held = true;
                        let now = std::time::Instant::now();
                        state.backspace_down = Some(now);
                        state.backspace_last = now;
                    } else {
                        state.backspace_held = false;
                        state.backspace_down = None;
                    }
                    return;
                }
                if ks != wl_keyboard::KeyState::Pressed {
                    return;
                }
                if key == 1 {
                    state.exit = true;
                    state.output_hex = false;
                    return;
                }
                if key == 28 {
                    state.exit = true;
                    return;
                }
                if state.ctrl {
                    match key {
                        54 => state.copy_hex(serial),
                        55 => state.paste_request(),
                        _ => {}
                    }
                } else if let Some(ch) = key_to_hex(key) {
                    state.type_hex_digit(ch);
                }
            }
            _ => {}
        }
    }
}

fn key_to_hex(key: u32) -> Option<char> {
    let ch = match key {
        11 => '0',
        2 => '1',
        3 => '2',
        4 => '3',
        5 => '4',
        6 => '5',
        7 => '6',
        8 => '7',
        9 => '8',
        10 => '9',
        30 => 'a',
        48 => 'b',
        46 => 'c',
        32 => 'd',
        18 => 'e',
        33 => 'f',
        _ => return None,
    };
    Some(ch)
}

impl Dispatch<wl_surface::WlSurface, ()> for AppState {
    fn event(
        _state: &mut AppState,
        _proxy: &wl_surface::WlSurface,
        _event: wl_surface::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<AppState>,
    ) {
    }
}

impl Dispatch<wl_callback::WlCallback, ()> for AppState {
    fn event(
        state: &mut AppState,
        _proxy: &wl_callback::WlCallback,
        _event: wl_callback::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<AppState>,
    ) {
        state.waiting_for_frame = false;
        state.frame_callback = None;
    }
}

impl Dispatch<wl_data_device::WlDataDevice, ()> for AppState {
    event_created_child!(AppState, WlDataDevice, [
        wl_data_device::EVT_DATA_OFFER_OPCODE => (WlDataOffer, ()),
    ]);
    fn event(
        _state: &mut AppState,
        _proxy: &wl_data_device::WlDataDevice,
        _event: wl_data_device::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<AppState>,
    ) {
    }
}

impl Dispatch<wl_data_offer::WlDataOffer, ()> for AppState {
    fn event(
        _state: &mut AppState,
        _proxy: &wl_data_offer::WlDataOffer,
        _event: wl_data_offer::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<AppState>,
    ) {
    }
}

impl Dispatch<wl_data_source::WlDataSource, ()> for AppState {
    fn event(
        state: &mut AppState,
        _proxy: &wl_data_source::WlDataSource,
        event: wl_data_source::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<AppState>,
    ) {
        match event {
            wl_data_source::Event::Send { fd, .. } => {
                use std::io::Write;
                use std::os::fd::{FromRawFd, IntoRawFd};
                let hex = state.color.hex();
                let data = format!("{}\n", hex);
                let mut f = std::io::BufWriter::new(unsafe {
                    std::fs::File::from_raw_fd(fd.into_raw_fd())
                });
                let _ = f.write_all(data.as_bytes());
                let _ = f.flush();
            }
            wl_data_source::Event::Cancelled => {
                if let Some(cur) = state.data_source.clone() {
                    if &cur == _proxy {
                        state.data_source = None;
                    }
                }
                _proxy.destroy();
            }
            _ => {}
        }
    }
}

// Dead simple no-op handlers for objects that never emit events.
macro_rules! noop {
    ($t:ty) => {
        impl Dispatch<$t, ()> for AppState {
            fn event(
                _s: &mut AppState,
                _p: &$t,
                _e: <$t as wayland_client::Proxy>::Event,
                _d: &(),
                _c: &Connection,
                _q: &QueueHandle<AppState>,
            ) {
            }
        }
    };
}
noop!(wl_compositor::WlCompositor);
noop!(wl_shm::WlShm);
noop!(wl_shm_pool::WlShmPool);
noop!(wl_buffer::WlBuffer);
noop!(wl_data_device_manager::WlDataDeviceManager);
noop!(zwlr_layer_shell_v1::ZwlrLayerShellV1);
