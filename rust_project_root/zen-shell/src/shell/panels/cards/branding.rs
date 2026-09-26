use super::super::*;
use super::shared::*;

impl Shell {

    /// BRANDING — the brand mark (`$states2/d` rendered in the family named
    /// by `$states2/branding_font`, default "opensuse"/"o") shown as a quiet
    /// sign. The glyph is re-read from `$states2/d` on every draw, so editing
    /// the id shows up with no restart (the shared `brand_glyph` the pill /
    /// banner chips draw also picks it up). The glyph is contain-fit inside the
    /// card: it scales to the SMALLER of the horizontal slot (estimated
    /// advance, so it can never overflow the width) or the full card height,
    /// keeps its own aspect, and sits centered with a generous padding. Never
    /// stretches to fill.
    pub(crate) fn draw_branding_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        self.brand_glyph = crate::vars::read_branding_glyph();
        let pad = self.scale.s(16.0);
        let hdr = self.scale.s(24.0);
        self.card_head(v, pal, x, y, w, pad, "Brand", Some(ICON_SPARKLE), None);

        let avail_w = (w - pad * 2.0).max(1.0);
        let avail_h = (h - hdr - pad).max(1.0);
        if self.brand_glyph.is_empty() {
            card_empty(v, pal, x, y, w, h, "no brand glyph in $states2/d", self.scale);
            return;
        }
        // aspect-fit: per-char advance ~0.62 em, so a single glyph can use
        // ~full height and a multi-char word shrinks to fit the width first.
        let chars = self.brand_glyph.chars().count().max(1) as f32;
        let size = ((avail_w / (chars * 0.62)).min(avail_h) * 0.95).max(10.0);
        let gx = x + w / 2.0;
        let gy = y + hdr + (avail_h - size) / 2.0;
        ui::text_cw(v, gx, gy, &self.brand_glyph, size, crate::text::Ff::Brand, crate::text::Fw::Regular, pal.fg, false);
    }
}