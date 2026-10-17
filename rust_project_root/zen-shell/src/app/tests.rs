    use super::*;

    #[test]
    fn dash_cmds_spawn_scr_main_for_the_whole_screen_look() {
        // `scr_main` is the ONE helper that owns the screen look: one GLSL
        // shader with saturation, gamma and Kelvin all baked in. Four actions
        // route into it and they must all keep spelling `scr_main` -- a stale
        // `shader_main`/`hs_main` here is a spawn failure that only shows up as a
        // dead fader or a dead toggle.
        use crate::shell::DashCmd;
        for (cmd, want_args) in [
            (DashCmd::Saturation(140), vec!["set", "140"]),
            (DashCmd::Kelvin(2330), vec!["temp", "2330"]),
            // the warmth gate, spelled `temp-toggle` so the saturation toggle's
            // own `toggle` can never be confused with it
            (DashCmd::SunsetToggle, vec!["temp-toggle"]),
            (DashCmd::SaturationToggle, vec!["toggle"]),
        ] {
            let (prog, args) = dash_cmd_call(cmd).expect("script-backed action");
            assert_eq!(prog, "scr_main", "{cmd:?} must go through scr_main");
            assert_eq!(args, want_args, "{cmd:?} argument list");
        }
        // brightness is the one action the shell applies itself
        assert!(
            dash_cmd_call(DashCmd::Brightness(50)).is_none(),
            "brightness is in-process, never a spawn"
        );
    }

    #[test]
    fn boot_seed_timing() {
        use crate::backends::{
            Audio, Brightness, Power, SysfsBrightness, SysfsPower, UPowerPower, WpctlAudio,
        };
        use std::time::Instant;
        let cfg = crate::config::Config::default();
        let t = Instant::now();
        let mut shell = crate::shell::Shell::new(cfg.clone());
        println!("Shell::new: {:?}", t.elapsed());

        let mut t = Instant::now();
        let hypr = crate::hypr::Hypr::new();
        println!("Hypr::new: {:?}", t.elapsed());
        t = Instant::now();
        let _n = hypr.update_shell(&mut shell);
        println!("hypr.update_shell: {:?} ({} changed)", t.elapsed(), _n);

        t = Instant::now();
        let mut power: Box<dyn Power> = Box::new(UPowerPower::new());
        println!("UPowerPower::new: {:?}", t.elapsed());
        t = Instant::now();
        let _c = power.poll(&mut shell);
        println!("UPowerPower::poll: {:?} (changed {_c})", t.elapsed());

        t = Instant::now();
        let mut s2 = SysfsPower::new();
        let _c = s2.poll(&mut shell);
        println!("SysfsPower::new+poll: {:?} (changed {_c})", t.elapsed());

        t = Instant::now();
        let mut bri = SysfsBrightness::new();
        let _g = bri.get();
        println!("SysfsBrightness::new+get: {:?}", t.elapsed());

        t = Instant::now();
        let mut au = WpctlAudio;
        let _m = au.default_volume();
        println!("WpctlAudio::default_volume: {:?}", t.elapsed());

        t = Instant::now();
        let mut svc = crate::services::Services::new();
        println!("Services::new: {:?}", t.elapsed());
        t = Instant::now();
        let _c = svc.poll(&mut shell);
        println!("Services::poll: {:?} (changed {_c})", t.elapsed());

        t = Instant::now();
        let mut clip = crate::clipboard::Clipboard::new();
        let mut img = crate::img::ImageStore::new();
        let _c = clip.seed_history(&mut shell, &mut img);
        println!("clipboard.seed_history: {:?} (changed {_c})", t.elapsed());

        t = Instant::now();
        shell.seed_data();
        println!("shell.seed_data: {:?}", t.elapsed());
    }

    #[test]
    fn dashboard_first_layout_timing() {
        use std::time::Instant;
        let cfg = crate::config::Config::default();
        let mut shell = crate::shell::Shell::new(cfg.clone());
        shell.seed_data();
        // expanded dashboard at a decent canvas size
        shell.set_mode(crate::shell::Mode::Expanded);
        let (tw, th) = shell.target_size();
        let _ = (tw, th);
        let (cw, ch) = (1024.0f32, 800.0f32);
        let t = Instant::now();
        let _pal = crate::ui::Pal {
            fg: 0xe8e6e3ff,
            bg: 0x141414ff,
            acc: 0x4fc2ffff,
            sfg: 0x101010ff,
        };
        let _v: Vec<crate::cmd::Cmd> = shell.layout(cw, ch);
        println!("layout #1 (cold first expand): {:?}", t.elapsed());
        let t = Instant::now();
        let _v2: Vec<crate::cmd::Cmd> = shell.layout(cw, ch);
        println!("layout #2: {:?}", t.elapsed());
        // a full expanded services tick: stats + card polls (first unthrottled run)
        let t = Instant::now();
        let mut stats = crate::stats::Stats::new();
        stats.poll(&mut shell);
        crate::stats::Stats::poll_top_procs(&mut shell);
        crate::stats::Stats::poll_active_win(&mut shell);
        crate::stats::Stats::poll_pkg_updates(&mut shell);
        crate::stats::Stats::poll_ssh_vpn(&mut shell);
        crate::stats::Stats::poll_docker(&mut shell);
        crate::stats::Stats::poll_kb_layout(&mut shell);
        // let _ = shell.poll_smart();
        // let _ = shell.poll_systemd();
        // let _ = shell.poll_journal();
        // let _ = shell.poll_conninfo();
        println!("expanded first tick (stats+card polls): {:?}", t.elapsed());
    }

    #[test]
    fn split_raw_keeps_the_newest_complete_frame() {
        // three frames in one contiguous stream; the read chunks never align
        // with the frame boundary (frames are 100 bytes, chunks are 64 KiB)
        let frame_bytes = 100usize;
        let stream: Vec<u8> = [0u8; 100]
            .iter()
            .chain(std::iter::repeat_n(&1u8, 100))
            .chain(std::iter::repeat_n(&2u8, 100))
            .copied()
            .collect();
        let frame = Arc::new(std::sync::Mutex::new(Vec::new()));
        let fresh = Arc::new(AtomicBool::new(false));
        split_raw(&stream[..], frame.clone(), fresh.clone(), frame_bytes);
        assert!(fresh.load(Ordering::Relaxed), "a frame must have landed");
        let got = frame.lock().unwrap();
        assert_eq!(got.len(), frame_bytes);
        assert_eq!(&got[..], &[2u8; 100], "only the newest frame survives");
    }

    #[test]
    fn split_raw_drops_the_partial_tail() {
        let frame_bytes = 100usize;
        let mut stream = vec![0x55u8; frame_bytes];
        stream.extend_from_slice(&[0x66u8; 37]); // partial next frame
        let frame = Arc::new(std::sync::Mutex::new(Vec::new()));
        let fresh = Arc::new(AtomicBool::new(false));
        split_raw(&stream[..], frame.clone(), fresh.clone(), frame_bytes);
        assert!(fresh.load(Ordering::Relaxed));
        let got = frame.lock().unwrap();
        assert_eq!(got.len(), frame_bytes);
        assert!(
            got.iter().all(|&b| b == 0x55),
            "partial tail must not be stored"
        );
    }

    #[test]
    fn split_raw_empty_stream_never_stores() {
        let frame = Arc::new(std::sync::Mutex::new(Vec::new()));
        let fresh = Arc::new(AtomicBool::new(false));
        split_raw(&[][..], frame.clone(), fresh.clone(), 100);
        assert!(!fresh.load(Ordering::Relaxed));
        assert!(frame.lock().unwrap().is_empty());
    }
