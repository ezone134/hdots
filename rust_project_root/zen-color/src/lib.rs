//! Shared zen color primitives — extracted so every project in
//! `rust_project_root/` decodes, blends, and converts colors through one
//! implementation instead of the six divergent copies that used to live in
//! `zen-shell`, `sp`, `gcp`, `zov`, `wfm-rs`, and `zen-image`.
//!
//! The canonical in-memory format is **`0xRRGGBBAA`** (alpha last) — matching
//! `zen-ui`, the shell theme files written by `theme_save_hex`, and the value
//! the GL/Vulkan backends expect. Consumers that paint opaque (wfm-rs, zov)
//! can use [`parse_rgb`]/[`to_rgb`] to work in `0x00RRGGBB`.

/// Opaque black (`0xRRGGBBAA`).
pub const BLACK: u32 = 0x0000_00ff;
/// Opaque white (`0xRRGGBBAA`).
pub const WHITE: u32 = 0xffff_ffff;
/// Loud magenta — the "broken color" marker the shell uses for a malformed
/// theme line (`0xRRGGBBAA`).
pub const MAGENTA: u32 = 0xff00_ffff;

/// Pack non-premultiplied `r,g,b,a` bytes into `0xRRGGBBAA`.
#[inline]
pub const fn from_rgba(r: u8, g: u8, b: u8, a: u8) -> u32 {
    ((r as u32) << 24) | ((g as u32) << 16) | ((b as u32) << 8) | (a as u32)
}

/// Pack `r,g,b` bytes into fully-opaque `0xRRGGBBAA`.
#[inline]
pub const fn from_rgb(r: u8, g: u8, b: u8) -> u32 {
    from_rgba(r, g, b, 0xff)
}

/// Split `0xRRGGBBAA` into `(r, g, b, a)`.
#[inline]
pub const fn channels(c: u32) -> (u8, u8, u8, u8) {
    ((c >> 24) as u8, (c >> 16) as u8, (c >> 8) as u8, c as u8)
}

/// Alpha byte of `0xRRGGBBAA`.
#[inline]
pub const fn alpha(c: u32) -> u8 {
    c as u8
}

/// Drop alpha, yielding `0x00RRGGBB`.
#[inline]
pub const fn to_rgb(c: u32) -> u32 {
    c >> 8
}

/// Replace the alpha byte of `0xRRGGBBAA`.
#[inline]
pub const fn with_alpha(c: u32, a: u8) -> u32 {
    (c & 0xffff_ff00) | (a as u32)
}

/// Parse `#rgb` / `#rgba` / `#rrggbb` / `#rrggbbaa` (the `#` and an optional
/// `0x`/`0X` prefix are both accepted) into `0xRRGGBBAA`. Returns `None` when
/// the input is empty or contains non-hex characters.
///
/// 6-digit input is fully opaque; 8-digit input is `rrggbbaa` (alpha last,
/// the CSS/hdots order).
pub fn parse_hex(s: &str) -> Option<u32> {
    let t = s.trim();
    let t = t.strip_prefix('#').unwrap_or(t);
    let t = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")).unwrap_or(t);
    let t = t.trim();
    if t.is_empty() || !t.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let (r, g, b, a) = match t.len() {
        3 => (nibble(t, 0), nibble(t, 1), nibble(t, 2), 0xff),
        4 => (nibble(t, 0), nibble(t, 1), nibble(t, 2), nibble(t, 3)),
        6 => (byte(t, 0), byte(t, 2), byte(t, 4), 0xff),
        8 => (byte(t, 0), byte(t, 2), byte(t, 4), byte(t, 6)),
        _ => return None,
    };
    Some(from_rgba(r, g, b, a))
}

/// Parse like [`parse_hex`] but return `0x00RRGGBB` (alpha dropped) — for
/// consumers that paint fully opaque.
pub fn parse_rgb(s: &str) -> Option<u32> {
    parse_hex(s).map(to_rgb)
}

/// Parse like [`parse_hex`], substituting `default` for malformed input.
pub fn parse_hex_or(s: &str, default: u32) -> u32 {
    parse_hex(s).unwrap_or(default)
}

/// Parse like [`parse_hex`] but fall back to loud [`MAGENTA`] — the shell's
/// "this master line is broken, notice it" convention.
pub fn parse_hex_or_magenta(s: &str) -> u32 {
    parse_hex_or(s, MAGENTA)
}

/// Repeat a single hex digit into the high nibble (`"f"` from `#rgb` → `0xff`).
fn nibble(t: &str, i: usize) -> u8 {
    let v = u8::from_str_radix(&t[i..i + 1], 16).unwrap_or(0);
    (v << 4) | v
}

/// Parse one hex byte at `t[i..i+2]`.
fn byte(t: &str, i: usize) -> u8 {
    u8::from_str_radix(&t[i..i + 2], 16).unwrap_or(0)
}

/// Linear mix of two `0xRRGGBBAA` colors (`t = 0` → `a`, `t = 1` → `b`).
/// The result is forced opaque, matching `zen-ui::ui::mix`.
pub fn mix(a: u32, b: u32, t: f32) -> u32 {
    let t = t.clamp(0.0, 1.0);
    let ar = ((a >> 24) & 0xff) as f32;
    let ag = ((a >> 16) & 0xff) as f32;
    let ab = ((a >> 8) & 0xff) as f32;
    let br = ((b >> 24) & 0xff) as f32;
    let bg = ((b >> 16) & 0xff) as f32;
    let bb = ((b >> 8) & 0xff) as f32;
    (((ar + (br - ar) * t) as u32) << 24)
        | (((ag + (bg - ag) * t) as u32) << 16)
        | (((ab + (bb - ab) * t) as u32) << 8)
        | 0xff
}

/// Source-over compositing of non-premultiplied `src` onto `dst`, both
/// `0xRRGGBBAA`. Used by the software rasterizers (shm/tiny-skia paths).
pub fn blend_over(dst: u32, src: u32) -> u32 {
    let (sr, sg, sb, sa) = channels(src);
    if sa == 0 {
        return dst;
    }
    if sa == 255 {
        return src;
    }
    let (dr, dg, db, da) = channels(dst);
    let sa_f = sa as f32 / 255.0;
    let da_f = da as f32 / 255.0;
    let out_a = sa_f + da_f * (1.0 - sa_f);
    if out_a <= 0.0 {
        return 0;
    }
    let ch = |s: u8, d: u8| -> u8 {
        ((s as f32 * sa_f + d as f32 * da_f * (1.0 - sa_f)) / out_a).round() as u8
    };
    from_rgba(ch(sr, dr), ch(sg, dg), ch(sb, db), (out_a * 255.0).round() as u8)
}

/// HSV (`h` 0..360, `s`/`v` 0..1) → `(r, g, b)` bytes.
pub fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (u8, u8, u8) {
    if s <= 0.0 {
        let g = (v * 255.0).round().clamp(0.0, 255.0) as u8;
        return (g, g, g);
    }
    let h = (h.rem_euclid(360.0)) / 60.0;
    let i = h.floor() as i32;
    let f = h - i as f32;
    let p = v * (1.0 - s);
    let q = v * (1.0 - s * f);
    let t = v * (1.0 - s * (1.0 - f));
    let (r, g, b) = match i {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    };
    let to = |x: f32| (x * 255.0).round().clamp(0.0, 255.0) as u8;
    (to(r), to(g), to(b))
}

/// `(r, g, b)` bytes → HSV (`h` 0..360, `s`/`v` 0..1).
pub fn rgb_to_hsv(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
    let r = r as f32 / 255.0;
    let g = g as f32 / 255.0;
    let b = b as f32 / 255.0;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let d = max - min;
    let h = if d == 0.0 {
        0.0
    } else if max == r {
        60.0 * (((g - b) / d).rem_euclid(6.0))
    } else if max == g {
        60.0 * ((b - r) / d + 2.0)
    } else {
        60.0 * ((r - g) / d + 4.0)
    };
    let s = if max == 0.0 { 0.0 } else { d / max };
    (h, s, max)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_6_and_8_digit() {
        assert_eq!(parse_hex("#aabbcc"), Some(0xaabbccff));
        assert_eq!(parse_hex("aabbcc"), Some(0xaabbccff));
        assert_eq!(parse_hex("#aabbccdd"), Some(0xaabbccdd));
        assert_eq!(parse_hex("0xaabbcc"), Some(0xaabbccff));
    }

    #[test]
    fn parses_short_forms() {
        assert_eq!(parse_hex("#abc"), Some(0xaabbccff));
        assert_eq!(parse_hex("#abcd"), Some(0xaabbccdd));
    }

    #[test]
    fn rejects_garbage() {
        assert!(parse_hex("hello").is_none());
        assert!(parse_hex("#12345").is_none());
        assert!(parse_hex("#1234567890").is_none());
        assert!(parse_hex("").is_none());
    }

    #[test]
    fn parse_rgb_drops_alpha() {
        assert_eq!(parse_rgb("#2e3440ff"), Some(0x2e3440));
        assert_eq!(parse_rgb("#E84E4Ee5"), Some(0xe84e4e));
        assert_eq!(parse_rgb("nope"), None);
    }

    #[test]
    fn channels_roundtrip() {
        let c = from_rgba(0x10, 0x20, 0x30, 0x40);
        assert_eq!(channels(c), (0x10, 0x20, 0x30, 0x40));
        assert_eq!(alpha(c), 0x40);
        assert_eq!(to_rgb(c), 0x102030);
        assert_eq!(with_alpha(c, 0xff), 0x102030ff);
    }

    #[test]
    fn hsv_roundtrip() {
        let (r, g, b) = hsv_to_rgb(340.0, 0.8, 1.0);
        let (h, s, v) = rgb_to_hsv(r, g, b);
        assert!((h - 340.0).abs() < 1.0, "hue {h}");
        assert!((s - 0.8).abs() < 0.01, "sat {s}");
        assert!((v - 1.0).abs() < 0.01, "val {v}");
    }

    #[test]
    fn blend_opaque_short_circuits() {
        assert_eq!(blend_over(0x000000ff, 0x123456ff), 0x123456ff);
        assert_eq!(blend_over(0x123456ff, 0x00000000), 0x123456ff);
    }
}
