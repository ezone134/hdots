//! The full equalizer: ten biquad bands wired in series, plus a master gain.

use crate::biquad::{db_to_linear, Kind, Biquad, Coeffs};

/// Centre frequency of each band in Hz. The classic ten-band layout.
pub const BAND_FREQS: [f32; 10] = [
    31.0, 62.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0,
];

/// Q (width) shared by every band.
pub const BAND_Q: f32 = 1.414;

pub const GAIN_MIN: f32 = -12.0;
pub const GAIN_MAX: f32 = 12.0;

/// The whole signal chain. Owns one `Biquad` per band; a sample enters the
/// first band, flows through all ten, and leaves changed (or not!).
pub struct Equalizer {
    bands: Vec<Biquad>,
    sample_rate: u32,
    gains: [f32; 10],
    master_db: f32,
    master_lin: f32,
    /// Set whenever gains, master gain or sample rate change, so `process`
    /// knows to recalculate the coefficients before the next sample.
    dirty: bool,
}

impl Equalizer {
    pub fn new(sample_rate: u32) -> Self {
        let bands = BAND_FREQS
            .iter()
            .map(|&f| Biquad::new(band_coeffs(sample_rate, f, 0.0)))
            .collect();
        Self {
            bands,
            sample_rate,
            gains: [0.0; 10],
            master_db: 0.0,
            master_lin: 1.0,
            dirty: false,
        }
    }

    /// Called by the host when the audio hardware's sample rate is known
    /// (LADSPA hands it to us in `instantiate`, so this usually happens once).
    pub fn set_sample_rate(&mut self, sample_rate: u32) {
        if sample_rate != self.sample_rate {
            self.sample_rate = sample_rate;
            self.dirty = true;
        }
    }

    /// Update the ten band gains (in dB) and the master gain (in dB).
    pub fn set_gains(&mut self, gains: &[f32; 10], master_db: f32) {
        if *gains != self.gains || master_db != self.master_db {
            self.gains = *gains;
            self.master_db = master_db;
            self.master_lin = db_to_linear(master_db);
            self.dirty = true;
        }
    }

    pub fn gains(&self) -> [f32; 10] {
        self.gains
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn reset(&mut self) {
        for band in &mut self.bands {
            band.reset();
        }
        self.dirty = true;
    }

    /// Push one sample through all ten bands → the filtered sample.
    pub fn process(&mut self, input: f32) -> f32 {
        if self.dirty {
            self.refresh();
        }
        let mut x = input * self.master_lin;
        for band in &mut self.bands {
            x = band.process(x);
        }
        x
    }

    fn refresh(&mut self) {
        for (i, band) in self.bands.iter_mut().enumerate() {
            band.set_coeffs(band_coeffs(self.sample_rate, BAND_FREQS[i], self.gains[i]));
        }
        self.dirty = false;
    }
}

fn band_coeffs(sample_rate: u32, freq: f32, gain_db: f32) -> Coeffs {
    crate::biquad::coeffs(Kind::Peaking, sample_rate, freq, gain_db.clamp(GAIN_MIN, GAIN_MAX), BAND_Q)
}

/// Handy named curves. Each array is the gain in dB for the ten bands.
pub const PRESETS: [(&str, [f32; 10]); 8] = [
    ("Flat", [0.0; 10]),
    ("Bass Boost", [5.0, 5.0, 4.0, 3.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0]),
    ("Vocal", [0.0, 0.0, 0.0, 1.0, 2.0, 3.0, 3.0, 2.0, 1.0, 0.0]),
    ("Rock", [3.0, 3.0, 2.0, 1.0, -1.0, -1.0, 1.0, 3.0, 4.0, 4.0]),
    ("Jazz", [2.0, 2.0, 1.0, 1.0, 0.0, 1.0, 2.0, 3.0, 3.0, 3.0]),
    ("Electronic", [4.0, 4.0, 3.0, 2.0, 1.0, 0.0, 2.0, 3.0, 4.0, 5.0]),
    ("Classical", [2.0, 2.0, 1.0, 0.0, 0.0, 0.0, 0.0, -1.0, -2.0, -2.0]),
    ("Loudness", [5.0, 4.0, 4.0, 3.0, 2.0, 1.0, 0.0, 1.0, 2.0, 3.0]),
];

/// Look up a preset by name (case-insensitive).
pub fn preset_gains(name: &str) -> Option<[f32; 10]> {
    PRESETS
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case(name))
        .map(|(_, g)| *g)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    #[test]
    fn flat_eq_passes_sine_through() {
        let mut eq = Equalizer::new(48000);
        let n = 8192;
        let mut err = 0.0f32;
        for i in 0..n {
            let x = (2.0 * PI * (i as f64) * 440.0 / 48000.0).sin() as f32;
            let y = eq.process(x);
            if i > n / 2 {
                err += (y - x).abs();
            }
        }
        assert!(err / (n as f32 / 2.0) < 0.01, "flat eq distorts: {err}");
    }

    #[test]
    fn bass_boost_preset_raises_low_freq_output() {
        let mut flat = Equalizer::new(48000);
        let mut bassy = Equalizer::new(48000);
        bassy.set_gains(&preset_gains("Bass Boost").unwrap(), 0.0);

        // A 60 Hz sine sits inside the first band; measure average loudness.
        let measure = |eq: &mut Equalizer| -> f32 {
            let n = 8192;
            let mut peak = 0.0f32;
            for i in 0..n {
                let x = (2.0 * PI * (i as f64) * 60.0 / 48000.0).sin() as f32;
                peak = peak.max(eq.process(x).abs());
            }
            peak
        };
        let (flat_peak, bassy_peak) = (measure(&mut flat), measure(&mut bassy));
        assert!(
            bassy_peak > flat_peak * 1.5,
            "bass boost too weak: {bassy_peak} vs {flat_peak}"
        );
    }

    #[test]
    fn changing_gains_recomputes_coefficients() {
        let mut eq = Equalizer::new(48000);
        let n = 4096;
        let run = |eq: &mut Equalizer| {
            let mut sum = 0.0f32;
            for i in 0..n {
                let x = (2.0 * PI * (i as f64) * 1000.0 / 48000.0).sin() as f32;
                sum += eq.process(x);
            }
            sum
        };
        let flat = run(&mut eq);
        eq.set_gains(&[10.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0], 0.0);
        let boosted = run(&mut eq);
        assert!((boosted - flat).abs() > 1.0, "no audible change after set_gains");
    }
}