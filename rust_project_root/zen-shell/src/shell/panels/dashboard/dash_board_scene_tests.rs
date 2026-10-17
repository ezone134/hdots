    use super::*;
    use crate::scene::SceneColor;


    /// A board with one card and a non-empty viewing strip, isolated from the
    /// live per-channel config (layout persists to it).
    fn dash_shell(tag: &str) -> Shell {
        let cfg: Config =
            toml::from_str(crate::config::DEFAULT_SHELL_TOML).expect("default config parses");
        let mut s = Shell::new(cfg);
        s.per_state_config =
            std::env::temp_dir().join(format!("zen-test-dash-board-{tag}.toml"));
        s.mode = crate::shell::Mode::Expanded;
        s.dash_edit = false;
        s.dash_enabled = true;
        s.dash_layout = vec![CardLayout {
            card: DashCard::Bluetooth,
            x: 0,
            y: 0,
            w: 6,
            h: 4,
        }];
        s.refresh_pack();
        // one chip so `banner_shown` is up: the band and the strip are drawn
        s.banner_order = vec![BannerToken::Chip(BannerItem::Clock)];
        s
    }

    /// THE BUG THIS CONVERSION FIXED: the declared plate, the opaque band and
    /// the strip chips must paint in scene order — card first, band over the
    /// card, chips over the band. Before, the grid was a Rust Ink whose body
    /// was drawn LAST-in-its-slot, so `dband` covered the cards and the chips.
    #[test]
    fn the_board_paints_under_the_strip_band_and_the_chips() {
        let mut s = dash_shell("zorder");
        let v = s.layout(1200.0, 800.0);
        // the strip band: an opaque theme-bg box the strip's height, wide as
        // the board (the LAST such box — the cards paint before it)
        let opaque = (s.sv_bg & 0xffffff00) | 0xff;
        let (band, band_x, band_y, band_w, band_h) = v
            .iter()
            .enumerate()
            .filter_map(|(i, c)| match c {
                Cmd::RectConcave { x, y, w, h, color, .. }
                    if *color == opaque && (10.0..40.0).contains(h) && *w > 100.0 =>
                    Some((i, *x, *y, *w, *h)),
                _ => None,
            })
            .last()
            .expect("the dband component painted the strip band");
        // the board's card, painted by its `Card` delegate
        let card = v
            .iter()
            .position(|c| matches!(c, Cmd::Text { text, .. } if text == "Bluetooth"))
            .expect("the Card delegate drew the Bluetooth body");
        // a strip chip: a BannerRow cell inside the band's box
        let chip = v
            .iter()
            .enumerate()
            .find_map(|(i, c)| match c {
                Cmd::Text { x, y, .. }
                    if i > band
                        && (band_x - 1.0..=band_x + band_w).contains(x)
                        && (band_y - 1.0..=band_y + band_h).contains(y) =>
                    Some(i),
                _ => None,
            })
            .expect("the dchips component painted a strip chip");
        assert!(
            card < band,
            "the card body must paint BEFORE the band (card@{card}, band@{band})"
        );
        assert!(
            band < chip,
            "the band must paint BEFORE the chips (band@{band}, chip@{chip})"
        );
    }

    /// The plate is not just painted: its hit region reaches the shell under
    /// the card's key, so hovering/clicking a board card still works with the
    /// Rust grid painter standing down.
    #[test]
    fn the_declared_board_registers_the_card_hit_region() {
        let mut s = dash_shell("region");
        let v = s.layout(1200.0, 800.0);
        let plate = v
            .iter()
            .position(|c| matches!(c, Cmd::Text { text, .. } if text == "Bluetooth"))
            .expect("the card body drew");
        let (rx, ry, rw, rh, key) = *s
            .hover_regions
            .iter()
            .find(|r| r.4 == crate::shell::DASH_CARD_KEY)
            .expect("the card region registered");
        // the region sits on the plate that carried it
        let plate_x = v
            .iter()
            .take(plate)
            .rev()
            .find_map(|c| match c {
                Cmd::Rect { x, y, w, h, color, .. }
                    if *w > 40.0 && *y > 38.0 && *color != (s.sv_bg & 0xffffff00) | 0xff =>
                {
                    Some((*x, *y, *w, *h))
                }
                _ => None,
            })
            .expect("a plate rect under the card body");
        assert!(
            (rx - plate_x.0).abs() < 1.0
                && (ry - plate_x.1).abs() < 1.0
                && (rw - plate_x.2).abs() < 1.0
                && (rh - plate_x.3).abs() < 1.0,
            "region {rx:?},{ry:?},{rw:?},{rh:?} must sit on the plate {plate_x:?}"
        );
        assert_eq!(key, crate::shell::DASH_CARD_KEY, "first card's key");
    }

    /// THE ENTRY POINT THE DEAD DRAWER USED TO OWN: `CONN_WIFI_KEY` /
    /// `CONN_BT_KEY` were registered ONLY by `layout_banner`'s popover arm —
    /// which the `!chips_via_scene` gate had made dead code while the declared
    /// surface owned the strip. The net chip could then open a menu that never
    /// painted, under rows nothing could click. The declared `dconn`
    /// component registers both again, from the surface.
    #[test]
    fn the_dashboard_surface_registers_both_connectivity_jump_keys() {
        let mut s = dash_shell("connkeys");
        s.banner_order = vec![
            BannerToken::Chip(BannerItem::Connectivity),
            BannerToken::Chip(BannerItem::Clock),
        ];
        s.connectivity_open = true;
        let v = s.layout(1200.0, 800.0);
        // registered AND visible — the keys belong to a popover actually on
        // screen, not to a menu drawn nowhere
        let shadow = (s.sv_bg & 0xffffff00) | 0x2e;
        assert!(
            v.iter()
                .any(|c| matches!(c, Cmd::Rect { color, .. } if *color == shadow)),
            "dconn painted the popover's shadow ring"
        );
        let rows: Vec<(f32, f32, f32, f32)> = [CONN_WIFI_KEY, CONN_BT_KEY]
            .iter()
            .map(|key| {
                let r = s
                    .hover_regions
                    .iter()
                    .find(|r| r.4 == *key)
                    .unwrap_or_else(|| panic!("key {key} must register so its click handler runs"));
                assert!(
                    r.2 > 0.0 && r.3 > 0.0,
                    "key {key} region has no size: {r:?}"
                );
                (r.0, r.1, r.2, r.3)
            })
            .collect();
        let (wifi, bt) = (rows[0], rows[1]);
        assert!(
            (wifi.0 - bt.0).abs() < 0.5 && (wifi.2 - bt.2).abs() < 0.5,
            "the two rows must share the popover's column: {wifi:?} vs {bt:?}"
        );
        assert!(
            wifi.1 + wifi.3 <= bt.1 + 0.5,
            "the Wi-Fi row must sit above the Bluetooth row: {wifi:?} vs {bt:?}"
        );
    }

    /// The pool the scene binds against: the plate carries the same alpha the
    /// Settings slider drives, and the radius is `card_r()` in BASE px (an
    /// `r_var` is scaled by the scene, like every other extent).
    #[test]
    fn the_board_pool_carries_the_painters_own_plate() {
        let s = dash_shell("pool");
        let vals = s.scene_values();
        let bg = match vals.color("dash_card_bg") {
            Some(SceneColor::Raw(v)) => v,
            other => panic!("dash_card_bg must be a resolved plate color, got {other:?}"),
        };
        assert_eq!(
            bg & 0xff,
            s.dash_card_alpha as u32,
            "the plate alpha is the Settings slider's"
        );
        let r = vals.scalar("dash_card_r").expect("dash_card_r published");
        assert!(
            (r - s.scale.base(s.card_r())).abs() < 0.001,
            "the plate radius is card_r() in base px, got {r}"
        );
    }

    /// THE BUG THE `dash_chips_via_scene` FLAG OPENED: the flag read the
    /// state of a DECLARED `dashboard` surface, but it gated the Rust chip
    /// sweep in `layout_banner` — which also runs on the FALLBACK path (a
    /// config with no `dashboard` surface, or the edit board). Cells
    /// published, the flag went up, and the fallback strip went BLANK. With
    /// the flag deleted there is no seam left: `layout_banner` only ever
    /// runs where nothing declared has drawn, so it always paints.
    #[test]
    fn the_fallback_banner_always_draws_its_strip_chips() {
        let mut s = dash_shell("fallback");
        let pal = Pal {
            fg: 0xff_ffff,
            bg: 0x000000,
            acc: 0x00ff_00,
            sfg: 0x888888,
        };
        let w = 1200.0;
        let lt = Layout::compute(&s, w, 800.0);
        let mut v = Vec::new();
        s.hover_regions.clear();
        s.layout_banner(&mut v, w, &lt, None, &pal);
        assert!(
            s.hover_regions.iter().any(|r| r.4 == BANNER_KEY_BASE),
            "the fallback chip sweep registered the clock chip's slot"
        );
        let (by, bh) = (lt.strip_top, lt.strip_h);
        assert!(
            v.iter()
                .any(|c| matches!(c, Cmd::Text { y, .. } if (*y - by).abs() <= bh)),
            "the fallback chip sweep painted a chip inside the strip band"
        );
    }
