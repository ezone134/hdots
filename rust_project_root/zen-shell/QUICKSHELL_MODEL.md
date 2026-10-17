# zen-shell — the declarative model, in Lua

> **What this file is:** the architecture and the ordered work to reach it — a
> design doc, not a session log. For what landed read `CHANGELOG.md`; for open
> items and the build recipe read `NEXT_STEPS.md`.
>
> **The goal in one line:** the ergonomics of Quickshell — declare objects, write
> bindings, instantiate delegates over models, override component properties —
> expressed in **`.lua` modules instead of QML**, on a shell that is event-driven
> and draws through one command list.

---

## 0. The model: like Quickshell, but Lua instead of QML

Quickshell declares its *entire* user interface in QML over a C++/Qt runtime.
zen-shell does the same on a different substrate:

> **Declare the whole UI in Lua. Rust is the backend and the Wayland renderer.**

Nothing decides a pixel in Rust. Every card and every surface — what to show,
where it goes, how big it is, which state it is in, every transition — is a
`.lua` module under `~/.config/zen-shell/ui/`. Rust only turns bytes into values
and turns the one `Cmd` list into Wayland buffers.

The shell binary is Rust: it owns the Wayland/EGL connection, the event loop,
IPC, the data providers, and it **hosts a LuaJIT VM** (`mlua`, vendored) that
loads and runs the modules. There is no separate Lua runtime — Lua is this
shell's *declarative language*, exactly as QML is Quickshell's.

| Lives in Rust (backend + renderer) | Lives in Lua (every decision) |
|---|---|
| sysfs / procfs / netlink / D-Bus reads | which data to show |
| spawning commands, IPC, the compositor connection | where it goes and how big |
| text shaping, the `Cmd` list → Wayland buffers | colors, radii, glyphs, hover fills |
| decoding a byte stream into numbers (PNG, FFT frame) | **all state**, and every transition between states |

**The line is not how much code sits in Rust — it is who decides a coordinate.**
Rust publishes *values*; Lua decides what they mean.

### The invariant, stated as a rule

**Rust never emits geometry.** Every `Cmd::Rect` / `Cmd::Text` / `Cmd::Image` in
the process originates from an item a module declared, never from a Rust function
that decided where to put it. A `layout_*` / `draw_*_card` drawer is a violation
because it computes a position — length is irrelevant.

The invariant is checkable; these are the metrics:

- **Zero scene DSLs.** There is exactly one way to declare UI: Lua. Nothing is
  authored in RON, TOML, or any other markup — the RON scene engine, its
  expression evaluator, and every `.ron` file are removed.
- **`ui/cards/*.ron` → 0.** All **72** cards are `.lua` modules under
  `~/.config/zen-shell/ui/cards/`. `DashCard::draw_rust` is an **all-empty
  match** — a migration record, not a painter.
- **`Ink()` count → 0.** `Ink(name)` was the escape hatch that handed a rectangle
  of the screen back to a Rust painter. No surface declares one any more.
- **No `layout_*` in the draw path.** A retired drawer survives only as a
  `#[cfg(test)]` parity oracle, never as a draw call. The useless ones are
  deleted outright.
- **A new `Ink` is a rejection.** Adding one means something could not be
  declared yet. Record *what* was missing instead.

### What "close to Quickshell" means here

**Same ergonomics, different substrate.** An author editing a module gets the
Quickshell experience: declare objects, bind properties, instantiate delegates
over models, override component defaults, keep state on the objects. Quickshell
gets that from QML + Qt; we get it from **Lua tables + this engine**.

Lua is a real language, so an author uses `functions`, `for`/`ipairs`, locals,
and `ctx.api` directly — closer to QML than any data format. The engine provides
the vocabulary (items, props, stores, delegates); Lua provides the syntax.

What is **not** being copied (§9 has the full list):

- **Qt is not the renderer.** We draw through one `Cmd` list into Wayland
  shm/vulkan buffers, not a scene graph the toolkit rasterizes.
- **The render loop is not copied.** Quickshell runs bindings continuously; we
  are event-driven and idle at 0% CPU, so dirty tracking is *required* here, not
  optional.
- **The object model is not QML's.** Lua has no property system, so bindings and
  delegates are the engine's own vocabulary (a string expression in a field, a
  model + templates), not language keywords.

What **is** copied is the good part: implicit property inheritance, bindings over
assignments, models + delegates, components with overridable defaults, state that
lives next to the UI that reads it, and animations declared on the field rather
than hand-rolled per widget.

**No "the algorithm is too complex" exemption.** Fluid drag is not too complex to
declare — it is *state plus a transition*. Undeclarable behavior is a gap in the
engine to close, not a license for the renderer to keep deciding. Only two things
stay in Rust: producing numbers from bytes, and turning a `Cmd` list into Wayland
buffers.

---

## 1. The five Quickshell rules, in Lua

Everything else in this file is a consequence of these.

| # | Quickshell rule | In zen-shell |
|---|---|---|
| 1 | **You declare objects; the tree IS the thing.** No imperative "build a node then configure it". | A module hands the engine a **Lua table item tree** (`ctx.ui.scene{ items = { ... } }`). |
| 2 | **Property inheritance is implicit.** A child reads the parent's box unless it overrides it (`w: parent.w * 0.5`). | `parent.w` / `.h` / `.cx` / `.cy` resolve against the box an item is already drawn in. |
| 3 | **Bindings, not assignments.** Write `w = "card_w - 28"`; it re-evaluates itself when anything it reads changes. | A string field is an **expression** resolved each frame against the value pool; a number is a literal. |
| 4 | **Models + delegates.** `Repeater { model: x; delegate: Y {} }` instantiates `Y` per row with `index` / the row in scope. | `Repeat { model = "...", templates = { ... } }`; a delegate binds `item.<field>` / `index`. The row's `tpl` picks the template. |
| 5 | **Components with overridable defaults; state on the objects.** | `components` with `props` defaults, instantiated by `Comp`; scene-owned `store`s for state. |

### Where the tree is today

Cards already paint from Lua, in two shapes:

- **Primitive calls** — a module walks `ctx.api.*` and paints through the
  component library (`ctx.ui.textc`, `ctx.ui.surface`, `ctx.ui.fader`, …). This is
  the `clock.lua` / `audiorec.lua` shape.
- **Declared tree** — a module hands `ctx.ui.scene{ ... }` a Lua item tree and the
  shared renderer walks it. This is the `workspaces.lua` / `sliders.lua` shape.

Both are Lua. The engine is the same walker in both cases.

---

## 2. The building blocks a module paints with

A module is a `.lua` file under `~/.config/zen-shell/ui/cards/<id>.lua`. It
defines **`draw(ctx)`**, called once per frame.

`ctx` carries:

| field | meaning |
|---|---|
| `ctx.x / y / w / h` | the card's box, in **device px** |
| `ctx.s(n)` | scale a base px by the ui scale |
| `ctx.fs(n)` | scale a base font size (rounded, matching the engine's `fs()`) |
| `ctx.hover` | the live hover key (0 = none) |
| `ctx.pal.*` | theme tokens: `fg`, `fg2`, `fg3`, `bg`, `acc`, `sfg`, `hover`, `hover_hl`, `raised`, `hairline`, … |
| `ctx.api.*` | the frame's backend values, published for this card by Rust |
| `ctx.ui.*` | the component library (below) |

`ctx.ui` exposes the same primitives the engine draws with, so parity is
structural:

- **shapes** — `surface(x,y,w,h,r,color)`, `outline(...)`, `line(x0,y0,x1,y1,w,color)`
- **text** — `text` / `textr` / `textc`, `hero` / `heroc` / `heror`, `title` /
  `titlec`, `caption` / `captionr` / `captionc`, `mono` / `monor` / `monoc`,
  `brandc`
- **vector primitives** — `bar(...)`, `fader(...)`, `spark(...)`, `ring(...)`,
  `battery(...)`, `moon(...)`, `map{...}`
- **the shared renderer** — `scene(t)`: walk a declared Lua item tree.

```lua
-- clock.lua — the whole Clock card
function draw(ctx)
  local s, fs, ui, pal = ctx.s, ctx.fs, ctx.ui, ctx.pal
  local x, y, w, h = ctx.x, ctx.y, ctx.w, ctx.h
  if ctx.api.clock then
    ui.heroc(x + w / 2, y + h / 2 - s(4), ctx.api.clock, fs(30.0), pal.fg, false)
  end
  if ctx.api.date then
    ui.textc(x + w / 2, y + h / 2 - s(22), ctx.api.date, fs(10.0), pal.fg2, false)
  end
end
```

---

## 3. Declaring a tree

Rule 1 is "the tree IS the thing". The engine renders a Lua item tree; the module
decides it:

```lua
function draw(ctx)
  ctx.ui.scene({
    items = {
      Header { title = "Workspaces", glyph = "\u{e9b0}", meta = "{ws_meta}", pad = 12 },
      Grid   { name = "ws_tiles", x = 12, y = 30, w = 0, h = -6,
               gap = 6, cols = 1, cell_max_w = 48, cell_max_h = 40,
               center = true, key_base = 30800, dot_size = 3, dot_dy = 6 },
    },
  })
end
```

The coordinates are **base px** and are multiplied by the scale on the way out. A
negative `w` / `h` measures back from the item's own right / bottom edge; `w = 0`
fills the parent; `right` / `center` anchor against the resolved box.

### The item vocabulary

| category | items |
|---|---|
| chrome | `Header`, `Composer`, `Rows`, `Dots` |
| layout | `Stack`, `Column`, `Row`, `Grid`, `Repeat` |
| shapes | `Surface`, `Outline`, `Hit` |
| text | `Text`, `TextWrap` |
| gauges | `Fader`, `Bar`, `Ring`, `Spark`, `Battery`, `Toggle` |
| special | `Card` (runs a dashboard card at a declared box), `Comp` (a component) |

---

## 4. Models and delegates (Rule 4)

A model carries **rows of fields**, not parallel families of pool names. A
delegate binds the row as `item` and its position as `index`:

```lua
Repeat {
  model = "pill_ctr",
  templates = {
    chip = {
      Surface { x = 0, y = 0, w = "item.w", h = 24, color = "item.ink" },
      Hit     { key = "item.key", x = 0, y = 0, w = "item.w", h = 0 },
    },
  },
}
```

The publisher writes rows:

```
pill_ctr = { { tag="wifi", tpl="chip", key=60, ink=fg, w=16 },
             { tag="volume", tpl="chip", key=70, ink=fg, w=22 } }
```

- **Dispatch is `tpl`, then `tag`, then `*`.** A row names its body's template
  (`tpl`) and keeps its own identity (`tag`); a `*` template is the anonymous
  default delegate. This one deviation lets one repeater hold rows of several
  *kinds* — the pill's center widgets share three bodies across twelve rows.
- **`dir`** — `across` (default, content-measured slots), `down` (a list: each
  slot is the repeat's inner box, stacked at `spacing`), `free` (a canvas: each
  child reads `item.x` / `item.y`).
- **A model row is a SLOT.** One row instantiates one template. Publishing a
  4-column grid row as *one* row of four field-sets stacks four cells on top of
  each other — one cell per row.
- **An unmatched `tpl` draws nothing**, so coverage is a test, not a compile
  error.

---

## 5. Components with parameters (Rule 5a)

Declare a body once with prop defaults; instantiate many with overrides:

```lua
components = {
  chip = {
    props = { w = 20, ink = "fg", key = 0 },
    items = {
      Surface { w = "w", color = "ink" },
      Hit     { key = "key", w = "w" },
    },
  },
}
-- use site:
Comp { name = "chip", props = { w = "pill_ws_reg_w", key = 50 } }
```

- `props` is a declared map of `name: default`; a use site overrides a subset.
  Defaults are the point — one declaration covers every call site.
- Props merge into the item's **scope**, so a body reads them as ordinary names.
- **Substitution is symbolic**, not numeric: `w = "w - 4"` in a body with
  `w = "pill_w"` at the site becomes `"pill_w - 4"`. If it folded at load, the
  value would freeze the first frame's number into the tree.
- **A prop is not a pool value.** `ink = "acc"` means *the accent token*; a bare
  string in a `value(...)`/color slot means *the pool color with this name*.

---

## 6. Scene-owned state (Rule 5b)

UI state lives next to the UI that reads it. A surface declares `stores`; an item
writes one with a `Set` action:

```lua
surface {
  name = "dash",
  stores = { nav = { scroll = 0, editing = false } },
  items = {
    Rows { name = "ws_tiles", start = "nav.scroll" },
    Hit  { key = 1, action = { set = { name = "nav.editing", value = true } } },
  },
}
```

- Stores are declared on the **surface**, not as an item (a store draws nothing).
- A store is a **closed set of names**, so both the write and the read are
  checked: a `Set` can only reach a prop its own surface declared, and every draw
  path merges only that surface's stores.
- A write is REFUSED on a type mismatch, never coerced.

---

## 7. The Rust side: exactly what stays, and what goes

Backend and renderer stay; the scene machinery goes.

**Stays** (`src/`):

- **The host + renderer** — `main.rs`, `app/`, `render.rs` / `render_vk.rs`
  (`Cmd` → buffers), `text.rs` (shaping), `ui.rs` (the shared component recipes
  behind `ctx.ui.*`), `scene.rs` (the Lua item-tree walker), `lua/mod.rs` (the
  LuaJIT host).
- **The backend data providers** — `backends.rs`, `hypr.rs`, `ipc.rs`, `tray.rs`,
  `sensors.rs`, `stats.rs`, `services.rs`, `weather.rs`, `news.rs`, `lyrics.rs`,
  `cliphist.rs`, `clipboard.rs`, `polkit.rs`, `auth.rs`, `mods.rs`, `themes.rs`,
  `config.rs`, `vars.rs`, `img.rs`, `eq.rs`, `viz.rs`, and the state/IC.

**Goes** (dead once the Lua model is complete):

- the **RON scene engine** — the RON parse path (`parse_scene_ron`, the `ron`
  dep). **`scene/expr.rs` does NOT go:** Lua modules author geometry as string
  expressions (`x = "parent.w - 54.0"`) that deserialize to `Val::Expr` and
  evaluate through `expr::compile`/`eval`/`fold`/`subst`, so it is live Lua-host
  code. (The runtime RON engine was removed 2026-10-13; only the test fixtures
  still parse RON.)
- the **all-empty `DashCard::draw_rust`** dispatch.
- the remaining **`#[cfg(test)]` oracle drawers** — once their parity tests are
  retired, the files and tests go together.
- any **`Ink` / `layout_*` / `draw_*_card`** survivor that is not a live backend
  path.

**Already deleted** (2026-10-12 all-Lua push — `cargo check` + `cargo test` green):

- `ui::bar`, `ui::fader_bal`; the notif + wallpaper Rust oracle drawers and their
  parity tests; `src/bin/font_diag.rs`; `ICON_SLIDER`.
- the **`layout_collapsed` Rust body** (~493 lines) — the pill now draws from the
  wired `pill` Lua surface; the wrapper keeps only the key-1 expand zone,
  brand-glyph refresh, and drag ghost. Its parity test + `pill_shell` helper went
  with it. The connectivity popover parity test
  (`the_connectivity_popover_scene_matches_its_own_drawer`) remains for now as it
  pins the `layout_banner` fallback.

**Final shape:** the *Stays* list collapses to a small core — `main.rs`, `wl.rs`
(from `app/`), `render.rs` (+ `render_vk.rs`), `text.rs`, `lua.rs` (from
`lua/mod.rs`), and OS access behind `ctx.api`. `scene.rs` / `ui.rs` fold into
`lua.rs` / `render.rs` last.

---

## 8. Migration status & plan

Where each milestone stands:

| milestone | state |
|---|---|
| Cards declared in Lua | **done** — 72/72; `DashCard::draw_rust` all-empty |
| Shell surfaces declared (zero `Ink`) | **done** |
| Dead code, oracle drawers, parity tests, `font_diag` bin | **done** (2026-10-12) |
| **Lua-native scene tables** (`ctx.ui.scene{...}`) | **in progress** |
| Wire `pill.lua`; retire `layout_collapsed` (+ drag ghost → Lua) | **done** (2026-10-12) |
| Retire `layout_banner` (dashboard edit chrome → Lua) | next (fallback parity test `the_connectivity_popover_scene_matches_its_own_drawer` pins it) |
| Port `shell.lua`/surfaces off RON; drop the RON fallback | planned |
| Delete the RON engine (`expr.rs` + parse path + `ron` dep) | after the above |
| Collapse modules to the core (`app/`→`wl.rs`, `scene`/`ui`→`lua`/`render`) | last |

The build order that got here — scope overlay → component props → store →
dirty tracking → `anim:` → `parent.*` → retire `Mode`/`Ink` → drop RON — is
recorded in §10; the hard-won rules are in §11. Both are format-agnostic and
apply unchanged to the Lua-native model.

---

## 9. What we are deliberately NOT copying

Note what is *not* on this list: Rust. §0 keeps the backend and the renderer in
Rust permanently, and nothing here proposes moving them out. "Declarative" is
about **who decides**, not about what language runs.

- **A free-running render loop.** The shell is event-driven and idles at 0% CPU;
  Quickshell's `always`-running bindings are why its dirty tracking is sound and
  ours must be too.
- **Qt's scene graph.** We draw one `Cmd` list into our own buffers.
- **A general UI toolkit.** Lua is the language, but the *engine vocabulary*
  (items, props, stores, delegates) is deliberate and validated at load.

---

## 10. Build order (historical)

The path that landed the declarative engine, kept as the migration record:

| Phase | Work | Deleted | Done when |
|---|---|---|---|
| **0** (2026-10-01) | Load-time validation | silent typos | 435/435 |
| **1** (2026-10-01) | Scope overlay + `item`/`index` in `Repeat`; pilot on the pill | `pill_key_<tag>` / `pill_ink_<tag>` families | 449/449 |
| **2** (2026-10-01) | Component `props` with defaults | the Rust `match self.mode` title table | 468/468 |
| **3** (demoted) | Dirty-set propagation + cached expressions | — | rolled into 4's validator half |
| **4** (2026-10-01) | `store` + `SceneAction::Set` | Rust state-only drawers | 489/489 |
| **5** (2026-10-02) | `anim:` on any field | hand-rolled settle/lerp | 508/508 |
| **6** (2026-10-02) | Implicit `parent` scope | positional box threading | 524/524 |
| **7** | Retire `Mode` + `Ink` | the last Rust draw paths | `Ink()` count 0; every `layout_*` a test fixture |
| **8** (next) | Lua-native scene tables; delete RON | the RON parser + `expr.rs` | no `.ron`, no `parse_scene_ron` |

---

## 11. Rules earned while building this

Format-agnostic lessons; they hold for the Lua-native model too.

- **A binding may ask what box it is in.** Any value a container computes for its
  children (`inner_w`, the center line) is derivable by a child that reads
  `parent.*`. When a tree restates a container's width inside the container, the
  fix is to name the box, not re-check the number.
- **A scope is a CLOSED set, so it is checkable where a pool is not.** `parent.*`,
  a store, and a delegate's `item.*` are all closed name sets, so a misspelled
  field is a load error.
- **Refuse, do not default, when a binding has no answer.** `parent.*` in a
  content-packed `Repeat` template has no measure-time value; 0 would draw a
  zero-width widget that looks like a layout bug rather than a schema refusal.
- **A measured width must never disagree with a drawn one.** When a value is read
  in two passes per frame, prove they agree.
- **A model row must carry fields, not tags.** A list of bare strings forces a
  parallel name per field per row, and every one is a silent-failure surface.
- **Conditional publication is a smell, not a feature.** A row that exists *is* a
  widget that is on.
- **The row picks the delegate; the repeater does not.** Letting the row name its
  template keeps one repeater per strip and lets identity (`tag`) and body (`tpl`)
  differ. The price: an unmatched `tpl` draws nothing, so coverage is a test.
- **A scope name must be checkable, or the scope is only a runtime feature.**
  Adding `item.w` bought nothing until `item.<field>` was validated against the
  model's rows.
- **The measure pass and the draw pass must instantiate identically.** One
  function produces the geometry both passes see.
- **A sweep is a test, not a habit.** New vocabulary gets a day-one walker.
- **A boundary you cannot count is not a boundary.** §0 names its metrics
  (`Ink()` → 0, no `layout_*` in the draw path).
- **Convert a drawer, then keep it as the test fixture.** A converted body as a
  `#[cfg(test)]` oracle is what makes correctness a *comparison* rather than a
  claim — until the test is retired and the file goes.
- **A gate and a swap are different verbs, and only having one is a trap.**
  `hover_key` alone is a gate (paint only while hovered); `hover_key` +
  `hover_color` is a swap (always painted, brighter on hover).
- **A primitive that can only express half of a common case does not get worked
  around — it gets completed.**
- **A component parameter is a name in a scope, not a field on the item.**
- **A load-time fold must not answer for a draw-time scope.** A scope answers
  only the names it owns and defers the rest; a name is "dead" only when the
  scope that should bind it has already run.
- **Convert a routed body one section at a time, and ratchet the seam.** When
  retiring a dispatcher one arm at a time, the invariant is "exactly one owner",
  and it wants its own test.
- **One box, one measure.** Layout that depends on a runtime width must not
  re-derive it; carry it down from the draw call.
- **A literal scales, a device constant does not.** A geometry number is base px
  and multiplied by the scale on the way out; a device constant is bound as a
  `*_var` the publisher divides by `scale.s(1.0)`.
- **A windowed model's keys are ABSOLUTE, and the click band must cover the whole
  collection.**
