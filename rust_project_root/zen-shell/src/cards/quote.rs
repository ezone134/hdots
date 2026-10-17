use crate::text::{Ff, Fw};
use crate::ui::Pal;
use crate::widgets::{HAlign, Node, VAlign};

/// Quote card: displays inspirational quotes.
pub(crate) fn quote(
    quote_text: &String,
    quote_author: &String,
    quote_state: u8,
    quote_saved_flash: u32,
    saved_quotes: &[String],
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
            Node::label("Quote", fs(11.5), pal.fg)
                .font(Ff::Ui, Fw::Semibold)
                .halign(HAlign::Left)
                .margin(0.0, s(9.0), 0.0, 0.0),
        );
        root = root.child(
            Node::label("", fs(12.0), pal.acc)
                .font(Ff::Ui, Fw::Regular)
                .halign(HAlign::Left)
                .margin(s(12.0), s(9.0), 0.0, 0.0),
        );
    }
    
    // Meta text (top right)
    let quote_has = quote_state == 2 && !quote_text.is_empty();
    let (qmeta, qmeta_ink) = if quote_state == 2 && quote_saved_flash > 0 {
        (format!("{} saved", ICON_CHECK), 0x2ee6a8ff)
    } else {
        match quote_state {
            1 => (
                "fetching…".to_string(),
                crate::ui::fg3(pal)
            ),
            2 => (
                format!("{} saved", saved_quotes.len()),
                crate::ui::fg3(pal)
            ),
            _ => (
                "—".to_string(),
                crate::ui::fg3(pal)
            ),
        }
    };
    root = root.child(
        Node::label(qmeta.as_str(), fs(8.5), qmeta_ink)
            .font(Ff::Ui, Fw::Regular)
            .halign(HAlign::Right)
            .valign(VAlign::Top)
            .margin(s(12.0), s(9.0), 0.0, s(12.0)),
    );
    
    // Quote text and author (visible when quote_has is true)
    if quote_has {
        let quote_display = format!("{} — {}", quote_text, quote_author);
        root = root.child(
            Node::label(quote_display.as_str(), fs(10.0), pal.fg)
                .font(Ff::Ui, Fw::Regular)
                .halign(HAlign::Center)
                .valign(VAlign::Top)
        );
    }
    
    // Fetching message
    if quote_state == 1 {
        root = root.child(
            Node::label("fetching a quote…", fs(9.0), crate::ui::fg3(pal))
                .font(Ff::Ui, Fw::Regular)
                .halign(HAlign::Center)
                .valign(VAlign::Center)
        );
    }
    
    // Empty message
    if quote_state != 1 && quote_text.is_empty() {
        root = root.child(
            Node::label("no quote yet", fs(9.0), crate::ui::fg3(pal))
                .font(Ff::Ui, Fw::Regular)
                .halign(HAlign::Center)
                .valign(VAlign::Center)
        );
    }
    
    // New quote button (bottom left)
    root = root.child(
        Node::label("  new", fs(10.0), crate::ui::fg2(pal))
            .font(Ff::Ui, Fw::Regular)
            .halign(HAlign::Left)
            .valign(VAlign::Bottom)
            .margin(s(-50.0), s(13.0), 0.0, 0.0),
    );
    
    // Save quote button (bottom right)
    root = root.child(
        Node::label("  save", fs(10.0), crate::ui::fg2(pal))
            .font(Ff::Ui, Fw::Regular)
            .halign(HAlign::Right)
            .valign(VAlign::Bottom)
            .margin(s(50.0), s(13.0), 0.0, 0.0),
    );
    
    root
}

// Icon constants (copied from shell.rs)
const ICON_CHECK: &str = "";