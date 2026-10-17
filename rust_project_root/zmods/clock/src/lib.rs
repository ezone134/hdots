//! `clock.so` — the example zen-shell data-provider module.
//!
//! Publishes three values into the shell's scene-value namespace:
//!
//! ```ron
//! Header(title: "Clock", meta: Some("{mod_clock.time}"))
//! ```
//!
//! Copy it to `$sources/lib/clock.so` (dotted names keep a plugin's keys from
//! colliding with the shell's own) and it is picked up on the next refresh —
//! no bar restart. Rename it to `clock.so.disabled` to take it out again.
//!
//! Everything here is hand-rolled on purpose: a module that needs a crate to
//! format a time is a module nobody will install. The one non-obvious bit is
//! [`Store`], the reused string buffer described below.

use std::cell::RefCell;
use std::ffi::{c_char, CString};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::OnceLock;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

/// Bumped only if the ABI itself changes — see `zmod/include/zenmod.h`.
const ABI_VERSION: u32 = 1;

const KEYS: [&str; 3] = ["mod_clock.time", "mod_clock.date", "mod_clock.uptime_s"];

// ---------------------------------------------------------------------------
// ABI
// ---------------------------------------------------------------------------

/// # Safety
/// Called by the host through a function pointer; nothing to uphold here.
#[no_mangle]
pub extern "C" fn zen_abi_version() -> u32 {
    ABI_VERSION
}

/// # Safety
/// Called by the host through a function pointer; nothing to uphold here.
#[no_mangle]
pub extern "C" fn zen_provider_count() -> u32 {
    guard(|| KEYS.len() as u32, 0)
}

/// # Safety
/// Called by the host through a function pointer, with `i < zen_provider_count()`.
#[no_mangle]
pub extern "C" fn zen_provider_key(i: u32) -> *const c_char {
    guard(
        || match keys().get(i as usize) {
            Some(k) => k.as_ptr(),
            None => std::ptr::null(),
        },
        std::ptr::null(),
    )
}

/// # Safety
/// Called by the host through a function pointer, with `i < zen_provider_count()`.
/// The returned pointer stays valid until the next call to this function: the
/// value buffers below are recycled per slot, so the host must copy the bytes
/// before asking again (the shell does — see `zmod::abi::copy_str`).
#[no_mangle]
pub extern "C" fn zen_provider_value(i: u32) -> *const c_char {
    guard(
        || match i as usize {
            0 => put(0, &clock_time()),
            1 => put(1, &iso_date()),
            2 => put(2, &uptime_seconds()),
            _ => std::ptr::null(),
        },
        std::ptr::null(),
    )
}

/// Run `f`, and if it panics, return `fallback` instead. An unwind must never
/// leave an `extern "C"` function (that aborts the process — Rust 1.71+), and
/// the host has no way to catch it on our behalf.
fn guard<T>(f: impl FnOnce() -> T, fallback: T) -> T {
    catch_unwind(AssertUnwindSafe(f)).unwrap_or(fallback)
}

// ---------------------------------------------------------------------------
// String storage
// ---------------------------------------------------------------------------

thread_local! {
    /// One recycled `CString` per slot, so a 1 Hz clock does not leak a string per
    /// tick for as long as the bar is up. Safe because the host copies each value
    /// out before requesting the next one.
    static STORE: RefCell<Vec<CString>> = const { RefCell::new(Vec::new()) };
}

/// Keys never change, so they are interned once for the life of the process
/// and handed out as stable pointers.
fn keys() -> &'static [CString] {
    static KEYS_C: OnceLock<Vec<CString>> = OnceLock::new();
    KEYS_C.get_or_init(|| KEYS.iter().map(|k| CString::new(*k).unwrap()).collect())
}

/// Recycle the buffer for slot `i`: store `s` and return a pointer to its
/// NUL-terminated bytes, valid until the next `put` on the same slot.
fn put(i: usize, s: &str) -> *const c_char {
    STORE.with(|buf| {
        let mut buf = buf.borrow_mut();
        if buf.len() <= i {
            buf.resize(i + 1, CString::default());
        }
        // A NUL inside the value would truncate it for the host; the values
        // here never contain one, but a plugin author adding a field should
        // know it matters.
        buf[i] = CString::new(s).unwrap_or_default();
        buf[i].as_ptr()
    })
}

// ---------------------------------------------------------------------------
// The actual values
// ---------------------------------------------------------------------------

fn uptime_seconds() -> String {
    static SINCE: OnceLock<Instant> = OnceLock::new();
    SINCE.get_or_init(Instant::now).elapsed().as_secs().to_string()
}

/// Seconds since the Unix epoch, or 0 if the clock predates it.
fn epoch_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Civil date from a day count since 1970-01-01 (Howard Hinnant's algorithm,
/// which is exact for the whole i64 range we care about).
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

fn iso_date() -> String {
    let (y, m, d) = civil_from_days(epoch_secs().div_euclid(86_400));
    format!("{y:04}-{m:02}-{d:02}")
}

/// Local wall-clock `HH:MM:SS`. The shell runs inside a session with TZ set,
/// but a plugin cannot read the compositor's timezone cheaply, so this is UTC
/// unless the `TZ` environment variable names a zone this code understands —
/// which it does not. Deliberate: the shell's own `{clock}` value is local time
/// and is what cards should use for local time; this module exists to show the
/// plugin path works and to publish something the shell does not already have.
fn clock_time() -> String {
    let secs = epoch_secs().rem_euclid(86_400);
    format!("{:02}:{:02}:{:02}", secs / 3600, (secs % 3600) / 60, secs % 60)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CStr;

    /// Read one of our own getters the way the host does.
    fn as_str(p: *const c_char) -> Option<String> {
        if p.is_null() {
            return None;
        }
        unsafe { CStr::from_ptr(p) }.to_str().ok().map(str::to_owned)
    }

    #[test]
    fn civil_dates_are_right() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(19_723), (2024, 1, 1));
        // 2026-09-25, the day this was written.
        assert_eq!(civil_from_days(20_721), (2026, 9, 25));
        // a leap day, to catch an off-by-one in the month table
        assert_eq!(civil_from_days(19_782), (2024, 2, 29));
    }

    #[test]
    fn abi_shape_is_what_the_host_expects() {
        assert_eq!(zen_abi_version(), zmod::abi::ABI_VERSION);
        assert_eq!(zen_provider_count() as usize, KEYS.len());
        for (i, want) in KEYS.iter().enumerate() {
            let got = as_str(zen_provider_key(i as u32)).unwrap();
            assert_eq!(got, *want);
        }
        // out of range must be a NULL, not a panic
        assert!(zen_provider_key(99).is_null());
        assert!(zen_provider_value(99).is_null());
    }

    #[test]
    fn values_are_plausible_and_stable_in_shape() {
        let time = as_str(zen_provider_value(0)).unwrap();
        assert_eq!(time.len(), 8, "HH:MM:SS, got {time:?}");
        assert_eq!(time.matches(':').count(), 2);
        let date = as_str(zen_provider_value(1)).unwrap();
        assert_eq!(date.len(), 10, "YYYY-MM-DD, got {date:?}");
        assert!(as_str(zen_provider_value(2)).unwrap().parse::<u64>().is_ok());
    }
}
