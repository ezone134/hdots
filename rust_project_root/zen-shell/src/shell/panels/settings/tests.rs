    use super::*;

    fn shell_with_settings_scene() -> Shell {
        let cfg: crate::config::Config =
            toml::from_str(crate::config::DEFAULT_SHELL_TOML).expect("default config parses");
        let mut s = Shell::new(cfg);
        let scene = crate::scene::SurfaceScene {
            name: "settings".to_string(),
            stores: vec![crate::scene::StoreDecl {
                name: "nav".to_string(),
                props: crate::scene::PropSet::from_pairs([(
                    "tab".to_string(),
                    crate::scene::PropValue::Num(0.0),
                )]),
            }],
            items: Vec::new(),
        };
        s.seed_scene_stores_for(&scene);
        s
    }

    #[test]
    fn the_live_settings_nav_rows_write_the_store_rather_than_a_key_handler() {
        // The migration's whole point: the sidebar SELECTS the pane through the
        // scene. If a row's action went back to `None` (or a Rust handler crept
        // back onto key 200), the scene would stop being the writer — and
        // because a Rust handler wins the key, the symptom would be a sidebar
        // that still works by accident, or one that does nothing at all.
        let cfg: crate::config::Config =
            toml::from_str(crate::config::DEFAULT_SHELL_TOML).expect("default config parses");
        let s = Shell::new(cfg);
        let vals = s.scene_values_for("settings");
        let Some(rows) = vals.rows("settings_nav") else {
            return; // no live shell.ron in this env
        };
        assert_eq!(rows.len(), SettingsTab::ALL.len(), "one row per tab");
        for (row, tab) in rows.iter().zip(SettingsTab::ALL.iter()) {
            match row.action.as_ref() {
                Some(crate::scene::SceneAction::Set { name, value }) => {
                    assert_eq!(name, "nav.tab", "row for {tab:?} writes the store");
                    assert_eq!(
                        *value,
                        crate::scene::PropValue::Num(tab.index()),
                        "row for {tab:?} writes its own index"
                    );
                }
                other => panic!("row for {tab:?} must carry a Set, got {other:?}"),
            }
        }
    }

    #[test]
    fn the_live_settings_surface_declares_the_tab_it_writes() {
        // The other half: a row can carry a perfect `Set` and still do nothing
        // if the surface never declares `nav.tab` — the write is refused, and a
        // refused write looks like a click that hit nothing.
        let text = crate::shell_scene::SHELL_SCENE_RON;
        let mut scene: crate::scene::ShellScene =
            crate::scene::parse_scene_ron(&text).unwrap_or_else(|_| crate::scene::ShellScene { surfaces: vec![], dead_props: Default::default(), components: std::collections::HashMap::new() });
        scene.expand();
        let surface = scene
            .surface("settings")
            .expect("the settings surface is declared");
        let nav = surface
            .stores
            .iter()
            .find(|s| s.name == "nav")
            .expect("the settings surface declares a `nav` store");
        assert!(
            nav.props.get("tab").is_some(),
            "`nav` must declare `tab` — a row writes it on every click"
        );
    }

    #[test]
    fn the_settings_frame_widget_tree_matches_the_retired_drawer_bytes() {
        // The frame is drawn through the widget system now; the hand-drawn
        // chrome survives as a cfg(test) reference. Proof of the handover:
        // BOTH painters, drawn under matched shell states (tab selection,
        // sidebar-row hover, close-chip hover), emit identical Vec<Cmd>
        // bytes \u2014 same ui::text helpers, same geometry, same key regions.
        let cfg: crate::config::Config =
            toml::from_str(crate::config::DEFAULT_SHELL_TOML).expect("default config parses");
        let mut s = Shell::new(cfg);
        s.set_settings_tab(SettingsTab::Appearance);
        let w = 920.0;
        let h = 640.0;
        let pal = Pal {
            fg: s.sv_fg,
            bg: s.sv_bg,
            acc: s.sv_acc,
            sfg: s.sv_sfg,
        };
        for hover in [0u32, NAV_SYSTEM, 1] {
            s.hover_key = hover;
            let mut native = Vec::new();
            s.draw_settings_chrome_native(&mut native, w, h, &pal);
            let mut wired = Vec::new();
            s.draw_settings_chrome_widgets(&mut wired, w, h, &pal);
            for (idx, (a, b)) in native.iter().zip(wired.iter()).enumerate() {
                assert_eq!(a, b, "cmd {idx} differs (hover {hover})\nnative: {a:?}\nwired:  {b:?}");
            }
            assert_eq!(native.len(), wired.len(), "stream lengths differ (hover {hover})");
        }
    }

    #[test]
    fn the_network_body_widget_tree_matches_the_declarative_stack_bytes() {
        // First tab body port (WIDGETS.md stage 6): the NETWORK & INTERNET
        // stack draws through the widget tree (`settings_body::network`);
        // proof of the handover: BOTH painters, drawn under matched states
        // (radios on/off, row hover, switch hover), emit identical bytes and
        // register the same hit regions.
        let cfg: crate::config::Config =
            toml::from_str(crate::config::DEFAULT_SHELL_TOML).expect("default config parses");
        let w = 920.0;
        let h = 640.0;
        let mut hits_seen = std::collections::HashSet::new();
        for wifi in [true, false] {
            for hover in [0u32, 2, 12, 3, 13] {
                let mut s = Shell::new(cfg.clone());
                s.wifi_on = wifi;
                s.bt_on = wifi;
                s.hover_key = hover;
                let pal = Pal {
                    fg: s.sv_fg,
                    bg: s.sv_bg,
                    acc: s.sv_acc,
                    sfg: s.sv_sfg,
                };
                // engine reference: the network tab stack as the AUTHORITATIVE
                // declarative text spells it — the embedded scene fixture
                // (crate::shell_scene), whose settings section was held
                // byte-for-byte to the drawer the const was generated from.
                // The const dropped the stack when the widgets took over; the
                // RON fixture keeps the reference alive.
                let text = crate::shell_scene::SHELL_SCENE_RON;
                let mut scene: crate::scene::ShellScene =
                    crate::scene::parse_scene_ron(&text).expect("the scene fixture parses");
                scene.expand();
                let surface = scene
                    .surface("settings")
                    .expect("the settings surface is declared in the fixture");
                let stack = surface
                    .items
                    .iter()
                    .find(|it| {
                        matches!(
                            it,
                            crate::scene::SceneItem::Stack { visible, .. }
                                if visible.as_deref() == Some("tab_network")
                        )
                    })
                    .cloned()
                    .expect("the network tab stack is declared in the fixture");
                let mut own = s.scene_values_for("settings");
                s.scene_stores.merge_into("settings", &mut own);
                let card = crate::scene::CardScene { items: vec![stack] };
                let mut engine = Vec::new();
                let (hit_list, inks) = card.draw(
                    &mut engine, 0.0, 0.0, w, h, s.scale, &pal, &own, hover, false, false,
                );
                assert!(inks.is_empty(), "no Ink in the network stack");
                // widget tree: the same rows state, painted through widgets
                let mut wired = Vec::new();
                {
                    let rows = s.network_body_rows();
                    let tree =
                        crate::settings_body::network(w, h, &pal, s.scale.0, rows);
                    let tree = tree.layout(
                        crate::widgets::Rect { x: 0.0, y: 0.0, w, h },
                        &mut crate::widgets::NoMeasurer,
                    );
                    let mut sink = ShellSink { shell: &mut s };
                    tree.render(&mut wired, &mut sink, &pal);
                }
                let regions: Vec<(f32, f32, f32, f32, u32)> = s
                    .hover_regions
                    .iter()
                    .map(|&(x, y, ww, hh, k)| (x, y, ww, hh, k))
                    .collect();
                assert_eq!(engine.len(), wired.len(), "stream lengths (hover {hover})");
                for (idx, (a, b)) in engine.iter().zip(wired.iter()).enumerate() {
                    assert_eq!(
                        a, b,
                        "cmd {idx} differs (hover {hover})\nengine: {a:?}\nwired:  {b:?}"
                    );
                }
                for (want, got) in hit_list.iter().zip(regions.iter()) {
                    assert_eq!(
                        (want.x, want.y, want.w, want.h, want.key),
                        *got,
                        "hit region differs (hover {hover})"
                    );
                }
                hits_seen.insert(hover);
            }
        }
        assert_eq!(hits_seen.len(), 5, "every hover state exercised");
    }

    #[test]
    fn a_claimed_key_never_reaches_the_rust_fallback_handler() {
        // The ordering the migration depends on, asserted on the source rather
        // than on behaviour: in `Shell::click`'s match, the
        // `scene_click_actions.contains_key(&k)` arm must come BEFORE the
        // `200 => …` settings arms. Rust match arms are tried in order, so if
        // that ever moves below them the scene's `Set` would never run and the
        // sidebar would keep working off the fallback — which looks correct and
        // is exactly the "declarative action quietly stops being declarative"
        // failure. A behavioural test cannot catch it, because both paths
        // produce the same tab.
        let src = include_str!("../../input.rs");
        let scene_arm = src
            .find("k if self.scene_click_actions.contains_key(&k) =>")
            .expect("the scene-action arm exists");
        let first_settings_arm = src
            .find("                200 => {")
            .expect("the settings fallback arms exist");
        assert!(
            scene_arm < first_settings_arm,
            "the scene-action arm must precede the settings fallback arms \
             (scene arm at byte {scene_arm}, settings arm at {first_settings_arm})"
        );
    }

    #[test]
    fn the_settings_tab_is_the_stores_value_once_a_scene_declares_it() {
        let mut s = shell_with_settings_scene();
        assert_eq!(
            s.settings_tab(),
            SettingsTab::Network,
            "the declared default"
        );
        s.set_settings_tab(SettingsTab::Pill);
        assert_eq!(s.settings_tab(), SettingsTab::Pill);
        // The two surfaces' views agree — the write went to the store the
        // scene declared, not to a private field.
        assert_eq!(
            s.scene_values_for("settings").scalar("nav.tab"),
            Some(SettingsTab::Pill.index())
        );
    }

    #[test]
    fn the_settings_tab_falls_back_when_no_scene_declares_one() {
        // No scene: the Rust-only layout must still switch panes, so the
        // fallback field is not dead weight.
        let cfg: crate::config::Config =
            toml::from_str(crate::config::DEFAULT_SHELL_TOML).expect("default config parses");
        let s = Shell::new(cfg);
        assert_eq!(s.settings_tab(), SettingsTab::Network);
    }

    #[test]
    fn an_out_of_range_tab_index_lands_on_the_default() {
        // `nav.tab` is writable by a `.ron`, so the index is untrusted input.
        // It must not panic — a bad index in a scene must cost the scene its
        // selection, not the frame.
        assert_eq!(SettingsTab::from_index(99.0), SettingsTab::Network);
        assert_eq!(SettingsTab::from_index(-1.0), SettingsTab::Network);
        assert_eq!(SettingsTab::from_index(f32::NAN), SettingsTab::Network);
        for (i, tab) in SettingsTab::ALL.iter().enumerate() {
            assert_eq!(SettingsTab::from_index(i as f32), *tab, "index {i}");
            assert_eq!(tab.index(), i as f32, "round trip for {tab:?}");
        }
    }
