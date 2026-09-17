use super::super::*;
use super::shared::*;

impl Shell {

    /// THERMAL — all hwmon zones in a small grid, colored by temperature.
    pub(crate) fn draw_thermal_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        let pad = self.scale.s(12.0);
        let hdr = self.scale.s(24.0);
        let n = self.thermal_zones.len();
        let meta = if n > 0 { format!("{n} zones") } else { String::new() };
        card_header(v, pal, x, y, w, pad, "Thermal zones", Some(ICON_THERMAL), Some((&meta, false)), self.scale, self.card_show_title, self.card_show_glyph);

        if self.thermal_zones.is_empty() {
            card_empty(v, &pal, x, y, w, h, "no /sys/class/thermal zones", self.scale);
            return;
        }
        let cols = 3usize;
        let cw = (w - pad * 2.0) / cols as f32;
        let row_h = self.scale.s(24.0);
        let y0 = y + hdr + self.scale.s(2.0);
        for (i, (zone, temp)) in self.thermal_zones.iter().enumerate() {
            let (cr, cc) = (i / cols, i % cols);
            let cx = x + pad + cc as f32 * cw;
            let cy = y0 + cr as f32 * row_h;
            if cy + row_h > y + h - 2.0 {
                break;
            }
            v.push(Cmd::Rect { x: cx + 1.0, y: cy + 1.0, w: cw - 2.0, h: row_h - 2.0, r: self.scale.s(5.0), color: ui::hover(&pal) });
            let tc = if *temp >= 80.0 { ui::DANGER } else if *temp >= 60.0 { ui::WARN } else { ui::fg2(&pal) };
            let zn: String = zone.chars().take(8).collect();
            ui::caption(v, cx + self.scale.s(4.0), cy + self.scale.s(3.0), &zn, self.scale.fs(7.5), ui::fg3(&pal), false);
            ui::caption(v, cx + self.scale.s(4.0), cy + self.scale.s(12.0), format!("{:.0}°", temp), self.scale.fs(9.0), tc, false);
        }
    }

}
