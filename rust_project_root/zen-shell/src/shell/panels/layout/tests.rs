    use super::*;

    /// Reference shell built from the pinned default config.
    fn sample_shell() -> Shell {
        let cfg: Config =
            toml::from_str(crate::config::DEFAULT_SHELL_TOML).expect("default config parses");
        Shell::new(cfg)
    }

    #[test]
    fn surface_size_reported() {
        let shell = sample_shell();
        let lt = Layout::compute(&shell, 900.0, 480.0);
        assert_eq!(lt.w, 900.0);
        assert_eq!(lt.h, 480.0);
        assert!(!lt.editing, "fresh shell is not in edit mode");
    }

    #[test]
    fn geometry_matches_shell_metrics() {
        let shell = sample_shell();
        let ew = shell.cfg.expanded_w();
        let lt = Layout::compute(&shell, 1100.0, 720.0);
        assert!((lt.cell_w - shell.dash_cell_base_w(ew)).abs() < 0.001);
        assert!((lt.cell_h - shell.dash_cell_h(ew)).abs() < 0.001);
        assert!((lt.pad - shell.dash_pad()).abs() < 0.001, "pad");
        assert!((lt.gap - shell.dash_gap()).abs() < 0.001, "gap");
        assert!((lt.card_r - shell.card_r()).abs() < 0.001, "card radius");
        assert!((lt.grid_top - shell.grid_top()).abs() < 0.001, "grid_top");
        assert!(
            (lt.strip_top - shell.banner_strip_top()).abs() < 0.001,
            "strip_top"
        );
        assert!(
            (lt.strip_h - shell.banner_strip_h()).abs() < 0.001,
            "strip_h"
        );
        assert_eq!(lt.mask_bottom, 0.0, "non-edit mode draws no chrome mask");
    }

    #[test]
    fn grid_starts_below_banner_strip() {
        let shell = sample_shell();
        let lt = Layout::compute(&shell, 900.0, 480.0);
        assert!(
            lt.strip_top + lt.strip_h <= lt.grid_top + 0.001,
            "banner strip must sit entirely above the card grid start"
        );
        assert!(lt.grid_top > 0.0);
    }

    #[test]
    fn edit_chrome_stacks_above_grid() {
        let mut shell = sample_shell();
        shell.dash_edit = true;
        // A card on the grid keeps the dashboard area non-collapsed; without
        // one the chrome rows may overlap the (empty) grid start — that's the
        // case `mask_bottom`'s `max()` exists for.
        shell.dash_layout.push(CardLayout {
            card: DashCard::System,
            x: 0,
            y: 0,
            w: 2,
            h: 2,
        });
        let lt = Layout::compute(&shell, 1400.0, 960.0);
        assert!(lt.editing, "edit mode flags the layout");
        // chrome rows stack strictly top-to-bottom …
        assert!(lt.tray_top + lt.banner_tray_h <= lt.ui_ctrl_top + 0.001);
        assert!(lt.ui_ctrl_top + lt.ui_ctrl_h <= lt.banner_ctrl_top + 0.001);
        assert!(lt.banner_ctrl_top + lt.banner_ctrl_h <= lt.grid_top + 0.001);
        // … and the strip-editor row sits above the card grid.
        assert!(
            lt.banner_ctrl_top + lt.banner_ctrl_h <= lt.grid_top + 0.001,
            "strip-editor row must sit above the grid start"
        );
        assert_eq!(lt.mask_bottom, lt.grid_top);
    }

    #[test]
    fn tray_gap_split_between_chrome_rows() {
        let mut shell = sample_shell();
        shell.dash_edit = true;
        // make the parking tray non-collapsed: park every chip by moving it
        // off the strip (a fresh default config may park no chips at all)
        shell
            .banner_order
            .retain(|t| !matches!(t, BannerToken::Chip(_)));
        let lt = Layout::compute(&shell, 1200.0, 800.0);
        let gap = lt.tray_gap;
        assert!(gap > 0.0);
        assert!(!shell.banner_chips_absent(), "tray must hold a parked chip");
        // ctrl rows stack with the tray_gap between them: the UI controls row
        // (pad/win/card) sits directly under the tray, the strip-editor row
        // below that
        assert!((lt.ui_ctrl_top - lt.tray_top - lt.banner_tray_h).abs() - gap < 0.001);
        assert!((lt.banner_ctrl_top - lt.ui_ctrl_top - lt.ui_ctrl_h).abs() - gap < 0.001);
    }

    #[test]
    fn clear_all_parks_board_cards_into_tray() {
        let mut shell = sample_shell();
        shell.dash_layout.push(CardLayout {
            card: DashCard::System,
            x: 0,
            y: 0,
            w: 2,
            h: 2,
        });
        shell.dash_layout.push(CardLayout {
            card: DashCard::Weather,
            x: 2,
            y: 0,
            w: 2,
            h: 2,
        });
        shell.tray_cards.push(DashCard::Clock);
        shell.park_all_cards();
        assert!(shell.dash_layout.len() == 2, "board is untouched by park");
        assert!(
            shell.tray_cards.contains(&DashCard::System),
            "System parked"
        );
        assert!(
            shell.tray_cards.contains(&DashCard::Weather),
            "Weather parked"
        );
        assert!(
            shell.tray_cards.contains(&DashCard::Clock),
            "parked card preserved"
        );
        assert!(
            !shell.dash_area_collapsed(),
            "non-empty tray keeps dashboard area"
        );
    }

    #[test]
    fn expanded_width_follows_dashboard_columns() {
        let mut shell = sample_shell();
        shell.mode = crate::shell::Mode::Expanded;
        shell.dash_enabled = true;
        shell.dash_layout.push(CardLayout {
            card: DashCard::System,
            x: 0,
            y: 0,
            w: 2,
            h: 2,
        });
        shell.dash_layout.push(CardLayout {
            card: DashCard::Weather,
            x: 2,
            y: 0,
            w: 2,
            h: 2,
        });
        shell.refresh_pack();
        assert!(!shell.dash_area_collapsed());
        let (w, _) = shell.target_size();
        let cw = shell.dash_cell_base_w(shell.cfg.expanded_w());
        let expected_w = (shell.dash_pad() * 2.0 + 4.0 * (cw + shell.dash_gap())
            - shell.dash_gap())
        .max(320.0)
        .min(shell.dash_view_w());
        assert!(
            (w - expected_w).abs() < 1.0,
            "width follows the 4-column extent, got {w}"
        );
        assert!(
            (w - shell.cfg.expanded_w()).abs() > 10.0,
            "fewer columns must NOT keep the full fixed expanded width"
        );
    }

    #[test]
    fn banner_tray_collapses_when_no_parked_chips() {
        let mut shell = sample_shell();
        shell.dash_edit = true;
        // park nothing: put every canonical chip back on the strip so the
        // tray collapses to zero height
        shell
            .banner_order
            .retain(|t| !matches!(t, BannerToken::Chip(_)));
        for b in DEFAULT_BANNER_ORDER {
            shell.banner_order.push(BannerToken::Chip(*b));
        }
        let lt = Layout::compute(&shell, 1200.0, 800.0);
        assert!(shell.banner_chips_absent());
        assert_eq!(lt.banner_tray_h, 0.0);
        // the UI controls row moves straight up to the tray top when the tray
        // collapses (no gap consumed)
        assert!((lt.ui_ctrl_top - lt.tray_top).abs() < 0.001);
        // and the strip-editor row follows below it
        assert!((lt.banner_ctrl_top - lt.ui_ctrl_top - lt.ui_ctrl_h).abs() - lt.tray_gap < 0.001);
    }
