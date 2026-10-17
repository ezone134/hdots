//! Settings tab bodies through the widget system (`WIDGETS.md` stage 6) —
//! first: NETWORK & INTERNET (Wi-Fi / Bluetooth rows).
//!
//! Layer law: no shell internals here — the body builder receives the
//! granted rect, the palette, the render scale and the two rows' plain
//! inputs (their state strings/flags/hover); everything else (geometry,
//! colors) derives from `crate::ui::*` palette helpers at build time.
//!
//! Coordinates are BASE px multiplied by the render scale — the same scale
//! the scene engine applied when the declarative body drew these rows.
//!
//! Parity: the adjacent test draws BOTH the declarative stack (the engine,
//! from the const, via the merged scene pool) and this tree and holds the
//! two `Vec<Cmd>` streams and their hit regions to identical bytes.

use crate::ui::Pal;
use crate::widgets::{Node, VAlign};

/// One radio row of the Network body: parked at `y`, state-driven toggle,
/// subtitle and inks resolved by its `on`/`hov` flags.
pub(crate) struct RadioRow {
    pub on: bool,
    /// hover gate: the row region OR its switch region is under the pointer
    pub hov: bool,
    /// the row's second line ("<ssid>" / "Not connected" / device / "On" / "Off")
    pub sub: String,
    pub glyph: &'static str,
    pub title: &'static str,
    /// hit key of the row region (toggles the radio)
    pub key: u32,
    /// hit key of the switch track right of the title
    pub switch_key: u32,
    pub y: f32,
}

/// Build the Network body tree for the granted `(w, h)` rect (device px).
///
/// Element placement rides the overlay-Box margin idiom: the root is a
/// transparent Box; every child's own margin places its rect inside it —
/// `(l, t, r, b)` so the granted inset is exactly the declarative item's
/// `(x, y, w, h)` in device px.
pub(crate) fn network(w: f32, h: f32, pal: &Pal, scale: f32, rows: [RadioRow; 2]) -> Node {
    let mut root = Node::box_(0.0, 0);
    // the section header both rows hang under (declared first)
    root = root.child(section_head(pal, scale));
    for row in &rows {
        root = root.child(radio_row(w, h, pal, scale, row));
    }
    root
}

/// "NETWORK & INTERNET" at (188, 50), fg3, 9.5 UI light.
fn section_head(pal: &Pal, scale: f32) -> Node {
    Node::label("NETWORK & INTERNET", 9.5 * scale, crate::ui::fg3(pal))
        .valign(VAlign::Top)
        .margin(188.0 * scale, 50.0 * scale, 0.0, 0.0)
}

/// One radio row, in the declarative order the const listed: wash
/// (hover-gated), icon tile, glyph, title, subtitle, track, knob, then the
/// two hits.
fn radio_row(w: f32, h: f32, pal: &Pal, scale: f32, row: &RadioRow) -> Node {
    let s = |v: f32| v * scale;
    let x = s(188.0);
    let y = s(row.y);
    let mut stack = Node::box_(0.0, 0);
    // hover wash — full row width, right inset 8, only under the pointer
    if row.hov {
        stack = stack.child(
            Node::box_(s(8.0), crate::ui::hover_hl(pal))
                .margin(x, y, s(8.0), h - y - s(48.0)),
        );
    }
    // 30² icon tile on the row's 20%-hover well
    stack = stack.child(
        Node::box_(s(7.0), crate::ui::hover(pal))
            .margin(s(196.0), s(row.y + 9.0), w - s(226.0), h - s(row.y + 39.0)),
    );
    // leading glyph inside the tile (state ink: accent when on, fg3 when off)
    stack = stack.child(
        Node::glyph(
            row.glyph,
            s(14.0),
            if row.on { pal.acc } else { crate::ui::fg3(pal) },
        )
        .valign(VAlign::Top)
        .margin(s(203.0), s(row.y + 17.0), 0.0, 0.0),
    );
    stack = stack.child(
        Node::label(row.title, s(13.0), pal.fg)
            .valign(VAlign::Top)
            .margin(s(236.0), s(row.y + 9.0), 0.0, 0.0),
    );
    stack = stack.child(
        Node::label(row.sub.as_str(), s(10.5), crate::ui::fg3(pal))
            .valign(VAlign::Top)
            .margin(s(236.0), s(row.y + 27.0), 0.0, 0.0),
    );
    // toggle track: 38×20 r10 — accent when on, the drawer's mix(bg,fg,0.14)
    // blend when off — right edge pinned 24 base px inside the surface
    stack = stack.child(
        Node::box_(
            s(10.0),
            if row.on {
                pal.acc
            } else {
                crate::ui::mix(pal.bg, pal.fg, 0.14)
            },
        )
        .margin(
            w - s(62.0),
            s(row.y + 14.0),
            s(24.0),
            h - s(row.y + 14.0) - s(20.0),
        ),
    );
    // knob: 16² r8 — on: 25 in from the right; off: 44 (travel 19 base px);
    // white filled circle when on, the quiet fg3 when off
    let knob_in = if row.on { 25.0 } else { 44.0 };
    stack = stack.child(
        Node::box_(s(8.0), if row.on { 0xff_ff_ff_ff } else { crate::ui::fg3(pal) })
            .margin(
                w - s(knob_in + 16.0),
                s(row.y + 16.0),
                s(knob_in),
                h - s(row.y + 16.0) - s(16.0),
            ),
    );
    // the whole row toggles the radio; the switch region toggles with it
    stack = stack.child(
        Node::hit(row.key).margin(x, y, s(66.0), h - y - s(48.0)),
    );
    stack = stack.child(
        Node::hit(row.switch_key).margin(
            w - s(74.0),
            s(row.y + 12.0),
            s(32.0),
            h - s(row.y + 12.0) - s(24.0),
        ),
    );
    stack
}
