use super::super::*;

impl Shell {

    /// CALENDAR — compact month with prev/next paging; clicking a day opens
    /// the full calendar panel with that day selected.
    pub(crate) fn draw_calendar_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        let (first_wday, days, today, month) = self.calendar_info();
        let hdr = 24.0;
        // header: date on the top-left, prev/next chevrons grouped top-right
        ui::text(v, x + self.scale.s(12.0), y + self.scale.s(4.0), month, self.scale.fs(11.0), pal.acc, false);
        let nav_w = self.scale.s(22.0);
        let nav_h = self.scale.s(20.0);
        let gap = self.scale.s(4.0);
        let py = y + (hdr - nav_h) / 2.0;
        let gx = x + w - self.scale.s(12.0) - nav_w * 2.0 - gap;
        let hov_prev = self.hover_key == crate::shell::CAL_KEY_BASE_PREV;
        let hov_next = self.hover_key == crate::shell::CAL_KEY_BASE_NEXT;
        if hov_prev {
            v.push(Cmd::Rect { x: gx, y: py, w: nav_w, h: nav_h, r: nav_h / 2.0, color: ui::hover_hl(&pal) });
        }
        ui::text_c(v, gx + nav_w / 2.0, py + self.scale.s(1.0), ICON_BACK, self.scale.fs(10.0), if hov_prev { pal.acc } else { pal.fg }, true);
        if hov_next {
            v.push(Cmd::Rect { x: gx + nav_w + gap, y: py, w: nav_w, h: nav_h, r: nav_h / 2.0, color: ui::hover_hl(&pal) });
        }
        ui::text_c(v, gx + nav_w + gap + nav_w / 2.0, py + self.scale.s(1.0), ICON_FORWARD, self.scale.fs(10.0), if hov_next { pal.acc } else { pal.fg }, true);
        self.region(gx - self.scale.s(2.0), py - self.scale.s(2.0), nav_w + self.scale.s(4.0), nav_h + self.scale.s(4.0), crate::shell::CAL_KEY_BASE_PREV);
        self.region(gx + nav_w + gap - self.scale.s(2.0), py - self.scale.s(2.0), nav_w + self.scale.s(4.0), nav_h + self.scale.s(4.0), crate::shell::CAL_KEY_BASE_NEXT);

        // fit 6 rows × 7 cols into the remaining card height
        let gy = y + hdr;
        let gh = (h - hdr).max(60.0);
        let cell_w = w / 7.0;
        let wd = ["S", "M", "T", "W", "T", "F", "S"];
        for (i, d) in wd.iter().enumerate() {
            ui::text_c(v, x + i as f32 * cell_w + cell_w / 2.0, gy + 2.0, *d, self.scale.fs(8.0), ui::fg3(&pal), false);
        }
        // weekday label row consumes the top of the grid
        let dy = gy + 16.0;
        let dcell_h = (gh - 16.0) / 6.0;
        for cell in 0..42 {
            let day = cell - first_wday + 1;
            if day < 1 || day > days {
                continue;
            }
            let cx = x + (cell % 7) as f32 * cell_w;
            let cy = dy + (cell / 7) as f32 * dcell_h;
            let key = crate::shell::CAL_KEY_BASE + cell as u32;
            let is_today = self.cal_offset == 0 && day == today;
            let hov = self.hover_key == key;
            if hov {
                v.push(Cmd::Rect { x: cx + 1.0, y: cy + 1.0, w: cell_w - 2.0, h: dcell_h - 2.0, r: self.scale.s(8.0), color: ui::hover(&pal) });
            }
            if is_today {
                v.push(Cmd::Rect { x: cx + 1.0, y: cy + 1.0, w: cell_w - 2.0, h: dcell_h - 2.0, r: self.scale.s(8.0), color: ui::acc_tint(&pal) });
            }
            let color = if is_today { pal.acc } else if hov { pal.fg } else { ui::fg2(&pal) };
            ui::text_c(v, cx + cell_w / 2.0, cy + dcell_h / 2.0 - self.scale.s(5.0), day.to_string(), self.scale.fs(9.0), color, false);
            self.region(cx, cy, cell_w, dcell_h, key);
        }
    }

}
