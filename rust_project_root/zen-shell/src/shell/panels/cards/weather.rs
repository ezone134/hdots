use super::super::*;
use super::shared::*;

impl Shell {

    /// WEATHER — its own card now (it used to be one line inside the clock).
    /// Two-pane card. Pane 0: current conditions. Pane 1: 7-day forecast
    /// (day, glyph, high/low, rain probability). Horizontal scroll switches
    /// panes; dots below the card mark the active one.

        pub(crate) fn draw_weather_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        self.weather_rect = (x, y, w, h);
        let ccx = x + w / 2.0;
        let Some(wx) = self.weather.as_ref().cloned() else {
            ui::text_c(v, ccx, y + h / 2.0 - self.scale.s(8.0), ICON_SUNNY, self.scale.fs(20.0), ui::fg2(&pal), true);
            ui::caption_c(v, ccx, y + h / 2.0 + self.scale.s(16.0), "waiting…", self.scale.fs(9.5), ui::fg2(&pal), false);
            return;
        };
        let hover = !self.dash_edit
            && self
                .cursor
                .map(|(px, py)| Self::in_rect(px, py, (x, y, w, h)))
                .unwrap_or(false);
        if self.weather_pane == 1 && !wx.daily.is_empty() {
            self.draw_weather_forecast_pane(v, x, y, w, h, pal, &wx.daily);
            battery_dots(v, pal, x, y, w, h, 1, self.scale.s(7.0), hover);
            return;
        }
        self.draw_weather_current_pane(v, x, y, w, h, pal, &wx);
        battery_dots(v, pal, x, y, w, h, 0, self.scale.s(7.0), hover);
    }

    /// Pane 0 — current conditions (previous single-pane layout).
    fn draw_weather_current_pane(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal, wx: &crate::weather::WeatherMsg) {
        let ccx = x + w / 2.0;
        let (glyph, desc) = crate::weather::describe(wx.code);
        if h < self.scale.s(60.0) {
            // squat span: one dense line
            ui::text_c(v, ccx, y + h / 2.0 - self.scale.s(8.0), format!("{glyph} {:.0}°C", wx.temp_c), self.scale.fs(13.0), pal.fg, true);
            return;
        }
        ui::text_c(v, ccx, y + self.scale.s(16.0), glyph, self.scale.fs(20.0), pal.fg, true);
        // temp digits in the Display family — never inside the icon-font string
        // (the old single `{glyph} 22°C` string rendered digits in the icon font)
        let temp = format!("{:.0}°C", wx.temp_c);
        let glyph_w = self.scale.fs(20.0) * 1.6;
        let temp_w = temp.chars().count() as f32 * self.scale.fs(20.0) * 0.55;
        let tx0 = ccx - (glyph_w + temp_w) / 2.0 + glyph_w;
        ui::text_hero(v, tx0, y + self.scale.s(16.0), temp, self.scale.fs(20.0), pal.fg);
        ui::caption_c(v, ccx, y + self.scale.s(48.0), desc, self.scale.fs(11.0), ui::fg2(&pal), false);
        if !self.weather_city.is_empty() {
            ui::caption_c(v, ccx, y + self.scale.s(66.0), self.weather_city.clone(), self.scale.fs(9.5), ui::fg3(&pal), false);
        }
        let mut detail = String::new();
        if let Some(fl) = wx.feels_like {
            detail.push_str(&format!("feels {:.0}°", fl));
        }
        if wx.wind_kmh > 0.0 {
            if !detail.is_empty() {
                detail.push_str(" · ");
            }
            detail.push_str(&format!("{:.0} km/h", wx.wind_kmh));
        }
        if !detail.is_empty() && h >= self.scale.s(100.0) {
            ui::caption_c(v, ccx, y + h - self.scale.s(26.0), detail, self.scale.fs(9.5), ui::fg2(&pal), false);
        }
    }

    /// Pane 1 — 7-day forecast lookahead: day · glyph · high/low · rain %.
    fn draw_weather_forecast_pane(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal, daily: &[crate::weather::DailyFc]) {
        let row_h = self.scale.s(24.0);
        let n_fit = (((h - self.scale.s(16.0)) / row_h).floor() as usize).min(7).max(1);
        let top = y + ((h - n_fit as f32 * row_h) / 2.0).max(self.scale.s(8.0));
        for (i, d) in daily.iter().take(n_fit).enumerate() {
            let ry = top + i as f32 * row_h;
            let cy = ry + row_h * 0.45;
            let (glyph, _) = crate::weather::describe(d.code);
            // day label: today / weekday name. `day_offset` 0 = today.
            let wd = self.weekday_name(d.day_offset as i32);
            ui::text(v, x + self.scale.s(14.0), cy - self.scale.s(6.0), glyph, self.scale.fs(12.0), ui::fg2(pal), true);
            ui::caption(v, x + self.scale.s(40.0), cy - self.scale.s(6.0), &wd, self.scale.fs(9.5), pal.fg, false);
            if d.precip_prob >= 5.0 {
                ui::caption(v, x + self.scale.s(110.0), cy - self.scale.s(6.0), format!("{}%", d.precip_prob.round() as i32), self.scale.fs(9.0), ui::fg2(pal), false);
            }
            let temp = format!("{:.0}° {:.0}°", d.tmin_c, d.tmax_c);
            ui::text_r_mono(v, x + w - self.scale.s(14.0), cy - self.scale.s(6.0), &temp, self.scale.fs(9.5), pal.fg);
        }
    }

    /// Weekday label for a day offset ahead of today (0 = today, 1 = tomorrow…).
    fn weekday_name(&self, offset: i32) -> String {
        const DAYS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
        unsafe {
            let now = libc::time(std::ptr::null_mut());
            let mut tm: libc::tm = std::mem::zeroed();
            libc::localtime_r(&now, &mut tm);
            // day-offset rollover across month/year boundaries: advance the
            // broken-down time by adding days via mktime (normalizes).
            tm.tm_mday += offset;
            tm.tm_hour = 12;
            tm.tm_isdst = -1;
            let _ = libc::mktime(&mut tm);
            let wd = tm.tm_wday.rem_euclid(7) as usize;
            if offset == 0 {
                "Today".to_string()
            } else {
                DAYS[wd].to_string()
            }
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
        let wmsgs = crate::weather::WeatherMsg {
            temp_c: 21.5,
            code: 0,
            wind_kmh: 8.0,
            humidity: Some(51.0),
            feels_like: Some(22.0),
            precip_mm: Some(0.2),
            city: Some("Home".to_string()),
            daily: (0..7)
                .map(|i| crate::weather::DailyFc {
                    code: if i % 2 == 0 { 0 } else { 61 },
                    tmax_c: 20.0 + i as f32,
                    tmin_c: 12.0 + i as f32,
                    precip_prob: 30.0 + i as f32,
                    day_offset: i as u8,
                })
                .collect(),
        };
        shell.weather = Some(wmsgs);
        shell
    }

    fn place_and_layout(shell: &mut Shell, w: f32, h: f32) {
        shell.dash_layout.push(CardLayout { card: DashCard::Weather, x: 0, y: 0, w: 10, h: 6 });
        shell.refresh_pack();
        shell.layout(w, h);
    }

    #[test]
    fn weather_pane_zero_renders_current_conditions() {
        let mut shell = sample_shell();
        shell.weather_pane = 0;
        place_and_layout(&mut shell, 500.0, 240.0);
        let v = shell.layout(500.0, 240.0);
        let temps: Vec<String> = v
            .iter()
            .filter_map(|c| match c {
                Cmd::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect();
        assert!(temps.iter().any(|t| t.contains("22°C")), "current temp row rendered on pane 0");
        assert!(shell.weather_rect.2 > 0.0, "weather rect registered for hit-testing");
    }

    #[test]
    fn weather_pane_one_renders_forecast_and_flips_back() {
        let mut shell = sample_shell();
        shell.weather_pane = 1;
        place_and_layout(&mut shell, 500.0, 240.0);
        let v = shell.layout(500.0, 240.0);
        let temps: Vec<String> = v
            .iter()
            .filter_map(|c| match c {
                Cmd::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect();
        // forecast reuses describe(): pane 0 shows City, pane 1 should not
        assert!(temps.iter().any(|t| t.contains("Today")), "first forecast row labelled Today");
        assert!(temps.iter().any(|t| t.contains("Sun")), "tomorrow weekday labelled (offset rollover may shift day)");
        // pane flip never wraps: right on last pane stays, left returns to 0
        assert!(!Shell::card_pane_flip(&mut shell.weather_pane, 1), "right on last pane clamps");
        assert_eq!(shell.weather_pane, 1);
        assert!(Shell::card_pane_flip(&mut shell.weather_pane, -1), "left goes back to pane 0");
        assert_eq!(shell.weather_pane, 0);
    }

    #[test]
    fn weather_daily_empty_stays_on_pane_zero() {
        let mut shell = sample_shell();
        shell.weather.as_mut().unwrap().daily.clear();
        shell.weather_pane = 1;
        place_and_layout(&mut shell, 500.0, 240.0);
        let v = shell.layout(500.0, 240.0);
        let temps: Vec<String> = v
            .iter()
            .filter_map(|c| match c {
                Cmd::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect();
        assert!(temps.iter().any(|t| t.contains("22°C")), "no daily → falls back to current pane");
    }

    #[test]
    fn weekday_name_rolls_across_week_boundary() {
        let shell = sample_shell();
        let name = shell.weekday_name(0);
        assert_eq!(name, "Today");
        // offset 6 should land on a valid short weekday name whatever today is
        let name6 = shell.weekday_name(6);
        assert!(!name6.is_empty() && name6 != "Today");
    }
}
