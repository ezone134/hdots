use crate::text::{Ff, Fw};
use crate::ui::Pal;
use crate::widgets::{HAlign, Node, VAlign};
use crate::lyrics::LyricLine;

/// Lyrics card: displays song lyrics.
pub(crate) fn lyrics(
    lyrics_lines: &[LyricLine],
    lyrics_track: &String,
    lyrics_state: u8,
    lyrics_scroll: usize,
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
            Node::label("Lyrics", fs(11.5), pal.fg)
                .font(Ff::Ui, Fw::Semibold)
                .halign(HAlign::Left)
                .margin(0.0, s(9.0), 0.0, 0.0),
        );
        root = root.child(
            Node::label("", fs(12.0), pal.acc)
                .font(Ff::Ui, Fw::Regular)
                .halign(HAlign::Left)
                .margin(s(18.0), s(9.0), 0.0, 0.0),
        );
    }
    
    // Status indicators
    let lyr_synced = lyrics_state == 2 && has_timing(lyrics_lines);
    let lyr_none = lyrics_state == 3;
    let lyr_other_lbl = match lyrics_state {
        1 => "fetching…".to_string(),
        2 => "plain".to_string(),
        _ => "—".to_string(),
    };
    let lyr_msg = match lyrics_state {
        1 => "fetching lyrics…",
        3 => "no lyrics found",
        _ => "no track playing",
    };
    let track = if lyrics_state == 2 && !lyrics_track.is_empty() {
        lyrics_track.replace(" || ", " — ")
    } else if !lyrics_track.is_empty() {
        lyrics_track.clone()
    } else {
        "—".to_string()
    };
    
    // Sync status
    if lyr_synced {
        root = root.child(
            Node::label("synced", fs(8.5), pal.acc)
                .font(Ff::Ui, Fw::Regular)
                .halign(HAlign::Right)
                .valign(VAlign::Top)
                .margin(0.0, s(10.0), 0.0, 0.0),
        );
    }
    // None status
    if lyr_none {
        root = root.child(
            Node::label("none", fs(8.5), crate::ui::DANGER)
                .font(Ff::Ui, Fw::Regular)
                .halign(HAlign::Right)
                .valign(VAlign::Top)
                .margin(0.0, s(10.0), 0.0, 0.0),
        );
    }
    // Other status
    if !lyr_synced && !lyr_none {
        root = root.child(
            Node::label(lyr_other_lbl.as_str(), fs(8.5), crate::ui::fg3(pal))
                .font(Ff::Ui, Fw::Regular)
                .halign(HAlign::Right)
                .valign(VAlign::Top)
                .margin(0.0, s(10.0), 0.0, 0.0),
        );
    }
    
    // Track name
    if !track.is_empty() {
        root = root.child(
            Node::label(track.as_str(), fs(8.5), crate::ui::fg3(pal))
                .font(Ff::Ui, Fw::Regular)
                .halign(HAlign::Left)
                .valign(VAlign::Top)
                .margin(s(16.0), s(28.0), 0.0, 0.0),
        );
    }
    
    // Message (empty state)
    if lyrics_lines.is_empty() {
        root = root.child(
        Node::label(lyr_msg, fs(9.0), crate::ui::fg3(pal))
            .font(Ff::Ui, Fw::Regular)
            .halign(HAlign::Center)
            .valign(VAlign::Center)
        );
        return root;
    }
    
    // Lyric rows
    let mut ry = s(33.0);
    let visible_lines = ((h - s(33.0)) / s(21.0)).ceil() as usize;
    let start = lyrics_scroll.min(lyrics_lines.len().saturating_sub(visible_lines));
    let end = (start + visible_lines).min(lyrics_lines.len());
    
    for i in start..end {
        if ry + s(21.0) > h + 0.5 {
            break;
        }
        let line = &lyrics_lines[i];
        let text = line.text.chars().take(60).collect::<String>();
        
        // Determine color based on sync state and karaoke (simplified)
        let color = if lyr_synced {
            // In a real implementation, we'd highlight the current word
            pal.fg
        } else {
            crate::ui::fg3(pal)
        };
        
        root = root.child(
            Node::label(text.as_str(), fs(10.5), color)
                .font(Ff::Ui, Fw::Regular)
                .halign(HAlign::Left)
                .valign(VAlign::Top)
                .margin(s(6.0), ry, 0.0, 0.0),
        );
        
        ry += s(21.0); // row_h
    }
    
    root
}

/// Check if lyrics have timing information (for sync detection)
fn has_timing(lines: &[LyricLine]) -> bool {
    lines.iter().any(|line| line.time >= 0.0)
}