use super::super::*;
use super::shared::*;

impl Shell {

    /// CPU — load average (1/5/15 min), live clock frequency, package temp
    /// and usage %, with a usage bar under the header.
    pub(crate) fn draw_cpu_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        let hdr = self.scale.s(26.0);
        let pad = self.scale.s(14.0);
        let freq = self.cpu_freq_mhz;
        let meta = if freq >= 1000 {
            format!("{:.2} GHz", freq as f32 / 1000.0)
        } else if freq > 0 {
            format!("{freq} MHz")
        } else {
            String::new()
        };
        card_header(v, pal, x, y, w, pad, "CPU", None, Some((&meta, true)), self.scale, self.card_show_title, self.card_show_glyph);
        // usage % — Display hero, warn/crit ladder via the shared token helper
        let uc = ui::pct_color(self.cpu, 70, 90, pal.fg);
        // main circular gauge (used % only) on the left, centered in its area
        let r = ((h - hdr - self.scale.s(16.0)) / 2.0).clamp(self.scale.s(12.0), self.scale.s(34.0));
        let col_x = x + 2.0 * r + pad + self.scale.s(10.0);
        let cx = x + r + pad / 2.0 + self.scale.s(5.0);
        let cy = y + hdr + (h - hdr) / 2.0;
        let dot_d = self.scale.s(4.0);
        let dim = mix(ui::hover(&pal), pal.fg, 0.10);
        let lit = (self.cpu.clamp(0, 100) as f32 / 100.0 * 36.0).round() as i32;
        Self::ring_beads(v, cx, cy, r, dot_d, lit, uc, dim);
        let fs_pct = if r >= self.scale.s(26.0) { self.scale.fs(15.0) } else { self.scale.fs(12.0) };
        ui::text_c_hero(v, cx, cy - self.scale.s(7.0), format!("{}%", self.cpu), fs_pct, uc);
        // detail rows to the right: load / freq / temp
        let col_w = (x + w - pad) - col_x;
        if col_w > self.scale.s(70.0) {
            let freq = if meta.is_empty() { "—".to_string() } else { meta };
            let rows: [(&'static str, String, u32); 3] = [
                ("Load", format!("{:.2}/{:.2}/{:.2}", self.cpu_load.0, self.cpu_load.1, self.cpu_load.2), pal.fg),
                ("Freq", freq, pal.fg),
                ("Temp", if self.cpu_temp > 0.0 { format!("{:.0}°C", self.cpu_temp) } else { "—".to_string() },
                    if self.cpu_temp > 80.0 { ui::DANGER } else if self.cpu_temp > 65.0 { ui::WARN } else { pal.fg }),
            ];
            // center the stacked rows on the circle's centre
            let ry0 = cy - (rows.len() as f32 - 1.0) * self.scale.s(11.0);
            for (i, (label, val, color)) in rows.into_iter().enumerate() {
                let row_y = ry0 + i as f32 * self.scale.s(22.0);
                metric_row(v, pal, col_x, row_y, x + w - pad, label, &val, color, self.scale);
            }
        }
    }

}
