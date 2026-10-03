//! Real audio visualizer: captures the default sink's monitor stream through
//! `pw-record` (raw f32le PCM on stdout) and publishes raw FFT band magnitudes
//! (0..1) for the Viz card and collapsed pill. Purposely zero new native
//! dependencies — it rides the system's PipeWire/pipewire-pulse tooling.
//!
//! A background thread owns the capture + FFT; the main thread samples the
//! latest bands (shipped in an `Arc<Mutex<State>>`). Bars are published ONLY
//! while the default sink has an active playback stream (`sink_playing` gate)
//! — the monitor puts out analog idle noise when suspended, which would
//! otherwise light the bars up off mic hiss. The cava-style shaping ("jump up
//! fast, melt back down slow") happens in the UI ticker, so the capture stays
//! a pure spectrum. If the capture dies (no audio server, device gone, monitor
//! renamed) the thread respawns the child every two seconds and the UI
//! honestly decays the bars toward silence.

use std::io::Read;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Bar count — must match `Shell::viz` (state.rs inits 24).
pub const BARS: usize = 24;
/// FFT size. 2048 @ 48 kHz ≈ 46 ms window.
const NFFT: usize = 2048;
/// Overlapping hop (50%) → ~46 updates/s.
const HOP: usize = 1024;
const RATE: u32 = 48000;
/// dB floor / ceiling for the magnitude→0..1 mapping.
const DB_LO: f32 = -62.0;
const DB_HI: f32 = -5.0;

struct State {
    bars: Vec<f32>,
    last: Instant,
}

/// Owns the capture thread. The background thread holds everything needed; the
/// process exits tear it all down, so this is intentionally minimal.
pub struct Capture {
    shared: Arc<Mutex<State>>,
}

impl Capture {
    pub fn start() -> Capture {
        let shared = Arc::new(Mutex::new(State {
            bars: vec![0.0; BARS],
            last: Instant::now() - Duration::from_secs(10), // stale until first FFT
        }));
        let t_shared = shared.clone();
        std::thread::Builder::new()
            .name("viz-capture".into())
            .spawn(move || {
                // Defensive: never let a panic take the app down.
                let _ = std::panic::catch_unwind(move || loop {
                    match run_capture(&t_shared) {
                        Ok(()) | Err(()) => {}
                    }
                    std::thread::sleep(Duration::from_secs(2));
                });
            })
            .expect("spawn viz capture thread");
        Capture { shared }
    }

    /// Copy the latest bar magnitudes into `out` (must be `BARS` long).
    /// Returns false when the capture is dead or stale (>2 s), so the UI
    /// decays the bars toward silence instead of showing last-known data.
    pub fn sample(&self, out: &mut Vec<f32>) -> bool {
        let st = match self.shared.lock() {
            Ok(s) => s,
            Err(e) => e.into_inner(),
        };
        if st.last.elapsed() > Duration::from_secs(2) {
            return false;
        }
        out.clear();
        out.extend_from_slice(&st.bars);
        true
    }
}

/// Resolve the default sink's name via pipewire-pulse.
fn default_sink() -> Option<String> {
    let out = Command::new("pactl").arg("get-default-sink").output().ok()?;
    if !out.status.success() {
        return None;
    }
    let sink = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if sink.is_empty() { None } else { Some(sink) }
}

/// Resolve the default sink's monitor source name. `pw-record` needs an
/// explicit capture target, and the "default source" is the microphone — what
/// we want is `$DEFAULT_SINK.monitor`.
fn monitor_target() -> Option<String> {
    Some(format!("{}.monitor", default_sink()?))
}

/// True only while the default sink is actively running playback (last column
/// of `pactl list short sinks`). The sink monitor ALSO coughs up analog/idle
/// noise when it's SUSPENDED, so raw monitor energy can't tell "music is
/// playing" from "mic hiss" — the sink state can. This is the gate that keeps
/// the visualizer silent unless audio is actually playing.
fn sink_playing() -> bool {
    let Some(sink) = default_sink() else { return false };
    let out = match Command::new("pactl").args(["list", "short", "sinks"]).output() {
        Ok(o) if o.status.success() => o,
        _ => return false,
    };
    let text = String::from_utf8_lossy(&out.stdout);
    text.lines().any(|l| {
        let mut c = l.split('\t');
        c.next(); // index
        if c.next() != Some(sink.as_str()) {
            return false;
        }
        // driver, sample-spec, then state (last tab-separated column)
        c.last() == Some("RUNNING")
    })
}

/// One capture run: spawn pw-record, consume stdout until EOF/failure, reap.
fn run_capture(shared: &Arc<Mutex<State>>) -> Result<(), ()> {
    let monitor = monitor_target().ok_or(())?;
    let mut child = Command::new("pw-record")
        .args([
            "--target",
            &monitor,
            "--rate",
            &RATE.to_string(),
            "--channels",
            "2",
            "--format",
            "f32",
            "--raw",
            "-",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| ())?;
    let mut reader = child.stdout.take().ok_or(())?;
    collect(shared, &mut reader);
    // Reap so the child never turns into a zombie.
    let _ = child.wait();
    Ok(())
}

/// Consume the f32le interleaved stereo stream, mono-mix it into an NFFT
/// sliding window, run an FFT per hop and publish smoothed bars.
fn collect(shared: &Arc<Mutex<State>>, reader: &mut dyn Read) {
    let hann = (0..NFFT)
        .map(|i| 0.5 - 0.5 * (2.0 * std::f32::consts::PI * i as f32 / (NFFT - 1) as f32).cos())
        .collect::<Vec<f32>>();
    let mut win: Vec<f32> = vec![0.0; NFFT];
    let mut w: usize = 0;
    let mut leftover: Vec<u8> = Vec::new();
    let mut buf = vec![0u8; 1 << 16];
    let mut bars = vec![0.0f32; BARS];
    // gate: publish real spectrum only while the sink is actually playing.
    // Polled ~5×/s — the sink monitor emits idle noise when suspended, so raw
    // energy would otherwise light the bars up off the mic/analog hiss.
    let mut playing = true;
    let mut last_check = Instant::now();

    loop {
        let rn = match reader.read(&mut buf) {
            Ok(0) => return,
            Ok(n) => n,
            Err(_) => return,
        };
        leftover.extend_from_slice(&buf[..rn]);
        // Complete stereo frames only (f32le interleaved → 8 bytes each).
        let frames = leftover.len() / (4 * 2);
        for f in 0..frames {
            let o = f * 8;
            let l = f32::from_le_bytes([leftover[o], leftover[o + 1], leftover[o + 2], leftover[o + 3]]);
            let r = f32::from_le_bytes([leftover[o + 4], leftover[o + 5], leftover[o + 6], leftover[o + 7]]);
            win[w] = (l + r) * 0.5;
            w += 1;
            if w == NFFT {
                process_window(&win, &hann, &mut bars);
                win.copy_within(HOP.., 0);
                w = NFFT - HOP;
                if last_check.elapsed() >= Duration::from_millis(200) {
                    playing = sink_playing();
                    last_check = Instant::now();
                }
                let mut st = match shared.lock() {
                    Ok(s) => s,
                    Err(e) => e.into_inner(),
                };
                if playing {
                    st.bars.copy_from_slice(&bars);
                } else {
                    st.bars.fill(0.0);
                }
                st.last = Instant::now();
            }
        }
        leftover.drain(..frames * 8);
    }
}

/// Window → FFT → log band magnitudes (0..1). Publish the RAW magnitudes —
    /// the cava-style "jump up fast, melt back down slow" shaping happens in the
    /// 60 fps UI ticker, so the capture stays a pure spectrum.
    fn process_window(win: &[f32], hann: &[f32], bars: &mut Vec<f32>) {
        let mut re = vec![0.0f32; NFFT];
        for i in 0..NFFT {
            re[i] = win[i] * hann[i];
        }
        let mut im = vec![0.0f32; NFFT];
        fft(&mut re, &mut im);

        const NBIN: usize = NFFT / 2;
        let mut mag = vec![0.0f32; NBIN];
        for b in 0..NBIN {
            mag[b] = (re[b] * re[b] + im[b] * im[b]).sqrt() / NFFT as f32;
        }

        // log-spaced bands: 20 Hz → 0.95 * Nyquist
        const NYQ: f32 = RATE as f32 / 2.0;
        let lo = 20.0f32.ln();
        let hi = (NYQ * 0.95).ln();
        for b in 0..BARS {
            let f0 = (lo + b as f32 / BARS as f32 * (hi - lo)).exp();
            let f1 = (lo + (b + 1) as f32 / BARS as f32 * (hi - lo)).exp();
            let i0 = (f0 / NYQ * (NBIN - 1) as f32) as usize;
            let i1 = ((f1 / NYQ * (NBIN - 1) as f32) as usize).min(NBIN - 1);
            let mut acc: f32 = 0.0;
            for i in i0..=i1 {
                acc += mag[i];
            }
            let m = acc / (i1 - i0 + 1) as f32;
            let db = 20.0 * m.max(1e-8).log10();
            bars[b] = ((db - DB_LO) / (DB_HI - DB_LO)).clamp(0.0, 1.0);
        }
    }

/// Iterative radix-2 Cooley–Tukey FFT (NFFT is a power of two).
fn fft(re: &mut [f32], im: &mut [f32]) {
    let n = re.len();
    let mut j = 0usize;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j ^= bit;
        if i < j {
            re.swap(i, j);
            im.swap(i, j);
        }
    }
    let mut len = 2usize;
    while len <= n {
        let ang = -2.0 * std::f32::consts::PI / len as f32;
        let (wr, wi) = (ang.cos(), ang.sin());
        let mut i = 0usize;
        while i < n {
            let (mut cr, mut ci) = (1.0f32, 0.0f32);
            let half = len / 2;
            for k in 0..half {
                let ur = re[i + k];
                let ui = im[i + k];
                let o = i + k + half;
                let vr = re[o] * cr - im[o] * ci;
                let vi = re[o] * ci + im[o] * cr;
                re[i + k] = ur + vr;
                im[i + k] = ui + vi;
                re[o] = ur - vr;
                im[o] = ui - vi;
                let nc = cr * wr - ci * wi;
                ci = cr * wi + ci * wr;
                cr = nc;
            }
            i += len;
        }
        len <<= 1;
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    /// A pure 1 kHz sine must light up the log band covering ~1 kHz
    /// (bar ~13 of 24) above everything else, proving FFT + band math.
    #[test]
    fn sine_peaks_its_band() {
        let hann = (0..NFFT)
            .map(|i| 0.5 - 0.5 * (2.0 * std::f32::consts::PI * i as f32 / (NFFT - 1) as f32).cos())
            .collect::<Vec<f32>>();
        let mut win = vec![0.0f32; NFFT];
        for i in 0..NFFT {
            let t = i as f32 / RATE as f32;
            win[i] = 0.5 * (2.0 * std::f32::consts::PI * 1000.0 * t).sin();
        }
        let mut bars = vec![0.12f32; BARS];
        // raw publish — the band value is stable pass-to-pass
        for _ in 0..5 {
            process_window(&win, &hann, &mut bars);
        }
        let peak = bars
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .unwrap()
            .0;
        assert!(
            (11..=14).contains(&peak),
            "1 kHz fell in bar {peak}, expected ~13. bars={bars:?}"
        );
        assert!(bars[peak] > 0.4, "peak bar too quiet: {bars:?}");
    }

    /// A fully silent window must decay (never grow) the bars.
    #[test]
    fn silence_floors() {
        let hann = vec![0.5; NFFT];
        let mut win = vec![0.0f32; NFFT];
        let mut bars = vec![0.12f32; BARS];
        for _ in 0..5 {
            process_window(&win, &hann, &mut bars);
        }
        assert!(bars.iter().all(|b| *b < 0.12), "{bars:?}");
    }
}
