# Root daemon logic

This directory is the persistent source for the root-owned daemon scripts.

- Source tree: `/home/tw/.config/hdots/rd_logic/`
- Runtime tree: `/tmp/r_daemon/`
- Execution user: `root`
- The source tree is copied to `/tmp/r_daemon/` during initialization.
- The daemon is launched and executed from `/tmp/r_daemon/`, not from the persistent source tree.

Every LuaJIT entry point resolves its runtime directory and sources the shared utilities with:

```lua
SCRIPT_DIR = arg[0]:match("@?(.*)"):gsub("/[^/]*$", "")
dofile(SCRIPT_DIR .. "/utils")
```

At runtime, that is `/tmp/r_daemon/utils`. The utilities provide the filesystem, process, sleep, permission, path, and command helpers used by the daemon.

## Script roles

- `loop`: long-running scheduling loop.
- `before_loop`: one-time daemon initialization.
- `during_loop`: queued work and disk-unmount processing.
- `cpu_set`: applies the current power plan on demand.
- `udev_run`: handles power-supply udev events.
- `main_body`: shared power-plan and trigger logic.

Make changes in the persistent `rd_logic` directory. The next initialization sync copies the updated files to `/tmp/r_daemon/`; do not treat the runtime copy as the source of truth.
