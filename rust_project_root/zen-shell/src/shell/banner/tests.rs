    use super::*;

    fn sample_shell() -> Shell {
        let cfg: crate::config::Config =
            toml::from_str(crate::config::DEFAULT_SHELL_TOML).expect("default config parses");
        Shell::new(cfg)
    }

    #[test]
    fn whole_strip_horizontal_scroll_when_narrow_panel() {
        let mut shell = sample_shell();
        shell.mode = Mode::Expanded;
        // default strip has NO L/C/R zone markers — many chips on a narrow
        // panel must overflow into a single horizontally-scrollable row
        shell.banner_order = DEFAULT_BANNER_ORDER
            .iter()
            .copied()
            .map(BannerToken::Chip)
            .collect();
        let row_w = 320.0;
        shell.strip_row_w = row_w;
        assert!(
            shell.banner_strip_overflow(),
            "default chips overflow a 320px panel"
        );
        let cells0 = shell.banner_cells(row_w);
        let (_, (fx0, _)) = cells0
            .iter()
            .enumerate()
            .find(|(_, c)| c.is_some())
            .map(|(i, c)| (i, c.unwrap()))
            .unwrap();
        assert!(
            (fx0 - shell.scale.s(10.0)).abs() < 0.001,
            "first chip at the left margin"
        );
        assert!(shell.banner_strip_scroll_by(4), "wheel scrolls the strip");
        let before = shell.banner_strip_scroll;
        assert!(before > 0.0);
        let cells1 = shell.banner_cells(row_w);
        let (_, (fx1, _)) = cells1
            .iter()
            .enumerate()
            .find(|(_, c)| c.is_some())
            .map(|(i, c)| (i, c.unwrap()))
            .unwrap();
        assert!(fx1 < fx0 - 0.5, "cells pan left by the scroll offset");
        assert!(shell.banner_strip_scroll_by(-400), "scrolling back works");
        assert_eq!(shell.banner_strip_scroll, 0.0, "resets at the start");
    }

    #[test]
    fn no_strip_scroll_when_content_fits() {
        let mut shell = sample_shell();
        shell.mode = Mode::Expanded;
        shell.banner_order = vec![BannerToken::Chip(BannerItem::Clock)];
        let row_w = 800.0;
        shell.strip_row_w = row_w;
        assert!(
            !shell.banner_strip_overflow(),
            "one chip fits an 800px panel"
        );
        assert!(
            !shell.banner_strip_scroll_by(3),
            "no overflow → wheel not consumed"
        );
    }
