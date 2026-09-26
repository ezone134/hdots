//! Session surfaces: lockscreen and calendar panels.

use super::*;

impl Shell {
        pub(crate) fn layout_lock(&mut self, v: &mut Vec<Cmd>, w: f32, h: f32, pal: &Pal) {
        // dim backdrop
        v.push(Cmd::Rect { x: 0.0, y: 0.0, w, h, r: 0.0, color: 0x00000059 });
        // hero clock + date
        let cx = w / 2.0;
        if !self.clock.is_empty() {
            ui::text_c(v, cx, h * 0.20, self.clock.clone(), self.scale.fs(64.0), pal.fg, false);
        }
        ui::text_c(v, cx, h * 0.20 + self.scale.s(74.0), self.date_str("%A, %d %B"), self.scale.fs(16.0), ui::fg3(&pal), false);
        // avatar + username
        let user = crate::auth::current_user();
        let ay = h * 0.20 + self.scale.s(144.0);
        v.push(Cmd::Rect {
            x: cx - self.scale.s(26.0),
            y: ay - self.scale.s(26.0),
            w: self.scale.s(52.0),
            h: self.scale.s(52.0),
            r: self.scale.s(26.0),
            color: mix(ui::hover(&pal), pal.acc, 0.35),
        });
        let initial: String = user
            .chars()
            .next()
            .map(|c| c.to_uppercase().collect())
            .unwrap_or_else(|| "?".into());
        ui::text_c(v, cx, ay - self.scale.s(12.0), &initial, self.scale.fs(22.0), pal.fg, false);
        ui::text_c(v, cx, ay + self.scale.s(30.0), &user, self.scale.fs(15.0), ui::fg2(&pal), false);
        // password field
        let fw = self.scale.s(340.0);
        let fh = self.scale.s(58.0);
        let fx = (w - fw) / 2.0;
        let fy = (h * 0.45).max(ay + self.scale.s(86.0));
        v.push(Cmd::Rect {
            x: fx,
            y: fy,
            w: fw,
            h: fh,
            r: self.scale.s(16.0),
            color: if self.hover_key == 1 { ui::hover_hl(&pal) } else { ui::hover(&pal) },
        });
        let dots: String = if self.lock_pw.is_empty() {
            if self.lock_checking {
                "Verifying…".to_string()
            } else {
                "Enter password".to_string()
            }
        } else {
            format!("{}|", "\u{2022}".repeat(self.lock_pw.chars().count()))
        };
        ui::text_c(v, cx, fy + (fh - self.scale.s(14.0)) / 2.0, &dots, self.scale.fs(14.0), if self.lock_pw.is_empty() && !self.lock_checking { ui::fg3(&pal) } else if self.lock_checking { pal.acc } else { pal.fg }, false);
        self.region(fx, fy, fw, fh, 1);
        // auth error / caps-lock warning under the field
        let mut err_y = fy + fh + self.scale.s(22.0);
        if let Some(err) = &self.lock_error {
            ui::text_c(v, cx, err_y, err, self.scale.fs(13.0), RED, false);
            err_y += self.scale.s(22.0);
        }
        if self.caps {
            ui::text_c(v, cx, err_y, format!("{} CAPS LOCK IS ON", ICON_KEYBOARD), self.scale.fs(13.0), RED, true);
        }
        // buttons: logout / restart / shutdown (hold to confirm)
        let bw = self.scale.s(150.0);
        let bh = self.scale.s(52.0);
        let gap = self.scale.s(16.0);
        let total = 3.0 * bw + 2.0 * gap;
        let mut bx = (w - total) / 2.0;
        let by = h - self.scale.s(120.0);
        let btns: [(u32, &str, &str, bool); 3] = [
            (2, ICON_LOGOUT, "Logout", false),
            (3, ICON_REFRESH, "Restart", false),
            (4, ICON_POWER, "Shutdown", true),
        ];
        for (key, glyph, label, danger) in btns {
            let hov = self.hover_key == key;
            let holding = self.power_hold == Some(key);
            v.push(Cmd::Rect {
                x: bx,
                y: by,
                w: bw,
                h: bh,
                r: self.scale.s(14.0),
                color: if holding {
                    ui::acc_tint(&pal)
                } else if hov {
                    ui::hover_hl(&pal)
                } else {
                    ui::hover(&pal)
                },
            });
            if holding {
                let fh = bh * self.power_hold_fill();
                if fh >= self.scale.s(1.5) {
                    v.push(Cmd::Rect {
                        x: bx + self.scale.s(2.0),
                        y: by + bh - self.scale.s(2.0) - fh,
                        w: bw - self.scale.s(4.0),
                        h: fh,
                        r: self.scale.s(13.0),
                        color: mix(ui::hover(&pal), pal.acc, 0.5),
                    });
                }
            }
            let gc = if danger { RED } else { pal.fg };
            ui::text(v, bx + (bw - self.scale.s(18.0)) / 2.0, by + self.scale.s(10.0), glyph, self.scale.fs(18.0), gc, true);
            ui::text_c(v, bx + bw / 2.0, by + self.scale.s(32.0), label, self.scale.fs(12.0), ui::fg2(&pal), false);
            self.region(bx, by, bw, bh, key);
            bx += bw + gap;
        }
        ui::text_c(v, cx, by + bh + self.scale.s(22.0), &format!("type your password and press Enter to unlock · hold a button {:.0}s", self.power_hold_delay), self.scale.fs(10.0), ui::fg3(&pal), false);
    }

        pub(crate) fn layout_calendar(&mut self, v: &mut Vec<Cmd>, w: f32, _h: f32, pal: &Pal) {        let (first_wday, days, today, month) = self.calendar_info();
        // prev / next
        if self.hover_key == 1 {
            v.push(Cmd::Rect { x: 10.0, y: 5.0, w: 30.0, h: 30.0, r: 15.0, color: ui::hover(&pal) });
        }
        ui::text(v, 15.0, 8.0, ICON_BACK, 13.0, if self.hover_key == 1 { pal.acc } else { pal.fg }, true);
        self.region(10.0, 5.0, 30.0, 30.0, 1);
        if self.hover_key == 2 {
            v.push(Cmd::Rect { x: w - 40.0, y: 5.0, w: 30.0, h: 30.0, r: 15.0, color: ui::hover(&pal) });
        }
        ui::text_r(v, w - 12.0, 8.0, ICON_FORWARD, 13.0, if self.hover_key == 2 { pal.acc } else { pal.fg }, true);
        self.region(w - 40.0, 5.0, 30.0, 30.0, 2);
        ui::text_c(v, w / 2.0, 12.0, month, 15.0, pal.acc, false);
        // weekday header
        let wd = ["S", "M", "T", "W", "T", "F", "S"];
        for (i, d) in wd.iter().enumerate() {
            ui::text_c(v, 16.0 + i as f32 * 52.0 + 26.0, 48.0, *d, 10.0, ui::fg3(&pal), false);
        }
        // days: hover → tint, today → accent tint, selected → solid accent
        for cell in 0..42 {
            let cx = 16.0 + (cell % 7) as f32 * 52.0;
            let cy = 64.0 + (cell / 7) as f32 * 44.0;
            let day = cell - first_wday + 1;
            if day < 1 || day > days {
                continue;
            }
            let key = 10 + cell as u32;
            let is_today = self.cal_offset == 0 && day == today;
            let is_sel = day == self.cal_selected;
            let hov = self.hover_key == key;
            if hov {
                v.push(Cmd::Rect { x: cx + 1.0, y: cy + 1.0, w: 50.0, h: 42.0, r: 10.0, color: ui::hover(&pal) });
            }
            if is_sel {
                v.push(Cmd::Rect { x: cx + 1.0, y: cy + 1.0, w: 50.0, h: 42.0, r: 10.0, color: pal.acc });
            } else if is_today {
                v.push(Cmd::Rect { x: cx + 1.0, y: cy + 1.0, w: 50.0, h: 42.0, r: 10.0, color: ui::acc_tint(&pal) });
            }
            let color = if is_sel {
                pal.sfg
            } else if is_today {
                pal.acc
            } else if hov {
                pal.fg
            } else {
                ui::fg2(&pal)
            };
            ui::text_c(v, cx + 26.0, cy + 15.0, day.to_string(), 12.0, color, false);
            self.region(cx, cy, 52.0, 44.0, key);
        }
    }
}
