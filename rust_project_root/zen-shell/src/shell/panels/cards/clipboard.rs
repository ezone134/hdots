use super::super::*;

impl Shell {

    pub(crate) fn draw_clipboard_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        self.clip_card_rect = (x, y, w, h);
if !self.scene_owns_header && self.card_show_title { ui::title(v, x + self.scale.s(14.0), y + self.scale.s(10.0), "Clipboard", self.scale.fs(11.5), pal.fg); }
        let entries = self.clip_text.clone();
        let row_h = self.scale.s(22.0);
        let visible = self.clip_card_visible().min(entries.len());
        let start = self.clip_scroll.min(entries.len().saturating_sub(visible));
        for j in 0..visible {
            let i = start + j;
            let ry = y + self.scale.s(32.0) + j as f32 * row_h;
            let key = crate::shell::CLIP_KEY_BASE + j as u32;
            let label: String = entries[i].chars().take(32).collect();
            let is_sel = self.clip_sel == i;
            let is_hov = self.hover_key == key;
            if is_sel || is_hov {
                v.push(Cmd::Rect { x: x + self.scale.s(10.0), y: ry - self.scale.s(2.0), w: w - self.scale.s(20.0), h: self.scale.s(20.0), r: self.scale.s(6.0), color: ui::hover_hl(&pal) });
            }
            let col = if is_sel {
                pal.acc
            } else if is_hov {
                ui::hover_fg(&pal)
            } else {
                pal.fg
            };
            ui::text(v, x + self.scale.s(14.0), ry, &label, self.scale.fs(9.0), col, false);
            self.region(x + self.scale.s(10.0), ry - self.scale.s(2.0), w - self.scale.s(20.0), self.scale.s(20.0), key);
        }
        if entries.is_empty() {
            ui::text_c(v, x + w / 2.0, y + h / 2.0, "Empty", self.scale.fs(9.5), pal.fg, false);
        }
    }

}
