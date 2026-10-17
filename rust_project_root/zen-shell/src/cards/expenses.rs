use crate::text::{Ff, Fw};
use crate::ui::Pal;
use crate::widgets::{HAlign, Node, Sizing, VAlign};

/// Expenses card: displays expense tracking.
pub(crate) fn expenses(
    expenses: &[(u32, String, String)],
    expense_input: &Option<String>,
    show_title: bool,
    w: f32,
    h: f32,
    grid: f32,
    pal: &Pal,
) -> Node {
    let s = |n: f32| n * grid;
    let fs = |n: f32| (n * grid * 10.0).round() / 10.0;
    let mut root = Node::box_(0.0, 0);
    
    // Calculate monthly summary using same approach as water_today
    let mut total = 0.0f32;
    let mut per: Vec<(String, f32)> = Vec::new();
    {
        let epoch = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let mut buf = [0u8; 16];
        let month_str = unsafe {
            let t = epoch as libc::time_t;
            let mut tm: libc::tm = std::mem::zeroed();
            libc::localtime_r(&t, &mut tm);
            let n = libc::strftime(
                buf.as_mut_ptr() as *mut libc::c_char,
                buf.len(),
                c"%Y-%m".as_ptr(),
                &tm,
            );
            std::str::from_utf8_unchecked(&buf[..n as usize]).to_string()
        };
        for (cents, cat, date) in expenses {
            if !date.starts_with(&month_str) {
                continue;
            }
            let v = *cents as f32 / 100.0;
            total += v;
            match per.iter_mut().find(|(c, _)| c == cat) {
                Some((_, s)) => *s += v,
                None => per.push((cat.clone(), v)),
            }
        }
    }
    per.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    let exp_max = per.first().map(|(_, v)| *v).unwrap_or(0.0).max(1.0);
    
    // Header
    if show_title {
        root = root.child(
            Node::label("Expenses", fs(11.5), pal.fg)
                .font(Ff::Ui, Fw::Semibold)
                .halign(HAlign::Left)
                .margin(0.0, s(9.0), 0.0, 0.0),
        );
        let meta = format!("MTD {total:.0}");
        root = root.child(
            Node::label(meta.as_str(), fs(8.5), pal.acc)
                .font(Ff::Ui, Fw::Regular)
                .halign(HAlign::Left)
                .margin(s(12.0), s(9.0), 0.0, s(12.0)),
        );
    }
    
    // Quick amount chips
    let chip_y = s(28.0);
    let chip_height = s(26.0);
    let chip_spacing = s(6.0);
    let amounts = [1, 5, 10, 20, 50, 100];
    for (i, &amount) in amounts.iter().enumerate() {
        let chip_x = s(0.0) + (i as f32 * (s(12.0) + chip_spacing));
        root = root.child(
            Node::label(amount.to_string().as_str(), fs(10.0), pal.fg)
                .font(Ff::Ui, Fw::Regular)
                .halign(HAlign::Center)
                .valign(VAlign::Center)
                .margin(chip_x, chip_y, s(12.0), 0.0),
        );
    }
    
    // Composer row
    let composer_y = s(60.0);
    let composer_height = s(22.0);
    let input_width = w - s(140.0); // Approximate
    root = root.child(
        Node::label("", fs(8.5), pal.fg) // Placeholder for input
            .font(Ff::Ui, Fw::Regular)
            .halign(HAlign::Left)
            .valign(VAlign::Center)
            .margin(s(0.0), composer_y, input_width, composer_height),
    );
    root = root.child(
        Node::label("clear mo", fs(8.5), crate::ui::fg2(pal))
            .font(Ff::Ui, Fw::Regular)
            .halign(HAlign::Center)
            .valign(VAlign::Center)
            .margin(w - s(70.0), composer_y, s(70.0), composer_height),
    );
    
    // Category bars (top 3)
    let bar_y_start = s(90.0);
    let bar_height = s(4.0);
    let bar_spacing = s(11.0); // 90 to 101 is 11 units
    for i in 0..3.min(per.len()) {
        let (cat, val) = per[i].clone();
        let bar_y = bar_y_start + (i as f32 * bar_spacing);
        
        // Category name (truncated to 7 chars like in panels.rs)
        let cat_display = cat.chars().take(7).collect::<String>();
        root = root.child(
            Node::label(cat_display.as_str(), fs(8.0), crate::ui::fg2(pal))
                .font(Ff::Ui, Fw::Regular)
                .halign(HAlign::Left)
                .valign(VAlign::Center)
                .margin(s(12.0), bar_y, 0.0, 0.0),
        );
        
        // Value (right-aligned)
        root = root.child(
            Node::label(format!("{val:.0}").as_str(), fs(8.0), pal.fg)
                .font(Ff::Ui, Fw::Regular)
                .halign(HAlign::Right)
                .valign(VAlign::Center)
                .margin(0.0, bar_y, s(12.0), 0.0),
        );
        
        // Bar background
        root = root.child(
            Node::box_(s(4.0), crate::ui::fg2(pal))
                .sizing(Sizing::Expand(1.0))
                .fixed(bar_height)
                .margin(s(12.0), bar_y + s(1.0), s(-12.0), 0.0),
        );
        
        // Bar fill
        let bar_width = ((val / exp_max).clamp(0.0, 1.0) * (w - s(24.0))).max(s(2.0));
        root = root.child(
            Node::box_(s(4.0), pal.acc)
                .sizing(Sizing::Fixed(bar_width))
                .fixed(bar_height)
                .margin(s(12.0), bar_y + s(1.0), 0.0, 0.0),
        );
    }
    
    root
}