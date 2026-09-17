# PROGRESS — hdots lua port (theming + scripts)

Tracking markdown for the ongoing Lua port of the hdots config at
`/acc/common/hdots` (aka `~/.config/hdots`). Continue from here.

---

## Objective

Convert the bash helpers/scripts of the hdots project to **pure Lua**
(LuaJIT) so the runtime uses no external file-IO binaries. `luajit` is the
engine; system actions stay as-is.

---

## Hard rules (user directives — non-negotiable)

- **No external file-IO binaries**: `cat`, `printf`, `stat`, `sleep`, `sed`,
  `awk`, `tee` are banned in ported code. Use pure-Lua IO instead.
- **No trailing `\n` on writes.**
- **No newline stripping on reads** — `tonumber("2\n")` handles it.
- **System actions remain**: `hyprctl`, `notify-send`, `kill`, `umount`,
  `sudo`, `zen-shell` (spawn of own luajit scripts is fine).
- Ported files are **self-contained**: `SCRIPT_DIR = arg[0]:match(...)`,
  inline FFI cdefs; scripts_global runs as root (no env, only `$SCRIPT_DIR`);
  user scripts get `hypr_conf/states2/hypr_sources/hypr_global/hypr_base/HOME`.
- Bash backups live in `/tmp/opencode/bash_version_scripts/`.

---

## Work state

### DONE — `scripts/` + `scripts_global/` (11/11 files, `luajit -b` OK)

Ported in place at `/acc/common/hdots/scripts/` + `/acc/common/hdots/scripts_global/`:

- `scripts/`: `run_inside_loop`, `user_loop`, `watch_dog`
- `scripts_global/`: `loop`, `before_loop`, `during_loop`, `main_body`,
  `on_udev_trigger`, `set_cpu_freqs`, `qpid`, `qpid_org`

Live-verified: `qpid`/`qpid_org` PID matching, `main_body` plan parsing
(`power_plan_ac`, `max_ac_freq_mhz`, ...), `set_cpu_freqs` gate
(state=0 skips sysfs), `on_udev_trigger run` loads `main_body` and exits 0.

### DONE — `sources/helpers/utils` (today)

- Added FFI `getpid()` cdef + function.
- `bg_delay(secs, cmd)` now pure Lua: spawns `luajit -e` that nanosleeps via
  FFI then runs `cmd` (no external `sleep`).
- Removed broken `spawn_lua` (was `spawn("(" .. tostring(fn) .. ")")` — invalid).
- Verified: `luajit -b` OK, `getpid()` returns pid.

### DONE — `/home/tw/bin/hypr` (launcher, today)

- Bash `~/bin/hypr` (printf random splash → `start-hyprland`) ported to LuaJIT
  at `hypr_lua/bin/hypr` and installed over `/home/tw/bin/hypr` (keeps the
  `hp` symlink working).
- Self-contained: dofiles the ported `sources/hyprland_branding`, inline FFI
  `getpid()` seeds `math.randomseed` (pure Lua, no `$RANDOM`/bash).
- `start-hyprland` stays a system command (allowed).
- Backup: `/tmp/opencode/bash_version_scripts/hypr`. Verified dofile + random
  pick (`#banners=10`); NOT exec'd (would start a real session).

### Next up

- Live-verify through the shell: `theme_main dark|light|toggle|restore`
  (apply paths spawn the ported `gen_gtk_css`, `center_pop`, debounce child,
  and the theme pipeline strictly against the hypr_lua tree).
- Revisit `gen_gtk_css`/`center_pop` spawns once live if any state-file read
  assumptions differ in the real `$hypr_sources` env.

### DONE — `sources/helpers/theme_main_body` (today)

1. Added missing `dofile(hypr_sources .. "/helpers/theme_body")` at top
   (bash does `. $hypr_sources/helpers/theme_body`).
2. Added standalone bootstrap (`if _G.lua_helpers == nil`) so
   `luajit <dir>/theme_main_body <cmd>` resolves its own paths from `arg[0]` +
   loads utils — used by restore/reapply spawns.
3. `wait_for_theme_proc_to_stop_func`: `run("sleep 1")` → `sleep(1)`.
4. `theme_reapply_func_accent_hot_reload`:
   - both `run("sleep 0.5")` → `sleep(0.5)`.
   - fixed **alt_flag inversion** (bash: alt=1 → `hypr-theme-dark`, else
     `*-alt`; port had it flipped).
   - added the missing `[[ -z $no_acc_reload ]]` gate (bash line 56) while
     keeping the unconditional flag reset.
5. `debounce_func` rewritten pure-Lua: writes `getpid()` to
   `states2/theme_switch_pid`, spawns `luajit -e` that does `sleep(0.5)` (FFI),
   pid re-check, `m == m_dummy` early-out, `wait_for_theme_proc_to_stop_func`,
   then `theme_apply_light_func`/`_dark`. No more `cat`/`[`/`sleep` bash string.
6. `theme_switch_func` now just flips `m_dummy` and calls `debounce_func()`
   (was `bg_delay(0, debounce_func())` — wrong signature; that evaluated
   `debounce_func()` at call time and passed its return value).
7. `theme_main_restore_func`: `spawn("sleep 0.1 && luajit … reapply")` →
   `bg_delay(0.1, "luajit <rot> reapply")`. Also fixed `shq` bug that had
   quoted `"path/theme_main_body reapply"` as ONE argument (never executed).
8. Bottom standalone dispatch: `arg[0]` basename `theme_main_body` + `reapply`
   → runs `theme_reapply_func_accent_hot_reload()`.

### DONE — `sources/helpers/theme_body` (today)

- `sfg_ec_d()`/`sfg_ec_l()` filled: `sfg_ec = sfg_l` / `sfg_ec = sfg_d`.
- Fixed literal path bug: `read_line("states/scheme_" .. m)` →
  `read_line(states .. "/scheme_" .. (sm or m))` (bash uses `scheme_${sm}`,
  files are `scheme_nd/nl/dd/ll`; plain `scheme_l` does not exist).
- `os.getpid()` → `getpid()` (theme_run_pid write).
- `show_mode_switch`: `bg("sleep 0.4 && …")` → `bg_delay(0.4, …)` and now
  passes `icon`/`text` args through to `center_pop` (bash passed `$1 $2`).

### DONE — standalone helper bootstraps (today)

- `gen_gtk_css`: spawned as own process by `init_global_vars`; added utils
  bootstrap + re-read of `gtk_acc/fg/bg_gtk/m/m_string/acc_ec/sfg_ec` from
  `states2/` (previously crashed — those globals lived only in the parent).
- `center_pop`: added utils bootstrap before `fetch_color_scheme`.

### DONE — integrity checks (today)

- Full `luajit -b` sweep: all 72 `sources/helpers/*` compile.
- Sandbox smoke test (`/tmp/tw_smoke*`, env pointed at hypr_lua tree):
  - `scripts/theme_main status` → `dark` (from crafted settings.json).
  - `theme_switch_func` end-to-end: `m_dummy` flips, `theme_switch_pid`
    written with self pid, spawned debounce child early-returns after its
    `sleep(0.5)`+re-checks with **zero apply side effects** (no new state2
    files, no spawns).
  - `luajit <helpers>/theme_main_body noop` standalone load path OK.

---

## Verified bash reference

- `theme_main_body` bash: sources `theme_body` at top; debounce writes
  `$$ > $states2/theme_switch_pid`, backgrounds `{ sleep 0.5; [[ pid == $$ ]] &&
  { m, final_dummy; [[ m == final ]] && exit; wait_for_theme_proc_to_stop_func;
  final==l ? theme_apply_light_func & : theme_apply_dark_func & } } &`;
  `theme_switch_func` flips `m_dummy` then `debounce_func &; exit 0`.
- `theme_main_restore_func` bash: `restore-${m_string}-function` then
  `theme_reapply_func_accent_hot_reload &`.
- `theme_body_run` pattern (pure-fied): dofiles utils, waits on `*_done`
  flags with `sleep(0.1)`, `hyprctl reload`, `kill -SIGUSR1 kitty`,
  backgrounds `luajit -e 'dofile(utils); dofile(remove_theme_lock_file)'`.

---

## Key files

- `/acc/common/hdots/scripts/`, `/acc/common/hdots/scripts_global/` — ported Lua scripts (11, executable)
- `/home/tw/.config/hdots/hypr_lua/sources/helpers/theme_main_body` — IN PROGRESS
- `/home/tw/.config/hdots/hypr_lua/sources/helpers/theme_body` — PENDING fixes
- `/home/tw/.config/hdots/hypr_lua/sources/helpers/utils` — DONE
- `/home/tw/.config/hdots/hypr_lua/sources/sr/theme_main_restore` — dofile pattern reference
- `/home/tw/.config/hdots/hypr_lua/scripts/theme_main` — CLI entry: dofile utils + theme_main_body + `theme_cmd(unpack(arg))`
- `/tmp/opencode/bash_version_scripts/` — bash backups