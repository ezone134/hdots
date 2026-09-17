use super::super::*;

impl Shell {

        pub(crate) fn draw_media_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        self.media_card_rect = (x, y, w, h);
        let pad = self.scale.s(14.0);
        let art_s = self.scale.s(48.0);
        let tx0 = x + pad + art_s + self.scale.s(10.0);
        let wide = w >= self.scale.s(210.0);

        // ── album art (always shown, rounded placeholder when empty) ──
        if self.media_art.is_empty() {
            v.push(Cmd::Rect {
                x: x + pad, y: y + self.scale.s(12.0),
                w: art_s, h: art_s, r: self.scale.s(10.0),
                color: mix(ui::hover(&pal), pal.fg, 0.06),
            });
            ui::text(v, x + pad + art_s * 0.3, y + self.scale.s(12.0) + art_s * 0.3,
                ICON_NOTE, self.scale.fs(16.0), pal.fg, true);
        } else {
            v.push(Cmd::Image {
                x: x + pad, y: y + self.scale.s(12.0),
                w: art_s,
                h: art_s,
                key: format!("file:{}", self.media_art),
            });
        }

        // ── track info (right of art, never truncated away) ──
        let info_tx = if wide { tx0 } else { x + pad };
        let avail = w - (info_tx - x) - pad;
        let fit = |s: &str, size: f32| -> String {
            let n = ((avail / (size * 0.62)).floor() as usize).max(4);
            if s.chars().count() > n {
                let mut t: String = s.chars().take(n.saturating_sub(1)).collect();
                t.push('…');
                t
            } else {
                s.to_string()
            }
        };
        let (mtitle, martist, malbum) = if self.media_title.is_empty() {
            ("Nothing playing".to_string(), String::new(), String::new())
        } else {
            (self.media_title.clone(), self.media_artist.clone(), self.media_album.clone())
        };
        let title = fit(&mtitle, self.scale.fs(12.0));
        ui::title(v, info_tx, y + self.scale.s(14.0), title, self.scale.fs(12.0), pal.fg);
        let line2 = if !malbum.is_empty() {
            format!("{malbum}{}", if !martist.is_empty() { format!(" — {martist}") } else { String::new() })
        } else {
            martist.clone()
        };
        let line2 = fit(&line2, self.scale.fs(9.5));
        if !line2.is_empty() {
            ui::text(v, info_tx, y + self.scale.s(32.0), line2, self.scale.fs(9.5), pal.fg, false);
        }

        // ── seek bar + time labels ──
        let sx = x + pad;
        let sw = w - pad * 2.0;
        let has_room = h >= self.scale.s(120.0);
        let sy = y + self.scale.s(80.0);
        let frac = self.media_seek.unwrap_or_else(|| self.media_frac());
        ui::slider_bar(v, sx, sy, sw, self.scale.s(5.0), frac, self.hover_key == 23, pal);
        self.region(sx - self.scale.s(2.0), sy - self.scale.s(6.0), sw + self.scale.s(4.0), self.scale.s(16.0), 23);
        self.media_seek_rect = (sx, sy, sw, self.scale.s(5.0));
        // time labels below the bar
        let cur = self.media_seek
            .map(|f| (f * self.media_len.max(1) as f32) as i64)
            .unwrap_or_else(|| self.media_pos_now());
        let tl_y = sy + self.scale.s(8.0);
        ui::text(v, sx + self.scale.s(2.0), tl_y, fmt_time(cur), self.scale.fs(8.0), pal.fg, false);
        ui::text_r(v, x + w - pad + self.scale.s(2.0), tl_y, fmt_time(self.media_len), self.scale.fs(8.0), pal.fg, false);

        // ── transport buttons (centered, below time labels) ──
        if has_room {
            let btn_y = tl_y + self.scale.s(16.0);
            let btn_cx = x + w / 2.0;
            let play_glyph = if self.media_playing { ICON_PAUSE } else { ICON_PLAY };
            ui::media_btn(v, btn_cx - self.scale.s(36.0), btn_y, ICON_PREV, self.scale.fs(14.0), self.hover_key == 20, pal.fg, pal);
            ui::media_btn(v, btn_cx, btn_y - self.scale.s(2.0), play_glyph, self.scale.fs(17.0), self.hover_key == 21, pal.acc, pal);
            ui::media_btn(v, btn_cx + self.scale.s(36.0), btn_y, ICON_SKIP_NEXT, self.scale.fs(15.0), self.hover_key == 22, pal.fg, pal);
            self.region(btn_cx - self.scale.s(50.0), btn_y - self.scale.s(10.0), self.scale.s(38.0), self.scale.s(36.0), 20);
            self.region(btn_cx - self.scale.s(22.0), btn_y - self.scale.s(12.0), self.scale.s(44.0), self.scale.s(40.0), 21);
            self.region(btn_cx + self.scale.s(12.0), btn_y - self.scale.s(10.0), self.scale.s(38.0), self.scale.s(36.0), 22);
        }
    }

}
