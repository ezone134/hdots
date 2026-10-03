use super::super::*;

impl Shell {

    /// APPS — user-pinned app launcher card ("+ + +" empty slate). Up to 8
    /// slots in a width-derived column grid. A filled slot shows the app icon
    /// (image or Nerd fallback) + clipped name and launches on click; hovering
    /// it reveals a ✕ corner that removes the slot. An empty slot is a "+"
    /// tile whose click opens the launcher in pick mode for that slot.
    pub(crate) fn draw_apps_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        use crate::shell::{APP_SHORTCUT_KEY_BASE, APP_SHORTCUT_REMOVE_BASE};
        self.app_shortcut_rect = (x, y, w, h);
        let pad = self.scale.s(12.0);
        let hdr = self.scale.s(28.0);
        // header (mirrors the Eq card): icon + label + count
if self.card_show_glyph { ui::text(v, x + pad, y + self.scale.s(8.0), ICON_APPS, self.scale.fs(12.0), pal.acc, true); }
if self.card_show_title { ui::title(v, if self.card_show_glyph { x + pad + self.scale.s(18.0) } else { x + pad }, y + self.scale.s(9.0), "Apps", self.scale.fs(12.0), pal.fg); }
        let filled = self.app_shortcuts.iter().filter(|s| !s.is_empty()).count();
        ui::text_r(v, x + w - pad, y + self.scale.s(10.0), format!("{filled}/8"), self.scale.fs(9.0), ui::fg3(&pal), false);
        let sep_y = y + hdr;
        v.push(Cmd::Rect { x: x + pad, y: sep_y, w: w - pad * 2.0, h: self.scale.s(1.0), r: self.scale.s(0.5), color: mix(ui::hover(&pal), pal.fg, 0.08) });

        // column grid from width; rows fill the body, capped at 8 slots total
        let cols: usize = if w >= self.scale.s(330.0) { 4 }
            else if w >= self.scale.s(250.0) { 3 }
            else if w >= self.scale.s(165.0) { 2 }
            else { 1 };
        let gap = self.scale.s(8.0);
        let tw = (w - pad * 2.0 - (cols as f32 - 1.0) * gap) / cols as f32;
        let body_h = (h - hdr - self.scale.s(6.0)).max(self.scale.s(24.0));
        let rows_fit = ((body_h + gap) / (tw + gap)) as usize;
        let rows = rows_fit.min(8usize.div_ceil(cols));
        let slots = (cols * rows).min(8);
        let th = tw;
        for i in 0..slots {
            let col = i % cols;
            let row = i / cols;
            let tx = x + pad + col as f32 * (tw + gap);
            let ty = y + hdr + self.scale.s(8.0) + row as f32 * (th + gap);
            if ty + th > y + h - self.scale.s(4.0) {
                continue;
            }
            let key = APP_SHORTCUT_KEY_BASE + i as u32;
            let rem_key = APP_SHORTCUT_REMOVE_BASE + i as u32;
            let hov = self.hover_key == key || self.hover_key == rem_key;
            let filled = i < self.app_shortcuts.len();
            // tile background
            v.push(Cmd::Rect {
                x: tx,
                y: ty,
                w: tw,
                h: th,
                r: self.scale.s(10.0),
                color: if filled {
                    if hov { ui::hover_hl(&pal) } else { mix(ui::hover(&pal), pal.fg, 0.05) }
                } else if hov {
                    ui::hover_hl(&pal)
                } else {
                    mix(ui::hover(&pal), pal.fg, 0.06)
                },
            });
            if filled {
                let name: String = match self.apps.apps.iter().find(|a| a.id == self.app_shortcuts[i]) {
                    Some(a) => {
                        if let Some(ik) = a.icon_key() {
                            v.push(Cmd::Image { x: tx + (tw - self.scale.s(26.0)) / 2.0, y: ty + self.scale.s(6.0), w: self.scale.s(26.0), h: self.scale.s(26.0), key: ik });
                        } else {
                            ui::text_c(v, tx + tw / 2.0, ty + self.scale.s(9.0), ICON_APPS, self.scale.fs(17.0), pal.acc, true);
                        }
                        a.name.clone()
                    }
                    None => {
                        ui::text_c(v, tx + tw / 2.0, ty + self.scale.s(9.0), ICON_APPS, self.scale.fs(17.0), pal.acc, true);
                        self.app_shortcuts[i].clone()
                    }
                };
                let label: String = name.chars().take(((tw - self.scale.s(4.0)) / self.scale.s(5.5)).max(3.0) as usize).collect();
                ui::text_c(v, tx + tw / 2.0, ty + th - self.scale.s(12.0), label, self.scale.fs(8.0), if hov { pal.fg } else { ui::fg2(&pal) }, false);
                // hover-only ✕ corner — removes the slot
                if hov {
                    let rm = (tx + tw - self.scale.s(15.0), ty - self.scale.s(2.0), self.scale.s(18.0), self.scale.s(18.0));
                    let rm_hov = self.hover_key == rem_key;
                    v.push(Cmd::Rect {
                        x: rm.0,
                        y: rm.1,
                        w: rm.2,
                        h: rm.3,
                        r: rm.2 / 2.0,
                        color: if rm_hov { mix(RED, ui::hover(&pal), 0.55) } else { mix(ui::hover(&pal), pal.fg, 0.10) },
                    });
                    ui::text_c(v, rm.0 + rm.2 / 2.0, rm.1 + self.scale.s(2.5), ICON_CLOSE, self.scale.fs(9.0), if rm_hov { RED } else { pal.fg }, true);
                    self.region(rm.0 - self.scale.s(2.0), rm.1 - self.scale.s(2.0), rm.2 + self.scale.s(4.0), rm.3 + self.scale.s(4.0), rem_key);
                }
            } else {
                // empty "+" slate — opens the launcher in pick mode
                ui::text_c(v, tx + tw / 2.0, ty + th / 2.0 - self.scale.s(3.0), "+", self.scale.fs(20.0), if hov { pal.acc } else { ui::fg3(&pal) }, true);
            }
            self.region(tx, ty, tw, th, key);
        }
    }

}
