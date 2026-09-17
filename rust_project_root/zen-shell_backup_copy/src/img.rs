//! Icon & image loading: icon-theme resolution + PNG decode + SVG rasterize.
//!
//! Output is premultiplied RGBA8 (tiny-skia is already premultiplied; PNG is
//! premultiplied after decode) — ready for `upload_texture` on either backend.
//! Everything rasterizes to the exact requested size, so glyph-style NEAREST
//! filtering stays crisp at 1:1.

use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};

/// Upper bound on decoded pixel entries. Live icons / tray pixmaps / the
/// visible clip-history strip are all re-touched every frame, so LRU never
/// evicts what's on screen; abandoned keys (`clip:N` history entries past the
/// trimmed history, stale tray serials) stop being re-derived anyway and were
/// previously a one-way CPU-memory leak.
const IMG_CAP: usize = 512;

pub struct ImageStore {
    /// resolved icon-theme name (from `ZEN_ICON_THEME` → gtk settings → hicolor)
    theme: String,
    /// loaded images: key → (w, h, premultiplied rgba)
    cache: HashMap<String, Option<(u32, u32, Vec<u8>)>>,
    /// LRU recency (front = most recently used; back = eviction target)
    lru: VecDeque<String>,
}

/// Read `gtk-icon-theme-name` from the gtk3/gtk4 settings.ini.
fn gtk_icon_theme() -> Option<String> {
    for p in [
        format!("{}/.config/gtk-4.0/settings.ini", std::env::var("HOME").ok()?),
        format!("{}/.config/gtk-3.0/settings.ini", std::env::var("HOME").ok()?),
    ] {
        let content = std::fs::read_to_string(p).ok()?;
        for line in content.lines() {
            let line = line.trim();
            if let Some(v) = line.strip_prefix("gtk-icon-theme-name=") {
                let v = v.trim().trim_matches('"').trim();
                if !v.is_empty() {
                    return Some(v.to_string());
                }
            }
        }
    }
    None
}

impl ImageStore {
    pub fn new() -> Self {
        let theme = std::env::var("ZEN_ICON_THEME")
            .ok()
            .filter(|t| !t.is_empty())
            .or_else(gtk_icon_theme)
            .unwrap_or_else(|| "hicolor".into());
        if std::env::var("ZEN_TRACE").is_ok() {
            eprintln!("zen: icon theme = {theme}");
        }
        ImageStore { theme, cache: HashMap::new(), lru: VecDeque::new() }
    }

    /// `put` raw premultiplied pixels under `key` (tray pixmaps). Size stays
    /// exactly as given — no rescale. `w == 0` or `h == 0` evicts the key.
    pub fn put(&mut self, key: &str, w: u32, h: u32, rgba: Vec<u8>) {
        if w == 0 || h == 0 {
            self.evict_key(key);
        } else {
            self.cache.insert(key.to_string(), Some((w, h, rgba)));
            self.touch(key);
            self.trim();
        }
    }

    /// Load `key` rasterized to fit a `size`×`size` box.
    ///
    /// Key forms:
    ///  - `file:<path>`            → absolute path to a .png / .jpg / .jpeg / .svg
    ///  - `icon:<name>`            → resolved through the icon theme
    ///  - anything pre-`put`       → raw pixels (tray)
    pub fn load(&mut self, key: &str, size: u32) -> Option<(u32, u32, Vec<u8>)> {
        if self.cache.contains_key(key) {
            self.touch(key);
            return self.cache.get(key).cloned().flatten();
        }
        let r = self.load_miss(key, size);
        self.cache.insert(key.to_string(), r.clone());
        self.touch(key);
        self.trim();
        r
    }

    /// Mark `key` most-recently-used.
    fn touch(&mut self, key: &str) {
        if let Some(pos) = self.lru.iter().position(|k| k.as_str() == key) {
            self.lru.remove(pos);
        }
        self.lru.push_front(key.to_string());
    }

    fn evict_key(&mut self, key: &str) {
        self.cache.remove(key);
        if let Some(pos) = self.lru.iter().position(|k| k.as_str() == key) {
            self.lru.remove(pos);
        }
    }

    /// Drop the least-recently-used entries past `IMG_CAP`.
    fn trim(&mut self) {
        while self.cache.len() > IMG_CAP {
            let Some(key) = self.lru.pop_back() else { break };
            self.cache.remove(&key);
        }
    }

    fn load_miss(&mut self, key: &str, size: u32) -> Option<(u32, u32, Vec<u8>)> {
        // `thumb:<basename>` → the wallpaper thumbnail from the disk cache
        // ($HOME/.cache/thumbnails/zen-shell), where each file is named after
        // the wallpaper's own basename (spaces and all). The picker grid only
        // lists items that are already cached, so the file exists; anything
        // else returns None (never a blank tile from a missing file).
        if let Some(name) = key.strip_prefix("thumb:") {
            let cache = crate::img::thumbnail_cache_dir().join(name);
            let bytes = std::fs::read(&cache).ok()?;
            return rasterize_png(&bytes, size);
        }
        let (path, icon_mode) = if let Some(p) = key.strip_prefix("file:") {
            (PathBuf::from(p), false)
        } else if let Some(name) = key.strip_prefix("icon:") {
            (resolve_icon(&self.theme, name)?, true)
        } else {
            return None; // not found and not a recognized key
        };
        let _ = icon_mode;
        rasterize(&path, size).or_else(|| {
            if std::env::var("ZEN_TRACE").is_ok() {
                eprintln!("zen: icon {}: failed to rasterize", path.display());
            }
            None
        })
    }
}

impl Default for ImageStore {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// icon theme resolution
// ---------------------------------------------------------------------------

/// Directories that may hold icon themes, most-relevant first.
fn theme_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
        dirs.push(PathBuf::from(xdg).join("icons"));
    }
    if let Ok(home) = std::env::var("HOME") {
        dirs.push(PathBuf::from(&home).join(".local/share/icons"));
        dirs.push(PathBuf::from(&home).join(".icons"));
    }
    dirs.push("/usr/local/share/icons".into());
    dirs.push("/usr/share/icons".into());
    dirs
}

/// `Inherits=` list from a theme's `index.theme`.
fn theme_inherits(theme_dir: &Path) -> Vec<String> {
    let Ok(content) = std::fs::read_to_string(theme_dir.join("index.theme")) else { return Vec::new() };
    for line in content.lines() {
        if let Some(v) = line.trim().strip_prefix("Inherits=") {
            return v.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
        }
    }
    Vec::new()
}

/// Does `d` look like an icon size dir (`24`, `24x24`, `scalable`, …)?
fn size_key(d: &str) -> Option<i32> {
    let base = d.split('@').next().unwrap_or(d);
    let base = base.split('x').next().unwrap_or(base);
    base.parse::<i32>().ok()
}

fn is_scalable_dir(d: &str) -> bool {
    d == "scalable"
}

/// Resolve `name` to the best matching icon file across the theme stack.
fn resolve_icon(theme: &str, name: &str) -> Option<PathBuf> {
    // theme priority: [configured] + its Inherits + hicolor
    let mut stack: Vec<String> = vec![theme.to_string()];
    let mut seen = std::collections::HashSet::new();
    let mut inherits = Vec::new();
    for d in theme_dirs() {
        let td = d.join(theme);
        if td.is_dir() {
            inherits = theme_inherits(&td);
            break;
        }
    }
    stack.extend(inherits);
    if !stack.iter().any(|s| s == "hicolor") {
        stack.push("hicolor".into());
    }

    // (theme_priority, size_distance, path)
    let mut best: Option<(usize, i32, PathBuf)> = None;
    for (ti, tname) in stack.iter().enumerate() {
        if !seen.insert(tname.clone()) {
            continue;
        }
        for d in theme_dirs() {
            let theme_dir = d.join(tname);
            let Ok(rd) = std::fs::read_dir(&theme_dir) else { continue };
            for entry in rd.flatten() {
                let sub = entry.path();
                if !sub.is_dir() {
                    continue;
                }
                let sname = entry.file_name().to_string_lossy().into_owned();
                let dist = if is_scalable_dir(&sname) {
                    1
                } else if let Some(n) = size_key(&sname) {
                    (n - 16).abs().min((n - 24).abs()).min((n - 32).abs()).min((n - 48).abs())
                } else {
                    continue;
                };
                // app icons live under `<size>/apps/`
                let apps = sub.join("apps");
                let mut found: Option<PathBuf> = None;
                for ext in ["svg", "png"] {
                    let f = apps.join(format!("{name}.{ext}"));
                    if f.is_file() {
                        found = Some(f);
                        break;
                    }
                }
                let Some(f) = found else { continue };
                let better = match &best {
                    None => true,
                    Some((bt, bd, _)) => ti < *bt || (ti == *bt && dist < *bd),
                };
                if better {
                    best = Some((ti, dist, f));
                }
            }
        }
    }
    best.map(|(_, _, p)| p)
}

// ---------------------------------------------------------------------------
// rasterization
// ---------------------------------------------------------------------------

/// Rasterize a file to premultiplied RGBA8 fit inside `size`²: png / jpg /
/// jpeg / gif rasterize directly; svg (icons) via resvg. Unsupported
/// formats → None.
fn rasterize(path: &Path, size: u32) -> Option<(u32, u32, Vec<u8>)> {
    match path.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase()).as_deref() {
        Some("svg") => {
            let data = std::fs::read(path).ok()?;
            rasterize_svg(&data, size)
        }
        _ => rasterize_straight(path, size).map(|(w, h, mut px)| {
            premultiply(&mut px);
            (w, h, px)
        }),
    }
}

/// Decode + aspect-fit a bitmap file to straight (non-premultiplied) RGBA8
/// inside `size`². Only the formats the thumbnail cache accepts (see
/// `supported_image`); everything else → None.
fn rasterize_straight(path: &Path, size: u32) -> Option<(u32, u32, Vec<u8>)> {
    let data = std::fs::read(path).ok()?;
    match path.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase()).as_deref() {
        Some("png") => decode_png_straight(&data, size),
        Some("jpg") | Some("jpeg") => rasterize_bitmap(&data, image::ImageFormat::Jpeg, size),
        Some("gif") => rasterize_bitmap(&data, image::ImageFormat::Gif, size),
        _ => None,
    }
}

/// Decode + cover-fit a bitmap file to a full-bleed `size`×`size` square
/// (short axis scaled to `size`, long axis center-cropped). Used for
/// wallpaper tiles so every cell is a clean filled photo, never letterboxed.
fn rasterize_cover(path: &Path, size: u32) -> Option<(u32, u32, Vec<u8>)> {
    let (w, h, rgba) = rasterize_straight(path, size)?;
    if w == 0 || h == 0 {
        return None;
    }
    let (w2, h2, px) = scale_rgba_cover(&rgba, w, h, size);
    Some((w2, h2, px))
}

/// Decode a jpg/jpeg/gif via the `image` crate, downscaled to fit inside a
/// `size`×`size` box (a 4K wallpaper stays a small thumbnail).
fn rasterize_bitmap(data: &[u8], fmt: image::ImageFormat, size: u32) -> Option<(u32, u32, Vec<u8>)> {
    let img = image::load_from_memory_with_format(data, fmt).ok()?;
    let thumb = img.thumbnail(size, size); // fits inside, preserves aspect
    let rgba = thumb.to_rgba8();
    let (w, h) = rgba.dimensions();
    Some((w, h, rgba.into_raw()))
}

fn rasterize_svg(data: &[u8], size: u32) -> Option<(u32, u32, Vec<u8>)> {
    let opt = resvg::usvg::Options::default();
    let tree = resvg::usvg::Tree::from_data(data, &opt).ok()?;
    let vw = tree.size().width().max(0.001);
    let vh = tree.size().height().max(0.001);
    let scale = size as f32 / vw.max(vh);
    let (w, h) = ((vw * scale).round().max(1.0) as u32, (vh * scale).round().max(1.0) as u32);
    let mut pixmap = resvg::tiny_skia::Pixmap::new(w, h)?;
    let ts = resvg::tiny_skia::Transform::from_scale(scale, scale);
    resvg::render(&tree, ts, &mut pixmap.as_mut());
    Some((w, h, pixmap.take()))
}

/// PNG decode + aspect-fit downscale → straight RGBA8 (caller premultiplies).
fn decode_png_straight(data: &[u8], size: u32) -> Option<(u32, u32, Vec<u8>)> {
    let mut dec = png::Decoder::new(data);
    dec.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = dec.read_info().ok()?;
    let info = reader.info();
    let w = info.width;
    let h = info.height;
    let color_type = info.color_type;
    let mut buf = vec![0u8; reader.output_buffer_size()];
    let out = reader.next_frame(&mut buf).ok()?;
    let bpp = match color_type {
        png::ColorType::Rgba => 4,
        png::ColorType::Rgb => 3,
        png::ColorType::Grayscale => 1,
        png::ColorType::GrayscaleAlpha => 2,
        _ => return None,
    };
    let px = &buf[..out.buffer_size().min(buf.len())];
    let _ = bpp;
    let rgba = match color_type {
        png::ColorType::Rgba => px.to_vec(),
        png::ColorType::Rgb => {
            let mut v = Vec::with_capacity(px.len() / 3 * 4);
            for c in px.chunks_exact(3) {
                v.extend_from_slice(&[c[0], c[1], c[2], 255]);
            }
            v
        }
        png::ColorType::Grayscale => {
            let mut v = Vec::with_capacity(px.len() * 4);
            for &g in px {
                v.extend_from_slice(&[g, g, g, 255]);
            }
            v
        }
        png::ColorType::GrayscaleAlpha => {
            let mut v = Vec::with_capacity(px.len() / 2 * 4);
            for c in px.chunks_exact(2) {
                v.extend_from_slice(&[c[0], c[0], c[0], c[1]]);
            }
            v
        }
        _ => return None,
    };
    let (w2, h2, px2) = if w != h || w > size {
        scale_rgba(&rgba, w, h, size)
    } else {
        (w, h, rgba)
    };
    Some((w2, h2, px2))
}

/// PNG decode + aspect-fit → premultiplied RGBA8.
fn rasterize_png(data: &[u8], size: u32) -> Option<(u32, u32, Vec<u8>)> {
    let (w, h, mut px) = decode_png_straight(data, size)?;
    premultiply(&mut px);
    Some((w, h, px))
}

/// Simple box-bilinear RGBA scaler, fit inside `size`², preserving aspect.
fn scale_rgba(src: &[u8], sw: u32, sh: u32, size: u32) -> (u32, u32, Vec<u8>) {
    if sw == 0 || sh == 0 || size == 0 {
        return (sw, sh, src.to_vec());
    }
    // clamp the source coordinates into the valid range so a very small
    // target (tiny `scale`) can never saturate to u32::MAX and overflow
    let y_max = (sh - 1) as f32;
    let x_max = (sw - 1) as f32;
    let scale = size as f32 / sw.max(sh) as f32;
    let scale = if scale >= 1.0 { 1.0 } else { scale }; // only downscale
    let dw = (sw as f32 * scale).round().max(1.0) as u32;
    let dh = (sh as f32 * scale).round().max(1.0) as u32;
    if dw == sw && dh == sh {
        return (sw, sh, src.to_vec());
    }
    let mut dst = vec![0u8; (dw * dh * 4) as usize];
    for y in 0..dh {
        let sy = (y as f32 + 0.5) / scale - 0.5;
        let y0 = sy.floor().clamp(0.0, y_max) as u32;
        let y1 = (y0 + 1).min(sh - 1);
        let fy = sy - sy.floor();
        for x in 0..dw {
            let sx = (x as f32 + 0.5) / scale - 0.5;
            let x0 = sx.floor().clamp(0.0, x_max) as u32;
            let x1 = (x0 + 1).min(sw - 1);
            let fx = sx - sx.floor();
            let p00 = ((y0 * sw + x0) * 4) as usize;
            let p01 = ((y0 * sw + x1) * 4) as usize;
            let p10 = ((y1 * sw + x0) * 4) as usize;
            let p11 = ((y1 * sw + x1) * 4) as usize;
            let d = ((y * dw + x) * 4) as usize;
            for c in 0..4 {
                let v = src[p00 + c] as f32 * (1.0 - fx) * (1.0 - fy)
                    + src[p01 + c] as f32 * fx * (1.0 - fy)
                    + src[p10 + c] as f32 * (1.0 - fx) * fy
                    + src[p11 + c] as f32 * fx * fy;
                dst[d + c] = v.round() as u8;
            }
        }
    }
    (dw, dh, dst)
}

/// Cover-mode bilinear RGBA scaler: scale so the SHORT axis fills `size`
/// (upscaling allowed), then center-crop to a `size`×`size` square — the
/// full-bleed tile for wallpaper grids.
fn scale_rgba_cover(src: &[u8], sw: u32, sh: u32, size: u32) -> (u32, u32, Vec<u8>) {
    if sw == 0 || sh == 0 {
        return (sw, sh, src.to_vec());
    }
    let norm = sw.min(sh);
    let scale = size as f32 / norm as f32;
    let dw = (sw as f32 * scale).round().max(1.0) as u32;
    let dh = (sh as f32 * scale).round().max(1.0) as u32;
    if dw != sw || dh != sh {
        let mut dst = vec![0u8; (dw * dh * 4) as usize];
        for y in 0..dh {
            let sy = (y as f32 + 0.5) / scale - 0.5;
            let y0 = sy.floor().max(0.0) as u32;
            let y1 = (y0 + 1).min(sh - 1);
            let fy = sy - sy.floor();
            for x in 0..dw {
                let sx = (x as f32 + 0.5) / scale - 0.5;
                let x0 = sx.floor().max(0.0) as u32;
                let x1 = (x0 + 1).min(sw - 1);
                let fx = sx - sx.floor();
                let p00 = ((y0 * sw + x0) * 4) as usize;
                let p01 = ((y0 * sw + x1) * 4) as usize;
                let p10 = ((y1 * sw + x0) * 4) as usize;
                let p11 = ((y1 * sw + x1) * 4) as usize;
                let d = ((y * dw + x) * 4) as usize;
                for c in 0..4 {
                    let v = src[p00 + c] as f32 * (1.0 - fx) * (1.0 - fy)
                        + src[p01 + c] as f32 * fx * (1.0 - fy)
                        + src[p10 + c] as f32 * (1.0 - fx) * fy
                        + src[p11 + c] as f32 * fx * fy;
                    dst[d + c] = v.round() as u8;
                }
            }
        }
        // center-crop to the square
        let ox = ((dw - size) / 2) as usize;
        let oy = ((dh - size) / 2) as usize;
        let mut out = vec![0u8; (size * size * 4) as usize];
        for yy in 0..size as usize {
            let srow = (oy + yy) * dw as usize + ox;
            let d = yy * size as usize;
            out[d * 4..(d + size as usize) * 4].copy_from_slice(&dst[srow * 4..(srow + size as usize) * 4]);
        }
        return (size, size, out);
    }
    // already square at the right side
    let ox = ((dw - size) / 2) as usize;
    let oy = ((dh - size) / 2) as usize;
    if ox == 0 && oy == 0 {
        return (dw, dh, src.to_vec());
    }
    let mut out = vec![0u8; (size * size * 4) as usize];
    for yy in 0..size as usize {
        let srow = (oy + yy) * dw as usize + ox;
        let d = yy * size as usize;
        out[d * 4..(d + size as usize) * 4].copy_from_slice(&src[srow * 4..(srow + size as usize) * 4]);
    }
    (size, size, out)
}

/// Straight RGBA → premultiplied RGBA in place.
fn premultiply(px: &mut [u8]) {
    for c in px.chunks_exact_mut(4) {
        let a = c[3] as u32;
        if a == 255 {
            continue;
        }
        c[0] = (c[0] as u32 * a / 255) as u8;
        c[1] = (c[1] as u32 * a / 255) as u8;
        c[2] = (c[2] as u32 * a / 255) as u8;
    }
}

// ---------------------------------------------------------------------------
// thumbnail cache ($HOME/.cache/thumbnails/zen-shell)
// ---------------------------------------------------------------------------

/// Side of the square box cached thumbnails are generated into. The picker
/// grid cells are ~88px, so 256px stays crisp on HiDPI.
pub const THUMB_SIZE: u32 = 256;

/// Formats we generate thumbnails for — jpg/jpeg/png/gif only. Everything
/// else in the wallpaper dir is skipped (never decoded, never cached).
pub const THUMB_EXTS: [&str; 4] = ["jpg", "jpeg", "png", "gif"];

/// `$XDG_CACHE_HOME/thumbnails/zen-shell`, falling back to
/// `$HOME/.cache/thumbnails/zen-shell` when XDG_CACHE_HOME is unset.
pub fn thumbnail_cache_dir() -> PathBuf {
    let base = std::env::var("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|_| std::env::var("HOME").map(|h| Path::new(&h).join(".cache")))
        .unwrap_or_else(|_| PathBuf::from(".cache"));
    base.join("thumbnails/zen-shell")
}

/// Is `p` a supported image (jpg / jpeg / png / gif, case-insensitive)?
pub fn supported_image(p: &Path) -> bool {
    p.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .map(|e| THUMB_EXTS.contains(&e.as_str()))
        .unwrap_or(false)
}

/// Cache file for `src`: `<cache>/<basename>` — the wallpaper's own file
/// name (spaces and all), so the picker grid can map a cached thumbnail
/// straight back to `$HOME/Wallpapers/<basename>`. Stable across runs, so a
/// re-generated thumbnail lands on the same file (incremental).
pub fn thumbnail_path(src: &Path) -> Option<PathBuf> {
    Some(thumbnail_cache_dir().join(src.file_name()?))
}

/// Fresh = the cached thumbnail exists, is a full-bleed `THUMB_SIZE`² tile,
/// and isn't older than the source file. Legacy caches written before the
/// square-tile switch are letterboxed (contain) — those read as stale and
/// regenerate. A touched/edited wallpaper regenerates; everything else is
/// reused without re-decoding.
pub fn thumbnail_fresh(cache: &Path, src: &Path) -> bool {
    let (Ok(cm), Ok(sm)) = (
        cache.metadata().and_then(|m| m.modified()),
        src.metadata().and_then(|m| m.modified()),
    ) else {
        return false;
    };
    if cm < sm {
        return false;
    }
    // PNG IHDR: w/h big-endian at byte offsets 16/20 (after 8-byte sig + len/type)
    let mut head = [0u8; 24];
    use std::io::Read;
    if std::fs::File::open(cache).and_then(|mut f| f.read_exact(&mut head)).is_err() {
        return false;
    }
    if &head[0..8] != b"\x89PNG\r\n\x1a\n" || &head[12..16] != b"IHDR" {
        return false;
    }
    let w = u32::from_be_bytes([head[16], head[17], head[18], head[19]]);
    let h = u32::from_be_bytes([head[20], head[21], head[22], head[23]]);
    w == THUMB_SIZE && h == THUMB_SIZE
}

/// Encode straight RGBA8 as a PNG file.
fn write_png(path: &Path, w: u32, h: u32, rgba: &[u8]) -> std::io::Result<()> {
    let file = std::fs::File::create(path)?;
    let mut enc = png::Encoder::new(file, w, h);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    let mut writer = enc.write_header()?;
    writer.write_image_data(rgba)?;
    Ok(())
}

/// Generate (or reuse) the cached thumbnail for `src`, fit inside `size`².
///
/// Incremental: a fresh thumbnail (mtime-verified) is returned without
/// re-decoding the source; missing/stale ones are regenerated. Unsupported
/// formats (not jpg/jpeg/png/gif) or undecodable files → None. The PNG is
/// written via a temp file + rename so a crash mid-write never leaves a
/// truncated thumbnail.
pub fn generate_thumbnail(src: &Path, size: u32) -> Option<PathBuf> {
    if !supported_image(src) {
        return None;
    }
    let cache = thumbnail_path(src)?;
    if thumbnail_fresh(&cache, src) {
        return Some(cache);
    }
    let (w, h, rgba) = rasterize_cover(src, size)?;
    let dir = cache.parent()?;
    std::fs::create_dir_all(dir).ok()?;
    let tmp = cache.with_extension("png.tmp");
    if write_png(&tmp, w, h, &rgba).is_err() {
        let _ = std::fs::remove_file(&tmp);
        return None;
    }
    let _ = std::fs::rename(&tmp, &cache);
    Some(cache)
}

/// Scan `dir` on a background thread and generate missing/stale thumbnails
/// (incremental — fresh ones are skipped; unsupported files are ignored).
/// Spawned at startup so the wallpaper picker is always warm without ever
/// blocking the bar's event loop.
pub fn spawn_thumbnail_warm(dir: PathBuf) {
    std::thread::spawn(move || {
        let Ok(rd) = std::fs::read_dir(&dir) else { return };
        let mut files: Vec<PathBuf> = rd
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_file() && supported_image(p))
            .collect();
        files.sort();
        for f in files {
            let _ = generate_thumbnail(&f, THUMB_SIZE);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supported_formats_only() {
        for e in ["jpg", "jpeg", "png", "gif", "PNG", "JPG"] {
            assert!(supported_image(Path::new(&format!("wp.{e}"))), "{e} must be supported");
        }
        for e in ["svg", "webp", "bmp", "tiff", "txt", ""] {
            assert!(!supported_image(Path::new(&format!("wp.{e}"))), "{e} must be rejected");
        }
    }

    #[test]
    fn cache_named_by_basename() {
        // basename with extension — spaces and all — lands in the cache dir
        let p = Path::new("/home/tw/Wallpapers/mountain lake.jpg");
        let c = thumbnail_path(p).expect("thumb path");
        assert_eq!(c.file_name().unwrap().to_str().unwrap(), "mountain lake.jpg");
        assert_eq!(thumbnail_path(p), thumbnail_path(p));
        // different basenames never collide
        let q = Path::new("/home/tw/Wallpapers/other.jpg");
        assert_ne!(thumbnail_path(p), thumbnail_path(q));
    }

    #[test]
    fn unsupported_format_never_generates() {
        let p = Path::new("/tmp/zen/x/wallpaper.webp");
        assert!(generate_thumbnail(p, 256).is_none());
    }

    /// Generate → mtime-verified reuse → regenerate when the source is newer.
    #[test]
    fn incremental_generate_and_verify() {
        let dir = std::env::temp_dir().join(format!("zen-thumb-test-{}", std::process::id()));
        // unique basename so the test can never clobber a real cached thumb
        let src = dir.join(format!("zen-thumb-test-{}.png", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        // 4x4 solid PNG (straight rgba)
        let px: Vec<u8> = vec![255u8, 0, 0, 255].repeat(16);
        write_png(&src, 4, 4, &px).unwrap();

        let c1 = generate_thumbnail(&src, 256).expect("png must thumbnail");
        assert!(c1.is_file());
        assert!(thumbnail_fresh(&c1, &src));
        let mtime1 = c1.metadata().unwrap().modified().unwrap();

        // Already generated → same file, not regenerated.
        let c2 = generate_thumbnail(&src, 256).expect("fresh thumb reused");
        assert_eq!(c1, c2);
        assert_eq!(c2.metadata().unwrap().modified().unwrap(), mtime1);

        // Age the cached thumb below the source → stale → regenerated into
        // the same slot.
        let past = std::time::SystemTime::now() - std::time::Duration::from_secs(120);
        filetime_set(&c1, past).unwrap();
        let c3 = generate_thumbnail(&src, 256).expect("stale thumb regenerated");
        assert_eq!(c1, c3); // same cache slot, fresh again
        assert!(thumbnail_fresh(&c3, &src));

        let _ = std::fs::remove_dir_all(&dir);
        // clean up only the cache files this test wrote
        if let Some(cp) = thumbnail_path(&src) {
            let _ = std::fs::remove_file(&cp);
            let _ = std::fs::remove_file(cp.with_extension("png.tmp"));
        }
    }

    /// Zero-size targets (negative f32 → u32 wraps to 0) used to make
    /// `sy = inf` saturate `y0 = u32::MAX` and overflow `y0 + 1` — panic on
    /// every frame with a huge source. Must return, never panic.
    #[test]
    fn scale_zero_target_never_panics() {
        let px = vec![0u8; 4096 * 4096 * 4];
        let (w, h, out) = scale_rgba(&px, 4096, 4096, 0);
        assert_eq!((w, h), (4096, 4096));
        assert_eq!(out.len(), px.len());
        // a tiny positive target close to the old failure mode also must not
        // overflow even for a very large source
        let big = vec![0u8; 32768 * 32768 * 4];
        let (w2, _, out2) = scale_rgba(&big, 32768, 32768, 2);
        assert!(w2 <= 2);
        assert!(!out2.is_empty());
    }

    /// Best-effort mtime setter (tests only; no external deps).
    fn filetime_set(p: &Path, t: std::time::SystemTime) -> std::io::Result<()> {
        let f = std::fs::File::options().write(true).open(p)?;
        f.set_times(std::fs::FileTimes::new().set_modified(t))?;
        Ok(())
    }
}
