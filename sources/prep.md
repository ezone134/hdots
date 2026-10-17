# misc/ — personal boot script

This directory ships with the dotfiles but stays separate — only used by the
owner's boot script (`prep` → `boot_logic`). Other users won't use it; they
get the Lua arrays in `sources/` (`symlink_dirs`, `symlink_files`, `ptc_array`,
`init_array`) via `dofile`.

## Why separate?

- `misc/` arrays = your personal boot setup (folders, files, ptc, perms)
- `sources/` Lua arrays = what other users get when they use your dotfiles

No overlap. No shared arrays. Intentional.

## Recovery note

If the boot script runs `chown -R tw:tw /`, it strips root ownership + the
setuid bit from `sudo` and every other setuid binary. **Fix `sudo` first** —
once sudo works again, use it to repair everything else (`su`, `passwd`,
`mount`, `ping`, ownerships, …).

`boot_logic` calls `fix_sudo_perm` (ran as root, after the chown steps) which
restores `sudo` to `root:root 4755` and echoes a notice if it had to.

## Files

| File | Purpose |
|------|---------|
| `vars` | Variable definitions (`global_vars`, `root_vars`, `none_root_vars`) |
| `functions` | Boot logic functions (`pb_load`, `link-all`, `boot_logic`, `fix_sudo_perm`) |
| `folders` | `folders_pairs` array — directory symlinks |
| `files` | `files_pairs` array — file symlinks |
| `ptc_array` | `ptc_pairs` array — point-to-contents links |
| `others` | chmod/chown arrays, `boss_a` |
| `prep` | Entry point — sources `vars`, calls `boot_logic`; prints the `Already x -> y` / `Linked x -> y` pair report (tee'd into `/tmp/boot.log`). `-q`/`--quiet` silences it (and passes `-q` down to `link`/`link_file`/`ptc`) |
| `fix_perms` | Standalone permission fixer |
| `env_bash` | Bash env loader |
| `opensuse` | zypper aliases/functions |
| `common` | chroot helper |
