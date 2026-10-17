use super::super::*;

// ── Card chrome helpers (minimal / flat / professional pass, 2026-09-14) ──
//
// One header style, one metric-row style, one empty state for ALL cards.
// Type ramp: title 11.5 Semibold · body/labels 8.5-9.5 Regular · values Mono
// · hero numerals Display Medium. Colors: labels `fg2`, meta/units `fg3`,
// numbers `fg` (mono); accent reserved for live data and active states.
//
// Every card now paints its own chrome from its Lua module, so the shared
// `card_header` / `card_head` / `card_empty` / `battery_dots` / `pane_dots`
// Rust painters have been retired. What remains here is the state the modules
// still depend on (the `Dots` hover gate) and the shared power-menu items.

/// Pagination dots under a multi-pane card (hard rule, see NEXT_STEPS.md).
/// One dot per pane; the active pane is filled with the accent color. The
/// dots appear ONLY while the cursor rests on the card (hidden otherwise so
/// the gauge stays clean). The painted version now lives in each Lua module
/// (`ctx.api.dots_show` gates the hover); `battery_dots_hover` still gates the
/// declarative `Dots`-style scene item on the `.ron` battery cards.

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
