# zen-image

Bare-metal Wayland + Vulkan GPU image viewer. No GTK/Qt — just `wayland-client`,
`ash` (Vulkan), and WGSL shaders compiled at startup via `naga`.

## Build

```bash
cargo build            # debug binary (on PATH)
cargo build --release  # optimized binary
```

> Note: this machine's cargo uses a shared target dir — `CARGO_TARGET_DIR`
> resolves to `/acc/data/p-user/user/.cargo/target` (symlinked at
> `/home/tw/.cargo/target`). Binaries land there, **not** in the project folder.

## Binary locations

| Binary | Path | Notes |
|---|---|---|
| debug | `/home/tw/.cargo/target/debug/zen-image` | **on PATH** — just run `zen-image` |
| release | `/home/tw/.cargo/target/release/zen-image` | copy to `~/.local/bin/` if wanted |

## Usage

```bash
zen-image photo.png                  # single image
zen-image ~/Pictures/wallpapers/     # whole folder, browse with arrows
zen-image --dir ~/Pictures/cat.jpg   # open one, browse its whole folder
```

## Supported formats

PNG, JPEG, GIF (static), WebP, TIFF, BMP, ICO, DDS, Farbfeld, QOI, HDR, EXR
(float data is clamped to 8-bit). Format is sniffed from magic bytes, so wrong
extensions still open. AVIF: rebuild with `cargo build --features avif`
(requires `nasm`).

## Controls

| Key | Action |
|---|---|
| Space / N / → | next image |
| P / ← | previous image |
| Home / End | first / last image |
| PageUp / PageDown | previous / next |
| R / T | rotate 90° CW / CCW |
| + / − (also keypad) | zoom in / out |
| scroll wheel | zoom |
| left-drag | pan |
| middle-click | reset view |
| 0 / F | fit to window |
| F11 | fullscreen |
| Esc / Q | quit |
