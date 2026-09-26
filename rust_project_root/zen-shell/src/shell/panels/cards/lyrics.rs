use super::super::*;

impl Shell {

    /// LYRICS — the currently playing track's lyrics (LRCLib + local cache).
    /// Synced lines highlight the sung line and auto-scroll with the
    /// playhead; wheel scrolls the list when it outgrows the card.
    pub(crate) fn draw_lyrics_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        self.lyrics_rect = (x, y, w, h);
        let pad = self.scale.s(12.0);
        let hdr = self.scale.s(Self::LYRICS_HDR);
        // header: glyph + title + state chip
if self.card_show_glyph { ui::text(v, x + pad, y + self.scale.s(8.0), ICON_NOTE, self.scale.fs(12.0), pal.acc, true); }
if !self.scene_owns_header && self.card_show_title { ui::title(v, if self.card_show_glyph { x + pad + self.scale.s(18.0) } else { x + pad }, y + self.scale.s(9.0), "Lyrics", self.scale.fs(12.0), pal.fg); }
        let (state_lbl, state_col) = match self.lyrics_state {
            1 => ("fetching…", ui::fg3(&pal)),
            2 if crate::lyrics::has_timing(&self.lyrics_lines) => ("synced", pal.acc),
            2 => ("plain", ui::fg3(&pal)),
            3 => ("none", RED),
            _ => ("—", ui::fg3(&pal)),
        };
        ui::text_r(v, x + w - pad, y + self.scale.s(10.0), state_lbl, self.scale.fs(8.5), state_col, false);
        // header second line: the track this belongs to
        let track = if self.lyrics_state == 2 && !self.lyrics_track.is_empty() {
            self.lyrics_track.replace(" || ", " — ")
        } else if !self.media_title.is_empty() {
            if !self.media_artist.is_empty() {
                format!("{} — {}", self.media_artist, self.media_title)
            } else {
                self.media_title.clone()
            }
        } else {
            "no track playing".to_string()
        };
        let t2: String = track.chars().take(42).collect();
        ui::text(v, x + pad, y + hdr - self.scale.s(2.0), &t2, self.scale.fs(8.5), ui::fg3(&pal), false);

        if self.lyrics_lines.is_empty() {
            let msg = match self.lyrics_state {
                1 => "fetching lyrics…",
                3 => "no lyrics found",
                _ => "no track playing",
            };
            ui::text_c(v, x + w / 2.0, y + h / 2.0 - self.scale.s(6.0), msg, self.scale.fs(9.0), ui::fg3(&pal), false);
            return;
        }

        let row_h = self.scale.s(Self::LYRICS_ROW_H);
        let x0 = x + pad;
        let ww = (w - pad * 2.0).max(20.0);
        let visible = self.lyrics_card_visible();
        // auto-follow the sung line while playing (keeps manual scrolls while
        // the active line stays on screen)
        self.lyrics_follow(visible);
        let len = self.lyrics_lines.len();
        let top = self.lyrics_scroll.min(len.saturating_sub(visible));
        let active = self.lyrics_active_line();
        let y0 = y + hdr + self.scale.s(3.0);

        for j in 0..visible {
            let i = top + j;
            let Some(l) = self.lyrics_lines.get(i) else { break };
            let rj = y0 + j as f32 * row_h;
            let key = crate::shell::LYRICS_KEY_BASE + j as u32;
            let hov = self.hover_key == key;
            let is_active = active == Some(i);
            let row_half = row_h - self.scale.s(4.0);
            if is_active {
                v.push(Cmd::Rect { x: x0, y: rj + self.scale.s(1.0), w: ww, h: row_half, r: self.scale.s(4.0), color: pal.acc });
            } else if hov {
                v.push(Cmd::Rect { x: x0, y: rj + self.scale.s(1.0), w: ww, h: row_half, r: self.scale.s(4.0), color: ui::hover(&pal) });
            }
            let fg = if is_active { 0xff141414 } else if hov { pal.acc } else { pal.fg };
            // WORD KARAOKE on the active line: split the line into words and
            // highlight the one the playhead has reached (estimated across the
            // line's time window). x is approximated per-word so the columns
            // track the real glyphs closely enough for a sung line.
            if is_active && crate::lyrics::has_timing(&self.lyrics_lines) {
                let pos = self.media_pos_now() as f64 / 1e6;
                let cur = crate::lyrics::active_word(&self.lyrics_lines, pos, i).unwrap_or(0);
                let fs = self.scale.fs(10.5);
                let est_w = |s: &str| -> f32 {
                    s.chars().map(|c| if c.is_ascii_uppercase() || c.is_ascii_digit() { 0.62 } else if c.is_whitespace() { 0.0 } else { 0.52 }).sum::<f32>() * fs
                };
                let mut wx = x0 + self.scale.s(6.0);
                let mut wi = 0usize;
                for word in l.text.split_whitespace() {
                    let wcol = if wi == cur { 0xffffffffu32 } else { 0x1d1d1du32 };
                    let txt: String = word.chars().take(80).collect();
                    ui::text(v, wx, rj + self.scale.s(0.5), &txt, fs, wcol, false);
                    // underline the live word
                    if wi == cur {
                        let uw = est_w(word);
                        v.push(Cmd::Rect { x: wx, y: rj + row_half - self.scale.s(2.5), w: uw, h: self.scale.s(1.5), r: 0.8, color: 0xffffffff });
                    }
                    wx += est_w(word) + self.scale.s(4.0);
                    wi += 1;
                    if wx > x0 + ww - self.scale.s(8.0) {
                        break;
                    }
                }
            } else {
                let txt: String = l.text.chars().take(60).collect();
                ui::text(v, x0 + self.scale.s(6.0), rj + self.scale.s(0.5), &txt, self.scale.fs(10.5), fg, false);
            }
            self.region(x0, rj, ww, row_h, key);
        }
    }

}
