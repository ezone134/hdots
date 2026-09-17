//! Text layout & rasterization via cosmic-text (pure Rust, no Pango/Cairo).
//!
//! Output is premultiplied RGBA8 — ready for GL upload with
//! `GL_ONE, GL_ONE_MINUS_SRC_ALPHA` blending.
//!
//! Font loading is *targeted*: only the configured families are loaded into
//! fontdb (the RAM roadmap item) — no `load_system_fonts()` blast radius.

use cosmic_text::fontdb::{Database, Weight};
use cosmic_text::{Attrs, Buffer, Color, Family, FontSystem, Metrics, Shaping, SwashCache};

/// Which loaded family a piece of text renders in. The *minimal* type stack:
///
/// - `Ui`      — the configured UI family (default Inter), Light weight body
/// - `Display` — the display family (default Inter Display), for hero numerals
/// - `Mono`    — the mono family (default Geist Mono), for technical tabular values
/// - `Brand`   — the branding family (`$states2/branding_font`, default
///   opensuse), for the brand glyph on the Branding card / banner chip / pill
#[derive(Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Ff {
    #[default]
    Ui,
    Display,
    Mono,
    Brand,
}

impl Ff {
    pub fn as_u8(self) -> u8 {
        match self {
            Ff::Ui => 0,
            Ff::Display => 1,
            Ff::Mono => 2,
            Ff::Brand => 3,
        }
    }
}

/// Typographic weight stack mapped to font weights at rasterize time. Kept as
/// a tiny enum so `Cmd::Text` and the texture cache key stay `Copy/Eq/Hash`.
/// The default is Light — the quiet, minimal body weight.
#[derive(Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Fw {
    #[default]
    Light,
    /// Ultrathin — reserved for ultra-secondary labels (part of the stack API).
    #[allow(dead_code)]
    Thin,
    Regular,
    Medium,
    Semibold,
}

impl Fw {
    pub fn weight(self) -> Weight {
        match self {
            Fw::Thin => Weight::THIN,
            Fw::Light => Weight::LIGHT,
            Fw::Regular => Weight::NORMAL,
            Fw::Medium => Weight::MEDIUM,
            Fw::Semibold => Weight::SEMIBOLD,
        }
    }
    pub fn as_u8(self) -> u8 {
        match self {
            Fw::Thin => 0,
            Fw::Light => 1,
            Fw::Regular => 2,
            Fw::Medium => 3,
            Fw::Semibold => 4,
        }
    }
}

pub struct TextEngine {
    pub font_system: FontSystem,
    swash: SwashCache,
    family: String,
    display_family: String,
    mono_family: String,
    brand_family: String,
    icon_family: String,
    /// Draw-time icon remap: canonical `ICON_*` codepoint → the active icon
    /// font's slot. Built by `crate::icons::slot_map` from the per-font conf.
    icon_slots: std::collections::HashMap<char, char>,
}

/// Standard font dirs, most-relevant first. Scanned for the configured
/// families; everything else stays out of RAM.
fn font_dirs() -> Vec<std::path::PathBuf> {
    let mut dirs: Vec<std::path::PathBuf> = vec![
        "/usr/share/fonts".into(),
        "/usr/local/share/fonts".into(),
        "/run/host/fonts".into(), // nix/container hosts
    ];
    if let Ok(home) = std::env::var("HOME") {
        dirs.push(format!("{home}/.local/share/fonts").into());
        dirs.push(format!("{home}/.fonts").into());
    }
    if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
        dirs.push(format!("{xdg}/fonts").into());
    }
    dirs
}

/// Peek a font file's family names with ttf-parser (cheap, no fontdb load).
fn file_families(path: &std::path::Path) -> Vec<String> {
    let Ok(data) = std::fs::read(path) else { return Vec::new() };
    let count = ttf_parser::fonts_in_collection(&data).unwrap_or(1);
    let mut out: Vec<String> = Vec::new();
    for i in 0..count {
        let Ok(face) = ttf_parser::Face::parse(&data, i) else { continue };
        for name in face.names() {
            let nid = name.name_id;
            if nid != ttf_parser::name_id::FAMILY && nid != ttf_parser::name_id::TYPOGRAPHIC_FAMILY {
                continue;
            }
            if let Some(s) = name.to_string() {
                if !out.iter().any(|o| o.eq_ignore_ascii_case(s.as_str())) {
                    out.push(s);
                }
            }
        }
    }
    out
}

/// Does the installed family `f` satisfy the requested family `w`? Exact
/// names match; a generic icon request like "Material Symbols" also matches
/// its installed variants ("…Rounded" / "…Outlined" / "…Sharp"). A LONGER
/// requested name is never allowed to prefix-match (so "Material Symbols
/// Rounded" won't silently grab plain "Material").
fn family_satisfies(f: &str, w: &str) -> bool {
    if w.is_empty() {
        return false;
    }
    if f.eq_ignore_ascii_case(w) {
        return true;
    }
    // generic request (no trailing variant word) → accept any longer variant
    let wl = w.split_whitespace().count();
    let fl = f.split_whitespace().count();
    fl > wl
        && w.split_whitespace()
            .zip(f.split_whitespace())
            .all(|(a, b)| a.eq_ignore_ascii_case(b))
}

fn load_font_dir(db: &mut Database, dir: &std::path::Path, wanted: &[String], loaded: &mut usize, depth: usize) {
    if depth > 4 {
        return;
    }
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for entry in rd.flatten() {
        let path = entry.path();
        if path.is_dir() {
            load_font_dir(db, &path, wanted, loaded, depth + 1);
        } else {
            let ext = path.extension().and_then(|x| x.to_str());
            if ext.is_some_and(|x| x == "ttf" || x == "otf" || x == "ttc")
                && file_families(&path)
                    .iter()
                    .any(|f| wanted.iter().any(|w| family_satisfies(f, w)))
                && db.load_font_file(&path).is_ok()
            {
                *loaded += 1;
            }
        }
    }
}

fn load_configured_families(db: &mut Database, wanted: &[String]) {
    let mut loaded = 0;
    for dir in font_dirs() {
        load_font_dir(db, &std::path::Path::new(&dir), wanted, &mut loaded, 0);
    }
    if loaded == 0 {
        // nothing matched — fall back so text still renders at all
        db.load_system_fonts();
    } else if std::env::var("ZEN_TRACE").is_ok() {
        eprintln!("zen: loaded {loaded} font files for {:?}", wanted);
    }
}

impl TextEngine {
    pub fn new_with_slots(
        family: String,
        display_family: String,
        mono_family: String,
        brand_family: String,
        icon_family: String,
        icon_slots: std::collections::HashMap<char, char>,
    ) -> Self {
        let mut db = Database::new();
        let wanted = [
            family.clone(),
            display_family.clone(),
            mono_family.clone(),
            brand_family.clone(),
            icon_family.clone(),
        ];
        load_configured_families(&mut db, &wanted);
        let font_system = FontSystem::new_with_locale_and_db("en_US.UTF-8".into(), db);
        TextEngine {
            font_system,
            swash: SwashCache::new(),
            family,
            display_family,
            mono_family,
            brand_family,
            icon_family,
            icon_slots,
        }
    }

    /// Translate icon text to the active font's slots (canonical → chosen).
    /// Returns an owned string only when something changed.
    fn resolve_icon<'a>(&self, text: &'a str) -> std::borrow::Cow<'a, str> {
        if self.icon_slots.is_empty() {
            return std::borrow::Cow::Borrowed(text);
        }
        if let Some(pos) = text.find(|c| self.icon_slots.contains_key(&c)) {
            let mut out = String::with_capacity(text.len());
            out.push_str(&text[..pos]);
            for c in text[pos..].chars() {
                out.push(*self.icon_slots.get(&c).unwrap_or(&c));
            }
            std::borrow::Cow::Owned(out)
        } else {
            std::borrow::Cow::Borrowed(text)
        }
    }

    /// Resolve the requested (face, weight) against the loaded families.
    /// Icon glyphs always use the icon family at Regular — fonts flip fluids
    /// the slot table, never the weight request.
    fn attrs<'a>(&'a self, f: Ff, w: Fw, icon: bool) -> Attrs<'a> {
        if icon {
            return Attrs::new().family(Family::Name(&self.icon_family)).weight(Weight::NORMAL);
        }
        let fam = match f {
            Ff::Ui => self.family.as_str(),
            Ff::Display => self.display_family.as_str(),
            Ff::Mono => self.mono_family.as_str(),
            Ff::Brand => self.brand_family.as_str(),
        };
        Attrs::new().family(Family::Name(fam)).weight(w.weight())
    }

    fn line_h(size: f32) -> f32 {
        (size * 1.3).ceil()
    }

    /// Measure text → (width, height) in device px.
    pub fn measure(&mut self, text: &str, size: f32, f: Ff, w: Fw, icon: bool) -> (f32, f32) {
        // guard a stray non-positive size so cosmic-text's "line height cannot
        // be 0" assert can't crash the bar (font size * 1.3 → 0)
        let size = if size > 0.0 { size } else { 1.0 };
        let mut buf = Buffer::new(&mut self.font_system, Metrics::new(size, Self::line_h(size)));
        buf.set_size(None, None); // no wrap — natural width
        let text = if icon { self.resolve_icon(text) } else { std::borrow::Cow::Borrowed(text) };
        buf.set_text(&text, &self.attrs(f, w, icon), Shaping::Advanced, None);
        buf.shape_until_scroll(&mut self.font_system, true);
        let w = buf.layout_runs().map(|r| r.line_w).fold(0.0f32, f32::max);
        (w.ceil(), Self::line_h(size))
    }

    /// Rasterize `text` into a premultiplied RGBA8 buffer, tightly cropped to
    /// the ink bounding box (1px padding each side).
    ///
    /// Returns (width, height, pixels, ink_top) where `ink_top` is the Y offset
    /// of the first opaque pixel inside the texture — the caller uses it to
    /// position the text exactly.
    pub fn rasterize(
        &mut self,
        text: &str,
        size: f32,
        f: Ff,
        fwq: Fw,
        color: u32,
        icon: bool,
    ) -> (u32, u32, Vec<u8>, i32) {
        // guard a stray non-positive size so cosmic-text's "line height cannot
        // be 0" assert can't crash the bar (font size * 1.3 → 0)
        let size = if size > 0.0 { size } else { 1.0 };
        let (mw, mh) = self.measure(text, size, f, fwq, icon);
        let w = (mw as u32).max(1) + 4;
        let h = (mh as u32).max(1) + 4;

        let mut buf = Buffer::new(&mut self.font_system, Metrics::new(size, Self::line_h(size)));
        buf.set_size(Some(w as f32), Some(h as f32));
        let text = if icon { self.resolve_icon(text) } else { std::borrow::Cow::Borrowed(text) };
        buf.set_text(&text, &self.attrs(f, fwq, icon), Shaping::Advanced, None);
        buf.shape_until_scroll(&mut self.font_system, true);

        let mut pixels = vec![0u8; (w * h * 4) as usize];
        let mut ink: Option<(i32, i32, i32, i32)> = None; // (x0, y0, x1, y1)
        let col = Color(((color & 0xff) << 24) | (color >> 8)); // 0xRRGGBBAA → skrifa 0xAARRGGBB
        buf.draw(&mut self.font_system, &mut self.swash, col, |x, y, gw, gh, c| {
            let a = c.a();
            if a == 0 {
                return;
            }
            // cosmic-text hands us STRAIGHT alpha (RGB = full color, A = glyph
            // coverage). The renderer blends premultiplied
            // (`GL_ONE, GL_ONE_MINUS_SRC_ALPHA`), so premultiply here — feeding
            // straight alpha into that blend adds the full text color onto
            // every partial-coverage edge pixel, blowing them out past the
            // glyph color (cyan halo / fuzz — the "BIOS text" look).
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
                    match ink {
                        Some((x0, y0, x1, y1)) => {
                            ink = Some((x0.min(px), y0.min(py), x1.max(px + 1), y1.max(py + 1)))
                        }
                        None => ink = Some((px, py, px + 1, py + 1)),
                    }
                }
            }
        });

        match ink {
            None => (w, h, pixels, 0), // no ink — caller shouldn't draw
            Some((x0, y0, x1, y1)) => {
                // crop to ink bbox with 1px margin
                let x0 = (x0 - 1).max(0);
                let y0 = (y0 - 1).max(0);
                let x1 = (x1 + 1).min(w as i32);
                let y1 = (y1 + 1).min(h as i32);
                let cw = (x1 - x0) as u32;
                let ch = (y1 - y0) as u32;
                let mut cropped = vec![0u8; (cw * ch * 4) as usize];
                for row in 0..ch {
                    let src = ((y0 as u32 + row) * w + x0 as u32) as usize * 4;
                    let dst = row as usize * (cw as usize * 4);
                    cropped[dst..dst + (cw as usize * 4)]
                        .copy_from_slice(&pixels[src..src + (cw as usize * 4)]);
                }
                (cw, ch, cropped, y0)
            }
        }
    }
}
