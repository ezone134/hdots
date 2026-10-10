/// One synodic month = 29.53059 days.
const SYNODIC: f64 = 29.530_59;
/// Known new-moon epoch: 2000-01-06 18:14 UTC.
const NEW_MOON_EPOCH: f64 = 947_182_440.0;

/// Moon phase as a 0..1 cycle fraction (0 = new, 0.5 = full) plus the
/// illuminated fraction (0..1). Pure date math from a known new-moon epoch.
pub fn moon_phase(epoch: f64) -> (f64, f64) {
    let days = (epoch - NEW_MOON_EPOCH) / 86_400.0;
    let phase = (days % SYNODIC / SYNODIC).rem_euclid(1.0);
    // illuminated fraction: 0 at new, 1 at full — cosine around the cycle
    let illum = (1.0 - (2.0 * std::f64::consts::PI * phase).cos()) / 2.0;
    (phase, illum)
}

/// Human phase name from the cycle fraction (8 classic names).
pub fn moon_name(phase: f64) -> &'static str {
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
pub fn days_to_full(phase: f64) -> f64 {
    ((0.5 - phase).rem_euclid(1.0)) * SYNODIC
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
        assert!(
            p_new < 0.05 || p_new > 0.95,
            "new moon at cycle edge, got {p_new}"
        );
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
        assert!(
            (dtf_new - SYNODIC / 2.0).abs() < 0.2,
            "new moon → half a cycle to full"
        );
    }
}