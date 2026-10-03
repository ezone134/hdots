use super::super::*;

impl Shell {

    pub(crate) fn draw_pkgupdates_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        let cx = x + w / 2.0;
        let cy = y + h / 2.0;
        let count = self.pkg_update_count;
        let glyph = if count > 0 { ICON_DOWN } else { ICON_CHECK };
        let col = if count > 0 { pal.acc } else { ui::OK };
        ui::text_c(v, cx, cy - self.scale.s(18.0), glyph, self.scale.fs(22.0), col, true);
        // the count is the hero figure — Display Medium voice
        ui::text_c_hero(v, cx, cy + self.scale.s(10.0), format!("{count}"), self.scale.fs(20.0), pal.fg);
        ui::caption_c(v, cx, cy + self.scale.s(34.0), if count > 0 { "updates" } else { "up to date" }, self.scale.fs(9.0), ui::fg2(&pal), false);
    }

}
