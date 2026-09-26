//! GL renderer (GLES3 or desktop GL 3.3 core via EGL).
//!
//! Two passes per frame, batched into a single dynamic VBO each:
//!  - solid pass: rounded-rect SDF quads (pill backgrounds, chips)
//!  - text pass:  premultiplied glyph textures
//!
//! Blending is premultiplied (`GL_ONE, GL_ONE_MINUS_SRC_ALPHA`) — matches
//! cosmic-text's premultiplied output and the C++ shell's Cairo pipeline.

use glow::HasContext;
use std::collections::{HashMap, VecDeque};
use std::ffi::c_void;
use wayland_egl::WlEglSurface;

/// Upper bound on cached glyph textures. Text is keyed by its full string,
/// so dynamic values (clock, media title, battery %, workspace names, stats)
/// would otherwise accumulate a fresh GL texture per unique string and never
/// free it — a slow, then terminal, memory leak that makes the shell
/// unresponsive after hours/days. With an LRU cap the live on-screen strings
/// stay warm and stale one-offs are evicted.
const TEXT_CAP: usize = 512;
/// Upper bound on cached image textures (icons, tray pixmaps, wallpaper
/// thumbnails). LRU-evicted past the cap for the same reason as text.
const IMG_CAP: usize = 384;

/// EGL constants missing from the `egl` 0.1 crate (stable EGL spec values).
const EGL_CONTEXT_MAJOR_VERSION: egl::EGLint = 0x3098;
const EGL_CONTEXT_MINOR_VERSION: egl::EGLint = 0x30FB;
const EGL_CONTEXT_OPENGL_PROFILE_MASK: egl::EGLint = 0x30FD;
const EGL_CONTEXT_OPENGL_CORE_PROFILE_BIT: egl::EGLint = 0x0000_0001;

// The `egl` 0.1 crate's `choose_config`/`gerconfigs` pass a NULL config
// buffer (a bug) and always return a NULL config. Declare the real binding
// with a proper output buffer instead.
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

/// Backend-neutral texture handle: `id` is backend-specific (GL texture name,
/// Vulkan texture slot) but the cache key/type is shared.
#[derive(Clone, Copy)]
pub struct Tex {
    pub id: u64,
    pub w: u32,
    pub h: u32,
}

pub struct Gl {
    pub ctx: glow::Context,
    pub display: egl::EGLDisplay,
    pub surface: egl::EGLSurface,
    pub context: egl::EGLContext,
    /// the `wl_egl_window` backing the EGL surface — MUST outlive the surface
    /// (destroying it early silently breaks buffer presentation)
    pub egl_window: WlEglSurface,
    pub w: i32,
    pub h: i32,

    solid_prog: glow::Program,
    tex_prog: glow::Program,
    solid_vao: glow::VertexArray,
    solid_vbo: glow::Buffer,
    tex_vao: glow::VertexArray,
    tex_vbo: glow::Buffer,
    res_loc: glow::UniformLocation,
    tex_res_loc: glow::UniformLocation,
    tex_loc: glow::UniformLocation,

    solid_verts: Vec<f32>,             // pos2 color4 local4 per vertex
    tex_quads: Vec<(u64, Vec<f32>)>,   // (texture id, pos2 uv2 color4 per vertex)

    /// glyph cache: key = (text, size_bits, color, is_icon, face, weight)
    pub text_cache: HashMap<(String, u32, u32, bool, u8, u8), Tex>,
    /// image cache: key = `ImageStore` key (`icon:<name>` / tray id)
    pub img_cache: HashMap<String, Tex>,
    /// LRU recency lists (front = most recently used; back = eviction target)
    /// so text_cache / img_cache stay bounded instead of leaking forever.
    text_lru: VecDeque<(String, u32, u32, bool, u8, u8)>,
    img_lru: VecDeque<String>,
}

fn build_program(
    ctx: &glow::Context,
    is_gles: bool,
    vert_body: &str,
    frag_body: &str,
    attribs: &[(&str, u32)],
) -> Result<glow::Program, String> {
    let version = if is_gles {
        "#version 300 es\nprecision highp float;\n"
    } else {
        "#version 330 core\n"
    };
    unsafe {
        let vs = ctx.create_shader(glow::VERTEX_SHADER)?;
        ctx.shader_source(vs, &format!("{version}{vert_body}"));
        ctx.compile_shader(vs);
        if !ctx.get_shader_compile_status(vs) {
            return Err(format!("vertex: {}", ctx.get_shader_info_log(vs)));
        }
        let fs = ctx.create_shader(glow::FRAGMENT_SHADER)?;
        ctx.shader_source(fs, &format!("{version}{frag_body}"));
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

const SOLID_VERT: &str = r#"
layout(location=0) in vec2 a_pos;
layout(location=1) in vec4 a_color;
layout(location=2) in vec4 a_local; // x,y local coords, z = stroke, w = unused
layout(location=3) in vec2 a_half;  // half-extents (size/2)
layout(location=4) in vec4 a_radii; // per-corner radii (tl,tr,br,bl), negative = concave
uniform vec2 u_res;
out vec4 v_color;
out vec4 v_local;
out vec2 v_half;
out vec4 v_radii;
void main() {
    v_color = a_color;
    v_local = a_local;
    v_half = a_half;
    v_radii = a_radii;
    vec2 ndc = (a_pos / u_res) * 2.0 - 1.0;
    gl_Position = vec4(ndc.x, -ndc.y, 0.0, 1.0);
}
"#;

const SOLID_FRAG: &str = r#"
in vec4 v_color;
in vec4 v_local;
in vec2 v_half;
in vec4 v_radii;
out vec4 o;
float sd_round_per_corner(vec2 p, vec2 b, vec4 radii) {
    vec2 ap = abs(p);
    float r;
    if (p.x >= 0.0) {
        r = (p.y >= 0.0) ? radii.z : radii.y;
    } else {
        r = (p.y >= 0.0) ? radii.w : radii.x;
    }
    if (r >= 0.0) {
        vec2 q = ap - b + r;
        return min(max(q.x, q.y), 0.0) + length(max(q, 0.0)) - r;
    } else {
        float ar = -r;
        vec2 d_rect = ap - b;
        float rect_d = max(d_rect.x, d_rect.y);
        float circle_d = ar - length(ap - b);
        return max(rect_d, circle_d);
    }
}
void main() {
    float d = sd_round_per_corner(v_local.xy - v_half, v_half, v_radii);
    float a = v_local.z > 0.0
        ? 1.0 - smoothstep(v_local.z - 1.0, v_local.z, abs(d))
        : 1.0 - smoothstep(0.0, 1.0, d);
    o = vec4(v_color.rgb * a, v_color.a * a);
}
"#;

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
    o = texture(u_tex, v_uv) * v_color;
}
"#;

impl Gl {
    /// Initialize EGL + GL. `display_ptr` is the raw `wl_display`, and
    /// `egl_surface_ptr` is the `wl_egl_window` pointer. The `egl_window` is
    /// kept alive for the lifetime of `Gl` — destroying it early makes the
    /// EGL surface reference a dead window and swaps never present.
    ///
    /// Probes desktop GL 3.3 core first, falls back to GLES 3.0.
    pub unsafe fn init(
        display_ptr: *mut c_void,
        egl_surface_ptr: *const c_void,
        egl_window: WlEglSurface,
        w: i32,
        h: i32,
    ) -> Result<Self, String> {
        let tr = std::env::var("ZEN_TRACE").is_ok();
        macro_rules! tr {
            ($($a:tt)*) => { if tr { eprintln!("zen: GL: {}", format!($($a)*)); } };
        }
        tr!("get_display");
        let display = egl::get_display(display_ptr as egl::EGLNativeDisplayType)
            .ok_or("eglGetDisplay failed")?;
        let (mut maj, mut min) = (0, 0);
        tr!("initialize");
        if !egl::initialize(display, &mut maj, &mut min) {
            return Err("eglInitialize failed".into());
        }
        tr!("initialized {maj}.{min}");

        // --- probe: GLES 3.0 first (proven on Wayland layer shells), then
        // desktop GL 3.3 core (opt-in via ZEN_DESKTOP_GL=1; some Mesa builds
        // crash in eglCreateWindowSurface for OPENGL_BIT configs on wl_egl). ---
        let want_desktop = std::env::var("ZEN_DESKTOP_GL").map(|v| v == "1").unwrap_or(false);
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
            tr!("probe GLES3");
            if let Some(cfg) = choose_config(display, &gles_attribs) {
                (cfg, vec![egl::CONTEXT_CLIENT_VERSION, 3, egl::NONE], true)
            } else if want_desktop {
                tr!("probe desktop GL 3.3 core");
                let attribs = [
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
                let cfg = choose_config(display, &attribs)
                    .ok_or("no usable EGL config (tried GLES3 + GL 3.3 core)")?;
                (
                    cfg,
                    vec![
                        EGL_CONTEXT_MAJOR_VERSION,
                        3,
                        EGL_CONTEXT_MINOR_VERSION,
                        3,
                        EGL_CONTEXT_OPENGL_PROFILE_MASK,
                        EGL_CONTEXT_OPENGL_CORE_PROFILE_BIT,
                        egl::NONE,
                    ],
                    false,
                )
            } else {
                return Err("no usable EGL config (GLES3 unavailable)".into());
            }
        };

        let api = if is_gles { egl::OPENGL_ES_API } else { egl::OPENGL_API };
        tr!("bind_api");
        if !egl::bind_api(api as egl::EGLenum) {
            return Err("eglBindAPI failed".into());
        }
        tr!("create_context");
        let context = egl::create_context(display, config, egl::NO_CONTEXT, &context_attribs)
            .ok_or_else(|| format!("eglCreateContext failed (err 0x{:x})", unsafe { eglGetError() }))?;
        tr!("create_window_surface");
        let surface = egl::create_window_surface(
            display,
            config,
            egl_surface_ptr as egl::EGLNativeWindowType,
            &[egl::NONE], // NB: never pass &[] — as_ptr() is a dangling 0x1
        )
        .ok_or_else(|| format!("eglCreateWindowSurface failed (err 0x{:x})", unsafe { eglGetError() }))?;
        tr!("make_current");
        if !egl::make_current(display, surface, surface, context) {
            return Err(format!("eglMakeCurrent failed (err 0x{:x})", unsafe { eglGetError() }));
        }
        tr!("glow loader");
        let ctx = glow::Context::from_loader_function(|name| {
            egl::get_proc_address(name) as *const c_void
        });
        tr!("build programs");

        let solid_prog = build_program(
            &ctx,
            is_gles,
            SOLID_VERT,
            SOLID_FRAG,
            &[("a_pos", 0), ("a_color", 1), ("a_local", 2), ("a_half", 3), ("a_radii", 4)],
        )?;
        let tex_prog = build_program(
            &ctx,
            is_gles,
            TEX_VERT,
            TEX_FRAG,
            &[("a_pos", 0), ("a_uv", 1), ("a_color", 2)],
        )?;

        let solid_vao = ctx.create_vertex_array()?;
        let solid_vbo = ctx.create_buffer()?;
        ctx.bind_vertex_array(Some(solid_vao));
        ctx.bind_buffer(glow::ARRAY_BUFFER, Some(solid_vbo));
        ctx.enable_vertex_attrib_array(0);
        ctx.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, 64, 0);
        ctx.enable_vertex_attrib_array(1);
        ctx.vertex_attrib_pointer_f32(1, 4, glow::FLOAT, false, 64, 8);
        ctx.enable_vertex_attrib_array(2);
        ctx.vertex_attrib_pointer_f32(2, 4, glow::FLOAT, false, 64, 24);
        ctx.enable_vertex_attrib_array(3);
        ctx.vertex_attrib_pointer_f32(3, 2, glow::FLOAT, false, 64, 40);
        ctx.enable_vertex_attrib_array(4);
        ctx.vertex_attrib_pointer_f32(4, 4, glow::FLOAT, false, 64, 48);

        let tex_vao = ctx.create_vertex_array()?;
        let tex_vbo = ctx.create_buffer()?;
        ctx.bind_vertex_array(Some(tex_vao));
        ctx.bind_buffer(glow::ARRAY_BUFFER, Some(tex_vbo));
        ctx.enable_vertex_attrib_array(0);
        ctx.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, 32, 0);
        ctx.enable_vertex_attrib_array(1);
        ctx.vertex_attrib_pointer_f32(1, 2, glow::FLOAT, false, 32, 8);
        ctx.enable_vertex_attrib_array(2);
        ctx.vertex_attrib_pointer_f32(2, 4, glow::FLOAT, false, 32, 16);
        ctx.bind_vertex_array(None);

        let res_loc = ctx.get_uniform_location(solid_prog, "u_res").unwrap();
        let tex_res_loc = ctx.get_uniform_location(tex_prog, "u_res").unwrap();
        let tex_loc = ctx.get_uniform_location(tex_prog, "u_tex").unwrap();

        unsafe {
            ctx.enable(glow::BLEND);
            ctx.blend_func(glow::ONE, glow::ONE_MINUS_SRC_ALPHA);
            ctx.disable(glow::DEPTH_TEST);
            ctx.disable(glow::CULL_FACE);
            ctx.viewport(0, 0, w, h);
        }

        Ok(Gl {
            ctx,
            display,
            surface,
            context,
            egl_window,
            w,
            h,
            solid_prog,
            tex_prog,
            solid_vao,
            solid_vbo,
            tex_vao,
            tex_vbo,
            res_loc,
            tex_res_loc,
            tex_loc,
            solid_verts: Vec::new(),
            tex_quads: Vec::new(),
            text_cache: HashMap::new(),
            img_cache: HashMap::new(),
            text_lru: VecDeque::new(),
            img_lru: VecDeque::new(),
        })
    }

    pub fn resize(&mut self, w: i32, h: i32) {
        self.w = w;
        self.h = h;
        // keep the wl_egl_window in sync so the next swap attaches a buffer of
        // the new size (this is what makes the hover morph actually resize)
        self.egl_window.resize(w, h, 0, 0);
        unsafe {
            self.ctx.viewport(0, 0, w, h);
        }
    }

    pub fn begin_frame(&mut self) {
        unsafe {
            // a previous frame's scissor must never leak into the next one
            self.ctx.disable(glow::SCISSOR_TEST);
            self.ctx.clear_color(0.0, 0.0, 0.0, 0.0);
            self.ctx.clear(glow::COLOR_BUFFER_BIT);
        }
        self.solid_verts.clear();
        self.tex_quads.clear();
    }

    fn premul(color: u32) -> [f32; 4] {
        let a = (color & 0xff) as f32 / 255.0;
        let r = ((color >> 24) & 0xff) as f32 / 255.0;
        let g = ((color >> 16) & 0xff) as f32 / 255.0;
        let b = ((color >> 8) & 0xff) as f32 / 255.0;
        [r * a, g * a, b * a, a]
    }

    /// Rounded-rect quad. `r` clamped to a capsule; `r=0` → sharp rect.
    /// Degenerate (negative) rects — e.g. a panel layout drawn once at the
    /// stale pill size before the size commit is acked — clamp to a zero
    /// radius instead of panicking on `min > max`.
    pub fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, r: f32, color: u32) {
        self.rect_ex(x, y, w, h, r, 0.0, color);
    }

    /// Rounded-rect quad with an optional stroke (`stroke > 0` → outline-only
    /// band of that thickness; the stroke is centered on the edge).
    ///
    /// The quad is expanded by `AA_PAD` on every side so the fragment shader's
    /// anti-aliasing band (and centered strokes) never clip at the quad edge —
    /// without this, a full circle (r = w/2) loses its outer AA ring at the
    /// four cardinal points and renders as a lumpy square-ish dot. The SDF
    /// stays in the ORIGINAL shape coordinates (a_half/radii unchanged), so
    /// shading is pixel-identical — only the rasterization window grows.
    #[allow(clippy::too_many_arguments)]
    pub fn rect_ex(&mut self, x: f32, y: f32, w: f32, h: f32, r: f32, stroke: f32, color: u32) {
        const AA_PAD: f32 = 2.0;
        let max_r = h.min(w).max(0.0) / 2.0;
        let r = r.clamp(0.0, max_r);
        let c = Self::premul(color);
        let x2 = x + w;
        let y2 = y + h;
        let hx = w / 2.0;
        let hy = h / 2.0;
        let push = |v: &mut Vec<f32>, px: f32, py: f32, lx: f32, ly: f32| {
            v.extend_from_slice(&[px, py, c[0], c[1], c[2], c[3], lx, ly, stroke, 0.0, hx, hy, r, r, r, r]);
        };
        let v = &mut self.solid_verts;
        push(v, x - AA_PAD, y - AA_PAD, -AA_PAD, -AA_PAD);
        push(v, x2 + AA_PAD, y - AA_PAD, w + AA_PAD, -AA_PAD);
        push(v, x2 + AA_PAD, y2 + AA_PAD, w + AA_PAD, h + AA_PAD);
        push(v, x - AA_PAD, y - AA_PAD, -AA_PAD, -AA_PAD);
        push(v, x2 + AA_PAD, y2 + AA_PAD, w + AA_PAD, h + AA_PAD);
        push(v, x - AA_PAD, y2 + AA_PAD, -AA_PAD, h + AA_PAD);
    }

    /// Rounded rect with per-corner radii — negative radius = concave corner.
    #[allow(clippy::too_many_arguments)]
    pub fn rect_concave(&mut self, x: f32, y: f32, w: f32, h: f32, r_tl: f32, r_tr: f32, r_br: f32, r_bl: f32, color: u32) {
        let max_r = h.min(w).max(0.0) / 2.0;
        let r_tl = r_tl.clamp(-max_r, max_r);
        let r_tr = r_tr.clamp(-max_r, max_r);
        let r_br = r_br.clamp(-max_r, max_r);
        let r_bl = r_bl.clamp(-max_r, max_r);
        let c = Self::premul(color);
        let x2 = x + w;
        let y2 = y + h;
        let hx = w / 2.0;
        let hy = h / 2.0;
        let push = |v: &mut Vec<f32>, px: f32, py: f32, lx: f32, ly: f32| {
            v.extend_from_slice(&[px, py, c[0], c[1], c[2], c[3], lx, ly, 0.0, 0.0, hx, hy, r_tl, r_tr, r_br, r_bl]);
        };
        let v = &mut self.solid_verts;
        push(v, x, y, 0.0, 0.0);
        push(v, x2, y, w, 0.0);
        push(v, x2, y2, w, h);
        push(v, x, y, 0.0, 0.0);
        push(v, x2, y2, w, h);
        push(v, x, y2, 0.0, h);
    }

    /// Thick line segment between two points — a capsule oriented along the
    /// segment (heartbeat-style graph lines). Same trick as the Vulkan
    /// backend: rotated corners + axis-aligned local coords feed the
    /// unchanged rounded-rect shader.
    pub fn line(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, t: f32, color: u32) {
        let dx = x1 - x0;
        let dy = y1 - y0;
        let len = (dx * dx + dy * dy).sqrt();
        if len < 0.01 {
            self.rect(x0 - t / 2.0, y0 - t / 2.0, t, t, t / 2.0, color);
            return;
        }
        let ux = dx / len;
        let uy = dy / len;
        let px = -uy * t / 2.0;
        let py = ux * t / 2.0;
        let e = t / 2.0; // end extension → seamless joints
        let ax = x0 - ux * e;
        let ay = y0 - uy * e;
        let bx = x1 + ux * e;
        let by = y1 + uy * e;
        let l = len + t;
        let c = Self::premul(color);
        let rad = t / 2.0;
        let push = |v: &mut Vec<f32>, pxx: f32, pyy: f32, lx: f32, ly: f32| {
            v.extend_from_slice(&[pxx, pyy, c[0], c[1], c[2], c[3], lx, ly, 0.0, 0.0, l / 2.0, t / 2.0, rad, rad, rad, rad]);
        };
        let v = &mut self.solid_verts;
        push(v, ax + px, ay + py, 0.0, 0.0);
        push(v, bx + px, by + py, l, 0.0);
        push(v, bx - px, by - py, l, t);
        push(v, ax + px, ay + py, 0.0, 0.0);
        push(v, bx - px, by - py, l, t);
        push(v, ax - px, ay - py, 0.0, t);
    }

    /// Textured quad (premultiplied glyph texture), batched per-texture.
    pub fn text_quad(&mut self, tex: &Tex, x: f32, y: f32, w: f32, h: f32) {
        self.text_quad_tinted(tex, x, y, w, h, 0xffffffff);
    }

    /// Textured quad with a tint (0xRRGGBBAA, premultiplied in the shader) —
    /// used for the wallpaper fade. `0xffffffff` ≡ `text_quad`.
    pub fn text_quad_tinted(&mut self, tex: &Tex, x: f32, y: f32, w: f32, h: f32, tint: u32) {
        let glow_tex = glow::NativeTexture(std::num::NonZeroU32::new(tex.id as u32).unwrap());
        let c = Self::premul(tint);
        self.text_quad_gl(glow_tex, x, y, w, h, c);
    }

    fn text_quad_gl(
        &mut self,
        tex: glow::Texture,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        c: [f32; 4],
    ) {
        self.text_quad_uv_gl(tex, x, y, w, h, 0.0, 0.0, 1.0, 1.0, c);
    }

    /// Textured quad sampling only a UV window of the texture — the world
    /// map's band texture: pan/zoom moves the window, the raster never
    /// re-rasterizes. `u0..v0..u1..v1` in 0..1 texture space, y-flip
    /// conventional (v=0 = top row).
    #[allow(clippy::too_many_arguments)]
    pub fn text_quad_uv(
        &mut self,
        tex: &Tex,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        u0: f32,
        v0: f32,
        u1: f32,
        v1: f32,
    ) {
        let u0 = u0.clamp(0.0, 1.0);
        let v0 = v0.clamp(0.0, 1.0);
        let u1 = u1.clamp(0.0, 1.0);
        let v1 = v1.clamp(0.0, 1.0);
        let glow_tex = glow::NativeTexture(std::num::NonZeroU32::new(tex.id as u32).unwrap());
        let c = Self::premul(0xffffffff);
        self.text_quad_uv_gl(glow_tex, x, y, w, h, u0, v0, u1, v1, c);
    }

    fn text_quad_uv_gl(
        &mut self,
        tex: glow::Texture,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        u0: f32,
        v0: f32,
        u1: f32,
        v1: f32,
        c: [f32; 4],
    ) {
        let x2 = x + w;
        let y2 = y + h;
        let mut verts = Vec::with_capacity(24);
        let push = |v: &mut Vec<f32>, px: f32, py: f32, u: f32, vt: f32| {
            v.extend_from_slice(&[px, py, u, vt, c[0], c[1], c[2], c[3]]);
        };
        push(&mut verts, x, y, u0, v0);
        push(&mut verts, x2, y, u1, v0);
        push(&mut verts, x2, y2, u1, v1);
        push(&mut verts, x, y, u0, v0);
        push(&mut verts, x2, y2, u1, v1);
        push(&mut verts, x, y2, u0, v1);
        let tex_id = tex.0.get() as u64;
        match self.tex_quads.iter_mut().find(|(t, _)| *t == tex_id) {
            Some((_, v)) => v.extend_from_slice(&verts),
            None => self.tex_quads.push((tex_id, verts)),
        }
    }

    /// Re-assert this surface as the EGL current one. Multiple `Gl` instances
    /// (bar + wallpaper + notification surfaces) share one EGL display and
    /// switch surfaces with `eglMakeCurrent`; each frame must re-make-current
    /// before drawing so `swap_buffers` targets the right surface.
    pub fn make_current(&mut self) {
        // safe fn — but must run before drawing each frame: another surface's
        // Gl may have switched the shared display's current EGL surface
        let _ = egl::make_current(self.display, self.surface, self.surface, self.context);
    }

    /// Resolve a cached `Tex` handle back to a glow texture for the flush loop.
    fn tex_handle(id: u64) -> glow::Texture {
        glow::NativeTexture(std::num::NonZeroU32::new(id as u32).unwrap())
    }

    /// Upload premultiplied RGBA8 pixels as a texture (CLAMP_TO_EDGE, NEAREST).
    pub fn upload_texture(&mut self, w: u32, h: u32, pixels: &[u8]) -> Result<Tex, String> {
        self.upload_texture_impl(w, h, pixels, false)
    }

    /// Same as `upload_texture` but LINEAR min/mag filtering — used for the
    /// world-map band texture, which is deliberately stretched/scaled by the
    /// GPU during pan-zoom instead of being re-rasterized per frame.
    pub fn upload_texture_linear(&mut self, w: u32, h: u32, pixels: &[u8]) -> Result<Tex, String> {
        self.upload_texture_impl(w, h, pixels, true)
    }

    fn upload_texture_impl(&mut self, w: u32, h: u32, pixels: &[u8], smooth: bool) -> Result<Tex, String> {
        unsafe {
            let id = self.ctx.create_texture()?;
            let tex_id = id.0.get() as u64;
            self.ctx.bind_texture(glow::TEXTURE_2D, Some(id));
            self.ctx.tex_image_2d(
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
            // NEAREST: glyph textures are drawn 1:1 at integer positions —
            // LINEAR would smear them when positions are fractional. The world
            // band uses LINEAR so stretched pan-zoom stays soft, never blocky.
            let filter = if smooth { glow::LINEAR } else { glow::NEAREST };
            self.ctx.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, filter as i32);
            self.ctx.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, filter as i32);
            self.ctx.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_WRAP_S,
                glow::CLAMP_TO_EDGE as i32,
            );
            self.ctx.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_WRAP_T,
                glow::CLAMP_TO_EDGE as i32,
            );
            self.ctx.bind_texture(glow::TEXTURE_2D, None);
            Ok(Tex { id: tex_id, w, h })
        }
    }

    // ------------------------------------------------------------- caches
    // Text and image textures are cached as GL objects keyed by their source
    // string. Those caches are bounded by LRU eviction (see TEXT_CAP / IMG_CAP):
    // the live set of strings stays warm and stale one-offs are freed, so a
    // long-running shell never accumulates unbounded GPU memory.

    pub fn text_contains(&self, key: &(String, u32, u32, bool, u8, u8)) -> bool {
        self.text_cache.contains_key(key)
    }
    pub fn text_get(&mut self, key: &(String, u32, u32, bool, u8, u8)) -> Option<Tex> {
        if !self.text_cache.contains_key(key) {
            return None;
        }
        self.touch_text(key.clone());
        self.text_cache.get(key).copied()
    }
    pub fn text_insert(&mut self, key: (String, u32, u32, bool, u8, u8), t: Tex) {
        self.text_cache.insert(key.clone(), t);
        self.touch_text(key);
        while self.text_cache.len() > TEXT_CAP {
            if !self.evict_text() {
                break;
            }
        }
    }

    fn touch_text(&mut self, key: (String, u32, u32, bool, u8, u8)) {
        // move the key to the front (MRU); O(n) but the list is ≤ TEXT_CAP
        if let Some(pos) = self.text_lru.iter().position(|k| k == &key) {
            self.text_lru.remove(pos);
        }
        self.text_lru.push_front(key);
    }

    /// Evict the least-recently-used text texture (free it, drop the key).
    /// Returns false when there's nothing to evict.
    fn evict_text(&mut self) -> bool {
        let Some(key) = self.text_lru.pop_back() else { return false };
        if let Some(t) = self.text_cache.remove(&key) {
            unsafe {
                let tex = glow::NativeTexture(std::num::NonZeroU32::new(t.id as u32).unwrap());
                self.ctx.delete_texture(tex);
            }
        }
        true
    }

    pub fn img_contains(&self, key: &str) -> bool {
        self.img_cache.contains_key(key)
    }
    pub fn img_insert(&mut self, key: String, t: Tex) {
        self.img_cache.insert(key.clone(), t);
        self.touch_img(&key);
        while self.img_cache.len() > IMG_CAP {
            if !self.evict_img() {
                break;
            }
        }
    }
    pub fn img_get(&mut self, key: &str) -> Option<Tex> {
        if !self.img_cache.contains_key(key) {
            return None;
        }
        self.touch_img(key);
        self.img_cache.get(key).copied()
    }

    /// Drop a texture immediately (live camera frames) — the next draw of
    /// that key re-uploads from the `ImageStore` pixels.
    pub fn img_remove(&mut self, key: &str) {
        if let Some(t) = self.img_cache.remove(key) {
            self.img_lru.retain(|k| k != key);
            unsafe {
                let tex = glow::NativeTexture(std::num::NonZeroU32::new(t.id as u32).unwrap());
                self.ctx.delete_texture(tex);
            }
        }
    }

    fn touch_img(&mut self, key: &str) {
        if let Some(pos) = self.img_lru.iter().position(|k| k.as_str() == key) {
            self.img_lru.remove(pos);
        }
        self.img_lru.push_front(key.to_string());
    }

    fn evict_img(&mut self) -> bool {
        let Some(key) = self.img_lru.pop_back() else { return false };
        if let Some(t) = self.img_cache.remove(&key) {
            unsafe {
                let tex = glow::NativeTexture(std::num::NonZeroU32::new(t.id as u32).unwrap());
                self.ctx.delete_texture(tex);
            }
        }
        true
    }

    /// Draw the accumulated geometry with the CURRENT GL state — does not
    /// present. Used by the scissor commands, which must split the batch at a
    /// scissor boundary (flush once, switch the test, keep accumulating).
    fn flush_batch(&mut self) {
        unsafe {
            let res = [self.w as f32, self.h as f32];

            if !self.solid_verts.is_empty() {
                self.ctx.use_program(Some(self.solid_prog));
                self.ctx.uniform_2_f32(Some(&self.res_loc), res[0], res[1]);
                self.ctx.bind_vertex_array(Some(self.solid_vao));
                self.ctx.bind_buffer(glow::ARRAY_BUFFER, Some(self.solid_vbo));
                self.ctx
                    .buffer_data_u8_slice(glow::ARRAY_BUFFER, as_bytes(&self.solid_verts), glow::STREAM_DRAW);
                self.ctx
                    .draw_arrays(glow::TRIANGLES, 0, (self.solid_verts.len() / 16) as i32);
            }

            if !self.tex_quads.is_empty() {
                self.ctx.use_program(Some(self.tex_prog));
                self.ctx.uniform_2_f32(Some(&self.tex_res_loc), res[0], res[1]);
                self.ctx.uniform_1_i32(Some(&self.tex_loc), 0);
                self.ctx.active_texture(glow::TEXTURE0);
                self.ctx.bind_vertex_array(Some(self.tex_vao));
                self.ctx.bind_buffer(glow::ARRAY_BUFFER, Some(self.tex_vbo));
                for (tex_id, verts) in &self.tex_quads {
                    let tex = Self::tex_handle(*tex_id);
                    self.ctx.bind_texture(glow::TEXTURE_2D, Some(tex));
                    self.ctx
                        .buffer_data_u8_slice(glow::ARRAY_BUFFER, as_bytes(verts), glow::STREAM_DRAW);
                    self.ctx.draw_arrays(glow::TRIANGLES, 0, (verts.len() / 8) as i32);
                }
                self.ctx.bind_texture(glow::TEXTURE_2D, None);
            }

            self.ctx.bind_vertex_array(None);
        }
    }

    /// Restrict subsequent drawing (both solid and text quads) to the given
    /// pixel rect, expressed in scene coords (top-left origin). Splits the
    /// current batch at the boundary so already-queued geometry keeps its
    /// PREVIOUS clip state. Degenerate (w/h <= 0) rects clip everything.
    pub fn set_scissor(&mut self, x: f32, y: f32, w: f32, h: f32) {
        self.flush_batch();
        unsafe {
            self.ctx.enable(glow::SCISSOR_TEST);
            // GL scissor uses window coords with a bottom-left origin, while
            // scene coords are top-down — flip the vertical axis.
            let sx = x.round() as i32;
            let sw = w.round().max(0.0) as i32;
            let sh = h.round().max(0.0) as i32;
            let sy = (self.h as f32 - y - h).round() as i32;
            self.ctx.scissor(sx, sy, sw, sh);
        }
    }

    /// Lift the active scissor: flush the clipped batch, then disable the test.
    pub fn clear_scissor(&mut self) {
        self.flush_batch();
        unsafe {
            self.ctx.disable(glow::SCISSOR_TEST);
        }
    }

    /// Flush batched geometry and present.
    pub fn flush(&mut self) {
        self.flush_batch();
        egl::swap_buffers(self.display, self.surface);
    }
}

fn as_bytes(v: &[f32]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(v.as_ptr() as *const u8, v.len() * 4) }
}

impl Drop for Gl {
    fn drop(&mut self) {
        unsafe {
            for tex in self.text_cache.values().chain(self.img_cache.values()) {
                let t = glow::NativeTexture(std::num::NonZeroU32::new(tex.id as u32).unwrap());
                self.ctx.delete_texture(t);
            }
            self.ctx.delete_program(self.solid_prog);
            self.ctx.delete_program(self.tex_prog);
            self.ctx.delete_vertex_array(self.solid_vao);
            self.ctx.delete_vertex_array(self.tex_vao);
            self.ctx.delete_buffer(self.solid_vbo);
            self.ctx.delete_buffer(self.tex_vbo);
            egl::make_current(self.display, egl::NO_SURFACE, egl::NO_SURFACE, egl::NO_CONTEXT);
            egl::destroy_surface(self.display, self.surface);
            egl::destroy_context(self.display, self.context);
            egl::terminate(self.display);
        }
    }
}
