use super::super::*;

impl Shell {

    /// PRICES — crypto ticker rows (CoinGecko); a row click opens the asset's
    /// market page. Numeric rows, News-style.
    pub(crate) fn draw_ticker_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        self.ticker_rect = (x, y, w, h);
        let pad = self.scale.s(12.0);
        let hdr = self.scale.s(24.0);
if self.card_show_glyph { ui::text(v, x + pad, y + self.scale.s(8.0), ICON_TICKER, self.scale.fs(12.0), ui::fg2(&pal), true); }
if !self.scene_owns_header && self.card_show_title { ui::title(v, if self.card_show_glyph { x + pad + self.scale.s(17.0) } else { x + pad }, y + self.scale.s(9.0), "Prices", self.scale.fs(11.5), pal.fg); }
        ui::text_r(v, x + w - pad, y + self.scale.s(9.0), "crypto", self.scale.fs(8.5), ui::fg3(&pal), false);

        if self.ticker_items.is_empty() {
            let msg = if self.ticker_state == 1 { "loading prices…" } else { "no price data" };
            ui::text_c(v, x + w / 2.0, y + h / 2.0 - self.scale.s(6.0), msg, self.scale.fs(9.0), ui::fg3(&pal), false);
            return;
        }
        let row_h = self.scale.s(18.0);
        let x0 = x + pad;
        let ww = (w - pad * 2.0).max(20.0);
        let visible = ((h - hdr - self.scale.s(4.0)) / row_h).floor().max(1.0) as usize;
        let len = self.ticker_items.len();
        self.ticker_scroll = self.ticker_scroll.min(len.saturating_sub(visible));
        let top = self.ticker_scroll;
        let y0 = y + hdr + self.scale.s(2.0);
        for j in 0..visible {
            let i = top + j;
            let Some((sym, price, chg)) = self.ticker_items.get(i) else { break };
            let rj = y0 + j as f32 * row_h;
            let key = crate::shell::TICKER_KEY_BASE + j as u32;
            let hov = self.hover_key == key;
            if hov {
                v.push(Cmd::Rect { x: x0, y: rj + 1.0, w: ww, h: row_h - 2.0, r: self.scale.s(4.0), color: ui::hover(&pal) });
            }
            ui::text(v, x0 + self.scale.s(6.0), rj + self.scale.s(1.0), sym, self.scale.fs(9.5), pal.fg, false);
            ui::text(v, x0 + self.scale.s(6.0) + self.scale.s(56.0), rj + self.scale.s(1.0), price, self.scale.fs(9.5), if hov { pal.acc } else { ui::fg2(&pal) }, false);
            ui::text_r(v, x + w - pad, rj + self.scale.s(1.0), format!("{:+.1}%", chg), self.scale.fs(9.0), if *chg >= 0.0 { ui::OK } else { ui::DANGER }, false);
            v.push(Cmd::Rect { x: x0, y: rj + row_h - 1.0, w: ww, h: 1.0, r: 0.5, color: ui::hover(&pal) });
            self.region(x0, rj, ww, row_h, key);
        }
    }

}
