use super::super::*;

impl Shell {

    /// Disk I/O — read (blue) + write (accent) speed on one history graph,
    /// normalised against the read peak as 100% (same language as network).
    pub(crate) fn draw_diskio_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        // read = one muted info blue, write = accent (same language as network)
        let r_col = ui::INFO;
        let w_col = pal.acc;
        let rx = x + w - self.scale.s(14.0);
        let dot = self.scale.s(7.0);
        let dx = rx - self.scale.s(44.0);
        v.push(Cmd::Rect { x: dx, y: y + self.scale.s(16.0), w: dot, h: dot, r: self.scale.s(2.0), color: r_col });
        ui::text_r(v, rx, y + self.scale.s(14.0), "R", self.scale.fs(9.0), pal.fg, false);
        v.push(Cmd::Rect { x: dx, y: y + self.scale.s(34.0), w: dot, h: dot, r: self.scale.s(2.0), color: w_col });
        ui::text_r(v, rx, y + self.scale.s(32.0), "W", self.scale.fs(9.0), pal.fg, false);
if self.card_show_title { ui::title(v, x + self.scale.s(14.0), y + self.scale.s(10.0), "Disk I/O", self.scale.fs(11.5), pal.fg); }
        // NOTE: R/W legend dots stay right-anchored; header has no icon (no
        // ICON_DISKIO glyph exists) — title-only card by design.
        let plot_x = x + self.scale.s(14.0);
        let plot_y = y + self.scale.s(46.0);
        let plot_w = (w - self.scale.s(28.0)).max(20.0);
        let plot_h = y + h - plot_y - self.scale.s(12.0);
        if plot_h < self.scale.s(20.0) {
            return;
        }
        let hist: Vec<(f32, f32)> = self.disk_history.iter().map(|&(r, w)| (r as f32, w as f32)).collect();
        if hist.len() >= 2 {
            let peak = hist.iter().fold(0.0_f32, |m, &(r, w)| m.max(r).max(w)).max(1.0);
            let r_data: Vec<f32> = hist.iter().map(|&(r, _)| (r / peak).clamp(0.0, 1.0)).collect();
            let w_data: Vec<f32> = hist.iter().map(|&(_, w)| (w / peak).clamp(0.0, 1.0)).collect();
            ui::pulse(v, plot_x, plot_y, plot_w, plot_h, &r_data, r_col, self.scale.s(2.0));
            ui::pulse(v, plot_x, plot_y, plot_w, plot_h, &w_data, w_col, self.scale.s(2.0));
        } else {
            let yb = plot_y + plot_h * 0.5;
            v.push(Cmd::Line { x0: plot_x, y0: yb, x1: plot_x + plot_w, y1: yb, w: self.scale.s(1.0), color: ui::fg3(&pal) });
        }
    }

}
