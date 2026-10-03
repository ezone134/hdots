# Handoff — Settings UI polish ✅ + Dashboard card polish ✅ (batches 1+2)

_Date: 2026-09-14 · Status: settings DONE · card batches 1+2 DONE (compiles, 106 tests green) · batches 3-4 remain_

---

## 0. DONE — Dashboard card polish batches 1+2 (latest session)

Landed exactly per the plan below, plus the gauges family. Read `CHANGELOG.md`
2026-09-14 for the full entry. In short:

- `ui.rs`: `caption`/`caption_r`/`caption_c` (Regular-weight small text),
  `text_c_mono`, status tokens `OK`/`WARN`/`DANGER`/`INFO`, `pct_color()`.
- `cards/shared.rs`: `card_header` (title + optional icon + optional mono
  meta), `metric_row(label_x, y, right_x, …)`, `card_empty`.
- Migrated to helpers: cpu, mem, disk, thermal, network, gpu,
  battery_frame, ticker, speedtest.
- Color sweep: legacy `0x44aaff`/`0xffb86c`/dual RED-GREEN gone; accent
  header icons dropped to fg2 (thermal/gpu/battery/ticker/speedtest).
- Numerals: gauge %s / pkg count / GPU util / kbd layout / weather temp →
  Display hero; forecast temps → mono. **Weather digit bug fixed**: big temp
  was `{glyph} 22°C` in one string → digits rasterized in the icon font;
  now split icon + text_hero.
- shell::RED/GREEN now alias ui::DANGER/ui::OK.

### Batch 3+4 remaining (unchanged from plan below)
- lists: topproc, procmon, docker, pkgupdates (count done), news, todo,
  notes, recent, clipboard, clipimg, sshvpn, sensors
- media/misc: media, lyrics, viz, eq, sliders (0xffb86c warm mix still
  there), mirror, worldmap, calendar, clock, activewin, apps, accents,
  toggles, wifi, bluetooth, quote, currency, wallpaper, powerv/powerh
  headers, diskio title (kept title-only by design)
- panels/overlays pass: system_panels.rs, app_panels.rs, overlays.rs,
  session.rs — still plain-Light headers, none of the settings treatment.

---

## 1. DONE — Settings UI polish (previous session)

Design direction locked in: **minimal / flat / professional** (Linear·VS Code·Grafana feel,
NOT Material). Flat tiles, slim switches, hairlines, generous whitespace, one accent.

### `src/shell/panels/settings.rs`
- **Header close button** (key 1): pill → flat 30×26 r6 square, `hover_hl`, muted `fg2` ✕.
- **Sidebar**: bg tint `0x08` (was `0x0d`), 1px `ui::hairline` divider at `x = SIDE-1`.
  Nav rows: selected = flat `hover_hl` + `acc` icon + `fg` label + **2px accent rail at
  x=0** (was acc-tinted pill + right-edge dot). Hover = `hover` bg, `fg` text.
  Unselected labels now `fg2` (was full `fg`).
- **`settings_row`** (Network rows, Accent toggles, Scrim): flat neutral icon tile
  `ui::hover(pal)` r7 (no acc-tint mix), icon = `acc` when ON else `fg3`; label always
  `fg` (hover no longer recolors label); **switch slimmed 42×24 → 38×20** r10,
  16px knob, OFF track `mix(bg, fg, 0.14)` (neutral, not fg-tinted); hover rect r8.
- **`settings_toggle_row`** (Pill rows): same treatment, **switch 38×22 → 36×18** r9,
  14px knob, 24px icon tile r6, label baseline +1, label color `fg2`→`fg` on hover only.
- **Pill tab layout** (`layout_settings_pill`): everything below the fixed "PILL" header
  now shifts by `let so = -self.pill_scroll;` — sliders (64/108/150 rows), 5 behavior
  toggles (`196 + so + i*40`), expand tip (`236 + so`), "PILL ITEMS" label (`404 + so + tip`),
  16 reorder rows (`422 + so + tip + i*40`), battery-% row (`422 + so + 16*40 + tip`),
  Position row follows bp_y. Header stays fixed.
- **Apply changes button** (Appearance, key 217): solid `pal.acc` bg (hover `mix(acc,bg,0.22)`),
  h 38, r8.

### Scroll for Settings → Pill (was missing entirely)
- `shell/mod.rs`: new field `pub pill_scroll: f32` (next to `appearance_scroll`).
- `shell/state.rs`: init `pill_scroll: 0.0`.
- `shell/scroll.rs`: `pill_scroll_by(notches)` (40px/notch, clamped) + `pill_max_scroll()`
  = `422 + tip + 16*40 + 76 − (target_h − 164)`. Mirrors the layout constants.
- `app/input.rs` wheel handler: new block after the Appearance one —
  `Mode::Settings && settings_tab == Pill → self.shell.pill_scroll_by(steps)`.

### Drag-reorder fix (`shell/input.rs` `drag_pill`)
Was mapping rows to the STALE offsets `392 + i*40` (layout had moved twice: 410/428 era →
422, plus tip shift and now scroll). Now: `ry0 = 422 − pill_scroll + (expand_on_hover ?
SETTINGS_EXPAND_TIP_H : 0)`, row band `ry-2 .. ry+36`. Scroll-aware and correct again.

**Verified:** `cargo check` clean. Not yet run live (user should test wheel-scroll on Pill
tab + drag reorder + hover states).

---

## 2. NEXT — Dashboard card polish (agreed, not started)

User's words: *"minimalistic design… uniform look… can use multiple fonts… professional"*.
Reference images live in `minimalist design/` (I can't see images — encode principles only):
generous whitespace, muted secondary labels, ONE accent on live data, hairline separators,
tabular/mono numerals, no fills-of-the-day.

### Agreed plan (user approved this order)
1. **Typography unification** — do first, biggest win.
2. **Spacing pass** — same edit, same batch (rows currently hand-placed → dead gaps).
3. **Calm colors** — separate pass AFTER user eyeballs 1+2 (most subjective).
4. **Card chrome (per-corner radii + 1px hairline)** — only if cards still feel flat.

### Shared helpers to add (suggest `cards/shared.rs` or `ui.rs`)
```rust
/// Uniform card header: title at (pad, y+10) size 11.5 fg — the ONLY header style.
/// Optional right-aligned mono meta (units/peak/count) in fg3.
fn card_header(v, pal, x, y, w, pad, title: &str, meta: Option<(String, bool /*mono*/)>)
/// Metric row: label fg2 left, mono value right, vertical pitch passed in.
fn metric_row(v, pal, x, y, w, pad, label: &str, value: &str, val_col: u32)
```
Rules: title = `ui::title` 11.5 `fg`, never colored, no icon pills (thermal card currently
tints its icon `pal.acc` — drop to `fg2`); numbers ALWAYS mono
(`ui::text_mono` / `ui::text_r_mono`, already exist in ui.rs lines ~117-133);
labels `fg2`; meta/units `fg3`; section headers could use `Ff::Display` for hierarchy
(multi-font, still quiet).

### Survey notes (from the 5 cards read)
- `cpu.rs` — good bones: `ui::title` + `ring_beads` + right col of label/`text_r_mono`
  rows at pitch 22, centered on gauge. Keep; just normalize pad (14) + row pitch vs height.
- `mem.rs` — same pattern as cpu (good). Swap bar acc-colored — keep accent here (live data),
  but track is `ui::hover` ✓.
- `network.rs` — graph `ui::pulse` ok; **colors noisy**: `dl_col = mix(0x44aaffff,…)` +
  `up_col = pal.acc` → change dl to `fg2`, keep ONE colored line (accent) or both muted with
  colored only under cursor/peak. Speed row centered — fine.
- `disk.rs` — bar colors `RED / 0xf9e2afff / pal.acc` for >=90 / >=75 / else → keep warn
  thresholds, but normal state should be `pal.acc` only (already is) — consider muted
  `fg2` mount labels (already `fg`), row pitch 24 fine, value string mixes too much info.
- `thermal.rs` — hand-rolled header (icon + title at different offsets) → use card_header;
  per-cell tinted rects ok; zone temp colors RED/ORANGE thresholds ok (real states).
- Card bg (dashboard.rs ~line 174): `raised/raised_hl | dash_card_alpha`, single `r: lt.card_r`.
  Hairline outline = pass 4 if chosen.

### Remaining cards to survey (~45): gauges, cpugpu, system, batteryh, batteryv, powerv/h,
weather(2), worldmap, mirror, sliders, eq, media, lyrics, news, quote, ticker, currency,
pkgupdates, speedtest, docker, topproc, procmon, clipboard, clipimg, notes, todo, apps,
recent, calendar, clock, kblayout, activewin, sensors, sshvpn, viz, wifi, bluetooth,
accent, toggles, gpu, diskio, network… — grep `ui::title(` to find stragglers with
hand-rolled headers.

### Suggested batches (cargo check + user live-test between each)
1. shared.rs helpers + cpu/mem/disk/thermal/network (the 5 surveyed)
2. gauges family (gauges, cpugpu, gpu, system, battery*, power*, diskio, speedtest)
3. lists (topproc, procmon, docker, pkgupdates, news, todo, notes, recent, clipboard…)
4. media/misc (media, lyrics, viz, eq, sliders, mirror, worldmap, calendar, clock, …)

### Key APIs / constants for resuming
- `ui.rs`: `title` (Ui/Semibold), `text_mono`, `text_r_mono`, `text_c_hero` (Display/Medium),
  `fg2 = mix(fg,bg,.55)`, `fg3 = mix(.82)`, `hover = mix(bg,fg,.07)`, `hover_hl = .12`,
  `raised = .05`, `hairline = .10`, `acc_tint`.
- Palette: `pal.fg / sfg / acc / bg`; `RED = 0xff6b6bff` (shared.rs), amber `0xf9e2afff`.
- Icons: `ICON_*` in icons.rs; `scale.s()` px→scaled, `scale.fs()` font.
- Card draw signature: `draw_x_card(&mut self, v, x, y, w, h, pal)`, header ≈26px, pad 12-14.
- Settings nav keys 200-204/219; content keys 1-227; `SETTINGS_SIDEBAR=180`;
  `SETTINGS_EXPAND_TIP_H=44`.
- ⚠️ `state.rs.bak` / `dashcards.rs.bak` / `.cards_groups.bak` exist — ignore them.
