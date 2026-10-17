//! The Settings app frame through the widget system (`WIDGETS.md` stage 6 —
//! surfaces): header strip, close chip, sidebar divider and the six nav rows,
//! rebuilt per frame from the frame's inputs.
//!
//! Layer law: this file sees NO shell internals — it receives the granted
//! rect, the palette, the selected tab and the pointer's hover key, and
//! derives every color through the `crate::ui::*` palette helpers at tree
//! build time (per-frame rebuild, so a palette hot-reload lands next frame).
//!
//! Parity: the tree reproduces the retired `draw_settings_chrome_native`
//! byte for byte (same `ui.rs` text/rect helpers, same coordinates) — the
//! test that holds the two side by side lives next to the drawer.
//!
//! Coordinate idiom: the header and the sidebar are two overlay [`Node::Box`]
//! children of the root column; because a Box's children all share ONE
//! content rect, each nested flow container positions itself with margins —
//! exact absolute placement, flex for the pieces inside.

use crate::icons::{ICON_MONITOR, ICON_PALETTE, ICON_SETTINGS, ICON_VOLUME, ICON_WIFI};
use crate::shell::panels::settings::{NAV_APPEARANCE, NAV_MISC, NAV_NETWORK, NAV_PILL, NAV_SOUND, NAV_SYSTEM};
use crate::shell::{SETTINGS_SIDEBAR, SettingsTab};
use crate::ui::Pal;
use crate::widgets::{HAlign, Node, VAlign};

/// One sidebar row: (nav key, glyph, label, tab).
const NAV: [(u32, &str, &str, SettingsTab); 6] = [
    (NAV_NETWORK, ICON_WIFI, "Network", SettingsTab::Network),
    (NAV_SOUND, ICON_VOLUME, "Sound & Display", SettingsTab::Sound),
    (NAV_APPEARANCE, ICON_PALETTE, "Appearance", SettingsTab::Appearance),
    (NAV_PILL, ICON_PALETTE, "Pill", SettingsTab::Pill),
    (NAV_SYSTEM, ICON_MONITOR, "System", SettingsTab::System),
    (NAV_MISC, ICON_SETTINGS, "Misc", SettingsTab::Misc),
];

/// Build the whole frame tree for the granted `(w, h)` rect.
pub(crate) fn frame(w: f32, h: f32, pal: &Pal, tab: SettingsTab, hover_key: u32) -> Node {
    let side = SETTINGS_SIDEBAR;
    let mut root = Node::col();
    root = root.child(header(w, h, pal, hover_key));
    root = root.child(sidebar(w, h, pal, side, tab, hover_key));
    root
}

/// Top chrome strip (0..44): title at (22, 14), close chip + ✕ at the right,
/// close hit at (w−42, 4, 36, 30) — the drawer's numbers verbatim.
fn header(w: f32, _h: f32, pal: &Pal, hover_key: u32) -> Node {
    let close_hov = hover_key == 1;
    // the fixed(44) reserves the strip (the h grant is irrelevant to it); the
    // box's children overlay its rect, each placed by its own margin
    let mut strip = Node::box_(0.0, 0).fixed(44.0);
    strip = strip.child(
        Node::row()
            .margin(22.0, 14.0, 0.0, 0.0)
            .child(Node::label("Settings", 17.0, pal.fg).valign(VAlign::Top).fixed(140.0)),
    );
    // hovered: the chip's wash rect; resting: bg 0 paints nothing
    strip = strip.child(
        Node::row()
            .margin(w - 42.0, 6.0, 0.0, 12.0)
            .child(
                Node::box_(6.0, if close_hov { crate::ui::hover_hl(pal) } else { 0 })
                    .fixed(30.0),
            ),
    );
    strip = strip.child(
        Node::row()
            .margin(w - 42.0, 13.0, 0.0, 0.0)
            .child(
                // right: true anchored at the slot's right edge (w−26),
                // exactly what ui::text_r emitted
                Node::label(
                    "✕",
                    12.0,
                    if close_hov { pal.fg } else { crate::ui::fg2(pal) },
                )
                .valign(VAlign::Top)
                .halign(HAlign::Right)
                .fixed(16.0),
            ),
    );
    strip = strip.child(
        Node::row()
            .margin(w - 42.0, 4.0, 0.0, 10.0)
            .child(Node::hit(1).fixed(36.0)),
    );
    strip
}

/// Sidebar area (44..h): divider wash + hairline, then the six nav rows —
/// selection wash, accent rail, glyph + label, hit region, in the drawer's
/// stacking order.
fn sidebar(w: f32, h: f32, pal: &Pal, side: f32, tab: SettingsTab, hover_key: u32) -> Node {
    let rest_h = h - 44.0;
    let tint = (pal.bg & 0xff_ff_ff_00) | 0x08;
    // the wash covers the sidebar width only — the root column stretches
    // every child, so the right margin clips the box back to `side`
    let mut sidebar = Node::box_(0.0, tint)
        .fixed(rest_h)
        .margin(0.0, 0.0, w - side, 0.0);
    // hairline splitting sidebar from content, at x = side−1
    sidebar = sidebar.child(
        Node::row()
            .margin(side - 1.0, 0.0, 0.0, 0.0)
            .child(Node::box_(0.0, crate::ui::hairline(pal)).fixed(1.0)),
    );
    for &(key, glyph, label_text, row_tab) in NAV.iter() {
        let y = 56.0 + row_of(key) * 46.0;
        // geometry relative to the sidebar's own top (44)
        let dy_sel = y - 2.0 - 44.0;
        let dy_txt = y + 13.0 - 44.0;
        let dy_rail = y + 8.0 - 44.0;
        let sel = tab == row_tab;
        let hov = hover_key == key;
        if sel || hov {
            let fill = if sel {
                crate::ui::hover_hl(pal)
            } else {
                crate::ui::hover(pal)
            };
            let mut row = Node::box_(0.0, 0);
            row = row.child(
                Node::col().child(
                    Node::box_(8.0, fill)
                        .fixed(40.0)
                        .margin(6.0, dy_sel, 6.0, 0.0),
                ),
            );
            sidebar = sidebar.child(row);
        }
        // glyph + label — one row, each label placed by margin
        let glyph_ink = if sel {
            pal.acc
        } else if hov {
            pal.fg
        } else {
            crate::ui::fg2(pal)
        };
        let label_ink = if sel || hov { pal.fg } else { crate::ui::fg2(pal) };
        let mut texts = Node::row();
        texts = texts.child(
            Node::glyph(glyph, 14.0, glyph_ink)
                .valign(VAlign::Top)
                .fixed(26.0)
                .margin(16.0, dy_txt, 0.0, 0.0),
        );
        texts = texts.child(
            Node::label(label_text, 12.5, label_ink)
                .valign(VAlign::Top)
                .fixed(side - 58.0)
                .margin(0.0, dy_txt, 0.0, 0.0),
        );
        let mut holder = Node::box_(0.0, 0);
        holder = holder.child(texts);
        sidebar = sidebar.child(holder);
        if sel {
            // 2px accent rail on the sidebar's left edge
            let mut holder = Node::box_(0.0, 0);
            let mb = (rest_h - dy_rail - 20.0).max(0.0);
            holder = holder.child(
                Node::row().child(
                    Node::box_(1.0, pal.acc)
                        .fixed(2.0)
                        .margin(0.0, dy_rail, side - 2.0, mb),
                ),
            );
            sidebar = sidebar.child(holder);
        }
        // hit region: (2, y−2, side−4, 40) — the drawer's registered rect
        let mut holder = Node::box_(0.0, 0);
        holder = holder.child(
            Node::row().child(
                Node::hit(key)
                    .fixed(side - 4.0)
                    .margin(2.0, dy_sel, 0.0, (rest_h - dy_sel - 40.0).max(0.0)),
            ),
        );
        sidebar = sidebar.child(holder);
    }
    sidebar
}

/// Sidebar row index (row order follows the tab list).
fn row_of(key: u32) -> f32 {
    NAV.iter()
        .position(|(k, _, _, _)| *k == key)
        .unwrap_or(0) as f32
}
