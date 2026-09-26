//! color_engine — merged Rust color engine.
//!
//!   color_engine pick_from_wall <image> [mode]   pick a hex color from a
//!       wallpaper and print it, then auto-run shade generation to
//!       `$shades_list` (creating its parent dir).
//!       modes: d (default/matugen-ish), s (most saturated), f (fast),
//!              v (vibrant), b (balanced)
//!   color_engine gen_shades <hex> <outfile>      write shade_0..shade_(2*anchor)
//!       accent ramp lines to <outfile>, creating its parent dir.
//!       Env: `m` (d = dark bg→accent→white, else light black→accent→bg),
//!            `accent_anchor` (default 1000), `t_bg` (dark/light base hex).

use image::{GenericImageView, ImageReader};
use std::collections::HashMap;
use std::env;
use std::fs::{self, File};
use std::io::{BufWriter, Write};

fn rgb_hex(r: u8, g: u8, b: u8) -> String {
    format!("#{:02x}{:02x}{:02x}", r, g, b)
}

fn sample_colors(img: &image::DynamicImage, size: u32) -> HashMap<u32, u32> {
    let (w, h) = img.dimensions();
    let mut counts = HashMap::new();

    for y in 0..size {
        let sy = (y * h) / size;
        for x in 0..size {
            let sx = (x * w) / size;
            let pixel = img.get_pixel(sx, sy);

            let r = pixel[0] >> 3;
            let g = pixel[1] >> 3;
            let b = pixel[2] >> 3;

            let key = ((r as u32) << 10) | ((g as u32) << 5) | (b as u32);
            *counts.entry(key).or_insert(0) += 1;
        }
    }
    counts
}

/// Iterate color buckets in a fixed order (ascending packed key).
///
/// The Lua engine resolved near-ties by whatever order `pairs()` produced,
/// which differed run to run; this Rust port must be deterministic, so every
/// "best score" scan walks the buckets in a stable order. With strict `>`
/// comparisons a tie resolves to the first (lowest-key) bucket scanned.
fn sorted_keys(counts: &HashMap<u32, u32>) -> Vec<u32> {
    let mut keys: Vec<u32> = counts.keys().copied().collect();
    keys.sort_unstable();
    keys
}

/// Unpack a quantized bucket key back into 8-bit RGB.
fn key_to_rgb(key: u32) -> (u8, u8, u8) {
    let r = ((key >> 10) & 0x1F) as u8 * 8;
    let g = ((key >> 5) & 0x1F) as u8 * 8;
    let b = (key & 0x1F) as u8 * 8;
    (r, g, b)
}

fn pick_color(img: &image::DynamicImage) -> String {
    let counts = sample_colors(img, 128);
    let mut best_score = -1.0f64;
    let mut best_count = 0u32;
    let mut best_hex = String::from("#000000");

    for &key in &sorted_keys(&counts) {
        let count = counts[&key];
        let (r, g, b) = key_to_rgb(key);

        let max_c = r.max(g).max(b);
        let min_c = r.min(g).min(b);
        let chroma = max_c - min_c;

        let luma = (r as f64) * 0.299 + (g as f64) * 0.587 + (b as f64) * 0.114;

        if luma >= 20.0 && luma <= 245.0 && chroma >= 10 {
            let score = (chroma as f64) * ((count as f64) + 1.0).ln();
            if score > best_score || (score == best_score && count > best_count) {
                best_score = score;
                best_count = count;
                best_hex = rgb_hex(r, g, b);
            }
        }
    }
    best_hex
}

fn run_saturated(img: &image::DynamicImage) -> String {
    let counts = sample_colors(img, 140);
    let mut best_sat = -1i32;
    let mut best_count = 0u32;
    let mut best_hex = String::from("#000000");

    for &key in &sorted_keys(&counts) {
        let count = counts[&key];
        let (r, g, b) = key_to_rgb(key);

        let bright = ((r as u32) * 299 + (g as u32) * 587 + (b as u32) * 114) / 1000;

        if bright <= 230 && bright >= 40 {
            let max_c = r.max(g).max(b);
            let min_c = r.min(g).min(b);
            let sat = (max_c - min_c) as i32;

            if sat > best_sat || (sat == best_sat && count > best_count) {
                best_sat = sat;
                best_count = count;
                best_hex = rgb_hex(r, g, b);
            }
        }
    }
    best_hex
}

fn run_fast(img: &image::DynamicImage) -> String {
    let counts = sample_colors(img, 100);
    let mut list: Vec<(u32, u32)> = counts.iter().map(|(&k, &c)| (c, k)).collect();
    list.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));

    if list.len() >= 3 {
        let (_, key) = list[2];
        let (r, g, b) = key_to_rgb(key);
        return rgb_hex(r, g, b);
    }
    String::from("#000000")
}

fn run_vibrant(img: &image::DynamicImage) -> String {
    let counts = sample_colors(img, 160);
    let mut best_score = -1i64;
    let mut best_hex = String::from("#FFFFFF");

    for &key in &sorted_keys(&counts) {
        let count = counts[&key];
        let (r, g, b) = key_to_rgb(key);

        let max_c = r.max(g).max(b);
        let min_c = r.min(g).min(b);
        let chroma = (max_c - min_c) as u32;
        let luma = (0.2126 * (r as f64) + 0.7152 * (g as f64) + 0.0722 * (b as f64)) as u32;

        if chroma > 30 && luma > 40 && luma < 220 {
            let score = (count * chroma) as i64;
            if score > best_score {
                best_score = score;
                best_hex = rgb_hex(r, g, b);
            }
        }
    }
    best_hex
}

/// Finds the most 'average' color that isn't a shade of grey. Mirrors the Lua
/// loop, which walks the **byte** buffer in steps of 300 (i.e. every 100th
/// pixel) — not 300 pixels — so we step the byte index and derive the pixel
/// as `byte/3`.
fn run_balanced(img: &image::DynamicImage) -> String {
    let mut total_r = 0u64;
    let mut total_g = 0u64;
    let mut total_b = 0u64;
    let mut samples = 0u64;

    let (w, h) = img.dimensions();
    let total_bytes = (w as u64) * (h as u64) * 3;

    let mut i: u64 = 0;
    while i < total_bytes {
        let pi = (i / 3) as u32;
        let x = pi % w;
        let y = pi / w;
        let pixel = img.get_pixel(x, y);
        let r = pixel[0];
        let g = pixel[1];
        let b = pixel[2];

        if (r as i32 - g as i32).abs() > 20 || (g as i32 - b as i32).abs() > 20 {
            total_r += r as u64;
            total_g += g as u64;
            total_b += b as u64;
            samples += 1;
        }
        i += 300;
    }

    if samples == 0 {
        return String::from("#444444");
    }
    rgb_hex(
        (total_r / samples) as u8,
        (total_g / samples) as u8,
        (total_b / samples) as u8,
    )
}

fn pick_hex(img_path: &str, mode: &str) -> String {
    let img = ImageReader::open(img_path).unwrap().decode().unwrap();
    match mode {
        "d" => pick_color(&img),
        "s" => run_saturated(&img),
        "v" => run_vibrant(&img),
        "b" => run_balanced(&img),
        _ => run_fast(&img),
    }
}

#[inline]
fn hex_nibble(b: u8) -> u8 {
    match b {
        b'0'..=b'9' => b - b'0',
        b'a'..=b'f' => b - b'a' + 10,
        b'A'..=b'F' => b - b'A' + 10,
        _ => 0,
    }
}

#[inline]
fn hex_to_rgb(h: &str) -> (u8, u8, u8) {
    let h = h.as_bytes();
    (
        (hex_nibble(h[0]) << 4) | hex_nibble(h[1]),
        (hex_nibble(h[2]) << 4) | hex_nibble(h[3]),
        (hex_nibble(h[4]) << 4) | hex_nibble(h[5]),
    )
}

#[inline]
fn clamp_u8(v: i32) -> u8 {
    v.max(0).min(255) as u8
}

#[inline]
fn write_hex_byte(buf: &mut [u8; 2], val: u8) {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    buf[0] = HEX[(val >> 4) as usize];
    buf[1] = HEX[(val & 0x0F) as usize];
}

/// Generate `shade_0..shade_(2*accent_anchor)="#RRGGBB"` ramp lines into
/// `<outpath>` (parent dir auto-created). Env: `m`, `accent_anchor`, `t_bg`.
fn gen_shades(hex_raw: &str, outpath: &str) {
    let hex_raw = hex_raw.trim_start_matches('#');

    if let Some(parent) = std::path::Path::new(outpath).parent() {
        fs::create_dir_all(parent).unwrap();
    }

    let mode = env::var("m").unwrap_or_default();
    let accent_anchor: i32 = env::var("accent_anchor")
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(1000);
    let t_bg = env::var("t_bg").unwrap_or_default();

    let light_anchor = accent_anchor * 2;

    let (dark_hex, light_hex) = if mode == "d" {
        (t_bg.as_str(), "ffffff")
    } else {
        ("000000", t_bg.as_str())
    };

    let (dr, dg, db) = hex_to_rgb(dark_hex.trim_start_matches('#'));
    let (r0, g0, b0) = hex_to_rgb(hex_raw);
    let (lr, lg, lb) = hex_to_rgb(light_hex.trim_start_matches('#'));

    let (dr, dg, db) = (dr as i32, dg as i32, db as i32);
    let (r0, g0, b0) = (r0 as i32, g0 as i32, b0 as i32);
    let (lr, lg, lb) = (lr as i32, lg as i32, lb as i32);

    let mut w = BufWriter::with_capacity(64 * 1024, File::create(outpath).unwrap());

    let mut line_buf = [0u8; 32];
    let mut hex_buf = [0u8; 2];

    for i in 0..=light_anchor {
        let (r, g, b) = if i <= accent_anchor {
            (
                dr + (r0 - dr) * i / accent_anchor,
                dg + (g0 - dg) * i / accent_anchor,
                db + (b0 - db) * i / accent_anchor,
            )
        } else {
            let t = i - accent_anchor;
            (
                r0 + (lr - r0) * t / accent_anchor,
                g0 + (lg - g0) * t / accent_anchor,
                b0 + (lb - b0) * t / accent_anchor,
            )
        };

        let r = clamp_u8(r);
        let g = clamp_u8(g);
        let b = clamp_u8(b);

        let prefix_len = {
            let mut pos = 6;
            line_buf[0..6].copy_from_slice(b"shade_");
            let mut tmp = [0u8; 7];
            let mut n = 0;
            let mut val = i;
            loop {
                tmp[n] = b'0' + (val % 10) as u8;
                val /= 10;
                n += 1;
                if val == 0 { break; }
            }
            let mut j = n;
            while j > 0 {
                j -= 1;
                line_buf[pos] = tmp[j];
                pos += 1;
            }
            line_buf[pos] = b'=';
            pos += 1;
            line_buf[pos] = b'"';
            pos += 1;
            line_buf[pos] = b'#';
            pos += 1;
            pos
        };

        write_hex_byte(&mut hex_buf, r);
        line_buf[prefix_len] = hex_buf[0];
        line_buf[prefix_len + 1] = hex_buf[1];
        write_hex_byte(&mut hex_buf, g);
        line_buf[prefix_len + 2] = hex_buf[0];
        line_buf[prefix_len + 3] = hex_buf[1];
        write_hex_byte(&mut hex_buf, b);
        line_buf[prefix_len + 4] = hex_buf[0];
        line_buf[prefix_len + 5] = hex_buf[1];

        let end = prefix_len + 6;
        line_buf[end] = b'"';
        line_buf[end + 1] = b'\n';

        let _ = w.write_all(&line_buf[..end + 2]);
    }

    w.flush().unwrap();
}

fn main() {
    let args: Vec<String> = env::args().collect();
    match args[1].as_str() {
        // pick_from_wall <image> [mode] → prints hex, then auto-runs the
        // shade generator to $shades_list (parent dir auto-created inside
        // gen_shades).
        "pick_from_wall" => {
            let img = &args[2];
            let mode = args.get(3).map(|s| s.as_str()).unwrap_or("f");
            let hex = pick_hex(img, mode);
            println!("{}", hex);

            let shades_out = env::var("shades_list").unwrap();
            gen_shades(&hex, &shades_out);
        }
        // gen_shades <hex> <outfile> → writes the accent ramp file.
        "gen_shades" => {
            gen_shades(&args[2], &args[3]);
        }
        _ => panic!("unknown subcommand: {}", args[1]),
    }
}