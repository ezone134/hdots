use std::fs::File;
use std::io::{Seek, SeekFrom, Write};
use std::os::fd::{AsFd, FromRawFd};

pub struct ShmPool {
    file: File,
    size: usize,
}

impl ShmPool {
    pub fn new(initial: usize) -> Self {
        let file = create_shm_file(initial);
        Self { file, size: initial }
    }

    pub fn fd(&self) -> std::os::fd::BorrowedFd<'_> {
        self.file.as_fd()
    }

    pub fn ensure_size(&mut self, needed: usize) {
        if needed <= self.size {
            return;
        }
        let new_size = ((self.size * 3) / 2).max(needed);
        nix::unistd::ftruncate(&self.file.as_fd(), new_size as i64).unwrap();
        self.size = new_size;
    }

    pub fn write_pixels(&mut self, offset: usize, width: usize, height: usize, pixels: &[u32]) {
        let bytes = width * height * 4;
        self.ensure_size(offset + bytes);
        self.file.seek(SeekFrom::Start(offset as u64)).unwrap();
        self.file
            .write_all(unsafe {
                std::slice::from_raw_parts(pixels.as_ptr() as *const u8, bytes)
            })
            .unwrap();
        self.file.flush().unwrap();
    }
}

fn create_shm_file(size: usize) -> File {
    use std::ffi::CString;
    use std::os::fd::IntoRawFd;
    let name = CString::new("color_chooser_shm").unwrap();
    let fd = nix::sys::memfd::memfd_create(
        name.as_c_str(),
        nix::sys::memfd::MemFdCreateFlag::MFD_CLOEXEC,
    )
    .expect("memfd_create failed");
    nix::unistd::ftruncate(&fd, size as i64).unwrap();
    unsafe { File::from_raw_fd(fd.into_raw_fd()) }
}

pub fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (u8, u8, u8) {
    if s <= 0.0 {
        let g = (v * 255.0).round() as u8;
        return (g, g, g);
    }
    let h = (h % 360.0) / 60.0;
    let i = h.floor() as i32;
    let f = h - i as f32;
    let p = v * (1.0 - s);
    let q = v * (1.0 - s * f);
    let t = v * (1.0 - s * (1.0 - f));
    let (r, g, b) = match i {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    };
    (
        (r * 255.0).round() as u8,
        (g * 255.0).round() as u8,
        (b * 255.0).round() as u8,
    )
}

pub fn rgb_to_hsv(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
    let r = r as f32 / 255.0;
    let g = g as f32 / 255.0;
    let b = b as f32 / 255.0;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let d = max - min;
    let h = if d == 0.0 {
        0.0
    } else if max == r {
        60.0 * (((g - b) / d).rem_euclid(6.0))
    } else if max == g {
        60.0 * ((b - r) / d + 2.0)
    } else {
        60.0 * ((r - g) / d + 4.0)
    };
    let s = if max == 0.0 { 0.0 } else { d / max };
    (h, s, max)
}

#[derive(Clone)]
pub struct ColorState {
    pub hue: f32,
    pub sat: f32,
    pub val: f32,
    pub alpha: u8,
}

impl Default for ColorState {
    fn default() -> Self {
        Self { hue: 340.0, sat: 0.8, val: 1.0, alpha: 255 }
    }
}

impl PartialEq for ColorState {
    fn eq(&self, other: &Self) -> bool {
        self.hue == other.hue && self.sat == other.sat && self.val == other.val && self.alpha == other.alpha
    }
}

impl ColorState {
    pub fn rgb(&self) -> (u8, u8, u8) {
        hsv_to_rgb(self.hue, self.sat, self.val)
    }

    pub fn hex(&self) -> String {
        let (r, g, b) = self.rgb();
        if self.alpha < 255 {
            format!("#{:02X}{:02X}{:02X}{:02X}", r, g, b, self.alpha)
        } else {
            format!("#{:02X}{:02X}{:02X}", r, g, b)
        }
    }

    pub fn pixel(&self) -> u32 {
        let (r, g, b) = self.rgb();
        ((self.alpha as u32) << 24) | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
    }
}

pub fn parse_hex(text: &str) -> Option<ColorState> {
    let t = text.trim().trim_start_matches('#');
    if (t.len() != 6 && t.len() != 8) || !t.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let r = u8::from_str_radix(&t[0..2], 16).ok()?;
    let g = u8::from_str_radix(&t[2..4], 16).ok()?;
    let b = u8::from_str_radix(&t[4..6], 16).ok()?;
    let a = if t.len() == 8 {
        u8::from_str_radix(&t[6..8], 16).ok()?
    } else {
        255
    };
    let (h, s, v) = rgb_to_hsv(r, g, b);
    Some(ColorState { hue: h, sat: s, val: v, alpha: a })
}

const WHEEL_SIZE: usize = 256;

pub fn generate_wheel() -> Vec<u32> {
    let mut pixels = vec![0u32; WHEEL_SIZE * WHEEL_SIZE];
    let cx = WHEEL_SIZE as f32 / 2.0;
    let max_r = cx - 1.0;
    for y in 0..WHEEL_SIZE {
        for x in 0..WHEEL_SIZE {
            let dx = x as f32 + 0.5 - cx;
            let dy = y as f32 + 0.5 - cx;
            let dist = (dx * dx + dy * dy).sqrt();
            if dist > max_r {
                continue;
            }
            let hue = (dy.atan2(dx).to_degrees() + 360.0) % 360.0;
            let sat = (dist / max_r).clamp(0.0, 1.0);
            let (r, g, b) = hsv_to_rgb(hue, sat, 1.0);
            pixels[y * WHEEL_SIZE + x] = 0xFF000000 | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32);
        }
    }
    pixels
}

pub struct RenderContext<'a> {
    pub buf: &'a mut [u32],
    pub width: usize,
    pub height: usize,
    pub scale: i32,
}

impl<'a> RenderContext<'a> {
    pub fn clear(&mut self, color: u32) {
        self.buf.fill(color);
    }

    pub fn set_pixel(&mut self, x: i32, y: i32, color: u32) {
        let x = x * self.scale;
        let y = y * self.scale;
        self.blend_pixel_scaled(x, y, color);
    }

    fn blend_pixel_scaled(&mut self, x: i32, y: i32, color: u32) {
        if x < 0 || y < 0 || (x as usize) >= self.width || (y as usize) >= self.height {
            return;
        }
        let dst = &mut self.buf[y as usize * self.width + x as usize];
        let a = ((color >> 24) & 0xFF) as f32 / 255.0;
        if a >= 1.0 {
            *dst = color;
            return;
        }
        let da = ((*dst >> 24) & 0xFF) as f32 / 255.0;
        let dr = ((*dst >> 16) & 0xFF) as f32;
        let dg = ((*dst >> 8) & 0xFF) as f32;
        let db = (*dst & 0xFF) as f32;
        let sr = ((color >> 16) & 0xFF) as f32;
        let sg = ((color >> 8) & 0xFF) as f32;
        let sb = (color & 0xFF) as f32;
        let out_a = a + da * (1.0 - a);
        let blend = |s: f32, d: f32| -> u8 {
            if out_a <= 0.0 { 0 } else { ((s * a + d * da * (1.0 - a)) / out_a).round() as u8 }
        };
        let out_r = blend(sr, dr);
        let out_g = blend(sg, dg);
        let out_b = blend(sb, db);
        *dst = ((out_a * 255.0) as u32) << 24 | ((out_r as u32) << 16) | ((out_g as u32) << 8) | (out_b as u32);
    }

    pub fn fill_rect(&mut self, x: i32, y: i32, w: i32, h: i32, color: u32) {
        let x0 = x * self.scale;
        let y0 = y * self.scale;
        for dy in 0..(h * self.scale) {
            for dx in 0..(w * self.scale) {
                self.blend_pixel_scaled(x0 + dx, y0 + dy, color);
            }
        }
    }

    pub fn fill_circle(&mut self, cx: i32, cy: i32, radius: i32, color: u32) {
        let rs = radius * self.scale;
        let r2 = rs * rs;
        let cx = cx * self.scale;
        let cy = cy * self.scale;
        for dy in -rs..=rs {
            for dx in -rs..=rs {
                if dx * dx + dy * dy <= r2 {
                    self.blend_pixel_scaled(cx + dx, cy + dy, color);
                }
            }
        }
    }

    pub fn stroke_circle(&mut self, cx: i32, cy: i32, radius: i32, thickness: i32, color: u32) {
        let rs = radius * self.scale;
        let ts = thickness * self.scale;
        let r2_outer = (rs + ts / 2).pow(2);
        let r2_inner = (rs - ts / 2).pow(2).max(0);
        let cx = cx * self.scale;
        let cy = cy * self.scale;
        for dy in -(rs + ts)..=(rs + ts) {
            for dx in -(rs + ts)..=(rs + ts) {
                let d2 = dx * dx + dy * dy;
                if d2 >= r2_inner && d2 <= r2_outer {
                    self.blend_pixel_scaled(cx + dx, cy + dy, color);
                }
            }
        }
    }

    pub fn render_wheel(&mut self, wheel: &[u32], offset_x: i32, offset_y: i32, brightness: f32) {
        let bw = (1.0 - brightness).max(0.0).min(1.0);
        for y in 0..WHEEL_SIZE {
            for x in 0..WHEEL_SIZE {
                let px = wheel[y * WHEEL_SIZE + x];
                if ((px >> 24) & 0xFF) == 0 {
                    continue;
                }
                let final_px = if bw > 0.0 {
                    let inv = 1.0 - bw;
                    let dr = ((px >> 16) & 0xFF) as f32 * inv;
                    let dg = ((px >> 8) & 0xFF) as f32 * inv;
                    let db = (px & 0xFF) as f32 * inv;
                    0xFF000000 | ((dr as u32) << 16) | ((dg as u32) << 8) | ((db as u32) & 0xFF)
                } else {
                    px
                };
                self.fill_rect(offset_x + x as i32, offset_y + y as i32, 1, 1, final_px);
            }
        }
    }

    pub fn render_marker(&mut self, cx: i32, cy: i32) {
        self.stroke_circle(cx, cy, 7, 2, 0xFFFFFFFF);
        self.stroke_circle(cx, cy, 7, 1, 0xFF000000);
    }

    pub fn render_slider(
        &mut self,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        value: f32,
        _color: u32,
        label_color: u32,
    ) {
        self.fill_rect(x, y, w, h, 0xFF333333);
        let filled = ((w as f32) * value) as i32;
        self.fill_rect(x, y, filled, h, 0xFF777777);
        let thumb_x = x + filled - 2;
        self.fill_rect(thumb_x, y - 2, 5, h + 4, label_color);
        self.fill_rect(thumb_x + 1, y - 3, 3, h + 6, 0xFFFFFFFF);
    }

    pub fn render_hex_text(&mut self, text: &str, x: i32, y: i32, color: u32) {
        let mut cx = x;
        for ch in text.chars() {
            render_char(self, ch, cx, y, color);
            cx += 10;
        }
    }
}

fn render_char(ctx: &mut RenderContext, ch: char, x: i32, y: i32, color: u32) {
    let pattern = match ch {
        '0' => [0x7C, 0xC6, 0xCE, 0xD6, 0xE6, 0xC6, 0x7C],
        '1' => [0x30, 0x70, 0x30, 0x30, 0x30, 0x30, 0xFC],
        '2' => [0x78, 0xCC, 0x0C, 0x38, 0x60, 0xCC, 0xFC],
        '3' => [0x78, 0xCC, 0x0C, 0x38, 0x0C, 0xCC, 0x78],
        '4' => [0x1C, 0x3C, 0x6C, 0xCC, 0xFE, 0x0C, 0x0C],
        '5' => [0xFC, 0xC0, 0xF8, 0x0C, 0x0C, 0xCC, 0x78],
        '6' => [0x38, 0x60, 0xC0, 0xF8, 0xCC, 0xCC, 0x78],
        '7' => [0xFC, 0xCC, 0x0C, 0x18, 0x30, 0x30, 0x30],
        '8' => [0x78, 0xCC, 0xCC, 0x78, 0xCC, 0xCC, 0x78],
        '9' => [0x78, 0xCC, 0xCC, 0x7C, 0x0C, 0x18, 0x70],
        'A' => [0x38, 0x6C, 0xC6, 0xFE, 0xC6, 0xC6, 0xC6],
        'B' => [0xFC, 0x66, 0x66, 0x7C, 0x66, 0x66, 0xFC],
        'C' => [0x3C, 0x66, 0xC0, 0xC0, 0xC0, 0x66, 0x3C],
        'D' => [0xF8, 0x6C, 0x66, 0x66, 0x66, 0x6C, 0xF8],
        'E' => [0xFE, 0x62, 0x68, 0x78, 0x68, 0x62, 0xFE],
        'F' => [0xFE, 0x62, 0x68, 0x78, 0x68, 0x60, 0xF0],
        'O' => [0x78, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC, 0x78],
        'K' => [0xC6, 0xCC, 0xD8, 0xF0, 0xD8, 0xCC, 0xC6],
        'P' => [0xF0, 0x88, 0x88, 0xF0, 0x80, 0x80, 0x80],
        'V' => [0x42, 0x42, 0x42, 0x42, 0x24, 0x18, 0x08],
        'R' => [0xFC, 0x66, 0x66, 0x7C, 0x60, 0x6C, 0x66],
        'G' => [0x3C, 0x66, 0xC0, 0xE6, 0xC6, 0x66, 0x3C],
        'o' => [0x00, 0x78, 0xCC, 0xCC, 0xCC, 0xCC, 0x78],
        'l' => [0x60, 0x60, 0x60, 0x60, 0x60, 0x60, 0x7C],
        's' => [0x00, 0x78, 0xC0, 0x78, 0x0C, 0xCC, 0x78],
        'e' => [0x00, 0x78, 0xCC, 0xFC, 0xC0, 0xCC, 0x78],
        'p' => [0xF8, 0xCC, 0xCC, 0xF8, 0xC0, 0xC0, 0xC0],
        'y' => [0x00, 0xCC, 0xCC, 0xCC, 0x7C, 0x0C, 0x78],
        'a' => [0x00, 0x70, 0x88, 0x78, 0x08, 0x70, 0x00],
        't' => [0x00, 0x20, 0x70, 0x20, 0x20, 0x20, 0x00],
        '#' => [0x00, 0x6C, 0xFE, 0x6C, 0xFE, 0x6C, 0x00],
        'X' => [0x81, 0x42, 0x24, 0x18, 0x24, 0x42, 0x81],
        _ => [0x00; 7],
    };
    for (row, &bits) in pattern.iter().enumerate() {
        for col in 0..8 {
            if bits & (0x80 >> col) != 0 {
                ctx.set_pixel(x + col, y + row as i32, color);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_6_and_8digit() {
        let c6 = parse_hex("#aabbcc").expect("6 digit");
        assert_eq!(c6.hex(), "#AABBCC");
        assert_eq!(c6.alpha, 255);
        let c8 = parse_hex("#aabbccdd").expect("8 digit");
        assert_eq!(c8.hex(), "#AABBCCDD");
        assert_eq!(c8.alpha, 0xdd);
    }

    #[test]
    fn reject_garbage() {
        assert!(parse_hex("hello").is_none());
        assert!(parse_hex("#12345").is_none());
        assert!(parse_hex("#1234567890").is_none());
        assert!(parse_hex("").is_none());
    }

    #[test]
    fn pixel_preserves_alpha() {
        let c = parse_hex("#10203040").unwrap();
        assert_eq!(c.pixel() >> 24, 0x40);
    }
}
