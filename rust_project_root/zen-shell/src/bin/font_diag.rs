//! Font diagnostic: does the configured family resolve in fontdb, and do the
//! Nerd Font icons rasterize with real ink (not tofu)?
//!
//! Run: cargo run --bin font_diag

use cosmic_text::fontdb;
use cosmic_text::{Attrs, Buffer, Color, Family, FontSystem, Metrics, Shaping, SwashCache};

fn main() {
    let mut db = fontdb::Database::new();
    db.load_system_fonts();
    println!("fontdb loaded {} fonts", db.len());
    if db.is_empty() {
        println!("FAIL: no fonts loaded at all");
        std::process::exit(1);
    }

    let families = ["JetBrainsMono Nerd Font", "Hack Nerd Font", "JetBrainsMono"];
    for fam in families {
        match db.query(&fontdb::Query {
            families: &[fontdb::Family::Name(fam)],
            ..Default::default()
        }) {
            Some(id) => {
                let info = db.face(id).unwrap();
                println!("OK   {fam:30} -> {}", info.families.first().map(|f| f.0.clone()).unwrap_or_default());
            }
            None => println!("MISS {fam:30} -> NOT FOUND"),
        }
    }

    // rasterize the icon glyphs + a clock string, count ink pixels
    let mut fs = FontSystem::new_with_locale_and_db("en".into(), db);
    let mut swash = SwashCache::new();
    let tests: [(&str, f32, bool); 6] = [
        ("\u{f0f3}", 13.0, true),   // bell
        ("\u{f489}", 13.0, true),   // apps
        ("\u{f002}", 14.0, true),   // search
        ("\u{f073}", 14.0, true),   // calendar
        ("17:21", 14.0, false),     // clock
        ("100%", 12.0, false),      // battery
    ];
    let mut ok = true;
    for (text, size, _icon) in tests {
        let attrs = Attrs::new().family(Family::Name("JetBrainsMono Nerd Font"));
        let mut buf = Buffer::new(&mut fs, Metrics::new(size, size * 1.3));
        buf.set_size(Some(256.0), Some(64.0));
        buf.set_text(text, &attrs, Shaping::Advanced, None);
        buf.shape_until_scroll(&mut fs, true);
        let mut ink = 0u32;
        buf.draw(&mut fs, &mut swash, Color(0xffffffff), |_x, _y, _w, _h, c| {
            if c.a() > 0 {
                ink += 1;
            }
        });
        let status = if ink > 0 { "PASS" } else { "FAIL (tofu)" };
        if ink == 0 {
            ok = false;
        }
        println!("{status} ink={ink:4} {text:8} size={size}");
    }
    if !ok {
        std::process::exit(1);
    }
}
