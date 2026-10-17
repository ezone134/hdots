//! Built-in SVG icons rendered via resvg + tiny-skia, tinted per theme color.
//!
//! Sources live in `icons/*.svg` (24×24 monochrome Material-style paths using a
//! `{{c}}` fill placeholder). Trees are parsed once per (name, color) and cached
//! for the process lifetime; drawing blits a tiny-skia pixmap into the ARGB
//! framebuffer with alpha blending.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use crate::app::App;

const ICONS: &[(&str, &str)] = &[
    // nav / view-mode / search (menu bar, path bar)
    ("back", include_str!("../icons/back.svg")),
    ("forward", include_str!("../icons/forward.svg")),
    ("up", include_str!("../icons/up.svg")),
    ("home", include_str!("../icons/home.svg")),
    ("list", include_str!("../icons/list.svg")),
    ("grid", include_str!("../icons/grid.svg")),
    ("compact", include_str!("../icons/compact.svg")),
    ("search", include_str!("../icons/search.svg")),
    ("split", include_str!("../icons/split.svg")),
    // file / folder type icons (list/grid/compact views)
    ("folder", include_str!("../icons/folder.svg")),
    ("file", include_str!("../icons/file.svg")),
    ("image", include_str!("../icons/image.svg")),
    ("archive", include_str!("../icons/archive.svg")),
    ("audio", include_str!("../icons/audio.svg")),
    ("video", include_str!("../icons/video.svg")),
    ("text", include_str!("../icons/text.svg")),
    ("code", include_str!("../icons/code.svg")),
    ("script", include_str!("../icons/script.svg")),
    ("config", include_str!("../icons/config.svg")),
    ("exec", include_str!("../icons/exec.svg")),
    ("pdf", include_str!("../icons/pdf.svg")),
    ("font", include_str!("../icons/font.svg")),
    ("disk", include_str!("../icons/disk.svg")),
    ("table", include_str!("../icons/table.svg")),
    ("present", include_str!("../icons/present.svg")),
    ("doc", include_str!("../icons/doc.svg")),
    ("database", include_str!("../icons/database.svg")),
    ("calendar", include_str!("../icons/calendar.svg")),
    // sidebar place shortcuts
    ("downloads", include_str!("../icons/downloads.svg")),
    ("trash", include_str!("../icons/trash.svg")),
    // outlined sidebar variants (Dolphin-style line icons)
    ("home_o", include_str!("../icons/home_o.svg")),
    ("doc_o", include_str!("../icons/doc_o.svg")),
    ("downloads_o", include_str!("../icons/downloads_o.svg")),
    ("audio_o", include_str!("../icons/audio_o.svg")),
    ("video_o", include_str!("../icons/video_o.svg")),
    ("trash_o", include_str!("../icons/trash_o.svg")),
    ("disk_o", include_str!("../icons/disk_o.svg")),
    ("folder_o", include_str!("../icons/folder_o.svg")),
];

static CACHE: OnceLock<Mutex<HashMap<String, Arc<resvg::usvg::Tree>>>> = OnceLock::new();

fn get(name: &str, color: u32) -> Option<Arc<resvg::usvg::Tree>> {
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let key = format!("{name}#{:06x}", color & 0xff_ffff);
    let mut guard = cache.lock().unwrap();
    if let Some(t) = guard.get(&key) {
        return Some(t.clone());
    }
    let src = ICONS.iter().find(|(n, _)| *n == name)?.1;
    let svg = src.replace("{{c}}", &format!("#{:06x}", color & 0xff_ffff));
    let tree = resvg::usvg::Tree::from_data(svg.as_bytes(), &resvg::usvg::Options::default()).ok()?;
    let arc = Arc::new(tree);
    guard.insert(key, arc.clone());
    Some(arc)
}

/// Draw icon `name` with top-left at (x,y), `size`px on a side, tinted `color`.
/// Returns false if the icon is unknown or failed to parse (caller may fall
/// back to procedural art).
pub fn draw(app: &App, buf: &mut [u32], x: i32, y: i32, size: i32, name: &str, color: u32) -> bool {
    let Some(tree) = get(name, color) else {
        return false;
    };
    let size = size.max(1);
    let Some(mut pm) = tiny_skia::Pixmap::new(size as u32, size as u32) else {
        return false;
    };
    let view = tree.size();
    let scale = size as f32 / view.width().max(1.0);
    let tr = tiny_skia::Transform::from_scale(scale, scale);
    resvg::render(&tree, tr, &mut pm.as_mut());
    blit(app, buf, x, y, &pm);
    true
}

fn blit(app: &App, buf: &mut [u32], x: i32, y: i32, pm: &tiny_skia::Pixmap) {
    let bw = app.width as i32;
    let bh = app.height as i32;
    let (w, h) = (pm.width() as i32, pm.height() as i32);
    let data = pm.data();
    for yy in 0..h {
        let dy = y + yy;
        if dy < 0 || dy >= bh {
            continue;
        }
        let row = (dy as usize).wrapping_mul(bw as usize);
        for xx in 0..w {
            let dx = x + xx;
            if dx < 0 || dx >= bw {
                continue;
            }
            let o = (yy as usize * w as usize + xx as usize) * 4;
            let a = data[o + 3] as u32;
            if a == 0 {
                continue;
            }
            let dst = buf[row + dx as usize];
            if a >= 255 {
                // fully opaque: paint the icon color outright (the old code
                // kept the background pixel here, so solid icon areas never
                // showed and every icon rendered as a faint ghost)
                buf[row + dx as usize] = 0xff00_0000
                    | ((data[o] as u32) << 16)
                    | ((data[o + 1] as u32) << 8)
                    | data[o + 2] as u32;
                continue;
            }
            let inv = 255 - a;
            let r = (((dst >> 16) & 0xff) * inv + data[o] as u32 * a) / 255;
            let g = (((dst >> 8) & 0xff) * inv + data[o + 1] as u32 * a) / 255;
            let b = ((dst & 0xff) * inv + data[o + 2] as u32 * a) / 255;
            buf[row + dx as usize] = 0xff00_0000 | (r << 16) | (g << 8) | b;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    #[test]
    fn opaque_icon_pixels_paint_the_icon_color() {
        // Regression: blit used to keep the background pixel for fully opaque
        // (a == 255) icon pixels, so solid icon areas never painted.
        let cfg = Config::default();
        let mut app = App::new(cfg);
        app.width = 32;
        app.height = 32;
        let mut buf = vec![0xff00_0000 | 0x00_0000ff; 32 * 32]; // blue bg
        let ok = draw(&app, &mut buf, 4, 4, 24, "folder", 0xffffff);
        assert!(ok, "folder icon failed to draw");
        // Count how many pixels are the icon's white, not the blue bg.
        let white = buf.iter().filter(|p| **p == 0xff_ffffff).count();
        assert!(white > 20, "icon solid area did not paint: only {white} white px");
    }

    #[test]
    fn outlined_sidebar_icons_render() {
        // Dolphin-style line icons must parse and paint visible pixels at
        // sidebar size (they use stroke instead of fill).
        for name in ["home_o", "doc_o", "downloads_o", "audio_o", "video_o", "trash_o", "disk_o", "folder_o"] {
            let tree = get(name, 0xffffff).unwrap_or_else(|| panic!("{name}: parse failed"));
            let mut pm = tiny_skia::Pixmap::new(24, 24).unwrap();
            let view = tree.size();
            let scale = 24.0 / view.width();
            resvg::render(&tree, tiny_skia::Transform::from_scale(scale, scale), &mut pm.as_mut());
            let alpha = pm.data().chunks_exact(4).filter(|p| p[3] > 0).count();
            assert!(alpha > 10, "{name}: no visible pixels ({alpha})");
        }
    }

    #[test]
    fn all_icons_parse_and_rasterize() {
        for (name, _) in ICONS {
            let tree = get(name, 0xffffff).unwrap_or_else(|| panic!("{name}: parse failed"));
            let view = tree.size();
            assert!(view.width() > 0.0 && view.height() > 0.0, "{name}: empty viewBox");
            let mut pm = tiny_skia::Pixmap::new(24, 24).unwrap();
            let scale = 24.0 / view.width();
            resvg::render(&tree, tiny_skia::Transform::from_scale(scale, scale), &mut pm.as_mut());
            let alpha = pm.data().chunks_exact(4).filter(|p| p[3] > 0).count();
            assert!(alpha > 10, "{name}: no visible pixels ({alpha})");
        }
    }
}
