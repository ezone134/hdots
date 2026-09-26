use super::super::*;

impl Shell {

    /// CURRENCY — static reference rates; a row click re-bases to that
    /// currency (1 USD ↔ …). No network — offline values with a note.
    pub(crate) fn draw_currency_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        const TBL: [(&str, &str, f64); 12] = [
            ("USD", "Dollar", 1.0),
            ("EUR", "Euro", 0.9200),
            ("GBP", "Pound", 0.7900),
            ("JPY", "Yen", 149.50),
            ("INR", "Rupee", 83.10),
            ("CNY", "Yuan", 7.2400),
            ("RUB", "Ruble", 92.50),
            ("AUD", "AU Dollar", 1.5200),
            ("CAD", "CA Dollar", 1.3700),
            ("KRW", "Won", 1347.00),
            ("XAU", "Gold oz", 0.000420),
            ("XBT", "BTC", 0.0000160),
        ];
        self.currency_rect = (x, y, w, h);
        let pad = self.scale.s(12.0);
        let hdr = self.scale.s(24.0);
if self.card_show_glyph { ui::text(v, x + pad, y + self.scale.s(8.0), ICON_MONEY, self.scale.fs(12.0), pal.acc, true); }
if !self.scene_owns_header && self.card_show_title { ui::title(v, if self.card_show_glyph { x + pad + self.scale.s(17.0) } else { x + pad }, y + self.scale.s(9.0), "Currency", self.scale.fs(12.0), pal.fg); }
        let base = self.currency_base.clone();
        let base_mult = TBL.iter().find(|(s, _, _)| *s == base.as_str()).map(|(_, _, m)| *m).unwrap_or(1.0);
        if self.currency_base != "USD" {
            ui::text_r(v, x + w - pad, y + self.scale.s(9.0), &self.currency_base, self.scale.fs(8.5), pal.acc, false);
        }

        let row_h = self.scale.s(20.0);
        let x0 = x + pad;
        let ww = (w - pad * 2.0).max(20.0);
        let visible = ((h - hdr - self.scale.s(18.0)) / row_h).floor().max(1.0) as usize;
        let len = TBL.len();
        self.currency_scroll = self.currency_scroll.min(len.saturating_sub(visible));
        let top = self.currency_scroll;
        let y0 = y + hdr + self.scale.s(2.0);
        for j in 0..visible {
            let i = top + j;
            let Some((sym, label, mult)) = TBL.get(i).copied() else { break };
            let rj = y0 + j as f32 * row_h;
            let key = crate::shell::CURRENCY_KEY_BASE + j as u32;
            let hov = self.hover_key == key;
            let active = sym == base.as_str();
            if active {
                v.push(Cmd::Rect { x: x0, y: rj + 1.0, w: ww, h: row_h - 2.0, r: self.scale.s(4.0), color: ui::acc_tint(&pal) });
            } else if hov {
                v.push(Cmd::Rect { x: x0, y: rj + 1.0, w: ww, h: row_h - 2.0, r: self.scale.s(4.0), color: ui::hover(&pal) });
            }
            let val = mult / base_mult;
            let vstr = if val >= 100.0 { format!("{:.0}", val) } else if val >= 10.0 { format!("{:.1}", val) } else { format!("{:.2}", val) };
            ui::text(v, x0 + self.scale.s(6.0), rj + self.scale.s(1.5), sym, self.scale.fs(10.0), if active { pal.acc } else { pal.fg }, false);
            ui::text(v, x0 + self.scale.s(6.0) + self.scale.s(30.0), rj + self.scale.s(2.0), label, self.scale.fs(8.0), ui::fg3(&pal), false);
            ui::text_r(v, x + w - pad, rj + self.scale.s(1.5), format!("1 {base} = {vstr} {sym}"), self.scale.fs(9.5), if active { pal.acc } else { pal.fg }, false);
            self.region(x0, rj, ww, row_h, key);
        }
        // footer note
        ui::text(v, x + pad, y + h - self.scale.s(12.0), "offline static rates", self.scale.fs(7.5), ui::fg3(&pal), false);
        self.currency_note = "static".to_string();
    }

}
