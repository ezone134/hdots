    use super::*;

    #[test]
    fn a_scene_clicks_command_can_reach_scr_main() {
        // A card row's `Command` runs as `sh -c "<cmd>"` with the SHELL's own
        // PATH. The rconf helpers (`scr_main`, `dash_status`) live in
        // `$sources/bin`, so pinning that PATH to `/usr/bin` turns every
        // rconf-backed row — the sliders card's Disable/Normal/Master list
        // included — into a silent "command not found" (the click does nothing
        // and no error ever surfaces). This runs the spawn the way the handler
        // does and pins that it resolves. Skipped where there is no rconf tree.
        let which = std::process::Command::new("sh")
            .args(["-c", "command -v scr_main"])
            .output()
            .expect("sh runs");
        if !which.status.success() {
            return; // no rconf on this box
        }
        let status = std::process::Command::new("sh")
            .args(["-c", "scr_main status"])
            .output()
            .expect("scr_main status runs");
        assert!(
            status.status.success(),
            "a card's `scr_main` row really runs: {}",
            String::from_utf8_lossy(&status.stderr).trim()
        );
        assert!(!status.stdout.is_empty(), "and reports a mode line");
    }

    #[test]
    fn kelvin_fader_defers_to_scr_main() {
        // The fader's backend call must stay `scr_main temp K` — that one helper
        // owns the WHOLE screen look: the value is baked into master.glsl and
        // the switch is turned on if it was off (6500 K is the neutral off
        // notch). If this ever routes to a second program, dragging the knob
        // silently stops updating the shader.
        let (prog, args) = crate::shell::kelvin_cmd_args(2330);
        assert_eq!(prog, "scr_main", "one entry point for the Kelvin knob");
        assert_eq!(args, vec!["temp".to_string(), "2330".to_string()]);

        // and the drag really defers that command to pointer release
        let mut shell = crate::shell::Shell::new(crate::config::Config::default());
        shell.drag_key = Some(55);
        shell.sunset_kelvin = 4200;
        assert!(
            !shell.end_drag(0.0, 0.0),
            "a plain release changes no layout"
        );
        match shell.pending_dash {
            Some(DashCmd::Kelvin(k)) => {
                assert_eq!(k, 4200, "the release carries the fader's Kelvin")
            }
            other => panic!("expected the deferred Kelvin command, got {other:?}"),
        }
    }

    #[test]
    fn kelvin_fader_spans_1200_to_6500() {
        // The whole point of the row: pointer at the left end of the groove =
        // 1200 K, right end = 6500 K, and a MOTION drag keeps following the
        // cursor (the press alone used to pin the value, so the knob looked
        // dead). Geometry is the scene-published groove (x 34, w 248 on a
        // 300 px card) — `set_dash_slider` reads `faders[key-51]`, nothing else.
        let mut shell = crate::shell::Shell::new(crate::config::Config::default());
        shell.faders[4] = (34.0, 12.0, 248.0);

        // press at the left end → the warm floor
        assert!(
            shell.set_dash_slider(55, 34.0, 12.0),
            "press moves the fader"
        );
        assert_eq!(shell.sunset_kelvin, 1200, "left end is 1200 K");
        assert_eq!(shell.drag_key, Some(55), "the press owns the drag");

        // drag to the right end → the neutral notch, and the release carries it
        shell.mode = Mode::Expanded;
        assert!(shell.drag(282.0, 12.0), "motion keeps tracking the fader");
        assert_eq!(shell.sunset_kelvin, 6500, "right end is 6500 K");
        assert!(
            !shell.end_drag(282.0, 12.0),
            "a plain release changes no layout"
        );
        match shell.pending_dash {
            Some(DashCmd::Kelvin(6500)) => {}
            other => panic!("the release must carry 6500 K to scr_main, got {other:?}"),
        }

        // a quarter across the 5300 K span, and past both ends it clamps
        shell.pending_dash = None;
        shell.drag_key = Some(55);
        shell.drag(96.0, 12.0);
        assert_eq!(shell.sunset_kelvin, 2525, "a quarter in is 2525 K");
        shell.drag(-500.0, 12.0);
        assert_eq!(
            shell.sunset_kelvin, 1200,
            "left of the groove clamps to 1200 K"
        );
        shell.drag(9999.0, 12.0);
        assert_eq!(
            shell.sunset_kelvin, 6500,
            "right of the groove clamps to 6500 K"
        );
    }

    #[test]
    fn balance_slider_reaches_extremes() {
        let mut shell = crate::shell::Shell::new(crate::config::Config::default());
        shell.balance_rect = (100.0, 200.0, 300.0);
        shell.vol_left = 0.72;
        shell.vol_right = 0.72;
        // full left: pointer at fader start → L carries the whole 2*mid sum
        assert!(shell.set_dash_slider(59, 100.0, 200.0));
        assert!(
            (shell.vol_left - 1.44).abs() < 0.01,
            "vol_left {}",
            shell.vol_left
        );
        assert!(
            shell.vol_right.abs() < 0.01,
            "vol_right {}",
            shell.vol_right
        );
        // full right: pointer at fader end → R carries the whole sum
        assert!(shell.set_dash_slider(59, 400.0, 200.0));
        assert!(shell.vol_left.abs() < 0.01, "vol_left {}", shell.vol_left);
        assert!(
            (shell.vol_right - 1.44).abs() < 0.01,
            "vol_right {}",
            shell.vol_right
        );
        // balanced center: frac 0.5 → both stay at mid
        assert!(shell.set_dash_slider(59, 250.0, 200.0));
        assert!(
            (shell.vol_left - 0.72).abs() < 0.01,
            "vol_left {}",
            shell.vol_left
        );
        assert!(
            (shell.vol_right - 0.72).abs() < 0.01,
            "vol_right {}",
            shell.vol_right
        );
    }
