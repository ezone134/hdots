use std::path::PathBuf;

use crate::app::App;
use crate::tab::{SortKey, ViewMode};

pub fn line_h(app: &App) -> i32 {
    app.text.as_ref().map(|t| t.line_h()).unwrap_or(18)
}

pub fn ascent(app: &App) -> i32 {
    app.text.as_ref().map(|t| t.ascent_px()).unwrap_or(14)
}

pub fn menu_bar_h(app: &App) -> i32 {
    line_h(app) + app.cfg.padding
}

/// tabs + path bars + menu ribbon.
pub fn bar_h(app: &App) -> i32 {
    menu_bar_h(app) + line_h(app) * 2 + app.cfg.padding * 3
}

pub fn status_h(app: &App) -> i32 {
    line_h(app) + app.cfg.padding
}

/// Top of the list/grid content area for a given tab. In split mode each
/// pane gets a path strip first, so content starts one line lower.
pub fn list_y_for(app: &App, t: &crate::tab::Tab) -> i32 {
    let y = bar_h(app) + if split_active(app) { line_h(app) } else { 0 };
    if t.view == ViewMode::List {
        y + line_h(app) + app.cfg.padding
    } else {
        y
    }
}

/// y of the column-header row (list view). Sits under the split path strip.
pub fn header_y(app: &App) -> i32 {
    bar_h(app) + if split_active(app) { line_h(app) } else { 0 }
}

pub fn list_y(app: &App) -> i32 {
    list_y_for(app, &app.tabs[app.cur_tab])
}

/// Entry-icon size for list/compact rows. `icon_size = 0` means auto
/// (font-derived); a user zoom writes a real value into the config.
pub fn list_icon_size(app: &App) -> i32 {
    let auto = (line_h(app) * 3 / 4).clamp(8, 24);
    if app.cfg.icon_size > 0 {
        app.cfg.icon_size.clamp(8, (line_h(app) * 3).max(32))
    } else {
        auto
    }
}

/// Row height for list/compact views — grows with the zoomed icon so a larger
/// icon never overlaps the row below.
pub fn row_h(app: &App) -> i32 {
    line_h(app).max(list_icon_size(app) + app.cfg.padding / 2)
}

pub fn list_h_for(app: &App, t: &crate::tab::Tab) -> i32 {
    let h = app.height as i32 - list_y_for(app, t) - status_h(app);
    if h > 0 {
        h
    } else {
        1
    }
}

pub fn list_h(app: &App) -> i32 {
    list_h_for(app, &app.tabs[app.cur_tab])
}

/// Width of the right preview pane (0 when hidden).
pub fn preview_w(app: &App) -> i32 {
    if !app.preview_visible || app.tabs.is_empty() {
        return 0;
    }
    let win = app.width as i32;
    if win < 400 {
        return 0;
    }
    let side = content_x_raw(app);
    let min_list = 160i32;
    let max_pw = (win - side - min_list).max(0);
    if max_pw < 100 {
        return 0;
    }
    let mut pw = app.cfg.preview_width;
    if pw < 100 {
        pw = 100;
    }
    if pw > max_pw {
        pw = max_pw;
    }
    if pw > 800 {
        pw = 800;
    }
    pw
}

/// Sidebar width without considering preview (avoids recursion with preview_w).
fn content_x_raw(app: &App) -> i32 {
    if !app.sidebar_visible {
        return 0;
    }
    let win = app.width as i32;
    if win < 360 {
        return 0;
    }
    let mut sw = app.cfg.sidebar_width;
    let max_side = (win - 200).clamp(80, win * 2 / 5);
    if sw > max_side {
        sw = max_side;
    }
    sw.max(0)
}

/// Clamp preview width given current window/sidebar.
pub fn clamp_preview_width(app: &App, w: i32) -> i32 {
    let win = app.width as i32;
    let side = content_x_raw(app);
    let max = (win - side - 160).clamp(100, 800);
    w.clamp(100, max)
}

/// x offset where list/grid content starts (right of the sidebar).
pub fn content_x(app: &App) -> i32 {
    content_x_raw(app)
}

/// Min/max sidebar width in logical pixels.
pub fn clamp_sidebar_width(app: &App, w: i32) -> i32 {
    let win = app.width as i32;
    let prev = if app.preview_visible {
        app.cfg.preview_width.clamp(100, 400)
    } else {
        0
    };
    let max = (win - 160 - prev).max(80).clamp(80, 600);
    w.clamp(80, max)
}

/// Hit-test the sidebar resize grip (right edge of sidebar).
pub fn sidebar_splitter_at(app: &App, x: i32, y: i32) -> bool {
    if !app.sidebar_visible {
        return false;
    }
    let y0 = bar_h(app);
    let top = app.height as i32 - status_h(app);
    if y < y0 || y >= top {
        return false;
    }
    let edge = content_x(app);
    // ~10px grab zone straddling the divider
    (x - edge).abs() <= 5
}

/// Hit-test the preview resize grip (left edge of preview pane).
pub fn preview_splitter_at(app: &App, x: i32, y: i32) -> bool {
    if !app.preview_visible {
        return false;
    }
    let pw = preview_w(app);
    if pw <= 0 {
        return false;
    }
    let y0 = bar_h(app);
    let top = app.height as i32 - status_h(app);
    if y < y0 || y >= top {
        return false;
    }
    let edge = app.width as i32 - pw;
    (x - edge).abs() <= 5
}

/// Content pane width (between sidebar and preview).
pub fn content_w(app: &App) -> i32 {
    let w = app.width as i32 - content_x(app) - preview_w(app);
    if w > 0 {
        w
    } else {
        1
    }
}

// ----------------------------------------------------------------------
// split pane (two tabs side by side)
// ----------------------------------------------------------------------

/// Split mode is on *and* the focused tab has a right pane.
pub fn split_active(app: &App) -> bool {
    app.tab().split && app.tab().pane.is_some()
}

/// Clamp a split-pane width (left pane) to a sane range. The divider may be
/// dragged almost to either edge — each pane keeps a small minimum so it
/// still has room for its scrollbar.
pub fn clamp_split_width(app: &App, w: i32) -> i32 {
    let min = 48;
    let max = (content_w(app) - 48).max(min + 40);
    w.clamp(min, max)
}

/// x of the divider between the left (cur_tab) and right (split_tab) pane.
pub fn split_x(app: &App) -> i32 {
    let cw = content_w(app);
    let w = if app.split_width > 0 {
        clamp_split_width(app, app.split_width)
    } else {
        cw / 2
    };
    content_x(app) + w
}

/// Is (x, y) on the split divider (draggable to resize the panes)?
pub fn split_splitter_at(app: &App, x: i32, y: i32) -> bool {
    if !split_active(app) {
        return false;
    }
    let x0 = content_x(app);
    let w = content_w(app);
    if x < x0 || x >= x0 + w {
        return false;
    }
    let sx = split_x(app);
    let y0 = bar_h(app);
    let top = app.height as i32 - status_h(app);
    y >= y0 && y < top && (x - sx).abs() <= 3
}

/// Pane under (x, y) when split is active: 0 = left (cur_tab), 1 = right
/// (split_tab), -1 = outside the content region (top bars / sidebar / preview).
pub fn pane_of(app: &App, x: i32, _y: i32) -> i32 {
    if !split_active(app) {
        return -1;
    }
    let x0 = content_x(app);
    let w = content_w(app);
    if x < x0 || x >= x0 + w {
        return -1;
    }
    if x < split_x(app) {
        0
    } else {
        1
    }
}

/// (x0, w) of a split pane: 0 = left, 1 = right. Without split both return
/// the full content region.
pub fn pane_geom(app: &App, pane: i32) -> (i32, i32) {
    let x0 = content_x(app);
    if !split_active(app) {
        return (x0, content_w(app));
    }
    let sx = split_x(app);
    if pane == 1 {
        (sx, x0 + content_w(app) - sx)
    } else {
        (x0, sx - x0)
    }
}

pub fn visible_rows_for(app: &App, t: &crate::tab::Tab) -> usize {
    let v = list_h_for(app, t) / row_h(app);
    if v > 0 {
        v as usize
    } else {
        1
    }
}

pub fn visible_rows(app: &App) -> usize {
    visible_rows_for(app, &app.tabs[app.cur_tab])
}

#[allow(dead_code)]
pub fn row_at(app: &App, x: i32, y: i32) -> i32 {
    let idx = vis_index_at(app, x, y);
    if idx < 0 {
        return -1;
    }
    let scroll = app.tabs[app.cur_tab].scroll as i32;
    // list/compact: return row relative to viewport (legacy)
    match app.tabs[app.cur_tab].view {
        ViewMode::Grid => idx - scroll * grid_cols(app) as i32,
        _ => idx - scroll,
    }
}

/// Absolute index into vis[] under pointer for an explicit tab + pane
/// geometry, or -1.
pub fn vis_index_at_for(app: &App, t: &crate::tab::Tab, x0: i32, w: i32, x: i32, y: i32) -> i32 {
    if x < x0 || x >= x0 + w {
        return -1;
    }
    let y0 = list_y_for(app, t);
    let h = list_h_for(app, t);
    if y < y0 || y >= y0 + h {
        return -1;
    }
    match t.view {
        ViewMode::Grid => {
            let cols = grid_cols_in(app, w).max(1);
            let cell = app.cfg.grid_cell + app.cfg.padding;
            let c = ((x - x0 - app.cfg.padding) / cell.max(1)).max(0) as usize;
            let r = ((y - y0 - app.cfg.padding) / cell.max(1)).max(0) as usize;
            if c >= cols {
                return -1;
            }
            let i = (t.scroll * cols + r * cols + c) as i32;
            if i < 0 || i as usize >= t.nvis() {
                -1
            } else {
                i
            }
        }
        _ => {
            let rel = (y - y0) / row_h(app);
            let i = t.scroll as i32 + rel;
            if i < 0 || i as usize >= t.nvis() {
                -1
            } else {
                i
            }
        }
    }
}

/// Absolute index into vis[] under pointer (focused/left pane), or -1.
pub fn vis_index_at(app: &App, x: i32, y: i32) -> i32 {
    let (x0, w) = pane_geom(app, 0);
    vis_index_at_for(app, &app.tabs[app.cur_tab], x0, w, x, y)
}

pub fn scrollbar_geom(app: &App) -> Option<(i32, i32, i32)> {
    let (x0, w) = pane_geom(app, 0);
    scrollbar_geom_for(app, &app.tabs[app.cur_tab], x0, w)
}

/// Scroll units of a tab, matching how `scroll` is counted: one row per item
/// for list/compact, one row per grid row for grid. Returns
/// `(visible_units, total_units)`.
#[allow(clippy::manual_div_ceil)]
pub fn scroll_units(app: &App, t: &crate::tab::Tab, w: i32) -> (usize, usize) {
    match t.view {
        ViewMode::List | ViewMode::Compact => (visible_rows_for(app, t).max(1), t.nvis()),
        ViewMode::Grid => {
            let cols = grid_cols_in(app, w).max(1);
            let vrows = grid_rows_for(app, t).max(1);
            let tot = (t.nvis() + cols - 1) / cols;
            (vrows, tot.max(1))
        }
    }
}

pub fn scrollbar_geom_for(app: &App, t: &crate::tab::Tab, x0: i32, w: i32) -> Option<(i32, i32, i32)> {
    let (vis, tot) = scroll_units(app, t, w);
    if tot <= vis {
        return None;
    }
    let y0 = list_y_for(app, t);
    let track_h = list_h_for(app, t);
    let mut thumb_h = track_h * vis as i32 / tot as i32;
    if thumb_h < 8 {
        thumb_h = 8;
    }
    let max_scroll = tot - vis;
    let sx = x0 + w - 10;
    let sy = y0 + ((t.scroll * (track_h - thumb_h) as usize) / max_scroll) as i32;
    Some((sx, sy, thumb_h))
}

/// Max scroll index for a given tab + pane geometry (in scroll units).
pub fn max_scroll_for(app: &App, t: &crate::tab::Tab, _x0: i32, w: i32) -> usize {
    let (vis, tot) = scroll_units(app, t, w);
    tot.saturating_sub(vis)
}

/// Max scroll index for the focused (left) pane.
pub fn max_scroll(app: &App) -> usize {
    let (x0, w) = pane_geom(app, 0);
    max_scroll_for(app, &app.tabs[app.cur_tab], x0, w)
}

/// Visible scroll units for the focused (left) pane.
pub fn visible_units(app: &App) -> usize {
    let (_x0, w) = pane_geom(app, 0);
    scroll_units(app, &app.tabs[app.cur_tab], w).0
}

pub fn scroll_drag_for(app: &mut App, pane: i32, _x0: i32, w: i32, y: i32, grab_off: i32) {
    let (vis, tot, y0, track_h) = {
        let t = app.pane_tab(pane);
        let (vis, tot) = scroll_units(app, t, w);
        (vis, tot, list_y_for(app, t), list_h_for(app, t))
    };
    let t = app.pane_tab_mut(pane);
    if tot <= vis {
        return;
    }
    let mut thumb_h = track_h * vis as i32 / tot as i32;
    if thumb_h < 8 {
        thumb_h = 8;
    }
    let max_scroll = tot - vis;
    let range = track_h - thumb_h;
    let pos = (y - grab_off) - y0;
    let s = if range > 0 {
        pos * max_scroll as i32 / range
    } else {
        0
    };
    let s = s.max(0).min(max_scroll as i32) as usize;
    if s != t.scroll {
        t.scroll = s;
        app.dirty = true;
    }
}

pub fn grid_cols_in(app: &App, w: i32) -> usize {
    let c = (w - app.cfg.padding) / (app.cfg.grid_cell + app.cfg.padding);
    if c > 0 {
        c as usize
    } else {
        1
    }
}

pub fn grid_cols(app: &App) -> usize {
    let (_, w) = pane_geom(app, 0);
    grid_cols_in(app, w)
}

pub fn grid_rows_for(app: &App, t: &crate::tab::Tab) -> usize {
    let r = list_h_for(app, t) / (app.cfg.grid_cell + app.cfg.padding);
    if r > 0 {
        r as usize
    } else {
        1
    }
}

pub fn grid_rows(app: &App) -> usize {
    grid_rows_for(app, &app.tabs[app.cur_tab])
}

fn text_width(app: &App, s: &str) -> i32 {
    app.text
        .as_ref()
        .map(|t| t.width(s))
        .unwrap_or(s.len() as f32 * 8.0) as i32
}

pub fn tab_bar_h(app: &App) -> i32 {
    line_h(app) + app.cfg.padding
}

pub fn tab_bar_y(app: &App) -> i32 {
    menu_bar_h(app)
}

pub fn tab_at(app: &App, x: i32, y: i32) -> Option<usize> {
    let y0 = tab_bar_y(app);
    if y < y0 || y >= y0 + tab_bar_h(app) || x < app.cfg.padding {
        return None;
    }
    let mut bx = app.cfg.padding;
    for (i, t) in app.tabs.iter().enumerate() {
        let w = crate::render::tab_width(app, t);
        if x >= bx && x < bx + w {
            return Some(i);
        }
        bx += w + 2;
        if bx > app.width as i32 {
            break;
        }
    }
    None
}

/// (x, w) of the tab at index `i` in the tab bar (None if clipped out).
pub fn tab_rect(app: &App, i: usize) -> Option<(i32, i32)> {
    if i >= app.tabs.len() {
        return None;
    }
    let mut bx = app.cfg.padding;
    for (j, t) in app.tabs.iter().enumerate() {
        let w = crate::render::tab_width(app, t);
        if j == i {
            return Some((bx, w));
        }
        bx += w + 2;
        if bx > app.width as i32 {
            break;
        }
    }
    None
}

/// The per-tab close (X) button rect: a `line_h`-square at the tab's right
/// edge, vertically centered.
pub fn tab_close_rect(app: &App, i: usize) -> Option<(i32, i32, i32, i32)> {
    let (x, w) = tab_rect(app, i)?;
    let y0 = tab_bar_y(app);
    let th = tab_bar_h(app);
    let s = line_h(app).min(th - 2);
    let cx = x + w - app.cfg.padding - s;
    let cy = y0 + (th - s) / 2;
    Some((cx, cy, s, s))
}

/// Which tab's close (X) button is at (x, y)?
pub fn tab_close_at(app: &App, x: i32, y: i32) -> Option<usize> {
    let y0 = tab_bar_y(app);
    if y < y0 || y >= y0 + tab_bar_h(app) {
        return None;
    }
    for i in 0..app.tabs.len() {
        if let Some((cx, cy, s, sh)) = tab_close_rect(app, i) {
            if x >= cx && x < cx + s && y >= cy && y < cy + sh {
                return Some(i);
            }
        }
    }
    None
}

pub fn header_key_at_for(app: &App, t: &crate::tab::Tab, x0: i32, w: i32, x: i32, y: i32) -> Option<SortKey> {
    if t.view != ViewMode::List {
        return None;
    }
    let y0 = header_y(app);
    let hh = line_h(app) + app.cfg.padding;
    if y < y0 || y >= y0 + hh {
        return None;
    }
    if x < x0 || x >= x0 + w {
        return None;
    }
    let (xs, xm) = crate::render::header_cols(app, x0, w);
    if x < xs {
        Some(SortKey::Name)
    } else if x < xm {
        Some(SortKey::Size)
    } else {
        Some(SortKey::Mtime)
    }
}

pub fn header_key_at(app: &App, x: i32, y: i32) -> Option<SortKey> {
    let (x0, w) = pane_geom(app, 0);
    header_key_at_for(app, &app.tabs[app.cur_tab], x0, w, x, y)
}

pub fn nav_button_w(app: &App) -> i32 {
    let w = line_h(app) + 2;
    if w > 0 {
        w
    } else {
        14
    }
}

pub fn nav_gap(_app: &App) -> i32 {
    2
}

/// Nav button rect: 0=back 1=forward 2=up 3=home.
pub fn path_bar_y(app: &App) -> i32 {
    menu_bar_h(app) + tab_bar_h(app)
}

fn view_btn_w(app: &App) -> i32 {
    // fixed-size icon buttons (drawn with primitives, no font glyphs)
    line_h(app) + app.cfg.padding
}

/// Width of one toolbar item in the path bar (Location is flexible).
pub fn toolbar_item_w(app: &App, it: &crate::toolbar::ToolbarItem) -> i32 {
    match it {
        crate::toolbar::ToolbarItem::Back
        | crate::toolbar::ToolbarItem::Forward
        | crate::toolbar::ToolbarItem::Up
        | crate::toolbar::ToolbarItem::Home => nav_button_w(app),
        crate::toolbar::ToolbarItem::Split
        | crate::toolbar::ToolbarItem::Search
        | crate::toolbar::ToolbarItem::List
        | crate::toolbar::ToolbarItem::Grid
        | crate::toolbar::ToolbarItem::Compact => view_btn_w(app),
        crate::toolbar::ToolbarItem::Separator => 6,
        crate::toolbar::ToolbarItem::Location => 0,
        crate::toolbar::ToolbarItem::Custom(n) => {
            let w = text_width(app, n);
            (w + app.cfg.padding * 2).max(nav_button_w(app))
        }
    }
}

/// Visible toolbar items in config order: (index into `cfg.toolbar`, item).
/// The Location bar is always visible.
pub fn toolbar_visible(app: &App) -> Vec<(usize, crate::toolbar::ToolbarItem)> {
    app.cfg
        .toolbar
        .iter()
        .enumerate()
        .filter(|(_, it)| {
            let k = crate::toolbar::item_key(it);
            **it == crate::toolbar::ToolbarItem::Location || !app.cfg.toolbar_hidden.contains(&k)
        })
        .map(|(i, it)| (i, it.clone()))
        .collect()
}

/// x just after the last left-aligned toolbar item (start of the tray).
pub fn toolbar_left_end(app: &App) -> i32 {
    let mut x = app.cfg.padding;
    for (_, it) in toolbar_visible(app) {
        if it == crate::toolbar::ToolbarItem::Location {
            break;
        }
        x += toolbar_item_w(app, &it) + nav_gap(app);
    }
    x
}

/// x just before the first right-aligned toolbar item (end of the tray).
pub fn toolbar_right_start(app: &App) -> i32 {
    let mut w = 0;
    let mut seen_loc = false;
    for (_, it) in toolbar_visible(app) {
        if it == crate::toolbar::ToolbarItem::Location {
            seen_loc = true;
            continue;
        }
        if seen_loc {
            w += toolbar_item_w(app, &it) + nav_gap(app);
        }
    }
    app.width as i32 - app.cfg.padding - w
}

/// Breadcrumb / URL-tray x0.
pub fn breadcrumb_x0(app: &App) -> i32 {
    toolbar_left_end(app) + app.cfg.padding / 2
}

/// x of the right-aligned toolbar cluster (items after Location).
pub fn right_btn_x0(app: &App) -> i32 {
    toolbar_right_start(app)
}

/// (x, y, w, h) of the toolbar item at config index `idx` (None if hidden or
/// clipped). Location yields the whole tray region.
pub fn toolbar_item_rect(app: &App, idx: usize) -> Option<(i32, i32, i32, i32)> {
    let lh = line_h(app);
    let y0 = path_bar_y(app);
    let ph = lh + app.cfg.padding * 2;
    let y = y0 + 2;
    let h = ph - 4;
    let left_end = toolbar_left_end(app);
    let right_start = toolbar_right_start(app);
    // item sits left of the tray when it comes before Location in config order
    let mut lx = app.cfg.padding;
    let mut rx = right_start;
    let mut after_loc = false;
    for (i, it) in toolbar_visible(app) {
        if it == crate::toolbar::ToolbarItem::Location {
            if i == idx {
                return Some((left_end, y, (right_start - left_end).max(40), h));
            }
            after_loc = true;
            continue;
        }
        if i != idx {
            let w = toolbar_item_w(app, &it);
            if after_loc {
                rx += w + nav_gap(app);
            } else {
                lx += w + nav_gap(app);
            }
            continue;
        }
        let w = toolbar_item_w(app, &it);
        let (x, cursor) = if after_loc {
            (rx, &mut rx)
        } else {
            (lx, &mut lx)
        };
        let rect = (x, y, w, h);
        *cursor += w + nav_gap(app);
        return Some(rect);
    }
    None
}

/// Is (x, y) inside the path bar at all?
fn path_bar_at(app: &App, x: i32, y: i32) -> bool {
    let lh = line_h(app);
    let y0 = path_bar_y(app);
    let ph = lh + app.cfg.padding * 2;
    y >= y0 && y < y0 + ph && x >= 0 && x < app.width as i32
}

/// The nav button (0=back 1=forward 2=up 3=home) under (x, y), or -1.
pub fn nav_at(app: &App, x: i32, y: i32) -> i32 {
    if !path_bar_at(app, x, y) {
        return -1;
    }
    for (i, it) in toolbar_visible(app) {
        let kind = match it {
            crate::toolbar::ToolbarItem::Back => 0,
            crate::toolbar::ToolbarItem::Forward => 1,
            crate::toolbar::ToolbarItem::Up => 2,
            crate::toolbar::ToolbarItem::Home => 3,
            _ => continue,
        };
        if let Some((rx, ry, rw, rh)) = toolbar_item_rect(app, i) {
            if x >= rx && x < rx + rw && y >= ry && y < ry + rh {
                return kind;
            }
        }
    }
    -1
}

/// Which toolbar item (if any) is under (x, y)? Used by pointer dispatch.
pub fn toolbar_at(app: &App, x: i32, y: i32) -> Option<(usize, crate::toolbar::ToolbarItem)> {
    if !path_bar_at(app, x, y) {
        return None;
    }
    for (i, it) in toolbar_visible(app) {
        if it == crate::toolbar::ToolbarItem::Location {
            continue;
        }
        if let Some((rx, ry, rw, rh)) = toolbar_item_rect(app, i) {
            if x >= rx && x < rx + rw && y >= ry && y < ry + rh {
                return Some((i, it));
            }
        }
    }
    None
}

/// True if pointer is over the Thunar-style location tray (not nav/view buttons).
pub fn location_tray_at(app: &App, x: i32, y: i32) -> bool {
    if !path_bar_at(app, x, y) {
        return false;
    }
    let left = toolbar_left_end(app);
    let right = toolbar_right_start(app);
    x >= left && x < right
}

/// Geometry of the red "close search" X at the right end of the location tray
/// while inline search is active.
pub fn search_close_rect(app: &App) -> (i32, i32, i32, i32) {
    let lh = line_h(app);
    let y0 = path_bar_y(app);
    let ph = lh + app.cfg.padding * 2;
    let view_w = view_btn_w(app);
    let right = right_btn_x0(app) - app.cfg.padding;
    (right - view_w, y0, view_w, ph)
}

pub fn search_close_at(app: &App, x: i32, y: i32) -> bool {
    if app.input_mode != crate::InputMode::Search && app.input_mode != crate::InputMode::Url {
        return false;
    }
    let (rx, ry, rw, rh) = search_close_rect(app);
    x >= rx && x < rx + rw && y >= ry && y < ry + rh
}

/// Editable location-field rect while the URL bar is being edited (between
/// the tray start and the drop-down button).
pub fn url_field_rect(app: &App) -> Rect {
    let lh = line_h(app);
    let y0 = path_bar_y(app);
    let ph = lh + app.cfg.padding * 2;
    let (rx, _, _, _) = search_close_rect(app);
    let left = breadcrumb_x0(app) - app.cfg.padding / 2;
    let w = (rx - view_btn_w(app) * 2 - 4 - left).max(40);
    Rect { x: left, y: y0, w, h: ph }
}

/// Tick/go button (navigates to the typed path), just left of the red X.
pub fn url_go_rect(app: &App) -> Rect {
    let (rx, ry, rw, rh) = search_close_rect(app);
    Rect { x: rx - rw, y: ry, w: rw, h: rh }
}

/// Drop-down/history button, just left of the tick button.
pub fn url_drop_rect(app: &App) -> Rect {
    let (rx, ry, rw, rh) = search_close_rect(app);
    Rect { x: rx - rw * 2, y: ry, w: rw, h: rh }
}

pub fn url_go_at(app: &App, x: i32, y: i32) -> bool {
    app.input_mode == crate::InputMode::Url && url_go_rect(app).contains(x, y)
}

pub fn url_drop_at(app: &App, x: i32, y: i32) -> bool {
    app.input_mode == crate::InputMode::Url && url_drop_rect(app).contains(x, y)
}

/// True if (x, y) is inside the editable URL field (only while URL editing).
pub fn url_field_at(app: &App, x: i32, y: i32) -> bool {
    app.input_mode == crate::InputMode::Url && url_field_rect(app).contains(x, y)
}

/// Byte offset in `app.input` under pixel `x` (nearest char boundary).
pub fn url_caret_at(app: &App, x: i32) -> usize {
    let f = url_field_rect(app);
    let x0 = f.x + app.cfg.padding / 2;
    let input = &app.input;
    let mut best = 0usize;
    let mut best_dx = (x - x0).abs();
    for (i, c) in input.char_indices() {
        let b = i + c.len_utf8();
        let nx = x0 + text_width(app, &input[..b]);
        let d = (x - nx).abs();
        if d < best_dx {
            best_dx = d;
            best = b;
        }
    }
    best
}

/// Sidebar row kinds (shared between hit-testing and rendering).
#[derive(Clone, PartialEq, Debug)]
pub enum SideKind {
    Header(&'static str),
    Gap,
    Place(u8),
    Volume(usize),
    Network(usize),
    Unmounted(usize),
    Bookmark(usize),
    /// Removable media (USB sticks, card readers, …) — mounted or unmounted,
    /// shown in their own section below Volumes so they're never mixed with
    /// the internal volumes. The index points into the app's removable list.
    Removable(usize),
}

pub struct SideRow {
    pub y: i32,
    pub h: i32,
    pub kind: SideKind,
}

/// Default place shortcut keys in order (Home, Documents, Downloads, Music,
/// Videos, Trash). Each can be hidden from the sidebar; hidden ones are
/// skipped by `sidebar_rows` and can be restored from View → Hidden Places.
pub const DEFAULT_PLACES: &[&str] = &["home", "documents", "downloads", "music", "videos", "trash"];

/// Is a default place (key like "documents") hidden in the sidebar?
pub fn place_hidden(app: &App, key: &str) -> bool {
    app.hidden_places.iter().any(|h| h == key)
}

/// Row id for a default place key (0..6, matching `SideKind::Place`).
/// home=0 documents=1 downloads=2 music=3 videos=4 trash=5. -1 for unknown.
pub fn place_id(key: &str) -> i32 {
    match key {
        "home" => 0,
        "documents" => 1,
        "downloads" => 2,
        "music" => 3,
        "videos" => 4,
        "trash" => 5,
        _ => -1,
    }
}

/// Build the sidebar rows. Places = the six default shortcuts (Home,
/// Documents, Downloads, Music, Videos, Trash, minus hidden ones) + user
/// bookmarks (merged in after the built-in places, no separate Bookmarks
/// header). Volumes holds File System (root), mounted volumes and (muted)
/// unmounted ones; Network is only shown when it has entries. Sections with
/// zero entries are skipped.
///
/// The vertical scroll offset (`App::side_scroll`, in whole rows) shifts the
/// rows up so overflowing content stays reachable.
pub fn sidebar_rows(app: &App) -> Vec<SideRow> {
    let off = app.side_scroll.min(side_scroll_max(app)) as i32 * line_h(app);
    let mut rows = sidebar_rows_raw(app);
    for r in &mut rows {
        r.y -= off;
    }
    rows
}

/// Unscrolled sidebar rows (scroll offset zero).
fn sidebar_rows_raw(app: &App) -> Vec<SideRow> {
    let lh = line_h(app);
    let gap = lh / 2;
    let mut rows = Vec::new();
    let mut y = bar_h(app);
    let mut push = |kind: SideKind, h: i32| {
        rows.push(SideRow { y, h, kind });
        y += h;
    };
    push(SideKind::Header("PLACES"), lh);
    for (i, key) in DEFAULT_PLACES.iter().enumerate() {
        if !place_hidden(app, key) {
            push(SideKind::Place(i as u8), lh);
        }
    }
    // bookmarks live inside Places, right after the built-ins
    if app.bookmark_n > 0 {
        push(SideKind::Gap, gap / 2);
        for i in 0..app.bookmark_n {
            push(SideKind::Bookmark(i), lh);
        }
    }
    push(SideKind::Gap, gap);
    push(SideKind::Header("VOLUMES"), lh);
    push(SideKind::Place(6), lh); // File System (root)
    for i in 0..app.side_n {
        push(SideKind::Volume(i), lh);
    }
    if app.unmounted_n > 0 {
        push(SideKind::Gap, gap / 2);
        for i in 0..app.unmounted_n {
            push(SideKind::Unmounted(i), lh);
        }
    }
    // removable media live in their own section, below the internal volumes
    if app.removable_n > 0 {
        push(SideKind::Gap, gap);
        push(SideKind::Header("REMOVABLE"), lh);
        for i in 0..app.removable_n {
            push(SideKind::Removable(i), lh);
        }
    }
    if app.net_n > 0 {
        push(SideKind::Gap, gap);
        push(SideKind::Header("NETWORK"), lh);
        for i in 0..app.net_n {
            push(SideKind::Network(i), lh);
        }
    }
    rows
}

/// Max sidebar scroll offset in whole rows (0 when everything fits).
pub fn side_scroll_max(app: &App) -> usize {
    if !app.sidebar_visible {
        return 0;
    }
    let lh = line_h(app);
    let y0 = bar_h(app);
    let top = app.height as i32 - status_h(app);
    let rows = sidebar_rows_raw(app);
    let total = rows.last().map(|r| r.y + r.h - y0).unwrap_or(0);
    let visible = top - y0;
    if total <= visible {
        0
    } else {
        let overflow = total - visible;
        // ceil(overflow / lh) without int_roundings (older toolchain)
        ((overflow + lh - 1) / lh).max(1) as usize
    }
}

/// Sidebar scrollbar thumb geometry: (sx, sy, thumb_h). None when the
/// sidebar content fits (no scrollbar).
pub fn sidebar_scrollbar_geom(app: &App) -> Option<(i32, i32, i32)> {
    if !app.sidebar_visible {
        return None;
    }
    let max = side_scroll_max(app);
    if max == 0 {
        return None;
    }
    let y0 = bar_h(app);
    let top = app.height as i32 - status_h(app);
    let track_h = top - y0;
    let rows = sidebar_rows_raw(app);
    let total = rows.last().map(|r| r.y + r.h - y0).unwrap_or(track_h).max(1);
    let mut thumb_h = track_h * track_h / total;
    if thumb_h < 8 {
        thumb_h = 8;
    }
    let sx = content_x(app) - 10;
    let sy = y0 + (app.side_scroll.min(max) * (track_h - thumb_h) as usize / max) as i32;
    Some((sx, sy, thumb_h))
}

/// Drag the sidebar scrollbar thumb to the pointer (mirror of the content
/// scrollbar's `scroll_drag_for`).
pub fn side_scroll_drag_to(app: &mut App, y: i32, grab_off: i32) {
    let max = side_scroll_max(app);
    if max == 0 {
        return;
    }
    let y0 = bar_h(app);
    let top = app.height as i32 - status_h(app);
    let track_h = top - y0;
    let rows = sidebar_rows_raw(app);
    let total = rows.last().map(|r| r.y + r.h - y0).unwrap_or(track_h).max(1);
    let mut thumb_h = track_h * track_h / total;
    if thumb_h < 8 {
        thumb_h = 8;
    }
    let range = (track_h - thumb_h).max(1);
    let pos = ((y - grab_off) - y0).clamp(0, range);
    app.side_scroll = (pos as usize * max / range as usize).min(max);
    app.dirty = true;
}

/// Encode a clickable sidebar row kind as the numeric id used by
/// `App::sidebar_nav`: 0-6 places (0 home, 1 documents, 2 downloads,
/// 3 music, 4 videos, 5 trash, 6 File System), 10+i volumes, 100+i network,
/// 200+i unmounted (under Volumes), 300+i bookmarks.
pub fn side_kind_id(kind: &SideKind) -> i32 {
    match kind {
        SideKind::Place(p) => *p as i32,
        SideKind::Volume(i) => 10 + *i as i32,
        SideKind::Network(i) => 100 + *i as i32,
        SideKind::Unmounted(i) => 200 + *i as i32,
        SideKind::Bookmark(i) => 300 + *i as i32,
        SideKind::Removable(i) => 400 + *i as i32,
        _ => -1,
    }
}

/// Sidebar row id under pointer (0-6 places, 10+i volumes, 100+i network,
/// 200+i unmounted, 300+i bookmarks), or -1.
pub fn sidebar_at(app: &App, x: i32, y: i32) -> i32 {
    if !app.sidebar_visible {
        return -1;
    }
    if x < 0 || x >= content_x(app) {
        return -1;
    }
    let y0 = bar_h(app);
    let top = app.height as i32 - status_h(app);
    if y < y0 || y >= top {
        return -1;
    }
    for row in sidebar_rows(app) {
        if y >= row.y && y < row.y + row.h {
            return side_kind_id(&row.kind);
        }
    }
    -1
}

/// True when (x, y) is over the sidebar pane (left of the file list).
pub fn sidebar_area_at(app: &App, x: i32, y: i32) -> bool {
    if !app.sidebar_visible {
        return false;
    }
    let y0 = bar_h(app);
    let top = app.height as i32 - status_h(app);
    x >= 0 && x < content_x(app) && y >= y0 && y < top
}

/// True when (x, y) is over the middle file pane (between sidebar and
/// preview) — the only place a DnD drop popup is offered.
pub fn middle_area_at(app: &App, x: i32, y: i32) -> bool {
    let y0 = bar_h(app);
    let top = app.height as i32 - status_h(app);
    let x0 = content_x(app);
    x >= x0 && x < x0 + content_w(app) && y >= y0 && y < top
}

/// True when a DnD drop at (x, y) should open the copy/move/link popup:
/// the **left (focused) file pane only**. The right split pane, the preview
/// pane and window chrome all cancel the drop instead.
pub fn dnd_popup_area_at(app: &App, x: i32, y: i32) -> bool {
    middle_area_at(app, x, y) && pane_of(app, x, y) != 1
}

/// True if (x, y) is inside the list/grid viewport but not on any entry —
/// the rubberband start area.
pub fn list_empty_at_for(app: &App, t: &crate::tab::Tab, x0: i32, w: i32, x: i32, y: i32) -> bool {
    let y0 = list_y_for(app, t);
    let h = list_h_for(app, t);
    if x < x0 || x >= x0 + w || y < y0 || y >= y0 + h {
        return false;
    }
    vis_index_at_for(app, t, x0, w, x, y) < 0
}

pub fn list_empty_at(app: &App, x: i32, y: i32) -> bool {
    let (x0, w) = pane_geom(app, 0);
    list_empty_at_for(app, &app.tabs[app.cur_tab], x0, w, x, y)
}

/// Bounding box of visible entry `vi` (index into `vis[]`) for a pane.
pub fn entry_rect_for(app: &App, t: &crate::tab::Tab, x0: i32, w: i32, vi: usize) -> Option<(i32, i32, i32, i32)> {
    let y0 = list_y_for(app, t);
    match t.view {
        ViewMode::Grid => {
            let cols = grid_cols_in(app, w).max(1);
            let cell = (app.cfg.grid_cell + app.cfg.padding) as usize;
            let i = t.scroll * cols + vi;
            let row = i / cols;
            let col = i % cols;
            let ex = x0 + app.cfg.padding + (col * cell) as i32;
            let y = y0 + app.cfg.padding + (row * cell) as i32;
            Some((ex, y, app.cfg.grid_cell, app.cfg.grid_cell))
        }
        _ => {
            let row = (t.scroll + vi) as i32;
            Some((x0, y0 + row * row_h(app), w, row_h(app)))
        }
    }
}


pub fn path_seg_at(app: &App, x: i32, y: i32) -> Option<PathBuf> {
    let lh = line_h(app);
    let y0 = path_bar_y(app);
    let ph = lh + app.cfg.padding * 2;
    if y < y0 || y >= y0 + ph {
        return None;
    }
    if x < breadcrumb_x0(app) || x >= toolbar_right_start(app) - app.cfg.padding / 2 {
        return None;
    }
    let t = &app.tabs[app.cur_tab];
    let cwd = t.cwd.display().to_string();
    let segs: Vec<&str> = if cwd == "/" {
        vec!["/"]
    } else {
        let mut v: Vec<&str> = cwd.split('/').filter(|s| !s.is_empty()).collect();
        v.insert(0, "/");
        v
    };
    let mut bx = breadcrumb_x0(app);
    if !t.filter.is_empty() {
        let fl = format!(
            "[filter: {}{}]  ",
            if t.filter_regex { "re " } else { "" },
            t.filter
        );
        bx += text_width(app, &fl);
    }
    let mut prefix = String::new();
    for (i, seg) in segs.iter().enumerate() {
        if i > 1 {
            bx += text_width(app, "/");
        }
        if i == 0 {
            prefix = "/".to_string();
        } else {
            if !prefix.ends_with('/') {
                prefix.push('/');
            }
            prefix.push_str(seg);
        }
        let seg_w = text_width(app, seg);
        if x >= bx && x < bx + seg_w {
            return Some(PathBuf::from(prefix));
        }
        bx += seg_w;
        if bx > app.width as i32 - app.cfg.padding {
            break;
        }
    }
    None
}

pub const RIBBON: &[&str] = &["File", "Edit", "View", "Theme", "Go", "Shortcuts", "Help"];

pub fn menu_bar_at(app: &App, x: i32, y: i32) -> Option<usize> {
    if y < 0 || y >= menu_bar_h(app) {
        return None;
    }
    let mut bx = app.cfg.padding;
    for (i, label) in RIBBON.iter().enumerate() {
        let w = text_width(app, label) + app.cfg.padding * 2;
        if x >= bx && x < bx + w {
            return Some(i);
        }
        bx += w + 4;
        if bx > app.width as i32 {
            break;
        }
    }
    None
}

pub fn menu_item_at(app: &App, menu: &crate::menu::Menu, x: i32, y: i32) -> i32 {
    let lh = line_h(app);
    let pad = app.cfg.padding;
    let mw = menu_width(app, menu);
    let mh = menu_height(app, menu);
    if x < menu.x || x >= menu.x + mw || y < menu.y || y >= menu.y + mh {
        return -1;
    }
    let mut yy = menu.y + pad / 2;
    for (i, it) in menu.items.iter().enumerate() {
        let h = if it.id == crate::menu::MenuId::Separator {
            pad
        } else {
            lh + pad / 2
        };
        if y >= yy && y < yy + h {
            return i as i32;
        }
        yy += h;
    }
    -1
}

pub fn menu_width(app: &App, menu: &crate::menu::Menu) -> i32 {
    let mut w = 120i32;
    for it in &menu.items {
        if it.id == crate::menu::MenuId::Separator {
            continue;
        }
        let mut lab: &str = &it.label;
        // strip the tabbed shortcut text for width purposes
        if let Some(t) = it.label.find('\t') {
            lab = &it.label[..t];
        }
        w = w.max(text_width(app, lab) + app.cfg.padding * 4 + 14);
    }
    w
}

/// Top edge (relative to `menu.y`) of the row for item `i`.
pub fn menu_item_y(app: &App, menu: &crate::menu::Menu, i: usize) -> i32 {
    let lh = line_h(app);
    let pad = app.cfg.padding;
    let mut y = pad / 2;
    for (idx, it) in menu.items.iter().enumerate() {
        if idx == i {
            return y;
        }
        y += if it.id == crate::menu::MenuId::Separator { pad } else { lh + pad / 2 };
    }
    0
}

pub fn menu_height(app: &App, menu: &crate::menu::Menu) -> i32 {
    let lh = line_h(app);
    let pad = app.cfg.padding;
    let mut h = pad;
    for it in &menu.items {
        if it.id == crate::menu::MenuId::Separator {
            h += pad;
        } else {
            h += lh + pad / 2;
        }
    }
    h + pad / 2
}

#[derive(Clone, Copy)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    pub fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.x && x < self.x + self.w && y >= self.y && y < self.y + self.h
    }
}

/// Geometry of the "close with open tabs" confirmation dialog.
pub struct CloseDlgGeom {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub btn_cancel: Rect,
    pub btn_tab: Rect,
    pub btn_window: Rect,
    pub checkbox: Rect,
}

pub fn close_dlg_geom(app: &App) -> Option<CloseDlgGeom> {
    let lh = line_h(app);
    let pad = app.cfg.padding;
    let title = "Close wfm?";
    let msg = format!("{} tabs are open. Close the window?", app.tabs.len());
    let dw = 380
        .max(text_width(app, title) + pad * 6)
        .max(text_width(app, &msg) + pad * 6);
    // 4 rows (title, message, checkbox, buttons) + outer padding.
    let dh = pad * 2 + (lh + pad) * 4;
    let dx = (app.width as i32 - dw) / 2;
    let dy = ((app.height as i32 - dh) / 2).max(bar_h(app));
    let btn_w = (dw - pad * 4) / 3;
    let btn_h = lh + pad;
    let btn_y = dy + dh - pad - btn_h;
    let btn_cancel = Rect { x: dx + pad, y: btn_y, w: btn_w, h: btn_h };
    let btn_tab = Rect { x: dx + pad * 2 + btn_w, y: btn_y, w: btn_w, h: btn_h };
    let btn_window = Rect { x: dx + pad * 3 + btn_w * 2, y: btn_y, w: btn_w, h: btn_h };
    let checkbox = Rect { x: dx + pad, y: btn_y - pad - lh, w: 240, h: lh };
    Some(CloseDlgGeom { x: dx, y: dy, w: dw, h: dh, btn_cancel, btn_tab, btn_window, checkbox })
}

/// Geometry of the "import Thunar custom actions?" confirmation dialog.
pub struct ImportDlgGeom {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub btn_cancel: Rect,
    pub btn_ok: Rect,
}

pub fn import_dlg_geom(app: &App) -> Option<ImportDlgGeom> {
    let lh = line_h(app);
    let pad = app.cfg.padding;
    let title = "Import Thunar Custom Actions?";
    let msg = format!(
        "This will replace your {} current custom action(s).",
        app.import_pending.len()
    );
    let dw = 420
        .max(text_width(app, title) + pad * 6)
        .max(text_width(app, &msg) + pad * 6);
    // 3 rows (title, message, buttons) + outer padding.
    let dh = pad * 2 + (lh + pad) * 3;
    let dx = (app.width as i32 - dw) / 2;
    let dy = ((app.height as i32 - dh) / 2).max(bar_h(app));
    let btn_w = (dw - pad * 3) / 2;
    let btn_h = lh + pad;
    let btn_y = dy + dh - pad - btn_h;
    let btn_cancel = Rect { x: dx + pad, y: btn_y, w: btn_w, h: btn_h };
    let btn_ok = Rect { x: dx + pad * 2 + btn_w, y: btn_y, w: btn_w, h: btn_h };
    Some(ImportDlgGeom { x: dx, y: dy, w: dw, h: dh, btn_cancel, btn_ok })
}

/// Geometry of the "create folder <path>?" confirmation dialog (shown when
/// the URL bar's go/tick targets a path that does not exist).
pub struct MkdirDlgGeom {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub btn_cancel: Rect,
    pub btn_create: Rect,
}

pub fn mkdir_dlg_geom(app: &App) -> Option<MkdirDlgGeom> {
    let path = app.confirm_mkdir.as_deref()?;
    let lh = line_h(app);
    let pad = app.cfg.padding;
    let title = "Folder not found";
    let msg = format!("Create folder \"{path}\"?");
    let dw = 420
        .max(text_width(app, title) + pad * 6)
        .max(text_width(app, &msg) + pad * 6);
    // 3 rows (title, message, buttons) + outer padding.
    let dh = pad * 2 + (lh + pad) * 3;
    let dx = (app.width as i32 - dw) / 2;
    let dy = ((app.height as i32 - dh) / 2).max(bar_h(app));
    let btn_w = (dw - pad * 3) / 2;
    let btn_h = lh + pad;
    let btn_y = dy + dh - pad - btn_h;
    let btn_cancel = Rect { x: dx + pad, y: btn_y, w: btn_w, h: btn_h };
    let btn_create = Rect { x: dx + pad * 2 + btn_w, y: btn_y, w: btn_w, h: btn_h };
    Some(MkdirDlgGeom { x: dx, y: dy, w: dw, h: dh, btn_cancel, btn_create })
}

/// Geometry of the View → Configure Toolbar… dialog.
pub struct ToolbarDlgGeom {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub list: Rect,
    pub btn_up: Rect,
    pub btn_down: Rect,
    pub btn_reset: Rect,
    pub btn_close: Rect,
}

pub fn toolbar_dlg_geom(app: &App) -> Option<ToolbarDlgGeom> {
    let dlg = app.toolbar_dlg.as_ref()?;
    let lh = line_h(app);
    let pad = app.cfg.padding;
    let row_h = lh + pad / 2;
    let title = "Configure Toolbar";
    let dw = 460
        .max(text_width(app, title) + pad * 6)
        .min(app.width as i32 - pad * 2);
    let visible = 8.min(dlg.items.len() as i32).max(1);
    let list_h = row_h * visible + pad;
    let btn_h = lh + pad;
    let dh = pad + (lh + pad) + list_h + pad + btn_h + pad;
    let dx = (app.width as i32 - dw) / 2;
    let dy = ((app.height as i32 - dh) / 2).max(bar_h(app));
    let list = Rect { x: dx + pad, y: dy + pad + (lh + pad), w: dw - pad * 2, h: list_h };
    let by = dy + dh - pad - btn_h;
    let bw = (dw - pad * 5) / 4;
    let btn_up = Rect { x: dx + pad, y: by, w: bw, h: btn_h };
    let btn_down = Rect { x: dx + pad * 2 + bw, y: by, w: bw, h: btn_h };
    let btn_reset = Rect { x: dx + pad * 3 + bw * 2, y: by, w: bw, h: btn_h };
    let btn_close = Rect { x: dx + pad * 4 + bw * 3, y: by, w: bw, h: btn_h };
    Some(ToolbarDlgGeom { x: dx, y: dy, w: dw, h: dh, list, btn_up, btn_down, btn_reset, btn_close })
}

/// Toolbar-dialog item index under (x, y), or None (also when the list is
/// scrolled past the end of the item list).
pub fn toolbar_dlg_row_at(app: &App, x: i32, y: i32) -> Option<usize> {
    let g = toolbar_dlg_geom(app)?;
    if !g.list.contains(x, y) {
        return None;
    }
    let lh = line_h(app);
    let pad = app.cfg.padding;
    let row_h = lh + pad / 2;
    let off = y - g.list.y - pad / 2;
    if off < 0 {
        return None;
    }
    let i = off / row_h;
    let dlg = app.toolbar_dlg.as_ref()?;
    let idx = dlg.scroll.max(0) as usize + i as usize;
    if idx >= dlg.items.len() {
        return None;
    }
    Some(idx)
}

/// True when (x, y) is on the checkbox of a toolbar-dialog row (clicking the
/// checkbox toggles the item; clicking the label only selects).
pub fn toolbar_dlg_check_at(app: &App, x: i32, y: i32) -> bool {
    if toolbar_dlg_row_at(app, x, y).is_none() {
        return false;
    }
    let Some(g) = toolbar_dlg_geom(app) else { return false };
    let lh = line_h(app);
    let pad = app.cfg.padding;
    let row_h = lh + pad / 2;
    let off = y - g.list.y - pad / 2;
    let r = off / row_h;
    let row_y = g.list.y + pad / 2 + r * row_h;
    let cs = (lh - 2).max(10);
    let cb_x = g.list.x + pad;
    x >= cb_x && x < cb_x + cs + 4 && y >= row_y && y < row_y + row_h
}

/// Geometry of the centered input dialog (rename / new folder / new file /
/// location …), including the OK and Cancel buttons.
pub struct InputDlgGeom {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub field: Rect,
    pub btn_ok: Rect,
    pub btn_cancel: Rect,
}

/// None when no modal input dialog is on screen (Search and Url are inline,
/// not modal).
pub fn input_dlg_geom(app: &App) -> Option<InputDlgGeom> {
    if app.input_mode == crate::InputMode::None
        || app.input_mode == crate::InputMode::Search
        || app.input_mode == crate::InputMode::Url
    {
        return None;
    }
    let lh = line_h(app);
    let pad = app.cfg.padding;
    let win_w = app.width as i32;
    let win_h = app.height as i32;
    let title = match app.input_mode {
        crate::InputMode::Rename => "Rename",
        crate::InputMode::NewDir | crate::InputMode::DndNewDir => "New Folder",
        crate::InputMode::NewFile => "New File",
        crate::InputMode::Location => "Go to Location",
        crate::InputMode::Url => "Location",
        crate::InputMode::Filter => "Filter",
        crate::InputMode::NewWindow => "New Window",
        crate::InputMode::OpenWith => "Open With",
        crate::InputMode::SetDefault => "Default App",
        crate::InputMode::Search => "Search",
        crate::InputMode::None => "Input",
    };
    let prompt_w = text_width(app, &app.input_prompt);
    let input_w = text_width(app, &app.input).max(220);
    let title_w = text_width(app, title);
    let hint = "Esc cancel  ·  Enter OK";
    let hint_w = text_width(app, hint);
    let inner_w = (prompt_w + pad + input_w + pad * 2)
        .max(title_w + pad * 2)
        .max(hint_w + pad * 2)
        .min(win_w - pad * 4)
        .max(280);
    // 4 content rows (title, field, hint, buttons) + outer padding.
    let dh = pad * 2 + (lh + pad) * 4;
    let dw = inner_w + pad * 2;
    let dx = ((win_w - dw) / 2).max(pad);
    let dy = ((win_h - dh) / 2).max(bar_h(app));
    let field_x = dx + pad;
    let field_w = dw - pad * 2;
    let field_y = dy + pad * 2 + lh;
    let field_h = lh + pad;
    let btn_w = 64;
    let btn_h = lh + pad;
    let btn_y = dy + dh - pad - btn_h;
    let btn_ok = Rect { x: dx + dw - pad - btn_w, y: btn_y, w: btn_w, h: btn_h };
    let btn_cancel = Rect { x: btn_ok.x - pad - btn_w, y: btn_y, w: btn_w, h: btn_h };
    Some(InputDlgGeom {
        x: dx,
        y: dy,
        w: dw,
        h: dh,
        field: Rect { x: field_x, y: field_y, w: field_w, h: field_h },
        btn_ok,
        btn_cancel,
    })
}

/// Geometry of the properties dialog: box + the four section tabs + a close
/// (X) button at the top right.
pub struct PropsGeom {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    /// Tab strip rect (clickable across its whole width).
    pub tabs: Rect,
    /// Close "X" button at the top-right corner of the dialog.
    pub close: Rect,
}

const PROPS_TABS: [&str; 4] = ["General", "Permissions", "Checksum", "Details"];

pub fn props_geom(app: &App) -> Option<PropsGeom> {
    let p = app.props.as_ref()?;
    let lh = line_h(app);
    let pad = app.cfg.padding;
    let label_w = text_width(app, "location").max(60);
    let val_w = text_width(app, &p.path).min(app.width as i32 - pad * 6);
    let dw = (label_w + pad + val_w + pad * 2).min(app.width as i32 - pad * 2).max(360);
    // title band + tabs row + content rows + close hint + outer padding.
    let dh = pad * 2 + (lh + pad) * 9;
    let dx = (app.width as i32 - dw) / 2;
    let dy = ((app.height as i32 - dh) / 2).max(bar_h(app));
    // title band: file name at left, close (X) button at top-right
    let title_y = dy + pad;
    let cs = lh.max(16);
    let tabs = Rect { x: dx + pad, y: title_y + lh + pad, w: dw - pad * 2, h: lh + pad / 2 };
    let close = Rect { x: dx + dw - pad - cs, y: dy + pad / 2, w: cs, h: cs };
    Some(PropsGeom { x: dx, y: dy, w: dw, h: dh, tabs, close })
}

/// Returns the props section tab index under (x, y), or None.
pub fn props_tab_at(app: &App, x: i32, y: i32) -> Option<i32> {
    let g = props_geom(app)?;
    if !g.tabs.contains(x, y) {
        return None;
    }
    let n = PROPS_TABS.len() as i32;
    let tw = g.tabs.w / n;
    let i = (x - g.tabs.x) / tw;
    Some(i.clamp(0, n - 1))
}

/// Center of the active props tab, for drawing the underline.
pub fn props_tab_center(app: &App, i: i32) -> Option<(i32, i32)> {
    let g = props_geom(app)?;
    let n = PROPS_TABS.len() as i32;
    let tw = g.tabs.w / n;
    Some((g.tabs.x + tw * i + tw / 2, g.tabs.y + g.tabs.h))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::App;
    use crate::config::Config;

    #[test]
    fn hidden_places_are_skipped_in_rows() {
        let cfg = Config::default();
        let mut app = App::new(cfg);
        app.width = 800;
        app.height = 500;
        app.sidebar_visible = true;
        app.hidden_places = vec!["downloads".to_string(), "music".to_string()];
        let rows = sidebar_rows(&app);
        let ids: Vec<i32> = rows.iter().map(|r| side_kind_id(&r.kind)).collect();
        // home=0, documents=1 present; downloads=2 and music=3 absent; trash=5, root=6
        assert!(ids.contains(&0));
        assert!(ids.contains(&1));
        assert!(!ids.contains(&2));
        assert!(!ids.contains(&3));
        assert!(ids.contains(&5));
        assert!(ids.contains(&6));
        assert!(place_id("documents") == 1);
        assert!(place_id("bogus") < 0);
    }

    /// A DnD drop only opens the copy/move/link popup on the left (focused)
    /// pane; the right split pane cancels the drop instead.
    #[test]
    fn dnd_popup_only_on_left_pane() {
        let cfg = Config::default();
        let mut app = App::new(cfg);
        app.width = 1200;
        app.height = 700;
        app.sidebar_visible = true;
        app.tabs.push(crate::tab::Tab::new(0, PathBuf::from("/tmp")));
        app.tabs.push(crate::tab::Tab::new(1, PathBuf::from("/usr")));
        app.cur_tab = 0;
        app.tabs[0].split = true;
        app.tabs[0].pane = Some(Box::new(crate::tab::Tab::new(2, PathBuf::from("/usr"))));
        let x0 = content_x(&app);
        let sx = split_x(&app);
        let y = 300;
        assert!(dnd_popup_area_at(&app, x0 + 10, y), "left pane accepts drops");
        assert!(!dnd_popup_area_at(&app, sx + 10, y), "right pane cancels drops");
        // without split the whole content region accepts drops
        app.tabs[0].split = false;
        app.tabs[0].pane = None;
        assert!(dnd_popup_area_at(&app, x0 + 10, y));
        assert!(dnd_popup_area_at(&app, sx + 10, y));
    }

    #[test]
    fn removable_media_get_their_own_section() {
        let cfg = Config::default();
        let mut app = App::new(cfg);
        app.width = 800;
        app.height = 500;
        app.sidebar_visible = true;
        // one internal volume + one removable (mounted) + one removable
        // (unmounted) — removable must sit under a REMOVABLE header, after
        // the VOLUMES rows, and never share the volume row range.
        app.side_n = 1;
        app.side_paths.push(PathBuf::from("/acc/data"));
        app.unmounted_n = 1;
        app.unmounted_paths.push(PathBuf::from("/dev/nvme0n1p5"));
        app.removable_n = 2;
        app.removable_mounted = vec![true, false];
        app.removable_paths = vec![PathBuf::from("/media/usb"), PathBuf::from("/dev/sdb1")];
        let rows = sidebar_rows(&app);
        let kinds: Vec<SideKind> = rows.iter().map(|r| r.kind.clone()).collect();
        let vol_headers: Vec<usize> = kinds
            .iter()
            .enumerate()
            .filter(|(_, k)| matches!(k, SideKind::Header("VOLUMES")))
            .map(|(i, _)| i)
            .collect();
        let rem_header = kinds
            .iter()
            .position(|k| matches!(k, SideKind::Header("REMOVABLE")))
            .expect("REMOVABLE header present");
        // the removable header comes after the volumes header
        assert!(rem_header > vol_headers[0]);
        // removable rows carry ids 400+; volumes 10+
        let ids: Vec<i32> = rows.iter().map(|r| side_kind_id(&r.kind)).collect();
        assert!(ids.contains(&10));
        assert!(ids.contains(&400));
        assert!(ids.contains(&401));
        // everything after the removable header is removable or a gap
        for k in &kinds[rem_header + 1..] {
            match k {
                SideKind::Removable(_) | SideKind::Gap => {}
                other => panic!("non-removable row leaked into removable section: {other:?}"),
            }
        }
    }

    #[test]
    fn side_kind_ids_are_stable_and_ordered() {
        assert_eq!(side_kind_id(&SideKind::Place(6)), 6);
        assert_eq!(side_kind_id(&SideKind::Volume(0)), 10);
        assert_eq!(side_kind_id(&SideKind::Network(0)), 100);
        assert_eq!(side_kind_id(&SideKind::Unmounted(0)), 200);
        assert_eq!(side_kind_id(&SideKind::Bookmark(0)), 300);
        assert_eq!(side_kind_id(&SideKind::Removable(0)), 400);
    }

    /// The per-tab close (X) button is hit-tested separately from the tab
    /// label, so clicking it closes the tab instead of selecting it.
    #[test]
    fn tab_close_button_is_distinct_from_tab() {
        let cfg = Config::default();
        let mut app = App::new(cfg);
        app.width = 800;
        app.height = 500;
        app.text = crate::text::Text::new_family(app.cfg.font_size as f32, None);
        for (i, dir) in ["/tmp", "/usr"].iter().enumerate() {
            app.tabs.push(crate::tab::Tab::new(i as i32, PathBuf::from(dir)));
        }
        app.cur_tab = 0;
        let (x0, w0) = tab_rect(&app, 0).unwrap();
        let (cx, cy, s, sh) = tab_close_rect(&app, 0).unwrap();
        // the close button is inside the first tab, at its right edge
        assert!(cx >= x0 && cx + s <= x0 + w0);
        // clicking the button hits tab 0's close; the tab label does not
        assert_eq!(tab_close_at(&app, cx + 2, cy + 2), Some(0));
        assert_eq!(tab_at(&app, cx + 2, cy + 2), Some(0));
        assert_eq!(tab_close_at(&app, x0 + app.cfg.padding, cy + 2), None);
        // outside the tab bar → no close hit
        assert_eq!(tab_close_at(&app, cx + 2, 400), None);
        assert_eq!(tab_close_rect(&app, 99), None);
        let _ = sh;
    }

    /// Toolbar geometry is data-driven: hiding view buttons shrinks the right
    /// cluster, hidden nav buttons are not hit-testable, and every visible
    /// item still gets a sane rect.
    #[test]
    fn toolbar_geometry_follows_hidden_items() {
        let cfg = Config::default();
        let mut app = App::new(cfg);
        app.width = 800;
        app.height = 500;
        app.text = crate::text::Text::new_family(app.cfg.font_size as f32, None);
        let default_right = right_btn_x0(&app);
        app.cfg.toolbar_hidden = vec![
            "split".into(),
            "search".into(),
            "list".into(),
            "grid".into(),
            "compact".into(),
        ];
        let smaller_right = right_btn_x0(&app);
        assert!(smaller_right > default_right, "hiding view buttons must shrink the right cluster");
        // hiding the Back button removes it from hit-testing (Forward slides
        // into the first slot, so the hit returns Forward = 1)
        app.cfg.toolbar_hidden = vec!["back".into()];
        assert_eq!(nav_at(&app, app.cfg.padding + 2, path_bar_y(&app) + 5), 1);
        // but every remaining visible item still maps to a real rect
        for (i, _) in toolbar_visible(&app) {
            let Some((rx, ry, rw, rh)) = toolbar_item_rect(&app, i) else {
                panic!("visible item {i} has no rect");
            };
            assert!(rx >= 0 && rw > 0 && ry >= 0 && rh > 0);
        }
        // the Location item always spans the tray between the clusters
        app.cfg.toolbar_hidden.clear();
        let loc = app
            .cfg
            .toolbar
            .iter()
            .position(|it| *it == crate::toolbar::ToolbarItem::Location)
            .unwrap();
        let (lx, _, lw, _) = toolbar_item_rect(&app, loc).unwrap();
        assert_eq!(lx, toolbar_left_end(&app));
        assert_eq!(lx + lw, toolbar_right_start(&app));
    }
}
