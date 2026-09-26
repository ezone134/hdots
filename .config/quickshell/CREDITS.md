# Credits

This project contains design and code adapted from other open-source projects.
We're grateful for their work — here's who to thank.

## Tide Island

- **Author:** enhaoswen
- **Repository:** https://github.com/enhaoswen/Tide-island
- **License:** GPL-3.0

### What we adapted

The **drawn battery icon** (`components/BatteryIcon.qml`, used in the pill bar's
expanded + collapsed battery chips) is a port of the battery shape from Tide
Island's `qml/island/SwipeCustomInfoLayer.qml`:

- Real battery silhouette: translucent rounded body, animated fill that tracks
the level, and a tip nub on the right (lights up at 100%)
- Level number drawn **inside** the battery instead of a font glyph + separate
  percentage text
- Charging (plugged-in) state: solid body + FA bolt glyph (`\uf0e7`) next to
  the number; body turns red below 20%

The port keeps our own data source (`PowerManager` UPower singleton), makes the
colors theme-aware via `Theme` (white body in dark mode, theme fg in light
mode), and reuses our `Txt` primitive for the inner text. The AC plug/unplug
OSD toast idea (bolt / battery-empty) was already covered by `PowerManager.acStateChanged`
→ `StateController.osdPower`.

The drawn-icon approach was then generalized into a shared **`StatusIcon`**
component (`components/StatusIcon.qml`): one API (`type` / `level` / `active` /
`muted` / `color`) that renders wifi signal arcs, the bluetooth rune, volume
waves + mute slash, brightness sun, and mic with the same state/level-aware
Canvas technique (the pill bar's wifi / bluetooth / mic / volume chips now use
it instead of font glyphs). The set was later extended to every remaining
status icon in the shell — sun/moon, gear, sliders, grid, bell, power, clock,
arrows, bolt, monitor, media controls, check, bulb, coffee, music, refresh,
lock — so the bar, control center, OSD toasts, sliders and audio panel all
draw icons with the same monochrome vector approach.

---

*This file is maintained manually — add an entry whenever code or design is
adapted from another project.*
