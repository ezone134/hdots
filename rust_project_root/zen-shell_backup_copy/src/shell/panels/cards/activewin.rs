use super::super::*;

impl Shell {

    pub(crate) fn draw_activewin_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        let cx = x + w / 2.0;
        if self.active_win_class.is_empty() {
            ui::text_c(v, cx, y + self.scale.s(14.0), ICON_ACTIVE, self.scale.fs(20.0), pal.fg, true);
            ui::text_c(v, cx, y + h / 2.0 + self.scale.s(10.0), "No window", self.scale.fs(10.0), pal.fg, false);
            return;
        }
        ui::text_c(v, cx, y + self.scale.s(14.0), &self.active_win_class, self.scale.fs(13.0), pal.fg, false);
        let title: String = self.active_win_title.chars().take(28).collect();
        ui::text_c(v, cx, y + self.scale.s(36.0), &title, self.scale.fs(9.5), pal.fg, false);
        // live per-app RAM (VmRSS of the active window's process) — a tiny
        // pill under the title so the card shows more than just the name
        if self.active_win_rss_kb > 0 {
            let kb = self.active_win_rss_kb;
            let (val, unit) = if kb >= 1_048_576 {
                (kb as f32 / 1_048_576.0, "GB")
            } else {
                (kb as f32 / 1024.0, "MB")
            };
            ui::text_c(v, cx, y + h - self.scale.s(16.0), format!("{} {:.1} {}", ICON_GAUGE_VALUE, val, unit), self.scale.fs(9.0), ui::fg2(&pal), false);
        }
    }

}
