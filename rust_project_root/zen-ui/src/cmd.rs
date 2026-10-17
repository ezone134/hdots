//! The renderer command stream (`Cmd`) — ONE vocabulary every surface,
//! card, widget tree and renderer speaks. Shared so \`zen-shell\` and the
//! other projects parse/rasterize the same bytes.

/// Derive Debug: widget-core parity tests assert on drawn `Cmd`s (and it is
/// free diagnostics for IPC traces). PartialEq: cards.rs parity tests diff
/// the Rust widget stream against the Lua module stream command-for-command.
#[derive(Clone, Debug, PartialEq)]
pub enum Cmd {
    Rect {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        r: f32,
        color: u32,
    },
    /// Rounded rect with per-corner radii — negative = concave (Noctalia-style screen-edge carve).
    RectConcave {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        r_tl: f32,
        r_tr: f32,
        r_br: f32,
        r_bl: f32,
        color: u32,
    },
    /// Stroked (outline-only) rounded rect — `width` is the stroke thickness
    /// centered on the edge. Vector icons (battery body, drag outlines).
    Outline {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        r: f32,
        width: f32,
        color: u32,
    },
    /// Thick line segment (capsule) from (x0,y0) to (x1,y1), `w` px thick —
    /// heartbeat-style graph lines.
    Line {
        x0: f32,
        y0: f32,
        x1: f32,
        y1: f32,
        w: f32,
        color: u32,
    },
    Text {
        x: f32,
        y: f32,
        right: bool,
        center: bool,
        text: String,
        size: f32,
        color: u32,
        icon: bool,
        f: crate::text::Ff,
        w: crate::text::Fw,
    },
    /// Rasterized image (app icon, tray pixmap) fit inside a `w`×`h` box
    /// (aspect-preserving, downscale-only), centered in that box.
    /// `key` is an `ImageStore` key (`icon:<name>` / `file:<path>` / tray id).
    Image {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        key: String,
    },
    /// World-map texture: ONE cached band raster drawn as a UV-windowed quad
    /// fitted to the body rect. Pan/zoom only moves the UV window (4 floats)
    /// — the heavy raster is re-baked only when the view leaves the band.
    /// `colors` = `[raised, land, rings, hairline, dots, region]`.
    Map {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        lon0: f32,
        lat0: f32,
        lon1: f32,
        lat1: f32,
        zoom: f32,
        style: u8,
        colors: [u32; 6],
        region: bool,
        version: u64,
    },
    /// Restrict all subsequent drawing to a pixel rect (scene coords). Clips
    /// overflow inside a region — per-zone banner strip scrolling clips each
    /// L/C/R zone so scrolled-out chips never bleed into their neighbours.
    Scissor { x: f32, y: f32, w: f32, h: f32 },
    /// End the current scissor; drawing is unrestricted again.
    ScissorEnd,
}
