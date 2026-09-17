# Handoff — Scene-Graph Declarative UI (zen-shell)

Date: 2026-09-15. Work in progress, continue next session.

## Goal

Make zen-shell's entire UI a **Quickshell/QML-like declarative RON scene graph**:
every dashboard card, settings panel, and control-center element defined under
`~/.config/zen-shell/ui/cards/<id>.ron` (and ultimately `ui/panels/`) as typed
scene items with semantic `ColorToken` colors, hot-reloaded without recompiling,
with all logic staying in pure Rust. Bars/meters, click actions, live text, tabs,
toggles, sliders, dynamic row lists — all driven from RON.

This session accomplished the **Header-chrome phase**: making the card title bar
(glyph + title + right-meta) declarative instead of hardcoded Rust for all 28
cards that use the shared `card_header` pattern.

---

## DONE — this session

### 1. `SceneItem::Header` variant added to scene engine
- `src/scene.rs:213` — `Header { title, glyph, meta, meta_mono, pad }`
- Draw arm at line 510: byte-for-byte mirror of `card_header` offsets/colors,
  uses `draw_text` so glyphs *and* metas get `{var}` substitution live.
- Honors `show_title`/`show_glyph` flags (matches `card_show_title`/`card_show_glyph` toggles).

### 2. `show_title`/`show_glyph` threaded through `CardScene::draw`
- Signature now: `CardScene::draw(&self, v, x, y, w, h, scale, pal, vals, hover_key, show_title: bool, show_glyph: bool)`
- All call sites updated: `panels/mod.rs:451` (draw_card_scene) + 6 test call sites.

### 3. `Shell::scene_owns_header` flag — opt-in Header ownership
- `src/shell/mod.rs:3134` — `pub(crate) scene_owns_header: bool` (init `false`, `state.rs:555`).
- `draw_card_scene` (`panels/mod.rs:446-459`): computed **before** `scene.draw()`, restored **after** `dispatch_scene_inks()`, so body painters see the flag during Ink dispatch.

### 4. `Shell::card_head` wrapper in `cards/shared.rs`
- `card_head(&self, v, pal, x, y, w, pad, title, icon, meta)` — **no-ops** when `scene_owns_header` is true, otherwise delegates to free `card_header(...)`.
- `battery_frame` now uses `self.card_head(...)`.
- 27 card files converted: `card_header(...)` → `self.card_head(...)` (sed, then manual fix of 7 mis-sed files).

### 5. 29 inline `ui::title` header sites guarded
- Pattern: `if !self.scene_owns_header && self.card_show_title { ui::title(...) }`
- Includes 4 multi-line sites: expenses, snippets, alarms, countdown.
- 13 now-unused `use super::shared::*;` imports removed.

### 6. `scene_values()` extended with 22 dynamic meta/glyph keys
Keys in `panels/mod.rs::scene_values()`:
- `disk_meta`, `cpu_meta` ("{cpu_freq_mhz} MHz"), `water_meta` ("x.x / y.y L"), `pomo_meta`, `speed_meta`, `wp_meta`, `jr_meta`, `sm_meta`, `sm_icon` (SHIELD/CHECK), `auddev_meta`, `conn_meta`, `moon_meta` (inline math, SYNODIC = 29.530_59), `comp_meta`, `fan_meta`, `er_meta`, `thermal_meta`, `wc_meta`, `mic_meta` ("{:>3}%"), `mic_icon` (MIC_OFF/MIC), `sus_meta`, `sus_icon` (CHECK/SHIELD), `ws_meta`.
- All field names verified against Shell struct (grep-matched).

### 7. 28 `Header(...)` + `Ink(...)` scenes written to `~/.config/zen-shell/ui/cards/`
accent, mem, network, gpu, branding, powerdraw, cpu, disk, water, pomodoro,
speedtest, latency, wallpaper, journaltail, smarthealth, audiodevice, conninfo,
moon, compositor, fans, eyerest, thermal, worldclock, micmeter, systemdunits,
workspaces, batteryh, batteryv.

### 8. Tests
- `committed_header_scenes_parse` — validates all 28 scene files parse + contain Header + Ink.
- `header_renders_title_glyph_and_meta_flagged` — unit test: title hidden when show_title=false, meta interpolates live values.
- **179/179 tests pass.** Build clean (only pre-existing input.rs:512 unreachable warning).
- Disk-full issue resolved: `cargo clean` freed 13G (was 100% full, 5.8G dev profile + 3.9G incremental).

---

## Architecture — how it works now (key contracts)

### Scene-first draw flow
```
DashCard::draw() → try draw_card_scene("card_id")
  ├─ scene loaded? → set scene_owns_header, scene.draw(), dispatch_scene_inks(), restore
  └─ scene missing? → draw_rust (full Rust fallback, unchanged)
```

When a scene has `Header`:
- Header draws the chrome (glyph + title + meta).
- Ink(name:"card_id") routes to draw_rust → card_head wrapper → no-op.
- **Zero double-draw.**

When a scene has no Header (or no scene at all):
- Rust `card_head` / `ui::title` draws chrome normally.

`audiorec` is the hybrid exception: scene draws its own Text chrome (not a Header item); Rust drawer gates `card_head` behind `if !scene_loaded`.

### Hot-reload
- `SceneCache::get` checks mtime → reloads RON on change.
- `vars::card_scene_path(id)` = `~/.config/zen-shell/ui/cards/<id>.ron` (resolves to `/acc/common/hdots/.config/zen-shell/ui/cards/`).

### RON gotchas
- Serde struct variants: `Text { ... }`, `Hit { ... }`.
- Optional fields need `#[serde(default)]`.
- `SceneAction`: must NOT use `#[serde(tag=...)]`; use `Command("...")`.
- RON string escapes: `"\u{f013}"`.

### Icons in scope
`shell/mod.rs:19 use crate::icons::*;` via `super::*`.
Codepoints: `ICON_SPARKLE \u{f004}`, `ICON_SETTINGS \u{f013}`, `ICON_CLOCK \u{f017}`,
`ICON_WALLPAPER \u{f03e}`, `ICON_MIC \u{f130}`, `ICON_MIC_OFF \u{f131}`,
`ICON_CHECK \u{f00c}`, `ICON_SHIELD \u{f132}`, `ICON_BATTERY \u{f240}`,
`ICON_SPEEDTEST \u{f554}`, `ICON_TIMER \u{f573}`, `ICON_VOLUME_FILL \u{e050}`,
`ICON_LINK \u{e250}`, `ICON_BRIGHTNESS_FILL \u{e430}`, `ICON_WIDGETS \u{e894}`,
`ICON_GRID \u{e9b0}`, `ICON_SPEED_FILL \u{e9e4}`.
Accent scene uses `\u{f1fc}` (STICKY), powerdraw `\u{f0e7}` (BOLT).

### Path hazard
Source dir: `/acc/common/hdots/rust_project_root/zen-shell/`
Config root: `/acc/common/hdots/.config/zen-shell/`
`/acc/common/hdocs` and `/acc/common/hdotvs` DON'T exist — edits fail if path is mistyped.

### Pre-existing warnings (do NOT fix, not yours)
- `src/shell/input.rs:512` — `TODO_KEY_ADD => unreachable` (intentional).
- `src/viz.rs:323` — unused `mut` (cfg(test) only).
- `src/shell/panels/cards/alarms.rs:70` — `pal()` unused helper (cfg(test) only).
- `src/shell/input.rs:3149` — `use super::*` unused import (cfg(test) only).

---

## Card head mapping (for reference — which cards draw what header chrome)

All converted to `self.card_head(...)` which is skipped when scene_owns_header:

| Card | Title | Glyph | Meta |
|------|-------|-------|------|
| accent | Theme & Accent | \u{f1fc} (STICKY) | — |
| mem | Memory | — | — |
| systemdunits | Services | {sus_icon} | {sus_meta} |
| workspaces | Workspaces | \u{e9b0} (GRID) | {ws_meta} |
| thermal | Thermal zones | {therbl_icon?} | {thermal_meta} |
| worldclock | World clock | \u{f017} (CLOCK) | {wc_meta} |
| branding | Brand | \u{f004} (SPARKLE) | — |
| powerdraw | Power | \u{f0e7} (BOLT) | — |
| micmeter | Mic | {mic_icon} | {mic_meta} mono |
| disk | Disk | — | {disk_meta} |
| pomodoro | Focus | \u{f573} (TIMER) | {pomo_meta} |
| water | Water | \u{e430} (BRIGHTNESS) | {water_meta} |
| speedtest | Speed test | \u{f554} (SPEEDTEST) | {speed_meta} |
| latency | Latency | \u{e9e4} (SPEED_FILL) | "1.1.1.1" mono |
| smarthealth | Disk health | {sm_icon} | {sm_meta} |
| wallpaper | Backgrounds | \u{f03e} (WALLPAPER) | {wp_meta} |
| journaltail | System log | \u{f2d0} | {jr_meta} |
| cpu | CPU | — | {cpu_meta} mono |
| audiodevice | Audio | \u{e050} (VOL) | {auddev_meta} |
| gpu | GPU | \u{f013} (SETTINGS) | — |
| network | Network | — | — |
| conninfo | Network | \u{e250} (LINK) | {conn_meta} |
| moon | Moon | \u{f186} | {moon_meta} |
| compositor | Effects | \u{e894} (WIDGETS) | {comp_meta} |
| fans | Fans | \u{e9e4} (SPEED_FILL) | {fan_meta} |
| eyerest | Eye rest | \u{f573} (TIMER) | {er_meta} |
| batteryh | Battery | \u{f240} | — |
| batteryv | Battery | \u{f240} | — |
| (battery_frame shared) | Battery | \u{f240} | — |

---

## NEXT — full declarative coverage (the "whole thing" plan)

### ✅ DONE — Batch 1 first tranche: list cards (landed 2026-09-15, 202/202 tests)
The `Rows` primitive grew the last four pieces list cards need, and the first
scrollable/data-rich cards are now fully declarative (no Rust drawer):

| Card | `.ron` | Data binding | Interaction |
|------|--------|--------------|-------------|
| `bluetooth` | `Header` + chip + `Rows` (icon·name·state) + empty | `bt_rows`/`bt_status`/`bt_chip_{on,off}` | rows → `bt_key_base` connect plumbing; `start: bt_scroll` |
| `ticker` | `Header`(meta "crypto") + `Rows` (sym·price·chg%) + empty | `ticker_rows`/`ticker_status` | rows → market page; `start: ticker_scroll` |
| `currency` | `Header` + base chip + `Rows` (sym·label·value) | `currency_rows`/`currency_chip` | rows → re-base; active row tinted; `start: currency_scroll` |
| `sshvpn` | `Header` + `Rows` (glyph·line) + empty | `sshvpn_rows`/`sshvpn_status` | none |
| `recent` | `Header`(meta count) + `Rows` (name·open-glyph) + empty | `recent_rows`/`recent_status`/`recent_files_n` | rows → open-file plumbing; hover tint via `col_colors`; `start: recent_scroll` |

Engine additions this tranche:
- `SceneRow.surface: Option<ColorToken>` — resting row tint (active/connected
  well); takes precedence over `hover_surface` exactly like the Rust drawers.
- `SceneRow.col_colors: Vec<Option<ColorToken>>` — per-cell color override that
  beats `row.color`, so a status column (chg% green/red, connected ✓) stays
  colored even when the row is otherwise dimmed.
- `Rows.start: Option<String>` — scalar-bound first-visible row; `skip(start_i)`,
  keys rebase to the visible window (`key_base + j`), clamped to the last row.
- `SceneCol.icon: bool` — renders the cell in the icon font (glyph columns).
- `draw_card_scene` sets the card scroll rects
  (`bluetooth`/`ticker`/`currency`/`recent`) when a scene has no `Ink` body,
  because the skipped Rust drawer was what set them (wheel scrolling must stay
  alive).
- `scene_values()` batch-1 rows: `bt_rows`, `ticker_rows`, `currency_rows`,
  `sshvpn_rows`, `recent_rows` (+ `bt_scroll`/`ticker_scroll`/`currency_scroll`/
  `recent_scroll` Rings, `bt_status`/`ticker_status`/`sshvpn_status`/
  `recent_status` texts, `bt_chip_on`/`bt_chip_off`/`currency_chip`/
  `recent_files_n`). Recent row hover tint (accent name + open glyph) is
  computed per frame from `hover_key` in `scene_values()` via `col_colors`.
- `recent` data stays live without a Rust body: the drawer's 30 s stale cache
  refresh moved into `refresh_recent_if_stale()` (drives the scene path too).

### ✅ DONE — Phase C: whole-shell declarative `shell.ron` (landed 2026-09-15, 204/204 tests)
The same scene model now covers whole surfaces, not just individual cards.
`~/.config/zen-shell/shell.ron` declares named surfaces + reusable component
templates; a declared surface REPLACES its Rust draw path when wired. No file
on disk → every surface runs its built-in Rust layout unchanged.

Engine surface (all in `scene.rs`):
- `SurfaceScene { name, items }` + `ShellScene { components: HashMap<String, Vec<SceneItem>>, surfaces: Vec<SurfaceScene> }` — components are keyed template blocks.
- `SceneItem::Comp { name, x, y }` — template instantiation: the component's
  items are INLINED at load time, translated by `(x, y)` base px (parent boxes
  shift only at their origin — children follow the moved box). `Comp`s nest.
  `ShellScene::expand()` runs once at load; the drawn surface is flat.
- `ShellSceneCache` (mtime hot-reload, mirrors `SceneCache`); `Shell::shell_scene`
  field (state init). `vars::shell_scene_path()` → `~/.config/zen-shell/shell.ron`.
- `Shell::draw_shell_surface(name, v, w, h, pal, vals) -> bool` (panels/mod.rs):
  draws the surface as a `CardScene`, registers hits, dispatches `dispatch_shell_inks`
  mirroring `dispatch_scene_inks`; returns false when the surface isn't declared
  (caller keeps the Rust body). `Ink{name}` now interpolates `{var}` (via
  `substitute`) so a static scene can pick its Rust body per state.
- `Rows` grew: `pitch: Option<f32>` (stride ≠ row height — 46 px nav over 40 px
  rows), `w: f32` (bounded row box for a fixed sidebar), `row_dy` (cell baseline
  offset), `r` (row corner radius). `Hit` grew `hover_only: bool` (surface fills
  only while hovered). `Surface` grew `center_x`/`center_y: bool` (x/y become
  base-px offsets from the parent box's mid — the fullscreen-lock hero pieces
  anchor this way). `NAV_*` settings keys are `pub(crate)`.

Live wiring so far:
- `layout_settings` consults the `settings` surface first (fallback = the Rust
  two-pane layout). Declared chrome: title, hover-✕ close (`Hit` key 1,
  `hover_only`, right-anchored at w−26), sidebar 1 px hairline rail, 180 px nav
  `Rows` (`settings_nav` in `scene_values()`, per-row `surface`+`col_colors`
  carry selected/HoverHl · hover/Hover · idle/Fg2, keys 200…204/219, pitch 46).
- The active section body is `Ink(name: "{settings_ink}")` → `settings_<tab>`
  value → the existing section painters (`layout_settings_*`), unchanged.
- `layout_notifs` consults the `notif` surface first (fallback = the full Rust
  notif layout). Declared chrome: title (16,12), Clear chip (`Hit` key 2,
  right-anchored at w−196, rest `raised` / lift `raised_hl`), and the DND chip
  (`Hit` key 3, right-anchored at w−104) which is fully STATE-DRIVEN through
  dynamic tokens: `surface: value("dnd_chip")` + `surface_hover:
  value("dnd_chip_hover")` + `color: value("dnd_chip_fg")`, bound per frame in
  `scene_values()` (accent when DND on / hover surface when off, label sfg or
  fg, hover whitens the accent via mix(acc, fg, 0.15)). Labels are
  left-anchored at the exact Rust insets (Clear 230, DND 324 — the notif pane
  is a fixed 420-wide surface). The card list body is `Ink(name: "notif_list")`
  → `layout_notifs_body` (cards + empty state).
- Dynamic tokens: `DynColor` + `ColorToken::Value(DynColor)` (Copy, 24-byte
  buffer, RON `value("name")`), `SceneValues` color pool
  (`SceneColor::Token(T)` = live theme re-resolve at draw time vs
  `SceneColor::Raw(u32)` = precomputed blend), `ColorTokenExt::resolve_with` /
  `hover_variant_with` — all 19 draw-path resolver sites in `scene.rs` thread
  the pool. Unknown `value(...)` name or wrong kind ⇒ `pal.fg` (never panics).
- `layout_lock` consults the `lock` surface FIRST, but only when the monitor
  height is ~1080 (the authored RON is center-anchored base px); fallback =
  the Rust lock. Declared `lhero` chrome: fullscreen dim
  (`Surface 0×0`, `value("lock_dim")` = 0x00000059), hero clock (`{clock}`,
  center_x/center_y y −324), `{lock_date}` (y −250, `%d` zero-padded),
  avatar disc (`Surface center`, 52² r26, `value("lock_avatar")` =
  mix(hover, acc, .35) — a new `Surface center_x`/`center_y` mirrors `Text`:
  x/y become base-px offsets from the parent box's mid), `{user_init}` (y
  −192), `{username}` (y −150). The body `layout_lock_body` (password field +
  error/caps + hold-power buttons + hint, own regions 1–4) routes through
  `Ink("lock_screen")`.
- `layout_wallpaper` consults the `wallpaper` surface first (fallback = the
  Rust header + grid). Declared `wphead` chrome: back `Hit` key 1 (12,8,28²,
  hover-only `hover_hl` — a subtle lift the Rust header lacks), `\u{f053}`
  glyph (22,16 icon) + "Backgrounds" (48,16). `layout_wallpaper_body` (empty
  state + thumbnail grid + scrollbar) routes through `Ink("wallpaper_picker")`;
  `layout_wallpaper` now takes `h` to consult/route the surface.
- Only `dashboard` is still declared with an Ink body; its mode layout has not
  been split yet (the banner needs live-scissor/drag-guide tokens). Wire via
  the chrome/body split — never point a mode layout at a surface whose own Ink
  recurses into it.
- **Dashboard surface LIVE (2026-09-16, third pass)**: `layout_expanded` now
  consults the `dashboard` surface in the VIEWING state only (`!editing &&
  !sys_menu_open`). It zeroes the edit-only caches (tray/banner ctrl rects),
  builds a one-frame borrowed `DashDrawCtx { lt, cursor, vals }`, routes the
  surface's `Ink("dash_grid")` (board → extracted `layout_dash_grid`, carrying
  the fluid `card_anim` glide, `edit_settling`, and `DASH_CARD_KEY` regions
  with it) + `Ink("dash_banner")` (→ `layout_banner`), then draws the
  scrollbars inline (`layout_dash_scrollbars`, hit zones 500/501) and returns.
  The viewing strip band now draws inside `layout_dash_grid`; the edit-mode
  chrome mask stays in `layout_expanded` behind the same `if editing` gate.
  Edit-mode chrome (trays, ctrl rows, dot lattice, ✕/grip) + the system ⋮
  menu skip the surface entirely. `draw_shell_surface`/`dispatch_shell_inks`
  gained the `dash: Option<&DashDrawCtx>` param (4 non-dashboard call sites
  pass `None`).
- **Dashboard prerequisites (landed 2026-09-16, 215/215 green)**: the scene
  engine grew `Scissor { x, y, w, h }` + `ScissorEnd` — rectangular clip regions
  emitted as `Cmd::Scissor`/`Cmd::ScissorEnd` into the existing live scissor
  stack (`src/app/mod.rs`, the banner's per-zone windows already use these).
  `w: 0`/`h: 0` axes span the parent box; inert for hit-testing; `offset_item`
  translates `Scissor` like other positioned items. `scene_values()` publishes
  the banner's live bindings: drop-guide inks `banner_guide` (mix(hover, acc,
  .35) = the column under a lifted token) and `banner_guide_dim` (mix(hover,
  fg, .10) = idle guide bars) plus the `banner_editing` / `banner_drag` /
  `banner_zones` (zoned clip active) / `banner_collapsed` / `banner_overflow`
  (whole-strip scroll) toggles — geometry stays Rust-computed, scenes bind the
  inks + show/hide toggles. Tests: clip emit at scaled coords, zero-dim spans
  the parent box, RON roundtrip of defaults, banner tokens resolve through the
  normal `value(...)`/`Toggle(name)` paths. `layout_expanded` still runs its
  own banner + grid untouched — no visual/behavior change this pass.
- Declared-lite deltas vs Rust chrome (first cut): sidebar bg tint + selected
  nav 2 px accent rail not declared (no alpha-8 token); ✕ glyph static fg2;
  chips have no hairline outline (DND fill flat beyond the guarded hover
  blend); lock chrome is authored at the 1080p reference (non-1080 heights use
  Rust); wallpaper back button gained a hover-only fill.
  Scene tests: `shell_scene_parses_and_expands_components` (RON map + Comp
  translation + nested surfaces), `shell_ron_live_file_parses_and_has_surfaces`
  (asserts all four live surfaces + their body Inks; skips when no file in the
  test HOME).

### ✅ DONE — Phase B: layout containers (landed 2026-09-15, 198/198 tests)
Three nestable containers bring Quickshell-style layout to the scene engine,
so card bodies can describe their layout in pure RON without hard-coded x/y:

| Container | Semantics |
|-----------|-----------|
| `Row(x, y, w, h, pad, spacing, valign, items)` | Horizontal slots; `w: 0` items fill remaining width equally; `valign: middle` centres short children. |
| `Column(x, y, w, h, pad, spacing, halign, items)` | Vertical slots; `h: 0` items fill remaining height; `halign: center` centres narrow children. |
| `Stack(x, y, w, h, pad, items)` | Overlay (declaration order z); `w: 0`/`h: 0` children fill the box via `set_dims()`. |

- `w` or `h` of **0** on any non-`Text` item means "fill parent box" — at the
  top level that's the card; inside a container it's the slot. `Text` is
  excluded (`w: 0` is the alignment anchor in Text, not a size).
- `set_dims()` fills zero axes on any variantable, so `Stack` clones children
  to the box size before drawing; committed scenes are unaffected (no
  top-level items use `w: 0` except `Text`).
- The top-level draw was refactored into `fn draw_piece(...)` which resolves
  one item inside a parent box; inner-loop `continue`s became `return`.

### ✅ DONE — Phase 2a: data-driven primitives (landed 2026-09-15, 198/198 tests)
The `SceneValue`/`SceneValues` typed map replaced the flat headers map, and
six body primitives now draw from it in `src/scene.rs`:

| Primitive | Status | Cards it enables (next) |
|-----------|--------|-------------------------|
| `Rows { name, y, row_h, key_base, cols[], pad, hover_surface, max_rows }` | ✅ | procmon, journaltail, wifi, worldclock, smarthealth, systemdunits, conninfo, notes, todo, recent, sensors, thermbl_zones, currency |
| `Spark { name, x, y, w, h, color, max, kind(line\|column), thickness }` | ✅ | cpu, gpu, mem, diskio, net_up, net_down, powerdraw, sensors |
| `Ring { name, cx, cy, radius, center_x, center_y, max, beads, dot, fill, track }` | ✅ | batteryh (pane 0), batteryv |
| `Fader { name, x, y, w, h, fill, key, action }` | ✅ | volume, brightness, sliders, eq |
| `Toggle { name, x, y, w, h, key, action }` | ✅ | settings toggles, bluetooth, compositor, sshvpn |
| `TabRow { name, sel, y, pad, h, key_base, active, idle }` | ✅ | panels with tabs, multi-pane cards |
| `Dropdown { name, x, y, w, options[], key_base }` | 🔜 Phase 3 | settings dropdowns, wallpaper-chooser |
| `Badge { name, x, y, glyph, key, action }` | 🔜 Phase 3 | control center badges |

Typed `scene_values()` keys added this session: `battery_ring`, `mem_pct_f`
(Ring); `volume_fader`, `brightness_fader`, `mic_fader` (Fader); `wifi_on`,
`bt_on`, `power_save_on`, `fx_blur_on`, `fx_shadow_on`, `fx_opacity_on`
(Toggle); `cpu_spark`, `gpu_spark`, `lat_spark`, `net_up_spark`,
`net_down_spark`, `disk_r_spark`, `disk_w_spark` (Spark); `jr_rows`,
`conn_rows`, `top_procs_rows`, `fans_rows` (Rows).

Engine contracts (see `src/scene.rs`):
- `SceneRow { cols, key (0→key_base+i), action, color, surface, col_colors }`;
  `SceneCol { x (req), w, right, center, font, size (≤0→9.5), weight, color,
  icon, dy, edge, truncate, pill, del_anchor }` — `dy: Option<f32>` overrides
  the row baseline for two-line rows; `edge` right-anchors the cell at
  `card_w − edge`; `truncate` cuts the cell to the width left by
  `card_w − 2·pad − truncate`; `pill: Option<PillSpec>` draws a chip; a row's
  ✕ anchors left of the `del_anchor` col's box. Should all land on SceneCol
  (RON-only, no SceneRow literal churn).
  `Rows` also takes `start: Option<String>` (a Ring/scalar name) for scroll
  windows, `visible: Option<String>` (a `Toggle` name — bound `false` hides
  the whole list, row loop skipped without `return` so later items still
  draw), `check: Option<String>` (a `Checks(Vec<bool>)` name → live checkbox
  cells at `check_x`) and the `del_base`/`del_pad`/`del_dy`/`del_size` ✕
  reveals; keys rebase to the visible window.
- `SceneValues::rows/spark/scalar/toggle/fader(name)` return `None` on
  missing **or type-mismatched** names → the item draws nothing, never panics.
- `substitute()` interpolates only `SceneValue::Text`; other types stay literal
  `{name}`.

### Phase A — Convert card bodies (batch by complexity)
Layout containers are in; now convert card bodies to declarative RON using
`Row`/`Column`/`Stack` for structure.

**Batch 1 (text-only + layout + list rows)** — calendar, appshortcut, mirror remain.
> DONE in this tranche: `bluetooth.ron`, `ticker.ron`, `currency.ron`, `sshvpn.ron`, `recent.ron`, `todo.ron`, `alarms.ron`, `notes.ron`, `countdown.ron`, `quote.ron`, `expenses.ron`, `news.ron`, `snippets.ron`, `lyrics.ron`.
> 🟢 composer LANDED (2026-09-16): `SceneItem::Composer` — bottom-anchored
> input strip (caret/placeholder/focus outline/typing hint + optional ⊕; binds
> `{text}` + `Toggle("focus")`; `bottom: true` pins from the card bottom). The
> To-Do split is the reference: checklist stays the `todo` Ink while the strip
> becomes a `Composer` item (keys 78/79), gated by `scene_owns_composer`
> (the `scene_owns_header` pattern) with `todo_focus`/`todo_buf` values.
> 🟢 checkbox cells + hover-✕ delete LANDED (2026-09-16): `Rows` gains a live
> `check` column (`Checks(Vec<bool>)` value — acc-tint well + check glyph vs
> stroke outline) and a `del_base`/`del_pad`/`del_dy`/`del_size` reveal: a
> hovered row shows a right-anchored `ICON_CLOSE` (fg → danger-red on the ✕)
> and registers its 22² corner region only while hovered — same keys/geometry
> as the Rust drawers; key-less rows keep the ✕ (revealed on the ✕ itself
> only). With **todo declarative end-to-end** (`scene_owns_rows` gates
> `draw_todo_card`'s rows loop; `key_base` 60 / `del_base` 70 /
> `start: "todo_scroll"` / `check: "todo_checks"`) and **alarms rows
> declarative** (`alarm_rows` mono-accent time + label, `del_base: ALARM_DEL_BASE`,
> add-row stays Ink) it's the reference for snippets/notes/countdown.
> 224/224 tests, bin clean (input.rs:512).
> 🟢 two-baseline rows + composer gate LANDED (2026-09-16): `SceneCol` gained
> `dy: Option<f32>` (per-column baseline offset → a row draws a title AND a
> dim caption beneath it; falls back to Rows `row_dy`) and `Rows` gained
> `visible: Option<String>` — a bound `Toggle` resolving `false` hides the
> whole list (the arm skips its row loop, NOT `return`, so later items in the
> same scene still draw). **notes** is declarative (`notes_rows`: bullet
> icon · title 9.5 · body caption 8 fg2 at `dy: Some(12)`, `y: 32`, 24 px
> pitch, 12 pad, `row_dy: 0`, `start: "notes_scroll"` wheel window, `visible:
> "notes_composing"` hiding the list while the composer Ink is open); header/
> `+`/empty-hint + the full composer stay Ink, `draw_notes_card`'s row loop
> gated by `scene_owns_rows`. New values: `notes_rows`/`notes_composing`
> (Toggle)/`notes_scroll` (Ring). 227/227 tests, bin clean (input.rs:512).
> 🟢 chip/pill cells + truncation + right-edge anchor LANDED (2026-09-16):
> `SceneCol.pill: Option<PillSpec>` — a chip cell whose rounded rect auto-fits
> its text (height/radius/pad/top), fill derived from the cell's per-row ink
> token (Acc/AccTint → accent tint, else hover) or `bg` override; `truncate:
> Option<f32>` reserves `n` base px on the right (cell cut at
> `card_w − 2·pad − n`, 0.62·fs px/glyph, ≥ 8); `edge: Option<f32>` right-anchors
> the cell at `card_w − edge`; `del_anchor: bool` lands the revealed ✕ + its
> full-row region left of that col. **countdown** is declarative
> (`countdown_rows`: label `truncate: 120` · date caption · right `Nd`/
> `today!`/`passed` pill from per-row `col_colors`, `y: 50`, 24 px pitch,
> `del_base: COUNTDOWN_DEL_BASE` at `del_pad: 20`/`del_dy: 4` via
> `del_anchor`, key-less reveal-on-✕ only); title/meta + composer row stay
> Ink, `draw_countdown_card`'s list loop gated by `scene_owns_rows`. Pills
> pre-unblock news/expenses/quote. 230/230 tests, bin clean (input.rs:512).
> 🟢 `SceneItem::TextWrap` + `visible` gate LANDED (2026-09-16):
> multiline text block (34 chars/line, ≤ 136 chars, vertically centered
> inside a `top`/`bottom` box — the Quote body geometry), optional dim
> `caption` after the block; `visible: Option<String>` on `TextWrap` (and
> `Rows`) gates the item without reserving space (the arm skips drawing,
> later items still draw). `scene_owns_rows` also detects `TextWrap` so
> the Rust drawer skips its own body when the scene owns it. **quote** is
> converted: wrapped text + author caption is a `TextWrap` item (visible
> when `quote_has`, `top: 28` clears header, `bottom: 36` clears pills);
> title/meta + the two pills stay the `quote` Ink. New values:
> `quote_text`, `quote_author`, `quote_has` (Toggle). 232/232 tests,
> bin clean (input.rs:512 only).
> 🟢 **expenses fully declarative — zero Ink LANDED (2026-09-16)**: the whole
> card is `expenses.ron`. `Header` (title + MTD total in accent mono via
> `meta_color`); the **quick-chip `Row`** (six equal-split `Hit`s — `w: 0`
> children share `(inner_w − fixed − sp·gaps)/fills`; 1/5/10/20/50/100,
> keys 31800..31805, `surface: hover` lifting to `hover_hl` exactly like the
> Rust chip hover); the **composer `Row`** (`Composer` `pad: 0` so the row pad
> supplies the inset, `focus: exp_focus`, key 31810 — Enter commits through the
> resident key handler, no action needed — + fixed-70 `Hit` "clear mo" with
> `color: fg2`/`color_hover: danger`, key 31811); top-3 per-category bars:
> `Text caption + Bar + right mono amount`, `Bar.reserve: 46` spans
> `card_w − 46` so the amount column stays clear, each group gated by
> `exp_has_n`, empty-month caption gated by `exp_empty`. New values:
> `exp_meta`, `exp_buf`, `exp_focus` (Toggle), `exp_empty` (Toggle),
> `exp_cat_i`/`exp_val_i`/`exp_bar_i` (ratio vs the top category),
> `exp_has_i` (Toggle). No `scene_owns_*` gating — a scene without Ink is
> dispatched whole-card so the Rust drawer never runs. Four tiny primitives:
> `Header.meta_color`, `Hit.color_hover`, `Text.visible`/`Bar.visible`,
> `Bar.reserve` — all no-ops by default, reusable for news/snippets.
> 234/234 tests, bin clean (input.rs:512 only).
> 🟢 **news fully declarative — zero Ink LANDED (2026-09-16)**: `news.ron`
> fetches its category chips from the new **`Strip`** item (the Batch-1
> "horizontal scrollable chip strip" gap): `Chips(Vec<String>)` labels +
> `sel` (Ring) + `scroll` (Ring, clamped at draw) bindings, active = accent
> + Sfg label, hover lifts, offscreen skip, keys `key_base + i`; header
> `{news_label}`; headline `Rows` (40-char title / right open-glyph / right
> source on a second baseline) with the new `SceneCol.hover` (per-col ink
> while the ROW is hovered), `Rows.hover_bar` (2.5 × row_h−6 acc rail on
> hover) and `Rows.hairline` (1 px dividers); `{news_msg}` centered while
> `news_empty`. 236/236 tests, bin clean (input.rs:512 only).
> 🟢 **snippets fully declarative — zero Ink LANDED (2026-09-16)**:
> `snippets.ron` — `Header` (clip count meta in fg), `Composer`
> (`snip_buf` / "add snippet — name text to copy" placeholder /
> `snip_focus`, keyed `SNIPPET_INPUT_KEY`), clipboard `Rows` (copy glyph ·
> 12-char name · dim body preview; `hover_surface: hover_hl`;
> `key_base: SNIPPET_COPY_BASE`, `del_base: SNIPPET_DEL_BASE` with
> `del_pad: 20`/`del_dy: 4`). New transient-flash engine pair:
> `Rows.flash` (`Ring`, scalar index + 1 / 0 = none) paints the
> translucent-accent wash, `SceneCol.flash` inks opted-in cells accent
> (copy glyph + name), preview stays fg3 — the exact Rust "just copied".
> 238/238 tests, bin clean (input.rs:512 only).
> 🟢 **lyrics fully declarative — zero Ink LANDED (2026-09-16)**: the karaoke
> word-highlight engine (`Rows.karaoke` binding + `SceneValue::Karaoke` +
> `Rows.karaoke_fs`) mirrors the Rust drawer byte-for-byte: the ACTIVE line
> washes accent, walks its words with auto-fitted spacing, paints the playhead
> word white with a 1.5 px underline and the rest near-black, breaking at the
> card edge; inactive rows draw the joined line, lifting hover → accent + a
> quiet hover surface; untimed lyrics unbind `lyr_karaoke` and fall back to
> the plain single-column cell path. `Header.glyph_color` tints the `\u{f001}`
> notes glyph accent. `lyrics.ron`: `Header` (accent note glyph); the dynamic
> state chip as three right-anchored gated `Text`s — `synced`/acc,
> `none`/danger, `{lyr_status_other}`/fg3 (exclusive toggles); the track
> caption (`{lyr_track}`); the centered fallback (`{lyr_msg}` behind
> `lyr_empty`); the karaoke `Rows` (`y: 33`, `row_h: 21`, `r: 4`,
> `hover_surface: hover`, `start: "lyr_scroll"`, `key_base: 33500`, one
> full-line col). `draw_card_scene` still drives `lyrics_follow`.
> 241/241 tests, bin clean (input.rs:512 only).

**Batch 2 (need Rows/Spark)**:
- `procmon.ron`, `journaltail.ron`, `systemdunits.ron`, `smarthealth.ron`, `sensors.ron`, `therbl_zones`, `conninfo.ron`, `worldclock.ron`, `fans.ron`, `diskio.ron`, `network.ron`, `topproc.ron`

**Batch 3 (need Fader/Ring/Toggle)**:
- `sliders.ron` (volume/brightness faders), `eq.ron`, `powerh/powerv.ron`, `batteryh/batteryv.ron` (Ring), `toggles.ron`, `bluetooth.ron`, `compositor.ron`

**Batch 4 (special)**:
- `clipimg.ron` (image thumbnails), `docker.ron` (container list), `worldmap.ron` (canvas), `viz.ron` (FFT), `accent.ron` (color picker), `gauges.ron` (ring gauges), `cpugpu.ron`

### Phase 3 — Panels declarative
- Settings panel: TabRow + Toggle + Slider + Dropdown items, scene-driven.
- Control center: Badge grid + Fader + Toggle.
- System menu (power): Hit items with Command actions.

### Phase 4 — Remove Ink fallback
Once all card bodies are declarative, `draw_rust` becomes dead code.
Remove `Ink` from scene items and the `dispatch_scene_inks` path.
`DashCard::draw` just does `scene.draw()` and registers regions.

---

## Reference — scene_values keys
Text (`{var}`-interpolated, `SceneValue::Text`): `clock, username, hostname, uptime, date, battery, battery_watts, battery_time, volume, brightness, mic_level, cpu, mem, disk, cpu_temp, gpu_temp, net_up, net_down, kblayout`
Header metas (Text): `disk_meta, cpu_meta, water_meta, pomo_meta, speed_meta, wp_meta, jr_meta, sm_meta, sm_icon, auddev_meta, conn_meta, moon_meta, comp_meta, fan_meta, er_meta, thermal_meta, wc_meta, mic_meta, mic_icon, sus_meta, sus_icon, ws_meta`
Phase 2a typed values (non-Text): `battery_ring`, `mem_pct_f` (Ring), `volume_fader`, `brightness_fader`, `mic_fader` (Fader), `wifi_on`, `bt_on`, `power_save_on`, `fx_blur_on`, `fx_shadow_on`, `fx_opacity_on` (Toggle), `cpu_spark`, `gpu_spark`, `lat_spark`, `net_up_spark`, `net_down_spark`, `disk_r_spark`, `disk_w_spark` (Spark), `jr_rows`, `conn_rows`, `top_procs_rows`, `fans_rows` (Rows).
Batch-1 list values (this session): `bt_rows`, `bt_scroll` (Ring), `bt_status`, `bt_chip_on`, `bt_chip_off`; `ticker_rows`, `ticker_scroll`, `ticker_status`; `currency_rows`, `currency_scroll`, `currency_chip`; `sshvpn_rows`, `sshvpn_status`; `notes_rows`, `notes_composing` (Toggle),
`notes_scroll` (Ring); `countdown_rows` (chip text + per-cell ink tokens);
`quote_text`, `quote_author`, `quote_has` (Toggle); `exp_meta`, `exp_buf`,
`exp_focus`, `exp_empty`, `exp_cat_1..3`, `exp_val_1..3`, `exp_bar_1..3`,
`exp_has_1..3`; `news_label`, `news_msg`, `news_chips` (Chips),
`news_cat_sel`/`news_cat_scroll`/`news_scroll` (Ring), `news_empty` (Toggle),
`news_rows` (title / open-glyph / source); `snip_meta`, `snip_buf`,
`snip_focus` (Toggle), `snip_flash` (Ring, index + 1 / 0 = none), `snip_rows`
(copy glyph / name / body preview); `lyr_state` (i32), `lyr_synced`/`lyr_none`
(bool), `lyr_other_lbl`, `lyr_status_other` (Text var), `lyr_track`,
`lyr_msg`, `lyr_empty` (Toggle), `lyr_scroll` (Ring), `lyr_karaoke`
(Karaoke, only bound when `state == 2 && has_timing`), `lyr_rows`.

---

## Snippets conversion notes (2026-09-16)

> The transient **flash** pattern — the raster analog of the Rust drawers'
> one-second "just snapped" highlight — is `Rows.flash: Option<String>`
> (field name `flash` in RON) bound to a `Ring` whose scalar is the flashing
> row index **+ 1** (0 = none, so an index-0 row still works). The row is
> painted with a translucent-accent wash (`(acc & 0xFFFFFF00) | 0x30`, same
> overlay the Rust snippet drawer draws), and each col opting into
> `SceneCol.flash` (field name `flash`, e.g. `flash: Some(acc)`) inks its
> cell with that token. Flash wins over hover surface + hover ink. The flash
> scalar lives in scene_values (`self.snippet_copied` gated by
> `snippet_copied_at.elapsed() < 1`), so the Rust drawer can go fully dark.

## Lyrics conversion notes (2026-09-16)

> Karaoke draws are a **per-word walk**, not cells. `Rows.karaoke: Option<String>`
> names a `SceneValue::Karaoke(Vec<KaraokeLine{words, active}>)` value aligned
> by visible-index against the parallel `Rows` value. When bound (and the
> values exist), each row short-circuits the cell machinery: the ACTIVE line
> washes accent (`r4`), then walks its words with `est_w` = Σ(0.62
> upper/digit, 0.0 space, 0.52 else) × `scale.fs(karaoke_fs)` per word plus
> 4 px spacing, clipping at `x+pmin+row_w−8` like the Rust drawer's
> `app_color_to` break; the playhead word renders white with a `1.5` px
> underline at `ry+row_h−6.5`, the rest `0x1d1d_1d`. Inactive rows draw the
> joined whole line; hover → accent ink + a quiet hover surface (the lifted
> `hover_hl` never appears in karaoke mode). Rows still register regions at
> `key_base + i`. **Plain (untimed) lyrics**: present NO `lyr_karaoke` value
> and the binding resolves None → the normal single-column cell path, no karaoke
> words, no active wash (the dedicated fallback test pins this). The karaoke
> row y0 sits directly below the header, so the `Text`-chip/track stack above
> is the header state — same layout as the Rust drawer. `Header.glyph_color`
> (RON `glyph_color: Some(acc)`, default Fg2) gives the note glyph its tint;
> Rows `karaoke_fs` defaults to 10.5 via the `tenth_half()` helper.

## File locations — where things live

| What | Path |
|------|------|
| Scene engine | `src/scene.rs` |
| Shell struct + DashCard::draw | `src/shell/mod.rs` |
| Shell state init | `src/shell/state.rs` |
| draw_card_scene + scene_values | `src/shell/panels/mod.rs` |
| card_head wrapper + shared types | `src/shell/panels/cards/shared.rs` |
| Card drawers | `src/shell/panels/cards/<id>.rs` |
| Icons | `src/icons.rs` |
| Config root | `/acc/common/hdots/.config/zen-shell/` |
| Scene files | `/acc/common/hdots/.config/zen-shell/ui/cards/<id>.ron` (70 total) |
| Source root | `/acc/common/hdots/rust_project_root/zen-shell/` |
