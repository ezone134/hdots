use super::super::*;

impl Shell {

    /// COMPOSITOR / EFFECTS — live blur / shadow / opacity toggle tiles
    /// (state from the `dash_status` extension) + two screenshot buttons
    /// (area / full) wired to `shot_main`. Tiles spawn the `*_main`
    /// helpers via `DashCmd`; the screenshot buttons set `pending_shot`.
    pub(crate) fn draw_compositor_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        let pad = self.scale.s(12.0);
        let hdr = self.scale.s(24.0);
        let on_n = [self.fx_blur_on, self.fx_shadow_on, self.fx_opacity_on].iter().filter(|b| **b).count();
        let meta = format!("{on_n} on");
        self.card_head(v, pal, x, y, w, pad, "Effects", Some(ICON_WIDGETS), Some((&meta, false)));
        // three equal toggle tiles
        let tiles: [(&str, bool, u32); 3] = [
            ("Blur", self.fx_blur_on, COMP_BLUR_KEY),
            ("Shadow", self.fx_shadow_on, COMP_SHADOW_KEY),
            ("Opacity", self.fx_opacity_on, COMP_OPACITY_KEY),
        ];
        let gap = self.scale.s(8.0);
        let tw = (w - pad * 2.0 - gap * 2.0) / 3.0;
        let th = self.scale.s(40.0);
        let ty = y + hdr + self.scale.s(6.0);
        for (i, (label, on, key)) in tiles.iter().enumerate() {
            let tx = x + pad + i as f32 * (tw + gap);
            let hov = self.hover_key == *key;
            let bg = if *on {
                if hov { mix(ui::hover(&pal), pal.acc, 0.25) } else { (pal.acc & 0xFFFF_FF00) | 0x28 }
            } else if hov {
                ui::hover_hl(&pal)
            } else {
                ui::hover(&pal)
            };
            v.push(Cmd::Rect { x: tx, y: ty, w: tw, h: th, r: self.scale.s(7.0), color: bg });
            // state dot on top, label under
            v.push(Cmd::Rect {
                x: tx + tw / 2.0 - self.scale.s(2.5),
                y: ty + self.scale.s(7.0),
                w: self.scale.s(5.0),
                h: self.scale.s(5.0),
                r: self.scale.s(2.5),
                color: if *on { pal.acc } else { ui::fg3(&pal) },
            });
            ui::text_c(v, tx + tw / 2.0, ty + self.scale.s(17.0), *label, self.scale.fs(9.5), if *on { pal.acc } else { pal.fg }, false);
            ui::caption_c(v, tx + tw / 2.0, ty + self.scale.s(29.0), if *on { "on" } else { "off" }, self.scale.fs(7.5), ui::fg3(&pal), false);
            self.region(tx, ty, tw, th, *key);
        }
        // screenshot row: area + full
        let sy = ty + th + gap;
        let sw = (w - pad * 2.0 - gap) / 2.0;
        let sh = self.scale.s(30.0);
        if sy + sh <= y + h - 2.0 {
            let shots: [(&str, &str, u32); 2] = [
                (ICON_SELECT, "Area", COMP_SHOT_AREA_KEY),
                (ICON_CAMERA, "Full", COMP_SHOT_FULL_KEY),
            ];
            for (i, (glyph, label, key)) in shots.iter().enumerate() {
                let sx = x + pad + i as f32 * (sw + gap);
                let hov = self.hover_key == *key;
                v.push(Cmd::Rect { x: sx, y: sy, w: sw, h: sh, r: self.scale.s(7.0), color: if hov { ui::hover_hl(&pal) } else { ui::hover(&pal) } });
                ui::text(v, sx + sw / 2.0 - self.scale.s(30.0), sy + self.scale.s(8.0), *glyph, self.scale.fs(11.0), pal.fg, true);
                ui::text(v, sx + sw / 2.0 + self.scale.s(2.0), sy + self.scale.s(9.0), *label, self.scale.fs(9.5), pal.fg, false);
                self.region(sx, sy, sw, sh, *key);
            }
        }
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    fn shell() -> Shell {
        let cfg: crate::config::Config =
            toml::from_str(crate::config::DEFAULT_SHELL_TOML).expect("default config parses");
        Shell::new(cfg)
    }

    fn pal() -> Pal {
        Pal { fg: 0xe8e6e3ff, bg: 0x141414ff, acc: 0x4fc2ffff, sfg: 0x101010ff }
    }

    #[test]
    fn compositor_card_registers_tiles_and_shots() {
        let mut s = shell();
        s.mode = crate::shell::Mode::Expanded;
        s.fx_blur_on = true;
        let p = pal();
        let mut v: Vec<Cmd> = Vec::new();
        s.draw_compositor_card(&mut v, 0.0, 0.0, 240.0, 120.0, &p);
        let keys: Vec<u32> = s.hover_regions.iter().map(|r| r.4).collect();
        for k in [COMP_BLUR_KEY, COMP_SHADOW_KEY, COMP_OPACITY_KEY, COMP_SHOT_AREA_KEY, COMP_SHOT_FULL_KEY] {
            assert!(keys.contains(&k), "key {k} registered");
        }
    }
}
