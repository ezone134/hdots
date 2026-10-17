use crate::text::{Ff, Fw};
use crate::ui::Pal;
use crate::widgets::{HAlign, Node, Sizing, VAlign};
use crate::news::NewsItem;

/// News card: displays news headlines.
pub(crate) fn news(
    news_items: &[NewsItem],
    news_cats: &[String],
    news_label: &String,
    news_chips: &[String],
    news_cat_sel: f32,
    news_cat_scroll: f32,
    news_msg: &String,
    news_empty: bool,
    news_scroll: usize,
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
            Node::label("News", fs(11.5), pal.fg)
                .font(Ff::Ui, Fw::Semibold)
                .halign(HAlign::Left)
                .margin(0.0, s(9.0), 0.0, 0.0),
        );
        root = root.child(
            Node::label(news_label.as_str(), fs(8.5), pal.fg)
                .font(Ff::Ui, Fw::Regular)
                .halign(HAlign::Left)
                .margin(s(12.0), s(9.0), 0.0, s(12.0)),
        );
    }
    
    // Message (empty state)
    if news_empty {
        root = root.child(
            Node::label(news_msg.as_str(), fs(9.0), crate::ui::fg3(pal))
                .font(Ff::Ui, Fw::Regular)
                .halign(HAlign::Center)
                .valign(VAlign::Center)
        );
        return root;
    }
    
    // Category chips (strip)
    let chip_y = s(30.0);
    let chip_height = s(26.0);
    let chip_spacing = s(6.0);
    let chip_pad = s(11.0);
    
    // In a real implementation, we'd render the actual chips with selection and scrolling
    // For now, just show a placeholder
    root = root.child(
        Node::label("[News Chips]", fs(8.5), pal.fg)
            .font(Ff::Ui, Fw::Regular)
            .halign(HAlign::Center)
            .valign(VAlign::Center)
    );
    
    // News rows
    let mut ry = s(59.0);
    let visible_rows = ((h - s(59.0)) / s(30.0)).ceil() as usize;
    let start = news_scroll.min(news_items.len().saturating_sub(visible_rows));
    let end = (start + visible_rows).min(news_items.len());
    
    for i in start..end {
        if ry + s(30.0) > h + 0.5 {
            break;
        }
        let item = &news_items[i];
        
        // Truncate title to 40 characters like in panels.rs
        let title = item.title.chars().take(40).collect::<String>();
        
        // Determine if URL exists (for open icon)
        let has_url = !item.url.trim().is_empty();
        
        // Source truncated to 16 characters like in panels.rs
        let source = item.source.chars().take(16).collect::<String>();
        
        // Create row with three columns
        let row = Node::row()
            .child(
                Node::label(title.as_str(), fs(9.5), pal.fg)
                    .font(Ff::Ui, Fw::Regular)
                    .halign(HAlign::Left)
                    .valign(VAlign::Top)
            )
            .child(
                Node::label(if has_url { "" } else { "" }, fs(8.0), 
                           if has_url { pal.acc } else { crate::ui::fg3(pal) })
                    .font(Ff::Ui, Fw::Regular)
                    .halign(HAlign::Right)
                    .valign(VAlign::Top)
            )
            .child(
                Node::label(source.as_str(), fs(7.5), crate::ui::fg3(pal))
                    .font(Ff::Ui, Fw::Regular)
                    .halign(HAlign::Right)
                    .valign(VAlign::Top)
            )
            .margin(s(5.0), ry, s(0.0), 0.0)
            .sizing(Sizing::Expand(1.0));
        
        root = root.child(row);
        ry += s(30.0); // row_h + pitch
    }
    
    root
}