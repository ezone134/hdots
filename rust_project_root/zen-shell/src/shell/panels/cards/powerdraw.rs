use super::super::*;
use super::shared::*;

/// max samples kept on the power-draw history (3 s cadence ≈ 3 min window)
pub(crate) const PW_HISTORY: usize = 60;

impl Shell {

    /// POWER DRAW — battery watts (already tracked once a minute by the
    /// battery backend) plus GPU watts (hwmon `power*_average`) as a dual
    /// line graph: battery = accent, GPU = info blue. Sample cadence is 3 s
    /// while packed, so the fan/turbo behaviour reads as a live trace.
    pub(crate) fn draw_powerdraw_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        let pad = self.scale.s(12.0);
        let last = self.pw_history.back().copied().unwrap_or((0.0, 0.0));
        self.card_head(v, pal, x, y, w, pad, "Power", Some(ICON_BOLT), None);

        if self.pw_history.is_empty() {
            card_empty(v, pal, x, y, w, h, "no power sensors", self.scale);
            return;
        }

        // battery is charging when its draw rose vs the previous sample — that
        // shows a ↑ next to B ("putting watts in"); falling/flat shows nothing.
        let charging = self.pw_history.len() >= 2
            && self.pw_history.back().map(|c| c.0).unwrap_or(0.0)
                > self.pw_history.get(self.pw_history.len() - 2).map(|p| p.0).unwrap_or(0.0) + 0.05;

        let b_col = pal.acc;
        let g_col = ui::INFO;

        // right-side info column: watts shown to the LEFT of the B / G letter
        // (e.g. `8.4W ↑ B`), stacked under the header along the card's right
        // edge. A ↑ before B signals charging; no arrow when falling/flat.
        let fs = self.scale.fs(10.0);
        let row_h = self.scale.s(18.0);
        let side_w = self.scale.s(80.0);
        let lx = x + w - pad;
        let dot = self.scale.s(7.0);
        let dot_x = lx - side_w + self.scale.s(4.0);
        let arrow = if charging { "↑" } else { "" };

        let ry_b = y + self.scale.s(32.0);
        let ry_g = ry_b + row_h;
        ui::text_r(v, lx, ry_b, &format!("{:.1}W {}B", last.0, arrow), fs, pal.fg, false);
        ui::text_r(v, lx, ry_g, &format!("{:.1}W G", last.1), fs, pal.fg, false);
        v.push(Cmd::Rect { x: dot_x, y: ry_b + self.scale.s(2.0), w: dot, h: dot, r: self.scale.s(2.0), color: b_col });
        v.push(Cmd::Rect { x: dot_x, y: ry_g + self.scale.s(2.0), w: dot, h: dot, r: self.scale.s(2.0), color: g_col });

        let plot_x = x + self.scale.s(14.0);
        let plot_y = y + self.scale.s(30.0);
        let plot_w = (w - self.scale.s(28.0) - side_w).max(20.0);
        let plot_h = (y + h - plot_y - self.scale.s(12.0)).max(self.scale.s(10.0));
        if self.pw_history.len() >= 2 {
            let peak = self.pw_history.iter().fold(1.0_f32, |m, &(b, g)| m.max(b).max(g));
            let b_data: Vec<f32> = self.pw_history.iter().map(|&(b, _)| (b / peak).clamp(0.0, 1.0)).collect();
            let g_data: Vec<f32> = self.pw_history.iter().map(|&(_, g)| (g / peak).clamp(0.0, 1.0)).collect();
            ui::pulse(v, plot_x, plot_y, plot_w, plot_h, &b_data, b_col, self.scale.s(2.0));
            ui::pulse(v, plot_x, plot_y, plot_w, plot_h, &g_data, g_col, self.scale.s(2.0));
        } else {
            let yb = plot_y + plot_h * 0.5;
            v.push(Cmd::Line { x0: plot_x, y0: yb, x1: plot_x + plot_w, y1: yb, w: self.scale.s(1.0), color: ui::fg3(&pal) });
        }
    }

    /// Sample battery + GPU watts and append to the history. Throttled to the
    /// 3 s services beat; returns true when a sample was pushed.
    pub(crate) fn poll_powerdraw(&mut self) -> bool {
        if let Some(next) = self.pw_next {
            if Instant::now() < next {
                return false;
            }
        }
        self.pw_next = Some(Instant::now() + std::time::Duration::from_secs(3));
        let sample = (self.battery_watts, gpu_watts());
        self.pw_history.push_back(sample);
        while self.pw_history.len() > PW_HISTORY {
            self.pw_history.pop_front();
        }
        true
    }
}

/// GPU power draw in watts from hwmon (`power*_average`, µW) — amdgpu /
/// radeon only; 0.0 when the chip doesn't expose a power sensor.
pub(crate) fn gpu_watts() -> f32 {
    let Ok(entries) = std::fs::read_dir("/sys/class/hwmon") else { return 0.0 };
    for e in entries.flatten() {
        let base = e.path();
        let Ok(name) = std::fs::read_to_string(base.join("name")) else { continue };
        if !(name.trim().eq_ignore_ascii_case("amdgpu") || name.trim().eq_ignore_ascii_case("radeon")) {
            continue;
        }
        let Ok(power) = std::fs::read_dir(&base) else { continue };
        for fe in power.flatten() {
            let f = fe.file_name().to_string_lossy().into_owned();
            if f.starts_with("power") && f.ends_with("_average") {
                if let Ok(raw) = std::fs::read_to_string(fe.path()) {
                    if let Ok(w) = raw.trim().parse::<f32>() {
                        return w / 1e6;
                    }
                }
            }
        }
    }
    0.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shell() -> Shell {
        let cfg: crate::config::Config =
            toml::from_str(crate::config::DEFAULT_SHELL_TOML).expect("default config parses");
        Shell::new(cfg)
    }

    fn pal() -> Pal {
        Pal { fg: 0xe8e6e3ff, bg: 0x141414ff, acc: 0x4fc2ffff, sfg: 0x101010ff }
    }

    #[test]
    fn card_draws_empty_and_sampled() {
        let mut s = shell();
        let p = pal();
        let mut v: Vec<Cmd> = Vec::new();
        s.draw_powerdraw_card(&mut v, 0.0, 0.0, 170.0, 110.0, &p);
        s.pw_history = vec![(8.0, 35.0), (8.4, 36.0)].into_iter().collect();
        s.draw_powerdraw_card(&mut v, 0.0, 0.0, 170.0, 110.0, &p);
        assert!(v.iter().any(|c| matches!(c, Cmd::Text { text, .. } if text.contains("8.4"))), "last battery watts in meta");
    }

    #[test]
    fn history_capped_and_throttled() {
        let mut s = shell();
        for _ in 0..(PW_HISTORY + 10) {
            s.pw_history.push_back((1.0, 2.0));
            while s.pw_history.len() > PW_HISTORY {
                s.pw_history.pop_front();
            }
        }
        assert_eq!(s.pw_history.len(), PW_HISTORY);
        // freshly sampled → throttled
        assert!(s.poll_powerdraw(), "first sample pushed");
        assert!(!s.poll_powerdraw(), "no second sample within 3 s window");
    }
}