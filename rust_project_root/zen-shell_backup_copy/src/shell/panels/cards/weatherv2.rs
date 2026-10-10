use super::super::*;

impl Shell {

    fn today_month_day(&self) -> (String, u8) {
        const MONTHS: [&str; 12] = [
            "JAN","FEB","MAR","APR","MAY","JUN",
            "JUL","AUG","SEP","OCT","NOV","DEC",
        ];
        unsafe {
            let now = libc::time(std::ptr::null_mut());
            let mut tm: libc::tm = std::mem::zeroed();
            libc::localtime_r(&now, &mut tm);
            let mon = tm.tm_mon.rem_euclid(12) as usize;
            (MONTHS[mon].to_string(), tm.tm_mday as u8)
        }
    }

    /// WEATHER V2 — centered weather card.
    ///
    /// ```text
    /// ┌──────────────────────────────────────┐
    /// │              ☀ (glyph)              │
    /// │             16°C  Clear              │
    /// │  Home              ┌────────┐       │
    /// │                      │ SEP  │       │
    /// │                      │  14   │       │
    /// │  ─────────────────────────────────── │
    /// │  💨 8 km/h    💧 51%    ☀ 70%      │
    /// └──────────────────────────────────────┘
    /// ```
    pub(crate) fn draw_weather_v2_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        self.weather_rect = (x, y, w, h);
        let pad = self.scale.s(14.0);
        let ccx = x + w / 2.0;

        let Some(wx) = self.weather.as_ref().cloned() else {
            ui::text_c(v, ccx, y + h / 2.0 - self.scale.s(4.0), ICON_SUNNY, self.scale.fs(18.0), ui::fg2(pal), true);
            ui::caption_c(v, ccx, y + h / 2.0 + self.scale.s(14.0), "waiting…", self.scale.fs(9.5), ui::fg2(pal), false);
            return;
        };

        let (glyph, desc) = crate::weather::describe(wx.code);
        let city = if self.weather_city.is_empty() {
            wx.city.clone().unwrap_or_default()
        } else {
            self.weather_city.clone()
        };

        // Sunshine = 100 − today's rain probability
        let sunshine = wx.daily.first()
            .map(|d| (100.0 - d.precip_prob).clamp(0.0, 100.0).round() as i32);

        // ── short card: compact single-line fallback ──
        if h < self.scale.s(140.0) {
            ui::text_c(v, ccx, y + h / 2.0 - self.scale.s(4.0), format!("{glyph} {:.0}°C", wx.temp_c), self.scale.fs(13.0), pal.fg, true);
            let mut meta = String::from(desc);
            if !city.is_empty() {
                meta.push_str(" · ");
                meta.push_str(&city);
            }
            ui::caption_c(v, ccx, y + h / 2.0 + self.scale.s(12.0), &meta, self.scale.fs(9.5), ui::fg2(pal), false);
            return;
        }

        // ── full layout ──
        let glyph_sz = self.scale.fs(28.0);
        let temp_sz  = self.scale.fs(24.0);
        let cond_sz  = self.scale.fs(11.0);
        let city_sz  = self.scale.fs(10.0);
        let stat_sz  = self.scale.fs(9.5);
        let sq_sz    = self.scale.s(30.0);

        // content block: glyph + temp/cond + city/date + hairline + stats
        let block_h = glyph_sz + self.scale.s(6.0) + cond_sz
                    + self.scale.s(8.0) + sq_sz + self.scale.s(10.0)
                    + self.scale.s(12.0) + stat_sz;
        let block_top = y + ((h - block_h) / 2.0).max(pad);

        // ── 1. weather glyph (centered) ──
        let gy = block_top;
        ui::text_c(v, ccx, gy, glyph, glyph_sz, pal.fg, true);

        // ── 2. temp + condition (centered pair on one line) ──
        let ty = gy + glyph_sz + self.scale.s(4.0);
        let temp_s = format!("{:.0}°C", wx.temp_c);
        let tw = temp_s.len() as f32 * temp_sz * 0.55;
        let cw = desc.len() as f32 * cond_sz * 0.55;
        let pair_gap = self.scale.s(8.0);
        let pair_w = tw + pair_gap + cw;
        let px = ccx - pair_w / 2.0;
        ui::text_hero(v, px, ty, &temp_s, temp_sz, pal.fg);
        ui::caption(v, px + tw + pair_gap, ty + temp_sz * 0.15, desc, cond_sz, ui::fg2(pal), false);

        // ── 3. city (left) + accent date square (right) ──
        let cy = ty + cond_sz + self.scale.s(8.0);
        if !city.is_empty() {
            ui::caption(v, x + pad, cy + sq_sz * 0.38, &city, city_sz, ui::fg2(pal), false);
        }
        let (month, day) = self.today_month_day();
        let sq_x = x + w - pad - sq_sz;
        let sq_r = self.scale.s(6.0);
        v.push(Cmd::Rect {
            x: sq_x, y: cy, w: sq_sz, h: sq_sz,
            r: sq_r, color: pal.acc,
        });
        ui::caption_c(v, sq_x + sq_sz / 2.0, cy + self.scale.s(6.0),
            &month, self.scale.fs(7.5), pal.sfg, false);
        ui::text_c_hero(v, sq_x + sq_sz / 2.0, cy + self.scale.s(17.0),
            format!("{day}"), self.scale.fs(13.0), pal.sfg);

        // ── 4. hairline divider ──
        let dy = cy + sq_sz + self.scale.s(6.0);
        v.push(Cmd::Rect {
            x: x + pad, y: dy, w: w - pad * 2.0, h: 1.0,
            r: 0.0, color: ui::hairline(pal),
        });

        // ── 5. stats row: wind · humidity · sunshine ──
        let sy = dy + self.scale.s(10.0);
        let icon_w = stat_sz * 1.2;
        let wind_s  = format!("{:.0} km/h", wx.wind_kmh);
        let hum_s   = wx.humidity.map(|h| format!("{:.0}%", h.round()));
        let sun_s   = sunshine.map(|s| format!("{s}%"));

        let avail = w - pad * 2.0;
        let zone  = avail / 3.0;

        // wind (left third)
        {
            let item_w = icon_w + wind_s.len() as f32 * stat_sz * 0.5;
            let sx = x + pad + (zone - item_w) / 2.0;
            ui::text(v, sx, sy, ICON_SPEED_FILL, stat_sz, ui::fg2(pal), true);
            ui::caption(v, sx + icon_w, sy + self.scale.s(1.0), &wind_s, stat_sz, pal.fg, false);
        }

        // humidity (center third)
        if let Some(ref hum) = hum_s {
            let item_w = icon_w + hum.len() as f32 * stat_sz * 0.5;
            let sx = x + pad + zone + (zone - item_w) / 2.0;
            ui::text(v, sx, sy, ICON_DROP, stat_sz, ui::fg2(pal), true);
            ui::caption(v, sx + icon_w, sy + self.scale.s(1.0), hum, stat_sz, pal.fg, false);
        }

        // sunshine (right third)
        if let Some(ref sun) = sun_s {
            let item_w = icon_w + sun.len() as f32 * stat_sz * 0.5;
            let sx = x + pad + zone * 2.0 + (zone - item_w) / 2.0;
            ui::text(v, sx, sy, ICON_SUNNY, stat_sz, ui::fg2(pal), true);
            ui::caption(v, sx + icon_w, sy + self.scale.s(1.0), sun, stat_sz, pal.fg, false);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_shell() -> Shell {
        let cfg: Config = toml::from_str(crate::config::DEFAULT_SHELL_TOML).expect("default config parses");
        let mut shell = Shell::new(cfg);
        shell.mode = crate::shell::Mode::Expanded;
        shell.dash_enabled = true;
        let wmsg = crate::weather::WeatherMsg {
            temp_c: 15.8,
            code: 0,
            wind_kmh: 8.0,
            humidity: Some(51.0),
            feels_like: Some(16.0),
            precip_mm: Some(0.2),
            city: Some("Home".to_string()),
            daily: (0..7)
                .map(|i| crate::weather::DailyFc {
                    code: if i % 2 == 0 { 0 } else { 61 },
                    tmax_c: 14.0 + i as f32,
                    tmin_c: 8.0 + i as f32,
                    precip_prob: 30.0 + i as f32,
                    day_offset: i as u8,
                })
                .collect(),
        };
        shell.weather = Some(wmsg);
        shell
    }

    fn place_and_layout(shell: &mut Shell, w: f32, h: f32) {
        shell.dash_layout.push(CardLayout { card: DashCard::WeatherV2, x: 0, y: 0, w: 10, h: 6 });
        shell.refresh_pack();
        shell.layout(w, h);
    }

    fn frame_pal(shell: &Shell) -> crate::ui::Pal {
        crate::ui::Pal { fg: shell.sv_fg, bg: shell.sv_bg, acc: shell.sv_acc, sfg: shell.sv_sfg }
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
    fn v2_card_full_layout() {
        let mut shell = sample_shell();
        place_and_layout(&mut shell, 500.0, 260.0);
        let v = shell.layout(500.0, 260.0);
        let t = texts(&v);
        assert!(t.iter().any(|x| x.contains("16°C")), "temp rendered");
        assert!(t.iter().any(|x| x.contains("Clear")), "condition rendered");
        assert!(t.iter().any(|x| x.contains("Home")), "city rendered");
        assert!(t.iter().any(|x| x.contains("8 km/h")), "wind rendered");
        assert!(t.iter().any(|x| x.contains("51%")), "humidity rendered");
        // sunshine = 100 − precip_prob[0] = 100 − 30 = 70
        assert!(t.iter().any(|x| x.contains("70%")), "sunshine rendered");
        let pal = frame_pal(&shell);
        assert!(v.iter().any(|c| matches!(c, Cmd::Rect { color, .. } if *color == pal.acc)),
            "accent date-square rect painted");
        assert!(shell.weather_rect.2 > 0.0, "weather rect registered for hit-testing");
        assert!(!v.iter().any(|c| matches!(c, Cmd::Rect { color, .. } if *color == 0xf6f7f9ff)),
            "no paper-white fill");
    }

    #[test]
    fn v2_card_compact_fallback() {
        let mut shell = sample_shell();
        place_and_layout(&mut shell, 500.0, 120.0);
        let t = texts(&shell.layout(500.0, 120.0));
        assert!(t.iter().any(|x| x.contains("16°C")), "compact temp shown");
        assert!(t.iter().any(|x| x.contains("Clear")), "compact condition shown");
    }
}
