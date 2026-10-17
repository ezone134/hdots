use crate::text::{Ff, Fw};
use crate::ui::Pal;
use crate::widgets::{HAlign, Node, Sizing, VAlign};

/// Currency card: displays currency exchange rates.
pub(crate) fn currency(
    currency_base: &str,
    currency_scroll: usize,
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
    if show_title {
        root = root.child(
            Node::label("Currency", fs(11.5), pal.fg)
                .font(Ff::Ui, Fw::Semibold)
                .halign(HAlign::Left)
                .margin(0.0, s(9.0), 0.0, 0.0),
        );
        root = root.child(
            Node::label("", fs(12.0), pal.acc)
                .font(Ff::Ui, Fw::Regular)
                .halign(HAlign::Left)
                .margin(s(12.0), s(9.0), 0.0, 0.0),
        );
    }
    
    // Currency chip (shows base symbol if not USD)
    if currency_base != "USD" {
        root = root.child(
            Node::label(currency_base, fs(8.5), pal.acc)
                .font(Ff::Ui, Fw::Regular)
                .halign(HAlign::Right)
                .valign(VAlign::Center)
                .margin(0.0, s(9.0), s(12.0), 0.0),
        );
    }
    
    // Currency rates list
    const CCY: [(&str, &str, f64); 12] = [
        ("USD", "Dollar", 1.0),
        ("EUR", "Euro", 0.9200),
        ("GBP", "Pound", 0.7900),
        ("JPY", "Yen", 149.50),
        ("INR", "Rupee", 83.10),
        ("CNY", "Yuan", 7.2400),
        ("RUB", "Ruble", 92.50),
        ("AUD", "AU Dollar", 1.5200),
        ("CAD", "CA Dollar", 1.3700),
        ("KRW", "Won", 1347.00),
        ("XAU", "Gold oz", 0.000420),
        ("XBT", "BTC", 0.0000160),
    ];
    
    let base = currency_base;
    let base_mult = CCY
        .iter()
        .find(|(s, _, _)| *s == base)
        .map(|(_, _, m)| *m)
        .unwrap_or(1.0);
    
    let mut ry = s(26.0);
    for &(sym, label, mult) in CCY.iter() {
        if ry + s(20.0) > _h + 0.5 {
            break;
        }
        
        let active = sym == base;
        let val = mult / base_mult;
        let vstr = if val >= 100.0 {
            format!("{:.0}", val)
        } else if val >= 10.0 {
            format!("{:.1}", val)
        } else {
            format!("{:.2}", val)
        };
        
        // Create row with three columns
        let row = Node::row()
            .child(
                Node::label(sym, fs(10.0), 
                           if active { pal.acc } else { pal.fg })
                    .font(Ff::Ui, Fw::Regular)
                    .halign(HAlign::Left)
                    .valign(VAlign::Center)
            )
            .child(
                Node::label(label, fs(8.0), 
                           if active { pal.acc } else { crate::ui::fg3(pal) })
                    .font(Ff::Ui, Fw::Regular)
                    .halign(HAlign::Left)
                    .valign(VAlign::Center)
            )
            .child(
                Node::label(
                    format!("1 {} = {} {}", base, vstr, sym).as_str(),
                    fs(9.5),
                    if active { pal.acc } else { pal.fg }
                )
                .font(Ff::Ui, Fw::Regular)
                .halign(HAlign::Right)
                .valign(VAlign::Center)
            )
            .margin(s(6.0), ry, s(0.0), 0.0)
            .sizing(Sizing::Expand(1.0));
        
        // Apply background if active
        if active {
            root = root.child(
                Node::box_(s(4.0), crate::ui::acc_tint(pal))
                    .sizing(Sizing::Expand(1.0))
                    .fixed(s(20.0))
                    .margin(s(0.0), ry, s(0.0), 0.0)
                    .child(row),
            );
        } else {
            root = root.child(row);
        }
        
        ry += s(20.0); // row_h + pad
    }
    
    root
}