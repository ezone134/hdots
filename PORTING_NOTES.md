# hdots — Lua source of truth

> LAST UPDATED: 2026-09-20

## ✅ SOURCE OF TRUTH: LUA IS CANONICAL (2026-09-19)

The Lua port under `$hdots/sources/` (the persist tree `/acc/common/hdots`) is
THE living source of truth for hdots. All helpers, `bin/*`, `sr/*`, `daemon/*`,
`theme_body`, `fetch_color_scheme`, `hypr_init_logic`, `prelaunch_body`, and the
CLIs (`brightness_main`, `audio_main`, `theme_main`, …) ARE Lua.

- **`legacy/` is a FROZEN bash reference only.** Read it for intent/history;
  never port from it bit-by-bit and never edit it. The Lua port intentionally
  diverges (pure-Lua FFI filesystem helpers, `ff`/`tf`/`append` I/O, env-var
  bootstraps, flag-driven themes, no variable fallbacks).
- **No new bash.** New code is Lua with the current conventions (see RULES
  below). Old paths that exist in `legacy/` but not in `sources/` are gone on
  purpose.
- When this file and `legacy/` disagree, THIS FILE + the Lua tree win.

## ⚠️ INCIDENT LOG (read this first after reboot)

### 2026-09-17 — I DELETED THE CANONICAL TREE (MY FAULT)

While verifying a new `rm_rf` guard in `sources/utils`, I loaded the **stale
tmpfs copy** of `utils` (synced BEFORE the guard was added) and ran a test that
called `rm_rf("/")`, `rm_rf("/home/tw")`, and `rm_rf(hdots)`.

Because the loaded code was the OLD unprotected version, it **actually deleted**:

- `/acc/common/hdots` — the canonical persist tree (nearly all of it)
- `/home` → `/tmp/home-tmp` (config root)
- `/tmp/tw_rconf` (runtime tree) and `/tmp/user-tw` (workspace)

**Lesson for future sessions: NEVER test code loaded from a stale copy, and never
call `rm_rf` with `/` or home paths even in a test. `rm -rf` should always be
used via the guarded `rm_rf` ONLY on explicit real targets, never on system roots.**

User restored the tree from the git backup at commit
`7af1330 Update zypp 2026-09-17 12:42:47` (in `/acc/common/hdots/.git`).

**State after restore:** the tree is back to the LEGACY BASH version. All Lua
porting done before the incident is LOST and must be REDONE. The `.md` you are
reading now is the master record so the port can be rebuilt after any reboot.

---

## ARCHITECTURE (THE SONCING RULE — most important thing)

Two trees exist:

1. **`$hdots` = `/acc/common/hdots`** — the PERSIST / canonical tree (also exposed
   via `/home/tw/.config/hdots`, a symlink). **ALWAYS EDIT HERE.** This is the
   living source of truth.
   - `/acc/common/hdots/.config/hypr/env.lua` is the env-var source of truth.
     `/home/tw/.config/hypr/env.lua` → symlink to the SAME file (inode 782418).
2. **`$sources` = `/tmp/<user>_rconf/sources`** (e.g. `/tmp/tw_rconf/sources`)
   — the TMPFS runtime copy, populated every boot.
   `/tmp` is `tmpfs`, cleared on reboot. GONE after reboot.

### THE SOURCING RULE
- **BEFORE sync (main_logic)** — use `$hdots/sources/...`:
  - `hypr_launch` bootstrap (`dofile(hdots .. "/sources/utils")` then
    `dofile(hdots .. "/sources/prelaunch_body")`)
  - `splash_init` (+ `fetch_color_scheme`, `splash_loading_texts`, `splash` binary)
  - `sync_to_tmpfs`
- **AFTER sync (main_logic)** — use `$sources/...`:
  - `create_dummy_configs`
  - `hypr_init_logic` + everything it sources, all other helpers
  - all `bin/*` and `sr/*` runtime scripts

> **LAYOUT NOTE (post-flatten):** all files that previously lived in
> `sources/` now live directly at `sources/` root (next to `utils`,
> `configs_array`, etc.). There is NO `sources/` directory anymore —
> any `/helpers/` mentioned in historical DIFFS of this file refers to the old
> layout.

### ENV VARS (set ONLY by env.lua; NO fallback checks anywhere)
`env.lua` exports: `hdots`, `hdots_sources`, `rbase`, `rconf`,
`sources`, `hypr_global`, `rcache`, `hypr_log`, `hypr_scripts`,
`hypr_temp`, `lock_files`, `states`, `states2`, `theme_elements`,
`custom_acc_master_list`, `garbage`, `hex_states`, `shaders`, `rofi_pid`,
`rofi_id`, `XDG_SESSION_DESKTOP`, `QML_IMPORT_PATH`, `PATH`, plus misc defaults.

**`hypr_bin` env var was REMOVED.** There is NO `$hypr_bin` anymore.
**`$sources/bin` is on `$PATH`** — `env.lua` prepends it
(`custom_path = sources .. "/bin"`) so every bin tool (`blur_main`,
`theme_main`, `hs_main`, ...) is a bare command. Invoke bin commands by BARE
NAME (`run("theme_main restore")`, `bg_spawn("blur_main restore")`), NEVER by
path (`sources .. "/bin/..."` or `lua_bin .. "/..."`); these full-path calls
were all removed (see item #24).

**`garbage` added to `env.lua`** — exported as `rconf .. "/garbage"`.
No script should derive it anymore.

### RULES (enforced)
1. **No env-var checks or fallbacks — env vars are ALWAYS defined upstream by
   `env.lua`.** So no `if not X or X == "" then os.exit(0) end` guards and no
   `os.getenv("X") or default` / `if not hs then ...` fallbacks anywhere. All
   env access in code is `os.getenv("var")` only, using the EXACT env var names
   (`$HOME` → `"HOME"`, `$USER` → `"USER"`, `$sources` → `"sources"`)
   — never `os.setenv(...)`, never `hl.env(...)` in helper/bin code.
2. **`rm_rf` is banned.** Do NOT call `rm_rf` on any directory, ever — not on
   `/`, `$HOME`, `$hdots`, `$rbase`, `$rconf`, not even on explicit
   targets, not even in tests. No exceptions.
3. **Bootstrap for all runtime scripts** (bin/*, sr/*):
   `sources = os.getenv("sources")` + `dofile(sources .. "/utils")`
   (plain `sources` only — NO `lua_bin` / `lua_sources` / `dir`-derived
   bootstrap anymore).
4. **`main_logic` (prelaunch_body) syncs ONLY `$hdots/sources → $sources`.**
   All other provisioning (quickshell, misc, hypr-theme, states, cache,
   icons, scripts) happens inside `hypr_init_logic`. NO linking in main_logic.
5. **All symlinking happens in `hypr_init_logic` only.** `hypr_init_logic` does
   all the symlink work via `symlink_dirs` / `symlink_files` / `target_point` /
   `point_to_contents` arrays + `link_it_defs` functions.
6. **`link_it_defs` semantics** — faithfully mirror bash:
   `mkdir -p` src + parent(dst), `-ef` same-file skip, symlink dst moved to
   `/tmp/moved_deleted.$rnd`, existing dir backup to `dst.$rnd` /
   file to `dst.bak.$rnd`, then `ln -s src dst`. Same for
   `point_to_contents_function` (loop entries, symlink→/tmp, then link).
7. **`set_theme_elements()` must run in `set_dark_function` and
   `set_light_function`** — the port previously omitted it, causing Thunar
   to stay white because `gsettings set gtk-theme` never fired on switch.
8. **NO external filesystem binaries.** Never call `cp`, `mv`, `rm`, `mkdir`,
   `touch`, `chmod`, `ln`, or `rsync` via `spawn`/`run`. Use the pure-Lua FFI
   helpers in `utils` instead: `copy_tree`/`copy_dir_contents` (recursive copy
   that preserves symlinks and OVERWRITES in place — it NEVER deletes),
   `mv`, `rm` (single file/symlink unlink ONLY), `mkdir_p`, `touch`, `chmod`,
   `chmod_r` (`chmod -R`), `chmod_x` (`chmod +x dir/*`), `symlink`,
   `sleep` (FFI nanosleep), `kill_pid`/`process_alive` (FFI `kill`, direct
   syscall — no external `kill` binary), `list_dir`, `readlink`. This also
   keeps the
   "no dir removal" rule: copy helpers never `rm -rf`; the old
   `rsync --delete-before` in `sync_back` was DROPPED — syncing back only
   overwrites matching entries and never deletes extraneous ones.
9. **Symlinks are preserved by copy.** `copy_tree` recreates a source symlink
   as a symlink (via `readlink`+`symlink`) instead of copying through it,
   matching `cp -r` behavior.
10. **Bin commands are on `$PATH`** — `env.lua` prepends `$sources/bin` to
     `PATH`. Call bin tools by BARE NAME (`run("theme_main restore")`,
     `bg_spawn("blur_main restore")`), never by full path
(`sources .. "/bin/..."`). `utils` exports plain `sources` — there is
      no `lua_bin` / `lua_sources` variable anymore.
11. **`notify-send` MUST always run in the background** (`run_bg`), never
     blocking. Same for `bg_delay` shells — they are daemonized by design.
12. **Binaries execute DIRECTLY** via FFI `fork`+`execvp`
     (`exec_cmd` → `run`/`run_bg`). A `/bin/sh -c` shell fallback is used ONLY
     when the command contains genuine unquoted meta chars
     (`| & ; < > ( ) $ * ? [ ] { } # ~ !`, backslash, newline, unquoted
     backtick). `needs_shell` scans for those; `shq` single-quote escaping
     (`'...'\`) routes values through the shell deliberately.
13. **NO value fallbacks anywhere except `prelaunch_body` + `utils`.**
     Banned everywhere else: `x or "default"`, `x or 0`, `tonumber(ff(...)) or N`,
     `ff(...) or ""`, CLI-arg defaults (`arg[1] or "..."`, `action or "status"`,
     `m_arg or "toggle"`), env defaults, `arg[0]:match(...) or "."`. Missing
     upstream state must ERROR LOUD, never silently default. `prelaunch_body`
     is the ONLY file that may seed/guard mode vars with fallbacks; `utils` is
     RESERVED (keeps its internal fallbacks per user decision, 2026-09-18).
     Things that are NOT fallbacks and remain: ternary expressions
     (`cond and a or b` like `find("MUTED") and "1" or "0"`), boolean
     conditions in `if` (`x == nil or x == ""`), and value-COMPUTING init
     guards (`if not m_init or m_init == "" then m_init = <from gsettings> end`
     in `hypr_init_logic` — computes real data, does not mask).
13. **NO value fallbacks anywhere except `prelaunch_body` + `utils`.**
     Banned everywhere else: `x or "default"`, `x or 0`, `tonumber(ff(...)) or N`,
     `ff(...) or ""`, CLI-arg defaults (`arg[1] or "..."`, `action or "status"`,
     `m_arg or "toggle"`), env defaults, `arg[0]:match(...) or "."`. Missing
     upstream state must ERROR LOUD, never silently default. `prelaunch_body`
     is the ONLY file that may seed/guard mode vars with fallbacks; `utils` is
     RESERVED (keeps its internal fallbacks per user decision, 2026-09-18).
     Things that are NOT fallbacks and remain: ternary expressions
     (`cond and a or b` like `find("MUTED") and "1" or "0"`), boolean
     conditions in `if` (`x == nil or x == ""`), and value-COMPUTING init
     guards (`if not m_init or m_init == "" then m_init = <from gsettings> end`
     in `hypr_init_logic` — computes real data, does not mask).
14. **`exit_0()` / `exit_1()` replace `os.exit(0)` / `os.exit(1)`** in all
     bin/* and sr/* scripts. `os.exit` is still used internally by `utils`
     (child-process error paths) and nonstandard codes stay as `os.exit(2)`.
15. **THEMES SCHEMA (flag-driven layout, PER-SIDE flags)** — see
    `sources/themes.md`:
    - **Themes are KEYED — no loop.** `sources/themes` is a dictionary of
      naked field arrays: `themes[name] = { ... }`.  `theme_ref(name)` is
      `return themes[name]` (O(1)), and discoverers iterate `pairs(themes)`
      collecting keys — never `ipairs` + `entry[1]` matching.
    - **All 24 layouts ship as real `combo_*` keys** (sort_index 9001–9024;
      8 singles + 16 duals, deterministic marker colors, inline comments with
      absolute field positions) — so the full layout space is verifiable off
      the shipped file, not just the 2 layouts the 287 real themes use.
    - Header is `sort_index theme_mode extras series` — the flags belong to
      the FIRST side (single = dark-only/light-only; dual = dark side).
    - **Dual light side carries its OWN inline flags**: after the dark block,
      `l_extras l_series` precede the light block, so dark and light sides may
      have different extras/series combos (4×4 = 16 possible dual layouts).
      **Two-level shift:** (L1) the light side's position depends on the dark
      side's length (`5 + dark_side_len(dark extras, dark series)`); (L2)
      inside the light block, the light series is pushed by the light-side's
      own extras (`+9` when `l_extras=1`). Full 16-combo index table in
      `sources/themes.md` — all combos verified via unique-marker tests.
    - Side block: core 8 (`sfg_mode bg bg2 bg3 fg fg2 fg3 acc`) + extras 9
      (`acc_us acc_hc acc_ec border hover focus disabled sfg_d sfg_l`,
      only when THAT side's extras=1) + series 6 (`series1..series6`,
      only when THAT side's series=1).
    - `success/warning/danger/info` are HARDCODED GLOBALS in `theme_init`
      (`set_global_semantic()`) — never stored in the themes file.
    - Series fallback sets hardcoded in `theme_init` (bright for dark bg,
      deep for light bg); themes override with their own 6 via `series=1`.
    - **On-Accent Foreground Rule:** dark side — `acc_us` has DARK sfg,
      `acc_hc`/`acc_ec` have LIGHT sfg; light side — `acc_us` has LIGHT sfg,
      `acc_hc`/`acc_ec` have DARK sfg. `acc_us` is always the opposite-sfg
      accent.
    - `acc_ec` = `$bg` + 10–20% `$acc` (unchanged, still excellent).
    - Accent field order in the side block: `acc acc_us acc_hc acc_ec`.
    - Field counts: single full = **27**, dual full both sides = **52**
      (header 4 + dark 23 + light flags 2 + light 23).
    - Themes file renders one entry per line with top legend + demo comments.

---

## PORTING DECISIONS / WHAT LUA CONVERSION LOOKS LIKE

### Pre-sync helpers (live on persist `$hdots`, read before tmpfs copy)
- `hypr_launch` (in `sources/bin/hypr_launch`) — entry point. Lua:
  ```
  #!/usr/bin/env luajit
  hdots = os.getenv("hdots")
  dofile(hdots .. "/sources/utils")
  dofile(hdots .. "/sources/prelaunch_body")
  launch_final_func()
  ```
- `prelaunch_body` (in `sources/`) — defines `main_logic()` (the tmpfs
  sync), `splash_func()`, `launch_final_func()` (no env guards — env.lua always
  defines vars). `launch_final_func`
  reads `states/s` + gtk-theme, builds `sm`/`s_init`/`m_init`, calls `main_logic()`,
  then after sync does `dofile(sources .. "/create_dummy_configs")`
  (if shadow.lua missing) and `dofile(sources .. "/hypr_init_logic")`.
- `sync_to_tmpfs` (in `sources/bin/`) — Lua script that calls
  `prelaunch_body`'s `main_logic()` directly to resync persist → tmpfs without
  a full launch. Bootstrap before sync (from `$hdots`).

### Main custom helpers (AFTER-sync consumers)
- `utils` — moved to `sources/utils` (sources root, NOT under `helpers/`).
  Loaded via `dofile(sources .. "/utils")` (runtime) /
  `dofile(hdots .. "/sources/utils")` (pre-sync). Reads globals from env,
  `run`, `spawn`, `shq`, `is_dir`, `is_symlink`,
  `list_dir`, `mkdir_p`, `ff`/`tf`/`append`, pure-Lua FFI `mkdir`/`rm`/`mv`/
  `symlink`/`touch`/`chmod`/`chmod_r`/`chmod_x`/`rm_glob`/`copy_tree`/
  `copy_dir_contents`/`readlink`/`sleep`/`kill_pid`/`process_alive`,
  all working straight off plain `sources`
  (bin tools are on `$PATH` via `env.lua` — no `lua_bin` variable anymore,
  see rule #10 / item #24).
  File I/O is just TWO primitives (bash-mirrored, no `read`/`write`/`state_*`
  families):
  - `ff(path)` = from-file: nil if missing; returns content with ALL trailing
    newlines stripped (mirrors bash `$(<file)`).
  - `tf(path, content)` = to-file: writes content verbatim (mirrors bash `>`).
  - `append(path, content)` = bash `>>`.
  `mv`/`copy_file`/`copy_dir_contents` use private `raw_read`/`raw_write`
  (`rb`/`wb`) so copies stay BYTE-EXACT (no newline stripping).
  `sources/splash` and `sources/color_engine` are compiled ELF
  binaries (invoked via `run`/`spawn`, never dofile'd) — SKIP in any Lua
  lint/transform pass.
- All other helpers live directly at `sources/` root (AFTER-sync via `sources`).
- `create_dummy_configs` — has its own standalone bootstrap
  (`src_root = os.getenv("sources")` + `dofile(src_root .. "/utils")` +
  `dofile(src_root .. "/link_it_defs")`) so it works BOTH dofile'd (after sync)
  and standalone (`luajit $sources/create_dummy_configs`, used by `.zshrc`
  first-boot bootstrap in place of the old bash `source`).

---

## CURRENT FILE STATE (as of this update)

### DONE & COMMITTED TO DISK (in `/acc/common/hdots/`) — survives reboot
1. `.config/hypr/env.lua` — `hypr_bin` env var line REMOVED;
   `custom_path = sources .. "/bin"` (PATH fix — `$sources/bin` on `$PATH`);
   added `garbage` export. ✓
2. `sources/bin/hypr_launch` — rewritten as Lua bootstrap (persist-based,
   pre-sync). ✓
3. `sources/prelaunch_body` — rewritten as Lua with guard, `main_logic`,
   `splash_func`, `launch_final_func`. `main_logic` does ONLY the tmpfs sync
   (pure-Lua `copy_dir_contents` on `sync_to_tmpfs_array`) — NO linking,
   NO external `cp`. ✓
4. `sources/utils` — Lua helper: reads globals from env (no fallbacks),
   `run`/`spawn`/`shq`/`mkdir_p`/`ff`/`tf`/`append`/`list_dir`/`is_dir`/
   `is_symlink`/`is_same_file`/`mv`/`symlink`/`touch`/`chmod`/`chmod_r`/
   `chmod_x`/`rm_glob`/`copy_tree`/`copy_dir_contents`/`readlink`/
   `sleep` (all pure-Lua via LuaJIT FFI, no external binaries), all working
   straight off plain `sources` (no `lua_sources`/`lua_helpers`/`lua_bin`
   aliases). ✓
5. `sources/hypr_init_logic` — Lua rewrite: guard at top
   (`rbase`/`rconf` empty → `os.exit(0)`), REMOVED the fallback default
   block. Contains ALL symlinking (symlink_dirs/files/target_point loops) and
   ALL other provisioning (quickshell/misc/hypr-theme/scripts/cache/icons
   via pure-Lua `copy_dir_contents`). ✓
6. `sources/sync_to_tmpfs_array` — pairs persist(hdots)→runtime paths for
   `main_logic` sync; now contains ONLY `sources→sources` (single pair);
   pure-Lua copy, NO rm_rf, NO linking, NO external binaries. ✓
7. `sources/link_it_defs` — Lua port faithfully mirroring bash semantics
   (`-ef` skip, symlink→/tmp, dir/file backup, `ln -s`). ✓
8. `sources/point_to_contents` — Lua port mirroring bash
   (`-ef` skip, symlink→/tmp, backup, `ln -s`). ✓
9. `sources/create_dummy_configs` — sources `link_it_defs` from
   `$sources` (sourced AFTER sync). ✓
10. `sources/theme_body` — added `set_theme_elements()` to
    `set_dark_function`/`set_light_function` (fixes Thunar staying white). ✓
11. `sources/bin/channel_switcher` — removed `os.setenv` (env-only rule). ✓
12. `sources/gen_primary` (hl.env XCURSOR/HYPRCURSOR) + all fallbacks in
    `rconfig_gen`/`awww_final`/`hyprlock_vars_gen`/`hypr_init_logic`/
    `power_main_body`/`power_body`/`create_dummy_configs` removed —
    env only via `os.getenv`, no `or default`. ✓
13. `sources/theme_block_init` — theme tree copy via pure-Lua
    `copy_dir_contents`; stale `index*` removal via `rm_glob` (files only). ✓
14. `sources/sync_first` / `sync_back` / `sync_etc_files` — all `cp -r`
    and `rsync` replaced with pure-Lua `copy_dir_contents` (overwrite, NEVER
    `--delete`); `sync_back`'s `--delete-before` semantics DROPPED. ✓
15. `sources/utils` MOVED → `sources/utils` (sources root). All
    references updated tree-wide: bin/*, sr/*, helpers/*, `symlink_files`,
    `channel_switcher` embedded string, PORTING_NOTES. `utils` reads `sources`
    from env and derives `lua_root` from its own path
    (`^(.-)/sources/utils$`). ✓
16. ALL env-var guards REMOVED (env.lua always defines vars): `hypr_launch`,
    `sync_to_tmpfs`, `prelaunch_body`, `hypr_init_logic` — no
    `if not X or X == "" then os.exit(0) end`, no `if not hs then ...` fallback.
    Env access is `os.getenv("EXACT_NAME")` only. ✓
17. ALL `sources/bin/*` and `sources/sr/*` converted to the env-based bootstrap:
    `sources = os.getenv("sources")` + `dofile(sources .. "/utils")`.
    NO `dir`/`lua_bin`/`lua_sources`-derived bootstraps remain — everything
    runs off plain `sources`. ✓
18. `channel_switcher` — embedded background luajit string is env-based
    (`os.getenv('sources')` + `dofile(sources .. "/utils")`);
    `dir_esc`/`src_esc` removed. ✓
19. READING/WRITING REFACTORED TO `ff`/`tf` (user-requested, see "no hundreds of
    near-duplicate helpers"): removed `read`, `read_line`, `write`,
    `write_line`, `read_num`, `write_num`, `state_read`, `state_write`,
    `state2_read`, `state2_write` (all `read`-variants collapsed into `ff`,
    all `write`-variants into `tf`; callers build their own path e.g.
    `tf(states2 .. "/" .. "s", v)`). `ff` strips ALL trailing newlines
    (bash `$(<...)`); `tf` writes verbatim. `mkdir_p` dupe guard trimmed.
    Tree-wide conversions applied with a context-aware transform
    (quote/comment/long-bracket aware; embedded generated-code strings
    hand-checked for `\"` escaping). `sync_etc_files` keeps BYTE-EXACT
    system-file backup via its own `raw_read`/`raw_write` (`rb`/`wb`). All
    `bin/* sr/* helpers/* utils` pass `loadfile`; zero `state_*`/`read_line`/
    `write_line`/`read_num`/`write_num` leftovers (ELF binaries + `.md` +
    `toggles_main_body` bash-remnant excluded). ✓
20. `sources/toggles_main_body` — the last bash-remnant helper PORTED to
    Lua (it was never converted and `bin/toggles_main` dofiles it → runtime
    break). Mirrors bash exactly: `acc_flag_read` (local `ch` threaded,
    missing/empty → fallback), `theme_toggle_func` / `theme_tone_func` /
    `theme_alpha_func` / `theme_acc_scrim_func` (flag whitelists, `%x%x` hex,
    lowercase, write-only-if-differ, `run("theme_main restore")`), and
    `toggles_cmd(...)` dispatcher. Now `loadfile`-clean like everything else. ✓
21. ENV VAR RENAMED `hypr_sources` → `sources` (user-requested): `env.lua`
    now `local sources = rconf .. "/sources"` + `hl.env("sources", sources)`;
    every bootstrap is `sources = os.getenv("sources")` +
    `dofile(sources .. "/utils")`; `utils` mirrors `sources` (no
    `lua_sources`/`lua_bin` aliases); `configs_array`,
    `custom_accents_array`, `symlink_files`, `symlink_dirs`, `daemon/*`,
    `channel_switcher`'s embedded string, `PORTING_NOTES.md` all updated.
    Literal `/sources` directory paths and `hdots .. "/sources/..."`
    pre-sync bootstrap strings are UNCHANGED (the directory keeps its name;
    pre-sync still reads from persist, not the env). Zero `hypr_sources`
    remains in `sources/` + `env.lua` + `PORTING_NOTES.md`; all loadfile-clean. ✓
22. HELPERS DIR FLATTENED (user-requested): all 76 files from
    `sources/helpers/` moved to `sources/` root; `sources/helpers/` directory
    REMOVED. Every reference updated (`sources/helpers/` → `sources/`,
    `lua_helpers .. "/` → `sources .. "/`, `lua_helpers = ...` deleted).
    Special cases fixed: `theme_main_body` standalone bootstrap now keys off
    `_G.sources == nil` + `dofile(sources .. "/utils")`; its debounce
    thread uses `dofile(uh .. '/utils')` with `sources`/`mypid` args;
    `channel_switcher` rewritten to standard env bootstrap. `splash` and
    `color_engine` ELF binaries moved to root too (still skipped by lint).
    Zero `lua_helpers` remains; all `bin/* sr/* daemon/* utils` + root files
    pass `loadfile`; leftover "helpers" mentions are comments/docs only. ✓
23. QML + EXTERNAL REFS FIXED (post-flatten): `RestoreCategory.qml` now runs
    `luajit $sources/gen_flags_master` after the `rm` of state files (the old
    `. $hypr_sources/helpers/gen_flags_master from_restore_default` was a
    dead pattern — the bash script never handled `from_restore_default`, it
    worked only because the `rm` made files missing so the `[[ -f ]] ||`
    seeds recreated them; the Lua port preserves that contract). Added a
    standalone bootstrap to `gen_flags_master` (verified: seeds 144 defaults
    into a fresh `$states`). `SystemSettingsManager.qml` now reads + exports
    the `sources` env (was `hypr_sources`). `.zshrc` first-boot dummy-config
    bootstrap switched from `. sources/helpers/create_dummy_configs` (bash
    `source` of a Lua file) to `sources=$HOME/.config/hdots/sources luajit
    $HOME/.config/hdots/sources/create_dummy_configs` (verified standalone).
    In-tree doc paths (`themes.md`, `gen_theme_cards.md`) and the GTK
    bookmarks entry updated. ✓
24. BIN COMMANDS ON `$PATH` (user-requested): since `env.lua` prepends
    `$sources/bin` to `PATH`, bin tools are called by BARE NAME. Removed the
    full-path invocations in `theme_body_run` (`bg_spawn("blur_main restore")`,
    `bg_spawn("shadow_main restore")`, `bg_spawn("opacity_main restore")`,
    `bg_spawn("general_main restore")`, `bg_spawn("hs_main restore")`,
    `bg_spawn("shader_main restore")`) and in `bin/pick_accent_main`
    (`run(lua_bin .. "/theme_main restore")` → `run("theme_main restore")`);
    dropped the then-unused `lua_bin = sources .. "/bin"` from `utils` (only
    plain `sources` remains). The only bin script still launched by
    persist path is the entry point `sources/bin/hypr_launch` (pre-sync, via
    `hl.exec_cmd("$HOME/.config/hdots/sources/bin/hypr_launch")`). ✓
25. THEME_BODY_RUN WAIT LOOP FIXED (user-requested, "takes very long"):
    `shader_body` `shader_restore_func` had an early return on the
    default-100.glsl path that skipped `tf(states2 .. "/shader_done", "1")`
    → `shader_done` stuck at "0" and `theme_body_run` always burned its full
    30×0.1s wait before `hyprctl reload`. Now the done flag is written
    unconditionally (incl. the early-return path), and `theme_body_run`'
    wait is stall-aware: it stops as soon as all six flags are `1`, or after
    1s straight with NO new flag completing (was: fixed 3s budget even with
    a dead flag).
26. THEMES FILE REWRITTEN TO FLAG-DRIVEN SCHEMA (user-requested redesign):
    - `themes` regenerated from the one-line 30/57 layout into the new
      `{sort_index theme_mode extras series}` header + variable side blocks
      (core 8 / extras 9 / series 6). All 287 themes currently ship
      `extras=1 series=1` → single **27**, dual **50** fields.
    - `success/warning/danger/info` REMOVED from the file → hardcoded global
      constants in `theme_init.set_global_semantic()`.
    - Accent quadrille reordered in the file to `acc acc_us acc_hc acc_ec`.
      (The previous one-line file actually stored `acc acc_ec acc_us acc_hc`
      — the parser comments had claimed a different order; now fixed and
      verified by the `nord_aurora` round-trip test.)
    - `theme_init` rewritten around `parse_side_vars(ref,start,extras,series,
      side_is_light)` with `side_block_size()`; `resolve_theme` / `d_side` /
      `l_side` use it. Series fallback sets (bright/deep) hardcoded; extras=0
      falls back via `load_default_extras()`.
    - `gen_theme_cards` rewritten to reuse `parse_side_vars` (no hardcoded
      offsets). 287/287 cards emit.
    - themes.md / gen_theme_cards.md fully rewritten for the new layout;
      `themes.bak` kept at `/tmp/opencode/rbtest/themes.bak`.
    - **Per-side flags for duals (amendment):** each side gets its OWN
      `extras series`. The header holds the dark side's; the light side has
      inline `l_extras l_series` right before the light block
      (`light_flags = 5 + side_block_size(dark_extras, dark_series)`).
      One fix while doing this: the first dual-flags insert attempt truncated
      `sources/themes` via `io.open(path, "w")` (only a TEMP path may ever be
      opened with "w" — write then `os.rename`). Duals are now **52** fields
      (header 4 + dark 23 + light-flags 2 + light 23). Parser verified for
      mixed combos (dark extras=0/series=0 with light extras=1/series=1 etc.)
      and 287/287 cards still emit.
27. **FALLBACK-STRIPPING PASS DONE (user-requested, 2026-09-18):** removed
    every `or` VALUE fallback tree-wide except `prelaunch_body` (only place
    that may keep fallbacks) and `utils` (user-reserved). Converted:
    `daemon/run_inside_loop` (`or "1"` — semantics-preserving, `nil ~= "0"`),
    `hypr_init_logic` (m_init/s_init blocks, caps_lock_state,
    vol/mic parse, all_theme_set wait), `theme_body` (~20), `theme_init`
    (display/save var lists), `theme_main_body` (flags, restore/apply/switch,
    save_flags_func, theme_cmd), `pull_colors_val` (all 17),
    `hyprlock_vars_gen`, `toggles_main_body` (`acc_flag_read` fallback param
    removed + 4× `or "n"`), `scheme_body`, `audio_body` (~35),
    `brightness_body`, `network_body`, `power_body`, `pp_body`,
    `power_main_body`, `center_pop`, `splash_init`, `gen_gtk_css`
    (globals re-read; CSS `$var` substitution now `assert`s missing vars),
    `rconfig_gen`, `theme_save_hex`, `bin/*` (dash_status, caps_lock_trigger,
    channel_switcher, pick_accent_main, shader_main, audio_main, hs_main,
    extract_here), `daemon/*`. CLI args no longer defaulted (`action`/`m_arg`/
    `arg[N]` must be supplied). Genuine ternaries/conditions and value-COMPUTING
    guards remain (rule #13). All touched files pass `loadfile`. Note: `bin/caps_lock_trigger`
    still `dofile(hypr_global .. "/qpid")`, a stale
    pre-existing ref to a `read_line`-era file — out of fallback scope. ✓

28. **LAST EXTERNAL BINARIES REMOVED — FFI `kill` + `sleep` migration
    (2026-09-19):** added `int kill(int pid, int sig);` to the `utils` cdef and
    two in-process wrappers — `kill_pid(pid, sig)` (libc `kill`, SIGTERM
    default, direct syscall, no shell) and `process_alive(pid)`
    (`kill(pid,0)` → bash `kill -0`). Migrated every `kill`-shell caller to
    direct libc: `daemon/watch_dog` (3 sites: kill -0/-9/-15 →
    `process_alive`/`kill_pid`), `theme_main_body` (`os.execute("kill -0 …")`
    → `process_alive`), `theme_body_run` (kitty `kill -SIGUSR1` via
    `run_bg` → per-pid `kill_pid(kpid, 10)`, SIGUSR1=10, iterating the
    `qpid("kitty")` pid list). Swapped every pure `run("sleep …")` to the
    existing FFI `sleep` (7 sites: `hs_body` ×2, `chsw_body` ×1,
    `awww_final` ×1, `hypr_init_logic` ×3). Left INTACT: `hs_body` restore
    chains that use `sleep 0.3 && …` INSIDE `run_bg` shell strings (genuine
    shell semantics, rule #12). `kill_pid`/`process_alive` smoke-tested live;
    all touched files pass `loadfile`. External filesystem/process binaries
    now gone everywhere. ✓
29. **FFI CONFINED TO `/utils` ONLY (user-requested, 2026-09-19):** no script
     declares its own `require("ffi")`/`ffi.cdef` anymore — every FFI binding
     and load happens in `utils` behind shared helpers:
     - `chsw_body` — dropped private `ffi.cdef[[ getpid ]]`; `ffi.C.getpid()`
       → shared `getpid()` from `utils`.
     - `asym_body` / `rsym_body` — dropped duplicated private
       `struct lstat_buf`/`lstat` `is_link()`; now use shared
       `is_symlink()` (readlink-based, equivalent for dangling links too).
     - `color_pick` — dropped private stb_image cdef + `ffi.load`; `utils` now
       hosts the stb_image cdef and a LAZY loader (`local stb_image()` loads
       `$sources/stb_image.so` only on first decode) exposing
       `load_stb_image(path) → (px,w,h)` / `free_stb_image(px)`. Only
       processes that actually decode load the .so.
     Grep-verified: zero `ffi` outside `utils`. All touched files pass
     `loadfile`; `is_symlink` (file/link/dangling) + stb decode round-trip
     (300x86 PNG) verified live. ✓
30. **SHARED-MEMORY STATE LAYER ADDED TO `utils` (2026-09-19, file-I/O
     bottleneck first):** new pure-FFI `mmap` primitives bypass the
     `io.open`/`read`/`write`/`close` syscall path for hot-path state — a
     tmpfs file's page-cache RAM is mapped `MAP_SHARED` into every process that
     attaches the same path, so reads/writes are direct RAM ops (ns) instead of
     syscalls (µs). The file stays on disk (debuggable, survives restarts —
     still fits the `$states`/`$states2` "state is a file" convention).
     - cdef added: `ftruncate`, `mmap`, `munmap`, `msync`.
     - Helpers: `shmem_init(path, size) → (ptr, size)` (create-or-attach,
       O_RDWR|O_CREAT 0600), `shmem_release(ptr, size)` (munmap),
       `shmem_sync(ptr, size)` (msync MS_SYNC), `shmem_write(ptr, size, str)`
       (bounded copy), `shmem_str(ptr, len)`, `shmem_view(ptr, ctype)`
       (struct/primitive overlay — no `ffi` access needed by callers).
     - **Gotcha fixed:** `open` is VARIADIC in libc; LuaJIT passes a bare Lua
       number as a `double`, so the vararg `mode` low bytes came out 0 (file
       created mode 000 → re-attach EACCES). Mode is now passed as
       `ffi.cast("int", tonumber("600", 8))`.
     - Verified live across THREE independent processes (writer / reader /
       reader2): writer stores into RAM, separate reader attaches and reads via
       pure RAM, mutates; reader2 sees the bump. File mode 0600.
     - NOT yet wired into callers (child-exec optimization deferred per user;
       file-I/O done first). Natural first adopters: `channel_switcher`
       debounce, `watch_dog` loop check. ✓
31. **`theme_body_run` WAIT LOOP → PURE INOTIFY (2026-09-20, resume point
     completed):** the sleep-poll loop (`max_wait=30` / `stalled` / `sleep(0.1)`)
     is GONE, replaced with a pure kernel-blocking inotify wait — comments
     updated, helpers `done_flags`/`flags_done_count` kept:
     - `inotify_init()` (IN_CLOEXEC) → `inotify_watch` each of the 6 flag
       FILES (`states2 .. "/" .. flag`) with `0x2+0x8`
       (IN_MODIFY | IN_CLOSE_WRITE; the terminal `"1"` flip always emits via
       the first-`flagf_set` `shmem_init`→`ftruncate`), plus the `states2`
       DIR with `0x100+0x400+0x8000` (IN_CREATE | IN_DELETE_SELF | IN_IGNORED)
       as the never-seeded fallback.
     - Loop: `repeat inotify_wait_event(fd, -1); until flags_done_count() >=
       #done_flags` — blocks in kernel `poll()`/`read()`, 0% CPU, no timer, no
       max-wait, no stalled counter; exits solely on the last event, then
       `inotify_close(fd)`. `assert(ifd)` errors loud if the inotify fd is nil.
     - Verified: standalone `dofile(utils)` parse + `loadfile(theme_body_run)`;
       live cross-process test (independent luajit writer flips all 6 flags at
       0.4s intervals while the reader runs the exact new loop — exits alone on
       event 6, no timer; test flag files cleaned back out of `$states2`). ✓
32. **`utils` FFI cdef GUARDED — LAUNCH PIPELINE FIXED (2026-09-20, crash at
     boot):** the real launch (`bin/launch` → `prelaunch_body.launch_final_func`)
     crashed with LuaJIT `attempt to redefine 'inotify_event'`. Root cause:
     `bin/launch` dofiles `utils` from PERSIST pre-sync, `main_logic()` syncs
     persist→tmpfs, then `launch_final_func` dofiles `gen_flags_master` which
     has its OWN standalone bootstrap (`dofile(src_root .. "/utils")` from the
     tmpfs copy) — so `utils` was `dofile`'d twice in ONE process. LuaJIT
     `ffi.cdef` merges identical typedefs/functions fine BUT rejects re-declaring
     a named struct (`struct inotify_event`, `struct pollfd` → `attempt to
     redefine`). The structs came from the inotify layer (#31), whose earlier
     verification never double-loaded utils in one process. Fix: guard both
     `ffi.cdef` blocks in `utils` behind `if not _G.utils_ffi_cdef then … end`;
     the mmap constants (`PROT_READ`/`PROT_WRITE`/`MAP_SHARED`/`MS_SYNC`) and all
     function REdefinitions stay OUTSIDE the guard (run on every dofile; only
     the cdef calls are skipped). A second bug fixed while testing: the constants
     were initially placed INSIDE the guard, so a re-dofile skipped them and
     `shmem_init` failed on `PROT_READ = nil` ("arithmetic on global") — moved
     above the guard. Verified: `bin/launch` runs the FULL real pipeline to
     `End of hypr-init` (EXIT 0), themes load, `theme_body_run` event-loop fires
     hyprctl reload; double-`dofile(utils)` regression test passes in-process.
     (Two cops `bin/extract_here` + `bin/hp` are pre-existing BASH scripts on
     the Lua tree — intentional, never loadfile'd.) ✓

### STILL PENDING
1. `.config/niri/config.kdl:385-386` — stale `spawn-sh ". $hypr_sources/helpers/
   cliphist_text|cliphist_image"` binds; those files no longer exist anywhere
   (clipboard handling moved to quickshell IPC per commented-out
   `keybinds.lua`). Pre-existing dead binds, OUTSIDE the Lua port scope.
2. `rust_project_root/` (zen-shell) + `legacy/` + `.bash_history/.zshrc
   profiles logging` — contain historical `hypr_sources`/`sources/helpers`
   references; frozen/backup copies, intentionally NOT updated.

---

## GIT / COMMIT RULES
- Repo: `/acc/common/hdots/.git`, branch `main`, remote `origin/main`.
- Last commit restored: `7af1330 Update zypp 2026-09-17 12:42:47`.
- Only commit when the USER explicitly asks. Commit message style: short phrase
  with date (e.g. "Update zypp 2026-09-17 12:42:47").
- Storage is tight — don't commit or add files without asking.

---

## NEXT STEPS (first actions after reboot)
1. Verify the survived files (env.lua, hypr_launch, prelaunch_body, utils,
   hypr_init_logic, link_it_defs, point_to_contents, theme_body,
   sync_to_tmpfs_array) are the Lua versions listed under CURRENT FILE STATE.
2. Verify `sources/bin/sync_to_tmpfs` exists (persist-based, resync persist→tmpfs).
3. DONE — `theme_body_run` X_main spawn paths (bare bin names, item #24).
4. Then the remaining script conversions (pending #4-6).
5. Symlink check: `/home/tw/.config/hdots -> /acc/common/hdots`,
   `/home/tw/.config/hypr/env.lua -> /acc/common/hdots/.config/hypr/env.lua`.

## COMMANDS THAT ARE FORBIDDEN in future sessions
- `rm -rf` of ANY directory — never, even in tests, even with a guard. No exceptions.
- Any external filesystem/process binary via `spawn`/`run`: `cp`, `mv`, `rm`,
  `mkdir`, `touch`, `chmod`, `ln`, `rsync`, `kill`, `sleep`. Always use the
  pure-Lua FFI helpers in `utils`
  (`copy_tree`, `copy_dir_contents`, `mv`, `rm`, `mkdir_p`, `touch`, `chmod`,
  `chmod_r`, `chmod_x`, `rm_glob`, `symlink`, `sleep`, `list_dir`, `readlink`,
  `kill_pid`, `process_alive`).
- `rsync --delete*` semantics — never delete extraneous/destination files during
  copy or sync-back. Copy helpers only OVERWRITE matching entries.
- Running code loaded from a stale tmpfs copy to "verify" a guard.
- Any destructive test without confirming the loaded file actually contains the guard.
- Setting/fallbacking any env var in helper/bin code (`os.setenv`, `hl.env`,
  `os.getenv(...) or default`) — env.lua is the ONLY setter.
## 2026-09-19 SESSION TAIL — inotify layer LANDED; theme_body_run event-loop DONE (2026-09-20)

**Landed and verified (2026-09-19) — inotify layer in `utils`:**
- `sources/utils` gained a pure-FFI **inotify** layer (`utils:251-301`): cdef, `inotify_init`, `inotify_watch`, `inotify_wait_event` (kernel `poll`+`read`, 0% CPU), `inotify_close`. Single definition each (grep-count == 3 for the fn set, no dupes).
- Constant fixed at `utils:268`: `inotify_init1` flags must be `0x800 + 0x80000` (`IN_NONBLOCK | IN_CLOEXEC`); `0x800 + 0x8000000` is **EINVAL** (verified empirically on this kernel: `0x8000000 → fd<0`, `0x80000 → fd OK`).
- Mask set `0x2 + 0x8` (IN_MODIFY|IN_CLOSE_WRITE) fires on a **cross-process** flip: tested with independent luajit process flipping via `flagf_set → shmem_init → ftruncate`; blocking reader woke, `flagf_get == "1"` byte-exact (single-iteration, no poll).

**Why a per-file watch wakes reliably here:** every bg `*_main restore` runs in a FRESH process whose FIRST `flagf_set(path,"1")` does `shmem_init` → `ftruncate(fd,1)` → kernel `IN_MODIFY` on the flag file — the terminal `"1"` flip always emits. Files exist (seeded by `theme_body:123-131`) before `theme_body:218 dofile(theme_body_run)`, so `inotify_add_watch` on each file succeeds; an `IN_CREATE` dir-watch fallback covers the never-seeded case.

**DONE 2026-09-20 — `theme_body_run` event-loop (resume point completed):**
The sleep-poll loop (`max_wait=30`, `stalled`, `sleep(0.1)`, 1s give-up) is replaced with a pure kernel-blocking inotify wait — no timer, no max-wait, no stalled counter:
- `inotify_init()` → watch the 6 flag FILES (`states2 .. "/" .. flag`) with `0x2+0x8` (IN_MODIFY | IN_CLOSE_WRITE) + the `states2` DIR with `0x100+0x400+0x8000` (IN_CREATE | IN_DELETE_SELF | IN_IGNORED) as dir/never-seeded fallback → `repeat inotify_wait_event(fd, -1); until flags_done_count() >= #done_flags` → `inotify_close(fd)`. Blocks in `poll()`/`read()` on the kernel queue; stays until all 6 read `"1"`; 0% CPU. `assert(ifd)` errors loud if the fd is nil. `done_flags`/`flags_done_count` kept; deleted only the loop. `grep -v inotifywait` rule continues (no external binary).
- Verified: standalone `dofile(utils)` parse; `loadfile(theme_body_run)`; live 3-process flip test (independent luajit writer flips all 6 at 0.4s spacing while the reader runs the exact new loop — exits solely on event 6, no timer needed); test flag files removed from `$states2` afterward.

## 2026-09-20 — inotify `theme_body_run` LANDED + LAUNCH PIPELINE FIXED (resume point + boot crash)

**DONE — `theme_body_run` event-loop (item #31):** the sleep-poll loop is replaced with a pure kernel-blocking inotify wait (keep `done_flags`/`flags_done_count`; delete only the `max_wait`/`stalled`/`sleep(0.1)` loop). Same inotify helpers in `utils`; verified with a live cross-process 6-flag flip test — loop exits solely on event 6.

**DONE — `utils` cdef guard fixed a REAL boot crash (item #32):** running `bin/launch` (the actual hyprland-start pipeline) hit LuaJIT `attempt to redefine 'inotify_event'` — `utils` was dofile'd twice in one process (persist in `bin/launch`, then tmpfs via `gen_flags_master`'s own standalone bootstrap after `main_logic()` sync). LuaJIT rejects re-declaring named structs on the 2nd cdef. Fixed by guarding both `ffi.cdef` blocks (`if not _G.utils_ffi_cdef`), keeping constants + function redefs outside the guard. Verified: full `bin/launch` → tmpfs sync → `gen_flags_master` → `create_dummy_configs` → `hypr_init_logic` → `theme` block → `End of hypr-init` runs to EXIT 0, themes parse, `hyprctl reload` fires. This is the REAL restart path (hyprland.lua `hyprland.start` → `bin/launch`); a plain extras-only or splash-only test would NOT have hit it.

## 2026-09-26 — GAMMA MERGED INTO `shader_body`; `shader_m_$s` MODE FILE (0|g|s|m); `mshader` BASH PORTED TO LUA

**User request:** drop `gamma_main`, merge `gamma_body` into `shader_body`, and
replace the 0|1 `$states/shader_state_$s` flag with a single mode file
`$states/shader_m_$s` holding **`0` | `g` | `s`** (0 = all disabled, g = adjust
gamma, s = adjust only shader). No bash — port the `mshader` logic to Lua.
Follow-ups: use the pre-generated `saturations/` + `gamma/` pools (never generate
GLSL), keep bare ids in the state files (never a path), and add a fourth mode
**`m`** that bakes both ids + temperature into `master.glsl` (the master script
had to come back).

**DONE — one owner, one picker:**
- `sources/gamma_body` and `sources/bin/gamma_main` DELETED (stale tmpfs copies
  removed too). `sources/bin/mshader` (bash) DELETED — its GLSL template lives on
  as the `m` mode below; the unrelated stale `bin/master_shader` bash leftover in
  the tmpfs copy is gone as well.
- `sources/shader_body` is now the whole path: mode file → id file → pick (or
  generate) the shader → write `~/.config/hypr/luas/shader.lua` → `hyprctl
  reload`.
- **Modes are `0` | `g` | `s` | `m`:**
  - `0` nothing; `g` → `$shaders/gamma/<id>.frag`; `s` →
    `$shaders/saturations/<id>.glsl`; `m` → `$shaders/master.glsl`.
  - `g` and `s` are mutually exclusive (g leaves saturation at 100, s leaves
    gamma at 0). `m` is the only mode that runs BOTH ids plus colour
    temperature in one shader — the `mshader` GLSL (restored from memory:
    `SATURATION_VAL`/`GAMMA_VAL`/`TEMP_VAL` + `kelvinToRGB`, ids substituted with
    `string.format`) is regenerated into `master.glsl` on every apply.
  - Ids (`states/gamma_v`, `states/shader_val_$s`) survive a mode switch; only
    the mode decides what runs. In `m`, `set N` and `gamma-*` re-bake
    `master.glsl` and KEEP the mode (they are knobs, not switches); outside `m`
    they take the mode as before.
  - **EVERY id reads `0` as NORMAL** ("leave the colours alone" — never "0 Kelvin"
    or "0% mix"). All three at `0` = nothing to do, so `shader.lua` is cleared and
    the mode falls back to `0`; same for a missing file (defaults are 0, not the
    neutral constant). Saturation additionally treats the plain pool entry `100`
    as normal, so `0` and `100` mean the same thing. `shader_sat_eff()` /
    `temp_eff()` normalise `0` to the GLSL constant (100 / 6500) at bake time, so
    a `0` can never reach `kelvinToRGB` (which would clamp to 1000K = deep red).
    A missing gamma/saturation target file errors and resets instead of pointing
    hyprland at nothing.
- New CLI: `shader_main master` (→ mode `m`) plus the earlier
  `gamma-up` / `gamma-down` / `gamma-set N` / `gamma-reset` (bare `up`/`down`
  aliases). `status` prints `"<mode> <sat>"` → `0 100`, `g 100`, `s 50`, `m 120`.
- **Temperature:** the temp id is `states/hyprsunset_val` ONLY when
  `hyprsunset_state` is `m` (manual); `c`/`a` read as `0` (= normal) so the shader
  and the hyprsunset daemon never double-apply. A `hyprsunset_val` of `0` is
  normal too, not 0K. (`mshader` tested
  `hyprsunset_state == 1`, but that file has always been a letter `c|a|m`, so its
  temperature branch was dead.) `mshader` also `pkill`ed hyprsunset — NOT
  ported: process ownership stays with `hs_body`.
- **State files hold BARE IDS, never a path** (user requirement):
  `gamma_v` = 1..19, `shader_val_$s` = 0..200. The path is derived from the id
  at apply time. `_id()` still reads a stray path as its trailing id so a
  hand-edited file lands on a real shader, and `shader_main set` accepts
  `N`, `N.glsl` or a full path but only ever STORES `N`.
- Reload is diff-gated on the `shader.lua` target: apply compares the wanted
  `hl.config` block with the file and only writes + `hyprctl reload`s when it
  changed (`run_bg`, so `hyprctl`'s "ok" never lands on stdout where
  `dash_body` parses `status`). `hyprctl reload` used to be a separate `run()`
  in `brightness_body` after every key press.
- Ranges: gamma 0..19 (`GAMMA_MAX`), saturation 0..200 (`SAT_MAX`, 100 neutral).
- New CLI on `shader_main`: `gamma-up` / `gamma-down` / `gamma-set N` /
  `gamma-reset` (plus bare `up`/`down` aliases). `status` prints
  `"<mode> <sat>"` → `0 100`, `g 100`, `s 50`.
- `shaders_gen` is now load-bearing (it fills both pools) and stays as is. The
  old `~/.config/hypr/shaders/master.glsl` is left on disk but unreferenced.

**State-file migration (`1` → `s`, stale files removed in BOTH `dots/states`
and `$states`):** `shader_state_{d,l,n}` → `shader_m_{d,l,n}`; `states/gamma_s`
DELETED (its job is the mode letter now); `gamma_v` kept. `gen_flags_master`
seeds `shader_m_{n,d,l}` = `0` (and no longer seeds `gamma_s`).

**Callers updated:**
- `sources/brightness_body`: `gamma_main up|down` → `shader_main gamma-up|
  gamma-down`; new `_gamma_active()` gate — a down-press only dims through gamma
  when the CURRENT channel is in mode `g` or `m` (gamma is a live knob there),
  so mode `0` (or `s`) always dims backlight. Its own `hyprctl reload` is gone (diff-gated inside `shader_body`).
- `sources/dash_body`: `status` on-flag is now `mode ~= "0"`; saturation forced
  to 100 unless mode is `s`.
- quickshell: `ControlPanel.qml` (`shader_state_` → `shader_m_`, on = mode !== "0"),
  `WmCategory.qml` (`sf("shader_m")`, on = mode !== "0"), `SettingEngine.qml`
  (channel path `shader_m`, field `mode`, raw `"1"` → `"s"`, write `on ? "s" : "0"`).
- `BrightnessManager.qml`: the `bash -c` gamma-reset one-liner (it guarded on
  `$states2/gamma`, a file nothing has ever written — permanently a no-op) is now
  QML-native: `readBatch($states/gamma_v)` and only then run
  `shader_main gamma-reset`.

**Verified** (live `$states`, tmpfs copy in sync): status/restore/set/gamma-up ×N
at the 19 clamp/gamma-down to 0/toggle/off/gamma-set/`set <glsl path>`/out-of-range;
`shader.lua` tracks the mode and the id (`s 50` → `saturations/50.glsl`,
`g 3` → `gamma/3.frag`, clamp → `gamma/19.frag`, `m` → `master.glsl` with
`SATURATION_VAL 120.0 / GAMMA_VAL 2.0 / TEMP_VAL 2330.0` from the ids +
`hyprsunset_val`); `set <full path>` stores the bare id; inside `m` both `set`
and `gamma-*` re-bake and keep the mode; `set 0` / `set 100` / `gamma-down`-to-0
(and `m` with all three ids 0) drop the mode to `0` and empty `shader.lua`;
`m` with only gamma 4 → `SAT 100.0 / GAMMA 4.0 / TEMP 6500.0`; hyprsunset `c` or
`hyprsunset_val 0` → `TEMP_VAL 6500.0` (never 0K); repeat applies are silent and reload-free; `bri down`
dims through gamma in mode `g` AND `m`; dash parse checked for every mode. (`dash_cmd` itself still dies in this container on
`audio_main vol get` — no sound device here, pre-existing and unrelated.)
