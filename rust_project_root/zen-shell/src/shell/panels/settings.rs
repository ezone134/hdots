//! Settings app — two-pane: left nav sidebar + right content pane.
//!
//! Every permanent setting here is written back to the channel master
//! `$states/shell_{n,d,l}` by `Shell::save_config` (pill layout, toggles,
//! geometry, dashboard zoom, accent). The glanceable rows (Wi-Fi / BT / CPU /
//! RAM / DISK) are live backend state, not persisted.

use super::*;

/// Sidebar width — mirrors `SETTINGS_SIDEBAR` in shell/mod.rs. Test-only: the
/// widget frame (`settings_frame`) reads the top-level const directly; this
/// alias keeps the cfg(test) byte-parity drawer's original text identical.
#[cfg(test)]
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
    /// Draw the two-pane Settings scene. The frame (header + sidebar) is the
    /// WIDGET tree (`settings_frame`); the `settings` surface const, when one
    /// is declared, draws the declarative body pane over it.
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
        // The FRAME (header, close chip, sidebar nav) is the WIDGET tree
        // (`settings_frame` — WIDGETS.md stage 6): one chrome source in every
        // build. The body pane stays declarative — the `settings` surface
        // draws it from its const (`surface_scenes.rs`), and a build with no
        // surface leaves the pane empty rather than falling back to a second
        // hand-drawn painter that could disagree with the tree.
        self.draw_settings_chrome_widgets(v, w, h, pal);
        // the ported tab bodies draw through the widget tree; every body not
        // ported yet is still covered by the declarative const below
        self.draw_settings_body_widgets(v, w, h, pal);
        let _ = self.draw_shell_surface("settings", v, w, h, pal, &vals);
    }

    /// The Settings frame (header strip, close chip, sidebar nav) drawn
    /// through the widget system. Clicks land on the same nav keys the
    /// retired drawer registered (1 close, 200\u2026204/219 tabs); `click()`'s
    /// Rust arms write the tab through `set_settings_tab`.
    pub(crate) fn draw_settings_chrome_widgets(
        &mut self,
        v: &mut Vec<Cmd>,
        w: f32,
        h: f32,
        pal: &Pal,
    ) {
        let tab = self.settings_tab();
        let hover = self.hover_key;
        let tree = crate::settings_frame::frame(w, h, pal, tab, hover);
        let tree = tree.layout(
            crate::widgets::Rect { x: 0.0, y: 0.0, w, h },
            &mut crate::widgets::NoMeasurer,
        );
        let mut sink = ShellSink { shell: self };
        tree.render(v, &mut sink, pal);
    }

    /// The settings frame's hand-drawn painter — retired to a test-only
    /// byte-parity reference: the widget tree (`settings_frame`) replaced it,
    /// and the adjacent test holds both, drawn under matched shell states,
    /// to identical `Vec<Cmd>` bytes.
    #[cfg(test)]
    pub(crate) fn draw_settings_chrome_native(
        &mut self,
        v: &mut Vec<Cmd>,
        w: f32,
        h: f32,
        pal: &Pal,
    ) {
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
        // The whole settings body is DECLARED (the `settings` surface, now a
        // Lua module `ui/surfaces/settings.lua`), so `draw_shell_surface` above
        // owns it in every build. There is no Rust content painter any more:
        // with no surface the pane simply stays empty rather than falling back
        // to a second hand-drawn painter that could disagree with the scene.
    }
}

impl Shell {
    /// The ported tab bodies, drawn through the widget tree over the rarest
    /// path: the Network tab (`settings_body`). A tab whose body is not
    /// ported yet simply doesn't run here — the declarative const still
    /// covers it — so adding a tab is additive.
    pub(crate) fn draw_settings_body_widgets(&mut self, v: &mut Vec<Cmd>, w: f32, h: f32, pal: &Pal) {
        match self.settings_tab() {
            SettingsTab::Network => {
                let rows = self.network_body_rows();
                let tree = crate::settings_body::network(w, h, pal, self.scale.0, rows);
                let tree = tree.layout(
                    crate::widgets::Rect { x: 0.0, y: 0.0, w, h },
                    &mut crate::widgets::NoMeasurer,
                );
                let mut sink = ShellSink { shell: self };
                tree.render(v, &mut sink, pal);
            }
            _ => {} // every other tab still draws declaratively
        }
    }

    /// The Network body's row inputs, derived from the same shell fields the
    /// declarative publisher reads (radio state, subtitle strings, the
    /// two-key hover gate the scene cannot spell itself).
    pub(crate) fn network_body_rows(&self) -> [crate::settings_body::RadioRow; 2] {
        let wifi_sub = if self.wifi_on && !self.ssid.is_empty() {
            self.ssid.clone()
        } else if self.wifi_on {
            "Not connected".to_string()
        } else {
            "Off".to_string()
        };
        let bt_sub = if self.bt_on {
            self.bt_devices
                .iter()
                .find(|d| d.1)
                .map(|d| d.0.clone())
                .unwrap_or_else(|| "On".to_string())
        } else {
            "Off".to_string()
        };
        [
            crate::settings_body::RadioRow {
                on: self.wifi_on,
                hov: self.hover_key == 2 || self.hover_key == 12,
                sub: wifi_sub,
                glyph: "\u{f1eb}",
                title: "Wi-Fi",
                key: 2,
                switch_key: 12,
                y: 64.0,
            },
            crate::settings_body::RadioRow {
                on: self.bt_on,
                hov: self.hover_key == 3 || self.hover_key == 13,
                sub: bt_sub,
                glyph: "\u{f293}",
                title: "Bluetooth",
                key: 3,
                switch_key: 13,
                y: 118.0,
            },
        ]
    }
}

/// The widget hit-region sink — the shell's `region()` pipeline, exactly
/// what the drawers used to call inline. Hover styling follows the frame's
/// single `hover_key` (the regions the PREVIOUS frame registered under the
/// pointer), so widget hover behaves like drawer hover.
pub(crate) struct ShellSink<'a> {
    pub(crate) shell: &'a mut Shell,
}

impl crate::widgets::RegionSink for ShellSink<'_> {
    fn region(&mut self, x: f32, y: f32, w: f32, h: f32, key: u32) {
        self.shell.region(x, y, w, h, key);
    }
    fn hov(&self, key: u32) -> bool {
        key != 0 && self.shell.hover_key == key
    }
}
#[cfg(test)]
mod tests;
