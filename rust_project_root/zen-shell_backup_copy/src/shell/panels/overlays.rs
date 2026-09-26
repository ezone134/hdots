//! Overlays: corner notification popup, OSD, workspace switcher overlay.

use super::*;

impl Shell {
        pub fn layout_notif_popup(&mut self, w: f32, h: f32, notifs: &[crate::shell::Notif]) -> Vec<Cmd> {
        use super::{NOTIF_CARD_H, NOTIF_CARD_H_ACTIONS, NOTIF_GAP, NOTIF_PAD};
        let fg = self.cfg.fg();
        let bg = self.cfg.bg();
        let hover = self.cfg.hover();
        let acc = self.cfg.acc();
        let mut v = Vec::new();
        // mako-style: translucent dark backdrop
        v.push(Cmd::Rect { x: 0.0, y: 0.0, w, h, r: self.scale.s(16.0), color: mix(bg, 0x000000ff, 0.40) });
        for (i, n) in notifs.iter().enumerate() {
            let card_h = if n.actions.is_empty() { NOTIF_CARD_H } else { NOTIF_CARD_H_ACTIONS };
            let y = NOTIF_PAD + i as f32 * (card_h + NOTIF_GAP);
            if y + card_h > h + 0.5 {
                break;
            }
            let card_bg = match n.urgency {
                0 => mix(hover, bg, 0.08),
                2 => mix(hover, 0xff4444ff, 0.10),
                _ => mix(hover, bg, 0.04),
            };
            let cx = self.scale.s(10.0);
            let cw = w - self.scale.s(20.0);
            v.push(Cmd::Rect { x: cx, y, w: cw, h: card_h, r: self.scale.s(14.0), color: card_bg });
            // urgency accent left strip (mako-style)
            if n.urgency == 2 {
                v.push(Cmd::Rect { x: cx, y: y + self.scale.s(6.0), w: self.scale.s(3.5), h: card_h - self.scale.s(12.0), r: self.scale.s(1.5), color: 0xff4444ff });
            }
            // app icon
            if let Some(ik) = n.icon_key() {
                v.push(Cmd::Image { x: cx + self.scale.s(14.0), y: y + self.scale.s(14.0), w: self.scale.s(32.0), h: self.scale.s(32.0), key: ik });
            }
            let tx = if n.icon_key().is_some() { cx + self.scale.s(56.0) } else { cx + self.scale.s(14.0) };
            let tw = cx + cw - tx - self.scale.s(10.0);
            // app name (mako-style: small, dim, uppercase feel)
            let app_name = if n.app.is_empty() { "unknown" } else { &n.app };
            ui::text(&mut v, tx, y + self.scale.s(10.0), app_name, self.scale.fs(9.5), FG3, false);
            // summary (bright, main text)
            let summary: String = n.summary.chars().take((tw / self.scale.s(7.0)).max(16.0) as usize).collect();
            ui::text(&mut v, tx, y + self.scale.s(26.0), &summary, self.scale.fs(12.5), fg, false);
            // body (dim, secondary)
            if !n.body.is_empty() {
                let body: String = n.body.chars().take((tw / self.scale.s(5.8)).max(20.0) as usize).collect();
                ui::text(&mut v, tx, y + self.scale.s(44.0), &body, self.scale.fs(10.5), FG2, false);
            }
            // action chips (mako-style: compact pill buttons)
            if !n.actions.is_empty() {
                let mut ax = tx;
                for (j, (_id, label)) in n.actions.iter().take(3).enumerate() {
                    let chip_label: String = label.chars().take(16).collect();
                    let chip_w = self.scale.s(14.0) + chip_label.chars().count() as f32 * self.scale.s(6.0);
                    let chip_y = y + card_h - self.scale.s(30.0);
                    let chip_key = (100 + i * 10 + j) as u32;
                    let chip_hov = self.hover_key == chip_key;
                    let chip_bg = if chip_hov { mix(acc, fg, 0.25) } else { mix(hover, acc, 0.10) };
                    v.push(Cmd::Rect { x: ax, y: chip_y, w: chip_w, h: self.scale.s(20.0), r: self.scale.s(10.0), color: chip_bg });
                    ui::text(&mut v, ax + self.scale.s(8.0), chip_y + self.scale.s(3.0), &chip_label, self.scale.fs(9.5), if chip_hov { fg } else { FG2 }, false);
                    self.region(ax - self.scale.s(2.0), chip_y - self.scale.s(3.0), chip_w + self.scale.s(4.0), self.scale.s(26.0), chip_key);
                    ax += chip_w + self.scale.s(5.0);
                }
            }
        }
        v
    }

    /// Transient OSD — the pill morphs into a small level/status box.

        pub(crate) fn layout_osd(&mut self, v: &mut Vec<Cmd>, w: f32, h: f32, pal: &Pal) {
        self.region(0.0, 0.0, w, h, 1); // click → dismiss
        let Some(osd) = &self.osd else { return };
        ui::text(v, self.scale.s(18.0), (h - self.scale.s(22.0)) / 2.0, osd.glyph, self.scale.fs(22.0), pal.acc, true);
        ui::text(v, self.scale.s(52.0), (h - self.scale.s(14.0)) / 2.0, &osd.text, self.scale.fs(14.0), pal.fg, false);
    }

    /// Tide Island–inspired workspace switcher: grid of workspace cards.
    /// Each card shows the workspace number; active one has an accent border.

        pub(crate) fn layout_workspace_switcher(&mut self, v: &mut Vec<Cmd>, w: f32, h: f32, pal: &Pal) {
        // background click → collapse
        self.region(0.0, 0.0, w, h, 1);
        // title
        ui::text(v, self.scale.s(16.0), self.scale.s(14.0), "Workspaces", self.scale.fs(17.0), pal.fg, false);
        ui::text_r(v, w - self.scale.s(20.0), self.scale.s(16.0), format!("{}", self.ws_count), self.scale.fs(13.0), ui::fg3(&pal), false);
        // grid of workspace cards — 2 columns, up to 10 workspaces
        let card_w = self.scale.s(274.0);
        let card_h = self.scale.s(56.0);
        let gap = self.scale.s(14.0);
        let cols = 2usize.min(self.ws_count.max(1));
        for i in 0..self.ws_count.min(10) {
            let col = i % cols;
            let row = i / cols;
            let cx = self.scale.s(14.0) + col as f32 * (card_w + gap);
            let cy = self.scale.s(44.0) + row as f32 * (card_h + gap);
            let active = i == self.ws_active;
            let hov = self.hover_key == (10 + i as u32);
            // card background
            let bg = if active {
                mix(ui::hover(&pal), pal.acc, 0.20)
            } else if hov {
                ui::hover_hl(&pal)
            } else {
                ui::hover(&pal)
            };
            v.push(Cmd::Rect { x: cx, y: cy, w: card_w, h: card_h, r: self.scale.s(14.0), color: bg });
            // active indicator (accent left bar)
            if active {
                v.push(Cmd::Rect { x: cx, y: cy + self.scale.s(8.0), w: self.scale.s(4.0), h: card_h - self.scale.s(16.0), r: self.scale.s(2.0), color: pal.acc });
            }
            // workspace number
            let num_color = if active { pal.acc } else if hov { pal.fg } else { ui::fg2(&pal) };
            ui::text(v, cx + self.scale.s(16.0), cy + self.scale.s(12.0), format!("{}", i + 1), self.scale.fs(20.0), num_color, false);
            // "Desktop" label
            ui::text(v, cx + self.scale.s(44.0), cy + self.scale.s(16.0), "Desktop", self.scale.fs(11.0), ui::fg3(&pal), false);
            // click region
            self.region(cx, cy, card_w, card_h, 10 + i as u32);
        }
    }
}
