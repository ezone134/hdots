# Changelog

## 2026-09-17 (Batch 2 tail — session paused mid-conversion)

- **`GridCell.sub_dy`/`sub_size`**: per-cell nudge + size override for the
  second line (thermal zone chips: zone caption + temp — the Rust drawer's
  cy+3 / cy+12 rows ≈ centered label + sub at dy −1, size 9). `thermal_cells`
  now publishes the full parity recipe (sub + warning-ladder sub_color +
  sub_dy −1 + sub_size 9).
- **Remaining-Batch-2 bindings all landed in scene_values**: `jr_tail_rows`
  (newest-first 5-row journal tail, prio→Danger/Warn/Fg2 ladder) + `jr_empty`;
  `sus_rows` (newest-first raised chips, danger name + ✕) + `sus_empty`;
  `sm_rows` (mono dev / dim model / temp warn≥55° / ✓✕ status glyph) +
  `sm_empty`; `thermal_cells`/`thermal_empty`; `conn_rows_v`/`conn_empty`
  already existed. Header metas (`jr_meta`/`sm_meta`/`sm_icon`/`sus_meta`/
  `sus_icon`) were in from earlier passes.
- Tests: 258/258, bin builds clean.
- **Paused here**: the 5 RON scenes (journaltail, systemdunits, smarthealth,
  thermal_zones, conninfo) not yet authored — worldclock deliberately stays
  Rust (stateful search overlay/hover-✕). Full state in NEXT_STEPS.

## 2026-09-16 (Batch 2 Spark trio — network/diskio/sensors fully declarative)

- **`net_meta` live value**: the network graph's y-axis caption (the Rust
  drawer's exact bytes→bits ladder: Mbps/Kbps/bps against the deque peak),
  empty until two samples. The `diskio` legend colors bind plain `info`/
  `acc` tokens (no new values needed).
- **Sensors live values**: `sensors_none`/`sensors_tilt`/`sensors_compass`/
  `sensors_gyro` presence gates + `sensor_tilt`/`sensor_compass` (octant
  gauge glyph baked into the string, Rust's exact 8-bucket ladder)/
  `sensor_gyro` (mono triplet) — all computed in `scene_values` with the
  drawer's byte-for-byte formats.
- **network / diskio / sensors converted (zero Ink)**: network = Header
  (live y-axis caption) + dual pre-normalized Sparks (down = info, up =
  accent, negative-h fill stopping above the speed row) + a bottom-anchored
  Row of ↓/↑ glyph+mono speed groups (Rust's 26px gap); diskio = R/W legend
  dots at the exact Rust insets + title-only Header + dual Sparks;
  sensors = accent-glyph Header + one row per exposed sensor (gated, right
  values, compass accent) + centered empty caption. Covered by
  `committed_batch2_spark_scenes_parse_zero_ink` (zero-Ink + both series
  bound); network left the Header-committed set accordingly.
- Tests: 258/258.

## 2026-09-16 (Batch 2 openers — procmon/fans/topproc fully declarative)

- **`Rows` per-row BAR cell (`SceneCol.bar: BarCellSpec`)**: the meter-row
  engine feature the Batch 2 list cards needed. A bar cell interpolates its
  cell text (`{name}`) and parses it as a fraction of `max` (default 1.0),
  drawing a track across the cell box with the fill hugging the left —
  honoring `x`/`w`/`right`/`edge` like every cell. `fill` defaults to the
  cell's resolved color (so a per-row `col_colors` ink drives the fill),
  `track` defaults to `hover`. Spec: `height`/`top`/`radius`/`max`/`fill`/
  `track`. Covered by `rows_bar_cells_parse_value_fill_and_track`,
  `rows_bar_cell_zero_width_draws_degenerate_track`,
  `rows_bar_cell_draws_track_and_fraction_fill`.
- **Live row bindings upgraded**: `top_procs_rows` now carries
  label · tick-fraction (of the top process, the drawer's implicit 100%
  reference); `fans_rows` carries label · rpm · slow-peak-decay fraction with
  per-column inks (label fg2, rpm fg/fg3 by liveness) + the `fans_empty`/
  `fans_live` gates.
- **procmon / fans / topproc converted (zero Ink)**: procmon = Header (live
  "CPU {cpu}%" mono meta) + truncated label + right-anchored accent meter;
  fans = Header + centered empty caption (`fans_empty`) + rows gated on
  `fans_live` with edge-anchored info bars; topproc = Header + 26px rows with
  track-toned neutral bars (Rust parity: it drew track-only). Covered by
  `committed_batch2_bar_row_scenes_parse_and_drive_rows` (zero-Ink + Rows +
  bar cell per scene); `fans` left the Header-committed set accordingly.
- Tests: 257/257.

## 2026-09-16 (Dashboard viewing strip CHIPS declarative — BannerRow)

- **`SceneValue::BannerCells` + `BannerCell`/`BannerCellKind`**: the typed
  data binding for the strip — per-frame geometry from `banner_cells(w)` (the
  exact function the Rust drawer hit-tests with, so scene and input can never
  disagree), a live **ink name** per chip (each `bchip_*` binding mirrors its
  Rust drawer arm's color ladder: wifi accent-while-named/dim-off, battery
  danger-under-20 unplugged, cpu/ram threshold ladders, dnd bell accent,
  conn/offline dim, workspaces hover→acc), the pre-computed label, optional
  glyph, tray icon keys, the L/C/R zone, the paint recipe, and the slot key.
- **`SceneItem::BannerRow`**: paints the cells with the Rust drawer's exact
  recipes — text bias (`TEXT_Y_BIAS`), icon@12/label@29 insets, ink-centered
  icons (settings/power/search/dnd at size 14), brand-font mark, dim seps,
  tray icons right-aligned with per-icon 20px sub-regions (keys 30+ti, SNI
  activate preserved), whole-strip overflow scissor at the band box and the
  per-zone scissor state machine (transitions interleave with the cell sweep,
  zone windows clipped independently). Covered by `banner_row_paints_cells…`,
  `banner_row_overflow_clips_at_the_band`,
  `banner_row_zone_scissors_interleave_between_cells`,
  `banner_row_visible_gate_and_missing_binding_draw_nothing`.
- **`publish_banner_cells(&mut vals, w)`** (banner.rs): builds the cell list
  per frame and seeds `dash_chips_via_scene`; skipped in edit mode (DashToggle
  skipped so keys stay put). `layout_banner`'s Rust chip sweep + the viewing
  connectivity popover stand down while the flag is up (edit chrome — guides,
  grips, tray — always stays Rust). The `vals` gate in `layout_expanded` now
  also builds values when the `dashboard` surface exists (pure-Rust board +
  declarative strip).
- **`dchips` declared in shell.ron**: `BannerRow(cells: "banner_cells_v",
  x:0, y:12, h:26, visible: banner_shown, overflow: banner_overflow, zones:
  banner_zones)` — authored after the `dash_banner` Ink so it rides the same
  dispatch slot AFTER the painter: grid → band → Rust banner Ink (edit chrome /
  suppressed chip loop) → declarative chips. Both config copies in sync.
- Tests: 253/253.

## 2026-09-16 (Dashboard viewing strip band declarative — z-order slot engine)

- **Ink `pre` z-order slots (`SceneInk.pre`)**: the engine change the dashboard
  chrome needed. Items authored AFTER an Ink and BEFORE the next one no longer
  draw upfront — they attach to the FOLLOWING Ink's dispatch slot and draw
  immediately before its Rust painter. Declared chrome can now sit BETWEEN two
  Rust bodies in z-order (cards → band → banner chips). Items before the first
  Ink / after the last keep drawing in scene order. Covered by
  `items_between_inks_attach_to_the_next_ink`.
- **`Surface` per-corner radii (`tr`/`br`/`bl`, negative = concave)**: any
  non-zero corner radius emits `Cmd::RectConcave` (the band's top corners
  CONCAVE into the screen edge, bottom corners square). Covered by
  `surface_concave_corners_emit_rect_concave`.
- **Viewing strip band declared (`dband` in shell.ron)**: the opaque band that
  covers board slides scrolled under the banner now layers between the two
  Inks in the `dashboard` surface — `y: 12` (BASE_BANNER_Y), `h: 26`
  (BASE_BANNER_H), concave top corners, `value("dash_opaque")` fill (opaque
  theme bg, new SceneColor), gated on `banner_shown` (new Toggle =
  `!banner_collapsed()`, the empty-strip collapse keeps working). The Rust
  band inside `layout_dash_grid` skips itself while `dash_band_via_scene` is
  up (set BEFORE the surface consult — the `dash_grid` Ink runs the grid
  painter — and reset when the surface is unavailable), so there is never a
  double paint. Edit-mode chrome mask is unrelated and stays Rust.
- Tests: 249/249.

## 2026-09-16 (Calendar + Apps + Mirror fully declarative — Batch 1 complete)

- **Build restored first**: the tree held a half-spliced scene-values edit
  (7 compile errors — `DynColor::named`, `ColorToken::SelBg`,
  `SceneRow::default`, `ICON_APPS.into()`), finished now: `DynColor::named()`
  for 'static names, `ColorToken::SelBg` (10% accent selected-row wash, was
  only a `ui::sel_bg` free fn), `Default for SceneRow`.
- **Engine additions for the three conversions** (all follow the existing
  Grid/Rows patterns, tests per feature):
  - `SceneItem::Dots` — pagination dots below a swipe-cycled card (the HARD
    RULE, previously Rust-only `pane_dots`): `active` (Ring pane index),
    `panes`, `step`, `dy` (inset from card bottom), `when` (visibility
    toggle). One dot per pane, active = accent, others hover — parity with
    `pane_dots`.
  - `Grid` negative `w`/`h` reserve from the opposite edge (the appshortcut
    body insets `w: -24`, `h: -46`); `Image` negative `w`/`h` likewise (the
    camera frame stops above the dots via `h: -98`).
  - `Row.bottom` — anchor the row's bottom edge `y` px above the card bottom
    (the mirror Photo/Record button bar); requires declared `h`.
  - `Row`/`Column`/`Stack`/`Toggle` gained `visible: Option<String>` pane
    gates (mirror panes flip items on/off without a Rust drawer).
  - `Hit.dy` label nudge, `Hit.glyph`/`glyph_pad`/`glyph_color`/
    `glyph_color_hover` — leading-glyph icon buttons (mirror Photo/Record);
    `Header.title_color` (calendar month in accent); `GridCell.hover` (day
    ink lifts to fg, today keeps acc; plus hover keeps its AccTint surface
    via `surface_hover`); `GridCell.truncate` (app-tile name clip parity,
    Rust math `(tw − 4)/5.5` chars, computed from the tracked card rect).
- **scene_values**: `cal_cells` (42 cells + hover/today parity), `cal_month`,
  `app_cells` (8 tiles + clip), `app_count` (`n/8` header meta),
  `app_tile`/`app_tile_empty` per-frame resting tile colors (dynamic pool),
  `mirror_pane_0/1`, `mirror_pane` (Ring), `mirror_hover` (shared with the
  Rust drawer via new `Shell::mirror_dots_hover`), `mirror_rec_bg`
  (`mix(RED, hover_hl, 0.15)` while rec), `mirror_fps_rows` (active row
  `SelBg` + real-fps suffix), `mirror_rec_label`, `mirror_show_fps`.
- **draw_card_scene** now tracks `mirror_rect` + `app_shortcut_rect` for
  scene-owned cards (wheel pane-flip and the Rust remove-corner math stay
  live without the Rust drawers).
- **calendar.ron / appshortcut.ron / mirror.ron converted** (zero Ink for
  calendar + appshortcut; mirror keeps the resident fps-row key dispatch but
  draws everything declaratively): calendar = `Header` (`{cal_month}`, acc
  title) + prev/next hover-only chevron `Hit`s (30100/30101) + 7-col `Grid`
  with weekday heads and `key_base: 30000`; appshortcut = `Header` (`n/8`)
  + square `Grid` (keys 34400+i, hover-✕ remove 34416+i); mirror = pane-0
  camera `Image` + no-camera fallback `Text` + bottom `Row` of two equal-split
  glyph `Hit`s (30700/30701) + pane-1 fps `Rows` (30710..) + "Show real fps"
  `Text`+`Toggle` (30704) + `Dots` (hover-gated). Repo copies live in
  `config/zen-shell/ui/cards/`, installed to `~/.config/zen-shell/ui/cards/`.
- **Batch 1 of the card conversions is COMPLETE** — all 17 text/list/grid
  cards now draw from RON. Remaining: Batch 2 (Rows/Spark: procmon, sensors,
  fans, …), Batch 3 (Fader/Ring/Toggle), Batch 4 (special), then banner
  chrome + transient popups.
- Tests: 247/247 (was 245 pre-session; added `live_grid_card_scenes_parse`,
  `dots_render_pane_rule_and_hover_gate`, `grid_cell_truncate_clips_long_labels`;
  removed one stale duplicated `#[test]` attribute the handoff tree carried).

## 2026-09-16 (Lyrics fully declarative — karaoke word highlight)

- **Karaoke engine (`Rows.karaoke` + `Rows.karaoke_fs` + `SceneValue::Karaoke`)**:
  the word-level playhead highlight the lyrics card needed, mirroring the Rust
  drawer byte-for-byte. `SceneValue::Karaoke(Vec<KaraokeLine>)` binds per-line
  `(words, active)` state alignined by visible-index against the parallel
  `Rows` value; `Rows.karaoke: Option<String>` names that binding. When each
  row draws, its karaoke line short-circuits the normal cell machinery: the
  ACTIVE line washes accent (r4), then walks its words left-to-right with
  auto-fitted `est_w` spacing, clipping at the card edge — the playhead word
  renders white (`0xffff_ffff`) with a 1.5 px underline, the rest near-black
  (`0x1d1d_1d`); the accent wash plus subtle clamped advance recreates the
  Rust lyric fade. Inactive rows draw the joined whole line, lifting
  `hover → accent` ink with a hover surface to read like an active lyric;
  hovered rows draw a small hover background (the quiet hover tone). Plain
  (untimed) lyrics simply unbind `lyr_karaoke` and the Rows falls back to the
  regular single-column cell path — zero karaoke, zero regressions (verified
  by a dedicated test). Regions still anchor at `key_base + visible index`.
- **`Header.glyph_color: Option<ColorToken>`** — per-header note-glyph tint
  (default Fg2), so the lyrics header can paint its `\u{f001}` notes glyph
  accent like the Rust card. serialization `glyph_color: Some(acc)`.
- **Lyrics card converted with zero Ink** (`lyrics.ron`, mirroring Rust):
  `Header` (accent note glyph, title); the dynamic state chip as three
  left/right-anchored gated `Text`s — `synced`/acc, `none`/danger, and
  `{lyr_status_other}`/fg3 for fetching…/plain/— (exclusive toggles
  `lyr_status_synced`/`lyr_status_none`/`lyr_status_other_on`); the track
  caption (`{lyr_track}`); the centered fallback message (`{lyr_msg}` behind
  `lyr_empty`) for "fetching lyrics…" / "no lyrics found" / "no track
  playing"; and the karaoke `Rows` (`y: 33`, `row_h: 21`, `r: 4`,
  `hover_surface: hover`, `start: "lyr_scroll"`, `key_base: 33500`,
  single full-line col). New values: `lyr_state`/`lyr_synced`/`lyr_none`/
  `lyr_other_lbl` (+ Text var `lyr_status_other`), `lyr_track`, `lyr_msg`,
  `lyr_empty`, `lyr_scroll` (Ring), `lyr_karaoke` (Karaoke, only bound when
  `state == 2 && has_timing`, active word from `crate::lyrics::active_word`),
  and `lyr_rows` (whole-line cells). `draw_card_scene` tracks `lyrics_rect`
  and keeps `lyrics_follow` driving the ring window live.
- Repaired a scene-values splice that had broken the `recent_rows` closure
  (mismatched delimiter) before shipping.
- Tests +3 (241/241): engine karaoke (accent wash + white live word +
  underline geometry + near-black caret words + joined-line hover lift +
  region keys), the karaoke-unbound plain fallback (fg idle cell, hover lift
  + accent tint, no stray word draws), and the live `lyrics.ron` parse (fully
  declarative, accent note glyph, synced/none chips, Rows geometry/keys/
  karaoke binding/col hover). Bin clean (input.rs:512 only).

## 2026-09-16 (Snippets fully declarative — flash + SceneCol.flash)

- **Transient flash (`Rows.flash` + `SceneCol.flash`)**: `Rows` gains
  `flash: Option<String>` — a `Ring` binding whose scalar is the flashed
  row index + 1 (0 = no flash so index 0 stays representable). When the
  scalar matches a visible row, a translucent-accent surface paints that
  row, and every column that opts into `SceneCol.flash: Option<ColorToken>`
  inks its cell in that accent token (the snippets copy glyph + name turn
  accent while the dim preview stays fg3). The flash wash overrides both the
  normal hover surface and hover ink, mirroring the one-second "just snapped"
  of the Rust snippets drawer. Scene-col case `flash: Some(acc)`.
- **Snippets card converted with zero Ink** (`snippets.ron`, mirroring Rust):
  `Header` (`{snip_meta}` clip count in fg, no glyph); a `Composer` field
  (`snip_buf` buffer / "add snippet — name text to copy" placeholder,
  `snip_focus` toggle; keyed SNIPPET_INPUT_KEY); the clipboard `Rows`
  (col 0: copy glyph icon fg2/flash acc, col 1: name fg/flash acc, col 2:
  body preview fg3; `hover_surface: hover_hl`; `flash: "snip_flash"`,
  key_base SNIPPET_COPY_BASE, del_base SNIPPET_DEL_BASE, `del_pad: 20`,
  `del_dy: 4`). Values: `snip_meta`, `snip_buf`, `snip_focus`, `snip_flash`
  (Ring, `elapsed < 1 s`), `snip_rows` (glyph · name · body). Copy/clear
  keys route through the resident handler. 238/238 tests, bin clean
  (input.rs:512 only).

## 2026-09-16 (News fully declarative — scrollable Strip + list hover ink)

- **`SceneItem::Strip`** — a horizontal scrollable chip row (the missing
  Batch-1 engine gap). Chips come from the new `SceneValue::Chips(Vec<String>)`
  binding, the active index from a `Ring` scalar (`sel`), and the horizontal
  scroll offset in base px from another `Ring` (`scroll`, clamped to the
  strip's own content overflow at draw time). Each chip auto-fits its label
  (`chip_pad + 0.62·fs·chars + chip_pad`), skips offscreen, lifts
  `hover → hover_hl`, and paints the selected chip accent with an `Sfg`
  label — byte-for-byte the Rust news category strip (keys `key_base + i`).
  The wheel handler clamps `news_cat_scroll`, so stale values can't push
  chips off.
- **Three `Rows`/`SceneCol` list-ink primitives**: `SceneCol.hover` — a
  per-column color applied only while its ROW is hovered (takes precedence
  over `color`, so the news title tinting accents on hover while the source
  label stays fg3); `Rows.hover_bar` — the 2.5 × (`row_h − 6`) rounded accent
  rail at the hovered row's left edge (byte-match of the Rust drawer);
  `Rows.hairline` — a 1 px hover-gray divider under every visible row.
- **News card converted with zero Ink** (`news.ron`, mirroring Rust): `Header`
  (`\u{f1ea}` glyph, label = `{n} stories` or the active category, empty when
  inert); the `Strip` (`news_chips`/`news_cat_sel`/`news_cat_scroll`, y 30,
  key 13000); the centered fetching / no-feeds message gated by `news_empty`;
  and the headline `Rows` (40-char title · right `ICON_OPEN` glyph when the
  story is clickable · right source label on a second baseline, `y: 59`,
  30 px pitch, `key_base: 13300`, `start: "news_scroll"`, `hairline` +
  `hover_bar`). New values: `news_label`, `news_msg`, `news_chips` (Chips),
  `news_cat_sel`/`news_cat_scroll`/`news_scroll` (Ring), `news_empty`
  (Toggle), `news_rows` (title/open-glyph/source, 40/16-char truncation).
  `draw_card_scene` tracks `news_rect` so wheel scrolling (list + strip)
  stays alive. No `scene_owns_*` gating — the scene is dispatched
  whole-card.
- Tests +2 (236/236): engine (strip chips + active accent/Sfg + offscreen
  skip + row regions; row hover rail/hairline/title-tint and idle fallback)
  and the live `news.ron` parse (fully declarative, glyph, strip bindings,
  Rows geometry/keys, col hover). Bin clean (input.rs:512 only).
  `Strip` + `SceneCol.hover` pre-unlock chips/hover lists in later cards.

## 2026-09-16 (Expenses fully declarative — rows of chips, reserve+visible)

- **Expenses card converted with zero Ink** (`expenses.ron`, matching the Rust
  drawer byte-for-byte): `Header` (title + MTD total in accent mono via
  `meta_color`); a **`Row` of six equal-split `Hit` chips** (1/5/10/20/50/100,
  keys 31800..31805, `surface: hover` lifting to `hover_hl`); the composer row
  — a `Composer` (`pad: 0` so the row pad supplies the inset, `focus:
  exp_focus`, key 31810 — Enter commits via the resident key handler) plus a
  fixed-70 `Hit` "clear mo" that flips `fg2 → danger` on hover (key 31811); and
  the top-3 per-category bars (`caption + Bar + right mono amount`, each group
  gated by an `exp_has_n` Toggle) with the "nothing logged this month" caption
  gated by `exp_empty`. Bare values in `scene_values`: `exp_meta`,
  `exp_buf`, `exp_focus`, `exp_empty`, `exp_cat_i`/`exp_val_i`/`exp_bar_i`
  (ratio vs the top category), `exp_has_i`. No `scene_owns_*` gating needed —
  a scene without `Ink` is dispatched whole-card, so the Rust drawer never
  runs.
- **Four tiny primitives landed for it**: `Header.meta_color` (meta ink that
  defaults to `fg3`, no-op elsewhere); `Hit.color_hover` (label ink swapped
  while hovered — pre-unblocks the news title hover); `Text.visible` and
  `Bar.visible` (bound `Toggle` gates — state-swapped labels/bars);
  `Bar.reserve` (`w: 0` bars span `card_w − reserve` instead of the full width,
  so the amount column at the right edge stays clear).
- Tests +2 (234/234): live `expenses.ron` is fully declarative (no Ink,
  header meta binding/color, chip keys + amounts, composer pad/key/focus,
  clear hit geometry + danger hover, reserve/visible on the three bars);
  engine test for reserve geometry, accent meta ink, the visible gates, and
  the hover color swap. Bin clean (input.rs:512 only).

## 2026-09-16 (Quote body declarative — multiline wrap + visible gate)

- **`SceneItem::TextWrap`** — a multi-line text block that wraps at 34
  chars/line (≤ 136 chars total, matching the Rust Quote drawer exactly), with
  every line horizontally centered, vertically centered inside a box bounded
  by `top`/`bottom` card insets (base px, scaled). An optional `caption` line
  (dimmer, smaller) trails the block — the "— author" line; both `text` and
  `caption` interpolate `{var}`s and skip drawing when empty after
  substitution. `visible: Option<String>` (bound `Toggle`) gates the whole
  block on/off without reserving space, swapping the body in against
  the Rust fallback messages (the artist swaps the state branch at runtime).
- **`scene_owns_rows` also detects `TextWrap`** (so the Rust quote drawer
  skips its own body wrap when the scene owns it, while keeping the two
  fallback message states and the title/meta/pills as resident Ink).
- **Quote split** (`quote.ron`): the wrapped quote text + author caption is a
  `TextWrap` item (visible when `quote_has`, top 28 clears the header,
  bottom 36 clears the pills); title/meta + the two pills (refresh
  always, save conditional) stay the `quote` Ink. `draw_quote_card` gates its
  body with `!self.scene_owns_rows`; the flash decrement stays inside the
  Ink. New values: `quote_text` (the raw text), `quote_author`, `quote_has`
  (Toggle of `state == 2 && !text.is_empty()`).
- Tests +2 (232/232): live `quote.ron` declares the `TextWrap` body with Ink
  (visible/top/bottom), wrap-at-34 lines + author caption drawn, and
  `visible=false` suppresses all output. Bin clean (input.rs:512 only).
  `TextWrap` pre-unblocks **lyrics** (multiline, partial) and simplifies
  future conditional body blocks.

## 2026-09-16 (Countdown list body declarative — chip cells + truncation)

- **`SceneCol` gained three list primitives** (RON-only serialization, no
  `SceneRow` churn): `truncate: Option<f32>` (cut a cell to fit
  `card_w − 2·pad − truncate` px at 0.62·fs px/glyph, ≥ 8 chars — the Rust
  drawers' `avail` reservation so names stop before the right-side chip);
  `edge: Option<f32>` (right-edge anchor at `card_w − edge` base px, survives
  card resize); and **`pill: Option<PillSpec>`** — a chip cell: rounded rect
  auto-fit to its text (0.62·fs px + 2·`pad`), fixed `height`/`radius`/`top`,
  fill derived from the cell's text color (Acc/AccTint → accent tint,
  anything else → hover) or overridden by `bg`. The chip texts' per-row ink
  comes from `col_colors` as usual. `del_anchor: bool` moves a row's revealed
  ✕ (and its full-row region) left of that col's box — the chip-relative
  deletes in the countdown drawer.
- **Countdown split** (`countdown.ron`): the event list is now a `Rows` item
  (`countdown_rows`: label `truncate: 120` · date caption · right `pill`
  chip, `y: 50` under the top composer, 24 px pitch, 12 pad, `del_base:
  COUNTDOWN_DEL_BASE`/`del_pad: 20`/`del_dy: 4` anchored left of the chip via
  `del_anchor`, key-less reveal-only-on-✕ like the Rust drawer). Title/meta +
  the add-event composer row stay the `countdown` Ink; the Rust list loop is
  gated by `scene_owns_rows`. New `countdown_rows` value builds chip
  text/tokens (`today!`/`passed`/`Nd`) + per-cell colors (Acc/Fg3/Warn/Fg2).
- Tests +3 (230/230): pill chip auto-size + derived fill (Warn→hover, Acc→
  tint) + centered glyph + `truncate` cutting, `del_anchor` ✕/region left of
  the chip, live `countdown.ron` declares the Rows list (truncate/pill/edge/
  del anchors + del keys) while keeping its Ink. Bin clean (input.rs:512
  only). The pill/edge/truncate primitives pre-unblock **news** (chips +
  two-line headlines), **expenses** (amount chips), **quote** (action pills).

## 2026-09-16 (Notes list body declarative — two-baseline rows + composer gate)

- **`SceneCol.dy: Option<f32>`** (base px, serde default → falls back to the
  Rows `row_dy`): per-column baseline offset lets one row carry a title +
  dim caption line beneath it. Pre-unblocks the news/lyrics/countdown cards;
  `SceneCol` and `Rows` are RON-only lists, so adding fields is churn-free.
- **`Rows.visible: Option<String>`** (`Toggle` value; when some and bound to
  `false` the whole list draws nothing — the arm skips the row loop instead
  of `return`, so later items in the same scene such as a Composer still
  draw). This models the Notes composer swap: the list hides while the shared
  title/body composer is open, so it can never draw over the input.
- **Notes split** (`notes.ron`): the note list is now a `Rows` item
  (`notes_rows`: bullet glyph tint col · title 9.5 · body caption 8 fg2 at
  `dy: Some(12)`, `y: 32` under the header, 24 px pitch, 12 pad, `row_dy: 0`,
  `start: Some("notes_scroll")` for the wheel window, `visible:
  Some("notes_composing")`). `draw_notes_card` keeps header/`+`/empty-hint
  chrome plus the full composer as Ink, and gates its own row loop with
  `scene_owns_rows`. New values: `notes_rows`, `notes_composing` (Toggle of
  `notes_input.is_some()`), `notes_scroll` (Ring).
- Tests +3 (227/227): `SceneCol.dy` baseline math, `Rows.visible` hiding the
  list while later items keep drawing, live `notes.ron` declares the Rows
  list (dy/title/body cols, scroll + composer toggle) while keeping its Ink.

## 2026-09-16 (Alarms list body declarative + key-less ✕ reveals)

- **`Rows` ✕ reveal no longer needs row keys**: the del-region guard dropped
  its `key != 0` clause — cards whose rows carry no clickable row still get
  the hover ✕ (revealed only while the ✕ itself is hovered, exactly like the
  old Rust drawer). New `del_dy` (base px glyph offset in the row; To-Do stays
  1.0, alarms 4.0) joins `del_pad`/`del_size`/`del_base`.
- **Alarms split** (`alarms.ron`): the alarm list is now a `Rows` item
  (`alarm_rows`: mono accent time + fg label columns, `y: 50` under the add
  row, 24 px pitch, 12 pad, `row_dy: 3`, `del_base: ALARM_DEL_BASE`,
  `del_pad: 26`/`del_dy: 4` — pixel-parity with the Rust drawer), gated by the
  existing `scene_owns_rows`; `draw_alarms_card` keeps title/`{n} set`
  meta/count + the flat add-input row as Ink chrome. Same del keys/geometry;
  no row-level regions (parity: the alarms list was click-free). New
  `alarm_rows` value in `scene_values()`.
- Tests +2 (224/224): key-less ✕ reveal (`del_pad`/`del_dy` anchored, no row
  hits), live `alarms.ron` declares the `Rows` list (del base/margins, mono
  accent time col) while keeping its Ink. Bin clean (input.rs:512 only).

## 2026-09-16 (Rows checkbox cells + hover-✕ delete + To-Do checklist body declarative)

The To-Do checklist body joins the declarative bed — first card to split its
Ink three ways (title/meta Ink + `Rows` body + `Composer` strip).

- **`Rows` gains a live checkbox column** (`check: Some("name")` +
  `check_x`): every visible row draws its leading 11² checkbox from the
  parallel `Checks(Vec<bool>)` value (indexed like the full `Rows` list, so
  windowed rows stay consistent). Checked = acc-tint well + `ICON_CHECK` in
  accent, unchecked = stroke-only outline with the hover-aware grey (exactly
  the Rust To-Do box). New `SceneValue::Checks` + `SceneValues::checks()`.
- **`Rows` gains hover-✕ delete** (`del_base`, `del_pad`, `del_size`): the
  hovered row reveals a right-anchored `ICON_CLOSE` at `card_w - del_pad`
  (fg, danger-red while its own del region is hovered — `ColorToken::Danger`)
  and registers the 22² corner region (`del_base + visible index`) ONLY while
  hovered, so it wins the row's corner exactly like the Rust drawers. `✕`
  shows while the row OR its ✕ is hovered.
- **To-Do card split** — the checklist is now a `Rows` item in `todo.ron`
  (`key_base: TODO_KEY_TOGGLE` remaps row keys to the visible window exactly
  as the Rust loop did; `del_base: TODO_KEY_DELETE`; `start: "todo_scroll"`
  keeps wheel-scroll alive; `check: "todo_checks"`) gated by the new
  `scene_owns_rows` (the `scene_owns_header`/`scene_owns_composer` pattern),
  so `draw_todo_card` draws only its title/count/`+N more`/empty-state.
  Same keys/geometry end-to-end: checkbox 14,1+; label 32; del at right-20.
- Tests +3 (222/222): checked/unchecked checkbox cells, ✕ reveal + conditional
  corner region (+ danger-Red on the ✕ itself), live `todo.ron` carries
  `Rows` (toggle/delete/check/scroll bounds) with its Ink + Composer. Bin
  clean (input.rs:512 only).

## 2026-09-16 (Composer scene item + To-Do card body split)

The first interactive card-body piece goes declarative: the bottom input strip
of the note cards is now `SceneItem::Composer`.

- **`SceneItem::Composer`** — the shared bottom-input chrome: rounded field
  (focused = stronger hover blend + 1.4px accent outline), `{var}` buffer with
  caret (width from char count, exactly the Rust math), placeholder while
  empty+unfocused, the "keyboard" typing hint on the right, and an optional ⊕
  add button that hides while focused. Binds `text` (`{var}`-interpolated) and
  `focus` (a `Toggle` name); **`bottom: true` anchors `y` from the card's
  bottom edge** (base px) so the strip stays pinned as the card resizes — the
  first bottom-anchored scene item. Registers the field (focus) + ⊕ regions,
  both with optional `SceneAction`s.
- **To-Do card split** (`todo.ron`): the checklist body stays the `todo` Ink
  (Rust), the composer is now a `Composer` item — `scene_owns_composer` (the
  `scene_owns_header` pattern) makes `draw_todo_card` skip its own strip
  (single draw + single regions, same keys 78/79). New bindings
  `todo_focus`/`todo_buf` published in `scene_values()`. Notes/Snippets (same
  strip shape) can adopt `Composer` next.
- Tests +4 (219/219): idle/focused composer rendering, bottom-anchored pins at
  two card heights, RON roundtrip of defaults, live `todo.ron` carries the
  Composer (keys 78/79) while keeping its Ink. Bin clean (input.rs:512 only).

## 2026-09-16 (dashboard surface LIVE in the viewing state)

`dashboard` was the last un-wired surface. Its two bodies now route through the
exact Rust painters via a chrome/body split — the board (`Ink("dash_grid")`,
extracted `layout_dash_grid`: dot lattice, empty hint, drag ghost, cards at
their packed placement with the fluid glide + `card_anim`/`edit_settling`/
`DASH_CARD_KEY` region side effects, tray-drag ghost, then the viewing strip
band) and the top banner strip (`Ink("dash_banner")` → `layout_banner`).

- **`DashDrawCtx`** (`panels/mod.rs`): a one-frame, borrowed context
  (`Layout` + drag cursor + scene values) handed to `draw_shell_surface` /
  `dispatch_shell_inks` / the new `dash` param; the dashboard never stores
  layout state (its `Layout` is pre-computed per frame anyway).
- **Consult gate** in `layout_expanded`: viewing state only (`!editing &&
  !sys_menu_open`). It zeroes the edit-only caches first (same as the old
  non-edit branch), calls the surface, draws `layout_dash_scrollbars` inline
  (V+H rails + thumbs, drag hit zones 500/501 still register), then returns.
  Edit-mode chrome (trays, ctrl rows, dot lattice, resize/✕ grips) and the
  system-card ⋮ menu stay Rust — frame-dynamic geometry, never dropped.
  The strip mask moved INTO `layout_dash_grid` (viewing case only) so the
  fallback path doesn't double-draw it.
- **shell.ron**: `dashboard` surface now declares `Ink("dash_grid")` +
  `Ink("dash_banner")`. The live-file test asserts that exact pairing.
- 215/215 tests, bin clean (only the pre-existing `input.rs:512` warning).

## 2026-09-16 (dashboard prerequisites: scene `Scissor` + banner tokens)

`dashboard` remains declared-but-unwired (the last surface); this pass landed
the two engine prerequisites the banner needs before its chrome can leave Rust.

- **`SceneItem::Scissor` + `SceneItem::ScissorEnd`** — the scene engine can now
  open/close rectangular clip regions (the strip-zone / scroll-cut primitive
  missing from scenes). `w: 0` / `h: 0` axes span the parent box; items emit
  `Cmd::Scissor`/`Cmd::ScissorEnd`, feeding the existing live scissor stack in
  `src/app/mod.rs` (the banner's per-zone windows already use it). Inert for hit
  testing; component inlining offsets Scissor like other positioned items.
- **Banner live tokens in `scene_values()`** — the named `banner_guide` /
  `banner_guide_dim` drop-guide inks (active = mix(hover, acc, .35) under a
  lifted token, idle = mix(hover, fg, .10)) plus the state booleans
  `banner_editing`, `banner_drag`, `banner_zones` (zoned clip active),
  `banner_collapsed`, `banner_overflow` (whole-strip scroll mode). A future
  declarative banner binds these via `value("banner_guide")` /
  `Toggle(name: "banner_*")` while Rust keeps computing the guide + zone
  geometry.
- **Tests**: scissor emits balanced clip commands at scaled coords; zero-dim
  scissor spans the parent box; scissor RON roundtrips; banner tokens resolve
  through the normal `value(...)`/`Toggle(name)` paths with the exact
  `scene_values()` inks. **215/215 pass**, bin build clean (only the
  pre-existing `input.rs:512` warning).
- `layout_expanded` still runs its own Rust banner/grid unchanged — no visual
  or behavior change this pass (the split happens once the banner chrome leaves
  Rust, per the build order below).

## 2026-09-16 (second pass: `lock` + `wallpaper` surfaces LIVE)

- **`lock` surface LIVE** (at the 1920×1080 reference height): `layout_lock`
  split into hero chrome + `layout_lock_body` (password field + auth error/
  caps + hold-power buttons + hint) routed through `Ink("lock_screen")`. The
  declared chrome — fullscreen dim (`value("lock_dim")`, the 0x00000059
  backdrop), hero clock (`{clock}`), date (`{lock_date}`, zero-padded `%d` day
  matching the Rust format), avatar disc (`value("lock_avatar")` =
  mix(hover, acc, .35)), `{user_init}` first-letter initial + `{username}` —
  is center-anchored to the fullscreen surface via new `Surface center_x`/
  `center_y` fields (mirroring `Text`; offsets are base px from the box mid).
  The surface is consulted only when the monitor height is ~1080 (`h ± 24`);
  any other size runs the built-in Rust layout unchanged. The body owns all
  its hit regions (keys 1–4), the scene owns none.
- **`wallpaper` surface LIVE**: `layout_wallpaper` split into header chrome
  (back button `Hit` key 1 with a hover-only fill the Rust header lacks,
  `\u{f053}` glyph + "Backgrounds" title, byte-for-byte positions) +
  `layout_wallpaper_body` (empty state + thumbnail grid + scrollbar) routed
  through `Ink("wallpaper_picker")`. `layout_wallpaper` now takes `h` to
  consult/route the surface.
- **`dispatch_shell_inks`** gained `lock_screen` → `layout_lock_body` and
  `wallpaper_picker` → `layout_wallpaper_body` arms; only `dashboard` remains
  un-wired (banner needs live-scissor/drag tokens).
- **`scene_values()`**: new `lock_dim` + `lock_avatar` dynamic colors
  (`SceneColor::Raw` — derived blends tokens can't express), `user_init` and
  `lock_date` text values (distinct from the shared `date` `%e`-formatted
  value cards use).
- **Tests**: new `surface_center_anchors_to_box_center` (0-dim spans the box +
  center offsets + raw dynamic color); `shell_ron_live_file_parses_and_has_surfaces`
  now asserts all four live surfaces carry their body Ink. **210/210 pass**
  (was 209), bin build clean (only the pre-existing input.rs:512 warning).
- Declared-lite deltas (noted, accepted): lock surface tops out at the 1080p
  reference (non-1080 heights use Rust); wallpaper back button gained a
  hover-only fill; lock wallpaper chrome uses static tokens for the hint-free
  hero (no dynamic hover tint on the hero clock).

## 2026-09-16 (dynamic tokens + `notif` surface LIVE)

- **Dynamic color tokens** (`ColorToken::Value`): new Copy `DynColor` in
  `src/ui.rs` (fixed 24-byte buffer, custom serde so RON stays natural —
  `value("dnd_chip")`), new `ColorTokenExt::resolve_with(pal, vals)` /
  `hover_variant_with(pal, vals)` in `scene.rs`. Every draw-path resolver site
  now threads the per-frame `SceneValues` (`surface`/`surface_hover`/`color`
  + `draw_text` across all 19 call sites), so a `value(name)` resolves
  dynamically each frame while `Token` colors still live-resolve against the
  theme (theme switches still apply).
- **`SceneValues` color pool**: `SceneColor::Token(ColorToken)` (re-derivable
  against the live theme at draw time) or `SceneColor::Raw(u32)` (computed
  threshold ladders / blends `mix` can't express), via `insert_color`/
  `color`. MISFIRE rule matches typed values: unknown name or wrong kind →
  `pal.fg` (never panics, never black).
- **`notif` surface is LIVE**: split `layout_notifs` into chrome (title +
  Clear chip, declarative `Hit` key 2) + `layout_notifs_body` (the card list
  + empty state, routed through `Ink("notif_list")`); `layout_notifs`
  consults the surface and falls back to the full Rust layout when it's not
  declared / not hittable. DND chip (key 3) is now fully state-driven
  declaratively: fill = accent when DND is on (hover whitens via
  `mix(acc, fg, 0.15)`), quiet hover surface when off, label `sfg` on accent
  / `fg` otherwise — all bound through `scene_values()` dynamic colors
  (`dnd_chip` / `dnd_chip_hover` / `dnd_chip_fg`, the latter a live
  `SceneColor::Token`). Chips rest `raised`/lift `raised_hl` on hover; labels
  left-anchored at the exact Rust insets (Clear: w−196 chip, 230 label;
  DND: w−104 chip, 324 label — the notif pane is a fixed 420-wide surface).
- **Tests**: +5 in `scene.rs` (value-token RON roundtrip, token/raw/unknown
  resolution, hover invariance). 209/209 pass, bin build clean (only the
  pre-existing input.rs:512 warning).
- Declared-lite deltas (noted, accepted): chips omit the hairline outline;
  the DND chip background stays flat (no hover/drag tint beyond the guarded
  `surface_hover` blend).

## 2026-09-15 (whole-shell declarative: `shell.ron` + settings live-cut)

- **Whole-shell declarative engine** (`scene.rs`): `SurfaceScene { name, items }`,
  `ShellScene { components: HashMap<String, Vec<SceneItem>>, surfaces }`,
  `SceneItem::Comp { name, x, y }` (template instantiation — inlined at load
  time, translated by `(x, y)` base px; components may nest), and
  `ShellSceneCache` (mtime hot-reload, mirrors `SceneCache`). A declared
  surface replaces its Rust draw path when wired; no file → every surface
  still runs its built-in Rust layout unchanged.
- **`vars::shell_scene_path()`** → `~/.config/zen-shell/shell.ron`.
- **`draw_shell_surface(name, v, w, h, pal, vals) -> bool`** (panels/mod.rs):
  draws a declared shell surface via `CardScene::draw`, registers hits, and
  routes `Ink{name}` (now with `{var}` interpolation) to
  `dispatch_shell_inks` — same plumbing as card scenes, elevated to surfaces.
- **`Rows` engine additions**: `pitch` (stride override — 46 px nav over a
  40 px body), `w` (bounded row box), `row_dy` (cell baseline), `r` (row
  corner radius). **`Hit`** gained `hover_only` (surface fill only while
  hovered — the inline-appear controls). `NAV_*` settings keys made
  `pub(crate)` so `scene_values()` can reference them.
- **`settings` surface is LIVE** in `shell.ron`: Settings title + hover-✕ close
  + 1 px sidebar rail + 180 px nav `Rows` (per-row `surface`/`col_colors` for
  selected/hover from `scene_values` `settings_nav`) + `Ink(name: "{settings_ink}")`
  body. The declared `dashboard`/`notif`/`lock`/`wallpaper` surfaces are
  present but not yet wired into their mode layouts.
- New tests: `shell_scene_parses_and_expands_components` (RON map + Comp
  offset + nesting), `shell_ron_live_file_parses_and_has_surfaces` (live
  canonical file parse — skips when no file in the test env).
- 204/204 tests pass, bin build clean (only input.rs:512 pre-existing).

## 2026-09-15 (scene_values built once per frame)

- `DashCard::draw` now takes a `&SceneValues` argument instead of building the
  whole value pool itself — the full map was being re-formatted once PER CARD
  PER FRAME in `layout_expanded` (one `scene_values()` call per drawn card).
- `layout_expanded` (`src/shell/panels/dashboard.rs`) builds the value pool
  ONCE before the card loop and threads `&vals` into every card draw (packed
  grid + dragged-from-tray card), so N cards cost 1× pool build instead of N×.
- Dirty gate: when no card actually on screen has a `.ron` scene (checked via
  `scene_cache.get()` on the packed cards + the edit drag target), the pool
  build is replaced by an empty `SceneValues` — pure-Rust-card boards skip the
  formatting work entirely on frames where nothing scene-y draws.
- Rust drawers are unaffected (they never consumed the pool); 202/202 tests
  pass, bin build clean (only pre-existing input.rs:512 warning).

## 2026-09-15 (Batch 1: `recent.ron` lands declarative)

- **`recent` converted to a fully-declarative scene** (`ui/cards/recent.ron`):
  `Header` (meta `{recent_files_n}` "N files") + centered empty-state +
  `recent_rows` (name · open-glyph) with `start: recent_scroll` for the
  wheel-scroll window and `key_base: 14300` for the open-file click plumbing.
- **Hover tint via data**: the recent drawer tints the hovered row's name +
  open glyph accent; `scene_values()` now computes `col_colors` per row from
  `hover_key` each frame so the scene matches it exactly.
- **Live data without a Rust body**: the drawer's 30 s stale-`read_recent_files`
  refresh moved into `refresh_recent_if_stale()`; `draw_card_scene` calls it
  for `recent` and tracks `recent_rect` so wheel scrolling + refresh survive
  the skipped drawer.
- **202/202 tests pass** (the committed batch-1 scene parse test now also
  covers `recent.ron`); bin build clean (only pre-existing input.rs:512).

## 2026-09-15 (Batch 1 first tranche: declarative list cards)

- **`Rows` engine extensions** (`src/scene.rs`) — the last four pieces needed
  to convert scrollable, stateful list cards to declarative RON:
  - `SceneRow.surface: Option<ColorToken>` — resting row tint (active /
    connected well) drawn behind the row, taking precedence over the hover
    lift exactly like the Rust list drawers.
  - `SceneRow.col_colors: Vec<Option<ColorToken>>` — per-cell color override
    that beats the row's own `color`, so a status column stays colored
    (chg% green/red, connected check) even on dimmed/active rows.
  - `Rows.start: Option<String>` — scalar-bound first-visible row. The list
    skips to the window and keys rebase to it (`key_base + visible index`),
    so wheel-scrolled cards migrate cleanly.
  - `SceneCol.icon: bool` — renders a cell in the icon font (glyph columns).
- **First fully-declarative cards** (`ui/cards/*.ron` replace the `Ink`
  placeholders; no Rust drawer runs any more):
  - `bluetooth.ron` — Header + adapter chip + `bt_rows` (icon·name·connected
    ✓/unpaired) with connected-row accent tint + `start: bt_scroll`.
  - `ticker.ron` — Header (meta "crypto") + `ticker_rows` (sym·price·chg%
    colored by sign) + `start: ticker_scroll`.
  - `currency.ron` — Header + re-base chip + `currency_rows` (active row
    accent-tinted) + `start: currency_scroll`.
  - `sshvpn.ron` — Header + `sshvpn_rows` (globe/shield glyph per line).
- **Scene-scroll plumbing**: `draw_card_scene` (`src/shell/panels/mod.rs`)
  sets the `bt_rect`/`ticker_rect`/`currency_rect` wheel-scroll targets for
  scenes without an `Ink` body (the skipped Rust drawer used to set them).
- **`scene_values()`** gains `bt_rows`, `bt_scroll`, `bt_status`,
  `bt_chip_on`/`bt_chip_off`, `ticker_rows`, `ticker_scroll`, `ticker_status`,
  `currency_rows`, `currency_scroll`, `currency_chip`, `sshvpn_rows`,
  `sshvpn_status`.
- **202/202 tests pass** (new: resting-surface-over-hover + icon cols, scroll
  window + key rebase, per-cell `col_colors`, committed batch-1 scenes parse);
  bin build clean (only pre-existing input.rs:512 warning).

## 2026-09-15 (scene Phase B: layout containers)

- **Layout containers** (`SceneItem::Row`, `Column`, `Stack`) bring
  Quickshell-style layout to the RON scene engine (`src/scene.rs`):
  - `Row(x, y, w, h, pad, spacing, valign, items)` — children laid out
    left-to-right; `want_w() == 0` items **fill** the remaining width
    equally; `valign: middle` centres short children vertically.
  - `Column(x, y, w, h, pad, spacing, halign, items)` — top-to-bottom
    layout; `want_h() == 0` children fill the remaining height; `halign:
    center` centres narrow children horizontally.
  - `Stack(x, y, w, h, pad, items)` — overlay: every child is drawn in
    the full inner box in declaration order; zero-dimension children
    (via `set_dims`) span the box for full-bleed surfaces/Inks.
  - All three treat `w` or `h` of **0** as "fill parent box" — at the
    top level this is the card; inside a container it's the slot.
- **Zero-dimension fill rule** applied uniformly to all non-`Text` leaf
  arms (`Hit`, `Surface`, `Divider`, `Bar`, `Spark`, `Fader`, `Toggle`,
  `Ink`, `Row`, `Column`, `Stack`): when declared `w` or `h` is 0 the
  item draws at the parent box's size rather than zero. This keeps
  committed top-level scenes untouched (no committed item uses `w: 0`
  except `Text`, whose `w: 0` is used only for alignment and is
  excluded) while letting containers resolve slots cleanly. `Stack`
  additionally `set_dims()`-clones children whose axes are zero so
  `Surface(x:0,y:0,w:0,h:0)` reliably spans the box.
- **`SceneItem::set_dims()`** method fills the zero axes on any
  variantable that carries `w`/`h`, enabling `Stack`'s clone-and-fill
  without modifying the enum's serde shape.
- **draw refactor** — the top-level `CardScene::draw` iterates items
  calling the new `fn draw_piece(...)` which resolves one item (or a
  whole nested box) inside a given parent `(x, y, card_w, card_h)`.
  Leaf arms were tightened so their hit regions use the parent box when
  the declared dimension is 0; inner-loop `continue`s in arms were
  converted to `return` (they no longer target the outer for-loop).
- **Tests 190 → 198**: row horizontal layout + spacing, row fill slot
  sharing, row vertical centering, column vertical stacking + halign,
  column fill slot sharing, stack full-bleed surface + ring overlay,
  nested container child scaling and relative child offsets, and
  container RON roundtrip. **198/198 pass**, bin build clean.

## 2026-09-15 (scene Phase 2a: data-driven primitives)

- **Typed scene values** — `scene_values()` now returns `SceneValue`/
  `SceneValues` (`src/scene.rs`) instead of a flat string map; all 22
  header/meta keys became `SceneValue::Text` (interpolation unchanged), and
  typed values were added for body primitives:
  - **Ring**: `battery_ring`, `mem_pct_f` · **Fader**: `volume_fader`,
    `brightness_fader`, `mic_fader` · **Toggle**: `wifi_on`, `bt_on`,
    `power_save_on`, `fx_blur_on`, `fx_shadow_on`, `fx_opacity_on` ·
    **Spark**: `cpu_spark`, `gpu_spark`, `lat_spark` (raw), `net_up_spark`,
    `net_down_spark`, `disk_r_spark`, `disk_w_spark` (peak-normalized via
    new `peak_of_pairs`) · **Rows**: `jr_rows`, `conn_rows`,
    `top_procs_rows`, `fans_rows`.
- **Six data-driven scene primitives** (`SceneItem::Rows / Spark / Ring /
  Fader / Toggle / TabRow`):
  - `Rows(name, y, row_h, cols[], pad, key_base, hover_surface, max_rows)`
    — repeats a row per `SceneRow { cols, key, action, color }`; rows past
    the card edge are clipped, per-row `key` wins over `key_base + i`, and
    `hover_surface` lifts the hovered row.
  - `Spark(name, x, y, w, h, color, max, kind: line|column, thickness)` —
    self-normalizes against the series peak when `max` is 0.
  - `Ring(name, cx, cy, center_x, center_y, radius, max=100, beads=36,
    dot=3, fill, track)` — battery-style bead ring (local `bead_ring`
    helper, since `cards/shared.rs::ring_beads` is unreachable through the
    private module chain).
  - `Fader(name, x, y, w, h, fill, track, key, action)` — slim slider with
    round handle that registers a hit region when `key != 0`.
  - `Toggle(name, x, y, w, h, key, action)` — accent-when-on switch.
  - `TabRow(name, sel, y, pad, h, key_base, active, idle)` — tab labels
    batched per-data-row, `sel` bound via `SceneValues::scalar()`, keys
    `key_base + i`, AccTint surface on the selected tab.
- **Accessors are type-safe**: `vals.rows/spark/scalar/toggle/fader` return
  `None` on a missing name **or** a type mismatch, so a misbound RON item
  draws nothing instead of panicking. `substitute()` only interpolates
  `SceneValue::Text`.
- **Col formats on Rows**: `SceneCol { x (req), w, right, center, font,
  size (≤0 → 9.5), weight (default Regular), color }` — cell colors fall
  back row → column → Fg.
- **Tests 179 → 190** — new coverage: RON parse of the new variants,
  Rows cell draw/edit clip + explicit keys + hover surface, Spark line &
  column shapes + auto-normalize, Ring bead count at 50%/overshoot, Fader
  fill/head/hit + no-key no-hit, Toggle off/on colors + key, TabRow tabs +
  selected tint, and type-mismatched names draw nothing. **190/190 pass**,
  bin build clean.

## 2026-09-14 (feature: Compositor, Countdown, Alarm, Snippets, Expenses cards)

- **Compositor / Effects card** (`DashCard::Compositor`, id `compositor`,
  tray "Effects") — live toggle tiles for **blur / shadow / opacity**
  wired to the existing `blur_main toggle` / `shadow_main toggle` /
  `opacity_main toggle` scripts via the `DashCmd` pipeline, plus a
  **screenshot row** (`shot_main` full screen / area select).
  - **Live state**: `dash_status` now also prints `blur 0|1`, `shadow
    0|1`, `opacity 0|1` (read from the scripts' state files
    `states/hypr_blur_$s`, `states/shadow_enabled_$s`,
    `states/opacity_active_$s`), so the tiles show real on/off tints and
    refresh on the existing status poll.
- **Countdown card** (`DashCard::Countdown`, id `countdown`, tray
  "Countdown") — days-until events: `label · date · Nd` rows sorted
  soonest-first, a `today!` accent chip on the day itself, past events
  **kept but grayed**. Add via the To-Do-style **inline composer**
  (`name YYYY-MM-DD`, libc-only date math — no chrono). Persisted to
  `$XDG_DATA_HOME/zen-shell/countdown.txt` (`label\tdate`).
- **Alarm / reminder card** (`DashCard::Alarm`, id `alarm`, tray
  "Alarm") — `HH:MM` + label rows with an inline `HH:MM label`
  composer and a repeat-arms row (`daily` / `weekdays` / `once`).
  **Fires the real notification popup** at zero seconds past the
  target: the always-running 60 s clock tick checks due alarms and
  pushes them through the notification channel; `once` arms
  self-remove after firing, repeating arms re-arm tomorrow. Persisted
  to `$XDG_DATA_HOME/zen-shell/alarms.txt` (`HH:MM\tlabel\tarms`).
- **Snippets card** (`DashCard::Snippets`, id `snippets`, tray
  "Snippets") — saved text clips; clicking a row **copies the body to
  the clipboard** (new `Clipboard::copy_text` alongside `set_entry`)
  and flashes a `copied` confirmation on the row. Add/remove via the
  same inline composer pattern. Persisted to
  `$XDG_DATA_HOME/zen-shell/snippets.txt` (`name\tbody`).
- **Expense tracker card** (`DashCard::Expenses`, id `expenses`, tray
  "Expenses") — month-to-date spend: quick-add chips (`1 5 10 20`) plus
  a `amount [label]` composer (comma decimals accepted), MTD total in
  the header, and per-label **ranked bars** for the current month (old
  months excluded from the ranking). Persisted to
  `$XDG_DATA_HOME/zen-shell/expenses.txt` (`cents\tlabel\tdate`).
- **Day-math fix**: `countdown_days` mixed a 1-based day-of-year
  against the 0-based `tm_yday`, making same-year diffs come out +1;
  today's day-number is now 1-based to match. `tests 128 → 139`
  (countdown day-math/sort/draw, alarm arm-expansion + due-fire + card
  draw, snippet copy/register, expense parse/summary/draw, compositor
  status parsing + chip registration).

## 2026-09-14 (feature: Water tracker + Moon phase cards)

- **Water tracker card** (`DashCard::Water`, id `water`, tray "Water") —
  daily hydration log: a bead-ring gauge (the CPU card's `ring_beads`
  style; INFO-blue, OK-green with a ✓ when the goal is met) around a
  glasses + "of N glasses" center readout, `−` undo and `+ glass` buttons
  below. One glass = 250 ml; the header meta shows `X.X / Y.Y L`. The
  count **auto-resets at midnight** (checked on load and on every
  interaction); the **daily goal adjusts by horizontal swipe** over the
  card (1–30 glasses, same swipe chain as the battery pane flips).
  Persisted to `$XDG_DATA_HOME/zen-shell/water.txt` (`goal` / `day` /
  `count` lines). Keys `WATER_INC_KEY = 31_300`, `WATER_DEC_KEY = 31_301`.
  `tests 123 → 128`.
- **Moon phase card** (`DashCard::Moon`, id `moon`, tray "Moon") — the
  current phase as a **vector shaded disc** (no bitmaps: the lit portion
  is painted as scanline slivers whose terminator ellipse collapses at the
  quarters — waxing lights right, waning lights left), next to the phase
  name, illumination % and days-until-full ("full tonight" within a day).
  Pure date math from a known new-moon epoch (synodic 29.53059 d) — no
  sources, no I/O. Phase math is tested against real 2024 new/full-moon
  epochs; the disc is exercised across the whole cycle (no panics,
  slivers drawn).

## 2026-09-14 (feature: World clock card)

- **World clock card** (`DashCard::WorldClock`, id `worldclock`, tray name
  "World clock") — pinned cities with their local time. One row per zone:
  sun/moon glyph (local hour 6–17 = day), city, right-aligned mono HH:MM
  and a `+H:MM` offset-vs-local chip (hidden for the local zone). Hover ✕
  removes a row; a "+ add city" row opens an **inline zone.tab search**
  (type ≥ 2 chars → up to 6 city/tz matches, click to pin; Esc closes;
  dedup by tz, max 16 rows). `tests 120 → 123` (offset/hour math, pin
  dedup + persistence, row + search-overlay draw).
  - **Data flow**: `HH:MM`/dayline/night are computed at draw time from a
    cached UTC-offset (pure libc, no per-frame subprocesses). Offsets +
    abbreviations refresh via ONE worker thread probing every pinned zone
    with `TZ=<tz> date +%z %Z` (results through a calloop channel), armed
    from the 3 s services timer only while the dashboard is open, at most
    every 5 min (DST).
  - **Persistence**: `$XDG_DATA_HOME/zen-shell/zones.txt` (`city\ttz`
    lines); first launch seeds the classic four — UTC · New York · London ·
    Tokyo. Keys: `WORLDCLOCK_DEL_BASE = 31_000`, `WORLDCLOCK_ADD_KEY =
    31_100`, `WORLDCLOCK_PICK_BASE = 31_200`.

## 2026-09-14 (feature: Focus / Fans / Workspaces cards + water-fill batteries)

- **Three new dashboard cards** (`tests 114 → 120`):
  - **Focus** (`DashCard::Pomodoro`, id `pomodoro`) — pomodoro timer: big
    mm:ss hero, start/pause/resume + reset, focus-length steppers (±5 min),
    phase progress bar. Focus ↔ break auto-flip (break tinted OK-green);
    each finished focus session bumps a "N done" counter. Durations +
    sessions persist to `$XDG_DATA_HOME/zen-shell/pomo.txt`; the 1 s
    countdown timer self-tears down when paused (0 CPU at rest, mirrors
    `sync_viz_timer`).
  - **Fans** (`DashCard::Fans`, id `fans`) — live fan RPMs from every hwmon
    `fanN_input`, labeled `fanN (chip)`, per-fan bar normalized against a
    slow-decaying peak (spin-ups visible, no hardcoded maxima); stopped
    fans dim; empty state mirrors the thermal card.
  - **Workspaces** (`DashCard::Workspaces`, id `workspaces`) — Hyprland
    workspace tiles (≤10/row, grid adapts); click switches via
    `pending_ws_dispatch` (same path as the pill's long-form chips); the
    active tile is accent-tinted with the dot. Keys `WORKSPACE_KEY_BASE
    = 30_800`.
  All three appear in the parked-cards tray and append to the default
  lattice (row 18 / below WeatherV2). New shell fields: `pomo_*`,
  `fans_rpm`, `fan_peaks`; stats poll samples fans with the 3 s services
  tick; `draw_edit_btn` / `draw_edit_lbl_btn` are now `pub(crate)` (shared
  with card modules).
- **Water-fill battery icons** (both battery cards). `push_battery_icon`'s
  interior reworked into `draw_water`: the liquid now spans the FULL inner
  cross-section so its edges sit against the case stroke on both sides (was
  an inset floating bar), with a vessel tint behind it, a lighter wave-crest
  line along the surface (8-segment sine) and a soft highlight band under
  the crest. Vertical battery: water climbs, horizontal crest; horizontal
  battery: water grows left→right, vertical crest. Charge colors unchanged
  (OK/acc/DANGER ladder).
- **Dual-glyph stack toggle** — the edit-mode scroll-direction button now
  renders BOTH direction glyphs side by side (↕ accent when rows, ↔ when
  columns) instead of swapping one glyph; button doubled in width and the
  right-anchored toolbar cluster math updated to match.
- **Build fix** — `notes.rs` composer held `&self.scale` across a
  `self.region()` call (E0502, blocked every build); `GridScale` is `Copy`,
  so the composer now copies it by value.

## 2026-09-14 (feature: card-header visibility toggles in edit mode)

- **"Card titles" / "Card glyphs" toggles in the edit-mode strip-editor row**
  (right of "Enable banner chips", dashboard ON only). Both default ON and
  persist per channel (`card_show_title` / `card_show_glyph` in the master
  config, honored on boot). Titles OFF drops every card's header TITLE text;
  glyphs OFF drops every card's header ICON and slides titles left into the
  freed slot — content, right-aligned meta and pagination dots are untouched.
  Gated in `card_header` (shared.rs) and at every hand-rolled header site
  (apps, bluetooth, clipboard, clipimg, currency, diskio, docker, eq, lyrics,
  mirror, news, notes, power v/h, procmon, quote, recent, sensors, sshvpn,
  ticker, todo, topproc, viz, wifi, worldmap). `banner_ctrl_min_w` grew to
  fit the two new chips. Tests 110 → 114 (draw+register, hidden when the
  dashboard is off, click→persist→redraw round-trips for both toggles).

## 2026-09-14 (fix: hover expanded to an empty dashboard)

- **Hover-expand deadlock in the size handshake.** The final tween commit
  (1009.79×494.79) was classified against the acked configure (1009×494) with
  a 0.5 px epsilon — but layer-shell sizes are truncated u32, so that IS the
  same surface size. The guard missed it, `set_size(1009,494)` went out as a
  no-op commit Hyprland never acks, and `size_pending` wedged: `tick()` holds
  the anim while pending, `maybe_render` drops frames while the anim is up.
  The 800 ms timeout then un-stuck pending just long enough for the next tick
  to clear the anim and re-commit the SAME never-acked size, and the morph
  timer's stop condition removed the timer while pending was still set —
  nothing left could redraw, so the dashboard expanded to full size and
  painted only its background ("hover does not show dashboard/cards").
  Fraction-dependent, hence every hover on this setup. Fix: `commit_size`
  compares truncated sizes (`App::size_is_configured`), and `morph_tick`
  keeps the timer alive while `size_pending`. Tests 107 → 110; verified live
  (expand renders the full card grid, 133 frames at 1009×494).

## 2026-09-14 (fix: edit-mode ✕ unclickable on a scrolled board)

- **Edit-mode card ✕ (park-to-tray) now clicks where it's drawn.** The three
  edit-mode hit rects were computed in three different coordinate spaces:
  the grip was scroll-corrected (`scrolled()`), the card body subtracted
  `dash_scroll_y` only, and the ✕ rect (`card_close_px`) not at all — while
  the draw loop scrolls ALL content by both `dash_scroll_x` and
  `dash_scroll_y`. With the board panned (trays stack above the grid in edit
  mode, so big boards scroll), the drawn ✕ sat at a different spot than its
  hit rect: the click fell through to the body check and started a DRAG
  instead of parking the card. All three rects now resolve in surface space
  via `scrolled()`; `card_close_px` also takes the scale (offsets/size now
  match the drawn `scale.s(27/7/20)` exactly — grid_scale ≠ 1 was silently
  off before). `edit_motion`'s resize branch gets the same scroll
  compensation the drag branch already had. Regression test:
  `close_button_hit_rect_follows_board_scroll` (tests 106 → 107).

## 2026-09-14 (minimal UI polish — batch 1+2: cards)

Design direction: **minimal / flat / professional** (Linear · VS Code ·
Grafana feel, NOT Material). Settings panel got this treatment 09-13/14;
this pass brings it to the dashboard cards.

- **Type ramp enforced via helpers** (`ui.rs`): small text now renders at
  REGULAR weight (`caption` / `caption_r` / `caption_c` — Light dissolves at
  ≤10px); numerals routed to their proper voices — Display Medium hero
  (`text_c_hero`/`text_hero`) for gauge %s, pkg-update count, GPU util,
  kbd layout, weather temp; Geist Mono for every tabular figure
  (`text_c_mono` added). The configured Inter / Inter Display / Geist Mono
  stack is now actually used everywhere instead of Light Inter by default.
- **Status color tokens** (`ui.rs`): `OK` / `WARN` / `DANGER` / `INFO`
  (muted Tokyo-Night tones) + `pct_color(pct, warn_at, crit_at, normal)` —
  the single threshold ladder. Replaced the two competing RED/GREEN pairs
  (`0xff6b6b`/`0xf38ba8`, `0x2ee6a8`/`0xa6e3a1`), the legacy Catppuccin
  blues `0x44aaff` (network dl, diskio read, speedtest, GPU VRAM bar →
  `INFO`) and oranges `0xffb86c` (cpugpu GPU line, GPU temp → quiet `fg`
  meta / `INFO` line; orange only ever marks a threshold now).
- **Shared card chrome** (`cards/shared.rs`): `card_header` (semibold 11.5
  title, optional quiet fg2 icon, optional right-aligned mono meta) is the
  ONLY header style; `metric_row` (fg2 label + right-aligned mono value);
  `card_empty` (one centered empty state). Migrated: cpu, mem, disk,
  thermal, network, gpu, battery_frame (both battery cards), ticker,
  speedtest. Acc-tinted header icons (thermal, gpu, battery, ticker,
  speedtest) dropped to fg2 — accent is for live data only.
- Per-card fixes: network dl line muted (`INFO`), disk mount labels fg2,
  gauges disk ring `ui::OK`, gauge % in Display, weather forecast temps in
  mono, kblayout icon fg2 + label Display, pkgupdates count Display hero.
- **Weather digit bug**: the big temp was one string `{glyph} 22°C` — the
  digits rasterized in the ICON font. Split into icon + `text_hero` temp.
- `shell::RED`/`GREEN` consts now alias the ui tokens (one source).
- Tests: 106 passed (weather card tests still green after the pane
  rework). `cargo check` clean.

## 2026-09-13 (world-map → always-on + pinch)

- **Embedded fallback back in the binary**. `src/shell/world_map.bin` is
  re-embedded (`include_bytes!`, byte-identical to the GitHub `world.bin`) and
  installed the moment the card draws, so the original world map ALWAYS
  renders — no placeholder, no waiting for downloads. `worldmap::ensure_fallback`
  / `using_fallback()` track the state; the first downloaded world chunk
  replaces it in place (flag clears). A compact "Download full map" pill now
  overlays the map body only while the fallback is active.
- **Touchpad pinch-zoom** (`zwp_pointer_gestures_v1`): bound in
  `ensure_pinch` (lazy, degrades silently when the compositor lacks the
  global), pinch events dispatched into the app — over the map body
  (`Shell::over_worldmap`) a pinch scales `world_zoom` continuously in the
  same 1×..32× band, sitting beside the +/− buttons.
- **Scissor containment**: the whole map body (fill, graticule, dots,
  borders, region rings, city labels, marker) is wrapped in
  `Cmd::Scissor`/`ScissorEnd`, so zoomed/overflowing primitives never bleed
  past the card bounds. Hover crosshair + outline stay outside the clip.
- **Save-downloads defaults OFF**: `[world] save_data = false` in the shipped
  TOML and the `World` default; the toggle still flips/clears cache dirs.
- Tests 93 → 95 (embedded parse/land + fallback flag round-trip; card test
  now asserts the fallback map + download pill draw together).

## 2026-09-13 (world-map → app)

- **Map data leaves the binary**. The world chunk (17 KB), city labels and
  per-country 50m chunks now ship from `ezone134/zen-shell-map` on GitHub
  (`base_url`, overridable via `[world] data_url`), fetched on demand by curl
  workers and verified by sha256 + size. New `pub mod mapdata` (manifest
  cache, `region_at`, atomic chunk downloads) + runtime store in `worldmap`
  (`set_world`/`ready`/`clear`). the binary then stopped embedding `world_map.bin` (it ships again since the "always-on + pinch" amendment above).
- **Placeholder + download pill**: with no cached data the card shows a
  placeholder and a "Download world map" button (`WORLD_DL_KEY`); accepting it
  spawns a background worker, and results are applied on the services tick.
  Cached chunks render instantly at boot.
- **Swipe-right menu** (`WORLD_MENU_KEY` backdrop + rows): 4 style panes
  (`WORLD_STYLE_KEY_BASE`..+4) and a **Save downloads** switch
  (`WORLD_MENU_TOGGLE_KEY`) that flips the cache dir
  `~/.local/share/zen-shell/maps` ⇄ `/tmp/zen-shell/maps` and persists to
  config (`[world] save_data`, default ON). Swipe-cycling + pagination dots
  are gone on this card (other cards unchanged via `card_pane_flip_n`).
- **Zoom +/−** (`WORLD_ZOOM_IN/OUT_KEY`, 1×..32× around the pinned marker),
  windowed projection (`win_for`/`fit`/antimeridian wrap, `Win::contains`),
  city labels from 2×.
- **Region detail**: cross 4× zoom inside a country → "Download <Country>"
  pill (`WORLD_REGION_DL_KEY`, region picked by smallest containing bbox via
  `region_at`); its rings draw accent-stroked over the base map; cached
  regions auto-load while zoomed.
- Map back-end reworked: `src/shell/worldmap.rs` (chunk parsers round-trip)
  + drawer `src/shell/panels/cards/worldmap.rs`; new shell fields for
  center/zoom/menu/dl-state/region/manifest/marker. Tests 87 → 93 (chunk
  roundtrip, manifest `region_at`, all pane drawings, placeholder, menu,
  region pill, marker band); world-store tests serialize on `TEST_LOCK`.

## 2026-09-13

### World map card (vector, clickable world clock)

- **New dashboard card `DashCard::WorldMap`** (id `worldmap`, region key
  `WORLD_MAP_KEY` = 14_900): a clickable world map drawn ENTIRELY from vector
  primitives, no bitmap. Earth 110m country polygons are pre-compiled into the
  embedded `src/shell/world_map.bin` (17 KB, 162 rings / 4096 points, i16
  lon/lat in 1/100°) and rasterized once into a 1440×720 scanline landmask;
  everything on screen derives from it.
- **4 style panes**, flipped by horizontal swipe (no wrap-around) with the usual
  pagination dots: **Real** (accent-tinted land + country borders + 30°
  graticule), **Filled** (clean silhouettes — pixel-exact at the map's own
  resolution and vertical-run merged), **Dots** (dot-matrix land), **Lines**
  (borders + graticule only). The card defaults to span (10, 9) — the 2.1:1
  world needs height on the ~6:1 grid cells.
- **Click → local time**: clicking the map un-projects the pointer to (lon, lat),
  pins an accent marker, finds the nearest timezone in `/usr/share/zoneinfo/
  zone.tab`, and shows that place's **HH:MM + timezone abbrev + weekday** in the
  card's bottom band. The UTC offset is probed once with a single
  `TZ=<zone> date +%z %Z` subprocess (`world_shell_probe` in `src/app/mod.rs`,
  3 s services timer) and re-probed ~every 5 min for DST; the clock itself is
  formatted at draw time by pure libc shifts (`world_hhmm` / `world_dayline`) —
  no per-frame processes.
- **Multi-pane machinery generalized**: `pane_dots` (any pane count —
  `battery_dots` is now a 2-pane alias) and `card_pane_flip_n`; weather's and
  the battery cards' behavior is unchanged.
- New module `src/shell/worldmap.rs` (parsing, landmask, equirectangular
  projection + fit, zone.tab nearest-zone lookup, tz-offset parse, time
  helpers) + drawer `src/shell/panels/cards/worldmap.rs`. Tests 84 → 87
  (worldmap data/mask/projection/zone/time + the 4-pane card drawer).

## 2026-09-11

### Edit-mode banner & parked-tray polish

- **Parked chips/cards columns stretch to the panel width**: the banner-chips
  tray and the dashboard-cards tray no longer leave a dead strip hugging the
  right edge on wide windows — each column STRETCHES so every row fills the
  tray's full width (`banner_tray_chip_px` / `tray_chip_px`). Wrap count stays
  the same, so tray heights, pagers, scrollbars and hit-testing are all
  unchanged (drawer and input keep reading the same shared rect).
- **Edit-mode banner chips render as ICONS, not word labels**: strip cells and
  the parked banner-chips tray now draw each chip's Nerd Font glyph
  (`BannerItem::glyph()` — clock → clock, wifi → wifi, settings → gear, …)
  instead of its descriptive word, so the strip reads at a glance while staying
  draggable. `Dash: ON/OFF` still keeps its words; separator glyphs and the
  spacer label are untouched.
- **Drag handlebar raised + easier to grab**: the edit-mode grab ledge on strip
  chips moved up (led 9 → 12 px) and the bar is slightly taller + brighter on
  hover, so it reads as a clear grab target instead of hugging the band's
  bottom edge. Chips show icons, fillers keep their centered handle.

### Dead-code sweep (≈96 audit warnings → 0 warnings, 0 errors)

`cargo build` is now clean. Deleted unused fns/fields/consts, incl. hypr
`dispatch_workspace_prev/next/previous`, `LogindBrightness::raw`, config
`weather_enabled` + `notif_timeout_for`, `Rate::Live`/`Rate::Fast` (+ their ttl
arms), `BANNER_W_MINUS_KEY`/`BANNER_W_PLUS_KEY`, `BannerToken::chip()`,
`BannerDrag.sx/sy`, `BASE_TRAY_STRIP_H`, the old `pack_cards()` (its 4 tests
rewritten against `pack_cards_max`), the drop-zone helpers
(`banner_drop_band`/`banner_drop_slot`/`banner_zone_at`/`default_banner_zone`/
`banner_home_zone` — superseded by the column-based drop grid), `dash_tray_rows`,
`tray_strip_h`, `card_at`, `tray_chip_at`, `launcher_scroll_by`, `gauge_gb`, and
ui.rs `warn`/`ok`/`frame`/`chip`/`vslider`/`shadow`. Kept as intentionally-unused:
`DEFAULT_SHELL_TOML` and `Layout.w/h` (both exercised by tests). `banner_drop_target`
is kept as `#[cfg(test)] pub(crate)` (used by 13 pack-test call-sites).

**Two strip-editor controls that were drawn but click-dead for a while were
re-wired** in `edit_press`: the dashboard ON/OFF row button (`BANNER_CTRL_KEY_BASE`)
and the banner-width AUTO/MANUAL toggle (`BANNER_W_AUTO_KEY`, restores the last
manual value via `banner_w_last`). (The `−/+` width buttons from the 09-10 batch
were removed; AUTO/MANUAL + the hidden live-width slider took over, and the sweep
deleted the orphaned `BANNER_W_MINUS_KEY`/`PLUS_KEY` bands.)

### Polkit agent registration fixed

The agent found a real bug on live startup: it registered with its **session-bus
unique name** as the subject, which polkitd (system bus) rejected with
`Unknown subject of kind ':1.1910'` — the agent thread then died with no retry, so
password auth could never work. `register_with_authority` now uses a
`unix-session` subject keyed by `XDG_SESSION_ID`; startup logs
`zen: polkit agent registered`. Verified across 4 live restarts on Hyprland (W1).

### Tests (58 pass) + live validation

- New `Layout` geometry tests (`shell/panels/layout.rs`): surface size, geometry
  matches shell metrics, grid starts below the banner strip, edit chrome stacks
  above the grid, tray-gap split between chrome rows.
- New `edit_drag_tests` (`shell/mod.rs`) drive the reconstructed
  `edit_press`/`edit_motion`/`edit_release`: move-and-commit, same-slot no-op,
  close-to-tray, grip-resize clamping, noop release.
- Live run of the new binary on the real Hyprland session: broker warm-up fires on
  dashboard open (cards flip to "fetching…"), the weather card populates
  (48°/43°/44°/59°) — request→render works end-to-end. Quote/news/ticker degrade
  gracefully here because their hosts are unreachable from this box
  (`api.quotable.io` → HTTP 000) and every fetch has an 8 s `curl --max-time` cap,
  so nothing hangs. Collapsed mode = zero fetches, as designed.
- `TODO-tomorrow.md` (sweep/layout-tests/edit-input-runtime/broker-warm-up)
  completed and deleted.

## 2026-09-10

### Banner strip editor + edit chrome (parts 2 & 3, merged from NEXT_STEPS)

- **Strip editor shipped**: text-only chips (no pill bg), separators + smart
  fillers, free drag (a chip slips *through* a filler), an always-visible control
  row below the banner tray (dashboard ON/OFF state-aware toggle, separator
  presets, spacer; width AUTO/MANUAL toggle + live-width slider hidden in auto
  mode), `DashToggle` removed from `DEFAULT_BANNER_ORDER` + filtered on load,
  empty-strip band collapse, ink-centered glyphs + one shared midline,
  drop-anywhere column grid, L/C/R zone sections (`Zone(u8)`) with click-to-home
  zones, 3 s liquid-fill "clear all", empty-state hints, opaque full-width chrome
  slabs.
- **Edit chrome (part 2)**: chip-based `banner_collapsed()`, click-clear for both
  clear-alls, `dash_area_collapsed()` folding, word labels for edit-mode chips,
  `Wifi`/`Bluetooth`/`Connectivity` + 9 basic status chips (battery, weather, cpu,
  ram, net_speed, dnd, volume, brightness, vpn) with live glyphs + accent states +
  click targets, stack-mode 16-col clamp + directional scrollers + hidden rail,
  hover-only pagination dots.
- **Tray scrollers + UI style sliders**: scrollable parked trays
  (`BANNER_TRAY_VROWS=2`/`DASH_TRAY_VROWS=3`, row-index pager, thin scrollbar);
  config `ui_pad`/`win_radius`/`card_radius`/`card_radius_sync` + edit-mode
  sliders (surface corners follow `panel_r()`); `DASH_CARD_KEY` moved 34_400 →
  34_480 to clear the app-shortcut band; Settings "Expand on hover" info-tip band.
  Compiled + tested (48 tests); three latent build bugs fixed (undeclared slider
  rects, `\u{00b7}` escape, connectivity-popover borrow).
- **BUGFIX**: dashboard no longer collapsed to a ~350 px ribbon when ON —
  `vert_col_cap()` is now viewport-based (up to the 16-col vertical-stack ceiling)
  when the dashboard is on; the strip clamp only applies when the dashboard is OFF.
- **Operational notes**: the bar configs live in `$states/shell_{ch}` (channel from
  `$states2/s`, live = `n`); card/grid edits need `zen-shell reload`; pin moves
  (grid placement) need a restart; power/cpu/clipimg cards are pinned on the `n`
  grid with copies in the `d`/`l` trays.

### Power cards, system-card settings chevron, wallpaper-card grid (v1–v6, merged)

- `DashCard::PowerV`/`PowerH` hold-to-confirm cards (keys 12_000 band — moved down
  from 35_000, which the banner arm swallowed); icon-only tiles.
- System card bottom-right `>` (⇢ Settings). Wallpaper card: adaptive cells (≥5 cols
  × 2 rows), live count, right-edge scrollbar (`WP_CARD_SB_UP/DOWN` 10_000/10_001),
  clip-safe rows.
- Memory + CPU cards reworked (mem used/avail/cached/free rows; cpu load/freq/temp),
  circular gauges (ring beads, pct + used/total, `disk_used_gb`/`disk_total_gb`).
- Clipboard text card click-to-copy (CLIP 80) + Clipboard-images card
  (`clipimg`, CLIPIMG 140); clipboard freeze fixed (skip own echo offer), panel
  scroll, `{texts:[...]}` persistence.
- Weather thread always runs; auto-geolocates via ip-api.com when lat/lon are 0
  (`WeatherMsg.city`, `Clone` not `Copy`).
- Media card glyph/transport fixes + Space play/pause; Theme & Accent moved off the
  system card with a Scheme chip → Themes; list cards (clipboard/clipimg/todo/notes)
  scroll via shared `list_scroll_by`.

## 2026-09-09

### Dashboard UI batch (Workstream B, 47/47 tests)

Scroll-direction −/+ toggle in the edit toolbar (`dash_scroll_dir`), edit-mode
opaque backdrop, banner separator + spacer below the banner, parked-chip
click-to-toggle placement, new `AppShortcut` card (esp. `app_shortcut_pick`
launcher flow), clock/date/app-search banner chips.

> Zeneq EQ crate: **HALTED** (user) — this machine's PipeWire cannot load LADSPA
> (`libspa-filter-graph-plugin-ladspa.so` needs `spa_log_topic_enum`, not in the
> installed spa). Do not reopen until PipeWire/spa are updated.

## 2026-08-29

### Ultra-fast startup: all file I/O deferred past the first frame

- `Shell::new()` no longer reads any files — hostname, uptime, todo list,
  dashboard layout, notification history, states colors, and accent flags are
  all loaded by a new `seed_data()` method that runs **right after the first
  frame is committed** (alongside the existing `seed_boot_data()`).
- **App-manager background scan**: the `.desktop` file scan (hundreds of
  files across 3 directories) now runs on a **background thread** via
  `AppManager::rescan_async()` and results arrive through a `std::sync::mpsc`
  channel polled every 3 s in the services tick.  The launcher starts with an
  empty list and populates within a second or two — the bar + launcher are
  interactive **immediately**.
- The pill, launcher, and any boot-mode surface now appear on screen before
  *any* disk I/O, sysfs read, or subprocess spawn — true sub-frame startup.

### Color-temperature slider range widened to 1200 K–6500 K

- Dashboard color-temp slider now spans **1200 K** (ultra-warm candlelight)
  to **6500 K** (daylight D65), up from the previous 2500 K–6500 K range.
- `Shell::seed_data()` handles the post-frame initialization including
  `hostname()`, `uptime_str()`, `load_todos()`, `load_notifs()`,
  `load_dash_layout()`, `apply_persisted_dashboard()`, `refresh_pack()`,
  `reload_states_colors()`, `reload_accent_flags()`, and `sync_dark_state()`.



### Dashboard layout persisted as an array in `$states/shell_{n,d,l}`

- Every **enabled** dashboard card is now remembered with its **order,
  size AND exact position** and written to the per-state channel config as a
  `[dash_cards]` array of `{ card, x, y, w, h }` (cells). Cards parked in
  the edit-mode tray are saved as id strings in `[dash_tray]`.
- Saved on every pill-settings change (`save_config()`) and on every
  edit-mode commit (move / resize / ✕-park / tray add) — so the dashboard
  survives restarts exactly as the user arranged it, per channel
  (dark/light/night each keep their own).
- On load (`Shell::new` + channel switch in `sync_per_state_config`), the
  saved cards are **pinned** (`layout_pinned`) to their exact positions so
  the flow packer does not re-derive/drift them; the pin clears on the first
  edit and the live packer takes over again.
- The legacy order+spans `~/.local/share/zen-shell/dash_layout.txt` is still
  written as a fallback / first-boot source.

### Flow-packer rules (documented)

- Cards are a vertical **stack**: first card = top of stack (placed first),
  last card = last of stack (placed last).
- **Left-side fill:** within its own row a card may slide left at most ONE
  column to fill an adjacent gap (never a far-left teleport).
- **Jump-up compaction:** when its own row is full, a card jumps up one row
  into the **rightmost** empty slot of an already-populated row — so the
  **2nd row's LEFT card pops into the 1st row's RIGHTMOST slot** — keeping
  rows left/top-packed without scrambling card order.
- Otherwise it scans down to the first row with enough room. The default
  grid is bit-for-bit preserved (regression-tested).

### Faster startup: peripheral data seeded after the first frame

- The bar + launcher now come up instantly. The blocking reads that used to
  run before the first frame — hyprctl ws/active-win IPC (`Hypr::update_shell`),
  D-Bus ssid/media (`Services::poll`), battery/AC (`power.poll`), sysfs
  brightness, and the `wpctl get-volume` subprocess — are deferred into a new
  `App::seed_boot_data()` that runs **right after the first frame is committed**
  at the initial wayland roundtrip in `App::start()`.
- `App::new` still constructs the objects (cheap sockets/structs) but no
  longer blocks on the data reads, so rendering + input are interactive
  immediately. `seed_boot_data()` only redraws if a seed actually changed
  something; the existing 3s services / 60s power timers take over from there.

### Custom accent section in Settings → Appearance

- New **ACCENT SOURCE + CUSTOM ACCENTS** section in Settings → Appearance.
- **Accent source** buttons: *From wallpaper* (`theme_main accent
  acc_from_wall`) and *Scheme default* (`theme_main accent theme_default`) —
  each writes the `$states/acc_source_<ch>` flag via `theme_main_body`.
- **Custom accents**: reads `$states2/custom_acc.json` (the `[{name,d,l}]`
  list regenerated by `gen_launcher_cache`/`gen_custom_acc_files`, now also
  mirrored into `$states2/`). Rendered as a **collapsible dropdown**: a
  "CUSTOM ACCENT" row shows the current selection (name + swatch); clicking it
  opens a **scrollable popup list** (wheel scrolls its own list; a thin
  scrollbar shows position). Clicking a name selects it and runs
  `theme_main accent custom_acc <name>` → turns custom accent on +
  `custom_acc_name_<ch>`, echoes `acc_changed`, and restores so GTK (thunar)
  picks the color up (restore applies the previously-selected value state).
- A `colors reload` is scheduled after each selection so the live palette
  (`$states2/shell_vars`) updates the shell without a restart. Selection is
  highlighted against the current `custom_acc_name_<ch>` flag.

### `theme_accent_func`: single `acc_source` flag + change-only restore

- The accent setter now writes the canonical `$states/acc_source_${s}`
  flag alongside the per-source flags (`d` = theme default, `w` = acc from
  wall, `c` = custom accent, `h` = defined hex) — the same flag the Rust
  settings pipeline and the old `theme_body` read.
- **Change-only restore:** if the requested source (and payload, for
  `custom_acc`/`define_hex`) already matches the current `acc_source` flag,
  it does nothing — no `theme_main restore`, no `acc_changed` write.
- On an actual change it echoes `1 > $states2/acc_changed` AND runs
  `theme_main restore`, so GTK apps (thunar) pick up the new accent color.
- `define_hex` with no payload reads `$HOME/documents/defined_hex` via
  bash's `$(<file)` — no `cat` subprocess spawned.
- `acc_from_wall all` now iterates the d/n/l channels and only triggers a
  restore if at least one channel actually changed.

### Accent toggles + Alpha in Settings → Appearance

- New **ACCENT TOGGLES** section: flip the per-channel `custom_acc_start_icon`,
  `custom_acc_app_border`, `custom_acc_hyprland`, `custom_acc_gtk` and
  `custom_acc_normal_app` flags, plus a **Start icon tone** slider
  (`start_icon_tone_<ch>`, 0–900).
- New **ALPHA** section: sliders for `bg_alpha` / `acc_alpha` / `scrim_alpha` /
  `border_alpha` (2-digit lowercase hex) and an **Accent scrim** toggle
  (`acc_scrim_<ch>`).
- Everything routes through `theme_main` / `theme_main_body`:
  `theme_main toggle_acc <flag> <0|1>`, `theme_main start_icon_tone <n>`,
  `theme_main alpha <flag> <hex>`, `theme_main acc_scrim <0|1>` — each writes
  the flag and runs **`theme_main restore`**. **No `acc_changed` write** — that
  marker is only for accent *source* picks that need the GTK (thunar)
  hot-reload; a plain restore is enough for these flags.
- A single **Apply changes** button at the bottom runs `theme_main restore` so
  all pending toggle/slider edits hit the live palette at once (the shell
  re-reads them via the scheduled `colors reload`).
- The Appearance pane gained a vertical scroll so the new sections + the custom
  accent grid all fit; flags are re-read on boot and every `colors reload`
  (`Shell::reload_accent_flags`), so the UI tracks the live `$states` values.
