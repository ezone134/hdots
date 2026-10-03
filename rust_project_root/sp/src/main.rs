//! sp — GPU-accelerated Wayland splash screen / pop window.
//!
//! Reads $states2/sp_vars (JSON), renders text on a solid background,
//! optionally shows a loading animation, sleeps for `duration` seconds, exits.

use std::env;
use std::ffi::c_void;
use std::fs;
use std::os::unix::io::{AsFd, AsRawFd};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use serde::Deserialize;

// Wayland
use smithay_client_toolkit::compositor::{CompositorHandler, CompositorState};
use smithay_client_toolkit::output::{OutputHandler, OutputState};
use smithay_client_toolkit::registry::{ProvidesRegistryState, RegistryState};
use smithay_client_toolkit::seat::keyboard::{KeyboardHandler, Keysym};
use smithay_client_toolkit::seat::{Capability, SeatHandler, SeatState};
use smithay_client_toolkit::shell::wlr_layer::{
    Anchor, KeyboardInteractivity, Layer, LayerShell, LayerShellHandler, LayerSurface,
    LayerSurfaceConfigure,
};
use smithay_client_toolkit::shell::WaylandSurface;

use wayland_client::globals::registry_queue_init;
use wayland_client::protocol::{wl_keyboard, wl_output, wl_seat, wl_surface};
use wayland_client::{Connection, Proxy, QueueHandle};
use wayland_egl::WlEglSurface;

// Text
use cosmic_text::fontdb::Database;
use cosmic_text::{Attrs, Buffer, Color, Family, FontSystem, Metrics, Shaping, SwashCache};

// GL
use glow::HasContext;

// ── sp_vars (JSON) ──────────────────────────────────────────────

fn default_true() -> bool {
    true
}

#[derive(Deserialize)]
struct Vars {
    #[serde(default = "default_val_f32_100")]
    w: f32,
    #[serde(default = "default_val_f32_100")]
    h: f32,
    #[serde(default = "default_val_bg")]
    bg: String,
    #[serde(default = "default_val_fg")]
    fg: String,
    #[serde(default)]
    b_text: String,
    #[serde(default = "default_val_sans")]
    b_font: String,
    #[serde(default = "default_val_f32_100")]
    b_size: f32,
    #[serde(default)]
    s_text: String,
    #[serde(default = "default_val_sans")]
    s_font: String,
    #[serde(default = "default_val_f32_40")]
    s_size: f32,
    #[serde(default = "default_val_u64_3")]
    duration: u64,
    #[serde(default = "default_val_f32_5")]
    spacing: f32,
    #[serde(default = "default_val_u32_1")]
    loading_id: u32,
    #[serde(default = "default_true")]
    show_loading_animation: bool,
    #[serde(default)]
    center_b_text: u32,
    #[serde(default)]
    corner_rounding: f32,
    /// Window/background opacity in percent (0-100), default 100 = opaque.
    #[serde(default = "default_val_alpha")]
    alpha: u8,
}

fn default_val_f32_100() -> f32 { 100.0 }
fn default_val_f32_40() -> f32 { 40.0 }
fn default_val_f32_5() -> f32 { 50.0 }
fn default_val_sans() -> String { "sans-serif".into() }
fn default_val_u64_3() -> u64 { 3 }
fn default_val_u32_1() -> u32 { 1 }
fn default_val_bg() -> String { "#000000".into() }
fn default_val_fg() -> String { "#ffffff".into() }
fn default_val_alpha() -> u8 { 100 }

impl Vars {
    /// Background as `0xRRGGBBAA`.
    fn bg_u32(&self) -> u32 { parse_hex(&self.bg) }
    /// Foreground as `0xRRGGBBAA`.
    fn fg_u32(&self) -> u32 { parse_hex(&self.fg) }
}

/// Parse `#rrggbb` or `#rrggbbaa` (CSS order, alpha last) into `0xRRGGBBAA`.
/// Six-digit input is fully opaque; anything malformed falls back to opaque black.
fn parse_hex(s: &str) -> u32 {
    let t = s.trim().trim_start_matches('#');
    let rgb = match t.get(..6) {
        Some(h) => u32::from_str_radix(h, 16).unwrap_or(0),
        None => 0,
    };
    let a = match t.get(6..8) {
        Some(h) => u32::from_str_radix(h, 16).unwrap_or(0xff),
        None => 0xff,
    };
    (rgb << 8) | a
}

fn parse_vars() -> Vars {
    let states2 = env::var("states2").unwrap_or_else(|_| "/tmp/tw_rconf/states2".into());
    let path = PathBuf::from(states2).join("sp_vars");
    let content = fs::read_to_string(&path).unwrap_or_default();
    serde_json::from_str(&content).unwrap_or_else(|e| {
        eprintln!("sp: parse sp_vars: {e}");
        // Return defaults
        Vars {
            w: 100.0,
            h: 100.0,
            bg: "#000000".into(),
            fg: "#ffffff".into(),
            b_text: String::new(),
            b_font: "sans-serif".into(),
            b_size: 100.0,
            s_text: String::new(),
            s_font: "sans-serif".into(),
            s_size: 40.0,
            duration: 3,
            spacing: 50.0,
            loading_id: 1,
            show_loading_animation: true,
            center_b_text: 0,
            corner_rounding: 0.0,
            alpha: 100,
        }
    })
}

// ── loading animation ────────────────────────────────────────────

fn loading_frames(id: u32) -> Vec<&'static str> {
    match id {
        1 => vec!["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"],
        2 => vec!["⣾", "⣽", "⣻", "⢿", "⡿", "⣟", "⣯", "⣷"],
        3 => vec!["◜", "◝", "◞", "◟"],
        _ => vec!["|", "/", "-", "\\"],
    }
}

// ── fonts (targeted loading) ────────────────────────────────────

fn font_dirs() -> Vec<PathBuf> {
    let mut dirs = vec![
        "/usr/share/fonts".into(),
        "/usr/local/share/fonts".into(),
        "/run/host/fonts".into(),
    ];
    if let Ok(home) = env::var("HOME") {
        dirs.push(format!("{home}/.local/share/fonts").into());
        dirs.push(format!("{home}/.fonts").into());
    }
    if let Ok(xdg) = env::var("XDG_DATA_HOME") {
        dirs.push(format!("{xdg}/fonts").into());
    }
    dirs
}

fn is_font_file(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|x| x.to_str())
        .is_some_and(|x| matches!(x, "ttf" | "otf" | "ttc"))
}

fn file_families(path: &std::path::Path) -> Vec<String> {
    let Ok(data) = fs::read(path) else {
        return Vec::new();
    };
    let count = ttf_parser::fonts_in_collection(&data).unwrap_or(1);
    let mut out: Vec<String> = Vec::new();
    for i in 0..count {
        let Ok(face) = ttf_parser::Face::parse(&data, i) else {
            continue;
        };
        for name in face.names() {
            let nid = name.name_id;
            if nid != ttf_parser::name_id::FAMILY
                && nid != ttf_parser::name_id::TYPOGRAPHIC_FAMILY
            {
                continue;
            }
            if let Some(s) = name.to_string() {
                if !out.iter().any(|o: &String| o.eq_ignore_ascii_case(s.as_str())) {
                    out.push(s);
                }
            }
        }
    }
    out
}

fn family_matches(families: &[String], wanted: &[String]) -> bool {
    families
        .iter()
        .any(|f| wanted.iter().any(|w| !w.is_empty() && f.eq_ignore_ascii_case(w)))
}

fn load_fonts(db: &mut Database, wanted: &[String]) {
    let mut loaded = 0;

    fn walk(dir: &std::path::Path, depth: usize, db: &mut Database, wanted: &[String], loaded: &mut usize) {
        if depth > 8 {
            return;
        }
        let Ok(rd) = fs::read_dir(dir) else {
            return;
        };
        for entry in rd.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, depth + 1, db, wanted, loaded);
            } else if is_font_file(&path)
                && family_matches(&file_families(&path), wanted)
                && db.load_font_file(&path).is_ok()
            {
                *loaded += 1;
            }
        }
    }

    for dir in font_dirs() {
        walk(&dir, 0, db, wanted, &mut loaded);
    }
    if loaded == 0 {
        debug_log("sp: no wanted fonts found in scan, loading system fonts");
        db.load_system_fonts();
    } else {
        debug_log(&format!("sp: loaded {loaded} wanted font files"));
        db.load_system_fonts();
    }
}

// ── text rasterization ──────────────────────────────────────────

/// Rasterize `text` into premultiplied RGBA8 pixels.
/// Returns (width, height, pixels).
fn rasterize(
    font_system: &mut FontSystem,
    swash: &mut SwashCache,
    text: &str,
    size: f32,
    color: u32,
    family: &str,
) -> Option<(u32, u32, Vec<u8>)> {
    if text.is_empty() || size <= 0.0 {
        return None;
    }
    let size = size.max(1.0);
    let attrs = Attrs::new().family(Family::Name(family));
    let line_h = (size * 1.3).ceil();

    // Measure
    let mut buf = Buffer::new(font_system, Metrics::new(size, line_h));
    buf.set_size(None, None);
    buf.set_text(text, &attrs, Shaping::Advanced, None);
    buf.shape_until_scroll(font_system, true);
    let mw = buf
        .layout_runs()
        .map(|r| r.line_w)
        .fold(0.0f32, f32::max);
    let w = (mw as u32).max(1) + 4;
    let h = (line_h as u32).max(1) + 4;

    // Rasterize
    let mut buf = Buffer::new(font_system, Metrics::new(size, line_h));
    buf.set_size(Some(w as f32), Some(h as f32));
    buf.set_text(text, &attrs, Shaping::Advanced, None);
    buf.shape_until_scroll(font_system, true);

    let mut pixels = vec![0u8; (w * h * 4) as usize];
    // 0xRRGGBBAA → cosmic-text 0xAARRGGBB
    let col = Color(((color & 0xff) << 24) | (color >> 8));
    buf.draw(font_system, swash, col, |x, y, gw, gh, c| {
        let a = c.a();
        if a == 0 {
            return;
        }
        let r = c.r() as u32 * a as u32 / 255;
        let g = c.g() as u32 * a as u32 / 255;
        let b = c.b() as u32 * a as u32 / 255;
        for dy in 0..gh {
            for dx in 0..gw {
                let px = x + dx as i32;
                let py = y + dy as i32;
                if px < 0 || py < 0 || px >= w as i32 || py >= h as i32 {
                    continue;
                }
                let idx = ((py * w as i32 + px) * 4) as usize;
                pixels[idx] = r as u8;
                pixels[idx + 1] = g as u8;
                pixels[idx + 2] = b as u8;
                pixels[idx + 3] = a;
            }
        }
    });

    let non_zero = pixels.iter().filter(|&&b| b != 0).count();
    debug_log(&format!("sp: rasterize text={text:?} family={family} size={size} w={w} h={h} non_zero_px={non_zero}"));
    Some((w, h, pixels))
}

// ── EGL / GL ────────────────────────────────────────────────────

// Workaround: the `egl` 0.1 crate's choose_config passes NULL.
#[link(name = "EGL")]
extern "C" {
    fn eglChooseConfig(
        display: egl::EGLDisplay,
        attribs: *const egl::EGLint,
        configs: *mut egl::EGLConfig,
        config_size: egl::EGLint,
        num_config: *mut egl::EGLint,
    ) -> egl::EGLBoolean;
    fn eglGetError() -> egl::EGLint;
}

fn choose_config(display: egl::EGLDisplay, attribs: &[egl::EGLint]) -> Option<egl::EGLConfig> {
    let mut buf: [egl::EGLConfig; 8] = [std::ptr::null_mut(); 8];
    let mut n: egl::EGLint = 0;
    unsafe {
        if eglChooseConfig(
            display,
            attribs.as_ptr(),
            buf.as_mut_ptr(),
            buf.len() as egl::EGLint,
            &mut n,
        ) == egl::TRUE
            && n > 0
        {
            Some(buf[0])
        } else {
            None
        }
    }
}

const TEX_VERT: &str = r#"
layout(location=0) in vec2 a_pos;
layout(location=1) in vec2 a_uv;
layout(location=2) in vec4 a_color;
uniform vec2 u_res;
out vec2 v_uv;
out vec4 v_color;
void main() {
    v_uv = a_uv;
    v_color = a_color;
    vec2 ndc = (a_pos / u_res) * 2.0 - 1.0;
    gl_Position = vec4(ndc.x, -ndc.y, 0.0, 1.0);
}
"#;

const TEX_FRAG: &str = r#"
in vec2 v_uv;
in vec4 v_color;
uniform sampler2D u_tex;
out vec4 o;
void main() {
    vec4 c = texture(u_tex, v_uv) * v_color;
    // Texels are already premultiplied (rgba = color * coverage). Output them
    // as-is; the compositor's de-premultiply recovers the exact color at
    // antialiased edges. Re-multiplying c.rgb * c.a would square the coverage
    // and darken every edge pixel.
    o = vec4(c.rgb, c.a);
}
"#;

const ROUND_VERT: &str = r#"
layout(location=0) in vec2 a_pos;
uniform vec2 u_res;
out vec2 v_pos;
void main() {
    v_pos = a_pos;
    vec2 ndc = (a_pos / u_res) * 2.0 - 1.0;
    gl_Position = vec4(ndc.x, -ndc.y, 0.0, 1.0);
}
"#;

const ROUND_FRAG: &str = r#"
uniform vec2 u_center;
uniform vec2 u_half;
uniform float u_radius;
uniform vec4 u_color;
in vec2 v_pos;
out vec4 o;
float sd_round_rect(vec2 p, vec2 b, float r) {
    vec2 q = abs(p) - b + vec2(r);
    return min(max(q.x, q.y), 0.0) + length(max(q, 0.0)) - r;
}
void main() {
    vec2 p = v_pos - u_center;
    float d = sd_round_rect(p, u_half, u_radius);
    float a = 1.0 - smoothstep(-1.0, 1.0, d);
    float af = u_color.a * a;
    // Premultiplied output so the compositor's de-premultiply recovers the
    // exact color instead of overshooting to white at antialiased edges.
    o = vec4(u_color.rgb * af, af);
}
"#;

fn build_program(
    ctx: &glow::Context,
    is_gles: bool,
    vert_src: &str,
    frag_src: &str,
    attribs: &[(&str, u32)],
) -> Result<glow::Program, String> {
    let version = if is_gles {
        "#version 300 es\nprecision highp float;\n"
    } else {
        "#version 330 core\n"
    };
    unsafe {
        let vs = ctx.create_shader(glow::VERTEX_SHADER)?;
        ctx.shader_source(vs, &format!("{version}{vert_src}"));
        ctx.compile_shader(vs);
        if !ctx.get_shader_compile_status(vs) {
            return Err(format!("vertex: {}", ctx.get_shader_info_log(vs)));
        }
        let fs = ctx.create_shader(glow::FRAGMENT_SHADER)?;
        ctx.shader_source(fs, &format!("{version}{frag_src}"));
        ctx.compile_shader(fs);
        if !ctx.get_shader_compile_status(fs) {
            return Err(format!("fragment: {}", ctx.get_shader_info_log(fs)));
        }
        let prog = ctx.create_program()?;
        for (name, loc) in attribs {
            ctx.bind_attrib_location(prog, *loc, name);
        }
        ctx.attach_shader(prog, vs);
        ctx.attach_shader(prog, fs);
        ctx.link_program(prog);
        if !ctx.get_program_link_status(prog) {
            return Err(format!("link: {}", ctx.get_program_info_log(prog)));
        }
        ctx.detach_shader(prog, vs);
        ctx.detach_shader(prog, fs);
        ctx.delete_shader(vs);
        ctx.delete_shader(fs);
        Ok(prog)
    }
}

fn upload_texture(
    ctx: &glow::Context,
    w: u32,
    h: u32,
    pixels: &[u8],
) -> Result<glow::NativeTexture, String> {
    unsafe {
        let id = ctx.create_texture()?;
        ctx.bind_texture(glow::TEXTURE_2D, Some(id));
        ctx.tex_image_2d(
            glow::TEXTURE_2D,
            0,
            glow::RGBA8 as i32,
            w as i32,
            h as i32,
            0,
            glow::RGBA,
            glow::UNSIGNED_BYTE,
            glow::PixelUnpackData::Slice(Some(pixels)),
        );
        ctx.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_MIN_FILTER,
            glow::NEAREST as i32,
        );
        ctx.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_MAG_FILTER,
            glow::NEAREST as i32,
        );
        ctx.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_WRAP_S,
            glow::CLAMP_TO_EDGE as i32,
        );
        ctx.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_WRAP_T,
            glow::CLAMP_TO_EDGE as i32,
        );
        ctx.bind_texture(glow::TEXTURE_2D, None);
        Ok(id)
    }
}

struct GlState {
    egl_display: egl::EGLDisplay,
    egl_surface: egl::EGLSurface,
    egl_context: egl::EGLContext,
    _egl_window: WlEglSurface,
    ctx: glow::Context,
    tex_prog: glow::Program,
    tex_vao: glow::VertexArray,
    tex_vbo: glow::Buffer,
    res_loc: glow::UniformLocation,
    tex_loc: glow::UniformLocation,
    round_prog: glow::Program,
    round_vao: glow::VertexArray,
    round_vbo: glow::Buffer,
    round_res_loc: glow::UniformLocation,
    round_radius_loc: glow::UniformLocation,
    round_color_loc: glow::UniformLocation,
    round_center_loc: glow::UniformLocation,
    round_half_loc: glow::UniformLocation,
}

impl GlState {
    unsafe fn init(
        display_ptr: *mut c_void,
        wl_surface: &wl_surface::WlSurface,
        w: i32,
        h: i32,
    ) -> Result<Self, String> {
        let egl_window = WlEglSurface::new(wl_surface.id(), w, h)
            .map_err(|e| format!("wl_egl_window: {e:?}"))?;

        let display = egl::get_display(display_ptr as egl::EGLNativeDisplayType)
            .ok_or("eglGetDisplay failed")?;
        let (mut maj, mut min) = (0, 0);
        if !egl::initialize(display, &mut maj, &mut min) {
            return Err("eglInitialize failed".into());
        }

        // Probe GLES 3.0 first, then desktop GL 3.3 core
        let (config, context_attribs, is_gles) = {
            let gles_attribs = [
                egl::SURFACE_TYPE,
                egl::WINDOW_BIT,
                egl::RED_SIZE,
                8,
                egl::GREEN_SIZE,
                8,
                egl::BLUE_SIZE,
                8,
                egl::ALPHA_SIZE,
                8,
                egl::RENDERABLE_TYPE,
                egl::OPENGL_ES2_BIT,
                egl::NONE,
            ];
            if let Some(cfg) = choose_config(display, &gles_attribs) {
                (cfg, vec![egl::CONTEXT_CLIENT_VERSION, 3, egl::NONE], true)
            } else {
                let gl_attribs = [
                    egl::SURFACE_TYPE,
                    egl::WINDOW_BIT,
                    egl::RED_SIZE,
                    8,
                    egl::GREEN_SIZE,
                    8,
                    egl::BLUE_SIZE,
                    8,
                    egl::ALPHA_SIZE,
                    8,
                    egl::RENDERABLE_TYPE,
                    egl::OPENGL_BIT,
                    egl::NONE,
                ];
                let cfg =
                    choose_config(display, &gl_attribs).ok_or("no usable EGL config")?;
                (
                    cfg,
                    vec![
                        0x3098, 3, // EGL_CONTEXT_MAJOR_VERSION
                        0x30FB, 3, // EGL_CONTEXT_MINOR_VERSION
                        0x30FD, 0x0000_0001, // EGL_CONTEXT_OPENGL_PROFILE_MASK = CORE
                        egl::NONE,
                    ],
                    false,
                )
            }
        };

        let api = if is_gles {
            egl::OPENGL_ES_API
        } else {
            egl::OPENGL_API
        };
        if !egl::bind_api(api as egl::EGLenum) {
            return Err("eglBindAPI failed".into());
        }

        let context = egl::create_context(display, config, egl::NO_CONTEXT, &context_attribs)
            .ok_or_else(|| {
                format!(
                    "eglCreateContext failed (err 0x{:x})",
                    eglGetError()
                )
            })?;

        let surface = egl::create_window_surface(
            display,
            config,
            egl_window.ptr() as egl::EGLNativeWindowType,
            &[egl::NONE],
        )
        .ok_or_else(|| {
            format!(
                "eglCreateWindowSurface failed (err 0x{:x})",
                eglGetError()
            )
        })?;

        if !egl::make_current(display, surface, surface, context) {
            return Err(format!(
                "eglMakeCurrent failed (err 0x{:x})",
                eglGetError()
            ));
        }

        let gl = glow::Context::from_loader_function(|name| {
            egl::get_proc_address(name) as *const c_void
        });

        let tex_prog = build_program(
            &gl,
            is_gles,
            TEX_VERT,
            TEX_FRAG,
            &[("a_pos", 0), ("a_uv", 1), ("a_color", 2)],
        )?;

        let tex_vao = gl.create_vertex_array()?;
        let tex_vbo = gl.create_buffer()?;
        gl.bind_vertex_array(Some(tex_vao));
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(tex_vbo));
        gl.enable_vertex_attrib_array(0);
        gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, 32, 0);
        gl.enable_vertex_attrib_array(1);
        gl.vertex_attrib_pointer_f32(1, 2, glow::FLOAT, false, 32, 8);
        gl.enable_vertex_attrib_array(2);
        gl.vertex_attrib_pointer_f32(2, 4, glow::FLOAT, false, 32, 16);
        gl.bind_vertex_array(None);

        let res_loc = gl.get_uniform_location(tex_prog, "u_res").unwrap();
        let tex_loc = gl.get_uniform_location(tex_prog, "u_tex").unwrap();

        // Rounded-rect background program (draws a fullscreen rounded quad)
        let round_prog = build_program(
            &gl,
            is_gles,
            ROUND_VERT,
            ROUND_FRAG,
            &[("a_pos", 0)],
        )?;
        let round_vao = gl.create_vertex_array()?;
        let round_vbo = gl.create_buffer()?;
        gl.bind_vertex_array(Some(round_vao));
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(round_vbo));
        gl.enable_vertex_attrib_array(0);
        gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, 8, 0);
        gl.bind_vertex_array(None);
        let round_res_loc = gl.get_uniform_location(round_prog, "u_res").unwrap();
        let round_radius_loc = gl.get_uniform_location(round_prog, "u_radius").unwrap();
        let round_color_loc = gl.get_uniform_location(round_prog, "u_color").unwrap();
        let round_center_loc = gl.get_uniform_location(round_prog, "u_center").unwrap();
        let round_half_loc = gl.get_uniform_location(round_prog, "u_half").unwrap();

        gl.enable(glow::BLEND);
        gl.blend_func(glow::ONE, glow::ONE_MINUS_SRC_ALPHA);
        gl.disable(glow::DEPTH_TEST);
        gl.viewport(0, 0, w, h);

        Ok(GlState {
            egl_display: display,
            egl_surface: surface,
            egl_context: context,
            _egl_window: egl_window,
            ctx: gl,
            tex_prog,
            tex_vao,
            tex_vbo,
            res_loc,
            tex_loc,
            round_prog,
            round_vao,
            round_vbo,
            round_res_loc,
            round_radius_loc,
            round_color_loc,
            round_center_loc,
            round_half_loc,
        })
    }

    fn draw(
        &self,
        w: i32,
        h: i32,
        bg: u32,
        rounding: f32,
        alpha: u8,
        textures: &[(glow::NativeTexture, u32, u32, f32, f32)],
    ) {
        let gl = &self.ctx;
        let mut alpha_f = (alpha as f32 / 100.0).clamp(0.0, 1.0);
        unsafe {
            static mut DRAW_COUNT: u32 = 0;
            DRAW_COUNT += 1;
            let draw_n = DRAW_COUNT;
            if draw_n <= 3 {
                let mut msg = format!("sp: draw #{draw_n} w={w} h={h} bg=0x{bg:08x} rounding={rounding} alpha={alpha} textures={}\n", textures.len());
                for (i, (_tex, tw, th, x, y)) in textures.iter().enumerate() {
                    msg += &format!("  tex[{i}]: {tw}x{th} at ({x:.1},{y:.1})\n");
                }
                debug_log(&msg);
            }
            let r = ((bg >> 24) & 0xff) as f32 / 255.0;
            let g = ((bg >> 16) & 0xff) as f32 / 255.0;
            let b = ((bg >> 8) & 0xff) as f32 / 255.0;
            // An 8-digit `bg` carries its own alpha byte; it scales the global
            // `alpha` percent so both knobs compose instead of fighting.
            alpha_f *= (bg & 0xff) as f32 / 255.0;

            // Clear to transparent; backgrounds are drawn as a quad so rounded
            // corners can punch through.
            gl.clear_color(0.0, 0.0, 0.0, 0.0);
            gl.clear(glow::COLOR_BUFFER_BIT);

            if rounding > 0.0 {
                // Rounded card: cover only the content bounding box (plus a little
                // padding) so the card is centered over the content while the rest
                // of the overlay stays transparent. Note the fragment shader works
                // in pixel coords and u_half is the half extent minus the radius.
                let pad = (rounding + 12.0) * 2.0;
                let mut min_x = f32::MAX;
                let mut min_y = f32::MAX;
                let mut max_x = f32::MIN;
                let mut max_y = f32::MIN;
                for (_, tw, th, x, y) in textures {
                    min_x = min_x.min(*x);
                    min_y = min_y.min(*y);
                    max_x = max_x.max(*x + *tw as f32);
                    max_y = max_y.max(*y + *th as f32);
                }
                let (x0, y0, x1, y1) = if min_x <= max_x {
                    (
                        (min_x - pad).max(0.0).min(w as f32 - 1.0),
                        (min_y - pad).max(0.0).min(h as f32 - 1.0),
                        (max_x + pad).max(1.0).min(w as f32),
                        (max_y + pad).max(1.0).min(h as f32),
                    )
                } else {
                    (0.0, 0.0, w as f32, h as f32)
                };
                let cw = (x1 - x0).max(1.0);
                let ch = (y1 - y0).max(1.0);
                let radius = rounding.min(cw / 2.0).min(ch / 2.0);

                // Rounded background quad
                gl.use_program(Some(self.round_prog));
                gl.uniform_2_f32(Some(&self.round_res_loc), w as f32, h as f32);
                gl.uniform_2_f32(
                    Some(&self.round_center_loc),
                    (x0 + x1) * 0.5,
                    (y0 + y1) * 0.5,
                );
                gl.uniform_2_f32(
                    Some(&self.round_half_loc),
                    (cw * 0.5 - radius).max(0.5),
                    (ch * 0.5 - radius).max(0.5),
                );
                gl.uniform_1_f32(Some(&self.round_radius_loc), radius);
                gl.uniform_4_f32(Some(&self.round_color_loc), r, g, b, alpha_f);
                gl.bind_vertex_array(Some(self.round_vao));
                gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.round_vbo));
                #[rustfmt::skip]
                let verts: [f32; 12] = [
                    x0, y0, x1, y0, x1, y1,
                    x0, y0, x1, y1, x0, y1,
                ];
                gl.buffer_data_u8_slice(
                    glow::ARRAY_BUFFER,
                    std::slice::from_raw_parts(verts.as_ptr() as *const u8, verts.len() * 4),
                    glow::STREAM_DRAW,
                );
                gl.draw_arrays(glow::TRIANGLES, 0, 6);
                gl.bind_vertex_array(None);
            } else {
                // Premultiplied to match Wayland buffer conventions.
                gl.clear_color(r * alpha_f, g * alpha_f, b * alpha_f, alpha_f);
                gl.clear(glow::COLOR_BUFFER_BIT);
            }

            // Draw text quads
            gl.use_program(Some(self.tex_prog));
            gl.uniform_2_f32(Some(&self.res_loc), w as f32, h as f32);
            gl.uniform_1_i32(Some(&self.tex_loc), 0);
            gl.active_texture(glow::TEXTURE0);
            gl.bind_vertex_array(Some(self.tex_vao));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.tex_vbo));

            for (tex, tw, th, x, y) in textures {
                let x2 = *x + *tw as f32;
                let y2 = *y + *th as f32;
                // pos2, uv2, color4 — premultiplied white
                #[rustfmt::skip]
                let verts: [f32; 48] = [
                    *x, *y,  0.0, 0.0,  1.0, 1.0, 1.0, 1.0,
                    x2, *y,  1.0, 0.0,  1.0, 1.0, 1.0, 1.0,
                    x2, y2,  1.0, 1.0,  1.0, 1.0, 1.0, 1.0,
                    *x, *y,  0.0, 0.0,  1.0, 1.0, 1.0, 1.0,
                    x2, y2,  1.0, 1.0,  1.0, 1.0, 1.0, 1.0,
                    *x, y2,  0.0, 1.0,  1.0, 1.0, 1.0, 1.0,
                ];
                gl.bind_texture(glow::TEXTURE_2D, Some(*tex));
                gl.buffer_data_u8_slice(
                    glow::ARRAY_BUFFER,
                    std::slice::from_raw_parts(verts.as_ptr() as *const u8, verts.len() * 4),
                    glow::STREAM_DRAW,
                );
                gl.draw_arrays(glow::TRIANGLES, 0, 6);
            }

            gl.bind_vertex_array(None);

            egl::swap_buffers(self.egl_display, self.egl_surface);
        }
    }
}

impl Drop for GlState {
    fn drop(&mut self) {
        unsafe {
            self.ctx.delete_program(self.tex_prog);
            self.ctx.delete_vertex_array(self.tex_vao);
            self.ctx.delete_buffer(self.tex_vbo);
            self.ctx.delete_program(self.round_prog);
            self.ctx.delete_vertex_array(self.round_vao);
            self.ctx.delete_buffer(self.round_vbo);
            egl::make_current(
                self.egl_display,
                egl::NO_SURFACE,
                egl::NO_SURFACE,
                egl::NO_CONTEXT,
            );
            egl::destroy_surface(self.egl_display, self.egl_surface);
            egl::destroy_context(self.egl_display, self.egl_context);
            egl::terminate(self.egl_display);
        }
    }
}

// ── Wayland app ─────────────────────────────────────────────────

struct App {
    conn: Connection,
    #[allow(dead_code)]
    qh: QueueHandle<App>,
    #[allow(dead_code)] // held for lifetime
    compositor: CompositorState,
    #[allow(dead_code)] // held for lifetime
    layer_shell: LayerShell,
    registry_state: RegistryState,
    output_state: OutputState,
    // seat + keyboard — needed so the fullscreen overlay can receive key events
    // (Esc / Enter close the popup like rofi) instead of being keyboard-dead
    seat_state: SeatState,
    keyboard: Option<wl_keyboard::WlKeyboard>,
    /// Set after the surface's on-screen lifetime is over (a `capability` drop
    /// raced our keyboard handling) so we close exactly once.
    cap_exhausted: bool,
    layer: Option<LayerSurface>,
    vars: Vars,
    configured: (i32, i32),
    gl: Option<GlState>,
    font_system: FontSystem,
    swash: SwashCache,
    // Pre-rasterized textures
    b_tex: Option<(glow::NativeTexture, u32, u32)>,
    s_tex: Option<(glow::NativeTexture, u32, u32)>,
    anim_texs: Vec<(glow::NativeTexture, u32, u32)>,
    anim_start: Instant,
}

impl App {
    /// Unconditionally tear the popup down (Esc / Enter / keyboard-leave /
    /// surface-closed). Destroys the surface first so the compositor drops the
    /// overlay, then exits.
    fn close(&mut self) {
        if let Some(layer) = self.layer.take() {
            layer.wl_surface().destroy();
        }
        if let Some(k) = self.keyboard.take() {
            k.release();
        }
        let _ = self.conn.flush();
        std::process::exit(0);
    }

    /// Initialize GL and upload all textures (called once after first configure).
    fn init_textures(&mut self) {
        let (w, h) = self.configured;
        if w <= 0 || h <= 0 || self.gl.is_some() {
            return;
        }

        // Init GL
        let display_ptr = self.conn.backend().display_ptr() as *mut c_void;
        let layer = self.layer.as_ref().unwrap();
        match unsafe { GlState::init(display_ptr, layer.wl_surface(), w, h) } {
            Ok(gl) => self.gl = Some(gl),
            Err(e) => {
                eprintln!("sp: GL init: {e}");
                std::process::exit(1);
            }
        }

        let gl = self.gl.as_ref().unwrap();
        let fg = self.vars.fg_u32(); // 0xRRGGBBAA (6-digit input → opaque)

        // Rasterize big text
        if !self.vars.b_text.is_empty() {
            if let Some((tw, th, px)) = rasterize(
                &mut self.font_system,
                &mut self.swash,
                &self.vars.b_text,
                self.vars.b_size,
                fg,
                &self.vars.b_font,
            ) {
                if let Ok(tex) = upload_texture(&gl.ctx, tw, th, &px) {
                    self.b_tex = Some((tex, tw, th));
                }
            }
        }

        // Rasterize small text
        if !self.vars.s_text.is_empty() {
            if let Some((tw, th, px)) = rasterize(
                &mut self.font_system,
                &mut self.swash,
                &self.vars.s_text,
                self.vars.s_size,
                fg,
                &self.vars.s_font,
            ) {
                if let Ok(tex) = upload_texture(&gl.ctx, tw, th, &px) {
                    self.s_tex = Some((tex, tw, th));
                }
            }
        }

        // Rasterize animation frames
        if self.vars.show_loading_animation {
            let frames = loading_frames(self.vars.loading_id);
            for frame_text in frames {
                if let Some((tw, th, px)) = rasterize(
                    &mut self.font_system,
                    &mut self.swash,
                    frame_text,
                    self.vars.s_size,
                    fg,
                    &self.vars.s_font,
                ) {
                    if let Ok(tex) = upload_texture(&gl.ctx, tw, th, &px) {
                        self.anim_texs.push((tex, tw, th));
                    }
                }
            }
        }
    }

    /// Draw the current frame (called every animation tick).
    fn draw(&self) {
        let (w, h) = self.configured;
        if w <= 0 || h <= 0 {
            return;
        }
        let Some(gl) = self.gl.as_ref() else {
            return;
        };

        // Calculate layout
        let mut elements: Vec<(glow::NativeTexture, u32, u32, f32, f32)> = Vec::new();

        let b_h = self.b_tex.as_ref().map_or(0.0, |(_, _, th)| *th as f32);
        let s_h = self.s_tex.as_ref().map_or(0.0, |(_, _, th)| *th as f32);
        let anim_h = if self.vars.show_loading_animation && !self.anim_texs.is_empty() {
            self.anim_texs[0].2 as f32
        } else {
            0.0
        };

        let has_b = self.b_tex.is_some();
        let has_anim = self.vars.show_loading_animation && !self.anim_texs.is_empty();
        let has_s = self.s_tex.is_some();

        let gap = self.vars.spacing;

        // Stack height with gaps only where elements follow one another.
        let mut total = 0.0;
        total += if has_b { b_h } else { 0.0 };
        if has_b && (has_anim || has_s) {
            total += gap;
        }
        total += if has_anim { anim_h } else { 0.0 };
        if has_anim && has_s {
            total += gap;
        }
        total += if has_s { s_h } else { 0.0 };

        // center_b_text=1: pin the big text at the exact vertical center and stack
        // the loading animation + small text below it. Otherwise center the whole
        // group vertically.
        let mut y = if self.vars.center_b_text != 0 && has_b {
            (h as f32 - b_h) / 2.0
        } else {
            (h as f32 - total) / 2.0
        }
        .round();

        // Big text: centered horizontally
        if let Some((tex, tw, th)) = &self.b_tex {
            let x = ((w as f32 - *tw as f32) / 2.0).round();
            elements.push((*tex, *tw, *th, x, y));
            y += b_h + gap;
        }

        // Animation: below big text
        if has_anim {
            let elapsed = self.anim_start.elapsed();
            let frame_ms = 100; // 10fps
            let idx = (elapsed.as_millis() / frame_ms) as usize % self.anim_texs.len();
            let (tex, tw, th) = &self.anim_texs[idx];
            let x = ((w as f32 - *tw as f32) / 2.0).round();
            elements.push((*tex, *tw, *th, x, y));
            y += anim_h + gap;
        }

        // Small text: below animation
        if let Some((tex, tw, th)) = &self.s_tex {
            let x = ((w as f32 - *tw as f32) / 2.0).round();
            elements.push((*tex, *tw, *th, x, y));
        }

        gl.draw(
            w,
            h,
            self.vars.bg_u32(),
            self.vars.corner_rounding,
            self.vars.alpha,
            &elements,
        );

        // Present: the EGL swap only attaches the buffer; the wl_surface must be
        // committed for the compositor to show it. Without this the first frame is
        // shown once and every later frame (loading animation) is ignored.
        if let Some(layer) = self.layer.as_ref() {
            layer.commit();
        }
        self.conn.flush().ok();
    }
}

// ── Wayland handlers ────────────────────────────────────────────

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
    fn frame(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: u32) {
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

impl LayerShellHandler for App {
    fn closed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &LayerSurface) {
        self.close();
    }

    fn configure(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &LayerSurface,
        configure: LayerSurfaceConfigure,
        _: u32,
    ) {
        let w = configure.new_size.0 as i32;
        let h = configure.new_size.1 as i32;
        if w > 0 && h > 0 {
            self.configured = (w, h);
        }
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
    fn new_seat(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _seat: wl_seat::WlSeat,
    ) {
    }
    fn new_capability(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        seat: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Keyboard && self.keyboard.is_none() {
            if let Ok(kbd) = self.seat_state.get_keyboard(qh, &seat, None) {
                self.keyboard = Some(kbd);
            }
        }
    }
    fn remove_capability(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _seat: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Keyboard {
            if let Some(k) = self.keyboard.take() {
                k.release();
            }
            // A `leave` would already have closed us, but protect against the
            // seat being yanked mid-grab with the surface still focused.
            if !self.cap_exhausted {
                self.cap_exhausted = true;
                debug_log("sp: keyboard capability dropped — closing");
                self.close();
            }
        }
    }
    fn remove_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}
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
        // Keyboard focus left our overlay — an ambient exterior keypress stole
        // it, or the compositor dropped the grab. Close just like rofi does
        // when it loses focus.
        debug_log("sp: keyboard leave — closing");
        self.close();
    }
    fn press_key(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        event: smithay_client_toolkit::seat::keyboard::KeyEvent,
    ) {
        match event.keysym {
            Keysym::Escape => {
                debug_log("sp: Esc pressed — closing");
                self.close();
            }
            Keysym::Return | Keysym::KP_Enter => {
                debug_log("sp: Enter pressed — closing");
                self.close();
            }
            _ => {}
        }
    }
    fn repeat_key(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        _: smithay_client_toolkit::seat::keyboard::KeyEvent,
    ) {
    }
    fn release_key(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        _: smithay_client_toolkit::seat::keyboard::KeyEvent,
    ) {
    }
    fn update_modifiers(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        _: smithay_client_toolkit::seat::keyboard::Modifiers,
        _: smithay_client_toolkit::seat::keyboard::RawModifiers,
        _: u32,
    ) {
    }
}

impl ProvidesRegistryState for App {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    smithay_client_toolkit::registry_handlers![OutputState, SeatState];
}

smithay_client_toolkit::delegate_registry!(App);
smithay_client_toolkit::delegate_dispatch2!(App);

// ── main ────────────────────────────────────────────────────────

extern "C" fn exit_handler(_: libc::c_int) {
    std::process::exit(0);
}

fn debug_log(msg: &str) {
    use std::io::Write;
    let _ = std::fs::OpenOptions::new().create(true).append(true)
        .open("/tmp/sp_debug.log")
        .and_then(|mut f| { writeln!(f, "{msg}") });
}

fn main() {
    let _ = std::fs::write("/tmp/sp_debug.log", "\n--- sp start ---\n");

    let vars = parse_vars();
    debug_log(&format!("vars: b_text={:?} s_text={:?} b_font={:?} s_font={:?} b_size={} s_size={} fg={:?} bg={:?}",
        vars.b_text, vars.s_text, vars.b_font, vars.s_font, vars.b_size, vars.s_size, vars.fg, vars.bg));

    // Load fonts (targeted — only the two configured families)
    let mut db = Database::new();
    let wanted = vec![vars.b_font.clone(), vars.s_font.clone()];
    load_fonts(&mut db, &wanted);
    let font_system = FontSystem::new_with_locale_and_db("en_US.UTF-8".into(), db);
    let swash = SwashCache::new();

    // Connect Wayland
    let conn = Connection::connect_to_env().expect("sp: wayland");
    let (globals, mut event_queue) =
        registry_queue_init::<App>(&conn).expect("sp: globals");
    let qh = event_queue.handle();

    // Bind protocols
    let compositor = CompositorState::bind(&globals, &qh).expect("sp: wl_compositor");
    let layer_shell = LayerShell::bind(&globals, &qh).expect("sp: wlr-layer-shell");
    let output_state = OutputState::new(&globals, &qh);
    let registry_state = RegistryState::new(&globals);
    // Also bind the seat so the overlay can grab the keyboard (Esc / Enter
    // close it like rofi). Seats bind lazily on `new_capability`.
    let seat_state = SeatState::new(&globals, &qh);

    // Fullscreen overlay — covers all outputs, no input, no reserve
    let surface = compositor.create_surface(&qh);
    let layer = layer_shell.create_layer_surface(
        &qh,
        surface,
        Layer::Overlay,
        Some("sp"),
        None,
    );
    layer.set_anchor(Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT);
    layer.set_exclusive_zone(-1);
    // Grab the keyboard so Esc / Enter dismiss the popup like rofi. Exclusive:
    // no other client receives keys while we're up (a popup is transient).
    layer.set_keyboard_interactivity(KeyboardInteractivity::Exclusive);
    layer.commit();
    conn.flush().expect("sp: flush");

    let mut app = App {
        conn,
        qh,
        compositor,
        layer_shell,
        registry_state,
        output_state,
        seat_state,
        keyboard: None,
        cap_exhausted: false,
        layer: Some(layer),
        vars,
        configured: (0, 0),
        gl: None,
        font_system,
        swash,
        b_tex: None,
        s_tex: None,
        anim_texs: Vec::new(),
        anim_start: Instant::now(),
    };

    // First roundtrip → compositor sends configure
    event_queue
        .roundtrip(&mut app)
        .expect("sp: roundtrip");

    // Init GL + upload all textures
    app.init_textures();
    debug_log(&format!("sp: after init: configured={:?} gl={} b_tex={} s_tex={} anim={}",
        app.configured, app.gl.is_some(), app.b_tex.is_some(), app.s_tex.is_some(), app.anim_texs.len()));

    // Free font data (no longer needed)
    app.font_system = FontSystem::new_with_locale_and_db("".into(), Database::new());
    app.swash = SwashCache::new();

    // Signals
    unsafe {
        libc::signal(libc::SIGTERM, exit_handler as *const () as libc::sighandler_t);
        libc::signal(libc::SIGINT, exit_handler as *const () as libc::sighandler_t);
    }

    // Animation loop — runs for `duration` seconds
    app.anim_start = Instant::now();
    let frame_dur = Duration::from_millis(100); // 10fps

    loop {
        // Read + dispatch incoming Wayland events. Reading the socket is required so
        // wl_buffer.release messages from the compositor are processed — otherwise EGL
        // never gets its buffers back, swap_buffers stalls, and the loading animation
        // freezes after the first frame or two. Use a 1ms poll so we never block.
        if let Some(guard) = app.conn.prepare_read() {
            let fd = app.conn.as_fd().as_raw_fd();
            let mut pfds = [libc::pollfd {
                fd,
                events: libc::POLLIN,
                revents: 0,
            }];
            let n = unsafe { libc::poll(pfds.as_mut_ptr(), 1, 1) };
            if n > 0 && (pfds[0].revents & libc::POLLIN) != 0 {
                let _ = guard.read();
            }
        }
        event_queue
            .dispatch_pending(&mut app)
            .ok();

        // Draw current animation frame (EGL swap + surface commit inside)
        app.draw();

        // Check if duration elapsed
        if app.anim_start.elapsed() >= Duration::from_secs(app.vars.duration) {
            break;
        }

        std::thread::sleep(frame_dur);
    }
}
