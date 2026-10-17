    use super::*;
    use std::sync::atomic::Ordering;

    fn shell() -> Shell {
        let cfg: crate::config::Config =
            toml::from_str(crate::config::DEFAULT_SHELL_TOML).expect("default config parses");
        Shell::new(cfg)
    }

    #[test]
    fn water_add_sub_and_goal_roundtrip() {
        let mut s = shell();
        s.per_state_config = std::env::temp_dir().join("zen-test-water.toml");
        s.water_glasses = 0;
        s.water_add();
        s.water_add();
        assert_eq!(s.water_glasses, 2);
        s.water_sub();
        assert_eq!(s.water_glasses, 1);
        s.water_set_goal(10);
        assert_eq!(s.water_goal, 10);
        s.water_set_goal(999);
        assert_eq!(s.water_goal, 30, "goal clamps at 30");
    }

    #[test]
    fn er_tick_laps_focus_into_rest_and_counts() {
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
    fn er_toggle_pause_resume_and_reset() {
        // the eye-rest buttons are ordinary Rust handlers now: the 31900/31901
        // click arms call `eyerest_toggle()` / the inline reset.
        let mut s = shell();
        s.eyerest_toggle();
        assert!(s.er_running);
        s.er_start = Some(Instant::now() - std::time::Duration::from_secs(30));
        s.eyerest_toggle();
        assert!(!s.er_running, "paused");
        assert!(s.er_paused_rem.is_some());
        let rem = s.er_remaining_secs();
        assert!(
            (ER_FOCUS_SECS - 31..ER_FOCUS_SECS - 29).contains(&rem),
            "paused rem ≈ 19:30, got {rem}"
        );
        s.eyerest_toggle();
        assert!(s.er_running, "resumed");
        // the reset arm (31901) is exactly this inline set
        s.er_running = false;
        s.er_start = None;
        s.er_paused_rem = None;
        s.er_rest_until = None;
        assert!(!s.er_running && s.er_paused_rem.is_none());
        assert_eq!(s.er_remaining_secs(), ER_FOCUS_SECS);
        // skipping a rest clears it without disturbing sessions
        s.er_rest_until = Some(Instant::now() + std::time::Duration::from_secs(10));
        s.eyerest_toggle();
        assert!(s.er_rest_until.is_none());
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
        s.pomo_deadline = Some(
            Instant::now()
                + std::time::Duration::from_secs(15 * 60)
                + std::time::Duration::from_millis(500),
        );
        s.pomo_toggle(); // pause
        assert!(!s.pomo_running);
        let rem = s.pomo_remaining_secs();
        assert!(
            (15 * 60 - 1..=15 * 60).contains(&rem),
            "paused rem ≈ 15min, got {rem}"
        );
        s.pomo_toggle(); // resume
        assert!(s.pomo_running);
        assert!(s.pomo_deadline.unwrap() > Instant::now());
        s.pomo_reset();
        assert_eq!(s.pomo_remaining_secs(), 25 * 60, "reset → fresh focus");
    }

    #[test]
    fn a_live_colour_picker_freezes_the_hover_rule() {
        let mut s = shell();
        let dash = s.dash_mode();
        s.mode = dash;
        s.hover = true;

        // The cursor walks off the bar onto the wheel (its own Overlay-layer
        // surface, above us) — that must NOT collapse the shell out from
        // under the picker.
        s.color_pick_hold.fetch_add(1, Ordering::AcqRel);
        s.set_hover(false);
        assert_eq!(s.mode, dash, "the shell stays up while the wheel is open");
        assert!(s.hover, "the rule is one store — frozen, not half-applied");

        // ...but a hover-EXPAND is still allowed (the cursor coming back).
        s.mode = Mode::Collapsed;
        s.hover = false;
        s.set_hover(true);
        assert_eq!(s.mode, dash, "expanding still follows the cursor");

        // Picker gone → the very next rule behaves exactly as before.
        s.color_pick_hold.store(0, Ordering::Release);
        s.mode = dash;
        s.hover = true;
        s.set_hover(false);
        assert_eq!(s.mode, Mode::Collapsed, "hover-collapse resumes");
    }
