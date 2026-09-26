use super::super::*;
use super::shared::*;

impl Shell {

    /// BATTERY — two-pane card. Pane 0: one vertical + one horizontal charge
    /// gauge and a CPU-governor power-save toggle. Pane 1: extra battery info
    /// (power draw, time remaining, AC state). Horizontal scroll switches
    /// panes; the dots below the card mark the active one.
    /// VERTICAL-BATTERY card — a vector-crafted upright battery icon. Pane 0
    /// = icon + percent, pane 1 = extra info + power-save toggle. Horizontal
    /// scroll flips panes; dots below show the active pane.
/// VERTICAL-BATTERY card — a vector-crafted upright battery icon. Pane 0
    /// = icon + percent, pane 1 = extra info + power-save toggle. Horizontal
    /// scroll flips panes; dots below show the active pane.
    pub(crate) fn draw_battery_v_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        self.battery_v_rect = (x, y, w, h);
        let Some(pct) = self.battery_frame(v, x, y, w, h, pal, "Battery") else {
            return;
        };
        if self.battery_v_pane == 0 {
            // big vertical battery icon, centered in the body
            let aw = (w * 0.30).clamp(16.0, 70.0);
            let ah = (h - self.scale.s(44.0)).max(aw * 1.6);
            let iw = (aw * 2.0 / 3.0).max(12.0);
            push_battery_icon(v, pal, x + w / 2.0, y + self.scale.s(38.0) + (ah - iw * 1.7) / 2.0, iw, iw * 1.7, pct, true);
            let charge = if self.ac_online { format!(" {}", ICON_BOLT) } else { String::new() };
            ui::text_c(v, x + w / 2.0, y + h - self.scale.s(24.0), format!("{pct}%{charge}"), self.scale.fs(11.0), battery_level_color(&pal, pct), true);
        } else {
            self.battery_info_pane(v, x, y, w, h, pal, pct);
        }
        let hover = !self.dash_edit
            && self
                .cursor
                .map(|(px, py)| Self::in_rect(px, py, (x, y, w, h)))
                .unwrap_or(false);
        battery_dots(v, pal, x, y, w, h, self.battery_v_pane, self.scale.s(7.0), hover);
    }

}
