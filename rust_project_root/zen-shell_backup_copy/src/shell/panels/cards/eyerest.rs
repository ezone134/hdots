use super::super::*;
use super::shared::*;

/// 20-20-20 eye-care timer: 20 minutes of focus, then a 20 second rest.
pub(crate) const ER_FOCUS_SECS: u64 = 20 * 60;
pub(crate) const ER_REST_SECS: u64 = 20;

impl Shell {

    /// EYE REST — 20-20-20 focus/rest ring. A bead ring shows focus progress,
    /// the mm:ss hero counts the remaining stretch (green during a rest). The
    /// session total persists to `eyerest.txt`; running deadlines are
    /// session-local by design (like the pomodoro).
    pub(crate) fn draw_eyerest_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        // advance the state machine inline so a (re)draw always shows the
        // truth even if a 1 s tick was skipped
        let _ = self.er_tick();
        let pad = self.scale.s(12.0);
        let hdr = self.scale.s(24.0);
        let meta = if self.er_sessions > 0 {
            format!("{} rests", self.er_sessions)
        } else {
            String::new()
        };
        card_header(v, pal, x, y, w, pad, "Eye rest", Some(ICON_TIMER), Some((&meta, false)), self.scale, self.card_show_title, self.card_show_glyph);

        let resting = self.er_rest_until.is_some();
        let rem = self.er_remaining_secs();
        let mm = rem / 60;
        let ss = rem % 60;
        let total = if resting { ER_REST_SECS } else { ER_FOCUS_SECS };
        let frac = (total.saturating_sub(rem) as f32 / total as f32).clamp(0.0, 1.0);

        let ring_cx = x + pad + self.scale.s(28.0);
        let ring_cy = y + hdr + self.scale.s(26.0);
        let ring_r = self.scale.s(23.0);
        let lit = (frac * 36.0).round() as i32;
        if resting {
            Shell::ring_beads(v, ring_cx, ring_cy, ring_r, self.scale.s(3.0), lit.clamp(0, 36), ui::OK, ui::fg3(&pal));
        } else {
            Shell::ring_beads(v, ring_cx, ring_cy, ring_r, self.scale.s(3.0), lit.clamp(0, 36), pal.acc, ui::fg3(&pal));
        }
        let hero_c = if resting { ui::OK } else { pal.fg };
        let hx = ring_cx + ring_r + self.scale.s(12.0);
        ui::text_hero(v, hx, ring_cy - self.scale.s(7.0), format!("{mm:02}:{ss:02}"), self.scale.fs(21.0), hero_c);
        let state = if resting {
            "rest your eyes"
        } else if self.er_running {
            "focus"
        } else if self.er_paused_rem.is_some() {
            "paused"
        } else {
            "ready"
        };
        ui::caption(v, hx, ring_cy + self.scale.s(10.0), state, self.scale.fs(8.5), ui::fg3(&pal), false);
        // hint chip under the ring
        ui::caption(v, x + pad, ring_cy + ring_r + self.scale.s(6.0), "20 min focus · 20 s rest", self.scale.fs(8.0), ui::fg3(&pal), false);

        // buttons: toggle (accent) + reset
        let bw = self.scale.s(52.0);
        let bh = self.scale.s(22.0);
        let cy2 = y + h - bh - self.scale.s(10.0);
        let tog_lbl = if resting {
            "Skip"
        } else if self.er_running {
            "Pause"
        } else if self.er_paused_rem.is_some() {
            "Resume"
        } else {
            "Start"
        };
        self.draw_edit_lbl_btn(v, x + pad, cy2, bw, bh, EYEREST_TOGGLE_KEY, tog_lbl, self.cursor, pal, true);
        self.draw_edit_lbl_btn(v, x + w - pad - bw, cy2, bw, bh, EYEREST_RESET_KEY, "Reset", self.cursor, pal, false);
    }

    /// Remaining seconds of the current phase (focus countdown or rest
    /// countdown; paused rem when parked mid-focus).
    pub(crate) fn er_remaining_secs(&self) -> u64 {
        if let Some(end) = self.er_rest_until {
            return end.saturating_duration_since(Instant::now()).as_secs();
        }
        if let Some(start) = self.er_start {
            return ER_FOCUS_SECS.saturating_sub(Instant::now().duration_since(start).as_secs());
        }
        if let Some(r) = self.er_paused_rem {
            return r;
        }
        ER_FOCUS_SECS
    }

    /// Advance the 20-20-20 machine: finish a focus stretch (bump + persist
    /// the count, enter the 20 s rest), and end the rest. Both conditions are
    /// pure time checks with runtime state, so a paused/reset timer costs
    /// nothing. Returns true when a phase boundary was crossed.
    pub(crate) fn er_tick(&mut self) -> bool {
        let now = Instant::now();
        let mut changed = false;
        if let Some(end) = self.er_rest_until {
            if now >= end {
                self.er_rest_until = None;
                changed = true;
            }
        }
        if self.er_running {
            if let Some(start) = self.er_start {
                if now.duration_since(start) >= std::time::Duration::from_secs(ER_FOCUS_SECS) {
                    self.er_running = false;
                    self.er_start = None;
                    self.er_sessions += 1;
                    self.save_eyerest();
                    self.er_rest_until = Some(now + std::time::Duration::from_secs(ER_REST_SECS));
                    changed = true;
                }
            }
        }
        changed
    }

    /// Start / pause / resume the focus stretch. Resting → skip the rest.
    pub(crate) fn er_toggle(&mut self) {
        if self.er_rest_until.is_some() {
            self.er_rest_until = None;
            return;
        }
        if self.er_running {
            self.er_paused_rem = Some(self.er_remaining_secs());
            self.er_start = None;
            self.er_running = false;
        } else {
            let rem = self.er_paused_rem.take().unwrap_or(ER_FOCUS_SECS);
            // re-base `er_start` so the displayed rem stays exact after resume
            self.er_start = Some(Instant::now() - std::time::Duration::from_secs(ER_FOCUS_SECS - rem));
            self.er_running = true;
        }
    }

    /// Full reset: stops any phase and clears the paused remainder.
    pub(crate) fn er_reset(&mut self) {
        self.er_running = false;
        self.er_start = None;
        self.er_paused_rem = None;
        self.er_rest_until = None;
    }

    // persistence ---------------------------------------------------------

    /// Eye-rest store (`eyerest.txt`) — sesssion count survives restarts.
    pub(crate) fn eye_path() -> std::path::PathBuf {
        Self::todos_path().with_file_name("eyerest.txt")
    }

    pub(crate) fn load_eyerest(&mut self) {
        if let Ok(s) = std::fs::read_to_string(Self::eye_path()) {
            for line in s.lines() {
                if let Some(n) = line.strip_prefix("sessions ") {
                    if let Ok(n) = n.trim().parse() {
                        self.er_sessions = n;
                    }
                }
            }
        }
    }

    pub(crate) fn save_eyerest(&self) {
        if let Some(dir) = Self::eye_path().parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(Self::eye_path(), format!("sessions {}\n", self.er_sessions));
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
    fn card_draws_idle_running_and_rest() {
        let mut s = shell();
        let p = pal();
        let mut v: Vec<Cmd> = Vec::new();
        s.draw_eyerest_card(&mut v, 0.0, 0.0, 200.0, 110.0, &p);
        assert!(v.iter().any(|c| matches!(c, Cmd::Text { text, .. } if text.contains(":"))), "mm:ss drawn idling");
        s.er_toggle();
        s.draw_eyerest_card(&mut v, 0.0, 0.0, 200.0, 110.0, &p);
        assert!(s.er_running, "running after start");
        s.er_rest_until = Some(Instant::now() + std::time::Duration::from_secs(ER_REST_SECS));
        s.draw_eyerest_card(&mut v, 0.0, 0.0, 200.0, 110.0, &p);
    }

    #[test]
    fn tick_laps_focus_into_rest_and_counts() {
        let mut s = shell();
        s.er_running = true;
        s.er_start = Some(Instant::now() - std::time::Duration::from_secs(ER_FOCUS_SECS + 1));
        assert!(s.er_tick(), "focus overdue → laps");
        assert!(!s.er_running, "focus stops");
        assert!(s.er_rest_until.is_some(), "rest phase begins");
        assert_eq!(s.er_sessions, 1);
        assert!(!s.er_tick(), "rest in progress → no boundary yet");
        s.er_rest_until = Some(Instant::now() - std::time::Duration::from_secs(1));
        assert!(s.er_tick(), "rest over → cleared");
        assert!(s.er_rest_until.is_none());
    }

    #[test]
    fn toggle_pause_resume_and_reset() {
        let mut s = shell();
        s.er_toggle();
        assert!(s.er_running);
        s.er_start = Some(Instant::now() - std::time::Duration::from_secs(30));
        s.er_toggle();
        assert!(!s.er_running, "paused");
        assert!(s.er_paused_rem.is_some());
        let rem = s.er_remaining_secs();
        assert!((ER_FOCUS_SECS - 31..ER_FOCUS_SECS - 29).contains(&rem), "paused rem ≈ 19:30, got {rem}");
        s.er_toggle();
        assert!(s.er_running, "resumed");
        s.er_reset();
        assert!(!s.er_running && s.er_paused_rem.is_none());
        assert_eq!(s.er_remaining_secs(), ER_FOCUS_SECS);
        // skipping a rest clears it without disturbing sessions
        s.er_rest_until = Some(Instant::now() + std::time::Duration::from_secs(10));
        s.er_toggle();
        assert!(s.er_rest_until.is_none());
    }
}