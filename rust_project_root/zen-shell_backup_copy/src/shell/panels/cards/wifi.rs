use super::super::*;

impl Shell {

    /// WIFI — nearby networks with 4-segment signal bars; a row click
    /// connects (open or OS-agent prompt), the connected row stays tinted.
    /// No data plumbing — reuses the Services `wifi_networks` poll.
    pub(crate) fn draw_wifi_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        self.wifi_rect = (x, y, w, h);
        let pad = self.scale.s(12.0);
        let hdr = self.scale.s(24.0);
if self.card_show_glyph { ui::text(v, x + pad, y + self.scale.s(8.0), ICON_WIFI, self.scale.fs(12.0), pal.acc, true); }
if self.card_show_title { ui::title(v, if self.card_show_glyph { x + pad + self.scale.s(17.0) } else { x + pad }, y + self.scale.s(9.0), "Wi-Fi", self.scale.fs(12.0), pal.fg); }
        ui::text_r(v, x + w - pad, y + self.scale.s(9.0), format!("{} nets", self.wifi_networks.len()), self.scale.fs(8.5), ui::fg3(&pal), false);

        if self.wifi_networks.is_empty() {
            let msg = if self.wifi_on {
                "no networks — run a scan"
            } else {
                "wi-fi off — enable in toggles"
            };
            ui::text_c(v, x + w / 2.0, y + h / 2.0 - self.scale.s(6.0), msg, self.scale.fs(9.0), ui::fg3(&pal), false);
            return;
        }
        let row_h = self.scale.s(19.0);
        let x0 = x + pad;
        let ww = (w - pad * 2.0).max(20.0);
        let visible = ((h - hdr - self.scale.s(4.0)) / row_h).floor().max(1.0) as usize;
        let len = self.wifi_networks.len();
        self.wifi_scroll = self.wifi_scroll.min(len.saturating_sub(visible));
        let top = self.wifi_scroll;
        let y0 = y + hdr + self.scale.s(2.0);
        for j in 0..visible {
            let i = top + j;
            let Some((ssid, strength, secured, connected)) = self.wifi_networks.get(i) else { break };
            let rj = y0 + j as f32 * row_h;
            let key = crate::shell::WIFI_KEY_BASE + j as u32;
            let hov = self.hover_key == key;
            let connecting = self.pending_wifi.as_deref() == Some(ssid.as_str());
            if *connected {
                v.push(Cmd::Rect { x: x0, y: rj + 1.0, w: ww, h: row_h - 2.0, r: self.scale.s(4.0), color: ui::acc_tint(&pal) });
            } else if hov {
                v.push(Cmd::Rect { x: x0, y: rj + 1.0, w: ww, h: row_h - 2.0, r: self.scale.s(4.0), color: ui::hover(&pal) });
            }
            // 4-segment signal bars
            let bars = (*strength / 25).clamp(0, 4) as usize;
            let bx = x0 + self.scale.s(4.0);
            let bw = self.scale.s(2.5);
            let gap = self.scale.s(2.0);
            let base_y = rj + row_h - self.scale.s(7.0);
            for k in 0..4 {
                let bh = self.scale.s(2.0) + k as f32 * self.scale.s(2.2);
                v.push(Cmd::Rect { x: bx + k as f32 * (bw + gap), y: base_y - bh, w: bw, h: bh, r: 0.5, color: if k < bars { pal.acc } else { ui::hover(&pal) } });
            }
            let tx = bx + 4.0 * (bw + gap) + self.scale.s(5.0);
            let name: String = ssid.chars().take(22).collect();
            let nc = name.chars().count() as f32 * self.scale.fs(9.5) * 0.62;
            ui::text(v, tx, rj + self.scale.s(1.5), &name, self.scale.fs(9.5), if *connected { pal.acc } else { pal.fg }, false);
            if *secured {
                ui::text(v, tx + nc + self.scale.s(6.0), rj + self.scale.s(2.0), ICON_LOCK, self.scale.fs(8.0), ui::fg3(&pal), true);
            }
            if *connected {
                ui::text_r(v, x + w - pad, rj + self.scale.s(2.0), ICON_CHECK, self.scale.fs(9.0), pal.acc, true);
            } else if connecting {
                ui::text_r(v, x + w - pad, rj + self.scale.s(2.0), "…", self.scale.fs(10.0), ui::fg3(&pal), false);
            }
            v.push(Cmd::Rect { x: x0, y: rj + row_h - 1.0, w: ww, h: 1.0, r: 0.5, color: ui::hover(&pal) });
            self.region(x0, rj, ww, row_h, key);
        }
    }

}
