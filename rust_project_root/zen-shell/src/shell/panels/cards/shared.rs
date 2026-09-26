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
/// The battery gauge's art and level ladder live in the scene engine, where
/// the declarative `Battery` item draws them; the Rust drawers below are the
/// fallback and share the exact same recipes.
pub(crate) use crate::scene::{battery_level_color, push_battery_icon};

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

impl Shell {

    /// Whether the cursor rests on a battery card (the `Dots` hover gate).
    /// The rect is published by `draw_card_scene` when the scene owns the
    /// card, exactly like the sliders card.
    pub(crate) fn battery_dots_hover(&self, rect: (f32, f32, f32, f32)) -> bool {
        let (x, y, w, h) = rect;
        !self.dash_edit
            && w > 0.0
            && self
                .cursor
                .map(|(px, py)| Self::in_rect(px, py, (x, y, w, h)))
                .unwrap_or(false)
    }

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
