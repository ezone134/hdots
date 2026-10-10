use super::super::*;
use super::shared::*;

impl Shell {

    /// MIRROR — two-pane card. Pane 0: live front-camera feed (app pumps
    /// decoded frames into the `cam` image key) + take-photo / record-video
    /// buttons. Pane 1: fps settings (15/30/60 rows + show-real-fps toggle;
    /// the active row also reports the actual delivered rate). Horizontal
    /// scroll flips panes; dots below mark the active one.
    pub(crate) fn draw_mirror_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        self.mirror_rect = (x, y, w, h);
        // header: icon + title only — fps controls live on pane 2
if self.card_show_glyph { ui::text(v, x + self.scale.s(12.0), y + self.scale.s(8.0), ICON_CAMERA, self.scale.fs(12.0), pal.acc, true); }
if self.card_show_title { ui::title(v, if self.card_show_glyph { x + self.scale.s(30.0) } else { x + self.scale.s(12.0) }, y + self.scale.s(9.0), "Mirror", self.scale.fs(12.0), pal.fg); }
        // body: pane 0 = feed + photo/record buttons, pane 1 = fps settings
        if self.mirror_pane == 0 {
            if self.cam_ok {
                self.draw_mirror_feed(v, x, y, w, h, pal);
            } else {
                ui::text_c(v, x + w / 2.0, y + h / 2.0 - self.scale.s(6.0), "no camera", self.scale.fs(9.0), ui::fg3(pal), false);
            }
        } else {
            self.draw_mirror_fps_pane(v, x, y, w, h, pal);
        }
        // pagination dots (appear only while the cursor rests on the card)
        let hover = !self.dash_edit
            && self
                .cursor
                .map(|(px, py)| Self::in_rect(px, py, (x, y, w, h)))
                .unwrap_or(false);
        battery_dots(v, pal, x, y, w, h, self.mirror_pane, self.scale.s(7.0), hover);
    }

    /// pane 0 body: full-width feed + photo / record-video buttons.
    fn draw_mirror_feed(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        let pad = self.scale.s(10.0);
        let btn_row_h = self.scale.s(30.0);
        // dots live at the very bottom edge, so the button row sits above them
        let fw = w - pad * 2.0;
        let fh = (h - self.scale.s(40.0) - btn_row_h - self.scale.s(18.0) - pad).max(self.scale.s(4.0));
        if fw > self.scale.s(4.0) && fh > self.scale.s(4.0) {
            v.push(Cmd::Image { x: x + pad, y: y + self.scale.s(38.0), w: fw, h: fh, key: "cam".to_string() });
        }

        let bx0 = x + pad;
        let bx1 = x + w - pad;
        let by = y + h - btn_row_h - self.scale.s(18.0);
        let total = bx1 - bx0;
        let gap = self.scale.s(8.0);
        let hw = (total - gap) / 2.0;

        // photo button (left)
        let kp = MIRROR_PHOTO_KEY;
        let hp = self.hover_key == kp;
        if hp { v.push(Cmd::Rect { x: bx0 - 2.0, y: by - 2.0, w: hw + 4.0, h: btn_row_h + 4.0, r: self.scale.s(6.0), color: ui::hover_hl(pal) }); }
        ui::text(v, bx0 + self.scale.s(8.0), by + (btn_row_h - self.scale.s(12.0)) / 2.0,
            ICON_CAMERA, self.scale.fs(11.0), if hp { pal.acc } else { pal.fg }, false);
        ui::text(v, bx0 + self.scale.s(26.0), by + (btn_row_h - self.scale.fs(10.0)) / 2.0,
            "Photo", self.scale.fs(10.0), if hp { pal.fg } else { ui::fg2(pal) }, false);
        self.region(bx0 - 2.0, by - 2.0, hw + 4.0, btn_row_h + 4.0, kp);

        // record / stop button (right)
        let kr = MIRROR_REC_KEY;
        let hr = self.hover_key == kr;
        let rx0 = bx0 + hw + gap;
        let rec = self.mirror_rec;
        let bg_col = if rec {
            mix(RED, ui::hover_hl(pal), 0.15)
        } else if hr {
            ui::hover_hl(pal)
        } else {
            ui::hover(pal)
        };
        v.push(Cmd::Rect { x: rx0 - 2.0, y: by - 2.0, w: hw + 4.0, h: btn_row_h + 4.0, r: self.scale.s(6.0), color: bg_col });
        let icon = if rec { ICON_SQUARE } else { ICON_DOT };
        let icon_col = if rec { RED } else if hr { pal.acc } else { RED };
        ui::text(v, rx0 + self.scale.s(8.0), by + (btn_row_h - self.scale.s(12.0)) / 2.0,
            icon, self.scale.fs(11.0), icon_col, false);
        let label = if rec {
            let m = self.mirror_rec_secs / 60;
            let s = self.mirror_rec_secs % 60;
            format!("Stop {m:02}:{s:02}")
        } else {
            "Record".to_string()
        };
        ui::text(v, rx0 + self.scale.s(26.0), by + (btn_row_h - self.scale.fs(10.0)) / 2.0,
            &label, self.scale.fs(10.0), if hr { pal.fg } else { ui::fg2(pal) }, false);
        self.region(rx0 - 2.0, by - 2.0, hw + 4.0, btn_row_h + 4.0, kr);
    }

    /// pane 1 body: capture-rate rows (15/30/60) + "show real fps" toggle.
    /// Rows fill the pane from the header down to the pagination dots.
    fn draw_mirror_fps_pane(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        let pad = self.scale.s(12.0);
        let top = y + self.scale.s(40.0);
        let bot = y + h - self.scale.s(14.0);
        let n = (MIRROR_FPS_OPTIONS.len() + 1) as f32;
        let row_h = ((bot - top - self.scale.s(6.0)) / n).min(self.scale.s(34.0)).max(self.scale.s(26.0));
        let row_w = w - pad * 2.0;
        let mut ry = top + self.scale.s(3.0);
        // capture-rate rows — the active row shows the real delivered rate
        // too (when the "Show real fps" toggle is on and the device can't
        // hit the requested rate)
        for (i, rate) in MIRROR_FPS_OPTIONS.iter().enumerate() {
            let key = MIRROR_FPS_ROW_BASE + i as u32;
            let active = self.mirror_fps == *rate;
            let mut label = format!("{rate} fps");
            if active {
                if *rate == 60 {
                    label.push_str(" · best");
                }
                if self.mirror_show_fps && self.cam_rate > 0 && self.cam_rate != *rate {
                    label.push_str(&format!(" · ~{} fps", self.cam_rate));
                }
            }
            let bg = if active { ui::sel_bg(pal) } else { self.hl(key, 0, ui::hover(pal)) };
            if bg != 0 {
                v.push(Cmd::Rect { x: x + pad, y: ry, w: row_w, h: row_h, r: 5.0, color: bg });
            }
            if active {
                v.push(Cmd::Rect { x: x + pad, y: ry + self.scale.s(4.0), w: self.scale.s(3.0), h: row_h - self.scale.s(8.0), r: 1.5, color: pal.acc });
            }
            ui::text(v, x + pad + self.scale.s(14.0), ry + (row_h - self.scale.fs(11.0)) / 2.0,
                &label, self.scale.fs(11.0), if active { pal.fg } else { ui::fg2(pal) }, false);
            self.region(x + pad, ry, row_w, row_h, key);
            ry += row_h + self.scale.s(6.0);
        }
        // toggle row: "Show real fps"
        let tkey = MIRROR_FPS_SHOW_KEY;
        let bg = self.hl(tkey, 0, ui::hover(pal));
        if bg != 0 {
            v.push(Cmd::Rect { x: x + pad, y: ry, w: row_w, h: row_h, r: 5.0, color: bg });
        }
        ui::text(v, x + pad + self.scale.s(14.0), ry + (row_h - self.scale.fs(11.0)) / 2.0,
            "Show real fps", self.scale.fs(11.0), pal.fg, false);
        // switch track + knob
        let sw_w = self.scale.s(26.0);
        let sw_h = self.scale.s(13.0);
        let swx = x + w - pad - sw_w;
        let swy = ry + (row_h - sw_h) / 2.0;
        let on = self.mirror_show_fps;
        let track = ui::mix(pal.bg, pal.fg, if on { 0.45 } else { 0.28 });
        v.push(Cmd::Rect { x: swx, y: swy, w: sw_w, h: sw_h, r: sw_h / 2.0, color: track });
        let kn = sw_h - self.scale.s(4.0);
        let kx = if on { swx + sw_w - kn - self.scale.s(2.0) } else { swx + self.scale.s(2.0) };
        v.push(Cmd::Rect { x: kx, y: swy + self.scale.s(2.0), w: kn, h: kn, r: kn / 2.0, color: pal.fg });
        self.region(x + pad, ry, row_w, row_h, tkey);
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::shell::Shell;

    fn newg() -> Shell {
        let cfg: Config = toml::from_str(crate::config::DEFAULT_SHELL_TOML).expect("default parses");
        let mut shell = Shell::new(cfg);
        shell.mode = Mode::Expanded;
        shell.dash_enabled = true;
        shell.cam_ok = true;
        shell
    }

    #[test]
    fn fps_row_sets_rate_and_persists() {
        let mut s = newg();
        s.mirror_fps = 60;
        let key = MIRROR_FPS_ROW_BASE + 1;
        s.region(100.0, 100.0, 50.0, 50.0, key);
        s.set_cursor(Some((120.0, 120.0)));
        let _ = s.click(120.0, 120.0);
        assert_eq!(s.mirror_fps, 30, "row click should set the rate");
        assert!(s.pending_mirror_fps, "pending flag should be set");
    }

    #[test]
    fn fps_show_toggle_flips_flag() {
        let mut s = newg();
        assert!(s.mirror_show_fps, "defaults on");
        s.region(200.0, 200.0, 50.0, 50.0, MIRROR_FPS_SHOW_KEY);
        s.set_cursor(Some((220.0, 220.0)));
        let _ = s.click(220.0, 220.0);
        assert!(!s.mirror_show_fps, "toggle should flip to off");
        s.region(200.0, 200.0, 50.0, 50.0, MIRROR_FPS_SHOW_KEY);
        s.set_cursor(Some((220.0, 220.0)));
        let _ = s.click(220.0, 220.0);
        assert!(s.mirror_show_fps, "toggle should flip back on");
    }
}