use super::super::*;
use super::shared::*;

use crate::shell::{
    AUDIO_REC_BACK_KEY, AUDIO_REC_CONFIRM_KEY, AUDIO_REC_DEL_BASE, AUDIO_REC_DEL_MAX,
    AUDIO_REC_MIC_KEY, AUDIO_REC_PANE_KEY, AUDIO_REC_PLAY_BASE, AUDIO_REC_PLAY_MAX,
    AUDIO_REC_REC_KEY, AUDIO_REC_SHOWMIC_KEY,
};

impl Shell {

    /// AUDIO RECORDER — two panes.
    /// · pane 0 (recorder): mic level fader (when `audiorec_show_mic`), live
    ///   record/stop button, a "›" chevron in the top-right that opens the
    ///   recordings list.
    /// · pane 1 (recordings): "‹" back chevron + title, a list of previously
    ///   saved files with per-row play / delete, and the confirm-delete +
    ///   show-mic-slider toggles.
    /// The actual pw-record child lives in app.rs; this card only flips the
    /// pending flags / pane / persist state.
    pub(crate) fn draw_audiorec_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        self.audiorec_rect = (x, y, w, h);
        if self.audiorec_pane == 1 {
            self.draw_audiorec_recordings(v, x, y, w, h, pal);
        } else {
            self.draw_audiorec_recorder(v, x, y, w, h, pal);
        }
        let hover = !self.dash_edit
            && self
                .cursor
                .map(|(px, py)| Self::in_rect(px, py, (x, y, w, h)))
                .unwrap_or(false);
        battery_dots(v, pal, x, y, w, h, self.audiorec_pane, self.scale.s(7.0), hover);
    }

    /// pane 0: mic fader, record button, "›" chevron to the recordings list.
    fn draw_audiorec_recorder(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        // If a declarative scene exists (`~/.config/zen-shell/ui/cards/audiorec.ron`),
        // it draws the title + chevron. The Rust code keeps the record button
        // (config-token colors) and mic fader (live values) since those can't
        // be expressed as static scene items yet.
        let scene_loaded = {
            let mut vals = crate::scene::SceneValues::new();
            vals.insert("ic_mic", crate::scene::SceneValue::Text(ICON_MIC.to_string()));
            vals.insert("rec_lbl", crate::scene::SceneValue::Text("Record".into()));
            let meta = if self.audiorec_recording {
                format!("●  {:02}:{:02}", self.audiorec_rec_secs / 60, self.audiorec_rec_secs % 60)
            } else {
                let n = self.audiorec_recordings.len();
                if n > 0 {
                    format!("{n} saved")
                } else {
                    String::new()
                }
            };
            vals.insert("meta", crate::scene::SceneValue::Text(meta));
            self.draw_card_scene("audiorec", v, x, y, w, h, pal, &vals)
        };

        let pad = self.scale.s(12.0);
        let hdr = self.scale.s(22.0);
        // header meta carries the live state (recording time / record count)
        let meta = if self.audiorec_recording {
            format!("●  {:02}:{:02}", self.audiorec_rec_secs / 60, self.audiorec_rec_secs % 60)
        } else {
            let n = self.audiorec_recordings.len();
            if n > 0 {
                format!("{n} saved")
            } else {
                String::new()
            }
        };
        if !scene_loaded {
            self.card_head(v, pal, x, y, w, pad, "Recorder", Some(ICON_MIC), Some((&meta, true)));
        }

        // "›" chevron button top-right → recordings pane (skip if scene drew it)
        if !scene_loaded {
            let cw = self.scale.s(16.0);
            let cx = x + w - pad - cw;
            let cy = y + self.scale.s(6.0);
            let hov_pane = self.hover_key == AUDIO_REC_PANE_KEY;
            v.push(Cmd::Rect {
                x: cx,
                y: cy,
                w: cw,
                h: cw,
                r: self.scale.s(5.0),
                color: if hov_pane { ui::raised_hl(&pal) } else { ui::raised(&pal) },
            });
            ui::text_c(v, cx + cw / 2.0, cy + self.scale.s(3.0), ICON_FORWARD, self.scale.fs(10.0), if hov_pane { pal.acc } else { pal.fg }, true);
            self.region(cx - self.scale.s(2.0), cy - self.scale.s(2.0), cw + self.scale.s(4.0), cw + self.scale.s(4.0), AUDIO_REC_PANE_KEY);
        }

        // mic volume fader (hidden by the pane-1 "Show mic slider" toggle)
        let mut ry = y + hdr + self.scale.s(6.0);
        if self.audiorec_show_mic {
            let row_h = self.scale.s(24.0);
            let hov = self.hover_key == AUDIO_REC_MIC_KEY;
            ui::text(v, x + pad, ry + self.scale.s(5.0), if self.mic_muted { ICON_MIC_OFF } else { ICON_MIC }, self.scale.fs(12.0), if hov { pal.acc } else { pal.fg }, true);
            let tx = x + pad + self.scale.s(20.0);
            let tw = (w - pad * 2.0 - self.scale.s(20.0) - self.scale.s(46.0)).max(self.scale.s(40.0));
            let cy = ry + row_h * 0.45;
            let fill = if self.mic_muted { RED } else { pal.acc };
            ui::fader(v, tx, cy, tw, self.mic_level.clamp(0.0, 1.0), fill, hov, pal);
            let lbl = if self.mic_muted { "Muted".to_string() } else { format!("{:>3}%", (self.mic_level * 100.0).round() as i32) };
            ui::text_r(v, x + w - pad, cy - self.scale.s(5.0), &lbl, self.scale.fs(9.5), if self.mic_muted { ui::fg3(&pal) } else { pal.fg }, false);
            self.audiorec_mic_rect = (tx, cy, tw);
            self.region(x + self.scale.s(6.0), ry, w - self.scale.s(12.0), row_h, AUDIO_REC_MIC_KEY);
            ry += row_h + self.scale.s(6.0);
            // hint under the fader (shows the config drivers; hidden while recording)
            if !self.audiorec_recording {
                let hint = if self.mic_muted {
                    "mic muted — recording will capture silence"
                } else {
                    "mic level drives wpctl mic volume"
                };
                ui::caption(v, x + pad, ry + self.scale.s(4.0), hint, self.scale.fs(8.0), ui::fg3(&pal), false);
            }
        } else {
            self.audiorec_mic_rect = (0.0, 0.0, 0.0);
        }

        // record / stop button — colors + pad from the declarative spec
        // (tokens resolve to Copy colors up front so the region registration
        // and scale reads below don't hold a borrow on `self.cfg`)
        let (rec_bg_idle, rec_bg_hov, rec_active, rec_dot) = {
            let spec = self.cfg.audiorec();
            (
                spec.rec_surface.resolve(&pal),
                spec.rec_surface.hover_variant(&pal),
                spec.rec_active_surface.resolve(&pal),
                spec.rec_dot.resolve(&pal),
            )
        };
        let rec_pad_bottom = self.cfg.audiorec().rec_pad_bottom;
        let bh = self.scale.s(26.0);
        let by = y + h - bh - self.scale.s(rec_pad_bottom);
        let hov_rec = self.hover_key == AUDIO_REC_REC_KEY;
        let recording = self.audiorec_recording;
        let rec_bg = if recording {
            rec_active
        } else if hov_rec {
            rec_bg_hov
        } else {
            rec_bg_idle
        };
        v.push(Cmd::Rect {
            x: x + pad - self.scale.s(4.0),
            y: by,
            w: w - pad * 2.0 + self.scale.s(8.0),
            h: bh,
            r: self.scale.s(7.0),
            color: rec_bg,
        });
        if recording {
            // stop glyph: white filled square inside the red button
            let sw = self.scale.s(8.0);
            let sxy = by + (bh - sw) / 2.0;
            v.push(Cmd::Rect { x: x + pad + self.scale.s(2.0), y: sxy, w: sw, h: sw, r: self.scale.s(1.5), color: pal.fg });
        } else {
            // record glyph: red filled circle
            let d = self.scale.s(8.0);
            v.push(Cmd::Rect { x: x + pad + self.scale.s(2.0), y: by + (bh - d) / 2.0, w: d, h: d, r: d / 2.0, color: rec_dot });
        }
        let lbl = if recording {
            format!("Stop  {:02}:{:02}", self.audiorec_rec_secs / 60, self.audiorec_rec_secs % 60)
        } else {
            "Record".to_string()
        };
        ui::text(v, x + pad + self.scale.s(16.0), by + self.scale.s(7.0), &lbl, self.scale.fs(9.5), if recording { mix(pal.fg, rec_active, 0.25) } else { pal.fg }, false);
        self.region(x + pad - self.scale.s(4.0) - self.scale.s(3.0), by - self.scale.s(3.0), w - pad * 2.0 + self.scale.s(8.0) + self.scale.s(6.0), bh + self.scale.s(6.0), AUDIO_REC_REC_KEY);
    }

    /// pane 1: "‹" back · recordings list (play / delete) + the two toggles.
    fn draw_audiorec_recordings(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        let pad = self.scale.s(12.0);
        let hdr = self.scale.s(22.0);

        // "‹" back chevron top-left → recorder pane
        let cw = self.scale.s(16.0);
        let cx = x + pad;
        let cy = y + self.scale.s(6.0);
        let hov_back = self.hover_key == AUDIO_REC_BACK_KEY;
        v.push(Cmd::Rect {
            x: cx,
            y: cy,
            w: cw,
            h: cw,
            r: self.scale.s(5.0),
            color: if hov_back { ui::raised_hl(&pal) } else { ui::raised(&pal) },
        });
        ui::text_c(v, cx + cw / 2.0, cy + self.scale.s(3.0), ICON_BACK, self.scale.fs(10.0), if hov_back { pal.acc } else { pal.fg }, true);
        self.region(cx - self.scale.s(2.0), cy - self.scale.s(2.0), cw + self.scale.s(4.0), cw + self.scale.s(4.0), AUDIO_REC_BACK_KEY);

        let tx = x + pad + cw + self.scale.s(6.0);
        ui::title(v, tx, y + self.scale.s(9.0), "Recordings", self.scale.fs(11.5), pal.fg);

        let rows_top = y + hdr + self.scale.s(4.0);
        let rows_bot = y + h - self.scale.s(44.0);
        let list_h = (rows_bot - rows_top).max(self.scale.s(8.0));
        let row_h = self.scale.s(self.cfg.audiorec().row_h);
        let n_fit = ((list_h / row_h).floor() as usize).max(1);
        let n = self.audiorec_recordings.len().min(n_fit);

        if n == 0 {
            card_empty(v, pal, x, y + rows_top, w, (rows_bot - rows_top).max(self.scale.s(10.0)), "no recordings yet", self.scale);
        }

        let mut ry = rows_top;
        for i in 0..n {
            let (name, _path) = &self.audiorec_recordings[i];
            let pkey = (AUDIO_REC_PLAY_BASE + i as u32).min(AUDIO_REC_PLAY_MAX);
            let dkey = (AUDIO_REC_DEL_BASE + i as u32).min(AUDIO_REC_DEL_MAX);
            let hov = self.hover_key == pkey || self.hover_key == dkey;
            let armed = self.audiorec_del_armed == Some(i);
            // resolve the spec's tokens up front (Copy colors) so we can call
            // `self.region()` below without holding a borrow on `self.cfg`
            let (del_armed, row_hover, del_ink, del_hover, play_hover) = {
                let spec = self.cfg.audiorec();
                (
                    spec.del_armed.resolve(&pal),
                    spec.row_hover.resolve(&pal),
                    if armed { mix(pal.fg, spec.del_armed.resolve(&pal), 0.25) } else { spec.row_ink.resolve(&pal) },
                    spec.del_hover.resolve(&pal),
                    spec.play_hover.resolve(&pal),
                )
            };
            if hov || armed {
                v.push(Cmd::Rect {
                    x: x + pad,
                    y: ry,
                    w: w - pad * 2.0,
                    h: row_h,
                    r: self.scale.s(5.0),
                    color: if armed { del_armed } else { row_hover },
                });
            }
            // file stem (name already trimmed by the app)
            let name_chars = (((w - pad * 2.0 - self.scale.s(56.0)) / self.scale.s(5.4)).max(4.0)) as usize;
            let shown: String = name.chars().take(name_chars).collect();
            ui::caption(v, x + pad + self.scale.s(2.0), ry + self.scale.s(5.0), &shown, self.scale.fs(8.5), del_ink, true);
            // delete glyph (left of play) — armed rows show a red confirm dot
            let d_gx = x + w - pad - self.scale.s(30.0);
            let d_hov = self.hover_key == dkey;
            if armed {
                ui::text(v, d_gx, ry + self.scale.s(5.0), ICON_CLOSE_FILL, self.scale.fs(9.0), mix(del_armed, pal.fg, 0.2), true);
            } else {
                ui::text(v, d_gx, ry + self.scale.s(5.0), ICON_RECYCLE, self.scale.fs(9.0), if d_hov { del_hover } else { ui::fg3(&pal) }, true);
            }
            self.region(d_gx - self.scale.s(3.0), ry - self.scale.s(2.0), self.scale.s(16.0), row_h + self.scale.s(2.0), dkey);
            // play glyph (far right)
            let p_gx = x + w - pad - self.scale.s(12.0);
            let p_hov = self.hover_key == pkey;
            ui::text(v, p_gx, ry + self.scale.s(5.0), ICON_PLAY, self.scale.fs(9.0), if p_hov { play_hover } else { ui::fg3(&pal) }, true);
            self.region(p_gx - self.scale.s(3.0), ry - self.scale.s(2.0), self.scale.s(16.0), row_h + self.scale.s(2.0), pkey);
            ry += row_h;
        }

        // bottom band: Confirm delete + Show mic slider switch rows
        let tog_w = w - pad * 2.0;
        let tog_h = self.scale.s(18.0);
        let mut ty = y + h - tog_h * 2.0 - self.scale.s(2.0);
        for (key, label, on) in [
            (AUDIO_REC_CONFIRM_KEY, "Confirm delete", self.audiorec_confirm_delete),
            (AUDIO_REC_SHOWMIC_KEY, "Show mic slider", self.audiorec_show_mic),
        ] {
            let bg = self.hl(key, 0, ui::hover(pal));
            if bg != 0 {
                v.push(Cmd::Rect { x: x + pad, y: ty, w: tog_w, h: tog_h, r: self.scale.s(4.0), color: bg });
            }
            ui::caption(v, x + pad + self.scale.s(12.0), ty + self.scale.s(5.0), label, self.scale.fs(9.0), pal.fg, false);
            // switch track + knob
            let sw_w = self.scale.s(24.0);
            let sw_h = self.scale.s(12.0);
            let swx = x + w - pad - sw_w;
            let swy = ty + (tog_h - sw_h) / 2.0;
            let track = ui::mix(pal.bg, pal.fg, if on { 0.45 } else { 0.28 });
            v.push(Cmd::Rect { x: swx, y: swy, w: sw_w, h: sw_h, r: sw_h / 2.0, color: track });
            let kn = sw_h - self.scale.s(4.0);
            let kx = if on { swx + sw_w - kn - self.scale.s(2.0) } else { swx + self.scale.s(2.0) };
            v.push(Cmd::Rect { x: kx, y: swy + self.scale.s(2.0), w: kn, h: kn, r: kn / 2.0, color: pal.fg });
            self.region(x + pad, ty - self.scale.s(1.0), tog_w, tog_h + self.scale.s(2.0), key);
            ty += tog_h + self.scale.s(2.0);
        }
    }

    /// pane-1 delete click: with "confirm delete" ON the first click arms the
    /// row (shows a red ✕) and a second click deletes; with it OFF one click
    /// deletes. Returns the action taken so the caller can dirty the surface.
    pub(crate) fn audiorec_del_click(&mut self, i: usize) {
        if self.audiorec_confirm_delete {
            if self.audiorec_del_armed == Some(i) {
                self.audiorec_del_armed = None;
                self.pending_audiorec_del = Some(i);
            } else {
                self.audiorec_del_armed = Some(i);
            }
        } else {
            self.pending_audiorec_del = Some(i);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shell() -> Shell {
        let cfg: crate::config::Config =
            toml::from_str(crate::config::DEFAULT_SHELL_TOML).expect("default config parses");
        let mut s = Shell::new(cfg);
        s.audiorec_recordings = vec![
            ("/tmp/rec-3.wav".into(), "rec-3".into()),
            ("/tmp/rec-2.wav".into(), "rec-2".into()),
            ("/tmp/rec-1.wav".into(), "rec-1".into()),
        ];
        s
    }

    fn pal() -> Pal {
        Pal { fg: 0xe8e6e3ff, bg: 0x141414ff, acc: 0x4fc2ffff, sfg: 0x101010ff }
    }

    #[test]
    fn recorder_draws_and_registers_rect() {
        let mut s = shell();
        let p = pal();
        let mut v: Vec<Cmd> = Vec::new();
        s.draw_audiorec_card(&mut v, 0.0, 0.0, 160.0, 110.0, &p);
        assert_eq!(s.audiorec_rect, (0.0, 0.0, 160.0, 110.0), "card registers its bounds");
        assert!(v.iter().any(|c| matches!(c, Cmd::Text { text, .. } if text.contains("Record"))), "record affordance drawn");
    }

    #[test]
    fn chevron_opens_recordings_pane_and_back() {
        let mut s = shell();
        s.mode = Mode::Expanded;
        // click the "›" chevron region (pane key) → pane 1
        s.region(0.0, 0.0, 160.0, 110.0, AUDIO_REC_PANE_KEY);
        s.set_cursor(Some((10.0, 10.0)));
        let _ = s.click(10.0, 10.0);
        assert_eq!(s.audiorec_pane, 1, "chevron flips to recordings");
        assert!(s.pending_audiorec_list, "opening the pane requests a list refresh");
    }

    #[test]
    fn delete_confirm_two_step() {
        let mut s = shell();
        s.audiorec_confirm_delete = true;
        s.audiorec_del_click(1);
        assert_eq!(s.audiorec_del_armed, Some(1), "first click arms the row");
        assert!(s.pending_audiorec_del.is_none(), "no delete on the arm click");
        s.audiorec_del_click(1);
        assert_eq!(s.pending_audiorec_del, Some(1), "second click deletes");
        assert_eq!(s.audiorec_del_armed, None, "armed state clears");
        // confirm off → one click deletes straight away
        s.audiorec_confirm_delete = false;
        s.audiorec_del_click(0);
        assert_eq!(s.pending_audiorec_del, Some(0), "confirm-off deletes in one click");
    }

    #[test]
    fn spec_defaults_resolve_on_theme() {
        // the default [cards.audiorec] spec (no toml section present) still
        // resolves every token against the live theme
        let s = shell();
        let p = pal();
        let spec = s.cfg.audiorec();
        assert_eq!(spec.del_armed.resolve(&p), ui::DANGER, "del_armed default is danger");
        assert_eq!(spec.row_ink.resolve(&p), ui::fg2(&p), "row_ink default is fg2");
        assert_eq!(spec.rec_active_surface.resolve(&p), ui::DANGER, "record active = danger");
        assert_eq!(spec.row_h, 24.0, "default row height");
    }

    #[test]
    fn spec_overrides_apply() {
        let toml = r#"
            [cards.audiorec]
            rec_surface = "acc"
            rec_active_surface = "ok"
            row_h = 30
        "#;
        let cfg: crate::config::Config = toml::from_str(toml).expect("overrides parse");
        let spec = cfg.audiorec();
        let p = pal();
        assert_eq!(spec.rec_surface.resolve(&p), p.acc, "rec_surface overridden to acc");
        assert_eq!(spec.rec_active_surface.resolve(&p), ui::OK, "rec_active overridden to ok");
        assert_eq!(spec.row_h, 30.0, "row height overridden");
        // untouched fields keep defaults
        assert_eq!(spec.rec_dot.resolve(&p), ui::DANGER);
    }
}