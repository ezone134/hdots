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
