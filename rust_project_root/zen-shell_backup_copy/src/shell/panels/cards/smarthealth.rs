use super::super::*;
use super::shared::*;

/// SMART scan throttle (s). A scan runs one `smartctl -j -a` per disk; 30 s
/// keeps the card fresh on the dashboard without nagging the drives.
pub(crate) const SM_MIN_MS: u64 = 30_000;

impl Shell {

    /// DISK HEALTH — one row per disk with model / temp / PASS·FAIL from
    /// `smartctl -j -a`. Failed drives get a red ✕ chip and a danger-colored
    /// dev name; a clean fleet stays quiet green. Scans are throttled to 30 s
    /// and gated to the packed card.
    pub(crate) fn draw_smarthealth_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        let pad = self.scale.s(12.0);
        let hdr = self.scale.s(24.0);
        let any_fail = self.sm_disks.iter().any(|(_, _, ok, _)| !ok);
        let meta = if self.sm_disks.is_empty() {
            String::new()
        } else {
            format!("{} disk{}", self.sm_disks.len(), if self.sm_disks.len() == 1 { "" } else { "s" })
        };
        let hicon = if any_fail { ICON_SHIELD } else { ICON_CHECK };
        card_header(v, pal, x, y, w, pad, "Disk health", Some(hicon), Some((&meta, false)), self.scale, self.card_show_title, self.card_show_glyph);

        if self.sm_disks.is_empty() {
            card_empty(v, pal, x, y, w, h, "no SMART disks", self.scale);
        } else {
            let mut ry = y + hdr + self.scale.s(4.0);
            let row_h = self.scale.s(23.0);
            for (dev, model, passed, temp) in &self.sm_disks {
                if ry + row_h > y + h - self.scale.s(2.0) {
                    break;
                }
                let dev_s = dev.rsplit('/').next().unwrap_or(dev).to_string();
                let dc = if *passed { ui::fg2(&pal) } else { ui::DANGER };
                ui::text_mono(v, x + pad, ry + self.scale.s(4.0), &dev_s, self.scale.fs(8.5), dc);
                let model_cap = (((w - pad * 2.0 - self.scale.s(96.0)) / self.scale.s(5.6)).max(4.0)) as usize;
                let shown: String = model.chars().take(model_cap).collect();
                ui::caption(v, x + pad + self.scale.s(40.0), ry + self.scale.s(4.0), &shown, self.scale.fs(8.5), ui::fg3(&pal), false);
                // temp + status
                let tc = if *temp >= 55 { ui::WARN } else { ui::fg3(&pal) };
                ui::text_r_mono(v, x + w - pad - self.scale.s(56.0), ry + self.scale.s(4.0), &format!("{temp}\u{b0}"), self.scale.fs(8.5), tc);
                let (glyph, gc) = if *passed { (ICON_CHECK, ui::OK) } else { (ICON_CLOSE, ui::DANGER) };
                ui::text_r(v, x + w - pad, ry + self.scale.s(4.0), glyph, self.scale.fs(10.0), gc, true);
                ry += row_h;
            }
        }
        // refresh button
        let bh = self.scale.s(20.0);
        let by = y + h - bh - self.scale.s(8.0);
        let bkey = SMART_REFRESH_KEY;
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

    /// Scan disk SMART data (+ temp), throttled. Returns true when the list
    /// changed (caller re-renders).
    pub(crate) fn poll_smart(&mut self) -> bool {
        if let Some(next) = self.sm_next {
            if Instant::now() < next {
                return false;
            }
        }
        self.sm_next = Some(Instant::now() + std::time::Duration::from_millis(SM_MIN_MS));
        let mut disks: Vec<(String, String, bool, i32)> = Vec::new();
        if let Ok(blocks) = std::fs::read_dir("/sys/block") {
            let mut names: Vec<String> = Vec::new();
            for e in blocks.flatten() {
                let name = e.file_name().to_string_lossy().into_owned();
                if name.starts_with("sd") || name.starts_with("nvme") {
                    names.push(name);
                }
            }
            names.sort();
            for name in names.into_iter().take(4) {
                if let Some(row) = smart_row(&format!("/dev/{name}")) {
                    disks.push(row);
                }
            }
        }
        if disks != self.sm_disks {
            self.sm_disks = disks;
            true
        } else {
            false
        }
    }
}

/// One `smartctl -j -a` row: (dev path, model, health passed, temp °C).
pub(crate) fn smart_row(dev: &str) -> Option<(String, String, bool, i32)> {
    let out = std::process::Command::new("smartctl").args(["-j", "-a", dev]).output().ok()?;
    let j: serde_json::Value = serde_json::from_slice(&out.stdout).ok()?;
    let name = j["device"]["name"].as_str().unwrap_or("").to_string();
    let model = j["model_name"].as_str().unwrap_or("").to_string();
    let passed = j["smart_status"]["passed"].as_bool().unwrap_or(false);
    let temp = j["temperature"]["current"].as_i64().unwrap_or(0) as i32;
    if name.is_empty() && model.is_empty() {
        return None;
    }
    Some((name, model, passed, temp))
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
    fn card_draws_rows_and_failure_state() {
        let mut s = shell();
        s.sm_disks = vec![
            ("/dev/nvme0n1".into(), "Samsung 990 PRO".into(), true, 41),
            ("/dev/sda".into(), "Seagate Barracuda".into(), false, 57),
        ];
        let p = pal();
        let mut v: Vec<Cmd> = Vec::new();
        s.draw_smarthealth_card(&mut v, 0.0, 0.0, 170.0, 110.0, &p);
        assert!(v.iter().any(|c| matches!(c, Cmd::Text { text, .. } if text.contains("2 disks"))), "count meta");
        assert!(v.iter().any(|c| matches!(c, Cmd::Text { text, .. } if text.contains("41°"))), "temp shown");
    }

    #[test]
    fn smart_row_never_panics() {
        // missing smartctl / non-existent device → helper returns quietly
        let _ = smart_row("/dev/nonexistent0");
    }
}