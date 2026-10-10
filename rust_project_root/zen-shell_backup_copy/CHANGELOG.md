# Changelog

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
