use crate::text::{Ff, Fw};
use crate::ui::Pal;
use crate::widgets::{HAlign, Node, Sizing, VAlign};

/// Calendar card: displays a monthly calendar grid.
pub(crate) fn calendar(
    cal_offset: i32,
    show_title: bool,
    _w: f32,
    _h: f32,
    grid: f32,
    pal: &Pal,
) -> Node {
    let s = |n: f32| n * grid;
    let fs = |n: f32| (n * grid * 10.0).round() / 10.0;
    let mut root = Node::box_(0.0, 0);
    
    // Calculate calendar info like in panels.rs (needed for both title and grid)
    let calendar_info = {
        unsafe {
            let now = libc::time(std::ptr::null_mut());
            let mut today_tm: libc::tm = std::mem::zeroed();
            libc::localtime_r(&now, &mut today_tm);
            let abs =
                today_tm.tm_year as i64 * 12 + today_tm.tm_mon as i64 + cal_offset as i64;
            let (y, m) = (abs.div_euclid(12), abs.rem_euclid(12));
            let mut first: libc::tm = std::mem::zeroed();
            first.tm_year = y as i32;
            first.tm_mon = m as i32;
            first.tm_mday = 1;
            first.tm_hour = 12;
            first.tm_isdst = -1;
            let _ = libc::mktime(&mut first);
            let first_wday = first.tm_wday as i32;
            let mut last: libc::tm = std::mem::zeroed();
            last.tm_year = y as i32;
            last.tm_mon = m as i32;
            last.tm_mday = 0;
            last.tm_hour = 12;
            last.tm_isdst = -1;
            let _ = libc::mktime(&mut last);
            let days = last.tm_mday as i32;
            let mut buf = [0u8; 64];
            let n = libc::strftime(
                buf.as_mut_ptr() as *mut libc::c_char,
                buf.len(),
                c"%B %Y".as_ptr(),
                &first,
            );
            let month = String::from_utf8_lossy(&buf[..n]).into_owned();
            (first_wday, days, today_tm.tm_mday as i32, month)
        }
    };
    let (first_wday, days, today, month) = calendar_info;
    
    if show_title {
        root = root.child(
            Node::label(month.as_str(), fs(11.5), pal.acc)
                .font(Ff::Ui, Fw::Semibold)
                .halign(HAlign::Center)
                .margin(0.0, s(9.0), 0.0, 0.0),
        );
    }
    
    // Calendar grid (6x7 matrix)
    let mut rx = s(0.0);
    let mut ry = if show_title { s(24.0) } else { s(0.0) };
    let cell_w = _w / 7.0;
    let cell_h = (_h - ry) / 6.0;
    
    // Weekday headers (from RON head row: ["S","M","T","W","T","F","S"])
    let weekdays = ["S", "M", "T", "W", "T", "F", "S"];
    for (col, day) in weekdays.iter().enumerate() {
        root = root.child(
            Node::label(*day, fs(9.0), crate::ui::fg2(pal))
                .font(Ff::Ui, Fw::Regular)
                .halign(HAlign::Center)
                .valign(VAlign::Center)
                .margin(rx + col as f32 * cell_w, ry, cell_w, cell_h * 0.3),
        );
    }
    
    // Calendar days
    let mut ry_days = ry + s(18.0); // Start below weekday headers
    for cell in 0..42 {
        let day = cell as i32 - first_wday + 1;
        let visible = day >= 1 && day <= days;
        let is_today = cal_offset == 0 && day == today;
        
        if !visible {
            // Empty cell for off-month days
            continue;
        }
        
        let col = (cell % 7) as f32;
        let row = (cell / 7) as f32;
        let cell_x = rx + col * cell_w;
        let cell_y = ry_days + row * cell_h;
        
        // Day background (today gets accent tint)
        if is_today {
            root = root.child(
                Node::box_(s(4.0), crate::ui::acc_tint(pal))
                    .sizing(Sizing::Fixed(cell_w))
                    .fixed(cell_h * 0.7)
                    .margin(cell_x, cell_y, 0.0, 0.0),
            );
        }
        
        // Day number
        root = root.child(
            Node::label(day.to_string().as_str(), fs(9.0), 
                       if is_today { pal.fg } else { crate::ui::fg2(pal) })
                .font(Ff::Ui, Fw::Regular)
                .halign(HAlign::Center)
                .valign(VAlign::Center)
                .margin(cell_x, cell_y, cell_w, cell_h * 0.7)
        );
    }
    
    root
}