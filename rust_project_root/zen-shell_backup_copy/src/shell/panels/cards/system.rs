use super::super::*;

impl Shell {

        pub(crate) fn draw_system_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        // LEFT: username → hostname → clock → date → current color scheme
        // RIGHT: bell + darkmode (top) → uptime → battery → wattage (stacked below)
        let uname: String = self.username.chars().take(12).collect();
        ui::text(v, x + self.scale.s(16.0), y + self.scale.s(14.0), &uname, self.scale.fs(21.0), pal.fg, true);
        let hname: String = self.hostname.chars().take((w / 8.0).max(8.0) as usize).collect();
        ui::text(v, x + self.scale.s(17.0), y + self.scale.s(42.0), &hname, self.scale.fs(13.0), pal.fg, false);

        // clock (left column, below hostname)
        if !self.clock.is_empty() {
            ui::text_hero(v, x + self.scale.s(16.0), y + self.scale.s(62.0), self.clock.clone(), self.scale.fs(26.0), pal.fg);
        }
        // date chip (left column, below clock)
        {
            let dtext = self.date_str("%a, %d %b");
            let dw = dtext.chars().count() as f32 * self.scale.s(12.0) * 0.62 + self.scale.s(18.0);
            let dx = x + self.scale.s(14.0);
            let dy = y + self.scale.s(98.0);
            let hov = self.hover_key == 25;
            v.push(Cmd::Rect {
                x: dx, y: dy, w: dw, h: self.scale.s(22.0), r: self.scale.s(11.0),
                color: if hov { ui::hover_hl(&pal) } else { mix(ui::hover(&pal), pal.fg, 0.06) },
            });
            ui::text(v, dx + self.scale.s(9.0), dy + self.scale.s(5.0), dtext, self.scale.fs(12.0), if hov { pal.acc } else { pal.fg }, false);
            self.region(dx - self.scale.s(4.0), dy - self.scale.s(3.0), dw + self.scale.s(8.0), self.scale.s(28.0), 25);
        }

        // UI state line (left column, below the date chip). Refreshed by the 3 s
        // services poll while the dashboard is open — no expand hook.
        // (Color scheme + accent source now live on the Accent card.)
        if h >= self.scale.s(150.0) {
            if !self.cur_channel.is_empty() {
                let ctext = format!("UI State: {}", self.cur_channel);
                ui::text(v, x + self.scale.s(16.0), y + self.scale.s(124.0), &ctext, self.scale.fs(9.5), pal.fg, false);
            }
        }

        // ── RIGHT column: hidden-menu dots at top, bell+darkmode below, then
        // │ uptime / battery / wattage stacked downward ──
        let rx = x + w - self.scale.s(16.0);

        // visible (⋮) hidden-menu button — top-right corner. Two stacked dots
        // imply a hidden menu: Bar position (settings moved to the ">" corner).
        {
            let bx = x + w - self.scale.s(40.0);
            let by = y + self.scale.s(10.0);
            let hov = self.hover_key == SYS_MENU_KEY;
            let bg = if hov {
                ui::hover_hl(&pal)
            } else if self.sys_menu_open {
                (pal.acc & 0xffff_ff00) | 0x34
            } else {
                0
            };
            if hov || self.sys_menu_open {
                v.push(Cmd::Rect { x: bx, y: by, w: self.scale.s(26.0), h: self.scale.s(22.0), r: self.scale.s(11.0), color: bg });
            }
            let dc = if hov || self.sys_menu_open { pal.acc } else { ui::fg2(&pal) };
            let dx = bx + self.scale.s(11.4);
            v.push(Cmd::Rect { x: dx, y: by + self.scale.s(6.0), w: self.scale.s(3.2), h: self.scale.s(3.2), r: self.scale.s(1.6), color: dc });
            v.push(Cmd::Rect { x: dx, y: by + self.scale.s(12.8), w: self.scale.s(3.2), h: self.scale.s(3.2), r: self.scale.s(1.6), color: dc });
            self.region(bx - self.scale.s(3.0), by - self.scale.s(3.0), self.scale.s(32.0), self.scale.s(28.0), SYS_MENU_KEY);
        }

        // bell (below the ⋮ row — the "empty row above" the notif icon)
        {
            let bx = x + w - self.scale.s(40.0);
            let by = y + self.scale.s(40.0);
            let hov = self.hover_key == 24;
            if hov {
                v.push(Cmd::Rect { x: bx, y: by, w: self.scale.s(26.0), h: self.scale.s(22.0), r: self.scale.s(11.0), color: ui::hover_hl(&pal) });
            }
            let glyph = if self.dnd { ICON_DND } else { ICON_BELL };
            ui::text(v, bx + self.scale.s(7.0), by + self.scale.s(4.0), glyph, self.scale.fs(13.0), if hov { pal.acc } else { pal.fg }, true);
            if self.notif_count > 0 && !self.dnd {
                v.push(Cmd::Rect { x: bx + self.scale.s(17.0), y: by - self.scale.s(2.0), w: self.scale.s(7.0), h: self.scale.s(7.0), r: self.scale.s(3.5), color: pal.acc });
            }
            self.region(bx - self.scale.s(3.0), by - self.scale.s(3.0), self.scale.s(32.0), self.scale.s(28.0), 24);
        }
        // dark/light toggle (left of bell)
        {
            let bx = x + w - self.scale.s(72.0);
            let by = y + self.scale.s(40.0);
            let hov = self.hover_key == 26;
            if hov {
                v.push(Cmd::Rect { x: bx, y: by, w: self.scale.s(26.0), h: self.scale.s(22.0), r: self.scale.s(11.0), color: ui::hover_hl(&pal) });
            }
            let glyph = if self.dark_mode { ICON_MOON } else { ICON_BRIGHTNESS };
            ui::text(v, bx + self.scale.s(6.5), by + self.scale.s(4.0), glyph, self.scale.fs(13.0), if hov { pal.acc } else { pal.fg }, true);
            self.region(bx - self.scale.s(3.0), by - self.scale.s(3.0), self.scale.s(32.0), self.scale.s(28.0), 26);
        }
        // uptime (below bell/darkmode)
        {
            let uy = y + self.scale.s(72.0);
            ui::text_r(v, rx, uy, format!("Uptime {}", self.uptime), self.scale.fs(9.5), pal.fg, false);
        }
        // battery + wattage (below uptime)
        if self.battery >= 0 {
            let low = self.battery <= 20 && !self.ac_online;
            let bc = if low { RED } else { pal.fg };
            let ic = if low { RED } else { pal.fg };
            let by = y + self.scale.s(92.0);
            let icon_w = ui::battery_icon_w(self.scale.s(13.0));
            // vertical nerd-font battery glyph (right-aligned, no text overlapping)
            let glyph = ui::battery_glyph(self.battery, self.ac_online);
            ui::text_r(v, rx, by + self.scale.s(1.0), glyph, self.scale.s(13.0), ic, true);
            // battery % as separate text OUTSIDE the icon (left of it)
            if self.pill_battery_pct {
                let pct = format!("{}%", self.battery);
                ui::text_r(v, rx - icon_w - self.scale.s(4.0), by + self.scale.s(1.0) + self.scale.s(1.0), pct, self.scale.fs(10.5), ic, false);
            }
            // time remaining (below battery icon)
            if !self.battery_time.is_empty() {
                ui::text_r(v, rx, by + self.scale.s(18.0), &self.battery_time, self.scale.fs(9.5), bc, false);
            }
            // wattage (below time)
            if self.battery_watts > 0.0 {
                ui::text_r(v, rx, by + self.scale.s(32.0), format!("{:.1}W", self.battery_watts), self.scale.fs(9.0), pal.fg, false);
            }
        }

        // ── bottom-right corner: ">" chevron → jumps straight to Settings ──
        {
            let bx = x + w - self.scale.s(40.0);
            let by = y + h - self.scale.s(34.0);
            let hov = self.hover_key == SYS_MENU_SETTINGS_KEY;
            if hov {
                v.push(Cmd::Rect { x: bx, y: by, w: self.scale.s(26.0), h: self.scale.s(22.0), r: self.scale.s(11.0), color: ui::hover_hl(&pal) });
            }
            ui::text(v, bx + self.scale.s(7.0), by + self.scale.s(4.0), ICON_MORE, self.scale.fs(14.0), if hov { pal.acc } else { ui::fg2(&pal) }, true);
            self.region(bx - self.scale.s(3.0), by - self.scale.s(3.0), self.scale.s(32.0), self.scale.s(28.0), SYS_MENU_SETTINGS_KEY);
        }
    }

}
