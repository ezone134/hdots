use super::super::*;

/// One synodic month = 29.53059 days.
const SYNODIC: f64 = 29.530_59;
/// Known new-moon epoch: 2000-01-06 18:14 UTC.
const NEW_MOON_EPOCH: f64 = 947_182_440.0;

/// Moon phase as a 0..1 cycle fraction (0 = new, 0.5 = full) plus the
/// illuminated fraction (0..1). Pure date math from a known new-moon epoch.
pub(crate) fn moon_phase(epoch: f64) -> (f64, f64) {
    let days = (epoch - NEW_MOON_EPOCH) / 86_400.0;
    let phase = (days % SYNODIC / SYNODIC).rem_euclid(1.0);
    // illuminated fraction: 0 at new, 1 at full — cosine around the cycle
    let illum = (1.0 - (2.0 * std::f64::consts::PI * phase).cos()) / 2.0;
    (phase, illum)
}

/// Human phase name from the cycle fraction (8 classic names).
pub(crate) fn moon_name(phase: f64) -> &'static str {
    const NAMES: [&str; 8] = [
        "New moon",
        "Waxing crescent",
        "First quarter",
        "Waxing gibbous",
        "Full moon",
        "Waning gibbous",
        "Last quarter",
        "Waning crescent",
    ];
    // center each name on its eighth of the cycle
    NAMES[((phase * 8.0 + 0.5).floor() as usize) % 8]
}

/// Days until the next full moon (0 = today is the full moon).
pub(crate) fn days_to_full(phase: f64) -> f64 {
    ((0.5 - phase).rem_euclid(1.0)) * SYNODIC
}

impl Shell {

    /// MOON — current moon phase: a vector shaded disc (no bitmaps — the
    /// terminator is drawn as lit slivers over a dark disc), the phase name,
    /// illumination % and days-until-full. Pure date math, no sources.
    pub(crate) fn draw_moon_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        let pad = self.scale.s(12.0);
        let hdr = self.scale.s(24.0);
        let epoch = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0);
        let (phase, illum) = moon_phase(epoch);
        let meta = format!("{:.0}%", illum * 100.0);
        self.card_head(v, pal, x, y, w, pad, "Moon", Some(ICON_MOON), Some((&meta, false)));
        // the disc
        let body_top = y + hdr + self.scale.s(4.0);
        let body_h = y + h - body_top - self.scale.s(34.0);
        let d = body_h.min(w - pad * 2.0 - self.scale.s(90.0)).max(self.scale.s(30.0));
        let cx = x + pad + d / 2.0 + self.scale.s(4.0);
        let cy = body_top + body_h / 2.0;
        draw_moon_disc(v, cx, cy, d, phase, pal);
        // side text: name + illumination + days-to-full
        let tx = cx + d / 2.0 + self.scale.s(12.0);
        let max_w = x + w - pad - tx;
        if max_w > self.scale.s(60.0) {
            ui::text(v, tx, cy - self.scale.s(12.0), moon_name(phase), self.scale.fs(10.5), pal.fg, false);
            ui::caption(v, tx, cy + self.scale.s(3.0), format!("{:.0}% lit", illum * 100.0), self.scale.fs(8.5), ui::fg2(&pal), false);
            let dtf = days_to_full(phase);
            let sub = if dtf < 0.8 {
                "full tonight".to_string()
            } else {
                format!("{:.0} d to full", dtf)
            };
            ui::caption(v, tx, cy + self.scale.s(14.0), sub, self.scale.fs(8.5), ui::fg3(&pal), false);
        }
    }

}

/// The vector moon: a dark disc base, then the LIT portion painted as thin
/// vertical slivers. For cycle fraction `p`, the lit width of each scanline
/// follows the classic terminator ellipse — half-disc when the moon is at
/// quarter, sliver → gibbous growing to the full disc at p=0.5, then waning
/// mirrored. `cos(2πp)` flips the lit side sign at the quarters.
fn draw_moon_disc(v: &mut Vec<Cmd>, cx: f32, cy: f32, d: f32, p: f64, pal: &Pal) {
    let r = d / 2.0;
    let dark = mix(ui::hover(pal), pal.bg, 0.45);
    let lit_c = mix(pal.fg, ui::WARN, 0.25);
    // base disc (dark)
    v.push(Cmd::Rect { x: cx - r, y: cy - r, w: d, h: d, r, color: dark });
    // lit slivers — step over the disc; each scanline's lit span is computed
    // from the terminator ellipse: x = ±r·cos(2πp) … simplified to a waxing/
    // waning pair of half-widths
    let waxing = p < 0.5;
    let k = ((2.0 * std::f64::consts::PI * p).cos()) as f32; // +1 new, -1 full
    let step = 1.6_f32.max(d / 40.0);
    let mut yy = cy - r;
    while yy < cy + r {
        // half-chord of the circle at this scanline
        let dy = (yy - cy) / r;
        let chord = (1.0 - dy * dy).max(0.0).sqrt() * r;
        // terminator x-offset: the ellipse collapses at quarters (k≈0 → half)
        let term = k * (chord.max(0.0));
        let (x0, x1) = if waxing {
            // waxing: lit on the RIGHT of the terminator
            let a = (cx + term).max(cx - chord);
            (a, cx + chord)
        } else {
            // waning: lit on the LEFT
            let b = (cx + term).min(cx + chord);
            (cx - chord, b)
        };
        let wdt = x1 - x0;
        if wdt > 0.4 {
            v.push(Cmd::Rect { x: x0, y: yy, w: wdt, h: step + 0.4, r: 0.0, color: lit_c });
        }
        yy += step;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_moon_phases() {
        // 2024-01-11 11:57 UTC was a new moon; 2024-01-25 17:54 UTC a full moon.
        let new_moon = 1_704_977_020.0;
        let full_moon = 1_706_207_640.0;
        let (p_new, i_new) = moon_phase(new_moon);
        assert!(p_new < 0.05 || p_new > 0.95, "new moon at cycle edge, got {p_new}");
        assert!(i_new < 0.05, "new moon ≈ 0% lit, got {i_new}");
        let (p_full, i_full) = moon_phase(full_moon);
        assert!((p_full - 0.5).abs() < 0.05, "full moon ≈ 0.5, got {p_full}");
        assert!(i_full > 0.95, "full moon ≈ 100% lit, got {i_full}");
    }

    #[test]
    fn names_and_days_to_full() {
        assert_eq!(moon_name(0.02), "New moon");
        assert_eq!(moon_name(0.5), "Full moon");
        assert_eq!(moon_name(0.25), "First quarter");
        let dtf = days_to_full(0.5);
        assert!(dtf < 1.5, "full moon → ~0 days to full, got {dtf}");
        let dtf_new = days_to_full(0.0);
        assert!((dtf_new - SYNODIC / 2.0).abs() < 0.2, "new moon → half a cycle to full");
    }

    #[test]
    fn card_draws_all_phases() {
        let cfg: crate::config::Config =
            toml::from_str(crate::config::DEFAULT_SHELL_TOML).expect("default config parses");
        let mut s = Shell::new(cfg);
        s.mode = crate::shell::Mode::Expanded;
        let p = Pal { fg: 0xe8e6e3ff, bg: 0x141414ff, acc: 0x4fc2ffff, sfg: 0x101010ff };
        let mut v: Vec<Cmd> = Vec::new();
        // exercise the disc across the whole cycle — no panics, slivers drawn
        for p10 in [0usize, 2, 4, 5, 7] {
            let phase = p10 as f64 / 10.0;
            draw_moon_disc(&mut v, 40.0, 40.0, 44.0, phase, &p);
        }
        s.draw_moon_card(&mut v, 0.0, 0.0, 220.0, 110.0, &p);
        assert!(v.iter().any(|c| matches!(c, Cmd::Text { text, .. } if text.contains("lit"))));
    }
}
