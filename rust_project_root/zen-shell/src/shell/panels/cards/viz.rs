use super::super::*;

impl Shell {

    /// VIZ — equalizer bars with a clickable style toggle (rounded bars /
    /// wave / blocks). Fed by `self.viz`, which the app ticker drives from the
    /// live sink monitor capture (real audio, whatever the source — cava-style).
    pub(crate) fn draw_viz_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
if self.card_show_glyph { ui::text(v, x + self.scale.s(12.0), y + self.scale.s(8.0), ICON_NOTE, self.scale.fs(12.0), pal.acc, true); }
if !self.scene_owns_header && self.card_show_title { ui::title(v, if self.card_show_glyph { x + self.scale.s(30.0) } else { x + self.scale.s(12.0) }, y + self.scale.s(9.0), "Visualizer", self.scale.fs(12.0), pal.fg); }
        // style cycle (top-right corner, icon only — no pill bg)
        let hov = self.hover_key == crate::shell::VIZ_KEY_BASE;
        ui::text(v, x + w - self.scale.s(18.0), y + self.scale.s(9.0), ICON_ROWS, self.scale.fs(12.0), if hov { pal.acc } else { pal.fg }, true);
        self.region(x + w - self.scale.s(20.0), y + self.scale.s(4.0), self.scale.s(16.0), self.scale.s(20.0), crate::shell::VIZ_KEY_BASE);

        let n = self.viz.len().min(24);
        let area_x = x + self.scale.s(16.0);
        let area_w = (w - self.scale.s(32.0)).max(20.0);
        let area_top = y + self.scale.s(42.0);
        let area_bot = y + h - self.scale.s(12.0);
        let area_h = (area_bot - area_top).max(12.0);
        let bw = area_w / n as f32;
        match self.viz_style {
        1 => {
            // wave: a filled ribbon
            let mut prev: Option<(f32, f32)> = None;
            for i in 0..n {
                let b = self.viz.get(i).copied().unwrap_or(0.0);
                let px = area_x + i as f32 * bw + bw / 2.0;
                let py = area_top + (area_h - area_h * b);
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
                let bh = area_h * b;
                if bh < 0.5 {
                    continue;
                }
                v.push(Cmd::Rect {
                    x: area_x + i as f32 * bw + self.scale.s(1.0),
                    y: area_top + area_h - bh,
                    w: (bw - self.scale.s(2.0)).max(self.scale.s(1.0)),
                    h: bh,
                    r: self.scale.s(1.0),
                    color: if i % 4 == 0 { pal.acc } else { mix(pal.acc, pal.fg, 0.4) },
                });
            }
        }
        _ => {
            // rounded bars (default) — they rise straight from the ground
            for i in 0..n {
                let b = self.viz.get(i).copied().unwrap_or(0.0);
                let bh = area_h * b;
                if bh < 0.5 {
                    continue;
                }
                let bwv = (bw * 0.5).max(self.scale.s(2.0));
                v.push(Cmd::Rect {
                    x: area_x + i as f32 * bw + (bw - bwv) / 2.0,
                    y: area_top + area_h - bh,
                    w: bwv,
                    h: bh,
                    r: bwv / 2.0,
                    color: if i % 4 == 0 { pal.acc } else { mix(pal.acc, pal.fg, 0.35) },
                });
            }
        }
        }
        let silent = self.viz.iter().take(24).all(|b| *b < 0.04);
        if silent && h >= self.scale.s(90.0) {
            ui::text_c(v, x + w / 2.0, y + h - self.scale.s(24.0), "no audio", self.scale.fs(8.0), ui::fg3(&pal), false);
        }
    }

}
