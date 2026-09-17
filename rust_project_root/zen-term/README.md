# zen-term

A pure-Wayland GPU-accelerated terminal emulator written in Rust.

- **Raw GL (EGL)** rendering over a Wayland `wl_egl_window` — the same proven
  path foot / alacritty / kitty use. A raw **Vulkan** backend slots in behind
  the same `GpuBackend` interface.
- **alacritty_terminal** emulation core: VT/ANSI parsing, scrollback,
  selection, clipboard, bracketed paste, OSC support.
- **Zero idle cost**: the event loop blocks in `epoll_wait` with no timers and
  no file watches. An idle terminal uses **0% CPU and no polling**.
- **Signal-driven hot reload**: `zen-term --reload` sends `SIGUSR1` to running
  instances, which re-read the config. No inotify, no `mtime` polling.
- **Instant startup**: only the primary font is read at launch; fallback faces
  load lazily on first use. Cold start to first frame is ~60 ms.

## Build

zen-term is a member of the master Cargo workspace, but like every app there
it builds standalone (`default-members = []`):

```bash
cargo build -p zen-term --release
# binary: target/release/zen-term
```

## Run

```bash
target/release/zen-term              # spawns $SHELL in a new window
target/release/zen-term --reload     # hot-reload config in running instances
target/release/zen-term --config PATH
target/release/zen-term --version | -h
```

Requires a Wayland compositor (sway, Hyprland, …). No X11, no GTK/Qt.

## Configuration

First run writes a default config to
`~/.config/zen-term/zen-term.toml` (or `$XDG_CONFIG_HOME/zen-term/zen-term.toml`).

```toml
# Font family. Empty = auto: a curated list of nice monospace fonts is tried
# (JetBrains Mono, Iosevka, Cascadia, Fira Code, …), then any monospaced face.
font_family = ""
font_size = 14.0          # pixels
padding = 4.0             # terminal padding, logical px
background_opacity = 1.0  # 0.0..=1.0

scrollback_lines = 10000
shell = ""                # empty = $SHELL
shell_args = []
working_directory = ""

vsync = true              # off = maximum throughput (tearing possible)
gpu = "gl"                # "vulkan" reserved for the future ash backend

[colors]
background = "#1e1e2e"
foreground = "#cdd6f4"
cursor = "#f5e0dc"
cursor_text = ""          # empty = auto (luminance-inverted)
selection_bg = ""
selection_fg = ""
normal = ["#1e1e2e", "#f38ba8", "#a6e3a1", "#f9e2af", "#89b4fa", "#cba6f7", "#94e2d5", "#bac2de"]
bright = ["#585b70", "#f38ba8", "#a6e3a1", "#f9e2af", "#89b4fa", "#cba6f7", "#94e2d5", "#cdd6f4"]
```

### Hot reload

Edit the config, then from any shell:

```bash
zen-term --reload
```

This scans `$XDG_RUNTIME_DIR` for `zen-term-*.pid` files and sends `SIGUSR1`
to each live instance. The instance re-reads the config, re-rasterizes fonts
and repacks the glyph atlas — colors/font/opacity change live, the session
(shell, scrollback) is untouched. The reloader never watches or polls files.

## Keybindings

| Keys                    | Action                              |
|-------------------------|-------------------------------------|
| `Ctrl+Shift+C`          | Copy selection                      |
| `Ctrl+Shift+V`          | Paste (honors bracketed paste)      |
| middle click            | Paste primary selection             |
| drag left button        | Select                              |
| wheel                   | Scroll history / alternate scroll   |

## Architecture

```
Term (alacritty_terminal) ──┬── pty thread (EventLoop: reads/writes PTY,
                            │          parses escape sequences)
                            └── UI channel (calloop) → redraw, clipboard,
                                                      title, bell, exit

Renderer ── quad list (glyphs, cursor, selection, underlines, damage)
   │
   └── GpuBackend trait
        ├── GlBackend    (glow + EGL + wl_egl_window)   ← implemented
        └── VulkanBackend (ash + VK_KHR_wayland_surface) ← future
```

- `src/wl.rs` — SCTK event loop; blocks in epoll with **zero timers**. The
  `SIGUSR1` handler sets an `AtomicBool`; the loop checks it after each
  dispatch and returns to sleep. Idle cost: 0 CPU.
- `src/render.rs` — text rendering logic shared by every backend.
- `src/render_gl.rs` — EGL/GLES3 (GL 3.3 core fallback), premultiplied
  batching, glyph atlas texture.
- `src/font.rs` — fontdb + ab_glyph; loads **one** font at startup, resolves
  fallback faces lazily per glyph and caches them.
- `src/term.rs` — `Term` + PTY `EventLoop` wiring, theme seeding via OSC.
- `src/input.rs` — keysym/modifier → escape sequences.

## Performance notes

- Startup reads exactly one font file; the full system font index is scanned
  (fast metadata-only pass) but faces are only parsed on demand.
- Font directories are scanned **following symlinks** — fontdb 0.20's system
  scan skips symlinked dirs, which would otherwise hide personal fonts in
  containerized homes and silently pick an emoji/icon font as "monospace".
- Rendering only redraws on actual damage (pty output, input, resize,
  reload), paced by the Wayland frame callback.

## License

MIT
