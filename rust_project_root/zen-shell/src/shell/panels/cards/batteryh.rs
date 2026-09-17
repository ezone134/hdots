use super::super::*;
use super::shared::*;

impl Shell {

    /// HORIZONTAL-BATTERY card — a vector-crafted side-laid battery icon.
    /// Same two panes as the vertical card.
    pub(crate) fn draw_battery_h_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        self.battery_h_rect = (x, y, w, h);
        let Some(pct) = self.battery_frame(v, x, y, w, h, pal, "Battery") else {
            return;
        };
        if self.battery_h_pane == 0 {
            let iw = (w * 0.46).clamp(40.0, 220.0);
            push_battery_icon(v, pal, x + w / 2.0, y + self.scale.s(42.0) + (h - self.scale.s(70.0)) / 2.0, iw, (h * 0.36).clamp(16.0, 60.0), pct, false);
            let charge = if self.ac_online { format!(" {}", ICON_BOLT) } else { String::new() };
            ui::text_c(v, x + w / 2.0, y + h - self.scale.s(22.0), format!("{pct}%{charge}"), self.scale.fs(12.0), battery_level_color(&pal, pct), true);
        } else {
            self.battery_info_pane(v, x, y, w, h, pal, pct);
        }
        let hov = !self.dash_edit
            && self
                .cursor
                .map(|(px, py)| Self::in_rect(px, py, (x, y, w, h)))
                .unwrap_or(false);
        battery_dots(v, pal, x, y, w, h, self.battery_h_pane, self.scale.s(7.0), hov);
    }

}
