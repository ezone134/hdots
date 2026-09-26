# zen-shell — Full Project Guide

> A complete guide to understand the architecture, pipelines, and how to manually
> edit every part of this Rust Wayland shell bar.

---

## 1. What This Project Is

A **zero-overhead Wayland shell bar** written in pure Rust. Think of it as a
desktop bar (like Waybar) but built from scratch with:
- A floating **pill** that morphs into a **dashboard** on hover
- A metro-tile grid dashboard (Windows 10 Start style)
- Panels: Settings, Launcher, Clipboard, Themes, Power Menu, Wi-Fi, Bluetooth, etc.
- 0% CPU when idle — event-driven, no free-running render loop

---

## 2. Project Structure

```
zen-shell/
├── Cargo.toml                    ← dependencies (smithay, glow, ash, cosmic-text, zbus...)
├── config/
│   └── shell.toml                ← USER CONFIG (fonts, colors, bar size, backends)
├── scripts/                      ← bash helper scripts (dash_status, theme_main, etc.)
├── docs/
│   └── ipc.md                    ← full IPC command reference
├── src/
│   ├── main.rs                   ← ENTRY POINT
│   ├── config.rs                 ← Config struct + TOML parsing
│   ├── vars.rs                   ← centralized session paths ($states, $states2)
│   ├── shell/                    ← STATE MACHINE (pure logic, no wayland/GL)
│   │   ├── mod.rs                ← Shell struct, Mode enum, Cmd enum, layout
│   │   └── panels/               ← panel-specific layout drawers
│   ├── app/                      ← WAYLAND GLUE (connects shell to renderer + input)
│   │   ├── mod.rs                ← App struct, event loop, renderer init
│   │   ├── input.rs              ← keyboard/pointer → shell.click() / motion()
│   │   ├── handlers.rs           ← Wayland protocol callbacks
│   │   ├── ipc_srv.rs            ← IPC server (receives "zen-shell open ..." commands)
│   │   ├── session.rs            ← lockscreen + PAM auth
│   │   └── surfaces.rs           ← EGL/Vulkan surface management
│   ├── render.rs                 ← OpenGL renderer (SDF rounded rects + text)
│   ├── render_vk.rs              ← Vulkan renderer (same Cmd dispatch; `vk` feature)
│   ├── text.rs                   ← cosmic-text → premultiplied RGBA glyph textures
│   ├── ui.rs                     ← COMPONENT LIBRARY (frame, card, chip, tile, slider...)
│   ├── backends.rs               ← pluggable system backends (sysfs, wpctl, etc.)
│   ├── clipboard.rs              ← wlr-data-control clipboard manager
│   ├── tray.rs                   ← StatusNotifierWatcher (system tray icons)
│   ├── apps.rs                   ← desktop app list + icon lookup
│   ├── stats.rs                  ← /proc stats (CPU, RAM, disk, GPU, net)
│   ├── weather.rs                ← Open-Meteo fetch (background thread)
│   ├── hypr.rs                   ← Hyprland IPC (cursorpos, clients, layers)
│   ├── ipc.rs                    ← IPC client (zen-shell open ... talks to running bar)
│   ├── img.rs                    ← image store (icons, tray, clipboard thumbnails)
│   ├── themes.rs                 ← theme card parsing from $states2/themes JSON
│   ├── auth.rs                   ← PAM authentication wrapper
│   └── bin/
│       └── font_diag.rs          ← font/glyph diagnostic tool
└── README.md                     ← comprehensive project docs
```

---

## 3. Why `mod.rs` Instead of `mode.rs`?

**This is NOT a file named "mod.rs" by accident.** It's a Rust language convention.

In Rust, every directory that acts as a module has a file called `mod.rs`:

```
src/
├── shell/
│   ├── mod.rs        ← this IS the shell module (Shell struct, Mode, Cmd)
│   └── panels/       ← child module
│       └── mod.rs    ← panels module
├── app/
│   ├── mod.rs        ← app module (App struct, Wayland handlers)
│   ├── input.rs
│   ├── handlers.rs
│   └── ...
```

The file is named `mod.rs` because that's how Rust knows a directory is a module.

**It's NOT named `mode.rs` because it does much more than just the Mode enum.**

The `shell/mod.rs` file contains:
- `Mode` enum (Collapsed, Expanded, Launcher, Settings, etc.)
- `Shell` struct (ALL UI state — 200+ fields)
- `Cmd` enum (draw primitives: Rect, Text, Image, Line, Outline)
- `Anim` struct (morph animation state)
- `CardLayout` struct (dashboard grid placement)
- `pack_cards_max()` (flow packer algorithm)
- `default_dash_layout()` (card positions)
- `DashCard` enum (System, Weather, Media, Network, etc.)
- `PillItem` enum (Workspaces, Clock, Battery, etc.)
- Color constants (Catppuccin Mocha palette)
- Helper functions (ease_out_cubic, hostname, whoami, uptime_str, etc.)

Naming it `mode.rs` would be misleading — it's the **entire shell state machine**.

**The Rust convention:**
- `src/foo.rs` → module `foo` (single file)
- `src/foo/mod.rs` → module `foo` (directory with children)
- Children: `src/foo/bar.rs` → `foo::bar` (sub-module)

---

## 4. The Three Core Pipelines

### Pipeline 1: Config → Shell → Render

This is the main data flow from file to screen:

```
config/shell.toml (TOML file on disk)
    │
    ↓  Config::load(path) in config.rs
    │
Config struct (parsed, with defaults)
    │
    ↓  Shell::new(cfg) in shell/mod.rs
    │
Shell struct (ALL UI state lives here)
    │
    ↓  shell.layout_collapsed() or shell.layout_expanded()
    │
Vec<Cmd> (scene graph: draw primitives)
    │
    ↓  app.rs dispatches each Cmd to renderer
    │
render.rs (OpenGL) or render_vk.rs (Vulkan, `--features vk`)
    │
    ↓
Wayland surface → your screen
```

**What this means for editing:**
- Change fonts/colors/size → edit `config/shell.toml`
- Change what's shown on the pill → edit `shell/mod.rs` (`PillItem`, `pill_order`)
- Change how panels look → edit `shell/panels/` (layout drawers)
- Change rendering → edit `render.rs` or `render_vk.rs`

### Pipeline 2: Live Colors (Bash Scripts → Rust Shell)

Colors come from external bash scripts, NOT from shell.toml at runtime:

```
theme_main (bash script in scripts/)
    │
    ↓  writes JSON
    │
$states2/shell_vars.json ← live color palette (fg, fg2, fg3, bg, bg2, bg3, acc, sfg)
$states2/shell_vars     ← legacy key=#rrggbb fallback
$states2/m_dummy     ← "d" (dark) or "l" (light)
$states2/s        ← "n" / "d" / "l"
    │
    ↓  on boot OR `zen-shell reload colors`
    │
Shell fields: sv_fg, sv_bg, sv_acc, sv_hover, sv_sfg...
    │
    ↓  shell uses these INSTEAD of config [colors]
    │
Cmd scene uses live palette → renderer
```

**What this means for editing:**
- `$states2/shell_vars.json` is the **live source of truth** for colors
- Config `[colors]` is only the **fallback** when the palette file is missing
- To change colors live: edit `$states2/shell_vars.json` or run `zen-shell reload colors`
- The bash pipeline (theme_main, theme_body) is **complex** — the README warns: "Do NOT edit theme_body"

### Pipeline 3: External Commands → Shell

IPC commands from keybinds or other tools:

```
zen-shell open launcher
    │
    ↓  ipc.rs client connects to unix socket
    │
app/ipc_srv.rs (server handler)
    │
    ↓  shell.set_mode(Mode::Launcher) or similar
    │
Shell mode changes → next render cycle
    │
    ↓  shell.layout_expanded() generates Cmd scene
    │
Renderer draws the panel
```

**What this means for editing:**
- Add a new panel: add Mode variant → add layout → add IPC handler
- The IPC socket is at `$XDG_RUNTIME_DIR/zen-shell.sock`

---

## 5. Key Files to Edit

### Easy Edits (no Rust knowledge needed):

| What | File | How |
|------|------|-----|
| Fonts | `config/shell.toml` | `[fonts] family = "..."` |
| Colors (boot fallback) | `config/shell.toml` | `[colors] fg = "#rrggbb"` |
| Bar height | `config/shell.toml` | `[bar] height = 38` |
| Pill width | `shell/mod.rs` | content-sized (**DON'T** set `[bar] collapsed_width`; see note below) |
| Dashboard width | `config/shell.toml` | `[bar] expanded_width = 1120` |
| Morph speed | `config/shell.toml` | `[animation] duration_ms = 240` |
| Pill items | `config/shell.toml` | `[bar] pill_order = ["clock", "battery"]` |
| Weather location | `config/shell.toml` | `[weather] latitude/longitude` |
| Wallpaper dir | `config/shell.toml` | `[wallpaper] dir = "~/Wallpapers"` |
| Backends | `config/shell.toml` | `[backends] power = "sysfs"` |
| Notifications | `config/shell.toml` | `[notifications] position = "top-right"` |
| Dashboard layout | `$states/shell_{d,l,n}` | `[[dash_cards]] card="system" x=0 y=0 w=6 h=4` |

### Medium Edits (some Rust):

| What | File | What to change |
|------|------|----------------|
| Add a pill item | `shell/mod.rs` | Add PillItem variant + DEFAULT_PILL_ORDER |
| Change dashboard grid size | `shell/mod.rs` | DASH_COLS, DASH_ROWS constants |
| Change card sizes | `shell/mod.rs` | CARD_SIZES array |
| Add a new panel mode | `shell/mod.rs` | Mode::size() match arm |
| Change panel layout | `shell/panels/*.rs` | The layout drawer functions |
| Change component look | `ui.rs` | frame(), card(), chip(), slider(), etc. |
| Change morph easing | `shell/mod.rs` | ease_out_cubic() function |

### Hard Edits (advanced Rust):

| What | File | What to change |
|------|------|----------------|
| New Wayland feature | `app/handlers.rs` | Wayland trait impls |
| New backend | `backends.rs` | Backend trait + impl |
| New IPC command | `app/ipc_srv.rs` | IPC handler match arm |
| Change GL rendering | `render.rs` | Gl struct methods |
| Add new Cmd primitive | `shell/mod.rs` + `render.rs` | Cmd enum + render dispatch |
| Change text rendering | `text.rs` | TextEngine methods |

> **Pill width is content-sized — DO NOT hand-set it.** The resting (collapsed)
> pill is sized to exactly what it draws: `Shell::collapsed_w()`
> (`shell/mod.rs`) sums each shown module (clock/date, battery %, workspace
> dots, brightness chip, settings/bell/tray, …) plus `PILL_GAP`s, and the drawer
> (`layout_collapsed`) draws to that same width, so the surface always matches
> its contents. `[bar] collapsed_width = 165` in `config/shell.toml` is a legacy
> fallback only and has no effect on the resting pill.
>
> **Two invariants keep the pill from booting "broken" (clipped until the
> first hover expand⇄collapse) — do not regress:**
> 1. In `App::new` (`app/mod.rs`) build `TextEngine` + `Shell::new`/`update_clock`
>    **before** creating the layer surface + renderer, and size the initial
>    `layer.set_size(cw, ch)` to `cw = shell.collapsed_w().ceil()` (not the config
>    constant). Sync `shell.cur_w`/`cur_h`/`last_committed` to `cw/ch` right
>    after.
> 2. In `start()` after `shell.seed_data()` + `seed_boot_data()` (which fill in
>    battery/AC, growing the measured width), call `shell.refresh_size()` and
>    `ensure_morph_timer()` so the pill auto-morphs to the final width instead of
>    staying clipped until the user hovers.
>
> If you ever replace the size-estimator or the morph handshake, keep both.

---

## 15b. Bash Script Style Guide (scripts/)

The helper scripts under `scripts/` are user-facing: they are sourced by the
Hyprland session (`audio_body`, `brightness_body`, …) and are also callable
directly from a terminal or a keybind. Write them so a user can read a script
top-to-bottom and understand what it does in a few seconds.

### 15b.1 Avoid external binaries when a builtin will do

Prefer bash builtins and parameter expansion over `cat`, `grep`, `tr`, `sed`,
`awk`, `head`, `cut`, etc. Builtins are faster (no fork), give clearer errors,
and keep the script self-contained.

| ❌ avoid (forks an external binary) | ✅ prefer (builtin / parameter expansion) |
|---|---|
| `cat file` | `$(<file)` |
| `echo "$var"` | `printf '%s\n' "$var"` (or just use the variable) |
| `grep -q 'pat' <<<"$str"` | `case "$str" in *pat*) … ;; esac` |
| `grep -o '[0-9]\+' <<<"$str"` | parameter expansion to extract the number |
| `tr -dc '0-9' <<<"$str"` | `${str//[!0-9]/}` |
| `cut -d' ' -f2 <<<"$str"` | `${str#* }` / `${str%% *}` |
| `head -1 <<<"$str"` | `${str%%$'\n'*}` |

**Exception:** `wpctl`, `pactl`, `hyprctl`, `swww`, `bluetoothctl`, `nmcli`,
`pam`/`login`, etc. are the actual system interfaces — those must stay. The rule
is about *text munging* binaries, not about the system CLIs the scripts wrap.

### 15b.2 Prefer `(())` over `[[ ]]` for arithmetic

When comparing or computing integers, `(( ))` is shorter and reads like math:

```sh
# preferred — looks like arithmetic
if (( m > max )); then m=$max; fi
if (( n < 0 )); then n=0; fi
(( count += 1 ))

# acceptable when you need a string test or pattern match
if [[ "$str" == *pattern* ]]; then …; fi
```

Use `[[ ]]` only when you need its extras: `&&`/`||` inside the test, `==` with
glob patterns, regex `=~`, or testing empty/unset strings with `-z`/`-n`.
For plain "is this number bigger than that one" use `(( ))`.

### 15b.3 Structure for readability

Write scripts as **use-case → if/else → function call**, not as a chain of
pipes on one line. A reader should be able to scan the top-level command handler
and immediately see the cases.

```sh
#!/usr/bin/env bash
# my_script — one-line description
set -u

# ---- helpers (private) ----
num() { … }   # strip non-digits from a string

# ---- command dispatch (public) ----
case "${1:-get}" in
    get)   my_get_cmd "$@" ;;
    set)   my_set_cmd "${2:-50}" ;;
    up)    my_step + "${2:-5}" ;;
    down)  my_step - "${2:-5}" ;;
    *)     echo "my_script: use get|set|up|down" >&2; exit 1 ;;
esac
```

Inside each function, prefer explicit `if/else` over `cmd && cmd || cmd` chains
when the logic matters — the short-circuit form is fine for trivial guards but
gets hard to follow once there are three branches.

### 15b.4 State files, not subprocess output

Volume/mic/state values persist in `$states2/` files (`vol`, `vol_f`,
`vol_muted`, `mic`, …). Scripts **read the file**, not the device, for the
current value — the device is only queried when the state file is missing
(`vol_ensure`). This keeps keybinds and the dashboard poll cheap (no wpctl
subprocess on every read) and makes the values stable across rapid steps.

### 15b.5 Readable number formatting

When you need an integer hundredths value (`40` → `"0.40"`) for wpctl, write a
small named function instead of inlining the arithmetic:

```sh
vol_fraction() { printf "%d.%02d\n" $((${1:-0} / 100)) $((${1:-0} % 100)); }
```

Avoid `10#` hex-octal guards unless the input genuinely could be misread as octal
— when values come from `$states2/vol` (always decimal digits), plain `$(( ))`
is fine and reads cleaner.

---

## 6. The Shell Struct — The Heart of Everything

`shell/mod.rs` contains the `Shell` struct with ~200 fields. Here are the key groups:

### Mode & Morph
```rust
pub mode: Mode,           // current: Collapsed, Expanded, Launcher, Settings...
pub prev_mode: Mode,      // for animation back
pub cur_w: f32,           // current surface width (animated)
pub cur_h: f32,           // current surface height (animated)
pub anim: Option<Anim>,   // morph animation state
```

### Live State (from services/scripts)
```rust
pub battery: i32,         // battery %
pub volume: f32,          // master volume 0..1
pub brightness: f32,      // backlight 0..1
pub wifi_on: bool,        // Wi-Fi radio
pub bt_on: bool,          // Bluetooth radio
pub cpu: i32,             // CPU %
pub mem_pct: i32,         // RAM %
pub media_title: String,  // now playing
pub weather: Option<...>, // weather data
```

### UI Toggles (from Settings panel)
```rust
pub expand_on_hover: bool,    // hover pill → dashboard
pub bar_floating: bool,       // floating vs flush to edge
pub bar_reserve: bool,        // reserve screen space
pub pill_clock: bool,         // show clock on pill
pub pill_battery: bool,       // show battery on pill
pub pill_tray: bool,          // show tray on pill
pub pill_order: Vec<PillItem>,// left→right pill layout
```

### Dashboard Grid
```rust
pub dash_layout: Vec<CardLayout>,   // card positions (x,y,w,h in cells)
pub dash_edit: bool,                // edit mode active
pub packed_layout: Vec<CardLayout>, // packed (rendered) positions
pub tray_cards: Vec<DashCard>,      // parked cards in edit tray
```

### Pending Actions (set by shell, consumed by app.rs)
```rust
pub pending_volume: Option<f32>,     // app.rs applies to wpctl
pub pending_brightness: Option<f32>, // app.rs writes to sysfs
pub pending_wallpaper: Option<String>,// app.rs spawns set command
pub pending_theme_apply: Option<String>,// app.rs runs theme_main
pub pending_wifi: Option<String>,    // app.rs runs nmcli
pub pending_dash: Option<DashCmd>,   // app.rs runs dashboard scripts
```

---

## 7. The Cmd Scene Graph

Everything drawn is a `Cmd` variant:

```rust
pub enum Cmd {
    // Filled rounded rectangle
    Rect { x, y, w, h, r, color },
    
    // Rounded rect with per-corner radii (concave = negative)
    RectConcave { x, y, w, h, r_tl, r_tr, r_br, r_bl, color },
    
    // Outline-only rounded rect (battery body, borders)
    Outline { x, y, w, h, r, width, color },
    
    // Thick line segment (heartbeat graphs, sparklines)
    Line { x0, y0, x1, y1, w, color },
    
    // Text (with font size, color, alignment, icon flag)
    Text { x, y, right, center, text, size, color, icon },
    
    // Rasterized image (app icon, tray pixmap)
    Image { x, y, size, key },
}
```

The layout functions in `shell/mod.rs` and `shell/panels/` build a `Vec<Cmd>` that
the renderer draws. To change what something looks like, you change the Cmd
primitives generated by its layout function.

---

## 8. The UI Component Library (ui.rs)

`ui.rs` provides reusable drawing helpers that emit Cmds:

```rust
ui::frame(x, y, w, h, r, bg)     // rounded rect background
ui::card(x, y, w, h, bg)          // dashboard card (frame + shadow)
ui::chip(x, y, text, bg, fg)      // small pill/chip (clock, battery, etc.)
ui::tile(x, y, w, h, text, icon)  // dashboard tile (toggle buttons)
ui::slider(x, y, h, value)        // vertical iOS-style slider
ui::bar(x, y, w, pct, color)      // horizontal progress bar
ui::media_btn(x, y, glyph)        // media transport button
ui::text(x, y, text, size, color) // text at position
ui::battery(x, y, pct, charging)  // tide-island battery icon
ui::pulse(x, y, w, h, data, color)// heartbeat sparkline
```

Changing `ui::chip()` changes EVERY chip in the entire UI — one edit, global effect.

---

## 9. How the Event Loop Works

```
main()
  │
  ├→ ipc::try_client(&args)  → if args present, talk to running bar
  │
  ├→ claim_single_instance() → flock prevents duplicate bars
  │
  ├→ Config::load() → Shell::new(cfg)
  │
  └→ App::run(cfg)
       │
       ├→ Connect to Wayland
       ├→ Create EGL surface (+ Vulkan if `vk` feature and ZEN_VULKAN=1)
       ├→ Create shell surface (layer-shell)
       ├→ Register calloop timers (1s clock, 3s stats, 60s memory janitor)
       ├→ Register IPC socket on calloop
       ├→ Register dbus services (notifications, tray)
       │
       └→ Event loop (calloop)
            │
            ├→ Wayland event → handler.rs → shell.* → mark dirty
            ├→ Timer tick → shell.update_*() → mark dirty
            ├→ IPC message → ipc_srv.rs → shell.set_mode() → mark dirty
            ├→ Dirty? → shell.layout_*() → Vec<Cmd> → renderer.draw()
            └→ Sleep until next event (0% CPU when idle)
```

---

## 10. Session State Dirs

Two directories are set by Hyprland for the bar's entire life:

### `$states` (persisted — syncs back at session end)
- `$states/themes` — master theme file (TAB-separated, one per theme)
- `$states/shell_{n,d,l}` — per-state shell config (the MASTER config)
- `$states/acc_source_{n,d,l}` — accent source flags

### `$states2` (volatile — reset each session)
- `$states2/shell_vars.json` — live color palette (JSON; `$states2/shell_vars`
  `key=#rrggbb` lines are the legacy fallback)
- `$states2/m_dummy` — dark/light marker ("d" or "l")
- `$states2/s` — current channel ("n"/"d"/"l")
- `$states2/themes` — JSON mirror of master themes
- `$states2/custom_acc.json` — custom accent list
- `$states2/cur_theme` — active theme id

**NEVER verify these directories exist — they are always present.**
**NEVER create them — Hyprland sets them up.**

---

## 11. Building & Running

```bash
# From workbench root (/home/tw/my_workspace):
cargo build -p zen-shell          # debug build (ALWAYS do this by default)
cargo run -p zen-shell            # start the bar (needs Hyprland)

# From inside zen-shell/:
cargo build                       # Cargo walks up to workspace root
cargo test -p zen-shell           # run tests

# Live test:
setsid nohup env ZEN_TRACE=1 .../debug/zen-shell > /tmp/zen.log 2>&1 < /dev/null &

# Kill:
pkill -x zen-shell
```

Binary location: `/home/tw/my_workspace/target/debug/zen-shell`

---

## 12. Common Edits Cookbook

### Change pill background color:
Edit `$states2/shell_vars.json` or `config/shell.toml`:
```toml
[colors]
bg = "#1e1e2e"    # Catppuccin Mocha base
```

### Change pill to flush (not floating):
```toml
[bar]
floating = false
```

### Change bar position (top instead of bottom):
```toml
[bar]
anchor = "top"
```

### Disable hover-to-expand:
```toml
[bar]
expand_on_hover = false
```

### Change dashboard card layout:
Edit `$states/shell_d` (or shell_l):
```toml
[[dash_cards]]
card = "system"
x = 0
y = 0
w = 6
h = 4

[[dash_cards]]
card = "weather"
x = 6
y = 0
w = 4
h = 4

[[dash_cards]]
card = "media"
x = 10
y = 0
w = 10
h = 4
```

### Hide battery on pill:
```toml
[bar]
show_battery = false
```

### Change weather location:
```toml
[weather]
latitude = 28.6139
longitude = 77.2090
city = "New Delhi"
```

### Change wallpaper directory:
```toml
[wallpaper]
dir = "~/Pictures/Wallpapers"
command = "swww img {path}"
```

### Change morph speed:
```toml
[animation]
duration_ms = 150    # faster (default 240)
```

---

## 13. Key Architecture Decisions

1. **Shell is pure logic** — no wayland, no GL, no I/O. Just state + layout functions.
   This makes it testable and portable.

2. **App is the glue** — connects wayland events to shell, shell output to renderer.
   This is where all the platform-specific stuff lives.

3. **Cmd is the scene graph** — everything drawn is a Cmd variant. The renderer
   doesn't know what it's drawing (rects, text, images) — it just renders Cmds.

4. **Config is per-channel** — dark/light/night each have their own shell config
   in `$states/shell_{d,l,n}`. The active channel determines which config is loaded.

5. **Colors are live** — the bar reads colors from `$states2/shell_vars.json`
   (JSON, falls back to the legacy `$states2/shell_vars` `key=#rrggbb` lines) at
   runtime, NOT from shell.toml. Config colors are only fallbacks.

6. **Event-driven, never polling** — calloop sleeps on the wayland fd. Timers are
   inserted only while needed (morph animation, stats polling while dashboard open).

7. **Single instance** — flock prevents two bars from running (they'd fight for
   pointer focus and get stuck).

8. **`colors reload` is a palette-only hot swap (waybar-style)** — it re-reads
   `$states2/shell_vars.json` + accent flags and redraws. It must NEVER re-read
   the channel config, re-apply bar geometry, touch the dashboard layout, or
   start a morph. Full channel/config/geometry adoption lives ONLY in
   `zen-shell reload` (`ipc::Cmd::ConfigReload` → `reload_config`). A routine
   palette flip must never disturb a live session.

9. **An IPC command can NEVER kill the shell** — every handler runs under a
   `catch_unwind` panic net (`App::run_ipc` in `ipc_srv.rs`): a panicking
   handler is logged, replies `error: internal`, and the bar keeps running.
   The renderer is re-owned even if a draw panics, so one bad frame can't
   strand the surface. If the shell keeps breaking on a reload, the answer
   is NEVER "accept a restart" — fix the control flow so it can't fault.

---

## 14. Glossary

| Term | Meaning |
|------|---------|
| **Pill** | The collapsed bar — a small floating capsule |
| **Dashboard** | The expanded metro-tile grid (hover pill → dashboard) |
| **Morph** | The animation between pill and dashboard (eased size commit) |
| **Mode** | Current UI state (Collapsed, Expanded, Launcher, Settings, etc.) |
| **Cmd** | A draw primitive in the scene graph (Rect, Text, Image, Line) |
| **Shell** | The pure state machine — all UI logic, emits Cmds |
| **App** | The wayland glue — event loop, renderer, IPC, input handling |
| **calloop** | The event loop library (like epoll, but Rust-native) |
| **layer-shell** | Wayland protocol for desktop bars/overlays |
| **SDF** | Signed Distance Field — technique for smooth rounded rects |
| **PipeWire** | Audio server (replaces PulseAudio) |
| **wpctl** | PipeWire's CLI control tool |
| **StatusNotifier** | KDE system tray protocol |
| **wlr-data-control** | wlroots clipboard access protocol |
| **ext-session-lock** | Wayland protocol for screen lockers |
| **cosmic-text** | Pure Rust text shaping + layout library |
| **glow** | Safe OpenGL wrapper for Rust |
| **ash** | Safe Vulkan wrapper for Rust |
| **zbus** | Pure Rust D-Bus library |
| **Catppuccin Mocha** | The color scheme (dark pastels) |

---

## 15. Where Things Are Defined (Quick Reference)

| I want to... | Edit this file |
|--------------|----------------|
| Change bar size/position | `config/shell.toml` → `[bar]` |
| Change colors | `$states2/shell_vars.json` (live) or `config/shell.toml` → `[colors]` |
| Change pill items | `config/shell.toml` → `[bar] pill_order` |
| Change dashboard cards | `$states/shell_{d,l,n}` → `[[dash_cards]]` |
| Change panel size | `shell/mod.rs` → `Mode::size()` |
| Change pill layout | `shell/mod.rs` → `layout_collapsed()` |
| Change dashboard layout | `shell/mod.rs` + `shell/panels/` |
| Change component look | `ui.rs` → `frame()`, `card()`, `chip()`, etc. |
| Change morph animation | `shell/mod.rs` → `ease_out_cubic()`, `Anim` struct |
| Add a new panel | `shell/mod.rs` (Mode) + `shell/panels/` + `app/ipc_srv.rs` |
| Change rendering | `render.rs` (GL) or `render_vk.rs` (Vulkan) |
| Change text rendering | `text.rs` |
| Change system backends | `backends.rs` |
| Add IPC command | `app/ipc_srv.rs` |
| Change weather | `config/shell.toml` → `[weather]` or `weather.rs` |
| Change wallpaper | `config/shell.toml` → `[wallpaper]` |
| Change tray behavior | `tray.rs` |
| Change notifications | `config/shell.toml` → `[notifications]` or `app/session.rs` |

---

*Generated by Buffy (Codebuff) — your coding assistant.*
