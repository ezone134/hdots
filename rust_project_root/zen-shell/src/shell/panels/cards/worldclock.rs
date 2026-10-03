use super::super::*;

impl Shell {
    /// The search's matches — city · tz path, in zone.tab order, keyed by
    /// pick index, CAPPED at what the card can show.
    ///
    /// This is a PURE function of the buffer, the zone table and the card
    /// height, and it is the reason the search overlay can be a scene at all:
    /// the scene values publish the rows and the click handler resolves the
    /// key through this same function, so neither needs a frame-carried map.
    /// That map used to be appended to on every draw and never cleared, so a
    /// key from an earlier buffer still resolved — and `.find` returned the
    /// STALE entry, pinning a city the user could no longer see.
    pub(crate) fn worldclock_matches(&self) -> Vec<(u32, String, String)> {
        let buf = self.worldclock_search.clone().unwrap_or_default();
        let needle = buf.to_lowercase();
        if needle.len() < 2 {
            return Vec::new();
        }
        let (_, _, _, h) = self.worldclock_rect;
        let ry0 = self.scale.s(4.0) + self.scale.s(24.0) + self.scale.s(30.0);
        let row_h = self.scale.s(24.0);
        // a row must FIT, not merely start inside: the `Rows` item skips a row
        // whose bottom crosses the card, and the old check only asked whether
        // the NEXT row's top was still on screen — so the last match drew 24 px
        // past the card's bottom edge. The cap is now the item's own rule, and
        // the cap of six is the search's.
        let fit = (((h + 0.5 - ry0) / row_h).floor().max(0.0) as usize).min(6);
        let mut out = Vec::new();
        // zone.tab's parse is a cached OnceLock, so this filter is a scan
        for z in worldmap::zones() {
            if out.len() >= fit {
                break;
            }
            let city_lc = z.city.to_lowercase();
            let tz_lc = z.tz.to_lowercase();
            if !city_lc.contains(&needle) && !tz_lc.contains(&needle) {
                continue;
            }
            out.push((
                WORLDCLOCK_PICK_BASE + out.len() as u32,
                z.tz.clone(),
                z.city.clone(),
            ));
        }
        out
    }

    /// Pin a zone from the search results (dedup by tz; persists).
    pub(crate) fn worldclock_pin(&mut self, tz: &str, city: &str) {
        if self.worldclock_zones.iter().any(|(_, t, _, _)| t == tz) {
            return;
        }
        if self.worldclock_zones.len() >= 16 {
            return;
        }
        self.worldclock_zones
            .push((city.to_string(), tz.to_string(), 0, String::new()));
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

/// "+5:45" / "-3:00" style offset formatting from signed minutes. The SIGN is
/// the caller's — it is the difference from local, and a zero difference has no
/// chip at all.
pub(crate) fn fmt_hhmm_min(mins: i32) -> String {
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

    #[test]
    fn offset_math_and_hour_shift() {
        // fixed epochs: 2026-01-01 00:00 UTC
        let epoch = 1767225600;
        assert_eq!(
            Shell::local_utc_offset_min(epoch).abs() % 15,
            0,
            "offset is a multiple of 15 min"
        );
        assert_eq!(
            Shell::shifted_hour(epoch, 5 * 60 + 45),
            5,
            "UTC+5:45 at midnight UTC → 05:xx"
        );
        assert_eq!(
            Shell::shifted_hour(epoch, -5 * 60),
            19,
            "UTC-5 at midnight UTC → 19:xx"
        );
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
    fn the_match_list_is_pure() {
        let mut s = shell();
        s.worldclock_zones = vec![
            ("Tokyo".into(), "Asia/Tokyo".into(), 9 * 60, "JST".into()),
            ("London".into(), "Europe/London".into(), 0, "GMT".into()),
        ];
        // What Rust still owns is the match LIST — a pure function of the
        // buffer and the card box, shared by the scene value, the input key
        // resolver and (until the golden sweep) the fallback painter. The
        // painting is the scene's now; its parity lives in
        // `src/fixtures/worldclock.golden`.
        s.worldclock_search = Some("tok".into());
        s.worldclock_rect = (0.0, 0.0, 240.0, 120.0);
        let m = s.worldclock_matches();
        assert!(!m.is_empty(), "search 'tok' matches zone.tab Tokyo");
        assert!(
            m.iter().any(|(_, tz, _)| tz == "Asia/Tokyo"),
            "and it resolves a real tz path"
        );
        assert_eq!(
            m[0].0,
            crate::shell::WORLDCLOCK_PICK_BASE,
            "pick keys start at the base"
        );
        // the same buffer twice gives the same keys — the click handler and
        // the scene cannot drift, because there is nothing to drift
        assert_eq!(s.worldclock_matches(), m);
        // a one-character needle is not a search yet
        s.worldclock_search = Some("t".into());
        assert!(s.worldclock_matches().is_empty());
    }
}
