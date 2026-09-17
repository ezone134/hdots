use super::super::*;
use super::shared::*;

impl Shell {

    /// ACCENT — pick the accent source (from wallpaper / scheme default /
    /// custom accent). Mirrors the Settings Appearance block — defined-hex is
    /// merged into the custom-acc logic, so only three sources are offered.
    pub(crate) fn draw_accent_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        // header pinned to the card's TOP-LEFT (uniform card chrome) — was
        // centered mid-card with the content block
        card_header(v, pal, x, y, w, self.scale.s(12.0), "Theme & Accent", Some(ICON_STICKY), None, self.scale, self.card_show_title, self.card_show_glyph);
        // vertically center the content block in the remaining space below
        // the header; capped so the rows never clip the lower edge
        let top = ((h - self.scale.s(156.0)) * 0.5).clamp(self.scale.s(30.0), self.scale.s(48.0));
        let y0 = y + top;
        // scheme row: the first of the four list items, pitched identically to the
        // accent-source rows below it (uniform spacing, no big gap)
        let sname = if self.cur_scheme.is_empty() { "default".to_string() } else { self.cur_scheme.clone() };
        let ctext = format!("Scheme: {sname}");
        let chov = self.hover_key == 28;
        let scheme_y = y0 + self.scale.s(28.0);
        ui::text(v, x + self.scale.s(14.0), scheme_y + self.scale.s(6.5), ctext, self.scale.fs(10.0), if chov { pal.acc } else { pal.fg }, false);
        ui::text_r(v, x + w - self.scale.s(18.0), scheme_y + self.scale.s(5.5), ICON_FORWARD, self.scale.fs(11.0), ui::fg2(&pal), true);
        self.region(x + self.scale.s(10.0), scheme_y, w - self.scale.s(20.0), self.scale.s(30.0), 28);
        // accent source picker (below the scheme chip). Tapping the `>`
        // chevron on "Custom accent" opens the one-column custom-accents
        // subview with a `<` back button.
        let cur = self.current_acc_source();
        let cur_name = self.current_acc_name();
        let rh = self.scale.s(30.0);
        // scheme row sits at y0+28 with the same 30 px pitch → rows start y0+58
        let rows_top = y0 + self.scale.s(58.0);
        if self.accent_list_open {
            self.load_custom_accs();
            // back row: `<` + title
            let back_key = crate::shell::ACC_LIST_BACK_KEY;
            let back_hov = self.hover_key == back_key;
            ui::text(v, x + self.scale.s(14.0), rows_top, ICON_BACK, self.scale.fs(10.0), if back_hov { pal.acc } else { pal.fg }, true);
            ui::text(v, x + self.scale.s(28.0), rows_top, "Custom accents", self.scale.fs(10.0), pal.fg, false);
            ui::text_r(v, x + w - self.scale.s(18.0), rows_top, format!("{}", self.custom_accs.len()), self.scale.fs(9.0), ui::fg3(&pal), false);
            self.region(x + self.scale.s(10.0), rows_top - self.scale.s(2.0), w - self.scale.s(20.0), self.scale.s(30.0), back_key);
            // one-column paged list
            let ly = rows_top + self.scale.s(32.0);
            let lh = (y + h - self.scale.s(4.0) - ly).max(8.0);
            let row_h = self.scale.s(26.0);
            let visible = ((lh - self.scale.s(4.0)) / row_h).floor().max(1.0) as usize;
            let len = self.custom_accs.len();
            self.accent_list_scroll = self.accent_list_scroll.min(len.saturating_sub(visible));
            let top = self.accent_list_scroll;
            let x0 = x + self.scale.s(10.0);
            let ww = w - self.scale.s(20.0);
            for j in 0..visible {
                let i = top + j;
                let Some(a) = self.custom_accs.get(i) else { break };
                let rj = ly + j as f32 * row_h;
                let key = crate::shell::ACC_LIST_KEY_BASE + j as u32;
                let hov = self.hover_key == key;
                let sel = cur == "c" && !cur_name.is_empty() && a.name == cur_name;
                if sel {
                    // active item: short accent bar on the left edge (no row bg)
                    v.push(Cmd::Rect {
                        x: x0,
                        y: rj + row_h / 2.0 - self.scale.s(6.0),
                        w: self.scale.s(3.0),
                        h: self.scale.s(12.0),
                        r: self.scale.s(1.5),
                        color: pal.acc,
                    });
                }
                let shown: String = a.name.chars().take(28).collect();
                ui::text(v, x0 + self.scale.s(14.0), rj + self.scale.s(6.5), shown, self.scale.fs(9.5), if hov { pal.acc } else { pal.fg }, false);
                self.region(x0, rj, ww, row_h, key);
            }
            self.accent_list_rect = (x0, ly, ww, lh);
            if self.custom_accs.is_empty() {
                ui::text(v, x + self.scale.s(20.0), rows_top + self.scale.s(54.0), "no custom accents — run gen_custom_acc_files", self.scale.fs(8.5), ui::fg3(&pal), false);
            }
            return;
        }
        self.accent_list_rect = (0.0, 0.0, 0.0, 0.0);
        let rows: [(&str, &str, bool); 3] = [
            ("w", "From wallpaper", cur == "w"),
            ("d", "Scheme default", cur == "d"),
            ("c", "Custom accent", cur == "c"),
        ];
        let mut ry = rows_top;
        for (i, &(key_src, name, active)) in rows.iter().enumerate() {
            if ry + rh > y + h - self.scale.s(3.0) {
                break;
            }
            let key = crate::shell::ACC_KEY_BASE + i as u32;
            let hov = self.hover_key == key;
            let row_x = x + self.scale.s(10.0);
            if active {
                // active item: short accent bar on the left edge (no row bg)
                v.push(Cmd::Rect {
                    x: row_x,
                    y: ry + (rh - self.scale.s(4.0)) / 2.0 - self.scale.s(6.0),
                    w: self.scale.s(3.0),
                    h: self.scale.s(12.0),
                    r: self.scale.s(1.5),
                    color: pal.acc,
                });
            }
            let label = if key_src == "c" && active && !cur_name.is_empty() {
                format!("{name} · {cur_name}")
            } else {
                name.to_string()
            };
            ui::text(v, x + self.scale.s(18.0), ry + self.scale.s(6.5), label, self.scale.fs(10.0), if hov { pal.acc } else { pal.fg }, false);
            self.region(row_x, ry, w - self.scale.s(20.0), rh - self.scale.s(4.0), key);
            ry += rh;
        }
    }

}
