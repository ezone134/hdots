# Theme Definitions (`sources/themes`)

This document is the source of truth for the `sources/themes` data file and
the schema consumed by `theme_init` / `gen_theme_cards`.

## Overview

`themes` is a **name-keyed table of naked field arrays** — the theme name is
the key, the value is the theme's field array directly (no nested
`{ name, ... }` wrapper).  This lets `theme_ref(name)` pull a theme with an
instant `themes[name]` lookup — **no loop, no name matching**:

```lua
themes = {
    ["aura"] = { "1", "00", "l", <side block> },
    ...
    ["nord_aurora"] = { "83", "0000", "l", <dark side>, "l", <light side> },
}
return themes
```

## Header — 2 fields, one `flags` string

Every entry starts with exactly **2 header fields** (no stored mode):

| Pos | Field       | Notes                                            |
|-----|-------------|--------------------------------------------------|
| 1   | sort_index  | Unpadded integer (1, 2, 10, 100)                 |
| 2   | flags       | Variation string — see below                     |

`flags` is a single string, never two separate fields, and it encodes the
whole theme as one **variation** (not per-side bits in the array); it is
stored **expanded**, never collapsed — width 2 for singles, width 4 for duals:

- **`"00"` (all 0)** — single, core+base only: `extras=0 series=0`.  Only the
  extras/series fallbacks fill in the missing blocks (base is always stored).
- **`"11"` (all 1)** — single, full: `extras=1 series=1`; all blocks stored.
- **`"01"` / `"10"`** — single where one side-level block is missing.  `01` =
  core + base + series only (extras falls to defaults), `10` = core + base +
  extras only (series falls to the luminance-set fallback).
- **16 dual strings `"0000"`..`"1111"`** — the 4 chars are
  `d_extras d_series l_extras l_series`; each side keeps whatever subset its
  chars grant and the rest default.

**Mode is derived, not stored** (see [Derived modes](#derived-modes-quick-reference)):
flags width 4 → dual; flags width 2 → the single's dark/light answer comes
from its `bg` luminance via `is_dark_theme` (`theme_is_dark`).  No Lua global
is set for this — readers derive the answer on their own.

There are **no inline side flags**. The first side (single block, or the dark
block of a dual) starts directly after the header; in a dual the light block
follows the dark block **immediately** — its position is derived from the dark
block's own length (the table in
[Dual themes](#dual-themes)), hardcoded per case in `theme_init`.  The file
ships **307 keys — all real themes, every single variation and every dual
variation represented**, no fixture keys (see
[Exhaustive enumeration](#exhaustive-enumeration--24-possible-layouts)).

## Side block layout

Each *block* = one side (single, single dark, or the light side of a dual),
parsed with **that side's OWN flag chars**.  Maximum 22 fields.

### Core — always present (7 fields)

```
bg  bg2  bg3  fg  fg2  fg3  acc
```

| Pos (0-b) | Name       | Description                              |
|-----------|------------|------------------------------------------|
| 0–2       | bg bg2 bg3 | Surface family (base → raised → elevated) |
| 3–5       | fg fg2 fg3 | Text family (primary → muted → faint)     |
| 6         | acc        | Brand / active / selected color            |

`sfg` is **never stored** — it is derived at runtime from `acc` via
`getBestContrastColor(acc, sfg_d, sfg_l)` (`sources/sfg_calc`).

### Base — always present (4 fields)

The base tier (`acc_us`, `acc_hc`, `acc_ec`, `border`) is **stored in every
theme, on every side** — it is part of the theme's identity, never omitted and
never defaulted:

```
acc_us  acc_hc  acc_ec  border
```

| Pos (0-b) | Name     | Description                                    |
|-----------|----------|------------------------------------------------|
| 7         | acc_us   | Opposite-direction tint (dark sfg on dark side) |
| 8         | acc_hc   | High-contrast accent (≥ 4.5:1 text floor)     |
| 9         | acc_ec   | bg + 10% acc mix (sidebar selection bg)       |
| 10        | border   | Outline; may differ from acc                    |

`acc_us month` and `acc_hc` are authored per theme; relative to `acc`, the mix
pull is read-dark/light.  `acc_ec` is **always `$bg` with 10% of `$acc`
mixed in** (the file stores exactly that blend — never a raw `acc`), so the
Thunar left-side pane selection reads as a faint accent tint off the theme's
own background.

### Extras tier — only when THAT side's `extras` flag is `1` (5 fields)

```
hover  focus  disabled  sfg_d  sfg_l
```

| Pos (0-b) | Name     | Description                                  |
|-----------|----------|----------------------------------------------|
| 11        | hover    | Hover surface tint (default `#1c1a20`)       |
| 12        | focus    | Focus ring color (default `#6a6d75`)         |
| 13        | disabled | Muted/disabled foreground (default `#6a6d75`)|
| 14        | sfg_d    | On-accent dark text (default `#1a1a1e`)      |
| 15        | sfg_l    | On-accent light text (default `#f3f4f6`)     |

On an `extras=0` side the five fall back to `default_extras_dark` /
`default_extras_light` (see [Derived fields](#derived-fields)).  The base tier
above is **always** in-file.

### Series block — only when THAT side's `series` flag is `1` (6 fields)

```
series1  series2  series3  series4  series5  series6
```

| Pos (0-b) | Description |
|-----------|-------------|
| 16–21     | Categorical palette (dashboard charts)      |

Categorical palette for line graphs and dashboard charts — **not** accent-
derived.  Two colorblind-safe fallback sets are hardcoded in `theme_init`
(bright for dark backgrounds, deep for light backgrounds) and used when a
side's `series` flag is `0`.

## Field counts

Stored length follows the variation's expanded flag string: core (7) and base
(4) are **always** stored; only the extras tier (5, flag `e=1`) and series
(6, flag `s=1`) are optional:

| Mode  | Stored flags | Header | Core | Base | Tier | Series | Total |
|-------|--------------|--------|------|------|------|--------|-------|
| single (`d`/`l`), full  | `"11"` | 2 | 7 | 4 | 5 | 6 | **24** |
| single, core+series     | `"01"` | 2 | 7 | 4 | 0 | 6 | **19** |
| single, core+extras     | `"10"` | 2 | 7 | 4 | 5 | 0 | **18** |
| single, core-only       | `"00"` | 2 | 7 | 4 | 0 | 0 | **13** |
| dual (`b`), full        | `"1111"` | 2 | 7×2 | 4×2 | 5×2 | 6×2 | **46** |
| dual, core-only         | `"0000"` | 2 | 7×2 | 4×2 | 0   | 0   | **24** |

All 24 expanded layouts (8 singles + 16 duals) are file-storable now.  The
full per-combo totals (storable via their expanded flag string):

| Layout    | Stored flags | Total fields | Shipped example |
|-----------|--------------|-------------:|-----------------|
| single 0 0 | `"00"` | **13** | `volcanic_ash`, `porcelain` |
| single 0 1 | `"01"` | **19** | `storm_cloud`, `marble` |
| single 1 0 | `"10"` | **18** | `obsidian_glass`, `warm_sand` |
| single 1 1 | `"11"` | **24** | `aylur_material`, … |
| dual 0 0 0 0 | `"0000"` | **24** | `prism_split` |
| dual 0 0 0 1 | `0001` | **30** | `ember_edge` |
| dual 0 0 1 0 | `0010` | **29** | `storm_chase` |
| dual 0 0 1 1 | `0011` | **35** | `venus_veil` |
| dual 0 1 0 0 | `0100` | **30** | `neon_stride` |
| dual 0 1 0 1 | `0101` | **36** | `galaxy_glow` |
| dual 0 1 1 0 | `0110` | **35** | `mirage_hue` |
| dual 0 1 1 1 | `0111` | **41** | `quasar_pulse` |
| dual 1 0 0 0 | `1000` | **29** | `lunar_arc` |
| dual 1 0 0 1 | `1001` | **35** | `comet_tail` |
| dual 1 0 1 0 | `1010` | **34** | `pulsar_jolt` |
| dual 1 0 1 1 | `1011` | **40** | `supernova` |
| dual 1 1 0 0 | `1100` | **35** | `horizon` |
| dual 1 1 0 1 | `1101` | **41** | `graphite` |
| dual 1 1 1 0 | `1110` | **40** | `nord_graphite` |
| dual 1 1 1 1 | `"1111"` | **46** | `aylur_material`, … |

`gen_theme_cards` has no hardcoded total — it parses through `resolve` /
`d_side` / `l_side` (which slice by the expanded flag chars), so any of these
counts is a valid key.  Old bi-state rows that used to be "internal only" are
now authored as real keys (see
[Exhaustive enumeration](#exhaustive-enumeration--24-possible-layouts)).

## Dual themes

Duals are detected by their flags length (4 chars) and treated as
`mode == "b"` by readers — there is **no stored mode field**.  Layout is:

```
{ sort_index, flags("0000".."1111"), <dark side>, <light side> }
```

The dark side starts at **index 3** (directly after the 2-field header).  The
light side has **no fixed position** — it follows directly after the dark side:

- **Level 1 (outer):** the whole light block is pushed by the DARK side's own
  extras/series (dark block length).  For a full dark block (`1 1`) that's 22
  fields, extras-only (`1 0`) 16, series-only (`0 1`) 17, core+base-only
  (`0 0`) 11 → the light block starts at `3 + dark length` = **14 / 20 / 19 /
  25**.  There are no inline light-side flags to hide behind the dark block.
  A dual whose stored flags have dark chars `00`/`01`/`10`/`11` lands the
  light block at **14 / 20 / 19 / 25** respectively.
- **Level 2 (inner):** inside the light block, the light **series** block is
  pushed by the light side's OWN `l_extras` (`+5` when `l_extras=1`).

`14 / 20 / 19 / 25` and the `+5` level-2 shift are a **conceptual map only** —
the code never computes them at runtime.  `theme_init` implements every combo
as a **hardcoded array pull** (see [Parsing strategy](#parsing-strategy--hardcoded-array-pull)):
each `light_side_XXXX` function literally names its own indices, e.g.
`light_side_0011` reads core `14..21`, base `22..25`, extras `26..30`, series
`31..36`.  The two levels describe WHY the positions are what they are; the
functions are the ground truth.

### MASTER TABLE — every single and dual layout (all possibilities)

This is the **expanded** layout space `theme_init` implements, and — since the
variation conversion — **every row is file-storable** in its fully expanded
form: singles `"00"`/`"01"`/`"10"`/`"11"` and all 16 dual strings.  All values
verified through the parser in `theme_init` and match the absolute indices
hardcoded in the corresponding `dark_side_XXXX` / `light_side_XXXX` /
extractor functions.

Keep in mind the base tier (`acc_us`/`acc_hc`/`acc_ec`/`border`) is **always
stored** in every side — the block positions below already include it (each
side = core 7 + base 4 + optional tier/series).  Columns: single flags
(`extras series`) / dual flags (`d_extras d_series l_extras l_series`);
`dark len` = dark-side block length (Level-1 driver); then the ABSOLUTE array
position of every light-side piece: the light block `@`, the light extras
block `@` (Level-2 driver, only when `l_extras=1`), and the light series block
`@` (Level-2 result, only when `l_series=1`); `fields` = total array length.

| Layout    | 1   | 2   | 3   | 4   | dark len | light blk@ | extras block@ | series block@ | fields |
|-----------|-----|-----|-----|-----|---------:|-----------:|--------------:|--------------:|-------:|
| single `d` | 0 | 0 | — | — | 7 | — | — | — | 13 |
| single `d` | 0 | 1 | — | — | 7 | — | — | — | 19 |
| single `d` | 1 | 0 | — | — | 7 | — | — | — | 18 |
| single `d` | 1 | 1 | — | — | 7 | — | — | — | 24 |
| single `l` | 0 | 0 | — | — | 7 | — | — | — | 13 |
| single `l` | 0 | 1 | — | — | 7 | — | — | — | 19 |
| single `l` | 1 | 0 | — | — | 7 | — | — | — | 18 |
| single `l` | 1 | 1 | — | — | 7 | — | — | — | 24 |
| dual      | 0 | 0 | 0 | 0 | 11 | 14 |  — |  — | 24 |
| dual      | 0 | 0 | 0 | 1 | 11 | 14 |  — | 21 | 30 |
| dual      | 0 | 0 | 1 | 0 | 11 | 14 | 25 |  — | 29 |
| dual      | 0 | 0 | 1 | 1 | 11 | 14 | 25 | 30 | 35 |
| dual      | 0 | 1 | 0 | 0 | 17 | 20 |  — |  — | 30 |
| dual      | 0 | 1 | 0 | 1 | 17 | 20 |  — | 27 | 36 |
| dual      | 0 | 1 | 1 | 0 | 17 | 20 | 31 |  — | 35 |
| dual      | 0 | 1 | 1 | 1 | 17 | 20 | 31 | 36 | 41 |
| dual      | 1 | 0 | 0 | 0 | 16 | 19 |  — |  — | 29 |
| dual      | 1 | 0 | 0 | 1 | 16 | 19 |  — | 26 | 35 |
| dual      | 1 | 0 | 1 | 0 | 16 | 19 | 30 |  — | 34 |
| dual      | 1 | 0 | 1 | 1 | 16 | 19 | 30 | 35 | 40 |
| dual      | 1 | 1 | 0 | 0 | 22 | 25 |  — |  — | 35 |
| dual      | 1 | 1 | 0 | 1 | 22 | 25 |  — | 32 | 41 |
| dual      | 1 | 1 | 1 | 0 | 22 | 25 | 36 |  — | 40 |
| dual      | 1 | 1 | 1 | 1 | 22 | 25 | 36 | 41 | 46 |

Read it as the light side walking left-to-right: the light block starts at
`3 + dark len`; inside it the extras block (if any) fills
`light blk@ + 11 .. +15`; then the series block (if any) starts at
`extras block@ + 5` — or at `light blk@ + 7` when there is no extras block.
Example: `dual 0 0 1 1` → light core at 14, base 22–25, extras 26–30, series
starts at **31**; `dual 1 1 1 1` → light core at 25, extras 36–40, series at
**41**.

## Parsing strategy — hardcoded array pull (intentional)

This is the design.  `theme_init` does **no offset arithmetic at parse time**
— no length tables, no loops over flag chars, no `light_block = 3 + ...`.
Every field is pulled at a literal array index by a per-combo function:

- **Dispatch first** — the flag strings are stored expanded, so the
  dispatchers simply route to the matching per-combo function:
  `resolve()` (`"00"` → `extract_first_00`, `"01"` → `extract_first_01`,
  `"10"` → `extract_first_10`, `"11"` → `extract_first_11`), and `d_side()` /
  `l_side()` match the 16 `dark/light_side_XXXX` combos directly.  A single's
  dark/light answer is derived from its bg luminance via `is_dark_theme`
  (`theme_is_dark`) before the extractor runs.
- **Singles** — one parser per combo: `extract_first_00` /
  `extract_first_01` / `extract_first_10` / `extract_first_11` (all called
  from `resolve()` with `is_light`).  Each pulls
  `extract_first_base` (core `3..9`) plus `extract_first_base_extra` (base
  tier `10..13`), then `extract_first_extra_tier` (`14..18`) when the combo is
  `"10"`/`"11"`, and series via `extract_first_series` (`19..24`) for `"11"`
  or `extract_first_series_01` (`14..19`, series shifted left because the
  extras tier is absent) for `"01"`.  The **base tier is always stored**; when
  a side lacks the extras tier or series, only those fall back
  (`default_extras_*` on an `extras=0` side, series fallback on a `series=0`
  side) and the side-appropriate semantic set
  (`set_global_semantic_dark` / `set_global_semantic_light`) is applied.
- **Duals** — `d_side()` / `l_side()` are thin dispatchers over 16
  `dark_side_XXXX` / `light_side_XXXX` functions, one per 4-char combo.  Each
  function pulls **its own absolute indices** from the array —
  e.g. the light block of `light_side_0000` starts at `ref[14]`,
  `light_side_0100` at `ref[20]` (dark block has series → 17 fields),
  `light_side_1000` at `ref[19]`, `light_side_1100` at `ref[25]`, and the
  extras/series inside each are likewise written out literally.

Every row of the MASTER TABLE above corresponds 1:1 to one of these
functions, so the implementation is auditable line-by-line against this
document.  Trade-offs are accepted on purpose: 32 side functions + 4 combo
parsers are more verbose than a generic slicer, but positions can never
drift between the table and the code, and every read is O(1) with zero
machinery.  `gen_theme_cards` inherits the same hardcoded pulls by delegating
to `resolve` / `d_side` / `l_side` — it never re-slices on its own.

## Exhaustive enumeration — 24 possible layouts

The parsers understand **exactly 24 distinct expanded field layouts**: 8
singles (dark/light × extras × series) + 16 duals (`d_extras d_series l_extras
l_series`).  **All 24 are shipped as real keys, stored fully expanded** —
no collapsed extremes: singles use `"00"`/`"01"`/`"10"`/`"11"` and duals the
16 strings `"0000"`..`"1111"`.  **No `combo_*` fixtures are shipped** —
every one of the 24 layouts is covered by a hardcoded extractor /
`dark_side_XXXX` / `light_side_XXXX` function AND by at least one real key.

Verification of the layouts runs **through the real parser on the real
data**: every one of the 307 shipped themes is run through
`resolve` / `d_side` / `l_side` and every parsed variable is asserted non-nil
and a valid hex; one cross-check also proves the variation claim (flags are
always one of `00`/`01`/`10`/`11` for singles and a 4-char string (or `0`/`1`
shorthand) for duals)
and the derived mode (dual = 4-char flags; a single's dark/light answer must
match its bg luminance).  The tables in this document are the ground truth for
each expanded layout.

### Real themes are grouped by luminance, then variation, in the file

The 307 real themes are physically grouped in `sources/themes` under
**per-luminance banners** (`single dark` / `single light` / `dual`) and then
**sub-grouped by stored variation flag** with a count banner per group — so
the exact variation breakdown is visible at a glance:

| Group | Stored flags (count) | Layouts | Fields | Total |
|-------|----------------------|---------|-------:|------:|
| single dark  | `"00"`(31) `"01"`(30) `"10"`(30) `"11"`(31) | all 4 combos | 13/19/18/24 | **122** |
| single light | `"00"`(25) `"01"`(24) `"10"`(24) `"11"`(25) | all 4 combos | 13/19/18/24 | **98**  |
| dual         | `"0000"`(5) `"1111"`(18) + 14 intermediates (9×5, 7×4) | all 16 combos | 24–46 | **87**  |

The whole set is **divided evenly across the variations** — as even as integer
division allows — so no variation is left with a lone theme: each mode splits
its themes across the variation strings with the smallest possible spread
(e.g. single dark is 31/30/30/31, duals are 5 per bucket or the closest 4).
The base tier (`acc_us`/`acc_hc`/`acc_ec`/`border`) is **stored on every
side**; only the extras-tier five and series can be absent, in which case they
fall to the defaults ([Derived fields](#derived-fields)) — the
(`default_extras_*` / `default_series_*`) paths run on every `"00"` / `"01"` /
`"10"` and every dual combo whose side lacks a block.

### Singles — 8 expanded combos (mode `d`/`l` × extras × series)

Single blocks never shift: the header is 2 fields, the block always starts at
index `3`.  The derived dark/light answer (bg luminance via `is_dark_theme`)
changes which `series` fallback applies (bright for a dark background, deep
for light) and the "stranded" on-accent sfg defaults and the base-tier
accent-family defaults.  `file flags` shows which stored string produces each
row (all rows are shipped by at least one real key).

| mode | extras | series | expanded flags | stored? | fields | base tier (always stored)                    | extras-tier (stored when `extras=1`)        | series |
|------|-------:|-------:|----------------|---------|-------:|-----------------------------------------------|------------------------------------------------|--------|
| d    | 0 | 0 | `00` | `"00"`  | 13 | `acc_us #a0a4ac · acc_hc #9aa0a8 · acc_ec = bg+10%acc · border #6a6d75` | defaults (`default_extras_dark`)               | **bright** |
| d    | 0 | 1 | `01` | `"01"`  | 19 | (as above)                                    | defaults (`default_extras_dark`)               | (kept) |
| d    | 1 | 0 | `10` | `"10"`  | 18 | (authored)                                    | (kept from file)                             | **bright** |
| d    | 1 | 1 | `11` | `"11"`  | 24 | (authored)                                    | (kept from file)                             | (kept) |
| l    | 0 | 0 | `00` | `"00"`  | 13 | `acc_us #4a4d54 · acc_hc #3f434a · acc_ec = bg+10%acc · border #5b5e64` | defaults (`default_extras_light`)               | **deep** |
| l    | 0 | 1 | `01` | `"01"`  | 19 | (as above)                                    | defaults (`default_extras_light`)               | (kept) |
| l    | 1 | 0 | `10` | `"10"`  | 18 | (authored)                                    | (kept from file)                             | **deep** |
| l    | 1 | 1 | `11` | `"11"`  | 24 | (authored)                                    | (kept from file)                             | (kept) |

The base tier is **always stored in the file** — never defaulted.  On an
`extras=0` side only the **extras tier** (hover/focus/disabled/sfg_d/sfg_l)
falls back via `default_extras_dark` / `default_extras_light`; on an
`extras=1` side all five come from the file.  Real full themes (`"11"`) carry
both tiers stored.

Expanded block positions (per single side):

| extras series | layout |
|---------------|--------|
| 0 0 | `[3..9]` core + `[10..13]` base |
| 0 1 | `[3..9]` core + `[10..13]` base + `[14..19]` series |
| 1 0 | `[3..9]` core + `[10..13]` base + `[14..18]` extras |
| 1 1 | `[3..9]` core + `[10..13]` base + `[14..18]` extras + `[19..24]` series |

### Duals — 16 expanded combos (d_extras d_series l_extras l_series)

The 16 `dark_side_XXXX` / `light_side_XXXX` functions cover one layout per row
of the master table above.  Neither side is ever expected to be "full" — dark
may be a bare core while its light twin carries extras+series: the two sides
are parsed with their own flag chars and only meet at the two-level shift
described above.

Every dual combo is shipped full-width — `"0000"` (24 fields) through `"1111"`
(46 fields), no collapsed extremes — so each of the 16
`dark_side_XXXX` / `light_side_XXXX` functions is exercised by real key data.

## Hard-coded global constants — never stored

These are globals set in `theme_init` and are **identical for every theme**.
Each semantic color exists as a **dark set and a light set** (like the
extras-tier / series fallbacks): the surface currently being
materialized picks its luminance tier — singles via
`set_global_semantic_dark` / `set_global_semantic_light`
in the four `extract_first_??` parsers, duals via `d_side()` / `l_side()`:

| Variable  | Dark tier  | Light tier  | Role                |
|-----------|------------|-------------|---------------------|
| success   | `#9ece6a`  | `#2f9e5f`   | Green / ok          |
| warning   | `#ff9e64`  | `#e07b2a`   | Amber / caution     |
| danger    | `#f7768e`  | `#d6435f`   | Red / error         |
| info      | `#7dcfff`  | `#2f6fd8`   | Blue / information  |

Series fallback sets (hardcoded in `theme_init`):

| Set        | Colors (colorblind-safe, non-acc)                                   |
|------------|---------------------------------------------------------------------|
| bright     | `#4ea1ff #ffa94d #5fd68a #b07cff #ff79c6 #35d0e8`  (dark bg)     |
| deep       | `#2f6fd8 #e07b2a #2f9e5f #7a3fd6 #d63d9d #1793a8`  (light bg)    |

## Accent family — `acc`, `acc_us`, `acc_hc`, `acc_ec`

Order in the side block: **`acc`, `acc_us`, `acc_hc`, `acc_ec`**.

### `acc_ec` — Thunar left-side pane selection

`acc_ec` always equals `$bg` with **exactly 10% of `$acc`** mixed in — a faint
accent cast on the background surface.  The file stores that blend on every
side (base tier), never a raw `acc`.  The selection text (`sfg_ec`) is set by
downstream pipelines (never stored).

- Dark themes: selection text is **white**.
- Light themes: selection text is **dark**.

### On-Accent Foreground Rule (hard, authoring-time rule)

The sfg direction for each accent-family surface is **fixed** — the same on
every theme, no exceptions.  Authors pick the stored hex so these foregrounds
keep their contrast floors; the sfg color itself is resolved by the pipeline,
never stored per surface.  On-accent text uses
`getBestContrastColor(acc, sfg_d, sfg_l)` (`sources/sfg_calc`) — whichever of
`sfg_d`/`sfg_l` has the higher luminance contrast against `acc`.

| Surface | dark themes / dark side | light themes / light side |
|---|---|---|
| `acc` (on `$acc`) | light sfg (contrast-picked `sfg_l`) | dark sfg (contrast-picked `sfg_d`) |
| `acc_us` | **dark** sfg | **light** sfg |
| `acc_hc` | **light** sfg | **dark** sfg |
| `acc_ec` | **light** sfg (white) | **dark** sfg |

So `acc_us` is always the accent with the **opposite** foreground direction
from `acc`'s own sfg, while `acc_hc` and `acc_ec` always **share** `acc`'s
sfg direction.

### `acc_hc` contrast floor

`acc_hc` must keep ≥ 4.5:1 contrast against its sfg.  Setting `acc_hc = $acc`
is only safe when `$acc` itself already passes that floor.

### `border`

May optionally differ from `$acc`.  Stored in the base tier on **every** side
of every theme (never defaulted).

### `acc_us` purpose

Reserved for future use, but always **authored and stored** in the base tier
(keeps the popup's "base colors" row populated).  It must be brighter than
`$acc` on dark sides (so dark sfg stays readable) and darker on light sides
(so light sfg is readable).

## Pipeline auto-resolves `sfg_hc` / `sfg_ec`

`sfg_hc` and `sfg_ec` are **never stored** in the theme array.  The pipeline
picks them on the fly:

- **Dark themes:** `sfg_hc` = light text; `sfg_ec` = white.
- **Light themes:** `sfg_hc` = dark text; `sfg_ec` = dark text.

Theme authors should preview the selection surface to verify aesthetic fit.

## `sfg` — derived, never stored

`sfg` is **not a field** in the theme array.  It is computed at runtime by
`getBestContrastColor(acc, sfg_d, sfg_l)` (`sources/sfg_calc`), which returns
whichever of the side's `sfg_d` (dark text, `#1a1a1e`) / `sfg_l` (light text,
`#f3f4f6`) has the larger absolute luminance difference against the accent.
Themes override via `sfg_d` / `sfg_l` in the extras block.

## Derived fields (defaults when a side's blocks are absent)

The **base tier** (`acc_us` / `acc_hc` / `acc_ec` / `border`) is always stored
in the file and never defaulted.  Only the extras tier and series fall back.

### Extras tier — when `extras=0` on a side
`default_extras_dark()` / `default_extras_light()` are identical:

| Field     | Default             |
|-----------|---------------------|
| hover     | `#1c1a20`           |
| focus     | `#6a6d75`           |
| disabled  | `#6a6d75`           |
| sfg_d     | `#1a1a1e`           |
| sfg_l     | `#f3f4f6`           |

### Series — when `series=0` on a side
Hardcoded per luminance (bright on dark surfaces, deep on light surfaces):

| Set        | Colors (colorblind-safe, non-acc)                                   |
|------------|---------------------------------------------------------------------|
| bright     | `#4ea1ff #ffa94d #5fd68a #b07cff #ff79c6 #35d0e8`  (dark bg)     |
| deep       | `#2f6fd8 #e07b2a #2f9e5f #7a3fd6 #d63d9d #1793a8`  (light bg)    |

Both tiers are set on the side being materialized (globals for downstream
readers like `theme_save_hex` remain defined).  On a side that stores neither,
the extras and series have zero stored values (base is always stored).

## Layout rules

Modes are **derived, never stored** — there is no `d`/`l`/`b` field.
`themes[name][2]` is always the flagship string; its length decides the shape:
2 chars = single, 4 chars = dual (`de ds le ls`).  A single's dark/light answer
comes from its own bg luminance via `is_dark_theme` (`theme_is_dark`).

### Single theme (flags length 2)

```
themes["name"] = { sort_index flags
                   bg bg2 bg3 fg fg2 fg3 acc
                   acc_us acc_hc acc_ec border
                   [hover focus disabled sfg_d sfg_l]
                   [series1..series6] }
```

`flags` = `"00"` (core+base only), `"01"` (core+base+series), `"10"`
(core+base+extras), or `"11"` (everything stored).

### Dual theme (flags length 4)

```
themes["name"] = { sort_index flags
                   <dark side  11+[5]+[6]>
                   <light side 11+[5]+[6]> }
```

`flags` = one of the 16 strings `"0000"`..`"1111"` (`de ds le ls`), never
collapsed.  A side's chars select its blocks (`1` stores that block); `"0000"`
= both sides core+base only, `"1111"` = both sides full, `"0110"` = dark
series-only, light extras-only, etc.  The light block follows the dark block
directly — no inline light-side flags; its start offset is `3 + dark_length`,
hardcoded per expanded case in `theme_init` (14/20/19/25).  Each side keeps
its own palette, so they are parsed independently via
`d_side` / `l_side`.

The `theme_init` single-side extractors read core `ref[3..9]`,
base `ref[10..13]`, extras tier `ref[14..18]`, series
`ref[19..24]`, and series-in-`"01"` `ref[14..19]`.

## File conventions

- **Backup:** before editing `sources/themes`, copy to `sources/themes.bak`.
- **Order:** themes are ordered by `sort_index` (numeric), not file position.
- **Entries:** one keyed field array per line for readability.
- **Legend comments** at the top of the file describe the layout.
- Real themes are grouped under per-luminance banner comments
  (`single dark` / `single light` / `dual`) with a **sub-banner per stored
  variation flag** (`00` / `01` / `10` / `11`; duals `0000`, intermediates,
  `1111`) carrying that group's theme count.
- **Uniqueness:** any side that stores a full block (extras + series, i.e.
  single `"11"`, dual `"1111"`, or a dual side whose chars include both `e` and
  `s`) must carry a block **unique across the file** — no two themes may share
  the same stored colour block (case-insensitive, so `#5E81AC` == `#5e81ac`).
  It ensures every palette represents its theme id (no hidden twins).
- `return themes` at the end (enables `loadfile`-based loading).

## Verification (run after every edit)

```sh
# 1. field counts
dofile("/home/tw/.config/hdots/sources/themes")
local fc = {}
for _, fields in pairs(themes) do fc[#fields] = (fc[#fields] or 0) + 1 end
for k, v in pairs(fc) do print(k, v) end

# 2. loadfile + side parses
sources=/home/tw/.config/hdots/sources luajit -e '
sources = os.getenv("sources")
dofile(sources.."/utils")
dofile(sources.."/theme_init")
resolve("nord_aurora")
assert(#themes.nord_aurora[2]==1 or #themes.nord_aurora[2]==4)
d_side(); l_side()
print("OK")'

# 3. card emission (dry run; writes to $states2/themes)
sources=/home/tw/.config/hdots/sources states2=/tmp luajit -e '
sources = os.getenv("sources")
states2 = os.getenv("states2")
dofile(sources.."/utils")
dofile(sources.."/theme_init")
dofile(sources.."/gen_theme_cards")
print("cards OK: "..#all_themes.." themes")'

# 4. variations — every theme resolves with a valid flags length + derived mode
sources=/home/tw/.config/hdots/sources luajit -e "
sources = os.getenv('sources')
dofile(sources..'/utils')
dofile(sources..'/theme_init')
local n=0
for name in pairs(themes) do
  assert(not name:match('^combo_'), 'fixture key leaked: '..name)
  local flags = themes[name][2]
  local L = #flags
  if L == 4 then
    assert(not flags:match('[^01]'), 'bad dual flags: '..name)
  elseif L == 1 then
    assert(flags == '0' or flags == '1', 'bad dual shorthand: '..name)
  else
    assert(flags == '00' or flags == '01' or flags == '10' or flags == '11',
           'bad single flags: '..name)
  end
  resolve(name)
  assert(found == 1, 'resolve failed: '..name)
  if L == 2 then
    local bg = themes[name][3]
    local mode = is_dark_theme(bg) and 'd' or 'l'
    assert(mode == 'd' or mode == 'l', 'mode derivation failed: '..name)
  end
  n=n+1
end
assert(n==307, 'expected 307 themes, got '..n)
print('real themes OK')"

# 5. uniqueness — no two full-blocks (extras+series stored) may be identical
sources=/home/tw/.config/hdots/sources luajit -e "
sources = os.getenv('sources')
dofile(sources..'/themes')
local seen = {}
for name, t in pairs(themes) do
  local sides = {}
  local flags = t[2]
  local L = #flags
  if L == 4 then
    local de, ds, le, ls = flags:sub(1,1)=='1', flags:sub(2,2)=='1',
                           flags:sub(3,3)=='1', flags:sub(4,4)=='1'
    local dn = 11 + (de and 5 or 0) + (ds and 6 or 0)
    if de and ds then sides[#sides+1] = { t, 3, 2+dn } end
    if le and ls then sides[#sides+1] = { t, 3+dn, #t } end
  elseif L == 1 then
    if flags == '1' then
      sides[#sides+1] = { t, 3, 24 }
      sides[#sides+1] = { t, 25, #t }
    end
  elseif flags == '11' then
    sides[#sides+1] = { t, 3, #t }
  end
  for _, s in ipairs(sides) do
    local a, b = s[2], s[3]
    local key = {}
    for i = a, b do key[#key+1] = s[1][i]:lower() end
    key = table.concat(key, ':')
    assert(not seen[key], 'duplicate full block on: '..name)
    seen[key] = name
  end
end
local u = 0
for _ in pairs(seen) do u = u + 1 end
print('uniqueness OK: '..u..' unique full-side blocks')"
```

Every field array must be a **file-storable** total from **Field counts** above
(13/19/18/24 for singles, and a member of {24,29,30,34,35,36,40,41,46} for
duals).  The `sort_index` values should be contiguous.

### Derived modes (quick reference — readers derive, nothing stored)

Each reader (`scheme_body`, `fetch_color_scheme`, `gen_theme_cards`) computes
the mode itself from `flags` length + `$bg` luminance via `is_dark_theme`
(`theme_is_dark`):

| flags length | shape    | derived mode |
|--------------|----------|-----------|
| 2            | single   | bg luminance dark → `d`, else `l` |
| 4            | dual     | `b` |