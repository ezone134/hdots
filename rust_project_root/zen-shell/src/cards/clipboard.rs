use crate::text::{Ff, Fw};
use crate::ui::Pal;
use crate::widgets::{HAlign, Node, Sizing, VAlign};

/// Clipboard card: displays clipboard history.
pub(crate) fn clipboard(
    clip_text: &[String],
    clip_sel: usize,
    clip_scroll: usize,
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
            Node::label("Clipboard", fs(11.5), pal.fg)
                .font(Ff::Ui, Fw::Semibold)
                .halign(HAlign::Center)
                .margin(0.0, s(9.0), 0.0, 0.0),
        );
    }
    
    // Clipboard history list
    let mut ry = s(32.0);
    for (i, text) in clip_text.iter().enumerate() {
        if ry + s(22.0) > _h + 0.5 {
            break;
        }
        let j = i.saturating_sub(clip_scroll) as u32;
        let is_sel = clip_sel == i;
        let is_hov = false; // Hover is handled by the scene engine, not needed for widget version
        
        // Truncate text to 32 characters like in panels.rs
        let label = text.chars().take(32).collect::<String>();
        
        // Background color for selected/hovered rows
        let bg_color = if is_sel || is_hov {
            Some(crate::ui::acc_tint(pal))
        } else {
            None
        };
        
        // Text color for selected/hovered rows
        let text_color = if is_sel || is_hov {
            Some(pal.fg)
        } else {
            Some(pal.fg)
        };
        
        // Create the row
        let row = Node::row()
            .child(
                Node::label(label.as_str(), fs(8.5), 
                           text_color.unwrap_or(pal.fg))
                    .font(Ff::Ui, Fw::Regular)
                    .halign(HAlign::Left)
                    .valign(VAlign::Center)
            )
            .margin(s(4.0), ry, s(4.0), 0.0)
            .sizing(Sizing::Expand(1.0));
            
        // Apply background if needed
        if let Some(bg) = bg_color {
            root = root.child(
                Node::box_(s(4.0), bg)
                    .sizing(Sizing::Expand(1.0))
                    .fixed(s(22.0))
                    .margin(s(0.0), ry, s(0.0), 0.0)
                    .child(row),
            );
        } else {
            root = root.child(row);
        }
        
        ry += s(22.0); // row_h + pad
    }
    
    // Show "Empty" message if applicable
    if clip_text.is_empty() {
        root = root.child(
            Node::label("Empty", fs(9.5), pal.fg)
                .font(Ff::Ui, Fw::Regular)
                .halign(HAlign::Center)
                .valign(VAlign::Center)
        );
    }
    
    root
}