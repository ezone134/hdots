    use super::*;
    use crate::widgets::{NoHit, NoMeasurer, Rect};

    fn pal() -> Pal {
        Pal {
            fg: 0xececf0ff,
            bg: 0x14141aff,
            acc: 0x7d5cffff,
            sfg: 0xffffff00 | 0xff,
        }
    }

    /// The api.x == nil guards: dropping the date line drops exactly one Cmd.
    #[test]
    fn the_clock_state_guards_match_the_module() {
        let p = pal();
        let grid = 1.0;
        let full = clock(Some("00:00"), Some("Mon, 1 Jan"), grid, &p);
        let no_date = clock(Some("00:00"), None, grid, &p);
        let count = |tree: Node| -> usize {
            let tree = tree.layout(Rect { x: 0.0, y: 0.0, w: 200.0, h: 100.0 }, &mut NoMeasurer);
            let mut v: Vec<Cmd> = Vec::new();
            tree.render(&mut v, &mut NoHit, &p);
            v.len()
        };
        assert_eq!(count(full), 2);
        assert_eq!(count(no_date), 1);
    }

    /// The cpu card's widget tree emits the EXACT `Cmd` bytes of the retired
    /// `cards::cpu` direct drawer (WIDGETS.md stage-3 reference port). The
    /// retired code is replayed here against the still-present `ui::*` /
    /// `scene::bead_ring` primitives and the two streams must match.
    #[test]
    fn the_cpu_widget_paints_exactly_like_the_retired_drawer() {
        let p = pal();
        let (w, h, grid) = (200.0f32, 100.0f32, 1.0f32);
        let (pct, meta, gauge_col, lit) = (42i32, "3200 MHz", p.acc, 15i32);
        let rows = vec![
            ("Load".to_string(), "0.10/0.20/0.30".to_string(), p.fg),
            ("Freq".to_string(), "3.20 GHz".to_string(), p.fg),
            ("Temp".to_string(), "48\u{b0}C".to_string(), p.fg),
        ];
        let (x, y) = (0.0f32, 0.0f32);
        let s = |n: f32| n * grid;
        let fs = |n: f32| (n * grid * 10.0).round() / 10.0;
        let p14 = s(14.0);
        let mut want: Vec<Cmd> = Vec::new();
        crate::ui::title(&mut want, x + p14, y + s(9.0), "CPU", fs(11.5), p.fg);
        crate::ui::text_r_mono(
            &mut want,
            x + w - p14,
            y + s(11.0),
            meta,
            fs(8.5),
            crate::ui::fg3(&p),
        );
        crate::scene::bead_ring(
            &mut want,
            x + s(38.0),
            y + h / 2.0 + s(13.0),
            s(26.0),
            s(4.0),
            36,
            lit.max(0) as usize,
            gauge_col,
            crate::ui::hover(&p),
        );
        let sx = x + s(38.0);
        crate::ui::text_cw(
            &mut want,
            sx + (w - s(38.0)) / 2.0,
            y + h / 2.0 + s(6.0),
            format!("{pct}%"),
            fs(15.0),
            Ff::Ui,
            Fw::Semibold,
            gauge_col,
            false,
        );
        let mut ry = y + s(30.0);
        for (label, value, color) in &rows {
            if ry + s(22.0) > y + h + 0.5 {
                break;
            }
            crate::ui::caption(
                &mut want,
                x + s(14.0) + s(62.0),
                ry,
                label.as_str(),
                fs(8.5),
                crate::ui::fg2(&p),
                false,
            );
            if !value.is_empty() {
                crate::ui::text_r_mono(
                    &mut want,
                    x + w - s(14.0),
                    ry,
                    value.as_str(),
                    fs(8.5),
                    *color,
                );
            }
            ry += s(22.0);
        }
        let tree = cpu(pct, meta, gauge_col, lit, &rows, true, w, h, grid, &p)
            .layout(Rect { x, y, w, h }, &mut NoMeasurer);
        let mut got: Vec<Cmd> = Vec::new();
        tree.render(&mut got, &mut NoHit, &p);
        assert_eq!(got, want);
    }

    /// The mem card's widget tree emits the EXACT `Cmd` bytes of the retired
    /// `cards::mem_native` drawer (ring, hero, rows, f64 swap bar).
    #[test]
    fn the_mem_widget_paints_exactly_like_the_retired_drawer() {
        let p = pal();
        let (w, h, grid) = (200.0f32, 130.0f32, 1.0f32);
        let (mem_pct, lit) = (63i32, 23i32);
        let rows = vec![
            ("Total".to_string(), "32.0G".to_string()),
            ("Available".to_string(), "11.8G".to_string()),
            ("Cached".to_string(), "9.4G".to_string()),
            ("Free".to_string(), "11.8G".to_string()),
        ];
        let (x, y) = (0.0f32, 0.0f32);
        let mut want: Vec<Cmd> = Vec::new();
        mem_native(
            &mut want, mem_pct, lit, &rows, 37, true, true, x, y, w, h, grid, &p,
        );
        let tree = mem(mem_pct, lit, &rows, 37, true, true, w, h, grid, &p)
            .layout(Rect { x, y, w, h }, &mut NoMeasurer);
        let mut got: Vec<Cmd> = Vec::new();
        tree.render(&mut got, &mut NoHit, &p);
        assert_eq!(got, want);
    }

    /// The viz card's widget tree emits the EXACT `Cmd` bytes of the retired
    /// `cards::viz_native` drawer (header + style hit + spectrum stamp).
    #[test]
    fn the_viz_widget_paints_exactly_like_the_retired_drawer() {
        let p = pal();
        let (w, h, grid) = (200.0f32, 130.0f32, 1.0f32);
        let (x, y) = (0.0f32, 0.0f32);
        let series: Vec<f32> = (0..24).map(|i| (i as f32 * 0.41).sin().abs()).collect();
        for style in [0u8, 1, 2] {
            let mut want: Vec<Cmd> = Vec::new();
            viz_native(
                &mut want, true, true, &series, style, true, 0, 7, x, y, w, h, grid, &p,
            );
            let tree = viz(true, true, &series, style, true, 0, 7, w, h, grid, &p)
                .layout(Rect { x, y, w, h }, &mut NoMeasurer);
            let mut got: Vec<Cmd> = Vec::new();
            tree.render(&mut got, &mut NoHit, &p);
            assert_eq!(got, want, "style={style}");
        }
    }

    /// The power cards' widget trees emit the EXACT `Cmd` bytes of the retired
    /// `cards::powerh_native` / `powerv_native` drawers (incl. hold bars).
    #[test]
    fn the_power_widgets_paint_exactly_like_the_retired_drawers() {
        let p = pal();
        let (w, h, grid) = (200.0f32, 130.0f32, 1.0f32);
        let (x, y) = (0.0f32, 0.0f32);
        let bg = [0x111111ff, 0x222222ff, 0x333333ff, 0x444444ff, 0x555555ff];
        let ink = [p.fg; 5];
        let hold = [0.5f32; 5];
        let hold_on = [true; 5];
        let hold_fill = 0x998877ff;
        for horiz in [true, false] {
            let mut want: Vec<Cmd> = Vec::new();
            if horiz {
                powerh_native(
                    &mut want, true, true, hold_fill, &bg, &ink, &hold, &hold_on, x, y, w, h,
                    grid, &p,
                );
            } else {
                powerv_native(
                    &mut want, true, true, hold_fill, &bg, &ink, &hold, &hold_on, x, y, w, h,
                    grid, &p,
                );
            }
            let tree = if horiz {
                powerh(true, true, hold_fill, &bg, &ink, &hold, &hold_on, w, h, grid, &p)
            } else {
                powerv(true, true, hold_fill, &bg, &ink, &hold, &hold_on, w, h, grid, &p)
            }
            .layout(Rect { x, y, w, h }, &mut NoMeasurer);
            let mut got: Vec<Cmd> = Vec::new();
            tree.render(&mut got, &mut NoHit, &p);
            assert_eq!(got, want, "horizontal={horiz}");
        }
    }

    /// Paint a widget `Node` and the retired drawer into byte streams and
    /// compare them (the info-row batch parity helper).
    fn assert_parity(tree: Node, want: Vec<Cmd>, w: f32, h: f32, label: &str) {
        let p = pal();
        let (x, y) = (0.0f32, 0.0f32);
        let tree = tree.layout(Rect { x, y, w, h }, &mut NoMeasurer);
        let mut got: Vec<Cmd> = Vec::new();
        tree.render(&mut got, &mut NoHit, &p);
        assert_eq!(got, want, "{label}");
    }

    #[test]
    fn the_ticker_widget_paints_exactly_like_the_retired_drawer() {
        let p = pal();
        let (w, h, grid) = (200.0f32, 130.0f32, 1.0f32);
        let (x, y) = (0.0f32, 0.0f32);
        let rows = vec![
            ("BTC".to_string(), "67,000".to_string(), "+1.2%".to_string(), p.acc),
            ("ETH".to_string(), "3,500".to_string(), "-0.4%".to_string(), crate::ui::DANGER),
            ("SOL".to_string(), "150".to_string(), "+2.0%".to_string(), p.acc),
        ];
        let key_base = 7000u32;
        for hover in [0u32, key_base, key_base + 2] {
            let mut want: Vec<Cmd> = Vec::new();
            ticker_native(
                &mut want, true, true, "3 coins", "live", &rows, 0, key_base, hover, x, y, w, h,
                grid, &p,
            );
            let tree = ticker(
                true, true, "3 coins", "live", &rows, 0, key_base, hover, w, h, grid, &p,
            );
            assert_parity(tree, want, w, h, &format!("hover={hover}"));
        }
    }

    #[test]
    fn the_sshvpn_widget_paints_exactly_like_the_retired_drawer() {
        let p = pal();
        let (w, h, grid) = (200.0f32, 130.0f32, 1.0f32);
        let (x, y) = (0.0f32, 0.0f32);
        let empty: Vec<(String, String)> = Vec::new();
        let full = vec![
            ("\u{f0ac}".to_string(), "VPN up".to_string()),
            ("\u{f132}".to_string(), "ssh host".to_string()),
        ];
        for (rows, status) in [(&empty, "No connections"), (&full, "")] {
            let mut want: Vec<Cmd> = Vec::new();
            sshvpn_native(&mut want, true, status, rows, x, y, w, h, grid, &p);
            let tree = sshvpn(true, status, rows, w, h, grid, &p);
            assert_parity(tree, want, w, h, "sshvpn");
        }
    }

    #[test]
    fn the_conninfo_widget_paints_exactly_like_the_retired_drawer() {
        let p = pal();
        let (w, h, grid) = (200.0f32, 130.0f32, 1.0f32);
        let (x, y) = (0.0f32, 0.0f32);
        let dim = crate::ui::fg3(&p);
        let rows = vec![
            ("iface".to_string(), "wlan0".to_string(), p.fg, p.fg),
            ("gateway".to_string(), "10.0.0.1".to_string(), dim, dim),
            ("dns".to_string(), "1.1.1.1".to_string(), dim, dim),
        ];
        for live in [true, false] {
            for hover in [0u32, 31960] {
                let meta = if live { "3 addrs" } else { "" };
                let r: &[(String, String, u32, u32)] = if live { &rows } else { &[] };
                let mut want: Vec<Cmd> = Vec::new();
                conninfo_native(&mut want, true, true, meta, r, live, hover, x, y, w, h, grid, &p);
                let tree = conninfo(true, true, meta, r, live, hover, w, h, grid, &p);
                assert_parity(tree, want, w, h, &format!("live={live} hover={hover}"));
            }
        }
    }

    #[test]
    fn the_journaltail_widget_paints_exactly_like_the_retired_drawer() {
        let p = pal();
        let (w, h, grid) = (200.0f32, 130.0f32, 1.0f32);
        let (x, y) = (0.0f32, 0.0f32);
        let rows = vec![
            ("boot ok".to_string(), crate::ui::fg2(&p)),
            ("usb fail".to_string(), crate::ui::DANGER),
        ];
        for live in [true, false] {
            for hover in [0u32, 31995] {
                let meta = if live { "2 lines" } else { "" };
                let r: &[(String, u32)] = if live { &rows } else { &[] };
                let mut want: Vec<Cmd> = Vec::new();
                journaltail_native(&mut want, true, true, meta, r, live, hover, x, y, w, h, grid, &p);
                let tree = journaltail(true, true, meta, r, live, hover, w, h, grid, &p);
                assert_parity(tree, want, w, h, &format!("live={live} hover={hover}"));
            }
        }
    }

    #[test]
    fn the_systemdunits_widget_paints_exactly_like_the_retired_drawer() {
        let p = pal();
        let (w, h, grid) = (200.0f32, 130.0f32, 1.0f32);
        let (x, y) = (0.0f32, 0.0f32);
        let rows = vec![("foo.service".to_string(), crate::ui::DANGER)];
        for live in [true, false] {
            for hover in [0u32, 31990] {
                let meta = if live { "1 failed" } else { "" };
                let r: &[(String, u32)] = if live { &rows } else { &[] };
                let mut want: Vec<Cmd> = Vec::new();
                systemdunits_native(
                    &mut want, true, true, "\u{f132}", meta, r, live, hover, x, y, w, h, grid, &p,
                );
                let tree = systemdunits(
                    true, true, "\u{f132}", meta, r, live, hover, w, h, grid, &p,
                );
                assert_parity(tree, want, w, h, &format!("live={live} hover={hover}"));
            }
        }
    }

    #[test]
    fn the_smarthealth_widget_paints_exactly_like_the_retired_drawer() {
        let p = pal();
        let (w, h, grid) = (200.0f32, 130.0f32, 1.0f32);
        let (x, y) = (0.0f32, 0.0f32);
        let rows = vec![(
            "sda".to_string(),
            "Samsung SSD 980".to_string(),
            "38\u{b0}".to_string(),
            crate::ui::fg2(&p),
            crate::ui::fg3(&p),
            crate::ui::fg3(&p),
            crate::ui::OK,
            "\u{f00c}".to_string(),
        )];
        for live in [true, false] {
            for hover in [0u32, 31980] {
                let meta = if live { "1 disk" } else { "" };
                let r: &[SmartRow] =
                    if live { &rows } else { &[] };
                let mut want: Vec<Cmd> = Vec::new();
                smarthealth_native(
                    &mut want, true, true, "\u{f0a0}", meta, r, live, hover, x, y, w, h, grid, &p,
                );
                let tree = smarthealth(
                    true, true, "\u{f0a0}", meta, r, live, hover, w, h, grid, &p,
                );
                assert_parity(tree, want, w, h, &format!("live={live} hover={hover}"));
            }
        }
    }

    #[test]
    fn the_docker_widget_paints_exactly_like_the_retired_drawer() {
        let p = pal();
        let (w, h, grid) = (200.0f32, 130.0f32, 1.0f32);
        let (x, y) = (0.0f32, 0.0f32);
        let empty: Vec<(u32, String, String)> = Vec::new();
        let full = vec![
            (crate::ui::OK, "web".to_string(), "nginx:latest".to_string()),
            (
                crate::ui::DANGER,
                "db".to_string(),
                "registry.example.com/verylongimage:tag".to_string(),
            ),
        ];
        for (rows, label) in [(&empty, "empty"), (&full, "full")] {
            let mut want: Vec<Cmd> = Vec::new();
            docker_native(&mut want, true, rows, x, y, w, h, grid, &p);
            let tree = docker(true, rows, w, h, grid, &p);
            assert_parity(tree, want, w, h, label);
        }
    }

    #[test]
    fn the_procmon_widget_paints_exactly_like_the_retired_drawer() {
        let p = pal();
        let (w, h, grid) = (200.0f32, 130.0f32, 1.0f32);
        let (x, y) = (0.0f32, 0.0f32);
        let rows = vec![
            ("firefox".to_string(), 0.5f32),
            ("systemd".to_string(), 0.25f32),
            ("idle".to_string(), 0.0f32),
        ];
        let mut want: Vec<Cmd> = Vec::new();
        procmon_native(&mut want, true, "CPU 12%", &rows, x, y, w, h, grid, &p);
        let tree = procmon(true, "CPU 12%", &rows, w, h, grid, &p);
        assert_parity(tree, want, w, h, "procmon");
    }

    #[test]
    fn the_sensors_widget_paints_exactly_like_the_retired_drawer() {
        let p = pal();
        let (w, h, grid) = (200.0f32, 130.0f32, 1.0f32);
        let (x, y) = (0.0f32, 0.0f32);
        for (none, tilt, compass, gyro) in [
            (false, true, true, true),
            (true, false, false, false),
            (false, true, false, false),
        ] {
            let mut want: Vec<Cmd> = Vec::new();
            sensors_native(
                &mut want, true, true, none, tilt, compass, gyro, "pitch 3 roll 4",
                "\u{f0d7} 270\u{b0}", "  1   2   3", x, y, w, h, grid, &p,
            );
            let tree = sensors(
                true, true, none, tilt, compass, gyro, "pitch 3 roll 4",
                "\u{f0d7} 270\u{b0}", "  1   2   3", w, h, grid, &p,
            );
            assert_parity(
                tree,
                want,
                w,
                h,
                &format!("none={none} tilt={tilt} compass={compass} gyro={gyro}"),
            );
        }
    }

    #[test]
    fn the_thermal_widget_paints_exactly_like_the_retired_drawer() {
        let p = pal();
        let (w, h, grid) = (103.0f32, 130.0f32, 1.0f32);
        let (x, y) = (0.0f32, 0.0f32);
        let cells = vec![
            ("cpu0".to_string(), "42\u{b0}".to_string(), crate::ui::fg2(&p)),
            ("cpu1".to_string(), "48\u{b0}".to_string(), crate::ui::fg2(&p)),
            ("gpu".to_string(), "65\u{b0}".to_string(), crate::ui::WARN),
            ("nvme".to_string(), "84\u{b0}".to_string(), crate::ui::DANGER),
        ];
        for empty in [false, true] {
            let c: &[(String, String, u32)] = if empty { &[] } else { &cells };
            let meta = if empty { "" } else { "4 zones" };
            let mut want: Vec<Cmd> = Vec::new();
            thermal_native(
                &mut want, true, true, "\u{f2c9}", meta, c, empty, !empty, x, y, w, h, grid, &p,
            );
            let tree = thermal(
                true, true, "\u{f2c9}", meta, c, empty, !empty, w, h, grid, &p,
            );
            assert_parity(tree, want, w, h, &format!("empty={empty}"));
        }
    }

    #[test]
    fn the_fans_widget_paints_exactly_like_the_retired_drawer() {
        let p = pal();
        let (w, h, grid) = (200.0f32, 130.0f32, 1.0f32);
        let (x, y) = (0.0f32, 0.0f32);
        let rows = vec![
            ("cpu-fan".to_string(), "1200 rpm".to_string(), 0.5f32, crate::ui::fg2(&p), p.fg),
            ("case".to_string(), "0 rpm".to_string(), 0.0f32, crate::ui::fg2(&p), crate::ui::fg3(&p)),
        ];
        for empty in [false, true] {
            let r: &[(String, String, f32, u32, u32)] = if empty { &[] } else { &rows };
            let meta = if empty { "" } else { "2 fans" };
            let mut want: Vec<Cmd> = Vec::new();
            fans_native(
                &mut want, true, true, "\u{e9e4}", meta, r, empty, !empty, x, y, w, h, grid, &p,
            );
            let tree = fans(
                true, true, "\u{e9e4}", meta, r, empty, !empty, w, h, grid, &p,
            );
            assert_parity(tree, want, w, h, &format!("empty={empty}"));
        }
    }
    #[test]
    fn the_gpu_widget_paints_exactly_like_the_retired_drawer() {
        let p = pal();
        let (w, h, grid) = (200.0f32, 130.0f32, 1.0f32);
        let (x, y) = (0.0f32, 0.0f32);
        let vram = Some(("VRAM 4.0/8.0 GB".to_string(), 0.5f32));
        for empty in [false, true] {
            let mut want: Vec<Cmd> = Vec::new();
            gpu_native(
                &mut want, true, true, empty, 42, 0.5, "52 \u{b0}C", vram.clone(), x, y, w, h,
                grid, &p,
            );
            let tree = gpu(
                true, true, empty, 42, 0.5, "52 \u{b0}C", vram.clone(), w, h, grid, &p,
            )
            .layout(Rect { x, y, w, h }, &mut NoMeasurer);
            let mut got: Vec<Cmd> = Vec::new();
            tree.render(&mut got, &mut NoHit, &p);
            assert_eq!(got, want, "empty={empty}");
        }
    }

    /// The branding card's widget tree emits the EXACT `Cmd` bytes of the
    /// retired `cards::branding_native` drawer (mark and empty states).
    #[test]
    fn the_branding_widget_paints_exactly_like_the_retired_drawer() {
        let p = pal();
        let (w, h, grid) = (200.0f32, 130.0f32, 1.0f32);
        let (x, y) = (0.0f32, 0.0f32);
        for glyph in ["\u{f1b9}", ""] {
            let mut want: Vec<Cmd> = Vec::new();
            branding_native(&mut want, true, true, glyph, 40.0, 60.0, x, y, w, h, grid, &p);
            let tree = branding(true, true, glyph, 40.0, 60.0, h, grid, &p)
                .layout(Rect { x, y, w, h }, &mut NoMeasurer);
            let mut got: Vec<Cmd> = Vec::new();
            tree.render(&mut got, &mut NoHit, &p);
            assert_eq!(got, want, "glyph={glyph:?}");
        }
    }

    /// The moon card's widget tree emits the EXACT `Cmd` bytes of the retired
    /// `cards::moon_native` drawer (disc stamp + phase column).
    #[test]
    fn the_moon_widget_paints_exactly_like_the_retired_drawer() {
        let p = pal();
        let (w, h, grid) = (200.0f32, 130.0f32, 1.0f32);
        let (x, y) = (0.0f32, 0.0f32);
        let epoch = 1_700_000_000.0f64;
        let mut want: Vec<Cmd> = Vec::new();
        moon_native(&mut want, true, true, "\u{f186}", epoch, x, y, w, h, grid, &p);
        let tree = moon(true, true, "\u{f186}", epoch, w, h, grid, &p)
            .layout(Rect { x, y, w, h }, &mut NoMeasurer);
        let mut got: Vec<Cmd> = Vec::new();
        tree.render(&mut got, &mut NoHit, &p);
        assert_eq!(got, want);
    }
    #[test]
    fn the_disk_widget_paints_exactly_like_the_retired_drawer() {
        let p = pal();
        let (w, h, grid) = (200.0f32, 130.0f32, 1.0f32);
        let rows = vec![
            ("/".to_string(), "412G/800G".to_string(), 52i32, p.acc),
            ("/home".to_string(), "1.2T/2.0T".to_string(), 60i32, p.fg),
            ("/boot".to_string(), "0G/1G".to_string(), 0i32, p.fg),
        ];
        let (x, y) = (0.0f32, 0.0f32);
        for live in [true, false] {
            let meta = if live { "3 mounts" } else { "" };
            let mut want: Vec<Cmd> = Vec::new();
            disk_native(&mut want, meta, &rows, live, true, x, y, w, h, grid, &p);
            let tree = disk(meta, &rows, live, true, w, h, grid, &p)
                .layout(Rect { x, y, w, h }, &mut NoMeasurer);
            let mut got: Vec<Cmd> = Vec::new();
            tree.render(&mut got, &mut NoHit, &p);
            assert_eq!(got, want, "live={live}");
        }
    }

    /// The gauges card's widget tree emits the EXACT `Cmd` bytes of the retired
    /// `cards::gauges_native` drawer (two bead columns).
    #[test]
    fn the_gauges_widget_paints_exactly_like_the_retired_drawer() {
        let p = pal();
        let (w, h, grid) = (200.0f32, 130.0f32, 1.0f32);
        let dim = crate::ui::mix(crate::ui::hover(&p), p.fg, 0.08);
        let (x, y) = (0.0f32, 0.0f32);
        let mut want: Vec<Cmd> = Vec::new();
        gauges_native(
            &mut want, "63%", "20G/32G", 23, p.acc, "52%", "412G/800G", 19, p.acc, dim, x, y, w, h,
            grid, &p,
        );
        let tree = gauges(
            "63%", "20G/32G", 23, p.acc, "52%", "412G/800G", 19, p.acc, dim, w, h, grid, &p,
        )
        .layout(Rect { x, y, w, h }, &mut NoMeasurer);
        let mut got: Vec<Cmd> = Vec::new();
        tree.render(&mut got, &mut NoHit, &p);
        assert_eq!(got, want);
    }

    #[test]
    fn the_diskio_widget_paints_exactly_like_the_retired_drawer() {
        let p = pal();
        let (w, h, grid) = (200.0f32, 130.0f32, 1.0f32);
        let (x, y) = (0.0f32, 0.0f32);
        let r_data = vec![0.2f32, 0.8, 0.5, 1.0, 0.3];
        let w_data = vec![0.9f32, 0.1, 0.4, 0.6, 0.2];
        for show_title in [true, false] {
            let mut want: Vec<Cmd> = Vec::new();
            diskio_native(&mut want, show_title, &r_data, &w_data, x, y, w, h, grid, &p);
            let tree = diskio(show_title, &r_data, &w_data, w, h, grid, &p);
            assert_parity(tree, want, w, h, &format!("show_title={show_title}"));
        }
        let empty: Vec<f32> = Vec::new();
        let mut want: Vec<Cmd> = Vec::new();
        diskio_native(&mut want, true, &empty, &empty, x, y, w, h, grid, &p);
        let tree = diskio(true, &empty, &empty, w, h, grid, &p);
        assert_parity(tree, want, w, h, "empty");
    }

    #[test]
    fn the_network_widget_paints_exactly_like_the_retired_drawer() {
        let p = pal();
        let (w, h, grid) = (200.0f32, 130.0f32, 1.0f32);
        let (x, y) = (0.0f32, 0.0f32);
        let down_data = vec![0.2f32, 0.7, 0.4, 1.0];
        let up_data = vec![0.1f32, 0.3, 0.9, 0.5];
        for (show_title, meta) in [(true, "12.4 KiB/s"), (false, "")] {
            let mut want: Vec<Cmd> = Vec::new();
            network_native(
                &mut want, show_title, meta, "1.2M", "240K", &down_data, &up_data, x, y, w, h,
                grid, &p,
            );
            let tree = network(
                show_title, meta, "1.2M", "240K", &down_data, &up_data, w, h, grid, &p,
            );
            assert_parity(tree, want, w, h, &format!("show_title={show_title}"));
        }
    }

    #[test]
    fn the_powerdraw_widget_paints_exactly_like_the_retired_drawer() {
        let p = pal();
        let (w, h, grid) = (200.0f32, 130.0f32, 1.0f32);
        let (x, y) = (0.0f32, 0.0f32);
        let b_data = vec![0.25f32, 0.5, 0.75, 1.0];
        let g_data = vec![1.0f32, 0.75, 0.5, 0.25];
        for (live, idle, empty, bd, gd) in [
            (true, false, false, b_data.as_slice(), g_data.as_slice()),
            (true, true, false, [].as_slice(), [].as_slice()),
            (false, false, true, [].as_slice(), [].as_slice()),
        ] {
            let (b, g) = if live { ("12.3W ↑B", "1.2W G") } else { ("", "") };
            let mut want: Vec<Cmd> = Vec::new();
            powerdraw_native(
                &mut want, true, true, b, g, bd, gd, live, idle, empty, x, y, w, h, grid, &p,
            );
            let tree = powerdraw(
                true, true, b, g, bd, gd, live, idle, empty, w, h, grid, &p,
            );
            assert_parity(tree, want, w, h, &format!("live={live} idle={idle} empty={empty}"));
        }
    }

    #[test]
    fn the_cpugpu_widget_paints_exactly_like_the_retired_drawer() {
        let p = pal();
        let (x, y) = (0.0f32, 0.0f32);
        let cpu_col = crate::ui::mix(p.acc, p.fg, 0.35);
        let gpu_col = crate::ui::mix(crate::ui::INFO, p.fg, 0.1);
        let cpu_data = vec![0.2f32, 0.5, 0.9];
        let gpu_data = vec![0.7f32, 0.3];
        for (w, h, gpu_live, cd, gd) in [
            (200.0f32, 130.0f32, true, cpu_data.as_slice(), gpu_data.as_slice()),
            (200.0f32, 130.0f32, false, cpu_data.as_slice(), [].as_slice()),
            (120.0f32, 60.0f32, true, cpu_data.as_slice(), gpu_data.as_slice()),
            (120.0f32, 40.0f32, true, [].as_slice(), [].as_slice()),
        ] {
            let mut want: Vec<Cmd> = Vec::new();
            cpugpu_native(
                &mut want, "CPU 42%", "GPU 60%", cpu_col, gpu_col, cd, gd, gpu_live, x, y, w, h,
                1.0f32,
                &p,
            );
            let tree = cpugpu(
                "CPU 42%", "GPU 60%", cpu_col, gpu_col, cd, gd, gpu_live, w, h, 1.0f32, &p,
            );
            assert_parity(
                tree,
                want,
                w,
                h,
                &format!("w={w} h={h} gpu_live={gpu_live}"),
            );
        }
    }

    #[test]
    fn the_latency_widget_paints_exactly_like_the_retired_drawer() {
        let p = pal();
        let (w, h, grid) = (200.0f32, 130.0f32, 1.0f32);
        let (x, y) = (0.0f32, 0.0f32);
        let series = vec![0.2f32, 0.4, 0.6, 0.8, 1.0];
        let empty: Vec<f32> = Vec::new();
        for (val, status, s, hover) in [
            ("42", "smooth", series.as_slice(), 0u32),
            ("42", "smooth", series.as_slice(), 31970),
            ("--", "idle", empty.as_slice(), 0),
            ("ERR", "no reply", empty.as_slice(), 31970),
        ] {
            let mut want: Vec<Cmd> = Vec::new();
            latency_native(
                &mut want, true, true, "\u{f0e7}", "8.8.8.8", "\u{f021}", val,
                crate::ui::OK, status, s, hover, x, y, w, h, grid, &p,
            );
            let tree = latency(
                true, true, "\u{f0e7}", "8.8.8.8", "\u{f021}", val, crate::ui::OK, status, s,
                hover, w, h, grid, &p,
            );
            assert_parity(tree, want, w, h, &format!("val={val} hover={hover}"));
        }
    }

    #[test]
    fn the_battery_widget_paints_exactly_like_the_retired_drawer() {
        let p = pal();
        let (w, h, grid) = (200.0f32, 130.0f32, 1.0f32);
        let (x, y) = (0.0f32, 0.0f32);
        let rows = vec![
            ("State".to_string(), "Charging".to_string()),
            ("Battery".to_string(), "63%".to_string()),
            ("Draw".to_string(), "12.4 W".to_string()),
            ("Remaining".to_string(), "1h 20m".to_string()),
        ];
        for horizontal in [true, false] {
            for (pane, present, hover_dots) in
                [(0usize, true, false), (1usize, true, true), (0usize, false, false)]
            {
                let pct_txt = if present { "63%" } else { "" };
                let mut want: Vec<Cmd> = Vec::new();
                battery_native(
                    &mut want, true, true, horizontal, 63, pct_txt, p.acc, pane, present,
                    hover_dots, &rows, 0x334455ff, p.fg, "Power saver", x, y, w, h, grid, &p,
                );
                let tree = battery(
                    true, true, horizontal, 63, pct_txt, p.acc, pane, present, hover_dots, &rows,
                    0x334455ff, p.fg, "Power saver", w, h, grid, &p,
                );
                assert_parity(
                    tree,
                    want,
                    w,
                    h,
                    &format!("h={horizontal} pane={pane} present={present}"),
                );
            }
        }
    }

    #[test]
    fn the_water_widget_paints_exactly_like_the_retired_drawer() {
        let p = pal();
        let (w, h, grid) = (200.0f32, 130.0f32, 1.0f32);
        let (x, y) = (0.0f32, 0.0f32);
        let dim = crate::ui::mix(crate::ui::hover(&p), p.fg, 0.10);
        for (lit, hero, meta) in
            [(5i32, "5", "1.2 / 2.0 L"), (36, "8 \u{2713}", "2.0 / 2.0 L"), (0, "0", "")]
        {
            let mut want: Vec<Cmd> = Vec::new();
            water_native(
                &mut want, true, true, lit, crate::ui::INFO, hero, p.fg, dim,
                "of 8 glasses", meta, 0x222222ff, 0x333333ff, x, y, w, h, grid, &p,
            );
            let tree = water(
                true, true, lit, crate::ui::INFO, hero, p.fg, dim, "of 8 glasses", meta,
                0x222222ff, 0x333333ff, w, h, grid, &p,
            );
            assert_parity(tree, want, w, h, &format!("lit={lit}"));
        }
    }

    #[test]
    fn the_eyerest_widget_paints_exactly_like_the_retired_drawer() {
        let p = pal();
        let (w, h, grid) = (200.0f32, 130.0f32, 1.0f32);
        let (x, y) = (0.0f32, 0.0f32);
        for (lit, meta, tog) in [
            (10i32, "2 sessions", "Rest"),
            (0, "", "Start"),
        ] {
            let mut want: Vec<Cmd> = Vec::new();
            eyerest_native(
                &mut want, true, true, meta, "20:00", p.fg, "Ready", tog, lit,
                crate::ui::OK, 0x222222ff, 0x333333ff, 0, x, y, w, h, grid, &p,
            );
            let tree = eyerest(
                true, true, meta, "20:00", p.fg, "Ready", tog, lit, crate::ui::OK,
                0x222222ff, 0x333333ff, 0, w, h, grid, &p,
            );
            assert_parity(tree, want, w, h, &format!("lit={lit} meta={meta:?}"));
        }
    }

    #[test]
    fn the_pomodoro_widget_paints_exactly_like_the_retired_drawer() {
        let p = pal();
        let (w, h, grid) = (200.0f32, 130.0f32, 1.0f32);
        let (x, y) = (0.0f32, 0.0f32);
        for (idle, hover, tog) in [
            (true, 0u32, "Start"),
            (false, 0, "Pause"),
            (true, 30730, "Start"),
            (true, 30731, "Start"),
            (true, 30732, "Start"),
            (true, 30733, "Start"),
        ] {
            let mut want: Vec<Cmd> = Vec::new();
            pomodoro_native(
                &mut want, true, true, "2 sessions", "20:00", p.fg, p.acc, "Focus",
                0.5, idle, "25m", -30.0, 30.0, 0x222222ff, 0x333333ff, 0x444444ff, tog,
                hover, x, y, w, h, grid, &p,
            );
            let tree = pomodoro(
                true, true, "2 sessions", "20:00", p.fg, p.acc, "Focus", 0.5, idle, "25m",
                -30.0, 30.0, 0x222222ff, 0x333333ff, 0x444444ff, tog, hover, w, h, grid, &p,
            );
            assert_parity(tree, want, w, h, &format!("idle={idle} hover={hover}"));
        }
    }

    #[test]
    fn the_micmeter_widget_paints_exactly_like_the_retired_drawer() {
        let p = pal();
        let (w, h, grid) = (200.0f32, 130.0f32, 1.0f32);
        let (x, y) = (0.0f32, 0.0f32);
        let dim = crate::ui::fg3(&p);
        for (lvl, meta, status, btn) in [
            (0.5f32, " 50%", "listening", "Mute mic"),
            (0.0, "", "muted", "Unmute mic"),
        ] {
            let mut want: Vec<Cmd> = Vec::new();
            micmeter_native(
                &mut want, true, true, "\u{f130}", meta, lvl, p.acc, status, dim,
                0x222222ff, btn, p.fg, x, y, w, h, grid, &p,
            );
            let tree = micmeter(
                true, true, "\u{f130}", meta, lvl, p.acc, status, dim, 0x222222ff, btn,
                p.fg, w, h, grid, &p,
            );
            assert_parity(tree, want, w, h, &format!("lvl={lvl}"));
        }
    }

    #[test]
    fn the_activewin_widget_paints_exactly_like_the_retired_drawer() {
        let p = pal();
        let (w, h, grid) = (200.0f32, 130.0f32, 1.0f32);
        let (x, y) = (0.0f32, 0.0f32);
        for (empty, class, title, rss) in [
            (true, "", "", None),
            (true, "", "", Some("1.2 GB")),
            (false, "firefox", "Docs \u{2014} Rust", Some("1.2 GB")),
            (false, "kitty", "shell", None),
        ] {
            let mut want: Vec<Cmd> = Vec::new();
            activewin_native(&mut want, empty, class, title, rss, x, y, w, h, grid, &p);
            let tree = activewin(empty, class, title, rss, w, h, grid, &p);
            assert_parity(tree, want, w, h, &format!("empty={empty} rss={rss:?}"));
        }
    }

    #[test]
    #[allow(clippy::type_complexity)]
    fn the_todo_widget_paints_exactly_like_the_retired_drawer() {
        let p = pal();
        let (w, h, grid) = (200.0f32, 130.0f32, 1.0f32);
        let (x, y) = (0.0f32, 0.0f32);
        use crate::shell::{TODO_KEY_ADD, TODO_KEY_DELETE, TODO_KEY_INPUT, TODO_KEY_TOGGLE};
        let n10: Vec<(String, bool)> = (0..10)
            .map(|i| (format!("task {i}"), i % 3 == 0))
            .collect();
        let cases: Vec<(Vec<(String, bool)>, usize, Option<&str>, u32)> = vec![
            (vec![], 0, None, 0),
            (
                vec![
                    ("milk".into(), true),
                    ("eggs".into(), false),
                    ("bread".into(), false),
                ],
                0,
                None,
                0,
            ),
            (
                vec![("milk".into(), true), ("eggs".into(), false)],
                0,
                None,
                TODO_KEY_TOGGLE,
            ),
            (
                vec![("milk".into(), true), ("eggs".into(), false)],
                0,
                None,
                TODO_KEY_DELETE,
            ),
            (vec![("milk".into(), false)], 0, Some("buy milk"), 0),
            (vec![("milk".into(), false)], 0, Some("   "), 0),
            (n10, 3, None, 0),
        ];
        for (todos, scroll, input, hover) in cases {
            let mut want: Vec<Cmd> = Vec::new();
            todo_native(
                &mut want, &todos, scroll, input, false, false, false, true, TODO_KEY_TOGGLE,
                TODO_KEY_DELETE, TODO_KEY_INPUT, TODO_KEY_ADD, hover, x, y, w, h, grid, &p,
            );
            let tree = todo(
                &todos, scroll, input, false, false, false, true, TODO_KEY_TOGGLE,
                TODO_KEY_DELETE, TODO_KEY_INPUT, TODO_KEY_ADD, hover, w, h, grid, &p,
            );
            assert_parity(tree, want, w, h, &format!("n={} hover={hover}", todos.len()));
        }
    }

    #[test]
    #[allow(clippy::type_complexity)]
    fn the_notes_widget_paints_exactly_like_the_retired_drawer() {
        let p = pal();
        let (w, h, grid) = (200.0f32, 130.0f32, 1.0f32);
        let (x, y) = (0.0f32, 0.0f32);
        use crate::shell::{NOTES_KEY_INPUT, NOTES_KEY_SAVE};
        let list: Vec<(String, String)> = vec![
            ("Groceries".into(), "milk, eggs, bread".into()),
            ("Ideas".into(), String::new()),
            (
                "A very long note title that runs past the truncation limit".into(),
                "body text here".into(),
            ),
        ];
        let cases: Vec<(
            Vec<(String, String)>,
            &str,
            &str,
            Option<u8>,
            usize,
            u32,
        )> = vec![
            (vec![], "", "", None, 0, 0),
            (list.clone(), "", "", None, 0, NOTES_KEY_INPUT),
            (list, "", "", None, 1, 0),
            (vec![], "Draft title", "line one\nline two", Some(0), 0, 0),
            (vec![], "", "line one\nline two\nthree", Some(1), 0, NOTES_KEY_SAVE),
            (vec![], "", "", Some(1), 0, 0),
        ];
        for (ns, title, body, input, scroll, hover) in cases {
            let mut want: Vec<Cmd> = Vec::new();
            notes_native(
                &mut want, &ns, title, body, input, scroll, false, false,
                NOTES_KEY_INPUT, NOTES_KEY_SAVE, hover, x, y, w, h, grid, &p,
            );
            let tree = notes(
                &ns, title, body, input, scroll, false, false, NOTES_KEY_INPUT,
                NOTES_KEY_SAVE, hover, w, h, grid, &p,
            );
            assert_parity(tree, want, w, h, &format!("n={} input={input:?}", ns.len()));
        }
    }
