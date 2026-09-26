//! Raw OpenGL backend: EGL window surface over a Wayland `wl_egl_window`,
//! loaded with glow. GLES 3.0 first, desktop GL 3.3 core fallback — the same
//! proven stack zen-shell uses. (Vulkan will land as a second `GpuBackend`
//! implementation in `render_vk.rs`.)

use glow::HasContext;
use std::ffi::c_void;
use wayland_egl::WlEglSurface;

use crate::atlas::Slot;
use crate::render::GpuBackend;

/// EGL constants missing from the `egl` 0.1 crate (stable EGL spec values).
const EGL_CONTEXT_MAJOR_VERSION: egl::EGLint = 0x3098;
const EGL_CONTEXT_MINOR_VERSION: egl::EGLint = 0x30FB;
const EGL_CONTEXT_OPENGL_PROFILE_MASK: egl::EGLint = 0x30FD;
const EGL_CONTEXT_OPENGL_CORE_PROFILE_BIT: egl::EGLint = 0x0000_0001;

// The `egl` 0.1 crate's `choose_config`/`gerconfigs` pass a NULL config
// buffer (a bug) and always return a NULL config. Declare the real binding.
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
    fn eglSwapInterval(display: egl::EGLDisplay, interval: egl::EGLint) -> egl::EGLBoolean;
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
uniform vec2 u_res;
out vec4 v_color;
void main() {
    v_color = a_color;
    vec2 ndc = (a_pos / u_res) * 2.0 - 1.0;
    gl_Position = vec4(ndc.x, -ndc.y, 0.0, 1.0);
}
"#;

const SOLID_FRAG: &str = r#"
in vec4 v_color;
out vec4 o;
void main() {
    o = v_color;
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

pub struct Gl {
    ctx: glow::Context,
    display: egl::EGLDisplay,
    surface: egl::EGLSurface,
    context: egl::EGLContext,
    /// the `wl_egl_window` backing the EGL surface — MUST outlive the surface
    egl_window: WlEglSurface,
    w: i32,
    h: i32,

    solid_prog: glow::Program,
    tex_prog: glow::Program,
    solid_vao: glow::VertexArray,
    solid_vbo: glow::Buffer,
    tex_vao: glow::VertexArray,
    tex_vbo: glow::Buffer,
    solid_res: glow::UniformLocation,
    tex_res: glow::UniformLocation,
    tex_loc: glow::UniformLocation,

    atlas_tex: Option<glow::Texture>,
    atlas_size: u32,

    solid_verts: Vec<f32>, // pos2 color4 per vertex
    tex_verts: Vec<f32>,   // pos2 uv2 color4 per vertex
}

impl Gl {
    /// Initialize EGL + GL. `display_ptr` is the raw `wl_display`;
    /// `egl_surface_ptr` is the `wl_egl_window` pointer. The `egl_window` is
    /// kept alive for the lifetime of `Gl` — destroying it early makes the EGL
    /// surface reference a dead window and swaps never present.
    pub unsafe fn init(
        display_ptr: *mut c_void,
        egl_surface_ptr: *const c_void,
        egl_window: WlEglSurface,
        w: i32,
        h: i32,
    ) -> Result<Self, String> {
        let display = egl::get_display(display_ptr as egl::EGLNativeDisplayType)
            .ok_or("eglGetDisplay failed")?;
        let (mut maj, mut min) = (0, 0);
        if !egl::initialize(display, &mut maj, &mut min) {
            return Err("eglInitialize failed".into());
        }

        // GLES 3.0 first (proven on Wayland), then desktop GL 3.3 core.
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
            }
        };

        let api = if is_gles { egl::OPENGL_ES_API } else { egl::OPENGL_API };
        if !egl::bind_api(api as egl::EGLenum) {
            return Err("eglBindAPI failed".into());
        }
        let context = egl::create_context(display, config, egl::NO_CONTEXT, &context_attribs)
            .ok_or_else(|| format!("eglCreateContext failed (err 0x{:x})", unsafe { eglGetError() }))?;
        let surface = egl::create_window_surface(
            display,
            config,
            egl_surface_ptr as egl::EGLNativeWindowType,
            &[egl::NONE], // NB: never pass &[] — as_ptr() is a dangling 0x1
        )
        .ok_or_else(|| format!("eglCreateWindowSurface failed (err 0x{:x})", unsafe { eglGetError() }))?;
        if !egl::make_current(display, surface, surface, context) {
            return Err(format!("eglMakeCurrent failed (err 0x{:x})", unsafe { eglGetError() }));
        }
        let ctx = glow::Context::from_loader_function(|name| egl::get_proc_address(name) as *const c_void);

        let solid_prog = build_program(&ctx, is_gles, SOLID_VERT, SOLID_FRAG, &[("a_pos", 0), ("a_color", 1)])?;
        let tex_prog = build_program(&ctx, is_gles, TEX_VERT, TEX_FRAG, &[("a_pos", 0), ("a_uv", 1), ("a_color", 2)])?;

        let solid_vao = ctx.create_vertex_array()?;
        let solid_vbo = ctx.create_buffer()?;
        ctx.bind_vertex_array(Some(solid_vao));
        ctx.bind_buffer(glow::ARRAY_BUFFER, Some(solid_vbo));
        ctx.enable_vertex_attrib_array(0);
        ctx.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, 24, 0);
        ctx.enable_vertex_attrib_array(1);
        ctx.vertex_attrib_pointer_f32(1, 4, glow::FLOAT, false, 24, 8);

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

        let solid_res = ctx.get_uniform_location(solid_prog, "u_res").unwrap();
        let tex_res = ctx.get_uniform_location(tex_prog, "u_res").unwrap();
        let tex_loc = ctx.get_uniform_location(tex_prog, "u_tex").unwrap();

        unsafe {
            ctx.enable(glow::BLEND);
            ctx.blend_func(glow::ONE, glow::ONE_MINUS_SRC_ALPHA);
            ctx.disable(glow::DEPTH_TEST);
            ctx.disable(glow::CULL_FACE);
            ctx.viewport(0, 0, w, h);
        }

        Ok(Self {
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
            solid_res,
            tex_res,
            tex_loc,
            atlas_tex: None,
            atlas_size: 0,
            solid_verts: Vec::new(),
            tex_verts: Vec::new(),
        })
    }

    fn make_atlas(&mut self, size: u32) -> Result<(), String> {
        let tex = unsafe {
            let id = self.ctx.create_texture()?;
            self.ctx.bind_texture(glow::TEXTURE_2D, Some(id));
            self.ctx.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGBA8 as i32,
                size as i32,
                size as i32,
                0,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(None),
            );
            // NEAREST: glyphs are drawn 1:1 at integer positions.
            self.ctx.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::NEAREST as i32);
            self.ctx.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::NEAREST as i32);
            self.ctx.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE as i32);
            self.ctx.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE as i32);
            self.ctx.bind_texture(glow::TEXTURE_2D, None);
            id
        };
        self.atlas_tex = Some(tex);
        self.atlas_size = size;
        Ok(())
    }
}

fn push_quad(v: &mut Vec<f32>, x: f32, y: f32, w: f32, h: f32) {
    let x2 = x + w;
    let y2 = y + h;
    v.extend_from_slice(&[x, y, x2, y, x2, y2, x, y, x2, y2, x, y2]);
}

impl GpuBackend for Gl {
    fn resize(&mut self, w: i32, h: i32) {
        self.w = w;
        self.h = h;
        self.egl_window.resize(w, h, 0, 0);
        unsafe {
            self.ctx.viewport(0, 0, w, h);
        }
    }

    fn begin_frame(&mut self, bg: [f32; 4]) {
        self.solid_verts.clear();
        self.tex_verts.clear();
        unsafe {
            self.ctx.clear_color(bg[0], bg[1], bg[2], bg[3]);
            self.ctx.clear(glow::COLOR_BUFFER_BIT);
        }
    }

    fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, color: [f32; 4]) {
        if w <= 0.0 || h <= 0.0 {
            return;
        }
        push_quad(&mut self.solid_verts, x, y, w, h);
        for _ in 0..6 {
            self.solid_verts.extend_from_slice(&color);
        }
    }

    fn glyph(&mut self, uv: Slot, x: f32, y: f32, w: f32, h: f32, color: [f32; 4]) {
        if w <= 0.0 || h <= 0.0 {
            return;
        }
        let x2 = x + w;
        let y2 = y + h;
        let verts: [(f32, f32, f32, f32); 6] = [
            (x, y, uv.u0, uv.v0),
            (x2, y, uv.u1, uv.v0),
            (x2, y2, uv.u1, uv.v1),
            (x, y, uv.u0, uv.v0),
            (x2, y2, uv.u1, uv.v1),
            (x, y2, uv.u0, uv.v1),
        ];
        for (px, py, u, v) in verts {
            self.tex_verts.extend_from_slice(&[px, py, u, v, color[0], color[1], color[2], color[3]]);
        }
    }

    fn upload_atlas(&mut self, size: u32, data: &[u8]) {
        if self.atlas_tex.is_none() || self.atlas_size != size {
            if self.make_atlas(size).is_err() {
                return;
            }
        }
        let tex = self.atlas_tex.unwrap();
        unsafe {
            self.ctx.bind_texture(glow::TEXTURE_2D, Some(tex));
            self.ctx.tex_sub_image_2d(
                glow::TEXTURE_2D,
                0,
                0,
                0,
                size as i32,
                size as i32,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(Some(data)),
            );
            self.ctx.bind_texture(glow::TEXTURE_2D, None);
        }
    }

    fn present(&mut self) {
        unsafe {
            let res = [self.w as f32, self.h as f32];

            if !self.solid_verts.is_empty() {
                self.ctx.use_program(Some(self.solid_prog));
                self.ctx.uniform_2_f32(Some(&self.solid_res), res[0], res[1]);
                self.ctx.bind_vertex_array(Some(self.solid_vao));
                self.ctx.bind_buffer(glow::ARRAY_BUFFER, Some(self.solid_vbo));
                self.ctx
                    .buffer_data_u8_slice(glow::ARRAY_BUFFER, as_bytes(&self.solid_verts), glow::STREAM_DRAW);
                self.ctx.draw_arrays(glow::TRIANGLES, 0, (self.solid_verts.len() / 6) as i32);
            }

            if !self.tex_verts.is_empty() {
                self.ctx.use_program(Some(self.tex_prog));
                self.ctx.uniform_2_f32(Some(&self.tex_res), res[0], res[1]);
                self.ctx.uniform_1_i32(Some(&self.tex_loc), 0);
                self.ctx.active_texture(glow::TEXTURE0);
                if let Some(tex) = self.atlas_tex {
                    self.ctx.bind_texture(glow::TEXTURE_2D, Some(tex));
                }
                self.ctx.bind_vertex_array(Some(self.tex_vao));
                self.ctx.bind_buffer(glow::ARRAY_BUFFER, Some(self.tex_vbo));
                self.ctx
                    .buffer_data_u8_slice(glow::ARRAY_BUFFER, as_bytes(&self.tex_verts), glow::STREAM_DRAW);
                self.ctx.draw_arrays(glow::TRIANGLES, 0, (self.tex_verts.len() / 8) as i32);
                self.ctx.bind_texture(glow::TEXTURE_2D, None);
            }

            self.ctx.bind_vertex_array(None);
        }
        egl::swap_buffers(self.display, self.surface);
    }
}

impl Gl {
    /// Enable/disable vsync via the EGL swap interval (1 = vsync, 0 = off).
    pub fn set_vsync(&mut self, on: bool) {
        unsafe {
            eglSwapInterval(self.display, if on { 1 } else { 0 });
        }
    }
}

fn as_bytes(v: &[f32]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(v.as_ptr() as *const u8, v.len() * 4) }
}

impl Drop for Gl {
    fn drop(&mut self) {
        unsafe {
            if let Some(tex) = self.atlas_tex {
                self.ctx.delete_texture(tex);
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
