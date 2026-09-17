# AGENTS.md — the rules, short version

Not a git repo — edit carefully, no rollback.

## Paths — that's it
- **QML path**: `~/.config/quickshell/` — THE final copy (symlink →
  `/acc/common/hdots/.config/quickshell`). All QML, config, and NEW scripts
  live here. One write, done. No tmpfs, no mirroring, no sync_back, no
  verification, no other copies.
- **Script path**: `scripts/` in this folder — every bash CLI (`*_main` +
  `*_body`) lives here, and only here.
- **Configs stay in `$states`**: runtime settings (`settings.json`, `themes`
  master, `scheme_*`, …) live in `$states`, synced to `$hdots/states/` at
  session end by `sync_back`. `$states2` is scratch only — NOT persisted.
- Nothing else. No alternate paths, no tmpfs trees, no mirroring.

## Write rules
- When asked to write something — **JUST WRITE IT**. No verification, no
  confirmation questions, no change-log. Write it in this folder, done.

## Technical must-knows
- **I/O via compiled modules only — never shell out**: `QsIo` (file
  read/write), `QsHypr` (Hyprland IPC, never fork `hyprctl`), `QsNet`
  (http + NetworkManager/Bluetooth D-Bus, never fork `curl`/`nmcli`/
  `bluetoothctl`). Import versioned: `import QsIo 1.0` — relative imports
  fail for `.so` modules.
- Every QML module needs its **`qmldir`** — new types don't resolve without it.
- Singletons (`pragma Singleton`) can't be instantiated — touch them via a
  property binding.
- Panels never own long-running `Process` objects — they die with the panel.
  One-shot post-close commands go on a manager singleton.
- **No timers/polling — event-driven only.** Shell idles at ~0% CPU, no
  process spawned on a timer. A new ticker/poller is a regression.
- Pure bash first: no `grep`/`cut`/`sed`/`awk` forks when builtins do it.
- New panel = `modules/<feature>/XxxPanel.qml` + `qmldir` +
  `core/PanelRegistry.qml` (`componentMap`, `exclusiveModes` for modals) +
  `StateController.openModal()` + optional `ShellIpc.qml` handler.
- Key/nav style: Esc close, Up/Down move, Enter activate, Delete remove.
  Visual style: `Theme.acc` fill, `Theme.sfg` selected text, Nerd Font icons.
- Sandbox validation: `QT_QPA_PLATFORM=offscreen quickshell -p <probe.qml>`.
