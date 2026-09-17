use super::super::*;

impl Shell {

    /// WORLD CLOCK — pinned cities with their local time: one row per zone
    /// (city · mono HH:MM · sun/moon glyph · offset-vs-local chip), a "+"
    /// row opening an inline zone.tab search, and hover ✕ per row. Offsets
    /// are a cache refreshed by the app's batched `TZ=<tz> date +%z %Z`
    /// probe; HH:MM/dayline/night are computed at draw time from the offset
    /// (pure libc, no per-frame subprocesses).
    pub(crate) fn draw_worldclock_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        self.worldclock_rect = (x, y, w, h);
        let pad = self.scale.s(12.0);
        let hdr = self.scale.s(24.0);
        let n = self.worldclock_zones.len();
        let meta = if n > 0 { format!("{n} cities") } else { String::new() };
        self.card_head(v, pal, x, y, w, pad, "World clock", Some(ICON_CLOCK), Some((&meta, false)));
        let now_epoch = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
        let local_off = Self::local_utc_offset_min(now_epoch);

        if self.worldclock_search.is_some() {
            self.draw_worldclock_search(v, x, y, w, h, pad, hdr, pal);
            return;
        }

        let row_h = self.scale.s(26.0);
        let y0 = y + hdr + self.scale.s(4.0);
        let max_rows = (((y + h - self.scale.s(8.0)) - y0) / row_h).floor().max(0.0) as usize;
        // per-row data snapshotted first so the ✕ hit-region registration
        // below can borrow self mutably
        let rows: Vec<(usize, String, i32)> = self
            .worldclock_zones
            .iter()
            .enumerate()
            .map(|(i, (city, _, off, _))| (i, city.clone(), *off))
            .collect();
        for (i, city, off) in rows.iter() {
            if *i >= max_rows.saturating_sub(1) && n >= max_rows {
                // reserve the last slot for the "+" row
                break;
            }
            let ry = y0 + *i as f32 * row_h;
            // day/night from the shifted local hour (6..=17 = day)
            let hour = Self::shifted_hour(now_epoch, *off);
            let (glyph, gcol) = if (6..=17).contains(&hour) {
                (ICON_SUNNY, ui::WARN)
            } else {
                (ICON_MOON, ui::fg3(&pal))
            };
            ui::text(v, x + pad, ry + self.scale.s(1.0), glyph, self.scale.fs(10.0), gcol, true);
            let name: String = city.chars().take(14).collect();
            ui::text(v, x + pad + self.scale.s(16.0), ry + self.scale.s(1.0), &name, self.scale.fs(9.5), pal.fg, false);
            ui::text_r_mono(v, x + w - pad - self.scale.s(34.0), ry + self.scale.s(1.0), worldmap::world_hhmm(now_epoch, *off), self.scale.fs(9.5), pal.fg);
            // offset-vs-local chip (hidden for the local zone itself: ±00:00)
            let d = *off - local_off;
            if d != 0 {
                let chip = format!("{}{}", if d > 0 { "+" } else { "" }, fmt_hhmm_min(d));
                ui::caption_r(v, x + w - pad, ry + self.scale.s(2.0), &chip, self.scale.fs(7.5), ui::fg3(&pal), false);
            }
            // hover ✕
            let del_key = WORLDCLOCK_DEL_BASE + *i as u32;
            let row_hov = self.hover_key == del_key;
            if row_hov {
                ui::text(v, x + w - pad - self.scale.s(56.0), ry + self.scale.s(1.0), ICON_CLOSE, self.scale.fs(9.0), RED, true);
                self.region(x + w - pad - self.scale.s(58.0), ry - self.scale.s(2.0), self.scale.s(22.0), row_h, del_key);
            }
        }
        // "+" add row (opens the inline search)
        let add_y = y0 + n.min(max_rows.saturating_sub(1)) as f32 * row_h;
        if add_y + row_h <= y + h - 2.0 {
            let add_hov = self.hover_key == WORLDCLOCK_ADD_KEY;
            ui::text(v, x + pad, add_y + self.scale.s(1.0), ICON_ADD, self.scale.fs(10.0), if add_hov { pal.acc } else { ui::fg3(&pal) }, true);
            ui::caption(v, x + pad + self.scale.s(16.0), add_y + self.scale.s(2.0), "add city", self.scale.fs(8.5), if add_hov { pal.acc } else { ui::fg3(&pal) }, false);
            self.region(x + pad, add_y, w - pad * 2.0, row_h, WORLDCLOCK_ADD_KEY);
        }
    }

    /// The inline search overlay: input row + up to 6 zone.tab matches
    /// (city · tz path); click a result to pin it.
    fn draw_worldclock_search(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pad: f32, hdr: f32, pal: &Pal) {
        let buf = self.worldclock_search.clone().unwrap_or_default();
        // input row
        let iy = y + hdr + self.scale.s(4.0);
        v.push(Cmd::Rect { x: x + pad, y: iy, w: w - pad * 2.0, h: self.scale.s(24.0), r: self.scale.s(6.0), color: ui::hover(&pal) });
        let shown = if buf.is_empty() { "type a city…".to_string() } else { buf.clone() };
        ui::text(v, x + pad + self.scale.s(8.0), iy + self.scale.s(6.0), shown, self.scale.fs(9.5), if buf.is_empty() { ui::fg3(&pal) } else { pal.fg }, false);
        ui::caption_r(v, x + w - pad - self.scale.s(8.0), iy + self.scale.s(7.0), "esc", self.scale.fs(7.5), ui::fg3(&pal), false);
        // matches (computed fresh; zone.tab parse is a cached OnceLock)
        let needle = buf.to_lowercase();
        let ry0 = iy + self.scale.s(30.0);
        let row_h = self.scale.s(24.0);
        if needle.len() >= 2 {
            let mut shown = 0usize;
            for z in worldmap::zones() {
                if shown >= 6 || ry0 + shown as f32 * row_h > y + h - 2.0 {
                    break;
                }
                let city_lc = z.city.to_lowercase();
                let tz_lc = z.tz.to_lowercase();
                if !city_lc.contains(&needle) && !tz_lc.contains(&needle) {
                    continue;
                }
                let key = WORLDCLOCK_PICK_BASE + shown as u32;
                let hov = self.hover_key == key;
                if hov {
                    v.push(Cmd::Rect { x: x + pad, y: ry0 + shown as f32 * row_h, w: w - pad * 2.0, h: row_h - 2.0, r: self.scale.s(5.0), color: ui::hover_hl(&pal) });
                }
                let city: String = z.city.chars().take(16).collect();
                ui::text(v, x + pad + self.scale.s(4.0), ry0 + shown as f32 * row_h + self.scale.s(4.0), city, self.scale.fs(9.0), pal.fg, false);
                let tz: String = z.tz.chars().take(20).collect();
                ui::caption_r(v, x + w - pad - self.scale.s(4.0), ry0 + shown as f32 * row_h + self.scale.s(5.0), &tz, self.scale.fs(7.5), ui::fg3(&pal), false);
                // result row maps pick-index → zone index
                self.worldclock_pick_map.push((key, z.tz.clone(), z.city.clone()));
                self.region(x + pad, ry0 + shown as f32 * row_h, w - pad * 2.0, row_h, key);
                shown += 1;
            }
            if shown == 0 {
                ui::caption_c(v, x + w / 2.0, ry0 + self.scale.s(6.0), "no matches", self.scale.fs(8.5), ui::fg3(&pal), false);
            }
        }
    }

    /// Pin a zone from the search results (dedup by tz; persists).
    pub(crate) fn worldclock_pin(&mut self, tz: &str, city: &str) {
        if self.worldclock_zones.iter().any(|(_, t, _, _)| t == tz) {
            return;
        }
        if self.worldclock_zones.len() >= 16 {
            return;
        }
        self.worldclock_zones.push((city.to_string(), tz.to_string(), 0, String::new()));
        self.worldclock_probed_at = None; // force a probe of the new zone
        self.save_worldclock_zones();
    }

    /// Remove pinned row `i`.
    pub(crate) fn worldclock_remove(&mut self, i: usize) {
        if i < self.worldclock_zones.len() {
            self.worldclock_zones.remove(i);
            self.save_worldclock_zones();
        }
    }

    /// UTC-offset (minutes) of the machine's local zone at `epoch` — via
    /// localtime_r's tm_gmtoff, no subprocess.
    pub(crate) fn local_utc_offset_min(epoch: i64) -> i32 {
        unsafe {
            let t = epoch as libc::time_t;
            let mut tm: libc::tm = std::mem::zeroed();
            libc::localtime_r(&t, &mut tm);
            (tm.tm_gmtoff / 60) as i32
        }
    }

    /// Local wall-hour (0..23) at a shifted offset — drives day/night.
    pub(crate) fn shifted_hour(epoch: i64, off_min: i32) -> u32 {
        let shifted = epoch + off_min as i64 * 60;
        unsafe {
            let t = shifted as libc::time_t;
            let mut tm: libc::tm = std::mem::zeroed();
            libc::gmtime_r(&t, &mut tm);
            tm.tm_hour as u32
        }
    }

}

/// "+5:45" / "-3:00" style offset formatting from signed minutes.
fn fmt_hhmm_min(mins: i32) -> String {
    let a = mins.abs();
    format!("{}:{:02}", a / 60, a % 60)
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
    fn offset_math_and_hour_shift() {
        // fixed epochs: 2026-01-01 00:00 UTC
        let epoch = 1767225600;
        assert_eq!(Shell::local_utc_offset_min(epoch).abs() % 15, 0, "offset is a multiple of 15 min");
        assert_eq!(Shell::shifted_hour(epoch, 5 * 60 + 45), 5, "UTC+5:45 at midnight UTC → 05:xx");
        assert_eq!(Shell::shifted_hour(epoch, -5 * 60), 19, "UTC-5 at midnight UTC → 19:xx");
    }

    #[test]
    fn pin_dedups_and_persists() {
        let mut s = shell();
        s.per_state_config = std::env::temp_dir().join("zen-test-worldclock.toml");
        s.worldclock_pin("Asia/Tokyo", "Tokyo");
        s.worldclock_pin("Asia/Tokyo", "Tokyo again");
        assert_eq!(s.worldclock_zones.len(), 1);
        s.worldclock_remove(0);
        assert!(s.worldclock_zones.is_empty());
    }

    #[test]
    fn card_draws_rows_and_search_overlay() {
        let mut s = shell();
        let p = pal();
        s.worldclock_zones = vec![
            ("Tokyo".into(), "Asia/Tokyo".into(), 9 * 60, "JST".into()),
            ("London".into(), "Europe/London".into(), 0, "GMT".into()),
        ];
        let mut v: Vec<Cmd> = Vec::new();
        s.draw_worldclock_card(&mut v, 0.0, 0.0, 240.0, 120.0, &p);
        assert!(v.iter().any(|c| matches!(c, Cmd::Text { text, .. } if text.contains("Tokyo"))));
        // open search → overlay renders, results map fills
        s.worldclock_search = Some("tok".into());
        s.worldclock_pick_map.clear();
        let mut v2: Vec<Cmd> = Vec::new();
        s.draw_worldclock_card(&mut v2, 0.0, 0.0, 240.0, 120.0, &p);
        assert!(!s.worldclock_pick_map.is_empty(), "search 'tok' matches zone.tab Tokyo");
        assert!(s.worldclock_pick_map.iter().any(|(_, tz, _)| tz == "Asia/Tokyo"));
    }
}
