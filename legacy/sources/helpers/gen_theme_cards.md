# gen_theme_cards

`gen_theme_cards` emits JSON cards for the Rust shell theme picker.

- **Reads:** each bare `name=(sort_index ...)` array from `$states/themes`
- **Emits (per side):** `bg`, `fg`, `acc`, `sfg` (resolved via `sfg_mode`), `border`

This is the rule document for `/acc/common/hdots/sources/helpers/gen_theme_cards`.
Keep that helper file clean; update rules here.

## Why a separate expansion

The card emitter only needs `bg`, `fg`, `acc`, `sfg` (resolved via `sfg_mode`),
and `border`. It does **not** need the full base + overrides that `theme_init`
expands. So it uses its own simple, efficient field slicing per side instead of
running the full resolve path.

## Discovery & ordering

Themes are bare arrays; there is **no `th_` prefix** and **no `raw_themes`
list**, so discovery reads `$states/themes` directly. `fetch_theme_names()`
extracts each `name=(sort_index ...)` line by slicing everything before `=(`,
then (for completeness) the names are emitted in file order — the picker ranks
them by `sort_index` downstream.

## Fixed field slicing logic

The themes file uses a **fixed 27-field side block**; there is no "extras
short-cut" anymore (`extras` header field is always `1`). Indices below are
0-based array offsets:

| Side | `sfg_mode` | `bg` | `fg` | `acc` | `border` | `sfg_d` | `sfg_l` |
|------|-----------|------|------|-------|----------|---------|---------|
| dark (first block) | 3 | 4 | 7 | 10 | 14 | 22 | 23 |
| light (second block, duals) | 30 | 31 | 34 | 37 | 41 | 49 | 50 |

- Dark side: `load_first_base` (offsets 3/4/7/10/14) + `load_first_extras`
  (offsets 22/23).
- Light side (duals, fixed `+27`): `load_light_base` (offsets 30/31/34/37/41)
  + `load_light_extras` (offsets 49/50).
- `sfg` resolution: if `sfg_mode == d` → `sfg_d`, else → `sfg_l` (never a
  stored field).
- The old `load_light_base_shifted` / `load_light_extras_shifted` variants and
  the `_l_pos` formula were removed; the light block always follows the dark
  block at the fixed `+27` offset.

The stored `sfg` color field was removed from the schema; the selected foreground
is always derived from `sfg_mode` + `sfg_d`/`sfg_l` at emit time.

## Output shape

```
# Dual theme
{"name":"...","dual":true,"dark":{"bg","fg","acc","sfg","border"},"light":{...}}

# Single theme
{"name":"...","dual":false,"dark":{"bg","fg","acc","sfg","border"},"light":null}
```

Result is written to `$states2/themes`.