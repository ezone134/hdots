# Changelog

## 2026-10-02 (phase 5 — `anim:`: a field can now declare its own easing)

Landed **phase 5** of `QUICKSHELL_MODEL.md` §8. The tree reads **508 passed / 0
failed / 0 ignored** (was 501 passed + 1 ignored), zero warnings.

`x: (val: "polkit_cursor_x", anim: (ms: 90))` — the wrapper is a **modifier on
the value it wraps**, not a new item field, so no existing scene changed shape.
Pilot: the polkit password cursor, which jumps ~8.06 px per keystroke and was
the most legible sign that a password field is still a Rust hack.

- **The table is keyed by BINDING SOURCE, not by item path.** Two items reading
  `nav.tab` are tracking one value and must move together; a per-item key would
  let them disagree for a frame. Inside a `Repeat`, substitution has already
  rewritten each row's source to its own `item.<field>` spelling, so rows are
  distinct without threading a path through `draw_piece`.
- **The first frame of a key rests AT its target.** Only *subsequent* target
  changes animate. `Entry` therefore keeps `target` even while settled — without
  it every frame looks like a first sighting and nothing ever moves. A retarget
  restarts from the *current* value, so an interrupted animation never snaps
  backwards, and `EPS = 1e-4` stops float noise from keeping a cell alive
  forever.
- **Measure reads the TARGET, never the eased value.** An animated `w` that
  measured its in-flight width would resize the very box that positions the
  item, so the two would chase each other and the layout would never settle.
  Because that makes an animated extent *silently inert* — laid out at its final
  size every frame — the validator **refuses** `anim:` on `w` / `h` /
  `font_size` outright (`AnimatedMeasuredField`) rather than accepting it.
- **`set_now` is stamped once per surface, not per field**, so two fields easing
  on one item read the same instant instead of differing by the microseconds
  between their reads. The shell drops the table when scene text changes under
  it: a rebind must not ease out of the old target.
- **The render loop has to be told to stay awake, or a glide freezes.** The
  shell is event-driven, so `App::anim_timer` (16 ms, self-stopping off
  `settling()`) is armed at the *end* of `maybe_render` — after the draw that
  starts the easing, since checking earlier would arm one frame late on first
  movement. Without it an `anim:` jumped mid-move on an idle screen and settled
  only on the next keypress.
- **`Val`'s `Deserialize` is hand-written — but NOT for the reason this phase
  first recorded.** The blocker was measured against ron **0.9**, where untagged
  buffers through serde's `Content` and a nested unit-variant enum does not
  survive the round trip; re-probed against the pinned **0.12.2** the derive
  carries `curve:` fine, so the measurement was right and the version was stale.
  The hand-written impl still earns its place on the two things untagged cannot
  do here: it accepts **`anim: ()`** (untagged does not match it *at all* — the
  whole `Val` fails, pointing at the item rather than the animation), and every
  real mistake says what it is instead of "data did not match any variant of
  untagged enum Val" — the same reason phase 2 hand-wrote `PropSet` and
  `SceneComponent`. `untagged` **stays on `Serialize`**, so a scene written back
  out still reads as the same document; a test now pins that round trip, because
  dropping the attribute there breaks five existing round-trip tests at once.
- **A map missing `val` or `anim` is an error, not a default** — and so is
  `x: ()`, which serde would otherwise report as "found a unit value instead".
  The alternative to complaining is an animation nobody declared, at a duration
  nobody chose, on a field the author believes is a plain constant. An unknown
  key is ignored, so a scene written for a newer build still loads.
- **Four tests were sitting in the tree without `#[test]`.** `anim:` shipped
  with three of its own tests (plus `ron_roundtrip_parses`) missing the
  attribute — dead functions that compile, run nothing, and cost nothing. They
  were the tests that would have caught the two bugs found on the way through
  (`color: "text"` is not a `ColorToken`, and the two fixtures omitted a
  required field), and they are why the suite said "501 passed, 1 ignored" while
  the animation feature was untested end to end. **A test without `#[test]` is a
  comment**, and the `dead_code` warning it leaves behind is the only thing
  telling you so.
- **The phase's original "done when" was not reachable through an item `anim:`
  and §10 now says so.** The pill expand is *surface geometry* (`Shell::anim` →
  `cur_w` / `cur_h` — the window morphs, no scene item owns it), and the
  dashboard glide is computed inside the Rust `dash_grid` ink off
  `packed_layout` (`card_anim` + the `edit_settling` flag that exists only to
  keep the loop awake). Neither is a field easing toward a bound target; both
  stay in Rust, and `anim:` is not the tool that deletes them.
- **A hit key's fingerprint now derives identity through `binding()` /
  `constant()`**, so an animated field and a plain one over the same name share
  a key — otherwise wrapping `x:` in `anim:` would have re-registered a hit
  region as a new claimant and tripped the duplicate-key check.

## 2026-10-01 (phase 4 — `store` + `Set`: the first state the scene owns)

Landed **phase 4** of `QUICKSHELL_MODEL.md` §5. The tree reads **489 passed / 0
failed** (was 468 — 21 new tests, zero warnings).

Settings nav selection is now the scene's. The `settings_nav` rows carry
`Set(name: "nav.tab", value: <index>)`, the `settings` surface declares
`stores: [(name: "nav", props: (tab: 0.0))]`, and Rust's six-arm
`match key { 200 => SettingsTab::Network, … }` table is deleted.

- **Stores live on the surface, not in the item list.**
  `SurfaceScene.stores: Vec<StoreDecl>`. An item was the wrong home: a store
  draws nothing, so it does not belong in a layout list, and `Comp` inlining
  would have copied one into every template that used it.
- **The scene-action arm had to be ABOVE the Rust arms, not the handlers
  deleted.** `scene_click_actions` is the path for keys no Rust handler *above*
  it owns, and the settings arms already sat below the scene arm — so the
  correct move was to leave them as the fallback rather than remove them. With
  no `settings` surface declared the rows do not exist, no key is claimed, and
  the Rust-only layout's own sidebar must still switch panes.
  `Shell::settings_tab()` reads the store; `settings_tab_fallback` is only read
  when no scene declares `nav.tab`.
  `a_claimed_key_never_reaches_the_rust_fallback_handler` asserts that ordering
  on the source, because no behavioural test can catch a regression there: both
  paths produce the same tab, so a reordering would keep every existing test
  green while quietly making the sidebar non-declarative again.
- **Legality is decided against the DECLARATION, not the live value.** The live
  map is empty until seeded, and seeding is idempotent *by design* (a `Set` has
  to survive the next frame's seed), so the first version's `None => accept`
  branch made an unseeded store accept a write to any name at all — the refusal
  depended on load order. `SceneStoreState` now keeps `decls` beside
  `by_surface`, and a hot reload that retypes a prop replaces its declaration
  wholesale rather than accumulating.
- **One type predicate for both directions.** `store_write_type_ok` is called by
  the runtime applier and by the load-time validator, deliberately not two
  copies: if they disagreed, the validator would pass a scene the runtime
  refuses, and a refused write looks exactly like a click that did nothing.
- **The validator learned the closed set.** `LoadIssue::BadStoreWrite` (a `Set`
  naming no declared prop, or one of the wrong type — the message lists what IS
  declared) and `LoadIssue::UnknownStoreProp` (a `<store>.<prop>` binding typo).
  This is phase 3's surviving half: a store's names cannot be
  absent-this-frame the way a flat-pool name can, so they are checkable. The
  walker's blanket skip of dotted names is narrowed to `item.<field>` — which
  `UnknownRowField` judges better — and to namespaces the validator cannot see
  (`mod_clock.time`), so a plugin name is still not reported.
- **Reads are surface-scoped, and that was a real leak.** `scene_values()` was
  merging *every* surface's stores into one pool; two surfaces naming a store
  `nav` silently shared one value. Every panel and surface draw now builds its
  pool through `scene_values_for(surface)`, which merges only its own, and
  `scene_values()` is `#[cfg(test)]` plus the validator's whole-world view.
- **`PropValue::Bool` and the scalar rule it exposed.** A store bool publishes
  as `SceneValue::Toggle`, and `SceneValues::scalar` did not read `Toggle` — so
  `w: "editing * 20"` collapsed to 0. A flag read as a number is 1/0, which is
  what `PropSet::num` has always done for component props; a store prop and a
  component prop are the same kind of value and must not disagree. A `true`
  reading as 0 hid behind any binding of the shape `x: flag`.
- **A refused `Set` now says so** on stderr. It was silently ignored, and the
  symptom — a control that does nothing — has no other cause.

## 2026-10-01 (phase 2 — component props: a Rust `match` table became RON)

Landed **phase 2** of `QUICKSHELL_MODEL.md` §4. The tree reads **468 passed / 0
failed** (was 449 — 19 new tests, and zero warnings, up from four).

The failure this fixes is not a missing feature so much as a misplaced decision.
`subhead` — the one header the Wi-Fi / Bluetooth submenus share — read four
**shared** pool names (`sub_title` / `sub_meta` / `sub_meta_size` /
`sub_meta_col`), so deciding *which panel is showing* meant a Rust
`match self.mode` table choosing the panel's title. The renderer was picking UI
copy, which is exactly what §0 forbids. That table is now deleted, along with
its dead `"Audio"` arm (no surface ever stamped a third `subhead`).

- **`PropValue` — a prop is deliberately NOT a `SceneValue`.** A prop is written
  by a human in a `props:` block where `ink: acc` must mean "the accent token",
  and no `SceneValue` variant can say that without making the pool's own schema
  ambiguous (`"x"` is a `Text` there, an expression here). So a prop is one of
  three things — a number, an expression, or a color token — and a body reads it
  through whichever channel it asked in.
- **`PropSet` is a hand-written ordered map.** RON cannot deserialize a map into
  `Vec<(K, V)>` at all, so `PropSet` carries its own `Deserialize` (accepting the
  authored `(w: 20.0)` form *and* an explicit pair list) and serializes back as a
  map so the round-trip tests hold. A duplicate name is an **error**, not
  last-one-wins: the effective default must not depend on which line you read.
- **`SceneComponent`'s codec is hand-written too, not `#[serde(untagged)]`.**
  Untagged reports every failure as "data did not match any variant", which
  swallowed all four real messages (a duplicate `w`, an unknown field `itemz`, a
  `(props: …)` with no `items`, and `items` declared twice).
- **Substitution is symbolic, which is the load-bearing detail.** `w:
  "pill_ws_reg_w"` at a use site must stay an expression; folding it to this
  frame's number would freeze the value into the expanded scene and every later
  frame would repaint the first one's width — a bug no single-frame test catches.
  So props go through `expr::subst` (textual) while a model row folds through
  `expr::fold` (numeric). Arithmetic composes: `w: "w - 4"` with `w: "pill_w"`
  becomes `"pill_w - 4"`. A prop that substitutes to a bare number is re-typed to
  `Val::Lit`, because `offset_item` and `Val::constant` only move and read
  literals — a substituted number left as `Expr("4.0")` would be silently dropped
  by the `Comp`'s own `x` / `y` translation.
- **`expr::subst` and `expr::fold` now share ONE name-walker.** Splitting them
  let `fold`'s stand-down rule (one non-finite field abandons the whole rewrite)
  leak into a per-name callback, which returns a half-substituted binding that
  draws a plausible number computed from a `NaN`. The walker takes a `Step` that
  can `Abort`, so the three scope rewrites cannot disagree about what a NAME is —
  if they did, a prop named `w` would be substituted inside `max(w, 8)`'s function
  name by one and not the other.
- **Dead props are reported, because a typo is otherwise invisible.**
  `props: (wid: 99.0)` on a component declaring `w` draws *exactly* as if the
  line were not written. `expand()` proves it by **rendering** the body once per
  prop and comparing against the no-props render — deliberately not by walking
  the body for name references, which would be a second hand-maintained
  inventory of every field of every one of the ~30 item variants, and the day it
  forgets one is the day a LIVE prop reads as dead and the author is told to
  delete it. A test asserts the live `shell.ron` declares none.
- **Two prop spellings carry the non-obvious cases,** and both exist because a
  component cannot hardcode what the shell alone knows: `meta: "{pill_meta}"`
  (braces bind; without them the text is literal, which is the common case and
  what lets `title: "Wi-Fi Networks"` survive) and `ink: "pill_meta_col"` (a bare
  string in a `value()` slot means *the pool color with this name*, deferred so a
  theme switch still lands; `DynColor` caps the name at 24 bytes).
- **A prop read through the wrong channel is left alone**, not substituted
  wrong — `ink: acc` read as a width stays the bare name `ink`, which the pool
  resolves 0. The validator reports the type mismatch; a wrong `0` here would
  draw silently.
- **Fixed on the way through:** `SceneScope::color` had started requiring a prop
  scope, which silently broke model-row colors — caught by the pill parity test
  (a hovered chip drew `fg` where the drawer draws the accent), not by any props
  test.

## 2026-10-01 (rule 4 — models and delegates: the pill's 12 widgets are 8 templates and 12 rows)

Landed **phase 1** of `QUICKSHELL_MODEL.md`. The tree reads **449 passed / 0
failed** (was 435 — 14 new tests: the scope lexer/fold cases, the validator's
model-row diagnostics, the delegate dispatch order, and the live pill template
coverage).

The failure this fixes is the shape of the publisher. A model was
`Vec<String>` — a list of **tags** — so every data-shaped card had to invent a
parallel pool name per field per row (`pill_key_<tag>` + `pill_ink_<tag>`), and
each of those names was a new place a typo could live. The bar's center strip
published `pill_key_ws`, `pill_ink_wifi`, `pill_key_volume`, … and then a
`declare_conditional()` loop to cover the widgets that were sometimes absent.

- **`SceneValue::Model(Vec<ModelRow>)` — a row is a value, not a naming
  convention.** `ModelRow` is a `Vec<(String, SceneValue)>`, the same shape
  `Rows` already used, so a row's `key` / `ink` / `w` are one value the
  delegate binds directly. `SceneValues::model()` is the one reader; `List` (of
  tags) still exists for the repeaters that are genuinely a tag set.
- **`SceneScope` — the overlay from §2, and it folds into the tree.** A `Repeat`
  with a `model` instantiates its template under `SceneScope::row(row, i)`, and
  `SceneItem::scoped` rewrites the subtree: `Val` bindings, `{name}`
  interpolations, and `value("…")` color tokens all read the scope first and the
  flat pool second. An overlay never mutates the pool, so a nested `Repeat` and
  the frame's own values are unaffected.
- **`RepeatSlot` makes the measure and draw passes structurally agree.** A slot
  owns its instantiated items instead of borrowing the template, so
  `repeat_slots()` is the single producer of slot geometry for both passes.
  That deletes a bug class ("a measured width disagrees with a drawn one") rather
  than adding a test against it.
- **`expr::fold` learned dotted names and scopes.** The lexer reads
  `[A-Za-z_][A-Za-z0-9_]*` runs joined by `.`, so `item.w` is ONE name — the
  previous bare-identifier rule would have folded `item` and left `.w` behind as
  a syntax error. A trailing dot is still an error. `fold` takes a scope
  resolver, `deps` reports scope-only names, and a non-finite result is `None`
  rather than a `NaN` that propagates into geometry.
- **Dispatch is the row's `tpl`, then its `tag`, then a `*` wildcard.** This is
  the one deviation from Quickshell and it is deliberate: its `Repeater` has one
  `delegate:` per repeater, but a repeater whose rows are of several *kinds*
  needs the row to choose. `tag` stays the row's identity, `tpl` names the body,
  so widgets that share a body share a template — the pill went from **14
  templates to 8**, and a new widget whose body matches an existing delegate
  needs no template at all. `templates` is reused as the delegate list rather
  than replaced by a separate `delegate:` key, because the tag lookup and the
  delegate lookup are the same find.
- **`UnknownRowField` — the scope is validated, not merely resolved.** A model's
  rows are a CLOSED set, so `item.<field>` is checked against the **union of
  every row's fields**, which is what makes the new expressive power smaller in
  silent failures than the name families it replaced rather than larger. Scope
  names are exempt from pool `UnknownProperty` for the same reason. An empty or
  all-conditional model **stands down**: "no rows today" is not "no rows ever".
- **`SceneItem::scope_names()` — colors and text are bindings too.** The
  evaluator reads scope names through `value("…")` and `{name}` as well as
  `Val::Expr`, so the validator walk has to see them or it would report a tree
  as clean while the evaluator was about to resolve `item.ink` to 0. A test
  asserts the walk covers every field the evaluator reads, so the two cannot
  drift apart.
- **`SceneValue::Color(SceneColor)` with `SceneColor::Raw`** — an ink that
  depends on the row cannot be a pool `ColorToken`, so a color is now a *value*
  and `value("item.ink")` resolves one.
- **The pill, end to end.** `pill_ctr` rows carry `tag`, `tpl`, `key`, `ink`,
  `slot`, `w`, `glyph`. `pill_key_<tag>` / `pill_ink_<tag>` and the
  `declare_conditional` loop are **gone** — a row that exists is a widget that
  is on, so there is no longer a name published only sometimes.
  `pill_item_width() == 0.0` is now a row's *absence* rather than a publishing
  rule. `PillItem::{tag, delegate, glyph}` in `src/shell/mod.rs` are the single
  naming source for all three row fields, and `ALL_PILL_ITEMS` +
  `every_pill_widget_has_a_template_for_its_delegate` walks the widget list
  against the live scene's declared templates — the check that catches a
  `delegate()` naming a body that does not exist, which would otherwise draw
  nothing and say so nowhere.
- **`Surface.hover_key` finally meant what it said.** The bound-`key` path
  resolved `hover_key_var` OR the literal `key` but added `hover_key_add` only
  to the first, so the pill's `ws_long` chips (five keys on a 20 px pitch) were
  off by one against the Rust drawer's hover regions. Both paths now add.

## 2026-10-01 (load-time validation — a scene that names nothing, claims a key twice, or inverts an anchor is a LOAD error)

Closed the last open item of the declarative-core rewrite. The tree reads
**435 passed / 0 failed** (was 417 — the 18 new tests are the validator's).

The failure this fixes is the quiet one. Every stage of the rewrite moved
geometry out of Rust and into RON, and a typo in RON does not fail: serde
ignores an unknown field, an unbound name resolves to `0`, and a duplicate key
resolves to whichever `Hit` the dispatcher happened to register last. A scene
that draws nothing, or a chip whose clicks go to the wrong button, is
indistinguishable from a scene working as intended — until you go looking.

- **`src/scene/validate.rs` — one `LoadIssue` sweep, run ONCE per load.**
  `validate_items()` / `validate_shell()` walk the parsed tree and report
  `BadBinding` (a `Val` that does not compile), `UnknownProperty` (a name
  nobody publishes), `KeyCollision` (two claimants, different actions) and
  `InvertedAnchors` (a box that extends past its parent). `report()` prints
  them as `file: path.to.field: message`, so a diagnostic points at the item
  the way RON is written. The draw paths call
  `SceneCache::get_validated()` / `ShellSceneCache::get_validated()`, so
  validation happens on load and on mtime hot-reload — **never per frame**.
- **A binding is checked by COMPILING it, and names by WALKING the tree.**
  `Val::compile()` already existed and was dead code; it is now the only
  check, which means the validator cannot disagree with the evaluator about
  what a valid binding is. `Expr::deps()` collects the property names an
  expression reads — no second interpreter, no regex over the source. The
  duplicate `Expr::fns` function table went with it: compilation already
  rejects an unknown function and a wrong arity.
- **Duplicate keys are only an error when the claimants DISAGREE.** `Hit`
  regions are resolved from a key→action table, so two boxes on one key means
  one of them is dead. But the same key on the same action is the fat-target
  idiom — `latency.ron`'s visible 30×20 pill plus its invisible 36×26 halo —
  and that must never be reported. The fingerprint is `(expression text,
  key_add)`, so `key: 50, key_add: 1` and `key: 51` are correctly the same
  key, while `50` and `50 + 0` are correctly the same too.
- **Anchor inversion is only proved against a PARENT WE KNOW.** The check
  fires when the child's parent is a container with a literal size and known
  `pad`, which is the case that can be settled at load: `left: -10, right: 0`
  inside a 100 px parent is inverted, full stop. A runtime-sized parent, an
  `auto` container, a bound size or a top-level item has no extent to fail
  against, and is skipped rather than guessed at.
- **A property name is only checked against a COMPLETE pool.** `SceneValues`
  grew `complete` + `conditional` with `mark_complete()` /
  `declare_conditional()` / `is_published()`. The shell's own
  `scene_values()` is marked complete, so it can answer "does anyone publish
  this?" — but a card's LOCAL pool (audiorec builds one) is not, and asking it
  would report every name the shell owns. Conditional names are the other
  half: a name published only sometimes is declared, so the validator can tell
  "collapsed, nothing published" from "typo".
- **A data-model name is checked too, and that found three real conditionals.**
  `Rows`/`Spark`/`Ring`/`Fader`/`Toggle`/`TabRow`/`Grid`/`Tiles`/`Spectrum`/
  `Repeat` name the model they draw and `Dots.active` / `Strip.chips` /
  `BannerRow.cells` name theirs; an unknown one draws an EMPTY item — axes,
  track and all, which looks deliberate rather than broken. Three
  declarations came out of turning it on:
  - `pill_key_<tag>` / `pill_ink_<tag>` for the WHOLE tag set. These were
    declared per visible widget, which never fired for a widget the user has
    switched off: `pill_item_width(WorkspacesLong)` returns 0 while
    `pill_ws_long` is false, so the chip is skipped before its name is
    published. A template may bind any tag — the user's bar order is data.
  - `banner_cells_v`, stamped only for an expanded, un-collapsed strip (and
    from the dashboard draw path, which is why it is not in the shell pool at
    all).
  - `pw_b_series` / `pw_g_series`, which need TWO samples; until then
    `pw_idle`'s flat midline stands in, deliberately.
- **Two sweeps, as tests, so the validator cannot rot.** One reads this
  crate's own source for `: Val,` fields on `SceneItem` and fails unless the
  walker names all **99** — the "a field added to the schema and not swept
  draws wrong in silence" rule, enforced instead of remembered. The other
  parses and resolves one fixture per geometry-bearing variant, so a new
  variant cannot be added without a resolvable binding. And a live audit
  (`every_live_scene_validates_clean_against_the_shells_own_pool`) runs all 72
  committed scene files through the shell's real pool — it is what found the
  three conditionals above, and it will fail on the next typo'd scene before
  the shell does.

Hard rules added today:
- **A name that is published conditionally must be DECLARED, and declared
  where the gate is**, not inside the branch that publishes it. A declaration
  inside `if hist.len() >= 2` is a no-op — it fires exactly when the name is
  already in the map.
- **A duplicate key is only a bug when the actions differ.** Before treating
  a key collision as an error, check whether the fat-target idiom explains it.

Follow-on work is specced in **`QUICKSHELL_MODEL.md`**: the Quickshell model in
RON (models + delegates, component `props`, scene-owned state, change
propagation, `anim:`), in 6 phases with what each deletes. The three
`conditional` declarations above are the kind of bookkeeping that phase 1
deletes outright — a model row that can say "this widget is absent today"
cannot need a conditional name.

## 2026-09-30 (evening — Stage 3: every geometry and key is a `Val`, the `*_var` family is gone, and the wallpaper card stops eating the wheel)

Continued the declarative-core rewrite from the morning's entry. Stage 1
(anchors) and Stage 2 (one implicit-width rule) are in the entry below; this
one is **Stage 3 — bindings** plus a wheel-claim bug fix, and the tree reads
**417 passed / 0 failed**.

- **`Val` — one field type for every extent and every key.**
  `#[serde(untagged)] enum Val { Lit(f32), Expr(String) }` (`src/scene.rs`) with
  `num(vals)` (a binding evaluated, a missing name → 0), `constant()` (the
  literal, or 0 for a binding — what the pre-value layout passes need),
  `binding()` and `uint()`. 99 schema fields are `Val` now, and the whole
  `x_var` / `y_var` / `w_var` / `h_var` / `d_var` / `key_var` / `size_var`
  family is deleted: a field is written `x: 18.0` or `x: "pill_clock_w"` and
  both spellings land in the same slot. `hover_key_var`, `cols_var`,
  `cell_var` and `max_rows_var` **stay**, deliberately — those name a
  *string* property, not a scalar, so they are typed `Option<String>` and are
  not part of the family.
- **The expression evaluator is live, not scaffolding.** `src/scene/expr.rs`
  compiles a binding to a tree and evaluates it against the property store, so
  a scene can write arithmetic (`"pill_ws_reg_w - 2"`) instead of asking Rust
  to publish a second property for the difference.
- **All 72 live scene files are off `*_var`** — `shell.ron` (71 fields) and the
  71 `ui/cards/*.ron` — with a paren-aware rewriter (`/tmp/opencode/mig.py`,
  backups in `/tmp/opencode/ron-bak/`) that bounds each item to its OWN field
  list before touching it. Nothing changed semantically, and that is not an
  assumption: the parity tests compare each scene against the Rust drawer
  command-for-command and region-for-region.
- **Three real bugs came out of it**, all of them silent:
  - **A bound extent was being overwritten by its fill parent.** `want_w` /
    `want_h` read `Val::constant()`, which is 0 for a binding — so the
    `Stack` fill pass could not tell "bound to 20 px" from "fills the parent"
    and overwrote it with the parent's width. The pill's workspace chip
    declared a 20 px click region inside an 18 px chip and got an 18 px one:
    two pixels of dead edge on the pill's most-used button, pinned by
    `the_pill_scene_matches_its_own_drawer`. Fixed by splitting the
    declaration from the measurement — `w_decl`/`h_decl` hand back the `Val`,
    `want_w(vals)`/`want_h(vals)` resolve it, and the new `fills_w`/`fills_h`
    say a fill is a declared literal `0` and never a binding.
    `a_bound_extent_survives_a_fill_parent` pins both halves.
  - **`shell.ron` still carried 71 stale `*_var` fields** after the field
    family was deleted — and serde ignores unknown fields, so every one of
    them was a no-op that drew the *literal* underneath. The polkit password
    caret was drawn at x = 0 instead of riding `polkit_cursor_x`. Found by the
    `shell_ron` parity test, fixed by the rewriter, and the lesson is recorded
    in NEXT_STEPS: a rename in `src/scene.rs` is not a rename until every
    scene file has been swept for it.
  - **85 geometry `Val` fields needed `#[serde(default)]`.** Without it every
    scene that omits `w`/`x`/`h` (most of them) failed to parse — the field
    that used to be a defaulted `f32` became a defaulted-`Val` `f32`-or-string
    and lost its `Default`. `Composer.h` keeps its own `26.0` default.
- **The Wallpaper card claims the wheel only while the grid can move that
  way.** Three fixes, one rule: a card under the pointer takes a notch only if
  the list actually moves; at either end the wheel falls through to the board
  pan underneath instead of dying at the boundary.
  - `wp_card_can_scroll(notches)` is the claim, and it is **direction-aware** —
    the old `wp_card_scrollable()` asked "could this grid EVER scroll", so a
    grid parked at the top swallowed every up notch forever.
  - The claim and the move now share **one geometry chain**:
    `wp_card_scroll_target` reads `wp_card_geometry()` — the same pure function
    the card draws and the scene publishes from — where the old
    `wp_card_scroll_by` measured off `wp_card_cols`/`wp_card_rows` and so
    missed the chain's clip-safe second clamp and could stop the wheel short of
    the last page the card draws.
  - `layout_for` now clears the per-frame wheel rects
    (`clear_frame_wheel_rects`), so a card that is not on the board this frame
    leaves no box behind to keep swallowing notches aimed at whatever is
    actually under the pointer.
  - `the_wallpaper_card_claims_the_wheel_only_while_it_can_move_that_way`
    sweeps five list lengths × four card boxes × two scales and pins: no page
    above the top, no page below the last, one notch is one row of the drawn
    grid, the end stop is the drawn one, and an undrawn card claims nothing.
- **Two more silent-zero bugs, same family.** `content_right` — the implicit
  width rule's "how far does this child's ink reach" — added the declared `x`
  with `constant()`, which is 0 for a binding, so an `auto` container reserved
  the child's WIDTH but not its bound OFFSET: a child bound to an 8 px leading
  inset painted 8 px past the block it was handed, and the next container up
  laid out against a width nothing covered. It resolves the binding now, like
  the draw arm does.
  `an_auto_container_reserves_a_bound_offset_not_just_the_width` pins it (and
  fails if the old `constant()` comes back). Separately, `SceneValues::scalar`
  read only `Ring`, so a `Val` binding naming a **Fader** value resolved to 0
  in silence — a fader is a number, and it is now read as one.
- Verification: **417 passed / 0 failed** (414 + the bound-extent, bound-offset
  and wheel tests).

Still open: Stage 3's load-time validation (the `Val::compile` / `Val::deps`
diagnostics are written and unwired), component extraction (`pill_chip`,
`ws_chip`, `hover_fill`), and the 35 `Ink` cards. See the RESUMED block in
`NEXT_STEPS.md`.

## 2026-09-30 (the scenes stopped spelling out `Some(…)` — implicit RON options, one implicit-width rule, and the anchors migration closed out)

Resumed from the 09-29 PAUSED block: the pill parity test was re-run first
(the mandatory `find src -name '*.rs' -exec touch -d '+1 hour' {} +` recipe —
this box's cargo freshness check reports `Fresh` for a file you just edited, so
a green run can be a stale binary) and the tree read **412 passed**. Then the
declarative-core rewrite moved forward, Stages 1 → 2.

- **`Option` fields are written bare.** RON's `IMPLICIT_SOME` extension is now
  on at the ONE load path every scene file goes through —
  `scene::parse_scene_ron` (`src/scene.rs`), used by both caches
  (`SceneCache::get`, `ShellSceneCache::get`) and by all 140+ test parse sites,
  so what the loader accepts and what a test asserts cannot drift. A scene now
  reads `color: fg`, `visible: "batt_h_pane_0"`, `key_base: 31920`,
  `anchors: (left: -2.0, top: 4.0, right: 2.0, bottom: -4.0)` instead of
  `color: Some(fg)`, `visible: Some("…")`, `anchors: Some((…))`. **The `Some(…)`
  spelling still parses**, so a file can be migrated a field at a time and a
  half-migrated tree still renders; `optional_fields_take_both_spellings` pins
  that the two forms load to the *same* scene.
- **All 72 live scene files migrated** — `shell.ron` (220 wrappers) and the 71
  `ui/cards/*.ron` (814) — with a string- and comment-aware rewrite, so prose
  in a `//` comment is untouched and no `Some(` inside a string literal was
  touched (there were none). Backups in `/tmp/opencode/ron-bak/`. Nothing
  changed semantically, which is not an assumption: the parity tests compare
  the scene against the Rust drawer command-for-command and region-for-region,
  and `every_live_card_scene_parses` walks the card directory.
- **Stage 2 — the implicit-width rule now has ONE implementation.** Five sites
  open-coded "the widest visible child's content extent" (the
  `Column` / `Stack` / `Repeat` arms of `intrinsic_w`, the `Column` and
  `Stack` draw arms' `inner_w`, and the `Repeat` per-slot measure); they now
  share `SceneItem::widest_content`, and padding/gaps stay the caller's
  business. One real behavior fix rode along: the `Column` draw arm measured a
  gated-off child while the `Stack` arm and `intrinsic_w` already skipped it, so
  a `Column { auto: true }` with a hidden wide child reserved a width nothing
  painted. `an_auto_container_measures_and_draws_the_widest_visible_child`
  pins both halves — the width a parent *reserves* and the width a `w: 0` child
  is actually *given* must be the same number, with the gate on and off.
- **The anchors migration is DONE, and `region_d*` survives on purpose.** The
  12 collapsed-pill chips moved to `anchors` yesterday; the 13 remaining card
  sites (`accent.ron` ×1, `audiodevice.ron` ×1, `clipimg.ron` ×1, `system.ron`
  ×5) are NOT expressible as anchors, and forcing them would be a regression.
  `anchors` is an *absolute* rect — four insets against the parent's inner rect
  — and it REPLACES the node's `x`/`y`/`w`/`h`, paint included. `region_d*` is
  a *delta* halo on top of a painted box that stays put, so it is
  parent-size-independent by construction: the system card's 26×22 icon buttons
  with a 32×28 region is one line, and in anchors it would be four numbers that
  each need the live card width, wrapped in a `Stack` to separate the paint
  from the click box. Same for the ws tail's dynamic-width region
  (`pill_ws_reg_w` is 26 or 20 px depending on how long the workspace number's
  string is — a runtime fact, so no static inset can spell it; it stays on
  `w_var` until Stage 3 bindings). `region_d*` is now documented as a
  first-class delta primitive, not a pending deletion.
- Verification: **414 passed / 0 failed** (412 + the two new tests).

Still open: Stage 3 (a typed property store + expression evaluator to delete
the `*_var` family, then load-time validation, then component extraction —
`pill_chip`, `ws_chip`, `hover_fill`), and the 35 `Ink` cards. See the PAUSED
block in `NEXT_STEPS.md`.

## 2026-09-29 (night session — declarative-core rewrite, Stage 1: `anchors`, and `Hit.no_region` retired)

The user pushed back on the whole approach: *"why is the declarative code not
like Quickshell? how can we make quickshell-like declarative?"* Diagnosis: what
we had was a serialization of imperative draw calls (every `x/y/w/h` a literal,
every click halo a second vocabulary in `region_d*`), not a layout language.
Agreed scope — **anchors + implicit sizing + bindings** — and Stage 1 (anchors)
is now landed and green.

- **`Anchors { left, top, right, bottom: f32 }`** (`src/scene.rs`) + an additive
  `anchors: Option<Anchors>` on `SceneItem::Hit`. It is a single geometry space:
  when present it **replaces** the node's `x/y/w/h`, declared as **insets from
  the parent's inner rect** — the `(x, y, card_w, card_h)` that `draw_piece`
  already receives, so no signature changes. `left`/`top` measure from the
  inner left/top edge, `right`/`bottom` from the inner right/bottom edge
  (negative insets), `max(0.0)` clamped.
- The **12 collapsed-pill chip region `Hit`s** in the live `shell.ron` moved off
  `region_dx: 2.0, region_dy: -4.0, region_dw: 0.0, region_dh: 0.0` and onto
  `anchors: Some((left: -2.0, top: 4.0, right: 2.0, bottom: -4.0))` — the same
  drawer's `region(cx − 2.0, 4.0, iw + 4.0, h − 8.0, key)` halo, now expressed
  relative to the painted chip instead of as a parallel coordinate system.
- **`Hit.no_region` is deleted.** It existed only because a hover fill whose box
  isn't its hit box had to be a `Hit` (which always registers a region), needing
  a flag to opt out. The fill is now what it should have been — a `Surface` —
  given two new fields: **`hover_key_var: Option<String>`** (bind the hover-gate
  key to a published scalar, for the per-slot bell / brightness / settings chips
  whose keys are runtime-derived) and **`hover_key_add: u32`** (the `Surface`
  twin of `Hit.key_add`, for the long-workspace tail's `long_key + 5`). A bound
  `hover_key_var` outranks the literal `hover_key`. The pill's four fills
  (bell, brightness, settings, ws_long tail) are now `Surface`s, and the
  bell/brightness `surface_hover: Some(...)` lift hacks are gone with them.
- Verification: `the_pill_scene_matches_its_own_drawer` green (parity test —
  command stream + hit regions compared item-for-item against the Rust drawer
  across the hover sweep), full suite **412 passed / 0 failed**.

Not yet done: `region_d*` stays in the schema because the cards still use it
(`ui/cards/audiodevice.ron`, `accent.ron`, `clipimg.ron`, `system.ron`); the
dynamic-width ws tail region still needs `w_var` until Stage 3 bindings land.
`NEXT_STEPS.md` carries the PAUSED block with the full resume plan.

## 2026-09-29 (the second whole-shell panels went declarative — the workspace grid and the auth dialog, with a caret that rides a measured run)

- **`workspace_switcher` and `polkit_auth` are zero-`Ink` surfaces** — the
  same shape as the first batch: each `layout_*` opens with
  `draw_shell_surface(...)` and an early `return`, so the Rust drawer stays
  the geometry of record and the no-scene fallback, and no key moved.
  - **`workspace_switcher`** — the dismiss `Hit` (key 1), `Workspaces` at
    16/14, and a right-anchored `{ws_count}` 20 px in from the right edge. The
    grid is **two `Rows` lists**, because `Rows` has no `x` of its own: the
    column offset rides `pad` (14 / 302, 274 wide, 56 tall on a 70 px stride
    from y 44), which is the drawer's 2-up flow (`(w - 48) / 2` with a 14 px
    gutter and a 56 px tile) written as two column lists. Each row carries its
    OWN `key` so the global `10 + i` sequence survives the split, and the
    publisher fills each list in the drawer's column-major order
    (`i = r · cols + c`). The active card's 4 × 40 accent rail is a `bar` cell
    (`height: 40, top: 8, radius: 2`) whose value is 1.0 on the active row and
    0 elsewhere on a `clear` track, so the inactive rails are free. The
    number (20 px) and the `Desktop` caption (11 px) ride the per-workspace
    `ws_wash_i` / `ws_num_i` bindings: this drawer resolves its ladder
    active-FIRST, while `Rows` prefers a row's `surface` over
    `hover_surface`, so the wash has to be published per row.
  - **`polkit_auth`** — the shield, both centered lines, the user, and the
    field: a `Stack` so the `Hit` (key 92) rides the exact box the surface
    paints, which is how the drawer registers it. The caret is a
    `Surface.x_var` bound to a published `polkit_cursor_x` — the drawer's
    measured run `36 + 13 · 0.62 · len` — so the blink rides the real text
    width rather than a guessed inset. The error line is gated on
    `polkit_err_on`; the Cancel/Authenticate pair is a `bottom: true` `Row`
    (`y: 24, h: 36`: `bottom` anchors the row's OWN height, so `h` has to be
    declared or the row lands at `y`) of two 120 × 36 `Stack`s with a 16 px
    gutter, `halign: center`. The publisher clips message and action with the
    drawer's own `(w - 48) / (12 · 0.62)` at the LIVE dialog width
    (`Mode::PolkitAuth.size(&self.cfg)`) so the two cannot drift, and
    publishes every moving fill: `polkit_pw_bg` / `polkit_pw_ink` (the empty
    field is fg3, the drawer's placeholder dim) / `polkit_cursor_on` /
    `polkit_cancel_bg` / `polkit_auth_bg` (a 15 % white blend on hover) plus
    `polkit_auth_label`, which reads Verifying… while the check is in flight.
  - `SceneItem::Row`'s `bottom` anchor now has its own test
    (`row_bottom_anchor_uses_the_row_height`): the two 120 × 36 buttons land
    at 82/218, y 260 on the 420 × 320 dialog.
  - `POLKIT_KEY_AUTH` / `_CANCEL` / `_PW` are `pub(crate)` now, so the
    publisher names them rather than repeating 90 / 91 / 92 as literals.
  - Guarded by the extended `shell_ron_first_batch_matches_its_rust_drawers`
    (it now measures all six surfaces — including the card grid, its rail,
    the captions, the field, the button pair, the caret and all three polkit
    keys). **404/404.**

## 2026-09-29 (the first whole-shell panels went declarative — the OSD, the Power tiles and both radio submenus, and `bars` learned to float)

- **`osd`, `power`, `wifi_menu` and `bt_menu` are zero-`Ink` surfaces** — the
  first `shell.ron` surfaces that own a panel COMPLETELY, with no Rust body
  behind them. Each `layout_*` opens with `let vals = self.scene_values();` and
  `if self.draw_shell_surface(...) { return; }`, so the Rust drawer that follows
  is the geometry of record AND the no-scene fallback; keys and regions are
  unchanged, so no input routing moved.
  - **`osd`** — the whole-surface dismiss `Hit` (key 1) plus the two strings,
    both gated on a new `osd_on` toggle, because the drawer returns before
    drawing when no OSD is live. The strings are `center_y` with `y: -11` /
    `-7`: the drawer's own `(h - fs)/2` centring rewritten as offsets from the
    panel's mid line (which is why they are negative).
  - **`power`** — title + the five 92 px tiles as one `Row(halign: center,
    valign: middle)` of `Stack`s, so the block centers itself the drawer's way
    (`(w - 516)/2`, `(h - 92)/2`). Each stack is the background wash, the
    hold-to-confirm liquid (`Bar(vertical: true, max: 1.0)` gated on the
    PANEL's own 92 px · fill >= 1.5 px sliver), the 24 px glyph at the drawer's
    `(tile_w - fs)/2` = 34 inset, the label centered on the tile at `y + 60`,
    and the `Hit` (keys 1..=5).
    The panel publishes its OWN ladder — `ppower_bg_N`, `ppower_hold_N`,
    `ppower_hold_on_N`, `ppower_ink_N` — because the power CARDS red Logout
    and Restart as well and key off 12000+; reusing `power_*` would have
    painted three red tiles. The liquid reuses the global `power_hold_fill` /
    `clear` colors, exactly as `powerv.ron` does. `Shell::power_panel_items()`
    is now the single source of the five (glyph, label, danger) tuples for the
    drawer and the publisher.
  - **`wifi_menu` / `bt_menu`** — a new shared **`subhead`** component (the
    back `Hit` at key 1, the back glyph, `{sub_title}` and a right-anchored
    meta whose SIZE rides a `sub_meta_size` scalar and whose INK rides a
    `sub_meta_col` color, so one stamp covers the radio menus' "On|Off" pair
    and Audio's quieter per-app meta) plus one `Rows` list per menu on the
    drawers' own 56 / 36 / 42 / 16 / 12 geometry, keyed 10+ with the empty
    state (`wifi_empty_on` / `bt_empty_on`) beside it.
  - **The signal staircase** is one `bars` cell: `SegmentsSpec.center` makes
    each bar straddle the row's mid rail (the drawer drew `y + 18 - bh/2`)
    instead of standing on a baseline, and the cell's `right` + a new
    `edge: 46.0` block anchor puts the whole 26 px staircase where the drawer
    measured it from the PANEL's right edge (`w - 72` start, not the cell's
    left). Two new engine lines, both generic: a centered `bars` staircase and
    a right-anchored one.
  - **Per-row washes**, not `hover_surface`: both submenu drawers resolve
    hover FIRST and then the connected accent tint, while `Rows` prefers a
    published `row.surface` over `hover_surface` — so a connected row would
    have lost its tint under the pointer. `Shell::publish_row_wash` publishes
    the ladder per row under a `DynColor` binding instead. The Wi-Fi label is
    published as the composited "{lock} {ssid}" string (the padlock is a
    PREFIX; the `suffix` cell kind only hangs a trailing glyph), and the
    Bluetooth state column is a per-row `col_colors` binding.
  - (`shell_ron_first_batch_matches_its_rust_drawers` measures the live
    surfaces against the drawer constants — row band, stride, label inset,
    check anchor, signal block, tile block, both OSD anchors — and
    `shell_ron_live_file_parses_and_has_surfaces` now requires all four
    surfaces, zero `Ink` each, plus the `subhead` component;
    `bars_cell_centers_its_staircase_and_can_anchor_the_block_right` and
    `bars_cell_without_center_still_stands_on_its_base` pin the new engine
    rules in isolation.) 403/403 tests.

## 2026-09-27i (the focus timer went declarative — a `Hit` learned to straddle the center line, to measure a string, and to move only its label)

- **`pomodoro.ron` is a zero-`Ink` scene.** The mm:ss figure (ui-light 26,
  centered, `y+30`), the phase word, the progress bar, the `[-] 25m [+]`
  stepper trio and the Start/Pause + Reset pair are all declared; the Rust
  drawer survives as the no-scene fallback and is the geometry of record.
  Everything per-frame rides a bound value — the figure, the phase ink
  (focus `fg` / break `OK`, dimmed a quarter while idle), the bar's fill and
  fraction, the phase word, the readout, the toggle label and the three
  button fills. The button fills must be bound: the drawer's
  `draw_edit_lbl_btn` *mixes* the resting fill and tints the accent chip on
  hover (`mix(hover, acc, 0.25)`), which no plain token expresses.
- **`Hit` grew the three rules a centered control pair needs:**
  - **`center_x`** — put the box's LEFT edge on the parent box's center line,
    `x` base px right of it (the `Surface` / `Text` rule). The drawer's pair
    is `w/2 − 58` and `w/2 + 6` — symmetric about the middle with a 6 px gap
    straddling it, which a centered `Row` block (110 px) cannot express
    (that would put it 3 px in on each side).
  - **`x_var`** — a bound scalar replaces the declared `x` (the `Text`
    `y_var` / `size_var` rule). The stepper chips flank a *runtime* string,
    so their center-relative x is measured, not assumed: the shell publishes
    the drawer's own `chars · 0.62 · fs(8.5) + 2` estimate (now a shared
    `Shell::pomo_readout_w`, so the fallback and the scene cannot drift) and
    the chips sit `±rw/2` off the middle. Without it a 1-digit duration
    ("5m") would sit 2.6 px out of place on every frame.
  - **`label_dy`** — nudge the glyph + label *inside* a fixed box. `dy`
    folds the nudge into the box itself (fill, region and label together),
    which is right for a box-anchored control but cannot express the
    drawer's `ty = y + (bh − fs) / 2` — where the chip's fill and its
    centered label need independent tops. `dy`'s doc said "label baseline"
    while the code moved the box; the doc now says what the code does, and
    `label_dy` is the label-only rule.
  - (`pomodoro_scene_is_zero_ink`,
  `the_pomodoro_timer_lands_on_the_drawer_geometry`,
  `the_pomodoro_timer_follows_its_phase_and_running_state`,
  `the_pomodoro_steppers_track_the_readout_width`.)
- **A differential parity test, because hand-computed geometry tests can agree
  with each other and both be wrong.**
  `the_pomodoro_scene_matches_its_own_drawer` draws the Rust drawer and the
  declarative scene from the SAME `Shell` — real state, real palette, real
  `scene_values()` — and compares the two command lists command for command,
  at two card sizes. It immediately caught a `[+]` chip sitting **6 px** too
  far right: the drawer computes it as `cx0 + bw + 6 + rw`, which collapses
  back to `w/2 + rw/2`, while the scene had published `rw/2 + 6` and kept the
  gap that had already been folded into `cx0`. The four hand-computed tests
  all passed with the wrong number, because they were derived from the same
  misreading. Every future zero-`Ink` conversion should get one of these —
  it needs no new harness, just a `Shell` and a comparable command
  fingerprint.
- 380/380 tests.

## 2026-09-27h (the countdown composer and the workspace tiles went declarative — `Grid` grew a tile mode, and a live half-a-cell label bug fell out)

- **`countdown.ron` is a zero-`Ink` scene.** `countdown_rows` had already
  landed as a declarative `Rows`; the composer beside it is now a `Surface` at
  `(12, 26, w − 24, 22)` with radius 6, a buffer `Text` at `(20, 31)` in
  8.5 px, and an invisible 24 px-tall `Hit` on key `31500` so the whole
  composer is clickable, not just the 8 px of text. The header chrome now
  comes from the shared `card_head` (the same anchor notes/expenses use), so
  the fallback header followed: title `y 10 → 9`, meta `y 12 → 11`, meta size
  `9 → 8.5`, meta ink `fg → fg3`. The `countdown_rect` the drawer tracks is
  write-only, so it needed no scene plumbing.
- **`workspaces.ron` is a zero-`Ink` scene**, and the tile grid is why `Grid`
  grew a tile mode:
  - **`cols_break`** — the drawer's `(w − 24) / 34` column ladder, written out
    as breakpoints (58 px → 1 column, up to 364 px → 10), last matching
    breakpoint wins, then clamped to the cell count. Same contract as the
    deck's `cols_break`: a ladder that only ever RAISES the base `cols`.
  - **`cell_max_w` / `cell_max_h` + `center`** — cap a cell, then center the
    capped block in the body, so a 3-workspace card renders three 48 px tiles
    centered rather than stretched to 1/3 of the card.
  - **`cell_w_break`** — the per-tile label ladder (9 px base, 11 px once a
    tile is 30 wide or more), declared in the same raise-only shape as
    `cols_break`.
  - **`GridCell::marker`** (an optional dot, default 3 px, drawn 6 px above
    the cell's bottom edge) and **`GridCell::vcenter`** (center the label on
    the cell's own midpoint, biased up by half its font height plus 1 px) —
    the drawer's active-tile pip and its optically centered number.
  - `ws_fill_act` publishes the drawer's straight 16 %-alpha accent wash as a
    `Raw` color. It is per-frame, not a palette constant, so a hovered active
    cell repeats it in `surface_hover`: the drawer tests `active` BEFORE
    `hover`, so the active tile keeps its wash under the cursor while a
    resting one lifts to `hover_hl`.
  - **A live engine bug: every `Grid` label was half a cell off.** `draw_text`
    takes a BOX and centers inside it (`x + w/2`), and the cell passed the
    already-centered anchor — so labels landed on the cell's **right edge**.
    The calendar's day numbers (and the app-tile and thermal labels) had been
    drawing a half-cell right of their cells; the region box was always right,
    so the tests that pinned hit geometry never saw it. Both the label and
    `sub` lines now pass the cell's left edge
    (`grid_labels_center_on_their_own_cell`).
  - (`workspaces_scene_is_zero_ink`,
  `the_workspaces_tiles_land_on_the_drawer_geometry`,
  `the_workspace_column_ladder_follows_the_card_width`,
  `the_active_workspace_keeps_its_wash_under_the_cursor`.)
- 375/375 tests.

## 2026-09-27g (the latency plot went declarative — a `Divider` grew a midpoint anchor, a gate and a right inset)

- **`latency.ron` is a zero-`Ink` scene.** The Rust body survives as the
  no-scene fallback and is the geometry of record. The ping series is a
  `Spark(kind: line, max: 1.0, thickness: 2.0)` — byte-for-byte `ui::pulse`
  once the shell publishes it pre-normalized against the drawer's own 120 ms
  peak floor — and the hero ms number, its `ms` unit, the status word and the
  30x20 re-probe pill (with its 3 px click halo as a second, invisible `Hit`
  on the same key) are declarative `Text` / `Hit` items. The pill's hover fill
  and ink are `surface_hover` / `color_hover`, so no pointer state is
  published. The hero and the unit land on `cy_pct: 0.5` (`h/2 + 17` and
  `h/2 + 3`) and the status word on `bottom: true, y: 23` (`h − 23`).
- **`Divider` gained three things, all the `Surface` / `Text` rules it was
  missing:** a **negative** `w` (stop that many base px short of the card's
  right edge, via `span_px`) so a plot can reserve a column, `cy_pct` (the
  same proportional anchor `Text` has) so a zero-line can ride the card's
  midpoint, and a `visible` live gate. The latency idle trace is one
  `Divider` gated on `lat_idle`.
  (`latency_scene_is_zero_ink`,
  `the_latency_plot_lands_on_the_drawer_geometry`,
  `the_latency_idle_trace_holds_the_plot_midpoint`,
  `the_latency_error_state_reads_err_in_danger`.)
- 368/368 tests.

## 2026-09-27f (the world-clock list and the brand mark went declarative — a `Text` can now bind its own size and top, and the last `Ink` no longer eats the tail)

- **A live engine bug: items authored AFTER the final `Ink` were never drawn.**
  The pre-pass buffered everything past the first `Ink` into `ink.pre`, but
  the LAST `Ink`'s buffer had no following `Ink` to hand it to, so it was
  dropped. A card that declared `Ink` then its own `Rows` (notes, countdown)
  therefore rendered **no list at all** while its Rust drawer stood down for
  it. The pre-pass now redraws everything after the last `Ink` in scene
  order (`scene::tests::items_after_the_last_ink_keep_scene_order`).
- **The world-clock city list is a declarative `Rows` block.** `worldclock.ron`
  keeps its `Header` and its resident `Ink` (the add-city row, the search
  overlay and the menu), and the cities became a `Rows` list beside it: the
  day/night glyph, the 14-char city, the mono `HH:MM` anchored at `edge: 46`,
  the offset chip, and the per-row delete key (`del_base: 31_000`, the
  drawer's own `31_000 + i`). The row cap is `bottom: 34.0` — the last row may
  not cross the add-row button.
  (`worldclock_declares_the_city_rows_beside_the_resident_ink`,
  `the_worldclock_rows_land_on_the_drawer_geometry`,
  `the_worldclock_list_leaves_the_last_slot_for_the_add_row`.)
- **`branding.ron` is a zero-`Ink` scene**, and it needed two new `Text`
  bindings: `size_var` (a bound font size, BASE px — the mark is aspect-fit,
  so the size depends on the card box) and `y_var` (a bound base `y`, which
  composes with `center_y` / `cy_pct` / `bottom` like the literal it replaces).
  `scene_values` computes the fit — `min(avail_w / (chars·0.62), avail_h) ·
  0.95` (≥ 10), top `24 + (avail_h − size) / 2` — in base px, and re-reads the
  glyph from `$states2/d` there, since the Rust drawer that used to re-read it
  no longer runs.
  (`branding_scene_is_zero_ink`, `a_text_binds_its_size_and_top_to_values`,
  `the_brand_mark_is_contain_fitted_to_the_card`,
  `the_brand_card_falls_back_to_its_empty_caption`.)
- **`wifi_rect` is published for a fully declarative card.** The row window
  (`wifi_card_visible`, the wheel clamp) lived in the Rust drawer, which a
  zero-`Ink` scene skips — so the live Wi-Fi card resolved a zero rect and
  no rows. `draw_card_scene` now tracks it the way it already tracks
  `todo_card_rect` / `sliders_rect`.
- 364/364 tests.

## 2026-09-27e (the alarms list, the quote body and the Wi-Fi list went declarative — the staircase row got two new cell kinds)

- **`alarms.ron` is a zero-`Ink` scene.** The Rust body survives as the
  no-scene fallback and is the geometry of record: a `Header` + the drawer's
  own right-anchored `HH:MM · ` meta, the composer as a declarative `Hit`
  (`ALARM_KEY_BASE`), and one `Rows` row per alarm with the hover ✕ delete
  (`del_base`). New values: `alarm_meta`, `alarm_input`, `alarm_fill`,
  `alarm_ink`, and `alarm_rows` — capped at the 16 entries the ✕ keys can
  address, the same cap the drawer's own delete loop used.
  (`scene::tests::alarms_scene_declares_rows_without_ink`,
  `alarms_rows_carry_their_delete_key_and_hover_lift`.)
- **`quote.ron` is a zero-`Ink` scene.** The wrapped quote is a `TextWrap`
  (the old hybrid test becomes
  `quote_scene_declares_textwrap_without_ink`), the two centered 88 px pills
  (refresh always, save behind `quote_saved`) are `Text` items over
  `SceneCol.pill`, and the loading / no-quote captions ride `quote_state`.
  The one-second `saved` flash still decrements in the Rust body — it is
  frame state, not layout.
- **`wifi.ron` is a zero-`Ink` scene**, and it needed two new `SceneCol` cell
  kinds: `bars: Some(SegmentsSpec(...))` climbs `count` caps off one shared
  baseline (the signal staircase, `floor(signal/25)` of them lit, quiet
  `track` for the rest) and `suffix: Some(SuffixSpec(glyph_col, gap, ...))`
  hangs a glyph `gap` px past a text cell's own measured end, read from
  another column of the same row (the padlock 6 px past the SSID; an open
  network just leaves it empty). The run is **measured, never clamped by
  `col.w`** — `w` is a box, and the drawers measure the glyph past the name.
  A new `SceneCol.hide` makes a carrier column (the padlock source) draw
  nothing of its own; `dot` and `bars` now draw from an empty cell too, since
  they are geometric cells that need no text.
- **The row wash chain moved into the published values.** The drawer's
  `connected → acc_tint, else hovered → hover` cannot be a `hover_surface`
  (its hover variant is one step lighter, and it would outrank a connected
  row's tint), so `scene_values` publishes the resolved `surface` per row —
  a hit key is a WINDOW index (`input.rs` resolves `top + j`), so the hovered
  data row is the one at `top + j`.
- **The header glyph is inside `!scene_owns_header` again** (wifi, quote) — a
  scene `Header` draws its own glyph, so leaving the Rust glyph outside the
  guard drew it twice.
- 356/356 tests.

## 2026-09-27d (the docker list went declarative — two new cell kinds for it)

- **`docker.ron` is a zero-`Ink` scene.** The Rust body survives as the
  no-scene fallback and is the geometry of record. One `Rows` row per
  container — a status dot, the name, and the image label — with the drawer's
  `floor((h − 36) / 26)` cap expressed as `bottom: 4.0` (a row may not cross
  `card_h − 4`) and the single centered `No containers` caption behind
  `docker_empty`.
- **Two new `SceneCol` cell kinds.** `dot: Some(DotSpec(size, radius, top))`
  paints a small filled capsule at the cell's `x` — no text needed, the ink
  comes from the row's `col_colors`, so one published status drives the whole
  dot (docker's Up = OK green vs the danger red). `max_chars: Some(20)` is a
  hard CHARACTER cap, next to `truncate`'s width reserve — the Rust drawers'
  `chars().take(20)` image labels
  (`scene::tests::a_dot_cell_paints_its_own_ink_and_a_char_cap_truncates`).
- 344/344 tests.

## 2026-09-27c (the mic meter went declarative — and `center_x` stopped being a left edge)

- **`micmeter.ron` is a zero-`Ink` scene.** The Rust body survives as the
  no-scene fallback and is the geometry of record. The 8 px level bar, its
  status caption and the 22 px mute button are declarative; the shell publishes
  the four state-driven colors the drawer computed inline (`mic_lvl`,
  `mic_bar_ink`, `mic_status` / `mic_status_ink`, `mic_btn_fill` / `mic_btn_ink`
  and the `mic_btn_label` glyph+label string). The button's hit keeps the
  drawer's own 3 px click halo — `Hit { x: 9, w: -9, bottom: true, h: 28 }` —
  and its `MIC_MUTE_KEY` region, so `input.rs`'s mute path is unchanged.
- **`center_x` on a `Text` centers the STRING, not its left edge.** The arm
  resolved `center_x` to `sx` and then handed `draw_text` the item's fill width
  with `center: true`, so `x` came out at `card_w / 2 + w / 2` — and with the
  `w: 0.0` every empty-state caption declares (no `center: true` at all), the
  string was left-anchored ON the center line instead. Roughly 25 committed
  captions were a half card-width right of center, and the gauges card's
  single-column hero sat 45 px right of its own ring. `center_x` now emits a
  zero width + `center: true`, which is the drawers' `ui::text_c*` idiom
  (`scene::tests::center_x_centers_the_string_not_its_left_edge`,
  `gauges_single_column_centers_its_hero_on_the_ring`). Every already-converted
  card's empty-state caption inherits that fix, so `disk`'s geometry test now
  pins the x/center flags too rather than only the string
  (`disk_rows_land_on_the_drawer_geometry`).
- 340/340 tests.

## 2026-09-27d (the audio-device card's muted row was the last red test in the shell)

- **The suite is green: 385/385.** `the_audiodevice_scene_matches_its_own_drawer`
  had been the one standing failure, and it was a real divergence rather than a
  test bug — on a row that is BOTH muted AND hovered, the scene and the drawer
  disagreed about the vol %'s ink.
- **The drawer tests `muted` FIRST, and a `SceneCol.hover` token cannot say
  that.** The drawer's rule is `if *muted { fg3 } else if hov { hover_fg } else
  { fg3 }`, so a muted row's pct stays dim even with the cursor sitting on it.
  The engine resolves a hovered row's cell color as `col.hover.or(base)` — the
  col's hover token outranks ANY published value — so `audiodevice.ron`'s
  `color: Some(fg3), hover: Some(hover_fg)` tinted muted rows the drawer left
  alone. Its comment claimed parity with the drawer's `hover_key != mkey` test:
  that guard is right for the UNMUTED case, and the muted case was never
  reachable through it. The pct ink is now published per row as `col_colors[1]`
  (`muted → Fg3`; else `hover_key == AUDIO_DEV_BASE + 2·i → HoverFg`; else
  `Fg3`) and the col declares no `hover`. The row-key test alone covers the mute
  box, because the aux key is `key + 1` — a pointer on the mute glyph is not on
  the row. Same shape as pomodoro's per-frame button fills: when a token can't
  express the drawer's precedence, `scene_values()` publishes the resolved
  value and the `.ron` stays static
  (`scene::tests::the_audiodevice_rows_follow_the_device_state` now pins all
  three branches — unhovered, hovered-unmuted, hovered-muted).
- 385/385 tests.

## 2026-09-27j (the system card went declarative — a hit halo, a width binding, and per-frame inks)

- **`system.ron` is a zero-`Ink` scene** and the first one with no `Header`:
  its chrome IS its body, a hand-positioned two-column identity block on the
  left and a control stack on the right. Everything the drawer trimmed or
  measured inline is published already done — the 12-char username, the
  hostname capped at `floor(card_w / 8)` chars (which is why the card box is
  now recorded in `sys_card_rect`; the drawer that read it stands down), and
  the date chip, whose width is `chars · 12 · 0.62 + 18`.
- **`Hit` grew `region_dx` / `region_dy`** — inflate the HIT box without
  touching the painted one, which is every icon button in this card: a 26×22
  pill inside a 32×28 well, and the date chip's own 4/3. The region used to
  be welded to the box, so a drawer with a click halo around its pill had no
  way to say so.
- **`Hit` grew `w_var`** — bind the base width to a published scalar, the
  `Text`/`x_var` mirror, so a pill can track a measured string. It also makes
  the battery %'s `x_var` a *right-edge* distance (`s(16)` gutter + the
  measured glyph + 4 px), which is width-independent like the drawer's own
  arithmetic — the first binding in the engine that needs no card box at all.
- **Two state-driven inks ride per-frame `value(...)` colors** rather than new
  tokens: the hidden-menu fill (`hover_hl` while hovered, a 20%-alpha accent
  wash while open — mutually exclusive, so it is ONE gated `Surface` with a
  published ink, not a hover fill plus an open fill), the ⋮ dots, the date
  chip's wash, and the battery's low red. The bell/DND and dark-mode glyphs
  are published as text so the `visible` swap is data, not a branch.
- `the_system_scene_matches_its_own_drawer` — 120 cases: five states (healthy
  battery, low battery, no battery at all so the block stands down, DND with
  the bell glyph swapped and the badge gone, the menu open) × four card boxes
  × the pointer in and out of all six click targets, with every region
  compared as a hit box. Plus `system_scene_is_zero_ink`.
- 393/393 tests.

## 2026-09-27n (the world clock went declarative — a pinned clock, and a search list that is one pure function)

- **`worldclock.ron` is a zero-`Ink` scene.** The pinned-city list is a `Rows`
  (glyph · city · right-anchored mono `HH:MM` · offset chip), the `+ add city`
  row is a bound `Hit` + two `Text`s, the search plate is a `Surface` with the
  buffer and the `esc` hint, the matches are a second `Rows` whose rows carry
  their OWN keys, and the empty result is one gated `Text`.
- **The match list is ONE pure function**, `Shell::worldclock_matches()`, and
  three callers share it: the scene value, the input key resolver (a keypress
  must resolve to the same zone the click handler will) and the Rust fallback
  painter. It used to be a `Vec` the body rebuilt per event, which is how the
  key table and the drawn list could disagree.
- **The clock is sampled once per frame** into `worldclock_epoch`, and the
  layout pass records `worldclock_rect` beside it. The list's own geometry (row
  cap, search origin) needs both, and `scene_values` is `&self` — it may read
  the frame's sample, never take a new one.
- **The match rows deliberately carry no `key_base`.** `Rows.key_base` is an
  `Option`; the match keys are `PICK_BASE + i` and already live on each row, so
  a base here would be a second, competing source for them.
- **A match row has to FIT, not merely start on screen.** The old cap asked
  whether the NEXT row's top was still inside the card, so the last match drew
  24 px past the card's bottom edge; the cap is now the `Rows` item's own rule
  (`floor((h + 0.5 - ry0) / row_h)`, six max) and the fallback follows it.
- **`Text.dy` — a baseline nudge from the resolved `y` (base px).** The row
  drawers' `ry + 1` / `ry + 2` idiom puts a cell's ink a pixel or two below the
  row box that owns it, and one published `y` cannot serve two baselines.
- **The `Rows` ✕ grew three region knobs and a colour**: `del_rx` (gap from the
  card's right edge to the CLICK BOX), `del_ry` (how far the box starts above
  the row), `del_rh` (`0` → the row's own height) and `del_color` (unbound
  keeps the affordance default — `danger` once the pointer is on the ✕, `fg`
  while it is merely revealed). The glyph and its click box are measured
  differently by every drawer — the world clock wraps one 2 px wider and 2 px
  higher than the glyph it holds — and one knob cannot be both. The defaults
  reproduce the old hardcoded box exactly, so the five other `del_base` lists
  (alarms, appshortcut, countdown, snippets, todo) are untouched.
- **A `Rows` list that omits `pitch` fanned out above `grid_scale = 1.0`.** The
  engine shadowed the base `row_h` with the SCALED one and then scaled the
  default stride from that, squaring the scale — so at the configured 1.75 the
  rows of countdown, snippets, to-do and world clock sat at
  `row_h x 1.75` apart. The stride now reads the base height.
- **The world clock's hover plate is 2 base px shorter than its row**, so its
  hairline gap scales with the card; the drawer subtracted a device-px `2.0`.
- `the_worldclock_scene_matches_its_own_drawer` — 480 cases: four city counts
  (none, one, three, eight) x four buffers (idle, a 1-char needle that is not a
  search yet, a needle that matches, one that does not) x three card boxes x
  five pointer states (away, over `+`, over a match, on a row's ✕, and on a
  DIFFERENT row's ✕ — the same reveal in another ink) x two scales — comparing
  the command list AND the sorted hit-region list.
- `worldclock_scene_is_zero_ink` pins the bindings a visual check cannot see:
  the absolute key bases, the row origin/height, the ✕'s geometry and the gate
  that hides the list while the search owns the card.
- The city rows are pinned to a fixed epoch in tests, and the zones use ABSOLUTE
  offsets (UTC+9, UTC-5) with one city deliberately placed ON the host's own
  zone — the chip is the difference from local, so a host that is not on UTC
  would otherwise have made the test's expectations a property of the machine.
- 400/400 tests.

## 2026-09-27m (the wallpaper card went declarative — a bound tile grid, and a scrollbar that is now a scene)

- **`wallpaper.ron` is a zero-`Ink` scene.** The tile grid is a `Grid` whose
  column count, cell size, scroll offset, row fit and block origin are ALL
  bound — the card shrinks its cell until ≥5 columns and ≥2 rows fit, so
  none of those numbers is a literal and a breakpoint ladder cannot express
  them (a ladder caps a MAXIMUM cell; this one keeps a MINIMUM count). The
  scrollbar is two `Surface`s (track + proportional thumb) and two page-jump
  `Hit`s, all placed with `right: true`.
- **The whole fit chain became ONE pure function**, `wp_card_geometry`, and the
  card's state is settled by `wp_card_settle` in the `&mut self` layout pass.
  That split is the point: `scene_values` is `&self` and must not move the
  scroll, yet the scene needs the CLAMPED value — publishing `self.wp_card_scroll`
  would let a stale offset draw a page the click handler cannot resolve. The
  geometry reports its clamp as an effective start, and the mutation lives
  beside the `wp_card_rect` write-back.
- **Six new `Grid`/`Hit`/`Surface` capabilities**, each one paying for itself
  here: `Grid.x_var` / `y_var` / `cols_var` / `cell_var` / `surface` /
  `image_inset` / `max_rows_var`, `Hit.h_var`, and `Surface`'s four edge
  bindings. `Grid.max_rows_var` mirrors `Rows.max_rows`: a window is a PAIR,
  `start` says which cells are on the page and this says how many rows the
  page holds — without it the grid tiled its whole tail past the fold.
  `Grid.start` keeps cell keys ABSOLUTE, so a tile below the fold resolves the
  same wallpaper as its row in the list.
- `Surface` hover precedence is now explicit: a cell's own surface, then the
  grid's `hover_surface`, then the grid's default `surface` — so the 295
  wallpaper tiles are all plates without 295 copies of one declaration.
- **A `Header` meta that binds to an empty string is no meta.** The drawers
  pass `None` for a card with nothing to say (the wallpaper card with no
  images); painting an empty label was a command no drawer emits.
- **`ladder_step` stopped double-scaling.** It returned DEVICE px while both
  callers scale on the way out (`scale.s` for a y anchor, `scale.fs` for a
  font), so every responsive ladder squared the scale above `grid_scale = 1.0`
  — the configured value, which is why it went unnoticed. The new parity test
  runs the whole matrix at 1.0 AND 1.75, and at 1.75 it also caught the
  wallpaper drawer emitting a NEGATIVE image size on a card squeezed under its
  own 3 px inset (the `Grid` `image_inset` floor of 1 px is now mirrored).
- `the_wallpaper_scene_matches_its_own_drawer` — 800 cases: five file counts
  (0, 1, 7, 40, 295) × four scroll offsets (top, mid, past-the-end so the clamp
  runs, and an out-of-range one) × five card boxes (a cramped 2-column strip
  to a wide board) × four pointer states (away, over a tile, over each
  scrollbar band) × two scales — comparing the command list AND the sorted
  hit-region list, since a grid that painted the right tiles at the wrong keys
  would look perfect and click the wrong wallpaper.
- `wallpaper_scene_is_zero_ink` pins the bindings a visual check cannot see:
  the absolute key base, the 3 px image inset and the two gated page-jump
  bands (an ungated 0-height region would still swallow clicks).
- 399/399 tests.

## 2026-09-27l (the moon card went declarative — a terminator-disc primitive, and the clock sampled once)

- **`moon.ron` is a zero-`Ink` scene** behind a new `Moon` item: a dark disc
  with the LIT portion painted as terminator slivers, from the cycle fraction
  the shell samples once per frame. The geometry is `ui::moon_disc` — the
  drawer's own function, MOVED, not reimplemented. A primitive with two
  implementations drifts, and the differential test compares the sliver stack
  to the last bit, so the two callers had better be the same code.
- **`Moon` binds its whole box** (`x_var` / `y_var` / `d_var`, the `Text`
  `x_var` rule): the card's diameter is `min(body_h, w − 2·pad − s(90)).max(s(30))`,
  a min/max over BOTH axes, so neither the diameter nor its center can be a
  literal. The `max(s(30))` floor stays in the publisher for the same reason
  `x_var` bindings are published in base px — a bound box skips the item's
  own scaling decisions. The three side captions ride `y_var` too, since every
  one of their baselines is `cy ±` a constant.
- **`moon_epoch` is sampled ONCE per frame** in `layout_for`, not twice by the
  two consumers. The phase math is pure date arithmetic off the wall clock, so
  a frame that sampled it in the header and again in the body could paint a
  header from one instant and a disc from the next — and a pinned value is
  what makes the parity test possible at all. `moon_meta` had been a THIRD
  inline copy of the epoch/synodic constants; it now calls `moon_phase` like
  everything else.
- The card box lands in `moon_rect` from the layout path (the drawer that set
  it stands down), and `moon_phase`/`moon_name`/`days_to_full` went `pub` so the
  scene values can publish the same strings the drawer drew.
- `the_moon_scene_matches_its_own_drawer` — 40 cases: all eight phases
  (new, both crescents, both quarters, both gibbous, full) × five card boxes,
  two too narrow for the side column (the diameter falls back to its 30 px
  floor) and one too short for the body. Plus `moon_scene_is_zero_ink`, which
  also pins the box bindings — a hardcoded diameter would pass a visual check
  and fail a resize.
- 397/397 tests.

## 2026-09-27k (the weather card went declarative — two panes, a height-clamped list, and one gate per line)

- **`weather.ron` is a zero-`Ink` scene**, both panes behind the usual
  pagination rule. Pane 0 is the current conditions: the condition glyph and
  the temperature as a centered *pair* — the temp is not centered on the card
  but at `ccx − (glyph_w + temp_w)/2 + glyph_w`, so its x rides the published
  `wx_temp_x` and the glyph + temp land exactly where the drawer put them.
  Pane 1 is the 7-day lookahead as a `Rows` list (glyph / weekday / rain % /
  right-anchored mono high-low), and `Rows.y_var` anchors its top.
- **The forecast's row COUNT is the drawer's own height arithmetic**, so the
  list is published already clamped to `n_fit = ⌊(h − s(16)) / s(24)⌋`
  capped at 7 with a `max(1)` floor, and `wx_rows_y` carries the matching
  centered top (`max(h − n·row_h)/2`, floored at `s(8)`). A static `max_rows`
  could not: a 200×120 card shows 4 rows and a 260×150 card 5, from the same
  scene. The rain cell is an *empty string* below the 5% cull, which the row
  renderer skips exactly like the drawer's `if`.
- **An empty string does not hide a `Text`** — the engine still registers the
  cell (it painted a space), so the two conditional lines publish their own
  gates. And since a `Text` carries exactly ONE gate, the pane gate and each
  line's condition are published pre-ANDed as `wx_pane0_city` /
  `wx_pane0_detail`. The squat card (h < 60) is pane 0's *only* line, so its
  one dense `wx_squat_text` also folds `!pane1` into its own gate; a card with
  no data yet keeps the centered "waiting…" placeholder, and per the drawer it
  returns before `battery_dots`, so `wx_hover` publishes false there.
- The card box now lands in `weather_rect` from the layout path — the drawer
  that used to set it stands down, and the pair's centering, the row fit and
  the dots' hover gate all read it. `weekday_name` became `pub(crate)` so the
  scene's rows share the one libc rollover instead of restating it.
- `the_weather_scene_matches_its_own_drawer` — 220 cases: 11 samples (both
  panes, the forecast pane with an empty list falling back to pane 0, a dry
  sample with no wind/feels-like so the detail line vanishes, and two
  no-data placeholders) × five card boxes (two below the 60 px squat
  threshold, one between it and the 100 px detail gate, two full size) × the
  dots hovering and not × the pointer in and out of the card. Plus
  `weather_scene_is_zero_ink`.
- 395/395 tests.

## 2026-09-27i (the visualizer went declarative — a bar spectrum, and the header learned its own metrics)

- **`viz.ron` is a zero-`Ink` scene.** Three `Spectrum` items over one bound
  series, gated on the `viz_style` cycle, plus the `no audio` caption behind
  the drawer's own `h ≥ s(90)` window. A new item rather than a `Spark`
  kind because a bar grid and a line trace do not agree on pitch: the bars
  space at `w / n` and stand on the plot's bottom edge, a `Spark` line spaces
  at `w / (n − 1)` and starts a half-thickness pad in. So the bar count IS
  the series length and the shell publishes `viz.len().min(24)` zero-padded —
  the drawer reads `viz.get(i).copied().unwrap_or(0)` past the end, and a
  short capture would otherwise re-pitch the whole grid.
- **`SpectrumKind::{Rounded, Block, Wave}`** carry the drawer's three styles
  with their own width rules (rounded = `bar_frac` of the step with a
  fully-round cap; block = inset from both step edges with a 1 px cap; wave =
  a line through each step's midpoint) and one shared floor: bars under 0.5 px
  are culled, so silence leaves nothing behind. The every-4th-bar accent
  (`accent_every` + `tint` toward fg) is the same rule the pill-row visualizer
  already draws by hand, so the two agree by construction.
- **`Header` grew `title_size` and `title_dx`.** Its defaults stay
  `card_head`'s (11.5 semibold, 17 px past the glyph) so every scene converted
  so far is untouched; a drawer that hand-rolls its own header — the
  visualizer, the equalizer — picks its own, and `viz.ron` declares them
  (12 px, 18 px) instead of drifting a pixel off its fallback.
- **The style button is a glyph-only `Hit`** — no label, so the glyph rides
  `glyph_pad` 2 px inside the 16×20 well the drawer registers, and
  `font_size: 11` because the glyph path runs one px over it (the bump that
  matches a label's cap height).
- `the_viz_scene_matches_its_own_drawer` — 150 cases: all three styles ×
  the empty capture, silence, a 3-sample capture (the pitch must hold), a
  capture straddling the 0.5 px cull and a full 24 × five card boxes
  (one too short for the caption) × pointer in and out of the button, with
  the well compared as a hit box too. Plus `viz_scene_is_zero_ink`.
- 391/391 tests.

## 2026-09-27h (the power-draw trace went declarative — and the parity harness stopped counting lines)

- **`powerdraw.ron` is a zero-`Ink` scene.** The Rust body survives as the
  no-scene fallback and is the geometry of record: the `Header`, the last
  sample of each channel printed beside its B / G dot, and the two traces over
  one plot box. No new primitive was needed — the `latency` card's `Spark`
  item already draws a `ui::pulse` series byte-for-byte. The box is the
  drawer's: `x + 14`, `y + 30`, 80 px of right column reserved (`w: -94` stops
  94 px short of the right edge = `card_w − 108`) and 12 px of bottom pad
  (`h: -12`). The two series are published PRE-NORMALIZED against the
  drawer's shared peak (a 1.0 floor, both channels on one scale), so
  `max: 1.0` plus the `Spark` item's own divide reproduces its arithmetic.
  The dots are the static `acc` / `info` tokens, so no ink is published.
- **The flat idle trace needs no new anchor.** With one sample the drawer
  draws a midline instead of a trace: the plot box starts at `y + 30` and is
  `card_h − 42` tall, so its midpoint is `card_h/2 + 9` — which `cy_pct: 0.5`
  with `y: 9` already expresses. (A `Divider.y_var` was tried first and
  reverted; the arithmetic folds into the existing proportional anchor.) The
  gate is `pw_idle` = EXACTLY one sample: the drawer's empty history returns
  before anything, so a `len < 2` gate would have drawn the midline over the
  empty state.
- **`Spark` and `Divider` grew `min_w` / `min_h`** — floors on the plot box
  after the span resolves, the drawer's own `.max(20.0)` width and
  `.max(s(10.0))` height. Without them a card narrower than 128 px collapsed
  the declarative trace to a 12 px stub while the fallback kept 20.
- **`fmt` now compares `Cmd::Line` and `Cmd::Image` geometry in full.**
  Both used to render as `#{i} <other>`, so the harness counted them without
  looking at them: the media card's artwork box and the clipboard thumbnails
  were only ever compared by ORDER, and the plot floors above slipped through
  a "passing" test. Line endpoints, width and ink plus the image box and key
  are now in the fingerprint. (The media and clipimg scenes were already
  exact; the strengthened harness confirms it.)
- `the_powerdraw_scene_matches_its_own_drawer` — 30 cases: the empty history
  (the drawer's early return), one sample (the midline), a rising pair (the
  ↑ charging arrow), a falling pair (no arrow), a flat four-sample run, and a
  full 60-sample window with both channels moving × 5 card boxes including
  the 120 px one where the plot floors bite. `powerdraw` also left
  `committed_header_scenes_parse`'s hybrid list.
- 389/389 tests.

## 2026-09-27g (the clipboard image list went declarative — and `Rows` grew an IMAGE cell)

- **`clipimg.ron` is a zero-`Ink` scene.** The Rust body survives as the
  no-scene fallback and is the geometry of record: the `Header`, one `Rows`
  row per paste, and the empty state. `y: 32` + `pitch: 28` + `bottom: 4`
  reproduce the drawer's own `floor((h − 36) / 28)` row count (the last row
  must clear `card_h − 4`), so the list needs no published row cap.
- **`SceneCol` grew `image: Option<ImageCellSpec>` — the IMAGE cell**, the
  primitive NEXT_STEPS has listed as the blocker for this card since the
  roadmap was written. The cell's own text IS the renderer's image key
  (interpolated, exactly like the `bar` cell's value) and the cell draws a
  rounded TILE behind it: the drawer's 24 × 22 `hover` plate with the 18 × 18
  paste 3 px in and 1 px above the row top. `keep_empty` leaves a bare tile
  for an empty key; the default draws nothing, matching the standalone `Image`
  item's new empty-key rule. An image cell is geometric, so it is exempt from
  the "empty cell draws nothing" skip (like `dot` and `bars`).
- **`Rows` grew `surface_dy` / `surface_dh`**, the `region_dy` / `region_dh`
  rule applied to the row's BACKGROUND box: this card paints its 6 px-rounded
  band 2 px above the row top and 4 px shorter, so consecutive bands never
  touch. `region_dy: -2, region_dh: -4` reproduces the clickable box, which
  is that same inset rather than the row box.
- **The name ink is published per row** (`col_colors`), not left to a
  per-column `hover` token: the drawer ranks selected ABOVE hovered
  (acc, then hover_fg, then fg) while `col.hover` outranks `col_colors`, so a
  token would have shown hover_fg under a selected row. The selected row's
  band rides the row's own `surface` (the same `hover_hl` the hovered row gets
  from `hover_surface`), which the engine already ranks above `hover_surface`.
- **Every entry is published and the list windows itself** on
  `clipimg_start` (the drawer's own clamped `min(scroll, len − visible)`).
  Publishing only the visible window AND binding the scroll applies the
  offset twice — the engine's `start` indexes the published list. Each row's
  key is window-RELATIVE (`key_base` + its index in the window, which is what
  `input.rs` maps back to a visible slot), so a row above the window is never
  hoverable.
- **`clipimg_card_rect` is published in the zero-`Ink` bookkeeping** — the
  scroll clamp and the visible-row count both read it, and the drawer that
  used to set it no longer runs.
- **The fallback's header moved 1 px** (`y + 10` → `y + 9`): this drawer
  hand-rolled `ui::title` a pixel lower than `ui::card_header` and the
  `Header` item put it. With `scene_owns_header` set that line is dead in
  production, and the fallback now matches the live scene and the other ~20
  cards.
- `the_clipimg_scene_matches_its_own_drawer` — 600 cases (6 lists, including
  the empty one, a 28-char-capped name, and a 12-entry list taller than every
  card in the matrix × 5 card boxes including the 64 px one where the
  drawer's `.max(1)` and the engine's own fit agree at exactly one row ×
  selection at rest / on a visible row / scrolled past the end × hover on the
  selection, on another row, and nowhere) compare both painters' command
  lists AND their hit regions to the pixel. The harness passes the Shell's
  `hover_key` through to the scene: the band behind the hovered row is the
  ENGINE's own hover test, not a published value.
- 388/388 tests.

## 2026-09-27f (the media card went declarative — the album art, and a `Surface` grew a gate)

- **`media.ron` is a zero-`Ink` scene.** The Rust body survives as the
  no-scene fallback and is the geometry of record: the art box (a rounded
  placeholder when MPRIS gave no path, the artwork itself otherwise), the
  semibold title and the album+artist line, the seek bar with its time
  labels, and the three transport buttons. The shell records
  `media_card_rect` in the layout pass (the `accent_rect` precedent) and
  publishes the strings and inks the drawer computed inline — including the
  `fit()` truncation (`floor(avail / (size · 0.62))`, floored at 4 chars, with
  an ellipsis) because a `Text` cell's char cap is a static reserve while this
  cap moves with the card box — plus `media_play_glyph` (play/pause),
  `media_prev_glyph` / `media_next_glyph`, the hover-dependent
  `media_track_ink`, and `media_prev_ink` / `media_next_ink`. The PLAY
  button needs no ink: the drawer passes `pal.acc` as its resting color too.
- **Four gates, one per DRAWN variant**: the info column's x rides the
  `wide` breakpoint (≥ 210 px puts it right of the 48 px art, else under it)
  and `media_has_room` (≥ 120 px) hides the transport row, so the title and
  the line2 are each declared twice at a static x with a combined toggle
  (`media_title_wide` / `_narrow`, `media_line2_wide` / `_narrow`) — `visible`
  binds ONE name and has no conjunction operator, so the emptiness of line2
  is folded into the toggle rather than expressed in the scene.
- **The gates compare DEVICE px, not base px.** The drawer's `w >= 210.0`
  and its `fit()` cap are device-px arithmetic, and the parity test runs at a
  non-1.0 scale; comparing `scale.base(w)` against 210 moved both the column
  and the truncation point and the card stopped matching its own fallback.
  Only the values bound to `x_var` (BASE px, per the item contract) convert.
- **`Text` grew `x_var`** (a bound scalar replaces the declared base `x`,
  the `Hit` rule and `y_var`'s sibling) because the three transport glyphs
  straddle `card_w / 2 ± 36` at the scale. They are left-anchored in the
  ICON face, not centered — the drawer's `text(.., icon = true)` — and so are
  their hit boxes, which sit at the drawer's own `-50 / -22 / +12` offsets
  rather than centered on the glyphs.
- **`Surface` grew `visible`** (the bound-`Toggle` gate every other item
  already has), so the album-art placeholder can yield the same box to the
  real artwork. `SceneItem::Image` now paints NOTHING for a bound-but-EMPTY
  key rather than pushing a `Cmd::Image` with a substitute — the placeholder
  and the artwork share one box, and the art key is `file:<path>` or empty.
- **`media_seek_rect` is recovered from the declared `Hit`**, in the same
  `match hit.key` arm block that already republished the dashboard faders and
  the balance slider: the halo is `(sx − 2, sy − 6, sw + 4, 16)` and the drag
  handler wants the bar itself, so the rect is `(hit.x + 2, hit.y + 6,
  hit.w − 4, 5)`. Without it a seek drag would read the default rect once the
  drawer stood down.
- **`ui::slider_bar` no longer emits a zero-width fill** — the same
  `frac > 0.001` rule the declarative `Bar` applies, so an unfilled slider's
  fallback and scene command lists agree. A zero-width rounded rect paints
  nothing either way, so nothing on screen changes; the helper is used only by
  the media card.
- `the_media_scene_matches_its_own_drawer` — 240 cases (4 track shapes ×
  art/no-art × 5 seek+playing states × 4 hover keys × 3 card boxes) compare
  both painters' command lists AND their hit regions to the pixel, plus the
  recovered `media_seek_rect` and the layout pass's `media_card_rect`. The
  differential caught four things hand-derivation got wrong: the transport
  glyphs are icon-face and left-anchored (not centered), the play button's
  resting color is already `acc`, the wide column is 72 px not 14, and a
  `Ring` value does not interpolate into a `Bar`'s `value` — only `Text` does,
  so `media_frac` is published as `frac.to_string()` (the shortest f32
  round-trip, as `mem` / `disk` / `swap_frac` already do). The test pins
  `media_seek` wherever a live position would be: with `media_playing` and no
  pinned seek the position advances with `Instant::elapsed()` between the two
  paint passes, so a sub-pixel fill could land on either side of the
  empty-fill guard.
- 387/387 tests.

## 2026-09-27e (the accent picker went declarative — and a `dot` grew a height)

- **`accent.ron`'s PICKER is declarative**: the scheme row (its label, the `>`
  chevron, its own 30 px region) and the three accent sources, each with the
  active row's 3 × 12 rail on the left edge. The custom-accents subview stays
  in the Rust drawer — it owns a paged scroll and the `accent_list_rect`
  write-back the wheel handler reads — so every declared item is gated on
  `accent_picker` (the subview is closed) and the resident `Ink` still handles
  the rest. The drawer stands down for the picker (`scene_owns_rows`) and for
  its header (`scene_owns_header`), the worldclock/countdown idiom.
- **`DotSpec` grew `h` (base px, unset → a square of `size`)**, so a `dot`
  cell can be a RAIL — the accent card's active-source bar is 3 wide × 12
  tall, which no square dot could express (`Grid` had a `marker` pip for
  workspaces, but `Rows` had nothing). A dot whose resolved ink is fully
  transparent (alpha 0) now paints NOTHING rather than pushing a
  `Raw(0)` rect, which is how the two inactive sources blank their rail —
  `ColorToken` has no transparent variant, so they publish `clear` by name.
- **`Hit` and `Rows` grew `y_var`** (a bound scalar replaces the declared
  base `y`, the `Text` rule), because the accent block is vertically CENTERED
  with a clamp — `top = ((h − 156) / 2).clamp(30, 48)` — so nothing on this
  card has a static y. The shell records `accent_rect` in the layout pass
  (the `branding_rect` precedent) and publishes the block's four anchors in
  base px: the scheme row's region/label/chevron and the first source row. A
  `Text` has `y_var` but no `dy`, so the drawer's 6.5 / 5.5 insets ride inside
  the published scalar rather than as a nudge.
- **`Rows.pitch` is an `Option<f32>`** — `pitch: 30.0` fails to parse with
  `Expected option`; it is `pitch: Some(30.0)` (news/mirror already did).
- `the_accent_picker_scene_matches_its_own_drawer` — 30 cases (2 schemes ×
  5 hover keys × 3 card boxes, including the short box that hits the clamp's
  low end) compare both painters' command lists AND their hit regions to the
  pixel. The differential caught two things hand-derivation got wrong: the
  scheme row's region is `x: 10, w: -10` (a negative `w` is the span from `x`
  to `right − inset`, so the drawer's `w − 20` is 10 in each side, not 20
  short), and the rail's y is `7` into the row, not 5 —
  `ry + ((30 − 4) / 2) − 6`. The acc source is read from `$states` by BOTH
  sides, so the test asserts it is readable first: an unreadable
  `acc_source_*` would leave the rail cell untested instead of failing.
- 386/386 tests.

## 2026-09-27b (`speedtest.ron` went declarative — and one span rule now means the same thing in every arm)

- **`speedtest.ron` is a zero-`Ink` scene.** The Rust body survives as the
  no-scene fallback and is the geometry of record. New values: `st_state`
  (idle / testing / done), `st_state_ink`, `st_clickable` (the hit region
  exists only between runs, like the drawer's own guard), `st_down` /
  `st_up` / `st_ping`, and the pill's `st_btn_fill` / `st_btn_ink` /
  `st_btn_label` — the shell owns the label so the drawer's
  "probing…" / "down" / "up" / "ping" strings stay in one place. The
  zero-`Ink` branch of `draw_card_scene` keeps `input.rs`'s
  `SPEED_KEY_BASE` click path alive; no side effect rect is needed.
  `Surface` grew a `bottom` anchor (the box's bottom edge `y` px above the
  card's bottom, the `Hit`/`Text` idiom) so the 30 px run pill declares only
  its 8 px lift.
- **A negative span now measures from the item's OWN x/y in every arm, not
  just the graph ones.** `Text`, `Hit`, `Fader`, `Toggle`, `Row`, `Column`,
  `Stack` and `Image` still resolved a negative `w`/`h` against the whole
  card, so `x` was silently ignored — a pill declared `x: 12, w: -50` came out
  50 px short of the card's RIGHT edge while starting 12 px in, i.e. 38 px
  wide on one side and 50 on the other. All of them (with `Surface` and `Bar`)
  now go through `span_px` against `card_w − x` / `card_h − y`, so
  `x: 12, w: -12` is a symmetric 12 px inset, `w: 0` reaches the box's far
  edge from wherever the item starts, and no span hard-codes a card width.
  The committed scenes were re-declared to keep their exact boxes:
  battery's power-save pill `-50 → -38`, compositor's two rows `-24 → -12`,
  the sliders' row stacks `-14 → 0` with their inner rows `-38 → -18`
  (and the balance row `-34 → -14`), the mirror's camera feed
  `-20/-98 → -10/-60`, and the power card's body row `h: -30 → h: 0`
  (`scene::tests::a_negative_span_measures_from_the_items_own_x_and_y`).
- 335/335 tests.

## 2026-09-27 (the disk / gpu / cpugpu card bodies went declarative — and the plot boxes they were drawing came back into the pads)

- **`todo.ron`, `disk.ron`, `gpu.ron` and `cpugpu.ron` are zero-`Ink` scenes.**
  Each Rust body drawer survives as the no-scene fallback and is the geometry
  of record. New generic primitive: the **`Rows` overflow footer**
  (`more` / `more_dy` / `more_fs` / `more_color`) — a centered caption
  `more_dy` under the last DRAWN row that interpolates `{n}` with the hidden
  row count and disappears when the list fits, so the To-Do card's `+N more`
  lands correctly at any card height
  (`scene::tests::rows_more_footer_counts_the_hidden_rows_and_hides_when_done`).
  New values: `todo_count` / `todo_empty` / `todo_empty_ink` (with the zero-`Ink`
  branch of `draw_card_scene` publishing `todo_card_rect` for the click-to-focus
  hit test), `disk_meta` / `disk_rows` / `disk_live` / `disk_empty`, and the
  gpu set `gpu_pct` / `gpu_frac` / `gpu_temp_c` / `gpu_live` / `gpu_empty` /
  `gpu_vram_live` (+ `gpu_vram_txt` / `gpu_vram_frac` only while the source
  reports a VRAM total).
- **Both row caps are geometric, and two of them were lying.** The disk card's
  `((h − 36) / 24).floor()` cap is `Rows { bottom: 4.0 }` and the To-Do card's
  `clamp(0, 8)` is `max_rows: 8` + `bottom: 40.0` — a row may not cross
  `card_h − 40`, the drawer's exact arithmetic. A static row count alone would
  have overflowed a short card.
- **Every dual-series graph was drawing outside its own padding.** `Spark` (and
  `Bar`) resolved a negative `w`/`h` as "the whole card", so a plot declared
  `h: -12.0` ran off the bottom edge and `w: 0.0` ignored its `x` entirely — the
  network, diskio and cpugpu graphs were all 12–28 px too wide/tall, and the
  To-Do-independent cards' plot right edge sat under the card border. Both items
  now measure a negative span against the room LEFT from the item's own x/y
  (the `Tiles` convention), so `x: 12, w: -12` is a symmetric 12 px inset; the
  scenes that need it were updated (network `-12/-21`, diskio `-14/-12`, cpugpu
  `-14/-14`, gpu bars `-12`, mem's swap bar `-14`)
  (`scene::tests::graph_cards_inset_their_plot_box_on_both_sides`).
- **A `Rows` meter cell with `w: 0` no longer collapses.** The bar cell honored
  `w` literally, so the fans card's full-width fan meter — and the new disk
  card's — drew a zero-width track. `w: 0` now runs from the cell's own `x` to
  the row's right edge, which is what a meter wants
  (`scene::tests::rows_bar_cell_zero_width_spans_the_rest_of_the_row`).
- **The diskio and network room checks reached the scene.** Both Rust drawers
  skip the graph entirely below a card height (diskio 78 px, network 59 px);
  each scene now wraps its two `Spark`s in `When(min_h: …)` at exactly that
  threshold, so a short card no longer squeezes a 2-sample line into a sliver.
- **The gpu empty state was gated on the wrong flag.** "no GPU source" hung off
  `gpu_live`, i.e. it appeared precisely when a source WAS present; the scene
  reads the new `gpu_empty` (the inverse) like `card_empty`'s Rust branch, and
  both empty captions sit 6 px above the card's middle to match
  `shared::card_empty`. 331/331 tests.

## 2026-09-26e (one master shader, one switch — `ms_$s` is 0 | 1, hyprsunset is gone)

- **The screen look is one program and one switch.** `states/shader_m_$s` was
  `0 | s | m`; it is now `states/ms_$s` and holds `0 | 1`. `0` drops
  `$hypr/luas/shader.lua` to no `screen_shader`; `1` bakes saturation, gamma
  and Kelvin into the single `$hypr/shaders/master.glsl` and points the config
  at it. There is no Normal mode, no single slot, and no second owner for the
  colour — the Slack/`m` split is gone. The Sliders card's mode list drops
  from three rows to two (`Disable` / `Master`), and the retired `s`/`m`/`g`
  values now parse as *no* value, so a state file that predates the switch
  highlights no row instead of a row that no longer exists
  (`vars::parse_ms`, `scene::tests::sliders_mode_rows_only_call_scr_main`).
- **`states/shader_val_$s` is `states/satu_$s`** — it was always a percentage
  (0..200, 100 = normal), so the name now says so. `ctemp_s` is deleted: one
  flag (`ctemp_v`, 6500 K = the neutral off notch) carries the whole gate.
  `gen_flags_master` seeds the new names, and the live and persisted trees were
  migrated together (`m|s|g` → 1, everything else → 0, old saturation values
  preserved verbatim under the new name).
- **hyprsunset is no longer involved anywhere.** The plugin was not even
  loadable on this install (no `libhyprsunset.so`, `hyprctl plugin list` empty,
  no config, unit disabled), so the daemon half was dead weight: the Kelvin
  fader always re-bakes `master.glsl`, and `scr_main hs-toggle` is now
  `scr_main temp-toggle` — a warm Kelvin snaps to the 6500 K notch, a neutral
  one goes back to 4000 K. `DashCmd::SunsetToggle` keeps its variant and its
  chip and routes to the new verb, so the chip is now the warmth gate on the
  shader rather than a daemon state. `sources/sr/hs_main_restore` is deleted.
- **`sources/shader_master_body` and `sources/shaders_gen` are gone.** The GLSL
  is back inline in `scr_body` (the extraction only made sense while a
  per-channel mode had to choose between two shader files), and the per-id
  saturation/gamma generator went with the per-id pool — nothing generates
  `shaders/saturations/*` or `shaders/gamma/*` any more, and the already-baked
  files are orphaned. The `dofile` in `init_logic` is removed with it.
- **The shader-order fix from `d` stands and is now the only path.** Saturation
  mixes the raw texture toward luma, then the Kelvin lands on that result as one
  per-channel factor `kelvinToRGB(k)/kelvinToRGB(6500)`, then gamma applies
  `pow(c, (20-g)/20)`. Verified numerically: the Kelvin ratio is identical at
  sat 0/40/79/100/200 (max deviation 2.2e-16) and the saturation level is
  unaffected by the Kelvin — the two are independent. Gamma is still a
  per-channel `pow` as supplied, so it moves brightness and compresses the
  channel ratios (deviation from gamma-first is 0.08 at gamma 5, 0.65 at
  gamma 19); it is the brightness knob, not a third independent axis.
  Normalising it by the max channel would decouple it, but that is not the logic
  that was asked for and it was not substituted.
- **The Sliders card no longer has a shader-mode list at all.** With the switch
  down to `0 | 1` there was nothing left for a list to choose between —
  `Disable` and `Master` were two spellings of one flag, next to a chip and two
  faders that already drive the same flag. The pane-1 `Rows` item, its "Shader
  Mode" title, the per-row `Command`s, the `sliders_mode_rows` binding, the
  `shader_mode` shell field, the 120 ms `poll_shader_mode` timer (and its
  `SLIDERS_MODE_ROW_BASE` keys 30_722..30_724) are all removed, and the two
  remaining switches now center in the whole pane instead of sitting under the
  list. `DashCmd::ShaderToggle` (the chip) and the `scr_main` verbs
  (`toggle`/`off`/`master`) are untouched — the flag and its CLI remain, only
  the redundant card UI is gone. `scene::tests::sliders_card_has_no_shader_mode_list`
  is the guard that keeps it from creeping back.
- Any knob now implies the switch on (`_shader_imply_on`), so dragging the
  Kelvin or gamma fader on a disabled screen lights the shader instead of
  silently doing nothing. Nothing auto-drops back to 0: only `scr_main off` or
  the Disable row clears it. The 120 ms poller (`App::poll_shader_mode`) still
  reads the flag directly so the wash follows the click that wrote it, and now
  covers knob writes too.

## 2026-09-26c (`shader_m_$s` is 0 | s | m — the `g` flag is gone)

- **A card row's click did nothing, because the child had no PATH to the
  helpers.** The scene-declared `Command` escape hatch spawned
  `sh -c "<cmd>"` with `PATH=/usr/bin:/usr/local/bin:/bin` pinned, but the rconf
  helpers live in `$sources/bin` — the first entry of the shell's own PATH — so
  every rconf-backed row (the Disable/Normal/Master list included) died as
  "command not found" with nothing shown. The child now inherits the shell's
  PATH, exactly like the shell's other spawns (`Command::new("scr_main")`,
  `dash_status`). `a_scene_clicks_command_can_reach_scr_main` runs the spawn the
  way the handler does and pins that `scr_main` resolves.
- **`$states/shader_m_$s` now has three values, not four.** The gamma mode is
  not a mode any more; gamma and saturation are the two dials of the one `s`
  slot:
  - `0` — nothing applied.
  - `s` — the single shader slot: it reads `gamma_v` and applies
    `$shaders/gamma/<id>.frag` whenever that level is not 0, and only when gamma
    is back at normal does the saturation shader
    `$shaders/saturations/<id>.glsl` run instead. Both normal leaves nothing to
    show, so the mode drops to 0 (unchanged rule, now the only place it lives).
  - `m` — the mix: saturation + gamma + Kelvin all baked into one
    `master.glsl`, so master keeps needing no special case.
- All of it is in `scr_body`: `shader_m_get`/`shader_m_set` accept only
  `0|s|m`, `shader_restore_func` grew the `s` arm (gamma first, saturation only
  once gamma is 0), `gamma_set_func` lands in `s` and lets the restore decide
  instead of claiming a gamma mode, and `shader_gamma_func` (the `gamma` verb,
  which the mode list's middle row calls) is now "single slot, gamma showing" —
  it seeds level 1 and selects `s`.
- **The mode highlight now comes from the flag file, not the status line.** The
  shell reads `$states/shader_m_<channel>` directly (`vars::read_shader_mode`,
  strict domain `0|s|m` with a legacy `g` read as `s`) on its own 120 ms beat —
  the 3 s services tick would leave the old row washed for three seconds after a
  click, and this way a `scr_main` call from a terminal or a keybinding lights
  the right row too. It renders only when the value actually changes, and an
  unreadable/out-of-domain file keeps the last value (no row beats a wrong one).
  `apply_dash_status` no longer parses field 13, so there is one source of truth
  for the mode instead of two that can disagree.
- The sliders card's pane 1 now reads `Shader Mode` over three rows labelled
  **Disable / Normal / Master** (the title used to be missing and the rows read
  "Normal shader (gamma)" / "Master shader"). Each row still only calls
  `scr_main` — Disable runs `off` (mode `0` + `shader.lua` emptied), Normal runs
  `on` (the `s` slot), Master runs `master` (the mix) — and `scr_body` owns what
  those mean. The list moved to `y: 32` (`row_h: 21`, `pitch: 25`) to make room
  for the title, which pushed the two row switches to `y: 110`.
- Pane 1's switch `Column` also had a real layout bug: `h: -16.0` is a *height*
  taken from the item's own `y` (`span_px`), not a bottom inset, so the column
  was really 90..274 in a 200 px card and both switches drew past the card's
  bottom edge and into the pagination dots' band. `powerh`/`powerv` already use
  the house idiom (`y: 30, h: -30` = "30 to the card bottom"), so pane 1 is now
  `y: 110, h: -126.0` and the switches center in 110..card_h − 16. `appshortcut`,
  `diskio` and `network` have the same `y > 0` + negative-`h` shape and still
  need a look.
- A state file still holding a legacy `g` migrates instead of going dark:
  `shader_m_get` reads it as `s`, and `brightness_body`'s `_gamma_active` (the
  only other reader) treats `g` the same way. The shell's mode list lost its
  `g` row's special case, and `apply_dash_status` maps a stray `g` to `s`.
- Verified live: `gamma-up` from 0 → `gamma/1.frag`; gamma 3 + `set 140` → the
  gamma frag stays (gamma wins the slot); `gamma-reset` → the saturation shader
  takes over; both normal → mode 0 and `shader.lua` is emptied.
- Same session, on the Kelvin path (all in the scripts, no Rust logic added):
  the `hs_cmd("set", K)` daemon branch wrote the Kelvin into `ctemp_s` — the
  state file — instead of `ctemp_v`, so every drag re-applied the previous
  temperature; `hs_manual_func` now parses with `_id` (the seeded value file
  holds a quoted `"4500"`, which `tonumber` reads as nil) and refuses the
  neutral notch; `hs_restore_func` was restarting `hyprsunset -t m`, the state
  letter, as a Kelvin. `scr_main status` prints a bare number so `dash_status`
  stops pinning the fader to its 4000 default, and `scr_body`'s
  `hs_toggle_func` had a syntax error (`if state == "c" or state == nctemp_s`,
  no `then`) that made *every* `scr_main` verb fail to load. `audio_body`'s
  `tonumber(f:gsub(...))` passed gsub's count as a base, which killed
  `dash_status` entirely. Regression test `kelvin_fader_spans_1200_to_6500`
  pins the fader's 1200..6500 mapping and its release-to-`scr_main` contract;
  `dash_cmd_call` had been pasted inside `impl App`, which is why the crate
  stopped compiling.

## 2026-09-26b (Kelvin fader routes through `shader_main` — no `hs_main`)

- **The sliders card's temperature fader no longer spawns `hs_main`.** It calls
  `shader_main temp K` (`shell::kelvin_cmd_args`, used by the
  `DashCmd::Kelvin` arm) and `shader_main` decides WHO owns the screen, so the
  shell holds no rule about it:
  - **flag `m`** — the shader bakes Kelvin, so the value is written, the
    hyprsunset daemon is stopped and `master.glsl` is re-baked. Before this,
    dragging the knob in master mode wrote the state file, restarted the daemon
    and left `master.glsl` holding the PREVIOUS Kelvin, so the fader never
    touched the shader it was supposed to be one knob of.
  - **any other flag** — hyprsunset owns the colour and that is `hs_body`'s
    job, so `shader_temp_func` `dofile`s `hs_body` and calls its own `hs_cmd`
    directly. `hs_main` is only a 4-line CLI shim over that body, so going
    through it spawned a second interpreter to move one number.
  - The master branch stopping the daemon is not cosmetic: with the daemon
    still running at the same Kelvin, the bake and the daemon would apply it
    twice. It is the rule `temp_get_func` already documents ("the shader and
    the daemon never double-apply"), now enforced on the write path.
- Bounds are clamped to `TEMP_MIN` 1200 / `TEMP_MAX` 6500 and floored to an
  integer, so a float never reaches a state file.
- Regression test `kelvin_fader_defers_to_shader_main`: the backend call is
  `shader_main temp <K>` (never `hs_main set`) and a key-55 drag still defers
  `DashCmd::Kelvin` to pointer release.
- `DashCmd::SunsetToggle` still calls `hs_main toggle` — that control IS the
  daemon's own on/off, not a shader knob.
- Verified live: mode `m` + `temp 2200` baked `#define TEMP_VAL 2200.0` and left
  no daemon running; mode `s` + `temp 3000` restarted `hyprsunset -t 3000`.
  State returned to `s 101` / gamma 0 / `4245m` / `hyprsunset -t 4245`.

## 2026-09-26 (Sliders pane 1: the shader-mode selector — zen-shell only CALLS)

- **The sliders card's second pane now carries an always-visible 3-item mode
  list** — `Disable` / `Normal shader (gamma)` / `Master shader` — above the
  "show balance" / "show saturation" row switches. It is a declarative `Rows`
  item (`sliders_mode_rows`, keys 30_722..30_724); each row's text and its
  resting `SelBg` wash come from the per-frame binding, so the live mode
  highlights itself. Mode `s` (the pane-0 saturation fader drove it) matches no
  row and highlights nothing: the list SELECTS modes, it does not mirror a
  fader.
- **zen-shell holds no shader logic.** Every row carries its own
  `SceneAction::Command`, so a click runs `shader_main {off|gamma|master}`
  through the `scene_click_actions` escape hatch and no key is handled in Rust
  — the mode rules (a gamma level of 0 means normal, master keeps every knob)
  stay in `shader_body`. Regression test
  `sliders_mode_rows_only_call_shader_main` walks the published rows for modes
  0/g/m/s and asserts each carries exactly one `shader_main …` command.
- **hdots side**: `shader_body` grew `shader_gamma_func` (`gamma` / `normal`) —
  gamma-only at the level the knob already holds, FORCING the mode to `g`
  (a selector, not a knob nudge: `gamma_set_func` keeps `m`), and starting the
  knob at 1 when it sat at 0 rather than dropping straight back to normal.
  `dash_body` now prints the mode LETTER as a trailing 13th field so a mode
  selector can highlight the live row; field 6 stays the 0|1 "any shader" bit,
  so every existing consumer is untouched. `apply_dash_status` reads it into
  the new `Shell::shader_mode` (`0`/`g`/`s`/`m`) and falls back to the 0|1
  bit's implied mode against an older 12-field `dash_body`.
- **Pane-1 geometry**: the mode list sits at `y: 12` (22 px rows, 26 px pitch,
  12 px inset) and the switch column moved to `y: 90, h: -16, valign: middle`,
  so the two switches center in the band between the list and the pagination
  dots at any card height. `sliders_rows_land_on_the_drawer_geometry` derives
  the two switch tops from that formula now.
- **Pre-existing failures, NOT from this batch** (5, all in the water/gauges/
  eyerest/header work that landed after the last handoff):
  `committed_header_scenes_parse`, `eyerest_ring_hero_and_buttons_match_the_drawer`,
  `gauges_hero_and_captions_ride_the_size_ladders`, `water_ring_grows_with_the_card_…`,
  and `sliders_rows_land_on_the_drawer_geometry` — the last one fails in its
  PANE-0 label block (`Cmd::Text` lands at `2×(card_w−14)`, so the scene's
  right-anchor is being applied twice); verified pre-existing by rebuilding the
  pristine pane 1 and re-running.
- Note: the card scenes the tests read are the LIVE tree
  (`~/.config/zen-shell/ui/cards/*.ron`), not `config/zen-shell/ui/cards/` —
  the repo copy holds only the 20 most recent cards. Copy the edited file
  across or the tests (and the shell) keep drawing the old body.

## 2026-09-25 (Batch 3 — battery cards: new `Battery` item, `Hit.bottom`, inert `Rows`)

- **Battery H / Battery V are now zero-`Ink` declarative cards**
  (`ui/cards/batteryh.ron`, `ui/cards/batteryv.ron`): header, the pane-0 gauge
  + centered percent, the pane-1 metric rows, the power-save pill and the pane
  dots. Swipe pane flips, the dots hover gate and the pill key
  (`BATTERY_PSAVE_KEY` 32130) behave exactly as before.
- **New generic engine features**:
  - `Battery { value, max, level, visible }` — a vector-crafted battery gauge
    (body outline, corner caps, charge fill, upright charge ladder, the AC bolt
    glyph). The art came out of `cards/shared.rs` into `scene.rs` as
    `push_battery_icon_parts`, so the scene draws the very same geometry. With
    `w`/`h` both `0` it re-derives the drawer's own box per orientation: the
    flat card `(card_w·0.46).clamp(40,220) × (card_h·0.36).clamp(16,60)`
    centered at `42 + (card_h − 70)/2`, the upright card
    `iw = ((card_w·0.30).clamp(16,70)·2/3).max(12)` wide and `iw·1.7` tall
    centered at `38 + (ah − iw·1.7)/2` with `ah = (card_h − 44).max(aw·1.6)`.
  - `Hit.bottom: true` — anchors the box's BOTTOM edge `y` px above the card's
    bottom (the `Text.bottom` mirror), and `Hit` now honours NEGATIVE `w`/`h`
    spans (`w: -50.0` = card width − 50), so a pill can fill the width between
    two margins: the power-save pill is `x: 12, y: 2, w: -50, h: 22, bottom`.
  - `Rows.key_base` is now `Option<u32>`: an info list that omits it registers
    **no** clickable regions. Bare `index` keys used to squat on the global
    1 / 2 / 3 keys, and 16 committed scenes (battery, alarms, cpu, mem,
    docker, todo-adjacent lists…) were quietly doing it. A row with its own
    `key` still opts in.
- **Per-frame bindings** (`scene_values`): `batt_pct` / `batt_pct_txt` (the
  percent plus the bolt on AC), `batt_ink` (`battery_level_color`),
  `batt_absent`, `batt_h_pane_0/1` + `batt_v_pane_0/1` (each gate folds in the
  no-battery guard, exactly like the drawer's early return),
  `batt_h_hover` / `batt_v_hover`, `batt_info` (4 rows), and the pill's
  `batt_psave_bg` (3-state) / `batt_psave_ink` / `batt_psave_lbl`. New
  `Shell::power_save_state()` reads the state file directly (the battery cards'
  Rust drawers were its only `load_power_save` callers and they no longer run),
  and `Shell::battery_dots_hover()` mirrors the drawer's cursor-in-card test.
  `draw_card_scene` publishes `battery_h_rect` / `battery_v_rect` in its
  zero-`Ink` branch, so the swipe hit-test and the dots gate still see the card.
- **RON gotchas learned here**: a card-level `Text` has NO implicit width
  (`w: 0.0` centers on its own anchor), so the percent needs a full-card
  `Stack` wrapper to inherit the card width; `bottom: true` takes a POSITIVE
  offset (`y: 22.0` = 22 px above the edge, not `-22`); `SceneCol.x` is a
  required field (use `x: 0.0` with `edge:`); and `Rows.key_base` needs the
  explicit `Some(n)` wrapper like every other `Option` field.
- **Tests** (ten new, **304/304 green**): the scene pair
  (`battery_scenes_parse_zero_ink_with_panes_dots_and_the_pill`,
  `battery_gauge_lands_where_the_drawer_puts_the_icon`,
  `battery_percent_text_centers_above_the_bottom_edge`,
  `battery_info_rows_pitch_18_and_right_edge_38`,
  `battery_power_save_pill_anchors_above_the_dots_and_takes_its_key`,
  `battery_dots_mark_the_active_pane_and_need_the_cursor`,
  `battery_absent_shows_the_header_and_the_empty_caption_only`), the new item
  (`battery_item_honors_its_gate_value_and_max`) and the two engine changes
  (`hit_bottom_anchors_its_box_and_negative_span_shrinks_to_the_card`,
  `rows_without_a_key_base_register_no_regions`).
- **Known, deliberate difference**: the drawer's pill click region grows 3 px
  past the capsule on every side; the scene registers the capsule itself (the
  visible target is identical, the margin is not expressible as a span).

## 2026-09-25 (Batch 3 — power cards: `Row.halign`, `Bar.vertical`, `Surface.stroke`)

- **Power H / Power V are now zero-`Ink` declarative cards**
  (`ui/cards/powerh.ron`, `ui/cards/powerv.ron`): the header plus the five
  power actions (lock · logout · sleep · restart · shutdown), each a `Stack` of
  a declared background + hairline, the hold-to-confirm liquid, the 14 px
  glyph and its hit region at the resident key (`POWER_CARD_KEY_BASE` 12000 +
  i), so the hold flow in `input.rs` is untouched.
- **Three small generic engine additions** (all reusable, all tested):
  - `Row.halign: center` — centers a block of FIXED-width children in the row
    box (the mirror of `Column.halign`); children that fill the width already
    span it, so it is a no-op for them.
  - `Bar.vertical: true` — fills the box BOTTOM-UP (height scaled by the
    fraction, anchored on the bottom edge): the hold-to-confirm liquid. A
    horizontal bar is unchanged, so the field defaults to off.
  - `Surface.stroke` + `stroke_color` — a hairline `Cmd::Outline` drawn OVER
    the fill on the box's own rect (the buttons' 1 px edge).
- **Per-frame bindings** (`scene_values`): `power_bg_N` climbs
  `raised → raised_hl → acc_tint` as the pointer arrives and the hold starts;
  `power_hold_N` carries the hold fraction (read against `max: 1.0`);
  `power_hold_on_N` mirrors the drawer's own 1.5 px sliver threshold (a fresh
  press paints nothing); `power_ink_N` tints the danger glyphs
  (`mix(DANGER, fg, 0.25)`); `power_hold_fill` is the accent-mixed liquid, and
  a new global **`clear`** (`Raw(0)`) gives the `Bar` an invisible track.
- **Geometry notes worth keeping**: a container box that should span "the rest
  of the card" is declared `h: -30.0` (negative = parent minus the inset, the
  same trick the compositor's `w: -24` uses) — `h: 0.0` would span the FULL
  card height from `y` and push the block 30 px too low. In RON, an
  `Option<ColorToken>` field needs the explicit `Some(...)` wrapper
  (`stroke_color: Some(hairline)`, `color: Some(value("power_ink_0"))`); a bare
  token only works for non-`Option` fields like `Surface.color` and `Bar.track`.
- **Tests** (seven new, **294/294 green**): the two engine features
  (`row_halign_centers_a_block_of_fixed_width_children`,
  `bar_vertical_fills_bottom_up_and_surface_strokes_over`), the scene pair
  (`power_scenes_parse_zero_ink_with_five_hit_buttons`,
  `power_buttons_land_on_the_drawer_geometry` — 32 px r-10 buttons on a 41 px
  pitch, the block centered in the body below the 30 px header, hairline +
  14 px glyph per button, no liquid at rest),
  `powerv_stacks_the_buttons_in_one_centered_column`,
  `power_hold_fills_the_button_bottom_up` (half a hold = a 28×16 liquid, 2 px
  inset, bottom-anchored, accent-mixed) and
  `power_hold_stands_down_under_the_sliver_threshold`.

## 2026-09-25 (Batch 3 — compositor: `Row.min_card_h`, centered text in containers)

- **Compositor / effects is now a zero-`Ink` declarative card**
  (`ui/cards/compositor.ron`): the header, the three effect tiles (Blur ·
  Shadow · Opacity) in one equal-column `Row` of `Stack` tiles (fill, 5 px
  state dot, label, `on`/`off` caption), and the two screenshot buttons
  (Area · Full) below. Keys stay at the resident values (`COMP_BLUR_KEY` 31400
  … `COMP_SHOT_FULL_KEY` 31404), so the click dispatch in `input.rs` is
  untouched, and the card moved out of the hybrid-stub list in
  `committed_header_scenes_parse`.
- **The tiles' four state inks ride per-frame values** (`comp_tile_N`,
  `comp_dot_N`, `comp_ink_N`, `comp_state_N`, `comp_shot_N`, published in one
  `format!` loop in `scene_values`): a lit tile fills with the accent at 16%
  alpha and lifts to 25% under the cursor, its dot and label go accent, and the
  caption spells `on`/`off` — blends a static token cannot spell, exactly like
  the drawer's `if on … else if hov …` ladder.
- **New `Row.min_card_h`** — the card must be at least this tall for the row
  to draw at all (`0` = always). The screenshot row declares `min_card_h: 110`
  (78 px of rows + 30 px of buttons + the 2 px the drawer keeps clear), so on a
  short card the whole row stands down instead of overflowing — the drawer's
  `if sy + sh <= y + h - 2.0` guard, declared.
- **Gotcha pinned by test** (and documented in the scene): a `Text` centered
  inside a container needs `center: true` with `w: 0.0` and NOTHING else. The
  container's fill pass hands it the full inner width and `draw_text` centers
  within that box, so it already lands on the parent center — adding
  `center_x: true` re-anchors the box's left edge to the center first and
  pushes the glyph a half-width right. (`Surface.center_x` is different: it
  places the box's LEFT edge on the parent center, so a 5 px dot needs
  `x: -2.5, center_x: true` to sit centered.)
- **Tests** (three new, **287/287 green**):
  `row_min_card_h_skips_the_row_on_a_short_card` (109.9 vs 110),
  `compositor_tiles_land_on_the_drawer_geometry` (equal-third tiles at
  y = 30, 40 px tall, r 7, the 5 px dot at +7 centered, label at +17, caption
  at +29, the shot row at 78, 30 px buttons, the glyph 30 px left of the
  button center) and `compositor_shot_row_stands_down_on_a_short_card`.

## 2026-09-25 (Batch 3 — toggles: a new `Tiles` primitive, fully declarative)

- **Toggles is now a zero-`Ink` declarative card** (`ui/cards/toggles.ron`):
  the six switch tiles (Wi-Fi · BT · DND · Caffeine · Sunset · Shader) are one
  `Tiles` frame, with the per-tile data published per frame on the
  `toggles_tiles` list (region key, icon glyph, caption, live on/off state, and
  the Wi-Fi / BT corner-chip key 42 / 44). Keys stay at the resident values
  (41, 43, 45-48 + chips 42 / 44), so the click dispatch in `input.rs` is
  untouched.
- **New primitive `SceneItem::Tiles`** — the tile-grid analog of `Grid`, for
  "pill" bodies where each cell is a rounded surface carrying an icon glyph and
  an optional caption. The RON owns the FRAME, Rust publishes the DATA (the
  `Rows` / `Grid` split):
  - `cols_break: [(225.0, 2), (330.0, 3)]` — responsive columns from ascending
    min-card-width pairs, so the drawer's `if w >= 330 {3} else if w >= 225 {2}
    else {1}` becomes data; the breakpoints read the CARD width even when the
    item box stops short of the card edge.
  - `label_min_h: 42.0` — a tile shorter than this drops its caption and
    centers the glyph instead (the drawer's `labeled` branch), so one
    declaration reads on a squat card and a tall one.
  - the ink ladder `on_surface` / `hover_surface` / `off_surface` (default
    `acc_tint` / `hover_hl` / `hover`) + `on_ink` / `off_ink` / `label_ink`;
    an "on" tile outranks hover, exactly like the drawer's
    `if on { acc_tint } else if hov { hover_hl } else { hover }`.
  - `chip` + `chip_w` / `chip_size` / `chip_dx` / `chip_dy` — the always-visible
    ⋯ chevron over a TALL right-edge region (the drawer registers an 18 px
    strip spanning the tile's height, not a corner box). Regions register
    tile-then-chip, so the strip wins the overlap like the drawer's order.
  - spans resolve through `span_px` (`w/h: 0` → fill, negative → stop short),
    and the item honours a `visible` gate; an unbound tile list draws nothing.
- **New `SceneTile` cell type** (`key`, `glyph`, `label`, `on`, `chip_key`) with
  a `SceneValue::Tiles` slot + `SceneValues::tiles(name)` accessor.
- **Tests** (nine new, **284/284 green**): `tiles_breakpoints_pick_the_columns_the_drawer_picks`
  (224/225/329/330 px edges, tile span + pitch per column count, all six
  regions), `tiles_breaks_only_on_the_card_box_width`,
  `tiles_squat_tiles_drop_the_caption_and_center_the_glyph`,
  `tiles_label_sits_below_the_glyph_on_a_tall_tile`,
  `tiles_on_state_outranks_hover_and_lights_the_glyph`,
  `tiles_chip_is_a_tall_right_edge_region`, `tiles_chip_ink_lifts_on_hover`,
  `tiles_gate_and_missing_binding_draw_nothing`, and
  `toggles_scene_parses_zero_ink_with_a_responsive_tiles_grid` (the committed
  RON declares the drawer's numbers: pad 10, gap 8, r 12, the 225/330
  breakpoints, the 42 px caption floor, the ink ladder, the chip).
## 2026-09-25 (Batch 3 head — sliders fully declarative + two engine footguns fixed)

- **Sliders is now a zero-`Ink` declarative card**
  (`ui/cards/sliders.ron`): pane 0's six fader rows (brightness · volume ·
  balance · mic · saturation · kelvin) in a vertically centered `Column`, pane
  1's "show balance" / "show saturation" switch rows, and the HARD-RULE
  pagination `Dots`. Each fader row is a `Stack` (row-wide hover/drag `Hit`
  carrying the icon, the groove, the right-anchored value label); balance +
  saturation carry `visible` gates so switching them off collapses the row and
  the block recenters, like the Rust drawer filtering its row list. Keys stay
  at the resident values (51-55, 59, reset 58/79, switches 30720/30721) and
  `draw_card_scene` now mirrors each fader's resolved geometry into
  `faders[]` / `balance_rect`, so the drag math in `input.rs` keeps working
  against a scene-defined groove.
- **New live values** (`scene_values`, sliders block): `sliders_pane_0`,
  `sliders_pane_1`, `sliders_pane` (Ring), `sliders_hover`, `show_balance`,
  `show_saturation`, `balance_fader`, `sat_fader`, `kelvin_fader`, `vol_muted`,
  `bri_lbl`/`vol_lbl`/`mic_lbl`/`sat_lbl`/`kel_lbl`, `vol_glyph`; dynamic
  colors `vol_fill` / `mic_fill` (RED when muted) and `kel_fill`
  (warn→accent warmth), `sliders_tog_bg_0` / `sliders_tog_bg_1` (hover wash).
- **Engine: `Fader.head` / `Fader.centered` / `visible` on more items.** A
  fader can declare its knob size (hover +2 px), fill from the middle
  (`ui::fader_bal`, the balance + saturation rows), and be gated by a live
  toggle. `span_px` (the Image span-minus convention) now resolves fader /
  row / column / stack / toggle boxes, and `Column.valign` centers a stacked
  block. `Hit` grew a `visible` gate: a gated-off row's region goes with the
  row instead of lingering as an invisible click target.
- **Engine: right-anchored `Text` inside a container no longer drifts.** A
  `Stack` treated any child with `w: 0.0` as a fill and rewrote its width —
  for a `right: true` label (whose anchor is the box's right edge minus `x`,
  independent of `w`) that pushed the label a full box-width to the right.
  Right-anchored texts are now exempt from the fill pass.
- **Three live scenes were silently broken** and are fixed: `activewin.ron`,
  `pkgupdates.ron`, `clipboard.ron` all wrote `visible: "name"` where the
  schema wants `visible: Some("name")` — a scene that fails to parse falls back
  to the Rust drawer without a word, so these cards had been rendering the old
  paint job with no signal. `every_live_card_scene_parses` now walks
  `ui/cards/*.ron` and parses each one, so this class of rot can't hide again.
- **Test suite: 275 passing (was 258).** Ten committed-scene tests were never
  running: an unterminated `r#"` raw string in the test module (two of them)
  swallowed ~240 lines of test code, so ten tests silently didn't exist and one
  was failing on a garbage blob. Both terminators are restored, `SceneHit`
  derives `Debug`, and the two revived tests that encoded stale assumptions
  (grid cells only paint a chip when they carry a `surface`; `pkgupdates` is
  now zero-`Ink`) were updated. New tests: fader span/centered-fill/gate,
  container skip-invisible-child, `Column.valign` middle, sliders zero-`Ink`
  + drawer-geometry parity, and the live-scene parse guard.

## 2026-09-17 (Batch 2 tail — session paused mid-conversion)

- **`GridCell.sub_dy`/`sub_size`**: per-cell nudge + size override for the
  second line (thermal zone chips: zone caption + temp — the Rust drawer's
  cy+3 / cy+12 rows ≈ centered label + sub at dy −1, size 9). `thermal_cells`
  now publishes the full parity recipe (sub + warning-ladder sub_color +
  sub_dy −1 + sub_size 9).
- **Remaining-Batch-2 bindings all landed in scene_values**: `jr_tail_rows`
  (newest-first 5-row journal tail, prio→Danger/Warn/Fg2 ladder) + `jr_empty`;
  `sus_rows` (newest-first raised chips, danger name + ✕) + `sus_empty`;
  `sm_rows` (mono dev / dim model / temp warn≥55° / ✓✕ status glyph) +
  `sm_empty`; `thermal_cells`/`thermal_empty`; `conn_rows_v`/`conn_empty`
  already existed. Header metas (`jr_meta`/`sm_meta`/`sm_icon`/`sus_meta`/
  `sus_icon`) were in from earlier passes.
- Tests: 258/258, bin builds clean.
- **Paused here**: the 5 RON scenes (journaltail, systemdunits, smarthealth,
  thermal_zones, conninfo) not yet authored — worldclock deliberately stays
  Rust (stateful search overlay/hover-✕). Full state in NEXT_STEPS.

## 2026-09-16 (Batch 2 Spark trio — network/diskio/sensors fully declarative)

- **`net_meta` live value**: the network graph's y-axis caption (the Rust
  drawer's exact bytes→bits ladder: Mbps/Kbps/bps against the deque peak),
  empty until two samples. The `diskio` legend colors bind plain `info`/
  `acc` tokens (no new values needed).
- **Sensors live values**: `sensors_none`/`sensors_tilt`/`sensors_compass`/
  `sensors_gyro` presence gates + `sensor_tilt`/`sensor_compass` (octant
  gauge glyph baked into the string, Rust's exact 8-bucket ladder)/
  `sensor_gyro` (mono triplet) — all computed in `scene_values` with the
  drawer's byte-for-byte formats.
- **network / diskio / sensors converted (zero Ink)**: network = Header
  (live y-axis caption) + dual pre-normalized Sparks (down = info, up =
  accent, negative-h fill stopping above the speed row) + a bottom-anchored
  Row of ↓/↑ glyph+mono speed groups (Rust's 26px gap); diskio = R/W legend
  dots at the exact Rust insets + title-only Header + dual Sparks;
  sensors = accent-glyph Header + one row per exposed sensor (gated, right
  values, compass accent) + centered empty caption. Covered by
  `committed_batch2_spark_scenes_parse_zero_ink` (zero-Ink + both series
  bound); network left the Header-committed set accordingly.
- Tests: 258/258.

## 2026-09-16 (Batch 2 openers — procmon/fans/topproc fully declarative)

- **`Rows` per-row BAR cell (`SceneCol.bar: BarCellSpec`)**: the meter-row
  engine feature the Batch 2 list cards needed. A bar cell interpolates its
  cell text (`{name}`) and parses it as a fraction of `max` (default 1.0),
  drawing a track across the cell box with the fill hugging the left —
  honoring `x`/`w`/`right`/`edge` like every cell. `fill` defaults to the
  cell's resolved color (so a per-row `col_colors` ink drives the fill),
  `track` defaults to `hover`. Spec: `height`/`top`/`radius`/`max`/`fill`/
  `track`. Covered by `rows_bar_cells_parse_value_fill_and_track`,
  `rows_bar_cell_zero_width_draws_degenerate_track`,
  `rows_bar_cell_draws_track_and_fraction_fill`.
- **Live row bindings upgraded**: `top_procs_rows` now carries
  label · tick-fraction (of the top process, the drawer's implicit 100%
  reference); `fans_rows` carries label · rpm · slow-peak-decay fraction with
  per-column inks (label fg2, rpm fg/fg3 by liveness) + the `fans_empty`/
  `fans_live` gates.
- **procmon / fans / topproc converted (zero Ink)**: procmon = Header (live
  "CPU {cpu}%" mono meta) + truncated label + right-anchored accent meter;
  fans = Header + centered empty caption (`fans_empty`) + rows gated on
  `fans_live` with edge-anchored info bars; topproc = Header + 26px rows with
  track-toned neutral bars (Rust parity: it drew track-only). Covered by
  `committed_batch2_bar_row_scenes_parse_and_drive_rows` (zero-Ink + Rows +
  bar cell per scene); `fans` left the Header-committed set accordingly.
- Tests: 257/257.

## 2026-09-16 (Dashboard viewing strip CHIPS declarative — BannerRow)

- **`SceneValue::BannerCells` + `BannerCell`/`BannerCellKind`**: the typed
  data binding for the strip — per-frame geometry from `banner_cells(w)` (the
  exact function the Rust drawer hit-tests with, so scene and input can never
  disagree), a live **ink name** per chip (each `bchip_*` binding mirrors its
  Rust drawer arm's color ladder: wifi accent-while-named/dim-off, battery
  danger-under-20 unplugged, cpu/ram threshold ladders, dnd bell accent,
  conn/offline dim, workspaces hover→acc), the pre-computed label, optional
  glyph, tray icon keys, the L/C/R zone, the paint recipe, and the slot key.
- **`SceneItem::BannerRow`**: paints the cells with the Rust drawer's exact
  recipes — text bias (`TEXT_Y_BIAS`), icon@12/label@29 insets, ink-centered
  icons (settings/power/search/dnd at size 14), brand-font mark, dim seps,
  tray icons right-aligned with per-icon 20px sub-regions (keys 30+ti, SNI
  activate preserved), whole-strip overflow scissor at the band box and the
  per-zone scissor state machine (transitions interleave with the cell sweep,
  zone windows clipped independently). Covered by `banner_row_paints_cells…`,
  `banner_row_overflow_clips_at_the_band`,
  `banner_row_zone_scissors_interleave_between_cells`,
  `banner_row_visible_gate_and_missing_binding_draw_nothing`.
- **`publish_banner_cells(&mut vals, w)`** (banner.rs): builds the cell list
  per frame and seeds `dash_chips_via_scene`; skipped in edit mode (DashToggle
  skipped so keys stay put). `layout_banner`'s Rust chip sweep + the viewing
  connectivity popover stand down while the flag is up (edit chrome — guides,
  grips, tray — always stays Rust). The `vals` gate in `layout_expanded` now
  also builds values when the `dashboard` surface exists (pure-Rust board +
  declarative strip).
- **`dchips` declared in shell.ron**: `BannerRow(cells: "banner_cells_v",
  x:0, y:12, h:26, visible: banner_shown, overflow: banner_overflow, zones:
  banner_zones)` — authored after the `dash_banner` Ink so it rides the same
  dispatch slot AFTER the painter: grid → band → Rust banner Ink (edit chrome /
  suppressed chip loop) → declarative chips. Both config copies in sync.
- Tests: 253/253.

## 2026-09-16 (Dashboard viewing strip band declarative — z-order slot engine)

- **Ink `pre` z-order slots (`SceneInk.pre`)**: the engine change the dashboard
  chrome needed. Items authored AFTER an Ink and BEFORE the next one no longer
  draw upfront — they attach to the FOLLOWING Ink's dispatch slot and draw
  immediately before its Rust painter. Declared chrome can now sit BETWEEN two
  Rust bodies in z-order (cards → band → banner chips). Items before the first
  Ink / after the last keep drawing in scene order. Covered by
  `items_between_inks_attach_to_the_next_ink`.
- **`Surface` per-corner radii (`tr`/`br`/`bl`, negative = concave)**: any
  non-zero corner radius emits `Cmd::RectConcave` (the band's top corners
  CONCAVE into the screen edge, bottom corners square). Covered by
  `surface_concave_corners_emit_rect_concave`.
- **Viewing strip band declared (`dband` in shell.ron)**: the opaque band that
  covers board slides scrolled under the banner now layers between the two
  Inks in the `dashboard` surface — `y: 12` (BASE_BANNER_Y), `h: 26`
  (BASE_BANNER_H), concave top corners, `value("dash_opaque")` fill (opaque
  theme bg, new SceneColor), gated on `banner_shown` (new Toggle =
  `!banner_collapsed()`, the empty-strip collapse keeps working). The Rust
  band inside `layout_dash_grid` skips itself while `dash_band_via_scene` is
  up (set BEFORE the surface consult — the `dash_grid` Ink runs the grid
  painter — and reset when the surface is unavailable), so there is never a
  double paint. Edit-mode chrome mask is unrelated and stays Rust.
- Tests: 249/249.

## 2026-09-16 (Calendar + Apps + Mirror fully declarative — Batch 1 complete)

- **Build restored first**: the tree held a half-spliced scene-values edit
  (7 compile errors — `DynColor::named`, `ColorToken::SelBg`,
  `SceneRow::default`, `ICON_APPS.into()`), finished now: `DynColor::named()`
  for 'static names, `ColorToken::SelBg` (10% accent selected-row wash, was
  only a `ui::sel_bg` free fn), `Default for SceneRow`.
- **Engine additions for the three conversions** (all follow the existing
  Grid/Rows patterns, tests per feature):
  - `SceneItem::Dots` — pagination dots below a swipe-cycled card (the HARD
    RULE, previously Rust-only `pane_dots`): `active` (Ring pane index),
    `panes`, `step`, `dy` (inset from card bottom), `when` (visibility
    toggle). One dot per pane, active = accent, others hover — parity with
    `pane_dots`.
  - `Grid` negative `w`/`h` reserve from the opposite edge (the appshortcut
    body insets `w: -24`, `h: -46`); `Image` negative `w`/`h` likewise (the
    camera frame stops above the dots via `h: -98`).
  - `Row.bottom` — anchor the row's bottom edge `y` px above the card bottom
    (the mirror Photo/Record button bar); requires declared `h`.
  - `Row`/`Column`/`Stack`/`Toggle` gained `visible: Option<String>` pane
    gates (mirror panes flip items on/off without a Rust drawer).
  - `Hit.dy` label nudge, `Hit.glyph`/`glyph_pad`/`glyph_color`/
    `glyph_color_hover` — leading-glyph icon buttons (mirror Photo/Record);
    `Header.title_color` (calendar month in accent); `GridCell.hover` (day
    ink lifts to fg, today keeps acc; plus hover keeps its AccTint surface
    via `surface_hover`); `GridCell.truncate` (app-tile name clip parity,
    Rust math `(tw − 4)/5.5` chars, computed from the tracked card rect).
- **scene_values**: `cal_cells` (42 cells + hover/today parity), `cal_month`,
  `app_cells` (8 tiles + clip), `app_count` (`n/8` header meta),
  `app_tile`/`app_tile_empty` per-frame resting tile colors (dynamic pool),
  `mirror_pane_0/1`, `mirror_pane` (Ring), `mirror_hover` (shared with the
  Rust drawer via new `Shell::mirror_dots_hover`), `mirror_rec_bg`
  (`mix(RED, hover_hl, 0.15)` while rec), `mirror_fps_rows` (active row
  `SelBg` + real-fps suffix), `mirror_rec_label`, `mirror_show_fps`.
- **draw_card_scene** now tracks `mirror_rect` + `app_shortcut_rect` for
  scene-owned cards (wheel pane-flip and the Rust remove-corner math stay
  live without the Rust drawers).
- **calendar.ron / appshortcut.ron / mirror.ron converted** (zero Ink for
  calendar + appshortcut; mirror keeps the resident fps-row key dispatch but
  draws everything declaratively): calendar = `Header` (`{cal_month}`, acc
  title) + prev/next hover-only chevron `Hit`s (30100/30101) + 7-col `Grid`
  with weekday heads and `key_base: 30000`; appshortcut = `Header` (`n/8`)
  + square `Grid` (keys 34400+i, hover-✕ remove 34416+i); mirror = pane-0
  camera `Image` + no-camera fallback `Text` + bottom `Row` of two equal-split
  glyph `Hit`s (30700/30701) + pane-1 fps `Rows` (30710..) + "Show real fps"
  `Text`+`Toggle` (30704) + `Dots` (hover-gated). Repo copies live in
  `config/zen-shell/ui/cards/`, installed to `~/.config/zen-shell/ui/cards/`.
- **Batch 1 of the card conversions is COMPLETE** — all 17 text/list/grid
  cards now draw from RON. Remaining: Batch 2 (Rows/Spark: procmon, sensors,
  fans, …), Batch 3 (Fader/Ring/Toggle), Batch 4 (special), then banner
  chrome + transient popups.
- Tests: 247/247 (was 245 pre-session; added `live_grid_card_scenes_parse`,
  `dots_render_pane_rule_and_hover_gate`, `grid_cell_truncate_clips_long_labels`;
  removed one stale duplicated `#[test]` attribute the handoff tree carried).

## 2026-09-16 (Lyrics fully declarative — karaoke word highlight)

- **Karaoke engine (`Rows.karaoke` + `Rows.karaoke_fs` + `SceneValue::Karaoke`)**:
  the word-level playhead highlight the lyrics card needed, mirroring the Rust
  drawer byte-for-byte. `SceneValue::Karaoke(Vec<KaraokeLine>)` binds per-line
  `(words, active)` state alignined by visible-index against the parallel
  `Rows` value; `Rows.karaoke: Option<String>` names that binding. When each
  row draws, its karaoke line short-circuits the normal cell machinery: the
  ACTIVE line washes accent (r4), then walks its words left-to-right with
  auto-fitted `est_w` spacing, clipping at the card edge — the playhead word
  renders white (`0xffff_ffff`) with a 1.5 px underline, the rest near-black
  (`0x1d1d_1d`); the accent wash plus subtle clamped advance recreates the
  Rust lyric fade. Inactive rows draw the joined whole line, lifting
  `hover → accent` ink with a hover surface to read like an active lyric;
  hovered rows draw a small hover background (the quiet hover tone). Plain
  (untimed) lyrics simply unbind `lyr_karaoke` and the Rows falls back to the
  regular single-column cell path — zero karaoke, zero regressions (verified
  by a dedicated test). Regions still anchor at `key_base + visible index`.
- **`Header.glyph_color: Option<ColorToken>`** — per-header note-glyph tint
  (default Fg2), so the lyrics header can paint its `\u{f001}` notes glyph
  accent like the Rust card. serialization `glyph_color: Some(acc)`.
- **Lyrics card converted with zero Ink** (`lyrics.ron`, mirroring Rust):
  `Header` (accent note glyph, title); the dynamic state chip as three
  left/right-anchored gated `Text`s — `synced`/acc, `none`/danger, and
  `{lyr_status_other}`/fg3 for fetching…/plain/— (exclusive toggles
  `lyr_status_synced`/`lyr_status_none`/`lyr_status_other_on`); the track
  caption (`{lyr_track}`); the centered fallback message (`{lyr_msg}` behind
  `lyr_empty`) for "fetching lyrics…" / "no lyrics found" / "no track
  playing"; and the karaoke `Rows` (`y: 33`, `row_h: 21`, `r: 4`,
  `hover_surface: hover`, `start: "lyr_scroll"`, `key_base: 33500`,
  single full-line col). New values: `lyr_state`/`lyr_synced`/`lyr_none`/
  `lyr_other_lbl` (+ Text var `lyr_status_other`), `lyr_track`, `lyr_msg`,
  `lyr_empty`, `lyr_scroll` (Ring), `lyr_karaoke` (Karaoke, only bound when
  `state == 2 && has_timing`, active word from `crate::lyrics::active_word`),
  and `lyr_rows` (whole-line cells). `draw_card_scene` tracks `lyrics_rect`
  and keeps `lyrics_follow` driving the ring window live.
- Repaired a scene-values splice that had broken the `recent_rows` closure
  (mismatched delimiter) before shipping.
- Tests +3 (241/241): engine karaoke (accent wash + white live word +
  underline geometry + near-black caret words + joined-line hover lift +
  region keys), the karaoke-unbound plain fallback (fg idle cell, hover lift
  + accent tint, no stray word draws), and the live `lyrics.ron` parse (fully
  declarative, accent note glyph, synced/none chips, Rows geometry/keys/
  karaoke binding/col hover). Bin clean (input.rs:512 only).

## 2026-09-16 (Snippets fully declarative — flash + SceneCol.flash)

- **Transient flash (`Rows.flash` + `SceneCol.flash`)**: `Rows` gains
  `flash: Option<String>` — a `Ring` binding whose scalar is the flashed
  row index + 1 (0 = no flash so index 0 stays representable). When the
  scalar matches a visible row, a translucent-accent surface paints that
  row, and every column that opts into `SceneCol.flash: Option<ColorToken>`
  inks its cell in that accent token (the snippets copy glyph + name turn
  accent while the dim preview stays fg3). The flash wash overrides both the
  normal hover surface and hover ink, mirroring the one-second "just snapped"
  of the Rust snippets drawer. Scene-col case `flash: Some(acc)`.
- **Snippets card converted with zero Ink** (`snippets.ron`, mirroring Rust):
  `Header` (`{snip_meta}` clip count in fg, no glyph); a `Composer` field
  (`snip_buf` buffer / "add snippet — name text to copy" placeholder,
  `snip_focus` toggle; keyed SNIPPET_INPUT_KEY); the clipboard `Rows`
  (col 0: copy glyph icon fg2/flash acc, col 1: name fg/flash acc, col 2:
  body preview fg3; `hover_surface: hover_hl`; `flash: "snip_flash"`,
  key_base SNIPPET_COPY_BASE, del_base SNIPPET_DEL_BASE, `del_pad: 20`,
  `del_dy: 4`). Values: `snip_meta`, `snip_buf`, `snip_focus`, `snip_flash`
  (Ring, `elapsed < 1 s`), `snip_rows` (glyph · name · body). Copy/clear
  keys route through the resident handler. 238/238 tests, bin clean
  (input.rs:512 only).

## 2026-09-16 (News fully declarative — scrollable Strip + list hover ink)

- **`SceneItem::Strip`** — a horizontal scrollable chip row (the missing
  Batch-1 engine gap). Chips come from the new `SceneValue::Chips(Vec<String>)`
  binding, the active index from a `Ring` scalar (`sel`), and the horizontal
  scroll offset in base px from another `Ring` (`scroll`, clamped to the
  strip's own content overflow at draw time). Each chip auto-fits its label
  (`chip_pad + 0.62·fs·chars + chip_pad`), skips offscreen, lifts
  `hover → hover_hl`, and paints the selected chip accent with an `Sfg`
  label — byte-for-byte the Rust news category strip (keys `key_base + i`).
  The wheel handler clamps `news_cat_scroll`, so stale values can't push
  chips off.
- **Three `Rows`/`SceneCol` list-ink primitives**: `SceneCol.hover` — a
  per-column color applied only while its ROW is hovered (takes precedence
  over `color`, so the news title tinting accents on hover while the source
  label stays fg3); `Rows.hover_bar` — the 2.5 × (`row_h − 6`) rounded accent
  rail at the hovered row's left edge (byte-match of the Rust drawer);
  `Rows.hairline` — a 1 px hover-gray divider under every visible row.
- **News card converted with zero Ink** (`news.ron`, mirroring Rust): `Header`
  (`\u{f1ea}` glyph, label = `{n} stories` or the active category, empty when
  inert); the `Strip` (`news_chips`/`news_cat_sel`/`news_cat_scroll`, y 30,
  key 13000); the centered fetching / no-feeds message gated by `news_empty`;
  and the headline `Rows` (40-char title · right `ICON_OPEN` glyph when the
  story is clickable · right source label on a second baseline, `y: 59`,
  30 px pitch, `key_base: 13300`, `start: "news_scroll"`, `hairline` +
  `hover_bar`). New values: `news_label`, `news_msg`, `news_chips` (Chips),
  `news_cat_sel`/`news_cat_scroll`/`news_scroll` (Ring), `news_empty`
  (Toggle), `news_rows` (title/open-glyph/source, 40/16-char truncation).
  `draw_card_scene` tracks `news_rect` so wheel scrolling (list + strip)
  stays alive. No `scene_owns_*` gating — the scene is dispatched
  whole-card.
- Tests +2 (236/236): engine (strip chips + active accent/Sfg + offscreen
  skip + row regions; row hover rail/hairline/title-tint and idle fallback)
  and the live `news.ron` parse (fully declarative, glyph, strip bindings,
  Rows geometry/keys, col hover). Bin clean (input.rs:512 only).
  `Strip` + `SceneCol.hover` pre-unlock chips/hover lists in later cards.

## 2026-09-16 (Expenses fully declarative — rows of chips, reserve+visible)

- **Expenses card converted with zero Ink** (`expenses.ron`, matching the Rust
  drawer byte-for-byte): `Header` (title + MTD total in accent mono via
  `meta_color`); a **`Row` of six equal-split `Hit` chips** (1/5/10/20/50/100,
  keys 31800..31805, `surface: hover` lifting to `hover_hl`); the composer row
  — a `Composer` (`pad: 0` so the row pad supplies the inset, `focus:
  exp_focus`, key 31810 — Enter commits via the resident key handler) plus a
  fixed-70 `Hit` "clear mo" that flips `fg2 → danger` on hover (key 31811); and
  the top-3 per-category bars (`caption + Bar + right mono amount`, each group
  gated by an `exp_has_n` Toggle) with the "nothing logged this month" caption
  gated by `exp_empty`. Bare values in `scene_values`: `exp_meta`,
  `exp_buf`, `exp_focus`, `exp_empty`, `exp_cat_i`/`exp_val_i`/`exp_bar_i`
  (ratio vs the top category), `exp_has_i`. No `scene_owns_*` gating needed —
  a scene without `Ink` is dispatched whole-card, so the Rust drawer never
  runs.
- **Four tiny primitives landed for it**: `Header.meta_color` (meta ink that
  defaults to `fg3`, no-op elsewhere); `Hit.color_hover` (label ink swapped
  while hovered — pre-unblocks the news title hover); `Text.visible` and
  `Bar.visible` (bound `Toggle` gates — state-swapped labels/bars);
  `Bar.reserve` (`w: 0` bars span `card_w − reserve` instead of the full width,
  so the amount column at the right edge stays clear).
- Tests +2 (234/234): live `expenses.ron` is fully declarative (no Ink,
  header meta binding/color, chip keys + amounts, composer pad/key/focus,
  clear hit geometry + danger hover, reserve/visible on the three bars);
  engine test for reserve geometry, accent meta ink, the visible gates, and
  the hover color swap. Bin clean (input.rs:512 only).

## 2026-09-16 (Quote body declarative — multiline wrap + visible gate)

- **`SceneItem::TextWrap`** — a multi-line text block that wraps at 34
  chars/line (≤ 136 chars total, matching the Rust Quote drawer exactly), with
  every line horizontally centered, vertically centered inside a box bounded
  by `top`/`bottom` card insets (base px, scaled). An optional `caption` line
  (dimmer, smaller) trails the block — the "— author" line; both `text` and
  `caption` interpolate `{var}`s and skip drawing when empty after
  substitution. `visible: Option<String>` (bound `Toggle`) gates the whole
  block on/off without reserving space, swapping the body in against
  the Rust fallback messages (the artist swaps the state branch at runtime).
- **`scene_owns_rows` also detects `TextWrap`** (so the Rust quote drawer
  skips its own body wrap when the scene owns it, while keeping the two
  fallback message states and the title/meta/pills as resident Ink).
- **Quote split** (`quote.ron`): the wrapped quote text + author caption is a
  `TextWrap` item (visible when `quote_has`, top 28 clears the header,
  bottom 36 clears the pills); title/meta + the two pills (refresh
  always, save conditional) stay the `quote` Ink. `draw_quote_card` gates its
  body with `!self.scene_owns_rows`; the flash decrement stays inside the
  Ink. New values: `quote_text` (the raw text), `quote_author`, `quote_has`
  (Toggle of `state == 2 && !text.is_empty()`).
- Tests +2 (232/232): live `quote.ron` declares the `TextWrap` body with Ink
  (visible/top/bottom), wrap-at-34 lines + author caption drawn, and
  `visible=false` suppresses all output. Bin clean (input.rs:512 only).
  `TextWrap` pre-unblocks **lyrics** (multiline, partial) and simplifies
  future conditional body blocks.

## 2026-09-16 (Countdown list body declarative — chip cells + truncation)

- **`SceneCol` gained three list primitives** (RON-only serialization, no
  `SceneRow` churn): `truncate: Option<f32>` (cut a cell to fit
  `card_w − 2·pad − truncate` px at 0.62·fs px/glyph, ≥ 8 chars — the Rust
  drawers' `avail` reservation so names stop before the right-side chip);
  `edge: Option<f32>` (right-edge anchor at `card_w − edge` base px, survives
  card resize); and **`pill: Option<PillSpec>`** — a chip cell: rounded rect
  auto-fit to its text (0.62·fs px + 2·`pad`), fixed `height`/`radius`/`top`,
  fill derived from the cell's text color (Acc/AccTint → accent tint,
  anything else → hover) or overridden by `bg`. The chip texts' per-row ink
  comes from `col_colors` as usual. `del_anchor: bool` moves a row's revealed
  ✕ (and its full-row region) left of that col's box — the chip-relative
  deletes in the countdown drawer.
- **Countdown split** (`countdown.ron`): the event list is now a `Rows` item
  (`countdown_rows`: label `truncate: 120` · date caption · right `pill`
  chip, `y: 50` under the top composer, 24 px pitch, 12 pad, `del_base:
  COUNTDOWN_DEL_BASE`/`del_pad: 20`/`del_dy: 4` anchored left of the chip via
  `del_anchor`, key-less reveal-only-on-✕ like the Rust drawer). Title/meta +
  the add-event composer row stay the `countdown` Ink; the Rust list loop is
  gated by `scene_owns_rows`. New `countdown_rows` value builds chip
  text/tokens (`today!`/`passed`/`Nd`) + per-cell colors (Acc/Fg3/Warn/Fg2).
- Tests +3 (230/230): pill chip auto-size + derived fill (Warn→hover, Acc→
  tint) + centered glyph + `truncate` cutting, `del_anchor` ✕/region left of
  the chip, live `countdown.ron` declares the Rows list (truncate/pill/edge/
  del anchors + del keys) while keeping its Ink. Bin clean (input.rs:512
  only). The pill/edge/truncate primitives pre-unblock **news** (chips +
  two-line headlines), **expenses** (amount chips), **quote** (action pills).

## 2026-09-16 (Notes list body declarative — two-baseline rows + composer gate)

- **`SceneCol.dy: Option<f32>`** (base px, serde default → falls back to the
  Rows `row_dy`): per-column baseline offset lets one row carry a title +
  dim caption line beneath it. Pre-unblocks the news/lyrics/countdown cards;
  `SceneCol` and `Rows` are RON-only lists, so adding fields is churn-free.
- **`Rows.visible: Option<String>`** (`Toggle` value; when some and bound to
  `false` the whole list draws nothing — the arm skips the row loop instead
  of `return`, so later items in the same scene such as a Composer still
  draw). This models the Notes composer swap: the list hides while the shared
  title/body composer is open, so it can never draw over the input.
- **Notes split** (`notes.ron`): the note list is now a `Rows` item
  (`notes_rows`: bullet glyph tint col · title 9.5 · body caption 8 fg2 at
  `dy: Some(12)`, `y: 32` under the header, 24 px pitch, 12 pad, `row_dy: 0`,
  `start: Some("notes_scroll")` for the wheel window, `visible:
  Some("notes_composing")`). `draw_notes_card` keeps header/`+`/empty-hint
  chrome plus the full composer as Ink, and gates its own row loop with
  `scene_owns_rows`. New values: `notes_rows`, `notes_composing` (Toggle of
  `notes_input.is_some()`), `notes_scroll` (Ring).
- Tests +3 (227/227): `SceneCol.dy` baseline math, `Rows.visible` hiding the
  list while later items keep drawing, live `notes.ron` declares the Rows
  list (dy/title/body cols, scroll + composer toggle) while keeping its Ink.

## 2026-09-16 (Alarms list body declarative + key-less ✕ reveals)

- **`Rows` ✕ reveal no longer needs row keys**: the del-region guard dropped
  its `key != 0` clause — cards whose rows carry no clickable row still get
  the hover ✕ (revealed only while the ✕ itself is hovered, exactly like the
  old Rust drawer). New `del_dy` (base px glyph offset in the row; To-Do stays
  1.0, alarms 4.0) joins `del_pad`/`del_size`/`del_base`.
- **Alarms split** (`alarms.ron`): the alarm list is now a `Rows` item
  (`alarm_rows`: mono accent time + fg label columns, `y: 50` under the add
  row, 24 px pitch, 12 pad, `row_dy: 3`, `del_base: ALARM_DEL_BASE`,
  `del_pad: 26`/`del_dy: 4` — pixel-parity with the Rust drawer), gated by the
  existing `scene_owns_rows`; `draw_alarms_card` keeps title/`{n} set`
  meta/count + the flat add-input row as Ink chrome. Same del keys/geometry;
  no row-level regions (parity: the alarms list was click-free). New
  `alarm_rows` value in `scene_values()`.
- Tests +2 (224/224): key-less ✕ reveal (`del_pad`/`del_dy` anchored, no row
  hits), live `alarms.ron` declares the `Rows` list (del base/margins, mono
  accent time col) while keeping its Ink. Bin clean (input.rs:512 only).

## 2026-09-16 (Rows checkbox cells + hover-✕ delete + To-Do checklist body declarative)

The To-Do checklist body joins the declarative bed — first card to split its
Ink three ways (title/meta Ink + `Rows` body + `Composer` strip).

- **`Rows` gains a live checkbox column** (`check: Some("name")` +
  `check_x`): every visible row draws its leading 11² checkbox from the
  parallel `Checks(Vec<bool>)` value (indexed like the full `Rows` list, so
  windowed rows stay consistent). Checked = acc-tint well + `ICON_CHECK` in
  accent, unchecked = stroke-only outline with the hover-aware grey (exactly
  the Rust To-Do box). New `SceneValue::Checks` + `SceneValues::checks()`.
- **`Rows` gains hover-✕ delete** (`del_base`, `del_pad`, `del_size`): the
  hovered row reveals a right-anchored `ICON_CLOSE` at `card_w - del_pad`
  (fg, danger-red while its own del region is hovered — `ColorToken::Danger`)
  and registers the 22² corner region (`del_base + visible index`) ONLY while
  hovered, so it wins the row's corner exactly like the Rust drawers. `✕`
  shows while the row OR its ✕ is hovered.
- **To-Do card split** — the checklist is now a `Rows` item in `todo.ron`
  (`key_base: TODO_KEY_TOGGLE` remaps row keys to the visible window exactly
  as the Rust loop did; `del_base: TODO_KEY_DELETE`; `start: "todo_scroll"`
  keeps wheel-scroll alive; `check: "todo_checks"`) gated by the new
  `scene_owns_rows` (the `scene_owns_header`/`scene_owns_composer` pattern),
  so `draw_todo_card` draws only its title/count/`+N more`/empty-state.
  Same keys/geometry end-to-end: checkbox 14,1+; label 32; del at right-20.
- Tests +3 (222/222): checked/unchecked checkbox cells, ✕ reveal + conditional
  corner region (+ danger-Red on the ✕ itself), live `todo.ron` carries
  `Rows` (toggle/delete/check/scroll bounds) with its Ink + Composer. Bin
  clean (input.rs:512 only).

## 2026-09-16 (Composer scene item + To-Do card body split)

The first interactive card-body piece goes declarative: the bottom input strip
of the note cards is now `SceneItem::Composer`.

- **`SceneItem::Composer`** — the shared bottom-input chrome: rounded field
  (focused = stronger hover blend + 1.4px accent outline), `{var}` buffer with
  caret (width from char count, exactly the Rust math), placeholder while
  empty+unfocused, the "keyboard" typing hint on the right, and an optional ⊕
  add button that hides while focused. Binds `text` (`{var}`-interpolated) and
  `focus` (a `Toggle` name); **`bottom: true` anchors `y` from the card's
  bottom edge** (base px) so the strip stays pinned as the card resizes — the
  first bottom-anchored scene item. Registers the field (focus) + ⊕ regions,
  both with optional `SceneAction`s.
- **To-Do card split** (`todo.ron`): the checklist body stays the `todo` Ink
  (Rust), the composer is now a `Composer` item — `scene_owns_composer` (the
  `scene_owns_header` pattern) makes `draw_todo_card` skip its own strip
  (single draw + single regions, same keys 78/79). New bindings
  `todo_focus`/`todo_buf` published in `scene_values()`. Notes/Snippets (same
  strip shape) can adopt `Composer` next.
- Tests +4 (219/219): idle/focused composer rendering, bottom-anchored pins at
  two card heights, RON roundtrip of defaults, live `todo.ron` carries the
  Composer (keys 78/79) while keeping its Ink. Bin clean (input.rs:512 only).

## 2026-09-16 (dashboard surface LIVE in the viewing state)

`dashboard` was the last un-wired surface. Its two bodies now route through the
exact Rust painters via a chrome/body split — the board (`Ink("dash_grid")`,
extracted `layout_dash_grid`: dot lattice, empty hint, drag ghost, cards at
their packed placement with the fluid glide + `card_anim`/`edit_settling`/
`DASH_CARD_KEY` region side effects, tray-drag ghost, then the viewing strip
band) and the top banner strip (`Ink("dash_banner")` → `layout_banner`).

- **`DashDrawCtx`** (`panels/mod.rs`): a one-frame, borrowed context
  (`Layout` + drag cursor + scene values) handed to `draw_shell_surface` /
  `dispatch_shell_inks` / the new `dash` param; the dashboard never stores
  layout state (its `Layout` is pre-computed per frame anyway).
- **Consult gate** in `layout_expanded`: viewing state only (`!editing &&
  !sys_menu_open`). It zeroes the edit-only caches first (same as the old
  non-edit branch), calls the surface, draws `layout_dash_scrollbars` inline
  (V+H rails + thumbs, drag hit zones 500/501 still register), then returns.
  Edit-mode chrome (trays, ctrl rows, dot lattice, resize/✕ grips) and the
  system-card ⋮ menu stay Rust — frame-dynamic geometry, never dropped.
  The strip mask moved INTO `layout_dash_grid` (viewing case only) so the
  fallback path doesn't double-draw it.
- **shell.ron**: `dashboard` surface now declares `Ink("dash_grid")` +
  `Ink("dash_banner")`. The live-file test asserts that exact pairing.
- 215/215 tests, bin clean (only the pre-existing `input.rs:512` warning).

## 2026-09-16 (dashboard prerequisites: scene `Scissor` + banner tokens)

`dashboard` remains declared-but-unwired (the last surface); this pass landed
the two engine prerequisites the banner needs before its chrome can leave Rust.

- **`SceneItem::Scissor` + `SceneItem::ScissorEnd`** — the scene engine can now
  open/close rectangular clip regions (the strip-zone / scroll-cut primitive
  missing from scenes). `w: 0` / `h: 0` axes span the parent box; items emit
  `Cmd::Scissor`/`Cmd::ScissorEnd`, feeding the existing live scissor stack in
  `src/app/mod.rs` (the banner's per-zone windows already use it). Inert for hit
  testing; component inlining offsets Scissor like other positioned items.
- **Banner live tokens in `scene_values()`** — the named `banner_guide` /
  `banner_guide_dim` drop-guide inks (active = mix(hover, acc, .35) under a
  lifted token, idle = mix(hover, fg, .10)) plus the state booleans
  `banner_editing`, `banner_drag`, `banner_zones` (zoned clip active),
  `banner_collapsed`, `banner_overflow` (whole-strip scroll mode). A future
  declarative banner binds these via `value("banner_guide")` /
  `Toggle(name: "banner_*")` while Rust keeps computing the guide + zone
  geometry.
- **Tests**: scissor emits balanced clip commands at scaled coords; zero-dim
  scissor spans the parent box; scissor RON roundtrips; banner tokens resolve
  through the normal `value(...)`/`Toggle(name)` paths with the exact
  `scene_values()` inks. **215/215 pass**, bin build clean (only the
  pre-existing `input.rs:512` warning).
- `layout_expanded` still runs its own Rust banner/grid unchanged — no visual
  or behavior change this pass (the split happens once the banner chrome leaves
  Rust, per the build order below).

## 2026-09-16 (second pass: `lock` + `wallpaper` surfaces LIVE)

- **`lock` surface LIVE** (at the 1920×1080 reference height): `layout_lock`
  split into hero chrome + `layout_lock_body` (password field + auth error/
  caps + hold-power buttons + hint) routed through `Ink("lock_screen")`. The
  declared chrome — fullscreen dim (`value("lock_dim")`, the 0x00000059
  backdrop), hero clock (`{clock}`), date (`{lock_date}`, zero-padded `%d` day
  matching the Rust format), avatar disc (`value("lock_avatar")` =
  mix(hover, acc, .35)), `{user_init}` first-letter initial + `{username}` —
  is center-anchored to the fullscreen surface via new `Surface center_x`/
  `center_y` fields (mirroring `Text`; offsets are base px from the box mid).
  The surface is consulted only when the monitor height is ~1080 (`h ± 24`);
  any other size runs the built-in Rust layout unchanged. The body owns all
  its hit regions (keys 1–4), the scene owns none.
- **`wallpaper` surface LIVE**: `layout_wallpaper` split into header chrome
  (back button `Hit` key 1 with a hover-only fill the Rust header lacks,
  `\u{f053}` glyph + "Backgrounds" title, byte-for-byte positions) +
  `layout_wallpaper_body` (empty state + thumbnail grid + scrollbar) routed
  through `Ink("wallpaper_picker")`. `layout_wallpaper` now takes `h` to
  consult/route the surface.
- **`dispatch_shell_inks`** gained `lock_screen` → `layout_lock_body` and
  `wallpaper_picker` → `layout_wallpaper_body` arms; only `dashboard` remains
  un-wired (banner needs live-scissor/drag tokens).
- **`scene_values()`**: new `lock_dim` + `lock_avatar` dynamic colors
  (`SceneColor::Raw` — derived blends tokens can't express), `user_init` and
  `lock_date` text values (distinct from the shared `date` `%e`-formatted
  value cards use).
- **Tests**: new `surface_center_anchors_to_box_center` (0-dim spans the box +
  center offsets + raw dynamic color); `shell_ron_live_file_parses_and_has_surfaces`
  now asserts all four live surfaces carry their body Ink. **210/210 pass**
  (was 209), bin build clean (only the pre-existing input.rs:512 warning).
- Declared-lite deltas (noted, accepted): lock surface tops out at the 1080p
  reference (non-1080 heights use Rust); wallpaper back button gained a
  hover-only fill; lock wallpaper chrome uses static tokens for the hint-free
  hero (no dynamic hover tint on the hero clock).

## 2026-09-16 (dynamic tokens + `notif` surface LIVE)

- **Dynamic color tokens** (`ColorToken::Value`): new Copy `DynColor` in
  `src/ui.rs` (fixed 24-byte buffer, custom serde so RON stays natural —
  `value("dnd_chip")`), new `ColorTokenExt::resolve_with(pal, vals)` /
  `hover_variant_with(pal, vals)` in `scene.rs`. Every draw-path resolver site
  now threads the per-frame `SceneValues` (`surface`/`surface_hover`/`color`
  + `draw_text` across all 19 call sites), so a `value(name)` resolves
  dynamically each frame while `Token` colors still live-resolve against the
  theme (theme switches still apply).
- **`SceneValues` color pool**: `SceneColor::Token(ColorToken)` (re-derivable
  against the live theme at draw time) or `SceneColor::Raw(u32)` (computed
  threshold ladders / blends `mix` can't express), via `insert_color`/
  `color`. MISFIRE rule matches typed values: unknown name or wrong kind →
  `pal.fg` (never panics, never black).
- **`notif` surface is LIVE**: split `layout_notifs` into chrome (title +
  Clear chip, declarative `Hit` key 2) + `layout_notifs_body` (the card list
  + empty state, routed through `Ink("notif_list")`); `layout_notifs`
  consults the surface and falls back to the full Rust layout when it's not
  declared / not hittable. DND chip (key 3) is now fully state-driven
  declaratively: fill = accent when DND is on (hover whitens via
  `mix(acc, fg, 0.15)`), quiet hover surface when off, label `sfg` on accent
  / `fg` otherwise — all bound through `scene_values()` dynamic colors
  (`dnd_chip` / `dnd_chip_hover` / `dnd_chip_fg`, the latter a live
  `SceneColor::Token`). Chips rest `raised`/lift `raised_hl` on hover; labels
  left-anchored at the exact Rust insets (Clear: w−196 chip, 230 label;
  DND: w−104 chip, 324 label — the notif pane is a fixed 420-wide surface).
- **Tests**: +5 in `scene.rs` (value-token RON roundtrip, token/raw/unknown
  resolution, hover invariance). 209/209 pass, bin build clean (only the
  pre-existing input.rs:512 warning).
- Declared-lite deltas (noted, accepted): chips omit the hairline outline;
  the DND chip background stays flat (no hover/drag tint beyond the guarded
  `surface_hover` blend).

## 2026-09-15 (whole-shell declarative: `shell.ron` + settings live-cut)

- **Whole-shell declarative engine** (`scene.rs`): `SurfaceScene { name, items }`,
  `ShellScene { components: HashMap<String, Vec<SceneItem>>, surfaces }`,
  `SceneItem::Comp { name, x, y }` (template instantiation — inlined at load
  time, translated by `(x, y)` base px; components may nest), and
  `ShellSceneCache` (mtime hot-reload, mirrors `SceneCache`). A declared
  surface replaces its Rust draw path when wired; no file → every surface
  still runs its built-in Rust layout unchanged.
- **`vars::shell_scene_path()`** → `~/.config/zen-shell/shell.ron`.
- **`draw_shell_surface(name, v, w, h, pal, vals) -> bool`** (panels/mod.rs):
  draws a declared shell surface via `CardScene::draw`, registers hits, and
  routes `Ink{name}` (now with `{var}` interpolation) to
  `dispatch_shell_inks` — same plumbing as card scenes, elevated to surfaces.
- **`Rows` engine additions**: `pitch` (stride override — 46 px nav over a
  40 px body), `w` (bounded row box), `row_dy` (cell baseline), `r` (row
  corner radius). **`Hit`** gained `hover_only` (surface fill only while
  hovered — the inline-appear controls). `NAV_*` settings keys made
  `pub(crate)` so `scene_values()` can reference them.
- **`settings` surface is LIVE** in `shell.ron`: Settings title + hover-✕ close
  + 1 px sidebar rail + 180 px nav `Rows` (per-row `surface`/`col_colors` for
  selected/hover from `scene_values` `settings_nav`) + `Ink(name: "{settings_ink}")`
  body. The declared `dashboard`/`notif`/`lock`/`wallpaper` surfaces are
  present but not yet wired into their mode layouts.
- New tests: `shell_scene_parses_and_expands_components` (RON map + Comp
  offset + nesting), `shell_ron_live_file_parses_and_has_surfaces` (live
  canonical file parse — skips when no file in the test env).
- 204/204 tests pass, bin build clean (only input.rs:512 pre-existing).

## 2026-09-15 (scene_values built once per frame)

- `DashCard::draw` now takes a `&SceneValues` argument instead of building the
  whole value pool itself — the full map was being re-formatted once PER CARD
  PER FRAME in `layout_expanded` (one `scene_values()` call per drawn card).
- `layout_expanded` (`src/shell/panels/dashboard.rs`) builds the value pool
  ONCE before the card loop and threads `&vals` into every card draw (packed
  grid + dragged-from-tray card), so N cards cost 1× pool build instead of N×.
- Dirty gate: when no card actually on screen has a `.ron` scene (checked via
  `scene_cache.get()` on the packed cards + the edit drag target), the pool
  build is replaced by an empty `SceneValues` — pure-Rust-card boards skip the
  formatting work entirely on frames where nothing scene-y draws.
- Rust drawers are unaffected (they never consumed the pool); 202/202 tests
  pass, bin build clean (only pre-existing input.rs:512 warning).

## 2026-09-15 (Batch 1: `recent.ron` lands declarative)

- **`recent` converted to a fully-declarative scene** (`ui/cards/recent.ron`):
  `Header` (meta `{recent_files_n}` "N files") + centered empty-state +
  `recent_rows` (name · open-glyph) with `start: recent_scroll` for the
  wheel-scroll window and `key_base: 14300` for the open-file click plumbing.
- **Hover tint via data**: the recent drawer tints the hovered row's name +
  open glyph accent; `scene_values()` now computes `col_colors` per row from
  `hover_key` each frame so the scene matches it exactly.
- **Live data without a Rust body**: the drawer's 30 s stale-`read_recent_files`
  refresh moved into `refresh_recent_if_stale()`; `draw_card_scene` calls it
  for `recent` and tracks `recent_rect` so wheel scrolling + refresh survive
  the skipped drawer.
- **202/202 tests pass** (the committed batch-1 scene parse test now also
  covers `recent.ron`); bin build clean (only pre-existing input.rs:512).

## 2026-09-15 (Batch 1 first tranche: declarative list cards)

- **`Rows` engine extensions** (`src/scene.rs`) — the last four pieces needed
  to convert scrollable, stateful list cards to declarative RON:
  - `SceneRow.surface: Option<ColorToken>` — resting row tint (active /
    connected well) drawn behind the row, taking precedence over the hover
    lift exactly like the Rust list drawers.
  - `SceneRow.col_colors: Vec<Option<ColorToken>>` — per-cell color override
    that beats the row's own `color`, so a status column stays colored
    (chg% green/red, connected check) even on dimmed/active rows.
  - `Rows.start: Option<String>` — scalar-bound first-visible row. The list
    skips to the window and keys rebase to it (`key_base + visible index`),
    so wheel-scrolled cards migrate cleanly.
  - `SceneCol.icon: bool` — renders a cell in the icon font (glyph columns).
- **First fully-declarative cards** (`ui/cards/*.ron` replace the `Ink`
  placeholders; no Rust drawer runs any more):
  - `bluetooth.ron` — Header + adapter chip + `bt_rows` (icon·name·connected
    ✓/unpaired) with connected-row accent tint + `start: bt_scroll`.
  - `ticker.ron` — Header (meta "crypto") + `ticker_rows` (sym·price·chg%
    colored by sign) + `start: ticker_scroll`.
  - `currency.ron` — Header + re-base chip + `currency_rows` (active row
    accent-tinted) + `start: currency_scroll`.
  - `sshvpn.ron` — Header + `sshvpn_rows` (globe/shield glyph per line).
- **Scene-scroll plumbing**: `draw_card_scene` (`src/shell/panels/mod.rs`)
  sets the `bt_rect`/`ticker_rect`/`currency_rect` wheel-scroll targets for
  scenes without an `Ink` body (the skipped Rust drawer used to set them).
- **`scene_values()`** gains `bt_rows`, `bt_scroll`, `bt_status`,
  `bt_chip_on`/`bt_chip_off`, `ticker_rows`, `ticker_scroll`, `ticker_status`,
  `currency_rows`, `currency_scroll`, `currency_chip`, `sshvpn_rows`,
  `sshvpn_status`.
- **202/202 tests pass** (new: resting-surface-over-hover + icon cols, scroll
  window + key rebase, per-cell `col_colors`, committed batch-1 scenes parse);
  bin build clean (only pre-existing input.rs:512 warning).

## 2026-09-15 (scene Phase B: layout containers)

- **Layout containers** (`SceneItem::Row`, `Column`, `Stack`) bring
  Quickshell-style layout to the RON scene engine (`src/scene.rs`):
  - `Row(x, y, w, h, pad, spacing, valign, items)` — children laid out
    left-to-right; `want_w() == 0` items **fill** the remaining width
    equally; `valign: middle` centres short children vertically.
  - `Column(x, y, w, h, pad, spacing, halign, items)` — top-to-bottom
    layout; `want_h() == 0` children fill the remaining height; `halign:
    center` centres narrow children horizontally.
  - `Stack(x, y, w, h, pad, items)` — overlay: every child is drawn in
    the full inner box in declaration order; zero-dimension children
    (via `set_dims`) span the box for full-bleed surfaces/Inks.
  - All three treat `w` or `h` of **0** as "fill parent box" — at the
    top level this is the card; inside a container it's the slot.
- **Zero-dimension fill rule** applied uniformly to all non-`Text` leaf
  arms (`Hit`, `Surface`, `Divider`, `Bar`, `Spark`, `Fader`, `Toggle`,
  `Ink`, `Row`, `Column`, `Stack`): when declared `w` or `h` is 0 the
  item draws at the parent box's size rather than zero. This keeps
  committed top-level scenes untouched (no committed item uses `w: 0`
  except `Text`, whose `w: 0` is used only for alignment and is
  excluded) while letting containers resolve slots cleanly. `Stack`
  additionally `set_dims()`-clones children whose axes are zero so
  `Surface(x:0,y:0,w:0,h:0)` reliably spans the box.
- **`SceneItem::set_dims()`** method fills the zero axes on any
  variantable that carries `w`/`h`, enabling `Stack`'s clone-and-fill
  without modifying the enum's serde shape.
- **draw refactor** — the top-level `CardScene::draw` iterates items
  calling the new `fn draw_piece(...)` which resolves one item (or a
  whole nested box) inside a given parent `(x, y, card_w, card_h)`.
  Leaf arms were tightened so their hit regions use the parent box when
  the declared dimension is 0; inner-loop `continue`s in arms were
  converted to `return` (they no longer target the outer for-loop).
- **Tests 190 → 198**: row horizontal layout + spacing, row fill slot
  sharing, row vertical centering, column vertical stacking + halign,
  column fill slot sharing, stack full-bleed surface + ring overlay,
  nested container child scaling and relative child offsets, and
  container RON roundtrip. **198/198 pass**, bin build clean.

## 2026-09-15 (scene Phase 2a: data-driven primitives)

- **Typed scene values** — `scene_values()` now returns `SceneValue`/
  `SceneValues` (`src/scene.rs`) instead of a flat string map; all 22
  header/meta keys became `SceneValue::Text` (interpolation unchanged), and
  typed values were added for body primitives:
  - **Ring**: `battery_ring`, `mem_pct_f` · **Fader**: `volume_fader`,
    `brightness_fader`, `mic_fader` · **Toggle**: `wifi_on`, `bt_on`,
    `power_save_on`, `fx_blur_on`, `fx_shadow_on`, `fx_opacity_on` ·
    **Spark**: `cpu_spark`, `gpu_spark`, `lat_spark` (raw), `net_up_spark`,
    `net_down_spark`, `disk_r_spark`, `disk_w_spark` (peak-normalized via
    new `peak_of_pairs`) · **Rows**: `jr_rows`, `conn_rows`,
    `top_procs_rows`, `fans_rows`.
- **Six data-driven scene primitives** (`SceneItem::Rows / Spark / Ring /
  Fader / Toggle / TabRow`):
  - `Rows(name, y, row_h, cols[], pad, key_base, hover_surface, max_rows)`
    — repeats a row per `SceneRow { cols, key, action, color }`; rows past
    the card edge are clipped, per-row `key` wins over `key_base + i`, and
    `hover_surface` lifts the hovered row.
  - `Spark(name, x, y, w, h, color, max, kind: line|column, thickness)` —
    self-normalizes against the series peak when `max` is 0.
  - `Ring(name, cx, cy, center_x, center_y, radius, max=100, beads=36,
    dot=3, fill, track)` — battery-style bead ring (local `bead_ring`
    helper, since `cards/shared.rs::ring_beads` is unreachable through the
    private module chain).
  - `Fader(name, x, y, w, h, fill, track, key, action)` — slim slider with
    round handle that registers a hit region when `key != 0`.
  - `Toggle(name, x, y, w, h, key, action)` — accent-when-on switch.
  - `TabRow(name, sel, y, pad, h, key_base, active, idle)` — tab labels
    batched per-data-row, `sel` bound via `SceneValues::scalar()`, keys
    `key_base + i`, AccTint surface on the selected tab.
- **Accessors are type-safe**: `vals.rows/spark/scalar/toggle/fader` return
  `None` on a missing name **or** a type mismatch, so a misbound RON item
  draws nothing instead of panicking. `substitute()` only interpolates
  `SceneValue::Text`.
- **Col formats on Rows**: `SceneCol { x (req), w, right, center, font,
  size (≤0 → 9.5), weight (default Regular), color }` — cell colors fall
  back row → column → Fg.
- **Tests 179 → 190** — new coverage: RON parse of the new variants,
  Rows cell draw/edit clip + explicit keys + hover surface, Spark line &
  column shapes + auto-normalize, Ring bead count at 50%/overshoot, Fader
  fill/head/hit + no-key no-hit, Toggle off/on colors + key, TabRow tabs +
  selected tint, and type-mismatched names draw nothing. **190/190 pass**,
  bin build clean.

## 2026-09-14 (feature: Compositor, Countdown, Alarm, Snippets, Expenses cards)

- **Compositor / Effects card** (`DashCard::Compositor`, id `compositor`,
  tray "Effects") — live toggle tiles for **blur / shadow / opacity**
  wired to the existing `blur_main toggle` / `shadow_main toggle` /
  `opacity_main toggle` scripts via the `DashCmd` pipeline, plus a
  **screenshot row** (`shot_main` full screen / area select).
  - **Live state**: `dash_status` now also prints `blur 0|1`, `shadow
    0|1`, `opacity 0|1` (read from the scripts' state files
    `states/hypr_blur_$s`, `states/shadow_enabled_$s`,
    `states/opacity_active_$s`), so the tiles show real on/off tints and
    refresh on the existing status poll.
- **Countdown card** (`DashCard::Countdown`, id `countdown`, tray
  "Countdown") — days-until events: `label · date · Nd` rows sorted
  soonest-first, a `today!` accent chip on the day itself, past events
  **kept but grayed**. Add via the To-Do-style **inline composer**
  (`name YYYY-MM-DD`, libc-only date math — no chrono). Persisted to
  `$XDG_DATA_HOME/zen-shell/countdown.txt` (`label\tdate`).
- **Alarm / reminder card** (`DashCard::Alarm`, id `alarm`, tray
  "Alarm") — `HH:MM` + label rows with an inline `HH:MM label`
  composer and a repeat-arms row (`daily` / `weekdays` / `once`).
  **Fires the real notification popup** at zero seconds past the
  target: the always-running 60 s clock tick checks due alarms and
  pushes them through the notification channel; `once` arms
  self-remove after firing, repeating arms re-arm tomorrow. Persisted
  to `$XDG_DATA_HOME/zen-shell/alarms.txt` (`HH:MM\tlabel\tarms`).
- **Snippets card** (`DashCard::Snippets`, id `snippets`, tray
  "Snippets") — saved text clips; clicking a row **copies the body to
  the clipboard** (new `Clipboard::copy_text` alongside `set_entry`)
  and flashes a `copied` confirmation on the row. Add/remove via the
  same inline composer pattern. Persisted to
  `$XDG_DATA_HOME/zen-shell/snippets.txt` (`name\tbody`).
- **Expense tracker card** (`DashCard::Expenses`, id `expenses`, tray
  "Expenses") — month-to-date spend: quick-add chips (`1 5 10 20`) plus
  a `amount [label]` composer (comma decimals accepted), MTD total in
  the header, and per-label **ranked bars** for the current month (old
  months excluded from the ranking). Persisted to
  `$XDG_DATA_HOME/zen-shell/expenses.txt` (`cents\tlabel\tdate`).
- **Day-math fix**: `countdown_days` mixed a 1-based day-of-year
  against the 0-based `tm_yday`, making same-year diffs come out +1;
  today's day-number is now 1-based to match. `tests 128 → 139`
  (countdown day-math/sort/draw, alarm arm-expansion + due-fire + card
  draw, snippet copy/register, expense parse/summary/draw, compositor
  status parsing + chip registration).

## 2026-09-14 (feature: Water tracker + Moon phase cards)

- **Water tracker card** (`DashCard::Water`, id `water`, tray "Water") —
  daily hydration log: a bead-ring gauge (the CPU card's `ring_beads`
  style; INFO-blue, OK-green with a ✓ when the goal is met) around a
  glasses + "of N glasses" center readout, `−` undo and `+ glass` buttons
  below. One glass = 250 ml; the header meta shows `X.X / Y.Y L`. The
  count **auto-resets at midnight** (checked on load and on every
  interaction); the **daily goal adjusts by horizontal swipe** over the
  card (1–30 glasses, same swipe chain as the battery pane flips).
  Persisted to `$XDG_DATA_HOME/zen-shell/water.txt` (`goal` / `day` /
  `count` lines). Keys `WATER_INC_KEY = 31_300`, `WATER_DEC_KEY = 31_301`.
  `tests 123 → 128`.
- **Moon phase card** (`DashCard::Moon`, id `moon`, tray "Moon") — the
  current phase as a **vector shaded disc** (no bitmaps: the lit portion
  is painted as scanline slivers whose terminator ellipse collapses at the
  quarters — waxing lights right, waning lights left), next to the phase
  name, illumination % and days-until-full ("full tonight" within a day).
  Pure date math from a known new-moon epoch (synodic 29.53059 d) — no
  sources, no I/O. Phase math is tested against real 2024 new/full-moon
  epochs; the disc is exercised across the whole cycle (no panics,
  slivers drawn).

## 2026-09-14 (feature: World clock card)

- **World clock card** (`DashCard::WorldClock`, id `worldclock`, tray name
  "World clock") — pinned cities with their local time. One row per zone:
  sun/moon glyph (local hour 6–17 = day), city, right-aligned mono HH:MM
  and a `+H:MM` offset-vs-local chip (hidden for the local zone). Hover ✕
  removes a row; a "+ add city" row opens an **inline zone.tab search**
  (type ≥ 2 chars → up to 6 city/tz matches, click to pin; Esc closes;
  dedup by tz, max 16 rows). `tests 120 → 123` (offset/hour math, pin
  dedup + persistence, row + search-overlay draw).
  - **Data flow**: `HH:MM`/dayline/night are computed at draw time from a
    cached UTC-offset (pure libc, no per-frame subprocesses). Offsets +
    abbreviations refresh via ONE worker thread probing every pinned zone
    with `TZ=<tz> date +%z %Z` (results through a calloop channel), armed
    from the 3 s services timer only while the dashboard is open, at most
    every 5 min (DST).
  - **Persistence**: `$XDG_DATA_HOME/zen-shell/zones.txt` (`city\ttz`
    lines); first launch seeds the classic four — UTC · New York · London ·
    Tokyo. Keys: `WORLDCLOCK_DEL_BASE = 31_000`, `WORLDCLOCK_ADD_KEY =
    31_100`, `WORLDCLOCK_PICK_BASE = 31_200`.

## 2026-09-14 (feature: Focus / Fans / Workspaces cards + water-fill batteries)

- **Three new dashboard cards** (`tests 114 → 120`):
  - **Focus** (`DashCard::Pomodoro`, id `pomodoro`) — pomodoro timer: big
    mm:ss hero, start/pause/resume + reset, focus-length steppers (±5 min),
    phase progress bar. Focus ↔ break auto-flip (break tinted OK-green);
    each finished focus session bumps a "N done" counter. Durations +
    sessions persist to `$XDG_DATA_HOME/zen-shell/pomo.txt`; the 1 s
    countdown timer self-tears down when paused (0 CPU at rest, mirrors
    `sync_viz_timer`).
  - **Fans** (`DashCard::Fans`, id `fans`) — live fan RPMs from every hwmon
    `fanN_input`, labeled `fanN (chip)`, per-fan bar normalized against a
    slow-decaying peak (spin-ups visible, no hardcoded maxima); stopped
    fans dim; empty state mirrors the thermal card.
  - **Workspaces** (`DashCard::Workspaces`, id `workspaces`) — Hyprland
    workspace tiles (≤10/row, grid adapts); click switches via
    `pending_ws_dispatch` (same path as the pill's long-form chips); the
    active tile is accent-tinted with the dot. Keys `WORKSPACE_KEY_BASE
    = 30_800`.
  All three appear in the parked-cards tray and append to the default
  lattice (row 18 / below WeatherV2). New shell fields: `pomo_*`,
  `fans_rpm`, `fan_peaks`; stats poll samples fans with the 3 s services
  tick; `draw_edit_btn` / `draw_edit_lbl_btn` are now `pub(crate)` (shared
  with card modules).
- **Water-fill battery icons** (both battery cards). `push_battery_icon`'s
  interior reworked into `draw_water`: the liquid now spans the FULL inner
  cross-section so its edges sit against the case stroke on both sides (was
  an inset floating bar), with a vessel tint behind it, a lighter wave-crest
  line along the surface (8-segment sine) and a soft highlight band under
  the crest. Vertical battery: water climbs, horizontal crest; horizontal
  battery: water grows left→right, vertical crest. Charge colors unchanged
  (OK/acc/DANGER ladder).
- **Dual-glyph stack toggle** — the edit-mode scroll-direction button now
  renders BOTH direction glyphs side by side (↕ accent when rows, ↔ when
  columns) instead of swapping one glyph; button doubled in width and the
  right-anchored toolbar cluster math updated to match.
- **Build fix** — `notes.rs` composer held `&self.scale` across a
  `self.region()` call (E0502, blocked every build); `GridScale` is `Copy`,
  so the composer now copies it by value.

## 2026-09-14 (feature: card-header visibility toggles in edit mode)

- **"Card titles" / "Card glyphs" toggles in the edit-mode strip-editor row**
  (right of "Enable banner chips", dashboard ON only). Both default ON and
  persist per channel (`card_show_title` / `card_show_glyph` in the master
  config, honored on boot). Titles OFF drops every card's header TITLE text;
  glyphs OFF drops every card's header ICON and slides titles left into the
  freed slot — content, right-aligned meta and pagination dots are untouched.
  Gated in `card_header` (shared.rs) and at every hand-rolled header site
  (apps, bluetooth, clipboard, clipimg, currency, diskio, docker, eq, lyrics,
  mirror, news, notes, power v/h, procmon, quote, recent, sensors, sshvpn,
  ticker, todo, topproc, viz, wifi, worldmap). `banner_ctrl_min_w` grew to
  fit the two new chips. Tests 110 → 114 (draw+register, hidden when the
  dashboard is off, click→persist→redraw round-trips for both toggles).

## 2026-09-14 (fix: hover expanded to an empty dashboard)

- **Hover-expand deadlock in the size handshake.** The final tween commit
  (1009.79×494.79) was classified against the acked configure (1009×494) with
  a 0.5 px epsilon — but layer-shell sizes are truncated u32, so that IS the
  same surface size. The guard missed it, `set_size(1009,494)` went out as a
  no-op commit Hyprland never acks, and `size_pending` wedged: `tick()` holds
  the anim while pending, `maybe_render` drops frames while the anim is up.
  The 800 ms timeout then un-stuck pending just long enough for the next tick
  to clear the anim and re-commit the SAME never-acked size, and the morph
  timer's stop condition removed the timer while pending was still set —
  nothing left could redraw, so the dashboard expanded to full size and
  painted only its background ("hover does not show dashboard/cards").
  Fraction-dependent, hence every hover on this setup. Fix: `commit_size`
  compares truncated sizes (`App::size_is_configured`), and `morph_tick`
  keeps the timer alive while `size_pending`. Tests 107 → 110; verified live
  (expand renders the full card grid, 133 frames at 1009×494).

## 2026-09-14 (fix: edit-mode ✕ unclickable on a scrolled board)

- **Edit-mode card ✕ (park-to-tray) now clicks where it's drawn.** The three
  edit-mode hit rects were computed in three different coordinate spaces:
  the grip was scroll-corrected (`scrolled()`), the card body subtracted
  `dash_scroll_y` only, and the ✕ rect (`card_close_px`) not at all — while
  the draw loop scrolls ALL content by both `dash_scroll_x` and
  `dash_scroll_y`. With the board panned (trays stack above the grid in edit
  mode, so big boards scroll), the drawn ✕ sat at a different spot than its
  hit rect: the click fell through to the body check and started a DRAG
  instead of parking the card. All three rects now resolve in surface space
  via `scrolled()`; `card_close_px` also takes the scale (offsets/size now
  match the drawn `scale.s(27/7/20)` exactly — grid_scale ≠ 1 was silently
  off before). `edit_motion`'s resize branch gets the same scroll
  compensation the drag branch already had. Regression test:
  `close_button_hit_rect_follows_board_scroll` (tests 106 → 107).

## 2026-09-14 (minimal UI polish — batch 1+2: cards)

Design direction: **minimal / flat / professional** (Linear · VS Code ·
Grafana feel, NOT Material). Settings panel got this treatment 09-13/14;
this pass brings it to the dashboard cards.

- **Type ramp enforced via helpers** (`ui.rs`): small text now renders at
  REGULAR weight (`caption` / `caption_r` / `caption_c` — Light dissolves at
  ≤10px); numerals routed to their proper voices — Display Medium hero
  (`text_c_hero`/`text_hero`) for gauge %s, pkg-update count, GPU util,
  kbd layout, weather temp; Geist Mono for every tabular figure
  (`text_c_mono` added). The configured Inter / Inter Display / Geist Mono
  stack is now actually used everywhere instead of Light Inter by default.
- **Status color tokens** (`ui.rs`): `OK` / `WARN` / `DANGER` / `INFO`
  (muted Tokyo-Night tones) + `pct_color(pct, warn_at, crit_at, normal)` —
  the single threshold ladder. Replaced the two competing RED/GREEN pairs
  (`0xff6b6b`/`0xf38ba8`, `0x2ee6a8`/`0xa6e3a1`), the legacy Catppuccin
  blues `0x44aaff` (network dl, diskio read, speedtest, GPU VRAM bar →
  `INFO`) and oranges `0xffb86c` (cpugpu GPU line, GPU temp → quiet `fg`
  meta / `INFO` line; orange only ever marks a threshold now).
- **Shared card chrome** (`cards/shared.rs`): `card_header` (semibold 11.5
  title, optional quiet fg2 icon, optional right-aligned mono meta) is the
  ONLY header style; `metric_row` (fg2 label + right-aligned mono value);
  `card_empty` (one centered empty state). Migrated: cpu, mem, disk,
  thermal, network, gpu, battery_frame (both battery cards), ticker,
  speedtest. Acc-tinted header icons (thermal, gpu, battery, ticker,
  speedtest) dropped to fg2 — accent is for live data only.
- Per-card fixes: network dl line muted (`INFO`), disk mount labels fg2,
  gauges disk ring `ui::OK`, gauge % in Display, weather forecast temps in
  mono, kblayout icon fg2 + label Display, pkgupdates count Display hero.
- **Weather digit bug**: the big temp was one string `{glyph} 22°C` — the
  digits rasterized in the ICON font. Split into icon + `text_hero` temp.
- `shell::RED`/`GREEN` consts now alias the ui tokens (one source).
- Tests: 106 passed (weather card tests still green after the pane
  rework). `cargo check` clean.

## 2026-09-13 (world-map → always-on + pinch)

- **Embedded fallback back in the binary**. `src/shell/world_map.bin` is
  re-embedded (`include_bytes!`, byte-identical to the GitHub `world.bin`) and
  installed the moment the card draws, so the original world map ALWAYS
  renders — no placeholder, no waiting for downloads. `worldmap::ensure_fallback`
  / `using_fallback()` track the state; the first downloaded world chunk
  replaces it in place (flag clears). A compact "Download full map" pill now
  overlays the map body only while the fallback is active.
- **Touchpad pinch-zoom** (`zwp_pointer_gestures_v1`): bound in
  `ensure_pinch` (lazy, degrades silently when the compositor lacks the
  global), pinch events dispatched into the app — over the map body
  (`Shell::over_worldmap`) a pinch scales `world_zoom` continuously in the
  same 1×..32× band, sitting beside the +/− buttons.
- **Scissor containment**: the whole map body (fill, graticule, dots,
  borders, region rings, city labels, marker) is wrapped in
  `Cmd::Scissor`/`ScissorEnd`, so zoomed/overflowing primitives never bleed
  past the card bounds. Hover crosshair + outline stay outside the clip.
- **Save-downloads defaults OFF**: `[world] save_data = false` in the shipped
  TOML and the `World` default; the toggle still flips/clears cache dirs.
- Tests 93 → 95 (embedded parse/land + fallback flag round-trip; card test
  now asserts the fallback map + download pill draw together).

## 2026-09-13 (world-map → app)

- **Map data leaves the binary**. The world chunk (17 KB), city labels and
  per-country 50m chunks now ship from `ezone134/zen-shell-map` on GitHub
  (`base_url`, overridable via `[world] data_url`), fetched on demand by curl
  workers and verified by sha256 + size. New `pub mod mapdata` (manifest
  cache, `region_at`, atomic chunk downloads) + runtime store in `worldmap`
  (`set_world`/`ready`/`clear`). the binary then stopped embedding `world_map.bin` (it ships again since the "always-on + pinch" amendment above).
- **Placeholder + download pill**: with no cached data the card shows a
  placeholder and a "Download world map" button (`WORLD_DL_KEY`); accepting it
  spawns a background worker, and results are applied on the services tick.
  Cached chunks render instantly at boot.
- **Swipe-right menu** (`WORLD_MENU_KEY` backdrop + rows): 4 style panes
  (`WORLD_STYLE_KEY_BASE`..+4) and a **Save downloads** switch
  (`WORLD_MENU_TOGGLE_KEY`) that flips the cache dir
  `~/.local/share/zen-shell/maps` ⇄ `/tmp/zen-shell/maps` and persists to
  config (`[world] save_data`, default ON). Swipe-cycling + pagination dots
  are gone on this card (other cards unchanged via `card_pane_flip_n`).
- **Zoom +/−** (`WORLD_ZOOM_IN/OUT_KEY`, 1×..32× around the pinned marker),
  windowed projection (`win_for`/`fit`/antimeridian wrap, `Win::contains`),
  city labels from 2×.
- **Region detail**: cross 4× zoom inside a country → "Download <Country>"
  pill (`WORLD_REGION_DL_KEY`, region picked by smallest containing bbox via
  `region_at`); its rings draw accent-stroked over the base map; cached
  regions auto-load while zoomed.
- Map back-end reworked: `src/shell/worldmap.rs` (chunk parsers round-trip)
  + drawer `src/shell/panels/cards/worldmap.rs`; new shell fields for
  center/zoom/menu/dl-state/region/manifest/marker. Tests 87 → 93 (chunk
  roundtrip, manifest `region_at`, all pane drawings, placeholder, menu,
  region pill, marker band); world-store tests serialize on `TEST_LOCK`.

## 2026-09-13

### World map card (vector, clickable world clock)

- **New dashboard card `DashCard::WorldMap`** (id `worldmap`, region key
  `WORLD_MAP_KEY` = 14_900): a clickable world map drawn ENTIRELY from vector
  primitives, no bitmap. Earth 110m country polygons are pre-compiled into the
  embedded `src/shell/world_map.bin` (17 KB, 162 rings / 4096 points, i16
  lon/lat in 1/100°) and rasterized once into a 1440×720 scanline landmask;
  everything on screen derives from it.
- **4 style panes**, flipped by horizontal swipe (no wrap-around) with the usual
  pagination dots: **Real** (accent-tinted land + country borders + 30°
  graticule), **Filled** (clean silhouettes — pixel-exact at the map's own
  resolution and vertical-run merged), **Dots** (dot-matrix land), **Lines**
  (borders + graticule only). The card defaults to span (10, 9) — the 2.1:1
  world needs height on the ~6:1 grid cells.
- **Click → local time**: clicking the map un-projects the pointer to (lon, lat),
  pins an accent marker, finds the nearest timezone in `/usr/share/zoneinfo/
  zone.tab`, and shows that place's **HH:MM + timezone abbrev + weekday** in the
  card's bottom band. The UTC offset is probed once with a single
  `TZ=<zone> date +%z %Z` subprocess (`world_shell_probe` in `src/app/mod.rs`,
  3 s services timer) and re-probed ~every 5 min for DST; the clock itself is
  formatted at draw time by pure libc shifts (`world_hhmm` / `world_dayline`) —
  no per-frame processes.
- **Multi-pane machinery generalized**: `pane_dots` (any pane count —
  `battery_dots` is now a 2-pane alias) and `card_pane_flip_n`; weather's and
  the battery cards' behavior is unchanged.
- New module `src/shell/worldmap.rs` (parsing, landmask, equirectangular
  projection + fit, zone.tab nearest-zone lookup, tz-offset parse, time
  helpers) + drawer `src/shell/panels/cards/worldmap.rs`. Tests 84 → 87
  (worldmap data/mask/projection/zone/time + the 4-pane card drawer).

## 2026-09-11

### Edit-mode banner & parked-tray polish

- **Parked chips/cards columns stretch to the panel width**: the banner-chips
  tray and the dashboard-cards tray no longer leave a dead strip hugging the
  right edge on wide windows — each column STRETCHES so every row fills the
  tray's full width (`banner_tray_chip_px` / `tray_chip_px`). Wrap count stays
  the same, so tray heights, pagers, scrollbars and hit-testing are all
  unchanged (drawer and input keep reading the same shared rect).
- **Edit-mode banner chips render as ICONS, not word labels**: strip cells and
  the parked banner-chips tray now draw each chip's Nerd Font glyph
  (`BannerItem::glyph()` — clock → clock, wifi → wifi, settings → gear, …)
  instead of its descriptive word, so the strip reads at a glance while staying
  draggable. `Dash: ON/OFF` still keeps its words; separator glyphs and the
  spacer label are untouched.
- **Drag handlebar raised + easier to grab**: the edit-mode grab ledge on strip
  chips moved up (led 9 → 12 px) and the bar is slightly taller + brighter on
  hover, so it reads as a clear grab target instead of hugging the band's
  bottom edge. Chips show icons, fillers keep their centered handle.

### Dead-code sweep (≈96 audit warnings → 0 warnings, 0 errors)

`cargo build` is now clean. Deleted unused fns/fields/consts, incl. hypr
`dispatch_workspace_prev/next/previous`, `LogindBrightness::raw`, config
`weather_enabled` + `notif_timeout_for`, `Rate::Live`/`Rate::Fast` (+ their ttl
arms), `BANNER_W_MINUS_KEY`/`BANNER_W_PLUS_KEY`, `BannerToken::chip()`,
`BannerDrag.sx/sy`, `BASE_TRAY_STRIP_H`, the old `pack_cards()` (its 4 tests
rewritten against `pack_cards_max`), the drop-zone helpers
(`banner_drop_band`/`banner_drop_slot`/`banner_zone_at`/`default_banner_zone`/
`banner_home_zone` — superseded by the column-based drop grid), `dash_tray_rows`,
`tray_strip_h`, `card_at`, `tray_chip_at`, `launcher_scroll_by`, `gauge_gb`, and
ui.rs `warn`/`ok`/`frame`/`chip`/`vslider`/`shadow`. Kept as intentionally-unused:
`DEFAULT_SHELL_TOML` and `Layout.w/h` (both exercised by tests). `banner_drop_target`
is kept as `#[cfg(test)] pub(crate)` (used by 13 pack-test call-sites).

**Two strip-editor controls that were drawn but click-dead for a while were
re-wired** in `edit_press`: the dashboard ON/OFF row button (`BANNER_CTRL_KEY_BASE`)
and the banner-width AUTO/MANUAL toggle (`BANNER_W_AUTO_KEY`, restores the last
manual value via `banner_w_last`). (The `−/+` width buttons from the 09-10 batch
were removed; AUTO/MANUAL + the hidden live-width slider took over, and the sweep
deleted the orphaned `BANNER_W_MINUS_KEY`/`PLUS_KEY` bands.)

### Polkit agent registration fixed

The agent found a real bug on live startup: it registered with its **session-bus
unique name** as the subject, which polkitd (system bus) rejected with
`Unknown subject of kind ':1.1910'` — the agent thread then died with no retry, so
password auth could never work. `register_with_authority` now uses a
`unix-session` subject keyed by `XDG_SESSION_ID`; startup logs
`zen: polkit agent registered`. Verified across 4 live restarts on Hyprland (W1).

### Tests (58 pass) + live validation

- New `Layout` geometry tests (`shell/panels/layout.rs`): surface size, geometry
  matches shell metrics, grid starts below the banner strip, edit chrome stacks
  above the grid, tray-gap split between chrome rows.
- New `edit_drag_tests` (`shell/mod.rs`) drive the reconstructed
  `edit_press`/`edit_motion`/`edit_release`: move-and-commit, same-slot no-op,
  close-to-tray, grip-resize clamping, noop release.
- Live run of the new binary on the real Hyprland session: broker warm-up fires on
  dashboard open (cards flip to "fetching…"), the weather card populates
  (48°/43°/44°/59°) — request→render works end-to-end. Quote/news/ticker degrade
  gracefully here because their hosts are unreachable from this box
  (`api.quotable.io` → HTTP 000) and every fetch has an 8 s `curl --max-time` cap,
  so nothing hangs. Collapsed mode = zero fetches, as designed.
- `TODO-tomorrow.md` (sweep/layout-tests/edit-input-runtime/broker-warm-up)
  completed and deleted.

## 2026-09-10

### Banner strip editor + edit chrome (parts 2 & 3, merged from NEXT_STEPS)

- **Strip editor shipped**: text-only chips (no pill bg), separators + smart
  fillers, free drag (a chip slips *through* a filler), an always-visible control
  row below the banner tray (dashboard ON/OFF state-aware toggle, separator
  presets, spacer; width AUTO/MANUAL toggle + live-width slider hidden in auto
  mode), `DashToggle` removed from `DEFAULT_BANNER_ORDER` + filtered on load,
  empty-strip band collapse, ink-centered glyphs + one shared midline,
  drop-anywhere column grid, L/C/R zone sections (`Zone(u8)`) with click-to-home
  zones, 3 s liquid-fill "clear all", empty-state hints, opaque full-width chrome
  slabs.
- **Edit chrome (part 2)**: chip-based `banner_collapsed()`, click-clear for both
  clear-alls, `dash_area_collapsed()` folding, word labels for edit-mode chips,
  `Wifi`/`Bluetooth`/`Connectivity` + 9 basic status chips (battery, weather, cpu,
  ram, net_speed, dnd, volume, brightness, vpn) with live glyphs + accent states +
  click targets, stack-mode 16-col clamp + directional scrollers + hidden rail,
  hover-only pagination dots.
- **Tray scrollers + UI style sliders**: scrollable parked trays
  (`BANNER_TRAY_VROWS=2`/`DASH_TRAY_VROWS=3`, row-index pager, thin scrollbar);
  config `ui_pad`/`win_radius`/`card_radius`/`card_radius_sync` + edit-mode
  sliders (surface corners follow `panel_r()`); `DASH_CARD_KEY` moved 34_400 →
  34_480 to clear the app-shortcut band; Settings "Expand on hover" info-tip band.
  Compiled + tested (48 tests); three latent build bugs fixed (undeclared slider
  rects, `\u{00b7}` escape, connectivity-popover borrow).
- **BUGFIX**: dashboard no longer collapsed to a ~350 px ribbon when ON —
  `vert_col_cap()` is now viewport-based (up to the 16-col vertical-stack ceiling)
  when the dashboard is on; the strip clamp only applies when the dashboard is OFF.
- **Operational notes**: the bar configs live in `$states/shell_{ch}` (channel from
  `$states2/s`, live = `n`); card/grid edits need `zen-shell reload`; pin moves
  (grid placement) need a restart; power/cpu/clipimg cards are pinned on the `n`
  grid with copies in the `d`/`l` trays.

### Power cards, system-card settings chevron, wallpaper-card grid (v1–v6, merged)

- `DashCard::PowerV`/`PowerH` hold-to-confirm cards (keys 12_000 band — moved down
  from 35_000, which the banner arm swallowed); icon-only tiles.
- System card bottom-right `>` (⇢ Settings). Wallpaper card: adaptive cells (≥5 cols
  × 2 rows), live count, right-edge scrollbar (`WP_CARD_SB_UP/DOWN` 10_000/10_001),
  clip-safe rows.
- Memory + CPU cards reworked (mem used/avail/cached/free rows; cpu load/freq/temp),
  circular gauges (ring beads, pct + used/total, `disk_used_gb`/`disk_total_gb`).
- Clipboard text card click-to-copy (CLIP 80) + Clipboard-images card
  (`clipimg`, CLIPIMG 140); clipboard freeze fixed (skip own echo offer), panel
  scroll, `{texts:[...]}` persistence.
- Weather thread always runs; auto-geolocates via ip-api.com when lat/lon are 0
  (`WeatherMsg.city`, `Clone` not `Copy`).
- Media card glyph/transport fixes + Space play/pause; Theme & Accent moved off the
  system card with a Scheme chip → Themes; list cards (clipboard/clipimg/todo/notes)
  scroll via shared `list_scroll_by`.

## 2026-09-09

### Dashboard UI batch (Workstream B, 47/47 tests)

Scroll-direction −/+ toggle in the edit toolbar (`dash_scroll_dir`), edit-mode
opaque backdrop, banner separator + spacer below the banner, parked-chip
click-to-toggle placement, new `AppShortcut` card (esp. `app_shortcut_pick`
launcher flow), clock/date/app-search banner chips.

> Zeneq EQ crate: **HALTED** (user) — this machine's PipeWire cannot load LADSPA
> (`libspa-filter-graph-plugin-ladspa.so` needs `spa_log_topic_enum`, not in the
> installed spa). Do not reopen until PipeWire/spa are updated.

## 2026-08-29

### Ultra-fast startup: all file I/O deferred past the first frame

- `Shell::new()` no longer reads any files — hostname, uptime, todo list,
  dashboard layout, notification history, states colors, and accent flags are
  all loaded by a new `seed_data()` method that runs **right after the first
  frame is committed** (alongside the existing `seed_boot_data()`).
- **App-manager background scan**: the `.desktop` file scan (hundreds of
  files across 3 directories) now runs on a **background thread** via
  `AppManager::rescan_async()` and results arrive through a `std::sync::mpsc`
  channel polled every 3 s in the services tick.  The launcher starts with an
  empty list and populates within a second or two — the bar + launcher are
  interactive **immediately**.
- The pill, launcher, and any boot-mode surface now appear on screen before
  *any* disk I/O, sysfs read, or subprocess spawn — true sub-frame startup.

### Color-temperature slider range widened to 1200 K–6500 K

- Dashboard color-temp slider now spans **1200 K** (ultra-warm candlelight)
  to **6500 K** (daylight D65), up from the previous 2500 K–6500 K range.
- `Shell::seed_data()` handles the post-frame initialization including
  `hostname()`, `uptime_str()`, `load_todos()`, `load_notifs()`,
  `load_dash_layout()`, `apply_persisted_dashboard()`, `refresh_pack()`,
  `reload_states_colors()`, `reload_accent_flags()`, and `sync_dark_state()`.



### Dashboard layout persisted as an array in `$states/shell_{n,d,l}`

- Every **enabled** dashboard card is now remembered with its **order,
  size AND exact position** and written to the per-state channel config as a
  `[dash_cards]` array of `{ card, x, y, w, h }` (cells). Cards parked in
  the edit-mode tray are saved as id strings in `[dash_tray]`.
- Saved on every pill-settings change (`save_config()`) and on every
  edit-mode commit (move / resize / ✕-park / tray add) — so the dashboard
  survives restarts exactly as the user arranged it, per channel
  (dark/light/night each keep their own).
- On load (`Shell::new` + channel switch in `sync_per_state_config`), the
  saved cards are **pinned** (`layout_pinned`) to their exact positions so
  the flow packer does not re-derive/drift them; the pin clears on the first
  edit and the live packer takes over again.
- The legacy order+spans `~/.local/share/zen-shell/dash_layout.txt` is still
  written as a fallback / first-boot source.

### Flow-packer rules (documented)

- Cards are a vertical **stack**: first card = top of stack (placed first),
  last card = last of stack (placed last).
- **Left-side fill:** within its own row a card may slide left at most ONE
  column to fill an adjacent gap (never a far-left teleport).
- **Jump-up compaction:** when its own row is full, a card jumps up one row
  into the **rightmost** empty slot of an already-populated row — so the
  **2nd row's LEFT card pops into the 1st row's RIGHTMOST slot** — keeping
  rows left/top-packed without scrambling card order.
- Otherwise it scans down to the first row with enough room. The default
  grid is bit-for-bit preserved (regression-tested).

### Faster startup: peripheral data seeded after the first frame

- The bar + launcher now come up instantly. The blocking reads that used to
  run before the first frame — hyprctl ws/active-win IPC (`Hypr::update_shell`),
  D-Bus ssid/media (`Services::poll`), battery/AC (`power.poll`), sysfs
  brightness, and the `wpctl get-volume` subprocess — are deferred into a new
  `App::seed_boot_data()` that runs **right after the first frame is committed**
  at the initial wayland roundtrip in `App::start()`.
- `App::new` still constructs the objects (cheap sockets/structs) but no
  longer blocks on the data reads, so rendering + input are interactive
  immediately. `seed_boot_data()` only redraws if a seed actually changed
  something; the existing 3s services / 60s power timers take over from there.

### Custom accent section in Settings → Appearance

- New **ACCENT SOURCE + CUSTOM ACCENTS** section in Settings → Appearance.
- **Accent source** buttons: *From wallpaper* (`theme_main accent
  acc_from_wall`) and *Scheme default* (`theme_main accent theme_default`) —
  each writes the `$states/acc_source_<ch>` flag via `theme_main_body`.
- **Custom accents**: reads `$states2/custom_acc.json` (the `[{name,d,l}]`
  list regenerated by `gen_launcher_cache`/`gen_custom_acc_files`, now also
  mirrored into `$states2/`). Rendered as a **collapsible dropdown**: a
  "CUSTOM ACCENT" row shows the current selection (name + swatch); clicking it
  opens a **scrollable popup list** (wheel scrolls its own list; a thin
  scrollbar shows position). Clicking a name selects it and runs
  `theme_main accent custom_acc <name>` → turns custom accent on +
  `custom_acc_name_<ch>`, echoes `acc_changed`, and restores so GTK (thunar)
  picks the color up (restore applies the previously-selected value state).
- A `colors reload` is scheduled after each selection so the live palette
  (`$states2/shell_vars`) updates the shell without a restart. Selection is
  highlighted against the current `custom_acc_name_<ch>` flag.

### `theme_accent_func`: single `acc_source` flag + change-only restore

- The accent setter now writes the canonical `$states/acc_source_${s}`
  flag alongside the per-source flags (`d` = theme default, `w` = acc from
  wall, `c` = custom accent, `h` = defined hex) — the same flag the Rust
  settings pipeline and the old `theme_body` read.
- **Change-only restore:** if the requested source (and payload, for
  `custom_acc`/`define_hex`) already matches the current `acc_source` flag,
  it does nothing — no `theme_main restore`, no `acc_changed` write.
- On an actual change it echoes `1 > $states2/acc_changed` AND runs
  `theme_main restore`, so GTK apps (thunar) pick up the new accent color.
- `define_hex` with no payload reads `$HOME/documents/defined_hex` via
  bash's `$(<file)` — no `cat` subprocess spawned.
- `acc_from_wall all` now iterates the d/n/l channels and only triggers a
  restore if at least one channel actually changed.

### Accent toggles + Alpha in Settings → Appearance

- New **ACCENT TOGGLES** section: flip the per-channel `custom_acc_start_icon`,
  `custom_acc_app_border`, `custom_acc_hyprland`, `custom_acc_gtk` and
  `custom_acc_normal_app` flags, plus a **Start icon tone** slider
  (`start_icon_tone_<ch>`, 0–900).
- New **ALPHA** section: sliders for `bg_alpha` / `acc_alpha` / `scrim_alpha` /
  `border_alpha` (2-digit lowercase hex) and an **Accent scrim** toggle
  (`acc_scrim_<ch>`).
- Everything routes through `theme_main` / `theme_main_body`:
  `theme_main toggle_acc <flag> <0|1>`, `theme_main start_icon_tone <n>`,
  `theme_main alpha <flag> <hex>`, `theme_main acc_scrim <0|1>` — each writes
  the flag and runs **`theme_main restore`**. **No `acc_changed` write** — that
  marker is only for accent *source* picks that need the GTK (thunar)
  hot-reload; a plain restore is enough for these flags.
- A single **Apply changes** button at the bottom runs `theme_main restore` so
  all pending toggle/slider edits hit the live palette at once (the shell
  re-reads them via the scheduled `colors reload`).
- The Appearance pane gained a vertical scroll so the new sections + the custom
  accent grid all fit; flags are re-read on boot and every `colors reload`
  (`Shell::reload_accent_flags`), so the UI tracks the live `$states` values.
