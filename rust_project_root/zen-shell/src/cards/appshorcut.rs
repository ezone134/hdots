use crate::text::{Ff, Fw};
use crate::ui::Pal;
use crate::widgets::{HAlign, Node, Sizing, VAlign};

/// App shortcuts card: displays a grid of application shortcuts.
pub(crate) fn appshorcut(
    app_shortcuts: &[String],
    app_shortcut_pick: Option<usize>,
    show_title: bool,
    _w: f32,
    _h: f32,
    grid: f32,
    pal: &Pal,
) -> Node {
    let s = |n: f32| n * grid;
    let fs = |n: f32| (n * grid * 10.0).round() / 10.0;
    let mut root = Node::box_(0.0, 0);
    if show_title {
        // Calculate app count like in panels.rs: "{filled}/8"
        let app_filled = app_shortcuts.iter().filter(|s| !s.is_empty()).count();
        let app_count_text = format!("{app_filled}/8");
        root = root.child(
            Node::row()
                .child(
                    Node::label("Apps", fs(11.5), pal.fg)
                        .font(Ff::Ui, Fw::Semibold)
                )
                .child(
                    Node::label(app_count_text.as_str(), fs(11.5), pal.fg)
                        .font(Ff::Ui, Fw::Regular)
                        .halign(HAlign::Right)
                )
                .child(
                    Node::glyph("\u{f0b1}", fs(11.5), pal.fg) // 
                        .halign(HAlign::Right)
                )
                .margin(s(14.0), s(9.0), 0.0, 0.0)
                .gap(s(10.0))
                .sizing(Sizing::Expand(1.0)),
        );
    }
    // App shortcuts grid
    let rx = s(12.0);
    let ry = s(36.0);
    // Calculate grid layout based on available width
    let _cell_size = s(26.0) + s(8.0); // image_size + image_top + text space
    let cols = if _w >= s(330.0) {
        4
    } else if _w >= s(250.0) {
        3
    } else if _w >= s(165.0) {
        2
    } else {
        1
    };
    let cell_w = if cols > 0 {
        (_w - s(24.0) - s(8.0) * (cols - 1) as f32) / cols as f32
    } else {
        _w
    };
    let cell_h = s(26.0) + s(10.0); // image + text approx
    for i in 0..8 {
        let col = i % cols;
        let row = i / cols;
        let _cell_x = rx + col as f32 * (cell_w + s(8.0));
        let cell_y = ry + row as f32 * (cell_h + s(8.0));
        if cell_y + cell_h > _h + s(2.0) {
            break;
        }
        let filled = i < app_shortcuts.len() && !app_shortcuts[i].is_empty();
        let is_picking = Some(i) == app_shortcut_pick;
        let mut cell = Node::box_(s(10.0), if is_picking { crate::ui::hover_hl(pal) } else { pal.bg })
            .sizing(Sizing::Fixed(cell_w))
            .fixed(cell_h);
        if filled {
            // Show app icon and name
            let icon = if /* has_icon */ true { "\u{f0b1}" } else { "\u{f0b1}" }; // apps icon fallback
            cell = cell.child(
                Node::glyph(icon, fs(18.0), pal.fg)
                    .halign(HAlign::Center)
                    .valign(VAlign::Top)
                    .margin(0.0, s(2.0), 0.0, 0.0),
            );
            cell = cell.child(
                Node::label(app_shortcuts[i].as_str(), fs(8.0), pal.fg)
                    .font(Ff::Ui, Fw::Regular)
                    .halign(HAlign::Center)
                    .valign(VAlign::Bottom)
                    .margin(0.0, 0.0, 0.0, s(2.0)),
            );
        } else {
            // Show empty slot indicator
            cell = cell.child(
                Node::label("+", fs(20.0), if is_picking { pal.acc } else { crate::ui::fg3(pal) })
                    .halign(HAlign::Center)
                    .valign(VAlign::Center),
            );
        }
        root = root.child(cell);
    }
    root
}