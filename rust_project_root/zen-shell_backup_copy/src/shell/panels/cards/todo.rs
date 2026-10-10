use super::super::*;

impl Shell {

        pub(crate) fn draw_todo_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        self.todo_card_rect = (x, y, w, h);
        let tx = x;
        let ty = y;
        let tw_ = w;
        let th_ = h;
if self.card_show_title { ui::title(v, tx + self.scale.s(14.0), ty + self.scale.s(10.0), "To-Do", self.scale.fs(11.5), pal.fg); }
        let n = self.todos.len();
        if n > 0 {
            let done_n = self.todos.iter().filter(|(_, d)| *d).count();
            ui::text_r(v, tx + tw_ - self.scale.s(14.0), ty + self.scale.s(12.0), format!("{done_n}/{n}"), self.scale.fs(9.0), pal.fg, false);
        }
        let visible = (((th_ - self.scale.s(76.0)) / self.scale.s(24.0)).floor() as usize).clamp(0, 8);
        if n == 0 {
            ui::text_c(v, tx + tw_ / 2.0, ty + self.scale.s(60.0), "No tasks yet", self.scale.fs(9.5), pal.fg, false);
            ui::text_c(v, tx + tw_ / 2.0, ty + self.scale.s(78.0), ICON_TODO, self.scale.fs(16.0), mix(ui::hover(&pal), pal.fg, 0.10), true);
        }
        let start = self.todo_scroll.min(n.saturating_sub(visible));
        let drawn = visible.min(n.saturating_sub(start));
        for si in 0..drawn {
            let i = start + si;
            let (text, done) = &self.todos[i];
            let row_y = ty + self.scale.s(36.0) + si as f32 * self.scale.s(24.0);
            let tog_key = TODO_KEY_TOGGLE + si as u32;
            let del_key = TODO_KEY_DELETE + si as u32;
            let row_hover = self.hover_key == tog_key || self.hover_key == del_key;
            if *done {
                v.push(Cmd::Rect { x: tx + self.scale.s(14.0), y: row_y + self.scale.s(1.0), w: self.scale.s(11.0), h: self.scale.s(11.0), r: self.scale.s(3.5), color: ui::acc_tint(&pal) });
                ui::text(v, tx + self.scale.s(15.5), row_y + self.scale.s(1.5), ICON_CHECK, self.scale.fs(8.0), pal.acc, true);
            } else {
                v.push(Cmd::Outline {
                    x: tx + self.scale.s(14.0),
                    y: row_y + self.scale.s(1.0),
                    w: self.scale.s(11.0),
                    h: self.scale.s(11.0),
                    r: self.scale.s(3.5),
                    width: self.scale.s(1.2),
                    color: if row_hover { pal.acc } else { mix(ui::hover(&pal), pal.fg, 0.25) },
                });
            }
            let label: String = text.chars().take(((tw_ - self.scale.s(50.0)) / self.scale.s(5.9)).max(8.0) as usize).collect();
            ui::text(v, tx + self.scale.s(32.0), row_y + self.scale.s(1.0), label, self.scale.fs(9.5), if *done { pal.fg } else { pal.fg }, false);
            if row_hover {
                let dhov = self.hover_key == del_key;
                ui::text(v, tx + tw_ - self.scale.s(20.0), row_y + self.scale.s(1.0), ICON_CLOSE, self.scale.fs(9.0),
                    if dhov { RED } else { pal.fg }, true);
            }
            self.region(tx + self.scale.s(10.0), row_y - self.scale.s(4.0), tw_ - self.scale.s(20.0), self.scale.s(22.0), tog_key);
            if row_hover {
                self.region(tx + tw_ - self.scale.s(24.0), row_y - self.scale.s(4.0), self.scale.s(22.0), self.scale.s(22.0), del_key);
            }
        }
        let rem = n.saturating_sub(start + drawn);
        if rem > 0 {
            let fy_ = ty + self.scale.s(36.0) + drawn as f32 * self.scale.s(24.0) + self.scale.s(2.0);
            ui::text_c(v, tx + tw_ / 2.0, fy_, format!("+{rem} more"), self.scale.fs(8.0), pal.fg, false);
        }
        let fy = ty + th_ - self.scale.s(38.0);
        let focused = self.todo_input.is_some();
        let buf = self.todo_input.clone().unwrap_or_default();
        v.push(Cmd::Rect {
            x: tx + self.scale.s(12.0),
            y: fy,
            w: tw_ - self.scale.s(24.0),
            h: self.scale.s(26.0),
            r: self.scale.s(13.0),
            color: mix(ui::hover(&pal), pal.fg, if focused { 0.09 } else { 0.04 }),
        });
        if focused {
            v.push(Cmd::Outline { x: tx + self.scale.s(12.0), y: fy, w: tw_ - self.scale.s(24.0), h: self.scale.s(26.0), r: self.scale.s(13.0), width: self.scale.s(1.4), color: pal.acc });
        }
        if buf.is_empty() && !focused {
            ui::text(v, tx + self.scale.s(24.0), fy + self.scale.s(7.0), "Add a task…", self.scale.fs(9.5), pal.fg, false);
        } else {
            ui::text(v, tx + self.scale.s(24.0), fy + self.scale.s(7.0), buf.clone(), self.scale.fs(9.5), pal.fg, false);
            if focused {
                let cwx = buf.chars().count() as f32 * self.scale.fs(9.5) * 0.62;
                v.push(Cmd::Rect { x: tx + self.scale.s(23.0) + cwx, y: fy + self.scale.s(6.0), w: self.scale.s(1.4), h: self.scale.s(14.0), r: self.scale.s(0.7), color: pal.acc });
                if !buf.trim().is_empty() {
                    ui::text_r(v, tx + tw_ - self.scale.s(22.0), fy + self.scale.s(8.0), ICON_KEYBOARD, self.scale.fs(8.5), pal.fg, true);
                }
            }
        }        self.region(tx + self.scale.s(10.0), fy - self.scale.s(4.0), tw_ - self.scale.s(20.0), self.scale.s(34.0), TODO_KEY_INPUT);
        // `+` add button — bottom-right corner (parity with the Notes card);
        // hidden while the input is focused so it never fights the caret.
        // Registered AFTER the input region so it wins the corner overlap.
        if !focused {
            let pbx = tx + tw_ - self.scale.s(20.0);
            let pby = ty + th_ - self.scale.s(18.0);
            let pb_hov = self.hover_key == TODO_KEY_ADD;
            v.push(Cmd::Rect {
                x: pbx - self.scale.s(10.0),
                y: pby - self.scale.s(10.0),
                w: self.scale.s(20.0),
                h: self.scale.s(20.0),
                r: self.scale.s(10.0),
                color: if pb_hov { ui::hover_hl(&pal) } else { ui::hover(&pal) },
            });
            ui::text_c(v, pbx, pby - self.scale.s(4.5), ICON_ADD, self.scale.fs(9.5), if pb_hov { pal.acc } else { pal.fg }, true);
            self.region(pbx - self.scale.s(11.0), pby - self.scale.s(11.0), self.scale.s(22.0), self.scale.s(22.0), TODO_KEY_ADD);
        }
    }

}
