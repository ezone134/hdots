use super::super::*;

impl Shell {

    /// WORKSPACES — Hyprland workspace tiles. One tile per workspace (1-based
    /// id, from `ws_count`); click switches (`pending_ws_dispatch`), the
    /// active one is accent-tinted with a dot below, exactly like the pill's
    /// long-form workspaces item. Tile grid adapts: 5 per row max.
    pub(crate) fn draw_workspaces_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        let pad = self.scale.s(12.0);
        let hdr = self.scale.s(24.0);
        let n = self.ws_count.max(1);
        let meta = format!("{}/{n}", (self.ws_active + 1).min(n));
        self.card_head(v, pal, x, y, w, pad, "Workspaces", Some(ICON_GRID), Some((&meta, false)));
        let body_w = w - pad * 2.0;
        let body_h = y + h - (y + hdr + self.scale.s(6.0)) - self.scale.s(6.0);
        let cols = ((body_w / self.scale.s(34.0)).floor() as usize).clamp(1, 10).min(n);
        let rows = ((n + cols - 1) / cols).max(1);
        let gap = self.scale.s(6.0);
        // tiles fill the body; caps at 10 per row for giant workspace counts
        let tw = ((body_w - (cols as f32 - 1.0) * gap) / cols as f32).min(self.scale.s(48.0));
        let th = if rows > 1 { ((body_h - (rows as f32 - 1.0) * gap) / rows as f32).min(self.scale.s(40.0)) } else { body_h.min(self.scale.s(40.0)) };
        let grid_w = cols as f32 * tw + (cols as f32 - 1.0) * gap;
        let grid_h = rows as f32 * th + (rows as f32 - 1.0) * gap;
        let gx0 = x + pad + (body_w - grid_w) / 2.0;
        let gy0 = y + hdr + self.scale.s(6.0) + (body_h - grid_h) / 2.0;
        for i in 0..n {
            let (r, c) = (i / cols, i % cols);
            let tx = gx0 + c as f32 * (tw + gap);
            let ty = gy0 + r as f32 * (th + gap);
            let active = i == self.ws_active;
            let key = WORKSPACE_KEY_BASE + i as u32;
            let hov = self.hover_key == key;
            let bg = if active {
                (pal.acc & 0xFFFF_FF00) | 0x28
            } else if hov {
                ui::hover_hl(&pal)
            } else {
                ui::hover(&pal)
            };
            v.push(Cmd::Rect { x: tx, y: ty, w: tw, h: th, r: self.scale.s(7.0), color: bg });
            let lbl = (i + 1).to_string();
            let fs = if tw >= self.scale.s(30.0) { self.scale.fs(11.0) } else { self.scale.fs(9.0) };
            ui::text_c(v, tx + tw / 2.0, ty + th / 2.0 - fs / 2.0 - self.scale.s(1.0), &lbl, fs, if active { pal.acc } else { pal.fg }, false);
            if active {
                v.push(Cmd::Rect { x: tx + tw / 2.0 - self.scale.s(1.5), y: ty + th - self.scale.s(6.0), w: self.scale.s(3.0), h: self.scale.s(3.0), r: self.scale.s(1.5), color: pal.acc });
            }
            self.region(tx, ty, tw, th, key);
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
    fn workspaces_card_draws_and_registers_tiles() {
        let mut s = shell();
        let p = pal();
        s.ws_count = 6;
        s.ws_active = 2;
        let mut v: Vec<Cmd> = Vec::new();
        s.draw_workspaces_card(&mut v, 0.0, 0.0, 220.0, 110.0, &p);
        // one hit region per workspace tile
        let keys: Vec<u32> = s.hover_regions.iter().map(|r| r.4).collect();
        for i in 0..6usize {
            assert!(keys.contains(&(WORKSPACE_KEY_BASE + i as u32)), "tile {} registered", i + 1);
        }
    }

    #[test]
    fn tile_click_dispatches_workspace_switch() {
        let mut s = shell();
        s.mode = crate::shell::Mode::Expanded;
        s.ws_count = 5;
        s.ws_active = 0;
        let key = WORKSPACE_KEY_BASE + 3; // 4th tile
        // click() hit-tests by coordinates — seed the region directly
        s.hover_regions.clear();
        s.hover_regions.push((0.0, 0.0, 40.0, 40.0, key));
        let mode = s.click(20.0, 20.0);
        assert!(mode.is_none(), "tile click stays on the dashboard");
        assert_eq!(s.pending_ws_dispatch, Some(4), "1-indexed ws id queued");
        assert_eq!(s.ws_active, 3);
    }
}
