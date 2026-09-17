use super::super::*;

impl Shell {

    /// EXPENSES — quick money log: preset amount chips (1 / 5 / 10 / 20 /
    /// 50 / 100 + custom via composer) log a spend under the "misc"
    /// category instantly; the header shows the month-to-date total and
    /// the body shows per-category bars (top 3). Entries persist to
    /// `expenses.txt` as cents; nothing leaves the machine.
    pub(crate) fn draw_expenses_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        self.expenses_rect = (x, y, w, h);
        let pad = self.scale.s(12.0);
        let hdr = self.scale.s(24.0);
        let (total, per) = self.expense_month_summary();
        let meta = format!("MTD {total:.0}");
        if !self.scene_owns_header && self.card_show_title {
            ui::title(v, x + pad, y + self.scale.s(10.0), "Expenses", self.scale.fs(11.5), pal.fg);
        }
        ui::text_r_mono(v, x + w - pad, y + self.scale.s(12.0), &meta, self.scale.fs(9.0), pal.acc);

        // quick-amount chips
        let chips = [1u32, 5, 10, 20, 50, 100];
        let gap = self.scale.s(6.0);
        let cw = (w - pad * 2.0 - gap * (chips.len() as f32 - 1.0)) / chips.len() as f32;
        let ch = self.scale.s(26.0);
        let cy = y + hdr + self.scale.s(4.0);
        for (i, amount) in chips.iter().enumerate() {
            let cx = x + pad + i as f32 * (cw + gap);
            let key = EXPENSE_CHIP_BASE + i as u32;
            let hov = self.hover_key == key;
            v.push(Cmd::Rect { x: cx, y: cy, w: cw, h: ch, r: self.scale.s(6.0), color: if hov { ui::hover_hl(&pal) } else { ui::hover(&pal) } });
            ui::text_c(v, cx + cw / 2.0, cy + self.scale.s(6.0), format!("{amount}"), self.scale.fs(10.0), pal.fg, false);
            self.region(cx, cy, cw, ch, key);
        }
        // custom-amount composer chip + clear-month button on the same row
        let ry = cy + ch + gap;
        let composing = self.expense_input.is_some();
        let buf = self.expense_input.clone().unwrap_or_default();
        let comp_w = w - pad * 2.0 - self.scale.s(70.0) - gap;
        v.push(Cmd::Rect { x: x + pad, y: ry, w: comp_w, h: self.scale.s(22.0), r: self.scale.s(6.0), color: if composing { ui::hover_hl(&pal) } else { ui::hover(&pal) } });
        let shown = if buf.is_empty() { "custom — 12.50 food" } else { &buf };
        ui::text(v, x + pad + self.scale.s(8.0), ry + self.scale.s(5.0), shown, self.scale.fs(8.5), if buf.is_empty() { ui::fg3(&pal) } else { pal.fg }, false);
        self.region(x + pad, ry, comp_w, self.scale.s(22.0), EXPENSE_INPUT_KEY);
        // clear-month (danger, right of the composer)
        let cb_w = self.scale.s(70.0);
        let cb_x = x + w - pad - cb_w;
        let cb_hov = self.hover_key == EXPENSE_CLEAR_KEY;
        v.push(Cmd::Rect { x: cb_x, y: ry, w: cb_w, h: self.scale.s(22.0), r: self.scale.s(6.0), color: if cb_hov { mix(RED, ui::hover(&pal), 0.6) } else { ui::hover(&pal) } });
        ui::text_c(v, cb_x + cb_w / 2.0, ry + self.scale.s(5.0), "clear mo", self.scale.fs(8.5), if cb_hov { RED } else { ui::fg2(&pal) }, false);
        self.region(cb_x, ry, cb_w, self.scale.s(22.0), EXPENSE_CLEAR_KEY);

        // per-category bars (top 3)
        let by0 = ry + self.scale.s(30.0);
        let bar_w = w - pad * 2.0 - self.scale.s(46.0);
        let max = per.first().map(|(_, v)| *v).unwrap_or(0.0).max(1.0);
        for (i, (cat, val)) in per.iter().take(3).enumerate() {
            let by = by0 + i as f32 * self.scale.s(20.0);
            if by + self.scale.s(18.0) > y + h - 2.0 {
                break;
            }
            let name: String = cat.chars().take(7).collect();
            ui::caption(v, x + pad, by, &name, self.scale.fs(8.0), ui::fg2(&pal), false);
            ui::bar(v, x + pad, by + self.scale.s(11.0), bar_w, self.scale.s(4.0), (val / max).clamp(0.0, 1.0), ui::INFO, ui::hover(&pal));
            ui::text_r_mono(v, x + w - pad, by, format!("{val:.0}"), self.scale.fs(8.0), pal.fg);
        }
        if per.is_empty() && by0 < y + h - 2.0 {
            ui::caption_c(v, x + w / 2.0, by0 + self.scale.s(6.0), "nothing logged this month", self.scale.fs(8.5), ui::fg3(&pal), false);
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
    fn expense_commit_parses_amount_category() {
        let mut s = shell();
        s.expense_input = Some("12.50 food".into());
        s.expense_commit();
        assert_eq!(s.expenses.len(), 1);
        assert_eq!(s.expenses[0].0, 1250, "12.50 → 1250 cents");
        assert_eq!(s.expenses[0].1, "food");
        // bare amount → misc
        s.expense_input = Some("7".into());
        s.expense_commit();
        assert_eq!(s.expenses[1].1, "misc");
        assert_eq!(s.expenses[1].0, 700);
        // comma decimal accepted
        s.expense_input = Some("3,25".into());
        s.expense_commit();
        assert_eq!(s.expenses[2].0, 325);
    }

    #[test]
    fn month_summary_totals_and_ranks() {
        let mut s = shell();
        let today = Shell::water_today();
        s.expenses = vec![
            (5000, "rent".into(), today.clone()),
            (1200, "food".into(), today.clone()),
            (3000, "food".into(), today.clone()),
            (9900, "old".into(), "2020-01-15".into()), // other month — excluded
        ];
        let (total, per) = s.expense_month_summary();
        assert!((total - 92.0).abs() < 0.01, "MTD total = 50+12+30, got {total}");
        assert_eq!(per[0].0, "rent", "rent (50) must rank above food (42)");
        assert_eq!(per[1].0, "food");
    }

    #[test]
    fn expenses_card_draws_and_registers_chips() {
        let mut s = shell();
        s.mode = crate::shell::Mode::Expanded;
        let p = pal();
        let mut v: Vec<Cmd> = Vec::new();
        s.draw_expenses_card(&mut v, 0.0, 0.0, 300.0, 120.0, &p);
        let keys: Vec<u32> = s.hover_regions.iter().map(|r| r.4).collect();
        assert!(keys.contains(&EXPENSE_CHIP_BASE), "chip 1 registered");
        assert!(keys.contains(&EXPENSE_INPUT_KEY), "composer registered");
    }
}
