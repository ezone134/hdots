//! A single digital biquad filter.
//!
//! Every EQ band is one of these. It looks at the last few samples and mixes
//! them together with fixed coefficients to boost or cut one blob of
//! frequencies. The coefficients come from the "RBJ Audio EQ Cookbook"
//! formulas, which are the standard way to build EQ filters.

use std::f64::consts::PI;

/// What shape the filter has in the frequency domain.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    /// A boost/cut bell centred on a frequency (what most EQ bands are).
    Peaking,
    /// Gentle tilt for everything below a frequency (bass controls).
    LowShelf,
    /// Gentle tilt for everything above a frequency (treble controls).
    HighShelf,
    /// Removes everything above a frequency.
    LowPass,
    /// Removes everything below a frequency.
    HighPass,
}

/// The five numbers that define a biquad filter.
///
/// Given an input sample `x` and the previous two inputs (`x1`, `x2`) and
/// outputs (`y1`, `y2`), the output is:
///
/// ```text
/// y = b0*x + b1*x1 + b2*x2 - a1*y1 - a2*y2
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Coeffs {
    pub b0: f64,
    pub b1: f64,
    pub b2: f64,
    pub a1: f64,
    pub a2: f64,
}

/// Compute the coefficients for a filter of the given kind.
///
/// * `sample_rate` — samples per second (e.g. 48000).
/// * `freq` — centre frequency in Hz.
/// * `gain_db` — how loud to make that blob of frequencies (−12 … +12).
/// * `q` — how wide the blob is (higher = narrower).
pub fn coeffs(kind: Kind, sample_rate: u32, freq: f32, gain_db: f32, q: f32) -> Coeffs {
    let fs = sample_rate as f64;
    // Never ask for a frequency above the Nyquist limit (half the sample rate)
    // or the formula divides by zero / wraps around.
    let f0 = (freq as f64).clamp(1.0, fs * 0.49);
    let w0 = 2.0 * PI * f0 / fs;
    let a = 10.0_f64.powf((gain_db as f64) / 40.0);
    let q = q.max(0.001) as f64;
    let alpha = w0.sin() / (2.0 * q);
    let cos_w = w0.cos();

    // The RBJ cookbook gives b0…b2 and a0…a2 un-normalised. We divide
    // everything by a0 so the filter becomes `y = b*x - a*y` (a0 folds away).
    let (b0, b1, b2, a0, a1, a2) = match kind {
        Kind::Peaking => (
            1.0 + alpha * a,
            -2.0 * cos_w,
            1.0 - alpha * a,
            1.0 + alpha / a,
            -2.0 * cos_w,
            1.0 - alpha / a,
        ),
        // Shelf formulas use the "shelf slope" S, which we fix at 1.0
        // (a gentle, natural-sounding slope).
        Kind::LowShelf => {
            let s = 1.0;
            let alpha = w0.sin() / 2.0 * ((a_plus_a_inv(a) * (1.0 / s - 1.0) + 2.0).sqrt());
            (
                a * ((a + 1.0) - (a - 1.0) * cos_w + 2.0 * a.sqrt() * alpha),
                2.0 * a * ((a - 1.0) - (a + 1.0) * cos_w),
                a * ((a + 1.0) - (a - 1.0) * cos_w - 2.0 * a.sqrt() * alpha),
                (a + 1.0) + (a - 1.0) * cos_w + 2.0 * a.sqrt() * alpha,
                -2.0 * ((a - 1.0) + (a + 1.0) * cos_w),
                (a + 1.0) + (a - 1.0) * cos_w - 2.0 * a.sqrt() * alpha,
            )
        }
        Kind::HighShelf => {
            let s = 1.0;
            let alpha = w0.sin() / 2.0 * ((a_plus_a_inv(a) * (1.0 / s - 1.0) + 2.0).sqrt());
            (
                a * ((a + 1.0) + (a - 1.0) * cos_w + 2.0 * a.sqrt() * alpha),
                -2.0 * a * ((a - 1.0) + (a + 1.0) * cos_w),
                a * ((a + 1.0) + (a - 1.0) * cos_w - 2.0 * a.sqrt() * alpha),
                (a + 1.0) - (a - 1.0) * cos_w + 2.0 * a.sqrt() * alpha,
                2.0 * ((a - 1.0) - (a + 1.0) * cos_w),
                (a + 1.0) - (a - 1.0) * cos_w - 2.0 * a.sqrt() * alpha,
            )
        }
        Kind::LowPass => (
            (1.0 - cos_w) / 2.0,
            1.0 - cos_w,
            (1.0 - cos_w) / 2.0,
            1.0 + alpha,
            -2.0 * cos_w,
            1.0 - alpha,
        ),
        Kind::HighPass => (
            (1.0 + cos_w) / 2.0,
            -(1.0 + cos_w),
            (1.0 + cos_w) / 2.0,
            1.0 + alpha,
            -2.0 * cos_w,
            1.0 - alpha,
        ),
    };

    Coeffs {
        b0: b0 / a0,
        b1: b1 / a0,
        b2: b2 / a0,
        a1: a1 / a0,
        a2: a2 / a0,
    }
}

/// `A + 1/A`, a value that shows up repeatedly in the shelf formulas.
fn a_plus_a_inv(a: f64) -> f64 {
    a + 1.0 / a
}

/// The running filter itself. Keeps the tiny slice of history each sample
/// needs, and applies the equation above one sample at a time.
#[derive(Clone, Debug)]
pub struct Biquad {
    c: Coeffs,
    x1: f64,
    x2: f64,
    y1: f64,
    y2: f64,
}

impl Biquad {
    pub fn new(c: Coeffs) -> Self {
        Self { c, x1: 0.0, x2: 0.0, y1: 0.0, y2: 0.0 }
    }

    /// Swap the coefficient set (e.g. when the user drags a slider).
    pub fn set_coeffs(&mut self, c: Coeffs) {
        self.c = c;
    }

    pub fn reset(&mut self) {
        self.x1 = 0.0;
        self.x2 = 0.0;
        self.y1 = 0.0;
        self.y2 = 0.0;
    }

    /// Push one sample through the filter and get the filtered sample out.
    pub fn process(&mut self, input: f32) -> f32 {
        let x = input as f64;
        let c = &self.c;
        let y = c.b0 * x + c.b1 * self.x1 + c.b2 * self.x2 - c.a1 * self.y1 - c.a2 * self.y2;
        self.x2 = self.x1;
        self.x1 = x;
        self.y2 = self.y1;
        self.y1 = y;
        y as f32
    }
}

/// db added to make a gain multiplier (`0 dB → 1.0`, `+6 dB → ~2.0`).
pub fn db_to_linear(db: f32) -> f32 {
    10.0_f32.powf(db / 20.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_gain_peaking_is_transparent() {
        // Feed a sine through a 0 dB peaking filter; it should come out
        // almost untouched (minus a tiny settling period).
        let c = coeffs(Kind::Peaking, 48000, 1000.0, 0.0, 1.414);
        let mut f = Biquad::new(c);
        const N: usize = 4096;
        let mut out = vec![0.0f32; N];
        for i in 0..N {
            let x = (2.0 * PI * i as f64 * 1000.0 / 48000.0).sin() as f32;
            out[i] = f.process(x);
        }
        let error = (N / 2..N)
            .map(|i| {
                let x = (2.0 * PI * i as f64 * 1000.0 / 48000.0).sin() as f32;
                (out[i] - x).abs()
            })
            .fold(0.0f32, |acc, d| acc + d);
        assert!(error / (N as f32 / 2.0) < 0.01, "avg deviation too big: {error}");
    }

    #[test]
    fn positive_gain_boosts_and_negative_cuts() {
        // Compare output amplitude at a boosted vs cut band. Process every
        // sample from the start (so the filter settles) and measure the tail.
        fn band_gain(db: f32) -> f32 {
            let c = coeffs(Kind::Peaking, 48000, 1000.0, db, 1.414);
            let mut f = Biquad::new(c);
            let mut peak = 0.0f32;
            for i in 0..12000 {
                let x = (2.0 * PI * i as f64 * 1000.0 / 48000.0).sin() as f32;
                let y = f.process(x);
                if i > 8000 {
                    peak = peak.max(y.abs());
                }
            }
            peak
        }
        let boosted = band_gain(6.0);
        let cut = band_gain(-6.0);
        assert!(boosted > 1.8, "boosted too weak: {boosted} (expect ~2.0)");
        assert!(cut < 0.6, "cut too weak: {cut} (expect ~0.5)");
    }
}