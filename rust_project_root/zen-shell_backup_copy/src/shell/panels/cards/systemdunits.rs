use super::super::*;
use super::shared::*;

/// systemctl scan throttle (s).
pub(crate) const SUS_MS: u64 = 20_000;

impl Shell {

    /// FAILED SYSTEMD UNITS — red list of units in `failed` state (system +
    /// user scopes). Clean fleet = a green "all services up" row. Scans are
    /// throttled and gated to the packed card.
    pub(crate) fn draw_systemdunits_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        let pad = self.scale.s(12.0);
        let hdr = self.scale.s(22.0);
        let n = self.sus_failed.len();
        let hicon = if n > 0 { ICON_SHIELD } else { ICON_CHECK };
        let meta = if n > 0 {
            format!("{n} failed")
        } else {
            String::new()
        };
        card_header(v, pal, x, y, w, pad, "Services", Some(hicon), Some((&meta, false)), self.scale, self.card_show_title, self.card_show_glyph);

        if self.sus_failed.is_empty() {
            card_empty(v, pal, x, y, w, h, "all services up", self.scale);
        } else {
            let mut ry = y + hdr + self.scale.s(6.0);
            let row_h = self.scale.s(19.0);
            let tail: std::collections::VecDeque<String> = self.sus_failed.iter().rev().take(5).cloned().collect();
            for unit in tail {
                if ry + row_h > y + h - self.scale.s(4.0) {
                    break;
                }
                let shown: String = unit.chars().take(((w - pad * 2.0 - self.scale.s(20.0)) / self.scale.s(5.6)).max(6.0) as usize).collect();
                v.push(Cmd::Rect { x: x + pad, y: ry, w: w - pad * 2.0, h: self.scale.s(15.0), r: self.scale.s(4.0), color: ui::raised(&pal) });
                ui::text(v, x + pad + self.scale.s(6.0), ry + self.scale.s(3.5), &shown, self.scale.fs(8.5), ui::DANGER, false);
                ui::text_r(v, x + w - pad - self.scale.s(14.0), ry + self.scale.s(3.5), ICON_CLOSE, self.scale.fs(9.0), ui::DANGER, true);
                ry += row_h;
            }
        }
        // refresh button
        let bh = self.scale.s(20.0);
        let by = y + h - bh - self.scale.s(8.0);
        let bkey = SYSTEMD_REFRESH_KEY;
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

    /// List failed units (system then user manager), throttled. Returns true
    /// when the list changed.
    pub(crate) fn poll_systemd(&mut self) -> bool {
        if let Some(next) = self.sus_next {
            if Instant::now() < next {
                return false;
            }
        }
        self.sus_next = Some(Instant::now() + std::time::Duration::from_millis(SUS_MS));
        let mut failed: Vec<String> = Vec::new();
        for extra in [Vec::<&str>::new(), vec!["--user"]] {
            let mut cmd = std::process::Command::new("systemctl");
            cmd.args(&extra);
            cmd.args(["list-units", "--type=service", "--state=failed", "--no-legend", "--no-pager", "-o", "json"]);
            let Ok(out) = cmd.output() else { continue };
            let j: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap_or(serde_json::Value::Null);
            if let Some(arr) = j.as_array() {
                for u in arr {
                    if let Some(name) = u["unit"].as_str() {
                        failed.push(name.to_string());
                    }
                }
            }
        }
        // dedupe across scopes, keep order
        let mut seen = std::collections::HashSet::new();
        failed.retain(|u| seen.insert(u.clone()));
        if failed != self.sus_failed {
            self.sus_failed = failed;
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
    fn card_draws_clean_and_failed() {
        let mut s = shell();
        let p = pal();
        let mut v: Vec<Cmd> = Vec::new();
        s.draw_systemdunits_card(&mut v, 0.0, 0.0, 230.0, 110.0, &p);
        s.sus_failed = vec![
            "netctl@wlan.service".into(),
            "smbd.service".into(),
            "lightdm.service long enough to overflow the chip".into(),
        ];
        s.draw_systemdunits_card(&mut v, 0.0, 0.0, 230.0, 110.0, &p);
        assert!(v.iter().any(|c| matches!(c, Cmd::Text { text, .. } if text.contains("3 failed"))), "count meta");
    }

    #[test]
    fn polling_is_throttled() {
        let mut s = shell();
        let _ = s.poll_systemd();
        assert!(!s.poll_systemd(), "second call within window is a no-op");
    }
}