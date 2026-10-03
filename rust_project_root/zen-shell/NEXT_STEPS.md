# zen-shell — session notes (open items only)

> Merged through 2026-09-13: everything that has landed is recorded in
> `CHANGELOG.md` (entries 2026-09-09 / 09-10 / 09-11, the 09-12 session, and
> the 09-13 world-map entries — card, app overhaul, then always-on map /
> pinch-zoom). This file keeps ONLY open items, hard rules, and reference pins.
>
> **The declarative architecture and its build order live in
> `QUICKSHELL_MODEL.md`** — read that before starting engine work.

## ▶ RESUMED 2026-10-02 — declarative-core rewrite: validation, models/delegates, component props, `store` AND `anim:` landed

**The goal is one line, and it is in `QUICKSHELL_MODEL.md` §0: _Rust is the
backend and the Wayland renderer; every pixel and every decision is declared in
`.ron`._ The line is **not** "how much code is in Rust" — it is *who decides a
coordinate*. Rust publishes values; the scene decides what they mean.

The phases are ordered by what gates what:
- **1 (landed)** — the scope overlay. `.ron` becomes expressive.
- **2 (landed)** — component `props`. The last cheap piece; makes 4 affordable.
- **3 (demoted 2026-10-01)** — dirty-set propagation + cached expression trees.
  **Measured, and it is not load-bearing**: the expanded live `shell.ron` is
  ~170 binding fields, a `compile()+eval()` is 43 ns (13 ns with the `Arc`
  held), so a frame pays **~7.6 µs** re-resolving everything and ~14.7 µs
  cloning the surfaces it draws — **0.13% of a 60 Hz frame**. The cache would
  save ~5 µs. Do it *with* 4, not before it; only its **validator** half
  survives as independently useful (a store scope is a CLOSED name set, so a
  card-local pool finally becomes checkable). See the correction in
  `QUICKSHELL_MODEL.md` §2.
- **4 (landed)** — `store` + `SceneAction::Set`. **This is what makes the
  boundary reachable.** Settings nav selection moved out of Rust: the
  `settings_nav` rows write `Set(name: "nav.tab", value: <index>)` and Rust's
  six-arm `match key { 200 => … }` table is deleted.
- **5 (landed 2026-10-02)** — `anim:`. A field eases toward its bound target
  when it says so: `x: (val: "polkit_cursor_x", anim: (ms: 90))`.
- **6** — implicit `parent` scope; **7** — retire `Mode`/`Ink`.

**The metric for §0 is the `Ink()` CALL-SITE count: 10 → 0** (`settings_ink`,
`dash_grid`, `dash_banner`, `notif_list`, `lock_screen`, `wallpaper_picker` in
`shell.ron`; `worldmap`, `notes`, `eq`, `accent` as whole cards). A bare `grep
-c` says 15 because five of those hits are *comment* lines naming the remaining
ones — count call sites, not matches. The rule is **Rust never emits
geometry** — so a `layout_*` in the draw path is a violation, and a new `Ink` is
a rejection that should instead record what could not be declared yet.

**Next: `QUICKSHELL_MODEL.md` phase 6 (implicit `parent` scope).** Current
state: full suite **508 passed / 0 failed / 0 ignored**, zero warnings.

Phase 6 is the one remaining phase that pays per frame rather than per
declaration: `w: "parent.w - 28"`, `x: "parent.cx"`, so a nested item stops
threading its container's box through every child — which is also what collapses
the positional-box gotcha list. Do it **after** the pool-build fix above if that
fix turns out to need the same threading removed; otherwise it is a large diff
across every draw arm for ~5 µs, and the honest order is to fix what grows.

**Standing perf note.** The per-frame cost that grows with this rewrite is NOT
expression evaluation (see the phase-3 correction above) — it is the pool build:
**594 `String`-keyed `HashMap` inserts** in `scene_values_raw()`
(`src/shell/panels/mod.rs`), ~30 µs today and linear in the number of converted
panels. Phase 4 made this *worse in call count, better in shape*: every panel
and surface now builds its own pool through `scene_values_for(surface)` (needed
so a store prop resolves only in its declaring surface), so the build runs once
per drawn surface instead of once per merged world. The fix is not "build the
pool less often" — it is a name that resolves without a lookup, i.e. exactly
phase 6's implicit `parent` scope collapsing the box threading. Measure before
and after rather than assuming.

**Two shapes to look for when hunting the next one:**
- **A Rust `match` on a key that picks UI state.** Phase 4's six-arm settings
  table was this. The tell is that the arms assign *state* (an enum, an index)
  rather than returning a `Cmd`.
- **A value published under a name two surfaces could both mean.** `scene_values()`
  merged every surface's stores into one pool, so two surfaces naming a store
  `nav` silently shared a value. Namespaces need enforcement, not convention.

Landed (full write-ups in `CHANGELOG.md`, top five entries):
- **Phase 5 — `anim:` — is done, for what an item-level `anim:` can reach.** The
  engine keeps one current value **per binding source string** (so two items on
  `nav.tab` cannot disagree mid-move) and eases toward the target; the polkit
  password cursor is the pilot (`shell.ron`, `anim: (ms: 90)`), and
  `App::anim_timer` (16 ms, self-stopping) is the declarative replacement for the
  `edit_settling` flag that existed only to keep the render loop awake. Four
  things are worth not rediscovering:
  1. **measure reads the TARGET, and an animated `w`/`h`/`font_size` is REFUSED
     (`AnimatedMeasuredField`), not ignored** — an in-flight extent would resize
     the box positioning it, and the schema must not accept an animation the
     renderer drops;
  2. **the table is keyed by binding source, and the first frame of a key rests
     at its target** — without the resting `target` every frame looks like a
     first sighting and nothing ever moves;
  3. **`Val`'s `Deserialize` is hand-written** (phase 2's rule: untagged's error
     message is useless, and `anim: ()` does not match it at all) while
     `untagged` STAYS on `Serialize`. The blocker this phase originally recorded
     — untagged dropping `curve:` — was measured against **ron 0.9** and does not
     reproduce on the pinned 0.12.2; re-run a measurement before believing it
     twice;
  4. **the phase's original "done when" was unreachable and §10 now says so**:
     the pill expand is *surface* geometry (`Shell::anim` → `cur_w` / `cur_h`)
     and the dashboard glide is derived inside the Rust `dash_grid` ink off
     `packed_layout` (`card_anim`). Neither is a field easing toward a bound
     target, so **both stay in Rust** — do not go looking for an `anim:` that
     deletes them.
  **Watch for this shape:** a Rust lerp toward a *published property* is an
  `anim:`; one toward *layout-derived* geometry is not.
- **Phase 4 — `store` + `SceneAction::Set` — is done.** A surface declares
  `stores: [ (name: "nav", props: (tab: 0.0)) ]`; a row or `Hit` writes it with
  `Set(name: "nav.tab", value: 2.0)`. Three decisions are written up in
  `QUICKSHELL_MODEL.md` §5 and are worth not rediscovering:
  1. **a store is declared on the SURFACE, not as an item** — it draws nothing,
     and `Comp` inlining would have copied it into every template;
  2. **legality is decided against the DECLARATION, not the live value** —
     seeding is idempotent by design, so the first version accepted a write to
     any name on an unseeded store, and the refusal depended on load order;
  3. **`store_write_type_ok` is one predicate used by both the runtime applier
     and the validator** — two copies would let the validator pass a scene the
     runtime refuses, and a refused write looks like a click that did nothing.
  A store's closed name set is now **load-checked** (`BadStoreWrite`,
  `UnknownStoreProp`) — phase 3's surviving half. The walker's blanket skip of
  dotted names is narrowed to `item.<field>` (which `UnknownRowField` judges
  better) plus namespaces it cannot see (`mod_clock.time`).
- **Phase 2 — component `props` — is done.** A `components:` entry may now be
  `(props: (…), items: […])` and a use site may pass `props:` overrides; the
  merge lands in the phase-1 `SceneScope`, so a prop is just another scoped name
  and props are confined to their own body. Three decisions were not obvious and
  are written up in `QUICKSHELL_MODEL.md` §4:
  1. a prop is a `PropValue` (number / expression / color token), **not** a
     `SceneValue` — the two schemas want opposite things out of a bare `acc`;
  2. substitution is **symbolic**, so `w: "pill_ws_reg_w"` keeps tracking the
     pool instead of freezing frame one's value into the scene;
  3. RON cannot deserialize a map into `Vec<(K, V)>` and `#[serde(untagged)]`
     swallows every real error, so `PropSet` and `SceneComponent` both carry
     hand-written codecs.
  `subhead` is the migrated call site: it read four *shared* pool names, which
  forced a Rust `match self.mode` to pick the panel's title. That table is
  **deleted** (with its dead `"Audio"` arm) — the titles are `title:` props now.
  **Look for that shape when hunting the next one: a Rust `match` that chooses UI
  copy or a color per panel is a component with props in disguise.** A prop
  nobody reads is reported (`props: (wid: 99.0)` against a declared `w` draws
  exactly as if the line were absent) by re-rendering each body once per prop,
  and a test asserts the live `shell.ron` declares none.
- **Phase 1 — models and delegates — is done.** `SceneValue::Model(Vec<ModelRow>)`
  makes a row a value; `SceneScope::row(row, i)` overlays `item.<field>` /
  `index` onto the pool for one template instantiation and is folded into the
  subtree (`SceneItem::scoped`); `RepeatSlot` owns its instantiated items so the
  measure and draw passes cannot disagree. Dispatch is the row's `tpl`
  (delegate) then its `tag` then `*`, which let the pill's center strip go from
  **14 templates to 8**. `pill_key_<tag>` / `pill_ink_<tag>` and the pill's
  `declare_conditional` loop are **deleted** — a row that exists *is* a widget
  that is on. `UnknownRowField` validates `item.<field>` against the union of
  every row's fields, and an empty/conditional model stands down.
- **Load-time validation is done.** `src/scene/validate.rs` sweeps a freshly
  parsed scene once per load / mtime hot-reload and reports `BadBinding`,
  `UnknownProperty`, `KeyCollision` and `InvertedAnchors` as
  `file: path.to.field: message`. The draw paths call `get_validated()`, so
  it never runs per frame. `Val::compile()` (previously dead) is the only
  binding check and `Expr::deps()` the only name walk, so the validator cannot
  disagree with the evaluator; `Expr::fns` is gone as the duplicate it was.
- **A duplicate key is only an error when the claimants' ACTIONS differ** —
  the fat-target idiom (`latency.ron`'s pill + its invisible halo) is
  legitimate and must stay silent.
- **Property checks run only against a COMPLETE pool** (`SceneValues::
  mark_complete()`; the shell's `scene_values()` is complete, a card's local
  audiorec pool is not), and conditional names are `declare_conditional()`d so
  "collapsed, nothing published" is distinguishable from "typo". That last
  part found three real conditionals on first run: the whole
  `pill_key_<tag>` / `pill_ink_<tag>` tag set (a config toggle drops a widget
  from the bar, so a per-visible-widget declaration never fired),
  `banner_cells_v`, and `pw_b_series` / `pw_g_series` (a trace needs two
  samples).
- **The 99-field sweep is now a test.** `the_walker_reads_every_val_field_in_
  the_schema` reads this crate's own source for `: Val,` fields on
  `SceneItem`; `every_declared_geometry_field_is_walked` resolves one fixture
  per geometry-bearing variant; `every_live_scene_validates_clean_against_the_
  shells_own_pool` runs all 72 committed scenes through the shell's real pool.

Hard rules earned across these two passes — do not skip these:
- **A scope name must be validated, or the scope is only a runtime feature.**
  `item.w` is not free: it is checked against the model's rows, and the walk that
  finds scope names (`SceneItem::scope_names`) must see every channel the
  evaluator reads — `Val::Expr`, `{name}` interpolation, and `value("…")`
  colors. A validator blind to one of those reports the tree as clean and is
  worse than no check.
- **A conditionally published name must be declared WHERE THE GATE IS**, not
  inside the branch that publishes it: a `declare_conditional` inside
  `if hist.len() >= 2` fires exactly when the name is already in the map.
- **Before calling a key collision a bug, ask whether the fat-target idiom
  explains it** (same key, same declared action, one box larger and invisible).
- **Row-selected dispatch trades a compile error for a blank.** A `tpl` naming
  no template draws nothing and reports nothing, so
  `every_pill_widget_has_a_template_for_its_delegate` walks `ALL_PILL_ITEMS`
  against the live scene. Do not add a `PillItem` variant without one.
- **A field rename in `src/scene.rs` is NOT done until every scene file has
  been swept for it** (serde silently ignores unknown fields).
- **Load-time validation is done.** `src/scene/validate.rs` sweeps a freshly
  parsed scene once per load / mtime hot-reload and reports `BadBinding`,
  `UnknownProperty`, `KeyCollision` and `InvertedAnchors` as
  `file: path.to.field: message`. The draw paths call `get_validated()`, so
  it never runs per frame. `Val::compile()` (previously dead) is the only
  binding check and `Expr::deps()` the only name walk, so the validator cannot
  disagree with the evaluator; `Expr::fns` is gone as the duplicate it was.
- **A duplicate key is only an error when the claimants' ACTIONS differ** —
  the fat-target idiom (`latency.ron`'s pill + its invisible halo) is
  legitimate and must stay silent.
- **Property checks run only against a COMPLETE pool** (`SceneValues::
  mark_complete()`; the shell's `scene_values()` is complete, a card's local
  audiorec pool is not), and conditional names are `declare_conditional()`d so
  "collapsed, nothing published" is distinguishable from "typo". That last
  part found three real conditionals on first run: the whole
  `pill_key_<tag>` / `pill_ink_<tag>` tag set (a config toggle drops a widget
  from the bar, so a per-visible-widget declaration never fired),
  `banner_cells_v`, and `pw_b_series` / `pw_g_series` (a trace needs two
  samples).
- **The 99-field sweep is now a test.** `the_walker_reads_every_val_field_in_
  the_schema` reads this crate's own source for `: Val,` fields on
  `SceneItem`; `every_declared_geometry_field_is_walked` resolves one fixture
  per geometry-bearing variant; `every_live_scene_validates_clean_against_the_
  shells_own_pool` runs all 72 committed scenes through the shell's real pool.

Hard rules earned today — do not skip these:
- **A conditionally published name must be declared WHERE THE GATE IS**, not
  inside the branch that publishes it: a `declare_conditional` inside
  `if hist.len() >= 2` fires exactly when the name is already in the map.
- **Before calling a key collision a bug, ask whether the fat-target idiom
  explains it** (same key, same declared action, one box larger and invisible).

Still open — in this order:
1. ~~**`config/zen-shell/shell.ron` in the repo is a stale snapshot**~~ —
   **RESOLVED 2026-10-01**: re-synced from the live file and its header now says
   MIRROR — NOT AUTHORITATIVE, with the `cp` command and the permission to
   delete it. Nothing reads the repo copy, so a test asserting it matches would
   only ever be noise; the honest fix is that it cannot look authoritative.
2. **`Val::num` recompiles its expression on every read.** Fine at 60 fps
   today, but it is a per-frame allocation per bound field; a cached tree
   (`#[serde(skip)]` + `OnceCell` on the `Expr` variant) is the fix if the
   frame budget ever complains. It shipped as part of `QUICKSHELL_MODEL.md`
   phase 3 (dirty-set propagation), which was **demoted on measurement** — a
   `compile()+eval()` is 43 ns against 13 ns with a held `Arc`, so ~7.6 µs of a
   16,666 µs frame. Keep this item closed unless the frame budget actually
   complains; if it does, phase 3 is the fix and the number above is the reason
   it was not done sooner.
3. **Component extraction** (`pill_chip`, `ws_chip`, `hover_fill`) — the
   `props` MECHANISM is done (`QUICKSHELL_MODEL.md` phase 2, landed
   2026-10-01: `props:` defaults + overrides, symbolic substitution, dead-prop
   reporting, and `subhead` migrated). What is still open is converting these
   three specific bodies. The pill's `icon_chip` template already serves six
   widgets off four row fields — which means these three may be best expressed
   as further delegates rather than components, so read the phase-1 note in
   `CHANGELOG.md` before assuming `props` is the right tool for each.
4. **Validator follow-ups worth considering**: `ColorToken::Value(name)` names
   (`value("pw_ink")`) are still unchecked against the color pool — note that a
   *scope* color (`value("item.ink")`) is checked as a row field, a *pool*
   color is not — and so is a `Rows`/`Spark` model name for a *card-local* pool
   (the shell pool is complete, so only the shell's own items are covered
   today). **The store case is now closed** (phase 4): `<store>.<prop>` is a
   declared closed set, so `UnknownStoreProp` reports a typo. What is still
   open is the *middle* namespace — a plugin's `mod_<module>.<name>`, which the
   validator deliberately stands down on because it cannot tell a plugin name
   from a typo.
5. **`SceneValues::set_num` / `set_str` / `set_bool` are test-only.** Every
   production publish goes through `insert(name, SceneValue::Ring(0.0))`, which
   is exactly the "any f32" ambiguity that made a `Fader` binding resolve to 0
   once already. Routing the shell's scalar/string/bool publishes through the
   typed setters would make the carrier type explicit at the call site.
6. **`WALKED_VAL_FIELDS` is declared coverage, not proven coverage.** The
   sweep test proves the list matches the schema's `: Val,` fields and the
   fixture sweep proves the geometry fields resolve; a non-geometry field added
   to the list with no arm in the walker is still uncovered.
7. **The other `Vec<String>`-shaped models are still tag lists**: the workspaces
   grid's `ws_wash_i` / `ws_num_i` and the banner strip's bespoke `BannerCell`
   are the next cards to convert. The banner strip is the harder of the two
   because its rows are heterogeneous by construction.
8. **The 15 `Ink()` bodies are the `QUICKSHELL_MODEL.md` §0 boundary.** They
   are not a cleanup list; each one is unfinished scene work. The four whole
   cards (`worldmap`, `notes`, `eq`, `accent`) are the cheapest conversions —
   one painter per card, so removing the `Ink` is the whole job — and `worldmap`
   first, since its fluid drag is the one body that is irreducibly an algorithm
   and will need the §0 "algorithm" carve-out spelled out rather than faked.
   The six `shell.ron` surfaces are the expensive ones (chrome is already
   declared; only the body remains). Do not start these before phase 4 — without
   `store` the state behind them has to be reimplemented in Rust-valued state,
   which is the same work twice.
- The 35 `Ink` cards are a separate track; the hover/pane helpers (`pane_dots`,
  `card_pane_flip_n`) have zero `Ink` consumers and can move into `scene.rs`
  whenever the last drawer that calls them is converted.

---

## ⏸️ RESUMED 2026-09-30 (evening) — declarative-core rewrite: Stages 1, 2 AND 3 landed

**Done: load-time validation — see the 2026-10-01 block above.** State then:
full suite **417 passed / 0 failed**.

Landed on the 09-30 evening session (full write-up in `CHANGELOG.md`):
- **Stage 3 (bindings) is done.** `Val` (`Lit(f32)` | `Expr(String)`) on all
  99 extent/key fields; the `x_var` / `y_var` / `w_var` / `h_var` / `d_var` /
  `key_var` / `size_var` family is **deleted**, and all 72 live scene files are
  migrated. `hover_key_var` / `cols_var` / `cell_var` / `max_rows_var` stay on
  purpose — they name a *string* property, not a scalar. The expression
  evaluator (`src/scene/expr.rs`) is live, so a binding can be arithmetic, not
  just a property name.
- **The wallpaper card's wheel claim is fixed**: direction-aware
  (`wp_card_can_scroll`), one geometry chain for the claim and the move, and
  the per-frame wheel rects cleared in `layout_for` so an off-board card cannot
  eat notches. See the CHANGELOG entry for the three-part rule.

Hard rules earned on 09-30 — do not skip these:
- **A field rename in `src/scene.rs` is NOT done until every scene file has
  been swept for it.** `shell.ron` kept 71 dead `*_var` fields through the
  whole migration and **serde silently ignores unknown fields**, so every one
  drew the literal underneath it (the polkit caret sat at x = 0). The parity
  tests caught it only because they compare against the Rust drawer. Sweep
  with `rg -n "\w+_var" shell.ron ui/cards/*.ron` and check every hit is one
  of the four surviving `*_var` fields.
- **A `Val` field that replaced a defaulted `f32` needs
  `#[serde(default)]`** (`Val::default()` is `Lit(0.0)`). 85 of them did not
  have it and every scene omitting that field stopped parsing. When adding a
  `Val` field, add the default in the same edit.
- **`want_w`/`want_h` resolve bindings; `fills_w`/`fills_h` decide "fills".**
  They must never collapse into one predicate again: `Val::constant()` is 0 for
  a binding, so a single `want_*() == 0.0` test makes a layout pass overwrite a
  bound extent with the parent's width (it cost the pill's workspace chip 2 px
  of click region).
- **A measure that uses `Val::constant()` is a bug waiting for a binding.**
  Same root cause, two more sites already fixed: `content_right` (an `auto`
  container reserved a bound child's width but not its bound `x`) and
  `SceneValues::scalar` (read only `Ring`, so a `Fader` binding resolved to 0).
  In the layout passes, resolve with `num(vals)`; keep `constant()` only for
  the "did this field declare anything?" question.

## ⏸️ RESUMED 2026-09-30 (morning) — declarative-core rewrite: Stages 1 AND 2 landed

Next was Stage 3 (landed in the evening session above). State then: full suite
**414 passed / 0 failed**.

Landed on the 09-30 morning session (full write-up in `CHANGELOG.md`):
- **Bare `Option` fields.** RON `IMPLICIT_SOME` is on at the single load path
  every scene goes through (`scene::parse_scene_ron`): `color: fg`,
  `visible: "batt_h_pane_0"`, `anchors: (left: 0.0, …)` — no `Some(…)`. The
  wrapper still parses, so files can migrate a field at a time. All 72 live
  scene files are migrated (1034 wrappers removed, comments and string literals
  untouched); backups in `/tmp/opencode/ron-bak/`.
- **Stage 2 (implicit sizing) is done**: five open-coded copies of "the widest
  visible child's content extent" now share `SceneItem::widest_content`. It
  fixed a real asymmetry — the `Column` draw arm used to measure gated-off
  children, so a `Column { auto: true }` could reserve a width nothing painted.

Decisions taken, so they don't get re-litigated:
- **`region_d*` is KEPT, deliberately — the anchors migration is complete.**
  `anchors` is an ABSOLUTE rect that replaces the node's box (paint included);
  `region_d*` is a DELTA halo on a painted box that must stay put, so it is
  parent-size-independent by construction. The 13 card sites still using it
  (`accent` ×1, `audiodevice` ×1, `clipimg` ×1, `system` ×5) cannot be written
  as anchors without restating the live card width in four insets and wrapping
  each in a `Stack` — that is more vocabulary and more fragility, not less.
  Same answer for the ws tail's `pill_ws_reg_w` (26 or 20 px from the length
  of the workspace number's string — runtime data, so it needs `w_var` until
  Stage 3 bindings exist).

Still open — Stage 3, in this order (item 1 landed in the evening session):
1. ~~**Typed property store + expression evaluator** to delete the `*_var`
   family~~ — DONE, see the evening block above.
2. **Load-time validation** — ~~still open~~ **DONE 2026-10-01**, see the
   block at the top of this file.
3. **Component extraction** — still open.

## ⏸️ PAUSED 2026-09-29 — declarative-core rewrite (Quickshell-style), Stage 1 done

**Do this first tomorrow:** re-run the pill test with the mandatory touch recipe
(see CHANGELOG 09-29) to confirm the tree is still green, then continue the
anchors migration / start Stage 2. Current state: full suite **412 passed**.

Landed today (all on disk, suite green):
- New `Anchors { left, top, right, bottom: f32 }` in `src/scene.rs`, an
  **additive** `anchors: Option<Anchors>` on `SceneItem::Hit`. When present it
  **replaces** the node's `x/y/w/h`, measured as insets from the parent's inner
  rect (`(x, y, card_w, card_h)` as passed to `draw_piece`; `left`/`top` from the
  inner left/top, `right`/`bottom` from the inner right/bottom).
- The 12 collapsed-pill chip region `Hit`s in the live `shell.ron` now use
  `anchors: Some((left: -2.0, top: 4.0, right: 2.0, bottom: -4.0))` instead of
  `region_dx/dy/dw/dh` (= the drawer's `region(cx−2, 4, iw+4, h−8)`).
- New `hover_key_var: Option<String>` + `hover_key_add: u32` on `SceneItem::Surface`
  (bind the hover-gate key to a published scalar, plus a slot offset). The pill's
  bell / brightness / settings fills and the ws_long tail fill are now `Surface`s
  using these; **`Hit.no_region` is deleted** from the schema and draw arm.

Gotchas re-confirmed today:
- This RON writes `Option` **explicitly** — `anchors: Some((…))`, not bare. An
  ergonomic fix under consideration is enabling RON `implicit_some` at the
  `ron::from_str` sites (`src/scene.rs:3105`, `:3266`) so `anchors: (…)` and
  `color: fg` lose their `Some(...)` — a real part of the "not Quickshell-like"
  feel. Verify it doesn't break the 71 card scenes first.
- `*_var` fields **replace** the declared base; `rekeyed` is **additive** and
  re-keys both `Hit { key }` and `Surface { hover_key: Some(k) }`.

Still open (Stage 3) and the card migration:
- `region_d*` remains in the schema and is **staying** — see the 09-30 block
  above: it is a delta primitive, not a pending migration. The cards
  (`ui/cards/audiodevice.ron`, `accent.ron`, `clipimg.ron`, `system.ron`, …)
  are its correct consumers.
- Stage 2: unify `intrinsic_w` / `inner_w` / `content_right` implicit sizing.
  **DONE 2026-09-30** — `SceneItem::widest_content`.
- Stage 3: typed property store + expression evaluator to delete the `*_var`
  family (**DONE 2026-09-30**); load-time validation (**DONE 2026-10-01**);
  then component extraction (`pill_chip`, `ws_chip`, `hover_fill`) — the `props`
  mechanism landed 2026-10-01, these three bodies still open.

## 🛑 HARD RULE — ultra-fast start (nothing blocks the boot critical path)

The bar must be hover/click-**expandable from the event loop's FIRST dispatch.**
`App::start()` must never run anything that can block — a subprocess spawn,
D-Bus, or a big file/db read — **before the loop starts**. The event loop is
the ONLY thing that dispatches hover/click, so a slow seed on that path reads
as "wait N seconds until the pill becomes expandable, else not": at login a
cold system D-Bus (MPRIS / NetworkManager / Bluetooth), a `wpctl` spawn racing
PipeWire, a `pacman`/`checkupdates` spawn, or the cliphist bbolt db reads can
each stall startup for seconds.

Enforced in `src/app/mod.rs` `App::start()`: boot seeds run on staged in-loop
one-shot timers, never inline before `event_loop.run(…)`:

- **0 ms** — shell-file seeds (per-state dash config, colors, accent flags,
  todos, hostname) + pill re-fit (`shell.seed_data`, `refresh_size`/morph).
- **80 ms** — fast local reads (`seed_boot_data_quick`: hyprctl ws/win, battery
  sysfs/upower, brightness, volume files).
- **450 ms** — heavy seeds (`seed_boot_data_heavy`: `services.poll` D-Bus,
  `clip.seed_history` cliphist dbs).

When adding any boot-time seeding in the future: measure it. If it can block
more than ~1 ms (any spawn, D-Bus call, or db read), put it in a later boot-seed
stage or on a background thread — never on the pre-loop critical path. Also keep
`App::new` free of anything but the instant backends (that is what makes the
first frame appear immediately).

## 🛑 HARD RULE — multi-pane cards (applies to ALL cards)

Any dashboard card that can be swiped / scrolled **left-right** (multiple panes /
pages of content) **MUST render pagination dots below the card**, centered or
right-aligned at the card's bottom edge, one dot per pane, with the **active
pane's dot highlighted in the accent color** and the others dimmed. This is
required for every such card, present and future — battery (pane 0 = gauges +
power-save, pane 1 = extra info), weather (2 panes), and any card added later
that gets panes. **Exception**: the world-map card switches its 4 panes from a
swipe-right menu (no swipe-cycling, no dots) — dots are required only where the
swipe itself flips panes.

- Horizontal wheel / trackpad-two-finger over the card flips panes (see
  `input.rs` `hsteps` handling: `over_battery` → `battery_pane_flip`).
- **No wrap-around**: swiping past the LAST pane does nothing (clamped), the
  card never jumps back to pane 0 mid-swipe.
- Vertical wheel over a multi-pane card keeps its normal behavior (list scroll
  or board pan if the card has no list).
- Shared helpers, N-pane safe: `pane_dots` (any pane count; `battery_dots` is
  now a 2-pane alias) and `card_pane_flip_n`. Current consumers: battery V/H
  (2 panes), weather (2 panes).

## 🛑 HARD RULE — cards never overlap

The metro grid must NEVER show overlapping cards:

- When new cards are added and don't fit the current rows, the canvas
  **auto-grows** rows (up to `DASH_MAX_ROWS` = 4096, the sanity ceiling) and
  packs them into the new row — it never paints on top of an existing card.
  `pack_cards_max` is guaranteed overlap-free; the old fallback that cloned a
  card verbatim onto occupied cells was removed (now it lands on the first
  fully-empty row below all content).
- Edit-mode drag / resize already re-packs the grid live with the dragged card
  pinned at the ghost slot — other cards are PUSHED aside so the dragged card
  always makes room for itself.

## OPEN / TO VERIFY (user iterates visually)

- **Banner strip editor** (shipped — fine-tuning only):
  - dashboard ON: is the always-visible strip-editor row + pushed-down cards
    tray OK?
  - column drop-guides: too faint / too loud? drop–highlight semantics feel right?
  - empty-strip collapse: removing every token → the board rises to the top
    (`dash_top`/`tray_at_top` fold to `BASE_BANNER_Y+2`).
  - width AUTO/MANUAL step + slider feel; spacer handle; chip spacing.
  - Known: `BannerDrag.sx/sy` never read — deleted in the 09-11 sweep (drag
    offsets ride `off_x`/`off_y` instead).
- **Power / clipboard / wallpaper visual pass**:
  - hold-to-confirm delay on PowerV/PowerH cards matches the power menu.
  - copy a few images → Clipboard-images card populates; hover- and click-copy.
  - wallpaper card: are adaptive tiles + scrollbar + count enough, or should the
    tile render "contain" (whole picture) instead of cover-crop? That touches the
    shared thumb pipeline (`generate_thumbnail` in `src/img.rs`, `rasterize_cover`
    → square `THUMB_SIZE`; tiles are square `Cmd::Image`s in `draw_wallpaper_card`).
- **Hygiene (OPEN)**: the 09-12 session (settings/accent panels rework,
  wiki-usage cards, weather/input/scroll work, README) never got CHANGELOG.md
  entries — README captured it, CHANGELOG stopped at 09-11. Write them when
  the current pass settles.
- Long-term items stay in `roadmap.md` (multi-monitor, fractional scale, pipewire
  audio backend, MPRIS event-driven, auto-lock, plugin system, etc.).

## SETTLED / HALTED

- **DONE (2026-09-12) — Vulkan feature-gated**: `render_vk` now compiles only
  with `cargo build --features vk` (`vk = ["dep:ash","dep:naga"]`,
  `#[cfg(feature = "vk")]` guards in `main.rs`/`app/mod.rs`). Default builds
  are GL-only; runtime still picks Vulkan via `ZEN_VULKAN=1`, falling back to
  GL on init failure. Docs synced 2026-09-13 (README stack + src-map + build
  note, PROJECT_GUIDE boot flow). `features.md` also re-synced: test count
  63→73, and the "chip width resize" section is now honestly marked
  never-landed (`banner_chip_widths`/`banner_resize` don't exist).
- **HALTED (do not reopen): zeneq EQ crate** — PipeWire here can't run LADSPA
  (`libspa-filter-graph-plugin-ladspa.so` needs `spa_log_topic_enum`, absent from
  the installed `libspa-0.2.so`; daemon crashes exit 254). Fix = update
  PipeWire/spa packages, then reinstall per the zeneq README.

## Quick reminders

- Canonical path: `/acc/common/hdots/rust_project_root/zen-shell` (the old
  `/tmp/user-tw/zs` alias is gone).
- Shared cargo target: `/acc/data/persist/user/.cargo/target` (debug + release).
- No git repo on this copy — no commits available.

## Key reference pins (updated 2026-09-13)

- Packer: `pack_cards_max` (NOT `pack_cards`) in `src/shell/mod.rs`; drag tests in
  `shell::edit_drag_tests`; layout geometry tests in `shell::panels::layout`;
  `banner_drop_target` is `#[cfg(test)] pub(crate)` (drop math, unit-tested).
- Strip-editor control row: `edit_press` in `src/shell/mod.rs` —
  `BANNER_CTRL_KEY_BASE`=34_300 (dashboard ON/OFF), `BANNER_W_AUTO_KEY`=34_332
  (width auto/manual), `BANNER_W_SLIDER_KEY`=34_333 (hidden in auto).
- Banner strip: `banner_cells(row_w)` + `banner_strip_w()` + `banner_cell_px(i,row_w)`
  (geometry, `src/shell/mod.rs`); drawer + drop grid in `src/shell/panels/dashboard.rs`.
- Scroll semantics: `input.rs` ~389 (vertical wheel) & ~475 (horizontal wheel),
  `dash_scroll_dir` swap.
- Grid toolbar draw: `dashboard.rs` ~473-509 (`bw=18` btn, `cw_t` label width,
  right-anchored cluster).
- `draw_edit_btn` / `draw_edit_lbl_btn`: `dashboard.rs` 803 / 836.
- Config persistence: `save_config` in `src/shell/mod.rs` (~2854+); DashCard draw
  dispatch ~126-171 & ~231-276.
- World-map card (landed 2026-09-13, overhauled into an app same day): an
  EMBEDDED 110m world (`src/shell/world_map.bin`, include_bytes → always
  renders, `ensure_fallback`/`using_fallback`) plus GitHub data
  `ezone134/zen-shell-map` (`src/shell/mapdata.rs` - manifest cache, sha256
  chunk downloads, `region_at`); runtime store in `src/shell/worldmap.rs`
  (chunk parsers, landmask, zone.tab projection, `ready`/`clear`); drawer
  `draw_worldmap_card` in `src/shell/panels/cards/worldmap.rs` (scissored
  body, compact download pill while fallback, 4 menu panes, +/− zoom +
  touchpad PINCH via `zwp_pointer_gestures_v1`, region-detail pill,
  `over_worldmap`). Keys: WORLD_MAP_KEY=14_900, WORLD_DL_KEY=14_901,
  WORLD_REGION_DL_KEY=14_902, WORLD_ZOOM_IN/OUT=14_903/4, WORLD_MENU_KEY=
  14_905, WORLD_MENU_TOGGLE_KEY=14_906, WORLD_STYLE_KEY_BASE=14_910,
  WORLD_PANES=4. Cache: ~/.local/share/zen-shell/maps (save, DEFAULT OFF —
  `[world] save_data=false`) or /tmp/zen-shell/maps. Pinch binding:
  `ensure_pinch` in `src/app/handlers.rs` (janky? verify). Offset probe
  `world_shell_probe` in `src/app/mod.rs` (3 s services timer; DST re-probe
   ~5 min). Default layout row 16: `(WorldMap, 0, 64, 10, 9)`.

---

## Scene-Graph Declarative UI (added 2026-09-15)

Full details: `HANDOFF_SCENE_DECLARATIVE.md`.

### State
- **28 Header-chrome scenes** live (all `card_head` cards), scenes declare their
  own title/glyph/meta; Rust body drawers use `Ink(name:…)` for bodies.
- `Shell::scene_owns_header` guards double-draw; `self.card_head` wrapper + 29
  `ui::title` guards all skip chrome when the scene owns it.
- `scene_values()` now exposes 22 dynamic meta/glyph keys (`disk_meta`,
  `cpu_meta`, `moon_meta`, `mic_icon`, etc.) **plus** typed values:
  `battery_ring`, `mem_pct_f` (Ring), `volume_fader`, `brightness_fader`,
  `mic_fader` (Fader), `wifi_on`, `bt_on`, `power_save_on`, `fx_blur_on`,
  `fx_shadow_on`, `fx_opacity_on` (Toggle), `cpu_spark`, `gpu_spark`,
  `lat_spark`, `net_up_spark`, `net_down_spark`, `disk_r_spark`,
  `disk_w_spark` (Spark), `jr_rows`, `conn_rows`, `top_procs_rows`,
  `fans_rows` (Rows).
- **Phase 2a LANDED (2026-09-15)**: six data-driven primitives — `Rows`,
  `Spark`, `Ring`, `Fader`, `Toggle`, `TabRow` — draw from the typed
  `SceneValue`/`SceneValues` map. 190/190 tests at the time.
- **Phase B LANDED (2026-09-15)**: layout containers — `Row`, `Column`,
  `Stack` — with `pad`, `spacing`, `valign`/`halign`, fill semantics (0-dim
  items span the parent box), and nested container nesting. Cards can now
  describe their body layout in pure RON without hard-coded x/y offsets.
- **Batch 1 start LANDED (2026-09-15)**: fully-declarative list cards. `Rows`
  gained `start` (scalar-bound scroll window — keys rebase to the visible
  window), per-row `surface` (resting active/connected tint; wins over hover
  like the Rust drawers), and per-cell `col_colors` (status columns stay
  colored on active/dimmed rows); `SceneCol` gained `icon` (glyph cells).
  `draw_card_scene` tracks card scroll rects
  (`bluetooth`/`ticker`/`currency`/`recent`) for wheel scrolling when a
  fully-declarative scene (no `Ink`) skips the Rust drawer. 202/202 tests at
  the time.
- **Converted so far (Batch 1)**: bluetooth, ticker, currency, sshvpn, recent
  — all data-driven via `Rows` (+ scroll / per-row state / hover tint via
  `col_colors`), no Rust drawer involved. `recent` also extracted its stale
  30 s cache refresh into `refresh_recent_if_stale()` so the scene path keeps
  the data live (`draw_card_scene` calls it when no `Ink`).
  Remaining Batch 1 cards to convert: ~~calendar, appshortcut, mirror~~ ALL
  DONE (2026-09-16 — calendar/appshortcut are zero-Ink `Grid` scenes, mirror
  is a two-pane scene with `Image`/`Row(bottom)`/`Rows`/`Toggle`/`Dots`).
  **Batch 1 COMPLETE.**
- **Interactive list cells LANDED (2026-09-16)**: `Rows` gained a live
  checkbox column (`check` + `check_x` over the parallel `Checks(Vec<bool>)`
  value — acc-tint well + check glyph vs stroke outline, the To-Do box) and a
  hover-✕ delete (`del_base`/`del_pad`/`del_dy`/`del_size`: reveal while the
  row or ✕ is hovered — key-less rows reveal on the ✕ only, like the alarms
  drawer — danger-red on the ✕ itself, 22² corner region registered only
  while hovered so it wins the row corner like the Rust drawers). **todo**
  became the first three-split card: title/count/`+N more`/empty-state stay
  the `todo` Ink, the checklist is a `Rows` item (`key_base: TODO_KEY_TOGGLE`,
  `del_base: TODO_KEY_DELETE`, `start: "todo_scroll"`, `check: "todo_checks"`),
  and the strip is the `Composer` — `scene_owns_rows` (the
  `scene_owns_header`/`scene_owns_composer` pattern) gates the Rust row loops.
  **alarms** followed the same way (rows only: mono accent time + label,
  `del_base: ALARM_DEL_BASE`, `del_pad: 26`/`del_dy: 4`, add-row + count stay
  Ink). Snippets/countdown adopt the ✕ the same way. 230/230 tests,
  bin clean (only input.rs:512 pre-existing).
- **Countdown list body declarative LANDED (2026-09-16)** (230/230):
  `SceneCol` gained `truncate: Option<f32>` (reserve `n` base px on the right,
  cut the cell at 0.62·fs px/glyph ≥ 8), `edge: Option<f32>` (right-edge
  anchor `card_w − edge`), `pill: Option<PillSpec>` (chip cell — rounded rect
  auto-fit to its text, fill derived from the text token: Acc→tint else
  hover, or `bg` override) and `del_anchor: bool` (revealed ✕ + full-row
  region lands left of that col). **countdown** is converted: `countdown_rows`
  (label `truncate: 120` · date caption · right `Nd`/`today!`/`passed` pill,
  `y: 50`, 24 px pitch, `del_base: COUNTDOWN_DEL_BASE` at `del_pad: 20`/
  `del_dy: 4` via `del_anchor`, key-less ✕); title/meta + composer row landed
  the same session (zero-`Ink`, the composer as `Surface`/`Text`/`Hit`). Pre-unblocks newspapers
   (chip strips + two-line headlines), expenses (amount chips), quote (pills).
- **News fully declarative LANDED (2026-09-16)** (236/236): `news.ron` is
   zero-Ink — `Header` (`\u{f1ea}`, label = `{n} stories`/active category),
   the new **horizontal `Strip`** (scrollable chip row: `Chips(Vec<String>)`
   value + `sel`/`scroll` Ring bindings, offscreen skip, active accent +
   Sfg label, `key_base + i`; wheel handler clamps `news_cat_scroll`), the
   centered fetching/no-feeds message gated by `news_empty`, and the headline
   `Rows` (40-char title, right open-glyph, right source label on a second
   baseline, `y: 59`, 30 px pitch, `key_base: 13300`, `start: "news_scroll"`).
   Three new list-ink primitives: `SceneCol.hover` (per-col ink while the ROW
   is hovered — title→acc, source stays fg3), `Rows.hover_bar` (the rounded
   2.5×24 accent rail on the hovered row), `Rows.hairline` (1 px dividers).
   Chip/headline keys route through the resident handler — no Ink.
- **Snippets fully declarative LANDED (2026-09-16)** (238/238): `snippets.ron`
   is zero-Ink — `Header` (clip-count meta in fg), `Composer` (buffer /
   placeholder / focus toggle, keyed SNIPPET_INPUT_KEY), and the clipboard
   `Rows` (copy glyph · name · body preview) with `hover_surface: hover_hl`
   and the new **`Rows.flash`** transient accent — bound scalar = index + 1
   (0 = none) from a `Ring`, opted-in cells use `SceneCol.flash` so the
   copy glyph + name wash accent while the dim preview stays fg3; flash
   overrides hover surface + ink exactly like the Rust drawer's 1 s flash.
   Values: `snip_meta`, `snip_buf`, `snip_focus`, `snip_flash` (Ring),
   `snip_rows`. 238/238 tests, bin clean.
- **Expenses fully declarative LANDED (2026-09-16)** (234/234): the whole card
   is `expenses.ron` with **zero Ink** — `Header` (title + MTD total in accent
   mono via `meta_color`), a **`Row` of six equal-split `Hit` chips**
   (1/5/10/20/50/100, `surface: hover`, keys 31800..31805), the composer row
   (`Composer` with `pad: 0` inside the row pad, `focus: exp_focus`, key
   31810, Enter commits via the resident key handler) + a fixed-70 `Hit`
   "clear mo" flipping `fg2 → danger` on hover (key 31811), and top-3 bars
   (`caption + Bar + right mono amount`, `Bar.reserve: 46` keeps the amount
   column clear, each group gated by `exp_has_n`, empty caption gated by
   `exp_empty`). Four tiny primitives: `Header.meta_color`, `Hit.color_hover`
   (pre-unblocks news title hover), `Text.visible` + `Bar.visible` (Toggle
   gates), `Bar.reserve`. No `scene_owns_*` needed — a scene without Ink is
   dispatched whole-card. The equal-split `Row` replaces the need for a fixed
   chips strip; news still needs the horizontal **scrollable** strip.
- **Quote body declarative LANDED (2026-09-16)** (232/232):
   `SceneItem::TextWrap` — a multiline text block that wraps at 34 chars/line
   (≤ 136 chars total), vertically centered inside a `top`/`bottom` box (card
   insets), with an optional dim `caption` line trailing the block. `visible:
   Option<String>` gates the whole item on/off without reserving space (the
   `scene_owns_rows` flag also detects `TextWrap` so the Rust drawer skips its
   own body when the scene owns it). **quote** is converted: the wrapped text
   + author caption is a `TextWrap` item (visible when `quote_has`, `top: 28`
   clears the header, `bottom: 36` clears the pills); title/meta + the two
   pills (refresh always, save conditional) stay the `quote` Ink; the flash
   decrement stays inside the Ink. New values: `quote_text`, `quote_author`,
   `quote_has` (Toggle). Pre-unblocks **lyrics** (multiline wrap, partial) and
   conditional body blocks in future cards.
- **Notes list body declarative LANDED (2026-09-16)** (227/227): `SceneCol`
  gained `dy: Option<f32>` (per-column baseline — a row now carries a title +
  dim caption line; falls back to `row_dy`), and `Rows` gained `visible:
  Option<String>` — a bound `Toggle` resolving `false` hides the whole list
  (the arm skips its row loop, not `return`, so later items still draw). The
  **notes** card is converted: `notes_rows` (bullet tint col · title 9.5 ·
  body caption at `dy: Some(12)`, `y: 32`, 24 px pitch, 12 pad, `row_dy: 0`,
  `start: "notes_scroll"`, `visible: "notes_composing"`) while header/`+`/
  empty-hint and the full composer stay Ink, the Rust row loop gated by
  `scene_owns_rows`. `dy` pre-unblocks the news/lyrics/countdown two-baseline
  rows.
- **202/202 tests pass**, bin build clean (only input.rs:512 pre-existing).
- **Phase C start LANDED (2026-09-15) — whole-shell declarative (`shell.ron`)**:
  new top-level `ShellScene` / `SurfaceScene` / `ShellSceneCache` in `scene.rs`
  (`~/.config/zen-shell/shell.ron`, mtime hot-reload like card scenes), the
  `Comp` item (named component template INLINED at load with a `(x, y)` anchor
  offset — components may nest), and `Shell::draw_shell_surface(name, …)` +
  `dispatch_shell_inks` (surface `Ink`s route to the section Rust painters).
  `Rows` gained `pitch` (stride override — 46 px nav rows over a 40 px body),
  `w` (bounded row box), `row_dy` (cell baseline) and `r` (row corners); `Hit`
  gained `hover_only` (surface only while hovered — the ✕ close button);
  `Ink.name` now interpolates `{var}` so the declared scene can pick its Rust
  body per state. **`settings` surface is LIVE**: the two-pane Settings app's
  chrome (title, hover-✕ close, sidebar rail + 180 px nav `Rows` with
  selected/hover surfaces from `scene_values` `settings_nav`) is declarative;
  each section body is `Ink(name: "{settings_ink}")` routed to its existing
  Rust painter. There is no `shell.ron` on disk → every surface still falls
  back to the built-in Rust layouts unchanged. 204/204 tests at the time.
- **Dynamic color tokens landed (2026-09-16)**: `ColorToken::Value(DynColor)`
  (Copy, 24-byte buffer, `value("name")` in RON), `SceneValues` color pool
  (`SceneColor::Token(T)` live-resolved at draw time vs `SceneColor::Raw(u32)`
  for computed blends), `ColorTokenExt::resolve_with`/`hover_variant_with` —
  all 19 draw-path resolver sites in `scene.rs` thread the pool. Together they
  unblock state-driven single-element colors (DND chip, chips, banner).
- **`notif` surface is LIVE (2026-09-16)**: `layout_notifs` split into chrome
  (title `ntitle` + Clear chip `nclear` → declarative `Hit`, key 2) +
  `layout_notifs_body` (list + empty state → `Ink("notif_list")`); consults
  `draw_shell_surface("notif")`, falls back to the full Rust layout when not
  declared. DND chip (`ndnd`, key 3) is declarative and state-driven via the
  dynamic tokens: `dnd_chip`/`dnd_chip_hover`/`dnd_chip_fg`. 209/209 tests,
  bin clean (only input.rs:512).
- **`lock` surface is LIVE (2026-09-16, second pass)**: hero chrome (`lhero`)
  — fullscreen dim, `{clock}`, `{lock_date}`, avatar disc + `{user_init}` +
  `{username}` — is center-anchored to the fullscreen surface via new
  `Surface` `center_x`/`center_y` fields (offsets are base px from the box
  mid) and bound colors `value("lock_dim")` / `value("lock_avatar")`;
  `layout_lock_body` (password field, error/caps, hold-power buttons, hint —
  owns keys 1–4) routes through `Ink("lock_screen")`. Consulted only at the
  ~1080 reference height (h ± 24); other sizes run the Rust layout.
- **`wallpaper` surface is LIVE (2026-09-16, second pass)**: header chrome
  (`wphead`) — back `Hit` key 1 (hover-only fill) + `\u{f053}` + "Backgrounds"
  — byte-for-byte positions; `layout_wallpaper_body` (empty state + thumbnail
  grid + scrollbar) routes through `Ink("wallpaper_picker")`.
- `shell.ron` now declares the `dashboard` surface with BOTH bodies live:
  `Ink("dash_grid")` (board) + `Ink("dash_banner")` (top banner strip) route
  through the exact Rust painters during the viewing state (`!editing &&
  !sys_menu_open`); the scrollbar overlay stays Rust inline with its drag hit
  zones 500/501. Edit-mode chrome + the system ⋮ menu remain Rust (frame-
  dynamic geometry) and never consult the surface. ⚠️ non-recursive by design:
  the painters are CALLED BY `layout_expanded`, never the reverse.
- **Viewing strip CHIPS are declarative (2026-09-16, fourth pass)**: the
  `dchips` component (a `BannerRow` bound to `banner_cells_v`, gated on
  `banner_shown`, with `banner_overflow`/`banner_zones` scissor modes) rides
  the `dash_banner` Ink's dispatch slot after the painter — z-order grid →
  band → edit chrome → chips. Cells publish per frame via
  `publish_banner_cells` (geometry from `banner_cells`, inks = `bchip_*`
  ladder bindings, tray sub-regions 30+ti); the Rust chip sweep + viewing
  connectivity popover stand down while `dash_chips_via_scene` is up. STILL
  RUST: the whole EDIT strip (guides, grips, park-minus badges, zone labels,
  drag ghost, parked tray, editor rows), the connectivity popover, and the
  tray-click/panel-open INPUT routing (keys are unchanged, so no input work
  is needed).
- **Dashboard prerequisites LANDED (2026-09-16, third pass next)**: the scene
  engine now has `Scissor`/`ScissorEnd` items (rect clip regions emitted as
  `Cmd::Scissor`/`Cmd::ScissorEnd` into the existing live scissor stack; `w:0`/
  `h:0` axes span the parent box) and `scene_values()` publishes the banner
  live tokens — `banner_guide`/`banner_guide_dim` drop-guide inks + the
  `banner_editing`/`banner_drag`/`banner_zones`/`banner_collapsed`/`banner_overflow`
  toggles. The `dashboard` surface split (banner chrome declarative via
  `Scissor` zones + guide `Hit`s; grid via the chrome/body Ink) can now be
  wired. 215/215 tests, bin clean (only input.rs:512).
- **First whole-shell BATCH (2026-09-29): `osd` / `power` / `wifi_menu` /
  `bt_menu` are zero-`Ink` surfaces** — the first `shell.ron` surfaces that
  own a panel COMPLETELY (no Rust body at all), each `layout_*` now opening
  with `draw_shell_surface` + `return`, Rust kept as the no-scene fallback:
  - `osd`: the dismiss `Hit` (key 1) + both strings, gated on `osd_on` (the
    drawer returns before drawing when no OSD is live, so the scene hides the
    texts the same way). The two centered anchors are the drawer's own
    `(h − fs)/2`: glyph `y: -11`, label `y: -7` under `center_y`.
  - `power`: title + the five 92 px tiles in one centered `Row` of `Stack`s
    (background wash → hold `Bar(vertical)` gated on the panel's own 1.5 px
    sliver → glyph at the drawer's `(tile_w − fs)/2` = 34 inset → label on the
    tile center at `y + 60` → `Hit` key 1..=5). The panel's ladder is its OWN:
    `ppower_bg_N` / `ppower_hold_N` / `ppower_hold_on_N` / `ppower_ink_N`
    (the cards' `power_*` reds Logout + Restart too, and keys 12000+). The
    liquid reuses the global `power_hold_fill` / `clear` colors, as powerv does.
    `Shell::power_panel_items()` is now the single source of the glyph/label/
    danger tuples for both the drawer and the publisher.
  - `wifi_menu` / `bt_menu`: the shared **`subhead`** component (back `Hit` +
    `\u{f053}` + `{sub_title}` + a right-anchored meta whose SIZE rides
    `sub_meta_size` and whose INK rides `sub_meta_col` — one stamp covers the
    radio menus' "On|Off" and Audio's quieter meta) plus one `Rows` list on the
    drawers' 56/36/42/16/12 geometry, keys 10+. The signal staircase is a
    `bars` cell: `SegmentsSpec.center` centers each bar on the row's mid rail
    and `col.right` + `col.edge: 46` anchors the whole BLOCK (the drawer's
    `w − 72` start, 4·5 + 3·2 = 26 wide). The row wash is published PER ROW
    (`wifi_wash_N` / `bt_wash_N`) because the submenus' chain is hover-FIRST
    while `Rows` resolves `row.surface` before `hover_surface`. The Wi-Fi
    label is the composited `"{lock} {ssid}"` string (the padlock is a PREFIX;
    the `suffix` cell kind only hangs a trailing glyph) and the Bluetooth state
    column is a per-row `col_colors` binding (`bt_state_N`).
  - **Engine gained**: `SegmentsSpec.center` (a segmented `bars` cell centers
    its staircase on `base` instead of standing on it) and right-edge block
    anchoring for that cell (`col.edge` already existed for text cells).
  - Guarded by `shell_ron_first_batch_matches_its_rust_drawers`, which draws
    the live surfaces and measures them against the drawer constants (row
    band, stride, label inset, check anchor, signal block, tile block, both
    OSD anchors) — so a drifted constant in either file fails the suite.
- **Second whole-shell BATCH (2026-09-29): `workspace_switcher` / `polkit_auth`
  are zero-`Ink` surfaces** — same shape as batch one (`draw_shell_surface` +
  early `return`, Rust kept as the fallback):
  - `workspace_switcher`: `Hit` key 1 dismiss, `Workspaces` at 16/14 (17 px)
    and a right-anchored `{ws_count}` 20 in from the right edge (13 px), then
    the grid as **TWO `Rows`** lists (`ws_col0` pad 14 / `ws_col1` pad 302,
    274 wide, 56 tall on a 70 stride from y 44) — `Rows` has no `x`, so the
    column offset rides `pad`, and each row's own `key` (not `key_base`) keeps
    the GLOBAL `10 + i` sequence across the two lists. The publisher fills each
    column in the drawer's column-major order (`i = r·cols + c`, capped at 10
    workspaces). The active card's 4 × 40 accent rail is a `bar` cell
    (`height: 40, top: 8, radius: 2`) whose value is 1.0 on the active row and
    0 elsewhere on a `clear` track; the number (20 px) and `Desktop` caption
    (11 px, 16 down — `col.dy` REPLACES `row_dy`, it does not add to it) ride
    the per-workspace `ws_wash_i` / `ws_num_i` bindings (active-FIRST, so they
    are published per row, not via `hover_surface`).
  - `polkit_auth`: the shield, the two centered lines, the user, the 36-tall
    field at y 164 (a `Stack` so the `Hit` key 92 rides the same box), the
    caret bound to a published `polkit_cursor_x` (`Surface.x_var` — the
    measured run `36 + 13 · 0.62 · len`), the error line gated on
    `polkit_err_on`, and the Cancel/Authenticate pair as a `bottom: true` `Row`
    (`y: 24, h: 36` — `bottom` anchors the row's OWN height, so `h` must be
    declared) of two 120 × 36 `Stack`s, 16 apart, centered. The publisher
    clips the message/action with the drawer's own formula at the LIVE dialog
    width (`Mode::PolkitAuth.size(&self.cfg)`, so scene and drawer cannot
    disagree) and publishes every hover/focus/verifying fill as a color:
    `polkit_pw_bg` / `polkit_pw_ink` / `polkit_cancel_bg` / `polkit_auth_bg`
    (a 15% white blend on hover, the drawer's own `blend`), plus
    `polkit_auth_label` (Verifying… while checking) and `polkit_err_ink`. The
    keys are now `pub(crate)` (`POLKIT_KEY_AUTH` 90 / `_CANCEL` 91 / `_PW` 92)
    so the publisher names them instead of hard-coding 90/91/92.
  - Guarded by the same `shell_ron_first_batch_matches_its_rust_drawers` (it
    now covers all six surfaces: the card grid, the rail, the captions, the
    field, the button pair, the caret and all three polkit keys). 403/403.
- Declared-lite deltas vs the Rust chrome (first cut): the sidebar's faint
  bg tint and the selected-nav 2 px accent rail are not yet declared (no
  alpha-8 token); ✕ glyph stays fg2 (not hover-whitened); chips have no
  hairline outline (the DND chip fill is flat beyond the guarded hover
  blend); lock chrome is authored at the 1080p reference (non-1080 heights
  use Rust); wallpaper back button gained a hover-only fill.

### Next step (Phase A — batch body conversions) — Batch 1 DONE 2026-09-16
Layout containers are in; card bodies can now be converted to fully-declarative
RON using `Row`/`Column`/`Stack` for structure (no absolute x/y). Start with the
text-only batch that needs no new primitives:

| Primitive | Purpose |
|-----------|---------|
| `Rows { name, y, row_h, key_base, cols[] }` | Dynamic row list from named `Vec<SceneRow>` |
| `Spark { name, x, y, w, h, color, max }` | Line/column chart from named `Vec<f32>` |
| `Ring { name, cx, cy, radius, max, fill, track }` | Circular gauge from `f32` |
| `Fader { name, x, y, w, h, fill, key, action }` | Slider (volume/brightness) |
| `Toggle { name, x, y, w, h, key, action }` | On/off switch bound to `bool` |
| `TabRow { name, sel, y, key_base }` | Horizontal tab bar for panels |
| `Row` / `Column` / `Stack` | Layout containers (nestable, zero-dim fills parent) |

`SceneValue` enum replaces the flat `HashMap<String, String>` in
`scene_values()`: `Text(String)`, `Rows(Vec<SceneRow>)`, `Spark(Vec<f32>)`,
`Ring(f32)`, `Toggle(bool)`, `Fader(f32)`.

Convert card bodies in batches:
1. **Batch 1** (text/list cards): quote, ticker, news, todo, snippets, notes,
   lyrics, currency, recent, alarms, countdown, expenses, calendar, bluetooth,
   sshvpn, appshortcut, mirror — list rows via `Rows(body)`, chrome via
`Header`/`Text`/`Hit`, scroll via the `start` binding. DONE: bluetooth,
    ticker, currency, sshvpn, recent, todo, alarms, notes, countdown, quote,
expenses, news, snippets, lyrics. Remaining: calendar, appshortcut,
    mirror.
   Engine gaps for these: 🟢 composers LANDED (2026-09-16 — `Composer`, the
   todo strip is the reference `todo_focus`/`todo_buf` split); 🟢 hover-✕
   delete + live checkbox cells LANDED (2026-09-16 — `Rows` `del_base`/`del_pad`
   reveal + `Checks` checkbox column; ✕ parities captured via `del_dy`/`del_pad`
   — todo/alarms are the references, alarms also proves key-less rows still
   reveal on the ✕); 🟢 two-baseline rows + stateful list hiding LANDED
   (2026-09-16 — `SceneCol.dy` per-column baseline offset, `Rows.visible`
   bound Toggle; notes is the reference, list hidden while its composer Ink
   is open, later items still draw); 🟢 chip/pill cells + truncation +
   right-edge anchor LANDED (2026-09-16 — `SceneCol.pill`/`truncate`/`edge`/
   `del_anchor`; countdown is the reference — per-row chip ink from
`col_colors` derives the fill, pills pre-unblock news/expenses/quote);
   still need:
     equal-width grid cells (calendar, appshortcut); image cells (appshortcut);
     camera feed (mirror).
   (lyrics' karaoke word highlight is now LANDED — see 2026-09-16 CHANGELOG.)
2. **Batch 2** (need Rows/Spark): procmon, journaltail, systemdunits,
   smarthealth, sensors, thermbl_zones, conninfo, fans, diskio,
   network, topproc.
3. **Batch 3** (need Fader/Ring/Toggle): sliders, eq, powerh/powerv,
   batteryh/batteryv (Ring), toggles, compositor, sshvpn. — all done except
   `eq` (parked) and `sshvpn`.
4. **Batch 4** (special): clipimg, docker, worldmap, viz, accent, gauges, cpugpu.
   — **2026-09-27: cpugpu LANDED** (zero-`Ink`: the two `When(max_w: 170)`
   legend/plot branches, both `Spark`s per branch with `max: 1.0` over the
   already-normalized `cpu_spark`/`gpu_spark`, `cpugpu_gpu_live` gates the GPU
   series + its legend dot, mode-adjusted inks published as
   `cpugpu_cpu_ink`/`cpugpu_gpu_ink`).
   — **2026-09-27: speedtest LANDED** (zero-`Ink`: header + three state
   captions, the bottom-anchored 30 px run pill and its hit region, gated on
   the shell's `st_state` / `st_clickable`; the new `Surface.bottom` anchor
   keeps the drawer's 8 px lift declarative. See the 2026-09-27b CHANGELOG
   entry for the span convention that changed with it).
   — **2026-09-27: micmeter LANDED** (zero-`Ink`: the 8 px level bar, the
   status caption, the bottom-anchored mute button with the drawer's 3 px
   click halo and its `MIC_MUTE_KEY`; the shell publishes `mic_lvl`,
   `mic_bar_ink`, `mic_status`/`mic_status_ink`, `mic_btn_fill`/`mic_btn_ink`
   and `mic_btn_label`. See the 2026-09-27c CHANGELOG entry — the same pass
   fixed `Text.center_x`, which every empty-state caption relies on).
   — **2026-09-27: docker LANDED** (zero-`Ink`: one `Rows` row per container
   with a new `dot` cell (the Up/down ink rides `col_colors`), the name, and a
   20-char image label via the new `max_chars` cell cap; the drawer's row cap
   is `bottom: 4.0` and the empty state is one centered caption).
   — **2026-09-27: alarms LANDED** (zero-`Ink`: header + the drawer's
   right-anchored `HH:MM · ` meta, the composer as a declarative `Hit`, and
   one `Rows` row per alarm with the hover ✕ delete; `alarm_rows` is capped at
   the 16 delete keys the ✕ can address).
   — **2026-09-27: quote LANDED** (zero-`Ink`: the `TextWrap` body, the two
   centered 88 px pills, and the loading / no-quote captions; the `saved`
   flash countdown stays in the Rust body as frame state).
   — **2026-09-27: wifi LANDED** (zero-`Ink`: the four-bar signal staircase
   via the new `bars` cell and the padlock via the new `suffix` cell, plus a
   new `SceneCol.hide` for the padlock's carrier column. The row wash chain —
   connected → acc tint, else hovered → hover — is published per row because
   a hit key is a window index. See the 2026-09-27e CHANGELOG entry).
   — **2026-09-27: worldclock LANDED** (zero-`Ink`: the cities are a
   declarative `Rows` — glyph · city · right-anchored mono `HH:MM` · offset
   chip — the `+ add city` row is a bound `Hit` + two `Text`s, the search plate
   is a `Surface` with the buffer and its `esc` hint, the matches are a second
   `Rows` whose rows carry their own keys, and the empty result is one gated
   `Text`. The match list is the pure `Shell::worldclock_matches()` shared by
   the scene value, the input key resolver and the fallback painter; the clock
   is sampled once per frame into `worldclock_epoch`. The row cap is
   `bottom: 34.0`. See the 2026-09-27n CHANGELOG entry — it also fixed the
   `Rows` default `pitch`, which squared `grid_scale` for every list that
   omitted it.)
   — **2026-09-27: branding LANDED** (zero-`Ink`: the aspect-fit mark, now
   through the new `Text.size_var` + `Text.y_var` bindings — the shell
   publishes the fitted size and top in base px and re-reads `$states2/d`
   itself, since the drawer no longer runs).
   — **2026-09-27: latency LANDED** (zero-`Ink`: the ping series as a
   `Spark(kind: line)` over the shell's pre-normalized history, the hero ms
   readout + `ms` unit on `cy_pct: 0.5`, the status word on `bottom: true`,
   and the re-probe pill + its 3 px halo. `Divider` grew the three rules it
   needed to be usable here: a negative `w`, `cy_pct`, and a `visible` gate.)
   — **2026-09-27: countdown + workspaces LANDED** (both zero-`Ink`, 375/375).
   Countdown: the composer is a `Surface` + `Text` + an invisible 24 px-tall
   `Hit` on `31500` beside the `countdown_rows` `Rows` (which had already
   landed), and its header chrome now comes from the shared `card_head`, so
   the fallback title/meta moved up 1 px and the meta's ink went `fg → fg3`
   and 9 → 8.5 px. `countdown_rect` is write-only, so it needed no plumbing.
   Workspaces: `Grid` grew the tile mode it needed — `cols_break` (the
   drawer's `(w − 24) / 34` ladder written out and clamped to the cell count),
   `cell_max_w` / `cell_max_h` + `center` (cap the tiles, then center the
   block), `cell_w_break` (the per-tile 9 → 11 px label ladder, which only
   ever RAISES the base size, exactly like `cols_break`), and the per-cell
   `marker` pip + `vcenter` optical centering that the drawer's active tile
   needs. `ws_fill_act` publishes the drawer's straight 16 %-alpha accent wash
   (per-frame, so a per-cell `surface_hover` has to repeat it to survive the
   cursor) and `ws_tiles` publishes the cells.
   — **2026-09-27: pomodoro LANDED** (zero-`Ink`, 380/380). The mm:ss figure,
   the phase word, the progress bar, the `[-] 25m [+]` trio and the
   Start/Pause + Reset pair are declared; per-frame values carry the figure,
   the phase ink, the bar's fill/fraction, the readout and the three MIXED
   button fills (a token can't express `mix(hover, acc, 0.25)`). `Hit` grew
   three rules for it: `center_x` (box left edge on the center line — the
   control pair straddles the middle with a 6 px gap, which a centered `Row`
   block can't express), `x_var` (a bound scalar replaces `x`, so the stepper
   chips measure the runtime readout's width via the shared
   `Shell::pomo_readout_w` instead of assuming a digit count), and
   `label_dy` (nudge the label inside a FIXED box — `dy` folds into the box
   itself, which can't express the drawer's `ty = y + (bh − fs) / 2`).
   — **2026-09-27d: audiodevice's mute/hover precedence LANDED** (the last red
   test in the shell; 385/385). The drawer's vol-% rule tests MUTED FIRST, so a
   muted row's pct never tints — which a `SceneCol.hover` token cannot express,
   since it outranks every published color during hover. The ink is published
   per row (`col_colors[1]`) and the col drops `hover`. See the 2026-09-27d
   CHANGELOG entry.
   — **2026-09-27e: accent's PICKER LANDED** (the subview stays in the Rust
   drawer — paged scroll + the `accent_list_rect` write-back — behind
   `accent_picker`). `DotSpec` grew `h` so a `dot` cell can be a 3 × 12 rail,
   a 0-alpha dot paints nothing, and `Hit` / `Rows` grew `y_var` for the
   block's centered `clamp(30, 48)` anchor (`accent_rect`, published in base
   px). See the 2026-09-27e CHANGELOG entry.
   — **2026-09-27f: media LANDED** (zero-`Ink`): the album art as a declarative
   `Image` (`file:<path>`, and an empty key paints nothing, so the rounded
   `Surface` placeholder can share the box), the semibold title + album/artist
   line, the seek `Bar` with its time labels, and the three transport buttons.
   `Surface` grew the bound-`Toggle` gate, `Text` grew `x_var` for the
   icon-face glyphs that straddle `card_w / 2 ± 36`, and `media_seek_rect` is
   recovered from the declared seek `Hit` so drags survive the drawer standing
   down. The `wide` / `has_room` gates are computed in DEVICE px (the drawer's
   own breakpoints) and the `fit()` cap is published pre-trimmed. See the
   2026-09-27f CHANGELOG entry.
   — **2026-09-27g: clipimg LANDED** (zero-`Ink`) and the IMAGE CELL it was
   blocked on exists: a `SceneCol` cell whose text is the renderer's image key
   over a rounded tile (`ImageCellSpec` — tile 24 × 22 r5 at −1, the 18 × 18
   paste 3 px in), so the picture lists no longer need a whole `Ink` body.
   `Rows` grew `surface_dy` / `surface_dh` for a band inset from its row, and
   the list publishes every entry and windows on the scroll (binding the
   scroll while publishing only the visible window applies the offset twice).
   See the 2026-09-27g CHANGELOG entry.
   — **2026-09-27h: powerdraw LANDED** (zero-`Ink`): two `Spark` traces (the
   `latency` card's `ui::pulse` item) over the drawer's own plot box, with
   the last sample of each channel beside its B / G dot. The one-sample flat
   midline rides the existing `cy_pct` anchor — `y: 9` + `cy_pct: 0.5` IS
   `plot_y + plot_h/2` — and `Spark` / `Divider` grew `min_w` / `min_h` for
   the drawer's plot floors. See the 2026-09-27h CHANGELOG entry.
   — **2026-09-27i: viz LANDED** (zero-`Ink`): a new `Spectrum` item
   (rounded bars / blocks / wave) over one zero-padded series, one gated item
   per `viz_style`, and `Header` grew `title_size` / `title_dx` for the
   drawers that hand-roll their own header. See the 2026-09-27i CHANGELOG entry.
   — **2026-09-27j: system LANDED** (zero-`Ink`, no `Header` — its
   chrome is its body): `Hit` grew `region_dx`/`region_dy` (the click halo
   around a visible pill) and `w_var` (a pill that tracks a measured string),
   and the state-driven inks are per-frame `value(...)` colors. See the
   2026-09-27j CHANGELOG entry.
   — **2026-09-27k: weather LANDED** (zero-`Ink`, two panes): the centered
   glyph+temp *pair* rides a published `x_var`, the 7-day list is a `Rows`
   whose row COUNT and centered top are the drawer's own height arithmetic
   (published pre-clamped + `y_var` — a static `max_rows` cannot express a
   per-box fit), and each conditional line publishes its own gate: an empty
   string does NOT hide a `Text`, and one gate per item means the pane and the
   line's condition are pre-ANDed. `weekday_name` went `pub(crate)` so the
   rows share the one rollover. See the 2026-09-27k CHANGELOG entry.
   — **2026-09-27l: moon LANDED** (zero-`Ink`): a new `Moon` item for the
   terminator disc, whose geometry is the drawer's OWN function moved to
   `ui::moon_disc` (one implementation, two callers — a primitive with two
   copies drifts), with `x_var`/`y_var`/`d_var` because the diameter is a
   min/max over both card axes. `moon_epoch` is now sampled once per frame
   instead of twice by the header and the body, and `moon_meta` stopped being
   a third copy of the epoch constants. See the 2026-09-27l CHANGELOG entry.
   — **2026-09-27m: wallpaper LANDED** (zero-`Ink`): the tile grid is a `Grid`
   with its column count, cell size, scroll offset, row fit AND block origin all
   bound — the card shrinks the cell until ≥5 columns and ≥2 rows fit, which no
   `cols_break` ladder can express (a ladder caps a max cell; this keeps a min
   count). The fit chain is now ONE pure `wp_card_geometry`, and the state
   settles in the `&mut self` layout pass (`wp_card_settle`) because
   `scene_values` is `&self` and must publish the CLAMPED scroll without moving
   it. `Grid.max_rows_var` (the `Rows` `max_rows` rule, bound) closes the
   window: `start` says which cells are on the page, this says how many rows it
   holds. Seven new binding fields in all (`Grid` x/y/cols/cell/surface/
   image_inset/max_rows, `Hit.h_var`, `Surface`'s four edges). Also: an empty
   `Header` meta is no meta, and `ladder_step` stopped double-scaling (it
   returned device px while both callers scale again — invisible at
   `grid_scale = 1.0`, which is why the new test's 1.75 pass caught it). See the
   2026-09-27m CHANGELOG entry.
   Remaining: worldmap
   primitive), branding's sibling `brand_glyph` consumers, system
   (bespoke two-column menu).

Engine note (2026-09-27): items authored AFTER the last `Ink` are redrawn in
scene order — before this, the pre-pass dropped the final `Ink`'s buffer and a
card that declared `Ink` + `Rows` (notes, countdown) showed no list at all.

Engine note (2026-09-27): a zero-`Ink` card should carry a DIFFERENTIAL
parity test — draw the Rust drawer and the scene from the same `Shell` (real
state, real palette, real `scene_values()`) and compare the two command lists.
The pomodoro's first version passed all four hand-computed geometry tests with
its `[+]` chip 6 px off, because those numbers were derived from the same
misreading of the drawer's algebra; the differential test caught it in one
run. Copy `the_pomodoro_scene_matches_its_own_drawer` (and the `fmt`
fingerprint helper next to it) for the next conversion.

Engine note (2026-09-27): a `Grid` label centers on its own cell. `draw_text`
takes a BOX and centers inside it (`x + w/2`), so the cell must pass its left
edge — handing it the already-centered anchor (as the label and `sub` lines
used to) landed every grid label half a cell to the RIGHT of its cell, which
is where the calendar's day numbers had been sitting. The region box was
always right, so the existing tests (which pinned hits, not label `x`) passed
straight through it.

Finally: remaining panels/modes declarative → remove Ink fallback. In
progression: `settings` chrome is live (2026-09-15), `notif` chrome + the
state-driven DND chip via dynamic tokens (2026-09-16), then `lock` (hero
chrome at the 1080p reference) + `wallpaper` (header chrome) went live
(2026-09-16, second pass); the `dashboard` surface went LIVE in the viewing
state (2026-09-16, third pass — board `dash_grid` + banner `dash_banner`
Inks route to their exact Rust painters; banner chrome declarative + the
card grid will be the next conversions), and finally the transient
popups/OSDs.

---

## ▶ NEXT SESSION (resume 2026-09-25)

State at handoff: **304/304 tests green** (was 258 — see the test-suite note
below), the bin builds clean, and **Batch 3 has five cards done** (sliders,
toggles, compositor, power, battery). Batch 1, Phase B, Phase 2a, Phase C and all of Batch 2
have landed; `CHANGELOG.md` is the record of what shipped.

### Build recipe (this box needs it — read before trusting any test run)
The rustup in this session is empty/unwritable, and **cargo's mtime-based
freshness check silently skips rebuilds here** — it reports `Fresh` for a file
you just edited, so a green run can be a stale binary (this is how ten tests
hid for weeks). Before any verification:

```bash
export PATH=/home/persist-ur/user/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:/usr/bin:/bin
export CARGO_HOME=/home/persist-ur/user/.cargo
export CARGO_TARGET_DIR=/tmp/opencode/zen-shell-target
cd /home/tw/.config/hdots/rust_project_root/zen-shell
find src -name '*.rs' -exec touch -d '+1 hour' {} +   # strictly-newer mtime ⇒ real rebuild
cargo test
```

`cargo clean -p zen-shell` (≈28 s) is the sledgehammer if it still says Fresh.
A warm rebuild is ~3-28 s depending on how much changed.

### Where things stand
- **Sliders (Batch 3 head) is zero-`Ink`**: `ui/cards/sliders.ron` — pane 0's
  six fader rows in a `Column(valign: middle)`, pane 1's two switch rows, hover
  `Dots`. Each row is a `Stack` (row-wide `Hit` + icon, the `Fader`, the
  right-anchored label); balance/saturation carry `visible` gates.
  `draw_card_scene` mirrors each fader's resolved geometry into `faders[]` /
  `balance_rect` so the drag math in `src/shell/input.rs` (keys 51-55, 59)
  keeps working — **any future card with a fader must do the same**.
- **Engine gained, this session**: `Fader.head` / `Fader.centered` /
  `Fader.visible`, `Column.valign`, `span_px` on fader/row/column/stack/
  toggle boxes, `Hit.visible` (a gated row's region leaves with the row), the
  `Tiles` primitive (responsive `cols_break`, `label_min_h` caption floor,
  on/hover/off ink ladder, corner chip) and the right-anchored-`Text`-in-a-
  container fix (see gotchas).
- **A silent config-rot class is now guarded**: `every_live_card_scene_parses`
  walks `ui/cards/*.ron` and parses each. A scene that fails to parse falls
  back to the Rust drawer with no error, so keep that test green; it caught
  `activewin.ron` / `pkgupdates.ron` / `clipboard.ron` writing
  `visible: "x"` where the schema wants `visible: Some("x")`.
- **Test-suite note (why 275, not 258)**: two unterminated `r#"` raw strings
  in the test module of `src/scene.rs` swallowed ~240 lines of test code, so
  ten committed-scene tests never compiled into the harness and one was
  failing on a 240-line "RON" blob. Both terminators are restored. When adding
  a test with a multi-line RON, sanity-check that the string ends where you
  think: a missing `"#` fails *silently* by eating the rest of the file.

### Batch 3 progress (new `Tiles` primitive, 35 `Ink` cards remain in the live tree)
- **sliders — DONE** (see above; zero `Ink`, six faders, two panes, dots).
- **toggles — DONE** (zero `Ink`): one `Tiles` frame; the six switches arrive on
  the `toggles_tiles` list, the frame owns the responsive columns
  (`cols_break: [(225.0, 2), (330.0, 3)]` = the drawer's width ladder), the 42 px
  caption floor, the on/hover/off ink ladder and the Wi-Fi / BT ⋯ chip.
- **powerh / powerv — DONE** (zero `Ink`): header + the five power actions,
  each a `Stack` of a declared background + hairline (`Surface.stroke`), the
  hold-to-confirm liquid (a `Bar(vertical: true, max: 1.0)` gated on the
  shell's own 1.5 px sliver threshold) and the 14 px glyph, with the resident
  keys 12000..12004. Three generic engine additions came with it:
  `Row.halign`, `Bar.vertical` and `Surface.stroke` / `stroke_color`.
  Per-frame: `power_bg_N` (raised → raised_hl → acc_tint), `power_hold_N`,
  `power_hold_on_N`, `power_ink_N`, `power_hold_fill` + a new global `clear`.
- **toggles — DONE**; **compositor — DONE** (zero `Ink`): header + three effect
  tiles in an equal-column `Row` of `Stack`s + the two screenshot buttons, with
  the per-tile fill/dot/label/caption inks published per frame (a lit tile's
  fill is the accent at 16% alpha — a blend no token spells). New
  **`Row.min_card_h`**: the shot row needs a 110 px card, so a short card drops
  the row instead of overflowing (the drawer's `if sy + sh <= y + h - 2`).
- **batteryh / batteryv — DONE** (zero `Ink`): the decision from last session
  was taken — a new generic **`Battery`** scene item now owns the vector gauge
  (its art moved from `cards/shared.rs` into `scene.rs`, so the scene draws
  the drawer's own geometry), plus the centered percent, the pane-1 metric
  rows (`Rows`, deliberately inert), the power-save pill (`Hit.bottom` +
  negative span) and the pane dots. Two more engine changes came with it:
  `Hit.bottom` and `Rows.key_base: Option<u32>` (a list that omits it registers
  NO regions — bare `index` keys used to squat on the global 1 / 2 / 3 keys, in
  16 committed scenes). New `Shell::power_save_state()` /
  `Shell::battery_dots_hover()`; `draw_card_scene` now publishes
  `battery_h_rect` / `battery_v_rect` in its zero-`Ink` branch.
- Remaining:
  1. **eq** — parked: zeneq is halted, decide before spending time here.
  2. Whatever the user picks next out of the 35 `Ink` cards; the hover/pane
     helpers (`pane_dots`, `card_pane_flip_n`) now have zero `Ink` consumers, so
     they can move into `scene.rs` whenever the last drawer that calls them is
     converted.

### Gotchas worth re-reading before starting
- **Centered text inside a container**: `center: true` with `w: 0.0` and
  nothing else. The container's fill pass hands the text the full inner width
  and `draw_text` centers inside that box, so it lands on the parent center;
  adding `center_x: true` (which re-anchors the box's LEFT edge to the parent
  center) pushes the glyph a half-width right. `Surface.center_x` is the other
  way round — it places the box's left edge on the center, so a 5 px dot needs
  `x: -2.5, center_x: true` to sit centered.
- **Right-anchored text inside a container**: `right: true` anchors at
  `parent_right - x`, and `w` is NOT a layout slot for it — but a container
  used to rewrite any child with `w: 0.0` to the fill width, which shoved the
  label a whole box-width right. Right-anchored `Text` is now exempt; if you
  nest one, write `x: <margin>, w: 0.0, right: true`.
- **`Hit.right: true`** is different: its region is `parent_right - (x + w)`,
  so a right-anchored hit needs BOTH `x` and `w` (e.g. a reset chip:
  `x: 14.0, w: 18.0` for an 18 px region ending at the card inset).
- **A `Hit`'s region y rides its text baseline** (`sy = y + (y + dy)`), so a
  small icon hit's region is shifted down by `dy`. Fine for icon rows; for a
  chip that must stay inside its row, shrink `h` so the shifted region fits.
- `Rows.w` = inner box width (pad applies both sides); `Rows.pitch` = stride
  ≠ `row_h`; `row_dy` = cell baseline within row; `Rows.bottom` reserves the
  last N px (a row whose box crosses that line is skipped). `Rows.key_base` is
  `Option<u32>` and takes the explicit `Some(n)` wrapper: omit it and the rows
  are pure text, no regions — an info list must never register keys. A row with
  its own `key` still opts in.
- **A card-level `Text` has NO implicit width**: `w: 0.0` centers on the
  anchor itself, so a centered caption at card level needs a full-card
  `Stack` wrapper (`w: 0.0, h: 0.0`) to inherit the card width — inside a
  container the fill pass does that for you. `bottom: true` takes a POSITIVE
  offset (`y: 22.0` = 22 px above the card's bottom edge, NOT `-22`).
- **`Hit` honours negative spans and `bottom`**, and every arm shares ONE span
  rule (2026-09-27b): a negative `w`/`h` is the span left from the item's own
  `x`/`y` minus that inset, so the power-save pill is `x: 12.0, w: -38.0`
  (12 px in, 38 px short of the right edge) on any card width, and `w: 0.0`
  reaches the box's far edge from wherever the item starts. `bottom: true`
  anchors the box's BOTTOM edge `y` px above the card's bottom (a POSITIVE
  offset). The pill's 3 px click halo is the one deliberate deviation from the
  Rust drawer.
- **Fold a guard into a gate, don't gate twice**: an item takes ONE
  `visible` name, so the battery cards' pane gates publish
  `pane_N && present` — that is exactly the drawer's early return, and
  `batt_absent` covers the empty state. An UNBOUND name is not "closed", it is
  "always open" (`gate_open`), so a typo'd binding draws silently.
- `Surface`/`Text` `center_x`/`center_y` = the x/y fields become base-px
  offsets from the PARENT BOX's center.
- RON requires every non-`serde(default)` field to be listed explicitly
  (`x: 0.0` / `y` are mandatory on most items), and **unknown fields are
  ignored silently** by serde — a typo'd field name is a no-op, not an error.
- **"The rest of the card" is a NEGATIVE span**: a box that should reach the
  card's bottom edge is declared `h: -30.0` (parent minus the inset — the same
  trick as the compositor's `w: -24`). `h: 0.0` spans the FULL card height from
  its own `y`, which pushes a centered block 30 px too low.
- **An `Option<ColorToken>` field needs the explicit `Some(...)` wrapper** in
  RON: `stroke_color: Some(hairline)`, `color: Some(value("power_ink_0"))`,
  `visible: Some("gate")`. A bare token works only for non-`Option` fields
  (`Surface.color`, `Bar.track`, `Bar.fill`). A missing `Some` fails with
  `Expected option` — and unknown/mistyped fields are still ignored silently.
- `Bar` reads its `value` against `max` (default **100**), so a 0..1 fraction
  needs `max: 1.0`. Its `track` always paints — bind a transparent track
  (`track: value("clear")`, the shell's global `clear` = `Raw(0)`) when the bar
  is a bare fill.
- `Cmd::Text`'s `y` is the text's TOP, even for `ui::text_c` — "centered" only
  means horizontally. Copy the drawer's y verbatim (a 14 px glyph centered in a
  32 px button is `y: 16.0`).
- Scene values are the ONLY place per-frame dynamic colors/labels may be
  computed (`scene_values()`); tokens/strings in `.ron` are static.
- **A cell's `hover` token is absolute: `col.hover.or(base)`.** It beats every
  published value, so a drawer whose hover rule has a HIGHER-precedence
  condition — audiodevice's vol % tests `muted` first, so a muted row's pct
  never tints — cannot be expressed as "static color + hover token". Drop the
  token and publish the whole chain per row (`col_colors`) instead, hover
  included; a hovered muted row is then just a different published color. The
  row-key test covers an `aux` box on its own, since `aux` is `key + 1`.
- **`DotSpec` is square by default but grew `h`** (`Some(12.0)` → a 3 × 12 rail,
  the accent card's active source; unset → a square of `size`, so every
  existing dot cell is unchanged). A dot cell paints NOTHING when its resolved
  ink is fully transparent (alpha 0) — `ColorToken` has no transparent variant,
  so a row that must not show one publishes `clear` by name, and an invisible
  rect is not a command the drawer would have emitted.
- **`Rows.pitch` is an `Option<f32>`**: `pitch: 30.0` is a parse error
  (`Expected option`), `pitch: Some(30.0)` is right. Same for every other
  `Option` field in a tuple spec.
- **`Hit` and `Rows` have `y_var`** (a bound scalar REPLACES the declared base
  `y`, the `Text` rule) for a list or a region that rides a height-dependent
  anchor; a `Text` has `y_var` but no `dy`, so a per-item inset has to ride
  inside the published scalar. Record the card box in the layout pass first
  (the `branding_rect` / `wifi_rect` precedent) — the Rust drawer that used
  to measure it stands down once the scene owns the rows.
