//! Glyph atlas: a single GPU texture, bump-allocated rows, cached glyphs.
//! Glyphs are rasterized once (pixels + raster offsets are retained so the
//! atlas can be re-packed without re-rasterizing when it outgrows its size).

use ab_glyph::{Font, ScaleFont};

use crate::font::Fonts;

/// Normalized UV rectangle of a glyph inside the atlas, plus its raster
/// offsets (px_bounds min, relative to the baseline pen origin).
#[derive(Clone, Copy, Debug)]
pub struct Slot {
    pub u0: f32,
    pub v0: f32,
    pub u1: f32,
    pub v1: f32,
    pub ox: f32,
    pub oy: f32,
}

/// Glyph cache key: font index + glyph id + quantized px size.
fn key(font_idx: u16, glyph_id: u16, px: f32) -> u64 {
    let px_bits = (px * 4.0).clamp(0.0, 65535.0) as u64;
    ((font_idx as u64) << 48) | ((glyph_id as u64) << 32) | px_bits
}

/// Retained raster: (w, h, offset_x, offset_y, rgba8).
type Raster = (u32, u32, f32, f32, Vec<u8>);

pub struct Atlas {
    pub size: u32,
    data: Vec<u8>,
    cursor_x: u32,
    cursor_y: u32,
    row_h: u32,
    cache: std::collections::HashMap<u64, Slot>,
    pixels: std::collections::HashMap<u64, Raster>,
    /// True when new glyphs were packed since the last texture upload.
    pub changed: bool,
    /// UV of the 1x1 white pixel (used for solid rects).
    pub white: Slot,
}

impl Atlas {
    pub fn new() -> Self {
        let size = 1024;
        let mut atlas = Self {
            size,
            data: vec![0u8; (size * size * 4) as usize],
            cursor_x: 1,
            cursor_y: 1,
            row_h: 0,
            cache: std::collections::HashMap::new(),
            pixels: std::collections::HashMap::new(),
            changed: false,
            white: Slot { u0: 0.0, v0: 0.0, u1: 0.0, v1: 0.0, ox: 0.0, oy: 0.0 },
        };
        let s = size as f32;
        atlas.white = Slot { u0: 0.5 / s, v0: 0.5 / s, u1: 1.5 / s, v1: 1.5 / s, ox: 0.0, oy: 0.0 };
        atlas
    }

    /// Get the atlas slot for `c`, rasterizing + packing on a miss.
    /// Returns None only for glyphs with no outline (space / control chars).
    pub fn get(&mut self, fonts: &mut Fonts, c: char) -> Option<Slot> {
        if c == ' ' {
            return None;
        }
        let font_idx = fonts.font_for(c);
        let glyph_id = fonts.font(font_idx).as_scaled(fonts.px).glyph_id(c).0 as u16;
        let k = key(font_idx, glyph_id, fonts.px);
        if let Some(slot) = self.cache.get(&k) {
            return Some(*slot);
        }
        let raster = fonts.rasterize(c)?;
        let (w, h, ox, oy) = (raster.w, raster.h, raster.offset_x, raster.offset_y);
        self.pixels.insert(k, (w, h, ox, oy, raster.rgba));
        self.pack(k)
    }

    /// Pack `pixels[key]` into the atlas, growing + repacking on overflow.
    fn pack(&mut self, k: u64) -> Option<Slot> {
        let (w, h, ox, oy, pixels) = self.pixels.get(&k)?;
        let (w, h, ox, oy) = (*w, *h, *ox, *oy);
        if w == 0 || h == 0 {
            return None;
        }
        if self.cursor_x + w > self.size {
            // Flush the current row, start a new one.
            self.cursor_x = 1;
            self.cursor_y += self.row_h;
            self.row_h = 0;
        }
        if self.cursor_y + h > self.size {
            // Full: grow the atlas and repack everything.
            if !self.grow() {
                return None;
            }
            return self.pack(k);
        }
        let s = self.size as f32;
        let (x, y) = (self.cursor_x, self.cursor_y);
        for yy in 0..h {
            let src = (yy as usize) * (w as usize) * 4;
            let dst = ((y + yy) as usize) * (self.size as usize) * 4 + (x as usize) * 4;
            self.data[dst..dst + (w as usize) * 4].copy_from_slice(&pixels[src..src + (w as usize) * 4]);
        }
        self.cursor_x += w + 1; // 1px gutter avoids bleeding between glyphs
        self.row_h = self.row_h.max(h);
        let slot = Slot {
            u0: (x as f32) / s,
            v0: (y as f32) / s,
            u1: ((x + w) as f32) / s,
            v1: ((y + h) as f32) / s,
            ox,
            oy,
        };
        self.cache.insert(k, slot);
        self.changed = true;
        Some(slot)
    }

    /// Double the atlas size and re-pack every cached glyph from retained pixels.
    fn grow(&mut self) -> bool {
        if self.size >= 4096 {
            return false;
        }
        self.size *= 2;
        self.data = vec![0u8; (self.size * self.size * 4) as usize];
        self.cursor_x = 1;
        self.cursor_y = 1;
        self.row_h = 0;
        self.cache.clear();
        let keys: Vec<u64> = self.pixels.keys().copied().collect();
        for k in keys {
            if self.pack(k).is_none() {
                return false;
            }
        }
        let s = self.size as f32;
        self.white = Slot { u0: 0.5 / s, v0: 0.5 / s, u1: 1.5 / s, v1: 1.5 / s, ox: 0.0, oy: 0.0 };
        true
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Drop all cached glyphs (called on font/size reload).
    pub fn reset(&mut self) {
        self.data = vec![0u8; (self.size * self.size * 4) as usize];
        self.cursor_x = 1;
        self.cursor_y = 1;
        self.row_h = 0;
        self.cache.clear();
        self.pixels.clear();
        self.changed = true;
    }
}

impl Default for Atlas {
    fn default() -> Self {
        Self::new()
    }
}
