# this file is $states/themes.md
# Theme Definitions (`states/themes`)

This document defines the schema/rules for the theme data file
`states/themes`.

The file holds **one bash array per theme** — e.g. `nord=(...)`, `aura=(...)`,
holding the actual color fields. The name on its own line is the same as the
variable name. There is **no** `raw_themes` master list.

Each array's **first field is an unpadded integer sort index** (readable by
`sort -n` up to 1000+ themes). Callers order themes by that index; the bare
variable name is the public theme name (O(1) lookup, no name-scanning or
hardcoded offsets).

- **Sourced by:** `fetch_color_scheme -> resolve_theme()`
- **Cards:** `gen_theme_cards -> read $states/themes`, sort by the index field
  via `sort -n`, load each via `_load_fields` (fills the `FIELDS` array) then
  `set -- "${FIELDS[@]}"`.

## Field Format — fixed layout, no extras short-cut

Since the schema was consolidated, **every side block is the full 27-field
record** — there is no shorter "extras=0" variant anymore. The `extras` field
(header position `[2]`) is always `1`.

```
Single: name=(sort_index theme_mode extras sfg_mode bg bg2 bg3 fg fg2 fg3 acc acc_hc acc_ec acc_us border hover focus disabled success warning danger info sfg_d sfg_l series1 series2 series3 series4 series5 series6)
        (30 fields)

Dual:   name=(sort_index b extras <dark block 27 fields> <light block 27 fields>)
        (57 fields — the light block follows immediately; no per-side extras flag)
```

- `sort_index` = unpadded integer (1, 2, 3, 10, 100 — never zero-padded); used
  for `sort -n` ordering. `sort_index` **is** the array's first field and is
  always stripped/shifted before reading theme fields.
- `theme_mode` = `d` (dark-only) | `l` (light-only) | `b` (both)
- Each field (including hex colors) is quoted so `#` is not treated as a
  comment: `nord=("107" "b" "1" "l" "#2e3440" ...)`.

## The 27-Field Side Block

Every single-theme block and every dual sub-block contains exactly **27 color
fields**, in this strict order:

```
sfg_mode bg bg2 bg3 fg fg2 fg3 acc acc_hc acc_ec acc_us border
hover focus disabled success warning danger info sfg_d sfg_l
series1 series2 series3 series4 series5 series6
```

1. `sfg_mode` — selected-foreground mode (`d` dark / `l` light), drives the
   on-accent foreground (see below).
2. Background family — `bg`, `bg2`, `bg3` (surface → raised surfaces).
3. Foreground family — `fg`, `fg2`, `fg3` (primary text → muted → faint).
4. Accent family — `acc`, `acc_hc`, `acc_ec`, `acc_us` (accent + derived shades).
5. `border` — outline color; may differ from `acc`.
6. Interaction states — `hover`, `focus`, `disabled`.
7. Semantic status — `success`, `warning`, `danger`, `info`.
8. On-accent text — `sfg_d` / `sfg_l` (the colors selected by `sfg_mode`).
9. Chart palette — `series1..series6` (colorblind-friendly series for graphs
   and dashboard spools).

The `extras` header field is always `1`; there is **no** optional-block
variant. `load_default_extras()` in `theme_init`/`gen_theme_cards` is retained
only as a defensive fallback — with the fixed layout it is never exercised.

## Color Guidelines

- The **selected item's** foreground color is resolved on the fly by the
  pipeline from `sfg_mode` — it is **not** a stored field. No `sfg`/`$sfg`
  field exists in the theme array.
- `sfg_mode`:
  - `d` → the selected-foreground must be **dark** (used when `$acc` is too bright).
  - `l` → the selected-foreground must be **light** (used when `$acc` is too dark).
- `sfg_d` / `sfg_l` supply the actual colors selected by `sfg_mode`.
- `$acc_hc` / `$acc_ec` = accent-derived shades used for secondary accent
  surfaces. `acc_hc` must look derived from `$acc` — never plain white/black.
  `acc_ec` is a background-tint, not an accent shade (see per-mode rules below).
- **Dark themes** (`theme_mode=d`, or dark side of `theme_mode=b`):
  - `acc_hc` — mid-dark shade of `$acc`, must stay **dark** enough that light
    text keeps **≥ 4.5:1** contrast
  - `acc_ec` — same as `$bg` with **10–20% of `$acc`** mixed in (a subtle
    tint of the background, not a distinct accent shade); it should read as
    `$bg` with only a faint accent cast
- **Light themes** (`theme_mode=l`, or light side of `theme_mode=b`):
  - `acc_hc` — lighter tint of `$acc`, must stay **light** enough that dark
    text keeps **≥ 4.5:1** contrast
  - `acc_ec` — same as `$bg` with **10–20% of `$acc`** mixed in (a subtle
    tint of the background, not a distinct accent shade); it should read as
    `$bg` with only a faint accent cast
- `acc_us` — sits **next to** (immediately after) `acc_ec` in each base block.
  Its foreground is the opposite direction from `acc_ec`:
  - **Dark themes/sides:** `acc_us` must be **brighter** than `$acc` (a light
    tint toward white), so that **dark** text keeps good contrast on it.
  - **Light themes/sides:** `acc_us` must be **darker** than `$acc` (a dark tint
    toward black), so that **light** text keeps good contrast on it.
  - `acc_us` is always derived from `$acc` (may equal `$acc` only if `$acc`
    already has the required brightness/darkness for contrast).
- `$border` can optionally differ from `$acc` if it suits the theme.
- Setting `acc_hc=$acc` is only safe when `$acc` itself already passes that
  field's contrast floor (≥ 4.5:1 vs light text on dark themes / dark text on
  light themes). Otherwise derive darker (dark themes) or lighter (light
  themes) shades — never ship the bare bright/dark `$acc`.
- **Pipeline auto-resolves foreground for acc_hc / acc_ec:** `sfg_hc` and
  `sfg_ec` are separate from `sfg_mode` / `sfg_d` / `sfg_l` (which control
  the foreground on `$acc`). When the pipeline renders text on `$acc_hc` or
  `$acc_ec` backgrounds, it picks `sfg_hc` / `sfg_ec` on the fly — these
  are **never stored** in the theme array. The rule:
  - **Dark themes:** `sfg_hc` is always **light text**; `sfg_ec` is always **white**.
  - **Light themes:** `sfg_hc` is always **dark text**; `sfg_ec` is always **dark text**.
  `sfg_ec` is **never stored** in the theme array — downstream pipelines
  (e.g. the Thunar left-pane selection) set it, so theme authors never
  author `sfg_ec`. The pipeline guarantees readability of text on `$acc_ec`
  but not aesthetic fit — theme authors should preview the selection surface.
- `focus` — keyboard-focus ring color. Derived from `$acc` blended 20% toward
  `$fg` so it reads brighter on dark themes and deeper on light themes.
- `disabled` — muted/disabled foreground, `$fg` blended 65% toward `$bg`.
- `hover` — hover-state surface tint (or author override).
- `series1..series6` — chart palette. Two pre-tuned ranges are used:
  dark sides get the brighter series, light sides get the deeper variants so
  series stay distinguishable on both backgrounds.

## `acc_ec` — Thunar Left-Side Pane Selection

- `$acc_ec` is the **background of the selected row in Thunar's left-side
  pane** (the sidebar / places-tree selection).
- Its foreground (`sfg_ec`) is set by downstream pipelines — it is **never
  stored** and **never authored** in the theme array. Theme authors only
  guarantee that `acc_ec` provides enough contrast for it.
- **Dark themes:** the pane selection text is always **white**; `acc_ec`
  equals `$bg` with 10–20% of `$acc` blended in.
- **Light themes:** the pane selection text is always **dark**; `acc_ec`
  equals `$bg` with 10–20% of `$acc` blended in.
- **`acc_hc` follows the same rule:** its foreground (`sfg_hc`) is also
  pipeline-set — always light text on dark themes, always dark text on light
  themes — so `acc_hc` must keep the same **≥ 4.5:1** contrast floor against
  it.
- All `acc_ec` values follow the bg+tint rule — keep it when adding or
  editing themes.

## Layout Rules

### Single theme

The array holds 30 fields **including** the `sort_index` first field (the name
is the variable name):

```
name=(sort_index theme_mode 1 sfg_mode bg bg2 bg3 fg fg2 fg3 acc acc_hc acc_ec acc_us border hover focus disabled success warning danger info sfg_d sfg_l series1..series6)
```

### Dual theme

Dual themes use `b` as their mode flag. The dark side block occupies the first
30 fields (header + 27), and the **light side block follows immediately** (27
fields, `+27` offset) — no per-side `extras` flag, no pipe/`l` divider.

```
name=(sort_index b 1 <27 dark fields> <27 light fields>)      (57 fields)
```

Both sides maintain their own `sfg_mode` and palette, so they are parsed
independently via `d_side` / `l_side`.

## Independent Contrast Evaluation

Both the dark side and light side of dual themes maintain their own distinct `sfg_mode` and expansion parameters, requiring isolated side-by-side parsing for correct theme card rendering.

## Field Count Reference

| Mode | Fields |
|------|--------|
| single (`d`/`l`) | 30 |
| dual (`b`) | 57 |

## Backup / Revert Convention

Before making changes to `states/themes`, **always copy the current file** to
`$states/themes.bak` so changes can be reverted:

```sh
cp "$states/themes" "$states/themes.bak"
```

This gives a safe rollback point in case a batch of edits needs to be undone.

## Array Organization (sort order)

Themes are **ordered by their `sort_index` field** (via `sort -n`), not by
position in the file. The file groups themes into contiguous, comment-labeled
blocks by category for readability; that grouping does **not** drive ordering.
The `sort_index` field is the single source of ordering truth.

### Singles (`theme_mode` = `d` / `l`)

1. Single themes with explicit overrides — `aura`, `aylur_material`, `cyberpunk_teal`.
2. The remainder, grouped by accent-vs-foreground relation (default, matched accent, light pastel).

### Duals (`theme_mode` = `b`)

All duals share one fixed block-based layout (57 fields); the comment blocks
that label internal sections are organizational only.

## Notes

- This is the **source of truth** for the `states/themes` file.
- All helper scripts live at `/acc/common/hdots/sources/helpers/`.
- When making changes to `states/themes`, we may also need to update:
  - `/acc/common/hdots/sources/helpers/theme_init`
  - `/acc/common/hdots/sources/helpers/gen_theme_cards`
- Do **not** edit `/acc/common/hdots/sources/helpers/fetch_color_scheme` — it
  only consumes `resolve_theme` + `d_side`/`l_side`, which stay compatible.
- See also: `/acc/common/hdots/sources/helpers/gen_theme_cards.md`

## Verification (always run after any edit)

1. **Field counts** must match the table above (single 30; dual 57).
   ```sh
   bash -n states/themes
   python3 - <<'EOF'
   import re
   from collections import Counter
   c = Counter()
   for l in open("states/themes"):
       m = re.match(r'^(\w+)=\((.*)\)\s*$', l.strip())
       if not m: continue
       name, body = m.group(1), m.group(2)
       f = [x.strip('"') for x in body.split()]
       if len(f) < 5: continue   # skip headers
       c[(f[1], len(f))] += 1
   for k, v in sorted(c): print(k, v)
   EOF
   ```
   Every theme must be one of: `('d'|'l', 30)`, `('b', 57)`. The `sort_index`
   values must be contiguous `1..N` (a gap is acceptable only if it predates
   the current edit and is deliberate).

2. **Cards still emit** — run `gen_theme_cards` and confirm valid JSON with
   exactly **287** themes:
   ```sh
   hypr_sources=... states=... states2=... bash \
     "$hypr_sources/helpers/gen_theme_cards"
   ```
   (Result is written to `$states2/themes`.)

## Side-Start Offsets (computed, no hardcoding)

Because the layout is now completely fixed, the light-side start offset is
**constant**: position `30` (0-based index 30) in `theme_init` (`parse_second_*`)
and in `gen_theme_cards` (`load_light_*`). There is no `_l_pos` formula and no
shifted variant anymore — the `load_light_base_shifted` /
`parse_second_base_shifted` variants were removed. If the base field count ever
changes (27 → N), update the fixed offsets and the header comment in
`theme_init`.

Never reintroduce a stored `sfg`.