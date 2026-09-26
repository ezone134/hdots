# gen_theme_cards

`gen_theme_cards` emits JSON cards for the Rust shell theme picker.

- **Reads:** the keyed `themes` table from `sources/themes` (via
  `dofile(sources .. "/theme_init")`, which loads the themes file) and
  `is_dark_theme` from `sources/theme_is_dark`.
- **Emits (per side):** `bg`, `fg`, `acc`, `sfg` (derived via
  `getBestContrastColor` from `sfg_d`/`sfg_l`, `sources/sfg_calc`),
  `border`.

Keep this helper clean; update rules here.

## Why a separate expansion

The card emitter only needs `bg`, `fg`, `acc`, `sfg`, `border`.  It delegates
to `resolve` / `d_side` / `l_side` from `theme_init` (the same parsers the
theme pipeline uses), so it stays in lock-step with the theme pipeline and
never hardcodes offsets or the merged-flag mapping itself.

## Discovery & ordering

`fetch_theme_names()` iterates the keyed `themes` table (`pairs`) and
collects each key — the theme name — in arbitrary order; the picker ranks by
`sort_index` downstream.  Each field array is pulled in O(1) via
`theme_ref(name)` (`= themes[name]`, no loop).

## Field slicing (delegated to theme_init)

The side block layout is defined in `theme_init` and documented in
`themes.md`.  Modes are derived, never stored.  Header is **2 fields**;
`flags` at header `[2]` is the theme's **variation string**: singles are
`"00"` (core+base only) / `"01"` (series) / `"10"` (extras) / `"11"` (full),
duals are the 16 four-char strings `0000`..`1111` (`de ds le ls`).  No inline
side flags;

| Block           | Size | Present when                |
|-----------------|------|-----------------------------|
| core            | 7    | always  (`bg bg2 bg3 fg fg2 fg3 acc`) |
| base            | 4    | always  (`acc_us acc_hc acc_ec border`) |
| extras          | 5    | THAT side's flag char = `1` (`hover focus disabled sfg_d sfg_l`) |
| series          | 6    | THAT side's flag char = `1` (`series1..series6`) |

`gen_theme_cards` never applies this table itself.  It does **not** re-slice —
it calls the same hardcoded-array-pull parsers the pipeline uses:

- Variation dispatch: `resolve()` maps `"00"` → `extract_first_00`, `"01"` →
  `extract_first_01`, `"10"` → `extract_first_10`, `"11"` →
  `extract_first_11`; `d_side()` / `l_side()` match the 16 `XXXX` dual combos.
  A single's dark/light answer comes from its bg luminance via `is_dark_theme`
  (`theme_is_dark`); a dual is detected by flags length 4.  `theme_init` sets
  **no** `theme_mode` global — every reader (this emitter included) derives the
  answer itself.  Every single/dual variation is shipped by a real key, so
  every parser is exercised by data.
- Single: block starts at index **3**; `resolve` dispatches (through the
  variation flags string) to the expanded combo parsers
  `extract_first_00` / `extract_first_01` / `extract_first_10` /
  `extract_first_11`, which call `extract_first_base` /
  `extract_first_base_extra` / `extract_first_extra_tier` /
  `extract_first_series` / `extract_first_series_01`, each a literal-index
  pull (no computed offsets).  The base tier is always stored.
- Dual (`b`): the dark block starts at index **3**; the light block has **no
  fixed position** — it is pushed by the dark side's extras/series:
  `light offset = 3 + dark length` = 14 / 20 / 19 / 25, but the offset is
  never computed at runtime — each `light_side_XXXX` parser hardcodes its own
  indices (`light_side_0000` reads core at `14..21`, `light_side_0100` at
  `20..27`, `light_side_1000` at `19..26`, `light_side_1100` at `25..32`,
  base/extras/series likewise literal).  `l_side` only dispatches by the flags
  string; all 16 combos documented in `themes.md`.  Both sides are parsed
  with their OWN flag chars, so dark/light sides may differ.
- `sfg` resolution: `getBestContrastColor(acc, sfg_d, sfg_l)` (`sources/
  sfg_calc`) — never a stored field, identical to the pipeline.

Keeping this delegation means the card emitter stays in lock-step with the
pipeline and has zero offset knowledge of its own.

## Derived mode emission

`gen_theme_cards` derives and emits `"mode"` on every card (Rust ignores
unknown fields, so this is additive-only for the shell):
`flags` length 4 → `"b"` (dual); otherwise `"d"` / `"l"` from the single's
bg luminance through `is_dark_theme`.  There is no global `theme_mode` to
rely on — the emitter computes these itself (same for `scheme_body` /
`fetch_color_scheme`).

## Output shape

```json
# Dual theme
{"name":"...","mode":"b","dual":true,"dark":{"bg","fg","acc","sfg","border"},"light":{...}}

# Single theme
{"name":"...","mode":"l","dual":false,"dark":{"bg","fg","acc","sfg","border"},"light":null}
```

Result is written to `$states2/themes`.  A single's palette always lives under
`"dark"` (the shell reads that key for singles), `"light"` is `null`.  The
`mode` value is informational for tooling/debugging.