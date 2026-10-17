# Root daemon logic

Runtime tree: `/run/<rdmnb>` (`/run/rdmnb`)
Execution user: `root`

`/run` is tmpfs (empty at boot). The systemd unit's bootstrap script creates
`/run/rdmnb` as root — base `711` (traversable, not listable by others) — then
seeds the whole static skeleton the daemon needs, because the user session is
unprivileged and cannot write into it:

- copies this logic tree (`loop`, `main_body`, `udev_run`, `performance`, `powersave`, `README.md`)
- symlinks `utils` -> `sources/utils` (shared helpers, loaded by every entry point)
- copies `g_icons` -> `icons` (referenced by generated `.desktop` files)
- writes `99-power-source.rules` -> symlinks into `/etc/udev/rules.d`
- creates `states/` and `states2/` as `777` and seeds default plan config/flags
- `exec`s `loop`

World-writable state (the only parts the session may touch):

- `states/`  — plan config: `ps` `ap` `bp` `bmf` `amf` (`777`)
- `states2/` — trip-flags: `fire_count` `flag_cpu` `flag_disk_unmount`
  `disk_to_umount` `unmounter_status` `loop_is_running` `last_trigger` `pid`
  `disks_to_unmount_batch/` (`777`)

Everything else under the base stays owner-only (`700` logic scripts, `644`
plan files). The session (`sources/init_logic`) waits for `/run/rdmnb` to
appear, then seeds persisted `states_r` -> `states/` and sets the trip-flags in
`states2/`. The daemon itself derives its paths at runtime:

```lua
SCRIPT_DIR = arg[0]:match("@?(.*)"):gsub("/[^/]*$", "")
dofile(SCRIPT_DIR .. "/utils")
rstates  = SCRIPT_DIR .. "/states"
rstates2 = SCRIPT_DIR .. "/states2"
```

## Entry point scripts (thin wrappers, copied to `/run/rdmnb`)

- `loop`: `before_loop()` once, then blocks on `fire_count` inotify — the scheduling loop (systemd `ExecStart=/run/rdmnb/loop`)
- `udev_run`: `on_udev_trigger_function()` — handles power-supply udev events
- `main_body`: defines every daemon function below (dofile'd by both entries)

## Functions defined in `main_body`

- `main_logic_function()` — power source check + plan apply
- `init_func_2()` — pid-guarded main logic run
- `normal_apply_function()` — apply plan if daemon enabled
- `on_udev_trigger_function()` — udev-triggered plan re-apply
- `before_loop()` — creates `states`/`states2`, seeds defaults, writes pid, hardens perms
- `during_loop()` — queued work + disk unmount processing (one iteration)
- `loop()` — main scheduling loop (blocking, never returns)

Plan data files (`performance`, `powersave`) remain in this directory as key=value config.

Make changes here; the root daemon bootstrap re-copies them to `/run/rdmnb` on
the next boot. Do not treat the runtime copy as the source of truth.
