use super::super::*;

impl Shell {

    pub(crate) fn draw_kblayout_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        let cx = x + w / 2.0;
        let cy = y + h / 2.0;
        let layout = if self.kb_layout.is_empty() { "Unknown".to_string() } else { self.kb_layout.clone() };
        // icon quiet fg2 (accent is for live data); layout label in the Display
        // family — it's the card's hero figure
        ui::text_c(v, cx, cy - self.scale.s(12.0), ICON_KEYBOARD, self.scale.fs(22.0), ui::fg2(&pal), true);
        ui::text_c_hero(v, cx, cy + self.scale.s(14.0), &layout, self.scale.fs(14.0), pal.fg);
    }

}
