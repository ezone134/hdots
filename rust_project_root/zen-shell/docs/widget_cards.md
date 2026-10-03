# Widget Cards — expanded dashboard catalog

A second wave of dashboard cards: clock variants, astro/solar widgets,
focus + task helpers, identity and now-playing cards, weather lookahead,
and a disk-usage map. Every card is a self-contained drawer like the
first-wave cards — no new renderer code, no new data services beyond the
forecast fetch (which reuses the existing weather thread).

Each card degrades gracefully when squeezed into a small span and is a
no-op (renders a placeholder) while its data isn't ready or its config is
missing (e.g. no lat/lon → SunArc shows a hint).

## Clock & date variants

### AnalogClock (`analogclock`)
Round clock face: 60 tick marks, hour + minute hands from the live
clock, a small sweeping accent second hand. No interaction — a clean
divider-friendly card like the digital Clock.

### DigitalClock (`digitclock`)
Big `HH:MM:SS` with a blinking colon and the date line; taller than the
compact Clock card, tuned for wide single-row spans. Ticks from the
system clock at draw time (nothing to poll).

### DateCard (`datecard`)
Pure typographic date: weekday, large day-of-month, month + year, plus a
tiny season glyph. Reads the live date; no buttons.

## Astro cards (pure local math, no network)

### Moon (`moon`)
Synodic lunar phase computed from the date: illumination %, waxing /
waning arrow, phase name (new → full → new), and a hand-drawn disc where
the lit fraction matches the phase. Pure arithmetic — no API, 0 CPU.

### SunArc (`sunarc`)
Daylight arc for the configured lat/lon (the same `[[weather]]` location
the forecast uses): sunrise → solar noon → sunset plotted as an arc,
with the sun's current position marked and rise/noon/set times labelled.
Uses a standard sunrise equation — no network.

## Focus & productivity

### Pomodoro (`pomodoro`)
Work / short-break / long-break timer: a progress ring around the
remaining time, phase label, and Start / Pause / Reset buttons. Tick and
phases are computed from an `Instant` watermark at draw time — the card
needs no background timer of its own.

### StickyNotes (`stickynotes`)
The notes store rendered as sticky-note tiles (slightly rotated,
pin-top corner), newest-first. Interacting is the same as the Notes
card — the composer is shared, so a note edited here shows up there and
vice-versa.

## Identity & media

### Profile (`profile`)
Who / where / how long: username, hostname, distro, kernel, uptime
(`/proc/uptime`), and the current session + locale. One-time static
reads, cached for the panel lifetime.

### Album (`album`)
Now-playing as a mood card: the album art fills the backdrop (with a
palette tint pulled from the cover), title / artist / playback state over
it, and small track-progress / play controls using the existing media
transport.

### BatteryRing (`battring`)
Circular battery gauge: 360° arc that drains with the charge, centered
big %, a bolt while charging, and watts + time-remaining line. Uses the
same upower data every battery card uses.

## Lookahead & disk

### Forecast (`forecast`)
7-day lookahead from Open-Meteo delivered by the existing weather thread
(`daily` block): per-day glyph, high/low, and precipitation probability.
Falls back to a "waiting…" placeholder like the Weather card.

### Treemap (`treemap`)
Disk usage drawn as a nested-rectangle squarified map of the mount:
top-level dirs sized by their real byte usage (bounded first-level walk
on a detached thread, same pattern as the weather thread), colour-ramped
by size. A hover-cursor rectangle shows the directory name + Live
Gigabytes readout; refresh re-scans.

## Card rules

- Same grid behaviour, drag / resize / park, as every other card.
- `needs()` stays empty for every card except Forecast (reuses the
  weather resource) and Treemap (a bounded one-shot scan thread).
- No new timers, no polling, no free-running draw — each card repaints
  only when the shell already would (hover, 3 s services beat, 60 s
  clock beat, or its own button presses).