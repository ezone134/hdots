use super::super::*;

// ── Card chrome helpers (minimal / flat / professional pass, 2026-09-14) ──
//
// One header style, one metric-row style, one empty state for ALL cards.
// Type ramp: title 11.5 Semibold · body/labels 8.5-9.5 Regular · values Mono
// · hero numerals Display Medium. Colors: labels `fg2`, meta/units `fg3`,
// numbers `fg` (mono); accent reserved for live data and active states.

/// Uniform card header: semibold title at (x+pad, y+10) — or after a quiet
/// icon when `icon` is Some (icon drawn in `fg2`, NEVER accent-tinted —
/// accent is reserved for live data). Optional right-aligned meta (units /
/// peak / count) in `fg3`, mono when `mono`. The ONLY header style.
///
/// The edit-mode "Card titles" / "Card glyphs" toggles (`shell.card_show_title`
/// / `shell.card_show_glyph`, both ON by default) hide the title text / the
/// header icon on EVERY card; the right-aligned meta stays (it carries real
/// data). With the glyph hidden the title slides left into its slot.
pub(crate) fn card_header(
    v: &mut Vec<Cmd>, pal: &Pal, x: f32, y: f32, w: f32, pad: f32,
    title: &str, icon: Option<&str>, meta: Option<(&str, bool)>, scale: crate::shell::GridScale,
    show_title: bool, show_glyph: bool,
) {
    let tx = if let Some(glyph) = icon {
        if show_glyph {
            ui::text(v, x + pad, y + scale.s(8.0), glyph, scale.fs(12.0), ui::fg2(pal), true);
            x + pad + scale.s(17.0)
        } else {
            x + pad
        }
    } else {
        x + pad
    };
    if show_title {
        ui::title(v, tx, y + scale.s(9.0), title, scale.fs(11.5), pal.fg);
    }
    if let Some((meta, mono)) = meta {
        let mx = x + w - pad;
        let my = y + scale.s(11.0);
        if mono {
            ui::text_r_mono(v, mx, my, meta, scale.fs(8.5), ui::fg3(pal));
        } else {
            ui::caption_r(v, mx, my, meta, scale.fs(8.5), ui::fg3(pal), false);
        }
    }
}

/// Guarded header: renders the card header UNLESS the card's declarative
/// scene supplies its own `Header` item (`scene_owns_header` is set during
/// the scene draw). Declarative-chrome cards therefore get zero duplicate
/// title — the scene owns it. Built-in cards pass `self.scale` etc. through.
impl Shell {
    pub(crate) fn card_head(
        &self, v: &mut Vec<Cmd>, pal: &Pal, x: f32, y: f32, w: f32, pad: f32,
        title: &str, icon: Option<&str>, meta: Option<(&str, bool)>,
    ) {
        if self.scene_owns_header {
            return;
        }
        card_header(v, pal, x, y, w, pad, title, icon, meta, self.scale, self.card_show_title, self.card_show_glyph);
    }
}

/// Metric row: label left in `fg2` (Regular — small sizes read better
/// up-weighted), mono value right-aligned at `right_x`. `label_x`/`right_x`
/// are explicit so rows work in split layouts (cpu/mem right columns) as
/// well as full-width rows (disk). `col` overrides the value color
/// (thresholds via `ui::pct_color`); pass `pal.fg` for plain values.
pub(crate) fn metric_row(
    v: &mut Vec<Cmd>, pal: &Pal, label_x: f32, y: f32, right_x: f32,
    label: &str, value: &str, col: u32, scale: crate::shell::GridScale,
) {
    ui::caption(v, label_x, y, label, scale.fs(8.5), ui::fg2(pal), false);
    ui::text_r_mono(v, right_x, y, value, scale.fs(8.5), col);
}

/// Uniform empty state: centered `fg3` caption — "no battery", "No
/// partitions", "no /sys/class/thermal zones" all share this one style.
pub(crate) fn card_empty(v: &mut Vec<Cmd>, pal: &Pal, x: f32, y: f32, w: f32, h: f32, msg: &str, scale: crate::shell::GridScale) {
    ui::caption_c(v, x + w / 2.0, y + h / 2.0 - scale.s(6.0), msg, scale.fs(9.0), ui::fg3(pal), false);
}

/// Pagination dots under a multi-pane card (hard rule, see NEXT_STEPS.md).
/// One dot per pane; the active pane is filled with the accent color. The
/// dots appear ONLY while the cursor rests on the card (hidden otherwise so
/// the gauge stays clean).
pub(crate) fn battery_dots(v: &mut Vec<Cmd>, pal: &Pal, x: f32, y: f32, w: f32, h: f32, active: usize, step: f32, show: bool) {
    pane_dots(v, pal, x, y, w, h, active, 2, step, show);
}

/// N-pane variant of [`battery_dots`] (world-map card has 4 style panes).
pub(crate) fn pane_dots(v: &mut Vec<Cmd>, pal: &Pal, x: f32, y: f32, w: f32, h: f32, active: usize, panes: usize, step: f32, show: bool) {
    if !show || panes < 2 {
        return;
    }
    let dot = step * 0.66;
    let gap = step;
    let n = panes;
    let total = n as f32 * gap;
    let dx = x + (w - total) / 2.0;
    let dy = y + h - dot - step * 0.5;
    for i in 0..n {
        let c = if i == active { pal.acc } else { ui::hover(&pal) };
        v.push(Cmd::Rect { x: dx + i as f32 * gap, y: dy, w: dot, h: dot, r: dot / 2.0, color: c });
    }
}

/// Fill color for a charge level (shared by both battery cards).
pub(crate) fn battery_level_color(pal: &Pal, pct: i32) -> u32 {
    if pct >= 50 {
        ui::OK
    } else if pct >= 20 {
        pal.acc
    } else {
        ui::DANGER
    }
}

/// A proper battery drawn from vector shapes — not a font glyph. Case =
/// rounded outline, terminal nub on the free end, colored by charge level.
/// The interior is a WATER fill: the liquid spans the FULL inner width so
/// its left/right edges touch the case stroke, rises with the charge, and
/// carries a gentle wave crest + highlight on its surface (reference:
/// liquid-battery icon). `vertical` = standing battery (nub on top, water
/// climbs up), else flat battery (nub on the right, water grows
/// left→right).
pub(crate) fn push_battery_icon(v: &mut Vec<Cmd>, pal: &Pal, cx: f32, cy: f32, w: f32, h: f32, pct: i32, vertical: bool) {
    let line = ui::fg2(pal);
    let body = ui::hover(pal);
    let fill_c = battery_level_color(pal, pct);
    let fill = (pct as f32 / 100.0).clamp(0.0, 1.0);
    if vertical {
        // terminal nub on top
        let nub_w = w * 0.5;
        let nub_h = h * 0.05;
        v.push(Cmd::Rect {
            x: cx - nub_w / 2.0, y: cy - h / 2.0 - nub_h, w: nub_w, h: nub_h, r: nub_h / 2.0, color: line,
        });
        // case outline
        let r = w * 0.16;
        let stroke = w.clamp(1.2, 2.2);
        v.push(Cmd::Outline { x: cx - w / 2.0, y: cy - h / 2.0, w, h, r, width: stroke, color: line });
        draw_water(
            v, fill, fill_c, body,
            cx - w / 2.0, cy - h / 2.0, w, h, stroke, r, stroke, true,
        );
    } else {
        // terminal nub on the right
        let nub_w = w * 0.05;
        let nub_h = h * 0.5;
        v.push(Cmd::Rect {
            x: cx + w / 2.0, y: cy - nub_h / 2.0, w: nub_w, h: nub_h, r: nub_w / 2.0, color: line,
        });
        let r = h * 0.16;
        let stroke = h.clamp(1.2, 2.2);
        v.push(Cmd::Outline { x: cx - w / 2.0, y: cy - h / 2.0, w, h, r, width: stroke, color: line });
        draw_water(
            v, fill, fill_c, body,
            cx - w / 2.0, cy - h / 2.0, w, h, stroke, r, stroke, false,
        );
    }
}

/// The water interior shared by both battery orientations: an empty-interior
/// tint, the liquid body spanning the FULL inner width (edges against the
/// case stroke), a lighter wave crest along its surface and a thin highlight
/// just below the crest. `rise` = vertical battery (water climbs, surface is
/// horizontal); else horizontal (water grows left→right, surface is vertical).
#[allow(clippy::too_many_arguments)]
fn draw_water(
    v: &mut Vec<Cmd>, fill: f32, fill_c: u32, body: u32,
    bx: f32, by: f32, w: f32, h: f32, stroke: f32, r: f32, _round: f32, rise: bool,
) {
    // inner rect — just inside the (edge-centered) stroke
    let in_x = bx + stroke * 0.75;
    let in_y = by + stroke * 0.75;
    let in_w = w - stroke * 1.5;
    let in_h = h - stroke * 1.5;
    // full inner cross-section in BOTH axes: the water touches the left and
    // right stroke (vertical) / top and bottom stroke (horizontal)
    let cross_w = if rise { in_w } else { in_w * fill };
    let cross_h = if rise { in_h * fill } else { in_h };
    let fr = r * 0.55; // follow the case corners a little
    // backwash: interior tint behind the water so the case reads as a vessel
    v.push(Cmd::Rect { x: in_x, y: in_y, w: in_w, h: in_h, r: fr, color: body });
    if cross_w < 1.5 || cross_h < 1.5 {
        return;
    }
    let (wx, wy) = if rise {
        (in_x, in_y + (in_h - cross_h))
    } else {
        (in_x, in_y)
    };
    v.push(Cmd::Rect { x: wx, y: wy, w: cross_w, h: cross_h, r: fr.min(cross_w / 2.0).min(cross_h / 2.0), color: fill_c });
    // wave crest across the surface + a lighter highlight band under it —
    // inset from the edges by the corner radius so the ripple never pokes
    // outside the case's rounded corners
    let crest = mix(fill_c, 0xffffff, 0.45);
    let hl = mix(fill_c, 0xffffff, 0.22);
    let segs = 8;
    let amp = (if rise { in_w } else { in_h }).min(6.0) * 0.28 + 0.4;
    if rise {
        // horizontal surface at the water top
        let sx0 = wx + fr;
        let sx1 = wx + cross_w - fr;
        if sx1 > sx0 {
            let step = (sx1 - sx0) / segs as f32;
            for i in 0..segs {
                let x0 = sx0 + i as f32 * step;
                let x1 = x0 + step;
                let y_mid = wy + amp * (i as f32 * 1.7).sin();
                let y_next = wy + amp * ((i + 1) as f32 * 1.7).sin();
                v.push(Cmd::Line { x0, y0: y_mid, x1, y1: y_next, w: 1.4, color: crest });
            }
            // highlight under the crest
            v.push(Cmd::Line { x0: sx0, y0: wy + amp * 2.2, x1: sx1, y1: wy + amp * 2.2, w: 1.0, color: hl });
        }
    } else {
        // vertical surface at the water's leading edge (right side)
        let sx = wx + cross_w;
        let sy0 = wy + fr;
        let sy1 = wy + cross_h - fr;
        if sy1 > sy0 {
            let step = (sy1 - sy0) / segs as f32;
            for i in 0..segs {
                let y0 = sy0 + i as f32 * step;
                let y1 = y0 + step;
                let x_mid = sx + amp * (i as f32 * 1.7).sin();
                let x_next = sx + amp * ((i + 1) as f32 * 1.7).sin();
                v.push(Cmd::Line { x0: x_mid, y0, x1: x_next, y1, w: 1.4, color: crest });
            }
            v.push(Cmd::Line { x0: sx + amp * 2.2, y0: sy0, x1: sx + amp * 2.2, y1: sy1, w: 1.0, color: hl });
        }
    }
}

impl Shell {

    /// Shared battery-card chrome: header, no-battery guard. Returns the
    /// charge % (or None when no battery is present).
    pub(crate) fn battery_frame(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal, title: &str) -> Option<i32> {
        self.load_power_save();
        let pad = self.scale.s(12.0);
        self.card_head(v, pal, x, y, w, pad, title, Some(ICON_BATTERY), None);
        let pct = self.battery;
        if pct < 0 {
            card_empty(v, &pal, x, y, w, h, "no battery", self.scale);
            return None;
        }
        Some(pct)
    }

    /// Shared battery card pane 1 (extra info) with the power-save toggle.
    pub(crate) fn battery_info_pane(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal, pct: i32) {
        let pad = self.scale.s(12.0);
        let hdr = self.scale.s(24.0);
        let dots_w = 2.0 * self.scale.s(7.0) + pad;
        let rows: Vec<(String, String)> = vec![
            ("State".into(), if self.ac_online { "Charging \u{26a1}".into() } else { "Discharging".into() }),
            ("Battery".into(), format!("{pct}%")),
            ("Draw".into(), if self.battery_watts > 0.0 { format!("{:.1} W", self.battery_watts) } else { "\u{2014}".into() }),
            ("Remaining".into(), if self.battery_time.is_empty() { "\u{2014}".into() } else { self.battery_time.clone() }),
        ];
        let mut ry = y + hdr + self.scale.s(6.0);
        for (lbl, val) in &rows {
            if ry + self.scale.s(18.0) > y + h - self.scale.s(26.0) {
                break;
            }
            ui::text(v, x + pad, ry, lbl, self.scale.fs(9.0), ui::fg3(&pal), false);
            ui::text_r(v, x + w - pad - dots_w, ry, val, self.scale.fs(9.5), pal.fg, false);
            ry += self.scale.s(18.0);
        }
        // power-save toggle (bottom-left, above the dots)
        let key = crate::shell::BATTERY_PSAVE_KEY;
        let psv = self.power_save_on;
        let bh = self.scale.s(22.0);
        let bw2 = (w - pad * 2.0 - dots_w).max(self.scale.s(50.0));
        let bxx = x + pad;
        let byy = y + h - bh - self.scale.s(2.0);
        let hov = self.hover_key == key;
        v.push(Cmd::Rect {
            x: bxx,
            y: byy,
            w: bw2,
            h: bh,
            r: bh / 2.0,
            color: if psv { ui::acc_tint(&pal) } else if hov { ui::raised_hl(&pal) } else { ui::raised(&pal) },
        });
        let glyph = if psv { ICON_REFRESH } else { ICON_BATTERY_SAVER };
        let lbl = if psv { "Power save on" } else { "Power save" };
        ui::text(v, bxx + self.scale.s(24.0), byy + self.scale.s(6.5), format!("{glyph} {lbl}"), self.scale.fs(9.0), if psv { pal.acc } else { pal.fg }, false);
        self.region(bxx - self.scale.s(3.0), byy - self.scale.s(3.0), bw2 + self.scale.s(6.0), bh + self.scale.s(6.0), key);
    }

    /// Draw one centered power button (fixed size, its own background only —
    /// never spans the card); the hold-to-confirm liquid fill rises inside it.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn power_btn(&mut self, v: &mut Vec<Cmd>, cx: f32, cy: f32, d: f32, i: u32, glyph: &str, danger: bool, pal: &Pal) {
        use crate::shell::POWER_CARD_KEY_BASE;
        let key = POWER_CARD_KEY_BASE + i;
        let hov = self.hover_key == key;
        let holding = self.power_hold == Some(key);
        let x = cx - d / 2.0;
        let y = cy - d / 2.0;
        v.push(Cmd::Rect {
            x,
            y,
            w: d,
            h: d,
            r: self.scale.s(10.0),
            color: if holding {
                ui::acc_tint(&pal)
            } else if hov {
                ui::raised_hl(&pal)
            } else {
                ui::raised(&pal)
            },
        });
        ui::outline(v, x, y, d, d, self.scale.s(10.0), ui::hairline(&pal));
        if holding {
            let fill = self.power_hold_fill();
            let fh = d * fill;
            if fh >= self.scale.s(1.5) {
                v.push(Cmd::Rect { x: x + self.scale.s(2.0), y: y + d - self.scale.s(2.0) - fh, w: d - self.scale.s(4.0), h: fh, r: self.scale.s(8.0), color: mix(ui::hover(&pal), pal.acc, 0.5) });
            }
        }
        let gc = if danger { mix(RED, pal.fg, 0.25) } else { pal.fg };
        ui::text_c(v, cx, cy, glyph, self.scale.fs(14.0), gc, true);
        self.region(x, y, d, d, key);
    }

    /// The five power actions shared by the vertical / horizontal power-menu
    /// cards: icon, label, and whether it's a danger (hold-to-confirm) action.
    pub(crate) fn power_card_items() -> [(&'static str, bool); 5] {
        [
            (ICON_LOCK, false), // lock
            (ICON_LOGOUT, true),  // logout
            (ICON_MOON, false), // sleep
            (ICON_REFRESH, true),  // restart
            (ICON_POWER, true),  // shutdown
        ]
    }

        /// Bead ring around a centre point — `lit` beads (of 36) are lit.
        pub(crate) fn ring_beads(v: &mut Vec<Cmd>, cx: f32, cy: f32, radius: f32, dot_d: f32, lit: i32, col: u32, dim: u32) {
            const BEADS: i32 = 36;
            for b in 0..BEADS {
                let ang = (b as f32 / BEADS as f32) * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
                let dx = cx + ang.cos() * radius - dot_d / 2.0;
                let dy = cy + ang.sin() * radius - dot_d / 2.0;
                v.push(Cmd::Rect { x: dx, y: dy, w: dot_d, h: dot_d, r: dot_d / 2.0, color: if b < lit { col } else { dim } });
            }
        }

}
