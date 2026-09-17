use super::super::*;

impl Shell {

    /// QUOTE — quote of the day with two pills: refresh (⟳) and save (→
    /// `$states/quotes`). Save flashes a green "saved ✓" and bumps the shelf.
    pub(crate) fn draw_quote_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        self.quote_rect = (x, y, w, h);
        let pad = self.scale.s(12.0);
        let hdr = self.scale.s(24.0);
if self.card_show_glyph { ui::text(v, x + pad, y + self.scale.s(8.0), ICON_QUOTE, self.scale.fs(12.0), pal.acc, true); }
if self.card_show_title { ui::title(v, if self.card_show_glyph { x + pad + self.scale.s(17.0) } else { x + pad }, y + self.scale.s(9.0), "Quote", self.scale.fs(12.0), pal.fg); }
        if self.quote_state == 2 && self.quote_saved_flash > 0 {
            ui::text_r(v, x + w - pad, y + self.scale.s(9.0), format!("{} saved", ICON_CHECK), self.scale.fs(8.5), 0x2ee6a8ff, false);
        } else {
            let (sl, sc) = match self.quote_state {
                1 => ("fetching…".to_string(), ui::fg3(&pal)),
                2 => (format!("{} saved", self.saved_quotes.len()), ui::fg3(&pal)),
                _ => ("—".to_string(), ui::fg3(&pal)),
            };
            ui::text_r(v, x + w - pad, y + self.scale.s(9.0), &sl, self.scale.fs(8.5), sc, false);
        }
        if self.quote_saved_flash > 0 {
            self.quote_saved_flash -= 1;
        }
        let has_quote = self.quote_state == 2 && !self.quote_text.is_empty();

        // body: wrapped quote text + author
        let body_y = y + hdr + self.scale.s(4.0);
        let body_h = (h - hdr - self.scale.s(40.0)).max(10.0);
        if self.quote_state == 1 {
            ui::text_c(v, x + w / 2.0, body_y + body_h / 2.0 - self.scale.s(6.0), "fetching a quote…", self.scale.fs(9.0), ui::fg3(&pal), false);
        } else if has_quote {
            let raw: String = self.quote_text.chars().take(136).collect();
            let mut lines: Vec<String> = Vec::new();
            let mut cur = String::new();
            for c in raw.chars() {
                cur.push(c);
                if cur.chars().count() >= 34 {
                    lines.push(std::mem::take(&mut cur));
                }
            }
            if !cur.is_empty() {
                lines.push(cur);
            }
            let line_h = self.scale.s(13.0);
            let start = body_y + (body_h - line_h * lines.len() as f32) / 2.0 - self.scale.s(4.0);
            for (li, l) in lines.iter().enumerate() {
                ui::text_c(v, x + w / 2.0, start + li as f32 * line_h, l, self.scale.fs(10.0), pal.fg, false);
            }
            if !self.quote_author.is_empty() {
                ui::text_c(v, x + w / 2.0, start + lines.len() as f32 * line_h + self.scale.s(2.0), format!("— {}", self.quote_author), self.scale.fs(8.5), ui::fg3(&pal), false);
            }
        } else {
            ui::text_c(v, x + w / 2.0, body_y + body_h / 2.0 - self.scale.s(6.0), "no quote yet", self.scale.fs(9.0), ui::fg3(&pal), false);
        }

        // two pills
        let pw = self.scale.s(88.0);
        let ph = self.scale.s(26.0);
        let gap = self.scale.s(12.0);
        let py = y + h - ph - self.scale.s(8.0);
        let total = pw * 2.0 + gap;
        let px = x + (w - total) / 2.0;
        let k_refresh = crate::shell::QUOTE_KEY_BASE;
        let k_save = crate::shell::QUOTE_SAVE_KEY;
        let hov_r = self.hover_key == k_refresh;
        let hov_s = self.hover_key == k_save;
        let save_ok = has_quote;
        v.push(Cmd::Rect { x: px, y: py, w: pw, h: ph, r: ph / 2.0, color: if hov_r { mix(pal.acc, pal.fg, 0.15) } else { ui::hover_hl(&pal) } });
        ui::text_c(v, px + pw / 2.0, py + ph / 2.0 - self.scale.s(5.0), format!("{}  new", ICON_RECYCLE), self.scale.fs(10.0), if hov_r { pal.acc } else { pal.fg }, false);
        self.region(px, py, pw, ph, k_refresh);
        v.push(Cmd::Rect { x: px + pw + gap, y: py, w: pw, h: ph, r: ph / 2.0, color: if save_ok { if hov_s { mix(0x2ee6a8ff, pal.fg, 0.15) } else { 0x2ee6a8ff } } else { ui::hover(&pal) } });
        ui::text_c(v, px + pw + gap + pw / 2.0, py + ph / 2.0 - self.scale.s(5.0), format!("{}  save", ICON_SAVE), self.scale.fs(10.0), if save_ok { 0xff141414 } else { ui::fg3(&pal) }, false);
        if save_ok {
            self.region(px + pw + gap, py, pw, ph, k_save);
        }
    }

}
