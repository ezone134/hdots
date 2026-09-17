use super::super::*;

impl Shell {

    /// RECENT — recently opened files from `~/.local/share/recently-used.xbel`
    /// (cached ~30 s); a row click opens the file.
    pub(crate) fn draw_recent_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        self.recent_rect = (x, y, w, h);
        if self.recent_loaded_at.elapsed().as_secs() > 30 {
            self.recent_files = Self::read_recent_files();
            self.recent_loaded_at = std::time::Instant::now();
        }
        let pad = self.scale.s(12.0);
        let hdr = self.scale.s(24.0);
if self.card_show_glyph { ui::text(v, x + pad, y + self.scale.s(8.0), ICON_RECENT, self.scale.fs(12.0), pal.acc, true); }
if self.card_show_title { ui::title(v, if self.card_show_glyph { x + pad + self.scale.s(17.0) } else { x + pad }, y + self.scale.s(9.0), "Recent files", self.scale.fs(12.0), pal.fg); }
        ui::text_r(v, x + w - pad, y + self.scale.s(9.0), format!("{} files", self.recent_files.len()), self.scale.fs(8.5), ui::fg3(&pal), false);

        if self.recent_files.is_empty() {
            ui::text_c(v, x + w / 2.0, y + h / 2.0 - self.scale.s(6.0), "no recent files tracked", self.scale.fs(9.0), ui::fg3(&pal), false);
            return;
        }
        let row_h = self.scale.s(19.0);
        let x0 = x + pad;
        let ww = (w - pad * 2.0).max(20.0);
        let visible = ((h - hdr - self.scale.s(4.0)) / row_h).floor().max(1.0) as usize;
        let len = self.recent_files.len();
        self.recent_scroll = self.recent_scroll.min(len.saturating_sub(visible));
        let top = self.recent_scroll;
        let y0 = y + hdr + self.scale.s(2.0);
        for j in 0..visible {
            let i = top + j;
            let Some((name, _path)) = self.recent_files.get(i) else { break };
            let rj = y0 + j as f32 * row_h;
            let key = crate::shell::RECENT_KEY_BASE + j as u32;
            let hov = self.hover_key == key;
            if hov {
                v.push(Cmd::Rect { x: x0, y: rj + 1.0, w: ww, h: row_h - 2.0, r: self.scale.s(4.0), color: ui::hover(&pal) });
            }
            let nm: String = name.chars().take(24).collect();
            ui::text(v, x0 + self.scale.s(6.0), rj + self.scale.s(1.5), &nm, self.scale.fs(9.5), if hov { pal.acc } else { pal.fg }, false);
            ui::text_r(v, x + w - pad, rj + self.scale.s(1.5), ICON_OPEN, self.scale.fs(8.0), if hov { pal.acc } else { ui::fg3(&pal) }, true);
            v.push(Cmd::Rect { x: x0, y: rj + row_h - 1.0, w: ww, h: 1.0, r: 0.5, color: ui::hover(&pal) });
            self.region(x0, rj, ww, row_h, key);
        }
    }

}
