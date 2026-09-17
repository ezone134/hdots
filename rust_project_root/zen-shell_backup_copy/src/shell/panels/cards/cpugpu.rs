use super::super::*;

impl Shell {

        pub(crate) fn draw_cpugpu_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        // graph line colours adapt to the mode ($states/m: d/l): brighter in
        // dark, darker in light, so the lines read against the transparent bg
        // (CPU = accent; GPU = muted info blue — was legacy 0xffb86c orange)
        let cpu_col = mix(pal.acc, pal.fg, if self.dark_mode { 0.35 } else { 0.30 });
        let gpu_col = mix(ui::INFO, pal.fg, if self.dark_mode { 0.10 } else { 0.40 });
        if w >= self.scale.s(170.0) {
            // CPU + GPU stacked in the top-right corner, CPU above GPU with a
            // clear vertical gap; each row is a colour dot + right-aligned text
            let rx = x + w - self.scale.s(14.0);
            let dot = self.scale.s(8.0);
            let dx = rx - self.scale.s(52.0);
            v.push(Cmd::Rect { x: dx, y: y + self.scale.s(18.0), w: dot, h: dot, r: self.scale.s(2.0), color: cpu_col });
            ui::text_r(v, rx, y + self.scale.s(15.0), format!("CPU {}%", self.cpu), self.scale.fs(10.0), pal.fg, false);
            if self.gpu >= 0 {
                v.push(Cmd::Rect { x: dx, y: y + self.scale.s(39.0), w: dot, h: dot, r: self.scale.s(2.0), color: gpu_col });
                ui::text_r(v, rx, y + self.scale.s(36.0), format!("GPU {}%", self.gpu), self.scale.fs(10.0), pal.fg, false);
            } else {
                ui::text_r(v, rx, y + self.scale.s(36.0), "GPU n/a", self.scale.fs(10.0), pal.fg, false);
            }
        }
        let plot_x = x + self.scale.s(14.0);
        let plot_y = if w >= self.scale.s(170.0) { y + self.scale.s(52.0) } else { y + self.scale.s(12.0) };
        let plot_w = w - self.scale.s(28.0);
        let plot_h = y + h - plot_y - self.scale.s(14.0);
        if plot_h < self.scale.s(24.0) {
            return;
        }
        // transparent plot background — the lines float on the card itself
        let gpu_data: Vec<f32> = self.gpu_history.iter().copied().collect();
        if gpu_data.len() >= 2 && self.gpu >= 0 {
            ui::pulse(v, plot_x, plot_y, plot_w, plot_h, &gpu_data, gpu_col, self.scale.s(2.0));
        }
        let cpu_data: Vec<f32> = self.cpu_history.iter().copied().collect();
        if cpu_data.len() >= 2 {
            ui::pulse(v, plot_x, plot_y, plot_w, plot_h, &cpu_data, cpu_col, self.scale.s(2.0));
        }
    }

}
