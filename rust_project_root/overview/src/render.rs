//! CPU rendering of the overview tape (tiny-skia + cosmic-text).
//!
//! Layout: a vertical tape of workspace *rows*; each row lays its windows out
//! as horizontal tiles. Scroll pans the tape. Rows are niri-style "columns"
//! turned on their side so workspace 1, 2, 3 stack top-to-bottom.

use crate::hypr::Snapshot;
use cosmic_text::{Attrs, Buffer, Color, Family, FontSystem, Metrics, Shaping, SwashCache};
use tiny_skia::{
    Color as SkColor, FillRule, Paint, PathBuilder, Pixmap, Stroke, Transform,
};

pub struct Theme {
    pub backdrop: u32,
    pub row_bg: u32,
    pub row_bg_active: u32,
    pub row_border: u32,
    pub row_border_active: u32,
    pub label: u32,
    pub label_active: u32,
    pub tile_bg: u32,
    pub tile_bg_active: u32,
    pub tile_border: u32,
    pub tile_border_active: u32,
    pub title: u32,
    pub class: u32,
}

impl Default for Theme {
    fn default() -> Self {
        Theme {
            backdrop: 0x14141a, // transparent dark
            row_bg: 0x232333,
            row_bg_active: 0x2d2d42,
            row_border: 0x3a3a52,
            row_border_active: 0x7aa2f7,
            label: 0x9aa0b4,
            label_active: 0xc0caf5,
            tile_bg: 0x1a1b2b,
            tile_bg_active: 0x232442,
            tile_border: 0x2f3050,
            tile_border_active: 0x7aa2f7,
            title: 0xe0e2ee,
            class: 0x82889e,
        }
    }
}

pub struct Layout {
    pub view_h: f32,
    pub row_h: f32,
    pub gap_x: f32,
    pub gap_y: f32,
    pub tile_w: f32,
    pub tile_h: f32,
    pub pad_x: f32,
    pub rows: Vec<RowLayout>,
    pub content_h: f32,
}

pub struct RowLayout {
    pub y: f32,
    pub ws_id: i64,
    pub tiles: Vec<TileLayout>,
}

pub struct TileLayout {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub focused: bool,
}

pub struct Renderer {
    pub font_system: FontSystem,
    swash: SwashCache,
    family: String,
}

fn load_fonts(db: &mut cosmic_text::fontdb::Database) {
    let mut loaded = 0;
    for dir in ["/usr/share/fonts", "/usr/local/share/fonts", "/run/host/fonts"] {
        let Ok(rd) = std::fs::read_dir(dir) else {
            continue;
        };
        for entry in rd.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let Ok(sub) = std::fs::read_dir(&path) else {
                    continue;
                };
                for e in sub.flatten() {
                    let p = e.path();
                    let ext = p.extension().and_then(|x| x.to_str()).unwrap_or("");
                    if (ext == "ttf" || ext == "otf" || ext == "ttc")
                        && db.load_font_file(&p).is_ok()
                    {
                        loaded += 1;
                    }
                }
            } else {
                let p = path;
                let ext = p.extension().and_then(|x| x.to_str()).unwrap_or("");
                if (ext == "ttf" || ext == "otf" || ext == "ttc")
                    && db.load_font_file(&p).is_ok()
                {
                    loaded += 1;
                }
            }
        }
    }
    if loaded == 0 {
        db.load_system_fonts();
    }
}

impl Renderer {
    pub fn new(family: String) -> Self {
        let mut db = cosmic_text::fontdb::Database::new();
        load_fonts(&mut db);
        let font_system = FontSystem::new_with_locale_and_db("en_US.UTF-8".into(), db);
        Renderer {
            font_system,
            swash: SwashCache::new(),
            family,
        }
    }

    fn attrs<'a>(&'a self) -> Attrs<'a> {
        Attrs::new().family(Family::Name(&self.family))
    }

    fn line_h(size: f32) -> f32 {
        (size * 1.3).ceil()
    }

    /// Compute row/tile geometry for a snapshot within a viewport.
    pub fn layout(&self, snap: &Snapshot, view_w: f32, view_h: f32) -> Layout {
        let pad_x = 90.0;
        let pad_top = 48.0;
        let gap_y = 18.0;
        let gap_x = 14.0;
        let tile_h: f32 = 96.0;
        // one row per workspace; tile width scales to fit but caps at 220
        let max_tile_w: f32 = 220.0;
        let avail = (view_w - pad_x * 2.0).max(200.0);
        let tile_w = avail.min(360.0).max(150.0).min(max_tile_w.max(150.0));
        // row height = tile height + workspace label above
        let row_h = 30.0 + tile_h;

        let mut rows = Vec::new();
        let mut y = pad_top;
        for ws in &snap.workspaces {
            let mut row = RowLayout {
                y,
                ws_id: ws.id,
                tiles: Vec::new(),
            };
            let mut x = pad_x;
            if ws.wins.is_empty() {
                // still draw the row frame, one placeholder tile
                row.tiles.push(TileLayout {
                    x,
                    y: y + 30.0,
                    w: tile_w,
                    h: tile_h,
                    focused: false,
                });
            } else {
                for win in &ws.wins {
                    row.tiles.push(TileLayout {
                        x,
                        y: y + 30.0,
                        w: tile_w,
                        h: tile_h,
                        focused: false,
                    });
                    x += tile_w + gap_x;
                }
            }
            rows.push(row);
            y += row_h + gap_y;
        }

        Layout {
            view_h,
            row_h,
            gap_x,
            gap_y,
            tile_w,
            tile_h,
            pad_x,
            rows,
            content_h: y - gap_y,
        }
    }

    /// Draw whole tape onto a pixmap at the given scroll offset.
    pub fn draw(&mut self, px: &mut Pixmap, snap: &Snapshot, th: &Theme, scroll: f32) {
        let (vw, vh) = (px.width() as f32, px.height() as f32);
        // backdrop
        px.fill(SkColor::from_rgba8(
            (th.backdrop >> 16 & 0xff) as u8,
            (th.backdrop >> 8 & 0xff) as u8,
            (th.backdrop & 0xff) as u8,
            255,
        ));

        let lay = self.layout(snap, vw, vh);
        // center content vertically if it fits, else scroll from top
        let max_scroll = (lay.content_h - vh + 60.0).max(0.0);
        let scroll = scroll.clamp(0.0, max_scroll);
        let offset = if lay.content_h <= vh {
            (vh - lay.content_h) / 2.0
        } else {
            0.0
        };

        for row in &lay.rows {
            let ry = row.y + offset - scroll;
            if ry > vh || ry + lay.row_h < 0.0 {
                continue;
            }
            let active = row_ws_active(snap, row.ws_id);
            let row_w = content_row_width(&lay, row);
            let row_bg = if active { th.row_bg_active } else { th.row_bg };
            let row_border = if active {
                th.row_border_active
            } else {
                th.row_border
            };
            draw_rounded_rect(
                px,
                lay.pad_x - 12.0,
                ry - 4.0,
                row_w + 24.0,
                lay.row_h + 8.0,
                12.0,
                row_bg,
            );
            draw_rounded_rect_border(
                px,
                lay.pad_x - 12.0,
                ry - 4.0,
                row_w + 24.0,
                lay.row_h + 8.0,
                12.0,
                row_border,
                1.5,
            );

            // workspace label
            let label = format!("workspace {}", row.ws_id);
            let lc = if active { th.label_active } else { th.label };
            self.draw_text(px, &label, lay.pad_x, ry, 13.0, lc);

            let has_wins = row_ws(snap, row.ws_id)
                .map(|w| !w.wins.is_empty())
                .unwrap_or(false);
            let mut win_idx = 0usize;
            for tile in &row.tiles {
                let tx = tile.x;
                let ty = tile.y + offset - scroll;
                let bg = if active { th.tile_bg_active } else { th.tile_bg };
                let border = if active {
                    th.tile_border_active
                } else {
                    th.tile_border
                };
                draw_rounded_rect(px, tx, ty, tile.w, tile.h, 8.0, bg);
                draw_rounded_rect_border(px, tx, ty, tile.w, tile.h, 8.0, border, 1.0);
                if has_wins {
                    if let Some(win) = window_at(snap, row.ws_id, win_idx) {
                        self.draw_text(
                            px,
                            &win.title,
                            tx + 10.0,
                            ty + 12.0,
                            14.0,
                            th.title,
                        );
                        self.draw_text(
                            px,
                            &win.class,
                            tx + 10.0,
                            ty + tile.h - 22.0,
                            11.0,
                            th.class,
                        );
                    }
                    win_idx += 1;
                }
            }
        }
    }

    fn draw_text(
        &mut self,
        px: &mut Pixmap,
        text: &str,
        x: f32,
        y: f32,
        size: f32,
        color: u32,
    ) {
        let size = if size > 0.0 { size } else { 1.0 };
        let w = px.width() as i32;
        let h = px.height() as i32;
        let mut buf = Buffer::new(
            &mut self.font_system,
            Metrics::new(size, Self::line_h(size)),
        );
        buf.set_size(Some(4000.0), Some(size * 2.0));
        buf.set_text(text, &self.attrs(), Shaping::Advanced, None);
        buf.shape_until_scroll(&mut self.font_system, true);
        // cosmic-text draw with our own swash cache
        let col = Color(((color & 0xff) << 24) | (color >> 8));
        buf.draw(
            &mut self.font_system,
            &mut self.swash,
            col,
            |gx, gy, gw, gh, c| {
                let a = c.a();
                if a == 0 {
                    return;
                }
                let r = c.r() as u32 * a as u32 / 255;
                let g = c.g() as u32 * a as u32 / 255;
                let b = c.b() as u32 * a as u32 / 255;
                let base_x = x as i32 + gx;
                let base_y = y as i32 + gy;
                for dy in 0..gh {
                    for dx in 0..gw {
                        let px0 = base_x + dx as i32;
                        let py0 = base_y + dy as i32;
                        if px0 < 0 || py0 < 0 || px0 >= w || py0 >= h {
                            continue;
                        }
                        let idx = ((py0 * w + px0) * 4) as usize;
                        let data = px.data_mut();
                        let src_a = a as u32;
                        let dst_a = data[idx + 3] as u32;
                        // source over (premultiplied source, straight dest blend)
                        let out_a = src_a + dst_a * (255 - src_a) / 255;
                        let sr = r;
                        let sg = g;
                        let sb = b;
                        let dr = data[idx] as u32;
                        let dg = data[idx + 1] as u32;
                        let db = data[idx + 2] as u32;
                        let out_r = if out_a == 0 {
                            0
                        } else {
                            (sr * src_a + dr * dst_a * (255 - src_a) / 255) / out_a.max(1)
                        };
                        let out_g = if out_a == 0 {
                            0
                        } else {
                            (sg * src_a + dg * dst_a * (255 - src_a) / 255) / out_a.max(1)
                        };
                        let out_b = if out_a == 0 {
                            0
                        } else {
                            (sb * src_a + db * dst_a * (255 - src_a) / 255) / out_a.max(1)
                        };
                        data[idx] = out_r as u8;
                        data[idx + 1] = out_g as u8;
                        data[idx + 2] = out_b as u8;
                        data[idx + 3] = out_a as u8;
                    }
                }
            },
        );
    }
}

// ---- helpers ----

fn row_ws_active(snap: &Snapshot, id: i64) -> bool {
    snap.active_id == id
}

fn row_ws(snap: &Snapshot, id: i64) -> Option<&crate::hypr::Workspace> {
    snap.workspaces.iter().find(|w| w.id == id)
}

fn content_row_width(lay: &Layout, row: &RowLayout) -> f32 {
    row.tiles.len() as f32 * lay.tile_w
        + (row.tiles.len().saturating_sub(1)) as f32 * lay.gap_x
}

fn window_at<'s>(snap: &'s Snapshot, ws_id: i64, idx: usize) -> Option<&'s crate::hypr::Win> {
    row_ws(snap, ws_id).and_then(|w| w.wins.get(idx))
}

fn draw_rounded_rect(px: &mut Pixmap, x: f32, y: f32, w: f32, h: f32, r: f32, color: u32) {
    if w <= 0.0 || h <= 0.0 {
        return;
    }
    let col = SkColor::from_rgba8(
        (color >> 16 & 0xff) as u8,
        (color >> 8 & 0xff) as u8,
        (color & 0xff) as u8,
        255,
    );
    let path = rounded_path(x, y, w, h, r);
    let mut paint = Paint::default();
    paint.set_color(col);
    paint.anti_alias = true;
    px.fill_path(&path, &paint, FillRule::default(), Transform::identity(), None);
}

fn draw_rounded_rect_border(
    px: &mut Pixmap,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    r: f32,
    color: u32,
    stroke_width: f32,
) {
    if w <= 0.0 || h <= 0.0 {
        return;
    }
    let col = SkColor::from_rgba8(
        (color >> 16 & 0xff) as u8,
        (color >> 8 & 0xff) as u8,
        (color & 0xff) as u8,
        255,
    );
    let path = rounded_path(x, y, w, h, r);
    let mut paint = Paint::default();
    paint.set_color(col);
    paint.anti_alias = true;
    let stroke = Stroke {
        width: stroke_width,
        ..Stroke::default()
    };
    px.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
}

fn rounded_path(x: f32, y: f32, w: f32, h: f32, r: f32) -> tiny_skia::Path {
    let r = r.min(w / 2.0).min(h / 2.0);
    let mut pb = PathBuilder::new();
    pb.move_to(x + r, y);
    pb.line_to(x + w - r, y);
    pb.quad_to(x + w, y, x + w, y + r);
    pb.line_to(x + w, y + h - r);
    pb.quad_to(x + w, y + h, x + w - r, y + h);
    pb.line_to(x + r, y + h);
    pb.quad_to(x, y + h, x, y + h - r);
    pb.line_to(x, y + r);
    pb.quad_to(x, y, x + r, y);
    pb.close();
    pb.finish().unwrap()
}
