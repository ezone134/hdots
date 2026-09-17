use super::super::*;
use super::shared::*;

impl Shell {

    /// SPEED TEST — on-demand Cloudflare bandwidth probe. The button sets
    /// `pending_speed_test`; app.rs runs the worker and reports back, then
    /// this card shows down / up / ping figures + "run again".
    pub(crate) fn draw_speedtest_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        self.speed_rect = (x, y, w, h);
        let pad = self.scale.s(12.0);
        let hdr = self.scale.s(24.0);
        let (st_lbl, _st_col) = match self.speed_state {
            1 => ("testing…", ui::fg3(&pal)),
            2 => ("done", ui::OK),
            _ => ("idle", ui::fg3(&pal)),
        };
        card_header(v, pal, x, y, w, pad, "Speed test", Some(ICON_SPEEDTEST), Some((st_lbl, false)), self.scale, self.card_show_title, self.card_show_glyph);

        // stat readout area above the button
        let body_y = y + hdr + self.scale.s(2.0);
        let body_h = (h - hdr - self.scale.s(46.0)).max(10.0);
        if self.speed_state == 1 {
            ui::text_c(v, x + w / 2.0, body_y + body_h / 2.0 - self.scale.s(6.0), "probing Cloudflare edge…", self.scale.fs(9.0), ui::fg3(&pal), false);
        } else if self.speed_state == 2 {
            let cols = [ui::INFO, pal.acc, ui::fg3(&pal)];
            let labels = [format!("down {}", self.speed_down), format!("up {}", self.speed_up), format!("ping {} ms", self.speed_ping)];
            let cw = w / 3.0;
            for (i, (lbl, col)) in labels.iter().zip(cols.iter()).enumerate() {
                let cxc = x + i as f32 * cw + cw / 2.0;
                ui::text_c_mono(v, cxc, body_y + body_h / 2.0 - self.scale.s(6.0), lbl, self.scale.fs(10.0), *col);
            }
        } else {
            ui::text_c(v, x + w / 2.0, body_y + body_h / 2.0 - self.scale.s(6.0), "click to measure", self.scale.fs(9.0), ui::fg3(&pal), false);
        }

        // run button
        let bx = x + pad;
        let bw = w - pad * 2.0;
        let bh = self.scale.s(30.0);
        let by = y + h - bh - self.scale.s(8.0);
        let key = crate::shell::SPEED_KEY_BASE;
        let hov = self.hover_key == key;
        let running = self.speed_state == 1;
        let bg = if running { mix(ui::hover(&pal), pal.fg, 0.10) } else if hov { mix(pal.acc, pal.fg, 0.15) } else { pal.acc };
        v.push(Cmd::Rect { x: bx, y: by, w: bw, h: bh, r: bh / 2.0, color: bg });
        let label = if running { "testing…".to_string() } else if self.speed_state == 2 { format!("{}  run again", ICON_REFRESH) } else { format!("{}  run test", ICON_PLAY) };
        let tc = if running { ui::fg3(&pal) } else { 0xff141414 };
        ui::text_c(v, bx + bw / 2.0, by + bh / 2.0 - self.scale.s(6.0), label, self.scale.fs(10.5), tc, false);
        if !running {
            self.region(bx, by, bw, bh, key);
        }
    }

}
