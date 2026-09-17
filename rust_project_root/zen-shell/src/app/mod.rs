//! App state + wayland handlers.
//!
//! 0-CPU discipline (learned the hard way in the C++ rewrite):
//!  - draw ONLY on configure acks / explicit dirt — never on a free-running timer
//!  - size changes are committed ALONE (no buffer attach in the same commit)
//!  - flush the socket right after queueing requests
//!  - the wayland fd is the only thing epoll sleeps on when idle

use crate::config::Config;
use crate::hypr::Hypr;
use crate::icons::*;
use crate::ipc;
use crate::weather;
use crate::render::{Gl, Tex};
use crate::tray::{self, TrayCmd, TrayMsg};
use crate::polkit::{self, PolkitCmd, PolkitMsg};
#[cfg(feature = "vk")]
use crate::render_vk::Vk;
use crate::services::Services;
use crate::shell::{Cmd, Shell};
use std::io::Write;
use crate::text::TextEngine;

mod handlers;
mod input;
mod ipc_srv;
mod fetcher;
mod session;
mod surfaces;

use surfaces::{AuxSurface, sig_handler};
use session::{LockSurface, LockAuthMsg, PolkitAuthMsg};


use smithay_client_toolkit::compositor::{CompositorHandler, CompositorState};
use smithay_client_toolkit::output::{OutputHandler, OutputState};
use smithay_client_toolkit::registry::{ProvidesRegistryState, RegistryState};
use smithay_client_toolkit::reexports::calloop;
use smithay_client_toolkit::reexports::calloop::LoopHandle;
use smithay_client_toolkit::reexports::calloop_wayland_source::WaylandSource;
use smithay_client_toolkit::seat::keyboard::{KeyboardHandler, KeyEvent, Keysym};
use smithay_client_toolkit::seat::pointer::{PointerEvent, PointerEventKind, PointerHandler};
use smithay_client_toolkit::seat::{Capability, SeatHandler, SeatState};
use smithay_client_toolkit::session_lock::{
    SessionLock, SessionLockHandler, SessionLockState, SessionLockSurface,
    SessionLockSurfaceConfigure,
};
use smithay_client_toolkit::shell::wlr_layer::{
    Anchor, KeyboardInteractivity, Layer, LayerShell, LayerShellHandler, LayerSurface,
    LayerSurfaceConfigure,
};
use smithay_client_toolkit::shell::WaylandSurface;
use smithay_client_toolkit::{delegate_registry};

use wayland_client::globals::registry_queue_init;
use wayland_client::protocol::{wl_keyboard, wl_output, wl_pointer, wl_seat, wl_surface};
use wayland_client::{Connection, Proxy, QueueHandle};
use wayland_protocols::wp::pointer_gestures::zv1::client::zwp_pointer_gesture_pinch_v1::ZwpPointerGesturePinchV1;
use wayland_protocols::wp::pointer_gestures::zv1::client::zwp_pointer_gestures_v1::ZwpPointerGesturesV1;
use wayland_protocols_wlr::data_control::v1::client::zwlr_data_control_device_v1::ZwlrDataControlDeviceV1;
use wayland_protocols_wlr::data_control::v1::client::zwlr_data_control_manager_v1::ZwlrDataControlManagerV1;
use wayland_protocols_wlr::data_control::v1::client::zwlr_data_control_offer_v1::ZwlrDataControlOfferV1;
use wayland_protocols_wlr::data_control::v1::client::zwlr_data_control_source_v1::ZwlrDataControlSourceV1;

use wayland_egl::WlEglSurface;

use std::ffi::c_void;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// GL or Vulkan — same scene graph, same `Cmd` dispatch.
/// Vulkan is opt-in via `ZEN_VULKAN=1`; GL stays the default (proven path).
#[allow(clippy::large_enum_variant)] // one instance, ever (App::renderer)
enum Renderer {
    Gl(Gl),
    #[cfg(feature = "vk")]
    Vk(Vk),
}

impl Renderer {
    /// Re-assert the EGL current surface before drawing (the aux surfaces'
    /// GL instances switch the shared display's current surface between
    /// frames). Vulkan needs no equivalent.
    fn make_current(&mut self) {
        match self {
            Renderer::Gl(g) => g.make_current(),
            #[cfg(feature = "vk")]
            Renderer::Vk(_) => {}
        }
    }
    fn resize(&mut self, w: i32, h: i32) {
        match self {
            Renderer::Gl(g) => g.resize(w, h),
            #[cfg(feature = "vk")]
            Renderer::Vk(v) => v.resize(w, h),
        }
    }
    fn begin_frame(&mut self) {
        match self {
            Renderer::Gl(g) => g.begin_frame(),
            #[cfg(feature = "vk")]
            Renderer::Vk(v) => v.begin_frame(),
        }
    }
    fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, r: f32, color: u32) {
        match self {
            Renderer::Gl(g) => g.rect(x, y, w, h, r, color),
            #[cfg(feature = "vk")]
            Renderer::Vk(v) => v.rect(x, y, w, h, r, color),
        }
    }
    #[allow(clippy::too_many_arguments)]
    fn outline(&mut self, x: f32, y: f32, w: f32, h: f32, r: f32, width: f32, color: u32) {
        match self {
            Renderer::Gl(g) => g.rect_ex(x, y, w, h, r, width, color),
            #[cfg(feature = "vk")]
            Renderer::Vk(v) => v.rect_ex(x, y, w, h, r, width, color),
        }
    }
    /// Rounded rect with per-corner radii — negative = concave.
    #[allow(clippy::too_many_arguments)]
    fn rect_concave(&mut self, x: f32, y: f32, w: f32, h: f32, r_tl: f32, r_tr: f32, r_br: f32, r_bl: f32, color: u32) {
        match self {
            Renderer::Gl(g) => g.rect_concave(x, y, w, h, r_tl, r_tr, r_br, r_bl, color),
            #[cfg(feature = "vk")]
            Renderer::Vk(v) => v.rect_concave(x, y, w, h, r_tl, r_tr, r_br, r_bl, color),
        }
    }
    /// Thick capsule segment (heartbeat graph lines).
    #[allow(clippy::too_many_arguments)]
    fn line(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, t: f32, color: u32) {
        match self {
            Renderer::Gl(g) => g.line(x0, y0, x1, y1, t, color),
            #[cfg(feature = "vk")]
            Renderer::Vk(v) => v.line(x0, y0, x1, y1, t, color),
        }
    }
    fn text_quad(&mut self, t: &Tex, x: f32, y: f32, w: f32, h: f32) {
        match self {
            Renderer::Gl(g) => g.text_quad(t, x, y, w, h),
            #[cfg(feature = "vk")]
            Renderer::Vk(v) => v.text_quad(t, x, y, w, h),
        }
    }
    fn text_quad_tinted(&mut self, t: &Tex, x: f32, y: f32, w: f32, h: f32, tint: u32) {
        match self {
            Renderer::Gl(g) => g.text_quad_tinted(t, x, y, w, h, tint),
            #[cfg(feature = "vk")]
            Renderer::Vk(v) => v.text_quad_tinted(t, x, y, w, h, tint),
        }
    }
    /// UV-windowed textured quad (world-map band). GL implements it; the
    /// optional Vulkan backend falls back to full-texture sampling (degraded).
    #[allow(clippy::too_many_arguments)]
    fn text_quad_uv(&mut self, t: &Tex, x: f32, y: f32, w: f32, h: f32, u0: f32, v0: f32, u1: f32, v1: f32) {
        match self {
            Renderer::Gl(g) => g.text_quad_uv(t, x, y, w, h, u0, v0, u1, v1),
            #[cfg(feature = "vk")]
            Renderer::Vk(v) => v.text_quad_uv(t, x, y, w, h, u0, v0, u1, v1),
        }
    }
    fn upload_texture(&mut self, w: u32, h: u32, px: &[u8]) -> Result<Tex, String> {
        match self {
            Renderer::Gl(g) => g.upload_texture(w, h, px),
            #[cfg(feature = "vk")]
            Renderer::Vk(v) => v.upload_texture(w, h, px),
        }
    }
    /// World-band texture: LINEAR filtering so stretched pan-zoom stays soft
    /// instead of blocky-pixelated (GL); VK mirrors GL.
    fn upload_texture_linear(&mut self, w: u32, h: u32, px: &[u8]) -> Result<Tex, String> {
        match self {
            Renderer::Gl(g) => g.upload_texture_linear(w, h, px),
            #[cfg(feature = "vk")]
            Renderer::Vk(v) => v.upload_texture_linear(w, h, px),
        }
    }
    fn flush(&mut self) {
        match self {
            Renderer::Gl(g) => g.flush(),
            #[cfg(feature = "vk")]
            Renderer::Vk(v) => v.flush(),
        }
    }
    /// Clip geometry to a rect (scene coords). GL uses the scissor test; the
    /// optional Vulkan backend ignores it (content overflows, degraded only).
    fn scissor(&mut self, x: f32, y: f32, w: f32, h: f32) {
        match self {
            Renderer::Gl(g) => g.set_scissor(x, y, w, h),
            #[cfg(feature = "vk")]
            Renderer::Vk(_) => {}
        }
    }
    /// Lift the active scissor — drawing continues unclipped.
    fn scissor_off(&mut self) {
        match self {
            Renderer::Gl(g) => g.clear_scissor(),
            #[cfg(feature = "vk")]
            Renderer::Vk(_) => {}
        }
    }
    fn cache_contains(&self, key: &(String, u32, u32, bool, u8, u8)) -> bool {
        match self {
            Renderer::Gl(g) => g.text_contains(key),
            #[cfg(feature = "vk")]
            Renderer::Vk(v) => v.text_contains(key),
        }
    }
    fn cache_insert(&mut self, key: (String, u32, u32, bool, u8, u8), t: Tex) {
        match self {
            Renderer::Gl(g) => g.text_insert(key, t),
            #[cfg(feature = "vk")]
            Renderer::Vk(v) => v.text_insert(key, t),
        };
    }
    fn cache_get(&mut self, key: &(String, u32, u32, bool, u8, u8)) -> Option<Tex> {
        match self {
            Renderer::Gl(g) => g.text_get(key),
            #[cfg(feature = "vk")]
            Renderer::Vk(v) => v.text_get(key),
        }
    }
    fn img_contains(&self, key: &str) -> bool {
        match self {
            Renderer::Gl(g) => g.img_contains(key),
            #[cfg(feature = "vk")]
            Renderer::Vk(v) => v.img_contains(key),
        }
    }
    fn img_insert(&mut self, key: String, t: Tex) {
        match self {
            Renderer::Gl(g) => g.img_insert(key, t),
            #[cfg(feature = "vk")]
            Renderer::Vk(v) => v.img_insert(key, t),
        }
    }
    fn img_get(&mut self, key: &str) -> Option<Tex> {
        match self {
            Renderer::Gl(g) => g.img_get(key),
            #[cfg(feature = "vk")]
            Renderer::Vk(v) => v.img_get(key),
        }
    }
    /// Drop an uploaded image texture so the next draw re-uploads the latest
    /// `ImageStore` pixels under that key (live camera frames).
    fn img_remove(&mut self, key: &str) {
        match self {
            Renderer::Gl(g) => g.img_remove(key),
            #[cfg(feature = "vk")]
            Renderer::Vk(v) => v.img_remove(key),
        }
    }
}

struct FrameStats {
    /// timestamp of the last frame-report (when we last printed fps)
    last: Instant,
    /// frames rendered since the last report
    frames: u32,
    /// Exponentially-smoothed frames-per-second over recent frames
    fps: f32,
    /// scene primitives drawn this frame (non-textured)
    solids: u32,
    /// textured quads drawn this frame (text + images)
    textured: u32,
    /// texture uploads issued this frame (glyphs/images rasterized on demand)
    uploads: u32,
}

/// A baked world-map band: the texture + the bake-window it covers.
struct WorldBand {
    tex: Tex,
    /// parameters the raster was made at (style, bin, colors, region, version)
    key: (u8, u32, [u32; 6], bool, u64),
    /// world-coords rect (lon0,lat0,lon1,lat1) — the actual texture window
    win: (f32, f32, f32, f32),
    /// the bake window (lon0,lat0,lon1,lat1); view must stay inside it
    window: (f32, f32, f32, f32),
}

/// World-map bake window: 2× the current view, centered, clamped to the
/// world (±180° lon, ±85° lat).
fn band_window(view: (f32, f32, f32, f32), span_lon: f32, span_lat: f32) -> (f32, f32, f32, f32) {
    let (vc_lon, vc_lat) = ((view.0 + view.2) * 0.5, (view.1 + view.3) * 0.5);
    let s_lon = (span_lon * 2.0).min(360.0);
    let s_lat = (span_lat * 2.0).min(170.0);
    let mut b_lon0 = (vc_lon - s_lon * 0.5).clamp(-180.0, 180.0);
    let b_lon1 = (b_lon0 + s_lon).min(180.0);
    b_lon0 = (b_lon1 - s_lon).max(-180.0);
    let mut b_lat0 = (vc_lat - s_lat * 0.5).clamp(-85.0, 85.0);
    let b_lat1 = (b_lat0 + s_lat).min(85.0);
    b_lat0 = (b_lat1 - s_lat).max(-85.0);
    (b_lon0, b_lat0, b_lon1, b_lat1)
}

/// Is `view` covered by the bake `window` (with tolerance)?
fn inside_band(view: (f32, f32, f32, f32), window: (f32, f32, f32, f32), tol_lon: f32, tol_lat: f32) -> bool {
    view.0 >= window.0 - tol_lon
        && view.1 >= window.1 - tol_lat
        && view.2 <= window.2 + tol_lon
        && view.3 <= window.3 + tol_lat
}

pub struct App {
    conn: Connection,
    qh: QueueHandle<App>,
    registry_state: RegistryState,    seat_state: SeatState,
    output_state: OutputState,
    #[allow(dead_code)] // held for lifetime: bindings must outlive the surface
    compositor: CompositorState,
    #[allow(dead_code)] // held for lifetime: bindings must outlive the surface
    layer_shell: LayerShell,

    layer: Option<LayerSurface>,
    renderer: Option<Renderer>,
    /// notification popup surface (Layer::Top, top-right corner)
    notif_surf: Option<AuxSurface>,
    /// ext-session-lock manager (Hyprland exposes it; other compositors may
    /// not, in which case locking falls back to the bar morph)
    session_lock: SessionLockState,
    /// the active session lock while the native lockscreen is up
    session_lock_active: Option<SessionLock>,
    /// the fullscreen lock surface (owned by the session lock)
    lock_surf: Option<LockSurface>,
    /// blurred wallpaper backdrop for the lock surface (path/size → texture)
    lock_bg_loaded: Option<String>,
    lock_bg_tex: Option<Tex>,
    lock_bg_size: (i32, i32),
    /// fade-in start while the lock backdrop fades in (~0.25 s)
    lock_fade: Option<Instant>,
    lock_fade_timer: Option<calloop::RegistrationToken>,
    /// sends the (threaded) PAM result back to the event loop
    lock_auth_tx: Option<calloop::channel::Sender<LockAuthMsg>>,
    /// polkit dialog: password-check verdicts come back on this channel
    polkit_auth_tx: Option<calloop::channel::Sender<PolkitAuthMsg>>,
    /// notifications currently shown in the corner popup (≤ 3, newest first)
    notif_popup: Vec<crate::shell::Notif>,
    /// auto-hides the notification popup
    notif_timer: Option<calloop::RegistrationToken>,
    text: Option<TextEngine>,
    img: crate::img::ImageStore,
    shell: Shell,
    hypr: Hypr,
    services: Services,
    stats: crate::stats::Stats,
    /// wlr-data-control clipboard manager (history + re-copy)
    clip: crate::clipboard::Clipboard,

    pointer: Option<wl_pointer::WlPointer>,
    keyboard: Option<wl_keyboard::WlKeyboard>,
    /// pointer position relative to our surface (for click hit-testing)
    pointer_x: f64,
    pointer_y: f64,

    /// zwp pointer-gestures manager (touchpad pinch → world-map zoom)
    gestures: Option<ZwpPointerGesturesV1>,
    /// live pinch-gesture object
    pinch: Option<ZwpPointerGesturePinchV1>,
    /// world_zoom at pinch begin (scale is absolute vs the begin distance)
    pinch_base_zoom: f32,
    /// a pinch is tracking AND started over the world-map body
    pinch_engaged: bool,
    /// the world point + window fractions under the cursor at pinch begin —
    /// the zoom-to-point anchor for the whole gesture
    pinch_anchor: Option<(f32, f32, f32, f32)>,
    /// cached world-map band texture + the bake window it covers. Pan/zoom
    /// just moves a UV window over it; a new raster is only baked when the
    /// view leaves the window / crosses a zoom band.
    world_band: Option<WorldBand>,

    /// last size the compositor configured us at (logical px)
    configured: (i32, i32),
    dirty: bool,
    /// per-frame render metrics, logged by `maybe_render` when ZEN_TRACE=1
    frame_stats: FrameStats,
    /// when the last size commit was issued (for the ack-timeout safety net)
    last_size_commit: Option<Instant>,

    loop_handle: Option<LoopHandle<'static, App>>,
    /// one-shot boot timer: re-runs the hover rule right after launch so a
    /// cursor already resting over the bar expands the dashboard immediately
    /// (the hover rule otherwise only fires on pointer events / the 60s tick)
    boot_timer: Option<calloop::RegistrationToken>,
    morph_timer: Option<calloop::RegistrationToken>,
    /// drives the power-menu hold-to-confirm fill (~30 fps while holding)
    hold_timer: Option<calloop::RegistrationToken>,
    /// drives the visualizer (60 fps while the card/pill is visible; bars rise
    /// with the live sink spectrum and fall slowly, cava-style)
    viz_timer: Option<calloop::RegistrationToken>,
    /// real audio capture → FFT bars behind the viz card
    viz_cap: crate::viz::Capture,
    /// drives the Sensors card IIO sampling (~1.5 s while the dashboard is
    /// open)
    sensor_timer: Option<calloop::RegistrationToken>,
    /// drives the Mirror card camera upload (~camera rate while the dashboard
    /// is open)
    cam_timer: Option<calloop::RegistrationToken>,
    /// edit-mode auto-arrange: every ~3 s shifts cards left into empty gaps
    /// while the dashboard is in edit mode
    autoarrange_timer: Option<calloop::RegistrationToken>,
    /// drives the edit-mode settle glide (~60 fps while cards glide to slots)
    edit_timer: Option<calloop::RegistrationToken>,
    clock_timer: Option<calloop::RegistrationToken>,
    /// pomodoro countdown driver — 1 s ticks while a phase is running
    pomo_timer: Option<calloop::RegistrationToken>,
    /// eye-rest (20-20-20) countdown driver — 1 s ticks while focus/rest live
    er_timer: Option<calloop::RegistrationToken>,
    /// mtime of the per-state config at last check — lets the declarative card
    /// spec hot-reload on file save WITHOUT an IPC call (picked up by the 3 s
    /// services tick).
    spec_mtime: Option<std::time::SystemTime>,
    /// audio recorder timer — 1 s ticks while a recording is live, plus a
    /// one-shot list refresh when the recordings pane opens
    audiorec_timer: Option<calloop::RegistrationToken>,
    services_timer: Option<calloop::RegistrationToken>,
    /// appends a heartbeat line to `$states2/zen_shell_watch` every 15 s
    watch_timer: Option<calloop::RegistrationToken>,
    /// battery / wattage polled once a minute (too slow for the 3s services timer)
    power_timer: Option<calloop::RegistrationToken>,
    /// auto-dismisses the transient OSD (Mode::Osd)
    osd_timer: Option<calloop::RegistrationToken>,
    /// world-clock zone-offset probes: batched `TZ=<tz> date +%z %Z` on a
    /// worker thread; results land here
    tz_tx: Option<calloop::channel::Sender<Vec<(String, i32, String)>>>,
    /// left mouse button held (drives slider drags)
    pointer_down: bool,
    /// hover-expansion armed. An explicit dismissal (selection, Esc, IPC /
    /// click close, click-away) DISARMS it, so a cursor that merely RESTS
    /// over the pill strip can't instantly re-expand the dashboard ("select
    /// something and the panel lingers big for seconds"). The next real
    /// pointer movement (motion / enter event) re-arms it.
    hover_armed: bool,
    /// Ctrl held (xkb depressed modifier bit 2) — wallpaper grid zoom
    ctrl_down: bool,
    /// fractional scroll accumulator — smooth trackpad deltas are tiny per
    /// event and would truncate to zero rows; accumulate until a whole row
    /// builds up (two-finger scroll in the launcher)
    scroll_acc: f32,
    /// fractional accumulator for the HORIZONTAL axis (dashboard board pan)
    scroll_h_acc: f32,
    /// last time drag values were flushed to the backend — throttles the
    /// per-motion subprocess spawn (wpctl) / sysfs writes so a fast drag
    /// can't flood the loop; the final value is flushed on release
    slider_flush: Instant,

    // backends (config-selected, see src/backends.rs)
    power: Box<dyn crate::backends::Power>,
    brightness: Box<dyn crate::backends::Brightness>,
    audio: Box<dyn crate::backends::Audio>,
    wallpaper: Box<dyn crate::backends::Wallpaper>,
    lock: Box<dyn crate::backends::Lock>,
    /// keeps the IPC listener fd alive for the process lifetime
    _ipc_listener: Option<std::os::unix::net::UnixListener>,
    tray_tx: Option<calloop::channel::Sender<TrayMsg>>,
    tray_cmd_tx: Option<std::sync::mpsc::Sender<TrayCmd>>,
    /// polkit agent: sender for auth responses
    polkit_cmd_tx: Option<std::sync::mpsc::Sender<PolkitCmd>>,
    /// sender for the weather thread
    wx_tx: Option<calloop::channel::Sender<crate::weather::WeatherMsg>>,
    /// sender for the dashboard-telemetry worker thread (dash_status script)
    dash_tx: Option<calloop::channel::Sender<String>>,
    /// sender for the News headline fetch worker
    news_tx: Option<calloop::channel::Sender<crate::news::NewsMsg>>,
    lyrics_tx: Option<calloop::channel::Sender<crate::lyrics::LyricsMsg>>,
    speed_tx: Option<calloop::channel::Sender<crate::shell::SpeedMsg>>,
    quote_tx: Option<calloop::channel::Sender<crate::shell::QuoteMsg>>,
    ticker_tx: Option<calloop::channel::Sender<crate::shell::TickerMsg>>,
    /// resource broker: "fetch what's rendered" — unions on-screen needs,
    /// and the services timer kicks whatever tick() says is due.
    broker: fetcher::Broker,
    /// guards the periodic Price-card refetch while one is in flight
    ticker_inflight: bool,
    /// true while a dash_status run is in flight (one at a time)
    dash_inflight: Arc<AtomicBool>,
    /// latest raw RGBA camera frame (written by the ffmpeg reader thread)
    cam_frame: Arc<std::sync::Mutex<Vec<u8>>>,
    /// true when a NEW frame landed since the last decode
    cam_fresh: Arc<std::sync::atomic::AtomicBool>,
    /// live ffmpeg child (rawvideo RGBA → pipe:1) feeding the reader thread
    cam_child: Option<std::process::Child>,
    /// capture rate currently requested from ffmpeg (60 preferred, 30 fallback)
    cam_rate: u32,
    /// true once the 60 → 30 fps fallback has been attempted
    cam_retried: bool,
    /// frames consumed since the current capture started (0 ⇒ feeds nothing)
    cam_frames: u64,
    /// frames seen at the start of the current delivered-fps window
    cam_fps_last: u64,
    /// when the current delivered-fps window started
    cam_fps_at: Instant,
    /// when the current capture child was spawned (drives the rate fallback)
    cam_spawned_at: Instant,
    /// next retry time when the capture has failed and we're backing off
    /// (retry keeps the mirror alive instead of giving up until the dashboard
    /// is reopened)
    cam_retry_at: Option<Instant>,
    /// ffmpeg encoder for the mirror recording (raw RGBA frames via stdin)
    mirror_rec: Option<std::process::Child>,
    /// writable stdin of `mirror_rec` — each fresh capture frame goes in here
    mirror_rec_stdin: Option<std::process::ChildStdin>,
    /// when the current recording started (drives the on-card timer)
    mirror_rec_at: Option<Instant>,
    /// output path of the recording in progress (for the "saved" OSD)
    mirror_rec_path: Option<std::path::PathBuf>,
    /// pw-record child for the audio recorder card
    audiorec_rec: Option<std::process::Child>,
    /// when the current audio recording started (drives the on-card timer)
    audiorec_rec_at: Option<Instant>,
    /// output path of the audio recording in progress (for the "Saved" OSD)
    audiorec_rec_path: Option<std::path::PathBuf>,
    /// derived recordings listing dir (~/Recordings/Audio)
    audiorec_dir: std::path::PathBuf,
    /// background app-index scan result receiver
    apps_rx: Option<std::sync::mpsc::Receiver<Vec<crate::apps::App>>>,
    /// world-map chunk download results (world.bin / country chunk)
    world_dl_rx: Option<std::sync::mpsc::Receiver<crate::shell::mapdata::DlResult>>,
    /// sender for those download worker threads (channel created lazily)
    world_dl_tx: Option<std::sync::mpsc::Sender<crate::shell::mapdata::DlResult>>,

    pub running: Arc<AtomicBool>,
}

/// Result of a lockscreen PAM check, posted from a worker thread.

/// The native lockscreen's fullscreen surface (ext-session-lock-v1): its own
/// `wl_surface` + GL context, sized by the compositor to the output. The
/// session lock routes ALL input to this surface and tells the compositor the
/// session is locked, which is what actually blocks Hyprland's own keybinds.

/// A secondary layer surface (wallpaper background, notification popup): its
/// own `wl_surface` + GL context and the same ack-gated size commit as the
/// bar, so a second surface can never wedge the bar's morph dance.

/// First decodable image in `dir` (sorted by name) — the wallpaper daemon's
/// boot background. Returns None when the dir is unset or empty.
impl App {
    /// Connect, build the surface + GL, and run the event loop.
    pub fn run(cfg: Config) -> Result<(), String> {
        let conn = Connection::connect_to_env().map_err(|e| format!("connect: {e}"))?;
        let (globals, event_queue) = registry_queue_init(&conn).map_err(|e| format!("globals: {e}"))?;
        let qh = event_queue.handle();
        let mut app = Self::new(conn.clone(), qh, globals, cfg)?;
        // apply the initial geometry (anchor / margin / exclusive zone) now
        // that the surface exists
        app.apply_bar_geometry();
        app.start(event_queue)
    }

    fn new(
        conn: Connection,
        qh: QueueHandle<App>,
        globals: wayland_client::globals::GlobalList,
        cfg: Config,
    ) -> Result<Self, String> {
        let trace = std::env::var("ZEN_TRACE").is_ok();
        macro_rules! tr {
            ($($a:tt)*) => { if trace { eprintln!("zen: {}", format!($($a)*)); } };
        }
        tr!("binding compositor");
        let compositor = CompositorState::bind(&globals, &qh)
            .map_err(|e| format!("wl_compositor unavailable: {e}"))?;
        let layer_shell = LayerShell::bind(&globals, &qh)
            .map_err(|e| format!("wlr-layer-shell unavailable: {e}"))?;
        let mut output_state = OutputState::new(&globals, &qh);
        let mut seat_state = SeatState::new(&globals, &qh);
        let session_lock = SessionLockState::new(&globals, &qh);
        let _ = (&mut output_state, &mut seat_state);

        // Pill shell first: `collapsed_w()` sizes the very first surface.
        // TextEngine / Shell::new are pure CPU (no Wayland roundtrip), so
        // running them here costs nothing on the critical path — and the bar
        // can size its initial layer commit to the measured pill width.
        tr!("loading fonts");
        let text = TextEngine::new_with_slots(
            cfg.family(),
            cfg.font_display(),
            cfg.font_mono(),
            crate::vars::read_branding_font(),
            cfg.icon_family(),
            crate::icons::slot_map(cfg.icon_style()),
        );
        tr!("shell init");
        let mut shell = Shell::new(cfg.clone());
        shell.weather_city = cfg.weather.city.clone();
        shell.update_clock();
        // Width-critical backends: the resting pill width is a function of its
        // CONTENTS (battery %, workspace dots, tray), so the very first surface
        // commit must already include them — otherwise the pill boots clipped
        // and only grows after the deferred seed (the "broken pill"). These
        // backends are cheap to construct; only the hyprctl/sysfs reads below
        // block briefly, and it's worth it to never show a broken pill.
        let hypr = Hypr::new();
        let mut power: Box<dyn crate::backends::Power> = match cfg.power_backend().as_str() {
            "upower" => Box::new(crate::backends::UPowerPower::new()),
            _ => Box::new(crate::backends::SysfsPower::new()),
        };
        // Populate ws_count / battery so `collapsed_w()` is the full pill width
        // from the first frame. The rest of the deferred seed is unchanged.
        hypr.update_shell(&mut shell);
        let _ = power.poll(&mut shell);
        // The pill is sized to its contents (clock/date/order/tray/battery),
        // not the config constant — the constant would size the surface to a
        // fixed 165px and render the resting pill clipped until the first
        // expand⇄collapse morph re-fits it (the "broken pill" at boot).
        let cw = shell.collapsed_w().ceil().max(1.0) as u32;
        let ch = cfg.bar_h().ceil().max(1.0) as u32;
        shell.cur_w = cw as f32;
        shell.cur_h = ch as f32;
        shell.note_committed(cw as f32, ch as f32);

        // --- layer surface (floating pill, bottom-center, 25px margin) ---
        let surface = compositor.create_surface(&qh);
        let layer = layer_shell.create_layer_surface(
            &qh,
            surface,
            Layer::Top,
            Some("zen-shell"),
            None,
        );
        layer.set_anchor(Anchor::BOTTOM);
        layer.set_margin(
            0,
            0,
            cfg.bar.floating_offset.max(0),
            0,
        );
        layer.set_keyboard_interactivity(KeyboardInteractivity::OnDemand);
        layer.set_size(cw, ch);
        layer.commit(); // initial commit → first configure
        let _ = conn.flush(); // get the requests out NOW (C++ lesson)

        // --- renderer: GL (default) or Vulkan (ZEN_VULKAN=1 + `vk` feature) ---
        let display_ptr = conn.backend().display_ptr() as *mut c_void;
        #[cfg(feature = "vk")]
        let renderer = {
            let want_vk = std::env::var("ZEN_VULKAN").map(|v| v == "1").unwrap_or(false);
            if want_vk {
                tr!("initializing Vulkan");
                let surface_ptr = layer.wl_surface().id().as_ptr() as *mut c_void;
                match unsafe { Vk::init(display_ptr, surface_ptr, cw as i32, ch as i32) } {
                    Ok(vk) => Renderer::Vk(vk),
                    Err(e) => {
                        eprintln!("zen: Vulkan init failed ({e}); falling back to GL");
                        Renderer::Gl(Self::init_gl(display_ptr, layer.wl_surface(), cw as i32, ch as i32)?)
                    }
                }
            } else {
                tr!("initializing GL");
                Renderer::Gl(Self::init_gl(display_ptr, layer.wl_surface(), cw as i32, ch as i32)?)
            }
        };
        #[cfg(not(feature = "vk"))]
        let renderer = {
            tr!("initializing GL");
            Renderer::Gl(Self::init_gl(display_ptr, layer.wl_surface(), cw as i32, ch as i32)?)
        };
        // --- secondary layer surface: notification popup (top layer,
        // top-right corner) ---
        let notif_surf = {
            let (right, top) = cfg.notif_position();
            let anchor = match (right, top) {
                (true, true) => Anchor::TOP | Anchor::RIGHT,
                (false, true) => Anchor::TOP | Anchor::LEFT,
                (true, false) => Anchor::BOTTOM | Anchor::RIGHT,
                (false, false) => Anchor::BOTTOM | Anchor::LEFT,
            };
            let margins = match (right, top) {
                (true, true) => (12, 12, 0, 0),    // top-right
                (false, true) => (12, 0, 0, 12),   // top-left
                (true, false) => (0, 12, 12, 0),   // bottom-right
                (false, false) => (0, 0, 12, 12),  // bottom-left
            };
            AuxSurface::create(
                &compositor,
                &layer_shell,
                &qh,
                display_ptr,
                Layer::Top,
                anchor,
                "zen-notif",
                margins,
                1,
                1,
            )
        };
        // wallpaper thumbnails: warm the disk cache
        // ($HOME/.cache/thumbnails/zen-shell) on a background thread —
        // incremental (mtime-verified, fresh ones skipped), only supported
        // formats (jpg/jpeg/png/gif), never blocks startup.
        crate::img::spawn_thumbnail_warm(cfg.wallpaper_dir());
        // clipboard manager: wlr-data-control + persisted history. The
        // cliphist dbs / persisted history are seeded lazy in
        // `seed_boot_data()` after the first frame (reading + rasterizing
        // image thumbnails is disk+CPU work that would delay the bar).
        let img_store = crate::img::ImageStore::new();
        let mut clip = crate::clipboard::Clipboard::new();
        clip.bind_manager(&qh, &globals);
        // TEMP test hook: ZEN_INIT_MODE=launcher opens the app grid at boot
        if std::env::var("ZEN_INIT_MODE").as_deref() == Ok("launcher") {
            shell.set_mode(crate::shell::Mode::Launcher);
        }
        // NOTE: ws count / active window, ssid / media, battery / AC,
        // brightness, volume are seeded LAZILY in `seed_boot_data()` (called
        // from `start()` right after the first frame is committed) — not here.
        // Those are hyprctl-subprocess / D-Bus / sysfs / wpctl reads that
        // would otherwise block the bar + launcher from appearing. Setting
        // them up here is cheap (sockets + structs); only the data reads are
        // deferred so the first frame comes up instantly.
        // (`hypr` and `power` were already constructed + seeded above, so the
        // very first surface commit is already the full resting-pill width.)
        let services = Services::new();
        // Brightness: use the sysfs file directly when this user can write it,
        // otherwise fall back to systemd-logind's D-Bus SetBrightness (whether
        // the sysfs path or logind is used, hardware control stays in the shell
        // — no brightnessctl / external script is ever spawned).
        let brightness: Box<dyn crate::backends::Brightness> = match cfg.brightness_backend().as_str() {
            "sysfs" => {
                let sfs = crate::backends::SysfsBrightness::new();
                if sfs.writable() {
                    Box::new(sfs)
                } else {
                    Box::new(crate::backends::LogindBrightness::new())
                }
            }
            _ => Box::new(crate::backends::SysfsBrightness::new()),
        };
        let audio: Box<dyn crate::backends::Audio> = match cfg.audio_backend().as_str() {
            _ => Box::new(crate::backends::WpctlAudio),
        };

        Ok(App {
            conn,
            qh,
            registry_state: RegistryState::new(&globals),
            seat_state,
            output_state,
            compositor,
            layer_shell,
            layer: Some(layer),
            renderer: Some(renderer),
            notif_surf,
            session_lock,
            session_lock_active: None,
            lock_surf: None,
            lock_bg_loaded: None,
            lock_bg_tex: None,
            lock_bg_size: (0, 0),
            lock_fade: None,
            lock_fade_timer: None,
            lock_auth_tx: None,
            polkit_auth_tx: None,
            notif_popup: Vec::new(),
            notif_timer: None,
            text: Some(text),
            img: img_store,
            shell,
            hypr,
            services,
            stats: crate::stats::Stats::new(),
            pointer: None,
            keyboard: None,
            pointer_x: 0.0,
            pointer_y: 0.0,
            gestures: None,
            pinch: None,
            pinch_base_zoom: 1.0,
            pinch_engaged: false,
            pinch_anchor: None,
            world_band: None,
            configured: (0, 0),
            dirty: false,
            frame_stats: FrameStats { last: Instant::now(), frames: 0, fps: 0.0, solids: 0, textured: 0, uploads: 0 },
            last_size_commit: None,
            loop_handle: None,
            boot_timer: None,
            morph_timer: None,
            hold_timer: None,
            viz_timer: None,
            viz_cap: crate::viz::Capture::start(),
            sensor_timer: None,
            cam_timer: None,
            edit_timer: None,
            autoarrange_timer: None,
            clock_timer: None,
            pomo_timer: None,
            er_timer: None,
            spec_mtime: None,
            audiorec_timer: None,
            services_timer: None,
            watch_timer: None,
            power_timer: None,
            osd_timer: None,
            tz_tx: None,
            pointer_down: false,
            ctrl_down: false,
            scroll_acc: 0.0,
            scroll_h_acc: 0.0,
            hover_armed: true,
            slider_flush: Instant::now(),
            power,
            brightness,
            audio,
            wallpaper: Box::new(crate::backends::CommandWallpaper::new(cfg.wallpaper_command()))
                as Box<dyn crate::backends::Wallpaper>,
            lock: if cfg.lockscreen_backend() == "hyprlock" {
                Box::new(crate::backends::HyprlockLock) as Box<dyn crate::backends::Lock>
            } else {
                Box::new(crate::backends::NativeLock) as Box<dyn crate::backends::Lock>
            },
            _ipc_listener: None,
            tray_tx: None,
            tray_cmd_tx: None,
            polkit_cmd_tx: None,
            wx_tx: None,
            dash_tx: None,
            dash_inflight: Arc::new(AtomicBool::new(false)),
            news_tx: None,
            lyrics_tx: None,
            speed_tx: None,
            quote_tx: None,
            ticker_tx: None,
            broker: fetcher::Broker::new(),
            ticker_inflight: false,
            cam_frame: Arc::new(std::sync::Mutex::new(Vec::new())),
            cam_fresh: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            cam_child: None,
            cam_rate: 0,
            cam_retried: false,
            cam_frames: 0,
            cam_fps_last: 0,
            cam_fps_at: Instant::now(),
            cam_spawned_at: Instant::now(),
            cam_retry_at: None,
            mirror_rec: None,
            mirror_rec_stdin: None,
            mirror_rec_at: None,
            mirror_rec_path: None,
            audiorec_rec: None,
            audiorec_rec_at: None,
            audiorec_rec_path: None,
            audiorec_dir: std::env::var("HOME")
                .map(|h| std::path::PathBuf::from(h).join("Recordings").join("Audio"))
                .unwrap_or_else(|_| std::path::PathBuf::from("/tmp/zen-rec")),
            apps_rx: None,
            world_dl_rx: None,
            world_dl_tx: None,
            clip,
            running: Arc::new(AtomicBool::new(true)),
        })
    }

    /// GL backend (default): EGL window over any wl_surface (the bar's layer
    /// surface, the aux surfaces, or the session-lock surface). Returns the
    /// raw `Gl`; the bar wraps it in `Renderer::Gl`, the aux surfaces (wallpaper
    /// background, notification popup) keep it as-is — each `Gl` is its own
    /// context on the shared EGL display, switched via `make_current`.
    fn init_gl(
        display_ptr: *mut c_void,
        wl_surface: &wl_surface::WlSurface,
        w: i32,
        h: i32,
    ) -> Result<crate::render::Gl, String> {
        let egl_window = WlEglSurface::new(wl_surface.id(), w, h)
            .map_err(|e| format!("wl_egl_window: {e:?}"))?;
        // `egl_window` is moved into Gl and kept alive for its whole
        // lifetime — dropping it here used to silently break presentation
        // (surface created, buffers swapped, nothing ever displayed).
        let gl = unsafe {
            Gl::init(display_ptr, egl_window.ptr(), egl_window, w, h)
                .map_err(|e| format!("GL init: {e}"))?
        };
        Ok(gl)
    }

    pub fn start(&mut self, mut event_queue: wayland_client::EventQueue<App>) -> Result<(), String> {
        // The event loop must exist BEFORE the initial roundtrip: capability
        // events during it create the keyboard, and the repeat-capable
        // keyboard needs this loop's handle for its repeat timer (holding an
        // arrow key in the launcher auto-scrolls only then).
        let mut event_loop: calloop::EventLoop<App> =
            calloop::EventLoop::try_new().map_err(|e| format!("event loop: {e}"))?;
        let handle = event_loop.handle();
        self.loop_handle = Some(handle.clone());

        // Force the initial configure synchronously: flush our layer-surface
        // requests, block until the compositor acks. This also draws the first
        // frame (configure handler → maybe_render → eglSwapBuffers).
        let _ = event_queue
            .roundtrip(self)
            .map_err(|e| format!("initial roundtrip: {e}"))?;

        // touchpad pinch-zoom: bind the gesture path once the pointer (seat
        // capability) and registry globals are confirmed
        self.ensure_pinch(&self.qh.clone());

        // Bar is confirmed up and interactive — write a heartbeat marker so
        // watchers (e.g. a statusline script) see the shell is running.
        let _ = std::fs::write(
            crate::vars::states2_dir().join("zen_shell_watch"),
            "zen-shell_is_running\n",
        );

        // First frame is up (bar + any boot-mode surface visible). Now fill in
        // the peripheral data (hyprctl ws/win, D-Bus ssid/media, battery, AC,
        // brightness, volume) that was deferred out of `App::new` for a fast
        // start. These block briefly here, but the shell is already on screen
        // and interactive, so the user never waits on them to open the
        // launcher. Redraw only if a seed actually changed something.
        if self.seed_boot_data() {
            self.dirty = true;
            self.maybe_render();
        }
        // Seed dashboard layout, colors, accent flags, todos, hostname, etc.
        // that were deferred out of Shell::new() for an instant first frame.
        self.shell.seed_data();

        // Boot re-fit: the resting pill is sized to its contents, and the
        // seeds above just filled in battery / AC / colors — the measured pill
        // width may now exceed the boot surface. Grow to it now (tiny morph)
        // instead of leaving the pill clipped until the user's first
        // expand⇄collapse ("broken pill" that only fixes itself on hover).
        if self.shell.mode == crate::shell::Mode::Collapsed
            || self.shell.mode == crate::shell::Mode::Expanded
        {
            self.shell.refresh_size();
            self.ensure_morph_timer();
        }

        // Kick off the background .desktop-file scan now that the first
        // frame is visible; results arrive via a std::sync::mpsc channel
        // polled each 1s tick.
        if self.shell.apps.apps.is_empty() {
            self.apps_rx = Some(self.shell.apps.rescan_async());
        }

        WaylandSource::new(self.conn.clone(), event_queue)
            .insert(handle.clone())
            .map_err(|e| format!("wayland source: {e:?}"))?;

        // Hyprland event socket → refresh workspaces / active window on change
        if let Some(ev) = self.hypr.event.as_ref().and_then(|s| s.try_clone().ok()) {
            use smithay_client_toolkit::reexports::calloop::generic::Generic;
            use smithay_client_toolkit::reexports::calloop::{Interest, Mode};
            let src = Generic::new(ev, Interest::READ, Mode::Level);
            use smithay_client_toolkit::reexports::calloop::PostAction;
            let _ = handle.insert_source(src, |_, _, app: &mut App| {
                if app.hypr.drain_events(&mut app.shell) {
                    app.dirty = true;
                    app.maybe_render();
                }
                Ok(PostAction::Continue)
            });
        }

        // IPC socket (`zen-shell open launcher` / `zen-shell ipc call …`)
        if let Err(e) = self.install_ipc(&handle) {
            eprintln!("zen: ipc socket: {e}");
        }

        // 1min clock: only wakes when the displayed time changes (60s).
        // The size-ack timeout + hover-reconcile safety nets run on the same
        // tick — both are no-ops at rest, and hover-collapse itself is driven
        // by pointer events (input.rs), so a 60s reconcile is only a rare
        // lost-pointer-event fallback, never the hot path.
        let clock_tok = handle
            .insert_source(
                calloop::timer::Timer::from_duration(Duration::from_secs(60)),
                |_, _, app: &mut App| {
                    if app.shell.update_clock() | app.shell.tick_watts() {
                        app.dirty = true;
                        app.maybe_render();
                    }
                    // alarms: fire the notification popup when a set HH:MM
                    // comes around (once per day per alarm)
                    if let Some((label, time)) = app.shell.alarms_due() {
                        app.on_notif(crate::shell::Notif {
                            id: 0,
                            app: "zen-shell".into(),
                            summary: format!("Alarm {time}"),
                            body: label,
                            icon: String::new(),
                            urgency: 1,
                            actions: Vec::new(),
                        });
                    }
                    app.check_size_timeout();
                    app.reconcile_hover();
                    calloop::timer::TimeoutAction::ToDuration(Duration::from_secs(60))
                },
            )
            .map_err(|e| format!("clock timer: {e:?}"))?;
        self.clock_timer = Some(clock_tok);
        self.sync_pomo_timer();

        // World-clock tz probes: worker replies land here and update the
        // pinned zones' offset/abbreviation caches.
        let (tz_tx, tz_rx) = calloop::channel::channel::<Vec<(String, i32, String)>>();
        self.tz_tx = Some(tz_tx);
        let tz_tok = handle
            .insert_source(tz_rx, |ev, _meta: &mut (), app: &mut App| {
                if let calloop::channel::Event::Msg(results) = ev {
                    for (tz, off, abbr) in results {
                        for (_city, zt, off_slot, ab_slot) in app.shell.worldclock_zones.iter_mut() {
                            if zt == &tz {
                                *off_slot = off;
                                *ab_slot = abbr.clone();
                            }
                        }
                    }
                    app.shell.worldclock_probed_at = Some(std::time::Instant::now());
                    app.dirty = true;
                    app.maybe_render();
                }
            })
            .map_err(|e| format!("tz channel: {e:?}"))?;
        let _ = tz_tok;

        // 2s services: MPRIS / upower / NetworkManager / system stats — redraw
        // only on change
        let svc_tok = handle
            .insert_source(
                calloop::timer::Timer::from_duration(Duration::from_secs(3)),
                |_, _, app: &mut App| {
                    // A slider drag owns the value: skip the poll-and-overwrite
                    // (and its OSD) while the user is dragging, or the backend's
                    // stale read would yank the knob back mid-drag. The drag
                    // writes the value itself; the next poll after release syncs.
                    let dragging = app.pointer_down && app.shell.drag_key.is_some();
                    // services.poll queries wpctl status + up to 14 get-volume
                    // via blocking subprocesses — don't let that stall the loop
                    // under an active slider drag.
let mut changed = false;
                    if !dragging {
                        changed = app.services.poll(&mut app.shell);
                        // media track changed → fetch its lyrics in the bg
                        app.check_lyrics();
                    }
                    // broker: fetch whatever's currently on-screen (warm new
                    // consumers + refresh settled ones on their TTL)
                    let now = std::time::Instant::now();
                    let wants = fetcher::Broker::shell_wants(&app.shell);
                    for res in app.broker.tick(now, &wants) {
                        app.broker.mark(res, now);
                        app.broker_fetch(res);
                    }
                    // workspace list + active id: the event socket keeps this
                    // live normally; poll every 3 s as a safety net so the
                    // pill's workspace digits can never go stale (e.g. missed
                    // events) — cheap local socket queries, no-op when unchanged
                    changed |= app.hypr.update_shell(&mut app.shell);
                    // world clock: refresh pinned zones' offsets when stale
                    app.tick_worldclock_probe();
                    // media playing/paused → equalizer on/off
                    app.sync_viz_timer();
                    // eye rest countdown driver (self-tearing 1 s tick)
                    app.sync_er_timer();
                    // audio recorder timer — live elapsed + list refresh
                    app.sync_audiorec_timer();
                    // declarative card spec hot-reload: if the per-state config
                    // file changed on disk since we last applied it, reload so
                    // `[cards.*]` edits land without a rebuild — no IPC needed
                    if let Some(mt) = std::fs::metadata(&app.shell.per_state_config)
                        .ok()
                        .and_then(|m| m.modified().ok())
                    {
                        if app.spec_mtime != Some(mt) {
                            app.spec_mtime = Some(mt);
                            app.reload_config();
                        }
                    }
                    // brightness backend: /sys/class/backlight, polled always
                    if !dragging {
                        if let Some(b) = app.brightness.get() {
                            if (app.shell.brightness - b).abs() > 0.005 {
                                app.shell.brightness = b;
                                changed = true;
                                app.show_osd(ICON_BRIGHTNESS, format!("{:.0}%", b * 100.0));
                            }
                        }
                    }
                    // default-sink volume + mute → OSD when it changes
                    if !dragging {
                        if let Some((vol, muted)) = app.audio.default_volume() {
                            if (app.shell.volume - vol).abs() > 0.005 {
                                app.shell.volume = vol;
                                changed = true;
                                app.show_osd(
                                    if muted { ICON_MUTE } else { ICON_VOLUME },
                                    format!("{:.0}%", vol * 100.0),
                                );
                            } else if app.shell.volume_muted != muted {
                                app.shell.volume_muted = muted;
                                changed = true;
                                app.show_osd(
                                    if muted { ICON_MUTE } else { ICON_VOLUME },
                                    if muted { "Muted".to_string() } else { "Unmuted".to_string() },
                                );
                            }
                        }
                    }
                    // CPU/RAM/disk/net are expensive-ish /proc reads shown only
                    // on the dashboard and the settings stats card — poll them
                    // only while one of those is on screen; they sleep in every
                    // other mode (first poll after waking just spans the gap).
                    // A live slider drag owns the frame: skip the /proc scans
                    // so the 3 s beat can't stall the loop under the pointer.
                    if !dragging
                        && matches!(
                            app.shell.mode,
                            crate::shell::Mode::Expanded | crate::shell::Mode::Settings
                        )
                    {
                        changed |= app.stats.poll(&mut app.shell);
                        crate::stats::Stats::poll_disk_parts(&mut app.shell);
                        crate::stats::Stats::poll_swap(&mut app.shell);
                        crate::stats::Stats::poll_top_procs(&mut app.shell);
                        crate::stats::Stats::poll_active_win(&mut app.shell);
                        crate::stats::Stats::poll_pkg_updates(&mut app.shell);
                        crate::stats::Stats::poll_ssh_vpn(&mut app.shell);
                        crate::stats::Stats::poll_docker(&mut app.shell);
                        crate::stats::Stats::poll_kb_layout(&mut app.shell);
                    }
                    // per-app audio is only shown on the Audio panel — wpctl
                    // subprocesses sleep in every other mode
                    if app.shell.mode == crate::shell::Mode::Audio
                        || (app.shell.mode == crate::shell::Mode::Expanded
                            && app.shell.card_packed(crate::shell::DashCard::AudioDevice))
                    {
                        changed |= app.audio.poll(&mut app.shell);
                    }
                    // dashboard telemetry (script-backed toggles / sliders) —
                    // only while the dashboard is open
                    if app.shell.mode == crate::shell::Mode::Expanded {
                        app.poll_dash_status();
                        // current color scheme + channel — two tiny state reads;
                        // the 3 s timer already redraws the open dashboard, so
                        // no refresh-on-expand hook is needed
                        let scheme = crate::themes::cur_scheme();
                        if scheme != app.shell.cur_scheme {
                            app.shell.cur_scheme = scheme;
                            changed = true;
                        }
                        let channel = crate::themes::channel_string();
                        if channel != app.shell.cur_channel {
                            app.shell.cur_channel = channel;
                            changed = true;
                        }
                        let acc_src = crate::themes::acc_source_string();
                        if acc_src != app.shell.cur_acc_src {
                            app.shell.cur_acc_src = acc_src;
                            changed = true;
                        }
                        // new-invoked card data: each poll throttles itself and
                        // only runs while its card is packed on the grid (skip
                        // during a slider drag — the blocking subprocess would
                        // stall the loop under the pointer)
                        if !dragging {
                            // card gated, self-throttled, blocking-aware
                            if app.shell.card_packed(crate::shell::DashCard::ConnInfo)
                                && app.shell.poll_conninfo()
                            {
                                changed = true;
                            }
                            if app.shell.card_packed(crate::shell::DashCard::Latency)
                                && app.shell.poll_latency()
                            {
                                changed = true;
                            }
                            if app.shell.card_packed(crate::shell::DashCard::PowerDraw)
                                && app.shell.poll_powerdraw()
                            {
                                changed = true;
                            }
                            if app.shell.card_packed(crate::shell::DashCard::SmartHealth)
                                && app.shell.poll_smart()
                            {
                                changed = true;
                            }
                            if app.shell.card_packed(crate::shell::DashCard::SystemdUnits)
                                && app.shell.poll_systemd()
                            {
                                changed = true;
                            }
                            if app.shell.card_packed(crate::shell::DashCard::JournalTail)
                                && app.shell.poll_journal()
                            {
                                changed = true;
                            }
                        }
                    }
                    // world-map card: resolve a clicked zone's UTC offset now
                    // (`TZ=<zone> date +%z %Z`), and re-probe the active zone
                    // once a DST window is likely (>5 min) to keep the marker
                    // time honest across transitions
                    if let Some(tz) = app.shell.pending_world_offset.take() {
                        app.world_shell_probe(&tz);
                    } else if !app.shell.world_tz.is_empty() {
                        let stale = match app.shell.world_off_at {
                            None => true,
                            Some(t) => t.elapsed() >= Duration::from_secs(300),
                        };
                        if stale {
                            app.world_shell_probe(&app.shell.world_tz.clone());
                        }
                    }
                    // world-map data: boot-load the cache, spawn/collect chunks
                    if app.world_tick() {
                        changed = true;
                    }
                    // Check if the background app-index scan finished
                    if let Some(rx) = app.apps_rx.as_ref() {
                        if let Ok(apps) = rx.try_recv() {
                            app.shell.apps.apps = apps;
                            app.apps_rx = None;
                            changed = true;
                            if std::env::var("ZEN_TRACE").is_ok() {
                                eprintln!("zen: {} apps loaded (async)", app.shell.apps.apps.len());
                            }
                        }
                    }
                    if std::env::var("ZEN_TRACE").is_ok()
                        && (app.shell.cpu > 0 || !app.shell.net_up.is_empty())
                    {
                        eprintln!(
                            "zen: stats cpu={}% mem={}% disk={}% net=↓{} ↑{}",
                            app.shell.cpu,
                            app.shell.mem_pct,
                            app.shell.disk_pct,
                            app.shell.net_down,
                            app.shell.net_up
                        );
                    }
                    if changed {
                        app.dirty = true;
                        app.maybe_render();
                    }
                    calloop::timer::TimeoutAction::ToDuration(Duration::from_secs(3))
                },
            )
            .map_err(|e| format!("services timer: {e:?}"))?;
        self.services_timer = Some(svc_tok);

        // Power timer: battery / wattage polled once a minute. Too slow for
        // the 3 s services timer — updating wattage every 3 s would spam the
        // UI with flickering numbers. 60 s keeps the pill's battery display
        // fresh without wasting CPU on /proc or sysfs reads. The same beat
        // drives the low-memory janitor (see memory_janitor).
        let pwr_tok = handle
            .insert_source(
                calloop::timer::Timer::from_duration(Duration::from_secs(60)),
                |_, _, app: &mut App| {
                    app.memory_janitor();
                    let changed = app.power.poll(&mut app.shell);
                    // AC plug / unplug → OSD
                    if app.power.ac_changed() {
                        app.show_osd(
                            if app.shell.ac_online { ICON_BOLT } else { ICON_POWER },
                            if app.shell.ac_online {
                                "AC plugged".to_string()
                            } else {
                                "AC unplugged".to_string()
                            },
                        );
                    }
                    if changed {
                        app.dirty = true;
                        app.maybe_render();
                    }
                    calloop::timer::TimeoutAction::ToDuration(Duration::from_secs(60))
                },
            )
            .map_err(|e| format!("power timer: {e:?}"))?;
        self.power_timer = Some(pwr_tok);

        // System tray: org.kde.StatusNotifierWatcher on a zbus thread.
        let (tray_tx, tray_rx) = calloop::channel::channel();
        self.tray_tx = Some(tray_tx);
        if let Some((daemon, cmd_tx)) =
            tray::spawn(self.tray_tx.clone().expect("tray_tx"))
        {
            let _ = daemon; // detached
            self.tray_cmd_tx = Some(cmd_tx);
        }
        let tray_tok = handle
            .insert_source(tray_rx, |ev, _meta: &mut (), app: &mut App| {
                app.on_tray_msg(ev);
            })
            .map_err(|e| format!("tray channel: {e:?}"))?;
        let _ = tray_tok;

        // Polkit authentication agent: org.freedesktop.PolicyKit1.Agent
        // on the session bus (replaces mate-polkit / hyprpolkitagent).
        let (polkit_tx, polkit_rx) = calloop::channel::channel();
        if let Some((daemon, cmd_tx)) = polkit::spawn(polkit_tx) {
            let _ = daemon; // detached
            self.polkit_cmd_tx = Some(cmd_tx);
        }
        let polkit_tok = handle
            .insert_source(polkit_rx, |ev, _meta: &mut (), app: &mut App| {
                if let calloop::channel::Event::Msg(msg) = ev {
                    app.on_polkit_msg(msg);
                }
            })
            .map_err(|e| format!("polkit channel: {e:?}"))?;
        let _ = polkit_tok;

        // Weather: Open-Meteo, fetched by the resource broker — one-shot
        // `fetch_once` only while ≥1 on-screen consumer (Weather card, banner
        // weather chip) wants it. The channel just delivers results; no
        // always-on thread anymore.
        {
            let (wx_tx, wx_rx) = calloop::channel::channel();
            self.wx_tx = Some(wx_tx);
            // honor the configured CADENCE as the broker TTL (Weather card
            // asks Slow; explicit interval overrides it)
            if self.shell.cfg.weather.interval_s > 0 {
                self.broker.set_ttl(
                    crate::shell::Res::Weather,
                    Duration::from_secs(self.shell.cfg.weather.interval_s),
                );
            }
            let wx_tok = handle
                .insert_source(wx_rx, |ev, _meta: &mut (), app: &mut App| {
                    if let calloop::channel::Event::Msg(w) = ev {
                        if let Some(city) = w.city.clone() {
                            app.shell.weather_city = city;
                        }
                        if std::env::var("ZEN_TRACE").is_ok() {
                            eprintln!(
                                "zen: weather {:.0}°C code={} wind={:.0}km/h",
                                w.temp_c, w.code, w.wind_kmh
                            );
                        }
                        if app.shell.weather.as_ref() != Some(&w) {
                            app.shell.weather = Some(w);
                            app.dirty = true;
                            app.maybe_render();
                        }
                    }
                })
                .map_err(|e| format!("weather channel: {e:?}"))?;
            let _ = wx_tok;
        }

        // Dashboard telemetry: `dash_status` runs on a worker thread while
        // the dashboard is open (3 s services timer); its one-line report
        // comes back here and syncs toggles / slider values.
        let (dash_tx, dash_rx) = calloop::channel::channel::<String>();
        self.dash_tx = Some(dash_tx);
        let dash_tok = handle
            .insert_source(dash_rx, |ev, _meta: &mut (), app: &mut App| {
                if let calloop::channel::Event::Msg(line) = ev {
                    app.apply_dash_status(&line);
                }
            })
            .map_err(|e| format!("dash status channel: {e:?}"))?;
        let _ = dash_tok;

        // News headlines: fetched on a worker thread (curl, 8 s timeout),
        // pumped back into the shell headline list.
        let (news_tx, news_rx) = calloop::channel::channel::<crate::news::NewsMsg>();
        self.news_tx = Some(news_tx);
        let news_tok = handle
            .insert_source(news_rx, |ev, _meta: &mut (), app: &mut App| {
                if let calloop::channel::Event::Msg(m) = ev {
                    match m {
                        crate::news::NewsMsg::Items(items) => {
                            app.shell.news_items = items;
                            app.shell.news_fetched = true;
                            app.shell.news_scroll = 0;
                            app.shell.news_active_cat = 0;
                            app.shell.news_cat_scroll = 0.0;
                            app.shell.news_cat_content_w = 0.0;
                            app.dirty = true;
                            app.maybe_render();
                        }
                    }
                }
            })
            .map_err(|e| format!("news channel: {e:?}"))?;
        let _ = news_tok;

        // Lyrics of the current track: fetched on a worker thread (LRCLib,
        // curl, 8 s timeout) whenever the MPRIS track changes; results land
        // here and only apply if the track is still current.
        let (lyrics_tx, lyrics_rx) = calloop::channel::channel::<crate::lyrics::LyricsMsg>();
        self.lyrics_tx = Some(lyrics_tx);
        let lyrics_tok = handle
            .insert_source(lyrics_rx, |ev, _meta: &mut (), app: &mut App| {
                if let calloop::channel::Event::Msg(m) = ev {
                    match m {
                        crate::lyrics::LyricsMsg::Result(a, t, lines) => {
                            let want = format!("{a} || {t}");
                            if app.shell.lyrics_track == want {
                                app.shell.lyrics_lines = lines;
                                app.shell.lyrics_state = if app.shell.lyrics_lines.is_empty() { 3 } else { 2 };
                                app.shell.lyrics_scroll = 0;
                                app.dirty = true;
                                app.maybe_render();
                            }
                        }
                    }
                }
            })
            .map_err(|e| format!("lyrics channel: {e:?}"))?;
        let _ = lyrics_tok;

        // Speed test → worker thread → (down, up, ping)
        let (speed_tx, speed_rx) = calloop::channel::channel::<crate::shell::SpeedMsg>();
        self.speed_tx = Some(speed_tx);
        let speed_tok = handle
            .insert_source(speed_rx, |ev, _meta: &mut (), app: &mut App| {
                if let calloop::channel::Event::Msg(m) = ev {
                    match m {
                        crate::shell::SpeedMsg::Done { down_mbps, up_mbps, ping_ms } => {
                            let mb = |m: f32| if m >= 100.0 { format!("{:.0} Mbps", m) } else if m >= 1.0 { format!("{:.1} Mbps", m) } else { format!("{:.0} Kbps", m * 1000.0) };
                            app.shell.speed_down = mb(down_mbps);
                            app.shell.speed_up = mb(up_mbps);
                            app.shell.speed_ping = ping_ms;
                            app.shell.speed_state = 2;
                            app.dirty = true;
                            app.maybe_render();
                        }
                    }
                }
            })
            .map_err(|e| format!("speed channel: {e:?}"))?;
        let _ = speed_tok;

        // Quote fetch → worker thread → text/author
        let (quote_tx, quote_rx) = calloop::channel::channel::<crate::shell::QuoteMsg>();
        self.quote_tx = Some(quote_tx);
        let quote_tok = handle
            .insert_source(quote_rx, |ev, _meta: &mut (), app: &mut App| {
                if let calloop::channel::Event::Msg(m) = ev {
                    match m {
                        crate::shell::QuoteMsg::Quote { text, author } => {
                            app.shell.quote_text = text;
                            app.shell.quote_author = author;
                            app.shell.quote_state = 2;
                            app.dirty = true;
                            app.maybe_render();
                        }
                        crate::shell::QuoteMsg::Failed => {
                            app.shell.quote_state = 3;
                            app.dirty = true;
                            app.maybe_render();
                        }
                    }
                }
            })
            .map_err(|e| format!("quote channel: {e:?}"))?;
        let _ = quote_tok;

        // Ticker fetch → worker thread → rows
        let (ticker_tx, ticker_rx) = calloop::channel::channel::<crate::shell::TickerMsg>();
        self.ticker_tx = Some(ticker_tx);
        let ticker_tok = handle
            .insert_source(ticker_rx, |ev, _meta: &mut (), app: &mut App| {
                if let calloop::channel::Event::Msg(m) = ev {
                    match m {
                        crate::shell::TickerMsg::Items { rows } => {
                            app.ticker_inflight = false;
                            app.shell.ticker_last_at = std::time::Instant::now();
                            app.shell.ticker_items = rows;
                            app.shell.ticker_state = if app.shell.ticker_items.is_empty() { 0 } else { 2 };
                            app.dirty = true;
                            app.maybe_render();
                        }
                    }
                }
            })
            .map_err(|e| format!("ticker channel: {e:?}"))?;
        let _ = ticker_tok;

        // Lockscreen PAM checks run on a worker thread; results come back here.
        let (lock_auth_tx, lock_auth_rx) = calloop::channel::channel();
        self.lock_auth_tx = Some(lock_auth_tx);
        let lock_auth_tok = handle
            .insert_source(lock_auth_rx, |ev, _meta: &mut (), app: &mut App| {
                match ev {
                    calloop::channel::Event::Msg(msg) => app.on_lock_auth(msg),
                    // the worker thread died without a result — don't strand
                    // the lockscreen on a forever-“Verifying…” field
                    calloop::channel::Event::Closed => {
                        if app.shell.mode == crate::shell::Mode::Lock
                            && app.shell.lock_checking
                        {
                            app.shell.lock_checking = false;
                            app.shell.lock_error = Some("Password check failed".to_string());
                            app.dirty = true;
                            app.maybe_render();
                        }
                    }
                }
            })
            .map_err(|e| format!("lock auth channel: {e:?}"))?;
        let _ = lock_auth_tok;

        // Polkit password checks run on the same worker model as the lockscreen:
        // a thread verifies against PAM, then both answers the authority (via the
        // zbus daemon) and reports the verdict back here to update the dialog.
        let (polkit_auth_tx, polkit_auth_rx) = calloop::channel::channel();
        self.polkit_auth_tx = Some(polkit_auth_tx);
        let polkit_auth_tok = handle
            .insert_source(polkit_auth_rx, |ev, _meta: &mut (), app: &mut App| {
                match ev {
                    calloop::channel::Event::Msg(msg) => app.on_polkit_result(msg),
                    // the worker died without a verdict — don't strand the
                    // dialog on a forever-“Checking…” field
                    calloop::channel::Event::Closed => {
                        if app.shell.mode == crate::shell::Mode::PolkitAuth
                            && app.shell.polkit_checking
                        {
                            app.shell.polkit_checking = false;
                            app.shell.polkit_pw.clear();
                            app.shell.polkit_error = Some("Password check failed".to_string());
                            app.dirty = true;
                            app.maybe_render();
                        }
                    }
                }
            })
            .map_err(|e| format!("polkit auth channel: {e:?}"))?;
        let _ = polkit_auth_tok;

        // A pointer-enter that arrived during the initial roundtrip above ran
        // before `loop_handle` existed, so `ensure_morph_timer` silently bailed
        // and the morph would be stuck forever (mode flips, size never moves).
        // Arm it now that the loop is up.
        if self.shell.anim.is_some() {
            self.ensure_morph_timer();
        }

        // The pill must be hover-expandable the moment it appears. The hover
        // rule only runs on pointer events / the 60s tick, so a cursor already
        // resting over the bar at launch — or a pointer-enter consumed during
        // the initial roundtrip before `configured` was set (inside=false) and
        // never moved again — left the pill collapsed and looking
        // un-expandable until the mouse moved. Reconcile now, and once more
        // shortly after the layer maps / Hyprland knows the geometry, so a
        // resting cursor expands immediately.
        self.reconcile_hover();
        let boot_tok = handle
            .insert_source(
                calloop::timer::Timer::from_duration(Duration::from_millis(300)),
                |_, _, app: &mut App| {
                    app.reconcile_hover();
                    app.boot_timer = None;
                    calloop::timer::TimeoutAction::Drop
                },
            )
            .map_err(|e| format!("boot timer: {e:?}"))?;
        self.boot_timer = Some(boot_tok);

        // Heartbeat: rewrite `$states2/zen_shell_watch` every 15 s (clean `>`
        // semantics — overwrite, never grow) so watchers can tell the shell is
        // alive.
        let watch_tok = handle
            .insert_source(
                calloop::timer::Timer::from_duration(Duration::from_secs(15)),
                |_, _, _app: &mut App| {
                    let _ = std::fs::write(
                        crate::vars::states2_dir().join("zen_shell_watch"),
                        "zen-shell_is_running\n",
                    );
                    calloop::timer::TimeoutAction::ToDuration(Duration::from_secs(15))
                },
            )
            .map_err(|e| format!("watch timer: {e:?}"))?;
        self.watch_timer = Some(watch_tok);

        let signal = event_loop.get_signal();
        unsafe {
            libc::signal(libc::SIGTERM, sig_handler as *const () as libc::sighandler_t);
            libc::signal(libc::SIGINT, sig_handler as *const () as libc::sighandler_t);
        }
        let _ = signal;

        // run with a 30s "wake to re-check" floor; epoll still sleeps otherwise
        event_loop
            .run(None, self, |_| {})
            .map_err(|e| format!("loop: {e}"))
    }

    /// Seed the peripheral data that was deferred out of `App::new` so the bar
    /// + launcher appear instantly. Runs once, right after the first frame is
    /// committed. Returns true if any seed changed shell state (caller redraws
    /// only then).
    fn seed_boot_data(&mut self) -> bool {
        let mut changed = false;
        if std::env::var("ZEN_TRACE").is_ok() {
            eprintln!("zen: seeding boot data");
        }
        // real workspace count + active window from Hyprland (3 short IPC
        // queries). The event socket in `start()` takes over from here.
        changed |= self.hypr.update_shell(&mut self.shell);
        // battery / AC from the configured backend
        changed |= self.power.poll(&mut self.shell);
        // brightness + volume so the OSD/pill show real values on first use
        if let Some(b) = self.brightness.get() {
            if (self.shell.brightness - b).abs() > 0.005 {
                self.shell.brightness = b;
                changed = true;
            }
        }
        if let Some((vol, muted)) = self.audio.default_volume() {
            if (self.shell.volume - vol).abs() > 0.005 {
                self.shell.volume = vol;
                changed = true;
            }
            if self.shell.volume_muted != muted {
                self.shell.volume_muted = muted;
                changed = true;
            }
        }
        // seed the L/R channel sliders from $states2/vol_left / vol_right
        // (written by audio_body vol_set_ch on every channel change), so the
        // balance slider restarts where the audio actually is.
        let st2 = crate::vars::states2_dir();
        for side in ["vol_left", "vol_right"] {
            if let Ok(s) = std::fs::read_to_string(st2.join(side)) {
                if let Ok(v) = s.trim().parse::<i32>() {
                    let f = (v as f32 / 100.0).clamp(0.0, 1.0);
                    let dst = if side == "vol_left" {
                        &mut self.shell.vol_left
                    } else {
                        &mut self.shell.vol_right
                    };
                    if (*dst - f).abs() > 0.005 {
                        *dst = f;
                        changed = true;
                    }
                }
            }
        }
        // read max_vol from $states/max_vol (written by Settings + mirrored by
        // the bash audio_body vol_set/vol_step clamp). Default 100 if missing.
        let max_vol_path = crate::vars::states_dir().join("max_vol");
        if let Ok(s) = std::fs::read_to_string(&max_vol_path) {
            if let Ok(mv) = s.trim().parse::<i32>() {
                let mv = mv.clamp(0, 200);
                if self.shell.max_vol != mv {
                    self.shell.max_vol = mv;
                    changed = true;
                }
            }
        }
        // allow_over_100: read from per-state shell config (synced by
        // sync_per_state_config at startup); default true.
        self.shell.allow_over_100 = self.shell.cfg.allow_over_100();
        self.shell.max_vol = self.shell.cfg.max_vol();
        // ssid / media / wifi / bluetooth (D-Bus)
        changed |= self.services.poll(&mut self.shell);
        // clipboard + cliphist history (reads the bbolt dbs, rasterizes image
        // thumbnails) — only visible in the clipboard panels, so seed it while
        // we're already past the first frame.
        changed |= self.clip.seed_history(&mut self.shell, &mut self.img);
        // the resource broker warms on-screen consumers on the next services
        // tick (lyrics / ticker / quote only fetch while their cards are up).
        changed
    }

    // ------------------------------------------------------------------
    // Size dance
    // ------------------------------------------------------------------

    /// Does the compositor already have this size? Layer-shell w/h are u32
    /// (truncated), so a request that truncates to the acked configure is the
    /// SAME size — re-sending it is a no-op Hyprland never acks, which used
    /// to wedge `size_pending` and block every dashboard frame.
    pub(crate) fn size_is_configured(configured: (i32, i32), w: f32, h: f32) -> bool {
        w.trunc() == configured.0 as f32 && h.trunc() == configured.1 as f32
    }

    fn commit_size(&mut self, w: f32, h: f32) {
        // The compositor already knows this size (it is the last acked
        // configure): sending it again is a no-op that Hyprland never acks,
        // which would wedge `size_pending` and block the next morph. This
        // happens every time a morph's completion tick re-offers the final
        // size it just committed — treat it as committed and move on.
        //
        // Compare TRUNCATED (the way layer-shell rounds w/h to u32), not
        // with a 0.5 f32 epsilon: a final tween size of 1009.79 vs an acked
        // configure of 1009 is the SAME surface size — set_size(1009,494) is
        // a no-op Hyprland never acks, and the stuck `size_pending` blocked
        // every dashboard frame ("hover expands to an empty surface").
        if Self::size_is_configured(self.configured, w, h) {
            if std::env::var("ZEN_TRACE").is_ok() {
                eprintln!("zen:   commit_size {}x{} already configured", w, h);
            }
            self.shell.size_pending = false;
            self.shell.note_committed(w, h);
            return;
        }
        if self.shell.size_pending {
            if std::env::var("ZEN_TRACE").is_ok() {
                eprintln!("zen:   commit_size SKIP {w}x{h} (pending)");
            }
            return;
        }
        let Some(layer) = self.layer.as_ref() else { return };
        if std::env::var("ZEN_TRACE").is_ok() {
            eprintln!(
                "zen:   commit_size -> {}x{} cur={:.0}x{:.0} anim={}",
                w, h, self.shell.cur_w, self.shell.cur_h, self.shell.anim.is_some()
            );
        }
        layer.set_size(w.max(1.0) as u32, h.max(1.0) as u32);
        layer.commit(); // size-only commit; buffer attaches on the next swap
        self.shell.size_pending = true;
        self.shell.note_committed(w, h);
        self.last_size_commit = Some(Instant::now());
    }

    fn check_size_timeout(&mut self) {
        // safety net: if the compositor never acked a size change, unstick
        if self.shell.size_pending {
            if let Some(t) = self.last_size_commit {
                if t.elapsed() > Duration::from_millis(800) {
                    self.shell.size_pending = false;
                    self.dirty = true;
                    self.maybe_render();
                }
            }
        }
    }

    /// Bind the quickshell-style IPC socket and register it with calloop.

    /// Accept any pending IPC connections, run the command, reply.

    /// Hovering the workspace box in the dashboard shows that workspace's
    /// window titles; refetch only when the workspace or hover target changed.

    /// THE one hover rule for the pill ⇄ dashboard morph — there is no other
    /// hover logic anywhere, so nothing can fight it:
    ///
    ///   cursor over the bar's (target) surface → expanded; anywhere else →
    ///   collapsed.
    ///
    /// Called from every pointer enter / motion / leave AND the 1s tick, so
    /// the event path and the stale-flow path compute the same `inside` from
    /// the same cursor and feed the same `set_hover` — expand and collapse
    /// can never race. Panels are modal and never participate (a panel open
    /// while the pointer wanders stays open).

    /// Global cursor over the target-projected bar rect — horizontally
    /// centered, pinned to one of the four screen edges per `shell.bar_edge`,
    /// with margin = floating_offset when floating, 0 when stuck flush.
    /// ((mon - target) / 2 centering; left/right edges also center vertically.)
    fn cursor_over_target(&self, cx: f64, cy: f64) -> bool {
        let (tw, th) = self.shell.target_size();
        let margin = self.bar_margin() as f64;
        let (mon_w, mon_h) = self.monitor_dims();
        use crate::shell::BarEdge;
        let (tx, ty) = match self.shell.bar_edge {
            BarEdge::Top => ((mon_w - tw as f64) / 2.0, margin),
            BarEdge::Bottom => ((mon_w - tw as f64) / 2.0, mon_h - margin - th as f64),
            BarEdge::Left => (margin, (mon_h - th as f64) / 2.0),
            BarEdge::Right => (mon_w - margin - tw as f64, (mon_h - th as f64) / 2.0),
        };
        cx >= tx && cx < tx + tw as f64 && cy >= ty && cy < ty + th as f64
    }

    /// The bar's edge margin: `floating_offset` while floating, 0 when the
    /// pill is stuck flush to the top/bottom edge (Floating toggle off).
    fn bar_margin(&self) -> i32 {
        if self.shell.bar_floating {
            self.shell.cfg.bar.floating_offset.max(0)
        } else {
            0
        }
    }

    /// Real logical monitor size, needed to place a left/right-anchored strip
    /// (vertical centering) and to hit-test the bar across all four edges.
    /// `j/monitors` is authoritative; fall back to the cached value.
    fn monitor_dims(&self) -> (f64, f64) {
        if let Some((mw, mh)) = self.hypr.focused_monitor_size() {
            return (mw, mh);
        }
        (self.shell.monitor.0 as f64, self.shell.monitor.1 as f64)
    }

    /// Re-apply the bar layer surface's geometry — anchor and margins follow
    /// `shell.bar_edge` (top/bottom flush; left/right edges keep the strip
    /// horizontal and vertically centered), margin (floating vs. flush), and
    /// exclusive zone. The reserve is PERSISTENT: while "Reserve space" is
    /// on, the same strip stays reserved in every mode — resting pill,
    /// dashboard, panels — so mode morphs never resize it and windows
    /// below/above never bounce. Floating reserves pill height + offset so
    /// nothing tucks under the gap. Called at startup, on a bar move, and on
    /// the Floating / Reserve toggles.
    fn apply_bar_geometry(&mut self) {
        let Some(layer) = self.layer.as_ref() else { return };
        let e = self.shell.bar_edge;
        let m = self.bar_margin();
        let (anchor, mt, mr, mb, ml) = match e {
            crate::shell::BarEdge::Top => (Anchor::TOP, m, 0, 0, 0),
            crate::shell::BarEdge::Bottom => (Anchor::BOTTOM, 0, 0, m, 0),
            crate::shell::BarEdge::Left | crate::shell::BarEdge::Right => {
                // horizontal strip pinned to a vertical edge — center it on
                // the monitor's height, floating margin pushes it off the edge
                let (_, mh) = self.monitor_dims();
                let top_m = ((mh as f32 - self.shell.cfg.bar_h()) / 2.0).max(0.0) as i32;
                match e {
                    crate::shell::BarEdge::Left => (Anchor::LEFT, top_m, 0, 0, m),
                    _ => (Anchor::RIGHT, top_m, m, 0, 0),
                }
            }
        };
        layer.set_anchor(anchor);
        layer.set_margin(mt, mr, mb, ml);
        let reserve = if self.shell.bar_reserve {
            self.shell.cfg.bar_h() as i32 + m
        } else {
            -1
        };
        layer.set_exclusive_zone(reserve);
        layer.commit();
        let _ = self.conn.flush();
        if std::env::var("ZEN_TRACE").is_ok() {
            eprintln!(
                "zen: bar geometry edge={} margin=({mt},{mr},{mb},{ml}) reserve={reserve}",
                e.name()
            );
        }
    }

    // ------------------------------------------------------------------
    // Morph
    // ------------------------------------------------------------------

    /// Whether a pointer position (surface-local) is over the surface as the
    /// compositor last configured it.

    /// Perform actions queued by the shell (wifi/bt toggles, volume,
    /// brightness, per-app audio, wallpaper) — each through its backend.
    fn run_pending_actions(&mut self) {
        // Settings → Appearance → "Icon font": swap the icon slot table AND
        // the glyph family at runtime — no restart, no recompile. The shell
        // reports are redrawn with the new engine on the next frame.
        if let Some(style) = self.shell.pending_icon_style.take() {
            self.shell.cfg.fonts.icon_style = style;
            // the family follows the chosen style so the slot table and the
            // rendered glyph family always agree (fonts.icon override synced).
            let style_obj = self.shell.cfg.icon_style();
            self.shell.cfg.fonts.icon = style_obj.family().to_string();
            self.shell.save_config();
            let fam = self.shell.cfg.family();
            let ifam = self.shell.cfg.icon_family();
            let slots = crate::icons::slot_map(style_obj);
            self.text = Some(TextEngine::new_with_slots(
                fam,
                self.shell.cfg.font_display(),
                self.shell.cfg.font_mono(),
                crate::vars::read_branding_font(),
                ifam,
                slots,
            ));
            self.dirty = true;
        }
        if let Some(ssid) = self.shell.pending_wifi.take() {
            self.services.connect_wifi(&ssid);
        }
        if let Some(name) = self.shell.pending_bt.take() {
            self.services.toggle_bt(&name);
        }
        if let Some(on) = self.shell.pending_wifi_power.take() {
            self.services.set_wifi_powered(on);
        }
        if let Some(on) = self.shell.pending_bt_power.take() {
            self.services.set_bt_powered(on);
        }
        if let Some(vol) = self.shell.pending_volume.take() {
            self.audio.set_default_volume(vol);
            // when main volume commits, L/R channels were reset to `vol` on the
            // drag — mirror the value to the per-channel state files so the
            // balance slider reads match the device (audio_body writes these
            // too, but only for an actual channel drag).
            let st2 = crate::vars::states2_dir();
            let v = ((self.shell.vol_left * 100.0).round().clamp(0.0, self.shell.max_vol as f32)) as i32;
            let _ = std::fs::write(st2.join("vol_left"), format!("{}\n", v));
            let _ = std::fs::write(st2.join("vol_right"), format!("{}\n", v));
        }
        if let Some(mv) = self.shell.pending_max_vol.take() {
            // persist to the per-state shell config ($states/shell_{n,d,l}) so
            // it survives a session — the bash audio_body reads $states/max_vol
            // at the file level, so also mirror it there for the terminal CLI
            let per_state = crate::vars::per_state_shell_path(crate::vars::read_channel());
            if let Ok(cfg) = std::fs::read_to_string(&per_state) {
                if let Ok(mut t) = toml::from_str::<crate::config::Config>(&cfg) {
                    t.bar.max_vol = Some(mv);
                    t.bar.allow_over_100 = Some(self.shell.allow_over_100);
                    let toml = toml::to_string_pretty(&t).unwrap_or_default();
                    let _ = std::fs::write(&per_state, toml);
                }
            }
            // mirror to $states/max_vol so the bash audio_body vol_set/vol_step
            // clamp against it (terminal CLI, keybinds, dash_status all agree)
            let max_vol_path = crate::vars::states_dir().join("max_vol");
            let _ = std::fs::write(&max_vol_path, format!("{}\n", mv));
            if std::env::var("ZEN_TRACE").is_ok() {
                eprintln!("zen: max_vol = {mv}");
            }
        }
        // L/R channel sliders (dashboard): live flushes use the native
        // wpctl channel-set (one spawn per tick, shell already has both
        // values); $states2/vol_left|vol_right are mirrored here so CLI and
        // dash_status agree with the sink.
        if let (Some(lv), Some(rv)) = (
            self.shell.pending_vol_left.take(),
            self.shell.pending_vol_right.take(),
        ) {
            self.audio.set_channels(lv, rv);
            let st2 = crate::vars::states2_dir();
            let vl = (lv * 100.0).round().clamp(0.0, self.shell.max_vol as f32) as i32;
            let vr = (rv * 100.0).round().clamp(0.0, self.shell.max_vol as f32) as i32;
            let _ = std::fs::write(st2.join("vol_left"), format!("{vl}\n"));
            let _ = std::fs::write(st2.join("vol_right"), format!("{vr}\n"));
        }
        if let Some(b) = self.shell.pending_brightness.take() {
            self.brightness.set(b);
        }
        // bar geometry changes: ▲/▼ move, Floating / Reserve space toggles
        if self.shell.pending_move.take().is_some()
            || self.shell.pending_floating.take().is_some()
            || self.shell.pending_reserve.take().is_some()
        {
            self.apply_bar_geometry();
        }
        // workspace dispatch to Hyprland
        if let Some(id) = self.shell.pending_ws_dispatch.take() {
            self.hypr.dispatch_workspace(id);
            // show "Workspace N" OSD when workspace dots are hidden
            if !self.shell.pill_ws && self.shell.mode == crate::shell::Mode::Collapsed {
                self.show_osd(ICON_GLOBE, format!("Workspace {id}"));
            }
        }
        // mirror card: take photo / toggle recording / cycle fps
        if std::mem::take(&mut self.shell.pending_mirror_photo) {
            self.mirror_snap_photo();
        }
        if std::mem::take(&mut self.shell.pending_mirror_rec) {
            self.toggle_mirror_rec();
        }
        if std::mem::take(&mut self.shell.pending_mirror_fps) {
            self.cycle_mirror_fps();
        }
        if let Some((id, vol)) = self.shell.pending_audio_volume.take() {
            self.audio.set_volume(id, vol);
        }
        if std::mem::take(&mut self.shell.pending_audiorec_rec) {
            self.toggle_audiorec_rec();
        }
        if std::mem::take(&mut self.shell.pending_audiorec_list) {
            self.refresh_audiorec_list();
        }
        if let Some(i) = self.shell.pending_audiorec_play.take() {
            if let Some(path) = self.shell.audiorec_recordings.get(i).map(|(_, p)| p.clone()) {
                self.play_audiorec_file(&path);
            }
        }
        if let Some(i) = self.shell.pending_audiorec_del.take() {
            if let Some((_, path)) = self.shell.audiorec_recordings.get(i) {
                let _ = std::fs::remove_file(path);
                self.shell.audiorec_del_armed = None;
                self.refresh_audiorec_list();
                self.show_osd(ICON_MIC, format!("Deleted  {}", self.shell.audiorec_recordings.get(i).map(|(n, _)| n.as_str()).unwrap_or("")));
            }
        }
        if let Some(id) = self.shell.pending_audio_mute.take() {
            self.audio.toggle_mute(id);
        }
        if let Some(id) = self.shell.pending_audio_default.take() {
            self.audio.set_default(id);
        }
        if let Some(path) = self.shell.pending_wallpaper.take() {
            let mon = self.hypr.focused_monitor();
            if std::env::var("ZEN_TRACE").is_ok() {
                eprintln!("zen: wallpaper: {path}");
            }
            self.wallpaper.set(&path, mon.as_deref());
        }
        // news headline click → open the article in the default browser
        if let Some(url) = self.shell.pending_news_open.take() {
            use std::process::Stdio;
            if std::env::var("ZEN_TRACE").is_ok() {
                eprintln!("zen: news open: {url}");
            }
            let _ = std::process::Command::new("xdg-open")
                .arg(url)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn();
        }
        // speedtest card: run → spawn the Cloudflare probe worker
        if std::mem::take(&mut self.shell.pending_speed_test) {
            if let Some(tx) = self.speed_tx.clone() {
                std::thread::spawn(move || {
                    let (d, u, p) = crate::shell::Shell::speedtest_probe();
                    let _ = tx.send(crate::shell::SpeedMsg::Done { down_mbps: d, up_mbps: u, ping_ms: p });
                });
            }
        }
        // quote card: refresh → fetch a new quote
        if std::mem::take(&mut self.shell.pending_quote_fetch) {
            if let Some(tx) = self.quote_tx.clone() {
                std::thread::spawn(move || {
                    let _ = match crate::shell::Shell::fetch_quote() {
                        Some((text, author)) => tx.send(crate::shell::QuoteMsg::Quote { text, author }),
                        None => tx.send(crate::shell::QuoteMsg::Failed),
                    };
                });
            }
        }
        // recent card: row → open the file
        if let Some(path) = self.shell.pending_recent_open.take() {
            Shell::open_path(&path);
        }
        // ticker card: row → open the asset's market page
        if let Some(url) = self.shell.pending_ticker_open.take() {
            Shell::open_path(&url);
        }
        // lockscreen unlock (native backend: collapsing the bar is the unlock;
        // external backends like hyprlock get the signal to tear down)
        if std::mem::take(&mut self.shell.pending_unlock) {
            self.lock.unlock();
        }
        // clipboard history row → re-copy that entry (idx, is_image)
        if let Some((idx, is_img)) = self.shell.pending_clip.take() {
            self.clip.set_entry(idx, is_img, &self.qh);
        }
        // snippets row → copy the clip body (named after the snippet)
        if let Some(i) = self.shell.pending_snippet_copy.take() {
            if let Some((_, body)) = self.shell.snippets.get(i) {
                self.clip.copy_text(body.clone(), &self.qh);
            }
        }
        // compositor card screenshot button → shot_main region|full
        if let Some(mode) = self.shell.pending_shot.take() {
            use std::process::{Command, Stdio};
            let _ = Command::new("shot_main")
                .arg(&mode)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn();
        }
        // dashboard quick tiles / vertical sliders → script-backed actions
        if let Some(cmd) = self.shell.pending_dash.take() {
            self.run_dash_cmd(cmd);
        }
        // theme card click → scheme_main apply <name>
        // Writes $states/scheme_${cm}, resets acc_source to default, then
        // calls theme_main restore so the palette regenerates.
        if let Some(name) = self.shell.pending_theme_apply.take() {
            use std::process::{Command, Stdio};
            let _ = Command::new("scheme_main")
                .args(["apply", &name])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn();
            self.schedule_colors_reload();
        }
        // Settings → Appearance theme-accent click → `scheme_main accent <kind>
        // [payload]`. scheme_main_body writes `$states/acc_source_<ch>` (w/c/h/d)
        // and, on an actual change, echoes `$states2/acc_changed` + restores so
        // GTK (thunar) picks up the new accent; the delayed colors-reload then
        // re-reads the live palette here without a restart.
        if let Some((kind, payload)) = self.shell.pending_accent_src.take() {
            use std::process::{Command, Stdio};
            let mut c = Command::new("scheme_main");
            c.arg("accent").arg(&kind)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null());
            if !payload.is_empty() {
                c.arg(&payload);
            }
            let _ = c.spawn();
            self.schedule_colors_reload();
        }
        // Settings → Appearance → "Pick from screen", or a preset circle: runs
        // `pick_accent_main [hex]`. With no arg it launches hyprpicker (Esc =
        // discard); with a hex it sets that preset directly. Either way it
        // stores `$states/custom_acc_${s}` + source `c`, marks acc_changed and
        // restores. Reload colors once the pipeline settles.
        if let Some(hex) = std::mem::take(&mut self.shell.pending_pick_accent) {
            use std::process::{Command, Stdio};
            let mut c = Command::new("pick_accent_main");
            c.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
            if !hex.is_empty() {
                c.arg(&hex);
            }
            let _ = c.spawn();
            self.schedule_colors_reload();
        }
        // Settings → Appearance → Accent toggles / Alpha: run
        // `theme_main <sub> <args…>` (toggle_acc | start_icon_tone | alpha |
        // acc_scrim — all added to theme_main_body). Reload colors after the
        // pipeline settles so the new flags / palette render.
        if let Some(args) = self.shell.pending_theme_cmd.take() {
            use std::process::{Command, Stdio};
            let _ = Command::new("theme_main")
                .args(&args)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn();
            self.schedule_colors_reload();
        }
        // battery card power-save button → CPU-governor toggle (sysfs, pkexec
        // fallback). Applied then marked dirty so the button reflects it.
        if std::mem::take(&mut self.shell.pending_power_save) {
            let changed = self.shell.toggle_power_save();
            if changed {
                self.dirty = true;
            }
        }
    }

    /// Run a script-backed dashboard action (quick-toggle tile or vertical
    /// slider). The *_main helpers are on PATH; spawn detached so a slow
    /// backend never stalls the event loop.
    fn run_dash_cmd(&mut self, cmd: crate::shell::DashCmd) {
        use std::process::{Command, Stdio};
        use crate::shell::DashCmd;
        // Brightness is baked into the shell's own backend (sysfs or logind);
        // the dashboard fader applies it directly instead of spawning
        // `brightness_main`. The other *_main helpers remain script-backed.
        let (prog, args): (&str, Vec<String>) = match cmd {
            DashCmd::Brightness(pct) => {
                self.brightness.set((pct as f32).clamp(0.0, 100.0) / 100.0);
                return;
            }
            DashCmd::Volume(pct) => (
                "audio_main",
                vec!["vol".into(), "set".into(), pct.to_string()],
            ),
            DashCmd::Mic(pct) => (
                "audio_main",
                vec!["mic".into(), "set".into(), pct.to_string()],
            ),
            DashCmd::MicToggle => ("audio_main", vec!["mic".into(), "toggle".into()]),
            DashCmd::Saturation(n) => {
                ("shader_main", vec!["set".into(), n.to_string()])
            }
            DashCmd::Kelvin(k) => ("hs_main", vec!["set".into(), k.to_string()]),
            DashCmd::CaffeineToggle => ("caffeine_main", vec!["toggle".into()]),
            DashCmd::SunsetToggle => ("hs_main", vec!["toggle".into()]),
            DashCmd::ShaderToggle => ("shader_main", vec!["toggle".into()]),
            DashCmd::BlurToggle => ("blur_main", vec!["toggle".into()]),
            DashCmd::ShadowToggle => ("shadow_main", vec!["toggle".into()]),
            DashCmd::OpacityToggle => ("opacity_main", vec!["toggle".into()]),
            DashCmd::ThemeSwitch => {
                self.schedule_colors_reload();
                ("theme_main", vec!["switch".into()])
            }
        };
        if std::env::var("ZEN_TRACE").is_ok() {
            eprintln!("zen: dash cmd: {prog} {}", args.join(" "));
        }
        let _ = Command::new(prog)
            .args(&args)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .stdin(Stdio::null())
            .spawn();
    }

    /// After a theme pipeline run (`theme_main restore` / `theme_main switch`) the new palette
    /// lands in $states2 a moment later; poke our own IPC once it settled so
    /// the bar re-reads every color without a restart.
    fn schedule_colors_reload(&mut self) {
        std::thread::spawn(|| {
            std::thread::sleep(std::time::Duration::from_millis(1500));
            crate::ipc::poke("call colors reload");
            crate::ipc::poke("call mode sync");
        });
    }

    /// Kick off one `dash_status` run (worker thread) while the dashboard is
    /// open. At most one in flight; results arrive via apply_dash_status.
    fn poll_dash_status(&mut self) {
        if self.dash_inflight.load(std::sync::atomic::Ordering::Acquire) {
            return;
        }
        let Some(tx) = self.dash_tx.clone() else { return };
        self.dash_inflight.store(true, std::sync::atomic::Ordering::Release);
        let inflight = self.dash_inflight.clone();
        std::thread::spawn(move || {
            let out = std::process::Command::new("dash_status")
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::null())
                .output();
            inflight.store(false, std::sync::atomic::Ordering::Release);
            if let Ok(o) = out {
                let _ = tx.send(String::from_utf8_lossy(&o.stdout).into_owned());
            }
        });
    }

    /// Apply a `dash_status` report line:
    /// bri% vol% mic% micmut(0|1) sat shaderon(0|1) sunset_state(c|a|m) kelvin
    /// caffeine(on|off) blur(0|1) shadow(0|1) opacity(0|1)
    /// Skips slider values while the user is dragging them (the drag owns the
    /// value until release).
    fn apply_dash_status(&mut self, line: &str) {
        let f: Vec<&str> = line.split_whitespace().collect();
        if f.len() < 9 {
            return;
        }
        let num = |s: &str| s.trim_end_matches('%').parse::<f32>().unwrap_or(0.0);
        let dragging = self.pointer_down && self.shell.drag_key.is_some();
        let mut changed = false;
        if !dragging {
            let bri = num(f[0]) / 100.0;
            if (self.shell.brightness - bri).abs() > 0.005 {
                self.shell.brightness = bri;
                changed = true;
            }
            let vol = num(f[1]) / 100.0;
            if (self.shell.volume - vol).abs() > 0.005 {
                self.shell.volume = vol;
                changed = true;
            }
            let mic = num(f[2]) / 100.0;
            if (self.shell.mic_level - mic).abs() > 0.005 {
                self.shell.mic_level = mic;
                changed = true;
            }
        }
        let mic_muted = f[3] == "1";
        if self.shell.mic_muted != mic_muted {
            self.shell.mic_muted = mic_muted;
            changed = true;
        }
        let sat = f[4].parse::<i32>().unwrap_or(100).clamp(0, 200);
        if !dragging && self.shell.saturation != sat {
            self.shell.saturation = sat;
            changed = true;
        }
        let shader_on = f[5] == "1";
        if self.shell.shader_on != shader_on {
            self.shell.shader_on = shader_on;
            changed = true;
        }
        let sunset_on = f[6] == "a" || f[6] == "m";
        if self.shell.sunset_on != sunset_on {
            self.shell.sunset_on = sunset_on;
            changed = true;
        }
        let kelvin = f[7].parse::<i32>().unwrap_or(4000);
        if !dragging && self.shell.sunset_kelvin != kelvin {
            self.shell.sunset_kelvin = kelvin;
            changed = true;
        }
        let caff = f[8] == "on";
        if self.shell.caffeine_on != caff {
            self.shell.caffeine_on = caff;
            changed = true;
        }
        // effects states (fields 9-11; absent in an older dash_status)
        if f.len() >= 12 {
            let blur = f[9] == "1";
            if self.shell.fx_blur_on != blur {
                self.shell.fx_blur_on = blur;
                changed = true;
            }
            let shadow = f[10] == "1";
            if self.shell.fx_shadow_on != shadow {
                self.shell.fx_shadow_on = shadow;
                changed = true;
            }
            let opacity = f[11] == "1";
            if self.shell.fx_opacity_on != opacity {
                self.shell.fx_opacity_on = opacity;
                changed = true;
            }
        }
        if changed {
            if std::env::var("ZEN_TRACE").is_ok() {
                eprintln!(
                    "zen: dash status bri={:.0}% vol={:.0}% mic={:.0}%{} sat={} shader={} sunset={}({}K) caffeine={} fx={}{}/{}",
                    self.shell.brightness * 100.0,
                    self.shell.volume * 100.0,
                    self.shell.mic_level * 100.0,
                    if self.shell.mic_muted { " muted" } else { "" },
                    self.shell.saturation,
                    self.shell.shader_on as i32,
                    self.shell.sunset_on as i32,
                    self.shell.sunset_kelvin,
                    self.shell.caffeine_on as i32,
                    self.shell.fx_blur_on as i32,
                    self.shell.fx_shadow_on as i32,
                    self.shell.fx_opacity_on as i32,
                );
            }
            self.dirty = true;
            self.maybe_render();
        }
    }

    /// Check the lockscreen password against the system (PAM) on a worker
    /// thread — pam_authenticate can block (retries, faillock backoff, …) and
    /// must never stall the event loop. The result comes back on a channel and
    /// is handled by `on_lock_auth`. If PAM is genuinely unavailable, log and
    /// unlock rather than trapping the user on a screen they can never leave.

    /// Handle a threaded PAM result: unlock on success, show the error and
    /// stay locked on denial, and never trap the user when PAM is missing.

    /// End the lockscreen: collapse the bar and release the session lock (the
    /// compositor then sends `finished`, which tears the lock surface down).

    /// World-map card: probe the UTC offset + abbreviation for a zone with a
    /// single `TZ=<zone> date +%z %Z` process. Writes the result into the
    /// shell (drive the marker's clock at draw time; no per-frame spawns).
    fn world_shell_probe(&mut self, tz: &str) {
        if std::env::var("ZEN_TRACE").is_ok() {
            eprintln!("zen: world tz probe: {tz}");
        }
        let out = std::process::Command::new("date")
            .env("TZ", tz)
            .arg("+%z %Z")
            .output();
        if let Ok(o) = out {
            if o.status.success() {
                let line = String::from_utf8_lossy(&o.stdout);
                let mut it = line.split_whitespace();
                let off = it.next().unwrap_or("");
                let abbr = it.next().unwrap_or("").to_string();
                if let Some(min) = crate::shell::worldmap::parse_tz_offset(off) {
                    self.shell.world_offset_min = min;
                    self.shell.world_abbrev = abbr;
                    self.shell.world_off_at = Some(std::time::Instant::now());
                    self.dirty = true;
                    self.maybe_render();
                }
            }
        }
    }

    /// World-map data maintenance, run from the 3 s services timer: boot-load
    /// the cached world chunk into the runtime store, spawn worker threads for
    /// any pending per-chunk downloads, then drain finished results. Returns
    /// true when the shell needs a redraw.
    fn world_tick(&mut self) -> bool {
        use crate::shell::mapdata;
        let mut changed = false;
        let save = self.shell.world_save_data;
        let base = mapdata::base_url(&self.shell.cfg.world.data_url);
        // 1) cached world chunk (previous download) → renderable immediately.
        // Run whenever the store is empty OR still holds just the embedded
        // fallback (a downloaded dataset must win over the fallback on boot).
        if !crate::shell::worldmap::ready()
            || crate::shell::worldmap::using_fallback()
        {
            changed |= self.world_load_from_disk(save, &base);
        }
        // 1b) auto-load a cached region chunk once the map is zoomed in
        if crate::shell::worldmap::ready() && self.shell.world_zoom >= 4.0 {
            if let Some(man) = self.shell.world_manifest.clone() {
                let (clon, clat) = self
                    .shell
                    .world_marker
                    .map(|(la, lo)| (lo, la))
                    .unwrap_or(self.shell.world_center);
                if let Some((iso, reg)) = mapdata::region_at(&man, clon, clat) {
                    if self.shell.world_region.as_deref() != Some(iso.as_str()) {
                        let meta = mapdata::Meta {
                            url: reg.url.clone(),
                            sha256: reg.sha256.clone(),
                            size: reg.size,
                        };
                        let p = mapdata::region_path(save, &iso);
                        if mapdata::file_valid(&p, &meta) {
                            if let Ok(bytes) = std::fs::read(&p) {
                                if let Some(rings) = crate::shell::worldmap::parse_ring_chunk(&bytes) {
                                    self.shell.world_region = Some(iso.clone());
                                    self.shell.world_region_rings = Some(rings);
                                    self.shell.world_region_target = None;
                                    self.shell.world_rev = self.shell.world_rev.wrapping_add(1);
                                    changed = true;
                                }
                            }
                        }
                    }
                }
            }
        }
        // 2) spawn pending downloads (base chunk or one country chunk)
        if let Some(dl) = self.shell.world_dl_pending.take() {
            self.world_dl_spawn(base, save, dl);
        }
        // 3) drain finished downloads
        if let Some(rx) = self.world_dl_rx.take() {
            while let Ok(res) = rx.try_recv() {
                changed |= self.world_apply(res);
            }
            self.world_dl_rx = Some(rx);
        }
        changed
    }

    /// Load the cached world + cities chunks and the manifest into the shell.
    fn world_load_from_disk(&mut self, save: bool, base: &str) -> bool {
        use crate::shell::mapdata;
        let Some(m) = mapdata::load_manifest(save, base) else { return false };
        if !mapdata::file_valid(&mapdata::world_path(save), &m.world)
            || !mapdata::file_valid(&mapdata::cities_path(save), &m.cities)
        {
            return false;
        }
        let (Ok(wb), Ok(cb)) = (
            std::fs::read(&mapdata::world_path(save)),
            std::fs::read(&mapdata::cities_path(save)),
        ) else {
            return false;
        };
        let (Some(rings), Some(cs)) = (
            crate::shell::worldmap::parse_ring_chunk(&wb),
            crate::shell::worldmap::parse_cities_chunk(&cb),
        ) else {
            return false;
        };
        crate::shell::worldmap::set_world(rings);
        crate::shell::worldmap::set_cities(cs);
        self.shell.world_manifest = Some(std::sync::Arc::new(m));
        self.shell.world_rev = self.shell.world_rev.wrapping_add(1);
        true
    }

    /// Spawn a curl worker for one pending chunk download. Results arrive on
    /// the world-download channel and are applied by `world_apply`.
    fn world_dl_spawn(&mut self, base: String, save: bool, dl: crate::shell::WorldDl) {
        use crate::shell::mapdata;
        if self.world_dl_tx.is_none() {
            let (tx, rx) = std::sync::mpsc::channel::<mapdata::DlResult>();
            self.world_dl_tx = Some(tx);
            self.world_dl_rx = Some(rx);
        }
        let Some(tx) = self.world_dl_tx.clone() else { return };
        if std::env::var("ZEN_TRACE").is_ok() {
            eprintln!("zen: world download: {dl:?}");
        }
        std::thread::spawn(move || {
            let res: mapdata::DlResult = match dl {
                crate::shell::WorldDl::Base => {
                    match mapdata::load_manifest(save, &base) {
                        Some(m) => {
                            let world_ok = mapdata::file_valid(&mapdata::world_path(save), &m.world)
                                || mapdata::download_chunk(&base, &m.world, &mapdata::world_path(save)).is_ok();
                            let cities_ok = mapdata::file_valid(&mapdata::cities_path(save), &m.cities)
                                || mapdata::download_chunk(&base, &m.cities, &mapdata::cities_path(save)).is_ok();
                            if world_ok && cities_ok {
                                mapdata::DlResult::WorldOk
                            } else {
                                mapdata::DlResult::Failed { why: "base chunk download failed".into() }
                            }
                        }
                        None => mapdata::DlResult::Failed { why: "manifest unavailable".into() },
                    }
                }
                crate::shell::WorldDl::Region(iso) => {
                    match mapdata::load_manifest(save, &base) {
                        Some(m) => match m.regions.get(&iso) {
                            Some(reg) => {
                                let meta = mapdata::Meta { url: reg.url.clone(), sha256: reg.sha256.clone(), size: reg.size };
                                let ok = mapdata::download_chunk(&base, &meta, &mapdata::region_path(save, &iso));
                                if ok.is_ok() {
                                    mapdata::DlResult::RegionOk { iso }
                                } else {
                                    mapdata::DlResult::Failed { why: ok.unwrap_err() }
                                }
                            }
                            None => mapdata::DlResult::Failed { why: "region not in manifest".into() },
                        },
                        None => mapdata::DlResult::Failed { why: "manifest unavailable".into() },
                    }
                }
            };
            let _ = tx.send(res);
        });
    }

    /// Apply one finished download result to the shell.
    fn world_apply(&mut self, res: crate::shell::mapdata::DlResult) -> bool {
        use crate::shell::mapdata;
        let mut changed = false;
        match res {
            mapdata::DlResult::WorldOk => {
                let save = self.shell.world_save_data;
                let base = mapdata::base_url(&self.shell.cfg.world.data_url);
                changed = self.world_load_from_disk(save, &base);
                self.shell.world_dl_err = None;
                if changed && std::env::var("ZEN_TRACE").is_ok() {
                    eprintln!("zen: world map ready");
                }
            }
            mapdata::DlResult::RegionOk { iso } => {
                if self.shell.world_manifest.is_none() {
                    let save = self.shell.world_save_data;
                    let base = mapdata::base_url(&self.shell.cfg.world.data_url);
                    let _ = &save;
                    let _ = mapdata::load_manifest(save, &base)
                        .map(|m| self.shell.world_manifest = Some(std::sync::Arc::new(m)));
                }
                let save = self.shell.world_save_data;
                let p = mapdata::region_path(save, &iso);
                if let Ok(bytes) = std::fs::read(&p) {
                    if let Some(rings) = crate::shell::worldmap::parse_ring_chunk(&bytes) {
                        self.shell.world_region = Some(iso.clone());
                        self.shell.world_region_rings = Some(rings);
                        self.shell.world_region_target = None;
                        self.shell.world_dl_err = None;
                        self.shell.world_rev = self.shell.world_rev.wrapping_add(1);
                        changed = true;
                        if std::env::var("ZEN_TRACE").is_ok() {
                            eprintln!("zen: world region {iso} ready");
                        }
                    }
                }
            }
            mapdata::DlResult::Failed { why, .. } => {
                self.shell.world_dl_err = Some(why);
                changed = true;
            }
        }
        if changed {
            self.dirty = true;
            self.maybe_render();
        }
        changed
    }

    /// Show a transient OSD — only morphs from the resting pill (a panel in
    /// the way is its own feedback; never yank it away). Auto-dismisses after
    /// ~1.4 s back to the pill.
    fn show_osd(&mut self, glyph: &'static str, text: String) {
        use crate::shell::Mode;
        if self.shell.mode != Mode::Collapsed {
            return;
        }
        self.shell.osd = Some(crate::shell::OsdInfo { glyph, text, prev: Mode::Collapsed });
        self.apply_mode(Mode::Osd);
        self.arm_osd_timer();
    }

    /// One-shot: morph the OSD back to the pill after ~1.4 s. Re-arms from
    /// scratch so a burst of OSDs (e.g. volume keys held down) extends it.
    fn arm_osd_timer(&mut self) {
        if let Some(tok) = self.osd_timer.take() {
            if let Some(h) = self.loop_handle.clone() {
                h.remove(tok);
            }
        }
        let Some(handle) = self.loop_handle.clone() else { return };
        let timer = calloop::timer::Timer::from_duration(Duration::from_millis(1400));
        let token = handle
            .insert_source(timer, |_, _, app: &mut App| {
                if let Some(tok) = app.osd_timer.take() {
                    if let Some(h) = app.loop_handle.clone() {
                        h.remove(tok);
                    }
                }
                if app.shell.mode == crate::shell::Mode::Osd {
                    let prev = app.shell.osd.as_ref().map(|o| o.prev).unwrap_or(crate::shell::Mode::Collapsed);
                    app.shell.osd = None;
                    app.apply_mode(prev);
                }
                calloop::timer::TimeoutAction::Drop
            })
            .expect("osd timer");
        self.osd_timer = Some(token);
    }

    /// Apply a mode transition requested by a click / key, driving the morph.
    fn apply_mode(&mut self, mode: crate::shell::Mode) {
        // locking: the hyprlock backend hands off to the external locker and
        // leaves the bar alone. The native backend grabs the session through
        // ext-session-lock (a fullscreen lock surface — this is what tells
        // Hyprland the session is locked, so its own keybinds stop firing and
        // ALL input goes to the lock surface); when the protocol is missing it
        // falls back to morphing the bar fullscreen + an exclusive keyboard
        // grab (visual only — compositor binds still fire).
        if mode == crate::shell::Mode::Lock {
            if self.shell.cfg.lockscreen_backend() == "hyprlock" {
                self.lock.lock();
                return;
            }
            // try the real session lock first; the flag tells shell::set_mode
            // whether the bar must stay put (session) or morph (fallback). A
            // second lock request (SUPER+L while already locked) must NOT
            // create a new SessionLock — its `finished` would tear down the
            // active one.
            self.shell.lock_session = false;
            if self.session_lock_active.is_some() {
                self.shell.lock_session = true;
            } else {
                match self.session_lock.lock(&self.qh) {
                    Ok(l) => {
                        if std::env::var("ZEN_TRACE").is_ok() {
                            eprintln!("zen: session locked via ext-session-lock");
                        }
                        self.session_lock_active = Some(l);
                        self.shell.lock_session = true;
                    }
                    Err(e) => {
                        eprintln!("zen: ext-session-lock unavailable ({e}) — bar-morph fallback");
                    }
                }
            }
            if let Some((x, y, w, h)) = self.hypr.layer_geom() {
                let margin = self.bar_margin() as f64;
                self.shell.monitor = ((2.0 * x + w) as f32, (y + margin + h) as f32);
            }
        }
        // An explicit collapse (selection made, Esc, IPC / click close,
        // clicked elsewhere) disarms hover-expansion: the pointer often RESTS
        // over the pill strip right where the panel was, and the 1s hover
        // reconcile would otherwise instantly re-expand the dashboard — the
        // "panel stays big for a few seconds after selecting" bug. The next
        // real pointer movement re-arms it (Motion / Enter handlers).
        if mode == crate::shell::Mode::Collapsed {
            self.hover_armed = false;
        }
        self.shell.set_mode(mode);
        // the equalizer only runs while the dashboard is visible + playing
        self.sync_viz_timer();
        // dashboard background feeds (sensors / mirror camera) live and die
        // with the dashboard; News/Ticker/Quote/WiFi are broker-managed now
        self.sync_sensor_timer();
        self.sync_cam_timer();
        // edit-mode auto-arrange timer (3 s left-gap fill while editing)
        self.sync_autoarrange_timer();
        // NB: the exclusive-zone reserve is persistent across modes — mode
        // flips must never resize it or windows below/above bounce.
        // opening the network menu asks NetworkManager for a fresh scan
        if mode == crate::shell::Mode::WifiMenu {
            self.services.request_wifi_scan();
        }
        // opening the audio panel populates it immediately (then the 3s timer
        // keeps it fresh while it's open)
        if mode == crate::shell::Mode::Audio {
            self.audio.poll(&mut self.shell);
        }
        // Modal panels grab EXCLUSIVE keyboard so typed characters reach the
        // launcher/command search box, and so clicking any other window drops
        // focus — a precise "clicked elsewhere" signal (keyboard-leave) for
        // every panel. The hover-driven dashboard and the toast stay OnDemand.
        // Takes effect on the next surface commit.
        if let Some(layer) = self.layer.as_ref() {
            let ki = if matches!(
                mode,
                crate::shell::Mode::Launcher
                    | crate::shell::Mode::ControlCenter
                    | crate::shell::Mode::Notifications
                    | crate::shell::Mode::Calendar
                    | crate::shell::Mode::Settings
                    | crate::shell::Mode::Power
                    | crate::shell::Mode::WifiMenu
                    | crate::shell::Mode::BtMenu
                    | crate::shell::Mode::Audio
                    | crate::shell::Mode::Wallpaper
                    | crate::shell::Mode::Weather
                    | crate::shell::Mode::Clipboard
                    | crate::shell::Mode::ClipboardImages
                    | crate::shell::Mode::Themes
                    | crate::shell::Mode::Lock
            ) && !(mode == crate::shell::Mode::ControlCenter && self.shell.cc_as_dashboard) {
                // Exception: when the Control Center is the hover-driven
                // "dashboard" (Settings → Open Control Center), it must stay
                // OnDemand — an exclusive grab while hover-opening would steal
                // the keyboard every time the pointer crosses the pill.
                KeyboardInteractivity::Exclusive
            } else {
                KeyboardInteractivity::OnDemand
            };
            layer.set_keyboard_interactivity(ki);
        }
        self.dirty = true;
        self.ensure_morph_timer();
        self.maybe_render();
    }

    fn ensure_morph_timer(&mut self) {
        if self.morph_timer.is_some() {
            return;
        }
        // Also start the timer when there's no anim but the committed size
        // doesn't match the target (e.g. toggling edit mode which snaps
        // cur_w/cur_h instantly — the compositor still needs to be told), or
        // while the dashboard scrollbar overlay still needs its fade-out
        // frames (it has no other live source of repaints at rest).
        if self.shell.anim.is_none() && !self.shell.dash_sb_pending_frames(Instant::now()) {
            let (tw, th) = self.shell.target_size();
            let (lw, lh) = self.shell.last_committed;
            if (tw - lw).abs() < 0.5 && (th - lh).abs() < 0.5 {
                return;
            }
        }
        let Some(handle) = self.loop_handle.clone() else { return };
        let qh = self.qh.clone();
        let conn = self.conn.clone();
        let timer = calloop::timer::Timer::from_duration(Duration::from_millis(16));
        let handle2 = handle.clone();
        let token = handle
            .insert_source(timer, move |_, _, app: &mut App| {
                app.morph_tick(&qh, &conn, &handle2);
                calloop::timer::TimeoutAction::ToDuration(Duration::from_millis(16))
            })
            .expect("morph timer");
        self.morph_timer = Some(token);
    }

    fn morph_tick(
        &mut self,
        qh: &QueueHandle<App>,
        conn: &Connection,
        handle: &LoopHandle<'static, App>,
    ) {
        let now = Instant::now();
        // If a size commit was never acked, `size_pending` stays set and
        // `maybe_render` silently drops every redraw — the pill freezes. The
        // 60s clock tick also runs this, but a morph in a long session must
        // unstick itself within the 800ms budget, not stall for a minute.
        self.check_size_timeout();
        if std::env::var("ZEN_TRACE").is_ok() {
            match self.shell.anim.as_ref() {
                Some(a) => eprintln!(
                    "zen: morph_tick now={:?} anim_start_ago={:?} dur={} from={:.0}x{:.0} to={:.0}x{:.0} cur={:.0}x{:.0}",
                    now, now.duration_since(a.start), a.dur, a.from_w, a.from_h, a.to_w, a.to_h,
                    self.shell.cur_w, self.shell.cur_h
                ),
                None => eprintln!("zen: morph_tick anim=none cur={:.0}x{:.0}", self.shell.cur_w, self.shell.cur_h),
            }
        }
        if let Some((w, h)) = self.shell.tick(now) {
            if std::env::var("ZEN_TRACE").is_ok() {
                eprintln!("zen:   tick -> commit {w}x{h}");
            }
            self.commit_size(w, h);
            let _ = conn.flush(); // flush the size commit NOW (C++ lesson)
        }
        // Stop only once the tween is done AND the target size has actually
        // been committed. If the final commit was skipped (a previous one was
        // still pending), `tick` keeps offering the target and the timer stays
        // alive until the pending clears and it lands — otherwise a morph that
        // ends while a commit is pending strands the surface at the wrong size.
        // size_pending gates this too: while a commit is un-acked the surface
        // can't redraw (maybe_render drops the frame), so the timer MUST keep
        // running — removing it here with pending set was the second half of
        // the stuck-dashboard deadlock (nothing left to un-stick the commit).
        if self.shell.anim.is_none()
            && self.shell.at_target()
            && !self.shell.size_pending
            && !self.shell.dash_sb_pending_frames(now)
        {
            if let Some(tok) = self.morph_timer.take() {
                handle.remove(tok);
            }
            // final size committed; redraw at whatever size the compositor acks
            self.dirty = true;
            self.maybe_render();
            let _ = qh;
        } else if self.shell.anim.is_none() {
            // no size morph left, but the dashboard scrollbar overlay is still
            // within its reveal/fade window — keep painting so the fade-out
            // actually animates instead of stalling on the last event frame.
            self.dirty = true;
            self.maybe_render();
        }
    }

    // ------------------------------------------------------------------
    // Power hold (3 s liquid fill on the power menu)
    // ------------------------------------------------------------------

    /// Drive the power-menu hold fill while a button is held (~30 fps).
    fn ensure_hold_timer(&mut self) {
        if self.hold_timer.is_some() {
            return;
        }
        let Some(handle) = self.loop_handle.clone() else { return };
        let timer = calloop::timer::Timer::from_duration(Duration::from_millis(33));
        let token = handle
            .insert_source(timer, move |_, _, app: &mut App| {
                if app.hold_tick() {
                    calloop::timer::TimeoutAction::ToDuration(Duration::from_millis(33))
                } else {
                    // done — drop the timer
                    if let Some(tok) = app.hold_timer.take() {
                        if let Some(h) = app.loop_handle.clone() {
                            h.remove(tok);
                        }
                    }
                    calloop::timer::TimeoutAction::Drop
                }
            })
            .expect("hold timer");
        self.hold_timer = Some(token);
    }

    /// Run the visualizer while the dashboard is open or the collapsed pill's viz
    /// is enabled. Ticks at 60 fps; renders only while the bars actually move, so a silent card/pill
    /// costs no rendering CPU. Removes the timer when the visualization is
    /// hidden (0 CPU while resting).
    fn sync_viz_timer(&mut self) {
        use crate::shell::Mode;
        let visible = self.shell.mode == Mode::Expanded
            || (self.shell.mode == Mode::Collapsed && self.shell.pill_viz);
        if visible && self.viz_timer.is_none() {
            let Some(handle) = self.loop_handle.clone() else { return };
            let timer = calloop::timer::Timer::from_duration(Duration::from_millis(16));
            let token = handle
                .insert_source(timer, move |_, _, app: &mut App| {
                    app.viz_tick();
                    let still = app.shell.mode == Mode::Expanded
                        || (app.shell.mode == Mode::Collapsed && app.shell.pill_viz);
                    if !still {
                        if let Some(tok) = app.viz_timer.take() {
                            if let Some(h) = app.loop_handle.clone() {
                                h.remove(tok);
                            }
                        }
                        calloop::timer::TimeoutAction::Drop
                    } else {
                        calloop::timer::TimeoutAction::ToDuration(Duration::from_millis(16))
                    }
                })
                .expect("viz timer");
            self.viz_timer = Some(token);
        } else if !visible && self.viz_timer.is_some() {
            if let Some(tok) = self.viz_timer.take() {
                if let Some(handle) = self.loop_handle.as_ref() {
                    let _ = handle.remove(tok);
                }
            }
        }
    }

    /// World-clock maintenance: when the dashboard is open and the pinned
    /// zones' offsets are missing/stale (> 5 min — DST), spawn ONE worker
    /// thread probing every zone with `TZ=<tz> date +%z %Z` and send the
    /// batch to the tz channel. No-op when a probe is already recent.
    fn tick_worldclock_probe(&mut self) {
        if self.shell.mode != crate::shell::Mode::Expanded || self.tz_tx.is_none() {
            return;
        }
        if let Some(at) = self.shell.worldclock_probed_at {
            if at.elapsed() < std::time::Duration::from_secs(5 * 60) {
                return;
            }
        }
        let batch: Vec<(String, String)> = self
            .shell
            .worldclock_zones
            .iter()
            .map(|(_, tz, _, _)| (tz.clone(), String::new()))
            .collect();
        if batch.is_empty() {
            return;
        }
        self.shell.worldclock_probed_at = Some(std::time::Instant::now()); // throttle re-spawns
        let tx = self.tz_tx.clone().unwrap();
        std::thread::spawn(move || {
            let mut results: Vec<(String, i32, String)> = Vec::new();
            for (tz, _) in batch {
                let out = std::process::Command::new("date")
                    .env("TZ", &tz)
                    .arg("+%z %Z")
                    .output();
                if let Ok(o) = out {
                    if o.status.success() {
                        let line = String::from_utf8_lossy(&o.stdout);
                        let mut it = line.split_whitespace();
                        let off = it.next().unwrap_or("");
                        let abbr = it.next().unwrap_or("").to_string();
                        if let Some(min) = crate::shell::worldmap::parse_tz_offset(off) {
                            results.push((tz, min, abbr));
                        }
                    }
                }
            }
            if !results.is_empty() {
                let _ = tx.send(results);
            }
        });
    }

    /// Pomodoro countdown driver: 1 s ticks ONLY while a phase is running
    /// (self-teardown on pause/reset/finish — 0 CPU at rest, mirroring
    /// sync_viz_timer). Each tick advances the phase machine and re-renders
    /// so the mm:ss numerals follow the deadline.
    fn sync_pomo_timer(&mut self) {
        let want = self.shell.pomo_running;
        if want && self.pomo_timer.is_none() {
            let Some(handle) = self.loop_handle.clone() else { return };
            let timer = calloop::timer::Timer::from_duration(Duration::from_secs(1));
            let token = handle
                .insert_source(timer, |_, _, app: &mut App| {
                    let flipped = app.shell.pomo_tick();
                    app.dirty = true;
                    app.maybe_render();
                    if !app.shell.pomo_running {
                        // phase machine finished everything (or paused from
                        // elsewhere) — tear the timer down
                        if let Some(tok) = app.pomo_timer.take() {
                            if let Some(h) = app.loop_handle.clone() {
                                h.remove(tok);
                            }
                        }
                        calloop::timer::TimeoutAction::Drop
                    } else {
                        let _ = flipped;
                        calloop::timer::TimeoutAction::ToDuration(Duration::from_secs(1))
                    }
                })
                .expect("pomo timer");
            self.pomo_timer = Some(token);
        } else if !want && self.pomo_timer.is_some() {
            if let Some(tok) = self.pomo_timer.take() {
                if let Some(handle) = self.loop_handle.as_ref() {
                    let _ = handle.remove(tok);
                }
            }
        }
    }

    /// Eye-rest (20-20-20) countdown driver: a 1 s tick while a focus stretch
    /// or a rest break is live, self-tearing when both are parked (0 CPU at
    /// rest, mirroring sync_pomo_timer). Each tick advances the phase machine
    /// and redraws so the mm:ss ring follows the deadline.
    fn sync_er_timer(&mut self) {
        let want = self.shell.er_running || self.shell.er_rest_until.is_some();
        if want && self.er_timer.is_none() {
            let Some(handle) = self.loop_handle.clone() else { return };
            let timer = calloop::timer::Timer::from_duration(Duration::from_secs(1));
            let token = handle
                .insert_source(timer, |_, _, app: &mut App| {
                    let flipped = app.shell.er_tick();
                    let still = app.shell.er_running || app.shell.er_rest_until.is_some();
                    if flipped || still {
                        app.dirty = true;
                        app.maybe_render();
                    }
                    if !still {
                        if let Some(tok) = app.er_timer.take() {
                            if let Some(h) = app.loop_handle.clone() {
                                h.remove(tok);
                            }
                        }
                        calloop::timer::TimeoutAction::Drop
                    } else {
                        calloop::timer::TimeoutAction::ToDuration(Duration::from_secs(1))
                    }
                })
                .expect("er timer");
            self.er_timer = Some(token);
        } else if !want && self.er_timer.is_some() {
            if let Some(tok) = self.er_timer.take() {
                if let Some(handle) = self.loop_handle.as_ref() {
                    let _ = handle.remove(tok);
                }
            }
        }
    }

    /// One visualizer step — exactly cava's shaping: bars JUMP UP fast toward
    /// the real captured spectrum, then melt back down slowly (per-bar
    /// exponential release). A slow auto-gain tracks the live peak so even
    /// quiet content climbs the full 0..1 range (like cava's autosens), and
    /// the cava "jump up fast, melt back down slow" happens here so the motion
    /// is buttery, never stepped. When the capture is silent or dead, bars fall
    /// to the ground. Redraw skipped when nothing moved (silent card/pill = zero
    /// rendering CPU).
    fn viz_tick(&mut self) {
        // rise toward a louder target fast; release toward silence slowly
        const RISE: f32 = 0.60;
        const FALL: f32 = 0.06;
        let mut live_bars: Vec<f32> = Vec::new();
        let live = self.viz_cap.sample(&mut live_bars)
            && live_bars.len() == self.shell.viz.len();
        // auto-gain: track the peak band with a slow fall so bars always have
        // headroom to the top. Quick rise, gentle melt — never a hard clamp.
        let frame_peak = live.then(|| live_bars.iter().cloned().fold(0.0f32, f32::max)).unwrap_or(0.0);
        self.shell.viz_peak = if frame_peak >= self.shell.viz_peak {
            frame_peak
        } else {
            self.shell.viz_peak * 0.997
        };
        let gain = if self.shell.viz_peak > 1e-4 {
            (1.0 / self.shell.viz_peak).min(6.0).max(0.4)
        } else {
            1.0
        };
        let mut moved = false;
        for i in 0..self.shell.viz.len() {
            let v = self.shell.viz[i];
            let t = if live { (live_bars[i] * gain).min(1.0) } else { 0.0 };
            let nv = if t >= v { v + (t - v) * RISE } else { v + (t - v) * FALL };
            moved |= (nv - v).abs() > 1e-4;
            self.shell.viz[i] = nv;
        }
        if moved {
            self.dirty = true;
            self.maybe_render();
        }
    }

    /// Edit-mode fluidity driver: keeps rendering while cards are still
    /// GLIDING to their packed slots after a move/resize (the ease itself
    /// happens inside the layout pass). Mirrors sync_viz_timer.
    fn sync_edit_timer(&mut self) {
        let want = self.shell.mode == crate::shell::Mode::Expanded
            && self.shell.dash_edit
            && self.shell.edit_settling;
        if want && self.edit_timer.is_none() {
            let Some(handle) = self.loop_handle.clone() else { return };
            let timer = calloop::timer::Timer::from_duration(Duration::from_millis(16));
            let token = handle
                .insert_source(timer, move |_, _, app: &mut App| {
                    app.dirty = true;
                    app.maybe_render();
                    let still = app.shell.mode == crate::shell::Mode::Expanded
                        && app.shell.dash_edit
                        && app.shell.edit_settling;
                    if still {
                        calloop::timer::TimeoutAction::ToDuration(Duration::from_millis(16))
                    } else {
                        if let Some(tok) = app.edit_timer.take() {
                            if let Some(h) = app.loop_handle.clone() {
                                h.remove(tok);
                            }
                        }
                        calloop::timer::TimeoutAction::Drop
                    }
                })
                .expect("edit settle timer");
            self.edit_timer = Some(token);
        } else if !want && self.edit_timer.is_some() {
            if let Some(tok) = self.edit_timer.take() {
                if let Some(handle) = self.loop_handle.as_ref() {
                    let _ = handle.remove(tok);
                }
            }
        }
    }

    // ── dashboard background feeds (news / sensors / camera) ─────────────
    // All three only run while the dashboard is open (Expanded mode) and
    // self-teardown when it closes — mirroring sync_viz_timer.

    /// News: refresh on open, then every 5 minutes while the dashboard stays
    /// open. Fetch runs on a worker thread (curl, 8 s timeout).
    /// Detect MPRIS track changes and fetch the new track's lyrics in the
    /// background. Idempotent per track (`lyrics_track` remembers the one we
    /// asked for); skipped under a live slider drag so curl can't stall the
    /// frame under the pointer. The broker gates the fetch: lyrics are only
    /// pulled while a Lyrics card is on screen.
    fn check_lyrics(&mut self) {
        if !self.broker.active(crate::shell::Res::Lyrics) {
            return;
        }
        if self.pointer_down && self.shell.drag_key.is_some() {
            return;
        }
        let title = self.shell.media_title.trim().to_string();
        let artist = self.shell.media_artist.trim().to_string();
        if title.is_empty() {
            if !self.shell.lyrics_track.is_empty() {
                self.shell.lyrics_track.clear();
                self.shell.lyrics_lines.clear();
                self.shell.lyrics_state = 0;
            }
            return;
        }
        let key = format!("{artist} || {title}");
        if self.shell.lyrics_track == key {
            return;
        }
        self.shell.lyrics_track = key;
        self.shell.lyrics_lines.clear();
        self.shell.lyrics_state = 1;
        let Some(tx) = self.lyrics_tx.clone() else { return };
        std::thread::spawn(move || {
            let lines = crate::lyrics::fetch_lyrics(&artist, &title);
            let _ = tx.send(crate::lyrics::LyricsMsg::Result(artist, title, lines));
        });
    }

    /// Kick a background headline fetch (all configured category feeds).
    fn fetch_news_now(&mut self) {
        let Some(tx) = self.news_tx.clone() else { return };
        let feeds = {
            let cfg = &self.shell.cfg;
            if !cfg.news_feeds.is_empty() {
                cfg.news_feeds.clone()
            } else if let Some(u) = &cfg.news_url {
                vec![crate::news::NewsFeedConfig { name: String::new(), url: u.clone() }]
            } else {
                Vec::new()
            }
        };
        std::thread::spawn(move || {
            let items = crate::news::fetch_news(&feeds);
            let _ = tx.send(crate::news::NewsMsg::Items(items));
        });
    }

    /// The broker's dispatcher: route a due resource to its existing fetcher.
    /// Each arm uses the same in-flight guards the old mode-gated timers did,
    /// so nothing here can pile up duplicate workers.
    fn broker_fetch(&mut self, res: crate::shell::Res) {
        use crate::shell::Res;
        match res {
            Res::Weather => {
                let Some(tx) = self.wx_tx.clone() else { return };
                weather::fetch_once(
                    tx,
                    self.shell.cfg.weather.latitude,
                    self.shell.cfg.weather.longitude,
                );
            }
            Res::News => self.fetch_news_now(),
            Res::Ticker => self.kick_ticker_fetch(),
            Res::Quote => self.kick_quote_fetch(),
            Res::Lyrics => self.check_lyrics(),
            Res::SpeedTest => {} // button-only (pending_speed_test)
        }
    }

    /// Broker sub-dispatcher: price ticker rows. Guarded by the same
    /// `ticker_inflight` flag kick_ticker_if_stale used.
    fn kick_ticker_fetch(&mut self) {
        if self.ticker_inflight {
            return;
        }
        let Some(tx) = self.ticker_tx.clone() else { return };
        self.ticker_inflight = true;
        self.shell.ticker_state = 1;
        std::thread::spawn(move || {
            let rows = crate::shell::Shell::fetch_ticker();
            let _ = tx.send(crate::shell::TickerMsg::Items { rows });
        });
    }

    /// Broker sub-dispatcher: seed a first quote-of-the-day when the Quote
    /// card comes on screen. Never auto-refreshes past that (Manual rate);
    /// the card's refresh button drives later fetches.
    fn kick_quote_fetch(&mut self) {
        if self.shell.quote_state != 0 {
            return;
        }
        let Some(tx) = self.quote_tx.clone() else { return };
        self.shell.quote_state = 1;
        std::thread::spawn(move || {
            let _ = match crate::shell::Shell::fetch_quote() {
                Some((text, author)) => tx.send(crate::shell::QuoteMsg::Quote { text, author }),
                None => tx.send(crate::shell::QuoteMsg::Failed),
            };
        });
    }

    /// Sensors: sample IIO every 1.5 s while the dashboard is open.
    fn sync_sensor_timer(&mut self) {
        let want = self.shell.mode == crate::shell::Mode::Expanded;
        if want && self.sensor_timer.is_none() {
            self.sample_sensors();
            let Some(handle) = self.loop_handle.clone() else { return };
            let timer = calloop::timer::Timer::from_duration(Duration::from_millis(1500));
            let token = handle
                .insert_source(timer, |_, _, app: &mut App| {
                    app.sample_sensors();
                    if app.shell.mode == crate::shell::Mode::Expanded {
                        calloop::timer::TimeoutAction::ToDuration(Duration::from_millis(1500))
                    } else {
                        if let Some(tok) = app.sensor_timer.take() {
                            if let Some(h) = app.loop_handle.clone() {
                                h.remove(tok);
                            }
                        }
                        calloop::timer::TimeoutAction::Drop
                    }
                })
                .expect("sensor timer");
            self.sensor_timer = Some(token);
        } else if !want && self.sensor_timer.is_some() {
            if let Some(tok) = self.sensor_timer.take() {
                if let Some(handle) = self.loop_handle.as_ref() {
                    let _ = handle.remove(tok);
                }
            }
        }
    }

    /// Sample the IIO sensors and redraw when anything moved.
    fn sample_sensors(&mut self) {
        let s = crate::sensors::sample();
        let old = &self.shell.sensors;
        if s.has_accel != old.has_accel
            || (s.pitch - old.pitch).abs() > 0.01
            || (s.roll - old.roll).abs() > 0.01
            || s.heading != old.heading
            || s.gyro != old.gyro
        {
            self.shell.sensors = s;
            self.dirty = true;
            self.maybe_render();
        }
    }

    /// Mirror-camera capture size (16:9 — small enough to pipe + upload
    /// cheaply at full rate while remaining crisp on the card).
    const CAM_W: u32 = 640;
    const CAM_H: u32 = 360;
    /// Raw RGBA bytes per frame — the pipe carries NO JPEG, so the loop just
    /// splits fixed-size frames and uploads the pixels straight to GL.
    const CAM_FRAME_BYTES: usize = (Self::CAM_W * Self::CAM_H * 4) as usize;

    /// Is the Mirror card VISIBLE right now (on the packed board, dashboard
    /// enabled, dashboard open)? The camera hardware must only run while its
    /// consumer is actually rendered — parking/removing the Mirror card (or
    /// toggling the dashboard off) MUST release the camera (LED off).
    fn mirror_card_visible(&self) -> bool {
        self.shell.mode == crate::shell::Mode::Expanded
            && self.shell.dash_enabled
            && self.shell.packed_layout.iter().any(|l| l.card == crate::shell::DashCard::Mirror)
    }

    /// Mirror card: spawn the ffmpeg rawvideo capture while the Mirror card
    /// is VISIBLE (shell.mirror_fps requested, 30 fallback on rejection),
    /// tear it down the moment it isn't — even with the dashboard still open.
    fn sync_cam_timer(&mut self) {
        if std::env::var("ZEN_TRACE").is_ok() {
            eprintln!("zen: sync_cam_timer mode={:?} timer={} child={}", self.shell.mode, self.cam_timer.is_some(), self.cam_child.is_some());
        }
        if self.mirror_card_visible() {
            if self.cam_timer.is_none() {
                self.start_cam();
            }
        } else if self.cam_timer.is_some() || self.cam_child.is_some() {
            self.stop_cam();
        }
    }

    fn start_cam(&mut self) {
        // A device that was busy a moment ago may have just freed up, so
        // respect any active backoff window and kick off the first attempt
        // immediately on a fresh dashboard open.
        if self.cam_child.is_none() {
            let due = self.cam_retry_at.map(|t| t <= Instant::now()).unwrap_or(true);
            if due {
                self.cam_retried = false;
                self.spawn_cam(self.shell.mirror_fps);
            }
        }
        if self.cam_timer.is_some() {
            return;
        }
        // ~camera-rate cadence: fresh frames are uploaded immediately, and
        // when the device only delivers 30 fps half the ticks just `return`.
        // The timer stays alive for the WHOLE time the Mirror card is up — even
        // while the capture is dead/backing off — so a camera that disappeared
        // (hot-unplug, busy, transient ioctl error) is retried until it comes
        // back instead of giving up until the next dashboard open.
        let Some(handle) = self.loop_handle.clone() else { return };
        let timer = calloop::timer::Timer::from_duration(Duration::from_millis(16));
        let token = handle
            .insert_source(timer, |_, _, app: &mut App| {
                app.cam_tick();
                // re-arm only while the Mirror card is actually visible —
                // parking it (or closing the dashboard) drops the timer and
                // releases the camera even though the mode is still Expanded
                if app.mirror_card_visible() {
                    calloop::timer::TimeoutAction::ToDuration(Duration::from_millis(16))
                } else {
                    app.stop_cam();
                    calloop::timer::TimeoutAction::Drop
                }
            })
            .expect("cam timer");
        self.cam_timer = Some(token);
    }

    /// Spawn the ffmpeg V4L2 → rawvideo-RGBA capture at `rate` fps. The pipe
    /// is a stream of fixed-size frames (`CAM_W×CAM_H×4` bytes each) that the
    /// reader thread splits with zero JPEG decode in the loop: the pixels go
    /// straight into the `cam` image key and get uploaded as one GL texture.
    /// Returns false (and schedules a retry) when no capture could start.
    fn spawn_cam(&mut self, rate: u32) -> bool {
        use std::process::{Command, Stdio};
        // configured device wins; else the auto-detected front camera
        let dev = self
            .shell
            .cfg
            .camera_dev
            .clone()
            .map(|d| d.trim().to_string())
            .filter(|d| !d.is_empty())
            .or_else(find_front_camera);
        let (Some(dev), true) = (dev, true) else {
            if std::env::var("ZEN_TRACE").is_ok() {
                eprintln!("zen: cam spawn — no camera device detected");
            }
            self.cam_retry_at = Some(Instant::now() + Duration::from_secs(5));
            return false;
        };
        let size = format!("{}x{}", Self::CAM_W, Self::CAM_H);
        let rate_s = rate.to_string();
        if std::env::var("ZEN_TRACE").is_ok() {
            eprintln!("zen: cam spawn dev={dev} rate={rate}");
        }
        let Ok(mut child) = Command::new("ffmpeg")
            .args([
                "-loglevel", "error",
                "-f", "v4l2",
                "-input_format", "mjpeg",
                "-video_size", &size,
                "-framerate", &rate_s,
                "-i", &dev,
                "-f", "rawvideo",
                "-pix_fmt", "rgba",
                "pipe:1",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
        else {
            if std::env::var("ZEN_TRACE").is_ok() {
                eprintln!("zen: cam spawn — ffmpeg failed to start");
            }
            self.cam_retry_at = Some(Instant::now() + Duration::from_secs(5));
            return false;
        };
        if let Some(pipe) = child.stdout.take() {
            let frame = self.cam_frame.clone();
            let fresh = self.cam_fresh.clone();
            std::thread::spawn(move || split_raw(pipe, frame, fresh, Self::CAM_FRAME_BYTES));
        }
        self.cam_child = Some(child);
        self.cam_rate = rate;
        self.cam_frames = 0;
        self.cam_fps_last = 0;
        self.cam_fps_at = Instant::now();
        self.cam_spawned_at = Instant::now();
        self.cam_retry_at = None;
        self.shell.cam_rate = 0;
        true
    }

    /// Kill the live capture + reader (keeps the timer, so `cam_tick` can
    /// respawn at another rate).
    fn kill_cam_child(&mut self) {
        if let Some(mut c) = self.cam_child.take() {
            let _ = c.kill();
            let _ = c.wait();
        }
        if let Ok(mut f) = self.cam_frame.lock() {
            f.clear();
        }
        self.cam_fresh.store(false, Ordering::Relaxed);
        self.shell.cam_ok = false;
        self.shell.cam_rate = 0;
    }

    /// A capture attempt failed to produce frames (dead child, or alive-but-
    /// silent past warmup). One lower-rate respawn (60 → 30; anything ≤ 30
    /// retries its own rate), then schedule another attempt after a short
    /// backoff — the mirror never gives up while the dashboard is open, so a
    /// busy/unplugged camera recovers on its own.
    fn cam_failed_attempt(&mut self) {
        if !self.cam_retried {
            self.cam_retried = true;
            self.kill_cam_child();
            self.spawn_cam(self.cam_fallback_rate());
        } else {
            self.kill_cam_child();
            self.cam_retry_at = Some(Instant::now() + Duration::from_secs(3));
        }
    }

    /// Secondary capture rate for a failed spawn: the user's target is the
    /// primary; only a 60 fps request falls back to 30 (the driver rejecting
    /// 60 is the case the fallback exists for). Rates ≤ 30 just retry as-is.
    fn cam_fallback_rate(&self) -> u32 {
        if self.shell.mirror_fps > 30 { 30 } else { self.shell.mirror_fps }
    }

    /// Mirror-card fps button: the click already cycled `shell.mirror_fps`.
    /// Restart the capture at the new rate right away and persist the choice
    /// so it survives a bar restart.
    fn cycle_mirror_fps(&mut self) {
        if self.shell.mode == crate::shell::Mode::Expanded {
            // recording is fed from the live capture — restart it too so the
            // requested rate is what actually gets encoded
            self.stop_cam();
            self.cam_retried = false;
            self.cam_retry_at = None;
            self.start_cam();
        }
        self.shell.save_config();
    }

    fn stop_cam(&mut self) {
        // finalize any in-progress recording (the capture is going away)
        self.stop_mirror_rec();
        self.kill_cam_child();
        if let Some(tok) = self.cam_timer.take() {
            if let Some(handle) = self.loop_handle.as_ref() {
                let _ = handle.remove(tok);
            }
        }
        self.shell.cam_ok = false;
        self.shell.cam_rate = 0;
        self.cam_rate = 0;
    }

    /// Push the newest raw camera frame into the image store ('cam' key,
    /// re-uploaded next draw via `img_remove`). Runs the 60 → 30 fps fallback
    /// and the retry-backoff loop so the feed self-heals after transient
    /// failures (device busy, ioctl errors, unplug) while the dashboard stays
    /// open.
    fn cam_tick(&mut self) {
        // no capture running — retry when the backoff window has elapsed
        if self.cam_child.is_none() {
            let due = self.cam_retry_at.map(|t| t <= Instant::now()).unwrap_or(true);
            if !due {
                return;
            }
            if std::env::var("ZEN_TRACE").is_ok() {
                eprintln!("zen: cam retry — attempting respawn");
            }
            self.cam_retried = false;
            self.spawn_cam(self.shell.mirror_fps);
            if self.cam_child.is_none() {
                return;
            }
        }
        // capture died — the driver rejected the requested rate, the device
        // was unplugged, or ffmpeg errored.
        let dead = self
            .cam_child
            .as_mut()
            .map(|c| matches!(c.try_wait(), Ok(Some(_))))
            .unwrap_or(false);
        if dead {
            if std::env::var("ZEN_TRACE").is_ok() {
                eprintln!("zen: cam died frames={}", self.cam_frames);
            }
            self.cam_failed_attempt();
            return;
        }
        // alive but silent past device warmup (rate clamped away / device
        // busy) — same fallback, then retry rather than giving up.
        if self.cam_frames == 0 && self.cam_spawned_at.elapsed() > Duration::from_millis(2500) {
            if std::env::var("ZEN_TRACE").is_ok() {
                eprintln!("zen: cam silent for 2.5s");
            }
            self.cam_failed_attempt();
            return;
        }
        if !self.cam_fresh.swap(false, Ordering::Relaxed) {
            return;
        }
        let px = self.cam_frame.lock().map(|f| f.clone()).unwrap_or_default();
        if px.len() < Self::CAM_FRAME_BYTES {
            return;
        }
        self.cam_frames += 1;
        if std::env::var("ZEN_TRACE").is_ok() && self.cam_frames == 1 {
            eprintln!("zen: cam first frame received");
        }
        // measure delivered fps once per window so the card shows the real
        // rate (requesting 60 doesn't guarantee the device delivers it)
        if self.cam_fps_at.elapsed() >= Duration::from_secs(1) {
            let secs = self.cam_fps_at.elapsed().as_secs_f32().max(0.001);
            let fps = (self.cam_frames as f32 - self.cam_fps_last as f32) / secs;
            self.shell.cam_rate = fps.round().clamp(1.0, 240.0) as u32;
            self.cam_fps_last = self.cam_frames;
            self.cam_fps_at = Instant::now();
        }
        // feed the live frame into the recording encoder when one is running
        if let Some(sin) = self.mirror_rec_stdin.as_mut() {
            if sin.write_all(&px).is_err() {
                self.stop_mirror_rec();
            }
        }
        if let Some(t) = self.mirror_rec_at {
            self.shell.mirror_rec_secs = t.elapsed().as_secs() as u32;
        }
        self.img.put("cam", Self::CAM_W, Self::CAM_H, px);
        if let Some(r) = self.renderer.as_mut() {
            r.img_remove("cam");
        }
        self.shell.cam_ok = true;
        self.dirty = true;
        self.maybe_render();
    }

    /// Mirror-card "Photo": encode the newest raw RGBA frame to a timestamped
    /// PNG in ~/Pictures via a one-shot ffmpeg encoder (off the capture device,
    /// so it can't contend with the live feed for the camera).
    fn mirror_snap_photo(&mut self) {
        let px = self.cam_frame.lock().map(|f| f.clone()).unwrap_or_default();
        if px.len() < Self::CAM_FRAME_BYTES || !self.shell.cam_ok {
            self.show_osd(ICON_CAMERA, "no camera frame — mirror is offline".to_string());
            return;
        }
        let dir = std::env::var("HOME")
            .map(|h| std::path::PathBuf::from(h).join("Pictures"))
            .unwrap_or_else(|_| std::path::PathBuf::from("/tmp"));
        let name = format!("mirror-{}.png", self.shell.date_str("%y%m%d-%H%M%S"));
        let path = dir.join(&name);
        let _ = std::fs::create_dir_all(&dir);
        let frame = self.cam_frame.clone();
        std::thread::spawn(move || {
            use std::io::Write;
            use std::process::{Command, Stdio};
            let Ok(mut child) = Command::new("ffmpeg")
                .args([
                    "-loglevel", "error",
                    "-f", "rawvideo",
                    "-pix_fmt", "rgba",
                    "-s", "640x360",
                    "-r", "1",
                    "-i", "pipe:0",
                    "-frames:v", "1",
                    "-y", "-an", "-sn", "-dn",
                ])
                .arg(&path)
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
            else {
                return;
            };
            if let Some(mut sin) = child.stdin.take() {
                if let Ok(f) = frame.lock() {
                    let _ = sin.write_all(&f);
                }
                let _ = sin.flush();
                drop(sin);
                let _ = child.wait();
            }
        });
        self.show_osd(ICON_CAMERA, format!("Photo → ~/Pictures/{name}"));
    }

    /// Mirror-card record button: start or stop the video recorder.
    fn toggle_mirror_rec(&mut self) {
        if self.mirror_rec.is_some() {
            self.stop_mirror_rec();
        } else {
            self.start_mirror_rec();
        }
    }

    /// Start recording the live frame stream (raw forced RGBA frames piped
    /// into an ffmpeg SVT-AV1 encoder → AV1 MP4 in ~/Videos; this builds
    /// ffmpeg lacks libx264/libopenh264, and svt-av1 encodes 640×360 in real
    /// time at preset 8).
    fn start_mirror_rec(&mut self) {
        if self.mirror_rec.is_some() {
            return;
        }
        if !self.shell.cam_ok || self.cam_child.is_none() {
            self.show_osd(ICON_CAMERA, "no camera feed — cannot record".to_string());
            return;
        }
        let dir = std::env::var("HOME")
            .map(|h| std::path::PathBuf::from(h).join("Videos"))
            .unwrap_or_else(|_| std::path::PathBuf::from("/tmp"));
        let name = format!("mirror-{}.mp4", self.shell.date_str("%y%m%d-%H%M%S"));
        let path = dir.join(&name);
        let _ = std::fs::create_dir_all(&dir);
        let Ok(mut child) = std::process::Command::new("ffmpeg")
            .args([
                "-loglevel", "error",
                "-y",
                "-f", "rawvideo",
                "-pix_fmt", "rgba",
                "-s", "640x360",
                "-r", "30",
                "-i", "pipe:0",
                "-c:v", "libsvtav1",
                "-preset", "8",
                "-crf", "40",
                "-pix_fmt", "yuv420p",
            ])
            .arg(&path)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
        else {
            self.show_osd(ICON_CAMERA, "ffmpeg failed to start recording".to_string());
            return;
        };
        self.mirror_rec_stdin = child.stdin.take();
        self.mirror_rec = Some(child);
        self.mirror_rec_path = Some(path);
        self.mirror_rec_at = Some(Instant::now());
        self.shell.mirror_rec = true;
        self.shell.mirror_rec_secs = 0;
        self.show_osd(ICON_CAMERA, format!("Recording → ~/Videos/{name}"));
    }

    /// Stop the recorder: closing stdin lets ffmpeg write the trailer, with a
    /// short grace wait before a hard kill.
    fn stop_mirror_rec(&mut self) {
        self.mirror_rec_stdin.take();
        let child = self.mirror_rec.take();
        let path = self.mirror_rec_path.take();
        self.mirror_rec_at = None;
        self.shell.mirror_rec = false;
        self.shell.mirror_rec_secs = 0;
        let mut child = if let Some(c) = child {
            c
        } else {
            return;
        };
        let deadline = Instant::now() + Duration::from_millis(1500);
        loop {
            if let Ok(Some(_)) = child.try_wait() {
                break;
            }
            if Instant::now() >= deadline {
                let _ = child.kill();
                let _ = child.wait();
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        if let Some(p) = path {
            let fname = p
                .file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default();
            self.show_osd(ICON_CAMERA, format!("Recording saved → ~/Videos/{fname}"));
        }
    }

    /// Audio recorder card: start/stop toggle.
    fn toggle_audiorec_rec(&mut self) {
        if self.audiorec_rec.is_some() {
            self.stop_audiorec_rec();
        } else {
            self.start_audiorec_rec();
        }
    }

    /// Start recording system audio into ~/Recordings/Audio via pw-record
    /// (the default audio sink; same tool family as the mic meter card).
    fn start_audiorec_rec(&mut self) {
        if self.audiorec_rec.is_some() {
            return;
        }
        use std::process::{Command, Stdio};
        let dir = self.audiorec_dir.clone();
        let _ = std::fs::create_dir_all(&dir);
        // timestamped name so existing recordings can be listed & told apart
        let name = format!("rec-{}.wav", self.shell.date_str("%y%m%d-%H%M%S"));
        let path = dir.join(&name);
        let Ok(child) = Command::new("pw-record")
            .arg(&path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .stdin(Stdio::null())
            .spawn()
        else {
            self.show_osd(ICON_MIC, "pw-record failed to start".to_string());
            return;
        };
        self.audiorec_rec = Some(child);
        self.audiorec_rec_path = Some(path);
        self.audiorec_rec_at = Some(Instant::now());
        self.shell.audiorec_recording = true;
        self.shell.audiorec_rec_secs = 0;
        self.sync_audiorec_timer();
        self.show_osd(ICON_MIC, format!("Recording → ~/Recordings/Audio/{name}"));
    }

    /// Stop recording: SIGINT lets pw-record finalize the WAV header, with a
    /// short grace wait before a hard kill.
    fn stop_audiorec_rec(&mut self) {
        let child = self.audiorec_rec.take();
        let path = self.audiorec_rec_path.take();
        self.audiorec_rec_at = None;
        self.shell.audiorec_recording = false;
        self.shell.audiorec_rec_secs = 0;
        let mut child = if let Some(c) = child {
            c
        } else {
            return;
        };
        let pid = child.id() as i32;
        if pid > 0 {
            unsafe {
                libc::kill(pid, libc::SIGINT);
            }
        }
        let deadline = Instant::now() + Duration::from_millis(1200);
        loop {
            if let Ok(Some(_)) = child.try_wait() {
                break;
            }
            if Instant::now() >= deadline {
                let _ = child.kill();
                let _ = child.wait();
                break;
            }
            std::thread::sleep(Duration::from_millis(40));
        }
        if let Some(p) = path {
            let fname = p
                .file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default();
            self.show_osd(ICON_MIC, format!("Recording saved → ~/Recordings/Audio/{fname}"));
            self.refresh_audiorec_list();
        }
    }

    /// Rescan the recordings dir; each entry is (path, short stem) with the
    /// newest first so the newest recording sits at the top of the list.
    fn refresh_audiorec_list(&mut self) {
        let mut out: Vec<(String, String)> = Vec::new();
        if let Ok(rd) = std::fs::read_dir(&self.audiorec_dir) {
            for e in rd.flatten() {
                let p = e.path();
                if p.extension().map(|x| x == "wav").unwrap_or(false) {
                    let path = p.to_string_lossy().into_owned();
                    let stem = p
                        .file_stem()
                        .map(|s| s.to_string_lossy().into_owned())
                        .unwrap_or_default();
                    out.push((path, stem));
                }
            }
        }
        out.sort_by(|a, b| b.1.cmp(&a.1));
        self.shell.audiorec_recordings = out;
        self.shell.audiorec_del_armed = None;
    }

    /// Play a saved recording through the default sink with pw-play.
    fn play_audiorec_file(&mut self, path: &str) {
        use std::process::{Command, Stdio};
        let Ok(_c) = Command::new("pw-play")
            .arg(path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .stdin(Stdio::null())
            .spawn()
        else {
            self.show_osd(ICON_PLAY, "pw-play failed to start".to_string());
            return;
        };
        std::thread::spawn({
            let c = _c;
            move || {
                let _ = c.wait_with_output();
            }
        });
        if let Some((name, _)) = self
            .shell
            .audiorec_recordings
            .iter()
            .find(|(_, pth)| **pth == *path)
        {
            self.show_osd(ICON_PLAY, format!("Playing {name}"));
        }
    }

    /// 1 s self-tearing tick while a recording is live: keep the on-card
    /// elapsed counter fresh. Idle → drop the timer.
    fn sync_audiorec_timer(&mut self) {
        let want = self.audiorec_rec.is_some();
        if want && self.audiorec_timer.is_none() {
            let Some(handle) = self.loop_handle.clone() else { return };
            let timer = calloop::timer::Timer::from_duration(Duration::from_secs(1));
            let token = handle
                .insert_source(timer, |_, _, app: &mut App| {
                    let still = app.audiorec_rec.is_some();
                    if still {
                        app.shell.audiorec_rec_secs = app
                            .audiorec_rec_at
                            .map(|t| t.elapsed().as_secs() as u32)
                            .unwrap_or(0);
                        app.dirty = true;
                        app.maybe_render();
                    }
                    if still {
                        calloop::timer::TimeoutAction::ToDuration(Duration::from_secs(1))
                    } else {
                        if let Some(tok) = app.audiorec_timer.take() {
                            if let Some(h) = app.loop_handle.clone() {
                                h.remove(tok);
                            }
                        }
                        calloop::timer::TimeoutAction::Drop
                    }
                })
                .expect("audiorec timer");
            self.audiorec_timer = Some(token);
        } else if !want && self.audiorec_timer.is_some() {
            if let Some(tok) = self.audiorec_timer.take() {
                if let Some(handle) = self.loop_handle.as_ref() {
                    let _ = handle.remove(tok);
                }
            }
        }
    }

    /// Edit-mode auto-arrange: every ~3 s scan the grid for empty columns and
    /// shift cards LEFT into them (live — no need to leave edit mode). Runs
    /// only while the dashboard is in edit mode and no drag/resize owns the
    /// pointer; cards glide to their new slots via the edit easing pass.
    fn sync_autoarrange_timer(&mut self) {
        let want = self.shell.mode == crate::shell::Mode::Expanded
            && self.shell.dash_edit
            && self.shell.edit_drag.is_none()
            && self.shell.edit_resize.is_none();
        if want && self.autoarrange_timer.is_none() {
            let Some(handle) = self.loop_handle.clone() else { return };
            let timer = calloop::timer::Timer::from_duration(Duration::from_secs(3));
            let token = handle
                .insert_source(timer, |_, _, app: &mut App| {
                    let changed = app.shell.autoarrange();
                    if changed {
                        app.dirty = true;
                        app.maybe_render();
                    }
                    let still = app.shell.mode == crate::shell::Mode::Expanded
                        && app.shell.dash_edit
                        && app.shell.edit_drag.is_none()
                        && app.shell.edit_resize.is_none();
                    if still {
                        calloop::timer::TimeoutAction::ToDuration(Duration::from_secs(3))
                    } else {
                        if let Some(tok) = app.autoarrange_timer.take() {
                            if let Some(h) = app.loop_handle.clone() {
                                h.remove(tok);
                            }
                        }
                        calloop::timer::TimeoutAction::Drop
                    }
                })
                .expect("auto-arrange timer");
            self.autoarrange_timer = Some(token);
        } else if !want && self.autoarrange_timer.is_some() {
            if let Some(tok) = self.autoarrange_timer.take() {
                if let Some(handle) = self.loop_handle.as_ref() {
                    let _ = handle.remove(tok);
                }
            }
        }
    }

    /// Every-minute low-memory janitor. The shell spends most of its life as
    /// a resting pill, and morph churn (scene Vecs, text shaping, image
    /// uploads) frees thousands of small allocations that glibc then HOARDS
    /// in its arenas — RSS only ever ratchets up. `malloc_trim` hands every
    /// free arena page back to the kernel, so long sessions stay at the
    /// memory floor instead of drifting upward. 0 CPU cost: one call a
    /// minute on the existing power timer.
    #[cfg(target_env = "musl")]
    fn memory_janitor(&mut self) {
        // musl returns freed memory eagerly; nothing to trim
    }

    #[cfg(not(target_env = "musl"))]
    fn memory_janitor(&mut self) {
        unsafe {
            libc::malloc_trim(0);
        }
    }

    /// Advance the power hold. Returns true while it should keep ticking:
    /// fires the action when the hold-delay fill completes, stops when the
    /// hold was cancelled (release / moved off / mode changed).
    fn hold_tick(&mut self) -> bool {
        use crate::shell::Mode;
        // dashboard Clear-all hold: the ~30 fps timer just keeps the button's
        // fill drawn while pressed. Clearing itself is release-driven and
        // INSTANT (click = clear), so the ticker never mid-fires.
        let mut keep_clear = false;
        if self.shell.dash_clear_all_hold.is_some() || self.shell.banner_clear_all_hold.is_some() {
            self.dirty = true;
            self.maybe_render();
            keep_clear = true;
        }

        let Some(key) = self.shell.power_hold else { return keep_clear };
        // holds exist on the power menu, the lockscreen, and the dashboard
        // power cards (Mode::Expanded, keys in the POWER_CARD_KEY band)
        if !matches!(self.shell.mode, Mode::Power | Mode::Lock | Mode::Expanded) {
            self.shell.power_hold = None;
            return false;
        }
        if self.shell.power_hold_fill() >= 1.0 {
            let action = self.shell.held_power_action(key);
            self.shell.power_hold = None;
            if !action.is_empty() {
                self.shell.run_power(action);
            }
            // the lock's restart/shutdown collapse back to the lock, the
            // power menu's actions go back to the pill
            self.apply_mode(if self.shell.mode == Mode::Lock {
                Mode::Lock
            } else {
                Mode::Collapsed
            });
            return false;
        }
        // still filling — redraw the rising liquid
        self.dirty = true;
        self.maybe_render();
        true
    }

    fn on_tray_msg(&mut self, ev: calloop::channel::Event<TrayMsg>) {
        match ev {
            calloop::channel::Event::Msg(m) => match m {
                TrayMsg::Add(item) => {
                    if let Some((w, h, px)) = item.pixmap.clone() {
                        self.img.put(&format!("tray:{}", item.id), w, h, px);
                    }
                    if std::env::var("ZEN_TRACE").is_ok() {
                        eprintln!(
                            "zen: tray '{}' status={} key={} tip='{}' menu={}",
                            item.title, item.status, item.image_key(), item.tooltip, item.menu_path
                        );
                    }
                    self.shell.push_tray(item);
                    self.shell.refresh_size();
                    // refresh_size starts a morph — make sure the timer is armed
                    // (tray changes can arrive while the bar is resting).
                    self.ensure_morph_timer();
                    self.dirty = true;
                    self.maybe_render();
                }
                TrayMsg::Remove(id) => {
                    self.shell.remove_tray(&id);
                    self.img.put(&format!("tray:{id}"), 0, 0, Vec::new());
                    self.shell.refresh_size();
                    self.ensure_morph_timer();
                    self.dirty = true;
                    self.maybe_render();
                }
                TrayMsg::Ready => {
                    if std::env::var("ZEN_TRACE").is_ok() {
                        eprintln!("zen: tray daemon ready");
                    }
                }
            },
            calloop::channel::Event::Closed => {
                eprintln!("zen: tray channel closed unexpectedly");
            }
        }
    }

    /// Handle polkit auth messages from the daemon thread.
    fn on_polkit_msg(&mut self, msg: PolkitMsg) {
        match msg {
            PolkitMsg::BeginAuth { action_id, message, icon_name, cookie, user_id } => {
                if std::env::var("ZEN_TRACE").is_ok() {
                    eprintln!("zen: polkit BeginAuth action={action_id} msg={message}");
                }
                self.shell.polkit_action_id = action_id;
                self.shell.polkit_message = message;
                self.shell.polkit_icon = icon_name;
                self.shell.polkit_cookie = cookie;
                self.shell.polkit_uid = user_id;
                self.shell.polkit_pw.clear();
                self.shell.polkit_error = None;
                self.shell.polkit_checking = false;
                self.shell.set_mode(crate::shell::Mode::PolkitAuth);
                self.dirty = true;
                self.maybe_render();
            }
            PolkitMsg::CancelAuth => {
                if std::env::var("ZEN_TRACE").is_ok() {
                    eprintln!("zen: polkit CancelAuth");
                }
                self.shell.polkit_pw.clear();
                self.shell.polkit_error = None;
                self.shell.polkit_checking = false;
                // Return to previous mode
                self.shell.set_mode(crate::shell::Mode::Collapsed);
                self.dirty = true;
                self.maybe_render();
            }
        }
    }

    #[allow(dead_code)] // notification UI kept; fed by no daemon (mako owns D-Bus)
    fn on_notif(&mut self, n: crate::shell::Notif) {
        if std::env::var("ZEN_TRACE").is_ok() {
            eprintln!(
                "zen: notif '{}' '{}' dnd={} mode={:?}",
                n.app, n.summary, self.shell.dnd, self.shell.mode
            );
        }
        // per-app block rule
        if self.shell.cfg.notif_blocked(&n.app) {
            return;
        }
        // DND: acknowledge (id returned to sender) but surface nothing
        if self.shell.dnd {
            return;
        }
        self.shell.push_notif(n.clone());
        self.dirty = true;
        // notifications pop up in their own corner surface (not the OSD,
        // not the bar): stacked cards, auto-dismiss after configured timeout
        self.notif_popup.insert(0, n);
        let max = self.shell.cfg.notifications.max_visible.max(1);
        self.notif_popup.truncate(max);
        self.show_notif_popup();
        self.arm_notif_timer();
        self.maybe_render();
    }

    #[allow(dead_code)] // notification UI kept; fed by no daemon (mako owns D-Bus)
    fn invoke_notif_action(&self, nid: u32, action_id: &str) {
        // Best-effort: send ActionInvoked signal on the session bus.
        // The actual invocation (e.g. opening a URL) happens in the sender's
        // process via the notification spec's action mechanism.
        // We just emit the signal so the sender knows the user clicked.
        // For now, log it — the D-Bus ActionInvoked signal emission requires
        // access to the zbus connection which lives on the daemon thread.
        if std::env::var("ZEN_TRACE").is_ok() {
            eprintln!("zen: notif action nid={nid} action='{action_id}'");
        }
    }

    // ------------------------------------------------------------------
    // Render driver
    // ------------------------------------------------------------------

    /// If we're at rest (Collapsed, no active morph / pending commit) and the
    /// pill's desired width differs from what's committed, kick off a morph so
    /// the surface grows/shrinks to match live item widths (e.g. the long
    // workspace item's appended active digit, or a 1→2 digit transition).
    fn reconcile_pill_size(&mut self) {
        if self.shell.mode != crate::shell::Mode::Collapsed {
            return;
        }
        if self.shell.anim.is_some() || self.shell.size_pending {
            return;
        }
        let (tw, th) = self.shell.target_size();
        if (self.shell.cur_w - tw).abs() >= 0.5 || (self.shell.cur_h - th).abs() >= 0.5 {
            if std::env::var("ZEN_TRACE").is_ok() {
                eprintln!("zen: pill size changed -> morph {:.0}x{:.0}", tw, th);
            }
            self.shell.start_morph(tw, th);
            self.ensure_morph_timer();
        }
    }

    fn maybe_render(&mut self) {
        if std::env::var("ZEN_TRACE").is_ok() {
            eprintln!(
                "zen: maybe_render dirty={} pending={} configured={:?} mode={:?}",
                self.dirty, self.shell.size_pending, self.configured, self.shell.mode
            );
        }
        if !self.dirty {
            return;
        }
        // resting pill: the target width follows its live items (active
        // workspace digits etc.). If it drifted from what's committed, morph
        // the surface to the new size before drawing — e.g. the workspace-long
        // item grows/shrinks when the active workspace leaves/enters the 1..5
        // row, or a 1-digit number becomes 2 digits.
        self.reconcile_pill_size();
        if self.shell.anim.is_some() {
            return;
        }
        // During an edit drag, render every frame even while a resize is
        // pending — the card must follow the cursor at full framerate.
        // The scene is built at the last configured (possibly smaller) size
        // so the card may be clipped, but it still tracks the pointer
        // without the 1-frame Wayland round-trip stall.
        let edit_drag = self.shell.dash_edit && self.shell.edit_drag.is_some();
        if self.shell.size_pending && !edit_drag {
            return;
        }
        // session-locked: the bar is fully covered by the opaque lock surface,
        // so drawing it is pure waste — the lock surface renders instead
        if self.shell.mode == crate::shell::Mode::Lock && self.session_lock_active.is_some() {
            self.dirty = false;
            self.render_lock();
            return;
        }
        let (cw, ch) = self.configured;
        if cw <= 0 || ch <= 0 {
            return; // no configure yet
        }
        let frame_t0 = Instant::now();
        // take the renderer OUT of `self` so the scene builder (self.shell) and
        // the draw loop (self.text / self.img) can borrow `self` freely — the
        // aux surfaces (wallpaper / notif popup) share this pattern
        let mut renderer = self.renderer.take();
        let scene = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| self.shell.layout(cw as f32, ch as f32))) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("zen: layout panicked ({}x{}) — frame skipped, shell kept alive", cw, ch);
                if let Some(m) = e.downcast_ref::<&str>() {
                    eprintln!("zen: layout panic message: {m}");
                } else if let Some(m) = e.downcast_ref::<String>() {
                    eprintln!("zen: layout panic message: {m}");
                }
                self.dirty = false;
                self.renderer = renderer;
                return;
            }
        };
        self.frame_stats.solids = 0;
        self.frame_stats.textured = 0;
        self.frame_stats.uploads = 0;
        if std::env::var("ZEN_TRACE").is_ok() {
            eprintln!("zen: scene {} cmds", scene.len());
        }
        // The draw block must be allowed to throw its GL as-is if a texture
        // upload fails — but a PANIC here must not lose the renderer: it is
        // re-owned first, then the panic is contained + logged so the shell
        // survives and the next frame renders normally.
        let drew = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            match renderer.as_mut() {
                Some(r) => {
                    r.make_current(); // another surface's Gl may be current right now
                    r.begin_frame();
                    self.draw_cmds(r, &scene);
                    r.flush();
                }
                None => {}
            }
        }));
        self.renderer = renderer;
        if drew.is_err() {
            eprintln!("zen: draw panicked — frame skipped, shell kept alive");
            self.dirty = false;
            return;
        }
        self.frame_stats.frames += 1;
        if std::env::var("ZEN_TRACE").is_ok() {
            // rolling FPS report — frames divided by wall time since last frame
            let st = &mut self.frame_stats;
            let dt = frame_t0.duration_since(st.last).as_secs_f32().max(0.001);
            st.fps = st.fps * 0.9 + (1.0 / dt) * (1.0 - 0.9);
            st.last = frame_t0;
            eprintln!(
                "zen: frame fps={:.1} solids={} textured={} uploads={} ({}cmds @{}x{})",
                st.fps, st.solids, st.textured, st.uploads, scene.len(), cw, ch
            );
        }
        self.dirty = false;
    }

    /// Dispatch a `Cmd` scene onto `r`, rasterizing text / images on demand
    /// through the shared text engine + image store.
    fn draw_cmds(&mut self, renderer: &mut Renderer, scene: &[Cmd]) {
        for cmd in scene {
            match cmd {
                Cmd::Rect { x, y, w, h, r, color } => { self.frame_stats.solids += 1; renderer.rect(*x, *y, *w, *h, *r, *color) }
                Cmd::RectConcave { x, y, w, h, r_tl, r_tr, r_br, r_bl, color } => {
                    self.frame_stats.solids += 1;
                    renderer.rect_concave(*x, *y, *w, *h, *r_tl, *r_tr, *r_br, *r_bl, *color)
                }
                Cmd::Outline { x, y, w, h, r, width, color } => {
                    self.frame_stats.solids += 1;
                    renderer.outline(*x, *y, *w, *h, *r, *width, *color)
                }
                Cmd::Line { x0, y0, x1, y1, w, color } => { self.frame_stats.solids += 1; renderer.line(*x0, *y0, *x1, *y1, *w, *color) }
                Cmd::Text { x, y, right, center, text, size, color, icon, f, w } => {
                    let Some(te) = self.text.as_mut() else { continue };
                    let key = (
                        text.clone(),
                        (*size * 10.0) as u32,
                        *color,
                        *icon,
                        f.as_u8(),
                        w.as_u8(),
                    );
                    if !renderer.cache_contains(&key) {
                        let (tw, th, px, _ink_top) = te.rasterize(text, *size, *f, *w, *color, *icon);
                        self.frame_stats.uploads += 1;
                        match renderer.upload_texture(tw, th, &px) {
                            Ok(t) => {
                                renderer.cache_insert(key.clone(), t);
                            }
                            Err(e) => {
                                eprintln!("zen: text upload failed: {e}");
                                continue;
                            }
                        }
                    }
                    if let Some(t) = renderer.cache_get(&key) {
                        // horizontal: right-anchored at the right edge, centered on x, else left
                        let dx = if *right {
                            *x - t.w as f32
                        } else if *center {
                            *x - t.w as f32 * 0.5
                        } else {
                            *x
                        };
                        // vertical: center the glyph ink in a `size`-tall box at `y`.
                        // the cropped texture is ink + 1px margin top/bottom.
                        let ink_h = (t.h as f32 - 2.0).max(1.0);
                        let dy = (*y + (*size - ink_h) * 0.5 - 1.0).round();
                        renderer.text_quad(&t, dx.round(), dy, t.w as f32, t.h as f32);
                        self.frame_stats.textured += 1;
                        if std::env::var("ZEN_TRACE").is_ok() {
                            eprintln!("zen:   text '{}' sz={} tex={}x{}", text, *size, t.w, t.h);
                        }
                    }
                }
                Cmd::Image { x, y, w, h, key } => {
                    if !renderer.img_contains(key) {
                        if let Some((iw, ih, px)) = self.img.load(key, w.max(*h) as u32) {
                            self.frame_stats.uploads += 1;
                            match renderer.upload_texture(iw, ih, &px) {
                                Ok(t) => renderer.img_insert(key.clone(), t),
                                Err(e) => eprintln!("zen: image upload failed ({key}): {e}"),
                            }
                        }
                    }
                    if let Some(t) = renderer.img_get(key) {
                        // fit the texture inside the w×h box, downscale-only
                        // (never upscale, keeps crisp icons), centered — the
                        // cam feed spans the card width this way.
                        let s = (*w / t.w as f32).min(*h / t.h as f32).min(1.0);
                        let (sw, sh) = ((t.w as f32 * s).round().max(1.0), (t.h as f32 * s).round().max(1.0));
                        let dx = (x + (*w - sw) * 0.5).round();
                        let dy = (y + (*h - sh) * 0.5).round();
                        renderer.text_quad(&t, dx, dy, sw, sh);
                        self.frame_stats.textured += 1;
                        if std::env::var("ZEN_TRACE").is_ok() {
                            eprintln!("zen:   img '{}' {}x{}", key, t.w, t.h);
                        }
                    }
                }
                Cmd::Scissor { x, y, w, h } => renderer.scissor(*x, *y, *w, *h),
                Cmd::ScissorEnd => renderer.scissor_off(),
                Cmd::Map { x, y, w, h, lon0, lat0, lon1, lat1, zoom, style, colors, region, version } => {
                    if *w <= 0.0 || *h <= 0.0 {
                        continue;
                    }
                    let bin = (zoom.ceil().max(1.0) as u32).max(1).next_power_of_two();
                    let key = (*style, bin, *colors, *region, *version);
                    let view = (*lon0, *lat0, *lon1, *lat1);
                    let span_v_lon = (view.2 - view.0).abs().max(1e-4);
                    let span_v_lat = (view.3 - view.1).abs().max(1e-4);
                    let band = band_window(view, span_v_lon, span_v_lat);
                    let stale = match &self.world_band {
                        Some(b) => b.key != key || !inside_band(view, b.window, span_v_lon * 0.02, span_v_lat * 0.02),
                        None => true,
                    };
                    if stale {
                        // never stretch an outdated band — bake the new one
                        // crisp right away (band res is large enough that
                        // in-band sampling is always ≥ 1 texel/screen px)
                        self.bake_world_band(renderer, key, band, *w, *h, *style, *colors, *region);
                    }
                    if let Some(b) = &self.world_band {
                        let b_span_lon = (b.win.2 - b.win.0).max(1e-4);
                        let b_span_lat = (b.win.3 - b.win.1).max(1e-4);
                        let u0 = ((view.0 - b.win.0) / b_span_lon).clamp(0.0, 1.0);
                        let u1 = ((view.2 - b.win.0) / b_span_lon).clamp(0.0, 1.0);
                        let v0 = ((b.win.3 - view.3) / b_span_lat).clamp(0.0, 1.0);
                        let v1 = ((b.win.3 - view.1) / b_span_lat).clamp(0.0, 1.0);
                        renderer.text_quad_uv(&b.tex, *x, *y, *w, *h, u0, v0, u1, v1);
                        self.frame_stats.textured += 1;
                    }
                }
            }
        }
    }

    /// Rasterize + upload a fresh world-band texture. Resolution = ~4× the
    /// on-screen body (cap 4096) so the worst-case in-band zoom (view shrinks
    /// to half the band before the next power-of-two crossing re-bakes) lands
    /// exactly at 1 texel/screen-pixel — the texture is never upscaled, so
    /// pan/zoom never stretches or blurs.
    fn bake_world_band(&mut self, renderer: &mut Renderer, key: (u8, u32, [u32; 6], bool, u64), band: (f32, f32, f32, f32), bw_body: f32, bh_body: f32, style: u8, colors: [u32; 6], region: bool) {
        let res_w = ((bw_body * 4.0).round().clamp(64.0, 4096.0)) as u32;
        let res_h = (((res_w as f32) * (bh_body / bw_body.max(1.0))).round().clamp(64.0, 4096.0)) as u32;
        let Some(w) = crate::shell::worldmap::world() else { return };
        let win = crate::shell::worldmap::Win { lon0: band.0, lat0: band.1, lon1: band.2, lat1: band.3 };
        let rings = if region { self.shell.world_region_rings.as_deref() } else { None };
        let px = crate::shell::worldmap::raster_band(res_w as usize, res_h as usize, &win, &w, rings, region, style, colors);
        match renderer.upload_texture_linear(res_w, res_h, &px) {
            Ok(tex) => {
                self.frame_stats.uploads += 1;
                self.world_band = Some(WorldBand { tex, key, win: (band.0, band.1, band.2, band.3), window: band });
                if std::env::var("ZEN_TRACE").is_ok() {
                    eprintln!("zen: world band baked {}×{} (zoom band {})", res_w, res_h, key.1);
                }
            }
            Err(e) => eprintln!("zen: world band upload failed: {e}"),
        }
    }

}

/// Auto-detect the front camera: the first `/dev/videoN` that `ffprobe`
/// reports as a V4L2 capture device with MJPEG support (the built-in cam is
/// typically the lowest-numbered device). Probed once per process; falls back
/// to the first device node that exists when ffprobe isn't available. None
/// when no camera is attached.
fn find_front_camera() -> Option<String> {
    use std::sync::OnceLock;
    static CAM: OnceLock<Option<String>> = OnceLock::new();
    CAM.get_or_init(|| {
        for i in 0..8 {
            let dev = format!("/dev/video{i}");
            if !std::path::Path::new(&dev).exists() {
                break;
            }
            if probe_v4l_capture(&dev) {
                return Some(dev);
            }
        }
        ["/dev/video0", "/dev/video1", "/dev/video2"]
            .into_iter()
            .find(|d| std::path::Path::new(d).exists())
            .map(str::to_string)
    })
    .clone()
}

/// Does `ffprobe -list_formats` report MJPEG support for `dev`? (`v4l2-ctl`
/// isn't installed on this box, so ffprobe doubles as the format enumerator.)
fn probe_v4l_capture(dev: &str) -> bool {
    use std::process::Command;
    let Ok(out) = Command::new("ffprobe")
        .args(["-hide_banner", "-f", "v4l2", "-list_formats", "all", "-i", dev])
        .output()
    else {
        return false;
    };
    let mut s = String::from_utf8_lossy(&out.stderr).into_owned();
    s.push_str(&String::from_utf8_lossy(&out.stdout));
    s.to_ascii_lowercase().contains("mjpeg")
}

/// ffmpeg rawvideo reader thread: the capture pipe is a stream of fixed-size
/// RGBA frames (`frame_bytes` each — no JPEG to split on). Keeps only the
/// newest complete frame and marks `fresh` so the loop uploads at its own
/// cadence without ever blocking on this thread.
fn split_raw(
    mut r: impl std::io::Read,
    frame: Arc<std::sync::Mutex<Vec<u8>>>,
    fresh: Arc<AtomicBool>,
    frame_bytes: usize,
) {
    let mut tmp: Vec<u8> = Vec::with_capacity(frame_bytes);
    let mut buf = [0u8; 65536];
    loop {
        match r.read(&mut buf) {
            Ok(0) | Err(_) => break,
            Ok(n) => {
                tmp.extend_from_slice(&buf[..n]);
                // rawvideo bytes arrive frame-aligned from the start, so
                // draining exact `frame_bytes` keeps `tmp` aligned; store every
                // complete frame so the last one written is the newest.
                while tmp.len() >= frame_bytes {
                    if let Ok(mut f) = frame.lock() {
                        f.clear();
                        f.extend_from_slice(&tmp[..frame_bytes]);
                        fresh.store(true, Ordering::Relaxed);
                    }
                    tmp.drain(..frame_bytes);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_raw_keeps_the_newest_complete_frame() {
        // three frames in one contiguous stream; the read chunks never align
        // with the frame boundary (frames are 100 bytes, chunks are 64 KiB)
        let frame_bytes = 100usize;
        let stream: Vec<u8> = [0u8; 100]
            .iter()
            .chain(std::iter::repeat(&1u8).take(100))
            .chain(std::iter::repeat(&2u8).take(100))
            .copied()
            .collect();
        let frame = Arc::new(std::sync::Mutex::new(Vec::new()));
        let fresh = Arc::new(AtomicBool::new(false));
        split_raw(&stream[..], frame.clone(), fresh.clone(), frame_bytes);
        assert!(fresh.load(Ordering::Relaxed), "a frame must have landed");
        let got = frame.lock().unwrap();
        assert_eq!(got.len(), frame_bytes);
        assert_eq!(&got[..], &[2u8; 100], "only the newest frame survives");
    }

    #[test]
    fn split_raw_drops_the_partial_tail() {
        let frame_bytes = 100usize;
        let mut stream = vec![0x55u8; frame_bytes];
        stream.extend_from_slice(&[0x66u8; 37]); // partial next frame
        let frame = Arc::new(std::sync::Mutex::new(Vec::new()));
        let fresh = Arc::new(AtomicBool::new(false));
        split_raw(&stream[..], frame.clone(), fresh.clone(), frame_bytes);
        assert!(fresh.load(Ordering::Relaxed));
        let got = frame.lock().unwrap();
        assert_eq!(got.len(), frame_bytes);
        assert!(got.iter().all(|&b| b == 0x55), "partial tail must not be stored");
    }

    #[test]
    fn split_raw_empty_stream_never_stores() {
        let frame = Arc::new(std::sync::Mutex::new(Vec::new()));
        let fresh = Arc::new(AtomicBool::new(false));
        split_raw(&[][..], frame.clone(), fresh.clone(), 100);
        assert!(!fresh.load(Ordering::Relaxed));
        assert!(frame.lock().unwrap().is_empty());
    }
}
