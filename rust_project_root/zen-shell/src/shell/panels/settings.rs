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

impl Shell {
    /// Draw the two-pane Settings scene. A `settings` surface declared in
    /// `shell.ron` owns the whole scene; without one the built-in Rust layout
    /// below runs unchanged.
    pub(crate) fn layout_settings(&mut self, v: &mut Vec<Cmd>, w: f32, h: f32, pal: &Pal) {
        let vals = self.scene_values();
        if self.draw_shell_surface("settings", v, w, h, pal, &vals, None) {
            return;
        }
        // ── top header strip (spans both panes) ──
        ui::text(v, 22.0, 14.0, "Settings", 17.0, pal.fg, false);
        if self.hover_key == 1 {
            v.push(Cmd::Rect { x: w - 42.0, y: 6.0, w: 30.0, h: 26.0, r: 6.0, color: ui::hover_hl(&pal) });
        }
        ui::text_r(v, w - 26.0, 13.0, "✕", 12.0, if self.hover_key == 1 { pal.fg } else { ui::fg2(&pal) }, false);
        self.region(w - 42.0, 4.0, 36.0, 30.0, 1);

        // ── left nav sidebar ──
        // subtle divider background separating sidebar from content
        v.push(Cmd::Rect { x: 0.0, y: 44.0, w: SIDE, h: h - 44.0, r: 0.0, color: (pal.bg & 0xffffff00) | 0x08 });
        v.push(Cmd::Rect { x: SIDE - 1.0, y: 44.0, w: 1.0, h: h - 44.0, r: 0.0, color: ui::hairline(&pal) });

        let nav: [(u32, &str, &str, SettingsTab); 6] = [
            (NAV_NETWORK, ICON_WIFI, "Network", SettingsTab::Network),
            (NAV_SOUND, ICON_VOLUME, "Sound & Display", SettingsTab::Sound),
            (NAV_APPEARANCE, ICON_PALETTE, "Appearance", SettingsTab::Appearance),
            (NAV_PILL, ICON_PALETTE, "Pill", SettingsTab::Pill),
            (NAV_SYSTEM, ICON_MONITOR, "System", SettingsTab::System),
            (NAV_MISC, ICON_SETTINGS, "Misc", SettingsTab::Misc),
        ];
        for (i, &(key, glyph, label, tab)) in nav.iter().enumerate() {
            let y = 56.0 + i as f32 * 46.0;
            let sel = self.settings_tab == tab;
            let hov = self.hover_key == key;
            if sel {
                v.push(Cmd::Rect { x: 6.0, y: y - 2.0, w: SIDE - 12.0, h: 40.0, r: 8.0, color: ui::hover_hl(&pal) });
            } else if hov {
                v.push(Cmd::Rect { x: 6.0, y: y - 2.0, w: SIDE - 12.0, h: 40.0, r: 8.0, color: ui::hover(&pal) });
            }
            let tc = if sel { pal.acc } else if hov { pal.fg } else { ui::fg2(&pal) };
            ui::text(v, 16.0, y + 13.0, glyph, 14.0, tc, true);
            ui::text(v, 42.0, y + 13.0, label, 12.5, if sel || hov { pal.fg } else { ui::fg2(&pal) }, false);
            if sel {
                // 2px accent rail on the sidebar's left edge
                v.push(Cmd::Rect { x: 0.0, y: y + 8.0, w: 2.0, h: 20.0, r: 1.0, color: pal.acc });
            }
            self.region(2.0, y - 2.0, SIDE - 4.0, 40.0, key);
        }

        // ── right content pane ──
        let cx = SIDE; // left edge of the content pane
        match self.settings_tab {
            SettingsTab::Network => self.layout_settings_network(v, w, cx, pal),
            SettingsTab::Sound => self.layout_settings_sound(v, w, cx, pal),
            SettingsTab::Appearance => self.layout_settings_appearance(v, w, cx, pal),
            SettingsTab::Pill => self.layout_settings_pill(v, w, cx, pal),
            SettingsTab::System => self.layout_settings_system(v, w, cx, h, pal),
            SettingsTab::Misc => self.layout_settings_misc(v, w, cx, pal),
        }
    }

    // ── Network & Internet ──
    pub(crate) fn layout_settings_network(&mut self, v: &mut Vec<Cmd>, w: f32, cx: f32, pal: &Pal) {
        ui::text(v, cx + 8.0, 50.0, "NETWORK & INTERNET", 9.5, ui::fg3(&pal), false);
        let ssid = if self.wifi_on && !self.ssid.is_empty() {
            self.ssid.clone()
        } else if self.wifi_on {
            "Not connected".to_string()
        } else {
            "Off".to_string()
        };
        Self::settings_row(v, pal, cx + 8.0, 64.0, w - cx - 16.0, ICON_WIFI, "Wi-Fi", Some(&ssid), self.wifi_on, self.hover_key == 2 || self.hover_key == 12);
        self.region(cx + 8.0, 64.0, w - cx - 16.0 - 58.0, 48.0, 2);
        self.region(w - 74.0, 76.0, 42.0, 24.0, 12);

        let bt = if self.bt_on {
            self.bt_devices.iter().find(|d| d.1).map(|d| d.0.clone()).unwrap_or_else(|| "On".to_string())
        } else {
            "Off".to_string()
        };
        Self::settings_row(v, pal, cx + 8.0, 118.0, w - cx - 16.0, ICON_BLUETOOTH, "Bluetooth", Some(&bt), self.bt_on, self.hover_key == 3 || self.hover_key == 13);
        self.region(cx + 8.0, 118.0, w - cx - 16.0 - 58.0, 48.0, 3);
        self.region(w - 74.0, 130.0, 42.0, 24.0, 13);
    }

    // ── Sound & Display ──
    pub(crate) fn layout_settings_sound(&mut self, v: &mut Vec<Cmd>, w: f32, cx: f32, pal: &Pal) {
        ui::text(v, cx + 8.0, 50.0, "SOUND & DISPLAY", 9.5, ui::fg3(&pal), false);
        // brightness
        v.push(Cmd::Rect { x: cx + 24.0, y: 64.0, w: 30.0, h: 30.0, r: 9.0, color: mix(ui::hover(&pal), pal.acc, 0.20) });
        ui::text(v, cx + 31.0, 71.0, ICON_BRIGHTNESS, 14.0, pal.acc, true);
        ui::text(v, cx + 64.0, 72.0, "Brightness", 13.0, pal.fg, false);
        ui::slider(v, cx + 64.0, 90.0, w - cx - 96.0, 10.0, self.brightness, self.hover_key == 5, pal);
        self.region(cx + 10.0, 55.0, w - cx - 20.0, 44.0, 5);
        // volume
        v.push(Cmd::Rect { x: cx + 24.0, y: 108.0, w: 30.0, h: 30.0, r: 9.0, color: mix(ui::hover(&pal), pal.acc, 0.20) });
        ui::text(v, cx + 31.0, 115.0, ICON_VOLUME, 14.0, pal.acc, true);
        ui::text(v, cx + 64.0, 116.0, "Volume", 13.0, pal.fg, false);
        ui::slider(v, cx + 64.0, 134.0, w - cx - 96.0, 10.0, self.volume, self.hover_key == 4, pal);
        self.region(cx + 10.0, 99.0, w - cx - 20.0, 44.0, 4);
        v.push(Cmd::Rect { x: cx + 24.0, y: 152.0, w: 30.0, h: 30.0, r: 9.0, color: mix(ui::hover(&pal), pal.acc, 0.20) });
        ui::text(v, cx + 31.0, 159.0, ICON_SQUARE, 14.0, pal.acc, true);
        ui::text(v, cx + 64.0, 160.0, "Max volume", 13.0, pal.fg, false);
        // toggle chip (on by default)
        let tog_x = w - cx - 118.0;
        let tog_on = self.allow_over_100;
        v.push(Cmd::Rect {
            x: tog_x, y: 154.0, w: 44.0, h: 24.0, r: 12.0,
            color: if tog_on { ui::acc_tint(&pal) } else { mix(ui::hover(&pal), pal.fg, 0.10) },
        });
        ui::text_c(v, tog_x + 22.0, 158.0, if tog_on { ICON_SUNNY } else { ICON_TOGGLE_OFF }, 10.0,
            if tog_on { pal.fg } else { pal.fg }, true);
        self.region(tog_x - 4.0, 150.0, 52.0, 32.0, 14);
        // max volume slider (0..200 → 0..1 of the track)
        ui::slider(v, cx + 64.0, 178.0, w - cx - 96.0, 10.0,
            (self.max_vol as f32 / 200.0).clamp(0.0, 1.0), self.hover_key == 15, pal);
        self.region(cx + 10.0, 153.0, w - cx - 20.0, 34.0, 15);
        // ── Balance slider + reset button ──
        let bal_y = 200.0;
        let bw = w - cx - 96.0;
        let lr = self.vol_left + self.vol_right;
        let bal = if lr > 0.001 { (self.vol_right / lr).clamp(0.0, 1.0) } else { 0.5 };
        let tx = cx + 64.0;
        let cy = bal_y + 3.0;
        let hov59 = self.hover_key == 59;
        ui::text(v, cx + 68.0, bal_y - 3.0, ICON_STAR, 9.0, pal.fg, true);
        ui::fader_bal(v, tx, cy, bw, bal, pal.acc, hov59, pal);
        self.balance_rect = (tx, cy, bw);
        ui::text_r(v, cx + 64.0 + bw - 4.0, bal_y - 3.0, ICON_VISIBILITY, 9.0, pal.fg, true);
        ui::text(v, cx + 64.0, bal_y + 10.0, "Balance", 9.5, ui::fg3(&pal), false);
        // reset button at right of balance row
        let rbx = cx + 64.0 + bw + 8.0;
        let rb_hov = self.hover_key == 58;
        v.push(Cmd::Rect { x: rbx, y: bal_y - 2.0, w: 52.0, h: 22.0, r: 7.0, color: if rb_hov { ui::hover_hl(&pal) } else { mix(ui::hover(&pal), pal.fg, 0.10) }});
        ui::text_c(v, rbx + 26.0, bal_y + 2.0, format!("{} Reset", ICON_SLIDER), 9.0, if rb_hov { pal.acc } else { pal.fg }, false);
        self.region(rbx - 2.0, bal_y - 4.0, 56.0, 26.0, 58);
        self.region(cx + 62.0, bal_y - 2.0, bw + 4.0, 24.0, 59);
    }

    // ── Appearance ──
    pub(crate) fn layout_settings_appearance(&mut self, v: &mut Vec<Cmd>, w: f32, cx: f32, pal: &Pal) {
        // refresh the custom-accent list each draw (regenerated into
        // $states2/custom_acc.json by the theme pipeline) + clamp the scroll
        self.load_custom_accs();
        self.appearance_scroll = self.appearance_scroll.min(self.appearance_max_scroll());
        let roww = w - cx - 16.0;

        // ── fixed header (never scrolls) ──
        ui::text(v, cx + 8.0, 50.0, "APPEARANCE", 9.5, ui::fg3(&pal), false);
        ui::text(v, cx + 8.0, 70.0, "Accent color", 13.0, pal.fg, false);
        // current accent is the live `$acc` from $states2/shell_vars
        let cur = self.sv_acc;
        v.push(Cmd::Rect { x: w - 44.0, y: 70.0, w: 14.0, h: 14.0, r: 7.0, color: cur });
        let cur_hex = format!("#{:06x}", (cur >> 8) & 0xffffff);
        let sw = 28.0;
        let gap = 12.0;
        let series = self.sv_series;
        let n = series.len() as f32;
        let total = n * sw + (n - 1.0) * gap;
        let avail = w - cx - 16.0;
        let mut ax = cx + (avail - total) / 2.0;
        for (i, &c) in series.iter().enumerate() {
            let key = 6 + i as u32;
            let chex = format!("#{:06x}", (c >> 8) & 0xffffff);
            let sel = cur_hex.eq_ignore_ascii_case(&chex);
            if sel {
                v.push(Cmd::Rect { x: ax - 3.0, y: 88.0, w: sw + 6.0, h: sw + 6.0, r: 17.0, color: mix(ui::hover(&pal), c, 0.45) });
            }
            v.push(Cmd::Rect { x: ax, y: 91.0, w: sw, h: sw, r: 14.0, color: c });
            self.region(ax - 8.0, 83.0, sw + 16.0, sw + 16.0, key);
            ax += sw + gap;
        }

        // ── scrollable content ──
        let so = -self.appearance_scroll;
        let mut y = 150.0 + so;

        // ---- ACCENT TOGGLES (writes the per-channel custom_acc_<flag>_<ch>) ----
        ui::text(v, cx + 8.0, y, "ACCENT TOGGLES", 9.5, ui::fg3(&pal), false);
        y += 22.0;
        let toggles = [
            (206u32, "start_icon", ICON_BOLT, "Start icon", self.acc_flag_start_icon),
            (207u32, "app_border", ICON_SQUARE, "App border", self.acc_flag_app_border),
            (208u32, "hyprland", ICON_WINDOWS, "Hyprland border", self.acc_flag_hyprland),
            (209u32, "gtk", ICON_MONITOR, "GTK app bg", self.acc_flag_gtk),
            (210u32, "normal_app", ICON_APP, "Normal app bg", self.acc_flag_normal_app),
        ];
        for (key, _flag, glyph, label, on) in toggles {
            Self::settings_row(v, pal, cx + 8.0, y, roww, &glyph.to_string(), label, None, on, self.hover_key == key);
            self.region(cx + 8.0, y, roww, 48.0, key);
            y += 52.0;
        }
        // start-icon tone slider (0..900)
        {
            let key = 211u32;
            let hov = self.hover_key == key;
            let icbg = mix(ui::hover(&pal), pal.acc, 0.20);
            v.push(Cmd::Rect { x: cx + 24.0, y, w: 30.0, h: 30.0, r: 9.0, color: icbg });
            ui::text(v, cx + 31.0, y + 7.0, ICON_SPARKLE.to_string(), 14.0, pal.acc, true);
            ui::text(v, cx + 64.0, y + 2.0, "Start icon tone", 13.0, pal.fg, false);
            ui::text(v, cx + 64.0, y + 20.0, &format!("{}", self.start_icon_tone), 10.5, ui::fg3(&pal), false);
            ui::slider(v, cx + 64.0, y + 25.0, w - cx - 96.0, 10.0, self.start_icon_tone as f32 / 900.0, hov, pal);
            self.region(cx + 10.0, y, w - cx - 20.0, 40.0, key);
            y += 52.0;
        }

        // ---- ALPHA (lowercase hex in $states/<flag>_<ch>) ----
        ui::text(v, cx + 8.0, y, "ALPHA", 9.5, ui::fg3(&pal), false);
        y += 22.0;
        let alphas = [
            (212u32, "bg_alpha", ICON_PALETTE, "Background", self.bg_alpha),
            (213u32, "acc_alpha", ICON_DROP, "Accent", self.acc_alpha),
            (214u32, "scrim_alpha", ICON_ACTIVE, "Scrim", self.scrim_alpha),
            (215u32, "border_alpha", ICON_SQUARE, "Border", self.border_alpha),
        ];
        for (key, _flag, glyph, label, val) in alphas {
            let hov = self.hover_key == key;
            let icbg = mix(ui::hover(&pal), pal.acc, 0.20);
            v.push(Cmd::Rect { x: cx + 24.0, y, w: 30.0, h: 30.0, r: 9.0, color: icbg });
            ui::text(v, cx + 31.0, y + 7.0, glyph.to_string(), 14.0, pal.acc, true);
            ui::text(v, cx + 64.0, y + 2.0, label, 13.0, pal.fg, false);
            ui::text(v, cx + 64.0, y + 20.0, &format!("{:02x}", val), 10.5, ui::fg3(&pal), false);
            ui::slider(v, cx + 64.0, y + 25.0, w - cx - 96.0, 10.0, val as f32 / 255.0, hov, pal);
            self.region(cx + 10.0, y, w - cx - 20.0, 40.0, key);
            y += 44.0;
        }
        // Accent scrim toggle ($states/acc_scrim_<ch>)
        {
            let key = 216u32;
            Self::settings_row(v, pal, cx + 8.0, y, roww, ICON_PALETTE, "Accent scrim", Some("overlay on wallpaper"), self.acc_scrim, self.hover_key == key);
            self.region(cx + 8.0, y, roww, 48.0, key);
            y += 52.0;
        }

        // ---- ACCENT SOURCE (writes $states/acc_source_<ch> via scheme_main) ----
        ui::text(v, cx + 8.0, y, "ACCENT SOURCE", 9.5, ui::fg3(&pal), false);
        y += 22.0;
        // this-state / all-states scope (wall & default only): shown while a
        // multi-state accent exists — i.e. the current channel isn't `n`.
        // Custom accent is always this-state, so the toggle is accent-source
        // wide but custom rows simply ignore it (they never get the `all` flag).
        if crate::vars::read_channel() != 'n' {
            let scope_y = y;
            let (k0, k1) = (228u32, 229u32);
            let seg_w = 120.0;
            let half = seg_w / 2.0;
            let hov0 = self.hover_key == k0;
            let hov1 = self.hover_key == k1;
            let on0 = !self.acc_all_states;
            let on1 = self.acc_all_states;
            let bg: u32 = (pal.bg & 0xffffff00) | 0x18;
            v.push(Cmd::Rect { x: cx + 8.0, y: scope_y, w: seg_w, h: 26.0, r: 13.0, color: bg });
            v.push(Cmd::Rect { x: cx + 8.0, y: scope_y, w: half, h: 26.0, r: 13.0, color: if on0 { mix(pal.acc, pal.bg, 0.4) } else { bg } });
            ui::text(v, cx + 8.0 + half / 2.0, scope_y + 6.5, "this state", 9.5, if on0 || hov0 { pal.sfg } else { ui::fg3(&pal) }, true);
            ui::text(v, cx + 8.0 + half + half / 2.0, scope_y + 6.5, "all states", 9.5, if on1 || hov1 { pal.sfg } else { ui::fg3(&pal) }, true);
            self.region(cx + 8.0, scope_y, half, 26.0, k0);
            self.region(cx + 8.0 + half, scope_y, half, 26.0, k1);
            y += 36.0;
        }
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
                x: cx + 8.0, y, w: roww, h: 38.0, r: 10.0,
                color: if sel { mix(pal.acc, pal.bg, 0.35) } else if hov { ui::hover(&pal) } else { (pal.bg & 0xffffff00) | 0x12 },
            });
            ui::text(v, cx + 24.0, y + 11.0, glyph.to_string(), 13.0, if sel { pal.sfg } else { pal.acc }, true);
            ui::text(v, cx + 50.0, y + 11.0, label, 12.5, sfmt, false);
            if sel {
                v.push(Cmd::Rect { x: w - 34.0, y: y + 13.0, w: 10.0, h: 10.0, r: 5.0, color: pal.sfg });
            }
            self.region(cx + 8.0, y, roww, 38.0, key);
            y += 50.0;
        }
        // "Pick from screen" — launches hyprpicker (Esc cancels/discards)
        {
            let key = 205u32;
            let hov = self.hover_key == key;
            v.push(Cmd::Rect {
                x: cx + 8.0, y, w: roww, h: 38.0, r: 10.0,
                color: if hov { mix(pal.acc, pal.bg, 0.25) } else { (pal.bg & 0xffffff00) | 0x12 },
            });
            ui::text(v, cx + 24.0, y + 11.0, ICON_SCHEME.to_string(), 13.0, pal.acc, true);
            ui::text(v, cx + 50.0, y + 11.0, "Pick from screen", 12.5, if hov { pal.sfg } else { pal.fg }, false);
            self.region(cx + 8.0, y, roww, 38.0, key);
            y += 50.0;
        }
        // "Custom color" — runs the gcp color-chooser GUI (via
        // `custom_color_main`); the picked hex becomes the custom accent
        // (custom_acc_<ch> + source c). Always this-state; no `all` flag.
        {
            let key = 230u32;
            let hov = self.hover_key == key;
            v.push(Cmd::Rect {
                x: cx + 8.0, y, w: roww, h: 38.0, r: 10.0,
                color: if hov { mix(pal.acc, pal.bg, 0.25) } else { (pal.bg & 0xffffff00) | 0x12 },
            });
            ui::text(v, cx + 24.0, y + 11.0, ICON_DROP.to_string(), 13.0, pal.acc, true);
            ui::text(v, cx + 50.0, y + 11.0, "Custom color", 12.5, if hov { pal.sfg } else { pal.fg }, false);
            self.region(cx + 8.0, y, roww, 38.0, key);
            y += 50.0;
        }

        // ---- CUSTOM ACCENT dropdown (from $states2/custom_acc.json) ----
        ui::text(v, cx + 8.0, y, "CUSTOM ACCENT", 9.5, ui::fg3(&pal), false);
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
                x: cx + 8.0, y, w: roww, h: 40.0, r: 10.0,
                color: if hov { ui::hover(&pal) } else { (pal.bg & 0xffffff00) | 0x12 },
            });
            v.push(Cmd::Rect { x: cx + 20.0, y: y + 10.0, w: 20.0, h: 20.0, r: 10.0, color: sel_col });
            let empty = sel_name.is_empty();
            let shown: String = if empty {
                "Select a custom accent…".chars().take(18).collect()
            } else {
                sel_name.chars().take(18).collect()
            };
            ui::text(v, cx + 50.0, y + 12.0, &shown, if empty { 11.5 } else { 13.0 }, if empty { ui::fg3(&pal) } else { pal.fg }, false);
            ui::text(v, w - 48.0, y + 11.0, if open { ICON_CHEVRON_DOWN } else { ICON_CHEVRON_UP }, 13.0, ui::fg3(&pal), true);
            self.region(cx + 8.0, y, roww, 40.0, key);
            y += 40.0;
        }
        // open dropdown popup: all accents as a grid of color circles (each
        // labeled below); clicking one selects it and applies via
        // `theme_main accent custom_acc <name>`
        if self.custom_acc_drop_open && !self.custom_accs.is_empty() {
            const CIRCLE: f32 = 40.0;      // circle diameter
            const LABEL_H: f32 = 14.0;     // name label under each circle
            const GAP_X: f32 = 12.0;
            const GAP_Y: f32 = 18.0;       // vertical space between label and next circle
            let cols = (roww / (CIRCLE + GAP_X)).floor().max(1.0) as usize;
            let rows = (self.custom_accs.len() + cols - 1) / cols;
            let visible_rows = 3usize;
            let n_rows = rows;
            let scroll = (self.custom_acc_drop_scroll / cols).min(n_rows - visible_rows);
            let pop_w = cols as f32 * CIRCLE + (cols as f32 - 1.0) * GAP_X;
            let pop_x = cx + 8.0 + (roww - pop_w) / 2.0;
            let pop_y = y + 4.0;
            let pop_h = visible_rows as f32 * (CIRCLE + LABEL_H + GAP_Y) + 8.0;
            v.push(Cmd::Rect {
                x: cx + 8.0, y: pop_y, w: roww, h: pop_h, r: 12.0,
                color: (pal.bg & 0xffffff00) | 0xd8,
            });
            v.push(Cmd::Rect { x: cx + 8.0, y: pop_y, w: roww, h: 2.0, r: 1.0, color: mix(pal.acc, pal.bg, 0.35) });
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
                            x: cx0 - 3.0, y: cy0 - 3.0, w: CIRCLE + 6.0, h: CIRCLE + 6.0, r: (CIRCLE + 6.0) / 2.0,
                            color: if sel { mix(pal.acc, pal.bg, 0.35) } else { ui::hover(&pal) },
                        });
                    }
                    // the color circle
                    v.push(Cmd::Rect { x: cx0, y: cy0, w: CIRCLE, h: CIRCLE, r: CIRCLE / 2.0, color: col });
                    if sel {
                        // small check inside the circle
                        ui::text(v, cx0 + CIRCLE / 2.0 - 6.0, cy0 + CIRCLE / 2.0 - 9.0, ICON_CHECK, 15.0, pal.sfg, true);
                    }
                    // name label under the circle
                    let shown: String = name.chars().take(10).collect();
                    ui::text(v, cx0 - 20.0, cy0 + CIRCLE + 2.0, &shown, 9.5, if sel { pal.sfg } else { pal.fg }, false);
                    self.region(cx0 - 4.0, cy0 - 4.0, CIRCLE + 8.0, CIRCLE + LABEL_H + 4.0, key);
                }
            }
            if n_rows > visible_rows {
                let bar_max = pop_h - 8.0;
                let bar_h = (bar_max * visible_rows as f32 / n_rows as f32).clamp(14.0, bar_max);
                let bar_y = pop_y + 4.0 + (scroll as f32 / (n_rows - visible_rows) as f32) * (bar_max - bar_h);
                v.push(Cmd::Rect { x: cx + roww - 6.0, y: bar_y, w: 2.0, h: bar_h, r: 1.0, color: mix(pal.acc, pal.bg, 0.35) });
            }
            // remember latest row-count so scroll resets can use it
            self.custom_acc_drop_scroll = scroll * cols;
            y += pop_h;
        } else if self.custom_acc_drop_open {
            ui::text(v, cx + 8.0, y, "No custom accents — run gen_custom_acc_files", 11.0, ui::fg3(&pal), false);
            y += 30.0;
        }

        // ---- ICON FONT (Settings → "Icon font": 4 slot flavors) ----
        ui::text(v, cx + 8.0, y, "ICON FONT", 9.5, ui::fg3(&pal), false);
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
                x: cx + 8.0, y, w: roww, h: 38.0, r: 10.0,
                color: if sel { mix(pal.acc, pal.bg, 0.35) } else if hov { ui::hover(&pal) } else { (pal.bg & 0xffffff00) | 0x12 },
            });
            ui::text(v, cx + 24.0, y + 11.0, ICON_APPS.to_string(), 13.0, if sel { pal.sfg } else { pal.acc }, true);
            ui::text(v, cx + 50.0, y + 11.0, style.label(), 12.5, sfmt, false);
            if sel {
                v.push(Cmd::Rect { x: w - 34.0, y: y + 13.0, w: 10.0, h: 10.0, r: 5.0, color: pal.sfg });
            }
            self.region(cx + 8.0, y, roww, 38.0, key);
            y += 50.0;
        }

        // ---- APPLY CHANGES (runs `theme_main restore`) ----
        {
            let key = 217u32;
            let hov = self.hover_key == key;
            v.push(Cmd::Rect {
                x: cx + 8.0, y, w: roww, h: 38.0, r: 8.0,
                color: if hov { mix(pal.acc, pal.bg, 0.22) } else { pal.acc },
            });
            ui::text(v, cx + 24.0, y + 11.0, ICON_SAVE.to_string(), 14.0, pal.sfg, true);
            ui::text(v, cx + 50.0, y + 11.0, "Apply changes", 12.5, pal.sfg, false);
            self.region(cx + 8.0, y, roww, 38.0, key);
        }
    }

    // ── Pill ──
    pub(crate) fn layout_settings_pill(&mut self, v: &mut Vec<Cmd>, w: f32, cx: f32, pal: &Pal) {
        // clamp to content height, then shift every row below the fixed header
        self.pill_scroll = self.pill_scroll.min(self.pill_max_scroll());
        let so = -self.pill_scroll;
        ui::text(v, cx + 8.0, 50.0, "PILL", 9.5, ui::fg3(&pal), false);

        // ── Margin slider ──
        v.push(Cmd::Rect { x: cx + 24.0, y: so + 64.0, w: 30.0, h: 30.0, r: 9.0, color: mix(ui::hover(&pal), pal.acc, 0.20) });
        ui::text(v, cx + 31.0, so + 71.0, ICON_PALETTE, 14.0, pal.acc, true);
        ui::text(v, cx + 64.0, so + 64.0, "Margin", 13.0, pal.fg, false);
        ui::text(v, cx + 64.0, so + 82.0, &format!("{:.0}px", self.pill_margin_x), 10.5, ui::fg3(&pal), false);
        ui::slider(v, cx + 64.0, so + 94.0, w - cx - 96.0, 10.0, self.pill_margin_x / 30.0, self.hover_key == 36, pal);
        self.region(cx + 10.0, so + 55.0, w - cx - 20.0, 44.0, 36);
        // ── Pill Transparency slider ──
        v.push(Cmd::Rect { x: cx + 24.0, y: 108.0, w: 30.0, h: 30.0, r: 9.0, color: mix(ui::hover(&pal), pal.acc, 0.20) });
        ui::text(v, cx + 31.0, 115.0, ICON_VISIBILITY, 14.0, pal.acc, true);
        ui::text(v, cx + 64.0, 108.0, "Pill transparency", 13.0, pal.fg, false);
        let pct = (self.pill_alpha as f32 / 255.0 * 100.0) as u32;
        ui::text(v, cx + 64.0, 126.0, &format!("{pct}%"), 10.5, ui::fg3(&pal), false);
        ui::slider(v, cx + 64.0, so + 138.0, w - cx - 96.0, 10.0, self.pill_alpha as f32 / 255.0, self.hover_key == 37, pal);
        self.region(cx + 10.0, so + 99.0, w - cx - 20.0, 44.0, 37);
        // ── Dashboard card Transparency slider ──
        v.push(Cmd::Rect { x: cx + 24.0, y: so + 150.0, w: 30.0, h: 30.0, r: 9.0, color: mix(ui::hover(&pal), pal.acc, 0.20) });
        ui::text(v, cx + 31.0, so + 157.0, ICON_VISIBILITY, 14.0, pal.acc, true);
        ui::text(v, cx + 64.0, so + 150.0, "Card transparency", 13.0, pal.fg, false);
        let card_pct = (self.dash_card_alpha as f32 / 255.0 * 100.0) as u32;
        ui::text(v, cx + 64.0, so + 168.0, &format!("{card_pct}%"), 10.5, ui::fg3(&pal), false);
        ui::slider(v, cx + 64.0, so + 182.0, w - cx - 96.0, 10.0, self.dash_card_alpha as f32 / 255.0, self.hover_key == 38, pal);
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
        let tip = if self.expand_on_hover { SETTINGS_EXPAND_TIP_H } else { 0.0 };

        // behavior toggles (first 5) — plain rows, not reorderable
        for (i, &(key, glyph, label, on)) in toggles.iter().take(5).enumerate() {
            let y = 196.0 + so + i as f32 * 40.0 + (if i == 0 { 0.0 } else { tip });
            Self::settings_toggle_row(v, pal, cx + 12.0, y, w - cx - 24.0, glyph, label, on, self.hover_key == key);
            self.region(cx + 4.0, y - 2.0, w - cx - 8.0, 36.0, key);
        }

        // ── "Expand on hover" info tip: with hover-expand ON, clickable
        // items on the resting pill are discouraged (hover opens them anyway)
        if tip > 0.0 {
            let ty = 236.0 + so; // the slot right under the first toggle row
            let bxx = cx + 12.0;
            let bww = w - cx - 24.0;
            v.push(Cmd::Rect { x: bxx, y: ty, w: bww, h: SETTINGS_EXPAND_TIP_H, r: 10.0, color: ui::acc_tint(&pal) });
            ui::text(v, bxx + 12.0, ty + 14.0, ICON_INFO, 14.0, pal.acc, true);
            ui::text(v, bxx + 36.0, ty + 7.0, "Expand on hover is ON", 10.5, pal.fg, false);
            ui::text(v, bxx + 36.0, ty + 24.0, "Adding clickable items to the resting pill is discouraged.", 10.5, ui::fg2(&pal), false);
        }

        // reorderable pill items (last 16) — tap to toggle, drag to reorder
        ui::text(v, cx + 8.0, 404.0 + so + tip, "PILL ITEMS · hold & drag to reorder", 9.5, ui::fg3(&pal), false);
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
            let this_dragging = self.pill_drag.as_ref().map_or(false, |d| {
                Self::settings_key_to_pill(key).map_or(false, |p| {
                    self.pill_order.iter().position(|&x| x == p) == Some(d.from_idx)
                })
            }) && lifted.is_some();
            if this_dragging {
                v.push(Cmd::Rect { x: cx + 12.0, y, w: w - cx - 24.0, h: 36.0, r: 10.0, color: (pal.bg & 0xffffff00) | 0x30 });
                continue;
            }
            let grip_hov = self.hover_key == key;
            let grip_color = if grip_hov { pal.fg } else { ui::fg3(&pal) };
            ui::text(v, cx + 2.0, y + 10.0, ICON_GRIP, 14.0, grip_color, true);
            Self::settings_toggle_row(v, pal, cx + 12.0, y, w - cx - 24.0, glyph, label, on, self.hover_key == key);
            self.region(cx + 4.0, y - 2.0, w - cx - 8.0, 36.0, key);
        }
        // the dragged row floats under the cursor
        if let Some((dx_, dy_)) = lifted {
            let found = pill_item_toggles.iter().find(|&&(key, ..)| {
                self.pill_drag.as_ref().map_or(false, |d| {
                    Self::settings_key_to_pill(key).map_or(false, |p| {
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
        Self::settings_toggle_row(v, pal, cx + 12.0, bp_y, w - cx - 24.0, ICON_BATTERY, "Battery %", self.pill_battery_pct, self.hover_key == 39);
        self.region(cx + 4.0, bp_y - 2.0, w - cx - 8.0, 36.0, 39);
        // Position row: four edge chips choose where the bar sits (left/top/
        // right/bottom). The active edge is accented; clicking re-anchors.
        let py = bp_y + 46.0;
        v.push(Cmd::Rect { x: cx + 24.0, y: py + 8.0, w: 30.0, h: 30.0, r: 9.0, color: mix(ui::hover(&pal), pal.fg, 0.20) });
        ui::text(v, cx + 31.0, py + 15.0, ICON_SQUARE, 14.0, ui::fg2(&pal), true);
        ui::text(v, cx + 64.0, py + 15.0, "Position", 13.0, pal.fg, false);
        let chips: [(u32, &str, crate::shell::BarEdge); 4] = [
            (SYS_EDGE_L_KEY, ICON_BACK, crate::shell::BarEdge::Left),
            (SYS_EDGE_T_KEY, ICON_ANCHOR_UP, crate::shell::BarEdge::Top),
            (SYS_EDGE_R_KEY, ICON_FORWARD, crate::shell::BarEdge::Right),
            (SYS_EDGE_B_KEY, ICON_ANCHOR_DOWN, crate::shell::BarEdge::Bottom),
        ];
        let (cw, gap, ch) = (34.0, 6.0, 30.0);
        let mut cx0 = cx + 140.0;
        for (key, glyph, edge) in chips {
            let active = self.bar_edge == edge;
            let bg = if active {
                (pal.acc & 0xffff_ff00) | 0x3e
            } else {
                self.hl(key, ui::hover(&pal), ui::hover_hl(&pal))
            };
            v.push(Cmd::Rect { x: cx0, y: py + 8.0, w: cw, h: ch, r: ch / 2.0, color: bg });
            ui::text_c(v, cx0 + cw / 2.0, py + 8.0 + (ch - 12.0) / 2.0 + 1.0, glyph, 12.0, if active { pal.acc } else { pal.fg }, true);
            self.region(cx0, py + 8.0, cw, ch, key);
            cx0 += cw + gap;
        }
    }

    // ── System ──
    pub(crate) fn layout_settings_system(&mut self, v: &mut Vec<Cmd>, w: f32, cx: f32, h: f32, pal: &Pal) {
        ui::text(v, cx + 8.0, 50.0, "SYSTEM", 9.5, ui::fg3(&pal), false);
        ui::card(v, cx + 8.0, 58.0, w - cx - 16.0, 58.0, 14.0, ui::raised(&pal));
        ui::outline(v, cx + 8.0, 58.0, w - cx - 16.0, 58.0, 14.0, ui::hairline(&pal));
        let rows: [(String, i32); 3] = [
            ("CPU".into(), self.cpu),
            ("RAM".into(), self.mem_pct),
            ("DISK".into(), self.disk_pct),
        ];
        for (i, (label, pct)) in rows.iter().enumerate() {
            let y = 66.0 + i as f32 * 14.0;
            ui::text(v, cx + 20.0, y, label.clone(), 9.0, pal.fg, false);
            ui::bar(v, cx + 62.0, y + 1.0, w - cx - 62.0 - 96.0, 5.0, *pct as f32 / 100.0, pal.acc, ui::hover(&pal));
            ui::text_r(v, w - 24.0, y, format!("{pct}%"), 9.0, ui::fg3(&pal), false);
        }
        if !self.net_up.is_empty() || !self.net_down.is_empty() {
            ui::text_r(v, w - 16.0, 122.0, format!("↓ {} · ↑ {}", self.net_down, self.net_up), 9.0, ui::fg3(&pal), false);
        }
        ui::text_r(v, w - 16.0, h - 20.0, "zen-shell v0.1.0", 9.0, ui::fg3(&pal), false);
    }

    // ── Misc ──
    pub(crate) fn layout_settings_misc(&mut self, v: &mut Vec<Cmd>, w: f32, cx: f32, pal: &Pal) {
        ui::text(v, cx + 8.0, 50.0, "MISC", 9.5, ui::fg3(&pal), false);

        // ── Power menu hold delay slider (0.5..3.0 s, default 1.5 s) ──
        let key = 220u32;
        let hov = self.hover_key == key;
        let icbg = mix(ui::hover(&pal), pal.acc, 0.20);
        v.push(Cmd::Rect { x: cx + 24.0, y: 64.0, w: 30.0, h: 30.0, r: 9.0, color: icbg });
        ui::text(v, cx + 31.0, 71.0, ICON_TIMER, 14.0, pal.acc, true);
        ui::text(v, cx + 64.0, 64.0, "Power menu hold delay", 13.0, pal.fg, false);
        ui::text(v, cx + 64.0, 82.0, &format!("{:.1}s", self.power_hold_delay), 10.5, ui::fg3(&pal), false);
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
            v.push(Cmd::Rect { x, y, w: row_w, h: 40.0, r: 8.0, color: ui::hover_hl(&pal) });
        }
        // flat icon tile — quiet when off, accented when on
        let ic = if on { pal.acc } else { ui::fg3(&pal) };
        v.push(Cmd::Rect { x: x + 8.0, y: y + 8.0, w: 24.0, h: 24.0, r: 6.0, color: ui::hover(&pal) });
        ui::text(v, x + 14.0, y + 14.0, glyph, 12.0, ic, true);
        ui::text(v, x + 44.0, y + 14.0, label, 12.5, if hovered { pal.fg } else { ui::fg2(&pal) }, false);
        // slim 36×18 switch
        let tx = x + row_w - 12.0 - 36.0;
        v.push(Cmd::Rect {
            x: tx,
            y: y + 11.0,
            w: 36.0,
            h: 18.0,
            r: 9.0,
            color: if on { pal.acc } else { mix(pal.bg, pal.fg, 0.14) },
        });
        v.push(Cmd::Rect {
            x: if on { tx + 36.0 - 16.0 } else { tx + 2.0 },
            y: y + 13.0,
            w: 14.0,
            h: 14.0,
            r: 7.0,
            color: if on { 0xffffffff } else { ui::fg3(&pal) },
        });
    }

    /// Android-style settings row: tinted icon square + label (+ optional
    /// summary) on the left, toggle switch on the right, full-row highlight on
    /// hover. Returns the switch's x (for the caller's split hit regions).
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
            v.push(Cmd::Rect { x, y, w: row_w, h: 48.0, r: 8.0, color: ui::hover_hl(&pal) });
        }
        // flat icon tile — quiet when off, accented when on
        let ic = if on { pal.acc } else { ui::fg3(&pal) };
        v.push(Cmd::Rect { x: x + 8.0, y: y + 9.0, w: 30.0, h: 30.0, r: 7.0, color: ui::hover(&pal) });
        ui::text(v, x + 15.0, y + 17.0, glyph, 14.0, ic, true);
        let ly = if summary.is_some() { y + 9.0 } else { y + 16.0 };
        ui::text(v, x + 48.0, ly, label, 13.0, pal.fg, false);
        if let Some(s) = summary {
            ui::text(v, x + 48.0, y + 27.0, s, 10.5, ui::fg3(&pal), false);
        }
        // slim 38×20 switch
        let tx = x + row_w - 16.0 - 38.0;
        v.push(Cmd::Rect {
            x: tx,
            y: y + 14.0,
            w: 38.0,
            h: 20.0,
            r: 10.0,
            color: if on { pal.acc } else { mix(pal.bg, pal.fg, 0.14) },
        });
        v.push(Cmd::Rect {
            x: if on { tx + 38.0 - 17.0 } else { tx + 2.0 },
            y: y + 16.0,
            w: 16.0,
            h: 16.0,
            r: 8.0,
            color: if on { 0xffffffff } else { ui::fg3(&pal) },
        });
        tx
    }
}
