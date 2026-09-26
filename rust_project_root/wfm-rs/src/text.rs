use ab_glyph::{Font, FontArc, FontRef, GlyphId, PxScale, ScaleFont};
use fontdb::{Database, Family, Query, Source, Style, Weight};

/// Text rasterizer backed by fontdb + ab_glyph (replaces freetype/fontconfig).
pub struct Text {
    font: FontArc,
    bold: Option<FontArc>,
    pub font_size: f32,
}

impl Text {
    /// Load a text engine, preferring the family named in the config
    /// (`font_name = …`); falls back to the usual defaults when the named
    /// family is missing or empty.
    pub fn new_family(font_size: f32, family: Option<&str>) -> Option<Self> {
        let mut db = Database::new();
        db.load_system_fonts();

        // Prefer known-good outline families; avoid symbol/nerd-only faces.
        let mut preferred: Vec<String> = Vec::new();
        if let Some(f) = family {
            if !f.trim().is_empty() {
                preferred.push(f.trim().to_string());
            }
        }
        for n in [
            "Inter",
            "Noto Sans",
            "DejaVu Sans",
            "Liberation Sans",
            "FreeSans",
            "Ubuntu",
            "Source Sans Pro",
            "Cantarell",
            "Roboto",
        ] {
            preferred.push(n.to_string());
        }
        let mut families: Vec<Family<'_>> = preferred.iter().map(|n| Family::Name(n)).collect();
        families.push(Family::SansSerif);

        let id = db
            .query(&Query {
                families: &families,
                weight: Weight::NORMAL,
                style: Style::Normal,
                ..Default::default()
            })
            .or_else(|| {
                db.query(&Query {
                    families: &[Family::SansSerif],
                    ..Default::default()
                })
            })?;

        let font = read_face(&db, id)?;

        // Sanity: must be able to outline a basic letter.
        let scale = PxScale::from(font_size.max(8.0));
        let gid = font.as_scaled(scale).glyph_id('A');
        if gid.0 == 0 {
            // glyph 0 is .notdef — try next fallback path
            return Self::load_first_working(&db, font_size);
        }
        let glyph = gid.with_scale(scale);
        if font.outline_glyph(glyph).is_none() {
            return Self::load_first_working(&db, font_size);
        }

        // Best-effort bold companion for section headers / emphasis. Absent on
        // systems without a bold face → fake-bold fallback in draw_bold_clip.
        let bold = db
            .query(&Query {
                families: &families,
                weight: Weight::BOLD,
                style: Style::Normal,
                ..Default::default()
            })
            .and_then(|bid| read_face(&db, bid));

        Some(Text {
            font,
            bold,
            font_size: font_size.max(8.0),
        })
    }

    fn load_first_working(db: &Database, font_size: f32) -> Option<Self> {
        let scale = PxScale::from(font_size.max(8.0));
        for face in db.faces() {
            if face.style != Style::Normal {
                continue;
            }
            let Some((source, index)) = db.face_source(face.id) else {
                continue;
            };
            let bytes: Vec<u8> = match source {
                Source::File(path) => match std::fs::read(path.as_path()) {
                    Ok(b) => b,
                    Err(_) => continue,
                },
                Source::Binary(data) => data.as_ref().as_ref().to_vec(),
                Source::SharedFile(_, data) => data.as_ref().as_ref().to_vec(),
            };
            let Some(font) = (if index == 0 {
                FontArc::try_from_vec(bytes).ok()
            } else {
                load_face_index(bytes, index)
            }) else {
                continue;
            };
            let gid = font.as_scaled(scale).glyph_id('A');
            let glyph = gid.with_scale(scale);
            if font.outline_glyph(glyph).is_some() {
                return Some(Text {
                    font,
                    bold: None,
                    font_size: font_size.max(8.0),
                });
            }
        }
        None
    }

    pub fn scale(&self) -> PxScale {
        PxScale::from(self.font_size)
    }

    fn unit_scale(&self) -> f32 {
        let upem = self.font.units_per_em().unwrap_or(1000.0);
        self.font_size / upem
    }

    pub fn ascent(&self) -> f32 {
        self.font.ascent_unscaled() * self.unit_scale()
    }

    pub fn ascent_px(&self) -> i32 {
        self.ascent().ceil() as i32
    }

    pub fn line_h(&self) -> i32 {
        let u = self.unit_scale();
        let h = (self.font.ascent_unscaled() - self.font.descent_unscaled()
            + self.font.line_gap_unscaled())
            * u;
        h.ceil() as i32 + 2
    }

    pub fn width(&self, text: &str) -> f32 {
        let scaled = self.font.as_scaled(self.scale());
        let mut w = 0.0f32;
        let mut prev: Option<GlyphId> = None;
        for c in text.chars() {
            let gid = scaled.glyph_id(c);
            if let Some(p) = prev {
                w += scaled.kern(p, gid);
            }
            w += scaled.h_advance(gid);
            prev = Some(gid);
        }
        w
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        &self,
        buf: &mut [u32],
        w: u32,
        h: u32,
        x: i32,
        y: i32,
        text: &str,
        color: u32,
    ) -> f32 {
        self.draw_clip(buf, w, h, x, y, text, color, w as i32)
    }

    /// Draw at baseline (x,y). `color` is 0xRRGGBB (alpha forced opaque).
    #[allow(clippy::too_many_arguments)]
    pub fn draw_clip(
        &self,
        buf: &mut [u32],
        w: u32,
        h: u32,
        x: i32,
        y: i32,
        text: &str,
        color: u32,
        clip_right: i32,
    ) -> f32 {
        self.raster(&self.font, buf, w, h, x, y, text, color, clip_right)
    }

    /// Draw with the bold face (falls back to a 1px-shifted double-draw when no
    /// bold face exists on the system).
    #[allow(clippy::too_many_arguments)]
    pub fn draw_bold(
        &self,
        buf: &mut [u32],
        w: u32,
        h: u32,
        x: i32,
        y: i32,
        text: &str,
        color: u32,
    ) -> f32 {
        self.draw_bold_clip(buf, w, h, x, y, text, color, w as i32)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw_bold_clip(
        &self,
        buf: &mut [u32],
        w: u32,
        h: u32,
        x: i32,
        y: i32,
        text: &str,
        color: u32,
        clip_right: i32,
    ) -> f32 {
        match &self.bold {
            Some(f) => self.raster(f, buf, w, h, x, y, text, color, clip_right),
            None => {
                let adv = self.raster(&self.font, buf, w, h, x, y, text, color, clip_right);
                self.raster(&self.font, buf, w, h, x + 1, y, text, color, clip_right);
                adv
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn raster(
        &self,
        f: &FontArc,
        buf: &mut [u32],
        w: u32,
        h: u32,
        x: i32,
        y: i32,
        text: &str,
        color: u32,
        clip_right: i32,
    ) -> f32 {
        let color = color & 0x00ff_ffff;
        // Guard inverted / empty clip windows (sidebar+preview can shrink columns).
        let clip_right = if clip_right <= x { w as i32 } else { clip_right };
        let scaled = f.as_scaled(self.scale());
        let mut pen = x as f32;
        let mut prev: Option<GlyphId> = None;
        for c in text.chars() {
            let gid = scaled.glyph_id(c);
            if let Some(p) = prev {
                pen += scaled.kern(p, gid);
            }
            let glyph = gid.with_scale_and_position(self.scale(), ab_glyph::point(pen, y as f32));
            if let Some(og) = f.outline_glyph(glyph) {
                let b = og.px_bounds();
                let x0 = b.min.x.floor() as i32;
                let y0 = b.min.y.floor() as i32;
                og.draw(|gx, gy, cov| {
                    if cov <= 0.0 {
                        return;
                    }
                    let ax = x0 + gx as i32;
                    let ay = y0 + gy as i32;
                    if ax >= clip_right || ay < 0 || ay >= h as i32 {
                        return;
                    }
                    if ax < 0 || ax >= w as i32 {
                        return;
                    }
                    blend(buf, w, ax as u32, ay as u32, color, cov);
                });
            }
            pen += scaled.h_advance(gid);
            prev = Some(gid);
        }
        pen - x as f32
    }
}

fn read_face(db: &Database, id: fontdb::ID) -> Option<FontArc> {
    let face = db.face(id)?;
    let index = face.index;
    let (source, _) = db.face_source(id)?;
    let bytes: Vec<u8> = match source {
        Source::File(path) => std::fs::read(path.as_path()).ok()?,
        Source::Binary(data) => data.as_ref().as_ref().to_vec(),
        Source::SharedFile(_, data) => data.as_ref().as_ref().to_vec(),
    };
    if index == 0 {
        FontArc::try_from_vec(bytes).ok()
    } else {
        load_face_index(bytes, index)
    }
}

fn load_face_index(bytes: Vec<u8>, index: u32) -> Option<FontArc> {
    // FontRef borrows; leak the buffer for process lifetime (fonts load once).
    let static_bytes: &'static [u8] = Box::leak(bytes.into_boxed_slice());
    let font = FontRef::try_from_slice_and_index(static_bytes, index).ok()?;
    Some(FontArc::from(font))
}

/// Premultiplied-friendly over: coverage × RGB onto opaque ARGB dst.
#[inline]
fn blend(buf: &mut [u32], w: u32, x: u32, y: u32, color: u32, cov: f32) {
    let a = (cov.clamp(0.0, 1.0) * 255.0).round() as u32;
    if a == 0 {
        return;
    }
    let idx = (y * w + x) as usize;
    if idx >= buf.len() {
        return;
    }
    let cr = (color >> 16) & 0xff;
    let cg = (color >> 8) & 0xff;
    let cb = color & 0xff;
    if a >= 255 {
        // Always write opaque ARGB — missing alpha made solid glyphs invisible.
        buf[idx] = 0xff00_0000 | (cr << 16) | (cg << 8) | cb;
        return;
    }
    let dst = buf[idx];
    let inv = 255 - a;
    let r = (cr * a + ((dst >> 16) & 0xff) * inv) / 255;
    let g = (cg * a + ((dst >> 8) & 0xff) * inv) / 255;
    let b = (cb * a + (dst & 0xff) * inv) / 255;
    buf[idx] = 0xff00_0000 | (r << 16) | (g << 8) | b;
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn draw_writes_opaque_pixels() {
        let Some(text) = Text::new_family(16.0, None) else {
            panic!("Text::new_family failed — no usable font");
        };
        eprintln!(
            "font metrics: size={} ascent={} line_h={} width(Hello)={}",
            text.font_size,
            text.ascent_px(),
            text.line_h(),
            text.width("Hello")
        );
        let w = 200u32;
        let h = 64u32;
        let mut buf = vec![0xff1e1e2eu32; (w * h) as usize];
        let adv = text.draw(&mut buf, w, h, 8, 40, "Hello WFM", 0x00cdd6f4);
        eprintln!("advance={adv}");
        let lit = buf.iter().filter(|&&p| p != 0xff1e1e2e && (p >> 24) == 0xff).count();
        eprintln!("lit_pixels={lit}");
        // write PPM for visual check
        let mut ppm = String::from("P3\n200 64\n255\n");
        for y in 0..h {
            for x in 0..w {
                let p = buf[(y * w + x) as usize];
                ppm.push_str(&format!("{} {} {} ", (p >> 16) & 0xff, (p >> 8) & 0xff, p & 0xff));
            }
            ppm.push('\n');
        }
        let _ = std::fs::write("/tmp/wfm_text_probe.ppm", ppm);
        assert!(lit > 10, "expected visible glyphs, got {lit} lit px");
        assert!(text.width("Hello") > 10.0);
        assert!(text.ascent_px() > 4);
        assert!(text.line_h() > 8);
    }

    #[test]
    fn draw_bold_writes_pixels() {
        let Some(text) = Text::new_family(16.0, None) else {
            panic!("Text::new_family failed — no usable font");
        };
        let w = 200u32;
        let h = 64u32;
        let mut buf = vec![0xff1e1e2eu32; (w * h) as usize];
        let adv = text.draw_bold(&mut buf, w, h, 8, 40, "Places", 0x00a6adc8);
        assert!(adv > 10.0, "bold advance too small: {adv}");
        let lit = buf.iter().filter(|&&p| p != 0xff1e1e2e && (p >> 24) == 0xff).count();
        assert!(lit > 10, "expected visible bold glyphs, got {lit} lit px");
    }
}
