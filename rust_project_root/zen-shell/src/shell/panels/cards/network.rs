use super::super::*;

impl Shell {

    pub(crate) fn draw_network_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        // ONE muted info blue for the download series; accent stays on upload.
        // (legacy hard-coded 0x44aaffff replaced by the shared token)
        let dl_col = ui::INFO;
        let up_col = pal.acc;
        // dual-line history graph fills the card above the speed row; the
        // y-axis normalises against the DOWNLOAD bandwidth (its peak = 100%),
        // so a fluctuating link always uses the full plot height
        let pad = self.scale.s(12.0);
        self.card_head(v, pal, x, y, w, pad, "Network", None, None);
        let row_h = self.scale.s(20.0);
        let gx = x + self.scale.s(12.0);
        let gw = (w - self.scale.s(24.0)).max(20.0);
        let gy = y + self.scale.s(28.0);
        let gh = (h - row_h - self.scale.s(34.0)).max(10.0);
        let hist: Vec<(f32, f32)> = self.net_history.iter().map(|&(d, u)| (d as f32, u as f32)).collect();
        if hist.len() >= 2 {
            let peak = hist.iter().fold(0.0_f32, |m, &(d, u)| m.max(d).max(u)).max(1.0);
            let max_bps = peak as u64;
            let bits = max_bps.saturating_mul(8);
            let mx = if bits >= 1_000_000 {
                format!("{:.1} Mbps", bits as f64 / 1_000_000.0)
            } else if bits >= 1_000 {
                format!("{:.0} Kbps", bits as f64 / 1_000.0)
            } else {
                format!("{bits} bps")
            };
            ui::text_r_mono(v, x + w - self.scale.s(14.0), y + self.scale.s(10.0), &mx, self.scale.fs(9.0), pal.fg);
            let down_data: Vec<f32> = hist.iter().map(|&(d, _)| (d / peak).clamp(0.0, 1.0)).collect();
            let up_data: Vec<f32> = hist.iter().map(|&(_, u)| (u / peak).clamp(0.0, 1.0)).collect();
            ui::pulse(v, gx, gy, gw, gh, &down_data, dl_col, self.scale.s(2.0));
            ui::pulse(v, gx, gy, gw, gh, &up_data, up_col, self.scale.s(2.0));
        }
        // real download + upload speeds, centered in one row below the graph
        let dtxt = if self.net_down.is_empty() { "0 bps".to_string() } else { self.net_down.clone() };
        let utxt = if self.net_up.is_empty() { "0 bps".to_string() } else { self.net_up.clone() };
        let fs = self.scale.fs(10.0);
        let isz = self.scale.fs(9.0);
        let gap = self.scale.s(26.0);
        let pad_s = self.scale.s(2.0);
        let txt_w = |s: &str, sz: f32| s.chars().count() as f32 * sz * 0.62;
        let total = isz + pad_s + txt_w(&dtxt, fs) + gap + isz + pad_s + txt_w(&utxt, fs);
        let mut cx = x + (w - total) / 2.0;
        let sy = y + h - self.scale.s(15.0);
        ui::text(v, cx, sy, ICON_DOWN, isz, dl_col, true);
        cx += isz + pad_s;
        ui::text_mono(v, cx, sy, &dtxt, fs, dl_col);
        cx += txt_w(&dtxt, fs) + gap;
        ui::text(v, cx, sy, ICON_UP, isz, up_col, true);
        cx += isz + pad_s;
        ui::text_mono(v, cx, sy, &utxt, fs, up_col);
    }

}
