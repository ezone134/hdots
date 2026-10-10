# zeneq — a hand-written 10-band equalizer for PipeWire

> ## STATUS: HALTED (paused)
>
> This project is deliberately on hold and will be **continued in the future**.
> The code is complete and verified (build + 6/6 tests, and the plugin loads
> correctly under a real `dlopen`). What stopped it: this machine's PipeWire
> install can't run **any** LADSPA plugin — its own
> `libspa-filter-graph-plugin-ladspa.so` fails to link because the installed
> `libspa-0.2.so` lacks `spa_log_topic_enum` (a package version mismatch in
> this container). Once the PipeWire/spa packages are re-synced (system
> update), resume here: install per the "Install" section and re-verify the
> filter appears under `wpctl status`.

A from-scratch parametric equalizer written in Rust as a LADSPA plugin.
PipeWire runs it as a filter on ALL system audio.

```
audio in ──► 31Hz ─► 62Hz ─► 125Hz ─► … ─► 16kHz ─► Master ─► audio out
              (ten biquad filters in a row)
```

## How it works (short version)

- `src/biquad.rs` — one biquad filter. Every EQ band is one of these. The
  coefficients come from the RBJ Audio EQ Cookbook formulas.
- `src/eq.rs` — ten bands wired in series + a master gain, plus named presets
  (Bass Boost, Vocal, Rock, …).
- `src/ladspa.rs` — the thin "packaging" that lets PipeWire load the `.so`
  and call it like any LADSPA plugin.

LADSPA ports (in order): `Input` (audio), `Output` (audio),
then the ten band gains in dB, then `Master` in dB.

## Build

```sh
cargo build --release
# → target/release/libzeneq.so   (this machine: /acc/data/persist/user/.cargo/target/release/libzeneq.so)
```

## Install (make PipeWire use it)

```sh
cp config/filter-chain.conf.d/10-zeneq.conf ~/.config/pipewire/pipewire.conf.d/
systemctl --user restart pipewire pipewire-pulse
```

PipeWire finds the plugin by name (`plugin = "zeneq"`) in its LADSPA search
path. This machine's target dir already contains a `zeneq.so` symlink; point
the daemon at it with a systemd override:

```sh
systemctl --user edit pipewire     # add:
```

```
[Service]
Environment=LADSPA_PATH=/acc/data/persist/user/.cargo/target/release
```

Then `wpctl status` should show a new **Filter** ("zeneq 10-band EQ").

> **Note — known blocker on this machine (2026-09-09):** PipeWire's own
> `libspa-filter-graph-plugin-ladspa.so` fails to link here because the
> installed `libspa-0.2.so` does not export `spa_log_topic_enum` (a version
> mismatch in this container's packages). Until the PipeWire/spa packages are
> re-synced (system update), **no** LADSPA plugin — ours or anyone's — can be
> loaded by this PipeWire. The plugin itself verified fine via direct
> `dlopen`. After the system package mismatch is fixed, this config should
> work as-is.

## Test

```sh
cargo test
```

The tests verify the DSP math honestly: a 0 dB band is transparent, +6 dB
actually boosts, −6 dB actually cuts, and a band change between runs takes
effect.

## Next steps (live control)

Right now the band gains are fixed by the config file. The natural next step
is to control them live — the filter-chain node exposes control ports that
`pw-cli set-param` can poke while audio is playing (that's how a shell
dashboard equalizer would drive these bands).