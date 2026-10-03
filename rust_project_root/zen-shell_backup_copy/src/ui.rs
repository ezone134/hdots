//! Reusable UI components — the Rust analogue of quickshell's QML component
//! library (Panel / Popup / Chip / Slider / …). Each component draws into a
//! `Cmd` scene given an explicit theme (`Pal`) and hover state, so layouts
//! compose them without reaching into shell state. Hit-test regions are
//! registered by the caller next to the draw call, keeping drawing and
//! interaction colocated but decoupled.

use crate::icons::*;
use crate::shell::Cmd;

/// Derived palette — minimal set of base colors; everything else is derived
/// at layout time from `fg`, `bg`, `acc`, `sfg`.  Fewer stored fields =
/// fewer places for color inconsistency.
#[derive(Clone, Copy)]
pub struct Pal {
    pub fg: u32,
    pub bg: u32,
    pub acc: u32,
    pub sfg: u32, // text on accent background
}

// ── Derived colors (computed once per frame from the base four) ───────────

/// Secondary text — 60% fg blended toward bg.
pub fn fg2(p: &Pal) -> u32 { mix(p.fg, p.bg, 0.55) }
/// Tertiary / muted text — 82% fg blended toward bg.
pub fn fg3(p: &Pal) -> u32 { mix(p.fg, p.bg, 0.82) }
/// Hover background — subtle lift above the surface bg.
pub fn hover(p: &Pal) -> u32 { mix(p.bg, p.fg, 0.07) }
/// Hover-highlighted element — slightly stronger than hover.
pub fn hover_hl(p: &Pal) -> u32 { mix(p.bg, p.fg, 0.12) }
pub fn hover_fg(p: &Pal) -> u32 { mix(p.fg, p.acc, 0.20) }
/// Accent-tinted background (active ws pill, icon tiles, today highlight).
pub fn acc_tint(p: &Pal) -> u32 { mix(p.bg, p.acc, 0.16) }
/// Selected / active row background.
pub fn sel_bg(p: &Pal) -> u32 { mix(p.bg, p.acc, 0.10) }
/// Hairline separator — minimal surface definition: a 1px tone line, never
/// a filled panel. Every flat container uses this for its edge.
pub fn hairline(p: &Pal) -> u32 { mix(p.bg, p.fg, 0.10) }
/// Hairline raised onto a hover state (slightly stronger than `hairline`).
pub fn hairline_hl(p: &Pal) -> u32 { mix(p.bg, p.fg, 0.20) }

/// Raised (non-interactive) surface — quiet tone lift above the panel bg,
/// for card bodies / wells. Minimal: tone does the depth, never shadow.
pub fn raised(p: &Pal) -> u32 { mix(p.bg, p.fg, 0.05) }
/// Raised surface on hover / active.
pub fn raised_hl(p: &Pal) -> u32 { mix(p.bg, p.fg, 0.10) }

/// Hairline outline around a flat surface (`color` = the hairline tone).
/// Minimal: 1px definition, no shadows, no filled chrome.
#[allow(clippy::too_many_arguments)]
pub fn outline(
    v: &mut Vec<Cmd>,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    r: f32,
    color: u32,
) {
    v.push(Cmd::Outline { x, y, w, h, r, width: 1.0, color });
}

/// Linear-mix two 0xRRGGBBAA colors (alpha forced opaque).
pub fn mix(a: u32, b: u32, t: f32) -> u32 {
    let t = t.clamp(0.0, 1.0);
    let ar = ((a >> 24) & 0xff) as f32;
    let ag = ((a >> 16) & 0xff) as f32;
    let ab = ((a >> 8) & 0xff) as f32;
    let br = ((b >> 24) & 0xff) as f32;
    let bg_ = ((b >> 16) & 0xff) as f32;
    let bb = ((b >> 8) & 0xff) as f32;
    (((ar + (br - ar) * t) as u32) << 24)
        | (((ag + (bg_ - ag) * t) as u32) << 16)
        | (((ab + (bb - ab) * t) as u32) << 8)
        | 0xff
}

// ------------------------------------------------------------------ text
//
// Font + weight stack: every card's text is part of the *minimal* type system
// (see crate::text). Plain `text/text_r/text_c` render Inter Light — the quiet
// default. Use the `*_w` variants to step up the stack for hierarchy:
//   `text_w(.., Ff::Ui, Fw::Light)`   — body / labels  (the default)
//   `text_w(.., Ff::Ui, Fw::Medium)`  — inline emphasis
//   `text_w(.., Ff::Display, Fw::Medium)` — hero numerals (big temps, %)
//   `text_w(.., Ff::Mono, Fw::Regular)`   — technical tabular values

pub fn text_w(v: &mut Vec<Cmd>, x: f32, y: f32, t: impl Into<String>, size: f32, f: crate::text::Ff, w: crate::text::Fw, color: u32, icon: bool) {
    v.push(Cmd::Text { x, y, right: false, center: false, text: t.into(), size, color, icon, f, w });
}
pub fn text_rw(v: &mut Vec<Cmd>, x: f32, y: f32, t: impl Into<String>, size: f32, f: crate::text::Ff, w: crate::text::Fw, color: u32, icon: bool) {
    v.push(Cmd::Text { x, y, right: true, center: false, text: t.into(), size, color, icon, f, w });
}
pub fn text_cw(v: &mut Vec<Cmd>, x: f32, y: f32, t: impl Into<String>, size: f32, f: crate::text::Ff, w: crate::text::Fw, color: u32, icon: bool) {
    v.push(Cmd::Text { x, y, right: false, center: true, text: t.into(), size, color, icon, f, w });
}

pub fn text(v: &mut Vec<Cmd>, x: f32, y: f32, t: impl Into<String>, size: f32, color: u32, icon: bool) {
    text_w(v, x, y, t, size, crate::text::Ff::Ui, crate::text::Fw::Light, color, icon);
}
pub fn text_r(v: &mut Vec<Cmd>, x: f32, y: f32, t: impl Into<String>, size: f32, color: u32, icon: bool) {
    text_rw(v, x, y, t, size, crate::text::Ff::Ui, crate::text::Fw::Light, color, icon);
}
pub fn text_c(v: &mut Vec<Cmd>, x: f32, y: f32, t: impl Into<String>, size: f32, color: u32, icon: bool) {
    text_cw(v, x, y, t, size, crate::text::Ff::Ui, crate::text::Fw::Light, color, icon);
}

/// Display-medium: the hero-numeral voice (big temp / % / clock digits).
/// Use for the single dominant figure on a card so it reads tight and strong.
pub fn text_hero(v: &mut Vec<Cmd>, x: f32, y: f32, t: impl Into<String>, size: f32, color: u32) {
    text_w(v, x, y, t, size, crate::text::Ff::Display, crate::text::Fw::Medium, color, false);
}
pub fn text_r_hero(v: &mut Vec<Cmd>, x: f32, y: f32, t: impl Into<String>, size: f32, color: u32) {
    text_rw(v, x, y, t, size, crate::text::Ff::Display, crate::text::Fw::Medium, color, false);
}
pub fn text_c_hero(v: &mut Vec<Cmd>, x: f32, y: f32, t: impl Into<String>, size: f32, color: u32) {
    text_cw(v, x, y, t, size, crate::text::Ff::Display, crate::text::Fw::Medium, color, false);
}

/// Mono-regular: technical / tabular figures (rates, sizes, sensor reads).
pub fn text_mono(v: &mut Vec<Cmd>, x: f32, y: f32, t: impl Into<String>, size: f32, color: u32) {
    text_w(v, x, y, t, size, crate::text::Ff::Mono, crate::text::Fw::Regular, color, false);
}
pub fn text_r_mono(v: &mut Vec<Cmd>, x: f32, y: f32, t: impl Into<String>, size: f32, color: u32) {
    text_rw(v, x, y, t, size, crate::text::Ff::Mono, crate::text::Fw::Regular, color, false);
}
pub fn text_c_mono(v: &mut Vec<Cmd>, x: f32, y: f32, t: impl Into<String>, size: f32, color: u32) {
    text_cw(v, x, y, t, size, crate::text::Ff::Mono, crate::text::Fw::Regular, color, false);
}

/// Semibold: the card title voice. Headers at semibold against the light body
/// give the minimal two-tier hierarchy (thin body, crisp title).
pub fn title(v: &mut Vec<Cmd>, x: f32, y: f32, t: impl Into<String>, size: f32, color: u32) {
    text_w(v, x, y, t, size, crate::text::Ff::Ui, crate::text::Fw::Semibold, color, false);
}

/// Caption: micro-labels (row labels, meta, units) at REGULAR weight. Small
/// text at Light weight dissolves into fuzz — the professional look steps
/// small sizes UP to Regular (Linear / VS Code do the same).
pub fn caption(v: &mut Vec<Cmd>, x: f32, y: f32, t: impl Into<String>, size: f32, color: u32, icon: bool) {
    text_w(v, x, y, t, size, crate::text::Ff::Ui, crate::text::Fw::Regular, color, icon);
}
pub fn caption_r(v: &mut Vec<Cmd>, x: f32, y: f32, t: impl Into<String>, size: f32, color: u32, icon: bool) {
    text_rw(v, x, y, t, size, crate::text::Ff::Ui, crate::text::Fw::Regular, color, icon);
}
pub fn caption_c(v: &mut Vec<Cmd>, x: f32, y: f32, t: impl Into<String>, size: f32, color: u32, icon: bool) {
    text_cw(v, x, y, t, size, crate::text::Ff::Ui, crate::text::Fw::Regular, color, icon);
}

// ------------------------------------------------------------ status tokens
//
// The ONLY status colors in the UI — muted, Tokyo-Night-toned so they sit
// quietly on dark surfaces and never fight the one accent. Threshold ladders
// (cpu ≥70, disk ≥75, temp ≥60…) must go through `pct_color` / these tokens,
// never hand-rolled hex.

/// OK / charging / online — muted green.
pub const OK: u32 = 0x9ece6aff;
/// Warning / elevated — muted amber.
pub const WARN: u32 = 0xe0af68ff;
/// Danger / critical / destructive — muted red.
pub const DANGER: u32 = 0xf7768eff;
/// Info / secondary data series — muted blue (download line, VRAM bar).
pub const INFO: u32 = 0x7aa2f7ff;

/// Threshold color for a percent-style metric (0..=100): `normal` below the
/// warning line (usually `pal.fg` or `pal.acc`), WARN from `warn_at`, DANGER
/// from `crit_at`. The single source for every ≥75/≥90-style ladder.
pub fn pct_color(pct: i32, warn_at: i32, crit_at: i32, normal: u32) -> u32 {
    if pct >= crit_at {
        DANGER
    } else if pct >= warn_at {
        WARN
    } else {
        normal
    }
}

// --------------------------------------------------------------- surfaces

/// Outer surface with per-corner radii — negative = concave (Noctalia-style screen-edge carve).
pub fn frame_concave(v: &mut Vec<Cmd>, w: f32, h: f32, r_tl: f32, r_tr: f32, r_br: f32, r_bl: f32, bg: u32) {
    v.push(Cmd::RectConcave { x: 0.0, y: 0.0, w, h, r_tl, r_tr, r_br, r_bl, color: bg });
}

/// Rounded card surface (media card, system card, search input…).
pub fn card(v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, r: f32, color: u32) {
    v.push(Cmd::Rect { x, y, w, h, r, color });
}

// --------------------------------------------------------------- widgets

/// Control-center quick tile: bare glyph + label + status on a hairline-outlined
/// flat surface (no icon circle, no filled chrome). `on` drives the accent
/// glyph / switch; hover lifts the hairline + tone so rows read flat until
/// touched. The caller registers the hit region.
#[allow(clippy::too_many_arguments)]
pub fn tile(
    v: &mut Vec<Cmd>,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    glyph: &str,
    label: &str,
    status: &str,
    on: bool,
    switch: bool,
    is_hov: bool,
    pal: &Pal,
) {
    v.push(Cmd::Rect {
        x,
        y,
        w,
        h,
        r: 12.0,
        color: if is_hov { hover(pal) } else { 0 },
    });
    outline(v, x, y, w, h, 12.0, if is_hov { hairline_hl(pal) } else { hairline(pal) });
    // bare glyph — accent when active, quiet tone otherwise
    let ic = if on { pal.acc } else { fg3(pal) };
    text(v, x + 16.0, y + 16.0, glyph, 18.0, ic, true);
    // top row: label (+ switch when interactive)
    text(v, x + 42.0, y + 15.0, label, 12.0, pal.fg, false);
    if switch {
        let tx = x + w - 58.0;
        v.push(Cmd::Rect {
            x: tx,
            y: y + 16.0,
            w: 42.0,
            h: 22.0,
            r: 11.0,
            color: if on { pal.acc } else { mix(pal.bg, pal.fg, 0.12) },
        });
        v.push(Cmd::Rect {
            x: if on { tx + 42.0 - 18.0 } else { tx + 2.0 },
            y: y + 18.0,
            w: 18.0,
            h: 18.0,
            r: 9.0,
            color: if on { pal.sfg } else { fg3(pal) },
        });
    }
    // bottom row: status, truncated to the available width
    let budget = ((w - 58.0) / 6.0).max(4.0) as usize;
    let status: String = status.chars().take(budget).collect();
    text(v, x + 42.0, y + 41.0, status, 10.0, if on { fg2(pal) } else { fg3(pal) }, false);
}

/// Slider track + fill + round thumb. `val` in 0..=1; the whole track is
/// hot. Minimal: a slim recessive track, accent fill, and a round head
/// dot in the SAME accent — head and active stroke blend into one line.
/// The caller registers the (padded) hit region.
pub fn slider(v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, val: f32, is_hov: bool, pal: &Pal) {
    let cy = y + h / 2.0;
    let t = 4.0_f32; // slim groove — `h` keeps the hit target
    let gy = cy - t / 2.0;
    v.push(Cmd::Rect {
        x,
        y: gy,
        w,
        h: t,
        r: t / 2.0,
        color: mix(pal.bg, pal.fg, if is_hov { 0.16 } else { 0.10 }),
    });
    let fill = (w * val).clamp(0.0, w);
    if fill > 0.0 {
        v.push(Cmd::Rect { x, y: gy, w: fill, h: t, r: t / 2.0, color: pal.acc });
    }
    // round head — same accent as the fill, grows slightly on hover
    let d = if is_hov { 14.0_f32 } else { 12.0_f32 };
    let kx = (x + w * val - d / 2.0).clamp(x, x + w - d / 2.0);
    v.push(Cmd::Rect {
        x: kx,
        y: cy - d / 2.0,
        w: d,
        h: d,
        r: d / 2.0,
        color: pal.acc,
    });
}

/// Flat progress bar — same as `slider` but no circular knob.
pub fn slider_bar(v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, val: f32, is_hov: bool, pal: &Pal) {
    v.push(Cmd::Rect {
        x,
        y,
        w,
        h,
        r: h / 2.0,
        color: mix(pal.bg, pal.fg, if is_hov { 0.16 } else { 0.10 }),
    });
    let fill = (w * val).clamp(0.0, w);
    v.push(Cmd::Rect { x, y, w: fill, h, r: h / 2.0, color: pal.acc });
}

/// Horizontal EQ-fader slider: a slim groove, accent fill,
/// and a ROUND head dot in the SAME color as the active fill — head and
/// filled track read as one continuous stroke (the minimal look).
pub fn fader(
    v: &mut Vec<Cmd>,
    x: f32,
    cy: f32,
    w: f32,
    frac: f32,
    color: u32,
    is_hov: bool,
    pal: &Pal,
) {
    let t = 4.0_f32; // groove thickness — slim
    let gy = cy - t / 2.0;
    let frac = frac.clamp(0.0, 1.0);
    // groove
    v.push(Cmd::Rect { x, y: gy, w, h: t, r: t / 2.0, color: mix(pal.bg, pal.fg, 0.10) });
    // fill — same color as the head so the stroke blends smoothly
    if frac > 0.001 {
        let fw = ((frac * w) as u32).max(1) as f32;
        v.push(Cmd::Rect {
            x,
            y: gy,
            w: fw.min(w),
            h: t,
            r: t / 2.0,
            color,
        });
    }
    // round head — dot in the fill color, slightly larger on hover
    let d = if is_hov { 12.0_f32 } else { 10.0_f32 };
    let hx = (x + frac * w).clamp(x + d / 2.0, x + w - d / 2.0) - d / 2.0;
    v.push(Cmd::Rect {
        x: hx,
        y: cy - d / 2.0,
        w: d,
        h: d,
        r: d / 2.0,
        color,
    });
}

/// Balance fader — same slim groove/round head as `fader`, but the
/// fill grows from the center (0.5) out toward the head, so an even L/R
/// split reads as centered with nothing filled.
pub fn fader_bal(
    v: &mut Vec<Cmd>,
    x: f32,
    cy: f32,
    w: f32,
    frac: f32,
    color: u32,
    is_hov: bool,
    pal: &Pal,
) {
    let t = 4.0_f32;
    let gy = cy - t / 2.0;
    let frac = frac.clamp(0.0, 1.0);
    v.push(Cmd::Rect { x, y: gy, w, h: t, r: t / 2.0, color: mix(pal.bg, pal.fg, 0.10) });
    let cx = x + w * 0.5;
    if (frac - 0.5).abs() > 0.001 {
        let flen = (((frac - 0.5).abs() * w) as u32).max(1) as f32;
        let flen = flen.min(w * 0.5);
        let (fx, fw) = if frac < 0.5 { (cx - flen, flen) } else { (cx, flen) };
        v.push(Cmd::Rect {
            x: fx,
            y: gy,
            w: fw,
            h: t,
            r: t / 2.0,
            color,
        });
    }
    // round head — same color as the fill
    let d = if is_hov { 12.0_f32 } else { 10.0_f32 };
    let hx = (x + frac * w).clamp(x + d / 2.0, x + w - d / 2.0) - d / 2.0;
    v.push(Cmd::Rect {
        x: hx,
        y: cy - d / 2.0,
        w: d,
        h: d,
        r: d / 2.0,
        color,
    });
}

/// Media transport button (prev / play-pause / next) — bare glyph, accent on
/// hover, no filled circle.
pub fn media_btn(
    v: &mut Vec<Cmd>,
    bx: f32,
    by: f32,
    glyph: &str,
    size: f32,
    is_hov: bool,
    color: u32,
    pal: &Pal,
) {
    text(v, bx, by, glyph, size, if is_hov { pal.acc } else { color }, true);
}

/// Static progress bar (track + fill), no knob, no hit region.
pub fn bar(v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pct: f32, fill: u32, track: u32) {
    v.push(Cmd::Rect { x, y, w, h, r: h / 2.0, color: track });
    if pct > 0.0 {
        v.push(Cmd::Rect { x, y, w: w * pct, h, r: h / 2.0, color: fill });
    }
}

/// Heartbeat / ECG-style line graph. Data is a slice of normalized `f32`
/// values (0..=1); samples are spread evenly across the width and connected
/// with thick capsule segments (see `Renderer::line`) so spikes render as
/// true diagonals, not filled hills.
pub fn pulse(v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, data: &[f32], color: u32, t: f32) {
    let n = data.len();
    if n < 2 {
        return;
    }
    // small vertical inset so peak/trough line thickness never clips
    let pad = t / 2.0 + 1.0;
    let usable = (h - pad * 2.0).max(2.0);
    let step = w / (n - 1).max(1) as f32;
    let mut prev: Option<(f32, f32)> = None;
    for (i, &d) in data.iter().enumerate() {
        let px = x + i as f32 * step;
        let py = y + h - pad - d.clamp(0.0, 1.0) * usable;
        if let Some((qx, qy)) = prev {
            v.push(Cmd::Line { x0: qx, y0: qy, x1: px, y1: py, w: t, color });
        }
        prev = Some((px, py));
    }
}

// ------------------------------------------------------------------ battery

/// Nerd-Font **vertical** battery glyph (MDI battery set) for a charge percent.
/// Charging shows the bolt-in-battery glyph; otherwise the fill rises with
/// level (empty at 10%, full shell from 81% up). Replaces the old hand-drawn
/// horizontal vector battery — a plain text glyph is cheaper and shares the
/// icon font used everywhere else.
pub fn battery_glyph(pct: i32, charging: bool) -> &'static str {
    if charging {
        return ICON_BATTERY_CHARGING; // material battery_charging_full (bolt in battery)
    }
    match pct {
        0..=10 => ICON_BATTERY_0, // material battery_0_bar
        11..=30 => ICON_BATTERY_3, // material battery_3_bar
        31..=55 => ICON_BATTERY_5, // material battery_5_bar
        56..=80 => ICON_BATTERY_6, // material battery_6_bar
        _ => ICON_BATTERY_FULL,       // material battery_full
    }
}

/// Layout width reserved for the vertical battery glyph at a given font size.
pub fn battery_icon_w(font_size: f32) -> f32 {
    font_size * 0.62
}
