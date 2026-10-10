# Phase 5 `anim:` — handoff

Status: **CLOSED 2026-10-02.** The blocker below is fixed, the pilot is migrated
in the live `shell.ron`, and the full write-up lives in `CHANGELOG.md`'s top
entry. This file is kept as the design record; the original status line read
"engine is built, wired, and green, one known blocker".

Build clean, zero warnings. Tests **508 passed / 0 failed / 0 ignored** (was 501
+ 1 ignored).

## What landed

### `src/scene/anim.rs` (new)

- `Curve` — `linear` / `out_cubic` / `in_out_cubic` / `out_back`, snake_case in
  RON, default `out_cubic`.
- `Anim { ms: f32, curve: Curve }` — default `ms: 180`. `ms: 0` means snap.
- `AnimTable` — one entry per **binding source string**, each holding a resting
  `target` plus an optional `Cell { from, to, start, dur, curve }`.
- `step` / `set_now` / `settling` / `clear`, plus a test-only `in_flight`.
- 13 unit tests driving time explicitly.

Design decisions worth keeping:

- **Keyed by binding source, not item path.** Two items reading `nav.tab` are
  tracking one value and must move together; a per-item key would let them
  disagree for a frame. Inside a `Repeat`, substitution has already rewritten
  each row's source to its own `item.<field>` spelling, so rows are distinct
  without threading a path through `draw_piece`.
- **First frame of a key rests at its target.** Only *subsequent* target changes
  animate. This is why `Entry` keeps `target` even while settled — without it
  every frame looks like a first sighting and nothing ever moves.
- **Retarget restarts from the current value**, so an interrupted animation
  never snaps backwards.
- `EPS = 1e-4` so float noise cannot keep a cell alive forever.
- `set_now` is called once per surface by the shell, so every animated field in
  a frame reads the same instant.

### `src/scene.rs`

- `Val::Anim { val: Box<Val>, anim: anim::Anim }` (untagged, so a bare number is
  still `Lit` and a bare string is still `Expr`).
- `Val::num_anim(&self, vals, &mut AnimTable)`.
- `Val::num`, `constant`, `binding`, `compile`, `fills_w`, `fills_h`, `span_v` all
  understand the wrapper — `num` deliberately reads the **target**.
- `CardScene::draw` kept its exact signature (≈153 call sites) and now delegates
  to new `CardScene::draw_with_anims(..., anims: &mut AnimTable)`.
- `draw_piece` / `draw` take one extra `&mut AnimTable`.
- 57 position reads (`ix`, `iy`, `dx0`) switched to `num_anim`. `w` / `h` /
  `font_size` / `k` / `d` were left on `num` — those are measured or not
  positions.
- `SceneCache::take_reloaded` and `ShellSceneCache::take_reloaded` — the shell
  drops its easing state when scene text changes under it (a rebind must not
  ease from the old target).

### `src/scene/validate.rs`

- `LoadIssue::AnimatedMeasuredField { path, detail }` + `Display` arm.
- `Validator::anim_axes` refuses `anim:` on `w`, `h`, `font_size`. Measure reads
  the target, so an animated extent would lay out at its final size and draw
  there every frame — declared, validated, and silently inert.
- Hit-key fingerprint now derives identity through `binding()` / `constant()`,
  so animated and plain spellings of one binding share a key.

### `src/shell/mod.rs`, `src/shell/state.rs`, `src/shell/panels/mod.rs`

- `Shell::scene_anims: AnimTable`, initialised in `state.rs` beside
  `scene_stores` for the same reason (an animation is measured across frames).
- `draw_shell_surface` stamps `set_now` once, passes the table to
  `draw_with_anims`, and clears on reload. `draw_card_scene` does the same
  against the card cache.

### `src/app/mod.rs`

- `App::anim_timer` + `sync_anim_timer`, called at the end of `maybe_render`
  *after* the draw (the draw is what starts the easing, so checking earlier
  would arm one frame late on first movement). 16 ms, self-stopping.
- Without it an `anim:` would freeze mid-move on an idle screen and jump on the
  next keypress.

## The blocker — RESOLVED 2026-10-02, and the diagnosis was half stale

The original claim: `#[serde(untagged)]` on `Val` cannot deserialize
`anim: (curve: out_cubic)`.

It was probed in isolation against **ron 0.9**, where untagged enums buffer
through `Content` and a nested unit-variant enum field does not survive that
round trip:

- `(val: "a.b", anim: (curve: out_cubic))` → whole `Val` fails to match.
- Nested `Option<CurveUnit>` → parses, but silently yields `None`, i.e. the
  curve is dropped with no diagnostic.
- A nested *struct* field in the same position works fine, so it is the enum
  specifically — which is why `(val: …, anim: (ms: …))` parsed and half the
  syntax appeared to work.

**Re-probed against the pinned ron 0.12.2, untagged carries `curve:` fine.** The
measurement was correct; the version it was taken on is not the one in
`Cargo.toml`. The lesson is the one worth keeping: a probe is only as good as
the version it ran on, so re-run it before believing a note twice.

The hand-rolled impl is still what shipped, for the two reasons that survive:

1. **Untagged's error is "data did not match any variant of untagged enum Val"**
   for every real mistake — the same reason phase 2 hand-wrote `PropSet` and
   `SceneComponent`. A dropped `anim:` line is a plausible typo.
2. **`anim: ()` does not match untagged at all.** The whole `Val` fails, pointing
   at the item rather than at the animation, and "all defaults" is the natural
   way to write it.

What shipped: `impl<'de> Deserialize<'de> for Val` calling `deserialize_any`,
with arms `visit_f64` / `visit_i64` / `visit_u64` → `Lit`, `visit_str` → `Expr`,
`visit_map` → `Anim` (reads `val` + `anim`, **errors** when either is missing,
**ignores** unknown keys). All of these parse, authored syntax unchanged:

```
(val: "a.b", anim: (ms: 90, curve: out_cubic))   → Anim { ms: 90.0, OutCubic }
(val: "a.b", anim: (curve: out_back))            → Anim { ms: 180.0, OutBack }
(val: "a.b", anim: (ms: 0))                      → Anim { ms: 0.0, OutCubic }
(val: "a.b", anim: ())                           → Anim { ms: 180.0, OutCubic }
1.5                                             → Lit(1.5)
"a.b"                                           → Expr("a.b")
```

**`#[serde(untagged)]` stays on `Serialize`.** Removing it breaks five existing
round-trip tests at once (`value_token_ron_parses_and_roundtrips` and friends
re-read ron this crate emits), because a tagged serialize writes
`Anim(val: …)`. `an_animated_val_survives_a_write_and_a_reread` now pins that
half.

## Remaining work — all four done 2026-10-02

1. ~~Hand-roll `Deserialize for Val`, then un-ignore the test.~~ Done, and the
   `#[ignore]` is gone (`508 passed / 0 ignored`).
2. ~~Confirm `parse_scene_ron` tolerates the manual impl.~~ It uses a non-default
   ron config, so the whole suite was re-run — and it earned its keep: five
   round-trip tests caught the missing `untagged` on `Serialize`.
3. ~~Migrate the pilot.~~ `shell.ron`'s polkit cursor is now
   `Surface(x: (val: "polkit_cursor_x", anim: (ms: 90)), …)`, verified by
   `every_live_scene_validates_clean_against_the_shells_own_pool`, which parses
   and validates the real config tree.
4. ~~Update `QUICKSHELL_MODEL.md`, `CHANGELOG.md`, `NEXT_STEPS.md`.~~ Done — and
   the roadmap's "done when pill expand + `dash_settling` are declared" is now
   recorded as **unreachable** rather than pending: pill morph is *surface*
   geometry in `Shell::anim` / `cur_w` / `cur_h` (no scene item owns it), and the
   dashboard glide is derived inside the Rust `dash_grid` ink off
   `packed_layout` (`card_anim` + the `edit_settling` flag).

## Traps hit

- The `SceneItem` enum is tagged, so items are `Surface(...)`, **not**
  `(Surface(...))`. Wrapping in extra parens gives a misleading
  `ExpectedIdentifier` far from the real error.
- `SceneValue` has no numeric variant — a number in a pool is
  `SceneValue::Ring(f32)` (or `Fader`, both read by `SceneValues::scalar`).
- `Val::Anim` is reached only via a tuple/struct form. `{ val: … }` and
  `Anim(val: …)` are still wrong; `anim:` with a struct-curve no longer fails.
- **`ColorToken` has no `text` variant** — `color: "text"` in a fixture is an
  `ExpectedIdentifier` pointing at the string, and `Surface.color` is the one
  field with no `#[serde(default)]`, so a test fixture must supply it.
- **Four tests were in the tree without `#[test]`** (`anim:`'s three plus
  `ron_roundtrip_parses`): they compiled, ran nothing, and left one `dead_code`
  warning. Two of them were also broken — wrong `color:`, and a missing
  required field. A test without `#[test]` is a comment.
