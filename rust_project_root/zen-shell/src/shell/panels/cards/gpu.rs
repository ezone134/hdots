use super::super::*;
use super::shared::*;

impl Shell {

    /// GPU — util %, temp and VRAM from the detected source (amdgpu sysfs /
    /// nvidia-smi). VRAM row hides when the source exposes none.
    pub(crate) fn draw_gpu_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        let pad = self.scale.s(12.0);
        let hdr = self.scale.s(24.0);
        // no header meta — temp already has its own row below
        self.card_head(v, pal, x, y, w, pad, "GPU", Some(ICON_SETTINGS), None);
        let has_gpu = self.gpu >= 0 || self.gpu_temp > 0.0;
        if !has_gpu {
            card_empty(v, &pal, x, y, w, h, "no GPU source", self.scale);
            return;
        }
        // util big number + bar
        let fs_big = self.scale.fs(22.0);
        let pct = self.gpu.max(0).clamp(0, 100);
        ui::text_r_hero(v, x + w - pad, y + hdr + self.scale.s(0.0), format!("{pct}%"), fs_big, pal.acc);
        ui::caption(v, x + pad, y + hdr + self.scale.s(4.0), "utilization", self.scale.fs(8.5), ui::fg2(&pal), false);
        let bw = w - pad * 2.0;
        ui::bar(v, x + pad, y + hdr + self.scale.s(26.0), bw, self.scale.s(5.0), pct as f32 / 100.0, pal.acc, ui::hover(&pal));
        // temp + vram rows
        let ry = y + hdr + self.scale.s(38.0);
        ui::text(v, x + pad, ry, ICON_THERMAL, self.scale.fs(9.0), ui::fg2(&pal), true);
        ui::text_mono(v, x + pad + self.scale.s(14.0), ry + self.scale.s(1.0), format!("{:.0} °C", self.gpu_temp), self.scale.fs(9.5), pal.fg);
        if self.gpu_vram_total_mb > 0 {
            let used_gb = self.gpu_vram_used_mb as f32 / 1024.0;
            let total_gb = self.gpu_vram_total_mb as f32 / 1024.0;
            ui::text_mono(v, x + pad, ry + self.scale.s(15.0), format!("VRAM {used_gb:.1}/{total_gb:.1} GB"), self.scale.fs(9.5), pal.fg);
            ui::bar(v, x + pad, ry + self.scale.s(27.0), bw, self.scale.s(4.0), self.gpu_vram_used_mb as f32 / self.gpu_vram_total_mb.max(1) as f32, ui::INFO, ui::hover(&pal));
        }
    }

}
