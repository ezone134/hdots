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
    v: &mut Vec<Cmd>,
    pal: &Pal,
    x: f32,
    y: f32,
    w: f32,
    pad: f32,
    title: &str,
    icon: Option<&str>,
    meta: Option<(&str, bool)>,
    scale: crate::shell::GridScale,
    show_title: bool,
    show_glyph: bool,
) {
    let tx = if let Some(glyph) = icon {
        if show_glyph {
            ui::text(
                v,
                x + pad,
                y + scale.s(8.0),
                glyph,
                scale.fs(12.0),
                ui::fg2(pal),
                true,
            );
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
        &self,
        v: &mut Vec<Cmd>,
        pal: &Pal,
        x: f32,
        y: f32,
        w: f32,
        pad: f32,
        title: &str,
        icon: Option<&str>,
        meta: Option<(&str, bool)>,
    ) {
        if self.scene_owns_header {
            return;
        }
        card_header(
            v,
            pal,
            x,
            y,
            w,
            pad,
            title,
            icon,
            meta,
            self.scale,
            self.card_show_title,
            self.card_show_glyph,
        );
    }
}

/// Uniform empty state: centered `fg3` caption — "no battery", "No
/// partitions", "no /sys/class/thermal zones" all share this one style.
pub(crate) fn card_empty(
    v: &mut Vec<Cmd>,
    pal: &Pal,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    msg: &str,
    scale: crate::shell::GridScale,
) {
    ui::caption_c(
        v,
        x + w / 2.0,
        y + h / 2.0 - scale.s(6.0),
        msg,
        scale.fs(9.0),
        ui::fg3(pal),
        false,
    );
}

/// Pagination dots under a multi-pane card (hard rule, see NEXT_STEPS.md).
/// One dot per pane; the active pane is filled with the accent color. The
/// dots appear ONLY while the cursor rests on the card (hidden otherwise so
/// the gauge stays clean).
pub(crate) fn battery_dots(
    v: &mut Vec<Cmd>,
    pal: &Pal,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    active: usize,
    step: f32,
    show: bool,
) {
    pane_dots(v, pal, x, y, w, h, active, 2, step, show);
}

/// N-pane variant of [`battery_dots`] (world-map card has 4 style panes).
pub(crate) fn pane_dots(
    v: &mut Vec<Cmd>,
    pal: &Pal,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    active: usize,
    panes: usize,
    step: f32,
    show: bool,
) {
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
        let c = if i == active { pal.acc } else { ui::hover(pal) };
        v.push(Cmd::Rect {
            x: dx + i as f32 * gap,
            y: dy,
            w: dot,
            h: dot,
            r: dot / 2.0,
            color: c,
        });
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

    /// The five power actions shared by the vertical / horizontal power-menu
    /// cards: icon, label, and whether it's a danger (hold-to-confirm) action.
    pub(crate) fn power_card_items() -> [(&'static str, bool); 5] {
        [
            (ICON_LOCK, false),   // lock
            (ICON_LOGOUT, true),  // logout
            (ICON_MOON, false),   // sleep
            (ICON_REFRESH, true), // restart
            (ICON_POWER, true),   // shutdown
        ]
    }
}
