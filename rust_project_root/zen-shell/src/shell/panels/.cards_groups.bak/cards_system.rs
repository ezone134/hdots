use super::*;

impl Shell {

        pub(crate) fn draw_system_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        // LEFT: username → hostname → clock → date → current color scheme
        // RIGHT: bell + darkmode (top) → uptime → battery → wattage (stacked below)
        let uname: String = self.username.chars().take(12).collect();
        ui::text(v, x + self.scale.s(16.0), y + self.scale.s(14.0), &uname, self.scale.fs(21.0), pal.fg, true);
        let hname: String = self.hostname.chars().take((w / 5.5).max(6.0) as usize).collect();
        ui::text(v, x + self.scale.s(17.0), y + self.scale.s(42.0), &hname, self.scale.fs(10.0), pal.fg, false);

        // clock (left column, below hostname)
        if !self.clock.is_empty() {
            ui::text(v, x + self.scale.s(16.0), y + self.scale.s(62.0), self.clock.clone(), self.scale.fs(26.0), pal.fg, false);
        }
        // date chip (left column, below clock)
        {
            let dtext = self.date_str("%a, %d %b");
            let dw = dtext.chars().count() as f32 * self.scale.s(12.0) * 0.62 + self.scale.s(18.0);
            let dx = x + self.scale.s(14.0);
            let dy = y + self.scale.s(98.0);
            let hov = self.hover_key == 25;
            v.push(Cmd::Rect {
                x: dx, y: dy, w: dw, h: self.scale.s(22.0), r: self.scale.s(11.0),
                color: if hov { ui::hover_hl(&pal) } else { mix(ui::hover(&pal), pal.fg, 0.06) },
            });
            ui::text(v, dx + self.scale.s(9.0), dy + self.scale.s(5.0), dtext, self.scale.fs(12.0), if hov { pal.acc } else { pal.fg }, false);
            self.region(dx - self.scale.s(4.0), dy - self.scale.s(3.0), dw + self.scale.s(8.0), self.scale.s(28.0), 25);
        }

        // UI state line (left column, below the date chip). Refreshed by the 3 s
        // services poll while the dashboard is open — no expand hook.
        // (Color scheme + accent source now live on the Accent card.)
        if h >= self.scale.s(150.0) {
            if !self.cur_channel.is_empty() {
                let ctext = format!("UI State: {}", self.cur_channel);
                ui::text(v, x + self.scale.s(16.0), y + self.scale.s(124.0), &ctext, self.scale.fs(9.5), pal.fg, false);
            }
        }

        // ── RIGHT column: bell+darkmode at top, uptime/battery/wattage stacked below ──
        let rx = x + w - self.scale.s(16.0);

        // bell (top-right)
        {
            let bx = x + w - self.scale.s(40.0);
            let by = y + self.scale.s(12.0);
            let hov = self.hover_key == 24;
            if hov {
                v.push(Cmd::Rect { x: bx, y: by, w: self.scale.s(26.0), h: self.scale.s(22.0), r: self.scale.s(11.0), color: ui::hover_hl(&pal) });
            }
            let glyph = if self.dnd { ICON_DND } else { ICON_BELL };
            ui::text(v, bx + self.scale.s(7.0), by + self.scale.s(4.0), glyph, self.scale.fs(13.0), if hov { pal.acc } else { pal.fg }, true);
            if self.notif_count > 0 && !self.dnd {
                v.push(Cmd::Rect { x: bx + self.scale.s(17.0), y: by - self.scale.s(2.0), w: self.scale.s(7.0), h: self.scale.s(7.0), r: self.scale.s(3.5), color: pal.acc });
            }
            self.region(bx - self.scale.s(3.0), by - self.scale.s(3.0), self.scale.s(32.0), self.scale.s(28.0), 24);
        }
        // dark/light toggle (left of bell)
        {
            let bx = x + w - self.scale.s(72.0);
            let by = y + self.scale.s(12.0);
            let hov = self.hover_key == 26;
            if hov {
                v.push(Cmd::Rect { x: bx, y: by, w: self.scale.s(26.0), h: self.scale.s(22.0), r: self.scale.s(11.0), color: ui::hover_hl(&pal) });
            }
            let glyph = if self.dark_mode { ICON_MOON } else { ICON_BRIGHTNESS };
            ui::text(v, bx + self.scale.s(6.5), by + self.scale.s(4.0), glyph, self.scale.fs(13.0), if hov { pal.acc } else { pal.fg }, true);
            self.region(bx - self.scale.s(3.0), by - self.scale.s(3.0), self.scale.s(32.0), self.scale.s(28.0), 26);
        }
        // uptime (below bell/darkmode)
        {
            let uy = y + self.scale.s(44.0);
            ui::text_r(v, rx, uy, format!("Uptime {}", self.uptime), self.scale.fs(9.5), pal.fg, false);
        }
        // battery + wattage (below uptime)
        if self.battery >= 0 {
            let low = self.battery <= 20 && !self.ac_online;
            let bc = if low { RED } else { pal.fg };
            let ic = if low { RED } else { pal.fg };
            let by = y + self.scale.s(64.0);
            let icon_w = ui::battery_icon_w(self.scale.s(13.0));
            // vertical nerd-font battery glyph (right-aligned, no text overlapping)
            let glyph = ui::battery_glyph(self.battery, self.ac_online);
            ui::text_r(v, rx, by + self.scale.s(1.0), glyph, self.scale.s(13.0), ic, true);
            // battery % as separate text OUTSIDE the icon (left of it)
            if self.pill_battery_pct {
                let pct = format!("{}%", self.battery);
                ui::text_r(v, rx - icon_w - self.scale.s(4.0), by + self.scale.s(1.0) + self.scale.s(1.0), pct, self.scale.fs(10.5), ic, false);
            }
            // time remaining (below battery icon)
            if !self.battery_time.is_empty() {
                ui::text_r(v, rx, by + self.scale.s(18.0), &self.battery_time, self.scale.fs(9.5), bc, false);
            }
            // wattage (below time)
            if self.battery_watts > 0.0 {
                ui::text_r(v, rx, by + self.scale.s(32.0), format!("{:.1}W", self.battery_watts), self.scale.fs(9.0), pal.fg, false);
            }
        }

        // ── bottom-right corner: `>` always opens Settings ──
        let sx = x + w - self.scale.s(26.0);
        let sy = y + h - self.scale.s(24.0);
        let hov = self.hover_key == 27;
        if hov {
            v.push(Cmd::Rect { x: sx, y: sy, w: self.scale.s(20.0), h: self.scale.s(18.0), r: self.scale.s(9.0), color: ui::hover_hl(&pal) });
        }
        ui::text(v, sx + self.scale.s(6.0), sy + self.scale.s(5.0), ICON_FORWARD, self.scale.fs(10.0), if hov { pal.acc } else { ui::fg2(&pal) }, true);
        self.region(sx - self.scale.s(2.0), sy - self.scale.s(2.0), self.scale.s(26.0), self.scale.s(22.0), 27);
    }


    /// WEATHER — its own card now (it used to be one line inside the clock).

        pub(crate) fn draw_weather_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        let ccx = x + w / 2.0;
        let Some(wx) = self.weather.as_ref() else {
            ui::text_c(v, ccx, y + h / 2.0 - self.scale.s(8.0), ICON_SUNNY, self.scale.fs(20.0), pal.fg, true);
            ui::text_c(v, ccx, y + h / 2.0 + self.scale.s(16.0), "waiting…", self.scale.fs(9.5), pal.fg, false);
            return;
        };
        let (glyph, desc) = crate::weather::describe(wx.code);
        if h < self.scale.s(60.0) {
            // squat span: one dense line
            ui::text_c(v, ccx, y + h / 2.0 - self.scale.s(8.0), format!("{glyph} {:.0}°C", wx.temp_c), self.scale.fs(13.0), pal.fg, true);
            return;
        }
        ui::text_c(v, ccx, y + self.scale.s(16.0), format!("{glyph} {:.0}°C", wx.temp_c), self.scale.fs(20.0), pal.fg, true);
        ui::text_c(v, ccx, y + self.scale.s(48.0), desc, self.scale.fs(11.0), pal.fg, false);
        if !self.weather_city.is_empty() {
            ui::text_c(v, ccx, y + self.scale.s(66.0), self.weather_city.clone(), self.scale.fs(9.5), pal.fg, false);
        }
        let mut detail = String::new();
        if let Some(fl) = wx.feels_like {
            detail.push_str(&format!("feels {:.0}°", fl));
        }
        if wx.wind_kmh > 0.0 {
            if !detail.is_empty() {
                detail.push_str(" · ");
            }
            detail.push_str(&format!("{:.0} km/h", wx.wind_kmh));
        }
        if !detail.is_empty() && h >= self.scale.s(100.0) {
            ui::text_c(v, ccx, y + h - self.scale.s(26.0), detail, self.scale.fs(9.5), pal.fg, false);
        }
    }


        pub(crate) fn draw_cpugpu_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        // graph line colours adapt to the mode ($states/m: d/l): brighter in
        // dark, darker in light, so the lines read against the transparent bg
        let cpu_col = mix(pal.acc, pal.fg, if self.dark_mode { 0.35 } else { 0.30 });
        let gpu_col = mix(0xffb86cff, pal.fg, if self.dark_mode { 0.10 } else { 0.40 });
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


    /// Disk I/O — read (blue) + write (accent) speed on one history graph,
    /// normalised against the read peak as 100% (same language as network).
    pub(crate) fn draw_diskio_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        let r_col = mix(0x44aaffff, pal.fg, 0.10);
        let w_col = pal.acc;
        let rx = x + w - self.scale.s(14.0);
        let dot = self.scale.s(7.0);
        let dx = rx - self.scale.s(44.0);
        v.push(Cmd::Rect { x: dx, y: y + self.scale.s(16.0), w: dot, h: dot, r: self.scale.s(2.0), color: r_col });
        ui::text_r(v, rx, y + self.scale.s(14.0), "R", self.scale.fs(9.0), pal.fg, false);
        v.push(Cmd::Rect { x: dx, y: y + self.scale.s(34.0), w: dot, h: dot, r: self.scale.s(2.0), color: w_col });
        ui::text_r(v, rx, y + self.scale.s(32.0), "W", self.scale.fs(9.0), pal.fg, false);
        ui::text(v, x + self.scale.s(14.0), y + self.scale.s(10.0), "Disk I/O", self.scale.fs(11.5), pal.fg, false);
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


        /// Bead ring around a centre point — `lit` beads (of 36) are lit.
        fn ring_beads(v: &mut Vec<Cmd>, cx: f32, cy: f32, radius: f32, dot_d: f32, lit: i32, col: u32, dim: u32) {
            const BEADS: i32 = 36;
            for b in 0..BEADS {
                let ang = (b as f32 / BEADS as f32) * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
                let dx = cx + ang.cos() * radius - dot_d / 2.0;
                let dy = cy + ang.sin() * radius - dot_d / 2.0;
                v.push(Cmd::Rect { x: dx, y: dy, w: dot_d, h: dot_d, r: dot_d / 2.0, color: if b < lit { col } else { dim } });
            }
        }

        /// Short GB readout for ring interiors ("3.4" / "120").
        fn gauge_gb(gb: f32) -> String {
            if gb >= 100.0 { format!("{gb:.0}") } else { format!("{gb:.1}") }
        }

        pub(crate) fn draw_gauges_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        let ring_r = if h >= self.scale.s(116.0) { self.scale.s(30.0) } else { self.scale.s(22.0) };
        let dot_d = if h >= self.scale.s(116.0) { self.scale.s(5.0) } else { self.scale.s(4.0) };
        let dim = mix(ui::hover(&pal), pal.fg, 0.08);
        let cy = y + h / 2.0;
        // (label, root-prefix, pct, ring color, used_gb, total_gb)
        let gauges: [(&str, &str, i32, u32, f32, f32); 2] = [
            ("Memory", "", self.mem_pct, pal.acc, self.mem_used_gb, self.mem_total_gb),
            ("Disk", "/", self.disk_pct, GREEN, self.disk_used_gb, self.disk_total_gb),
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
            // inside the gauge: percentage only
            ui::text_c(v, cx, cy - self.scale.s(8.0), format!("{}%", pct), fs_pct, pal.fg, true);
            // below the gauge: label, then used/total GB ("/ 3G/100G" on disk)
            if h >= ring_r * 2.0 + self.scale.s(50.0) {
                ui::text_c(v, cx, cy + ring_r + self.scale.s(15.0), label, self.scale.fs(10.0), pal.fg, false);
                let gb = if total > 0.0 {
                    format!("{} {:.0}G/{:.0}G", root, used, total).trim_start().to_string()
                } else {
                    format!("{root} {pct}%").trim_start().to_string()
                };
                ui::text_c(v, cx, cy + ring_r + self.scale.s(28.0), gb, self.scale.fs(8.0), ui::fg2(&pal), false);
            }
        }
    }


    pub(crate) fn draw_disk_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        ui::text(v, x + self.scale.s(14.0), y + self.scale.s(10.0), "Disk", self.scale.fs(11.5), pal.fg, false);
        let parts = self.disk_parts.clone();
        let row_h = self.scale.s(24.0);
        let visible = (((h - self.scale.s(36.0)) / row_h).floor() as usize).min(parts.len());
        for i in 0..visible {
            let (ref mount, pct, used_gb, total_gb) = parts[i];
            let ry = y + self.scale.s(32.0) + i as f32 * row_h;
            let bar_x = x + self.scale.s(14.0);
            let bar_w = w - self.scale.s(28.0);
            ui::text(v, bar_x, ry, mount, self.scale.fs(9.0), pal.fg, false);
            ui::text_r(v, x + w - self.scale.s(14.0), ry, format!("{used_gb:.0}/{total_gb:.0} GB {pct}%"), self.scale.fs(9.0), pal.fg, false);
            let bar_y = ry + self.scale.s(13.0);
            let col = if pct >= 90 { RED } else if pct >= 75 { 0xf9e2afff } else { pal.acc };
            v.push(Cmd::Rect { x: bar_x, y: bar_y, w: bar_w, h: self.scale.s(6.0), r: self.scale.s(3.0), color: ui::hover(&pal) });
            v.push(Cmd::Rect { x: bar_x, y: bar_y, w: bar_w * pct as f32 / 100.0, h: self.scale.s(6.0), r: self.scale.s(3.0), color: col });
        }
        if parts.is_empty() {
            ui::text_c(v, x + w / 2.0, y + h / 2.0, "No partitions", self.scale.fs(9.5), pal.fg, false);
        }
    }


    pub(crate) fn draw_procmon_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        ui::text(v, x + self.scale.s(14.0), y + self.scale.s(10.0), "Processes", self.scale.fs(11.5), pal.fg, false);
        ui::text_r(v, x + w - self.scale.s(14.0), y + self.scale.s(12.0), format!("CPU {}%", self.cpu), self.scale.fs(9.0), pal.fg, false);
        let procs = self.top_procs.clone();
        let visible = (((h - self.scale.s(36.0)) / self.scale.s(22.0)).floor() as usize).min(procs.len());
        for i in 0..visible {
            let (ref name, _ticks, _) = procs[i];
            let ry = y + self.scale.s(32.0) + i as f32 * self.scale.s(22.0);
            let label: String = name.chars().take(16).collect();
            ui::text(v, x + self.scale.s(14.0), ry, &label, self.scale.fs(9.0), pal.fg, false);
        }
    }


    pub(crate) fn draw_mem_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        let hdr = self.scale.s(26.0);
        let pad = self.scale.s(14.0);
        ui::text(v, x + pad, y + self.scale.s(10.0), "Memory", self.scale.fs(11.5), pal.fg, false);
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
        ui::text_c(v, cx, cy - self.scale.s(7.0), format!("{}%", self.mem_pct), fs_pct, pal.fg, true);
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
                ui::text(v, col_x, row_y, label, self.scale.fs(8.5), ui::fg2(&pal), false);
                ui::text_r(v, x + w - pad, row_y, format!("{gb:.1}G"), self.scale.fs(8.5), pal.fg, false);
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


    /// CPU — load average (1/5/15 min), live clock frequency, package temp
    /// and usage %, with a usage bar under the header.
    pub(crate) fn draw_cpu_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        let hdr = self.scale.s(26.0);
        let pad = self.scale.s(14.0);
        ui::text(v, x + pad, y + self.scale.s(10.0), "CPU", self.scale.fs(11.5), pal.fg, false);
        let uc = if self.cpu >= 90 { RED } else if self.cpu >= 70 { 0xf9e2afff } else { pal.fg };
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
        ui::text_c(v, cx, cy - self.scale.s(7.0), format!("{}%", self.cpu), fs_pct, uc, true);
        // detail rows to the right: load / freq / temp
        let col_w = (x + w - pad) - col_x;
        if col_w > self.scale.s(70.0) {
            let mhz = self.cpu_freq_mhz;
            let freq = if mhz >= 1000 { format!("{:.2} GHz", mhz as f32 / 1000.0) } else if mhz > 0 { format!("{mhz} MHz") } else { "—".to_string() };
            let rows: [(&'static str, String, u32); 3] = [
                ("Load", format!("{:.2}/{:.2}/{:.2}", self.cpu_load.0, self.cpu_load.1, self.cpu_load.2), pal.fg),
                ("Freq", freq, pal.fg),
                ("Temp", if self.cpu_temp > 0.0 { format!("{:.0}°C", self.cpu_temp) } else { "—".to_string() },
                    if self.cpu_temp > 80.0 { RED } else if self.cpu_temp > 65.0 { 0xf9e2afff } else { pal.fg }),
            ];
            // center the stacked rows on the circle's centre
            let ry0 = cy - (rows.len() as f32 - 1.0) * self.scale.s(11.0);
            for (i, (label, val, color)) in rows.into_iter().enumerate() {
                let row_y = ry0 + i as f32 * self.scale.s(22.0);
                ui::text(v, col_x, row_y, label, self.scale.fs(8.5), ui::fg2(&pal), false);
                ui::text_r(v, x + w - pad, row_y, val, self.scale.fs(8.5), color, false);
            }
        }
    }


    pub(crate) fn draw_topproc_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        ui::text(v, x + self.scale.s(14.0), y + self.scale.s(10.0), "Top Processes", self.scale.fs(11.5), pal.fg, false);
        let procs = self.top_procs.clone();
        let visible = (((h - self.scale.s(36.0)) / self.scale.s(26.0)).floor() as usize).min(procs.len());
        for i in 0..visible {
            let (ref name, _ticks, _) = procs[i];
            let ry = y + self.scale.s(32.0) + i as f32 * self.scale.s(26.0);
            let label: String = name.chars().take(18).collect();
            ui::text(v, x + self.scale.s(14.0), ry, &label, self.scale.fs(9.5), pal.fg, false);
            // small bar
            let bar_x = x + self.scale.s(14.0);
            let bar_y = ry + self.scale.s(14.0);
            let bar_w = w - self.scale.s(28.0);
            v.push(Cmd::Rect { x: bar_x, y: bar_y, w: bar_w, h: self.scale.s(4.0), r: self.scale.s(2.0), color: ui::hover(&pal) });
        }
    }


    pub(crate) fn draw_activewin_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        let cx = x + w / 2.0;
        if self.active_win_class.is_empty() {
            ui::text_c(v, cx, y + self.scale.s(14.0), ICON_ACTIVE, self.scale.fs(20.0), pal.fg, true);
            ui::text_c(v, cx, y + h / 2.0 + self.scale.s(10.0), "No window", self.scale.fs(10.0), pal.fg, false);
            return;
        }
        ui::text_c(v, cx, y + self.scale.s(14.0), &self.active_win_class, self.scale.fs(13.0), pal.fg, false);
        let title: String = self.active_win_title.chars().take(28).collect();
        ui::text_c(v, cx, y + self.scale.s(36.0), &title, self.scale.fs(9.5), pal.fg, false);
        // live per-app RAM (VmRSS of the active window's process) — a tiny
        // pill under the title so the card shows more than just the name
        if self.active_win_rss_kb > 0 {
            let kb = self.active_win_rss_kb;
            let (val, unit) = if kb >= 1_048_576 {
                (kb as f32 / 1_048_576.0, "GB")
            } else {
                (kb as f32 / 1024.0, "MB")
            };
            ui::text_c(v, cx, y + h - self.scale.s(16.0), format!("\u{f2db} {val:.1} {unit}"), self.scale.fs(9.0), ui::fg2(&pal), false);
        }
    }


    /// GPU — util %, temp and VRAM from the detected source (amdgpu sysfs /
    /// nvidia-smi). VRAM row hides when the source exposes none.
    pub(crate) fn draw_gpu_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        let pad = self.scale.s(12.0);
        let hdr = self.scale.s(24.0);
        ui::text(v, x + pad, y + self.scale.s(8.0), ICON_SETTINGS, self.scale.fs(12.0), pal.acc, true);
        ui::text(v, x + pad + self.scale.s(17.0), y + self.scale.s(9.0), "GPU", self.scale.fs(12.0), pal.fg, false);
        if self.gpu_temp > 0.0 {
            ui::text_r(v, x + w - pad, y + self.scale.s(9.0), format!("{:.0} °C", self.gpu_temp), self.scale.fs(8.5), 0xffb86cff, false);
        }
        let has_gpu = self.gpu >= 0 || self.gpu_temp > 0.0;
        if !has_gpu {
            ui::text_c(v, x + w / 2.0, y + h / 2.0 - self.scale.s(6.0), "no GPU source", self.scale.fs(9.0), ui::fg3(&pal), false);
            return;
        }
        // util big number + bar
        let fs_big = self.scale.fs(22.0);
        let pct = self.gpu.max(0).clamp(0, 100);
        ui::text_r(v, x + w - pad, y + hdr + self.scale.s(0.0), format!("{pct}%"), fs_big, pal.acc, false);
        ui::text(v, x + pad, y + hdr + self.scale.s(4.0), "utilization", self.scale.fs(8.5), ui::fg3(&pal), false);
        let bw = w - pad * 2.0;
        ui::bar(v, x + pad, y + hdr + self.scale.s(26.0), bw, self.scale.s(5.0), pct as f32 / 100.0, pal.acc, ui::hover(&pal));
        // temp + vram rows
        let ry = y + hdr + self.scale.s(38.0);
        ui::text(v, x + pad, ry, ICON_THERMAL, self.scale.fs(9.0), 0xffb86cff, true);
        ui::text(v, x + pad + self.scale.s(14.0), ry + self.scale.s(1.0), format!("{:.0} °C", self.gpu_temp), self.scale.fs(9.5), pal.fg, false);
        if self.gpu_vram_total_mb > 0 {
            let used_gb = self.gpu_vram_used_mb as f32 / 1024.0;
            let total_gb = self.gpu_vram_total_mb as f32 / 1024.0;
            ui::text(v, x + pad, ry + self.scale.s(15.0), format!("VRAM {used_gb:.1}/{total_gb:.1} GB"), self.scale.fs(9.5), pal.fg, false);
            ui::bar(v, x + pad, ry + self.scale.s(27.0), bw, self.scale.s(4.0), self.gpu_vram_used_mb as f32 / self.gpu_vram_total_mb.max(1) as f32, 0x44aaffff, ui::hover(&pal));
        }
    }


    /// THERMAL — all hwmon zones in a small grid, colored by temperature.
    pub(crate) fn draw_thermal_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        const ORANGE: u32 = 0xffb86cff;
        let pad = self.scale.s(12.0);
        let hdr = self.scale.s(24.0);
        ui::text(v, x + pad, y + self.scale.s(8.0), ICON_THERMAL, self.scale.fs(12.0), pal.acc, true);
        ui::text(v, x + pad + self.scale.s(17.0), y + self.scale.s(9.0), "Thermal zones", self.scale.fs(12.0), pal.fg, false);
        ui::text_r(v, x + w - pad, y + self.scale.s(9.0), format!("{} zones", self.thermal_zones.len()), self.scale.fs(8.5), ui::fg3(&pal), false);

        if self.thermal_zones.is_empty() {
            ui::text_c(v, x + w / 2.0, y + h / 2.0 - self.scale.s(6.0), "no /sys/class/thermal zones", self.scale.fs(9.0), ui::fg3(&pal), false);
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
            let tc = if *temp >= 80.0 { RED } else if *temp >= 60.0 { ORANGE } else { ui::fg2(&pal) };
            let zn: String = zone.chars().take(8).collect();
            ui::text(v, cx + self.scale.s(4.0), cy + self.scale.s(3.0), &zn, self.scale.fs(7.5), ui::fg3(&pal), false);
            ui::text(v, cx + self.scale.s(4.0), cy + self.scale.s(12.0), format!("{:.0}°", temp), self.scale.fs(9.0), tc, false);
        }
    }


    /// SENSORS — live accelerometer tilt + optional gyro/compass readout.
    pub(crate) fn draw_sensors_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        ui::text(v, x + self.scale.s(12.0), y + self.scale.s(8.0), ICON_SENSORS, self.scale.fs(12.0), pal.acc, true);
        ui::text(v, x + self.scale.s(30.0), y + self.scale.s(9.0), "Sensors", self.scale.fs(12.0), pal.fg, false);
        let s = &self.sensors;
        let none = !s.has_accel && s.heading.is_none() && s.gyro.is_none();
        if none {
            ui::text_c(v, x + w / 2.0, y + h / 2.0 - self.scale.s(6.0), "no IIO sensors", self.scale.fs(9.0), ui::fg3(&pal), false);
            return;
        }
        let mut ry = y + self.scale.s(32.0);
        ui::text(v, x + self.scale.s(16.0), ry, "tilt", self.scale.fs(9.0), ui::fg2(&pal), false);
        ui::text_r(v, x + w - self.scale.s(16.0), ry, format!("pitch {:.0}°  roll {:.0}°", s.pitch, s.roll), self.scale.fs(9.5), pal.fg, false);
        ry += self.scale.s(20.0);
        if let Some(hdg) = s.heading {
            ui::text(v, x + self.scale.s(16.0), ry, "compass", self.scale.fs(9.0), ui::fg2(&pal), false);
            let arrow = match (0.5 + hdg / 360.0 * 8.0) as usize % 8 {
                0 => ICON_GAUGE_0,
                1 => ICON_GAUGE_1,
                2 => ICON_GAUGE_2,
                3 => ICON_GAUGE_3,
                4 => ICON_GAUGE_4,
                5 => ICON_GAUGE_5,
                6 => ICON_GAUGE_6,
                _ => ICON_GAUGE_7,
            };
            ui::text_r(v, x + w - self.scale.s(16.0), ry, format!("{arrow} {:.0}°", hdg), self.scale.fs(9.5), pal.acc, true);
            ry += self.scale.s(20.0);
        }
        if let Some((gx, gy, gz)) = s.gyro {
            ui::text(v, x + self.scale.s(16.0), ry, "gyro °/s", self.scale.fs(9.0), ui::fg2(&pal), false);
            ui::text_r(v, x + w - self.scale.s(16.0), ry, format!("{gx:3.0} {gy:3.0} {gz:3.0}"), self.scale.fs(9.0), pal.fg, false);
        }
    }


    /// MIRROR — live front-camera feed (app pumps decoded frames into the
    /// `cam` image key).
    pub(crate) fn draw_mirror_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        ui::text(v, x + self.scale.s(12.0), y + self.scale.s(8.0), ICON_CAMERA, self.scale.fs(12.0), pal.acc, true);
        ui::text(v, x + self.scale.s(30.0), y + self.scale.s(9.0), "Mirror", self.scale.fs(12.0), pal.fg, false);
        if !self.cam_ok {
            ui::text_c(v, x + w / 2.0, y + h / 2.0 - self.scale.s(6.0), "no camera", self.scale.fs(9.0), ui::fg3(&pal), false);
            return;
        }
        // aspect-fit the frame in the card body
        let pad = self.scale.s(10.0);
        let bw = w - pad * 2.0;
        let bh = (h - self.scale.s(40.0)).max(20.0);
        let size = bw.min(bh);
        let cx = x + (w - size) / 2.0;
        let cy = y + self.scale.s(38.0) + (bh - size) / 2.0;
        v.push(Cmd::Image { x: cx, y: cy, size, key: "cam".to_string() });
    }


    pub(crate) fn draw_pkgupdates_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        let cx = x + w / 2.0;
        let cy = y + h / 2.0;
        let count = self.pkg_update_count;
        let glyph = if count > 0 { ICON_DOWN } else { ICON_CHECK };
        let col = if count > 0 { pal.acc } else { GREEN };
        ui::text_c(v, cx, cy - self.scale.s(18.0), glyph, self.scale.fs(22.0), col, true);
        ui::text_c(v, cx, cy + self.scale.s(10.0), format!("{count}"), self.scale.fs(20.0), pal.fg, true);
        ui::text_c(v, cx, cy + self.scale.s(34.0), if count > 0 { "updates" } else { "up to date" }, self.scale.fs(9.0), pal.fg, false);
    }


    pub(crate) fn draw_sshvpn_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        ui::text(v, x + self.scale.s(14.0), y + self.scale.s(10.0), "SSH / VPN", self.scale.fs(11.5), pal.fg, false);
        let lines = self.ssh_vpn_lines.clone();
        if lines.is_empty() {
            ui::text_c(v, x + w / 2.0, y + h / 2.0, "No connections", self.scale.fs(9.5), pal.fg, false);
            return;
        }
        let visible = (((h - self.scale.s(36.0)) / self.scale.s(22.0)).floor() as usize).min(lines.len());
        for i in 0..visible {
            let ry = y + self.scale.s(32.0) + i as f32 * self.scale.s(22.0);
            let glyph = if lines[i].starts_with("VPN") { ICON_GLOBE } else { ICON_SHIELD };
            ui::text(v, x + self.scale.s(14.0), ry, glyph, self.scale.fs(10.0), pal.acc, true);
            ui::text(v, x + self.scale.s(30.0), ry, &lines[i], self.scale.fs(9.5), pal.fg, false);
        }
    }


    pub(crate) fn draw_docker_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        ui::text(v, x + self.scale.s(14.0), y + self.scale.s(10.0), "Docker", self.scale.fs(11.5), pal.fg, false);
        let containers = self.docker_containers.clone();
        let visible = (((h - self.scale.s(36.0)) / self.scale.s(26.0)).floor() as usize).min(containers.len());
        for i in 0..visible {
            let (ref name, ref status, ref image) = containers[i];
            let ry = y + self.scale.s(32.0) + i as f32 * self.scale.s(26.0);
            let dot_col = if status.contains("Up") { GREEN } else { RED };
            v.push(Cmd::Rect { x: x + self.scale.s(14.0), y: ry + self.scale.s(3.0), w: self.scale.s(6.0), h: self.scale.s(6.0), r: 3.0, color: dot_col });
            ui::text(v, x + self.scale.s(24.0), ry, name, self.scale.fs(9.0), pal.fg, false);
            let img_label: String = image.chars().take(20).collect();
            ui::text_r(v, x + w - self.scale.s(14.0), ry, &img_label, self.scale.fs(7.5), pal.fg, false);
        }
        if containers.is_empty() {
            ui::text_c(v, x + w / 2.0, y + h / 2.0, "No containers", self.scale.fs(9.5), pal.fg, false);
        }
    }


    pub(crate) fn draw_kblayout_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        let cx = x + w / 2.0;
        let cy = y + h / 2.0;
        let layout = if self.kb_layout.is_empty() { "Unknown".to_string() } else { self.kb_layout.clone() };
        ui::text_c(v, cx, cy - self.scale.s(12.0), ICON_KEYBOARD, self.scale.fs(22.0), pal.acc, true);
        ui::text_c(v, cx, cy + self.scale.s(14.0), &layout, self.scale.fs(14.0), pal.fg, false);
    }


    /// The five power actions shared by the vertical / horizontal power-menu
    /// cards: icon, label, and whether it's a danger (hold-to-confirm) action.
    pub(crate) fn power_card_items() -> [(&'static str, bool); 5] {
        [
            (ICON_LOCK, false), // lock
            (ICON_LOGOUT, true),  // logout
            (ICON_MOON, false), // sleep
            (ICON_REFRESH, true),  // restart
            (ICON_POWER, true),  // shutdown
        ]
    }

    /// Draw one centered power button (fixed size, its own background only —
    /// never spans the card); the hold-to-confirm liquid fill rises inside it.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn power_btn(&mut self, v: &mut Vec<Cmd>, cx: f32, cy: f32, d: f32, i: u32, glyph: &str, danger: bool, pal: &Pal) {
        use crate::shell::POWER_CARD_KEY_BASE;
        let key = POWER_CARD_KEY_BASE + i;
        let hov = self.hover_key == key;
        let holding = self.power_hold == Some(key);
        let x = cx - d / 2.0;
        let y = cy - d / 2.0;
        v.push(Cmd::Rect {
            x,
            y,
            w: d,
            h: d,
            r: self.scale.s(10.0),
            color: if holding {
                ui::acc_tint(&pal)
            } else if hov {
                ui::hover_hl(&pal)
            } else {
                ui::hover(&pal)
            },
        });
        if holding {
            let fill = self.power_hold_fill();
            let fh = d * fill;
            if fh >= self.scale.s(1.5) {
                v.push(Cmd::Rect { x: x + self.scale.s(2.0), y: y + d - self.scale.s(2.0) - fh, w: d - self.scale.s(4.0), h: fh, r: self.scale.s(8.0), color: mix(ui::hover(&pal), pal.acc, 0.5) });
            }
        }
        let gc = if danger { mix(RED, pal.fg, 0.25) } else { pal.fg };
        ui::text_c(v, cx, cy, glyph, self.scale.fs(14.0), gc, true);
        self.region(x, y, d, d, key);
    }

    /// POWER V — a narrow vertical stack of five centered power buttons.
    /// Danger actions use the same hold-to-confirm flow as the power menu.
    pub(crate) fn draw_power_v_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        let hdr = self.scale.s(30.0);
        ui::text(v, x + self.scale.s(12.0), y + self.scale.s(8.0), ICON_BOLT, self.scale.fs(12.0), pal.acc, true);
        ui::text(v, x + self.scale.s(30.0), y + self.scale.s(9.0), "Power", self.scale.fs(12.0), pal.fg, false);

        let d = self.scale.s(32.0);
        let gap = self.scale.s(9.0);
        let stack_h = 5.0 * d + 4.0 * gap;
        let body_h = (h - hdr).max(0.0);
        let base = y + hdr + ((body_h - stack_h).max(0.0) * 0.5);
        let cx = x + w / 2.0;
        for (i, &(glyph, danger)) in Self::power_card_items().iter().enumerate() {
            let cy = base + d / 2.0 + i as f32 * (d + gap);
            self.power_btn(v, cx, cy, d, i as u32, glyph, danger, pal);
        }
    }


    /// POWER H — five centered power buttons in a single row, sized to the
    /// card; same hold-to-confirm flow for the danger actions.
    pub(crate) fn draw_power_h_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        let hdr = self.scale.s(30.0);
        ui::text(v, x + self.scale.s(12.0), y + self.scale.s(8.0), ICON_BOLT, self.scale.fs(12.0), pal.acc, true);
        ui::text(v, x + self.scale.s(30.0), y + self.scale.s(9.0), "Power", self.scale.fs(12.0), pal.fg, false);

        let d = self.scale.s(32.0);
        let gap = self.scale.s(9.0);
        let total = 5.0 * d + 4.0 * gap;
        let body_h = (h - hdr).max(0.0);
        let cx0 = x + (w - total) / 2.0 + d / 2.0;
        let cy = y + hdr + ((body_h - d).max(0.0) * 0.5) + d / 2.0;
        for (i, &(glyph, danger)) in Self::power_card_items().iter().enumerate() {
            let cx = cx0 + i as f32 * (d + gap);
            self.power_btn(v, cx, cy, d, i as u32, glyph, danger, pal);
        }
    }

}
