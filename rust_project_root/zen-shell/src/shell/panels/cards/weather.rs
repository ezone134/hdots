use super::super::*;

impl Shell {
    /// WEATHER — its own card now (it used to be one line inside the clock).
    /// Two-pane card. Pane 0: current conditions. Pane 1: 7-day forecast
    /// (day, glyph, high/low, rain probability). Horizontal scroll switches
    /// panes; dots below the card mark the active one.

    /// Weekday label for a day offset ahead of today (0 = today, 1 = tomorrow…).
    /// `pub(crate)` so the declarative scene's forecast rows share the one
    /// rollover rule instead of restating it.
    pub(crate) fn weekday_name(&self, offset: i32) -> String {
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
        let cfg: Config =
            toml::from_str(crate::config::DEFAULT_SHELL_TOML).expect("default config parses");
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
        shell.dash_layout.push(CardLayout {
            card: DashCard::Weather,
            x: 0,
            y: 0,
            w: 10,
            h: 6,
        });
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
        assert!(
            temps.iter().any(|t| t.contains("22°C")),
            "current temp row rendered on pane 0"
        );
        assert!(
            shell.weather_rect.2 > 0.0,
            "weather rect registered for hit-testing"
        );
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
        assert!(
            temps.iter().any(|t| t.contains("Today")),
            "first forecast row labelled Today"
        );
        // The six non-today rows carry a weekday abbreviation. Don't hardcode
        // one (the old "Sun"): `weekday_name(0)` returns "Today", so on a day
        // that *is* Sunday the abbreviation only reappears at offset 7, which
        // is outside the 0..6 sample — the assertion would fail only on
        // Sundays. Check the label set instead, which is day-of-week agnostic.
        const WD: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
        assert!(
            WD.iter().any(|d| temps.iter().any(|t| t == d)),
            "the non-today forecast rows carry weekday abbreviations"
        );
        // pane flip never wraps: right on last pane stays, left returns to 0
        assert!(
            !Shell::card_pane_flip(&mut shell.weather_pane, 1),
            "right on last pane clamps"
        );
        assert_eq!(shell.weather_pane, 1);
        assert!(
            Shell::card_pane_flip(&mut shell.weather_pane, -1),
            "left goes back to pane 0"
        );
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
        assert!(
            temps.iter().any(|t| t.contains("22°C")),
            "no daily → falls back to current pane"
        );
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
