use super::super::*;

impl Shell {
    /// The accent card's content anchor — `((h − s(156)) / 2).clamp(s(30),
    /// s(48))` in DEVICE px.
    ///
    /// ONE formula for the card's height-dependent top: the drawer that
    /// draws the block, `draw_card_scene`'s stamp, and the `scene_values`
    /// pool (which unscales this back to base for the scene's published
    /// y's) all come here. A second copy is how the drawer and the scene
    /// end up measuring two different cards.
    pub(crate) fn accent_top(&self, h: f32) -> f32 {
        ((h - self.scale.s(156.0)) * 0.5).clamp(self.scale.s(30.0), self.scale.s(48.0))
    }

    /// The paged custom-accents list's box inside a card box — the drawer's
    /// `x0 / ly / ww / lh`, in DEVICE px.
    ///
    /// Three publishers of this geometry call THIS: the drawer that draws
    /// the list, `draw_card_scene`'s `accent_list_rect` stamp (the wheel and
    /// the row clicks read that), and `scene_values`, which clamps the
    /// published scroll against the same viewport. A second copy would let
    /// the window that scrolls drift from the window that draws — the split
    /// that silently clicks the wrong accent.
    pub(crate) fn accent_list_box_in(&self, x: f32, y: f32, w: f32, h: f32) -> (f32, f32, f32, f32) {
        let ly = y + self.accent_top(h) + self.scale.s(122.0);
        let lh = (y + h - self.scale.s(4.0) - ly).max(8.0);
        (x + self.scale.s(10.0), ly, w - self.scale.s(20.0), lh)
    }

    /// How many 26 px rows fit a `lh`-tall list viewport — never fewer
    /// than 1, the drawer's `((lh − s(4)) / row_h).floor().max(1)`.
    pub(crate) fn accent_list_visible_in(&self, lh: f32) -> usize {
        ((lh - self.scale.s(4.0)) / self.scale.s(26.0))
            .floor()
            .max(1.0) as usize
    }

    /// The window's first row: the scroll offset clamped so the window
    /// never runs past the accents that exist.
    pub(crate) fn accent_list_start_in(&self, lh: f32) -> usize {
        let visible = self.accent_list_visible_in(lh);
        self.accent_list_scroll
            .min(self.custom_accs.len().saturating_sub(visible))
    }

}
