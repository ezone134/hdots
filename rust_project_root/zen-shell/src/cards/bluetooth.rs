use crate::text::{Ff, Fw};
use crate::ui::Pal;
use crate::widgets::{HAlign, Node, Sizing, VAlign};

/// Bluetooth card: displays bluetooth status and paired devices.
pub(crate) fn bluetooth(
    bt_devices: &[(String, bool, bool)],
    bt_on: bool,
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
        // Calculate bluetooth chip status like in panels.rs
        let bt_chip_on = if bt_on { "on" } else { "" };
        let bt_chip_off = if bt_on { "" } else { "off" };
        let bt_status = if bt_devices.is_empty() {
            if bt_on {
                "no paired devices"
            } else {
                "bluetooth off — enable in toggles"
            }
        } else {
            ""
        };
        
        root = root.child(
            Node::row()
                .child(
                    Node::label("Bluetooth", fs(11.5), pal.fg)
                        .font(Ff::Ui, Fw::Semibold)
                )
                .child(
                    Node::label(bt_chip_on, fs(8.5), pal.acc)
                        .font(Ff::Ui, Fw::Regular)
                        .halign(HAlign::Right)
                )
                .child(
                    Node::label(bt_chip_off, fs(8.5), crate::ui::fg3(pal))
                        .font(Ff::Ui, Fw::Regular)
                        .halign(HAlign::Right)
                )
                .child(
                    Node::label(bt_status, fs(9.0), crate::ui::fg3(pal))
                        .font(Ff::Ui, Fw::Regular)
                        .halign(HAlign::Center)
                )
                .margin(s(14.0), s(9.0), 0.0, 0.0)
                .gap(s(6.0))
                .sizing(Sizing::Expand(1.0)),
        );
    }
    
    // Bluetooth devices list
    let mut ry = s(26.0);
    for (d, (name, connected, _paired)) in bt_devices.iter().take(18).enumerate() {
        if ry + s(19.0) > h + 0.5 {
            break;
        }
        
        // Device name column
        root = root.child(
            Node::label(name.as_str(), fs(9.5), pal.fg)
                .font(Ff::Ui, Fw::Regular)
                .halign(HAlign::Left)
                .valign(VAlign::Top)
                .margin(s(6.0), ry, 0.0, 0.0),
        );
        
        // Connection status column
        let status_text = if *connected {
            "Connected"
        } else {
            "Tap to connect"
        };
        let status_color = if *connected { pal.acc } else { crate::ui::fg3(pal) };
        
        root = root.child(
            Node::label(status_text, fs(9.5), status_color)
                .font(Ff::Ui, Fw::Regular)
                .halign(HAlign::Right)
                .valign(VAlign::Top)
                .margin(0.0, ry, s(24.0), 0.0) // approx width for status
                .fixed(s(24.0)),
        );
        
        ry += s(19.0); // row_h
    }
    
    // Show "no devices" message if applicable
    if bt_devices.is_empty() {
        let empty_text = if bt_on {
            "No devices found"
        } else {
            "Bluetooth is off"
        };
        root = root.child(
            Node::label(empty_text, fs(9.0), crate::ui::fg3(pal))
                .font(Ff::Ui, Fw::Regular)
                .halign(HAlign::Center)
                .valign(VAlign::Center)
        );
    }
    
    root
}