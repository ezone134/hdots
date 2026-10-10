use super::super::*;

impl Shell {

        pub(crate) fn draw_gauges_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        let ring_r = if h >= self.scale.s(116.0) { self.scale.s(30.0) } else { self.scale.s(22.0) };
        let dot_d = if h >= self.scale.s(116.0) { self.scale.s(5.0) } else { self.scale.s(4.0) };
        let dim = mix(ui::hover(&pal), pal.fg, 0.08);
        let cy = y + h / 2.0;
        // (label, root-prefix, pct, ring color, used_gb, total_gb)
        // Memory = accent (live data), Disk = muted OK green — was legacy GREEN
        let gauges: [(&str, &str, i32, u32, f32, f32); 2] = [
            ("Memory", "", self.mem_pct, pal.acc, self.mem_used_gb, self.mem_total_gb),
            ("Disk", "/", self.disk_pct, ui::OK, self.disk_used_gb, self.disk_total_gb),
        ];
        let two = w >= self.scale.s(170.0);
        for (idx, &(label, root, pct, col, used, total)) in gauges.iter().enumerate() {
            if idx == 1 && !two {
                break;
            }
            let cx = if two { x + w * (if idx == 0 { 0.27 } else { 0.73 }) } else { x + w / 2.0 };
            let lit = ((pct as f32 / 100.0) * 36.0).round() as i32;
            Self::ring_beads(v, cx, cy, ring_r, dot_d, lit, col, dim);
            let fs_pct = if ring_r >= self.scale.s(30.0) { self.scale.fs(13.0) } else { self.scale.fs(11.0) };
            // inside the gauge: percentage only — Display hero voice
            ui::text_c_hero(v, cx, cy - self.scale.s(8.0), format!("{}%", pct), fs_pct, pal.fg);
            // below the gauge: label, then used/total GB ("/ 3G/100G" on disk)
            if h >= ring_r * 2.0 + self.scale.s(50.0) {
                ui::caption_c(v, cx, cy + ring_r + self.scale.s(15.0), label, self.scale.fs(10.0), pal.fg, false);
                let gb = if total > 0.0 {
                    format!("{} {:.0}G/{:.0}G", root, used, total).trim_start().to_string()
                } else {
                    format!("{root} {pct}%").trim_start().to_string()
                };
                ui::text_c_mono(v, cx, cy + ring_r + self.scale.s(28.0), gb, self.scale.fs(8.0), ui::fg2(&pal));
            }
        }
    }

}
