    use super::*;

    /// A dashboard shell with the Bluetooth card on the board, isolated from
    /// the live per-channel config (save_config writes there).
    fn bt_shell(tag: &str) -> Shell {
        let cfg: Config =
            toml::from_str(crate::config::DEFAULT_SHELL_TOML).expect("default config parses");
        let mut s = Shell::new(cfg);
        s.per_state_config = std::env::temp_dir().join(format!("zen-test-card-toggles-{tag}.toml"));
        s.mode = crate::shell::Mode::Expanded;
        s.dash_enabled = true;
        s.dash_layout = vec![CardLayout {
            card: DashCard::Bluetooth,
            x: 0,
            y: 0,
            w: 6,
            h: 4,
        }];
        s.refresh_pack();
        s
    }

    /// Both toggles render in the strip-editor row (edit mode, dashboard ON)
    /// and register clickable regions under their keys.
    #[test]
    fn header_toggles_draw_and_register_in_edit_mode() {
        let mut s = bt_shell("titles-draw");
        s.dash_edit = true;
        s.layout(1200.0, 800.0);
        let keys: Vec<u32> = s.hover_regions.iter().map(|r| r.4).collect();
        assert!(
            keys.contains(&CARD_TITLE_TOGGLE_KEY),
            "titles toggle must register"
        );
        assert!(
            keys.contains(&CARD_GLYPH_TOGGLE_KEY),
            "glyphs toggle must register"
        );
    }

    /// Toggles only exist while the dashboard is enabled — with the board
    /// off there is nothing to label.
    #[test]
    fn header_toggles_hidden_when_dashboard_off() {
        let mut s = bt_shell("dash-off");
        s.dash_enabled = false;
        s.dash_edit = true;
        s.layout(1200.0, 800.0);
        let keys: Vec<u32> = s.hover_regions.iter().map(|r| r.4).collect();
        assert!(
            !keys.contains(&CARD_TITLE_TOGGLE_KEY),
            "no titles toggle without the board"
        );
        assert!(
            !keys.contains(&CARD_GLYPH_TOGGLE_KEY),
            "no glyphs toggle without the board"
        );
    }

    /// Clicking "Card titles" flips the flag, persists it to the channel
    /// config, and the next frame drops every card's header title TEXT.
    #[test]
    fn titles_toggle_flips_persists_and_hides_titles() {
        let mut s = bt_shell("titles");
        assert!(s.card_show_title, "titles default ON");
        s.dash_edit = true;
        s.layout(1200.0, 800.0);
        let (rx, ry, _, _, _) = s
            .hover_regions
            .iter()
            .find(|r| r.4 == CARD_TITLE_TOGGLE_KEY)
            .copied()
            .expect("toggle registered");
        s.edit_press(rx + 1.0, ry + 1.0);
        assert!(!s.card_show_title, "click flips the toggle");
        let saved = std::fs::read_to_string(&s.per_state_config).unwrap_or_default();
        assert!(
            saved.contains("card_show_title = false"),
            "persisted to the channel config"
        );
        // the drawer must obey: no card title TEXT anywhere in the scene
        let v = s.layout(1200.0, 800.0);
        let has_bt_title = v
            .iter()
            .any(|c| matches!(c, Cmd::Text { text, .. } if text == "Bluetooth"));
        assert!(!has_bt_title, "header title hidden after toggling off");
        // and back on
        let (rx, ry, _, _, _) = s
            .hover_regions
            .iter()
            .find(|r| r.4 == CARD_TITLE_TOGGLE_KEY)
            .copied()
            .unwrap();
        s.edit_press(rx + 1.0, ry + 1.0);
        assert!(s.card_show_title, "second click restores");
        let v = s.layout(1200.0, 800.0);
        assert!(
            v.iter()
                .any(|c| matches!(c, Cmd::Text { text, .. } if text == "Bluetooth")),
            "header title drawn again"
        );
        let _ = std::fs::remove_file(&s.per_state_config);
    }

    /// Same flow for the glyph toggle: the header ICON disappears while the
    /// title TEXT stays.
    #[test]
    fn glyphs_toggle_hides_icon_keeps_title() {
        let mut s = bt_shell("glyphs");
        assert!(s.card_show_glyph, "glyphs default ON");
        s.dash_edit = true;
        s.layout(1200.0, 800.0);
        let (rx, ry, _, _, _) = s
            .hover_regions
            .iter()
            .find(|r| r.4 == CARD_GLYPH_TOGGLE_KEY)
            .copied()
            .unwrap();
        s.edit_press(rx + 1.0, ry + 1.0);
        assert!(!s.card_show_glyph, "click flips the toggle");
        let saved = std::fs::read_to_string(&s.per_state_config).unwrap_or_default();
        assert!(saved.contains("card_show_glyph = false"), "persisted");
        let v = s.layout(1200.0, 800.0);
        let glyph = crate::icons::ICON_BLUETOOTH;
        let has_icon = v
            .iter()
            .any(|c| matches!(c, Cmd::Text { text, icon, .. } if *icon && text == glyph));
        assert!(!has_icon, "header icon hidden after toggling off");
        assert!(
            v.iter()
                .any(|c| matches!(c, Cmd::Text { text, .. } if text == "Bluetooth")),
            "title text STAYS"
        );
        let _ = std::fs::remove_file(&s.per_state_config);
    }
