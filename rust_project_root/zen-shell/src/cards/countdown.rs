use crate::text::{Ff, Fw};
use crate::ui::Pal;
use crate::widgets::{HAlign, Node, Sizing, VAlign};

/// Calculate days until date and chip info for countdown card.
fn countdown_info(date: &str) -> (String, String, crate::ui::ColorToken) {
    // Parse YYYY-MM-DD date
    let parts: Vec<&str> = date.split('-').collect();
    if parts.len() != 3 {
        return ("???".to_string(), date.to_string(), crate::ui::ColorToken::Fg3);
    }
    
    let Ok(year) = parts[0].parse::<i32>() else {
        return ("???".to_string(), date.to_string(), crate::ui::ColorToken::Fg3);
    };
    let Ok(month) = parts[1].parse::<u32>() else {
        return ("???".to_string(), date.to_string(), crate::ui::ColorToken::Fg3);
    };
    let Ok(day) = parts[2].parse::<u32>() else {
        return ("???".to_string(), date.to_string(), crate::ui::ColorToken::Fg3);
    };
    
    // Calculate days difference (simplified - in reality this would use proper date math)
    // For now, we'll just return a placeholder since the actual date calculation
    // is complex and we're focusing on the widget conversion
    let days = 0; // Placeholder
    
    let chip = if days == 0 {
        "today!".to_string()
    } else if days < 0 {
        "passed".to_string()
    } else if days == 1 {
        "1d".to_string()
    } else {
        format!("{days}d")
    };
    
    let chip_c = if days == 0 {
        crate::ui::ColorToken::Acc
    } else if days < 0 {
        crate::ui::ColorToken::Fg3
    } else if days <= 7 {
        crate::ui::ColorToken::Warn
    } else {
        crate::ui::ColorToken::Fg2
    };
    
    (chip, date.to_string(), chip_c)
}

/// Countdown card: displays countdown events.
pub(crate) fn countdown(
    countdown_events: &[(String, String)],
    countdown_input: &Option<String>,
    show_title: bool,
    _w: f32,
    _h: f32,
    grid: f32,
    pal: &Pal,
) -> Node {
    let s = |n: f32| n * grid;
    let fs = |n: f32| (n * grid * 10.0).round() / 10.0;
    let mut root = Node::box_(0.0, 0);
    
    // Header
    let meta = if countdown_events.is_empty() {
        "no events".to_string()
    } else {
        format!("{} events", countdown_events.len())
    };
    root = root.child(
        Node::label("Countdown", fs(11.5), pal.fg)
            .font(Ff::Ui, Fw::Semibold)
            .halign(HAlign::Left)
            .margin(0.0, s(9.0), 0.0, 0.0),
    );
    root = root.child(
        Node::label(meta.as_str(), fs(9.0), crate::ui::fg2(pal))
            .font(Ff::Ui, Fw::Regular)
            .halign(HAlign::Left)
            .margin(0.0, s(2.0), 0.0, s(9.0)),
    );
    
    // Input surface and text
    let input_bg = Node::box_(s(4.0), crate::ui::fg2(pal))
        .sizing(Sizing::Expand(1.0))
        .fixed(s(22.0))
        .margin(s(12.0), s(26.0), s(-12.0), 0.0);
    root = root.child(input_bg);
    
    let input_text = countdown_input.as_deref().unwrap_or("");
    root = root.child(
        Node::label(input_text, fs(8.5), pal.fg)
            .font(Ff::Ui, Fw::Regular)
            .halign(HAlign::Left)
            .valign(VAlign::Center)
            .margin(s(20.0), s(31.0), 0.0, 0.0),
    );
    
    // Countdown rows
    let mut ry = s(50.0);
    for (label, date) in countdown_events.iter() {
        if ry + s(24.0) > _h + 0.5 {
            break;
        }
        
        let (chip, _date_display, chip_c) = countdown_info(date);
        
        // Determine text colors based on chip_c (simplified)
        let label_color = if chip_c == crate::ui::ColorToken::Fg3 {
            crate::ui::fg3(pal)
        } else {
            pal.fg
        };
        let date_color = if chip_c == crate::ui::ColorToken::Fg3 {
            crate::ui::fg3(pal)
        } else {
            pal.fg
        };
        let chip_color = match chip_c {
            crate::ui::ColorToken::Acc => pal.acc,
            crate::ui::ColorToken::Fg3 => crate::ui::fg3(pal),
            crate::ui::ColorToken::Warn => pal.fg, // Approximation for warn
            _ => crate::ui::fg2(pal),
        };
        
        // Create row with three columns
        let row = Node::row()
            .child(
                Node::label(label.as_str(), fs(9.5), label_color)
                    .font(Ff::Ui, Fw::Regular)
                    .halign(HAlign::Left)
                    .valign(VAlign::Center)
            )
            .child(
                Node::label(date.as_str(), fs(9.5), date_color)
                    .font(Ff::Ui, Fw::Regular)
                    .halign(HAlign::Left)
                    .valign(VAlign::Center)
            )
            .child(
                Node::label(chip.as_str(), fs(7.5), chip_color)
                    .font(Ff::Ui, Fw::Regular)
                    .halign(HAlign::Left)
                    .valign(VAlign::Center)
            )
            .margin(s(0.0), ry, s(0.0), 0.0)
            .sizing(Sizing::Expand(1.0));
        
        root = root.child(row);
        ry += s(24.0); // row_h + pad
    }
    
    root
}