# splash — Project Notes

## Overview
GPU-accelerated Wayland splash screen. Reads config from `$states2/splash_vars` (JSON), renders text on solid background via EGL/GL, sleeps `duration` seconds, exits. Dismisses immediately on **Esc** / **Enter** (rofi-style) or when keyboard focus is lost.

## Key Files
- `src/main.rs` — single-file binary, ~550 lines
- `Cargo.toml` — depends on workspace members + external crates
- `../splash_init` (in `hypr_bin/sources/helpers/`) — bash script that writes `splash_vars` JSON and launches splash binary

## Workspace
- Workspace root: `my_workspace/Cargo.toml` (symlinked from `~/.config/hdots/rust_project/`)
- Package name in Cargo.toml: `splash`
- Build: `cargo build -p splash` from workspace root
- `edition.workspace = true` → edition 2021

## Config File (`$states2/splash_vars`)
Format: JSON object. Parsed with `serde_json`.

### Fields
| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `w` | f32 | 100 | Window width in percentage of output |
| `h` | f32 | 100 | Window height in percentage of output |
| `bg` | hex string | "#000000" | Background color |
| `fg` | hex string | "#ffffff" | Foreground/text color |
| `b_text` | string | "" | Big text content |
| `b_font` | string | "sans-serif" | Big text font family |
| `b_size` | f32 | 100.0 | Big text font size |
| `s_text` | string | "" | Small text content |
| `s_font` | string | "sans-serif" | Small text font family |
| `s_size` | f32 | 40.0 | Small text font size |
| `duration` | u64 | 3 | Splash screen duration in seconds |
| `spacing` | f32 | 50.0 | Uniform pixel gap between big text, loading animation & small text |
| `loading_id` | u32 | 1 | Loading animation id (1=braille spinner, 2=braille dots, 3=quarter circles) |
| `show_loading_animation` | bool | true | Show/hide loading animation |
| `center_b_text` | u32 | 0 | 1=center whole group vertically, 0=big text at exact vertical center |
| `corner_rounding` | f32 | 0.0 | Corner rounding radius (0=no rounding, draws bg via clear) |

### Example JSON
```json
{
  "w": 100,
  "h": 100,
  "bg": "#2e3440",
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
  "corner_rounding": 0
}
```

## Architecture
- **Config**: Read JSON from `$states2/splash_vars` via serde_json
- **Font loading**: Targeted — scans system font dirs for only the two configured families
- **Text rasterization**: `cosmic-text` → premultiplied RGBA8 pixels
- **Rendering**: EGL → GLES 3.0 / GL 3.3 core → `glow` → textured quads
- **Wayland**: `smithay-client-toolkit` layer shell (Overlay, fullscreen, `KeyboardInteractivity::Exclusive`)
- **Keyboard**: `SeatState` bound in registry; `SeatHandler` grabs `wl_keyboard` on `Capability::Keyboard`; `KeyboardHandler::press_key` closes on `Keysym::Escape` / `Keysym::Return` / `Keysym::KP_Enter`; `leave` and a `Keyboard` capability drop also close (never left stuck up fullscreen with no focus). Seats arrive on a later registry roundtrip after layer creation — keyboard grabs once the compositor advertises the keyboard capability.
- **Animation**: Pre-rasterizes braille spinner frames, cycles at ~10fps in event loop
- **Window sizing**: `w`/`h` percentages applied via `set_size()` after first configure
- **Layout**: Vertically centered, gap = spacing in pixels

## Dependencies (splash Cargo.toml)
```toml
smithay-client-toolkit.workspace = true
wayland-client = { workspace = true, features = ["dlopen"] }
wayland-egl = "0.32"
egl = "0.1"
glow = "0.16"
cosmic-text = { version = "0.19", features = ["fontconfig"] }
fontdb = { version = "0.24", features = ["fontconfig"] }
ttf-parser = "0.25"
libc.workspace = true
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

## splash_init (bash)
- Sources `fetch_color_scheme` for `$bg`/`$fg`
- Sources `splash_loading_texts` for random message array
- Writes JSON to `$states2/splash_vars`
- Launches `splash` binary in background

## Binary
- Built binary copied to: `$hdots/sources/helpers/splash` (i.e. `hypr_bin/sources/helpers/splash`)
- After rebuild: `cargo build -p splash && cp target/debug/splash ~/hypr_bin/sources/helpers/splash`

## Gotchas
- `states2` env var defaults to `/tmp/tw_hypr_conf/states2`
- GL init happens in configure handler (first roundtrip)
- Fonts freed after texture upload (no longer needed)
- `LayerSurface::set_size()` used for w/h percentages
- Esc / Enter close via `App::close()` (destroy surface + release keyboard + `process::exit(0)`)
- Animation loop uses `event_queue.dispatch()` with zero timeout
- `corner_rounding > 0` draws bg as rounded-rect quad; `corner_rounding == 0` uses glClear
