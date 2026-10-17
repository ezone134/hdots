# zen-shell — widget system (sole painter)

> **Directive (current):** EVERY card and EVERY UI/surface paints through
> `zen-ui::widgets`. The declarative scene engine (`scene.rs`, `card_scenes.rs`,
> `surface_scenes.rs`), the per-frame value pool (`scene_values_raw`), and every
> hand-rolled `&mut Vec<Cmd>` drawer are being retired. There is ONE shape:
>
> ```rust
> fn <card|surface>(data …, area: Rect, pal: &Pal, hover: u32) -> Node
> ```
>
> plus an optional side-effect hook for scroll rects / cache refreshes.

The reference port is the **CPU card** (`cards.rs::cpu`, byte-parity pinned by
`cards::tests::the_cpu_widget_paints_exactly_like_the_retired_drawer`). Copy its
shape for every port. A port is DONE only when a parity test proves the new tree
emits the same `Cmd` bytes as the retired drawer/scene and the whole suite is
green.

## Layer law

Each layer may touch only what its row allows:

| Layer | May | May NOT |
| --- | --- | --- |
| Card/surface builder (`cards.rs`, `settings_frame.rs`, `settings_body.rs`, `cards/*.rs`) | `crate::widgets::*`, `crate::ui::*` palette helpers, `crate::icons::*` | `Cmd`, shell internals, literal `u32` colors |
| Widget core (`zen-ui/src/widgets.rs`) | `Cmd`, `zen-ui::text/ui`, geometry only | shell state |
| Shell dispatch (`shell/panels.rs::draw_lua_if_any`) | `layout()` + `render(sink, pal)`, side effects | card geometry |

## The three phases (unchanged)

Every tree goes through three passes:

1. **Build** — pure data → `Node` tree (rebuild every frame; palette hot-reload
   lands next frame for free).
2. **Layout** — `tree.layout(Rect{x,y,w,h}, &mut Measurer)`; geometry only.
3. **Render** — `tree.render(&mut Vec<Cmd>, &mut RegionSink, pal)`; emits
   `Cmd`s and registers hit regions.

Pure-anchor trees (no width-based flow, no measured right-align) use the
one-shot `widgets::paint_anchored(tree, v, x, y, w, h, pal)` (uses `NoMeasurer`
+ `NoHit`). Interactive trees lay out + render themselves with the shell's
`ShellSink` (real `region()` pipeline) — see `shell/panels/settings.rs`.

## Widget core primitive set

Kinds today (`zen-ui/src/widgets.rs`):

| Kind | Purpose |
| --- | --- |
| `Box` | filled rounded rect; children OVERLAY the content rect (inset per child margin) |
| `Row` / `Col` | flex flow (fixed + expand sizing, per-child margin, `gap`) |
| `Label` | one text run; `halign` Left/Center/Right, `valign` Top/Center/Bottom |
| `Button` | hover-styled fill + centered label + `key` region |
| `Toggle` | pill track + knob |
| `Slider` | `ui::slider` + drag `key` |
| `Graph` | series visualization |
| `Image` | `Cmd::Image` fitted |
| `Center {dx,dy}` | center-anchored single run (the retired `ui.heroc`/`textc`) |
| `Scroll {off}` | clip one child to the slot, offset |
| `Hit` | transparent hit region (paints nothing) |
| `Beads {dx,dy}` | bead-ring gauge (the retired `scene::bead_ring`) |

Builders compose: a "Header" is a `Box`/`Row` of `Label`s; a "Surface" is a
`Box`; a "Rows list" is `Scroll(Col(Row…)…)` with per-row `Box` (hover fill) +
`Hit`. New reusable primitives go here (with unit tests), never in cards.

## Palette hot reload (INVARIANT)

`Pal` is rebuilt from `$states2/shell_vars.json` every frame (inotify watcher).
Card builders derive tones through `crate::ui::{fg2,fg3,acc_tint,hover,…}` at
build time. **A literal `u32` color constant inside a card builder is a bug.**
Only widget-internal derived tones may consult the palette implicitly.

## Inventory & status

Legend: **W** widget tree (done), **D** direct `Vec<Cmd>` drawer (retired
next), **S** declarative scene const (retired next).

### Cards (65)

- **Widget (W, done):** `clock`, `pkgupdates`, `kblayout`, `cpu`
- **Native metric batch (D):** `mem`, `disk`, `gauges`, `gpu`, `branding`,
  `moon`, `viz`, `powerh`, `powerv`
- **Direct drawers (D):** `ticker`, `sshvpn`, `conninfo`, `journaltail`,
  `systemdunits`, `smarthealth`, `docker`, `procmon`, `sensors`, `thermal`,
  `fans`, `diskio`, `network`, `powerdraw`, `cpugpu`, `latency`, `batteryh`,
  `batteryv`, `water`, `activewin`, `micmeter`, `eyerest`, `pomodoro`,
  `weatherv2`, `todo`, `notes`, `customacc`, `audiorec`, `worldmap`
- **Declarative scenes (S):** `accent`, `alarms`, `appshortcut`, `audiodevice`,
  `bluetooth`, `calendar`, `clipboard`, `clipimg`, `compositor`, `countdown`,
  `currency`, `eq`, `expenses`, `lyrics`, `media`, `mirror`, `news`, `quote`,
  `recent`, `sliders`, `snippets`, `speedtest`, `system`, `toggles`,
  `wallpaper`, `weather`, `wifi`, `workspaces`, `worldclock`

### Surfaces / UIs

- **Widget (W, done):** `settings` frame (`settings_frame.rs`), `settings`
  network body (`settings_body.rs`)
- **Declarative scenes (S):** `bt_menu`, `dashboard`, `lock`, `notif`, `osd`,
  `pill`, `polkit_auth`, `power`, `settings`, `wallpaper`, `wifi_menu`,
  `workspace_switcher`
- **Settings body tabs still S:** misc, system, sound, pill, appearance

### Retirement targets (delete when the above read W)

- `scene.rs` (`CardScene`/`SurfaceScenes` engine, `draw_piece`), `card_scenes.rs`,
  `surface_scenes.rs`, `shell_scene.rs`
- `shell/panels.rs::scene_values_raw` (the value pool) + `SceneValues`
- every remaining `&mut Vec<Cmd>` card drawer + `cards::is_native`
- `draw_lua_if_any`'s per-card `if card_id == …` ladder → one dispatch table

## Stage plan (remaining)

Work remaining, in order:

1. **Metric batch** (D→W): mem, disk, gauges, gpu, branding, moon, viz,
   powerh, powerv — shared `Bar`, `Beads` already built.
2. **Direct drawers** (D→W): the long tail, batched, each parity-pinned.
3. **Scene cards** (S→W): re-author each as a builder, drop its const.
4. **Surfaces** (S→W): dashboard first (it hosts the card board), then the
   overlays/settings bodies.
5. **Delete** the engine + pool + registry files once nothing reads them.

## What we keep

`Cmd` renderer, glyph atlas, `Pal`, the full input pipeline (`region`, `click`,
`Effect` semantics become direct field writes in the owning builder).
