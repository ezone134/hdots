# zen-wm — Blueprint & Handover Doc

Status: **compiles clean** (`cargo build` green, no warnings from our code). The former
"winit surface not presenting" issue is **RESOLVED — it never existed**: it was a testing
artifact (window tiled offscreen by the host Hyprland + hyprland opacity blending skewing
pixel forensics). The compositor renders and presents correctly. One known limitation
remains: GPU clients (e.g. alacritty) cannot render inside the nested compositor on **this**
machine because its Mesa/EGL build lacks `EGL_WL_bind_wayland_display` (details in section 6b).

Project root: `/home/tw/my_workspace/zen-wm/` (member of the shared workspace at
`/home/tw/my_workspace/`, which has `default-members = []` — **always build with**:

```
cargo build --manifest-path /home/tw/my_workspace/zen-wm/Cargo.toml
```

Binary: `/home/tw/my_workspace/target/debug/zen-wm` (shared target dir, forced by
`/home/tw/my_workspace/.cargo/config.toml`).

---

## 1. What it is

A Wayland window manager in Rust built on **smithay 0.7** ("zen-wm"). First target is the
**nested `backend_winit`** backend (window inside the existing openSUSE/Hyprland session), so
the whole WM can be developed/tested as a regular app. A DRM/headless backend is a later goal.

Feature set (v1):
- Hyprland-flavored keybindings (mod = Super by default; configurable).
- **Scrolling layout by default** (niri-flavored): windows tile as a full-width vertical
  column per workspace; `layout = "master"` switches to dwm-style **master/stack**
  (`master_side` left/right).
- 8 workspaces (config), independent `Space` each; Super+Up/Down cycles only over
  non-empty workspaces.
- **niri-style overview (scroll-over-view)**: Super+Tab toggles a **vertical strip** of
  per-workspace thumbnails (default `workspace_columns = 1`; set >1 for a grid); while
  active the compositor **grabs all keys** (arrow keys move selection, Return selects,
  Tab = next, Escape cancels, and it consumes key *releases* so nothing leaks to the app).
- **Live config reload**: `~/.config/zen-wm/config.toml` is mtime-polled (~2x/s) and
  hot-applied — keybinds, terminal/launcher, gap, layout, saturation, overview grid,
  colors, workspace_count (safe grow/shrink).
- **Hyprland-style config sections**: `[bindings]` (`"Mod+Return" = "terminal"` map,
  `exec` actions, merge-over-defaults) and `[autostart]` (`exec = [...]`, exec-once).
- **Global saturation** via a custom GLES texture shader override (`GlesFrame::override_default_tex_program`),
  toggled Super+S (1.0 → 0.0 → 0.35 → 1.0).
- `shm`, `wlr-layer-shell`, `xdg-shell` (toplevels + popups), data control, primary selection,
  presentation (stub), single-pixel-buffer, viewporter, keyboard-shortcuts-inhibit.
- Config from `~/.config/zen-wm/config.toml` (TOML, serde); missing file → defaults.

---

## 2. File map

| File | Role |
|---|---|
| `src/main.rs` | winit backend init, `bind_wl_display` (wl_drm for EGL clients), calloop event loop, socket source, output globals, per-frame render dispatch |
| `src/state.rs` | `ZenWm` struct, smithay handlers/delegates, input processing, `process_action`, live config reload (`check_config_reload`/`apply_config`), popups, client data |
| `src/renderer.rs` | `render_once` (custom loop, no OutputDamageTracker), cursor element, `draw_overview` strip/grid |
| `src/saturation.rs` | GLSL texture-shader compile + `apply_saturation` per-frame override |
| `src/workspace.rs` | `Workspaces`, per-workspace `Space`, tiling layout, ws switch, window lookup |
| `src/overview.rs` | `OverviewState` (active flag + grid selection index) |
| `src/keybindings.rs` | keymap → `KeyAction` resolution (data-driven from `[bindings]`, normal + overview modes) + key/action parsers + unit tests |
| `src/focus.rs` | `FocusTarget`, `WaylandFocus`/`IsAlive` impls, `PointerTarget`/`KeyboardTarget`/`TouchTarget` |
| `src/element.rs` | `WindowElement(Window)`, `render_elements!` macro `WindowRenderElement<R>` |
| `src/config.rs` | `Config` struct + defaults + TOML load + `layout` field + `[bindings]` (hyprland-style) + `[autostart]` (exec-once) + unit test |
| `Cargo.toml` | deps (smithay 0.7 with `use_system_lib`, toml, serde, tracing, tracing-subscriber+env-filter) |

### Key behaviors (source-level)

- **Tiling** (`workspace.rs::layout_workspace`): window 0 = master on `master_side`
  (left/right half), rest tile vertically in the stack column. Gap from config.
  `map_and_configure` uses `window.configure(state)`/`set_activated`.
- **Window spawn side** (`KeyAction::Terminal(Option<SplitSide>)`): Super+Return →
  `Terminal(None)` (no master change); Super+Left/Right → `Terminal(Some(side))` sets
  `pending_master_side`, consumed in `map_window` via `.take()`.
- **Input routing** (`state.rs::handle_key`): only `KeyState::Pressed` resolves actions.
  Release: overview → `FilterResult::Intercept(KeyAction::None)`; normal mode → `Forward`.
  `process_input_event_windowed` matches winit `InputEvent::{Keyboard, PointerMotionAbsolute,
  PointerButton, PointerAxis, ...}`.
- **Saturation pipeline**: `compile_saturation_program` (state init, stored
  `saturation_program`) → each frame: `frame.clear(...)` **then**
  `apply_saturation(&mut frame, program, state.saturation)` **then** draw elements
  (elements are drawn with the override active, cursor and overview too) → `finish()`.
- **Render loop** (`renderer.rs::render_once`): bind → build `space_render_elements`
  (normal mode) or `import_surface_tree` for each window (overview thumbnails) → clear →
  draw elements `.rev()` (space_render_elements returns back-to-front) → draw overview grid →
  draw cursor last → `frame.finish()` → `backend.submit(&damage)`. Full-window damage v1.
  Bind is scoped in a block so `fb` (GlesTarget, has Drop) releases `backend` before `submit`.

---

## 3. Keybinds (defaults)

Config: `mod_key` = `"Super"`, `layout` = `"scrolling"`, `gap` = 4, `workspace_columns` = 1
(vertical strip / niri scroll-over-view), `workspace_rows` = 8, `workspace_count` = 8,
`saturation` = 1.0, `overview_dim` = 0.65, `background` = `[0x11, 0x11, 0x18]`.

**Default binds** (overridable via `[bindings]` in the config — hyprland-style `"Mod+Return" =
"terminal"` strings; user entries override defaults by key, unmentioned keys keep their default):

| Keys | Action |
|---|---|
| Mod+Return | spawn terminal (`config.terminal`) |
| Mod+Left / Mod+Right | spawn terminal as new master on that side |
| Mod+Space | spawn launcher (`config.launcher`) |
| Mod+Up / Mod+Down | next/prev workspace **with windows** |
| Mod+Tab | toggle overview |
| Mod+s | cycle saturation (1.0 → 0.0 → 0.35 → 1.0) |
| Mod+q | quit |
| any custom `exec ...` bind | spawn an arbitrary command |
| Overview: arrows | move selection (respects columns/rows) |
| Overview: Return | select workspace (switch + focus + deactivate old) |
| Overview: Tab / Shift+Tab | next / prev tile |
| Overview: Escape | cancel (toggle off) |
| Overview: Mod+Tab, Mod+s, Mod+q, Mod+Space | still work while overview grabs keys |

Key syntax: `[Mod4|Super|Mod|Alt|Mod1|Ctrl|Control|Shift]+<key>` (multiple mods allowed,
e.g. `Ctrl+Alt+t`). Key names: letters, digits, `F1`-`F24`, `Return`, `Space`, `Tab`,
`Escape`, `Left/Right/Up/Down`, `Home`, `End`, `Page_Up/Down`, punctuation names (`comma`,
`period`, `minus`, ...). Actions: `terminal`, `terminal left/right`, `launcher`, `overview`,
`workspace next/prev`, `saturation`, `quit`, `exec <cmd args...>` (whitespace-split).
Parsing is unit-tested (`keybindings.rs::tests`, `config.rs::tests`).

Overview keys are fixed for now (arrows/Return/Tab/Escape + mod actions).

---

## 4. Config file`~/.config/zen-wm/config.toml` — currently exists (used for testing):

```toml
terminal = ["/tmp/boss-bins/alacritty"]
launcher = ["/usr/bin/fuzzel"]
layout = "scrolling"          # or "master"
mod_key = "Super"
gap = 4

# Hyprland-style exec-once (runs once at startup):
[autostart]
exec = [
    "/usr/bin/waybar",                                  # bare program
    ["/usr/bin/fuzzel", "--launcher", "drun"],         # or full argv
]

# Hyprland-style keybinds; entries override defaults by key:
[bindings]
"Mod+Return" = "terminal"
"Mod+x"      = "exec /usr/bin/rofi -show drun"
"Mod+q"      = "quit"
```


All fields optional (serde defaults). Keys see `src/config.rs`. Note: `spawn` uses the raw
argv + sets `WAYLAND_DISPLAY` from `state.socket_name` (stored in `ZenWm::new` via main.rs).

**Live reload**: the file is mtime-polled ~2x/s from the main loop
(`state.check_config_reload`); any edit is hot-applied without restart (verified
2026-08-16: gap/layout/saturation/columns/rows/background all applied live, log line
`config reloaded and applied changed=[...]`). `workspace_count` shrinks only if the tail
workspaces are empty (refused + logged otherwise); `saturation` from the file is applied
only when the file value itself changed, so a runtime Mod+s toggle isn't stomped.

`layout = "scrolling"` (default) tiles windows as a full-width vertical column;
`layout = "master"` = dwm-style master/stack. Note: the renderer clear color now comes
from `background` (was hardcoded).

---

## 5. smithay 0.7 API notes (battle-tested while getting to compile)

All verified against `~/.cargo/registry/src/.../smithay-0.7.0/`:

- `ModifiersState` is a **plain struct** (`ctrl, alt, shift, caps_lock, logo, num_lock,
  iso_level3_shift, iso_level5_shift, serialized`) — no `LOGO/ALT/CONTROL/SHIFT` consts, no
  `is_empty()`. Compare bool fields; `==` is unreliable. See `keybindings.rs` `mods_match`/`no_mods`.
- `KeyboardKeyEvent::key_code()` returns xkbcommon `Keycode` (import
  `smithay::input::keyboard::Keycode`), NOT u32.
- `position_transformed` lives on **`AbsolutePositionEvent`** trait
  (`smithay::backend::input`), not `PointerMotionAbsoluteEvent`.
- `PointerButtonEvent::state()` already returns smithay `ButtonState` (import from
  `smithay::backend::input`); `ButtonEvent { button: u32, state: ButtonState, serial, time }`.
- `PointerAxisEvent::amount(axis)` (and `amount_v120`) take an `Axis` arg; `AxisFrame` has no
  `horizontal/vertical` — use builder `source(..).value(Axis::Vertical, v)`.
- `Renderer::render(&mut fb, Size<i32,Physical>, Transform)` → `GlesFrame`; `finish()` returns a
  **`#[must_use] SyncPoint`** (assign `let _ =`), then `backend.submit(&[damage])` swaps.
  `GlesFrame` borrows the renderer **and** the framebuffer, and `GlesTarget` (Framebuffer)
  has a `Drop` → you cannot call `backend.submit` until the bind scope ends.
- `space_render_elements(renderer, &[space], &output, alpha)` is in
  `smithay::desktop::space`; returns `Vec<SpaceRenderElements<R, E>>` back-to-front.
  `OutputRenderElements` is **private** in 0.7 — don't use it.
- `Color32F::new(r,g,b,a)` — no `from_rgb`.
- `MemoryRenderBuffer::from_slice(mem, format, size, scale: i32, transform, opaque_regions: Option<...>)`.
- `with_renderer_surface_state(...)` returns `Option<T>`; wrap + `.flatten()` if the callback
  returns an Option.
- `Window::new` is deprecated → `Window::new_wayland_window(ToplevelSurface)`.
- `Rectangle::new(Point, Size)` takes typed Point/Size (tuples only worked with the deprecated
  `from_loc_and_size`).
- `ClientData` (wayland-backend 0.3.17): `impl ClientData { fn initialized(&self, ClientId) {} }`
  — import from `smithay::reexports::wayland_server::backend`.
- Handlers that are all-default in 0.7 (e.g. `OutputHandler`, `FractionalScaleHandler`) are
  implemented as empty `impl` blocks.
- `Display` moves into `Generic::new(display, Interest::READ, CalloopMode::Level)`; store only a
  `DisplayHandle` in state (`ZenWm::new` takes `DisplayHandle`).
- `on_commit_buffer_handler` stores buffer states only — overview must call
  `import_surface_tree(renderer, &surface)` explicitly.

---

## 6. ~~Open issue~~ RESOLVED: "winit surface not presenting" was a testing artifact

**Original symptom**: grim captures of the parent Hyprland screen showed wallpaper through the
nested winit window, so the compositor's content appeared to never present.

**Actual root cause** (found 2026-08-16):
1. **The winit window was tiled OFFSCREEN by Hyprland's layout.** `hyprctl clients` showed
   "Smithay" at `[1918, 53]` size 1884x1134 on a 1920-wide screen → only ~2px visible. The
   position shifts on every window open/close/focus change (e.g. Freebuff at [-3787,53],
   Thunar at [3818,53]). The previous session's pixel analysis region `(18,13)-(1907,1187)`
   was actually covering **other windows** (Freebuff terminal), not Smithay.
2. **Hyprland applies window opacity**: `decoration:active_opacity 0.8`, `inactive_opacity 0.5`.
   The compositor's clear color `(0.06,0.06,0.08)` = `(15,15,20)` appears on screen blended
   with the wallpaper behind it, e.g. at 0.8 → `(21,22,27)`, at 0.5 → `(18,19,22)`. So
   "no (17,17,24) pixels anywhere" was expected, not a bug.

**Verification**: after floating + centering the window (`hl.dsp.window.float` +
`hl.dsp.window.center`), grim shows the Smithay region = exactly the clear color at 0.8
opacity over the wallpaper (`(21,22,27)`/`(40,33,33)` clusters, ~390 distinct colors, region
pixel count exactly 1280x800) — the compositor's frame **is** presented. `render_once` runs
on-demand (0 renders while idle; bursts only while clients commit). No busy-repaint bug, no
RAM issue from the loop. (`workspaces.refresh()` only returns true on actual window removal.)

**For visual verification, always float+center the window first** (host-layout artifact):
```bash
ADDR=$(hyprctl clients -j | python3 -c "import json,sys; print([c['address'] for c in json.load(sys.stdin) if c.get('title')=='Smithay'][0])")
hyprctl dispatch "hl.dsp.window.float({ window = '$ADDR' })"
hyprctl dispatch "hl.dsp.focus({ window = '$ADDR' })"
hyprctl dispatch 'hl.dsp.window.center()'
```

## 6b. Known limitation: GPU clients can't render inside the nested compositor (this machine)

**Symptom**: alacritty spawned with `WAYLAND_DISPLAY=wayland-2` fails EGL init and never
opens a window (`alacritty.log`):
```
libEGL warning: failed to get driver name for fd -1
MESA: error: ZINK: failed to choose pdev
libEGL warning: egl: failed to create dri2 screen
```
The process stays alive but has no surface. Alacritty requires EGL on Wayland — no SHM
fallback — so the nested WM can't run the default terminal on this box.

**Root cause chain** (verified with C EGL probes in `/tmp/opencode/eglprobe*.c`):
1. Mesa's wayland-platform EGL display on this openSUSE build does **not** advertise
   `EGL_WL_bind_wayland_display` (checked both via probe and via smithay's own
   `bind_wl_display` failure: `EglExtensionNotSupported`). Without it, smithay's
   `ImportEgl::bind_wl_display` cannot create the `wl_drm` global.
2. No `wl_drm` → Mesa clients can't get a DRM fd → "failed to get driver name for fd -1" →
   Mesa falls back to ZINK → ZINK needs a Vulkan device, but the only Vulkan ICD installed
   is **lavapipe** (`/usr/share/vulkan/icd.d/lvp_icd.x86_64.json`; no `vulkan-radeon`/radv)
   → "ZINK: failed to choose pdev".
3. The compositor's own rendering is unaffected (radeonsi DRI works: renderD128 mapped,
   winit EGL display reports hardware 10-bit config + full dmabuf import formats).

**Code added for this (2026-08-16)**:
- `Cargo.toml`: smithay feature `use_system_lib` (system libwayland-server backend; enables
  `ImportEgl`; same backend anvil uses).
- `src/main.rs`: `backend.renderer().bind_wl_display(&display_handle)` right after renderer
  setup, before `ZenWm::new` (display handle moves into state). Logs INFO on success, WARN on
  failure (this machine logs the WARN; fully graceful).

**To fix on this machine later** (pick one, in order of ease):
1. Install the missing **radv** Vulkan driver (`zypper in vulkan-radeon`) — this would make
   ZINK work as a fallback and is likely the real gap; then retest.
2. Find why this Mesa build lacks `EGL_WL_bind_wayland_display` (possibly a packaging quirk;
   the extension is normally present on Mesa wayland-platform displays).
3. Alternatively expose `zwp_linux_dmabuf_v1` via smithay's `DmabufState` and test whether
   modern Mesa EGL clients can drive it without wl_drm.
4. SHM-only clients (simple apps, `weston-terminal`, etc.) should already work.

---

## 7. How to run / smoke test

```bash
# build
cargo build --manifest-path /home/tw/my_workspace/zen-wm/Cargo.toml

# run nested in the Hyprland session (Wayland). Detach so it survives:
setsid RUST_LOG=zen_wm=debug /home/tw/my_workspace/target/debug/zen-wm \
  </dev/null >/tmp/opencode/zenwm.log 2>&1 &

# NOTE: the bash tool will kill plain `nohup ... &` children when the command returns;
# `setsid ... &` survives. `pkill -f debug/zen-wm` stops it.

# IMPORTANT: Hyprland tiles the winit window OFFSCREEN (e.g. at [1918,53] on a 1920-wide
# screen). Always float+center it before screenshotting (lua dispatch, see section 6):
#   hyprctl dispatch "hl.dsp.window.float({ window = '<ADDR>' })"
#   hyprctl dispatch "hl.dsp.focus({ window = '<ADDR>' })"
#   hyprctl dispatch 'hl.dsp.window.center()'
# Then the window sits at [320,220] size 1280x800 (its winit-requested size).

# spawn a client into the nested compositor (it listens on wayland-2 by default):
setsid env WAYLAND_DISPLAY=wayland-2 /tmp/boss-bins/alacritty -e /bin/bash </dev/null >/tmp/opencode/alacritty.log 2>&1 &
# NOTE: on THIS machine alacritty fails EGL and opens no window (section 6b). The WM itself
# is unaffected. Use a SHM-only client or fix the system EGL/Vulkan gaps to see client content.

# screenshot the parent screen:
grim -t png /tmp/opencode/shot.png
```

Then analyze pixels with PIL (the model can't view images — use scripts). Remember the
window's on-screen region is [320,220]-[1600,1020] after float+center, and Hyprland opacity
(active 0.8 / inactive 0.5) blends the compositor colors with the wallpaper:
```python
from PIL import Image; from collections import Counter
im = Image.open('/tmp/opencode/shot.png').convert('RGB')
print(Counter(im.crop((320,220,1600,1020)).getdata()).most_common(5))
# expect (21,22,27)-ish clusters (clear (15,15,20) at 0.8 over wallpaper), NOT flat (15,15,20)
```

Config for tests currently: `~/.config/zen-wm/config.toml` → alacritty at
`/tmp/boss-bins/alacritty`, fuzzel at `/usr/bin/fuzzel` (fish not installed; use `/bin/bash`).

**Input**: no `wtype/ydotool` installed, so sending keys to the focused winit window from the
shell is not yet possible. For full keybind testing you'd want `wtype` (or xdotool on X11).
Without it, keybinds can only be exercised by clicking the window and typing.

---

## 8. Next steps (handover queue)

1. ~~Resolve the "surface not presenting" issue~~ **DONE** — it was a testing artifact (section 6).
2. ~~Live config reload~~ **DONE** (2026-08-16): mtime poll + hot-apply; `layout` option
   (`scrolling` default / `master`); overview vertical strip via `workspace_columns = 1`.
2b. ~~Hyprland-style `[bindings]` + `[autostart]`~~ **DONE** (2026-08-16): data-driven binds
    (merge over defaults, `exec` actions) + exec-once startup; unit-tested.
3. Niri-fication depth (the clarified vision):
   - **Scroll to switch workspaces** — niri switches on scroll-wheel; wire Mod+scroll (and
     plain scroll on empty background) → `WorkspaceSwitch`.
   - **Smooth pan animation** between workspaces (niri's signature feel) — needs a render
     animation state (eased offset), then only the panning workspace is drawn.
   - **Natural window heights in the scrolling layout** (windows keep preferred sizes,
     workspace scrolls internally) instead of equal-height slices.
4. Verify rendering with a real client: blocked by the EGL-client limitation (section 6b —
   try `zypper in vulkan-radeon`), then overview (Mod+Tab), ws switching, saturation toggle.
5. Wire real input if possible (install `wtype` or add a debug key-injection path).
6. Consider `damage_tracking` (buffer_age) once full-window redraw is confirmed working.
7. Write `README.md` + project `AGENTS.md` (keybinds, config format, winit-first vs future DRM).
8. Reconsider the `bind_wl_display` WARN: it's noise on this machine but correct on
   EGL_WL-capable systems; keep it, or gate it behind a debug level.

## 8b. Hyprland 0.55.4 lua-dispatch cheat sheet (host window control)

Since 0.55, `hyprctl dispatch` parses Lua — legacy syntax (`focuswindow address:...`,
`movewindowpixel`, `--batch`) is rejected. Working forms (verified):

```bash
hyprctl dispatch 'hl.dsp.window.float()'                     # float the ACTIVE window
hyprctl dispatch 'hl.dsp.window.center()'                    # center active (floating) window
hyprctl dispatch 'hl.dsp.focus({ direction = "right" })'     # directional focus
hyprctl dispatch 'hl.dsp.focus({ window = "0xADDR" })'       # focus by address
hyprctl dispatch 'hl.dsp.window.float({ window = "0xADDR" })' # float by address
hyprctl dispatch 'hl.dsp.exec_cmd("ghostty")'                # exec
```
Discover the API: `hyprctl eval 'print(...)'` is quiet; probe candidate names — nil names
error with "attempt to call a nil value", valid ones run.

## 9. Environment facts

- rustc 1.94.1, openSUSE Tumbleweed, Hyprland 0.55.4 (Wayland session, `WAYLAND_DISPLAY=wayland-1`),
  eDP-1 1920x1200, AMD radeonsi (Mesa 26.1.4), GL ES 3.2, AMD Lucienne APU.
- **Vulkan: only lavapipe (software) ICD installed** — no `vulkan-radeon`/radv → ZINK can't
  choose a pdev. GL (radeonsi DRI) works fine via renderD128.
- Hyprland window opacity: `decoration:active_opacity = 0.8`, `inactive_opacity = 0.5` —
  account for it in pixel forensics.
- smithay's `use_system_lib` feature is now enabled (system libwayland-server backend;
  `/usr/lib64/libwayland-server.so` present).
- Shell: bash tool runs each command in a fresh-ish persistent shell; long-running child
  processes make the tool wait to its 120s timeout — plan around it (nohup/setsid + ignore
  timeout).
- Reference sources: smithay 0.7.0 at `~/.cargo/registry/src/index.crates.io-.../smithay-0.7.0/`;
  anvil reference files at `/tmp/opencode/anvil-*.rs` (winit init/loop, input handlers, delegates).
- Visual artifacts from this session: `/tmp/opencode/zen*.png`, `nested*.png`, `zenwm*.log`,
  `alacritty.log`.
