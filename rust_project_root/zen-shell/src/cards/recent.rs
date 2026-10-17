use crate::text::{Ff, Fw};
use crate::ui::Pal;
use crate::widgets::{HAlign, Node, Sizing, VAlign};

/// Recent files card: displays recently opened files.
pub(crate) fn recent(
    recent_files: &[(String, String)],
    recent_scroll: usize,
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
            Node::label("Recent files", fs(11.5), pal.fg)
                .font(Ff::Ui, Fw::Semibold)
                .halign(HAlign::Left)
                .margin(0.0, s(9.0), 0.0, 0.0),
        );
        let meta = format!("{} files", recent_files.len());
        root = root.child(
            Node::label(meta.as_str(), fs(9.0), crate::ui::fg2(pal))
                .font(Ff::Ui, Fw::Regular)
                .halign(HAlign::Left)
                .margin(s(12.0), s(9.0), 0.0, s(12.0)),
        );
    }
    
    // Recent files list
    let mut ry = s(26.0);
    for (i, (name, _path)) in recent_files.iter().enumerate() {
        if ry + s(19.0) > h + 0.5 {
            break;
        }
        let j = i.saturating_sub(recent_scroll) as u32;
        // In the widget version we don't have hover state, so we render default colors.
        // The scene would highlight on hover, but we omit that for simplicity.
        let truncated = name.chars().take(24).collect::<String>();
        
        // Create row with two columns: filename and open icon
        let row = Node::row()
            .child(
                Node::label(truncated.as_str(), fs(9.5), pal.fg)
                    .font(Ff::Ui, Fw::Regular)
                    .halign(HAlign::Left)
                    .valign(VAlign::Top)
            )
            .child(
                Node::label("", fs(8.0), crate::ui::fg3(pal))
                    .font(Ff::Ui, Fw::Regular)
                    .halign(HAlign::Right)
                    .valign(VAlign::Top)
            )
            .margin(s(6.0), ry, s(0.0), 0.0)
            .sizing(Sizing::Expand(1.0));
        
        root = root.child(row);
        ry += s(19.0); // row_h + pad (from scene: row_h=19, pad=12? Actually row_h=19, pad=12? Let's check scene: row_h=19.0, pad=12.0? The scene has pad:12.0, row_h:19.0. So total per row = 19 + 12 = 31? But they also have row_dy:0.0. Actually the scene rows: pad=12.0, row_h=19.0. So each row occupies 31 vertical? However they also have y start at 26.0 and they insert rows with that pitch. We'll approximate with row_h + pad = 19+12=31. But we used 19 earlier; let's compute: In the scene they have y:26.0, row_h:19.0, pad:12.0. So the distance between row starts is row_h + pad = 31.0. We'll use 31.
        ry += s(31.0); // row_h + pad
    }
    
    // Show status message if empty
    if recent_files.is_empty() {
        root = root.child(
            Node::label("no recent files tracked", fs(9.0), crate::ui::fg3(pal))
                .font(Ff::Ui, Fw::Regular)
                .halign(HAlign::Center)
                .valign(VAlign::Center)
        );
    }
    
    root
}