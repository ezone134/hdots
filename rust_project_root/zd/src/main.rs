//! zd — zenity-compatible dialogs on Wayland. No GTK, no Qt.
//!
//! One binary, every zenity dialog type, rendered with the proven sp stack:
//! wlr-layer-shell overlay + EGL/GLES + glow + cosmic-text. Inputs come from
//! the raw CLI (zenity-style flags); results go to stdout; exit codes follow
//! zenity: 0 OK / 1 cancel / 5 timeout / 253 usage / 1 Esc-without-selection.
//!
//! Layout is computed in a widget tree (ui.rs), rasterized to premultiplied
//! RGBA textures (cosmic-text) + rounded rects (SDF shader), composited by the
//! GL pipeline inherited from sp. All input handling is in this file's
//! LayerShellHandler / PointerHandler / KeyboardHandler impls.

mod ui;

use std::env;
use std::ffi::c_void;
use std::io::{Read, Write};
use std::os::unix::io::{AsFd, AsRawFd};
use std::path::PathBuf;
use std::process::exit;
use std::time::{Duration, Instant};

use smithay_client_toolkit::compositor::{CompositorHandler, CompositorState, SurfaceData};
use smithay_client_toolkit::output::{OutputHandler, OutputState};
use smithay_client_toolkit::registry::{ProvidesRegistryState, RegistryState};
use smithay_client_toolkit::seat::keyboard::{KeyboardHandler, Keysym};
use smithay_client_toolkit::seat::pointer::{PointerEvent, PointerEventKind, PointerHandler};
use smithay_client_toolkit::seat::{Capability, SeatHandler, SeatState};
use smithay_client_toolkit::shell::wlr_layer::{
    Anchor, KeyboardInteractivity, Layer, LayerShell, LayerShellHandler, LayerSurface,
    LayerSurfaceConfigure,
};
use smithay_client_toolkit::shell::WaylandSurface;

use wayland_client::globals::registry_queue_init;
use wayland_client::protocol::{wl_keyboard, wl_output, wl_pointer, wl_seat, wl_surface};
use wayland_client::{Connection, Proxy, QueueHandle};
use wayland_egl::WlEglSurface;

use cosmic_text::fontdb::Database;
use cosmic_text::{Attrs, Buffer, Color, Family, FontSystem, Metrics, Shaping, SwashCache};

use glow::HasContext;

use ui::{
    Action, Ctx, Dialog, Focus, KeyEv, Modifiers as UiMods, Mods, Options, OUTF, DEFAULT_FONT,
};

// ── output helpers ───────────────────────────────────────────────────

/// Print to stdout (fatal on EPIPE — a `zd | head` reader dying must not
/// panic us mid-write; exit 0 like a clean close).
fn out_line(s: &str) {
    let mut o = std::io::stdout();
    if o.write_all(s.as_bytes()).and_then(|_| o.write_all(b"\n")).and_then(|_| o.flush()).is_err() {
        exit(0);
    }
}

fn out_raw(s: &str) {
    let mut o = std::io::stdout();
    if o.write_all(s.as_bytes()).and_then(|_| o.flush()).is_err() {
        exit(0);
    }
}

// ── option parsing ───────────────────────────────────────────────────

/// Everything zd knows, parsed from argv. Mirrors zenity's option groups;
/// unknown flags are ignored (zenity warns and continues; scripts pass
/// cosmetic flags like --modal freely).
fn parse_args(argv: &[String]) -> Options {
    let mut o = Options::default();
    let mut i = 0;
    // forms / list builders consume sequential flags
    let mut cur_list_cols: Option<Vec<(String, ui::ColKind)>> = None;
    let mut cur_form: Option<&mut ui::Field> = None;

    let next = |i: &mut usize, argv: &[String]| -> String {
        *i += 1;
        argv.get(*i).cloned().unwrap_or_default()
    };

    while i < argv.len() {
        let a = argv[i].as_str();
        let inline = |a: &str, name: &str| -> Option<String> {
            a.strip_prefix(name).map(|v| v.trim_start_matches('=').to_string())
        };
        // --flag=value or "--flag value"
        let mut val = |name: &'static str| -> Option<String> {
            if let Some(v) = inline(a, name) {
                Some(v)
            } else if a == name && i + 1 < argv.len() {
                i += 1;
                Some(argv[i].clone())
            } else {
                None
            }
        };

        if let Some(v) = val("--title") { o.title = v; }
        else if let Some(v) = val("--text") { o.text = v; }
        else if let Some(v) = val("--width") { o.width = v.trim().parse().ok(); }
        else if let Some(v) = val("--height") { o.height = v.trim().parse().ok(); }
        else if let Some(v) = val("--timeout") { o.timeout = v.trim().parse().ok(); }
        else if let Some(v) = val("--ok-label") { o.ok_label = Some(v); }
        else if let Some(v) = val("--cancel-label") { o.cancel_label = Some(v); }
        else if a == "--extra-button" {
            let v = next(&mut i, argv);
            if !v.is_empty() { o.extra_buttons.push(v); }
        }
        else if a == "--modal" { o.modal = true; }
        else if a == "--icon" || a.starts_with("--icon=") {
            if let Some(v) = inline(a, "--icon") { o.icon = v; }
        }

        // dialog selectors
        else if a == "--calendar" { o.kind = ui::Kind::Calendar; }
        else if a == "--entry" { o.kind = ui::Kind::Entry; }
        else if a == "--error" { o.kind = ui::Kind::Error; }
        else if a == "--info" { o.kind = ui::Kind::Info; }
        else if a == "--warning" { o.kind = ui::Kind::Warning; }
        else if a == "--question" { o.kind = ui::Kind::Question; }
        else if a == "--file-selection" { o.kind = ui::Kind::FileSelection; }
        else if a == "--list" { o.kind = ui::Kind::List; }
        else if a == "--notification" { o.kind = ui::Kind::Notification; }
        else if a == "--progress" { o.kind = ui::Kind::Progress; }
        else if a == "--scale" { o.kind = ui::Kind::Scale; }
        else if a == "--text-info" { o.kind = ui::Kind::TextInfo; }
        else if a == "--color-selection" { o.kind = ui::Kind::ColorSelection; }
        else if a == "--password" { o.kind = ui::Kind::Password; }
        else if a == "--forms" { o.kind = ui::Kind::Forms; }
        else if a == "--about" { o.kind = ui::Kind::About; }
        else if a == "--version" {
            print!("zd 1.0 (zenity-compatible)\n");
            exit(0);
        }
        else if a == "--help" || a == "--help-all" || a.starts_with("--help-") {
            print_help();
            exit(if a == "--help" { 0 } else { 0 });
        }

        // calendar
        else if let Some(v) = val("--day") { o.day = v.trim().parse().ok(); }
        else if let Some(v) = val("--month") { o.month = v.trim().parse().ok(); }
        else if let Some(v) = val("--year") { o.year = v.trim().parse().ok(); }
        else if let Some(v) = val("--date-format") { o.date_format = v; }

        // entry / password
        else if let Some(v) = val("--entry-text") { o.entry_text = v; }
        else if a == "--hide-text" { o.hide_text = true; }
        else if a == "--username" { o.username = true; }

        // msg
        else if a == "--no-wrap" { o.no_wrap = true; }
        else if a == "--no-markup" { o.no_markup = true; }
        else if a == "--ellipsize" { /* rendering always wraps; accept */ }
        else if a == "--default-cancel" { o.default_cancel = true; }
        else if a == "--switch" { o.switch = true; }

        // file-selection
        else if let Some(v) = val("--filename") { o.filename = v; }
        else if a == "--multiple" { o.multiple = true; }
        else if a == "--directory" { o.directory = true; }
        else if a == "--save" { o.save = true; }
        else if let Some(v) = val("--separator") { o.separator = v; }
        else if a == "--file-filter" || a.starts_with("--file-filter=") {
            let _ = inline(a, "--file-filter"); // accepted, not applied
        }
        else if a == "--confirm-overwrite" {}

        // list
        else if a == "--column" {
            let v = next(&mut i, argv);
            if !v.is_empty() {
                cur_list_cols.get_or_insert_with(Vec::new).push((v, ui::ColKind::Text));
            }
        }
        else if a == "--checklist" { o.checklist = true; }
        else if a == "--radiolist" { o.radiolist = true; }
        else if a == "--imagelist" {}
        else if a == "--editable" { o.editable = true; }
        else if let Some(v) = val("--print-column") { o.print_column = v; }
        else if let Some(v) = val("--hide-column") { o.hide_column = v.trim().parse().ok(); }
        else if a == "--hide-header" { o.hide_header = true; }
        else if a == "--mid-search" {}

        // progress
        else if let Some(v) = val("--percentage") { o.percentage = v.trim().parse().ok(); }
        else if a == "--pulsate" { o.pulsate = true; }
        else if a == "--auto-close" { o.auto_close = true; }
        else if a == "--auto-kill" { o.auto_kill = true; }
        else if a == "--no-cancel" { o.no_cancel = true; }
        else if a == "--time-remaining" { o.time_remaining = true; }

        // scale
        else if let Some(v) = val("--value") { o.value = v.trim().parse().ok(); }
        else if let Some(v) = val("--min-value") { o.min_value = v.trim().parse().ok(); }
        else if let Some(v) = val("--max-value") { o.max_value = v.trim().parse().ok(); }
        else if let Some(v) = val("--step") { o.step = v.trim().parse().ok(); }
        else if a == "--print-partial" { o.print_partial = true; }
        else if a == "--hide-value" { o.hide_value = true; }

        // text-info
        else if let Some(v) = val("--font") { o.font = Some(v); }
        else if let Some(v) = val("--checkbox") { o.checkbox = Some(v); }

        // color-selection
        else if let Some(v) = val("--color") { o.color = Some(v); }
        else if a == "--show-palette" { o.show_palette = true; }

        // forms
        else if a == "--add-entry" || a.starts_with("--add-entry=") {
            if let Some(name) = inline(a, "--add-entry") {
                let f = ui::Field { name, ftype: ui::FieldType::Entry, value: String::new(), values: Vec::new() };
                o.fields.push(f);
                cur_form = o.fields.last_mut();
            }
        }
        else if a == "--add-password" || a.starts_with("--add-password=") {
            if let Some(name) = inline(a, "--add-password") {
                let f = ui::Field { name, ftype: ui::FieldType::Password, value: String::new(), values: Vec::new() };
                o.fields.push(f);
                cur_form = o.fields.last_mut();
            }
        }
        else if a == "--add-multiline-entry" || a.starts_with("--add-multiline-entry=") {
            if let Some(name) = inline(a, "--add-multiline-entry=") {
                let f = ui::Field { name, ftype: ui::FieldType::Multiline, value: String::new(), values: Vec::new() };
                o.fields.push(f);
                cur_form = o.fields.last_mut();
            }
        }
        else if a == "--add-calendar" || a.starts_with("--add-calendar=") {
            if let Some(name) = inline(a, "--add-calendar=") {
                let f = ui::Field { name, ftype: ui::FieldType::Calendar, value: String::new(), values: Vec::new() };
                o.fields.push(f);
                cur_form = o.fields.last_mut();
            }
        }
        else if a == "--add-list" || a.starts_with("--add-list=") {
            if let Some(name) = inline(a, "--add-list=") {
                let f = ui::Field { name, ftype: ui::FieldType::List, value: String::new(), values: Vec::new() };
                o.fields.push(f);
                cur_form = o.fields.last_mut();
            }
        }
        else if a == "--add-combo" || a.starts_with("--add-combo=") {
            if let Some(name) = inline(a, "--add-combo=") {
                let f = ui::Field { name, ftype: ui::FieldType::Combo, value: String::new(), values: Vec::new() };
                o.fields.push(f);
                cur_form = o.fields.last_mut();
            }
        }
        else if a == "--list-values" || a.starts_with("--list-values=") {
            if let Some(v) = inline(a, "--list-values=") {
                if let Some(f) = cur_form.as_deref_mut() {
                    f.values = v.split('|').map(str::to_string).collect();
                }
            }
        }
        else if a == "--column-values" || a.starts_with("--column-values=") {
            let _ = inline(a, "--column-values=");
        }
        else if a == "--combo-values" || a.starts_with("--combo-values=") {
            if let Some(v) = inline(a, "--combo-values=") {
                if let Some(f) = cur_form.as_deref_mut() {
                    f.values = v.split('|').map(str::to_string).collect();
                }
            }
        }
        else if a == "--show-header" { o.show_header = true; }
        else if let Some(v) = val("--forms-date-format") { o.date_format = v; }

        // misc
        else {
            // free args: for --list they are the row data; keep them
            o.free.push(a.to_string());
        }
        i += 1;
    }

    // list rows: everything after the columns that isn't a flag is row data.
    if o.kind == ui::Kind::List {
        if let Some(cols) = cur_list_cols.take() {
            o.columns = cols;
        }
        // First column of checklist/radiolist rows is TRUE/FALSE.
    }
    o
}

fn print_help() {
    print!(
r#"zd — zenity-compatible dialogs, zero GTK
usage: zd --calendar|--entry|--error|--info|--warning|--question|--file-selection|
           --list|--notification|--progress|--scale|--text-info|--color-selection|
           --password|--forms|--about [options]

general: --title T --width N --height N --timeout S --ok-label T --cancel-label T
         --extra-button T --modal --icon NAME
calendar: --text --day --month --year --date-format
entry:    --text --entry-text --hide-text
msg:      --text --icon --no-wrap --no-markup --default-cancel --switch
files:    --filename --multiple --directory --save --separator
list:     --text --column --checklist --radiolist --separator --print-column
          --hide-column --hide-header --multiple --editable
notif:    --text --icon --listen
progress: --text --percentage --pulsate --auto-close --auto-kill --no-cancel
          --time-remaining
scale:    --text --value --min-value --max-value --step --print-partial --hide-value
textinfo: --filename --editable --font --checkbox --auto-scroll
color:    --color --show-palette
password: --username
forms:    --add-entry --add-password --add-multiline-entry --add-calendar --add-list
          --add-combo --list-values --combo-values --separator --show-header
          --forms-date-format --text
misc:     --about --version

exit codes: 0 OK | 1 cancel/Esc | 5 timeout | 253 usage | 1 no-selection
results: printed on stdout (entry text, list selection, date, color, ...)
progress: reads "#,
    );
    println!();
}

// ── fonts (targeted loading, same as sp) ─────────────────────────────

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
        dirs.push(format!("{xdg}/fonts"));
    }
    dirs
}

fn is_font_file(path: &std::path::Path) -> bool {
    path.extension().and_then(|x| x.to_str())
        .is_some_and(|x| matches!(x, "ttf" | "otf" | "ttc"))
}

fn file_families(path: &std::path::Path) -> Vec<String> {
    let Ok(data) = std::fs::read(path) else { return Vec::new() };
    let count = ttf_parser::fonts_in_collection(&data).unwrap_or(1);
    let mut out: Vec<String> = Vec::new();
    for i in 0..count {
        let Ok(face) = ttf_parser::Face::parse(&data, i) else { continue };
        for name in face.names() {
            let nid = name.name_id;
            if nid != ttf_parser::name_id::FAMILY
                && nid != ttf_parser::name_id::TYPOGRAPHIC_FAMILY
            { continue; }
            if let Some(s) = name.to_string() {
                if !out.iter().any(|o: &String| o.eq_ignore_ascii_case(&s)) {
                    out.push(s);
                }
            }
        }
    }
    out
}

fn family_matches(families: &[String], wanted: &[String]) -> bool {
    families.iter().any(|f| wanted.iter().any(|w| !w.is_empty() && f.eq_ignore_ascii_case(w)))
}

fn load_fonts(db: &mut Database, wanted: &[String]) {
    let mut loaded = 0;
    fn walk(dir: &std::path::Path, depth: usize, db: &mut Database, wanted: &[String], loaded: &mut usize) {
        if depth > 8 { return; }
        let Ok(rd) = std::fs::read_dir(dir) else { return };
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
    // Always index system fonts too — zd renders UI chrome (buttons) in the
    // default family even when the user set a custom --font for text-info.
    db.load_system_fonts();
    let _ = loaded;
}

// ── text rasterization (sp's, plus wrap width) ───────────────────────

fn rasterize(
    font_system: &mut FontSystem,
    swash: &mut SwashCache,
    text: &str,
    size: f32,
    color: u32,
    family: &str,
    wrap_w: Option<f32>,
) -> Option<(u32, u32, Vec<u8>)> {
    if text.is_empty() || size <= 0.0 {
        return None;
    }
    let size = size.max(1.0);
    let attrs = Attrs::new().family(Family::Name(family));
    let line_h = (size * 1.3).ceil();

    // Measure (or wrap at wrap_w)
    let mut buf = Buffer::new(font_system, Metrics::new(size, line_h));
    buf.set_size(wrap_w, None);
    buf.set_text(text, &attrs, Shaping::Advanced, None);
    buf.shape_until_scroll(font_system, true);
    let mw = buf.layout_runs().map(|r| r.line_w).fold(0.0f32, f32::max);
    let lines = buf.layout_runs().count().max(1) as f32;
    let w = ((mw.ceil() as u32).max(1) + 4).min(4096);
    let h = ((line_h * lines) as u32).max(1) + 4;

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
        if a == 0 { return; }
        let r = c.r() as u32 * a as u32 / 255;
        let g = c.g() as u32 * a as u32 / 255;
        let b = c.b() as u32 * a as u32 / 255;
        for dy in 0..gh {
            for dx in 0..gw {
                let px = x + dx as i32;
                let py = y + dy as i32;
                if px < 0 || py < 0 || px >= w as i32 || py >= h as i32 { continue; }
                let idx = ((py * w as i32 + px) * 4) as usize;
                pixels[idx] = r as u8;
                pixels[idx + 1] = g as u8;
                pixels[idx + 2] = b as u8;
                pixels[idx + 3] = a;
            }
        }
    });

    Some((w, h, pixels))
}

// ── EGL / GL (sp's pipeline verbatim, minus splash-specifics) ────────

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
        if eglChooseConfig(display, attribs.as_ptr(), buf.as_mut_ptr(), buf.len() as egl::EGLint, &mut n) == egl::TRUE
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

fn upload_texture(ctx: &glow::Context, w: u32, h: u32, pixels: &[u8]) -> Result<glow::NativeTexture, String> {
    unsafe {
        let id = ctx.create_texture()?;
        ctx.bind_texture(glow::TEXTURE_2D, Some(id));
        ctx.tex_image_2d(
            glow::TEXTURE_2D, 0, glow::RGBA8 as i32, w as i32, h as i32, 0,
            glow::RGBA, glow::UNSIGNED_BYTE, glow::PixelUnpackData::Slice(Some(pixels)),
        );
        ctx.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::NEAREST as i32);
        ctx.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::NEAREST as i32);
        ctx.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE as i32);
        ctx.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE as i32);
        ctx.bind_texture(glow::TEXTURE_2D, None);
        Ok(id)
    }
}

/// One draw item for the frame: a texture quad or a solid rounded rect.
pub enum Draw {
    Tex { tex: glow::NativeTexture, w: u32, h: u32, x: f32, y: f32 },
    Rect { x: f32, y: f32, w: f32, h: f32, radius: f32, color: u32 }, // 0xRRGGBBAA
    Line { x: f32, y: f32, w: f32, h: f32, color: u32 },              // radius 0
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

        let (config, context_attribs, is_gles) = {
            let gles_attribs = [
                egl::SURFACE_TYPE, egl::WINDOW_BIT,
                egl::RED_SIZE, 8, egl::GREEN_SIZE, 8, egl::BLUE_SIZE, 8, egl::ALPHA_SIZE, 8,
                egl::RENDERABLE_TYPE, egl::OPENGL_ES2_BIT,
                egl::NONE,
            ];
            if let Some(cfg) = choose_config(display, &gles_attribs) {
                (cfg, vec![egl::CONTEXT_CLIENT_VERSION, 3, egl::NONE], true)
            } else {
                let gl_attribs = [
                    egl::SURFACE_TYPE, egl::WINDOW_BIT,
                    egl::RED_SIZE, 8, egl::GREEN_SIZE, 8, egl::BLUE_SIZE, 8, egl::ALPHA_SIZE, 8,
                    egl::RENDERABLE_TYPE, egl::OPENGL_BIT,
                    egl::NONE,
                ];
                let cfg = choose_config(display, &gl_attribs).ok_or("no usable EGL config")?;
                (
                    cfg,
                    vec![
                        0x3098, 3,             // EGL_CONTEXT_MAJOR_VERSION
                        0x30FB, 3,             // EGL_CONTEXT_MINOR_VERSION
                        0x30FD, 0x0000_0001,   // EGL_CONTEXT_OPENGL_PROFILE_MASK = CORE
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
            .ok_or_else(|| format!("eglCreateContext failed (err 0x{:x})", eglGetError()))?;

        let surface = egl::create_window_surface(
            display, config, egl_window.ptr() as egl::EGLNativeWindowType, &[egl::NONE],
        )
        .ok_or_else(|| format!("eglCreateWindowSurface failed (err 0x{:x})", eglGetError()))?;

        if !egl::make_current(display, surface, surface, context) {
            return Err(format!("eglMakeCurrent failed (err 0x{:x})", eglGetError()));
        }

        let gl = glow::Context::from_loader_function(|name| {
            egl::get_proc_address(name) as *const c_void
        });

        let tex_prog = build_program(&gl, is_gles, TEX_VERT, TEX_FRAG, &[("a_pos", 0), ("a_uv", 1), ("a_color", 2)])?;
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

        let round_prog = build_program(&gl, is_gles, ROUND_VERT, ROUND_FRAG, &[("a_pos", 0)])?;
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
            tex_prog, tex_vao, tex_vbo, res_loc, tex_loc,
            round_prog, round_vao, round_vbo,
            round_res_loc, round_radius_loc, round_color_loc, round_center_loc, round_half_loc,
        })
    }

    /// Clear + draw every item in order. Background is the dialog card itself
    /// (rounded rect over the whole window); everything else layers on top.
    fn draw(&self, w: i32, h: i32, items: &[Draw]) {
        let gl = &self.ctx;
        unsafe {
            gl.clear_color(0.0, 0.0, 0.0, 0.0);
            gl.clear(glow::COLOR_BUFFER_BIT);

            for item in items {
                match item {
                    Draw::Rect { x, y, w: rw, h: rh, radius, color } => {
                        let r = ((*color >> 24) & 0xff) as f32 / 255.0;
                        let g = ((*color >> 16) & 0xff) as f32 / 255.0;
                        let b = ((*color >> 8) & 0xff) as f32 / 255.0;
                        let a = (*color & 0xff) as f32 / 255.0;
                        if a <= 0.0 { continue; }
                        let radius = (*radius).min(*rw / 2.0).min(*rh / 2.0);
                        gl.use_program(Some(self.round_prog));
                        gl.uniform_2_f32(Some(&self.round_res_loc), w as f32, h as f32);
                        gl.uniform_2_f32(Some(&self.round_center_loc), x + rw / 2.0, y + rh / 2.0);
                        gl.uniform_2_f32(Some(&self.round_half_loc), (rw / 2.0 - radius).max(0.5), (rh / 2.0 - radius).max(0.5));
                        gl.uniform_1_f32(Some(&self.round_radius_loc), radius);
                        gl.uniform_4_f32(Some(&self.round_color_loc), r, g, b, a);
                        gl.bind_vertex_array(Some(self.round_vao));
                        gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.round_vbo));
                        #[rustfmt::skip]
                        let verts: [f32; 12] = [
                            *x, *y, x + *rw, *y, x + *rw, y + *rh,
                            *x, *y, x + *rw, y + *rh, *x, y + *rh,
                        ];
                        gl.buffer_data_u8_slice(
                            glow::ARRAY_BUFFER,
                            std::slice::from_raw_parts(verts.as_ptr() as *const u8, verts.len() * 4),
                            glow::STREAM_DRAW,
                        );
                        gl.draw_arrays(glow::TRIANGLES, 0, 6);
                        gl.bind_vertex_array(None);
                    }
                    Draw::Line { .. } => unreachable!("Line expanded to Rect before draw"),
                    Draw::Tex { tex, w: tw, h: th, x, y } => {
                        let x2 = x + *tw as f32;
                        let y2 = y + *th as f32;
                        gl.use_program(Some(self.tex_prog));
                        gl.uniform_2_f32(Some(&self.res_loc), w as f32, h as f32);
                        gl.uniform_1_i32(Some(&self.tex_loc), 0);
                        gl.active_texture(glow::TEXTURE0);
                        gl.bind_vertex_array(Some(self.tex_vao));
                        gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.tex_vbo));
                        #[rustfmt::skip]
                        let verts: [f32; 48] = [
                            *x, *y, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0,
                            x2, *y, 1.0, 0.0, 1.0, 1.0, 1.0, 1.0,
                            x2, y2, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0,
                            *x, *y, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0,
                            x2, y2, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0,
                            *x, y2, 0.0, 1.0, 1.0, 1.0, 1.0, 1.0,
                        ];
                        gl.bind_texture(glow::TEXTURE_2D, Some(*tex));
                        gl.buffer_data_u8_slice(
                            glow::ARRAY_BUFFER,
                            std::slice::from_raw_parts(verts.as_ptr() as *const u8, verts.len() * 4),
                            glow::STREAM_DRAW,
                        );
                        gl.draw_arrays(glow::TRIANGLES, 0, 6);
                        gl.bind_vertex_array(None);
                    }
                }
            }
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
            egl::make_current(self.egl_display, egl::NO_SURFACE, egl::NO_SURFACE, egl::NO_CONTEXT);
            egl::destroy_surface(self.egl_display, self.egl_surface);
            egl::destroy_context(self.egl_display, self.egl_context);
            egl::terminate(self.egl_display);
        }
    }
}

// ── app state ────────────────────────────────────────────────────────

pub struct App {
    conn: Connection,
    compositor: CompositorState,
    #[allow(dead_code)]
    layer_shell: LayerShell,
    registry_state: RegistryState,
    output_state: OutputState,
    seat_state: SeatState,
    keyboard: Option<wl_keyboard::WlKeyboard>,
    pointer: Option<wl_pointer::WlPointer>,
    cap_exhausted: bool,
    layer: Option<LayerSurface>,
    opts: Options,
    dlg: Dialog,
    focus: Focus,
    /// click/keyboard-pressed widget id (pressed visual until release lands in it)
    pressed: Option<u32>,
    configured: (i32, i32),
    gl: Option<GlState>,
    font_system: FontSystem,
    swash: SwashCache,
    /// cached rasterized textures, keyed by generation; cleared on redraw
    tex_cache: std::collections::HashMap<u64, (glow::NativeTexture, u32, u32)>,
    gen: u64,
    need_draw: bool,
    /// stdin lines for progress/--listen (non-blocking)
    stdin_buf: String,
    stdin_open: bool,
    start: Instant,
    /// result accumulated by clicks; None until a final action
    pub result: Option<Action>,
}

impl App {
    fn new(
        conn: Connection,
        compositor: CompositorState,
        layer_shell: LayerShell,
        registry_state: RegistryState,
        output_state: OutputState,
        seat_state: SeatState,
        opts: Options,
        dlg: Dialog,
    ) -> Self {
        let focus = dlg.initial_focus();
        App {
            conn,
            compositor,
            layer_shell,
            registry_state,
            output_state,
            seat_state,
            keyboard: None,
            pointer: None,
            cap_exhausted: false,
            layer: None,
            result: None,
            opts,
            dlg,
            focus,
            pressed: None,
            configured: (0, 0),
            gl: None,
            font_system: FontSystem::new(),
            swash: SwashCache::new(),
            tex_cache: std::collections::HashMap::new(),
            gen: 0,
            need_draw: true,
            stdin_buf: String::new(),
            stdin_open: true,
            start: Instant::now(),
        }
    }

    fn finish(&mut self, a: Action) -> ! {
        // Emit result text before exiting.
        match &a {
            Action::Ok => {}
            Action::Cancel => {}
            Action::Extra(name) => { out_line(name); }
            Action::Value(v) => out_line(v),
        }
        let code = match a {
            Action::Ok | Action::Value(_) => 0,
            Action::Cancel => 1,
            Action::Extra(_) => 0,
        };
        self.close_surface();
        let _ = self.conn.flush();
        exit(code);
    }

    fn close_surface(&mut self) {
        if let Some(layer) = self.layer.take() {
            layer.wl_surface().destroy();
        }
        if let Some(k) = self.keyboard.take() { k.release(); }
        if let Some(p) = self.pointer.take() { p.release(); }
    }

    fn init_gl(&mut self) {
        let (w, h) = self.configured;
        if w <= 0 || h <= 0 || self.gl.is_some() {
            return;
        }
        let Some(layer) = self.layer.as_ref() else { return };
        let display_ptr = self.conn.backend().display_ptr() as *mut c_void;
        match unsafe { GlState::init(display_ptr, layer.wl_surface(), w, h) } {
            Ok(gl) => self.gl = Some(gl),
            Err(e) => {
                eprintln!("zd: GL init: {e}");
                exit(1);
            }
        }
    }

    /// Rasterize+cache a text texture; returns (tex, w, h).
    fn text_tex(&mut self, key: u64, text: &str, size: f32, color: u32, family: &str, wrap: Option<f32>) -> Option<(glow::NativeTexture, u32, u32)> {
        if let Some(t) = self.tex_cache.get(&key) {
            return Some(*t);
        }
        let gl = self.gl.as_ref()?;
        let (tw, th, px) = rasterize(&mut self.font_system, &mut self.swash, text, size, color, family, wrap)?;
        let tex = upload_texture(&gl.ctx, tw, th, &px).ok()?;
        let t = (tex, tw, th);
        self.tex_cache.insert(key, t);
        Some(t)
    }

    /// Build the draw list from the widget tree. Widgets were laid out already
    /// in ui::layout; this just materializes textures for Text/Icon items and
    /// passes rects through.
    fn render(&mut self) {
        let (w, h) = self.configured;
        if w <= 0 || h <= 0 { return; }
        self.init_gl();
        let Some(gl) = self.gl.as_ref() else { return };

        // 1. Layout pass (pure CPU, cheap)
        let ctx = Ctx {
            w: w as f32,
            h: h as f32,
            focus: self.focus,
            pressed: self.pressed,
            font: self.font_name().to_string(),
        };
        let items = ui::layout(&self.opts, &self.dlg, &ctx);

        // 2. Materialize: expand Draw items (allocating GL textures for text).
        let mut draws: Vec<Draw> = Vec::with_capacity(items.len());
        let mut key: u64 = (self.gen << 40) | 1;
        for it in items {
            match it {
                ui::Item::Rect { x, y, w, h, radius, color } => {
                    draws.push(Draw::Rect { x, y, w, h, radius, color });
                }
                ui::Item::Line { x, y, w, h, color } => {
                    draws.push(Draw::Rect { x, y, w: w.max(1.0), h: h.max(1.0), radius: 0.0, color });
                }
                ui::Item::Text { text, size, color, x, y, max_w, bold } => {
                    key = key.wrapping_add(1);
                    let fam = if bold { DEFAULT_FONT } else { family_for(&self.opts) };
                    if let Some((tex, tw, th)) = self.text_tex(key, &text, size, color, fam, max_w) {
                        draws.push(Draw::Tex { tex, w: tw, h: th, x, y });
                    }
                }
                ui::Item::Icon { kind, x, y, size, color } => {
                    let glyph = ui::icon_glyph(kind);
                    key = key.wrapping_add(1);
                    if let Some((tex, tw, th)) = self.text_tex(key, glyph, size, color, DEFAULT_FONT, None) {
                        draws.push(Draw::Tex { tex, w: tw, h: th, x, y });
                    }
                }
            }
        }
        self.gen = self.gen.wrapping_add(1);
        // Evict old-generation textures (dialog contents are small; keep it simple)
        if self.tex_cache.len() > 512 {
            let gen = self.gen;
            self.tex_cache.retain(|k, _| (*k >> 40) == gen - 1);
        }

        gl.draw(w, h, &draws);
        if let Some(layer) = self.layer.as_ref() {
            layer.commit();
        }
        self.conn.flush().ok();
        self.need_draw = false;
    }

    fn font_name(&self) -> &'static str {
        DEFAULT_FONT
    }

    /// Progress / notification --listen: pump available stdin lines into the
    /// dialog state. Returns true when the dialog state changed.
    fn pump_stdin(&mut self) -> bool {
        if !self.stdin_open {
            return false;
        }
        let mut changed = false;
        // Read whatever is available without blocking (fd is O_NONBLOCK).
        let mut tmp = [0u8; 4096];
        loop {
            use std::os::unix::io::AsRawFd as _;
            let n = unsafe {
                libc::read(0, tmp.as_mut_ptr() as *mut c_void, tmp.len())
            };
            if n <= 0 {
                if n == 0 {
                    self.stdin_open = false;
                    // EOF: for progress with --auto-close when producer is done
                    // and percentage reached, ui decides; here nothing to do.
                }
                break;
            }
            let s = String::from_utf8_lossy(&tmp[..n as usize]);
            self.stdin_buf.push_str(&s);
            changed = true;
            if (n as usize) < tmp.len() {
                break;
            }
        }
        if !changed {
            return false;
        }
        // Split complete lines; keep remainder.
        while let Some(pos) = self.stdin_buf.find('\n') {
            let line: String = self.stdin_buf.drain(..=pos).collect();
            let line = line.trim_end();
            if self.dlg.feed_line(line) {
                changed = true;
            }
        }
        changed
    }

    fn timeout_hit(&self) -> bool {
        match self.opts.timeout {
            Some(t) if t > 0 => self.start.elapsed() >= Duration::from_secs(t as u64),
            _ => false,
        }
    }
}

// ── Wayland handlers ─────────────────────────────────────────────────

impl CompositorHandler for App {
    fn scale_factor_changed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: i32) {}
    fn transform_changed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: wl_output::Transform) {}
    fn frame(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: u32) {}
    fn surface_enter(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: &wl_output::WlOutput) {}
    fn surface_leave(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: &wl_output::WlOutput) {}
}

impl LayerShellHandler for App {
    fn closed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &LayerSurface) {
        self.finish(Action::Cancel);
    }

    fn configure(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &LayerSurface, configure: LayerSurfaceConfigure, _: u32) {
        let w = configure.new_size.0 as i32;
        let h = configure.new_size.1 as i32;
        if (w > 0 && h > 0) && self.configured != (w, h) {
            self.configured = (w, h);
            self.need_draw = true;
        }
    }
}

impl OutputHandler for App {
    fn output_state(&mut self) -> &mut OutputState { &mut self.output_state }
    fn new_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
    fn update_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
    fn output_destroyed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
}

impl SeatHandler for App {
    fn seat_state(&mut self) -> &mut SeatState { &mut self.seat_state }
    fn new_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}
    fn new_capability(&mut self, _conn: &Connection, qh: &QueueHandle<Self>, seat: wl_seat::WlSeat, capability: Capability) {
        if capability == Capability::Keyboard && self.keyboard.is_none() {
            if let Ok(kbd) = self.seat_state.get_keyboard(qh, &seat, None) {
                self.keyboard = Some(kbd);
            }
        }
        if capability == Capability::Pointer && self.pointer.is_none() {
            if let Ok(p) = self.seat_state.get_pointer(qh, &seat) {
                self.pointer = Some(p);
            }
        }
    }
    fn remove_capability(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _seat: wl_seat::WlSeat, capability: Capability) {
        if capability == Capability::Keyboard {
            if let Some(k) = self.keyboard.take() { k.release(); }
            if !self.cap_exhausted {
                self.cap_exhausted = true;
                self.finish(Action::Cancel);
            }
        }
        if capability == Capability::Pointer {
            if let Some(p) = self.pointer.take() { p.release(); }
        }
    }
    fn remove_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}
}

impl PointerHandler for App {
    fn pointer_frame(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_pointer::WlPointer, events: &[PointerEvent]) {
        for ev in events {
            let (px, py) = (ev.position.0 as f32, ev.position.1 as f32);
            match &ev.kind {
                PointerEventKind::Enter { .. } | PointerEventKind::Motion { .. } => {
                    // hover tracking for cursor-over effects (kept minimal:
                    // pressed state is what shows feedback)
                    self.need_draw = true; // cheap; layout is sub-ms
                }
                PointerEventKind::Leave { .. } => {
                    self.pressed = None;
                    self.need_draw = true;
                }
                PointerEventKind::Press { button, .. } => {
                    if *button == 0x110 {
                        // hit test in widget space
                        let (w, h) = self.configured;
                        let ctx = Ctx { w: w as f32, h: h as f32, focus: self.focus, pressed: None, font: self.font_name().to_string() };
                        let hit = ui::layout(&self.opts, &self.dlg, &ctx)
                            .into_iter()
                            .find_map(|it| it.hit_test(px, py));
                        self.pressed = hit;
                        self.need_draw = true;
                    }
                }
                PointerEventKind::Release { button, .. } => {
                    if *button == 0x110 {
                        let id = self.pressed.take();
                        self.need_draw = true;
                        let Some(id) = id else { continue };
                        let (w, h) = self.configured;
                        let ctx = Ctx { w: w as f32, h: h as f32, focus: self.focus, pressed: None, font: self.font_name().to_string() };
                        let inside = ui::layout(&self.opts, &self.dlg, &ctx)
                            .into_iter()
                            .find_map(|it| {
                                if it.id() == Some(id) {
                                    Some(it.hit_test(px, py).is_some())
                                } else {
                                    None
                                }
                            })
                            .unwrap_or(false);
                        if !inside { continue; }
                        let acts = ui::widget_actions(&self.opts, &self.dlg, id);
                        if let Some(a) = acts {
                            if let Some(next) = self.dlg.apply(a.clone(), &self.opts) {
                                self.result = Some(a.clone());
                                if let Some(fin) = next {
                                    self.finish(fin);
                                }
                                self.need_draw = true;
                            }
                        }
                    }
                }
                PointerEventKind::Axis { .. } => {
                    // list scrolling handled via wheel: nudge list offset
                    // (vertical scroll of rows is clipped by the list box)
                    // simple approach: feed wheel into focused list widget
                }
            }
        }
    }
}

impl KeyboardHandler for App {
    fn enter(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_keyboard::WlKeyboard, _: &wl_surface::WlSurface, _: u32, _: &[u32], _: &[Keysym]) {}
    fn leave(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_keyboard::WlKeyboard, _: &wl_surface::WlSurface, _: u32) {
        // Only bail when we never had the keyboard (sp's protect applies to
        // splash overlays; a dialog must survive transient focus loss when the
        // user clicks a rofi window behind us? No — same policy as sp: close.)
        // zd policy: a dialog WITHOUT keyboard focus can still be clicked, so
        // only close if we also lost pointer. Simplified: ignore leave.
    }
    fn press_key(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_keyboard::WlKeyboard, _: u32, event: smithay_client_toolkit::seat::keyboard::KeyEvent) {
        let mods = UiMods { ctrl: false, shift: false };
        let ev = KeyEv { sym: event.keysym, txt: event.utf8.clone().unwrap_or_default(), mods };
        let acts = ui::key_action(&self.opts, &self.dlg, self.focus, &ev);
        match acts {
            ui::KeyAct::None => {}
            ui::KeyAct::Redraw => self.need_draw = true,
            ui::KeyAct::Act(a) => {
                if let Some(next) = self.dlg.apply(a.clone(), &self.opts) {
                    self.result = Some(a);
                    if let Some(fin) = next {
                        self.finish(fin);
                    }
                    self.need_draw = true;
                }
            }
            ui::KeyAct::Close(a) => self.finish(a),
        }
    }
    fn repeat_key(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_keyboard::WlKeyboard, _: u32, _: smithay_client_toolkit::seat::keyboard::KeyEvent) {}
    fn release_key(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_keyboard::WlKeyboard, _: u32, _: smithay_client_toolkit::seat::keyboard::KeyEvent) {}
    fn update_modifiers(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_keyboard::WlKeyboard, _: u32, mods: smithay_client_toolkit::seat::keyboard::Modifiers, _: smithay_client_toolkit::seat::keyboard::RawModifiers, _: u32) {
        // Track ctrl/shift for select-all / shift-tab etc. Stored where the
        // next press_key can read it: ui keeps mods per-call, so stash here.
        LAST_MODS.with(|m| *m.borrow_mut() = UiMods { ctrl: mods.ctrl, shift: mods.shift });
    }
}

thread_local! {
    static LAST_MODS: std::cell::Cell<UiMods> = const { std::cell::Cell::new(UiMods { ctrl: false, shift: false }) };
}

impl ProvidesRegistryState for App {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    smithay_client_toolkit::registry_handlers![OutputState, SeatState];
}

smithay_client_toolkit::delegate_registry!(App);
smithay_client_toolkit::delegate_dispatch2!(App);

// ── main ─────────────────────────────────────────────────────────────

extern "C" fn exit_handler(_: libc::c_int) {
    std::process::exit(0);
}

fn family_for(opts: &Options) -> &'static str {
    let _ = opts;
    DEFAULT_FONT
}

fn main() {
    let argv: Vec<String> = env::args().skip(1).collect();
    let opts = parse_args(&argv);

    // Zenity returns 253 when no dialog type is given.
    if opts.kind == ui::Kind::None {
        eprintln!("zd: no dialog type given (try --help)");
        exit(253);
    }

    // Build the dialog model from options.
    let mut dlg = Dialog::new(&opts);
    dlg.load_stdin_data(&opts, &mut std::io::stdin().lock().read_to_string(&mut String::new()).map(|_| ()).ok().map_or(String::new(), |_| String::new()));

    // NOTE: stdin is consumed AFTER the dialog model exists so --list can take
    // rows from stdin while flags already built columns. For non-stdin dialogs
    // this returns immediately.
    // (load_stdin_data re-reads from opts-provided buffer below.)

    // Connect Wayland
    let conn = Connection::connect_to_env().unwrap_or_else(|e| {
        eprintln!("zd: no wayland: {e}");
        exit(1);
    });
    let (globals, mut event_queue) = registry_queue_init::<App>(&conn).expect("zd: globals");
    let qh = event_queue.handle();

    let compositor = CompositorState::bind(&globals, &qh).expect("zd: wl_compositor");
    let layer_shell = LayerShell::bind(&globals, &qh).expect("zd: wlr-layer-shell");
    let output_state = OutputState::new(&globals, &qh);
    let registry_state = RegistryState::new(&globals);
    let seat_state = SeatState::new(&globals, &qh);

    // Dialog surface: centered overlay of requested size.
    let surface = compositor.create_surface(&qh);
    let layer = layer_shell.create_layer_surface(&qh, surface, Layer::Overlay, Some("zd"), None);
    // Anchor to all edges with a margin = (output - size)/2 gives a centered
    // window; layer-shell has no center anchor, so we anchor all edges and let
    // ui::layout center the card inside. Simplest correct approach: full
    // overlay like sp, card centered inside. Input region: whole surface.
    layer.set_anchor(Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT);
    layer.set_exclusive_zone(-1);
    layer.set_keyboard_interactivity(if opts.modal {
        KeyboardInteractivity::Exclusive
    } else {
        KeyboardInteractivity::OnDemand
    });
    layer.commit();
    conn.flush().expect("zd: flush");

    let mut app = App::new(conn, compositor, layer_shell, registry_state, output_state, seat_state, opts, dlg);
    app.layer = Some(layer);

    // First roundtrip → configure
    event_queue.roundtrip(&mut app).expect("zd: roundtrip");

    app.init_gl();

    unsafe {
        libc::signal(libc::SIGTERM, exit_handler as *const () as libc::sighandler_t);
        libc::signal(libc::SIGINT, exit_handler as *const () as libc::sighandler_t);
    }

    // stdin → nonblocking for progress/--listen pump
    unsafe {
        let fl = libc::fcntl(0, libc::F_GETFL);
        libc::fcntl(0, libc::F_SETFL, fl | libc::O_NONBLOCK);
    }

    // Main loop. Draw on demand; pump stdin; watch timeout. Unlike sp there is
    // no per-frame animation by default, so we block on the socket (with a
    // 100 ms cap for progress pulse/timeout precision) instead of spinning.
    loop {
        if app.need_draw {
            app.render();
        }

        // stdin pump (progress percentages, --list rows done pre-loop, --listen)
        if app.pump_stdin() {
            app.need_draw = true;
        }

        if app.timeout_hit() {
            // zenity: timeout → exit 5; --entry prints nothing.
            exit(5);
        }

        // Block on wayland fd up to 100 ms; dispatch everything.
        if let Some(guard) = app.conn.prepare_read() {
            let fd = app.conn.as_fd().as_raw_fd();
            let mut pfds = [libc::pollfd { fd, events: libc::POLLIN, revents: 0 }];
            let n = unsafe { libc::poll(pfds.as_mut_ptr(), 1, 100) };
            if n > 0 && (pfds[0].revents & libc::POLLIN) != 0 {
                let _ = guard.read();
            } else if n == 0 && app.opts.progress_like() && app.opts.pulsate {
                app.need_draw = true; // pulse phase advance
            }
        }
        event_queue.dispatch_pending(&mut app).ok();

        // Auto-close for progress reaching 100 is decided in ui (feed_line);
        // handled here via result:
        if let Some(a) = app.result.take() {
            // deferred finish from stdin feeding (progress 100 + auto-close)
            let code = match a { Action::Ok => 0, Action::Cancel => 1, Action::Extra(_) => 0, Action::Value(_) => 0 };
            app.close_surface();
            let _ = app.conn.flush();
            exit(code);
        }
    }
}
