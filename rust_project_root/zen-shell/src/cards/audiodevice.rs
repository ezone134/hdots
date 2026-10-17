use crate::text::{Ff, Fw};
use crate::ui::Pal;
use crate::widgets::{HAlign, Node, Sizing, VAlign};

/// Audio device card: displays audio input/output devices.
pub(crate) fn audiodevice(
    audio_sinks: &[(u32, String, f32, bool)],
    audio_sources: &[(u32, String, f32, bool)],
    audio_default: u32,
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
        // Calculate device count like in panels.rs
        let total_devices = audio_sinks.len() + audio_sources.len();
        let auddev_meta = if total_devices > 0 {
            format!("{} dev", total_devices)
        } else {
            String::new()
        };
        if !auddev_meta.is_empty() {
            root = root.child(
                Node::row()
                    .child(
                        Node::label("Audio", fs(11.5), pal.fg)
                            .font(Ff::Ui, Fw::Semibold)
                    )
                    .child(
                        Node::label(auddev_meta.as_str(), fs(11.5), pal.fg)
                            .font(Ff::Ui, Fw::Regular)
                            .halign(HAlign::Right)
                    )
                    .child(
                        Node::glyph("\u{e010}", fs(11.5), pal.fg) // 
                            .halign(HAlign::Right)
                    )
                    .margin(s(14.0), s(9.0), 0.0, 0.0)
                    .gap(s(10.0))
                    .sizing(Sizing::Expand(1.0)),
            );
        }
    }
    // Audio devices list (sinks first, then sources)
    let mut ry = s(26.0);
    let mut _row_index = 0usize;
    
    // Process audio sinks (output devices)
    for &(id, ref name, vol, muted) in audio_sinks {
        if ry + s(21.0) > _h + 0.5 {
            break;
        }
        let is_default = id == audio_default;
        let prefix = if is_default {
            format!("{} ", "\u{f00c}") // ICON_STAR
        } else {
            String::new()
        };
        let volume_pct = (vol * 100.0).round() as i32;
        let volume_text = format!("{:>3}%", volume_pct);
        let icon = if muted {
            "\u{f026}" // ICON_MUTE
        } else {
            "\u{f028}" // ICON_VOLUME_FILL
        };
        let icon_color = if muted { pal.fg } else { crate::ui::fg2(pal) };
        
        // Device name column
        root = root.child(
            Node::row()
                .child(
                    Node::glyph(icon, fs(8.5), icon_color)
                        .halign(HAlign::Left)
                        .valign(VAlign::Top)
                )
                .child(
                    Node::label(format!("{prefix}{name}").as_str(), fs(8.5), if is_default { pal.fg } else { crate::ui::fg2(pal) })
                        .font(Ff::Mono, Fw::Regular)
                        .halign(HAlign::Left)
                        .valign(VAlign::Top)
                )
                .margin(s(2.0), ry, 0.0, 0.0)
                .gap(s(6.0))
                .sizing(Sizing::Expand(1.0)),
        );
        
        // Volume percentage column
        root = root.child(
            Node::label(volume_text.as_str(), fs(8.5), crate::ui::fg3(pal))
                .font(Ff::Ui, Fw::Regular)
                .halign(HAlign::Right)
                .valign(VAlign::Top)
                .margin(0.0, ry, s(12.0), 0.0)
                .fixed(s(24.0)), // approx width for volume %
        );
        
        ry += s(21.0); // row_h
        _row_index += 1;
    }
    
    // Process audio sources (input devices)
    for &(id, ref name, vol, muted) in audio_sources {
        if ry + s(21.0) > _h + 0.5 {
            break;
        }
        let volume_pct = (vol * 100.0).round() as i32;
        let volume_text = format!("{:>3}%", volume_pct);
        let icon = if muted {
            "\u{f026}" // ICON_MUTE
        } else {
            "\u{f028}" // ICON_VOLUME_FILL
        };
        let icon_color = if muted { pal.fg } else { crate::ui::fg2(pal) };
        
        // Device name column with source indicator
        root = root.child(
            Node::row()
                .child(
                    Node::glyph(icon, fs(8.5), icon_color)
                        .halign(HAlign::Left)
                        .valign(VAlign::Top)
                )
                .child(
                    Node::label(format!("{} {}", "\u{f061}", name).as_str(), fs(8.5), crate::ui::fg2(pal))
                        .font(Ff::Mono, Fw::Regular)
                        .halign(HAlign::Left)
                        .valign(VAlign::Top)
                )
                .margin(s(2.0), ry, 0.0, 0.0)
                .gap(s(6.0))
                .sizing(Sizing::Expand(1.0)),
        );
        
        // Volume percentage column
        root = root.child(
            Node::label(volume_text.as_str(), fs(8.5), crate::ui::fg3(pal))
                .font(Ff::Ui, Fw::Regular)
                .halign(HAlign::Right)
                .valign(VAlign::Top)
                .margin(0.0, ry, s(12.0), 0.0)
                .fixed(s(24.0)), // approx width for volume %
        );
        
        ry += s(21.0); // row_h
        _row_index += 1;
    }
    
    // Show "no audio devices" message if applicable
    if audio_sinks.is_empty() && audio_sources.is_empty() {
        root = root.child(
            Node::label("no audio devices", fs(9.0), crate::ui::fg3(pal))
                .font(Ff::Ui, Fw::Regular)
                .halign(HAlign::Center)
                .valign(VAlign::Center)
        );
    }
    
    root
}