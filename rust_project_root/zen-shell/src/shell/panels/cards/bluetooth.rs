use super::super::*;

impl Shell {

    /// BLUETOOTH — adapter state + device list; row click connects /
    /// disconnects via the existing `pending_bt` plumbing.
    pub(crate) fn draw_bt_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        self.bt_rect = (x, y, w, h);
        let pad = self.scale.s(12.0);
        let hdr = self.scale.s(24.0);
if self.card_show_glyph { ui::text(v, x + pad, y + self.scale.s(8.0), ICON_BLUETOOTH, self.scale.fs(12.0), pal.acc, true); }
if !self.scene_owns_header && self.card_show_title { ui::title(v, if self.card_show_glyph { x + pad + self.scale.s(17.0) } else { x + pad }, y + self.scale.s(9.0), "Bluetooth", self.scale.fs(12.0), pal.fg); }
        let chip = if self.bt_on { "on" } else { "off" };
        let chip_col = if self.bt_on { pal.acc } else { ui::fg3(&pal) };
        ui::text_r(v, x + w - pad, y + self.scale.s(9.0), chip, self.scale.fs(8.5), chip_col, false);

        if self.bt_devices.is_empty() {
            let msg = if self.bt_on {
                "no paired devices"
            } else {
                "bluetooth off — enable in toggles"
            };
            ui::text_c(v, x + w / 2.0, y + h / 2.0 - self.scale.s(6.0), msg, self.scale.fs(9.0), ui::fg3(&pal), false);
            return;
        }
        let row_h = self.scale.s(19.0);
        let x0 = x + pad;
        let ww = (w - pad * 2.0).max(20.0);
        let visible = ((h - hdr - self.scale.s(4.0)) / row_h).floor().max(1.0) as usize;
        let len = self.bt_devices.len();
        self.bt_scroll = self.bt_scroll.min(len.saturating_sub(visible));
        let top = self.bt_scroll;
        let y0 = y + hdr + self.scale.s(2.0);
        for j in 0..visible {
            let i = top + j;
            let Some((name, connected, paired)) = self.bt_devices.get(i) else { break };
            let rj = y0 + j as f32 * row_h;
            let key = crate::shell::BT_KEY_BASE + j as u32;
            let hov = self.hover_key == key;
            if *connected {
                v.push(Cmd::Rect { x: x0, y: rj + 1.0, w: ww, h: row_h - 2.0, r: self.scale.s(4.0), color: ui::acc_tint(&pal) });
            } else if hov {
                v.push(Cmd::Rect { x: x0, y: rj + 1.0, w: ww, h: row_h - 2.0, r: self.scale.s(4.0), color: ui::hover(&pal) });
            }
            ui::text(v, x0 + self.scale.s(6.0), rj + self.scale.s(1.5), format!("{} {}", ICON_BLUETOOTH, name), self.scale.fs(9.5), if *connected { pal.acc } else { pal.fg }, false);
            if *connected {
                ui::text_r(v, x + w - pad, rj + self.scale.s(2.0), ICON_CHECK, self.scale.fs(9.0), pal.acc, true);
            } else if !*paired {
                ui::text_r(v, x + w - pad, rj + self.scale.s(2.0), "unpaired", self.scale.fs(8.0), ui::fg3(&pal), false);
            }
            v.push(Cmd::Rect { x: x0, y: rj + row_h - 1.0, w: ww, h: 1.0, r: 0.5, color: ui::hover(&pal) });
            self.region(x0, rj, ww, row_h, key);
        }
    }

}
