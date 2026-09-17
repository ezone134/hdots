//! System panels: control center, notifications, power, wifi, bluetooth, audio.

use super::*;

impl Shell {
        pub(crate) fn layout_cc(&mut self, v: &mut Vec<Cmd>, w: f32, h: f32, pal: &Pal) {
        // header
        ui::text(v, self.scale.s(16.0), self.scale.s(14.0), "Control Center", self.scale.fs(17.0), pal.fg, false);
        if self.battery >= 0 {
            let c = if self.battery <= 20 { RED } else { pal.acc };
            let wline = if self.battery_watts > 0.0 {
                format!("{}% · {:.1}W", self.battery, self.battery_watts)
            } else {
                format!("{}%", self.battery)
            };
            ui::text_r(v, w - self.scale.s(96.0), self.scale.s(16.0), wline, self.scale.fs(12.0), c, false);
        }
        if self.hover_key == 1 {
            v.push(Cmd::Rect { x: w - self.scale.s(40.0), y: self.scale.s(6.0), w: self.scale.s(28.0), h: self.scale.s(28.0), r: self.scale.s(14.0), color: ui::hover(&pal) });
        }
        ui::text_r(v, w - self.scale.s(28.0), self.scale.s(14.0), "✕", self.scale.fs(13.0), pal.fg, false);
        self.region(w - self.scale.s(40.0), self.scale.s(4.0), self.scale.s(36.0), self.scale.s(32.0), 1);
        // quick tiles
        let ssid = if self.wifi_on && !self.ssid.is_empty() {
            self.ssid.clone()
        } else if self.wifi_on {
            "Not Connected".to_string()
        } else {
            "Off".to_string()
        };
        let bt = if self.bt_on { "On" } else { "Off" };
        // > chevron opens the full menu for the tile
        fn chevron(v: &mut Vec<Cmd>, x: f32, y: f32, hover: bool, pal: &Pal, scale: &crate::shell::GridScale) {
            ui::text(v, x, y, ICON_MORE, scale.fs(16.0), if hover { pal.acc } else { ui::fg3(&pal) }, true);
        }
        ui::tile(v, self.scale.s(16.0), self.scale.s(44.0), self.scale.s(182.0), self.scale.s(76.0), ICON_WIFI, "Wi-Fi", &ssid, self.wifi_on, true, self.hover_key == 2, pal);
        self.region(self.scale.s(16.0), self.scale.s(44.0), self.scale.s(150.0), self.scale.s(76.0), 2);
        chevron(v, self.scale.s(148.0), self.scale.s(74.0), self.hover_key == 6, pal, &self.scale);
        self.region(self.scale.s(166.0), self.scale.s(44.0), self.scale.s(32.0), self.scale.s(76.0), 6);
        let bw = (w - self.scale.s(214.0)).max(self.scale.s(16.0));
        ui::tile(v, self.scale.s(198.0), self.scale.s(44.0), bw, self.scale.s(76.0), ICON_BLUETOOTH, "Bluetooth", bt, self.bt_on, false, self.hover_key == 3, pal);
        self.region(self.scale.s(198.0), self.scale.s(44.0), (bw - self.scale.s(32.0)).max(self.scale.s(16.0)), self.scale.s(76.0), 3);
        chevron(v, self.scale.s(198.0) + bw - self.scale.s(26.0), self.scale.s(74.0), self.hover_key == 7, pal, &self.scale);
        self.region(self.scale.s(198.0) + bw - self.scale.s(32.0), self.scale.s(44.0), self.scale.s(32.0), self.scale.s(76.0), 7);
        // media card
        ui::card(v, self.scale.s(16.0), self.scale.s(136.0), w - self.scale.s(32.0), self.scale.s(72.0), self.scale.s(16.0), ui::raised(&pal));
        ui::outline(v, self.scale.s(16.0), self.scale.s(136.0), w - self.scale.s(32.0), self.scale.s(72.0), self.scale.s(16.0), ui::hairline(&pal));
        let (mtitle, martist) = if self.media_title.is_empty() {
            ("Nothing playing".to_string(), String::new())
        } else {
            (self.media_title.clone(), self.media_artist.clone())
        };
        ui::text(v, self.scale.s(32.0), self.scale.s(138.0), mtitle, self.scale.fs(12.0), pal.fg, false);
        if !martist.is_empty() {
            ui::text(v, self.scale.s(32.0), self.scale.s(152.0), martist, self.scale.fs(10.0), ui::fg3(&pal), false);
        }
        let cx = w / 2.0 - self.scale.s(45.0);
        let cy = self.scale.s(172.0);
        let play_glyph = if self.media_playing { ICON_PAUSE } else { ICON_PLAY };
        ui::media_btn(v, cx, cy, ICON_PREV, self.scale.fs(15.0), self.hover_key == 20, pal.fg, pal);
        ui::media_btn(v, cx + self.scale.s(40.0), cy, play_glyph, self.scale.fs(17.0), self.hover_key == 21, pal.acc, pal);
        ui::media_btn(v, cx + self.scale.s(84.0), cy, ICON_PAUSE, self.scale.fs(15.0), self.hover_key == 22, pal.fg, pal);
        self.region(cx - self.scale.s(12.5), cy - self.scale.s(12.5), self.scale.s(40.0), self.scale.s(40.0), 20);
        self.region(cx + self.scale.s(27.5), cy - self.scale.s(11.5), self.scale.s(40.0), self.scale.s(40.0), 21);
        self.region(cx + self.scale.s(71.5), cy - self.scale.s(12.5), self.scale.s(40.0), self.scale.s(40.0), 22);
        // sliders
        ui::slider(v, self.scale.s(16.0), self.scale.s(222.0), w - self.scale.s(32.0), self.scale.s(10.0), self.volume, self.hover_key == 4, pal);
        self.region(self.scale.s(10.0), self.scale.s(214.0), w - self.scale.s(20.0), self.scale.s(26.0), 4);
        ui::text(v, self.scale.s(16.0), self.scale.s(240.0), format!("{} {:.0}%", ICON_VOLUME, self.volume * 100.0), self.scale.fs(11.0), pal.fg, true);
        ui::slider(v, self.scale.s(16.0), self.scale.s(254.0), w - self.scale.s(32.0), self.scale.s(10.0), self.brightness, self.hover_key == 5, pal);
        self.region(self.scale.s(10.0), self.scale.s(246.0), w - self.scale.s(20.0), self.scale.s(26.0), 5);
        ui::text_r(v, w - self.scale.s(16.0), self.scale.s(272.0), format!("{} {:.0}%", ICON_BRIGHTNESS, self.brightness * 100.0), self.scale.fs(11.0), pal.fg, true);
        // audio + wallpaper tiles → open their panels
        let tw = (w - self.scale.s(44.0)) / 2.0;
        ui::tile(v, self.scale.s(16.0), self.scale.s(300.0), tw, self.scale.s(66.0), ICON_VOLUME, "Audio", "per-app volume", false, false, self.hover_key == 8, pal);
        self.region(self.scale.s(16.0), self.scale.s(300.0), tw, self.scale.s(66.0), 8);
        ui::tile(v, self.scale.s(28.0) + tw, self.scale.s(300.0), tw, self.scale.s(66.0), ICON_WALLPAPER, "Wallpaper", "click to set", false, false, self.hover_key == 9, pal);
        self.region(self.scale.s(28.0) + tw, self.scale.s(300.0), tw, self.scale.s(66.0), 9);
        let _ = h;
    }

        pub(crate) fn layout_notifs(&mut self, v: &mut Vec<Cmd>, w: f32, h: f32, pal: &Pal) {
        // A declared `notif` surface in `shell.ron` owns the whole scene: the
        // header chrome (title + Clear chip + dynamic DND chip) is declarative
        // and the card list body is handed to `layout_notifs_body` via its
        // `notif_list` Ink. Without a surface the built-in Rust layout below
        // runs unchanged.
        let vals = self.scene_values();
        if self.draw_shell_surface("notif", v, w, h, pal, &vals, None) {
            return;
        }
        ui::text(v, self.scale.s(16.0), self.scale.s(12.0), "Notifications", self.scale.fs(15.0), pal.fg, false);
        // Clear chip
        v.push(Cmd::Rect {
            x: w - self.scale.s(196.0),
            y: self.scale.s(10.0),
            w: self.scale.s(84.0),
            h: self.scale.s(30.0),
            r: self.scale.s(15.0),
            color: if self.hover_key == 2 { ui::raised_hl(&pal) } else { ui::raised(&pal) },
        });
        ui::outline(v, w - self.scale.s(196.0), self.scale.s(10.0), self.scale.s(84.0), self.scale.s(30.0), self.scale.s(15.0), ui::hairline(&pal));
        ui::text(v, w - self.scale.s(190.0), self.scale.s(17.0), "Clear", self.scale.fs(11.0), pal.fg, false);
        self.region(w - self.scale.s(196.0), self.scale.s(10.0), self.scale.s(84.0), self.scale.s(30.0), 2);
        // DND chip
        let dh = self.hover_key == 3;
        let dbg = if self.dnd {
            if dh {
                mix(pal.acc, pal.fg, 0.15)
            } else {
                pal.acc
            }
        } else if dh {
            ui::hover_hl(&pal)
        } else {
            ui::hover(&pal)
        };
        v.push(Cmd::Rect { x: w - self.scale.s(104.0), y: self.scale.s(10.0), w: self.scale.s(88.0), h: self.scale.s(30.0), r: self.scale.s(15.0), color: dbg });
        ui::text(v, w - self.scale.s(96.0), self.scale.s(17.0), format!("{} DND", ICON_SNOW), self.scale.fs(11.0), if self.dnd { pal.sfg } else { pal.fg }, true);
        self.region(w - self.scale.s(104.0), self.scale.s(10.0), self.scale.s(88.0), self.scale.s(30.0), 3);
        self.layout_notifs_body(v, w, h, pal);
    }

    /// Body painter for a declared `notif` surface: the notification cards
    /// (newest first, urgency-aware accent strip + hover) + empty state only —
    /// the title / Clear / DND chips are declared chrome in `shell.ron`. Also
    /// the tail half of the fallback `layout_notifs`.
    pub(crate) fn layout_notifs_body(&mut self, v: &mut Vec<Cmd>, w: f32, _h: f32, pal: &Pal) {
        // cards (newest first), urgency-aware accent strip + hover
        let n = self.notifs.len().min(8);
        for i in 0..n {
            let key = 10 + i as u32;
            let ry = self.scale.s(52.0) + i as f32 * self.scale.s(52.0);
            let notif = &self.notifs[self.notifs.len() - 1 - i];
            v.push(Cmd::Rect {
                x: self.scale.s(16.0),
                y: ry,
                w: w - self.scale.s(32.0),
                h: self.scale.s(46.0),
                r: self.scale.s(12.0),
                color: self.hl(key, ui::raised(&pal), ui::raised_hl(&pal)),
            });
            // urgency-based strip color: low → accent, normal → accent, critical → red
            let strip = match notif.urgency {
                0 => pal.acc,
                2 => RED,
                _ => pal.acc,
            };
            v.push(Cmd::Rect { x: self.scale.s(20.0), y: ry + self.scale.s(10.0), w: self.scale.s(4.0), h: self.scale.s(26.0), r: self.scale.s(2.0), color: strip });
            if let Some(ik) = notif.icon_key() {
                v.push(Cmd::Image { x: self.scale.s(32.0), y: ry + self.scale.s(4.0), w: self.scale.s(20.0), h: self.scale.s(20.0), key: ik });
            }
            ui::text(v, self.scale.s(60.0), ry + self.scale.s(6.0), if notif.app.is_empty() { "unknown" } else { &notif.app }, self.scale.fs(10.0), ui::fg3(&pal), false);
            ui::text(v, self.scale.s(60.0), ry + self.scale.s(18.0), if notif.summary.is_empty() { "Notification" } else { &notif.summary }, self.scale.fs(13.0), pal.fg, false);
            if !notif.body.is_empty() {
                let body: String = notif.body.chars().take(44).collect();
                ui::text(v, self.scale.s(60.0), ry + self.scale.s(32.0), body, self.scale.fs(11.0), ui::fg2(&pal), false);
            }
            self.region(self.scale.s(16.0), ry, w - self.scale.s(32.0), self.scale.s(46.0), key);
        }
        if n == 0 {
            ui::text(v, self.scale.s(24.0), self.scale.s(70.0), "No notifications", self.scale.fs(12.0), ui::fg3(&pal), false);
        }
    }

        pub(crate) fn layout_power(&mut self, v: &mut Vec<Cmd>, w: f32, h: f32, pal: &Pal) {
        ui::text(v, self.scale.s(22.0), self.scale.s(16.0), "Power", self.scale.fs(15.0), pal.fg, false);
        let items: [(&str, &str, bool); 5] = [
            (ICON_LOCK, "Lock", false),
            (ICON_LOGOUT, "Logout", false),
            (ICON_MOON, "Sleep", false),
            (ICON_REFRESH, "Restart", false),
            (ICON_POWER, "Shutdown", true),
        ];
        // square tiles with rounded edges, centered vertically
        let bw = self.scale.s(92.0);
        let bh = self.scale.s(92.0);
        let gap = self.scale.s(14.0);
        let n = items.len() as f32;
        let total = n * bw + (n - 1.0) * gap;
        let mut bx = (w - total) / 2.0;
        let by = (h - bh) / 2.0;
        for (i, &(glyph, label, danger)) in items.iter().enumerate() {
            let key = 1 + i as u32;
            let hov = self.hover_key == key;
            let holding = self.power_hold == Some(key);
            v.push(Cmd::Rect {
                x: bx,
                y: by,
                w: bw,
                h: bh,
                r: self.scale.s(18.0),
                color: if holding {
                    ui::acc_tint(&pal)
                } else if hov {
                    ui::raised_hl(&pal)
                } else {
                    ui::raised(&pal)
                },
            });
            // liquid fill: rises from the bottom over the hold delay
            if holding {
                let fill = self.power_hold_fill();
                let fh = bh * fill;
                if fh >= self.scale.s(1.5) {
                    v.push(Cmd::Rect {
                        x: bx + self.scale.s(2.0),
                        y: by + bh - self.scale.s(2.0) - fh,
                        w: bw - self.scale.s(4.0),
                        h: fh,
                        r: self.scale.s(16.0),
                        color: mix(ui::hover(&pal), pal.acc, 0.5),
                    });
                }
            }
            let gc = if danger { RED } else { pal.fg };
            ui::text(v, bx + (bw - self.scale.s(24.0)) / 2.0, by + self.scale.s(24.0), glyph, self.scale.fs(24.0), gc, true);
            ui::text_c(v, bx + bw / 2.0, by + bh - self.scale.s(32.0), label, self.scale.fs(12.0), ui::fg2(&pal), false);
            self.region(bx, by, bw, bh, key);
            bx += bw + gap;
        }
    }

        pub(crate) fn layout_wifi_menu(&mut self, v: &mut Vec<Cmd>, w: f32, h: f32, pal: &Pal) {
        // header: back + title + on/off state
        ui::text(v, self.scale.s(22.0), self.scale.s(16.0), ICON_BACK, self.scale.fs(14.0), pal.fg, true);
        self.region(self.scale.s(12.0), self.scale.s(8.0), self.scale.s(28.0), self.scale.s(28.0), 1);
        ui::text(v, self.scale.s(48.0), self.scale.s(16.0), "Wi-Fi Networks", self.scale.fs(15.0), pal.fg, false);
        ui::text_r(
            v,
            w - self.scale.s(16.0),
            self.scale.s(16.0),
            if self.wifi_on { "On" } else { "Off" },
            self.scale.fs(12.0),
            if self.wifi_on { pal.acc } else { ui::fg3(&pal) },
            false,
        );
        // network rows
        let mut y = self.scale.s(56.0);
        if self.wifi_networks.is_empty() {
            ui::text(v, self.scale.s(24.0), y, if self.wifi_on { "Scanning…" } else { "Wi-Fi is off" }, self.scale.fs(12.0), ui::fg3(&pal), false);
        }
        let nets: Vec<(String, u8, bool, bool)> = self.wifi_networks.clone();
        for (i, (ssid, strength, secured, connected)) in nets.iter().enumerate() {
            if i >= 18 {
                break;
            }
            let key = 10 + i as u32;
            let hov = self.hover_key == key;
            v.push(Cmd::Rect {
                x: self.scale.s(16.0),
                y,
                w: w - self.scale.s(32.0),
                h: self.scale.s(36.0),
                r: self.scale.s(12.0),
                color: if hov {
                    ui::raised_hl(&pal)
                } else if *connected {
                    ui::acc_tint(&pal)
                } else {
                    ui::raised(&pal)
                },
            });
            let lock = if *secured { format!("{} ", ICON_LOCK) } else { String::new() };
            ui::text(v, self.scale.s(30.0), y + self.scale.s(10.0), format!("{lock}{ssid}"), self.scale.fs(12.0), pal.fg, false);
            // strength bars + connected checkmark
            let bars = (*strength / 25).clamp(0, 4) as usize;
            for b in 0..4 {
                let bh = self.scale.s(4.0) + b as f32 * self.scale.s(3.0);
                let c = if b < bars { pal.acc } else { ui::fg3(&pal) };
                v.push(Cmd::Rect {
                    x: w - self.scale.s(72.0) + b as f32 * self.scale.s(7.0),
                    y: y + self.scale.s(18.0) - bh / 2.0,
                    w: self.scale.s(5.0),
                    h: bh,
                    r: self.scale.s(2.0),
                    color: c,
                });
            }
            if *connected {
                ui::text_r(v, w - self.scale.s(28.0), y + self.scale.s(9.0), ICON_CHECK, self.scale.fs(13.0), pal.acc, true);
            }
            self.region(self.scale.s(16.0), y, w - self.scale.s(32.0), self.scale.s(36.0), key);
            y += self.scale.s(42.0);
        }
        let _ = h;
    }

        pub(crate) fn layout_bt_menu(&mut self, v: &mut Vec<Cmd>, w: f32, h: f32, pal: &Pal) {
        // header: back + title + on/off state
        ui::text(v, self.scale.s(22.0), self.scale.s(16.0), ICON_BACK, self.scale.fs(14.0), pal.fg, true);
        self.region(self.scale.s(12.0), self.scale.s(8.0), self.scale.s(28.0), self.scale.s(28.0), 1);
        ui::text(v, self.scale.s(48.0), self.scale.s(16.0), "Bluetooth", self.scale.fs(15.0), pal.fg, false);
        ui::text_r(
            v,
            w - self.scale.s(16.0),
            self.scale.s(16.0),
            if self.bt_on { "On" } else { "Off" },
            self.scale.fs(12.0),
            if self.bt_on { pal.acc } else { ui::fg3(&pal) },
            false,
        );
        // device rows
        let mut y = self.scale.s(56.0);
        if self.bt_devices.is_empty() {
            ui::text(v, self.scale.s(24.0), y, if self.bt_on { "No devices found" } else { "Bluetooth is off" }, self.scale.fs(12.0), ui::fg3(&pal), false);
        }
        let devs: Vec<(String, bool, bool)> = self.bt_devices.clone();
        for (i, (name, connected, _paired)) in devs.iter().enumerate() {
            if i >= 18 {
                break;
            }
            let key = 10 + i as u32;
            let hov = self.hover_key == key;
            v.push(Cmd::Rect {
                x: self.scale.s(16.0),
                y,
                w: w - self.scale.s(32.0),
                h: self.scale.s(36.0),
                r: self.scale.s(12.0),
                color: if hov {
                    ui::raised_hl(&pal)
                } else if *connected {
                    ui::acc_tint(&pal)
                } else {
                    ui::raised(&pal)
                },
            });
            ui::text(v, self.scale.s(30.0), y + self.scale.s(10.0), name, self.scale.fs(12.0), pal.fg, false);
            ui::text_r(
                v,
                w - self.scale.s(28.0),
                y + self.scale.s(10.0),
                if *connected { "Connected" } else { "Tap to connect" },
                self.scale.fs(11.0),
                if *connected { pal.acc } else { ui::fg3(&pal) },
                false,
            );
            self.region(self.scale.s(16.0), y, w - self.scale.s(32.0), self.scale.s(36.0), key);
            y += self.scale.s(42.0);
        }
        let _ = h;
    }

    /// Flat list of audio rows in display order: output devices, input
    /// devices, then apps. Tuple = (wpctl id, name, volume 0-1, muted,
    /// is_input, is_app) — shared by the layout and the click handler so the
    /// slider/mute hit-regions always match what is drawn.
    pub(crate) fn audio_rows(&self) -> Vec<(u32, String, f32, bool, bool, bool)> {
        let mut r = Vec::new();
        for (id, name, vol, muted) in &self.audio_sinks {
            r.push((*id, name.clone(), *vol, *muted, false, false));
        }
        for (id, name, vol, muted) in &self.audio_sources {
            r.push((*id, name.clone(), *vol, *muted, true, false));
        }
        for (id, name, vol, muted, is_input) in &self.audio_streams {
            r.push((*id, name.clone(), *vol, *muted, *is_input, true));
        }
        r
    }

    /// Re-list the cached wallpaper thumbnails — each cache file in
    /// `$HOME/.cache/thumbnails/zen-shell` is named after the wallpaper's
    /// basename (spaces and all), exactly like the main picker. Only `.tmp`
    /// files from an interrupted thumbnail write are skipped. Called when the
    /// picker opens (and the warm-up thread topped the cache up at boot).
    pub(crate) fn refresh_wallpapers(&mut self) {
        self.wallpapers.clear();
        let cdir = crate::img::thumbnail_cache_dir();
        let Ok(rd) = std::fs::read_dir(&cdir) else { return };
        let mut v: Vec<String> = Vec::new();
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if e.path().is_file() && !name.ends_with(".tmp") {
                v.push(name);
            }
        }
        v.sort();
        self.wallpapers = v;
        self.wallpaper_scroll = self.wallpaper_scroll.min(self.wallpaper_max_scroll());
    }

        pub(crate) fn layout_audio(&mut self, v: &mut Vec<Cmd>, w: f32, h: f32, pal: &Pal) {
        // header: close + title
        ui::text(v, self.scale.s(22.0), self.scale.s(16.0), ICON_BACK, self.scale.fs(14.0), pal.fg, true);
        self.region(self.scale.s(12.0), self.scale.s(8.0), self.scale.s(28.0), self.scale.s(28.0), 1);
        ui::text(v, self.scale.s(48.0), self.scale.s(16.0), "Audio", self.scale.fs(15.0), pal.fg, false);
        ui::text_r(v, w - self.scale.s(16.0), self.scale.s(16.0), "per-app volume", self.scale.fs(10.0), ui::fg3(&pal), false);
        let rows = self.audio_rows();
        if rows.is_empty() {
            ui::text(v, self.scale.s(24.0), self.scale.s(64.0), "No audio devices or apps (needs WirePlumber's wpctl)", self.scale.fs(12.0), ui::fg3(&pal), false);
            return;
        }
        let rh = self.scale.s(46.0);
        let mut y = self.scale.s(46.0);
        let mut last_section = String::new();
        for (i, (_id, name, vol, muted, is_input, is_app)) in rows.iter().enumerate() {
            let key = 10 + i as u32;
            let mkey = 40 + i as u32;
            let section = if *is_app {
                "Apps"
            } else if *is_input {
                "Input devices"
            } else {
                "Output devices"
            };
            if section != last_section {
                ui::text(v, self.scale.s(24.0), y, section, self.scale.fs(10.0), ui::fg3(&pal), false);
                y += self.scale.s(20.0);
                last_section = section.to_string();
            }
            if y + rh > h - self.scale.s(6.0) {
                break;
            }
            let hov = self.hover_key == key;
            let mhov = self.hover_key == mkey;
            v.push(Cmd::Rect {
                x: self.scale.s(16.0),
                y,
                w: w - self.scale.s(32.0),
                h: rh,
                r: self.scale.s(10.0),
                color: if hov || mhov { ui::hover_hl(&pal) } else { ui::hover(&pal) },
            });
            let nm: String = name.chars().take(24).collect();
            ui::text(v, self.scale.s(28.0), y + self.scale.s(6.0), nm, self.scale.fs(11.0), pal.fg, false);
            // mute toggle (right)
            let mc = if *muted {
                pal.acc
            } else if mhov {
                pal.fg
            } else {
                ui::fg3(&pal)
            };
            ui::text(v, w - self.scale.s(48.0), y + self.scale.s(13.0), if *muted { ICON_MUTE } else { ICON_VOLUME }, self.scale.fs(14.0), mc, true);
            self.region(w - self.scale.s(54.0), y, self.scale.s(42.0), rh, mkey);
            // volume slider
            ui::slider(v, self.scale.s(44.0), y + self.scale.s(27.0), w - self.scale.s(44.0) - self.scale.s(66.0), self.scale.s(8.0), *vol, hov, pal);
            self.region(self.scale.s(40.0), y + self.scale.s(22.0), w - self.scale.s(40.0) - self.scale.s(62.0), self.scale.s(18.0), key);
            y += rh + self.scale.s(4.0);
        }
        let _ = h;
    }
}
