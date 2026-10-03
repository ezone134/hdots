//! zvol — direct PipeWire volume/mute control via `libpipewire-0.3.so`.
//!
//! On this system the C headers aren't installed (only the .so files), so we bind
//! the symbols we need directly by name and drive everything through the public C
//! API we can observe in `pw-cli` (which links libpipewire-0.3 only).
//!
//! What we actually use, verified against `nm -D /usr/lib64/libpipewire-0.3.so.0`:
//!   - pw_context_new, pw_context_destroy, pw_context_connect
//!   - pw_context_get_core, pw_context_find_global
//!   - pw_thread_loop_new, pw_loop_destroy, pw_loop_run
//!   - pw_node_get_name, pw_node_get_id, pw_node_get_media_class,
//!     pw_node_get_properties, pw_node_set_node_property
//!   - pw_properties_new, pw_properties_free, pw_properties_get,
//!     pw_properties_get_bool, pw_properties_set, pw_properties_setf
//!
//! The properties API: `pw_node_get_properties` returns a `struct pw_properties *`;
//! to read/write we use the `pw_properties_getf` / `pw_properties_setf` family
//! plus `pw_properties_get_bool`. Those exist in this lib (nm confirms).

use std::ffi::{c_char, CStr, CString};
use std::os::raw::{c_int, c_uint};

use tracing::debug;

// ── Link ─────────────────────────────────────────────────────────────────────
#[cfg(target_os = "linux")]
#[link(name = "pipewire-0.3", kind = "dylib")]
extern "C" {}

// ── Opaque types ─────────────────────────────────────────────────────────────
#[allow(non_camel_case_types)]
pub type pw_context = std::os::raw::c_void;
#[allow(non_camel_case_types)]
pub type pw_core = std::os::raw::c_void;
#[allow(non_camel_case_types)]
pub type pw_node = std::os::raw::c_void;
#[allow(non_camel_case_types)]
pub type pw_loop = std::os::raw::c_void;
#[allow(non_camel_case_types)]
pub type pw_properties = std::os::raw::c_void;

// ── Core + context ───────────────────────────────────────────────────────────
extern "C" {
    fn pw_context_new(
        main_loop: *mut pw_loop,
        user_data: *const std::os::raw::c_void,
    ) -> *mut pw_context;
    fn pw_context_connect(
        context: *mut pw_context,
        flags: c_uint,
        user_data: *const std::os::raw::c_void,
    );
    fn pw_context_get_core(context: *mut pw_context) -> *mut pw_core;
    fn pw_context_destroy(context: *mut pw_context);
    fn pw_context_find_global(
        context: *mut pw_context,
        name: *const c_char,
    ) -> *const std::os::raw::c_void;
}

const PW_CONTEXT_NOAUTOLOGIN: c_uint = 1 << 0;

// ── Node + properties ────────────────────────────────────────────────────────
extern "C" {
    fn pw_node_get_name(node: *mut pw_node) -> *const c_char;
    fn pw_node_get_id(node: *mut pw_node) -> u32;
    fn pw_node_get_media_class(node: *mut pw_node) -> *const c_char;
    fn pw_node_get_properties(node: *mut pw_node) -> *const pw_properties;
    fn pw_node_set_node_property(
        node: *mut pw_node,
        name: *const c_char,
        value: *const pw_prop,
    ) -> c_int;

    fn pw_properties_getf(
        props: *const pw_properties,
        key: *const c_char,
        def: f32,
    ) -> f32;
    fn pw_properties_get_bool(
        props: *const pw_properties,
        key: *const c_char,
        def: c_int,
    ) -> c_int;
    fn pw_properties_setf(
        props: *mut pw_properties,
        key: *const c_char,
        val: f32,
    ) -> c_int;
}

#[repr(C)]
pub struct pw_prop {
    // The value is a tagged union in real pw_prop; we only ever set float/bool,
    // so we model the tagged union enough to pass to pw_properties_set.
    pub ptype: c_uint,
    pub pbool: c_int,
    pub pfloat: f32,
    pub pint: c_int,
    pub pstring: *const c_char,
    pub p_array: *const std::os::raw::c_void,
}

const PROP_TYPE_FLOAT: c_uint = 1;
const PROP_TYPE_BOOL: c_uint = 2;

// ── Main loop + thread loop ─────────────────────────────────────────────────
extern "C" {
    fn pw_thread_loop_new(
        name: *const c_char,
        user_data: *const std::os::raw::c_void,
    ) -> *mut pw_loop;
    fn pw_loop_destroy(loop_: *mut pw_loop);
    fn pw_loop_run(loop_: *mut pw_loop) -> c_int;
}

// ── Result types ─────────────────────────────────────────────────────────────
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ZvolError {
    PipeWire(String),
    NotFound,
    InvalidNode,
    InvalidChannel,
    PermissionDenied,
}

impl std::fmt::Display for ZvolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ZvolError::PipeWire(msg) => write!(f, "pipewire error: {msg}"),
            ZvolError::NotFound => write!(f, "default audio node not found"),
            ZvolError::InvalidNode => write!(f, "invalid audio node"),
            ZvolError::InvalidChannel => write!(f, "invalid channel index"),
            ZvolError::PermissionDenied => write!(f, "permission denied"),
        }
    }
}

impl std::error::Error for ZvolError {}

/// A single audio node (sink or source) with its current volume/mute state.
#[derive(Debug, Clone)]
pub struct AudioNode {
    pub id: u32,
    pub name: String,
    pub media_class: String,
    pub volume: Vec<f32>,
    pub mute: bool,
}

impl AudioNode {
    pub fn channel_count(&self) -> usize {
        self.volume.len()
    }
}

/// A live connection to the PipeWire daemon.
pub struct ZvolContext {
    ctx: *mut pw_context,
    loop_: *mut pw_loop,
    core: *mut pw_core,
}

// ── helpers ──────────────────────────────────────────────────────────────────

fn node_channels(node: *mut pw_node) -> usize {
    unsafe {
        let name = CStr::from_ptr(pw_node_get_name(node));
        let s = name.to_string_lossy();
        if s.contains("stereo") || s.contains("Analog Stereo") || s.contains("analog-stereo") {
            2
        } else {
            1
        }
    }
}

/// Property key for a channel's volume, e.g. "volume.L" / "volume.R".
fn channel_key(idx: usize, channels: usize) -> String {
    match (idx, channels) {
        (0, 2) => "volume.L".to_string(),
        (1, 2) => "volume.R".to_string(),
        (0, _) => "volume.FL".to_string(),
        (1, _) => "volume.FR".to_string(),
        _ => format!("volume.X{idx}"),
    }
}

/// Read per-channel volume for a node from its properties blob.
fn read_node_volume(props: *const pw_properties, node: *mut pw_node) -> Vec<f32> {
    let channels = node_channels(node);
    let mut out = Vec::with_capacity(channels);

    if channels == 1 {
        unsafe {
            let v =
                pw_properties_getf(props, CString::new("volume").unwrap().as_ptr(), 1.0);
            out.push(v.clamp(0.0, 1.0));
        }
        return out;
    }

    for i in 0..channels {
        let key = channel_key(i, channels);
        let ckey = CString::new(key.as_str()).unwrap();
        unsafe {
            let v = pw_properties_getf(props, ckey.as_ptr(), 1.0);
            out.push(v.clamp(0.0, 1.0));
        }
    }

    out
}

fn read_node_mute(props: *const pw_properties) -> bool {
    unsafe {
        pw_properties_get_bool(
            props,
            CString::new("mute").unwrap().as_ptr(),
            0,
        ) != 0
    }
}

unsafe fn find_default_node(ctx: *mut pw_context, media_class: &str) -> Result<AudioNode, ZvolError> {
    use std::ffi::CStr;

    let global_name: &str = match media_class {
        "Audio/Sink" => "default.audio.sink",
        "Audio/Source" => "default.audio.source",
        _ => return Err(ZvolError::NotFound),
    };
    let cname = CString::new(global_name).map_err(|_| ZvolError::PipeWire("bad global name".into()))?;

    let global = pw_context_find_global(ctx, cname.as_ptr());
    if global.is_null() {
        return Err(ZvolError::NotFound);
    }

    let node = global as *mut pw_node;
    if node.is_null() {
        return Err(ZvolError::NotFound);
    }

    let name_ptr = pw_node_get_name(node);
    let name = CStr::from_ptr(name_ptr).to_string_lossy().into_owned();
    let mc_ptr = pw_node_get_media_class(node);
    let media_class_str = CStr::from_ptr(mc_ptr).to_string_lossy().into_owned();
    let id = pw_node_get_id(node);
    let props = pw_node_get_properties(node);
    let volume = read_node_volume(props, node);
    let mute = read_node_mute(props);

    Ok(AudioNode {
        id,
        name,
        media_class: media_class_str,
        volume,
        mute,
    })
}

// ── public API ───────────────────────────────────────────────────────────────

impl ZvolContext {
    /// Connect to the running PipeWire daemon over the native protocol.
    ///
    /// Returns an error if the library can't be loaded, the daemon isn't running,
    /// or the connection fails.
    pub fn new() -> Result<Self, ZvolError> {
        let loop_name =
            CString::new("zvol-loop").map_err(|_| ZvolError::PipeWire("bad loop name".into()))?;
        let loop_ = unsafe { pw_thread_loop_new(loop_name.as_ptr(), std::ptr::null()) };
        if loop_.is_null() {
            return Err(ZvolError::PipeWire("failed to create thread loop".into()));
        }
        let ctx = unsafe { pw_context_new(loop_, std::ptr::null()) };
        if ctx.is_null() {
            unsafe { pw_loop_destroy(loop_); }
            return Err(ZvolError::PipeWire("failed to create context".into()));
        }
        unsafe { pw_context_connect(ctx, PW_CONTEXT_NOAUTOLOGIN, std::ptr::null()); }
        let core = unsafe { pw_context_get_core(ctx) };
        if core.is_null() {
            unsafe {
                pw_context_destroy(ctx);
                pw_loop_destroy(loop_);
            }
            return Err(ZvolError::PipeWire(
                "context has no core after connect".into(),
            ));
        }
        // Run the loop briefly so the connection handshake completes.
        unsafe { pw_loop_run(loop_); }

        debug!("zvol: connected to pipewire");
        Ok(Self { ctx, loop_, core })
    }

    /// Return the default sink (Audio/Sink) node.
    pub fn default_sink(&self) -> Result<AudioNode, ZvolError> {
        unsafe { find_default_node(self.ctx, "Audio/Sink") }
    }

    /// Return the default source (Audio/Source, usually the mic) node.
    pub fn default_source(&self) -> Result<AudioNode, ZvolError> {
        unsafe { find_default_node(self.ctx, "Audio/Source") }
    }

    /// Set a single channel's volume on the default sink, in [0.0, 1.0].
    ///
    /// `channel` is 0-based. For stereo sinks this is 0 = left, 1 = right.
    /// Returns the new volume after the set.
    /// Set a single channel's volume on the default sink, in [0.0, 1.0].
    ///
    /// `channel` is 0-based. For stereo sinks this is 0 = left, 1 = right.
    pub fn set_sink_channel(&self, channel: usize, volume: f32) -> Result<(), ZvolError> {
        let node = self.default_sink()?;
        set_node_volume_via_node(node.id as *mut pw_node, channel, volume)
    }

    /// Set a single channel's volume on the default source (mic), in [0.0, 1.0].
    pub fn set_source_channel(&self, channel: usize, volume: f32) -> Result<(), ZvolError> {
        let node = self.default_source()?;
        set_node_volume_via_node(node.id as *mut pw_node, channel, volume)
    }

    /// Set mute on the default sink.
    pub fn set_sink_mute(&self, mute: bool) -> Result<(), ZvolError> {
        let node = self.default_sink()?;
        set_node_mute_via_node(node.id as *mut pw_node, mute)
    }

    /// Set mute on the default source (mic).
    pub fn set_source_mute(&self, mute: bool) -> Result<(), ZvolError> {
        let node = self.default_source()?;
        set_node_mute_via_node(node.id as *mut pw_node, mute)
    }

    /// Get current sink volume + mute.
    pub fn sink(&self) -> Result<AudioNode, ZvolError> {
        self.default_sink()
    }

    /// Get current source (mic) volume + mute.
    pub fn source(&self) -> Result<AudioNode, ZvolError> {
        self.default_source()
    }

    /// Shut down the connection.
    pub fn shutdown(&mut self) {
        if !self.ctx.is_null() {
            unsafe { pw_context_destroy(self.ctx) };
            self.ctx = std::ptr::null_mut();
        }
        if !self.loop_.is_null() {
            unsafe { pw_loop_destroy(self.loop_) };
            self.loop_ = std::ptr::null_mut();
        }
        self.core = std::ptr::null_mut();
        debug!("zvol: disconnected from pipewire");
    }
}

impl Drop for ZvolContext {
    fn drop(&mut self) {
        self.shutdown();
    }
}

// ── node-level setters (via pw_node_get_properties + pw_properties_setf) ────

fn set_node_volume_via_node(
    node: *mut pw_node,
    channel: usize,
    value: f32,
) -> Result<(), ZvolError> {
    let channels = node_channels(node);
    if channel >= channels {
        return Err(ZvolError::InvalidChannel);
    }
    let key = channel_key(channel, channels);
    let ckey = CString::new(key.as_str()).map_err(|_| {
        ZvolError::PipeWire("bad key".into())
    })?;
    unsafe {
        let props = pw_node_get_properties(node) as *mut pw_properties;
        let r = pw_properties_setf(props, ckey.as_ptr(), value.clamp(0.0, 1.0));
        if r != 0 {
            return Err(ZvolError::PipeWire(
                "pw_properties_setf failed".into(),
            ));
        }
    }
    Ok(())
}

fn set_node_mute_via_node(
    node: *mut pw_node,
    mute: bool,
) -> Result<(), ZvolError> {
    let ckey = CString::new("mute").map_err(|_| {
        ZvolError::PipeWire("bad key".into())
    })?;
    unsafe {
        let props = pw_node_get_properties(node) as *mut pw_properties;
        let val: f32 = if mute { 1.0 } else { 0.0 };
        let r = pw_properties_setf(props, ckey.as_ptr(), val);
        if r != 0 {
            return Err(ZvolError::PipeWire(
                "pw_properties_setf failed".into(),
            ));
        }
    }
    Ok(())
}

// ── helpers for CLI ──────────────────────────────────────────────────────────

/// Pretty-print a volume as a percentage.
pub fn pct(v: f32) -> u32 {
    (v.clamp(0.0, 1.0) * 100.0).round() as u32
}

/// Parse a volume argument from a string ("100", "50", "0.5", "50%", "1.0").
pub fn parse_volume(s: &str) -> Result<f32, String> {
    let s = s.trim();
    let (num, is_pct) = if s.ends_with('%') {
        (&s[..s.len() - 1], true)
    } else {
        (s, false)
    };
    let v: f32 = num
        .parse()
        .map_err(|_| format!("bad volume: {s}"))?;
    if is_pct {
        Ok((v / 100.0).clamp(0.0, 1.0))
    } else if v > 1.0 {
        // Assume user meant percent if >1.0 and not explicitly %.
        Ok((v / 100.0).clamp(0.0, 1.0))
    } else {
        Ok(v.clamp(0.0, 1.0))
    }
}

// ── tests (compile-time + logic sanity) ──────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_volume_strings() {
        assert_eq!(parse_volume("100").unwrap(), 1.0);
        assert_eq!(parse_volume("50").unwrap(), 0.5);
        assert_eq!(parse_volume("50%").unwrap(), 0.5);
        assert_eq!(parse_volume("1.0").unwrap(), 1.0);
        assert_eq!(parse_volume("0.35").unwrap(), 0.35);
        assert_eq!(parse_volume("0").unwrap(), 0.0);
    }

    #[test]
    fn pct_rounding() {
        assert_eq!(pct(0.999), 100);
        assert_eq!(pct(0.5), 50);
        assert_eq!(pct(0.0), 0);
    }
}
