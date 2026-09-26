//! VECTOR WORLD MAP — an embedded 110m Earth dataset (`world_map.bin`, the
//! "original world map") is ALWAYS available, so the card renders from the
//! first frame. Downloaded chunks from `ezone134/zen-shell-map` (see
//! `crate::shell::mapdata`) replace it at runtime (city labels, full checksum
//! verification, per-country detail). All data is public domain.
//!
//! Projection is equirectangular over a **view window** (default the whole
//! world: lon -180..180, lat -85..85). `Win` describes the windowed source
//! range; `proj`/`unproj` map between window coordinates and the fit rect.
//! Zooming shrinks the window around a center → the same polygon pipeline
//! renders any region of the world, still purely with rect/line/text cmds.
//!
//! The same polygons feed every style:
//!   - filled  → merged scanline rects (real / filled panes)
//!   - dotted  → dot grid where land (dotted pane)
//!   - stroked → country border polylines (stroked / real panes)
//!
//! Clicking the map resolves the pointer to latitude/longitude, then finds the
//! nearest entry in `/usr/share/zoneinfo/zone.tab` so the card can show that
//! area's current local time.

use std::sync::{Arc, Mutex, OnceLock};

/// The embedded "original" world (110m, i16 lon/lat in 1/100°) — the caller's
/// fallback so the map is visible the moment the card exists, even with the
/// offline-downloads toggle off. Same format as the downloaded `world.bin`.
const EMBEDDED_WORLD: &[u8] = include_bytes!("world_map.bin");

/// Grid resolution for the landmask (columns × rows, equirectangular).
const GW: usize = 1440;
const GH: usize = 720;

// ---------------- runtime world + city data ----------------

/// Rows of normalized-land x-runs (from the current world ring set).
type Runs = Vec<Vec<(f32, f32)>>;

/// A parsed, ready-to-draw world dataset (rings + derived scanline mask).
pub struct World {
    pub rings: Vec<Vec<(f32, f32)>>,
    runs: Runs,
}

/// One city label.
#[derive(Clone)]
pub struct City {
    pub lon: f32,
    pub lat: f32,
    pub name: String,
}

/// Process-global world dataset (set after a world chunk download/cache).
static WORLD_EQ: OnceLock<Mutex<Option<Arc<World>>>> = OnceLock::new();
/// Process-global city label list (set with the world dataset).
static CITIES: OnceLock<Mutex<Option<Arc<Vec<City>>>>> = OnceLock::new();
/// True while the store holds the EMBEDDED fallback (nothing downloaded yet).
static FALLBACK: OnceLock<Mutex<bool>> = OnceLock::new();

fn world_cell() -> &'static Mutex<Option<Arc<World>>> {
    WORLD_EQ.get_or_init(|| Mutex::new(None))
}

fn cities_cell() -> &'static Mutex<Option<Arc<Vec<City>>>> {
    CITIES.get_or_init(|| Mutex::new(None))
}

fn fallback_cell() -> &'static Mutex<bool> {
    FALLBACK.get_or_init(|| Mutex::new(false))
}

/// True while the current world is the EMBEDDED fallback (i.e. no downloaded
/// chunk has been applied yet). The card shows a small "download full map"
/// pill only in that state.
pub fn using_fallback() -> bool {
    fallback_cell().lock().map(|f| *f).unwrap_or(false)
}

/// Make sure the card ALWAYS has something to draw: if no world data is
/// present, parse the embedded 110m chunk and install it (marked as fallback).
pub fn ensure_fallback() {
    if world().is_some() {
        return;
    }
    if let Some(rings) = parse_ring_chunk(EMBEDDED_WORLD) {
        let w = Arc::new(World { runs: build_runs(&rings), rings });
        if let Ok(mut m) = world_cell().lock() {
            *m = Some(w.clone());
        }
        if let Ok(mut f) = fallback_cell().lock() {
            *f = true;
        }
    }
}

/// Swap in a freshly downloaded/parsed world ring set (rebuilds the mask).
/// A real swap always marks the store as non-fallback.
pub fn set_world(rings: Vec<Vec<(f32, f32)>>) -> Option<Arc<World>> {
    let w = Arc::new(World { runs: build_runs(&rings), rings });
    let c = world_cell();
    if let Ok(mut m) = c.lock() {
        *m = Some(w.clone());
    }
    if let Ok(mut f) = fallback_cell().lock() {
        *f = false;
    }
    Some(w)
}

/// Swap in the parsed city label list.
pub fn set_cities(cities: Vec<City>) {
    if let Ok(mut m) = cities_cell().lock() {
        *m = Some(Arc::new(cities));
    }
}

/// Current world dataset (None until the first world chunk is available).
pub fn world() -> Option<Arc<World>> {
    world_cell().lock().ok().and_then(|m| m.clone())
}

/// Current city label list (None until cities are available).
pub fn cities() -> Option<Arc<Vec<City>>> {
    cities_cell().lock().ok().and_then(|m| m.clone())
}

/// True once any world data is present (the card can render).
pub fn ready() -> bool {
    world().is_some()
}

/// Drop the runtime world + cities (tests + next boot).
pub fn clear() {
    if let Ok(mut m) = world_cell().lock() {
        *m = None;
    }
    if let Ok(mut m) = cities_cell().lock() {
        *m = None;
    }
    if let Ok(mut f) = fallback_cell().lock() {
        *f = false;
    }
}

/// Land test for a (lon, lat) in degrees, using the current world mask.
pub fn land_lonlat(w: &World, lon: f32, lat: f32) -> bool {
    let fx = (lon + 180.0) / 360.0;
    let fy = (85.0 - lat) / 170.0;
    if !(0.0..1.0).contains(&fx) {
        return false;
    }
    let y = ((fy.clamp(0.0, 0.9999)) * GH as f32) as usize;
    let runs = &w.runs[y];
    let x = fx.clamp(0.0, 0.9999);
    runs.iter().any(|(a, b)| x >= *a && x < *b)
}

fn build_runs(rings: &[Vec<(f32, f32)>]) -> Runs {
    let lat_st = 180.0 / GH as f32;
    let mut rows: Runs = Vec::with_capacity(GH);
    let mut cells = vec![false; GW];
    for y in 0..GH {
        cells.fill(false);
        let lat_mid = 90.0 - (y as f32 + 0.5) * lat_st;
        let mut xs: Vec<f32> = Vec::new();
        for ring in rings {
            for w in 0..ring.len() {
                let (lon0, lat0) = ring[w];
                let (lon1, lat1) = ring[(w + 1) % ring.len()];
                if (lon1 - lon0).abs() > 180.0 {
                    continue;
                }
                if (lat0 <= lat_mid && lat_mid < lat1) || (lat1 <= lat_mid && lat_mid < lat0) {
                    let t = (lat_mid - lat0) / (lat1 - lat0);
                    xs.push(lon0 + t * (lon1 - lon0));
                }
            }
        }
        xs.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let mut i = 0;
        while i + 1 < xs.len() {
            let mut a = ((xs[i] + 180.0) / 360.0 * GW as f32).round() as i64;
            let mut b = ((xs[i + 1] + 180.0) / 360.0 * GW as f32).round() as i64;
            if a > b {
                std::mem::swap(&mut a, &mut b);
            }
            a = a.clamp(0, GW as i64 - 1);
            b = b.clamp(0, GW as i64 - 1);
            for c in a as usize..=b as usize {
                cells[c] = true;
            }
            i += 2;
        }
        let mut runs: Vec<(f32, f32)> = Vec::new();
        let mut c = 0usize;
        while c < GW {
            if !cells[c] {
                c += 1;
                continue;
            }
            let s = c;
            while c < GW && cells[c] {
                c += 1;
            }
            runs.push((s as f32 / GW as f32, c as f32 / GW as f32));
        }
        rows.push(runs);
    }
    rows
}

// ---------------- binary parsers ----------------

/// Parse a ring blob (u32 ring_count, per ring u32 pt_count + i16 lon/lat*100).
pub fn parse_ring_chunk(bytes: &[u8]) -> Option<Vec<Vec<(f32, f32)>>> {
    let mut p = 0usize;
    let rd_u32 = |p: &mut usize| -> Option<u32> {
        let v = u32::from_le_bytes(bytes.get(*p..*p + 4)?.try_into().ok()?);
        *p += 4;
        Some(v)
    };
    let rd_i16 = |p: &mut usize| -> Option<i16> {
        let v = i16::from_le_bytes(bytes.get(*p..*p + 2)?.try_into().ok()?);
        *p += 2;
        Some(v)
    };
    let n = rd_u32(&mut p)?;
    let mut out = Vec::with_capacity(n as usize);
    for _ in 0..n {
        let cnt = rd_u32(&mut p)? as usize;
        let mut ring = Vec::with_capacity(cnt);
        for _ in 0..cnt {
            let lon = rd_i16(&mut p)? as f32 / 100.0;
            let lat = rd_i16(&mut p)? as f32 / 100.0;
            ring.push((lon, lat));
        }
        if ring.len() >= 3 {
            out.push(ring);
        }
    }
    Some(out)
}

/// Parse a cities blob (u32 count; per entry i16 lon*100, i16 lat*100,
/// u16 name_len + utf-8 name).
pub fn parse_cities_chunk(bytes: &[u8]) -> Option<Vec<City>> {
    let mut p = 0usize;
    let rd = |p: &mut usize, n: usize| -> Option<&[u8]> {
        let s = bytes.get(*p..*p + n)?;
        *p += n;
        Some(s)
    };
    let n = u32::from_le_bytes(rd(&mut p, 4)?.try_into().ok()?);
    let mut out = Vec::with_capacity(n as usize);
    for _ in 0..n {
        let c = rd(&mut p, 6)?;
        let lon = i16::from_le_bytes(c[0..2].try_into().ok()?) as f32 / 100.0;
        let lat = i16::from_le_bytes(c[2..4].try_into().ok()?) as f32 / 100.0;
        let nl = u16::from_le_bytes(c[4..6].try_into().ok()?) as usize;
        let nb = rd(&mut p, nl)?;
        let name = String::from_utf8_lossy(nb).into_owned();
        out.push(City { lon, lat, name });
    }
    Some(out)
}

// ---------------- view window + projection ----------------

/// Source window in degrees. `lon0 < lon1`, `lat0 < lat1` (lat0 = south edge).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Win {
    pub lon0: f32,
    pub lat0: f32,
    pub lon1: f32,
    pub lat1: f32,
}

impl Win {
    pub fn lon_span(&self) -> f32 {
        self.lon1 - self.lon0
    }
    pub fn lat_span(&self) -> f32 {
        self.lat1 - self.lat0
    }
    pub fn contains(&self, lon: f32, lat: f32) -> bool {
        lon >= self.lon0 && lon <= self.lon1 && lat >= self.lat0 && lat <= self.lat1
    }
}

/// Build a view window for a center + zoom (1.0 = whole world). Clamped to the
/// world bounds so east/west never leak past ±180 and lat stays within ±85.
pub fn win_for(center_lon: f32, center_lat: f32, zoom: f32) -> Win {
    let zoom = zoom.clamp(1.0, 64.0);
    let lon_span = (360.0 / zoom).min(360.0);
    let lat_span = (170.0 / zoom).min(170.0);
    let mut lon0 = center_lon - lon_span / 2.0;
    let mut lat0 = center_lat - lat_span / 2.0;
    if lon_span < 360.0 {
        lon0 = lon0.clamp(-180.0, 180.0 - lon_span);
    } else {
        lon0 = -180.0;
    }
    if lat_span < 170.0 {
        lat0 = lat0.clamp(-85.0, 85.0 - lat_span);
    } else {
        lat0 = -85.0;
    }
    Win { lon0, lat0, lon1: lon0 + lon_span, lat1: lat0 + lat_span }
}

/// lon/lat → px inside a windowed map rect.
pub fn proj(mx: f32, my: f32, mw: f32, mh: f32, win: &Win, lon: f32, lat: f32) -> (f32, f32) {
    let fx = (lon - win.lon0) / win.lon_span();
    let fy = (win.lat1 - lat) / win.lat_span();
    (mx + fx * mw, my + fy * mh)
}

/// px → lon/lat inside a windowed map rect.
pub fn unproj(mx: f32, my: f32, mw: f32, mh: f32, win: &Win, px: f32, py: f32) -> (f32, f32) {
    let fx = ((px - mx) / mw).clamp(0.0, 1.0);
    let fy = ((py - my) / mh).clamp(0.0, 1.0);
    (win.lon0 + fx * win.lon_span(), win.lat1 - fy * win.lat_span())
}

// ---------------- raster band (bake-to-texture) ----------------

/// Rasterize the map window into a premultiplied RGBA8 buffer — the ONCE
/// per-band "photo" that the UI stretches on pan/zoom instead of re-tracing
/// vectors every frame. Mirrors the vector look: base fill, land, dots,
/// border rings, graticule and optional region borders.
///
/// `colors` = `[raised, land, rings, hairline, dots, region]` — all in
/// `0xRRGGBBAA`, `a` carries the final alpha (renderer premultiplies the
/// same way, so alpha is baked here).
pub fn raster_band(
    bw: usize,
    bh: usize,
    win: &Win,
    w: &World,
    region: Option<&[Vec<(f32, f32)>]>,
    region_on: bool,
    style: u8,
    colors: [u32; 6],
) -> Vec<u8> {
    let mut out = vec![0u8; bw * bh * 4];
    let mw = bw as f32;
    let mh = bh as f32;
    let span_lon = win.lon_span();
    let span_lat = win.lat_span();
    // base backdrop (Real + Lines styles fill the pane)
    if style == 0 || style == 3 {
        let a = (colors[0] & 0xff) as u32;
        if a != 0 {
            let (r, g, b) = (
                ((colors[0] >> 24) & 0xff) as u32,
                ((colors[0] >> 16) & 0xff) as u32,
                ((colors[0] >> 8) & 0xff) as u32,
            );
            let (pr, pg, pb) = (r * a / 255, g * a / 255, b * a / 255);
            for px in out.chunks_exact_mut(4) {
                px[0] = pr as u8;
                px[1] = pg as u8;
                px[2] = pb as u8;
                px[3] = a as u8;
            }
        }
    }
    // land fill / dot pattern
    match style {
        0 | 1 => {
            let mut y = 0i64;
            while y < bh as i64 {
                let lat = win.lat1 - (y as f32 + 0.5) / mh * span_lat;
                let mut x = 0i64;
                while x < bw as i64 {
                    let lon0 = win.lon0 + (x as f32 + 0.5) / mw * span_lon;
                    if land_lonlat(w, lon0, lat) {
                        let sx = x;
                        let mut ex = x + 1;
                        while ex < bw as i64 {
                            let lon = win.lon0 + (ex as f32 + 0.5) / mw * span_lon;
                            if !land_lonlat(w, lon, lat) {
                                break;
                            }
                            ex += 1;
                        }
                        fill_run(&mut out, bw, bh, sx, ex, y, colors[1]);
                        x = ex;
                    } else {
                        x += 1;
                    }
                }
                y += 1;
            }
        }
        2 => {
            let dstep = ((mw / 32.0).round() as i64).max(2);
            let rad = (dstep as f32 * 0.24).round().max(1.0) as i64;
            let mut gy = dstep / 2;
            while gy < bh as i64 {
                let lat = win.lat1 - (gy as f32 + 0.5) / mh * span_lat;
                let mut gx = dstep / 2;
                while gx < bw as i64 {
                    let lon = win.lon0 + (gx as f32 + 0.5) / mw * span_lon;
                    if land_lonlat(w, lon, lat) {
                        for dy in -rad..=rad {
                            let y = gy + dy;
                            if y < 0 || y >= bh as i64 {
                                continue;
                            }
                            for dx in -rad..=rad {
                                put_px(&mut out, bw, bh, gx + dx, y, colors[4]);
                            }
                        }
                    }
                    gx += dstep;
                }
                gy += dstep;
            }
        }
        _ => {}
    }
    // border polylines + graticule (Real + Lines), region borders on top
    if style == 0 || style == 3 {
        stroke_region_raster(&mut out, bw, bh, win, &w.rings, colors[2]);
        raster_graticule(&mut out, bw, bh, win, colors[3]);
    }
    if region_on {
        if let Some(rings) = region {
            stroke_region_raster(&mut out, bw, bh, win, rings, colors[5]);
        }
    }
    out
}

fn fill_run(out: &mut [u8], bw: usize, bh: usize, sx: i64, ex: i64, y: i64, color: u32) {
    if y < 0 || y >= bh as i64 {
        return;
    }
    if sx >= bw as i64 {
        return;
    }
    let ex = ex.min(bw as i64);
    let a = (color & 0xff) as u32;
    if a == 0 {
        return;
    }
    let (r, g, b) = (
        ((color >> 24) & 0xff) as u32,
        ((color >> 16) & 0xff) as u32,
        ((color >> 8) & 0xff) as u32,
    );
    let (pr, pg, pb) = (r * a / 255, g * a / 255, b * a / 255);
    for x in sx.max(0)..ex.min(bw as i64) {
        let i = (y as usize * bw + x as usize) * 4;
        out[i] = pr as u8;
        out[i + 1] = pg as u8;
        out[i + 2] = pb as u8;
        out[i + 3] = a as u8;
    }
}

fn put_px(out: &mut [u8], bw: usize, bh: usize, x: i64, y: i64, color: u32) {
    if x < 0 || y < 0 || x >= bw as i64 || y >= bh as i64 {
        return;
    }
    let a = (color & 0xff) as u32;
    if a == 0 {
        return;
    }
    let (r, g, b) = (
        ((color >> 24) & 0xff) as u32,
        ((color >> 16) & 0xff) as u32,
        ((color >> 8) & 0xff) as u32,
    );
    let i = (y as usize * bw + x as usize) * 4;
    out[i] = (r * a / 255) as u8;
    out[i + 1] = (g * a / 255) as u8;
    out[i + 2] = (b * a / 255) as u8;
    out[i + 3] = a as u8;
}

fn stroke_line_raster(out: &mut [u8], bw: usize, bh: usize, x0: f32, y0: f32, x1: f32, y1: f32, t: f32, color: u32) {
    let dx = x1 - x0;
    let dy = y1 - y0;
    let steps = dx.abs().max(dy.abs()).ceil() as i64;
    let half = (t / 2.0).ceil() as i64;
    let step = if steps == 0 { 1 } else { steps };
    for i in 0..=steps {
        let px = (x0 + dx * i as f32 / step as f32).round() as i64;
        let py = (y0 + dy * i as f32 / step as f32).round() as i64;
        for oy in -half..=half {
            for ox in -half..=half {
                put_px(out, bw, bh, px + ox, py + oy, color);
            }
        }
    }
}

/// Border polylines in raster space, ports `stroke_rings` (antimeridian
/// wrap at `mw * 0.5`, gaps shorter than a raster px are dropped).
fn stroke_region_raster(out: &mut [u8], bw: usize, bh: usize, win: &Win, rings: &[Vec<(f32, f32)>], color: u32) {
    let mw = bw as f32;
    let mh = bh as f32;
    let span_lon = win.lon_span();
    let span_lat = win.lat_span();
    let proj = |lon: f32, lat: f32| {
        ((lon - win.lon0) / span_lon * mw, (win.lat1 - lat) / span_lat * mh)
    };
    let half = mw * 0.5;
    for ring in rings {
        let mut prev: Option<(f32, f32)> = None;
        for &(lon, lat) in ring.iter() {
            let (px, py) = proj(lon, lat);
            if let Some((x0, y0)) = prev {
                if (px - x0) * (px - x0) + (py - y0) * (py - y0) < 2.25 {
                    prev = Some((px, py));
                    continue;
                }
                let mut x1 = px;
                if x1 - x0 > half {
                    x1 -= mw;
                } else if x1 - x0 < -half {
                    x1 += mw;
                }
                let dx = x1 - x0;
                if dx * dx + (py - y0) * (py - y0) >= 0.81 {
                    stroke_line_raster(out, bw, bh, x0, y0, x1, py, 1.0, color);
                }
            }
            prev = Some((px, py));
        }
    }
}

/// 1px meridian/parallel grid at the same spacing as the vector `world_graticule`.
fn raster_graticule(out: &mut [u8], bw: usize, bh: usize, win: &Win, color: u32) {
    let mw = bw as f32;
    let mh = bh as f32;
    let span_lon = win.lon_span();
    let span_lat = win.lat_span();
    let mut lon = -150.0f32;
    while lon <= 150.0 {
        let px = ((lon - win.lon0) / span_lon * mw).round() as i64;
        if px >= 0 && px < bw as i64 {
            for y in 0..bh as i64 {
                put_px(out, bw, bh, px, y, color);
            }
        }
        lon += 30.0;
    }
    let mut lat = -60.0f32;
    while lat <= 60.0 {
        let py = ((win.lat1 - lat) / span_lat * mh).round() as i64;
        if py >= 0 && py < bh as i64 {
            for x in 0..bw as i64 {
                put_px(out, bw, bh, x, py, color);
            }
        }
        lat += 30.0;
    }
}

// ---------------- zone.tab timezone lookup ----------------

/// One `/usr/share/zoneinfo/zone.tab` row: display city + IANA tz + origin.
#[derive(Clone, Debug)]
pub struct Zone {
    pub city: String,
    pub tz: String,
    pub lat: f32,
    pub lon: f32,
}

/// Parse zone.tab text (fields tab-separated; comment column optional).
/// Format: `AD\t+4230+00131\tEurope/Andorra[ \tAndorra]`.
pub fn parse_zone_tab(text: &str) -> Vec<Zone> {
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut f = line.split('\t');
        let _cc = f.next();
        let Some(coords) = f.next() else { continue };
        let Some(tz) = f.next() else { continue };
        let comment = f.next().map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
        let (Some(lat), Some(lon)) = parse_coords(coords) else { continue };
        out.push(Zone {
            city: comment.unwrap_or_else(|| {
                tz.rsplit('/').next().unwrap_or(tz).replace('_', " ").to_string()
            }),
            tz: tz.trim().to_string(),
            lat,
            lon,
        });
    }
    out
}

fn parse_coords(s: &str) -> (Option<f32>, Option<f32>) {
    let signs: Vec<usize> = s.match_indices(['+', '-']).map(|(i, _)| i).collect();
    if signs.len() != 2 {
        return (None, None);
    }
    let lat_s = &s[signs[0]..signs[1]];
    let lon_s = &s[signs[1]..];
    (parse_ll(lat_s, 2), parse_ll(lon_s, 3))
}

/// `±DDMM[SS]` (lat) or `±DDDMM[SS]` (lon) → signed degrees. `deg` = number of
/// digits in the whole-degree field. Never validated as a real place.
fn parse_ll(s: &str, deg: usize) -> Option<f32> {
    let (sign, digits) = if let Some(r) = s.strip_prefix('+') {
        (1.0, r)
    } else if let Some(r) = s.strip_prefix('-') {
        (-1.0, r)
    } else {
        return None;
    };
    if digits.len() < deg {
        return None;
    }
    let (d, rest) = digits.split_at(deg);
    let d: f32 = d.parse().ok()?;
    let mut min = 0.0;
    let mut sec = 0.0;
    if rest.len() >= 2 {
        min = rest[..2].parse().ok()?;
    }
    if rest.len() >= 4 {
        sec = rest[2..4].parse().ok()?;
    }
    Some(sign * (d + min / 60.0 + sec / 3600.0))
}

/// The zone.tab file + parsed zones (lazily).
pub fn zones() -> &'static Vec<Zone> {
    static Z: OnceLock<Vec<Zone>> = OnceLock::new();
    Z.get_or_init(|| {
        std::fs::read_to_string("/usr/share/zoneinfo/zone.tab")
            .map(|t| parse_zone_tab(&t))
            .unwrap_or_default()
    })
}

/// Nearest zone to (lat, lon) — squared distance on the lat/lon plane
/// (lon scaled by cos lat, fine for a picker).
pub fn nearest_zone(lat: f32, lon: f32) -> Option<&'static Zone> {
    let zs = zones();
    if zs.is_empty() {
        return None;
    }
    let mut best: Option<(usize, f32)> = None;
    let k = lat.to_radians().cos().max(0.05);
    for (i, z) in zs.iter().enumerate() {
        let dlat = (z.lat - lat) * 111.0;
        let dlon = (z.lon - lon) * 111.0 * k;
        let d2 = dlat * dlat + dlon * dlon;
        match best {
            None => best = Some((i, d2)),
            Some((_, b)) if d2 < b => best = Some((i, d2)),
            _ => {}
        }
    }
    let (i, _) = best.unwrap();
    zs.get(i)
}

/// Parse a `%z` offset like "+0530"/"-0330"/"+05:30" into signed minutes.
pub fn parse_tz_offset(s: &str) -> Option<i32> {
    let s: String = s.trim().chars().filter(|c| !c.is_whitespace()).collect();
    let (sign, digits) = if let Some(r) = s.strip_prefix('+') {
        (1, r)
    } else if let Some(r) = s.strip_prefix('-') {
        (-1, r)
    } else {
        return None;
    };
    let digits: String = digits.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() < 2 {
        return None;
    }
    let (hh, mm) = if digits.len() <= 2 {
        (digits.parse::<i32>().ok()?, 0)
    } else {
        let ms = digits.len() - 2;
        (digits[..ms].parse().ok()?, digits[ms..].parse().ok()?)
    };
    Some(sign * (hh * 60 + mm))
}

/// Wall-clock "HH:MM" for a UTC epoch shifted by a signed minute offset.
pub fn world_hhmm(epoch: i64, off_min: i32) -> String {
    let shifted = epoch + off_min as i64 * 60;
    let mut buf = [0u8; 8];
    unsafe {
        let t = shifted as libc::time_t;
        let mut tm: libc::tm = std::mem::zeroed();
        libc::gmtime_r(&t, &mut tm);
        let n = libc::strftime(buf.as_mut_ptr() as *mut libc::c_char, buf.len(), c"%H:%M".as_ptr(), &tm);
        String::from_utf8_lossy(&buf[..n]).into_owned()
    }
}

/// Weekday + day + month, e.g. "Tue 07 Jan" for a shifted epoch.
pub fn world_dayline(epoch: i64, off_min: i32) -> String {
    let shifted = epoch + off_min as i64 * 60;
    let mut buf = [0u8; 24];
    unsafe {
        let t = shifted as libc::time_t;
        let mut tm: libc::tm = std::mem::zeroed();
        libc::gmtime_r(&t, &mut tm);
        let n = libc::strftime(buf.as_mut_ptr() as *mut libc::c_char, buf.len(), c"%a %e %b".as_ptr(), &tm);
        String::from_utf8_lossy(&buf[..n]).into_owned()
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    /// Serializes tests that touch the process-global world store (any test
    /// can `set_world`/`clear`, so behavior-sensitive tests must not race).
    pub static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn ring_chunk_roundtrips() {
        let buf = vec![
            0x01, 0x00, 0x00, 0x00, // 1 ring
            0x03, 0x00, 0x00, 0x00, // 3 points
            0x64, 0x00, 0x64, 0x00, // (100, 100) → (1.00, 1.00)
            0xC4, 0xFF, 0xCE, 0xFF, // (-60, -50) → (-0.60, -0.50)
            0x00, 0x00, 0xC8, 0x00, // (0, 200) → (0.00, 2.00)
        ];
        let rings = parse_ring_chunk(&buf).expect("chunk parses");
        assert_eq!(rings.len(), 1);
        assert_eq!(rings[0][0], (1.0, 1.0));
        assert_eq!(rings[0][1], (-0.6, -0.5));
        assert_eq!(rings[0][2], (0.0, 2.0));
    }

    #[test]
    fn city_chunk_roundtrips() {
        let mut buf = vec![1u8, 0, 0, 0];
        for v in [3i16, 4i16] {
            buf.extend_from_slice(&v.to_le_bytes());
        }
        let name = "Rio";
        buf.extend_from_slice(&(name.len() as u16).to_le_bytes());
        buf.extend_from_slice(name.as_bytes());
        let cities = parse_cities_chunk(&buf).expect("cities parse");
        assert_eq!(cities.len(), 1);
        assert_eq!(cities[0].name, "Rio");
        assert_eq!(cities[0].lon, 0.03);
        assert_eq!(cities[0].lat, 0.04);
    }

    #[test]
    fn mask_builds_without_panic_and_has_land() {
        let _g = TEST_LOCK.lock().unwrap();
        // a single square covering lon 0..10, lat 0..10
        let rings = vec![vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]];
        let w = set_world(rings).expect("set");
        assert!(land_lonlat(&w, 2.0, 2.0), "inside the square should be land");
        assert!(!land_lonlat(&w, -45.0, -20.0), "elsewhere is ocean");
    }

    #[test]
    fn raster_band_paints_land_ocean_and_rings() {
        let _g = TEST_LOCK.lock().unwrap();
        clear();
        // a square covering lon 0..60, lat 0..40 (lon 0..10 test square is too
        // tiny at whole-world scale — use the card-test's bigger square)
        let rings = vec![vec![(0.0, 0.0), (60.0, 0.0), (60.0, 40.0), (0.0, 40.0)]];
        let w = set_world(rings).expect("set");
        let win = Win { lon0: -180.0, lat0: -85.0, lon1: 180.0, lat1: 85.0 };
        let colors = [
            0x141414ff, 0x4fc2ffff, 0x404040ff, 0x222222ff, 0x333333ff, 0x43a0e0ff,
        ];
        let (bw, bh) = (360usize, 170usize);
        // filled style: land tinted, ocean transparent, no ring overdraw
        let px = raster_band(bw, bh, &win, &w, None, false, 1, colors);
        let at = |lon: f32, lat: f32| -> usize {
            let x = ((((lon - win.lon0) / win.lon_span()) * bw as f32).floor() as usize).min(bw - 1);
            let y = ((((win.lat1 - lat) / win.lat_span()) * bh as f32).floor() as usize).min(bh - 1);
            (y * bw + x) * 4
        };
        let i = at(30.0, 20.0);
        assert_eq!([px[i], px[i + 1], px[i + 2], px[i + 3]], [0x4f, 0xc2, 0xff, 0xff], "land tint");
        let j = at(120.0, -30.0);
        assert_eq!(px[j + 3], 0, "ocean stays transparent in filled style");
        // real style: base backdrop + border rings drawn over land/ocean edge
        let px2 = raster_band(bw, bh, &win, &w, None, false, 0, colors);
        let k = at(20.0, -45.0); // ocean — the raised backdrop must be there
        assert_eq!(px2[k], 0x14, "raised backdrop fills the pane");
        let k = at(0.0, 20.0); // near the ring's left edge (lon 0)
        assert_ne!(px2[k], 0x14, "ring stroke over the backdrop at lon≈0");
        clear();
    }

    #[test]
    fn embedded_world_is_always_parseable() {
        let _g = TEST_LOCK.lock().unwrap();
        // the compiled-in chunk must always yield a real map
        let rings = parse_ring_chunk(EMBEDDED_WORLD).expect("embedded chunk parses");
        assert!(!rings.is_empty(), "embedded chunk has rings");
        let total: usize = rings.iter().map(|r| r.len()).sum();
        assert!(total > 1000, "embedded chunk is a real dataset ({total} pts)");
        clear();
        ensure_fallback();
        assert!(ready(), "fallback makes the store ready");
        assert!(using_fallback(), "fallback flag set by ensure_fallback");
        let w = world().expect("world present");
        // somewhere in Brazil should be land with the embedded 110m dataset
        assert!(land_lonlat(&w, -52.0, -12.0), "South America must be land");
        clear();
        assert!(!using_fallback(), "clear resets the fallback flag");
    }

    #[test]
    fn set_world_marks_non_fallback() {
        let _g = TEST_LOCK.lock().unwrap();
        clear();
        ensure_fallback();
        assert!(using_fallback(), "precondition: fallback active");
        set_world(vec![vec![(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)]]);
        assert!(!using_fallback(), "a swapped-in world replaces the fallback");
        clear();
    }

    #[test]
    fn land_tracking_matches_fill() {
        let _g = TEST_LOCK.lock().unwrap();
        // equirect square spanning the equator
        let rings = vec![vec![(-20.0, -5.0), (20.0, -5.0), (20.0, 5.0), (-20.0, 5.0)]];
        let w = set_world(rings).expect("set");
        assert!(land_lonlat(&w, 0.0, 0.0), "equator centre should be land");
        assert!(!land_lonlat(&w, 0.0, 40.0), "north should be ocean");
    }

    #[test]
    fn window_and_projection_roundtrip() {
        // whole-world window at zoom 1 → identical to the old full map
        let wn = win_for(0.0, 0.0, 1.0);
        assert!((wn.lon0 - -180.0).abs() < 0.001);
        assert!((wn.lon1 - 180.0).abs() < 0.001);
        assert!((wn.lat0 - -85.0).abs() < 0.001);
        assert!((wn.lat1 - 85.0).abs() < 0.001);

        let (mx, my, mw, mh) = (10.0f32, 20.0f32, 400.0f32, 220.0f32);
        for (lon, lat) in [(0.0, 0.0), (-150.0, -10.0), (179.0, 80.0), (-179.0, -80.0)] {
            let (px, py) = proj(mx, my, mw, mh, &wn, lon, lat);
            let (rl, ra) = unproj(mx, my, mw, mh, &wn, px, py);
            assert!((rl - lon).abs() < 0.6, "lon mismatch {rl} vs {lon}");
            assert!((ra - lat).abs() < 0.6, "lat mismatch {ra} vs {lat}");
        }

        // zoomed window roundtrip around Tokyo
        let wz = win_for(139.0, 35.0, 8.0);
        assert!(wz.contains(139.0, 35.0), "center must be inside the window");
        for (lon, lat) in [(139.7, 35.7), (139.0, 35.0)] {
            let (px, py) = proj(mx, my, mw, mh, &wz, lon, lat);
            let (rl, ra) = unproj(mx, my, mw, mh, &wz, px, py);
            assert!((rl - lon).abs() < 0.1, "zoomed lon mismatch {rl} vs {lon}");
            assert!((ra - lat).abs() < 0.1, "zoomed lat mismatch {ra} vs {lat}");
        }
    }

    #[test]
    fn window_clamps_to_world() {
        let w = win_for(190.0, 89.0, 2.0);
        assert!(w.lon0 >= -180.0 && w.lon1 <= 180.0, "lon must stay in world");
        assert!(w.lat0 >= -85.0 && w.lat1 <= 85.0, "lat must stay in world bounds");
        let w = win_for(-200.0, 0.0, 2.0);
        assert!(w.lon0 >= -180.0 && w.lon1 <= 180.0, "lon must stay in world");
    }

    #[test]
    fn zone_tab_parser_reads_columns() {
        let txt = "AD\t+4230+00131\tEurope/Andorra\tAndorra\nJP\t+353916+1394441\tAsia/Tokyo\n";
        let z = parse_zone_tab(txt);
        assert_eq!(z.len(), 2);
        assert_eq!(z[0].tz, "Europe/Andorra");
        assert_eq!(z[0].city, "Andorra");
        assert!((z[0].lat - 42.5).abs() < 0.01);
        assert!((z[0].lon - 1.516).abs() < 0.02);
        assert!((z[1].lat - 35.654444).abs() < 0.01, "lat {}", z[1].lat);
        assert!((z[1].lon - 139.744722).abs() < 0.01, "lon {}", z[1].lon);
        assert_eq!(z[1].city, "Tokyo");
    }

    #[test]
    fn nearest_zone_picks_closest() {
        let txt = "AD\t+4230+00131\tEurope/Andorra\tAndorra\nJP\t+353916+1394441\tAsia/Tokyo\n";
        let z = parse_zone_tab(txt);
        let idx = z.iter().enumerate()
            .min_by(|a, b| {
                let da = (a.1.lat - 35.0).powi(2) + (a.1.lon - 139.0).powi(2);
                let db = (b.1.lat - 35.0).powi(2) + (b.1.lon - 139.0).powi(2);
                da.partial_cmp(&db).unwrap()
            })
            .map(|(i, _)| i)
            .unwrap();
        assert_eq!(idx, 1);
    }

    #[test]
    fn tz_offset_parsing() {
        assert_eq!(parse_tz_offset("+0530"), Some(330));
        assert_eq!(parse_tz_offset("-0330"), Some(-210));
        assert_eq!(parse_tz_offset("+0000"), Some(0));
        assert_eq!(parse_tz_offset("bogus"), None);
        assert_eq!(parse_tz_offset("+05:30"), Some(330));
        assert_eq!(parse_tz_offset("-05"), Some(-300));
    }

    #[test]
    fn world_time_shift() {
        let e0 = 1716268800;
        let s = world_hhmm(e0, 0);
        assert_eq!(s.len(), 5);
        let shifted = world_hhmm(e0, 60);
        assert_ne!(s, shifted, "one-hour shift must change the clock");
        let d = world_dayline(e0, 0);
        assert!(d.contains(' '));
    }
}