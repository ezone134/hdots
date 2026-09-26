use super::super::*;
use super::shared::*;

impl Shell {

    pub(crate) fn draw_mem_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        let hdr = self.scale.s(26.0);
        let pad = self.scale.s(14.0);
        self.card_head(v, pal, x, y, w, pad, "Memory", None, None);
        // main circular gauge (used % only) on the left, centered in its area
        let r = ((h - hdr - self.scale.s(16.0)) / 2.0).clamp(self.scale.s(12.0), self.scale.s(34.0));
        let col_x = x + 2.0 * r + pad + self.scale.s(10.0);
        let cx = x + r + pad / 2.0 + self.scale.s(5.0);
        let cy = y + hdr + (h - hdr) / 2.0;
        let dot_d = self.scale.s(4.0);
        let dim = mix(ui::hover(&pal), pal.fg, 0.10);
        let lit = ((self.mem_pct as f32 / 100.0) * 36.0).round() as i32;
        Self::ring_beads(v, cx, cy, r, dot_d, lit, pal.acc, dim);
        let fs_pct = if r >= self.scale.s(26.0) { self.scale.fs(15.0) } else { self.scale.fs(12.0) };
        ui::text_c_hero(v, cx, cy - self.scale.s(7.0), format!("{}%", self.mem_pct), fs_pct, pal.fg);
        // detail rows to the right: total on top, then available / cached / free
        let col_w = (x + w - pad) - col_x;
        if col_w > self.scale.s(70.0) {
            let rows: [(&'static str, f32); 4] = [
                ("Total", self.mem_total_gb),
                ("Available", self.mem_avail_gb),
                ("Cached", self.mem_cached_gb),
                ("Free", self.mem_free_gb),
            ];
            // center the stacked rows on the circle's centre
            let ry0 = cy - (rows.len() as f32 - 1.0) * self.scale.s(11.0);
            for (i, &(label, gb)) in rows.iter().enumerate() {
                let row_y = ry0 + i as f32 * self.scale.s(22.0);
                metric_row(v, pal, col_x, row_y, x + w - pad, label, &format!("{gb:.1}G"), pal.fg, self.scale);
            }
        }
        // swap bar below
        if self.swap_pct > 0 && y + h - self.scale.s(18.0) >= cy + r {
            let sy = y + h - self.scale.s(18.0);
            let sw = w - pad * 2.0;
            v.push(Cmd::Rect { x: x + pad, y: sy, w: sw, h: self.scale.s(5.0), r: self.scale.s(2.5), color: ui::hover(&pal) });
            v.push(Cmd::Rect { x: x + pad, y: sy, w: sw * self.swap_pct as f32 / 100.0, h: self.scale.s(5.0), r: self.scale.s(2.5), color: pal.acc });
        }
    }

}
