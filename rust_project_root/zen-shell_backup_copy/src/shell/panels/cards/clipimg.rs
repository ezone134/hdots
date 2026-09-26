use super::super::*;

impl Shell {

    pub(crate) fn draw_clipimg_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        self.clipimg_card_rect = (x, y, w, h);
if self.card_show_title { ui::title(v, x + self.scale.s(14.0), y + self.scale.s(10.0), "Clipboard images", self.scale.fs(11.5), pal.fg); }
        let entries = self.clip_images.clone();
        let row_h = self.scale.s(28.0);
        let visible = self.clipimg_card_visible().min(entries.len());
        let start = self.clipimg_scroll.min(entries.len().saturating_sub(visible));
        for j in 0..visible {
            let i = start + j;
            let ry = y + self.scale.s(32.0) + j as f32 * row_h;
            let key = crate::shell::CLIPIMG_KEY_BASE + j as u32;
            let name: String = entries[i].chars().take(28).collect();
            let is_sel = self.clip_sel == i;
            let is_hov = self.hover_key == key;
            if is_sel || is_hov {
                v.push(Cmd::Rect { x: x + self.scale.s(10.0), y: ry - self.scale.s(2.0), w: w - self.scale.s(20.0), h: row_h - self.scale.s(4.0), r: self.scale.s(6.0), color: ui::hover_hl(&pal) });
            }
            let col = if is_sel { pal.acc } else if is_hov { ui::hover_fg(&pal) } else { pal.fg };
            v.push(Cmd::Rect { x: x + self.scale.s(14.0), y: ry - self.scale.s(1.0), w: self.scale.s(24.0), h: self.scale.s(22.0), r: self.scale.s(5.0), color: ui::hover(&pal) });
            v.push(Cmd::Image { x: x + self.scale.s(17.0), y: ry + self.scale.s(2.0), w: self.scale.s(18.0), h: self.scale.s(18.0), key: entries[i].clone() });
            ui::text(v, x + self.scale.s(46.0), ry + self.scale.s(5.0), &name, self.scale.fs(8.5), col, false);
            self.region(x + self.scale.s(10.0), ry - self.scale.s(2.0), w - self.scale.s(20.0), row_h - self.scale.s(4.0), key);
        }
        if entries.is_empty() {
            ui::text_c(v, x + w / 2.0, y + h / 2.0, "No images copied yet", self.scale.fs(9.5), pal.fg, false);
        }
    }

}
