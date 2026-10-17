use super::super::*;
use crate::shell::worldmap::Win;

/// Zoom at which the on-demand country-chunk download pill turns on.
const ZOOM_REGION: f32 = 4.0;

impl Shell {
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
        let (cx, cy) = self.cursor?;
        let sx = cx - self.content_dx;
        let sy = cy - self.content_dy;
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
    /// nothing should happen (cursor not over the body) or when the pan cannot
    /// move the window (whole world at zoom ≤ 1, or a world edge already
    /// clamped) — the wheel then falls through to the board instead.
    pub(crate) fn world_scroll(&mut self, v: f32, h: f32) -> bool {
        if !self.over_worldmap() {
            return false;
        }
        let win = self.world_win();
        let k = 0.12; // ~12% of the visible span per notch
        let (clon, clat) = self.world_center;
        let nl = clon + h * win.lon_span() * k;
        let nt = clat + v * win.lat_span() * k;
        // commit the new center only if the clamped window actually moves
        // (win_for pins the whole world at zoom ≤ 1 and the world edges),
        // otherwise leave the center untouched and report "nothing scrolled".
        if self.world_win_at(nl, nt) != win {
            self.world_center = (nl, nt);
            true
        } else {
            false
        }
    }

    /// The current view window. At zoom 1 it's the whole world (2:1, matched
    /// to the letterboxed body). Zoomed in, it tracks the BODY's aspect so the
    /// map fills the full card top-to-bottom with uniform degrees/pixel —
    /// the projection is never squeezed to fit.
    pub(crate) fn world_win(&self) -> Win {
        self.world_win_at(self.world_center.0, self.world_center.1)
    }

    /// The view window for an arbitrary candidate center. At zoom ≤ 1 it's
    /// pinned to the whole world on every edge regardless of the center, and
    /// zoomed in it's still clamped to the world edges.
    pub(crate) fn world_win_at(&self, clon: f32, clat: f32) -> Win {
        if self.world_zoom <= 1.0 {
            return worldmap::win_for(clon, clat, 1.0);
        }
        let (_, _, mw, mh) = self.worldmap_body_rect;
        let lon_span = (360.0 / self.world_zoom).min(360.0);
        let lat_span = (lon_span * (mh / mw.max(1.0))).min(170.0).max(0.05);
        let mut lon0 = (clon - lon_span * 0.5).max(-180.0);
        let lon1 = (lon0 + lon_span).min(180.0);
        lon0 = (lon1 - lon_span).max(-180.0);
        let mut lat0 = (clat - lat_span * 0.5).max(-85.0);
        let lat1 = (lat0 + lat_span).min(85.0);
        lat0 = (lat1 - lat_span).max(-85.0);
        Win {
            lon0,
            lat0,
            lon1,
            lat1,
        }
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

    /// The on-demand region download pill's state — shared by
    /// `card_side_effects` (which records the target iso for the click) and the
    /// Lua api. `None` when the pill is hidden: below the region zoom, no
    /// manifest, no country under the marker/center, or the chunk is already
    /// cached AND loaded. Otherwise `(iso, name, pending, cached)`.
    pub(crate) fn world_region_pill(&self) -> Option<(String, String, bool, bool)> {
        if self.world_zoom < ZOOM_REGION {
            return None;
        }
        let man = self.world_manifest.as_ref()?;
        let (clon, clat) = self
            .world_marker
            .map(|(la, lo)| (lo, la))
            .unwrap_or(self.world_center);
        let (iso, reg) = crate::shell::mapdata::region_at(man, clon, clat)?;
        let save = self.world_save_data;
        let meta = crate::shell::mapdata::Meta {
            url: reg.url.clone(),
            sha256: reg.sha256.clone(),
            size: reg.size,
        };
        let cached: bool = crate::shell::mapdata::file_valid(
            &crate::shell::mapdata::region_path(save, &iso),
            &meta,
        );
        let loaded = self.world_region.as_deref() == Some(iso.as_str());
        if cached && loaded {
            return None;
        }
        let pending =
            matches!(&self.world_dl_pending, Some(crate::shell::WorldDl::Region(s)) if s == &iso);
        Some((iso, reg.name.clone(), pending, cached))
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

    fn seed_world() {
        // a small square so the mask has land (covers most of Eurasia range)
        worldmap::set_world(vec![vec![
            (0.0, 0.0),
            (60.0, 0.0),
            (60.0, 40.0),
            (0.0, 40.0),
        ]]);
        worldmap::set_cities(vec![
            worldmap::City {
                name: "Tokyo".into(),
                lon: 139.0,
                lat: 35.0,
            },
        ]);
    }

    /// Place the card, run the full shell fan-out (which loads the live
    /// `worldmap.lua` module), and return the frame's commands.
    fn frame(shell: &mut Shell) -> Vec<Cmd> {
        shell.dash_layout.push(CardLayout {
            card: DashCard::WorldMap,
            x: 0,
            y: 0,
            w: 10,
            h: 9,
        });
        shell.refresh_pack();
        shell.layout(900.0, 340.0)
    }

    fn keys(shell: &Shell) -> Vec<u32> {
        shell.hover_regions.iter().map(|r| r.4).collect()
    }

    fn texts(v: &[Cmd]) -> Vec<String> {
        v.iter()
            .filter_map(|c| match c {
                Cmd::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn each_style_pane_draws_when_data_ready() {
        let _g = worldmap::tests::TEST_LOCK.lock().unwrap();
        let mut shell = newsg();
        seed_world();
        for pane in 0..crate::shell::WORLD_PANES {
            shell.world_pane = pane;
            let v = frame(&mut shell);
            assert!(shell.worldmap_rect.2 > 0.0, "card rect must be stored");
            assert!(
                shell.worldmap_body_rect.2 > 2.0,
                "pane {pane}: body too small"
            );
            let rects = v.iter().filter(|c| matches!(c, Cmd::Rect { .. })).count();
            let maps = v.iter().filter(|c| matches!(c, Cmd::Map { .. })).count();
            assert!(rects > 0 || maps > 0, "pane {pane}: body must draw");
            let k = keys(&shell);
            assert!(k.contains(&crate::shell::WORLD_MAP_KEY), "pane {pane}: map");
            assert!(
                k.contains(&crate::shell::WORLD_ZOOM_IN_KEY),
                "pane {pane}: zoom in"
            );
            assert!(
                k.contains(&crate::shell::WORLD_ZOOM_OUT_KEY),
                "pane {pane}: zoom out"
            );
        }
    }

    #[test]
    fn map_still_draws_without_downloads_and_shows_download_pill() {
        let _g = worldmap::tests::TEST_LOCK.lock().unwrap();
        let mut shell = newsg();
        worldmap::clear();
        let v = frame(&mut shell);
        // embedded fallback always renders the map (no placeholder screen)
        assert!(
            worldmap::using_fallback(),
            "empty store must install the embedded fallback"
        );
        assert!(worldmap::ready(), "fallback must make the card ready");
        let k = keys(&shell);
        assert!(
            k.contains(&crate::shell::WORLD_MAP_KEY),
            "map region must register without downloads"
        );
        assert!(
            k.contains(&crate::shell::WORLD_DL_KEY),
            "download-world button must register without data"
        );
        let rects = v.iter().filter(|c| matches!(c, Cmd::Rect { .. })).count();
        let maps = v.iter().filter(|c| matches!(c, Cmd::Map { .. })).count();
        assert!(rects > 0 || maps > 0, "fallback map must draw");
        // a swapped-in world marks the store as non-fallback
        worldmap::clear();
        assert!(
            !worldmap::using_fallback(),
            "clear resets the fallback flag"
        );
    }

    #[test]
    fn menu_draws_styles_and_toggle_and_closes_backdrop() {
        let _g = worldmap::tests::TEST_LOCK.lock().unwrap();
        let mut shell = newsg();
        seed_world();
        shell.world_menu = true;
        let v = frame(&mut shell);
        let k = keys(&shell);
        assert!(
            k.contains(&crate::shell::WORLD_MENU_KEY),
            "backdrop must close the menu"
        );
        for i in 0..crate::shell::WORLD_PANES {
            assert!(
                k.contains(&(crate::shell::WORLD_STYLE_KEY_BASE + i as u32)),
                "style row {i}"
            );
        }
        assert!(
            k.contains(&crate::shell::WORLD_MENU_TOGGLE_KEY),
            "toggle row"
        );
        assert!(
            texts(&v).iter().any(|t| t == "Save downloads"),
            "toggle label must render"
        );
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
            world: crate::shell::mapdata::Meta {
                url: "world.bin".into(),
                sha256: "a".into(),
                size: 1,
            },
            cities: crate::shell::mapdata::Meta {
                url: "cities.bin".into(),
                sha256: "b".into(),
                size: 1,
            },
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
        let v = frame(&mut shell);
        let k = keys(&shell);
        assert!(
            k.contains(&crate::shell::WORLD_REGION_DL_KEY),
            "region download pill must register"
        );
        assert_eq!(shell.world_region_target.as_deref(), Some("JP"));
        assert!(
            texts(&v).iter().any(|t| t.contains("Japan")),
            "region pill label must render"
        );
    }

    #[test]
    fn draw_with_marker_keeps_bottom_band_text() {
        let _g = worldmap::tests::TEST_LOCK.lock().unwrap();
        let mut shell = newsg();
        seed_world();
        shell.world_marker = Some((35.0, 139.0));
        shell.world_zoom = 1.0;
        shell.world_city = "Tokyo".into();
        shell.world_offset_min = 540;
        shell.world_abbrev = "JST".into();
        let v = frame(&mut shell);
        assert!(
            texts(&v).iter().any(|t| t.contains("Tokyo")),
            "city line must render"
        );
    }
}
