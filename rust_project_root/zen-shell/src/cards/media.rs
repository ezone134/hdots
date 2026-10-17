use crate::text::{Ff, Fw};
use crate::ui::Pal;
use crate::widgets::{HAlign, Node, Sizing, VAlign};

/// Media card: displays now-playing information.
pub(crate) fn media(
    media_title: &String,
    media_artist: &String,
    media_album: &String,
    media_playing: bool,
    media_pos: i64,
    media_len: i64,
    media_art: &String,
    media_seek: &Option<f32>,
    show_title: bool,
    w: f32,
    h: f32,
    grid: f32,
    pal: &Pal,
) -> Node {
    let s = |n: f32| n * grid;
    let fs = |n: f32| (n * grid * 10.0).round() / 10.0;
    let mut root = Node::box_(0.0, 0);
    
    // Calculate current position and fraction
    let cur_pos = media_seek
        .map(|f| (f * media_len.max(1) as f32) as i64)
        .unwrap_or_else(|| {
            // Simple interpolation - in reality this would use media_pos_at
            media_pos.max(0)
        });
    let frac = if media_len > 0 {
        cur_pos as f32 / media_len as f32
    } else {
        0.0
    };
    let frac = frac.clamp(0.0, 1.0);
    
    // Format time functions (simplified version of shell::fmt_time)
    let fmt_time = |mut micros: i64| {
        if micros < 0 {
            micros = 0;
        }
        let seconds = (micros / 1_000_000) % 60;
        let minutes = (micros / 1_000_000) / 60;
        format!("{:02}:{:02}", minutes, seconds)
    };
    
    // Header (if show_title is true, though media card doesn't typically use this)
    if show_title {
        root = root.child(
            Node::label("Media", fs(11.5), pal.fg)
                .font(Ff::Ui, Fw::Semibold)
                .halign(HAlign::Left)
                .margin(0.0, s(9.0), 0.0, 0.0),
        );
    }
    
    // Album art or placeholder
    let art_size = s(48.0);
    let art_x = s(14.0);
    let art_y = s(12.0);
    
    if media_art.is_empty() {
        // Show placeholder with music icon
        root = root.child(
            Node::box_(s(4.0), crate::ui::fg2(pal))
                .sizing(Sizing::Fixed(art_size))
                .fixed(art_size)
                .margin(art_x, art_y, 0.0, 0.0),
        );
        root = root.child(
            Node::label("", fs(16.0), pal.fg)
                .font(Ff::Ui, Fw::Regular)
                .halign(HAlign::Center)
                .valign(VAlign::Center)
                .margin(art_x, art_y, art_size, art_size),
        );
    } else {
        // In a real implementation, we'd load and display the image
        // For now, show a placeholder indicating art would be here
        root = root.child(
            Node::box_(s(4.0), crate::ui::fg2(pal))
                .sizing(Sizing::Fixed(art_size))
                .fixed(art_size)
                .margin(art_x, art_y, 0.0, 0.0),
        );
        root = root.child(
            Node::label("[Art]", fs(10.0), pal.fg)
                .font(Ff::Ui, Fw::Regular)
                .halign(HAlign::Center)
                .valign(VAlign::Center)
                .margin(art_x, art_y, art_size, art_size),
        );
    }
    
    // Title (wide and narrow variants)
    let title_y = s(14.0);
    // In a real implementation, we'd check if title fits and choose wide/narrow
    // For now, always show the title
    root = root.child(
        Node::label(media_title.as_str(), fs(12.0), pal.fg)
            .font(Ff::Ui, Fw::Semibold)
            .halign(HAlign::Left)
            .valign(VAlign::Top)
            .margin(s(72.0), title_y, 0.0, 0.0),
    );
    root = root.child(
        Node::label(media_title.as_str(), fs(12.0), pal.fg)
            .font(Ff::Ui, Fw::Semibold)
            .halign(HAlign::Left)
            .valign(VAlign::Top)
            .margin(s(14.0), title_y, 0.0, 0.0),
    );
    
    // Line 2 (artist/album info)
    let line2_y = s(32.0);
    let line2 = if !media_artist.is_empty() {
        if !media_album.is_empty() {
            format!("{} — {}", media_artist, media_album)
        } else {
            media_artist.clone()
        }
    } else if !media_album.is_empty() {
        media_album.clone()
    } else {
        "".to_string()
    };
    // In a real implementation, we'd check if line2 fits and choose wide/narrow
    // For now, always show the line2
    root = root.child(
        Node::label(line2.as_str(), fs(9.5), pal.fg)
            .font(Ff::Ui, Fw::Regular)
            .halign(HAlign::Left)
            .valign(VAlign::Top)
            .margin(s(72.0), line2_y, 0.0, 0.0),
    );
    root = root.child(
        Node::label(line2.as_str(), fs(9.5), pal.fg)
            .font(Ff::Ui, Fw::Regular)
            .halign(HAlign::Left)
            .valign(VAlign::Top)
            .margin(s(14.0), line2_y, 0.0, 0.0),
    );
    
    // Progress bar
    let bar_y = s(80.0);
    let bar_height = s(5.0);
    let bar_width = w - s(28.0); // Account for margins
    let filled_width = (bar_width * frac).max(s(2.0)); // Minimum 2px for visibility
    
    // Bar background (track)
    root = root.child(
        Node::box_(s(4.0), crate::ui::fg2(pal))
            .sizing(Sizing::Fixed(bar_width))
            .fixed(bar_height)
            .margin(s(14.0), bar_y, s(-14.0), 0.0),
    );
    
    // Bar fill (progress)
    root = root.child(
        Node::box_(s(4.0), pal.acc)
            .sizing(Sizing::Fixed(filled_width))
            .fixed(bar_height)
            .margin(s(14.0), bar_y, 0.0, 0.0),
    );
    
    // Time labels
    let time_y = s(88.0);
    root = root.child(
        Node::label(fmt_time(cur_pos).as_str(), fs(8.0), pal.fg)
            .font(Ff::Ui, Fw::Regular)
            .halign(HAlign::Left)
            .valign(VAlign::Center)
            .margin(s(16.0), time_y, 0.0, 0.0),
    );
    root = root.child(
        Node::label(fmt_time(media_len).as_str(), fs(8.0), pal.fg)
            .font(Ff::Ui, Fw::Regular)
            .halign(HAlign::Right)
            .valign(VAlign::Center)
            .margin(0.0, time_y, s(12.0), 0.0),
    );
    
    // Playback controls (simplified - just show glyphs)
    let control_y = s(102.0);
    let control_size = s(40.0);
    let bw = s(w); // base scale factor
    let prev_x = bw / 2.0 - s(36.0);
    let play_x = bw / 2.0;
    let next_x = bw / 2.0 + s(36.0);
    
    // Previous button
    root = root.child(
        Node::label("", fs(14.0), pal.fg)
            .font(Ff::Ui, Fw::Regular)
            .halign(HAlign::Center)
            .valign(VAlign::Center)
            .margin(prev_x, control_y, 0.0, 0.0),
    );
    
    // Play/Pause button
    let play_pause_glyph = if media_playing { "" } else { "" };
    root = root.child(
        Node::label(play_pause_glyph, fs(17.0), 
                   if media_playing { pal.acc } else { pal.fg })
            .font(Ff::Ui, Fw::Regular)
            .halign(HAlign::Center)
            .valign(VAlign::Center)
            .margin(play_x, control_y, 0.0, 0.0),
    );
    
    // Next button
    root = root.child(
        Node::label("", fs(15.0), pal.fg)
            .font(Ff::Ui, Fw::Regular)
            .halign(HAlign::Center)
            .valign(VAlign::Center)
            .margin(next_x, control_y, 0.0, 0.0),
    );
    
    root
}