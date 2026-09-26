//! zeneq — a hand-written 10-band equalizer as a LADSPA plugin.
//!
//! Compile with `cargo build --release`, then point PipeWire's filter-chain
//! module at `target/release/libzeneq.so` with `type = ladspa`.

pub mod biquad;
pub mod eq;
pub mod ladspa;

pub use ladspa::ladspa_descriptor;

/// Convenience export so tests (and curious readers) get a fully-built EQ
/// without knowing LADSPA details.
pub fn ten_band_eq(sample_rate: u32) -> eq::Equalizer {
    eq::Equalizer::new(sample_rate)
}