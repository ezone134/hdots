# zen-overview — Project Status (2026-09-01)

## What it is
Niri-like scrollable workspace overview for Hyprland. A full-size layer-shell window
(keyboard/pointer grabbed) that draws a vertical tape of workspaces, each workspace
as a row of window tiles. Standalone binary: `zen-overview`, `zen-overview toggle`, `zen-overview close`.

## Workspace context
- Workspace root: `/home/tw/my_workspace/Cargo.toml`
- Source: `rust_project_src/overview/` (symlinked to `/acc/common/hdots/rust_project/overview/`)
- Build: `cargo build -p zen-overview` from `/home/tw/my_workspace/`
- Already-compiled sibling: `zen-shell` binary at `target/debug/zen-shell`

## Dependency versions (workspace-defined)
- smithay-client-toolkit = 0.21
- wayland-client = 0.31
- tiny-skia = 0.11
- calloop = 0.14
- cosmic-text = 0.19, fontdb = 0.24

## Source files
| File | Purpose |
|---|---|
| `src/main.rs` | App struct, Wayland handlers, event loop, IPC, entry point |
| `src/hypr.rs` | Hyprland IPC — workspace/window queries + event socket |
| `src/render.rs` | CPU rendering via tiny-skia + cosmic-text (layout, drawing, text) |

## What was fixed today (47 → 1 error)
The original code was written against older SCTK/tiny-skia/calloop APIs. Both `main.rs`
and `render.rs` were **rewritten** to match the actual installed versions:

### main.rs changes
- `delegate_registry!(App)` + `delegate_dispatch2!(App)` replace manual Dispatch impls
- `Shm` + `SlotPool` + `ShmHandler` for proper SHM buffer rendering
- `WaylandSurface` trait imported for `.commit()` and `.wl_surface()`
- `Anchor` imported from `smithay_client_toolkit::shell::wlr_layer`
- `registry_queue_init` imported from `wayland_client::globals`
- `LoopHandle<'static, App>` (lifetime parameter required)
- `LayerSurfaceConfigure.new_size` is `(u32, u32)` — cast to i32 where needed
- KeyboardHandler: `press_key`, `repeat_key`, `release_key`, `update_modifiers` (old API used `key_state_changed`, `modifiers`, `repeat_info`)
- `enter` signature: `(serial: u32, raw: &[u32], keysyms: &[Keysym])`
- `leave` signature: extra `serial: u32` parameter
- Pointer Axis scroll: `vertical.discrete`, `.value120`, `.absolute` (not `Option<Fixed>`)
- `WaylandSource::insert()` replaces `quick_insert()`
- `event_loop.dispatch(timeout, &mut app)` — 2 args, no callback (calloop 0.14)
- Frame callbacks via `FrameCallbackData` for continuous redraw
- `draw()` method uses `SlotPool::create_buffer` + damage + frame + attach + commit

### render.rs changes
- `fill_path(path, paint, FillRule::default(), Transform::identity(), None)` — 5 args
- `stroke_path(path, paint, &Stroke { width, ..Default::default() }, Transform::identity(), None)` — 5 args
- `PaintStyle::Stroke` / `paint.set_style()` / `paint.stroke_width` removed — use `Stroke` struct
- Added missing `focused: false` field to all `TileLayout` constructors
- `max_tile_w` annotated as `f32` to fix ambiguous numeric type

## Current build status
```
cargo build -p zen-overview
```
**1 error remaining** — `App::new` called with `&qh` instead of `qh` (by value).
This was the last fix applied but the session ended before verification.

**Fix in main.rs line ~604:**
```rust
// BEFORE (error):
let mut app = match App::new(conn, &qh, globals) {
// AFTER (fixed):
let mut app = match App::new(conn, qh, globals) {
```

Plus 3 warnings (unused imports `Ordering`, `AsRawFd`; unused variable `win` in render.rs).

## To continue tomorrow

### Step 1: Verify build
```bash
cd /home/tw/my_workspace && cargo build -p zen-overview 2>&1
```
Should compile with only warnings. Fix any remaining issues.

### Step 2: Clean warnings
- Remove `use std::sync::atomic::Ordering` from main.rs (unused)
- Remove `use std::os::fd::AsRawFd` from main.rs (unused at top level, only used in main fn)
- Prefix `win` with `_` in render.rs line 171, or use it

### Step 3: Test runtime
```bash
# On Hyprland:
./target/debug/zen-overview           # opens the overview
./target/debug/zen-overview toggle    # IPC toggle
./target/debug/zen-overview close     # IPC close
```

### Step 4: Potential improvements
- [ ] IPC toggle: implement show/hide without exiting (currently exits on toggle)
- [ ] Animation: smooth scroll easing (niri has spring physics)
- [ ] Window thumbnails: render actual window content via wlr-screencopy
- [ ] Keyboard: Escape closes, 1-9 jumps to workspace, Arrow keys scroll
- [ ] Mouse: hover highlighting on tiles, click to select
- [ ] Exclusion list: filter zen-shell / bar windows from overview
- [ ] Multi-monitor: per-output rendering
- [ ] Config file: colors, sizes, animation params
- [ ] Release profile: test with `cargo build -p zen-overview --release`

## Hyprland keybind example
```conf
bind = SUPER, TAB, exec, zen-overview toggle
bind = SUPER, ESCAPE, exec, zen-overview close
```
