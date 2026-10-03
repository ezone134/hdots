use super::*;

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
                size: art_s,
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
        ui::text(v, info_tx, y + self.scale.s(14.0), title, self.scale.fs(12.0), pal.fg, false);
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


    /// NEWS — clickable headline list with a horizontal feed-category strip.
    /// The strip scrolls horizontally (trackpad two-finger / horizontal wheel)
    /// and each chip filters the vertical list; clicking a headline opens the
    /// article (`xdg-open`). Fed by the app's background fetcher
    /// (`self.news_items`, one category per `[[news_feeds]]`).
    pub(crate) fn draw_news_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        self.news_rect = (x, y, w, h);
        let pad = self.scale.s(12.0);
        let hdr = self.scale.s(Self::NEWS_HDR);
        ui::text(v, x + pad, y + self.scale.s(8.0), ICON_NEWS, self.scale.fs(12.0), pal.acc, true);
        ui::text(v, x + pad + self.scale.s(18.0), y + self.scale.s(9.0), "News", self.scale.fs(12.0), pal.fg, false);
        if !self.news_items.is_empty() && self.news_items.len() > 1 {
            let cats = self.news_cats();
            let label = if self.news_active_cat == 0 {
                format!("{} stories", self.news_filtered_len())
            } else {
                cats.get(self.news_active_cat).cloned().unwrap_or_default()
            };
            ui::text_r(v, x + w - pad, y + self.scale.s(9.0), label, self.scale.fs(8.5), ui::fg3(&pal), false);
        }

        if self.news_items.is_empty() {
            let msg = if self.news_fetched {
                "no feeds — set [[news_feeds]]/news_url or write ~/.cache/zen-shell/news.json"
            } else {
                "fetching headlines…"
            };
            ui::text_c(v, x + w / 2.0, y + h / 2.0 - self.scale.s(6.0), msg, self.scale.fs(9.0), ui::fg3(&pal), false);
            return;
        }

        // ── horizontal category strip (chips) ────────────────────────────
        let cats = self.news_cats();
        let cat_h = if cats.len() > 1 { self.scale.s(Self::NEWS_CAT_H) } else { 0.0 };
        if cat_h > 0.0 {
            let fs = self.scale.fs(8.5);
            let chip_pad = self.scale.s(Self::NEWS_CAT_PAD);
            let sep = self.scale.s(6.0);
            // estimate per-chip width: label glyph-width + padding
            let widths: Vec<f32> = cats
                .iter()
                .map(|c| chip_pad + c.chars().count() as f32 * fs * 0.62 + chip_pad)
                .collect();
            let total: f32 = widths.iter().sum::<f32>() + sep * (widths.len() as f32 - 1.0);
            self.news_cat_content_w = total;
            let avail = (w - pad * 2.0).max(0.0);
            let max_cs = (total - avail).max(0.0);
            self.news_cat_scroll = self.news_cat_scroll.clamp(0.0, max_cs);

            let cy = y + hdr + self.scale.s(1.0);
            let ch = cat_h - self.scale.s(2.0);
            let mut cx = x + pad - self.news_cat_scroll;
            for (i, cat) in cats.iter().enumerate() {
                let cw = widths[i];
                if cx + cw < x + pad - 1.0 || cx > x + w - pad + 1.0 {
                    cx += cw + sep;
                    continue;
                }
                let active = self.news_active_cat == i;
                let key = crate::shell::NEWS_CAT_KEY_BASE + i as u32;
                let hov = self.hover_key == key;
                let bg = if active { pal.acc } else if hov { ui::hover_hl(&pal) } else { ui::hover(&pal) };
                v.push(Cmd::Rect { x: cx, y: cy, w: cw, h: ch, r: ch / 2.0, color: bg });
                let tc = if active { 0xff141414 } else { pal.fg };
                ui::text_c(v, cx + cw / 2.0, cy + ch / 2.0 - self.scale.s(4.0), cat, fs, tc, false);
                self.region(cx, cy, cw, ch, key);
                cx += cw + sep;
            }
        }

        // ── vertical headline list (filtered by active category) ─────────
        let row_h = Self::NEWS_ROW_H;
        let x0 = x + pad;
        let ww = (w - pad * 2.0).max(20.0);
        let visible = self.news_card_visible();
        let len = self.news_filtered_len();
        let top = self.news_scroll.min(len.saturating_sub(visible));
        let y0 = y + hdr + cat_h + self.scale.s(3.0);

        for j in 0..visible {
            let i = top + j;
            if i >= len {
                break;
            }
            let rj = y0 + j as f32 * row_h;
            let key = crate::shell::NEWS_HEAD_KEY_BASE + j as u32;
            let hov = self.hover_key == key;
            if let Some(item) = self.news_item_at(i) {
                // hover accent bar + title tint
                if hov {
                    v.push(Cmd::Rect { x: x0, y: rj + self.scale.s(2.0), w: self.scale.s(2.5), h: row_h - self.scale.s(6.0), r: 1.0, color: pal.acc });
                }
                let title: String = item.title.chars().take(40).collect();
                ui::text(v, x0 + self.scale.s(5.0), rj + self.scale.s(2.0), title, self.scale.fs(9.5), if hov { pal.acc } else { pal.fg }, false);
                // right side: open-glyph when clickable + source label
                let mut rx = x + w - pad;
                if !item.url.trim().is_empty() {
                    rx -= self.scale.s(13.0);
                    ui::text(v, rx, rj + self.scale.s(2.0), ICON_OPEN, self.scale.fs(8.0), if hov { pal.acc } else { ui::fg3(&pal) }, false);
                }
                if !item.source.is_empty() {
                    let src: String = item.source.chars().take(16).collect();
                    ui::text_r(v, rx, rj + self.scale.s(14.0), &src, self.scale.fs(7.5), ui::fg3(&pal), false);
                }
                v.push(Cmd::Rect { x: x0, y: rj + row_h - 1.0, w: ww, h: 1.0, r: 0.5, color: ui::hover(&pal) });
                self.region(x0, rj, ww, row_h, key);
            }
        }
    }


    /// LYRICS — the currently playing track's lyrics (LRCLib + local cache).
    /// Synced lines highlight the sung line and auto-scroll with the
    /// playhead; wheel scrolls the list when it outgrows the card.
    pub(crate) fn draw_lyrics_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        self.lyrics_rect = (x, y, w, h);
        let pad = self.scale.s(12.0);
        let hdr = self.scale.s(Self::LYRICS_HDR);
        // header: glyph + title + state chip
        ui::text(v, x + pad, y + self.scale.s(8.0), ICON_NOTE, self.scale.fs(12.0), pal.acc, true);
        ui::text(v, x + pad + self.scale.s(18.0), y + self.scale.s(9.0), "Lyrics", self.scale.fs(12.0), pal.fg, false);
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


    /// VIZ — equalizer bars with a clickable style toggle (rounded bars /
    /// wave / blocks). Fed by `self.viz` (app ticker, live while playing).
    pub(crate) fn draw_viz_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        ui::text(v, x + self.scale.s(12.0), y + self.scale.s(8.0), ICON_NOTE, self.scale.fs(12.0), pal.acc, true);
        ui::text(v, x + self.scale.s(30.0), y + self.scale.s(9.0), "Visualizer", self.scale.fs(12.0), pal.fg, false);
        // style toggle chip (right of the header)
        let style_names = ["bars", "wave", "blocks"];
        let sname = style_names[self.viz_style as usize % 3];
        let sx = x + w - self.scale.s(64.0);
        let sw = self.scale.s(52.0);
        let sh = self.scale.s(20.0);
        let hov = self.hover_key == crate::shell::VIZ_KEY_BASE;
        v.push(Cmd::Rect {
            x: sx,
            y: y + self.scale.s(6.0),
            w: sw,
            h: sh,
            r: sh / 2.0,
            color: if hov { ui::hover_hl(&pal) } else { ui::hover(&pal) },
        });
        ui::text_c(v, sx + sw / 2.0, y + self.scale.s(8.0), sname, self.scale.fs(9.0), if hov { pal.acc } else { pal.fg }, false);
        self.region(sx - self.scale.s(3.0), y + self.scale.s(4.0), sw + self.scale.s(6.0), sh + self.scale.s(4.0), crate::shell::VIZ_KEY_BASE);

        let play = self.media_playing;
        // animated bars only while audio actually plays — when paused the card
        // goes fully static (no tick, no bar motion, just a flat baseline)
        let n = self.viz.len().min(24);
        let area_x = x + self.scale.s(16.0);
        let area_w = (w - self.scale.s(32.0)).max(20.0);
        let area_top = y + self.scale.s(42.0);
        let area_bot = y + h - self.scale.s(12.0);
        let area_h = (area_bot - area_top).max(12.0);
        let bw = area_w / n as f32;
        if !play {
            // static flat baseline — nothing moves, nothing re-randomizes
            let yb = area_top + area_h * 0.5;
            v.push(Cmd::Line {
                x0: area_x,
                y0: yb,
                x1: area_x + area_w,
                y1: yb,
                w: self.scale.s(1.0),
                color: ui::fg3(&pal),
            });
        } else {
            match self.viz_style {
            1 => {
                // wave: a filled ribbon
                let mut prev: Option<(f32, f32)> = None;
                for i in 0..n {
                    let b = self.viz.get(i).copied().unwrap_or(0.0);
                    let px = area_x + i as f32 * bw + bw / 2.0;
                    let py = area_top + (area_h - area_h * b * 0.9);
                    if let Some((ox, oy)) = prev {
                        v.push(Cmd::Line { x0: ox, y0: oy, x1: px, y1: py, w: self.scale.s(2.0), color: pal.acc });
                    }
                    prev = Some((px, py));
                }
            }
            2 => {
                // blocks: thicker stepped bars with hard square caps
                for i in 0..n {
                    let b = self.viz.get(i).copied().unwrap_or(0.0);
                    let btop = area_top + area_h - area_h * b;
                    v.push(Cmd::Rect {
                        x: area_x + i as f32 * bw + self.scale.s(1.0),
                        y: btop,
                        w: (bw - self.scale.s(2.0)).max(self.scale.s(1.0)),
                        h: (area_h * b).max(self.scale.s(2.0)),
                        r: self.scale.s(1.0),
                        color: if i % 4 == 0 { pal.acc } else { mix(pal.acc, pal.fg, 0.4) },
                    });
                }
            }
            _ => {
                // rounded bars (default)
                for i in 0..n {
                    let b = self.viz.get(i).copied().unwrap_or(0.0);
                    let btop = area_top + area_h - area_h * b;
                    let bwv = (bw * 0.5).max(self.scale.s(2.0));
                    v.push(Cmd::Rect {
                        x: area_x + i as f32 * bw + (bw - bwv) / 2.0,
                        y: btop,
                        w: bwv,
                        h: (area_h * b).max(self.scale.s(2.0)),
                        r: bwv / 2.0,
                        color: if i % 4 == 0 { pal.acc } else { mix(pal.acc, pal.fg, 0.35) },
                    });
                }
            }
        }
        }
        if !play && h >= self.scale.s(90.0) {
            ui::text_c(v, x + w / 2.0, y + h - self.scale.s(24.0), "no audio playing", self.scale.fs(8.0), ui::fg3(&pal), false);
        }
    }


    /// Shared battery-card chrome: header, no-battery guard. Returns the
    /// charge % (or None when no battery is present).
    pub(crate) fn battery_frame(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal, title: &str) -> Option<i32> {
        self.load_power_save();
        let pad = self.scale.s(12.0);
        ui::text(v, x + pad, y + self.scale.s(8.0), ICON_BATTERY, self.scale.fs(12.0), pal.acc, true);
        ui::text(v, x + pad + self.scale.s(17.0), y + self.scale.s(9.0), title, self.scale.fs(12.0), pal.fg, false);
        let pct = self.battery;
        if pct < 0 {
            ui::text_c(v, x + w / 2.0, y + h / 2.0 - self.scale.s(4.0), "no battery", self.scale.fs(9.5), ui::fg3(&pal), false);
            return None;
        }
        Some(pct)
    }

    /// Shared battery card pane 1 (extra info) with the power-save toggle.
    pub(crate) fn battery_info_pane(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal, pct: i32) {
        let pad = self.scale.s(12.0);
        let hdr = self.scale.s(24.0);
        let dots_w = 2.0 * self.scale.s(7.0) + pad;
        let rows: Vec<(String, String)> = vec![
            ("State".into(), if self.ac_online { "Charging \u{26a1}".into() } else { "Discharging".into() }),
            ("Battery".into(), format!("{pct}%")),
            ("Draw".into(), if self.battery_watts > 0.0 { format!("{:.1} W", self.battery_watts) } else { "\u{2014}".into() }),
            ("Remaining".into(), if self.battery_time.is_empty() { "\u{2014}".into() } else { self.battery_time.clone() }),
        ];
        let mut ry = y + hdr + self.scale.s(6.0);
        for (lbl, val) in &rows {
            if ry + self.scale.s(18.0) > y + h - self.scale.s(26.0) {
                break;
            }
            ui::text(v, x + pad, ry, lbl, self.scale.fs(9.0), ui::fg3(&pal), false);
            ui::text_r(v, x + w - pad - dots_w, ry, val, self.scale.fs(9.5), pal.fg, false);
            ry += self.scale.s(18.0);
        }
        // power-save toggle (bottom-left, above the dots)
        let key = crate::shell::BATTERY_PSAVE_KEY;
        let psv = self.power_save_on;
        let bh = self.scale.s(22.0);
        let bw2 = (w - pad * 2.0 - dots_w).max(self.scale.s(50.0));
        let bxx = x + pad;
        let byy = y + h - bh - self.scale.s(2.0);
        let hov = self.hover_key == key;
        v.push(Cmd::Rect {
            x: bxx,
            y: byy,
            w: bw2,
            h: bh,
            r: bh / 2.0,
            color: if psv { ui::acc_tint(&pal) } else if hov { ui::hover_hl(&pal) } else { ui::hover(&pal) },
        });
        let glyph = if psv { ICON_REFRESH } else { ICON_BATTERY_SAVER };
        let lbl = if psv { "Power save on" } else { "Power save" };
        ui::text(v, bxx + self.scale.s(24.0), byy + self.scale.s(6.5), format!("{glyph} {lbl}"), self.scale.fs(9.0), if psv { pal.acc } else { pal.fg }, false);
        self.region(bxx - self.scale.s(3.0), byy - self.scale.s(3.0), bw2 + self.scale.s(6.0), bh + self.scale.s(6.0), key);
    }
}

/// Fill color for a charge level (shared by both battery cards).
pub(crate) fn battery_level_color(pal: &Pal, pct: i32) -> u32 {
    const GREEN: u32 = 0x2ee6a8ff;
    const RED: u32 = 0xff6b6bff;
    if pct >= 50 {
        GREEN
    } else if pct >= 20 {
        pal.acc
    } else {
        RED
    }
}

/// Pagination dots under a multi-pane card (hard rule, see NEXT_STEPS.md).
/// One dot per pane; the active pane is filled with the accent color. The
/// dots appear ONLY while the cursor rests on the card (hidden otherwise so
/// the gauge stays clean).
pub(crate) fn battery_dots(v: &mut Vec<Cmd>, pal: &Pal, x: f32, y: f32, w: f32, h: f32, active: usize, step: f32, show: bool) {
    if !show {
        return;
    }
    let dot = step * 0.66;
    let gap = step;
    let n = 2usize;
    let total = n as f32 * gap;
    let dx = x + (w - total) / 2.0;
    let dy = y + h - dot - step * 0.5;
    for i in 0..n {
        let c = if i == active { pal.acc } else { ui::hover(&pal) };
        v.push(Cmd::Rect { x: dx + i as f32 * gap, y: dy, w: dot, h: dot, r: dot / 2.0, color: c });
    }
}

/// A proper battery drawn from vector shapes — not a font glyph. Case =
/// rounded outline, terminal nub on the free end, fill bar inset from the
/// case, colored by charge level. `vertical` = standing battery (nub on
/// top, fill climbs up), else flat battery (nub on the right, fill grows
/// left→right).
pub(crate) fn push_battery_icon(v: &mut Vec<Cmd>, pal: &Pal, cx: f32, cy: f32, w: f32, h: f32, pct: i32, vertical: bool) {
    let line = ui::fg2(pal);
    let body = ui::hover(pal);
    let fill_c = battery_level_color(pal, pct);
    let fill = (pct as f32 / 100.0).clamp(0.0, 1.0);
    if vertical {
        // terminal nub on top
        let nub_w = w * 0.5;
        let nub_h = h * 0.05;
        v.push(Cmd::Rect {
            x: cx - nub_w / 2.0, y: cy - h / 2.0 - nub_h, w: nub_w, h: nub_h, r: nub_h / 2.0, color: line,
        });
        // case outline
        let r = w * 0.16;
        v.push(Cmd::Outline { x: cx - w / 2.0, y: cy - h / 2.0, w, h, r, width: w.clamp(1.0, 2.2), color: line });
        // inner body + fill
        let inset = w * 0.14;
        let fx = cx - w / 2.0 + inset;
        let fw = w - inset * 2.0;
        let fh = (h - inset * 2.0) * fill;
        if fh > 1.0 {
            v.push(Cmd::Rect { x: fx, y: cy - h / 2.0 + inset + (h - inset * 2.0 - fh), w: fw, h: fh, r: r * 0.7, color: fill_c });
        } else {
            v.push(Cmd::Rect { x: fx, y: cy - h / 2.0 + inset, w: fw, h: (h - inset * 2.0).max(2.0), r: r * 0.7, color: body });
        }
        // inner body behind the fill, so low levels still show the case
        if fill < 1.0 {
            v.push(Cmd::Rect {
                x: fx, y: cy - h / 2.0 + inset, w: fw, h: (h - inset * 2.0 - fh).max(1.0), r: r * 0.7, color: body,
            });
        }
    } else {
        // terminal nub on the right
        let nub_w = w * 0.05;
        let nub_h = h * 0.5;
        v.push(Cmd::Rect {
            x: cx + w / 2.0, y: cy - nub_h / 2.0, w: nub_w, h: nub_h, r: nub_w / 2.0, color: line,
        });
        let r = h * 0.16;
        v.push(Cmd::Outline { x: cx - w / 2.0, y: cy - h / 2.0, w, h, r, width: h.clamp(1.0, 2.2), color: line });
        let inset = h * 0.14;
        let fy = cy - h / 2.0 + inset;
        let fh = h - inset * 2.0;
        let fw = (w - inset * 2.0) * fill;
        if fw > 1.0 {
            v.push(Cmd::Rect { x: cx - w / 2.0 + inset, y: fy, w: fw, h: fh, r: r * 0.7, color: fill_c });
        } else {
            v.push(Cmd::Rect { x: cx - w / 2.0 + inset, y: fy, w: (w - inset * 2.0).max(2.0), h: fh, r: r * 0.7, color: body });
        }
        if fill < 1.0 {
            v.push(Cmd::Rect {
                x: cx - w / 2.0 + inset + fw, y: fy, w: (w - inset * 2.0 - fw).max(1.0), h: fh, r: r * 0.7, color: body,
            });
        }
    }
}

// ── EQ card ───────────────────────────────────────────────────────────────

impl Shell {
    /// Equalizer card — 10-band parametric EQ with vertical sliders,
    /// frequency response curve, and preset chips.
    pub(crate) fn draw_eq_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        self.eq_rect = (x, y, w, h);
        let pad = self.scale.s(12.0);
        let hdr = self.scale.s(28.0);
        // Snapshot the EQ state so we can call self.region() while drawing
        // (the mutex guard can't outlive a &mut self borrow).
        let (active, bands, preset) = {
            let eq = self.eq.lock().unwrap();
            (eq.active, eq.bands, eq.preset)
        };

        // Header: icon + label + on/off toggle
        ui::text(v, x + pad, y + self.scale.s(8.0), ICON_VOLUME, self.scale.fs(12.0), if active { pal.acc } else { ui::fg3(&pal) }, true);
        ui::text(v, x + pad + self.scale.s(18.0), y + self.scale.s(9.0), "Equalizer", self.scale.fs(12.0), pal.fg, false);
        // on/off toggle chip
        {
            let tx = x + w - pad - self.scale.s(42.0);
            let ty = y + self.scale.s(5.0);
            let tw = self.scale.s(38.0);
            let th = self.scale.s(18.0);
            let hov = self.hover_key == crate::shell::EQ_TOGGLE_KEY;
            let col = if active { pal.acc } else { ui::fg3(&pal) };
            v.push(Cmd::Rect {
                x: tx, y: ty, w: tw, h: th, r: th / 2.0,
                color: if hov { mix(col, pal.fg, 0.2) } else { mix(col, pal.bg, 0.85) },
            });
            let lbl = if active { "ON" } else { "OFF" };
            ui::text_c(v, tx + tw / 2.0, ty + self.scale.s(3.0), lbl, self.scale.fs(8.5), if active { 0xff141414 } else { ui::fg3(&pal) }, false);
            self.region(tx - self.scale.s(2.0), ty - self.scale.s(2.0), tw + self.scale.s(4.0), th + self.scale.s(4.0), crate::shell::EQ_TOGGLE_KEY);
        }

        if !active {
            let msg = "Press ON to enable";
            ui::text_c(v, x + w / 2.0, y + h / 2.0, msg, self.scale.fs(10.0), ui::fg3(&pal), false);
            return;
        }

        // ── Frequency response curve (top section) ──
        let curve_h = self.scale.s(50.0);
        let curve_y = y + hdr + self.scale.s(4.0);
        let curve_x = x + pad;
        let curve_w = w - pad * 2.0;
        // Background
        v.push(Cmd::Rect { x: curve_x, y: curve_y, w: curve_w, h: curve_h, r: self.scale.s(4.0), color: mix(pal.bg, pal.fg, 0.03) });
        // Zero line
        let zero_y = curve_y + curve_h / 2.0;
        v.push(Cmd::Rect { x: curve_x, y: zero_y - 0.5, w: curve_w, h: 1.0, r: 0.5, color: mix(pal.fg, pal.bg, 0.8) });
        // Draw the response curve as connected line segments
        let curve = crate::eq::Eq { bands, active, preset, module_id: None }.response_curve();
        let db_range = crate::eq::GAIN_MAX - crate::eq::GAIN_MIN;
        for i in 1..curve.len() {
            let (x0, y0) = curve[i - 1];
            let (x1, y1) = curve[i];
            let sx0 = curve_x + x0 * curve_w;
            let sy0 = curve_y + (1.0 - (y0 - crate::eq::GAIN_MIN) / db_range) * curve_h;
            let sx1 = curve_x + x1 * curve_w;
            let sy1 = curve_y + (1.0 - (y1 - crate::eq::GAIN_MIN) / db_range) * curve_h;
            v.push(Cmd::Line { x0: sx0, y0: sy0, x1: sx1, y1: sy1, w: self.scale.s(2.0), color: pal.acc });
        }
        // Band dots on the curve
        for (i, freq) in crate::eq::BAND_FREQS.iter().enumerate() {
            let frac = (*freq as f32).log2() / 20000.0_f32.log2(); // log-scale x
            let gain = bands[i];
            let sx = curve_x + frac * curve_w;
            let sy = curve_y + (1.0 - (gain - crate::eq::GAIN_MIN) / db_range) * curve_h;
            v.push(Cmd::Rect { x: sx - 3.0, y: sy - 3.0, w: 6.0, h: 6.0, r: 3.0, color: pal.acc });
        }

        // ── 10-band sliders (below curve) ──
        let slider_y = curve_y + curve_h + self.scale.s(10.0);
        let slider_h = h - (slider_y - y) - self.scale.s(36.0); // leave room for presets
        let band_w = (w - pad * 2.0) / 10.0;
        let band_keys: [u32; 10] = [
            crate::shell::EQ_KEY_BASE,
            crate::shell::EQ_KEY_BASE + 1,
            crate::shell::EQ_KEY_BASE + 2,
            crate::shell::EQ_KEY_BASE + 3,
            crate::shell::EQ_KEY_BASE + 4,
            crate::shell::EQ_KEY_BASE + 5,
            crate::shell::EQ_KEY_BASE + 6,
            crate::shell::EQ_KEY_BASE + 7,
            crate::shell::EQ_KEY_BASE + 8,
            crate::shell::EQ_KEY_BASE + 9,
        ];
        for (i, (_freq, label)) in crate::eq::BAND_FREQS.iter().zip(crate::eq::BAND_LABELS.iter()).enumerate() {
            let bx = x + pad + i as f32 * band_w;
            let cy = slider_y + slider_h / 2.0;
            let gain = bands[i];
            // Map gain (-12..+12) to fraction (0..1)
            let frac = ((gain - crate::eq::GAIN_MIN) / (crate::eq::GAIN_MAX - crate::eq::GAIN_MIN)).clamp(0.0, 1.0);
            let hov = self.hover_key == band_keys[i];
            // Draw vertical fader
            let fader_x = bx + band_w / 2.0;
            let fader_w = self.scale.s(6.0);
            let groove_y = slider_y + self.scale.s(8.0);
            let groove_h = slider_h - self.scale.s(16.0);
            // groove
            v.push(Cmd::Rect { x: fader_x - fader_w / 2.0, y: groove_y, w: fader_w, h: groove_h, r: fader_w / 2.0, color: ui::hover(pal) });
            // fill from bottom
            let fill_h = groove_h * frac;
            if fill_h > 1.0 {
                v.push(Cmd::Rect {
                    x: fader_x - fader_w / 2.0, y: groove_y + groove_h - fill_h, w: fader_w, h: fill_h, r: fader_w / 2.0,
                    color: if hov { pal.acc } else { mix(pal.acc, pal.fg, 0.2) },
                });
            }
            // thumb
            let thumb_y = groove_y + groove_h - fill_h - self.scale.s(6.0);
            v.push(Cmd::Rect {
                x: fader_x - self.scale.s(8.0), y: thumb_y, w: self.scale.s(16.0), h: self.scale.s(4.0), r: self.scale.s(2.0),
                color: if hov { pal.fg } else { pal.acc },
            });
            // label below
            ui::text_c(v, fader_x, groove_y + groove_h + self.scale.s(4.0), (*label).to_string(), self.scale.fs(7.5), ui::fg3(&pal), false);
            // hit region
            self.region(bx, slider_y, band_w, slider_h + self.scale.s(16.0), band_keys[i]);
        }

        // ── Preset chips (bottom) ──
        let preset_y = y + h - self.scale.s(28.0);
        let mut px = x + pad;
        for (i, preset_def) in crate::eq::PRESETS.iter().enumerate() {
            let key = crate::shell::EQ_PRESET_BASE + i as u32;
            let tw = preset_def.name.len() as f32 * self.scale.s(7.5) + self.scale.s(14.0);
            let th = self.scale.s(18.0);
            let hov = self.hover_key == key;
            let selected = preset == i;
            let col = if selected { pal.acc } else { ui::fg3(&pal) };
            v.push(Cmd::Rect {
                x: px, y: preset_y, w: tw, h: th, r: th / 2.0,
                color: if hov { mix(col, pal.fg, 0.2) } else if selected { mix(col, pal.bg, 0.85) } else { mix(pal.fg, pal.bg, 0.92) },
            });
            ui::text_c(v, px + tw / 2.0, preset_y + self.scale.s(3.0), preset_def.name, self.scale.fs(8.0), if selected { 0xff141414 } else { ui::fg3(&pal) }, false);
            self.region(px - self.scale.s(2.0), preset_y - self.scale.s(2.0), tw + self.scale.s(4.0), th + self.scale.s(4.0), key);
            px += tw + self.scale.s(4.0);
        }
    }


    /// WALLPAPER — a scrollable thumbnail grid inside the card. Thumbnails
    /// come from the shared `self.wallpapers` cache; wheel scrolls the card
    /// (one row per notch), clicking a tile queues `set_wall {path}`.
    pub(crate) fn draw_wallpaper_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        // remember this frame's rect so the wheel handler can hit-test it
        self.wp_card_rect = (x, y, w, h);

        // header: glyph + title left, live count right
        let hdr = self.wp_card_header_h();
        let pad = self.wp_card_pad();
        ui::text(v, x + pad, y + self.scale.s(8.0), ICON_WALLPAPER, self.scale.fs(12.0), pal.acc, true);
        ui::text(v, x + pad + self.scale.s(18.0), y + self.scale.s(9.0), "Backgrounds", self.scale.fs(12.0), pal.fg, false);
        let n = self.wallpapers.len();
        if n > 0 {
            ui::text_r(v, x + w - pad - self.scale.s(12.0), y + self.scale.s(9.0), format!("{} wallpapers", n), self.scale.fs(8.5), ui::fg3(&pal), false);
        }

        if self.wallpapers.is_empty() {
            ui::text_c(v, x + w / 2.0, y + h - self.scale.s(18.0), "No images here", self.scale.fs(9.5), ui::fg3(&pal), false);
            return;
        }

        // adaptive cell: shrinks so ≥5 columns and ≥2 rows always fit, hiding
        // the card with a proper mini-grid instead of a sparse single row
        let gap = self.wp_card_gap();
        let cell = self.wp_card_fit_cell(w, h);
        self.wp_card_cell_eff = cell;

        let cols = self.wp_card_cols(w).max(1);
        let rows = self.wp_card_rows(h).max(1);
        // clamp the scroll so the last page sits flush with the card bottom
        let max_scroll = self.wp_card_max_scroll(cols, rows);
        self.wp_card_scroll = self.wp_card_scroll.min(max_scroll);

        // clip-safe rows: never tile past the card boundary (guards odd sizes)
        let fit_rows = ((h - hdr - self.scale.s(6.0) + gap) / (cell + gap)).floor() as usize;
        let rows = rows.min(fit_rows).max(1);
        let max_scroll = self.wp_card_max_scroll(cols, rows);
        self.wp_card_scroll = self.wp_card_scroll.min(max_scroll);

        let files = self.wallpapers.clone();
        let start = self.wp_card_scroll;
        let visible_cells = cols * rows;
        let grid_w = cols as f32 * cell + (cols as f32 - 1.0) * gap;
        let x0 = x + (w - grid_w) / 2.0;
        let gy0 = y + hdr + self.scale.s(2.0);

        for (vi, path) in files.iter().enumerate().skip(start).take(visible_cells) {
            // position from the LOCAL tile index — `vi` already includes the
            // scroll offset, so using it directly pushes every page down
            // below the card the moment you scroll
            let t = vi - start;
            let cx = x0 + (t as f32 % cols as f32) * (cell + gap);
            let cy = gy0 + (t as f32 / cols as f32).floor() * (cell + gap);
            let key = crate::shell::WP_CARD_KEY_BASE + vi as u32;
            let hov = self.hover_key == key;
            v.push(Cmd::Rect {
                x: cx,
                y: cy,
                w: cell,
                h: cell,
                r: self.scale.s(10.0),
                color: if hov { ui::hover_hl(&pal) } else { ui::hover(&pal) },
            });
            v.push(Cmd::Image {
                x: cx + self.scale.s(3.0),
                y: cy + self.scale.s(3.0),
                size: cell - self.scale.s(6.0),
                key: format!("thumb:{path}"),
            });
            self.region(cx, cy, cell, cell, key);
        }

        // right-edge scrollbar (page-jump regions + proportional thumb) —
        // only when the grid is actually scrollable
        if max_scroll > 0 {
            self.draw_wp_card_scrollbar(v, x, y, w, h, cols, rows, cell, pal);
        }
    }


    fn draw_wp_card_scrollbar(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, _cols: usize, rows: usize, _cell: f32, pal: &Pal) {
        let total_rows = Self::wp_card_total_rows(self.wallpapers.len(), self.wp_card_cols(w));
        let track_x = x + w - self.scale.s(9.0);
        let track_y = y + self.wp_card_header_h() + self.scale.s(6.0);
        let track_h = (h - self.wp_card_header_h() - self.scale.s(12.0)).max(self.scale.s(10.0));
        let track_w = self.scale.s(4.0);
        // track
        v.push(Cmd::Rect { x: track_x, y: track_y, w: track_w, h: track_h, r: self.scale.s(2.0), color: mix(ui::hover(&pal), pal.fg, 0.08) });
        // thumb
        let span = total_rows.saturating_sub(rows).max(1) as f32;
        let frac = (self.wp_card_scroll as f32 / self.wp_card_cols(w) as f32) / span;
        let thumb_h = (track_h * rows as f32 / total_rows as f32).clamp(self.scale.s(10.0), track_h);
        let thumb_y = track_y + (track_h - thumb_h) * frac.clamp(0.0, 1.0);
        let sb_hover = self.hover_key == crate::shell::WP_CARD_SB_UP || self.hover_key == crate::shell::WP_CARD_SB_DOWN;
        v.push(Cmd::Rect {
            x: track_x - self.scale.s(1.0),
            y: thumb_y,
            w: track_w + self.scale.s(2.0),
            h: thumb_h,
            r: self.scale.s(3.0),
            color: if sb_hover { pal.acc } else { mix(pal.acc, ui::hover(&pal), 0.45) },
        });
        // page-jump regions: above the thumb → page up, below → page down
        let up_h = (thumb_y - track_y - self.scale.s(2.0)).max(0.0);
        let down_y = (thumb_y + thumb_h + self.scale.s(2.0)).min(track_y + track_h);
        let down_h = (track_y + track_h - down_y).max(0.0);
        let hit_w = track_w + self.scale.s(6.0);
        if up_h >= self.scale.s(4.0) {
            self.region(track_x - self.scale.s(1.0), track_y, hit_w, up_h, crate::shell::WP_CARD_SB_UP);
        }
        if down_h >= self.scale.s(4.0) {
            self.region(track_x - self.scale.s(1.0), down_y, hit_w, down_h, crate::shell::WP_CARD_SB_DOWN);
        }
    }

}
