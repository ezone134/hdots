use super::super::*;
use super::shared::*;

/// Latency probe host + throttle (ms between pings). The card re-probes every
/// 3 s-ish while packed; each probe pings once and parks for `LAT_PING_MS`.
pub(crate) const LAT_HOST: &str = "1.1.1.1";
pub(crate) const LAT_PING_MS: u64 = 4000;
pub(crate) const LAT_HISTORY: usize = 60;

impl Shell {

    /// LATENCY — continuous `ping` sparkline to a fixed probe host. The big
    /// right-aligned number is the latest ms, color-coded by threshold
    /// (<60 ok, <120 warn, else danger; grey when idle/err). A run button
    /// forces an immediate re-probe.
    pub(crate) fn draw_latency_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        let pad = self.scale.s(12.0);
        let hdr = self.scale.s(22.0);
        card_header(v, pal, x, y, w, pad, "Latency", Some(ICON_SPEED_FILL), Some((LAT_HOST, true)), self.scale, self.card_show_title, self.card_show_glyph);

        // sparkline of recent probes
        let plot_x = x + pad;
        let plot_y = y + hdr + self.scale.s(10.0);
        let plot_w = w - pad * 2.0 - self.scale.s(58.0);
        let plot_h = y + h - plot_y - self.scale.s(12.0);
        let values: Vec<f32> = self.lat_history.iter().map(|&m| m as f32).collect();
        if values.len() >= 2 {
            let peak = values.iter().fold(120.0_f32, |m, &v| m.max(v)).max(1.0);
            let norm: Vec<f32> = values.iter().map(|&v| (v / peak).clamp(0.0, 1.0)).collect();
            ui::pulse(v, plot_x, plot_y, plot_w, plot_h, &norm, ui::INFO, self.scale.s(2.0));
        } else {
            let yb = plot_y + plot_h * 0.5;
            v.push(Cmd::Line { x0: plot_x, y0: yb, x1: plot_x + plot_w, y1: yb, w: self.scale.s(1.0), color: ui::fg3(&pal) });
        }

        // live readout
        let (val, col) = match self.lat_state {
            0 => (String::from("--"), ui::fg3(&pal)),       // idle
            1 => (String::from("…"), ui::fg3(&pal)),        // probing
            3 => (String::from("ERR"), ui::DANGER),         // no reply
            _ => {
                let v = self.lat_current;
                let c = if v < 60 { ui::OK } else if v < 120 { ui::WARN } else { ui::DANGER };
                (format!("{v}"), c)
            }
        };
        ui::text_r_hero(v, x + w - pad, plot_y + plot_h * 0.5 + self.scale.s(7.0), &val, self.scale.fs(20.0), col);
        ui::caption_r(v, x + w - pad, plot_y + plot_h * 0.5 - self.scale.s(7.0), "ms", self.scale.fs(7.5), ui::fg3(&pal), false);

        // status row + run button
        let bh = self.scale.s(20.0);
        let by = y + h - bh - self.scale.s(8.0);
        let status = match self.lat_state {
            0 => "idle",
            1 => "probing…",
            3 => "no reply",
            _ => {
                if self.lat_current < 60 { "smooth" } else if self.lat_current < 120 { "slow" } else { "bad" }
            }
        };
        ui::caption(v, x + pad, by + self.scale.s(5.0), status, self.scale.fs(8.5), ui::fg2(&pal), false);
        let run_key = LATENCY_RUN_KEY;
        let hov = self.hover_key == run_key;
        v.push(Cmd::Rect {
            x: x + w - pad - self.scale.s(30.0),
            y: by,
            w: self.scale.s(30.0),
            h: bh,
            r: self.scale.s(6.0),
            color: if hov { ui::raised_hl(&pal) } else { ui::raised(&pal) },
        });
        ui::text_c(v, x + w - pad - self.scale.s(15.0), by + self.scale.s(5.5), ICON_REFRESH, self.scale.fs(9.0), if hov { pal.acc } else { ui::fg3(&pal) }, true);
        self.region(x + w - pad - self.scale.s(30.0) - self.scale.s(3.0), by - self.scale.s(3.0), self.scale.s(30.0) + self.scale.s(6.0), bh + self.scale.s(6.0), run_key);
    }

    /// Probe the host once (throttled). Runs on the 3 s services timer and
    /// the card's run button. Returns true when the history/readout changed.
    pub(crate) fn poll_latency(&mut self) -> bool {
        if let Some(next) = self.lat_next {
            if Instant::now() < next {
                return false;
            }
        }
        self.lat_state = 1;
        // `ping -c 1 -W 1`: one packet, 1 s timeout — a dead link costs at
        // most a second once every LAT_PING_MS, never per-frame.
        let out = std::process::Command::new("ping")
            .args(["-c", "1", "-W", "1", LAT_HOST])
            .output()
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
            .unwrap_or_default();
        self.lat_next = Some(Instant::now() + std::time::Duration::from_millis(LAT_PING_MS));
        let mut changed = false;
        if let Some(ms) = parse_ping_ms(&out) {
            if ms != self.lat_current {
                self.lat_current = ms;
                changed = true;
            }
            self.lat_state = 2;
            if self.lat_history.back() != Some(&ms) {
                self.lat_history.push_back(ms);
                while self.lat_history.len() > LAT_HISTORY {
                    self.lat_history.pop_front();
                }
                changed = true;
            }
        } else {
            if self.lat_state != 3 {
                self.lat_state = 3;
                changed = true;
            }
        }
        changed
    }
}

/// Extract `time=NN.N ms` from a ping packet line.
pub(crate) fn parse_ping_ms(out: &str) -> Option<u32> {
    for line in out.lines() {
        if line.contains("bytes from") && line.contains("time=") {
            let rest = line.split("time=").nth(1)?;
            let num: String = rest.chars().take_while(|c| c.is_ascii_digit() || *c == '.').collect();
            return num.parse::<f32>().ok().map(|f| f.round() as u32);
        }
    }
    None
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
    fn ping_line_parses() {
        let line = "64 bytes from 1.1.1.1: icmp_seq=1 ttl=58 time=12.3 ms";
        assert_eq!(parse_ping_ms(line), Some(12));
        let idle = "PING 1.1.1.1 (1.1.1.1) 56(84) bytes of data.";
        assert_eq!(parse_ping_ms(idle), None);
        assert_eq!(parse_ping_ms(""), None);
    }

    #[test]
    fn card_draws_empty_history() {
        let mut s = shell();
        let p = pal();
        let mut v: Vec<Cmd> = Vec::new();
        s.draw_latency_card(&mut v, 0.0, 0.0, 170.0, 110.0, &p);
        assert!(v.iter().any(|c| matches!(c, Cmd::Text { text, .. } if text.contains("--"))), "idle readout");
    }

    #[test]
    fn history_ring() {
        let mut s = shell();
        for i in 0..80 {
            s.lat_state = 2;
            s.lat_current = (i % 100) as u32;
            s.lat_history.push_back((i % 100) as u32);
            while s.lat_history.len() > LAT_HISTORY {
                s.lat_history.pop_front();
            }
        }
        assert_eq!(s.lat_history.len(), LAT_HISTORY, "history capped");
        assert_eq!(*s.lat_history.front().unwrap(), 20, "oldest dropped");
    }
}