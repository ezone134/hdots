# zen-shell (Rust)

Zero-overhead Wayland shell bar. Pure Rust. GL-first, Vulkan opt-in.
Same design as the C++ raze-shell: a floating pill that morphs into a
dashboard on hover — Catppuccin Mocha, JetBrainsMono Nerd Font.

## Philosophy

- **0 CPU**: epoll sleeps on the wayland fd when idle. No free-running
  render loop, ever.
- **0 wasted frames**: draw only on configure acks / explicit dirt. Size
  changes are committed *alone* (never attach a buffer in the same commit as
  a size change — the compositor ignores it), then the ack triggers the draw.
- **Event-driven**: everything is a wayland event, a 1s clock timer, or the
  morph / hold timers (inserted only while animating, removed on completion).
- Flush the wayland socket immediately after queueing requests.

## Design

We love **minimal design** — clean, professional, modern, minimal. Not
Material, not decorative UI — the opposite: flat, quiet, one primary accent,
nothing that doesn't need to be there. Every pixel should look like it was
deliberately placed by a product designer.

- **Flat + hairline**: no drop shadows, no heavy chrome. Depth comes from
  *tone*, never decoration. Surfaces are defined with subtle `raised` tone and
  1px `hairline` outlines.
- **One accent**: `acc` is the only accent color the shell drives. Everything
  else is derived from `fg`/`bg`/`acc`/`sfg` (`ui::mix`) — never hardcoded.
  Semantic hues (danger red, uplink blue, thermal orange) are reserved for
  data, not chrome.
- **Monochrome-first**: surface hierarchy is shown by tone (`bg` → `raised`
  → `raised_hl`), text weight by `fg`/`fg2`/`fg3`.
- **Bare glyphs, not icon circles**: no filled circles behind icons, no
  rounded-pill icon tiles. Just the glyph.
- **Text-only inline controls**: small status/cycle chips (e.g. the Mirror
  card's `15fps`/`30fps`/`60fps` label) are *bare text* — no background pill
  behind them. Hover/interaction shows as a colour change (fg → acc) only.
  A background appears only where the UI needs a hit-zone the text alone can't
  provide (button rows, menu rows), never behind a header label.
- **Less is more**: strip secondary text, chevrons, status lines, and bullets
  when they don't earn their pixels. Resting state is empty; chrome appears
  only on hover / edit mode.
- **Restraint on corners**: small radii (8–12px); only the outer capsule
  keeps its pill morph.
- **Professional finish**: consistent typographic rhythm (one label weight,
  aligned to the `scale` grid), generous whitespace, no misaligned rows, no
  information clutter. If a layer of the UI doesn't pass "would I ship this
  to a paying customer at enterprise scale" — it isn't done.

Keep every new card, panel, and widget inside these rules.

## Stack

| piece        | crate                                     |
|--------------|-------------------------------------------|
| wayland      | `smithay-client-toolkit` 0.21 + calloop   |
| egl surface  | `wayland-egl` + raw EGL                   |
| GL           | `glow` (GLES3, desktop-GL 3.3 opt-in)     |
| Vulkan       | `ash` + `naga` (opt-in `vk` cargo feature; runtime `ZEN_VULKAN=1`) |
| text         | `cosmic-text` (pure Rust, fontconfig)     |
| dbus         | `zbus` — Notifications + StatusNotifierWatcher |
| icons        | `png` + `resvg` (SVG) + `image` (JPEG wallpaper thumbs) |
| clipboard    | `wayland-protocols-wlr` (wlr-data-control)            |

## Build & run

> **Use a debug build by default.** Always compile with `cargo build` (the
> `dev`/debug profile) — not `--release` — unless explicitly told otherwise.
> Debug builds are what we iterate on here. Use `--release` only when I
> specifically ask for a release build.

```sh
# From the workbench root (/home/tw/my_workspace):
cargo build -p zen-shell            # debug build (default — do this unless told otherwise)
cargo run -p zen-shell              # debug run (needs a running wayland compositor)
# cargo run -p zen-shell --release  # only when a release build is explicitly requested

# From inside this directory (zen-shell/), bare `cargo build` works too —
# Cargo walks up to the workspace root automatically.
```

**Vulkan backend:** `render_vk.rs` is compiled+linked only with
`cargo build --features vk` (ash + naga are optional deps). At runtime Vulkan
is still chosen via `ZEN_VULKAN=1` and falls back to GL if init fails; default
builds are GL-only and carry no Vulkan code at all.

**Artifacts go to `/home/tw/my_workspace/target`** (the workbench's shared
target-dir, set in `/home/tw/my_workspace/.cargo/config.toml`) — `/tmp` and `~`
are RAM-backed tmpfs on this box, but `my_workspace` resolves to `/acc/data`
(nvme disk). The debug binary lands at
`/home/tw/my_workspace/target/debug/zen-shell`.

## Workbench

`zen-shell` is **not a standalone workspace** — it is one app inside the shared
Rust workbench rooted at `/home/tw/my_workspace/`:

```
/home/tw/my_workspace/            ← workbench root (Cargo.toml = [workspace])
├── Cargo.toml                    ← members, [workspace.dependencies], profiles
├── .cargo/config.toml            ← shared target-dir (real disk, not tmpfs)
├── crates/                       ← workbench-wide shared crates
├── quickshell_rewrite_in_rust/   ← this app (binary `zen-shell`)
├── wfm-rs/                       ← wfm (pure Wayland file manager)
└── rust_color_from_wallpaper_engine/       ← color_from_wallpaper_engine (image → hex CLI)
```

Rules:

- One lockfile + one target dir for every app. Build with
  `cargo build -p zen-shell --release` from the workbench root (or `cargo build`
  inside this dir — Cargo walks up to the workspace).
- Common stack (`smithay-client-toolkit`, `wayland-client`, `libc`) is versioned
  once in `[workspace.dependencies]`, consumed via `dep.workspace = true`.
- Shared code lives in `crates/` (`crates/README.md`) — never duplicated.
- Profiles live at the workbench root; member `[profile]` tables are ignored.

Config: `config/shell.toml` (same schema as the C++ raze-shell).

Live test: `setsid nohup env ZEN_TRACE=1 .../debug/zen-shell > /tmp/zen.log 2>&1 < /dev/null &`
Kill with `pkill -x zen-shell` (`pkill -f` self-matches the bash command).
Tray test item: `cargo run --example tray_item` (kill with `pkill -x tray_item`).

## Pill transparency

The collapsed pill renders with a **semi-transparent dark background**
(`bg` color at ~69% alpha) so the wallpaper shows through the pill shape.
Expanded mode and panels keep the fully opaque background for readability.
The drop shadow behind the collapsed pill is removed (nothing to lift from
with a transparent bg).

## Pill shape & styling

By default the collapsed pill is a **capsule**. With the **Floating** toggle
**off**, the bar sits flush to a screen edge and the shape becomes a
**rectangle with only the two outer corners rounded** — top-anchored → the
bottom-left and bottom-right corners are rounded with a square top edge;
bottom-anchored → the top-left and top-right corners are rounded. The dashboard
and panels adopt the same edge-squared rectangle in non-floating mode.
Floating on keeps all four corners rounded (capsule / panel).

Across the resting pill every text module is drawn at a **uniform 14 px font
in the foreground color** (matching the clock); the battery icon keeps its own
fixed look. A **uniform `PILL_GAP` (6 px)** sits between every module (this is
shared by the size estimator and the drawer, so the pill is always sized
exactly to what it draws). The sun ⇄ moon **Brightness chip** (dark-mode
toggle) is shown or hidden via Settings → Pill → **Brightness chip**.

## Wallpaper

The wallpaper backend defaults to `command` with `set_wall {path}` — an
external script that receives the absolute path. The in-process daemon
backend (`[backends] wallpaper = "daemon"`) is disabled by default. The
wallpaper picker panel is 620×540 with a **4×5 thumbnail grid** (20 items
visible, scrollable), using `thumb:<basename>` cached thumbnails.

## Colors

**Live palette — `$states2/shell_vars.json`.** From now on the shell reads every
fg/bg/… color from `$states2/shell_vars.json` (JSON object: `fg`, `fg2`, `fg3`,
bg, bg2, bg3, acc, sfg, hover, …) — on boot and on every
`zen-shell ipc call colors reload`. The `[colors]` table in `config/shell.toml`
is only the boot-time fallback when a key is missing. The Settings accent
picker still overrides the accent at runtime.

### State dirs (important)

> **Always present — do NOT verify them.** The `states` and `states2` env vars
> are set by the Hyprland session for the whole life of the bar. Treat both
> directories (and the files below) as existing at all times; never check,
> create, or bail when they're missing. Read/write them directly.

- **`$states`** — session state that **syncs back to persist when the session
  ends**. Holds the master theme file `$states/themes`: one TAB-separated line
  per theme (single-mode = 16 columns, `dual` = 30 — dark half first, then
  light). This is the single source of truth for the theme picker. Also holds
  the **per-state shell config** `$states/shell_{n|d|l}` (see below).
- **`$states2`** — volatile runtime state that does **NOT** sync to persist
  (pure runtime, reset each session). `hypr_init_logic` and the `theme_main`
  pipeline write everything here:
  - `$states2/themes` — **JSON mirror** of the bash master `$states/themes`
    (generated by `emit_rust_theme_cards()` at boot). The Rust shell reads
    this for the theme picker; it never sources the bash file directly.
  - `$states2/shell_vars.json` — every live color (JSON object; legacy
    `$states2/shell_vars` `key=#rrggbb` lines are the fallback).
  - `$states2/m_dummy` — dark/light marker (`d` / `l`).
  - `$states2/cur_theme` — active theme id.
  - `$states2/s` — current channel (`n`/`d`/`l`).
  zen-shell reads all of it from here.

### Per-state shell config (`$states/shell_{n,d,l}`)

The shell's UI theme is per-state (`n` = normal, `d` = dark, `l` = light). The
current state's channel is read from `$states2/s` (a single char:
`n`/`d`/`l`); the matching per-state config is loaded from
`$states/shell_{state}` — a small TOML holding the per-state pill settings:

```toml
# $states/shell_d — per-state overrides for the "dark" UI state
pill_margin_x = 18  # px of side padding in the collapsed pill (0–30)
pill_alpha    = 176 # collapsed pill background alpha (0–255)
```

- `pill_margin_x` and `pill_alpha` are **per-state**: switching the channel
  swaps these values. Everything else lives in the base `config/shell.toml`.
- The channel + per-state config are read **on boot** and re-read on every
  `zen-shell ipc call colors reload` (`sync_per_state_config()`), so an
  external channel switch is picked up live — no bar restart.
- Values are persisted back to `$states/shell_{state}` on auto-save
  (`save_per_state()`). Both files (the per-state files and `$states/themes`)
  sit in `$states`, so they **sync back to persist at session end**. The
  channel flag lives in `$states2` and is re-derived each session.
- Default per-state values: `pill_margin_x = 18`, `pill_alpha = 176`.

Colors are stored as **`0xRRGGBBAA`** (alpha in the low byte), matching
`parse_color` and every palette constant in `shell.rs`. All decoders must
follow suit — `premul()` in `render.rs`/`render_vk.rs`, `mix()` in `shell.rs`,
and text colors converted via `((color & 0xff) << 24) | (color >> 8)` when
handed to skrifa's `Color` (`0xAARRGGBB`). Mixing these up swaps R/B and
reads the R channel as alpha — the "blue pill" bug (2026-08-14).

## Text rendering

- `cosmic-text` rasterizes glyphs into premultiplied RGBA8 textures.
- Textures are **tightly cropped to the ink bounding box** (1px margin) and
  drawn at **integer pixel positions** with **NEAREST** filtering — crisp
  glyphs, no smearing.
- Ink is vertically centered per item (glyph block centered in the pill),
  so digits and icons align with the bar.

## Morph (hover expand / collapse)

The pill ⇄ dashboard morph is an **ack-gated eased size commit**: a 16ms
morph timer advances an eased tween, committing each stepped size *alone*
(no buffer attach — the compositor ignores a buffer committed with a size
change), and the configure ack triggers the draw at the new size. The timer
runs only while a morph is in flight (0 CPU when resting).

**No bounce in either direction.** Both the growing (pill → dashboard / panels)
and shrinking (dashboard → pill) morphs settle with a plain **`ease_out_cubic`**
— a smooth ease-out with zero overshoot, so expand and collapse neither pop
nor bounce. (The corner-radius morph tracks the same easing, clamped to 0..1 so
the radius never overshoots, easing the capsule corners into the panel's corners
in sync with the surface.)

While a morph is in flight, contents are laid out at the mode's **target**
size (the frame alone follows the intermediate size), so the clock, chips and
cards hold still and are revealed/clipped by the growing surface instead of
reflowing every stepped frame — no "contents slide around" glitch on expand
or collapse.

Three bugs in that dance were found and fixed (2026-08-15):

- **Config must always load.** `find_config()` used to be cwd-relative, so
  launching the binary from anywhere but the app dir (workbench root, a
  keybind, …) silently fell back to the *default* config — and its
  `duration_ms = 0` made every morph complete in one tick, racing the ack
  gate and stranding the surface at the pill size ("hover never grows past
  the pill") or stuck expanded. Config is now resolved via
  `CARGO_MANIFEST_DIR` (plus the old candidates), so `duration_ms` is always
  the real 150ms.
- **A bogus off-surface enter no longer expands.** wlroots can focus a freshly
  mapped layer surface with the cursor still outside its bounds, delivering
  an enter whose position is outside the surface; honoring it expanded the
  bar with no leave ever coming ("bar does not morph back to pill
  sometimes"). Enters are only trusted when the position is inside the
  last-acked surface; motion into the bar also expands (covers a stuck seat
  focus).
- **A morph can't end a step short.** When a morph's completion tick hit a
  still-pending size commit, the final size was dropped and the timer
  removed, stranding the surface a step (or a whole mode) short. `tick` now
  keeps offering the target until it is actually committed, the timer runs
  until `last_committed == target`, and re-committing a size the compositor
  already knows is skipped (an unchanged-size commit is never acked and used
  to wedge `size_pending`, stalling the next morph for up to 800ms).

**The bar is expanded iff the cursor is over it.** The morph state is
reconciled against ground truth: on the 1s tick, when the pointer event flow
has gone stale (a panel open while we hold no pointer focus), `reconcile_hover`
queries Hyprland (`j/cursorpos` + `j/layers`) and projects the *target* size at
the  anchor — cursor over it → expand / stay expanded; cursor gone → collapse.
  This is why the dashboard never collapses mid-morph under the cursor, and
  why one that ends up away from the cursor collapses once the pointer never
  comes over. (Panels are modal — they don't reconcile with the cursor at
  all.)

Dismissal paths, by panel type:

- **Dashboard (Expanded)** — hover-driven: expands on hover, collapses on
  hover-out (and when the cursor is elsewhere, per the reconcile above). With
  the Settings → Pill **Expand on hover** toggle off, the cursor never morphs
  the pill: the dashboard is opened by a click / `zen-shell open expanded`
  and dismissed by a background click on it or Esc, and **right-clicking the
  pill always opens Settings**.
- **Modal panels (control center, notifications, calendar, power, wifi, bt,
  launcher)** — grab EXCLUSIVE keyboard while open and NEVER close
  on hover-out, so you can move the mouse freely (the powermenu stays up
  while you aim at its buttons). They close on Esc, a background click *on*
  the panel, or clicking any other window (keyboard-leave — the precise
  "clicked elsewhere" signal). The launcher also closes on Enter.

Clicking a tray chip, a media button, or a widget control keeps the current
mode. Also fixed: a panel's first frame used to render at the stale pill size,
producing a negative-width tile that panicked the rounded-rect clamp — the
radius clamp is now degenerate-safe and the CC tile width is clamped.

**Shape language** — the pill and the toast are capsules (radius = h/2); every
widget (dashboard, control center, launcher, notifications, calendar,
settings, power, audio, wallpaper, weather) is square-ish with a 20px corner
radius. The panel layouts draw through the shared component library in
`src/ui.rs` (`frame`, `card`, `chip`, `tile`, `slider`, `bar`, `media_btn`,
`text`), so a radius or shade change in one place restyles every panel — the
same idea as quickshell's QML component library.

## IPC (quickshell `qs ipc call` parity)

The bar listens on `$XDG_RUNTIME_DIR/zen-shell.sock`; the same binary,
invoked with an argument, is the client — the Rust answer to opening a panel
from a Hyprland keybind or any external tool:

```sh
zen-shell open launcher        # app launcher
zen-shell open cc              # control center
zen-shell open notifications   # notifications panel
zen-shell open calendar
zen-shell open settings        # settings panel (accent picker, stats)
zen-shell open power           # power menu (hold 3s to act)
zen-shell open audio           # per-app volume
zen-shell open wallpaper       # wallpaper picker
zen-shell open weather         # detailed weather
zen-shell open clipboard       # clipboard history — text (SUPER+V)
zen-shell open clipboard_images  # clipboard history — images (SUPER+SHIFT+V)
zen-shell open themes          # theme picker (SUPER+T)
zen-shell wallpaper set <path> # set the wallpaper daemon background
zen-shell reload colors           # palette-only hot reload from $states2/shell_vars.json
zen-shell colors reload           # same as above (legacy form)
zen-shell reload                  # hot-reload ALL configs (per-channel shell.toml)
zen-shell ipc call mode sync      # re-read dark/light state from $states2/m_dummy
zen-shell open expanded        # the dashboard (as if hovered)
zen-shell open lock            # lockscreen (fullscreen; unlock button / Esc)
zen-shell close                # collapse back to the pill
zen-shell toggle launcher      # open / close
zen-shell ipc call open launcher   # quickshell-flavored forms work too
zen-shell ipc call close
```

**Full command reference:** every target, alias and keybind example lives in
[`docs/ipc.md`](docs/ipc.md).

E.g. in Hyprland: `bind = SUPER, SPACE, exec, zen-shell open launcher`.
A bad handler replies `error: …` and exits non-zero; with no bar running it
prints `no bar running` and exits 1. The server is a calloop fd on the same
loop — 0 CPU when idle.

**Single instance enforced.** `main.rs` takes a `flock` on
`$XDG_RUNTIME_DIR/zen-shell.lock`; a second launch exits. Two bars at the same
anchor overlap and steal pointer focus from each other, which left one stuck
expanded ("bar stays dashboard-sized when not hovered").

Live-verified on Hyprland: hover → eased expand to 520×256, leave → eased
collapse to the pill, 10 rapid hover/leave cycles end on the pill with zero
skipped commits, and a duplicate launch exits cleanly.

## Panels & widgets

- **Dashboard** — a **Windows-10-Metro-style tile grid**: every card is an
  N×M span on a logical **20×12 cell lattice** inside a canvas that
  AUTO-FITS the packed content (`1×1`, `1×2`, `2×2`, `3×4`… any span),
  rendered as a seamless rounded mosaic with soft drop shadows. Top strip:
  workspace box, active-window title, tray + settings/power chips.
  - **Right-click enters EDIT MODE** (Win10 "customize" style): a dot
    lattice appears, **drag cards to move them**, **drag the corner grip
    (⌟) to resize spans** — others glide aside live, and dragging past the
    edge GROWS the canvas in real time (it trims back on drop). A **parking
    tray strip** appears at the bottom: drag a card onto it (or click its
    **✕**, top-right) to REMOVE it from the dashboard, drag a chip from the
    tray back onto the grid to re-place it. Parked chips/cards columns
    **stretch to fill the panel's full width** — no dead space on the right
    edge of wide windows. Edit-mode banner chips (on the strip and in the
    parked tray) render as **icons** (clock → clock glyph, wifi → wifi, gear →
    settings) instead of word labels, with a raised **drag handlebar** under
    each strip chip for easy grabbing. Right-click / Esc leaves
    editing. The layout — every enabled card with its size AND exact
    position, plus the parked tray — persists per channel to
`$states/shell_{n,d,l}` (the legacy order+spans
     `~/.local/share/zen-shell/dash_layout.txt` is still written as a
     fallback); unknown/hand-mangled entries fall back to defaults per card.
   - **Adaptive area collapsing — the rule.** The dashboard surface is never
     bigger than its content. When the dashboard is OFF — or ON but with
     **zero cards anywhere** (board + parked tray) — the board collapses to a
     **banner-only strip**; in edit mode that state keeps only the **banner
     chrome** (banner-chips tray + strip-editor row) and hides the card grid /
     cards tray / UI-controls row. Conversely, when the banner-chips tray
     holds **no parked chips** (only separator presets + the smart filler) it
     collapses so the cards area takes the space (`banner_chips_absent`).
     The strip-editor row carries two mirror toggles: **Enable dashboard
     cards** (left) and **Enable banner chips** (right) — turning the latter
     off hides every item on the top banner strip and collapses the band
     (`banner_collapsed`); it also hides the banner-chips tray.
- **Flow packer — how cards are placed.** Cards are a vertical **stack**:
     the first card sits at the top of the stack (placed first) and the last
     card is the last of the stack (placed last). Placement fills rows
     left→right and always prefers to **use up left-side space when a row
     has enough room**:
     - *Own-row left-fill:* a card may slide left at most ONE column to fill
       an adjacent gap — it never teleports all the way to the far-left empty
       cell of its row.
     - *Jump-up compaction:* when its own row is full, a card may jump up into
       a row above (only one row up, and only into a row that already holds
       cards). Because cards are ordered top-of-stack → bottom-of-stack, the
       **2nd row's LEFT card pops into the 1st row's RIGHTMOST empty slot** —
       keeping every row as left/top-packed as possible without scrambling
       card order.
     - Otherwise it scans down to the first row with enough room.
     The default grid is bit-for-bit preserved (regression-tested), and once
     a per-channel dashboard has been saved the cards are pinned to those
     exact remembered positions until the user edits again.
   - **Auto-arrange — HARD RULES** (edit mode only; runs ~every 3 s while
     editing and immediately after a card is ✕-deleted). Cards gravitate
     toward the top-left corner under exactly two moves — nothing else may
     ever relocate a card:
     1. *Left-fill (right → left):* a card shifts LEFT into the left-most
        free cells of its own row when a gap opens beside it. The old column
        it vacated frees the next card to its RIGHT to slide left into it —
        a cascading gravity feed that closes gaps column by column.
     2. *Up-fill (bottom → top):* when a row ABOVE has room, the **LEFTMOST
        card of the row below lifts UP into the RIGHTMOST free slot of the
        row above** (only into space its full width `w` fits; one row at a
        time, bottom rows processed first). Deleting a card at the top lets
        everything below it climb up through the freed space, filling each
        receiving row right → left.
     A card NEVER moves right into empty space, never jumps down, never hops
     to an arbitrary empty cell — it only falls toward the top-left corner.
  - Cards degrade gracefully at small spans (compact system card,
    icon-only toggle tiles, label-less sliders, single-gauge gauges…).
  - Card set: **System** (identity/clock/date chip → calendar/uptime/
    battery), **Weather** (glyph · temp · conditions · wind), **Media**
    (art/transport/drag-seek),
    **Network** (heartbeat ↓/↑ pulse graphs), **CPU/GPU** (two-line ECG-style
    chart; amdgpu sysfs / nvidia-smi auto-detect), **RAM/DISK bead-ring
    donuts**, **To-Do** (click ✓ / ✕, live composer field, persisted to
    `~/.local/share/zen-shell/todos.txt`), **Toggles** (Wi-Fi · BT · DND ·
    Caffeine · Sunset · Shader tiles, chevrons → full menus), **Sliders**
    (five tall iOS verticals — brightness/volume/mic stream live;
    saturation/Kelvin apply on drag release). All script values sync from
    `scripts/dash_status` every 3s while open.
  - **World map** (id `worldmap`) — vector world clock: an embedded 110m Earth
    dataset always renders the original map (downloadable chunks add cities and
    per-country detail via a compact pill + cache toggle); 4 style panes picked
    in a swipe-right menu; +/− buttons AND touchpad pinch zoom (1×..32×,
    scissor-clipped to the card); clicking a spot pins a marker and shows that
    place's local time via a nearest-timezone lookup.
  - **Weather v2** (id `weatherv2`) — the same Open-Meteo data on a light
    "paper" surface (a deliberate contrast to the dark tiles): a left hero
    (Today · city · big condition glyph · oversized temp · H/L high-low) and
    a right 5-row forecast strip (small glyph + temp per day, today first).
    Clicking it opens the Weather detail panel like the classic card.
  - **Focus** (id `pomodoro`) — pomodoro timer: big mm:ss countdown,
    start/pause/reset, ±5-min focus-length steppers, progress bar; focus ↔
    break auto-flip (break in green) and a "N done" session counter.
    Durations + sessions persist to `~/.local/share/zen-shell/pomo.txt`.
  - **Fans** (id `fans`) — live fan RPMs from hwmon (`fanN (chip)` rows,
    per-fan bars normalized against a slow peak; stopped fans dim).
  - **Workspaces** (id `workspaces`) — Hyprland workspace tiles, active
    accent-tinted; click switches workspaces (same dispatch as the pill).
  - **World clock** (id `worldclock`) — pinned cities with local times:
    sun/moon glyph per row, mono HH:MM, offset-vs-local chip; "+ add city"
    opens an inline zone.tab search; offsets refresh on a worker probe
    every 5 min (DST-safe). Persisted to `~/.local/share/zen-shell/zones.txt`.
  - **Water** (id `water`) — daily hydration tracker: bead-ring gauge with
    `+ glass` / `−` undo, goal adjusted by horizontal swipe, auto-resets at
    midnight (`~/.local/share/zen-shell/water.txt`).
  - **Moon** (id `moon`) — current moon phase as a vector shaded disc with
    phase name, illumination % and days-until-full (pure date math, no
    sources).
  - **Battery V / Battery H** (`batteryv` / `batteryh`) — two-pane battery
    cards whose vector battery fills like a LIQUID: the water spans the full
    interior, touching the case stroke on both sides, with a wave crest +
    highlight on its surface; pane 1 carries the power-save toggle + draw /
    remaining info.
- **Settings** (gear chip / gear chip on the collapsed pill / right-click the
  pill / `zen-shell open settings`) — an **Android-style settings list**, not
  a control center: slim rows grouped under section headers (**Network &
  internet**, **Sound & display**, **Appearance**, **Pill**, **System**),
  each with a tinted icon square, a label + summary, and a switch or chevron.
  Wi-Fi / Bluetooth rows open their submenus (`zen-shell open wifi` / `bt`)
  and carry a power switch; brightness + volume are inline sliders; the
  accent picker shows the current color dot + 6 Catppuccin swatches (the pick
  overrides `colors.acc` at runtime, no restart); the **Pill** section
  toggles, at runtime: **Expand on hover**  (off → the pill never morphs from the cursor — the dashboard opens by click /
  IPC and **right-clicking the pill always opens Settings**), **Floating**
  (on → hovers with a 25px margin over content; off → sticks flush to the
  top/bottom edge), **Reserve space** (on → the resting pill takes an
  **exclusive zone** and pushes windows below/above instead of floating over
  them), what the collapsed pill shows (**Workspaces / Clock / Battery / Tray
  icons / Settings chip** — the gear chip on the collapsed pill → Settings),
  and a **Position** row with ▲/▼ to float the pill top/bottom. Seeded from
  `[bar]` in `config/shell.toml` (`expand_on_hover`, `floating`, `reserve`,
  `show_ws`/`show_clock`/`show_battery`/`show_tray`, `settings_on_pill`,
  `anchor = "bottom"|"top"`), flipped live from Settings. The Pill section
  items (Workspaces / Clock / Battery / Tray / Visualizer / Settings chip /
  Notifications / User) have a **⠿ grip handle** on the left — **click to
toggle, drag to reorder** (press + hold + move vertically to swap items;
  click without moving toggles the setting on/off). A System card (CPU / RAM /
  disk bars + net up/down) and an About line close the list.
- **Control center** — Wi-Fi + Bluetooth tiles with `>` submenus, the media
  card, volume + brightness sliders, and an Audio + Wallpaper tile row.
- **Power menu** (power chip / `zen-shell open power`) — Lock / Logout / Sleep
  / Restart / Shutdown. Actions are **hold-to-confirm**: press and hold the
  button for 3 s while a liquid fill rises from the bottom; the action fires
  when the fill completes, and releasing (or sliding off) cancels it. Esc
  closes.
- **Audio** (`zen-shell open audio` / CC tile) — per-app volume via
  WirePlumber's `wpctl`: output devices, input devices, and every playing /
  recording app stream, each with its own slider and mute toggle. Polled only
  while the panel is open (no subprocess churn elsewhere).
- **Wallpaper picker** (`zen-shell open wallpaper` / CC tile) — a thumbnail
  grid that **fills the whole card**: columns × rows derive from a zoomable
  cell size (**Ctrl+= / Ctrl++ grows, Ctrl+- shrinks**, clamped 56–160px and
  **remembered in `[wallpaper] cell`**), so small sizes show many rows and
  big ones show a few large tiles. Every image in `[wallpaper] dir`
  (png / jpg / jpeg / gif) is reachable: cells are laid out by visible slot
  (scrolling slides the window, never walks cells off-panel), wheel scrolls
  a row per notch, two-finger trackpad scroll follows the natural
  fingers-down direction, and the **scrollbar is grab-and-draggable**
  (thumb drag keeps its in-thumb offset; pressing the empty track
  page-jumps). Clicking a tile sets the wallpaper via `set_wall {path}`
  (or the native daemon when `[backends] wallpaper = "daemon"`); thumbnails
  come from a 256px disk cache rasterized at the requested cell size.
- **Wallpaper daemon** (swww-like, no external binary) — a `Layer::Background`
  surface spanning the monitor renders the current wallpaper **cover-fit**
  with a 0.25 s fade-in; set via the picker or `zen-shell wallpaper set
  <path>` (boots to the first image in `[wallpaper] dir`). Re-fits on monitor
  resize; the picker panel doubles as its control UI.
- **Theme picker** (`zen-shell open themes` / SUPER+T) — a card grid read
  from `$states2/themes` (the JSON mirror generated at boot by
  `emit_rust_theme_cards()` in `hypr_init_logic`). Each card paints its
  palette — bg surface with fg / sfg / acc swatch dots — and a **dual
  theme shows BOTH halves in one card** (dark left, light right): no
  dark/light label, the colors speak for themselves. The active theme
  (`$states2/cur_theme`) gets an accent dot. Wheel scrolls a row of cards
  per notch; clicking a card sources `$states/themes` inline, writes the
  scheme files, and calls `theme_main restore` (or `theme_main <mode>` on
  a mode switch). The bar re-reads the new palette from
  `$states2/shell_vars.json` ~1.5 s later via the delayed colors-reload.
  > **Do NOT edit `theme_body`.** The shared color/theme pipeline (in
  > `$hypr_sources/helpers/theme_body`, sourced by `theme_main restore`) is
  > off-limits — it is the system-critical generator that turns the selected
  > `$states/scheme_<cm>` into the palette. `theme_main_body` (`scripts/`)
  > may be edited.
- **Dashboard identity card** — the user/host card carries a **dark/light
  chip** next to the bell (moon in dark mode, sun in light). Clicking it fires
  `theme_main switch`; the real state comes back from `$states2/m_dummy`
  (`d` / `l`) via the delayed self-sync or `zen-shell ipc call mode sync`.
- **Clipboard manager** (`zen-shell open clipboard` / `clipboard_images`;
  SUPER+V / SUPER+SHIFT+V) — the bar is its own cliphist: it observes the
  seat's clipboard via **wlr-data-control**, keeps a **text + image history**
  (persisted to `$XDG_STATE_HOME/zen-shell/clip.json` + `clipimg-*.png`),
   and clicking a row / pressing Enter re-copies it. Arrow keys navigate and
  wheel / two-finger scroll moves the highlighted row;
  text entries show a preview, images show thumbnails. On boot it also
  **seeds the history from cliphist's own databases** (`src/cliphist.rs` — a
  dependency-free read-only bbolt parser): text from
  `$XDG_CACHE_HOME/cliphist/db`, images from
  `$XDG_CACHE_HOME/cliphist_image/db` (the separate-db `cliphist -db-path`
  setup; images found inside the text db are routed to the image list too),
  so the panels show your long-running cliphist history (up to the cap) even
  before zen-shell records anything itself.
- **Corner notification popup** — `notify-send` notifications appear in their
  own **top-right layer surface** (not the OSD, not the bar): up to 3 stacked
  cards (icon, summary, body), auto-dismissed after ~6 s; clicking one opens
  the notifications panel. Separate GL surface, so it never disturbs the bar.
- **Weather detail** (click the dashboard weather line / `zen-shell open
  weather`) — big temp + condition glyph, feels-like, humidity, wind and
  precipitation from Open-Meteo (fetched alongside the dashboard weather in
  the same 10-minute poll).
- **Lockscreen** (`zen-shell open lock`) — a real lock, no external binary:
  the native backend grabs the session through **ext-session-lock-v1** (the
  same protocol hyprlock uses) into a fullscreen lock surface. That is what
  tells Hyprland the session is locked, so **the compositor's own keybinds
  stop firing** (only `bind … , l`-flagged binds still work) and **all
  keyboard + pointer input goes to the lock surface and nowhere else** — a
  locked screen really swallows everything. The lock surface shows the
  **current wallpaper, blurred, behind a dim** (half-res software Gaussian,
  faded in over 0.25 s) with the hero clock + date, an **avatar + username**
  line, a **password field** — type your user password and press Enter to
  unlock, verified through **PAM on a worker thread** (never stalls the
  loop; denials log the PAM error code so faillock lockouts are diagnosable)
  — a **caps-lock badge** when caps is on, and three buttons: **Logout**,
  Restart and Shutdown (**hold 3 s** with the same liquid fill). Esc only
  clears the field — a real lock never dismisses itself. On compositors
  without ext-session-lock the bar falls back to morphing fullscreen with an
  exclusive keyboard grab (visual only — binds still fire). Backend-swappable:
  `[backends] lockscreen = "native"` (default) | `"hyprlock"` (external locker).
- **OSD** (morphed pill overlay) — auto-shows and fades back to the pill after
  ~1.4 s on **AC plug / unplug**, **volume / brightness change** (keys,
  slider, or external), and **mute / unmute**; only morphs from the resting
  pill (a panel in the way is its own feedback).
- **Wi-Fi menu** (`> ` on the Wi-Fi tile / `zen-shell open wifi`) — live
  network list from NetworkManager (ssid, lock, strength bars, connected
  checkmark); click to connect via `AddAndActivateConnection` (open networks
  connect directly; secured ones need a saved keyring). Opening the menu
  triggers a rescan; the tile toggles now really power the NM radio.
- **Bluetooth menu** (`> ` on the Bluetooth tile / `zen-shell open bluetooth`)
  — device list from bluez with connected state; click connects/disconnects,
  and the tile toggle powers the adapter (`SetPowered`).
- **System stats** (`stats.rs`) — CPU % (from `/proc/stat` deltas), RAM %
  (`/proc/meminfo`), disk % (`statvfs`), **GPU %** (auto-detected once:
  `amdgpu` sysfs `gpu_busy_percent`, falling back to `nvidia-smi`; hidden in
  the UI when neither exists), and net up/down rates (`/sys/class/net`
  deltas) — pure std, polled on the existing 3s timer but ONLY while the
  dashboard or Settings is on screen (the /proc reads sleep in every other
  mode; the first poll after waking just spans the gap). Shown in the
  dashboard and the Settings card. **Battery % / AC / wattage** now come
  straight from `/sys/class/power_supply` (no upower daemon; see Backends).
- **Launcher** (`zen-shell open launcher`) — app search with **arrow-key +
  Enter navigation** (or Tab/Shift-Tab) — **holding Up/Down auto-repeats**
  through the list (the keyboard is created repeat-capable), a visible
  **scrollbar** when the hits overflow, and **wheel + two-finger trackpad
  scrolling that moves the selection** (rofi-style: each notch / accumulated
  smooth delta steps the highlighted row, and the visible window scrolls
  along to keep it in view).
- **Weather** (`weather.rs`) — Open-Meteo (no API key) fetched by a detached
  thread over a minimal std-only HTTP GET (chunked-encoding aware; plain
  HTTP, no TLS deps), pushed over a calloop channel, refreshed every
  `interval_s` (default 10 min). Dashboard line: `☁ 30°C Cloudy · New Delhi`.
  Configure `[weather] latitude/longitude/city` in `config/shell.toml`;
  unset → hidden.
- **Workspace previews** — hovering the workspace box in the dashboard shows
  that workspace's window titles (most recently focused first), fetched via
  `j/clients` on hover and cached per workspace.
- **Notification history** — notifications persist to
  `$XDG_STATE_HOME/zen-shell/notifs.json` and reload on start, so the panel
  survives a bar restart.

## Backends (pluggable system integrations)

Every system integration lives behind a small trait in `src/backends.rs`,
selected by the `[backends]` config table — swapping an implementation is a
one-line config change, never a code rewrite:

```toml
[backends]
power      = "sysfs"    # sysfs | upower
brightness = "sysfs"    # sysfs
audio      = "wpctl"    # wpctl (PipeWire / WirePlumber)
lockscreen = "native"   # native | hyprlock
wallpaper  = "command"  # command (e.g. set_wall {path}) | daemon (in-process bg layer)
```

The rule: **read `/proc` / `/sys` when it can, and never depend on an external
binary when it can be done in-process.** Battery % / AC / wattage come from
`/sys/class/power_supply` (`upower` is the fallback), brightness is a plain
write to `/sys/class/backlight/*/brightness`, and the lockscreen is the bar
itself (no hyprlock). `wpctl` remains the audio backend because PipeWire has
no stable filesystem interface — but the trait means a pipewire-dbus impl can
slot in without touching the UI.

### Bash script style (scripts/)

The helper scripts under `scripts/` are user-facing: they are sourced by the
Hyprland session and also callable from a terminal or keybind. Write them so a
user can read a script top-to-bottom and understand it in a few seconds.

- **Avoid external text-munging binaries when a builtin will do.** Prefer
  `$(<file)` over `cat file`, `${var//[!0-9]/}` over `tr -dc '0-9'`,
  `${var%% *}` / `${var#* }` over `cut`, and `case` over `grep -q`.
  The rule is about *text processing*, not the system CLIs the scripts wrap
  (`wpctl`, `pactl`, `hyprctl`, `nmcli`, …).
- **Prefer `(( ))` over `[[ ]]` for arithmetic.** `(( m > max ))` reads like
  math and is shorter. Keep `[[ ]]` for string/pattern tests and regex.
- **Structure as use-case → if/else → function call.** A reader should scan the
  top-level `case "${1:-…}" in` and immediately see the branches, then follow
  each into a named function. Avoid dense one-liner pipes for logic that matters.
- **Persist state in `$states2/` files, not by re-querying the device.** Read the
  file for the current value; query the device only when the state file is missing
  (see `audio_body` `vol_ensure`). This keeps polling and keybinds cheap.
- **Name small formatting helpers.** `vol_fraction` / `mic_fraction` (integer
  hundredths → wpctl fraction string) are clearer than inlining the arithmetic.

## Services (dbus daemons, one thread each)

- `notif.rs` — `org.freedesktop.Notifications` daemon (bubble popups).
- `tray.rs` — `org.kde.StatusNotifierWatcher` daemon + host. Items register
  over dbus; the daemon thread introspects (IconName / IconPixmap / Title /
  Status / ToolTip / Menu), forwards `TrayMsg` to the shell, and shell
  clicks/scrolls come back as `TrayCmd::Activate` / `Scroll`. **Never do a
  blocking zbus round-trip inside a served method handler** — handlers run on
  the zbus executor thread and deadlock. Handlers only push to a
  `std::sync::mpsc`; the daemon loop does the calls.

## Layout

```
src/
  main.rs     entry
  config.rs   shell.toml + parse_color (0xRRGGBBAA)
  backends.rs pluggable system integrations (sysfs/upower, wpctl, lockscreen, wallpaper)
  app.rs      wayland handlers, 3 layer surfaces (bar / wallpaper / notif popup),
              size dance, render driver
  render.rs   EGL + GL: rounded-rect SDF pass + premultiplied text pass
  render_vk.rs Vulkan backend (compiled only with `--features vk`; runtime ZEN_VULKAN=1)
  text.rs     cosmic-text → premultiplied RGBA textures
  shell.rs    pill state machine + scene layout (collapsed / expanded)
  ui.rs       reusable components: frame, card, chip, tile, slider, bar,
              media_btn, text (the Rust analogue of quickshell's QML lib)
  clipboard.rs wlr-data-control clipboard manager (text + image history)
  tray.rs     StatusNotifierWatcher daemon + host
  notif.rs    Notifications daemon
  img.rs      image store (icon:<name> / tray:<id> / clip:<n>)
  apps.rs     launcher app list + icons
  services.rs spawns the dbus service threads
  stats.rs    system stats (/proc + statvfs, 2s timer)
  weather.rs  Open-Meteo fetch thread + minimal HTTP GET
  hypr.rs     Hyprland IPC (cursorpos / layer geometry / clients ground truth)
  ipc.rs      unix-socket IPC server + client (`zen-shell open …`)
  bin/font_diag.rs  font & glyph diagnostics (cargo run --bin font_diag)
examples/
  tray_item.rs  minimal SNI test item
```## Roadmap

Implemented since `399f9e5` (2026-08-16): stats polling gated to the dashboard
at a 3s cadence; morph no longer reflows contents; one master hover rule
(cursor over the bar ⇔ expanded); **draggable sliders**; a **native
lockscreen** (fullscreen morph, caps-lock badge, hold-to-confirm restart /
shutdown); **OSD** on AC / volume / brightness / mute; launcher **scrollbar +
keyboard navigation**; the `commands` file; battery / AC / brightness from
**`/sys`** behind the `[backends]` layer; per-app **Audio** + **Wallpaper**
picker + **weather detail** panels; a **clipboard manager** (text + image
history via wlr-data-control, SUPER+V / SUPER+SHIFT+V); a **native wallpaper
daemon** (background layer, cover-fit, no external binary); and
**notifications in their own top-right corner surface** (separate from the
OSD).

The full, prioritized plan — multi-monitor, real PAM lockscreen auth, wallpaper
transitions, JSON-RPC IPC, plugins, distribution, fuzzing, and more — lives in
[`roadmap.md`](roadmap.md).

## Implemented 2026-08-28 (this session)

- **Faster startup — bar/launcher appear instantly.** The blocking reads that
  used to run before the first frame (hyprctl ws/active-win, D-Bus
  ssid/media, battery/AC, sysfs brightness, `wpctl get-volume`) were moved
  out of `App::new` into a new `App::seed_boot_data()` that runs **right
  after the first frame is committed** at the initial wayland roundtrip.
  `App::new` still builds the cheap backend objects (sockets/structs) but no
  longer blocks on the data reads, so rendering + input are interactive
  immediately; the 3s services / 60s power timers take over from there.
- **Custom accent section in Settings → Appearance.** New **ACCENT SOURCE +
  CUSTOM ACCENTS** UI. *Accent source* buttons: **From wallpaper**
  (`theme_main accent acc_from_wall`) and **Scheme default**
  (`theme_main accent theme_default`). *Custom accents* reads
  `$states2/custom_acc.json` (the `[{name,d,l}]` list regenerated by
  `gen_launcher_cache`/`gen_custom_acc_files`, now also mirrored into
  `$states2/`); the *custom accent* list is a **collapsible dropdown** — a row
  shows the current selection, clicking it opens a **scrollable popup** (wheel
  scrolls its own list); picking a name runs `theme_main accent custom_acc
  <name>`. Every selection routes through
  the `$states/acc_source_<ch>` flag + GTK hot-reload and schedules a colors
  reload so the live palette updates without a restart.
- **`theme_accent_func` (theme_main_body)** — now writes the canonical
  `$states/acc_source_<ch>` flag (`w`/`c`/`h`/`d`) and only runs
  `theme_main restore` (and `acc_changed`) on an **actual source change**;
  `define_hex` with no payload reads `$HOME/documents/defined_hex` via
  `$(<file)`, no `cat` subprocess.
- **Accent toggles + Alpha** — Settings → Appearance now has an **ACCENT
  TOGGLES** section (`custom_acc_start_icon` / `_app_border` / `_hyprland` /
  `_gtk` / `_normal_app`, plus the `start_icon_tone_<ch>` 0–900 slider) and an
  **ALPHA** section (`bg_alpha` / `acc_alpha` / `scrim_alpha` /
  `border_alpha` in lowercase hex, plus the `acc_scrim_<ch>` toggle). They
  route through `theme_main toggle_acc | start_icon_tone | alpha | acc_scrim`
  and an **Apply changes** button runs `theme_main restore`. Unlike accent
  *source* picks, these do **not** write `$states2/acc_changed` — a plain
  `theme_main restore` is enough (acc_changed only gates the GTK/thunar
  hot-reload). The pane scrolls to fit the new sections.

## Implemented 2026-08-25 (this session)

- **Live colors from `$states2/shell_vars.json`** — the bar reads every fg/bg/…
  from `$states2/shell_vars.json` on boot and on `zen-shell reload colors`
  (config `[colors]` is the fallback only). `$states` syncs back to
  persist at session end; `$states2` never does — it is pure runtime state.
- **Theme picker** (`zen-shell open themes`, SUPER+T) — card grid from the
  master file `$states/themes`: bg/fg/sfg/acc painted per card, dual themes
  show both dark + light halves in one card, active theme marked, click →
  `theme_main apply <id>` with an automatic delayed palette re-read. The
  script pipeline is untouched.
- **Dark/light chip on the identity card** — moon/sun toggle fires
  `theme_main switch`; state mirrored from `$states2/m_dummy` via
  `mode sync` (IPC or dashboard open).
- Two new IPC handlers: **`colors reload`** (re-read all colors from
  `$states2/shell_vars.json`) and **`mode sync`** (re-read dark/light state).
- keybinds: SUPER+T now opens the theme picker.

## Implemented 2026-08-27 (this session)

- **Per-state (UI theme) config** — the shell reads the current channel from
  `$states2/s` (`n`/`d`/`l`) and loads the matching per-state config
  `$states/shell_{n,d,l}`. It exposes two new **per-state** settings in the
  Settings → Pill panel: **Margin** (key 36, 0–30 px) and **Transparency**
  (key 37, 0–255 alpha), which set `pill_margin_x` / `pill_alpha` respectively
  and are persisted to `$states/shell_{state}` on auto-save
  (`save_per_state()`). The channel + per-state config are re-read on every
  `zen-shell ipc call colors reload` (`sync_per_state_config()`), so an
  external channel switch is picked up live — no bar restart. Defaults:
  `pill_margin_x = 8`, `pill_alpha = 176`.
- **Path centralization (`src/vars.rs`)** — both session dirs (`$states`,
  `$states2`) and the channel / per-state-shell paths live in one module so the
  source is a single edit. The doc comments repeat the rule: the env vars are
  set by Hyprland for the bar's whole life, so never verify / create / bail on
  those directories.
- **Dashboard flow pack — no far-left teleport.** `pack_cards_max()` no longer
  scans from x=0 for every placement; a card may now slide left at most ONE
  column to fill an adjacent gap (`try_place` with `max_left=1`) instead of
  hopping all the way to the far-left empty cell of its row. Cards are a
  stack: first card = top of stack (placed first), last card = last of
  stack. The "jump up one row" fallback (current row full) fills the
  **rightmost** empty slot of the row above — so the **2nd row's LEFT card
  pops into the 1st row's RIGHTMOST slot** — and only into a row that
  already holds cards. The default grid is bit-for-bit preserved
  (regression-tested).
- **Dashboard layout saved as an array in `$states/shell_{n,d,l}`.**
  Every ENABLED card is a `[dash_cards]` entry (`card` id + `x`,`y`,`w`,`h`
  cells) holding its exact order, size AND position; parked cards live in
  `[dash_tray]` as id strings. Saved per channel on any pill-settings change
  and on every edit-mode commit. On load, cards are pinned to those exact
  positions until the user edits again. The legacy order+spans
  `dash_layout.txt` is still written as a fallback.
- **Morph: spring pop on open, settle on close.** Growing morphs use the
  existing `ease_out_back` overshoot spring; shrinking morphs use a clean
  `ease_out_cubic`. The corner-radius morph follows the same easing (clamped)
  so the capsule corners track the surface.

## Implemented 2026-08-24 (this session)

- **Metro grid dashboard + edit mode** — the fixed card mosaic became a real
  tile grid: cards are `DashCard` spans (`x,y,w,h` in cells) on a 12×6
  lattice, drawn by nine position-independent drawers that adapt to any
  span. Right-click toggles EDIT MODE with move-drag, corner-grip resize,
  cell-snapped ghost previews (green/red validity), and persistence to
  `dash_layout.txt`. Slider/seek hit-testing now reads runtime geometry
  written by the drawers (no absolute consts), so everything stays correct
  wherever a card lands. (`src/shell/mod.rs`, `src/shell/panels.rs`)
- **Heartbeat graphs** — new `Cmd::Line` primitive: thick capsule segments
  rendered as *rotated quads* whose linearly-interpolated local coords feed
  the unchanged SDF shader (both GL + Vulkan backends, zero shader edits).
  Net ↓/↑ and the CPU/GPU chart now draw true diagonal pulse lines instead
  of filled hills (`ui::pulse`). (`src/ui.rs`, `src/render.rs`,
  `src/render_vk.rs`, `src/app.rs`)
- **To-Do card** — click ✓ to complete, ✕ to delete, live composer field
  (click, type, ⏎ chains tasks; Esc blurs; keyboard-leave blurs too),
  persisted as `~/.local/share/zen-shell/todos.txt`.
- **Wide dashboard rework (now 1120×480)** — the dashboard is now a polished
  3-column card mosaic instead of the old two-column layout: a bigger
  identity/system card, clock + media cards in row 1; **network gets its own
  card** with stacked ↓/↑ history graphs; **CPU and GPU share one chart with
  two separate lines** (GPU auto-detects amdgpu sysfs / `nvidia-smi`);
  **RAM/DISK became circular bead-ring donut gauges** with live % centers;
  quick toggles moved into their **own 2×3 tile card**, and the five vertical
  iOS sliders sit beside them in their own card. Saturation / color-temperature
  apply strictly on **drag release** (one script invocation per gesture).
  (`src/shell/panels.rs`, `src/shell/mod.rs`, `src/stats.rs`,
  `scripts/dash_status`, `scripts/shader_main`, `scripts/hs_main`)
- **Tide-island battery icon** — `ui::battery` ported from the quickshell
  `BatteryIcon.qml` reference (`~/.config/quickshell/components/`): a
  translucent rounded body (~56%), a solid level fill growing from the left
  that fully rounds off at ≥85%, the **level number drawn inside** the
  silhouette (glyph shade auto-flips dark/light from fill luminance),
  number **+ bolt** while charging, and a terminal tip that lights solid at
  100%. Red ≤20% via the existing caller logic. The collapsed pill renders
  it at 13px with no external "NN%" text; the dashboard card shows only the
  time estimate beside it. (`src/ui.rs`, `src/shell/mod.rs`,
  `src/shell/panels.rs`)
- **Settings › Pill drag-to-reorder actually works now** — root cause: the
  main surface's pointer-press handler went straight to `shell.click()`,
  which toggled the row on press, so no drag state was ever created (the
  intercept existed only in the lock-surface branch). Press now starts the
  drag first; tap-without-move toggles on release. Rows lift and follow the
  cursor with live swapping; order persists to shell.toml. (`src/app.rs`
  press arm, `src/shell/mod.rs`)
- **Wallpaper picker overhaul**
  - *Full list reachable*: cells were positioned by absolute index, which
    walked them off the panel past page one. Cells are laid out by visible
    slot now, wheel scrolls whole rows, and max-scroll lands flush with the
    last page.
  - *Grab-and-drag scrollbar*: press the thumb to drag with its in-thumb
    offset; press the empty track to page-jump then drag; wide invisible hit
    strip beside the grid, hover/drag highlight.
  - *Zoom*: **Ctrl+= / Ctrl++ grows, Ctrl+- shrinks** the thumbnails
    (numpad keys too, holding auto-repeats), clamped 56–160 px and persisted
    to `[wallpaper] cell` in shell.toml. Columns × rows derive from panel
    size × cell size, so the grid fills the card edge to edge at any zoom
    (96px default → 5 columns).
  - *Trackpad scroll flipped*: smooth `absolute` axis deltas are no longer
    sign-negated — two-finger scroll runs natural fingers-down; mouse-wheel
    notches unchanged.
- **Scroll = move selection** — in the launcher (and both clipboard lists)
  the mouse wheel / two-finger trackpad scroll **moves the highlighted row**
  instead of just scrolling the viewport; the visible window auto-scrolls to
  keep the selection in view. Arrow keys behave as before.
- **No more lingering panel after a selection** — dismissing a widget
  (launching an app, picking a clipboard row, Esc, click/IPC close,
  click-away) disarms hover-expansion until the pointer actually moves. A
  cursor that merely rests over the pill strip right after a dismissal no
  longer pops the dashboard back open ("it stays big and takes seconds to
  return to the pill"); one real mouse movement re-arms hover as usual.
- **Slider drags can't collapse the dashboard anymore** — dragging the
  saturation (shader) or color-temperature sliders used to let the dashboard
  snap shut mid-gesture: their backend scripts (`shader_main`,
  `hs_main`) restart backends AND `notify-send`, and the popup
  mapping bounces pointer focus so a Leave read as "cursor left". The hover
  rule now never collapses while a drag is live (`pointer_down` +
  `drag_key`), plus a 2.5 s post-release grace window covers the script's
  notify; a genuine move-away still closes as usual.
- **EDIT MODE is collapse-proof** — while `dash_edit` is on the dashboard
  NEVER hover-collapses (a wandering cursor or a stolen focus can't yank
  the canvas away); only Esc / right-click / explicit dismissal leave it.
- **Cards are borderless — shadows only** — edit-mode per-card outlines are
  gone; the lifted card gets a soft accent GLOW shadow instead of a border.
- **Fluid edit mode** — the moved card rides 1:1 under the cursor, the
  resized card's corner tracks it continuously, and every other card GLIDES
  to its re-packed slot (exponential ease driven by a 60 fps settle timer
  that stops itself when the layout is at rest).
- **Auto-fitting dashboard canvas** — the background is no longer a fixed (or
  manually resized) rect: it AUTO-TRIMS to exactly what the packed cards use.
  Width = rightmost card edge, height = lowest row — no empty strips on the
  right, no dead rows at the bottom. Shrink a layout and the canvas shrinks;
  while editing, dragging or resizing a card past the edge GROWS the canvas
  live under your cursor (the surface re-morphs during the drag), and it
  trims back on drop. Cell size stays anchored to the configured width so
  auto-fit never rescales the lattice.
- **Hole-free default layout (v3)** — the legacy default spans summed to more
  than the grid and even overlapped, so flow-packing them left gaps and dead
  rows (the "empty spaces" on the bg). The new defaults tile the 20×12
  lattice EXACTLY — System 6·Clock 5·Media 9 / Network 6·CpuGpu 6·Gauges 8 /
  Toggles 7·Todo 5·Sliders 8 — and the saved-layout marker bumped to
  `zen-layout-v3`, so pre-existing files fall back to the tight defaults
  once. Exiting edit mode also re-morphs to the committed extent (no stale
  oversized bg).
- **Bigger notification popups** — corner popup cards grew from 352×64 to
  470×92 (122 with action chips), larger icon/text, drop shadows, and one
  shared geometry source (`shell::notif_popup_size`) so the surface is
  exactly as tall as its content (action rows no longer clip).
- **Network card: numbers over graphs** — the ↓/↑ history charts became two
  big live readouts with glyphs: ⬇ download (blue) / ⬆ upload (accent),
  formatted as bits ("12.5 Mbps" / "340 Kbps" via `stats::fmt_rate`).
- **"Desktop" placeholder hidden under apps** — the top-left strip shows
  the focused window title in its place; "Desktop" only appears on a truly
  empty desktop now.
- **Every-minute memory janitor** — see *Long-running sessions* below:
  `malloc_trim(0)` each minute keeps long pill sessions at the memory floor.

## Long-running sessions: 0 CPU · minimal memory

The bar spends nearly all its life as a resting pill, so the hard rules are:

- **0 CPU at rest.** Every poll is gated by mode (stats / dash_status / audio
  only run while their panel is on screen), every redraw is change-driven
  (no frame loop — timers fire, compare, and sleep), and dbus daemons park
  their threads on the bus fd. A resting pill wakes the CPU only when
  something actually changed.
- **Memory must not ratchet.** Morph churn (scene `Vec`s, text shaping,
  image uploads) frees thousands of small allocations that glibc then hoards
  in its arenas — RSS climbs forever in long sessions.
- **Every-minute janitor.** The 60 s power timer calls `malloc_trim(0)`
  (`App::memory_janitor`), handing all free arena pages back to the kernel,
  so a session that lives on the pill for days stays at the memory floor.
  One call per minute = zero measurable CPU; musl builds are unaffected
  (it returns memory eagerly already).
- **No leaks by construction.** Histories are capped (net 40 / cpu+gpu 48
  samples), notification history truncates to `[notifications] max_history`,
  the tray list to 8 chips, image keys are deduplicated, and the edit-mode
  glide map is cleared whenever editing ends.

## Open notes

- for long running session pill freezes and doesnot respond (sometimes)…
- pill top/bottom gaps should stay equal and the resting margin Hyprland
  grants must not jump around (keep it fixed once occupied).
