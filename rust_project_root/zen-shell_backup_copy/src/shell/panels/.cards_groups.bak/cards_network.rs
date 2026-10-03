use super::*;

impl Shell {


        pub(crate) fn draw_network_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        let dl_col = mix(0x44aaffff, pal.fg, 0.10);
        let up_col = pal.acc;
        // dual-line history graph fills the card above the speed row; the
        // y-axis normalises against the DOWNLOAD bandwidth (its peak = 100%),
        // so a fluctuating link always uses the full plot height
        let row_h = self.scale.s(20.0);
        let gx = x + self.scale.s(12.0);
        let gw = (w - self.scale.s(24.0)).max(20.0);
        let gy = y + self.scale.s(8.0);
        let gh = (h - row_h - self.scale.s(14.0)).max(10.0);
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
            ui::text_r(v, x + w - self.scale.s(14.0), y + self.scale.s(10.0), &mx, self.scale.fs(9.0), pal.fg, false);
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
        ui::text(v, cx, sy, &dtxt, fs, dl_col, false);
        cx += txt_w(&dtxt, fs) + gap;
        ui::text(v, cx, sy, ICON_UP, isz, up_col, true);
        cx += isz + pad_s;
        ui::text(v, cx, sy, &utxt, fs, up_col, false);
    }


    /// WIFI — nearby networks with 4-segment signal bars; a row click
    /// connects (open or OS-agent prompt), the connected row stays tinted.
    /// No data plumbing — reuses the Services `wifi_networks` poll.
    pub(crate) fn draw_wifi_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        self.wifi_rect = (x, y, w, h);
        let pad = self.scale.s(12.0);
        let hdr = self.scale.s(24.0);
        ui::text(v, x + pad, y + self.scale.s(8.0), ICON_WIFI, self.scale.fs(12.0), pal.acc, true);
        ui::text(v, x + pad + self.scale.s(17.0), y + self.scale.s(9.0), "Wi-Fi", self.scale.fs(12.0), pal.fg, false);
        ui::text_r(v, x + w - pad, y + self.scale.s(9.0), format!("{} nets", self.wifi_networks.len()), self.scale.fs(8.5), ui::fg3(&pal), false);

        if self.wifi_networks.is_empty() {
            let msg = if self.wifi_on {
                "no networks — run a scan"
            } else {
                "wi-fi off — enable in toggles"
            };
            ui::text_c(v, x + w / 2.0, y + h / 2.0 - self.scale.s(6.0), msg, self.scale.fs(9.0), ui::fg3(&pal), false);
            return;
        }
        let row_h = self.scale.s(19.0);
        let x0 = x + pad;
        let ww = (w - pad * 2.0).max(20.0);
        let visible = ((h - hdr - self.scale.s(4.0)) / row_h).floor().max(1.0) as usize;
        let len = self.wifi_networks.len();
        self.wifi_scroll = self.wifi_scroll.min(len.saturating_sub(visible));
        let top = self.wifi_scroll;
        let y0 = y + hdr + self.scale.s(2.0);
        for j in 0..visible {
            let i = top + j;
            let Some((ssid, strength, secured, connected)) = self.wifi_networks.get(i) else { break };
            let rj = y0 + j as f32 * row_h;
            let key = crate::shell::WIFI_KEY_BASE + j as u32;
            let hov = self.hover_key == key;
            let connecting = self.pending_wifi.as_deref() == Some(ssid.as_str());
            if *connected {
                v.push(Cmd::Rect { x: x0, y: rj + 1.0, w: ww, h: row_h - 2.0, r: self.scale.s(4.0), color: ui::acc_tint(&pal) });
            } else if hov {
                v.push(Cmd::Rect { x: x0, y: rj + 1.0, w: ww, h: row_h - 2.0, r: self.scale.s(4.0), color: ui::hover(&pal) });
            }
            // 4-segment signal bars
            let bars = (*strength / 25).clamp(0, 4) as usize;
            let bx = x0 + self.scale.s(4.0);
            let bw = self.scale.s(2.5);
            let gap = self.scale.s(2.0);
            let base_y = rj + row_h - self.scale.s(7.0);
            for k in 0..4 {
                let bh = self.scale.s(2.0) + k as f32 * self.scale.s(2.2);
                v.push(Cmd::Rect { x: bx + k as f32 * (bw + gap), y: base_y - bh, w: bw, h: bh, r: 0.5, color: if k < bars { pal.acc } else { ui::hover(&pal) } });
            }
            let tx = bx + 4.0 * (bw + gap) + self.scale.s(5.0);
            let name: String = ssid.chars().take(22).collect();
            let nc = name.chars().count() as f32 * self.scale.fs(9.5) * 0.62;
            ui::text(v, tx, rj + self.scale.s(1.5), &name, self.scale.fs(9.5), if *connected { pal.acc } else { pal.fg }, false);
            if *secured {
                ui::text(v, tx + nc + self.scale.s(6.0), rj + self.scale.s(2.0), ICON_LOCK, self.scale.fs(8.0), ui::fg3(&pal), true);
            }
            if *connected {
                ui::text_r(v, x + w - pad, rj + self.scale.s(2.0), ICON_CHECK, self.scale.fs(9.0), pal.acc, true);
            } else if connecting {
                ui::text_r(v, x + w - pad, rj + self.scale.s(2.0), "…", self.scale.fs(10.0), ui::fg3(&pal), false);
            }
            v.push(Cmd::Rect { x: x0, y: rj + row_h - 1.0, w: ww, h: 1.0, r: 0.5, color: ui::hover(&pal) });
            self.region(x0, rj, ww, row_h, key);
        }
    }


    /// BLUETOOTH — adapter state + device list; row click connects /
    /// disconnects via the existing `pending_bt` plumbing.
    pub(crate) fn draw_bt_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        self.bt_rect = (x, y, w, h);
        let pad = self.scale.s(12.0);
        let hdr = self.scale.s(24.0);
        ui::text(v, x + pad, y + self.scale.s(8.0), ICON_BLUETOOTH, self.scale.fs(12.0), pal.acc, true);
        ui::text(v, x + pad + self.scale.s(17.0), y + self.scale.s(9.0), "Bluetooth", self.scale.fs(12.0), pal.fg, false);
        let chip = if self.bt_on { "on" } else { "off" };
        let chip_col = if self.bt_on { pal.acc } else { ui::fg3(&pal) };
        ui::text_r(v, x + w - pad, y + self.scale.s(9.0), chip, self.scale.fs(8.5), chip_col, false);

        if self.bt_devices.is_empty() {
            let msg = if self.bt_on {
                "no paired devices"
            } else {
                "bluetooth off — enable in toggles"
            };
            ui::text_c(v, x + w / 2.0, y + h / 2.0 - self.scale.s(6.0), msg, self.scale.fs(9.0), ui::fg3(&pal), false);
            return;
        }
        let row_h = self.scale.s(19.0);
        let x0 = x + pad;
        let ww = (w - pad * 2.0).max(20.0);
        let visible = ((h - hdr - self.scale.s(4.0)) / row_h).floor().max(1.0) as usize;
        let len = self.bt_devices.len();
        self.bt_scroll = self.bt_scroll.min(len.saturating_sub(visible));
        let top = self.bt_scroll;
        let y0 = y + hdr + self.scale.s(2.0);
        for j in 0..visible {
            let i = top + j;
            let Some((name, connected, paired)) = self.bt_devices.get(i) else { break };
            let rj = y0 + j as f32 * row_h;
            let key = crate::shell::BT_KEY_BASE + j as u32;
            let hov = self.hover_key == key;
            if *connected {
                v.push(Cmd::Rect { x: x0, y: rj + 1.0, w: ww, h: row_h - 2.0, r: self.scale.s(4.0), color: ui::acc_tint(&pal) });
            } else if hov {
                v.push(Cmd::Rect { x: x0, y: rj + 1.0, w: ww, h: row_h - 2.0, r: self.scale.s(4.0), color: ui::hover(&pal) });
            }
            ui::text(v, x0 + self.scale.s(6.0), rj + self.scale.s(1.5), format!("\u{f293} {}", name), self.scale.fs(9.5), if *connected { pal.acc } else { pal.fg }, false);
            if *connected {
                ui::text_r(v, x + w - pad, rj + self.scale.s(2.0), ICON_CHECK, self.scale.fs(9.0), pal.acc, true);
            } else if !*paired {
                ui::text_r(v, x + w - pad, rj + self.scale.s(2.0), "unpaired", self.scale.fs(8.0), ui::fg3(&pal), false);
            }
            v.push(Cmd::Rect { x: x0, y: rj + row_h - 1.0, w: ww, h: 1.0, r: 0.5, color: ui::hover(&pal) });
            self.region(x0, rj, ww, row_h, key);
        }
    }


    /// SPEED TEST — on-demand Cloudflare bandwidth probe. The button sets
    /// `pending_speed_test`; app.rs runs the worker and reports back, then
    /// this card shows down / up / ping figures + "run again".
    pub(crate) fn draw_speedtest_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        self.speed_rect = (x, y, w, h);
        let pad = self.scale.s(12.0);
        let hdr = self.scale.s(24.0);
        ui::text(v, x + pad, y + self.scale.s(8.0), ICON_SPEEDTEST, self.scale.fs(12.0), pal.acc, true);
        ui::text(v, x + pad + self.scale.s(17.0), y + self.scale.s(9.0), "Speed test", self.scale.fs(12.0), pal.fg, false);
        let (st_lbl, st_col) = match self.speed_state {
            1 => ("testing…", ui::fg3(&pal)),
            2 => ("done", pal.acc),
            _ => ("idle", ui::fg3(&pal)),
        };
        ui::text_r(v, x + w - pad, y + self.scale.s(9.0), st_lbl, self.scale.fs(8.5), st_col, false);

        // stat readout area above the button
        let body_y = y + hdr + self.scale.s(2.0);
        let body_h = (h - hdr - self.scale.s(46.0)).max(10.0);
        if self.speed_state == 1 {
            ui::text_c(v, x + w / 2.0, body_y + body_h / 2.0 - self.scale.s(6.0), "probing Cloudflare edge…", self.scale.fs(9.0), ui::fg3(&pal), false);
        } else if self.speed_state == 2 {
            let cols = [0x44aaffff, pal.acc, ui::fg3(&pal)];
            let labels = [format!("down {}", self.speed_down), format!("up {}", self.speed_up), format!("ping {} ms", self.speed_ping)];
            let cw = w / 3.0;
            for (i, (lbl, col)) in labels.iter().zip(cols.iter()).enumerate() {
                let cxc = x + i as f32 * cw + cw / 2.0;
                ui::text_c(v, cxc, body_y + body_h / 2.0 - self.scale.s(6.0), lbl, self.scale.fs(10.0), *col, false);
            }
        } else {
            ui::text_c(v, x + w / 2.0, body_y + body_h / 2.0 - self.scale.s(6.0), "click to measure", self.scale.fs(9.0), ui::fg3(&pal), false);
        }

        // run button
        let bx = x + pad;
        let bw = w - pad * 2.0;
        let bh = self.scale.s(30.0);
        let by = y + h - bh - self.scale.s(8.0);
        let key = crate::shell::SPEED_KEY_BASE;
        let hov = self.hover_key == key;
        let running = self.speed_state == 1;
        let bg = if running { mix(ui::hover(&pal), pal.fg, 0.10) } else if hov { mix(pal.acc, pal.fg, 0.15) } else { pal.acc };
        v.push(Cmd::Rect { x: bx, y: by, w: bw, h: bh, r: bh / 2.0, color: bg });
        let label = if running { "testing…" } else if self.speed_state == 2 { "\u{f021}  run again" } else { "\u{f04b}  run test" };
        let tc = if running { ui::fg3(&pal) } else { 0xff141414 };
        ui::text_c(v, bx + bw / 2.0, by + bh / 2.0 - self.scale.s(6.0), label, self.scale.fs(10.5), tc, false);
        if !running {
            self.region(bx, by, bw, bh, key);
        }
    }


    /// RECENT — recently opened files from `~/.local/share/recently-used.xbel`
    /// (cached ~30 s); a row click opens the file.
    pub(crate) fn draw_recent_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        self.recent_rect = (x, y, w, h);
        if self.recent_loaded_at.elapsed().as_secs() > 30 {
            self.recent_files = Self::read_recent_files();
            self.recent_loaded_at = std::time::Instant::now();
        }
        let pad = self.scale.s(12.0);
        let hdr = self.scale.s(24.0);
        ui::text(v, x + pad, y + self.scale.s(8.0), ICON_RECENT, self.scale.fs(12.0), pal.acc, true);
        ui::text(v, x + pad + self.scale.s(17.0), y + self.scale.s(9.0), "Recent files", self.scale.fs(12.0), pal.fg, false);
        ui::text_r(v, x + w - pad, y + self.scale.s(9.0), format!("{} files", self.recent_files.len()), self.scale.fs(8.5), ui::fg3(&pal), false);

        if self.recent_files.is_empty() {
            ui::text_c(v, x + w / 2.0, y + h / 2.0 - self.scale.s(6.0), "no recent files tracked", self.scale.fs(9.0), ui::fg3(&pal), false);
            return;
        }
        let row_h = self.scale.s(19.0);
        let x0 = x + pad;
        let ww = (w - pad * 2.0).max(20.0);
        let visible = ((h - hdr - self.scale.s(4.0)) / row_h).floor().max(1.0) as usize;
        let len = self.recent_files.len();
        self.recent_scroll = self.recent_scroll.min(len.saturating_sub(visible));
        let top = self.recent_scroll;
        let y0 = y + hdr + self.scale.s(2.0);
        for j in 0..visible {
            let i = top + j;
            let Some((name, _path)) = self.recent_files.get(i) else { break };
            let rj = y0 + j as f32 * row_h;
            let key = crate::shell::RECENT_KEY_BASE + j as u32;
            let hov = self.hover_key == key;
            if hov {
                v.push(Cmd::Rect { x: x0, y: rj + 1.0, w: ww, h: row_h - 2.0, r: self.scale.s(4.0), color: ui::hover(&pal) });
            }
            let nm: String = name.chars().take(24).collect();
            ui::text(v, x0 + self.scale.s(6.0), rj + self.scale.s(1.5), &nm, self.scale.fs(9.5), if hov { pal.acc } else { pal.fg }, false);
            ui::text_r(v, x + w - pad, rj + self.scale.s(1.5), ICON_OPEN, self.scale.fs(8.0), if hov { pal.acc } else { ui::fg3(&pal) }, true);
            v.push(Cmd::Rect { x: x0, y: rj + row_h - 1.0, w: ww, h: 1.0, r: 0.5, color: ui::hover(&pal) });
            self.region(x0, rj, ww, row_h, key);
        }
    }


    /// CURRENCY — static reference rates; a row click re-bases to that
    /// currency (1 USD ↔ …). No network — offline values with a note.
    pub(crate) fn draw_currency_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        const TBL: [(&str, &str, f64); 12] = [
            ("USD", "Dollar", 1.0),
            ("EUR", "Euro", 0.9200),
            ("GBP", "Pound", 0.7900),
            ("JPY", "Yen", 149.50),
            ("INR", "Rupee", 83.10),
            ("CNY", "Yuan", 7.2400),
            ("RUB", "Ruble", 92.50),
            ("AUD", "AU Dollar", 1.5200),
            ("CAD", "CA Dollar", 1.3700),
            ("KRW", "Won", 1347.00),
            ("XAU", "Gold oz", 0.000420),
            ("XBT", "BTC", 0.0000160),
        ];
        self.currency_rect = (x, y, w, h);
        let pad = self.scale.s(12.0);
        let hdr = self.scale.s(24.0);
        ui::text(v, x + pad, y + self.scale.s(8.0), ICON_MONEY, self.scale.fs(12.0), pal.acc, true);
        ui::text(v, x + pad + self.scale.s(17.0), y + self.scale.s(9.0), "Currency", self.scale.fs(12.0), pal.fg, false);
        let base = self.currency_base.clone();
        let base_mult = TBL.iter().find(|(s, _, _)| *s == base.as_str()).map(|(_, _, m)| *m).unwrap_or(1.0);
        if self.currency_base != "USD" {
            ui::text_r(v, x + w - pad, y + self.scale.s(9.0), &self.currency_base, self.scale.fs(8.5), pal.acc, false);
        }

        let row_h = self.scale.s(20.0);
        let x0 = x + pad;
        let ww = (w - pad * 2.0).max(20.0);
        let visible = ((h - hdr - self.scale.s(18.0)) / row_h).floor().max(1.0) as usize;
        let len = TBL.len();
        self.currency_scroll = self.currency_scroll.min(len.saturating_sub(visible));
        let top = self.currency_scroll;
        let y0 = y + hdr + self.scale.s(2.0);
        for j in 0..visible {
            let i = top + j;
            let Some((sym, label, mult)) = TBL.get(i).copied() else { break };
            let rj = y0 + j as f32 * row_h;
            let key = crate::shell::CURRENCY_KEY_BASE + j as u32;
            let hov = self.hover_key == key;
            let active = sym == base.as_str();
            if active {
                v.push(Cmd::Rect { x: x0, y: rj + 1.0, w: ww, h: row_h - 2.0, r: self.scale.s(4.0), color: ui::acc_tint(&pal) });
            } else if hov {
                v.push(Cmd::Rect { x: x0, y: rj + 1.0, w: ww, h: row_h - 2.0, r: self.scale.s(4.0), color: ui::hover(&pal) });
            }
            let val = mult / base_mult;
            let vstr = if val >= 100.0 { format!("{:.0}", val) } else if val >= 10.0 { format!("{:.1}", val) } else { format!("{:.2}", val) };
            ui::text(v, x0 + self.scale.s(6.0), rj + self.scale.s(1.5), sym, self.scale.fs(10.0), if active { pal.acc } else { pal.fg }, false);
            ui::text(v, x0 + self.scale.s(6.0) + self.scale.s(30.0), rj + self.scale.s(2.0), label, self.scale.fs(8.0), ui::fg3(&pal), false);
            ui::text_r(v, x + w - pad, rj + self.scale.s(1.5), format!("1 {base} = {vstr} {sym}"), self.scale.fs(9.5), if active { pal.acc } else { pal.fg }, false);
            self.region(x0, rj, ww, row_h, key);
        }
        // footer note
        ui::text(v, x + pad, y + h - self.scale.s(12.0), "offline static rates", self.scale.fs(7.5), ui::fg3(&pal), false);
        self.currency_note = "static".to_string();
    }


    /// QUOTE — quote of the day with two pills: refresh (⟳) and save (→
    /// `$states/quotes`). Save flashes a green "saved ✓" and bumps the shelf.
    pub(crate) fn draw_quote_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        self.quote_rect = (x, y, w, h);
        let pad = self.scale.s(12.0);
        let hdr = self.scale.s(24.0);
        ui::text(v, x + pad, y + self.scale.s(8.0), ICON_QUOTE, self.scale.fs(12.0), pal.acc, true);
        ui::text(v, x + pad + self.scale.s(17.0), y + self.scale.s(9.0), "Quote", self.scale.fs(12.0), pal.fg, false);
        if self.quote_state == 2 && self.quote_saved_flash > 0 {
            ui::text_r(v, x + w - pad, y + self.scale.s(9.0), "\u{f00c} saved", self.scale.fs(8.5), 0x2ee6a8ff, false);
        } else {
            let (sl, sc) = match self.quote_state {
                1 => ("fetching…".to_string(), ui::fg3(&pal)),
                2 => (format!("{} saved", self.saved_quotes.len()), ui::fg3(&pal)),
                _ => ("—".to_string(), ui::fg3(&pal)),
            };
            ui::text_r(v, x + w - pad, y + self.scale.s(9.0), &sl, self.scale.fs(8.5), sc, false);
        }
        if self.quote_saved_flash > 0 {
            self.quote_saved_flash -= 1;
        }
        let has_quote = self.quote_state == 2 && !self.quote_text.is_empty();

        // body: wrapped quote text + author
        let body_y = y + hdr + self.scale.s(4.0);
        let body_h = (h - hdr - self.scale.s(40.0)).max(10.0);
        if self.quote_state == 1 {
            ui::text_c(v, x + w / 2.0, body_y + body_h / 2.0 - self.scale.s(6.0), "fetching a quote…", self.scale.fs(9.0), ui::fg3(&pal), false);
        } else if has_quote {
            let raw: String = self.quote_text.chars().take(136).collect();
            let mut lines: Vec<String> = Vec::new();
            let mut cur = String::new();
            for c in raw.chars() {
                cur.push(c);
                if cur.chars().count() >= 34 {
                    lines.push(std::mem::take(&mut cur));
                }
            }
            if !cur.is_empty() {
                lines.push(cur);
            }
            let line_h = self.scale.s(13.0);
            let start = body_y + (body_h - line_h * lines.len() as f32) / 2.0 - self.scale.s(4.0);
            for (li, l) in lines.iter().enumerate() {
                ui::text_c(v, x + w / 2.0, start + li as f32 * line_h, l, self.scale.fs(10.0), pal.fg, false);
            }
            if !self.quote_author.is_empty() {
                ui::text_c(v, x + w / 2.0, start + lines.len() as f32 * line_h + self.scale.s(2.0), format!("— {}", self.quote_author), self.scale.fs(8.5), ui::fg3(&pal), false);
            }
        } else {
            ui::text_c(v, x + w / 2.0, body_y + body_h / 2.0 - self.scale.s(6.0), "no quote yet", self.scale.fs(9.0), ui::fg3(&pal), false);
        }

        // two pills
        let pw = self.scale.s(88.0);
        let ph = self.scale.s(26.0);
        let gap = self.scale.s(12.0);
        let py = y + h - ph - self.scale.s(8.0);
        let total = pw * 2.0 + gap;
        let px = x + (w - total) / 2.0;
        let k_refresh = crate::shell::QUOTE_KEY_BASE;
        let k_save = crate::shell::QUOTE_SAVE_KEY;
        let hov_r = self.hover_key == k_refresh;
        let hov_s = self.hover_key == k_save;
        let save_ok = has_quote;
        v.push(Cmd::Rect { x: px, y: py, w: pw, h: ph, r: ph / 2.0, color: if hov_r { mix(pal.acc, pal.fg, 0.15) } else { ui::hover_hl(&pal) } });
        ui::text_c(v, px + pw / 2.0, py + ph / 2.0 - self.scale.s(5.0), "\u{f01e}  new", self.scale.fs(10.0), if hov_r { pal.acc } else { pal.fg }, false);
        self.region(px, py, pw, ph, k_refresh);
        v.push(Cmd::Rect { x: px + pw + gap, y: py, w: pw, h: ph, r: ph / 2.0, color: if save_ok { if hov_s { mix(0x2ee6a8ff, pal.fg, 0.15) } else { 0x2ee6a8ff } } else { ui::hover(&pal) } });
        ui::text_c(v, px + pw + gap + pw / 2.0, py + ph / 2.0 - self.scale.s(5.0), "\u{f0c7}  save", self.scale.fs(10.0), if save_ok { 0xff141414 } else { ui::fg3(&pal) }, false);
        if save_ok {
            self.region(px + pw + gap, py, pw, ph, k_save);
        }
    }


    /// PRICES — crypto ticker rows (CoinGecko); a row click opens the asset's
    /// market page. Numeric rows, News-style.
    pub(crate) fn draw_ticker_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        self.ticker_rect = (x, y, w, h);
        const GREEN: u32 = 0x2ee6a8ff;
        let pad = self.scale.s(12.0);
        let hdr = self.scale.s(24.0);
        ui::text(v, x + pad, y + self.scale.s(8.0), ICON_TICKER, self.scale.fs(12.0), pal.acc, true);
        ui::text(v, x + pad + self.scale.s(17.0), y + self.scale.s(9.0), "Prices", self.scale.fs(12.0), pal.fg, false);
        ui::text_r(v, x + w - pad, y + self.scale.s(9.0), "crypto", self.scale.fs(8.5), ui::fg3(&pal), false);

        if self.ticker_items.is_empty() {
            let msg = if self.ticker_state == 1 { "loading prices…" } else { "no price data" };
            ui::text_c(v, x + w / 2.0, y + h / 2.0 - self.scale.s(6.0), msg, self.scale.fs(9.0), ui::fg3(&pal), false);
            return;
        }
        let row_h = self.scale.s(18.0);
        let x0 = x + pad;
        let ww = (w - pad * 2.0).max(20.0);
        let visible = ((h - hdr - self.scale.s(4.0)) / row_h).floor().max(1.0) as usize;
        let len = self.ticker_items.len();
        self.ticker_scroll = self.ticker_scroll.min(len.saturating_sub(visible));
        let top = self.ticker_scroll;
        let y0 = y + hdr + self.scale.s(2.0);
        for j in 0..visible {
            let i = top + j;
            let Some((sym, price, chg)) = self.ticker_items.get(i) else { break };
            let rj = y0 + j as f32 * row_h;
            let key = crate::shell::TICKER_KEY_BASE + j as u32;
            let hov = self.hover_key == key;
            if hov {
                v.push(Cmd::Rect { x: x0, y: rj + 1.0, w: ww, h: row_h - 2.0, r: self.scale.s(4.0), color: ui::hover(&pal) });
            }
            ui::text(v, x0 + self.scale.s(6.0), rj + self.scale.s(1.0), sym, self.scale.fs(9.5), pal.fg, false);
            ui::text(v, x0 + self.scale.s(6.0) + self.scale.s(56.0), rj + self.scale.s(1.0), price, self.scale.fs(9.5), if hov { pal.acc } else { ui::fg2(&pal) }, false);
            let cc = if *chg >= 0.0 { GREEN } else { RED };
            ui::text_r(v, x + w - pad, rj + self.scale.s(1.0), format!("{:+.1}%", chg), self.scale.fs(9.0), cc, false);
            v.push(Cmd::Rect { x: x0, y: rj + row_h - 1.0, w: ww, h: 1.0, r: 0.5, color: ui::hover(&pal) });
            self.region(x0, rj, ww, row_h, key);
        }
    }

}
