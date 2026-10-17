use super::super::*;

/// Latency probe host + throttle (ms between pings). The card re-probes every
/// 3 s-ish while packed; each probe pings once and parks for `LAT_PING_MS`.
pub(crate) const LAT_HOST: &str = "1.1.1.1";
pub(crate) const LAT_PING_MS: u64 = 4000;
pub(crate) const LAT_HISTORY: usize = 60;

impl Shell {
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
            let num: String = rest
                .chars()
                .take_while(|c| c.is_ascii_digit() || *c == '.')
                .collect();
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

    #[test]
    fn ping_line_parses() {
        let line = "64 bytes from 1.1.1.1: icmp_seq=1 ttl=58 time=12.3 ms";
        assert_eq!(parse_ping_ms(line), Some(12));
        let idle = "PING 1.1.1.1 (1.1.1.1) 56(84) bytes of data.";
        assert_eq!(parse_ping_ms(idle), None);
        assert_eq!(parse_ping_ms(""), None);
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
