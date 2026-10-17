//! The widget system core — Noctalia-v5-style owned widgets, zen-shell style
//! (`WIDGETS.md`). Replaces the LuaJIT UI host: a card (or surface) becomes a
//! function from that frame's backend data to a widget [`Node`] tree.
//!
//! Layers:
//! - **Widget core** (this file): builder API + layout + render. Knows only
//!   `ui.rs` helpers and `Cmd` — never touches shell state.
//! - **Cards** (future `cards.rs`): build trees, mint hit keys, own their
//!   state. Never push raw `Cmd::…`.
//! - **Shell** (`shell.rs`): supplies the [`Measurer`] (renderer-owned text
//!   metrics, cached there as today) and the [`RegionSink`] (the existing
//!   `region()` hit pipeline) per frame.
//!
//! Phases per frame, enforced order:
//! 1. **Update** — the card snaps backend data into the tree's plain values
//!    (builder calls); nothing else runs.
//! 2. **Layout** — [`Node::layout`] computes geometry from the granted rect;
//!    all structural work happens here.
//! 3. **Render** — [`Node::render`] emits `Cmd`s + registers hit regions; it
//!    does no geometry work and asserts the tree was laid out.
//!
//! The tree is rebuilt per frame (zen-shell already redraws only on
//! configure/hover/data-change, so retention buys nothing yet — see
//! WIDGETS.md §Phases). Visual parity with the retired drawers comes free:
//! every visual delegates to the SAME `ui.rs` helpers they used.

//!
//! STAGE 4 of the port (`WIDGETS.md` §Port strategy): the shell-side wiring
//! is live — the shell supplies the [`RegionSink`]
//! (`shell::ShellSink`, the existing `region()` pipeline) and renders whole
//! surfaces' chrome through trees (`settings_frame`). The `#![allow]` stays
//! only for builders not yet referenced outside unit tests (Graph, Image,
//! Scroll, …); it drops when the those cards port.
#![allow(dead_code)]

use crate::cmd::Cmd;
use crate::ui::Pal;

// ── environment traits (shell-supplied) ─────────────────────────────────────

/// Text metrics, owned by the render/text layer (widths are glyph-cached
/// there). The shell passes one per frame; tests pass stubs.
pub trait Measurer {
    /// (width, height) of the rendered run — the same metrics the renderer
    /// uses when it paints `Cmd::Text` at that size/family/weight.
    fn measure(
        &mut self,
        text: &str,
        size: f32,
        f: crate::text::Ff,
        w: crate::text::Fw,
        icon: bool,
    ) -> (f32, f32);
}

/// Hit-region sink — the shell's `region(x, y, w, h, key)` pipeline. Widgets
/// with a `key` register their visual rect here; input routing through
/// `click()` is UNCHANGED from the Lua host era.
pub trait RegionSink {
    fn region(&mut self, x: f32, y: f32, w: f32, h: f32, key: u32);
    /// whether `key` is the element under the pointer (hover styling)
    fn hov(&self, key: u32) -> bool;
}

// ── geometry ────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    /// shrink by (left, top, right, bottom), clamped to positives
    pub fn inset(self, l: f32, t: f32, r: f32, b: f32) -> Rect {
        Rect {
            x: self.x + l,
            y: self.y + t,
            w: (self.w - l - r).max(0.0),
            h: (self.h - t - b).max(0.0),
        }
    }
}

/// Main-axis sizing a container grants its children.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Sizing {
    /// hug content (labels: their measured run)
    Hug,
    /// fixed length along the container's main axis (excludes margins)
    Fixed(f32),
    /// take a weighted share of the leftover main-axis space
    Expand(f32),
}

/// Label horizontal alignment inside its slot.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum HAlign {
    #[default]
    Left,
    Center,
    Right,
}

/// Label vertical alignment inside its slot.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum VAlign {
    #[default]
    Center,
    Top,
    Bottom,
}

/// Series-graph style.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum GMode {
    /// one rounded bar per sample
    #[default]
    Bars,
    /// capsule segments between samples (heartbeat style)
    Line,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Kind {
    /// filled rounded rect (fill = `bg`, 0 = none); children overlay it
    Box,
    /// horizontal flow
    Row,
    /// vertical flow
    Col,
    /// one text run
    Label,
    /// hover-styled hit region (fills `bg`, pal-derived unless overridden)
    /// with one centered label
    Button,
    /// pill track + knob; `on` state; `key` hit region; width = granted slot
    Toggle,
    /// delegates to `ui::slider`; `val` 0..1; `key` region if set
    Slider,
    /// series visualization (`series`, `gmode`)
    Graph,
    /// `Cmd::Image` fitted to the box (`imkey` = ImageStore key)
    Image,
    /// Center-anchored text: one `Cmd::Text` at (slot centre + dx, + dy) with
    /// `center: true` — the renderer-anchor contract the retired drawers
    /// used (`ui.heroc` / `ui.textc`). No measurement; coords are pre-scaled
    /// DEVICE px (the Lua host's `s()` / `fs()` semantics, applied by the
    /// card before building).
    Center { dx: f32, dy: f32 },
    /// clip the single child to the granted rect, offset by `off` px
    Scroll { off: (f32, f32) },
    /// transparent hit region — registers `key` into the [`RegionSink`],
    /// paints NOTHING. Hover/selected styling lives on sibling nodes so a
    /// region's rect can be bigger than its visual box.
    Hit,
    /// bead ring: `beads` round dots evenly spaced on a circle of radius
    /// `beads_r`, the first `beads_lit` filled with `color` and the rest with
    /// `bg` (the dim track). Centre = slot centre + (dx, dy); dot diameter =
    /// `beads_dot`. Paints only — no hit region (the retired
    /// `scene::bead_ring` / `ui.ring` shape).
    Beads { dx: f32, dy: f32 },
    /// segmented progress bar — a track rounded rect (`bg`) plus a rounded
    /// fill (`color`) clamped to `0..bar_max` (sub-0.1 % culled). Fills the
    /// granted slot; `vertical` plugs bottom-up. The retired `bar_prim`.
    Bar { vertical: bool },
    /// prebuilt `Cmd`s stamped at the slot's top-left (each command's x/y is
    /// shifted by `laid.x`/`laid.y`) — the escape hatch for bespoke vector
    /// work (moon disc, spectrum, sparks) that carries its own geometry.
    Stamp,
}

/// One widget node. Build via the constructors below; chain props with the
/// suffixed setters (`.expand()`, `.font(..)`, `.radius(..)`, …).
#[derive(Clone, Debug)]
pub struct Node {
    kind: Kind,
    children: Vec<Node>,
    main: Sizing,
    margin: (f32, f32, f32, f32), // (left, top, right, bottom)
    pad: (f32, f32, f32, f32),
    gap: f32,
    halign: HAlign,
    valign: VAlign,
    text: String,
    text_size: f32,
    ff: crate::text::Ff,
    fw: crate::text::Fw,
    icon: bool,
    color: u32, // text / graph color (0 = derive from pal)
    bg: u32,    // box / button / toggle fill (0 = derive from pal)
    radius: f32,
    key: u32,
    on: bool,
    val: f32,
    series: Vec<f32>,
    gmode: GMode,
    imkey: String,
    beads: usize,
    beads_lit: usize,
    beads_dot: f32,
    beads_r: f32,
    bar_raw: f32,
    bar_max: f32,
    stamp: Vec<Cmd>,
    mw: f32,
    mh: f32,
    /// filled by `layout()`; `render()` asserts it
    laid: Rect,
}

impl Default for Node {
    /// mem::take sentinel for in-place layouts — a cheap empty node that is
    /// replaced before any render, never drawn.
    fn default() -> Self {
        Node::bare(Kind::Row)
    }
}

impl Node {
    // ── constructors ───────────────────────────────────────────────────────

    /// Filled rounded rect. Children OVERLAY it: every child is granted the
    /// full content rect (wrap children in `row()`/`col()` to flow them).
    pub fn box_(r: f32, bg: u32) -> Node {
        Node::bare(Kind::Box)
            .radius_(r)
            .bg_(bg)
            .sizing(Sizing::Expand(1.0))
    }

    /// Horizontal flow container.
    pub fn row() -> Node {
        Node::bare(Kind::Row).sizing(Sizing::Expand(1.0))
    }

    /// Vertical flow container.
    pub fn col() -> Node {
        Node::bare(Kind::Col).sizing(Sizing::Expand(1.0))
    }

    /// Text label (UI family, Light) at `size` px in `color`.
    pub fn label(t: impl Into<String>, size: f32, color: u32) -> Node {
        Node::bare(Kind::Label)
            .sizing(Sizing::Hug)
            .text_(t)
            .size_(size)
            .color_(color)
    }

    /// Icon glyph (PUA codepoint run in the icon font).
    pub fn glyph(g: impl Into<String>, size: f32, color: u32) -> Node {
        Node::label(g, size, color).icon()
    }

    /// Hover-styled button: pal-derived fill (or `.bg_`), one centered
    /// label; `key` is its hit region.
    pub fn button(label: impl Into<String>, size: f32, key: u32) -> Node {
        Node::bare(Kind::Button)
            .sizing(Sizing::Expand(1.0))
            .radius_(8.0)
            .text_(label)
            .size_(size)
            .key_(key)
    }

    /// Pill chip — button with capsule corners (default radius < 0).
    pub fn chip(glyph: impl Into<String>, size: f32, key: u32) -> Node {
        Node::button(glyph, size, key).radius_(-1.0)
    }

    /// Center-anchored text — `Cmd::Text` at the slot centre + (dx, dy),
    /// `center: true`, exactly what `ui.heroc` / `ui.textc` emitted. Text,
    /// size, font, color and icon are the usual chained props; the card
    /// pre-scales dx/dy/size (device px, same `s()`/`fs()` math as Lua).
    pub fn anchored(dx: f32, dy: f32) -> Node {
        Node::bare(Kind::Center { dx, dy }).sizing(Sizing::Expand(1.0))
    }

    /// On/off pill track + knob; `w` is the track width, height = slot.
    pub fn toggle(on: bool, key: u32, w: f32) -> Node {
        Node::bare(Kind::Toggle)
            .sizing(Sizing::Fixed(w))
            .key_(key)
            .on_(on)
    }

    /// `ui::slider` at `val` (0..1); `key` (if set) registers for hover+drag.
    pub fn slider(val: f32, key: u32) -> Node {
        Node::bare(Kind::Slider)
            .sizing(Sizing::Expand(1.0))
            .val_(val)
            .key_(key)
    }

    /// Transparent hit region for `key` — paints nothing, registers
    /// `(x, y, w, h) = this node's laid rect` so the existing `region()` /
    /// `click()` pipeline routes it. Size it and position it through the
    /// flow containers around it (`.fixed(w)` + margins).
    pub fn hit(key: u32) -> Node {
        Node::bare(Kind::Hit)
            .sizing(Sizing::Expand(1.0))
            .key_(key)
    }

    /// Series graph, `gmode`, fill color (0 = accent).
    pub fn graph(series: Vec<f32>, gmode: GMode, color: u32) -> Node {
        Node::bare(Kind::Graph)
            .sizing(Sizing::Expand(1.0))
            .series_(series)
            .gmode_(gmode)
            .color_(color)
    }

    /// Raster image (aspect-fit box), `imkey` = ImageStore key.
    pub fn image(imkey: impl Into<String>) -> Node {
        Node::bare(Kind::Image)
            .sizing(Sizing::Expand(1.0))
            .imkey_(imkey)
    }

    /// Bead-ring gauge: `beads` dots of diameter `dot` on a circle of radius
    /// `radius`, the first `lit` filled with `col` and the rest with `dim`.
    /// Centred on the slot centre + `(dx, dy)`. Paints only; no hit region.
    #[allow(clippy::too_many_arguments)]
    pub fn beads(
        beads: usize,
        lit: usize,
        dot: f32,
        radius: f32,
        col: u32,
        dim: u32,
        dx: f32,
        dy: f32,
    ) -> Node {
        let mut n = Node::bare(Kind::Beads { dx, dy }).sizing(Sizing::Expand(1.0));
        n.beads = beads;
        n.beads_lit = lit;
        n.beads_dot = dot;
        n.beads_r = radius;
        n.color = col;
        n.bg = dim;
        n
    }

    /// Segmented progress bar filling the granted slot: track `track` +
    /// rounded `fill` (`col`) clamped to `0..max` (sub-0.1 % culled).
    /// `vertical` plugs bottom-up. The retired `bar_prim`.
    pub fn bar(raw: f32, max: f32, fill: u32, track: u32, radius: f32, vertical: bool) -> Node {
        let mut n = Node::bare(Kind::Bar { vertical }).sizing(Sizing::Expand(1.0));
        n.bar_raw = raw;
        n.bar_max = max;
        n.color = fill;
        n.bg = track;
        n.radius = radius;
        n
    }

    /// Stamp prebuilt `Cmd`s at the slot's top-left (escape hatch for bespoke
    /// vector work — the card computes geometry relative to the origin, the
    /// tree positions the stamp). No hit region.
    pub fn stamp(cmds: Vec<Cmd>) -> Node {
        let mut n = Node::bare(Kind::Stamp).sizing(Sizing::Expand(1.0));
        n.stamp = cmds;
        n
    }

    /// Clip the single child to the granted rect, offset by `(dx, dy)` px
    /// (card-local scrolling). Child content sizing is computed naturally.
    pub fn scroll(child: Node, off: (f32, f32)) -> Node {
        let mut n = Node::bare(Kind::Scroll { off }).sizing(Sizing::Expand(1.0));
        n.children.push(child);
        n
    }

    fn bare(kind: Kind) -> Node {
        Node {
            kind,
            children: Vec::new(),
            main: Sizing::Fixed(0.0),
            margin: (0.0, 0.0, 0.0, 0.0),
            pad: (0.0, 0.0, 0.0, 0.0),
            gap: 0.0,
            halign: HAlign::default(),
            valign: VAlign::default(),
            text: String::new(),
            text_size: 16.0,
            ff: crate::text::Ff::default(),
            fw: crate::text::Fw::default(),
            icon: false,
            color: 0,
            bg: 0,
            radius: 0.0,
            key: 0,
            on: false,
            val: 0.0,
            series: Vec::new(),
            gmode: GMode::default(),
            imkey: String::new(),
            beads: 0,
            beads_lit: 0,
            beads_dot: 0.0,
            beads_r: 0.0,
            bar_raw: 0.0,
            bar_max: 100.0,
            stamp: Vec::new(),
            mw: 0.0,
            mh: 0.0,
            laid: Rect::default(),
        }
    }

    // ── chained props (suffix `_` keeps field/method namespaces apart) ─────

    pub fn child(mut self, c: Node) -> Node {
        self.children.push(c);
        self
    }

    pub fn children(mut self, cs: impl IntoIterator<Item = Node>) -> Node {
        self.children.extend(cs);
        self
    }

    pub fn sizing(mut self, s: Sizing) -> Node {
        self.main = s;
        self
    }

    /// take 1 unit of leftover main-axis space
    pub fn expand(self) -> Node {
        self.sizing(Sizing::Expand(1.0))
    }

    pub fn expand_w(self, weight: f32) -> Node {
        self.sizing(Sizing::Expand(weight.max(0.0)))
    }

    pub fn fixed(self, v: f32) -> Node {
        self.sizing(Sizing::Fixed(v))
    }

    pub fn margin(mut self, l: f32, t: f32, r: f32, b: f32) -> Node {
        self.margin = (l, t, r, b);
        self
    }

    /// padding around the content box
    pub fn pad(mut self, l: f32, t: f32, r: f32, b: f32) -> Node {
        self.pad = (l, t, r, b);
        self
    }

    /// space between children (main axis)
    pub fn gap(mut self, g: f32) -> Node {
        self.gap = g.max(0.0);
        self
    }

    pub fn halign(mut self, a: HAlign) -> Node {
        self.halign = a;
        self
    }

    pub fn center_x(self) -> Node {
        self.halign(HAlign::Center)
    }

    pub fn valign(mut self, a: VAlign) -> Node {
        self.valign = a;
        self
    }

    pub fn text_(mut self, t: impl Into<String>) -> Node {
        self.text = t.into();
        self
    }

    pub fn size_(mut self, s: f32) -> Node {
        self.text_size = s.max(0.0);
        self
    }

    pub fn font(mut self, f: crate::text::Ff, w: crate::text::Fw) -> Node {
        self.ff = f;
        self.fw = w;
        self
    }

    pub fn icon(mut self) -> Node {
        self.icon = true;
        self
    }

    pub fn color_(mut self, c: u32) -> Node {
        self.color = c;
        self
    }

    pub fn bg_(mut self, c: u32) -> Node {
        self.bg = c;
        self
    }

    /// corner radius; negative = capsule (height/2)
    pub fn radius_(mut self, r: f32) -> Node {
        self.radius = r;
        self
    }

    pub fn key_(mut self, k: u32) -> Node {
        self.key = k;
        self
    }

    pub fn on_(mut self, on: bool) -> Node {
        self.on = on;
        self
    }

    pub fn val_(mut self, v: f32) -> Node {
        self.val = v.clamp(0.0, 1.0);
        self
    }

    pub fn series_(mut self, s: Vec<f32>) -> Node {
        self.series = s;
        self
    }

    pub fn gmode_(mut self, m: GMode) -> Node {
        self.gmode = m;
        self
    }

    pub fn imkey_(mut self, k: impl Into<String>) -> Node {
        self.imkey = k.into();
        self
    }

    // ── phase 2: layout ─────────────────────────────────────────────────────

    /// Compute geometry for `self` granted `r`; recurses into children.
    /// Call ONCE per tree per frame, after the card wrote its data (Update)
    /// and before `render()`.
    pub fn layout(mut self, r: Rect, m: &mut dyn Measurer) -> Node {
        let inner = r.inset(self.pad.0, self.pad.1, self.pad.2, self.pad.3);
        match self.kind {
            Kind::Box => {
                // overlay semantics: all children share the content rect,
                // inset by each child's OWN margin — the margins are the
                // child's absolute placement inside the overlay (a row/col
                // around it handles flow distribution inside that slot)
                for i in 0..self.children.len() {
                    let (ml, mt, mr, mb) = self.children[i].margin;
                    let cr = inner.inset(ml, mt, mr, mb);
                    let moved = std::mem::take(&mut self.children[i]);
                    self.children[i] = moved.layout(cr, m);
                }
            }
            Kind::Row => self.flow(inner, m, false),
            Kind::Col => self.flow(inner, m, true),
            Kind::Scroll { off } => {
                if let Some(c) = self.children.first() {
                    // natural content height (vertical lists are the rule)
                    let hug = c.hug_main(m, true);
                    let cr = Rect {
                        x: inner.x - off.0,
                        y: inner.y - off.1,
                        w: inner.w,
                        h: hug.max(inner.h),
                    };
                    self.children[0] = std::mem::take(&mut self.children[0]).layout(cr, m);
                }
            }
            Kind::Label => {
                let (mw, mh) = m.measure(&self.text, self.text_size, self.ff, self.fw, self.icon);
                self.mw = mw;
                self.mh = mh;
            }
            _ => {
                // Button/Toggle/Slider/Graph/Image/Image size from the slot;
                // Button records its text metrics here too.
                if matches!(self.kind, Kind::Button) && !self.text.is_empty() {
                    let (mw, mh) =
                        m.measure(&self.text, self.text_size, self.ff, self.fw, self.icon);
                    self.mw = mw;
                    self.mh = mh;
                }
            }
        }
        self.laid = r;
        self
    }

    /// Main-axis flex pass: stack children along `vertical`, distributing
    /// `Sizing::Expand` children across the free space (weights), honoring
    /// margins and `gap`.
    fn flow(&mut self, inner: Rect, m: &mut dyn Measurer, vertical: bool) {
        let n = self.children.len();
        if n == 0 {
            return;
        }
        // natural main sizes + margins of non-expand children
        let mut natural = vec![0.0; n];
        let mut fixed_sum = 0.0;
        let mut margins_sum = 0.0;
        let mut weight_sum = 0.0;
        for (i, c) in self.children.iter().enumerate() {
            natural[i] = c.natural_main(m, vertical);
            let msum = margin_main(c, vertical);
            match c.main {
                Sizing::Expand(w) => {
                    weight_sum += w.max(0.0);
                    margins_sum += msum;
                }
                _ => {
                    fixed_sum += natural[i];
                    margins_sum += msum;
                }
            }
        }
        // leftover goes to Expand children AFTER every fixed content, every
        // child margin (expand children pay their own margins too) and gaps
        let gaps = self.gap * (n.saturating_sub(1)) as f32;
        let free =
            (main_len(inner, vertical) - fixed_sum - margins_sum - gaps).max(0.0);
        let mut cursor = main_coord(inner, vertical);
        for (i, c) in self.children.iter_mut().enumerate() {
            let c_main = c.main;
            let (ml, mt, mr, mb) = c.margin;
            let (m0, m1) = if vertical { (mt, mb) } else { (ml, mr) };
            cursor += m0;
            let side = match c_main {
                Sizing::Expand(w) => free * (w.max(0.0) / weight_sum.max(1e-6)),
                _ => natural[i],
            };
            // cross-axis: child stretches to the container minus cross margins
            let child_rect = if vertical {
                Rect {
                    x: inner.x + ml,
                    y: cursor,
                    w: (inner.w - ml - mr).max(0.0),
                    h: side,
                }
            } else {
                Rect {
                    x: cursor,
                    y: inner.y + mt,
                    w: side,
                    h: (inner.h - mt - mb).max(0.0),
                }
            };
            let moved = std::mem::take(c);
            *c = moved.layout(child_rect, m);
            cursor += side + m1 + self.gap;
        }
    }

    /// A child's CONTENT main-axis length (margins excluded — `flow()` and
    /// `hug_main()` add them explicitly): fixed = its value; hug = measured
    /// content. (Expand ⇒ 0 here; the grant comes from the leftover pool.)
    fn natural_main(&self, m: &mut dyn Measurer, vertical: bool) -> f32 {
        match self.main {
            Sizing::Fixed(v) => v,
            Sizing::Expand(_) => 0.0,
            Sizing::Hug => match self.kind {
                Kind::Label => {
                    let (w, h) = m.measure(&self.text, self.text_size, self.ff, self.fw, self.icon);
                    if vertical {
                        h
                    } else {
                        w
                    }
                }
                Kind::Row | Kind::Col => self.hug_main(m, vertical),
                _ => 0.0,
            },
        }
    }

    /// The main-axis length this container-node hugs naturally (recursive for
    /// Row/Col): Σ child naturals + gaps + own padding.
    fn hug_main(&self, m: &mut dyn Measurer, vertical: bool) -> f32 {
        let pad = if vertical {
            self.pad.1 + self.pad.3
        } else {
            self.pad.0 + self.pad.2
        };
        match &self.kind {
            Kind::Row | Kind::Col => {
                let n = self.children.len();
                let sum: f32 = self
                    .children
                    .iter()
                    .map(|c| c.natural_main(m, vertical) + margin_main(c, vertical))
                    .sum();
                (sum + self.gap * (n.saturating_add(0).saturating_sub(1)) as f32 + pad).max(0.0)
            }
            _ => match self.main {
                Sizing::Fixed(v) => v,
                _ => 0.0,
            },
        }
    }

    // ── phase 3: render ─────────────────────────────────────────────────────

    /// Emit `Cmd`s + register hit regions. MUST run after `layout()`; does no
    /// geometry work (Render-phase rule) and mutates nothing.
    pub fn render(&self, v: &mut Vec<Cmd>, s: &mut dyn RegionSink, pal: &Pal) {
        assert_laid(self.laid);
        match &self.kind {
            Kind::Box => {
                if self.bg != 0 {
                    v.push(Cmd::Rect {
                        x: self.laid.x,
                        y: self.laid.y,
                        w: self.laid.w,
                        h: self.laid.h,
                        r: self.radius,
                        color: self.bg,
                    });
                }
                for c in &self.children {
                    c.render(v, s, pal);
                }
            }
            Kind::Row | Kind::Col => {
                for c in &self.children {
                    c.render(v, s, pal);
                }
            }
            Kind::Center { dx, dy } => {
                // renderer-anchor contract: x IS the centre, `center: true`
                // — identical Cmd bytes to the retired drawers' ui.heroc/textc
                let tx = self.laid.x + self.laid.w / 2.0 + dx;
                let ty = self.laid.y + self.laid.h / 2.0 + dy;
                v.push(Cmd::Text {
                    x: tx,
                    y: ty,
                    right: false,
                    center: true,
                    text: self.text.clone(),
                    size: self.text_size,
                    color: self.color,
                    icon: self.icon,
                    f: self.ff,
                    w: self.fw,
                });
            }
            Kind::Label => {
                // Horizontal alignment lives in the node; the vertical slot
                // centering matches the drawers' `text()` conventions (the
                // Cmd::Text y is the placed run origin, height `mh`). Right
                // alignment emits the renderer's `right: true` anchor at the
                // SLOT's right edge — the byte contract `ui::text_r` gives
                // the drawers (x IS the right edge of the run).
                let (tx, right) = match self.halign {
                    HAlign::Left => (self.laid.x, false),
                    HAlign::Center => (
                        self.laid.x + (self.laid.w - self.mw).max(0.0) / 2.0,
                        false,
                    ),
                    HAlign::Right => (self.laid.x + self.laid.w, true),
                };
                let ty = self.laid.y + valign_y(self.laid.h, self.mh, self.valign);
                v.push(Cmd::Text {
                    x: tx,
                    y: ty,
                    right,
                    center: false,
                    text: self.text.clone(),
                    size: self.text_size,
                    color: self.color,
                    icon: self.icon,
                    f: self.ff,
                    w: self.fw,
                });
            }
            Kind::Button => {
                let hov = self.key != 0 && s.hov(self.key);
                let fill = match (self.bg, hov) {
                    (0, false) => crate::ui::raised(pal),
                    (0, true) => crate::ui::raised_hl(pal),
                    (c, _) => c,
                };
                v.push(Cmd::Rect {
                    x: self.laid.x,
                    y: self.laid.y,
                    w: self.laid.w,
                    h: self.laid.h,
                    r: if self.radius < 0.0 {
                        self.laid.h / 2.0
                    } else {
                        self.radius
                    },
                    color: fill,
                });
                if !self.text.is_empty() {
                    let tcol = if self.color != 0 { self.color } else { pal.fg };
                    let tx = self.laid.x + (self.laid.w - self.mw).max(0.0) / 2.0;
                    let ty = self.laid.y + valign_y(self.laid.h, self.mh, VAlign::Center);
                    v.push(Cmd::Text {
                        x: tx,
                        y: ty,
                        right: false,
                        center: false,
                        text: self.text.clone(),
                        size: self.text_size,
                        color: tcol,
                        icon: self.icon,
                        f: self.ff,
                        w: self.fw,
                    });
                }
                if self.key != 0 {
                    s.region(self.laid.x, self.laid.y, self.laid.w, self.laid.h, self.key);
                }
            }
            Kind::Toggle => {
                let hov = self.key != 0 && s.hov(self.key);
                let track = if self.on {
                    pal.acc
                } else {
                    if hov {
                        crate::ui::hover_hl(pal)
                    } else {
                        crate::ui::hover(pal)
                    }
                };
                v.push(Cmd::Rect {
                    x: self.laid.x,
                    y: self.laid.y,
                    w: self.laid.w,
                    h: self.laid.h,
                    r: self.laid.h / 2.0,
                    color: track,
                });
                let inset = 3.0;
                let kr = ((self.laid.h - 2.0 * inset) / 2.0).max(1.0);
                let kx = if self.on {
                    self.laid.x + self.laid.w - inset - 2.0 * kr
                } else {
                    self.laid.x + inset
                };
                v.push(Cmd::Rect {
                    x: kx,
                    y: self.laid.y + inset,
                    w: 2.0 * kr,
                    h: 2.0 * kr,
                    r: kr,
                    color: pal.fg,
                });
                if self.key != 0 {
                    s.region(self.laid.x, self.laid.y, self.laid.w, self.laid.h, self.key);
                }
            }
            Kind::Slider => {
                let hov = self.key != 0 && s.hov(self.key);
                crate::ui::slider(
                    v,
                    self.laid.x,
                    self.laid.y,
                    self.laid.w,
                    self.laid.h,
                    self.val,
                    hov,
                    pal,
                );
                if self.key != 0 {
                    s.region(self.laid.x, self.laid.y, self.laid.w, self.laid.h, self.key);
                }
            }
            Kind::Graph => {
                let color = if self.color != 0 { self.color } else { pal.acc };
                match self.gmode {
                    GMode::Bars => {
                        let n = self.series.len();
                        if n > 0 && self.laid.w > 0.0 {
                            let pitch = self.laid.w / n as f32;
                            let bw = pitch * 0.7;
                            for (i, &val) in self.series.iter().enumerate() {
                                let val = val.clamp(0.0, 1.0);
                                let bh = val * self.laid.h;
                                v.push(Cmd::Rect {
                                    x: self.laid.x + pitch * i as f32 + (pitch - bw) / 2.0,
                                    y: self.laid.y + self.laid.h - bh,
                                    w: bw,
                                    h: bh,
                                    r: bw / 2.0,
                                    color,
                                });
                            }
                        }
                    }
                    GMode::Line => {
                        let n = self.series.len();
                        if n > 1 && self.laid.w > 0.0 {
                            let span = self.laid.w / (n - 1) as f32;
                            for i in 0..n - 1 {
                                let a = self.series[i].clamp(0.0, 1.0);
                                let b = self.series[i + 1].clamp(0.0, 1.0);
                                v.push(Cmd::Line {
                                    x0: self.laid.x + span * i as f32,
                                    y0: self.laid.y + self.laid.h * (1.0 - a),
                                    x1: self.laid.x + span * (i + 1) as f32,
                                    y1: self.laid.y + self.laid.h * (1.0 - b),
                                    w: 2.0,
                                    color,
                                });
                            }
                        }
                    }
                }
            }
            Kind::Beads { dx, dy } => {
                let cx = self.laid.x + self.laid.w / 2.0 + dx;
                let cy = self.laid.y + self.laid.h / 2.0 + dy;
                let n = self.beads.max(2);
                let lit = self.beads_lit.min(n);
                let dot = self.beads_dot;
                for b in 0..n {
                    let ang = (b as f32 / n as f32) * std::f32::consts::TAU
                        - std::f32::consts::FRAC_PI_2;
                    v.push(Cmd::Rect {
                        x: cx + ang.cos() * self.beads_r - dot / 2.0,
                        y: cy + ang.sin() * self.beads_r - dot / 2.0,
                        w: dot,
                        h: dot,
                        r: dot / 2.0,
                        color: if b < lit { self.color } else { self.bg },
                    });
                }
            }
            Kind::Bar { vertical } => {
                let (bx, by, bw, bh) = (self.laid.x, self.laid.y, self.laid.w, self.laid.h);
                v.push(Cmd::Rect {
                    x: bx,
                    y: by,
                    w: bw,
                    h: bh,
                    r: self.radius,
                    color: self.bg,
                });
                let max = if self.bar_max > 0.0 { self.bar_max } else { 100.0 };
                let frac = (self.bar_raw as f64 / max as f64).clamp(0.0, 1.0);
                if frac > 0.001 {
                    let (fx, fy, fw, fh) = if *vertical {
                        (
                            bx,
                            by + bh * (1.0 - frac as f32),
                            bw,
                            (bh as f64 * frac) as f32,
                        )
                    } else {
                        (bx, by, (bw as f64 * frac) as f32, bh)
                    };
                    v.push(Cmd::Rect {
                        x: fx,
                        y: fy,
                        w: fw,
                        h: fh,
                        r: self.radius,
                        color: self.color,
                    });
                }
            }
            Kind::Stamp => {
                for c in &self.stamp {
                    v.push(shift_cmd(c, self.laid.x, self.laid.y));
                }
            }
            Kind::Image => {
                v.push(Cmd::Image {
                    x: self.laid.x,
                    y: self.laid.y,
                    w: self.laid.w,
                    h: self.laid.h,
                    key: self.imkey.clone(),
                });
            }
            Kind::Hit => {
                if self.key != 0 {
                    s.region(self.laid.x, self.laid.y, self.laid.w, self.laid.h, self.key);
                }
            }
            Kind::Scroll { .. } => {
                if self.laid.w > 0.0 && self.laid.h > 0.0 {
                    v.push(Cmd::Scissor {
                        x: self.laid.x,
                        y: self.laid.y,
                        w: self.laid.w,
                        h: self.laid.h,
                    });
                }
                for c in &self.children {
                    c.render(v, s, pal);
                }
                if self.laid.w > 0.0 && self.laid.h > 0.0 {
                    v.push(Cmd::ScissorEnd);
                }
            }
        }
    }

    /// The rect this node was granted after `layout()` — for side-effect
    /// plumbing (a card remembering its slider rect for drag math).
    pub fn rect(&self) -> Rect {
        self.laid
    }
}

// ── small layout helpers ────────────────────────────────────────────────────

fn main_len(r: Rect, vertical: bool) -> f32 {
    if vertical {
        r.h
    } else {
        r.w
    }
}

fn main_coord(r: Rect, vertical: bool) -> f32 {
    if vertical {
        r.y
    } else {
        r.x
    }
}

/// a child's main-axis margin sum (its own share of the walk)
fn margin_main(n: &Node, vertical: bool) -> f32 {
    if vertical {
        n.margin.1 + n.margin.3
    } else {
        n.margin.0 + n.margin.2
    }
}

/// translate a stamped command by (dx, dy) — the `Stamp` widget's positioning
fn shift_cmd(c: &Cmd, dx: f32, dy: f32) -> Cmd {
    match c {
        Cmd::Rect { x, y, w, h, r, color } => Cmd::Rect {
            x: x + dx,
            y: y + dy,
            w: *w,
            h: *h,
            r: *r,
            color: *color,
        },
        Cmd::RectConcave {
            x,
            y,
            w,
            h,
            r_tl,
            r_tr,
            r_br,
            r_bl,
            color,
        } => Cmd::RectConcave {
            x: x + dx,
            y: y + dy,
            w: *w,
            h: *h,
            r_tl: *r_tl,
            r_tr: *r_tr,
            r_br: *r_br,
            r_bl: *r_bl,
            color: *color,
        },
        Cmd::Outline {
            x,
            y,
            w,
            h,
            r,
            width,
            color,
        } => Cmd::Outline {
            x: x + dx,
            y: y + dy,
            w: *w,
            h: *h,
            r: *r,
            width: *width,
            color: *color,
        },
        Cmd::Line {
            x0,
            y0,
            x1,
            y1,
            w,
            color,
        } => Cmd::Line {
            x0: x0 + dx,
            y0: y0 + dy,
            x1: x1 + dx,
            y1: y1 + dy,
            w: *w,
            color: *color,
        },
        Cmd::Text {
            x,
            y,
            right,
            center,
            text,
            size,
            color,
            icon,
            f,
            w,
        } => Cmd::Text {
            x: x + dx,
            y: y + dy,
            right: *right,
            center: *center,
            text: text.clone(),
            size: *size,
            color: *color,
            icon: *icon,
            f: *f,
            w: *w,
        },
        Cmd::Image { x, y, w, h, key } => Cmd::Image {
            x: x + dx,
            y: y + dy,
            w: *w,
            h: *h,
            key: key.clone(),
        },
        Cmd::Map {
            x,
            y,
            w,
            h,
            lon0,
            lat0,
            lon1,
            lat1,
            zoom,
            style,
            colors,
            region,
            version,
        } => Cmd::Map {
            x: x + dx,
            y: y + dy,
            w: *w,
            h: *h,
            lon0: *lon0,
            lat0: *lat0,
            lon1: *lon1,
            lat1: *lat1,
            zoom: *zoom,
            style: *style,
            colors: *colors,
            region: *region,
            version: *version,
        },
        Cmd::Scissor { x, y, w, h } => Cmd::Scissor {
            x: x + dx,
            y: y + dy,
            w: *w,
            h: *h,
        },
        Cmd::ScissorEnd => Cmd::ScissorEnd,
    }
}

fn valign_y(slot_h: f32, content_h: f32, v: VAlign) -> f32 {
    match v {
        VAlign::Top => 0.0,
        VAlign::Bottom => (slot_h - content_h).max(0.0),
        VAlign::Center => (slot_h - content_h).max(0.0) / 2.0,
    }
}

fn assert_laid(r: Rect) {
    // A tree laid out by the shell always has a positive grant (cards are
    // never granted zero); an all-zero rect means render() ran pre-layout.
    debug_assert!(r.w != 0.0 || r.h != 0.0, "render() before layout()");
}

/// Sink for trees with NO interactive widgets — `region()` records nothing.
/// The shell-facing sink adapter (real hit registration through the existing
/// `region()` pipeline) lands with the first interactive card port
/// (WIDGETS.md stage 4).
pub struct NoHit;
impl RegionSink for NoHit {
    fn region(&mut self, _x: f32, _y: f32, _w: f32, _h: f32, _key: u32) {}
    fn hov(&self, _key: u32) -> bool {
        false
    }
}

/// Measurer for trees that contain no measured runs (anchored-center text
/// only). Use the shell's real measurer the moment a card needs width-based
/// flow.
pub struct NoMeasurer;
impl Measurer for NoMeasurer {
    fn measure(
        &mut self,
        _text: &str,
        _size: f32,
        _f: crate::text::Ff,
        _w: crate::text::Fw,
        _icon: bool,
    ) -> (f32, f32) {
        (0.0, 0.0)
    }
}

/// Shell-facing convenience for pure-anchor trees: lay out over the card
/// rect (they need no metric input) and render. Interactive cards go through
/// `layout()` + `render()` themselves once shell wiring lands.
pub fn paint_anchored(
    tree: Node,
    v: &mut Vec<Cmd>,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    pal: &Pal,
) {
    let tree = tree.layout(Rect { x, y, w, h }, &mut NoMeasurer);
    tree.render(v, &mut NoHit, pal);
}

// ── tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::{Ff, Fw};

    /// Deterministic stub: width = 5 px/char, height = 10 px.
    struct M;
    impl Measurer for M {
        fn measure(&mut self, t: &str, _s: f32, _f: Ff, _w: Fw, _icon: bool) -> (f32, f32) {
            ((t.chars().count() as f32) * 5.0, 10.0)
        }
    }

    struct Sink {
        regions: Vec<(f32, f32, f32, f32, u32)>,
        hover: u32,
    }
    impl Sink {
        fn new() -> Sink {
            Sink { regions: Vec::new(), hover: 0 }
        }
    }
    impl RegionSink for Sink {
        fn region(&mut self, x: f32, y: f32, w: f32, h: f32, key: u32) {
            self.regions.push((x, y, w, h, key));
        }
        fn hov(&self, key: u32) -> bool {
            self.hover == key && key != 0
        }
    }

    fn pal() -> Pal {
        Pal {
            fg: 0xececf000 | 0xff,
            bg: 0x14141a00 | 0xff,
            acc: 0x7d5cff00 | 0xff,
            sfg: 0xffffff00 | 0xff,
        }
    }

    fn r(x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect { x, y, w, h }
    }

    #[test]
    fn row_distributes_fixed_and_expand() {
        // [box 100px][gap 10][label expand]
        let tree = Node::row()
            .child(Node::box_(4.0, 0x112233ff).fixed(100.0))
            .child(Node::label("abcd", 16.0, 0xff).expand())
            .gap(10.0);
        let tree = tree.layout(r(0.0, 0.0, 300.0, 50.0), &mut M);
        assert!(tree.rect().w == 300.0);
        // children: box at x=0 w=100; label at x=110 w=190
        let kids = &tree.children;
        assert_eq!(kids[0].rect(), r(0.0, 0.0, 100.0, 50.0));
        assert_eq!(kids[1].rect(), r(110.0, 0.0, 190.0, 50.0));
    }

    #[test]
    fn col_stacks_with_gap_and_hug_height() {
        // labels are Hug by default → 10px tall each (stub metrics)
        let tree = Node::col()
            .child(Node::label("abcd", 16.0, 0xff))
            .child(Node::label("ab", 16.0, 0xff))
            .gap(8.0);
        let tree = tree.layout(r(0.0, 0.0, 200.0, 100.0), &mut M);
        let kids = &tree.children;
        assert_eq!(kids[0].rect(), r(0.0, 0.0, 200.0, 10.0));
        assert_eq!(kids[1].rect(), r(0.0, 18.0, 200.0, 10.0));
    }

    #[test]
    fn label_centering_and_valign() {
        let tree = Node::label("abcdefgh", 16.0, 0x010203ff)
            .center_x()
            .expand();
        let tree = tree.layout(r(0.0, 0.0, 100.0, 60.0), &mut M);
        let mut v = Vec::new();
        let mut sink = Sink::new();
        tree.render(&mut v, &mut sink, &pal());
        // text = 8 chars * 5 = 40 wide, 10 tall → x=30, y=(60-10)/2=25
        match &v[0] {
            Cmd::Text { x, y, .. } => {
                assert_eq!(*x, 30.0);
                assert_eq!(*y, 25.0);
            }
            other => panic!("expected Text, got {other:?}"),
        }
    }

    #[test]
    fn button_registers_region_and_hover_fill() {
        let p = pal();
        let tree = Node::button("Ok", 14.0, 42).layout(r(0.0, 0.0, 120.0, 32.0), &mut M);
        // not hovered → raised()
        let mut v = Vec::new();
        tree.render(&mut v, &mut Sink::new(), &p);
        let fill = match &v[0] {
            Cmd::Rect { color, .. } => *color,
            other => panic!("expected Rect, got {other:?}"),
        };
        assert_eq!(fill, crate::ui::raised(&p));
        // hovered → raised_hl()
        let mut hv = Sink::new();
        hv.hover = 42;
        let mut v = Vec::new();
        tree.render(&mut v, &mut hv, &p);
        let fill = match &v[0] {
            Cmd::Rect { color, .. } => *color,
            other => panic!("expected Rect, got {other:?}"),
        };
        assert_eq!(fill, crate::ui::raised_hl(&p));
        // region registered in both worlds
        assert_eq!(hv.regions, vec![(0.0, 0.0, 120.0, 32.0, 42)]);
    }

    #[test]
    fn toggle_knob_moves_and_fills_accent_when_on() {
        let p = pal();
        let tree = Node::toggle(true, 7, 52.0)
            .fixed(52.0)
            .layout(r(0.0, 0.0, 52.0, 24.0), &mut M);
        let mut v = Vec::new();
        tree.render(&mut v, &mut Sink::new(), &p);
        let track = match &v[0] {
            Cmd::Rect { color, r, .. } => {
                assert_eq!(*r, 12.0); // capsule = h/2
                *color
            }
            other => panic!("expected Rect, got {other:?}"),
        };
        assert_eq!(track, p.acc);
        let knob = match &v[1] {
            Cmd::Rect { x, .. } => *x,
            other => panic!("expected Rect, got {other:?}"),
        };
        // inset 3, kr = (24-6)/2 = 9 → kx = 52-3-18 = 31
        assert_eq!(knob, 31.0);
        // off: track is hover(pal), knob at x=3
        let tree = Node::toggle(false, 7, 52.0)
            .fixed(52.0)
            .layout(r(0.0, 0.0, 52.0, 24.0), &mut M);
        let mut v = Vec::new();
        tree.render(&mut v, &mut Sink::new(), &p);
        let (track, knob) = match (&v[0], &v[1]) {
            (Cmd::Rect { color, .. }, Cmd::Rect { x, .. }) => (*color, *x),
            _ => panic!("expected two Rects"),
        };
        assert_eq!(track, crate::ui::hover(&p));
        assert_eq!(knob, 3.0);
    }

    #[test]
    fn graph_bars_and_line_emit_expected_shapes() {
        let p = pal();
        let series = vec![0.2, 0.5, 0.8];
        // Bars: 3 rounded rects
        let tree = Node::graph(series.clone(), GMode::Bars, 0).layout(r(0.0, 0.0, 300.0, 40.0), &mut M);
        let mut v = Vec::new();
        tree.render(&mut v, &mut Sink::new(), &p);
        let rects: Vec<(f32, f32, f32, f32)> = v
            .iter()
            .filter_map(|c| match c {
                Cmd::Rect { x, y, w, h, .. } => Some((*x, *y, *w, *h)),
                _ => None,
            })
            .collect();
        assert_eq!(rects.len(), 3);
        // pitch 100, bar w 70: first bar x=15, y = 40-8 (0.2*40)
        assert_eq!(rects[0], (15.0, 32.0, 70.0, 8.0));
        assert_eq!(rects[1], (115.0, 20.0, 70.0, 20.0));
        assert_eq!(rects[2], (215.0, 8.0, 70.0, 32.0));
        // Line: 2 capsules
        let tree = Node::graph(series, GMode::Line, 0).layout(r(0.0, 0.0, 300.0, 40.0), &mut M);
        let mut v = Vec::new();
        tree.render(&mut v, &mut Sink::new(), &p);
        let lines = v
            .iter()
            .filter(|c| matches!(c, Cmd::Line { .. }))
            .count();
        assert_eq!(lines, 2);
    }

    #[test]
    fn pad_insets_the_content_box() {
        let tree = Node::box_(4.0, 0x112233ff)
            .pad(10.0, 12.0, 10.0, 12.0)
            .child(Node::label("x", 16.0, 0xff).expand());
        let tree = tree.layout(r(0.0, 0.0, 100.0, 60.0), &mut M);
        assert_eq!(tree.children[0].rect(), r(10.0, 12.0, 80.0, 36.0));
    }

    #[test]
    fn scroll_clips_and_offsets_child() {
        let list = Node::col()
            .child(Node::label("a".repeat(20), 16.0, 0xff))
            .child(Node::label("b".repeat(20), 16.0, 0xff));
        let tree = Node::scroll(list, (0.0, 10.0));
        let tree = tree.layout(r(0.0, 0.0, 100.0, 30.0), &mut M);
        let mut v = Vec::new();
        tree.render(&mut v, &mut Sink::new(), &pal());
        // scissor open, two texts, scissor close
        assert!(matches!(v[0], Cmd::Scissor { .. }));
        assert!(matches!(v[v.len() - 1], Cmd::ScissorEnd));
        // child was laid 10px above the clip
        let kids = &tree.children;
        assert_eq!(kids[0].rect().y, -10.0);
    }

    #[test]
    fn slider_region_and_val_clamped() {
        let tree = Node::slider(2.0, 9).fixed(100.0);
        let tree = tree.layout(r(0.0, 0.0, 100.0, 20.0), &mut M);
        // val clamped to 1.0 — render should not panic and region registers
        let mut s = Sink::new();
        let mut v = Vec::new();
        tree.render(&mut v, &mut s, &pal());
        assert_eq!(s.regions.len(), 1);
        let tree = Node::slider(0.0, 9).fixed(100.0);
        // fixed() on slider: slot width honored
        assert_eq!(tree.layout(r(0.0, 0.0, 100.0, 20.0), &mut M).rect(), r(0.0, 0.0, 100.0, 20.0));
    }

    #[test]
    fn margins_shift_children_and_count_in_fixed_sizing() {
        // [m20 | box 50 | m10] [expand fills the rest]: second child x=80 w=120
        let tree = Node::row()
            .child(Node::box_(0.0, 0xff0000ff).fixed(50.0).margin(20.0, 0.0, 10.0, 0.0))
            .child(Node::box_(0.0, 0x00ff00ff).expand());
        let tree = tree.layout(r(0.0, 0.0, 200.0, 30.0), &mut M);
        let kids = &tree.children;
        assert_eq!(kids[0].rect(), r(20.0, 0.0, 50.0, 30.0));
        assert_eq!(kids[1].rect(), r(80.0, 0.0, 120.0, 30.0));
    }
}
