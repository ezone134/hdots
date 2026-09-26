# Quickshell Project Roadmap

This roadmap is tailored to the current project structure.

## Current structure

### Entry / shell surface
- `quickshell/shell.qml`
- `quickshell/core/ShellRoot.qml` + `core/PanelRegistry.qml`
- `quickshell/core/MorphSurface.qml`, `core/MorphContainer.qml`, `core/MorphPlaceholder.qml`

### Layout (module per directory, each with its own `qmldir`)
- `components/` — pure reusable widgets: `Txt`, `TxtInput`, `Slider`, `ToggleSwitch`
- `services/` — singletons/state: `Theme`, `SettingsManager`, `StateController`,
  `ShellIpc`, and every `*Manager` (Audio, Brightness, Calendar, Clipboard, Hyprland,
  Mpris, Network, Notification, Polkit, Power, Volume, Wallpaper, App)
- `core/` — island engine: `ShellRoot`, `MorphSurface`, `MorphContainer`,
  `MorphPlaceholder`, `PanelRegistry`, `Pill`
- `modules/<feature>/` — one dir per panel: `audio`, `calendar`, `clipboard`,
  `command`, `control`, `launcher`, `mpris`, `notifications` (incl. `ToastDaemon`),
  `osd`, `polkit`, `power`, `settings`, `wallpaper`, `window`, `workspace`

### CLI — `*_main`
The shell's CLI is a family of one-per-domain bash tools: `scripts/<domain>_main`
(symlinked as `~/bin/<domain>_main`, so they're on `$PATH`) — `audio_main`,
`brightness_main`, `theme_main`, `power_main`, `caffeine_main`, `pp_main`,
`shot_main`, `media_main`, `network_main`, `ws_main`, `status_main`. Every
`*_main` sources `scripts/qs_main_common` (state-path defaults + `err()`) then its
`<domain>_body` function library and dispatches one `case` (adding a knob = a
function + one case line). External tools (`wpctl`, `brightnessctl`, `playerctl`, `nmcli`,
`bluetoothctl`, `hyprctl`) are used only for the actual device I/O — every
line of output is parsed with parameter expansion / `[[ ]]` / `read`, never a
text-munging binary.

```
audio_main vol|mic [get|set N|up [N]|down [N]|mute|unmute|toggle]
audio_main audio [sinks|sources|set-sink N|set-source N|streams|stream-vol ID PCT|stream-mute ID]
brightness_main [get|set N|up [N]|down [N]]         backlight
theme_main [status|dark|light|toggle|restore|reapply]
power_main [status|battery|ac|state]
power_main session [lock|logout|suspend|hibernate|reboot|shutdown]
caffeine_main [status|on|off|toggle]
pp_main [get|set PROFILE|cycle]
shot_main [full|region]
media_main [status|play|pause|next|prev|stop]
network_main [wifi|bt] [status|on|off|toggle]
ws_main [cur|next|prev|goto N|N]             Hyprland workspaces
status_main                                  everything at once
```

Notes:
- No subcommand = get/status. `battery`/`ac` are read straight from sysfs
  (pure bash, no external process); `bri` percent is pulled from
  `brightnessctl -m`'s 4th field with pure parameter expansion.
- Hyprland 0.55 dispatchers are Lua now: `hyprctl dispatch workspace next`
  no longer works — the CLI uses `hyprctl dispatch "hl.dsp.focus({workspace=...})"`
  (same API the Lua keybinds use).
- **Global env vars are the source of truth — never hardcode paths.** The
  `*_main` CLIs consume the paths exported by `~/.config/hypr/env.lua`
  (`hl.env`): `rconf`, `states`, `states2`, `hypr_global`, `hdots`,
  `rcache`, `hypr_bin`, `hypr_sources`, … They are ALWAYS set in the live
  session; the `: "${var:=…}"` fallbacks in `scripts/qs_main_common` exist
  only so the CLIs run from a bare sandbox/terminal or cron. Fallbacks must
  stay in sync with `env.lua` (`states2` ≠ `states_2`, `hypr_global` =
  `${rconf%/*}/hypr_global`). The `*_body` helper files live
  DIRECTLY in `scripts/` next to the mains — only `power_body` /
  `theme_main_body` still source hdots helpers (`power_body`,
  `theme_body`) that stay in `$hypr_sources/helpers/` (the apply/switch
  funcs formerly in `helpers/theme_main_body` merged into
  `scripts/theme_main_body`, 2026-08).
- **tmpfs-first, sync-at-session-end (minimal SSD writes).** Everything runs
  on RAM-backed tmpfs on purpose — `$hypr_bin`, `$hypr_sources`, `$states`,
  `$states2`, `$rcache` and the live quickshell config are ALL under
  `/tmp` (see "Launch pipeline" below); persistence happens only when a
  session action runs. Every `power_main session <action>` sources
  `$hypr_sources/helpers/power_body` first, which fires `sync_back`
  (rsync `$states/` → `$hdots/states/`, `$rconf/quickshell/` →
  `$hdots/.config/quickshell/`, `$hypr_global/states/` →
  `$hdots/states_global/` + conditional accent cache) and `wait`s before
  the action proceeds — so state is persisted before lock/logout/reboot/
  shutdown. Never write to persistent storage on every change.
- **`$states` persists, `$states2` is ephemeral.** `$states` is synced back
  and survives reboots; `$states2` is NOT synced back — it holds throwaway
  runtime values (rofi pid/id, channel, …). Only `channel` is copied
  one-way into `$states/s` at sync time. Anything that must survive a
  reboot belongs in `$states`; `$states2` is scratch only. See the
  "State files" section below for the file catalog + audio state table.

## State files — `$states` (persists) vs `$states2` (ephemeral)

Two RAM-backed dirs under `$rconf` (`states` and `states2`) hold every
runtime value the shell and hdots scripts share. Nothing here touches the
SSD during the session.

- **`$states` — persists.** Synced back by `sync_back` (runs on logout /
  shutdown / reboot / lock via `power_main session …`) to `$hdots/states/` and
  restored at startup. Holds per-channel settings (`acc_*`, `battery_*`,
  `allow_tearing_*`, `acc_scrim_*`, …), `accent_caching` (0/1, gates the
  accent-cache rsync), `caffeine` (0/1), `hyprsunset_state` (eye-care tile),
  `audio_pop_notif`, and `current_wall_<ch>` (active wallpaper per channel).
  At sync time `$states2/s` is copied one-way into `$states/s`.
- **`$states2` — ephemeral, never synced back.** Scratch for the current
  session only: the audio state files below, `channel` / `mode` / `cm`
  (theme channel state), `rofi_pid` / `rofi_id` (popup lifecycle),
  `caps_lock_state`, `battery_found`, `audio_id` (PipeWire master sink),
  `cur_theme` / `cur_font` / … Anything that must survive a reboot belongs
  in `$states`, not here.

### Audio state (shared by `audio_main`/`audio_body`, hypr_init)

Volume/mic state lives in `$states2` (so a status read NEVER spawns `wpctl`);
the `max_vol`/`max_mic` clamps live in `$states` and persist across sessions.
`wpctl` runs only to apply an actual change (one spawn per mutation).

| file        | format                        | written by                              |
| ---         | ---                           | ---                                     |
| `vol`       | int 0..`max_vol` (percent)    | hypr_init seed, `audio_main`                |
| `vol_f`     | wpctl fraction "0.40"        | derived `%d.%02d` of `vol/100`          |
| `vol_muted` | 0/1                           | mute / unmute / toggle                  |
| `mic`       | int 0..`max_mic`              | same as `vol`                           |
| `mic_f`     | fraction "1.00"              | same as `vol_f`                         |
| `mic_muted` | 0/1                           | same as `vol_muted`                     |
| `max_vol`   | clamp (default 500)           | hypr_init → `$states` (persists)        |
| `max_mic`   | clamp (default 200)           | hypr_init → `$states` (persists)        |

Seeded ONCE at startup by `hypr_init_logic` (reads `wpctl get-volume` and the
`MUTED` flag); `audio_body`'s `vol_ensure` / `mic_ensure` re-seed from
the device only when a file is missing (bare terminal before Hyprland). Rule:
to read a value, read the state file — never spawn a binary; to change a value,
write the state file AND apply via the device exactly once.

---

## Launch pipeline — boot blueprint (tmpfs vs persistent)

The hdots stack runs on tmpfs on purpose (minimal SSD writes). Almost every
path has TWO trees: a PERSISTENT one under `$hdots` (= `/acc/common/hdots`,
symlinked as `~/.config/hdots`) that survives reboots, and a RAM-backed tmpfs
one under `/tmp` that is wiped and re-copied from `$hdots` at every boot.

| tree | tmpfs? | wiped at boot? | syncs back? |
| --- | --- | --- | --- |
| `$hypr_bin` = `/tmp/<user>_rconf/hypr_bin` | ✅ RAM | ✅ wiped, recopied from `$hdots/hypr_bin/` | ❌ (persistent copy is canonical) |
| `$hypr_sources` = `/tmp/<user>_rconf/sources` | ✅ RAM | ✅ wiped, recopied from `$hdots/sources/` | ❌ |
| `$states` = `/tmp/<user>_rconf/states` | ✅ RAM | — | ✅ `sync_back` → `$hdots/states/` |
| `$states2` = `/tmp/<user>_rconf/states2` | ✅ RAM | — | ❌ never (ephemeral) |
| `$rcache` = `/tmp/<user>_rconf/rcache` | ✅ RAM | — | ⚠️ accent files only |
| live quickshell = `~/.config/quickshell` → `/tmp/<user>_rconf/quickshell` | ✅ RAM | ✅ recopied from `$hdots/.config/quickshell/` | ✅ `sync_back` → `$hdots/.config/quickshell/` |
| `$hypr_global` = `/tmp/hypr_global` | ✅ RAM | ✅ from `$hdots/source_run/` | ⚠️ `states/` → `$hdots/states_global/` |
| `$hdots` = `/acc/common/hdots` | ❌ disk | — | canonical persistent tree |

### Boot flow (what actually happens)

1. **Login → Hyprland starts.** Loads `~/.config/hypr/hyprland.lua` — a
   direct symlink into `$hdots/.config/hypr/` (hypr config is PERSISTENT by
   symlink, not a tmpfs copy — edits there are immediately permanent).
2. **`require('env')`** → `~/.config/hypr/env.lua` exports every global via
   `hl.env(...)`: `rconf`, `states`, `states2`, `hypr_bin`,
   `hypr_sources`, `hypr_global`, `rcache`, `hdots`, PATH — all under
   `/tmp` except `hdots`. (`$hdots/launch_bins/env` is the bash twin of
   env.lua for shell contexts.)
3. **`hl.on("hyprland.start")` #1** → `exec_cmd("$hdots/launch_bins/prelaunch_hypr")`
   → sources `prelaunch_body` → `launch_final_func`:
   - `main_logic` **WIPES the tmpfs runtime**: `rm -rf $hypr_bin/*
     $hypr_sources/* /tmp/hypr_misc/misc/* $hypr_sources/quickshell/*`, then
     re-copies from persistent: `$hdots/source_run/*` → `$hypr_global/`,
     `$hdots/hypr_bin/*` → `$hypr_bin/`, `$hdots/sources/*` →
     `$hypr_sources/`, `$hdots/.config/quickshell/*` →
     `$rconf/quickshell`, `$hdots/misc/*` → `/tmp/hypr_misc/misc/`, then
     `$hypr_sources/scripts/*` → `$rconf/scripts/`.
   - mode/channel init → `$states2/sm`; optional splash (gated on
     `$hdots/states/login_splash_notif`).
   - `. $hypr_sources/helpers/hypr_init_logic` — seeds audio state from the
     devices once: `max_vol`/`max_mic` → `$states`, `vol`/`mic`/… → `$states2`.
4. **`hl.on("hyprland.start")` #2** → `exec_cmd("qs -p ~/.config/quickshell")`
   — the shell runs from the tmpfs copy populated in step 3.
5. **Session end** (lock / logout / shutdown / reboot): `power_main session
   <action>` → sources `$hypr_sources/helpers/power_body` → `sync_back` →
   rsync `$states/` → `$hdots/states/`, `$rconf/quickshell/` →
   `$hdots/.config/quickshell/`, `$hypr_global/states/` →
   `$hdots/states_global/` (+ conditional accent cache) and `wait`s before
   the action proceeds.

### Consequences (what it means when you edit)

- **`$hypr_bin` and `$hypr_sources` are BOTH tmpfs.** Editing the live `/tmp`
  copies is pointless — the next boot wipes them. Edit `$hdots/hypr_bin/…` /
  `$hdots/sources/…` (persistent) and the next boot picks it up.
- **hypr config** (`env.lua`, `hyprland.lua`, `configs/*.lua`, keybinds) is the
  ONE tree that is persistent-by-symlink: `~/.config/hypr` →
  `$hdots/.config/hypr`. Edits there are immediately permanent (no sync, no
  boot copy).
- **quickshell** is the deliberate exception: it runs live from tmpfs
  (`~/.config/quickshell` → `/tmp/tw_rconf/quickshell`) so reloads are
  fast, and its edits persist automatically via `sync_back` at session end.
- `prelaunch` (used by `hyprland _noc.lua`, the no-config fallback variant)
  runs the same `launch_final_func` plus `hypr_init`; `prelaunch_hypr` (the
  normal path) runs `launch_final_func` only.

---

# Project goals

1. Make the shell feel like one coherent system instead of separate overlays.
2. Reduce polling and process spawning where possible — idle must sit at ~0%
   CPU with the smallest RAM footprint that still renders the bar.
3. Standardize keyboard navigation, selection, and focus behavior.
4. Add high-value daily-use shell features.
5. Introduce a maintainable architecture for future growth.
6. Avoid external binaries in scripts: prefer pure bash builtins wherever possible.

---

# Engineering principles

## Bash scripts: pure bash first

External tools (`grep`, `cut`, `head`, `tail`, `awk`, `sed`, `find`, `sort`, `tr`, ...) are
separate processes. Every invocation costs a full fork/exec and is measurably slower than
native bash — and inside loops the overhead compounds per iteration. This matters in this
project because several features run shell snippets for every refresh (clipboard thumbs,
wallpaper scan, etc.).

### Rules
- Prefer bash builtins and native constructs: parameter expansion
  (`${var##*/}`, `${var%%\t*}`, `${var//a/b}`), `[[ ]]` tests, `case`, `read -r` with `IFS`,
  arrays, arithmetic, string ops, `while` loops with process substitution.
- Never spawn `dirname`/`basename`/`readlink`/`pwd` to derive a script's own directory —
  that's a bash builtin: `QS_MAIN_DIR=${BASH_SOURCE[0]%/*}` (guard the bare-name case with
  `[[ "$QS_MAIN_DIR" == */* ]] || QS_MAIN_DIR=.`). Runtime paths (`$states`, `$states2`,
  `$rconf`, `$qs_scripts`) are already exported, so a `$PATH`-resolved CLI never
  re-derives a path; it sources siblings by name from `$qs_scripts` / `$hypr_sources`.
  The theme grid moved from `scripts/qs-themes` (deleted 2026-08) INTO `theme_main`
  (`theme_main list|apply <id>`) precisely to share one pure-bash body and drop the
  extra path-spawning entry.
- Reserve external binaries for what bash genuinely cannot do (`cliphist`, `wl-copy`,
  `hyprctl`, dbus tools, `rm`/`mkdir`, ...). The rule is "where possible", not a hard ban.
- Never put a text-munging external tool inside a loop when `read`/expansion can do it.
- Every new `Process` command or `scripts/*.sh` addition must be reviewed against this.

## Examples

| Anti-pattern (external) | Pure bash equivalent |
| --- | --- |
| `cut -f1 \| head -n 60` in a loop | `while IFS=$'\t' read -r i rest; do ((n+=1)); [ "$n" -gt 60 ] && break; ...; done < <(cmd)` |
| `grep -q foo <<< "$s"` | `[[ "$s" == *foo* ]]` |
| `sed 's/foo/bar/g' <<< "$s"` | `s=${s//foo/bar}` |
| `tr '\n' ' '` on a variable | `s=${s//$'\n'/ }` |
| `awk '{print $1}'` | `read -r first rest <<< "$line"` |
| `dir="$(dirname "$(readlink -f "$0")")"` | `dir=${BASH_SOURCE[0]%/*}; [[ "$dir" == */* ]] \|\| dir=.` |
| `basename "$path"` | `${path##*/}` |

---

## Feature lifecycle — add/remove a panel in 3 edits

`PanelRegistry.qml` is the single wiring point for every panel/OSD mode; the
island's component map (`MorphSurface`) and keyboard focus (`ShellRoot`) both
derive from it. This is what keeps the shell cheap to grow.

Adding a panel:
1. Create `components/XxxPanel.qml`.
2. Add `XxxPanel 1.0 XxxPanel.qml` to `components/qmldir`.
3. Register it in `PanelRegistry.qml`: `property Component xxxPanel:
   Component { XxxPanel {} }`, a `"xxx"` entry in `componentMap`, and — for a
   modal overlay — `"xxx"` in `exclusiveModes`.
4. Add `function xxx() { openModal("xxx") }` to `StateController.qml` (+ a
   `ShellIpc.qml` handler only if it needs a keybind).

Removing a panel is the same 3 lines in reverse.

---

# Phase 1 — Foundation cleanup

## 1. Shared design system components
Create reusable primitives so all panels look and behave consistently.

### New files to add
- `quickshell/components/ui/OverlayCard.qml`
- `quickshell/components/ui/IconButton.qml`
- `quickshell/components/ui/ListItem.qml`
- `quickshell/components/ui/ScrollDragger.qml`
- `quickshell/components/ui/SectionTitle.qml`
- `quickshell/components/ui/EmptyState.qml`

### Why
Right now many components define their own rectangle/button/list styling inline.

### Apply to
- `LauncherPanel.qml`
- `WorkspacePanel.qml`
- `MprisPanel.qml`
- `ControlPanel.qml`
- `NotificationDaemon.qml`
- `WallpaperPanel.qml`

---

## 2. Shared keyboard navigation model
Introduce a reusable keyboard/focus pattern for all overlays.

### Goals
- `Esc` closes current overlay
- arrows move selection
- `Enter` activates
- `Tab` cycles controls if needed
- same selected-state visuals everywhere

### New files to add
- `quickshell/components/navigation/ListSelectionModel.qml`
- `quickshell/components/navigation/OverlayFocusScope.qml`

### Apply to
- `LauncherPanel.qml`
- `WorkspacePanel.qml`
- `MprisPanel.qml`
- future clipboard/history panels

---

## 3. Theme system cleanup
Current theme is intentionally small. Keep that, but standardize usage.

### Theme keys (semantic meaning kept)

Surfaces: `bg`, `bg2`, `bg3`, `hover`, `border`

Text: `fg`, `fg2`, `fg3`

Accent: `acc`, `sfg`

Status: `success`, `warning`, `danger`, `info`

Fonts: `fontName`, `iconFont`, `brandingFont`

### Goal
Every component should only use these keys and stop inventing one-off color logic.

### Files to refactor
- `ControlPanel.qml`
- `NotificationDaemon.qml`
- `WallpaperPanel.qml`
- `MprisPanel.qml`
- `WorkspacePanel.qml`
- `LauncherPanel.qml`

---

# Phase 2 — Architecture and state

## 4. Central shell overlay registry
**Done:** `PanelRegistry.qml` (singleton) is the single wiring point for every
panel/OSD mode — `componentMap` (mode→Component) feeds `MorphSurface`'s morph
map and `exclusiveModes` drives `ShellRoot`'s `WlrKeyboardFocus.Exclusive`, so
the mode list lives in exactly one place. `StateController.openModal(name)`
consolidates every modal open (re-opening the same mode toggles back to the
pill); the 13 previously-identical openers are now thin wrappers.

### Remaining
- normalized overlay mode constants (`ShellModes.qml`) if the string set grows
- current / previous overlay metadata if a panel ever needs to know what it replaced

### Files updated
- `PanelRegistry.qml` (new)
- `StateController.qml`
- `MorphSurface.qml`
- `ShellRoot.qml`

---

## 5. Event-driven manager migration
Reduce shell polling over time.

### Done
#### `HyprlandManager.qml`
No longer polls `hyprctl` on an 800ms timer. Now:
- `Quickshell.Hyprland` `rawEvent` socket listener (workspace/window/focus events)
- 60ms-debounced full refresh (one clients+workspaces batch per burst)
- `currentWorkspace` bound to `Hyprland.focusedWorkspace.id` (no process)
- 60s fallback heartbeat only

Snapshot/action IPC went fully in-process via the compiled `QsHypr` module
(0.55 socket protocol — `[flags]/command`, `j` prefix = JSON): the
workspaces/clients JSON snapshots and `dispatch workspace|focuswindow` calls
no longer fork `hyprctl` at all (no process even when something changes).
Same migration applied to `CaptureManager` (activewindow geometry),
`KeyboardCategory` (devices + `switchxkblayout`), `LockScreen` (caps-lock seed).

Network I/O went fully in-process via the compiled `QsNet` module
(`backends/QsNet/`): `WeatherManager` fetches both stages (ip-api geolocation
+ Open-Meteo forecast) through `QNetworkAccessManager`, and `NetworkManager`
reads/toggles wifi and bluetooth over NetworkManager/BlueZ D-Bus — no `curl`,
`nmcli` or `bluetoothctl` processes anywhere in the shell.

#### `MprisManager.qml`
No longer polls `playerctl`. Now uses `Quickshell.Services.Mpris`
(DBus signal-driven players; active player = the one playing, else first).
No timers, no processes.

#### `PowerManager.qml`
No longer polls sysfs every 2s. Now uses `Quickshell.Services.UPower`
(DBus event-driven AC/percentage/state) with `batteryWatts` sampled once/minute
(local 60s `Timer`, same free-running pattern as the clock).

#### `Pill.qml` clock
Ticks once per minute (was 1s): local 60s `Timer` with `triggeredOnStart: true`
— fires at startup, then free-runs every 60s.

#### `NotificationManager.qml`
Already reactive via the notification daemon. Keep moving in that direction.

### Decisions
- **No master `Timer.qml`.** A shared global tick object was evaluated and
  rejected: after this migration only 2-3 cheap 60s timers remain (clock,
  wattage, Hyprland fallback heartbeat), so consolidating them would couple
  unrelated modules to one tick source with no measurable CPU/RAM gain. Timers
  stay local to the module that needs them; timing *values* are static
  declaration-time constants (`Timer.interval` cannot be reassigned from JS in
  Quickshell 0.3.x — it throws `TypeError`).

### Benefits
- fewer processes
- lower CPU usage
- better responsiveness

---

## 6. Shared command/process helpers
Many components spawn processes directly.

### Add helpers for
- running commands
- parsing outputs
- detached execution
- debounced refresh

### New files to add
- `quickshell/components/utils/CommandRunner.qml`
- `quickshell/components/utils/DetachedLauncher.qml`
- `quickshell/components/utils/TextParsers.js`

### Refactor targets
- `LauncherPanel.qml`
- `MprisManager.qml`
- `HyprlandManager.qml`
- `NotificationManager.qml`
- `WallpaperManager.qml`

---

# Phase 3 — High-value features

## 7. Universal command palette
This is the highest-value next feature for the whole shell.

### New files to add
- `quickshell/components/CommandPanel.qml`
- `quickshell/components/CommandPaletteManager.qml`

### It should search
- installed apps
- open windows
- workspaces
- clipboard entries
- wallpapers
- media actions
- power actions
- quick toggles

### Suggested bindings
- `SUPER+Space` → command palette

### Integrations
Use data from:
- `LauncherPanel.qml`
- `HyprlandManager.qml`
- `MprisManager.qml`
- `WallpaperManager.qml`
- future clipboard manager

---

## 8. Notification center
Upgrade from transient notifications to full history.

### New files to add
- `quickshell/components/NotificationPanel.qml`

### Manager upgrades
Extend `NotificationManager.qml` with:
- persistent history list
- unread state
- clear all
- do not disturb
- optional grouping by app

### Suggested bindings
- `SUPER+N` → notification center

---

## 9. Clipboard manager
A major daily-use feature.

### New files to add
- `quickshell/components/ClipboardManager.qml`
- `quickshell/components/ClipboardPanel.qml`

### Features
- text history
- pin/favorite
- search
- keyboard navigation
- click to recopy

### Suggested bindings
- `SUPER+V` → clipboard picker

---

## 10. Better app/window/workspace switching
Build a unified switching story.

### App launcher improvements
File: `LauncherPanel.qml`
- parse `Exec=` as fallback when `gtk-launch` fails
- icon fallback improvements
- app categories / recent apps

### Workspace switcher improvements
File: `WorkspacePanel.qml`
- app icons
- MRU ordering
- wrap-around navigation
- monitor awareness
- urgent indicators

### New feature
- `WindowPanel.qml` for MRU window switching — **built** (current workspace first, then others; Enter/click focuses via `hyprctl dispatch focuswindow`)

### Suggested bindings
- `SUPER+Tab` → workspace switcher
- `ALT+Tab` or `SUPER+Shift+Tab` → window switcher

---

# Phase 4 — Shell completeness

## 11. Audio routing panel
Current audio support is volume-only.

### New files to add
- `quickshell/components/AudioPanel.qml`
- `quickshell/components/AudioManager.qml`

### Features
- output device switcher
- input device switcher
- mic mute
- per-app volumes

### Suggested bindings
- `SUPER+A` → audio panel

---

## 12. Better control center
Refactor `ControlPanel.qml` into a more complete quick settings surface.

### Done
- WiFi + Bluetooth toggles, brightness + volume sliders
- DND toggle, mic mute toggle
- Media player row (prev / play-pause / next + track info)

### Next candidates
- connected SSID + signal display
- bluetooth device list
- power profile switch
- theme toggle(s)
- audio output device quick-switch

---

## 13. Dynamic island behavior
Apply this not only to MPRIS, but to the whole shell experience.

### Use cases
- workspace changes — **built** (`StateController.osdWorkspace` → `OsdToast`)
- charging state changes / AC plug-unplug — **built** (`PowerManager.acStateChanged` → `osdPower`)
- media changes
- mic mute/unmute
- screenshot saved
- clipboard copied

### Files involved
- `Pill.qml`
- `MorphSurface.qml`
- `StateController.qml`
- all manager files that emit events

---

## 14. Session / power UX
Improve session management.

### Add
- lock
- suspend
- hibernate
- reboot
- shutdown
- logout
- confirmation overlays if desired

### Possible files
- `quickshell/components/SessionPanel.qml`
- `quickshell/components/SessionManager.qml`

---

## 14b. QML lock screen (DONE)

Decision: build a full-screen QML lock panel inside the shell (hyprlock is
configured but NOT installed on this machine). Real session lock via the
Wayland `ext-session-lock` protocol — NOT a layer-shell overlay.

### Architecture
- `components/LockScreen.qml` — new. `WlSessionLock` whose `locked` binds to
  `StateController.locked`. Its surface component (`WlSessionLockSurface`,
  auto-created per screen) renders: blurred+dimmed current wallpaper,
  clock/date, user, password field.
- Auth: `Quickshell.Services.Pam` `PamContext`, `config: "common-auth"`,
  `user` set explicitly. IMPORTANT (fixed 2026-08): this machine uses the
  pam-config (Debian-style) layout — `/etc/pam.d/system-auth` does NOT exist,
  so the old `config: "system-auth"` fell back to the deny-by-default `other`
  stack and rejected the CORRECT password, locking the user out. `common-auth`
  exists and is plain `auth required pam_unix.so try_first_pass` (shadow auth,
  same stack as login). A temporary "bypass" button on the lock surface
  unlocks without a password — remove it once auth is confirmed live.
- Flow: Enter -> if `pam.active && pam.responseRequired` -> `pam.respond(pw)`,
  else `pam.start()`; a `Connections` on `responseRequiredChanged` responds
  with the pending password (handles pam_unix internal retries). On
  `completed(PamResult.Success)` -> `StateController.unlock()`; else show
  `pam.message` (or "Wrong password") and clear the field.
- Unlock only via correct PAM password (no IPC unlock — security). Esc clears
  the field, click focuses it.
- One `PamContext` lives on the lock root (a single `WlSessionLock` serves all
  screens); each screen's `WlSessionLockSurface` only renders. A `resetToken`
  on the root clears + refocuses every surface's field on failure/relock so
  no stale password survives. Wallpaper path is resolved at lock time with a
  pure-bash `Process` (no polling, no tick while unlocked).

### Trigger wiring (done)
- `StateController.qml`: `property bool locked` + `lock()` (also `osdMode = "pill"`)
  + `unlock()`.
- `ShellIpc.qml`: `lock()` -> `StateController.lock()` (IPC `session lock`).
- `scripts/power_body`: `session lock` -> `quickshell ipc call session lock`,
  fall back to `hyprlock` if IPC fails.
- `~/.config/hypr/hypridle.conf`: left untouched (out of quickshell scope) —
  the CLI entry is `power_main session lock`; point hypridle's `lock_cmd`/idle
  listener at it on the user's machine when desired.
- `shell.qml`: instantiate `LockScreen` (needs `components/qmldir` entry).

### Current wallpaper (verified)
- `/tmp/tw_rconf/states2/s` -> channel letter (currently `d`),
  `/tmp/tw_rconf/states/current_wall_<ch>` -> image path. Fallback: parse
  `$current_wall=` from `~/.config/hypr/hyprlock/current_wall.conf`.
- Wallpaper at `/home/tw/Wallpapers/...jpg`.

### Testing notes / hazards
- WARNING: if quickshell reloads/exits while `WlSessionLock.locked` is true,
  the compositor leaves the screen locked solid (inoperable). Never hot-reload
  while locked; recover by restarting the Hyprland session.
- No wtype/ydotool on the box — keyboard input testing needs the real user.
- PowerPanel "Lock" already calls `power_main session lock` — no change needed.

---

# Phase 5 — Polishing and maintainability

## 15. Settings system
Introduce user-configurable behavior.

### Done
- `SettingsManager.qml` (singleton): `$states/settings.json`, JSON config with
  defaults merged per section, live apply on change, file watched so hand-edits
  reload automatically, debounced writes.
- `SettingsPanel.qml` full settings app: category nav (Appearance / Notifications /
  Wallpaper / Shell / Keyboard / Keybinds / About), keyboard navigation, live controls.
- The rofi-derived **Dotfiles** settings UI was REMOVED (schema engine, `DotfilesCategory`,
  `SettingSection`, `ChannelModeSwitcher`, `settings-schema.json`, and the
  `Popups/Capture/Clipboard/Restore` categories). A native settings app is being coded in
  its place; the rofi `master-menu` + submenus in `hypr_bin` stay untouched for now.
- Configurable today: accent color (swatches), font family, icon theme,
  DND, notification timeout, island auto-hide delay,
  **floating bar** (pill floats 25px below the top vs. the same bar sliding
  up flush against the top into a flatter, Mac-Dynamic-Island bar shape) and
  **expanded bar**
  (always-expanded vs. collapsed 135px clock-only pill that expands on
  hover over the whole bar rect) —
  `ToggleSwitch.qml` is the shared iOS-style switch.
- Live wiring: `SettingsManager.config.shell.floating` / `shell.expandedBar`
  → `MorphSurface.stuckTop` (animated `anchors.topMargin` 25→0) → `Pill.qml`
  (`stuckTop` corner morph 19→10, `hoverCollapse` + `externalHover` via
  `ShellRoot.qml`'s `barHover` MouseArea).
  The bar is never full-screen: always 135px collapsed / 520px expanded,
  centered, capsule pill when floating, flatter bar corners when stuck to
  the top. The pill renders outside the OpacityMask layer and is never
  scaled, so the clock text never shimmers while morphing. No springs — all
  OutCubic NumberAnimations.

### New files
- `quickshell/components/SettingsManager.qml`
- `$states/settings.json` (generated on first change; defaults live in the singleton)

---

## 16. Logging and diagnostics helpers
Improve maintainability.

### New files to add
- `quickshell/components/utils/Logger.js`
- `quickshell/components/utils/DebugOverlay.qml`

### Benefits
- easier debugging of command failures
- easier diagnosis of empty states
- better visibility into manager refreshes

---

## 17. Performance pass
After more features are in place, do a cleanup pass.

### Audit
- idle CPU above ~0% — a stray timer/poller, a process spawned on a schedule,
  or an always-live panel component (panels must stay lazy via `PanelRegistry`
  + `MorphContainer`'s Loader so closed panels cost 0 RAM)
- repeated polling
- multiple repeated shell commands
- external binary calls inside bash loops (grep/cut/head/awk/sed...) — must be pure bash
- expensive scans on every open
- duplicate parsing logic
- unnecessary always-live components

### Targets
- `LauncherPanel.qml`
- `HyprlandManager.qml`
- `MprisManager.qml`
- `NotificationManager.qml`
- `WallpaperManager.qml`

---

# Community research — tray, weather, quick actions (2026-08)

Decisions below are lifted from the popular Quickshell Hyprland ricers, not
guessed. Sources (starred/active configs, checked against the installed
Quickshell 0.3.0 API):

- **end-4/dots-hyprland** (13k★) — the reference Quickshell shell: tray,
  util buttons, weather service, overview. Weather rewritten in
  `PR #3070` to Open-Meteo + ip-api.com geo (wttr.in retired as flaky).
- **sonroyaalmerol/snry-shell** (end-4 fork, Go daemon backend) — tray/conflict
  killer, weather via wttr.in + geoclue.
- **hireri/israshell** — quick settings: NetworkManager, Blueman, Pipewire
  volume, hyprsunset night light, caffeine toggle.
- **yurihikari/ml4w-lightcrimson-dotfiles** — Open-Meteo live weather in the
  calendar popup.
- **jimallen/quickshell** — notch dropdowns, weather with offline cache.
- **anomshell**, **Ricelin** — two battle-tested tray + native-menu card
  implementations.

## Tray — Quickshell.Services.SystemTray (build this)

The compositor-agnostic way is the **StatusNotifierItem** service Quickshell
already ships (`Quickshell.Services.SystemTray`), NOT the Wayland
`zwlr_foreign_toplevel` icon hack. Referencing the `SystemTray` singleton starts
tracking automatically.

API (0.3.0, from quickshell `src/services/status_notifier/qml.hpp`):
- `SystemTray.items` — list of `SystemTrayItem`; each has `icon` (usable
  directly as an Image/IconImage source via Quickshell's `icon` provider),
  `activate()`, `secondaryActivate()`, `scroll(delta, horizontal)`,
  `hasMenu`, `onlyMenu`, `menu` (`QsMenuHandle`), `tooltipTitle`, `title`, `id`.
- Menus: `QsMenuOpener { menu: item.menu }` then `opener.children` gives
  `DBusMenuItem`s (`text`, `isSeparator`, `enabled`, `checkState`,
  `buttonType`, `hasChildren`, `triggered()`). Nested `QsMenuOpener` for
  submenus.
- `SystemTrayItem.display(parentWindow, relativeX, relativeY)` can open a
  **platform menu**; ricers instead render their own card for theming.

Adopted pattern (Ricelin-style, the most robust):
- Chip grid in the pill `statusRow`; each item 24px, hover = accent tint.
- Left-click `activate()` (or menu if `onlyMenu`), right-click = native menu
  card, wheel = `scroll()`.
- Menu card = own `PanelWindow` (Overlay layer, `WlrKeyboardFocus.Exclusive`,
  Esc/click-outside to close), entries with checkbox/radio state + submenu
  chevron. Use `QsMenuOpener` for the tree.

### Built
- `components/Tray.qml` (registered in `components/qmldir`, rendered in
  `Pill.qml`'s `statusRow`): chips 24px/3px spacing, hover fill
  `Theme.hover`, tooltip below on hover, Nerd Font glyph fallback
  (`glyphFor`, same map style as `WindowPanel`). Menu card = full-screen
  `PanelWindow` (Overlay layer, `WlrKeyboardFocus.Exclusive`, Esc +
  click-away close) with checkbox/radio/separator rows, indented submenu
  level and chevron via `QsMenuOpener`. Idle cost is zero — no timers, the
  `SystemTray` DBus model is signal-driven.
- Pill width is now content-aware: `Pill.qml` grows `expandedWidth`
  (`Math.max(520, leftClusterWidth + 18 + statusRow.implicitWidth + 24)`)
  instead of a fixed 520px, so tray chips, battery and watts always fit
  with a 24px right margin (520 stays the floor when the bar is empty).
  `statusRow` is anchored right after the clock/date cluster instead of
  being centered via the (removed) `hello` spacer. This also fixed the
  pre-existing overflow when battery + watts show.

## Weather — Open-Meteo + ip-api.com geo (build this)

end-4 `PR #3070` beats wttr.in (unstable, rate-limited): two-stage fetch with
zero API key and zero config:
1. `curl -s ip-api.com/json/` → lat/lon + city (fallback: configured city
   string in $states/settings.json).
2. `curl -s "https://api.open-meteo.com/v1/forecast?latitude=..&longitude=..&current=temperature_2m,weather_code,wind_speed_10m,relative_humidity_2m"` → JSON.
3. `wmoToWwo()` mapping of Open-Meteo WMO codes → weather icon glyph.
4. `Process` + `StdioCollector` (QML), 10-min repeat timer, `triggeredOnStart`,
   manual right-click refresh. No polling below 10 min.

Same shape as the existing `HyprlandManager`/`MprisManager` (event-driven,
`Process` for device I/O), consistent with the ROADMAP engineering rules.

## Quick actions / quick settings (upgrade ControlPanel)

Ricers converge on a "quick settings" surface (israshell / end-4 UtilButtons):
- **Toggles** (circular accent buttons): mic mute (`wpctl set-mute
  @DEFAULT_SOURCE@ toggle`), dark mode (theme switch), DND (existing
  `NotificationManager.toggleDnd`), performance profile
  (`powerprofilesctl` / `PowerProfiles`), night light (hyprsunset).
- **Sliders**: Pipewire volume + brightness (already present in this repo).
- **Rows**: connected SSID + signal, bluetooth device list, audio output
  quick-switch, power profile, theme.

This repo already has the ControlPanel quick-toggles row; the job is to bring
it to the ricer standard (accent-tinted active state, glyph toggles, power
profile + night light + theme as extra toggles) and theme the new pieces.

# 2026-08 — Follow-up features & decisions

## Shell silhouette: pill when floating, flat bar when stuck (DONE)

The deep-U bottom curve is gone. Both `Pill.qml` and `MorphContainer.qml`
now derive `topR`/`botR` from `stuckTop` (`SettingsManager.config.shell.floating`):

- **floating = true**: the island is a true capsule — corner radius is half
the height on every corner (`height / 2`). The bar is a pill.
- **floating = false (stuck)**: the island sits flush at the screen top as a
flat bar — small 8px rounded top corners, straight bottom edge (`botR: 0`).

Panels follow the same rule with a 28px radius cap while floating so a tall
panel (e.g. command palette) stays a rounded card instead of ballooning into
an ellipse. `pillMask` / `maskShape` mirror the silhouette so content clipping
always matches the drawn shape.

## Per-app volume mixer (DONE)

`AudioPanel` gained a third tab — **Streams** — listing running sink-inputs
(app, volume %, muted). `audio_main audio streams|stream-vol ID PCT|stream-mute ID`
(pure-bash parse of `pactl list sink-inputs`).
Controls: `←/→` nudge the selected stream ±10%, `Enter`/`M` toggles its mute,
wheel scroll on a row adjusts too. `AudioManager` fetches streams on demand
with the sinks/sources batch — still zero idle polling.

## Keyboard layout switcher — in Settings, not a chip (DONE)

New **Keyboard** settings category (`modules/settings/KeyboardCategory.qml`):
reads `hyprctl devices -j` once on open (main keyboard's `active_keymap`),
Previous/Next buttons run `hyprctl switchxkblayout <device> prev|next` and
re-read. Deliberately no pill chip — layout switching lives in Settings.

## Quick-settings trio in the control center (DONE)

`ControlPanel` gained a second quick-toggle row (accent-tinted tiles, same
pattern as row 1):

- **Eye care** — hyprsunset on/off. State read from `$states/hyprsunset_state`;
toggle goes through `SystemSettingsManager.apply("hyprsunset_main", "toggle")`
so it uses the dotfiles' own daemon handling.
- **Caffeine** — hypridle inhibitor. `caffeine_main [status|on|off|toggle]`
swaps hypridle to a listener-free `hypridle-caffeine.conf` (no timeouts, so
nothing can fire) and back on decaf; state in `$states/caffeine`.
- **Power profile** — `powerprofilesctl` cycle (power-saver → balanced →
performance), tinted by profile. The tile hides itself when
power-profiles-daemon is not installed.

## Screenshot / screen recording (DONE)

`services/CaptureManager.qml` (singleton) — long-lived `Process`es for grim
(shotFull/region/window), slurp (region pick), wf-recorder (region recording,
`.mkv` container — utvideo does not support `.mp4`), and `pkill -INT` to
finalize. IPC: `capture shot_full|shot_region|shot_window|record_toggle|record_stop|capture_settings`.
Config in the `capture` section of `$states/settings.json` (screenshotDir,
recordDir, format png|jpg, copyScreenshot, fps). Settings UI:
`modules/settings/CaptureCategory.qml`. Recording filename `rec_<ts>.mkv`,
screenshots `shot_<ts>.<fmt>` in `~/Pictures/Screenshots` (defaults).

## Theme selector grid (DONE)

`modules/themes/ThemesPanel.qml` — scheme cards (pretty name, base + extended
swatches, active ring from `$states/scheme_<cm>`), panel registered as `"themes"` in
`PanelRegistry` and as the Settings "Themes" category. `theme_main list` /
`theme_main apply <id>` (pure bash, mirrors `theme_menu`); apply writes `scheme_<cm>` ONLY if
it differs (re-selecting the active scheme is a no-op), resets
`custom_acc_*`/`acc_from_wall_*`/`acc_from_hex_*`, then runs the dotfiles'
`theme_main` for the real apply — a mode flip fires `theme_main dark|light`, a
same-mode change sets `$states2/acc_changed=1` + `theme_main restore`. The
bar re-syncs through the `theme restore` IPC hook, which makes `Theme` a dumb
reader: it re-reads the whole `$states2` palette — `fg fg2 fg3 bg bg2 bg3 hover
border acc sfg success warning danger info` + `mode` (written by `theme_body`
from the master `$states/themes` file) — and applies, never computes a palette.
Cards show the theme's pretty name (`default_dark` → "Default Dark"), not a
dark/light mode label. Dual themes (dark+light in one master line) apply to BOTH
mode slots (`scheme_<ch>d` + `scheme_<ch>l`), so toggling light/dark keeps the
same scheme and each mode reads its own half of the dual line. `Theme.sfg` is mutable and defaults to
`#e5e9f0`; sfg comes from the master `$states/themes` file (column 5 /
`sfg_d`+`sfg_l` on dual lines) — the scheme's own `sfg_mode`/`sfg_d`/`sfg_l`,
never contrast-computed in QML. IPC: `panel themes|panel theme_settings`.

## Accent changes: idempotent apply (DONE)

Every accent change (custom acc ↔ acc-from-wall ↔ defined hex ↔ theme default)
is a single command: write the source flags to `$states`, set
`$states2/acc_changed=1`, then run `theme_main restore`. The apply is a NO-OP
when the chosen source equals the one already applied — if the user picks the
same option as the active one, skip the flag writes, skip `acc_changed`, skip
`theme_main restore` entirely. `acc_changed` is what makes the dotfiles
hot-reload the new accent (GTK/thunar re-apply via the alt-theme swap trick in
`theme_reapply_func`), so it must only ever be set when the accent actually
changes. The `theme restore` IPC hook (`quickshell ipc call theme restore`,
fired from `theme_body_run` at the end of every theme_body pass) re-reads the
palette into the shell.


## Next up — user-requested (2026-08-16)

### 1. Network speed pinning (no text dance)
Pin upload/download speed to the **leftmost position** in the expanded bar so
adding/removing other chips (tray, battery, watts) does not shift the speed
text. The speed cluster gets a fixed left anchor; everything else flows right
of it.

### 2. Music visualizer — expanded bar
Audio-reactive visualizer bar inside the expanded pill when music is playing.
Driven by MPRIS metadata (song active = show, no song = hide). Uses a
lightweight amplitude estimation from PipeWire/PulseAudio peak levels
(exposed via `wpctl` or a small compiled helper — no heavy FFT dependency).
Renders as a row of thin accent-tinted bars between the track info and the
status chips.

### 3. Music visualizer — collapsed bar
Mini visualizer in the collapsed 135px pill. Same peak-level source, rendered
as 3-5 tiny bars next to the MPRIS chip (or the clock if no MPRIS chip is
visible). Fades in/out with music state.

### 4. Proper Settings app — pill toggles + draggable items
Rebuild `SettingsPanel.qml` as a real settings **app**, not a control center:

- **Pill toggles** for boolean options (expand-on-hover, floating bar,
  always-expanded, DND, etc.) — same `ToggleSwitch` style but laid out as
  labeled pill rows.
- **Draggable pill items** in collapsed mode: the user can reorder which
  chips appear in the collapsed bar by dragging them in a settings sub-view.
  Order persisted in `$states/settings.json` under `shell.pillOrder`.
- Categories: Appearance, Shell Behavior, Pill Order, Notifications,
  Wallpaper, Keyboard, Capture, Themes, About.

### 5. UI font chooser + font size
New **Appearance** settings sub-category:
- **Font family picker**: lists monospace + sans families from fontconfig,
  live preview in the pill.
- **Font size slider**: 8–24px range, live apply to `Theme.fontName` size
  override.
- **Icon font picker**: choose the Nerd Font variant.
- Persisted in `$states/settings.json` under `appearance.font*`.

---

## Not planned for the near future

- **System monitor (CPU / RAM / temp)** — explicitly deferred by the owner.
Not on the radar for ~1 year. Do not build a sysmon chip/module and do not
suggest / ask about it before then; this file is the record of that decision.

## Fixes

- **Theme apply killed quickshell (root cause, fixed 2026-08).** hdots'
  `qpid` helper's `check_process_pid` ignored its `$1` argument and always
  matched `quickshell`/`qs` — so `kill -SIGUSR1 $(. $hypr_global/qpid kitty)`
  in `theme_body_run` (and every `thunar`/`hyprsunset`/`hypridle`/`wl-paste`
  restart guard) targeted the quickshell PID. SIGUSR1 terminates quickshell
  (default disposition), so any theme apply killed the shell ~1s in; the log
  ended cleanly at `configreloaded` with no crash trace. `qpid` now honors
  `$1` (keeping the `qs` Splash.qml exclusion). Fixed in the persistent
  `$hdots/source_run/qpid` and the live tmpfs `/tmp/hypr_global/qpid` — keep
  both in sync (boot re-copies the former to the latter).
- Removed the duplicate `Tray` instance in `Pill.qml`'s `statusRow` (tray
icons were rendering twice).
- **Toast countdown bar removed** (`ToastDaemon.qml`): the top-right corner
notifications no longer show the auto-dismiss progress bar, and the
250ms-per-toast repeating ticker is gone. Dismissal is one single-shot
`Timer` (interval bound via `tickMs` — never assigned from JS), so a
visible toast burns zero CPU. Hovering pauses and **resumes from the
remaining time** on leave (deadline/holdMs bookkeeping), not a full
restart.
- **In-panel OSD chip** (`AudioPanel.qml`): adjusting a per-app stream's
volume/mute flashes a small accent chip inside the audio panel (the island
toast would collapse the open modal, so the feedback stays local).
- **Smoothed corner morphs** (`Pill.qml` + `MorphContainer.qml`): the corner
radius (`topR`/`botR`) now animates via `Behavior` (150ms OutCubic) so the
capsule-to-panel radius jump morphs instead of snapping.
- **Settings deep-link** (`ShellIpc.qml` `keyboard_settings` IPC): opens the
settings app directly on the Keyboard category via
`StateController.settingsCategory` (consumed+cleared by `SettingsPanel`
on creation).- The `*_main` CLIs fall back to the same dotfiles paths as
  `SystemSettingsManager` (`$rconf`/`$states`/…), so they work from any
  context — terminal, quickshell Process, or cron.

---

# Recommended implementation order

## Immediate next steps
1. Add shared UI primitives (`ui/` components).
2. Add shared navigation helpers.
3. Build `CommandPanel.qml`.
4. Build `ClipboardManager.qml` + `ClipboardPanel.qml`.
5. Build `NotificationPanel.qml`.

## Then
6. Refactor launcher and workspace switcher onto shared primitives.
7. Improve MPRIS into full dynamic-island behavior.
8. ~~Migrate Hyprland and MPRIS managers toward event-driven updates~~ **done**
   (Hyprland rawEvent + Mpris/UPower services; clock + wattage once per minute).
9. Add audio routing panel.
10. Add settings system.

---

# Suggested keybind map

- `SUPER+Space` → command palette
- `SUPER+Tab` → workspace switcher
- `SUPER+Shift+Tab` → window switcher
- `SUPER+V` → clipboard picker
- `SUPER+N` → notification center
- `SUPER+M` → MPRIS island
- `SUPER+A` → audio panel
- `SUPER+,` → control center / settings

---

# Recommended first feature to implement next

## Universal command palette
This is the best next whole-project feature because it ties together:
- app launching
- open windows
- workspace actions
- media controls
- power actions
- future clipboard and notification actions

It will also force a cleaner architecture for shared item rendering, keyboard navigation, and action dispatch.
