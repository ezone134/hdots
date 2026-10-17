    use super::*;

    /// Serializes tests that touch the process-global world store (any test
    /// can `set_world`/`clear`, so behavior-sensitive tests must not race).
    pub static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn ring_chunk_roundtrips() {
        let buf = vec![
            0x01, 0x00, 0x00, 0x00, // 1 ring
            0x03, 0x00, 0x00, 0x00, // 3 points
            0x64, 0x00, 0x64, 0x00, // (100, 100) → (1.00, 1.00)
            0xC4, 0xFF, 0xCE, 0xFF, // (-60, -50) → (-0.60, -0.50)
            0x00, 0x00, 0xC8, 0x00, // (0, 200) → (0.00, 2.00)
        ];
        let rings = parse_ring_chunk(&buf).expect("chunk parses");
        assert_eq!(rings.len(), 1);
        assert_eq!(rings[0][0], (1.0, 1.0));
        assert_eq!(rings[0][1], (-0.6, -0.5));
        assert_eq!(rings[0][2], (0.0, 2.0));
    }

    #[test]
    fn city_chunk_roundtrips() {
        let mut buf = vec![1u8, 0, 0, 0];
        for v in [3i16, 4i16] {
            buf.extend_from_slice(&v.to_le_bytes());
        }
        let name = "Rio";
        buf.extend_from_slice(&(name.len() as u16).to_le_bytes());
        buf.extend_from_slice(name.as_bytes());
        let cities = parse_cities_chunk(&buf).expect("cities parse");
        assert_eq!(cities.len(), 1);
        assert_eq!(cities[0].name, "Rio");
        assert_eq!(cities[0].lon, 0.03);
        assert_eq!(cities[0].lat, 0.04);
    }

    #[test]
    fn mask_builds_without_panic_and_has_land() {
        let _g = TEST_LOCK.lock().unwrap();
        // a single square covering lon 0..10, lat 0..10
        let rings = vec![vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]];
        let w = set_world(rings).expect("set");
        assert!(
            land_lonlat(&w, 2.0, 2.0),
            "inside the square should be land"
        );
        assert!(!land_lonlat(&w, -45.0, -20.0), "elsewhere is ocean");
    }

    #[test]
    fn raster_band_paints_land_ocean_and_rings() {
        let _g = TEST_LOCK.lock().unwrap();
        clear();
        // a square covering lon 0..60, lat 0..40 (lon 0..10 test square is too
        // tiny at whole-world scale — use the card-test's bigger square)
        let rings = vec![vec![(0.0, 0.0), (60.0, 0.0), (60.0, 40.0), (0.0, 40.0)]];
        let w = set_world(rings).expect("set");
        let win = Win {
            lon0: -180.0,
            lat0: -85.0,
            lon1: 180.0,
            lat1: 85.0,
        };
        let colors = [
            0x141414ff, 0x4fc2ffff, 0x404040ff, 0x222222ff, 0x333333ff, 0x43a0e0ff,
        ];
        let (bw, bh) = (360usize, 170usize);
        // filled style: land tinted, ocean transparent, no ring overdraw
        let px = raster_band(bw, bh, &win, &w, None, false, 1, colors);
        let at = |lon: f32, lat: f32| -> usize {
            let x =
                ((((lon - win.lon0) / win.lon_span()) * bw as f32).floor() as usize).min(bw - 1);
            let y =
                ((((win.lat1 - lat) / win.lat_span()) * bh as f32).floor() as usize).min(bh - 1);
            (y * bw + x) * 4
        };
        let i = at(30.0, 20.0);
        assert_eq!(
            [px[i], px[i + 1], px[i + 2], px[i + 3]],
            [0x4f, 0xc2, 0xff, 0xff],
            "land tint"
        );
        let j = at(120.0, -30.0);
        assert_eq!(px[j + 3], 0, "ocean stays transparent in filled style");
        // real style: base backdrop + border rings drawn over land/ocean edge
        let px2 = raster_band(bw, bh, &win, &w, None, false, 0, colors);
        let k = at(20.0, -45.0); // ocean — the raised backdrop must be there
        assert_eq!(px2[k], 0x14, "raised backdrop fills the pane");
        let k = at(0.0, 20.0); // near the ring's left edge (lon 0)
        assert_ne!(px2[k], 0x14, "ring stroke over the backdrop at lon≈0");
        clear();
    }

    #[test]
    fn embedded_world_is_always_parseable() {
        let _g = TEST_LOCK.lock().unwrap();
        // the compiled-in chunk must always yield a real map
        let rings = parse_ring_chunk(EMBEDDED_WORLD).expect("embedded chunk parses");
        assert!(!rings.is_empty(), "embedded chunk has rings");
        let total: usize = rings.iter().map(|r| r.len()).sum();
        assert!(
            total > 1000,
            "embedded chunk is a real dataset ({total} pts)"
        );
        clear();
        ensure_fallback();
        assert!(ready(), "fallback makes the store ready");
        assert!(using_fallback(), "fallback flag set by ensure_fallback");
        let w = world().expect("world present");
        // somewhere in Brazil should be land with the embedded 110m dataset
        assert!(land_lonlat(&w, -52.0, -12.0), "South America must be land");
        clear();
        assert!(!using_fallback(), "clear resets the fallback flag");
    }

    #[test]
    fn set_world_marks_non_fallback() {
        let _g = TEST_LOCK.lock().unwrap();
        clear();
        ensure_fallback();
        assert!(using_fallback(), "precondition: fallback active");
        set_world(vec![vec![(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)]]);
        assert!(
            !using_fallback(),
            "a swapped-in world replaces the fallback"
        );
        clear();
    }

    #[test]
    fn land_tracking_matches_fill() {
        let _g = TEST_LOCK.lock().unwrap();
        // equirect square spanning the equator
        let rings = vec![vec![(-20.0, -5.0), (20.0, -5.0), (20.0, 5.0), (-20.0, 5.0)]];
        let w = set_world(rings).expect("set");
        assert!(land_lonlat(&w, 0.0, 0.0), "equator centre should be land");
        assert!(!land_lonlat(&w, 0.0, 40.0), "north should be ocean");
    }

    #[test]
    fn window_and_projection_roundtrip() {
        // whole-world window at zoom 1 → identical to the old full map
        let wn = win_for(0.0, 0.0, 1.0);
        assert!((wn.lon0 - -180.0).abs() < 0.001);
        assert!((wn.lon1 - 180.0).abs() < 0.001);
        assert!((wn.lat0 - -85.0).abs() < 0.001);
        assert!((wn.lat1 - 85.0).abs() < 0.001);

        let (mx, my, mw, mh) = (10.0f32, 20.0f32, 400.0f32, 220.0f32);
        for (lon, lat) in [(0.0, 0.0), (-150.0, -10.0), (179.0, 80.0), (-179.0, -80.0)] {
            let (px, py) = proj(mx, my, mw, mh, &wn, lon, lat);
            let (rl, ra) = unproj(mx, my, mw, mh, &wn, px, py);
            assert!((rl - lon).abs() < 0.6, "lon mismatch {rl} vs {lon}");
            assert!((ra - lat).abs() < 0.6, "lat mismatch {ra} vs {lat}");
        }

        // zoomed window roundtrip around Tokyo
        let wz = win_for(139.0, 35.0, 8.0);
        assert!(wz.contains(139.0, 35.0), "center must be inside the window");
        for (lon, lat) in [(139.7, 35.7), (139.0, 35.0)] {
            let (px, py) = proj(mx, my, mw, mh, &wz, lon, lat);
            let (rl, ra) = unproj(mx, my, mw, mh, &wz, px, py);
            assert!((rl - lon).abs() < 0.1, "zoomed lon mismatch {rl} vs {lon}");
            assert!((ra - lat).abs() < 0.1, "zoomed lat mismatch {ra} vs {lat}");
        }
    }

    #[test]
    fn window_clamps_to_world() {
        let w = win_for(190.0, 89.0, 2.0);
        assert!(
            w.lon0 >= -180.0 && w.lon1 <= 180.0,
            "lon must stay in world"
        );
        assert!(
            w.lat0 >= -85.0 && w.lat1 <= 85.0,
            "lat must stay in world bounds"
        );
        let w = win_for(-200.0, 0.0, 2.0);
        assert!(
            w.lon0 >= -180.0 && w.lon1 <= 180.0,
            "lon must stay in world"
        );
    }

    #[test]
    fn zone_tab_parser_reads_columns() {
        let txt = "AD\t+4230+00131\tEurope/Andorra\tAndorra\nJP\t+353916+1394441\tAsia/Tokyo\n";
        let z = parse_zone_tab(txt);
        assert_eq!(z.len(), 2);
        assert_eq!(z[0].tz, "Europe/Andorra");
        assert_eq!(z[0].city, "Andorra");
        assert!((z[0].lat - 42.5).abs() < 0.01);
        assert!((z[0].lon - 1.516).abs() < 0.02);
        assert!((z[1].lat - 35.654444).abs() < 0.01, "lat {}", z[1].lat);
        assert!((z[1].lon - 139.744_72).abs() < 0.01, "lon {}", z[1].lon);
        assert_eq!(z[1].city, "Tokyo");
    }

    #[test]
    fn nearest_zone_picks_closest() {
        let txt = "AD\t+4230+00131\tEurope/Andorra\tAndorra\nJP\t+353916+1394441\tAsia/Tokyo\n";
        let z = parse_zone_tab(txt);
        let idx = z
            .iter()
            .enumerate()
            .min_by(|a, b| {
                let da = (a.1.lat - 35.0).powi(2) + (a.1.lon - 139.0).powi(2);
                let db = (b.1.lat - 35.0).powi(2) + (b.1.lon - 139.0).powi(2);
                da.partial_cmp(&db).unwrap()
            })
            .map(|(i, _)| i)
            .unwrap();
        assert_eq!(idx, 1);
    }

    #[test]
    fn tz_offset_parsing() {
        assert_eq!(parse_tz_offset("+0530"), Some(330));
        assert_eq!(parse_tz_offset("-0330"), Some(-210));
        assert_eq!(parse_tz_offset("+0000"), Some(0));
        assert_eq!(parse_tz_offset("bogus"), None);
        assert_eq!(parse_tz_offset("+05:30"), Some(330));
        assert_eq!(parse_tz_offset("-05"), Some(-300));
    }

    #[test]
    fn world_time_shift() {
        let e0 = 1716268800;
        let s = world_hhmm(e0, 0);
        assert_eq!(s.len(), 5);
        let shifted = world_hhmm(e0, 60);
        assert_ne!(s, shifted, "one-hour shift must change the clock");
        let d = world_dayline(e0, 0);
        assert!(d.contains(' '));
    }
