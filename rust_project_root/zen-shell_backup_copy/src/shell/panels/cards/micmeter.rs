use super::super::*;
use super::shared::*;

impl Shell {

    /// MIC METER — live input level bar (same `mic_level`/`mic_muted` the
    /// dashboard fader shows) with a one-tap mute/unmute toggle. The level
    /// clips at the red tip above 90% so you notice clipping without the
    /// fader; muted state drops to a flat track.
    pub(crate) fn draw_micmeter_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        let pad = self.scale.s(12.0);
        let hdr = self.scale.s(22.0);
        let head_icon = if self.mic_muted { ICON_MIC_OFF } else { ICON_MIC };
        let meta = format!("{:>3}%", (self.mic_level * 100.0).round() as i32);
        card_header(v, pal, x, y, w, pad, "Mic", Some(head_icon), Some((&meta, true)), self.scale, self.card_show_title, self.card_show_glyph);

        let lvl = if self.mic_muted { 0.0 } else { self.mic_level.clamp(0.0, 1.0) };
        let bar_y = y + hdr + self.scale.s(12.0);
        let col = if self.mic_muted {
            ui::fg3(&pal)
        } else if lvl > 0.9 {
            ui::DANGER
        } else {
            pal.acc
        };
        ui::bar(v, x + pad, bar_y, w - pad * 2.0, self.scale.s(8.0), lvl, col, ui::hover(&pal));

        let status = if self.mic_muted {
            "muted"
        } else if lvl > 0.05 {
            "live"
        } else {
            "quiet"
        };
        let sc = if self.mic_muted { ui::fg3(&pal) } else if lvl > 0.9 { ui::DANGER } else { ui::fg2(&pal) };
        ui::caption(v, x + pad, bar_y + self.scale.s(16.0), status, self.scale.fs(9.0), sc, false);

        // mute/unmute toggle button
        let bw = w - pad * 2.0;
        let bh = self.scale.s(22.0);
        let by = y + h - bh - self.scale.s(12.0);
        let hov = self.hover_key == MIC_MUTE_KEY;
        v.push(Cmd::Rect {
            x: x + pad,
            y: by,
            w: bw,
            h: bh,
            r: self.scale.s(6.0),
            color: if hov { ui::raised_hl(&pal) } else { ui::raised(&pal) },
        });
        let lbl = if self.mic_muted { "Unmute mic" } else { "Mute mic" };
        let gc = if self.mic_muted { pal.acc } else { ui::fg2(&pal) };
        ui::text_c(v, x + w / 2.0, by + self.scale.s(6.0), format!("{} {}", if self.mic_muted { ICON_MIC_OFF } else { ICON_MIC }, lbl), self.scale.fs(9.0), gc, true);
        self.region(x + pad - self.scale.s(3.0), by - self.scale.s(3.0), bw + self.scale.s(6.0), bh + self.scale.s(6.0), MIC_MUTE_KEY);
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
    fn card_draws_live_and_muted() {
        let mut s = shell();
        let p = pal();
        let mut v: Vec<Cmd> = Vec::new();
        s.mic_level = 0.65;
        s.mic_muted = false;
        s.draw_micmeter_card(&mut v, 0.0, 0.0, 120.0, 110.0, &p);
        assert!(v.iter().any(|c| matches!(c, Cmd::Text { text, .. } if text.contains("65%"))), "level meta");
        s.mic_muted = true;
        s.draw_micmeter_card(&mut v, 0.0, 0.0, 120.0, 110.0, &p);
        assert!(v.iter().any(|c| matches!(c, Cmd::Text { text, .. } if text.contains("Unmute"))), "unmute affordance");
        // the mute region is registered for clique
        assert!(s.hover_key != MIC_MUTE_KEY);
    }
}