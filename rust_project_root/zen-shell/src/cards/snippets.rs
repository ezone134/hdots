use crate::text::{Ff, Fw};
use crate::ui::Pal;
use crate::widgets::{HAlign, Node, Sizing, VAlign};
use std::time::Instant;

/// Snippets card: displays saved snippets.
pub(crate) fn snippets(
    snippets: &[(String, String)],
    snippet_input: &Option<String>,
    snippet_focus: bool,
    snippet_scroll: usize,
    snippet_copied: Option<usize>,
    snippet_copied_at: Option<Instant>,
    show_title: bool,
    w: f32,
    h: f32,
    grid: f32,
    pal: &Pal,
) -> Node {
    let s = |n: f32| n * grid;
    let fs = |n: f32| (n * grid * 10.0).round() / 10.0;
    let mut root = Node::box_(0.0, 0);
    
    // Header
    if show_title {
        root = root.child(
            Node::label("Snippets", fs(11.5), pal.fg)
                .font(Ff::Ui, Fw::Semibold)
                .halign(HAlign::Left)
                .margin(0.0, s(9.0), 0.0, 0.0),
        );
        let meta = format!("{} snippets", snippets.len());
        root = root.child(
            Node::label(meta.as_str(), fs(9.0), crate::ui::fg2(pal))
                .font(Ff::Ui, Fw::Regular)
                .halign(HAlign::Left)
                .margin(s(12.0), s(9.0), 0.0, s(12.0)),
        );
    }
    
    // Composer
    let composer_y = s(26.0);
    let composer_h = s(22.0);
    let composer = Node::row()
        .child(
            Node::label(
                snippet_input.as_deref().unwrap_or(""),
                fs(9.5),
                if snippet_focus { pal.fg } else { crate::ui::fg2(pal) }
            )
            .font(Ff::Ui, Fw::Regular)
            .halign(HAlign::Left)
            .valign(VAlign::Top)
        )
        .child(
            Node::label("add snippet — name text to copy", fs(9.5), crate::ui::fg2(pal))
                .font(Ff::Ui, Fw::Regular)
                .halign(HAlign::Left)
                .valign(VAlign::Top)
        )
        .margin(s(12.0), composer_y, s(0.0), 0.0)
        .sizing(Sizing::Expand(1.0));
    
    root = root.child(composer);
    
    // Snippets list
    let mut ry = s(50.0);
    for (i, (name, _body)) in snippets.iter().enumerate() {
        if ry + s(24.0) > h + 0.5 {
            break;
        }
        let j = i.saturating_sub(snippet_scroll) as u32;
        // Determine if this row should be flashed (copied recently)
        let mut flash = false;
        if let Some(copied_idx) = snippet_copied {
            if copied_idx == i {
                if let Some(copied_at) = snippet_copied_at {
                    if copied_at.elapsed() < std::time::Duration::from_secs(2) {
                        flash = true;
                    }
                }
            }
        }
        
        let truncated = name.chars().take(24).collect::<String>();
        
        // Create row with two columns: filename and copy icon
        let row = Node::row()
            .child(
                Node::label(truncated.as_str(), fs(9.5), if flash { pal.acc } else { pal.fg })
                    .font(Ff::Ui, Fw::Regular)
                    .halign(HAlign::Left)
                    .valign(VAlign::Top)
            )
            .child(
                Node::label("", fs(8.0), if flash { pal.acc } else { crate::ui::fg3(pal) })
                    .font(Ff::Ui, Fw::Regular)
                    .halign(HAlign::Right)
                    .valign(VAlign::Top)
            )
            .margin(s(6.0), ry, s(0.0), 0.0)
            .sizing(Sizing::Expand(1.0));
        
        root = root.child(row);
        ry += s(24.0); // row_h + pad (from scene: row_h=24, pad=12? Actually row_h=24.0, pad=12.0? The scene has pad:12.0, row_h:24.0. So total per row = 24 + 12 = 36? But they also have row_dy:0.0. Actually the scene rows: pad=12.0, row_h:24.0. So each row occupies 36 vertical? However they also have y start at 50.0 and they insert rows with that pitch. We'll approximate with row_h + pad = 24+12=36. But we used 24 earlier; let's compute: In the scene they have y:50.0, row_h:24.0, pad:12.0. So the distance between row starts is row_h + pad = 36.0. We'll use 36.
        ry += s(36.0); // row_h + pad
    }
    
    // Show status message if empty
    if snippets.is_empty() {
        root = root.child(
            Node::label("no snippets saved", fs(9.0), crate::ui::fg3(pal))
                .font(Ff::Ui, Fw::Regular)
                .halign(HAlign::Center)
                .valign(VAlign::Center)
        );
    }
    
    root
}
