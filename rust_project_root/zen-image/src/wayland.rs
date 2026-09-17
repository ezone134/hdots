use std::ffi::c_void;
use std::path::PathBuf;

use wayland_client::protocol::{
    wl_compositor, wl_keyboard, wl_pointer, wl_registry, wl_seat, wl_surface,
};
use wayland_client::protocol::{wl_pointer::WlPointer, wl_registry::WlRegistry};
use wayland_client::{Dispatch, Proxy, QueueHandle, WEnum};
use wayland_protocols::xdg::shell::client::{
    xdg_surface, xdg_toplevel, xdg_wm_base,
};

pub const MIN_W: u32 = 160;
pub const MIN_H: u32 = 120;

pub struct AppState {
    pub qh: QueueHandle<AppState>,
    pub conn: wayland_client::Connection,

    pub compositor: Option<wl_compositor::WlCompositor>,
    pub xdg_wm_base: Option<xdg_wm_base::XdgWmBase>,
    pub seat: Option<wl_seat::WlSeat>,

    pub surface: Option<wl_surface::WlSurface>,
    pub xdg_surface: Option<xdg_surface::XdgSurface>,
    pub toplevel: Option<xdg_toplevel::XdgToplevel>,
    pub configured: bool,

    pub win_w: u32,
    pub win_h: u32,
    pub target_w: u32,
    pub target_h: u32,

    pub img_w: u32,
    pub img_h: u32,

    pub paths: Vec<PathBuf>,
    pub cur: usize,
    pub want_next: bool,
    pub want_prev: bool,

    pub zoom: f64,
    pub pan_x: f64,
    pub pan_y: f64,

    pub exit: bool,
    pub needs_render: bool,
    pub resize_pending: bool,

    pub keyboard: Option<wl_keyboard::WlKeyboard>,
    pub dragging: bool,
    pub last_x: f64,
    pub last_y: f64,
}

impl AppState {
    pub fn new(
        conn: wayland_client::Connection,
        qh: QueueHandle<AppState>,
        img_w: u32,
        img_h: u32,
    ) -> Self {
        let (win_w, win_h) = Self::default_size(img_w, img_h);
        Self {
            qh,
            conn,
            compositor: None,
            xdg_wm_base: None,
            seat: None,
            surface: None,
            xdg_surface: None,
            toplevel: None,
            configured: false,
            win_w,
            win_h,
            target_w: win_w,
            target_h: win_h,
            img_w: img_w.max(1),
            img_h: img_h.max(1),
            paths: Vec::new(),
            cur: 0,
            want_next: false,
            want_prev: false,
            zoom: 1.0,
            pan_x: 0.0,
            pan_y: 0.0,
            exit: false,
            needs_render: true,
            resize_pending: true,
            keyboard: None,
            dragging: false,
            last_x: 0.0,
            last_y: 0.0,
        }
    }

    fn default_size(img_w: u32, img_h: u32) -> (u32, u32) {
        let iw = img_w.max(1) as f32;
        let ih = img_h.max(1) as f32;
        let scale = (1024.0 / iw).min(768.0 / ih).min(1.0);
        let w = (iw * scale).round().max(MIN_W as f32) as u32;
        let h = (ih * scale).round().max(MIN_H as f32) as u32;
        (w, h)
    }

    pub fn surface_ptrs(&self) -> (*mut c_void, *mut c_void) {
        let display = self.conn.backend().display_id().as_ptr() as *mut c_void;
        let surface = self.surface.as_ref().unwrap().id().as_ptr() as *mut c_void;
        (display, surface)
    }

    pub fn current_size(&self) -> (u32, u32) {
        (self.win_w.max(MIN_W), self.win_h.max(MIN_H))
    }

    pub fn create_window(&mut self) {
        let compositor = self.compositor.as_ref().expect("no compositor");
        let wm = self.xdg_wm_base.as_ref().expect("no xdg_wm_base");
        let surface = compositor.create_surface(&self.qh, ());
        let xdg_surface = wm.get_xdg_surface(&surface, &self.qh, ());
        let toplevel = xdg_surface.get_toplevel(&self.qh, ());
        toplevel.set_title("zen-image".to_string());
        toplevel.set_app_id("zen-image".to_string());
        surface.commit();

        self.surface = Some(surface);
        self.xdg_surface = Some(xdg_surface);
        self.toplevel = Some(toplevel);
    }

    pub fn view(&self, w: u32, h: u32) -> crate::vulkan::View {
        let w = w.max(1) as f32;
        let h = h.max(1) as f32;
        let iw = self.img_w as f32;
        let ih = self.img_h as f32;
        let fit = (w / iw).min(h / ih);
        let disp_w = iw * fit * self.zoom as f32;
        let disp_h = ih * fit * self.zoom as f32;
        crate::vulkan::View {
            scale: [disp_w / w, disp_h / h],
            offset: [self.pan_x as f32, self.pan_y as f32],
        }
    }

    pub fn reset_view(&mut self) {
        self.zoom = 1.0;
        self.pan_x = 0.0;
        self.pan_y = 0.0;
        self.needs_render = true;
    }

    pub fn command_image(&mut self, dir: i32) -> bool {
        let n = self.paths.len();
        if n <= 1 {
            return false;
        }
        self.cur =
            (((self.cur as i32 + dir) % n as i32 + n as i32) % n as i32) as usize;
        true
    }

    fn apply_configure(&mut self, w: i32, h: i32) {
        if w > 0 && h > 0 {
            self.target_w = w as u32;
            self.target_h = h as u32;
            self.resize_pending = true;
        }
    }
}

impl Dispatch<WlRegistry, ()> for AppState {
    fn event(
        state: &mut AppState,
        registry: &WlRegistry,
        event: wl_registry::Event,
        _data: &(),
        _conn: &wayland_client::Connection,
        qh: &QueueHandle<AppState>,
    ) {
        if let wl_registry::Event::Global { name, interface, version, .. } = event {
            match interface.as_str() {
                "wl_compositor" => {
                    let v = version.min(4);
                    let c = registry.bind::<wl_compositor::WlCompositor, _, _>(name, v, qh, ());
                    state.compositor = Some(c);
                }
                "wl_seat" => {
                    let v = version.min(7);
                    let s = registry.bind::<wl_seat::WlSeat, _, _>(name, v, qh, ());
                    state.seat = Some(s);
                }
                "xdg_wm_base" => {
                    let v = version.min(5);
                    let w = registry.bind::<xdg_wm_base::XdgWmBase, _, _>(name, v, qh, ());
                    state.xdg_wm_base = Some(w);
                }
                _ => {}
            }
        }
    }
}

impl Dispatch<xdg_wm_base::XdgWmBase, ()> for AppState {
    fn event(
        state: &mut AppState,
        _p: &xdg_wm_base::XdgWmBase,
        event: xdg_wm_base::Event,
        _data: &(),
        _conn: &wayland_client::Connection,
        _qh: &QueueHandle<AppState>,
    ) {
        if let xdg_wm_base::Event::Ping { serial } = event {
            if let Some(wm) = state.xdg_wm_base.as_ref() {
                wm.pong(serial);
            }
        }
    }
}

impl Dispatch<xdg_surface::XdgSurface, ()> for AppState {
    fn event(
        state: &mut AppState,
        proxy: &xdg_surface::XdgSurface,
        event: xdg_surface::Event,
        _data: &(),
        _conn: &wayland_client::Connection,
        _qh: &QueueHandle<AppState>,
    ) {
        match event {
            xdg_surface::Event::Configure { serial } => {
                proxy.ack_configure(serial);
                state.configured = true;
            }
            _ => {}
        }
    }
}

impl Dispatch<xdg_toplevel::XdgToplevel, ()> for AppState {
    fn event(
        state: &mut AppState,
        _proxy: &xdg_toplevel::XdgToplevel,
        event: xdg_toplevel::Event,
        _data: &(),
        _conn: &wayland_client::Connection,
        _qh: &QueueHandle<AppState>,
    ) {
        match event {
            xdg_toplevel::Event::Configure { width, height, .. } => {
                state.apply_configure(width, height);
            }
            xdg_toplevel::Event::Close => {
                state.exit = true;
            }
            _ => {}
        }
    }
}

impl Dispatch<wl_compositor::WlCompositor, ()> for AppState {
    fn event(
        _s: &mut AppState,
        _p: &wl_compositor::WlCompositor,
        _e: wl_compositor::Event,
        _d: &(),
        _c: &wayland_client::Connection,
        _q: &QueueHandle<AppState>,
    ) {
    }
}

impl Dispatch<wl_seat::WlSeat, ()> for AppState {
    fn event(
        state: &mut AppState,
        seat: &wl_seat::WlSeat,
        event: wl_seat::Event,
        _data: &(),
        _conn: &wayland_client::Connection,
        qh: &QueueHandle<AppState>,
    ) {
        if let wl_seat::Event::Capabilities {
            capabilities: WEnum::Value(capabilities),
        } = event
        {
            if capabilities.contains(wl_seat::Capability::Pointer) {
                let _p = seat.get_pointer(qh, ());
            }
            if capabilities.contains(wl_seat::Capability::Keyboard)
                && state.keyboard.is_none()
            {
                let k = seat.get_keyboard(qh, ());
                state.keyboard = Some(k);
            }
        }
    }
}

fn wl_f(p: impl Into<f64>) -> f64 {
    p.into()
}

impl Dispatch<WlPointer, ()> for AppState {
    fn event(
        state: &mut AppState,
        _proxy: &WlPointer,
        event: wl_pointer::Event,
        _data: &(),
        _conn: &wayland_client::Connection,
        _qh: &QueueHandle<AppState>,
    ) {
        match event {
            wl_pointer::Event::Enter { surface_x, surface_y, .. } => {
                state.last_x = wl_f(surface_x);
                state.last_y = wl_f(surface_y);
            }
            wl_pointer::Event::Motion { surface_x, surface_y, .. } => {
                let x = wl_f(surface_x);
                let y = wl_f(surface_y);
                if state.dragging {
                    let (w, h) = state.current_size();
                    let dx = (x - state.last_x) / w as f64 * 2.0;
                    let dy = (y - state.last_y) / h as f64 * 2.0;
                    state.pan_x += dx;
                    state.pan_y += dy;
                    state.needs_render = true;
                }
                state.last_x = x;
                state.last_y = y;
            }
            wl_pointer::Event::Leave { .. } => {
                state.dragging = false;
            }
            wl_pointer::Event::Button {
                button,
                state: WEnum::Value(bs),
                ..
            } => {
                if button == 272 {
                    let pressed = bs == wl_pointer::ButtonState::Pressed;
                    state.dragging = pressed;
                    if !pressed {
                        state.needs_render = true;
                    }
                }
            }
            wl_pointer::Event::Axis {
                axis: WEnum::Value(ax),
                value,
                ..
            } => {
                if ax == wl_pointer::Axis::VerticalScroll {
                    let delta = wl_f(value);
                    let factor = 1.15f64.powf(-(delta / 15.0));
                    state.zoom = (state.zoom * factor).clamp(0.01, 256.0);
                    state.needs_render = true;
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
        _conn: &wayland_client::Connection,
        _qh: &QueueHandle<AppState>,
    ) {
        match event {
            wl_keyboard::Event::Key {
                key,
                state: WEnum::Value(ks),
                ..
            } => {
                if ks != wl_keyboard::KeyState::Pressed {
                    return;
                }
                match key {
                    1 | 16 => state.exit = true, // Esc, Q (Hyprland evdev)
                    11 | 33 => state.reset_view(),    // 0, F
                    106 | 57 => state.want_next = true, // Right, N
                    105 | 55 => state.want_prev = true, // Left, P
                    _ => {}
                }
            }
            _ => {}
        }
    }
}

impl Dispatch<wl_surface::WlSurface, ()> for AppState {
    fn event(
        _s: &mut AppState,
        _p: &wl_surface::WlSurface,
        _e: wl_surface::Event,
        _d: &(),
        _c: &wayland_client::Connection,
        _q: &QueueHandle<AppState>,
    ) {
    }
}