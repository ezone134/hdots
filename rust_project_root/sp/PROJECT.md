# sp — Project Notes

## Overview
GPU-accelerated Wayland splash screen **and** pop window. Reads config from `$states2/sp_vars` (JSON), renders text on a solid (optionally rounded, optionally translucent) background via EGL/GL, sleeps `duration` seconds, exits. Dismisses immediately on **Esc** / **Enter** (rofi-style) or when keyboard focus is lost.

`sp` = **s**plash + **p**op window. Both callers share one binary; only the JSON differs.

## Key Files
- `src/main.rs` — single-file binary, ~1400 lines
- `Cargo.toml` — standalone crate (no workspace root; deps pinned inline)
- Callers in `~/sources/`: `splash_init` (fullscreen boot splash), `center_pop` (centered pop window, e.g. state switch)

## Config File (`$states2/sp_vars`)
Format: JSON object. Parsed with `serde_json`.

### Fields
| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `w` | f32 | 100 | Window width in percentage of output |
| `h` | f32 | 100 | Window height in percentage of output |
| `bg` | hex string | "#000000" | Background color — 6 **or** 8 digits (see below) |
| `fg` | hex string | "#ffffff" | Foreground/text color — 6 **or** 8 digits (see below) |
| `b_text` | string | "" | Big text content |
| `b_font` | string | "sans-serif" | Big text font family |
| `b_size` | f32 | 100.0 | Big text font size |
| `s_text` | string | "" | Small text content |
| `s_font` | string | "sans-serif" | Small text font family |
| `s_size` | f32 | 40.0 | Small text font size |
| `duration` | u64 | 3 | On-screen lifetime in seconds |
| `spacing` | f32 | 50.0 | Uniform pixel gap between big text, loading animation & small text |
| `loading_id` | u32 | 1 | Loading animation id (1=braille spinner, 2=braille dots, 3=quarter circles) |
| `show_loading_animation` | bool | true | Show/hide loading animation |
| `center_b_text` | u32 | 0 | 1=center whole group vertically, 0=big text at exact vertical center |
| `corner_rounding` | f32 | 0.0 | Corner rounding radius (0=no rounding, draws bg via clear) |
| `alpha` | u8 | 100 | Global window opacity, percent 0-100 |

### Color formats
`parse_hex` accepts both, `#` optional:
- `#rrggbb` → opaque (`alpha` byte = `ff`)
- `#rrggbbaa` → **CSS order**: RGB then alpha last (`#282a3680` = 50% alpha)

Alpha semantics:
- `fg` alpha bakes into the rasterized glyph pixels (text fades).
- `bg` alpha **multiplies** the `alpha` percent, so the two compose instead of fighting: effective opacity = `alpha/100 × bg_a/255`.
- 6-digit `bg` therefore behaves exactly as before (multiplier 1.0).

### Example JSON
```json
{
  "w": 100,
  "h": 100,
  "bg": "#2e3440ff",
  "fg": "#eceff4",
  "b_text": "a",
  "b_font": "distro_splash",
  "b_size": 500,
  "s_text": "Deploying chromatic aberration...",
  "s_font": "JetBrainsMonoNL Nerd Font",
  "s_size": 20,
  "duration": 3,
  "spacing": 50,
  "loading_id": 1,
  "show_loading_animation": true,
  "center_b_text": 1,
  "corner_rounding": 0,
  "alpha": 100
}
```

## Architecture
- **Config**: Read JSON from `$states2/sp_vars` via serde_json (bad JSON → defaults, warning on stderr)
- **Font loading**: Targeted — scans system font dirs for only the two configured families
- **Text rasterization**: `cosmic-text` → premultiplied RGBA8 pixels
- **Rendering**: EGL → GLES 3.0 / GL 3.3 core → `glow` → textured quads
- **Wayland**: `smithay-client-toolkit` layer shell (Overlay, fullscreen, `KeyboardInteractivity::Exclusive`, app id `sp`)
- **Keyboard**: `SeatState` bound in registry; `SeatHandler` grabs `wl_keyboard` on `Capability::Keyboard`; `KeyboardHandler::press_key` closes on `Keysym::Escape` / `Keysym::Return` / `Keysym::KP_Enter`; `leave` and a `Keyboard` capability drop also close (never left stuck up fullscreen with no focus). Seats arrive on a later registry roundtrip after layer creation — keyboard grabs once the compositor advertises the keyboard capability.
- **Animation**: Pre-rasterizes braille spinner frames, cycles at ~10fps in event loop
- **Window sizing**: `w`/`h` percentages applied via `set_size()` after first configure
- **Layout**: Vertically centered, gap = spacing in pixels

## Dependencies (sp Cargo.toml)
```toml
smithay-client-toolkit = { version = "0.21.1", default-features = false, features = ["calloop", "xkbcommon", "system"] }
wayland-client = { version = "0.31.15", features = ["dlopen"] }
wayland-egl = "0.32"
egl = "0.1"
glow = "0.16"
cosmic-text = { version = "0.19", features = ["fontconfig"] }
fontdb = { version = "0.24", features = ["fontconfig"] }
ttf-parser = "0.25"
libc = "0.2.158"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

## Callers (lua, in `~/sources/`)
- `splash_init` — fullscreen boot splash. `dofile`s `fetch_color_scheme`, sources `msg/splash_loading_texts`, writes `bg .. bg_alpha` (hence the 8-digit `#rrggbbaa`) and `alpha = bf`, then `run_bg(dots .. "/sources/sp")`.
- `center_pop` — centered pop window. Same shape, 20%×20%, `bg8`/`bf`, optional `auto_fetch` of the color scheme.

Both write `$states2/sp_vars` and launch `$dots/sources/sp`. Neither passes arguments.

## Build
Target dir is shared via `rust_project_root/.cargo/config.toml` → `/acc/data/p-user/user/.cargo/target`.
```sh
cd ~/.config/hdots/rust_project_root/sp
cargo build --release
cp /acc/data/p-user/user/.cargo/target/release/sp ~/.config/hdots/sources/sp
```
Dev loop: `cargo build` (opt-level 1, debuginfo stripped) → `target/debug/sp`.

## Binary
- Deployed to: `$dots/sources/sp` (i.e. `~/.config/hdots/sources/sp`) — same path both callers launch
- Debug log (unconditional): `/tmp/sp_debug.log`

## Gotchas
- `states2` env var defaults to `/tmp/tw_rconf/states2`
- GL init happens in configure handler (first roundtrip)
- Fonts freed after texture upload (no longer needed)
- `LayerSurface::set_size()` used for w/h percentages
- Esc / Enter close via `App::close()` (destroy surface + release keyboard + `process::exit(0)`)
- Animation loop uses `event_queue.dispatch()` with zero timeout
- `corner_rounding > 0` draws bg as rounded-rect quad; `corner_rounding == 0` uses glClear
- Colors are `0xRRGGBBAA` in memory; `rasterize` converts to cosmic-text's `0xAARRGGBB`
- `w`/`h` are parsed but currently unread (window is always fullscreen overlay)
