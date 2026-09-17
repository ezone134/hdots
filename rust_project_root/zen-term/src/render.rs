//! Renderer: turns `RenderableContent` (alacritty_terminal) into a batch of
//! GPU quads. The text logic lives here, shared by every backend.
//!
//! `GpuBackend` is the only backend-specific surface. `render_gl` implements
//! it with raw EGL + OpenGL; a future `render_vk` implements it with ash +
//! Vulkan behind the exact same interface.

use alacritty_terminal::grid::GridCell;
use alacritty_terminal::index::{Column, Line, Point};
use alacritty_terminal::term::cell::Flags;
use alacritty_terminal::term::color::Colors;
use alacritty_terminal::term::RenderableContent;
use alacritty_terminal::term::TermMode;
use alacritty_terminal::vte::ansi::{Color, CursorShape, NamedColor, Rgb};

use crate::atlas::{Atlas, Slot};
use crate::config::Config;
use crate::font::Fonts;

/// The backend draw surface. All colors are premultiplied RGBA.
pub trait GpuBackend {
    /// Resize the drawable (wl_egl_window + viewport), buffer pixels.
    fn resize(&mut self, w: i32, h: i32);
    /// Start a frame: reset batches. `bg` is the premultiplied clear color.
    fn begin_frame(&mut self, bg: [f32; 4]);
    /// Solid premultiplied quad (backgrounds, cursor, underlines, selection).
    fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, color: [f32; 4]);
    /// Textured glyph quad sampling `uv` from the atlas.
    fn glyph(&mut self, uv: Slot, x: f32, y: f32, w: f32, h: f32, color: [f32; 4]);
    /// Upload the whole atlas texture (RGBA8, `size`×`size`).
    fn upload_atlas(&mut self, size: u32, data: &[u8]);
    /// Present the frame (swap buffers).
    fn present(&mut self);
}

pub struct Renderer {
    pub backend: Box<dyn GpuBackend>,
    pub fonts: Fonts,
    pub atlas: Atlas,
    pub cell_w: f32,
    pub cell_h: f32,
    ascent: f32,
    /// Effective padding in buffer pixels (logical × scale).
    pub padding: f32,
    pub opacity: f32,
    /// Drawable size in buffer pixels.
    pub size: (i32, i32),
    pub cols: usize,
    pub rows: usize,
    // resolved theme colors (rebuilt on config reload)
    bg: Rgb,
    fg: Rgb,
    cursor: Rgb,
    cursor_text: Rgb,
    sel_bg: Rgb,
    sel_fg: Rgb,
    // font identity, to detect changes on reload
    font_family: String,
    /// Logical font size (px) from config; actual raster px = font_px × scale.
    font_px: f32,
    logical_padding: f32,
    scale: f32,
}

impl Renderer {
    pub fn new(backend: Box<dyn GpuBackend>, cfg: &Config) -> Self {
        let mut r = Self {
            backend,
            fonts: Fonts::load(&cfg.font_family, cfg.font_size.max(4.0)),
            atlas: Atlas::new(),
            cell_w: 0.0,
            cell_h: 0.0,
            ascent: 0.0,
            padding: cfg.padding,
            opacity: cfg.background_opacity.clamp(0.0, 1.0),
            size: (0, 0),
            cols: 1,
            rows: 1,
            bg: crate::config::parse_rgb(&cfg.colors.background),
            fg: crate::config::parse_rgb(&cfg.colors.foreground),
            cursor: crate::config::parse_rgb(&cfg.colors.cursor),
            cursor_text: crate::config::parse_rgb(&cfg.colors.cursor),
            sel_bg: crate::config::parse_rgb(&cfg.colors.foreground),
            sel_fg: crate::config::parse_rgb(&cfg.colors.background),
            font_family: cfg.font_family.clone(),
            font_px: cfg.font_size.max(4.0),
            logical_padding: cfg.padding,
            scale: 1.0,
        };
        // Compute cell metrics from the font we already loaded (avoids a
        // second full system-font scan — the single biggest startup cost).
        let (cw, ch, ascent) = r.fonts.metrics();
        r.cell_w = cw;
        r.cell_h = ch;
        r.ascent = ascent;
        r.apply_config(cfg);
        r
    }

    /// Apply a (possibly reloaded) config. Returns true if the terminal grid
    /// size may have changed (font metrics changed), so the caller resizes.
    pub fn apply_config(&mut self, cfg: &Config) -> bool {
        let font_changed = self.font_family != cfg.font_family
            || (self.font_px - cfg.font_size.max(4.0)).abs() > f32::EPSILON;
        self.font_family = cfg.font_family.clone();
        self.font_px = cfg.font_size.max(4.0);
        self.logical_padding = cfg.padding;
        self.opacity = cfg.background_opacity.clamp(0.0, 1.0);
        self.rebuild_theme(cfg);
        if font_changed {
            self.reload_fonts();
        }
        self.padding = self.logical_padding * self.scale;
        self.upload_atlas();
        font_changed
    }

    /// HiDPI scale change: re-rasterize fonts at px×scale, recompute metrics.
    pub fn set_scale(&mut self, scale: f32) {
        // Also reload when metrics are uninitialized (cell_w == 0), so the
        // very first call — even with scale 1.0 — computes the cell size.
        if (self.scale - scale).abs() < 0.001 && self.cell_w > 0.0 {
            return;
        }
        self.scale = scale;
        self.padding = self.logical_padding * scale;
        self.reload_fonts();
    }

    fn reload_fonts(&mut self) {
        self.fonts = Fonts::load(&self.font_family, self.font_px * self.scale);
        let (cell_w, cell_h, ascent) = self.fonts.metrics();
        self.cell_w = cell_w;
        self.cell_h = cell_h;
        self.ascent = ascent;
        self.atlas.reset();
    }

    fn rebuild_theme(&mut self, cfg: &Config) {
        self.bg = crate::config::parse_rgb(&cfg.colors.background);
        self.fg = crate::config::parse_rgb(&cfg.colors.foreground);
        self.cursor = crate::config::parse_rgb(&cfg.colors.cursor);
        let (cr, cg, cb) = (self.cursor.r, self.cursor.g, self.cursor.b);
        // Auto cursor text: luminance-based inversion for guaranteed contrast.
        let lum = 0.299 * cr as f32 + 0.587 * cg as f32 + 0.114 * cb as f32;
        self.cursor_text = if cfg.colors.cursor_text.trim().is_empty() {
            if lum > 128.0 { Rgb { r: 0, g: 0, b: 0 } } else { Rgb { r: 255, g: 255, b: 255 } }
        } else {
            crate::config::parse_rgb(&cfg.colors.cursor_text)
        };
        self.sel_bg = if cfg.colors.selection_bg.trim().is_empty() {
            self.fg
        } else {
            crate::config::parse_rgb(&cfg.colors.selection_bg)
        };
        self.sel_fg = if cfg.colors.selection_fg.trim().is_empty() {
            self.bg
        } else {
            crate::config::parse_rgb(&cfg.colors.selection_fg)
        };
    }

    pub fn upload_atlas(&mut self) {
        self.backend.upload_atlas(self.atlas.size, self.atlas.data());
        self.atlas.changed = false;
    }

    /// Grid dimensions (cols, rows) that fit `w`×`h` buffer pixels.
    pub fn grid_size(&mut self, w: i32, h: i32) -> (usize, usize) {
        let pad = self.padding;
        // Defensive: a 0/NaN cell metric would make the division +inf and the
        // `as usize` cast saturate to usize::MAX, overflowing the term grid.
        let cw = if self.cell_w.is_finite() && self.cell_w > 0.0 { self.cell_w } else { 1.0 };
        let ch = if self.cell_h.is_finite() && self.cell_h > 0.0 { self.cell_h } else { 1.0 };
        let cols = (((w as f32 - 2.0 * pad) / cw).floor().max(1.0)) as usize;
        let rows = (((h as f32 - 2.0 * pad) / ch).floor().max(1.0)) as usize;
        self.cols = cols;
        self.rows = rows;
        (cols, rows)
    }

    /// Draw one frame of terminal content.
    pub fn draw(&mut self, rc: RenderableContent<'_>, offset: usize) {
        let (w, h) = self.size;
        let pad = self.padding;

        // Background: one full-window quad with the configured opacity.
        self.backend.begin_frame([0.0, 0.0, 0.0, 0.0]);
        self.backend.rect(0.0, 0.0, w as f32, h as f32, premul_rgb(self.bg, self.opacity));

        let cursor_shape = rc.cursor.shape;
        let show_cursor = rc.mode.contains(TermMode::SHOW_CURSOR)
            && !matches!(cursor_shape, CursorShape::Hidden);

        // Selection highlight (behind the text).
        if let Some(sel) = &rc.selection {
            for line in sel.start.line.0..=sel.end.line.0 {
                let Some(viewport) = point_to_viewport(offset, Point::new(Line(line), Column(0))) else {
                    continue;
                };
                let row = viewport.line;
                if row >= self.rows {
                    continue;
                }
                let c0 = if line == sel.start.line.0 { sel.start.column.0 } else { 0 };
                let c1 = if line == sel.end.line.0 { sel.end.column.0 } else { self.cols - 1 };
                let x = pad + c0 as f32 * self.cell_w;
                let y = pad + row as f32 * self.cell_h;
                let width = (c1 as i64 - c0 as i64 + 1).max(0) as f32 * self.cell_w;
                if width > 0.0 {
                    self.backend.rect(x, y, width, self.cell_h, premul_rgb(self.sel_bg, 1.0));
                }
            }
        }

        for idx in rc.display_iter {
            let point = idx.point;
            let cell = idx.cell;
            let flags = cell.flags();

            // Grid line → viewport row.
            let row = (point.line.0 as i64 + offset as i64) as usize;
            if row >= self.rows {
                continue;
            }
            let col = point.column.0;
            let x = pad + col as f32 * self.cell_w;
            let y = pad + row as f32 * self.cell_h;

            let mut fg = resolve(cell.fg, rc.colors);
            let mut bg = resolve(cell.bg, rc.colors);

            if flags.contains(Flags::INVERSE) {
                std::mem::swap(&mut fg, &mut bg);
            }

            let selected = rc.selection.as_ref().map_or(false, |s: &alacritty_terminal::selection::SelectionRange| {
                s.contains(point)
            });
            if selected {
                fg = self.sel_fg;
                bg = self.sel_bg;
            }

            let is_cursor = show_cursor && rc.cursor.point == point;
            let cursor_block = is_cursor
                && matches!(cursor_shape, CursorShape::Block | CursorShape::HollowBlock);
            let mut text_color = fg;
            if cursor_block {
                bg = self.cursor;
                text_color = self.cursor_text;
            }

            // Cell background (skip when it matches the terminal background).
            if bg != self.bg || cursor_block || selected {
                self.backend.rect(x, y, self.cell_w, self.cell_h, premul_rgb(bg, 1.0));
            }

            // Glyph.
            let c = cell.c;
            if c != ' ' && c != '\0'
                && !flags.contains(Flags::WIDE_CHAR_SPACER)
                && !flags.contains(Flags::HIDDEN)
            {
                if let Some(slot) = self.atlas.get(&mut self.fonts, c) {
                    let gx = x + slot.ox;
                    let gy = y + self.ascent + slot.oy;
                    let gw = (slot.u1 - slot.u0) * self.atlas.size as f32;
                    let gh = (slot.v1 - slot.v0) * self.atlas.size as f32;
                    let color = premul_rgb(text_color, 1.0);
                    // Bold: double-draw, offset one pixel (classic embolden).
                    if flags.contains(Flags::BOLD) && !flags.contains(Flags::DIM) {
                        self.backend.glyph(slot, gx + 1.0, gy, gw, gh, color);
                    }
                    self.backend.glyph(slot, gx, gy, gw, gh, color);
                }
            }

            // Underlines (a single line suffices for the fancy variants in v1).
            if flags.intersects(Flags::ALL_UNDERLINES) {
                let thickness = (self.cell_h * 0.12).max(1.0);
                let underline_color =
                    cell.underline_color().map_or(text_color, |c| resolve(c, rc.colors));
                self.backend.rect(
                    x,
                    y + self.cell_h - thickness,
                    self.cell_w,
                    thickness,
                    premul_rgb(underline_color, 1.0),
                );
            }
            if flags.contains(Flags::STRIKEOUT) {
                self.backend.rect(
                    x,
                    y + self.cell_h * 0.55,
                    self.cell_w,
                    (self.cell_h * 0.06).max(1.0),
                    premul_rgb(text_color, 1.0),
                );
            }

            // Beam / underline cursors are thin bars, drawn after the glyph.
            if is_cursor {
                match cursor_shape {
                    CursorShape::Beam => {
                        self.backend.rect(x, y, 2.0, self.cell_h, premul_rgb(self.cursor, 1.0));
                    }
                    CursorShape::Underline => {
                        self.backend.rect(x, y + self.cell_h - 2.0, self.cell_w, 2.0, premul_rgb(self.cursor, 1.0));
                    }
                    _ => {}
                }
            }
        }

        // Upload glyphs packed during this frame (once).
        if self.atlas.changed {
            self.upload_atlas();
        }

        self.backend.present();
    }
}

/// Convert a terminal point to a viewport-relative point (row, col).
fn point_to_viewport(offset: usize, point: Point) -> Option<Point<usize>> {
    alacritty_terminal::term::point_to_viewport(offset, point)
}

/// Resolve a terminal color to concrete RGB using the theme table.
pub fn resolve(color: Color, colors: &Colors) -> Rgb {
    match color {
        Color::Spec(rgb) => rgb,
        Color::Indexed(i) => colors[i as usize].unwrap_or(Rgb { r: 0, g: 0, b: 0 }),
        Color::Named(n) => colors[n].unwrap_or_else(|| named_default(n)),
    }
}

fn named_default(n: NamedColor) -> Rgb {
    use NamedColor::*;
    match n {
        Black | DimBlack | BrightBlack => Rgb { r: 0, g: 0, b: 0 },
        Red | DimRed | BrightRed => Rgb { r: 205, g: 0, b: 0 },
        Green | DimGreen | BrightGreen => Rgb { r: 0, g: 205, b: 0 },
        Yellow | DimYellow | BrightYellow => Rgb { r: 205, g: 205, b: 0 },
        Blue | DimBlue | BrightBlue => Rgb { r: 0, g: 0, b: 238 },
        Magenta | DimMagenta | BrightMagenta => Rgb { r: 205, g: 0, b: 205 },
        Cyan | DimCyan | BrightCyan => Rgb { r: 0, g: 205, b: 205 },
        White | DimWhite | BrightWhite => Rgb { r: 229, g: 229, b: 229 },
        _ => Rgb { r: 255, g: 255, b: 255 },
    }
}

/// Premultiplied RGBA from an RGB + alpha.
fn premul_rgb(rgb: Rgb, a: f32) -> [f32; 4] {
    [
        rgb.r as f32 / 255.0 * a,
        rgb.g as f32 / 255.0 * a,
        rgb.b as f32 / 255.0 * a,
        a,
    ]
}
