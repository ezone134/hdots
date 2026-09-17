use super::super::*;
use super::shared::*;

impl Shell {

    /// WATER — daily hydration tracker: a bead-ring gauge (reuses the CPU
    /// card's `ring_beads` style) with the glass count + liters readout in
    /// the middle, `−` undo and `+` glass buttons below. One glass = 250 ml.
    /// Count auto-resets at midnight (checked on every interaction and on
    /// load); goal is adjustable by scrolling over the card.
    pub(crate) fn draw_water_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        self.roll_water_day();
        self.water_rect = (x, y, w, h);
        let pad = self.scale.s(12.0);
        let hdr = self.scale.s(24.0);
        let liters = self.water_glasses as f32 * 0.25;
        let goal_l = self.water_goal as f32 * 0.25;
        let meta = format!("{:.1} / {:.1} L", liters, goal_l);
        card_header(v, pal, x, y, w, pad, "Water", Some(ICON_BRIGHTNESS_FILL), Some((&meta, false)), self.scale, self.card_show_title, self.card_show_glyph);
        // bead ring — lit beads follow the fraction of the goal (capped so an
        // over-goal count keeps the ring full instead of wrapping)
        let frac = (self.water_glasses as f32 / self.water_goal.max(1) as f32).min(1.0);
        let done = frac >= 1.0;
        let body_top = y + hdr;
        let body_h = y + h - body_top - self.scale.s(38.0);
        let r = (body_h / 2.0).clamp(self.scale.s(20.0), self.scale.s(34.0));
        let cx = x + w / 2.0;
        let cy = body_top + body_h / 2.0;
        let dot_d = if r >= self.scale.s(26.0) { self.scale.s(4.0) } else { self.scale.s(3.5) };
        let dim = mix(ui::hover(&pal), pal.fg, 0.10);
        let lit = (frac * 36.0).round() as i32;
        let ring_col = if done { ui::OK } else { ui::INFO };
        Self::ring_beads(v, cx, cy, r, dot_d, lit, ring_col, dim);
        // center readout: glasses + a % caption
        let fs = if r >= self.scale.s(26.0) { self.scale.fs(14.0) } else { self.scale.fs(12.0) };
        ui::text_c_hero(v, cx, cy - self.scale.s(6.0), format!("{}{}", self.water_glasses, if done { " ✓" } else { "" }), fs, if done { ui::OK } else { pal.fg });
        ui::caption_c(v, cx, cy + self.scale.s(8.0), format!("of {} glasses", self.water_goal), self.scale.fs(7.5), ui::fg3(&pal), false);
        // − undo · + glass buttons
        let bw = self.scale.s(46.0);
        let bh = self.scale.s(22.0);
        let by = y + h - bh - self.scale.s(10.0);
        self.draw_edit_lbl_btn(v, cx - bw - self.scale.s(6.0), by, bw, bh, WATER_DEC_KEY, "−", cursor_px_of(self), pal, false);
        self.draw_edit_lbl_btn(v, cx + self.scale.s(6.0), by, bw, bh, WATER_INC_KEY, "+ glass", cursor_px_of(self), pal, true);
    }

}

/// Cursor passthrough for the shared edit-button helpers.
fn cursor_px_of(s: &Shell) -> Option<(f32, f32)> {
    s.cursor
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
    fn add_sub_and_goal_roundtrip() {
        let mut s = shell();
        s.per_state_config = std::env::temp_dir().join("zen-test-water.toml");
        s.water_glasses = 0;
        s.water_add();
        s.water_add();
        assert_eq!(s.water_glasses, 2);
        s.water_sub();
        assert_eq!(s.water_glasses, 1);
        s.water_set_goal(10);
        assert_eq!(s.water_goal, 10);
        s.water_set_goal(999);
        assert_eq!(s.water_goal, 30, "goal clamps at 30");
    }

    #[test]
    fn card_draws_and_registers_buttons() {
        let mut s = shell();
        s.mode = crate::shell::Mode::Expanded;
        let p = pal();
        let mut v: Vec<Cmd> = Vec::new();
        s.draw_water_card(&mut v, 0.0, 0.0, 180.0, 110.0, &p);
        let keys: Vec<u32> = s.hover_regions.iter().map(|r| r.4).collect();
        assert!(keys.contains(&WATER_INC_KEY), "+ glass registered");
        assert!(keys.contains(&WATER_DEC_KEY), "− undo registered");
    }
}
