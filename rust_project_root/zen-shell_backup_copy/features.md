# zen-shell — feature ledger

A running inventory of the shell's user-facing features, in particular the ones
added and refined across recent sessions. The CHANGELOG records *what changed
when*; this file records *what exists and how it behaves*. Keep both in sync when
you land something new.

## Expanded dashboard panel

- **Content-fit sizing**: the expanded panel no longer insists on a fixed wide
  width. In normal (non-edit) mode the surface sizes to the dashboard's actual
  grid columns/rows (`dash_canvas_w/h`, clamped to the viewport) when the
  dashboard is enabled with cards. A banner-only strip (dashboard off) keeps its
  fit-to-content or manual width (`banner_width`).
- **Card memory**: card ids, sizes and positions are remembered per channel in
  the config (edit-mode move/resize/place persist), dashboard layout is one auth
  source — `pack_cards_max` packs cards deterministically (no overlaps, gravity
  left/top), `layout.rs` computes the render geometry once per frame.
- **Edit mode**: grid boost `rows/cols` (+/−) buttons, parked-cards tray with
  click-to-add and drag `→` affordance, card resize grip (bottom-right corner)
  and move-drag, drop-ghost slot, `Clear all` (3 s hold, liquid fill).

## Top banner strip

- **Strip layout**: ordered slots (chips, `sep:` separators, `filler`,
  `zone:0|1|2` L/C/R section markers). Persisted per channel as
  `[bar] banner_order`. Chips are draggable to replace / reorder; parked chips
  live in the banner-chips tray above the strip.
- **Edit-mode chip ink = icons**: on-strip cells and the parked banner-chips
  tray render each chip's Nerd Font glyph (`BannerItem::glyph()`) instead of a
  word label, so the strip reads at a glance while staying draggable. The only
  text kept is `Dash: ON/OFF` (the dash toggle) and separator/spacer labels.
- **Drag handlebar**: every non-filler strip cell gets a dim rounded grab-bar in
  a reserved bottom ledge (raised so it sits comfortably inside the band) —
  grab it to lift / reorder the chip; fillers show a small centered handle.
- **Whole-strip horizontal scroll**: when the strip's natural width overflows
  the panel, it becomes one left-aligned row panned with `banner_strip_scroll`
  (wheel anywhere on the strip band). L/C/R chips pan together as a unit — no
  per-zone independence in that mode. A thin scrollbar (track + thumb) along
  the strip's bottom ledge shows the pan position.
- **Per-zone scroll**: with zone markers on L/C/R, each zone clips + scrolls
  independently when its chunk overflows its third.
- **Banner width control**: `banner_width` (0 = auto-fit). Dashboard ON → width
  follows the dashboard columns; dashboard OFF → editable via the edit-mode
  AUTO/MANUAL toggle + live slider.

### Chip width (auto-only — no per-chip override)

- Chip widths are **natural/auto** (`banner_chip_w`: measured text width, or a
  fixed glyph-chip width). A per-chip width-override map (`banner_chip_widths`)
  and an edit-mode resize grip were sketched but **never landed** — there is no
  override in the config and no `banner_resize` drag state (the chip-drag
  offsets ride `off_x`/`off_y`, see the 09-11 sweep).
- Spacing between chips is the uniform `banner_gap`. If per-chip widths are
  wanted later, add the override map + grip fresh, mirroring the card
  move/resize drag pattern in `dashboard.rs`.

## Edit-mode chrome (parked trays)

- **Full-width columns**: parked banner chips and parked dashboard cards lay out
  in columns that STRETCH so every row fills the tray's full width — wide
  windows never leave dead space hugging the right edge. Wrap count, pagers,
  scrollbars and hit-testing all share the same column math.
- **Rectangle style**: parked banner chips and parked dashboard cards render as
  plain rectangles (`r: 0`) — no rounded corners.
- **Add-back affordance**: every parked chip/card shows a `+` square badge at
  its top-right corner. Clicking the chip (or the badge) adds it back to the
  strip / the grid at the next free slot.
- **Remove affordance**: on-strip chips' top-right corner shows a `−` badge in
  edit mode (removes the chip back into the parked tray). Both badges are flat
  squares.
- **Clear-all buttons**: the banner-tray `clear all` pill and the board's
  `Clear all` button are rectangle too, with a left→right red liquid fill while
  held (3 s to fire).

## Parked-cards tray

- Independent scroller: wheel over the tray scrolls it (up to `DASH_TRAY_VROWS`
  rows with a right-edge scrollbar indicator); the board itself never moves.
- Columns stretch to fill the tray's full width (see "Edit-mode chrome").
- `Clear all` on the board parks every card back into the tray via the pure
  `park_all_cards()` routine (unit-tested).

## Mirror card

- Live front-camera feed (the first `/dev/videoN` whose `ffprobe` reports MJPEG;
  `camera_dev` in the config still overrides). Captured via ffmpeg as **rawvideo
  RGBA 640×360** — no JPEG in the pipe, so each frame is a fixed-size buffer
  split by the reader thread and uploaded straight to GL (no decode in the loop).
- Rate: **60 fps preferred**, respawned once at **30 fps** if the device rejects
  the rate (dies) or stays silent past warmup (busy). Ticks ~every 16 ms, so the
  on-screen framerate equals whatever the device actually delivers.
- The card shows the **measured** rate (actual frames/sec over a 1 s window),
  not just the requested one — a 30-only camera reads "30 fps", a 60 camera
  reads "60 fps".

## World map card

- **Interactive vector world clock** (`DashCard::WorldMap`, id `worldmap`): a
  clickable world map rendered purely from vector primitives. No bitmap is
  ever shown. A 17 KB embedded 110m Earth dataset (`world_map.bin`, 162 rings /
  4096 points, i16 lon/lat in 1/100°) ships in the binary, so the card ALWAYS
  renders the original world map from the first frame — no data, no wait.
  Optional downloaded chunks (city labels 110m, per-country 50m borders) stream
  from GitHub (`ezone134/zen-shell-map`), fetched on demand and verified by
  sha256, and replace the embedded fallback in place.
- **Download flow**: while the embedded fallback is on screen a compact
  "Download full map" pill overlays the map body. Clicking it spawns a curl
  worker that fetches world + cities chunks and the manifest, checks each
  file's sha256/size, and caches them atomically. Results land on a channel
  and are applied on the next services tick. Cached files render instantly at
  boot; the pill disappears once downloads replace the fallback.
- **Save-downloads toggle**: a menu switch decides the cache dir —
  `~/.local/share/zen-shell/maps` (persistent) or `/tmp/zen-shell/maps`
  (ephemeral). Default OFF (embedded fallback always works; the toggle only
  controls whether downloaded chunks persist across reboots). Persisted to
  config (`[world] save_data`). Flipping it clears the in-memory store and
  region rings so the card reloads from the new location.
- **4 style panes**, switched via a swipe-right menu (no swipe cycling, no
  pagination dots on this card):
  - *Real* — accent-tinted land + country border polylines + a 30° graticule.
  - *Filled* — clean continent silhouettes only (pixel-exact at the map's own
    resolution, vertical-run merged so they draw as a handful of rects).
  - *Dots* — dot-matrix land sampled from the landmask.
  - *Lines* — stroked borders + graticule, no fill.
- **Swipe-right menu**: an overlay with a backdrop (tap closes) + a row per
  pane + the save-downloads switch. Right-swipe opens it; a swipe while open
  closes it.
- **Zoom**: +/− buttons zoom around the pinned marker (or the map center),
  clamped 1×..32× with doublings/halvings. A touchpad **pinch** over the map
  body does the same continuously (zwp_pointer_gestures_v1; the scale from
  gesture start × base zoom, same 1×..32× band). All zoomed primitives are
  scissor-clipped to the card body, so nothing ever overflows the card.
- **Click → local time**: clicking the map pins an accent marker at that
  (lat, lon) and resolves the nearest timezone in `/usr/share/zoneinfo/zone.tab`
  (nearest-zone lookup, lon scaled by cos lat). The app probes its UTC offset
  once with a single `TZ=<zone> date +%z %Z` (`world_shell_probe`), then the
  card formats the local wall-clock + day line at draw time (pure libc shift,
  no per-frame subprocesses). The active zone re-probes for DST ~every 5 min.
- **Region detail**: cross the 4× zoom threshold inside a country and the card
  shows a "Download <Country>" pill; accepting the pill fetches that country's
  50m chunk, and its rings are drawn accent-stroked over the base map. A cached
  region chunk auto-loads while zoomed in.
- **Fit/click**: equirectangular 360°×170° (lat clipped to ±85) preserved
  inside the card body; projection is invertible (local click → lon/lat) via
  `worldmap`'s `win_for`/`fit`/`proj`/`unproj`; city labels appear from 2× zoom.
  The card's default span is (10, 9) — the 2.1:1 world needs height more than
  width on the ~6:1 grid cells.

## Weather v2 card

- **Minimalist "paper" card** (`DashCard::WeatherV2`, id `weatherv2`): the
  same installable card data as the classic Weather card, drawn on a **light,
  near-white rounded surface** — a deliberate contrast to the dark tiles.
- **Left hero**: `Today` + city at the top, a big condition glyph, the
  oversized current temperature, and a `H:`/`L:` high-low footer from the
  daily block (falls back to a single dense hero line when the span is squat;
  a `waiting…` placeholder while the first Open-Meteo frame hasn't landed).
- **Right strip**: a 5-row forecast column (today first, then the next four
  days) — a small per-day glyph on the left and a right-aligned temperature,
  evenly distributed down the card, separated from the hero by a hairline.
- **Click** opens the full Weather detail panel (shares `weather_rect`, so
  both weather cards lead to the same mode). Default span (8, 6) on its own
  row 17 of the default lattice; parked on the tray like every other card.

## Focus timer card (pomodoro)

- **`DashCard::Pomodoro` (id `pomodoro`)** — a focus timer on the grid: a
  big mm:ss Display hero, a phase progress bar and a `N done` session
  counter in the header meta.
- **Controls**: Start / Pause / Resume (accent chip) + Reset; `−`/`+`
  steppers adjust the focus length in 5-min steps (1–180) while idle. The
  phase label under the numerals reads `focus` (fg) or `break` (OK-green).
- **Phase machine**: when the deadline passes the timer flips focus ↔
  break, auto-starts the next phase and — after each FOCUS phase — bumps
  the session counter. `pomo_tick()` runs on a 1 s calloop timer that is
  armed only while a phase is running (`sync_pomo_timer`, re-synced on
  every click) so a paused timer costs 0 CPU.
- **Persistence**: focus/break lengths + session count live in
  `$XDG_DATA_HOME/zen-shell/pomo.txt` (`focus|break|done` lines), loaded
  in `seed_data` alongside the todos. The running deadline is
  session-local by design.

## Fans card

- **`DashCard::Fans` (id `fans`)** — live fan speeds from hwmon: every
  `fanN_input` across `/sys/class/hwmon/chipN` becomes one row
  (`fanN (chip)` label + right-aligned mono RPM), sampled with the 3 s
  services poll.
- **Relative bars**: each row's bar normalizes against a slow-decaying
  per-fan peak (`fan_peaks`, rise instant, decay ×0.995/tick) so spin-ups
  are visible without hardcoding maxima; stopped (0-RPM) fans dim. No
  sensors → the shared empty state (`no hwmon fan sensors`).

## Workspaces card

- **`DashCard::Workspaces` (id `workspaces`)** — Hyprland workspace tiles:
  one rounded tile per workspace (1-based, from `ws_count`), laid out ≤10
  per row with the grid adapting to the card span. The active tile is
  accent-tinted with the pill's signature dot below the number.
- **Click to switch**: each tile registers `WORKSPACE_KEY_BASE + i`
  (30_800…); clicking sets `ws_prev`/`ws_active` and queues
  `pending_ws_dispatch` — the exact switch path the pill's long-form
  chips use, consumed by app.rs against the Hyprland socket.

## Water-fill battery icons

- Both battery cards (`batteryv` / `batteryh`) draw their vector battery
  with a **liquid interior** (`draw_water` in shared.rs): the water spans
  the FULL inner cross-section so its edges touch the case stroke on both
  sides (reference: liquid-battery icon), over a quiet vessel tint.
- The surface carries a **wave crest** — an 8-segment sine line, lighter
  than the fill — plus a soft highlight band under it; crest orientation
  follows the battery (horizontal surface on the vertical battery, vertical
  surface at the leading edge of the horizontal one). Fill color still
  follows the shared charge ladder (`battery_level_color`).

## World clock card

- **`DashCard::WorldClock` (id `worldclock`)** — pinned cities with their
  local time: one row per zone with a sun/moon glyph (local hour 6–17 =
  day, moon otherwise), the city name, a right-aligned mono `HH:MM`, and a
  `+H:MM` offset-vs-local chip (hidden when the zone IS local time).
- **Add / remove**: a "+ add city" row opens an inline search over the
  cached `zone.tab` list (`worldmap::zones()` — parsed once, shared with
  the world map): type ≥ 2 chars, up to 6 city · tz matches render, click
  pins the zone (dedup by tz, max 16 rows). Hovering a row reveals a ✕.
  Esc closes the search; keyboard focus loss blurs it like the to-do
  composer.
- **Time math**: `HH:MM` and the day/night hour are computed at draw time
  from the zone's cached UTC offset (pure `gmtime_r` shifts — no per-frame
  subprocesses). The offset + abbreviation cache refreshes via ONE worker
  thread running `TZ=<tz> date +%z %Z` per pinned zone, delivered through
  a calloop channel; the app re-probes at most every 5 min and only while
  the dashboard is open (DST-safe, zero idle cost).
- **Persistence**: `$XDG_DATA_HOME/zen-shell/zones.txt` (`city\ttz` per
  line). First launch seeds UTC · New York · London · Tokyo. Keys:
  `WORLDCLOCK_DEL_BASE` 31_000 (row ✕), `WORLDCLOCK_ADD_KEY` 31_100,
  `WORLDCLOCK_PICK_BASE` 31_200 (search results, resolved against the
  `worldclock_pick_map` the last frame drew).

## Water tracker card

- **`DashCard::Water` (id `water`)** — daily hydration: a bead-ring gauge
  (36 beads, the CPU card's style) lit by the fraction of the daily goal,
  with the glass count in the middle (`✓` + green when the goal is met)
  and `−` undo / `+ glass` buttons below. One glass = 250 ml; the header
  meta reads `X.X / Y.Y L`.
- **Goal**: swipe horizontally over the card to adjust (1–30 glasses,
  same chain as the battery pane flips). Count caps at 3× the goal so a
  stuck click can't run away.
- **Midnight reset**: the count belongs to a `YYYY-MM-DD` key
  (`water_today()` via `localtime_r`); a mismatch on load or on any
  interaction rolls the day and zeroes the count.
- **Persistence**: `$XDG_DATA_HOME/zen-shell/water.txt` (`goal` / `day` /
  `count`), loaded in `seed_data`.

## Moon phase card

- **`DashCard::Moon` (id `moon`)** — current moon phase with zero data
  sources: the cycle fraction comes from a known new-moon epoch
  (2000-01-06 18:14 UTC) over a 29.53059-day synodic month; illumination
  is `(1 − cos 2πp) / 2`.
- **Vector disc**: a dark base disc + the lit portion as scanline
  slivers — each scanline's lit span runs from the terminator ellipse
  (`±r·cos 2πp`, half-disc at the quarters) to the rim, right side while
  waxing, left while waning. No bitmaps, consistent with the vector
  battery/water styling.
- **Readout**: phase name (8 classic names, each centered on its eighth
  of the cycle), illumination %, and days-until-full (`full tonight`
  within a day).

## Compositor / Effects card

- **`DashCard::Compositor` (id `compositor`)** — toggle tiles for blur /
  shadow / opacity wired to the shipped `*_main toggle` scripts through
  the `DashCmd` pipeline (`BlurToggle` / `ShadowToggle` /
  `OpacityToggle` → `pending_dash`), plus a screenshot row (full
  screen / area select → `shot_main`).
- **Live state**: `dash_status` prints `blur 0|1`, `shadow 0|1`,
  `opacity 0|1` read from the scripts' state files
  (`states/hypr_blur_$s`, `states/shadow_enabled_$s`,
  `states/opacity_active_$s`); the app parses these into
  `fx_blur/fx_shadow/fx_opacity` on the existing status poll and the
  tiles tint on/off accordingly.

## Countdown card

- **`DashCard::Countdown` (id `countdown`)** — days-until events:
  `label · date · Nd` rows sorted soonest-first; `today!` chip on the
  day itself; **past events stay, grayed**.
- **Composer**: To-Do-style inline composer, `name YYYY-MM-DD` on one
  line; dates parsed with libc (`localtime_r`) and compared with the
  naive day-number in `Shell::countdown_days` (today's day-number is
  1-based to match the computed day-of-year).
- **Persistence**: `$XDG_DATA_HOME/zen-shell/countdown.txt`
  (`label\tdate`).

## Alarm / reminder card

- **`DashCard::Alarm` (id `alarm`)** — `HH:MM` rows with labels and
  repeat arms (`once` / `daily` / `weekdays`, drawn as a small arms
  row per alarm); inline composer `HH:MM label`.
- **Firing**: the always-running 60 s clock tick checks every alarm
  against `HH:MM`; a due alarm pushes through the notification channel
  so the **real notification popup** appears. `once` alarms remove
  themselves after firing; repeating arms re-arm for the next matching
  day (a `fired` guard keyed to the day prevents double-fires within
  the minute).
- **Persistence**: `$XDG_DATA_HOME/zen-shell/alarms.txt`
  (`HH:MM\tlabel\tarms`).

## Snippets card

- **`DashCard::Snippets` (id `snippets`)** — saved text clips shown as
  name rows with a body preview; **click copies the body** via new
  `Clipboard::copy_text` (same data-control path as `set_entry`) and
  flashes `copied` on the row for ~1 s.
- **Composer**: name and body fields, add/remove inline.
- **Persistence**: `$XDG_DATA_HOME/zen-shell/snippets.txt`
  (`name\tbody`).

## Expense tracker card

- **`DashCard::Expenses` (id `expenses`)** — month-to-date spend:
  quick-add chips (`1 5 10 20`), a `amount [label]` composer (comma
  decimals accepted, default label `misc`), the **MTD total** in the
  header, and per-label bars for the current month ranked by amount
  (other months are excluded from the ranking).
- **Persistence**: `$XDG_DATA_HOME/zen-shell/expenses.txt`
  (`cents\tlabel\tdate`); `expense_month_summary()` sums and ranks in
  one pass.

## Testing

- `cargo test` — 139 tests, all green (layout/pack determinism, banner scroll
  modes, tray/chrome geometry, tray/board card transitions, wallpaper grid,
  rawvideo frame splitting + worldmap chunk/roundtrip, embedded fallback
  parse/land, mask, window/projection, manifest `region_at`, mapdata sha/cache
  formatting + the worldmap card drawer: all 4 panes, fallback map + download
  pill, menu + toggle, region pill, marker/city band; pomodoro phase
  flip/session count/pause-resume round-trip + card draw; fans card draw;
  workspaces tile registration + click dispatch; world-clock offset/hour
  math, pin dedup + persistence, row + search-overlay draw; water add/sub
  + goal clamp + button registration; moon phase math against real 2024
  new/full epochs, name/day-to-full mapping, disc draw across the cycle).
  World-store tests serialize on `TEST_LOCK` (shared process globals).
- Live workflow: `cargo build` (debug) → `pkill zen-shell` → Hyprland
  auto-respawns the new binary.