# zen-shell — session notes (open items only)

> Merged through 2026-09-13: everything that has landed is recorded in
> `CHANGELOG.md` (entries 2026-09-09 / 09-10 / 09-11, the 09-12 session, and
> the 09-13 world-map entries — card, app overhaul, then always-on map /
> pinch-zoom). This file keeps ONLY open items, hard rules, and reference pins.

## 🛑 HARD RULE — multi-pane cards (applies to ALL cards)

Any dashboard card that can be swiped / scrolled **left-right** (multiple panes /
pages of content) **MUST render pagination dots below the card**, centered or
right-aligned at the card's bottom edge, one dot per pane, with the **active
pane's dot highlighted in the accent color** and the others dimmed. This is
required for every such card, present and future — battery (pane 0 = gauges +
power-save, pane 1 = extra info), weather (2 panes), and any card added later
that gets panes. **Exception**: the world-map card switches its 4 panes from a
swipe-right menu (no swipe-cycling, no dots) — dots are required only where the
swipe itself flips panes.

- Horizontal wheel / trackpad-two-finger over the card flips panes (see
  `input.rs` `hsteps` handling: `over_battery` → `battery_pane_flip`).
- **No wrap-around**: swiping past the LAST pane does nothing (clamped), the
  card never jumps back to pane 0 mid-swipe.
- Vertical wheel over a multi-pane card keeps its normal behavior (list scroll
  or board pan if the card has no list).
- Shared helpers, N-pane safe: `pane_dots` (any pane count; `battery_dots` is
  now a 2-pane alias) and `card_pane_flip_n`. Current consumers: battery V/H
  (2 panes), weather (2 panes).

## 🛑 HARD RULE — cards never overlap

The metro grid must NEVER show overlapping cards:

- When new cards are added and don't fit the current rows, the canvas
  **auto-grows** rows (up to `DASH_MAX_ROWS` = 4096, the sanity ceiling) and
  packs them into the new row — it never paints on top of an existing card.
  `pack_cards_max` is guaranteed overlap-free; the old fallback that cloned a
  card verbatim onto occupied cells was removed (now it lands on the first
  fully-empty row below all content).
- Edit-mode drag / resize already re-packs the grid live with the dragged card
  pinned at the ghost slot — other cards are PUSHED aside so the dragged card
  always makes room for itself.

## OPEN / TO VERIFY (user iterates visually)

- **Banner strip editor** (shipped — fine-tuning only):
  - dashboard ON: is the always-visible strip-editor row + pushed-down cards
    tray OK?
  - column drop-guides: too faint / too loud? drop–highlight semantics feel right?
  - empty-strip collapse: removing every token → the board rises to the top
    (`dash_top`/`tray_at_top` fold to `BASE_BANNER_Y+2`).
  - width AUTO/MANUAL step + slider feel; spacer handle; chip spacing.
  - Known: `BannerDrag.sx/sy` never read — deleted in the 09-11 sweep (drag
    offsets ride `off_x`/`off_y` instead).
- **Power / clipboard / wallpaper visual pass**:
  - hold-to-confirm delay on PowerV/PowerH cards matches the power menu.
  - copy a few images → Clipboard-images card populates; hover- and click-copy.
  - wallpaper card: are adaptive tiles + scrollbar + count enough, or should the
    tile render "contain" (whole picture) instead of cover-crop? That touches the
    shared thumb pipeline (`generate_thumbnail` in `src/img.rs`, `rasterize_cover`
    → square `THUMB_SIZE`; tiles are square `Cmd::Image`s in `draw_wallpaper_card`).
- **Hygiene (OPEN)**: the 09-12 session (settings/accent panels rework,
  wiki-usage cards, weather/input/scroll work, README) never got CHANGELOG.md
  entries — README captured it, CHANGELOG stopped at 09-11. Write them when
  the current pass settles.
- Long-term items stay in `roadmap.md` (multi-monitor, fractional scale, pipewire
  audio backend, MPRIS event-driven, auto-lock, plugin system, etc.).

## SETTLED / HALTED

- **DONE (2026-09-12) — Vulkan feature-gated**: `render_vk` now compiles only
  with `cargo build --features vk` (`vk = ["dep:ash","dep:naga"]`,
  `#[cfg(feature = "vk")]` guards in `main.rs`/`app/mod.rs`). Default builds
  are GL-only; runtime still picks Vulkan via `ZEN_VULKAN=1`, falling back to
  GL on init failure. Docs synced 2026-09-13 (README stack + src-map + build
  note, PROJECT_GUIDE boot flow). `features.md` also re-synced: test count
  63→73, and the "chip width resize" section is now honestly marked
  never-landed (`banner_chip_widths`/`banner_resize` don't exist).
- **HALTED (do not reopen): zeneq EQ crate** — PipeWire here can't run LADSPA
  (`libspa-filter-graph-plugin-ladspa.so` needs `spa_log_topic_enum`, absent from
  the installed `libspa-0.2.so`; daemon crashes exit 254). Fix = update
  PipeWire/spa packages, then reinstall per the zeneq README.

## Quick reminders

- Canonical path: `/acc/common/hdots/rust_project_root/zen-shell` (the old
  `/tmp/user-tw/zs` alias is gone).
- Shared cargo target: `/acc/data/persist/user/.cargo/target` (debug + release).
- No git repo on this copy — no commits available.

## Key reference pins (updated 2026-09-13)

- Packer: `pack_cards_max` (NOT `pack_cards`) in `src/shell/mod.rs`; drag tests in
  `shell::edit_drag_tests`; layout geometry tests in `shell::panels::layout`;
  `banner_drop_target` is `#[cfg(test)] pub(crate)` (drop math, unit-tested).
- Strip-editor control row: `edit_press` in `src/shell/mod.rs` —
  `BANNER_CTRL_KEY_BASE`=34_300 (dashboard ON/OFF), `BANNER_W_AUTO_KEY`=34_332
  (width auto/manual), `BANNER_W_SLIDER_KEY`=34_333 (hidden in auto).
- Banner strip: `banner_cells(row_w)` + `banner_strip_w()` + `banner_cell_px(i,row_w)`
  (geometry, `src/shell/mod.rs`); drawer + drop grid in `src/shell/panels/dashboard.rs`.
- Scroll semantics: `input.rs` ~389 (vertical wheel) & ~475 (horizontal wheel),
  `dash_scroll_dir` swap.
- Grid toolbar draw: `dashboard.rs` ~473-509 (`bw=18` btn, `cw_t` label width,
  right-anchored cluster).
- `draw_edit_btn` / `draw_edit_lbl_btn`: `dashboard.rs` 803 / 836.
- Config persistence: `save_config` in `src/shell/mod.rs` (~2854+); DashCard draw
  dispatch ~126-171 & ~231-276.
- World-map card (landed 2026-09-13, overhauled into an app same day): an
  EMBEDDED 110m world (`src/shell/world_map.bin`, include_bytes → always
  renders, `ensure_fallback`/`using_fallback`) plus GitHub data
  `ezone134/zen-shell-map` (`src/shell/mapdata.rs` - manifest cache, sha256
  chunk downloads, `region_at`); runtime store in `src/shell/worldmap.rs`
  (chunk parsers, landmask, zone.tab projection, `ready`/`clear`); drawer
  `draw_worldmap_card` in `src/shell/panels/cards/worldmap.rs` (scissored
  body, compact download pill while fallback, 4 menu panes, +/− zoom +
  touchpad PINCH via `zwp_pointer_gestures_v1`, region-detail pill,
  `over_worldmap`). Keys: WORLD_MAP_KEY=14_900, WORLD_DL_KEY=14_901,
  WORLD_REGION_DL_KEY=14_902, WORLD_ZOOM_IN/OUT=14_903/4, WORLD_MENU_KEY=
  14_905, WORLD_MENU_TOGGLE_KEY=14_906, WORLD_STYLE_KEY_BASE=14_910,
  WORLD_PANES=4. Cache: ~/.local/share/zen-shell/maps (save, DEFAULT OFF —
  `[world] save_data=false`) or /tmp/zen-shell/maps. Pinch binding:
  `ensure_pinch` in `src/app/handlers.rs` (janky? verify). Offset probe
  `world_shell_probe` in `src/app/mod.rs` (3 s services timer; DST re-probe
   ~5 min). Default layout row 16: `(WorldMap, 0, 64, 10, 9)`.

---

## Scene-Graph Declarative UI (added 2026-09-15)

Full details: `HANDOFF_SCENE_DECLARATIVE.md`.

### State
- **28 Header-chrome scenes** live (all `card_head` cards), scenes declare their
  own title/glyph/meta; Rust body drawers use `Ink(name:…)` for bodies.
- `Shell::scene_owns_header` guards double-draw; `self.card_head` wrapper + 29
  `ui::title` guards all skip chrome when the scene owns it.
- `scene_values()` now exposes 22 dynamic meta/glyph keys (`disk_meta`,
  `cpu_meta`, `moon_meta`, `mic_icon`, etc.) **plus** typed values:
  `battery_ring`, `mem_pct_f` (Ring), `volume_fader`, `brightness_fader`,
  `mic_fader` (Fader), `wifi_on`, `bt_on`, `power_save_on`, `fx_blur_on`,
  `fx_shadow_on`, `fx_opacity_on` (Toggle), `cpu_spark`, `gpu_spark`,
  `lat_spark`, `net_up_spark`, `net_down_spark`, `disk_r_spark`,
  `disk_w_spark` (Spark), `jr_rows`, `conn_rows`, `top_procs_rows`,
  `fans_rows` (Rows).
- **Phase 2a LANDED (2026-09-15)**: six data-driven primitives — `Rows`,
  `Spark`, `Ring`, `Fader`, `Toggle`, `TabRow` — draw from the typed
  `SceneValue`/`SceneValues` map. 190/190 tests at the time.
- **Phase B LANDED (2026-09-15)**: layout containers — `Row`, `Column`,
  `Stack` — with `pad`, `spacing`, `valign`/`halign`, fill semantics (0-dim
  items span the parent box), and nested container nesting. Cards can now
  describe their body layout in pure RON without hard-coded x/y offsets.
- **Batch 1 start LANDED (2026-09-15)**: fully-declarative list cards. `Rows`
  gained `start` (scalar-bound scroll window — keys rebase to the visible
  window), per-row `surface` (resting active/connected tint; wins over hover
  like the Rust drawers), and per-cell `col_colors` (status columns stay
  colored on active/dimmed rows); `SceneCol` gained `icon` (glyph cells).
  `draw_card_scene` tracks card scroll rects
  (`bluetooth`/`ticker`/`currency`/`recent`) for wheel scrolling when a
  fully-declarative scene (no `Ink`) skips the Rust drawer. 202/202 tests at
  the time.
- **Converted so far (Batch 1)**: bluetooth, ticker, currency, sshvpn, recent
  — all data-driven via `Rows` (+ scroll / per-row state / hover tint via
  `col_colors`), no Rust drawer involved. `recent` also extracted its stale
  30 s cache refresh into `refresh_recent_if_stale()` so the scene path keeps
  the data live (`draw_card_scene` calls it when no `Ink`).
  Remaining Batch 1 cards to convert: ~~calendar, appshortcut, mirror~~ ALL
  DONE (2026-09-16 — calendar/appshortcut are zero-Ink `Grid` scenes, mirror
  is a two-pane scene with `Image`/`Row(bottom)`/`Rows`/`Toggle`/`Dots`).
  **Batch 1 COMPLETE.**
- **Interactive list cells LANDED (2026-09-16)**: `Rows` gained a live
  checkbox column (`check` + `check_x` over the parallel `Checks(Vec<bool>)`
  value — acc-tint well + check glyph vs stroke outline, the To-Do box) and a
  hover-✕ delete (`del_base`/`del_pad`/`del_dy`/`del_size`: reveal while the
  row or ✕ is hovered — key-less rows reveal on the ✕ only, like the alarms
  drawer — danger-red on the ✕ itself, 22² corner region registered only
  while hovered so it wins the row corner like the Rust drawers). **todo**
  became the first three-split card: title/count/`+N more`/empty-state stay
  the `todo` Ink, the checklist is a `Rows` item (`key_base: TODO_KEY_TOGGLE`,
  `del_base: TODO_KEY_DELETE`, `start: "todo_scroll"`, `check: "todo_checks"`),
  and the strip is the `Composer` — `scene_owns_rows` (the
  `scene_owns_header`/`scene_owns_composer` pattern) gates the Rust row loops.
  **alarms** followed the same way (rows only: mono accent time + label,
  `del_base: ALARM_DEL_BASE`, `del_pad: 26`/`del_dy: 4`, add-row + count stay
  Ink). Snippets/countdown adopt the ✕ the same way. 230/230 tests,
  bin clean (only input.rs:512 pre-existing).
- **Countdown list body declarative LANDED (2026-09-16)** (230/230):
  `SceneCol` gained `truncate: Option<f32>` (reserve `n` base px on the right,
  cut the cell at 0.62·fs px/glyph ≥ 8), `edge: Option<f32>` (right-edge
  anchor `card_w − edge`), `pill: Option<PillSpec>` (chip cell — rounded rect
  auto-fit to its text, fill derived from the text token: Acc→tint else
  hover, or `bg` override) and `del_anchor: bool` (revealed ✕ + full-row
  region lands left of that col). **countdown** is converted: `countdown_rows`
  (label `truncate: 120` · date caption · right `Nd`/`today!`/`passed` pill,
  `y: 50`, 24 px pitch, `del_base: COUNTDOWN_DEL_BASE` at `del_pad: 20`/
  `del_dy: 4` via `del_anchor`, key-less ✕); title/meta + composer row stay
  Ink, list loop gated by `scene_owns_rows`. Pre-unblocks newspapers
   (chip strips + two-line headlines), expenses (amount chips), quote (pills).
- **News fully declarative LANDED (2026-09-16)** (236/236): `news.ron` is
   zero-Ink — `Header` (`\u{f1ea}`, label = `{n} stories`/active category),
   the new **horizontal `Strip`** (scrollable chip row: `Chips(Vec<String>)`
   value + `sel`/`scroll` Ring bindings, offscreen skip, active accent +
   Sfg label, `key_base + i`; wheel handler clamps `news_cat_scroll`), the
   centered fetching/no-feeds message gated by `news_empty`, and the headline
   `Rows` (40-char title, right open-glyph, right source label on a second
   baseline, `y: 59`, 30 px pitch, `key_base: 13300`, `start: "news_scroll"`).
   Three new list-ink primitives: `SceneCol.hover` (per-col ink while the ROW
   is hovered — title→acc, source stays fg3), `Rows.hover_bar` (the rounded
   2.5×24 accent rail on the hovered row), `Rows.hairline` (1 px dividers).
   Chip/headline keys route through the resident handler — no Ink.
- **Snippets fully declarative LANDED (2026-09-16)** (238/238): `snippets.ron`
   is zero-Ink — `Header` (clip-count meta in fg), `Composer` (buffer /
   placeholder / focus toggle, keyed SNIPPET_INPUT_KEY), and the clipboard
   `Rows` (copy glyph · name · body preview) with `hover_surface: hover_hl`
   and the new **`Rows.flash`** transient accent — bound scalar = index + 1
   (0 = none) from a `Ring`, opted-in cells use `SceneCol.flash` so the
   copy glyph + name wash accent while the dim preview stays fg3; flash
   overrides hover surface + ink exactly like the Rust drawer's 1 s flash.
   Values: `snip_meta`, `snip_buf`, `snip_focus`, `snip_flash` (Ring),
   `snip_rows`. 238/238 tests, bin clean.
- **Expenses fully declarative LANDED (2026-09-16)** (234/234): the whole card
   is `expenses.ron` with **zero Ink** — `Header` (title + MTD total in accent
   mono via `meta_color`), a **`Row` of six equal-split `Hit` chips**
   (1/5/10/20/50/100, `surface: hover`, keys 31800..31805), the composer row
   (`Composer` with `pad: 0` inside the row pad, `focus: exp_focus`, key
   31810, Enter commits via the resident key handler) + a fixed-70 `Hit`
   "clear mo" flipping `fg2 → danger` on hover (key 31811), and top-3 bars
   (`caption + Bar + right mono amount`, `Bar.reserve: 46` keeps the amount
   column clear, each group gated by `exp_has_n`, empty caption gated by
   `exp_empty`). Four tiny primitives: `Header.meta_color`, `Hit.color_hover`
   (pre-unblocks news title hover), `Text.visible` + `Bar.visible` (Toggle
   gates), `Bar.reserve`. No `scene_owns_*` needed — a scene without Ink is
   dispatched whole-card. The equal-split `Row` replaces the need for a fixed
   chips strip; news still needs the horizontal **scrollable** strip.
- **Quote body declarative LANDED (2026-09-16)** (232/232):
   `SceneItem::TextWrap` — a multiline text block that wraps at 34 chars/line
   (≤ 136 chars total), vertically centered inside a `top`/`bottom` box (card
   insets), with an optional dim `caption` line trailing the block. `visible:
   Option<String>` gates the whole item on/off without reserving space (the
   `scene_owns_rows` flag also detects `TextWrap` so the Rust drawer skips its
   own body when the scene owns it). **quote** is converted: the wrapped text
   + author caption is a `TextWrap` item (visible when `quote_has`, `top: 28`
   clears the header, `bottom: 36` clears the pills); title/meta + the two
   pills (refresh always, save conditional) stay the `quote` Ink; the flash
   decrement stays inside the Ink. New values: `quote_text`, `quote_author`,
   `quote_has` (Toggle). Pre-unblocks **lyrics** (multiline wrap, partial) and
   conditional body blocks in future cards.
- **Notes list body declarative LANDED (2026-09-16)** (227/227): `SceneCol`
  gained `dy: Option<f32>` (per-column baseline — a row now carries a title +
  dim caption line; falls back to `row_dy`), and `Rows` gained `visible:
  Option<String>` — a bound `Toggle` resolving `false` hides the whole list
  (the arm skips its row loop, not `return`, so later items still draw). The
  **notes** card is converted: `notes_rows` (bullet tint col · title 9.5 ·
  body caption at `dy: Some(12)`, `y: 32`, 24 px pitch, 12 pad, `row_dy: 0`,
  `start: "notes_scroll"`, `visible: "notes_composing"`) while header/`+`/
  empty-hint and the full composer stay Ink, the Rust row loop gated by
  `scene_owns_rows`. `dy` pre-unblocks the news/lyrics/countdown two-baseline
  rows.
- **202/202 tests pass**, bin build clean (only input.rs:512 pre-existing).
- **Phase C start LANDED (2026-09-15) — whole-shell declarative (`shell.ron`)**:
  new top-level `ShellScene` / `SurfaceScene` / `ShellSceneCache` in `scene.rs`
  (`~/.config/zen-shell/shell.ron`, mtime hot-reload like card scenes), the
  `Comp` item (named component template INLINED at load with a `(x, y)` anchor
  offset — components may nest), and `Shell::draw_shell_surface(name, …)` +
  `dispatch_shell_inks` (surface `Ink`s route to the section Rust painters).
  `Rows` gained `pitch` (stride override — 46 px nav rows over a 40 px body),
  `w` (bounded row box), `row_dy` (cell baseline) and `r` (row corners); `Hit`
  gained `hover_only` (surface only while hovered — the ✕ close button);
  `Ink.name` now interpolates `{var}` so the declared scene can pick its Rust
  body per state. **`settings` surface is LIVE**: the two-pane Settings app's
  chrome (title, hover-✕ close, sidebar rail + 180 px nav `Rows` with
  selected/hover surfaces from `scene_values` `settings_nav`) is declarative;
  each section body is `Ink(name: "{settings_ink}")` routed to its existing
  Rust painter. There is no `shell.ron` on disk → every surface still falls
  back to the built-in Rust layouts unchanged. 204/204 tests at the time.
- **Dynamic color tokens landed (2026-09-16)**: `ColorToken::Value(DynColor)`
  (Copy, 24-byte buffer, `value("name")` in RON), `SceneValues` color pool
  (`SceneColor::Token(T)` live-resolved at draw time vs `SceneColor::Raw(u32)`
  for computed blends), `ColorTokenExt::resolve_with`/`hover_variant_with` —
  all 19 draw-path resolver sites in `scene.rs` thread the pool. Together they
  unblock state-driven single-element colors (DND chip, chips, banner).
- **`notif` surface is LIVE (2026-09-16)**: `layout_notifs` split into chrome
  (title `ntitle` + Clear chip `nclear` → declarative `Hit`, key 2) +
  `layout_notifs_body` (list + empty state → `Ink("notif_list")`); consults
  `draw_shell_surface("notif")`, falls back to the full Rust layout when not
  declared. DND chip (`ndnd`, key 3) is declarative and state-driven via the
  dynamic tokens: `dnd_chip`/`dnd_chip_hover`/`dnd_chip_fg`. 209/209 tests,
  bin clean (only input.rs:512).
- **`lock` surface is LIVE (2026-09-16, second pass)**: hero chrome (`lhero`)
  — fullscreen dim, `{clock}`, `{lock_date}`, avatar disc + `{user_init}` +
  `{username}` — is center-anchored to the fullscreen surface via new
  `Surface` `center_x`/`center_y` fields (offsets are base px from the box
  mid) and bound colors `value("lock_dim")` / `value("lock_avatar")`;
  `layout_lock_body` (password field, error/caps, hold-power buttons, hint —
  owns keys 1–4) routes through `Ink("lock_screen")`. Consulted only at the
  ~1080 reference height (h ± 24); other sizes run the Rust layout.
- **`wallpaper` surface is LIVE (2026-09-16, second pass)**: header chrome
  (`wphead`) — back `Hit` key 1 (hover-only fill) + `\u{f053}` + "Backgrounds"
  — byte-for-byte positions; `layout_wallpaper_body` (empty state + thumbnail
  grid + scrollbar) routes through `Ink("wallpaper_picker")`.
- `shell.ron` now declares the `dashboard` surface with BOTH bodies live:
  `Ink("dash_grid")` (board) + `Ink("dash_banner")` (top banner strip) route
  through the exact Rust painters during the viewing state (`!editing &&
  !sys_menu_open`); the scrollbar overlay stays Rust inline with its drag hit
  zones 500/501. Edit-mode chrome + the system ⋮ menu remain Rust (frame-
  dynamic geometry) and never consult the surface. ⚠️ non-recursive by design:
  the painters are CALLED BY `layout_expanded`, never the reverse.
- **Viewing strip CHIPS are declarative (2026-09-16, fourth pass)**: the
  `dchips` component (a `BannerRow` bound to `banner_cells_v`, gated on
  `banner_shown`, with `banner_overflow`/`banner_zones` scissor modes) rides
  the `dash_banner` Ink's dispatch slot after the painter — z-order grid →
  band → edit chrome → chips. Cells publish per frame via
  `publish_banner_cells` (geometry from `banner_cells`, inks = `bchip_*`
  ladder bindings, tray sub-regions 30+ti); the Rust chip sweep + viewing
  connectivity popover stand down while `dash_chips_via_scene` is up. STILL
  RUST: the whole EDIT strip (guides, grips, park-minus badges, zone labels,
  drag ghost, parked tray, editor rows), the connectivity popover, and the
  tray-click/panel-open INPUT routing (keys are unchanged, so no input work
  is needed).
- **Dashboard prerequisites LANDED (2026-09-16, third pass next)**: the scene
  engine now has `Scissor`/`ScissorEnd` items (rect clip regions emitted as
  `Cmd::Scissor`/`Cmd::ScissorEnd` into the existing live scissor stack; `w:0`/
  `h:0` axes span the parent box) and `scene_values()` publishes the banner
  live tokens — `banner_guide`/`banner_guide_dim` drop-guide inks + the
  `banner_editing`/`banner_drag`/`banner_zones`/`banner_collapsed`/`banner_overflow`
  toggles. The `dashboard` surface split (banner chrome declarative via
  `Scissor` zones + guide `Hit`s; grid via the chrome/body Ink) can now be
  wired. 215/215 tests, bin clean (only input.rs:512).
- Declared-lite deltas vs the Rust chrome (first cut): the sidebar's faint
  bg tint and the selected-nav 2 px accent rail are not yet declared (no
  alpha-8 token); ✕ glyph stays fg2 (not hover-whitened); chips have no
  hairline outline (the DND chip fill is flat beyond the guarded hover
  blend); lock chrome is authored at the 1080p reference (non-1080 heights
  use Rust); wallpaper back button gained a hover-only fill.

### Next step (Phase A — batch body conversions) — Batch 1 DONE 2026-09-16
Layout containers are in; card bodies can now be converted to fully-declarative
RON using `Row`/`Column`/`Stack` for structure (no absolute x/y). Start with the
text-only batch that needs no new primitives:

| Primitive | Purpose |
|-----------|---------|
| `Rows { name, y, row_h, key_base, cols[] }` | Dynamic row list from named `Vec<SceneRow>` |
| `Spark { name, x, y, w, h, color, max }` | Line/column chart from named `Vec<f32>` |
| `Ring { name, cx, cy, radius, max, fill, track }` | Circular gauge from `f32` |
| `Fader { name, x, y, w, h, fill, key, action }` | Slider (volume/brightness) |
| `Toggle { name, x, y, w, h, key, action }` | On/off switch bound to `bool` |
| `TabRow { name, sel, y, key_base }` | Horizontal tab bar for panels |
| `Row` / `Column` / `Stack` | Layout containers (nestable, zero-dim fills parent) |

`SceneValue` enum replaces the flat `HashMap<String, String>` in
`scene_values()`: `Text(String)`, `Rows(Vec<SceneRow>)`, `Spark(Vec<f32>)`,
`Ring(f32)`, `Toggle(bool)`, `Fader(f32)`.

Convert card bodies in batches:
1. **Batch 1** (text/list cards): quote, ticker, news, todo, snippets, notes,
   lyrics, currency, recent, alarms, countdown, expenses, calendar, bluetooth,
   sshvpn, appshortcut, mirror — list rows via `Rows(body)`, chrome via
`Header`/`Text`/`Hit`, scroll via the `start` binding. DONE: bluetooth,
    ticker, currency, sshvpn, recent, todo, alarms, notes, countdown, quote,
expenses, news, snippets, lyrics. Remaining: calendar, appshortcut,
    mirror.
   Engine gaps for these: 🟢 composers LANDED (2026-09-16 — `Composer`, the
   todo strip is the reference `todo_focus`/`todo_buf` split); 🟢 hover-✕
   delete + live checkbox cells LANDED (2026-09-16 — `Rows` `del_base`/`del_pad`
   reveal + `Checks` checkbox column; ✕ parities captured via `del_dy`/`del_pad`
   — todo/alarms are the references, alarms also proves key-less rows still
   reveal on the ✕); 🟢 two-baseline rows + stateful list hiding LANDED
   (2026-09-16 — `SceneCol.dy` per-column baseline offset, `Rows.visible`
   bound Toggle; notes is the reference, list hidden while its composer Ink
   is open, later items still draw); 🟢 chip/pill cells + truncation +
   right-edge anchor LANDED (2026-09-16 — `SceneCol.pill`/`truncate`/`edge`/
   `del_anchor`; countdown is the reference — per-row chip ink from
`col_colors` derives the fill, pills pre-unblock news/expenses/quote);
   still need:
     equal-width grid cells (calendar, appshortcut); image cells (appshortcut);
     camera feed (mirror).
   (lyrics' karaoke word highlight is now LANDED — see 2026-09-16 CHANGELOG.)
2. **Batch 2** (need Rows/Spark): procmon, journaltail, systemdunits,
   smarthealth, sensors, thermbl_zones, conninfo, worldclock, fans, diskio,
   network, topproc.
3. **Batch 3** (need Fader/Ring/Toggle): sliders, eq, powerh/powerv,
   batteryh/batteryv (Ring), toggles, compositor, sshvpn.
4. **Batch 4** (special): clipimg, docker, worldmap, viz, accent, gauges, cpugpu.

Finally: remaining panels/modes declarative → remove Ink fallback. In
progression: `settings` chrome is live (2026-09-15), `notif` chrome + the
state-driven DND chip via dynamic tokens (2026-09-16), then `lock` (hero
chrome at the 1080p reference) + `wallpaper` (header chrome) went live
(2026-09-16, second pass); the `dashboard` surface went LIVE in the viewing
state (2026-09-16, third pass — board `dash_grid` + banner `dash_banner`
Inks route to their exact Rust painters; banner chrome declarative + the
card grid will be the next conversions), and finally the transient
popups/OSDs.

---

## ▶ NEXT SESSION (resume 2026-09-16)

State at handoff: **253/253 tests green**, bin builds clean (only the
pre-existing `src/shell/input.rs:512 unreachable-pattern warning — do NOT
fix**). `~/.config/zen-shell/shell.ron` exists and parses (live
`shell_ron_live_file_parses_and_has_surfaces` test guards it).
**Batch 1 card conversions COMPLETE** (2026-09-16): calendar, appshortcut,
mirror are the last three — calendar/appshortcut zero-Ink `Grid` scenes,
mirror a full two-pane scene (new `Dots` item, `Row.bottom`, negative
`w/h` fill on `Grid`/`Image`, `Hit.glyph`, `Header.title_color`,
`GridCell.hover`/`truncate`, `ColorToken::SelBg`, `DynColor::named`).
See CHANGELOG 2026-09-16 (Batch 1 complete) for the full list.
**Dashboard banner viewing state is DECLARATIVE too** (2026-09-16, passes
3+4): `dband` (band) + `dchips` (`BannerRow` ← `publish_banner_cells`)
ride the `dash_banner` Ink's dispatch slot; Rust chip sweep stands down via
`dash_chips_via_scene`. Edit-mode banner chrome + edit input stay Rust.
NEXT: **Batch 2** card conversions (Rows/Spark cards — procmon, journaltail,
systemdunits, smarthealth, sensors, thermbl_zones, conninfo, worldclock,
fans, diskio, network, topproc). **Batch 2 openers DONE (2026-09-16):**
procmon, fans, topproc are fully declarative via the new `Rows` per-row BAR
cell (`SceneCol.bar` — interpolated fraction value, track + fill, honors
x/w/right/edge); `top_procs_rows`/`fans_rows` now publish the per-row bar
fractions + per-column inks. **Spark trio DONE (2026-09-16):** network,
diskio, sensors are fully declarative too — network/diskio drive the
pre-normalized `net_*_spark`/`disk_*_spark` bindings (dual series, one ink
per series: info + acc), sensors gates per-hardware rows on the new
`sensors_*` toggles + `sensor_*` live strings; new `net_meta` y-axis caption.
Remaining Batch 2: journaltail, systemdunits, smarthealth, thermbl_zones,
conninfo, worldclock. **Session paused here (2026-09-17, mid-Batch-2):**
all their live bindings ALREADY EXIST in scene_values (`jr_tail_rows`/
`jr_empty`/`jr_meta`, `sus_rows`/`sus_empty`/`sus_meta`/`sus_icon`,
`sm_rows`/`sm_empty`/`sm_meta`/`sm_icon`, `thermal_cells`/`thermal_empty`,
`conn_rows_v`/`conn_empty`) — only the RON scenes are unwritten.
Engine now has `GridCell.sub_dy`/`sub_size` (thermal parity: zone caption
+ temp per chip; Rust zone cy+3 · temp cy+12 ≈ centered + sub_dy −1),
thermal_cells publishes sub+sub_color+sub_dy(-1)+sub_size(9). 258/258
green, bin builds. Remaining: author the 5 RON scenes (worldclock stays
Rust — stateful search/overlay/hover-✕), extend committed-scenes test,
sync live copies, docs. journaltail/systemdunits/smarthealth each also
have a refresh Hit (bottom-anchored `Row(bottom)`, keys 31_995/31_990/
31_980, ICON_REFRESH `\u{f021}`, hover lifted-surface + acc glyph).

### Where things stand
- Whole-shell engine lives in `scene.rs`: `ShellScene`/`SurfaceScene`/
  `ShellSceneCache`, `SceneItem::Comp` (template inlining at load, `(x,y)`
  anchor, nestable), `Surface` `center_x`/`center_y`, `draw_shell_surface()`
  in `panels/mod.rs`, `Rows` (`pitch`/`w`/`row_dy`/`r`), `Hit.hover_only`,
  `Ink{name}` `{var}` interp.
- **Live surfaces: `settings` + `notif` + `lock` + `wallpaper` + `dashboard`** —
  settings: chrome (title, hover-✕, sidebar rail, nav `Rows`) declarative,
  section bodies = `Ink("{settings_ink}")`. notif: title + Clear + state-driven
  DND chip declarative, cards = `Ink("notif_list")`. lock: hero chrome
  declarative (consulted at ~1080 only), field/buttons = `Ink("lock_screen")`.
  wallpaper: back + title declarative, grid = `Ink("wallpaper_picker")`.
  dashboard: board + banner = `Ink("dash_grid")` + `Ink("dash_banner")`
  (viewing state only; scrollbars = Rust inline after the surface; edit-mode
  chrome + sys ⋮ menu stay Rust). **Nav `w` must
  be 168 (180 − 2×6 pad), NOT 180** — `Rows.w` is the inner box between pads.
- **Dynamic tokens done**: `value("name")` resolves per frame via
  `SceneValues.colors` (`SceneColor::Token`/`Raw`); `Surface` (and `color`,
  `surface_hover`, `draw_text`) honor it. DND chip + lock dim/avatar =
  reference impls.
- Every fixed panel/mode surface is now LIVE except the `dashboard` banner/board
  **chrome** (the surface itself is wired — its two Inks already draw the exact
  Rust board + banner bodies). Do NOT wire by pointing a
  mode layout at a surface whose own `Ink` recurses into that layout (infinite
  recursion). The viewing-state consult is the reference: `layout_expanded`
  zeroes the edit-only caches, calls `draw_shell_surface("dashboard", …,
  Some(&DashDrawCtx{lt, cursor, vals}))`, then draws the scrollbars inline before
  returning; edit mode + the sys ⋮ menu skip the surface entirely.

### Build order (do the first item first — it unblocks the rest)
1. ~~**Dashboard banner chrome declarative**~~ **VIEWING STATE DONE
   (2026-09-16, passes 3+4)**: the strip band (`dband`) rides
   `dash_banner`'s dispatch slot via the Ink `pre` z-order feature (cards →
   band → chips, no double paint — `dash_band_via_scene` set before the
   surface consult; `Surface` per-corner concave radii for the band's
   screen-edge shaping), and the strip CHIPS are declared too: the `dchips`
   component (a `BannerRow` bound to `banner_cells_v`, gated on
   `banner_shown`, with `banner_overflow`/`banner_zones` scissor modes)
   rides the SAME slot after the painter. Cells publish per frame via
   `publish_banner_cells` (geometry from `banner_cells`, inks = the
   `bchip_*` ladder bindings, tray sub-regions 30+ti preserved); the Rust
   chip sweep + viewing connectivity popover stand down while
   `dash_chips_via_scene` is up. REMAINING (all edit-mode, frame-dynamic,
   deliberately Rust): drop guides, grips, park-minus badges, zone labels,
   the drag ghost, the parked tray + editor rows — plus the banner edit
   INPUT. See CHANGELOG 2026-09-16 (BannerRow) for the full picture.
2. ~~**Resume card conversions** — remaining 3~~ **DONE (2026-09-16)**:
   calendar, appshortcut, mirror all converted (Batch 1 complete). The engine
   gained `Dots`, `Row.bottom`, negative-dim fills, `Hit.glyph`,
   `Header.title_color`, `GridCell.hover`/`truncate`, `SelBg` — details in
   CHANGELOG. Next in the progression: **Batch 2** (Rows/Spark cards:
   procmon, journaltail, systemdunits, smarthealth, sensors, thermbl_zones,
   conninfo, worldclock, fans, diskio, network, topproc) — most need only
   `Rows` + `Spark` + `TabRow` bindings that already exist; add features one
   at a time, tests per feature.
3. **Transient popups/OSDs last** — after every fixed-size panel is
   declarative, the Ink fallback can be removed.

### Gotchas worth re-reading before starting
- `Rows.w` = inner box width (pad applies both sides); `Rows.pitch` =
  stride (46) ≠ `row_h` (40); `row_dy` = cell baseline within row.
- `Hit.right: x` = margin of the box's RIGHT edge from card right.
- `Surface`/`Text` `center_x`/`center_y` = the x/y fields become base-px
  offsets from the PARENT BOX's center (the lock `lhero` component anchors the
  fullscreen hero this way — RON must still list `x` explicitly: RON requires
  non-`serde(default)` struct fields, so `x: 0.0` (and `y`) are mandatory).
- `Rows` ignores `x` (uses `pad`); shell-surface coordinates are base px from
  the surface origin, identical space to the Rust layouts.
- Scene values are the ONLY place per-frame dynamic colors/labels may be
  computed (`scene_values()`); tokens/strings in `.ron` are static.
  `settings_nav` + `{recent_*}` are the reference implementation.
