# zov — niri-like workspace overview for Hyprland

## What it is
Niri-like scrollable workspace overview for Hyprland. A full-size layer-shell window
(keyboard/pointer grabbed) that draws a vertical tape of workspaces, each workspace
as a row of window tiles. Standalone binary: `zov`, `zov toggle`, `zov sel-left|right|up|down`, `zov activate`, `zov close`.

## Project location (updated)
- Standalone crate: `/home/tw/.config/hdots/rust_project_root/zov` (was part of `~/my_workspace`, which no longer exists)
- Build (debug only): `cd /home/tw/.config/hdots/rust_project_root/zov && CARGO_TARGET_DIR=/home/tw/.cargo/target cargo build`
- The binary lands directly at `/home/tw/.cargo/target/debug/zov` — the user keeps
  binaries there (first on PATH). Leave it in place: no copy, no release build,
  no moving anywhere.

## Keyboard selection (implemented & verified live)
- `Left/Right` = move selection within the **same row (workspace) only**, clamped at
  its first/last card — never crosses rows, never wraps (hold-right stops at the
  row's last card).
- `Up/Down` = move selection between workspace rows, keeping the column when possible;
  clamped at the first/last row — never cycles.
- **Row order is always workspace-id order**: ws1 first, ws2 second, … (the active
  row is highlighted but never re-ordered).
- **On open the selection starts on the row of the current workspace**, pre-highlighted
  on the focused window (`focused_card()`; startup snapshot has a retry guard against
  empty first reads).
- **Live refocus**: switching workspaces (or focusing another window) while the
  overview is up moves the highlight to the new active window card. Driven by event
  payloads (`workspace>>N`, `activewindowv2>>0x…`), NOT by re-querying
  activewindow/activeworkspace — those lag a frame behind a switch on this build and
  caused missed re-selections. `drain_events` now returns (event, payload) pairs.
- `Enter/Space` = focus selected window and close. `Escape` = close in-process.
- **SUPER+Left/Right = jump**: move the highlight one card and activate it immediately
  (no Enter). zov tracks the Super modifier itself because the grabbed keyboard
  bypasses Hyprland's binds while the overlay is up. THREE sources, OR-ed in
  `App::super_active()`:
  1. the wl_keyboard **enter** event's held-key list — covers the SUPER+Tab open,
     where Super went down BEFORE the grab and its press was never delivered
     (`is_super_keycode` on raw evdev codes 133/134, plus `is_super_key` on the
     enter keysyms);
  2. raw Super/Meta key press/release tracking (`App.super_held`,
     `config::is_super_key`) — covers Super pressed while the overlay is up;
  3. the compositor's Modifiers event (`App.mods.logo`) — can arrive late or not
     at all mid-grab on this build, so it alone is NOT sufficient (both earlier
     attempts relied on it and the jump silently did nothing).
  `App::resolve_key` resolves Super+arrows as the PLAIN movement always (config
  `[keys]` cannot break the jump), and `press_key` applies move-then-activate.
  `leave` clears `super_held` (held-at-grab state is void). Plain arrows remain
  browse-only.
- **Activate timing**: Enter/click hides the overlay first, then a 120 ms timer
  dispatches the focus (after the keyboard-grab focus-revert settles) and exits.
- Selected tile gets a pink ring (`Theme::selection`, `render.rs::draw`).
- **Card order mirrors the screen**: each row is sorted by the window's on-screen x
  position (y as tie-break) — leftmost window = first card, matching the scrolling
  columns layout. The clients query's mapping order is ignored, and no focus-history
  sort (that pinned the focused window to the first card).
- Key mapping lives in `hypr.rs::KeyAction::resolve`; movement in `main.rs::move_sel/move_row`.

### Keyboard focus note (important)
On this Hyprland (0.56.2, openSUSE build) the overlay **does receive keyboard focus
directly** — the older comment claiming "the compositor never grants this surface
keyboard focus" is outdated. Arrows reach zov in-process via `KeyboardHandler::press_key`.

### Hyprland dispatch dialect (IMPORTANT)
This Hyprland build (0.56.2 + Lua config) evaluates every `.socket.sock` request
through its Lua bridge (`return hl.dispatch(<args>)`), so **raw dispatch strings fail**
(e.g. `dispatch focuswindow address:0x…` → Lua parse error). Working form, verified:
```
dispatch hl.dsp.focus({ window = "address:0x<hex>" })   # focus a window
dispatch hl.dsp.focus({ workspace = N })                # switch ws (existing ones)
```
Implemented in `Hypr::focus_window` / `Hypr::switch_workspace` (sent as plain
`dispatch …` requests, not through `keyword()`). Note `keyword bind/unbind` is also
rejected here ("non-legacy parsers"), so the old submap-relay idea cannot work and
was removed — the overlay gets keyboard focus directly anyway.

### Relay fallback
Removed. It relied on `keyword bind`, which this build rejects.

## Config file (read at startup)
zov parses `~/.config/zov/config` (`$XDG_CONFIG_HOME/zov/config` respected) — the
rconfig_gen-expanded copy of `hdots/sources/templates/zov/config`. INI with three
sections:
- `[theme]` — 14 named colors (`backdrop`, `row_bg`, `row_bg_active`,
  `row_border`, `row_border_active`, `label`, `label_active`, `tile_bg`,
  `tile_bg_active`, `tile_border`, `tile_border_active`, `title`, `class`,
  `selection`), `#rrggbb` or `#rrggbbaa` (alpha ignored, renderer paints opaque).
  The template fills them from hdots theme globals (`$bg8`, `$fg`, `$acc`, …).
- `[layout]` — `font`, `font_size` (parsed, reserved), `gap` (tile spacing),
  `tile_radius` (tile corner radius).
- `[keys]` — `Key = action`; optional `Super+` prefix; actions: `hide`,
  `activate`, `scroll_to_ws` (ws number from the digit key), `move_sel_left/right`,
  `move_row_up/down`. `KP_Enter` normalizes to `Return`.

Missing file/entries fall back to `Theme::default()` + built-in geometry + the
built-in keymap (`seed_builtins` fills any key the user did not configure), so
the overlay always comes up. Implemented in `src/config.rs` (`Config::load()` in
`App::new`; `App::resolve_key` in main.rs replaces direct `KeyAction::resolve`
calls). Super+Left/Right keep their jump semantics (move then activate) even
when configured as `activate`.

## Keybind
- `~/.config/hypr/keybinds.lua`: `SUPER + Tab` → `zov toggle`
  (toggle closes via IPC so the daemon can clean up; the old `pkill zov` bind
  bypassed cleanup and could strand state).

### IPC listener must stay nonblocking
`bind_ipc()` sets the UnixListener nonblocking so the calloop handler drains pending
connections and returns. With a blocking listener the handler parks in `accept()` and
starves the activation timer: activate would hide the overlay but the focus dispatch
+ exit never run (seen in testing 2026-09-30).

## IPC (one-shot daemon, `$XDG_RUNTIME_DIR/zov.sock`)
`open` (spawn+open), `close` (clean shutdown), `toggle`, `sel-left|sel-right|sel-up|sel-down`,
`activate`. The daemon exits after `activate`/`close`/`Escape`. Debug: `ZOV_DEBUG=1`.

## Dependency versions
smithay-client-toolkit = 0.21, wayland-client = 0.31, tiny-skia = 0.11, calloop = 0.14,
cosmic-text = 0.19, fontdb = 0.24. Build clean; 18 unit tests pass (`cargo test`):
9 hypr key tests + 9 config tests. Remaining warnings: unused `focused` field,
unused var `win`, cosmic-text "No default font match for Sans" fallback noise
(harmless).

## Source files
| File | Purpose |
|---|---|
| `src/main.rs` | App struct, Wayland handlers, event loop, IPC, entry point |
| `src/hypr.rs` | Hyprland IPC — workspace/window queries, key action resolution, event socket |
| `src/config.rs` | INI config parsing — theme colors, layout, key bindings (`~/.config/zov/config`) |
| `src/render.rs` | CPU rendering via tiny-skia + cosmic-text (layout, drawing, text) |

## Potential improvements
- [ ] IPC toggle: keep daemon resident (show/hide) instead of exit-on-close
- [ ] Animation: smooth scroll easing (niri spring physics)
- [ ] Window thumbnails via wlr-screencopy
- [ ] Mouse: hover highlighting on tiles (click-to-select already works)
- [ ] Multi-monitor: per-output rendering
- [ ] Config file: colors, sizes, animation params
- [ ] Remove relay fallback if direct keyboard focus proves stable
