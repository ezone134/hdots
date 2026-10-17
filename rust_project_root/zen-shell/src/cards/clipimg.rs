use crate::text::{Ff, Fw};
use crate::ui::Pal;
use crate::widgets::{HAlign, Node, Sizing, VAlign};

/// Clipboard images card: displays clipboard image history.
pub(crate) fn clipimg(
    clip_images: &[String],
    clip_sel: usize,
    clipimg_scroll: usize,
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
            Node::label("Clipboard images", fs(11.5), pal.fg)
                .font(Ff::Ui, Fw::Semibold)
                .halign(HAlign::Center)
                .margin(0.0, s(9.0), 0.0, 0.0),
        );
    }
    
    // Clipboard images list
    let mut ry = s(32.0);
    for (i, image_key) in clip_images.iter().enumerate() {
        if ry + s(28.0) > _h + 0.5 {
            break;
        }
        
        // Truncate image key to 28 characters like in panels.rs
        let truncated = image_key.chars().take(28).collect::<String>();
        
        // Background color for selected rows (simplified - no hover in widget version)
        let bg_color = if clip_sel == i {
            Some(crate::ui::acc_tint(pal))
        } else {
            None
        };
        
        // Create the row with image placeholder and text
        let row = Node::row()
            .child(
                // Image placeholder (rounded square with hover color)
                Node::box_(s(4.0), crate::ui::hover(pal))
                    .sizing(Sizing::Fixed(s(18.0)))
                    .fixed(s(18.0))
            )
            .child(
                Node::label(truncated.as_str(), fs(8.5), 
                           if clip_sel == i { pal.fg } else { pal.fg })
                    .font(Ff::Ui, Fw::Regular)
                    .halign(HAlign::Left)
                    .valign(VAlign::Center)
            )
            .margin(s(4.0), ry, s(4.0), 0.0)
            .sizing(Sizing::Expand(1.0));
            
        // Apply background if selected
        if let Some(bg) = bg_color {
            root = root.child(
                Node::box_(s(4.0), bg)
                    .sizing(Sizing::Expand(1.0))
                    .fixed(s(28.0))
                    .margin(s(0.0), ry, s(0.0), 0.0)
                    .child(row),
            );
        } else {
            root = root.child(row);
        }
        
        ry += s(28.0); // row_h + pitch (from scene: row_h=28, pitch=28)
    }
    
    // Show "No images copied yet" message if applicable
    if clip_images.is_empty() {
        root = root.child(
            Node::label("No images copied yet", fs(9.5), pal.fg)
                .font(Ff::Ui, Fw::Regular)
                .halign(HAlign::Center)
                .valign(VAlign::Center)
        );
    }
    
    root
}