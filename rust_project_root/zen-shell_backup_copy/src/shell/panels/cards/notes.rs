use super::super::*;

impl Shell {

    /// Shared composer for the Notes / Sticky Notes cards: title + body input
    /// fields with focus, blinking caret, save/next/cancel hint. Both cards
    /// render it dropped into their own card rect so a note edited on either
    /// card lands in the same store.
    /// Word-wrap `text` to `max_chars` per line, returning at most `max_lines`
    /// lines (over-long words are hard-broken so nothing spills the box).
    pub(crate) fn wrap_body(text: &str, max_chars: usize, max_lines: usize) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for para in text.split('\n') {
            let mut cur = String::new();
            for word in para.split_whitespace() {
                let wc = word.chars().count();
                if !cur.is_empty() && cur.chars().count() + 1 + wc > max_chars {
                    out.push(std::mem::take(&mut cur));
                    if out.len() == max_lines {
                        return out;
                    }
                }
                if wc > max_chars {
                    let mut chunk = String::new();
                    for c in word.chars() {
                        chunk.push(c);
                        if chunk.chars().count() == max_chars {
                            out.push(std::mem::take(&mut chunk));
                            if out.len() == max_lines {
                                return out;
                            }
                        }
                    }
                    cur = chunk;
                } else {
                    if !cur.is_empty() {
                        cur.push(' ');
                    }
                    cur.push_str(word);
                }
            }
            if !cur.is_empty() {
                out.push(cur);
                if out.len() == max_lines {
                    return out;
                }
            }
        }
        out
    }

    pub(crate) fn draw_notes_composer(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        let s = self.scale; // GridScale is Copy — copy to avoid holding a borrow of self
        let py = y + s.s(30.0);
        let ph = (h - s.s(30.0) - s.s(6.0)).max(20.0);
        v.push(Cmd::Rect {
            x: x + s.s(8.0),
            y: py,
            w: w - s.s(16.0),
            h: ph,
            r: s.s(10.0),
            color: mix(ui::hover(&pal), pal.fg, 0.06),
        });
        let title = self.notes_title.clone();
        let body = self.notes_body.clone();
        // title field — single line, unchanged behaviour
        let ty = py + s.s(19.0);
        let t_active = self.notes_input == Some(0);
        ui::text(v, x + s.s(20.0), py + s.s(8.0), "Title", s.fs(8.0), ui::fg3(&pal), false);
        v.push(Cmd::Rect {
            x: x + s.s(18.0),
            y: ty,
            w: w - s.s(36.0),
            h: s.s(18.0),
            r: s.s(6.0),
            color: if t_active { mix(ui::hover(&pal), pal.acc, 0.12) } else { ui::hover(&pal) },
        });
        if t_active {
            v.push(Cmd::Outline {
                x: x + s.s(18.0), y: ty, w: w - s.s(36.0), h: s.s(18.0),
                r: s.s(6.0), width: s.s(1.2), color: pal.acc,
            });
        }
        let t_shown: String = if title.is_empty() && !t_active { "Title…".into() } else { title.chars().take(40).collect() };
        let tcw = t_shown.chars().count() as f32 * s.fs(9.0) * 0.62;
        ui::text(v, x + s.s(26.0), ty + s.s(4.0), t_shown, s.fs(9.0), if t_active || title.is_empty() { pal.fg } else { ui::fg2(&pal) }, false);
        if t_active {
            v.push(Cmd::Rect { x: x + s.s(25.0) + tcw, y: ty + s.s(5.0), w: s.s(1.4), h: s.s(11.0), r: s.s(0.7), color: pal.acc });
        }
        // body field — a tall multi-line area instead of a single 18px row
        let b_active = self.notes_input == Some(1);
        let body_label_y = ty + s.s(18.0) + s.s(10.0);
        let by = body_label_y + s.s(11.0);
        let bh = (py + ph - s.s(24.0) - by).max(s.s(34.0));
        ui::text(v, x + s.s(20.0), body_label_y, "Body", s.fs(8.0), ui::fg3(&pal), false);
        v.push(Cmd::Rect {
            x: x + s.s(18.0),
            y: by,
            w: w - s.s(36.0),
            h: bh,
            r: s.s(8.0),
            color: if b_active { mix(ui::hover(&pal), pal.acc, 0.12) } else { ui::hover(&pal) },
        });
        if b_active {
            v.push(Cmd::Outline {
                x: x + s.s(18.0), y: by, w: w - s.s(36.0), h: bh,
                r: s.s(8.0), width: s.s(1.2), color: pal.acc,
            });
        }
        let fs_b = s.fs(9.0);
        let lh = s.s(12.0);
        let max_chars = ((w - s.s(48.0)) / (fs_b * 0.62)).max(12.0) as usize;
        let lines = Self::wrap_body(&body, max_chars, 6);
        if lines.is_empty() {
            if !b_active {
                ui::text(v, x + s.s(26.0), by + s.s(7.0), "Body…", fs_b, ui::fg3(&pal), false);
            } else {
                v.push(Cmd::Rect { x: x + s.s(25.0), y: by + s.s(8.0), w: s.s(1.4), h: s.s(11.0), r: s.s(0.7), color: pal.acc });
            }
        } else {
            for (li, line) in lines.iter().enumerate() {
                let ly = by + s.s(7.0) + li as f32 * lh;
                ui::text(v, x + s.s(26.0), ly, line.clone(), fs_b, if b_active { pal.fg } else { ui::fg2(&pal) }, false);
                if b_active && li == lines.len() - 1 {
                    let cx = x + s.s(25.0) + line.chars().count() as f32 * fs_b * 0.62;
                    v.push(Cmd::Rect { x: cx, y: ly + s.s(1.0), w: s.s(1.4), h: s.s(11.0), r: s.s(0.7), color: pal.acc });
                }
            }
        }
        // save `+` — bottom-right corner of the composer (body stage only)
        if b_active {
            let sbx = x + w - s.s(22.0);
            let sby = py + ph - s.s(18.0);
            let sb_hov = self.hover_key == crate::shell::NOTES_KEY_SAVE;
            v.push(Cmd::Rect {
                x: sbx - s.s(10.0),
                y: sby - s.s(10.0),
                w: s.s(20.0),
                h: s.s(20.0),
                r: s.s(10.0),
                color: if sb_hov { ui::hover_hl(&pal) } else { ui::hover(&pal) },
            });
            ui::text_c(v, sbx, sby - s.s(4.5), ICON_ADD, s.fs(9.5), if sb_hov { pal.acc } else { pal.fg }, true);
            self.region(sbx - s.s(11.0), sby - s.s(11.0), s.s(22.0), s.s(22.0), crate::shell::NOTES_KEY_SAVE);
        }
        ui::text(v, x + s.s(20.0), py + ph - s.s(16.0), "↵ next field · ↵ save · esc cancel", s.fs(7.5), ui::fg3(&pal), false);
    }

    pub(crate) fn draw_notes_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        self.notes_card_rect = (x, y, w, h);
        self.load_notes_if_needed();
        let composing = self.notes_input.is_some();
if self.card_show_title { ui::title(v, x + self.scale.s(14.0), y + self.scale.s(10.0), "Notes", self.scale.fs(11.5), pal.fg); }
        ui::text_r(v, x + w - self.scale.s(14.0), y + self.scale.s(11.0), format!("{}", self.notes.len()), self.scale.fs(8.5), ui::fg3(&pal), false);
        if composing {
            self.draw_notes_composer(v, x, y, w, h, pal);
            return;
        }
        let entries: Vec<(String, String)> = self.notes.clone();
        let row_h = self.scale.s(24.0);
        let visible = self.notes_card_visible();
        let start = self.notes_scroll.min(entries.len().saturating_sub(visible));
        for j in 0..visible {
            let i = start + j;
            let Some((title, body)) = entries.get(i) else { break };
            let ry = y + self.scale.s(32.0) + j as f32 * row_h;
            let title_s: String = title.chars().take(36).collect();
            ui::text(v, x + self.scale.s(14.0), ry, format!("{} ", ICON_BULLET), self.scale.fs(9.0), ui::fg3(&pal), true);
            ui::text(v, x + self.scale.s(28.0), ry, title_s, self.scale.fs(9.5), pal.fg, false);
            if !body.is_empty() {
                let body_s: String = body.chars().take(44).collect();
                ui::text(v, x + self.scale.s(36.0), ry + self.scale.s(12.0), body_s, self.scale.fs(8.0), ui::fg2(&pal), false);
            }
            v.push(Cmd::Rect { x: x + self.scale.s(14.0), y: ry + row_h - self.scale.s(2.0), w: w - self.scale.s(28.0), h: 1.0, r: 0.5, color: ui::hover(&pal) });
        }
        if entries.is_empty() {
            ui::text_c(v, x + w / 2.0, y + h / 2.0 - self.scale.s(10.0), "No notes", self.scale.fs(9.5), pal.fg, false);
        }
        // `+` add button — bottom-right corner
        let pbx = x + w - self.scale.s(20.0);
        let pby = y + h - self.scale.s(18.0);
        let pb_hov = self.hover_key == crate::shell::NOTES_KEY_INPUT;
        v.push(Cmd::Rect {
            x: pbx - self.scale.s(10.0),
            y: pby - self.scale.s(10.0),
            w: self.scale.s(20.0),
            h: self.scale.s(20.0),
            r: self.scale.s(10.0),
            color: if pb_hov { ui::hover_hl(&pal) } else { ui::hover(&pal) },
        });
        ui::text_c(v, pbx, pby - self.scale.s(4.5), ICON_ADD, self.scale.fs(9.5), if pb_hov { pal.acc } else { pal.fg }, true);
        self.region(pbx - self.scale.s(11.0), pby - self.scale.s(11.0), self.scale.s(22.0), self.scale.s(22.0), crate::shell::NOTES_KEY_INPUT);
    }

}
