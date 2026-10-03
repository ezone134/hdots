use super::super::*;
use super::shared::*;

impl Shell {

    pub(crate) fn draw_disk_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        let pad = self.scale.s(14.0);
        let n = self.disk_parts.len();
        let meta = if n > 0 { format!("{n} mounts") } else { String::new() };
        card_header(v, pal, x, y, w, pad, "Disk", None, Some((&meta, false)), self.scale, self.card_show_title, self.card_show_glyph);
        let parts = self.disk_parts.clone();
        let row_h = self.scale.s(24.0);
        let visible = (((h - self.scale.s(36.0)) / row_h).floor() as usize).min(parts.len());
        for i in 0..visible {
            let (ref mount, pct, used_gb, total_gb) = parts[i];
            let ry = y + self.scale.s(32.0) + i as f32 * row_h;
            let bar_x = x + pad;
            let bar_w = w - pad * 2.0;
            // mount label in fg2 (secondary — the value is the data here),
            // usage mono right, warn/crit ladder through the shared helper
            ui::caption(v, bar_x, ry, mount, self.scale.fs(9.0), ui::fg2(&pal), false);
            ui::text_r_mono(v, x + w - pad, ry, format!("{used_gb:.0}/{total_gb:.0} GB {pct}%"), self.scale.fs(9.0), pal.fg);
            let bar_y = ry + self.scale.s(13.0);
            let col = ui::pct_color(pct, 75, 90, pal.acc);
            v.push(Cmd::Rect { x: bar_x, y: bar_y, w: bar_w, h: self.scale.s(6.0), r: self.scale.s(3.0), color: ui::hover(&pal) });
            v.push(Cmd::Rect { x: bar_x, y: bar_y, w: bar_w * pct as f32 / 100.0, h: self.scale.s(6.0), r: self.scale.s(3.0), color: col });
        }
        if parts.is_empty() {
            card_empty(v, &pal, x, y, w, h, "No partitions", self.scale);
        }
    }

}
