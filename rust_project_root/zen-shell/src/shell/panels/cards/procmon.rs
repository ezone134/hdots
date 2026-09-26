use super::super::*;

impl Shell {

    pub(crate) fn draw_procmon_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
if !self.scene_owns_header && self.card_show_title { ui::title(v, x + self.scale.s(14.0), y + self.scale.s(10.0), "Processes", self.scale.fs(11.5), pal.fg); }
        ui::text_r_mono(v, x + w - self.scale.s(14.0), y + self.scale.s(12.0), format!("CPU {}%", self.cpu), self.scale.fs(9.0), pal.fg);
        let procs = self.top_procs.clone();
        let visible = (((h - self.scale.s(36.0)) / self.scale.s(22.0)).floor() as usize).min(procs.len());
        for i in 0..visible {
            let (ref name, _ticks, _) = procs[i];
            let ry = y + self.scale.s(32.0) + i as f32 * self.scale.s(22.0);
            let label: String = name.chars().take(16).collect();
            ui::text(v, x + self.scale.s(14.0), ry, &label, self.scale.fs(9.0), pal.fg, false);
        }
    }

}
