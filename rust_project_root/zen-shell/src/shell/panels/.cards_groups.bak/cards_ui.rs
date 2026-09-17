use super::*;
use super::cards_media::{
    battery_dots, battery_level_color, push_battery_icon,
};

impl Shell {


        pub(crate) fn draw_todo_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        self.todo_card_rect = (x, y, w, h);
        let tx = x;
        let ty = y;
        let tw_ = w;
        let th_ = h;
        ui::text(v, tx + self.scale.s(14.0), ty + self.scale.s(10.0), "To-Do", self.scale.fs(11.5), pal.fg, false);
        let n = self.todos.len();
        if n > 0 {
            let done_n = self.todos.iter().filter(|(_, d)| *d).count();
            ui::text_r(v, tx + tw_ - self.scale.s(14.0), ty + self.scale.s(12.0), format!("{done_n}/{n}"), self.scale.fs(9.0), pal.fg, false);
        }
        v.push(Cmd::Rect { x: tx + self.scale.s(14.0), y: ty + self.scale.s(28.0), w: tw_ - self.scale.s(28.0), h: self.scale.s(1.0), r: self.scale.s(0.5), color: mix(ui::hover(&pal), pal.fg, 0.08) });
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
        }
        self.region(tx + self.scale.s(10.0), fy - self.scale.s(4.0), tw_ - self.scale.s(20.0), self.scale.s(34.0), TODO_KEY_INPUT);
    }


        pub(crate) fn draw_toggles_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        let cols: usize = if w >= 330.0 { 3 } else if w >= 225.0 { 2 } else { 1 };
        let rows = (6 + cols - 1) / cols;
        let tw_ = (w - self.scale.s(20.0) - (cols as f32 - 1.0) * self.scale.s(8.0)) / cols as f32;
        let th_ = (h - self.scale.s(20.0) - (rows as f32 - 1.0) * self.scale.s(8.0)) / rows as f32;
        let tiles: [(u32, &str, &str, bool); 6] = [
            (41, ICON_WIFI, "Wi-Fi", self.wifi_on),
            (43, ICON_BLUETOOTH, "BT", self.bt_on),
            (45, ICON_SNOW, "DND", self.dnd),
            (46, ICON_CAFFEINE, "Caffeine", self.caffeine_on),
            (47, ICON_MOON, "Sunset", self.sunset_on),
            (48, ICON_SHADER, "Shader", self.shader_on),
        ];
        for (i, &(key, glyph, label, on)) in tiles.iter().enumerate() {
            let tx0 = x + self.scale.s(10.0) + (i % cols) as f32 * (tw_ + self.scale.s(8.0));
            let ty0 = y + self.scale.s(10.0) + (i / cols) as f32 * (th_ + self.scale.s(8.0));
            let cx = tx0 + tw_ / 2.0;
            let hov = self.hover_key == key;
            v.push(Cmd::Rect {
                x: tx0,
                y: ty0,
                w: tw_,
                h: th_,
                r: self.scale.s(12.0),
                color: if on { ui::acc_tint(&pal) } else if hov { ui::hover_hl(&pal) } else { ui::hover(&pal) },
            });
            let labeled = th_ >= self.scale.s(42.0);
            if labeled {
                ui::text_c(v, cx, ty0 + self.scale.s(8.0), glyph, self.scale.fs(13.0), if on { pal.acc } else { pal.fg }, true);
                ui::text_c(v, cx, ty0 + self.scale.s(30.0), label, self.scale.fs(8.5), if on { pal.fg } else { pal.fg }, false);
            } else {
                ui::text_c(v, cx, ty0 + (th_ - self.scale.s(13.0)) / 2.0, glyph, self.scale.fs(13.0), if on { pal.acc } else { pal.fg }, true);
            }
            self.region(tx0, ty0, tw_, th_, key);
            if key == 41 || key == 43 {
                let ck = if key == 41 { 42 } else { 44 };
                let chov = self.hover_key == ck;
                ui::text_c(v, tx0 + tw_ - self.scale.s(10.0), ty0 + th_ - self.scale.s(14.0), ICON_MORE, self.scale.fs(9.0),
                    if chov { pal.acc } else { pal.fg }, true);
                self.region(tx0 + tw_ - self.scale.s(18.0), ty0, self.scale.s(18.0), th_, ck);
            }
        }
    }


        pub(crate) fn draw_sliders_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        let row_h = self.scale.s(24.0);
        let labeled = w >= 300.0;
        // 6 fader rows: brightness, volume, balance, mic, saturation, kelvin
        self.faders = [(0.0, 0.0, 0.0); 5];
        self.balance_rect = (0.0, 0.0, 0.0);
        let bri_lbl = format!("{}%", (self.brightness * 100.0).round() as i32);
        let vol_lbl =
            if self.volume_muted { "Muted".to_string() } else { format!("{}%", (self.volume * 100.0).round() as i32) };
        // balance: 0..1 centered at 0.5 (L heavy → 0, R heavy → 1)
        let lr = self.vol_left + self.vol_right;
        let bal = if lr > 0.001 { (self.vol_right / lr).clamp(0.0, 1.0) } else { 0.5 };
        let mic_lbl =
            if self.mic_muted { "Muted".to_string() } else { format!("{}%", (self.mic_level * 100.0).round() as i32) };
        let sat_lbl = format!("{}", self.saturation);
        let kel_lbl = format!("{}K", self.sunset_kelvin);
        let warm = ((self.sunset_kelvin as f32 - 1200.0) / 5300.0).clamp(0.0, 1.0);
        let blank = "";
        let sliders: [(u32, f32, &str, &str, u32); 6] = [
            (51, self.brightness.clamp(0.0, 1.0), ICON_BRIGHTNESS, if labeled { &bri_lbl } else { blank }, pal.acc),
            (
                52,
                self.volume,
                if self.volume_muted { ICON_MUTE } else { ICON_VOLUME },
                if labeled { &vol_lbl } else { blank },
                if self.volume_muted { RED } else { pal.acc },
            ),
            (59, bal, blank, blank, pal.acc),
            (
                53,
                self.mic_level,
                if self.mic_muted { ICON_MIC_OFF } else { ICON_MIC },
                if labeled { &mic_lbl } else { blank },
                if self.mic_muted { RED } else { pal.acc },
            ),
            (54, self.saturation as f32 / 200.0, ICON_DROP, if labeled { &sat_lbl } else { blank }, pal.acc),
            (55, warm, ICON_THERMAL, if labeled { &kel_lbl } else { blank }, mix(0xffb86cff, pal.acc, warm)),
        ];
        let n_fit = (((h - self.scale.s(16.0)) / row_h).floor() as usize).min(6);
        // centre the fitted rows vertically instead of pushing them to the top
        let top = y + ((h - n_fit as f32 * row_h) / 2.0).max(self.scale.s(8.0));
        // reset chip geometry (registered after its row so it wins the hit test)
        let mut reset_region: Option<(f32, f32, f32, f32)> = None;
        let mut sat_reset_region: Option<(f32, f32, f32, f32)> = None;
        for (i, &(key, val, glyph, label, fill)) in sliders.iter().enumerate() {
            if i >= n_fit {
                break;
            }
            let ry = top + i as f32 * row_h;
            let cy = ry + row_h * 0.45;
            let hov = self.hover_key == key;
            ui::text(
                v,
                x + self.scale.s(14.0),
                cy - self.scale.s(6.0),
                glyph,
                self.scale.fs(12.0),
                if hov { fill } else { pal.fg },
                true,
            );
            let tx = x + self.scale.s(34.0);
            let tw = (w - self.scale.s(34.0) - if labeled { self.scale.s(52.0) } else { self.scale.s(14.0) }).max(self.scale.s(40.0));
            if key == 59 {
                // balance row: L/R flank the slider, borderless reset icon far right
                ui::text(v, x + self.scale.s(14.0), cy - self.scale.s(5.0), "L", self.scale.fs(9.0), pal.fg, true);
                let rsw = self.scale.s(14.0);
                let rx = x + w - self.scale.s(14.0) - rsw;
                let twb = (rx - self.scale.s(20.0) - tx).max(self.scale.s(40.0)).min(tw);
                ui::fader_bal(v, tx, cy, twb, val.clamp(0.0, 1.0), fill, hov, pal);
                self.balance_rect = (tx, cy, twb);
                ui::text(v, tx + twb + self.scale.s(6.0), cy - self.scale.s(5.0), "R", self.scale.fs(9.0), pal.fg, true);
                let rb_hov = self.hover_key == 58;
                ui::text_c(v, rx + rsw * 0.5, cy - self.scale.s(4.0), ICON_SLIDER, self.scale.fs(9.5), if rb_hov { pal.acc } else { pal.fg }, true);
                reset_region = Some((rx - 2.0, cy - self.scale.s(9.0), rsw + 4.0, self.scale.s(18.0)));
            } else {
                if key == 54 {
                    // saturation row: centered fader + numeric label + reset chip (key 79)
                    ui::fader_bal(v, tx, cy, tw, val.clamp(0.0, 1.0), fill, hov, pal);
                    let rsw = self.scale.s(14.0);
                    let rx = x + w - self.scale.s(14.0) - rsw;
                    if labeled {
                        ui::text_r(v, rx - self.scale.s(8.0), cy - self.scale.s(5.0), label, self.scale.fs(9.5), pal.fg, false);
                    }
                    let rb_hov = self.hover_key == 79;
                    ui::text_c(v, rx + rsw * 0.5, cy - self.scale.s(4.0), ICON_SLIDER, self.scale.fs(9.5), if rb_hov { pal.acc } else { pal.fg }, true);
                    sat_reset_region = Some((rx - 2.0, cy - self.scale.s(9.0), rsw + 4.0, self.scale.s(18.0)));
                } else {
                    ui::fader(v, tx, cy, tw, val.clamp(0.0, 1.0), fill, hov, pal);
                }
                self.faders[(key - 51) as usize] = (tx, cy, tw);
                if labeled && key != 54 {
                    ui::text_r(v, x + w - self.scale.s(14.0), cy - self.scale.s(5.0), label, self.scale.fs(9.5), pal.fg, false);
                }
            }
            self.region(x + self.scale.s(6.0), ry, w - self.scale.s(12.0), row_h, key);
            if let Some((rx, rdy, rw, rh)) = reset_region {
                self.region(rx, rdy, rw, rh, 58);
                reset_region = None;
            }
            if let Some((rx, rdy, rw, rh)) = sat_reset_region {
                self.region(rx, rdy, rw, rh, 79);
                sat_reset_region = None;
            }
        }
    }


    pub(crate) fn draw_clipboard_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        self.clip_card_rect = (x, y, w, h);
        ui::text(v, x + self.scale.s(14.0), y + self.scale.s(10.0), "Clipboard", self.scale.fs(11.5), pal.fg, false);
        let entries = self.clip_text.clone();
        let row_h = self.scale.s(22.0);
        let visible = self.clip_card_visible().min(entries.len());
        let start = self.clip_scroll.min(entries.len().saturating_sub(visible));
        for j in 0..visible {
            let i = start + j;
            let ry = y + self.scale.s(32.0) + j as f32 * row_h;
            let key = crate::shell::CLIP_KEY_BASE + j as u32;
            let label: String = entries[i].chars().take(32).collect();
            let is_sel = self.clip_sel == i;
            let is_hov = self.hover_key == key;
            if is_sel || is_hov {
                v.push(Cmd::Rect { x: x + self.scale.s(10.0), y: ry - self.scale.s(2.0), w: w - self.scale.s(20.0), h: self.scale.s(20.0), r: self.scale.s(6.0), color: ui::hover_hl(&pal) });
            }
            let col = if is_sel {
                pal.acc
            } else if is_hov {
                ui::hover_fg(&pal)
            } else {
                pal.fg
            };
            ui::text(v, x + self.scale.s(14.0), ry, &label, self.scale.fs(9.0), col, false);
            self.region(x + self.scale.s(10.0), ry - self.scale.s(2.0), w - self.scale.s(20.0), self.scale.s(20.0), key);
        }
        if entries.is_empty() {
            ui::text_c(v, x + w / 2.0, y + h / 2.0, "Empty", self.scale.fs(9.5), pal.fg, false);
        }
    }


    pub(crate) fn draw_clipimg_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        self.clipimg_card_rect = (x, y, w, h);
        ui::text(v, x + self.scale.s(14.0), y + self.scale.s(10.0), "Clipboard images", self.scale.fs(11.5), pal.fg, false);
        let entries = self.clip_images.clone();
        let row_h = self.scale.s(28.0);
        let visible = self.clipimg_card_visible().min(entries.len());
        let start = self.clipimg_scroll.min(entries.len().saturating_sub(visible));
        for j in 0..visible {
            let i = start + j;
            let ry = y + self.scale.s(32.0) + j as f32 * row_h;
            let key = crate::shell::CLIPIMG_KEY_BASE + j as u32;
            let name: String = entries[i].chars().take(28).collect();
            let is_sel = self.clip_sel == i;
            let is_hov = self.hover_key == key;
            if is_sel || is_hov {
                v.push(Cmd::Rect { x: x + self.scale.s(10.0), y: ry - self.scale.s(2.0), w: w - self.scale.s(20.0), h: row_h - self.scale.s(4.0), r: self.scale.s(6.0), color: ui::hover_hl(&pal) });
            }
            let col = if is_sel { pal.acc } else if is_hov { ui::hover_fg(&pal) } else { pal.fg };
            v.push(Cmd::Rect { x: x + self.scale.s(14.0), y: ry - self.scale.s(1.0), w: self.scale.s(24.0), h: self.scale.s(22.0), r: self.scale.s(5.0), color: ui::hover(&pal) });
            v.push(Cmd::Image { x: x + self.scale.s(17.0), y: ry + self.scale.s(2.0), size: self.scale.s(18.0), key: entries[i].clone() });
            ui::text(v, x + self.scale.s(46.0), ry + self.scale.s(5.0), &name, self.scale.fs(8.5), col, false);
            self.region(x + self.scale.s(10.0), ry - self.scale.s(2.0), w - self.scale.s(20.0), row_h - self.scale.s(4.0), key);
        }
        if entries.is_empty() {
            ui::text_c(v, x + w / 2.0, y + h / 2.0, "No images copied yet", self.scale.fs(9.5), pal.fg, false);
        }
    }


    pub(crate) fn draw_notes_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        self.notes_card_rect = (x, y, w, h);
        self.load_notes_if_needed();
        let composing = self.notes_input.is_some();
        ui::text(v, x + self.scale.s(14.0), y + self.scale.s(10.0), "Notes", self.scale.fs(11.5), pal.fg, false);
        ui::text_r(v, x + w - self.scale.s(14.0), y + self.scale.s(11.0), format!("{}", self.notes.len()), self.scale.fs(8.5), ui::fg3(&pal), false);
        if composing {
            // composer panel: title / body inputs + hint
            let py = y + self.scale.s(30.0);
            let ph = (h - self.scale.s(30.0) - self.scale.s(6.0)).max(20.0);
            v.push(Cmd::Rect {
                x: x + self.scale.s(8.0),
                y: py,
                w: w - self.scale.s(16.0),
                h: ph,
                r: self.scale.s(10.0),
                color: mix(ui::hover(&pal), pal.fg, 0.06),
            });
            let field_h = (ph - self.scale.s(30.0)) / 2.0;
            let title = self.notes_title.clone();
            let body = self.notes_body.clone();
            for (fi, (name, text)) in [("Title", title.as_str()), ("Body", body.as_str())].into_iter().enumerate() {
                let fy0 = py + self.scale.s(8.0) + fi as f32 * field_h;
                let active = self.notes_input == Some(fi as u8);
                ui::text(v, x + self.scale.s(20.0), fy0 + self.scale.s(2.0), name, self.scale.fs(8.0), ui::fg3(&pal), false);
                let iy = fy0 + self.scale.s(13.0);
                v.push(Cmd::Rect {
                    x: x + self.scale.s(18.0),
                    y: iy,
                    w: w - self.scale.s(36.0),
                    h: self.scale.s(18.0),
                    r: self.scale.s(6.0),
                    color: if active { mix(ui::hover(&pal), pal.acc, 0.12) } else { ui::hover(&pal) },
                });
                if active {
                    v.push(Cmd::Outline {
                        x: x + self.scale.s(18.0), y: iy, w: w - self.scale.s(36.0), h: self.scale.s(18.0),
                        r: self.scale.s(6.0), width: self.scale.s(1.2), color: pal.acc,
                    });
                }
                let shown: String = if text.is_empty() && !active {
                    format!("{name}…")
                } else {
                    text.chars().take(40).collect()
                };
                let cwx = shown.chars().count() as f32 * self.scale.fs(9.0) * 0.62;
                ui::text(v, x + self.scale.s(26.0), iy + self.scale.s(4.0), shown, self.scale.fs(9.0), if active || text.is_empty() { pal.fg } else { ui::fg2(&pal) }, false);
                if active {
                    v.push(Cmd::Rect { x: x + self.scale.s(25.0) + cwx, y: iy + self.scale.s(5.0), w: self.scale.s(1.4), h: self.scale.s(11.0), r: self.scale.s(0.7), color: pal.acc });
                }
            }
            ui::text(v, x + self.scale.s(20.0), py + ph - self.scale.s(16.0), "↵ next field · ↵ save · esc cancel", self.scale.fs(7.5), ui::fg3(&pal), false);
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
            ui::text(v, x + self.scale.s(14.0), ry, "\u{f0f6} ", self.scale.fs(9.0), ui::fg3(&pal), true);
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


    /// CALENDAR — compact month with prev/next paging; clicking a day opens
    /// the full calendar panel with that day selected.
    pub(crate) fn draw_calendar_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        let (first_wday, days, today, month) = self.calendar_info();
        let hdr = 24.0;
        // header: date on the top-left, prev/next chevrons grouped top-right
        ui::text(v, x + self.scale.s(12.0), y + self.scale.s(4.0), month, self.scale.fs(11.0), pal.acc, false);
        let nav_w = self.scale.s(22.0);
        let nav_h = self.scale.s(20.0);
        let gap = self.scale.s(4.0);
        let py = y + (hdr - nav_h) / 2.0;
        let gx = x + w - self.scale.s(12.0) - nav_w * 2.0 - gap;
        let hov_prev = self.hover_key == crate::shell::CAL_KEY_BASE_PREV;
        let hov_next = self.hover_key == crate::shell::CAL_KEY_BASE_NEXT;
        if hov_prev {
            v.push(Cmd::Rect { x: gx, y: py, w: nav_w, h: nav_h, r: nav_h / 2.0, color: ui::hover_hl(&pal) });
        }
        ui::text_c(v, gx + nav_w / 2.0, py + self.scale.s(1.0), ICON_BACK, self.scale.fs(10.0), if hov_prev { pal.acc } else { pal.fg }, true);
        if hov_next {
            v.push(Cmd::Rect { x: gx + nav_w + gap, y: py, w: nav_w, h: nav_h, r: nav_h / 2.0, color: ui::hover_hl(&pal) });
        }
        ui::text_c(v, gx + nav_w + gap + nav_w / 2.0, py + self.scale.s(1.0), ICON_FORWARD, self.scale.fs(10.0), if hov_next { pal.acc } else { pal.fg }, true);
        self.region(gx - self.scale.s(2.0), py - self.scale.s(2.0), nav_w + self.scale.s(4.0), nav_h + self.scale.s(4.0), crate::shell::CAL_KEY_BASE_PREV);
        self.region(gx + nav_w + gap - self.scale.s(2.0), py - self.scale.s(2.0), nav_w + self.scale.s(4.0), nav_h + self.scale.s(4.0), crate::shell::CAL_KEY_BASE_NEXT);

        // fit 6 rows × 7 cols into the remaining card height
        let gy = y + hdr;
        let gh = (h - hdr).max(60.0);
        let cell_w = w / 7.0;
        let wd = ["S", "M", "T", "W", "T", "F", "S"];
        for (i, d) in wd.iter().enumerate() {
            ui::text_c(v, x + i as f32 * cell_w + cell_w / 2.0, gy + 2.0, *d, self.scale.fs(8.0), ui::fg3(&pal), false);
        }
        // weekday label row consumes the top of the grid
        let dy = gy + 16.0;
        let dcell_h = (gh - 16.0) / 6.0;
        for cell in 0..42 {
            let day = cell - first_wday + 1;
            if day < 1 || day > days {
                continue;
            }
            let cx = x + (cell % 7) as f32 * cell_w;
            let cy = dy + (cell / 7) as f32 * dcell_h;
            let key = crate::shell::CAL_KEY_BASE + cell as u32;
            let is_today = self.cal_offset == 0 && day == today;
            let hov = self.hover_key == key;
            if hov {
                v.push(Cmd::Rect { x: cx + 1.0, y: cy + 1.0, w: cell_w - 2.0, h: dcell_h - 2.0, r: self.scale.s(8.0), color: ui::hover(&pal) });
            }
            if is_today {
                v.push(Cmd::Rect { x: cx + 1.0, y: cy + 1.0, w: cell_w - 2.0, h: dcell_h - 2.0, r: self.scale.s(8.0), color: ui::acc_tint(&pal) });
            }
            let color = if is_today { pal.acc } else if hov { pal.fg } else { ui::fg2(&pal) };
            ui::text_c(v, cx + cell_w / 2.0, cy + dcell_h / 2.0 - self.scale.s(5.0), day.to_string(), self.scale.fs(9.0), color, false);
            self.region(cx, cy, cell_w, dcell_h, key);
        }
    }


    /// ACCENT — pick the accent source (from wallpaper / scheme default /
    /// custom accent). Mirrors the Settings Appearance block — defined-hex is
    /// merged into the custom-acc logic, so only three sources are offered.
    pub(crate) fn draw_accent_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        ui::text(v, x + self.scale.s(12.0), y + self.scale.s(8.0), ICON_STICKY, self.scale.fs(12.0), pal.acc, true);
        ui::text(v, x + self.scale.s(30.0), y + self.scale.s(9.0), "Theme & Accent", self.scale.fs(12.0), pal.fg, false);
        // color scheme chip: shows the current scheme; click opens the picker
        let sname = if self.cur_scheme.is_empty() { "default".to_string() } else { self.cur_scheme.clone() };
        let ctext = format!("Scheme: {sname}");
        let chov = self.hover_key == 28;
        let cy = y + self.scale.s(26.0);
        v.push(Cmd::Rect {
            x: x + self.scale.s(10.0),
            y: cy,
            w: w - self.scale.s(20.0),
            h: self.scale.s(26.0),
            r: self.scale.s(8.0),
            color: if chov { ui::hover_hl(&pal) } else { ui::hover(&pal) },
        });
        ui::text(v, x + self.scale.s(20.0), cy + self.scale.s(7.0), ctext, self.scale.fs(10.0), if chov { pal.acc } else { pal.fg }, false);
        ui::text_r(v, x + w - self.scale.s(18.0), cy + self.scale.s(6.0), ICON_FORWARD, self.scale.fs(11.0), ui::fg2(&pal), true);
        self.region(x + self.scale.s(10.0), cy - self.scale.s(2.0), w - self.scale.s(20.0), self.scale.s(30.0), 28);
        // accent source picker (below the scheme chip). Tapping the `>`
        // chevron on "Custom accent" opens the one-column custom-accents
        // subview with a `<` back button.
        let cur = self.current_acc_source();
        let cur_name = self.current_acc_name();
        let rh = self.scale.s(30.0);
        let rows_top = cy + self.scale.s(32.0);
        if self.accent_list_open {
            self.load_custom_accs();
            // back row: `<` + title
            let back_key = crate::shell::ACC_LIST_BACK_KEY;
            let back_hov = self.hover_key == back_key;
            v.push(Cmd::Rect {
                x: x + self.scale.s(10.0),
                y: rows_top,
                w: w - self.scale.s(20.0),
                h: self.scale.s(26.0),
                r: self.scale.s(9.0),
                color: if back_hov { ui::hover_hl(&pal) } else { ui::hover(&pal) },
            });
            ui::text(v, x + self.scale.s(24.0), rows_top + self.scale.s(6.5), ICON_BACK, self.scale.fs(10.0), if back_hov { pal.acc } else { pal.fg }, true);
            ui::text(v, x + self.scale.s(38.0), rows_top + self.scale.s(6.5), "Custom accents", self.scale.fs(10.0), pal.fg, false);
            ui::text_r(v, x + w - self.scale.s(20.0), rows_top + self.scale.s(6.5), format!("{}", self.custom_accs.len()), self.scale.fs(9.0), ui::fg3(&pal), false);
            self.region(x + self.scale.s(10.0), rows_top - self.scale.s(2.0), w - self.scale.s(20.0), self.scale.s(30.0), back_key);
            // one-column paged list
            let ly = rows_top + self.scale.s(32.0);
            let lh = (y + h - self.scale.s(4.0) - ly).max(8.0);
            let row_h = self.scale.s(26.0);
            let visible = ((lh - self.scale.s(4.0)) / row_h).floor().max(1.0) as usize;
            let len = self.custom_accs.len();
            self.accent_list_scroll = self.accent_list_scroll.min(len.saturating_sub(visible));
            let top = self.accent_list_scroll;
            let x0 = x + self.scale.s(10.0);
            let ww = w - self.scale.s(20.0);
            for j in 0..visible {
                let i = top + j;
                let Some(a) = self.custom_accs.get(i) else { break };
                let rj = ly + j as f32 * row_h;
                let key = crate::shell::ACC_LIST_KEY_BASE + j as u32;
                let hov = self.hover_key == key;
                let sel = cur == "c" && !cur_name.is_empty() && a.name == cur_name;
                if sel || hov {
                    v.push(Cmd::Rect {
                        x: x0, y: rj + 1.0, w: ww, h: row_h - 3.0, r: self.scale.s(8.0),
                        color: if sel { ui::acc_tint(&pal) } else { ui::hover(&pal) },
                    });
                }
                let col = self.accent_display(a);
                v.push(Cmd::Rect {
                    x: x0 + self.scale.s(9.0),
                    y: rj + row_h / 2.0 - self.scale.s(5.0),
                    w: self.scale.s(10.0),
                    h: self.scale.s(10.0),
                    r: self.scale.s(5.0),
                    color: col,
                });
                let shown: String = a.name.chars().take(28).collect();
                ui::text(v, x0 + self.scale.s(28.0), rj + self.scale.s(6.5), shown, self.scale.fs(9.5), if sel || hov { pal.acc } else { pal.fg }, false);
                if sel {
                    ui::text_r(v, x + w - self.scale.s(20.0), rj + self.scale.s(6.5), ICON_CHECK, self.scale.fs(9.0), pal.acc, true);
                }
                self.region(x0, rj, ww, row_h, key);
            }
            self.accent_list_rect = (x0, ly, ww, lh);
            if self.custom_accs.is_empty() {
                ui::text(v, x + self.scale.s(20.0), rows_top + self.scale.s(54.0), "no custom accents — run gen_custom_acc_files", self.scale.fs(8.5), ui::fg3(&pal), false);
            }
            return;
        }
        self.accent_list_rect = (0.0, 0.0, 0.0, 0.0);
        let rows: [(&str, &str, bool); 3] = [
            ("w", "From wallpaper", cur == "w"),
            ("d", "Scheme default", cur == "d"),
            ("c", "Custom accent", cur == "c"),
        ];
        let mut ry = rows_top;
        for (i, &(key_src, name, active)) in rows.iter().enumerate() {
            if ry + rh > y + h - self.scale.s(3.0) {
                break;
            }
            let key = crate::shell::ACC_KEY_BASE + i as u32;
            let hov = self.hover_key == key;
            v.push(Cmd::Rect {
                x: x + self.scale.s(10.0),
                y: ry,
                w: w - self.scale.s(20.0),
                h: rh - self.scale.s(4.0),
                r: self.scale.s(9.0),
                color: if active { ui::acc_tint(&pal) } else if hov { ui::hover_hl(&pal) } else { ui::hover(&pal) },
            });
            let dot = pal.acc;
            v.push(Cmd::Rect {
                x: x + self.scale.s(20.0),
                y: ry + (rh - 4.0) / 2.0 - self.scale.s(4.5),
                w: self.scale.s(9.0),
                h: self.scale.s(9.0),
                r: self.scale.s(4.5),
                color: dot,
            });
            let label = if key_src == "c" && active && !cur_name.is_empty() {
                format!("{name} · {cur_name}")
            } else {
                name.to_string()
            };
            ui::text(v, x + self.scale.s(38.0), ry + self.scale.s(6.5), label, self.scale.fs(10.0), if active { pal.acc } else { pal.fg }, false);
            if active {
                ui::text_r(v, x + w - self.scale.s(20.0), ry + self.scale.s(6.5), ICON_CHECK, self.scale.fs(10.0), pal.acc, true);
            }
            self.region(x + self.scale.s(10.0), ry, w - self.scale.s(20.0), rh - self.scale.s(4.0), key);
            ry += rh;
        }
    }


    /// CLOCK & DATE — nothing but the time, the seconds, and the date. No
    /// actions, no buttons — a clean divider-friendly card.
    pub(crate) fn draw_clock_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        let mut tbuf = [0u8; 16];
        let mut dtbuf = [0u8; 32];
        unsafe {
            let now = libc::time(std::ptr::null_mut());
            let mut tm: libc::tm = std::mem::zeroed();
            libc::localtime_r(&now, &mut tm);
            let n = libc::strftime(tbuf.as_mut_ptr() as *mut libc::c_char, tbuf.len(), c"%H:%M".as_ptr(), &tm);
            let time_s = String::from_utf8_lossy(&tbuf[..n]).into_owned();
            let m = libc::strftime(dtbuf.as_mut_ptr() as *mut libc::c_char, dtbuf.len(), c"%A, %e %b".as_ptr(), &tm);
            let date_s = String::from_utf8_lossy(&dtbuf[..m]).into_owned();
            // big time, centered slightly above the middle
            let size = (h * 0.42).clamp(18.0, 44.0);
            let tw = time_s.chars().count() as f32 * size * 0.62;
            ui::text(v, x + (w - tw) / 2.0, y + (h - size * 2.2) / 2.0, time_s, size, pal.fg, false);
            ui::text_c(v, x + w / 2.0, y + h - self.scale.s(26.0), date_s, self.scale.fs(10.0), ui::fg2(&pal), false);
        }
    }


    /// BATTERY — two-pane card. Pane 0: one vertical + one horizontal charge
    /// gauge and a CPU-governor power-save toggle. Pane 1: extra battery info
    /// (power draw, time remaining, AC state). Horizontal scroll switches
    /// panes; the dots below the card mark the active one.
    /// VERTICAL-BATTERY card — a vector-crafted upright battery icon. Pane 0
    /// = icon + percent, pane 1 = extra info + power-save toggle. Horizontal
    /// scroll flips panes; dots below show the active pane.
/// VERTICAL-BATTERY card — a vector-crafted upright battery icon. Pane 0
    /// = icon + percent, pane 1 = extra info + power-save toggle. Horizontal
    /// scroll flips panes; dots below show the active pane.
    pub(crate) fn draw_battery_v_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        self.battery_v_rect = (x, y, w, h);
        let Some(pct) = self.battery_frame(v, x, y, w, h, pal, "Battery") else {
            return;
        };
        if self.battery_v_pane == 0 {
            // big vertical battery icon, centered in the body
            let aw = (w * 0.30).clamp(16.0, 70.0);
            let ah = (h - self.scale.s(44.0)).max(aw * 1.6);
            let iw = (aw * 2.0 / 3.0).max(12.0);
            push_battery_icon(v, pal, x + w / 2.0, y + self.scale.s(38.0) + (ah - iw * 1.7) / 2.0, iw, iw * 1.7, pct, true);
            let charge = if self.ac_online { " \u{f0e7}" } else { "" };
            ui::text_c(v, x + w / 2.0, y + h - self.scale.s(24.0), format!("{pct}%{charge}"), self.scale.fs(11.0), battery_level_color(&pal, pct), true);
        } else {
            self.battery_info_pane(v, x, y, w, h, pal, pct);
        }
        let hover = !self.dash_edit
            && self
                .cursor
                .map(|(px, py)| Self::in_rect(px, py, (x, y, w, h)))
                .unwrap_or(false);
        battery_dots(v, pal, x, y, w, h, self.battery_v_pane, self.scale.s(7.0), hover);
    }


    /// HORIZONTAL-BATTERY card — a vector-crafted side-laid battery icon.
    /// Same two panes as the vertical card.
    pub(crate) fn draw_battery_h_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        self.battery_h_rect = (x, y, w, h);
        let Some(pct) = self.battery_frame(v, x, y, w, h, pal, "Battery") else {
            return;
        };
        if self.battery_h_pane == 0 {
            let iw = (w * 0.46).clamp(40.0, 220.0);
            push_battery_icon(v, pal, x + w / 2.0, y + self.scale.s(42.0) + (h - self.scale.s(70.0)) / 2.0, iw, (h * 0.36).clamp(16.0, 60.0), pct, false);
            let charge = if self.ac_online { " \u{f0e7}" } else { "" };
            ui::text_c(v, x + w / 2.0, y + h - self.scale.s(22.0), format!("{pct}%{charge}"), self.scale.fs(12.0), battery_level_color(&pal, pct), true);
        } else {
            self.battery_info_pane(v, x, y, w, h, pal, pct);
        }
        let hov = !self.dash_edit
            && self
                .cursor
                .map(|(px, py)| Self::in_rect(px, py, (x, y, w, h)))
                .unwrap_or(false);
        battery_dots(v, pal, x, y, w, h, self.battery_h_pane, self.scale.s(7.0), hov);
    }


    /// APPS — user-pinned app launcher card ("+ + +" empty slate). Up to 8
    /// slots in a width-derived column grid. A filled slot shows the app icon
    /// (image or Nerd fallback) + clipped name and launches on click; hovering
    /// it reveals a ✕ corner that removes the slot. An empty slot is a "+"
    /// tile whose click opens the launcher in pick mode for that slot.
    pub(crate) fn draw_apps_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        use crate::shell::{APP_SHORTCUT_KEY_BASE, APP_SHORTCUT_REMOVE_BASE};
        self.app_shortcut_rect = (x, y, w, h);
        let pad = self.scale.s(12.0);
        let hdr = self.scale.s(28.0);
        // header (mirrors the Eq card): icon + label + count
        ui::text(v, x + pad, y + self.scale.s(8.0), ICON_APPS, self.scale.fs(12.0), pal.acc, true);
        ui::text(v, x + pad + self.scale.s(18.0), y + self.scale.s(9.0), "Apps", self.scale.fs(12.0), pal.fg, false);
        let filled = self.app_shortcuts.iter().filter(|s| !s.is_empty()).count();
        ui::text_r(v, x + w - pad, y + self.scale.s(10.0), format!("{filled}/8"), self.scale.fs(9.0), ui::fg3(&pal), false);
        let sep_y = y + hdr;
        v.push(Cmd::Rect { x: x + pad, y: sep_y, w: w - pad * 2.0, h: self.scale.s(1.0), r: self.scale.s(0.5), color: mix(ui::hover(&pal), pal.fg, 0.08) });

        // column grid from width; rows fill the body, capped at 8 slots total
        let cols: usize = if w >= self.scale.s(330.0) { 4 }
            else if w >= self.scale.s(250.0) { 3 }
            else if w >= self.scale.s(165.0) { 2 }
            else { 1 };
        let gap = self.scale.s(8.0);
        let tw = (w - pad * 2.0 - (cols as f32 - 1.0) * gap) / cols as f32;
        let body_h = (h - hdr - self.scale.s(6.0)).max(self.scale.s(24.0));
        let rows_fit = ((body_h + gap) / (tw + gap)) as usize;
        let rows = rows_fit.min(8usize.div_ceil(cols));
        let slots = (cols * rows).min(8);
        let th = tw;
        for i in 0..slots {
            let col = i % cols;
            let row = i / cols;
            let tx = x + pad + col as f32 * (tw + gap);
            let ty = y + hdr + self.scale.s(8.0) + row as f32 * (th + gap);
            if ty + th > y + h - self.scale.s(4.0) {
                continue;
            }
            let key = APP_SHORTCUT_KEY_BASE + i as u32;
            let rem_key = APP_SHORTCUT_REMOVE_BASE + i as u32;
            let hov = self.hover_key == key || self.hover_key == rem_key;
            let filled = i < self.app_shortcuts.len();
            // tile background
            v.push(Cmd::Rect {
                x: tx,
                y: ty,
                w: tw,
                h: th,
                r: self.scale.s(10.0),
                color: if filled {
                    if hov { ui::hover_hl(&pal) } else { mix(ui::hover(&pal), pal.fg, 0.05) }
                } else if hov {
                    ui::hover_hl(&pal)
                } else {
                    mix(ui::hover(&pal), pal.fg, 0.06)
                },
            });
            if filled {
                let name: String = match self.apps.apps.iter().find(|a| a.id == self.app_shortcuts[i]) {
                    Some(a) => {
                        if let Some(ik) = a.icon_key() {
                            v.push(Cmd::Image { x: tx + (tw - self.scale.s(26.0)) / 2.0, y: ty + self.scale.s(6.0), size: self.scale.s(26.0), key: ik });
                        } else {
                            ui::text_c(v, tx + tw / 2.0, ty + self.scale.s(9.0), ICON_APPS, self.scale.fs(17.0), pal.acc, true);
                        }
                        a.name.clone()
                    }
                    None => {
                        ui::text_c(v, tx + tw / 2.0, ty + self.scale.s(9.0), ICON_APPS, self.scale.fs(17.0), pal.acc, true);
                        self.app_shortcuts[i].clone()
                    }
                };
                let label: String = name.chars().take(((tw - self.scale.s(4.0)) / self.scale.s(5.5)).max(3.0) as usize).collect();
                ui::text_c(v, tx + tw / 2.0, ty + th - self.scale.s(12.0), label, self.scale.fs(8.0), if hov { pal.fg } else { ui::fg2(&pal) }, false);
                // hover-only ✕ corner — removes the slot
                if hov {
                    let rm = (tx + tw - self.scale.s(15.0), ty - self.scale.s(2.0), self.scale.s(18.0), self.scale.s(18.0));
                    let rm_hov = self.hover_key == rem_key;
                    v.push(Cmd::Rect {
                        x: rm.0,
                        y: rm.1,
                        w: rm.2,
                        h: rm.3,
                        r: rm.2 / 2.0,
                        color: if rm_hov { mix(RED, ui::hover(&pal), 0.55) } else { mix(ui::hover(&pal), pal.fg, 0.10) },
                    });
                    ui::text_c(v, rm.0 + rm.2 / 2.0, rm.1 + self.scale.s(2.5), ICON_CLOSE, self.scale.fs(9.0), if rm_hov { RED } else { pal.fg }, true);
                    self.region(rm.0 - self.scale.s(2.0), rm.1 - self.scale.s(2.0), rm.2 + self.scale.s(4.0), rm.3 + self.scale.s(4.0), rem_key);
                }
            } else {
                // empty "+" slate — opens the launcher in pick mode
                ui::text_c(v, tx + tw / 2.0, ty + th / 2.0 - self.scale.s(3.0), "+", self.scale.fs(20.0), if hov { pal.acc } else { ui::fg3(&pal) }, true);
            }
            self.region(tx, ty, tw, th, key);
        }
    }

}
