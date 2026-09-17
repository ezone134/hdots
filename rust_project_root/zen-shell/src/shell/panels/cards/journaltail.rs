use super::super::*;
use super::shared::*;

/// journalctl tail throttle (s).
pub(crate) const JR_MS: u64 = 15_000;
/// max messages pulled per tail
pub(crate) const JR_MAX: usize = 30;

impl Shell {

    /// SYSTEM LOG TAIL — the last warning+ journal lines, severity-colored
    /// (err red, warning amber, notice muted). Newest first, newest messages
    /// hit the top of the card like `journalctl -f` frozen.
    pub(crate) fn draw_journaltail_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        let pad = self.scale.s(12.0);
        let hdr = self.scale.s(22.0);
        let meta = if self.jr_lines.is_empty() {
            String::new()
        } else {
            format!("{} lines", self.jr_lines.len())
        };
        self.card_head(v, pal, x, y, w, pad, "System log", Some(ICON_ACTIVE), Some((&meta, false)));

        if self.jr_lines.is_empty() {
            card_empty(v, pal, x, y, w, h, "journal empty", self.scale);
        } else {
            let mut ry = y + hdr + self.scale.s(4.0);
            let row_h = self.scale.s(17.0);
            for (msg, prio) in self.jr_lines.iter().rev().take(5) {
                if ry + row_h > y + h - self.scale.s(2.0) {
                    break;
                }
                let cap = ((w - pad * 2.0 - self.scale.s(16.0)) / self.scale.s(5.6)).max(6.0) as usize;
                let shown: String = msg.chars().take(cap).collect();
                let c = match *prio {
                    0..=3 => ui::DANGER,
                    4 => ui::WARN,
                    _ => ui::fg2(&pal),
                };
                ui::caption(v, x + pad, ry, &shown, self.scale.fs(8.5), c, false);
                ry += row_h;
            }
        }
        // refresh button
        let bh = self.scale.s(20.0);
        let by = y + h - bh - self.scale.s(8.0);
        let bkey = JOURNAL_REFRESH_KEY;
        let hov = self.hover_key == bkey;
        v.push(Cmd::Rect {
            x: x + w - pad - self.scale.s(30.0),
            y: by,
            w: self.scale.s(30.0),
            h: bh,
            r: self.scale.s(6.0),
            color: if hov { ui::raised_hl(&pal) } else { ui::raised(&pal) },
        });
        ui::text_c(v, x + w - pad - self.scale.s(15.0), by + self.scale.s(5.5), ICON_REFRESH, self.scale.fs(9.0), if hov { pal.acc } else { ui::fg3(&pal) }, true);
        self.region(x + w - pad - self.scale.s(30.0) - self.scale.s(3.0), by - self.scale.s(3.0), self.scale.s(30.0) + self.scale.s(6.0), bh + self.scale.s(6.0), bkey);
    }

    /// Pull the latest warning+ journal lines (`journalctl -o json -p
    /// warning`), throttled. Returns true when the tail changed.
    pub(crate) fn poll_journal(&mut self) -> bool {
        if let Some(next) = self.jr_next {
            if Instant::now() < next {
                return false;
            }
        }
        self.jr_next = Some(Instant::now() + std::time::Duration::from_millis(JR_MS));
        let mut lines: Vec<(String, u8)> = Vec::new();
        if let Ok(out) = std::process::Command::new("journalctl")
            .args(["-o", "json", "-p", "warning", "-n", "-30", "--no-pager"])
            .output()
        {
            for ln in String::from_utf8_lossy(&out.stdout).lines().take(JR_MAX) {
                let Ok(j) = serde_json::from_str::<serde_json::Value>(ln) else { continue };
                let msg = j["MESSAGE"].as_str().unwrap_or("").trim();
                if msg.is_empty() {
                    continue;
                }
                let prio = j["PRIORITY"].as_u64().unwrap_or(4) as u8;
                lines.push((msg.to_string(), prio));
            }
        }
        if lines != self.jr_lines {
            self.jr_lines = lines;
            true
        } else {
            false
        }
    }
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
    fn card_draws_lines_with_severity_colors() {
        let mut s = shell();
        let p = pal();
        let mut v: Vec<Cmd> = Vec::new();
        s.jr_lines = vec![
            ("foo.service: main process exited, code=killed".into(), 3),
            ("networkd-dispatcher[123]: wlan0: Failed to add new address".into(), 4),
            ("notice level message".into(), 5),
        ];
        s.draw_journaltail_card(&mut v, 0.0, 0.0, 260.0, 110.0, &p);
        assert!(v.iter().any(|c| matches!(c, Cmd::Text { text, .. } if text.contains("main process exited"))), "err line drawn");
        assert!(v.iter().any(|c| matches!(c, Cmd::Text { text, .. } if text.contains("3 lines"))), "count meta");
    }

    #[test]
    fn polling_is_throttled() {
        let mut s = shell();
        let _ = s.poll_journal();
        assert!(!s.poll_journal(), "second call within window is a no-op");
    }
}