use super::super::*;

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
if self.card_show_glyph { ui::text(v, x + pad, y + self.scale.s(8.0), ICON_VOLUME, self.scale.fs(12.0), if active { pal.acc } else { ui::fg3(&pal) }, true); }
if !self.scene_owns_header && self.card_show_title { ui::title(v, if self.card_show_glyph { x + pad + self.scale.s(18.0) } else { x + pad }, y + self.scale.s(9.0), "Equalizer", self.scale.fs(12.0), pal.fg); }
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

}
