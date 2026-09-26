//! App-facing panels: launcher, weather detail, wallpaper, themes, clipboard x2.

use super::*;

impl Shell {
        pub(crate) fn layout_launcher(&mut self, v: &mut Vec<Cmd>, w: f32, _h: f32, pal: &Pal) {
        ui::text(v, self.scale.s(16.0), self.scale.s(12.0), "Search apps", self.scale.fs(15.0), pal.fg, false);
        // search input
        v.push(Cmd::Rect {
            x: self.scale.s(16.0),
            y: self.scale.s(40.0),
            w: w - self.scale.s(32.0),
            h: self.scale.s(42.0),
            r: self.scale.s(12.0),
            color: self.hl(1, ui::raised(&pal), ui::raised_hl(&pal)),
        });
        ui::outline(v, self.scale.s(16.0), self.scale.s(40.0), w - self.scale.s(32.0), self.scale.s(42.0), self.scale.s(12.0), ui::hairline(&pal));
        ui::text(v, self.scale.s(28.0), self.scale.s(52.0), ICON_SEARCH, self.scale.fs(14.0), ui::fg3(&pal), true);
        ui::text(v, self.scale.s(52.0), self.scale.s(52.0), format!("{}|", self.query), self.scale.fs(14.0), pal.fg, false);
        self.region(self.scale.s(16.0), self.scale.s(40.0), w - self.scale.s(32.0), self.scale.s(42.0), 1);
        // Prefer cache; refresh without storing when stale.
        let hits: Vec<usize> = if self.launcher_hits_query == self.query {
            self.launcher_hits.clone()
        } else {
            self.apps.search_indices(&self.query, 20)
        };
        let visible = self.launcher_visible();
        let n = hits.len();
        let scroll = self.launcher_scroll.min(n.saturating_sub(visible));
        let row_w = w - self.scale.s(32.0) - if n > visible { self.scale.s(14.0) } else { 0.0 };
        for (vi, &idx) in hits.iter().skip(scroll).take(visible).enumerate() {
            let Some(app) = self.apps.apps.get(idx) else { continue };
            let i = scroll + vi;
            let ry = self.scale.s(92.0) + vi as f32 * self.scale.s(46.0);
            let key = 10 + vi as u32;
            let sel = i == self.sel;
            let hov = self.hover_key == key;
            if sel {
                v.push(Cmd::Rect { x: self.scale.s(16.0), y: ry, w: row_w, h: self.scale.s(40.0), r: self.scale.s(10.0), color: ui::sel_bg(&pal) });
            }
            if hov {
                v.push(Cmd::Rect { x: self.scale.s(16.0), y: ry, w: row_w, h: self.scale.s(40.0), r: self.scale.s(10.0), color: ui::hover(&pal) });
            }
            if sel {
                v.push(Cmd::Rect { x: self.scale.s(16.0), y: ry + self.scale.s(6.0), w: self.scale.s(3.0), h: self.scale.s(28.0), r: self.scale.s(1.5), color: pal.acc });
            }
            // icon tile
            v.push(Cmd::Rect { x: self.scale.s(28.0), y: ry + self.scale.s(6.0), w: self.scale.s(28.0), h: self.scale.s(28.0), r: self.scale.s(8.0), color: ui::acc_tint(&pal) });
            ui::text(v, self.scale.s(34.0), ry + self.scale.s(10.0), ICON_APPS, self.scale.fs(15.0), pal.acc, true);
            if let Some(key) = app.icon_key() {
                v.push(Cmd::Image { x: self.scale.s(28.0), y: ry + self.scale.s(6.0), w: self.scale.s(28.0), h: self.scale.s(28.0), key });
            }
            let name_c = if sel || hov { pal.fg } else { ui::fg2(&pal) };
            ui::text(v, self.scale.s(66.0), ry + self.scale.s(6.0), app.name.clone(), self.scale.fs(13.0), name_c, false);
            if !app.category.is_empty() {
                ui::text(v, self.scale.s(66.0), ry + self.scale.s(24.0), app.category.clone(), self.scale.fs(10.0), ui::fg3(&pal), false);
            }
            self.region(self.scale.s(16.0), ry, row_w, self.scale.s(40.0), key);
        }
        // scrollbar (only when the list overflows)
        if n > visible {
            let track_y = self.scale.s(92.0);
            let track_h = visible as f32 * self.scale.s(46.0) - self.scale.s(8.0);
            v.push(Cmd::Rect { x: w - self.scale.s(12.0), y: track_y, w: self.scale.s(4.0), h: track_h, r: self.scale.s(2.0), color: ui::hover(&pal) });
            let span = (n - visible) as f32;
            let thumb_h = (track_h / n as f32 * visible as f32).max(self.scale.s(16.0));
            let thumb_y = track_y + (track_h - thumb_h) * (scroll as f32 / span);
            v.push(Cmd::Rect { x: w - self.scale.s(12.0), y: thumb_y, w: self.scale.s(4.0), h: thumb_h, r: self.scale.s(2.0), color: mix(ui::hover(&pal), pal.fg, 0.32) });
        }
        if hits.is_empty() {
            ui::text(
                v,
                self.scale.s(24.0),
                self.scale.s(100.0),
                if self.query.is_empty() {
                    "Type to search installed apps"
                } else {
                    "No matches"
                },
                self.scale.fs(12.0),
                ui::fg3(&pal),
                false,
            );
        }
    }

        pub(crate) fn layout_weather(&mut self, v: &mut Vec<Cmd>, w: f32, _h: f32, pal: &Pal) {
        // header: close + city
        ui::text(v, self.scale.s(22.0), self.scale.s(16.0), ICON_BACK, self.scale.fs(14.0), pal.fg, true);
        self.region(self.scale.s(12.0), self.scale.s(8.0), self.scale.s(28.0), self.scale.s(28.0), 1);
        ui::text(v, self.scale.s(48.0), self.scale.s(16.0), "Weather", self.scale.fs(15.0), pal.fg, false);
        ui::text_r(v, w - self.scale.s(16.0), self.scale.s(16.0), &self.weather_city, self.scale.fs(12.0), ui::fg3(&pal), false);
        let Some(wx) = self.weather.as_ref() else {
            ui::text(v, self.scale.s(24.0), self.scale.s(64.0), "No weather data — set [weather] in config/shell.toml", self.scale.fs(12.0), ui::fg3(&pal), false);
            return;
        };
        let (glyph, desc) = crate::weather::describe(wx.code);
        // hero
        ui::text_c(v, w / 2.0, self.scale.s(74.0), format!("{:.0}°", wx.temp_c), self.scale.fs(56.0), pal.fg, false);
        ui::text_c(v, w / 2.0, self.scale.s(144.0), format!("{glyph} {desc}"), self.scale.fs(18.0), pal.acc, true);
        // detail rows
        let rows: [(&str, String); 4] = [
            ("Feels like", wx.feels_like.map(|f| format!("{f:.0}°C")).unwrap_or_default()),
            ("Humidity", wx.humidity.map(|x| format!("{x:.0}%")).unwrap_or_default()),
            ("Wind", format!("{:.0} km/h", wx.wind_kmh)),
            ("Precipitation", wx.precip_mm.map(|p| format!("{p:.1} mm")).unwrap_or_default()),
        ];
        let mut y = self.scale.s(196.0);
        for (label, val) in rows {
            if val.is_empty() {
                continue;
            }
            ui::text(v, self.scale.s(70.0), y, label, self.scale.fs(12.0), ui::fg2(&pal), false);
            ui::text_r(v, w - self.scale.s(70.0), y, val, self.scale.fs(12.0), pal.fg, false);
            y += self.scale.s(28.0);
        }
    }

        pub(crate) fn layout_wallpaper(&mut self, v: &mut Vec<Cmd>, w: f32, h: f32, pal: &Pal) {
        // A declared `wallpaper` surface in `shell.ron` owns the header chrome
        // (back button + "Backgrounds" title); the thumbnail grid + empty
        // state + scrollbar stay in Rust via the `wallpaper_picker` Ink
        // (`layout_wallpaper_body`). Without a surface the built-in Rust
        // header below runs unchanged.
        let vals = self.scene_values();
        if self.draw_shell_surface("wallpaper", v, w, h, pal, &vals, None) {
            return;
        }
        // header: close + title
        ui::text(v, self.scale.s(22.0), self.scale.s(16.0), ICON_BACK, self.scale.fs(14.0), pal.fg, true);
        self.region(self.scale.s(12.0), self.scale.s(8.0), self.scale.s(28.0), self.scale.s(28.0), 1);
        ui::text(v, self.scale.s(48.0), self.scale.s(16.0), "Backgrounds", self.scale.fs(15.0), pal.fg, false);
        self.layout_wallpaper_body(v, w, h, pal);
    }

    /// Body painter for a declared `wallpaper` surface: the empty state, the
    /// thumbnail grid, and the scrollbar — everything below the header chrome.
    /// Also the tail half of the fallback `layout_wallpaper`.
    pub(crate) fn layout_wallpaper_body(&mut self, v: &mut Vec<Cmd>, w: f32, _h: f32, pal: &Pal) {
        let files = self.wallpapers.clone();
        if files.is_empty() {
            ui::text(
                v,
                self.scale.s(24.0),
                self.scale.s(64.0),
                "No images — set [wallpaper] dir in config/shell.toml",
                self.scale.fs(12.0),
                ui::fg3(&pal),
                false,
            );
            return;
        }
        // thumbnail grid
        let cols = self.wp_cols() as f32;
        let n = files.len();
        let visible = self.wallpaper_visible();
        let scroll = self.wallpaper_scroll.min(self.wallpaper_max_scroll());
        let grid_w = cols * self.wp_cell + (cols - 1.0) * Self::WP_GAP;
        let x0 = (w - grid_w) / 2.0;
        let y0 = Self::WP_Y0;
        for (vi, path) in files.iter().skip(scroll).take(visible).enumerate() {
            let cx = x0 + (vi as f32 % cols) * (self.wp_cell + Self::WP_GAP);
            let cy = y0 + (vi as f32 / cols).floor() * (self.wp_cell + Self::WP_GAP);
            let key = 10 + vi as u32;
            let hov = self.hover_key == key;
            v.push(Cmd::Rect {
                x: cx,
                y: cy,
                w: self.wp_cell,
                h: self.wp_cell,
                r: self.scale.s(12.0),
                color: if hov { ui::hover_hl(&pal) } else { ui::hover(&pal) },
            });
v.push(Cmd::Image {
                    x: cx + self.scale.s(4.0),
                    y: cy + self.scale.s(4.0),
                    w: self.wp_cell - self.scale.s(8.0),
                    h: self.wp_cell - self.scale.s(8.0),
                    key: format!("thumb:{path}"),
                });
                self.region(cx, cy, self.wp_cell, self.wp_cell, key);
        }
        // scrollbar
        if n > visible {
            let (tx, ty, tw, th) = self.wp_track();
            v.push(Cmd::Rect { x: tx, y: ty, w: tw, h: th, r: tw / 2.0, color: ui::hover(&pal) });
            let (rel, thumb_h) = self.wp_thumb(th);
            let active = self.wp_sb_drag || self.hover_key == 2;
            v.push(Cmd::Rect {
                x: tx,
                y: ty + rel,
                w: tw,
                h: thumb_h,
                r: tw / 2.0,
                color: if active { mix(ui::hover(&pal), pal.fg, 0.55) } else { mix(ui::hover(&pal), pal.fg, 0.32) },
            });
            self.region(tx - self.scale.s(8.0), ty - self.scale.s(4.0), tw + self.scale.s(14.0), th + self.scale.s(8.0), 2);
        }
    }

    /// Theme picker — one card per entry of $states2/themes JSON mirror.
    /// Each half shows the theme's palette in action: bg surface, border
    /// ring (2 px), "Lorem" in fg, and an acc pill with "Ipsum" in sfg.
    /// A dual theme shows BOTH halves side-by-side (dark left, light right).
    /// Click runs `scheme_main apply <id>`; scroll wheels rows.

        pub(crate) fn layout_themes(&mut self, v: &mut Vec<Cmd>, w: f32, h: f32, pal: &Pal) {
        // color-scheme detail subview (opened by a card's ">" chevron)
        if let Some(detail) = self.theme_detail.clone() {
            return self.layout_theme_detail(v, w, pal, &detail);
        }
        // header: close + title
        ui::text(v, self.scale.s(22.0), self.scale.s(16.0), ICON_BACK, self.scale.fs(14.0), pal.fg, true);
        self.region(self.scale.s(12.0), self.scale.s(8.0), self.scale.s(28.0), self.scale.s(28.0), 1);
        ui::text(v, self.scale.s(48.0), self.scale.s(16.0), "Themes", self.scale.fs(15.0), pal.fg, false);
        ui::text_r(v, w - self.scale.s(16.0), self.scale.s(16.0), "click to apply · theme_main", self.scale.fs(10.0), ui::fg3(&pal), false);

        let total = self.themes.len();
        let dual_count = self.themes.iter().filter(|c| c.dual).count();
        // count line under the title: "N themes · M dual" (dual fill on dual cards)
        if total > 0 {
            let count_str = format!("{total} themes · {dual_count} dual");
            ui::text(v, self.scale.s(48.0), self.scale.s(31.0), count_str, self.scale.fs(10.0), ui::fg3(&pal), false);
        }
        if self.themes.is_empty() {
            ui::text(v, self.scale.s(24.0), self.scale.s(64.0), "No themes — mirror missing or empty ($states2/themes)", self.scale.fs(12.0), ui::fg3(&pal), false);
            return;
        }
        // search bar — filters the grid as you type
        v.push(Cmd::Rect {
            x: self.scale.s(48.0),
            y: self.scale.s(44.0),
            w: w - self.scale.s(96.0),
            h: self.scale.s(40.0),
            r: self.scale.s(12.0),
            color: self.hl(1, ui::hover(&pal), ui::hover_hl(&pal)),
        });
        ui::text(v, self.scale.s(60.0), self.scale.s(56.0), ICON_SEARCH, self.scale.fs(14.0), ui::fg3(&pal), true);
        ui::text(v, self.scale.s(82.0), self.scale.s(56.0), format!("{}|", self.themes_query), self.scale.fs(14.0), pal.fg, false);
        self.region(self.scale.s(48.0), self.scale.s(44.0), w - self.scale.s(96.0), self.scale.s(40.0), 1);

        let hits = self.filtered_themes();
        let cols = self.th_cols() as f32;
        let card_w = self.th_card_w();
        let visible = self.themes_visible();
        self.themes_scroll = self.themes_scroll.min(self.themes_max_scroll());
        let scroll = self.themes_scroll;
        let grid_w = cols * card_w + (cols - 1.0) * Shell::TH_GAP;
        let x0 = (w - grid_w) / 2.0;
        let y0 = Shell::TH_Y0;
        if hits.is_empty() {
            ui::text(v, self.scale.s(24.0), self.scale.s(120.0), "No themes match", self.scale.fs(12.0), ui::fg3(&pal), false);
            let _ = x0;
            return;
        }
        for (vi, &idx) in hits.iter().skip(scroll).take(visible).enumerate() {
            let Some(card) = self.themes.get(idx) else { continue };
            let cx = x0 + (vi as f32 % cols) * (card_w + Shell::TH_GAP);
            let cy = y0 + (vi as f32 / cols).floor() * (Shell::TH_H + Shell::TH_GAP);
            let key = 10 + vi as u32;
            let hov = self.hover_key == key;
            let is_cur = card.name == self.cur_theme;
            v.push(Cmd::Rect {
                x: cx,
                y: cy,
                w: card_w,
                h: Shell::TH_H,
                r: self.scale.s(14.0),
                color: if hov { ui::hover_hl(&pal) } else { mix(ui::hover(&pal), pal.fg, 0.04) },
            });
            // name + current-theme marker dot
            if is_cur {
                v.push(Cmd::Rect { x: cx + self.scale.s(14.0), y: cy + self.scale.s(17.0), w: self.scale.s(7.0), h: self.scale.s(7.0), r: self.scale.s(3.5), color: pal.acc });
            }
            let nx = if is_cur { cx + self.scale.s(26.0) } else { cx + self.scale.s(14.0) };
            ui::text(v, nx, cy + self.scale.s(10.0), card.name.clone(), self.scale.fs(12.5), if is_cur { pal.acc } else { pal.fg }, true);
            // preview strip — each half shows the theme's palette in action:
            //   bg surface, border ring (2 px), "Lorem" in fg, then a
            //   small acc-colored pill with "Ipsum" in sfg on top.
            // Dual themes render both halves side-by-side.
            let px = cx + self.scale.s(12.0);
            let py = cy + self.scale.s(36.0);
            let pw = card_w - self.scale.s(24.0);
            let ph = Shell::TH_H - self.scale.s(48.0);
            let gap = self.scale.s(6.0);
            let draw_half = |v: &mut Vec<Cmd>, hx: f32, hw: f32, p: &crate::themes::Palette| {
                // bg surface
                v.push(Cmd::Rect { x: hx, y: py, w: hw, h: ph, r: self.scale.s(9.0), color: p.bg });
                // border ring — 2 px
                v.push(Cmd::Outline { x: hx, y: py, w: hw, h: ph, r: self.scale.s(9.0), width: self.scale.s(2.0), color: p.border });
                // "Lorem" in fg
                let pad = self.scale.s(10.0);
                ui::text(v, hx + pad, py + self.scale.s(6.0), "Lorem", self.scale.fs(11.0), p.fg, false);
                // small acc-colored pill with "Ipsum" in sfg
                let pill_y = py + self.scale.s(22.0);
                let pill_w = hw - pad * 2.0;
                let pill_h = self.scale.s(20.0);
                v.push(Cmd::Rect { x: hx + pad, y: pill_y, w: pill_w, h: pill_h, r: self.scale.s(6.0), color: p.acc });
                ui::text_c(v, hx + pad + pill_w / 2.0, pill_y + pill_h / 2.0, "Ipsum", self.scale.fs(10.0), p.sfg, false);
            };
            match &card.light {
                Some(l) => {
                    let hw = pw / 2.0 - gap / 2.0;
                    draw_half(v, px, hw, &card.dark);
                    draw_half(v, px + hw + gap, hw, l);
                }
                None => draw_half(v, px, pw, &card.dark),
            }
            self.region(cx, cy, card_w, Shell::TH_H, key);
            // ">" chevron (top-right corner) — fetches + shows the FULL color
            // scheme for this card. Key band 100..=199, registered AFTER the
            // card region so it wins the corner (regions hit-test in reverse).
            let chevron_key = 100 + vi as u32;
            let chx = cx + card_w - self.scale.s(24.0);
            let chy = cy + self.scale.s(8.0);
            ui::text(v, chx, chy, ICON_FORWARD, self.scale.fs(12.0), self.hl(chevron_key, ui::fg3(&pal), pal.acc), true);
            self.region(chx - self.scale.s(6.0), chy - self.scale.s(4.0), self.scale.s(24.0), self.scale.s(24.0), chevron_key);
        }
        // theme-grid scrollbar (only when it overflows vertically)
        let total_rows = (hits.len() + self.th_cols() - 1) / self.th_cols();
        if total_rows > self.th_rows() {
            let (tx, ty, tw, th) = self.th_track();
            v.push(Cmd::Rect { x: tx, y: ty, w: tw, h: th, r: tw / 2.0, color: ui::hover(&pal) });
            let (rel, thumb_h) = self.th_thumb(th);
            let active = self.th_sb_drag || self.hover_key == 2;
            v.push(Cmd::Rect {
                x: tx,
                y: ty + rel,
                w: tw,
                h: thumb_h,
                r: tw / 2.0,
                color: if active { mix(ui::hover(&pal), pal.fg, 0.55) } else { mix(ui::hover(&pal), pal.fg, 0.32) },
            });
            self.region(tx - self.scale.s(8.0), ty - self.scale.s(4.0), tw + self.scale.s(14.0), th + self.scale.s(8.0), 2);
        }
        let _ = h;
    }

    /// Theme color-scheme detail (opened by a card's ">" chevron). Reads the
    /// wire shape `scheme_body`'s fetch_card emits and shows it grouped under
    /// `Base colors` / `Extras` / `Series` section headers. No dark/light
    /// labels — the grid preview already shows which half is which, and a
    /// block's own flag decides if it is shown at all: extras only when the
    /// side's `e` flag is 1, series only when its `s` flag is 1.
    /// The `<` at top-left returns to the grid.
    pub(crate) fn layout_theme_detail(
        &mut self,
        v: &mut Vec<Cmd>,
        w: f32,
        pal: &Pal,
        detail: &crate::themes::ThemeDetail,
    ) {
        // header: `<` back-to-grid + title (no dark/light labels — see doc)
        ui::text(v, self.scale.s(22.0), self.scale.s(16.0), ICON_BACK, self.scale.fs(14.0), self.hl(1, pal.fg, pal.acc), true);
        self.region(self.scale.s(12.0), self.scale.s(8.0), self.scale.s(28.0), self.scale.s(28.0), 1);
        ui::text(v, self.scale.s(48.0), self.scale.s(16.0), format!("{} — color scheme", detail.name), self.scale.fs(15.0), pal.fg, false);
        ui::text_r(v, w - self.scale.s(16.0), self.scale.s(16.0), "click < to back", self.scale.fs(10.0), ui::fg3(&pal), false);

        let dark = match &detail.dark {
            Some(s) => s,
            None => {
                ui::text(v, self.scale.s(24.0), self.scale.s(64.0), "No colors — fetch link missing", self.scale.fs(12.0), ui::fg3(&pal), false);
                return;
            }
        };

        let col_gap = self.scale.s(16.0);
        let avail = w - self.scale.s(48.0) - col_gap;
        let two_col = detail.light.is_some();
        let col_w = if two_col { avail / 2.0 } else { avail };

        let row_y0 = self.scale.s(56.0);
        let row_h = self.scale.s(17.0);
        let sw = self.scale.s(13.0);

        let draw_side = |v: &mut Vec<Cmd>, x: f32, label: &str, side: &crate::themes::WireSide, extras_on: bool, series_on: bool| {
            let draw_row = |v: &mut Vec<Cmd>, ri: usize, label: &str, color: &str| {
                let y = row_y0 + ri as f32 * row_h;
                let c = if color.is_empty() || !color.starts_with('#') { 0xff00ffff } else { crate::themes::hex(color) };
                v.push(Cmd::Rect { x, y, w: sw, h: sw, r: self.scale.s(3.0), color: c });
                ui::text(v, x + sw + self.scale.s(6.0), y, label.to_string(), self.scale.fs(10.0), ui::fg2(&pal), false);
                ui::text_r(v, x + col_w, y, color.to_string(), self.scale.fs(9.5), pal.fg, false);
            };
            let draw_header = |v: &mut Vec<Cmd>, ri: usize, text: &str| {
                let y = row_y0 + ri as f32 * row_h;
                ui::text(v, x, y, text.to_string(), self.scale.fs(10.5), pal.acc, false);
            };

            let mut ri = 0;
            // Dark/Light heading (this half of the theme) above the base block
            draw_header(v, ri, label);
            ri += 1;
            // Base colors — always shown. The base tier (acc_us/acc_hc/
            // acc_ec) are flat side fields now, always present on every side
            // (previously they lived inside the extras object).
            draw_header(v, ri, "Base colors");
            ri += 1;
            for (label, color) in [
                ("bg", side.bg.as_str()),
                ("fg", side.fg.as_str()),
                ("acc", side.acc.as_str()),
            ] {
                draw_row(v, ri, label, color);
                ri += 1;
            }
            for (label, color) in [
                ("acc_us", side.acc_us.as_str()),
                ("acc_hc", side.acc_hc.as_str()),
                ("acc_ec", side.acc_ec.as_str()),
            ] {
                draw_row(v, ri, label, color);
                ri += 1;
            }
            for (label, color) in [
                ("sfg", side.sfg.as_str()),
                ("border", side.border.as_str()),
            ] {
                draw_row(v, ri, label, color);
                ri += 1;
            }
            // extras block: only when the theme actually STORED it (e flag = 1)
            if extras_on {
                draw_header(v, ri, "Extras");
                ri += 1;
                let e = &side.extras;
                for (label, color) in [
                    ("hover", e.hover.as_str()),
                    ("focus", e.focus.as_str()),
                    ("disabled", e.disabled.as_str()),
                    ("sfg_d", e.sfg_d.as_str()),
                    ("sfg_l", e.sfg_l.as_str()),
                ] {
                    draw_row(v, ri, label, color);
                    ri += 1;
                }
            }
            // series block: only when the theme actually STORED it (s flag = 1)
            if series_on {
                draw_header(v, ri, "Series");
                ri += 1;
                for i in 0..6 {
                    let color = side.series.get(i).map(|s| s.as_str()).unwrap_or("");
                    draw_row(v, ri, &format!("series{}", i + 1), color);
                    ri += 1;
                }
            }
        };

        // dual themes render dark (e1/s1) on the LEFT, light (e2/s2) on the RIGHT
        let dark_label = if two_col { "Dark" } else if detail.mode == "l" { "Light" } else { "Dark" };
        draw_side(v, self.scale.s(24.0), dark_label, dark, detail.e1 == 1, detail.s1 == 1);
        if let Some(light) = &detail.light {
            draw_side(v, self.scale.s(24.0) + col_w + col_gap, "Light", light, detail.e2 == 1, detail.s2 == 1);
        }
    }

    /// Keybind viewer (SUPER+SHIFT+/) — a read-only scrollable list from
    /// $states2/keybinds_cache (JSON `{key,desc}[]` emitted by the
    /// gen_keybinds_cache helper). Each row shows the combo in acc and the
    /// description in fg. Click / Esc collapses; wheel scrolls the list.
    pub(crate) fn layout_keybinds(&mut self, v: &mut Vec<Cmd>, w: f32, h: f32, pal: &Pal) {
        // header: close + title
        ui::text(v, self.scale.s(22.0), self.scale.s(16.0), ICON_BACK, self.scale.fs(14.0), pal.fg, true);
        self.region(self.scale.s(12.0), self.scale.s(8.0), self.scale.s(28.0), self.scale.s(28.0), 1);
        ui::text(v, self.scale.s(48.0), self.scale.s(16.0), "Keybinds", self.scale.fs(15.0), pal.fg, false);
        ui::text_r(v, w - self.scale.s(16.0), self.scale.s(16.0), "scroll to browse", self.scale.fs(10.0), ui::fg3(&pal), false);

        // count line under the title
        if !self.keybinds.is_empty() {
            let count = format!("{} binds", self.keybinds.len());
            ui::text(v, self.scale.s(48.0), self.scale.s(31.0), &count, self.scale.fs(10.0), ui::fg3(&pal), false);
        }
        if self.keybinds.is_empty() {
            ui::text(v, self.scale.s(24.0), self.scale.s(64.0), "No keybinds — generate $states2/keybinds_cache from keybinds.lua first", self.scale.fs(12.0), ui::fg3(&pal), false);
            return;
        }
        let row_h = Shell::KB_ROW_H;
        let gap = Shell::KB_GAP;
        let y0 = Shell::KB_Y0;
        let visible = self.keybinds_visible();
        self.keybinds_scroll = self.keybinds_scroll.min(self.keybinds_max_scroll());
        let scroll = self.keybinds_scroll;
        let x = self.scale.s(12.0);
        let row_w = w - self.scale.s(30.0);
        let slice: Vec<crate::themes::Keybind> = self.keybinds.iter().skip(scroll).take(visible).cloned().collect();
        for (vi, kb) in slice.iter().enumerate() {
            let key = 10 + vi as u32;
            let y = y0 + vi as f32 * (row_h + gap);
            let hov = self.hover_key == key;
            v.push(Cmd::Rect {
                x,
                y,
                w: row_w,
                h: row_h,
                r: self.scale.s(8.0),
                color: if hov { ui::hover_hl(&pal) } else { ui::hover(&pal) },
            });
            // key combo in acc
            ui::text(v, x + self.scale.s(12.0), y + self.scale.s(9.0), &kb.key, self.scale.fs(11.0), pal.acc, false);
            // description — char-clipped so it never overflows past the scrollbar
            let desc: String = kb.desc.chars().take(56).collect();
            ui::text(v, x + self.scale.s(170.0), y + self.scale.s(9.0), &desc, self.scale.fs(11.0), ui::fg2(&pal), false);
            self.region(x, y, row_w, row_h, key);
        }
        // scrollbar (only when the list overflows vertically)
        if self.keybinds.len() > visible {
            let track_h = visible as f32 * (row_h + gap) - gap;
            let sb_x = w - self.scale.s(14.0);
            v.push(Cmd::Rect { x: sb_x, y: y0, w: self.scale.s(4.0), h: track_h, r: self.scale.s(2.0), color: ui::hover(&pal) });
            let span = self.keybinds_max_scroll() as f32;
            let thumb_h = (track_h * visible as f32 / self.keybinds.len() as f32).max(self.scale.s(16.0));
            let thumb_y = y0 + (track_h - thumb_h) * (scroll.min(span as usize) as f32 / span.max(1.0));
            v.push(Cmd::Rect { x: sb_x, y: thumb_y, w: self.scale.s(4.0), h: thumb_h, r: self.scale.s(2.0), color: mix(ui::hover(&pal), pal.fg, 0.32) });
        }
        let _ = h;
    }

    /// Clipboard history — text entries (SUPER+V). Rows show a glyph + a
    /// single-line preview; click (or arrow + Enter) copies and closes.

        pub(crate) fn layout_clipboard(&mut self, v: &mut Vec<Cmd>, w: f32, _h: f32, pal: &Pal) {
        ui::text(v, self.scale.s(18.0), self.scale.s(16.0), "Clipboard — text", self.scale.fs(14.0), pal.fg, false);
        ui::text_r(v, w - self.scale.s(16.0), self.scale.s(16.0), "click to copy · scroll to browse", self.scale.fs(10.0), ui::fg3(&pal), false);
        if self.clip_text.is_empty() {
            ui::text(v, self.scale.s(24.0), self.scale.s(64.0), "No text history yet — copy something", self.scale.fs(12.0), ui::fg3(&pal), false);
            return;
        }
        let row_h = Shell::CLIP_ROW_H;
        let y0 = Shell::CLIP_Y0;
        self.clip_scroll = self.clip_scroll.min(self.clip_max_scroll());
        let scroll = self.clip_scroll;
        let visible = self.clip_visible();
        for vi in 0..visible {
            let i = scroll + vi;
            if i >= self.clip_text.len() {
                break;
            }
            let key = 10 + vi as u32;
            let y = y0 + vi as f32 * row_h;
            let sel = self.clip_sel == i;
            let hov = self.hover_key == key;
            v.push(Cmd::Rect {
                x: self.scale.s(10.0),
                y,
                w: w - self.scale.s(28.0),
                h: row_h - self.scale.s(6.0),
                r: self.scale.s(10.0),
                color: if sel {
                    ui::sel_bg(&pal)
                } else if hov {
                    ui::hover_hl(&pal)
                } else {
                    ui::hover(&pal)
                },
            });
            ui::text(v, self.scale.s(20.0), y + self.scale.s(9.0), ICON_SELECT, self.scale.fs(15.0), if sel { pal.acc } else { ui::fg3(&pal) }, true);
            let preview: String = self.clip_text[i].chars().take(72).collect();
            ui::text(v, self.scale.s(42.0), y + self.scale.s(11.0), &preview, self.scale.fs(11.0), if sel { pal.fg } else { ui::fg2(&pal) }, false);
            self.region(self.scale.s(10.0), y, w - self.scale.s(28.0), row_h - self.scale.s(6.0), key);
        }
        // scrollbar (only when the list overflows vertically)
        if self.clip_text.len() > visible {
            let track_h = visible as f32 * row_h - self.scale.s(4.0);
            let sb_x = w - self.scale.s(14.0);
            v.push(Cmd::Rect { x: sb_x, y: y0, w: self.scale.s(4.0), h: track_h, r: self.scale.s(2.0), color: ui::hover(&pal) });
            let span = self.clip_max_scroll() as f32;
            let thumb_h = (track_h * visible as f32 / self.clip_text.len() as f32).max(self.scale.s(16.0));
            let thumb_y = y0 + (track_h - thumb_h) * (scroll.min(span as usize) as f32 / span.max(1.0));
            v.push(Cmd::Rect { x: sb_x, y: thumb_y, w: self.scale.s(4.0), h: thumb_h, r: self.scale.s(2.0), color: mix(ui::hover(&pal), pal.fg, 0.32) });
        }
    }

    /// Clipboard history — image entries (SUPER+SHIFT+V), thumbnail grid.

        pub(crate) fn layout_clipboard_images(&mut self, v: &mut Vec<Cmd>, w: f32, _h: f32, pal: &Pal) {
        ui::text(v, self.scale.s(18.0), self.scale.s(16.0), "Clipboard — images", self.scale.fs(14.0), pal.fg, false);
        ui::text_r(v, w - self.scale.s(16.0), self.scale.s(16.0), "click to copy", self.scale.fs(10.0), ui::fg3(&pal), false);
        if self.clip_images.is_empty() {
            ui::text(v, self.scale.s(24.0), self.scale.s(64.0), "No image history yet — copy an image", self.scale.fs(12.0), ui::fg3(&pal), false);
            return;
        }
        let cell = self.scale.s(96.0);
        let gap = self.scale.s(14.0);
        let cols = 4.0;
        let x0 = (w - (cols * cell + (cols - 1.0) * gap)) / 2.0;
        let y0 = self.scale.s(52.0);
        let keys: Vec<String> = self.clip_images.iter().cloned().take(12).collect();
        for (i, key) in keys.iter().enumerate() {
            let cx = x0 + (i as f32 % cols) * (cell + gap);
            let cy = y0 + (i as f32 / cols).floor() * (cell + gap);
            let keyn = 10 + i as u32;
            let sel = self.clip_sel == i;
            let hov = self.hover_key == keyn;
            v.push(Cmd::Rect {
                x: cx,
                y: cy,
                w: cell,
                h: cell,
                r: self.scale.s(12.0),
                color: if sel {
                    ui::sel_bg(&pal)
                } else if hov {
                    ui::hover_hl(&pal)
                } else {
                    ui::hover(&pal)
                },
            });
            v.push(Cmd::Image { x: cx + self.scale.s(4.0), y: cy + self.scale.s(4.0), w: cell - self.scale.s(8.0), h: cell - self.scale.s(8.0), key: key.clone() });
            self.region(cx, cy, cell, cell, keyn);
        }
    }
}
