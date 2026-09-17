use std::sync::{Arc, Mutex};

/// 10-band parametric equalizer for PipeWire audio output.
///
/// Manages a `libpipewire-module-filter-chain` with a built-in `param_eq`
/// node. Band gains are adjusted in real-time via `pw-cli set-param`.

/// Standard 10-band frequencies (Hz) — covers sub-bass through presence.
pub const BAND_FREQS: [u32; 10] = [31, 62, 125, 250, 500, 1000, 2000, 4000, 8000, 16000];
pub const BAND_LABELS: [&str; 10] = [
    "31", "62", "125", "250", "500", "1k", "2k", "4k", "8k", "16k",
];

/// Gain range in dB for each band.
pub const GAIN_MIN: f32 = -12.0;
pub const GAIN_MAX: f32 = 12.0;

#[derive(Debug, Clone)]
pub struct EqPreset {
    pub name: &'static str,
    pub gains: [f32; 10],
}

/// Built-in presets.
pub const PRESETS: &[EqPreset] = &[
    EqPreset { name: "Flat", gains: [0.0; 10] },
    EqPreset {
        name: "Bass Boost",
        gains: [6.0, 5.0, 3.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    },
    EqPreset {
        name: "Vocal",
        gains: [-2.0, 0.0, 0.0, 2.0, 4.0, 4.0, 2.0, 0.0, 0.0, -2.0],
    },
    EqPreset {
        name: "Rock",
        gains: [4.0, 2.0, 0.0, -1.0, 0.0, 2.0, 3.0, 4.0, 4.0, 3.0],
    },
    EqPreset {
        name: "Jazz",
        gains: [2.0, 1.0, 0.0, 2.0, -1.0, -1.0, 0.0, 2.0, 3.0, 4.0],
    },
    EqPreset {
        name: "Electronic",
        gains: [5.0, 4.0, 1.0, 0.0, -2.0, 0.0, 1.0, 3.0, 4.0, 5.0],
    },
    EqPreset {
        name: "Classical",
        gains: [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -2.0, -3.0, -3.0, -4.0],
    },
    EqPreset {
        name: "Loudness",
        gains: [6.0, 4.0, 0.0, 0.0, -1.0, 0.0, 0.0, 0.0, 2.0, 4.0],
    },
];

/// Shared equalizer state — wrapped in `Arc<Mutex<…>>` so the shell, input
/// handler and draw code can all access it without lifetimes.
pub type EqState = Arc<Mutex<Eq>>;

#[derive(Debug, Clone)]
pub struct Eq {
    /// Per-band gain in dB (index matches `BAND_FREQS`).
    pub bands: [f32; 10],
    /// Whether the filter chain is loaded in PipeWire.
    pub active: bool,
    /// Index of the current preset (usize::MAX = custom).
    pub preset: usize,
    /// PipeWire filter-chain module variable id (for unload).
    pub module_id: Option<u32>,
}

impl Default for Eq {
    fn default() -> Self {
        Self {
            bands: [0.0; 10],
            active: false,
            preset: 0, // Flat
            module_id: None,
        }
    }
}

impl Eq {
    /// Apply a preset by index.
    pub fn apply_preset(&mut self, idx: usize) {
        if let Some(p) = PRESETS.get(idx) {
            self.bands = p.gains;
            self.preset = idx;
            self.push_to_pipewire();
        }
    }

    /// Set a single band's gain (clamped).
    pub fn set_band(&mut self, idx: usize, gain: f32) {
        if idx < 10 {
            self.bands[idx] = gain.clamp(GAIN_MIN, GAIN_MAX);
            self.preset = usize::MAX; // custom
            self.push_to_pipewire();
        }
    }

    /// Load or unload the PipeWire filter chain.
    pub fn toggle(&mut self) {
        if self.active {
            self.unload();
        } else {
            self.load();
        }
    }

    /// Load the filter-chain module with current band settings.
    pub fn load(&mut self) {
        if self.active {
            return;
        }
        let filter_config = self.build_filter_config();
        let module_args = format!(
            "{filter_config} node.description=\"zen-shell EQ\" audio.channels=2 audio.position=\"[FL, FR]\""
        );
        if let Some(id_str) = pw_cli(&["load-module", "libpipewire-module-filter-chain", &module_args]) {
            self.module_id = id_str.trim().parse().ok();
            self.active = true;
            // Push current gains to the loaded filter
            self.push_to_pipewire();
        }
    }

    /// Unload the filter-chain module.
    pub fn unload(&mut self) {
        if let Some(id) = self.module_id.take() {
            pw_cli(&["unload-module", &id.to_string()]);
        }
        self.active = false;
    }

    /// Build the filter.graph JSON for pw-cli load-module.
    fn build_filter_config(&self) -> String {
        let filters: Vec<String> = BAND_FREQS
            .iter()
            .zip(self.bands.iter())
            .enumerate()
            .map(|(_i, (freq, gain))| {
                format!(
                    "{{ type = \"param_eq\" freq = {} gain = {:.1} q = 1.414 }}",
                    freq, gain
                )
            })
            .collect();

        format!(
            "filter.graph = {{ nodes = [{{ type = \"builtin\" name = \"eq\" label = \"param_eq\" filters = [{}] }}] }}",
            filters.join(", ")
        )
    }

    /// Push current band gains to the running PipeWire filter via pw-cli.
    fn push_to_pipewire(&self) {
        if !self.active {
            return;
        }
        // The param_eq filter accepts control changes via set-param.
        // Each filter in the chain is addressed by its index (0..N-1).
        for (i, gain) in self.bands.iter().enumerate() {
            let param = format!("\"filters\" \"{}\" \"gain\" \"{}\"", i, gain);
            pw_cli(&["set-param", "eq", &param]);
        }
    }

    /// Compute a smoothed frequency-response curve for visualization.
    /// Returns 100 points (0.0–1.0 x, dB y) representing the combined EQ curve.
    pub fn response_curve(&self) -> [(f32, f32); 100] {
        let mut out = [(0.0f32, 0.0f32); 100];
        for xi in 0..100 {
            let frac = xi as f32 / 99.0;
            // Log-scale frequency from 20 Hz to 20 kHz
            let f = 20.0 * (1000.0_f32).powf(frac);
            let mut db = 0.0f32;
            // Sum each band's contribution (simplified peaking EQ response)
            for (freq, gain) in BAND_FREQS.iter().zip(self.bands.iter()) {
                let ratio = f / *freq as f32;
                // Simple bell curve approximation: gain * exp(-((ln(r))^2 / (2*ln(q)^2)))
                let ln_r = ratio.ln();
                let q = 1.414_f32; // default Q
                let ln_q = q.ln();
                let weight = (-ln_r * ln_r / (2.0 * ln_q * ln_q)).exp();
                db += gain * weight;
            }
            out[xi] = (frac, db.clamp(GAIN_MIN, GAIN_MAX));
        }
        out
    }
}

/// Shell out to `pw-cli` and return stdout.
fn pw_cli(args: &[&str]) -> Option<String> {
    std::process::Command::new("pw-cli")
        .args(args)
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flat_preset_is_zero() {
        assert_eq!(PRESETS[0].gains, [0.0; 10]);
    }

    #[test]
    fn set_band_clamps() {
        let mut eq = Eq::default();
        eq.set_band(0, 20.0); // above max
        assert!((eq.bands[0] - GAIN_MAX).abs() < 0.01);
        eq.set_band(0, -20.0); // below min
        assert!((eq.bands[0] - GAIN_MIN).abs() < 0.01);
    }

    #[test]
    fn response_curve_length() {
        let eq = Eq::default();
        let curve = eq.response_curve();
        assert_eq!(curve.len(), 100);
    }

    #[test]
    fn response_curve_flat_is_zero() {
        let eq = Eq::default();
        let curve = eq.response_curve();
        for (_, db) in &curve {
            assert!(db.abs() < 0.5, "flat EQ should be ~0 dB, got {}", db);
        }
    }

    #[test]
    fn apply_preset_updates_bands() {
        let mut eq = Eq::default();
        eq.apply_preset(1); // Bass Boost
        assert_eq!(eq.preset, 1);
        assert!(eq.bands[0] > 0.0); // 31 Hz boosted
    }
}
