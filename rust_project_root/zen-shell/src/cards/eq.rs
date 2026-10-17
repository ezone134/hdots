use crate::text::{Ff, Fw};
use crate::ui::Pal;
use crate::widgets::{HAlign, Node, Sizing, VAlign};
use crate::eq::{EqState, BAND_LABELS, GAIN_MIN, GAIN_MAX};

/// Equalizer card: displays audio equalizer bands.
pub(crate) fn eq(
    eq_state: &EqState,
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
    let eq = eq_state.lock().unwrap();
    root = root.child(
        Node::label("", fs(12.0), crate::ui::fg2(pal))
            .font(Ff::Ui, Fw::Regular)
            .halign(HAlign::Left)
            .margin(0.0, s(8.0), 0.0, 0.0),
    );
    root = root.child(
        Node::label("Equalizer", fs(12.0), pal.fg)
            .font(Ff::Ui, Fw::Semibold)
            .halign(HAlign::Left)
            .margin(s(30.0), s(9.0), 0.0, 0.0),
    );
    
    // Toggle switch
    let toggle_width = s(38.0);
    let toggle_height = s(18.0);
    let knob_size = s(14.0);
    let knob_position = if eq.active { toggle_width - knob_size } else { s(0.0) };
    
    // Toggle background
    root = root.child(
        Node::box_(s(4.0), 
                   if eq.active { pal.acc } else { crate::ui::fg2(pal) })
            .sizing(Sizing::Fixed(toggle_width))
            .fixed(toggle_height)
            .margin(w - s(54.0), s(5.0), 0.0, 0.0),
    );
    
    // Toggle knob
    root = root.child(
        Node::box_(s(4.0), pal.fg)
            .sizing(Sizing::Fixed(knob_size))
            .fixed(knob_size)
            .margin(w - s(54.0) + knob_position, s(5.0) + s(2.0), 0.0, 0.0),
    );
    
    // Toggle label (simplified - in scene this shows text like "On"/"Off")
    root = root.child(
        Node::label("", fs(8.5), pal.fg)
            .font(Ff::Ui, Fw::Regular)
            .halign(HAlign::Center)
            .valign(VAlign::Center)
            .margin(w - s(54.0), s(8.0), 0.0, 0.0),
    );
    
    // EQ bands visualization (only when active)
    if eq.active {
        let eq_width = w - s(24.0);
        let eq_height = s(50.0);
        let eq_x = s(12.0);
        let eq_y = s(32.0);
        
        // EQ panel background
        root = root.child(
            Node::box_(s(4.0), crate::ui::fg2(pal))
                .sizing(Sizing::Fixed(eq_width))
                .fixed(eq_height)
                .margin(eq_x, eq_y, 0.0, 0.0),
        );
        
        // Draw 10 band bars
        let band_width = eq_width / 10.0 - s(2.0); // Band width with gap
        let max_gain = GAIN_MAX - GAIN_MIN; // 24.0 dB range
        
        for (i, &gain) in eq.bands.iter().enumerate() {
            let band_x = eq_x + s(2.0) + (i as f32 * (eq_width / 10.0));
            let band_y = eq_y + eq_height - s(4.0); // Start from bottom
            
            // Calculate bar height based on gain (GAIN_MIN to GAIN_MAX maps to 0 to eq_height)
            let gain_normalized = (gain - GAIN_MIN) / max_gain; // 0.0 to 1.0
            let bar_height = gain_normalized * eq_height * 0.8; // 80% of available height
            
            // Bar color based on gain (negative = cool colors, positive = warm colors)
            let bar_color = if gain >= 0.0 {
                // Positive gain: green to red
                crate::ui::mix(pal.acc, pal.fg, gain_normalized * 0.5)
            } else {
                // Negative gain: blue to green
                crate::ui::mix(crate::ui::fg2(pal), pal.acc, (gain_normalized - 0.5).abs())
            };
            
            // Bar background (groove)
            root = root.child(
                Node::box_(s(4.0), crate::ui::fg2(pal))
                    .sizing(Sizing::Fixed(band_width))
                    .fixed(eq_height * 0.8)
                    .margin(band_x, eq_y + s(4.0), 0.0, 0.0),
            );
            
            // Bar fill (gain)
            root = root.child(
                Node::box_(s(4.0), bar_color)
                    .sizing(Sizing::Fixed(band_width))
                    .fixed(bar_height)
                    .margin(band_x, eq_y + eq_height - s(4.0) - bar_height, 0.0, 0.0),
            );
            
            // Band label
            root = root.child(
                Node::label(BAND_LABELS[i], fs(7.5), pal.fg)
                    .font(Ff::Ui, Fw::Regular)
                    .halign(HAlign::Center)
                    .valign(VAlign::Top)
                    .margin(band_x, eq_y + eq_height + s(2.0), band_width, 0.0),
            );
        }
    } else {
        // Show "Press ON to enable" message when inactive
        root = root.child(
            Node::label("Press ON to enable", fs(10.0), crate::ui::fg2(pal))
                .font(Ff::Ui, Fw::Regular)
                .halign(HAlign::Center)
                .valign(VAlign::Center)
                .margin(s(12.0), h / 2.0, w - s(24.0), 0.0),
        );
    }
    
    root
}