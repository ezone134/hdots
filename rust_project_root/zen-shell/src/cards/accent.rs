use crate::text::{Ff, Fw};
use crate::ui::Pal;
use crate::widgets::{HAlign, Node, Sizing, VAlign};

/// Accent card: displays the current accent color and name.
pub(crate) fn accent(
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
        root = root.child(
            Node::label("Accent", fs(11.5), pal.fg)
                .font(Ff::Ui, Fw::Semibold)
                .halign(HAlign::Left)
                .valign(VAlign::Top)
                .margin(s(14.0), s(9.0), 0.0, 0.0),
        );
    }
    // A rectangle filled with accent color
    root = root.child(
        Node::box_(s(4.0), pal.acc)
            .sizing(Sizing::Expand(1.0))
            .margin(s(10.0), s(20.0), s(10.0), s(10.0)),
    );
    root
}