# zen-shell

Zero-overhead Wayland shell bar. **UI declared entirely in Lua; Rust is only the
backend + renderer.** GL-first, Vulkan opt-in. Same design as the C++ raze-shell:
a floating pill that morphs into a dashboard on hover — Catppuccin Mocha,
JetBrainsMono Nerd Font.

## Architecture — declarative in Lua, like Quickshell in QML

The shell binary is Rust and hosts a LuaJIT VM (`mlua`). Every card and surface
is a `.lua` module under `~/.config/zen-shell/ui/` that declares its objects,
bindings, models and delegates; the engine walks that Lua item tree and turns it
into one `Cmd` list. **Nothing decides a pixel in Rust** — the backend only turns
bytes into values (sysfs/procfs/netlink/D-Bus, IPC, image/FFT decode), and the
renderer turns the `Cmd` list into Wayland buffers. There is one way to author
UI: Lua. See [`QUICKSHELL_MODEL.md`](QUICKSHELL_MODEL.md).

## Philosophy

Rules for idle cost and frame discipline:

- **0 CPU**: epoll sleeps on the wayland fd when idle. No free-running
  render loop, ever.
- **0 wasted frames**: draw only on configure acks / explicit dirt. Size
  changes are committed *alone* (never attach a buffer in the same commit as
  a size change — the compositor ignores it), then the ack triggers the draw.
- **Event-driven**: everything is a wayland event, a 1s clock timer, or the
  morph / hold timers (inserted only while animating, removed on completion).
- Flush the wayland socket immediately after queueing requests.

## Design

**Minimal design** — flat, quiet, one primary accent, nothing that doesn't need
to be there; neither Material nor decorative UI. Every pixel is deliberately
placed.

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

Crate choices by layer:

| piece         | crate                                                                       |
| ------------- | --------------------------------------------------------------------------- |
| wayland       | `smithay-client-toolkit` 0.21 + calloop                                     |
| egl surface   | `wayland-egl` + raw EGL                                                     |
| GL            | `glow` (GLES3, desktop-GL 3.3 opt-in)                                       |
| Vulkan        | `ash` + `naga` (opt-in `vk` cargo feature; auto VK-hw → GL-hw → software)   |
| text          | `cosmic-text` (pure Rust, fontconfig)                                       |
| dbus          | `zbus` — Notifications + StatusNotifierWatcher                              |
| icons         | `png` + `resvg` (SVG) + `image` (JPEG wallpaper thumbs)                     |
| clipboard     | `wayland-protocols-wlr` (wlr-data-control)                                  |

## Build & run

> **Use a debug build by default.** Always compile with `cargo build` (the
> `dev`/debug profile) — not `--release` — unless explicitly told otherwise.
> Debug builds are what we iterate on here. Use `--release` only when I
> specifically ask for a release build.

```sh
# From the workbench root (/home/tw/my_workspace):
cargo build -p zen-shell            # debug build (default — do this unless told otherwise)
cargo run -p zen-shell              # debug run (needs a running wayland compositor)
cargo test -p zen-shell             # run tests
# cargo run -p zen-shell --release  # only when a release build is explicitly requested

# From inside this directory (zen-shell/), bare `cargo build` works too —
# Cargo walks up to the workspace root automatically.
```

**Vulkan backend:** `render_vk.rs` is compiled+linked only with
`cargo build --features vk` (ash + naga are optional deps). With that feature
the shell auto-selects the strongest renderer: **Vulkan on a hardware GPU
first, then GL (hardware), software only as a last resort** — an llvmpipe
(CPU) device is never picked ahead of a real GPU regardless of enumeration
order. `ZEN_RENDERER=gl` forces GL, `ZEN_RENDERER=vk` (or legacy `ZEN_VULKAN=1`)
forces Vulkan (which may then land on software). Default builds are GL-only
and carry no Vulkan code at all.

**Artifacts go to `/home/tw/my_workspace/target`** (the workbench's shared
target-dir, set in `/home/tw/my_workspace/.cargo/config.toml`) — `/tmp` and `~`
are RAM-backed tmpfs, and `my_workspace` resolves to `/acc/data` (nvme disk).
The debug binary lands at `/home/tw/my_workspace/target/debug/zen-shell`.

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

- **Battery icon** — `ui::battery`, ported from Quickshell's `BatteryIcon.qml`
  reference (`~/.config/quickshell/components/`): a translucent rounded body
  (~56%), a solid level fill growing from the left that fully rounds off at
  ≥85%, the **level number drawn inside** the silhouette (glyph shade
  auto-flips dark/light from fill luminance), number **+ bolt** while charging,
  and a terminal tip that lights solid at 100%. Red ≤20% via the existing
  caller logic. The collapsed pill renders it at 13px with no external "NN%"
  text; the dashboard card shows only the time estimate beside it.
  (`src/ui.rs`, `src/shell/mod.rs`, `src/shell/panels.rs`)

## Wallpaper

The wallpaper backend defaults to `command` with `set_wall {path}` — an
external script that receives the absolute path. The in-process daemon
backend (`[backends] wallpaper = "daemon"`) is disabled by default. The
wallpaper picker panel is 620×540 with a **4×5 thumbnail grid** (20 items
visible, scrollable), using `thumb:<basename>` cached thumbnails.

## Colors

**Live palette — `$states2/shell_vars.json`.** The shell reads every
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
  the **per-state shell config** `$states/shell_{n|d|l}` (the master config;
  see below) and the **accent source flags** `$states/acc_source_{n,d,l}`.
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
  - `$states2/custom_acc.json` — custom accent list (`[{name,d,l}]`).
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
- Settings → Pill exposes them as **Margin** (key 36, 0–30 px) and
  **Transparency** (key 37, 0–255 alpha), which set `pill_margin_x` /
  `pill_alpha` respectively.
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

How `cosmic-text` output reaches the screen:

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

Three bugs in that dance were fixed on 2026-08-15:

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
the anchor — cursor over it → expand / stay expanded; cursor gone → collapse.
This is why the dashboard never collapses mid-morph under the cursor, and
why one that ends up away from the cursor collapses once the pointer never
comes over. (Panels are modal — they don't reconcile with the cursor at all.)

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
- **Dismissal disarms hover until the pointer moves.** Launching an app,
  picking a clipboard row, Esc, an IPC click-away close — after any of them a
  cursor merely resting over the pill strip does not pop the dashboard back
  open; one real mouse movement re-arms hover expansion.
- **Live drags never collapse the dashboard.** While a drag is live
  (`pointer_down` + `drag_key`) the hover rule never collapses; a 2.5 s
  post-release grace window covers the backend scripts (`shader_main`,
  `hs_main`) that restart backends and fire `notify-send`. A genuine
  move-away still closes as usual.

Clicking a tray chip, a media button, or a widget control keeps the current
mode. A panel's first frame renders at the current size: it used to render at
the stale pill size, producing a negative-width tile that panicked the
rounded-rect clamp — the radius clamp is now degenerate-safe and the CC tile
width is clamped.

**Shape language** — the pill and the toast are capsules (radius = h/2); every
widget (dashboard, control center, launcher, notifications, calendar,
settings, power, audio, wallpaper, weather) is square-ish with a 20px corner
radius. The panel layouts draw through the shared component library in
`src/ui.rs` (`frame`, `card`, `chip`, `tile`, `slider`, `bar`, `media_btn`,
`text`), so a radius or shade change in one place restyles every panel — the
same idea as Quickshell's QML component library.

## IPC (Quickshell `qs ipc call` parity)

The bar listens on `$XDG_RUNTIME_DIR/zen-shell.sock`; the same binary,
invoked with an argument, is the client — the shell's answer to opening a
panel from a Hyprland keybind or any external tool:

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
zen-shell ipc call open launcher   # Quickshell-flavored forms work too
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

Verified live on Hyprland: hover → eased expand to 520×256, leave → eased
collapse to the pill, 10 rapid hover/leave cycles end on the pill with zero
skipped commits, and a duplicate launch exits cleanly.

## Panels & widgets

Every panel, card and widget the shell exposes:

- **Dashboard** — a **Windows-10-Metro-style tile grid**: every card is an
  N×M span on a logical **20×12 cell lattice** inside a canvas that
  AUTO-FITS the packed content (`1×1`, `1×2`, `2×2`, `3×4`… any span),
  rendered as a seamless rounded mosaic with soft drop shadows. Top strip:
  workspace box, active-window title (the "Desktop" placeholder shows only on
  a truly empty desktop), tray + settings/power chips.
  - **Right-click enters EDIT MODE** (Win10 "customize" style): a dot
    lattice appears, **drag cards to move them**, **drag the corner grip
    (⌟) to resize spans** — others glide aside live (the moved card rides 1:1
    under the cursor, the resized corner tracks it continuously, and every
    other card eases exponentially to its re-packed slot on a 60 fps settle
    timer that stops itself when the layout is at rest), and dragging past the
    edge GROWS the canvas in real time (it trims back on drop; cell size stays
    anchored to the configured width, so auto-fit never rescales the lattice).
    Cell-snapped ghost previews show green/red validity. A **parking
    tray strip** appears at the bottom: drag a card onto it (or click its
    **✕**, top-right) to REMOVE it from the dashboard, drag a chip from the
    tray back onto the grid to re-place it. Parked chips/cards columns
    **stretch to fill the panel's full width** — no dead space on the right
    edge of wide windows. Edit-mode banner chips (on the strip and in the
    parked tray) render as **icons** (clock → clock glyph, wifi → wifi, gear →
    settings) instead of word labels, with a raised **drag handlebar** under
    each strip chip for easy grabbing. Right-click / Esc leaves
    editing; while editing the dashboard NEVER hover-collapses (only Esc /
    right-click / explicit dismissal leave it), and the lifted card gets a
    soft accent glow instead of a border. The layout — every enabled card
    with its size AND exact position (`[[dash_cards]]` entries: `card` id +
    `x`,`y`,`w`,`h` cells), plus the parked tray (`[dash_tray]`, id strings) —
    persists per channel to `$states/shell_{n,d,l}` on any pill-settings
    change and on every edit-mode commit (the legacy order+spans
    `$XDG_DATA_HOME/zen-shell/dash_layout.txt` is still written as a
    fallback); unknown/hand-mangled entries fall back to defaults per card,
    and cards stay pinned to remembered positions until the user edits again.
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
      an adjacent gap (`try_place` with `max_left=1`; `pack_cards_max()` no
      longer scans from x=0 for every placement) — it never teleports all the
      way to the far-left empty cell of its row.
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
  - The default layout tiles the lattice exactly (hole-free v3): System 6 ·
    Clock 5 · Media 9 / Network 6 · CpuGpu 6 · Gauges 8 / Toggles 7 · Todo 5 ·
    Sliders 8, with the saved-layout marker `zen-layout-v3` (pre-existing
    files fall back to the tight defaults once).
  - Card set: **System** (identity/clock/date chip → calendar/uptime/
    battery), **Weather** (glyph · temp · conditions · wind), **Media**
    (art/transport/drag-seek),
    **Network** (heartbeat ↓/↑ pulse graphs), **CPU/GPU** (two-line ECG-style
    chart; amdgpu sysfs / nvidia-smi auto-detect), **RAM/DISK bead-ring
    donuts**, **To-Do** (click ✓ / ✕, live composer field, Esc or
    keyboard-leave blurs, persisted to
    `$XDG_DATA_HOME/zen-shell/todos.txt`), **Toggles** (Wi-Fi · BT · DND ·
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
    Durations + sessions persist to `$XDG_DATA_HOME/zen-shell/pomo.txt`.
  - **Fans** (id `fans`) — live fan RPMs from hwmon (`fanN (chip)` rows,
    per-fan bars normalized against a slow peak; stopped fans dim).
  - **Workspaces** (id `workspaces`) — Hyprland workspace tiles, active
    accent-tinted; click switches workspaces (same dispatch as the pill).
  - **World clock** (id `worldclock`) — pinned cities with local times:
    sun/moon glyph per row, mono HH:MM, offset-vs-local chip; "+ add city"
    opens an inline zone.tab search; offsets refresh on a worker probe
    every 5 min (DST-safe). Persisted to `$XDG_DATA_HOME/zen-shell/zones.txt`.
  - **Water** (id `water`) — daily hydration tracker: bead-ring gauge with
    `+ glass` / `−` undo, goal adjusted by horizontal swipe, auto-resets at
    midnight (`$XDG_DATA_HOME/zen-shell/water.txt`).
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
  toggles, at runtime: **Expand on hover** (off → the pill never morphs from
  the cursor — the dashboard opens by click / IPC and **right-clicking the
  pill always opens Settings**), **Floating**
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
  click without moving toggles the setting on/off); order persists to
  `shell.toml`. A System card (CPU / RAM / disk bars + net up/down) and an
  About line close the list.
- **Settings → Appearance** — accent source, custom accents, accent toggles
  and alpha, all at runtime:
  - *Accent source* buttons: **From wallpaper**
    (`theme_main accent acc_from_wall`) and **Scheme default**
    (`theme_main accent theme_default`).
  - *Custom accents* reads `$states2/custom_acc.json` (the `[{name,d,l}]`
    list regenerated by `gen_launcher_cache`/`gen_custom_acc_files`, also
    mirrored into `$states2/`); the list is a **collapsible dropdown** — a
    row shows the current selection, clicking it opens a **scrollable
    popup** (wheel scrolls its own list); picking a name runs
    `theme_main accent custom_acc <name>`. Every selection routes through
    the `$states/acc_source_<ch>` flag + GTK hot-reload and schedules a
    colors reload, so the live palette updates without a restart.
  - *Accent toggles* section: `custom_acc_start_icon` / `_app_border` /
    `_hyprland` / `_gtk` / `_normal_app`, plus the `start_icon_tone_<ch>`
    0–900 slider.
  - *Alpha* section: `bg_alpha` / `acc_alpha` / `scrim_alpha` /
    `border_alpha` in lowercase hex, plus the `acc_scrim_<ch>` toggle.
  - Toggles and alpha route through
    `theme_main toggle_acc | start_icon_tone | alpha | acc_scrim`; an
    **Apply changes** button runs `theme_main restore`. Unlike accent
    *source* picks, these do **not** write `$states2/acc_changed` — a plain
    `theme_main restore` is enough (`acc_changed` only gates the GTK/thunar
    hot-reload). The pane scrolls to fit the new sections.
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
  cell size (**Ctrl+= / Ctrl++ grows, Ctrl+- shrinks**, numpad keys too with
  auto-repeat, clamped 56–160px and **remembered in `[wallpaper] cell`**), so
  small sizes show many rows and big ones show a few large tiles (96px
  default → 5 columns). Every image in `[wallpaper] dir` (png / jpg / jpeg /
  gif) is reachable: cells are laid out by visible slot (scrolling slides the
  window, never walks cells off-panel; max-scroll lands flush with the last
  page), wheel scrolls a row per notch, two-finger trackpad scroll follows
  the natural fingers-down direction, and the **scrollbar is
  grab-and-draggable** (thumb drag keeps its in-thumb offset; pressing the
  empty track page-jumps, then drags; a wide invisible hit strip sits beside
  the grid with hover/drag highlight). Clicking a tile sets the wallpaper via
  `set_wall {path}` (or the native daemon when
  `[backends] wallpaper = "daemon"`); thumbnails come from a 256px disk cache
  rasterized at the requested cell size.
- **Wallpaper daemon** (swww-like, no external binary) — a `Layer::Background`
  surface spanning the monitor renders the current wallpaper **cover-fit**
  with a 0.25 s fade-in; set via the picker or
  `zen-shell wallpaper set <path>` (boots to the first image in
  `[wallpaper] dir`). Re-fits on monitor
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
  wheel / two-finger scroll moves the highlighted row (the selection moves
  with it, and the visible window follows); text entries show a preview,
  images show thumbnails. On boot it also
  **seeds the history from cliphist's own databases** (`src/cliphist.rs` — a
  dependency-free read-only bbolt parser): text from
  `$XDG_CACHE_HOME/cliphist/db`, images from
  `$XDG_CACHE_HOME/cliphist_image/db` (the separate-db `cliphist -db-path`
  setup; images found inside the text db are routed to the image list too),
  so the panels show your long-running cliphist history (up to the cap) even
  before zen-shell records anything itself.
- **Corner notification popup** — `notify-send` notifications appear in their
  own **top-right layer surface** (not the OSD, not the bar): up to 3 stacked
  cards (icon, summary, body) at 470×92 (122 with action chips; grew from
  352×64) with larger icon/text and drop shadows, auto-dismissed after ~6 s;
  clicking one opens the notifications panel. One shared geometry source
  (`shell::notif_popup_size`) keeps the surface exactly as tall as its
  content (action rows don't clip). Separate
  GL surface, so it never disturbs the bar.
- **Weather detail** (click the dashboard weather line /
  `zen-shell open weather`) — big temp + condition glyph, feels-like,
  humidity, wind and precipitation from Open-Meteo (fetched alongside the
  dashboard weather in the same 10-minute poll).
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

Bash script style for `scripts/` is canonical in
[`PROJECT_GUIDE.md`](PROJECT_GUIDE.md) §6 — bash script style.

## Services (dbus daemons, one thread each)

Two daemons, one thread each:

- `notif.rs` — `org.freedesktop.Notifications` daemon (bubble popups).
- `tray.rs` — `org.kde.StatusNotifierWatcher` daemon + host. Items register
  over dbus; the daemon thread introspects (IconName / IconPixmap / Title /
  Status / ToolTip / Menu), forwards `TrayMsg` to the shell, and shell
  clicks/scrolls come back as `TrayCmd::Activate` / `Scroll`. **Never do a
  blocking zbus round-trip inside a served method handler** — handlers run on
  the zbus executor thread and deadlock. Handlers only push to a
  `std::sync::mpsc`; the daemon loop does the calls.

## Layout

The file-by-file map (`src/`, `examples/`, top-level docs) is canonical in
[`PROJECT_GUIDE.md`](PROJECT_GUIDE.md) §2 — project structure.

## Roadmap

The full, prioritized plan — multi-monitor, real PAM lockscreen auth, wallpaper
transitions, JSON-RPC IPC, plugins, distribution, fuzzing, and more — lives in
[`roadmap.md`](roadmap.md).

## History

Dated changes, one entry each (session logs folded into outcomes):

- **2026-08-16** — since `399f9e5`: stats polling gated to the dashboard at a
  3s cadence; morph no longer reflows contents; one master hover rule (cursor
  over the bar ⇔ expanded); draggable sliders; native lockscreen (fullscreen
  morph, caps-lock badge, hold-to-confirm restart / shutdown); OSD on AC /
  volume / brightness / mute; launcher scrollbar + keyboard navigation; the
  `commands` file; battery / AC / brightness from `/sys` behind the
  `[backends]` layer; per-app audio + wallpaper picker + weather detail
  panels; clipboard manager (text + image history via wlr-data-control,
  SUPER+V / SUPER+SHIFT+V); native wallpaper daemon (background layer,
  cover-fit, no external binary); notifications in their own top-right corner
  surface (separate from the OSD).
- **2026-08-24** — metro grid dashboard + edit mode landed on a 12×6 lattice
  (`DashCard` spans `x,y,w,h`, nine position-independent drawers,
  cell-snapped ghost previews, persistence to `dash_layout.txt`, slider/seek
  hit-testing from runtime geometry; `src/shell/mod.rs`, `src/shell/panels.rs`);
  `Cmd::Line` heartbeat graphs — thick capsule segments as rotated quads
  feeding the unchanged SDF shader on both GL and Vulkan backends
  (`ui::pulse`; `src/ui.rs`, `src/render.rs`, `src/render_vk.rs`,
  `src/app.rs`); wide dashboard rework from the old two-column layout to a
  1120×480 3-column mosaic (bigger identity/system card; clock + media cards
  in row 1; toggles in their own 2×3 tile card;
  `src/shell/panels.rs`, `src/stats.rs`, `scripts/dash_status`,
  `scripts/shader_main`, `scripts/hs_main`); network card as big ↓/↑ readouts
  formatted by `stats::fmt_rate` ("12.5 Mbps" / "340 Kbps"); settings
  Pill rows made drag-to-reorderable (`src/app.rs` press arm,
  `src/shell/mod.rs`); tide-island battery icon; wallpaper picker overhaul;
  notification popups grown to 470×92.
- **2026-08-25** — live colors moved to `$states2/shell_vars.json` (boot +
  `zen-shell reload colors`; `[colors]` fallback only); theme picker added
  (`zen-shell open themes`, SUPER+T — card grid, click →
  `theme_main apply <id>` with an automatic delayed palette re-read; script
  pipeline untouched); dark/light chip on the identity card; new IPC
  handlers `colors reload` and `mode sync`.
- **2026-08-27** — per-state (UI theme) config added (Settings → Pill
  Margin/Transparency; logged defaults `pill_margin_x = 8`,
  `pill_alpha = 176` — current defaults in Per-state shell config above);
  `src/vars.rs` centralizes the `$states` /
  `$states2` / channel / per-state paths (doc comments repeat: the env vars
  are set by Hyprland for the bar's whole life — never verify / create /
  bail); dashboard layout saved as an array in `$states/shell_{n,d,l}`
  (`[dash_cards]` / `[dash_tray]`); growing morphs used `ease_out_back`
  overshoot with `ease_out_cubic` on shrink (both directions are
  `ease_out_cubic` now).
- **2026-08-28** — faster startup: the bar and launcher appear instantly.
  The pre-frame reads (hyprctl ws/active-win, D-Bus ssid/media, battery/AC,
  sysfs brightness, `wpctl get-volume`) were moved out of `App::new` into
  `App::seed_boot_data()`, which runs right after the first frame is
  committed at the initial wayland roundtrip. `App::new` still builds the
  cheap backend objects (sockets/structs) but no longer blocks on the data
  reads, so rendering + input are interactive immediately; the 3s services /
  60s power timers take over from there. Settings → Appearance gained accent
  source / custom accents / accent toggles / alpha (folded into Panels &
  widgets above); `theme_accent_func`
  (`theme_main_body`) now writes the canonical `$states/acc_source_<ch>` flag
  (`w`/`c`/`h`/`d`) and runs `theme_main restore` (and `acc_changed`) only on
  an actual source change; `define_hex` with no payload reads
  `$HOME/documents/defined_hex` via `$(<file)`, no `cat` subprocess.

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

Unresolved issues:

- Long-running sessions: the pill sometimes freezes and stops responding.
- Pill top/bottom gaps should stay equal, and the resting margin Hyprland
  grants must not jump around (keep it fixed once occupied).
