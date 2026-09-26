use crate::app::{App, InputMode};
use crate::entry::{Entry, EntryType};
use crate::layout;
use crate::tab::{SortKey, ViewMode};

// ------------------------------------------------------------------
// pixel primitives (ports of ui.c fill_rect / blend_rect / draw_line)
// ------------------------------------------------------------------

pub fn fill_rect(app: &App, buf: &mut [u32], x: i32, y: i32, w: i32, h: i32, color: u32) {
    let bw = app.width as i32;
    let bh = app.height as i32;
    if w <= 0 || h <= 0 {
        return;
    }
    let c = 0xFF000000 | color;
    for yy in y..y + h {
        if yy < 0 || yy >= bh {
            continue;
        }
        let row = yy * bw;
        for xx in x..x + w {
            if xx < 0 || xx >= bw {
                continue;
            }
            buf[(row + xx) as usize] = c;
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn blend_rect(app: &App, buf: &mut [u32], x: i32, y: i32, w: i32, h: i32, color: u32, alpha: u8) {
    let bw = app.width as i32;
    let bh = app.height as i32;
    if w <= 0 || h <= 0 || alpha == 0 {
        return;
    }
    let cr = ((color >> 16) & 0xFF) as i32;
    let cg = ((color >> 8) & 0xFF) as i32;
    let cb = (color & 0xFF) as i32;
    for yy in y..y + h {
        if yy < 0 || yy >= bh {
            continue;
        }
        let row = yy * bw;
        for xx in x..x + w {
            if xx < 0 || xx >= bw {
                continue;
            }
            let dst = buf[(row + xx) as usize];
            let dr = ((dst >> 16) & 0xFF) as i32;
            let dg = ((dst >> 8) & 0xFF) as i32;
            let db = (dst & 0xFF) as i32;
            let r = (cr * alpha as i32 + dr * (255 - alpha as i32)) >> 8;
            let g = (cg * alpha as i32 + dg * (255 - alpha as i32)) >> 8;
            let b = (cb * alpha as i32 + db * (255 - alpha as i32)) >> 8;
            buf[(row + xx) as usize] = 0xFF000000 | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32);
        }
    }
}

/// Baseline Y for a row starting at `row_y` with height `row_h`.
/// Places the em-box inside the row (ascent from top padding).
fn text_baseline(app: &App, row_y: i32, row_h: i32) -> i32 {
    let a = layout::ascent(app);
    // prefer a small top inset; never push baseline past row bottom - 1
    row_y + a.min(row_h.saturating_sub(2).max(1))
}

pub fn dim_color(c: u32) -> u32 {
    let r = ((c >> 16) & 0xFF) * 2 / 3;
    let g = ((c >> 8) & 0xFF) * 2 / 3;
    let b = (c & 0xFF) * 2 / 3;
    (r << 16) | (g << 8) | b
}

pub fn draw_line(app: &App, buf: &mut [u32], x0: i32, y0: i32, x1: i32, y1: i32, color: u32) {
    let bw = app.width as i32;
    let bh = app.height as i32;
    let mut x0 = x0;
    let mut y0 = y0;
    let dx = (x1 - x0).abs();
    let dy = (y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx - dy;
    loop {
        if x0 >= 0 && x0 < bw && y0 >= 0 && y0 < bh {
            buf[(y0 * bw + x0) as usize] = 0xFF000000 | color;
        }
        if x0 == x1 && y0 == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 > -dy {
            err -= dy;
            x0 += sx;
        }
        if e2 < dx {
            err += dx;
            y0 += sy;
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn fill_triangle(app: &App, buf: &mut [u32], x0: i32, y0: i32, x1: i32, y1: i32, x2: i32, y2: i32, color: u32) {
    if y0 == y2 {
        let lo = x0.min(x1).min(x2);
        let hi = x0.max(x1).max(x2);
        fill_rect(app, buf, lo, y0, hi - lo + 1, 1, color);
        return;
    }
    let d01 = y1 - y0;
    let d02 = y2 - y0;
    let d12 = y2 - y1;
    for y in y0..=y2 {
        let xa = if y < y1 && d01 > 0 {
            x0 + (x1 - x0) * (y - y0) / d01
        } else if d12 > 0 {
            x1 + (x2 - x1) * (y - y1) / d12
        } else {
            x1
        };
        let xb = x0 + (x2 - x0) * (y - y0) / d02;
        let (lo, hi) = if xa > xb { (xb, xa) } else { (xa, xb) };
        fill_rect(app, buf, lo, y, hi - lo + 1, 1, color);
    }
}

// ------------------------------------------------------------------
// icons (ports of draw_icon_* in ui.c)
// ------------------------------------------------------------------

fn draw_icon_folder(app: &App, buf: &mut [u32], x: i32, y: i32, s: i32, color: u32) {
    let back = dim_color(color);
    fill_rect(app, buf, x, y + s * 3 / 16, s, s - s * 3 / 16, back);
    fill_rect(app, buf, x, y, s * 9 / 16, s * 5 / 16, color);
    fill_rect(app, buf, x, y + s / 4, s, s - s / 4, color);
}

fn draw_icon_file(app: &App, buf: &mut [u32], x: i32, y: i32, s: i32, color: u32) {
    let fold = dim_color(color);
    fill_rect(app, buf, x, y, s, s, color);
    fill_triangle(app, buf, x + s * 3 / 4, y, x + s, y, x + s, y + s / 4, fold);
    draw_line(app, buf, x + s * 3 / 4, y, x + s, y + s / 4, dim_color(fold));
    fill_rect(app, buf, x + s / 4, y + s * 2 / 4, s / 2, 1, fold);
    fill_rect(app, buf, x + s / 4, y + s * 2 / 4 + s / 6, s * 2 / 3, 1, fold);
}

fn draw_icon_image(app: &App, buf: &mut [u32], x: i32, y: i32, s: i32, color: u32) {
    let bg = app.cfg.bg;
    fill_rect(app, buf, x, y, s, s, color);
    fill_rect(app, buf, x, y + s - 1, s, 1, dim_color(color));
    fill_rect(app, buf, x, y, s, 1, dim_color(color));
    fill_triangle(app, buf, x + s / 6, y + s * 5 / 8, x + s / 2, y + s / 4, x + s * 5 / 6, y + s * 5 / 8, bg);
    fill_rect(app, buf, x + s * 5 / 8, y + s / 8, s / 4, s / 4, bg);
}

fn draw_icon_archive(app: &App, buf: &mut [u32], x: i32, y: i32, s: i32, color: u32) {
    let dark = dim_color(color);
    fill_rect(app, buf, x, y, s, s, color);
    draw_line(app, buf, x, y + s / 2, x + s, y + s / 2, dark);
    fill_rect(app, buf, x + s / 4, y + s / 8, s / 2, s / 8, dark);
    fill_rect(app, buf, x + s / 6, y + s * 3 / 4, s / 6, s / 8, dark);
    fill_rect(app, buf, x + s * 2 / 3, y + s * 3 / 4, s / 6, s / 8, dark);
}

/// File-type styles applied on top of the plain file sheet.
#[derive(Clone, Copy, PartialEq)]
enum FileStyle {
    Text,
    Code,
    Script,
    Config,
    Media,
}

/// Classify a regular file's icon style from its name / extension.
fn file_style(e: &Entry) -> Option<FileStyle> {
    if e.kind != EntryType::File {
        return None;
    }
    let lower = e.name.to_lowercase();
    let ext = lower.rsplit('.').next().unwrap_or("");
    match ext {
        "c" | "h" | "rs" | "go" | "js" | "ts" | "jsx" | "tsx" | "java" | "kt" | "swift"
        | "php" | "cs" | "cpp" | "cc" | "cxx" | "hpp" | "html" | "htm" | "css" | "scss"
        | "xml" | "json" | "toml" | "yaml" | "yml" | "sql" | "zig" | "ex" | "exs" | "erl"
        | "hs" | "clj" | "scala" | "ml" | "vue" | "svelte" | "qml" | "v" | "dart" => {
            Some(FileStyle::Code)
        }
        "sh" | "bash" | "zsh" | "fish" | "py" | "rb" | "pl" | "lua" | "r" | "awk" => {
            Some(FileStyle::Script)
        }
        "conf" | "cfg" | "ini" | "env" | "properties" | "rc" | "gitignore" => {
            Some(FileStyle::Config)
        }
        "mp3" | "wav" | "flac" | "ogg" | "m4a" | "aac" | "opus" | "mp4" | "mkv" | "avi"
        | "mov" | "webm" | "flv" | "wmv" | "mpg" | "mpeg" | "3gp" | "m4v" => {
            Some(FileStyle::Media)
        }
        "txt" | "md" | "log" | "rst" | "readme" | "license" | "copying" | "changelog"
        | "changes" | "todo" | "notes" | "list" | "makefile" | "cmakelists" => {
            Some(FileStyle::Text)
        }
        _ => None,
    }
}

/// `</>` glyph on the file sheet.
fn draw_icon_code(app: &App, buf: &mut [u32], x: i32, y: i32, s: i32, color: u32) {
    draw_icon_file(app, buf, x, y, s, color);
    let bg = app.cfg.bg;
    let cx = x + s / 2;
    let cy = y + s / 2;
    let lt = (s / 9).max(2);
    let arm = s / 7;
    draw_line(app, buf, cx - arm, cy, cx - arm - lt, cy - s / 9, bg);
    draw_line(app, buf, cx - arm, cy, cx - arm - lt, cy + s / 9, bg);
    draw_line(app, buf, cx + arm, cy, cx + arm + lt, cy - s / 9, bg);
    draw_line(app, buf, cx + arm, cy, cx + arm + lt, cy + s / 9, bg);
    draw_line(app, buf, cx - lt / 2, cy - s / 9, cx + lt / 2, cy + s / 9, bg);
}

/// `>_` glyph for interpreted scripts.
fn draw_icon_script(app: &App, buf: &mut [u32], x: i32, y: i32, s: i32, color: u32) {
    draw_icon_file(app, buf, x, y, s, color);
    let bg = app.cfg.bg;
    let cy = y + s / 2;
    let lt = (s / 9).max(2);
    let px = x + s / 3;
    draw_line(app, buf, px, cy, px + lt, cy - s / 9, bg);
    draw_line(app, buf, px, cy, px + lt, cy + s / 9, bg);
    let uw = s / 6;
    fill_rect(app, buf, x + s * 5 / 8 - uw / 2, cy + lt / 2, uw, lt, bg);
}

/// Dot (key-value) marker for config files.
fn draw_icon_config(app: &App, buf: &mut [u32], x: i32, y: i32, s: i32, color: u32) {
    draw_icon_file(app, buf, x, y, s, color);
    let bg = app.cfg.bg;
    let q = s / 7;
    let cxo = x + s / 2;
    let cyo = y + s / 2;
    fill_rect(app, buf, cxo - q, cyo - q, q * 2, q * 2, bg);
    fill_rect(app, buf, cxo - q / 2, cyo - q / 2, q, q, color);
}

/// Play triangle for audio / video files.
fn draw_icon_media(app: &App, buf: &mut [u32], x: i32, y: i32, s: i32, color: u32) {
    draw_icon_file(app, buf, x, y, s, color);
    let bg = app.cfg.bg;
    let mx = x + s * 11 / 24;
    let top = y + s * 2 / 5;
    let bot = y + s * 3 / 5;
    fill_triangle(app, buf, mx, top, mx, bot, x + s * 5 / 8, y + s / 2, bg);
}

/// Shortcut-arrow badge for symlinks. Dirs get it centered on the bottom
/// edge of the icon; files get it tucked into the bottom-right corner.
fn draw_icon_link_badge(app: &App, buf: &mut [u32], x: i32, y: i32, s: i32, color: u32, dir: bool) {
    let sy = y + s * 3 / 4;
    let bw = s / 4;
    let (bx, arrow_x) = if dir {
        // centered at the bottom edge
        (x + s / 2 - bw / 2, x + s / 2 + bw / 2)
    } else {
        // bottom-right corner
        (x + s - bw - 2, x + s - 2)
    };
    fill_rect(app, buf, bx, sy - 1, bw, 2, color);
    fill_triangle(app, buf, arrow_x - bw / 2, sy - 2, arrow_x - bw / 2, sy + 2, arrow_x, sy, color);
}

/// Small "H" badge (two names → one inode) at the icon's bottom-right.
fn draw_icon_hardlink_badge(app: &App, buf: &mut [u32], x: i32, y: i32, s: i32, color: u32) {
    let bw = 3;
    let bh = 5;
    let bx = x + s - bw - 1;
    let by = y + s - bh - 1;
    fill_rect(app, buf, bx, by, 1, bh, color);
    fill_rect(app, buf, bx + bw - 1, by, 1, bh, color);
    fill_rect(app, buf, bx, by + bh / 2, bw, 1, color);
}

/// SVG icon name + theme color for an entry (Dolphin-style per-type icons).
/// Symlinks keep the target's base icon (folder vs file) and get the arrow
/// badge drawn on top by `draw_entry_icon`.
/// Tint for the image icon — same glyph, a different palette color per
/// extension so .png / .jpg / .gif / … are distinguishable at a glance.
fn image_icon_color(app: &App, e: &crate::entry::Entry) -> u32 {
    let ext = e.name.rsplit('.').next().unwrap_or("");
    match ext.to_ascii_lowercase().as_str() {
        // raster formats
        "png" => app.cfg.icon_img,
        "jpg" | "jpeg" => app.cfg.icon_arc,
        "gif" => app.cfg.icon_dir,
        "webp" => app.cfg.icon_file,
        "bmp" => app.cfg.icon_img,
        "tiff" | "tif" => app.cfg.icon_arc,
        "ico" => app.cfg.icon_dir,
        "avif" | "heic" | "heif" => app.cfg.icon_file,
        // vector / raw
        "svg" => app.cfg.icon_dir,
        "raw" | "dng" | "nef" | "cr2" | "arw" => app.cfg.icon_arc,
        "psd" | "xcf" => app.cfg.icon_file,
        _ => app.cfg.icon_img,
    }
}

fn entry_icon(app: &App, e: &crate::entry::Entry) -> (&'static str, u32) {
    if e.is_link {
        return if e.is_dir { ("folder", app.cfg.icon_dir) } else { ("file", app.cfg.icon_file) };
    }
    match e.kind {
        EntryType::Dir => ("folder", app.cfg.icon_dir),
        EntryType::Image => ("image", image_icon_color(app, e)),
        EntryType::Archive => ("archive", app.cfg.icon_arc),
        EntryType::File => file_icon(app, e),
        EntryType::Link => ("file", app.cfg.icon_file),
    }
}

/// Pick the per-type icon (name + tint color) for a regular file from its
/// extension / name, mirroring what Dolphin shows for each kind of file.
fn file_icon(app: &App, e: &crate::entry::Entry) -> (&'static str, u32) {
    let lower = e.name.to_lowercase();
    let ext = lower.rsplit('.').next().unwrap_or("");
    let name = match ext {
        // source code
        "c" | "h" | "rs" | "go" | "js" | "ts" | "jsx" | "tsx" | "java" | "kt" | "swift"
        | "php" | "cs" | "cpp" | "cc" | "cxx" | "hpp" | "html" | "htm" | "css" | "scss"
        | "xml" | "json" | "toml" | "yaml" | "yml" | "zig" | "ex" | "exs" | "erl" | "hs"
        | "clj" | "scala" | "ml" | "vue" | "svelte" | "qml" | "v" | "dart" | "dockerfile"
        | "gitignore" | "gitattributes" | "editorconfig" => "code",
        // interpreted scripts
        "sh" | "bash" | "zsh" | "fish" | "py" | "rb" | "pl" | "lua" | "r" | "awk" | "ps1" => {
            "script"
        }
        // settings files
        "conf" | "cfg" | "ini" | "env" | "properties" | "rc" | "prefs" => "config",
        // audio
        "mp3" | "wav" | "flac" | "ogg" | "m4a" | "aac" | "opus" | "wma" | "mid" | "midi"
        | "ape" | "aiff" | "alac" => "audio",
        // video
        "mp4" | "mkv" | "avi" | "mov" | "webm" | "flv" | "wmv" | "mpg" | "mpeg" | "3gp"
        | "m4v" | "ogv" => "video",
        // archives / packages
        "zip" | "tar" | "gz" | "bz2" | "xz" | "7z" | "rar" | "zst" | "tgz" | "txz"
        | "tbz2" | "deb" | "rpm" | "apk" | "lz4" | "z" => "archive",
        // disk images
        "iso" | "img" | "dmg" | "vhd" | "qcow2" => "disk",
        // plain text
        "txt" | "md" | "log" | "rst" | "org" | "tex" | "readme" | "license" | "copying"
        | "changelog" | "changes" | "todo" | "notes" | "list" | "makefile" | "cmakelists" => {
            "text"
        }
        "pdf" => "pdf",
        // office documents
        "doc" | "docx" | "odt" | "rtf" | "pages" => "doc",
        "xls" | "xlsx" | "csv" | "ods" | "tsv" => "table",
        "ppt" | "pptx" | "odp" | "key" => "present",
        // fonts
        "ttf" | "otf" | "woff" | "woff2" | "eot" | "ttc" | "fon" => "font",
        // databases
        "db" | "sqlite" | "sqlite3" | "sql" | "mdb" | "accdb" => "database",
        // calendar / contacts
        "ics" | "vcf" | "ical" | "vcs" => "calendar",
        // fallbacks: executables, dotfiles, anything else
        _ if e.is_exec => "exec",
        _ if lower.starts_with('.') => "config",
        _ => "file",
    };
    let color = match name {
        "code" => app.cfg.icon_dir,
        "archive" | "script" | "exec" | "pdf" | "present" | "disk" => app.cfg.icon_arc,
        "audio" | "video" => app.cfg.icon_img,
        _ => app.cfg.icon_file,
    };
    (name, color)
}

fn draw_entry_icon(app: &App, buf: &mut [u32], x: i32, y: i32, s: i32, e: &crate::entry::Entry) {
    let (name, color) = entry_icon(app, e);
    if !crate::icons::draw(app, buf, x, y, s, name, color) {
        // SVG missing/failed to rasterize → fall back to procedural art
        let icon_col = match e.kind {
            EntryType::Dir => app.cfg.icon_dir,
            EntryType::Image => app.cfg.icon_img,
            EntryType::Archive => app.cfg.icon_arc,
            _ => app.cfg.icon_file,
        };
        match e.kind {
            EntryType::Dir => draw_icon_folder(app, buf, x, y, s, icon_col),
            EntryType::Image => draw_icon_image(app, buf, x, y, s, icon_col),
            EntryType::Archive => draw_icon_archive(app, buf, x, y, s, icon_col),
            EntryType::File => match file_style(e) {
                Some(FileStyle::Code) => draw_icon_code(app, buf, x, y, s, app.cfg.icon_dir),
                Some(FileStyle::Script) => draw_icon_script(app, buf, x, y, s, app.cfg.icon_arc),
                Some(FileStyle::Config) => draw_icon_config(app, buf, x, y, s, icon_col),
                Some(FileStyle::Media) => draw_icon_media(app, buf, x, y, s, app.cfg.icon_img),
                _ => draw_icon_file(app, buf, x, y, s, icon_col),
            },
            _ => draw_icon_file(app, buf, x, y, s, icon_col),
        }
    }
    if e.is_link {
        draw_icon_link_badge(app, buf, x, y, s, app.cfg.icon_link, e.is_dir);
    } else if e.is_hardlink {
        draw_icon_hardlink_badge(app, buf, x, y, s, app.cfg.icon_hardlink);
    }
}

fn icon_size(app: &App) -> i32 {
    layout::list_icon_size(app)
}

// ------------------------------------------------------------------
// bars
// ------------------------------------------------------------------

fn draw_menu_bar(app: &App, buf: &mut [u32]) {
    let th = layout::menu_bar_h(app);
    fill_rect(app, buf, 0, 0, app.width as i32, th, app.cfg.dim);
    let mut x = app.cfg.padding;
    let base = text_baseline(app, 0, th);
    for (i, lab) in layout::RIBBON.iter().enumerate() {
        let w = app.text.as_ref().map(|t| t.width(lab) as i32).unwrap_or(40) + app.cfg.padding * 2;
        let active = app.menu.as_ref().map(|m| !m.is_context && app.ribbon_which == i as i32).unwrap_or(false);
        if active {
            fill_rect(app, buf, x, 1, w, th - 2, app.cfg.tab_active);
        }
        if let Some(text) = &app.text {
            text.draw(buf, app.width, app.height, x + app.cfg.padding, base, lab, app.cfg.fg);
        }
        x += w + 4;
    }
}

fn draw_tabs(app: &App, buf: &mut [u32]) {
    let lh = layout::line_h(app);
    let th = lh + app.cfg.padding;
    let y0 = layout::tab_bar_y(app);
    fill_rect(app, buf, 0, y0, app.width as i32, th, app.cfg.dim);
    let mut x = app.cfg.padding;
    let hover_close = layout::tab_close_at(app, app.ptr_x, app.ptr_y);
    for (i, t) in app.tabs.iter().enumerate() {
        let base = t.cwd.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| t.cwd.display().to_string());
        let label = format!(" {} {}{}", i + 1, base, if t.loading { " …" } else { "" });
        let w = tab_width(app, t);
        let bg = if i == app.cur_tab { app.cfg.tab_active } else { app.cfg.tab_idle };
        fill_rect(app, buf, x, y0, w, th, bg);
        if let Some(text) = &app.text {
            let base = text_baseline(app, y0, th);
            text.draw(buf, app.width, app.height, x + app.cfg.padding, base, &label, app.cfg.fg);
            // per-tab close (X) button at the right edge
            if let Some((cx, cy, s, _sh)) = layout::tab_close_rect(app, i) {
                let c = if hover_close == Some(i) {
                    app.cfg.sel_fg
                } else if i == app.cur_tab {
                    app.cfg.fg
                } else {
                    app.cfg.dim
                };
                draw_tab_close(app, buf, cx, cy, s, c);
            }
        }
        x += w + 2;
        if x > app.width as i32 {
            break;
        }
    }
}

/// Small X glyph for the per-tab close button.
fn draw_tab_close(app: &App, buf: &mut [u32], cx: i32, cy: i32, s: i32, color: u32) {
    let m = s / 4;
    draw_line(app, buf, cx + m, cy + m, cx + s - m, cy + s - m, color);
    draw_line(app, buf, cx + s - m, cy + m, cx + m, cy + s - m, color);
}

pub fn tab_width(app: &App, t: &crate::tab::Tab) -> i32 {
    let base = t.cwd.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let w = app.text.as_ref().map(|text| text.width(&format!(" {} {}", t.id, base))).unwrap_or(80.0) as i32;
    w + app.cfg.padding * 4 + 24
}

fn draw_nav_symbol(app: &App, buf: &mut [u32], cx: i32, cy: i32, which: i32, s: i32, color: u32) {
    let h = s / 2;
    match which {
        0 => fill_triangle(app, buf, cx - h, cy, cx + h, cy - h, cx + h, cy + h, color),
        1 => fill_triangle(app, buf, cx + h, cy, cx - h, cy - h, cx - h, cy + h, color),
        2 => fill_triangle(app, buf, cx, cy - h, cx - h, cy + h, cx + h, cy + h, color),
        3 => {
            // house: roof + body
            fill_triangle(app, buf, cx, cy - h, cx - h, cy, cx + h, cy, color);
            fill_rect(app, buf, cx - h * 2 / 3, cy, h * 4 / 3, h, color);
        }
        _ => {}
    }
}

fn draw_toolbar_items(app: &App, buf: &mut [u32]) {
    let t = &app.tabs[app.cur_tab];
    let view = t.view;
    let split_on = layout::split_active(app);
    for (i, it) in layout::toolbar_visible(app) {
        let Some((rx, ry, rw, rh)) = layout::toolbar_item_rect(app, i) else {
            continue;
        };
        let cy = ry + rh / 2;
        match &it {
            crate::toolbar::ToolbarItem::Back
            | crate::toolbar::ToolbarItem::Forward
            | crate::toolbar::ToolbarItem::Up
            | crate::toolbar::ToolbarItem::Home => {
                let which = match it {
                    crate::toolbar::ToolbarItem::Back => 0,
                    crate::toolbar::ToolbarItem::Forward => 1,
                    crate::toolbar::ToolbarItem::Up => 2,
                    _ => 3,
                };
                let enabled = match which {
                    0 => t.hist_pos > 0,
                    1 => (t.hist_pos as usize) + 1 < t.hist.len(),
                    _ => true,
                };
                let bg = if !enabled {
                    app.cfg.dim
                } else if app.nav_hover == which {
                    app.cfg.tab_active
                } else {
                    app.cfg.tab_idle
                };
                fill_rect(app, buf, rx, ry, rw, rh, bg);
                let col = if enabled { app.cfg.fg } else { app.cfg.status_c };
                let s = (rw.min(rh) * 2 / 3).max(6);
                let sx = rx + (rw - s) / 2;
                let sy = ry + (rh - s) / 2;
                let names = ["back", "forward", "up", "home"];
                if !crate::icons::draw(app, buf, sx, sy, s, names[which as usize], col) {
                    draw_nav_symbol(app, buf, rx + rw / 2, cy, which, s, col);
                }
            }
            crate::toolbar::ToolbarItem::Split => {
                fill_rect(
                    app,
                    buf,
                    rx,
                    ry,
                    rw,
                    rh,
                    if split_on { app.cfg.tab_active } else { app.cfg.tab_idle },
                );
                let scol = if split_on { app.cfg.sel_fg } else { app.cfg.fg };
                if !crate::icons::draw(app, buf, rx + rw / 2 - 8, cy - 8, 16, "split", scol) {
                    // fallback: two side-by-side panes
                    fill_rect(app, buf, rx + rw / 2 - 4, cy - 3, 2, 6, scol);
                    fill_rect(app, buf, rx + rw / 2 - 1, cy - 3, 1, 6, scol);
                    fill_rect(app, buf, rx + rw / 2 + 2, cy - 3, 2, 6, scol);
                }
            }
            crate::toolbar::ToolbarItem::Search => {
                fill_rect(app, buf, rx, ry, rw, rh, app.cfg.tab_idle);
                draw_search_icon(app, buf, rx + rw / 2, cy, app.cfg.status_c);
            }
            crate::toolbar::ToolbarItem::List
            | crate::toolbar::ToolbarItem::Grid
            | crate::toolbar::ToolbarItem::Compact => {
                let which = match it {
                    crate::toolbar::ToolbarItem::List => 0,
                    crate::toolbar::ToolbarItem::Grid => 1,
                    _ => 2,
                };
                let active = matches!(
                    (which, view),
                    (0, crate::tab::ViewMode::List)
                        | (1, crate::tab::ViewMode::Grid)
                        | (2, crate::tab::ViewMode::Compact)
                );
                let bg = if active { app.cfg.tab_active } else { app.cfg.tab_idle };
                fill_rect(app, buf, rx, ry, rw, rh, bg);
                let col = if active { app.cfg.sel_fg } else { app.cfg.fg };
                draw_view_icon(app, buf, rx + rw / 2, cy, which, col);
            }
            crate::toolbar::ToolbarItem::Separator => {
                let mx = rx + rw / 2;
                fill_rect(app, buf, mx, ry + 4, 1, rh - 8, app.cfg.status_c);
            }
            crate::toolbar::ToolbarItem::Custom(label) => {
                fill_rect(app, buf, rx, ry, rw, rh, app.cfg.tab_idle);
                if let Some(text) = &app.text {
                    let tw = text.width(label) as i32;
                    let bx = rx + (rw - tw) / 2;
                    let base = ry + app.cfg.padding / 2 + layout::ascent(app);
                    text.draw(buf, app.width, app.height, bx, base, label, app.cfg.fg);
                }
            }
            crate::toolbar::ToolbarItem::Location => {}
        }
    }
}

fn draw_path(app: &App, buf: &mut [u32]) {
    let lh = layout::line_h(app);
    let y0 = layout::path_bar_y(app);
    let ph = lh + app.cfg.padding * 2;
    fill_rect(app, buf, 0, y0, app.width as i32, ph, app.cfg.dim);
    let t = &app.tabs[app.cur_tab];
    let base = y0 + app.cfg.padding / 2 + layout::ascent(app);

    draw_toolbar_items(app, buf);

    // location tray (Thunar-style URL bar between nav and view buttons)
    let tray_left = layout::toolbar_left_end(app);
    let tray_right = layout::toolbar_right_start(app) - app.cfg.padding;
    let tray_w = (tray_right - tray_left).max(40);
    fill_rect(app, buf, tray_left, y0 + 3, tray_w, ph - 6, app.cfg.input_bg);
    fill_rect(app, buf, tray_left, y0 + 3, tray_w, 1, app.cfg.thumb_c);
    fill_rect(app, buf, tray_left, y0 + ph - 4, tray_w, 1, app.cfg.thumb_c);

    // Thunar-style inline search: editable field in the location bar.
    if app.input_mode == InputMode::Search {
        let mut x = tray_left + app.cfg.padding / 2;
        if let Some(text) = &app.text {
            if !app.input_prompt.is_empty() {
                x += text.draw(
                    buf,
                    app.width,
                    app.height,
                    x,
                    base,
                    &app.input_prompt,
                    app.cfg.dir,
                ) as i32;
                x += app.cfg.padding / 2;
            }
            let clip = tray_left + tray_w - app.cfg.padding;
            let caret_text = &app.input[..app.input_cursor.min(app.input.len())];
            let before_w = text.width(caret_text) as i32;
            text.draw_clip(
                buf,
                app.width,
                app.height,
                x,
                base,
                &app.input,
                app.cfg.fg,
                clip,
            );
            let cx = (x + before_w).min(clip - 2);
            fill_rect(app, buf, cx, y0 + app.cfg.padding, 2, lh, app.cfg.sel_bg);
        }
        // Red close X at the right end of the tray.
        let (rx, ry, rw, rh) = layout::search_close_rect(app);
        let red = 0x000000ff | 0xff000000; // opaque red
        let cxx = rx + rw / 2;
        let cyy = ry + rh / 2;
        let half = (rw.min(rh) / 2 - 2).max(3) as i32;
        for d in -half..=half {
            fill_rect(app, buf, cxx - d, cyy - d, 1, 1, red);
            fill_rect(app, buf, cxx - d, cyy - d - 1, 1, 1, red);
            fill_rect(app, buf, cxx + d, cyy - d, 1, 1, red);
            fill_rect(app, buf, cxx + d, cyy - d - 1, 1, 1, red);
        }
        return;
    }

    // Inline editable location bar (tray click / Ctrl+L): full URL text with
    // selection highlight, caret and a red close X.
    if app.input_mode == InputMode::Url {
        let f = layout::url_field_rect(app);
        let pad = app.cfg.padding;
        let text_x = f.x + pad / 2;
        let clip = f.x + f.w - 2;
        let (lo, hi) = app.url_range();
        let n = app.input.len();
        let lo = lo.min(n);
        let hi = hi.min(n);
        if let Some(text) = &app.text {
            if hi > lo {
                let sel_w = text.width(&app.input[lo..hi]) as i32;
                let sel_x = text_x + text.width(&app.input[..lo]) as i32;
                if sel_x < clip {
                    let right = (sel_x + sel_w).min(clip);
                    fill_rect(app, buf, sel_x, f.y + pad, right - sel_x, f.h - pad * 2, app.cfg.sel_bg);
                }
            }
            let mut x = text_x;
            x += text.draw_clip(buf, app.width, app.height, x, base, &app.input[..lo], app.cfg.fg, clip) as i32;
            x += text.draw_clip(buf, app.width, app.height, x, base, &app.input[lo..hi], app.cfg.sel_fg, clip) as i32;
            text.draw_clip(buf, app.width, app.height, x, base, &app.input[hi..], app.cfg.fg, clip);
        }
        // caret (2px bar) at input_cursor
        if let Some(text) = &app.text {
            let caret_w = text.width(&app.input[..app.input_cursor.min(n)]) as i32;
            let cx = (text_x + caret_w).min(clip - 2);
            fill_rect(app, buf, cx, f.y + pad, 2, f.h - pad * 2, app.cfg.sel_bg);
        }
        // Red close X at the right end of the tray.
        let (rx, ry, rw, rh) = layout::search_close_rect(app);
        let red = 0x000000ff | 0xff000000; // opaque red
        let cxx = rx + rw / 2;
        let cyy = ry + rh / 2;
        let half = (rw.min(rh) / 2 - 2).max(3);
        for d in -half..=half {
            fill_rect(app, buf, cxx - d, cyy - d, 1, 1, red);
            fill_rect(app, buf, cxx - d, cyy - d - 1, 1, 1, red);
            fill_rect(app, buf, cxx + d, cyy - d, 1, 1, red);
            fill_rect(app, buf, cxx + d, cyy - d - 1, 1, 1, red);
        }
        // Tick/go button (opens the typed path), left of the X.
        let (gx, gy, gw, gh) = {
            let r = layout::url_go_rect(app);
            (r.x, r.y, r.w, r.h)
        };
        let gxx = gx + gw / 2;
        let gyy = gy + gh / 2;
        draw_line(app, buf, gxx - 3, gyy - 1, gxx - 1, gyy + 2, app.cfg.dir);
        draw_line(app, buf, gxx - 1, gyy + 2, gxx + 3, gyy - 2, app.cfg.dir);
        // Drop-down/history button (suggestions), left of the tick.
        let (dx, dy, dw, dh) = {
            let r = layout::url_drop_rect(app);
            (r.x, r.y, r.w, r.h)
        };
        let dxx = dx + dw / 2;
        let dyy = dy + dh / 2;
        draw_line(app, buf, dxx - 3, dyy - 1, dxx, dyy + 2, app.cfg.status_c);
        draw_line(app, buf, dxx, dyy + 2, dxx + 3, dyy - 1, app.cfg.status_c);
        return;
    }

    let mut bx = layout::breadcrumb_x0(app);

    if !t.filter.is_empty() {
        let fl = format!(
            "[filter: {}{}]  ",
            if t.filter_regex { "re " } else { "" },
            t.filter
        );
        if let Some(text) = &app.text {
            bx += text.draw(buf, app.width, app.height, bx, base, &fl, app.cfg.status_c) as i32;
        }
    }

    let cwd = t.cwd.display().to_string();
    let segs: Vec<&str> = if cwd == "/" {
        vec!["/"]
    } else {
        let mut v: Vec<&str> = cwd.split('/').filter(|s| !s.is_empty()).collect();
        v.insert(0, "/");
        v
    };
    let mut path_prefix = String::new();
    for (i, seg) in segs.iter().enumerate() {
        if i > 1 {
            let adv = app
                .text
                .as_ref()
                .map(|t| t.draw(buf, app.width, app.height, bx, base, "/", app.cfg.status_c))
                .unwrap_or(6.0) as i32;
            bx += adv;
        }
        if i == 0 {
            path_prefix = "/".to_string();
        } else {
            if !path_prefix.ends_with('/') {
                path_prefix.push('/');
            }
            path_prefix.push_str(seg);
        }
        if i == app.breadcrumb_hover as usize && app.breadcrumb_hover >= 0 {
            let seg_w = app.text.as_ref().map(|t| t.width(seg)).unwrap_or(40.0) as i32;
            fill_rect(app, buf, bx - 2, y0 + 2, seg_w + 4, ph - 4, app.cfg.sel_bg);
        }
        let col = if i == segs.len() - 1 {
            app.cfg.fg
        } else {
            app.cfg.dir
        };
        let adv = app
            .text
            .as_ref()
            .map(|t| t.draw(buf, app.width, app.height, bx, base, seg, col))
            .unwrap_or(0.0) as i32;
        bx += adv;
        if bx > app.width as i32 - app.cfg.padding - 120 {
            break;
        }
    }
}

/// Display name honoring the "Show File Extensions" toggle: folders and
/// dotfiles keep their full name; a file's last extension is stripped when
/// the toggle is off (Thunar-style). Real names still drive sort/copy/rename.
fn display_name(app: &App, e: &crate::entry::Entry) -> String {
    if app.cfg.show_ext || e.is_dir {
        return e.name.clone();
    }
    let name = e.name.as_str();
    if name.starts_with('.') {
        return name.to_string();
    }
    match name.rsplit_once('.') {
        Some((stem, _)) if !stem.is_empty() => stem.to_string(),
        _ => name.to_string(),
    }
}

/// Mini view-mode icons: 0=list (lines), 1=grid (squares), 2=compact (tiles).
fn draw_view_icon(app: &App, buf: &mut [u32], cx: i32, cy: i32, mode: i32, color: u32) {
    let name = match mode {
        0 => "list",
        1 => "grid",
        _ => "compact",
    };
    if crate::icons::draw(app, buf, cx - 8, cy - 8, 16, name, color) {
        return;
    }
    match mode {
        0 => {
            fill_rect(app, buf, cx - 3, cy - 3, 6, 1, color);
            fill_rect(app, buf, cx - 3, cy, 6, 1, color);
            fill_rect(app, buf, cx - 3, cy + 3, 6, 1, color);
        }
        1 => {
            fill_rect(app, buf, cx - 3, cy - 3, 2, 2, color);
            fill_rect(app, buf, cx + 1, cy - 3, 2, 2, color);
            fill_rect(app, buf, cx - 3, cy + 1, 2, 2, color);
            fill_rect(app, buf, cx + 1, cy + 1, 2, 2, color);
        }
        _ => {
            fill_rect(app, buf, cx - 3, cy - 3, 2, 7, color);
            fill_rect(app, buf, cx, cy - 3, 3, 1, color);
            fill_rect(app, buf, cx, cy + 3, 3, 1, color);
        }
    }
}

/// Small magnifier for the filter/search button.
fn draw_search_icon(app: &App, buf: &mut [u32], cx: i32, cy: i32, color: u32) {
    if crate::icons::draw(app, buf, cx - 8, cy - 8, 16, "search", color) {
        return;
    }
    draw_line(app, buf, cx - 3, cy - 2, cx - 2, cy - 3, color);
    draw_line(app, buf, cx + 2, cy - 3, cx + 3, cy - 2, color);
    draw_line(app, buf, cx - 3, cy + 2, cx - 2, cy + 3, color);
    draw_line(app, buf, cx + 2, cy + 3, cx + 3, cy + 2, color);
    fill_rect(app, buf, cx - 3, cy, 1, 1, color);
    fill_rect(app, buf, cx + 2, cy, 1, 1, color);
    draw_line(app, buf, cx + 1, cy + 4, cx + 4, cy + 7, color);
}

fn draw_headers(app: &App, buf: &mut [u32], t: &crate::tab::Tab, x0: i32, w: i32) {
    let lh = layout::line_h(app);
    let y0 = layout::header_y(app);
    let hh = lh + app.cfg.padding;
    fill_rect(app, buf, x0, y0, w, hh, app.cfg.dim);
    fill_rect(app, buf, x0, y0 + hh - 1, w, 1, app.cfg.thumb_c);
    let base = y0 + app.cfg.padding / 2 + layout::ascent(app);
    let (x_size, x_mtime) = header_cols(app, x0, w);
    let show_meta = x_size < x0 + w - app.cfg.padding;
    let cols = [
        ("Name", SortKey::Name, x0 + app.cfg.padding, false),
        (
            if show_meta { "Size" } else { "" },
            SortKey::Size,
            x_size,
            true,
        ),
        (
            if show_meta { "Modified" } else { "" },
            SortKey::Mtime,
            x_mtime,
            true,
        ),
    ];
    for (label, key, x, right_align) in cols {
        if label.is_empty() {
            continue;
        }
        let active = t.sort_key == key;
        let text_str = if active {
            format!("{} {}", label, if t.sort_desc { "v" } else { "^" })
        } else {
            label.to_string()
        };
        let mut dx = x;
        if right_align {
            let tw = app.text.as_ref().map(|t| t.width(&text_str)).unwrap_or(40.0) as i32;
            dx = x - tw;
        }
        let color = if active { app.cfg.dir } else { app.cfg.status_c };
        if let Some(text) = &app.text {
            text.draw(buf, app.width, app.height, dx, base, &text_str, color);
        }
    }
}

pub fn header_cols(app: &App, x0: i32, w: i32) -> (i32, i32) {
    let pad = app.cfg.padding;
    let mw = app.text.as_ref().map(|t| t.width("0000-00-00 00:00")).unwrap_or(120.0) as i32;
    let size_w = 96;
    // Reserve icon + a sane minimum name width; if the pane is too narrow for
    // name + size + mtime, push the meta columns off the right edge so the
    // name column never gets squeezed (Dolphin behaviour).
    let need = size_w + mw + 96 + icon_size(app) + pad * 4 + pad / 2;
    let show_meta = w >= need;
    let xs = if show_meta { x0 + w - pad - size_w } else { x0 + w };
    let x_mtime = if show_meta { xs - pad - mw } else { x0 + w };
    (xs, x_mtime)
}

fn draw_list(app: &App, buf: &mut [u32], t: &crate::tab::Tab, x0: i32, w: i32, hover: i32) {
    let lh = layout::row_h(app);
    let y0 = layout::list_y_for(app, t);
    let vis = layout::visible_rows_for(app, t);
    let pad = app.cfg.padding;
    let (_, x_mtime, mtime_w, show_meta) = {
        let (xs, xm) = header_cols(app, x0, w);
        (
            xs,
            xm,
            app.text.as_ref().map(|t| t.width("0000-00-00 00:00")).unwrap_or(120.0) as i32,
            xs < x0 + w - pad,
        )
    };
    draw_headers(app, buf, t, x0, w);
    draw_headers(app, buf, t, x0, w);
    for i in 0..vis {
        let vi = t.scroll + i;
        if vi >= t.nvis() {
            break;
        }
        let e = &t.entries[t.vis[vi]];
        let row_y = y0 + i as i32 * lh;
        let selected = !t.sel_none && vi == t.sel;
        let mut color = if e.is_dir {
            app.cfg.dir
        } else {
            match e.kind {
                EntryType::Image => app.cfg.type_img,
                EntryType::Archive => app.cfg.type_arc,
                EntryType::Dir => app.cfg.type_dir,
                _ => app.cfg.type_file,
            }
        };
        if selected {
            fill_rect(app, buf, x0, row_y, w, lh, app.cfg.sel_bg);
            color = app.cfg.sel_fg;
        } else if e.selected {
            fill_rect(app, buf, x0, row_y, w, lh, app.cfg.tab_idle);
        } else if vi as i32 == hover {
            blend_rect(app, buf, x0, row_y, w, lh, app.cfg.tab_idle, 80);
        }
        if app.dnd_active && vi as i32 == app.dnd_hover_row {
            blend_rect(app, buf, x0, row_y, w, lh, app.cfg.sel_bg, 110);
        }
        let nm = if e.is_dir {
            format!("{}/", display_name(app, e))
        } else {
            display_name(app, e)
        };
        let isz = icon_size(app);
        draw_entry_icon(app, buf, x0 + pad, row_y + (lh - isz) / 2, isz, e);
        let tx = x0 + pad + isz + pad / 2;
        let base = text_baseline(app, row_y, lh);
        // name clips before size/mtime columns; never clip to the left of the pen
        let name_clip = (x_mtime - pad).max(tx + 8);
        let dim = if selected { app.cfg.sel_fg } else { app.cfg.status_c };
        if let Some(text) = &app.text {
            text.draw_clip(buf, app.width, app.height, tx, base, &nm, color, name_clip);
        }
        if show_meta && !e.is_dir && !t.filter_regex {
            let sz = crate::config::fmt_size(e.size as i64);
            let sw = app.text.as_ref().map(|t| t.width(&sz)).unwrap_or(30.0) as i32;
            if let Some(text) = &app.text {
                text.draw(buf, app.width, app.height, x0 + w - pad - sw, base, &sz, dim);
            }
        }
        if show_meta {
            let mt = fmt_mtime(e.mtime);
            let mw = app.text.as_ref().map(|t| t.width(&mt)).unwrap_or(100.0) as i32;
            if let Some(text) = &app.text {
                text.draw(buf, app.width, app.height, x_mtime + (mtime_w - mw), base, &mt, dim);
            }
        }
    }
    draw_scrollbar(app, buf, t, x0, w);
}

pub fn fmt_mtime(t: i64) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let Some(dur) = SystemTime::UNIX_EPOCH.checked_add(std::time::Duration::from_secs(t.max(0) as u64))
        .and_then(|s| s.duration_since(UNIX_EPOCH).ok()) else { return "?".into() };
    let secs = dur.as_secs() as i64;
    let (y, mo, d, h, mi) = break_ymd(secs);
    format!("{y:04}-{mo:02}-{d:02} {h:02}:{mi:02}")
}

// civil-from-days + time of day (no libc); ~1970..2100 correctness is fine
fn break_ymd(secs: i64) -> (i64, i64, i64, i64, i64) {
    let days = secs.div_euclid(86400);
    let tod = secs.rem_euclid(86400);
    let h = tod / 3600;
    let mi = (tod % 3600) / 60;
    let (y, m, d) = civil_from_days(days);
    (y, m, d, h, mi)
}

fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719468;
    let era = z.div_euclid(146097);
    let doe = z.rem_euclid(146097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

fn draw_scrollbar(app: &App, buf: &mut [u32], t: &crate::tab::Tab, x0: i32, w: i32) {
    if let Some((sx, sy, sh)) = layout::scrollbar_geom_for(app, t, x0, w) {
        let y0 = layout::list_y(app);
        let track_h = layout::list_h(app);
        fill_rect(app, buf, sx, y0, 8, track_h, app.cfg.dim);
        fill_rect(app, buf, sx + 1, sy, 6, sh, app.cfg.thumb_c);
    }
}

fn draw_status(app: &App, buf: &mut [u32]) {
    let lh = layout::line_h(app);
    let top = app.height as i32 - layout::status_h(app);
    fill_rect(app, buf, 0, top, app.width as i32, layout::status_h(app), app.cfg.dim);
    fill_rect(app, buf, 0, top, app.width as i32, 1, app.cfg.thumb_c);
    let msg = if !app.err.is_empty() {
        app.err.clone()
    } else if !app.status.is_empty() {
        app.status.clone()
    } else {
        "wfm — pure Wayland".to_string()
    };
    let base = top + (layout::status_h(app) - lh) / 2 + layout::ascent(app);
    let color = if !app.err.is_empty() { app.cfg.dir } else { app.cfg.status_c };
    if let Some(text) = &app.text {
        text.draw_clip(buf, app.width, app.height, app.cfg.padding, base, &msg, color, app.width as i32 - 260);
    }

    let mut right = String::new();
    let t = &app.tabs[app.cur_tab];
    if app.n_sel > 0 {
        right.push_str(&format!("{} items ({} selected)", t.nvis(), app.n_sel));
    } else {
        right.push_str(&format!("{} items", t.nvis()));
    }
    if app.free_bytes >= 0 {
        right.push_str(&format!(" · free {}", crate::config::fmt_size(app.free_bytes)));
    }
    let sw = app.text.as_ref().map(|t| t.width(&right)).unwrap_or(60.0) as i32;
    if let Some(text) = &app.text {
        text.draw(buf, app.width, app.height, app.width as i32 - app.cfg.padding - sw, base, &right, app.cfg.status_c);
    }
}

fn draw_input(app: &App, buf: &mut [u32]) {
    let Some(g) = layout::input_dlg_geom(app) else { return };
    let lh = layout::line_h(app);
    let pad = app.cfg.padding;
    let win_w = app.width as i32;
    let win_h = app.height as i32;
    let title = match app.input_mode {
        InputMode::Rename => "Rename",
        InputMode::NewDir | InputMode::DndNewDir => "New Folder",
        InputMode::NewFile => "New File",
        InputMode::Location => "Go to Location",
        InputMode::Url => "Location",
        InputMode::Filter => "Filter",
        InputMode::NewWindow => "New Window",
        InputMode::OpenWith => "Open With",
        InputMode::SetDefault => "Default App",
        InputMode::Search => "Search",
        InputMode::None => return,
    };

    // fully hide the window content behind the dialog (no see-through)
    fill_rect(app, buf, 0, 0, win_w, win_h, app.cfg.bg);

    // dialog chrome
    fill_rect(app, buf, g.x - 2, g.y - 2, g.w + 4, g.h + 4, app.cfg.thumb_c);
    fill_rect(app, buf, g.x, g.y, g.w, g.h, app.cfg.input_bg);
    fill_rect(app, buf, g.x, g.y, g.w, 3, app.cfg.dir);

    if let Some(text) = &app.text {
        text.draw(buf, app.width, app.height, g.x + pad, g.y + pad + layout::ascent(app), title, app.cfg.fg);
    }

    // field background
    let f = g.field;
    fill_rect(app, buf, f.x, f.y, f.w, f.h, app.cfg.dim);
    fill_rect(app, buf, f.x, f.y, f.w, 1, app.cfg.thumb_c);
    fill_rect(app, buf, f.x, f.y + f.h - 1, f.w, 1, app.cfg.thumb_c);

    let mut x = f.x + pad / 2;
    let base = f.y + pad / 2 + layout::ascent(app);
    if let Some(text) = &app.text {
        if !app.input_prompt.is_empty() {
            x += text.draw(
                buf,
                app.width,
                app.height,
                x,
                base,
                &app.input_prompt,
                app.cfg.status_c,
            ) as i32;
            x += pad / 2;
        }
        // draw full input clipped to field
        let clip = f.x + f.w - pad / 2;
        let caret_text = &app.input[..app.input_cursor.min(app.input.len())];
        let before_w = text.width(caret_text) as i32;
        text.draw_clip(
            buf,
            app.width,
            app.height,
            x,
            base,
            &app.input,
            app.cfg.fg,
            clip,
        );
        // caret
        let cx = (x + before_w).min(clip - 2);
        fill_rect(app, buf, cx, f.y + pad / 2, 2, lh, app.cfg.sel_bg);
        // hint between field and buttons
        text.draw(
            buf,
            app.width,
            app.height,
            g.x + pad,
            f.y + f.h + pad / 2 + layout::ascent(app),
            "Esc cancel  ·  Enter OK",
            app.cfg.status_c,
        );
    }

    // OK / Cancel buttons
    for (rect, label, accent) in [(&g.btn_cancel, "Cancel", false), (&g.btn_ok, "OK", true)] {
        fill_rect(
            app,
            buf,
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            if accent { app.cfg.tab_active } else { app.cfg.tab_idle },
        );
        fill_rect(app, buf, rect.x, rect.y, rect.w, 1, app.cfg.thumb_c);
        if let Some(text) = &app.text {
            let lw = text.width(label) as i32;
            text.draw(
                buf,
                app.width,
                app.height,
                rect.x + (rect.w - lw) / 2,
                rect.y + (rect.h - lh) / 2 + layout::ascent(app),
                label,
                app.cfg.fg,
            );
        }
    }
}

// ------------------------------------------------------------------
// main entry
// ------------------------------------------------------------------


fn draw_side_free(app: &App, buf: &mut [u32], sw: i32, y: i32, free_bytes: i64) {
    if free_bytes < 0 {
        return;
    }
    let sz = crate::config::fmt_size(free_bytes);
    let w = app.text.as_ref().map(|t| t.width(&sz)).unwrap_or(40.0) as i32;
    if let Some(text) = &app.text {
        text.draw(
            buf,
            app.width,
            app.height,
            sw - app.cfg.padding - w,
            y + app.cfg.padding / 2 + layout::ascent(app),
            &sz,
            app.cfg.status_c,
        );
    }
}


/// Soft <-> cue on the splitter (shown always; brighter while dragging).
fn draw_resize_arrows(app: &App, buf: &mut [u32], cx: i32, cy: i32, col: u32) {
    // small horizontal double-arrow: <—>
    let s = 6i32;
    // left chevron
    fill_rect(app, buf, cx - s - 2, cy - 1, 2, 3, col);
    fill_rect(app, buf, cx - s, cy - 3, 2, 2, col);
    fill_rect(app, buf, cx - s, cy + 2, 2, 2, col);
    // shaft
    fill_rect(app, buf, cx - s + 2, cy, s * 2 - 2, 2, col);
    // right chevron
    fill_rect(app, buf, cx + s, cy - 1, 2, 3, col);
    fill_rect(app, buf, cx + s - 2, cy - 3, 2, 2, col);
    fill_rect(app, buf, cx + s - 2, cy + 2, 2, 2, col);
}

fn path_base(p: &std::path::Path) -> String {
    p.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| p.display().to_string())
}

fn side_label_color(app: &App, kind: &layout::SideKind) -> u32 {
    match kind {
        layout::SideKind::Place(0) | layout::SideKind::Place(6) => app.cfg.dir,
        layout::SideKind::Place(_) => app.cfg.fg,
        layout::SideKind::Network(_) | layout::SideKind::Bookmark(_) => app.cfg.dir,
        // mounted volumes / removable: strong fg text; unmounted: 50% opacity
        layout::SideKind::Volume(_) => app.cfg.fg,
        layout::SideKind::Unmounted(_) => blend_colors(app.cfg.fg, app.cfg.dim, 128),
        layout::SideKind::Removable(i) => {
            if app.removable_mounted.get(*i).copied().unwrap_or(false) {
                app.cfg.fg
            } else {
                blend_colors(app.cfg.fg, app.cfg.dim, 128)
            }
        }
        _ => app.cfg.status_c,
    }
}

/// Icon name for a sidebar row — outlined (Dolphin-style line icons) for the
/// places, volumes and removable sections. "" = no icon.
fn side_icon(kind: &layout::SideKind) -> &'static str {
    match kind {
        layout::SideKind::Place(0) => "home_o",
        layout::SideKind::Place(1) => "doc_o",
        layout::SideKind::Place(2) => "downloads_o",
        layout::SideKind::Place(3) => "audio_o",
        layout::SideKind::Place(4) => "video_o",
        layout::SideKind::Place(5) => "trash_o",
        layout::SideKind::Place(_) => "disk_o",
        layout::SideKind::Volume(_) | layout::SideKind::Unmounted(_) | layout::SideKind::Network(_)
        | layout::SideKind::Removable(_) => "disk_o",
        layout::SideKind::Bookmark(_) => "folder_o",
        _ => "",
    }
}

/// Blend `fg` over `bg` at alpha 0..255 (for 50%-opacity labels etc.).
fn blend_colors(fg: u32, bg: u32, a: u32) -> u32 {
    let a = a.min(255);
    let fr = (fg >> 16) & 0xFF;
    let fg2 = (fg >> 8) & 0xFF;
    let fb = fg & 0xFF;
    let br = (bg >> 16) & 0xFF;
    let bg2 = (bg >> 8) & 0xFF;
    let bb = bg & 0xFF;
    let r = (fr * a + br * (255 - a)) / 255;
    let g = (fg2 * a + bg2 * (255 - a)) / 255;
    let b = (fb * a + bb * (255 - a)) / 255;
    0xFF000000 | (r << 16) | (g << 8) | b
}

/// Nearest-neighbor scale an RGBA image (src_w×src_h) into a w×h box.
fn blit_rgba(
    app: &App,
    buf: &mut [u32],
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    rgba: &[u8],
    src_w: i32,
    src_h: i32,
) {
    if w <= 0 || h <= 0 || src_w <= 0 || src_h <= 0 {
        return;
    }
    if (src_w as u64) * (src_h as u64) * 4 > rgba.len() as u64 {
        return;
    }
    let bw = app.width as i32;
    let bh = app.height as i32;
    let swu = src_w as u64;
    let shu = src_h as u64;
    let wu = w as u64;
    let hu = h as u64;
    for yy in 0..h {
        let dy = y + yy;
        if dy < 0 || dy >= bh {
            continue;
        }
        let src_y = (yy as u64 * shu / hu) as usize;
        let row = (dy * bw) as usize;
        for xx in 0..w {
            let dx = x + xx;
            if dx < 0 || dx >= bw {
                continue;
            }
            let src_x = (xx as u64 * swu / wu) as usize;
            let si = (src_y * src_w as usize + src_x) * 4;
            let a = rgba[si + 3] as u32;
            let sr = rgba[si] as u32;
            let sg = rgba[si + 1] as u32;
            let sb = rgba[si + 2] as u32;
            let out = &mut buf[row + dx as usize];
            if a >= 255 {
                *out = 0xFF000000 | (sr << 16) | (sg << 8) | sb;
            } else if a > 0 {
                let dst = *out;
                let dr = (dst >> 16) & 0xFF;
                let dg = (dst >> 8) & 0xFF;
                let db = dst & 0xFF;
                let r = (sr * a + dr * (255 - a)) / 255;
                let g = (sg * a + dg * (255 - a)) / 255;
                let b = (sb * a + db * (255 - a)) / 255;
                *out = 0xFF000000 | (r << 16) | (g << 8) | b;
            }
        }
    }
}

fn draw_sidebar(app: &App, buf: &mut [u32]) {
    if !app.sidebar_visible {
        return;
    }
    let sw = layout::content_x(app);
    let y0 = layout::bar_h(app);
    let top = app.height as i32 - layout::status_h(app);
    let lh = layout::line_h(app);
    fill_rect(app, buf, 0, y0, sw, top - y0, app.cfg.dim);
    // resize grip: thicker / brighter when hovered or dragging
    let grip_w = if app.side_resize || app.side_resize_hover { 3 } else { 1 };
    let grip_col = if app.side_resize || app.side_resize_hover {
        app.cfg.sel_bg
    } else {
        app.cfg.thumb_c
    };
    fill_rect(app, buf, sw - grip_w, y0, grip_w, top - y0, grip_col);
    if app.side_resize || app.side_resize_hover {
        draw_resize_arrows(app, buf, sw, (y0 + top) / 2, app.cfg.sel_fg);
    }
    // DnD feedback: an external drag hovering the sidebar means "drop to add
    // a shortcut to Places".
    if app.dnd_hover && app.dnd_side_hover {
        blend_rect(app, buf, 0, y0, sw, top - y0, app.cfg.sel_bg, 36);
        if let Some(text) = &app.text {
            text.draw_clip(
                buf,
                app.width,
                app.height,
                app.cfg.padding,
                top - app.cfg.padding - layout::ascent(app),
                "Drop to add shortcut",
                app.cfg.sel_fg,
                (sw - app.cfg.padding).max(app.cfg.padding + 8),
            );
        }
    }
    let base_off = app.cfg.padding / 2 + layout::ascent(app);
    // vertical scrollbar when the sidebar content overflows
    if let Some((sx, sy, sh)) = layout::sidebar_scrollbar_geom(app) {
        let track_h = top - y0;
        fill_rect(app, buf, sx, y0, 8, track_h, app.cfg.dim);
        fill_rect(app, buf, sx + 1, sy, 6, sh, app.cfg.thumb_c);
    }
    for row in layout::sidebar_rows(app) {
        let y = row.y;
        if y + lh <= y0 {
            continue; // scrolled above the visible sidebar
        }
        if y + lh > top {
            break;
        }
        match &row.kind {
            layout::SideKind::Gap => {}
            layout::SideKind::Header(label) => {
                if let Some(text) = &app.text {
                    text.draw_bold(buf, app.width, app.height, app.cfg.padding, y + base_off, label, app.cfg.status_c);
                }
            }
            kind => {
                let id = layout::side_kind_id(kind);
                if app.side_hover == id {
                    fill_rect(app, buf, 0, y, sw, row.h, app.cfg.tab_idle);
                }
                let label = match kind {
                    layout::SideKind::Place(0) => "Home".to_string(),
                    layout::SideKind::Place(1) => "Documents".to_string(),
                    layout::SideKind::Place(2) => "Downloads".to_string(),
                    layout::SideKind::Place(3) => "Music".to_string(),
                    layout::SideKind::Place(4) => "Videos".to_string(),
                    layout::SideKind::Place(5) => "Trash".to_string(),
                    // root: partition label when it has one, else "/"
                    layout::SideKind::Place(6) => {
                        if app.root_label.is_empty() {
                            "/".to_string()
                        } else {
                            app.root_label.clone()
                        }
                    }
                    layout::SideKind::Volume(i) => {
                        let lab = app.side_labels.get(*i).cloned().unwrap_or_default();
                        if lab.is_empty() {
                            path_base(&app.side_paths[*i])
                        } else {
                            lab
                        }
                    }
                    layout::SideKind::Network(i) => path_base(&app.net_paths[*i]),
                    layout::SideKind::Unmounted(i) => app
                        .unmounted_labels
                        .get(*i)
                        .cloned()
                        .unwrap_or_else(|| path_base(&app.unmounted_paths[*i])),
                    layout::SideKind::Removable(i) => {
                        let lab = app.removable_labels.get(*i).cloned().unwrap_or_default();
                        if lab.is_empty() {
                            path_base(&app.removable_paths[*i])
                        } else {
                            lab
                        }
                    }
                    layout::SideKind::Bookmark(i) => path_base(&app.bookmark_paths[*i]),
                    _ => String::new(),
                };
                let color = side_label_color(app, kind);
                // per-row icon, then label to its right
                let icon_name = side_icon(kind);
                let isz = (lh - 6).max(10);
                let mut tx = app.cfg.padding;
                if !icon_name.is_empty() {
                    crate::icons::draw(app, buf, tx, y + (lh - isz) / 2, isz, icon_name, color);
                    tx += isz + app.cfg.padding / 2;
                }
                if let Some(text) = &app.text {
                    text.draw(buf, app.width, app.height, tx, y + base_off, &label, color);
                }
                // drive free space shows only while the row is hovered
                let hovered = app.side_hover == id;
                match kind {
                    layout::SideKind::Place(6) if hovered => {
                        draw_side_free(app, buf, sw, y, app.side_free.get(1).copied().unwrap_or(-1))
                    }
                    layout::SideKind::Volume(i) if hovered => {
                        draw_side_free(app, buf, sw, y, app.side_free.get(2 + *i).copied().unwrap_or(-1))
                    }
                    layout::SideKind::Removable(i) if hovered => {
                        let f = app.removable_free.get(*i).copied().unwrap_or(-1);
                        if app.removable_mounted.get(*i).copied().unwrap_or(false) {
                            draw_side_free(app, buf, sw, y, f)
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

fn is_text_preview_name(name: &str) -> bool {
    const EXTS: &[&str] = &[
        ".txt", ".md", ".c", ".h", ".py", ".sh", ".json", ".toml", ".yaml", ".yml", ".conf", ".cfg",
        ".ini", ".log", ".rs", ".go", ".js", ".ts", ".css", ".html", ".xml",
    ];
    EXTS.iter().any(|e| crate::config::has_suffix_ci(name, e))
}

fn draw_preview_right(app: &App, buf: &mut [u32]) {
    let pw = layout::preview_w(app);
    if pw <= 0 {
        return;
    }
    let t = &app.tabs[app.cur_tab];
    let win_w = app.width as i32;
    let win_h = app.height as i32;
    let lh = layout::line_h(app);
    let pad = app.cfg.padding;
    let px_ = win_w - pw;
    let y0 = layout::bar_h(app);
    let top = win_h - layout::status_h(app);
    fill_rect(app, buf, px_, y0, pw, top - y0, app.cfg.dim);
    let grip_w = if app.preview_resize || app.preview_resize_hover { 3 } else { 1 };
    let grip_col = if app.preview_resize || app.preview_resize_hover {
        app.cfg.sel_bg
    } else {
        app.cfg.thumb_c
    };
    fill_rect(app, buf, px_, y0, grip_w, top - y0, grip_col);
    if app.preview_resize || app.preview_resize_hover {
        draw_resize_arrows(app, buf, px_, (y0 + top) / 2, app.cfg.sel_fg);
    }
    let bx = px_ + pad;
    let bw = (pw - pad * 2).max(1);

    let Some(e) = t.selected() else {
        if let Some(text) = &app.text {
            text.draw_clip(
                buf,
                app.width,
                app.height,
                bx,
                y0 + pad + layout::ascent(app),
                "No selection",
                app.cfg.status_c,
                px_ + bw,
            );
        }
        return;
    };
    if e.is_dir {
        if let Some(text) = &app.text {
            text.draw_clip(
                buf,
                app.width,
                app.height,
                bx,
                y0 + pad + layout::ascent(app),
                &display_name(app, e),
                app.cfg.dir,
                px_ + bw,
            );
            text.draw_clip(
                buf,
                app.width,
                app.height,
                bx,
                y0 + pad + lh + layout::ascent(app),
                "Folder",
                app.cfg.status_c,
                px_ + bw,
            );
        }
        return;
    }

    if !e.is_dir && e.kind != EntryType::Archive && e.size < 65536 && is_text_preview_name(&e.name) {
        let full = crate::fs::full_path(&t.cwd, &e.name);
        let content = crate::fs::read_preview_text(&full, 2048);
        let avail = ((top - y0 - pad * 2) / lh).max(0) as usize;
        for (lines, line) in content.lines().enumerate() {
            if lines >= avail {
                break;
            }
            let clipped: String = line.chars().take(200).collect();
            if let Some(text) = &app.text {
                text.draw_clip(
                    buf,
                    app.width,
                    app.height,
                    bx,
                    y0 + pad + layout::ascent(app) + lines as i32 * lh,
                    &clipped,
                    app.cfg.fg,
                    px_ + bw,
                );
            }
        }
    } else {
        // name + type placeholder when no text preview
        if let Some(text) = &app.text {
            text.draw_clip(
                buf,
                app.width,
                app.height,
                bx,
                y0 + pad + layout::ascent(app),
                &display_name(app, e),
                app.cfg.fg,
                px_ + bw,
            );
        }
    }

    let info_y = top - lh * 3;
    if info_y > y0 + pad {
        let sz = crate::config::fmt_size(e.size as i64);
        let kind = if e.is_link { "symlink" } else { "file" };
        if let Some(text) = &app.text {
            text.draw(buf, app.width, app.height, bx, info_y + layout::ascent(app), &sz, app.cfg.status_c);
            text.draw(
                buf,
                app.width,
                app.height,
                bx,
                info_y + lh + layout::ascent(app),
                kind,
                app.cfg.status_c,
            );
        }
    }
}

fn draw_menu_panel(app: &App, buf: &mut [u32], menu: &crate::menu::Menu) {
    let lh = layout::line_h(app);
    let pad = app.cfg.padding;
    let mw = layout::menu_width(app, menu);
    let mh = layout::menu_height(app, menu);
    // shadow
    blend_rect(app, buf, menu.x + 3, menu.y + 3, mw, mh, 0x000000, 100);
    fill_rect(app, buf, menu.x, menu.y, mw, mh, app.cfg.input_bg);
    fill_rect(app, buf, menu.x, menu.y, mw, 1, app.cfg.thumb_c);
    fill_rect(app, buf, menu.x, menu.y + mh - 1, mw, 1, app.cfg.thumb_c);
    fill_rect(app, buf, menu.x, menu.y, 1, mh, app.cfg.thumb_c);
    fill_rect(app, buf, menu.x + mw - 1, menu.y, 1, mh, app.cfg.thumb_c);
    let mut yy = menu.y + pad / 2;
    for (i, it) in menu.items.iter().enumerate() {
        if it.id == crate::menu::MenuId::Separator {
            fill_rect(app, buf, menu.x + pad, yy + pad / 4, mw - pad * 2, 1, app.cfg.thumb_c);
            yy += pad;
            continue;
        }
        let h = lh + pad / 2;
        if menu.hover == i as i32 && it.enabled {
            fill_rect(app, buf, menu.x + 1, yy, mw - 2, h, app.cfg.sel_bg);
        }
        let col = if !it.enabled {
            app.cfg.status_c
        } else if menu.hover == i as i32 {
            app.cfg.sel_fg
        } else {
            app.cfg.fg
        };
        if let Some(text) = &app.text {
            let (main, shortcut) = match it.label.find('\t') {
                Some(t) => (&it.label[..t], Some(&it.label[t + 1..])),
                None => (&it.label[..], None),
            };
            text.draw(buf, app.width, app.height, menu.x + pad, yy + layout::ascent(app), main, col);
            if it.sub.is_some() {
                // submenu marker at the right edge
                let aw = text.width("›") as i32;
                text.draw(
                    buf,
                    app.width,
                    app.height,
                    menu.x + mw - pad - aw,
                    yy + layout::ascent(app),
                    "›",
                    col,
                );
            } else if let Some(sc) = shortcut {
                if !sc.is_empty() {
                    let scw = text.width(sc) as i32;
                    text.draw(
                        buf,
                        app.width,
                        app.height,
                        menu.x + mw - pad - scw,
                        yy + layout::ascent(app),
                        sc,
                        app.cfg.status_c,
                    );
                }
            }
        }
        yy += h;
    }
}

fn draw_popup_menu(app: &App, buf: &mut [u32]) {
    let Some(menu) = &app.menu else { return };
    draw_menu_panel(app, buf, menu);
    // the open submenu floats on top of the parent
    if let Some(oi) = menu.sub_open {
        if let Some(sub) = menu.items.get(oi).and_then(|it| it.sub.as_deref()) {
            draw_menu_panel(app, buf, sub);
        }
    }
}

pub fn render(app: &App) -> Vec<u32> {
    let bw = app.width as usize;
    let bh = app.height as usize;
    let mut buf = vec![0u32; bw * bh];
    fill_rect(app, &mut buf, 0, 0, app.width as i32, app.height as i32, app.cfg.bg);
    draw_menu_bar(app, &mut buf);
    draw_tabs(app, &mut buf);
    draw_path(app, &mut buf);
    draw_input(app, &mut buf);
    draw_sidebar(app, &mut buf);
    let x0 = layout::content_x(app);
    let w = layout::content_w(app);
    if layout::split_active(app) {
        let sx = layout::split_x(app);
        let t0 = &app.tabs[app.cur_tab];
        let t1 = app.tab().pane.as_ref().unwrap();
        let hover0 = if app.hover_pane == 0 { app.hover_row } else { -1 };
        let hover1 = if app.hover_pane == 1 { app.hover_row } else { -1 };
        draw_pane(app, &mut buf, t0, x0, sx - x0, hover0);
        // divider between the panes (draggable to resize)
        let y0 = layout::bar_h(app);
        let dh = app.height as i32 - y0 - layout::status_h(app);
        let active = app.split_resize || app.split_resize_hover;
        let grip_w = if active { 3 } else { 1 };
        let grip_col = if active { app.cfg.sel_bg } else { app.cfg.thumb_c };
        fill_rect(app, &mut buf, sx - grip_w, y0, grip_w * 2, dh, grip_col);
        // always show the <-> cue so the divider reads as draggable
        let mid = y0 + dh / 2;
        if active {
            draw_resize_arrows(app, &mut buf, sx, mid, app.cfg.sel_fg);
        } else {
            draw_resize_arrows(app, &mut buf, sx, mid, app.cfg.thumb_c);
        }
        draw_pane(app, &mut buf, t1, sx, x0 + w - sx, hover1);
    } else {
        let t = &app.tabs[app.cur_tab];
        draw_pane(app, &mut buf, t, x0, w, app.hover_row);
    }
    draw_preview_right(app, &mut buf);
    draw_status(app, &mut buf);
    draw_props(app, &mut buf);
    draw_close_dlg(app, &mut buf);
    draw_import_dlg(app, &mut buf);
    draw_mkdir_dlg(app, &mut buf);
    draw_toolbar_dlg(app, &mut buf);
    draw_rubberband(app, &mut buf);
    draw_typeahead(app, &mut buf);
    draw_popup_menu(app, &mut buf);
    draw_tooltip(app, &mut buf);
    buf
}

/// One split pane (or the whole content area): optional path strip, then
/// the tab's view + scrollbar.
fn draw_pane(app: &App, buf: &mut [u32], t: &crate::tab::Tab, x0: i32, w: i32, hover: i32) {
    if layout::split_active(app) {
        // path strip at the top of each pane
        let lh = layout::line_h(app);
        let y0 = layout::bar_h(app);
        fill_rect(app, buf, x0, y0, w, lh, app.cfg.dim);
        let s = t.cwd.display().to_string();
        if let Some(text) = &app.text {
            text.draw_clip(
                buf,
                app.width,
                app.height,
                x0 + app.cfg.padding / 2,
                y0 + app.cfg.padding / 2 + layout::ascent(app),
                &s,
                app.cfg.status_c,
                x0 + w - app.cfg.padding / 2,
            );
        }
    }
    match t.view {
        ViewMode::List => draw_list(app, buf, t, x0, w, hover),
        ViewMode::Grid => draw_grid(app, buf, t, x0, w, hover),
        ViewMode::Compact => draw_list(app, buf, t, x0, w, hover),
    }
}

/// Small hover tooltip near the pointer (sidebar places / volumes).
fn draw_tooltip(app: &App, buf: &mut [u32]) {
    let Some((lines, x, y)) = &app.tooltip else { return };
    let lh = layout::line_h(app);
    let pad = app.cfg.padding;
    let w = lines
        .iter()
        .map(|l| {
            app.text
                .as_ref()
                .map(|t| t.width(l) as i32)
                .unwrap_or(l.len() as i32 * 8)
        })
        .max()
        .unwrap_or(100)
        + pad * 2;
    let h = lines.len() as i32 * lh + pad;
    fill_rect(app, buf, *x, *y, w, h, app.cfg.input_bg);
    fill_rect(app, buf, *x, *y, w, 1, app.cfg.thumb_c);
    fill_rect(app, buf, *x, *y + h - 1, w, 1, app.cfg.thumb_c);
    fill_rect(app, buf, *x, *y, 1, h, app.cfg.thumb_c);
    fill_rect(app, buf, *x + w - 1, *y, 1, h, app.cfg.thumb_c);
    if let Some(text) = &app.text {
        for (i, line) in lines.iter().enumerate() {
            text.draw(
                buf,
                app.width,
                app.height,
                *x + pad,
                *y + pad / 2 + layout::ascent(app) + i as i32 * lh,
                line,
                app.cfg.fg,
            );
        }
    }
}

/// Type-to-select indicator (bottom right, above the status bar) showing the
/// chars typed so far, like Thunar's type-ahead box.
fn draw_typeahead(app: &App, buf: &mut [u32]) {
    if app.typeahead.is_empty() {
        return;
    }
    let pad = app.cfg.padding;
    let lh = layout::line_h(app);
    let Some(text) = &app.text else { return };
    let label = format!("{}|", app.typeahead);
    let tw = text.width(&label) as i32;
    let w = tw + pad * 2;
    let h = lh + pad;
    let x = app.width as i32 - w - pad;
    let y = app.height as i32 - layout::status_h(app) - h - pad;
    fill_rect(app, buf, x, y, w, h, app.cfg.input_bg);
    fill_rect(app, buf, x, y, w, 1, app.cfg.sel_bg);
    fill_rect(app, buf, x, y + h - 1, w, 1, app.cfg.sel_bg);
    fill_rect(app, buf, x, y, 1, h, app.cfg.sel_bg);
    fill_rect(app, buf, x + w - 1, y, 1, h, app.cfg.sel_bg);
    text.draw(
        buf,
        app.width,
        app.height,
        x + pad,
        y + pad / 2 + layout::ascent(app),
        &label,
        app.cfg.fg,
    );
}

/// Rubberband selection overlay (drawn above entries, below menus).
fn draw_rubberband(app: &App, buf: &mut [u32]) {
    let Some((sx, sy, ex, ey)) = app.rubber else { return };
    let (x0, y0) = (sx.min(ex), sy.min(ey));
    let (x1, y1) = (sx.max(ex), sy.max(ey));
    let col = app.cfg.sel_bg;
    fill_rect(app, buf, x0, y0, (x1 - x0).max(1), 1, col);
    fill_rect(app, buf, x0, y1, (x1 - x0).max(1), 1, col);
    fill_rect(app, buf, x0, y0, 1, (y1 - y0).max(1), col);
    fill_rect(app, buf, x1, y0, 1, (y1 - y0).max(1), col);
    blend_rect(app, buf, x0 + 1, y0 + 1, (x1 - x0 - 1).max(0), (y1 - y0 - 1).max(0), col, 40);
}

fn draw_grid(app: &App, buf: &mut [u32], t: &crate::tab::Tab, x0: i32, w: i32, hover: i32) {
    let y0 = layout::list_y_for(app, t);
    let cols = layout::grid_cols_in(app, w);
    let cell = app.cfg.grid_cell;
    let pad = app.cfg.padding;
    let isz = app.cfg.thumb_size;
    let rows = layout::grid_rows_for(app, t);
    let nvis = t.nvis();
    let per_row = cols;
    for i in 0..rows * per_row {
        let vi = t.scroll * per_row + i;
        if vi >= nvis {
            break;
        }
        let cx = x0 + pad + (i % per_row) as i32 * (cell + pad);
        let cy = y0 + (i / per_row) as i32 * (cell + pad);
        let e = &t.entries[t.vis[vi]];
        let selected = !t.sel_none && vi == t.sel;
        let mut color = if e.is_dir {
            app.cfg.dir
        } else {
            match e.kind {
                EntryType::Image => app.cfg.type_img,
                EntryType::Archive => app.cfg.type_arc,
                _ => app.cfg.type_file,
            }
        };
        if selected {
            fill_rect(app, buf, cx, cy, cell, cell, app.cfg.sel_bg);
            color = app.cfg.sel_fg;
        } else if e.selected {
            fill_rect(app, buf, cx, cy, cell, cell, app.cfg.tab_idle);
        } else if vi as i32 == hover {
            blend_rect(app, buf, cx, cy, cell, cell, app.cfg.tab_idle, 80);
        }
        if app.dnd_active && vi as i32 == app.dnd_hover_row {
            blend_rect(app, buf, cx, cy, cell, cell, app.cfg.sel_bg, 110);
        }
        // real thumbnail for images
        if let Some(thumb) = &e.thumb {
            let (tw, th) = (e.thumb_w.max(1), e.thumb_h.max(1));
            let (dw, dh) = if tw >= th {
                (isz, isz * th / tw)
            } else {
                (isz * tw / th, isz)
            };
            let (dw, dh) = (dw.max(1), dh.max(1));
            let ix = cx + (cell - isz) / 2 + (isz - dw) / 2;
            let iy = cy + pad + (isz - dh) / 2;
            blit_rgba(app, buf, ix, iy, dw, dh, thumb, tw, th);
        } else {
            draw_entry_icon(app, buf, cx + (cell - isz) / 2, cy + pad, isz, e);
        }
        if let Some(text) = &app.text {
            let base = cy + pad + isz + layout::ascent(app);
            text.draw_clip(
                buf,
                app.width,
                app.height,
                cx + pad / 2,
                base,
                &display_name(app, e),
                color,
                (cx + cell - pad / 2).max(cx + pad / 2 + 8),
            );
        }
    }
    draw_scrollbar(app, buf, t, x0, w);
}

/// Properties dialog overlay (F9 / Ctrl+Enter).
fn draw_props(app: &App, buf: &mut [u32]) {
    let Some(g) = layout::props_geom(app) else { return };
    let Some(p) = &app.props else { return };
    let lh = layout::line_h(app);
    let pad = app.cfg.padding;
    let tabs = ["General", "Permissions", "Checksum", "Details"];
    let tab_n = tabs.len() as i32;

    // fully hide the window content behind the dialog (no see-through)
    fill_rect(app, buf, 0, 0, app.width as i32, app.height as i32, app.cfg.bg);
    fill_rect(app, buf, g.x, g.y, g.w, g.h, app.cfg.input_bg);
    fill_rect(app, buf, g.x, g.y, g.w, 2, app.cfg.dir);

    // title band: file name at the left, close (X) button at top-right
    if let Some(text) = &app.text {
        let name = p.path.rsplit('/').next().unwrap_or(&p.path);
        text.draw_clip(
            buf,
            app.width,
            app.height,
            g.x + pad,
            g.y + pad + layout::ascent(app),
            name,
            app.cfg.fg,
            (g.close.x - pad).max(g.x + pad + 8),
        );
    }
    let c = g.close;
    let inset = (c.w / 3).max(3);
    let (x0, y0) = (c.x + inset, c.y + inset);
    let (x1, y1) = (c.x + c.w - inset, c.y + c.h - inset);
    draw_line(app, buf, x0, y0, x1, y1, app.cfg.status_c);
    draw_line(app, buf, x1, y0, x0, y1, app.cfg.status_c);

    // section tabs
    let tw = g.tabs.w / tab_n;
    for (i, name) in tabs.iter().enumerate() {
        let active = i as i32 == app.props_tab;
        let tx = g.tabs.x + tw * i as i32;
        if active {
            fill_rect(app, buf, tx, g.tabs.y, tw, g.tabs.h, app.cfg.tab_active);
        }
        if let Some(text) = &app.text {
            let lw = text.width(name) as i32;
            text.draw(
                buf,
                app.width,
                app.height,
                tx + (tw - lw) / 2,
                g.tabs.y + (g.tabs.h - lh) / 2 + layout::ascent(app),
                name,
                if active { app.cfg.fg } else { app.cfg.status_c },
            );
        }
    }
    // underline on the active tab
    if let Some((cx, cy)) = layout::props_tab_center(app, app.props_tab) {
        fill_rect(app, buf, cx - tw / 4, cy - 1, tw / 2, 2, app.cfg.sel_bg);
    }

    let x0 = g.x + pad;
    let label_w = app.text.as_ref().map(|t| t.width("location")).unwrap_or(60.0) as i32;
    let val_x = x0 + label_w;
    let val_clip = g.x + g.w - pad;
    let mut row_y = g.tabs.y + g.tabs.h + pad + layout::ascent(app);

    let name = p.path.rsplit('/').next().unwrap_or(&p.path).to_string();
    let size_str = if p.is_dir {
        "(directory)".to_string()
    } else {
        format!("{} ({})", crate::config::fmt_size(p.size as i64), p.size)
    };
    let rows: Vec<(&str, String)> = match app.props_tab {
        0 => vec![
            ("name", name),
            ("location", p.path.clone()),
            ("type", p.kind.clone()),
            ("size", size_str),
            ("modified", p.mtime.clone()),
        ],
        1 => vec![
            ("permissions", p.perm.clone()),
            ("owner", p.owner.clone()),
            ("group", p.group.clone()),
            ("links", p.nlink.to_string()),
            ("is directory", if p.is_dir { "yes" } else { "no" }.to_string()),
        ],
        2 => {
            if p.is_dir {
                vec![("note", "checksums only apply to files".to_string())]
            } else if app.props_checksum_pending {
                vec![("status", "computing…".to_string())]
            } else {
                match &app.props_checksum {
                    Some((md5, sha1, sha256)) => vec![
                        ("md5", md5.clone()),
                        ("sha1", sha1.clone()),
                        ("sha256", sha256.clone()),
                    ],
                    None => vec![("status", "not available".to_string())],
                }
            }
        }
        _ => vec![
            ("inode", p.inode.to_string()),
            ("type", p.kind.clone()),
            ("size (bytes)", p.size.to_string()),
            ("modified", p.mtime.clone()),
            ("path", p.path.clone()),
        ],
    };
    for (label, value) in rows {
        if let Some(text) = &app.text {
            text.draw(buf, app.width, app.height, x0, row_y, label, app.cfg.status_c);
            text.draw_clip(buf, app.width, app.height, val_x, row_y, &value, app.cfg.fg, val_clip);
        }
        row_y += lh + pad;
    }
    if let Some(text) = &app.text {
        text.draw(
            buf,
            app.width,
            app.height,
            x0,
            g.y + g.h - pad - layout::ascent(app),
            "Esc close  ·  ← / → switch section",
            app.cfg.status_c,
        );
    }
}

/// "Close wfm?" confirmation shown when closing with multiple open tabs.
fn draw_close_dlg(app: &App, buf: &mut [u32]) {
    if !app.confirm_close {
        return;
    }
    let Some(g) = layout::close_dlg_geom(app) else { return };
    let lh = layout::line_h(app);
    let pad = app.cfg.padding;
    fill_rect(app, buf, 0, 0, app.width as i32, app.height as i32, app.cfg.bg);
    fill_rect(app, buf, g.x, g.y, g.w, g.h, app.cfg.input_bg);
    fill_rect(app, buf, g.x, g.y, g.w, 2, app.cfg.dir);
    let msg = format!("{} tabs are open. Close the window?", app.tabs.len());
    if let Some(text) = &app.text {
        text.draw_bold(
            buf,
            app.width,
            app.height,
            g.x + pad,
            g.y + pad + layout::ascent(app),
            "Close wfm?",
            app.cfg.fg,
        );
        text.draw(
            buf,
            app.width,
            app.height,
            g.x + pad,
            g.y + pad + (lh + pad) + layout::ascent(app),
            &msg,
            app.cfg.status_c,
        );
    }
    // "Don't ask again" checkbox
    let cb = g.checkbox;
    let cs = (lh - 2).max(10);
    fill_rect(app, buf, cb.x, cb.y, cs, cs, app.cfg.bg);
    fill_rect(app, buf, cb.x, cb.y, cs, 1, app.cfg.thumb_c);
    fill_rect(app, buf, cb.x, cb.y + cs - 1, cs, 1, app.cfg.thumb_c);
    fill_rect(app, buf, cb.x, cb.y, 1, cs, app.cfg.thumb_c);
    fill_rect(app, buf, cb.x + cs - 1, cb.y, 1, cs, app.cfg.thumb_c);
    if app.confirm_dont_ask {
        let c = app.cfg.sel_bg;
        let (sx, sy) = (cb.x + 2, cb.y + 2);
        draw_line(app, buf, sx, sy + cs / 3, sx + cs / 3, sy + cs / 2, c);
        draw_line(app, buf, sx + cs / 3, sy + cs / 2, sx + cs - 3, sy, c);
    }
    if let Some(text) = &app.text {
        let bl = cb.y + (cs + layout::ascent(app)) / 2 - 1;
        text.draw(buf, app.width, app.height, cb.x + cs + pad, bl, "Don't ask again", app.cfg.fg);
    }
    // buttons: Cancel | Close Tab | Close Window
    for (rect, label, accent) in [
        (&g.btn_cancel, "Cancel", false),
        (&g.btn_tab, "Close Tab", false),
        (&g.btn_window, "Close Window", true),
    ] {
        fill_rect(app, buf, rect.x, rect.y, rect.w, rect.h, if accent { app.cfg.tab_active } else { app.cfg.tab_idle });
        fill_rect(app, buf, rect.x, rect.y, rect.w, 1, app.cfg.thumb_c);
        if let Some(text) = &app.text {
            let lw = text.width(label) as i32;
            text.draw(
                buf,
                app.width,
                app.height,
                rect.x + (rect.w - lw) / 2,
                rect.y + (rect.h - lh) / 2 + layout::ascent(app),
                label,
                app.cfg.fg,
            );
        }
    }
}

/// "Import Thunar Custom Actions?" confirmation shown when the (temporary)
/// import menu item is used and current wfm actions would be replaced.
fn draw_import_dlg(app: &App, buf: &mut [u32]) {
    if !app.confirm_import {
        return;
    }
    let Some(g) = layout::import_dlg_geom(app) else { return };
    let lh = layout::line_h(app);
    let pad = app.cfg.padding;
    fill_rect(app, buf, 0, 0, app.width as i32, app.height as i32, app.cfg.bg);
    fill_rect(app, buf, g.x, g.y, g.w, g.h, app.cfg.input_bg);
    fill_rect(app, buf, g.x, g.y, g.w, 2, app.cfg.dir);
    let msg = format!(
        "This will replace your {} current custom action(s).",
        app.import_pending.len()
    );
    if let Some(text) = &app.text {
        text.draw_bold(
            buf,
            app.width,
            app.height,
            g.x + pad,
            g.y + pad + layout::ascent(app),
            "Import Thunar Custom Actions?",
            app.cfg.fg,
        );
        text.draw(
            buf,
            app.width,
            app.height,
            g.x + pad,
            g.y + pad + (lh + pad) + layout::ascent(app),
            &msg,
            app.cfg.status_c,
        );
    }
    // buttons: Cancel | Import
    for (rect, label, accent) in [
        (&g.btn_cancel, "Cancel", false),
        (&g.btn_ok, "Import", true),
    ] {
        fill_rect(app, buf, rect.x, rect.y, rect.w, rect.h, if accent { app.cfg.tab_active } else { app.cfg.tab_idle });
        fill_rect(app, buf, rect.x, rect.y, rect.w, 1, app.cfg.thumb_c);
        if let Some(text) = &app.text {
            let lw = text.width(label) as i32;
            text.draw(
                buf,
                app.width,
                app.height,
                rect.x + (rect.w - lw) / 2,
                rect.y + (rect.h - lh) / 2 + layout::ascent(app),
                label,
                app.cfg.fg,
            );
        }
    }
}

fn draw_mkdir_dlg(app: &App, buf: &mut [u32]) {
    let Some(g) = layout::mkdir_dlg_geom(app) else { return };
    let lh = layout::line_h(app);
    let pad = app.cfg.padding;
    fill_rect(app, buf, 0, 0, app.width as i32, app.height as i32, app.cfg.bg);
    fill_rect(app, buf, g.x, g.y, g.w, g.h, app.cfg.input_bg);
    fill_rect(app, buf, g.x, g.y, g.w, 2, app.cfg.dir);
    let path = app.confirm_mkdir.as_deref().unwrap_or("");
    if let Some(text) = &app.text {
        text.draw_bold(
            buf,
            app.width,
            app.height,
            g.x + pad,
            g.y + pad + layout::ascent(app),
            "Folder not found",
            app.cfg.fg,
        );
        text.draw(
            buf,
            app.width,
            app.height,
            g.x + pad,
            g.y + pad + (lh + pad) + layout::ascent(app),
            &format!("Create folder \"{path}\"?"),
            app.cfg.status_c,
        );
    }
    // buttons: Cancel | Create
    for (rect, label, accent) in [
        (&g.btn_cancel, "Cancel", false),
        (&g.btn_create, "Create", true),
    ] {
        fill_rect(app, buf, rect.x, rect.y, rect.w, rect.h, if accent { app.cfg.tab_active } else { app.cfg.tab_idle });
        fill_rect(app, buf, rect.x, rect.y, rect.w, 1, app.cfg.thumb_c);
        if let Some(text) = &app.text {
            let lw = text.width(label) as i32;
            text.draw(
                buf,
                app.width,
                app.height,
                rect.x + (rect.w - lw) / 2,
                rect.y + (rect.h - lh) / 2 + layout::ascent(app),
                label,
                app.cfg.fg,
            );
        }
    }
}

/// View → Configure Toolbar… dialog: checkbox list of toolbar items plus
/// Move Up / Move Down / Reset to Default / Close buttons.
fn draw_toolbar_dlg(app: &App, buf: &mut [u32]) {
    let Some(g) = layout::toolbar_dlg_geom(app) else { return };
    let lh = layout::line_h(app);
    let pad = app.cfg.padding;
    let row_h = lh + pad / 2;
    fill_rect(app, buf, 0, 0, app.width as i32, app.height as i32, app.cfg.bg);
    fill_rect(app, buf, g.x, g.y, g.w, g.h, app.cfg.input_bg);
    fill_rect(app, buf, g.x, g.y, g.w, 2, app.cfg.dir);
    if let Some(text) = &app.text {
        text.draw_bold(
            buf,
            app.width,
            app.height,
            g.x + pad,
            g.y + pad + layout::ascent(app),
            "Configure Toolbar",
            app.cfg.fg,
        );
    }
    // checkbox list
    let Some(dlg) = app.toolbar_dlg.as_ref() else { return };
    let visible: i32 = 8.min(dlg.items.len() as i32).max(1);
    let cs = (lh - 2).max(10);
    let cb_x = g.list.x + pad;
    let label_x = cb_x + cs + pad;
    let n_items = dlg.items.len() as i32;
    for i in 0..visible {
        let idx = dlg.scroll.max(0) + i;
        if idx >= n_items {
            break;
        }
        let it = &dlg.items[idx as usize];
        let row_y = g.list.y + pad / 2 + i * row_h;
        if idx == dlg.sel as i32 {
            fill_rect(app, buf, g.list.x, row_y, g.list.w, row_h, app.cfg.sel_bg);
        }
        let key = crate::toolbar::item_key(it);
        let checked = !dlg.is_hidden(&key);
        let locked = *it == crate::toolbar::ToolbarItem::Location;
        let col = if locked { app.cfg.status_c } else { app.cfg.fg };
        fill_rect(app, buf, cb_x, row_y + (row_h - cs) / 2, cs, cs, app.cfg.bg);
        fill_rect(app, buf, cb_x, row_y + (row_h - cs) / 2, cs, 1, app.cfg.thumb_c);
        fill_rect(app, buf, cb_x, row_y + (row_h - cs) / 2 + cs - 1, cs, 1, app.cfg.thumb_c);
        fill_rect(app, buf, cb_x, row_y + (row_h - cs) / 2, 1, cs, app.cfg.thumb_c);
        fill_rect(app, buf, cb_x + cs - 1, row_y + (row_h - cs) / 2, 1, cs, app.cfg.thumb_c);
        if checked {
            let (sx, sy) = (cb_x + 2, row_y + (row_h - cs) / 2 + 2);
            draw_line(app, buf, sx, sy + cs / 3, sx + cs / 3, sy + cs / 2, col);
            draw_line(app, buf, sx + cs / 3, sy + cs / 2, sx + cs - 3, sy, col);
        }
        let label = crate::toolbar::item_label(it);
        if let Some(text) = &app.text {
            text.draw(
                buf,
                app.width,
                app.height,
                label_x,
                row_y + (row_h - lh) / 2 + layout::ascent(app),
                &label,
                if idx == dlg.sel as i32 { app.cfg.sel_fg } else { col },
            );
        }
    }
    // scroll hint when the list overflows
    if n_items > visible && dlg.scroll > 0 {
        if let Some(text) = &app.text {
            text.draw(
                buf,
                app.width,
                app.height,
                g.list.x + g.list.w - pad * 3,
                g.list.y + pad / 2 + layout::ascent(app),
                "↑",
                app.cfg.status_c,
            );
        }
    }
    // buttons: Up | Down | Reset | Close
    for (rect, label, accent) in [
        (&g.btn_up, "Up", false),
        (&g.btn_down, "Down", false),
        (&g.btn_reset, "Reset", false),
        (&g.btn_close, "Close", true),
    ] {
        fill_rect(app, buf, rect.x, rect.y, rect.w, rect.h, if accent { app.cfg.tab_active } else { app.cfg.tab_idle });
        fill_rect(app, buf, rect.x, rect.y, rect.w, 1, app.cfg.thumb_c);
        if let Some(text) = &app.text {
            let lw = text.width(label) as i32;
            text.draw(
                buf,
                app.width,
                app.height,
                rect.x + (rect.w - lw) / 2,
                rect.y + (rect.h - lh) / 2 + layout::ascent(app),
                label,
                app.cfg.fg,
            );
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::App;
    use crate::config::Config;
    use crate::tab::Tab;
    use std::path::PathBuf;

    #[test]
    fn render_has_text_pixels() {
        let cfg = Config::default();
        let mut app = App::new(cfg);
        app.text = crate::text::Text::new_family(app.cfg.font_size as f32, None);
        assert!(app.text.is_some(), "font load failed");
        app.width = 800;
        app.height = 500;
        let mut tab = Tab::new(0, PathBuf::from("/tmp"));
        crate::fs::list_dir(&mut tab, false, false);
        tab.rebuild_visible();
        app.tabs.push(tab);
        app.sidebar_visible = true;
        app.sidebar_refresh();
        let buf = render(&app);
        let bg = 0xff000000 | app.cfg.bg;
        let lit = buf.iter().filter(|&&p| p != bg && (p >> 24) == 0xff).count();
        // icons alone light some pixels; text should light many more
        eprintln!("render lit non-bg pixels: {lit}");
        let _ = std::fs::write("/tmp/wfm_render_probe.raw", buf.iter().flat_map(|p| p.to_le_bytes()).collect::<Vec<_>>());
        assert!(lit > 500, "too few non-bg pixels: {lit}");
    }

    #[test]
    fn entry_icons_classify_like_dolphin() {
        use crate::entry::Entry;
        let cfg = Config::default();
        let app = App::new(cfg);
        let mk = |name: &str, kind: EntryType| Entry {
            name: name.to_string(),
            kind,
            ..Default::default()
        };
        assert_eq!(entry_icon(&app, &mk("docs", EntryType::Dir)).0, "folder");
        assert_eq!(entry_icon(&app, &mk("pic.png", EntryType::Image)).0, "image");
        assert_eq!(entry_icon(&app, &mk("bundle.zip", EntryType::Archive)).0, "archive");
        // same image icon, different palette tint per extension
        let png = mk("pic.png", EntryType::Image);
        let jpg = mk("pic.jpg", EntryType::Image);
        let gif = mk("pic.gif", EntryType::Image);
        assert_eq!(entry_icon(&app, &png), ("image", app.cfg.icon_img));
        assert_eq!(entry_icon(&app, &jpg), ("image", app.cfg.icon_arc));
        assert_eq!(entry_icon(&app, &gif), ("image", app.cfg.icon_dir));
        assert_ne!(entry_icon(&app, &png).1, entry_icon(&app, &jpg).1);
        assert_eq!(file_icon(&app, &mk("a.mp3", EntryType::File)).0, "audio");
        assert_eq!(file_icon(&app, &mk("a.mkv", EntryType::File)).0, "video");
        assert_eq!(file_icon(&app, &mk("a.pdf", EntryType::File)).0, "pdf");
        assert_eq!(file_icon(&app, &mk("a.rs", EntryType::File)).0, "code");
        assert_eq!(file_icon(&app, &mk("a.py", EntryType::File)).0, "script");
        assert_eq!(file_icon(&app, &mk("a.conf", EntryType::File)).0, "config");
        assert_eq!(file_icon(&app, &mk("a.ttf", EntryType::File)).0, "font");
        assert_eq!(file_icon(&app, &mk("a.xlsx", EntryType::File)).0, "table");
        assert_eq!(file_icon(&app, &mk("a.pptx", EntryType::File)).0, "present");
        assert_eq!(file_icon(&app, &mk("a.docx", EntryType::File)).0, "doc");
        assert_eq!(file_icon(&app, &mk("a.sqlite", EntryType::File)).0, "database");
        assert_eq!(file_icon(&app, &mk("a.ics", EntryType::File)).0, "calendar");
        assert_eq!(file_icon(&app, &mk("a.iso", EntryType::File)).0, "disk");
        // dotfiles → config, unknown → generic file
        assert_eq!(file_icon(&app, &mk(".bashrc", EntryType::File)).0, "config");
        assert_eq!(file_icon(&app, &mk("README", EntryType::File)).0, "text");
        assert_eq!(file_icon(&app, &mk("mystery.xyz", EntryType::File)).0, "file");
        // executables (no recognised extension) get the bolt
        let mut exe = mk("tool", EntryType::File);
        exe.is_exec = true;
        assert_eq!(file_icon(&app, &exe).0, "exec");
        // symlink to a dir keeps the folder icon; the arrow badge is drawn on top
        let mut lnk = mk("target", EntryType::Link);
        lnk.is_link = true;
        lnk.is_dir = true;
        assert_eq!(entry_icon(&app, &lnk).0, "folder");
    }

    #[test]
    fn symlink_badge_pixels_render() {
        use crate::entry::Entry;
        let cfg = Config::default();
        let mut app = App::new(cfg);
        app.width = 48;
        app.height = 48;
        let mk_link = |dir: bool| Entry {
            name: "target".into(),
            kind: EntryType::Link,
            is_link: true,
            is_dir: dir,
            ..Default::default()
        };
        // dir symlink: badge centered on the bottom edge
        let mut buf = vec![0xff00_0000 | app.cfg.bg; 48 * 48];
        draw_entry_icon(&app, &mut buf, 8, 8, 32, &mk_link(true));
        let link_px = buf.iter().filter(|&&p| p == 0xff_ff_ff_ff).count();
        assert!(link_px >= 8, "dir symlink badge not visible ({link_px} px)");
        // file symlink: badge in the bottom-right corner
        let mut buf = vec![0xff00_0000 | app.cfg.bg; 48 * 48];
        draw_entry_icon(&app, &mut buf, 8, 8, 32, &mk_link(false));
        let link_px = buf.iter().filter(|&&p| p == 0xff_ff_ff_ff).count();
        assert!(link_px >= 8, "file symlink badge not visible ({link_px} px)");
    }

    #[test]
    fn import_dialog_renders_and_has_geometry() {
        let cfg = Config::default();
        let mut app = App::new(cfg);
        app.text = crate::text::Text::new_family(app.cfg.font_size as f32, None);
        app.width = 800;
        app.height = 500;
        let mut tab = Tab::new(0, PathBuf::from("/tmp"));
        crate::fs::list_dir(&mut tab, false, false);
        tab.rebuild_visible();
        app.tabs.push(tab);
        app.confirm_import = true;
        app.import_pending.push(crate::actions::CustomAction {
            name: "x".into(),
            command: "echo %f".into(),
            patterns: vec!["*".into()],
            directories: false,
            audio_files: false,
            image_files: false,
            other_files: false,
            text_files: false,
            video_files: false,
        });
        let geom = layout::import_dlg_geom(&app).expect("dialog geometry");
        assert!(geom.btn_ok.contains(geom.btn_ok.x + geom.btn_ok.w / 2, geom.btn_ok.y + 4));
        assert!(!geom.btn_cancel.contains(geom.btn_ok.x + 4, geom.btn_ok.y + 4));
        let buf = render(&app);
        let bg = 0xff000000 | app.cfg.bg;
        let lit = buf.iter().filter(|&&p| p != bg && (p >> 24) == 0xff).count();
        assert!(lit > 200, "dialog should light pixels, got {lit}");
        // dismissing via Esc clears the pending import
        app.on_key(crate::wl::Keysym::Escape, None, false, false);
        assert!(!app.confirm_import);
        assert!(app.import_pending.is_empty());
    }

    #[test]
    fn close_dialog_renders_over_multiple_tabs() {
        let cfg = Config::default();
        let mut app = App::new(cfg);
        app.text = crate::text::Text::new_family(app.cfg.font_size as f32, None);
        app.width = 800;
        app.height = 500;
        for i in 0..2 {
            let mut tab = Tab::new(i, PathBuf::from("/tmp"));
            crate::fs::list_dir(&mut tab, false, false);
            tab.rebuild_visible();
            app.tabs.push(tab);
        }
        app.cur_tab = 0;
        app.confirm_close = true;
        let geom = layout::close_dlg_geom(&app).expect("dialog geometry");
        assert!(geom.btn_window.contains(geom.btn_window.x + geom.btn_window.w / 2, geom.btn_window.y + 4));
        assert!(!geom.btn_cancel.contains(geom.btn_window.x + 4, geom.btn_window.y + 4));
        let buf = render(&app);
        let bg = 0xff000000 | app.cfg.bg;
        let lit = buf.iter().filter(|&&p| p != bg && (p >> 24) == 0xff).count();
        assert!(lit > 200, "dialog should light pixels, got {lit}");
        // "Don't ask again" checkbox present → text drawn under it
        app.confirm_close = true;
        app.confirm_dont_ask = true;
        let buf2 = render(&app);
        assert_ne!(buf, buf2, "checked checkbox must change pixels");
    }

    #[test]
    fn split_geometry_routes_panes() {
        let cfg = Config::default();
        let mut app = App::new(cfg);
        app.width = 1200;
        app.height = 700;
        app.tabs.push(Tab::new(0, PathBuf::from("/tmp")));
        app.tabs.push(Tab::new(1, PathBuf::from("/usr")));
        app.cur_tab = 0;
        app.tabs[0].split = true;
        app.tabs[0].pane = Some(Box::new(Tab::new(2, PathBuf::from("/usr"))));
        assert!(layout::split_active(&app));
        let x0 = layout::content_x(&app);
        let sx = layout::split_x(&app);
        assert_eq!(layout::pane_of(&app, x0 + 10, 300), 0);
        assert_eq!(layout::pane_of(&app, sx + 10, 300), 1);
        let (rx, rw) = layout::pane_geom(&app, 1);
        assert_eq!(rx, sx);
        assert!(rw > 0);
        let (lx, lw) = layout::pane_geom(&app, 0);
        assert_eq!(lx, x0);
        assert_eq!(lw, sx - x0);
        // grid column counts follow the pane width, not the full content width
        assert!(layout::grid_cols(&app) <= layout::grid_cols_in(&app, layout::content_w(&app)));
    }

    #[test]
    fn split_panes_render_divider() {
        let cfg = Config::default();
        let mut app = App::new(cfg);
        app.text = crate::text::Text::new_family(app.cfg.font_size as f32, None);
        app.width = 1200;
        app.height = 700;
        for (i, dir) in ["/tmp", "/usr"].iter().enumerate() {
            let mut tab = Tab::new(i as i32, PathBuf::from(dir));
            crate::fs::list_dir(&mut tab, false, false);
            tab.rebuild_visible();
            app.tabs.push(tab);
        }
        app.cur_tab = 0;
        {
            let mut pane = Tab::new(2, PathBuf::from("/usr"));
            crate::fs::list_dir(&mut pane, false, false);
            pane.rebuild_visible();
            app.tabs[0].pane = Some(Box::new(pane));
        }
        app.tabs[0].split = true;
        let buf = render(&app);
        // the divider column mid-window must not be the plain background
        let sx = layout::split_x(&app) as usize;
        let y = 300usize;
        let bg = 0xff000000 | app.cfg.bg;
        assert_ne!(buf[y * app.width as usize + sx] & 0x00ffffff, bg & 0x00ffffff, "divider not drawn");
        // the split button icon (top-right cluster) renders too
        let mut ib = vec![0u32; (app.width * app.height) as usize];
        assert!(crate::icons::draw(&app, &mut ib, 8, 8, 16, "split", 0xffffff));
    }

    #[test]
    fn shift_click_selects_anchor_to_clicked_span() {
        let cfg = Config::default();
        let mut app = App::new(cfg);
        app.text = crate::text::Text::new_family(app.cfg.font_size as f32, None);
        app.width = 800;
        app.height = 600;
        let mut tab = Tab::new(0, PathBuf::from("/tmp"));
        for i in 0..5 {
            tab.entries.push(crate::entry::Entry {
                name: format!("f{i}.txt"),
                kind: EntryType::File,
                ..Default::default()
            });
        }
        tab.rebuild_visible();
        app.tabs.push(tab);
        let (x0, _) = layout::pane_geom(&app, 0);
        // keep well clear of the sidebar resize grip (±5px around content_x)
        let cx = x0 + 40;
        let y0 = layout::list_y(&app);
        let rh = layout::row_h(&app);
        // plain click on row 0 sets the anchor
        app.on_pointer_button(cx, y0 + 5, 100, 272, true);
        app.on_pointer_button(cx, y0 + 5, 100, 272, false);
        assert_eq!(app.range_anchor, 0);
        // shift+click on row 3 (well after the double-click window) selects 0..=3
        app.shift = true;
        app.on_pointer_button(cx, y0 + 5 + 3 * rh, 600, 272, true);
        app.on_pointer_button(cx, y0 + 5 + 3 * rh, 600, 272, false);
        app.shift = false;
        let sel: Vec<usize> = app
            .tab()
            .entries
            .iter()
            .enumerate()
            .filter(|(_, e)| e.selected)
            .map(|(i, _)| i)
            .collect();
        assert_eq!(sel, vec![0, 1, 2, 3], "shift+click must select the anchor span");
    }

    /// The "Show File Extensions" toggle only changes the displayed name:
    /// folders, dotfiles and extensionless files keep their full name.
    #[test]
    fn display_name_honors_show_ext() {
        use crate::entry::{Entry, EntryType};
        let mk = |name: &str, is_dir: bool| Entry {
            name: name.to_string(),
            kind: if is_dir { EntryType::Dir } else { EntryType::File },
            is_dir,
            ..Default::default()
        };
        let on = App::new(Config::default());
        let mut off = App::new(Config::default());
        off.cfg.show_ext = false;
        assert_eq!(display_name(&on, &mk("a.tar.gz", false)), "a.tar.gz");
        assert_eq!(display_name(&off, &mk("a.tar.gz", false)), "a.tar");
        assert_eq!(display_name(&off, &mk("docs", true)), "docs");
        assert_eq!(display_name(&off, &mk(".bashrc", false)), ".bashrc");
        assert_eq!(display_name(&off, &mk("README", false)), "README");
        assert_eq!(display_name(&off, &mk("noext", false)), "noext");
    }
}
