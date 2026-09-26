use super::super::*;

impl Shell {

    /// POWER H — five centered power buttons in a single row, sized to the
    /// card; same hold-to-confirm flow for the danger actions.
    pub(crate) fn draw_power_h_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        let hdr = self.scale.s(30.0);
if self.card_show_glyph { ui::text(v, x + self.scale.s(12.0), y + self.scale.s(8.0), ICON_BOLT, self.scale.fs(12.0), pal.acc, true); }
if !self.scene_owns_header && self.card_show_title { ui::title(v, if self.card_show_glyph { x + self.scale.s(30.0) } else { x + self.scale.s(12.0) }, y + self.scale.s(9.0), "Power", self.scale.fs(12.0), pal.fg); }

        let d = self.scale.s(32.0);
        let gap = self.scale.s(9.0);
        let total = 5.0 * d + 4.0 * gap;
        let body_h = (h - hdr).max(0.0);
        let cx0 = x + (w - total) / 2.0 + d / 2.0;
        let cy = y + hdr + ((body_h - d).max(0.0) * 0.5) + d / 2.0;
        for (i, &(glyph, danger)) in Self::power_card_items().iter().enumerate() {
            let cx = cx0 + i as f32 * (d + gap);
            self.power_btn(v, cx, cy, d, i as u32, glyph, danger, pal);
        }
    }

}
