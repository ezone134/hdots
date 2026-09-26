use super::super::*;

impl Shell {

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

}
