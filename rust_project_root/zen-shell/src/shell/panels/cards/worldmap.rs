use super::super::*;

/// Map-app-in-a-card: full world map, zoomable around the pinned marker,
/// with a swipe-open menu (style picker + "save downloads" toggle) and
/// per-country chunks downloaded on demand from the data repo (mapdata).
use crate::shell::worldmap::{self, Win};

/// Zoom at which fine region chunks (50m borders) load + city labels appear.
const ZOOM_REGION: f32 = 4.0;
/// Zoom at which the city-label layer turns on.
const ZOOM_CITIES: f32 = 2.0;
/// The style names shown in the menu.
const MENU_STYLES: [&str; 4] = ["Real", "Filled", "Dots", "Lines"];

impl Shell {

    pub(crate) fn draw_worldmap_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        // the card ALWAYS has a map to draw: without any downloaded chunks the
        // embedded 110m world is installed (marked as fallback).
        worldmap::ensure_fallback();
        self.worldmap_rect = (x, y, w, h);
        let pad = self.scale.s(12.0);
        // header: globe glyph + title + current style badge (+ zoom hint)
if self.card_show_glyph { ui::text(v, x + pad, y + self.scale.s(8.0), ICON_GLOBE, self.scale.fs(12.0), pal.acc, true); }
if !self.scene_owns_header && self.card_show_title { ui::title(v, if self.card_show_glyph { x + pad + self.scale.s(18.0) } else { x + pad }, y + self.scale.s(9.0), "World", self.scale.fs(12.0), pal.fg); }
        if !self.world_menu {
            let style = MENU_STYLES[self.world_pane.min(WORLD_PANES - 1)];
            let mut hint = style.to_string();
            if self.world_zoom > 1.0 {
                hint.push_str("  ·  ×");
                hint.push_str(&format!("{:.1}", self.world_zoom));
            }
            ui::text_r(v, x + w - pad, y + self.scale.s(9.0), hint, self.scale.fs(9.5), ui::fg3(pal), false);
        }
        let hover = !self.dash_edit
            && self
                .cursor
                .map(|(px, py)| Self::in_rect(px, py, (x, y, w, h)))
                .unwrap_or(false);

        // body: at zoom 1 the whole world shows at its natural 2:1 aspect
        // (letterboxed in the body rect). Zoomed views EXPAND to the full
        // body rect and the view window stretches to match the body's aspect,
        // so the map covers the card top-to-bottom with real content — the
        // projection stays uniform (equal degrees/pixel), never a squeezed
        // version of a 2:1 window.
        let has_marker = self.world_marker.is_some();
        let reserve = if has_marker { self.scale.s(46.0) } else { self.scale.s(30.0) };
        let body_y = y + self.scale.s(26.0);
        let body_h = (h - self.scale.s(26.0) - reserve).max(self.scale.s(24.0));
        let (bx, by, bw, bh) = (x + pad, body_y, (w - pad * 2.0).max(1.0), body_h);
        self.worldmap_body_rect = if self.world_zoom <= 1.0 {
            let s = (bw / 360.0).min(bh / 170.0).max(1.0);
            (bx + (bw - 360.0 * s) / 2.0, by + (bh - 170.0 * s) / 2.0, 360.0 * s, 170.0 * s)
        } else {
            (bx, by, bw, bh)
        };
        let (mx, my, mw, mh) = self.worldmap_body_rect;

        if worldmap::ready() {
            self.draw_world_body(v, pal, mx, my, mw, mh, hover);
            self.draw_world_zoom_buttons(v, pal, mx, my, mw, mh);
            self.draw_world_region_pill(v, pal, mx, my, mw, mh);
            // nothing downloaded yet → the embedded fallback is on screen;
            // offer a compact overlay pill to fetch the full dataset.
            if worldmap::using_fallback() {
                self.draw_world_dl_pill(v, pal, mx, my, mw, mh);
            }
        } else {
            self.draw_world_placeholder(v, pal, mx, my, mw, mh);
        }

        // bottom band: local time for the pinned marker (outside the menu)
        if self.world_menu {
            self.draw_world_menu(v, pal, x, y, w, h);
        } else {
            let ccx = x + w / 2.0;
            if has_marker {
                let now = unsafe { libc::time(std::ptr::null_mut()) };
                let hhmm = worldmap::world_hhmm(now, self.world_offset_min);
                let mut line = String::new();
                if !self.world_city.is_empty() {
                    line.push_str(&self.world_city);
                    line.push_str("  ·  ");
                }
                line.push_str(&hhmm);
                if !self.world_abbrev.is_empty() {
                    line.push(' ');
                    line.push_str(&self.world_abbrev);
                }
                ui::text_c(v, ccx, y + h - self.scale.s(30.0), line, self.scale.fs(10.5), pal.fg, false);
                ui::text_c(
                    v,
                    ccx,
                    y + h - self.scale.s(17.0),
                    worldmap::world_dayline(now, self.world_offset_min),
                    self.scale.fs(9.5),
                    ui::fg3(pal),
                    false,
                );
            } else {
                ui::text_c(v, ccx, y + h - self.scale.s(24.0), "click a spot for its local time", self.scale.fs(9.5), ui::fg3(pal), false);
            }
        }
    }

    /// True when the pointer rests on the live map body — the pinch-zoom
    /// engagement test. Content coords are compared (cursor minus the spring
    /// offset), mirroring how hover regions are hit-tested.
    pub(crate) fn over_worldmap(&self) -> bool {
        if self.mode != crate::shell::Mode::Expanded || self.world_menu {
            return false;
        }
        let (mx, my, mw, mh) = self.worldmap_body_rect;
        if mw <= 1.0 || mh <= 1.0 {
            return false;
        }
        let (cx, cy) = match self.cursor {
            Some(c) => c,
            None => return false,
        };
        let x = cx - self.content_dx;
        let y = cy - self.content_dy;
        x >= mx && x < mx + mw && y >= my && y < my + mh
    }

    /// The world point under the cursor plus its fractional position in the
    /// body window — the zoom-to-point anchor (lon, lat, tx, ty).
    pub(crate) fn world_cursor_anchor(&self) -> Option<(f32, f32, f32, f32)> {
        if !self.over_worldmap() {
            return None;
        }
        let (mx, my, mw, mh) = self.worldmap_body_rect;
        if mw <= 1.0 || mh <= 1.0 {
            return None;
        }
        let (cx, cy) = match self.cursor {
            Some(c) => c,
            None => return None,
        };
        let sx = cx as f32 - self.content_dx;
        let sy = cy as f32 - self.content_dy;
        let tx = ((sx - mx) / mw).clamp(0.0, 1.0);
        let ty = ((sy - my) / mh).clamp(0.0, 1.0);
        let win = self.world_win();
        let (alon, alat) = worldmap::unproj(mx, my, mw, mh, &win, sx, sy);
        Some((alon, alat, tx, ty))
    }

    /// Zoom the map, keeping the world point under the cursor fixed on screen
    /// (when the cursor is over the body); otherwise zoom around the marker /
    /// current center. Button and pinch zoom both route here.
    pub(crate) fn world_zoom_to(&mut self, zoom: f32) {
        let anchor = self.world_cursor_anchor();
        self.world_apply_zoom(zoom, anchor);
    }

    /// Pinch variant: applies a previously captured anchor. Re-deriving it on
    /// every frame would drift, because the center moves as we zoom.
    pub(crate) fn world_pinch_zoom(&mut self, zoom: f32, anchor: (f32, f32, f32, f32)) {
        self.world_apply_zoom(zoom, Some(anchor));
    }

    fn world_apply_zoom(&mut self, zoom: f32, anchor: Option<(f32, f32, f32, f32)>) {
        let z1 = zoom.clamp(1.0, 32.0);
        if let Some((alon, alat, tx, ty)) = anchor {
            // recentre the window at z1 so the anchor world point lands on its
            // old screen fraction (tx, ty) instead of drifting toward center
            let span_lon = 360.0 / z1;
            let span_lat = 170.0 / z1;
            self.world_center.0 = alon + (0.5 - tx) * span_lon;
            self.world_center.1 = alat + (ty - 0.5) * span_lat;
        }
        self.world_zoom = z1;
    }

    /// Scroll PANS the map while the cursor rests on the body: vertical delta
    /// moves latitude (positive = north), horizontal moves longitude (positive
    /// = east), a fraction of the visible window per unit. Returns false when
    /// nothing should happen (cursor not over the body).
    pub(crate) fn world_scroll(&mut self, v: f32, h: f32) -> bool {
        if !self.over_worldmap() {
            return false;
        }
        let win = self.world_win();
        let k = 0.12; // ~12% of the visible span per notch
        self.world_center.0 += h * win.lon_span() * k;
        self.world_center.1 += v * win.lat_span() * k;
        true
    }

    /// The current view window. At zoom 1 it's the whole world (2:1, matched
    /// to the letterboxed body). Zoomed in, it tracks the BODY's aspect so the
    /// map fills the full card top-to-bottom with uniform degrees/pixel —
    /// the projection is never squeezed to fit.
    pub(crate) fn world_win(&self) -> Win {
        if self.world_zoom <= 1.0 {
            return worldmap::win_for(self.world_center.0, self.world_center.1, 1.0);
        }
        let (_, _, mw, mh) = self.worldmap_body_rect;
        let lon_span = (360.0 / self.world_zoom).min(360.0);
        let lat_span = (lon_span * (mh / mw.max(1.0))).min(170.0).max(0.05);
        let (clon, clat) = self.world_center;
        let mut lon0 = (clon - lon_span * 0.5).max(-180.0);
        let lon1 = (lon0 + lon_span).min(180.0);
        lon0 = (lon1 - lon_span).max(-180.0);
        let mut lat0 = (clat - lat_span * 0.5).max(-85.0);
        let lat1 = (lat0 + lat_span).min(85.0);
        lat0 = (lat1 - lat_span).max(-85.0);
        Win { lon0, lat0, lon1, lat1 }
    }

    /// Press on the map body: grab it like a gallery photo. Returns false when
    /// the pointer isn't over the body (menu open, collapsed, no cursor) so the
    /// press can fall through to normal click handling.
    pub(crate) fn begin_world_drag(&mut self) -> bool {
        if !self.over_worldmap() {
            return false;
        }
        let (x, y) = match self.cursor {
            Some(c) => c,
            None => return false,
        };
        self.world_drag = Some((
            x - self.content_dx,
            y - self.content_dy,
            self.world_center.0,
            self.world_center.1,
        ));
        self.world_drag_moved = false;
        true
    }

    /// Live pan from the stored press anchor. Content-follows-fingers: drag
    /// right → see west (lon −), drag down → see north (lat +). Crossed the
    /// ~3px gallery threshold → remember so release skips the click.
    pub(crate) fn world_drag_to(&mut self) -> bool {
        let Some((sx0, sy0, clon, clat)) = self.world_drag else {
            return false;
        };
        let (x, y) = match self.cursor {
            Some(c) => c,
            None => return false,
        };
        let dx = (x - self.content_dx) - sx0;
        let dy = (y - self.content_dy) - sy0;
        if dx.abs() > 3.0 || dy.abs() > 3.0 {
            self.world_drag_moved = true;
        }
        let (_, _, mw, mh) = self.worldmap_body_rect;
        if mw <= 1.0 || mh <= 1.0 {
            return false;
        }
        let win = self.world_win();
        self.world_center.0 = clon - dx * win.lon_span() / mw;
        self.world_center.1 = clat + dy * win.lat_span() / mh;
        true
    }

    pub(crate) fn world_drag_active(&self) -> bool {
        self.world_drag.is_some()
    }

    /// Release: returns true when the gesture actually dragged (suppresses the
    /// place-click). Always clears the grab state.
    pub(crate) fn end_world_drag(&mut self) -> bool {
        let moved = self.world_drag_moved;
        self.world_drag = None;
        self.world_drag_moved = false;
        moved
    }

    /// Release-without-move over the body: clip the (lat, lon), pin the marker
    /// and schedule the nearest-zone probe (the old WORLD_MAP_KEY click).
    pub(crate) fn world_click_at(&mut self) {
        let (mx, my, mw, mh) = self.worldmap_body_rect;
        if mw <= 1.0 || mh <= 1.0 {
            return;
        }
        let (x, y) = match self.cursor {
            Some(c) => c,
            None => return,
        };
        if !Self::in_rect(x, y, (mx, my, mw, mh)) {
            return;
        }
        let win = self.world_win();
        let (lon, lat) = worldmap::unproj(mx, my, mw, mh, &win, x, y);
        self.world_marker = Some((lat, lon));
        if self.world_zoom > 1.0 {
            // zoom is anchored to the pinned marker
            self.world_center = (lon, lat);
        }
        if let Some(z) = worldmap::nearest_zone(lat, lon).cloned() {
            self.world_city = z.city;
            self.world_tz = z.tz.clone();
            self.world_abbrev.clear();
            self.pending_world_offset = Some(z.tz);
        }
    }

    /// The map body: ONE band `Cmd::Map` quad. The App bakes the window to a
    /// texture once per zoom band; pan/zoom/pinch just stretch its UV window
    /// (4 floats) — the per-frame coast-line probe + rect tracing is gone.
    /// Only the city labels, the pinned marker and the hover crosshair stay
    /// as live vector overlays on top of the bake.
    fn draw_world_body(&mut self, v: &mut Vec<Cmd>, pal: &Pal, mx: f32, my: f32, mw: f32, mh: f32, hover: bool) {
        if mw > 2.0 && mh > 2.0 {
            self.region(mx, my, mw, mh, crate::shell::WORLD_MAP_KEY);
        }
        if worldmap::world().is_none() {
            return;
        }
        let win = self.world_win();
        let style = self.world_pane.min(WORLD_PANES - 1);
        // style → bake palette [raised, land, rings, hairline, dots, region]
        let colors = match style {
            0 => [
                ui::raised(pal),
                ui::mix(pal.bg, pal.acc, 0.10),
                ui::mix(pal.fg, pal.bg, 0.58),
                ui::hairline(pal),
                ui::mix(pal.bg, pal.fg, 0.30),
                ui::mix(pal.bg, pal.acc, 0.40),
            ],
            1 => [0x00000000, ui::acc_tint(pal), 0, 0, 0, 0],
            2 => [0x00000000, 0, 0, 0, ui::mix(pal.bg, pal.fg, 0.30), 0],
            3 => [
                ui::raised(pal),
                0,
                ui::mix(pal.fg, pal.bg, 0.55),
                ui::hairline(pal),
                0,
                ui::mix(pal.bg, pal.acc, 0.40),
            ],
            _ => [0, 0, 0, 0, 0, 0],
        };
        let region_on = self.world_region.is_some() && self.world_zoom >= ZOOM_REGION;
        v.push(Cmd::Map {
            x: mx,
            y: my,
            w: mw,
            h: mh,
            lon0: win.lon0,
            lat0: win.lat0,
            lon1: win.lon1,
            lat1: win.lat1,
            zoom: self.world_zoom,
            style: style as u8,
            colors,
            region: region_on,
            version: self.world_rev,
        });
        // city labels stay live vectors, clamped inside the body
        if self.world_zoom >= ZOOM_CITIES {
            if let Some(cities) = worldmap::cities() {
                for c in cities.iter() {
                    if !(win.lon0 - 1.0..=win.lon1 + 1.0).contains(&c.lon)
                        || !(win.lat0 - 1.0..=win.lat1 + 1.0).contains(&c.lat)
                    {
                        continue;
                    }
                    let (cx, cy) = worldmap::proj(mx, my, mw, mh, &win, c.lon.min(win.lon1).max(win.lon0), c.lat);
                    let tx = (cx - self.scale.s(3.0)).clamp(mx, mx + mw - self.scale.s(6.0));
                    let ty = (cy + self.scale.s(2.0)).clamp(my, my + mh - self.scale.fs(8.0));
                    ui::text(v, tx, ty, &c.name, self.scale.fs(8.0), ui::fg2(pal), false);
                }
            }
        }
        // pinned marker (accent dot with a halo); only when inside the window
        if let Some((lat, lon)) = self.world_marker {
            let (px, py) = worldmap::proj(mx, my, mw, mh, &win, lon, lat);
            if win.contains(lon, lat) && Self::in_rect(px, py, (mx - 2.0, my - 2.0, mw + 4.0, mh + 4.0)) {
                v.push(Cmd::Rect { x: px - self.scale.s(4.0), y: py - self.scale.s(4.0), w: self.scale.s(8.0), h: self.scale.s(8.0), r: self.scale.s(4.0), color: ui::mix(pal.fg, pal.acc, 0.40) });
                v.push(Cmd::Rect { x: px - self.scale.s(2.0), y: py - self.scale.s(2.0), w: self.scale.s(4.0), h: self.scale.s(4.0), r: self.scale.s(2.0), color: pal.acc });
            }
        }
        // hover crosshair feedback over the map
        if hover
            && !self.world_menu
            && self
                .cursor
                .map(|(px, py)| Self::in_rect(px, py, (mx, my, mw, mh)))
                .unwrap_or(false)
        {
            v.push(Cmd::Outline { x: mx, y: my, w: mw, h: mh, r: 4.0, width: 1.0, color: ui::hover_hl(pal) });
        }
    }

    /// "downloaded" confirmation: the bottom band + a green-ish dot
    fn draw_world_placeholder(&mut self, v: &mut Vec<Cmd>, pal: &Pal, mx: f32, my: f32, mw: f32, mh: f32) {
        let cx = mx + mw / 2.0;
        let cy = my + mh / 2.0;
        if mw < 40.0 || mh < 20.0 {
            return;
        }
        let pending = matches!(&self.world_dl_pending, Some(crate::shell::WorldDl::Base));
        let pw = self.scale.s(168.0);
        let ph = self.scale.s(19.0);
        let px = cx - pw / 2.0;
        let py = cy - self.scale.s(22.0);
        let (bg, label) = if pending {
            (ui::raised(pal), "Downloading…".to_string())
        } else {
            (self.hl(crate::shell::WORLD_DL_KEY, ui::sel_bg(pal), ui::raised_hl(pal)), "Download world map".to_string())
        };
        v.push(Cmd::Rect { x: px, y: py, w: pw, h: ph, r: ph / 2.0, color: bg });
        ui::text_c(v, cx, py + ph - self.scale.s(5.0) - self.scale.s(1.0), &label, self.scale.fs(9.5), if pending { ui::fg3(pal) } else { pal.fg }, false);
        if !pending {
            self.region(px, py, pw, ph, crate::shell::WORLD_DL_KEY);
        }
        if let Some(err) = &self.world_dl_err {
            ui::text_c(v, cx, cy + self.scale.s(2.0) + self.scale.s(14.0), err, self.scale.fs(8.5), ui::mix(pal.bg, pal.acc, 0.55), false);
        }
    }

    /// + / − zoom buttons, bottom-right of the map body.
    fn draw_world_zoom_buttons(&mut self, v: &mut Vec<Cmd>, pal: &Pal, mx: f32, my: f32, mw: f32, mh: f32) {
        let bsz = self.scale.s(17.0);
        let gap = self.scale.s(5.0);
        let rx = mx + mw - bsz * 1.55;
        let ry = my + mh - bsz - self.scale.s(5.0);
        // [−] then [+] (rightmost = in)
        for (i, (glyph, key)) in [("–", crate::shell::WORLD_ZOOM_OUT_KEY), ("+", crate::shell::WORLD_ZOOM_IN_KEY)].iter().enumerate() {
            let bx = rx - i as f32 * (bsz + gap);
            let by = ry;
            let c = self.hl(*key, ui::raised_hl(pal), ui::hover_hl(pal));
            v.push(Cmd::Rect { x: bx, y: by, w: bsz, h: bsz, r: bsz / 2.0, color: c });
            ui::text_c(v, bx + bsz / 2.0, by + bsz / 2.0 - self.scale.fs(11.0) / 2.0 + self.scale.s(1.0), *glyph, self.scale.fs(11.0), pal.fg, false);
            self.region(bx, by, bsz, bsz, *key);
        }
    }

    /// On-demand region download pill: appears at high zoom when the active
    /// country chunk is missing. Click → downloads just that chunk.
    fn draw_world_region_pill(&mut self, v: &mut Vec<Cmd>, pal: &Pal, mx: f32, my: f32, mw: f32, _mh: f32) {
        if self.world_zoom < ZOOM_REGION {
            return;
        }
        let Some(man) = &self.world_manifest else { return };
        let (clon, clat) = self.world_marker.map(|(la, lo)| (lo, la)).unwrap_or(self.world_center);
        let Some((iso, reg)) = crate::shell::mapdata::region_at(man, clon, clat) else { return };
        let save = self.world_save_data;
        let meta = crate::shell::mapdata::Meta {
            url: reg.url.clone(),
            sha256: reg.sha256.clone(),
            size: reg.size,
        };
        let cached: bool = crate::shell::mapdata::file_valid(&crate::shell::mapdata::region_path(save, &iso), &meta);
        let loaded = self.world_region.as_deref() == Some(iso.as_str());
        if cached && loaded {
            return; // already have it on screen
        }
        let pending = matches!(&self.world_dl_pending, Some(crate::shell::WorldDl::Region(s)) if s == &iso);
        let pw = self.scale.s(172.0);
        let ph = self.scale.s(18.0);
        let px = mx + mw / 2.0 - pw / 2.0;
        let py = my + self.scale.s(6.0);
        self.world_region_target = Some(iso.clone());
        if pending {
            v.push(Cmd::Rect { x: px, y: py, w: pw, h: ph, r: ph / 2.0, color: ui::raised(pal) });
            ui::text_c(v, px + pw / 2.0, py + ph - self.scale.s(4.5) - self.scale.s(1.0), &format!("Downloading {}", reg.name), self.scale.fs(9.5), ui::fg3(pal), false);
            return;
        }
        if cached && !loaded {
            // services timer will pick it up; show a small "loading" hint
            v.push(Cmd::Rect { x: px, y: py, w: pw, h: ph, r: ph / 2.0, color: ui::raised(pal) });
            ui::text_c(v, px + pw / 2.0, py + ph - self.scale.s(5.0) - self.scale.s(1.0), &format!("Loading {}", reg.name), self.scale.fs(9.5), ui::fg3(pal), false);
            return;
        }
        let c = self.hl(crate::shell::WORLD_REGION_DL_KEY, ui::sel_bg(pal), ui::raised_hl(pal));
        v.push(Cmd::Rect { x: px, y: py, w: pw, h: ph, r: ph / 2.0, color: c });
        ui::text_c(v, px + pw / 2.0, py + ph - self.scale.s(5.0) - self.scale.s(1.0), &format!("Download {} map", reg.name), self.scale.fs(9.5), pal.fg, false);
        self.region(px, py, pw, ph, crate::shell::WORLD_REGION_DL_KEY);
    }

    /// Compact overlay pill shown over the embedded fallback map: fetch the
    /// full world + city dataset (replaces the fallback in place). Pushed
    /// after the body so it wins the hover/click hit-test.
    fn draw_world_dl_pill(&mut self, v: &mut Vec<Cmd>, pal: &Pal, mx: f32, my: f32, mw: f32, _mh: f32) {
        let cx = mx + mw / 2.0;
        let pending = matches!(&self.world_dl_pending, Some(crate::shell::WorldDl::Base));
        let pw = self.scale.s(118.0);
        let ph = self.scale.s(16.0);
        let px = cx - pw / 2.0;
        let py = my + self.scale.s(6.0);
        let (bg, label) = if pending {
            (ui::raised(pal), "Downloading world…".to_string())
        } else {
            (self.hl(crate::shell::WORLD_DL_KEY, ui::sel_bg(pal), ui::raised_hl(pal)), "Download full map".to_string())
        };
        v.push(Cmd::Rect { x: px, y: py, w: pw, h: ph, r: ph / 2.0, color: bg });
        ui::text_c(v, cx, py + ph - self.scale.s(4.5) - self.scale.s(1.0), &label, self.scale.fs(8.5), if pending { ui::fg3(pal) } else { pal.fg }, false);
        if !pending {
            self.region(px, py, pw, ph, crate::shell::WORLD_DL_KEY);
        }
        if let Some(err) = &self.world_dl_err {
            ui::text_c(v, cx, py + ph + self.scale.s(3.0) + self.scale.s(11.0), err, self.scale.fs(8.0), ui::mix(pal.bg, pal.acc, 0.55), false);
        }
    }
    /// Swipe-open menu: style picker + save-downloads toggle. Pushed LAST so
    /// its regions win over the map body (clicks land on the menu).
    fn draw_world_menu(&mut self, v: &mut Vec<Cmd>, pal: &Pal, x: f32, y: f32, w: f32, h: f32) {
        // dim the card; the whole-card backdrop closes the menu
        v.push(Cmd::Rect { x, y, w, h, r: 6.0, color: 0x8a000000 });
        self.region(x, y, w, h, crate::shell::WORLD_MENU_KEY);
        // panel
        let pw = self.scale.s(172.0);
        let row_h = self.scale.s(29.0);
        let row_gap = self.scale.s(4.0);
        let title_h = self.scale.s(22.0);
        let pad_y = self.scale.s(10.0);
        let rows = WORLD_PANES + 1; // 4 styles + toggle
        let ph = pad_y * 2.0 + title_h + rows as f32 * (row_h + row_gap);
        let px = x + (w - pw) / 2.0;
        let py = y + (h - ph) / 2.0;
        v.push(Cmd::Rect { x: px, y: py, w: pw, h: ph, r: 8.0, color: ui::raised(pal) });
        v.push(Cmd::Outline { x: px, y: py, w: pw, h: ph, r: 8.0, width: 1.0, color: ui::hairline(pal) });
        ui::text(v, px + self.scale.s(12.0), py + self.scale.s(14.0), "Map style", self.scale.fs(10.5), ui::fg3(pal), false);
        let mut ry = py + pad_y + title_h;
        for (i, name) in MENU_STYLES.iter().enumerate() {
            let key = crate::shell::WORLD_STYLE_KEY_BASE + i as u32;
            let active = i == self.world_pane.min(WORLD_PANES - 1);
            let bg = if active { ui::sel_bg(pal) } else { self.hl(key, 0, ui::hover(pal)) };
            if bg != 0 {
                v.push(Cmd::Rect { x: px + self.scale.s(8.0), y: ry, w: pw - self.scale.s(16.0), h: row_h, r: 5.0, color: bg });
            }
            if active {
                v.push(Cmd::Rect { x: px + self.scale.s(8.0), y: ry + self.scale.s(4.0), w: self.scale.s(3.0), h: row_h - self.scale.s(8.0), r: 1.5, color: pal.acc });
            }
            ui::text(v, px + self.scale.s(22.0), ry + self.scale.s(19.0), *name, self.scale.fs(11.0), if active { pal.fg } else { ui::fg2(pal) }, false);
            self.region(px + self.scale.s(8.0), ry, pw - self.scale.s(16.0), row_h, key);
            ry += row_h + row_gap;
        }
        // toggle row: "Save downloads"
        let tkey = crate::shell::WORLD_MENU_TOGGLE_KEY;
        let bg = self.hl(tkey, 0, ui::hover(pal));
        if bg != 0 {
            v.push(Cmd::Rect { x: px + self.scale.s(8.0), y: ry, w: pw - self.scale.s(16.0), h: row_h, r: 5.0, color: bg });
        }
        ui::text(v, px + self.scale.s(22.0), ry + self.scale.s(19.0), "Save downloads", self.scale.fs(11.0), pal.fg, false);
        // switch track + knob
        let sw_w = self.scale.s(26.0);
        let sw_h = self.scale.s(13.0);
        let swx = px + pw - self.scale.s(8.0) - sw_w;
        let swy = ry + (row_h - sw_h) / 2.0;
        let on = self.world_save_data;
        let track = ui::mix(pal.bg, pal.fg, if on { 0.45 } else { 0.28 });
        v.push(Cmd::Rect { x: swx, y: swy, w: sw_w, h: sw_h, r: sw_h / 2.0, color: track });
        let kn = sw_h - self.scale.s(4.0);
        let kx = if on { swx + sw_w - kn - self.scale.s(2.0) } else { swx + self.scale.s(2.0) };
        v.push(Cmd::Rect { x: kx, y: swy + self.scale.s(2.0), w: kn, h: kn, r: kn / 2.0, color: pal.fg });
        self.region(px + self.scale.s(8.0), ry, pw - self.scale.s(16.0), row_h, tkey);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn newsg() -> Shell {
        let cfg: crate::config::Config =
            toml::from_str(crate::config::DEFAULT_SHELL_TOML).expect("default config parses");
        let mut shell = Shell::new(cfg);
        shell.mode = crate::shell::Mode::Expanded;
        shell.dash_enabled = true;
        shell
    }

    fn pal() -> Pal {
        Pal { fg: 0xe8e6e3ff, bg: 0x141414ff, acc: 0x4fc2ffff, sfg: 0x101010ff }
    }

    fn seed_world() {
        // a small square so the mask has land (covers most of Eurasia range)
        worldmap::set_world(vec![vec![(0.0, 0.0), (60.0, 0.0), (60.0, 40.0), (0.0, 40.0)]]);
        worldmap::set_cities(vec![
            worldmap::City { lon: 20.0, lat: 10.0, name: "Testerville".into() },
            worldmap::City { lon: 30.0, lat: 15.0, name: "Second".into() },
        ]);
    }

    #[test]
    fn each_style_pane_draws_when_data_ready() {
        let _g = worldmap::tests::TEST_LOCK.lock().unwrap();
        let mut shell = newsg();
        seed_world();
        shell.dash_layout.push(CardLayout { card: DashCard::WorldMap, x: 0, y: 0, w: 10, h: 9 });
        shell.refresh_pack();
        let p = pal();
        for pane in 0..crate::shell::WORLD_PANES {
            shell.world_pane = pane;
            let mut v = Vec::new();
            shell.draw_worldmap_card(&mut v, 10.0, 10.0, 620.0, 120.0, &p);
            assert!(shell.worldmap_rect.2 > 0.0, "card rect must be stored");
            assert!(shell.worldmap_body_rect.2 > 2.0, "pane {pane}: body too small");
            let rects = v.iter().filter(|c| matches!(c, Cmd::Rect { .. })).count();
            let maps = v.iter().filter(|c| matches!(c, Cmd::Map { .. })).count();
            assert!(rects > 0 || maps > 0, "pane {pane}: body must draw");
            let keys: Vec<u32> = shell.hover_regions.iter().map(|r| r.4).collect();
            assert!(keys.contains(&crate::shell::WORLD_MAP_KEY), "pane {pane}: map region must register");
            assert!(keys.contains(&crate::shell::WORLD_ZOOM_IN_KEY), "pane {pane}: zoom in must register");
            assert!(keys.contains(&crate::shell::WORLD_ZOOM_OUT_KEY), "pane {pane}: zoom out must register");
        }
    }

    #[test]
    fn map_still_draws_without_downloads_and_shows_download_pill() {
        let _g = worldmap::tests::TEST_LOCK.lock().unwrap();
        let mut shell = newsg();
        worldmap::clear();
        shell.dash_layout.push(CardLayout { card: DashCard::WorldMap, x: 0, y: 0, w: 10, h: 9 });
        shell.refresh_pack();
        let p = pal();
        let mut v = Vec::new();
        shell.draw_worldmap_card(&mut v, 0.0, 0.0, 620.0, 120.0, &p);
        // embedded fallback always renders the map (no placeholder screen)
        assert!(worldmap::using_fallback(), "empty store must install the embedded fallback");
        assert!(worldmap::ready(), "fallback must make the card ready");
        let keys: Vec<u32> = shell.hover_regions.iter().map(|r| r.4).collect();
        assert!(keys.contains(&crate::shell::WORLD_MAP_KEY), "map region must register without downloads");
        assert!(keys.contains(&crate::shell::WORLD_DL_KEY), "download-world button must register without data");
        let rects = v.iter().filter(|c| matches!(c, Cmd::Rect { .. })).count();
        let maps = v.iter().filter(|c| matches!(c, Cmd::Map { .. })).count();
        assert!(rects > 0 || maps > 0, "fallback map must draw");
        // a swapped-in world marks the store as non-fallback
        worldmap::clear();
        assert!(!worldmap::using_fallback(), "clear resets the fallback flag");
    }

    #[test]
    fn menu_draws_styles_and_toggle_and_closes_backdrop() {
        let _g = worldmap::tests::TEST_LOCK.lock().unwrap();
        let mut shell = newsg();
        seed_world();
        shell.world_menu = true;
        let p = pal();
        let mut v = Vec::new();
        shell.draw_worldmap_card(&mut v, 0.0, 0.0, 620.0, 120.0, &p);
        let keys: Vec<u32> = shell.hover_regions.iter().map(|r| r.4).collect();
        assert!(keys.contains(&crate::shell::WORLD_MENU_KEY), "backdrop must close the menu");
        for i in 0..crate::shell::WORLD_PANES {
            assert!(keys.contains(&(crate::shell::WORLD_STYLE_KEY_BASE + i as u32)), "style row {i}");
        }
        assert!(keys.contains(&crate::shell::WORLD_MENU_TOGGLE_KEY), "toggle row");
        let texts: Vec<String> = v.iter().filter_map(|c| if let Cmd::Text { text, .. } = c { Some(text.clone()) } else { None }).collect();
        assert!(texts.iter().any(|t| t == "Save downloads"), "toggle label must render");
    }

    #[test]
    fn region_pill_registers_when_zoomed_and_missing() {
        let _g = worldmap::tests::TEST_LOCK.lock().unwrap();
        let mut shell = newsg();
        seed_world();
        shell.world_zoom = 8.0;
        shell.world_center = (139.0, 35.0);
        use std::collections::BTreeMap;
        let m = crate::shell::mapdata::Manifest {
            world: crate::shell::mapdata::Meta { url: "world.bin".into(), sha256: "a".into(), size: 1 },
            cities: crate::shell::mapdata::Meta { url: "cities.bin".into(), sha256: "b".into(), size: 1 },
            regions: BTreeMap::from([(
                "JP".into(),
                crate::shell::mapdata::RegionMeta {
                    name: "Japan".into(),
                    bbox: [122.0, 24.0, 154.0, 46.0],
                    url: "countries/JP.bin".into(),
                    sha256: "c".into(),
                    size: 1,
                },
            )]),
        };
        shell.world_manifest = Some(std::sync::Arc::new(m));
        let p = pal();
        let mut v = Vec::new();
        shell.draw_worldmap_card(&mut v, 0.0, 0.0, 620.0, 120.0, &p);
        let keys: Vec<u32> = shell.hover_regions.iter().map(|r| r.4).collect();
        assert!(keys.contains(&crate::shell::WORLD_REGION_DL_KEY), "region download pill must register");
        assert_eq!(shell.world_region_target.as_deref(), Some("JP"));
    }

    #[test]
    fn draw_with_marker_keeps_bottom_band_text() {
        let _g = worldmap::tests::TEST_LOCK.lock().unwrap();
        let mut shell = newsg();
        seed_world();
        shell.world_marker = Some((35.68, 139.76));
        shell.world_center = (139.0, 35.0);
        shell.world_city = "Tokyo".into();
        shell.world_tz = "Asia/Tokyo".into();
        shell.world_offset_min = 540;
        shell.world_abbrev = "JST".into();
        let p = pal();
        let mut v = Vec::new();
        shell.draw_worldmap_card(&mut v, 0.0, 0.0, 620.0, 120.0, &p);
        let texts: Vec<&String> = v.iter().filter_map(|c| if let Cmd::Text { text, .. } = c { Some(text) } else { None }).collect();
        assert!(texts.iter().any(|t| t.contains("Tokyo")), "city line must render");
    }
}