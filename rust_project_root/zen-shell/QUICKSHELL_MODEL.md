# The Quickshell model, in Lua — design + build order

> **What this file is:** the target architecture for the declarative engine and
> the ordered work to get there. It is a *design* doc, not a session log.
> For what landed read `CHANGELOG.md`; for open items and the build recipe read
> `NEXT_STEPS.md`.
>
> **The goal in one line:** the ergonomics of Quickshell — declare objects,
> write bindings, instantiate delegates over models, override component
> properties — expressed in `.lua` modules instead of QML, on top of a shell
> that is event-driven and draws through one command list.

---

## 0. The boundary: what Rust is allowed to be

Everything else in this file is detail. This section is the goal, and it is a
single line:

> **Rust is the backend and the Wayland renderer. Every pixel and every decision
> lives in Lua.** All card and surface logic — what to show, where it goes, how
> big, which state, every transition — is a `.lua` module under
> `~/.config/zen-shell/ui/`. Rust only produces values from bytes and turns the
> one `Cmd` list into Wayland buffers.

That means:

| Lives in Rust | Lives in Lua |
|---|---|
| sysfs / procfs / netlink / D-Bus reads | which data to show |
| spawning commands, IPC, the compositor connection | where it goes and how big |
| text shaping and the `Cmd` list → Wayland buffers | colors, radii, glyphs, hover fills |
| decoding a byte stream into numbers (a PNG, an FFT frame) | **all state**, and every transition between states |

The Rust side is a **backend + renderer**: it exposes the primitives the Lua
modules paint with (`ctx.ui.*`) and the values they read (`ctx.api.*`), and it
renders the declarative item tree a module hands back (`ctx.ui.scene(...)` — the
shared renderer, not a per-card drawer). The line is not "how much code is in
Rust" — it is **who decides a coordinate**. Rust publishes *values*; Lua decides
what they mean. A `layout_*` / `draw_*_card` drawer is a violation not because
it is long, but because it computes a position.

### What "close to Quickshell" means here, precisely

**Same ergonomics, different substrate.** The target is that an author editing a
`.ron` file has the experience of editing a Quickshell file: declare objects,
bind properties, instantiate delegates over models, override component defaults,
keep state on the objects. Quickshell gets that from QML + Qt; we get it from
RON + serde + this engine. Nothing about the *authoring model* is a compromise.

What is **not** being copied, and will not be (§9 has the full list):

- **QML is not the model.** QML is a whole language with functions, imperative
  blocks, and a runtime object tree. `.ron` is a data format. So there are no
  user-defined functions in a scene — every behavior is data plus a transition,
  which is *more* of a constraint than Quickshell accepts, and the point of it.
- **Qt is not the renderer.** We draw through one `Cmd` list into Wayland
  shm/vulkan buffers. That is why the engine can afford a hand-written `Val`
  codec and a load-time validator; QML's dynamic everything is not a target.
- **The render loop is not copied.** Quickshell runs bindings continuously; we
  are event-driven and idle at 0% CPU, so rule 3's dirty tracking is *required*
  here rather than optional.

What **is** copied is the part that is genuinely good: implicit property
inheritance, bindings over assignments, models + delegates, components with
overridable defaults, state that lives next to the UI that reads it, and
animations declared on the field rather than hand-rolled per widget.

**There is no "the algorithm is too complex" exemption.** An earlier draft of
this table excused "genuinely irreducible algorithms (FFT, fluid drag, image
decode)" in Rust; that was wrong and it is the exemption every rewrite quietly
grows back. Fluid drag is not too complex for `.ron` — it is *state plus a
transition*, which is exactly what §5's `store` and §6's dirty-set make
expressible. If some behavior cannot be written declaratively, that is a gap in
the engine to close, not a license for the renderer to keep deciding. Only two
things stay: producing numbers from bytes, and turning a `Cmd` list into
Wayland buffers.

### The invariant that makes this a rule, not a vibe

**Rust never emits geometry.** Every `Cmd::Rect` / `Cmd::Text` / `Cmd::Image` in
the process originates from a parsed scene tree, never from a function that
decided where to put it.

This is checkable, and it is how progress gets measured:

- **`Ink()` count → 0.** `Ink(name)` is the escape hatch that hands a rectangle
  of the screen back to a Rust painter. **0 call sites remain in `shell.ron`** —
  `dash_banner` was the last one and is deleted; the `dashboard` surface now
  declares its board (a `Card` `Repeat`), its strip band (`dband`), its strip
  chips (`dchips`) and its connectivity popover (`dconn`), so `layout_banner`
  runs only as the fallback for a config with no surface of that name.
  **The settings drawer is done**: its six bodies are
  declared, the `settings_ink` dynamic name is **gone** (not stood down — the
  binding is unpublished and the `Ink` item deleted), and there is no
  `settings_<tab>` painter left in Rust to disagree with a declaration.
  `notif_list`, `lock_screen` and `wallpaper_picker` went the same way — the
  picker's whole body (empty state, thumbnail grid, scrollbar) is declared, and
  its Rust painter survives only as a `#[cfg(test)]` parity oracle.
  **Count call sites, not matches**: a bare
  `grep -c` over-counts, because comments here name the remaining ones. The
  remaining two are the CARDS below — every shell surface body is declared.
- **Card scenes: two `Ink`s left** — the metric above counts `shell.ron` call
  sites, so read this separately: `ui/cards/notes.ron` and `worldmap.ron` still
  route a body through an `Ink` (the ratchet's `notes.ron/notes`,
  `worldmap.ron/worldmap`). Every other card scene is fully declarative —
  `accent` and `eq` were the last two that were not, and both landed.
- **No `layout_*` in the draw path.** `layout_collapsed` (493 lines) is already
  a non-participant: the `pill` surface is live-declared, and the Rust function
  survives only as the parity oracle the tests compare against. That is the
  shape every conversion ends in — **the Rust drawer becomes the test fixture.**
- **A new `Ink` is a rejection.** Adding one means something could not be
  declared yet. Record *what* was missing instead.

The `Mode` enum (**21 variants**, 1 declarative) is the same problem at a coarser
grain: it is a hand-maintained dispatch table of "which panel is showing",
which is exactly the state phase 7 moves into the scene.

### The scene today

One file, `~/.config/zen-shell/shell.ron` (798 lines), holding **12 top-level
surfaces**, **18 declared components**, and **~170 binding fields**. Delegates
are live (`item.` resolves in 29 places), `props:` overrides 4 call sites,
`store` holds `nav.tab`, one field animates, and 15 sites read `parent.*`.

### The file that is authoritative, and the one that is not

`vars::shell_scene_path()` resolves to **`~/.config/zen-shell/shell.ron`**, and
every scene test reads that same path. `config/zen-shell/shell.ron` is a
**mirror** that nothing reads.

This distinction has now cost real work twice, so it is worth stating as a rule
rather than a fact:

- On 2026-09 the mirror drifted silently for three weeks (207 vs 742 lines)
  because nothing read it.
- On 2026-10-03 phase 6's `parent.*` migration was hand-edited into the
  **mirror** and never copied to the live file. The suite stayed green at
  524/524 the whole time — because the suite reads the live file, which had
  **zero** `parent.*`. Phase 6 was documented as landed and had never executed.

**A green suite proves the file the tests opened, not the file you edited.**
Always sync live → repo, never the reverse:

    cp ~/.config/zen-shell/shell.ron config/zen-shell/shell.ron

Both copies carry this warning in their header. If the mirror is ever more than
one commit behind, delete it — a stale copy that looks authoritative is worse
than no copy.

### Why the phases are ordered the way they are

Phases 1–2 make `.ron` *expressive*. Phases 3–4 make it *affordable*, and 4 is
what makes it **logical** rather than merely visual:

- Without **4**, all state stays in Rust (`SceneAction` has one variant,
  `Command(String)`), so "logic in `.ron`" is unreachable by construction.

So the ceiling is not reached by adding scene features. It is reached when
**Rust stops making decisions**, and only **4** is on the path there.

> **Correction (measured 2026-10-01, after phase 2 landed).** This section used
> to argue phase 3 gates 4 on performance: "every binding recompiles its
> expression on every read, every frame — so declaratively-written logic would be
> too slow to ship." **That is wrong by about three orders of magnitude.** The
> expanded live `shell.ron` is ~170 binding fields across 12 surfaces; one
> `compile()` + `eval()` is 43 ns, and holding the `Arc<Expr>` instead is 13 ns.
> So a full frame pays **~7.6 µs** re-resolving every binding and **~14.7 µs**
> cloning the 12 surfaces it draws — **~22 µs, or 0.13% of a 60 Hz frame's
> 16,666 µs**. The expression cache phase 3 was built around would save ~5 µs.
> Phase 3 was therefore demoted: it is **not** a gate on 4.
>
> The cost that *does* grow with the rewrite is not evaluation — it is the pool.
> `Shell::scene_values()` does **594 `String`-keyed `HashMap` inserts per frame**,
> rebuilding `SceneValues` from scratch every frame (~30 µs today), and that
> grows linearly with every panel converted. Phases 1–7 never touch it, so a
> scene that is *correct* can still be the frame's hot spot. Phase 3 keeps one
> genuinely non-perf half: a store/delegate scope is a **closed** set of names,
> which is what finally lets the validator answer "does anybody publish this?"
> for a card-local pool (today it must stand down — `is_complete()`).

Keep this correction in mind as the standing lesson: **the roadmap's ordering
justifications were written before anything was measured, so treat them as
hypotheses.** Phase 4 is next not because 3 is too slow, but because it is the
only phase that moves a *decision* out of Rust.

---

## 1. What "like Quickshell" actually means

Five rules. Everything else in this file is a consequence of them.

| # | Quickshell rule | Why it matters |
|---|---|---|
| 1 | **You declare objects; the tree IS the thing.** No imperative construction, no "build a node then configure it". | The scene file is the whole description — there is nothing to keep in sync with a Rust constructor. |
| 2 | **Property inheritance is implicit.** A child reads the parent's property unless it overrides it (`width: parent.width * 0.5`). | You never restate context. This is the single biggest source of brevity in a Quickshell file. |
| 3 | **Bindings, not assignments.** Write `w: card_w - 28`; it re-evaluates itself when anything it reads changes. You never "update" anything. | No cache invalidation in the scene, no `refresh()` calls, no per-frame "apply the new state" pass written by hand. |
| 4 | **Models + delegates.** `Repeater { model: x; delegate: Y { } }` instantiates `Y` per row with `index` / the row in scope. | One declaration covers any number of rows, and the row's *fields* are addressable. |
| 5 | **Components with overridable property defaults; state lives on the objects, read/write.** `Component { id: c; property real w: 20 }` then `c { w: 40 }`. | Declare once, instantiate many with different props. State next to the UI that reads it. |

### Where the tree is today

| # | Status | Evidence |
|---|---|---|
| 1 | **done** | `SceneItem` + `CardScene`/`ShellScene` are pure data (`src/scene.rs`); `shell.ron` declares whole panels with no Rust body. |
| 2 | **partial** | A parent box is threaded by hand: `draw_piece(x, y, card_w, card_h)`. Containers re-derive it through `pad` / `spacing` / `fills_w` / `fills_h` / `widest_content`. Every new field re-threads it, and rule 3's gotchas (centered text, right-anchored text) are all consequences of the threading being positional. |
| 3 | **partial** | `Val::Expr` bindings exist and are real (`src/scene/expr.rs`), but they are resolved **every frame, for every bound field**, with no dirty tracking, and `Val::num` recompiles the expression text on every read. |
| 4 | **done (pill pilot)** | `SceneValue::Model(Vec<ModelRow>)` (`scene.rs:828`) carries per-row FIELDS; `Repeat::repeat_slots` (`scene.rs:3191`) instantiates a template under `SceneScope::row(row, index)`, so a delegate binds `item.<field>` / `index`. Dispatch is the row's `tpl` (a delegate name) then its `tag`, then a `*` wildcard. `Repeat { templates }` doubles as the delegate list — a `*`-tagged template *is* the anonymous default delegate. |
| 5 | **missing** | `components:` is already declared in RON (`shell.ron:27`, resolved through `ShellScene.components` and inlined at `scene.rs:3540`), but `Comp { name, x, y }` (`scene.rs:2961`) takes **no arguments** — every variation is a copy-paste. And `SceneAction` has one variant, `Command(String)` (`scene.rs:111`), so state is Rust-only. |

### The cost of rule 4 having been missing

A model of strings cannot carry structure, so every data-shaped card has to
publish **parallel name families** instead:

- the pill: `pill_key_<tag>` + `pill_ink_<tag>` per widget
- the workspaces grid: `ws_wash_i`, `ws_num_i` per row
- the banner strip: a bespoke `BannerCell` struct rather than a model
- powerdraw: `pw_b_series` / `pw_g_series`

That flattening is the reason `SceneValues` needed `complete` +
`conditional` + `declare_conditional()` at all: the validator cannot ask "does
the model publish this?" when the model is four independent name families that
each appear under their own condition. **Fixing rule 4 deletes a class of
bookkeeping, not just lines** — the first model (the pill) deleted the
`declare_conditional` loop outright, because a row that exists *is* a widget
that is on, so no name is published only sometimes.

---

## 2. The foundation: a scope overlay

Rules 4 and 5 are the same mechanism, and it is small. When the engine inflates
a delegate or a component, it pushes a small overlay map that bindings resolve
against **before** the flat `SceneValues` pool:

```
scope = { "item"  → the model row's fields,
          "index" → usize,
          "slot"  → the Repeat's per-slot scalars,
          <component prop defaults / overrides> }
```

Rules it must obey:

- **An overlay never mutates the pool.** It is a stack frame around inflation,
  popped when the subtree is done. A nested `Repeat` pushes its own.
- **Resolution order is scope, then pool.** A name in scope wins; a name that is
  in neither resolves to 0 exactly as it does today. A scope name is
  `item.<field>` / `index` — a *dotted* name, which is why the expression lexer
  reads dotted identifier runs (`scene/expr.rs`) rather than stopping at the
  first `.`: a bare `item` plus a stranded `.w` would fold to a wrong number
  instead of erroring.
- **An overlay is only *sound* if its contents are known.** This is the part
  that makes the validator better rather than weaker: when the engine inflates a
  delegate it knows the model's row shape, so `name_prop` can check `"item.w"`
  against the actual row instead of skipping the check. That is how card-local
  names become checkable (see §6). A model's rows are a CLOSED set, so
  `item.<field>` is checked against the union of every row's fields —
  `LoadIssue::UnknownRowField` — while an empty or all-conditional model stands
  down, because "no rows today" says nothing about "no rows ever".
- **`Val::deps` already walks an expression's names** (`scene/expr.rs:445`, used
  by the validator at `scene/validate.rs:198`), so "which names does this binding
  read" is answered by the compiler we already have. A *color* token and a
  `{name}` interpolation are not expressions, so `SceneItem::scope_names` has to
  walk for those too — and a test asserts that walk covers every field the
  evaluator actually reads, so the two cannot drift.

---

## 3. Rule 4 — models and delegates

```ron
items: [
    Repeat(model: "pill_ctr", templates: [
        (tag: "chip", items: [
            Surface(x: 0.0, y: 0.0, w: "item.w", h: 24.0, color: value("item.ink")),
            Hit(key: "item.key", x: 0.0, y: 0.0, w: "item.w", h: 0.0),
        ]),
    ]),
]
```

with the publisher writing

```ron
pill_ctr: [ (tag: "wifi", tpl: "chip", key: 60, ink: fg, w: 16.0),
            (tag: "volume", tpl: "chip", key: 70, ink: fg, w: 22.0) ],
```

In the delegate, `item` is the row and `index` is its position.

- **New carrier:** `SceneValue::Model(Vec<ModelRow>)`. `ModelRow` is a `Vec` of
  `(String, SceneValue)` — the same "row is a value, not a naming convention"
  shape `Rows` already used — so a row's width/ink/key are one value instead of
  a family of pool names.
- **New field on `Repeat`:** `model`. `templates` is reused rather than replaced
  by a separate `delegate:` key, because the tag-dispatch and delegate cases are
  the same lookup over a different key: `templates.iter().find(|t| t.tag == tag)`
  is what Quickshell's `delegate:` does. One code path, two ways in.
- **`templates` + `slot_key` stay** for the `SceneValue::List` of tags, which is
  still what the non-model repeaters use. A `*`-tagged template is the wildcard /
  anonymous default delegate in both.
- **Engine:** `repeat_slots` pushes `SceneScope::row(row, i)` per model row and
  folds the scope into the template subtree (`SceneItem::scoped`), instead of
  string-matching a tag.
- **Dispatch is `tpl`, then `tag`, then `*`.** This is the one deviation from
  Quickshell and it is deliberate: Quickshell has one `delegate:` per repeater,
  but a repeater whose rows are of several *kinds* (the pill's 12 center widgets
  share three bodies) needs the row to choose. Naming the body's delegate on the
  row (`tpl`) and keeping `tag` as the row's own identity means:
  - two widgets with different identities but the same body share a template —
    which is why the pill went from 14 templates to 8;
  - a new widget whose body matches an existing delegate needs *no* template, so
    adding a widget cannot fail to appear. A row whose `tpl` names nothing and
    which has no `*` template draws nothing, so the coverage has to be checked;
    `scene::every_pill_widget_has_a_template_for_its_delegate` walks
    `ALL_PILL_ITEMS` against the live scene's declared templates for exactly
    this reason.
- **`Card(name: "<id>")` runs the shell's OWN drawer at a declared box.** It is
  the one item that calls back into Rust, and it exists because a dashboard
  card's body is a per-frame stateful painter (`DashCard::draw`) with wheel
  rects and caches, not a static scene — declaring its *box* while it paints
  its *body* keeps the coordinate decision in `.ron` without reimplementing
  every card as RON. The surface installs a `CardSink`; the sink registers the
  plate's region first (so the card's own buttons win the hover) then runs the
  painter, so the Rust board and the declared board share one body. An unknown
  `name`, or a `Card` drawn with no sink, warns and paints nothing.
- **`slot_key` is sugar for `key: "item.key"`** on a list, and does not apply to
  a model slot — a model row is already keyed by itself.
- **`dir: free` makes a repeater a CANVAS.** It advances no cursor: each
  template child reads `item.x` / `item.y` for itself and the repeat only hands
  over the parent box. A flow-packed board has no uniform slot pitch, so
  `across` (measure each slot) and `down` (stack at `spacing`) are both lies
  about it; the dashboard board's `dash_cards` rows carry their own boxes from
  `publish_dash_cards`, which is the one correct spelling.
- **`dir: down` makes a repeater a LIST.** Default `across`: each slot is
  content-measured and slots advance on x, which is right for a cluster of
  differently-sized widgets (the bar's center strip) and wrong for a card list.
  A card row is neither its text's width nor beside its neighbour, so `down`
  gives every slot the repeat's inner box and stacks them on y at `spacing`,
  with the row box `h` and the insets from `x` / `y` and a negative `w` span.
  This existed because the *only* vertical list was `Rows`, which is tabular
  (`SceneCol`s) — so a card with a bespoke body had **no declarative spelling at
  all** and had to stay a Rust drawer. Landing `notif` needed it; §11 has the
  rule it taught.
- **Pilot: the pill (landed).** Convert `pill_key_<tag>` / `pill_ink_<tag>`
  first. It was the messiest model in the tree (17 tags, three config-gated to
  zero width, which is what forced the `conditional` declarations), so proving
  the scope model here means it holds elsewhere.
- **Second landing: the notification cards.** The first *surface body* to go,
  and the first to need `dir`, a delegate `tpl`, and the `hover_color` swap
  (§11). Its parity oracle is `layout_notifs_body`, now `#[cfg(test)]`.
- **Third landing: the Settings Pill rows, where the delegate is itself a
  `Comp`.** `pill_switch` is declared once and inlined into three repeater
  templates — the strongest form of "the recipe is the same, so the state each
  row carries is the same". It also found the one thing a props scope must not
  do: see the scope rule in §11.
- **Fourth landing: Settings → Appearance, the sixth and last section.** The
  pane is fully declared and the `settings_ink` name is gone rather than stood
  down. It needed no new vocabulary, which is the actual result of the phases
  above — 27 controls, five sliders, a scrollable grid and a segmented switch
  all fitted the language the earlier landings had already extended. What it
  found instead were two rules about *data*: a model row is a slot (§11), and one
  box has one measure.

## 4. Rule 5a — components with parameters

**STATUS: LANDED.** The syntax below is the syntax that ships; `PropValue` /
`PropSet` in `scene.rs` are the implementation.

```ron
components: {
    "chip": (props: (w: 20.0, ink: fg, key: 0),
             items: [ Surface(w: "w", color: value("ink")),
                      Hit(key: "key", w: "w") ]),
},
surfaces: [ (name: "settings",
             items: [ Comp(name: "chip", props: (w: "pill_ws_reg_w", key: 50)) ]) ]
```

- `props` is a declared map of `name: default`. A use site overrides any subset;
  unspecified props take the default. **Defaults are the whole point** — that is
  how one declaration covers every call site.
- `Comp { name, x, y, props }`: `props` merges into the scope described in §2, so
  a component's body sees its props as ordinary names. They are scoped to the
  body: a sibling `Comp` of the same component keeps its own defaults.
- A component body may still be the bare `[ … ]` item list it has always been.
  That is the backwards-compatibility floor, and every pre-phase-2 scene in the
  repo is written that way.

### The three prop forms, and which channel reads which

`PropValue` is deliberately **not** `SceneValue`. A prop is written by a human in
a `props:` block, where `ink: acc` must mean "the accent token" — and no
`SceneValue` variant can say that without making the pool's own schema ambiguous
(`"x"` is a `Text` there and an expression here). So a prop is one of exactly
three things, and a body reads it through whichever channel it asked in:

| authored | channel that reads it | meaning |
| --- | --- | --- |
| `w: 20.0` | a `Val` (`w: "w"`) | the number `20.0` |
| `title: "Wi-Fi Networks"` | `{…}` text | the **literal** four-word string |
| `meta: "{pill_wifi_meta}"` | `{…}` text | braces mean *resolve this pool name* |
| `ink: fg` | `value("ink")` | the `fg` token |
| `ink: "pill_meta_col"` | `value("ink")` | a **bare** string in a `value()` slot means *the pool color with this name* |

Two of those five rows are the non-obvious ones, and both exist because a
component cannot hardcode something the shell alone knows:

- **`meta: "{name}"`** — braces are what bind. Without them `title: "Wi-Fi
  Networks"` would vanish, because no one publishes a property by that name. A
  prop is literal copy by default, which is the common case.
- **`ink: "name"`** — a component must be able to parameterize a color whose
  choice is per-frame state (`subhead`'s meta is the accent while the radio is
  up and `fg3` while it is down). The name is deferred to the pool, so a theme
  switch still lands and the validator still reports a typo as an unknown color
  property. `DynColor` caps the name at 24 bytes.

### Substitution is symbolic, which is the load-bearing detail

`w: "pill_ws_reg_w"` at a use site must stay an *expression*, not become this
frame's number. If it folded to a literal at load, the value would freeze into
the expanded scene and every later frame would repaint the first one's width — a
bug no single-frame test would catch. So `props` substitute through
`expr::subst` (textual) while a model row folds through `expr::fold` (numeric):
a prop may itself be an expression, a row field IS the frame's answer.

Arithmetic composes, because substitution is textual:
`w: "w - 4"` in the body with `w: "pill_w"` at the site becomes
`"pill_w - 4"`. A prop that substitutes to a bare number *is* re-typed to
`Val::Lit`, because `offset_item` and `Val::constant` only move and read
literals — a substituted number left as `Expr("4.0")` would be silently dropped
by the `Comp`'s own `x` / `y` translation.

A prop read through the **wrong** channel is left alone (`ink: acc` read as a
width stays the bare name `ink`, which the pool resolves 0) rather than
substituted wrong. The validator reports the type mismatch; a wrong `0` here
would draw silently.

### Dead props are reported, because a typo is invisible otherwise

`props: (wid: 99.0)` on a component that declares `w` draws *exactly* as if the
line were not written. Nothing in the pipeline can notice — the default still
substitutes — so `expand()` proves it instead, by **rendering** the body once per
prop and comparing against the body with no props. Rendering rather than walking
the body for name references is the point: a walk would be a second,
hand-maintained inventory of every field of every one of the ~30 item variants,
and the day it forgets one is the day a **live** prop reads as dead and the
author is told to delete it. `ShellScene::dead_props` carries the lines and
`get_validated` prints them.

### The engine refuses to be vague

Errors an author actually hits, all verified to name the offending thing:

- duplicate prop name → "duplicate prop `w`: a prop name may be given once"
  (not last-one-wins — the effective default must not depend on line order)
- unknown field → "unknown component field `itemz`: expected `props` or `items`"
- `(props: …)` with no `items` → names the omission
- `SceneComponent`'s codec is **hand-written** rather than `#[serde(untagged)]`
  for this reason: untagged reports every failure as "data did not match any
  variant", which would swallow all four of the messages above
- RON cannot deserialize a map into `Vec<(K, V)>` at all, which is why `PropSet`
  carries its own `Deserialize` (and serializes back as a map, so the round-trip
  tests hold)

### What landing this actually bought

`subhead` is the migrated call site, and it is the shape to look for. It read
four **shared** pool names (`sub_title` / `sub_meta` / `sub_meta_size` /
`sub_meta_col`), which forced a Rust `match self.mode` table to choose the
panel's title — UI copy, chosen by the renderer, which is exactly what §0
forbids. The title is now a `title:` prop each call site passes, so that table
is **deleted**, along with its dead `"Audio"` arm (no surface stamped a third
`subhead`). What Rust still publishes is only the per-frame state the scene
cannot know: the `"On"`/`"Off"` text and the accent-vs-`fg3` color, each behind
its own per-panel name (`pill_wifi_meta*`, `pill_bt_meta*`) so the scene file,
not Rust, says which panel reads what.

- **This is what retires the `pill_chip` / `ws_chip` / `hover_fill` item** in
  `NEXT_STEPS.md`: those are not "extract these three components" any more,
  they are "declare these three components with their props".
- **The pilot already proved the mechanism a different way.** The pill's
  `icon_chip` template is one template serving six widgets whose only difference
  is the row's `key` / `ink` / `glyph` / `w` — that is a component with props,
  written as a delegate. Rule 5a was therefore *smaller* than it looked: the
  engine work (a scope overlay, `scene/expr.rs`) was already done and reused.

## 5. Rule 5b — scene-owned, writable state

This is the reason 35 `Ink` cards remain. The scene tree is strictly read-only
today: `SceneAction` can only spawn a command string, so every stateful
behavior (strip editing, settings nav selection, scroll offsets, panel toggles)
has to live in a Rust drawer.

```ron
(name: "dash",
 stores: [ (name: "nav", props: (scroll: 0.0, editing: false)) ],
 items: [
     Rows(name: "ws_tiles", start: "nav.scroll"),
     Hit(key: 1, action: Set(name: "nav.editing", value: true)),
 ])
```

- **Stores are declared on the SURFACE, not as an item:** `SurfaceScene.stores:
  Vec<StoreDecl>`. An item was the wrong home — a store draws nothing, so it has
  no place in a layout list, and `SceneItem::Comp` inlining would have copied it
  into every template that used one.
- **New action:** `SceneAction::Set { name, value }` alongside `Command`.
- **A prop's type is fixed by its declaration** (`PropValue`: `Num`/`Bool`/
  `Expr`/`Color`). A write of a different type is REFUSED, not coerced —
  coercion is how a binding starts resolving to 0 forever with no other symptom.
- **A store is a closed set of names, and both directions are enforced:**
  - *write*: `ClickedAction` carries the surface, and `SceneStoreState::write`
    is keyed by it, so a `Set` can only reach the store its own surface
    declared. A surface that declares no such prop refuses the write.
  - *read*: every panel and surface draw builds its pool through
    `Shell::scene_values_for(surface)`, which merges ONLY that surface's
    stores. `Shell::scene_values()` (the whole-world pool) is the validator's
    view, not a draw path.
- **Legality is decided against the DECLARATION, not the live value.** Seeding
  is idempotent and non-destructive (a `Set` must survive the next frame's
  seed), so the live map cannot answer "is this prop allowed to exist" — an
  unseeded store would accept any name. `SceneStoreState` keeps declarations
  separately for exactly this.
- **The closed set is what the validator checks** (see §6's validator half, which
  landed with this phase). A store's names cannot
  be absent-this-frame the way a flat-pool name can, so `Set` to an undeclared
  prop and a `<store>.<prop>` binding typo are both load errors, not runtime
  surprises.
- **Does not require §6.** A write lands in state that outlives the frame; the
  next frame re-resolves against it. Phase 3's measurement is why that is enough
  here — and its validator half is what folded into this phase.

## 6. Rule 3 — change propagation (makes §5 affordable)

Quickshell re-evaluates a binding only when a dependency changes. Two passes:

1. **Dirty names.** The publisher diffs what it wrote this frame against the
   previous frame; changed names go in a dirty set. `SceneValues::insert*` is
   the natural place to compare.
2. **Re-resolve only what reads a dirty name.** `Expr::deps` gives each binding's
   name set, so a changed `pill_gap` invalidates the bindings that read it and
   nothing else. Layout becomes a dirty-subtree pass instead of a full walk.

Same pass retires the standing `NEXT_STEPS` item: **cache the compiled
expression** (`#[serde(skip)]` + `OnceCell` on `Val::Expr`), so a clean frame
allocates nothing.

This is also what lets the validator check **card-local** names. Today the
validator only asks pools marked `complete` (`Shell::scene_values()` is; a
card's local audiorec pool is not), because a partial pool cannot answer "does
anyone publish this?". A store/delegate scope is a *closed* set of names — the
validator knows every name in it — so the question becomes answerable locally.

## 7. Rule 2 — implicit property inheritance

The long one, and the least urgent: `draw_piece` threads a box through every
call, which is why "centered text in a container" and "right-anchored text in a
container" are documented gotchas rather than natural expressions. A scope-based
`parent` (`w: "parent.w - 28"`, `x: "parent.cx"`) plus implicit inheritance
deletes that class of bug. **Do this after §6** — it is a large diff across every
draw arm and it buys nothing while the tree is re-resolved wholesale anyway.

---

## 8. Extras from Quickshell worth copying, in priority order

| Feature | RON | Notes |
|---|---|---|
| **`anim:` on any field** — landed, phase 5 | `x: (val: "nav.tab", anim: (ms: 90))` | The engine keeps **one current value per binding source string** (not per item path: two items reading `nav.tab` must not disagree mid-move) and eases toward the bound target. Only *position* reads animate — `w` / `h` / `font_size` are refused by the validator, because measure reads the target. See the corrections in §10. |
| **Component validation** | — | A declared component whose props are never overridden, or a prop default nobody reads, is dead vocabulary. The `WALKED_VAL_FIELDS` sweep is the precedent — extend it. |
| **`Connections`** | `on: [(when: "battery.charging", set: ("dash.ink", "info"))]` | Reacting to *data* rather than clicks. Mostly subsumed by §5's store. |
| **Per-property types** | `w: f32`, `ink: color` | RON + serde already gives parse-time type errors; a declared type per prop would let the validator reject `ink: 4.0` at load instead of drawing nothing. |
| **Dev-time introspection** | `debug: true` on a surface | Dump every item's resolved `x/y/w/h` and its binding's value to the log. The `report()` machinery from the validator is the model. |

## 9. What we are deliberately NOT copying

Note what is *not* on this list: Rust. §0 keeps the backend and the renderer in
Rust permanently, and nothing here proposes moving them out. "Declarative" is
about **who decides**, not about what language runs.

- **A free-running render loop.** The shell is event-driven and idles at 0%
  CPU; Quickshell's `always`-running bindings are why its dirty tracking is
  sound and ours must be too. Rules 3's propagation is required *because* there
  is no loop to hide the cost.
- **A general-purpose language.** No user-defined functions in a scene, no
  stringly-typed `var`. The shell's publishers stay Rust — and so does every
  algorithm that is genuinely one (FFT, fluid drag, image decode). §0's
  "irreducible algorithms" carve-out is the same boundary, stated per-item.
- **Qt-style attached properties / `Setter`.** `Value("pw_ink")` already covers
  the state-driven color case in one field.

---

## 10. Build order

| Phase | Work | Deletes | Done when |
|---|---|---|---|
| **0** (landed 2026-10-01) | Load-time validation | silent typos | 435/435 |
| **1** (landed 2026-10-01) | Scope overlay + `item`/`index` in `Repeat` delegates; pilot on the pill | `pill_key_<tag>` / `pill_ink_<tag>` and the `conditional` declarations they forced | **done** — the pill's center cluster is one `Repeat` over the `pill_ctr` model in 8 templates; `pill_item_width`'s zero-width gate is a row's absence, not a publishing rule; `item.<field>` is a validated name; 449/449 |
| **2** (landed 2026-10-01) | Component `props` with defaults | `pill_chip` / `ws_chip` / `hover_fill` as copy-paste | **done** — `subhead` is parameterized (`title:` / `meta:` / `meta_size:` / `ink:`), which deleted the Rust `match self.mode` title table outright; props substitute *symbolically* so an expression prop keeps tracking the pool; a dead prop is proved by re-rendering the body, not by walking it; 468/468 |
| **3** (demoted 2026-10-01) | Dirty-set propagation + cached expression trees | the per-frame re-resolve cost; makes §5 viable | **not a gate on 4** — measured, it would save ~5 µs of a 16,666 µs frame. What survives is the *validator* half: a closed-name scope lets card-local pools be checked. **landed with 4** (2026-10-01): the store closed-set check |
| **4** (landed 2026-10-01) | `store` + `SceneAction::Set` | Rust drawers that exist only to hold state | **done** — settings nav selection is a `nav.tab` store prop written by the `settings_nav` rows' `Set`, with the Rust `match key { 200 => … }` table demoted to a fallback below the scene-action arm; reads are surface-scoped through `scene_values_for`; a store's closed name set is load-checked; 489/489 |
| **5** (landed 2026-10-02) | `anim:` on any field | hand-rolled settle/lerp state in drawers | **done for what an item `anim:` can reach** — the polkit password cursor glides (`shell.ron`: `x: (val: "polkit_cursor_x", anim: (ms: 90))`). The two settles this row originally named are **not** reachable and stay in Rust (see below); 508/508 |
| **6** (landed 2026-10-02) | Implicit `parent` scope | positional box threading + its gotcha list | **done** — `parent.w` / `.h` / `.cx` / `.cy` resolve against the box an item is already drawn in, in base px; a child no longer restates its container's width; the power menu's five 92 px chips now say `w: "parent.w - 4"` instead of hardcoding 88; 524/524 |
| **7** | Retire `Mode` + `Ink` | the last Rust draw paths | `Ink()` count is 0; every `layout_*` is a test fixture, not a draw call |

**Phase 3 was demoted after measurement** (see the correction in §2), so what
remains is 4 → 5 → 6 → 7, with 3's validator half folded into 4. Phase 4 is also
where the *real* per-frame cost becomes visible: not expression evaluation, but
`scene_values()`'s 594-insert pool rebuild.

**Phase 4's cost was not the store — it was deciding who owns the name.** Three
decisions that were not obvious in advance, and are worth not rediscovering:

1. **A store is a property of the scene, so legality must be decided against
   the declaration, not the live value.** The live map is empty until the store
   is seeded, and seeding is idempotent *by design* (a `Set` has to survive the
   next frame). A runtime check that read the live value would therefore accept
   any name on an unseeded store — the refusal would depend on load order, and
   the bug would only ever appear on the one frame where the two disagree.
   `SceneStoreState` keeps `decls` beside `by_surface` for this.
2. **"A dotted name is a scope name, skip it" was a real validator hole, and it
   hid a real one.** The walker used to skip every dotted name, because a pool
   cannot judge a scope. But a *store* IS a closed set, so it can. The fix keeps
   the skip for `item.<field>` (the row-shape check owns that one, and owns it
   better) and for namespaces the validator cannot see (`mod_clock.time`), and
   reports only `<declared store>.<undeclared prop>`.
3. **`scene_values()` was merging every surface's stores into one pool.** Names
   are namespaced, and a namespace nobody enforces is a convention: two surfaces
   naming a store `nav` silently shared one value. The fix was not a check but a
   change of shape — every draw path now builds its pool through
   `scene_values_for(surface)`, and the whole-world pool is marked
   `#[cfg(test)]` plus the validator's view. It cost one pool clone per surface
   per frame, which is a memcpy of a map that was already being rebuilt.

**A literal scales, a device constant does not — and `gap_var` / `r_var` are
the escape from picking one.** Every geometry number in a scene is base px and
multiplied by the scale on the way out. That is right for anything the Rust
painter also scaled, and wrong for anything it did not. Converting the wallpaper
picker ran into two such numbers, both of which were invisible at scale 1.0 and
visibly wrong at 1.25:

- `WP_GAP` — the 10 px tile pitch. The painter added it raw while scaling the
  tile radius and the thumbnail inset. Declared as `gap: 10.0` it agreed at
  1.0 and drifted 7.5 px per gap at 1.75;
- the scrollbar pill radius, which is `tw / 2` on a 6 px **device** track.
  `r: 3.0` scaled to 3.75 and made every track and thumb subtly oval.

So `Grid.gap_var` and `Surface.r_var` exist: a bound value replaces the literal,
is still base px, and falls back to the literal when the name does not resolve.
The general rule they encode — worth applying to any future body that mixed
units — is that **the publisher divides by `scale.s(1.0)` and the scene
multiplies back**, so the unit question is answered once, in the place that
actually knew the answer, rather than being re-guessed per literal. Two tests
pin the pair (`a_grid_gap_var_...`, `a_surface_r_var_...`) and each asserts the
agreement at 1.0 alongside the disagreement at 1.75, because the agreement is
the reason the bug survived review.

**A windowed model's keys are ABSOLUTE, and the click band must cover the whole
collection.** `start` slides which rows a `Grid` shows, but a cell keeps
`key_base + its own index in the full model` — a tile is numbered by the thing
it shows, not by where it sits on the page. The wallpaper picker did the
opposite (`10 + position-in-window`, with `wallpaper_scroll` added back in the
handler), which meant a stale scroll — clamped by the painter, never by the
handler — resolved a different file than the tile under the pointer.

Making the keys absolute has a second consequence that is easy to miss: **the
band has to be as wide as the collection, not as wide as a page.** The old
`10..=99` was exactly a page wide *because* the keys were relative. Left alone
after the change it capped the grid at 90 files and every tile below index 89
fell through to `_`, collapsing the panel on click — silently, and only once
the user had scrolled there. Hence `WP_PICKER_KEY_BASE` / `WP_PICKER_KEY_MAX`
and the card's existing `WP_CARD_KEY_BASE` / `WP_CARD_KEY_MAX`. When a key range
and a windowing model meet, ask which one the range was sized for.

**Phase 5's "done when" was wrong when it was written, and correcting it is the
useful part.** It read "the pill expand and `dash_settling` are declared".
Neither is a field easing toward a bound target, so `anim:` cannot express
either, and both stay in Rust:

- the **pill expand** is *surface* geometry — `Shell::anim` →
  `anim::{from_w, to_w, …}` → `cur_w` / `cur_h` morphs the window itself. No
  item in a scene owns it, so there is nothing for a field-level wrapper to
  wrap. Declaring it would take a *surface*-level `anim:` instead — a
  different feature, on the surface's own box;
- the **dashboard glide** is computed inside the Rust `dash_grid` ink off
  `packed_layout`: a `card_anim` lerp toward the packed slot with an
  `edit_settling` flag existing only to keep the render loop awake until it
  converges. The position it eases is *derived from layout*, not bound to a
  published property — so there is no binding for the table to key on.

What phase 5 *did* buy is the small-motion case, which was the majority of what
the hand-rolled lerps were for: a cursor stepping one glyph per keystroke is now
a 90 ms glide declared in `shell.ron`, and the loop that keeps the bar awake
while it moves (`App::anim_timer`) is the declarative replacement for
`edit_settling`. Three decisions were not obvious in advance, and are worth not
rediscovering:

1. **Key the table by binding SOURCE, not by item path.** Two items reading
   `nav.tab` are tracking one value and must not disagree mid-move; inside a
   `Repeat`, substitution has already rewritten each row's source to its own
   `item.<field>` spelling, so rows come out distinct without threading a path
   through `draw_piece`.
2. **The first frame of a key rests AT its target.** Only *subsequent* target
   changes animate, so `Entry` keeps `target` while settled — otherwise every
   frame looks like a first sighting and nothing ever moves. A retarget restarts
   from the current value, so an interrupted animation never snaps backwards.
3. **Measure must read the target, and an animated extent must be REFUSED
   rather than ignored.** An animated `w` that measured its in-flight width
   would resize the box positioning it, and the two would chase each other
   forever. But the honest alternative to fixing that is not to leave `anim:` on
   `w` inert — it is `LoadIssue::AnimatedMeasuredField`, so the schema cannot
   accept an animation the renderer will silently drop.

**Phase 6 is the first phase that pays per frame rather than per declaration,
and its "done when" was too weak.** It read "centered/right-anchored text needs
no container workaround", which is already true of `center:`/`right:` — those
existed. What was actually missing is that a **binding could not ask what box
it was in**, so the only way to size a child against its container was to
restate the container's number, and every restatement was a place for the two
to drift. Six decisions were not obvious in advance:

1. **The scope is four names, and they are base px.** `parent.w` / `parent.h` /
   `parent.cx` / `parent.cy` — no `parent.x` / `parent.y`, no grandparent, no
   `parent` alone. `cx` / `cy` are the *arithmetic* spelling of "centered":
   they are an offset from the parent's left/top edge to its midpoint, which is
   what `center: true` computes internally and could not be reached from a
   scene. Base px (not device px) because that is what every other authored
   length in a `.ron` is, so `x: "parent.cx"` and `x: 40.0` are the same kind of
   number; the scale factor is stored beside the value and applied on read.
2. **The scope wins over the pool, and `parent.` is RESERVED.** A publisher that
   wanted to publish `parent.w` would silently change the meaning of every
   `Row` in the tree, so the namespace is claimed by the engine rather than
   left to whoever writes the next panel. This is also why the pool's
   completeness gate never sees these names: a scope is a **closed** set, so the
   validator answers "is this a field?" on its own terms and reports
   `UnknownParentField` with the real names in the message. That check is
   `parent_name()`, shared by expression bindings and bare `*_var` fields,
   because two copies would let the validator pass a name the runtime resolves
   to 0 — and a `w` of 0 is a box with no area, not a visible failure.
3. **`parent.*` in a `Repeat` template is answered only when the slot has a
   box at measure time — and it does for a SPREAD slot.** A content-packed
   (`auto: true`) template is refused (`ParentInRepeat`, with the message
   naming the `item.<field>` way out): a packed slot's width is *the sum of
   its template's content*, so asking the template what its parent is asks for
   the answer to the sum it is a term of — `parent.w` there has no measure-time
   value. A **spread** (`across`, no `auto`) slot is measured under the equal
   share it is later painted in, so `parent.w` / `parent.cx` / `parent.h` are
   real names there (the ten EQ band columns are `avail / 10` wide and each
   asks `parent.cx` for its groove center). The two boxes agree exactly when
   the slots measure alike — one template whose rows differ only in values,
   `spacing: 0` — and the EQ parity test pins that the sheet and drawer still
   agree sheet-wide. A `down` list never measures its template content at all
   (each row is the inner box), so its `parent.*` answers at draw.
   Defaulting to 0 in the packed case would have drawn a zero-width widget;
   refusing is honest, where the bar's own widgets all use `item.<field>`.
4. **A bound extent is PINNED to the measurement that made its slot.** This is
   the one that was a real bug rather than a design choice. A container turns a
   child's own `w` / `h` into that child's SLOT, and the slot is the box the
   child is then drawn in — so a binding on those axes is read **twice per
   frame**, once by `box_w` against the container's box and once by `span_v`
   against the slot the first read produced. Every binding that existed before
   read the same number both times (a positive extent ignores its parent, a zero
   fills it), which is exactly why the double read had been invisible. `w:
   "parent.w - 20"` is the first binding that does not: 164 against a pad-8
   row's 184-wide inner box, and 144 against the 164-wide slot measurement then
   created — the item would **reserve** 164 and **paint** 144, in a row whose
   other children were sized against the 164. `SceneItem::pinned` freezes a
   BOUND axis to what was measured, which keeps "a measured width can never
   disagree with a drawn one" true for a binding that depends on the box at all.
   The alternative — threading the measured value into `draw_piece` as the
   parent for bound axes — was rejected because it would hand a child whose own
   extent IS a fraction of its parent a *different* box than its unmeasured
   sibling, for no gain. Pinning is the smaller change and keeps every sibling
   in a row on one box. A `Literal` axis is left exactly as declared, because
   `w: 0` is the fill idiom and `fills_w` answers from the declaration.
5. **`Stack` needs no pinning, and that is a fact about it, not luck.** A
   `Stack` has no measure pass: every child is handed the container's own box,
   once. So the double read of point 4 is a `Row` / `Column` property, and the
   live migration landed on `Stack` shapes (the power menu's chips, the pill's
   two toggle chips) where the question cannot arise.
6. **The migration is proved by the draw stream, not by the validator.** The
   validator only proves the names are legal. For parity, every surface of the
   live `shell.ron` was rendered through the full draw pass at three box sizes
   and the entire `Cmd` stream + hit regions diffed before and after: 342 lines,
   byte-identical. So the restated numbers became `parent.*` with a measured
   guarantee rather than an eyeballed one.

What phase 6 *bought* is the shape of a scene's geometry becoming relative
rather than duplicated. The power menu is five identical chips; before, each
carried its own `92` and `88`; now the `92` appears once per chip and the `88`
is written as the subtraction it always was. That is the same argument §0 makes
about state, applied to coordinates: a number that is restated is a number that
can drift.

`Val`'s `Deserialize` is hand-written for the two reasons phase 2 established
(§4): untagged's error message is useless, and `anim: ()`
does not match it at all. Note that the *original* blocker recorded for this
phase — untagged dropping `curve:` — was measured against ron 0.9 and does not
reproduce on the pinned 0.12.2; a measurement taken on a version the crate no
longer pins is worth re-running before it is believed twice. `untagged` stays on
`Serialize`, which is the half that still relies on it.

Phase 1 shipped with no other scene file changed, and phase 2 needed only the
`props` syntax on top of the overlay phase 1 built — so 1 and 2 together cost
roughly what 1 alone did. What 2 cost *beyond* syntax was three decisions that
were not obvious in advance, and they are worth not rediscovering:

- a prop cannot be a `SceneValue` (see §4) — the two schemas want opposite
  things out of a bare `acc` and a bare `"x"`;
- substitution must be **symbolic**, not numeric, or an expression prop freezes
  the first frame's value into the expanded scene;
- RON cannot deserialize a map into `Vec<(K, V)>`, and `#[serde(untagged)]`
  swallows every real error message — so both codecs are hand-written.

`subhead` also showed the *shape* to migrate: look for a Rust `match` that picks
UI copy or a color per panel. That table is a component with props wearing a
disguise. Phases 3–6 are each a bigger bet; 3 gates 4, and 4 is
what §0's boundary actually requires. Phase 7 is not a design phase at all — it
is the sweep that *verifies* §0, and it only becomes reachable once 4 has moved
the state out of Rust. Do not start it early: without `store`, "retire `Mode`"
means reimplementing 31 panel bodies in Rust-valued state instead of
scene-owned state, which is the same work twice.

---

## 11. Rules earned while building this

- **A binding may ask what box it is in.** Any value a container computes for
  its children (`inner_w`, `inner_h`, the center line) is derivable by a child
  that can read `parent.*`, and a value a child has to be *told* is a value the
  two can disagree about. When a scene restates a container's width inside the
  container, the fix is not to re-check the number — it is to name the box.
- **A scope is a CLOSED set, so it is checkable where a pool is not.** This is
  the same property phase 4 found for stores, and it is why `parent.*` survives
  a partial pool (§4 decision 3) and can report a misspelled field on a card
  whose pool publishes three local values. It also costs almost nothing: four
  names, one lookup.
- **Refuse, do not default, when a binding has no answer.** `parent.*` in a
  CONTENT-PACKED (`auto: true`) `Repeat` template has no measure-time value; 0
  would have drawn a zero-width widget that looks like a bug in the layout
  rather than a refusal in the schema. A spread slot's `parent.*` is NOT the
  no-answer case — the engine measures its template under the equal share it
  paints that slot in, so the ten EQ band columns read `parent.cx` / `parent.h`
  without the card's number being restated in RON.
- **A measured width must never disagree with a drawn one.** When a value is
  read in two passes per frame, prove they agree — for every binding that
  existed before the second reader, they agreed for a reason (`Literal` ignores
  its parent; a zero fills it), and that reason is what made the duplication
  safe until it wasn't.
- **A model row must carry fields, not tags.** A `Vec<String>` model forces the
  publisher to invent a parallel name per field per row, and every one of those
  names is a new silent-failure surface.
- **Conditional publication is a smell, not a feature.** It exists because a
  model cannot say "this row is absent today". `conditional` on
  `SceneValues` should shrink to zero as models arrive — if a new one appears,
  the publisher is flattening something it should not. (Phase 1 removed the
  pill's loop outright: a row that exists *is* a widget that is on.)
- **A component parameter is a name in a scope, not a field on the item.**
  Anything else needs the `Comp` variant to grow a field per parameter.
- **The row picks the delegate; the repeater does not.** Quickshell's
  `Repeater { delegate: Y }` is one body per repeater, which is why a repeater
  whose rows are of several kinds has to be split into several repeaters. Letting
  the row name its template's `tag` as a field (`tpl`) keeps one repeater per
  *strip* and lets identity (`tag`) and body (`tpl`) differ — 14 pill templates
  became 8, and a new widget that matches a body needs no template at all.
  The price is that an unmatched `tpl` now draws *nothing* instead of raising,
  so the check for it has to be a test that walks every widget against the live
  scene, not a compile error.
- **A scope name must be checkable, or the scope is only a runtime feature.**
  Adding `item.w` bought nothing until `item.<field>` was validated against the
  model's rows — otherwise the new expressive power came with a *larger* silent
  -failure surface than the name families it replaced. Same for
  `SceneItem::scope_names`: a validator walk that cannot see a name the evaluator
  reads is worse than no check, because it reports the tree as clean.
- **The measure pass and the draw pass must instantiate identically.** A
  `RepeatSlot` owns its instantiated items rather than borrowing the template, so
  one function (`repeat_slots`) produces the geometry both passes see. The
  earlier bug class — "a measured width disagrees with a drawn one" — is
  structurally gone rather than tested away.
- **A sweep is a test, not a habit.** `WALKED_VAL_FIELDS` and the live audit
  exist because "the schema grew and the engine did not" is invisible until a
  card draws wrong. Any new vocabulary (models, props, stores) gets the same
  treatment on day one.
- **A boundary you cannot count is not a boundary.** "Rust stays a backend" is
  only enforceable if something measures it, which is why §0 names the metrics
  (`Ink()` → 0, no `layout_*` in the draw path) rather than leaving the
  architecture as an intention. The same reasoning as the row-dispatch rule
  above: a rule with no check is a comment.
- **Convert a drawer, then keep it as the test fixture.** `layout_collapsed` is
  493 lines of Rust that no longer participates in drawing, and that is the
  single most valuable thing in the tree — it is what makes the `pill` surface's
  correctness a *comparison* rather than a claim. Do not delete a drawer on
  conversion; demote it. `layout_notifs_body` and the `notif` fallback are the
  same move one phase on: `#[cfg(test)]` is what makes "demoted" real, since
  the oracle can then only disagree with the scene in CI, never at runtime.
- **A gate and a swap are different verbs, and only having one of them is a
  trap.** `Surface.hover_key` originally meant *paint this fill only while that
  key is live* — a chip's hover wash on a box that is not its hit. A card row is
  the other case: always painted, *brighter* when hovered. With only the gate
  available, the only way to write a card was to publish its fill color per row
  from Rust's live hover key — putting a presentation decision back in the
  publisher, which is the thing this whole migration exists to remove. The fix
  was a second field (`hover_color`), not a cleverer encoding: `hover_key` alone
  stays a gate, `hover_key` + `hover_color` is a swap, and no existing surface
  changes meaning. **A primitive that can only express half of a common case
  does not get worked around — it gets completed.**
- **A row's `tpl` is load-bearing, so the check for it must not be per-widget.**
  An unmatched `tpl` draws *nothing* — silently, because the alternative (a
  guessed template) is worse. §3's rule was covered by a test walking every pill
  widget, which is fine while one model exists and quietly stops covering the
  moment a second model lands: `notif_rows` shipped with no `tpl` at all and
  every card vanished, and the *only* thing that caught it was the parity
  differential. Prefer the differential (it is stronger), but a named coverage
  test per model is the floor, not the ceiling.
- **An engine gap is not a reason to publish pixels.** `notif` wanted a card that
  spans its surface; a repeat slot was content-measured, so the honest-looking
  workaround was to publish the card width from Rust (`item.w`, as `pill_ctr`
  already does for the bar). That is the anti-goal wearing a type-safe costume,
  and it would have put the geometry back where the migration is taking it from.
  The right move was to add the missing capability (`dir: down`) — the gap was
  in the *language*, not in the caller.
- **A primitive's HOVER behaviour is part of its contract, and matching the
  resting state is not matching it.** The Settings Misc body is one slider, and
  `ui::slider` turned out to be `Fader` almost exactly — same 4 px groove, same
  accent fill, same 12 → 14 px knob growth, same left-edge fill — except that
  its groove *changes color* under hover (`mix(bg, fg, .10)` → `.16`). A parity
  test that only hovered the row's *other* elements would have called that a
  match and shipped a slider that stopped responding to hover on the one pixel
  band the user aims at. The fix was `Fader.track_hover`, the same shape as
  `Surface.hover_color` from the rule above: a second field, not a republish.
  **When a drawer becomes a scene item, diff the drawer's hover branches
  first — they are the part most likely to have no declarative spelling, and
  the part a resting-state comparison cannot see.**
- **A load-time fold must not answer for a draw-time scope.** `inflate` folds a
  component body's props at LOAD, and `substitute_scope` used to drop every
  dotted name its scope could not answer — which is right for a row scope (a
  dotted name can only have come from a row, so an unanswered one is dead) and
  catastrophic for a props one, because the row that will instantiate the body
  does not exist yet. So a shared row delegate behind a repeater had its
  `{item.glyph}` erased at load: every row drew a blank icon and a blank label,
  in the shipped binary, with no diagnostic anywhere. **A scope answers only the
  names it owns and defers the rest; a name is only "dead" when the scope that
  was supposed to bind it has already run.**
- **Convert a routed body one section at a time, and ratchet the seam.** The
  settings pane reached its six sections through ONE interpolated Ink name
  (`settings_ink`), so converting any single section meant two files had to
  change together: a declared `tab_*` section in `shell.ron`, and that tab's
  name leaving the publisher's `match`. Get one right and you have either a
  pane painted twice or a blank pane, and neither is caught by the per-section
  parity test, which only ever looks at one section. Two decisions made the
  seam safe. First, **an empty Ink name is a silent no-op**, so the publisher
  could express "this tab is declared now" as `""` and the dispatcher could not
  double-draw or warn. Second, `every_settings_tab_has_exactly_one_body`
  asserted `open declared sections + non-empty Ink name == 1` for **every tab** —
  the one test that made the incremental conversion safe, because it is the
  only one that looks across sections. **When retiring a dispatcher one arm at
  a time, the invariant is "exactly one owner", and it wants its own test.**
  When the LAST arm lands, that test has to **invert** rather than just keep
  passing: "exactly one owner" is now "exactly one declared owner", so it
  asserts `open == 1` *and* that the retired name is unpublished. A binding no
  tab routes to is dead code with a dispatcher still attached, and that is
  precisely what a future contributor would reuse.
- **A model row is a SLOT.** In a `Repeat`, one model row instantiates one
  template. Publishing a 4-column grid row as *one* row of four sets of fields
  therefore produced a single slot whose four cells drew on top of each other —
  and the popover's cells stacked vertically at the row pitch. **One cell per
  row.** The follow-on trap is the measure: an `across` Repeat sizes each slot
  by its widest child, so the 46 px hover ring and the 48 px hit inside the cell
  stretched it and broke the 52 px column pitch. Neither bug shows up on the
  first cell, and both show up on every one after it. **A cell's declared width
  is load-bearing — wrap it in a `Stack` whose `w` is the cell, not the widest
  thing inside it.**
- **One box, one measure.** Layout that depends on a runtime width must not
  re-derive that width: the popover's column count is `floor(roww / 52)`, and
  the publisher computed it from `target_size()` while the drawer used the width
  it was handed. Both were "the pane width" and they were the same number in
  production, so nothing failed until the parity test drove two widths at once —
  and then the scene and the drawer disagreed about the popover's shape. The
  publisher now reads a `settings_pane_w` that `layout_settings` stamps from the
  width it receives. **If a value is a property of the thing being drawn rather
  than of shell state, carry it down from the draw call instead of deriving it
  twice.**
