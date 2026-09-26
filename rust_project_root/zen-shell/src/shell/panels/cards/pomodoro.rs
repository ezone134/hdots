use super::super::*;

impl Shell {

    /// POMODORO — focus timer card. A big mm:ss countdown with start/pause,
    /// reset and focus-length steppers. Phases alternate focus (fg) → break
    /// (ok green); finishing a focus phase bumps the session counter. State
    /// (durations, phase, sessions) persists to $XDG_DATA_HOME/zen-shell/
    /// pomo.txt; the running deadline is session-local by design.
    pub(crate) fn draw_pomodoro_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        let pad = self.scale.s(12.0);
        let hdr = self.scale.s(24.0);
        let meta = if self.pomo_sessions > 0 {
            format!("{} done", self.pomo_sessions)
        } else {
            String::new()
        };
        self.card_head(v, pal, x, y, w, pad, "Focus", Some(ICON_TIMER), Some((&meta, false)));
        let running = self.pomo_running;
        // countdown — Display hero, green during breaks
        let rem = self.pomo_remaining_secs();
        let mm = rem / 60;
        let ss = rem % 60;
        let phase_c = if self.pomo_phase_focus { pal.fg } else { ui::OK };
        let hero_y = y + hdr + self.scale.s(6.0);
        ui::text_c(v, x + w / 2.0, hero_y, format!("{mm:02}:{ss:02}"), self.scale.fs(26.0), if running { phase_c } else { mix(ui::hover(&pal), phase_c, 0.25) }, false);
        // phase caption + progress bar under the numerals
        let phase_lbl = if self.pomo_phase_focus { "focus" } else { "break" };
        ui::caption_c(v, x + w / 2.0, hero_y + self.scale.s(28.0), phase_lbl, self.scale.fs(8.5), ui::fg3(&pal), false);
        let total = (if self.pomo_phase_focus { self.pomo_focus_min } else { self.pomo_break_min }).max(1) as f32 * 60.0;
        let frac = 1.0 - rem as f32 / total;
        let bar_y = hero_y + self.scale.s(42.0);
        ui::bar(v, x + pad, bar_y, w - pad * 2.0, self.scale.s(4.0), frac.clamp(0.0, 1.0), phase_c, ui::hover(&pal));
        // duration steppers (idle only): [-] 25m [+]
        let bw = self.scale.s(18.0);
        let bh = self.scale.s(18.0);
        let row2 = bar_y + self.scale.s(10.0);
        if !running {
            let readout = format!("{}m", self.pomo_focus_min);
            let rw = readout.chars().count() as f32 * 0.62 * self.scale.fs(8.5) + 2.0;
            let cx0 = x + w / 2.0 - rw / 2.0 - bw - self.scale.s(6.0);
            self.draw_edit_btn(v, cx0, row2, bw, bh, POMODORO_DEC_KEY, "-", cursor_px_of(self), pal, true);
            ui::text_c(v, x + w / 2.0, row2 + self.scale.s(3.0), &readout, self.scale.fs(8.5), ui::fg3(&pal), false);
            self.draw_edit_btn(v, cx0 + bw + self.scale.s(6.0) + rw, row2, bw, bh, POMODORO_INC_KEY, "+", cursor_px_of(self), pal, true);
        }
        // controls: start/pause (accent) + reset
        let bw2 = self.scale.s(52.0);
        let bh2 = self.scale.s(22.0);
        let cy2 = y + h - bh2 - self.scale.s(10.0);
        let tog_lbl = if running { "Pause" } else if self.pomo_paused_rem.is_some() { "Resume" } else { "Start" };
        self.draw_edit_lbl_btn(v, x + w / 2.0 - bw2 - self.scale.s(6.0), cy2, bw2, bh2, POMODORO_TOGGLE_KEY, tog_lbl, cursor_px_of(self), pal, true);
        self.draw_edit_lbl_btn(v, x + w / 2.0 + self.scale.s(6.0), cy2, bw2, bh2, POMODORO_RESET_KEY, "Reset", cursor_px_of(self), pal, false);
    }

}

/// Cursor passthrough for the shared edit-button helpers (they take the raw
/// Option); keeps the call sites above terse.
fn cursor_px_of(s: &Shell) -> Option<(f32, f32)> {
    s.cursor
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
    fn card_draws_idle_running_and_break() {
        let mut s = shell();
        let p = pal();
        let mut v: Vec<Cmd> = Vec::new();
        s.draw_pomodoro_card(&mut v, 0.0, 0.0, 220.0, 110.0, &p);
        // start/pause + reset hit regions registered
        assert!(v.iter().any(|c| matches!(c, Cmd::Text { text, .. } if text.contains(":"))), "mm:ss drawn");
        s.pomo_toggle();
        s.draw_pomodoro_card(&mut v, 0.0, 0.0, 220.0, 110.0, &p);
        assert!(s.pomo_deadline.is_some(), "running after start");
        s.pomo_phase_focus = false;
        s.draw_pomodoro_card(&mut v, 0.0, 0.0, 220.0, 110.0, &p);
    }

    #[test]
    fn tick_flips_phase_and_counts_sessions() {
        let mut s = shell();
        s.pomo_running = true;
        s.pomo_deadline = Some(Instant::now() - std::time::Duration::from_secs(1));
        assert!(s.pomo_tick(), "deadline elapsed → phase flips");
        assert!(!s.pomo_phase_focus, "focus → break");
        assert_eq!(s.pomo_sessions, 1);
        assert!(s.pomo_running, "auto-continues into the break");
        // finishing a break does NOT bump sessions further
        s.pomo_deadline = Some(Instant::now() - std::time::Duration::from_secs(1));
        assert!(s.pomo_tick());
        assert!(s.pomo_phase_focus, "break → focus");
        assert_eq!(s.pomo_sessions, 1);
    }

    #[test]
    fn toggle_pause_resume_keeps_remaining() {
        let mut s = shell();
        s.pomo_focus_min = 25;
        s.pomo_toggle();
        // pretend 10 minutes elapsed (½ s padding so secs don't truncate)
        s.pomo_deadline = Some(Instant::now() + std::time::Duration::from_secs(15 * 60) + std::time::Duration::from_millis(500));
        s.pomo_toggle(); // pause
        assert!(!s.pomo_running);
        let rem = s.pomo_remaining_secs();
        assert!((15 * 60 - 1..=15 * 60).contains(&rem), "paused rem ≈ 15min, got {rem}");
        s.pomo_toggle(); // resume
        assert!(s.pomo_running);
        assert!(s.pomo_deadline.unwrap() > Instant::now());
        s.pomo_reset();
        assert_eq!(s.pomo_remaining_secs(), 25 * 60, "reset → fresh focus");
    }
}
