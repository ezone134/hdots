# zen-shell — roadmap

The plan for the zero-overhead Wayland shell bar. Statuses:

- ✅ done (this session / live on Hyprland)
- 🟦 near-term — small, self-contained, high value
- 🟩 medium — a few days each, touches a subsystem
- 🟥 long-term / vision — big lifts, revisit when the foundations are solid

---

## Current state (2026-08-16)

A floating pill that morphs into a dashboard on hover, plus a full panel
ecosystem — all event-driven (0 CPU idle), pure Rust, no external binaries
for anything we can do in-process.

- **Surfaces**: the bar (morphing layer surface) + a `Layer::Background`
  **wallpaper daemon** + a top-right **notification popup** — three GL
  contexts on one EGL display, switched per frame.
- **Panels**: control center, launcher (scrollbar + keyboard nav), command,
  notifications, calendar, settings, power (3s hold-to-confirm), wifi, bluetooth,
  per-app **audio**, **wallpaper picker**, **weather detail**, **clipboard
  history** (text + images), OSD, native lockscreen.
- **Collapsed-pill features**: music **visualizer bars** (toggle in Settings),
  **drag-to-reorder** left-cluster items (config: `[bar] pill_order`),
  **drag-to-reorder from Settings** (⠿ grip handles on pill item rows — click
to toggle, drag vertically to reorder). Pill bg is **semi-transparent**
  (see-through pill shape over wallpaper).
- **Services**: system tray (SNI), `org.freedesktop.Notifications` daemon +
  history, MPRIS, /sys battery/AC/brightness, /proc system stats, Open-Meteo
  weather, wlr-data-control clipboard manager.
- **Backends layer** (`[backends]`): sysfs | upower · sysfs brightness ·
  wpctl audio · native | hyprlock lockscreen · **command wallpaper** (`set_wall
  {path}`).
- **IPC**: `zen-shell open …` / `ipc call …` over a unix socket (see
  `commands` + `docs/ipc.md`).

---

## 🟦 Near-term — quick wins

**Already done (this session)**
- [x] Pomodoro **Focus card** (`pomodoro`): mm:ss countdown, start/pause/
      reset, focus-length steppers, focus↔break auto-flip + session counter,
      persisted to `$XDG_DATA_HOME/zen-shell/pomo.txt`; 1 s countdown timer
      self-tears down when paused (0 CPU at rest).
- [x] **Fans card** (`fans`): hwmon `fanN_input` RPMs, per-fan bars against
      a slow-decaying peak, stopped fans dim.
- [x] **Workspaces card** (`workspaces`): clickable Hyprland workspace
      tiles, active accent-tinted, dispatch via `pending_ws_dispatch`.
- [x] **Water-fill battery icons**: both battery cards draw a liquid that
      touches the case stroke on both sides, with a wave crest + highlight.
- [x] **World clock card** (`worldclock`): pinned cities with sun/moon
      glyphs, mono local times, offset-vs-local chips, inline zone.tab
      search to add; batched DST-safe offset probe on a worker thread.
- [x] **Water tracker card** (`water`): bead-ring gauge, +glass/−undo,
      swipe-to-set goal, midnight auto-reset.
- [x] **Moon phase card** (`moon`): vector shaded disc from synodic-cycle
      math, phase name + illumination % + days-until-full.
- [x] Ultra-fast startup: all file I/O deferred past the first frame; app
      manager scans .desktop files on a background thread. True sub-frame
      startup — bar + launcher appear before any disk I/O.
- [x] Color-temperature slider range widened to 1200 K–6500 K (was 2500–6500).
- [x] Settings drag-to-reorder: ⠿ grip handles on pill item rows in Settings.
      Click toggles, drag vertically reorders. Ghost row shows drag position.
- [x] Collapsed pill semi-transparent background (bg color at ~69% alpha).
- [x] Wallpaper backend switched to `command` (`set_wall {path}`); daemon
      backend disabled by default. Wallpaper picker panel enlarged to 620×540
      with 4×5 thumbnail grid (20 visible, scrollable).
- [x] Clock/visualizer overlap fix: clock flows sequentially from left instead
      of centering in the pill width.
- [x] Weather v2 card (`weatherv2`): minimalist light "paper" card — left hero
      (Today · city · big glyph · oversized temp · H/L) + right 5-row forecast
      strip (glyph + temp per day); classic Weather card stays.


**Reliability & verification**
- [ ] Live input-path verification: tray click → `Activate`, scroll → `Scroll`,
      notification click-through, clipboard copy/paste round-trip. The morph
      is live-verified; the rest needs a real input-injection harness.
- [ ] Commit the session's work; tag a v0.1 release.
- [ ] Runtime config reload (SIGHUP → re-read `shell.toml`, apply colors /
      sizes without a restart).

**Text & fonts**
- [ ] CJK / non-Latin fallback: `text.rs` → Noto Sans CJK SC (and per-script
      fallbacks: Arabic, Devanagari…) when the primary family lacks glyphs.
- [ ] Load only configured families instead of every system font (RAM).

**Card ideas (unbuilt, same recipe: enum + id + drawer + tray)**
- [ ] Countdown / days-until — pinned events with dates; pairs with Calendar
      and now the Water card's persistence pattern.
- [ ] Eye rest (20-20-20) — piggyback the pomodoro machine: every 20 min
      of focus, flash a "look away" OSD.
- [ ] Compositor / Effects card — blur/shadow/opacity/animation tiles + a
      screenshot button; the `*_main` scripts already ship.
- [ ] Mic meter — input-level bar + mute toggle from PipeWire (Viz covers
      output only).
- [ ] Audio device picker — sink/source list, click-to-default (per-app
      audio panel exists; no grid card).
- [ ] Connection info — local IP, gateway, DNS, public IP (on-demand),
      interface state; the Network card is the graph, not the identity.
- [ ] Latency monitor — continuous ping sparkline to a configurable host.
- [ ] Power draw graph — battery/GPU watts over time, dual-line like DiskIo
      (hwmon `power*_average` + existing battery_watts).
- [ ] Storage health (SMART) — per-disk temp + status via `smartctl`
      (command-backend style).
- [ ] Failed systemd units — green dot at rest; red count + unit list on
      failure; pairs with Docker.
- [ ] Journal tail — last N `journalctl -p warning` lines, color-coded,
      click → full log panel.

**UX polish**
- [ ] Keyboard navigation for ALL panels (Tab / arrows / Enter / Esc), not
      just launcher + clipboard — consistent focus model across the shell.
- [ ] Reduced-motion option (`animation.reduced = true` → morphs become
      instant commits).
- [ ] Notification actions (buttons on the corner popup cards).

## 🟩 Medium — subsystem work

**Multi-monitor**
- [ ] One bar + one wallpaper + one popup per output (per-monitor wallpaper,
      correct monitor for `wallpaper set`, workspace previews per monitor).
- [ ] Fractional scale / HiDPI: honor `wl_output` scale (buffer_scale, logical
      sizing), crisp text at 125% / 150%.
- [ ] Output hotplug: move surfaces, re-cover-fit wallpapers, re-anchor.

**Wallpaper daemon**
- [ ] Transition library (swww-style): crossfade, wipe, zoom — a small
      shader/timer framework on the background surface.
- [ ] Slideshow mode: timed rotation through `[wallpaper] dir` with transition.
- [ ] Per-monitor assignment (`zen-shell wallpaper set <path> --monitor HDMI-A-1`).

**Clipboard**
- [ ] Move reads off the event loop (calloop fd source instead of bounded
      blocking read) so a giant copy never pauses animations.
- [ ] Search / filter in the history panels; pin favorites; persist thumbnails.
- [ ] Primary selection support (wlr-primary-selection) for middle-click.

**Lockscreen**
- [x] Real lock: ext-session-lock-v1 (blocks compositor keybinds, routes all
      input to the lock surface) + PAM verification on a worker thread.
- [ ] Auto-lock: idle timeout (hypridle-like, own timer) + lock on suspend.
- [ ] Fingerprint / alternative auth methods behind the PAM backend.
- [ ] Multi-monitor: a lock surface per output.
- [ ] Session-aware unlock signals (logind `Lock`/`Unlock` dbus messages).

**Audio**
- [ ] pipewire-dbus backend behind the `Audio` trait (drop the `wpctl`
      subprocess entirely) — device enumeration, stream volume, app icons.
- [ ] Default-output switching (sink picker in the audio panel).

**Notifications**
- [ ] Notification grouping by app; per-app DND rules; urgency-based
      prioritization (critical breaks through DND with a persistent banner).
- [ ] Popup hover-pause (timer stops while the cursor is over the popup).

**Performance & architecture**
- [ ] Share one GL context across surfaces (texture atlases) instead of three
      contexts — fewer resources, one cache.
- [ ] Event-driven MPRIS (D-Bus signals) instead of 3s polling; same for
      NetworkManager state.
- [ ] Wake-budget audit: assert per-frame wake count with ZEN_TRACE.

## 🟥 Long-term / vision

**Extensibility**
- [ ] JSON-RPC over the IPC socket (instead of bare line commands): scripting,
      `zen-shell eval '…'`, subscribe/notify events (mode changed, notification,
      clipboard changed).
- [ ] Plugin system: user-provided widgets (QML-like declarative layout
      language à la quickshell) — the `ui.rs` component library is the seed.
- [ ] Theme packages: drop-in `theme/` dirs (colors, fonts, radii, icons) that
      override `shell.toml` without editing config.

**Rendering**
- [ ] Software fallback renderer (wl_shm) for machines without GL — the scene
      graph is backend-neutral already; a CPU rasterizer closes the gap.
- [ ] Compositor effects: blur/transparency on panels (Hyprland
      `background_effect` / KDE protocols), rounded-corner shadows.
- [ ] Live wallpapers (video / shader) behind a daemon backend.

**System integration**
- [ ] `org.freedesktop.ScreenSaver` + idle inhibition (so fullscreen video
      doesn't lock).
- [ ] Global media hotkeys (XF86AudioPlay etc.) via the keyboard path.
- [ ] Power: hybrid sleep / hibernate actions, lid-close policy.

**Distribution**
- [ ] Nix flake + AUR package + `cargo install`; systemd user service /
      Hyprland autostart snippets.
- [ ] `zen-shell(1)` man page; `zen-shell doctor` (env/backend diagnostics);
      sample configs for common themes.
- [ ] CI: cargo check/test/lint + a headless layout golden-test harness.

**Robustness**
- [ ] Fuzz the parsers (wpctl, /proc, IPC lines, clipboard mimes).
- [ ] Structured logging (ZEN_TRACE → JSONL) + log rotation.
- [ ] Watchdog: detect a wedged surface (no configure ack) and self-heal.

---

## Guiding principles (why items are ordered this way)

1. **0 CPU / 0 wasted frames stays sacred** — anything that adds a free-running
   timer or a per-frame syscall gets reworked or cut.
2. **No external binaries for things we can do in-process** — each feature
   first asks "can I read /proc or /sys, or bind a Wayland protocol?" before
   spawning anything.
3. **Backends over hard-coding** — new integrations land behind the existing
   `[backends]` traits so swapping (wpctl → pipewire-dbus, sysfs → upower)
   stays a config change.
4. **Document as you go** — every shipped feature updates `README.md`,
   `docs/ipc.md`, and `commands` in the same commit.
