use super::super::*;

impl Shell {

    /// COUNTDOWN — dated events sorted by soonest: one row per event
    /// (label · date · `Nd` / `today!` / `passed` chip). Past events gray
    /// out with a `passed` chip and stay until deleted (hover ✕). The
    /// composer row (`Name YYYY-MM-DD`, Enter) adds events — same
    /// keyboard path as the To-Do card.
    pub(crate) fn draw_countdown_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        self.countdown_rect = (x, y, w, h);
        let pad = self.scale.s(12.0);
        let hdr = self.scale.s(24.0);
        let n = self.countdown_events.len();
        let upcoming = self
            .countdown_events
            .iter()
            .filter(|(_, d)| self.countdown_days(d) >= 0)
            .count();
        let meta = if n > 0 { format!("{upcoming} upcoming") } else { String::new() };
        if !self.scene_owns_header && self.card_show_title {
            ui::title(v, x + pad, y + self.scale.s(10.0), "Countdown", self.scale.fs(11.5), pal.fg);
        }
        ui::text_r(v, x + w - pad, y + self.scale.s(12.0), &meta, self.scale.fs(9.0), pal.fg, false);

        // composer row (focused → it owns the body)
        let row_h = self.scale.s(24.0);
        let cy0 = y + hdr + self.scale.s(2.0);
        let composing = self.countdown_input.is_some();
        let buf = self.countdown_input.clone().unwrap_or_default();
        v.push(Cmd::Rect { x: x + pad, y: cy0, w: w - pad * 2.0, h: row_h - 2.0, r: self.scale.s(6.0), color: if composing { ui::hover_hl(&pal) } else { ui::hover(&pal) } });
        let shown = if buf.is_empty() { "add event — name 2026-12-31" } else { &buf };
        ui::text(v, x + pad + self.scale.s(8.0), cy0 + self.scale.s(5.0), shown, self.scale.fs(8.5), if buf.is_empty() { ui::fg3(&pal) } else { pal.fg }, false);
        self.region(x + pad, cy0, w - pad * 2.0, row_h, COUNTDOWN_INPUT_KEY);

        // event rows (snapshotted so the ✕ region registration can borrow mutably);
        // the scene owns the list when countdown.ron declares a `Rows`
        let y0 = cy0 + row_h;
        if !self.scene_owns_rows {
        let max_rows = (((y + h - 2.0) - y0) / row_h).floor() as usize;
        let rows: Vec<(usize, String, String)> = self
            .countdown_events
            .iter()
            .enumerate()
            .map(|(i, (l, d))| (i, l.clone(), d.clone()))
            .collect();
        for (i, label, date) in rows.iter() {
            let i = *i;
            if i >= max_rows {
                break;
            }
            let ry = y0 + i as f32 * row_h;
            let days = self.countdown_days(date);
            let passed = days < 0;
            let is_today = days == 0;
            let col = if passed { ui::fg3(&pal) } else { pal.fg };
            let name: String = label.chars().take(((w - pad * 2.0 - self.scale.s(120.0)) / self.scale.s(5.9)).max(8.0) as usize).collect();
            ui::text(v, x + pad, ry + self.scale.s(3.0), &name, self.scale.fs(9.5), col, false);
            ui::caption(v, x + pad, ry + self.scale.s(13.0), date, self.scale.fs(7.5), ui::fg3(&pal), false);
            // days chip (right-aligned)
            let chip = if is_today {
                "today!".to_string()
            } else if passed {
                "passed".to_string()
            } else if days == 1 {
                "1d".to_string()
            } else {
                format!("{days}d")
            };
            let chip_c = if is_today {
                pal.acc
            } else if passed {
                ui::fg3(&pal)
            } else if days <= 7 {
                ui::WARN
            } else {
                ui::fg2(&pal)
            };
            let cw = chip.chars().count() as f32 * 0.62 * self.scale.fs(8.5) + self.scale.s(10.0);
            v.push(Cmd::Rect {
                x: x + w - pad - cw,
                y: ry + self.scale.s(3.0),
                w: cw,
                h: self.scale.s(16.0),
                r: self.scale.s(8.0),
                color: if is_today { (pal.acc & 0xFFFF_FF00) | 0x28 } else { ui::hover(&pal) },
            });
            ui::text_c(v, x + w - pad - cw / 2.0, ry + self.scale.s(6.0), &chip, self.scale.fs(8.5), chip_c, false);
            // hover ✕
            let del_key = COUNTDOWN_DEL_BASE + i as u32;
            if self.hover_key == del_key {
                ui::text(v, x + w - pad - cw - self.scale.s(20.0), ry + self.scale.s(4.0), ICON_CLOSE, self.scale.fs(9.0), RED, true);
                self.region(x + w - pad - cw - self.scale.s(22.0), ry, self.scale.s(20.0), row_h, del_key);
            }
        }
        }
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    fn shell() -> Shell {
        let cfg: crate::config::Config =
            toml::from_str(crate::config::DEFAULT_SHELL_TOML).expect("default config parses");
        Shell::new(cfg)
    }

    fn pal() -> Pal {
        Pal { fg: 0xe8e6e3ff, bg: 0x141414ff, acc: 0x4fc2ffff, sfg: 0x101010ff }
    }

    #[test]
    fn countdown_day_math_and_sort() {
        let mut s = shell();
        let today = Shell::water_today();
        // past / today / future
        let past_y = (today[0..4].parse::<i32>().unwrap() - 1).to_string();
        s.countdown_events = vec![
            ("far".into(), format!("{past_y}-01-01")),
            ("today".into(), today.clone()),
            ("soon".into(), format!("{}-12-31", today[0..4].to_string())),
        ];
        s.sort_countdown();
        assert_eq!(s.countdown_events[0].0, "far", "past sorts first");
        assert!(s.countdown_days(&s.countdown_events[0].1) < 0, "past is negative");
        assert_eq!(s.countdown_days(&today), 0, "today is 0");
        assert!(s.countdown_days(&s.countdown_events[2].1) > 0, "future is positive");
    }

    #[test]
    fn countdown_commit_parses_name_date() {
        let mut s = shell();
        s.countdown_input = Some("Trip 2030-05-05".into());
        s.countdown_add();
        assert_eq!(s.countdown_events.len(), 1);
        assert_eq!(s.countdown_events[0].0, "Trip");
        assert_eq!(s.countdown_events[0].1, "2030-05-05");
        // malformed input (no date) is rejected
        s.countdown_input = Some("no date here".into());
        s.countdown_add();
        assert_eq!(s.countdown_events.len(), 1);
    }

    #[test]
    fn countdown_card_draws_rows_and_chips() {
        let mut s = shell();
        let today = Shell::water_today();
        s.countdown_events = vec![("Release".into(), today.clone())];
        let p = pal();
        let mut v: Vec<Cmd> = Vec::new();
        s.draw_countdown_card(&mut v, 0.0, 0.0, 260.0, 120.0, &p);
        assert!(v.iter().any(|c| matches!(c, Cmd::Text { text, .. } if text == "today!")), "today chip drawn");
    }
}
