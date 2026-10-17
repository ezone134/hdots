use crate::text::{Ff, Fw};
use crate::ui::Pal;
use crate::widgets::{HAlign, Node, Sizing, VAlign};

/// Alarms card: displays alarm list and input for new alarms.
pub(crate) fn alarms(
    alarm_list: &[(String, String)],
    alarm_input: Option<String>,
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
            Node::label("Alarms", fs(11.5), pal.fg)
                .font(Ff::Ui, Fw::Semibold)
                .halign(HAlign::Left)
                .valign(VAlign::Top)
                .margin(s(14.0), s(9.0), 0.0, 0.0),
        );
    }
    // Alarm meta text (e.g., "3 set")
    let alarm_meta = if alarm_list.is_empty() {
        String::new()
    } else {
        format!("{} set", alarm_list.len())
    };
    if !alarm_meta.is_empty() {
        root = root.child(
            Node::label(alarm_meta.as_str(), fs(9.0), pal.fg)
                .font(Ff::Ui, Fw::Regular)
                .halign(HAlign::Right)
                .valign(VAlign::Top)
                .margin(0.0, s(12.0), s(14.0), 0.0),
        );
    }
    // Alarm input field background (hit region for input)
    let input_bg = Node::box_(s(4.0), crate::ui::hover(pal))
        .sizing(Sizing::Expand(1.0))
        .margin(s(12.0), s(26.0), s(12.0), 0.0);
    root = root.child(input_bg);
    // Alarm input text
    if let Some(input) = alarm_input.as_ref() {
        root = root.child(
            Node::label(input.as_str(), fs(8.5), pal.fg)
                .font(Ff::Ui, Fw::Regular)
                .halign(HAlign::Left)
                .valign(VAlign::Top)
                .margin(s(20.0), s(31.0), 0.0, 0.0),
        );
    }
    // Alarm list rows
    let mut ry = s(50.0);
    for (time, label) in alarm_list {
        if ry + s(22.0) > h + 0.5 {
            break;
        }
        // Time column (mono, acc color)
        root = root.child(
            Node::label(time.as_str(), fs(10.0), pal.acc)
                .font(Ff::Mono, Fw::Regular)
                .halign(HAlign::Left)
                .valign(VAlign::Top)
                .margin(s(12.0 + s(0.0)), ry, 0.0, 0.0),
        );
        // Label column (regular, fg color, truncated)
        root = root.child(
            Node::label(label.as_str(), fs(9.5), pal.fg)
                .font(Ff::Ui, Fw::Regular)
                .halign(HAlign::Left)
                .valign(VAlign::Top)
                .margin(s(12.0 + s(50.0)), ry, s(12.0), 0.0),
        );
        ry += s(24.0); // row_h + pad
    }
    root
}