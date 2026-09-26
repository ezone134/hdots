use super::super::*;
use super::shared::*;

impl Shell {

        /// SLIDERS — two-pane card. Pane 0: the six fader rows; pane 1: the
        /// "show balance" / "show saturation" switches that add/remove rows
        /// from pane 0. The whole body is `sliders.ron` (zero Ink) — this
        /// helper only answers the `Dots` hover gate, and the Rust drawer
        /// below stays the fallback for when no scene is authored.
        pub(crate) fn sliders_dots_hover(&self) -> bool {
            let (x, y, w, h) = self.sliders_rect;
            !self.dash_edit
                && w > 0.0
                && self
                    .cursor
                    .map(|(px, py)| Self::in_rect(px, py, (x, y, w, h)))
                    .unwrap_or(false)
        }

        pub(crate) fn draw_sliders_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        self.sliders_rect = (x, y, w, h);
        if self.sliders_pane == 1 {
            self.draw_sliders_toggles(v, x, y, w, h, pal);
        } else {
            self.draw_sliders_faders(v, x, y, w, h, pal);
        }
        // pagination dots (appear only while the cursor rests on the card)
        let hover = !self.dash_edit
            && self
                .cursor
                .map(|(px, py)| Self::in_rect(px, py, (x, y, w, h)))
                .unwrap_or(false);
        battery_dots(v, pal, x, y, w, h, self.sliders_pane, self.scale.s(7.0), hover);
    }

    /// pane 0: the six fader rows (brightness, volume, balance, mic,
    /// saturation, kelvin) — balance and saturation rows are omitted when
    /// their pane-2 toggles are off.
    fn draw_sliders_faders(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
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
        let all: [(u32, f32, &str, &str, u32); 6] = [
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
            (55, warm, ICON_THERMAL, if labeled { &kel_lbl } else { blank }, mix(ui::WARN, pal.acc, warm)),
        ];
        // filter out rows disabled by the pane-2 toggles (keys stay fixed, so
        // `faders[key-51]` / `balance_rect` geometry stays key-accurate)
        let sliders: Vec<(u32, f32, &str, &str, u32)> = all
            .into_iter()
            .filter(|&(key, ..)| match key {
                59 => self.show_balance,
                54 => self.show_saturation,
                _ => true,
            })
            .collect();
        // keep the bottom band free for the pane dots (mirror card does the same)
        let n_fit = (((h - self.scale.s(16.0) - self.scale.s(16.0)) / row_h).floor() as usize).min(sliders.len());
        // centre the fitted rows vertically instead of pushing them to the top
        let top = y + ((h - self.scale.s(16.0) - n_fit as f32 * row_h) / 2.0).max(self.scale.s(8.0));
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

    /// pane 1: "Show balance" / "Show saturation" switch rows — toggling one
    /// hides its fader row on pane 0 (persisted to the per-channel config).
    fn draw_sliders_toggles(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        let pad = self.scale.s(12.0);
        let top = y + self.scale.s(10.0);
        let bot = y + h - self.scale.s(16.0);
        let row_h = ((bot - top - self.scale.s(6.0)) / 2.0).min(self.scale.s(34.0)).max(self.scale.s(26.0));
        let row_w = w - pad * 2.0;
        let rows: [(u32, &str, bool); 2] = [
            (SLIDERS_TOGGLE_BALANCE_KEY, "Show balance", self.show_balance),
            (SLIDERS_TOGGLE_SAT_KEY, "Show saturation", self.show_saturation),
        ];
        let mut ry = top + self.scale.s(3.0);
        for (key, label, on) in rows {
            let bg = self.hl(key, 0, ui::hover(pal));
            if bg != 0 {
                v.push(Cmd::Rect { x: x + pad, y: ry, w: row_w, h: row_h, r: 5.0, color: bg });
            }
            ui::text(v, x + pad + self.scale.s(14.0), ry + (row_h - self.scale.fs(11.0)) / 2.0,
                label, self.scale.fs(11.0), pal.fg, false);
            // switch track + knob
            let sw_w = self.scale.s(26.0);
            let sw_h = self.scale.s(13.0);
            let swx = x + w - pad - sw_w;
            let swy = ry + (row_h - sw_h) / 2.0;
            let track = ui::mix(pal.bg, pal.fg, if on { 0.45 } else { 0.28 });
            v.push(Cmd::Rect { x: swx, y: swy, w: sw_w, h: sw_h, r: sw_h / 2.0, color: track });
            let kn = sw_h - self.scale.s(4.0);
            let kx = if on { swx + sw_w - kn - self.scale.s(2.0) } else { swx + self.scale.s(2.0) };
            v.push(Cmd::Rect { x: kx, y: swy + self.scale.s(2.0), w: kn, h: kn, r: kn / 2.0, color: pal.fg });
            self.region(x + pad, ry, row_w, row_h, key);
            ry += row_h + self.scale.s(6.0);
        }
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    fn newg() -> crate::shell::Shell {
        let cfg: crate::config::Config = toml::from_str(crate::config::DEFAULT_SHELL_TOML).expect("default parses");
        let mut shell = crate::shell::Shell::new(cfg);
        shell.mode = crate::shell::Mode::Expanded;
        shell.dash_enabled = true;
        shell
    }

    #[test]
    fn sliders_toggles_default_on_and_flip() {
        let mut s = newg();
        assert!(s.show_balance, "balance rows defaults on");
        assert!(s.show_saturation, "saturation row defaults on");
        s.region(100.0, 100.0, 60.0, 30.0, SLIDERS_TOGGLE_BALANCE_KEY);
        s.set_cursor(Some((115.0, 110.0)));
        let _ = s.click(115.0, 110.0);
        assert!(!s.show_balance, "toggle should flip off");
        s.region(100.0, 100.0, 60.0, 30.0, SLIDERS_TOGGLE_SAT_KEY);
        s.set_cursor(Some((115.0, 110.0)));
        let _ = s.click(115.0, 110.0);
        assert!(!s.show_saturation, "toggle should flip off");
    }
}