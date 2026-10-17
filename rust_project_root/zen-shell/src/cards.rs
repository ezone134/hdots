//! Card front-ends on the widget core (`WIDGETS.md` §Layers). A card is ONE
//! pure function: that frame's backend data → a widget [`Node`] tree. Cards
//! never touch `Cmd` or shell state; the palette invariant holds by
//! construction — color *values* pass in, derived tones come from `ui.rs`
//! helpers, and the tree is rebuilt every frame so live palette reloads land
//! on the next render (watch.rs makes the reload itself zero-CPU).
//!
//! PILOT (stage 2): the Clock card. Its committed LuaJIT module is pinned
//! byte-for-byte by the parity test below, and the shell dispatches here
//! INSTEAD of the module (shell.rs `draw_lua_if_any`). Stages 3–7 port every
//! remaining card into this file, then the Lua host dies.

use crate::text::{Ff, Fw};
use crate::ui::Pal;
use crate::widgets::{HAlign, Node, Sizing, VAlign};
use crate::cmd::Cmd;
pub mod accent;
pub mod alarms;
pub mod appshorcut;
pub mod audiodevice;
pub mod bluetooth;
pub mod calendar;
pub mod clipboard;
pub mod clipimg;
pub mod countdown;
pub mod currency;
pub mod eq;
pub mod expenses;
pub mod lyrics;
pub mod media;
pub mod news;
pub mod quote;
pub mod recent;
pub mod snippets;

/// Cards whose whole body is now painted by a native Rust drawer in this file
/// (stage-2 pilots + the stage-3 metric batch). The dashboard's value-pool gate
/// (`shell::card_has_decls`) consults this so a card keeps its declarative
/// status after its `.lua` module is deleted.
pub(crate) fn is_native(_id: &str) -> bool {
    false
}

/// One hit region a native card registers: device-px rect + input key — the
/// same tuple the Lua host's `ctx.ui.hit(...)` collected.
pub(crate) type CardHit = (f32, f32, f32, f32, u32);

/// The horizontal segmented `Bar` primitive, exactly as the Lua host's
/// `ctx.ui.bar` binding emitted it (track rect + fill rect, the fraction
/// clamped into 0..1, sub-0.1% fills culled; `vertical` plugs bottom-up).
#[allow(clippy::too_many_arguments)]
fn bar_prim(
    v: &mut Vec<Cmd>,
    sx: f32,
    sy: f32,
    sw: f32,
    sh: f32,
    r: f32,
    raw: f32,
    max: f32,
    fill: u32,
    track: u32,
    vertical: bool,
) {
    v.push(Cmd::Rect { x: sx, y: sy, w: sw, h: sh, r, color: track });
    let max = if max > 0.0 { max } else { 100.0 };
    let frac = if max > 0.0 {
        (raw / max).clamp(0.0, 1.0)
    } else {
        0.0
    };
    if frac > 0.001 {
        if vertical {
            v.push(Cmd::Rect {
                x: sx,
                y: sy + sh * (1.0 - frac),
                w: sw,
                h: sh * frac,
                r,
                color: fill,
            });
        } else {
            v.push(Cmd::Rect {
                x: sx,
                y: sy,
                w: sw * frac,
                h: sh,
                r,
                color: fill,
            });
        }
    }
}

/// The Clock card — hero time + date line, both centre-anchored, the exact
/// contract the retired drawer and `ui/cards/clock.lua` used:
///   hero time `{clock}` at card centre, y − s(4),  display/medium fs(30)
///   date line `{date}`  at card centre, y − s(22), ui/light      fs(10), fg2
/// `clock` / `date` = `None` mimics the module's `api.x == nil` skips.
///
/// `s(n)` / `fs(n)` reproduce the Lua host's scaling (`n * grid`, size
/// rounded to 0.1) — the card is the place device-px math lives, exactly
/// where the Lua module held it.
pub(crate) fn clock(clock: Option<&str>, date: Option<&str>, grid: f32, pal: &Pal) -> Node {
    let s = |n: f32| n * grid;
    let fs = |n: f32| (n * grid * 10.0).round() / 10.0;

    // overlay container (no fill): every child is granted the card rect and
    // anchors itself from its centre — same shape as the module's draw.
    let mut root = Node::box_(0.0, 0).sizing(Sizing::Expand(1.0));

    if let Some(t) = clock {
        let t = t.to_string();
        root = root.child(
            Node::anchored(0.0, -s(4.0))
                .text_(t)
                .size_(fs(30.0))
                .font(Ff::Display, Fw::Medium)
                .color_(pal.fg),
        );
    }
    if let Some(t) = date {
        let t = t.to_string();
        root = root.child(
            Node::anchored(0.0, -s(22.0))
                .text_(t)
                .size_(fs(10.0))
                .color_(crate::ui::fg2(pal)),
        );
    }
    root
}

/// The Packages card — center-anchored hero figure, the exact contract
/// `ui/cards/pkgupdates.lua` used: pending → DOWN glyph (fs22 icon) at −18 in
/// accent + accent count hero (fs20 display/medium) at +10 + "updates"
/// caption (fs9 fg2) at +34; ok → CHECK glyph in ok-green + fg count +
/// "up to date". `count > 0` / `count == 0` mirror the module's two gates.
pub(crate) fn pkgupdates(count: i32, grid: f32, pal: &Pal) -> Node {
    let s = |n: f32| n * grid;
    let fs = |n: f32| (n * grid * 10.0).round() / 10.0;
    let mut root = Node::box_(0.0, 0).sizing(Sizing::Expand(1.0));
    if count > 0 {
        root = root
            .child(
                Node::anchored(0.0, -s(18.0))
                    .text_("\u{f063}")
                    .size_(fs(22.0))
                    .color_(pal.acc)
                    .icon(),
            )
            .child(
                Node::anchored(0.0, s(10.0))
                    .text_(count.to_string())
                    .size_(fs(20.0))
                    .font(Ff::Display, Fw::Medium)
                    .color_(pal.acc),
            )
            .child(
                Node::anchored(0.0, s(34.0))
                    .text_("updates")
                    .size_(fs(9.0))
                    .color_(crate::ui::fg2(pal)),
            );
    }
    if count == 0 {
        root = root
            .child(
                Node::anchored(0.0, -s(18.0))
                    .text_("\u{f00c}")
                    .size_(fs(22.0))
                    .color_(crate::ui::OK)
                    .icon(),
            )
            .child(
                Node::anchored(0.0, s(10.0))
                    .text_(count.to_string())
                    .size_(fs(20.0))
                    .font(Ff::Display, Fw::Medium)
                    .color_(pal.fg),
            )
            .child(
                Node::anchored(0.0, s(34.0))
                    .text_("up to date")
                    .size_(fs(9.0))
                    .color_(crate::ui::fg2(pal)),
            );
    }
    root
}

/// The KbLayout card — quiet keyboard mark above, layout label below as the
/// hero figure; the exact contract `ui/cards/kblayout.lua` used:
///   Text  U+F11C keyboard glyph (icon font), ui/light fs22, fg2, centred, y − 12
///   Text  `{kb_layout}`, display/medium fs14, fg, centred, y + 15
/// `layout` arrives pre-mapped (the shell arm maps empty → "Unknown",
/// identical to the api builder the Lua path used).
pub(crate) fn kblayout(layout: &str, grid: f32, pal: &Pal) -> Node {
    let s = |n: f32| n * grid;
    let fs = |n: f32| (n * grid * 10.0).round() / 10.0;
    Node::box_(0.0, 0)
        .sizing(Sizing::Expand(1.0))
        .child(
            Node::anchored(0.0, -s(12.0))
                .text_("\u{f11c}")
                .size_(fs(22.0))
                .color_(crate::ui::fg2(pal))
                .icon(),
        )
        .child(
            Node::anchored(0.0, s(15.0))
                .text_(layout.to_string())
                .size_(fs(14.0))
                .font(Ff::Display, Fw::Medium)
                .color_(pal.fg),
        )
}

// Declarative cards live in the GENERATED registry (`card_scenes.rs`): their
// Lua modules were pure SCENE data pumps drawn by the same native engine
// (`scene.rs CardScene::draw`), so retirement = the same table as a JSON
// const + the same engine call, byte parity pinned per card by the
// registry's own runtime test (26 cards adapted in one mechanical pass).

// ── Metric cards (absolute device-px draw, WIDGETS.md stage 3) ──────────────
// The bead-ring metric cards lay their body out at absolute device px (the
// same coordinates their retired Rust drawers used), so they are transcribed
// straight onto the shared `ui::*` text primitives + `scene::bead_ring` the
// Lua host itself calls — `ui.title` IS `ui::title`, so the streams are
// byte-identical and the parity test compares the Lua host's output directly.
// `s(n)` / `fs(n)` reproduce the Lua host's scaling (grid px, size rounded to
// 0.1), exactly where the module held that math.

/// The CPU card — title + mono freq meta, a 36-bead load ring with the hero
/// percent centred beside it, then the load/freq/temp metric rows. Reference
/// widget-tree port (`WIDGETS.md` stage 3): ONE pure function, backend data
/// in, `Node` out.
#[allow(clippy::too_many_arguments)]
pub(crate) fn cpu(
    cpu: i32,
    meta: &str,
    gauge_col: u32,
    lit: i32,
    rows: &[(String, String, u32)],
    show_title: bool,
    w: f32,
    h: f32,
    grid: f32,
    pal: &Pal,
) -> Node {
    let s = |n: f32| n * grid;
    let fs = |n: f32| (n * grid * 10.0).round() / 10.0;
    let p = s(14.0);
    let mut root = Node::box_(0.0, 0);
    if show_title {
        root = root.child(
            Node::label("CPU", fs(11.5), pal.fg)
                .font(Ff::Ui, Fw::Semibold)
                .halign(HAlign::Left)
                .valign(VAlign::Top)
                .margin(s(14.0), s(9.0), 0.0, 0.0),
        );
    }
    if !meta.is_empty() {
        root = root.child(
            Node::label(meta, fs(8.5), crate::ui::fg3(pal))
                .font(Ff::Mono, Fw::Regular)
                .halign(HAlign::Right)
                .valign(VAlign::Top)
                .margin(0.0, s(11.0), p, 0.0),
        );
    }
    root = root.child(Node::beads(
        36,
        lit.max(0) as usize,
        s(4.0),
        s(26.0),
        gauge_col,
        crate::ui::hover(pal),
        s(38.0) - w / 2.0,
        s(13.0),
    ));
    root = root.child(
        Node::anchored(s(38.0) / 2.0, s(6.0))
            .text_(format!("{cpu}%"))
            .size_(fs(15.0))
            .font(Ff::Ui, Fw::Semibold)
            .color_(gauge_col),
    );
    let mut ry = s(30.0);
    for (label, value, color) in rows {
        if ry + s(22.0) > h + 0.5 {
            break;
        }
        root = root.child(
            Node::label(label.as_str(), fs(8.5), crate::ui::fg2(pal))
                .font(Ff::Ui, Fw::Regular)
                .halign(HAlign::Left)
                .valign(VAlign::Top)
                .margin(s(14.0) + s(62.0), ry, 0.0, 0.0),
        );
        if !value.is_empty() {
            root = root.child(
                Node::label(value.as_str(), fs(8.5), *color)
                    .font(Ff::Mono, Fw::Regular)
                    .halign(HAlign::Right)
                    .valign(VAlign::Top)
                    .margin(0.0, ry, p, 0.0),
            );
        }
        ry += s(22.0);
    }
    root
}

/// The Memory card — accent bead ring, hero percent, the four metric rows,
/// and the swap bar along the bottom while swap is in use (widget port;
/// parity pinned against `mem_native`).
#[allow(clippy::too_many_arguments)]
pub(crate) fn mem(
    mem_pct: i32,
    lit: i32,
    rows: &[(String, String)],
    swap_pct: i32,
    swap_show: bool,
    show_title: bool,
    w: f32,
    h: f32,
    grid: f32,
    pal: &Pal,
) -> Node {
    let s = |n: f32| n * grid;
    let fs = |n: f32| (n * grid * 10.0).round() / 10.0;
    let mut root = Node::box_(0.0, 0);
    if show_title {
        root = root.child(
            Node::label("Memory", fs(11.5), pal.fg)
                .font(Ff::Ui, Fw::Semibold)
                .halign(HAlign::Left)
                .valign(VAlign::Top)
                .margin(s(14.0), s(9.0), 0.0, 0.0),
        );
    }
    root = root.child(Node::beads(
        36,
        lit.max(0) as usize,
        s(4.0),
        s(26.0),
        pal.acc,
        crate::ui::hover(pal),
        s(38.0) - w / 2.0,
        s(13.0),
    ));
    root = root.child(
        Node::anchored(s(19.0), s(6.0))
            .text_(format!("{mem_pct}%"))
            .size_(fs(15.0))
            .font(Ff::Ui, Fw::Semibold)
            .color_(pal.fg),
    );
    let mut ry = s(30.0);
    for (label, value) in rows {
        if ry + s(22.0) > h + 0.5 {
            break;
        }
        root = root.child(
            Node::label(label.as_str(), fs(8.5), crate::ui::fg2(pal))
                .font(Ff::Ui, Fw::Regular)
                .halign(HAlign::Left)
                .valign(VAlign::Top)
                .margin(s(14.0) + s(62.0), ry, 0.0, 0.0),
        );
        if !value.is_empty() {
            root = root.child(
                Node::label(value.as_str(), fs(8.5), pal.fg)
                    .font(Ff::Mono, Fw::Regular)
                    .halign(HAlign::Right)
                    .valign(VAlign::Top)
                    .margin(0.0, ry, s(14.0), 0.0),
            );
        }
        ry += s(22.0);
    }
    if swap_show {
        let mut bsw = w - s(28.0);
        if bsw < 2.0 {
            bsw = 2.0;
        }
        let bsh = s(5.0);
        let top = s(124.0);
        root = root.child(
            Node::bar(
                swap_pct as f32,
                100.0,
                pal.acc,
                crate::ui::hover(pal),
                s(2.5),
                false,
            )
            .margin(s(14.0), top, w - s(14.0) - bsw, h - top - bsh),
        );
    }
    root
}

/// The retired Memory drawer — the byte-parity oracle for `mem`.
#[cfg(test)]
#[allow(clippy::too_many_arguments)]
fn mem_native(
    v: &mut Vec<Cmd>,
    mem_pct: i32,
    lit: i32,
    rows: &[(String, String)],
    swap_pct: i32,
    swap_show: bool,
    show_title: bool,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    grid: f32,
    pal: &Pal,
) {
    let s = |n: f32| n * grid;
    let fs = |n: f32| (n * grid * 10.0).round() / 10.0;
    let p = s(14.0);
    if show_title {
        crate::ui::title(v, x + p, y + s(9.0), "Memory", fs(11.5), pal.fg);
    }
    crate::scene::bead_ring(
        v,
        x + s(38.0),
        y + h / 2.0 + s(13.0),
        s(26.0),
        s(4.0),
        36,
        lit.max(0) as usize,
        pal.acc,
        crate::ui::hover(pal),
    );
    let sx = x + s(38.0);
    crate::ui::text_cw(
        v,
        sx + (w - s(38.0)) / 2.0,
        y + h / 2.0 + s(6.0),
        format!("{mem_pct}%"),
        fs(15.0),
        Ff::Ui,
        Fw::Semibold,
        pal.fg,
        false,
    );
    let mut ry = y + s(30.0);
    for (label, value) in rows {
        if ry + s(22.0) > y + h + 0.5 {
            break;
        }
        crate::ui::caption(
            v,
            x + s(14.0) + s(62.0),
            ry,
            label.as_str(),
            fs(8.5),
            crate::ui::fg2(pal),
            false,
        );
        if !value.is_empty() {
            crate::ui::text_r_mono(v, x + w - s(14.0), ry, value.as_str(), fs(8.5), pal.fg);
        }
        ry += s(22.0);
    }
    if swap_show {
        let bsx = x + s(14.0);
        let bsy = y + s(124.0);
        let mut bsw = w - s(28.0);
        if bsw < 2.0 {
            bsw = 2.0;
        }
        let bsh = s(5.0);
        v.push(Cmd::Rect {
            x: bsx,
            y: bsy,
            w: bsw,
            h: bsh,
            r: s(2.5),
            color: crate::ui::hover(pal),
        });
        // The module multiplies in Lua doubles then hands the f32 Cmd field;
        // mirror that exactly (f32 math rounds one ulp away here).
        let frac = (swap_pct as f64 / 100.0).clamp(0.0, 1.0);
        if frac > 0.001 {
            v.push(Cmd::Rect {
                x: bsx,
                y: bsy,
                w: (bsw as f64 * frac) as f32,
                h: bsh,
                r: s(2.5),
                color: pal.acc,
            });
        }
    }
}

/// The Disk card — title + mount-count meta, then one label / right readout /
/// full-width meter bar row per mount; an empty-state caption when no mounts.
/// Widget port; oracle is `disk_native`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn disk(
    meta: &str,
    rows: &[(String, String, i32, u32)],
    live: bool,
    show_title: bool,
    _w: f32,
    h: f32,
    grid: f32,
    pal: &Pal,
) -> Node {
    let s = |n: f32| n * grid;
    let fs = |n: f32| (n * grid * 10.0).round() / 10.0;
    let p = s(14.0);
    let mut root = Node::box_(0.0, 0);
    if show_title {
        root = root.child(
            Node::label("Disk", fs(11.5), pal.fg)
                .font(Ff::Ui, Fw::Semibold)
                .halign(HAlign::Left)
                .valign(VAlign::Top)
                .margin(p, s(9.0), 0.0, 0.0),
        );
    }
    if !meta.is_empty() {
        root = root.child(
            Node::label(meta, fs(8.5), crate::ui::fg3(pal))
                .font(Ff::Ui, Fw::Regular)
                .halign(HAlign::Right)
                .valign(VAlign::Top)
                .margin(0.0, s(11.0), p, 0.0),
        );
    }
    if live {
        let row_h = s(24.0);
        let mut i = 0;
        while i < rows.len() {
            let ry = s(32.0) + i as f32 * row_h;
            if ry + row_h > h - s(4.0) {
                break;
            }
            let (label, value, pct, color) = &rows[i];
            root = root.child(
                Node::label(label.as_str(), fs(9.0), crate::ui::fg2(pal))
                    .font(Ff::Ui, Fw::Regular)
                    .halign(HAlign::Left)
                    .valign(VAlign::Top)
                    .margin(p, ry, 0.0, 0.0),
            );
            root = root.child(
                Node::label(value.as_str(), fs(9.0), pal.fg)
                    .font(Ff::Mono, Fw::Regular)
                    .halign(HAlign::Right)
                    .valign(VAlign::Top)
                    .margin(0.0, ry, p, 0.0),
            );
            let bar_y = ry + s(13.0);
            root = root.child(
                Node::bar(
                    *pct as f32,
                    100.0,
                    *color,
                    crate::ui::hover(pal),
                    s(3.0),
                    false,
                )
                .margin(p, bar_y, p, h - bar_y - s(6.0)),
            );
            i += 1;
        }
    } else {
        root = root.child(
            Node::anchored(0.0, -s(6.0))
                .text_("No partitions")
                .size_(fs(9.0))
                .font(Ff::Ui, Fw::Regular)
                .color_(crate::ui::fg3(pal)),
        );
    }
    root
}

/// The retired Disk drawer — the byte-parity oracle for `disk`.
#[cfg(test)]
#[allow(clippy::too_many_arguments)]
fn disk_native(
    v: &mut Vec<Cmd>,
    meta: &str,
    rows: &[(String, String, i32, u32)],
    live: bool,
    show_title: bool,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    grid: f32,
    pal: &Pal,
) {
    let s = |n: f32| n * grid;
    let fs = |n: f32| (n * grid * 10.0).round() / 10.0;
    let p = s(14.0);
    if show_title {
        crate::ui::title(v, x + p, y + s(9.0), "Disk", fs(11.5), pal.fg);
    }
    if !meta.is_empty() {
        crate::ui::caption_r(v, x + w - p, y + s(11.0), meta, fs(8.5), crate::ui::fg3(pal), false);
    }
    if live {
        let row_h = s(24.0);
        let bar_w = w - p * 2.0;
        let mut i = 0;
        while i < rows.len() {
            let ry = y + s(32.0) + i as f32 * row_h;
            if ry + row_h > y + h - s(4.0) {
                break;
            }
            let (label, value, pct, color) = &rows[i];
            crate::ui::caption(v, x + p, ry, label.as_str(), fs(9.0), crate::ui::fg2(pal), false);
            crate::ui::text_r_mono(v, x + w - p, ry, value.as_str(), fs(9.0), pal.fg);
            let bar_y = ry + s(13.0);
            v.push(Cmd::Rect {
                x: x + p,
                y: bar_y,
                w: bar_w,
                h: s(6.0),
                r: s(3.0),
                color: crate::ui::hover(pal),
            });
            let fill_w = (bar_w as f64 * (*pct as f64 / 100.0)) as f32;
            if fill_w > 0.5 {
                v.push(Cmd::Rect {
                    x: x + p,
                    y: bar_y,
                    w: fill_w,
                    h: s(6.0),
                    r: s(3.0),
                    color: *color,
                });
            }
            i += 1;
        }
    } else {
        crate::ui::caption_c(
            v,
            x + w / 2.0,
            y + h / 2.0 - s(6.0),
            "No partitions",
            fs(9.0),
            crate::ui::fg3(pal),
            false,
        );
    }
}

/// One gauge column of the Memory/Disk card: bead ring + hero percent centred
/// inside it, then the label + capacity line while the card is tall enough.
/// One widget gauge column: bead ring + hero percent centred on the slot
/// centre + (`dx`, 0), then label + capacity line while tall enough.
#[allow(clippy::too_many_arguments)]
fn gauge_col(
    dx: f32,
    pct_txt: &str,
    label: &str,
    gb: &str,
    lit: i32,
    fill: u32,
    dim: u32,
    h: f32,
    grid: f32,
    pal: &Pal,
) -> Node {
    let s = |n: f32| n * grid;
    let fs = |n: f32| (n * grid * 10.0).round() / 10.0;
    let big = h >= s(116.0);
    let r = s(if big { 30.0 } else { 22.0 });
    let dot = s(if big { 5.0 } else { 4.0 });
    let mut node = Node::box_(0.0, 0);
    node = node.child(Node::beads(
        36,
        lit.max(0) as usize,
        dot,
        r,
        fill,
        dim,
        dx,
        0.0,
    ));
    node = node.child(
        Node::anchored(dx, -s(8.0))
            .text_(pct_txt)
            .size_(fs(if big { 13.0 } else { 11.0 }))
            .font(Ff::Display, Fw::Medium)
            .color_(pal.fg),
    );
    if h >= s(94.0) {
        node = node.child(
            Node::anchored(dx, s(if big { 45.0 } else { 37.0 }))
                .text_(label)
                .size_(fs(10.0))
                .font(Ff::Ui, Fw::Regular)
                .color_(pal.fg),
        );
        node = node.child(
            Node::anchored(dx, s(if big { 58.0 } else { 50.0 }))
                .text_(gb)
                .size_(fs(8.0))
                .font(Ff::Mono, Fw::Regular)
                .color_(crate::ui::fg2(pal)),
        );
    }
    node
}

/// The Memory/Disk gauge card — two columns while wide, else one centred
/// Memory column. Widget port; oracle is `gauges_native`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn gauges(
    mem_pct_txt: &str,
    mem_gb: &str,
    mem_lit: i32,
    mem_fill: u32,
    disk_pct_txt: &str,
    disk_gb: &str,
    disk_lit: i32,
    disk_fill: u32,
    dim: u32,
    w: f32,
    h: f32,
    grid: f32,
    pal: &Pal,
) -> Node {
    let s = |n: f32| n * grid;
    let mut root = Node::box_(0.0, 0);
    if w >= s(170.0) {
        root = root.child(gauge_col(
            w * 0.27 - w / 2.0,
            mem_pct_txt,
            "Memory",
            mem_gb,
            mem_lit,
            mem_fill,
            dim,
            h,
            grid,
            pal,
        ));
        root = root.child(gauge_col(
            w * 0.73 - w / 2.0,
            disk_pct_txt,
            "Disk",
            disk_gb,
            disk_lit,
            disk_fill,
            dim,
            h,
            grid,
            pal,
        ));
    } else {
        root = root.child(gauge_col(
            0.0,
            mem_pct_txt,
            "Memory",
            mem_gb,
            mem_lit,
            mem_fill,
            dim,
            h,
            grid,
            pal,
        ));
    }
    root
}

/// The retired Memory/Disk gauge columns — the parity oracle for `gauges`.
#[cfg(test)]
#[allow(clippy::too_many_arguments)]
fn gauge_col_native(
    v: &mut Vec<Cmd>,
    cx: f32,
    pct_txt: &str,
    label: &str,
    gb: &str,
    lit: i32,
    fill: u32,
    dim: u32,
    y: f32,
    h: f32,
    grid: f32,
    pal: &Pal,
) {
    let s = |n: f32| n * grid;
    let fs = |n: f32| (n * grid * 10.0).round() / 10.0;
    let big = h >= s(116.0);
    let r = s(if big { 30.0 } else { 22.0 });
    let dot = s(if big { 5.0 } else { 4.0 });
    let cy = y + h / 2.0;
    crate::scene::bead_ring(v, cx, cy, r, dot, 36, lit.max(0) as usize, fill, dim);
    crate::ui::text_cw(
        v,
        cx,
        cy - s(8.0),
        pct_txt,
        fs(if big { 13.0 } else { 11.0 }),
        Ff::Display,
        Fw::Medium,
        pal.fg,
        false,
    );
    if h >= s(94.0) {
        crate::ui::caption_c(
            v,
            cx,
            cy + s(if big { 45.0 } else { 37.0 }),
            label,
            fs(10.0),
            pal.fg,
            false,
        );
        crate::ui::text_c_mono(
            v,
            cx,
            cy + s(if big { 58.0 } else { 50.0 }),
            gb,
            fs(8.0),
            crate::ui::fg2(pal),
        );
    }
}

/// The Memory/Disk gauge card — two columns while the card is wide, else one
/// centred Memory column.
#[cfg(test)]
#[allow(clippy::too_many_arguments)]
fn gauges_native(
    v: &mut Vec<Cmd>,
    mem_pct_txt: &str,
    mem_gb: &str,
    mem_lit: i32,
    mem_fill: u32,
    disk_pct_txt: &str,
    disk_gb: &str,
    disk_lit: i32,
    disk_fill: u32,
    dim: u32,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    grid: f32,
    pal: &Pal,
) {
    let s = |n: f32| n * grid;
    if w >= s(170.0) {
        gauge_col_native(
            v,
            x + w * 0.27,
            mem_pct_txt,
            "Memory",
            mem_gb,
            mem_lit,
            mem_fill,
            dim,
            y,
            h,
            grid,
            pal,
        );
        gauge_col_native(
            v,
            x + w * 0.73,
            disk_pct_txt,
            "Disk",
            disk_gb,
            disk_lit,
            disk_fill,
            dim,
            y,
            h,
            grid,
            pal,
        );
    } else {
        gauge_col_native(
            v,
            x + w / 2.0,
            mem_pct_txt,
            "Memory",
            mem_gb,
            mem_lit,
            mem_fill,
            dim,
            y,
            h,
            grid,
            pal,
        );
    }
}

/// The GPU card — util hero + bar, thermal + VRAM rows, the geometry of the
/// retired `ui/cards/gpu.ron` scene and its master module (api.card_show_glyph,
/// api.card_show_title, api.empty, api.pct, api.frac, api.temp, api.vram_*).
/// `Bar` boxes here are the host primitive's exact geometry.
/// The GPU card as a widget tree — glyph/title header, util hero + bar, then
/// the thermal + VRAM rows. Widget port; oracle is `gpu_native`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn gpu(
    show_glyph: bool,
    show_title: bool,
    empty: bool,
    pct: i32,
    frac: f32,
    temp: &str,
    vram: Option<(String, f32)>,
    _w: f32,
    h: f32,
    grid: f32,
    pal: &Pal,
) -> Node {
    let s = |n: f32| n * grid;
    let fs = |n: f32| (n * grid * 10.0).round() / 10.0;
    let pad = s(12.0);
    let hdr = s(24.0);
    let fg2 = crate::ui::fg2(pal);
    let fg3 = crate::ui::fg3(pal);
    let mut root = Node::box_(0.0, 0);

    let mut tx = pad;
    if show_glyph {
        root = root.child(
            Node::glyph("\u{f013}", fs(12.0), fg2)
                .halign(HAlign::Left)
                .valign(VAlign::Top)
                .margin(pad, s(8.0), 0.0, 0.0),
        );
        tx = pad + s(17.0);
    }
    if show_title {
        root = root.child(
            Node::label("GPU", fs(11.5), pal.fg)
                .font(Ff::Ui, Fw::Semibold)
                .halign(HAlign::Left)
                .valign(VAlign::Top)
                .margin(tx, s(9.0), 0.0, 0.0),
        );
    }

    if empty {
        root = root.child(
            Node::anchored(0.0, -s(6.0))
                .text_("no GPU source")
                .size_(fs(9.0))
                .font(Ff::Ui, Fw::Regular)
                .color_(fg3),
        );
        return root;
    }

    root = root.child(
        Node::label(format!("{pct}%"), fs(22.0), pal.acc)
            .font(Ff::Display, Fw::Medium)
            .halign(HAlign::Right)
            .valign(VAlign::Top)
            .margin(0.0, hdr, pad, 0.0),
    );
    root = root.child(
        Node::label("utilization", fs(8.5), fg2)
            .font(Ff::Ui, Fw::Regular)
            .halign(HAlign::Left)
            .valign(VAlign::Top)
            .margin(pad, hdr + s(4.0), 0.0, 0.0),
    );
    root = root.child(
        Node::bar(frac, 1.0, pal.acc, crate::ui::hover(pal), s(2.5), false).margin(
            pad,
            hdr + s(26.0),
            pad,
            h - (hdr + s(26.0)) - s(5.0),
        ),
    );

    let ry = hdr + s(38.0);
    root = root.child(
        Node::glyph("\u{f2c9}", fs(9.0), fg2)
            .halign(HAlign::Left)
            .valign(VAlign::Top)
            .margin(pad, ry, 0.0, 0.0),
    );
    root = root.child(
        Node::label(temp, fs(9.5), pal.fg)
            .font(Ff::Mono, Fw::Regular)
            .halign(HAlign::Left)
            .valign(VAlign::Top)
            .margin(pad + s(14.0), ry + s(1.0), 0.0, 0.0),
    );
    if let Some((vram_txt, vram_frac)) = vram {
        root = root.child(
            Node::label(vram_txt, fs(9.5), pal.fg)
                .font(Ff::Mono, Fw::Regular)
                .halign(HAlign::Left)
                .valign(VAlign::Top)
                .margin(pad, ry + s(15.0), 0.0, 0.0),
        );
        root = root.child(
            Node::bar(vram_frac, 1.0, crate::ui::INFO, crate::ui::hover(pal), s(2.0), false)
                .margin(pad, ry + s(27.0), pad, h - (ry + s(27.0)) - s(4.0)),
        );
    }
    root
}

/// The retired GPU drawer — the parity oracle for `gpu`.
#[cfg(test)]
#[allow(clippy::too_many_arguments)]
fn gpu_native(
    v: &mut Vec<Cmd>,
    show_glyph: bool,
    show_title: bool,
    empty: bool,
    pct: i32,
    frac: f32,
    temp: &str,
    vram: Option<(String, f32)>,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    grid: f32,
    pal: &Pal,
) {
    let s = |n: f32| n * grid;
    let fs = |n: f32| (n * grid * 10.0).round() / 10.0;
    let pad = s(12.0);
    let hdr = s(24.0);
    let fg2 = crate::ui::fg2(pal);
    let fg3 = crate::ui::fg3(pal);

    let mut tx = pad;
    if show_glyph {
        crate::ui::text(v, x + pad, y + s(8.0), "\u{f013}", fs(12.0), fg2, true);
        tx = pad + s(17.0);
    }
    if show_title {
        crate::ui::title(v, x + tx, y + s(9.0), "GPU", fs(11.5), pal.fg);
    }

    if empty {
        crate::ui::caption_c(v, x + w / 2.0, y + h / 2.0 - s(6.0), "no GPU source", fs(9.0), fg3, false);
        return;
    }

    // util big number + bar
    crate::ui::text_r_hero(v, x + w - pad, y + hdr, format!("{pct}%"), fs(22.0), pal.acc);
    crate::ui::caption(v, x + pad, y + hdr + s(4.0), "utilization", fs(8.5), fg2, false);
    let bw = w - pad * 2.0;
    bar_prim(v, x + pad, y + hdr + s(26.0), bw, s(5.0), s(2.5), frac, 1.0, pal.acc, crate::ui::hover(pal), false);

    // temp + vram rows
    let ry = y + hdr + s(38.0);
    crate::ui::text(v, x + pad, ry, "\u{f2c9}", fs(9.0), fg2, true);
    crate::ui::text_mono(v, x + pad + s(14.0), ry + s(1.0), temp, fs(9.5), pal.fg);
    if let Some((vram_txt, vram_frac)) = vram {
        crate::ui::text_mono(v, x + pad, ry + s(15.0), vram_txt, fs(9.5), pal.fg);
        bar_prim(v, x + pad, ry + s(27.0), bw, s(4.0), s(2.0), vram_frac, 1.0, crate::ui::INFO, crate::ui::hover(pal), false);
    }
}

/// The Branding card — contain-fit brand mark centred on the card's mid-line
/// (api.glyph / api.size / api.y arrive already resolved; `size` is BASE px).
/// The Branding card — contain-fit brand mark centred on the card's mid-line.
/// Widget port; oracle is `branding_native`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn branding(
    show_glyph: bool,
    show_title: bool,
    glyph: &str,
    size: f32,
    mark_y: f32,
    h: f32,
    grid: f32,
    pal: &Pal,
) -> Node {
    let s = |n: f32| n * grid;
    let fs = |n: f32| (n * grid * 10.0).round() / 10.0;
    let p = s(16.0);
    let mut root = Node::box_(0.0, 0);
    let mut tx = p;
    if show_glyph {
        root = root.child(
            Node::glyph("\u{f004}", fs(12.0), crate::ui::fg2(pal))
                .halign(HAlign::Left)
                .valign(VAlign::Top)
                .margin(p, s(8.0), 0.0, 0.0),
        );
        tx = p + s(17.0);
    }
    if show_title {
        root = root.child(
            Node::label("Brand", fs(11.5), pal.fg)
                .font(Ff::Ui, Fw::Semibold)
                .halign(HAlign::Left)
                .valign(VAlign::Top)
                .margin(tx, s(9.0), 0.0, 0.0),
        );
    }
    if glyph.is_empty() {
        root = root.child(
            Node::anchored(0.0, -s(6.0))
                .text_("no brand glyph in $states2/d")
                .size_(fs(9.0))
                .color_(crate::ui::fg3(pal)),
        );
        return root;
    }
    root = root.child(
        Node::anchored(0.0, s(mark_y) - h / 2.0)
            .text_(glyph)
            .size_(fs(size))
            .font(crate::text::Ff::Brand, crate::text::Fw::Light)
            .color_(pal.fg),
    );
    root
}

/// The retired Branding drawer — the parity oracle for `branding`.
#[cfg(test)]
#[allow(clippy::too_many_arguments)]
fn branding_native(
    v: &mut Vec<Cmd>,
    show_glyph: bool,
    show_title: bool,
    glyph: &str,
    size: f32,
    mark_y: f32,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    grid: f32,
    pal: &Pal,
) {
    let s = |n: f32| n * grid;
    let fs = |n: f32| (n * grid * 10.0).round() / 10.0;
    let p = s(16.0);
    let mut tx = p;
    if show_glyph {
        crate::ui::text(v, x + p, y + s(8.0), "\u{f004}", fs(12.0), crate::ui::fg2(pal), true);
        tx = p + s(17.0);
    }
    if show_title {
        crate::ui::title(v, x + tx, y + s(9.0), "Brand", fs(11.5), pal.fg);
    }
    if glyph.is_empty() {
        crate::ui::text_c(
            v,
            x + w / 2.0,
            y + h / 2.0 - s(6.0),
            "no brand glyph in $states2/d",
            fs(9.0),
            crate::ui::fg3(pal),
            false,
        );
        return;
    }
    crate::ui::text_cw(
        v,
        x + w / 2.0,
        y + s(mark_y),
        glyph,
        fs(size),
        crate::text::Ff::Brand,
        crate::text::Fw::Light,
        pal.fg,
        false,
    );
}

/// The Moon card — disc + illumination hero maths (the SYNODIC phase ladder
/// the module held) and the phase-name column gated on card room.
/// The Moon card — disc (a translated `ui::moon_disc` stamp) + illumination
/// hero maths and the phase-name column. Widget port; oracle is `moon_native`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn moon(
    show_glyph: bool,
    show_title: bool,
    glyph: &str,
    epoch: f64,
    w: f32,
    h: f32,
    grid: f32,
    pal: &Pal,
) -> Node {
    const SYNODIC: f64 = 29.53059;
    const NEW_MOON_EPOCH: f64 = 947_182_440.0;
    const NAMES: [&str; 8] = [
        "New moon",
        "Waxing crescent",
        "First quarter",
        "Waxing gibbous",
        "Full moon",
        "Waning gibbous",
        "Last quarter",
        "Waning crescent",
    ];
    let phase = ((epoch - NEW_MOON_EPOCH) / 86_400.0).rem_euclid(SYNODIC) / SYNODIC;
    let illum = (1.0 - (2.0 * std::f64::consts::PI * phase).cos()) / 2.0;
    let meta = format!("{:.0}%", illum * 100.0);
    let name_idx = (((phase * 8.0 + 0.5).floor() as i64).rem_euclid(8) + 1) as usize - 1;
    let name = NAMES[name_idx];
    let lit = format!("{:.0}% lit", illum * 100.0);
    let dtf = (0.5 - phase).rem_euclid(1.0) * SYNODIC;
    let sub = if dtf < 0.8 {
        "full tonight".to_string()
    } else {
        format!("{dtf:.0} d to full")
    };

    let s = |n: f32| n * grid;
    let fs = |n: f32| (n * grid * 10.0).round() / 10.0;
    let p = s(12.0);
    let mut root = Node::box_(0.0, 0);

    if show_glyph {
        root = root.child(
            Node::glyph(glyph, fs(12.0), crate::ui::fg2(pal))
                .halign(HAlign::Left)
                .valign(VAlign::Top)
                .margin(p, s(8.0), 0.0, 0.0),
        );
    }
    if show_title {
        root = root.child(
            Node::label("Moon", fs(11.5), pal.fg)
                .font(Ff::Ui, Fw::Semibold)
                .halign(HAlign::Left)
                .valign(VAlign::Top)
                .margin(p + s(17.0), s(9.0), 0.0, 0.0),
        );
    }
    if !meta.is_empty() {
        root = root.child(
            Node::label(meta.as_str(), fs(8.5), crate::ui::fg3(pal))
                .font(Ff::Mono, Fw::Regular)
                .halign(HAlign::Right)
                .valign(VAlign::Top)
                .margin(0.0, s(11.0), p, 0.0),
        );
    }

    let body_top = s(24.0) + s(4.0);
    let body_h = h - body_top - s(34.0);
    let d = (body_h.min(w - p * 2.0 - s(90.0))).max(s(30.0));
    let cx = p + d / 2.0 + s(4.0);
    let cy = body_top + body_h / 2.0;

    let mut disc: Vec<Cmd> = Vec::new();
    crate::ui::moon_disc(&mut disc, 0.0, 0.0, d, phase, pal);
    root = root.child(Node::stamp(disc).margin(cx, cy, w - cx - d, h - cy - d));

    let tx = cx + d / 2.0 + s(12.0);
    if w - p - tx > s(60.0) {
        root = root.child(
            Node::label(name, fs(10.5), pal.fg)
                .halign(HAlign::Left)
                .valign(VAlign::Top)
                .margin(tx, cy - s(12.0), 0.0, 0.0),
        );
        root = root.child(
            Node::label(lit.as_str(), fs(8.5), crate::ui::fg2(pal))
                .font(Ff::Ui, Fw::Regular)
                .halign(HAlign::Left)
                .valign(VAlign::Top)
                .margin(tx, cy + s(3.0), 0.0, 0.0),
        );
        root = root.child(
            Node::label(sub.as_str(), fs(8.5), crate::ui::fg3(pal))
                .font(Ff::Ui, Fw::Regular)
                .halign(HAlign::Left)
                .valign(VAlign::Top)
                .margin(tx, cy + s(14.0), 0.0, 0.0),
        );
    }
    root
}

/// The retired Moon drawer — the parity oracle for `moon`.
#[cfg(test)]
#[allow(clippy::too_many_arguments)]
fn moon_native(
    v: &mut Vec<Cmd>,
    show_glyph: bool,
    show_title: bool,
    glyph: &str,
    epoch: f64,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    grid: f32,
    pal: &Pal,
) {
    const SYNODIC: f64 = 29.53059;
    const NEW_MOON_EPOCH: f64 = 947_182_440.0;
    const NAMES: [&str; 8] = [
        "New moon",
        "Waxing crescent",
        "First quarter",
        "Waxing gibbous",
        "Full moon",
        "Waning gibbous",
        "Last quarter",
        "Waning crescent",
    ];
    let phase = ((epoch - NEW_MOON_EPOCH) / 86_400.0).rem_euclid(SYNODIC) / SYNODIC;
    let illum = (1.0 - (2.0 * std::f64::consts::PI * phase).cos()) / 2.0;
    let meta = format!("{:.0}%", illum * 100.0);
    let name_idx = (((phase * 8.0 + 0.5).floor() as i64).rem_euclid(8) + 1) as usize - 1;
    let name = NAMES[name_idx];
    let lit = format!("{:.0}% lit", illum * 100.0);
    let dtf = (0.5 - phase).rem_euclid(1.0) * SYNODIC;
    let sub = if dtf < 0.8 {
        "full tonight".to_string()
    } else {
        format!("{dtf:.0} d to full")
    };

    let s = |n: f32| n * grid;
    let fs = |n: f32| (n * grid * 10.0).round() / 10.0;
    let p = s(12.0);

    if show_glyph {
        crate::ui::text(v, x + p, y + s(8.0), glyph, fs(12.0), crate::ui::fg2(pal), true);
    }
    if show_title {
        crate::ui::title(v, x + p + s(17.0), y + s(9.0), "Moon", fs(11.5), pal.fg);
    }
    if !meta.is_empty() {
        crate::ui::text_r_mono(v, x + w - p, y + s(11.0), &meta, fs(8.5), crate::ui::fg3(pal));
    }

    // the disc's box is the card's own body arithmetic (min floor of 30)
    let body_top = s(24.0) + s(4.0);
    let body_h = h - body_top - s(34.0);
    let d = (body_h.min(w - p * 2.0 - s(90.0))).max(s(30.0));
    let cx = x + p + d / 2.0 + s(4.0);
    let cy = y + body_top + body_h / 2.0;

    // the primitive draws at the ORIGIN and the host translates the rects —
    // keep the same translate step so the f32 sums land bit-identically
    let mut disc: Vec<Cmd> = Vec::new();
    crate::ui::moon_disc(&mut disc, 0.0, 0.0, d, phase, pal);
    for cmd in disc {
        match cmd {
            Cmd::Rect { x: rx, y: ry, w, h, r, color } => {
                v.push(Cmd::Rect { x: cx + rx, y: cy + ry, w, h, r, color });
            }
            other => v.push(other),
        }
    }

    // the side column rides the disc's center, gated on card room
    let tx = cx + d / 2.0 + s(12.0);
    if x + w - p - tx > s(60.0) {
        crate::ui::text(v, tx, cy - s(12.0), name, fs(10.5), pal.fg, false);
        crate::ui::caption(v, tx, cy + s(3.0), &lit, fs(8.5), crate::ui::fg2(pal), false);
        crate::ui::caption(v, tx, cy + s(14.0), &sub, fs(8.5), crate::ui::fg3(pal), false);
    }
}

/// The Visualizer card — header + style-cycle hit + one `Spectrum` per style
/// (only the active style paints), the silent caption on a tall enough card.
/// Returns the style-button hit region for the shell's input pipeline.
/// The Visualizer card — header + style-cycle hit + one `Spectrum` per style,
/// the silent caption on a tall enough card. Widget port; oracle `viz_native`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn viz(
    show_glyph: bool,
    show_title: bool,
    series: &[f32],
    style: u8,
    silent: bool,
    hover: u32,
    key: u32,
    w: f32,
    h: f32,
    grid: f32,
    pal: &Pal,
) -> Node {
    let s = |n: f32| n * grid;
    let fs = |n: f32| (n * grid * 10.0).round() / 10.0;
    let mut root = Node::box_(0.0, 0);
    if show_glyph {
        root = root.child(
            Node::glyph("\u{f001}", fs(12.0), pal.acc)
                .halign(HAlign::Left)
                .valign(VAlign::Top)
                .margin(s(12.0), s(8.0), 0.0, 0.0),
        );
    }
    if show_title {
        let tx = if show_glyph { s(30.0) } else { s(12.0) };
        root = root.child(
            Node::label("Visualizer", fs(12.0), pal.fg)
                .font(Ff::Ui, Fw::Semibold)
                .halign(HAlign::Left)
                .valign(VAlign::Top)
                .margin(tx, s(9.0), 0.0, 0.0),
        );
    }

    let hov = hover == key;
    root = root.child(
        Node::glyph("\u{f009}", fs(12.0), if hov { pal.acc } else { pal.fg })
            .halign(HAlign::Left)
            .valign(VAlign::Top)
            .margin(w - s(18.0), s(9.0), 0.0, 0.0),
    );
    root = root.child(Node::hit(key).margin(
        w - s(20.0),
        s(4.0),
        s(4.0),
        h - s(4.0) - s(20.0),
    ));

    let area_x = s(16.0);
    let mut area_w = w - s(32.0);
    if area_w < 20.0 {
        area_w = 20.0;
    }
    let area_top = s(42.0);
    let area_bot = h - s(12.0);
    let mut area_h = area_bot - area_top;
    if area_h < 12.0 {
        area_h = 12.0;
    }

    let kind = match style {
        1 => crate::scene::SpectrumKind::Wave,
        2 => crate::scene::SpectrumKind::Block,
        _ => crate::scene::SpectrumKind::Rounded,
    };
    let (bar_frac, inset, min_bar_w, block_r, accent_every, tint): (f32, f32, f32, f32, u32, f32) =
        match style {
            1 => (0.0, s(0.0), s(0.0), s(1.0), 0, 0.0),
            2 => (0.0, s(1.0), s(1.0), s(1.0), 4, 0.4),
            _ => (0.5, s(0.0), s(2.0), s(1.0), 4, 0.35),
        };
    let mut spec: Vec<Cmd> = Vec::new();
    crate::scene::push_spectrum(
        &mut spec,
        0.0,
        0.0,
        area_w,
        area_h,
        series,
        kind,
        0.5,
        bar_frac,
        inset,
        min_bar_w,
        s(2.0),
        block_r,
        pal.acc,
        accent_every,
        tint,
        pal.fg,
    );
    root = root.child(
        Node::stamp(spec).margin(
            area_x,
            area_top,
            w - area_x - area_w,
            h - area_top - area_h,
        ),
    );

    if silent && h >= s(90.0) {
        root = root.child(
            Node::anchored(0.0, h / 2.0 - s(24.0))
                .text_("no audio")
                .size_(fs(8.0))
                .color_(crate::ui::fg3(pal)),
        );
    }
    root
}

/// The retired Visualizer drawer — the parity oracle for `viz`.
#[cfg(test)]
#[allow(clippy::too_many_arguments)]
fn viz_native(
    v: &mut Vec<Cmd>,
    show_glyph: bool,
    show_title: bool,
    series: &[f32],
    style: u8,
    silent: bool,
    hover: u32,
    key: u32,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    grid: f32,
    pal: &Pal,
) -> Vec<CardHit> {
    let s = |n: f32| n * grid;
    let fs = |n: f32| (n * grid * 10.0).round() / 10.0;

    if show_glyph {
        crate::ui::text(v, x + s(12.0), y + s(8.0), "\u{f001}", fs(12.0), pal.acc, true);
    }
    if show_title {
        let tx = if show_glyph { s(30.0) } else { s(12.0) };
        crate::ui::title(v, x + tx, y + s(9.0), "Visualizer", fs(12.0), pal.fg);
    }

    let hov = hover == key;
    crate::ui::text(
        v,
        x + w - s(18.0),
        y + s(9.0),
        "\u{f009}",
        fs(12.0),
        if hov { pal.acc } else { pal.fg },
        true,
    );
    let hits = vec![(x + w - s(20.0), y + s(4.0), s(16.0), s(20.0), key)];

    let area_x = x + s(16.0);
    let mut area_w = w - s(32.0);
    if area_w < 20.0 {
        area_w = 20.0;
    }
    let area_top = y + s(42.0);
    let area_bot = y + h - s(12.0);
    let mut area_h = area_bot - area_top;
    if area_h < 12.0 {
        area_h = 12.0;
    }

    let kind = match style {
        1 => crate::scene::SpectrumKind::Wave,
        2 => crate::scene::SpectrumKind::Block,
        _ => crate::scene::SpectrumKind::Rounded,
    };
    let (bar_frac, inset, min_bar_w, block_r, accent_every, tint): (f32, f32, f32, f32, u32, f32) =
        match style {
            1 => (0.0, s(0.0), s(0.0), s(1.0), 0, 0.0),
            2 => (0.0, s(1.0), s(1.0), s(1.0), 4, 0.4),
            _ => (0.5, s(0.0), s(2.0), s(1.0), 4, 0.35),
        };
    crate::scene::push_spectrum(
        v,
        area_x,
        area_top,
        area_w,
        area_h,
        series,
        kind,
        0.5,
        bar_frac,
        inset,
        min_bar_w,
        s(2.0),
        block_r,
        pal.acc,
        accent_every,
        tint,
        pal.fg,
    );

    if silent && h >= s(90.0) {
        crate::ui::text_c(v, x + w / 2.0, y + h - s(24.0), "no audio", fs(8.0), crate::ui::fg3(pal), false);
    }
    hits
}

/// One hold-to-confirm Power button, shared by the horizontal and vertical
/// layouts (the retired Lua modules' per-button Stack).
/// One widget hold-to-confirm Power button (bg box + hairline outline stamp +
/// optional hold bar + centred glyph + hit region). Oracle `power_button_native`.
#[allow(clippy::too_many_arguments)]
fn power_button(
    d: f32,
    grid: f32,
    pal: &Pal,
    key: u32,
    glyph: &str,
    bg: u32,
    ink: u32,
    hold: f32,
    hold_on: bool,
    hold_fill: u32,
) -> Node {
    let s = |n: f32| n * grid;
    let mut n = Node::box_(s(10.0), bg);
    n = n.child(Node::stamp(vec![Cmd::Outline {
        x: 0.0,
        y: 0.0,
        w: d,
        h: d,
        r: s(10.0),
        width: s(1.0),
        color: crate::ui::hairline(pal),
    }]));
    if hold_on {
        n = n.child(
            Node::bar(hold, 1.0, hold_fill, 0, s(8.0), true).margin(
                s(2.0),
                -s(2.0),
                d - s(2.0) - s(28.0),
                s(2.0),
            ),
        );
    }
    n = n.child(
        Node::anchored(0.0, 0.0)
            .text_(glyph)
            .size_((14.0 * grid * 10.0).round() / 10.0)
            .color_(ink)
            .icon(),
    );
    n.child(Node::hit(key))
}

/// The horizontal Power card — five hold buttons on a 41 px pitch row.
/// Widget port; oracle `powerh_native`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn powerh(
    show_glyph: bool,
    show_title: bool,
    hold_fill: u32,
    bg: &[u32; 5],
    ink: &[u32; 5],
    hold: &[f32; 5],
    hold_on: &[bool; 5],
    w: f32,
    h: f32,
    grid: f32,
    pal: &Pal,
) -> Node {
    power_layout(
        show_glyph, show_title, hold_fill, bg, ink, hold, hold_on, true, w, h, grid, pal,
    )
}

/// The vertical Power card — five hold buttons centred as a column.
/// Widget port; oracle `powerv_native`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn powerv(
    show_glyph: bool,
    show_title: bool,
    hold_fill: u32,
    bg: &[u32; 5],
    ink: &[u32; 5],
    hold: &[f32; 5],
    hold_on: &[bool; 5],
    w: f32,
    h: f32,
    grid: f32,
    pal: &Pal,
) -> Node {
    power_layout(
        show_glyph, show_title, hold_fill, bg, ink, hold, hold_on, false, w, h, grid, pal,
    )
}

/// The five POWER_CARD_KEY_BASE-button widget layout shared by `powerh`/`powerv`.
#[allow(clippy::too_many_arguments)]
fn power_layout(
    show_glyph: bool,
    show_title: bool,
    hold_fill: u32,
    bg: &[u32; 5],
    ink: &[u32; 5],
    hold: &[f32; 5],
    hold_on: &[bool; 5],
    horizontal: bool,
    w: f32,
    h: f32,
    grid: f32,
    pal: &Pal,
) -> Node {
    use crate::shell::POWER_CARD_KEY_BASE;
    let s = |n: f32| n * grid;
    let fs = |n: f32| (n * grid * 10.0).round() / 10.0;
    let p = s(12.0);
    let mut root = Node::box_(0.0, 0);
    let mut tx = p;
    if show_glyph {
        root = root.child(
            Node::glyph("\u{f0e7}", fs(12.0), crate::ui::fg2(pal))
                .halign(HAlign::Left)
                .valign(VAlign::Top)
                .margin(p, s(8.0), 0.0, 0.0),
        );
        tx = p + s(17.0);
    }
    if show_title {
        root = root.child(
            Node::label("Power", fs(11.5), pal.fg)
                .font(Ff::Ui, Fw::Semibold)
                .halign(HAlign::Left)
                .valign(VAlign::Top)
                .margin(tx, s(9.0), 0.0, 0.0),
        );
    }

    const GLYPHS: [&str; 5] = [
        "\u{f023}", "\u{f08b}", "\u{f186}", "\u{f021}", "\u{f011}",
    ];
    let d = s(32.0);
    let gap = s(9.0);
    let block = 5.0 * d + 4.0 * gap;
    let body_h = h - s(30.0);
    for i in 0..5 {
        let (bx, by) = if horizontal {
            let bx0 = (w - block) / 2.0;
            let by = s(30.0) + (body_h - d).max(0.0) / 2.0;
            (bx0 + i as f32 * (d + gap), by)
        } else {
            let bx = (w - d) / 2.0;
            let by0 = s(30.0) + (body_h - block).max(0.0) / 2.0;
            (bx, by0 + i as f32 * (d + gap))
        };
        let btn = power_button(
            d,
            grid,
            pal,
            POWER_CARD_KEY_BASE + i as u32,
            GLYPHS[i],
            bg[i],
            ink[i],
            hold[i],
            hold_on[i],
            hold_fill,
        );
        let _ = fs;
        root = root.child(btn.margin(bx, by, w - bx - d, h - by - d));
    }
    root
}

/// The retired Power drawers — the parity oracles for `powerh`/`powerv`.
#[cfg(test)]
#[allow(clippy::too_many_arguments)]
fn power_button_native(
    v: &mut Vec<Cmd>,
    bx: f32,
    by: f32,
    d: f32,
    grid: f32,
    pal: &Pal,
    key: u32,
    glyph: &str,
    bg: u32,
    ink: u32,
    hold: f32,
    hold_on: bool,
    hold_fill: u32,
) -> CardHit {
    let s = |n: f32| n * grid;
    v.push(Cmd::Rect {
        x: bx,
        y: by,
        w: d,
        h: d,
        r: s(10.0),
        color: bg,
    });
    v.push(crate::cmd::Cmd::Outline {
        x: bx,
        y: by,
        w: d,
        h: d,
        r: s(10.0),
        width: s(1.0),
        color: crate::ui::hairline(pal),
    });
    if hold_on {
        bar_prim(
            v,
            bx + s(2.0),
            by - s(2.0),
            s(28.0),
            s(32.0),
            s(8.0),
            hold,
            1.0,
            hold_fill,
            0,
            true,
        );
    }
    crate::ui::text_c(v, bx + d / 2.0, by + d / 2.0, glyph, (14.0 * grid * 10.0).round() / 10.0, ink, true);
    (bx, by, d, d, key)
}

/// The horizontal Power card — five 32 px hold buttons on a 41 px pitch,
/// centred as a row below the header. Returns the five button regions.
#[cfg(test)]
#[allow(clippy::too_many_arguments)]
fn powerh_native(
    v: &mut Vec<Cmd>,
    show_glyph: bool,
    show_title: bool,
    hold_fill: u32,
    bg: &[u32; 5],
    ink: &[u32; 5],
    hold: &[f32; 5],
    hold_on: &[bool; 5],
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    grid: f32,
    pal: &Pal,
) -> Vec<CardHit> {
    power_layout_native(
        v, show_glyph, show_title, hold_fill, bg, ink, hold, hold_on,
        true, x, y, w, h, grid, pal,
    )
}

/// The vertical Power card — five hold buttons centred as a column.
#[cfg(test)]
#[allow(clippy::too_many_arguments)]
fn powerv_native(
    v: &mut Vec<Cmd>,
    show_glyph: bool,
    show_title: bool,
    hold_fill: u32,
    bg: &[u32; 5],
    ink: &[u32; 5],
    hold: &[f32; 5],
    hold_on: &[bool; 5],
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    grid: f32,
    pal: &Pal,
) -> Vec<CardHit> {
    power_layout_native(
        v, show_glyph, show_title, hold_fill, bg, ink, hold, hold_on,
        false, x, y, w, h, grid, pal,
    )
}

/// The five POWER_CARD_KEY_BASE-button layout shared by `powerh` / `powerv`;
/// `horizontal` arranges the block across (row) or down (column).
#[cfg(test)]
#[allow(clippy::too_many_arguments)]
fn power_layout_native(
    v: &mut Vec<Cmd>,
    show_glyph: bool,
    show_title: bool,
    hold_fill: u32,
    bg: &[u32; 5],
    ink: &[u32; 5],
    hold: &[f32; 5],
    hold_on: &[bool; 5],
    horizontal: bool,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    grid: f32,
    pal: &Pal,
) -> Vec<CardHit> {
    use crate::shell::POWER_CARD_KEY_BASE;
    let s = |n: f32| n * grid;
    let fs = |n: f32| (n * grid * 10.0).round() / 10.0;
    let p = s(12.0);
    let mut tx = p;
    if show_glyph {
        crate::ui::text(v, x + p, y + s(8.0), "\u{f0e7}", fs(12.0), crate::ui::fg2(pal), true);
        tx = p + s(17.0);
    }
    if show_title {
        crate::ui::title(v, x + tx, y + s(9.0), "Power", fs(11.5), pal.fg);
    }

    const GLYPHS: [&str; 5] = [
        "\u{f023}", // lock
        "\u{f08b}", // logout
        "\u{f186}", // sleep
        "\u{f021}", // restart
        "\u{f011}", // shutdown
    ];
    let d = s(32.0);
    let gap = s(9.0);
    let block = 5.0 * d + 4.0 * gap;
    let body_h = h - s(30.0);
    let mut hits = Vec::with_capacity(5);
    for i in 0..5 {
        let (bx, by) = if horizontal {
            let total = block;
            let bx0 = x + (w - total) / 2.0;
            let by = y + s(30.0) + (body_h - d).max(0.0) / 2.0;
            (bx0 + i as f32 * (d + gap), by)
        } else {
            let bx = x + (w - d) / 2.0;
            let by0 = y + s(30.0) + (body_h - block).max(0.0) / 2.0;
            (bx, by0 + i as f32 * (d + gap))
        };
        hits.push(power_button_native(
            v,
            bx,
            by,
            d,
            grid,
            pal,
            POWER_CARD_KEY_BASE + i as u32,
            GLYPHS[i],
            bg[i],
            ink[i],
            hold[i],
            hold_on[i],
            hold_fill,
        ));
    }
    hits
}

/// The `Spark` line trace, exactly as the Lua host's `ctx.ui.spark` binding
/// ran it (inset none, min w/h 0): samples joined by segments across `sw`,
/// pad_v inset, peak-normalized unless an explicit positive `vmax` overrides
/// (the drawer's `if max > 0 { max } else { peak }` rule).
#[allow(clippy::too_many_arguments)]
fn spark(
    v: &mut Vec<Cmd>,
    sx: f32,
    sy: f32,
    sw: f32,
    sh: f32,
    data: &[f32],
    color: u32,
    thickness: f32,
    vmax: Option<f32>,
) {
    if data.len() < 2 {
        return;
    }
    let vals: Vec<f32> = data.to_vec();
    let t = thickness;
    let pad_v = t / 2.0 + 1.0;
    let usable = (sh - pad_v * 2.0).max(2.0);
    let peak = match vmax {
        Some(m) if m > 0.0 => m,
        _ => vals.iter().cloned().fold(0.0f32, f32::max).max(1e-6),
    };
    let step = sw / (vals.len() - 1) as f32;
    let mut prev: Option<(f32, f32)> = None;
    for (i, &d) in vals.iter().enumerate() {
        let px = sx + i as f32 * step;
        let py = sy + sh - pad_v - (d / peak).clamp(0.0, 1.0) * usable;
        if let Some((qx, qy)) = prev {
            v.push(Cmd::Line {
                x0: qx,
                y0: qy,
                x1: px,
                y1: py,
                w: t,
                color,
            });
        }
        prev = Some((px, py));
    }
}

/// Shared winget-string culling: the card's `0.62·fs` per-glyph budget and
/// the eight-char floor, exactly as `ui.len` / `ui.sub` capped in the modules.
fn trunc_at(t: &str, avail: f32, gp: f32) -> String {
    let gp = gp.max(1.0);
    let mut n = (avail / gp).floor() as usize;
    if n < 8 {
        n = 8;
    }
    if t.chars().count() > n {
        t.chars().take(n).collect()
    } else {
        t.to_string()
    }
}

/// The Prices card — header + always-on status line + the scroll-window rows
/// with hover bands and per-row hits (`key_base + visible index`). Widget port;
/// oracle is `ticker_native`.
#[allow(clippy::too_many_arguments)]
#[allow(clippy::needless_range_loop)]
pub(crate) fn ticker(
    show_glyph: bool,
    show_title: bool,
    meta: &str,
    status: &str,
    rows: &[(String, String, String, u32)],
    scroll: usize,
    key_base: u32,
    hover: u32,
    w: f32,
    h: f32,
    grid: f32,
    pal: &Pal,
) -> Node {
    let s = |n: f32| n * grid;
    let fs = |n: f32| (n * grid * 10.0).round() / 10.0;
    let p = s(12.0);
    let mut root = Node::box_(0.0, 0);
    if show_glyph {
        root = root.child(
            Node::glyph("\u{f201}", fs(12.0), crate::ui::fg2(pal))
                .halign(HAlign::Left)
                .valign(VAlign::Top)
                .margin(p, s(8.0), 0.0, 0.0),
        );
    }
    if show_title {
        root = root.child(
            Node::label("Prices", fs(11.5), pal.fg)
                .font(Ff::Ui, Fw::Semibold)
                .halign(HAlign::Left)
                .valign(VAlign::Top)
                .margin(p + s(17.0), s(9.0), 0.0, 0.0),
        );
    }
    let fg3 = crate::ui::fg3(pal);
    if !meta.is_empty() {
        root = root.child(
            Node::label(meta, fs(8.5), fg3)
                .font(Ff::Ui, Fw::Regular)
                .halign(HAlign::Right)
                .valign(VAlign::Top)
                .margin(0.0, s(11.0), p, 0.0),
        );
    }
    root = root.child(
        Node::anchored(0.0, -s(6.0))
            .text_(status)
            .size_(fs(9.0))
            .color_(fg3),
    );

    let n = rows.len();
    if n == 0 {
        return root;
    }
    let row_h = s(18.0);
    let pitch = s(18.0);
    let mut row_w = w - p * 2.0;
    if row_w < 2.0 {
        row_w = 2.0;
    }
    let start = scroll.min(n - 1);
    for i in start..n {
        let j = i - start;
        let ry = s(26.0) + j as f32 * pitch;
        if ry + row_h > h + 0.5 {
            break;
        }
        let key = key_base + j as u32;
        let (sym, price, chg, chg_color) = &rows[i];
        if hover == key && key != 0 {
            root = root.child(
                Node::box_(s(5.0), crate::ui::hover_hl(pal)).margin(
                    p,
                    ry,
                    w - p - row_w,
                    h - ry - row_h,
                ),
            );
        }
        root = root.child(
            Node::label(sym.as_str(), fs(9.5), pal.fg)
                .halign(HAlign::Left)
                .valign(VAlign::Top)
                .margin(p + s(6.0), ry + s(1.0), 0.0, 0.0),
        );
        root = root.child(
            Node::label(price.as_str(), fs(9.5), crate::ui::fg2(pal))
                .halign(HAlign::Left)
                .valign(VAlign::Top)
                .margin(p + s(62.0), ry + s(1.0), 0.0, 0.0),
        );
        root = root.child(
            Node::label(chg.as_str(), fs(9.0), *chg_color)
                .halign(HAlign::Right)
                .valign(VAlign::Top)
                .margin(0.0, ry + s(1.0), s(12.0), 0.0),
        );
        root = root.child(
            Node::hit(key).margin(p, ry, w - p - row_w, h - ry - row_h),
        );
    }
    root
}

/// The retired Prices drawer — the byte-parity oracle for `ticker`.
#[cfg(test)]
#[allow(clippy::too_many_arguments)]
#[allow(clippy::needless_range_loop)]
fn ticker_native(
    v: &mut Vec<Cmd>,
    show_glyph: bool,
    show_title: bool,
    meta: &str,
    status: &str,
    rows: &[(String, String, String, u32)],
    scroll: usize,
    key_base: u32,
    hover: u32,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    grid: f32,
    pal: &Pal,
) -> Vec<CardHit> {
    let s = |n: f32| n * grid;
    let fs = |n: f32| (n * grid * 10.0).round() / 10.0;
    let p = s(12.0);
    if show_glyph {
        crate::ui::text(v, x + p, y + s(8.0), "\u{f201}", fs(12.0), crate::ui::fg2(pal), true);
    }
    if show_title {
        crate::ui::title(v, x + p + s(17.0), y + s(9.0), "Prices", fs(11.5), pal.fg);
    }
    let fg3 = crate::ui::fg3(pal);
    if !meta.is_empty() {
        crate::ui::caption_r(v, x + w - p, y + s(11.0), meta, fs(8.5), fg3, false);
    }
    crate::ui::text_c(v, x + w / 2.0, y + h / 2.0 - s(6.0), status, fs(9.0), fg3, false);

    let n = rows.len();
    let mut hits = Vec::new();
    if n == 0 {
        return hits;
    }
    let row_h = s(18.0);
    let pitch = s(18.0);
    let mut row_w = w - p * 2.0;
    if row_w < 2.0 {
        row_w = 2.0;
    }
    let start = scroll.min(n - 1);
    for i in start..n {
        let j = i - start;
        let ry = y + s(26.0) + j as f32 * pitch;
        if ry + row_h > y + h + 0.5 {
            break;
        }
        let key = key_base + j as u32;
        let (sym, price, chg, chg_color) = &rows[i];
        if hover == key && key != 0 {
            v.push(Cmd::Rect {
                x: x + p,
                y: ry,
                w: row_w,
                h: row_h,
                r: s(5.0),
                color: crate::ui::hover_hl(pal),
            });
        }
        crate::ui::text(v, x + p + s(6.0), ry + s(1.0), sym, fs(9.5), pal.fg, false);
        crate::ui::text(v, x + p + s(62.0), ry + s(1.0), price, fs(9.5), crate::ui::fg2(pal), false);
        crate::ui::text_r(v, x + w - s(12.0), ry + s(1.0), chg, fs(9.0), *chg_color, false);
        hits.push((x + p, ry, row_w, row_h, key));
    }
    hits
}

/// The SSH / VPN card — header + centered status + the connection rows
/// (globe for VPN, shield for SSH, inked accent). Widget port; oracle
/// `sshvpn_native`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn sshvpn(
    show_title: bool,
    status: &str,
    rows: &[(String, String)],
    _w: f32,
    h: f32,
    grid: f32,
    pal: &Pal,
) -> Node {
    let s = |n: f32| n * grid;
    let fs = |n: f32| (n * grid * 10.0).round() / 10.0;
    let p = s(14.0);
    let mut root = Node::box_(0.0, 0);
    if show_title {
        root = root.child(
            Node::label("SSH / VPN", fs(11.5), pal.fg)
                .font(Ff::Ui, Fw::Semibold)
                .halign(HAlign::Left)
                .valign(VAlign::Top)
                .margin(p, s(9.0), 0.0, 0.0),
        );
    }
    root = root.child(
        Node::anchored(0.0, 0.0)
            .text_(status)
            .size_(fs(9.5))
            .color_(pal.fg),
    );
    let n = rows.len();
    if n == 0 {
        return root;
    }
    let row_h = s(22.0);
    let pitch = s(22.0);
    for (i, (icon, line)) in rows.iter().enumerate() {
        let ry = s(32.0) + i as f32 * pitch;
        if ry + row_h > h + 0.5 {
            break;
        }
        root = root.child(
            Node::glyph(icon.as_str(), fs(10.0), pal.acc)
                .halign(HAlign::Left)
                .valign(VAlign::Top)
                .margin(p, ry + s(1.0), 0.0, 0.0),
        );
        root = root.child(
            Node::label(line.as_str(), fs(9.5), pal.fg)
                .halign(HAlign::Left)
                .valign(VAlign::Top)
                .margin(p + s(30.0), ry + s(1.0), 0.0, 0.0),
        );
    }
    root
}

/// The retired SSH / VPN drawer — the byte-parity oracle for `sshvpn`.
#[cfg(test)]
#[allow(clippy::too_many_arguments)]
fn sshvpn_native(
    v: &mut Vec<Cmd>,
    show_title: bool,
    status: &str,
    rows: &[(String, String)],
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    grid: f32,
    pal: &Pal,
) {
    let s = |n: f32| n * grid;
    let fs = |n: f32| (n * grid * 10.0).round() / 10.0;
    let p = s(14.0);
    if show_title {
        crate::ui::title(v, x + p, y + s(9.0), "SSH / VPN", fs(11.5), pal.fg);
    }
    crate::ui::text_c(v, x + w / 2.0, y + h / 2.0, status, fs(9.5), pal.fg, false);
    let n = rows.len();
    if n == 0 {
        return;
    }
    let row_h = s(22.0);
    let pitch = s(22.0);
    for (i, (icon, line)) in rows.iter().enumerate() {
        let ry = y + s(32.0) + i as f32 * pitch;
        if ry + row_h > y + h + 0.5 {
            break;
        }
        crate::ui::text(v, x + p, ry + s(1.0), icon, fs(10.0), pal.acc, true);
        crate::ui::text(v, x + p + s(30.0), ry + s(1.0), line, fs(9.5), pal.fg, false);
    }
}

/// The Network info card — iface/gateway/dns rows plus the bottom-right
/// refresh well (key 31960; the module defined no `on_click` for it).
/// Widget port; oracle `conninfo_native`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn conninfo(
    show_glyph: bool,
    show_title: bool,
    meta: &str,
    rows: &[(String, String, u32, u32)],
    live: bool,
    hover: u32,
    w: f32,
    h: f32,
    grid: f32,
    pal: &Pal,
) -> Node {
    let s = |n: f32| n * grid;
    let fs = |n: f32| (n * grid * 10.0).round() / 10.0;
    let p = s(12.0);
    let fg3 = crate::ui::fg3(pal);
    let mut root = Node::box_(0.0, 0);
    let mut tx = p;
    if show_glyph {
        root = root.child(
            Node::glyph("\u{e250}", fs(12.0), crate::ui::fg2(pal))
                .halign(HAlign::Left)
                .valign(VAlign::Top)
                .margin(p, s(8.0), 0.0, 0.0),
        );
        tx = p + s(17.0);
    }
    if show_title {
        root = root.child(
            Node::label("Network", fs(11.5), pal.fg)
                .font(Ff::Ui, Fw::Semibold)
                .halign(HAlign::Left)
                .valign(VAlign::Top)
                .margin(tx, s(9.0), 0.0, 0.0),
        );
    }
    if !meta.is_empty() {
        root = root.child(
            Node::label(meta, fs(8.5), fg3)
                .font(Ff::Ui, Fw::Regular)
                .halign(HAlign::Right)
                .valign(VAlign::Top)
                .margin(0.0, s(11.0), p, 0.0),
        );
    }
    if !live {
        root = root.child(
            Node::anchored(0.0, 0.0)
                .text_("no network interfaces")
                .size_(fs(9.0))
                .color_(fg3),
        );
    } else {
        let row_h = s(18.0);
        let pitch = s(18.0);
        for (i, (label, value, lcolor, vcolor)) in rows.iter().enumerate() {
            let ry = s(30.0) + i as f32 * pitch;
            if ry + row_h > h - s(32.0) + 0.5 {
                break;
            }
            root = root.child(
                Node::label(label.as_str(), fs(8.5), *lcolor)
                    .font(Ff::Ui, Fw::Regular)
                    .halign(HAlign::Left)
                    .valign(VAlign::Top)
                    .margin(p, ry, 0.0, 0.0),
            );
            root = root.child(
                Node::label(value.as_str(), fs(8.5), *vcolor)
                    .font(Ff::Mono, Fw::Regular)
                    .halign(HAlign::Right)
                    .valign(VAlign::Top)
                    .margin(0.0, ry, s(14.0), 0.0),
            );
        }
    }
    let key = 31960;
    let hov = hover == key;
    let by = s(34.5);
    let bw = s(30.0);
    let bh = s(22.0);
    root = root.child(
        Node::box_(
            s(6.0),
            if hov { crate::ui::raised_hl(pal) } else { crate::ui::raised(pal) },
        )
        .margin(w - s(45.0), by, s(45.0) - bw, h - by - bh),
    );
    root = root.child(
        Node::anchored(w / 2.0 - s(30.0), by - h / 2.0)
            .text_("\u{f021}")
            .size_(fs(9.0))
            .color_(if hov { pal.acc } else { fg3 })
            .icon(),
    );
    root = root.child(Node::hit(key).margin(w - s(45.0), by, s(45.0) - bw, h - by - bh));
    root
}

/// The retired Network info drawer — the byte-parity oracle for `conninfo`.
#[cfg(test)]
#[allow(clippy::too_many_arguments)]
fn conninfo_native(
    v: &mut Vec<Cmd>,
    show_glyph: bool,
    show_title: bool,
    meta: &str,
    rows: &[(String, String, u32, u32)],
    live: bool,
    hover: u32,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    grid: f32,
    pal: &Pal,
) -> Vec<CardHit> {
    let s = |n: f32| n * grid;
    let fs = |n: f32| (n * grid * 10.0).round() / 10.0;
    let p = s(12.0);
    let fg3 = crate::ui::fg3(pal);
    let mut tx = p;
    if show_glyph {
        crate::ui::text(v, x + p, y + s(8.0), "\u{e250}", fs(12.0), crate::ui::fg2(pal), true);
        tx = p + s(17.0);
    }
    if show_title {
        crate::ui::title(v, x + tx, y + s(9.0), "Network", fs(11.5), pal.fg);
    }
    if !meta.is_empty() {
        crate::ui::caption_r(v, x + w - p, y + s(11.0), meta, fs(8.5), fg3, false);
    }
    if !live {
        crate::ui::text_c(v, x + w / 2.0, y + h / 2.0, "no network interfaces", fs(9.0), fg3, false);
    } else {
        let row_h = s(18.0);
        let pitch = s(18.0);
        for (i, (label, value, lcolor, vcolor)) in rows.iter().enumerate() {
            let ry = y + s(30.0) + i as f32 * pitch;
            if ry + row_h > y + h - s(32.0) + 0.5 {
                break;
            }
            crate::ui::caption(v, x + p, ry, label, fs(8.5), *lcolor, false);
            crate::ui::text_r_mono(v, x + w - s(14.0), ry, value, fs(8.5), *vcolor);
        }
    }
    // refresh well — right edge 45 in, top 34.5 down, 30×22
    let key = 31960;
    let hov = hover == key;
    let bx = x + w - s(45.0);
    let by = y + s(34.5);
    let bw = s(30.0);
    let bh = s(22.0);
    v.push(Cmd::Rect {
        x: bx,
        y: by,
        w: bw,
        h: bh,
        r: s(6.0),
        color: if hov { crate::ui::raised_hl(pal) } else { crate::ui::raised(pal) },
    });
    crate::ui::text_c(
        v,
        bx + bw / 2.0,
        by,
        "\u{f021}",
        fs(9.0),
        if hov { pal.acc } else { fg3 },
        true,
    );
    vec![(bx, by, bw, bh, key)]
}

/// The System log card — newest-first severity rows plus the refresh well
/// (key 31995 → the host's `refresh_journal` action). Widget port; oracle
/// `journaltail_native`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn journaltail(
    show_glyph: bool,
    show_title: bool,
    meta: &str,
    rows: &[(String, u32)],
    live: bool,
    hover: u32,
    w: f32,
    h: f32,
    grid: f32,
    pal: &Pal,
) -> Node {
    let s = |n: f32| n * grid;
    let fs = |n: f32| (n * grid * 10.0).round() / 10.0;
    let p = s(12.0);
    let fg3 = crate::ui::fg3(pal);
    let mut root = Node::box_(0.0, 0);
    let mut tx = p;
    if show_glyph {
        root = root.child(
            Node::glyph("\u{f2d0}", fs(12.0), crate::ui::fg2(pal))
                .halign(HAlign::Left)
                .valign(VAlign::Top)
                .margin(p, s(8.0), 0.0, 0.0),
        );
        tx = p + s(17.0);
    }
    if show_title {
        root = root.child(
            Node::label("System log", fs(11.5), pal.fg)
                .font(Ff::Ui, Fw::Semibold)
                .halign(HAlign::Left)
                .valign(VAlign::Top)
                .margin(tx, s(9.0), 0.0, 0.0),
        );
    }
    if !meta.is_empty() {
        root = root.child(
            Node::label(meta, fs(8.5), fg3)
                .font(Ff::Ui, Fw::Regular)
                .halign(HAlign::Right)
                .valign(VAlign::Top)
                .margin(0.0, s(11.0), p, 0.0),
        );
    }
    if !live {
        root = root.child(
            Node::anchored(0.0, 0.0)
                .text_("journal empty")
                .size_(fs(9.0))
                .color_(fg3),
        );
    } else {
        let row_h = s(17.0);
        let pitch = s(17.0);
        let avail = w - p * 2.0 - s(16.0);
        let gp = 0.62 * fs(8.5);
        for (i, (text, color)) in rows.iter().enumerate() {
            let ry = s(26.0) + i as f32 * pitch;
            if ry + row_h > h - s(28.0) + 0.5 {
                break;
            }
            let t = trunc_at(text, avail, gp);
            root = root.child(
                Node::label(t, fs(8.5), *color)
                    .font(Ff::Ui, Fw::Regular)
                    .halign(HAlign::Left)
                    .valign(VAlign::Top)
                    .margin(p, ry, 0.0, 0.0),
            );
        }
    }
    let key = 31995;
    let hov = hover == key;
    let by = s(29.5);
    let bw = s(30.0);
    let bh = s(20.0);
    root = root.child(
        Node::box_(
            s(6.0),
            if hov { crate::ui::raised_hl(pal) } else { crate::ui::raised(pal) },
        )
        .margin(w - s(45.0), by, s(45.0) - bw, h - by - bh),
    );
    root = root.child(
        Node::anchored(w / 2.0 - s(30.0), by - h / 2.0)
            .text_("\u{f021}")
            .size_(fs(9.0))
            .color_(if hov { pal.acc } else { fg3 })
            .icon(),
    );
    root = root.child(Node::hit(key).margin(w - s(45.0), by, s(45.0) - bw, h - by - bh));
    root
}

/// The retired System log drawer — the byte-parity oracle for `journaltail`.
#[cfg(test)]
#[allow(clippy::too_many_arguments)]
fn journaltail_native(
    v: &mut Vec<Cmd>,
    show_glyph: bool,
    show_title: bool,
    meta: &str,
    rows: &[(String, u32)],
    live: bool,
    hover: u32,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    grid: f32,
    pal: &Pal,
) -> Vec<CardHit> {
    let s = |n: f32| n * grid;
    let fs = |n: f32| (n * grid * 10.0).round() / 10.0;
    let p = s(12.0);
    let fg3 = crate::ui::fg3(pal);
    let mut tx = p;
    if show_glyph {
        crate::ui::text(v, x + p, y + s(8.0), "\u{f2d0}", fs(12.0), crate::ui::fg2(pal), true);
        tx = p + s(17.0);
    }
    if show_title {
        crate::ui::title(v, x + tx, y + s(9.0), "System log", fs(11.5), pal.fg);
    }
    if !meta.is_empty() {
        crate::ui::caption_r(v, x + w - p, y + s(11.0), meta, fs(8.5), fg3, false);
    }
    if !live {
        crate::ui::text_c(v, x + w / 2.0, y + h / 2.0, "journal empty", fs(9.0), fg3, false);
    } else {
        let row_h = s(17.0);
        let pitch = s(17.0);
        let avail = w - p * 2.0 - s(16.0);
        let gp = 0.62 * fs(8.5);
        for (i, (text, color)) in rows.iter().enumerate() {
            let ry = y + s(26.0) + i as f32 * pitch;
            if ry + row_h > y + h - s(28.0) + 0.5 {
                break;
            }
            let t = trunc_at(text, avail, gp);
            crate::ui::caption(v, x + p, ry, &t, fs(8.5), *color, false);
        }
    }
    let key = 31995;
    let hov = hover == key;
    let bx = x + w - s(45.0);
    let by = y + s(29.5);
    let bw = s(30.0);
    let bh = s(20.0);
    v.push(Cmd::Rect {
        x: bx,
        y: by,
        w: bw,
        h: bh,
        r: s(6.0),
        color: if hov { crate::ui::raised_hl(pal) } else { crate::ui::raised(pal) },
    });
    crate::ui::text_c(v, bx + bw / 2.0, by, "\u{f021}", fs(9.0), if hov { pal.acc } else { fg3 }, true);
    vec![(bx, by, bw, bh, key)]
}

/// The Services card — failed-unit chips plus the refresh well (key 31990 →
/// `refresh_systemd`). Widget port; oracle `systemdunits_native`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn systemdunits(
    show_glyph: bool,
    show_title: bool,
    icon: &str,
    meta: &str,
    rows: &[(String, u32)],
    live: bool,
    hover: u32,
    w: f32,
    h: f32,
    grid: f32,
    pal: &Pal,
) -> Node {
    let s = |n: f32| n * grid;
    let fs = |n: f32| (n * grid * 10.0).round() / 10.0;
    let p = s(12.0);
    let fg3 = crate::ui::fg3(pal);
    let mut root = Node::box_(0.0, 0);
    let mut tx = p;
    if show_glyph {
        root = root.child(
            Node::glyph(icon, fs(12.0), crate::ui::fg2(pal))
                .halign(HAlign::Left)
                .valign(VAlign::Top)
                .margin(p, s(8.0), 0.0, 0.0),
        );
        tx = p + s(17.0);
    }
    if show_title {
        root = root.child(
            Node::label("Services", fs(11.5), pal.fg)
                .font(Ff::Ui, Fw::Semibold)
                .halign(HAlign::Left)
                .valign(VAlign::Top)
                .margin(tx, s(9.0), 0.0, 0.0),
        );
    }
    if !meta.is_empty() {
        root = root.child(
            Node::label(meta, fs(8.5), fg3)
                .font(Ff::Ui, Fw::Regular)
                .halign(HAlign::Right)
                .valign(VAlign::Top)
                .margin(0.0, s(11.0), p, 0.0),
        );
    }
    if !live {
        root = root.child(
            Node::anchored(0.0, 0.0)
                .text_("all services up")
                .size_(fs(9.0))
                .color_(fg3),
        );
    } else {
        let row_h = s(19.0);
        let pitch = s(19.0);
        let avail = w - p * 2.0 - s(20.0);
        let gp = 0.62 * fs(8.5);
        for (i, (unit, color)) in rows.iter().enumerate() {
            let ry = s(28.0) + i as f32 * pitch;
            if ry + row_h > h - s(28.0) + 0.5 {
                break;
            }
            root = root.child(
                Node::box_(s(5.0), crate::ui::raised(pal))
                    .margin(p, ry, p, h - ry - row_h),
            );
            let t = trunc_at(unit, avail, gp);
            root = root.child(
                Node::label(t, fs(8.5), *color)
                    .font(Ff::Ui, Fw::Regular)
                    .halign(HAlign::Left)
                    .valign(VAlign::Top)
                    .margin(p + s(6.0), ry, 0.0, 0.0),
            );
            root = root.child(
                Node::label(crate::icons::ICON_CLOSE, fs(9.0), *color)
                    .font(Ff::Ui, Fw::Regular)
                    .icon()
                    .halign(HAlign::Right)
                    .valign(VAlign::Top)
                    .margin(0.0, ry, s(14.0), 0.0),
            );
        }
    }
    let key = 31990;
    let hov = hover == key;
    let by = s(29.5);
    let bw = s(30.0);
    let bh = s(20.0);
    root = root.child(
        Node::box_(
            s(6.0),
            if hov { crate::ui::raised_hl(pal) } else { crate::ui::raised(pal) },
        )
        .margin(w - s(45.0), by, s(45.0) - bw, h - by - bh),
    );
    root = root.child(
        Node::anchored(w / 2.0 - s(30.0), by - h / 2.0)
            .text_("\u{f021}")
            .size_(fs(9.0))
            .color_(if hov { pal.acc } else { fg3 })
            .icon(),
    );
    root = root.child(Node::hit(key).margin(w - s(45.0), by, s(45.0) - bw, h - by - bh));
    root
}

/// The retired Services drawer — the byte-parity oracle for `systemdunits`.
#[cfg(test)]
#[allow(clippy::too_many_arguments)]
fn systemdunits_native(
    v: &mut Vec<Cmd>,
    show_glyph: bool,
    show_title: bool,
    icon: &str,
    meta: &str,
    rows: &[(String, u32)],
    live: bool,
    hover: u32,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    grid: f32,
    pal: &Pal,
) -> Vec<CardHit> {
    let s = |n: f32| n * grid;
    let fs = |n: f32| (n * grid * 10.0).round() / 10.0;
    let p = s(12.0);
    let fg3 = crate::ui::fg3(pal);
    let mut tx = p;
    if show_glyph {
        crate::ui::text(v, x + p, y + s(8.0), icon, fs(12.0), crate::ui::fg2(pal), true);
        tx = p + s(17.0);
    }
    if show_title {
        crate::ui::title(v, x + tx, y + s(9.0), "Services", fs(11.5), pal.fg);
    }
    if !meta.is_empty() {
        crate::ui::caption_r(v, x + w - p, y + s(11.0), meta, fs(8.5), fg3, false);
    }
    if !live {
        crate::ui::text_c(v, x + w / 2.0, y + h / 2.0, "all services up", fs(9.0), fg3, false);
    } else {
        let row_h = s(19.0);
        let pitch = s(19.0);
        let row_w = w - p * 2.0;
        let avail = w - p * 2.0 - s(20.0);
        let gp = 0.62 * fs(8.5);
        for (i, (unit, color)) in rows.iter().enumerate() {
            let ry = y + s(28.0) + i as f32 * pitch;
            if ry + row_h > y + h - s(28.0) + 0.5 {
                break;
            }
            v.push(Cmd::Rect {
                x: x + p,
                y: ry,
                w: row_w,
                h: row_h,
                r: s(5.0),
                color: crate::ui::raised(pal),
            });
            let t = trunc_at(unit, avail, gp);
            crate::ui::caption(v, x + p + s(6.0), ry, &t, fs(8.5), *color, false);
            crate::ui::caption_r(v, x + w - s(14.0), ry, crate::icons::ICON_CLOSE, fs(9.0), *color, true);
        }
    }
    let key = 31990;
    let hov = hover == key;
    let bx = x + w - s(45.0);
    let by = y + s(29.5);
    let bw = s(30.0);
    let bh = s(20.0);
    v.push(Cmd::Rect {
        x: bx,
        y: by,
        w: bw,
        h: bh,
        r: s(6.0),
        color: if hov { crate::ui::raised_hl(pal) } else { crate::ui::raised(pal) },
    });
    crate::ui::text_c(v, bx + bw / 2.0, by, "\u{f021}", fs(9.0), if hov { pal.acc } else { fg3 }, true);
    vec![(bx, by, bw, bh, key)]
}

/// One SMART disk row: `(dev, model, temp, model_ink, dev_ink, temp_ink,
/// state_ink, state_glyph)`.
pub(crate) type SmartRow = (String, String, String, u32, u32, u32, u32, String);

/// The Disk health card — SMART rows plus the refresh well (key 31980 →
/// `refresh_smarthealth`). Widget port; oracle `smarthealth_native`.
#[allow(clippy::too_many_arguments)]
#[allow(clippy::type_complexity)]
pub(crate) fn smarthealth(
    show_glyph: bool,
    show_title: bool,
    icon: &str,
    meta: &str,
    rows: &[SmartRow],
    live: bool,
    hover: u32,
    w: f32,
    h: f32,
    grid: f32,
    pal: &Pal,
) -> Node {
    let s = |n: f32| n * grid;
    let fs = |n: f32| (n * grid * 10.0).round() / 10.0;
    let p = s(12.0);
    let fg3 = crate::ui::fg3(pal);
    let mut root = Node::box_(0.0, 0);
    let mut tx = p;
    if show_glyph {
        root = root.child(
            Node::glyph(icon, fs(12.0), crate::ui::fg2(pal))
                .halign(HAlign::Left)
                .valign(VAlign::Top)
                .margin(p, s(8.0), 0.0, 0.0),
        );
        tx = p + s(17.0);
    }
    if show_title {
        root = root.child(
            Node::label("Disk health", fs(11.5), pal.fg)
                .font(Ff::Ui, Fw::Semibold)
                .halign(HAlign::Left)
                .valign(VAlign::Top)
                .margin(tx, s(9.0), 0.0, 0.0),
        );
    }
    if !meta.is_empty() {
        root = root.child(
            Node::label(meta, fs(8.5), fg3)
                .font(Ff::Ui, Fw::Regular)
                .halign(HAlign::Right)
                .valign(VAlign::Top)
                .margin(0.0, s(11.0), p, 0.0),
        );
    }
    if !live {
        root = root.child(
            Node::anchored(0.0, 0.0)
                .text_("no SMART disks")
                .size_(fs(9.0))
                .color_(fg3),
        );
    } else {
        let row_h = s(23.0);
        let pitch = s(23.0);
        let avail = w - p * 2.0 - s(96.0);
        let gp = 0.62 * fs(8.5);
        for (i, (dev, model, temp, dcolor, mcolor, tcolor, scolor, sg)) in
            rows.iter().enumerate()
        {
            let ry = s(28.0) + i as f32 * pitch;
            if ry + row_h > h - s(28.0) + 0.5 {
                break;
            }
            root = root.child(
                Node::label(dev.as_str(), fs(8.5), *dcolor)
                    .font(Ff::Mono, Fw::Regular)
                    .halign(HAlign::Left)
                    .valign(VAlign::Top)
                    .margin(p, ry, 0.0, 0.0),
            );
            let m = trunc_at(model, avail, gp);
            root = root.child(
                Node::label(m, fs(8.5), *mcolor)
                    .font(Ff::Ui, Fw::Regular)
                    .halign(HAlign::Left)
                    .valign(VAlign::Top)
                    .margin(p + s(40.0), ry, 0.0, 0.0),
            );
            root = root.child(
                Node::label(temp.as_str(), fs(8.5), *tcolor)
                    .font(Ff::Mono, Fw::Regular)
                    .halign(HAlign::Right)
                    .valign(VAlign::Top)
                    .margin(0.0, ry, s(56.0), 0.0),
            );
            root = root.child(
                Node::label(sg.as_str(), fs(10.0), *scolor)
                    .font(Ff::Ui, Fw::Regular)
                    .icon()
                    .halign(HAlign::Right)
                    .valign(VAlign::Top)
                    .margin(0.0, ry + s(-0.5), s(10.0), 0.0),
            );
        }
    }
    let key = 31980;
    let hov = hover == key;
    let by = s(29.5);
    let bw = s(30.0);
    let bh = s(20.0);
    root = root.child(
        Node::box_(
            s(6.0),
            if hov { crate::ui::raised_hl(pal) } else { crate::ui::raised(pal) },
        )
        .margin(w - s(45.0), by, s(45.0) - bw, h - by - bh),
    );
    root = root.child(
        Node::anchored(w / 2.0 - s(30.0), by - h / 2.0)
            .text_("\u{f021}")
            .size_(fs(9.0))
            .color_(if hov { pal.acc } else { fg3 })
            .icon(),
    );
    root = root.child(Node::hit(key).margin(w - s(45.0), by, s(45.0) - bw, h - by - bh));
    root
}

/// The retired Disk health drawer — the byte-parity oracle for `smarthealth`.
#[cfg(test)]
#[allow(clippy::too_many_arguments)]
fn smarthealth_native(
    v: &mut Vec<Cmd>,
    show_glyph: bool,
    show_title: bool,
    icon: &str,
    meta: &str,
    rows: &[SmartRow],
    live: bool,
    hover: u32,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    grid: f32,
    pal: &Pal,
) -> Vec<CardHit> {
    let s = |n: f32| n * grid;
    let fs = |n: f32| (n * grid * 10.0).round() / 10.0;
    let p = s(12.0);
    let fg3 = crate::ui::fg3(pal);
    let mut tx = p;
    if show_glyph {
        crate::ui::text(v, x + p, y + s(8.0), icon, fs(12.0), crate::ui::fg2(pal), true);
        tx = p + s(17.0);
    }
    if show_title {
        crate::ui::title(v, x + tx, y + s(9.0), "Disk health", fs(11.5), pal.fg);
    }
    if !meta.is_empty() {
        crate::ui::caption_r(v, x + w - p, y + s(11.0), meta, fs(8.5), fg3, false);
    }
    if !live {
        crate::ui::text_c(v, x + w / 2.0, y + h / 2.0, "no SMART disks", fs(9.0), fg3, false);
    } else {
        let row_h = s(23.0);
        let pitch = s(23.0);
        let avail = w - p * 2.0 - s(96.0);
        let gp = 0.62 * fs(8.5);
        for (i, (dev, model, temp, dcolor, mcolor, tcolor, scolor, sg)) in
            rows.iter().enumerate()
        {
            let ry = y + s(28.0) + i as f32 * pitch;
            if ry + row_h > y + h - s(28.0) + 0.5 {
                break;
            }
            crate::ui::text_mono(v, x + p, ry, dev, fs(8.5), *dcolor);
            let m = trunc_at(model, avail, gp);
            crate::ui::caption(v, x + p + s(40.0), ry, &m, fs(8.5), *mcolor, false);
            crate::ui::text_r_mono(v, x + w - s(56.0), ry, temp, fs(8.5), *tcolor);
            crate::ui::caption_r(v, x + w - s(10.0), ry + s(-0.5), sg, fs(10.0), *scolor, true);
        }
    }
    let key = 31980;
    let hov = hover == key;
    let bx = x + w - s(45.0);
    let by = y + s(29.5);
    let bw = s(30.0);
    let bh = s(20.0);
    v.push(Cmd::Rect {
        x: bx,
        y: by,
        w: bw,
        h: bh,
        r: s(6.0),
        color: if hov { crate::ui::raised_hl(pal) } else { crate::ui::raised(pal) },
    });
    crate::ui::text_c(v, bx + bw / 2.0, by, "\u{f021}", fs(9.0), if hov { pal.acc } else { fg3 }, true);
    vec![(bx, by, bw, bh, key)]
}

/// The Docker card — status dot + name + (capped) image label per container.
/// Widget port; oracle `docker_native`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn docker(
    show_title: bool,
    rows: &[(u32, String, String)],
    w: f32,
    h: f32,
    grid: f32,
    pal: &Pal,
) -> Node {
    let s = |n: f32| n * grid;
    let fs = |n: f32| (n * grid * 10.0).round() / 10.0;
    let p = s(14.0);
    let mut root = Node::box_(0.0, 0);
    if show_title {
        root = root.child(
            Node::label("Docker", fs(11.5), pal.fg)
                .font(Ff::Ui, Fw::Semibold)
                .halign(HAlign::Left)
                .valign(VAlign::Top)
                .margin(p, s(9.0), 0.0, 0.0),
        );
    }
    if rows.is_empty() {
        return root.child(
            Node::anchored(0.0, 0.0)
                .text_("No containers")
                .size_(fs(9.5))
                .color_(pal.fg),
        );
    }
    let row_h = s(26.0);
    let pitch = s(26.0);
    for (i, (dot, name, image)) in rows.iter().enumerate() {
        let ry = s(32.0) + i as f32 * pitch;
        if ry + row_h > h - s(4.0) + 0.5 {
            break;
        }
        root = root.child(
            Node::box_(s(3.0), *dot).margin(
                p,
                ry + s(3.0),
                w - p - s(6.0),
                h - (ry + s(3.0)) - s(6.0),
            ),
        );
        root = root.child(
            Node::label(name.as_str(), fs(9.0), pal.fg)
                .font(Ff::Ui, Fw::Regular)
                .halign(HAlign::Left)
                .valign(VAlign::Top)
                .margin(p + s(10.0), ry, 0.0, 0.0),
        );
        let img: String = if image.chars().count() > 20 {
            image.chars().take(20).collect()
        } else {
            image.clone()
        };
        root = root.child(
            Node::label(img, fs(7.5), pal.fg)
                .font(Ff::Ui, Fw::Regular)
                .halign(HAlign::Right)
                .valign(VAlign::Top)
                .margin(0.0, ry, s(14.0), 0.0),
        );
    }
    root
}

/// The retired Docker drawer — the byte-parity oracle for `docker`.
#[cfg(test)]
#[allow(clippy::too_many_arguments)]
fn docker_native(
    v: &mut Vec<Cmd>,
    show_title: bool,
    rows: &[(u32, String, String)],
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    grid: f32,
    pal: &Pal,
) {
    let s = |n: f32| n * grid;
    let fs = |n: f32| (n * grid * 10.0).round() / 10.0;
    let p = s(14.0);
    if show_title {
        crate::ui::title(v, x + p, y + s(9.0), "Docker", fs(11.5), pal.fg);
    }
    if rows.is_empty() {
        crate::ui::text_c(v, x + w / 2.0, y + h / 2.0, "No containers", fs(9.5), pal.fg, false);
        return;
    }
    let row_h = s(26.0);
    let pitch = s(26.0);
    for (i, (dot, name, image)) in rows.iter().enumerate() {
        let ry = y + s(32.0) + i as f32 * pitch;
        if ry + row_h > y + h - s(4.0) + 0.5 {
            break;
        }
        v.push(Cmd::Rect {
            x: x + p,
            y: ry + s(3.0),
            w: s(6.0),
            h: s(6.0),
            r: s(3.0),
            color: *dot,
        });
        crate::ui::caption(v, x + p + s(10.0), ry, name, fs(9.0), pal.fg, false);
        let img: String = if image.chars().count() > 20 {
            image.chars().take(20).collect()
        } else {
            image.clone()
        };
        crate::ui::caption_r(v, x + w - s(14.0), ry, &img, fs(7.5), pal.fg, false);
    }
}

/// The Processes card — name + right-anchored CPU-share meter per process.
/// Widget port; oracle `procmon_native`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn procmon(
    show_title: bool,
    meta: &str,
    rows: &[(String, f32)],
    w: f32,
    h: f32,
    grid: f32,
    pal: &Pal,
) -> Node {
    let s = |n: f32| n * grid;
    let fs = |n: f32| (n * grid * 10.0).round() / 10.0;
    let p = s(12.0);
    let mut root = Node::box_(0.0, 0);
    if show_title {
        root = root.child(
            Node::label("Processes", fs(11.5), pal.fg)
                .font(Ff::Ui, Fw::Semibold)
                .halign(HAlign::Left)
                .valign(VAlign::Top)
                .margin(p, s(9.0), 0.0, 0.0),
        );
    }
    if !meta.is_empty() {
        root = root.child(
            Node::label(meta, fs(8.5), crate::ui::fg3(pal))
                .font(Ff::Mono, Fw::Regular)
                .halign(HAlign::Right)
                .valign(VAlign::Top)
                .margin(0.0, s(11.0), p, 0.0),
        );
    }
    let row_h = s(22.0);
    let pitch = s(22.0);
    let pmin = s(14.0);
    let avail = w - pmin * 2.0 - s(64.0);
    let gp = 0.62 * fs(9.0);
    let bw = s(80.0);
    let bl = w - s(14.0) - bw;
    for (i, (name, frac)) in rows.iter().enumerate() {
        let ry = s(32.0) + i as f32 * pitch;
        if ry + row_h <= h + 0.5 {
            let label = trunc_at(name, avail, gp);
            root = root.child(
                Node::label(label, fs(9.0), pal.fg)
                    .font(Ff::Ui, Fw::Regular)
                    .halign(HAlign::Left)
                    .valign(VAlign::Top)
                    .margin(pmin, ry, 0.0, 0.0),
            );
            root = root.child(
                Node::bar(*frac, 1.0, pal.acc, crate::ui::hover(pal), s(2.0), false).margin(
                    bl,
                    ry + s(14.0),
                    w - bl - bw,
                    h - (ry + s(14.0)) - s(4.0),
                ),
            );
        }
    }
    root
}

/// The retired Processes drawer — the byte-parity oracle for `procmon`.
#[cfg(test)]
#[allow(clippy::too_many_arguments)]
fn procmon_native(
    v: &mut Vec<Cmd>,
    show_title: bool,
    meta: &str,
    rows: &[(String, f32)],
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    grid: f32,
    pal: &Pal,
) {
    let s = |n: f32| n * grid;
    let fs = |n: f32| (n * grid * 10.0).round() / 10.0;
    let p = s(12.0);
    if show_title {
        crate::ui::title(v, x + p, y + s(9.0), "Processes", fs(11.5), pal.fg);
    }
    if !meta.is_empty() {
        crate::ui::text_r_mono(v, x + w - p, y + s(11.0), meta, fs(8.5), crate::ui::fg3(pal));
    }
    let row_h = s(22.0);
    let pitch = s(22.0);
    let pmin = s(14.0);
    let avail = w - pmin * 2.0 - s(64.0);
    let gp = 0.62 * fs(9.0);
    let bw = s(80.0);
    let bl = x + w - s(14.0) - bw;
    for (i, (name, frac)) in rows.iter().enumerate() {
        let ry = y + s(32.0) + i as f32 * pitch;
        if ry + row_h <= y + h + 0.5 {
            let label = trunc_at(name, avail, gp);
            crate::ui::caption(v, x + pmin, ry, &label, fs(9.0), pal.fg, false);
            v.push(Cmd::Rect {
                x: bl,
                y: ry + s(14.0),
                w: bw,
                h: s(4.0),
                r: s(2.0),
                color: crate::ui::hover(pal),
            });
            let fw = bw * frac;
            if fw > 0.5 {
                v.push(Cmd::Rect {
                    x: bl,
                    y: ry + s(14.0),
                    w: fw,
                    h: s(4.0),
                    r: s(2.0),
                    color: pal.acc,
                });
            }
        }
    }
}

/// The Sensors card — tilt/compass/gyro rows from the platform sensor set.
/// Widget port; oracle `sensors_native`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn sensors(
    show_glyph: bool,
    show_title: bool,
    none: bool,
    tilt: bool,
    compass: bool,
    gyro: bool,
    tilt_txt: &str,
    compass_txt: &str,
    gyro_txt: &str,
    _w: f32,
    _h: f32,
    grid: f32,
    pal: &Pal,
) -> Node {
    let s = |n: f32| n * grid;
    let fs = |n: f32| (n * grid * 10.0).round() / 10.0;
    let p = s(12.0);
    let fg3 = crate::ui::fg3(pal);
    let mut root = Node::box_(0.0, 0);
    let mut tx = p;
    if show_glyph {
        root = root.child(
            Node::glyph("\u{f6c7}", fs(12.0), pal.acc)
                .halign(HAlign::Left)
                .valign(VAlign::Top)
                .margin(p, s(8.0), 0.0, 0.0),
        );
        tx = p + s(17.0);
    }
    if show_title {
        root = root.child(
            Node::label("Sensors", fs(11.5), pal.fg)
                .font(Ff::Ui, Fw::Semibold)
                .halign(HAlign::Left)
                .valign(VAlign::Top)
                .margin(tx, s(9.0), 0.0, 0.0),
        );
    }
    if none {
        root = root.child(
            Node::anchored(0.0, 0.0)
                .text_("no IIO sensors")
                .size_(fs(9.0))
                .color_(fg3),
        );
    }
    if tilt {
        root = root.child(
            Node::label("tilt", fs(9.0), crate::ui::fg2(pal))
                .halign(HAlign::Left)
                .valign(VAlign::Top)
                .margin(s(16.0), s(32.0), 0.0, 0.0),
        );
        root = root.child(
            Node::label(tilt_txt, fs(9.5), pal.fg)
                .halign(HAlign::Right)
                .valign(VAlign::Top)
                .margin(0.0, s(32.0), s(16.0), 0.0),
        );
    }
    if compass {
        root = root.child(
            Node::label("compass", fs(9.0), crate::ui::fg2(pal))
                .halign(HAlign::Left)
                .valign(VAlign::Top)
                .margin(s(16.0), s(52.0), 0.0, 0.0),
        );
        root = root.child(
            Node::glyph(compass_txt, fs(9.5), pal.acc)
                .halign(HAlign::Right)
                .valign(VAlign::Top)
                .margin(0.0, s(52.0), s(16.0), 0.0),
        );
    }
    if gyro {
        root = root.child(
            Node::label("gyro \u{b0}/s", fs(9.0), crate::ui::fg2(pal))
                .halign(HAlign::Left)
                .valign(VAlign::Top)
                .margin(s(16.0), s(72.0), 0.0, 0.0),
        );
        root = root.child(
            Node::label(gyro_txt, fs(9.0), pal.fg)
                .font(Ff::Mono, Fw::Regular)
                .halign(HAlign::Right)
                .valign(VAlign::Top)
                .margin(0.0, s(72.0), s(16.0), 0.0),
        );
    }
    root
}

/// The retired Sensors drawer — the byte-parity oracle for `sensors`.
#[cfg(test)]
#[allow(clippy::too_many_arguments)]
fn sensors_native(
    v: &mut Vec<Cmd>,
    show_glyph: bool,
    show_title: bool,
    none: bool,
    tilt: bool,
    compass: bool,
    gyro: bool,
    tilt_txt: &str,
    compass_txt: &str,
    gyro_txt: &str,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    grid: f32,
    pal: &Pal,
) {
    let s = |n: f32| n * grid;
    let fs = |n: f32| (n * grid * 10.0).round() / 10.0;
    let p = s(12.0);
    let mut tx = p;
    if show_glyph {
        crate::ui::text(v, x + p, y + s(8.0), "\u{f6c7}", fs(12.0), pal.acc, true);
        tx = p + s(17.0);
    }
    if show_title {
        crate::ui::title(v, x + tx, y + s(9.0), "Sensors", fs(11.5), pal.fg);
    }
    if none {
        crate::ui::text_c(v, x + w / 2.0, y + h / 2.0, "no IIO sensors", fs(9.0), crate::ui::fg3(pal), false);
    }
    if tilt {
        crate::ui::text(v, x + s(16.0), y + s(32.0), "tilt", fs(9.0), crate::ui::fg2(pal), false);
        crate::ui::text_r(v, x + w - s(16.0), y + s(32.0), tilt_txt, fs(9.5), pal.fg, false);
    }
    if compass {
        crate::ui::text(v, x + s(16.0), y + s(52.0), "compass", fs(9.0), crate::ui::fg2(pal), false);
        crate::ui::text_r(v, x + w - s(16.0), y + s(52.0), compass_txt, fs(9.5), pal.acc, true);
    }
    if gyro {
        crate::ui::text(v, x + s(16.0), y + s(72.0), "gyro \u{b0}/s", fs(9.0), crate::ui::fg2(pal), false);
        crate::ui::text_r_mono(v, x + w - s(16.0), y + s(72.0), gyro_txt, fs(9.0), pal.fg);
    }
}

/// The Thermal zones card — a 3-wide chip grid whose cells register key `i`.
/// The /3 cell-width is double-precision in the module (every coordinate is a
/// Lua f64 coerced to f32 per primitive call), so the grid math runs in f64
/// and casts at the boundary to land bit-identically. Widget port; oracle
/// `thermal_native`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn thermal(
    show_glyph: bool,
    show_title: bool,
    glyph: &str,
    meta: &str,
    cells: &[(String, String, u32)],
    empty: bool,
    live: bool,
    w: f32,
    h: f32,
    grid: f32,
    pal: &Pal,
) -> Node {
    let s = |n: f32| n * grid;
    let fs = |n: f32| (n * grid * 10.0).round() / 10.0;
    let p = s(12.0);
    let mut root = Node::box_(0.0, 0);
    if show_glyph {
        root = root.child(
            Node::glyph(glyph, fs(12.0), crate::ui::fg2(pal))
                .halign(HAlign::Left)
                .valign(VAlign::Top)
                .margin(p, s(8.0), 0.0, 0.0),
        );
    }
    if show_title {
        root = root.child(
            Node::label("Thermal zones", fs(11.5), pal.fg)
                .font(Ff::Ui, Fw::Semibold)
                .halign(HAlign::Left)
                .valign(VAlign::Top)
                .margin(p + s(17.0), s(9.0), 0.0, 0.0),
        );
    }
    if !meta.is_empty() {
        root = root.child(
            Node::label(meta, fs(8.5), crate::ui::fg3(pal))
                .font(Ff::Ui, Fw::Regular)
                .halign(HAlign::Right)
                .valign(VAlign::Top)
                .margin(0.0, s(11.0), p, 0.0),
        );
    }
    if empty {
        return root.child(
            Node::anchored(0.0, 0.0)
                .text_("no /sys/class/thermal zones")
                .size_(fs(9.0))
                .color_(crate::ui::fg3(pal)),
        );
    }
    if live {
        let cell_w = (w as f64 - 4.0 * grid as f64) / 3.0;
        let cell_h = 24.0 * grid as f64;
        let gap = 2.0 * grid as f64;
        let grid_y = 26.0 * grid as f64;
        let h_lim = h as f64 + 0.5;
        for (i, (label, sub, sub_color)) in cells.iter().enumerate() {
            let col = (i % 3) as f64;
            let row = (i / 3) as f64;
            let cx = col * (cell_w + gap);
            let cy = grid_y + row * (cell_h + gap);
            if cy > h_lim {
                break;
            }
            let cx_f = cx as f32;
            let cy_f = cy as f32;
            let cw = cell_w as f32;
            let ch = cell_h as f32;
            root = root.child(
                Node::box_((5.0 * grid as f64) as f32, crate::ui::hover(pal)).margin(
                    cx_f,
                    cy_f,
                    w - cx_f - cw,
                    h - cy_f - ch,
                ),
            );
            let lx = (cx + cell_w / 2.0) as f32;
            root = root.child(
                Node::anchored(lx - w / 2.0, (cy + 3.0 * grid as f64) as f32 - h / 2.0)
                    .text_(label)
                    .size_(fs(7.5))
                    .font(Ff::Ui, Fw::Regular)
                    .color_(pal.fg),
            );
            root = root.child(
                Node::anchored(lx - w / 2.0, (cy + 12.0 * grid as f64) as f32 - h / 2.0)
                    .text_(sub)
                    .size_(fs(9.0))
                    .font(Ff::Ui, Fw::Regular)
                    .color_(*sub_color),
            );
            root = root.child(
                Node::hit(i as u32).margin(cx_f, cy_f, w - cx_f - cw, h - cy_f - ch),
            );
        }
    }
    root
}

/// The retired Thermal zones drawer — the byte-parity oracle for `thermal`.
#[cfg(test)]
#[allow(clippy::too_many_arguments)]
fn thermal_native(
    v: &mut Vec<Cmd>,
    show_glyph: bool,
    show_title: bool,
    glyph: &str,
    meta: &str,
    cells: &[(String, String, u32)],
    empty: bool,
    live: bool,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    grid: f32,
    pal: &Pal,
) -> Vec<CardHit> {
    let s = |n: f32| n * grid;
    let fs = |n: f32| (n * grid * 10.0).round() / 10.0;
    let p = s(12.0);
    if show_glyph {
        crate::ui::text(v, x + p, y + s(8.0), glyph, fs(12.0), crate::ui::fg2(pal), true);
    }
    if show_title {
        crate::ui::title(v, x + p + s(17.0), y + s(9.0), "Thermal zones", fs(11.5), pal.fg);
    }
    if !meta.is_empty() {
        crate::ui::caption_r(v, x + w - p, y + s(11.0), meta, fs(8.5), crate::ui::fg3(pal), false);
    }
    if empty {
        crate::ui::text_c(
            v,
            x + w / 2.0,
            y + h / 2.0,
            "no /sys/class/thermal zones",
            fs(9.0),
            crate::ui::fg3(pal),
            false,
        );
        return Vec::new();
    }
    let mut hits = Vec::new();
    if live {
        let cell_w = (w as f64 - 4.0 * grid as f64) / 3.0;
        let cell_h = 24.0 * grid as f64;
        let gap = 2.0 * grid as f64;
        let grid_y = y as f64 + 26.0 * grid as f64;
        let h_lim = y as f64 + h as f64 + 0.5;
        for (i, (label, sub, sub_color)) in cells.iter().enumerate() {
            let col = (i % 3) as f64;
            let row = (i / 3) as f64;
            let cx = x as f64 + col * (cell_w + gap);
            let cy = grid_y + row * (cell_h + gap);
            if cy > h_lim {
                break;
            }
            v.push(Cmd::Rect {
                x: cx as f32,
                y: cy as f32,
                w: cell_w as f32,
                h: cell_h as f32,
                r: (5.0 * grid as f64) as f32,
                color: crate::ui::hover(pal),
            });
            crate::ui::caption_c(
                v,
                (cx + cell_w / 2.0) as f32,
                (cy + 3.0 * grid as f64) as f32,
                label,
                fs(7.5),
                pal.fg,
                false,
            );
            crate::ui::caption_c(
                v,
                (cx + cell_w / 2.0) as f32,
                (cy + 12.0 * grid as f64) as f32,
                sub,
                fs(9.0),
                *sub_color,
                false,
            );
            hits.push((cx as f32, cy as f32, cell_w as f32, cell_h as f32, i as u32));
        }
    }
    hits
}

/// The Fans card — rpm rows with per-fan meter bars over a hover track.
/// Widget port; oracle `fans_native`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn fans(
    show_glyph: bool,
    show_title: bool,
    glyph: &str,
    meta: &str,
    rows: &[(String, String, f32, u32, u32)],
    empty: bool,
    live: bool,
    w: f32,
    h: f32,
    grid: f32,
    pal: &Pal,
) -> Node {
    let s = |n: f32| n * grid;
    let fs = |n: f32| (n * grid * 10.0).round() / 10.0;
    let p = s(12.0);
    let mut root = Node::box_(0.0, 0);
    if show_glyph {
        root = root.child(
            Node::glyph(glyph, fs(12.0), crate::ui::fg2(pal))
                .halign(HAlign::Left)
                .valign(VAlign::Top)
                .margin(p, s(8.0), 0.0, 0.0),
        );
    }
    if show_title {
        let tx = if show_glyph { p + s(17.0) } else { p };
        root = root.child(
            Node::label("Fans", fs(11.5), pal.fg)
                .font(Ff::Ui, Fw::Semibold)
                .halign(HAlign::Left)
                .valign(VAlign::Top)
                .margin(tx, s(9.0), 0.0, 0.0),
        );
    }
    if !meta.is_empty() {
        root = root.child(
            Node::label(meta, fs(8.5), crate::ui::fg3(pal))
                .font(Ff::Ui, Fw::Regular)
                .halign(HAlign::Right)
                .valign(VAlign::Top)
                .margin(0.0, s(11.0), p, 0.0),
        );
    }
    if empty {
        root = root.child(
            Node::anchored(0.0, 0.0)
                .text_("no hwmon fan sensors")
                .size_(fs(9.0))
                .color_(crate::ui::fg3(pal)),
        );
    }
    if live {
        let row_h = s(24.0);
        let pitch = s(24.0);
        let row_w = w - p * 2.0;
        let avail = w - p * 2.0 - s(80.0);
        let gp = 0.62 * fs(8.5);
        for (i, (label, rpm, frac, lcolor, rcolor)) in rows.iter().enumerate() {
            let ry = s(28.0) + i as f32 * pitch;
            if ry + row_h <= h + 0.5 {
                let l = trunc_at(label, avail, gp);
                root = root.child(
                    Node::label(l, fs(8.5), *lcolor)
                        .font(Ff::Ui, Fw::Regular)
                        .halign(HAlign::Left)
                        .valign(VAlign::Top)
                        .margin(p, ry, 0.0, 0.0),
                );
                root = root.child(
                    Node::label(rpm.as_str(), fs(8.5), *rcolor)
                        .font(Ff::Mono, Fw::Regular)
                        .halign(HAlign::Right)
                        .valign(VAlign::Top)
                        .margin(0.0, ry, s(12.0), 0.0),
                );
                root = root.child(
                    Node::bar(*frac, 1.0, pal.fg, crate::ui::hover(pal), s(1.5), false).margin(
                        p,
                        ry + s(13.0),
                        w - p - row_w,
                        h - (ry + s(13.0)) - s(3.0),
                    ),
                );
            }
        }
    }
    root
}

/// The retired Fans drawer — the byte-parity oracle for `fans`.
#[cfg(test)]
#[allow(clippy::too_many_arguments)]
fn fans_native(
    v: &mut Vec<Cmd>,
    show_glyph: bool,
    show_title: bool,
    glyph: &str,
    meta: &str,
    rows: &[(String, String, f32, u32, u32)],
    empty: bool,
    live: bool,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    grid: f32,
    pal: &Pal,
) {
    let s = |n: f32| n * grid;
    let fs = |n: f32| (n * grid * 10.0).round() / 10.0;
    let p = s(12.0);
    if show_glyph {
        crate::ui::text(v, x + p, y + s(8.0), glyph, fs(12.0), crate::ui::fg2(pal), true);
    }
    if show_title {
        let tx = if show_glyph { p + s(17.0) } else { p };
        crate::ui::title(v, x + tx, y + s(9.0), "Fans", fs(11.5), pal.fg);
    }
    if !meta.is_empty() {
        crate::ui::caption_r(v, x + w - p, y + s(11.0), meta, fs(8.5), crate::ui::fg3(pal), false);
    }
    if empty {
        crate::ui::text_c(v, x + w / 2.0, y + h / 2.0, "no hwmon fan sensors", fs(9.0), crate::ui::fg3(pal), false);
    }
    if live {
        let row_h = s(24.0);
        let pitch = s(24.0);
        let row_w = w - p * 2.0;
        let avail = w - p * 2.0 - s(80.0);
        let gp = 0.62 * fs(8.5);
        for (i, (label, rpm, frac, lcolor, rcolor)) in rows.iter().enumerate() {
            let ry = y + s(28.0) + i as f32 * pitch;
            if ry + row_h <= y + h + 0.5 {
                let l = trunc_at(label, avail, gp);
                crate::ui::caption(v, x + p, ry, &l, fs(8.5), *lcolor, false);
                crate::ui::text_r_mono(v, x + w - s(12.0), ry, rpm, fs(8.5), *rcolor);
                v.push(Cmd::Rect {
                    x: x + p,
                    y: ry + s(13.0),
                    w: row_w,
                    h: s(3.0),
                    r: s(1.5),
                    color: crate::ui::hover(pal),
                });
                let fw = row_w * frac;
                if fw > 0.5 {
                    v.push(Cmd::Rect {
                        x: x + p,
                        y: ry + s(13.0),
                        w: fw,
                        h: s(3.0),
                        r: s(1.5),
                        color: pal.fg,
                    });
                }
            }
        }
    }
}

/// The Disk I/O card — legend dots + the dual normalized history traces.
#[allow(clippy::too_many_arguments)]
pub(crate) fn diskio(
    show_title: bool,
    r_data: &[f32],
    w_data: &[f32],
    w: f32,
    h: f32,
    grid: f32,
    pal: &Pal,
) -> Node {
    let s = |n: f32| n * grid;
    let fs = |n: f32| (n * grid * 10.0).round() / 10.0;
    let mut root = Node::box_(0.0, 0);
    root = root.child(
        Node::box_(s(2.0), crate::ui::INFO)
            .margin(w - s(58.0), s(16.0), s(58.0) - s(7.0), h - s(16.0) - s(7.0)),
    );
    root = root.child(
        Node::label("R", fs(9.0), pal.fg)
            .halign(HAlign::Right)
            .valign(VAlign::Top)
            .margin(0.0, s(14.0), s(14.0), 0.0),
    );
    root = root.child(
        Node::box_(s(2.0), pal.acc)
            .margin(w - s(58.0), s(34.0), s(58.0) - s(7.0), h - s(34.0) - s(7.0)),
    );
    root = root.child(
        Node::label("W", fs(9.0), pal.fg)
            .halign(HAlign::Right)
            .valign(VAlign::Top)
            .margin(0.0, s(32.0), s(14.0), 0.0),
    );
    if show_title {
        root = root.child(
            Node::label("Disk I/O", fs(11.5), pal.fg)
                .font(Ff::Ui, Fw::Semibold)
                .halign(HAlign::Left)
                .valign(VAlign::Top)
                .margin(s(12.0), s(9.0), 0.0, 0.0),
        );
    }
    if h >= s(78.0) {
        let mut cv: Vec<Cmd> = Vec::new();
        spark(
            &mut cv, s(14.0), s(46.0), w - s(28.0), h - s(58.0), r_data, crate::ui::INFO, s(2.0),
            None,
        );
        spark(
            &mut cv, s(14.0), s(46.0), w - s(28.0), h - s(58.0), w_data, pal.acc, s(2.0), None,
        );
        root = root.child(Node::stamp(cv));
    }
    root
}

/// The retired Disk I/O drawer — the byte-parity oracle for `diskio`.
#[cfg(test)]
#[allow(clippy::too_many_arguments)]
fn diskio_native(
    v: &mut Vec<Cmd>,
    show_title: bool,
    r_data: &[f32],
    w_data: &[f32],
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    grid: f32,
    pal: &Pal,
) {
    let s = |n: f32| n * grid;
    let fs = |n: f32| (n * grid * 10.0).round() / 10.0;
    v.push(Cmd::Rect {
        x: x + w - s(58.0),
        y: y + s(16.0),
        w: s(7.0),
        h: s(7.0),
        r: s(2.0),
        color: crate::ui::INFO,
    });
    crate::ui::text_r(v, x + w - s(14.0), y + s(14.0), "R", fs(9.0), pal.fg, false);
    v.push(Cmd::Rect {
        x: x + w - s(58.0),
        y: y + s(34.0),
        w: s(7.0),
        h: s(7.0),
        r: s(2.0),
        color: pal.acc,
    });
    crate::ui::text_r(v, x + w - s(14.0), y + s(32.0), "W", fs(9.0), pal.fg, false);

    if show_title {
        crate::ui::title(v, x + s(12.0), y + s(9.0), "Disk I/O", fs(11.5), pal.fg);
    }
    if h >= s(78.0) {
        let sx = x + s(14.0);
        let sy = y + s(46.0);
        let sw = w - s(28.0);
        let sh = h - s(58.0);
        spark(v, sx, sy, sw, sh, r_data, crate::ui::INFO, s(2.0), None);
        spark(v, sx, sy, sw, sh, w_data, pal.acc, s(2.0), None);
    }
}

/// The Network card — dual traces + the centered down/up speed legend row.
#[allow(clippy::too_many_arguments)]
pub(crate) fn network(
    show_title: bool,
    meta: &str,
    net_down: &str,
    net_up: &str,
    down_data: &[f32],
    up_data: &[f32],
    w: f32,
    h: f32,
    grid: f32,
    pal: &Pal,
) -> Node {
    let s = |n: f32| n * grid;
    let fs = |n: f32| (n * grid * 10.0).round() / 10.0;
    let p = s(12.0);
    let mut root = Node::box_(0.0, 0);
    if show_title {
        root = root.child(
            Node::label("Network", fs(11.5), pal.fg)
                .font(Ff::Ui, Fw::Semibold)
                .halign(HAlign::Left)
                .valign(VAlign::Top)
                .margin(p, s(9.0), 0.0, 0.0),
        );
    }
    if !meta.is_empty() {
        root = root.child(
            Node::label(meta, fs(8.5), crate::ui::fg3(pal))
                .font(Ff::Mono, Fw::Regular)
                .halign(HAlign::Right)
                .valign(VAlign::Top)
                .margin(0.0, s(11.0), p, 0.0),
        );
    }
    if h >= s(59.0) {
        let mut cv: Vec<Cmd> = Vec::new();
        spark(
            &mut cv, s(12.0), s(28.0), w - s(24.0), h - s(49.0), down_data, crate::ui::INFO,
            s(2.0), None,
        );
        spark(
            &mut cv, s(12.0), s(28.0), w - s(24.0), h - s(49.0), up_data, pal.acc, s(2.0), None,
        );
        root = root.child(Node::stamp(cv));
    }
    let glyph_y = h - s(20.0) + s(12.0) + s(20.0) / 2.0;
    let c0 = s(12.0);
    let c1 = s(12.0) + s(79.0) + s(26.0);
    root = root.child(
        Node::anchored(c0 + s(9.0) / 2.0 - w / 2.0, glyph_y - h / 2.0)
            .text_("\u{f063}")
            .size_(fs(9.0))
            .color_(crate::ui::INFO)
            .icon(),
    );
    root = root.child(
        Node::label(net_down, fs(10.0), crate::ui::INFO)
            .font(Ff::Mono, Fw::Regular)
            .halign(HAlign::Left)
            .valign(VAlign::Top)
            .margin(c0 + s(11.0), glyph_y, 0.0, 0.0),
    );
    root = root.child(
        Node::anchored(c1 + s(9.0) / 2.0 - w / 2.0, glyph_y - h / 2.0)
            .text_("\u{f062}")
            .size_(fs(9.0))
            .color_(pal.acc)
            .icon(),
    );
    root = root.child(
        Node::label(net_up, fs(10.0), pal.acc)
            .font(Ff::Mono, Fw::Regular)
            .halign(HAlign::Left)
            .valign(VAlign::Top)
            .margin(c1 + s(11.0), glyph_y, 0.0, 0.0),
    );
    root
}

/// The retired Network drawer — the byte-parity oracle for `network`.
#[cfg(test)]
#[allow(clippy::too_many_arguments)]
fn network_native(
    v: &mut Vec<Cmd>,
    show_title: bool,
    meta: &str,
    net_down: &str,
    net_up: &str,
    down_data: &[f32],
    up_data: &[f32],
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    grid: f32,
    pal: &Pal,
) {
    let s = |n: f32| n * grid;
    let fs = |n: f32| (n * grid * 10.0).round() / 10.0;
    let p = s(12.0);
    if show_title {
        crate::ui::title(v, x + p, y + s(9.0), "Network", fs(11.5), pal.fg);
    }
    if !meta.is_empty() {
        crate::ui::text_r_mono(v, x + w - p, y + s(11.0), meta, fs(8.5), crate::ui::fg3(pal));
    }
    if h >= s(59.0) {
        let sx = x + s(12.0);
        let sy = y + s(28.0);
        let sw = w - s(24.0);
        let sh = h - s(49.0);
        spark(v, sx, sy, sw, sh, down_data, crate::ui::INFO, s(2.0), None);
        spark(v, sx, sy, sw, sh, up_data, pal.acc, s(2.0), None);
    }
    let cy = y + h - s(20.0) + s(12.0);
    let glyph_y = cy + s(20.0) / 2.0;
    let mut cx = x + s(12.0);
    crate::ui::text_c(v, cx + s(9.0) / 2.0, glyph_y, "\u{f063}", fs(9.0), crate::ui::INFO, true);
    crate::ui::text_mono(v, cx + s(11.0), glyph_y, net_down, fs(10.0), crate::ui::INFO);
    cx += s(79.0) + s(26.0);
    crate::ui::text_c(v, cx + s(9.0) / 2.0, glyph_y, "\u{f062}", fs(9.0), pal.acc, true);
    crate::ui::text_mono(v, cx + s(11.0), glyph_y, net_up, fs(10.0), pal.acc);
}

/// The Power draw card — watts legend + the two traces + the idle midline.
#[allow(clippy::too_many_arguments)]
pub(crate) fn powerdraw(
    show_glyph: bool,
    show_title: bool,
    pw_b: &str,
    pw_g: &str,
    b_data: &[f32],
    g_data: &[f32],
    live: bool,
    idle: bool,
    empty: bool,
    w: f32,
    h: f32,
    grid: f32,
    pal: &Pal,
) -> Node {
    let s = |n: f32| n * grid;
    let fs = |n: f32| (n * grid * 10.0).round() / 10.0;
    let p = s(12.0);
    let mut root = Node::box_(0.0, 0);
    let mut tx = p;
    if show_glyph {
        root = root.child(
            Node::glyph("\u{f0e7}", fs(12.0), crate::ui::fg2(pal))
                .halign(HAlign::Left)
                .valign(VAlign::Top)
                .margin(p, s(8.0), 0.0, 0.0),
        );
        tx = p + s(17.0);
    }
    if show_title {
        root = root.child(
            Node::label("Power", fs(11.5), pal.fg)
                .font(Ff::Ui, Fw::Semibold)
                .halign(HAlign::Left)
                .valign(VAlign::Top)
                .margin(tx, s(9.0), 0.0, 0.0),
        );
    }
    if live {
        root = root.child(
            Node::label(pw_b, fs(10.0), pal.fg)
                .halign(HAlign::Right)
                .valign(VAlign::Top)
                .margin(0.0, s(32.0), p, 0.0),
        );
        root = root.child(
            Node::label(pw_g, fs(10.0), pal.fg)
                .halign(HAlign::Right)
                .valign(VAlign::Top)
                .margin(0.0, s(50.0), p, 0.0),
        );
        root = root.child(
            Node::box_(s(2.0), pal.acc)
                .margin(w - s(88.0), s(34.0), s(88.0) - s(7.0), h - s(34.0) - s(7.0)),
        );
        root = root.child(
            Node::box_(s(2.0), crate::ui::INFO)
                .margin(w - s(88.0), s(52.0), s(88.0) - s(7.0), h - s(52.0) - s(7.0)),
        );
    }
    let mut plot_w = w - s(108.0);
    if plot_w < s(20.0) {
        plot_w = s(20.0);
    }
    let mut plot_h = h - s(42.0);
    if plot_h < s(10.0) {
        plot_h = s(10.0);
    }
    let (plot_x, plot_y) = (s(14.0), s(30.0));
    let mut cv: Vec<Cmd> = Vec::new();
    if idle {
        let yb = plot_y + (plot_h as f64 * 0.5) as f32;
        cv.push(Cmd::Line {
            x0: plot_x,
            y0: yb,
            x1: plot_x + plot_w,
            y1: yb,
            w: s(1.0),
            color: crate::ui::fg3(pal),
        });
    }
    if !b_data.is_empty() {
        spark(&mut cv, plot_x, plot_y, plot_w, plot_h, b_data, pal.acc, s(2.0), Some(1.0));
    }
    if !g_data.is_empty() {
        spark(&mut cv, plot_x, plot_y, plot_w, plot_h, g_data, crate::ui::INFO, s(2.0), Some(1.0));
    }
    root = root.child(Node::stamp(cv));
    if empty {
        root = root.child(
            Node::anchored(0.0, -s(6.0))
                .text_("no power sensors")
                .size_(fs(9.0))
                .color_(crate::ui::fg3(pal))
                .font(Ff::Ui, Fw::Regular),
        );
    }
    root
}

/// The retired Power draw drawer — the byte-parity oracle for `powerdraw`.
#[cfg(test)]
#[allow(clippy::too_many_arguments)]
fn powerdraw_native(
    v: &mut Vec<Cmd>,
    show_glyph: bool,
    show_title: bool,
    pw_b: &str,
    pw_g: &str,
    b_data: &[f32],
    g_data: &[f32],
    live: bool,
    idle: bool,
    empty: bool,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    grid: f32,
    pal: &Pal,
) {
    let s = |n: f32| n * grid;
    let fs = |n: f32| (n * grid * 10.0).round() / 10.0;
    let p = s(12.0);
    let mut tx = p;
    if show_glyph {
        crate::ui::text(v, x + p, y + s(8.0), "\u{f0e7}", fs(12.0), crate::ui::fg2(pal), true);
        tx = p + s(17.0);
    }
    if show_title {
        crate::ui::title(v, x + tx, y + s(9.0), "Power", fs(11.5), pal.fg);
    }
    if live {
        crate::ui::text_r(v, x + w - p, y + s(32.0), pw_b, fs(10.0), pal.fg, false);
        crate::ui::text_r(v, x + w - p, y + s(50.0), pw_g, fs(10.0), pal.fg, false);
        v.push(Cmd::Rect {
            x: x + w - s(88.0),
            y: y + s(34.0),
            w: s(7.0),
            h: s(7.0),
            r: s(2.0),
            color: pal.acc,
        });
        v.push(Cmd::Rect {
            x: x + w - s(88.0),
            y: y + s(52.0),
            w: s(7.0),
            h: s(7.0),
            r: s(2.0),
            color: crate::ui::INFO,
        });
    }
    let mut plot_w = w - s(108.0);
    if plot_w < s(20.0) {
        plot_w = s(20.0);
    }
    let mut plot_h = h - s(42.0);
    if plot_h < s(10.0) {
        plot_h = s(10.0);
    }
    let (plot_x, plot_y) = (x + s(14.0), y + s(30.0));
    if idle {
        let yb = plot_y + (plot_h as f64 * 0.5) as f32; // module: plot_h * 0.5 in f64
        v.push(Cmd::Line {
            x0: plot_x,
            y0: yb,
            x1: plot_x + plot_w,
            y1: yb,
            w: s(1.0),
            color: crate::ui::fg3(pal),
        });
    }
    if !b_data.is_empty() {
        spark(v, plot_x, plot_y, plot_w, plot_h, b_data, pal.acc, s(2.0), Some(1.0));
    }
    if !g_data.is_empty() {
        spark(v, plot_x, plot_y, plot_w, plot_h, g_data, crate::ui::INFO, s(2.0), Some(1.0));
    }
    if empty {
        crate::ui::caption_c(v, x + w / 2.0, y + h / 2.0 - s(6.0), "no power sensors", fs(9.0), crate::ui::fg3(pal), false);
    }
}

/// The CPU/GPU card — top-right legend on a wide card plus the dual 0..1
/// traces (GPU under CPU), the plot box gated on card size.
#[allow(clippy::too_many_arguments)]
pub(crate) fn cpugpu(
    cpu_txt: &str,
    gpu_txt: &str,
    cpu_ink: u32,
    gpu_ink: u32,
    cpu_data: &[f32],
    gpu_data: &[f32],
    gpu_live: bool,
    w: f32,
    h: f32,
    grid: f32,
    pal: &Pal,
) -> Node {
    let s = |n: f32| n * grid;
    let fs = |n: f32| (n * grid * 10.0).round() / 10.0;
    let mut root = Node::box_(0.0, 0);
    if w >= s(170.0) {
        let rx = w - s(14.0);
        root = root.child(
            Node::box_(s(2.0), cpu_ink).margin(
                rx - s(52.0),
                s(18.0),
                s(14.0) + s(52.0) - s(8.0),
                h - s(18.0) - s(8.0),
            ),
        );
        root = root.child(
            Node::label(cpu_txt, fs(10.0), pal.fg)
                .halign(HAlign::Right)
                .valign(VAlign::Top)
                .margin(0.0, s(15.0), s(14.0), 0.0),
        );
        if gpu_live {
            root = root.child(
                Node::box_(s(2.0), gpu_ink).margin(
                    rx - s(52.0),
                    s(39.0),
                    s(14.0) + s(52.0) - s(8.0),
                    h - s(39.0) - s(8.0),
                ),
            );
        }
        root = root.child(
            Node::label(gpu_txt, fs(10.0), pal.fg)
                .halign(HAlign::Right)
                .valign(VAlign::Top)
                .margin(0.0, s(36.0), s(14.0), 0.0),
        );
    }
    let plot_y: Option<f32> = if w >= s(170.0) {
        if h >= s(90.0) {
            Some(s(52.0))
        } else {
            None
        }
    } else if h >= s(50.0) {
        Some(s(12.0))
    } else {
        None
    };
    let mut cv: Vec<Cmd> = Vec::new();
    if let Some(plot_y) = plot_y {
        let plot_x = s(14.0);
        let plot_w = w - s(28.0);
        let plot_h = h - plot_y - s(14.0);
        if !gpu_data.is_empty() {
            spark(&mut cv, plot_x, plot_y, plot_w, plot_h, gpu_data, gpu_ink, s(2.0), Some(1.0));
        }
        if !cpu_data.is_empty() {
            spark(&mut cv, plot_x, plot_y, plot_w, plot_h, cpu_data, cpu_ink, s(2.0), Some(1.0));
        }
    }
    root.child(Node::stamp(cv))
}

/// The retired CPU/GPU drawer — the byte-parity oracle for `cpugpu`.
#[cfg(test)]
#[allow(clippy::too_many_arguments)]
fn cpugpu_native(
    v: &mut Vec<Cmd>,
    cpu_txt: &str,
    gpu_txt: &str,
    cpu_ink: u32,
    gpu_ink: u32,
    cpu_data: &[f32],
    gpu_data: &[f32],
    gpu_live: bool,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    grid: f32,
    pal: &Pal,
) {
    let s = |n: f32| n * grid;
    let fs = |n: f32| (n * grid * 10.0).round() / 10.0;
    if w >= s(170.0) {
        let rx = x + w - s(14.0);
        v.push(Cmd::Rect {
            x: (rx - s(52.0)) as f32,
            y: y + s(18.0),
            w: s(8.0),
            h: s(8.0),
            r: s(2.0),
            color: cpu_ink,
        });
        crate::ui::text_r(v, rx, y + s(15.0), cpu_txt, fs(10.0), pal.fg, false);
        if gpu_live {
            v.push(Cmd::Rect {
                x: (rx - s(52.0)) as f32,
                y: y + s(39.0),
                w: s(8.0),
                h: s(8.0),
                r: s(2.0),
                color: gpu_ink,
            });
        }
        crate::ui::text_r(v, rx, y + s(36.0), gpu_txt, fs(10.0), pal.fg, false);
    }
    // the plot box: wide card → y+52 (needs h≥90); narrow → y+12 (needs h≥50)
    let plot_y: Option<f32> = if w >= s(170.0) {
        if h >= s(90.0) {
            Some(y + s(52.0))
        } else {
            None
        }
    } else if h >= s(50.0) {
        Some(y + s(12.0))
    } else {
        None
    };
    if let Some(plot_y) = plot_y {
        let plot_x = x + s(14.0);
        let plot_w = w - s(28.0);
        let plot_h = y + h - plot_y - s(14.0);
        if !gpu_data.is_empty() {
            spark(v, plot_x, plot_y, plot_w, plot_h, gpu_data, gpu_ink, s(2.0), Some(1.0));
        }
        if !cpu_data.is_empty() {
            spark(v, plot_x, plot_y, plot_w, plot_h, cpu_data, cpu_ink, s(2.0), Some(1.0));
        }
    }
}

/// The Latency card — probe trace (or idle midline), the ms hero readout and
/// the rerun pill (key 31970, Rust click arm applies the re-probe).
#[allow(clippy::too_many_arguments)]
pub(crate) fn latency(
    show_glyph: bool,
    show_title: bool,
    glyph: &str,
    meta: &str,
    rerun_glyph: &str,
    val: &str,
    val_ink: u32,
    status: &str,
    series: &[f32],
    hover: u32,
    w: f32,
    h: f32,
    grid: f32,
    pal: &Pal,
) -> Node {
    let s = |n: f32| n * grid;
    let fs = |n: f32| (n * grid * 10.0).round() / 10.0;
    let p = s(12.0);
    let fg3 = crate::ui::fg3(pal);
    let mut root = Node::box_(0.0, 0);
    if show_glyph {
        root = root.child(
            Node::glyph(glyph, fs(12.0), crate::ui::fg2(pal))
                .halign(HAlign::Left)
                .valign(VAlign::Top)
                .margin(p, s(8.0), 0.0, 0.0),
        );
    }
    if show_title {
        root = root.child(
            Node::label("Latency", fs(11.5), pal.fg)
                .font(Ff::Ui, Fw::Semibold)
                .halign(HAlign::Left)
                .valign(VAlign::Top)
                .margin(p + s(17.0), s(9.0), 0.0, 0.0),
        );
    }
    if !meta.is_empty() {
        root = root.child(
            Node::label(meta, fs(8.5), fg3)
                .font(Ff::Mono, Fw::Regular)
                .halign(HAlign::Right)
                .valign(VAlign::Top)
                .margin(0.0, s(11.0), p, 0.0),
        );
    }
    let plot_x = p;
    let plot_y = s(32.0);
    let plot_w = w - p * 2.0 - s(58.0);
    let plot_h = h - s(32.0) - s(12.0);
    let mid = plot_y + (plot_h as f64 * 0.5) as f32;
    let mut cv: Vec<Cmd> = Vec::new();
    if series.is_empty() {
        cv.push(Cmd::Line {
            x0: plot_x,
            y0: mid,
            x1: plot_x + plot_w,
            y1: mid,
            w: s(1.0),
            color: fg3,
        });
    } else {
        spark(&mut cv, plot_x, plot_y, plot_w, plot_h, series, crate::ui::INFO, s(2.0), Some(1.0));
    }
    root = root.child(Node::stamp(cv));
    root = root.child(
        Node::label(val, fs(20.0), val_ink)
            .font(Ff::Display, Fw::Medium)
            .halign(HAlign::Right)
            .valign(VAlign::Top)
            .margin(0.0, mid + s(7.0), p, 0.0),
    );
    root = root.child(
        Node::label("ms", fs(7.5), fg3)
            .font(Ff::Ui, Fw::Regular)
            .halign(HAlign::Right)
            .valign(VAlign::Top)
            .margin(0.0, mid - s(7.0), p, 0.0),
    );
    let bh = s(20.0);
    let by = h - bh - s(8.0);
    root = root.child(
        Node::label(status, fs(8.5), crate::ui::fg2(pal))
            .font(Ff::Ui, Fw::Regular)
            .halign(HAlign::Left)
            .valign(VAlign::Top)
            .margin(p, by + s(5.0), 0.0, 0.0),
    );
    let hov = hover == 31970;
    let bx = w - p - s(30.0);
    root = root.child(
        Node::box_(
            s(6.0),
            if hov { crate::ui::raised_hl(pal) } else { crate::ui::raised(pal) },
        )
        .margin(bx, by, w - bx - s(30.0), h - by - bh),
    );
    root = root.child(
        Node::anchored(bx + s(15.0) - w / 2.0, by + s(5.5) - h / 2.0)
            .text_(rerun_glyph)
            .size_(fs(9.0))
            .color_(if hov { pal.acc } else { fg3 })
            .icon(),
    );
    root = root.child(Node::hit(31970).margin(bx, by, w - bx - s(30.0), h - by - bh));
    root = root.child(
        Node::hit(31970).margin(bx - s(3.0), by - s(3.0), w - (bx - s(3.0)) - s(36.0), h - (by - s(3.0)) - s(26.0)),
    );
    root
}

/// The retired Latency drawer — the byte-parity oracle for `latency`.
#[cfg(test)]
#[allow(clippy::too_many_arguments)]
fn latency_native(
    v: &mut Vec<Cmd>,
    show_glyph: bool,
    show_title: bool,
    glyph: &str,
    meta: &str,
    rerun_glyph: &str,
    val: &str,
    val_ink: u32,
    status: &str,
    series: &[f32],
    hover: u32,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    grid: f32,
    pal: &Pal,
) -> Vec<CardHit> {
    let s = |n: f32| n * grid;
    let fs = |n: f32| (n * grid * 10.0).round() / 10.0;
    let p = s(12.0);
    if show_glyph {
        crate::ui::text(v, x + p, y + s(8.0), glyph, fs(12.0), crate::ui::fg2(pal), true);
    }
    if show_title {
        crate::ui::title(v, x + p + s(17.0), y + s(9.0), "Latency", fs(11.5), pal.fg);
    }
    if !meta.is_empty() {
        crate::ui::text_r_mono(v, x + w - p, y + s(11.0), meta, fs(8.5), crate::ui::fg3(pal));
    }
    let plot_x = x + p;
    let plot_y = y + s(32.0);
    let plot_w = w - p * 2.0 - s(58.0);
    let plot_h = h - s(32.0) - s(12.0);
    let mid = plot_y + (plot_h as f64 * 0.5) as f32;

    if series.is_empty() {
        v.push(Cmd::Line {
            x0: plot_x,
            y0: mid,
            x1: plot_x + plot_w,
            y1: mid,
            w: s(1.0),
            color: crate::ui::fg3(pal),
        });
    } else {
        spark(v, plot_x, plot_y, plot_w, plot_h, series, crate::ui::INFO, s(2.0), Some(1.0));
    }
    crate::ui::text_r_hero(v, x + w - p, mid + s(7.0), val, fs(20.0), val_ink);
    crate::ui::caption_r(v, x + w - p, mid - s(7.0), "ms", fs(7.5), crate::ui::fg3(pal), false);

    let bh = s(20.0);
    let by = y + h - bh - s(8.0);
    crate::ui::caption(v, x + p, by + s(5.0), status, fs(8.5), crate::ui::fg2(pal), false);

    let hov = hover == 31970;
    let bx = x + w - p - s(30.0);
    v.push(Cmd::Rect {
        x: bx,
        y: by,
        w: s(30.0),
        h: bh,
        r: s(6.0),
        color: if hov { crate::ui::raised_hl(pal) } else { crate::ui::raised(pal) },
    });
    crate::ui::text_c(
        v,
        bx + s(15.0),
        by + s(5.5),
        rerun_glyph,
        fs(9.0),
        if hov { pal.acc } else { crate::ui::fg3(pal) },
        true,
    );
    vec![
        (bx, by, s(30.0), bh, 31970),
        (bx - s(3.0), by - s(3.0), s(36.0), s(26.0), 31970),
    ]
}

/// One pane dot pair for the battery cards (the retired modules' `hover_dots`
/// block) — shared by batteryh / batteryv.
fn battery_dots(
    v: &mut Vec<Cmd>,
    pane: usize,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    grid: f32,
    pal: &Pal,
) {
    // the module's dot block runs in Lua f64 (step·0.66 is not exact in f32);
    // keep the same double-width math and coerce per field
    let step = 7.0 * grid as f64;
    let dot = step * 0.66;
    let total = 2.0 * step;
    let dx = x as f64 + w as f64 / 2.0 - total / 2.0;
    let dyy = y as f64 + h as f64 - 3.5 * grid as f64 - dot;
    for i in 0..2_u32 {
        let c = if i as usize == pane { pal.acc } else { crate::ui::hover(pal) };
        let px = dx + i as f64 * step;
        v.push(Cmd::Rect {
            x: px as f32,
            y: dyy as f32,
            w: dot as f32,
            h: dot as f32,
            r: (dot / 2.0) as f32,
            color: c,
        });
    }
}

/// One battery-card metric row (pane 1, both orientations).
fn battery_rows_pane(
    v: &mut Vec<Cmd>,
    rows: &[(String, String)],
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    grid: f32,
    pal: &Pal,
) {
    let s = |n: f32| n * grid;
    let fs = |n: f32| (n * grid * 10.0).round() / 10.0;
    let bottom_line = y + h - s(26.0);
    let mut ry = y + s(30.0);
    for i in 0..4_usize {
        if ry + s(18.0) > y + h + 0.5 {
            break;
        }
        if ry + s(18.0) > bottom_line {
            break;
        }
        if let Some((label, value)) = rows.get(i) {
            crate::ui::caption(v, x + s(12.0), ry, label, fs(9.0), crate::ui::fg3(pal), false);
            crate::ui::caption(v, x + w - s(38.0), ry, value, fs(9.5), pal.fg, false);
        }
        ry += s(18.0);
    }
}

/// The Battery card — one painter for BOTH orientations (`horizontal` is the
/// batteryh module's card-fit side-laid gauge, else the batteryv upright one);
/// pane 0 = gauge + percent, pane 1 = metric rows + the power-save pill.
#[allow(clippy::too_many_arguments)]
pub(crate) fn battery(
    show_glyph: bool,
    show_title: bool,
    horizontal: bool,
    pct: i32,
    pct_txt: &str,
    ink: u32,
    pane: usize,
    present: bool,
    hover_dots: bool,
    rows: &[(String, String)],
    psave_bg: u32,
    psave_ink: u32,
    psave_lbl: &str,
    w: f32,
    h: f32,
    grid: f32,
    pal: &Pal,
) -> Node {
    let s = |n: f32| n * grid;
    let fs = |n: f32| (n * grid * 10.0).round() / 10.0;
    let p = s(12.0);
    let mut root = Node::box_(0.0, 0);
    let mut tx = p;
    if show_glyph {
        root = root.child(
            Node::glyph("\u{f240}", fs(12.0), crate::ui::fg2(pal))
                .halign(HAlign::Left)
                .valign(VAlign::Top)
                .margin(p, s(8.0), 0.0, 0.0),
        );
        tx = p + s(17.0);
    }
    if show_title {
        root = root.child(
            Node::label("Battery", fs(11.5), pal.fg)
                .font(Ff::Ui, Fw::Semibold)
                .halign(HAlign::Left)
                .valign(VAlign::Top)
                .margin(tx, s(9.0), 0.0, 0.0),
        );
    }

    let pane0 = present && pane == 0;
    let pane1 = present && pane == 1;

    if pane0 {
        if horizontal {
            let bw = ((w as f64 * 0.46).clamp(40.0, 220.0)) as f32;
            let bh = ((h as f64 * 0.36).clamp(16.0, 60.0)) as f32;
            let cx = w / 2.0;
            let cy = s(42.0) + (h - s(70.0)) / 2.0;
            let mut buf: Vec<Cmd> = Vec::new();
            crate::scene::push_battery_icon_parts(
                &mut buf,
                crate::ui::fg2(pal),
                crate::ui::hover(pal),
                ink,
                pct,
                cx,
                cy,
                bw,
                bh,
                false,
            );
            root = root.child(Node::stamp(buf));
            root = root.child(
                Node::anchored(0.0, h - s(22.0) - h / 2.0)
                    .text_(pct_txt)
                    .size_(fs(12.0))
                    .color_(ink),
            );
        } else {
            let aw = (w as f64 * 0.30).clamp(16.0, 70.0);
            let ah = (h as f64 - 44.0 * grid as f64).max(aw * 1.6);
            let iw = (aw * 2.0 / 3.0).max(12.0);
            let cx = w / 2.0;
            let cy = 38.0 * grid as f64 + (ah - iw * 1.7) / 2.0;
            let mut buf: Vec<Cmd> = Vec::new();
            crate::scene::push_battery_icon_parts(
                &mut buf,
                crate::ui::fg2(pal),
                crate::ui::hover(pal),
                ink,
                pct,
                cx,
                cy as f32,
                iw as f32,
                (iw * 1.7) as f32,
                true,
            );
            root = root.child(Node::stamp(buf));
            root = root.child(
                Node::anchored(0.0, h - s(24.0) - h / 2.0)
                    .text_(pct_txt)
                    .size_(fs(11.0))
                    .color_(ink),
            );
        }
    }

    if pane1 {
        let mut buf: Vec<Cmd> = Vec::new();
        battery_rows_pane(&mut buf, rows, 0.0, 0.0, w, h, grid, pal);
        root = root.child(Node::stamp(buf));
        root = root.child(
            Node::box_(s(11.0), psave_bg)
                .margin(s(12.0), h - s(24.0), s(38.0), s(2.0)),
        );
        root = root.child(
            Node::label(psave_lbl, fs(9.0), psave_ink)
                .halign(HAlign::Left)
                .valign(VAlign::Top)
                .margin(s(36.0), h - s(17.5), 0.0, 0.0),
        );
    }

    if !present {
        root = root.child(
            Node::anchored(0.0, -s(6.0))
                .text_("no battery")
                .size_(fs(9.0))
                .color_(crate::ui::fg3(pal)),
        );
    }

    if hover_dots {
        let mut buf: Vec<Cmd> = Vec::new();
        battery_dots(&mut buf, pane, 0.0, 0.0, w, h, grid, pal);
        root = root.child(Node::stamp(buf));
    }

    if pane1 {
        root = root.child(
            Node::hit(crate::shell::BATTERY_PSAVE_KEY)
                .margin(s(12.0), h - s(24.0), s(38.0), s(2.0)),
        );
    }
    root
}

/// The retired Battery drawer — the byte-parity oracle for `battery`.
#[cfg(test)]
#[allow(clippy::too_many_arguments)]
fn battery_native(
    v: &mut Vec<Cmd>,
    show_glyph: bool,
    show_title: bool,
    horizontal: bool,
    pct: i32,
    pct_txt: &str,
    ink: u32,
    pane: usize,
    present: bool,
    hover_dots: bool,
    rows: &[(String, String)],
    psave_bg: u32,
    psave_ink: u32,
    psave_lbl: &str,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    grid: f32,
    pal: &Pal,
) -> Vec<CardHit> {
    let s = |n: f32| n * grid;
    let fs = |n: f32| (n * grid * 10.0).round() / 10.0;
    let p = s(12.0);
    let mut tx = p;
    if show_glyph {
        crate::ui::text(v, x + p, y + s(8.0), "\u{f240}", fs(12.0), crate::ui::fg2(pal), true);
        tx = p + s(17.0);
    }
    if show_title {
        crate::ui::title(v, x + tx, y + s(9.0), "Battery", fs(11.5), pal.fg);
    }

    let pane0 = present && pane == 0;
    let pane1 = present && pane == 1;

    // ── pane 0: gauge + percent ──
    if pane0 {
        if horizontal {
            // side-laid, card-fit (the batteryh module's clamped 0.46/0.36 box)
            let bw = ((w as f64 * 0.46).clamp(40.0, 220.0)) as f32;
            let bh = ((h as f64 * 0.36).clamp(16.0, 60.0)) as f32;
            let cx = x + w / 2.0;
            let cy = y + s(42.0) + (h - s(70.0)) / 2.0;
            crate::scene::push_battery_icon_parts(
                v,
                crate::ui::fg2(pal),
                crate::ui::hover(pal),
                ink,
                pct,
                cx,
                cy,
                bw,
                bh,
                false,
            );
            crate::ui::text_c(v, x + w / 2.0, y + h - s(22.0), pct_txt, fs(12.0), ink, false);
        } else {
            // upright, the batteryv module's 0.30-fit body
            let aw = ((w as f64 * 0.30).clamp(16.0, 70.0)) as f64;
            let ah = ((h as f64 - 44.0 * grid as f64).max(aw * 1.6)) as f64;
            let iw = (aw * 2.0 / 3.0).max(12.0);
            let cx = x + w / 2.0;
            let cy = y as f64 + 38.0 * grid as f64 + (ah - iw * 1.7) / 2.0;
            crate::scene::push_battery_icon_parts(
                v,
                crate::ui::fg2(pal),
                crate::ui::hover(pal),
                ink,
                pct,
                cx,
                cy as f32,
                iw as f32,
                (iw * 1.7) as f32,
                true,
            );
            crate::ui::text_c(v, x + w / 2.0, y + h - s(24.0), pct_txt, fs(11.0), ink, false);
        }
    }

    // ── pane 1: metric rows + power-save pill + its label ──
    if pane1 {
        battery_rows_pane(v, rows, x, y, w, h, grid, pal);
        v.push(Cmd::Rect {
            x: x + s(12.0),
            y: y + h - s(24.0),
            w: w - s(50.0),
            h: s(22.0),
            r: s(11.0),
            color: psave_bg,
        });
        crate::ui::text(v, x + s(36.0), y + h - s(17.5), psave_lbl, fs(9.0), psave_ink, false);
    }

    // ── the no-battery state (the module's early return equivalent) ──
    if !present {
        crate::ui::text_c(v, x + w / 2.0, y + h / 2.0 - s(6.0), "no battery", fs(9.0), crate::ui::fg3(pal), false);
    }

    // ── pane dots (only while the cursor rests on the card) ──
    if hover_dots {
        battery_dots(v, pane, x, y, w, h, grid, pal);
    }

    // the power-save pill hit, pane 1 only (the module gated its Hit on the pane)
    let mut hits = Vec::new();
    if pane1 {
        hits.push((
            x + s(12.0),
            y + h - s(24.0),
            w - s(50.0),
            s(22.0),
            crate::shell::BATTERY_PSAVE_KEY,
        ));
    }
    hits
}

/// The Water card — bead-ring gauge + hero readout + the − / + glass chips
/// (keys 31301 / 31300, Rust click arms apply the actions).
#[allow(clippy::too_many_arguments)]
pub(crate) fn water(
    show_glyph: bool,
    show_title: bool,
    lit: i32,
    ring_ink: u32,
    hero: &str,
    hero_ink: u32,
    dim: u32,
    goal_txt: &str,
    meta: &str,
    dec_bg: u32,
    inc_bg: u32,
    w: f32,
    h: f32,
    grid: f32,
    pal: &Pal,
) -> Node {
    let s = |n: f32| n * grid;
    let fs = |n: f32| (n * grid * 10.0).round() / 10.0;
    let p = s(12.0);
    let mut root = Node::box_(0.0, 0);
    let mut tx = p;
    if show_glyph {
        root = root.child(
            Node::glyph("\u{e430}", fs(12.0), crate::ui::fg2(pal))
                .halign(HAlign::Left)
                .valign(VAlign::Top)
                .margin(p, s(8.0), 0.0, 0.0),
        );
        tx = p + s(17.0);
    }
    if show_title {
        root = root.child(
            Node::label("Water", fs(11.5), pal.fg)
                .font(Ff::Ui, Fw::Semibold)
                .halign(HAlign::Left)
                .valign(VAlign::Top)
                .margin(tx, s(9.0), 0.0, 0.0),
        );
    }
    if !meta.is_empty() {
        root = root.child(
            Node::label(meta, fs(8.5), crate::ui::fg3(pal))
                .font(Ff::Ui, Fw::Regular)
                .halign(HAlign::Right)
                .valign(VAlign::Top)
                .margin(0.0, s(11.0), p, 0.0),
        );
    }

    let base_r = ((h as f64 / grid as f64 - 62.0) / 2.0).clamp(20.0, 34.0);
    let base_d = if base_r >= 26.0 { 4.0 } else { 3.5 };
    let mut ring: Vec<Cmd> = Vec::new();
    crate::scene::bead_ring(
        &mut ring,
        w / 2.0,
        h / 2.0 - s(7.0),
        (base_r * grid as f64) as f32,
        (base_d * grid as f64) as f32,
        36,
        lit.max(0) as usize,
        ring_ink,
        dim,
    );
    root = root.child(Node::stamp(ring));

    let box_cx = s(60.0);
    let hsz = if h >= s(114.0) { 14.0 } else { 12.0 };
    root = root.child(
        Node::anchored(box_cx - w / 2.0, -s(13.0))
            .text_(hero)
            .size_(fs(hsz))
            .color_(hero_ink)
            .font(Ff::Display, Fw::Medium),
    );
    root = root.child(
        Node::anchored(box_cx - w / 2.0, s(1.0))
            .text_(goal_txt)
            .size_(fs(7.5))
            .color_(crate::ui::fg3(pal))
            .font(Ff::Ui, Fw::Regular),
    );

    let bw = s(46.0);
    let bh = s(22.0);
    let sp = s(12.0);
    let block = 2.0 * bw + sp;
    let b0 = w / 2.0 - block / 2.0;
    let by = (h - s(32.0)) + s(6.25);
    let b1 = b0 + bw + sp;
    root = root.child(
        Node::box_(s(6.0), dec_bg).margin(b0, by, w - b0 - bw, h - by - bh),
    );
    root = root.child(
        Node::anchored(b0 + bw / 2.0 - w / 2.0, by - h / 2.0)
            .text_("\u{2212}")
            .size_(fs(9.5))
            .color_(pal.fg),
    );
    root = root.child(
        Node::box_(s(6.0), inc_bg).margin(b1, by, w - b1 - bw, h - by - bh),
    );
    root = root.child(
        Node::anchored(b1 + bw / 2.0 - w / 2.0, by - h / 2.0)
            .text_("+ glass")
            .size_(fs(9.5))
            .color_(pal.acc),
    );
    root = root.child(
        Node::hit(crate::shell::WATER_DEC_KEY).margin(b0, by, w - b0 - bw, h - by - bh),
    );
    root = root.child(
        Node::hit(crate::shell::WATER_INC_KEY).margin(b1, by, w - b1 - bw, h - by - bh),
    );
    root
}

/// The retired Water drawer — the byte-parity oracle for `water`.
#[cfg(test)]
#[allow(clippy::too_many_arguments)]
fn water_native(
    v: &mut Vec<Cmd>,
    show_glyph: bool,
    show_title: bool,
    lit: i32,
    ring_ink: u32,
    hero: &str,
    hero_ink: u32,
    dim: u32,
    goal_txt: &str,
    meta: &str,
    dec_bg: u32,
    inc_bg: u32,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    grid: f32,
    pal: &Pal,
) -> Vec<CardHit> {
    let s = |n: f32| n * grid;
    let fs = |n: f32| (n * grid * 10.0).round() / 10.0;
    let p = s(12.0);
    let mut tx = p;
    if show_glyph {
        crate::ui::text(v, x + p, y + s(8.0), "\u{e430}", fs(12.0), crate::ui::fg2(pal), true);
        tx = p + s(17.0);
    }
    if show_title {
        crate::ui::title(v, x + tx, y + s(9.0), "Water", fs(11.5), pal.fg);
    }
    if !meta.is_empty() {
        crate::ui::caption_r(v, x + w - p, y + s(11.0), meta, fs(8.5), crate::ui::fg3(pal), false);
    }

    // bead-ring gauge — card-centred, sized by the `Half` fit (offset 62);
    // the fit ladder runs in the module's f64, coerced per call
    let mut base_r = (h as f64 / grid as f64 - 62.0) / 2.0;
    if base_r < 20.0 {
        base_r = 20.0;
    } else if base_r > 34.0 {
        base_r = 34.0;
    }
    let base_d = if base_r >= 26.0 { 4.0 } else { 3.5 };
    crate::scene::bead_ring(
        v,
        x + w / 2.0,
        y + h / 2.0 - s(7.0),
        (base_r * grid as f64) as f32,
        (base_d * grid as f64) as f32,
        36,
        lit.max(0) as usize,
        ring_ink,
        dim,
    );

    // hero readout + goal caption — both centered on the 120px box from x
    let box_cx = x + s(60.0);
    let hsz = if h >= s(114.0) { 14.0 } else { 12.0 };
    crate::ui::text_cw(
        v,
        box_cx,
        y + h / 2.0 - s(13.0),
        hero,
        fs(hsz),
        crate::text::Ff::Display,
        crate::text::Fw::Medium,
        hero_ink,
        false,
    );
    crate::ui::caption_c(v, box_cx, y + h / 2.0 + s(1.0), goal_txt, fs(7.5), crate::ui::fg3(pal), false);

    // − / + glass buttons, centered on the card, 10px above the bottom
    let bw = s(46.0);
    let bh = s(22.0);
    let sp = s(12.0);
    let block = 2.0 * bw + sp;
    let b0 = x + ((w - block).max(0.0)) / 2.0;
    let by = (y + h - s(32.0)) + s(6.25);
    v.push(Cmd::Rect {
        x: b0,
        y: by,
        w: bw,
        h: bh,
        r: s(6.0),
        color: dec_bg,
    });
    crate::ui::text_c(v, b0 + bw / 2.0, by, "\u{2212}", fs(9.5), pal.fg, false);
    let b1 = b0 + bw + sp;
    v.push(Cmd::Rect {
        x: b1,
        y: by,
        w: bw,
        h: bh,
        r: s(6.0),
        color: inc_bg,
    });
    crate::ui::text_c(v, b1 + bw / 2.0, by, "+ glass", fs(9.5), pal.acc, false);
    vec![(b0, by, bw, bh, 31301), (b1, by, bw, bh, 31300)]
}

/// The Eye rest card — bead ring + mm:ss hero + the toggle/reset chips
/// (keys 31900 / 31901, Rust click arms apply the actions).
#[allow(clippy::too_many_arguments)]
pub(crate) fn eyerest(
    show_glyph: bool,
    show_title: bool,
    meta: &str,
    hero: &str,
    hero_ink: u32,
    state: &str,
    tog_lbl: &str,
    lit: i32,
    ring_ink: u32,
    tog_bg: u32,
    rst_bg: u32,
    _hover: u32,
    w: f32,
    h: f32,
    grid: f32,
    pal: &Pal,
) -> Node {
    let s = |n: f32| n * grid;
    let fs = |n: f32| (n * grid * 10.0).round() / 10.0;
    let p = s(12.0);
    let mut root = Node::box_(0.0, 0);
    let mut tx = p;
    if show_glyph {
        root = root.child(
            Node::glyph("\u{f573}", fs(12.0), crate::ui::fg2(pal))
                .halign(HAlign::Left)
                .valign(VAlign::Top)
                .margin(p, s(8.0), 0.0, 0.0),
        );
        tx = p + s(17.0);
    }
    if show_title {
        root = root.child(
            Node::label("Eye rest", fs(11.5), pal.fg)
                .font(Ff::Ui, Fw::Semibold)
                .halign(HAlign::Left)
                .valign(VAlign::Top)
                .margin(tx, s(9.0), 0.0, 0.0),
        );
    }
    if !meta.is_empty() {
        root = root.child(
            Node::label(meta, fs(8.5), crate::ui::fg3(pal))
                .font(Ff::Ui, Fw::Regular)
                .halign(HAlign::Right)
                .valign(VAlign::Top)
                .margin(0.0, s(11.0), p, 0.0),
        );
    }

    let mut ring: Vec<Cmd> = Vec::new();
    crate::scene::bead_ring(
        &mut ring,
        s(40.0),
        s(50.0),
        s(23.0),
        s(3.0),
        36,
        lit.max(0) as usize,
        ring_ink,
        crate::ui::fg3(pal),
    );
    root = root.child(Node::stamp(ring));

    root = root.child(
        Node::label(hero, fs(21.0), hero_ink)
            .font(Ff::Display, Fw::Medium)
            .halign(HAlign::Left)
            .valign(VAlign::Top)
            .margin(s(75.0), s(43.0), 0.0, 0.0),
    );
    root = root.child(
        Node::label(state, fs(8.5), crate::ui::fg3(pal))
            .font(Ff::Ui, Fw::Regular)
            .halign(HAlign::Left)
            .valign(VAlign::Top)
            .margin(s(75.0), s(60.0), 0.0, 0.0),
    );
    root = root.child(
        Node::label("20 min focus \u{b7} 20 s rest", fs(8.0), crate::ui::fg3(pal))
            .font(Ff::Ui, Fw::Regular)
            .halign(HAlign::Left)
            .valign(VAlign::Top)
            .margin(s(12.0), s(79.0), 0.0, 0.0),
    );

    let by = h - s(32.0);
    let rx = w - p - s(52.0);
    root = root.child(
        Node::box_(s(6.0), tog_bg).margin(p, by, w - p - s(52.0), h - by - s(22.0)),
    );
    root = root.child(
        Node::anchored(p + s(52.0) / 2.0 - w / 2.0, by - h / 2.0)
            .text_(tog_lbl)
            .size_(fs(9.5))
            .color_(pal.acc),
    );
    root = root.child(
        Node::box_(s(6.0), rst_bg).margin(rx, by, w - rx - s(52.0), h - by - s(22.0)),
    );
    root = root.child(
        Node::anchored(rx + s(52.0) / 2.0 - w / 2.0, by - h / 2.0)
            .text_("Reset")
            .size_(fs(9.5))
            .color_(pal.fg),
    );
    root = root.child(
        Node::hit(crate::shell::EYEREST_TOGGLE_KEY).margin(p, by, s(52.0), s(22.0)),
    );
    root = root.child(
        Node::hit(crate::shell::EYEREST_RESET_KEY).margin(rx, by, s(52.0), s(22.0)),
    );
    root
}

/// The retired Eye rest drawer — the byte-parity oracle for `eyerest`.
#[cfg(test)]
#[allow(clippy::too_many_arguments)]
fn eyerest_native(
    v: &mut Vec<Cmd>,
    show_glyph: bool,
    show_title: bool,
    meta: &str,
    hero: &str,
    hero_ink: u32,
    state: &str,
    tog_lbl: &str,
    lit: i32,
    ring_ink: u32,
    tog_bg: u32,
    rst_bg: u32,
    _hover: u32,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    grid: f32,
    pal: &Pal,
) -> Vec<CardHit> {
    let s = |n: f32| n * grid;
    let fs = |n: f32| (n * grid * 10.0).round() / 10.0;
    let p = s(12.0);
    let mut tx = p;
    if show_glyph {
        crate::ui::text(v, x + p, y + s(8.0), "\u{f573}", fs(12.0), crate::ui::fg2(pal), true);
        tx = p + s(17.0);
    }
    if show_title {
        crate::ui::title(v, x + tx, y + s(9.0), "Eye rest", fs(11.5), pal.fg);
    }
    if !meta.is_empty() {
        crate::ui::caption_r(v, x + w - p, y + s(11.0), meta, fs(8.5), crate::ui::fg3(pal), false);
    }

    crate::scene::bead_ring(
        v,
        x + s(40.0),
        y + s(50.0),
        s(23.0),
        s(3.0),
        36,
        lit.max(0) as usize,
        ring_ink,
        crate::ui::fg3(pal),
    );

    crate::ui::text_hero(v, x + s(75.0), y + s(43.0), hero, fs(21.0), hero_ink);
    crate::ui::caption(v, x + s(75.0), y + s(60.0), state, fs(8.5), crate::ui::fg3(pal), false);
    crate::ui::caption(
        v,
        x + s(12.0),
        y + s(79.0),
        "20 min focus \u{b7} 20 s rest",
        fs(8.0),
        crate::ui::fg3(pal),
        false,
    );

    let by = y + h - s(32.0);
    v.push(Cmd::Rect {
        x: x + p,
        y: by,
        w: s(52.0),
        h: s(22.0),
        r: s(6.0),
        color: tog_bg,
    });
    crate::ui::text_c(v, x + p + s(52.0) / 2.0, by, tog_lbl, fs(9.5), pal.acc, false);
    let rx = x + w - p - s(52.0);
    v.push(Cmd::Rect {
        x: rx,
        y: by,
        w: s(52.0),
        h: s(22.0),
        r: s(6.0),
        color: rst_bg,
    });
    crate::ui::text_c(v, rx + s(52.0) / 2.0, by, "Reset", fs(9.5), pal.fg, false);
    vec![(x + p, by, s(52.0), s(22.0), 31900), (rx, by, s(52.0), s(22.0), 31901)]
}

/// The Focus (pomodoro) card — hero countdown, progress bar, idle steppers
/// and the toggle/reset chips (keys 30730..30733 via Rust click arms).
#[allow(clippy::too_many_arguments)]
pub(crate) fn pomodoro(
    show_glyph: bool,
    show_title: bool,
    meta: &str,
    hero: &str,
    ink: u32,
    bar_ink: u32,
    phase_lbl: &str,
    frac: f32,
    idle: bool,
    focus_readout: &str,
    dec_dx: f32,
    inc_dx: f32,
    chip_fill: u32,
    tog_fill: u32,
    tog_hover: u32,
    tog_lbl: &str,
    hover: u32,
    w: f32,
    h: f32,
    grid: f32,
    pal: &Pal,
) -> Node {
    let s = |n: f32| n * grid;
    let fs = |n: f32| (n * grid * 10.0).round() / 10.0;
    let p = s(12.0);
    let mut root = Node::box_(0.0, 0);
    let mut tx = p;
    if show_glyph {
        root = root.child(
            Node::glyph("\u{f573}", fs(12.0), crate::ui::fg2(pal))
                .halign(HAlign::Left)
                .valign(VAlign::Top)
                .margin(p, s(8.0), 0.0, 0.0),
        );
        tx = p + s(17.0);
    }
    if show_title {
        root = root.child(
            Node::label("Focus", fs(11.5), pal.fg)
                .font(Ff::Ui, Fw::Semibold)
                .halign(HAlign::Left)
                .valign(VAlign::Top)
                .margin(tx, s(9.0), 0.0, 0.0),
        );
    }
    if !meta.is_empty() {
        root = root.child(
            Node::label(meta, fs(8.5), crate::ui::fg3(pal))
                .font(Ff::Ui, Fw::Regular)
                .halign(HAlign::Right)
                .valign(VAlign::Top)
                .margin(0.0, s(11.0), p, 0.0),
        );
    }

    root = root.child(
        Node::anchored(0.0, s(30.0) - h / 2.0)
            .text_(hero)
            .size_(fs(26.0))
            .color_(ink),
    );
    root = root.child(
        Node::anchored(0.0, s(58.0) - h / 2.0)
            .text_(phase_lbl)
            .size_(fs(8.5))
            .color_(crate::ui::fg3(pal))
            .font(Ff::Ui, Fw::Regular),
    );
    root = root.child(
        Node::bar(frac, 1.0, bar_ink, crate::ui::hover(pal), s(2.0), false)
            .margin(s(12.0), s(72.0), s(12.0), h - s(72.0) - s(4.0)),
    );

    if idle {
        let bw = s(18.0);
        let sy = s(82.0);
        let ly = sy + s(3.5);
        let dec = w / 2.0 + s(dec_dx);
        let dcol = if hover == 30733 {
            crate::ui::hover_hl(pal)
        } else {
            chip_fill
        };
        root = root.child(
            Node::box_(s(6.0), dcol).margin(dec, sy, w - dec - bw, h - sy - bw),
        );
        root = root.child(
            Node::anchored(dec + bw / 2.0 - w / 2.0, ly - h / 2.0)
                .text_("-")
                .size_(fs(9.5))
                .color_(pal.fg),
        );
        root = root.child(
            Node::anchored(0.0, s(85.0) - h / 2.0)
                .text_(focus_readout)
                .size_(fs(8.5))
                .color_(crate::ui::fg3(pal))
                .font(Ff::Ui, Fw::Regular),
        );
        let inc = w / 2.0 + s(inc_dx);
        let icol = if hover == 30732 {
            crate::ui::hover_hl(pal)
        } else {
            chip_fill
        };
        root = root.child(
            Node::box_(s(6.0), icol).margin(inc, sy, w - inc - bw, h - sy - bw),
        );
        root = root.child(
            Node::anchored(inc + bw / 2.0 - w / 2.0, ly - h / 2.0)
                .text_("+")
                .size_(fs(9.5))
                .color_(pal.fg),
        );
    }

    let bw2 = s(52.0);
    let bh2 = s(22.0);
    let cy2 = h - s(32.0);
    let ly2 = cy2 + s(6.25);
    let tog = w / 2.0 + s(-58.0);
    let tcol = if hover == 30730 { tog_hover } else { tog_fill };
    root = root.child(
        Node::box_(s(6.0), tcol).margin(tog, cy2, w - tog - bw2, h - cy2 - bh2),
    );
    root = root.child(
        Node::anchored(s(-58.0) + bw2 / 2.0, ly2 - h / 2.0)
            .text_(tog_lbl)
            .size_(fs(9.5))
            .color_(pal.acc),
    );
    let rst = w / 2.0 + s(6.0);
    let rcol = if hover == 30731 {
        crate::ui::hover_hl(pal)
    } else {
        chip_fill
    };
    root = root.child(
        Node::box_(s(6.0), rcol).margin(rst, cy2, w - rst - bw2, h - cy2 - bh2),
    );
    root = root.child(
        Node::anchored(s(6.0) + bw2 / 2.0, ly2 - h / 2.0)
            .text_("Reset")
            .size_(fs(9.5))
            .color_(pal.fg),
    );

    if idle {
        let bw = s(18.0);
        let sy = s(82.0);
        let dec = w / 2.0 + s(dec_dx);
        let inc = w / 2.0 + s(inc_dx);
        root = root.child(Node::hit(30733).margin(dec, sy, w - dec - bw, h - sy - bw));
        root = root.child(Node::hit(30732).margin(inc, sy, w - inc - bw, h - sy - bw));
    }
    root = root.child(Node::hit(30730).margin(tog, cy2, w - tog - bw2, h - cy2 - bh2));
    root = root.child(Node::hit(30731).margin(rst, cy2, w - rst - bw2, h - cy2 - bh2));
    root
}

/// The retired Focus drawer — the byte-parity oracle for `pomodoro`.
#[cfg(test)]
#[allow(clippy::too_many_arguments)]
fn pomodoro_native(
    v: &mut Vec<Cmd>,
    show_glyph: bool,
    show_title: bool,
    meta: &str,
    hero: &str,
    ink: u32,
    bar_ink: u32,
    phase_lbl: &str,
    frac: f32,
    idle: bool,
    focus_readout: &str,
    dec_dx: f32,
    inc_dx: f32,
    chip_fill: u32,
    tog_fill: u32,
    tog_hover: u32,
    tog_lbl: &str,
    hover: u32,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    grid: f32,
    pal: &Pal,
) -> Vec<CardHit> {
    let s = |n: f32| n * grid;
    let fs = |n: f32| (n * grid * 10.0).round() / 10.0;
    let p = s(12.0);
    let mut tx = p;
    if show_glyph {
        crate::ui::text(v, x + p, y + s(8.0), "\u{f573}", fs(12.0), crate::ui::fg2(pal), true);
        tx = p + s(17.0);
    }
    if show_title {
        crate::ui::title(v, x + tx, y + s(9.0), "Focus", fs(11.5), pal.fg);
    }
    if !meta.is_empty() {
        crate::ui::caption_r(v, x + w - p, y + s(11.0), meta, fs(8.5), crate::ui::fg3(pal), false);
    }

    crate::ui::text_c(v, x + w / 2.0, y + s(30.0), hero, fs(26.0), ink, false);
    crate::ui::caption_c(v, x + w / 2.0, y + s(58.0), phase_lbl, fs(8.5), crate::ui::fg3(pal), false);
    bar_prim(
        v,
        x + s(12.0),
        y + s(72.0),
        w - s(12.0) - s(12.0),
        s(4.0),
        s(2.0),
        frac,
        1.0,
        bar_ink,
        crate::ui::hover(pal),
        false,
    );

    let mut hits = Vec::new();
    if idle {
        let bw = s(18.0);
        let sy = y + s(82.0);
        let ly = sy + s(3.5);
        let dec = x + w / 2.0 + s(dec_dx);
        let dcol = if hover == 30733 {
            crate::ui::hover_hl(pal)
        } else {
            chip_fill
        };
        v.push(Cmd::Rect {
            x: dec,
            y: sy,
            w: bw,
            h: bw,
            r: s(6.0),
            color: dcol,
        });
        crate::ui::text_c(v, dec + bw / 2.0, ly, "-", fs(9.5), pal.fg, false);
        hits.push((dec, sy, bw, bw, 30733));
        crate::ui::caption_c(v, x + w / 2.0, y + s(85.0), focus_readout, fs(8.5), crate::ui::fg3(pal), false);
        let inc = x + w / 2.0 + s(inc_dx);
        let icol = if hover == 30732 {
            crate::ui::hover_hl(pal)
        } else {
            chip_fill
        };
        v.push(Cmd::Rect {
            x: inc,
            y: sy,
            w: bw,
            h: bw,
            r: s(6.0),
            color: icol,
        });
        crate::ui::text_c(v, inc + bw / 2.0, ly, "+", fs(9.5), pal.fg, false);
        hits.push((inc, sy, bw, bw, 30732));
    }

    let bw2 = s(52.0);
    let bh2 = s(22.0);
    let cy2 = y + h - s(32.0);
    let ly2 = cy2 + s(6.25);
    let tog = x + w / 2.0 + s(-58.0);
    let tcol = if hover == 30730 { tog_hover } else { tog_fill };
    v.push(Cmd::Rect {
        x: tog,
        y: cy2,
        w: bw2,
        h: bh2,
        r: s(6.0),
        color: tcol,
    });
    crate::ui::text_c(v, tog + bw2 / 2.0, ly2, tog_lbl, fs(9.5), pal.acc, false);
    hits.push((tog, cy2, bw2, bh2, 30730));
    let rst = x + w / 2.0 + s(6.0);
    let rcol = if hover == 30731 {
        crate::ui::hover_hl(pal)
    } else {
        chip_fill
    };
    v.push(Cmd::Rect {
        x: rst,
        y: cy2,
        w: bw2,
        h: bh2,
        r: s(6.0),
        color: rcol,
    });
    crate::ui::text_c(v, rst + bw2 / 2.0, ly2, "Reset", fs(9.5), pal.fg, false);
    hits.push((rst, cy2, bw2, bh2, 30731));
    hits
}

/// The Mic card — level bar + live status + the mute button (key 31910 via
/// the Rust click arm).
#[allow(clippy::too_many_arguments)]
pub(crate) fn micmeter(
    show_glyph: bool,
    show_title: bool,
    icon: &str,
    meta: &str,
    lvl: f32,
    bar_ink: u32,
    status: &str,
    status_ink: u32,
    btn_fill: u32,
    btn_label: &str,
    btn_ink: u32,
    w: f32,
    h: f32,
    grid: f32,
    pal: &Pal,
) -> Node {
    let s = |n: f32| n * grid;
    let fs = |n: f32| (n * grid * 10.0).round() / 10.0;
    let p = s(12.0);
    let mut root = Node::box_(0.0, 0);
    if show_glyph {
        root = root.child(
            Node::glyph(icon, fs(12.0), crate::ui::fg2(pal))
                .halign(HAlign::Left)
                .valign(VAlign::Top)
                .margin(p, s(8.0), 0.0, 0.0),
        );
    }
    if show_title {
        root = root.child(
            Node::label("Mic", fs(11.5), pal.fg)
                .font(Ff::Ui, Fw::Semibold)
                .halign(HAlign::Left)
                .valign(VAlign::Top)
                .margin(p + s(17.0), s(9.0), 0.0, 0.0),
        );
    }
    if !meta.is_empty() {
        root = root.child(
            Node::label(meta, fs(8.5), crate::ui::fg3(pal))
                .font(Ff::Mono, Fw::Regular)
                .halign(HAlign::Right)
                .valign(VAlign::Top)
                .margin(0.0, s(11.0), p, 0.0),
        );
    }
    let bar_w = (w - p * 2.0).max(2.0);
    root = root.child(
        Node::bar(lvl, 1.0, bar_ink, crate::ui::hover(pal), s(4.0), false)
            .margin(s(12.0), s(34.0), w - s(12.0) - bar_w, h - s(34.0) - s(8.0)),
    );
    root = root.child(
        Node::label(status, fs(9.0), status_ink)
            .font(Ff::Ui, Fw::Regular)
            .halign(HAlign::Left)
            .valign(VAlign::Top)
            .margin(s(12.0), s(50.0), 0.0, 0.0),
    );

    root = root.child(
        Node::box_(s(6.0), btn_fill).margin(
            s(12.0),
            h - s(12.0) - s(22.0),
            s(12.0),
            s(12.0),
        ),
    );
    root = root.child(
        Node::anchored(0.0, (h - s(28.0)) - h / 2.0)
            .text_(btn_label)
            .size_(fs(9.0))
            .color_(btn_ink)
            .icon(),
    );
    root = root.child(
        Node::hit(crate::shell::MIC_MUTE_KEY).margin(
            s(9.0),
            h - s(37.0),
            s(18.0),
            h - (h - s(37.0)) - s(28.0),
        ),
    );
    root
}

/// The retired Mic drawer — the byte-parity oracle for `micmeter`.
#[cfg(test)]
#[allow(clippy::too_many_arguments)]
fn micmeter_native(
    v: &mut Vec<Cmd>,
    show_glyph: bool,
    show_title: bool,
    icon: &str,
    meta: &str,
    lvl: f32,
    bar_ink: u32,
    status: &str,
    status_ink: u32,
    btn_fill: u32,
    btn_label: &str,
    btn_ink: u32,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    grid: f32,
    pal: &Pal,
) -> Vec<CardHit> {
    let s = |n: f32| n * grid;
    let fs = |n: f32| (n * grid * 10.0).round() / 10.0;
    let p = s(12.0);
    if show_glyph {
        crate::ui::text(v, x + p, y + s(8.0), icon, fs(12.0), crate::ui::fg2(pal), true);
    }
    if show_title {
        crate::ui::title(v, x + p + s(17.0), y + s(9.0), "Mic", fs(11.5), pal.fg);
    }
    if !meta.is_empty() {
        crate::ui::text_r_mono(v, x + w - p, y + s(11.0), meta, fs(8.5), crate::ui::fg3(pal));
    }
    let bar_w = (w - p * 2.0).max(2.0);
    bar_prim(
        v,
        x + s(12.0),
        y + s(34.0),
        bar_w,
        s(8.0),
        s(4.0),
        lvl,
        1.0,
        bar_ink,
        crate::ui::hover(pal),
        false,
    );
    crate::ui::caption(v, x + s(12.0), y + s(50.0), status, fs(9.0), status_ink, false);

    v.push(Cmd::Rect {
        x: x + s(12.0),
        y: y + h - s(12.0) - s(22.0),
        w: w - s(24.0),
        h: s(22.0),
        r: s(6.0),
        color: btn_fill,
    });
    crate::ui::text_c(v, x + w / 2.0, y + h - s(28.0), btn_label, fs(9.0), btn_ink, true);
    vec![(
        x + s(9.0),
        y + h - s(37.0),
        w - s(18.0),
        s(28.0),
        crate::shell::MIC_MUTE_KEY,
    )]
}

/// The Active Window card — twin-state hero (empty vs live app class/title)
/// plus the per-app RAM pill.
#[allow(clippy::too_many_arguments)]
pub(crate) fn activewin(
    empty: bool,
    class: &str,
    title: &str,
    rss: Option<&str>,
    _w: f32,
    h: f32,
    grid: f32,
    pal: &Pal,
) -> Node {
    let s = |n: f32| n * grid;
    let fs = |n: f32| (n * grid * 10.0).round() / 10.0;
    let mut root = Node::box_(0.0, 0);
    if empty {
        root = root.child(
            Node::anchored(0.0, s(14.0) - h / 2.0)
                .text_("\u{f2d0}")
                .size_(fs(20.0))
                .color_(pal.fg)
                .icon(),
        );
        root = root.child(
            Node::anchored(0.0, s(10.0))
                .text_("No window")
                .size_(fs(10.0))
                .color_(pal.fg),
        );
    }
    if !empty {
        root = root.child(
            Node::anchored(0.0, s(14.0) - h / 2.0)
                .text_(class)
                .size_(fs(13.0))
                .color_(pal.fg),
        );
        root = root.child(
            Node::anchored(0.0, s(36.0) - h / 2.0)
                .text_(title)
                .size_(fs(9.5))
                .color_(pal.fg),
        );
    }
    if let Some(rss) = rss {
        root = root.child(
            Node::anchored(0.0, (h - s(16.0)) - h / 2.0)
                .text_(rss)
                .size_(fs(9.0))
                .color_(crate::ui::fg2(pal)),
        );
    }
    root
}

/// The retired Active Window drawer — the byte-parity oracle for `activewin`.
#[cfg(test)]
#[allow(clippy::too_many_arguments)]
fn activewin_native(
    v: &mut Vec<Cmd>,
    empty: bool,
    class: &str,
    title: &str,
    rss: Option<&str>,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    grid: f32,
    pal: &Pal,
) {
    let s = |n: f32| n * grid;
    let fs = |n: f32| (n * grid * 10.0).round() / 10.0;
    let cx = x + w / 2.0;
    if empty {
        crate::ui::text_c(v, cx, y + s(14.0), "\u{f2d0}", fs(20.0), pal.fg, true);
        crate::ui::text_c(v, cx, y + h / 2.0 + s(10.0), "No window", fs(10.0), pal.fg, false);
    }
    if !empty {
        crate::ui::text_c(v, cx, y + s(14.0), class, fs(13.0), pal.fg, false);
        crate::ui::text_c(v, cx, y + s(36.0), title, fs(9.5), pal.fg, false);
    }
    if let Some(rss) = rss {
        crate::ui::text_c(v, cx, y + h - s(16.0), rss, fs(9.0), crate::ui::fg2(pal), false);
    }
}

// ── BATCH 5 (stage 5): the interactive heavyweights. The committed modules
// are pure-paint (whole backend state in `ctx.api`), so these drawers take
// exactly the api values; the shell arms feed them + register the returned
// hit tuples. All module arithmetic stays in f64 (the Lua number type),
// cast to f32 at the primitive boundary, so the streams land bit-identical.

/// The Weather v2 card — glyph/condition/city/date-square + the wind ·
/// humidity · sunshine stat row (the retired `draw_weather_v2_card` shape).
/// `u8` glyph sizes arrive pre-rounded via the api; `month`/`day` come from
/// the shell's `today_month_day`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn weatherv2(
    v: &mut Vec<Cmd>,
    live: bool,
    glyph: &str,
    desc: &str,
    city: &str,
    temp_c: f32,
    wind_kmh: f32,
    humidity: Option<f32>,
    sunshine: Option<i32>,
    month: &str,
    day: u8,
    ic_sunny: &str,
    ic_speed: &str,
    ic_drop: &str,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    grid: f32,
    pal: &Pal,
) {
    // the module's every coordinate is a Lua f64 — device math in f64
    let gx = x as f64;
    let gy = y as f64;
    let gw = w as f64;
    let gh = h as f64;
    let g = grid as f64;
    let s = |n: f64| n * g;
    let fs = |n: f64| (n * g * 10.0).round() / 10.0;
    let pad = s(14.0);
    let ccx = gx + gw / 2.0;

    // no reading yet: the sunny glyph + "waiting…" placeholder
    if !live {
        crate::ui::text_c(
            v,
            ccx as f32,
            (gy + gh / 2.0 - s(4.0)) as f32,
            ic_sunny,
            fs(18.0) as f32,
            crate::ui::fg2(pal),
            true,
        );
        crate::ui::caption_c(
            v,
            ccx as f32,
            (gy + gh / 2.0 + s(14.0)) as f32,
            "waiting\u{2026}",
            fs(9.5) as f32,
            crate::ui::fg2(pal),
            false,
        );
        return;
    }

    // ── short card: compact single-line fallback ──
    if gh < s(140.0) {
        let label = format!("{glyph} {:.0}\u{b0}C", temp_c as f64);
        crate::ui::text_c(
            v,
            ccx as f32,
            (gy + gh / 2.0 - s(4.0)) as f32,
            &label,
            fs(13.0) as f32,
            pal.fg,
            true,
        );
        let meta = if city.is_empty() {
            desc.to_string()
        } else {
            format!("{desc} \u{b7} {city}")
        };
        crate::ui::caption_c(
            v,
            ccx as f32,
            (gy + gh / 2.0 + s(12.0)) as f32,
            &meta,
            fs(9.5) as f32,
            crate::ui::fg2(pal),
            false,
        );
        return;
    }

    // ── full layout ──
    let glyph_sz = fs(28.0);
    let temp_sz = fs(24.0);
    let cond_sz = fs(11.0);
    let city_sz = fs(10.0);
    let stat_sz = fs(9.5);
    let sq_sz = s(30.0);

    // content block: glyph + temp/cond + city/date + hairline + stats
    let block_h = glyph_sz + s(6.0) + cond_sz + s(8.0) + sq_sz + s(10.0) + s(12.0) + stat_sz;
    let block_top = gy + ((gh - block_h) / 2.0).max(pad);

    // 1. weather glyph (centered)
    let gy2 = block_top;
    crate::ui::text_c(v, ccx as f32, gy2 as f32, glyph, glyph_sz as f32, pal.fg, true);

    // 2. temp + condition (centered pair on one line) — widths run on the
    // module's BYTE lengths (#s), not char counts
    let ty = gy2 + glyph_sz + s(4.0);
    let temp_s = format!("{:.0}\u{b0}C", temp_c as f64);
    let tw = temp_s.len() as f64 * temp_sz * 0.55;
    let cw = desc.len() as f64 * cond_sz * 0.55;
    let pair_gap = s(8.0);
    let pair_w = tw + pair_gap + cw;
    let px = ccx - pair_w / 2.0;
    crate::ui::text_hero(v, px as f32, ty as f32, temp_s.as_str(), temp_sz as f32, pal.fg);
    crate::ui::caption(
        v,
        (px + tw + pair_gap) as f32,
        (ty + temp_sz * 0.15) as f32,
        desc,
        cond_sz as f32,
        crate::ui::fg2(pal),
        false,
    );

    // 3. city (left) + accent date square (right)
    let cy = ty + cond_sz + s(8.0);
    if !city.is_empty() {
        crate::ui::caption(
            v,
            (gx + pad) as f32,
            (cy + sq_sz * 0.38) as f32,
            city,
            city_sz as f32,
            crate::ui::fg2(pal),
            false,
        );
    }
    let sq_x = gx + gw - pad - sq_sz;
    v.push(Cmd::Rect {
        x: sq_x as f32,
        y: cy as f32,
        w: sq_sz as f32,
        h: sq_sz as f32,
        r: s(6.0) as f32,
        color: pal.acc,
    });
    crate::ui::caption_c(
        v,
        (sq_x + sq_sz / 2.0) as f32,
        (cy + s(6.0)) as f32,
        month,
        fs(7.5) as f32,
        pal.sfg,
        false,
    );
    crate::ui::text_cw(
        v,
        (sq_x + sq_sz / 2.0) as f32,
        (cy + s(17.0)) as f32,
        day.to_string(),
        fs(13.0) as f32,
        Ff::Display,
        Fw::Medium,
        pal.sfg,
        false,
    );

    // 4. hairline divider
    let dy = cy + sq_sz + s(6.0);
    v.push(Cmd::Rect {
        x: (gx + pad) as f32,
        y: dy as f32,
        w: (gw - pad * 2.0) as f32,
        h: 1.0,
        r: 0.0,
        color: crate::ui::hairline(pal),
    });

    // 5. stats row: wind · humidity · sunshine
    let sy = dy + s(10.0);
    let icon_w = stat_sz * 1.2;
    let wind_s = format!("{:.0} km/h", wind_kmh as f64);
    let hum_s = humidity.map(|hum| format!("{}%", (hum as f64 + 0.5).floor()));
    let sun_s = sunshine.map(|sun| format!("{sun}%"));

    let avail = gw - pad * 2.0;
    let zone = avail / 3.0;

    // wind (left third)
    let mut item_w = icon_w + wind_s.len() as f64 * stat_sz * 0.5;
    let mut sx = gx + pad + (zone - item_w) / 2.0;
    crate::ui::text(v, sx as f32, sy as f32, ic_speed, stat_sz as f32, crate::ui::fg2(pal), true);
    crate::ui::caption(
        v,
        (sx + icon_w) as f32,
        (sy + s(1.0)) as f32,
        &wind_s,
        stat_sz as f32,
        pal.fg,
        false,
    );

    // humidity (center third)
    if let Some(hum) = &hum_s {
        item_w = icon_w + hum.len() as f64 * stat_sz * 0.5;
        sx = gx + pad + zone + (zone - item_w) / 2.0;
        crate::ui::text(v, sx as f32, sy as f32, ic_drop, stat_sz as f32, crate::ui::fg2(pal), true);
        crate::ui::caption(
            v,
            (sx + icon_w) as f32,
            (sy + s(1.0)) as f32,
            hum,
            stat_sz as f32,
            pal.fg,
            false,
        );
    }

    // sunshine (right third)
    if let Some(sun) = &sun_s {
        item_w = icon_w + sun.len() as f64 * stat_sz * 0.5;
        sx = gx + pad + zone * 2.0 + (zone - item_w) / 2.0;
        crate::ui::text(v, sx as f32, sy as f32, ic_sunny, stat_sz as f32, crate::ui::fg2(pal), true);
        crate::ui::caption(
            v,
            (sx + icon_w) as f32,
            (sy + s(1.0)) as f32,
            sun,
            stat_sz as f32,
            pal.fg,
            false,
        );
    }
}

/// The To-Do card — checkbox rows over the window + the composer plate / `+`
/// chip (compose + toggle/delete keys via the Rust click arms & key handler).
/// The same f64-device discipline as `weatherv2`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn todo(
    todos: &[(String, bool)],
    scroll: usize,
    input: Option<&str>,
    scene_owns_header: bool,
    scene_owns_rows: bool,
    scene_owns_composer: bool,
    card_show_title: bool,
    toggle_key: u32,
    delete_key: u32,
    input_key: u32,
    add_key: u32,
    hover: u32,
    w: f32,
    h: f32,
    grid: f32,
    pal: &Pal,
) -> Node {
    let s = |n: f32| n * grid;
    let fs = |n: f32| (n * grid * 10.0).round() / 10.0;
    let n = todos.len();
    let done = todos.iter().filter(|(_, d)| *d).count();
    let mut root = Node::box_(0.0, 0);

    if !scene_owns_header && card_show_title {
        root = root.child(
            Node::label("To-Do", fs(11.5), pal.fg)
                .font(Ff::Ui, Fw::Semibold)
                .halign(HAlign::Left)
                .valign(VAlign::Top)
                .margin(s(14.0), s(9.0), 0.0, 0.0),
        );
    }
    if n > 0 {
        root = root.child(
            Node::label(format!("{done}/{n}"), fs(8.5), crate::ui::fg3(pal))
                .font(Ff::Ui, Fw::Regular)
                .halign(HAlign::Right)
                .valign(VAlign::Top)
                .margin(0.0, s(11.0), s(14.0), 0.0),
        );
    }

    let limit = n.min(8);
    let start = scroll.min(limit.saturating_sub(1));
    let mut drawn = 0usize;

    if n == 0 {
        root = root.child(
            Node::anchored(0.0, s(60.0) - h / 2.0)
                .text_("No tasks yet")
                .size_(fs(9.5))
                .color_(pal.fg),
        );
        root = root.child(
            Node::anchored(0.0, s(78.0) - h / 2.0)
                .text_("\u{f256}")
                .size_(fs(16.0))
                .color_(crate::ui::mix(crate::ui::hover(pal), pal.fg, 0.10))
                .icon(),
        );
    }

    if !scene_owns_rows {
        let bottom_line = h - s(40.0);
        for (i, (it_text, it_done)) in todos
            .iter()
            .enumerate()
            .skip(start)
            .take(limit.saturating_sub(start))
        {
            let si = i - start;
            let row_y = s(36.0) + si as f32 * s(24.0);
            if row_y + s(24.0) > h + 0.5 {
                break;
            }
            if row_y + s(24.0) > bottom_line {
                break;
            }
            drawn += 1;
            let tog_key = toggle_key + si as u32;
            let del_key = delete_key + si as u32;
            let row_hover = hover == tog_key;
            let show_del = row_hover || hover == del_key;
            if *it_done {
                root = root.child(
                    Node::box_(s(3.5), crate::ui::acc_tint(pal)).margin(
                        s(14.0),
                        row_y + s(1.0),
                        w - s(14.0) - s(11.0),
                        h - (row_y + s(1.0)) - s(11.0),
                    ),
                );
                root = root.child(
                    Node::label("\u{f00c}", fs(8.0), pal.acc)
                        .font(Ff::Ui, Fw::Regular)
                        .halign(HAlign::Right)
                        .valign(VAlign::Top)
                        .margin(0.0, row_y + s(2.5), w - s(26.5), 0.0)
                        .icon(),
                );
            } else {
                root = root.child(Node::stamp(vec![Cmd::Outline {
                    x: s(14.0),
                    y: row_y + s(1.0),
                    w: s(11.0),
                    h: s(11.0),
                    r: s(3.5),
                    width: s(1.2),
                    color: if row_hover {
                        pal.acc
                    } else {
                        crate::ui::mix(crate::ui::hover(pal), pal.fg, 0.25)
                    },
                }]));
            }
            root = root.child(
                Node::label(it_text, fs(9.5), pal.fg)
                    .halign(HAlign::Left)
                    .valign(VAlign::Top)
                    .margin(s(32.0), row_y + s(1.0), 0.0, 0.0),
            );
            if show_del {
                let dhov = hover == del_key;
                root = root.child(
                    Node::label("\u{f00d}", fs(9.0), if dhov { 0xf776_8eff } else { pal.fg })
                        .halign(HAlign::Left)
                        .valign(VAlign::Top)
                        .margin(w - s(20.0), row_y + s(1.0), 0.0, 0.0)
                        .icon(),
                );
            }
            root = root.child(Node::hit(tog_key).margin(s(12.0), row_y, s(12.0), h - row_y - s(24.0)));
            if show_del {
                root = root.child(Node::hit(del_key).margin(
                    w - s(24.0),
                    row_y - s(4.0),
                    s(2.0),
                    h - (row_y - s(4.0)) - s(22.0),
                ));
            }
        }
    }

    let rem = n.saturating_sub(start + drawn);
    if rem > 0 {
        let fy = s(36.0) + drawn as f32 * s(24.0) + s(2.0);
        root = root.child(
            Node::anchored(0.0, fy - h / 2.0)
                .text_(format!("+{rem} more"))
                .size_(fs(8.0))
                .color_(pal.fg),
        );
    }

    if !scene_owns_composer {
        let fy = h - s(38.0);
        let focused = input.is_some();
        let buf = input.unwrap_or("");
        root = root.child(
            Node::box_(
                s(13.0),
                crate::ui::mix(
                    crate::ui::hover(pal),
                    pal.fg,
                    if focused { 0.09 } else { 0.04 },
                ),
            )
            .margin(s(12.0), fy, s(12.0), h - fy - s(26.0)),
        );
        if focused {
            root = root.child(Node::stamp(vec![Cmd::Outline {
                x: s(12.0),
                y: fy,
                w: w - s(24.0),
                h: s(26.0),
                r: s(13.0),
                width: s(1.4),
                color: pal.acc,
            }]));
        }
        if buf.is_empty() && !focused {
            root = root.child(
                Node::label("Add a task\u{2026}", fs(9.5), pal.fg)
                    .halign(HAlign::Left)
                    .valign(VAlign::Top)
                    .margin(s(24.0), fy + s(7.0), 0.0, 0.0),
            );
        } else {
            if !buf.is_empty() {
                root = root.child(
                    Node::label(buf, fs(9.5), pal.fg)
                        .halign(HAlign::Left)
                        .valign(VAlign::Top)
                        .margin(s(24.0), fy + s(7.0), 0.0, 0.0),
                );
            }
            if focused {
                let cwx = buf.chars().count() as f64 * fs(9.5) as f64 * 0.62;
                root = root.child(Node::stamp(vec![Cmd::Rect {
                    x: (s(23.0) as f64 + cwx) as f32,
                    y: fy + s(6.0),
                    w: s(1.4),
                    h: s(14.0),
                    r: s(0.7),
                    color: pal.acc,
                }]));
                if !buf.bytes().all(|b| matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')) {
                    root = root.child(
                        Node::label("\u{f11c}", fs(8.5), pal.fg)
                            .halign(HAlign::Right)
                            .valign(VAlign::Top)
                            .margin(0.0, fy + s(8.0), s(22.0), 0.0)
                            .icon(),
                    );
                }
            }
        }
        root = root.child(Node::hit(input_key).margin(
            s(10.0),
            fy - s(4.0),
            s(10.0),
            h - (fy - s(4.0)) - s(34.0),
        ));
        if !focused {
            let pbx = w - s(20.0);
            let pby = h - s(18.0);
            let pb_hov = hover == add_key;
            root = root.child(
                Node::box_(
                    s(10.0),
                    if pb_hov {
                        crate::ui::hover_hl(pal)
                    } else {
                        crate::ui::hover(pal)
                    },
                )
                .margin(
                    w - s(30.0),
                    h - s(28.0),
                    s(10.0),
                    s(8.0),
                ),
            );
            root = root.child(
                Node::anchored(pbx - w / 2.0, (pby - s(4.5)) - h / 2.0)
                    .text_("\u{f067}")
                    .size_(fs(9.5))
                    .color_(if pb_hov { pal.acc } else { pal.fg })
                    .icon(),
            );
            root = root.child(Node::hit(add_key).margin(s(9.0), s(7.0), s(9.0), s(7.0)));
        }
    }
    root
}

/// The retired To-Do drawer — the byte-parity oracle for `todo`.
#[cfg(test)]
#[allow(clippy::too_many_arguments)]
fn todo_native(
    v: &mut Vec<Cmd>,
    todos: &[(String, bool)],
    scroll: usize,
    input: Option<&str>,
    scene_owns_header: bool,
    scene_owns_rows: bool,
    scene_owns_composer: bool,
    card_show_title: bool,
    toggle_key: u32,
    delete_key: u32,
    input_key: u32,
    add_key: u32,
    hover: u32,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    grid: f32,
    pal: &Pal,
) -> Vec<CardHit> {
    let gx = x as f64;
    let gy = y as f64;
    let gw = w as f64;
    let gh = h as f64;
    let g = grid as f64;
    let s = |n: f64| n * g;
    let fs = |n: f64| (n * g * 10.0).round() / 10.0;
    let n = todos.len();
    let done = todos.iter().filter(|(_, d)| *d).count();

    if !scene_owns_header && card_show_title {
        crate::ui::title(
            v,
            (gx + s(14.0)) as f32,
            (gy + s(9.0)) as f32,
            "To-Do",
            fs(11.5) as f32,
            pal.fg,
        );
    }
    if n > 0 {
        crate::ui::caption_r(
            v,
            (gx + gw - s(14.0)) as f32,
            (gy + s(11.0)) as f32,
            &format!("{done}/{n}"),
            fs(8.5) as f32,
            crate::ui::fg3(pal),
            false,
        );
    }

    // the module's window math, verbatim (limit clamps at 8 rows, `start`
    // clamps to limit − 1 — the committed module's own quirk)
    let limit = n.min(8);
    let start = scroll.min(limit.saturating_sub(1));
    let mut hits = Vec::new();
    let mut drawn = 0usize;

    if n == 0 {
        crate::ui::text_c(
            v,
            (gx + gw / 2.0) as f32,
            (gy + s(60.0)) as f32,
            "No tasks yet",
            fs(9.5) as f32,
            pal.fg,
            false,
        );
        crate::ui::text_c(
            v,
            (gx + gw / 2.0) as f32,
            (gy + s(78.0)) as f32,
            "\u{f256}",
            fs(16.0) as f32,
            crate::ui::mix(crate::ui::hover(pal), pal.fg, 0.10),
            true,
        );
    }

    if !scene_owns_rows {
        let bottom_line = gy + gh - s(40.0);
        for i in start..limit {
            let row_y = gy + s(36.0) + (i - start) as f64 * s(24.0);
            if row_y + s(24.0) > gy + gh + 0.5 {
                break;
            }
            if row_y + s(24.0) > bottom_line {
                break;
            }
            drawn += 1;
            let si = i - start;
            let (it_text, it_done) = &todos[i];
            let tog_key = toggle_key + si as u32;
            let del_key = delete_key + si as u32;
            let row_hover = hover == tog_key;
            let show_del = row_hover || hover == del_key;
            if *it_done {
                v.push(Cmd::Rect {
                    x: (gx + s(14.0)) as f32,
                    y: (row_y + s(1.0)) as f32,
                    w: s(11.0) as f32,
                    h: s(11.0) as f32,
                    r: s(3.5) as f32,
                    color: crate::ui::acc_tint(pal),
                });
                crate::ui::caption_r(
                    v,
                    (gx + s(26.5)) as f32,
                    (row_y + s(2.5)) as f32,
                    "\u{f00c}",
                    fs(8.0) as f32,
                    pal.acc,
                    true,
                );
            } else {
                v.push(Cmd::Outline {
                    x: (gx + s(14.0)) as f32,
                    y: (row_y + s(1.0)) as f32,
                    w: s(11.0) as f32,
                    h: s(11.0) as f32,
                    r: s(3.5) as f32,
                    width: s(1.2) as f32,
                    color: if row_hover {
                        pal.acc
                    } else {
                        crate::ui::mix(crate::ui::hover(pal), pal.fg, 0.25)
                    },
                });
            }
            crate::ui::text(
                v,
                (gx + s(32.0)) as f32,
                (row_y + s(1.0)) as f32,
                it_text,
                fs(9.5) as f32,
                pal.fg,
                false,
            );
            if show_del {
                let dhov = hover == del_key;
                crate::ui::text(
                    v,
                    (gx + gw - s(20.0)) as f32,
                    (row_y + s(1.0)) as f32,
                    "\u{f00d}",
                    fs(9.0) as f32,
                    if dhov { 0xf7768e_ff } else { pal.fg },
                    true,
                );
            }
            hits.push((
                (gx + s(12.0)) as f32,
                row_y as f32,
                (gw - s(24.0)) as f32,
                s(24.0) as f32,
                tog_key,
            ));
            if show_del {
                hits.push((
                    (gx + gw - s(24.0)) as f32,
                    (row_y - s(4.0)) as f32,
                    s(22.0) as f32,
                    s(22.0) as f32,
                    del_key,
                ));
            }
        }
    }

    let rem = n.saturating_sub(start + drawn);
    if rem > 0 {
        let fy = gy + s(36.0) + drawn as f64 * s(24.0) + s(2.0);
        crate::ui::text_c(
            v,
            (gx + gw / 2.0) as f32,
            fy as f32,
            &format!("+{rem} more"),
            fs(8.0) as f32,
            pal.fg,
            false,
        );
    }

    if !scene_owns_composer {
        let fy = gy + gh - s(38.0);
        let focused = input.is_some();
        let buf = input.unwrap_or("");
        v.push(Cmd::Rect {
            x: (gx + s(12.0)) as f32,
            y: fy as f32,
            w: (gw - s(24.0)) as f32,
            h: s(26.0) as f32,
            r: s(13.0) as f32,
            color: crate::ui::mix(
                crate::ui::hover(pal),
                pal.fg,
                if focused { 0.09 } else { 0.04 },
            ),
        });
        if focused {
            v.push(Cmd::Outline {
                x: (gx + s(12.0)) as f32,
                y: fy as f32,
                w: (gw - s(24.0)) as f32,
                h: s(26.0) as f32,
                r: s(13.0) as f32,
                width: s(1.4) as f32,
                color: pal.acc,
            });
        }
        if buf.is_empty() && !focused {
            crate::ui::text(
                v,
                (gx + s(24.0)) as f32,
                (fy + s(7.0)) as f32,
                "Add a task\u{2026}",
                fs(9.5) as f32,
                pal.fg,
                false,
            );
        } else {
            if !buf.is_empty() {
                crate::ui::text(
                    v,
                    (gx + s(24.0)) as f32,
                    (fy + s(7.0)) as f32,
                    buf,
                    fs(9.5) as f32,
                    pal.fg,
                    false,
                );
            }
            if focused {
                let cwx = buf.chars().count() as f64 * fs(9.5) * 0.62;
                v.push(Cmd::Rect {
                    x: (gx + s(23.0) + cwx) as f32,
                    y: (fy + s(6.0)) as f32,
                    w: s(1.4) as f32,
                    h: s(14.0) as f32,
                    r: s(0.7) as f32,
                    color: pal.acc,
                });
                if !buf.bytes().all(|b| matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')) {
                    crate::ui::text_r(
                        v,
                        (gx + gw - s(22.0)) as f32,
                        (fy + s(8.0)) as f32,
                        "\u{f11c}",
                        fs(8.5) as f32,
                        pal.fg,
                        true,
                    );
                }
            }
        }
        hits.push((
            (gx + s(10.0)) as f32,
            (fy - s(4.0)) as f32,
            (gw - s(20.0)) as f32,
            s(34.0) as f32,
            input_key,
        ));
        if !focused {
            let pbx = gx + gw - s(20.0);
            let pby = gy + gh - s(18.0);
            let pb_hov = hover == add_key;
            v.push(Cmd::Rect {
                x: (pbx - s(10.0)) as f32,
                y: (pby - s(10.0)) as f32,
                w: s(20.0) as f32,
                h: s(20.0) as f32,
                r: s(10.0) as f32,
                color: if pb_hov {
                    crate::ui::hover_hl(pal)
                } else {
                    crate::ui::hover(pal)
                },
            });
            crate::ui::text_c(
                v,
                pbx as f32,
                (pby - s(4.5)) as f32,
                "\u{f067}",
                fs(9.5) as f32,
                if pb_hov { pal.acc } else { pal.fg },
                true,
            );
            hits.push((
                (pbx - s(11.0)) as f32,
                (pby - s(11.0)) as f32,
                s(22.0) as f32,
                s(22.0) as f32,
                add_key,
            ));
        }
    }
    hits
}

/// The Notes card — sticky-record list / the title+body composer (the shared
/// `Shell::wrap_body` provides the exact line window the module's Lua
/// wrap re-created). Same f64-device discipline.
#[allow(clippy::too_many_arguments)]
pub(crate) fn notes(
    notes: &[(String, String)],
    title: &str,
    body: &str,
    input: Option<u8>,
    scroll: usize,
    scene_owns_header: bool,
    scene_owns_rows: bool,
    input_key: u32,
    save_key: u32,
    hover: u32,
    w: f32,
    h: f32,
    grid: f32,
    pal: &Pal,
) -> Node {
    let s = |n: f32| n * grid;
    let fs = |n: f32| (n * grid * 10.0).round() / 10.0;
    let mut root = Node::box_(0.0, 0);

    if !scene_owns_header {
        root = root.child(
            Node::label("Notes", fs(11.5), pal.fg)
                .font(Ff::Ui, Fw::Semibold)
                .halign(HAlign::Left)
                .valign(VAlign::Top)
                .margin(s(14.0), s(9.0), 0.0, 0.0),
        );
        root = root.child(
            Node::label(notes.len().to_string(), fs(8.5), crate::ui::fg3(pal))
                .font(Ff::Ui, Fw::Regular)
                .halign(HAlign::Right)
                .valign(VAlign::Top)
                .margin(0.0, s(11.0), s(14.0), 0.0),
        );
    }

    if let Some(stage) = input {
        let py = s(30.0);
        let ph = (h - s(30.0) - s(6.0)).max(20.0);
        root = root.child(
            Node::box_(s(10.0), crate::ui::mix(crate::ui::hover(pal), pal.fg, 0.06))
                .margin(s(8.0), py, s(8.0), h - py - ph),
        );

        let ty = py + s(19.0);
        let t_active = stage == 0;
        root = root.child(
            Node::label("Title", fs(8.0), crate::ui::fg3(pal))
                .halign(HAlign::Left)
                .valign(VAlign::Top)
                .margin(s(20.0), py + s(8.0), 0.0, 0.0),
        );
        root = root.child(
            Node::box_(
                s(6.0),
                if t_active {
                    crate::ui::hover_hl(pal)
                } else {
                    crate::ui::hover(pal)
                },
            )
            .margin(s(18.0), ty, s(18.0), h - ty - s(18.0)),
        );
        let t_shown: String = if title.is_empty() && !t_active {
            "Title\u{2026}".to_string()
        } else {
            title.chars().take(40).collect()
        };
        let t_ink = if t_active || title.is_empty() {
            pal.fg
        } else {
            crate::ui::fg2(pal)
        };
        root = root.child(
            Node::label(&t_shown, fs(9.0), t_ink)
                .halign(HAlign::Left)
                .valign(VAlign::Top)
                .margin(s(26.0), ty + s(4.0), 0.0, 0.0),
        );
        if t_active {
            let tcw = t_shown.chars().count() as f64 * fs(9.0) as f64 * 0.62;
            root = root.child(Node::stamp(vec![Cmd::Rect {
                x: (s(25.0) as f64 + tcw) as f32,
                y: ty + s(5.0),
                w: s(1.4),
                h: s(11.0),
                r: s(0.7),
                color: pal.acc,
            }]));
        }

        let b_active = stage == 1;
        let body_label_y = ty + s(18.0) + s(10.0);
        let by = body_label_y + s(11.0);
        let bh = (py + ph - s(24.0) - by).max(s(34.0));
        root = root.child(
            Node::label("Body", fs(8.0), crate::ui::fg3(pal))
                .halign(HAlign::Left)
                .valign(VAlign::Top)
                .margin(s(20.0), body_label_y, 0.0, 0.0),
        );
        root = root.child(
            Node::box_(
                s(8.0),
                if b_active {
                    crate::ui::hover_hl(pal)
                } else {
                    crate::ui::hover(pal)
                },
            )
            .margin(s(18.0), by, s(18.0), h - by - bh),
        );

        let fsb = fs(9.0);
        let lh = s(12.0);
        let max_chars =
            (((w as f64 - s(48.0) as f64) / (fsb as f64 * 0.62)).floor().max(12.0)) as usize;
        let lines = crate::shell::Shell::wrap_body(body, max_chars, 6);
        let fit = (((h as f64 + 0.5 - s(95.0) as f64) / lh as f64).floor().max(1.0)) as usize;
        let n = lines.len().min(fit);
        if n == 0 {
            if !b_active {
                root = root.child(
                    Node::label("Body\u{2026}", fsb, crate::ui::fg3(pal))
                        .halign(HAlign::Left)
                        .valign(VAlign::Top)
                        .margin(s(26.0), by + s(7.0), 0.0, 0.0),
                );
            } else {
                root = root.child(Node::stamp(vec![Cmd::Rect {
                    x: s(25.0),
                    y: by + s(8.0),
                    w: s(1.4),
                    h: s(11.0),
                    r: s(0.7),
                    color: pal.acc,
                }]));
            }
        } else {
            for (li, line) in lines.iter().take(n).enumerate() {
                let ly = by + s(7.0) + li as f32 * lh;
                root = root.child(
                    Node::label(line, fsb, if b_active { pal.fg } else { crate::ui::fg2(pal) })
                        .halign(HAlign::Left)
                        .valign(VAlign::Top)
                        .margin(s(26.0), ly, 0.0, 0.0),
                );
                if b_active && li + 1 == n {
                    let cx = s(25.0) as f64 + line.chars().count() as f64 * fsb as f64 * 0.62;
                    root = root.child(Node::stamp(vec![Cmd::Rect {
                        x: cx as f32,
                        y: ly + s(1.0),
                        w: s(1.4),
                        h: s(11.0),
                        r: s(0.7),
                        color: pal.acc,
                    }]));
                }
            }
        }

        if b_active {
            let sbx = w - s(22.0);
            let sby = py + ph - s(18.0);
            let hov = hover == save_key;
            root = root.child(
                Node::box_(
                    s(10.0),
                    if hov {
                        crate::ui::hover_hl(pal)
                    } else {
                        crate::ui::hover(pal)
                    },
                )
                .margin(sbx - s(10.0), sby - s(10.0), w - sbx - s(10.0), h - sby - s(10.0)),
            );
            root = root.child(
                Node::anchored(sbx - w / 2.0, (sby - s(4.5)) - h / 2.0)
                    .text_("\u{f067}")
                    .size_(fs(9.5))
                    .color_(if hov { pal.acc } else { pal.fg })
                    .icon(),
            );
            root = root.child(Node::hit(save_key).margin(
                sbx - s(11.0),
                sby - s(11.0),
                w - sbx - s(11.0),
                h - sby - s(11.0),
            ));
        }
        root = root.child(
            Node::label(
                "\u{21b5} next field \u{b7} \u{21b5} save \u{b7} esc cancel",
                fs(7.5),
                crate::ui::fg3(pal),
            )
            .halign(HAlign::Left)
            .valign(VAlign::Top)
            .margin(s(20.0), py + ph - s(16.0), 0.0, 0.0),
        );
        return root;
    }

    if !scene_owns_rows {
        let row_h = s(24.0);
        let visible =
            (((h - s(30.0) - s(34.0)) / row_h).floor().max(1.0)) as usize;
        let start = scroll.min(notes.len().saturating_sub(visible));
        for j in 0..visible {
            let Some((it_title, it_body)) = notes.get(start + j) else {
                break;
            };
            let ry = s(32.0) + j as f32 * row_h;
            root = root.child(
                Node::label("\u{f0f6}", fs(9.0), crate::ui::fg3(pal))
                    .halign(HAlign::Left)
                    .valign(VAlign::Top)
                    .margin(s(14.0), ry, 0.0, 0.0)
                    .icon(),
            );
            root = root.child(
                Node::label(it_title.chars().take(36).collect::<String>(), fs(9.5), pal.fg)
                    .halign(HAlign::Left)
                    .valign(VAlign::Top)
                    .margin(s(28.0), ry, 0.0, 0.0),
            );
            if !it_body.is_empty() {
                root = root.child(
                    Node::label(
                        it_body.chars().take(44).collect::<String>(),
                        fs(8.0),
                        crate::ui::fg2(pal),
                    )
                    .halign(HAlign::Left)
                    .valign(VAlign::Top)
                    .margin(s(36.0), ry + s(12.0), 0.0, 0.0),
                );
            }
            root = root.child(
                Node::box_(s(0.5), crate::ui::hover(pal))
                    .margin(s(12.0), ry + row_h - s(1.0), s(12.0), h - ry - row_h),
            );
        }
    }
    if notes.is_empty() {
        root = root.child(
            Node::anchored(0.0, -s(10.0))
                .text_("No notes")
                .size_(fs(9.5))
                .color_(pal.fg),
        );
    }
    let pbx = w - s(20.0);
    let pby = h - s(18.0);
    let hov = hover == input_key;
    root = root.child(
        Node::box_(
            s(10.0),
            if hov {
                crate::ui::hover_hl(pal)
            } else {
                crate::ui::hover(pal)
            },
        )
        .margin(w - s(30.0), h - s(28.0), s(10.0), s(8.0)),
    );
    root = root.child(
        Node::anchored(pbx - w / 2.0, (pby - s(4.5)) - h / 2.0)
            .text_("\u{f067}")
            .size_(fs(9.5))
            .color_(if hov { pal.acc } else { pal.fg })
            .icon(),
    );
    root = root.child(Node::hit(input_key).margin(
        w - s(31.0),
        h - s(29.0),
        s(9.0),
        s(7.0),
    ));
    root
}

/// The retired Notes drawer — the byte-parity oracle for `notes`.
#[cfg(test)]
#[allow(clippy::too_many_arguments)]
fn notes_native(
    v: &mut Vec<Cmd>,
    notes: &[(String, String)],
    title: &str,
    body: &str,
    input: Option<u8>,
    scroll: usize,
    scene_owns_header: bool,
    scene_owns_rows: bool,
    input_key: u32,
    save_key: u32,
    hover: u32,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    grid: f32,
    pal: &Pal,
) -> Vec<CardHit> {
    let gx = x as f64;
    let gy = y as f64;
    let gw = w as f64;
    let gh = h as f64;
    let g = grid as f64;
    let s = |n: f64| n * g;
    let fs = |n: f64| (n * g * 10.0).round() / 10.0;
    let mut hits = Vec::new();

    // shared card header (matches card_header, pad 14) unless the scene owns it
    if !scene_owns_header {
        crate::ui::title(
            v,
            (gx + s(14.0)) as f32,
            (gy + s(9.0)) as f32,
            "Notes",
            fs(11.5) as f32,
            pal.fg,
        );
        crate::ui::caption_r(
            v,
            (gx + gw - s(14.0)) as f32,
            (gy + s(11.0)) as f32,
            &notes.len().to_string(),
            fs(8.5) as f32,
            crate::ui::fg3(pal),
            false,
        );
    }

    // ── the composer (title + body stages, input == Some(0) / Some(1)) ──
    if let Some(stage) = input {
        let py = gy + s(30.0);
        let ph = (gh - s(30.0) - s(6.0)).max(20.0);
        v.push(Cmd::Rect {
            x: (gx + s(8.0)) as f32,
            y: py as f32,
            w: (gw - s(16.0)) as f32,
            h: ph as f32,
            r: s(10.0) as f32,
            color: crate::ui::mix(crate::ui::hover(pal), pal.fg, 0.06),
        });

        // title field — single line
        let ty = py + s(19.0);
        let t_active = stage == 0;
        crate::ui::text(
            v,
            (gx + s(20.0)) as f32,
            (py + s(8.0)) as f32,
            "Title",
            fs(8.0) as f32,
            crate::ui::fg3(pal),
            false,
        );
        v.push(Cmd::Rect {
            x: (gx + s(18.0)) as f32,
            y: ty as f32,
            w: (gw - s(36.0)) as f32,
            h: s(18.0) as f32,
            r: s(6.0) as f32,
            color: if t_active {
                crate::ui::hover_hl(pal)
            } else {
                crate::ui::hover(pal)
            },
        });
        let t_shown: String = if title.is_empty() && !t_active {
            "Title\u{2026}".to_string()
        } else {
            title.chars().take(40).collect()
        };
        let t_ink = if t_active || title.is_empty() {
            pal.fg
        } else {
            crate::ui::fg2(pal)
        };
        crate::ui::text(
            v,
            (gx + s(26.0)) as f32,
            (ty + s(4.0)) as f32,
            &t_shown,
            fs(9.0) as f32,
            t_ink,
            false,
        );
        if t_active {
            let tcw = t_shown.chars().count() as f64 * fs(9.0) * 0.62;
            v.push(Cmd::Rect {
                x: (gx + s(25.0) + tcw) as f32,
                y: (ty + s(5.0)) as f32,
                w: s(1.4) as f32,
                h: s(11.0) as f32,
                r: s(0.7) as f32,
                color: pal.acc,
            });
        }

        // body field — a tall multi-line area
        let b_active = stage == 1;
        let body_label_y = ty + s(18.0) + s(10.0);
        let by = body_label_y + s(11.0);
        let bh = (py + ph - s(24.0) - by).max(s(34.0));
        crate::ui::text(
            v,
            (gx + s(20.0)) as f32,
            body_label_y as f32,
            "Body",
            fs(8.0) as f32,
            crate::ui::fg3(pal),
            false,
        );
        v.push(Cmd::Rect {
            x: (gx + s(18.0)) as f32,
            y: by as f32,
            w: (gw - s(36.0)) as f32,
            h: bh as f32,
            r: s(8.0) as f32,
            color: if b_active {
                crate::ui::hover_hl(pal)
            } else {
                crate::ui::hover(pal)
            },
        });

        let fsb = fs(9.0);
        let lh = s(12.0);
        let max_chars = (((gw - s(48.0)) / (fsb * 0.62)).floor().max(12.0)) as usize;
        let lines = crate::shell::Shell::wrap_body(body, max_chars, 6);
        // window like the scene's Rows: clip a line once ry+row_h leaves the card
        let fit = (((gh + 0.5 - s(95.0)) / lh).floor().max(1.0)) as usize;
        let n = lines.len().min(fit);
        if n == 0 {
            if !b_active {
                crate::ui::text(
                    v,
                    (gx + s(26.0)) as f32,
                    (by + s(7.0)) as f32,
                    "Body\u{2026}",
                    fsb as f32,
                    crate::ui::fg3(pal),
                    false,
                );
            } else {
                v.push(Cmd::Rect {
                    x: (gx + s(25.0)) as f32,
                    y: (by + s(8.0)) as f32,
                    w: s(1.4) as f32,
                    h: s(11.0) as f32,
                    r: s(0.7) as f32,
                    color: pal.acc,
                });
            }
        } else {
            for (li, line) in lines.iter().take(n).enumerate() {
                let ly = by + s(7.0) + li as f64 * lh;
                crate::ui::text(
                    v,
                    (gx + s(26.0)) as f32,
                    ly as f32,
                    line,
                    fsb as f32,
                    if b_active { pal.fg } else { crate::ui::fg2(pal) },
                    false,
                );
                if b_active && li + 1 == n {
                    let cx = gx + s(25.0) + line.chars().count() as f64 * fsb * 0.62;
                    v.push(Cmd::Rect {
                        x: cx as f32,
                        y: (ly + s(1.0)) as f32,
                        w: s(1.4) as f32,
                        h: s(11.0) as f32,
                        r: s(0.7) as f32,
                        color: pal.acc,
                    });
                }
            }
        }

        // save `+` — body stage only
        if b_active {
            let sbx = gx + gw - s(22.0);
            let sby = py + ph - s(18.0);
            let hov = hover == save_key;
            v.push(Cmd::Rect {
                x: (sbx - s(10.0)) as f32,
                y: (sby - s(10.0)) as f32,
                w: s(20.0) as f32,
                h: s(20.0) as f32,
                r: s(10.0) as f32,
                color: if hov {
                    crate::ui::hover_hl(pal)
                } else {
                    crate::ui::hover(pal)
                },
            });
            crate::ui::text_c(
                v,
                sbx as f32,
                (sby - s(4.5)) as f32,
                "\u{f067}",
                fs(9.5) as f32,
                if hov { pal.acc } else { pal.fg },
                true,
            );
            hits.push((
                (sbx - s(11.0)) as f32,
                (sby - s(11.0)) as f32,
                s(22.0) as f32,
                s(22.0) as f32,
                save_key,
            ));
        }
        crate::ui::text(
            v,
            (gx + s(20.0)) as f32,
            (py + ph - s(16.0)) as f32,
            "\u{21b5} next field \u{b7} \u{21b5} save \u{b7} esc cancel",
            fs(7.5) as f32,
            crate::ui::fg3(pal),
            false,
        );
        return hits;
    }

    // ── list stage ──
    if !scene_owns_rows {
        let row_h = s(24.0);
        let visible = (((gh - s(30.0) - s(34.0)) / row_h).floor().max(1.0)) as usize;
        let start = scroll.min(notes.len().saturating_sub(visible));
        for j in 0..visible {
            let Some((it_title, it_body)) = notes.get(start + j) else {
                break;
            };
            let ry = gy + s(32.0) + j as f64 * row_h;
            crate::ui::text(
                v,
                (gx + s(14.0)) as f32,
                ry as f32,
                "\u{f0f6}",
                fs(9.0) as f32,
                crate::ui::fg3(pal),
                true,
            );
            crate::ui::text(
                v,
                (gx + s(28.0)) as f32,
                ry as f32,
                &it_title.chars().take(36).collect::<String>(),
                fs(9.5) as f32,
                pal.fg,
                false,
            );
            if !it_body.is_empty() {
                crate::ui::text(
                    v,
                    (gx + s(36.0)) as f32,
                    (ry + s(12.0)) as f32,
                    &it_body.chars().take(44).collect::<String>(),
                    fs(8.0) as f32,
                    crate::ui::fg2(pal),
                    false,
                );
            }
            v.push(Cmd::Rect {
                x: (gx + s(12.0)) as f32,
                y: (ry + row_h - s(1.0)) as f32,
                w: (gw - s(24.0)) as f32,
                h: s(1.0) as f32,
                r: s(0.5) as f32,
                color: crate::ui::hover(pal),
            });
        }
    }
    if notes.is_empty() {
        crate::ui::text_c(
            v,
            (gx + gw / 2.0) as f32,
            (gy + gh / 2.0 - s(10.0)) as f32,
            "No notes",
            fs(9.5) as f32,
            pal.fg,
            false,
        );
    }
    // add `+` — bottom-right corner
    let pbx = gx + gw - s(20.0);
    let pby = gy + gh - s(18.0);
    let hov = hover == input_key;
    v.push(Cmd::Rect {
        x: (pbx - s(10.0)) as f32,
        y: (pby - s(10.0)) as f32,
        w: s(20.0) as f32,
        h: s(20.0) as f32,
        r: s(10.0) as f32,
        color: if hov {
            crate::ui::hover_hl(pal)
        } else {
            crate::ui::hover(pal)
        },
    });
    crate::ui::text_c(
        v,
        pbx as f32,
        (pby - s(4.5)) as f32,
        "\u{f067}",
        fs(9.5) as f32,
        if hov { pal.acc } else { pal.fg },
        true,
    );
    hits.push((
        (pbx - s(11.0)) as f32,
        (pby - s(11.0)) as f32,
        s(22.0) as f32,
        s(22.0) as f32,            input_key,
        ));
    hits
}

/// The Custom Accents browser — header + view toggle + search bar, then the
/// list OR grid view with its right-edge scrollbar. The filtered rows, the
/// active selection and the layout constants ride in via the api.
#[allow(clippy::too_many_arguments)]
pub(crate) fn customacc(
    v: &mut Vec<Cmd>,
    total: usize,
    rows: &[(String, u32)],
    search_active: bool,
    query: &str,
    list_view: bool,
    scroll: usize,
    sel_name: &str,
    cur_is_c: bool,
    has_rows: bool,
    empty_msg: &str,
    idle_bg: u32,
    ic_sticky: &str,
    ic_grid: &str,
    ic_list: &str,
    ic_search: &str,
    ic_check: &str,
    card_show_title: bool,
    card_show_glyph: bool,
    view_key: u32,
    search_key: u32,
    key_base: u32,
    hover: u32,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    grid: f32,
    pal: &Pal,
) -> Vec<CardHit> {
    let gx = x as f64;
    let gy = y as f64;
    let gw = w as f64;
    let gh = h as f64;
    let g = grid as f64;
    let s = |n: f64| n * g;
    let fs = |n: f64| (n * g * 10.0).round() / 10.0;
    let pad = s(12.0);
    let mut hits = Vec::new();

    // ── header: sticky glyph + title + mono count (the shared card_head) ──
    if card_show_glyph {
        crate::ui::text(
            v,
            (gx + pad) as f32,
            (gy + s(8.0)) as f32,
            ic_sticky,
            fs(12.0) as f32,
            crate::ui::fg2(pal),
            true,
        );
    }
    let tx = if card_show_glyph {
        gx + pad + s(17.0)
    } else {
        gx + pad
    };
    if card_show_title {
        crate::ui::title(
            v,
            tx as f32,
            (gy + s(9.0)) as f32,
            "Custom Accents",
            fs(11.5) as f32,
            pal.fg,
        );
    }
    crate::ui::text_r_mono(
        v,
        (gx + gw - pad) as f32,
        (gy + s(11.0)) as f32,
        &total.to_string(),
        fs(8.5) as f32,
        crate::ui::fg3(pal),
    );

    // ── top-right corner: list / grid view toggle (header row) ──
    let ts = s(20.0);
    let txx = gx + gw - s(28.0);
    let ty = gy + s(8.0);
    v.push(Cmd::Rect {
        x: txx as f32,
        y: ty as f32,
        w: ts as f32,
        h: ts as f32,
        r: s(6.0) as f32,
        color: if hover == view_key {
            crate::ui::hover(pal)
        } else {
            idle_bg
        },
    });
    let ticon = if list_view { ic_grid } else { ic_list };
    crate::ui::text_c(
        v,
        (txx + ts / 2.0) as f32,
        (ty + ts / 2.0 - s(2.0)) as f32,
        ticon,
        fs(12.0) as f32,
        if hover == view_key { pal.acc } else { crate::ui::fg2(pal) },
        true,
    );
    hits.push((
        (txx - s(3.0)) as f32,
        (ty - s(3.0)) as f32,
        (ts + s(6.0)) as f32,
        (ts + s(6.0)) as f32,
        view_key,
    ));

    // ── search bar at the top-middle (below the header row) ──
    let sy = gy + s(30.0);
    let sh = s(crate::shell::CUSTOMACC_SEARCH_H as f64);
    let sw = gw - s(20.0);
    let sbg = if search_active {
        crate::ui::acc_tint(pal)
    } else if hover == search_key {
        crate::ui::hover(pal)
    } else {
        idle_bg
    };
    v.push(Cmd::Rect {
        x: (gx + s(10.0)) as f32,
        y: sy as f32,
        w: sw as f32,
        h: sh as f32,
        r: (sh / 2.0) as f32,
        color: sbg,
    });
    crate::ui::text(
        v,
        (gx + s(24.0)) as f32,
        (sy + s(7.0)) as f32,
        ic_search,
        fs(11.0) as f32,
        if search_active { pal.acc } else { crate::ui::fg3(pal) },
        true,
    );
    crate::ui::text(
        v,
        (gx + s(40.0)) as f32,
        (sy + s(7.5)) as f32,
        query,
        fs(9.5) as f32,
        if query.is_empty() {
            crate::ui::fg3(pal)
        } else {
            pal.fg
        },
        false,
    );
    if query.is_empty() && !search_active {
        crate::ui::text(
            v,
            (gx + s(40.0)) as f32,
            (sy + s(7.5)) as f32,
            "Search custom accents\u{2026}",
            fs(9.5) as f32,
            crate::ui::fg3(pal),
            false,
        );
    }
    hits.push((
        (gx + s(10.0)) as f32,
        sy as f32,
        sw as f32,
        sh as f32,
        search_key,
    ));

    // ── content viewport (the SAME box card_side_effects publishes) ──
    let cxo = gx + s(10.0);
    let cyo = gy + s(crate::shell::CUSTOMACC_CONTENT_Y as f64);
    let cw = gw - s(20.0);
    let chh = (gy + gh - s(3.0) - cyo).max(s(8.0));

    let nrows = rows.len();
    if !has_rows {
        crate::ui::caption_c(
            v,
            (gx + gw / 2.0) as f32,
            (gy + gh / 2.0 - s(6.0)) as f32,
            empty_msg,
            fs(9.0) as f32,
            crate::ui::fg3(pal),
            false,
        );
        return hits;
    }

    let is_sel = |name: &str| cur_is_c && name == sel_name;

    let sb_scroll_color = crate::ui::mix(pal.acc, pal.bg, 0.35);
    if list_view {
        // ── list view: one row per accent (swatch, name, check active) ──
        let rh = s(crate::shell::CUSTOMACC_ROW_H as f64);
        let visible = (((chh - s(2.0)) / rh).floor().max(1.0)) as usize;
        for j in 0..visible {
            let Some((rname, rswatch)) = rows.get(scroll + j) else {
                break;
            };
            let key = key_base + (scroll + j) as u32;
            let hov = hover == key;
            let isel = is_sel(rname);
            let rj = cyo + j as f64 * rh;
            if isel {
                v.push(Cmd::Rect {
                    x: cxo as f32,
                    y: (rj + rh / 2.0 - s(6.0)) as f32,
                    w: s(3.0) as f32,
                    h: s(12.0) as f32,
                    r: s(1.5) as f32,
                    color: pal.acc,
                });
            }
            v.push(Cmd::Rect {
                x: (cxo + s(10.0)) as f32,
                y: (rj + (rh - s(14.0)) / 2.0) as f32,
                w: s(14.0) as f32,
                h: s(14.0) as f32,
                r: s(4.0) as f32,
                color: *rswatch,
            });
            crate::ui::text(
                v,
                (cxo + s(32.0)) as f32,
                (rj + s(6.5)) as f32,
                &rname.chars().take(30).collect::<String>(),
                fs(9.5) as f32,
                if hov { pal.acc } else { pal.fg },
                false,
            );
            if isel {
                crate::ui::text_r(
                    v,
                    (cxo + cw - s(4.0)) as f32,
                    (rj + s(6.5)) as f32,
                    ic_check,
                    fs(10.0) as f32,
                    pal.acc,
                    true,
                );
            }
            hits.push((cxo as f32, rj as f32, cw as f32, rh as f32, key));
        }
        if nrows > visible {
            let bar_max = chh - s(4.0);
            let bar_h = (bar_max * visible as f64 / nrows as f64).clamp(s(12.0), bar_max);
            let ratio = scroll as f64 / (nrows - visible) as f64;
            let bar_y = cyo + s(2.0) + ratio * (bar_max - bar_h);
            v.push(Cmd::Rect {
                x: (gx + gw - s(6.0)) as f32,
                y: bar_y as f32,
                w: s(2.0) as f32,
                h: bar_h as f32,
                r: s(1.0) as f32,
                color: sb_scroll_color,
            });
        }
    } else {
        // ── grid view: labeled color circles ──
        let cell_const = crate::shell::CUSTOMACC_CELL as f64;
        let gap_const = crate::shell::CUSTOMACC_GAP_X as f64;
        let pitch_const = crate::shell::CUSTOMACC_PITCH_Y as f64;
        let cols = (((cw - s(8.0)) / s(cell_const + gap_const)).floor().max(1.0)) as usize;
        let grows = (((chh + s(2.0)) / s(pitch_const)).floor().max(1.0)) as usize;
        let cell = s(cell_const);
        let gapx = s(gap_const);
        let pitch = s(pitch_const);
        let gx0 = cxo + s(4.0);
        'rows: for r in 0..grows {
            for c in 0..cols {
                let idx = scroll + r * cols + c;
                let Some((rname, rswatch)) = rows.get(idx) else {
                    break 'rows;
                };
                let key = key_base + idx as u32;
                let hov = hover == key;
                let isel = is_sel(rname);
                let cx0 = gx0 + c as f64 * (cell + gapx);
                let cy0 = cyo + s(2.0) + r as f64 * pitch;
                if isel || hov {
                    v.push(Cmd::Rect {
                        x: (cx0 - s(3.0)) as f32,
                        y: (cy0 - s(3.0)) as f32,
                        w: (cell + s(6.0)) as f32,
                        h: (cell + s(6.0)) as f32,
                        r: ((cell + s(6.0)) / 2.0) as f32,
                        color: if isel {
                            crate::ui::mix(pal.acc, pal.bg, 0.35)
                        } else {
                            crate::ui::hover(pal)
                        },
                    });
                }
                v.push(Cmd::Rect {
                    x: cx0 as f32,
                    y: cy0 as f32,
                    w: cell as f32,
                    h: cell as f32,
                    r: (cell / 2.0) as f32,
                    color: *rswatch,
                });
                if isel {
                    crate::ui::text(
                        v,
                        (cx0 + cell / 2.0 - s(5.0)) as f32,
                        (cy0 + cell / 2.0 - s(8.0)) as f32,
                        ic_check,
                        fs(11.0) as f32,
                        pal.sfg,
                        true,
                    );
                }
                crate::ui::text(
                    v,
                    (cx0 - s(4.0)) as f32,
                    (cy0 + cell + s(1.0)) as f32,
                    &rname.chars().take(9).collect::<String>(),
                    fs(8.5) as f32,
                    if isel { pal.sfg } else { pal.fg },
                    false,
                );
                hits.push((
                    (cx0 - s(4.0)) as f32,
                    (cy0 - s(4.0)) as f32,
                    (cell + s(8.0)) as f32,
                    (cell + s(12.0)) as f32,
                    key,
                ));
            }
        }
        if nrows > cols * grows {
            let bar_max = chh - s(4.0);
            let total_rows = nrows.div_ceil(cols);
            let bar_h = (bar_max * grows as f64 / total_rows as f64).clamp(s(12.0), bar_max);
            let ratio = (scroll / cols) as f64 / (total_rows - grows) as f64;
            let bar_y = cyo + s(2.0) + ratio * (bar_max - bar_h);
            v.push(Cmd::Rect {
                x: (gx + gw - s(6.0)) as f32,
                y: bar_y as f32,
                w: s(2.0) as f32,
                h: bar_h as f32,
                r: s(1.0) as f32,
                color: sb_scroll_color,
            });
        }
    }
    hits
}

/// The Audio Recorder card — pane 0: the mic fader, record button and the
/// "›" chevron; pane 1: the recordings list and the two switch rows. Layout
/// constants (row_h / rec_pad_bottom) come from the recorder config; the
/// colors have been resolved for the frame.
#[allow(clippy::too_many_arguments)]
pub(crate) fn audiorec(
    v: &mut Vec<Cmd>,
    pane: usize,
    show_glyph: bool,
    show_title: bool,
    mic_show: bool,
    mic_muted: bool,
    mic_level: f32,
    recording: bool,
    rec_secs: u32,
    saved_count: usize,
    confirm_delete: bool,
    armed: Option<usize>,
    rows: &[String],
    row_h_cfg: f32,
    rec_pad_bottom: f32,
    colors: &[u32; 10],
    ic_mic: &str,
    ic_mic_off: &str,
    ic_forward: &str,
    ic_back: &str,
    ic_close_fill: &str,
    ic_recycle: &str,
    ic_play: &str,
    dots_show: bool,
    hover: u32,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    grid: f32,
    pal: &Pal,
) -> Vec<CardHit> {
    let gx = x as f64;
    let gy = y as f64;
    let gw = w as f64;
    let gh = h as f64;
    let g = grid as f64;
    let s = |n: f64| n * g;
    let fs = |n: f64| (n * g * 10.0).round() / 10.0;
    let pad = s(12.0);
    let hdr = s(22.0);
    let mut hits = Vec::new();
    let [rec_bg_idle, rec_bg_hov, rec_active, rec_dot, row_ink, row_hover_ink, play_hover, del_hover, del_armed, mic_fill] =
        *colors;
    let pane_key = crate::shell::AUDIO_REC_PANE_KEY;
    let back_key = crate::shell::AUDIO_REC_BACK_KEY;
    let mic_key = crate::shell::AUDIO_REC_MIC_KEY;
    let rec_key = crate::shell::AUDIO_REC_REC_KEY;
    let confirm_key = crate::shell::AUDIO_REC_CONFIRM_KEY;
    let showmic_key = crate::shell::AUDIO_REC_SHOWMIC_KEY;
    let hov = |key: u32| hover == key;

    if pane == 1 {
        // ── pane 1: "‹" back · recordings list (play / delete) + toggles ──
        let cw = s(16.0);
        let cx = gx + pad;
        let cy = gy + s(6.0);
        let hb = hov(back_key);
        v.push(Cmd::Rect {
            x: cx as f32,
            y: cy as f32,
            w: cw as f32,
            h: cw as f32,
            r: s(5.0) as f32,
            color: if hb {
                crate::ui::raised_hl(pal)
            } else {
                crate::ui::raised(pal)
            },
        });
        crate::ui::text_c(
            v,
            (cx + cw / 2.0) as f32,
            (cy + s(3.0)) as f32,
            ic_back,
            fs(10.0) as f32,
            if hb { pal.acc } else { pal.fg },
            true,
        );
        hits.push((
            (cx - s(2.0)) as f32,
            (cy - s(2.0)) as f32,
            (cw + s(4.0)) as f32,
            (cw + s(4.0)) as f32,
            back_key,
        ));

        let tx = gx + pad + cw + s(6.0);
        crate::ui::title(
            v,
            tx as f32,
            (gy + s(9.0)) as f32,
            "Recordings",
            fs(11.5) as f32,
            pal.fg,
        );

        let rows_top = gy + hdr + s(4.0);
        let rows_bot = gy + gh - s(44.0);
        let list_h = (rows_bot - rows_top).max(s(8.0));
        let row_h = s(row_h_cfg as f64);
        let n_fit = ((list_h / row_h).floor().max(1.0)) as usize;
        let n = rows.len().min(n_fit);

        if n == 0 {
            let eh = (rows_bot - rows_top).max(s(10.0));
            crate::ui::caption_c(
                v,
                ((gx + gw / 2.0)) as f32,
                (gy + rows_top + eh / 2.0 - s(6.0)) as f32,
                "no recordings yet",
                fs(9.0) as f32,
                crate::ui::fg3(pal),
                false,
            );
        }

        let mut ry = rows_top;
        for i in 0..n {
            let name = &rows[i];
            let pkey = (crate::shell::AUDIO_REC_PLAY_BASE + i as u32)
                .min(crate::shell::AUDIO_REC_PLAY_MAX);
            let dkey = (crate::shell::AUDIO_REC_DEL_BASE + i as u32)
                .min(crate::shell::AUDIO_REC_DEL_MAX);
            let is_armed = armed == Some(i);
            let rhover = hov(pkey) || hov(dkey);
            let del_ink = if is_armed {
                crate::ui::mix(pal.fg, del_armed, 0.25)
            } else {
                row_ink
            };
            if rhover || is_armed {
                v.push(Cmd::Rect {
                    x: (gx + pad) as f32,
                    y: ry as f32,
                    w: (gw - pad * 2.0) as f32,
                    h: s(row_h_cfg as f64) as f32,
                    r: s(5.0) as f32,
                    color: if is_armed { del_armed } else { row_hover_ink },
                });
            }
            // file name (clamped by char count)
            let n_chars = (((gw - pad * 2.0 - s(56.0)) / s(5.4)).floor().max(4.0)) as usize;
            let shown: &str = if name.len() > n_chars {
                let mut end = n_chars;
                while end > 0 && !name.is_char_boundary(end) {
                    end -= 1;
                }
                &name[..end]
            } else {
                name
            };
            crate::ui::caption(
                v,
                (gx + pad + s(2.0)) as f32,
                (ry + s(5.0)) as f32,
                shown,
                fs(8.5) as f32,
                del_ink,
                true,
            );
            // delete glyph (left of play) — armed rows show a red confirm dot
            let d_gx = gx + gw - pad - s(30.0);
            if is_armed {
                crate::ui::text(
                    v,
                    d_gx as f32,
                    (ry + s(5.0)) as f32,
                    ic_close_fill,
                    fs(9.0) as f32,
                    crate::ui::mix(del_armed, pal.fg, 0.2),
                    true,
                );
            } else {
                crate::ui::text(
                    v,
                    d_gx as f32,
                    (ry + s(5.0)) as f32,
                    ic_recycle,
                    fs(9.0) as f32,
                    if hov(dkey) { del_hover } else { crate::ui::fg3(pal) },
                    true,
                );
            }
            hits.push((
                (d_gx - s(3.0)) as f32,
                (ry - s(2.0)) as f32,
                s(16.0) as f32,
                (s(row_h_cfg as f64) + s(2.0)) as f32,
                dkey,
            ));
            // play glyph (far right)
            let p_gx = gx + gw - pad - s(12.0);
            crate::ui::text(
                v,
                p_gx as f32,
                (ry + s(5.0)) as f32,
                ic_play,
                fs(9.0) as f32,
                if hov(pkey) { play_hover } else { crate::ui::fg3(pal) },
                true,
            );
            hits.push((
                (p_gx - s(3.0)) as f32,
                (ry - s(2.0)) as f32,
                s(16.0) as f32,
                (s(row_h_cfg as f64) + s(2.0)) as f32,
                pkey,
            ));
            ry += s(row_h_cfg as f64);
        }

        // bottom band: Confirm delete + Show mic slider switch rows
        let tog_w = gw - pad * 2.0;
        let tog_h = s(18.0);
        let mut ty = gy + gh - tog_h * 2.0 - s(2.0);
        for (tog_key, tog_label, tog_on) in [
            (confirm_key, "Confirm delete", confirm_delete),
            (showmic_key, "Show mic slider", mic_show),
        ] {
            if hov(tog_key) {
                v.push(Cmd::Rect {
                    x: (gx + pad) as f32,
                    y: ty as f32,
                    w: tog_w as f32,
                    h: tog_h as f32,
                    r: s(4.0) as f32,
                    color: crate::ui::hover(pal),
                });
            }
            crate::ui::caption(
                v,
                (gx + pad + s(12.0)) as f32,
                (ty + s(5.0)) as f32,
                tog_label,
                fs(9.0) as f32,
                pal.fg,
                false,
            );
            // switch track + knob
            let sw_w = s(24.0);
            let sw_h = s(12.0);
            let swx = gx + gw - pad - sw_w;
            let swy = ty + (tog_h - sw_h) / 2.0;
            v.push(Cmd::Rect {
                x: swx as f32,
                y: swy as f32,
                w: sw_w as f32,
                h: sw_h as f32,
                r: (sw_h / 2.0) as f32,
                color: crate::ui::mix(pal.bg, pal.fg, if tog_on { 0.45 } else { 0.28 }),
            });
            let kn = sw_h - s(4.0);
            let kx = if tog_on {
                swx + sw_w - kn - s(2.0)
            } else {
                swx + s(2.0)
            };
            v.push(Cmd::Rect {
                x: kx as f32,
                y: (ty + (tog_h - sw_h) / 2.0 + s(2.0)) as f32,
                w: kn as f32,
                h: kn as f32,
                r: (kn / 2.0) as f32,
                color: pal.fg,
            });
            hits.push((
                (gx + pad) as f32,
                (ty - s(1.0)) as f32,
                tog_w as f32,
                (tog_h + s(2.0)) as f32,
                tog_key,
            ));
            ty += tog_h + s(2.0);
        }
    } else {
        // ── pane 0: mic fader, record button, "›" chevron ──
        let mut tx = gx + pad;
        if show_glyph {
            crate::ui::text(
                v,
                (gx + pad) as f32,
                (gy + s(8.0)) as f32,
                ic_mic,
                fs(12.0) as f32,
                crate::ui::fg2(pal),
                true,
            );
            tx = gx + pad + s(17.0);
        }
        if show_title {
            crate::ui::title(v, tx as f32, (gy + s(9.0)) as f32, "Recorder", fs(11.5) as f32, pal.fg);
        }
        let meta = if recording {
            format!(
                "\u{25cf}  {:02}:{:02}",
                rec_secs / 60,
                rec_secs % 60
            )
        } else if saved_count > 0 {
            format!("{saved_count} saved")
        } else {
            String::new()
        };
        if !meta.is_empty() {
            crate::ui::text_r_mono(
                v,
                (gx + gw - pad) as f32,
                (gy + s(11.0)) as f32,
                &meta,
                fs(8.5) as f32,
                crate::ui::fg3(pal),
            );
        }

        // "›" chevron button top-right → recordings pane
        let cw = s(16.0);
        let cx = gx + gw - pad - cw;
        let cy = gy + s(6.0);
        let hp = hov(pane_key);
        v.push(Cmd::Rect {
            x: cx as f32,
            y: cy as f32,
            w: cw as f32,
            h: cw as f32,
            r: s(5.0) as f32,
            color: if hp {
                crate::ui::raised_hl(pal)
            } else {
                crate::ui::raised(pal)
            },
        });
        crate::ui::text_c(
            v,
            (cx + cw / 2.0) as f32,
            (cy + s(3.0)) as f32,
            ic_forward,
            fs(10.0) as f32,
            if hp { pal.acc } else { pal.fg },
            true,
        );
        hits.push((
            (cx - s(2.0)) as f32,
            (cy - s(2.0)) as f32,
            (cw + s(4.0)) as f32,
            (cw + s(4.0)) as f32,
            pane_key,
        ));

        // mic volume fader (hidden by the pane-1 "Show mic slider" toggle)
        let mut ry = gy + hdr + s(6.0);
        if mic_show {
            let row_h = s(24.0);
            let mic_hov = hov(mic_key);
            crate::ui::text(
                v,
                (gx + pad) as f32,
                (ry + s(5.0)) as f32,
                if mic_muted { ic_mic_off } else { ic_mic },
                fs(12.0) as f32,
                if mic_hov { pal.acc } else { pal.fg },
                true,
            );
            let ftx = gx + pad + s(20.0);
            let ftw = (gw - pad * 2.0 - s(20.0) - s(46.0)).max(s(40.0));
            let fcy = ry + row_h * 0.45;
            crate::ui::fader(
                v,
                ftx as f32,
                fcy as f32,
                ftw as f32,
                mic_level,
                mic_fill,
                mic_hov,
                pal,
            );
            let lbl = if mic_muted {
                "Muted".to_string()
            } else {
                format!("{:3}%", (mic_level as f64 * 100.0 + 0.5).floor())
            };
            crate::ui::text_r(
                v,
                (gx + gw - pad) as f32,
                (fcy - s(5.0)) as f32,
                &lbl,
                fs(9.5) as f32,
                if mic_muted {
                    crate::ui::fg3(pal)
                } else {
                    pal.fg
                },
                false,
            );
            hits.push((
                (gx + s(6.0)) as f32,
                ry as f32,
                (gw - s(12.0)) as f32,
                row_h as f32,
                mic_key,
            ));
            ry += row_h + s(6.0);
            // hint under the fader (hidden while recording)
            if !recording {
                let hint = if mic_muted {
                    "mic muted \u{2014} recording will capture silence"
                } else {
                    "mic level drives wpctl mic volume"
                };
                crate::ui::caption(
                    v,
                    (gx + pad) as f32,
                    (ry + s(4.0)) as f32,
                    hint,
                    fs(8.0) as f32,
                    crate::ui::fg3(pal),
                    false,
                );
            }
        }

        // record / stop button
        let bh = s(26.0);
        let by = gy + gh - bh - s(rec_pad_bottom as f64);
        let rec_hov = hov(rec_key);
        let rec_bg = if recording {
            rec_active
        } else if rec_hov {
            rec_bg_hov
        } else {
            rec_bg_idle
        };
        v.push(Cmd::Rect {
            x: (gx + pad - s(4.0)) as f32,
            y: by as f32,
            w: (gw - pad * 2.0 + s(8.0)) as f32,
            h: bh as f32,
            r: s(7.0) as f32,
            color: rec_bg,
        });
        if recording {
            // stop glyph: white filled square inside the red button
            let sw = s(8.0);
            v.push(Cmd::Rect {
                x: (gx + pad + s(2.0)) as f32,
                y: (by + (bh - sw) / 2.0) as f32,
                w: sw as f32,
                h: sw as f32,
                r: s(1.5) as f32,
                color: pal.fg,
            });
        } else {
            // record glyph: red filled circle
            let d = s(8.0);
            v.push(Cmd::Rect {
                x: (gx + pad + s(2.0)) as f32,
                y: (by + (bh - d) / 2.0) as f32,
                w: d as f32,
                h: d as f32,
                r: (d / 2.0) as f32,
                color: rec_dot,
            });
        }
        let (rlbl, rcol) = if recording {
            (
                format!("Stop  {:02}:{:02}", rec_secs / 60, rec_secs % 60),
                crate::ui::mix(pal.fg, rec_active, 0.25),
            )
        } else {
            ("Record".to_string(), pal.fg)
        };
        crate::ui::text(
            v,
            (gx + pad + s(16.0)) as f32,
            (by + s(7.0)) as f32,
            &rlbl,
            fs(9.5) as f32,
            rcol,
            false,
        );
        hits.push((
            (gx + pad - s(4.0) - s(3.0)) as f32,
            (by - s(3.0)) as f32,
            (gw - pad * 2.0 + s(8.0) + s(6.0)) as f32,
            (bh + s(6.0)) as f32,
            rec_key,
        ));
    }

    // pagination dots (one per pane, shown ONLY while the cursor rests on the
    // card) — the `dot = step * 0.66` product runs in f64 like the module
    if dots_show {
        let step = s(7.0);
        let dot = step * 0.66;
        let dx = gx + (gw - 2.0 * step) / 2.0;
        let dy = (gy + gh - dot - step * 0.5) as f32;
        for i in 0..2 {
            let c = if i == pane { pal.acc } else { crate::ui::hover(pal) };
            v.push(Cmd::Rect {
                x: (dx + i as f64 * step) as f32,
                y: dy,
                w: dot as f32,
                h: dot as f32,
                r: (dot / 2.0) as f32,
                color: c,
            });
        }
    }
    hits
}

/// The World Map card — header + style badge, the map body (ONE `Cmd::Map` +
/// city/marker/crosshair overlays, the same catcher as the Lua binding), zoom
/// buttons, download pills, and either the style menu or the bottom time band.
#[allow(clippy::too_many_arguments)]
pub(crate) fn worldmap(
    v: &mut Vec<Cmd>,
    show_glyph: bool,
    show_title: bool,
    scene_owns_header: bool,
    glyph: &str,
    style_hint: &str,
    menu: bool,
    save: bool,
    ready: bool,
    fallback: bool,
    body: (f32, f32, f32, f32),
    win: (f32, f32, f32, f32),
    zoom: f32,
    style_idx: usize,
    colors: &[u32; 6],
    region_on: bool,
    rev: u64,
    marker: Option<(f32, f32)>,
    cursor: Option<(f32, f32)>,
    hover_flag: bool,
    dl_pending_base: bool,
    dl_err: Option<&str>,
    region_pill: Option<(&str, bool, bool)>,
    city: &str,
    abbrev: &str,
    hhmm: &str,
    dayline: &str,
    hover: u32,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    grid: f32,
    pal: &Pal,
) -> Vec<CardHit> {
    let gx = x as f64;
    let gy = y as f64;
    let gw = w as f64;
    let gh = h as f64;
    let g = grid as f64;
    let s = |n: f64| n * g;
    let fs = |n: f64| (n * g * 10.0).round() / 10.0;
    let pad = s(12.0);
    let panes = crate::shell::WORLD_PANES;
    let _map_key = crate::shell::WORLD_MAP_KEY;
    let zoom_in_key = crate::shell::WORLD_ZOOM_IN_KEY;
    let zoom_out_key = crate::shell::WORLD_ZOOM_OUT_KEY;
    let dl_key = crate::shell::WORLD_DL_KEY;
    let region_dl_key = crate::shell::WORLD_REGION_DL_KEY;
    let menu_key = crate::shell::WORLD_MENU_KEY;
    let menu_toggle_key = crate::shell::WORLD_MENU_TOGGLE_KEY;
    let style_key_base = crate::shell::WORLD_STYLE_KEY_BASE;
    let mut hits = Vec::new();
    let hl = |key: u32, a: u32, b: u32| if hover == key { a } else { b };

    // header: globe glyph + title + current style badge (+ zoom hint)
    if show_glyph {
        crate::ui::text(
            v,
            (gx + pad) as f32,
            (gy + s(8.0)) as f32,
            glyph,
            fs(12.0) as f32,
            pal.acc,
            true,
        );
    }
    if !scene_owns_header && show_title {
        let tx = if show_glyph {
            gx + pad + s(18.0)
        } else {
            gx + pad
        };
        crate::ui::title(v, tx as f32, (gy + s(9.0)) as f32, "World", fs(12.0) as f32, pal.fg);
    }
    if !menu {
        crate::ui::text_r(
            v,
            (gx + gw - pad) as f32,
            (gy + s(9.0)) as f32,
            style_hint,
            fs(9.5) as f32,
            crate::ui::fg3(pal),
            false,
        );
    }

    let (mx, my, mw, mh) = (body.0 as f64, body.1 as f64, body.2 as f64, body.3 as f64);

    if ready {
        // the map body (raster + live vector overlays)
        map_body(
            v,
            body,
            win,
            zoom,
            style_idx as u8,
            *colors,
            region_on,
            rev,
            marker,
            cursor,
            hover_flag,
            menu,
            grid,
            pal,
        );

        // zoom buttons: [–] then [+] (rightmost = in)
        let bsz = s(17.0);
        let gap = s(5.0);
        let zx = mx + mw - bsz * 1.55;
        let zy = my + mh - bsz - s(5.0);
        for (i, (glyph2, key)) in [("\u{2013}", zoom_out_key), ("+", zoom_in_key)]
            .into_iter()
            .enumerate()
        {
            let bx = zx - i as f64 * (bsz + gap);
            let c = hl(key, crate::ui::raised_hl(pal), crate::ui::hover_hl(pal));
            v.push(Cmd::Rect {
                x: bx as f32,
                y: zy as f32,
                w: bsz as f32,
                h: bsz as f32,
                r: (bsz / 2.0) as f32,
                color: c,
            });
            crate::ui::text_c(
                v,
                (bx + bsz / 2.0) as f32,
                (zy + bsz / 2.0 - fs(11.0) / 2.0 + s(1.0)) as f32,
                glyph2,
                fs(11.0) as f32,
                pal.fg,
                false,
            );
            hits.push((bx as f32, zy as f32, bsz as f32, bsz as f32, key));
        }

        // on-demand region download pill (high zoom, missing country chunk)
        if let Some((rname, rpending, rcached)) = region_pill {
            let pw = s(172.0);
            let ph = s(18.0);
            let px = mx + mw / 2.0 - pw / 2.0;
            let py = my + s(6.0);
            if rpending {
                v.push(Cmd::Rect {
                    x: px as f32,
                    y: py as f32,
                    w: pw as f32,
                    h: ph as f32,
                    r: (ph / 2.0) as f32,
                    color: crate::ui::raised(pal),
                });
                crate::ui::text_c(
                    v,
                    (px + pw / 2.0) as f32,
                    (py + ph - s(4.5) - s(1.0)) as f32,
                    &format!("Downloading {rname}"),
                    fs(9.5) as f32,
                    crate::ui::fg3(pal),
                    false,
                );
            } else if rcached {
                // the services timer will pick it up; a small "loading" hint
                v.push(Cmd::Rect {
                    x: px as f32,
                    y: py as f32,
                    w: pw as f32,
                    h: ph as f32,
                    r: (ph / 2.0) as f32,
                    color: crate::ui::raised(pal),
                });
                crate::ui::text_c(
                    v,
                    (px + pw / 2.0) as f32,
                    (py + ph - s(5.0) - s(1.0)) as f32,
                    &format!("Loading {rname}"),
                    fs(9.5) as f32,
                    crate::ui::fg3(pal),
                    false,
                );
            } else {
                let c = hl(region_dl_key, crate::ui::sel_bg(pal), crate::ui::raised_hl(pal));
                v.push(Cmd::Rect {
                    x: px as f32,
                    y: py as f32,
                    w: pw as f32,
                    h: ph as f32,
                    r: (ph / 2.0) as f32,
                    color: c,
                });
                crate::ui::text_c(
                    v,
                    (px + pw / 2.0) as f32,
                    (py + ph - s(5.0) - s(1.0)) as f32,
                    &format!("Download {rname} map"),
                    fs(9.5) as f32,
                    pal.fg,
                    false,
                );
                hits.push((px as f32, py as f32, pw as f32, ph as f32, region_dl_key));
            }
        }

        // compact overlay pill over the embedded fallback: fetch the full dataset
        if fallback {
            let cx = mx + mw / 2.0;
            let pw = s(118.0);
            let ph = s(16.0);
            let px = cx - pw / 2.0;
            let py = my + s(6.0);
            let (bg, label) = if dl_pending_base {
                (crate::ui::raised(pal), "Downloading world\u{2026}".to_string())
            } else {
                (
                    hl(dl_key, crate::ui::sel_bg(pal), crate::ui::raised_hl(pal)),
                    "Download full map".to_string(),
                )
            };
            v.push(Cmd::Rect {
                x: px as f32,
                y: py as f32,
                w: pw as f32,
                h: ph as f32,
                r: (ph / 2.0) as f32,
                color: bg,
            });
            crate::ui::text_c(
                v,
                cx as f32,
                (py + ph - s(4.5) - s(1.0)) as f32,
                &label,
                fs(8.5) as f32,
                if dl_pending_base {
                    crate::ui::fg3(pal)
                } else {
                    pal.fg
                },
                false,
            );
            if !dl_pending_base {
                hits.push((px as f32, py as f32, pw as f32, ph as f32, dl_key));
            }
            if let Some(err) = dl_err {
                crate::ui::text_c(
                    v,
                    cx as f32,
                    (py + ph + s(3.0) + s(11.0)) as f32,
                    err,
                    fs(8.0) as f32,
                    crate::ui::mix(pal.bg, pal.acc, 0.55),
                    false,
                );
            }
        }
    } else {
        // placeholder screen: no map data at all (normally unreachable — the
        // embedded fallback always installs a world)
        let cx = mx + mw / 2.0;
        let cy = my + mh / 2.0;
        if mw >= 40.0 && mh >= 20.0 {
            let pw = s(168.0);
            let ph = s(19.0);
            let px = cx - pw / 2.0;
            let py = cy - s(22.0);
            let (bg, label) = if dl_pending_base {
                (crate::ui::raised(pal), "Downloading\u{2026}".to_string())
            } else {
                (
                    hl(dl_key, crate::ui::sel_bg(pal), crate::ui::raised_hl(pal)),
                    "Download world map".to_string(),
                )
            };
            v.push(Cmd::Rect {
                x: px as f32,
                y: py as f32,
                w: pw as f32,
                h: ph as f32,
                r: (ph / 2.0) as f32,
                color: bg,
            });
            crate::ui::text_c(
                v,
                cx as f32,
                (py + ph - s(5.0) - s(1.0)) as f32,
                &label,
                fs(9.5) as f32,
                if dl_pending_base {
                    crate::ui::fg3(pal)
                } else {
                    pal.fg
                },
                false,
            );
            if !dl_pending_base {
                hits.push((px as f32, py as f32, pw as f32, ph as f32, dl_key));
            }
            if let Some(err) = dl_err {
                crate::ui::text_c(
                    v,
                    cx as f32,
                    (cy + s(2.0) + s(14.0)) as f32,
                    err,
                    fs(8.5) as f32,
                    crate::ui::mix(pal.bg, pal.acc, 0.55),
                    false,
                );
            }
        }
    }

    if menu {
        // swipe-open menu: dim backdrop, style picker, save-downloads toggle
        v.push(Cmd::Rect {
            x: x,
            y: y,
            w: w,
            h: h,
            r: 6.0,
            color: 0x8a000000,
        });
        hits.push((x, y, w, h, menu_key));
        let pw = s(172.0);
        let row_h = s(29.0);
        let row_gap = s(4.0);
        let title_h = s(22.0);
        let pad_y = s(10.0);
        let rows = panes + 1;
        let ph = pad_y * 2.0 + title_h + rows as f64 * (row_h + row_gap);
        let px = gx + (gw - pw) / 2.0;
        let py = gy + (gh - ph) / 2.0;
        v.push(Cmd::Rect {
            x: px as f32,
            y: py as f32,
            w: pw as f32,
            h: ph as f32,
            r: 8.0,
            color: crate::ui::raised(pal),
        });
        v.push(Cmd::Outline {
            x: px as f32,
            y: py as f32,
            w: pw as f32,
            h: ph as f32,
            r: 8.0,
            width: 1.0,
            color: crate::ui::hairline(pal),
        });
        crate::ui::text(
            v,
            (px + s(12.0)) as f32,
            (py + s(14.0)) as f32,
            "Map style",
            fs(10.5) as f32,
            crate::ui::fg3(pal),
            false,
        );
        let mut ry = py + pad_y + title_h;
        for i in 0..panes {
            let key = style_key_base + i as u32;
            let active = i == style_idx;
            let bg = if active {
                crate::ui::sel_bg(pal)
            } else if hover == key {
                0
            } else {
                crate::ui::hover(pal)
            };
            if bg != 0 {
                v.push(Cmd::Rect {
                    x: (px + s(8.0)) as f32,
                    y: ry as f32,
                    w: (pw - s(16.0)) as f32,
                    h: row_h as f32,
                    r: 5.0,
                    color: bg,
                });
            }
            if active {
                v.push(Cmd::Rect {
                    x: (px + s(8.0)) as f32,
                    y: (ry + s(4.0)) as f32,
                    w: s(3.0) as f32,
                    h: (row_h - s(8.0)) as f32,
                    r: 1.5,
                    color: pal.acc,
                });
            }
            crate::ui::text(
                v,
                (px + s(22.0)) as f32,
                (ry + s(19.0)) as f32,
                crate::shell::WORLD_STYLE_NAMES[i],
                fs(11.0) as f32,
                if active { pal.fg } else { crate::ui::fg2(pal) },
                false,
            );
            hits.push((
                (px + s(8.0)) as f32,
                ry as f32,
                (pw - s(16.0)) as f32,
                row_h as f32,
                key,
            ));
            ry += row_h + row_gap;
        }
        // toggle row: "Save downloads"
        if hover != menu_toggle_key {
            v.push(Cmd::Rect {
                x: (px + s(8.0)) as f32,
                y: ry as f32,
                w: (pw - s(16.0)) as f32,
                h: row_h as f32,
                r: 5.0,
                color: crate::ui::hover(pal),
            });
        }
        crate::ui::text(
            v,
            (px + s(22.0)) as f32,
            (ry + s(19.0)) as f32,
            "Save downloads",
            fs(11.0) as f32,
            pal.fg,
            false,
        );
        let sw_w = s(26.0);
        let sw_h = s(13.0);
        let swx = px + pw - s(8.0) - sw_w;
        let swy = ry + (row_h - sw_h) / 2.0;
        let track = crate::ui::mix(pal.bg, pal.fg, if save { 0.45 } else { 0.28 });
        v.push(Cmd::Rect {
            x: swx as f32,
            y: swy as f32,
            w: sw_w as f32,
            h: sw_h as f32,
            r: (sw_h / 2.0) as f32,
            color: track,
        });
        let kn = sw_h - s(4.0);
        let kx = if save {
            swx + sw_w - kn - s(2.0)
        } else {
            swx + s(2.0)
        };
        v.push(Cmd::Rect {
            x: kx as f32,
            y: (swy + s(2.0)) as f32,
            w: kn as f32,
            h: kn as f32,
            r: (kn / 2.0) as f32,
            color: pal.fg,
        });
        hits.push((
            (px + s(8.0)) as f32,
            ry as f32,
            (pw - s(16.0)) as f32,
            row_h as f32,
            menu_toggle_key,
        ));
    } else {
        // bottom band: local time for the pinned marker
        let ccx = gx + gw / 2.0;
        if marker.is_some() {
            let mut line = String::new();
            if !city.is_empty() {
                line = format!("{city}  \u{b7}  ");
            }
            line.push_str(hhmm);
            if !abbrev.is_empty() {
                line.push(' ');
                line.push_str(abbrev);
            }
            crate::ui::text_c(
                v,
                ccx as f32,
                (gy + gh - s(30.0)) as f32,
                &line,
                fs(10.5) as f32,
                pal.fg,
                false,
            );
            crate::ui::text_c(
                v,
                ccx as f32,
                (gy + gh - s(17.0)) as f32,
                dayline,
                fs(9.5) as f32,
                crate::ui::fg3(pal),
                false,
            );
        } else {
            crate::ui::text_c(
                v,
                ccx as f32,
                (gy + gh - s(24.0)) as f32,
                "click a spot for its local time",
                fs(9.5) as f32,
                crate::ui::fg3(pal),
                false,
            );
        }
    }
    hits
}

/// The map body primitive — ONE `Cmd::Map` raster + the live vector overlays:
/// city labels (zoom≥2, clamped in-body), the pinned marker (two dots) and
/// the hover crosshair outline. Exactly the Lua host's `ctx.ui.map` binding.
#[allow(clippy::too_many_arguments)]
pub(crate) fn map_body(
    v: &mut Vec<Cmd>,
    (mx, my, mw, mh): (f32, f32, f32, f32),
    (lon0, lat0, lon1, lat1): (f32, f32, f32, f32),
    zoom: f32,
    style: u8,
    colors: [u32; 6],
    region: bool,
    version: u64,
    marker: Option<(f32, f32)>,
    cursor: Option<(f32, f32)>,
    hover: bool,
    menu: bool,
    grid: f32,
    pal: &Pal,
) {
    v.push(Cmd::Map {
        x: mx,
        y: my,
        w: mw,
        h: mh,
        lon0,
        lat0,
        lon1,
        lat1,
        zoom,
        style,
        colors,
        region,
        version,
    });
    let win = crate::shell::worldmap::Win {
        lon0,
        lat0,
        lon1,
        lat1,
    };
    // city labels stay live vectors, clamped inside the body
    if zoom >= 2.0 {
        if let Some(cities) = crate::shell::worldmap::cities() {
            for c in cities.iter() {
                if !(win.lon0 - 1.0..=win.lon1 + 1.0).contains(&c.lon)
                    || !(win.lat0 - 1.0..=win.lat1 + 1.0).contains(&c.lat)
                {
                    continue;
                }
                let (cx, cy) = crate::shell::worldmap::proj(
                    mx,
                    my,
                    mw,
                    mh,
                    &win,
                    c.lon.min(win.lon1).max(win.lon0),
                    c.lat,
                );
                let fs8 = (grid * 8.0 * 10.0).round() / 10.0;
                let tx = (cx - grid * 3.0).clamp(mx, mx + mw - grid * 6.0);
                let ty = (cy + grid * 2.0).clamp(my, my + mh - fs8);
                crate::ui::text(v, tx, ty, &c.name, fs8, crate::ui::fg2(pal), false);
            }
        }
    }
    if let Some((lat, lon)) = marker {
        let (px, py) = crate::shell::worldmap::proj(mx, my, mw, mh, &win, lon, lat);
        if win.contains(lon, lat)
            && crate::shell::Shell::in_rect(px, py, (mx - 2.0, my - 2.0, mw + 4.0, mh + 4.0))
        {
            v.push(Cmd::Rect {
                x: px - grid * 4.0,
                y: py - grid * 4.0,
                w: grid * 8.0,
                h: grid * 8.0,
                r: grid * 4.0,
                color: crate::ui::mix(pal.fg, pal.acc, 0.40),
            });
            v.push(Cmd::Rect {
                x: px - grid * 2.0,
                y: py - grid * 2.0,
                w: grid * 4.0,
                h: grid * 4.0,
                r: grid * 2.0,
                color: pal.acc,
            });
        }
    }
    if hover && !menu {
        if let Some((px, py)) = cursor {
            if crate::shell::Shell::in_rect(px, py, (mx, my, mw, mh)) {
                v.push(Cmd::Outline {
                    x: mx,
                    y: my,
                    w: mw,
                    h: mh,
                    r: 4.0,
                    width: 1.0,
                    color: crate::ui::hover_hl(pal),
                });
            }
        }
    }
}

#[cfg(test)]
mod tests;
