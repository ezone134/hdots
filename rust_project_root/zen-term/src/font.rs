//! Font loading + glyph rasterization via fontdb + ab_glyph (the same stack
//! wfm uses).
//!
//! Startup is deliberately cheap: only the PRIMARY font is read from disk and
//! parsed. Fallback faces are resolved lazily — a face is only loaded the
//! first time a glyph is missing from the primary font, then cached. That
//! keeps cold launch to one font read regardless of how many fonts the
//! system has installed.
//!
//! Note: we never rely on fontdb's generic `Family::Monospace` query — it
//! matches the *literal* default family name ("Courier New"), which exists on
//! no modern Linux. Instead we pick faces by their `monospaced` flag.

use ab_glyph::{Font, FontArc, FontRef, GlyphId, ScaleFont};
use fontdb::Source;

/// High-quality monospace families, in preference order, tried when no
/// explicit family is configured (or the configured one is missing).
const NICE_MONO: &[&str] = &[
    "JetBrains Mono",
    "Iosevka",
    "Iosevka Nerd Font",
    "Cascadia Mono",
    "CaskaydiaCove Nerd Font",
    "CaskaydiaCove Nerd Font Propo",
    "Fira Code",
    "Hack",
    "Source Code Pro",
    "Ubuntu Mono",
    "DejaVu Sans Mono",
    "Liberation Mono",
    "Noto Sans Mono",
    "Courier New",
];

pub struct Fonts {
    /// fontdb stays alive for lazy fallback resolution.
    db: fontdb::Database,
    /// Primary (monospace) font — index 0.
    pub primary: FontArc,
    /// Lazily-loaded fallback chain for glyphs missing from the primary font.
    fallbacks: Vec<FontArc>,
    /// fontdb id each fallback was loaded from (skip on later scans).
    fallback_src: Vec<fontdb::ID>,
    /// Font index per cached char, so the atlas key can stay a plain u64.
    char_font: std::collections::HashMap<char, u16>,
    pub px: f32,
}

impl Fonts {
    pub fn load(family: &str, px: f32) -> Self {
        let mut db = fontdb::Database::new();
        db.load_system_fonts();
        // fontdb 0.20's system scan skips symlinked directories, which is the
        // norm for ~/.local/share/fonts in containerized homes. Manually scan
        // the standard user dirs (following symlinks) so personal fonts load.
        load_user_font_dirs(&mut db);

        // Primary: exact requested family → curated mono list → any monospaced
        // face → any face at all. Only ONE font file is read here.
        let primary = if family.trim().is_empty() {
            None
        } else {
            db.query(&fontdb::Query {
                families: &[fontdb::Family::Name(family.trim())],
                ..Default::default()
            })
            .and_then(|id| read_face(&db, id))
        }
        .or_else(|| {
            NICE_MONO.iter().find_map(|name| {
                db.query(&fontdb::Query {
                    families: &[fontdb::Family::Name(name)],
                    ..Default::default()
                })
                .and_then(|id| read_face(&db, id))
            })
        })
        .or_else(|| pick_face(&db, |f| f.monospaced && has_ascii(&db, f.id)))
        .or_else(|| pick_face(&db, |_| true))
        .expect("zen-term: no usable font found on this system (fontconfig/fontdb found nothing)");

        Self {
            db,
            primary,
            fallbacks: Vec::new(),
            fallback_src: Vec::new(),
            char_font: std::collections::HashMap::new(),
            px,
        }
    }

    #[inline]
    pub fn font(&self, idx: u16) -> &FontArc {
        if idx == 0 {
            &self.primary
        } else {
            &self.fallbacks[(idx - 1) as usize]
        }
    }

    /// Font index for `c`, resolving the fallback chain lazily on a miss.
    pub fn font_for(&mut self, c: char) -> u16 {
        if let Some(&idx) = self.char_font.get(&c) {
            return idx;
        }
        let idx = if self.primary.as_scaled(self.px).glyph_id(c).0 != 0 {
            0
        } else {
            self.load_fallback_for(c)
        };
        self.char_font.insert(c, idx);
        idx
    }

    /// Scan the system for the first face containing `c` (monospaced faces
    /// first), load it, and append it to the fallback chain. Returns the new
    /// font index, or 0 when nothing has the glyph (renders .notdef).
    fn load_fallback_for(&mut self, c: char) -> u16 {
        // Build scan order: monospaced faces first, then everything else.
        let mut order: Vec<fontdb::ID> = Vec::new();
        for face in self.db.faces() {
            if face.monospaced {
                order.push(face.id);
            }
        }
        for face in self.db.faces() {
            if !face.monospaced {
                order.push(face.id);
            }
        }

        for id in order {
            // Already loaded and known to lack this glyph.
            if self.fallback_src.contains(&id) {
                continue;
            }
            // Peek at the face data (memory-mapped, cheap) for the glyph.
            let has = self
                .db
                .with_face_data(id, |data, index| {
                    FontRef::try_from_slice_and_index(data, index)
                        .map(|f| f.as_scaled(self.px).glyph_id(c).0 != 0)
                        .unwrap_or(false)
                })
                .unwrap_or(false);
            if !has {
                continue;
            }
            if let Some(f) = read_face(&self.db, id) {
                self.fallback_src.push(id);
                self.fallbacks.push(f);
                return self.fallbacks.len() as u16;
            }
        }
        0
    }

    /// Rasterize `c` at the current size into premultiplied RGBA.
    /// Returns None when the glyph has no outline (space, control chars).
    pub fn rasterize(&mut self, c: char) -> Option<GlyphRaster> {
        let idx = self.font_for(c);
        let font = self.font(idx).clone();
        let scaled = font.as_scaled(self.px);
        let gid: GlyphId = scaled.glyph_id(c);
        if gid.0 == 0 {
            return None;
        }
        let glyph = gid.with_scale_and_position(self.px, ab_glyph::point(0.0, 0.0));
        let outline = font.outline_glyph(glyph)?;
        let bounds = outline.px_bounds();
        let w = bounds.width().ceil() as u32;
        let h = bounds.height().ceil() as u32;
        if w == 0 || h == 0 {
            return None;
        }
        let mut alpha = vec![0u8; (w * h) as usize];
        outline.draw(|x, y, cov| {
            // ab_glyph's draw closure yields u32 pixel coords within bounds.
            if x < w && y < h {
                let i = (y as usize) * (w as usize) + (x as usize);
                alpha[i] = (cov * 255.0) as u8;
            }
        });
        // Premultiplied RGBA (rgb = alpha) for the ONE, ONE_MINUS_SRC_ALPHA blend.
        let mut rgba = Vec::with_capacity(alpha.len() * 4);
        for a in alpha {
            rgba.extend_from_slice(&[a, a, a, a]);
        }
        Some(GlyphRaster {
            w,
            h,
            offset_x: bounds.min.x,
            offset_y: bounds.min.y,
            rgba,
        })
    }

    /// Cell metrics at the configured size: (cell_w, cell_h, ascent).
    pub fn metrics(&self) -> (f32, f32, f32) {
        let scaled = self.primary.as_scaled(self.px);
        let ascent = scaled.ascent();
        let descent = scaled.descent();
        let line_gap = scaled.line_gap();
        // Monospace advance: probe common glyphs, take the widest.
        let mut cell_w = 0.0f32;
        for c in ['M', '0', 'W', '@'] {
            let g = scaled.glyph_id(c);
            cell_w = cell_w.max(scaled.h_advance(g));
        }
        if cell_w <= 0.0 {
            cell_w = self.px * 0.6;
        }
        let cell_h = (ascent - descent + line_gap).ceil().max(1.0);
        (cell_w, cell_h, ascent)
    }
}

pub struct GlyphRaster {
    pub w: u32,
    pub h: u32,
    pub offset_x: f32,
    pub offset_y: f32,
    pub rgba: Vec<u8>,
}

/// Scan the standard per-user font directories (symlinks followed) and load
/// every font file. fontdb 0.20's `load_system_fonts` uses `DirEntry::
/// file_type()`, which reports symlinks as symlinks and skips them — so in
/// homes where the font dir is a symlink (containers, network homes), the
/// user's fonts would otherwise never be seen.
fn load_user_font_dirs(db: &mut fontdb::Database) {
    let mut roots: Vec<std::path::PathBuf> = Vec::new();
    if let Ok(home) = std::env::var("HOME") {
        if !home.is_empty() {
            roots.push(std::path::Path::new(&home).join(".fonts"));
            roots.push(std::path::Path::new(&home).join(".local/share/fonts"));
        }
    }
    if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
        if !xdg.is_empty() {
            roots.push(std::path::Path::new(&xdg).join("fonts"));
        }
    }
    let mut seen: std::collections::HashSet<std::path::PathBuf> = std::collections::HashSet::new();
    let mut stack = roots;
    while let Some(dir) = stack.pop() {
        // metadata() follows symlinks; canonicalize dedupes to the real path.
        let Ok(real) = std::fs::canonicalize(&dir) else { continue };
        if !seen.insert(real) {
            continue;
        }
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        for ent in entries.flatten() {
            let path = ent.path();
            let Ok(meta) = std::fs::metadata(&path) else { continue };
            if meta.is_dir() {
                stack.push(path);
            } else if meta.is_file() {
                let Some(ext) = path.extension().and_then(|e| e.to_str()) else { continue };
                if matches!(ext.to_ascii_lowercase().as_str(), "ttf" | "ttc" | "otf" | "otc") {
                    let _ = db.load_font_file(&path);
                }
            }
        }
    }
}

/// True when the face has glyphs for basic ASCII (rejects icon/emoji fonts
/// that can't render text).
fn has_ascii(db: &fontdb::Database, id: fontdb::ID) -> bool {
    db.with_face_data(id, |data, index| {
        FontRef::try_from_slice_and_index(data, index)
            .map(|f| {
                let s = f.as_scaled(14.0);
                ['M', '0', 'a'].iter().all(|&c| s.glyph_id(c).0 != 0)
            })
            .unwrap_or(false)
    })
    .unwrap_or(false)
}

/// First face matching `want` that actually loads as a font.
fn pick_face(db: &fontdb::Database, want: impl Fn(&fontdb::FaceInfo) -> bool) -> Option<FontArc> {
    for face in db.faces() {
        if want(face) {
            if let Some(f) = read_face(db, face.id) {
                return Some(f);
            }
        }
    }
    None
}

/// Load a face as an owned `FontArc` (fontdb 0.20 API, mirrors wfm).
fn read_face(db: &fontdb::Database, id: fontdb::ID) -> Option<FontArc> {
    let face = db.face(id)?;
    let index = face.index;
    let (source, _) = db.face_source(id)?;
    let bytes: Vec<u8> = match source {
        Source::File(path) => std::fs::read(path.as_path()).ok()?,
        Source::Binary(data) => data.as_ref().as_ref().to_vec(),
        Source::SharedFile(_, data) => data.as_ref().as_ref().to_vec(),
    };
    if index == 0 {
        FontArc::try_from_vec(bytes).ok()
    } else {
        // FontRef borrows; leak the buffer for process lifetime (fonts load once).
        let static_bytes: &'static [u8] = Box::leak(bytes.into_boxed_slice());
        FontRef::try_from_slice_and_index(static_bytes, index).ok().map(FontArc::from)
    }
}
