//! Settings app — two-pane: left nav sidebar + right content pane.
//!
//! Every permanent setting here is written back to the channel master
//! `$states/shell_{n,d,l}` by `Shell::save_config` (pill layout, toggles,
//! geometry, dashboard zoom, accent). The glanceable rows (Wi-Fi / BT / CPU /
//! RAM / DISK) are live backend state, not persisted.

use super::*;

/// Sidebar width — mirrors `SETTINGS_SIDEBAR` in shell/mod.rs.
const SIDE: f32 = SETTINGS_SIDEBAR;
/// nav keys for the sidebar categories (above the content-row key space)
pub(crate) const NAV_NETWORK: u32 = 200;
pub(crate) const NAV_SOUND: u32 = 201;
pub(crate) const NAV_APPEARANCE: u32 = 202;
pub(crate) const NAV_PILL: u32 = 203;
pub(crate) const NAV_SYSTEM: u32 = 204;
pub(crate) const NAV_MISC: u32 = 219;

/// The `nav.tab` store prop the `.ron` settings sidebar writes.
const NAV_TAB_PROP: &str = "nav.tab";

impl Shell {
    /// The active Settings category.
    ///
    /// The store is authoritative when the `settings` surface declares
    /// `nav.tab`; the Rust field is only the fallback for the no-scene case.
    /// Reading the field directly is the bug this accessor exists to prevent:
    /// a scene-driven sidebar click writes the store, so the field would keep
    /// reporting the old tab and the click would look like it did nothing.
    pub(crate) fn settings_tab(&self) -> SettingsTab {
        match self.scene_stores.tab_index("settings", NAV_TAB_PROP) {
            Some(n) => SettingsTab::from_index(n),
            None => self.settings_tab_fallback,
        }
    }

    /// Select a Settings category.
    ///
    /// Writes BOTH the store and the fallback field, so the two can never
    /// disagree — whichever one the next reader consults, it agrees. The store
    /// write is refused when the surface declares no `nav.tab`, which is
    /// exactly the case the fallback field exists for.
    ///
    /// Called from the Rust key handlers BELOW the scene-action arm in
    /// `click()`, so they only run when no `settings_nav` row claimed the key.
    pub(crate) fn set_settings_tab(&mut self, tab: SettingsTab) {
        self.settings_tab_fallback = tab;
        self.scene_stores.write(
            "settings",
            NAV_TAB_PROP,
            crate::scene::PropValue::Num(tab.index()),
        );
    }
}

impl Shell {
    /// Draw the two-pane Settings scene. A `settings` surface declared in
    /// `shell.ron` owns the whole scene; without one the built-in Rust layout
    /// below runs unchanged.
    pub(crate) fn layout_settings(&mut self, v: &mut Vec<Cmd>, w: f32, h: f32, pal: &Pal) {
        // the pane we are drawing into, for the publisher's one measure of it
        // (the custom-accent popover's column count) — see `settings_pane_w`
        self.settings_pane_w = w;
// the custom-accent list is read from disk ($states2/custom_acc.json, written
        // by the theme pipeline) and the declared Appearance section publishes
        // its circle grid + the trigger's selected swatch FROM that list, so
        // the refresh has to happen in this `&mut self` window — the publisher
        // itself takes `&self`. Gated on the tab that shows it, which is what
        // the painter's own per-frame `load_custom_accs()` used to cost.
        if self.settings_tab() == SettingsTab::Appearance {
            self.load_custom_accs();
        }
        let vals = self.scene_values_for("settings");
        if self.draw_shell_surface("settings", v, w, h, pal, &vals) {
            return;
        }
        // ── top header strip (spans both panes) ──
        ui::text(v, 22.0, 14.0, "Settings", 17.0, pal.fg, false);
        if self.hover_key == 1 {
            v.push(Cmd::Rect {
                x: w - 42.0,
                y: 6.0,
                w: 30.0,
                h: 26.0,
                r: 6.0,
                color: ui::hover_hl(pal),
            });
        }
        ui::text_r(
            v,
            w - 26.0,
            13.0,
            "✕",
            12.0,
            if self.hover_key == 1 {
                pal.fg
            } else {
                ui::fg2(pal)
            },
            false,
        );
        self.region(w - 42.0, 4.0, 36.0, 30.0, 1);

        // ── left nav sidebar ──
        // subtle divider background separating sidebar from content
        v.push(Cmd::Rect {
            x: 0.0,
            y: 44.0,
            w: SIDE,
            h: h - 44.0,
            r: 0.0,
            color: (pal.bg & 0xffffff00) | 0x08,
        });
        v.push(Cmd::Rect {
            x: SIDE - 1.0,
            y: 44.0,
            w: 1.0,
            h: h - 44.0,
            r: 0.0,
            color: ui::hairline(pal),
        });

        let nav: [(u32, &str, &str, SettingsTab); 6] = [
            (NAV_NETWORK, ICON_WIFI, "Network", SettingsTab::Network),
            (
                NAV_SOUND,
                ICON_VOLUME,
                "Sound & Display",
                SettingsTab::Sound,
            ),
            (
                NAV_APPEARANCE,
                ICON_PALETTE,
                "Appearance",
                SettingsTab::Appearance,
            ),
            (NAV_PILL, ICON_PALETTE, "Pill", SettingsTab::Pill),
            (NAV_SYSTEM, ICON_MONITOR, "System", SettingsTab::System),
            (NAV_MISC, ICON_SETTINGS, "Misc", SettingsTab::Misc),
        ];
        for (i, &(key, glyph, label, tab)) in nav.iter().enumerate() {
            let y = 56.0 + i as f32 * 46.0;
            let sel = self.settings_tab() == tab;
            let hov = self.hover_key == key;
            if sel {
                v.push(Cmd::Rect {
                    x: 6.0,
                    y: y - 2.0,
                    w: SIDE - 12.0,
                    h: 40.0,
                    r: 8.0,
                    color: ui::hover_hl(pal),
                });
            } else if hov {
                v.push(Cmd::Rect {
                    x: 6.0,
                    y: y - 2.0,
                    w: SIDE - 12.0,
                    h: 40.0,
                    r: 8.0,
                    color: ui::hover(pal),
                });
            }
            let tc = if sel {
                pal.acc
            } else if hov {
                pal.fg
            } else {
                ui::fg2(pal)
            };
            ui::text(v, 16.0, y + 13.0, glyph, 14.0, tc, true);
            ui::text(
                v,
                42.0,
                y + 13.0,
                label,
                12.5,
                if sel || hov { pal.fg } else { ui::fg2(pal) },
                false,
            );
            if sel {
                // 2px accent rail on the sidebar's left edge
                v.push(Cmd::Rect {
                    x: 0.0,
                    y: y + 8.0,
                    w: 2.0,
                    h: 20.0,
                    r: 1.0,
                    color: pal.acc,
                });
            }
            self.region(2.0, y - 2.0, SIDE - 4.0, 40.0, key);
        }

        // ── right content pane ──
        // Only reached when `shell.ron` declares no `settings` surface (the
        // four arms below) — the Misc, Network, System and Sound tabs are
        // DECLARED, so they have no release arms here and the surface owns
        // them in every build. Consequence, same as the notif conversion: a
        // missing or broken `settings` surface paints no declared body rather
        // than falling back to a second hand-drawn painter that could disagree
        // with the scene.
        // left edge of the content pane — only the cfg(test) parity oracles
        // below take it; a release build declares all six sections instead
        #[cfg(test)]
        let cx = SIDE;
        match self.settings_tab() {
            #[cfg(test)]
            SettingsTab::Network => self.layout_settings_network(v, w, cx, pal),
            #[cfg(not(test))]
            SettingsTab::Network => {}
            SettingsTab::Appearance => {
                #[cfg(test)]
                self.layout_settings_appearance(v, w, cx, pal);
                #[cfg(not(test))]
                {}
            }
            #[cfg(test)]
            SettingsTab::Pill => self.layout_settings_pill(v, w, cx, pal),
            #[cfg(not(test))]
            SettingsTab::Pill => {}
            #[cfg(test)]
            SettingsTab::System => self.layout_settings_system(v, w, cx, h, pal),
            #[cfg(not(test))]
            SettingsTab::System => {}
            #[cfg(test)]
            SettingsTab::Misc => self.layout_settings_misc(v, w, cx, pal),
            #[cfg(not(test))]
            SettingsTab::Misc => {}
            #[cfg(test)]
            SettingsTab::Sound => self.layout_settings_sound(v, w, cx, pal),
            #[cfg(not(test))]
            SettingsTab::Sound => {}
        }
    }

    // ── Network & Internet ──
    // DECLARED in shell.ron (the `tab_network` section of `scontent`), so this
    // painter is the parity ORACLE and nothing else — it is not compiled into
    // a release binary and the dispatcher has no arm for it. Kept verbatim
    // because `the_settings_network_scene_matches_its_own_drawer` compares the
    // declared section's whole `Cmd` stream against it; deleting this would
    // delete the evidence that the conversion was lossless.
    #[cfg(test)]
    pub(crate) fn layout_settings_network(&mut self, v: &mut Vec<Cmd>, w: f32, cx: f32, pal: &Pal) {
        ui::text(
            v,
            cx + 8.0,
            50.0,
            "NETWORK & INTERNET",
            9.5,
            ui::fg3(pal),
            false,
        );
        let ssid = if self.wifi_on && !self.ssid.is_empty() {
            self.ssid.clone()
        } else if self.wifi_on {
            "Not connected".to_string()
        } else {
            "Off".to_string()
        };
        Self::settings_row(
            v,
            pal,
            cx + 8.0,
            64.0,
            w - cx - 16.0,
            ICON_WIFI,
            "Wi-Fi",
            Some(&ssid),
            self.wifi_on,
            self.hover_key == 2 || self.hover_key == 12,
        );
        self.region(cx + 8.0, 64.0, w - cx - 16.0 - 58.0, 48.0, 2);
        self.region(w - 74.0, 76.0, 42.0, 24.0, 12);

        let bt = if self.bt_on {
            self.bt_devices
                .iter()
                .find(|d| d.1)
                .map(|d| d.0.clone())
                .unwrap_or_else(|| "On".to_string())
        } else {
            "Off".to_string()
        };
        Self::settings_row(
            v,
            pal,
            cx + 8.0,
            118.0,
            w - cx - 16.0,
            ICON_BLUETOOTH,
            "Bluetooth",
            Some(&bt),
            self.bt_on,
            self.hover_key == 3 || self.hover_key == 13,
        );
        self.region(cx + 8.0, 118.0, w - cx - 16.0 - 58.0, 48.0, 3);
        self.region(w - 74.0, 130.0, 42.0, 24.0, 13);
    }

    // ── Sound & Display ──
    // DECLARED in shell.ron (the `tab_sound` section of `scontent`), so this
    // painter is the parity ORACLE and nothing else — it is not compiled into
    // a release binary and the dispatcher has no arm for it. Kept verbatim
    // because `the_settings_sound_scene_matches_its_own_drawer` compares the
    // declared section's whole `Cmd` stream against it; deleting this would
    // delete the evidence that the conversion was lossless.
    #[cfg(test)]
    pub(crate) fn layout_settings_sound(&mut self, v: &mut Vec<Cmd>, w: f32, cx: f32, pal: &Pal) {
        ui::text(
            v,
            cx + 8.0,
            50.0,
            "SOUND & DISPLAY",
            9.5,
            ui::fg3(pal),
            false,
        );
        // brightness
        v.push(Cmd::Rect {
            x: cx + 24.0,
            y: 64.0,
            w: 30.0,
            h: 30.0,
            r: 9.0,
            color: mix(ui::hover(pal), pal.acc, 0.20),
        });
        ui::text(v, cx + 31.0, 71.0, ICON_BRIGHTNESS, 14.0, pal.acc, true);
        ui::text(v, cx + 64.0, 72.0, "Brightness", 13.0, pal.fg, false);
        ui::slider(
            v,
            cx + 64.0,
            90.0,
            w - cx - 96.0,
            10.0,
            self.brightness,
            self.hover_key == 5,
            pal,
        );
        self.region(cx + 10.0, 55.0, w - cx - 20.0, 44.0, 5);
        // volume
        v.push(Cmd::Rect {
            x: cx + 24.0,
            y: 108.0,
            w: 30.0,
            h: 30.0,
            r: 9.0,
            color: mix(ui::hover(pal), pal.acc, 0.20),
        });
        ui::text(v, cx + 31.0, 115.0, ICON_VOLUME, 14.0, pal.acc, true);
        ui::text(v, cx + 64.0, 116.0, "Volume", 13.0, pal.fg, false);
        ui::slider(
            v,
            cx + 64.0,
            134.0,
            w - cx - 96.0,
            10.0,
            self.volume,
            self.hover_key == 4,
            pal,
        );
        self.region(cx + 10.0, 99.0, w - cx - 20.0, 44.0, 4);
        v.push(Cmd::Rect {
            x: cx + 24.0,
            y: 152.0,
            w: 30.0,
            h: 30.0,
            r: 9.0,
            color: mix(ui::hover(pal), pal.acc, 0.20),
        });
        ui::text(v, cx + 31.0, 159.0, ICON_SQUARE, 14.0, pal.acc, true);
        ui::text(v, cx + 64.0, 160.0, "Max volume", 13.0, pal.fg, false);
        // toggle chip (on by default)
        let tog_x = w - cx - 118.0;
        let tog_on = self.allow_over_100;
        v.push(Cmd::Rect {
            x: tog_x,
            y: 154.0,
            w: 44.0,
            h: 24.0,
            r: 12.0,
            color: if tog_on {
                ui::acc_tint(pal)
            } else {
                mix(ui::hover(pal), pal.fg, 0.10)
            },
        });
        ui::text_c(
            v,
            tog_x + 22.0,
            158.0,
            if tog_on { ICON_SUNNY } else { ICON_TOGGLE_OFF },
            10.0,
            if tog_on { pal.fg } else { pal.fg },
            true,
        );
        self.region(tog_x - 4.0, 150.0, 52.0, 32.0, 14);
        // max volume slider (0..200 → 0..1 of the track)
        ui::slider(
            v,
            cx + 64.0,
            178.0,
            w - cx - 96.0,
            10.0,
            (self.max_vol as f32 / 200.0).clamp(0.0, 1.0),
            self.hover_key == 15,
            pal,
        );
        self.region(cx + 10.0, 153.0, w - cx - 20.0, 34.0, 15);
        // ── Balance slider + reset button ──
        let bal_y = 200.0;
        let bw = w - cx - 96.0;
        let lr = self.vol_left + self.vol_right;
        let bal = if lr > 0.001 {
            (self.vol_right / lr).clamp(0.0, 1.0)
        } else {
            0.5
        };
        let tx = cx + 64.0;
        let cy = bal_y + 3.0;
        let hov59 = self.hover_key == 59;
        ui::text(v, cx + 68.0, bal_y - 3.0, ICON_STAR, 9.0, pal.fg, true);
        ui::fader_bal(v, tx, cy, bw, bal, pal.acc, hov59, pal);
        self.balance_rect = (tx, cy, bw);
        ui::text_r(
            v,
            cx + 64.0 + bw - 4.0,
            bal_y - 3.0,
            ICON_VISIBILITY,
            9.0,
            pal.fg,
            true,
        );
        ui::text(
            v,
            cx + 64.0,
            bal_y + 10.0,
            "Balance",
            9.5,
            ui::fg3(pal),
            false,
        );
        // reset button at right of balance row
        let rbx = cx + 64.0 + bw + 8.0;
        let rb_hov = self.hover_key == 58;
        v.push(Cmd::Rect {
            x: rbx,
            y: bal_y - 2.0,
            w: 52.0,
            h: 22.0,
            r: 7.0,
            color: if rb_hov {
                ui::hover_hl(pal)
            } else {
                mix(ui::hover(pal), pal.fg, 0.10)
            },
        });
        ui::text_c(
            v,
            rbx + 26.0,
            bal_y + 2.0,
            format!("{} Reset", ICON_SLIDER),
            9.0,
            if rb_hov { pal.acc } else { pal.fg },
            false,
        );
        self.region(rbx - 2.0, bal_y - 4.0, 56.0, 26.0, 58);
        self.region(cx + 62.0, bal_y - 2.0, bw + 4.0, 24.0, 59);
    }

    // ── Appearance ──
    // DECLARED in shell.ron (the `tab_appearance` section of `scontent`), so
    // this painter is the parity ORACLE and nothing else — it is not compiled
    // into a release binary and the dispatcher has no arm for it. Kept
    // verbatim because `the_settings_appearance_scene_matches_its_own_drawer`
    // compares the declared section's whole `Cmd` stream against it; deleting
    // this would delete the evidence that the conversion was lossless.
    #[cfg(test)]
    pub(crate) fn layout_settings_appearance(
        &mut self,
        v: &mut Vec<Cmd>,
        w: f32,
        cx: f32,
        pal: &Pal,
    ) {
        let roww = w - cx - 16.0;

        // ── fixed header (never scrolls) ──
        ui::text(v, cx + 8.0, 50.0, "APPEARANCE", 9.5, ui::fg3(pal), false);
        ui::text(v, cx + 8.0, 70.0, "Accent color", 13.0, pal.fg, false);
        // current accent is the live `$acc` from $states2/shell_vars
        let cur = self.sv_acc;
        v.push(Cmd::Rect {
            x: w - 44.0,
            y: 70.0,
            w: 14.0,
            h: 14.0,
            r: 7.0,
            color: cur,
        });
        ui::text_r(
            v,
            w - 52.0,
            71.0,
            format!("#{:06x}", (cur >> 8) & 0xffffff),
            9.5,
            ui::fg3(pal),
            false,
        );

        // ── scrollable content ──
        let so = -self.appearance_scroll_px();
        let mut y = 118.0 + so;

        // ---- ACCENT TOGGLES (writes the per-channel custom_acc_<flag>_<ch>) ----
        ui::text(v, cx + 8.0, y, "ACCENT TOGGLES", 9.5, ui::fg3(pal), false);
        y += 22.0;
        let toggles = [
            (
                206u32,
                "start_icon",
                ICON_BOLT,
                "Start icon",
                self.acc_flag_start_icon,
            ),
            (
                207u32,
                "app_border",
                ICON_SQUARE,
                "App border",
                self.acc_flag_app_border,
            ),
            (
                208u32,
                "hyprland",
                ICON_WINDOWS,
                "Hyprland border",
                self.acc_flag_hyprland,
            ),
            (209u32, "gtk", ICON_MONITOR, "GTK app bg", self.acc_flag_gtk),
            (
                210u32,
                "normal_app",
                ICON_APP,
                "Normal app bg",
                self.acc_flag_normal_app,
            ),
        ];
        for (key, _flag, glyph, label, on) in toggles {
            Self::settings_row(
                v,
                pal,
                cx + 8.0,
                y,
                roww,
                glyph,
                label,
                None,
                on,
                self.hover_key == key,
            );
            self.region(cx + 8.0, y, roww, 48.0, key);
            y += 52.0;
        }
        // start-icon tone slider (0..900)
        {
            let key = 211u32;
            let hov = self.hover_key == key;
            let icbg = mix(ui::hover(pal), pal.acc, 0.20);
            v.push(Cmd::Rect {
                x: cx + 24.0,
                y,
                w: 30.0,
                h: 30.0,
                r: 9.0,
                color: icbg,
            });
            ui::text(
                v,
                cx + 31.0,
                y + 7.0,
                ICON_SPARKLE.to_string(),
                14.0,
                pal.acc,
                true,
            );
            ui::text(
                v,
                cx + 64.0,
                y + 2.0,
                "Start icon tone",
                13.0,
                pal.fg,
                false,
            );
            ui::text(
                v,
                cx + 64.0,
                y + 20.0,
                format!("{}", self.start_icon_tone),
                10.5,
                ui::fg3(pal),
                false,
            );
            ui::slider(
                v,
                cx + 64.0,
                y + 25.0,
                w - cx - 96.0,
                10.0,
                self.start_icon_tone as f32 / 900.0,
                hov,
                pal,
            );
            self.region(cx + 10.0, y, w - cx - 20.0, 40.0, key);
            y += 52.0;
        }

        // ---- ALPHA (lowercase hex in $states/<flag>_<ch>) ----
        ui::text(v, cx + 8.0, y, "ALPHA", 9.5, ui::fg3(pal), false);
        y += 22.0;
        let alphas = [
            (
                212u32,
                "bg_alpha",
                ICON_PALETTE,
                "Background",
                self.bg_alpha,
            ),
            (213u32, "acc_alpha", ICON_DROP, "Accent", self.acc_alpha),
            (
                214u32,
                "scrim_alpha",
                ICON_ACTIVE,
                "Scrim",
                self.scrim_alpha,
            ),
            (
                215u32,
                "border_alpha",
                ICON_SQUARE,
                "Border",
                self.border_alpha,
            ),
        ];
        for (key, _flag, glyph, label, val) in alphas {
            let hov = self.hover_key == key;
            let icbg = mix(ui::hover(pal), pal.acc, 0.20);
            v.push(Cmd::Rect {
                x: cx + 24.0,
                y,
                w: 30.0,
                h: 30.0,
                r: 9.0,
                color: icbg,
            });
            ui::text(
                v,
                cx + 31.0,
                y + 7.0,
                glyph.to_string(),
                14.0,
                pal.acc,
                true,
            );
            ui::text(v, cx + 64.0, y + 2.0, label, 13.0, pal.fg, false);
            ui::text(
                v,
                cx + 64.0,
                y + 20.0,
                format!("{:02x}", val),
                10.5,
                ui::fg3(pal),
                false,
            );
            ui::slider(
                v,
                cx + 64.0,
                y + 25.0,
                w - cx - 96.0,
                10.0,
                val as f32 / 255.0,
                hov,
                pal,
            );
            self.region(cx + 10.0, y, w - cx - 20.0, 40.0, key);
            y += 44.0;
        }
        // Accent scrim toggle ($states/acc_scrim_<ch>)
        {
            let key = 216u32;
            Self::settings_row(
                v,
                pal,
                cx + 8.0,
                y,
                roww,
                ICON_PALETTE,
                "Accent scrim",
                Some("overlay on wallpaper"),
                self.acc_scrim,
                self.hover_key == key,
            );
            self.region(cx + 8.0, y, roww, 48.0, key);
            y += 52.0;
        }

        // ---- ACCENT SOURCE (writes $states/acc_source_<ch> via scheme_main) ----
        ui::text(v, cx + 8.0, y, "ACCENT SOURCE", 9.5, ui::fg3(pal), false);
        y += 22.0;
        let src = self.current_acc_source();
        let src_rows: [(u32, &str, &str, &str); 3] = [
            (60, ICON_BAG, "From wallpaper", "w"),
            (61, ICON_PALETTE, "Scheme default", "0"),
            (223, ICON_DROP, "Custom accent", "c"),
        ];
        for (key, glyph, label, srcv) in src_rows.iter().copied() {
            let sel = src == srcv;
            let hov = self.hover_key == key;
            // selected row's foreground is $sfg (text on the acc-tinted surface)
            let sfmt = if sel { pal.sfg } else { pal.fg };
            v.push(Cmd::Rect {
                x: cx + 8.0,
                y,
                w: roww,
                h: 38.0,
                r: 10.0,
                color: if sel {
                    mix(pal.acc, pal.bg, 0.35)
                } else if hov {
                    ui::hover(pal)
                } else {
                    (pal.bg & 0xffffff00) | 0x12
                },
            });
            ui::text(
                v,
                cx + 24.0,
                y + 11.0,
                glyph.to_string(),
                13.0,
                if sel { pal.sfg } else { pal.acc },
                true,
            );
            ui::text(v, cx + 50.0, y + 11.0, label, 12.5, sfmt, false);
            if sel {
                v.push(Cmd::Rect {
                    x: w - 34.0,
                    y: y + 13.0,
                    w: 10.0,
                    h: 10.0,
                    r: 5.0,
                    color: pal.sfg,
                });
            }
            self.region(cx + 8.0, y, roww, 38.0, key);
            y += 50.0;
        }
        // "Pick from screen" — launches hyprpicker (Esc cancels/discards)
        {
            let key = 205u32;
            let hov = self.hover_key == key;
            v.push(Cmd::Rect {
                x: cx + 8.0,
                y,
                w: roww,
                h: 38.0,
                r: 10.0,
                color: if hov {
                    mix(pal.acc, pal.bg, 0.25)
                } else {
                    (pal.bg & 0xffffff00) | 0x12
                },
            });
            ui::text(
                v,
                cx + 24.0,
                y + 11.0,
                ICON_SCHEME.to_string(),
                13.0,
                pal.acc,
                true,
            );
            ui::text(
                v,
                cx + 50.0,
                y + 11.0,
                "Pick from screen",
                12.5,
                if hov { pal.sfg } else { pal.fg },
                false,
            );
            self.region(cx + 8.0, y, roww, 38.0, key);
            y += 50.0;
        }
        // "Pick from color wheel" — runs the gcp color-chooser GUI (via
        // `custom_color_main`); the picked hex becomes the custom accent
        // (custom_acc_<ch> + source c). Always this-state; no `all` flag.
        {
            let key = 230u32;
            let hov = self.hover_key == key;
            v.push(Cmd::Rect {
                x: cx + 8.0,
                y,
                w: roww,
                h: 38.0,
                r: 10.0,
                color: if hov {
                    mix(pal.acc, pal.bg, 0.25)
                } else {
                    (pal.bg & 0xffffff00) | 0x12
                },
            });
            ui::text(
                v,
                cx + 24.0,
                y + 11.0,
                ICON_DROP.to_string(),
                13.0,
                pal.acc,
                true,
            );
            ui::text(
                v,
                cx + 50.0,
                y + 11.0,
                "Pick from color wheel",
                12.5,
                if hov { pal.sfg } else { pal.fg },
                false,
            );
            self.region(cx + 8.0, y, roww, 38.0, key);
            y += 50.0;
        }

        // "Saved Colors" (key 232) — expands the `$states/sca` list below. It
        // is an accent-source row in name only: it selects nothing (a saved
        // colour applies from its own row), it only opens the list, so it
        // wears the source rows' selected tint while open but NOT their dot.
        {
            let key = crate::shell::SAVED_ROW_KEY;
            let open = self.saved_drop_open;
            let hov = self.hover_key == key;
            v.push(Cmd::Rect {
                x: cx + 8.0,
                y,
                w: roww,
                h: 38.0,
                r: 10.0,
                color: if open {
                    mix(pal.acc, pal.bg, 0.35)
                } else if hov {
                    ui::hover(pal)
                } else {
                    (pal.bg & 0xffffff00) | 0x12
                },
            });
            ui::text(
                v,
                cx + 24.0,
                y + 11.0,
                ICON_SAVE.to_string(),
                13.0,
                if open { pal.sfg } else { pal.acc },
                true,
            );
            ui::text(
                v,
                cx + 50.0,
                y + 11.0,
                "Saved Colors",
                12.5,
                if open { pal.sfg } else { pal.fg },
                false,
            );
            self.region(cx + 8.0, y, roww, 38.0, key);
            y += 50.0;
        }

        // ---- the SAVED COLORS list ($states/sca), one 34 px row per entry on
        // a 44 px pitch. Index 0 is the file's `sample_color`, which is never
        // deletable — its hint says so and no hold charges on it. Everything
        // after it charges a 1.5 s hold-to-delete: releasing early (still over
        // the row) applies the colour instead.
        if self.saved_drop_open {
            if self.saved_colors.is_empty() {
                ui::text(
                    v,
                    cx + 8.0,
                    y + 7.0,
                    "No saved colors yet",
                    11.0,
                    ui::fg3(pal),
                    false,
                );
                y += 30.0;
            } else {
                y += 10.0;
                for i in 0..self.saved_colors.len().min(200) {
                    let key = crate::shell::SAVED_KEY_BASE + i as u32;
                    let (name, hex) = self.saved_colors[i].clone();
                    let hov = self.hover_key == key;
                    let held = self.saved_del_hold == Some(key);
                    let base = (pal.bg & 0xffffff00) | 0x10;
                    let fill = if held {
                        mix(base, ui::DANGER, self.saved_del_fill)
                    } else if hov {
                        ui::hover(pal)
                    } else {
                        base
                    };
                    let hint = if i == 0 { "sample" } else { "hold to delete" };
                    let shown: String = name.chars().take(18).collect();
                    v.push(Cmd::Rect {
                        x: cx + 8.0,
                        y,
                        w: roww,
                        h: 34.0,
                        r: 8.0,
                        color: fill,
                    });
                    v.push(Cmd::Rect {
                        x: cx + 20.0,
                        y: y + 8.0,
                        w: 18.0,
                        h: 18.0,
                        r: 9.0,
                        color: self.hex_display(&hex),
                    });
                    ui::text(v, cx + 50.0, y + 9.0, &shown, 12.0, pal.fg, false);
                    ui::text_r(v, w - 26.0, y + 11.0, hint, 9.5, ui::fg3(pal), false);
                    self.region(cx + 8.0, y, roww, 34.0, key);
                    y += 44.0;
                }
            }
        }

        // ---- CUSTOM ACCENT dropdown (from $states2/custom_acc.json) ----
        ui::text(v, cx + 8.0, y, "CUSTOM ACCENT", 9.5, ui::fg3(pal), false);
        y += 22.0;
        let sel_name = self.current_acc_name();
        let cname = sel_name.clone();
        let sel_col = self
            .custom_accs
            .iter()
            .find(|a| a.name == sel_name)
            .map(|a| self.accent_display(a))
            .unwrap_or(cur);
        // dropdown trigger row (key 218): shows the current selection, toggles
        // the popup open/closed
        {
            let key = 218u32;
            let open = self.custom_acc_drop_open;
            let hov = self.hover_key == key;
            v.push(Cmd::Rect {
                x: cx + 8.0,
                y,
                w: roww,
                h: 40.0,
                r: 10.0,
                color: if hov {
                    ui::hover(pal)
                } else {
                    (pal.bg & 0xffffff00) | 0x12
                },
            });
            v.push(Cmd::Rect {
                x: cx + 20.0,
                y: y + 10.0,
                w: 20.0,
                h: 20.0,
                r: 10.0,
                color: sel_col,
            });
            let empty = sel_name.is_empty();
            // same truncation the publisher publishes: the Save chip shares
            // the row's right edge with the chevron, so a pending hex shortens
            // the name to 10 chars (placeholder stays 18).
            let shown: String = if empty {
                "Select a custom accent…".chars().take(18).collect()
            } else {
                sel_name
                    .chars()
                    .take(if self.acc_hex_pending_save { 10 } else { 18 })
                    .collect()
            };
            ui::text(
                v,
                cx + 50.0,
                y + 12.0,
                &shown,
                if empty { 11.5 } else { 13.0 },
                if empty { ui::fg3(pal) } else { pal.fg },
                false,
            );
            ui::text(
                v,
                w - 48.0,
                y + 11.0,
                if open {
                    ICON_CHEVRON_DOWN
                } else {
                    ICON_CHEVRON_UP
                },
                13.0,
                ui::fg3(pal),
                true,
            );
            self.region(cx + 8.0, y, roww, 40.0, key);
            // Save chip (key 231) on the Custom accent row — only while
            // `$states/custom_acc_${s}` holds a hex (flag cached by
            // `reload_acc_hex_flag` on boot / colors reload). Declared AFTER
            // region 218 so hit-test's reverse walk lets the chip win the
            // overlap. Geometry mirrors shell.ron: w−120, y+6, 64×28.
            if self.acc_hex_pending_save {
                let skey = 231u32;
                let shov = self.hover_key == skey;
                let sx = w - 120.0;
                let sy = y + 6.0;
                v.push(Cmd::Rect {
                    x: sx,
                    y: sy,
                    w: 64.0,
                    h: 28.0,
                    r: 8.0,
                    color: if shov {
                        mix(pal.acc, pal.bg, 0.22)
                    } else {
                        pal.acc
                    },
                });
                ui::text(v, sx + 10.0, sy + 7.0, ICON_SAVE, 12.0, pal.sfg, true);
                ui::text(v, sx + 26.0, sy + 7.5, "save", 11.0, pal.sfg, false);
                self.region(sx, sy, 64.0, 28.0, skey);
            }
            y += 40.0;
        }
        // open dropdown popup: all accents as a grid of color circles (each
        // labeled below); clicking one selects it and applies via
        // `theme_main accent custom_acc <name>`
        if self.custom_acc_drop_open && !self.custom_accs.is_empty() {
            const CIRCLE: f32 = 40.0; // circle diameter
            const LABEL_H: f32 = 14.0; // name label under each circle
            const GAP_X: f32 = 12.0;
            const GAP_Y: f32 = 18.0; // vertical space between label and next circle
            let cols = (roww / (CIRCLE + GAP_X)).floor().max(1.0) as usize;
            let rows = self.custom_accs.len().div_ceil(cols);
            let visible_rows = 3usize;
            let n_rows = rows;
            // `saturating_sub`, not `-`: a custom_acc.json with fewer than
            // three rows of circles (a single accent, say) used to underflow
            // the usize and panic the whole draw. The window is empty then
            // either way.
            let scroll = (self.custom_acc_drop_scroll / cols).min(n_rows.saturating_sub(visible_rows));
            let pop_w = cols as f32 * CIRCLE + (cols as f32 - 1.0) * GAP_X;
            let pop_x = cx + 8.0 + (roww - pop_w) / 2.0;
            let pop_y = y + 4.0;
            let pop_h = visible_rows as f32 * (CIRCLE + LABEL_H + GAP_Y) + 8.0;
            v.push(Cmd::Rect {
                x: cx + 8.0,
                y: pop_y,
                w: roww,
                h: pop_h,
                r: 12.0,
                color: (pal.bg & 0xffffff00) | 0xd8,
            });
            v.push(Cmd::Rect {
                x: cx + 8.0,
                y: pop_y,
                w: roww,
                h: 2.0,
                r: 1.0,
                color: mix(pal.acc, pal.bg, 0.35),
            });
            for r in 0..visible_rows {
                for c in 0..cols {
                    let idx = (scroll + r) * cols + c;
                    if idx >= self.custom_accs.len() {
                        continue;
                    }
                    // snapshot name/color so region() can borrow `self` mutably
                    let (name, col, sel) = {
                        let a = &self.custom_accs[idx];
                        (a.name.clone(), self.accent_display(a), a.name == cname)
                    };
                    let key = crate::shell::CUSTOM_ACC_KEY_BASE + idx as u32;
                    let cx0 = pop_x + c as f32 * (CIRCLE + GAP_X);
                    let cy0 = pop_y + 4.0 + r as f32 * (CIRCLE + LABEL_H + GAP_Y);
                    let hov = self.hover_key == key;
                    // hover/selection ring
                    if sel || hov {
                        v.push(Cmd::Rect {
                            x: cx0 - 3.0,
                            y: cy0 - 3.0,
                            w: CIRCLE + 6.0,
                            h: CIRCLE + 6.0,
                            r: (CIRCLE + 6.0) / 2.0,
                            color: if sel {
                                mix(pal.acc, pal.bg, 0.35)
                            } else {
                                ui::hover(pal)
                            },
                        });
                    }
                    // the color circle
                    v.push(Cmd::Rect {
                        x: cx0,
                        y: cy0,
                        w: CIRCLE,
                        h: CIRCLE,
                        r: CIRCLE / 2.0,
                        color: col,
                    });
                    if sel {
                        // small check inside the circle
                        ui::text(
                            v,
                            cx0 + CIRCLE / 2.0 - 6.0,
                            cy0 + CIRCLE / 2.0 - 9.0,
                            ICON_CHECK,
                            15.0,
                            pal.sfg,
                            true,
                        );
                    }
                    // name label under the circle
                    let shown: String = name.chars().take(10).collect();
                    ui::text(
                        v,
                        cx0 - 20.0,
                        cy0 + CIRCLE + 2.0,
                        &shown,
                        9.5,
                        if sel { pal.sfg } else { pal.fg },
                        false,
                    );
                    self.region(
                        cx0 - 4.0,
                        cy0 - 4.0,
                        CIRCLE + 8.0,
                        CIRCLE + LABEL_H + 4.0,
                        key,
                    );
                }
            }
            if n_rows > visible_rows {
                let bar_max = pop_h - 8.0;
                let bar_h = (bar_max * visible_rows as f32 / n_rows as f32).clamp(14.0, bar_max);
                let bar_y = pop_y
                    + 4.0
                    + (scroll as f32 / (n_rows - visible_rows) as f32) * (bar_max - bar_h);
                v.push(Cmd::Rect {
                    x: cx + roww - 6.0,
                    y: bar_y,
                    w: 2.0,
                    h: bar_h,
                    r: 1.0,
                    color: mix(pal.acc, pal.bg, 0.35),
                });
            }
            // remember latest row-count so scroll resets can use it
            self.custom_acc_drop_scroll = scroll * cols;
            y += pop_h;
        } else if self.custom_acc_drop_open {
            ui::text(
                v,
                cx + 8.0,
                y,
                "No custom accents — run gen_custom_acc_files",
                11.0,
                ui::fg3(pal),
                false,
            );
            y += 30.0;
        }

        // ---- ICON FONT (Settings → "Icon font": 4 slot flavors) ----
        ui::text(v, cx + 8.0, y, "ICON FONT", 9.5, ui::fg3(pal), false);
        y += 22.0;
        let cur_style = crate::icons::IconStyle::parse(&self.icon_style);
        let styles: [(u32, crate::icons::IconStyle); 4] = [
            (224u32, crate::icons::IconStyle::Google),
            (225u32, crate::icons::IconStyle::Caskaydia),
            (226u32, crate::icons::IconStyle::JetBrains),
            (227u32, crate::icons::IconStyle::Phosphor),
        ];
        for (key, style) in styles {
            let sel = style == cur_style;
            let hov = self.hover_key == key;
            let sfmt = if sel { pal.sfg } else { pal.fg };
            v.push(Cmd::Rect {
                x: cx + 8.0,
                y,
                w: roww,
                h: 38.0,
                r: 10.0,
                color: if sel {
                    mix(pal.acc, pal.bg, 0.35)
                } else if hov {
                    ui::hover(pal)
                } else {
                    (pal.bg & 0xffffff00) | 0x12
                },
            });
            ui::text(
                v,
                cx + 24.0,
                y + 11.0,
                ICON_APPS.to_string(),
                13.0,
                if sel { pal.sfg } else { pal.acc },
                true,
            );
            ui::text(v, cx + 50.0, y + 11.0, style.label(), 12.5, sfmt, false);
            if sel {
                v.push(Cmd::Rect {
                    x: w - 34.0,
                    y: y + 13.0,
                    w: 10.0,
                    h: 10.0,
                    r: 5.0,
                    color: pal.sfg,
                });
            }
            self.region(cx + 8.0, y, roww, 38.0, key);
            y += 50.0;
        }

        // ---- APPLY CHANGES (runs `theme_main restore`) ----
        {
            let key = 217u32;
            let hov = self.hover_key == key;
            v.push(Cmd::Rect {
                x: cx + 8.0,
                y,
                w: roww,
                h: 38.0,
                r: 8.0,
                color: if hov {
                    mix(pal.acc, pal.bg, 0.22)
                } else {
                    pal.acc
                },
            });
            ui::text(
                v,
                cx + 24.0,
                y + 11.0,
                ICON_SAVE.to_string(),
                14.0,
                pal.sfg,
                true,
            );
            ui::text(
                v,
                cx + 50.0,
                y + 11.0,
                "Apply changes",
                12.5,
                pal.sfg,
                false,
            );
            self.region(cx + 8.0, y, roww, 38.0, key);
        }
    }

    // ── Pill ──
    // DECLARED in shell.ron (the `tab_pill` section of `scontent`), so this
    // painter is the parity ORACLE and nothing else — it is not compiled into
    // a release binary and the dispatcher has no arm for it. Kept verbatim
    // because `the_settings_pill_scene_matches_its_own_drawer` compares the
    // declared section's whole `Cmd` stream against it; deleting this would
    // delete the evidence that the conversion was lossless.
    #[cfg(test)]
    pub(crate) fn layout_settings_pill(&mut self, v: &mut Vec<Cmd>, w: f32, cx: f32, pal: &Pal) {
        // the shared clamped offset (see `pill_scroll_px`): the same accessor
        // the scene publisher and the drag hit-maths read.
        let so = -self.pill_scroll_px();
        ui::text(v, cx + 8.0, 50.0, "PILL", 9.5, ui::fg3(pal), false);

        // ── Margin slider ──
        v.push(Cmd::Rect {
            x: cx + 24.0,
            y: so + 64.0,
            w: 30.0,
            h: 30.0,
            r: 9.0,
            color: mix(ui::hover(pal), pal.acc, 0.20),
        });
        ui::text(v, cx + 31.0, so + 71.0, ICON_PALETTE, 14.0, pal.acc, true);
        ui::text(v, cx + 64.0, so + 64.0, "Margin", 13.0, pal.fg, false);
        ui::text(
            v,
            cx + 64.0,
            so + 82.0,
            format!("{:.0}px", self.pill_margin_x),
            10.5,
            ui::fg3(pal),
            false,
        );
        ui::slider(
            v,
            cx + 64.0,
            so + 94.0,
            w - cx - 96.0,
            10.0,
            self.pill_margin_x / 30.0,
            self.hover_key == 36,
            pal,
        );
        self.region(cx + 10.0, so + 55.0, w - cx - 20.0, 44.0, 36);
        // ── Pill Transparency slider ──
        // The tile, glyph and both labels scroll WITH the rest (`so + …`):
        // this row used to pin them at 108/115/126 while its slider and hit
        // region moved, so scrolling the pane left the label behind its own
        // slider. The card row below is the shape every row here follows.
        v.push(Cmd::Rect {
            x: cx + 24.0,
            y: so + 108.0,
            w: 30.0,
            h: 30.0,
            r: 9.0,
            color: mix(ui::hover(pal), pal.acc, 0.20),
        });
        ui::text(
            v,
            cx + 31.0,
            so + 115.0,
            ICON_VISIBILITY,
            14.0,
            pal.acc,
            true,
        );
        ui::text(
            v,
            cx + 64.0,
            so + 108.0,
            "Pill transparency",
            13.0,
            pal.fg,
            false,
        );
        let pct = (self.pill_alpha as f32 / 255.0 * 100.0) as u32;
        ui::text(
            v,
            cx + 64.0,
            so + 126.0,
            format!("{pct}%"),
            10.5,
            ui::fg3(pal),
            false,
        );
        ui::slider(
            v,
            cx + 64.0,
            so + 138.0,
            w - cx - 96.0,
            10.0,
            self.pill_alpha as f32 / 255.0,
            self.hover_key == 37,
            pal,
        );
        self.region(cx + 10.0, so + 99.0, w - cx - 20.0, 44.0, 37);
        // ── Dashboard card Transparency slider ──
        v.push(Cmd::Rect {
            x: cx + 24.0,
            y: so + 150.0,
            w: 30.0,
            h: 30.0,
            r: 9.0,
            color: mix(ui::hover(pal), pal.acc, 0.20),
        });
        ui::text(
            v,
            cx + 31.0,
            so + 157.0,
            ICON_VISIBILITY,
            14.0,
            pal.acc,
            true,
        );
        ui::text(
            v,
            cx + 64.0,
            so + 150.0,
            "Card transparency",
            13.0,
            pal.fg,
            false,
        );
        let card_pct = (self.dash_card_alpha as f32 / 255.0 * 100.0) as u32;
        ui::text(
            v,
            cx + 64.0,
            so + 168.0,
            format!("{card_pct}%"),
            10.5,
            ui::fg3(pal),
            false,
        );
        ui::slider(
            v,
            cx + 64.0,
            so + 182.0,
            w - cx - 96.0,
            10.0,
            self.dash_card_alpha as f32 / 255.0,
            self.hover_key == 38,
            pal,
        );
        self.region(cx + 10.0, so + 141.0, w - cx - 20.0, 44.0, 38);

        let toggles: [(u32, &str, &str, bool); 22] = [
            (20, ICON_EXPAND, "Expand on hover", self.expand_on_hover),
            (28, ICON_FLOATING, "Floating", self.bar_floating),
            (29, ICON_SQUARE, "Reserve space", self.bar_reserve),
            (27, ICON_BRIGHTNESS, "Brightness chip", self.pill_brightness),
            (35, ICON_ROWS, "Open Control Center", self.cc_as_dashboard),
            (21, ICON_MONITOR, "Workspaces", self.pill_ws),
            (22, ICON_CLOCK, "Clock", self.pill_clock),
            (23, ICON_BATTERY, "Battery", self.pill_battery),
            (24, ICON_WINDOWS, "Tray icons", self.pill_tray),
            (31, ICON_NOTE, "Visualizer", self.pill_viz),
            (25, ICON_SETTINGS, "Settings chip", self.pill_settings),
            (32, ICON_BELL, "Notifications", self.pill_notif),
            (33, ICON_USER, "User info", self.pill_user),
            (34, ICON_BOLT, "Battery draw", self.pill_watts),
            (40, ICON_MONITOR, "Workspaces 1-5", self.pill_ws_long),
            (41, ICON_MONITOR, "Active workspace", self.pill_ws_short),
            (42, ICON_WIFI, "Wi-Fi", self.pill_wifi),
            (43, ICON_BLUETOOTH, "Bluetooth", self.pill_bluetooth),
            (44, ICON_VOLUME, "Volume", self.pill_volume),
            (45, ICON_WALLPAPER, "Wallpaper", self.pill_wallpaper),
            (46, ICON_PALETTE, "Themes", self.pill_themes),
            (47, ICON_SPARKLE, "Branding", self.pill_branding),
        ];

        // vertical space taken by the "expand on hover" info tip (shown only
        // while that toggle is ON — everything below shifts down by this)
        let tip = if self.expand_on_hover {
            SETTINGS_EXPAND_TIP_H
        } else {
            0.0
        };

        // behavior toggles (first 5) — plain rows, not reorderable
        for (i, &(key, glyph, label, on)) in toggles.iter().take(5).enumerate() {
            let y = 196.0 + so + i as f32 * 40.0 + (if i == 0 { 0.0 } else { tip });
            Self::settings_toggle_row(
                v,
                pal,
                cx + 12.0,
                y,
                w - cx - 24.0,
                glyph,
                label,
                on,
                self.hover_key == key,
            );
            self.region(cx + 4.0, y - 2.0, w - cx - 8.0, 36.0, key);
        }

        // ── "Expand on hover" info tip: with hover-expand ON, clickable
        // items on the resting pill are discouraged (hover opens them anyway)
        if tip > 0.0 {
            let ty = 236.0 + so; // the slot right under the first toggle row
            let bxx = cx + 12.0;
            let bww = w - cx - 24.0;
            v.push(Cmd::Rect {
                x: bxx,
                y: ty,
                w: bww,
                h: SETTINGS_EXPAND_TIP_H,
                r: 10.0,
                color: ui::acc_tint(pal),
            });
            ui::text(v, bxx + 12.0, ty + 14.0, ICON_INFO, 14.0, pal.acc, true);
            ui::text(
                v,
                bxx + 36.0,
                ty + 7.0,
                "Expand on hover is ON",
                10.5,
                pal.fg,
                false,
            );
            ui::text(
                v,
                bxx + 36.0,
                ty + 24.0,
                "Adding clickable items to the resting pill is discouraged.",
                10.5,
                ui::fg2(pal),
                false,
            );
        }

        // reorderable pill items (last 16) — tap to toggle, drag to reorder
        ui::text(
            v,
            cx + 8.0,
            404.0 + so + tip,
            "PILL ITEMS · hold & drag to reorder",
            9.5,
            ui::fg3(pal),
            false,
        );
        let pill_item_toggles = &toggles[5..];
        let lifted = self.pill_drag.as_ref().and_then(|d| {
            let dx = d.x - d.sx;
            let dy = d.y - d.sy;
            if dx * dx + dy * dy > 16.0 {
                Some((d.x, d.y))
            } else {
                None
            }
        });
        for (pi, &(key, glyph, label, on)) in pill_item_toggles.iter().enumerate() {
            let y = 422.0 + so + tip + pi as f32 * 40.0;
            let this_dragging = self.pill_drag.as_ref().is_some_and(|d| {
                Self::settings_key_to_pill(key).is_some_and(|p| {
                    self.pill_order.iter().position(|&x| x == p) == Some(d.from_idx)
                })
            }) && lifted.is_some();
            if this_dragging {
                v.push(Cmd::Rect {
                    x: cx + 12.0,
                    y,
                    w: w - cx - 24.0,
                    h: 36.0,
                    r: 10.0,
                    color: (pal.bg & 0xffffff00) | 0x30,
                });
                continue;
            }
            let grip_hov = self.hover_key == key;
            let grip_color = if grip_hov { pal.fg } else { ui::fg3(pal) };
            ui::text(v, cx + 2.0, y + 10.0, ICON_GRIP, 14.0, grip_color, true);
            Self::settings_toggle_row(
                v,
                pal,
                cx + 12.0,
                y,
                w - cx - 24.0,
                glyph,
                label,
                on,
                self.hover_key == key,
            );
            self.region(cx + 4.0, y - 2.0, w - cx - 8.0, 36.0, key);
        }
        // the dragged row floats under the cursor
        if let Some((dx_, dy_)) = lifted {
            let found = pill_item_toggles.iter().find(|&&(key, ..)| {
                self.pill_drag.as_ref().is_some_and(|d| {
                    Self::settings_key_to_pill(key).is_some_and(|p| {
                        self.pill_order.iter().position(|&x| x == p) == Some(d.from_idx)
                    })
                })
            });
            if let Some(&(_, d_glyph, d_label, d_on)) = found {
                let row_w = w - cx - 24.0;
                let fx = (dx_ - row_w / 2.0).clamp(cx + 8.0, w - row_w - 8.0);
                let fy = dy_ - 19.0;
                Self::settings_toggle_row(v, pal, fx, fy, row_w, d_glyph, d_label, d_on, false);
            }
        }
        // battery percentage readout
        let bp_y = 422.0 + so + 17.0 * 40.0 + tip;
        Self::settings_toggle_row(
            v,
            pal,
            cx + 12.0,
            bp_y,
            w - cx - 24.0,
            ICON_BATTERY,
            "Battery %",
            self.pill_battery_pct,
            self.hover_key == 39,
        );
        self.region(cx + 4.0, bp_y - 2.0, w - cx - 8.0, 36.0, 39);
        // Position row: four edge chips choose where the bar sits (left/top/
        // right/bottom). The active edge is accented; clicking re-anchors.
        let py = bp_y + 46.0;
        v.push(Cmd::Rect {
            x: cx + 24.0,
            y: py + 8.0,
            w: 30.0,
            h: 30.0,
            r: 9.0,
            color: mix(ui::hover(pal), pal.fg, 0.20),
        });
        ui::text(
            v,
            cx + 31.0,
            py + 15.0,
            ICON_SQUARE,
            14.0,
            ui::fg2(pal),
            true,
        );
        ui::text(v, cx + 64.0, py + 15.0, "Position", 13.0, pal.fg, false);
        let chips: [(u32, &str, crate::shell::BarEdge); 4] = [
            (SYS_EDGE_L_KEY, ICON_BACK, crate::shell::BarEdge::Left),
            (SYS_EDGE_T_KEY, ICON_ANCHOR_UP, crate::shell::BarEdge::Top),
            (SYS_EDGE_R_KEY, ICON_FORWARD, crate::shell::BarEdge::Right),
            (
                SYS_EDGE_B_KEY,
                ICON_ANCHOR_DOWN,
                crate::shell::BarEdge::Bottom,
            ),
        ];
        let (cw, gap, ch) = (34.0, 6.0, 30.0);
        let mut cx0 = cx + 140.0;
        for (key, glyph, edge) in chips {
            let active = self.bar_edge == edge;
            let bg = if active {
                (pal.acc & 0xffff_ff00) | 0x3e
            } else {
                self.hl(key, ui::hover(pal), ui::hover_hl(pal))
            };
            v.push(Cmd::Rect {
                x: cx0,
                y: py + 8.0,
                w: cw,
                h: ch,
                r: ch / 2.0,
                color: bg,
            });
            ui::text_c(
                v,
                cx0 + cw / 2.0,
                py + 8.0 + (ch - 12.0) / 2.0 + 1.0,
                glyph,
                12.0,
                if active { pal.acc } else { pal.fg },
                true,
            );
            self.region(cx0, py + 8.0, cw, ch, key);
            cx0 += cw + gap;
        }
    }

    // ── System ──
    // DECLARED in shell.ron (the `tab_system` section of `scontent`), so this
    // painter is the parity ORACLE and nothing else — it is not compiled into
    // a release binary and the dispatcher has no arm for it. Kept verbatim
    // because `the_settings_system_scene_matches_its_own_drawer` compares the
    // declared section's whole `Cmd` stream against it; deleting this would
    // delete the evidence that the conversion was lossless.
    #[cfg(test)]
    pub(crate) fn layout_settings_system(
        &mut self,
        v: &mut Vec<Cmd>,
        w: f32,
        cx: f32,
        h: f32,
        pal: &Pal,
    ) {
        ui::text(v, cx + 8.0, 50.0, "SYSTEM", 9.5, ui::fg3(pal), false);
        ui::card(
            v,
            cx + 8.0,
            58.0,
            w - cx - 16.0,
            58.0,
            14.0,
            ui::raised(pal),
        );
        ui::outline(
            v,
            cx + 8.0,
            58.0,
            w - cx - 16.0,
            58.0,
            14.0,
            ui::hairline(pal),
        );
        let rows: [(String, i32); 3] = [
            ("CPU".into(), self.cpu),
            ("RAM".into(), self.mem_pct),
            ("DISK".into(), self.disk_pct),
        ];
        for (i, (label, pct)) in rows.iter().enumerate() {
            let y = 66.0 + i as f32 * 14.0;
            ui::text(v, cx + 20.0, y, label.clone(), 9.0, pal.fg, false);
            ui::bar(
                v,
                cx + 62.0,
                y + 1.0,
                w - cx - 62.0 - 96.0,
                5.0,
                *pct as f32 / 100.0,
                pal.acc,
                ui::hover(pal),
            );
            ui::text_r(v, w - 24.0, y, format!("{pct}%"), 9.0, ui::fg3(pal), false);
        }
        if !self.net_up.is_empty() || !self.net_down.is_empty() {
            ui::text_r(
                v,
                w - 16.0,
                122.0,
                format!("↓ {} · ↑ {}", self.net_down, self.net_up),
                9.0,
                ui::fg3(pal),
                false,
            );
        }
        ui::text_r(
            v,
            w - 16.0,
            h - 20.0,
            "zen-shell v0.1.0",
            9.0,
            ui::fg3(pal),
            false,
        );
    }

    // ── Misc ──
    // DECLARED in shell.ron (the `tab_misc` section of `scontent`), so this
    // painter is the parity ORACLE and nothing else — it is not compiled into
    // a release binary and the dispatcher has no arm for it. Kept verbatim
    // (cx-relative geometry, `ui::slider`, the region registration) because
    // `the_settings_misc_scene_matches_its_own_drawer` compares the declared
    // scene's whole `Cmd` stream against it; deleting this would delete the
    // evidence that the conversion was lossless.
    #[cfg(test)]
    pub(crate) fn layout_settings_misc(&mut self, v: &mut Vec<Cmd>, w: f32, cx: f32, pal: &Pal) {
        ui::text(v, cx + 8.0, 50.0, "MISC", 9.5, ui::fg3(pal), false);

        // ── Power menu hold delay slider (0.5..3.0 s, default 1.5 s) ──
        let key = 220u32;
        let hov = self.hover_key == key;
        let icbg = mix(ui::hover(pal), pal.acc, 0.20);
        v.push(Cmd::Rect {
            x: cx + 24.0,
            y: 64.0,
            w: 30.0,
            h: 30.0,
            r: 9.0,
            color: icbg,
        });
        ui::text(v, cx + 31.0, 71.0, ICON_TIMER, 14.0, pal.acc, true);
        ui::text(
            v,
            cx + 64.0,
            64.0,
            "Power menu hold delay",
            13.0,
            pal.fg,
            false,
        );
        ui::text(
            v,
            cx + 64.0,
            82.0,
            format!("{:.1}s", self.power_hold_delay),
            10.5,
            ui::fg3(pal),
            false,
        );
        ui::slider(
            v,
            cx + 64.0,
            96.0,
            w - cx - 96.0,
            10.0,
            (self.power_hold_delay - 0.5) / 2.5,
            hov,
            pal,
        );
        self.region(cx + 10.0, 55.0, w - cx - 20.0, 44.0, key);
    }

    /// Compact Android-style toggle row for the Pill section: small tinted
    /// icon, label, 38px switch. Caller registers the hit region.
    ///
    /// The Pill section is DECLARED (the `pill_switch` delegate in shell.ron),
    /// so this is the oracle's own copy and nothing else compiles it — the
    /// same arrangement as `layout_settings_pill` itself.
    #[cfg(test)]
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn settings_toggle_row(
        v: &mut Vec<Cmd>,
        pal: &Pal,
        x: f32,
        y: f32,
        row_w: f32,
        glyph: &str,
        label: &str,
        on: bool,
        hovered: bool,
    ) {
        if hovered {
            v.push(Cmd::Rect {
                x,
                y,
                w: row_w,
                h: 40.0,
                r: 8.0,
                color: ui::hover_hl(pal),
            });
        }
        // flat icon tile — quiet when off, accented when on
        let ic = if on { pal.acc } else { ui::fg3(pal) };
        v.push(Cmd::Rect {
            x: x + 8.0,
            y: y + 8.0,
            w: 24.0,
            h: 24.0,
            r: 6.0,
            color: ui::hover(pal),
        });
        ui::text(v, x + 14.0, y + 14.0, glyph, 12.0, ic, true);
        ui::text(
            v,
            x + 44.0,
            y + 14.0,
            label,
            12.5,
            if hovered { pal.fg } else { ui::fg2(pal) },
            false,
        );
        // slim 36×18 switch
        let tx = x + row_w - 12.0 - 36.0;
        v.push(Cmd::Rect {
            x: tx,
            y: y + 11.0,
            w: 36.0,
            h: 18.0,
            r: 9.0,
            color: if on {
                pal.acc
            } else {
                mix(pal.bg, pal.fg, 0.14)
            },
        });
        v.push(Cmd::Rect {
            x: if on { tx + 36.0 - 16.0 } else { tx + 2.0 },
            y: y + 13.0,
            w: 14.0,
            h: 14.0,
            r: 7.0,
            color: if on { 0xffffffff } else { ui::fg3(pal) },
        });
    }

    /// Android-style settings row: tinted icon square + label (+ optional
    /// summary) on the left, toggle switch on the right, full-row highlight on
    /// hover. Returns the switch's x (for the caller's split hit regions).
    ///
    /// Test-only now: every caller is a cfg(test) parity oracle, and the
    /// declared sections spell this recipe out in shell.ron instead.
    #[cfg(test)]
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn settings_row(
        v: &mut Vec<Cmd>,
        pal: &Pal,
        x: f32,
        y: f32,
        row_w: f32,
        glyph: &str,
        label: &str,
        summary: Option<&str>,
        on: bool,
        hovered: bool,
    ) -> f32 {
        if hovered {
            v.push(Cmd::Rect {
                x,
                y,
                w: row_w,
                h: 48.0,
                r: 8.0,
                color: ui::hover_hl(pal),
            });
        }
        // flat icon tile — quiet when off, accented when on
        let ic = if on { pal.acc } else { ui::fg3(pal) };
        v.push(Cmd::Rect {
            x: x + 8.0,
            y: y + 9.0,
            w: 30.0,
            h: 30.0,
            r: 7.0,
            color: ui::hover(pal),
        });
        ui::text(v, x + 15.0, y + 17.0, glyph, 14.0, ic, true);
        let ly = if summary.is_some() { y + 9.0 } else { y + 16.0 };
        ui::text(v, x + 48.0, ly, label, 13.0, pal.fg, false);
        if let Some(s) = summary {
            ui::text(v, x + 48.0, y + 27.0, s, 10.5, ui::fg3(pal), false);
        }
        // slim 38×20 switch
        let tx = x + row_w - 16.0 - 38.0;
        v.push(Cmd::Rect {
            x: tx,
            y: y + 14.0,
            w: 38.0,
            h: 20.0,
            r: 10.0,
            color: if on {
                pal.acc
            } else {
                mix(pal.bg, pal.fg, 0.14)
            },
        });
        v.push(Cmd::Rect {
            x: if on { tx + 38.0 - 17.0 } else { tx + 2.0 },
            y: y + 16.0,
            w: 16.0,
            h: 16.0,
            r: 8.0,
            color: if on { 0xffffffff } else { ui::fg3(pal) },
        });
        tx
    }
}

#[cfg(test)]
mod tests {
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
        let Ok(text) = std::fs::read_to_string(crate::vars::shell_scene_path()) else {
            return;
        };
        let mut scene: crate::scene::ShellScene =
            crate::scene::parse_scene_ron(&text).expect("shell.ron must parse");
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
        let src = include_str!("../input.rs");
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
}
