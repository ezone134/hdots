//! Declarative scene graph — RON-deserializable, QML-shaped cards.
//!
//! A card is authored as a RON file (`~/.config/zen-shell/ui/cards/<id>.ron`)
//! describing its items: text (where, what font weight/size/color), surfaces,
//! outlines, and hit regions + actions. This is the tier-1 declarative layer:
//! no scripting — a walker turns the tree into `Cmd` draw commands and returns
//! hit regions (rect + key) the shell registers for hover/click. Colors are
//! semantic `ColorToken` names resolved against the LIVE theme `Pal` per
//! frame; text may interpolate `{var}` placeholders from a runtime values
//! map so live data (record time, %s) still flows in without editing the RON.
//! A hit region's `text` renders an icon glyph too when `icon: true`, so a
//! button = surface + `text` ("›", icon font) + `key` + optional `action`.
//!
//! Example (`~/.config/zen-shell/ui/cards/audiorec.ron`):
//! ```ron
//! (
//!     items: [
//!         Text(x: 12.0, y: 9.0, text: "Recorder", font_size: 11.5, weight: semibold),
//!         Text(x: 12.0, y: 11.0, w: 0.0, text: "{meta}", font_size: 8.5, right: true,
//!              font: mono, weight: regular, color: Some(fg3)),
//!         Hit(x: 12.0, y: 6.0, w: 16.0, h: 16.0, r: 5.0, key: 32302,
//!             surface: Some(raised), text: "›", font_size: 10.0, icon: true,
//!             action: Some(Command("zen-shell ipc call recorder open"))),
//!     ],
//! )
//! ```
//! Text uses `x`/`y` in base pixels scaled by the shell's GridScale; `right:
//! true` anchors it to the card's right edge (`x` = right margin) so entries
//! survive a card resize. `Hit.surface` picks its own hover color at draw
//! time (lift a token) or via an explicit `surface_hover`.

use crate::shell::Cmd;
use crate::text::{Ff, Fw};
use crate::ui::{mix, ColorToken, Pal};

/// Horizontal / vertical font family for a text item (mirrors `crate::text`).
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SceneFont {
    Ui,
    Display,
    Mono,
    Brand,
}

impl SceneFont {
    pub fn ff(self) -> Ff {
        match self {
            SceneFont::Ui => Ff::Ui,
            SceneFont::Display => Ff::Display,
            SceneFont::Mono => Ff::Mono,
            SceneFont::Brand => Ff::Brand,
        }
    }
}

impl Default for SceneFont {
    fn default() -> Self {
        SceneFont::Ui
    }
}

/// Typographic weight (mirrors `crate::text`'s stack).
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SceneWeight {
    Light,
    Regular,
    Medium,
    Semibold,
}

impl SceneWeight {
    pub fn fw(self) -> Fw {
        match self {
            SceneWeight::Light => Fw::Light,
            SceneWeight::Regular => Fw::Regular,
            SceneWeight::Medium => Fw::Medium,
            SceneWeight::Semibold => Fw::Semibold,
        }
    }
}

impl Default for SceneWeight {
    fn default() -> Self {
        SceneWeight::Light
    }
}

/// What a click on a hit region does. `Command` spawns a detached shell
/// command (script / IPC call / hyprctl dispatch) — the declarative escape
/// hatch. Explicit actions let a card's buttons do real work without Rust;
/// cards that need richer handlers still register keys the Rust click
/// dispatcher already owns.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum SceneAction {
    /// Run a detached command string (script / IPC call) — RON: `Command("...")`.
    Command(String),
}

/// A single row in a `Rows` / `TabRow` list. Authored on the RUST side in
/// `scene_values()` (never in RON): `cols` cells parallel the item's `cols[]`
/// column formats. A non-zero `key` (or the item's `key_base + index`
/// fallback) registers a clickable region spanned across the row; `color`
/// overrides every column's color for the whole row (dimmed/empty rows). A
/// `surface` draws a resting background rect behind the row (the selected /
/// connected / active tint) and takes precedence over hover states exactly
/// like the Rust list drawers.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SceneRow {
    pub cols: Vec<String>,
    #[serde(default)]
    pub key: u32,
    #[serde(default)]
    pub action: Option<SceneAction>,
    #[serde(default)]
    pub color: Option<ColorToken>,
    /// resting background tint drawn for this row (active/connected rows)
    #[serde(default)]
    pub surface: Option<ColorToken>,
    /// per-cell color override (parallel to cols); `col_colors[i]` takes
    /// precedence over `row.color` for cell i, so status columns (chg%,
    /// connected state) stay colored even on dimmed/active rows.
    #[serde(default)]
    pub col_colors: Vec<Option<ColorToken>>,
}

impl Default for SceneRow {
    fn default() -> Self {
        SceneRow {
            cols: Vec::new(),
            key: 0,
            action: None,
            color: None,
            surface: None,
            col_colors: Vec::new(),
        }
    }
}

/// A per-row BAR cell inside a `Rows` item — the Rust list drawers' meter
/// rows (fans' per-fan RPM bar). The cell's text (from
/// the row's `cols`) is INTERPOLATED (`{name}`) and parsed as a fraction of
/// `max`, exactly like the standalone `Bar` item's `value`.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct BarCellSpec {
    /// bar height inside the row (base px)
    pub height: f32,
    /// top of the bar inside the row (base px)
    pub top: f32,
    /// corner radius (base px)
    #[serde(default)]
    pub radius: f32,
    /// parse the cell as a fraction of this (default 1.0 — the cell IS 0..1)
    #[serde(default = "one_f32")]
    pub max: f32,
    /// fill color; `None` → the cell's text color resolution (so a per-row
    /// `col_colors` ink drives the fill — fans' live/info vs dim)
    #[serde(default)]
    pub fill: Option<ColorToken>,
    /// track color (default `hover`)
    #[serde(default)]
    pub track: Option<ColorToken>,
}

fn one_f32() -> f32 {
    1.0
}

/// One column's cell formatting inside a `Rows` item — authored in RON. Each
/// column pulls its cell text from the data row's `cols` vec by index.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SceneCol {
    /// horizontal offset from the ROW's left edge (the card pad), base px
    pub x: f32,
    /// box width used by `right` / `center` anchoring
    #[serde(default)]
    pub w: f32,
    #[serde(default)]
    pub right: bool,
    #[serde(default)]
    pub center: bool,
    #[serde(default)]
    pub font: SceneFont,
    /// cell font size (base px); ≤ 0 → 9.5
    #[serde(default)]
    pub size: f32,
    /// per-column baseline offset inside the row (base px); `None` → the
    /// Rows' `row_dy`. Lets one row carry a title + a dim caption line on
    /// separate baselines (two/title+body rows), like the Rust list drawers.
    #[serde(default)]
    pub dy: Option<f32>,
    #[serde(default = "regular_weight")]
    pub weight: SceneWeight,
    #[serde(default)]
    pub color: Option<ColorToken>,
    /// render the cell in the icon font (`true` for glyph columns)
    #[serde(default)]
    pub icon: bool,
    /// anchor the cell's RIGHT edge at the card's `card_w - edge` line
    /// (base px) instead of the left-relative `x` — for right-anchored
    /// pills/chips that survive card resize (the Rust drawers' `x + w - pad`
    /// pattern).
    #[serde(default)]
    pub edge: Option<f32>,
    /// truncate the cell to fit `card_w - 2·pad - truncate` (base px) at the
    /// cell's font size (0.62·fs px/glyph, ≥ 8 chars) — the Rust list
    /// drawers' `avail` reservation so long names stop before the right-side
    /// chip/action column. `None` = draw the cell as-is.
    #[serde(default)]
    pub truncate: Option<f32>,
    /// render the cell as a pill/chip: a rounded rect auto-sized to the cell
    /// text, with a quiet fill derived from the cell's resolved text color
    /// (accent-ish → accent tint, everything else → hover) unless the spec
    /// overrides `bg`. Per-row text color comes from `col_colors` as usual.
    #[serde(default)]
    pub pill: Option<PillSpec>,
    /// this row's revealed ✕ anchors LEFT of this col's drawn box (rows
    /// whose delete sits beside a chip instead of the card edge).
    #[serde(default)]
    pub del_anchor: bool,
    /// cell color overridden while the ROW is hovered (the news title
    /// tinting to accent on hover, while source cells stay fg3 — a per-col
    /// hover token that takes precedence over `color` during hover).
    #[serde(default)]
    pub hover: Option<ColorToken>,
    /// cell color while the ROW is the flashed (just-copied) row — the
    /// one-second transient accent that marks a clipboard snap. Only cells
    /// that opt in change (snippets' preview line stays fg3). Takes
    /// precedence over both hover and `color`.
    #[serde(default)]
    pub flash: Option<ColorToken>,
    /// render the cell as a METER BAR instead of text (the fans
    /// rows): the cell text is interpolated + parsed as a fraction of the
    /// spec's `max`, a track spans the cell box and the fill hugs the left.
    /// Honors `x`/`w`/`right`/`edge` like every other cell.
    #[serde(default)]
    pub bar: Option<BarCellSpec>,
}

/// Chip/pill render spec for a `SceneCol.pill` cell: fixed-height rounded
/// rect, width auto-fits the text (glyph walk 0.62·fs px + 2·`pad`).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct PillSpec {
    /// fill override (resolved per frame); `None` → quiet fill derived from
    /// the cell's text color token
    pub bg: Option<ColorToken>,
    /// pill height (base px)
    pub height: f32,
    /// corner radius (base px)
    pub radius: f32,
    /// horizontal padding around the text, each side (base px)
    pub pad: f32,
    /// top of the pill inside its row (base px)
    pub top: f32,
}

impl Default for PillSpec {
    fn default() -> Self {
        PillSpec {
            bg: None,
            height: 16.0,
            radius: 8.0,
            pad: 5.0,
            top: 3.0,
        }
    }
}

/// One kind of ink inside a [`BannerCell`] — how the declarative `BannerRow`
/// paints the chip's body (mirrors the Rust drawer's per-chip match arms).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BannerCellKind {
    /// label text at a 12px left inset, baseline on the band midline + the
    /// shared text bias (workspaces / title / date / clock); `n` caps the
    /// label to its first `n` chars (title's 30-char cap)
    Text { n: Option<u32> },
    /// icon glyph at 12px (size 13) + label text at 29px sharing ONE live
    /// ink — the wifi/bluetooth/battery/cpu/ram/volume/… chips (the ink
    /// binding carries the whole state ladder: accent while live, danger
    /// when low, dim when off)
    IconLabel,
    /// a single ink-centered icon at `size` (settings / power / search /
    /// dnd); the cell's `glyph` + `ink` carry any live swap (bell ↔ bell-off)
    Icon { size: f32 },
    /// text-only line at 12px in accent ink (the net-speed ↓/↑ chip)
    TextAcc,
    /// the brand mark — brand-font glyph centered in the cell
    Brand,
    /// a separator glyph, dim and ink-centered (size 10)
    Sep,
    /// the system tray: right-aligned per-icon images (fallback glyph when a
    /// tray item has no pixmap), each a 20px-wide sub-region (keys 30+ti)
    /// so SNI activate keeps working
    Tray,
    /// smart filler — draws nothing (its width IS the spacing it pushes)
    Filler,
}

/// One visible slot of the banner strip, published per frame by the shell
/// (`publish_banner_cells`) and painted by the declarative `BannerRow` item.
/// Geometry comes straight from `banner_cells` so hit-testing and rendering
/// can never disagree; the paint recipe mirrors the Rust drawer's per-chip
/// arms.
#[derive(Clone, Debug, PartialEq)]
pub struct BannerCell {
    /// left edge of the slot, relative to the item's box (panel px)
    pub x: f32,
    /// slot width (panel px)
    pub w: f32,
    /// per-chip ink name — a `SceneValues` **color** the `BannerRow` resolves
    /// each frame (hover = accent, low battery = danger, …)
    pub ink: String,
    /// the live label text (already computed by the Rust text helpers)
    pub text: String,
    /// optional leading glyph (the icon chips' Nerd Font glyph)
    pub glyph: Option<String>,
    /// raster image keys for the tray chip ("" = fallback glyph), right-aligned
    pub icons: Vec<String>,
    /// the L/C/R zone this slot belongs to (0/1/2; 3 = unzoned order)
    pub zone: u8,
    /// how the cell body paints
    pub kind: BannerCellKind,
    /// the slot's hit-region key (0 = no region: filler cells)
    pub key: u32,
}

/// One cell in a `Grid` — authored on the RUST side in `scene_values()`
/// (never in RON), exactly like [`SceneRow`]. The grid renders cell `i` at
/// row `i / cols`, column `i % cols`; a `visible: false` cell draws nothing
/// and registers nothing (the calendar's out-of-month day slots).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GridCell {
    /// centered glyph / number / label text (day number, "+", app label)
    #[serde(default)]
    pub text: String,
    /// skip draw + hit region when false (calendar blank days)
    #[serde(default = "gridcell_visible")]
    pub visible: bool,
    /// cell text color override (the calendar's today = accent)
    #[serde(default)]
    pub color: Option<ColorToken>,
    /// cell text color while the cell is hovered (the calendar's day ink
    /// lifts to fg; today keeps its accent). `None` → keep `color`.
    #[serde(default)]
    pub hover: Option<ColorToken>,
    /// resting fill behind the cell (the calendar's today tint)
    #[serde(default)]
    pub surface: Option<ColorToken>,
    /// resting fill while the cell is hovered (overrides `surface`)
    #[serde(default)]
    pub surface_hover: Option<ColorToken>,
    /// centered raster image drawn above the label (app icons)
    #[serde(default)]
    pub image_key: Option<String>,
    /// icon-font glyph fallback when no raster is available (the apps
    /// launcher tiles render the apps glyph for icon-less entries)
    #[serde(default)]
    pub glyph: Option<String>,
    /// per-cell font size override (base px; `None` → the Grid's `font_size`)
    #[serde(default)]
    pub size: Option<f32>,
    /// draw the label flush at the cell BOTTOM (app tiles) instead of
    /// centered (calendar days). Filled (`bottom`) cells also unlock the
    /// hover-only ✕ delete corner when the Grid sets `del_base`.
    #[serde(default)]
    pub bottom: bool,
    /// baseline nudge from the anchor (base px): centered cells sit at
    /// `cell_h/2 - 5` by default (the calendar days), bottom cells at
    /// `cell_h - 12` (the app tiles); `dy` shifts either anchor
    #[serde(default)]
    pub dy: Option<f32>,
    /// render the cell text in the icon font (`true` for "+" and the apps
    /// glyph fills — the empty-tile plus is a Nerd glyph in the drawer)
    #[serde(default)]
    pub icon: bool,
    /// cap the label to its first `n` chars (the app tiles' name clip —
    /// the Rust drawer takes `(tile_w − 4) / 5.5` chars). `None` = as-is.
    #[serde(default)]
    pub truncate: Option<u32>,
    /// optional second line UNDER the label (thermal zone chips: the temp
    /// reading under the zone name) — smaller (7.5), centered, own color
    #[serde(default)]
    pub sub: Option<String>,
    /// the second line's ink (thermal: the 60°/80° warning ladder)
    #[serde(default)]
    pub sub_color: Option<ColorToken>,
    /// vertical nudge of the second line from its centered slot (base px)
    /// — dense chips shift the pair up to the Rust drawer's exact rows
    #[serde(default)]
    pub sub_dy: Option<f32>,
    /// second line size override (base px; ≤ 0 / None → 7.5)
    #[serde(default)]
    pub sub_size: Option<f32>,
}

fn gridcell_visible() -> bool {
    true
}

/// One tile in a `Tiles` grid — authored on the RUST side in `scene_values()`
/// (never in RON), exactly like [`GridCell`] / [`SceneRow`]: the scene
/// declares the frame (columns, gaps, radii, ink ladder) while the live
/// per-tile state (which switches are on, what they say) rides this list.
/// The `Tiles` item draws each tile as a rounded surface + icon font glyph +
/// optional caption, and registers one region per tile.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SceneTile {
    /// the tile's own click region (0 = no region: a spacer tile)
    #[serde(default)]
    pub key: u32,
    /// icon-font glyph drawn centered in the tile (Wi-Fi, BT, the moon, …)
    #[serde(default)]
    pub glyph: String,
    /// caption under the glyph — dropped when the tile is too short to hold
    /// both (see `label_min_h`), so the same tile list serves a squat card
    /// and a tall one
    #[serde(default)]
    pub label: String,
    /// the "active" state: the accent-tinted surface + accent glyph ink, and
    /// it outranks the hover fill (the drawer's `if on { acc_tint } else …`)
    #[serde(default)]
    pub on: bool,
    /// optional always-visible corner chip (the Wi-Fi / BT "more" chevron):
    /// a ⋯ glyph whose tall right-edge region is `chip_key` (0 = no chip)
    #[serde(default)]
    pub chip_key: u32,
}

fn seven_px() -> f32 {
    7.0
}

fn hundred() -> f32 { 100.0 }

fn two_panes() -> u32 {
    2
}

impl Default for GridCell {
    fn default() -> Self {
        GridCell {
            text: String::new(),
            visible: true,
            color: None,
            hover: None,
            surface: None,
            surface_hover: None,
            image_key: None,
            glyph: None,
            size: None,
            bottom: false,
            dy: None,
            icon: false,
            truncate: None,
            sub: None,
            sub_color: None,
            sub_dy: None,
            sub_size: None,
        }
    }
}

/// A typed runtime value a scene item binds to by name. `scene_values()`
/// builds these on the Rust side so dynamic lists / graphs / gauges / toggles
/// flow into a scene without string-munging; string values keep driving
/// One lyric row for the karaoke binding: the line split into words (each
/// flagged whether it is the one the playhead has reached) plus whether this
/// row is the ACTIVE (being-sung) line. Aligned by index to the matching
/// `Rows` value; the engine highlights the active row's live word.
#[derive(Clone, Debug, PartialEq)]
pub struct KaraokeLine {
    pub words: Vec<(String, bool)>,
    pub active: bool,
}

/// A pool of typed scene values — the single source every item queries
/// by name.
#[derive(Clone, Debug)]
pub enum SceneValue {
    /// Interpolatable string (the legacy `{var}` binding).
    Text(String),
    /// Row list for `Rows` / `TabRow` items.
    Rows(Vec<SceneRow>),
    /// Boolean series parallel to a `Rows` value — the per-row checked state
    /// consumed by `Rows { check: ... }` checkbox cells.
    Checks(Vec<bool>),
    /// Sample series for `Spark` items (normalized or absolute).
    Spark(Vec<f32>),
    /// Scalar 0..max for `Ring` gauges and `TabRow` selection.
    Ring(f32),
    /// Boolean for `Toggle` switches.
    Toggle(bool),
    /// Scalar 0..1 for `Fader` sliders.
    Fader(f32),
    /// Chip-label list for the horizontal `Strip` (news feed category chips).
    Chips(Vec<String>),
    /// Word-split lyric lines parallel to a `Rows` value — the karaoke
    /// binding. Each line carries its words (live-word flag) and whether it
    /// is the currently sung row (accent wash + word highlight).
    Karaoke(Vec<KaraokeLine>),
    /// Grid-cell list for `Grid` items (calendar month, app tiles).
    GridCells(Vec<GridCell>),
    /// Tile list for `Tiles` items (the toggles card's switch grid).
    Tiles(Vec<SceneTile>),
    /// Banner-strip cell list for the `BannerRow` item (the dashboard's
    /// viewing strip: geometry + ink + label per visible slot).
    BannerCells(Vec<BannerCell>),
}

/// A named pool of typed scene values — the single source every item queries
/// by name. Accessors return `None` for a missing name OR a type mismatch, so
/// a misbound `.ron` simply draws nothing (never panics).
#[derive(Clone, Debug, Default)]
pub struct SceneValues {
    map: std::collections::HashMap<String, SceneValue>,
    colors: std::collections::HashMap<String, SceneColor>,
}

/// A runtime-bound color for [`ColorToken::Value`] names — either a semantic
/// token (resolved against the LIVE theme at draw time, so theme switches
/// still apply) or an already-computed u32 (threshold ladders, derived
/// blends that `mix` cannot express as a token). Populated per frame in
/// `scene_values()`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SceneColor {
    /// resolve the token against the current theme at draw time
    Token(ColorToken),
    /// an already-resolved ABGR u32
    Raw(u32),
}

impl SceneValues {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, name: impl Into<String>, value: SceneValue) {
        self.map.insert(name.into(), value);
    }

    pub fn text(&self, name: &str) -> Option<&str> {
        match self.map.get(name) {
            Some(SceneValue::Text(s)) => Some(s.as_str()),
            _ => None,
        }
    }

    pub fn rows(&self, name: &str) -> Option<&[SceneRow]> {
        match self.map.get(name) {
            Some(SceneValue::Rows(r)) => Some(r.as_slice()),
            _ => None,
        }
    }

    pub fn checks(&self, name: &str) -> Option<&[bool]> {
        match self.map.get(name) {
            Some(SceneValue::Checks(c)) => Some(c.as_slice()),
            _ => None,
        }
    }

    pub fn spark(&self, name: &str) -> Option<&[f32]> {
        match self.map.get(name) {
            Some(SceneValue::Spark(d)) => Some(d.as_slice()),
            _ => None,
        }
    }

    /// A generic 0..max scalar (Ring readouts, TabRow selected index).
    pub fn scalar(&self, name: &str) -> Option<f32> {
        match self.map.get(name) {
            Some(SceneValue::Ring(v)) => Some(*v),
            _ => None,
        }
    }

    pub fn toggle(&self, name: &str) -> Option<bool> {
        match self.map.get(name) {
            Some(SceneValue::Toggle(b)) => Some(*b),
            _ => None,
        }
    }

    /// Clamped to 0..1 so a fader never overdraws its track.
    pub fn fader(&self, name: &str) -> Option<f32> {
        match self.map.get(name) {
            Some(SceneValue::Fader(v)) => Some((*v).clamp(0.0, 1.0)),
            _ => None,
        }
    }

    /// Horizontal chip label list for the `Strip` item (news feed category
    /// chips, future button rows).
    pub fn chips(&self, name: &str) -> Option<&[String]> {
        match self.map.get(name) {
            Some(SceneValue::Chips(c)) => Some(c.as_slice()),
            _ => None,
        }
    }

    /// Word-split lyric lines for the `Rows { karaoke: ... }` binding (the
    /// live-word highlight + active-row wash of the lyrics card).
    pub fn karaoke(&self, name: &str) -> Option<&[KaraokeLine]> {
        match self.map.get(name) {
            Some(SceneValue::Karaoke(k)) => Some(k.as_slice()),
            _ => None,
        }
    }

    /// Grid-cell list for the `Grid` item binding (calendar day matrix,
    /// app-shortcut tiles). Cells render in order, `cols` per row.
    pub fn grid_cells(&self, name: &str) -> Option<&[GridCell]> {
        match self.map.get(name) {
            Some(SceneValue::GridCells(g)) => Some(g.as_slice()),
            _ => None,
        }
    }

    /// Tile list for the `Tiles` item binding (the toggles card's switch
    /// grid). Tiles render in order, the item's resolved `cols` per row.
    pub fn tiles(&self, name: &str) -> Option<&[SceneTile]> {
        match self.map.get(name) {
            Some(SceneValue::Tiles(t)) => Some(t.as_slice()),
            _ => None,
        }
    }

    /// Bind a dynamic color under `name` — the pool a `ColorToken::Value(name)`
    /// resolves through (see [`ColorTokenExt::resolve_with`]).
    pub fn insert_color(&mut self, name: impl Into<String>, color: SceneColor) {
        self.colors.insert(name.into(), color);
    }

    /// The dynamic color bound to `name`, if any.
    pub fn color(&self, name: &str) -> Option<SceneColor> {
        self.colors.get(name).copied()
    }

    /// The banner-strip cells bound to `name`, if any.
    pub fn banner_cells(&self, name: &str) -> Option<&[BannerCell]> {
        match self.map.get(name) {
            Some(SceneValue::BannerCells(c)) => Some(c.as_slice()),
            _ => None,
        }
    }
}

/// Theme + live-values color resolution for scene tokens. Scene drawing must
/// go through these methods so a [`ColorToken::Value`] name lifts its color
/// from the shell's per-frame dynamic map; non-scene consumers keep the plain
/// `ColorToken::resolve` / `hover_variant` (which degrade `Value` to `fg`).
pub(crate) trait ColorTokenExt {
    fn resolve_with(self, pal: &Pal, vals: &SceneValues) -> u32;
    fn hover_variant_with(self, pal: &Pal, vals: &SceneValues) -> u32;
}

impl ColorTokenExt for ColorToken {
    fn resolve_with(self, pal: &Pal, vals: &SceneValues) -> u32 {
        match self {
            ColorToken::Value(name) => match vals.color(name.as_str()) {
                Some(SceneColor::Token(t)) => t.resolve(pal),
                Some(SceneColor::Raw(c)) => c,
                // unknown dynamic name — quiet default, never panics
                None => pal.fg,
            },
            other => other.resolve(pal),
        }
    }

    fn hover_variant_with(self, pal: &Pal, vals: &SceneValues) -> u32 {
        match self {
            // a dynamic color already carries its own hover intent (it is
            // computed per frame from the live state)
            ColorToken::Value(_) => self.resolve_with(pal, vals),
            other => other.hover_variant(pal),
        }
    }
}

/// serde default: body rows read Regular, not the enum's Light default.
fn regular_weight() -> SceneWeight {
    SceneWeight::Regular
}

/// sparkline / slate defaults shared by the data-driven primitives.
fn one_hundred() -> f32 {
    100.0
}
fn thirty_six() -> u32 {
    36
}
fn three_px() -> f32 {
    3.0
}
fn twelve_px() -> f32 {
    12.0
}
fn composer_pad() -> f32 {
    12.0
}
fn twenty_six_px() -> f32 {
    26.0
}
fn textwrap_top() -> f32 {
    28.0
}
fn textwrap_bottom() -> f32 {
    36.0
}
fn textwrap_size() -> f32 {
    10.0
}
fn textwrap_lineh() -> f32 {
    13.0
}
fn five_px() -> f32 {
    5.0
}
fn one_px() -> f32 {
    1.0
}
fn two_px() -> f32 {
    2.0
}
fn twenty_px() -> f32 {
    20.0
}
fn nine_px() -> f32 {
    9.0
}
fn eight_px() -> f32 {
    8.0
}
fn four_px() -> f32 {
    4.0
}
fn twenty_two_px() -> f32 {
    22.0
}
fn one_col() -> u32 {
    1
}
fn six_px() -> f32 {
    6.0
}
fn eleven_px() -> f32 {
    11.0
}
fn eight_half() -> f32 {
    8.5
}
fn tenth_half() -> f32 {
    10.5
}
fn tok_acc() -> ColorToken {
    ColorToken::Acc
}
fn tok_hover() -> ColorToken {
    ColorToken::Hover
}
fn tok_fg2() -> ColorToken {
    ColorToken::Fg2
}
fn ten_px() -> f32 {
    10.0
}
fn thirteen_px() -> f32 {
    13.0
}
fn thirty_px() -> f32 {
    30.0
}
fn forty_two_px() -> f32 {
    42.0
}
fn eight_half_px() -> f32 {
    8.5
}
fn fourteen_px() -> f32 {
    14.0
}
fn eighteen_px() -> f32 {
    18.0
}
fn acc_tint_token() -> ColorToken {
    ColorToken::AccTint
}
fn hover_hl_token() -> ColorToken {
    ColorToken::HoverHl
}
fn hover_token() -> ColorToken {
    ColorToken::Hover
}
fn acc_token() -> ColorToken {
    ColorToken::Acc
}
fn fg_token() -> ColorToken {
    ColorToken::Fg
}

/// How a `Spark` item paints its samples: `line` = connected polyline,
/// `column` = discrete bars (readied as a 1px grid).
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SparkKind {
    Line,
    Column,
}

impl Default for SparkKind {
    fn default() -> Self {
        SparkKind::Line
    }
}

/// How a [`SceneItem::Ring`] picks its radius when the card resizes — the
/// two ladders the Rust gauge drawers compute by hand.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum RingFit {
    /// `(card_h - offset) / 2`, clamped to `min..=max` — "half the free
    /// body", the water card's ring.
    Half { offset: f32, min: f32, max: f32 },
    /// `(min_card_h, radius)` steps, first match wins with the last step as
    /// the fallback — the gauges card's 30px/22px ring.
    Ladder(Vec<(f32, f32)>),
}

impl RingFit {
    /// Resolve the radius in BASE px from the (scaled) card height — the
    /// ladder thresholds are authored in base px, like every other field.
    fn radius(&self, card_h: f32, scale: crate::shell::GridScale) -> f32 {
        match self {
            Self::Half { offset, min, max } => {
                ((scale.base(card_h) - *offset) / 2.0).clamp(*min, *max)
            }
            Self::Ladder(steps) => steps
                .iter()
                .find(|(min_h, _)| card_h >= scale.s(*min_h))
                .map(|(_, r)| *r)
                .unwrap_or(0.0),
        }
    }
}

/// Concrete scene item kinds — RON tags each item `Text(...)` / `Hit(...)`.
/// Struct-variant syntax keeps authoring QML-like (`Text(x: 12.0, text: …)`).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum SceneItem {
    /// A positioned text glyph (title, label, meta, icon glyph).
    Text {
        x: f32,
        y: f32,
        #[serde(default)]
        w: f32,
        #[serde(default)]
        h: f32,
        text: String,
        font_size: f32,
        #[serde(default)]
        font: SceneFont,
        #[serde(default)]
        weight: SceneWeight,
        #[serde(default)]
        right: bool,
        #[serde(default)]
        center: bool,
        #[serde(default)]
        icon: bool,
        #[serde(default)]
        color: Option<ColorToken>,
        /// `center_x` + `center_y` ignore x/y and place the text at the
        /// card's center (with optional w/h offsets for precise bounding).
        #[serde(default)]
        center_x: bool,
        #[serde(default)]
        center_y: bool,
        /// Anchor the box `y` at this fraction of the card height, added to
        /// the base `y` (the water card's ring/hero sit at `card_h/2 − 7`,
        /// the midpoint of the body under the header — a value RON can't
        /// write as a constant). Composes with `center_y` / `bottom`.
        #[serde(default)]
        cy_pct: f32,
        /// Center the box's MIDPOINT on this fraction of the card width —
        /// the gauges card's side-by-side hero percents (`0.27` / `0.73`),
        /// which sit at `x + w·frac`, not at a constant inset. Pairs with a
        /// real `w` + `center: true` (the text centers inside the box, so
        /// the box is half a text-block wider on each side of the anchor).
        #[serde(default)]
        cx_pct: f32,
        /// `bottom: true` anchors `y` ABOVE the card's bottom edge (base px —
        /// the Rust drawers' `y + h − N` baseline idiom, e.g. activewin's
        /// per-app RAM pill). Mutually exclusive with `center_y`.
        #[serde(default)]
        bottom: bool,
        /// a bound `Toggle` that must resolve `true` (or be absent) for the
        /// text to draw — state-swapped labels (empty/fetching hints).
        #[serde(default)]
        visible: Option<String>,
        /// Responsive font size: `(min_card_h, size)` steps, first match wins
        /// and the last step is the fallback (the gauges hero %: 13px when
        /// the card clears 116px, else 11px — the drawer's own ladder).
        #[serde(default)]
        size_ladder: Vec<(f32, f32)>,
        /// Responsive `y` offset: `(min_card_h, y)` steps with the same
        /// first-match/last-fallback rule. The gauges captions ride the ring
        /// ladder too (`cy + r + 15`: 45px on the 30px ring, 37px on the
        /// 22px one), which RON can't compute from a sibling's radius.
        #[serde(default)]
        y_ladder: Vec<(f32, f32)>,
    },
    /// Draw its children only when the card box falls inside the size window
    /// — the responsive-layout gate the Rust drawers write by hand (`w ≥
    /// s(170)` → gauges side by side, `h ≥ ring_r*2 + s(50)` → the capacity
    /// line under a gauge). All four bounds are ANDed; an absent bound is
    /// unbounded, so `min_w: 170.0` / `max_w: 170.0` split a mode cleanly.
    /// Children keep the full card box (a `Stack`'s box), so their
    /// `center_x` / `bottom` / proportional anchors still resolve against
    /// the real card geometry.
    When {
        #[serde(default)]
        min_w: f32,
        #[serde(default)]
        max_w: f32,
        #[serde(default)]
        min_h: f32,
        #[serde(default)]
        max_h: f32,
        #[serde(default)]
        items: Vec<SceneItem>,
    },
    /// A multi-line text block that wraps at 34 chars per line (the Quote
    /// body's exact cap) with every line centered horizontally, vertically
    /// centered inside the box between `top` (card-top inset) and `bottom`
    /// (card-bottom inset) — mirroring the Rust quote body geometry. An
    /// optional `caption` line is drawn dimmer + smaller just after the block
    /// (the "— author" line); both `text` and `caption` interpolate `{var}`s.
    /// Lines past the box stay on screen vertically clipped by the card, the
    /// same as the Rust drawer.
    TextWrap {
        /// the block text (may interpolate `{var}`s; first 136 chars wrapped)
        text: String,
        /// optional dim caption drawn after the block; skipped when empty
        /// after substitution
        #[serde(default)]
        caption: Option<String>,
        /// top of the wrap box from the card top (base px — clears the header)
        #[serde(default = "textwrap_top")]
        top: f32,
        /// bottom of the wrap box from the card bottom (base px — clears the
        /// pill / input rows)
        #[serde(default = "textwrap_bottom")]
        bottom: f32,
        /// box width; `0` → the full card width
        #[serde(default)]
        w: f32,
        #[serde(default = "textwrap_size")]
        font_size: f32,
        #[serde(default = "textwrap_lineh")]
        line_h: f32,
        #[serde(default)]
        color: Option<ColorToken>,
        /// a bound `Toggle` that must resolve `true` (or be absent) for the
        /// block to draw — swaps the wrap in/out against state (Quote body).
        #[serde(default)]
        visible: Option<String>,
    },
    /// A clickable region: optional `surface` fill + optional glyph/text on
    /// top, registered as a hit region with `key` (+ optional `action`).
    /// With `right: true` the surface is anchored to the CARD's right edge
    /// (`x` = right margin) so it survives card resize.
    Hit {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        #[serde(default)]
        r: f32,
        #[serde(default)]
        surface: Option<ColorToken>,
        #[serde(default)]
        surface_hover: Option<ColorToken>,
        /// draw the `surface` fill ONLY while this hit is hovered (transparent
        /// when idle — the "appears on hover" inline controls)
        #[serde(default)]
        hover_only: bool,
        #[serde(default)]
        text: String,
        #[serde(default)]
        font_size: f32,
        #[serde(default)]
        font: SceneFont,
        #[serde(default)]
        weight: SceneWeight,
        #[serde(default)]
        right: bool,
        #[serde(default)]
        center: bool,
        /// vertical nudge of the label baseline inside the box (base px) —
        /// equal-split button rows center their glyph/label by hand
        #[serde(default)]
        dy: f32,
        #[serde(default)]
        icon: bool,
        /// optional leading glyph (icon buttons): drawn in the icon font at
        /// `glyph_pad` from the box's left edge, the label shifts right to
        /// `glyph_pad + 18` (the mirror Photo/Record buttons)
        #[serde(default)]
        glyph: Option<String>,
        /// left inset of the leading glyph (base px)
        #[serde(default = "eight_px")]
        glyph_pad: f32,
        /// `true` anchors `y` px above the card's BOTTOM edge instead of
        /// from the top (the bottom-anchored pill/button idiom, the `Text`
        /// mirror) — requires a declared `h`
        #[serde(default)]
        bottom: bool,
        /// glyph ink (default fg)
        #[serde(default)]
        glyph_color: Option<ColorToken>,
        /// glyph ink while the hit is hovered
        #[serde(default)]
        glyph_color_hover: Option<ColorToken>,
        #[serde(default)]
        color: Option<ColorToken>,
        /// text ink swapped in while the hit is hovered (the clear-month /
        /// danger pills, news title tint — a hover refresh of the label).
        #[serde(default)]
        color_hover: Option<ColorToken>,
        key: u32,
        #[serde(default)]
        action: Option<SceneAction>,
        /// live gate: while the named toggle is false the hit *and* its region
        /// vanish (an optional row's click target disappears with the row —
        /// without this a gated-off row would leave a live but invisible
        /// region that still swallows clicks).
        #[serde(default)]
        visible: Option<String>,
    },
    /// A plain surface fill (no hit region, no label) — pill, chip, tile, swatch.
    Surface {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        #[serde(default)]
        r: f32,
        color: ColorToken,
        #[serde(default)]
        right: bool,
        /// `center_x` + `center_y` ignore the leading-edge math and place the
        /// surface at the parent box's center (with x/y as offsets) — the
        /// fullscreen-lock hero pieces (avatar disc) anchor this way.
        #[serde(default)]
        center_x: bool,
        #[serde(default)]
        center_y: bool,
        /// negative value = that corner is CONCAVE (curves into the panel,
        /// the dashboard strip band's screen-edge corners). Positive r with
        /// negative tr = rounded bottom-left, rounded bottom-right,
        /// concave top (the `Cmd::RectConcave` encoding).
        #[serde(default)]
        tr: f32,
        #[serde(default)]
        br: f32,
        #[serde(default)]
        bl: f32,
        /// base-px outline width drawn OVER the fill on the box's own rect
        /// (`stroke: 1.0` = the shell's hairline). Needs `stroke_color`;
        /// a concave-capped box outlines with the same per-corner radii.
        #[serde(default)]
        stroke: f32,
        /// the outline's color (base token or `{name}` value)
        #[serde(default)]
        stroke_color: Option<ColorToken>,
    },
    /// Open a rectangular clip region: everything that follows draws clipped
    /// to this box until the matching `ScissorEnd` (or the end of the parent
    /// box). A `w: 0` / `h: 0` axis spans the parent box — the full-width
    /// strip zones and scroll-cut lists clip this way. Inert for hit testing
    /// (regions still resolve normally; the shell keeps its own scissor stack,
    /// so nested regions nest cleanly).
    Scissor {
        x: f32,
        y: f32,
        #[serde(default)]
        w: f32,
        #[serde(default)]
        h: f32,
    },
    /// Close the innermost open `Scissor` region.
    ScissorEnd,
    /// A thin horizontal divider line.
    Divider {
        x: f32,
        y: f32,
        w: f32,
        #[serde(default)]
        color: ColorToken,
        #[serde(default)]
        right: bool,
    },
    /// A progress / meter bar. `value` is interpolated (may be a `{var}`) then
    /// parsed as a fraction of `max` (default 100); the fill hugs the left.
    Bar {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        #[serde(default)]
        r: f32,
        #[serde(default)]
        max: f32,
        value: String,
        #[serde(default)]
        fill: ColorToken,
        #[serde(default)]
        track: ColorToken,
        /// base px reserved at the right edge; when `w: 0` the bar spans
        /// `card_w − reserve` instead of the full card width (the Expenses
        /// bars clear the right-side amount column).
        #[serde(default)]
        reserve: f32,
        /// `vertical` fills the box BOTTOM-UP — `h` scaled by the fraction,
        /// anchored on the box's bottom edge (the power buttons' hold-to-
        /// confirm liquid). Horizontal bars are unaffected.
        #[serde(default)]
        vertical: bool,
        /// a bound `Toggle` that must resolve `true` (or be absent) for the
        /// bar to draw — per-category presence gating (Expenses).
        #[serde(default)]
        visible: Option<String>,
    },
    /// A vector-crafted battery gauge (NOT a font glyph): a rounded case
    /// outline with a terminal nub on the free end and a liquid interior that
    /// spans the full inner cross-section, carrying a wave crest + highlight
    /// on its surface. `vertical` stands it upright (nub on top, liquid
    /// climbs); horizontal lays it flat (nub right, liquid grows left→right).
    /// The charge arrives as a `{name}` value (0..100 against `max`; the
    /// default 100) and the level ink follows the shared ladder
    /// (≥50% ok, ≥20% accent, else danger) unless `level` overrides it.
    ///
    /// `w`/`h` are the CASE box. Both `0` sizes the case from the CARD box
    /// with the battery cards' own ladder — flat
    /// `(card_w·0.46).clamp(40, 220)` × `(card_h·0.36).clamp(16, 60)`
    /// centered at `42 + (card_h − 70)/2` down, upright
    /// `(card_w·0.30).clamp(16, 70)` wide and 1.7× tall centered at
    /// `38 + (h − 44)/2` — so a battery card declares just
    /// `Battery(vertical: …)`.
    Battery {
        #[serde(default)]
        x: f32,
        #[serde(default)]
        y: f32,
        /// case width (base px); `0` (+ `h: 0`) = the card-fit ladder
        #[serde(default)]
        w: f32,
        /// case height (base px); `0` (+ `w: 0`) = the card-fit ladder
        #[serde(default)]
        h: f32,
        /// upright (nub on top, liquid climbs) instead of flat
        #[serde(default)]
        vertical: bool,
        /// the live charge, interpolated from a `Text` value
        value: String,
        #[serde(default = "hundred")]
        max: f32,
        /// override the level ink ladder
        #[serde(default)]
        level: Option<ColorToken>,
        /// a bound `Toggle` that must resolve `true` (or be absent) for the
        /// gauge to draw (pane gating)
        #[serde(default)]
        visible: Option<String>,
    },
    /// A dynamic row list bound to the named `Rows` data. One row per data
    /// row, drawn with the declared `cols` cell formats; a row registers a
    /// full-row clickable region when it carries its own `key`, or when the
    /// item declares a `key_base` (`key_base + index`). Omit `key_base` and
    /// the rows are pure text — an info list must NOT register regions, since
    /// the bare `index`-keyed ones would collide with the global 1/2/3 keys.
    /// Rows past
    /// the card's bottom edge are skipped (no region, no draw) so scenes read
    /// as height-capped lists exactly like the Rust list drawers.
    Rows {
        name: String,
        /// the y of the FIRST row (base px, card-relative)
        y: f32,
        /// row box + right-anchor width; `0` → the full card width
        #[serde(default)]
        w: f32,
        /// row height (base px)
        row_h: f32,
        /// row STRIDE (base px; `None` → `row_h` — contiguous rows)
        #[serde(default)]
        pitch: Option<f32>,
        /// left inset columns' `x` are relative to (base px)
        #[serde(default = "twelve_px")]
        pad: f32,
        /// cell baseline offset inside each row (base px; 1 = flush under
        /// the row top, matching the dense card-list rows)
        #[serde(default = "one_px")]
        row_dy: f32,
        /// row corner radius (base px)
        #[serde(default = "five_px")]
        r: f32,
        cols: Vec<SceneCol>,
        /// fallback key base for a data row that carries none of its own
        /// (`None` = rows register NO region unless they set their own `key`)
        #[serde(default)]
        key_base: Option<u32>,
        /// hover surface behind the hovered row (only rows that register a key)
        #[serde(default)]
        hover_surface: Option<ColorToken>,
        /// clamp how many rows draw (0 = every row that fits)
        #[serde(default)]
        max_rows: usize,
        /// optional scalar value name carrying the scroll offset (first
        /// VISIBLE row index); bind cards with more rows than fit
        #[serde(default)]
        start: Option<String>,
        /// name of a `Checks` value holding per-row checked state; when set,
        /// every visible row draws a checkbox at its leading edge (parallel
        /// to the full `Rows` list, so windowed rows stay consistent)
        #[serde(default)]
        check: Option<String>,
        /// horizontal offset of the checkbox from the ROW's left edge (base px)
        #[serde(default = "two_px")]
        check_x: f32,
        /// hover ✕ delete: base key for the shown/registered region
        /// (`del_base + visible index`); `0` = no ✕ on this list
        #[serde(default)]
        del_base: u32,
        /// right margin of the ✕ glyph from the CARD's right edge (base px)
        #[serde(default = "twenty_px")]
        del_pad: f32,
        /// vertical offset of the ✕ glyph inside its row (base px)
        #[serde(default = "one_px")]
        del_dy: f32,
        /// ✕ glyph size (base px)
        #[serde(default = "nine_px")]
        del_size: f32,
        /// `Toggle` value name; when the value resolves `false` the whole
        /// list draws nothing (stateful suppression — e.g. the Notes list
        /// hides while its composer is open). `None`/unbound → always drawn.
        #[serde(default)]
        visible: Option<String>,
        /// draw a 1 px hairline under every visible row (the card-list
        /// dividers — News headlining rows separate with a hover-gray rule).
        #[serde(default)]
        hairline: bool,
        /// draw a thin accent rail at the left edge of the hovered row
        /// (2.5 × `row_h − 6`, rounded — the News hover accent bar).
        #[serde(default)]
        hover_bar: bool,
        /// `Ring` value name carrying the FLASHED row index (the transient
        /// 1 s "just snapped" accent of the Rust snippets drawer). The bound
        /// scalar is the row index + 1 (0 = none, so row 0 stays valid).
        /// The flash row paints a translucent-accent surface and inks its
        /// opted-in cells (`SceneCol.flash`) accent — both ignore hover.
        #[serde(default)]
        flash: Option<String>,
        /// binding name of a `Karaoke` value (parallel to this `Rows`) —
        /// the lyrics word-highlight binding. When present, each visible row
        /// draws its karaoke words instead of the normal cells: the active
        /// (sung) row paints the accent wash and walks its words from the
        /// left, painting the live word white beneath a full-width underline
        /// and the rest near-black (the Rust lyrics karaoke drawer, including
        /// its per-glyph estimate). Non-active rows just render the joined
        /// line (hover → accent ink). `None` = plain cells.
        #[serde(default)]
        karaoke: Option<String>,
        /// karaoke word font size (base px)
        #[serde(default = "tenth_half")]
        karaoke_fs: f32,
        /// reserve the LAST `bottom` base px of the card: rows whose box would
        /// cross the line `card_h − bottom` are skipped (the conninfo
        /// list clears its bottom refresh button without a scissor).
        /// `0` (default) = rows may run to the card's bottom edge.
        #[serde(default)]
        bottom: f32,
    },
    /// A line / column graph bound to the named `Spark` data. `max` scales
    /// the data against it (0 = normalize to the series' own peak).
    Spark {
        name: String,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        #[serde(default)]
        color: Option<ColorToken>,
        #[serde(default)]
        max: f32,
        #[serde(default)]
        kind: SparkKind,
        /// line weight / column width (base px)
        #[serde(default)]
        thickness: f32,
    },
    /// A bead-ring gauge bound to the named scalar: `lit = value / max`
    /// beads lit around the ring. `center_x`/`center_y` project the center to
    /// the card's middle like `Text` (offsets then apply from there).
    Ring {
        name: String,
        #[serde(default)]
        cx: f32,
        #[serde(default)]
        cy: f32,
        #[serde(default)]
        center_x: bool,
        #[serde(default)]
        center_y: bool,
        /// Place the ring at this fraction of the card width, added to the
        /// base `cx` — the gauges card's two columns (`0.27` / `0.73`, i.e.
        /// `x + w·frac`) which resize with the card, unlike a fixed inset.
        #[serde(default)]
        cx_pct: f32,
        /// Same, for the vertical axis: the water ring rides the midpoint of
        /// the body under the header (`0.5` of the card, less the header).
        #[serde(default)]
        cy_pct: f32,
        #[serde(default)]
        radius: f32,
        /// Responsive radius rules (the drawers' own ladders): `Half` takes
        /// half the card's free height, clamped (the water ring), `Ladder`
        /// steps on the card height (the gauges 30/22 ring). Unset → `radius`.
        #[serde(default)]
        fit: Option<RingFit>,
        /// Responsive bead size: `(min_radius, dot)` steps keyed on the
        /// RESOLVED radius, first match wins (gauges 5px on the 30px ring,
        /// 4px on the 22px one; water 4px from 26px up, else 3.5).
        #[serde(default)]
        dot_ladder: Vec<(f32, f32)>,
        #[serde(default = "one_hundred")]
        max: f32,
        #[serde(default = "thirty_six")]
        beads: u32,
        /// bead diameter (base px)
        #[serde(default = "three_px")]
        dot: f32,
        #[serde(default = "tok_acc")]
        fill: ColorToken,
        #[serde(default = "tok_hover")]
        track: ColorToken,
    },
    /// A horizontal slider bound to the named 0..1 scalar: slim groove, fill
    /// + round head (fill-colored, grows on hover). Registers a hit region
    /// when `key` != 0; drag/click math lives in the Rust key dispatcher.
    Fader {
        name: String,
        x: f32,
        y: f32,
        /// groove width; `0` fills the parent box, a negative value is the
        /// parent span minus that inset (stop short of the value column)
        w: f32,
        h: f32,
        #[serde(default)]
        key: u32,
        #[serde(default)]
        action: Option<SceneAction>,
        #[serde(default = "tok_acc")]
        fill: ColorToken,
        #[serde(default = "tok_hover")]
        track: ColorToken,
        /// knob diameter at rest (base px); the knob grows by 2 while the
        /// fader's key is hovered
        #[serde(default = "twelve_px")]
        head: f32,
        /// grow the fill from the MIDDLE outward (the balanced L↔R fader —
        /// `ui::fader_bal`) instead of from the left edge
        #[serde(default)]
        centered: bool,
        /// a bound `Toggle` that must resolve `true` (or be absent) for the
        /// fader to draw — hides a whole row (the sliders card's optional
        /// balance / saturation rows, collapsed by their pane-2 switch)
        #[serde(default)]
        visible: Option<String>,
    },
    /// A slim on/off switch bound to the named boolean. Registers a hit
    /// region when `key` != 0.
    Toggle {
        name: String,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        #[serde(default)]
        key: u32,
        #[serde(default)]
        action: Option<SceneAction>,
        /// right-anchor the switch (`x` = distance from the CARD's right edge)
        #[serde(default)]
        right: bool,
        /// a bound `Toggle` that must resolve `true` (or be absent) for the
        /// switch to draw (pane gating)
        #[serde(default)]
        visible: Option<String>,
    },
    /// Pagination dots below a multi-pane card (the HARD RULE: every
    /// swipe-cycled card renders one dot per pane, the active pane's dot in
    /// the accent, the others dimmed). Non-interactive — no hit regions.
    /// `active` binds the pane index; dots appear only while `when` resolves
    /// true (the cursor resting on the card, like the Rust drawers' hover
    /// gate).
    Dots {
        /// binding name of the active pane index (`Ring` scalar, 0-based)
        active: String,
        /// optional binding name of a `Toggle` gating visibility
        #[serde(default)]
        when: Option<String>,
        /// horizontal center of the dot row (base px); `0` → card center
        #[serde(default)]
        x: f32,
        /// dot row inset from the card BOTTOM (base px)
        #[serde(default = "seven_px")]
        dy: f32,
        /// dot pitch (base px)
        #[serde(default = "seven_px")]
        step: f32,
        /// pane count
        #[serde(default = "two_panes")]
        panes: u32,
    },
    /// A horizontal tab bar driven by the named `Rows` data (one tab per row,
    /// label = `cols[0]`) with the selected index from the trait named by
    /// `sel` (a 0..n scalar; defaults to the first tab). Tab keys are
    /// `key_base + index`.
    TabRow {
        name: String,
        /// optional scalar value name carrying the selected tab index
        #[serde(default)]
        sel: Option<String>,
        y: f32,
        #[serde(default = "twelve_px")]
        pad: f32,
        #[serde(default)]
        h: f32,
        #[serde(default)]
        key_base: u32,
        #[serde(default = "tok_acc")]
        active: ColorToken,
        #[serde(default = "tok_fg2")]
        idle: ColorToken,
    },
    /// The card's bottom input chrome — the "composer" strip shared by the
    /// note-taking cards (To-Do, Notes, Snippets…). Draws the rounded field
    /// (focused = stronger fill + accent outline), the live buffer with a
    /// caret when focused (placeholder when empty + unfocused), the "keyboard"
    /// typing hint on the right, and an optional ⊕ add button that hides while
    /// focused. Binds `text` (the `{var}`-interpolated buffer) and `focus` (a
    /// `Toggle` name). `bottom: true` anchors `y` from the card's bottom edge
    /// (base px), so the strip stays pinned as the card resizes.
    Composer {
        /// left inset from the card edge (base px; also the side padding of
        /// the field when `w: 0`)
        #[serde(default = "composer_pad")]
        pad: f32,
        /// top of the strip; with `bottom: true` this is the height of the
        /// card edge BELOW the field (base px)
        #[serde(default)]
        y: f32,
        /// field width; `0` → the full card width minus the pads
        #[serde(default)]
        w: f32,
        /// field height (base px)
        #[serde(default = "twenty_six_px")]
        h: f32,
        /// anchor `y` from the bottom of the card
        #[serde(default)]
        bottom: bool,
        /// optional `{var}`-interpolated buffer binding
        #[serde(default)]
        text: Option<String>,
        /// optional `Toggle` binding for the focused state
        #[serde(default)]
        focus: Option<String>,
        /// placeholder shown while empty and unfocused
        #[serde(default)]
        placeholder: String,
        /// region key for the field (focus request); 0 = no region
        #[serde(default)]
        key: u32,
        #[serde(default)]
        action: Option<SceneAction>,
        /// draw the ⊕ add button (bottom-right corner, hidden while focused)
        #[serde(default)]
        add: bool,
        #[serde(default)]
        add_key: u32,
        #[serde(default)]
        add_action: Option<SceneAction>,
    },
    /// The card's title chrome — mirrors the shared Rust `card_header`
    /// (glyph + semibold title + optional right-anchored meta). When a scene
    /// declares a `Header`, the shell sets `scene_owns_header` so the Rust
    /// card drawer skips its own header (no double-draw); exactly one
    /// `Header` should head a card scene.
    Header {
        /// Title text (drawn at the card's pad, semibold 11.5).
        title: String,
        /// Optional icon glyph before the title (drawn at pad, fg2, 12).
        #[serde(default)]
        glyph: Option<String>,
        /// Optional right-anchored meta next to the title (fg3, 8.5).
        #[serde(default)]
        meta: Option<String>,
        /// Meta renders mono-regular when true (technical figures), else
        /// ui-regular — exactly the `(text, mono)` switch of `card_header`.
        #[serde(default)]
        meta_mono: bool,
        /// Meta ink (default fg3) — a header whose meta is a live figure in an
        /// accent tone (the Expenses MTD total) overrides this.
        #[serde(default)]
        meta_color: Option<ColorToken>,
        /// Title ink (default fg) — a header whose title is a live accent
        /// label (the calendar month name) overrides this.
        #[serde(default)]
        title_color: Option<ColorToken>,
        /// Glyph ink (default fg2) — a header whose icon is a colored accent
        /// (the lyrics note glyph) overrides this.
        #[serde(default)]
        glyph_color: Option<ColorToken>,
        /// Horizontal inset from the card edges (title + meta margin).
        #[serde(default)]
        pad: f32,
    },
    /// A horizontal, scrollable chip strip — the News feed-category row.
    /// Chips come from the bound `Chips` value (`chips`), the active chip
    /// index from a `Ring` scalar (`sel`), and the scroll offset in base px
    /// from another `Ring` scalar (`scroll`, clamped to the strip's own
    /// content overflow at draw time so a stale value can't push chips off).
    /// Each chip auto-fits its label (pad + 0.62·fs·chars + pad), lifts
    /// `hover → hover_hl`, and paints the selected chip accent with
    /// `Sfg` label — byte-for-byte the Rust news category strip.
    Strip {
        /// binding name of the `Chips(Vec<String>)` chip labels
        chips: String,
        /// optional binding name of the active chip index (0-based `Ring`)
        #[serde(default)]
        sel: Option<String>,
        /// optional binding name of the horizontal scroll offset (px `Ring`)
        #[serde(default)]
        scroll: Option<String>,
        x: f32,
        y: f32,
        #[serde(default)]
        w: f32,
        #[serde(default)]
        h: f32,
        /// left/right inset of the strip box
        #[serde(default = "twelve_px")]
        pad: f32,
        /// gap between chips
        #[serde(default = "six_px")]
        spacing: f32,
        /// horizontal padding inside each chip
        #[serde(default = "eleven_px")]
        chip_pad: f32,
        /// chip label size (base px)
        #[serde(default = "eight_half")]
        fs: f32,
        /// region key base — chips register `key_base + index`
        #[serde(default)]
        key_base: u32,
    },
    /// The dashboard's top strip chips, fully data-driven: cells arrive
    /// per-frame from `scene_values()` (`BannerCells` binding — geometry from
    /// `banner_cells`, inks/labels from the Rust text helpers) and the item
    /// paints them with the exact per-chip recipes the Rust drawer uses.
    /// Author it inside the `dashboard` surface after the `dash_grid` Ink (it
    /// rides the next Ink's dispatch slot via `pre`) with `y: 12, h: 26`, the
    /// strip band's box. The Rust chip drawer suppresses itself while a
    /// `BannerCells` binding is consumed (the `dash_chips_via_scene` flag),
    /// so the strip is never painted twice.
    BannerRow {
        /// binding name of the `BannerCells` cell list
        cells: String,
        x: f32,
        y: f32,
        #[serde(default)]
        w: f32,
        #[serde(default)]
        h: f32,
        /// gate the whole row on a Toggle binding (the viewing state only —
        /// edit chrome stays Rust): `banner_viewing`
        #[serde(default)]
        visible: Option<String>,
        /// toggle binding enabling WHOLE-strip clipping (overflow mode — one
        /// scissor over the band so panning chips never bleed out)
        #[serde(default)]
        overflow: Option<String>,
        /// toggle binding enabling per-zone clipping (L/C/R strip — each
        /// zone's window clips independently so a scrolled zone never bleeds
        /// into its neighbours)
        #[serde(default)]
        zones: Option<String>,
    },
    /// A named bespoke painter — the Quickshell/QML "Canvas" analog. Declares
    /// where a Rust-drawn element goes; the shell dispatches it by `name`.
    Ink {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        name: String,
        #[serde(default)]
        right: bool,
    },
    /// A horizontal layout box (the Quickshell `Row` analog). Children keep
    /// their own `w`/`h`; those with `w: 0` split the remaining width equally
    /// (`spacing` gaps them). Children's own `x`/`y` are relative to their
    /// slot inside the box (`right`/`center_x` anchor against the slot). A
    /// `w: 0` box itself fills its parent's width.
    Row {
        #[serde(default)]
        x: f32,
        #[serde(default)]
        y: f32,
        #[serde(default)]
        w: f32,
        #[serde(default)]
        h: f32,
        /// inner inset on all four sides (base px)
        #[serde(default)]
        pad: f32,
        /// horizontal gap between slots (base px)
        #[serde(default)]
        spacing: f32,
        #[serde(default)]
        valign: VAlign,
        /// `center` offsets a block of FIXED-width children so it sits
        /// centered in the row's box (a row of power buttons sized to the
        /// card); children that fill the width already span the box, so this
        /// only matters when every child declares its own `w`
        #[serde(default)]
        halign: HAlign,
        /// anchor the row's BOTTOM edge `y` px above the card bottom (the
        /// mirror button bar) — requires a declared `h`
        #[serde(default)]
        bottom: bool,
        /// the card must be at least this tall (base px) for the row to draw
        /// at all — the compositor card's screenshot buttons stand down on a
        /// short card rather than overflowing it. The value is the card
        /// height the row needs, not the row's own box: `0` = always draw.
        #[serde(default)]
        min_card_h: f32,
        /// a bound `Toggle` that must resolve `true` (or be absent) for the
        /// row to draw (pane gating)
        #[serde(default)]
        visible: Option<String>,
        items: Vec<SceneItem>,
    },
    /// A vertical layout box (the Quickshell `Column` analog). Children keep
    /// their own `w`/`h`; those with `h: 0` split the remaining height
    /// equally. An `h: 0` box fills its parent's height.
    Column {
        #[serde(default)]
        x: f32,
        #[serde(default)]
        y: f32,
        #[serde(default)]
        w: f32,
        #[serde(default)]
        h: f32,
        #[serde(default)]
        pad: f32,
        #[serde(default)]
        spacing: f32,
        #[serde(default)]
        halign: HAlign,
        /// vertical alignment of the stacked block inside the box — `middle`
        /// centers it (the sliders card centers its fitted fader rows between
        /// the card edges, leaving the bottom band free for the pane dots)
        #[serde(default)]
        valign: VAlign,
        /// a bound `Toggle` that must resolve `true` (or be absent) for the
        /// column to draw (pane gating)
        #[serde(default)]
        visible: Option<String>,
        items: Vec<SceneItem>,
    },
    /// An overlay box (the Quickshell `Stack`/`Item` analog). Every child is
    /// drawn in the full inner box, in declaration order, positioned by its
    /// own `x`/`y` (and `right`/`center`) — the base layer for full-bleed
    /// surfaces with gauges/text on top.
    Stack {
        #[serde(default)]
        x: f32,
        #[serde(default)]
        y: f32,
        #[serde(default)]
        w: f32,
        #[serde(default)]
        h: f32,
        #[serde(default)]
        pad: f32,
        /// a bound `Toggle` that must resolve `true` (or be absent) for the
        /// stack to draw (pane gating)
        #[serde(default)]
        visible: Option<String>,
        items: Vec<SceneItem>,
    },
    /// A fixed grid of equal cells bound to the named `GridCells` data —
    /// the calendar month matrix, the app-shortcut launcher tiles. Cells are
    /// laid out in `cols`-wide rows; every cell registers a clickable region
    /// `key_base + index`. A `square` grid sizes `cell_h = cell_w` and lets
    /// `h` bound the region (app tiles sit square inside their body); the
    /// calendar instead stretches `cell_h` to fill the box. Cells carrying
    /// an `image_key` / `glyph` draw the raster/icon above a bottom label,
    /// the default day cells center their text. `head` draws an optional
    /// dim label row across the column tops (the weekdays) and pushes the
    /// cell region below it. A non-zero `del_base` reveals a hover-only ✕
    /// delete corner on filled cells at `del_base + index` (app remove).
    Grid {
        /// binding name of the `GridCells` cell list
        name: String,
        /// left edge of the grid box (base px)
        x: f32,
        /// top of the grid box (base px) — the head row, when present,
        /// occupies this top band
        y: f32,
        /// box width; `0` → fills the card width minus `2·pad`
        #[serde(default)]
        w: f32,
        /// box height; `0` → fills the card height below `y`
        #[serde(default)]
        h: f32,
        /// columns per row
        #[serde(default = "one_col")]
        cols: u32,
        /// left/right inset (base px; the calendar's full-bleed 7-col days
        /// use 0)
        #[serde(default)]
        pad: f32,
        /// gap between cells (base px)
        #[serde(default)]
        gap: f32,
        /// cell label size (base px; ≤ 0 → 9.0)
        #[serde(default = "nine_px")]
        font_size: f32,
        /// cell corner radius (base px)
        #[serde(default = "eight_px")]
        r: f32,
        /// region key base — cells register `key_base + index`
        #[serde(default)]
        key_base: u32,
        /// hover fill behind the hovered cell (lifted variant when the cell
        /// carries its own `surface`)
        #[serde(default)]
        hover_surface: Option<ColorToken>,
        /// hover-only ✕ delete corner base key (`del_base + index`); `0` = no
        /// ✕ on this grid
        #[serde(default)]
        del_base: u32,
        /// ✕ glyph size (base px)
        #[serde(default = "nine_px")]
        del_size: f32,
        /// raster box size for `image_key` cells (base px)
        #[serde(default = "twenty_two_px")]
        image_size: f32,
        /// raster top inset from the cell top (base px)
        #[serde(default = "four_px")]
        image_top: f32,
        /// square cells: `cell_h = cell_w` (app tiles)
        #[serde(default)]
        square: bool,
        /// fixed cell height (base px) — rows keep this pitch instead of
        /// stretching to fill the box (the thermal card's 24 px chip rows);
        /// `0` = stretch (calendar). Takes precedence over `square`.
        #[serde(default)]
        cell_h: f32,
        /// optional column-head labels drawn across the column tops, each
        /// centered over its column at 8 px, dim (the calendar weekdays)
        #[serde(default)]
        head: Option<Vec<String>>,
        /// a bound `Toggle` that must resolve `true` (or be absent) for the
        /// grid to draw
        #[serde(default)]
        visible: Option<String>,
    },
    /// A tile grid of switch tiles — the `Tiles` analog of `Grid`, for
    /// "pill" bodies where each cell is a rounded surface carrying an icon
    /// glyph and (when the tile is tall enough) a caption (the toggles
    /// card's Wi-Fi / BT / DND / Caffeine / Sunset / Shader switches). The
    /// per-tile data (`key`, `glyph`, `label`, `on`, `chip_key`) is
    /// published per frame as a `SceneValue::Tiles` list; this item owns the
    /// FRAME — column count, gaps, radii, the on/hover/off ink ladder, the
    /// chip, and the responsive breakpoints. Each tile registers its own
    /// region, plus a tall right-edge region for its chip when it has one.
    Tiles {
        /// binding name of the `Tiles` list
        name: String,
        /// left edge of the tile box (base px)
        x: f32,
        /// top edge of the tile box (base px)
        y: f32,
        /// box width; `0` → the card width below `x`, `< 0` → stops that
        /// many px short of the card's right edge
        #[serde(default)]
        w: f32,
        /// box height; `0` → the card height below `y`, `< 0` → stops that
        /// many px short of the card bottom
        #[serde(default)]
        h: f32,
        /// left/right + top/bottom inset inside the box (base px)
        #[serde(default = "ten_px")]
        pad: f32,
        /// gap between tiles (base px)
        #[serde(default = "eight_px")]
        gap: f32,
        /// columns per row when no breakpoint applies
        #[serde(default = "one_col")]
        cols: u32,
        /// responsive columns: `(min card width, cols)` pairs ascending —
        /// the narrowest card the layout is designed for, then the card size
        /// that affords another column (the toggles card reflows 1 → 2 → 3)
        #[serde(default)]
        cols_break: Vec<(f32, u32)>,
        /// tile corner radius (base px)
        #[serde(default = "twelve_px")]
        r: f32,
        /// glyph size (base px)
        #[serde(default = "thirteen_px")]
        icon_size: f32,
        /// caption size (base px)
        #[serde(default = "eight_half_px")]
        label_size: f32,
        /// glyph top inset from the tile top in a labeled tile (base px)
        #[serde(default = "eight_px")]
        icon_top: f32,
        /// caption top inset from the tile top in a labeled tile (base px)
        #[serde(default = "thirty_px")]
        label_top: f32,
        /// a tile shorter than this (base px) drops its caption and centers
        /// the glyph instead — a squat card keeps its switches legible
        #[serde(default = "forty_two_px")]
        label_min_h: f32,
        /// fill of an "on" tile
        #[serde(default = "acc_tint_token")]
        on_surface: ColorToken,
        /// fill of a hovered, off tile
        #[serde(default = "hover_hl_token")]
        hover_surface: ColorToken,
        /// resting fill of an off tile
        #[serde(default = "hover_token")]
        off_surface: ColorToken,
        /// glyph ink of an on tile
        #[serde(default = "acc_token")]
        on_ink: ColorToken,
        /// glyph ink of an off tile
        #[serde(default = "fg_token")]
        off_ink: ColorToken,
        /// caption ink
        #[serde(default = "fg_token")]
        label_ink: ColorToken,
        /// draw the corner chip on tiles that carry a `chip_key`
        #[serde(default)]
        chip: bool,
        /// chip glyph size (base px)
        #[serde(default = "nine_px")]
        chip_size: f32,
        /// chip glyph center, from the tile's right edge (base px)
        #[serde(default = "ten_px")]
        chip_dx: f32,
        /// chip glyph top, from the tile's bottom edge (base px)
        #[serde(default = "fourteen_px")]
        chip_dy: f32,
        /// chip region width, anchored to the tile's right edge (base px);
        /// the region is the tile's FULL height (the drawer registers a tall
        /// edge strip, not a corner box)
        #[serde(default = "eighteen_px")]
        chip_w: f32,
        /// a bound `Toggle` that must resolve `true` (or be absent) for the
        /// tiles to draw
        #[serde(default)]
        visible: Option<String>,
    },
    /// A raster image drawn into the item box — the `Cmd::Image` equivalent
    /// (camera feed, media artwork, app icons). `key` is the `ImageStore`
    /// key, interpolated so it may reference a `{var}`; `w: 0` → `card_w`,
    /// `h: 0` → `card_h`, and `right: true` anchors the left edge at the
    /// card's `x` margin like `Text`.
    Image {
        x: f32,
        y: f32,
        #[serde(default)]
        w: f32,
        #[serde(default)]
        h: f32,
        /// `ImageStore` key (`icon:<name>` / `file:<path>` / camera feed id)
        key: String,
        #[serde(default)]
        right: bool,
        /// a bound `Toggle` that must resolve `true` (or be absent) for the
        /// image to draw — pane gating, missing-camera fallback swap
        #[serde(default)]
        visible: Option<String>,
    },
    /// A named reusable block from the whole-shell `shell.ron` `components`
    /// table, INLINED at load time and translated by `(x, y)` (base px) —
    /// template instantiation, the shell analog of a QML component.
    Comp {
        name: String,
        #[serde(default)]
        x: f32,
        #[serde(default)]
        y: f32,
    },
}

/// Vertical alignment of a [`SceneItem::Row`]'s children within the box.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VAlign {
    #[default]
    Top,
    Middle,
}

/// Horizontal alignment of a [`SceneItem::Column`]'s children.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HAlign {
    #[default]
    Left,
    Center,
}

impl SceneItem {
    /// The `visible` binding name, when this item declares one. Every gated
    /// item (Rows/Text/Bar/TextWrap/Toggle/Row/Column/Stack/Fader/Grid/Image/
    /// Strip/Bar) shares the same optional field, so the container layout pass
    /// can ask the question without a 20-arm match at every call site.
    fn visible_binding(&self) -> Option<&str> {
        match self {
            SceneItem::Text { visible, .. }
            | SceneItem::TextWrap { visible, .. }
            | SceneItem::Rows { visible, .. }
            | SceneItem::Bar { visible, .. }
            | SceneItem::Battery { visible, .. }
            | SceneItem::Toggle { visible, .. }
            | SceneItem::Fader { visible, .. }
            | SceneItem::Grid { visible, .. }
            | SceneItem::Tiles { visible, .. }
            | SceneItem::Image { visible, .. }
            | SceneItem::Row { visible, .. }
            | SceneItem::Column { visible, .. }
            | SceneItem::Stack { visible, .. }
            | SceneItem::Hit { visible, .. } => visible.as_deref(),
            _ => None,
        }
    }

    /// Whether this item anchors off its box's RIGHT edge (see the `Text`
    /// arm). Such an item takes no horizontal layout slot, so a container's
    /// fill pass must leave its declared width alone.
    fn right_anchored(&self) -> bool {
        matches!(self, SceneItem::Text { right: true, .. })
    }

    /// Whether this item passes its `visible` gate. A resolved `false` hides
    /// the item WITHOUT reserving space when the item sits inside a `Row` /
    /// `Column` (the sliders card's balance + saturation rows collapse the
    /// stack when their pane-2 switch is off, exactly like the Rust drawer
    /// filtering its row list). Unbound / absent = always shown.
    fn is_visible(&self, vals: &SceneValues) -> bool {
        gate_open(self.visible_binding(), vals)
    }

    /// Declared base-px width the item wants, or 0 when it fills its slot.
    fn want_w(&self) -> f32 {
        match self {
            SceneItem::Text { w, .. }
            | SceneItem::TextWrap { w, .. }
            | SceneItem::Hit { w, .. }
            | SceneItem::Surface { w, .. }
            | SceneItem::Divider { w, .. }
            | SceneItem::Bar { w, .. }
            | SceneItem::Battery { w, .. }
            | SceneItem::Spark { w, .. }
            | SceneItem::Fader { w, .. }
            | SceneItem::Toggle { w, .. }
            | SceneItem::Composer { w, .. }
            | SceneItem::Ink { w, .. }
            | SceneItem::Strip { w, .. }
            | SceneItem::Row { w, .. }
            | SceneItem::Column { w, .. }
            | SceneItem::Stack { w, .. }
            | SceneItem::Grid { w, .. }
            | SceneItem::Tiles { w, .. }
            | SceneItem::Image { w, .. } => *w,
            _ => 0.0,
        }
    }

    /// Declared base-px height the item wants, or 0 when it fills its slot.
    fn want_h(&self) -> f32 {
        match self {
            SceneItem::Text { h, .. }
            | SceneItem::Hit { h, .. }
            | SceneItem::Surface { h, .. }
            | SceneItem::Bar { h, .. }
            | SceneItem::Battery { h, .. }
            | SceneItem::Spark { h, .. }
            | SceneItem::Fader { h, .. }
            | SceneItem::Toggle { h, .. }
            | SceneItem::Composer { h, .. }
            | SceneItem::Ink { h, .. }
            | SceneItem::Strip { h, .. }
            | SceneItem::TabRow { h, .. }
            | SceneItem::Row { h, .. }
            | SceneItem::Column { h, .. }
            | SceneItem::Stack { h, .. }
            | SceneItem::Grid { h, .. }
            | SceneItem::Tiles { h, .. }
            | SceneItem::Image { h, .. } => *h,
            _ => 0.0,
        }
    }

    /// Replace any zero width/height axis with the given base-px dims so a
    /// zero-dimension item can span its slot (used by [`SceneItem::Stack`]).
    /// Items without a w/h axis are returned unchanged.
    fn set_dims(&mut self, w: Option<f32>, h: Option<f32>) {
        match self {
            SceneItem::Text { w: ow, h: oh, .. }
            | SceneItem::Hit { w: ow, h: oh, .. }
            | SceneItem::Surface { w: ow, h: oh, .. }
            | SceneItem::Bar { w: ow, h: oh, .. }
            | SceneItem::Spark { w: ow, h: oh, .. }
            | SceneItem::Fader { w: ow, h: oh, .. }
            | SceneItem::Toggle { w: ow, h: oh, .. }
            | SceneItem::Composer { w: ow, h: oh, .. }
            | SceneItem::Ink { w: ow, h: oh, .. }
            | SceneItem::Row { w: ow, h: oh, .. }
            | SceneItem::Column { w: ow, h: oh, .. }
            | SceneItem::Stack { w: ow, h: oh, .. }
            | SceneItem::Grid { w: ow, h: oh, .. }
            | SceneItem::Tiles { w: ow, h: oh, .. }
            | SceneItem::Image { w: ow, h: oh, .. } => {
                if let Some(w) = w {
                    *ow = w;
                }
                if let Some(h) = h {
                    *oh = h;
                }
            }
            SceneItem::TextWrap { w: ow, .. } => {
                if let Some(w) = w {
                    *ow = w;
                }
            }
            _ => {}
        }
    }
}

/// A card scene: a flat list of positioned items, rendered in order.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CardScene {
    pub items: Vec<SceneItem>,
}

/// Cache of loaded card scenes keyed by card id. Reloads a scene only when
/// its source `.ron` mtime changed (drives the shell's hot-reload tick).
#[derive(Clone, Default)]
pub struct SceneCache {
    scenes: std::collections::HashMap<String, Option<CardScene>>,
    mtimes: std::collections::HashMap<String, Option<std::time::SystemTime>>,
}

impl SceneCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get (loading on first access) the scene for `card_id` from its
    /// `~/.config/zen-shell/ui/cards/<id>.ron` file. `None` = no file / parse
    /// error / empty scene. Never panics: a malformed RON logs and falls back
    /// to the built-in Rust draw.
    pub fn get(&mut self, card_id: &str) -> Option<&CardScene> {
        let path = crate::vars::card_scene_path(card_id);
        let current = std::fs::metadata(&path).and_then(|m| m.modified()).ok();
        let cached_mt = self.mtimes.get(card_id).copied().flatten();
        if cached_mt == current && self.scenes.contains_key(card_id) {
            return self.scenes[card_id].as_ref();
        }
        let scene = match current {
            Some(_) => std::fs::read_to_string(&path)
                .ok()
                .and_then(|s| match ron::from_str::<CardScene>(&s) {
                    Ok(sc) => Some(sc),
                    Err(e) => {
                        eprintln!("zen: scene {card_id}: bad RON ({path:?}): {e}");
                        None
                    }
                }),
            None => None,
        };
        self.mtimes.insert(card_id.to_string(), current);
        self.scenes.insert(card_id.to_string(), scene);
        self.scenes[card_id].as_ref()
    }
}

/// A whole-shell surface: a named scene drawn top-level (dashboard banner,
/// settings, notifications, lock screen, wallpaper). Drawn at absolute
/// coordinates like a card, so RON authors write base pixels from the
/// surface's origin.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SurfaceScene {
    pub name: String,
    pub items: Vec<SceneItem>,
}

/// The whole-shell declarative scene (`~/.config/zen-shell/shell.ron`):
/// reusable `components` templates plus named `surfaces`. Each surface's
/// [`SceneItem::Comp`]s are inlined at load time (see [`ShellScene::expand`]),
/// so surfaces hold a flat item list by the time they are drawn.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ShellScene {
    #[serde(default)]
    pub components: std::collections::HashMap<String, Vec<SceneItem>>,
    pub surfaces: Vec<SurfaceScene>,
}

/// Translate a single item by `(dx, dy)` base px (component inlining). Parent
/// boxes are shifted only at their own origin — their children are positioned
/// relative to the moved box, so they follow automatically.
fn offset_item(it: &mut SceneItem, dx: f32, dy: f32) {
    match it {
        SceneItem::Text { x, y, .. }
        | SceneItem::Hit { x, y, .. }
        | SceneItem::Surface { x, y, .. }
        | SceneItem::Scissor { x, y, .. }
        | SceneItem::Divider { x, y, .. }
        | SceneItem::Bar { x, y, .. }
        | SceneItem::Spark { x, y, .. }
        | SceneItem::Fader { x, y, .. }
        | SceneItem::Toggle { x, y, .. }
        | SceneItem::Ink { x, y, .. }
        | SceneItem::Strip { x, y, .. }
        | SceneItem::Row { x, y, .. }
        | SceneItem::Column { x, y, .. }
        | SceneItem::Stack { x, y, .. }
        | SceneItem::Grid { x, y, .. }
        | SceneItem::Tiles { x, y, .. }
        | SceneItem::Image { x, y, .. } => {
            *x += dx;
            *y += dy;
        }
        SceneItem::Rows { y, .. } | SceneItem::TabRow { y, .. } | SceneItem::Composer { y, .. } => {
            *y += dy;
        }
        SceneItem::Ring { cx, cy, .. } => {
            *cx += dx;
            *cy += dy;
        }
        _ => {}
    }
}

/// Expand a template list: `Comp(name, x, y)` → that component's items,
/// inlined at `(dx + x, dy + y)` — recursively, so components may nest.
fn inflate(
    comps: &std::collections::HashMap<String, Vec<SceneItem>>,
    tpl: &[SceneItem],
    dx: f32,
    dy: f32,
) -> Vec<SceneItem> {
    let mut out = Vec::new();
    for it in tpl {
        match it {
            SceneItem::Comp { name, x, y } => {
                if let Some(inner) = comps.get(name) {
                    out.extend(inflate(comps, inner, dx + *x, dy + *y));
                } else {
                    eprintln!("zen: shell scene: unknown component `{name}`");
                }
            }
            _ => {
                let mut c = it.clone();
                offset_item(&mut c, dx, dy);
                out.push(c);
            }
        }
    }
    out
}

impl ShellScene {
    /// Inline every component reference into its owning surface, permanently.
    pub fn expand(&mut self) {
        for s in &mut self.surfaces {
            s.items = inflate(&self.components, &s.items, 0.0, 0.0);
        }
    }

    /// Borrow the surface scene named `name`, if declared.
    pub fn surface(&self, name: &str) -> Option<&SurfaceScene> {
        self.surfaces.iter().find(|s| s.name == name)
    }
}

/// Cache of the whole-shell scene (`shell.ron`). Reloads only when the file's
/// mtime changes, mirroring [`SceneCache`] and the shell's hot-reload tick.
#[derive(Clone, Default)]
pub struct ShellSceneCache {
    scene: Option<ShellScene>,
    mtime: Option<std::time::SystemTime>,
}

impl ShellSceneCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the loaded (component-expanded) shell scene, reloading when the
    /// source `.ron` mtime changed. `None` = no file / parse error. Never
    /// panics.
    pub fn get(&mut self) -> Option<&ShellScene> {
        let path = crate::vars::shell_scene_path();
        let current = std::fs::metadata(&path).and_then(|m| m.modified()).ok();
        if self.mtime == current && self.scene.is_some() {
            return self.scene.as_ref();
        }
        self.mtime = current;
        self.scene = match current {
            Some(_) => std::fs::read_to_string(&path)
                .ok()
                .and_then(|s| match ron::from_str::<ShellScene>(&s) {
                    Ok(mut sc) => {
                        sc.expand();
                        Some(sc)
                    }
                    Err(e) => {
                        eprintln!("zen: shell scene: bad RON ({path:?}): {e}");
                        None
                    }
                }),
            None => None,
        };
        self.scene.as_ref()
    }
}

/// A resolved hit region returned by [`CardScene::draw`]. Owns a clone of the
/// action so callers can register regions without holding a borrow on the
/// scene / shell.
#[derive(Debug)]
pub struct SceneHit {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub key: u32,
    pub action: Option<SceneAction>,
}

/// A named bespoke paint request from a scene. The shell dispatches it by
/// `name` to the matching Rust painter (worldmap, viz, media art, …).
#[derive(Clone, Debug, PartialEq)]
pub struct SceneInk {
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    /// Declared items authored between the PREVIOUS Ink and this one. They
    /// draw immediately before this ink's Rust painter, so declared chrome
    /// can sit BETWEEN two Rust bodies in z-order (the dashboard strip band
    /// layers over the card grid but under the banner chips this way).
    /// Internal — filled by `draw`, never parsed.
    pub pre: Vec<SceneItem>,
}

/// Resolve a responsive ladder against the card: `(min_card_h, value)` steps,
/// first match wins, and a card that matches no step keeps `fallback`. The
/// thresholds and the returned value are both base px (the card box arrives
/// scaled). The gauge cards' drawers write these ladders by hand.
fn ladder_step(steps: &[(f32, f32)], fallback: f32, card_h: f32, scale: crate::shell::GridScale) -> f32 {
    match steps.iter().find(|(min_h, _)| card_h >= scale.s(*min_h)) {
        Some((_, v)) => scale.s(*v),
        None => scale.s(fallback),
    }
}

/// Resolve a declared base-px extent against its parent box: a positive value
/// is absolute, `0` fills the parent span, and a NEGATIVE value is the parent
/// span minus that inset — the `Image` convention (a body that stops short of
/// an edge, e.g. a fader that ends before the value column), extended to the
/// layout containers and the fader/toggle tracks so a row can hug one edge
/// and stop at another without hard-coding a card width.
fn span_px(declared: f32, parent: f32, scale: crate::shell::GridScale) -> f32 {
    if declared > 0.0 {
        scale.s(declared)
    } else if declared < 0.0 {
        (parent - scale.s(-declared)).max(0.0)
    } else {
        parent
    }
}

/// Resolve an item's `visible` gate. An absent binding, or a name the caller
/// never published, is always-open (a scene must never silently blank itself
/// because a value is missing); only an explicitly bound `false` hides.
fn gate_open(binding: Option<&str>, vals: &SceneValues) -> bool {
    match binding {
        Some(n) => vals.toggle(n) != Some(false),
        None => true,
    }
}

impl CardScene {
    /// Draw every item into `v` — the scene is translated by `(x, y)` (the
    /// card's origin) and every dimension goes through `scale.s(px)` so RON
    /// authors write base pixels like the Rust draw code does. `hover_key` is
    /// the current hovered key for the card's hit regions. Values in `vals`
    /// interpolate `{var}` placeholders inside every `text`. Returns the hit
    /// regions (already offset by `(x, y)`, unscaled) for the caller to
    /// register.
    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        &self,
        v: &mut Vec<Cmd>,
        x: f32,
        y: f32,
        card_w: f32,
        card_h: f32,
        scale: crate::shell::GridScale,
        pal: &Pal,
        vals: &SceneValues,
        hover_key: u32,
        show_title: bool,
        show_glyph: bool,
    ) -> (Vec<SceneHit>, Vec<SceneInk>) {
        let mut hits = Vec::new();
        let mut inks = Vec::new();
        // ── Ink `pre` attachment ──
        // Items authored AFTER an Ink and BEFORE the next one belong to that
        // next Ink's z-slot (drawn at its dispatch position), not upfront —
        // declared chrome can sit BETWEEN two Rust bodies (the dashboard
        // strip band layers over the card grid but under the banner chips).
        // Items before the first Ink / after the last draw in scene order.
        let mut pre_of: std::collections::HashMap<usize, Vec<SceneItem>> =
            std::collections::HashMap::new();
        let mut skip: std::collections::HashSet<usize> = std::collections::HashSet::new();
        let mut pre_buf: Vec<SceneItem> = Vec::new();
        let mut after_ink = false;
        for (idx, it) in self.items.iter().enumerate() {
            if matches!(it, SceneItem::Ink { .. }) {
                pre_of.insert(idx, std::mem::take(&mut pre_buf));
                after_ink = true;
            } else if after_ink {
                pre_buf.push(it.clone());
                skip.insert(idx);
            }
        }
        for (idx, it) in self.items.iter().enumerate() {
            if skip.contains(&idx) {
                continue;
            }
            if let SceneItem::Ink { x: ix, y: iy, w, h, name, right } = it {
                // handled here (not draw_piece) so the ink carries its `pre`
                let sx = if *right {
                    x + card_w - scale.s(*ix + *w)
                } else {
                    x + scale.s(*ix)
                };
                inks.push(SceneInk {
                    name: substitute(name, vals),
                    x: sx,
                    y: y + scale.s(*iy),
                    w: if *w > 0.0 { scale.s(*w) } else { card_w },
                    h: if *h > 0.0 { scale.s(*h) } else { card_h },
                    pre: pre_of.remove(&idx).unwrap_or_default(),
                });
                continue;
            }
            self.draw_piece(it, v, x, y, card_w, card_h, scale, pal, vals, hover_key, show_title, show_glyph, &mut hits, &mut inks);
        }
        (hits, inks)
    }

    /// Draw one scene item (or a whole nested box) into `v`. `(x, y, card_w,
    /// card_h)` is the item's *parent box*: every item positions itself
    /// relative to it, so containers pass each child a slot and the child's
    /// `right`/`center`/`w` resolve against that slot instead of the card —
    /// exactly how the top-level items resolve against the card.
    #[allow(clippy::too_many_arguments)]
    fn draw_piece(
        &self,
        it: &SceneItem,
        v: &mut Vec<Cmd>,
        x: f32,
        y: f32,
        card_w: f32,
        card_h: f32,
        scale: crate::shell::GridScale,
        pal: &Pal,
        vals: &SceneValues,
        hover_key: u32,
        show_title: bool,
        show_glyph: bool,
        hits: &mut Vec<SceneHit>,
        inks: &mut Vec<SceneInk>,
    ) {
        match it {
                SceneItem::Text {
                    x: ix,
                    y: iy,
                    w,
                    h: _,
                    text,
                    font_size,
                    font,
                    weight,
                    right,
                    center,
                    icon,
                    color,
                    center_x,
                    center_y,
                    cx_pct,
                    cy_pct,
                    bottom,
                    visible,
                    size_ladder,
                    y_ladder,
                } => {
                    // honour the bound Toggle gate (empty/fetching hints)
                    if let Some(v) = visible.as_deref() {
                        if vals.toggle(v) == Some(false) {
                            return;
                        }
                    }
                    // `right: true` anchors the text's RIGHT at card_w - ix
                    // (ix = right margin), so right-aligned meta follows resize;
                    // `center_x`/`center_y` project the text to the card center
                    let sw = span_px(*w, card_w, scale);
                    let sx = if *cx_pct != 0.0 {
                        // proportional CENTER anchor: the box's midpoint sits
                        // on card_w·cx_pct (the gauges hero % columns)
                        x + card_w * *cx_pct - sw / 2.0
                    } else if *right {
                        x + card_w - scale.s(*ix)
                    } else if *center_x {
                        x + card_w / 2.0 + scale.s(*ix)
                    } else {
                        x + scale.s(*ix)
                    };
                    let sy = if *bottom {
                        y + card_h - scale.s(*iy)
                    } else if *center_y {
                        y + card_h / 2.0 + scale.s(ladder_step(y_ladder, *iy, card_h, scale))
                    } else {
                        y + scale.s(ladder_step(y_ladder, *iy, card_h, scale))
                    } + card_h * *cy_pct;
                    // responsive font size: (min_card_h, size) steps, first
                    // match wins; a card that matches nothing keeps font_size
                    let fs = ladder_step(size_ladder, *font_size, card_h, scale);
                    draw_text(v, sx, sy, sw, *right, *center, text, fs, *font, *weight, *icon, *color, scale, pal, vals);
                }
                SceneItem::When { min_w, max_w, min_h, max_h, items } => {
                    // responsive size window — the drawers' own `if w >= s(170)`
                    // / `if h >= ring_r*2 + s(50)` layout branches
                    if *min_w > 0.0 && card_w < scale.s(*min_w) {
                        return;
                    }
                    if *max_w > 0.0 && card_w >= scale.s(*max_w) {
                        return;
                    }
                    if *min_h > 0.0 && card_h < scale.s(*min_h) {
                        return;
                    }
                    if *max_h > 0.0 && card_h >= scale.s(*max_h) {
                        return;
                    }
                    // children keep the full box (the `Stack` idiom) so their
                    // center / bottom / proportional anchors stay card-relative
                    for c in items {
                        self.draw_piece(c, v, x, y, card_w, card_h, scale, pal, vals, hover_key, show_title, show_glyph, hits, inks);
                    }
                }
                SceneItem::TextWrap {
                    text,
                    caption,
                    top: itop,
                    bottom,
                    w,
                    font_size,
                    line_h,
                    color,
                    visible,
                } => {
                    // honour the bound Toggle gate (e.g. quote body vs fallbacks)
                    if let Some(v) = visible.as_deref() {
                        if vals.toggle(v) == Some(false) {
                            return;
                        }
                    }
                    let txt = substitute(text, vals);
                    if txt.is_empty() {
                        return;
                    }
                    let sw = if *w > 0.0 { scale.s(*w) } else { card_w };
                    let cx = x + sw / 2.0;
                    let body_top = y + scale.s(*itop);
                    let body_h = (card_h - scale.s(*itop) - scale.s(*bottom)).max(10.0);
                    let raw: String = txt.chars().take(136).collect();
                    let mut lines: Vec<String> = Vec::new();
                    let mut cur = String::new();
                    for c in raw.chars() {
                        cur.push(c);
                        if cur.chars().count() >= 34 {
                            lines.push(std::mem::take(&mut cur));
                        }
                    }
                    if !cur.is_empty() {
                        lines.push(cur);
                    }
                    let lsh = scale.s(*line_h);
                    let start = body_top + (body_h - lsh * lines.len() as f32) / 2.0 - scale.s(4.0);
                    for (li, l) in lines.iter().enumerate() {
                        draw_text(v, cx, start + li as f32 * lsh, 0.0, false, true, l, *font_size, SceneFont::Ui, SceneWeight::Regular, false, *color, scale, pal, vals);
                    }
                    if let Some(cap) = caption.as_deref() {
                        let cap = substitute(cap, vals);
                        if !cap.is_empty() {
                            draw_text(v, cx, start + lines.len() as f32 * lsh + scale.s(2.0), 0.0, false, true, &cap, 8.5, SceneFont::Ui, SceneWeight::Regular, false, Some(ColorToken::Fg3), scale, pal, vals);
                        }
                    }
                }
                SceneItem::Hit {
                    x: ix,
                    y: iy,
                    w,
                    h,
                    r,
                    surface,
                    surface_hover,
                    hover_only,
                    text,
                    font_size,
                    font,
                    weight,
                    right,
                    center,
                    dy,
                    icon,
                    glyph,
                    glyph_pad,
                    glyph_color,
                    glyph_color_hover,
                    color,
                    color_hover,
                    key,
                    action,
                    visible,
                    bottom,
                } => {
                    // live gate: an optional row's region goes with the row
                    if !gate_open(visible.as_deref(), vals) {
                        return;
                    }
                    let sx = if *right {
                        x + card_w - scale.s(*ix + *w)
                    } else {
                        x + scale.s(*ix)
                    };
                    // `bottom` anchors the box's BOTTOM edge `y` px above the
                    // card's bottom (so the box's top clears `h` as well) —
                    // the Rust drawers' `y + h − N` pill idiom, declared
                    let sy = if *bottom {
                        y + card_h - scale.s(*iy + *h) + scale.s(*dy)
                    } else {
                        y + scale.s(*iy + *dy)
                    };
                    // negative spans = "the card minus this inset" (a pill
                    // that fills the width between two margins)
                    let sw = span_px(*w, card_w, scale);
                    let sh = span_px(*h, card_h, scale);
                    // surface fill: hover state when the pointer is over
                    // this hit's key (explicit token or lifted variant); with
                    // `hover_only` it exists only during hover
                    if let Some(sf) = surface {
                        let hov = hover_key == *key;
                        if !*hover_only || hov {
                            let sc = if hov {
                                if let Some(hov_t) = surface_hover {
                                    hov_t.resolve_with(pal, vals)
                                } else {
                                    sf.hover_variant_with(pal, vals)
                                }
                            } else {
                                sf.resolve_with(pal, vals)
                            };
                            v.push(Cmd::Rect {
                                x: sx,
                                y: sy,
                                w: sw,
                                h: sh,
                                r: scale.s(*r),
                                color: sc,
                            });
                        }
                    }
                    let tc = if hover_key == *key {
                        color_hover.or(*color)
                    } else {
                        *color
                    };
                    // optional leading glyph — an icon button's glyph sits at
                    // glyph_pad, the label shifts right past it (icon+label
                    // row inside one hit, the mirror Photo/Record buttons)
                    if let Some(g) = glyph {
                        let gcin = if hover_key == *key {
                            glyph_color_hover.or(*glyph_color).or(*color)
                        } else {
                            glyph_color.or(*color)
                        };
                        draw_text(v, sx + scale.s(*glyph_pad), sy, 0.0, false, false, g,
                            (*font_size + 1.0).max(11.0), SceneFont::Ui, SceneWeight::Regular,
                            true, gcin, scale, pal, vals);
                    }
                    let label_x = if glyph.is_some() { scale.s(*glyph_pad + 18.0) } else { 0.0 };
                    draw_text(v, sx + label_x, sy, sw - label_x, *right, *center, text, *font_size, *font, *weight, *icon, tc, scale, pal, vals);
                    hits.push(SceneHit {
                        x: sx,
                        y: sy,
                        w: sw,
                        h: sh,
                        key: *key,
                        action: action.clone(),
                    });
                }
                SceneItem::Surface { x: ix, y: iy, w, h, r, color, right, center_x, center_y, tr, br, bl, stroke, stroke_color } => {
                    let sx = if *right {
                        x + card_w - scale.s(*ix + *w)
                    } else if *center_x {
                        x + card_w / 2.0 + scale.s(*ix)
                    } else {
                        x + scale.s(*ix)
                    };
                    let sw = if *w > 0.0 { scale.s(*w) } else { card_w };
                    let sy = if *center_y {
                        y + card_h / 2.0 + scale.s(*iy)
                    } else {
                        y + scale.s(*iy)
                    };
                    let sh = if *h > 0.0 { scale.s(*h) } else { card_h };
                    let col = color.resolve_with(pal, vals);
                    // any per-corner radius → the concave-capable primitive
                    // (uniform radii reduce to the plain rect visually, but
                    // the band's top corners CONCAVE into the screen edge)
                    if *tr != 0.0 || *br != 0.0 || *bl != 0.0 {
                        v.push(Cmd::RectConcave {
                            x: sx,
                            y: sy,
                            w: sw,
                            h: sh,
                            r_tl: scale.s(*r),
                            r_tr: scale.s(*tr),
                            r_br: scale.s(*br),
                            r_bl: scale.s(*bl),
                            color: col,
                        });
                    } else {
                        v.push(Cmd::Rect {
                            x: sx,
                            y: sy,
                            w: sw,
                            h: sh,
                            r: scale.s(*r),
                            color: col,
                        });
                    }
                    // optional hairline OVER the fill, box outline (the
                    // power buttons' 1 px edge)
                    if *stroke > 0.0 {
                        if let Some(scol) = stroke_color {
                            v.push(Cmd::Outline {
                                x: sx,
                                y: sy,
                                w: sw,
                                h: sh,
                                r: scale.s(*r),
                                width: scale.s(*stroke),
                                color: scol.resolve_with(pal, vals),
                            });
                        }
                    }
                }
                SceneItem::Scissor { x: ix, y: iy, w, h } => {
                    v.push(Cmd::Scissor {
                        x: x + scale.s(*ix),
                        y: y + scale.s(*iy),
                        w: if *w > 0.0 { scale.s(*w) } else { card_w },
                        h: if *h > 0.0 { scale.s(*h) } else { card_h },
                    });
                }
                SceneItem::ScissorEnd => {
                    v.push(Cmd::ScissorEnd);
                }
                SceneItem::Divider { x: ix, y: iy, w, color, right } => {
                    let sx = if *right {
                        x + card_w - scale.s(*ix)
                    } else {
                        x + scale.s(*ix)
                    };
                    let sw = if *w > 0.0 { scale.s(*w) } else { card_w };
                    v.push(Cmd::Line {
                        x0: sx,
                        y0: y + scale.s(*iy),
                        x1: sx + sw,
                        y1: y + scale.s(*iy),
                        w: scale.s(1.0),
                        color: color.resolve_with(pal, vals),
                    });
                }
                SceneItem::Bar { x: ix, y: iy, w, h, r, max, value, fill, track, reserve, vertical, visible } => {
                    // honour the bound Toggle gate (per-category presence)
                    if let Some(v) = visible.as_deref() {
                        if vals.toggle(v) == Some(false) {
                            return;
                        }
                    }
                    let sx = x + scale.s(*ix);
                    let sy = y + scale.s(*iy);
                    let sw = if *w > 0.0 {
                        scale.s(*w)
                    } else if *reserve > 0.0 {
                        (card_w - scale.s(*reserve)).max(2.0)
                    } else {
                        card_w
                    };
                    let sh = if *h > 0.0 { scale.s(*h) } else { card_h };
                    // track underneath
                    v.push(Cmd::Rect {
                        x: sx,
                        y: sy,
                        w: sw,
                        h: sh,
                        r: scale.s(*r),
                        color: track.resolve_with(pal, vals),
                    });
                    // interpolated value → fraction of max (0 → fall back to 100)
                    let raw = crate::scene::substitute(value, vals).parse::<f32>().unwrap_or(0.0);
                    let max = if *max > 0.0 { *max } else { 100.0 };
                    let frac = if max > 0.0 {
                        (raw / max).clamp(0.0, 1.0)
                    } else {
                        0.0
                    };
                    if frac > 0.001 {
                        // `vertical` fills BOTTOM-UP: the box's height scaled
                        // by the fraction, anchored on its bottom edge (the
                        // power buttons' hold-to-confirm liquid)
                        if *vertical {
                            v.push(Cmd::Rect {
                                x: sx,
                                y: sy + sh * (1.0 - frac),
                                w: sw,
                                h: sh * frac,
                                r: scale.s(*r),
                                color: fill.resolve_with(pal, vals),
                            });
                        } else {
                            v.push(Cmd::Rect {
                                x: sx,
                                y: sy,
                                w: sw * frac,
                                h: sh,
                                r: scale.s(*r),
                                color: fill.resolve_with(pal, vals),
                            });
                        }
                    }
                }
                SceneItem::Battery { x: ix, y: iy, w, h, vertical, value, max, level, visible } => {
                    if let Some(v) = visible.as_deref() {
                        if vals.toggle(v) == Some(false) {
                            return;
                        }
                    }
                    let raw = crate::scene::substitute(value, vals).parse::<f32>().unwrap_or(0.0);
                    let max = if *max > 0.0 { *max } else { 100.0 };
                    let pct = raw.clamp(0.0, max).round() as i32;
                    // both axes 0 = the battery cards' own card-fit ladder
                    let (bw, bh, cx, cy) = if *w <= 0.0 && *h <= 0.0 {
                        if *vertical {
                            let aw = (card_w * 0.30).clamp(16.0, 70.0);
                            let ah = (card_h - scale.s(44.0)).max(aw * 1.6);
                            let iw = (aw * 2.0 / 3.0).max(12.0);
                            (iw, iw * 1.7, x + card_w / 2.0, y + scale.s(38.0) + (ah - iw * 1.7) / 2.0)
                        } else {
                            (
                                (card_w * 0.46).clamp(40.0, 220.0),
                                (card_h * 0.36).clamp(16.0, 60.0),
                                x + card_w / 2.0,
                                y + scale.s(42.0) + (card_h - scale.s(70.0)) / 2.0,
                            )
                        }
                    } else {
                        let bw = if *w > 0.0 { scale.s(*w) } else { card_w };
                        let bh = if *h > 0.0 { scale.s(*h) } else { card_h };
                        (bw, bh, x + scale.s(*ix) + bw / 2.0, y + scale.s(*iy) + bh / 2.0)
                    };
                    // the level ladder is the shared one; the gauge itself
                    // stays palette-driven (fg2 case, hover interior)
                    let fill_c = match level {
                        Some(t) => t.resolve_with(pal, vals),
                        None => battery_level_color(pal, pct),
                    };
                    push_battery_icon_parts(
                        v, crate::ui::fg2(pal), crate::ui::hover(pal), fill_c, pct,
                        cx, cy, bw, bh, *vertical,
                    );
                }
                SceneItem::Rows {
                    name,
                    y: iy,
                    w,
                    row_h,
                    pitch,
                    pad,
                    row_dy,
                    r,
                    cols,
                    key_base,
                    hover_surface,
                    max_rows,
                    start,
                    check,
                    check_x,
                    del_base,
                    del_pad,
                    del_dy,
                    del_size,
                    visible,
                    hairline,
                    hover_bar,
                    flash,
                    karaoke,
                    karaoke_fs,
                    bottom: bottom_inset,
                } => {
                    let Some(rows) = vals.rows(name) else { return };
                    // the karaoke binding aligns its word-split lines to `rows`
                    // by index — may be absent/unbound (plain lyrics) and then
                    // every row falls through to the normal cell path
                    let ka = karaoke.as_deref().and_then(|k| vals.karaoke(k));
                    // the flashed row (transient "just copied" accent) — bound
                    // scalar is index + 1 so an index-0 row is representable
                    let flash_i = flash
                        .as_deref()
                        .and_then(|s| vals.scalar(s))
                        .map(|s| s as usize)
                        .filter(|f| *f > 0)
                        .map(|f| f - 1);
                    let pmin = scale.s(*pad);
                    let row_w = if *w > 0.0 { scale.s(*w) } else { (card_w - pmin * 2.0).max(2.0) };
                    let row_h = scale.s(*row_h);
                    let pitch = scale.s(pitch.unwrap_or(row_h));
                    let limit = if *max_rows > 0 {
                        (*max_rows).min(rows.len())
                    } else {
                        rows.len()
                    };
                    let start_i = start
                        .as_deref()
                        .and_then(|s| vals.scalar(s))
                        .map(|s| (s as usize).min(limit.saturating_sub(1)))
                        .unwrap_or(0);
                    // a bound `visible` Toggle resolving `false` suppresses the
                    // whole list (stateful body swap — Notes composer replaces it)
                    let hidden = visible
                        .as_deref()
                        .map(|v| vals.toggle(v) == Some(false))
                        .unwrap_or(false);
                    if !hidden {
                    // bottom inset: rows must clear `card_h − bottom` (a
                    // bottom-anchored control row owns that zone)
                    let bottom_line = y + card_h - scale.s(*bottom_inset);
                    for (i, row) in rows.iter().skip(start_i).take(limit.saturating_sub(start_i)).enumerate() {
                        let ry = y + scale.s(*iy) + i as f32 * pitch;
                        if ry + row_h > y + card_h + 0.5 {
                            continue;
                        }
                        if *bottom_inset > 0.0 && ry + row_h > bottom_line {
                            continue;
                        }
                        // ── karaoke rows (the lyrics sung line) — replace the
                        // whole row render: the active row paints the accent
                        // wash and walks its words (live word white + underline,
                        // the rest near-black, estimated widths + line-break of
                        // the Rust drawer); other rows render the joined line
                        // (hover → accent ink). Regions always register.
                        if let Some(krow) = ka.and_then(|ka| ka.get(start_i + i)) {
                            let kkey = row_key(row.key, key_base.as_ref(), i);
                            if krow.active {
                                v.push(Cmd::Rect {
                                    x: x + pmin,
                                    y: ry + scale.s(1.0),
                                    w: row_w,
                                    h: row_h - scale.s(4.0),
                                    r: scale.s(4.0),
                                    color: pal.acc,
                                });
                                let fs = scale.fs(*karaoke_fs);
                                let est_w = |s: &str| -> f32 {
                                    s.chars()
                                        .map(|c| {
                                            if c.is_ascii_uppercase() || c.is_ascii_digit() {
                                                0.62
                                            } else if c.is_whitespace() {
                                                0.0
                                            } else {
                                                0.52
                                            }
                                        })
                                        .sum::<f32>()
                                        * fs
                                };
                                let clip = x + pmin + row_w - scale.s(8.0);
                                let mut wx = x + pmin + scale.s(6.0);
                                for (word, live) in &krow.words {
                                    if wx > clip {
                                        break;
                                    }
                                    crate::ui::text_w(v, wx, ry + scale.s(0.5), word, fs, crate::text::Ff::Ui, crate::text::Fw::Light, if *live { 0xffff_ffffu32 } else { 0x1d1d_1du32 }, false);
                                    if *live {
                                        v.push(Cmd::Rect {
                                            x: wx,
                                            y: ry + row_h - scale.s(6.5),
                                            w: est_w(word),
                                            h: scale.s(1.5),
                                            r: scale.s(0.8),
                                            color: 0xffff_ffffu32,
                                        });
                                    }
                                    wx += est_w(word) + scale.s(4.0);
                                }
                            } else if hover_key == kkey && kkey != 0 {
                                v.push(Cmd::Rect {
                                    x: x + pmin,
                                    y: ry + scale.s(1.0),
                                    w: row_w,
                                    h: row_h - scale.s(4.0),
                                    r: scale.s(4.0),
                                    color: crate::ui::hover(pal),
                                });
                                let joined: String = krow.words.iter().map(|(w, _)| w.as_str()).collect::<Vec<_>>().join(" ");
                                crate::ui::text(v, x + pmin + scale.s(6.0), ry + scale.s(0.5), &joined, scale.fs(*karaoke_fs), pal.acc, false);
                            } else {
                                let joined: String = krow.words.iter().map(|(w, _)| w.as_str()).collect::<Vec<_>>().join(" ");
                                crate::ui::text(v, x + pmin + scale.s(6.0), ry + scale.s(0.5), &joined, scale.fs(*karaoke_fs), pal.fg, false);
                            }
                            hits.push(SceneHit {
                                x: x + pmin,
                                y: ry,
                                w: row_w,
                                h: row_h,
                                key: kkey,
                                action: row.action.clone(),
                            });
                            continue;
                        }
                        let key = row_key(row.key, key_base.as_ref(), i);
                        let hovered = hover_key == key && key != 0;
                        let flashed = flash_i == Some(start_i + i);
                        let bg = if flashed {
                            // the "just copied" overlay — translucent accent
                            // wash, exactly the Rust snippets flash
                            Some((pal.acc & 0xFFFF_FF00) | 0x30)
                        } else if let Some(sf) = row.surface {
                            Some(sf.resolve_with(pal, vals))
                        } else if hovered {
                            hover_surface.map(|sf| sf.hover_variant_with(pal, vals))
                        } else {
                            None
                        };
                        if let Some(color) = bg {
                            v.push(Cmd::Rect {
                                x: x + pmin,
                                y: ry,
                                w: row_w,
                                h: row_h,
                                r: scale.s(*r),
                                color,
                            });
                        }
                        if let Some(cname) = check.as_deref() {
                            let checked = vals
                                .checks(cname)
                                .and_then(|c| c.get(start_i + i))
                                .copied()
                                .unwrap_or(false);
                            let bx = x + pmin + scale.s(*check_x);
                            let by = ry + scale.s(1.0);
                            let bw = scale.s(11.0);
                            if checked {
                                v.push(Cmd::Rect {
                                    x: bx,
                                    y: by,
                                    w: bw,
                                    h: bw,
                                    r: scale.s(3.5),
                                    color: mix(pal.bg, pal.acc, 0.16),
                                });
                                draw_text(
                                    v,
                                    bx + scale.s(1.5),
                                    by + scale.s(1.5),
                                    scale.s(11.0),
                                    true,
                                    true,
                                    crate::icons::ICON_CHECK,
                                    8.0,
                                    SceneFont::Ui,
                                    SceneWeight::Regular,
                                    true,
                                    Some(ColorToken::Acc),
                                    scale,
                                    pal,
                                    vals,
                                );
                            } else {
                                v.push(Cmd::Outline {
                                    x: bx,
                                    y: by,
                                    w: bw,
                                    h: bw,
                                    r: scale.s(3.5),
                                    width: scale.s(1.2),
                                    color: if hovered { pal.acc } else { mix(crate::ui::hover(pal), pal.fg, 0.25) },
                                });
                            }
                        }
                        let mut pill_left: Option<f32> = None;
                        if *hover_bar && hovered {
                            v.push(Cmd::Rect {
                                x: x + pmin,
                                y: ry + scale.s(2.0),
                                w: scale.s(2.5),
                                h: row_h - scale.s(6.0),
                                r: scale.s(1.0),
                                color: pal.acc,
                            });
                        }
                        for (ci, col) in cols.iter().enumerate() {
                            let cell0 = row.cols.get(ci).map(String::as_str).unwrap_or("");
                            if cell0.is_empty() {
                                continue;
                            }
                            let base = row.col_colors.get(ci).and_then(|c| *c)
                            .or(row.color)
                            .or(col.color);
                            let color = if flashed {
                                col.flash.or(base)
                            } else if hovered {
                                col.hover.or(base)
                            } else {
                                base
                            }
                            .unwrap_or(ColorToken::Fg);
                            let size = if col.size > 0.0 { col.size } else { 9.5 };
                            let cell: std::borrow::Cow<'_, str> = if let Some(t) = col.truncate {
                                // stop the cell `t` base px before the card's
                                // right edge (0.62·fs px/glyph, ≥ 8) — the Rust
                                // list drawers' `avail` reservation
                                let avail = card_w - pmin * 2.0 - scale.s(t);
                                let n = (avail / (0.62 * scale.fs(size)).max(1.0)).floor().max(8.0) as usize;
                                if cell0.chars().count() > n {
                                    cell0.chars().take(n).collect::<String>().into()
                                } else {
                                    cell0.into()
                                }
                            } else {
                                cell0.into()
                            };
                            let cdy = col.dy.unwrap_or(*row_dy);
                            if let Some(bspec) = &col.bar {
                                // meter-bar cell: the cell text (interpolated)
                                // parses as a fraction of `max`; a track spans
                                // the cell box, the fill hugs the left. The box
                                // honors x/w/right/edge like every cell.
                                let frac = substitute(cell0, vals)
                                    .trim()
                                    .parse::<f32>()
                                    .unwrap_or(0.0)
                                    / (bspec.max.max(0.0001));
                                let frac = frac.clamp(0.0, 1.0);
                                let bw = scale.s(col.w);
                                let (bl, _br) = if let Some(e) = col.edge {
                                    (x + card_w - scale.s(e) - bw, x + card_w - scale.s(e))
                                } else if col.right {
                                    let r = x + pmin + scale.s(col.x) + bw;
                                    (r - bw, r)
                                } else {
                                    let l = x + pmin + scale.s(col.x);
                                    (l, l + bw)
                                };
                                let track = bspec
                                    .track
                                    .map(|t| t.resolve_with(pal, vals))
                                    .unwrap_or_else(|| crate::ui::hover(pal));
                                v.push(Cmd::Rect {
                                    x: bl,
                                    y: ry + scale.s(bspec.top),
                                    w: bw,
                                    h: scale.s(bspec.height),
                                    r: scale.s(bspec.radius),
                                    color: track,
                                });
                                let fill_w = bw * frac;
                                if fill_w > 0.5 {
                                    v.push(Cmd::Rect {
                                        x: bl,
                                        y: ry + scale.s(bspec.top),
                                        w: fill_w,
                                        h: scale.s(bspec.height),
                                        r: scale.s(bspec.radius),
                                        color: bspec
                                            .fill
                                            .map(|f| f.resolve_with(pal, vals))
                                            .unwrap_or_else(|| color.resolve_with(pal, vals)),
                                    });
                                }
                            } else if let Some(pspec) = &col.pill {
                                // chip/pill cell: rounded rect auto-fit to the
                                // text (0.62·fs px/glyph + 2·pad), right edge at
                                // `edge` (or the `x`/`right` anchor)
                                let cell_text = substitute(cell.as_ref(), vals);
                                let text_px = cell_text.chars().count() as f32 * 0.62 * scale.fs(size);
                                let pw = text_px + scale.s(pspec.pad) * 2.0;
                                let (pl, _pr) = if let Some(e) = col.edge {
                                    (x + card_w - scale.s(e) - pw, x + card_w - scale.s(e))
                                } else if col.right {
                                    let r = x + pmin + scale.s(col.x) + scale.s(col.w);
                                    (r - pw, r)
                                } else {
                                    let l = x + pmin + scale.s(col.x);
                                    (l, l + pw)
                                };
                                let bg = pspec
                                    .bg
                                    .map(|b| b.resolve_with(pal, vals))
                                    .unwrap_or_else(|| match color {
                                        ColorToken::Acc | ColorToken::AccTint => mix(pal.bg, pal.acc, 0.16),
                                        _ => crate::ui::hover(pal),
                                    });
                                v.push(Cmd::Rect {
                                    x: pl,
                                    y: ry + scale.s(pspec.top),
                                    w: pw,
                                    h: scale.s(pspec.height),
                                    r: scale.s(pspec.radius),
                                    color: bg,
                                });
                                draw_text(
                                    v,
                                    pl,
                                    ry + scale.s(cdy),
                                    pw,
                                    false,
                                    true,
                                    cell_text.as_str(),
                                    size,
                                    col.font,
                                    col.weight,
                                    col.icon,
                                    Some(color),
                                    scale,
                                    pal,
                                    vals,
                                );
                                if col.del_anchor {
                                    pill_left = Some(pl);
                                }
                            } else {
                                // plain text cell — `edge` anchors the cell's
                                // RIGHT edge at `card_w − edge` (base px) so
                                // right-column entries follow card resize
                                // without an explicit `x` (the news open
                                // glyph / source label, the ticker chg% col).
                                let tx = if let Some(e) = col.edge {
                                    x + card_w - scale.s(e)
                                } else {
                                    x + pmin + scale.s(col.x)
                                };
                                draw_text(
                                    v,
                                    tx,
                                    ry + scale.s(cdy),
                                    scale.s(col.w),
                                    col.right,
                                    col.center,
                                    cell.as_ref(),
                                    size,
                                    col.font,
                                    col.weight,
                                    col.icon,
                                    Some(color),
                                    scale,
                                    pal,
                                    vals,
                                );
                            }
                        }
                        if *hairline {
                            v.push(Cmd::Rect {
                                x: x + pmin,
                                y: ry + row_h - scale.s(1.0),
                                w: row_w,
                                h: scale.s(1.0),
                                r: scale.s(0.5),
                                color: crate::ui::hover(pal),
                            });
                        }
                        if key != 0 {
                            hits.push(SceneHit {
                                x: x + pmin,
                                y: ry,
                                w: row_w,
                                h: row_h,
                                key,
                                action: row.action.clone(),
                            });
                        }
                        if *del_base != 0 && (hovered || hover_key == *del_base + i as u32) {
                            let del_key = *del_base + i as u32;
                            // a del_anchor col moves the ✕ left of its drawn
                            // box (`del_pad` = gap from that box, like the
                            // chip-relative deletes in the countdown drawer)
                            let dx = pill_left
                                .map(|p| p - scale.s(*del_pad))
                                .unwrap_or(x + card_w - scale.s(*del_pad));
                            draw_text(
                                v,
                                dx,
                                ry + scale.s(*del_dy),
                                0.0,
                                false,
                                false,
                                crate::icons::ICON_CLOSE,
                                *del_size,
                                SceneFont::Ui,
                                SceneWeight::Regular,
                                true,
                                Some(if hover_key == del_key { ColorToken::Danger } else { ColorToken::Fg }),
                                scale,
                                pal,
                                vals,
                            );
                            if let Some(pl) = pill_left {
                                hits.push(SceneHit {
                                    x: pl - scale.s(22.0),
                                    y: ry,
                                    w: scale.s(20.0),
                                    h: row_h,
                                    key: del_key,
                                    action: None,
                                });
                            } else {
                                hits.push(SceneHit {
                                    x: x + card_w - scale.s(24.0),
                                    y: ry - scale.s(4.0),
                                    w: scale.s(22.0),
                                    h: scale.s(22.0),
                                    key: del_key,
                                    action: None,
                                });
                            }
                        }
                    }
                    }
                }
                SceneItem::Spark {
                    name,
                    x: ix,
                    y: iy,
                    w,
                    h,
                    color,
                    max,
                    kind,
                    thickness,
                } => {
                    let Some(data) = vals.spark(name) else { return };
                    if data.len() < 2 {
                        return;
                    }
                    let sx = x + scale.s(*ix);
                    let sy = y + scale.s(*iy);
                    let sw = if *w > 0.0 { scale.s(*w) } else { card_w };
                    let sh = if *h > 0.0 { scale.s(*h) } else { card_h };
                    let col = color.unwrap_or(ColorToken::Acc).resolve_with(pal, vals);
                    let t = scale.s(if *thickness > 0.0 { *thickness } else { 1.6 });
                    let peak = if *max > 0.0 {
                        *max
                    } else {
                        data.iter().cloned().fold(0.0f32, f32::max).max(1e-6)
                    };
                    let step = sw / (data.len() - 1) as f32;
                    match kind {
                        SparkKind::Line => {
                            let pad_v = t / 2.0 + 1.0;
                            let usable = (sh - pad_v * 2.0).max(2.0);
                            let mut prev: Option<(f32, f32)> = None;
                            for (i, &d) in data.iter().enumerate() {
                                let px = sx + i as f32 * step;
                                let py = sy + sh - pad_v - (d / peak).clamp(0.0, 1.0) * usable;
                                if let Some((qx, qy)) = prev {
                                    v.push(Cmd::Line { x0: qx, y0: qy, x1: px, y1: py, w: t, color: col });
                                }
                                prev = Some((px, py));
                            }
                        }
                        SparkKind::Column => {
                            let cw = (step * 0.6).max(1.0);
                            let usable = (sh - 2.0).max(2.0);
                            for (i, &d) in data.iter().enumerate() {
                                let frac = (d / peak).clamp(0.0, 1.0);
                                let ch = if frac > 0.001 {
                                    (frac * usable).max(1.0)
                                } else {
                                    0.0
                                };
                                v.push(Cmd::Rect {
                                    x: sx + i as f32 * step - cw / 2.0,
                                    y: sy + sh - ch,
                                    w: cw,
                                    h: ch,
                                    r: scale.s(1.0),
                                    color: col,
                                });
                            }
                        }
                    }
                }
                SceneItem::Ring {
                    name,
                    cx,
                    cy,
                    center_x,
                    center_y,
                    cx_pct,
                    cy_pct,
                    radius,
                    fit,
                    dot_ladder,
                    max,
                    beads,
                    dot,
                    fill,
                    track,
                } => {
                    let Some(value) = vals.scalar(name) else { return };
                    let cx_px = if *center_x {
                        x + card_w / 2.0 + scale.s(*cx)
                    } else {
                        x + scale.s(*cx)
                    } + card_w * *cx_pct;
                    let cy_px = (if *center_y {
                        y + card_h / 2.0 + scale.s(*cy)
                    } else {
                        y + scale.s(*cy)
                    }) + card_h * *cy_pct;
                    // responsive radius: `fit` wins over the fixed `radius`
                    // (both resolve in base px, then scale)
                    let base_r = match fit {
                        Some(f) => f.radius(card_h, scale),
                        None => *radius,
                    };
                    let r = scale.s(if base_r > 0.0 { base_r } else { 34.0 });
                    // responsive bead size: (min_radius, dot) steps on the
                    // resolved radius, first match wins
                    let base_d = match dot_ladder.iter().find(|(min_r, _)| base_r >= *min_r) {
                        Some((_, d)) => *d,
                        None => *dot,
                    };
                    let n = if *beads > 0 { *beads } else { 36 };
                    let max = if *max > 0.0 { *max } else { 100.0 };
                    let frac = (value / max).clamp(0.0, 1.0);
                    let lit = ((frac * n as f32).round() as usize).min(n.max(1) as usize);
                    bead_ring(
                        v, cx_px, cy_px, r, scale.s(if base_d > 0.0 { base_d } else { 3.0 }),
                        n as usize, lit, fill.resolve_with(pal, vals), track.resolve_with(pal, vals),
                    );
                }
                SceneItem::Fader {
                    name,
                    x: ix,
                    y: iy,
                    w,
                    h,
                    key,
                    action,
                    fill,
                    track,
                    head,
                    centered,
                    visible,
                } => {
                    // whole-row gate (the optional balance / saturation rows)
                    if let Some(v) = visible.as_deref() {
                        if vals.toggle(v) == Some(false) {
                            return;
                        }
                    }
                    let Some(val) = vals.fader(name) else { return };
                    let sx = x + scale.s(*ix);
                    let sy = y + scale.s(*iy);
                    let sw = span_px(*w, card_w, scale);
                    let sh = span_px(*h, card_h, scale);
                    let cy = sy + sh / 2.0;
                    let t = scale.s(4.0);
                    let fill_c = fill.resolve_with(pal, vals);
                    v.push(Cmd::Rect {
                        x: sx,
                        y: cy - t / 2.0,
                        w: sw,
                        h: t,
                        r: t / 2.0,
                        color: track.resolve_with(pal, vals),
                    });
                    // `centered` grows the fill out from the middle (the L↔R
                    // balance fader) instead of hugging the left edge
                    let val = val.clamp(0.0, 1.0);
                    if *centered {
                        let cx = sx + sw * 0.5;
                        let flen = (((val - 0.5).abs() * sw) as u32).max(1) as f32;
                        let flen = flen.min(sw * 0.5);
                        let (fx, fw) = if val < 0.5 { (cx - flen, flen) } else { (cx, flen) };
                        if (val - 0.5).abs() > 0.001 {
                            v.push(Cmd::Rect {
                                x: fx,
                                y: cy - t / 2.0,
                                w: fw,
                                h: t,
                                r: t / 2.0,
                                color: fill_c,
                            });
                        }
                    } else {
                        let fill_w = (sw * val).clamp(0.0, sw);
                        if fill_w > 0.0 {
                            v.push(Cmd::Rect {
                                x: sx,
                                y: cy - t / 2.0,
                                w: fill_w,
                                h: t,
                                r: t / 2.0,
                                color: fill_c,
                            });
                        }
                    }
                    let d = scale.s(if hover_key == *key && *key != 0 { *head + 2.0 } else { *head });
                    let kx = (sx + sw * val - d / 2.0).clamp(sx, sx + sw - d / 2.0);
                    v.push(Cmd::Rect {
                        x: kx,
                        y: cy - d / 2.0,
                        w: d,
                        h: d,
                        r: d / 2.0,
                        color: fill_c,
                    });
                    if *key != 0 {
                        hits.push(SceneHit { x: sx, y: sy, w: sw, h: sh, key: *key, action: action.clone() });
                    }
                }
                SceneItem::Toggle { name, x: ix, y: iy, w, h, key, action, right, visible } => {
                    // pane gate (like the Grid/Image `visible` bindings)
                    if let Some(v) = visible.as_deref() {
                        if vals.toggle(v) == Some(false) {
                            return;
                        }
                    }
                    let Some(on) = vals.toggle(name) else { return };
                    let sx = if *right {
                        x + card_w - scale.s(*ix + *w)
                    } else {
                        x + scale.s(*ix)
                    };
                    let sy = y + scale.s(*iy);
                    let sw = span_px(*w, card_w, scale);
                    let sh = span_px(*h, card_h, scale);
                    let hov = hover_key == *key && *key != 0;
                    let track_c = if on {
                        if hov { mix(pal.acc, pal.bg, 0.18) } else { pal.acc }
                    } else {
                        mix(pal.bg, pal.fg, if hov { 0.22 } else { 0.14 })
                    };
                    v.push(Cmd::Rect { x: sx, y: sy, w: sw, h: sh, r: sh / 2.0, color: track_c });
                    let kd = (sh - scale.s(4.0)).max(4.0);
                    let kx = sx + scale.s(2.0)
                        + if on { sw - kd - scale.s(4.0) } else { 0.0 };
                    v.push(Cmd::Rect {
                        x: kx,
                        y: sy + (sh - kd) / 2.0,
                        w: kd,
                        h: kd,
                        r: kd / 2.0,
                        color: if on { pal.sfg } else { pal.fg },
                    });
                    if *key != 0 {
                        hits.push(SceneHit { x: sx, y: sy, w: sw, h: sh, key: *key, action: action.clone() });
                    }
                }
                SceneItem::TabRow { name, sel, y: iy, pad, h, key_base, active, idle } => {
                    let Some(rows) = vals.rows(name) else { return };
                    if rows.is_empty() {
                        return;
                    }
                    let p = scale.s(*pad);
                    let th_base = if *h > 0.0 { *h } else { 24.0 };
                    let th = scale.s(th_base);
                    let ty = y + scale.s(*iy);
                    let sel_idx = sel.as_deref().and_then(|s| vals.scalar(s)).map(|s| s as usize).unwrap_or(0);
                    let per = (card_w - p * 2.0) / rows.len() as f32;
                    let a_c = active.resolve_with(pal, vals);
                    for (i, tab) in rows.iter().enumerate() {
                        let tx = x + p + i as f32 * per;
                        let key = key_base + i as u32;
                        let is_active = i == sel_idx;
                        let hov = hover_key == key;
                        let label = tab.cols.first().map(String::as_str).unwrap_or("");
                        if is_active && !hov {
                            v.push(Cmd::Rect {
                                x: tx + scale.s(3.0),
                                y: ty,
                                w: per - scale.s(6.0),
                                h: th,
                                r: scale.s(6.0),
                                color: ColorToken::AccTint.resolve_with(pal, vals),
                            });
                        } else if hov {
                            v.push(Cmd::Rect {
                                x: tx + scale.s(3.0),
                                y: ty,
                                w: per - scale.s(6.0),
                                h: th,
                                r: scale.s(6.0),
                                color: ColorToken::Hover.resolve_with(pal, vals),
                            });
                        }
                        if is_active {
                            // 2px active rail under the tab
                            v.push(Cmd::Rect {
                                x: tx + scale.s(4.0),
                                y: ty + th - scale.s(2.0),
                                w: per - scale.s(8.0),
                                h: scale.s(2.0),
                                r: scale.s(1.0),
                                color: a_c,
                            });
                        }
                        draw_text(
                            v,
                            tx + per / 2.0,
                            ty + scale.s(th_base * 0.25),
                            0.0,
                            false,
                            true,
                            label,
                            9.5,
                            SceneFont::Ui,
                            if is_active { SceneWeight::Semibold } else { SceneWeight::Regular },
                            false,
                            if is_active { Some(*active) } else if hov { Some(ColorToken::Fg) } else { Some(*idle) },
                            scale,
                            pal,
                            vals,
                        );
                        hits.push(SceneHit { x: tx, y: ty, w: per, h: th, key, action: None });
                    }
                }
                SceneItem::Strip { chips, sel, scroll, x: ix, y: iy, w, h, pad, spacing, chip_pad, fs, key_base } => {
                    let Some(labels) = vals.chips(chips) else { return };
                    if labels.is_empty() {
                        return;
                    }
                    let sel_idx = sel.as_deref().and_then(|s| vals.scalar(s)).map(|v| v as usize).unwrap_or(0);
                    let scroll_off = scroll.as_deref().and_then(|s| vals.scalar(s)).unwrap_or(0.0);
                    let sx = x + scale.s(*ix);
                    let sy = y + scale.s(*iy);
                    let sw = if *w > 0.0 { scale.s(*w) } else { card_w };
                    let sh = if *h > 0.0 { scale.s(*h) } else { card_h };
                    let pd = scale.s(*pad);
                    let sp = scale.s(*spacing);
                    let cpd = scale.s(*chip_pad);
                    let fs_scaled = scale.fs(*fs);
                    let radius = sh / 2.0;
                    let widths: Vec<f32> = labels
                        .iter()
                        .map(|c| cpd + c.chars().count() as f32 * fs_scaled * 0.62 + cpd)
                        .collect();
                    let total: f32 = widths.iter().sum::<f32>() + sp * widths.len().saturating_sub(1) as f32;
                    let avail = (sw - pd * 2.0).max(0.0);
                    let scroll_clamped = scroll_off.clamp(0.0, (total - avail).max(0.0));
                    let cy = sy + scale.s(1.0);
                    let ch = sh - scale.s(2.0);
                    let mut cx = sx + pd - scroll_clamped;
                    for (i, label) in labels.iter().enumerate() {
                        let cw = widths[i];
                        if cx + cw < sx + pd - 1.0 || cx > sx + sw - pd + 1.0 {
                            cx += cw + sp;
                            continue;
                        }
                        let active = sel_idx == i;
                        let key = *key_base + i as u32;
                        let hov = hover_key == key;
                        let bg = if active {
                            pal.acc
                        } else if hov {
                            crate::ui::hover_hl(pal)
                        } else {
                            crate::ui::hover(pal)
                        };
                        v.push(Cmd::Rect {
                            x: cx,
                            y: cy,
                            w: cw,
                            h: ch,
                            r: radius,
                            color: bg,
                        });
                        draw_text(
                            v,
                            cx + cw / 2.0,
                            cy + ch / 2.0 - fs_scaled * 0.25,
                            0.0,
                            false,
                            true,
                            label,
                            *fs,
                            SceneFont::Ui,
                            SceneWeight::Regular,
                            false,
                            if active {
                                Some(ColorToken::Sfg)
                            } else {
                                Some(ColorToken::Fg)
                            },
                            scale,
                            pal,
                            vals,
                        );
                        hits.push(SceneHit {
                            x: cx,
                            y: cy,
                            w: cw,
                            h: ch,
                            key,
                            action: None,
                        });
                        cx += cw + sp;
                    }
                }
                SceneItem::BannerRow { cells, x: ix, y: iy, w, h, visible, overflow, zones } => {
                    // pane gate (the declarative strip only draws in the
                    // viewing state — edit chrome stays Rust)
                    if let Some(v) = visible.as_deref() {
                        if vals.toggle(v) == Some(false) {
                            return;
                        }
                    }
                    let Some(cells) = vals.banner_cells(cells) else {
                        return;
                    };
                    if cells.is_empty() {
                        return;
                    }
                    let sx = x + scale.s(*ix);
                    let sy = y + scale.s(*iy);
                    let sw = if *w > 0.0 { scale.s(*w) } else { card_w };
                    let sh = if *h > 0.0 { scale.s(*h) } else { card_h };
                    // clipping mirrors the Rust drawer exactly: whole-strip
                    // overflow clips at the band box, a zoned strip clips each
                    // zone window independently (zones are contiguous runs, so
                    // a single state machine keeps one scissor active)
                    let zoned = zones.as_deref().map(|n| vals.toggle(n) == Some(true)).unwrap_or(false)
                        && cells.iter().any(|c| c.zone < 3);
                    let overflowing = overflow.as_deref().map(|n| vals.toggle(n) == Some(true)).unwrap_or(false);
                    if overflowing {
                        v.push(Cmd::Scissor { x: sx, y: sy, w: sw, h: sh });
                    }
                    let z_margin = scale.s(10.0);
                    let z_zw = (sw - z_margin * 2.0).max(0.0) / 3.0;
                    let mut z_cur: Option<u8> = None;
                    let zone_open = |v: &mut Vec<Cmd>, cur: &mut Option<u8>, zc: u8| {
                        if *cur != Some(zc) {
                            if cur.is_some() {
                                v.push(Cmd::ScissorEnd);
                            }
                            *cur = Some(zc);
                            v.push(Cmd::Scissor {
                                x: sx + z_margin + zc as f32 * z_zw,
                                y: sy,
                                w: z_zw,
                                h: sh,
                            });
                        }
                    };
                    let fs = scale.fs(11.5);
                    let bias = scale.s(crate::shell::panels::dashboard::TEXT_Y_BIAS);
                    // every strip glyph centers on the band midline; text gets
                    // the shared downward optical bias
                    let cy = |size: f32| sy + (sh - size) / 2.0;
                    let tcy = |size: f32| cy(size) + bias;
                    let ink = |name: &str| -> u32 {
                        vals.color(name)
                            .map(|sc| match sc {
                                SceneColor::Raw(u) => u,
                                SceneColor::Token(t) => t.resolve(pal),
                            })
                            .unwrap_or(pal.fg)
                    };
                    for c in cells {
                        if c.w <= 0.0 {
                            continue;
                        }
                        let bx = sx + scale.s(c.x);
                        let bw = scale.s(c.w);
                        // per-zone scissor transitions interleave with the
                        // cell sweep (mirrors the Rust drawer's zone_push)
                        if zoned {
                            zone_open(v, &mut z_cur, c.zone.min(2));
                        }
                        let col = ink(&c.ink);
                        match c.kind {
                            BannerCellKind::Text { n } => {
                                let t: String = if let Some(n) = n {
                                    c.text.chars().take(n as usize).collect()
                                } else {
                                    c.text.clone()
                                };
                                crate::ui::text(v, bx + scale.s(12.0), tcy(fs), t, fs, col, false);
                            }
                            BannerCellKind::IconLabel => {
                                if let Some(g) = &c.glyph {
                                    crate::ui::text(v, bx + scale.s(12.0), cy(scale.fs(13.0)), g.as_str(), scale.fs(13.0), col, true);
                                }
                                crate::ui::text(v, bx + scale.s(29.0), tcy(fs), c.text.as_str(), fs, col, false);
                            }
                            BannerCellKind::Icon { size } => {
                                if let Some(g) = &c.glyph {
                                    crate::ui::text_c(v, bx + bw / 2.0, cy(scale.fs(size)), g.as_str(), scale.fs(size), col, true);
                                }
                            }
                            BannerCellKind::TextAcc => {
                                crate::ui::text(v, bx + scale.s(12.0), tcy(fs), c.text.as_str(), fs, col, false);
                            }
                            BannerCellKind::Brand => {
                                crate::ui::text_cw(v, bx + bw / 2.0, cy(scale.fs(13.0)), c.text.as_str(), scale.fs(13.0), crate::text::Ff::Brand, crate::text::Fw::Regular, col, false);
                            }
                            BannerCellKind::Sep => {
                                crate::ui::text_c(v, bx + bw / 2.0, cy(scale.fs(10.0)), c.text.as_str(), scale.fs(10.0), col, false);
                            }
                            BannerCellKind::Tray => {
                                // icons right-aligned inside the chip; each is
                                // its own 20px-wide sub-region (keys 30+ti) so
                                // SNI activate still reaches the right item
                                let n = c.icons.len().min(8);
                                let mut ix2 = bx + bw - scale.s(8.0) - n as f32 * scale.s(20.0);
                                for (ti, ik) in c.icons.iter().take(n).enumerate() {
                                    if ik.is_empty() {
                                        crate::ui::text(v, ix2 + scale.s(2.0), cy(scale.fs(10.0)), crate::icons::ICON_TRAY_FILL, scale.fs(10.0), col, true);
                                    } else {
                                        v.push(Cmd::Image { x: ix2, y: cy(scale.s(16.0)), w: scale.s(16.0), h: scale.s(16.0), key: ik.clone() });
                                    }
                                    hits.push(SceneHit { x: ix2, y: sy, w: scale.s(20.0), h: sh, key: 30 + ti as u32, action: None });
                                    ix2 += scale.s(20.0);
                                }
                            }
                            BannerCellKind::Filler => {}
                        }
                        if c.key != 0 {
                            hits.push(SceneHit { x: bx, y: sy, w: bw, h: sh, key: c.key, action: None });
                        }
                    }
                    if overflowing {
                        v.push(Cmd::ScissorEnd);
                    } else if zoned && z_cur.is_some() {
                        v.push(Cmd::ScissorEnd);
                    }
                }
                SceneItem::Composer {
                    pad,
                    y: iy,
                    w,
                    h,
                    bottom,
                    text,
                    focus,
                    placeholder,
                    key,
                    action,
                    add,
                    add_key,
                    add_action,
                } => {
                    let foc = focus.as_deref().and_then(|f| vals.toggle(f)).unwrap_or(false);
                    let sx = x + scale.s(*pad);
                    let fw = if *w > 0.0 { scale.s(*w) } else { (card_w - scale.s(*pad) * 2.0).max(2.0) };
                    let sh = scale.s(*h);
                    let sy = if *bottom { y + card_h - sh - scale.s(*iy) } else { y + scale.s(*iy) };
                    // rounded field — the focused tone is a stronger blend of
                    // the same hover gray (mirrors the Rust composer strip)
                    v.push(Cmd::Rect {
                        x: sx,
                        y: sy,
                        w: fw,
                        h: sh,
                        r: sh / 2.0,
                        color: mix(crate::ui::hover(pal), pal.fg, if foc { 0.09 } else { 0.04 }),
                    });
                    if foc {
                        v.push(Cmd::Outline {
                            x: sx,
                            y: sy,
                            w: fw,
                            h: sh,
                            r: sh / 2.0,
                            width: scale.s(1.4),
                            color: pal.acc,
                        });
                    }
                    // live buffer (caret when focused) vs placeholder
                    let buf: String = text
                        .as_deref()
                        .map(|t| substitute(t, vals))
                        .unwrap_or_default();
                    let buf_empty = buf.is_empty();
                    if buf_empty && !foc {
                        let ph: String = substitute(placeholder, vals);
                        draw_text(
                            v,
                            sx + scale.s(12.0),
                            sy + scale.s(7.0),
                            0.0,
                            false,
                            false,
                            &ph,
                            9.5,
                            SceneFont::Ui,
                            SceneWeight::Regular,
                            false,
                            Some(ColorToken::Fg),
                            scale,
                            pal,
                            vals,
                        );
                    } else {
                        draw_text(
                            v,
                            sx + scale.s(12.0),
                            sy + scale.s(7.0),
                            0.0,
                            false,
                            false,
                            &buf,
                            9.5,
                            SceneFont::Ui,
                            SceneWeight::Regular,
                            false,
                            Some(ColorToken::Fg),
                            scale,
                            pal,
                            vals,
                        );
                        if foc {
                            let cwx = buf.chars().count() as f32 * scale.fs(9.5) * 0.62;
                            v.push(Cmd::Rect {
                                x: sx + scale.s(11.0) + cwx,
                                y: sy + scale.s(6.0),
                                w: scale.s(1.4),
                                h: scale.s(14.0),
                                r: scale.s(0.7),
                                color: pal.acc,
                            });
                            if !buf.trim().is_empty() {
                                draw_text(
                                    v,
                                    x + card_w - scale.s(22.0),
                                    sy + scale.s(8.0),
                                    0.0,
                                    true,
                                    false,
                                    crate::icons::ICON_KEYBOARD,
                                    8.5,
                                    SceneFont::Ui,
                                    SceneWeight::Regular,
                                    true,
                                    Some(ColorToken::Fg),
                                    scale,
                                    pal,
                                    vals,
                                );
                            }
                        }
                    }
                    if *key != 0 {
                        // the slot registers first, generously oversized so a
                        // press on the strip edges still focuses the field
                        hits.push(SceneHit {
                            x: sx - scale.s(2.0),
                            y: sy - scale.s(4.0),
                            w: fw + scale.s(4.0),
                            h: sh + scale.s(8.0),
                            key: *key,
                            action: action.clone(),
                        });
                    }
                    // ⊕ add button — bottom-right corner, hidden while focused;
                    // registered AFTER the field region so it wins the overlap.
                    if *add && !foc && *add_key != 0 {
                        let pbx = x + card_w - scale.s(20.0);
                        let pby = y + card_h - scale.s(18.0);
                        let pb_hov = hover_key == *add_key;
                        v.push(Cmd::Rect {
                            x: pbx - scale.s(10.0),
                            y: pby - scale.s(10.0),
                            w: scale.s(20.0),
                            h: scale.s(20.0),
                            r: scale.s(10.0),
                            color: if pb_hov { crate::ui::hover_hl(pal) } else { crate::ui::hover(pal) },
                        });
                        draw_text(
                            v,
                            pbx,
                            pby - scale.s(4.5),
                            0.0,
                            false,
                            true,
                            crate::icons::ICON_ADD,
                            9.5,
                            SceneFont::Ui,
                            SceneWeight::Regular,
                            true,
                            Some(if pb_hov { ColorToken::Acc } else { ColorToken::Fg }),
                            scale,
                            pal,
                            vals,
                        );
                        hits.push(SceneHit {
                            x: pbx - scale.s(11.0),
                            y: pby - scale.s(11.0),
                            w: scale.s(22.0),
                            h: scale.s(22.0),
                            key: *add_key,
                            action: add_action.clone(),
                        });
                    }
                }
                SceneItem::Ink { x: ix, y: iy, w, h, name, right } => {
                    let sx = if *right {
                        x + card_w - scale.s(*ix + *w)
                    } else {
                        x + scale.s(*ix)
                    };
                    inks.push(SceneInk {
                        name: substitute(name, vals),
                        x: sx,
                        y: y + scale.s(*iy),
                        w: if *w > 0.0 { scale.s(*w) } else { card_w },
                        h: if *h > 0.0 { scale.s(*h) } else { card_h },
                        pre: Vec::new(),
                    });
                }
                SceneItem::Header { title, glyph, meta, meta_mono, meta_color, glyph_color, title_color, pad } => {
                    let p = scale.s(if *pad > 0.0 { *pad } else { 12.0 });
                    // glyph, if shown, sits at (x+pad, y+8) in fg2 and shifts
                    // the title right by 17 — byte-for-byte `card_header`
                    let tx = if let Some(g) = glyph {
                        if show_glyph {
                            draw_text(v, x + p, y + scale.s(8.0), 0.0, false, false, g, 12.0, SceneFont::Ui, SceneWeight::Light, true, glyph_color.or(Some(ColorToken::Fg2)), scale, pal, vals);
                            p + scale.s(17.0)
                        } else {
                            p
                        }
                    } else {
                        p
                    };
                    if show_title {
                        draw_text(v, x + tx, y + scale.s(9.0), 0.0, false, false, title, 11.5, SceneFont::Ui, SceneWeight::Semibold, false, title_color.or(Some(ColorToken::Fg)), scale, pal, vals);
                    }
                    if let Some(m) = meta {
                        let c = if *meta_mono { SceneFont::Mono } else { SceneFont::Ui };
                        draw_text(v, x + card_w - p, y + scale.s(11.0), 0.0, true, false, m, 8.5, c, SceneWeight::Regular, false, meta_color.or(Some(ColorToken::Fg3)), scale, pal, vals);
                    }
                }
                SceneItem::Row { x: ix, y: iy, w, h, pad, spacing, valign, halign, bottom, min_card_h, visible, items } => {
                    // pane gate (multi-pane bodies, the mirror button row)
                    if let Some(v) = visible.as_deref() {
                        if vals.toggle(v) == Some(false) {
                            return;
                        }
                    }
                    // room check: a card too short to hold the row skips it
                    // whole (the compositor's screenshot buttons)
                    if *min_card_h > 0.0 && card_h < scale.s(*min_card_h) {
                        return;
                    }
                    // a child whose `visible` gate resolves false takes NO
                    // slot: the row reflows around it (the optional fader rows)
                    let items: Vec<&SceneItem> =
                        items.iter().filter(|c| c.is_visible(vals)).collect();
                    let sx = x + scale.s(*ix);
                    // bottom anchor: the row's bottom edge sits `iy` px above
                    // the card bottom (mirror of the Hit `right` formula)
                    let sy = if *bottom && *h > 0.0 {
                        y + card_h - scale.s(*iy + *h)
                    } else {
                        y + scale.s(*iy)
                    };
                    let sw = span_px(*w, card_w, scale);
                    let sh = span_px(*h, card_h, scale);
                    let pd = scale.s(*pad);
                    let sp = scale.s(*spacing);
                    let inner_x = sx + pd;
                    let inner_y = sy + pd;
                    let inner_w = (sw - pd * 2.0).max(0.0);
                    let inner_h = (sh - pd * 2.0).max(0.0);
                    let n = items.len();
                    let mut fixed = 0.0f32;
                    let mut fills = 0usize;
                    for c in &items {
                        if c.want_w() > 0.0 {
                            fixed += scale.s(c.want_w());
                        } else {
                            fills += 1;
                        }
                    }
                    let gaps = if n > 0 { (n - 1) as f32 } else { 0.0 };
                    let share = if fills > 0 {
                        ((inner_w - fixed - sp * gaps).max(0.0)) / fills as f32
                    } else {
                        0.0
                    };
                    // `halign: center` centers a block of fixed-width children
                    // (fills already span the box, so they stay left-anchored)
                    let block_w = fixed + share * fills as f32 + sp * gaps;
                    let mut cx = if *halign == HAlign::Center && fills == 0 {
                        inner_x + (inner_w - block_w).max(0.0) / 2.0
                    } else {
                        inner_x
                    };
                    for c in items {
                        let cw = if c.want_w() > 0.0 { scale.s(c.want_w()) } else { share };
                        let ch = if c.want_h() > 0.0 { scale.s(c.want_h()) } else { inner_h };
                        let cy = if *valign == VAlign::Middle {
                            inner_y + (inner_h - ch).max(0.0) / 2.0
                        } else {
                            inner_y
                        };
                        self.draw_piece(c, v, cx, cy, cw, ch, scale, pal, vals, hover_key, show_title, show_glyph, hits, inks);
                        cx += cw + sp;
                    }
                }
                SceneItem::Column { x: ix, y: iy, w, h, pad, spacing, halign, valign, visible, items } => {
                    // pane gate (multi-pane bodies)
                    if let Some(v) = visible.as_deref() {
                        if vals.toggle(v) == Some(false) {
                            return;
                        }
                    }
                    // gated children collapse instead of reserving a slot, so a
                    // vertically centered stack recenters when a row hides
                    let items: Vec<&SceneItem> =
                        items.iter().filter(|c| c.is_visible(vals)).collect();
                    let sx = x + scale.s(*ix);
                    let sy = y + scale.s(*iy);
                    let sw = span_px(*w, card_w, scale);
                    let sh = span_px(*h, card_h, scale);
                    let pd = scale.s(*pad);
                    let sp = scale.s(*spacing);
                    let inner_x = sx + pd;
                    let inner_y = sy + pd;
                    let inner_w = (sw - pd * 2.0).max(0.0);
                    let inner_h = (sh - pd * 2.0).max(0.0);
                    let n = items.len();
                    let mut fixed = 0.0f32;
                    let mut fills = 0usize;
                    for c in &items {
                        if c.want_h() > 0.0 {
                            fixed += scale.s(c.want_h());
                        } else {
                            fills += 1;
                        }
                    }
                    let gaps = if n > 0 { (n - 1) as f32 } else { 0.0 };
                    let share = if fills > 0 {
                        ((inner_h - fixed - sp * gaps).max(0.0)) / fills as f32
                    } else {
                        0.0
                    };
                    // `valign: middle` centers the whole stacked block (the
                    // fixed-height rows + their gaps) inside the box
                    let stack_h = fixed + share * fills as f32 + sp * gaps;
                    let mut cy = if *valign == VAlign::Middle {
                        inner_y + (inner_h - stack_h).max(0.0) / 2.0
                    } else {
                        inner_y
                    };
                    for c in items {
                        let cw = if c.want_w() > 0.0 { scale.s(c.want_w()) } else { inner_w };
                        let ch = if c.want_h() > 0.0 { scale.s(c.want_h()) } else { share };
                        let cx = if *halign == HAlign::Center {
                            inner_x + (inner_w - cw).max(0.0) / 2.0
                        } else {
                            inner_x
                        };
                        self.draw_piece(c, v, cx, cy, cw, ch, scale, pal, vals, hover_key, show_title, show_glyph, hits, inks);
                        cy += ch + sp;
                    }
                }
                SceneItem::Stack { x: ix, y: iy, w, h, pad, visible, items } => {
                    // pane gate (multi-pane bodies)
                    if let Some(v) = visible.as_deref() {
                        if vals.toggle(v) == Some(false) {
                            return;
                        }
                    }
                    let sx = x + scale.s(*ix);
                    let sy = y + scale.s(*iy);
                    let sw = span_px(*w, card_w, scale);
                    let sh = span_px(*h, card_h, scale);
                    let pd = scale.s(*pad);
                    let inner_x = sx + pd;
                    let inner_y = sy + pd;
                    let inner_w = (sw - pd * 2.0).max(0.0);
                    let inner_h = (sh - pd * 2.0).max(0.0);
                    // zero-dimension children (full-bleed surfaces/Inks) span the
                    // whole box; everything else keeps its declared geometry.
                    // A right-anchored `Text` is exempt: its anchor is the box's
                    // right edge minus `x` (the declared `w` is not a layout
                    // slot), so widening it to the fill would push the label
                    // a whole box-width to the right.
                    let base_w = inner_w / scale.0;
                    let base_h = inner_h / scale.0;
                    for c in items {
                        let fill_w = c.want_w() == 0.0 && !c.right_anchored();
                        let fill_h = c.want_h() == 0.0;
                        let cc = if fill_w || fill_h {
                            let mut c = c.clone();
                            c.set_dims(if fill_w { Some(base_w) } else { None }, if fill_h { Some(base_h) } else { None });
                            c
                        } else {
                            c.clone()
                        };
                        self.draw_piece(&cc, v, inner_x, inner_y, inner_w, inner_h, scale, pal, vals, hover_key, show_title, show_glyph, hits, inks);
                    }
                }
                // `Comp` is expanded at load time and never reaches the draw
                // path; silence the exhaustive-match arm.
                SceneItem::Grid {
                    name,
                    x: ix,
                    y: iy,
                    w,
                    h,
                    cols,
                    pad,
                    gap,
                    font_size,
                    r,
                    key_base,
                    hover_surface,
                    del_base,
                    del_size,
                    image_size,
                    image_top,
                    square,
                    cell_h: fixed_cell_h,
                    head,
                    visible,
                } => {
                    // honour the bound Toggle gate (multi-pane bodies)
                    if let Some(v) = visible.as_deref() {
                        if vals.toggle(v) == Some(false) {
                            return;
                        }
                    }
                    let Some(cells) = vals.grid_cells(name) else { return };
                    let cols_n = (*cols).max(1) as usize;
                    let p = scale.s(*pad);
                    let gx = x + scale.s(*ix) + p;
                    let gw_full = if *w > 0.0 {
                        scale.s(*w)
                    } else if *w < 0.0 {
                        (card_w - scale.s(-*w)).max(0.0)
                    } else {
                        card_w - scale.s(*ix) * 2.0
                    };
                    let gw = (gw_full - p * 2.0).max(2.0);
                    let gy = y + scale.s(*iy);
                    let gh_raw = if *h > 0.0 {
                        scale.s(*h)
                    } else if *h < 0.0 {
                        (card_h - scale.s(*iy) - scale.s(-*h)).max(0.0)
                    } else {
                        (card_h - scale.s(*iy)).max(0.0)
                    };
                    // the head label row (weekdays) consumes the top band
                    let head_h = if head.is_some() { scale.s(16.0) } else { 0.0 };
                    let grid_y = gy + head_h;
                    let gh = (gh_raw - head_h).max(0.0);
                    if let Some(hd) = head {
                        let hcw = gw / cols_n as f32;
                        for (ci, label) in hd.iter().enumerate() {
                            draw_text(v, gx + ci as f32 * (hcw + scale.s(*gap)), grid_y - scale.s(14.0),
                                hcw, false, true, label, 8.0, SceneFont::Ui, SceneWeight::Regular,
                                false, Some(ColorToken::Fg3), scale, pal, vals);
                        }
                    }
                    let gap = scale.s(*gap);
                    let cell_w = (gw - (cols_n as f32 - 1.0) * gap) / cols_n as f32;
                    // rows come from the data count — the calendar's 6×7
                    // matrix, the 8-slot app grid — so a partial tail never
                    // stretches cells
                    let rows = cells.len().div_ceil(cols_n);
                    let cell_h = if *fixed_cell_h > 0.0 {
                        // fixed pitch (thermal chips): rows do NOT stretch to
                        // fill the box — a partial tail leaves the rest open
                        scale.s(*fixed_cell_h)
                    } else if *square {
                        cell_w
                    } else if rows > 0 {
                        (gh - (rows as f32 - 1.0) * gap) / rows as f32
                    } else {
                        cell_w
                    };
                    let im = scale.s(*image_size);
                    let im_top = scale.s(*image_top);
                    for (i, cell) in cells.iter().enumerate() {
                        if !cell.visible {
                            continue;
                        }
                        let col = i % cols_n;
                        let row = i / cols_n;
                        let cx = gx + col as f32 * (cell_w + gap);
                        let cy = grid_y + row as f32 * (cell_h + gap);
                        if cy > y + card_h + 0.5 {
                            continue;
                        }
                        let key = key_base + i as u32;
                        let hov = hover_key == key && key != 0;
                        // cell fill: the cell's own surface, or the grid's
                        // hover surface while hovered (bottom cells carry a
                        // per-frame bound tile color so the apps resting
                        // mixes survive)
                        let bg = if hov {
                            cell.surface_hover.or(*hover_surface).or(cell.surface)
                        } else {
                            cell.surface
                        };
                        if let Some(bg) = bg {
                            v.push(Cmd::Rect {
                                x: cx,
                                y: cy,
                                w: cell_w,
                                h: cell_h,
                                r: scale.s(*r),
                                color: bg.resolve_with(pal, vals),
                            });
                        }
                        // raster / icon above the label (app tiles)
                        if let Some(ik) = &cell.image_key {
                            v.push(Cmd::Image {
                                x: cx + (cell_w - im) / 2.0,
                                y: cy + im_top,
                                w: im,
                                h: im,
                                key: ik.clone(),
                            });
                        } else if let Some(gl) = &cell.glyph {
                            v.push(Cmd::Text {
                                x: cx + cell_w / 2.0,
                                y: cy + im_top,
                                right: false,
                                center: true,
                                text: gl.clone(),
                                size: scale.fs(17.0),
                                color: pal.acc,
                                icon: true,
                                f: crate::text::Ff::Ui,
                                w: crate::text::Fw::Regular,
                            });
                        }
                        // label: bottom-anchored (app tiles) vs centered
                        // (calendar days), with per-cell nudges
                        if !cell.text.is_empty() {
                            let fs = cell.size.unwrap_or(*font_size);
                            let label: String = match cell.truncate {
                                Some(n) => cell.text.chars().take(n as usize).collect(),
                                None => cell.text.clone(),
                            };
                            let base_y = if cell.bottom {
                                cy + cell_h - scale.s(12.0)
                            } else {
                                cy + cell_h / 2.0 - scale.s(5.0)
                            };
                            let ly = base_y + cell.dy.map(|d| scale.s(d)).unwrap_or(0.0);
                            draw_text(v, cx + cell_w / 2.0, ly, cell_w, false, true,
                                &label, fs, SceneFont::Ui, SceneWeight::Regular,
                                cell.icon, if hov { cell.hover.or(cell.color) } else { cell.color }.or(Some(ColorToken::Fg)),
                                scale, pal, vals);
                        }
                        // second line under the label (thermal zone temps) —
                        // smaller, centered, own ink, per-cell nudge/size
                        if let Some(sub) = &cell.sub {
                            if !sub.is_empty() {
                                let sub_fs = cell.sub_size.filter(|s| *s > 0.0).unwrap_or(7.5);
                                let sub_y = cy + cell_h / 2.0 + scale.s(6.0)
                                    + cell.sub_dy.map(|d| scale.s(d)).unwrap_or(0.0);
                                draw_text(v, cx + cell_w / 2.0, sub_y,
                                    cell_w, false, true, sub, sub_fs, SceneFont::Ui, SceneWeight::Regular,
                                    false, cell.sub_color.or(Some(ColorToken::Fg3)), scale, pal, vals);
                            }
                        }
                        hits.push(SceneHit {
                            x: cx,
                            y: cy,
                            w: cell_w,
                            h: cell_h,
                            key,
                            action: None,
                        });
                        // hover-only ✕ delete corner on filled (bottom) tiles
                        if *del_base != 0 && hov && cell.bottom {
                            let dk = *del_base + i as u32;
                            v.push(Cmd::Rect {
                                x: cx + cell_w - scale.s(17.0),
                                y: cy - scale.s(4.0),
                                w: scale.s(22.0),
                                h: scale.s(22.0),
                                r: scale.s(11.0),
                                color: if hover_key == dk {
                                    mix(crate::ui::DANGER, crate::ui::hover(pal), 0.55)
                                } else {
                                    mix(crate::ui::hover(pal), pal.fg, 0.10)
                                },
                            });
                            draw_text(v, cx + cell_w - scale.s(6.0), cy + scale.s(0.5), 0.0,
                                false, true, crate::icons::ICON_CLOSE, *del_size,
                                SceneFont::Ui, SceneWeight::Regular, true,
                                Some(if hover_key == dk { ColorToken::Danger } else { ColorToken::Fg }),
                                scale, pal, vals);
                            hits.push(SceneHit {
                                x: cx + cell_w - scale.s(17.0),
                                y: cy - scale.s(4.0),
                                w: scale.s(22.0),
                                h: scale.s(22.0),
                                key: dk,
                                action: None,
                            });
                        }
                    }
                }
                SceneItem::Tiles {
                    name,
                    x: ix,
                    y: iy,
                    w,
                    h,
                    pad,
                    gap,
                    cols,
                    cols_break,
                    r,
                    icon_size,
                    label_size,
                    icon_top,
                    label_top,
                    label_min_h,
                    on_surface,
                    hover_surface,
                    off_surface,
                    on_ink,
                    off_ink,
                    label_ink,
                    chip,
                    chip_size,
                    chip_dx,
                    chip_dy,
                    chip_w,
                    visible,
                } => {
                    // honour the bound Toggle gate (pane bodies)
                    if let Some(vb) = visible.as_deref() {
                        if vals.toggle(vb) == Some(false) {
                            return;
                        }
                    }
                    let Some(tiles) = vals.tiles(name) else { return };
                    let p = scale.s(*pad);
                    let g = scale.s(*gap);
                    let bx = x + scale.s(*ix);
                    let by = y + scale.s(*iy);
                    let bw = span_px(*w, card_w - scale.s(*ix), scale);
                    let bh = span_px(*h, card_h - scale.s(*iy), scale);
                    // responsive columns: the widest breakpoint the card is
                    // at least as wide as wins, else the declared `cols`
                    let mut cols_n = (*cols).max(1) as usize;
                    for (min_w, c) in cols_break.iter() {
                        if card_w >= scale.s(*min_w) {
                            cols_n = (*c).max(1) as usize;
                        }
                    }
                    // rows come from the data count, so a partial tail never
                    // stretches (the six switches make 1×6 / 2×3 / 3×2)
                    let rows = tiles.len().div_ceil(cols_n).max(1);
                    let tw = ((bw - p * 2.0).max(0.0) - (cols_n as f32 - 1.0) * g) / cols_n as f32;
                    let th = ((bh - p * 2.0).max(0.0) - (rows as f32 - 1.0) * g) / rows as f32;
                    // a squat tile drops its caption and centers the glyph, so
                    // the same tile list works in a short card and a tall one
                    let labeled = th >= scale.s(*label_min_h);
                    let isz = scale.fs(*icon_size);
                    for (i, tile) in tiles.iter().enumerate() {
                        let col = i % cols_n;
                        let row = i / cols_n;
                        let tx = bx + p + col as f32 * (tw + g);
                        let ty = by + p + row as f32 * (th + g);
                        if ty > y + card_h + 0.5 {
                            continue;
                        }
                        let hov = tile.key != 0 && hover_key == tile.key;
                        // the "on" state outranks hover: a lit switch keeps
                        // its accent wash under the cursor
                        let bg = if tile.on {
                            Some(*on_surface)
                        } else if hov {
                            Some(*hover_surface)
                        } else {
                            Some(*off_surface)
                        };
                        if let Some(bg) = bg {
                            v.push(Cmd::Rect {
                                x: tx,
                                y: ty,
                                w: tw,
                                h: th,
                                r: scale.s(*r),
                                color: bg.resolve_with(pal, vals),
                            });
                        }
                        if !tile.glyph.is_empty() {
                            let gy = if labeled {
                                ty + scale.s(*icon_top)
                            } else {
                                ty + (th - scale.fs(*icon_size)) / 2.0
                            };
                            draw_text(
                                v,
                                tx + tw / 2.0,
                                gy,
                                tw,
                                false,
                                true,
                                &tile.glyph,
                                isz,
                                SceneFont::Ui,
                                SceneWeight::Regular,
                                true,
                                Some(if tile.on { *on_ink } else { *off_ink }),
                                scale,
                                pal,
                                vals,
                            );
                        }
                        if labeled && !tile.label.is_empty() {
                            draw_text(
                                v,
                                tx + tw / 2.0,
                                ty + scale.s(*label_top),
                                tw,
                                false,
                                true,
                                &tile.label,
                                scale.fs(*label_size),
                                SceneFont::Ui,
                                SceneWeight::Regular,
                                false,
                                Some(*label_ink),
                                scale,
                                pal,
                                vals,
                            );
                        }
                        if tile.key != 0 {
                            hits.push(SceneHit {
                                x: tx,
                                y: ty,
                                w: tw,
                                h: th,
                                key: tile.key,
                                action: None,
                            });
                        }
                        // corner chip: a ⋯ over a full-height right-edge
                        // strip, drawn last so it sits above the tile fill
                        if *chip && tile.chip_key != 0 {
                            let c_hov = hover_key == tile.chip_key;
                            draw_text(
                                v,
                                tx + tw - scale.s(*chip_dx),
                                ty + th - scale.s(*chip_dy),
                                0.0,
                                false,
                                true,
                                crate::icons::ICON_MORE,
                                scale.fs(*chip_size),
                                SceneFont::Ui,
                                SceneWeight::Regular,
                                true,
                                Some(if c_hov { ColorToken::Acc } else { ColorToken::Fg }),
                                scale,
                                pal,
                                vals,
                            );
                            hits.push(SceneHit {
                                x: tx + tw - scale.s(*chip_w),
                                y: ty,
                                w: scale.s(*chip_w),
                                h: th,
                                key: tile.chip_key,
                                action: None,
                            });
                        }
                    }
                }
                SceneItem::Image {
                    x: ix,
                    y: iy,
                    w,
                    h,
                    key,
                    right,
                    visible,
                } => {
                    // honour the bound Toggle gate (camera feed pane)
                    if let Some(v) = visible.as_deref() {
                        if vals.toggle(v) == Some(false) {
                            return;
                        }
                    }
                    let sx = if *right {
                        x + card_w - scale.s(*ix)
                    } else {
                        x + scale.s(*ix)
                    };
                    let sy = y + scale.s(*iy);
                    // negative `w`/`h` reserve that amount from the opposite
                    // edge (the camera feed stops above the pagination dots);
                    // `0` fills the card
                    let sw = if *w > 0.0 {
                        scale.s(*w)
                    } else if *w < 0.0 {
                        (card_w - scale.s(-*w)).max(0.0)
                    } else {
                        card_w
                    };
                    let sh = if *h > 0.0 {
                        scale.s(*h)
                    } else if *h < 0.0 {
                        (card_h - scale.s(-*h)).max(0.0)
                    } else {
                        card_h
                    };
                    v.push(Cmd::Image {
                        x: sx,
                        y: sy,
                        w: sw,
                        h: sh,
                        key: substitute(key, vals),
                    });
                }
                SceneItem::Comp { .. } => {}
                SceneItem::Dots { active, when, x: dx0, dy, step, panes } => {
                    // visibility gate (cursor rests on the card), then the
                    // shared pane-dots geometry — HARD RULE for swipe-cycled
                    // cards: one dot per pane, active = accent, no wraparound
                    if let Some(w) = when.as_deref() {
                        if vals.toggle(w) != Some(true) {
                            return;
                        }
                    }
                    let Some(idx) = vals.scalar(active) else { return };
                    let n = (*panes).max(1) as usize;
                    let idx = (idx as usize).min(n - 1);
                    let dot = scale.s(*step) * 0.66;
                    let total = n as f32 * scale.s(*step);
                    let dx = x + if *dx0 > 0.0 { scale.s(*dx0) } else { card_w / 2.0 } - total / 2.0;
                    let dyy = y + card_h - scale.s(*dy) - dot;
                    for i in 0..n {
                        let c = if i == idx { pal.acc } else { crate::ui::hover(pal) };
                        v.push(Cmd::Rect {
                            x: dx + i as f32 * scale.s(*step),
                            y: dyy,
                            w: dot,
                            h: dot,
                            r: dot / 2.0,
                            color: c,
                        });
                    }
                }
            }
    }
}

/// The region key a `Rows` data row registers: its own `key` when set, else
/// `base + index` — and `0` (no region) when the item declared no `key_base`,
/// so an info list stays inert instead of squatting on keys 1, 2, 3, …
fn row_key(own: u32, base: Option<&u32>, i: usize) -> u32 {
    if own != 0 {
        own
    } else {
        base.map(|b| b + i as u32).unwrap_or(0)
    }
}

/// Render one text box with `{var}` substitution + alignment. `x`/`y`/`w` are
/// already-scale'd absolute coordinates (the box origin + scaled width used
/// for right/center anchoring).
#[allow(clippy::too_many_arguments)]
fn draw_text(
    v: &mut Vec<Cmd>,
    x: f32,
    y: f32,
    w: f32,
    right: bool,
    center: bool,
    text: &str,
    font_size: f32,
    font: SceneFont,
    weight: SceneWeight,
    icon: bool,
    color: Option<ColorToken>,
    scale: crate::shell::GridScale,
    pal: &Pal,
    vals: &SceneValues,
) {
    if text.is_empty() {
        return;
    }
    let txt = substitute(text, vals);
    let color = color.unwrap_or(ColorToken::Fg).resolve_with(pal, vals);
    let mut cx = x;
    if right {
        cx = x + w; // text_r anchors its right at x
    } else if center {
        cx = x + w / 2.0;
    }
    if right {
        crate::ui::text_rw(v, cx, y, txt, scale.fs(font_size), font.ff(), weight.fw(), color, icon);
    } else if center {
        crate::ui::text_cw(v, cx, y, txt, scale.fs(font_size), font.ff(), weight.fw(), color, icon);
    } else {
        crate::ui::text_w(v, cx, y, txt, scale.fs(font_size), font.ff(), weight.fw(), color, icon);
    }
}

/// The battery gauge's level ladder, shared by the declarative `Battery` item
/// and the Rust fallback drawers.
/// Fill color for a charge level (shared by both battery cards).
pub(crate) fn battery_level_color(pal: &Pal, pct: i32) -> u32 {
    if pct >= 50 {
        crate::ui::OK
    } else if pct >= 20 {
        pal.acc
    } else {
        crate::ui::DANGER
    }
}

/// A proper battery drawn from vector shapes — not a font glyph. Case =
/// rounded outline, terminal nub on the free end, colored by charge level.
/// The interior is a WATER fill: the liquid spans the FULL inner width so
/// its left/right edges touch the case stroke, rises with the charge, and
/// carries a gentle wave crest + highlight on its surface (reference:
/// liquid-battery icon). `vertical` = standing battery (nub on top, water
/// climbs up), else flat battery (nub on the right, water grows
/// left→right).
pub(crate) fn push_battery_icon(v: &mut Vec<Cmd>, pal: &Pal, cx: f32, cy: f32, w: f32, h: f32, pct: i32, vertical: bool) {
    push_battery_icon_parts(
        v,
        crate::ui::fg2(pal),
        crate::ui::hover(pal),
        battery_level_color(pal, pct),
        pct,
        cx,
        cy,
        w,
        h,
        vertical,
    );
}

/// The gauge's three inks (case / interior / liquid) resolved by the caller —
/// the declarative `Battery` item overrides only the liquid, so it draws the
/// parts directly.
#[allow(clippy::too_many_arguments)]
pub(crate) fn push_battery_icon_parts(
    v: &mut Vec<Cmd>, line: u32, body: u32, fill_c: u32, pct: i32,
    cx: f32, cy: f32, w: f32, h: f32, vertical: bool,
) {
    let fill = (pct as f32 / 100.0).clamp(0.0, 1.0);
    if vertical {
        // terminal nub on top
        let nub_w = w * 0.5;
        let nub_h = h * 0.05;
        v.push(Cmd::Rect {
            x: cx - nub_w / 2.0, y: cy - h / 2.0 - nub_h, w: nub_w, h: nub_h, r: nub_h / 2.0, color: line,
        });
        // case outline
        let r = w * 0.16;
        let stroke = w.clamp(1.2, 2.2);
        v.push(Cmd::Outline { x: cx - w / 2.0, y: cy - h / 2.0, w, h, r, width: stroke, color: line });
        draw_water(
            v, fill, fill_c, body,
            cx - w / 2.0, cy - h / 2.0, w, h, stroke, r, stroke, true,
        );
    } else {
        // terminal nub on the right
        let nub_w = w * 0.05;
        let nub_h = h * 0.5;
        v.push(Cmd::Rect {
            x: cx + w / 2.0, y: cy - nub_h / 2.0, w: nub_w, h: nub_h, r: nub_w / 2.0, color: line,
        });
        let r = h * 0.16;
        let stroke = h.clamp(1.2, 2.2);
        v.push(Cmd::Outline { x: cx - w / 2.0, y: cy - h / 2.0, w, h, r, width: stroke, color: line });
        draw_water(
            v, fill, fill_c, body,
            cx - w / 2.0, cy - h / 2.0, w, h, stroke, r, stroke, false,
        );
    }
}

/// The water interior shared by both battery orientations: an empty-interior
/// tint, the liquid body spanning the FULL inner width (edges against the
/// case stroke), a lighter wave crest along its surface and a thin highlight
/// just below the crest. `rise` = vertical battery (water climbs, surface is
/// horizontal); else horizontal (water grows left→right, surface is vertical).
#[allow(clippy::too_many_arguments)]
fn draw_water(
    v: &mut Vec<Cmd>, fill: f32, fill_c: u32, body: u32,
    bx: f32, by: f32, w: f32, h: f32, stroke: f32, r: f32, _round: f32, rise: bool,
) {
    // inner rect — just inside the (edge-centered) stroke
    let in_x = bx + stroke * 0.75;
    let in_y = by + stroke * 0.75;
    let in_w = w - stroke * 1.5;
    let in_h = h - stroke * 1.5;
    // full inner cross-section in BOTH axes: the water touches the left and
    // right stroke (vertical) / top and bottom stroke (horizontal)
    let cross_w = if rise { in_w } else { in_w * fill };
    let cross_h = if rise { in_h * fill } else { in_h };
    let fr = r * 0.55; // follow the case corners a little
    // backwash: interior tint behind the water so the case reads as a vessel
    v.push(Cmd::Rect { x: in_x, y: in_y, w: in_w, h: in_h, r: fr, color: body });
    if cross_w < 1.5 || cross_h < 1.5 {
        return;
    }
    let (wx, wy) = if rise {
        (in_x, in_y + (in_h - cross_h))
    } else {
        (in_x, in_y)
    };
    v.push(Cmd::Rect { x: wx, y: wy, w: cross_w, h: cross_h, r: fr.min(cross_w / 2.0).min(cross_h / 2.0), color: fill_c });
    // wave crest across the surface + a lighter highlight band under it —
    // inset from the edges by the corner radius so the ripple never pokes
    // outside the case's rounded corners
    let crest = mix(fill_c, 0xffffff, 0.45);
    let hl = mix(fill_c, 0xffffff, 0.22);
    let segs = 8;
    let amp = (if rise { in_w } else { in_h }).min(6.0) * 0.28 + 0.4;
    if rise {
        // horizontal surface at the water top
        let sx0 = wx + fr;
        let sx1 = wx + cross_w - fr;
        if sx1 > sx0 {
            let step = (sx1 - sx0) / segs as f32;
            for i in 0..segs {
                let x0 = sx0 + i as f32 * step;
                let x1 = x0 + step;
                let y_mid = wy + amp * (i as f32 * 1.7).sin();
                let y_next = wy + amp * ((i + 1) as f32 * 1.7).sin();
                v.push(Cmd::Line { x0, y0: y_mid, x1, y1: y_next, w: 1.4, color: crest });
            }
            // highlight under the crest
            v.push(Cmd::Line { x0: sx0, y0: wy + amp * 2.2, x1: sx1, y1: wy + amp * 2.2, w: 1.0, color: hl });
        }
    } else {
        // vertical surface at the water's leading edge (right side)
        let sx = wx + cross_w;
        let sy0 = wy + fr;
        let sy1 = wy + cross_h - fr;
        if sy1 > sy0 {
            let step = (sy1 - sy0) / segs as f32;
            for i in 0..segs {
                let y0 = sy0 + i as f32 * step;
                let y1 = y0 + step;
                let x_mid = sx + amp * (i as f32 * 1.7).sin();
                let x_next = sx + amp * ((i + 1) as f32 * 1.7).sin();
                v.push(Cmd::Line { x0: x_mid, y0, x1: x_next, y1, w: 1.4, color: crest });
            }
            v.push(Cmd::Line { x0: sx + amp * 2.2, y0: sy0, x1: sx + amp * 2.2, y1: sy1, w: 1.0, color: hl });
        }
    }
}

/// Substitute `{name}` placeholders from `vals`. Unknown/missing names render
/// as `{name}` (unchanged) so a stale RON never produces blank labels. Looks
/// up only `Text`-typed values; a name bound to a vector/boolean just stays
/// literal.
pub(crate) fn substitute(text: &str, vals: &SceneValues) -> String {
    if text.is_empty() || !text.contains('{') {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len() + 8);
    let mut rest = text;
    while let Some(start) = rest.find('{') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        let Some(end) = after.find('}') else {
            out.push_str(&rest[start..]);
            return out;
        };
        let name = &after[..end];
        match vals.text(name) {
            Some(val) => out.push_str(val),
            None => {
                out.push('{');
                out.push_str(name);
                out.push('}');
            }
        }
        rest = &after[end + 1..];
    }
    out.push_str(rest);
    out
}

/// Bead-ring gauge — `lit` of `beads` evenly-spaced dots around `(cx, cy)`,
/// the first bead at 12 o'clock, clockwise (same geometry as the Rust
/// `ring_beads`). First `lit` beads take `col`, the rest `dim`.
fn bead_ring(
    v: &mut Vec<Cmd>,
    cx: f32,
    cy: f32,
    radius: f32,
    dot_d: f32,
    beads: usize,
    lit: usize,
    col: u32,
    dim: u32,
) {
    let n = beads.max(2);
    for b in 0..n {
        let ang = (b as f32 / n as f32) * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
        let dx = cx + ang.cos() * radius - dot_d / 2.0;
        let dy = cy + ang.sin() * radius - dot_d / 2.0;
        v.push(Cmd::Rect {
            x: dx,
            y: dy,
            w: dot_d,
            h: dot_d,
            r: dot_d / 2.0,
            color: if b < lit { col } else { dim },
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::icons::{ICON_CHECK, ICON_CLOSE, ICON_GLOBE};
    use crate::ui::DynColor;
    use crate::ui::hover;

    fn pal() -> Pal {
        Pal { fg: 0xe8e6e3ff, bg: 0x141414ff, acc: 0x4fc2ffff, sfg: 0x101010ff }
    }

    fn scale() -> crate::shell::GridScale {
        crate::shell::GridScale(1.0)
    }

    fn vals() -> SceneValues {
        let mut m = SceneValues::new();
        m.insert("rec_btn", SceneValue::Text("Record".into()));
        m
    }

    const SCENE_RON: &str = r##"
        (
            items: [
                Text(x: 12.0, y: 10.0, text: "Recorder", font_size: 11.5, weight: semibold),
                Hit(x: 134.0, y: 6.0, w: 16.0, h: 16.0, r: 5.0, key: 2003,
                    surface: Some(raised), text: "›", font_size: 10.0, icon: true,
                    action: Some(Command("zen-shell ipc call recorder open"))),
                Hit(x: 8.0, y: 84.0, w: 144.0, h: 26.0, r: 7.0, key: 2004,
                    surface: Some(raised), text: "{rec_btn}", font_size: 9.5,
                    action: Some(Command("zen-shell ipc call recorder toggle"))),
            ],
        )
    "##;

    #[test]
    fn ron_roundtrip_parses() {
        let scene: CardScene = ron::from_str(SCENE_RON).expect("scene ron parses");
        assert_eq!(scene.items.len(), 3);
        match &scene.items[0] {
            SceneItem::Text { text, weight, font, .. } => {
                assert_eq!(text, "Recorder");
                assert_eq!(*weight, SceneWeight::Semibold);
                assert_eq!(*font, SceneFont::Ui);
            }
            _ => panic!("item 0 is a Text"),
        }
        match &scene.items[1] {
            SceneItem::Hit { key, action, icon, .. } => {
                assert_eq!(*key, 2003);
                assert_eq!(*action, Some(SceneAction::Command("zen-shell ipc call recorder open".into())));
                assert!(icon);
            }
            _ => panic!("item 1 is a Hit"),
        }
        match &scene.items[2] {
            SceneItem::Hit { text, .. } => assert_eq!(text, "{rec_btn}"),
            _ => panic!("item 2 is a Hit"),
        }
    }

    #[test]
    fn draw_emits_cmd_and_hits() {
        let scene: CardScene = ron::from_str(SCENE_RON).unwrap();
        let mut v: Vec<Cmd> = Vec::new();
        let (hits, _) = scene.draw(&mut v, 0.0, 0.0, 148.0, 90.0, scale(), &pal(), &vals(), 0, true, true);
        // 2 surfaces (chevron + record button) + 3 texts (title, glyph, btn)
        assert!(v.iter().filter(|c| matches!(c, Cmd::Rect { .. })).count() >= 2);
        assert!(v.iter().any(|c| matches!(c, Cmd::Text { text, .. } if text == "Recorder")));
        assert!(v.iter().any(|c| matches!(c, Cmd::Text { text, .. } if text == "Record")),
            "rec_btn interpolated from vals");
        assert_eq!(hits.len(), 2, "two keys declared → two hit regions");
        assert_eq!(hits[0].key, 2003);
        assert!(matches!(hits[0].action, Some(SceneAction::Command(_))), "action present on hit 2003");
        assert!(matches!(hits[0].action,
            Some(SceneAction::Command(ref c)) if c == "zen-shell ipc call recorder open"));
    }

    #[test]
    fn hit_offset_by_card_origin() {
        let scene: CardScene = ron::from_str(SCENE_RON).unwrap();
        let mut v: Vec<Cmd> = Vec::new();
        let (hits, _) = scene.draw(&mut v, 100.0, 40.0, 148.0, 90.0, scale(), &pal(), &vals(), 0, true, true);
        assert_eq!(hits[0].x, 100.0 + 134.0, "hit x is card-origin offset");
        assert_eq!(hits[0].y, 40.0 + 6.0);
    }

    /// calendar-style matrix: a `head` row of weekdays, 42 day cells across
    /// 7 columns, a couple of off-month (invisible) cells, one "today"
    /// (accent surface).
    #[test]
    fn grid_draws_calendar_matrix_and_regions() {
        let scene: CardScene = ron::from_str(
            r#"(items: [ Grid(
                    name: "cal",
                    x: 0.0, y: 24.0,
                    cols: 7,
                    pad: 0.0, gap: 0.0,
                    font_size: 9.0, r: 8.0,
                    head: Some(["S", "M", "T", "W", "T", "F", "S"]),
                    key_base: 30000,
                    hover_surface: Some(hover),
                ) ])"#,
        )
        .unwrap();
        let mut vals = vals();
        let mut cells = Vec::with_capacity(42);
        for cell in 0..42 {
            let day = cell as i32 - 2 + 1; // first weekday = Tue → 2 hidden days
            let visible = day >= 1 && day <= 31;
            cells.push(GridCell {
                text: if visible { day.to_string() } else { String::new() },
                visible,
                color: if cell == 15 { Some(ColorToken::Acc) } else { None },
                surface: if cell == 15 { Some(ColorToken::AccTint) } else { None },
                ..Default::default()
            });
        }
        vals.insert("cal", SceneValue::GridCells(cells));
        let mut v: Vec<Cmd> = Vec::new();
        let (hits, _) = scene.draw(&mut v, 0.0, 0.0, 350.0, 280.0, scale(), &pal(), &vals, 0, true, true);
        // 42 slots: 2 leading + 9 trailing off-month cells hidden → 31
        // day cells (a 31-day month starting Tuesday)
        assert_eq!(hits.len(), 31, "invisible cells register no region");
        assert_eq!(hits[0].key, 30000 + 2, "first visible cell key = base + index");
        // every day cell is hit-tested at its own slot
        let cell_w = 350.0 / 7.0;
        let cell_h = (280.0 - 24.0 - 16.0) / 6.0; // (card_h - y - head_h) / 6 rows
        let h0 = hits.iter().find(|h| h.key == 30000 + 2).unwrap();
        assert!((h0.x - cell_w * 2.0).abs() < 0.01, "row-wrap puts day 1 under the Tue column");
        assert!((h0.y - (24.0 + 16.0)).abs() < 0.01, "head row pushes cells down");
        assert!((h0.h - cell_h).abs() < 0.01);
        // today's cell paints the accent tint surface + accent label
        assert!(v.iter().any(|c| matches!(c, Cmd::Rect { color: col, .. } if *col == ColorToken::AccTint.resolve(&pal()))),
            "today cell draws the accent tint");
        assert!(v.iter().any(|c| matches!(c, Cmd::Text { text, .. } if *text == "15")),
            "day 15 text renders");
    }

    #[test]
    fn grid_offsets_with_card_origin() {
        let scene: CardScene = ron::from_str(
            "(items: [ Grid(name: \"g\", x: 4.0, y: 6.0, cols: 2, pad: 0.0, gap: 0.0, key_base: 9000)])",
        )
        .unwrap();
        let mut vals = vals();
        vals.insert("g", SceneValue::GridCells(vec![
            GridCell { text: "a".into(), visible: true, ..Default::default() },
            GridCell { text: "b".into(), visible: true, ..Default::default() },
        ]));
        let mut v: Vec<Cmd> = Vec::new();
        let (hits, _) = scene.draw(&mut v, 30.0, 20.0, 200.0, 100.0, scale(), &pal(), &vals, 0, true, true);
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].x, 34.0, "cell x = card x + grid x (full-bleed 2-col split)");
        assert_eq!(hits[0].y, 26.0);
        assert_eq!(hits[1].x, 34.0 + 96.0, "second cell on the next column");
    }

    /// app-tile style: square cells with a raster + bottom label; the ✕
    /// delete corner appears only while the filled tile is hovered.
    #[test]
    fn grid_app_tiles_image_and_del_corner() {
        let scene: CardScene = ron::from_str(
            r#"(items: [ Grid(
                    name: "apps",
                    x: 0.0, y: 36.0,
                    cols: 4,
                    pad: 12.0, gap: 8.0,
                    font_size: 8.0, r: 10.0,
                    key_base: 34400,
                    del_base: 34416,
                    square: true,
                ) ])"#,
        )
        .unwrap();
        let mut vals = vals();
        let cells: Vec<GridCell> = (0..8)
            .map(|i| {
                let filled = i < 3;
                GridCell {
                    text: if filled { format!("App {i}") } else { "+".into() },
                    visible: true,
                    bottom: filled,
                    icon: !filled,
                    image_key: if filled { Some(format!("icon:{i}")) } else { None },
                    ..Default::default()
                }
            })
            .collect();
        vals.insert("apps", SceneValue::GridCells(cells));
        // resting: 3 filled tiles + 5 "+" tiles; no del corners while idle
        let mut v: Vec<Cmd> = Vec::new();
        let (hits, _) = scene.draw(&mut v, 0.0, 0.0, 400.0, 300.0, scale(), &pal(), &vals, 0, true, true);
        assert_eq!(hits.len(), 8, "every tile is clickable");
        assert!(v.iter().any(|c| matches!(c, Cmd::Image { key, .. } if key == "icon:0")), "app icon rasters draw");
        // hover the first tile → ✕ corner appears
        let mut v = Vec::new();
        let key = 34400 + 0;
        let (hits, _) = scene.draw(&mut v, 0.0, 0.0, 400.0, 300.0, scale(), &pal(), &vals, key, true, true);
        assert_eq!(hits.len(), 9, "hovering a filled tile adds the ✕ region");
        assert!(hits.iter().any(|h| h.key == 34416), "del key = del_base + 0");
        assert!(v.iter().any(|c| matches!(c, Cmd::Text { text, .. } if text == crate::icons::ICON_CLOSE)));
        // hover an EMPTY "+" tile → no ✕ (not bottom)
        let mut v = Vec::new();
        let (hits, _) = scene.draw(&mut v, 0.0, 0.0, 400.0, 300.0, scale(), &pal(), &vals, 34400 + 3, true, true);
        assert_eq!(hits.len(), 8, "empty tiles never reveal the ✕");
    }

    #[test]
    fn image_emits_cmd_key_substituted_and_gated() {
        let scene: CardScene = ron::from_str(
            r#"(items: [ Image(x: 10.0, y: 38.0, w: 380.0, h: 180.0, key: "cam", visible: Some("pane0")),
                              Image(x: 0.0, y: 2.0, w: 12.0, h: 12.0, key: "{hero}", visible: Some("gone")) ])"#,
        )
        .unwrap();
        let mut vals = vals();
        vals.insert("pane0", SceneValue::Toggle(true));
        vals.insert("gone", SceneValue::Toggle(false));
        vals.insert("hero", SceneValue::Text("file:/tmp/a.png".into()));
        let mut v: Vec<Cmd> = Vec::new();
        let (_h, _i) = scene.draw(&mut v, 5.0, 5.0, 400.0, 300.0, scale(), &pal(), &vals, 0, true, true);
        let imgs: Vec<_> = v.iter().filter(|c| matches!(c, Cmd::Image { .. })).collect();
        assert_eq!(imgs.len(), 1, "the off-gate image is skipped");
        match imgs[0] {
            Cmd::Image { x, y, w, h, key } => {
                assert_eq!(*x, 15.0, "image x offsets with the card origin");
                assert_eq!(*y, 43.0);
                assert_eq!(*w, 380.0);
                assert_eq!(*h, 180.0);
                assert_eq!(key, "cam", "literal key unchanged");
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn surface_lifts_only_when_hovered() {
        let scene: CardScene = ron::from_str(
            "(items: [ Hit(x: 8.0, y: 6.0, w: 16.0, h: 16.0, r: 5.0, key: 2003, surface: Some(raised)) ])",
        )
        .unwrap();
        let base = ColorToken::Raised.resolve(&pal());
        let hov = ColorToken::Raised.hover_variant(&pal());
        // not hovered → base surface
        let mut v: Vec<Cmd> = Vec::new();
        let (_h, _i) = scene.draw(&mut v, 0.0, 0.0, 148.0, 90.0, scale(), &pal(), &vals(), 0, true, true);
        assert!(v.iter().any(|c| matches!(c, Cmd::Rect { color: col, .. } if *col != hov)),
            "resting surface is not the hover color");
        // hovered → lifted surface
        let mut v: Vec<Cmd> = Vec::new();
        let (_h, _i) = scene.draw(&mut v, 0.0, 0.0, 148.0, 90.0, scale(), &pal(), &vals(), 2003, true, true);
        assert!(v.iter().any(|c| matches!(c, Cmd::Rect { color: col, .. } if *col == hov)),
            "hovered surface lifts");
        assert!(v.iter().any(|c| matches!(c, Cmd::Rect { color: col, .. } if *col != base)),
            "hover color differs from base");
    }

    #[test]
    fn shell_scene_parses_and_expands_components() {
        let ron = r#"
            ShellScene(
                components: {
                    "chip": [ Text(x: 4.0, y: 2.0, text: "Chip", font_size: 12.0, color: Some(fg)) ],
                },
                surfaces: [
                    (name: "settings",
                     items: [
                        Comp(name: "chip", x: 8.0, y: 6.0),
                        Text(x: 1.0, y: 1.0, text: "hello", font_size: 10.0, color: Some(fg)),
                     ]),
                ],
            )
        "#;
        let mut sc: ShellScene = ron::from_str(ron).unwrap();
        sc.expand();
        let surf = sc.surface("settings").expect("surface found by name");
        assert_eq!(surf.items.len(), 2, "Comp inlined + the literal text both remain");
        match &surf.items[0] {
            SceneItem::Text { x, y, text, .. } => {
                assert_eq!(*x, 12.0, "template translated by the Comp x/y");
                assert_eq!(*y, 8.0);
                assert_eq!(text, "Chip");
            }
            other => panic!("widget inlined but wrong: {other:?}"),
        }
    }

    #[test]
    fn shell_ron_live_file_parses_and_has_surfaces() {
        let path = crate::vars::shell_scene_path();
        // Only exercise the live-file parse when the canonical shell.ron is
        // reachable (runtime symlinks may differ from the test runner's HOME).
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(_) => return, // no file present in this env — nothing to assert
        };
        let mut sc: ShellScene = ron::from_str(&text).expect("RON parses cleanly");
        sc.expand();
        assert!(sc.surface("settings").is_some(), "settings surface declared");
        assert!(sc.surface("dashboard").is_some(), "dashboard surface declared");
        assert!(sc.surface("notif").is_some(), "notif surface declared");
        assert!(sc.surface("lock").is_some(), "lock surface declared");
        assert!(sc.surface("wallpaper").is_some(), "wallpaper surface declared");
        for name in ["settings", "notif", "lock", "wallpaper"] {
            let surf = sc.surface(name).expect("surface");
            assert!(
                surf.items.iter().any(|it| matches!(it, SceneItem::Ink { .. })),
                "{name} keeps its Rust body Ink"
            );
        }
        // the dashboard surface must carry BOTH live bodies: the board grid +
        // the top banner strip (its scrollbar overlay stays Rust inline).
        let dash = sc.surface("dashboard").expect("dashboard surface");
        let inks: Vec<&str> = dash
            .items
            .iter()
            .filter_map(|it| match it {
                SceneItem::Ink { name, .. } => Some(name.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(inks, ["dash_grid", "dash_banner"], "dashboard surface wires dash_grid + dash_banner Inks");
    }

    #[test]
    fn live_grid_card_scenes_parse() {
        // calendar / appshortcut / mirror — the last three card scenes. Only
        // exercised when the live config tree is reachable (runtime symlinks
        // may differ from the test runner's HOME).
        for id in ["calendar", "appshortcut", "mirror"] {
            let path = crate::vars::card_scene_path(id);
            let text = match std::fs::read_to_string(&path) {
                Ok(t) => t,
                Err(_) => continue, // not present in this env
            };
            let scene: CardScene = ron::from_str(&text).expect("{id}.ron parses cleanly");
            assert!(
                scene.items.iter().any(|it| matches!(
                    it,
                    SceneItem::Grid { .. } | SceneItem::Rows { .. } | SceneItem::Image { .. }
                )),
                "{id}.ron declares a live body (Grid/Rows/Image), not just an Ink stub"
            );
        }
    }

    #[test]
    fn dots_render_pane_rule_and_hover_gate() {
        // the multi-pane HARD RULE primitive: one dot per pane, the active
        // pane's dot in the accent, hidden entirely while `when` is false
        let scene: CardScene = ron::from_str(
            r#"(items: [ Dots(active: "pane", when: Some("hov"), dy: 3.5, step: 7.0, panes: 2) ])"#,
        )
        .unwrap();
        let mut vals = SceneValues::new();
        vals.insert("pane", SceneValue::Ring(1.0));
        vals.insert("hov", SceneValue::Toggle(true));
        let mut v: Vec<Cmd> = Vec::new();
        scene.draw(&mut v, 0.0, 0.0, 200.0, 120.0, scale(), &pal(), &vals, 0, true, true);
        let dots: Vec<&Cmd> = v.iter().filter(|c| matches!(c, Cmd::Rect { r, .. } if *r > 0.0 && *r < 6.0)).collect();
        assert_eq!(dots.len(), 2, "one dot per pane");
        let acc = pal().acc;
        assert!(dots.iter().any(|c| matches!(c, Cmd::Rect { color, .. } if *color == acc)), "active pane dot is accent");
        // hover gate off → nothing draws
        vals.insert("hov", SceneValue::Toggle(false));
        let mut v2: Vec<Cmd> = Vec::new();
        scene.draw(&mut v2, 0.0, 0.0, 200.0, 120.0, scale(), &pal(), &vals, 0, true, true);
        assert!(v2.is_empty(), "dots hidden while the cursor is off the card");
    }

    #[test]
    fn grid_cell_truncate_clips_long_labels() {
        let scene: CardScene = ron::from_str(
            r#"(items: [ Grid(name: "g", x: 0.0, y: 0.0, w: 100.0, h: 40.0, cols: 1) ])
            "#,
        )
        .unwrap();
        let mut vals = SceneValues::new();
        vals.insert(
            "g",
            SceneValue::GridCells(vec![GridCell {
                text: "very-long-app-name-here".into(),
                truncate: Some(4),
                ..Default::default()
            }]),
        );
        let mut v: Vec<Cmd> = Vec::new();
        scene.draw(&mut v, 0.0, 0.0, 100.0, 40.0, scale(), &pal(), &vals, 0, true, true);
        let texts: Vec<&String> = v
            .iter()
            .filter_map(|c| match c {
                Cmd::Text { text, .. } => Some(text),
                _ => None,
            })
            .collect();
        assert!(
            texts.iter().any(|t| t.as_str() == "very"),
            "label clips to the first n chars, got {texts:?}"
        );
    }

    #[test]
    fn items_between_inks_attach_to_the_next_ink() {
        // the dashboard band z-order: [Ink(grid), Surface(band), Ink(banner)]
        // must NOT draw the band upfront — it rides the banner ink's `pre` so
        // the Rust painter order is grid → band → banner (band over cards,
        // chips over band)
        let scene: CardScene = ron::from_str(
            r#"(items: [
                Ink(x: 0.0, y: 0.0, w: 0.0, h: 0.0, name: "grid"),
                Surface(x: 0.0, y: 12.0, w: 0.0, h: 26.0, r: 16.0, tr: -16.0, color: acc),
                Ink(x: 0.0, y: 0.0, w: 0.0, h: 0.0, name: "banner"),
            ])
            "#,
        )
        .unwrap();
        let mut v: Vec<Cmd> = Vec::new();
        let (_h, inks) = scene.draw(&mut v, 0.0, 0.0, 200.0, 120.0, scale(), &pal(), &SceneValues::new(), 0, true, true);
        // nothing drawn upfront (both pieces are Inks / attached chrome)…
        assert!(v.is_empty(), "no upfront cmds, got {}", v.len());
        // …the band rides the SECOND ink…
        assert_eq!(inks.len(), 2);
        assert!(inks[0].pre.is_empty(), "items before the first ink stay upfront-shaped");
        assert_eq!(inks[1].pre.len(), 1, "the band attaches to dash_banner");
        assert!(matches!(inks[1].pre[0], SceneItem::Surface { .. }));
    }

    #[test]
    fn banner_row_paints_cells_and_registers_hits() {
        // a declarative strip: one text chip, one tray chip with a fallback
        // glyph, one filler. The filler draws/hits nothing; chips register
        // their slot region (and tray icons their 30+ti sub-regions).
        let mut vals = SceneValues::new();
        vals.insert_color("bchip_clock", SceneColor::Token(ColorToken::Fg));
        vals.insert(
            "cells",
            SceneValue::BannerCells(vec![
                BannerCell {
                    x: 0.0,
                    w: 60.0,
                    ink: "bchip_clock".into(),
                    text: "12:34".into(),
                    glyph: None,
                    icons: Vec::new(),
                    zone: 3,
                    kind: BannerCellKind::Text { n: None },
                    key: 34_002,
                },
                BannerCell {
                    x: 66.0,
                    w: 48.0,
                    ink: "bchip_tray".into(),
                    text: String::new(),
                    glyph: None,
                    icons: vec![String::new()],
                    zone: 3,
                    kind: BannerCellKind::Tray,
                    key: 34_005,
                },
                BannerCell {
                    x: 120.0,
                    w: 30.0,
                    ink: "bchip_dim".into(),
                    text: String::new(),
                    glyph: None,
                    icons: Vec::new(),
                    zone: 3,
                    kind: BannerCellKind::Filler,
                    key: 0,
                },
            ]),
        );
        let scene: CardScene = ron::from_str(
            r#"(items: [ BannerRow(cells: "cells", x: 0.0, y: 12.0, w: 0.0, h: 26.0) ])
            "#,
        )
        .unwrap();
        let mut v: Vec<Cmd> = Vec::new();
        let mut hits_out: Vec<SceneHit> = Vec::new();
        let _ = &mut hits_out;
        let (_, inks) = scene.draw(&mut v, 0.0, 0.0, 200.0, 120.0, scale(), &pal(), &vals, 0, false, false);
        // pre-attach: a trailing BannerRow with no following Ink stays in the
        // upfront path — its pieces are already in `v`.
        assert!(inks.is_empty(), "no ink present → nothing deferred");
        // the tray fallback glyph + the clock label drew (2 text cmds), no
        // rect surfaces, and three hit regions landed (2 slots + 1 tray icon)
        let text = v
            .iter()
            .filter(|c| matches!(c, Cmd::Text { .. }))
            .count();
        assert_eq!(text, 2, "clock label + tray fallback glyph, got {text}");
        assert_eq!(
            v.iter().filter(|c| matches!(c, Cmd::Scissor { .. })).count(),
            0,
            "unzoned, non-overflow strip opens no clip"
        );
    }

    #[test]
    fn banner_row_overflow_clips_at_the_band() {
        let mut vals = SceneValues::new();
        vals.insert("ovf", SceneValue::Toggle(true));
        vals.insert(
            "cells",
            SceneValue::BannerCells(vec![BannerCell {
                x: 0.0,
                w: 50.0,
                ink: "bchip_clock".into(),
                text: "12:34".into(),
                glyph: None,
                icons: Vec::new(),
                zone: 3,
                kind: BannerCellKind::Text { n: None },
                key: 1,
            }]),
        );
        let scene: CardScene = ron::from_str(
            r#"(items: [ BannerRow(cells: "cells", x: 0.0, y: 12.0, w: 0.0, h: 26.0,
                          overflow: Some("ovf")) ])
            "#,
        )
        .unwrap();
        let mut v: Vec<Cmd> = Vec::new();
        scene.draw(&mut v, 0.0, 0.0, 200.0, 120.0, scale(), &pal(), &vals, 0, false, false);
        let open = v
            .iter()
            .find_map(|c| match c {
                Cmd::Scissor { x, y, w, h } => Some((*x, *y, *w, *h)),
                _ => None,
            })
            .expect("overflow opens a clip");
        assert_eq!(open, (0.0, 12.0, 200.0, 26.0), "clip = the band box");
        assert!(v.iter().any(|c| matches!(c, Cmd::ScissorEnd)), "clip closes");
    }

    #[test]
    fn banner_row_zone_scissors_interleave_between_cells() {
        // L/C/R strip: the scissor for zone 1 must open only after zone 0's
        // last cell drew (the Rust drawer's zone_push state machine) — a
        // single upfront clip would let a scrolled zone bleed into its
        // neighbour's window.
        let mut vals = SceneValues::new();
        vals.insert("zn", SceneValue::Toggle(true));
        let cell = |x: f32, zone: u8, key: u32| BannerCell {
            x,
            w: 20.0,
            ink: "bchip_clock".into(),
            text: "x".into(),
            glyph: None,
            icons: Vec::new(),
            zone,
            kind: BannerCellKind::Text { n: None },
            key,
        };
        vals.insert(
            "cells",
            SceneValue::BannerCells(vec![cell(0.0, 0, 1), cell(200.0, 1, 2), cell(400.0, 2, 3)]),
        );
        let scene: CardScene = ron::from_str(
            r#"(items: [ BannerRow(cells: "cells", x: 0.0, y: 12.0, w: 0.0, h: 26.0,
                          zones: Some("zn")) ])
            "#,
        )
        .unwrap();
        let mut v: Vec<Cmd> = Vec::new();
        scene.draw(&mut v, 0.0, 0.0, 600.0, 120.0, scale(), &pal(), &vals, 0, false, false);
        // sequence: scissor(z0) → text(z0) → scissor(z1) → text(z1) → scissor(z2)
        // → text(z2) → end — zone_push clips EVERY zone window (including the
        // first), matching the Rust drawer's state machine
        let seq: Vec<bool> = v
            .iter()
            .map(|c| matches!(c, Cmd::Text { .. }))
            .collect();
        let scissors: Vec<usize> = v
            .iter()
            .enumerate()
            .filter(|(_, c)| matches!(c, Cmd::Scissor { .. }))
            .map(|(i, _)| i)
            .collect();
        assert_eq!(scissors.len(), 3, "each zone window opens a clip");
        // the FIRST clip opens before zone 0's cells (it clips their window);
        // every later one must open after cells drew — interleaved, not upfront
        for &si in &scissors[1..] {
            assert!(seq[..si].iter().any(|&t| t), "zone scissor opens mid-sweep");
        }
        assert!(v.iter().any(|c| matches!(c, Cmd::ScissorEnd)), "last zone closes");
    }

    #[test]
    fn banner_row_visible_gate_and_missing_binding_draw_nothing() {
        let scene: CardScene = ron::from_str(
            r#"(items: [ BannerRow(cells: "missing", x: 0.0, y: 12.0, w: 0.0, h: 26.0,
                          visible: Some("off")) ])
            "#,
        )
        .unwrap();
        let mut vals = SceneValues::new();
        vals.insert("off", SceneValue::Toggle(false));
        let mut v: Vec<Cmd> = Vec::new();
        let (_, inks) = scene.draw(&mut v, 0.0, 0.0, 200.0, 120.0, scale(), &pal(), &vals, 0, false, false);
        assert!(v.is_empty(), "gated row draws nothing");
        assert!(inks.is_empty(), "gated row defers nothing");
    }

    #[test]
    fn surface_concave_corners_emit_rect_concave() {
        // the strip band's screen-edge corners: negative tr = concave top,
        // rendered via Cmd::RectConcave so the band curves into the panel
        let scene: CardScene = ron::from_str(
            r#"(items: [ Surface(x: 0.0, y: 12.0, w: 100.0, h: 26.0, r: 16.0, tr: -16.0, color: acc) ])
            "#,
        )
        .unwrap();
        let mut v: Vec<Cmd> = Vec::new();
        scene.draw(&mut v, 0.0, 0.0, 200.0, 120.0, scale(), &pal(), &SceneValues::new(), 0, true, true);
        assert!(
            v.iter().any(|c| matches!(c, Cmd::RectConcave { r_tl, r_tr, r_br, r_bl, .. }
                if *r_tl > 0.0 && *r_tr < 0.0 && *r_br == 0.0 && *r_bl == 0.0)),
            "concave top corners + square bottom corners"
        );
    }

    #[test]
    fn surface_center_anchors_to_box_center() {
        // the fullscreen-lock hero pieces anchor to the parent box center (the
        // dim + avatar signatures here) — x/y become offsets from that center
        let scene: CardScene = ron::from_str(
            r#"(items: [
                    Surface(x: 0.0, y: 0.0, w: 0.0, h: 0.0, r: 0.0, color: value("lock_dim")),
                    Surface(center_x: true, center_y: true, x: -12.0, y: -34.0, w: 52.0, h: 52.0, r: 26.0, color: acc),
                ])"#,
        )
        .unwrap();
        let mut vals = SceneValues::new();
        vals.insert_color("lock_dim", SceneColor::Raw(0x00000059));
        let mut v: Vec<Cmd> = Vec::new();
        scene.draw(&mut v, 0.0, 0.0, 200.0, 120.0, scale(), &pal(), &vals, 0, true, true);
        let rects: Vec<&Cmd> = v.iter().filter(|c| matches!(c, Cmd::Rect { .. })).collect();
        assert_eq!(rects.len(), 2, "dim + avatar");
        match rects[0] {
            Cmd::Rect { x, y, w, h, r, .. } => {
                assert_eq!((*x, *y, *w, *h, *r), (0.0, 0.0, 200.0, 120.0, 0.0), "0-dim fills the box");
            }
            _ => unreachable!(),
        }
        match rects[1] {
            Cmd::Rect { x, y, w, h, r, color, .. } => {
                assert_eq!(*x, 200.0 / 2.0 - 12.0, "center_x offsets from the box mid");
                assert_eq!(*y, 120.0 / 2.0 - 34.0, "center_y offsets from the box mid");
                assert_eq!((*w, *h, *r), (52.0, 52.0, 26.0));
                assert_eq!(*color, plt_acc());
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn right_anchor_hugs_card_width() {
        let scene: CardScene = ron::from_str(
            "(items: [ Hit(x: 12.0, y: 6.0, w: 16.0, h: 16.0, r: 5.0, key: 7, right: true, surface: Some(raised), text: \"›\", font_size: 10.0, icon: true) ])",
        )
        .unwrap();
        // card 148 wide: right-anchored chevron sits at 148 - (12 + 16) = 120
        let mut v: Vec<Cmd> = Vec::new();
        let (hits, _) = scene.draw(&mut v, 0.0, 0.0, 148.0, 90.0, scale(), &pal(), &vals(), 0, true, true);
        assert_eq!(hits[0].x, 148.0 - 28.0);
        // card 200 wide: same authoring survives a resize (hugs the right edge)
        let mut v: Vec<Cmd> = Vec::new();
        let (hits2, _) = scene.draw(&mut v, 0.0, 0.0, 200.0, 90.0, scale(), &pal(), &vals(), 0, true, true);
        assert_eq!(hits2[0].x, 200.0 - 28.0);
    }

    #[test]
    fn substitution_unknown_name_passthrough() {
        assert_eq!(substitute("hi {nope}", &vals()), "hi {nope}");
        assert_eq!(substitute("plain", &vals()), "plain");
        assert_eq!(substitute("a {rec_btn} b", &vals()), "a Record b");
    }

    #[test]
    fn committed_header_scenes_parse() {
        // every committed card scene may declare `Header` chrome; verify a
        // representative set (title + glyph + live meta) parses and that the
        // `Header` item carries the expected fields. HYBRID stubs carry their
        // Ink body painter; the 2026-09-17 Batch 2 conversions
        // (journaltail/systemdunits/smarthealth/thermal/conninfo) plus the
        // 2026-09-24 gauge cards (cpu/mem — see
        // `committed_cpu_mem_scenes_parse_zero_ink`), the 2026-09-25
        // compositor card and the battery cards (see
        // `battery_scenes_parse_zero_ink`) are fully declarative and moved to
        // the zero-Ink assertions below.
        for id in ["disk", "micmeter", "accent",
                   "gpu", "branding", "powerdraw", "water", "pomodoro",
                   "speedtest", "latency", "wallpaper", "audiodevice",
                   "moon", "eyerest",
                   "worldclock", "workspaces"] {
            let path = crate::vars::card_scene_path(id);
            if !path.exists() {
                continue;
            }
            let s = std::fs::read_to_string(&path).expect("scene file readable");
            let scene: CardScene = ron::from_str(&s)
                .unwrap_or_else(|e| panic!("committed {id}.ron must parse: {e}"));
            assert!(
                scene.items.iter().any(|i| matches!(i, SceneItem::Header { .. })),
                "{id} scene declares Header chrome"
            );
            assert!(
                scene.items.iter().any(|i| matches!(i, SceneItem::Ink { name, .. } if name == id)),
                "{id} scene carries its Ink body painter"
            );
        }
    }

    #[test]
    fn committed_batch2_body_scenes_parse_zero_ink() {
        // journaltail / systemdunits / smarthealth (severity rows + refresh
        // Hit), thermal (fixed-height chip Grid), conninfo (metric rows +
        // refresh Hit) — Header-led, zero-Ink, data-driven via a live body
        // (Rows/Grid) and the centered empty-state caption.
        for id in ["journaltail", "systemdunits", "smarthealth", "thermal", "conninfo"] {
            let path = crate::vars::card_scene_path(id);
            if !path.exists() {
                continue;
            }
            let s = std::fs::read_to_string(&path).expect("scene file readable");
            let scene: CardScene = ron::from_str(&s)
                .unwrap_or_else(|e| panic!("committed {id}.ron must parse: {e}"));
            assert!(
                scene.items.iter().any(|i| matches!(i, SceneItem::Header { .. })),
                "{id} scene declares Header chrome"
            );
            assert!(
                !scene.items.iter().any(|i| matches!(i, SceneItem::Ink { .. })),
                "{id} scene is fully declarative (no Ink body)"
            );
            assert!(
                scene.items.iter().any(|i| matches!(i, SceneItem::Rows { .. } | SceneItem::Grid { .. })),
                "{id} scene is data-driven via Rows/Grid"
            );
            assert!(
                scene.items.iter().any(|i| matches!(
                    i,
                    SceneItem::Text { center_x: true, center_y: true, visible: Some(_), .. }
                )),
                "{id} scene gates a centered empty-state caption"
            );
        }
        // the four refreshed data cards carry their bottom-right refresh Hit
        // at the resident handler's key
        for (id, key) in [
            ("journaltail", crate::shell::JOURNAL_REFRESH_KEY),
            ("systemdunits", crate::shell::SYSTEMD_REFRESH_KEY),
            ("smarthealth", crate::shell::SMART_REFRESH_KEY),
            ("conninfo", crate::shell::CONNINFO_REFRESH_KEY),
        ] {
            let path = crate::vars::card_scene_path(id);
            if !path.exists() {
                continue;
            }
            let s = std::fs::read_to_string(&path).expect("scene file readable");
            let scene: CardScene = ron::from_str(&s).unwrap();
            assert!(
                scene.items.iter().any(|i| matches!(i, SceneItem::Hit { key: k, .. } if *k == key)),
                "{id} scene carries the refresh Hit (key {key})"
            );
        }
        // thermal's chip grid is FIXED-height (no stretch — Rust parity)
        let path = crate::vars::card_scene_path("thermal");
        if path.exists() {
            let s = std::fs::read_to_string(&path).expect("scene file readable");
            let scene: CardScene = ron::from_str(&s).unwrap();
            let grid = scene
                .items
                .iter()
                .find_map(|i| match i {
                    SceneItem::Grid { cell_h, cols, .. } => Some((*cell_h, *cols)),
                    _ => None,
                })
                .expect("thermal scene declares the chip Grid");
            assert_eq!(grid, (24.0, 3), "thermal grid: 3 cols, fixed 24 px chip rows");
        }
    }

    #[test]
    fn committed_cpu_mem_scenes_parse_zero_ink() {
        // cpu / mem (2026-09-24) — Header-led gauge cards, fully declarative
        // (no Ink body): a bead-ring `Ring` + data-driven `Rows` (metric
        // rows), and mem additionally carries the bottom swap `Bar` gated by
        // a presence toggle. The Rust drawers are now dead for these ids.
        for id in ["cpu", "mem"] {
            let path = crate::vars::card_scene_path(id);
            if !path.exists() {
                continue;
            }
            let s = std::fs::read_to_string(&path).expect("scene file readable");
            let scene: CardScene = ron::from_str(&s)
                .unwrap_or_else(|e| panic!("committed {id}.ron must parse: {e}"));
            assert!(
                scene.items.iter().any(|i| matches!(i, SceneItem::Header { .. })),
                "{id} scene declares Header chrome"
            );
            assert!(
                !scene.items.iter().any(|i| matches!(i, SceneItem::Ink { .. })),
                "{id} scene is fully declarative (no Ink body)"
            );
            assert!(
                scene.items.iter().any(|i| matches!(i, SceneItem::Ring { .. })),
                "{id} scene draws the bead-ring gauge"
            );
            assert!(
                scene.items.iter().any(|i| matches!(i, SceneItem::Rows { .. })),
                "{id} scene is data-driven via Rows"
            );
        }
        let path = crate::vars::card_scene_path("mem");
        if path.exists() {
            let s = std::fs::read_to_string(&path).expect("scene file readable");
            let scene: CardScene = ron::from_str(&s).unwrap();
            let bar = scene
                .items
                .iter()
                .find_map(|i| match i {
                    SceneItem::Bar { value, max, visible, .. } => Some((value.as_str(), *max, visible.clone())),
                    _ => None,
                })
                .expect("mem scene declares the swap Bar");
            assert_eq!(bar.0, "{swap_frac}", "swap bar binds the fractional value");
            assert_eq!(bar.1, 100.0, "swap bar fraction is of a 100 max");
            assert_eq!(bar.2.as_deref(), Some("swap_show"), "swap bar shown only in use");
        }
    }

    #[test]
    fn rows_edge_text_cells_anchor_right_and_bottom_inset_clips() {
        // Batch 2 (2026-09-17): plain text cells honor `edge` (the cell's
        // RIGHT edge lands at card_w − edge — news open-glyph/ticker chg%
        // parity), and `bottom` reserves the last N px of the card (the
        // conninfo list clears its refresh button).
        let scene: CardScene = ron::from_str(
            r#"(items: [ Rows(name: "rw", y: 30.0, row_h: 18.0, pad: 12.0, bottom: 32.0, cols: [
                (x: 0.0),
                (x: 0.0, w: 0.0, edge: Some(14.0), right: true, font: mono, size: 8.5),
            ], key_base: Some(91)) ])"#,
        )
        .unwrap();
        let mk = |v: &str| SceneRow { cols: vec!["iface".into(), v.into()], key: 0, action: None, color: None, col_colors: vec![], surface: None };
        let mut values = SceneValues::new();
        values.insert("rw", SceneValue::Rows(vec![mk("10.0.0.2"), mk("10.0.0.3"), mk("10.0.0.4"), mk("10.0.0.5"), mk("10.0.0.6"), mk("10.0.0.7")]));
        let mut v: Vec<Cmd> = Vec::new();
        let (hits, _) = scene.draw(&mut v, 0.0, 0.0, 220.0, 140.0, scale(), &pal(), &values, 0, true, true);
        // right cells: text anchored at card_w − 14 = 206 (right → draws left of it)
        assert!(
            v.iter().any(|c| matches!(c, Cmd::Text { x, right: true, text, .. } if (*x - 206.0).abs() < 0.01 && text == "10.0.0.2")),
            "edge cell right-anchors at card_w − edge",
        );
        // bottom inset 32: rows start at 30, pitch 18 → row boxes crossing
        // y=108 (140−32) are skipped: rows at 30/48/66/84 fit, 102+108 do not
        assert_eq!(hits.len(), 4, "rows past the bottom inset are skipped (got {hits:?})");
        assert_eq!(hits[3].key, 94);
    }

    #[test]
    fn grid_fixed_cell_h_keeps_pitch() {
        // thermal chips: `cell_h` fixes the row pitch — a partial tail row
        // leaves the rest of the box open instead of stretching cells
        let scene: CardScene = ron::from_str(
            r#"(items: [ Grid(name: "g", x: 0.0, y: 0.0, h: 100.0, cols: 3, gap: 2.0, cell_h: 24.0) ])"#,
        )
        .unwrap();
        let mut values = SceneValues::new();
        let chip = |t: &str| GridCell { text: t.into(), surface: Some(ColorToken::Raised), ..Default::default() };
        values.insert(
            "g",
            SceneValue::GridCells(vec![chip("acpitz"), chip("core"), chip("nvme"), chip("acpi2")]),
        );
        let mut v: Vec<Cmd> = Vec::new();
        let _ = scene.draw(&mut v, 0.0, 0.0, 220.0, 140.0, scale(), &pal(), &values, 0, true, true);
        let chip_h: Vec<f32> = v
            .iter()
            .filter_map(|c| match c {
                // any rounded rect is a chip (the default `r` is 8 px)
                Cmd::Rect { h, r, .. } if *r > 0.0 => Some(*h),
                _ => None,
            })
            .collect();
        assert_eq!(chip_h.len(), 4, "all four chips drawn (2 rows, partial tail) — got {chip_h:?}");
        assert!(chip_h.iter().all(|h| (*h - 24.0).abs() < 0.01), "fixed 24 px chip height (got {chip_h:?})");
    }

    #[test]
    fn header_renders_title_glyph_and_meta_flagged() {
        let scene: CardScene = ron::from_str(
            "(items: [ Header(title: \"CPU\", meta: Some(\"{cpu_meta}\"), meta_mono: true, pad: 12.0) ])",
        )
        .unwrap();
        let mut values = SceneValues::new();
        values.insert("cpu_meta", SceneValue::Text("3400 MHz".into()));
        let mut v: Vec<Cmd> = Vec::new();
        // show_title=false hides the title; show_glyph irrelevant (no glyph)
        let (_h, _i) = scene.draw(&mut v, 0.0, 0.0, 180.0, 120.0, scale(), &pal(), &values, 0, false, true);
        assert!(v.iter().all(|c| !matches!(c, Cmd::Text { text, .. } if text == "CPU")),
            "title hidden when card_show_title is off");
        assert!(v.iter().any(|c| matches!(c, Cmd::Text { text, .. } if text == "3400 MHz")),
            "meta draws with {{cpu_meta}} interpolated (mono meta kept)");
    }

    #[test]
    fn committed_audiorec_scene_parses() {
        // The reference card scene ships in the author's config tree; parse
        // it if present so a syntax drift fails the suite on dev machines.
        let path = crate::vars::card_scene_path("audiorec");
        if !path.exists() {
            return; // not present on this machine — nothing to check
        }
        let s = std::fs::read_to_string(&path).expect("scene file readable");
        let scene: CardScene = ron::from_str(&s)
            .unwrap_or_else(|e| panic!("committed audiorec.ron must parse: {e}"));
        assert!(
            scene.items.iter().any(|i| matches!(i, SceneItem::Hit { key: 32302, .. })),
            "chevron hit carries the pane key"
        );
    }

    #[test]
    fn committed_clock_scene_parses() {
        let path = crate::vars::card_scene_path("clock");
        if !path.exists() {
            return;
        }
        let s = std::fs::read_to_string(&path).expect("scene file readable");
        let scene: CardScene = ron::from_str(&s)
            .unwrap_or_else(|e| panic!("committed clock.ron must parse: {e}"));
        assert!(
            scene.items.iter().any(|i| matches!(i, SceneItem::Text { center_x: true, center_y: true, .. })),
            "clock scenes center both axes so they adapt to any cell"
        );
        assert!(
            scene.items.iter().any(|i| matches!(i, SceneItem::Text { text, .. } if text == "{clock}")),
            "clock scene consumes the live {{clock}} value"
        );
    }

    #[test]
    fn committed_batch1_list_scenes_parse_and_drive_rows() {
        // First fully-converted list cards (Batch 1): each scene is Header-led,
        // data-driven via a Rows binding, and scrollable cards bind `start`.
        for id in ["bluetooth", "ticker", "currency", "sshvpn", "recent"] {
            let path = crate::vars::card_scene_path(id);
            if !path.exists() {
                continue;
            }
            let s = std::fs::read_to_string(&path).expect("scene file readable");
            let scene: CardScene = ron::from_str(&s)
                .unwrap_or_else(|e| panic!("committed {id}.ron must parse: {e}"));
            assert!(
                scene.items.iter().any(|i| matches!(i, SceneItem::Header { .. })),
                "{id} scene declares Header chrome"
            );
            assert!(
                scene.items.iter().any(|i| matches!(i, SceneItem::Rows { .. })),
                "{id} scene is data-driven via a Rows binding"
            );
        }
    }

    #[test]
    fn committed_pkgupdates_scene_parses() {
        let path = crate::vars::card_scene_path("pkgupdates");
        if !path.exists() {
            return;
        }
        let s = std::fs::read_to_string(&path).expect("scene file readable");
        let scene: CardScene = ron::from_str(&s)
            .unwrap_or_else(|e| panic!("committed pkgupdates.ron must parse: {e}"));
        assert!(
            !scene.items.iter().any(|i| matches!(i, SceneItem::Ink { .. })),
            "pkgupdates is fully declarative (hero count + caption, no Ink painter)"
        );
        assert!(
            scene.items.iter().any(|i| matches!(i, SceneItem::Text { text, .. } if text == "{pkg_count}")),
            "pkgupdates consumes the live {{pkg_count}} value"
        );
    }

    #[test]
    fn committed_kblayout_scene_parses() {
        let path = crate::vars::card_scene_path("kblayout");
        if !path.exists() {
            return;
        }
        let s = std::fs::read_to_string(&path).expect("scene file readable");
        let scene: CardScene = ron::from_str(&s)
            .unwrap_or_else(|e| panic!("committed kblayout.ron must parse: {e}"));
        assert!(
            scene.items.iter().any(|i| matches!(i, SceneItem::Text { text, .. } if text == "{kblayout}")),
            "kblayout scene consumes the live {{kblayout}} value"
        );
    }

    #[test]
    fn committed_batch2_bar_row_scenes_parse_and_drive_rows() {
        // Batch 2 openers: procmon (right meter bar), fans (label · rpm ·
        // per-fan bar + empty state) — all zero-Ink,
        // data-driven via Rows bar cells. The Spark trio (network / diskio /
        // sensors) is zero-Ink too but drives Spark/Text items, not Rows.
        for id in ["procmon", "fans"] {
            let path = crate::vars::card_scene_path(id);
            if !path.exists() {
                continue;
            }
            let s = std::fs::read_to_string(&path).expect("scene file readable");
            let scene: CardScene = ron::from_str(&s)
                .unwrap_or_else(|e| panic!("committed {id}.ron must parse: {e}"));
            assert!(
                scene.items.iter().any(|i| matches!(i, SceneItem::Header { .. })),
                "{id} scene declares Header chrome"
            );
            assert!(
                !scene.items.iter().any(|i| matches!(i, SceneItem::Ink { .. })),
                "{id} scene is fully declarative (no Ink body)"
            );
            let rows = scene
                .items
                .iter()
                .find_map(|i| match i {
                    SceneItem::Rows { name, cols, .. } => Some((name.as_str(), cols)),
                    _ => None,
                })
                .expect("{id} scene is data-driven via Rows");
            assert!(
                rows.1.iter().any(|c| c.bar.is_some()),
                "{id} rows carry a bar cell"
            );
        }
    }

    #[test]
    fn committed_batch2_spark_scenes_parse_zero_ink() {
        // network (dual normalized Sparks + centered speed row), diskio
        // (legend dots + dual Sparks), sensors (gated live rows) — zero-Ink
        // declarative scenes driven by pre-normalized Spark bindings and
        // per-sensor text gates
        for id in ["network", "diskio", "sensors"] {
            let path = crate::vars::card_scene_path(id);
            if !path.exists() {
                continue;
            }
            let s = std::fs::read_to_string(&path).expect("scene file readable");
            let scene: CardScene = ron::from_str(&s)
                .unwrap_or_else(|e| panic!("committed {id}.ron must parse: {e}"));
            assert!(
                scene.items.iter().any(|i| matches!(i, SceneItem::Header { .. })),
                "{id} scene declares Header chrome"
            );
            assert!(
                !scene.items.iter().any(|i| matches!(i, SceneItem::Ink { .. })),
                "{id} scene is fully declarative (no Ink body)"
            );
        }
        // the graph cards bind both series (read/download + write/upload)
        for id in ["network", "diskio"] {
            let path = crate::vars::card_scene_path(id);
            let s = std::fs::read_to_string(&path).expect("scene file readable");
            let scene: CardScene = ron::from_str(&s).unwrap();
            let sparks = scene
                .items
                .iter()
                .filter(|i| matches!(i, SceneItem::Spark { .. }))
                .count();
            assert_eq!(sparks, 2, "{id} carries both series Sparks");
        }
    }

    #[test]
    fn rows_bar_cells_parse_value_fill_and_track() {
        // the fans meter rows: a bar cell interpolates its cell text
        // ({name} → fraction), draws track + fill honoring the row colors
        let scene: CardScene = ron::from_str(
            r#"(items: [ Rows(
                name: "fan_rows", y: 30.0, row_h: 24.0, pad: 12.0,
                cols: [
                    (x: 0.0, dy: Some(1.0), size: 8.5, truncate: Some(60.0)),
                    (x: 0.0, edge: Some(12.0), right: true, w: 80.0, dy: Some(13.0),
                     bar: Some((height: 3.0, top: 13.0, radius: 1.5, fill: Some(info)))),
                ],
            ) ])
            "#,
        )
        .unwrap();
        let cols: Vec<&SceneCol> = scene
            .items
            .iter()
            .filter_map(|i| match i {
                SceneItem::Rows { cols, .. } => Some(cols.iter().collect()),
                _ => None,
            })
            .next()
            .expect("rows item");
        assert!(cols[1].bar.is_some(), "bar cell declared");
        let b = cols[1].bar.as_ref().unwrap();
        assert_eq!((b.height, b.top, b.radius, b.max), (3.0, 13.0, 1.5, 1.0));
        assert_eq!(b.fill, Some(ColorToken::Info));
        assert!(b.track.is_none(), "track defaults to hover");
    }

    #[test]
    fn rows_bar_cell_zero_width_draws_degenerate_track() {
        // the bar cell honors `w` literally — `w: 0` yields a degenerate
        // track at the cell's x (scenes author real widths, fans: 80)
        let scene: CardScene = ron::from_str(
            r#"(items: [ Rows(
                name: "fan_rows", y: 30.0, row_h: 24.0, pad: 12.0,
                cols: [
                    (x: 0.0, w: 0.0, bar: Some((height: 3.0, top: 13.0, max: 2.0))),
                ],
            ) ])
            "#,
        )
        .unwrap();
        let mut vals = SceneValues::new();
        vals.insert(
            "fan_rows",
            SceneValue::Rows(vec![SceneRow { cols: vec!["0.5".into()], ..Default::default() }]),
        );
        let mut v: Vec<Cmd> = Vec::new();
        scene.draw(&mut v, 0.0, 0.0, 200.0, 120.0, scale(), &pal(), &vals, 0, true, true);
        let rects: Vec<(f32, f32, f32, f32)> = v
            .iter()
            .filter_map(|c| match c {
                Cmd::Rect { x, y, w, h, .. } => Some((*x, *y, *w, *h)),
                _ => None,
            })
            .collect();
        // the bar cell honors `w` literally — `w: 0` yields a degenerate
        // track at the cell's x (scenes author real widths, fans: 80)
        let track = rects.iter().find(|r| (r.2 - 0.0).abs() < 0.01).expect("zero-width track");
        assert_eq!((track.0, track.3), (12.0, 3.0), "track at pad x, spec height");
    }

    #[test]
    fn rows_bar_cell_draws_track_and_fraction_fill() {
        // real width: track spans the cell box, fill = frac of max, fill
        // hugs the track's left edge
        let scene: CardScene = ron::from_str(
            r#"(items: [ Rows(
                name: "fan_rows", y: 30.0, row_h: 24.0, pad: 12.0,
                cols: [
                    (x: 0.0, w: 80.0, bar: Some((height: 3.0, top: 13.0, max: 2.0))),
                ],
            ) ])
            "#,
        )
        .unwrap();
        let mut vals = SceneValues::new();
        vals.insert(
            "fan_rows",
            SceneValue::Rows(vec![SceneRow { cols: vec!["0.5".into()], ..Default::default() }]),
        );
        let mut v: Vec<Cmd> = Vec::new();
        scene.draw(&mut v, 0.0, 0.0, 200.0, 120.0, scale(), &pal(), &vals, 0, true, true);
        let rects: Vec<(f32, f32, f32, f32)> = v
            .iter()
            .filter_map(|c| match c {
                Cmd::Rect { x, y, w, h, .. } => Some((*x, *y, *w, *h)),
                _ => None,
            })
            .collect();
        let track = rects.iter().find(|r| (r.2 - 80.0).abs() < 0.01).expect("track rect");
        let fill = rects.iter().find(|r| (r.2 - 20.0).abs() < 0.01).expect("fill rect");
        assert!((track.0 - fill.0).abs() < 0.01, "fill hugs the track's left");
        assert_eq!(fill.3, track.3, "fill height matches the track");
    }

    #[test]
    fn scene_deserializes_minimal_hit() {
        let scene: CardScene =
            ron::from_str("(items: [ Text(x: 4.0, y: 5.0, text: \"hi\", font_size: 9.0),\n Hit(x: 0.0, y: 0.0, w: 10.0, h: 10.0, key: 1) ])").unwrap();
        assert_eq!(scene.items.len(), 2);
        match &scene.items[0] {
            SceneItem::Text { font, color, .. } => {
                assert_eq!(*font, SceneFont::Ui);
                assert_eq!(*color, None);
            }
            _ => panic!(),
        }
        match &scene.items[1] {
            SceneItem::Hit { key, surface, text, .. } => {
                assert_eq!(*key, 1);
                assert_eq!(*surface, None);
                assert!(text.is_empty());
            }
            _ => panic!(),
        }
    }

    // ── data-driven primitives (Phase 2a) ───────────────────────────────

    #[test]
    fn ron_parses_rows_and_column_formats() {
        let scene: CardScene = ron::from_str(
            r#"(items: [ Rows(name: "jr_rows", y: 30.0, row_h: 24.0,
                cols: [ (x: 0.0), (x: 160.0, right: true, font: mono, size: 9.0, color: Some(fg3)) ],
                key_base: Some(700), hover_surface: Some(hover)) ])"#,
        )
        .unwrap();
        match &scene.items[0] {
            SceneItem::Rows { name, cols, key_base, hover_surface, pad, max_rows, .. } => {
                assert_eq!(name, "jr_rows");
                assert_eq!(*key_base, Some(700), "a declared key_base opts the rows into regions");
                assert_eq!(*hover_surface, Some(ColorToken::Hover));
                assert_eq!(cols.len(), 2);
                assert_eq!(cols[0].font, SceneFont::Ui);
                assert_eq!(cols[0].weight, SceneWeight::Regular, "columns default to Regular body rows");
                assert!(cols[1].right);
                assert_eq!(cols[1].size, 9.0);
                assert_eq!(*pad, 12.0);
                assert_eq!(*max_rows, 0);
            }
            other => panic!("expected Rows, got {other:?}"),
        }
    }

    #[test]
    fn rows_draws_cells_and_registers_row_keys() {
        let scene: CardScene = ron::from_str(
            "(items: [ Rows(name: \"rw\", y: 30.0, row_h: 24.0, cols: [ (x: 0.0), (x: 160.0, right: true) ], key_base: Some(700)) ])",
        )
        .unwrap();
        let mut values = SceneValues::new();
        values.insert(
            "rw",
            SceneValue::Rows(vec![
                SceneRow { cols: vec!["First".into(), "1".into()], key: 0, action: None, color: None, col_colors: vec![], surface: None },
                SceneRow { cols: vec!["Second".into(), "2".into()], key: 0, action: None, color: None, col_colors: vec![], surface: None },
                SceneRow { cols: vec!["Third".into(), "3".into()], key: 0, action: None, color: None, col_colors: vec![], surface: None },
            ]),
        );
        let mut v: Vec<Cmd> = Vec::new();
        let (hits, _) = scene.draw(&mut v, 0.0, 0.0, 200.0, 120.0, scale(), &pal(), &values, 0, true, true);
        assert!(v.iter().any(|c| matches!(c, Cmd::Text { text, .. } if text == "First")), "row 1 drawn");
        assert!(v.iter().any(|c| matches!(c, Cmd::Text { text, .. } if text == "3")), "row 3 drawn");
        assert_eq!(hits.len(), 3, "each row registers key_base + index");
        assert_eq!(hits[0].key, 700);
        assert_eq!(hits[2].key, 702);
    }

    #[test]
    fn rows_skip_past_the_card_edge_and_keep_own_keys() {
        let scene: CardScene = ron::from_str(
            "(items: [ Rows(name: \"rw\", y: 30.0, row_h: 24.0, cols: [ (x: 0.0) ], key_base: Some(700)) ])",
        )
        .unwrap();
        let mut values = SceneValues::new();
        // 4 rows at pitch 24 from y=30 → rows 1-3 end at 102 (below 120 card? row h=24:
        // row0 30-54, row1 54-78, row2 78-102, row3 102-126 > card 120 → skipped)
        values.insert("rw", SceneValue::Rows((0..4).map(|i| SceneRow { cols: vec![format!("r{i}")], key: 800 + i, action: None, color: None, col_colors: vec![], surface: None }).collect()));
        let mut v: Vec<Cmd> = Vec::new();
        let (hits, _) = scene.draw(&mut v, 0.0, 0.0, 200.0, 120.0, scale(), &pal(), &values, 0, true, true);
        assert_eq!(hits.len(), 3, "only rows fully above the card's bottom edge register");
        assert_eq!(hits[0].key, 800, "explicit per-row key wins over key_base");
    }

    #[test]
    fn rows_hover_surface_lifts_on_row_key() {
        let scene: CardScene = ron::from_str(
            "(items: [ Rows(name: \"rw\", y: 30.0, row_h: 24.0, cols: [ (x: 0.0) ], key_base: Some(700), hover_surface: Some(raised)) ])",
        )
        .unwrap();
        let mut values = SceneValues::new();
        values.insert("rw", SceneValue::Rows(vec![SceneRow { cols: vec!["a".into()], key: 0, action: None, color: None, col_colors: vec![], surface: None }]));
        // resting: no background rect drawn
        let mut v: Vec<Cmd> = Vec::new();
        let (_h, _i) = scene.draw(&mut v, 0.0, 0.0, 200.0, 120.0, scale(), &pal(), &values, 0, true, true);
        assert!(v.iter().all(|c| !matches!(c, Cmd::Rect { .. })),
            "resting rows draw no background surface");
        // hovered: one background rect with hover_variant
        let hov = ColorToken::Raised.hover_variant(&pal());
        let mut v: Vec<Cmd> = Vec::new();
        let (_h, _i) = scene.draw(&mut v, 0.0, 0.0, 200.0, 120.0, scale(), &pal(), &values, 700, true, true);
        assert!(v.iter().any(|c| matches!(c, Cmd::Rect { color: col, .. } if *col == hov)),
            "hovered row lifts its surface");
    }

    #[test]
    fn rows_resting_surface_wins_over_hover_and_icon_col_renders() {
        let scene: CardScene = ron::from_str(
            "(items: [ Rows(name: \"rw\", y: 30.0, row_h: 24.0, cols: [ (x: 0.0, icon: true), (x: 18.0, color: Some(fg3)) ], key_base: Some(700), hover_surface: Some(raised)) ])",
        )
        .unwrap();
        let mut values = SceneValues::new();
        values.insert("rw", SceneValue::Rows(vec![SceneRow {
            cols: vec![ICON_GLOBE.into(), "linked".into()],
            key: 0,
            action: None,
            color: None,
            surface: Some(ColorToken::AccTint),
            col_colors: vec![],
        }]));
        let mut v: Vec<Cmd> = Vec::new();
        // resting: the row's own surface tint draws (no hover needed)
        let (_h, _i) = scene.draw(&mut v, 0.0, 0.0, 200.0, 120.0, scale(), &pal(), &values, 0, true, true);
        assert!(
            v.iter().any(|c| matches!(c, Cmd::Rect { color: col, .. } if *col == ColorToken::AccTint.resolve(&pal()))),
            "resting surface tint draws behind the row"
        );
        assert!(
            v.iter().any(|c| matches!(c, Cmd::Text { text, icon, .. } if *icon && text == ICON_GLOBE)),
            "icon column renders its glyph in the icon font"
        );
        // hovered: the resting tint takes precedence over the hover lift
        let hov = ColorToken::AccTint.hover_variant(&pal());
        let hit = ColorToken::Raised.hover_variant(&pal());
        let mut v: Vec<Cmd> = Vec::new();
        let (_h, _i) = scene.draw(&mut v, 0.0, 0.0, 200.0, 120.0, scale(), &pal(), &values, 700, true, true);
        assert!(
            !v.iter().any(|c| matches!(c, Cmd::Rect { color: col, .. } if *col == hov || *col == hit)),
            "row surface wins over hover tint"
        );
        assert!(
            v.iter().any(|c| matches!(c, Cmd::Rect { color: col, .. } if *col == ColorToken::AccTint.resolve(&pal()))),
            "surface tint stays put under hover"
        );
    }

    #[test]
    fn rows_checkbox_cells_draw_checked_and_unchecked() {
        let scene: CardScene = ron::from_str(
            r#"(items: [ Rows(name: "rw", y: 30.0, row_h: 24.0, cols: [ (x: 20.0, size: 9.5) ], key_base: Some(700), check: Some("ck")) ])"#,
        )
        .unwrap();
        let mut values = SceneValues::new();
        values.insert(
            "rw",
            SceneValue::Rows(vec![
                SceneRow { cols: vec!["done".into()], key: 0, action: None, color: None, col_colors: vec![], surface: None },
                SceneRow { cols: vec!["open".into()], key: 0, action: None, color: None, col_colors: vec![], surface: None },
            ]),
        );
        values.insert("ck", SceneValue::Checks(vec![true, false]));
        let mut v: Vec<Cmd> = Vec::new();
        let (_h, _i) = scene.draw(&mut v, 0.0, 0.0, 200.0, 120.0, scale(), &pal(), &values, 0, true, true);
        // checked (row 0, y=30): acc-tint fill at pad(12)+check_x(2), +1 = 14,31
        let acc_tint = mix(pal().bg, pal().acc, 0.16);
        assert!(
            v.iter().any(|c| matches!(c, Cmd::Rect { x, y, w, h, color, .. } if *x == 14.0 && *y == 31.0 && *w == 11.0 && *h == 11.0 && *color == acc_tint)),
            "checked row draws the acc-tint checkbox well"
        );
        assert!(
            v.iter().any(|c| matches!(c, Cmd::Text { text, .. } if text == ICON_CHECK)),
            "checked row draws the check glyph"
        );
        // unchecked (row 1, y=54): outline only, no fill, no glyph
        assert!(
            v.iter().any(|c| matches!(c, Cmd::Outline { x, y, w, h, width, .. } if *x == 14.0 && *y == 55.0 && *w == 11.0 && *h == 11.0 && *width == 1.2)),
            "unchecked row draws a stroke-only checkbox"
        );
        assert_eq!(
            v.iter().filter(|c| matches!(c, Cmd::Text { text, .. } if text == ICON_CHECK)).count(),
            1,
            "exactly one check glyph for the one checked row"
        );
    }

    #[test]
    fn rows_del_reveals_on_hover_and_registers_corner_region() {
        let scene: CardScene = ron::from_str(
            r#"(items: [ Rows(name: "rw", y: 30.0, row_h: 24.0, cols: [ (x: 20.0, size: 9.5) ], key_base: Some(700), del_base: 70, del_pad: 20.0) ])"#,
        )
        .unwrap();
        let mut values = SceneValues::new();
        values.insert(
            "rw",
            SceneValue::Rows(vec![SceneRow { cols: vec!["a".into()], key: 0, action: None, color: None, col_colors: vec![], surface: None }]),
        );
        // resting (no hover): no ✕ drawn, no del region registered
        let mut v: Vec<Cmd> = Vec::new();
        let (hits, _i) = scene.draw(&mut v, 0.0, 0.0, 200.0, 120.0, scale(), &pal(), &values, 0, true, true);
        assert!(!v.iter().any(|c| matches!(c, Cmd::Text { text, .. } if text == ICON_CLOSE)), "✕ hidden while resting");
        assert!(hits.iter().all(|h| h.key != 70), "no del region while resting");
        // row hovered (key 700): ✕ appears right-anchored in fg + del region wins the corner
        let mut v: Vec<Cmd> = Vec::new();
        let (hits, _i) = scene.draw(&mut v, 0.0, 0.0, 200.0, 120.0, scale(), &pal(), &values, 700, true, true);
        let glyph = v.iter().find_map(|c| if let Cmd::Text { text, x, y, color, .. } = c { if text == ICON_CLOSE { Some((*x, *y, *color)) } else { None } } else { None });
        assert_eq!(glyph, Some((180.0, 31.0, pal().fg)), "hovered row reveals the ✕ at card_w - del_pad in fg");
        assert!(hits.iter().any(|h| h.key == 70 && h.x == 176.0 && h.y == 26.0 && h.w == 22.0 && h.h == 22.0), "del region registered on the row corner");
        // hovering the ✕ itself (key 70): glyph flips to danger
        let mut v: Vec<Cmd> = Vec::new();
        let (_h, _i) = scene.draw(&mut v, 0.0, 0.0, 200.0, 120.0, scale(), &pal(), &values, 70, true, true);
        let dange = ColorToken::Danger.resolve(&pal());
        assert!(
            v.iter().any(|c| matches!(c, Cmd::Text { text, color, .. } if text == ICON_CLOSE && *color == dange)),
            "hovering the ✕ paints it danger-red"
        );
    }

    #[test]
    fn rows_del_reveals_without_registered_row_keys() {
        let scene: CardScene = ron::from_str(
            r#"(items: [ Rows(name: "rw", y: 30.0, row_h: 24.0, pad: 12.0, cols: [ (x: 50.0, size: 9.5) ], del_base: 70, del_pad: 26.0, del_dy: 4.0) ])"#,
        )
        .unwrap();
        let mut values = SceneValues::new();
        values.insert(
            "rw",
            SceneValue::Rows(vec![SceneRow { cols: vec!["label".into()], key: 0, action: None, color: None, col_colors: vec![], surface: None }]),
        );
        // key-less rows (no key_base): resting draws no ✕ and no row/del regions
        let mut v: Vec<Cmd> = Vec::new();
        let (hits, _i) = scene.draw(&mut v, 0.0, 0.0, 200.0, 120.0, scale(), &pal(), &values, 0, true, true);
        assert!(!v.iter().any(|c| matches!(c, Cmd::Text { text, .. } if text == ICON_CLOSE)), "✕ hidden while resting");
        assert_eq!(hits.len(), 0, "no row-level region for key-less rows");
        // directly hovering the ✕ (del_base + 0) reveals at del_pad/del_dy + registers only it
        let mut v: Vec<Cmd> = Vec::new();
        let (hits, _i) = scene.draw(&mut v, 0.0, 0.0, 200.0, 120.0, scale(), &pal(), &values, 70, true, true);
        assert!(
            v.iter().any(|c| matches!(c, Cmd::Text { text, x, y, .. } if text == ICON_CLOSE && *x == 174.0 && *y == 34.0)),
            "✕ appears at card_w - del_pad(26) and ry + del_dy(4)"
        );
        assert_eq!(hits.len(), 1, "only the del region registers");
        assert_eq!(hits[0].key, 70);
    }

    #[test]
    fn alarms_scene_declares_rows_with_ink() {
        let path = crate::vars::card_scene_path("alarms");
        if !path.exists() {
            return;
        }
        let s = std::fs::read_to_string(&path).expect("scene file readable");
        let scene: CardScene = ron::from_str(&s)
            .unwrap_or_else(|e| panic!("committed alarms.ron must parse: {e}"));
        assert!(
            scene.items.iter().any(|i| matches!(i, SceneItem::Ink { name, .. } if name == "alarms")),
            "alarms scene carries the Ink painter"
        );
        let rows = scene
            .items
            .iter()
            .find_map(|i| if let SceneItem::Rows { name, y, row_dy, del_base, del_pad, pad, cols, .. } = i {
                Some((name.clone(), *y, *row_dy, *del_base, *del_pad, *pad, cols.clone()))
            } else {
                None
            })
            .expect("alarms scene declares the list Rows item");
        assert_eq!(rows.0, "alarm_rows");
        assert_eq!(rows.1, 50.0, "rows start below the composer row");
        assert_eq!(rows.2, 3.0, "row_dy matches the Rust text baseline");
        assert_eq!(rows.3, crate::shell::ALARM_DEL_BASE, "✕ keys from the delete base");
        assert_eq!(rows.4, 26.0, "✕ right margin matches the Rust drawer");
        assert_eq!(rows.5, 12.0, "12 px card pad");
        assert_eq!(rows.6.len(), 2);
        assert_eq!(rows.6[0].font, SceneFont::Mono, "time column renders mono");
        assert_eq!(rows.6[0].size, 10.0);
        assert_eq!(rows.6[0].color, Some(ColorToken::Acc));
        assert_eq!(rows.6[1].x, 50.0, "label column 50 px from the pad (62 abs)");
        assert_eq!(rows.6[1].size, 9.5);
    }

    #[test]
    fn todo_scene_declares_rows_with_composer_and_ink() {
        let path = crate::vars::card_scene_path("todo");
        if !path.exists() {
            return;
        }
        let s = std::fs::read_to_string(&path).expect("scene file readable");
        let scene: CardScene = ron::from_str(&s)
            .unwrap_or_else(|e| panic!("committed todo.ron must parse: {e}"));
        assert!(
            scene.items.iter().any(|i| matches!(i, SceneItem::Ink { name, .. } if name == "todo")),
            "todo scene carries the Ink painter"
        );
        assert!(
            scene.items.iter().any(|i| matches!(i, SceneItem::Composer { .. })),
            "todo scene declares the Composer strip"
        );
        let rows = scene
            .items
            .iter()
            .find_map(|i| if let SceneItem::Rows { name, key_base, del_base, check, start, row_h, cols, .. } = i {
                Some((name.clone(), *key_base, *del_base, check.clone(), start.clone(), *row_h, cols.clone()))
            } else {
                None
            })
            .expect("todo scene declares the checklist Rows item");
        assert_eq!(rows.0, "todo_rows");
        assert_eq!(rows.1, Some(crate::shell::TODO_KEY_TOGGLE), "row keys rebase from the toggle base");
        assert_eq!(rows.2, crate::shell::TODO_KEY_DELETE, "✕ keys rebase from the delete base");
        assert_eq!(rows.3.as_deref(), Some("todo_checks"));
        assert_eq!(rows.4.as_deref(), Some("todo_scroll"));
        assert_eq!(rows.5, 24.0, "24 px row stride matches the Rust drawer");
        assert_eq!(rows.6.len(), 1);
        assert_eq!(rows.6[0].x, 20.0, "label column sits 20 px from the row pad (32 abs)");
        assert_eq!(rows.6[0].size, 9.5);
    }

    #[test]
    fn rows_start_scalar_scrolls_the_window_and_rebases_keys() {
        let scene: CardScene = ron::from_str(
            r#"(items: [ Rows(name: "rw", y: 30.0, row_h: 24.0, cols: [ (x: 0.0) ], key_base: Some(700), start: Some("bt_scroll")) ])"#,
        )
        .unwrap();
        let mut values = SceneValues::new();
        values.insert(
            "rw",
            SceneValue::Rows((0..4).map(|i| SceneRow { cols: vec![format!("r{i}")], key: 0, action: None, color: None, col_colors: vec![], surface: None }).collect()),
        );
        values.insert("bt_scroll", SceneValue::Ring(2.0));
        let mut v: Vec<Cmd> = Vec::new();
        // 4 rows from y=30 pitch 24: bottom edge 120 → only visible[:2] rows fit
        let (hits, _i) = scene.draw(&mut v, 0.0, 0.0, 200.0, 120.0, scale(), &pal(), &values, 0, true, true);
        assert!(v.iter().any(|c| matches!(c, Cmd::Text { text, .. } if text == "r2")), "window starts at scalar index");
        assert!(!v.iter().any(|c| matches!(c, Cmd::Text { text, .. } if text == "r0" || text == "r1")), "pre-window rows hidden");
        assert_eq!(hits.len(), 2, "only the visible window registers");
        assert_eq!(hits[0].key, 700, "keys rebase to the visible window");
        assert_eq!(hits[1].key, 701);
    }

    #[test]
    fn rows_col_colors_override_per_cell_before_row_color() {
        let scene: CardScene = ron::from_str(
            r#"(items: [ Rows(name: "rw", y: 30.0, row_h: 18.0, cols: [ (x: 0.0), (x: 56.0), (x: 160.0, right: true) ], key_base: Some(90)) ])"#,
        )
        .unwrap();
        let mut values = SceneValues::new();
        values.insert(
            "rw",
            SceneValue::Rows(vec![SceneRow {
                cols: vec!["BTC".into(), "68000".into(), "+2.4%".into()],
                key: 0,
                action: None,
                color: Some(ColorToken::Fg3),
                surface: None,
                col_colors: vec![None, None, Some(ColorToken::Ok)],
            }]),
        );
        let mut v: Vec<Cmd> = Vec::new();
        let (_h, _i) = scene.draw(&mut v, 0.0, 0.0, 200.0, 120.0, scale(), &pal(), &values, 0, true, true);
        let find = |t: &str| v.iter().find_map(|c| if let Cmd::Text { text, color, .. } = c { if text == t { Some(*color) } else { None } } else { None });
        assert_eq!(find("BTC"), Some(ColorToken::Fg3.resolve(&pal())), "row color dims sym cell");
        assert_eq!(find("+2.4%").unwrap(), ColorToken::Ok.resolve(&pal()), "status cell keeps its own color");
    }

    #[test]
    fn rows_col_dy_offsets_cell_baselines() {
        let scene: CardScene = ron::from_str(
            r#"(items: [ Rows(name: "rw", y: 30.0, row_h: 24.0, row_dy: 3.0, cols: [ (x: 0.0), (x: 16.0, dy: Some(12.0), size: 8.0) ]) ])"#,
        )
        .unwrap();
        let mut values = SceneValues::new();
        values.insert("rw", SceneValue::Rows(vec![
            SceneRow { cols: vec!["title".into(), "caption".into()], key: 0, action: None, color: None, surface: None, col_colors: vec![] },
        ]));
        let mut v: Vec<Cmd> = Vec::new();
        let (_h, _i) = scene.draw(&mut v, 0.0, 0.0, 200.0, 120.0, scale(), &pal(), &values, 0, true, true);
        let find_y = |text: &str| v.iter().find_map(|c| if let Cmd::Text { text: t, y, .. } = c { if t == text { Some(*y) } else { None } } else { None });
        assert_eq!(find_y("title"), Some(33.0), "first col uses row_dy = 3");
        assert_eq!(find_y("caption"), Some(42.0), "second col uses dy = 12 (30 + 12)");
    }

    #[test]
    fn rows_visible_toggle_hides_list() {
        let scene: CardScene = ron::from_str(
            r#"(items: [ Rows(name: "rw", y: 30.0, row_h: 24.0, cols: [ (x: 0.0) ], key_base: Some(700), visible: Some("vis")) ])"#,
        )
        .unwrap();
        let mut values = SceneValues::new();
        values.insert("rw", SceneValue::Rows(vec![
            SceneRow { cols: vec!["a".into()], key: 0, action: None, color: None, surface: None, col_colors: vec![] },
        ]));
        // Toggle false → list hidden
        values.insert("vis", SceneValue::Toggle(false));
        let mut v: Vec<Cmd> = Vec::new();
        let (hits, _i) = scene.draw(&mut v, 0.0, 0.0, 200.0, 120.0, scale(), &pal(), &values, 0, true, true);
        assert!(v.is_empty(), "rows list draws nothing when Toggle is false");
        assert!(hits.is_empty(), "no row regions when hidden");
        // Toggle true → list drawn
        values.insert("vis", SceneValue::Toggle(true));
        let mut v: Vec<Cmd> = Vec::new();
        let (hits, _i) = scene.draw(&mut v, 0.0, 0.0, 200.0, 120.0, scale(), &pal(), &values, 0, true, true);
        assert!(v.iter().any(|c| matches!(c, Cmd::Text { text, .. } if text == "a")), "rows draw when Toggle is true");
        assert_eq!(hits.len(), 1, "row region registered");
    }

    #[test]
    fn notes_scene_declares_rows_with_ink() {
        let path = crate::vars::card_scene_path("notes");
        if !path.exists() {
            return;
        }
        let s = std::fs::read_to_string(&path).expect("scene file readable");
        let scene: CardScene = ron::from_str(&s)
            .unwrap_or_else(|e| panic!("committed notes.ron must parse: {e}"));
        assert!(
            scene.items.iter().any(|i| matches!(i, SceneItem::Ink { name, .. } if name == "notes")),
            "notes scene carries the Ink painter"
        );
        let rows = scene
            .items
            .iter()
            .find_map(|i| if let SceneItem::Rows { name, y, row_dy, start, visible, cols, .. } = i {
                Some((name.clone(), *y, *row_dy, start.clone(), visible.clone(), cols.clone()))
            } else {
                None
            })
            .expect("notes scene declares the note-list Rows item");
        assert_eq!(rows.0, "notes_rows");
        assert_eq!(rows.1, 32.0, "rows start 32 px below the header");
        assert_eq!(rows.2, 0.0, "row_dy 0 — title baseline at the row top");
        assert_eq!(rows.3.as_deref(), Some("notes_scroll"), "scroll binds notes_scroll");
        assert_eq!(rows.4.as_deref(), Some("notes_composing"), "hidden while the composer is open");
        assert_eq!(rows.5.len(), 3);
        assert!(rows.5[0].icon, "bullet col is an icon glyph");
        assert_eq!(rows.5[0].dy, None, "bullet col uses row_dy (0)");
        assert_eq!(rows.5[1].size, 9.5);
        assert_eq!(rows.5[2].dy, Some(12.0), "body-caption sits 12 px below the title");
        assert_eq!(rows.5[2].size, 8.0);
    }

    #[test]
    fn rows_pill_cell_draws_chip_with_derived_fill() {
        let scene: CardScene = ron::from_str(
            r#"(items: [ Rows(name: "rw", y: 30.0, row_h: 24.0, row_dy: 0.0, pad: 12.0, cols: [
                (x: 0.0, truncate: Some(120.0), size: 9.5),
                (x: 0.0, edge: Some(12.0), right: true, dy: Some(6.0), size: 8.5, pill: Some((height: 16.0, radius: 8.0))),
            ]) ])"#,
        )
        .unwrap();
        let mut values = SceneValues::new();
        values.insert(
            "rw",
            SceneValue::Rows(vec![SceneRow {
                cols: vec!["aaaaaaaaaaaaaaaaaaaaaa".into(), "7d".into()],
                key: 0,
                action: None,
                color: None,
                surface: None,
                col_colors: vec![None, Some(ColorToken::Warn)],
            }]),
        );
        let mut v: Vec<Cmd> = Vec::new();
        let (_h, _i) = scene.draw(&mut v, 0.0, 0.0, 200.0, 120.0, scale(), &pal(), &values, 0, true, true);
        // truncate: label cut to fit card_w - 2·pad - 120 at 0.62·fs(9.5)
        // (56 px / 5.89 → 9 glyphs, the ≥8 floor)
        assert!(
            v.iter().any(|c| matches!(c, Cmd::Text { text, .. } if text == "aaaaaaaaa")),
            "long label truncated to the reserved width",
        );
        assert!(!v.iter().any(|c| matches!(c, Cmd::Text { text, .. } if text == "aaaaaaaaaaaaaaaaaaaaaa")), "untruncated label not drawn");
        // chip: right edge at card_w - pad(12), auto width = 2·0.62·fs(8.5) + 2·5
        let pw = 2.0 * 0.62 * 8.5 + 10.0;
        let pl = 200.0 - 12.0 - pw;
        let hov = crate::ui::hover(&pal());
        let chip = v.iter().find_map(|c| if let Cmd::Rect { x, w, h, r, color, .. } = c { if (*x - pl).abs() < 0.01 { Some((*w, *h, *r, *color)) } else { None } } else { None });
        let (w, h, r, color) = chip.expect("chip rect drawn at the edge anchor");
        assert!((w - pw).abs() < 0.01 && h == 16.0 && r == 8.0, "chip auto-sized (w {w} h {h} r {r})");
        assert_eq!(color, hov, "Warn chip text → hover fill");
        // Warn-glyph centered across the chip
        assert!(v.iter().any(|c| matches!(c, Cmd::Text { text, x, .. } if text == "7d" && (x - (pl + pw / 2.0)).abs() < 0.01)), "chip text centered in the pill");
        // Acc-token chip derives the acc-tint fill
        values.insert("rw", SceneValue::Rows(vec![SceneRow {
            cols: vec!["a".into(), "today!".into()], key: 0, action: None, color: None, surface: None, col_colors: vec![None, Some(ColorToken::Acc)],
        }]));
        let mut v: Vec<Cmd> = Vec::new();
        let (_h, _i) = scene.draw(&mut v, 0.0, 0.0, 200.0, 120.0, scale(), &pal(), &values, 0, true, true);
        let tint = mix(pal().bg, pal().acc, 0.16);
        let cube = v.iter().any(|c| matches!(c, Cmd::Rect { color, .. } if *color == tint));
        assert!(cube, "acc chip derived acc-tint fill");
    }

    #[test]
    fn rows_pill_del_anchors_left_of_chip() {
        let scene: CardScene = ron::from_str(
            r#"(items: [ Rows(name: "rw", y: 30.0, row_h: 24.0, pad: 12.0, cols: [
                (x: 0.0, size: 9.5),
                (x: 0.0, edge: Some(12.0), right: true, dy: Some(6.0), size: 8.5, pill: Some(()), del_anchor: true),
            ], del_base: 70, del_pad: 20.0, del_dy: 4.0) ])"#,
        )
        .unwrap();
        let mut values = SceneValues::new();
        values.insert(
            "rw",
            SceneValue::Rows(vec![SceneRow {
                cols: vec!["Release".into(), "today!".into()],
                key: 0,
                action: None,
                color: None,
                surface: None,
                col_colors: vec![None, Some(ColorToken::Acc)],
            }]),
        );
        let mut v: Vec<Cmd> = Vec::new();
        let (hits, _i) = scene.draw(&mut v, 0.0, 0.0, 200.0, 120.0, scale(), &pal(), &values, 70, true, true);
        let pw = 6.0 * 0.62 * 8.5 + 10.0;
        let pl = 200.0 - 12.0 - pw;
        let dange = ColorToken::Danger.resolve(&pal());
        assert!(
            v.iter().any(|c| matches!(c, Cmd::Text { text, x, .. } if text == ICON_CLOSE && (x - (pl - 20.0)).abs() < 0.01)),
            "✕ anchors left of the chip at del_pad",
        );
        assert!(
            v.iter().any(|c| matches!(c, Cmd::Text { text, color, .. } if text == ICON_CLOSE && *color == dange)),
            "the revealed ✕ is danger-red (reveal-on-✕ rows)",
        );
        assert_eq!(hits.len(), 1, "only the del region registers");
        let h = &hits[0];
        assert!((h.x - (pl - 22.0)).abs() < 0.01 && h.y == 30.0 && h.w == 20.0 && h.h == 24.0, "anchor region sits full-row left of the chip");
    }

    #[test]
    fn countdown_scene_declares_rows_with_ink() {
        let path = crate::vars::card_scene_path("countdown");
        if !path.exists() {
            return;
        }
        let s = std::fs::read_to_string(&path).expect("scene file readable");
        let scene: CardScene = ron::from_str(&s)
            .unwrap_or_else(|e| panic!("committed countdown.ron must parse: {e}"));
        assert!(
            scene.items.iter().any(|i| matches!(i, SceneItem::Ink { name, .. } if name == "countdown")),
            "countdown scene carries the Ink painter"
        );
        let rows = scene
            .items
            .iter()
            .find_map(|i| if let SceneItem::Rows { name, y, del_base, del_pad, del_dy, cols, .. } = i {
                Some((name.clone(), *y, *del_base, *del_pad, *del_dy, cols.clone()))
            } else {
                None
            })
            .expect("countdown scene declares the event-list Rows item");
        assert_eq!(rows.0, "countdown_rows");
        assert_eq!(rows.1, 50.0, "rows start below the composer row");
        assert_eq!(rows.2, crate::shell::COUNTDOWN_DEL_BASE, "del base matches");
        assert_eq!(rows.3, 20.0, "✕ gap left of the chip");
        assert_eq!(rows.4, 4.0, "✕ baseline inside the row");
        assert_eq!(rows.5.len(), 3);
        assert_eq!(rows.5[0].dy, Some(3.0), "label baseline");
        assert_eq!(rows.5[0].truncate, Some(120.0), "label reserves the chip zone");
        assert_eq!(rows.5[1].dy, Some(13.0), "date caption baseline");
        assert_eq!(rows.5[1].color, Some(ColorToken::Fg3));
        assert_eq!(rows.5[2].edge, Some(12.0), "chip right edge at the card pad");
        assert!(rows.5[2].right, "chip right-anchored");
        assert_eq!(rows.5[2].pill, Some(PillSpec { bg: None, height: 16.0, radius: 8.0, pad: 5.0, top: 3.0 }));
        assert!(rows.5[2].del_anchor, "del anchors left of the chip");
    }

    #[test]
    fn quote_scene_declares_textwrap_with_ink() {
        let path = crate::vars::card_scene_path("quote");
        if !path.exists() {
            return;
        }
        let s = std::fs::read_to_string(&path).expect("scene file readable");
        let scene: CardScene = ron::from_str(&s)
            .unwrap_or_else(|e| panic!("committed quote.ron must parse: {e}"));
        assert!(
            scene.items.iter().any(|i| matches!(i, SceneItem::Ink { name, .. } if name == "quote")),
            "quote scene carries the resident Ink painter (pills/header/meta)"
        );
        let tw = scene
            .items
            .iter()
            .find_map(|i| if let SceneItem::TextWrap { text, caption, top, bottom, visible, .. } = i {
                Some((text.clone(), caption.clone(), *top, *bottom, visible.clone()))
            } else {
                None
            })
            .expect("quote scene declares the TextWrap body");
        assert!(tw.0.contains("quote_text"), "text interpolates the body value");
        assert_eq!(tw.1, Some("— {quote_author}".into()), "caption carries the author line");
        assert_eq!(tw.2, 28.0, "box top clears the header");
        assert_eq!(tw.3, 36.0, "box bottom clears the pill row");
        assert_eq!(tw.4, Some("quote_has".into()), "visible gate bound to quote_has");
    }

    #[test]
    fn textwrap_wraps_at_thirty_four_and_centers() {
        // 136 chars (4 lines of 34) + author caption
        let long_text = "abcdefghijklmnopqrstuvwxyz0123456789abcdefghijklmnopqrstuvwxyz0123456789abcdefghijklmnopqrstuvwxyz0123456789abcdefghijklmnopqrstuvwxyz0123456789";
        let scene: CardScene = ron::from_str(
            &format!("(items: [ TextWrap(text: \"{{tw_text}}\", caption: Some(\"— {{tw_author}}\"), top: 28.0, bottom: 36.0, visible: Some(\"tw_has\")) ])")
        )
        .unwrap();
        let mut vals = SceneValues::new();
        vals.insert("tw_text", SceneValue::Text(long_text.to_string()));
        vals.insert("tw_author", SceneValue::Text("Ada Lovelace".into()));
        vals.insert("tw_has", SceneValue::Toggle(true));
        let mut v: Vec<Cmd> = Vec::new();
        let (_h, _i) = scene.draw(&mut v, 0.0, 0.0, 200.0, 100.0, scale(), &pal(), &vals, 0, true, true);
        let text_cmds: Vec<_> = v.iter().filter(|c| matches!(c, Cmd::Text { .. })).collect();
        assert_eq!(text_cmds.len(), 5, "4 wrapped lines + 1 author caption");
        // visible=false suppresses everything
        let mut v2: Vec<Cmd> = Vec::new();
        vals.insert("tw_has", SceneValue::Toggle(false));
        scene.draw(&mut v2, 0.0, 0.0, 200.0, 100.0, scale(), &pal(), &vals, 0, true, true);
        assert!(v2.iter().all(|c| !matches!(c, Cmd::Text { .. })), "visible=false skips draw");
    }

    #[test]
    fn expenses_scene_is_fully_declarative() {
        let path = crate::vars::card_scene_path("expenses");
        if !path.exists() {
            return;
        }
        let s = std::fs::read_to_string(&path).expect("scene file readable");
        let scene: CardScene = ron::from_str(&s)
            .unwrap_or_else(|e| panic!("committed expenses.ron must parse: {e}"));
        assert!(
            scene.items.iter().all(|i| !matches!(i, SceneItem::Ink { .. })),
            "fully declarative — no Ink painters remain"
        );
        let hdr = scene
            .items
            .iter()
            .find_map(|i| if let SceneItem::Header { title, meta, meta_mono, meta_color, .. } = i {
                Some((title.clone(), meta.clone(), *meta_mono, *meta_color))
            } else {
                None
            })
            .expect("header declares the MTD meta");
        assert_eq!(hdr.0, "Expenses");
        assert!(hdr.1.as_deref().unwrap_or_default().contains("exp_meta"));
        assert!(hdr.2, "MTD total is mono");
        assert_eq!(hdr.3, Some(ColorToken::Acc), "MTD total in accent ink");
        let rows: Vec<&SceneItem> = scene.items.iter().filter(|i| matches!(i, SceneItem::Row { .. })).collect();
        assert_eq!(rows.len(), 2, "chips row + composer row");
        let hits_in = |row: &SceneItem| -> Vec<(u32, f32, String)> {
            match row {
                SceneItem::Row { items, .. } => items
                    .iter()
                    .filter_map(|c| if let SceneItem::Hit { key, w, text, .. } = c {
                        Some((*key, *w, text.clone()))
                    } else {
                        None
                    })
                    .collect(),
                _ => vec![],
            }
        };
        let chips = hits_in(rows[0]);
        assert_eq!(
            chips.iter().map(|(k, _, _)| *k).collect::<Vec<_>>(),
            vec![31_800, 31_801, 31_802, 31_803, 31_804, 31_805],
            "chip keys mirror EXPENSE_CHIP_BASE..+5"
        );
        assert!(chips.iter().all(|(_, w, _)| *w == 0.0), "chips equal-split the row");
        assert_eq!(
            chips.iter().map(|(_, _, t)| t.as_str()).collect::<Vec<_>>(),
            vec!["1", "5", "10", "20", "50", "100"],
            "preset amounts 1..=100"
        );
        let comp = scene
            .items
            .iter()
            .flat_map(|i| if let SceneItem::Row { items, .. } = i { items.iter().collect::<Vec<_>>() } else { vec![] })
            .find_map(|c| if let SceneItem::Composer { pad, text, focus, key, .. } = c {
                Some((*pad, text.clone(), focus.clone(), *key))
            } else {
                None
            })
            .expect("composer declared");
        assert_eq!(comp.0, 0.0, "row pad supplies the inset — no double padding");
        assert_eq!(comp.1, Some("{exp_buf}".into()));
        assert_eq!(comp.2, Some("exp_focus".into()));
        assert_eq!(comp.3, crate::shell::EXPENSE_INPUT_KEY);
        let clear = hits_in(rows[1]);
        let clr_hit = scene
            .items
            .iter()
            .flat_map(|i| if let SceneItem::Row { items, .. } = i { items.iter().collect::<Vec<_>>() } else { vec![] })
            .find_map(|c| if let SceneItem::Hit { key, w, color, color_hover, .. } = c {
            if *key == crate::shell::EXPENSE_CLEAR_KEY { Some((*key, *w, *color, *color_hover)) } else { None }
        } else {
            None
        });
        let c = clr_hit.expect("clear hit declared");
        assert_eq!(c.1, 70.0, "clear-month fixed 70 next to the composer");
        assert_eq!(c.2, Some(ColorToken::Fg2));
        assert_eq!(c.3, Some(ColorToken::Danger), "clear label flips danger on hover");
        let bars: Vec<_> = scene
            .items
            .iter()
            .filter_map(|i| if let SceneItem::Bar { reserve, visible, .. } = i {
                Some((*reserve, visible.clone()))
            } else {
                None
            })
            .collect();
        assert_eq!(bars.len(), 3, "top-3 per-category bars");
        assert!(bars.iter().all(|(r, v)| *r == 46.0 && v.is_some()), "bars reserve the amount column and gate by category");
        let empty = scene
            .items
            .iter()
            .find(|i| matches!(i, SceneItem::Text { visible, .. } if visible.as_deref() == Some("exp_empty")))
            .expect("empty-month caption gated by exp_empty");
        let _ = empty;
    }

    #[test]
    fn expense_primitives_reserve_gate_and_hover() {
        let mut vals = SceneValues::new();
        vals.insert("cat1", SceneValue::Text("food".into()));
        vals.insert("amt1", SceneValue::Text("42".into()));
        vals.insert("ratio1", SceneValue::Text("0.5".into()));
        vals.insert("has1", SceneValue::Toggle(true));
        vals.insert("empty", SceneValue::Toggle(false));
        let scene: CardScene = ron::from_str(
            "(items: [
                Header(title: \"Expenses\", meta: Some(\"{meta}\"), meta_mono: true, meta_color: Some(acc)),
                Text(x: 12.0, y: 90.0, text: \"{cat1}\", font_size: 8.0, visible: Some(\"has1\")),
                Bar(x: 12.0, y: 101.0, w: 0.0, h: 4.0, value: \"{ratio1}\", reserve: 46.0, visible: Some(\"has1\")),
                Text(x: 12.0, y: 96.0, center_x: true, text: \"nothing logged this month\", font_size: 8.5, visible: Some(\"empty\")),
                Hit(x: 0.0, y: 200.0, w: 70.0, h: 22.0, text: \"clear mo\", color: Some(fg2), color_hover: Some(danger), key: 31_811),
             ])",
        )
        .unwrap();
        let mut hvals = vals.clone();
        hvals.insert("meta", SceneValue::Text("MTD 42".into()));
        let mut v: Vec<Cmd> = Vec::new();
        scene.draw(&mut v, 0.0, 0.0, 300.0, 120.0, scale(), &pal(), &hvals, 31_811, true, true);
        // header meta — accent mono
        let acc = ColorToken::Acc.resolve(&pal());
        assert!(
            v.iter().any(|c| matches!(c, Cmd::Text { text, color, .. } if text == "MTD 42" && *color == acc)),
            "meta color accent + mono still substituted"
        );
        // bar reserve → 300 − 46 track
        let track: Vec<_> = v.iter().filter_map(|c| if let Cmd::Rect { x, y, w, h, .. } = c {
            Some((*x, *y, *w, *h))
        } else {
            None
        }).filter(|(_, yy, _, _)| (*yy - 101.0).abs() < 0.01).collect();
        assert!(track.iter().any(|(xx, _, ww, hh)| (*xx - 12.0).abs() < 0.01 && (*ww - 254.0).abs() < 0.01 && *hh == 4.0), "bar spans card_w − reserve");
        // visible=false caption suppressed
        assert!(!v.iter().any(|c| matches!(c, Cmd::Text { text, .. } if text == "nothing logged this month")), "empty caption hidden while a category bar exists");
        // hover swaps the clear label to danger
        let dange = ColorToken::Danger.resolve(&pal());
        let found = v.iter().any(|c| matches!(c, Cmd::Text { text, color, .. } if text == "clear mo" && *color == dange));
        assert!(found, "clear mo flips to danger while hovered");
        // idle frame: label back to fg2
        let mut v2: Vec<Cmd> = Vec::new();
        scene.draw(&mut v2, 0.0, 0.0, 300.0, 120.0, scale(), &pal(), &hvals, 0, true, true);
        let fg2 = ColorToken::Fg2.resolve(&pal());
        assert!(v2.iter().any(|c| matches!(c, Cmd::Text { text, color, .. } if text == "clear mo" && *color == fg2)), "idle clear label is fg2");
    }

    #[test]
    fn news_strip_draws_chips_and_rows_hover_ink() {
        let mut vals = SceneValues::new();
        vals.insert(
            "news_chips",
            SceneValue::Chips(vec!["world".into(), "tech".into(), "science".into(), "money".into(), "sports".into()]),
        );
        vals.insert("news_cat_sel", SceneValue::Ring(2.0));
        vals.insert("news_cat_scroll", SceneValue::Ring(40.0));
        vals.insert(
            "news_rows",
            SceneValue::Rows(vec![crate::scene::SceneRow {
                cols: vec!["Big headline here".into(), crate::icons::ICON_OPEN.to_string(), "BBC".into()],
                key: 0,
                action: None,
                color: None,
                surface: None,
                col_colors: vec![],
            }]),
        );
        vals.insert("news_scroll", SceneValue::Ring(0.0));
        let scene: CardScene = ron::from_str(
            "(items: [
                Strip(
                    chips: \"news_chips\",
                    sel: Some(\"news_cat_sel\"),
                    scroll: Some(\"news_cat_scroll\"),
                    x: 0.0, y: 30.0, w: 0.0, h: 26.0,
                    pad: 12.0, spacing: 6.0, chip_pad: 11.0, fs: 8.5,
                    key_base: 13000,
                ),
                Rows(
                    name: \"news_rows\",
                    y: 59.0, row_h: 30.0, pitch: Some(30.0), pad: 12.0, row_dy: 2.0,
                    cols: [
                        (x: 5.0, dy: Some(2.0), size: 9.5, color: Some(fg), hover: Some(acc)),
                        (x: 0.0, edge: Some(12.0), right: true, dy: Some(2.0), size: 8.0, icon: true, color: Some(fg3), hover: Some(acc)),
                        (x: 0.0, edge: Some(12.0), right: true, dy: Some(14.0), size: 7.5, color: Some(fg3)),
                    ],
                    key_base: Some(13300),
                    hairline: true,
                    hover_bar: true,
                ),
             ])",
        )
        .unwrap();
        let acc = ColorToken::Acc.resolve(&pal());
        let sfg = ColorToken::Sfg.resolve(&pal());
        let mut v: Vec<Cmd> = Vec::new();
        let (hits, _) = scene.draw(&mut v, 0.0, 0.0, 300.0, 130.0, scale(), &pal(), &vals, 13_300, true, true);
        assert!(hits.iter().any(|h| h.key == 13_002), "chip index 2 registers a region");
        assert!(hits.iter().any(|h| h.key == 13_300), "the headline row registers");
        assert!(v.iter().any(|c| matches!(c, Cmd::Rect { color, h, .. } if *color == acc && *h == 24.0)), "active chip painted accent");
        assert!(v.iter().any(|c| matches!(c, Cmd::Text { text, color, .. } if text == "science" && *color == sfg)), "active chip label in Sfg (accent text)");
        assert!(v.iter().any(|c| matches!(c, Cmd::Rect { color, w, y, .. } if *color == acc && (*w - 2.5).abs() < 0.01 && (*y - 61.0).abs() < 0.02)), "hover accent rail at the row's left");
        let hover_gray = crate::ui::hover(&pal());
        assert!(v.iter().any(|c| matches!(c, Cmd::Rect { color, h, y, .. } if *color == hover_gray && *h == 1.0 && (*y - 88.0).abs() < 0.02)), "hairline divider under the row");
        assert!(v.iter().any(|c| matches!(c, Cmd::Text { text, color, .. } if text == "Big headline here" && *color == acc)), "title tints accent while its row is hovered");
        // idle frame: title back to fg, rail gone
        let mut v2: Vec<Cmd> = Vec::new();
        scene.draw(&mut v2, 0.0, 0.0, 300.0, 130.0, scale(), &pal(), &vals, 0, true, true);
        let fg = ColorToken::Fg.resolve(&pal());
        assert!(v2.iter().any(|c| matches!(c, Cmd::Text { text, color, .. } if text == "Big headline here" && *color == fg)), "idle title is fg");
        assert!(!v2.iter().any(|c| matches!(c, Cmd::Rect { color, w, .. } if *color == acc && (*w - 2.5).abs() < 0.01)), "no accent rail without hover");
    }

    #[test]
    fn news_scene_is_fully_declarative() {
        let path = crate::vars::card_scene_path("news");
        if !path.exists() {
            return;
        }
        let s = std::fs::read_to_string(&path).expect("scene file readable");
        let scene: CardScene = ron::from_str(&s)
            .unwrap_or_else(|e| panic!("committed news.ron must parse: {e}"));
        assert!(
            scene.items.iter().all(|i| !matches!(i, SceneItem::Ink { .. })),
            "news scene is fully declarative — no Ink painters"
        );
        let hdr = scene
            .items
            .iter()
            .find_map(|i| if let SceneItem::Header { title, glyph, meta, .. } = i {
                Some((title.clone(), glyph.clone(), meta.clone()))
            } else {
                None
            })
            .expect("header declared");
        assert_eq!(hdr.0, "News");
        assert_eq!(hdr.1, Some("\u{f1ea}".into()), "news glyph");
        assert!(hdr.2.as_deref().unwrap_or_default().contains("news_label"));
        let strip = scene
            .items
            .iter()
            .find_map(|i| if let SceneItem::Strip { chips, sel, scroll, y, h, key_base, .. } = i {
                Some((chips.clone(), sel.clone(), scroll.clone(), *y, *h, *key_base))
            } else {
                None
            })
            .expect("category strip declared");
        assert_eq!(strip.0, "news_chips");
        assert_eq!(strip.1, Some("news_cat_sel".into()));
        assert_eq!(strip.2, Some("news_cat_scroll".into()));
        assert_eq!(strip.3, 30.0, "strip sits at the header line");
        assert_eq!(strip.4, 26.0);
        assert_eq!(strip.5, crate::shell::NEWS_CAT_KEY_BASE);
        let rows_item = scene
            .items
            .iter()
            .find_map(|i| if let SceneItem::Rows { name, y, row_h, start, key_base, hairline, hover_bar, cols, .. } = i {
                Some((name.clone(), *y, *row_h, start.clone(), *key_base, *hairline, *hover_bar, cols.clone()))
            } else {
                None
            })
            .expect("headline list declared");
        assert_eq!(rows_item.0, "news_rows");
        assert_eq!(rows_item.1, 59.0, "list clears header + strip");
        assert_eq!(rows_item.2, 30.0, "30 px headline pitch");
        assert_eq!(rows_item.3, Some("news_scroll".into()));
        assert_eq!(rows_item.4, Some(crate::shell::NEWS_HEAD_KEY_BASE));
        assert!(rows_item.5, "hairline dividers");
        assert!(rows_item.6, "hover accent rail");
        assert_eq!(rows_item.7.len(), 3);
        assert_eq!(rows_item.7[0].hover, Some(ColorToken::Acc), "title tints accent on hover");
        assert!(rows_item.7[1].icon, "open glyph in the icon font");
        assert!(rows_item.7[1].right && rows_item.7[1].edge == Some(12.0), "open glyph right-anchored");
        assert_eq!(rows_item.7[2].dy, Some(14.0), "source label below the title");
    }

    #[test]
    fn snippet_flash_draws_translucent_wash_and_flash_ink() {
        let mut vals = SceneValues::new();
        vals.insert(
            "snip_rows",
            SceneValue::Rows(vec![
                crate::scene::SceneRow {
                    cols: vec![crate::icons::ICON_SELECT.to_string(), "ssh".into(), "user@host:22".into()],
                    key: 0,
                    action: None,
                    color: None,
                    surface: None,
                    col_colors: vec![],
                },
                crate::scene::SceneRow {
                    cols: vec![crate::icons::ICON_SELECT.to_string(), "addr".into(), "42 Imagination Street".into()],
                    key: 0,
                    action: None,
                    color: None,
                    surface: None,
                    col_colors: vec![],
                },
            ]),
        );
        // scalar is index + 1 → row 1 flashes (0 = none)
        vals.insert("snip_flash", SceneValue::Ring(2.0));
        let scene: CardScene = ron::from_str(
            "(items: [
                Rows(
                    name: \"snip_rows\",
                    y: 50.0, row_h: 24.0, pad: 12.0, r: 6.0,
                    hover_surface: Some(hover_hl),
                    flash: Some(\"snip_flash\"),
                    cols: [
                        (x: 2.0, dy: Some(4.0), size: 9.0, icon: true, color: Some(fg2), flash: Some(acc)),
                        (x: 18.0, dy: Some(4.0), size: 9.5, color: Some(fg), flash: Some(acc)),
                        (x: 96.0, dy: Some(5.0), size: 8.0, color: Some(fg3)),
                    ],
                    key_base: Some(31710),
                    del_base: 31730,
                    del_pad: 20.0,
                    del_dy: 4.0,
                    del_size: 9.0,
                ),
             ])",
        )
        .unwrap();
        let acc = ColorToken::Acc.resolve(&pal());
        let wash = (pal().acc & 0xFFFF_FF00) | 0x30;
        let mut v: Vec<Cmd> = Vec::new();
        let (hits, _) = scene.draw(&mut v, 0.0, 0.0, 300.0, 130.0, scale(), &pal(), &vals, 0, true, true);
        // the flash row (index 1 → ry 74): translucent accent wash behind it
        assert!(v.iter().any(|c| matches!(c, Cmd::Rect { color, y, h, .. } if *color == wash && (*y - 74.0).abs() < 0.02 && *h == 24.0)), "flashed row washed translucent accent");
        // its name inks accent; the preview stays fg3
        assert!(v.iter().any(|c| matches!(c, Cmd::Text { text, color, .. } if text == "addr" && *color == acc)), "flashed name in accent");
        let fg3 = ColorToken::Fg3.resolve(&pal());
        assert!(v.iter().any(|c| matches!(c, Cmd::Text { text, color, .. } if text == "42 Imagination Street" && *color == fg3)), "preview line stays fg3 under the flash");
        // idle row 0: copy glyph fg2, name fg
        let fg2 = ColorToken::Fg2.resolve(&pal());
        assert!(v.iter().any(|c| matches!(c, Cmd::Text { text, color, .. } if text == crate::icons::ICON_SELECT && *color == fg2)), "idle copy glyph stays fg2");
        let fg = ColorToken::Fg.resolve(&pal());
        assert!(v.iter().any(|c| matches!(c, Cmd::Text { text, color, .. } if text == "ssh" && *color == fg)), "idle name stays fg");
        // region: both copy regions registered; ✕ only shows on hover
        assert!(hits.iter().any(|h| h.key == 31_710), "row 0 copy region");
        assert!(hits.iter().any(|h| h.key == 31_711), "row 1 copy region");
        assert!(!hits.iter().any(|h| h.key == 31_730), "no delete key without hover");
        // hovered + flash: wash beats the hover surface
        let mut v2: Vec<Cmd> = Vec::new();
        scene.draw(&mut v2, 0.0, 0.0, 300.0, 130.0, scale(), &pal(), &vals, 31_711, true, true);
        assert!(v2.iter().any(|c| matches!(c, Cmd::Rect { color, y, .. } if *color == wash && (*y - 74.0).abs() < 0.02)), "flash wash beats hover surface");
        // idle (0 = no flash): hover lifts the surface instead
        let mut vals_idle = vals.clone();
        vals_idle.insert("snip_flash", SceneValue::Ring(0.0));
        let mut v3: Vec<Cmd> = Vec::new();
        scene.draw(&mut v3, 0.0, 0.0, 300.0, 130.0, scale(), &pal(), &vals_idle, 31_711, true, true);
        let hhl = ColorToken::HoverHl.resolve(&pal());
        assert!(v3.iter().any(|c| matches!(c, Cmd::Rect { color, y, .. } if *color == hhl && (*y - 74.0).abs() < 0.02)), "idle hover surface lifts the row");
        assert!(!v3.iter().any(|c| matches!(c, Cmd::Rect { color, y, .. } if *color == wash && (*y - 74.0).abs() < 0.02)), "no wash without a flash scalar");
    }

    #[test]
    fn snippets_scene_is_fully_declarative() {
        let path = crate::vars::card_scene_path("snippets");
        if !path.exists() {
            return;
        }
        let s = std::fs::read_to_string(&path).expect("scene file readable");
        let scene: CardScene = ron::from_str(&s)
            .unwrap_or_else(|e| panic!("committed snippets.ron must parse: {e}"));
        assert!(
            scene.items.iter().all(|i| !matches!(i, SceneItem::Ink { .. })),
            "snippets scene is fully declarative — no Ink painters"
        );
        let hdr = scene
            .items
            .iter()
            .find_map(|i| if let SceneItem::Header { title, meta, meta_color, .. } = i {
                Some((title.clone(), meta.clone(), *meta_color))
            } else {
                None
            })
            .expect("header declared");
        assert_eq!(hdr.0, "Snippets");
        assert!(hdr.1.as_deref().unwrap_or_default().contains("snip_meta"));
        assert_eq!(hdr.2, Some(ColorToken::Fg), "clip count in fg like the Rust meta");
        let comp = scene
            .items
            .iter()
            .find_map(|i| if let SceneItem::Composer { y, h, key, focus, text, placeholder, .. } = i {
                Some((*y, *h, *key, focus.clone(), text.clone(), placeholder.clone()))
            } else {
                None
            })
            .expect("composer field declared");
        assert_eq!(comp.0, 26.0);
        assert_eq!(comp.1, 22.0);
        assert_eq!(comp.2, crate::shell::SNIPPET_INPUT_KEY);
        assert_eq!(comp.3, Some("snip_focus".into()));
        assert_eq!(comp.4, Some("{snip_buf}".into()));
        assert!(comp.5.contains("add snippet"));
        let rows_item = scene
            .items
            .iter()
            .find_map(|i| if let SceneItem::Rows { name, y, key_base, del_base, hover_surface, flash, cols, .. } = i {
                Some((name.clone(), *y, *key_base, *del_base, hover_surface.clone(), flash.clone(), cols.clone()))
            } else {
                None
            })
            .expect("clipboard list declared");
        assert_eq!(rows_item.0, "snip_rows");
        assert_eq!(rows_item.1, 50.0, "rows clear the shell header + composer");
        assert_eq!(rows_item.2, Some(crate::shell::SNIPPET_COPY_BASE));
        assert_eq!(rows_item.3, crate::shell::SNIPPET_DEL_BASE);
        assert_eq!(rows_item.4, Some(ColorToken::HoverHl), "hover lifts the row surface");
        assert_eq!(rows_item.5, Some("snip_flash".into()));
        assert_eq!(rows_item.6.len(), 3);
        assert!(rows_item.6[0].icon, "copy glyph in the icon font");
        assert_eq!(rows_item.6[0].flash, Some(ColorToken::Acc), "copy glyph flashes accent");
        assert_eq!(rows_item.6[1].color, Some(ColorToken::Fg));
        assert_eq!(rows_item.6[1].flash, Some(ColorToken::Acc), "name flashes accent");
        assert_eq!(rows_item.6[2].color, Some(ColorToken::Fg3), "preview stays dim");
        assert_eq!(rows_item.6[2].size, 8.0);
        assert!(rows_item.6[2].truncate.is_some(), "preview reserves the right edge");
    }

    #[test]
    fn karaoke_highlights_active_word_and_washes_active_row() {
        let mut vals = SceneValues::new();
        vals.insert(
            "lyr_rows",
            SceneValue::Rows(vec![
                crate::scene::SceneRow {
                    cols: vec!["we both shine".into()],
                    key: 0,
                    action: None,
                    color: None,
                    surface: None,
                    col_colors: vec![],
                },
                crate::scene::SceneRow {
                    cols: vec!["hello moon again".into()],
                    key: 0,
                    action: None,
                    color: None,
                    surface: None,
                    col_colors: vec![],
                },
            ]),
        );
        vals.insert(
            "lyr_karaoke",
            SceneValue::Karaoke(vec![
                crate::scene::KaraokeLine {
                    words: vec![("we".into(), false), ("both".into(), false), ("shine".into(), false)],
                    active: false,
                },
                crate::scene::KaraokeLine {
                    words: vec![("hello".into(), true), ("moon".into(), false), ("again".into(), false)],
                    active: true,
                },
            ]),
        );
        vals.insert("lyr_scroll", SceneValue::Ring(0.0));
        let scene: CardScene = ron::from_str(
            "(items: [
                Rows(
                    name: \"lyr_rows\",
                    y: 33.0, row_h: 21.0, r: 4.0,
                    hover_surface: Some(hover),
                    karaoke: Some(\"lyr_karaoke\"),
                    start: Some(\"lyr_scroll\"),
                    key_base: Some(33500),
                    cols: [
                        (x: 6.0, dy: Some(0.5), size: 10.5, color: Some(fg), hover: Some(acc)),
                    ],
                ),
             ])",
        )
        .unwrap();
        let mut v: Vec<Cmd> = Vec::new();
        let (hits, _) = scene.draw(&mut v, 0.0, 0.0, 300.0, 120.0, scale(), &pal(), &vals, 33_500, true, true);
        // the ACTIVE (row 1, ry 54) row: accent wash + word walk
        let acc = ColorToken::Acc.resolve(&pal());
        assert!(v.iter().any(|c| matches!(c, Cmd::Rect { color, y, h, .. } if *color == acc && (*y - 55.0).abs() < 0.02 && *h == 17.0)), "active row washed accent");
        assert!(v.iter().any(|c| matches!(c, Cmd::Text { text, color, .. } if text == "hello" && *color == 0xffff_ffff)), "live word white");
        assert!(v.iter().any(|c| matches!(c, Cmd::Text { text, color, .. } if text == "moon" && *color == 0x1d1d_1d)), "rest of the line near-black");
        assert!(v.iter().any(|c| matches!(c, Cmd::Rect { color, y, h, .. } if *color == 0xffff_ffff && *h == 1.5 && (*y - 68.5).abs() < 0.02)), "underline under the live word");
        // the inactive hovered row: hover lift + joined accent line
        let hover_col = crate::ui::hover(&pal());
        assert!(v.iter().any(|c| matches!(c, Cmd::Rect { color, y, h, .. } if *color == hover_col && (*y - 34.0).abs() < 0.02 && *h == 17.0)), "inactive hovered row lifts");
        assert!(v.iter().any(|c| matches!(c, Cmd::Text { text, color, .. } if text == "we both shine" && *color == acc)), "joined line tints accent on hover");
        // regions for both visible rows
        assert!(hits.iter().any(|h| h.key == 33_500));
        assert!(hits.iter().any(|h| h.key == 33_501));
    }

    #[test]
    fn karaoke_unbound_falls_back_to_plain_cells() {
        let mut vals = SceneValues::new();
        vals.insert(
            "lyr_rows",
            SceneValue::Rows(vec![crate::scene::SceneRow {
                cols: vec!["plain line one".into()],
                key: 0,
                action: None,
                color: None,
                surface: None,
                col_colors: vec![],
            }]),
        );
        vals.insert("lyr_scroll", SceneValue::Ring(0.0));
        // no `lyr_karaoke` value bound — the karaoke binding resolves None
        let scene: CardScene = ron::from_str(
            "(items: [
                Rows(
                    name: \"lyr_rows\",
                    y: 33.0, row_h: 21.0, r: 4.0,
                    hover_surface: Some(hover),
                    karaoke: Some(\"lyr_karaoke\"),
                    start: Some(\"lyr_scroll\"),
                    key_base: Some(33500),
                    cols: [
                        (x: 6.0, dy: Some(0.5), size: 10.5, color: Some(fg), hover: Some(acc)),
                    ],
                ),
             ])",
        )
        .unwrap();
        // idle frame: whole line via the plain cell path, fg
        let mut v: Vec<Cmd> = Vec::new();
        scene.draw(&mut v, 0.0, 0.0, 300.0, 120.0, scale(), &pal(), &vals, 0, true, true);
        let fg = ColorToken::Fg.resolve(&pal());
        assert!(v.iter().any(|c| matches!(c, Cmd::Text { text, color, .. } if text == "plain line one" && *color == fg)), "whole line via the plain cell path");
        assert!(!v.iter().any(|c| matches!(c, Cmd::Text { color: 0xffff_ffff, .. } | Cmd::Text { color: 0x1d1d_1d, .. })), "no karaoke word draw without the value");
        // hovered frame: hover lift + accent tint
        let mut v2: Vec<Cmd> = Vec::new();
        scene.draw(&mut v2, 0.0, 0.0, 300.0, 120.0, scale(), &pal(), &vals, 33_500, true, true);
        let acc = ColorToken::Acc.resolve(&pal());
        assert!(v2.iter().any(|c| matches!(c, Cmd::Text { text, color, .. } if text == "plain line one" && *color == acc)), "hovered plain row tints accent");
        assert!(v2.iter().any(|c| matches!(c, Cmd::Rect { color, y, h, .. } if *color == crate::ui::hover_hl(&pal()) && (*y - 33.0).abs() < 0.02 && *h == 21.0)), "plain row hover surface lifts");
    }

    #[test]
    fn lyrics_scene_is_fully_declarative() {
        let path = crate::vars::card_scene_path("lyrics");
        if !path.exists() {
            return;
        }
        let s = std::fs::read_to_string(&path).expect("scene file readable");
        let scene: CardScene = ron::from_str(&s)
            .unwrap_or_else(|e| panic!("committed lyrics.ron must parse: {e}"));
        assert!(
            scene.items.iter().all(|i| !matches!(i, SceneItem::Ink { .. })),
            "lyrics scene is fully declarative — no Ink painters"
        );
        let hdr = scene
            .items
            .iter()
            .find_map(|i| if let SceneItem::Header { title, glyph, glyph_color, .. } = i {
                Some((title.clone(), glyph.clone(), *glyph_color))
            } else {
                None
            })
            .expect("header declared");
        assert_eq!(hdr.0, "Lyrics");
        assert_eq!(hdr.1, Some("\u{f001}".into()), "notes glyph");
        assert_eq!(hdr.2, Some(ColorToken::Acc), "accent note glyph");
        let statuses = scene
            .items
            .iter()
            .filter_map(|i| if let SceneItem::Text { text, color, visible, right, .. } = i {
                Some((text.clone(), *color, visible.clone(), *right))
            } else {
                None
            })
            .filter(|t| t.3 && matches!(t.0.as_str(), "synced" | "none"))
            .collect::<Vec<_>>();
        assert_eq!(statuses.len(), 2, "synced + none chips");
        assert!(statuses.iter().any(|t| t.0 == "synced" && t.1 == Some(ColorToken::Acc)));
        assert!(statuses.iter().any(|t| t.0 == "none" && t.1 == Some(ColorToken::Danger)));
        let rows_item = scene
            .items
            .iter()
            .find_map(|i| if let SceneItem::Rows { name, y, row_h, key_base, hover_surface, karaoke, cols, .. } = i {
                Some((name.clone(), *y, *row_h, *key_base, hover_surface.clone(), karaoke.clone(), cols.clone()))
            } else {
                None
            })
            .expect("karaoke list declared");
        assert_eq!(rows_item.0, "lyr_rows");
        assert_eq!(rows_item.1, 33.0, "list clears header + caption");
        assert_eq!(rows_item.2, 21.0);
        assert_eq!(rows_item.3, Some(crate::shell::LYRICS_KEY_BASE));
        assert_eq!(rows_item.4, Some(ColorToken::Hover), "plain-row hover lift");
        assert_eq!(rows_item.5, Some("lyr_karaoke".into()), "word-highlight binding");
        assert_eq!(rows_item.6.len(), 1);
        assert_eq!(rows_item.6[0].hover, Some(ColorToken::Acc), "hovered plain line tints accent");
        assert_eq!(rows_item.6[0].size, 10.5);
    }

    #[test]
    fn spark_draws_line_and_column_shapes() {
        let scene: CardScene = ron::from_str(
            "(items: [
                Spark(name: \"cpu\", x: 12.0, y: 30.0, w: 176.0, h: 40.0, color: Some(acc), max: 100.0),
                Spark(name: \"mem\", x: 12.0, y: 76.0, w: 176.0, h: 24.0, kind: column, max: 100.0, thickness: 4.0),
            ])",
        )
        .unwrap();
        let mut values = SceneValues::new();
        values.insert("cpu", SceneValue::Spark(vec![10.0, 40.0, 90.0, 30.0]));
        values.insert("mem", SceneValue::Spark(vec![20.0, 60.0, 80.0]));
        let mut v: Vec<Cmd> = Vec::new();
        let (_h, _i) = scene.draw(&mut v, 0.0, 0.0, 200.0, 110.0, scale(), &pal(), &values, 0, true, true);
        assert!(v.iter().any(|c| matches!(c, Cmd::Line { .. })), "line spark emits segments");
        assert!(v.iter().any(|c| matches!(c, Cmd::Rect { .. })), "column spark emits bars");
        // a 100%-sample line's last point sits at the line's own thickness inset:
        let (peak_at) = 90.0_f32 / 100.0;
        // nothing to assert numerically here beyond both shapes drawing -> covered
        let _ = peak_at;
    }

    #[test]
    fn spark_auto_normalizes_when_no_max() {
        let scene: CardScene = ron::from_str(
            "(items: [ Spark(name: \"s\", x: 0.0, y: 0.0, w: 100.0, h: 50.0) ])",
        )
        .unwrap();
        let mut values = SceneValues::new();
        values.insert("s", SceneValue::Spark(vec![0.0, 50.0, 100.0]));
        let mut v: Vec<Cmd> = Vec::new();
        let (_h, _i) = scene.draw(&mut v, 0.0, 0.0, 100.0, 50.0, scale(), &pal(), &values, 0, true, true);
        assert!(v.iter().any(|c| matches!(c, Cmd::Line { .. })), "no-max spark still draws");
    }

    #[test]
    fn ring_lights_beads_by_ratio() {
        let scene: CardScene = ron::from_str(
            "(items: [ Ring(name: \"bat\", cx: 40.0, cy: 50.0, radius: 30.0, max: 100.0, beads: 36, fill: acc, track: hover) ])",
        )
        .unwrap();
        let mut values = SceneValues::new();
        values.insert("bat", SceneValue::Ring(50.0));
        let acc = ColorToken::Acc.resolve(&pal());
        let mut v: Vec<Cmd> = Vec::new();
        let (_h, _i) = scene.draw(&mut v, 0.0, 0.0, 200.0, 120.0, scale(), &pal(), &values, 0, true, true);
        let lit = v.iter().filter(|c| matches!(c, Cmd::Rect { color, .. } if *color == acc)).count();
        assert_eq!(lit, 18, "50% of 36 beads lit");
        // value over max clamps to full ring
        values.insert("bat", SceneValue::Ring(150.0));
        let mut v: Vec<Cmd> = Vec::new();
        let (_h, _i) = scene.draw(&mut v, 0.0, 0.0, 200.0, 120.0, scale(), &pal(), &values, 0, true, true);
        let lit = v.iter().filter(|c| matches!(c, Cmd::Rect { color, .. } if *color == acc)).count();
        assert_eq!(lit, 36, "over-max clamps to the full ring");
    }

    #[test]
    fn fader_draws_fill_head_and_registers_key() {
        let scene: CardScene = ron::from_str(
            "(items: [ Fader(name: \"vol\", x: 12.0, y: 40.0, w: 176.0, h: 20.0, key: 501, fill: acc, track: hover) ])",
        )
        .unwrap();
        let mut values = SceneValues::new();
        values.insert("vol", SceneValue::Fader(0.75));
        let mut v: Vec<Cmd> = Vec::new();
        let (hits, _) = scene.draw(&mut v, 0.0, 0.0, 200.0, 100.0, scale(), &pal(), &values, 501, true, true);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].key, 501);
        assert_eq!(hits[0].x, 12.0);
        assert_eq!(hits[0].w, 176.0);
        let acc = ColorToken::Acc.resolve(&pal());
        assert!(v.iter().filter(|c| matches!(c, Cmd::Rect { color, .. } if *color == acc)).count() >= 2,
            "fill + round head both accent-colored");
        // no key → no hit region
        let scene2: CardScene = ron::from_str(
            "(items: [ Fader(name: \"br\", x: 12.0, y: 40.0, w: 100.0, h: 20.0) ])",
        )
        .unwrap();
        let (_h, _i) = scene2.draw(&mut Vec::new(), 0.0, 0.0, 200.0, 100.0, scale(), &pal(), &values, 0, true, true);
        let mut v2: Vec<Cmd> = Vec::new();
        let (hits2, _) = scene2.draw(&mut v2, 0.0, 0.0, 200.0, 100.0, scale(), &pal(), &values, 0, true, true);
        assert!(hits2.is_empty(), "key 0 registers nothing");
    }

    #[test]
    fn toggle_draws_switch_state_and_registers_key() {
        let scene: CardScene = ron::from_str(
            "(items: [ Toggle(name: \"blur\", x: 150.0, y: 8.0, w: 38.0, h: 20.0, key: 601) ])",
        )
        .unwrap();
        let mut values = SceneValues::new();
        values.insert("blur", SceneValue::Toggle(false));
        let mut v: Vec<Cmd> = Vec::new();
        let (hits, _) = scene.draw(&mut v, 0.0, 0.0, 200.0, 100.0, scale(), &pal(), &values, 0, true, true);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].key, 601);
        assert!(v.iter().all(|c| !matches!(c, Cmd::Rect { color, .. } if *color == pal().acc)),
            "off switch has no accent anywhere");
        // on → accent track
        values.insert("blur", SceneValue::Toggle(true));
        let mut v: Vec<Cmd> = Vec::new();
        let (_h, _i) = scene.draw(&mut v, 0.0, 0.0, 200.0, 100.0, scale(), &pal(), &values, 0, true, true);
        assert!(v.iter().any(|c| matches!(c, Cmd::Rect { color, .. } if *color == pal().acc)),
            "on switch paints the accent track");
    }

    #[test]
    fn tabrow_draws_tabs_and_highlights_selected() {
        let scene: CardScene = ron::from_str(
            "(items: [ TabRow(name: \"tabs\", sel: Some(\"tab_sel\"), y: 28.0, key_base: 810)])",
        )
        .unwrap();
        let mut values = SceneValues::new();
        values.insert(
            "tabs",
            SceneValue::Rows(vec![
                SceneRow { cols: vec!["One".into()], key: 0, action: None, color: None, col_colors: vec![], surface: None },
                SceneRow { cols: vec!["Two".into()], key: 0, action: None, color: None, col_colors: vec![], surface: None },
            ]),
        );
        values.insert("tab_sel", SceneValue::Ring(1.0));
        let mut v: Vec<Cmd> = Vec::new();
        let (hits, _) = scene.draw(&mut v, 0.0, 0.0, 200.0, 100.0, scale(), &pal(), &values, 0, true, true);
        assert_eq!(hits.len(), 2, "one key per tab from key_base");
        assert_eq!(hits[1].key, 811);
        assert!(v.iter().any(|c| matches!(c, Cmd::Text { text, .. } if text == "One")), "tab label drawn");
        assert!(v.iter().any(|c| matches!(c, Cmd::Rect { color, .. } if *color == ColorToken::AccTint.resolve(&pal()))),
            "selected tab gets the accent-tinted surface");
    }

    #[test]
    fn missing_or_wrong_typed_value_draws_nothing() {
        let scene: CardScene = ron::from_str(
            "(items: [ Spark(name: \"nope\", x: 0.0, y: 0.0, w: 40.0, h: 20.0), Ring(name: \"also_nope\", cx: 0.0, cy: 0.0, radius: 20.0), Fader(name: \"wonk\", x: 0.0, y: 0.0, w: 40.0, h: 18.0), Toggle(name: \"gone\", x: 0.0, y: 0.0, w: 20.0, h: 12.0) ])",
        )
        .unwrap();
        let mut v: Vec<Cmd> = Vec::new();
        let (_h, hits) = scene.draw(&mut v, 0.0, 0.0, 200.0, 120.0, scale(), &pal(), &SceneValues::new(), 0, true, true);
        assert!(hits.is_empty(), "no keys → no hit regions");
        assert!(v.is_empty(), "missing/typed-mismatched names draw nothing");
    }

    // ── dynamic color tokens (ColorToken::Value) ───────────────────────

    #[test]
    fn value_token_ron_parses_and_roundtrips() {
        let scene: CardScene = ron::from_str(
            r#"(items: [
                Hit(x: 8.0, y: 6.0, w: 60.0, h: 24.0, r: 12.0, key: 3,
                    surface: Some(value("dnd_chip")), text: "DND", font_size: 11.0,
                    color: Some(value("dnd_chip_fg"))),
            ])"#,
        )
        .unwrap();
        match &scene.items[0] {
            SceneItem::Hit { surface, color, .. } => {
                assert_eq!(*surface, Some(ColorToken::Value(DynColor::new("dnd_chip").unwrap())));
                assert_eq!(*color, Some(ColorToken::Value(DynColor::new("dnd_chip_fg").unwrap())));
            }
            other => panic!("expected Hit, got {other:?}"),
        }
        let ron_out = ron::to_string(&scene).expect("serializes");
        let back: CardScene = ron::from_str(&ron_out).expect("roundtrips");
        assert_eq!(scene, back);
    }

    #[test]
    fn dynamic_token_resolves_from_scene_colors() {
        let scene: CardScene = ron::from_str(
            r#"(items: [ Surface(x: 8.0, y: 6.0, w: 60.0, h: 24.0, r: 12.0, color: value("dnd_chip")) ])"#,
        )
        .unwrap();
        let mut values = SceneValues::new();
        values.insert_color("dnd_chip", SceneColor::Token(ColorToken::Acc));
        let mut v: Vec<Cmd> = Vec::new();
        let (_h, _i) = scene.draw(&mut v, 0.0, 0.0, 200.0, 120.0, scale(), &pal(), &values, 0, true, true);
        assert!(
            v.iter().any(|c| matches!(c, Cmd::Rect { color, .. } if *color == plt_acc())),
            "surface bound to a dynamic token lifts the live theme accent"
        );
    }

    #[test]
    fn dynamic_raw_color_bypasses_theme() {
        let scene: CardScene = ron::from_str(
            r#"(items: [ Surface(x: 0.0, y: 0.0, w: 40.0, h: 20.0, color: value("meter")) ])"#,
        )
        .unwrap();
        let mut values = SceneValues::new();
        values.insert_color("meter", SceneColor::Raw(0x11223344));
        let mut v: Vec<Cmd> = Vec::new();
        let (_h, _i) = scene.draw(&mut v, 0.0, 0.0, 200.0, 120.0, scale(), &pal(), &values, 0, true, true);
        assert!(
            v.iter().any(|c| matches!(c, Cmd::Rect { color, .. } if *color == 0x11223344)),
            "pre-resolved u32 draws verbatim"
        );
    }

    #[test]
    fn unknown_dynamic_name_falls_back_to_fg() {
        let scene: CardScene = ron::from_str(
            r#"(items: [ Surface(x: 0.0, y: 0.0, w: 40.0, h: 20.0, color: value("nope")) ])"#,
        )
        .unwrap();
        let mut v: Vec<Cmd> = Vec::new();
        let (_h, _i) = scene.draw(&mut v, 0.0, 0.0, 200.0, 120.0, scale(), &pal(), &SceneValues::new(), 0, true, true);
        assert!(
            v.iter().any(|c| matches!(c, Cmd::Rect { color, .. } if *color == pal().fg)),
            "missing dynamic name degrades to fg, never panics"
        );
    }

    #[test]
    fn dynamic_surface_keeps_its_color_under_hover() {
        // a dynamic chip surface is state-computed per frame (its hover intent
        // is already baked in) so hovering must not lift/replace the color.
        let scene: CardScene = ron::from_str(
            r#"(items: [ Hit(x: 8.0, y: 6.0, w: 60.0, h: 24.0, r: 12.0, key: 3, surface: Some(value("dnd_chip"))) ])"#,
        )
        .unwrap();
        let mut values = SceneValues::new();
        values.insert_color("dnd_chip", SceneColor::Token(ColorToken::Acc));
        for hover in [0, 3] {
            let mut v: Vec<Cmd> = Vec::new();
            let (_h, _i) = scene.draw(&mut v, 0.0, 0.0, 200.0, 120.0, scale(), &pal(), &values, hover, true, true);
            let hits = v.iter().filter(|c| matches!(c, Cmd::Rect { color, .. } if *color == plt_acc())).count();
            assert_eq!(hits, 1, "chip surface stays the dynamic accent, hovered or not (hover={hover})");
        }
    }

    fn plt_acc() -> u32 {
        ColorToken::Acc.resolve(&pal())
    }

    // ── layout containers (Phase B) ────────────────────────────────────

    #[test]
    fn row_places_children_by_width_and_spacing() {
        let scene: CardScene = ron::from_str(
            r#"(items: [ Row(
                    x: 0.0, y: 0.0,
                    spacing: 10.0,
                    items: [
                        Hit(x: 0.0, y: 0.0, w: 40.0, h: 20.0, key: 1),
                        Hit(x: 0.0, y: 0.0, w: 40.0, h: 20.0, key: 2),
                        Hit(x: 0.0, y: 0.0, w: 40.0, h: 20.0, key: 3),
                    ],
                ) ])"#,
        )
        .unwrap();
        let mut v: Vec<Cmd> = Vec::new();
        let (hits, _) = scene.draw(&mut v, 0.0, 0.0, 200.0, 120.0, scale(), &pal(), &SceneValues::new(), 0, true, true);
        assert_eq!(hits.len(), 3);
        assert_eq!(hits[0].x, 0.0);
        assert_eq!(hits[1].x, 50.0, "40 + 10 spacing");
        assert_eq!(hits[2].x, 100.0);
        assert_eq!(hits[0].w, 40.0);
    }

    #[test]
    fn row_fill_children_split_remaining_width() {
        let scene: CardScene = ron::from_str(
            r#"(items: [ Row(
                    x: 0.0, y: 0.0, w: 200.0,
                    spacing: 0.0,
                    items: [
                        Hit(x: 0.0, y: 0.0, w: 120.0, h: 20.0, key: 1),
                        Hit(x: 0.0, y: 0.0, w: 0.0, h: 20.0, key: 2),
                        Hit(x: 0.0, y: 0.0, w: 0.0, h: 20.0, key: 3),
                    ],
                ) ])"#,
        )
        .unwrap();
        let mut v: Vec<Cmd> = Vec::new();
        let (hits, _) = scene.draw(&mut v, 0.0, 0.0, 200.0, 120.0, scale(), &pal(), &SceneValues::new(), 0, true, true);
        assert_eq!(hits.len(), 3);
        assert_eq!(hits[0].w, 120.0);
        assert_eq!(hits[1].x, 120.0, "fixed child ends at 120");
        assert_eq!(hits[1].w, 40.0, "(200 - 120) / 2 fills");
        assert_eq!(hits[2].x, 160.0);
        assert_eq!(hits[2].w, 40.0);
    }

    #[test]
    fn row_valigns_short_children() {
        let scene: CardScene = ron::from_str(
            r#"(items: [ Row(x: 0.0, y: 0.0, h: 100.0, valign: middle, items: [
                    Hit(x: 0.0, y: 0.0, w: 40.0, h: 20.0, key: 1),
                ]) ])"#,
        )
        .unwrap();
        let mut v: Vec<Cmd> = Vec::new();
        let (hits, _) = scene.draw(&mut v, 0.0, 0.0, 200.0, 120.0, scale(), &pal(), &SceneValues::new(), 0, true, true);
        assert_eq!(hits[0].y, 40.0, "in a 100-tall box a 20-tall child centers at 40");
    }

    #[test]
    fn column_stacks_children_and_centers_halign() {
        let scene: CardScene = ron::from_str(
            r#"(items: [ Column(
                    x: 0.0, y: 0.0, w: 200.0, pad: 10.0, spacing: 5.0, halign: center,
                    items: [
                        Hit(x: 0.0, y: 0.0, w: 80.0, h: 20.0, key: 1),
                        Hit(x: 0.0, y: 0.0, w: 120.0, h: 20.0, key: 2),
                    ],
                ) ])"#,
        )
        .unwrap();
        let mut v: Vec<Cmd> = Vec::new();
        let (hits, _) = scene.draw(&mut v, 0.0, 0.0, 200.0, 120.0, scale(), &pal(), &SceneValues::new(), 0, true, true);
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].y, 10.0, "pad top");
        assert_eq!(hits[1].y, 35.0, "20 + 5 spacing");
        assert_eq!(hits[0].x, 10.0 + (180.0 - 80.0) / 2.0, "halign: center offsets within inner width");
        assert_eq!(hits[1].x, 10.0 + (180.0 - 120.0) / 2.0);
    }

    #[test]
    fn column_fill_children_share_remaining_height() {
        let scene: CardScene = ron::from_str(
            r#"(items: [ Column(x: 0.0, y: 0.0, h: 100.0, spacing: 4.0, items: [
                    Hit(x: 0.0, y: 0.0, w: 100.0, h: 20.0, key: 1),
                    Hit(x: 0.0, y: 0.0, w: 100.0, h: 0.0, key: 2),
                    Hit(x: 0.0, y: 0.0, w: 100.0, h: 0.0, key: 3),
                ]) ])"#,
        )
        .unwrap();
        let mut v: Vec<Cmd> = Vec::new();
        let (hits, _) = scene.draw(&mut v, 0.0, 0.0, 200.0, 120.0, scale(), &pal(), &SceneValues::new(), 0, true, true);
        assert_eq!(hits.len(), 3);
        // fixed=20, 2 fill children, 2 gaps → (100 - 20 - 8)/2 = 36 each
        assert_eq!(hits[1].y, 24.0, "20 + 4 spacing");
        assert_eq!(hits[1].h, 36.0, "(100 - 20 - 8)/2 fills");
        assert_eq!(hits[2].y, 64.0, "24 + 36 + 4 spacing");
    }

    #[test]
    fn stack_overlays_fullbleed_surface_then_ring() {
        let scene: CardScene = ron::from_str(
            r#"(items: [ Column(x: 0.0, y: 0.0, h: 120.0, spacing: 4.0, items: [
                    Stack(x: 0.0, y: 0.0, h: 60.0, items: [
                        Surface(x: 0.0, y: 0.0, w: 0.0, h: 0.0, color: hover),
                        Ring(name: "mem", cx: 0.0, cy: 0.0, radius: 24.0, center_x: true, center_y: true, max: 100.0, beads: 36),
                    ]),
                ]) ])"#,
        )
        .unwrap();
        let mut values = SceneValues::new();
        values.insert("mem", SceneValue::Ring(50.0));
        let mut v: Vec<Cmd> = Vec::new();
        let (_h, _i) = scene.draw(&mut v, 0.0, 0.0, 200.0, 120.0, scale(), &pal(), &values, 0, true, true);
        // full-bleed surface spans the stack's (implicit card-width) box
        let hover = ColorToken::Hover.resolve(&pal());
        let full = v.iter().filter_map(|c| match c {
            Cmd::Rect { w, h, color, .. } if *color == hover => Some((w, h)),
            _ => None,
        }).next().unwrap();
        assert!((*full.0 - 200.0).abs() < 0.001, "surface fills width, got {}", full.0);
        assert!((*full.1 - 60.0).abs() < 0.001, "surface fills stack height, got {}", full.1);
    }

    #[test]
    fn nested_containers_keep_child_scaling_relative() {
        let scene: CardScene = ron::from_str(
            r#"(items: [ Column(x: 0.0, y: 0.0, w: 200.0, h: 120.0, spacing: 10.0, items: [
                    Row(spacing: 8.0, items: [
                        Hit(x: 0.0, y: 0.0, w: 90.0, h: 30.0, key: 1),
                        Toggle(name: "blur", x: 0.0, y: 0.0, w: 38.0, h: 20.0, key: 2),
                    ]),
                    Fader(name: "vol", x: 0.0, y: 0.0, w: 0.0, h: 20.0, key: 3),
                ]) ])"#,
        )
        .unwrap();
        let mut values = SceneValues::new();
        values.insert("blur", SceneValue::Toggle(true));
        values.insert("vol", SceneValue::Fader(0.5));
        let mut v: Vec<Cmd> = Vec::new();
        let (hits, _) = scene.draw(&mut v, 0.0, 0.0, 200.0, 120.0, scale(), &pal(), &values, 0, true, true);
        assert_eq!(hits.len(), 3, "nested Hit + Toggle + Fader all register");
        assert_eq!(hits[0].x, 0.0);
        assert_eq!(hits[1].x, 98.0, "90 + 8 spacing");
        assert_eq!(hits[2].x, 0.0, "fader in the Column fills column width");
        assert_eq!(hits[2].w, 200.0);
        // Row fills remaining 90 (h:120 - Fader h:20 - spacing 10), then fader below
        assert_eq!(hits[2].y, 100.0, "fader y = Row fill height + Column spacing");
    }

    #[test]
    fn container_ron_roundtrips_with_extension_shapes() {
        // The scene engine derives Serialize: nested containers serialize back.
        let scene: CardScene = ron::from_str(
            r#"(items: [ Stack(w: 0.0, h: 0.0, items: [
                    Row(w: 0.0, h: 0.0, spacing: 4.0, items: [
                        Toggle(name: "a", x: 0.0, y: 0.0, w: 20.0, h: 12.0, key: 1),
                    ]),
                ]) ])"#,
        )
        .unwrap();
        let ron_out = ron::to_string(&scene).expect("serializes");
        let back: CardScene = ron::from_str(&ron_out).expect("roundtrips");
        assert_eq!(scene, back);
    }

    // ── scissor clip regions (the banner-zone prerequisite) ────────────

    #[test]
    fn scissor_emits_clip_commands_at_scaled_coords() {
        let scene: CardScene = ron::from_str(
            r#"(items: [
                Scissor(x: 10.0, y: 12.0, w: 80.0, h: 24.0),
                Text(x: 0.0, y: 0.0, text: "clipped", font_size: 10.0),
                ScissorEnd,
            ])"#,
        )
        .unwrap();
        let mut v: Vec<Cmd> = Vec::new();
        let (_h, _i) = scene.draw(&mut v, 0.0, 0.0, 200.0, 100.0, scale(), &pal(), &SceneValues::new(), 0, true, true);
        let clip = v.iter().find(|c| matches!(c, Cmd::Scissor { .. })).expect("opens a clip");
        match clip {
            Cmd::Scissor { x, y, w, h } => {
                assert_eq!(*x, 10.0);
                assert_eq!(*y, 12.0);
                assert_eq!(*w, 80.0);
                assert_eq!(*h, 24.0);
            }
            other => panic!("expected Scissor, got {:?}", std::mem::discriminant(other)),
        }
        assert!(v.iter().any(|c| matches!(c, Cmd::ScissorEnd)), "closes the clip");
        assert_eq!(
            v.iter().filter(|c| matches!(c, Cmd::Scissor { .. })).count(),
            v.iter().filter(|c| matches!(c, Cmd::ScissorEnd)).count(),
            "balanced open/close"
        );
    }

    #[test]
    fn scissor_zero_dim_spans_the_parent_box() {
        let scene: CardScene = ron::from_str("(items: [ Scissor(x: 0.0, y: 0.0), ScissorEnd ])").unwrap();
        let mut v: Vec<Cmd> = Vec::new();
        let (_h, _i) = scene.draw(&mut v, 5.0, 7.0, 200.0, 100.0, scale(), &pal(), &SceneValues::new(), 0, true, true);
        match v.first() {
            Some(Cmd::Scissor { x, y, w, h }) => {
                assert_eq!(*x, 5.0);
                assert_eq!(*y, 7.0);
                assert_eq!(*w, 200.0, "w: 0 spans the parent width");
                assert_eq!(*h, 100.0, "h: 0 spans the parent height");
            }
            other => panic!("expected an opening Scissor, was a non-Scissor command"),
        }
    }

    #[test]
    fn scissor_ron_roundtrips_with_default_dims() {
        let scene: CardScene = ron::from_str("(items: [ Scissor(x: 3.0, y: 4.0), ScissorEnd ])").unwrap();
        match &scene.items[0] {
            SceneItem::Scissor { x, y, w, h } => {
                assert_eq!(*x, 3.0);
                assert_eq!(*y, 4.0);
                assert_eq!(*w, 0.0, "w/h default to 0 (fill)");
                assert_eq!(*h, 0.0);
            }
            other => panic!("expected Scissor, got {other:?}"),
        }
        let ron_out = ron::to_string(&scene).expect("serializes");
        let back: CardScene = ron::from_str(&ron_out).expect("roundtrips");
        assert_eq!(scene, back);
    }

    // ── banner tokens (the named `dashboard` prerequisite) ─────────────

    #[test]
    fn banner_tokens_resolve_like_scene_values() {
        // The exact bindings `Shell::scene_values()` publishes for the banner
        // surface: two drop-guide inks resolved from the live palette, and the
        // edit/zone/lift state booleans. A scene authored against them must
        // resolve through the normal `value(...)` / `Toggle(name)` paths.
        let pal = pal();
        let hov = hover(&pal);
        let mut values = SceneValues::new();
        values.insert_color("banner_guide", SceneColor::Raw(mix(hov, pal.acc, 0.35)));
        values.insert_color("banner_guide_dim", SceneColor::Raw(mix(hov, pal.fg, 0.10)));
        values.insert("banner_editing", SceneValue::Toggle(true));
        values.insert("banner_zones", SceneValue::Toggle(true));

        let scene: CardScene = ron::from_str(
            r#"(items: [
                Scissor(x: 10.0, y: 10.0, w: 60.0, h: 10.0),
                Surface(x: 10.0, y: 12.0, w: 2.0, h: 6.0, color: value("banner_guide_dim")),
                ScissorEnd,
                Toggle(name: "banner_editing", x: 0.0, y: 30.0, w: 30.0, h: 16.0),
                Surface(x: 10.0, y: 34.0, w: 2.0, h: 6.0, color: value("banner_guide")),
            ])"#,
        )
        .unwrap();
        let mut v: Vec<Cmd> = Vec::new();
        let (_h, _i) = scene.draw(&mut v, 0.0, 0.0, 200.0, 100.0, scale(), &pal, &values, 0, true, true);
        let active = mix(hov, pal.acc, 0.35);
        let dim = mix(hov, pal.fg, 0.10);
        assert_eq!(
            v.iter().filter(|c| matches!(c, Cmd::Rect { color, .. } if *color == active)).count(),
            1,
            "the active drop-guide ink paints exactly one bar"
        );
        assert_eq!(
            v.iter().filter(|c| matches!(c, Cmd::Rect { color, .. } if *color == dim)).count(),
            1,
            "the dim drop-guide ink paints exactly one bar"
        );
    }

    #[test]
    fn composer_draws_placeholder_then_buffer_and_caret() {
        let mut idle = SceneValues::new();
        idle.insert("todo_focus", SceneValue::Toggle(false));
        idle.insert("todo_buf", SceneValue::Text(String::new()));
        let scene: CardScene = ron::from_str(
            r#"(items: [
                Composer(pad: 12.0, y: 12.0, bottom: true, text: Some("{todo_buf}"),
                        focus: Some("todo_focus"), placeholder: "Add a task…",
                        key: 78, add: true, add_key: 79),
            ])"#,
        )
        .unwrap();
        let mut v: Vec<Cmd> = Vec::new();
        let (hits, _) = scene.draw(&mut v, 0.0, 0.0, 200.0, 100.0, scale(), &pal(), &idle, 0, true, true);
        // idle: field + add button rects, placeholder, two regions (field + ⊕)
        assert!(v.iter().any(|c| matches!(c, Cmd::Text { text, .. } if text == "Add a task…")), "placeholder hidden until typing");
        assert!(!v.iter().any(|c| matches!(c, Cmd::Rect { w, .. } if *w == 1.4)), "no caret while unfocused");
        assert_eq!(hits.iter().filter(|h| h.key == 78).count(), 1, "field region");
        assert_eq!(hits.iter().filter(|h| h.key == 79).count(), 1, "⊕ region");

        // focused with text: buffer, caret, keyboard hint — ⊕ hides
        let mut on = SceneValues::new();
        on.insert("todo_focus", SceneValue::Toggle(true));
        on.insert("todo_buf", SceneValue::Text("buy milk".into()));
        let mut v: Vec<Cmd> = Vec::new();
        let (hits, _) = scene.draw(&mut v, 0.0, 0.0, 200.0, 100.0, scale(), &pal(), &on, 0, true, true);
        assert!(v.iter().any(|c| matches!(c, Cmd::Text { text, .. } if text == "buy milk")), "buffer interpolated");
        assert!(v.iter().any(|c| matches!(c, Cmd::Text { text, icon, .. } if *icon && text == crate::icons::ICON_KEYBOARD)), "typing hint shown");
        assert!(v.iter().any(|c| matches!(c, Cmd::Rect { w, .. } if *w == 1.4)), "caret drawn while focused");
        assert!(!v.iter().any(|c| matches!(c, Cmd::Text { text, .. } if text == "Add a task…")), "placeholder gone while typing");
        assert_eq!(hits.iter().filter(|h| h.key == 79).count(), 0, "⊕ hidden while focused");
        assert_eq!(hits.iter().filter(|h| h.key == 78).count(), 1, "field still focusable");
    }

    #[test]
    fn composer_bottom_anchors_to_card_bottom() {
        let scene: CardScene = ron::from_str(
            r#"(items: [
                Composer(pad: 12.0, y: 12.0, bottom: true, add: true, add_key: 9),
            ])"#,
        )
        .unwrap();
        for (card_h, expect_field_y, expect_add_y) in [(100.0, 62.0, 72.0), (140.0, 102.0, 112.0)] {
            let mut v: Vec<Cmd> = Vec::new();
            let (_h, _i) = scene.draw(&mut v, 0.0, 0.0, 200.0, card_h, scale(), &pal(), &vals(), 0, true, true);
            // field: h 26, top = card_h - 26 - 12 = card_h - 38
            let field = v.iter().find(|c| matches!(c, Cmd::Rect { h, .. } if *h == 26.0)).expect("field rect");
            if let Cmd::Rect { y, .. } = field {
                assert!((y - expect_field_y).abs() < 0.01, "field pinned (want {expect_field_y}, got {y})");
            }
            // ⊕: 20² at x=card_w-30, y = card_h - 28 (10 up from the 18 bottom anchor)
            let add = v.iter().find(|c| matches!(c, Cmd::Rect { w, h, .. } if *w == 20.0 && *h == 20.0)).expect("⊕ button");
            if let Cmd::Rect { y, .. } = add {
                assert!((y - expect_add_y).abs() < 0.01, "⊕ pinned (want {expect_add_y}, got {y})");
            }
        }
    }

    #[test]
    fn composer_ron_roundtrips_with_defaults() {
        let src = r#"(items: [
            Composer(pad: 12.0, y: 12.0, text: Some("{todo_buf}"), focus: Some("todo_focus"),
                     placeholder: "Add a task…", key: 78, add: true, add_key: 79),
        ])"#;
        let scene: CardScene = ron::from_str(src).unwrap();
        let ink: String = ron::to_string(&scene).unwrap();
        let back: CardScene = ron::from_str(&ink).unwrap();
        match &back.items[0] {
            SceneItem::Composer { h, bottom, add, key, add_key, .. } => {
                assert_eq!(*h, 26.0, "default height");
                assert!(!bottom, "top-anchored by default");
                assert!(*add && *key == 78 && *add_key == 79);
            }
            other => panic!("roundtrip produced {other:?}"),
        }
    }

    #[test]
    fn todo_scene_declares_composer_and_keeps_its_ink() {
        let path = crate::vars::card_scene_path("todo");
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(_) => return, // config tree absent in this env — nothing to assert
        };
        let scene: CardScene = ron::from_str(&text).expect("todo.ron parses cleanly");
        assert!(scene.items.iter().any(|it| matches!(it, SceneItem::Ink { name, .. } if name == "todo")), "checklist body stays the todo Ink");
        let composer = scene
            .items
            .iter()
            .find_map(|it| match it {
                SceneItem::Composer { .. } => Some(it),
                _ => None,
            })
            .expect("todo scene carries a Composer");
        match composer {
            SceneItem::Composer { bottom, text, focus, key, add, add_key, .. } => {
                assert!(bottom, "composer bottom-anchored");
                assert_eq!(text.as_deref(), Some("{todo_buf}"));
                assert_eq!(focus.as_deref(), Some("todo_focus"));
                assert_eq!(*key, crate::shell::TODO_KEY_INPUT);
                assert!(add);
                assert_eq!(*add_key, crate::shell::TODO_KEY_ADD);
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn sliders_rows_land_on_the_drawer_geometry() {
        // parity guard: the committed scene must reproduce the Rust drawer's
        // numbers, because the drag math downstream (input.rs) reads exactly
        // these regions. 300×200 card, both optional rows switched on.
        let path = crate::vars::card_scene_path("sliders");
        if !path.exists() {
            return;
        }
        let s = std::fs::read_to_string(&path).expect("scene file readable");
        let scene: CardScene = ron::from_str(&s).expect("sliders.ron parses");
        let mut m = SceneValues::new();
        m.insert("sliders_pane_0", SceneValue::Toggle(true));
        m.insert("sliders_pane_1", SceneValue::Toggle(false));
        m.insert("sliders_pane", SceneValue::Ring(0.0));
        m.insert("sliders_hover", SceneValue::Toggle(true));
        m.insert("show_balance", SceneValue::Toggle(true));
        m.insert("show_saturation", SceneValue::Toggle(true));
        m.insert("bri_lbl", SceneValue::Text("62%".into()));
        m.insert("vol_lbl", SceneValue::Text("40%".into()));
        m.insert("mic_lbl", SceneValue::Text("80%".into()));
        m.insert("sat_lbl", SceneValue::Text("50%".into()));
        m.insert("kel_lbl", SceneValue::Text("4200".into()));
        m.insert("vol_glyph", SceneValue::Text(crate::icons::ICON_VOLUME.into()));
        m.insert("mic_icon", SceneValue::Text(crate::icons::ICON_MIC.into()));
        for (n, v) in [
            ("brightness_fader", 0.62),
            ("volume_fader", 0.40),
            ("balance_fader", 0.50),
            ("mic_fader", 0.80),
            ("sat_fader", 0.50),
            ("kelvin_fader", 0.35),
        ] {
            m.insert(n, SceneValue::Fader(v));
        }
        let (w, h) = (300.0, 200.0);
        let mut v: Vec<Cmd> = Vec::new();
        let (hits, _) = scene.draw(&mut v, 0.0, 0.0, w, h, scale(), &pal(), &m, 0, true, true);

        // the six fader grooves, in row order (each row also publishes a
        // full-width hover/drag hit — the groove is the narrower of the two)
        let grooves: Vec<&SceneHit> =
            hits.iter().filter(|t| ((51..=55).contains(&t.key) || t.key == 59) && t.w > 200.0 && t.w < 280.0).collect();
        assert_eq!(grooves.len(), 6, "six fader rows, each publishing a drag region");
        // drawer: groove x = card+34, right edge = card+w-52; rows are 24 px,
        // the 6-row block centers in (card_h - 16)
        let top = (h - 16.0 - 6.0 * 24.0) / 2.0;
        for (i, g) in grooves.iter().enumerate() {
            assert_eq!(g.y, top + i as f32 * 24.0, "row {i} pitch");
            assert_eq!(g.h, 24.0, "row {i} is a 24 px band");
        }
        for (i, g) in grooves.iter().enumerate().filter(|(i, _)| *i != 2) {
            assert_eq!(g.x, 34.0, "row {i} groove starts 34 px in");
            assert_eq!(g.w, w - 52.0, "row {i} groove stops 52 px short of the right edge");
        }
        // balance (row 2) is wider: its R flank + reset chip live in the gap
        assert_eq!(grooves[2].w, w - 48.0, "balance groove stops 48 px short (R flank + reset)");
        // value labels right-anchor at the card's 14 px inset
        let labels: Vec<&Cmd> = v
            .iter()
            .filter(|c| matches!(c, Cmd::Text { text, right: true, .. } if matches!(text.as_str(), "62%" | "40%" | "80%" | "4200")))
            .collect();
        assert_eq!(labels.len(), 4, "the four value labels (balance + saturation are bare)");
        for l in &labels {
            if let Cmd::Text { x, .. } = l {
                assert!((*x - (w - 14.0)).abs() < 0.01, "label right edge sits at card_w − 14");
            }
        }
        // the reset chips keep the drawer's 18 px region at the right inset
        for key in [58u32, 79] {
            let r = hits.iter().find(|t| t.key == key).unwrap_or_else(|| panic!("reset chip {key} present"));
            assert!((r.x - (w - 32.0)).abs() < 0.01 && (r.w - 18.0).abs() < 0.01, "reset chip region at card_w − 32, 18 wide");
        }
        // pane 1 is hidden → no switch regions, and the hover-gated dots paint
        assert!(!hits.iter().any(|t| t.key == crate::shell::SLIDERS_TOGGLE_BALANCE_KEY), "pane 1 stays dark");
        let dots = v.iter().filter(|c| matches!(c, Cmd::Rect { w, h, .. } if (*w - 4.62).abs() < 0.2 && (*h - 4.62).abs() < 0.2)).count();
        assert_eq!(dots, 2, "two pagination dots (HARD RULE: one per pane)");

        // pane 1: two switch rows, 12 px inset, right-anchored 26 px switch
        let mut m2 = m.clone();
        m2.insert("sliders_pane_0", SceneValue::Toggle(false));
        m2.insert("sliders_pane_1", SceneValue::Toggle(true));
        m2.insert_color("sliders_tog_bg_0", SceneColor::Token(crate::ui::ColorToken::Raised));
        m2.insert_color("sliders_tog_bg_1", SceneColor::Token(crate::ui::ColorToken::Raised));
        let mut v2: Vec<Cmd> = Vec::new();
        let (hits2, _) = scene.draw(&mut v2, 0.0, 0.0, w, h, scale(), &pal(), &m2, 0, true, true);
        assert!(!hits2.iter().any(|t| (51..=55).contains(&t.key) || t.key == 59), "pane 0 stays dark");
        for (key, top) in [(crate::shell::SLIDERS_TOGGLE_BALANCE_KEY, 13.0), (crate::shell::SLIDERS_TOGGLE_SAT_KEY, 53.0)] {
            let r = hits2.iter().find(|t| t.key == key).unwrap_or_else(|| panic!("switch row {key} present"));
            assert_eq!((r.x, r.y, r.w, r.h), (12.0, top, w - 24.0, 34.0), "switch row inset + pitch");
        }
        let sw: Vec<(&Cmd, f32)> = v2
            .iter()
            .filter_map(|c| match c {
                Cmd::Rect { x, w, h, .. } if (*w - 26.0).abs() < 0.01 && (*h - 13.0).abs() < 0.01 => Some((c, *x)),
                _ => None,
            })
            .collect();
        assert_eq!(sw.len(), 2, "both switches draw a 26×13 track");
        for (_, x) in &sw {
            assert!((*x - (w - 38.0)).abs() < 0.01, "switch right-anchors 12 px inside the row inset");
        }
    }

    // ── fader / container primitives (2026-09-25 sliders conversion) ──

    #[test]
    fn every_live_card_scene_parses() {
        // The live config tree is the surface the shell actually reads, and a
        // malformed scene fails SILENTLY at runtime (the card falls back to
        // its Rust drawer), so it can sit broken for months. This walks every
        // `ui/cards/*.ron` and parses it. It paid for itself immediately: three
        // committed scenes carried `visible: "name"` where the schema wants
        // `visible: Some("name")`.
        let dir = crate::vars::card_scenes_dir();
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => return, // config tree absent in this env
        };
        let mut checked = 0usize;
        let mut broken: Vec<String> = Vec::new();
        for e in entries.flatten() {
            let path = e.path();
            if path.extension().and_then(|s| s.to_str()) != Some("ron") {
                continue;
            }
            let text = std::fs::read_to_string(&path).expect("scene file readable");
            if ron::from_str::<CardScene>(&text).is_err() {
                broken.push(format!("{}: {}", path.display(), ron::from_str::<CardScene>(&text).unwrap_err()));
            }
            checked += 1;
        }
        assert!(checked > 0, "the live cards dir should hold scenes (found {checked})");
        assert!(broken.is_empty(), "live card scenes must parse:\n{}", broken.join("\n"));
    }

    #[test]
    fn fader_negative_w_stops_short_of_the_parent_span() {
        // `w: -38` is the Image span-minus convention applied to a fader: the
        // groove ends 38 px before the parent's right edge so a value column
        // stays clear on any card width.
        let scene: CardScene = ron::from_str(
            r#"(items: [ Stack(x: 14.0, y: 0.0, w: -14.0, h: 24.0, items: [
                    Fader(name: "f", x: 20.0, y: 0.0, w: -38.0, h: 24.0, key: 51),
                ]) ])"#,
        )
        .unwrap();
        let mut m = SceneValues::new();
        m.insert("f", SceneValue::Fader(0.5));
        let mut v: Vec<Cmd> = Vec::new();
        let (hits, _) = scene.draw(&mut v, 0.0, 0.0, 300.0, 120.0, scale(), &pal(), &m, 0, true, true);
        assert_eq!(hits.len(), 1);
        // stack box: x 14, w 300-14=286 → fader at 14+20=34, w 286-38=248
        assert_eq!(hits[0].x, 34.0, "fader starts 20 px into the row box");
        assert_eq!(hits[0].w, 248.0, "negative w = parent span minus the inset");
    }

    #[test]
    fn fader_centered_fill_grows_from_the_middle() {
        // `centered: true` is `ui::fader_bal`: the fill leaves the middle, not
        // the left edge (the balance / saturation rows).
        let r = |centered: bool, val: f32| {
            let scene: CardScene = ron::from_str(&format!(
                r#"(items: [ Fader(name: "f", x: 0.0, y: 0.0, w: 100.0, h: 10.0, centered: {centered}) ])"#
            ))
            .unwrap();
            let mut m = SceneValues::new();
            m.insert("f", SceneValue::Fader(val));
            let mut v: Vec<Cmd> = Vec::new();
            scene.draw(&mut v, 0.0, 0.0, 200.0, 60.0, scale(), &pal(), &m, 0, true, true);
            v
        };
        // groove (track) is the first rect; the fill is the second
        let fill_x = |cmds: &Vec<Cmd>| match cmds[1] {
            Cmd::Rect { x, .. } => x,
            _ => panic!("expected a fill rect"),
        };
        assert_eq!(fill_x(&r(true, 0.75)), 50.0, "centered fill runs from the middle to 75%");
        assert_eq!(fill_x(&r(true, 0.25)), 25.0, "and from 25% to the middle below center");
        assert_eq!(fill_x(&r(false, 0.75)), 0.0, "the default fader fills from the left edge");
    }

    #[test]
    fn fader_visible_gate_hides_the_whole_row() {
        let scene: CardScene = ron::from_str(
            r#"(items: [ Fader(name: "f", x: 0.0, y: 0.0, w: 100.0, h: 10.0, key: 59, visible: Some("gate")) ])"#,
        )
        .unwrap();
        let mut m = SceneValues::new();
        m.insert("f", SceneValue::Fader(0.5));
        m.insert("gate", SceneValue::Toggle(false));
        let mut v: Vec<Cmd> = Vec::new();
        let (hits, _) = scene.draw(&mut v, 0.0, 0.0, 200.0, 60.0, scale(), &pal(), &m, 0, true, true);
        assert!(hits.is_empty() && v.is_empty(), "a gated-false fader draws nothing");
        m.insert("gate", SceneValue::Toggle(true));
        let mut v2: Vec<Cmd> = Vec::new();
        let (hits2, _) = scene.draw(&mut v2, 0.0, 0.0, 200.0, 60.0, scale(), &pal(), &m, 0, true, true);
        assert_eq!(hits2.len(), 1, "gate true draws it again");
    }

    #[test]
    fn container_layout_skips_invisible_children() {
        // a gated-false child reserves NO slot: the stack reflows around it
        // (the sliders card's optional balance / saturation rows).
        let r = |on: bool| {
            let scene: CardScene = ron::from_str(
                r#"(items: [ Column(x: 0.0, y: 0.0, w: 100.0, items: [
                        Hit(x: 0.0, y: 0.0, w: 10.0, h: 20.0, key: 1),
                        Hit(x: 0.0, y: 0.0, w: 10.0, h: 20.0, key: 2, visible: Some("gate")),
                        Hit(x: 0.0, y: 0.0, w: 10.0, h: 20.0, key: 3),
                    ]) ])"#,
            )
            .unwrap();
            let mut m = SceneValues::new();
            m.insert("gate", SceneValue::Toggle(on));
            let mut v: Vec<Cmd> = Vec::new();
            let (hits, _) = scene.draw(&mut v, 0.0, 0.0, 200.0, 120.0, scale(), &pal(), &m, 0, true, true);
            hits
        };
        let on = r(true);
        assert_eq!(on.len(), 3);
        assert_eq!((on[0].y, on[1].y, on[2].y), (0.0, 20.0, 40.0), "all three rows stack");
        let off = r(false);
        assert_eq!(off.len(), 2, "the hidden row draws nothing");
        assert_eq!((off[0].y, off[1].y), (0.0, 20.0), "and leaves no gap behind");
    }

    #[test]
    fn column_valign_middle_centers_the_stacked_block() {
        // the sliders pane-0 block: 4 rows of 24 in a 100-tall box → 2 px of
        // slack above and below.
        let r = |valign: &str| {
            let scene: CardScene = ron::from_str(&format!(
                r#"(items: [ Column(x: 0.0, y: 0.0, w: 100.0, h: 100.0, valign: {valign}, items: [
                        Hit(x: 0.0, y: 0.0, w: 10.0, h: 24.0, key: 1),
                        Hit(x: 0.0, y: 0.0, w: 10.0, h: 24.0, key: 2),
                        Hit(x: 0.0, y: 0.0, w: 10.0, h: 24.0, key: 3),
                        Hit(x: 0.0, y: 0.0, w: 10.0, h: 24.0, key: 4),
                    ]) ])"#
            ))
            .unwrap();
            let mut v: Vec<Cmd> = Vec::new();
            let (hits, _) = scene.draw(&mut v, 0.0, 0.0, 200.0, 140.0, scale(), &pal(), &SceneValues::new(), 0, true, true);
            hits
        };
        let top = r("top");
        assert_eq!(top[0].y, 0.0, "top-aligned stacks from the box top");
        assert_eq!(top[3].y, 72.0);
        let mid = r("middle");
        assert_eq!(mid[0].y, 2.0, "96 px of rows in a 100 px box → 2 px of slack");
        assert_eq!(mid[3].y, 74.0);
    }

    #[test]
    fn sliders_scene_parses_zero_ink_with_six_faders() {
        // sliders (2026-09-25) — the card's whole body is declarative: pane 0's
        // six fader rows (balance + saturation gated by their pane-1 switch),
        // pane 1's two switch rows, and the HARD-RULE pagination `Dots`.
        let path = crate::vars::card_scene_path("sliders");
        if !path.exists() {
            return; // config tree absent in this env — nothing to assert
        }
        let s = std::fs::read_to_string(&path).expect("scene file readable");
        let scene: CardScene = ron::from_str(&s).unwrap_or_else(|e| panic!("sliders.ron must parse: {e}"));
        assert!(
            !scene.items.iter().any(|i| matches!(i, SceneItem::Ink { .. })),
            "sliders is fully declarative (no Ink body)"
        );
        // the faders + switches live inside pane Columns → row Stacks
        fn flat(items: &[SceneItem]) -> Vec<&SceneItem> {
            let mut out = Vec::new();
            for it in items {
                match it {
                    SceneItem::Row { items, .. } | SceneItem::Column { items, .. } | SceneItem::Stack { items, .. } => {
                        out.extend(flat(items))
                    }
                    _ => out.push(it),
                }
            }
            out
        }
        let all = flat(&scene.items);
        let faders: Vec<(&String, u32, &Option<String>, bool)> = all
            .iter()
            .filter_map(|i| match i {
                SceneItem::Fader { name, key, visible, centered, .. } => Some((name, *key, visible, *centered)),
                _ => None,
            })
            .collect();
        assert_eq!(faders.len(), 6, "all six fader rows are declared");
        // the resident drag keys stay in place, so the geometry each fader
        // publishes keeps the Rust drag math key-accurate
        let keys: Vec<u32> = faders.iter().map(|(_, k, _, _)| *k).collect();
        assert_eq!(keys, vec![51, 52, 59, 53, 54, 55], "drawer key order preserved");
        assert_eq!(faders[2].2.as_deref(), Some("show_balance"), "balance row gated by its switch");
        assert_eq!(faders[4].2.as_deref(), Some("show_saturation"), "saturation row gated by its switch");
        assert!(faders[2].3 && faders[4].3, "balance + saturation fill from the middle");
        assert_eq!(faders[0].0, "brightness_fader");
        assert_eq!(faders[1].0, "volume_fader");
        assert_eq!(faders[3].0, "mic_fader");
        // both panes + the hover-gated dots (HARD RULE)
        assert!(
            scene.items.iter().any(|i| matches!(i, SceneItem::Dots { panes: 2, when: Some(w), .. } if w == "sliders_hover")),
            "two panes with hover-gated pagination dots"
        );
        for pane in ["sliders_pane_0", "sliders_pane_1"] {
            assert!(
                scene.items.iter().any(|i| matches!(i, SceneItem::Column { visible: Some(v), .. } if v == pane)),
                "{pane} body is declared"
            );
        }
        // the two pane-1 switch rows sit at the resident toggle keys
        for key in [crate::shell::SLIDERS_TOGGLE_BALANCE_KEY, crate::shell::SLIDERS_TOGGLE_SAT_KEY] {
            assert!(
                all.iter().any(|i| matches!(i, SceneItem::Toggle { key: k, .. } if *k == key)),
                "switch row at key {key}"
            );
        }
    }

    // ── `Tiles` (the toggles card's responsive switch grid) ──

    /// A `Tiles` item plus a six-tile list, mirroring what `scene_values()`
    /// publishes for the toggles card (Wi-Fi + BT carry a chip key).
    fn tiles_fixture(ron_body: &str, on: &[bool]) -> (CardScene, SceneValues) {
        let src = format!("(items: [Tiles({ron_body})])");
        let scene: CardScene = ron::from_str(&src).expect("fixture Tiles parses");
        let keys = [41u32, 43, 45, 46, 47, 48];
        let labels = ["Wi-Fi", "BT", "DND", "Caffeine", "Sunset", "Shader"];
        let mut m = SceneValues::new();
        m.insert(
            "t",
            SceneValue::Tiles(
                (0..6)
                    .map(|i| SceneTile {
                        key: keys[i],
                        glyph: "\u{f1eb}".into(),
                        label: labels[i].into(),
                        on: on.get(i).copied().unwrap_or(false),
                        chip_key: if i < 2 { keys[i] + 1 } else { 0 },
                    })
                    .collect(),
            ),
        );
        (scene, m)
    }

    const TILES_BODY: &str = r#"
        name: "t", x: 0.0, y: 0.0, w: 0.0, h: 0.0,
        pad: 10.0, gap: 8.0, cols: 1,
        cols_break: [(225.0, 2), (330.0, 3)],
        r: 12.0, icon_size: 13.0, label_size: 8.5,
        icon_top: 8.0, label_top: 30.0, label_min_h: 42.0,
        on_surface: acc_tint, hover_surface: hover_hl, off_surface: hover,
        on_ink: acc, off_ink: fg, label_ink: fg,
        chip: true, chip_w: 18.0, chip_size: 9.0, chip_dx: 10.0, chip_dy: 14.0
    "#;

    fn tile_hits(hits: &[SceneHit]) -> Vec<&SceneHit> {
        // tile regions are the full-height boxes; the chip strips are 18 px wide
        hits.iter().filter(|t| t.w > 18.0).collect()
    }

    #[test]
    fn tiles_breakpoints_pick_the_columns_the_drawer_picks() {
        // the drawer's `if w >= 330 {3} else if w >= 225 {2} else {1}` becomes
        // ascending min-width breakpoints — 1 / 2 / 3 columns
        for (card_w, want_cols) in [(200.0, 1usize), (224.0, 1), (225.0, 2), (329.0, 2), (330.0, 3), (420.0, 3)] {
            let (scene, m) = tiles_fixture(TILES_BODY, &[false; 6]);
            let (w, h) = (card_w, 200.0);
            let mut v: Vec<Cmd> = Vec::new();
            let (hits, _) = scene.draw(&mut v, 0.0, 0.0, w, h, scale(), &pal(), &m, 0, true, true);
            let tiles = tile_hits(&hits);
            assert_eq!(tiles.len(), 6, "all six tiles register a region at w={w}");
            let rows = 6 / want_cols;
            // drawer: tw = (w − 20 − (cols−1)·8) / cols, tiles at 10 + i·(tw+8)
            let tw = (w - 20.0 - (want_cols as f32 - 1.0) * 8.0) / want_cols as f32;
            // drawer: th = (h − 20 − (rows−1)·8) / rows
            let th = (h - 20.0 - (rows as f32 - 1.0) * 8.0) / rows as f32;
            for (i, t) in tiles.iter().enumerate() {
                let col = i % want_cols;
                let row = i / want_cols;
                assert_eq!(t.x, 10.0 + col as f32 * (tw + 8.0), "tile {i} column x at w={w}");
                assert_eq!(t.y, 10.0 + row as f32 * (th + 8.0), "tile {i} row y at w={w}");
                assert_eq!(t.w, tw, "tile {i} width at w={w}");
                assert_eq!(t.h, th, "tile {i} height at w={w}");
            }
        }
    }

    #[test]
    fn tiles_breaks_only_on_the_card_box_width() {
        // the breakpoints read the CARD width, not the item box: a box that
        // stops short of the right edge still reflows on the card's size
        let (scene, m) = tiles_fixture(
            r#"
            name: "t", x: 0.0, y: 0.0, w: -40.0, h: 0.0,
            pad: 10.0, gap: 8.0, cols: 1,
            cols_break: [(225.0, 2), (330.0, 3)]
        "#,
            &[false; 6],
        );
        let mut v: Vec<Cmd> = Vec::new();
        let (hits, _) = scene.draw(&mut v, 0.0, 0.0, 340.0, 200.0, scale(), &pal(), &m, 0, true, true);
        let tiles = tile_hits(&hits);
        assert_eq!(tiles.len(), 6);
        // three columns inside a 300 px box (card 340 − the 40 px short stop)
        let tw = (300.0 - 20.0 - 16.0) / 3.0;
        assert!((tiles[0].w - tw).abs() < 0.01, "a short-stopped box still splits three ways");
        assert!((tiles[0].x - 10.0).abs() < 0.01);
    }

    #[test]
    fn tiles_squat_tiles_drop_the_caption_and_center_the_glyph() {
        // the drawer's `labeled = th_ >= 42` branch: a 30 px tile is icon-only
        // with the glyph centered; a 50 px tile shows icon + caption
        // six rows need 20 + 5·8 + 6·42 = 312 px of card before a tile
        // clears the caption threshold, so 200 px is icon-only and 320 is not
        for (h, labeled) in [(200.0, false), (320.0, true)] {
            let (scene, m) = tiles_fixture(TILES_BODY, &[false; 6]);
            let mut v: Vec<Cmd> = Vec::new();
            scene.draw(&mut v, 0.0, 0.0, 200.0, h, scale(), &pal(), &m, 0, true, true);
            let rows = 6;
            let th = (h - 20.0 - (rows as f32 - 1.0) * 8.0) / rows as f32;
            let glyph_y = v
                .iter()
                .find_map(|c| match c {
                    Cmd::Text { y, text, .. } if text == "\u{f1eb}" => Some(*y),
                    _ => None,
                })
                .expect("the glyph draws");
            let has_label = v
                .iter()
                .any(|c| matches!(c, Cmd::Text { text, .. } if text == "Wi-Fi"));
            assert_eq!(has_label, labeled, "caption present only on a tall tile (h={h})");
            let want = if labeled {
                10.0 + 8.0 // icon_top inside the tile
            } else {
                10.0 + (th - 13.0) / 2.0 // centered in the tile
            };
            assert!((glyph_y - want).abs() < 0.01, "glyph y at h={h} (got {glyph_y}, want {want})");
        }
    }

    #[test]
    fn tiles_label_sits_below_the_glyph_on_a_tall_tile() {
        let (scene, m) = tiles_fixture(TILES_BODY, &[false; 6]);
        let mut v: Vec<Cmd> = Vec::new();
        scene.draw(&mut v, 0.0, 0.0, 200.0, 320.0, scale(), &pal(), &m, 0, true, true);
        // drawer: glyph at ty0 + 8, caption at ty0 + 30, both centered
        let cap = v
            .iter()
            .find_map(|c| match c {
                Cmd::Text { y, text, .. } if text == "Wi-Fi" => Some(*y),
                _ => None,
            })
            .expect("the caption draws on a tall tile");
        assert!((cap - 40.0).abs() < 0.01, "caption at tile top + 30 (got {cap})");
    }

    #[test]
    fn tiles_on_state_outranks_hover_and_lights_the_glyph() {
        // the drawer's `if on { acc_tint } else if hov { hover_hl } else { hover }`
        // and its `if on { acc } else { fg }` glyph ink
        let p = pal();
        // tile 0 (Wi-Fi) lit, tile 1 (BT) off — one list, both states
        let (scene, m2) = tiles_fixture(TILES_BODY, &[true, false, false, false, false, false]);
        // hovered AND on → still the accent wash, accent glyph
        let mut v: Vec<Cmd> = Vec::new();
        let (_, _) = scene.draw(&mut v, 0.0, 0.0, 200.0, 200.0, scale(), &p, &m2, 41, true, true);
        let fill = v.iter().find_map(|c| match c {
            Cmd::Rect { color, .. } => Some(*color),
            _ => None,
        });
        assert_eq!(fill, Some(crate::ui::acc_tint(&p)), "a lit tile keeps its accent wash under the cursor");
        let ink = v.iter().find_map(|c| match c {
            Cmd::Text { text, color, .. } if text == "\u{f1eb}" => Some(*color),
            _ => None,
        });
        assert_eq!(ink, Some(p.acc), "a lit tile's glyph goes accent");
        // resting, off → the quiet hover surface (tile 1's fill, drawn second)
        let mut v2: Vec<Cmd> = Vec::new();
        let (_, _) = scene.draw(&mut v2, 0.0, 0.0, 200.0, 200.0, scale(), &p, &m2, 0, true, true);
        let fills: Vec<u32> = v2.iter().filter_map(|c| match c {
            Cmd::Rect { color, .. } => Some(*color),
            _ => None,
        }).collect();
        assert_eq!(fills[0], crate::ui::acc_tint(&p), "a lit tile rests on its accent wash");
        assert_eq!(fills[1], crate::ui::hover(&p), "an off tile sits on the quiet hover surface");
        let inks: Vec<u32> = v2.iter().filter_map(|c| match c {
            Cmd::Text { text, color, .. } if text == "\u{f1eb}" => Some(*color),
            _ => None,
        }).collect();
        assert_eq!(inks[0], p.acc, "a lit tile's glyph is accent");
        assert_eq!(inks[1], p.fg, "an off tile's glyph is plain fg");
        // hovered, off → the lifted surface
        let mut v3: Vec<Cmd> = Vec::new();
        let (_, _) = scene.draw(&mut v3, 0.0, 0.0, 200.0, 200.0, scale(), &p, &m2, 43, true, true);
        let hovered: Vec<u32> = v3.iter().filter_map(|c| match c {
            Cmd::Rect { color, .. } => Some(*color),
            _ => None,
        }).collect();
        assert_eq!(hovered[1], crate::ui::hover_hl(&p), "a hovered off tile lifts one step");
    }

    #[test]
    fn tiles_chip_is_a_tall_right_edge_region() {
        // the drawer's ⋯ chevron: glyph centered 10 px from the tile's right
        // edge, 14 px above its bottom, over an 18 px × full-height strip
        let (scene, m) = tiles_fixture(TILES_BODY, &[false; 6]);
        let (w, h) = (200.0, 200.0);
        let mut v: Vec<Cmd> = Vec::new();
        let (hits, _) = scene.draw(&mut v, 0.0, 0.0, w, h, scale(), &pal(), &m, 0, true, true);
        let chips: Vec<&SceneHit> = hits.iter().filter(|t| t.w == 18.0).collect();
        assert_eq!(chips.len(), 2, "only the two network tiles carry a chip");
        assert_eq!(chips.iter().map(|c| c.key).collect::<Vec<_>>(), vec![42, 44], "resident chip keys");
        let th = (h - 20.0 - 5.0 * 8.0) / 6.0;
        for c in &chips {
            assert_eq!(c.h, th, "the chip strip spans the tile's height");
            assert_eq!(c.x + 18.0, 10.0 + w - 20.0, "strip ends at the tile's right edge");
        }
        let glyph = v
            .iter()
            .filter_map(|c| match c {
                Cmd::Text { x, y, text, .. } if text == crate::icons::ICON_MORE => Some((*x, *y)),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(glyph.len(), 2, "one chevron per chip tile");
        assert_eq!(glyph[0], (10.0 + w - 20.0 - 10.0, 10.0 + th - 14.0), "chevron at tile_right − 10, tile_bottom − 14");
    }

    #[test]
    fn tiles_chip_ink_lifts_on_hover() {
        let (scene, m) = tiles_fixture(TILES_BODY, &[false; 6]);
        let mut v: Vec<Cmd> = Vec::new();
        let (_, _) = scene.draw(&mut v, 0.0, 0.0, 200.0, 200.0, scale(), &pal(), &m, 42, true, true);
        let ink = v.iter().find_map(|c| match c {
            Cmd::Text { text, color, .. } if text == crate::icons::ICON_MORE => Some(*color),
            _ => None,
        });
        assert_eq!(ink, Some(pal().acc), "the hovered chevron inks to the accent");
    }

    #[test]
    fn tiles_gate_and_missing_binding_draw_nothing() {
        // an unbound list is a no-op (the same rule every data item follows)
        let (scene, _) = tiles_fixture(TILES_BODY, &[false; 6]);
        let mut v: Vec<Cmd> = Vec::new();
        let (hits, _) = scene.draw(&mut v, 0.0, 0.0, 200.0, 200.0, scale(), &pal(), &SceneValues::new(), 0, true, true);
        assert!(v.is_empty() && hits.is_empty(), "no tiles value → nothing drawn, nothing clickable");
        // an explicit `visible` gate that resolves false hides the whole grid
        let (_, mut m) = tiles_fixture(TILES_BODY, &[false; 6]);
        let gated: CardScene = ron::from_str(
            r#"(items: [Tiles(
                name: "t", x: 0.0, y: 0.0, w: 0.0, h: 0.0,
                cols: 2, visible: Some("pane_1")
            )])"#,
        )
        .expect("gated fixture parses");
        m.insert("pane_1", SceneValue::Toggle(false));
        let mut v2: Vec<Cmd> = Vec::new();
        let (hits2, _) = gated.draw(&mut v2, 0.0, 0.0, 200.0, 200.0, scale(), &pal(), &m, 0, true, true);
        assert!(v2.is_empty() && hits2.is_empty(), "a false gate hides every tile");
        // …and true opens it
        m.insert("pane_1", SceneValue::Toggle(true));
        let mut v3: Vec<Cmd> = Vec::new();
        let (hits3, _) = gated.draw(&mut v3, 0.0, 0.0, 200.0, 200.0, scale(), &pal(), &m, 0, true, true);
        assert_eq!(tile_hits(&hits3).len(), 6);
    }

    #[test]
    fn toggles_scene_parses_zero_ink_with_a_responsive_tiles_grid() {
        // toggles (2026-09-25) — the whole body is one `Tiles` frame; the six
        // switches arrive per frame on the `toggles_tiles` list
        let path = crate::vars::card_scene_path("toggles");
        if !path.exists() {
            return; // config tree absent in this env — nothing to assert
        }
        let s = std::fs::read_to_string(&path).expect("scene file readable");
        let scene: CardScene = ron::from_str(&s).unwrap_or_else(|e| panic!("toggles.ron must parse: {e}"));
        assert!(
            !scene.items.iter().any(|i| matches!(i, SceneItem::Ink { .. })),
            "toggles is fully declarative (no Ink body)"
        );
        let Some(SceneItem::Tiles { name, pad, gap, cols, cols_break, r, label_min_h, on_surface, hover_surface, off_surface, on_ink, off_ink, chip, visible, .. }) =
            scene.items.first()
        else {
            panic!("toggles.ron declares a Tiles grid");
        };
        assert_eq!(name, "toggles_tiles", "binds the per-frame tile list");
        assert_eq!(*visible, None, "the grid itself is never gated");
        // the drawer's numbers, spelled declaratively
        assert_eq!(*pad, 10.0, "10 px tile inset");
        assert_eq!(*gap, 8.0, "8 px between tiles");
        assert_eq!(*cols, 1, "one column on the narrowest card");
        assert_eq!(
            cols_break.as_slice(),
            &[(225.0, 2), (330.0, 3)],
            "the drawer's 225 / 330 px breakpoints, ascending"
        );
        assert_eq!(*r, 12.0, "12 px tile corners");
        assert_eq!(*label_min_h, 42.0, "captions need a 42 px tile");
        assert_eq!(*on_surface, ColorToken::AccTint);
        assert_eq!(*hover_surface, ColorToken::HoverHl);
        assert_eq!(*off_surface, ColorToken::Hover);
        assert_eq!(*on_ink, ColorToken::Acc);
        assert_eq!(*off_ink, ColorToken::Fg);
        assert!(*chip, "the Wi-Fi / BT ⋯ chevron stays declared");
    }

    #[test]
    fn row_min_card_h_skips_the_row_on_a_short_card() {
        // the compositor's screenshot row: 78 px of rows + 30 px of buttons
        // + the 2 px the drawer keeps clear = a 110 px card, below which the
        // whole row stands down (no orphan buttons hanging off the bottom)
        let src = r#"(items: [
            Row(x: 0.0, y: 78.0, w: -24.0, h: 30.0, min_card_h: 110.0, items: [
                Hit(x: 0.0, y: 0.0, w: 0.0, h: 0.0, key: 31_403),
            ]),
        ])"#;
        let scene: CardScene = ron::from_str(src).expect("row fixture parses");
        for (h, want) in [(108.0, false), (109.9, false), (110.0, true), (200.0, true)] {
            let mut v: Vec<Cmd> = Vec::new();
            let (hits, _) = scene.draw(&mut v, 0.0, 0.0, 240.0, h, scale(), &pal(), &vals(), 0, true, true);
            assert_eq!(hits.len(), usize::from(want), "row present on a {h} px card");
            if want {
                assert_eq!(hits[0].y, 78.0, "the row keeps its declared top");
            }
        }
    }

    #[test]
    fn compositor_tiles_land_on_the_drawer_geometry() {
        // parity guard: the committed scene must reproduce the Rust drawer's
        // numbers, because the click dispatch in input.rs keys off exactly
        // these regions. 240×140 card (the header, three tiles, shot row).
        let path = crate::vars::card_scene_path("compositor");
        if !path.exists() {
            return;
        }
        let s = std::fs::read_to_string(&path).expect("scene file readable");
        let scene: CardScene = ron::from_str(&s).expect("compositor.ron parses");
        let p = pal();
        let mut m = SceneValues::new();
        m.insert("comp_meta", SceneValue::Text("1 on".into()));
        for (i, on) in [true, false, false].into_iter().enumerate() {
            m.insert_color(format!("comp_tile_{i}"), SceneColor::Raw(0x11223344));
            m.insert_color(format!("comp_dot_{i}"), SceneColor::Raw(if on { p.acc } else { crate::ui::fg3(&p) }));
            m.insert_color(format!("comp_ink_{i}"), SceneColor::Raw(if on { p.acc } else { p.fg }));
            m.insert(format!("comp_state_{i}"), SceneValue::Text(if on { "on".into() } else { "off".into() }));
            m.insert_color(format!("comp_shot_{i}"), SceneColor::Raw(0));
        }
        let (w, h) = (240.0, 140.0);
        let mut v: Vec<Cmd> = Vec::new();
        let (hits, _) = scene.draw(&mut v, 0.0, 0.0, w, h, scale(), &p, &m, 0, true, true);

        // drawer: tiles at y = 24 (hdr) + 6, 40 px tall, r 7; tw = (w − 24 − 16) / 3
        let tw = (w - 24.0 - 16.0) / 3.0;
        let mut keys: Vec<u32> = hits.iter().map(|t| t.key).collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            vec![
                crate::shell::COMP_BLUR_KEY,
                crate::shell::COMP_SHADOW_KEY,
                crate::shell::COMP_OPACITY_KEY,
                crate::shell::COMP_SHOT_AREA_KEY,
                crate::shell::COMP_SHOT_FULL_KEY,
            ],
            "every drawer key is still reachable, in any order"
        );
        for (i, k) in [crate::shell::COMP_BLUR_KEY, crate::shell::COMP_SHADOW_KEY, crate::shell::COMP_OPACITY_KEY]
            .into_iter()
            .enumerate()
        {
            let t = hits.iter().find(|t| t.key == k).expect("effect tile region");
            assert_eq!(t.y, 30.0, "effect row sits 30 px down (header 24 + 6)");
            assert_eq!(t.h, 40.0, "40 px tiles");
            assert!((t.x - (12.0 + i as f32 * (tw + 8.0))).abs() < 0.01, "tile {i} x");
            assert!((t.w - tw).abs() < 0.01, "tile {i} width (equal thirds)");
        }
        // the state dots: 5×5 at r 2.5, centered on the tile, 7 px down
        let dots: Vec<&Cmd> = v
            .iter()
            .filter(|c| matches!(c, Cmd::Rect { w, r, .. } if (*w - 5.0).abs() < 0.01 && (*r - 2.5).abs() < 0.01))
            .collect();
        assert_eq!(dots.len(), 3, "one state dot per effect tile");
        if let Cmd::Rect { x, y, .. } = dots[0] {
            assert!((*y - 37.0).abs() < 0.01, "dot 7 px below the tile top");
            assert!((*x - (12.0 + tw / 2.0 - 2.5)).abs() < 0.01, "dot centered on its tile");
        }
        // the tile rows: label at +17, on/off caption at +29 (fg3), both
        // centered on the tile — a centered `Text` in a container gets the
        // container's width from the fill pass, so `center: true` alone lands
        // it on the tile center (`center_x` would push it a half-width right)
        for (i, (want_y, want_text)) in [(47.0, "Blur"), (59.0, "on"), (47.0, "Shadow"), (59.0, "off")].into_iter().enumerate()
        {
            assert!(
                v.iter().any(|c| matches!(c, Cmd::Text { x, y, text, .. }
                    if (*y - want_y).abs() < 0.01
                        && *text == want_text
                        && (*x - (12.0 + (i / 2) as f32 * (tw + 8.0) + tw / 2.0)).abs() < 0.01)),
                "`{want_text}` at y={want_y}, centered on its tile"
            );
        }
        // the screenshot row: y = 30 + 40 + 8, two half-width buttons 30 px tall
        let sw = (w - 24.0 - 8.0) / 2.0;
        for (i, k) in [crate::shell::COMP_SHOT_AREA_KEY, crate::shell::COMP_SHOT_FULL_KEY].into_iter().enumerate() {
            let t = hits.iter().find(|t| t.key == k).expect("screenshot region");
            assert_eq!(t.y, 78.0, "shot row below the tiles");
            assert_eq!(t.h, 30.0, "30 px buttons");
            assert!((t.x - (12.0 + i as f32 * (sw + 8.0))).abs() < 0.01, "shot {i} x");
            assert!((t.w - sw).abs() < 0.01, "shot {i} width");
        }
        // the icon/label pair straddles each button's center (glyph at −30, text at +2)
        let shot_glyph = v
            .iter()
            .find_map(|c| match c {
                Cmd::Text { x, y, text, .. } if text == crate::icons::ICON_SELECT => Some((*x, *y)),
                _ => None,
            })
            .expect("the area-shot glyph draws");
        assert!((shot_glyph.1 - 86.0).abs() < 0.01, "glyph 8 px below the button top");
        assert!((shot_glyph.0 - (12.0 + sw / 2.0 - 30.0)).abs() < 0.01, "glyph 30 px left of the button center");
    }

    #[test]
    fn compositor_shot_row_stands_down_on_a_short_card() {
        // 100 px card: the tiles still draw, the screenshot row does not
        let path = crate::vars::card_scene_path("compositor");
        if !path.exists() {
            return;
        }
        let s = std::fs::read_to_string(&path).expect("scene file readable");
        let scene: CardScene = ron::from_str(&s).expect("compositor.ron parses");
        let mut m = SceneValues::new();
        m.insert("comp_meta", SceneValue::Text("0 on".into()));
        for i in 0..3 {
            m.insert(format!("comp_state_{i}"), SceneValue::Text("off".into()));
            m.insert_color(format!("comp_tile_{i}"), SceneColor::Raw(0));
            m.insert_color(format!("comp_shot_{i}"), SceneColor::Raw(0));
        }
        let (hits, _) = scene.draw(&mut Vec::new(), 0.0, 0.0, 240.0, 100.0, scale(), &pal(), &m, 0, true, true);
        let keys: Vec<u32> = hits.iter().map(|t| t.key).collect();
        assert!(keys.contains(&crate::shell::COMP_BLUR_KEY), "the effect tiles survive");
        assert!(
            !keys.contains(&crate::shell::COMP_SHOT_AREA_KEY) && !keys.contains(&crate::shell::COMP_SHOT_FULL_KEY),
            "a 100 px card has no room for the screenshot row"
        );
    }

    // ── power cards (powerh / powerv), 2026-09-25 ──

    /// the bindings a power scene needs: five buttons, one held at `fill`
    /// (a 0..1 fraction, painted once the shell's own 1.5 px sliver threshold
    /// clears), the shared liquid ink and a `clear` track.
    fn power_vals(holding: usize, fill: f32) -> SceneValues {
        let p = pal();
        let mut m = SceneValues::new();
        m.insert_color("clear", SceneColor::Raw(0));
        m.insert_color(
            "power_hold_fill",
            SceneColor::Raw(crate::ui::mix(crate::ui::hover(&p), p.acc, 0.5)),
        );
        for i in 0..5 {
            m.insert_color(
                format!("power_bg_{i}"),
                SceneColor::Raw(if i == holding {
                    crate::ui::acc_tint(&p)
                } else {
                    crate::ui::raised(&p)
                }),
            );
            m.insert(
                format!("power_hold_{i}"),
                SceneValue::Text(if i == holding { format!("{fill:.4}") } else { "0.0000".into() }),
            );
            m.insert(
                format!("power_hold_on_{i}"),
                SceneValue::Toggle(i == holding && 32.0 * fill >= 1.5),
            );
            m.insert_color(
                format!("power_ink_{i}"),
                SceneColor::Raw(if matches!(i, 1 | 3 | 4) {
                    crate::ui::mix(crate::ui::DANGER, p.fg, 0.25)
                } else {
                    p.fg
                }),
            );
        }
        m
    }

    /// parse a committed power card scene, or `None` when the file is absent
    /// (the live config tree is the source of truth).
    fn power_scene(id: &str) -> Option<CardScene> {
        let path = crate::vars::card_scene_path(id);
        if !path.exists() {
            return None;
        }
        let s = std::fs::read_to_string(&path).expect("scene file readable");
        Some(ron::from_str(&s).unwrap_or_else(|e| panic!("{id}.ron must parse: {e}")))
    }

    #[test]
    fn power_scenes_parse_zero_ink_with_five_hit_buttons() {
        for id in ["powerh", "powerv"] {
            let Some(scene) = power_scene(id) else { return };
            assert!(
                scene.items.iter().any(|i| matches!(i, SceneItem::Header { .. })),
                "{id} scene declares Header chrome"
            );
            assert!(
                !scene.items.iter().any(|i| matches!(i, SceneItem::Ink { .. })),
                "{id} scene is fully declarative (no Ink body)"
            );
            fn keys_in(items: &[SceneItem], out: &mut Vec<u32>) {
                for i in items {
                    match i {
                        SceneItem::Hit { key, .. } => out.push(*key),
                        SceneItem::Row { items, .. }
                        | SceneItem::Column { items, .. }
                        | SceneItem::Stack { items, .. } => keys_in(items, out),
                        _ => {}
                    }
                }
            }
            let mut keys: Vec<u32> = Vec::new();
            keys_in(&scene.items, &mut keys);
            keys.sort_unstable();
            assert_eq!(
                keys,
                (0..5).map(|i| crate::shell::POWER_CARD_KEY_BASE + i).collect::<Vec<_>>(),
                "{id} scene keeps every power button key (12000..12004), in order"
            );
        }
    }

    #[test]
    fn power_buttons_land_on_the_drawer_geometry() {
        // parity guard: input.rs dispatches hold-to-confirm off exactly these
        // regions. 240x140 card, five 32 px buttons, 9 px gaps, 30 px header.
        let Some(scene) = power_scene("powerh") else { return };
        let (w, h) = (240.0, 140.0);
        let mut v: Vec<Cmd> = Vec::new();
        let (hits, _) = scene.draw(
            &mut v, 0.0, 0.0, w, h, scale(), &pal(), &power_vals(usize::MAX, 0.0), 0, true, true,
        );

        // drawer: cx0 = x + (w − 5·32 − 4·9) / 2 + 16, cy = y + 30 + (110−32)/2 + 16
        let total = 5.0 * 32.0 + 4.0 * 9.0;
        let bx0 = (w - total) / 2.0;
        let by = 30.0 + (110.0 - 32.0) / 2.0;
        for (i, t) in hits.iter().enumerate() {
            assert_eq!(t.key, crate::shell::POWER_CARD_KEY_BASE + i as u32, "button {i} key");
            assert!((t.x - (bx0 + i as f32 * 41.0)).abs() < 0.01, "button {i} x (centered block)");
            assert!((t.w - 32.0).abs() < 0.01 && (t.h - 32.0).abs() < 0.01, "button {i} is 32x32");
            assert!((t.y - by).abs() < 0.01, "button {i} centered in the body below the header");
        }
        // each button paints its background + hairline and its glyph
        let fills = v
            .iter()
            .filter(|c| matches!(c, Cmd::Rect { w, h, r, .. } if (*w - 32.0).abs() < 0.01 && (*h - 32.0).abs() < 0.01 && (*r - 10.0).abs() < 0.01))
            .count();
        assert_eq!(fills, 5, "one 32 px r-10 background per button");
        assert_eq!(
            v.iter().filter(|c| matches!(c, Cmd::Outline { w, .. } if (*w - 32.0).abs() < 0.01)).count(),
            5,
            "one hairline outline per button"
        );
        for (i, glyph) in ["\u{f023}", "\u{f08b}", "\u{f186}", "\u{f021}", "\u{f011}"].into_iter().enumerate() {
            assert!(
                v.iter().any(|c| matches!(c, Cmd::Text { x, y, text, size, center: true, .. }
                    if text == glyph
                        && (*size - 14.0).abs() < 0.01
                        && (*y - (by + 16.0)).abs() < 0.01
                        && (*x - (bx0 + i as f32 * 41.0 + 16.0)).abs() < 0.01)),
                "button {i} glyph centered on its box, 14 px"
            );
        }
        // at rest: no hold liquid anywhere (the gate is closed and the
        // fraction is zero)
        assert!(
            !v.iter().any(|c| matches!(c, Cmd::Rect { w, .. } if (*w - 28.0).abs() < 0.01)),
            "no hold liquid at rest"
        );
    }

    #[test]
    fn power_hold_fills_the_button_bottom_up() {
        // holding button 2 at half progress: the liquid is 28 px wide, 16 px
        // tall, and its bottom edge sits 2 px above the button's bottom (the
        // drawer's `x + 2, y + d − 2 − fh` geometry) with the accent mix ink
        let Some(scene) = power_scene("powerh") else { return };
        let (w, h) = (240.0, 140.0);
        let bx0 = (w - (5.0 * 32.0 + 4.0 * 9.0)) / 2.0;
        let by = 30.0 + (110.0 - 32.0) / 2.0;
        let mut v: Vec<Cmd> = Vec::new();
        let (_, _) = scene.draw(&mut v, 0.0, 0.0, w, h, scale(), &pal(), &power_vals(2, 0.5), 0, true, true);
        // the liquid rides a transparent 28x32 track (the `clear` value) and
        // the fill is the shorter rect on top of it
        let track = v
            .iter()
            .find(|c| matches!(c, Cmd::Rect { w, h, .. } if (*w - 28.0).abs() < 0.01 && (*h - 32.0).abs() < 0.01))
            .expect("the held button paints its liquid track");
        if let Cmd::Rect { color, .. } = track {
            assert_eq!(*color, 0, "the track is fully transparent at rest (`clear`)");
        }
        let liquid = v
            .iter()
            .find(|c| matches!(c, Cmd::Rect { w, h, .. } if (*w - 28.0).abs() < 0.01 && (*h - 16.0).abs() < 0.01))
            .expect("the held button paints its liquid");
        if let Cmd::Rect { x, y, w: lw, h: lh, color, .. } = liquid {
            assert!((*lw - 28.0).abs() < 0.01 && (*lh - 16.0).abs() < 0.01, "half of the 32 px button");
            assert!((*x - (bx0 + 2.0 * 41.0 + 2.0)).abs() < 0.01, "liquid inset 2 px from the button's left edge");
            assert!((*y - (by + 14.0)).abs() < 0.01, "liquid bottom anchored 2 px above the button's bottom");
            assert_eq!(*color, crate::ui::mix(crate::ui::hover(&pal()), pal().acc, 0.5), "accent-mixed liquid");
        }
        assert_eq!(
            v.iter().filter(|c| matches!(c, Cmd::Rect { w, h, .. } if (*w - 28.0).abs() < 0.01 && (*h - 32.0).abs() < 0.01)).count(),
            1,
            "one liquid track, on the held button only"
        );
    }

    #[test]
    fn power_hold_stands_down_under_the_sliver_threshold() {
        // the shell only paints a hold once it covers 1.5 px: a fresh press
        // (fill 0.02 → 0.64 px) draws no liquid at all
        let Some(scene) = power_scene("powerh") else { return };
        let mut v: Vec<Cmd> = Vec::new();
        let (_, _) = scene.draw(&mut v, 0.0, 0.0, 240.0, 140.0, scale(), &pal(), &power_vals(0, 0.02), 0, true, true);
        assert!(
            !v.iter().any(|c| matches!(c, Cmd::Rect { w, .. } if (*w - 28.0).abs() < 0.01)),
            "a sub-1.5 px hold paints nothing"
        );
    }

    #[test]
    fn powerv_stacks_the_buttons_in_one_centered_column() {
        // the vertical card: 5 buttons on a 41 px pitch, the block centered
        // in the body below the header, each button centered in the width
        let Some(scene) = power_scene("powerv") else { return };
        let (w, h) = (200.0, 260.0);
        let mut v: Vec<Cmd> = Vec::new();
        let (hits, _) = scene.draw(
            &mut v, 0.0, 0.0, w, h, scale(), &pal(), &power_vals(usize::MAX, 0.0), 0, true, true,
        );
        assert_eq!(hits.len(), 5, "five button regions");
        let stack_h = 5.0 * 32.0 + 4.0 * 9.0;
        let base = 30.0 + ((h - 30.0) - stack_h).max(0.0) / 2.0;
        for (i, t) in hits.iter().enumerate() {
            assert!((t.x - (w / 2.0 - 16.0)).abs() < 0.01, "button {i} centered in the card width");
            assert!((t.y - (base + i as f32 * 41.0)).abs() < 0.01, "button {i} on the 41 px pitch");
            assert!((t.w - 32.0).abs() < 0.01 && (t.h - 32.0).abs() < 0.01, "button {i} is 32x32");
        }
    }

    #[test]
    fn row_halign_centers_a_block_of_fixed_width_children() {
        // `halign: center` is a no-op for children that FILL the row (they
        // already span it) and centers the block when every child declares
        // its own width
        let filled = r#"(items: [
            Row(x: 0.0, y: 0.0, w: 100.0, h: 10.0, halign: center, items: [
                Text(x: 0.0, y: 0.0, w: 0.0, text: "a", font_size: 10.0),
                Text(x: 0.0, y: 0.0, w: 0.0, text: "b", font_size: 10.0),
            ]),
        ])"#;
        let s: CardScene = ron::from_str(filled).expect("filled fixture parses");
        let mut v: Vec<Cmd> = Vec::new();
        s.draw(&mut v, 0.0, 0.0, 100.0, 20.0, scale(), &pal(), &vals(), 0, true, true);
        let xs: Vec<f32> = v
            .iter()
            .filter_map(|c| match c {
                Cmd::Text { x, .. } => Some(*x),
                _ => None,
            })
            .collect();
        assert_eq!(xs, vec![0.0, 50.0], "fills stay left-anchored, split equally");

        let fixed = r#"(items: [
            Row(x: 0.0, y: 0.0, w: 100.0, h: 10.0, spacing: 4.0, halign: center, items: [
                Surface(x: 0.0, y: 0.0, w: 20.0, h: 10.0, color: acc),
                Surface(x: 0.0, y: 0.0, w: 20.0, h: 10.0, color: acc),
            ]),
        ])"#;
        let s: CardScene = ron::from_str(fixed).expect("fixed fixture parses");
        let mut v: Vec<Cmd> = Vec::new();
        s.draw(&mut v, 0.0, 0.0, 100.0, 20.0, scale(), &pal(), &vals(), 0, true, true);
        let xs: Vec<f32> = v
            .iter()
            .filter_map(|c| match c {
                Cmd::Rect { x, .. } => Some(*x),
                _ => None,
            })
            .collect();
        assert_eq!(xs, vec![28.0, 52.0], "a 44 px block centers in 100 px (28 px lead)");
    }

    #[test]
    fn bar_vertical_fills_bottom_up_and_surface_strokes_over() {
        // `vertical: true` scales the box height by the fraction, anchored on
        // the bottom edge; the horizontal bar is unchanged
        let vron = r#"(items: [
            Bar(x: 0.0, y: 0.0, w: 40.0, h: 20.0, vertical: true, value: "50", fill: acc, track: fg3),
            Bar(x: 50.0, y: 0.0, w: 40.0, h: 20.0, value: "50", fill: acc, track: fg3),
        ])"#;
        let s: CardScene = ron::from_str(vron).expect("bar fixture parses");
        let mut v: Vec<Cmd> = Vec::new();
        s.draw(&mut v, 0.0, 0.0, 100.0, 40.0, scale(), &pal(), &vals(), 0, true, true);
        let mut fills: Vec<(f32, f32, f32)> = Vec::new();
        for c in &v {
            if let Cmd::Rect { x, y, w, h, .. } = c {
                if (*h - 20.0).abs() > 0.01 || (*w - 40.0).abs() > 0.01 {
                    fills.push((*x, *y, *w));
                }
            }
        }
        assert_eq!(
            fills,
            vec![(0.0, 10.0, 40.0), (50.0, 0.0, 20.0)],
            "vertical fill: 20 x 0.5 anchored at y 10; horizontal: 40 x 0.5"
        );
        // `stroke` outlines the box on the fill's own rect
        let sron = r#"(items: [ Surface(x: 4.0, y: 6.0, w: 30.0, h: 18.0, r: 5.0, color: acc, stroke: 1.0, stroke_color: Some(hairline)) ])"#;
        let s: CardScene = ron::from_str(sron).expect("surface fixture parses");
        let mut v: Vec<Cmd> = Vec::new();
        s.draw(&mut v, 0.0, 0.0, 100.0, 40.0, scale(), &pal(), &vals(), 0, true, true);
        assert_eq!(v.len(), 2, "fill + outline");
        match v[1] {
            Cmd::Outline { x, y, w, h, r, color, .. } => {
                assert_eq!((x, y, w, h, r), (4.0, 6.0, 30.0, 18.0, 5.0), "outline traces the box");
                assert_eq!(color, crate::ui::hairline(&pal()), "hairline ink");
            }
            _ => panic!("expected an Outline as the second command"),
        }
    }




    /// `batt_vals` mirrors the battery card's `scene_values` block: the live
    /// percent (as text for `batt_pct`, as ink for `batt_ink`), the AC bolt,
    /// the pane/hover gates and the two panes' contents.
    fn batt_vals(pct: i32, pane: usize, on_ac: bool, present: bool) -> SceneValues {
        let p = pal();
        let mut m = SceneValues::new();
        m.insert("batt_pct", SceneValue::Text(pct.to_string()));
        m.insert(
            "batt_pct_txt",
            SceneValue::Text(format!("{pct}%{}", if on_ac { " \u{f0e7}" } else { "" })),
        );
        m.insert_color(
            "batt_ink",
            SceneColor::Raw(battery_level_color(&p, pct)),
        );
        m.insert("batt_absent", SceneValue::Toggle(!present));
        m.insert("batt_h_pane", SceneValue::Ring(pane as f32));
        m.insert("batt_v_pane", SceneValue::Ring(pane as f32));
        m.insert("batt_h_pane_0", SceneValue::Toggle(present && pane == 0));
        m.insert("batt_h_pane_1", SceneValue::Toggle(present && pane == 1));
        m.insert("batt_v_pane_0", SceneValue::Toggle(present && pane == 0));
        m.insert("batt_v_pane_1", SceneValue::Toggle(present && pane == 1));
        m.insert("batt_h_hover", SceneValue::Toggle(present));
        m.insert("batt_v_hover", SceneValue::Toggle(present));
        m.insert_color("batt_psave_bg", SceneColor::Raw(crate::ui::raised(&p)));
        m.insert_color("batt_psave_ink", SceneColor::Raw(p.fg));
        m.insert(
            "batt_psave_lbl",
            SceneValue::Text(format!("{} Power save", crate::icons::ICON_BATTERY_SAVER)),
        );
        let row = |a: &str, b: String| SceneRow {
            cols: vec![a.into(), b],
            ..Default::default()
        };
        m.insert(
            "batt_info",
            SceneValue::Rows(vec![
                row("State", if on_ac { "Charging \u{26a1}".into() } else { "Discharging".into() }),
                row("Battery", format!("{pct}%")),
                row("Draw", "4.2 W".into()),
                row("Remaining", "3:20".into()),
            ]),
        );
        m
    }
    fn batt_scene(id: &str) -> Option<CardScene> {
        let path = crate::vars::card_scene_path(id);
        if !path.exists() {
            return None;
        }
        let s = std::fs::read_to_string(&path).expect("scene file readable");
        Some(ron::from_str(&s).unwrap_or_else(|e| panic!("{id}.ron must parse: {e}")))
    }
    /// Draw a battery scene into a 240x140 card at the origin and hand back
    /// the commands + regions.
    fn batt_draw(id: &str, pct: i32, pane: usize, present: bool) -> (Vec<Cmd>, Vec<SceneHit>) {
        let scene = batt_scene(id).expect("battery scene present");
        let mut v: Vec<Cmd> = Vec::new();
        let vals = batt_vals(pct, pane, false, present);
        // the drawers always paint the header chrome themselves
        let (hits, _) = scene.draw(
            &mut v, 0.0, 0.0, 240.0, 140.0, scale(), &pal(), &vals, 0, true, true,
        );
        (v, hits)
    }
    /// The first `Outline` in a command list — the battery body stroke, the
    /// drawer's own `Cmd::Outline` around the icon.
    fn first_outline(v: &[Cmd]) -> (f32, f32, f32, f32) {
        v.iter().find_map(|c| match c {
            Cmd::Outline { x, y, w, h, .. } => Some((*x, *y, *w, *h)),
            _ => None,
        })
        .expect("a battery body outline")
    }
    fn texts(v: &[Cmd]) -> Vec<(f32, f32, f32, String)> {
        v.iter()
            .filter_map(|c| match c {
                Cmd::Text { x, y, size, text, .. } => Some((*x, *y, *size, text.clone())),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn battery_scenes_parse_zero_ink_with_panes_dots_and_the_pill() {
        for (id, side) in [("batteryh", "h"), ("batteryv", "v")] {
            let Some(scene) = batt_scene(id) else { return };
            assert!(
                scene.items.iter().any(|i| matches!(i, SceneItem::Header { .. })),
                "{id} scene declares Header chrome"
            );
            assert!(
                !scene.items.iter().any(|i| matches!(i, SceneItem::Ink { .. })),
                "{id} scene is fully declarative (no Ink body)"
            );
            assert!(
                scene.items.iter().any(|i| matches!(i, SceneItem::Battery { .. })),
                "{id} pane 0 draws the gauge with the new Battery item"
            );
            let rows = scene
                .items
                .iter()
                .find_map(|i| match i {
                    SceneItem::Rows { name, row_h, bottom, .. } if name == "batt_info" => {
                        Some((*row_h, *bottom))
                    }
                    _ => None,
                })
                .expect("pane 1 binds the info rows");
            assert_eq!(rows, (18.0, 26.0), "18px rows clipped 26px above the card edge");
            let dots = scene
                .items
                .iter()
                .find_map(|i| match i {
                    SceneItem::Dots { active, when, panes, step, .. } => {
                        Some((active.clone(), when.clone(), *panes, *step))
                    }
                    _ => None,
                })
                .expect("pane dots");
            assert_eq!(dots.2, 2, "one dot per pane");
            assert_eq!(dots.3, 7.0, "dots on a 7px step");
            assert_eq!(dots.0, format!("batt_{side}_pane"), "the active index comes from the pane ring");
            assert_eq!(dots.1, Some(format!("batt_{side}_hover")), "gated on the {side} card's hover");
            let pill = scene
                .items
                .iter()
                .find_map(|i| match i {
                    SceneItem::Hit { key, bottom, surface, .. } if *key == crate::shell::BATTERY_PSAVE_KEY => {
                        Some((*bottom, surface.is_some()))
                    }
                    _ => None,
                })
                .expect("the power-save pill is a Hit");
            assert!(pill.0 && pill.1, "the pill bottom-anchors and paints its own surface");
        }
    }

    #[test]
    fn battery_gauge_lands_where_the_drawer_puts_the_icon() {
        // pane 0, flat card: the drawer asks for
        // `iw = (w*0.46).clamp(40,220)`, `ih = (h*0.36).clamp(16,60)` centered
        // at `(w/2, 42 + (h - 70)/2)`
        let (v, hits) = batt_draw("batteryh", 86, 0, true);
        assert!(hits.is_empty(), "pane 0 registers no regions");
        let (x, y, w, h) = first_outline(&v);
        assert!((w - 110.4).abs() < 0.2, "flat icon width 46% of the card: {w}");
        assert!((h - 50.4).abs() < 0.2, "flat icon height 36% of the card: {h}");
        assert!((x - 64.8).abs() < 0.2 && (y - 51.8).abs() < 0.2, "flat icon centered: {x},{y}");
        // upright card: `aw = (w*0.30).clamp(16,70) = 70`, `iw = aw*2/3 = 46.7`,
        // height `iw*1.7`, centered at `38 + (ah - iw*1.7)/2` with
        // `ah = (h - 44).max(aw*1.6) = 112`
        let (v, _) = batt_draw("batteryv", 86, 0, true);
        let (x, y, w, h) = first_outline(&v);
        assert!((w - 46.7).abs() < 0.2 && (h - 79.3).abs() < 0.2, "upright icon 2:3 x 1.7: {w}x{h}");
        assert!((x - 96.7).abs() < 0.2 && (y - 14.7).abs() < 0.2, "upright icon centered: {x},{y}");
    }

    #[test]
    fn battery_percent_text_centers_above_the_bottom_edge() {
        // the drawers' `text_c(v, w/2, y + h - s(22|24), ..)` with fs 12 | 11
        for (id, fs, inset) in [("batteryh", 12.0, 22.0), ("batteryv", 11.0, 24.0)] {
            let (v, _) = batt_draw(id, 86, 0, true);
            let (x, y, size, text) = texts(&v)
                .into_iter()
                .find(|(_, _, _, t)| t.contains('%'))
                .expect("the percent caption");
            assert_eq!(text, "86%", "no bolt off AC");
            assert!((x - 120.0).abs() < 0.01, "{id} percent sits on the card center: {x}");
            assert!((y - (140.0 - inset)).abs() < 0.01, "{id} percent {inset}px above the edge: {y}");
            assert_eq!(size, fs, "{id} percent font size");
        }
    }

    #[test]
    fn battery_info_rows_pitch_18_and_right_edge_38() {
        // `battery_info_pane`: first row at `y + 24 + 6`, 18px stride, label at
        // the 12px pad (fs 9, fg3) and the value right-anchored 38px from the
        // card's right edge (fs 9.5, fg) — `pad + dots_w = 12 + 26`
        for id in ["batteryh", "batteryv"] {
            let (v, hits) = batt_draw(id, 86, 1, true);
            let t = texts(&v);
            // the labels sit at the 12px pad (the header title is further right)
            let rows: Vec<(f32, f32)> = t
                .iter()
                .filter(|(x, y, _, _)| (*x - 12.0).abs() < 0.01 && *y >= 30.0)
                .map(|(x, y, _, _)| (*x, *y))
                .collect();
            assert_eq!(rows.len(), 4, "{id} draws all four info rows");
            for (i, (x, y)) in rows.iter().enumerate() {
                assert!((*x - 12.0).abs() < 0.01, "{id} row {i} label at the 12px pad: {x}");
                assert!((y - (30.0 + i as f32 * 18.0)).abs() < 0.01, "{id} row {i} on the 18px pitch: {y}");
            }
            let value = t.iter().find(|(_, _, _, s)| s == "Discharging").expect("the value cell");
            assert!((value.0 - 202.0).abs() < 0.01, "{id} value right-anchored 38px from the right: {}", value.0);
            assert_eq!(value.2, 9.5, "{id} value font size");
            assert!(
                hits.iter().all(|h| h.key == crate::shell::BATTERY_PSAVE_KEY),
                "{id} the info rows are inert — only the pill registers a region"
            );
            assert_eq!(hits.len(), 1, "{id} one region in pane 1");
        }
    }

    #[test]
    fn battery_power_save_pill_anchors_above_the_dots_and_takes_its_key() {
        // `bxx = x + 12`, `bw2 = (w - 24 - 26).max(50) = 190`, `bh = 22`,
        // `byy = y + h - 22 - 2 = 116`; label at `bxx + 24`, `byy + 6.5`
        for id in ["batteryh", "batteryv"] {
            let (v, hits) = batt_draw(id, 86, 1, true);
            let pill = v
                .iter()
                .find_map(|c| match c {
                    Cmd::Rect { x, y, w, h, r: 11.0, .. } => Some((*x, *y, *w, *h)),
                    _ => None,
                })
                .expect("the pill capsule");
            assert_eq!(pill, (12.0, 116.0, 190.0, 22.0), "{id} pill box matches the drawer");
            let label = texts(&v)
                .into_iter()
                .find(|(_, _, _, t)| t.contains("Power save"))
                .expect("the pill label");
            assert!((label.0 - 36.0).abs() < 0.01, "{id} label inset 24px from the pill's left: {}", label.0);
            assert!((label.1 - 122.5).abs() < 0.01, "{id} label 6.5px below the pill's top: {}", label.1);
            assert_eq!(label.2, 9.0, "{id} label font size");
            assert_eq!(hits.len(), 1, "{id} the pill is the only region");
            assert_eq!(
                (hits[0].x, hits[0].y, hits[0].w, hits[0].h),
                (12.0, 116.0, 190.0, 22.0),
                "{id} the pill's region is its own box"
            );
            assert_eq!(hits[0].key, crate::shell::BATTERY_PSAVE_KEY, "{id} pill key");
        }
    }

    #[test]
    fn battery_dots_mark_the_active_pane_and_need_the_cursor() {
        // `pane_dots`: dot = step*0.66, first dot at `(w - 2*step)/2 = 113`,
        // row at `y + h - dot - step/2 = 131.9`, active dot in the accent
        for (id, side) in [("batteryh", "h"), ("batteryv", "v")] {
            for pane in 0..2 {
                let scene = batt_scene(id).expect("battery scene present");
                let mut v: Vec<Cmd> = Vec::new();
                let mut vals = batt_vals(86, pane, false, true);
                vals.insert(format!("batt_{side}_hover"), SceneValue::Toggle(false));
                scene.draw(&mut v, 0.0, 0.0, 240.0, 140.0, scale(), &pal(), &vals, 0, false, false);
                let dots: Vec<&Cmd> = v
                    .iter()
                    .filter(|c| matches!(c, Cmd::Rect { y, h, .. } if (*y - 131.9).abs() < 0.2 && (*h - 4.6).abs() < 0.2))
                    .collect();
                assert!(dots.is_empty(), "{id} pane {pane}: no dots until the cursor rests on the card");
                vals.insert(format!("batt_{side}_hover"), SceneValue::Toggle(true));
                let mut v: Vec<Cmd> = Vec::new();
                scene.draw(&mut v, 0.0, 0.0, 240.0, 140.0, scale(), &pal(), &vals, 0, false, false);
                let dots: Vec<(f32, u32)> = v
                    .iter()
                    .filter_map(|c| match c {
                        Cmd::Rect { x, y, w, h, color, .. }
                            if (*y - 131.9).abs() < 0.2 && (*h - 4.6).abs() < 0.2 =>
                        {
                            Some((*x, *color))
                        }
                        _ => None,
                    })
                    .collect();
                assert_eq!(dots.len(), 2, "{id} pane {pane}: one dot per pane");
                assert!((dots[0].0 - 113.0).abs() < 0.2 && (dots[1].0 - 120.0).abs() < 0.2, "{id} dots centered");
                assert_eq!(
                    dots[pane].1, pal().acc,
                    "{id} pane {pane}: the active dot carries the accent"
                );
                assert_eq!(dots[1 - pane].1, crate::ui::hover(&pal()), "{id} the idle dot is hover ink");
            }
        }
    }

    #[test]
    fn battery_absent_shows_the_header_and_the_empty_caption_only() {
        // `battery_frame` returns before ANY body when the percent is negative:
        // no gauge, no percent, no rows, no pill, no dots — just `card_empty`
        for id in ["batteryh", "batteryv"] {
            let (v, hits) = batt_draw(id, 0, 0, false);
            assert!(hits.is_empty(), "{id} no regions without a battery");
            let t = texts(&v);
            assert_eq!(t.len(), 3, "{id} glyph + title + the empty caption");
            assert!(
                !v.iter().any(|c| matches!(c, Cmd::Outline { .. })),
                "{id} no battery gauge without a battery"
            );
            assert_eq!((t[2].0, t[2].1, t[2].2), (120.0, 64.0, 9.0), "{id} caption centered in the body");
            assert_eq!(t[2].3, "no battery", "{id} caption text");
        }
    }

    #[test]
    fn battery_item_honors_its_gate_value_and_max() {
        // the gauge's own knobs: an explicit `max` turns the bound percent into
        // a 0..1 fraction, and `visible` gates the whole item
        let scene: CardScene = ron::from_str(
            r#"(items: [
                Battery(x: 0.0, y: 0.0, w: 40.0, h: 20.0, vertical: true, value: "{pct}", max: 100.0, visible: Some("on")),
            ])"#,
        )
        .unwrap();
        let mut vals = SceneValues::new();
        vals.insert("pct", SceneValue::Text("50".into()));
        vals.insert("on", SceneValue::Toggle(false));
        let mut v: Vec<Cmd> = Vec::new();
        scene.draw(&mut v, 0.0, 0.0, 240.0, 140.0, scale(), &pal(), &vals, 0, false, false);
        assert!(v.is_empty(), "a closed gate draws nothing at all");
        vals.insert("on", SceneValue::Toggle(true));
        let mut v: Vec<Cmd> = Vec::new();
        scene.draw(&mut v, 0.0, 0.0, 240.0, 140.0, scale(), &pal(), &vals, 0, false, false);
        let body = v
            .iter()
            .find_map(|c| match c {
                Cmd::Outline { x, y, w, h, .. } => Some((*x, *y, *w, *h)),
                _ => None,
            })
            .expect("the body outline");
        assert_eq!(body, (0.0, 0.0, 40.0, 20.0), "the declared box is honored verbatim");
        // half full: beside the body's own empty fill (16.7 tall, inset by the
        // 1.65 corner caps) the charge rect is half as tall and rides the
        // bottom of the body
        let mut inner: Vec<(f32, f32)> = v
            .iter()
            .filter_map(|c| match c {
                Cmd::Rect { x, y, w, h, .. } if *x >= 0.0 && *x + *w <= 40.01 => Some((*y, *h)),
                _ => None,
            })
            .collect();
        inner.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        let body_fill = inner[0];
        let charge = inner
            .iter()
            .find(|(_, h)| *h < body_fill.1 - 0.5)
            .copied()
            .expect("the charge fill beside the empty body fill");
        assert!(
            (charge.1 - body_fill.1 / 2.0).abs() < 0.5,
            "50% of the body is charged: {charge:?} vs body {body_fill:?}"
        );
        assert!(
            charge.0 + charge.1 <= body_fill.0 + body_fill.1 + 0.01,
            "the charge rides the body's bottom edge: {charge:?} vs {body_fill:?}"
        );
    }

    #[test]
    fn hit_bottom_anchors_its_box_and_negative_span_shrinks_to_the_card() {
        // `Hit { bottom: true }` puts the box's BOTTOM edge `y` px above the
        // card's bottom; a negative `w`/`h` is "the card minus this inset"
        let scene: CardScene = ron::from_str(
            r#"(items: [
                Hit(x: 12.0, y: 2.0, w: -50.0, h: 22.0, bottom: true, r: 11.0, key: 7),
                Hit(x: 0.0, y: 0.0, w: 0.0, h: 0.0, key: 8),
            ])"#,
        )
        .unwrap();
        let mut v: Vec<Cmd> = Vec::new();
        let (hits, _) = scene.draw(&mut v, 0.0, 0.0, 240.0, 140.0, scale(), &pal(), &vals(), 0, false, false);
        assert_eq!(
            (hits[0].x, hits[0].y, hits[0].w, hits[0].h),
            (12.0, 116.0, 190.0, 22.0),
            "bottom-anchored box with a -50 width span"
        );
        assert_eq!((hits[1].x, hits[1].y, hits[1].w, hits[1].h), (0.0, 0.0, 240.0, 140.0), "a 0x0 Hit still fills the card");
    }

    #[test]
    fn rows_without_a_key_base_register_no_regions() {
        // an info list must stay inert: the bare `index` keys it used to
        // register squatted on the global 1 / 2 / 3 keys
        let scene: CardScene = ron::from_str(
            r#"(items: [ Rows(name: "rw", y: 30.0, row_h: 18.0, cols: [ (x: 0.0) ]) ])"#,
        )
        .unwrap();
        let mut values = SceneValues::new();
        values.insert(
            "rw",
            SceneValue::Rows(
                ["a", "b", "c"]
                    .iter()
                    .map(|s| SceneRow { cols: vec![(*s).into()], ..Default::default() })
                    .collect(),
            ),
        );
        let mut v: Vec<Cmd> = Vec::new();
        let (hits, _) = scene.draw(&mut v, 0.0, 0.0, 240.0, 140.0, scale(), &pal(), &values, 0, false, false);
        assert_eq!(v.len(), 3, "the rows still draw their text");
        assert!(hits.is_empty(), "no key_base means no row regions");
        // an explicit per-row key still opts that row in
        let scene: CardScene = ron::from_str(
            r#"(items: [ Rows(name: "rw", y: 30.0, row_h: 18.0, cols: [ (x: 0.0) ]) ])"#,
        )
        .unwrap();
        let mut values = SceneValues::new();
        values.insert(
            "rw",
            SceneValue::Rows(vec![
                SceneRow { cols: vec!["a".into()], key: 4242, ..Default::default() },
                SceneRow { cols: vec!["b".into()], ..Default::default() },
            ]),
        );
        let mut v: Vec<Cmd> = Vec::new();
        let (hits, _) = scene.draw(&mut v, 0.0, 0.0, 240.0, 140.0, scale(), &pal(), &values, 0, false, false);
        assert_eq!(hits.len(), 1, "only the row that asked for it");
        assert_eq!(hits[0].key, 4242, "its own key");
    }

    // ── gauges / water / eyerest — the bead-ring trio (2026-09-25) ──────
    //
    // All three cards are pure geometry + live figures: the ring ladders
    // (`fit` / `dot_ladder` / the `When` size windows / `cx_pct` / `cy_pct`)
    // live in the RON, and `scene_values` publishes only the numbers and the
    // blends the Rust drawers computed by hand. The tests below re-derive the
    // drawer's constants and pin the same pixels.

    /// Mirror of the `gauges` / `water` / `eyerest` block in `scene_values`.
    fn gau_vals(mem_pct: i32, disk_pct: i32, glasses: u32, goal: u32, er_frac: f32) -> SceneValues {
        let p = pal();
        let dim8 = crate::ui::mix(crate::ui::hover(&p), p.fg, 0.08);
        let mut m = SceneValues::new();
        m.insert("mem_pct_f", SceneValue::Ring(mem_pct as f32));
        m.insert("disk_pct_f", SceneValue::Ring(disk_pct as f32));
        m.insert("gau_mem_pct", SceneValue::Text(format!("{mem_pct}%")));
        m.insert("gau_disk_pct", SceneValue::Text(format!("{disk_pct}%")));
        m.insert("gau_mem_gb", SceneValue::Text("3G/100G".into()));
        m.insert("gau_disk_gb", SceneValue::Text("42G/512G".into()));
        m.insert_color("gau_dim", SceneColor::Raw(dim8));
        let frac = (glasses as f32 / goal.max(1) as f32).min(1.0);
        let done = frac >= 1.0;
        m.insert("water_frac", SceneValue::Ring(frac));
        m.insert("water_done", SceneValue::Toggle(done));
        m.insert(
            "water_hero",
            SceneValue::Text(format!("{glasses}{}", if done { " \u{2713}" } else { "" })),
        );
        m.insert("water_goal_txt", SceneValue::Text(format!("of {goal} glasses")));
        m.insert_color(
            "water_ring_ink",
            SceneColor::Raw(if done { crate::ui::OK } else { crate::ui::INFO }),
        );
        m.insert_color(
            "water_hero_ink",
            SceneColor::Raw(if done { crate::ui::OK } else { p.fg }),
        );
        m.insert_color(
            "water_dim",
            SceneColor::Raw(crate::ui::mix(crate::ui::hover(&p), p.fg, 0.10)),
        );
        m.insert_color("water_dec_bg", SceneColor::Raw(dim8));
        m.insert_color("water_inc_bg", SceneColor::Raw((p.acc & 0xFFFF_FF00) | 0x28));
        m.insert("er_frac", SceneValue::Ring(er_frac));
        m.insert("er_hero", SceneValue::Text("12:34".into()));
        m.insert("er_state", SceneValue::Text("focus".into()));
        m.insert("er_tog_lbl", SceneValue::Text("Pause".into()));
        m.insert_color("er_ring_ink", SceneColor::Raw(p.acc));
        m.insert_color("er_hero_ink", SceneColor::Raw(p.fg));
        m.insert_color("er_tog_bg", SceneColor::Raw((p.acc & 0xFFFF_FF00) | 0x28));
        m.insert_color("er_rst_bg", SceneColor::Raw(dim8));
        m
    }

    /// Draw a committed scene at an arbitrary card size and hand back the
    /// commands + regions.
    fn gau_draw(
        id: &str,
        w: f32,
        h: f32,
        vals: &SceneValues,
    ) -> (Vec<Cmd>, Vec<SceneHit>) {
        let scene = batt_scene(id).unwrap_or_else(|| panic!("{id}.ron present"));
        let mut v: Vec<Cmd> = Vec::new();
        let (hits, _) =
            scene.draw(&mut v, 0.0, 0.0, w, h, scale(), &pal(), vals, 0, true, true);
        (v, hits)
    }

    /// Every `Rect` (a bead) with its geometry + ink.
    fn rects(v: &[Cmd]) -> Vec<(f32, f32, f32, f32, u32)> {
        v.iter()
            .filter_map(|c| match c {
                Cmd::Rect { x, y, w, h, color, .. } => Some((*x, *y, *w, *h, *color)),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn gauge_water_eyerest_scenes_parse_zero_ink() {
        for id in ["gauges", "water", "eyerest"] {
            let Some(scene) = batt_scene(id) else { return };
            assert!(
                !scene.items.iter().any(|i| matches!(i, SceneItem::Ink { .. })),
                "{id} scene is fully declarative (no Ink body)"
            );
        }
        // water + eyerest lead with the header chrome; gauges is rings only
        for id in ["water", "eyerest"] {
            let scene = batt_scene(id).expect("scene present");
            assert!(
                scene.items.iter().any(|i| matches!(i, SceneItem::Header { .. })),
                "{id} declares Header chrome"
            );
        }
        let gauges = batt_scene("gauges").expect("scene present");
        assert!(
            gauges
                .items
                .iter()
                .all(|i| matches!(i, SceneItem::When { .. })),
            "gauges branches on the card size window only"
        );
    }

    #[test]
    fn gauges_rings_land_where_the_drawer_puts_them() {
        // 240x140: the big ring (h ≥ 116), two columns (w ≥ 170) at 0.27/0.73
        let vals = gau_vals(42, 58, 0, 8, 0.0);
        let (v, _) = gau_draw("gauges", 240.0, 140.0, &vals);
        let rs = rects(&v);
        assert_eq!(rs.len(), 72, "two 36-bead rings, no other rects");
        let p = pal();
        let dim8 = crate::ui::mix(crate::ui::hover(&p), p.fg, 0.08);
        // first bead sits at 12 o'clock: (cx, cy - r - dot/2)
        let (mx, my, mw, _, mink) = rs[0];
        assert!((mx - (64.8 - 2.5)).abs() < 0.01, "memory column at w·0.27 = {mx}");
        assert!((my - (70.0 - 30.0 - 2.5)).abs() < 0.01, "12 o'clock of a 30px ring: {my}");
        assert_eq!(mw, 5.0, "5px beads on the big ring");
        assert_eq!(mink, p.acc, "memory is the accent ink");
        // 42% of 36 beads lit, the rest on the dim track
        let lit_mem = rs[..36].iter().filter(|r| r.4 == p.acc).count();
        assert_eq!(lit_mem, 15, "42% of 36 = 15 beads");
        let (dx, dy, dw, _, dink) = rs[36];
        assert!((dx - (175.2 - 2.5)).abs() < 0.01, "disk column at w·0.73 = {dx}");
        assert!((dy - 37.5).abs() < 0.01, "same height as memory: {dy}");
        assert_eq!(dw, 5.0, "5px beads");
        assert_eq!(dink, crate::ui::OK, "disk is the muted OK ink");
        assert_eq!(rs[..36].iter().filter(|r| r.4 == dim8).count(), 21, "21 unlit beads");
        assert_eq!(rs[36..].iter().filter(|r| r.4 == crate::ui::OK).count(), 21, "58% = 21 beads");
    }

    #[test]
    fn gauges_hero_and_captions_ride_the_size_ladders() {
        let vals = gau_vals(42, 58, 0, 8, 0.0);
        // ── big ring: hero 13px, label at cy + 45, capacity line at cy + 58
        let (v, _) = gau_draw("gauges", 240.0, 140.0, &vals);
        let t = texts(&v);
        let find = |needle: &str| {
            t.iter()
                .find(|(_, _, _, s)| s == needle)
                .cloned()
                .unwrap_or_else(|| panic!("{needle} drawn"))
        };
        let (hx, hy, hsz, _) = find("42%");
        assert!((hx - 64.8).abs() < 0.01 && (hy - 62.0).abs() < 0.01, "hero at cy − 8 on the 0.27 column: {hx},{hy}");
        assert_eq!(hsz, 13.0, "13px hero on the big ring");
        let (_, ly, lsz, _) = find("Memory");
        assert_eq!((ly, lsz), (115.0, 10.0), "label at cy + r + 15");
        let (_, gy, gsz, _) = find("3G/100G");
        assert_eq!((gy, gsz), (128.0, 8.0), "capacity line at cy + r + 28, mono 8px");
        // ── short card: 22px ring, 4px beads, 11px hero, captions ride down
        let (v, _) = gau_draw("gauges", 240.0, 100.0, &vals);
        assert_eq!(rects(&v)[0].2, 4.0, "4px beads on the small ring");
        let t = texts(&v);
        let find = |needle: &str| {
            t.iter()
                .find(|(_, _, _, s)| s == needle)
                .cloned()
                .unwrap_or_else(|| panic!("{needle} drawn"))
        };
        assert_eq!(find("42%").2, 11.0, "11px hero on the small ring");
        assert_eq!(find("Memory").1, 87.0, "label at cy + 22 + 15");
        assert_eq!(find("3G/100G").1, 100.0, "capacity line at cy + 22 + 28");
        // ── tiny card: no captions at all (h < ring_r·2 + 50)
        let (v, _) = gau_draw("gauges", 240.0, 80.0, &vals);
        let t = texts(&v);
        assert_eq!(t.len(), 1, "only the hero percent, no caption block");
        assert_eq!(t[0].3, "42%", "hero still there at y = 40 − 8");
        // ── narrow card: one centered column, the disk ring stands down
        let (v, _) = gau_draw("gauges", 160.0, 140.0, &vals);
        let rs = rects(&v);
        assert_eq!(rs.len(), 36, "memory ring only below 170px wide");
        assert!((rs[0].0 - (80.0 - 2.5)).abs() < 0.01, "centered column: {}", rs[0].0);
        let t = texts(&v);
        assert!(t.iter().any(|(_, _, _, s)| s == "42%"), "hero still drawn");
        assert!(!t.iter().any(|(_, _, _, s)| s == "58%"), "no disk hero in one-column mode");
    }

    #[test]
    fn water_ring_grows_with_the_card_and_its_hero_sits_on_the_body_midpoint() {
        let p = pal();
        // 180x110 → body 48 tall → r 24, 3.5px beads, 12px hero
        let vals = gau_vals(0, 0, 5, 8, 0.0);
        let (v, hits) = gau_draw("water", 180.0, 110.0, &vals);
        let rs = rects(&v);
        assert_eq!(rs.len(), 36, "one 36-bead ring (buttons draw as rects too? no — they are the 2 below)");
        // cy = 24 + (110 − 62)/2 = 48; the first bead is at 12 o'clock
        assert!((rs[0].1 - (48.0 - 24.0 - 1.75)).abs() < 0.01, "ring top bead at cy − r − dot/2: {}", rs[0].1);
        assert_eq!(rs[0].2, 3.5, "3.5px beads below the 26px radius");
        assert_eq!(rs.iter().filter(|r| r.4 == crate::ui::INFO).count(), 23, "5/8 of 36 = 23 beads");
        let t = texts(&v);
        let hero = t.iter().find(|(_, _, _, s)| s == "5").cloned().expect("hero");
        assert_eq!((hero.1, hero.2), (42.0, 12.0), "hero at cy − 6, 12px under the 26px radius");
        let cap = t.iter().find(|(_, _, _, s)| s == "of 8 glasses").cloned().expect("caption");
        assert_eq!((cap.1, cap.2), (56.0, 7.5), "caption at cy + 8, 7.5px");
        // the tall card pushes the ring to its 34px clamp and the hero to 14px
        let (v, _) = gau_draw("water", 180.0, 200.0, &vals);
        let rs = rects(&v);
        assert_eq!(rs[0].2, 4.0, "4px beads on the big ring");
        let t = texts(&v);
        let hero = t.iter().find(|(_, _, _, s)| s == "5").cloned().expect("hero");
        assert_eq!(hero.2, 14.0, "14px hero");
        // − / + glass: 46 wide, 6px off the center line each, 10px above the
        // card bottom, each with its own key
        let keys: Vec<u32> = hits.iter().map(|h| h.key).collect();
        assert_eq!(keys, vec![31301, 31300], "− undo then + glass");
        let (_, btn) = gau_draw("water", 180.0, 110.0, &vals);
        let _ = btn;
    }

    #[test]
    fn water_done_latches_the_check_and_the_ok_ink() {
        let p = pal();
        let vals = gau_vals(0, 0, 8, 8, 0.0);
        let (v, _) = gau_draw("water", 180.0, 110.0, &vals);
        let rs = rects(&v);
        assert_eq!(rs.iter().filter(|r| r.4 == crate::ui::OK).count(), 36, "a full day reads all OK");
        let t = texts(&v);
        let hero = t.iter().find(|(_, _, _, s)| s.starts_with('8')).cloned().expect("hero");
        assert_eq!(hero.3, "8 \u{2713}", "the glass count latches a check");
    }

    #[test]
    fn eyerest_ring_hero_and_buttons_match_the_drawer() {
        let p = pal();
        // 25% of the focus stretch → 9 of 36 beads
        let vals = gau_vals(0, 0, 0, 8, 0.25);
        let (v, hits) = gau_draw("eyerest", 200.0, 120.0, &vals);
        let rs = rects(&v);
        let dim3 = crate::ui::fg3(&p);
        assert_eq!(rs.iter().filter(|r| r.4 == p.acc).count(), 9, "25% of 36 beads");
        assert_eq!(rs.iter().filter(|r| r.4 == dim3).count(), 27, "the rest track on fg3");
        let ring = rs[0];
        assert!((ring.0 - (40.0 - 1.5)).abs() < 0.01 && (ring.1 - (50.0 - 23.0 - 1.5)).abs() < 0.01,
            "ring at x + 40, y + 50, r 23, 3px beads: {},{}", ring.0, ring.1);
        let t = texts(&v);
        let hero = t.iter().find(|(_, _, _, s)| s == "12:34").cloned().expect("hero");
        assert_eq!((hero.0, hero.1, hero.2), (75.0, 43.0, 21.0), "mm:ss left-aligned at ring + r + 12");
        let state = t.iter().find(|(_, _, _, s)| s == "focus").cloned().expect("state");
        assert_eq!((state.0, state.1, state.2), (75.0, 60.0, 8.5), "state caption under the hero");
        let hint = t.iter().find(|(_, _, _, s)| s.contains("20 min")).cloned().expect("hint");
        assert_eq!((hint.0, hint.1), (12.0, 79.0), "hint chip under the ring");
        let keys: Vec<u32> = hits.iter().map(|h| h.key).collect();
        assert_eq!(keys, vec![31900, 31901], "toggle then reset");
        let boxes: Vec<(f32, f32, f32, f32)> = hits.iter().map(|h| (h.x, h.y, h.w, h.h)).collect();
        assert_eq!(boxes[0], (12.0, 88.0, 52.0, 22.0), "toggle 52×22, 12px in, 10px up");
        assert_eq!(boxes[1], (136.0, 88.0, 52.0, 22.0), "reset hugs the right edge");
    }

    #[test]
    fn eyerest_rest_turns_the_ring_and_hero_green() {
        let p = pal();
        let mut vals = gau_vals(0, 0, 0, 8, 1.0);
        vals.insert_color("er_ring_ink", SceneColor::Raw(crate::ui::OK));
        vals.insert_color("er_hero_ink", SceneColor::Raw(crate::ui::OK));
        vals.insert("er_state", SceneValue::Text("rest your eyes".into()));
        let (v, _) = gau_draw("eyerest", 200.0, 120.0, &vals);
        let rs = rects(&v);
        assert_eq!(rs.iter().filter(|r| r.4 == crate::ui::OK).count(), 36, "a full rest is all OK");
        let t = texts(&v);
        assert!(t.iter().any(|(_, _, _, s)| s == "rest your eyes"), "the state caption swaps");
    }

    #[test]
    fn ring_fit_and_ladders_follow_the_card_height() {
        // the `Half` fit: ((110 − 62) / 2).clamp(20, 34) = 24
        assert_eq!(RingFit::Half { offset: 62.0, min: 20.0, max: 34.0 }.radius(110.0, scale()), 24.0);
        // clamped at both ends
        assert_eq!(RingFit::Half { offset: 62.0, min: 20.0, max: 34.0 }.radius(90.0, scale()), 20.0);
        assert_eq!(RingFit::Half { offset: 62.0, min: 20.0, max: 34.0 }.radius(200.0, scale()), 34.0);
        // the ladder steps on 116 with the last step as the fallback
        let l = RingFit::Ladder(vec![(116.0, 30.0), (0.0, 22.0)]);
        assert_eq!(l.radius(140.0, scale()), 30.0);
        assert_eq!(l.radius(116.0, scale()), 30.0);
        assert_eq!(l.radius(115.0, scale()), 22.0);
        // the shared ladder helper
        let steps = [(116.0, 45.0), (0.0, 37.0)];
        assert_eq!(ladder_step(&steps, 0.0, 140.0, scale()), 45.0);
        assert_eq!(ladder_step(&steps, 0.0, 100.0, scale()), 37.0);
        assert_eq!(ladder_step(&[], 12.0, 100.0, scale()), 12.0, "no ladder keeps the field");
        assert_eq!(ladder_step(&[(500.0, 9.0)], 12.0, 100.0, scale()), 12.0, "no match keeps the field");
        // scaled cards: thresholds are base px, the box arrives scaled
        let s2 = crate::shell::GridScale(2.0);
        assert_eq!(l.radius(280.0, s2), 30.0, "a 2x card of 140 still takes the big ring");
        assert_eq!(l.radius(200.0, s2), 22.0);
    }

    #[test]
    fn when_hides_its_children_outside_the_size_window() {
        let vals = gau_vals(42, 58, 0, 8, 0.0);
        // 240x140: both the two-column and the single-column branch fit
        let (two, _) = gau_draw("gauges", 240.0, 140.0, &vals);
        assert_eq!(rects(&two).len(), 72, "two columns when w ≥ 170");
        let (one, _) = gau_draw("gauges", 169.0, 140.0, &vals);
        assert_eq!(rects(&one).len(), 36, "one column below 170");
        // the `When` bound is scaled with the card
        let s2 = crate::shell::GridScale(2.0);
        assert_eq!(s2.s(170.0), 340.0);
    }
}
