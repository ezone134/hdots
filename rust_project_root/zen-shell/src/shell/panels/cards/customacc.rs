use super::super::*;
use super::shared::*;

impl Shell {

    /// CUSTOM ACCENTS — searchable accent browser card. Search bar at the
    /// top-middle (click → live filter, Esc blurs), list/grid view toggle in
    /// the top-right corner. Clicking an accent activates it (acc_source = c,
    /// same `pending_accent_src` path as the Settings dropdown). The failed
    /// heuristics (WCAG-scaled brightness names) are surfaced raw here so the
    /// sort order is just the file order.
    pub(crate) fn draw_customacc_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        use crate::shell::{
            CUSTOMACC_CONTENT_Y, CUSTOMACC_SEARCH_H, CUSTOMACC_ROW_H, CUSTOMACC_CELL,
            CUSTOMACC_GAP_X, CUSTOMACC_PITCH_Y, CUSTOMACC_VIEW_KEY, CUSTOMACC_SEARCH_KEY,
            CUSTOMACC_KEY_BASE,
        };
        let pad = self.scale.s(12.0);
        self.load_custom_accs();
        let cnt = format!("{}", self.custom_accs.len());
        self.card_head(
            v, pal, x, y, w, pad, "Custom Accents", Some(ICON_STICKY),
            Some((cnt.as_str(), true)),
        );

        // top-right corner: list / grid view toggle (header row)
        let tk = CUSTOMACC_VIEW_KEY;
        let thov = self.hover_key == tk;
        let ts = self.scale.s(20.0);
        let tx = x + w - self.scale.s(28.0);
        let ty = y + self.scale.s(8.0);
        v.push(Cmd::Rect {
            x: tx, y: ty, w: ts, h: ts, r: self.scale.s(6.0),
            color: if thov { ui::hover(pal) } else { (pal.bg & 0xffffff00) | 0x16 },
        });
        let icon = if self.customacc_list_view { ICON_GRID } else { ICON_LIST };
        ui::text_c(v, tx + ts / 2.0, ty + ts / 2.0 - self.scale.s(2.0), icon, self.scale.fs(12.0), if thov { pal.acc } else { ui::fg2(pal) }, true);
        self.region(tx - self.scale.s(3.0), ty - self.scale.s(3.0), ts + self.scale.s(6.0), ts + self.scale.s(6.0), tk);

        // search bar at the top-middle (below the header row)
        let focus = self.customacc_search.is_some();
        let sk = CUSTOMACC_SEARCH_KEY;
        let shov = self.hover_key == sk;
        let sy = y + self.scale.s(30.0);
        let sh = self.scale.s(CUSTOMACC_SEARCH_H);
        let sw = w - self.scale.s(20.0);
        v.push(Cmd::Rect {
            x: x + self.scale.s(10.0), y: sy, w: sw, h: sh, r: sh / 2.0,
            color: if focus { ui::acc_tint(pal) } else if shov { ui::hover(pal) } else { (pal.bg & 0xffffff00) | 0x16 },
        });
        ui::text(v, x + self.scale.s(24.0), sy + self.scale.s(7.0), ICON_SEARCH, self.scale.fs(11.0), if focus { pal.acc } else { ui::fg3(pal) }, true);
        let q = self.customacc_search.as_deref().unwrap_or("");
        ui::text(v, x + self.scale.s(40.0), sy + self.scale.s(7.5), q, self.scale.fs(9.5), if q.is_empty() { ui::fg3(pal) } else { pal.fg }, false);
        if q.is_empty() && !focus {
            ui::text(v, x + self.scale.s(40.0), sy + self.scale.s(7.5), "Search custom accents…", self.scale.fs(9.5), ui::fg3(pal), false);
        }
        self.region(x + self.scale.s(10.0), sy, sw, sh, sk);

        // content viewport (stored so wheel scrolling + click math agree)
        let cxo = x + self.scale.s(10.0);
        let cyo = y + self.scale.s(CUSTOMACC_CONTENT_Y);
        let cw = w - self.scale.s(20.0);
        let chh = (y + h - self.scale.s(3.0) - cyo).max(self.scale.s(8.0));
        self.customacc_rect = (cxo, cyo, cw, chh);

        let names = self.customacc_filtered();
        if names.is_empty() {
            card_empty(
                v, pal, x, y, w, h,
                if self.custom_accs.is_empty() { "no custom accents — run gen_custom_acc_files" } else { "no matches" },
                self.scale,
            );
            return;
        }

        let sel_name = if self.current_acc_source() == "c" {
            self.current_acc_name()
        } else {
            String::new()
        };
        let cur = self.current_acc_source();

        if self.customacc_list_view {
            // ── list view: one row per accent (swatch, name, check active) ──
            let rh = self.scale.s(CUSTOMACC_ROW_H);
            let visible = ((chh - self.scale.s(2.0)) / rh).floor().max(1.0) as usize;
            self.customacc_scroll = self.customacc_scroll.min(names.len().saturating_sub(visible));
            for j in 0..visible {
                let Some(&ai) = names.get(self.customacc_scroll + j) else { break };
                let a = &self.custom_accs[ai];
                let key = CUSTOMACC_KEY_BASE + (self.customacc_scroll + j) as u32;
                let hov = self.hover_key == key;
                let sel = cur == "c" && a.name == sel_name;
                let rj = cyo + j as f32 * rh;
                if sel {
                    v.push(Cmd::Rect {
                        x: cxo,
                        y: rj + rh / 2.0 - self.scale.s(6.0),
                        w: self.scale.s(3.0),
                        h: self.scale.s(12.0),
                        r: self.scale.s(1.5),
                        color: pal.acc,
                    });
                }
                v.push(Cmd::Rect {
                    x: cxo + self.scale.s(10.0),
                    y: rj + (rh - self.scale.s(14.0)) / 2.0,
                    w: self.scale.s(14.0),
                    h: self.scale.s(14.0),
                    r: self.scale.s(4.0),
                    color: self.accent_display(a),
                });
                let shown: String = a.name.chars().take(30).collect();
                ui::text(v, cxo + self.scale.s(32.0), rj + self.scale.s(6.5), &shown, self.scale.fs(9.5), if hov { pal.acc } else { pal.fg }, false);
                if sel {
                    ui::text_r(v, cxo + cw - self.scale.s(4.0), rj + self.scale.s(6.5), ICON_CHECK, self.scale.fs(10.0), pal.acc, true);
                }
                self.region(cxo, rj, cw, rh, key);
            }
            if names.len() > visible {
                let bar_max = chh - self.scale.s(4.0);
                let bar_h = (bar_max * visible as f32 / names.len() as f32).clamp(self.scale.s(12.0), bar_max);
                let ratio = self.customacc_scroll as f32 / (names.len().saturating_sub(visible)) as f32;
                let bar_y = cyo + self.scale.s(2.0) + ratio * (bar_max - bar_h);
                v.push(Cmd::Rect { x: x + w - self.scale.s(6.0), y: bar_y, w: self.scale.s(2.0), h: bar_h, r: self.scale.s(1.0), color: mix(pal.acc, pal.bg, 0.35) });
            }
        } else {
            // ── grid view: labeled color circles ──
            let cols = ((cw - self.scale.s(8.0)) / self.scale.s(CUSTOMACC_CELL + CUSTOMACC_GAP_X)).floor().max(1.0) as usize;
            let rows = ((chh + self.scale.s(2.0)) / self.scale.s(CUSTOMACC_PITCH_Y)).floor().max(1.0) as usize;
            let cell = self.scale.s(CUSTOMACC_CELL);
            let gapx = self.scale.s(CUSTOMACC_GAP_X);
            let pitch = self.scale.s(CUSTOMACC_PITCH_Y);
            let gx0 = cxo + self.scale.s(4.0);
            self.customacc_scroll = self.customacc_scroll.min(names.len().saturating_sub(cols * rows));
            for r in 0..rows {
                for c in 0..cols {
                    let idx = self.customacc_scroll + r * cols + c;
                    let Some(&ai) = names.get(idx) else { break };
                    let a = &self.custom_accs[ai];
                    let key = CUSTOMACC_KEY_BASE + idx as u32;
                    let hov = self.hover_key == key;
                    let sel = cur == "c" && a.name == sel_name;
                    let cx0 = gx0 + c as f32 * (cell + gapx);
                    let cy0 = cyo + self.scale.s(2.0) + r as f32 * pitch;
                    if sel || hov {
                        v.push(Cmd::Rect {
                            x: cx0 - self.scale.s(3.0),
                            y: cy0 - self.scale.s(3.0),
                            w: cell + self.scale.s(6.0),
                            h: cell + self.scale.s(6.0),
                            r: (cell + self.scale.s(6.0)) / 2.0,
                            color: if sel { mix(pal.acc, pal.bg, 0.35) } else { ui::hover(pal) },
                        });
                    }
                    v.push(Cmd::Rect { x: cx0, y: cy0, w: cell, h: cell, r: cell / 2.0, color: self.accent_display(a) });
                    if sel {
                        ui::text(v, cx0 + cell / 2.0 - self.scale.s(5.0), cy0 + cell / 2.0 - self.scale.s(8.0), ICON_CHECK, self.scale.fs(11.0), pal.sfg, true);
                    }
                    let shown: String = a.name.chars().take(9).collect();
                    ui::text(v, cx0 - self.scale.s(4.0), cy0 + cell + self.scale.s(1.0), &shown, self.scale.fs(8.5), if sel { pal.sfg } else { pal.fg }, false);
                    self.region(cx0 - self.scale.s(4.0), cy0 - self.scale.s(4.0), cell + self.scale.s(8.0), cell + self.scale.s(12.0), key);
                }
            }
            if names.len() > cols * rows {
                let bar_max = chh - self.scale.s(4.0);
                let total_rows = (names.len() + cols - 1) / cols;
                let bar_h = (bar_max * rows as f32 / total_rows as f32).clamp(self.scale.s(12.0), bar_max);
                let ratio = ((self.customacc_scroll / cols) as f32) / (total_rows.saturating_sub(rows)) as f32;
                let bar_y = cyo + self.scale.s(2.0) + ratio * (bar_max - bar_h);
                v.push(Cmd::Rect { x: x + w - self.scale.s(6.0), y: bar_y, w: self.scale.s(2.0), h: bar_h, r: self.scale.s(1.0), color: mix(pal.acc, pal.bg, 0.35) });
            }
        }
    }

}