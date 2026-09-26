use super::super::*;
use super::shared::*;

impl Shell {

    /// FANS — live fan speeds from hwmon (`fan*_input`, RPM). One row per
    /// fan: label + right-aligned mono RPM, with a per-fan bar normalized
    /// against a slow peak so spin-ups are visible without hardcoding
    /// maxima. Empty state mirrors the thermal card.
    pub(crate) fn draw_fans_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        let pad = self.scale.s(12.0);
        let hdr = self.scale.s(24.0);
        let n = self.fans_rpm.len();
        let meta = if n > 0 { format!("{n} fans") } else { String::new() };
        card_header(v, pal, x, y, w, pad, "Fans", Some(ICON_SPEED_FILL), Some((&meta, false)), self.scale, self.card_show_title, self.card_show_glyph);
        if self.fans_rpm.is_empty() {
            card_empty(v, &pal, x, y, w, h, "no hwmon fan sensors", self.scale);
            return;
        }
        let row_h = self.scale.s(24.0);
        let y0 = y + hdr + self.scale.s(4.0);
        for (i, (label, rpm)) in self.fans_rpm.iter().enumerate() {
            let ry = y0 + i as f32 * row_h;
            if ry + row_h > y + h - 2.0 {
                break;
            }
            // slow per-fan peak (decay) → relative bar without hardcoding a max
            let peak = self.fan_peaks.get(i).copied().unwrap_or(1.0).max(1.0);
            let frac = (*rpm as f32 / peak).clamp(0.0, 1.0);
            let bar_w = w - pad * 2.0;
            ui::caption(v, x + pad, ry + self.scale.s(1.0), label, self.scale.fs(8.5), ui::fg2(&pal), false);
            ui::text_r_mono(v, x + w - pad, ry + self.scale.s(1.0), format!("{rpm} rpm"), self.scale.fs(8.5), if *rpm > 0 { pal.fg } else { ui::fg3(&pal) });
            ui::bar(v, x + pad, ry + self.scale.s(13.0), bar_w, self.scale.s(3.0), frac, if *rpm > 0 { ui::INFO } else { ui::hover(&pal) }, ui::hover(&pal));
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
    fn fans_card_draws_empty_and_populated() {
        let mut s = shell();
        let p = pal();
        let mut v: Vec<Cmd> = Vec::new();
        s.draw_fans_card(&mut v, 0.0, 0.0, 220.0, 110.0, &p);
        s.fans_rpm = vec![("fan1 (nct6775)".into(), 1240), ("fan2 (nct6775)".into(), 0)];
        s.fan_peaks = vec![1400.0, 1.0];
        s.draw_fans_card(&mut v, 0.0, 0.0, 220.0, 110.0, &p);
        assert!(v.iter().any(|c| matches!(c, Cmd::Text { text, .. } if text.contains("rpm"))), "rpm readouts drawn");
    }
}
