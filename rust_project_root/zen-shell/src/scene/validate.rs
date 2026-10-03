//! Load-time scene validation — the diagnostics that used to be dead code.
//!
//! Every silent-wrong-draw bug the declarative engine has shipped came from
//! the same shape: **the scene asked for something nobody supplies, and the
//! engine answered with a 0** instead of an error. A `Val` binding whose name
//! is misspelled `num`s to 0 (by design — a binding must never fail a frame),
//! so a typo'd `w: "pill_chip_widht"` painted a zero-width chip. An unknown
//! `visible` gate is always-open (`gate_open`), so a pane that never gated
//! anything looked correct. Two `Hit`s with the same key gave the click to
//! whichever registered last. `anchors` that do not fit their parent gave a box
//! with negative area. None of them throw, none of them log: they draw.
//!
//! So the check runs at LOAD, once per file, against the property pool the
//! scene will actually be drawn with — and prints. The rules it enforces:
//!
//! | complaint | what it catches |
//! |---|---|
//! | [`LoadIssue::BadBinding`] | a binding that does not parse (`Val::compile`) |
//! | [`LoadIssue::UnknownProperty`] | a binding naming a property nobody publishes (`Val::deps`) |
//! | [`LoadIssue::KeyCollision`] | two `Hit`s whose keys are provably equal |
//! | [`LoadIssue::InvertedAnchors`] | `anchors` that resolve to a box with no area |
//!
//! Two of the four are only provable against a *complete* pool
//! ([`SceneValues::is_complete`]); the other two need nothing but the scene.
//! The property checks are skipped — never faked — when the caller hands over a
//! partial pool, because a card that publishes three values of its own cannot
//! testify about the rest of the shell.

use super::{declared_kind, store_write_type_ok, value_kind, SceneItem, SceneValues, ShellScene};

/// One load-time complaint, carrying enough context to find it in the `.ron`
/// by eye. Deliberately a data type and not a log line: the tests assert on it,
/// and a future `zen-shell scenes check` would want the same structure.
#[derive(Clone, Debug, PartialEq)]
pub enum LoadIssue {
    /// A `Val` binding that does not compile. [`super::Val::num`] turns this
    /// into 0, so the field silently becomes "no width", "no key", "x = 0".
    BadBinding {
        /// Where in the tree: `items[4]`, or `items[2]/items[0]` for a nested
        /// child, plus the field name — `items[2].Hit.w`.
        path: String,
        /// The binding source as authored, for the copy-paste.
        src: String,
        /// The parser's message (includes the offending offset).
        err: String,
    },
    /// A binding, gate or name-property naming a property the pool does not
    /// publish. Also a 0 (`Val::num`) or an always-open gate (`gate_open`).
    ///
    /// `kind` is which kind of binding it was, because the fix differs: a
    /// misspelled scalar in `x`, or a misspelled `visible` toggle.
    UnknownProperty {
        path: String,
        src: String,
        name: String,
        /// `binding` = a `Val` expression, `visible` = a `visible` gate,
        /// `property` = a `*_var` / model-name field.
        kind: &'static str,
    },
    /// Two `Hit`s in one scene whose keys resolve to the SAME value, so one
    /// silently wins every click aimed at the other.
    KeyCollision {
        /// The key they share, as authored (`"4090"` or a binding source).
        key: String,
        first: String,
        second: String,
    },
    /// `anchors` whose inset pair leaves no area (or flips inside out) inside
    /// a parent whose size the scene itself declares — provable at load, with
    /// no frame and no runtime card size.
    InvertedAnchors {
        path: String,
        /// The arithmetic, e.g. `left 40 + right 2 > parent width 36`.
        detail: String,
    },
    /// A `Repeat` delegate binding `item.<field>` that NO row of the bound
    /// model carries. This is the one "nobody publishes this?" question a
    /// scope makes answerable: a model's rows are a CLOSED set of names, so
    /// unlike the flat pool the engine can say for certain that the field is
    /// missing rather than merely absent this frame.
    ///
    /// It is a 0 at draw, and the failure is the worst kind — a chip that
    /// draws with no width, or no key, and is silently unclickable.
    UnknownRowField {
        path: String,
        /// The scope name as authored (`"item.ink"`).
        name: String,
        /// The model's name, for the copy-paste (`"pill_ctr"`).
        model: String,
        /// The fields the rows DO carry, so the fix is a typo away.
        have: Vec<String>,
    },
    /// A `Val` binding a `<store>.<prop>` name this surface's stores do not
    /// declare. The surface's stores are a closed set of names, so — unlike the
    /// flat pool, where a name may simply be absent this frame — the engine
    /// can say for certain the name is a typo, and can list what IS there.
    UnknownStoreProp {
        path: String,
        /// The scope name as authored (`"nav.scrol"`).
        name: String,
        /// The store namespace (`"nav"`), for the copy-paste.
        store: String,
        /// The props that store DOES declare, so the fix is a typo away.
        have: Vec<String>,
    },
    /// A `Set` writing a name no store in this surface declares, or writing it
    /// with the WRONG TYPE.
    ///
    /// This is the phase-3 survivor: a store's props are a CLOSED set of names
    /// (the surface declares every one of them), so unlike the flat pool the
    /// engine can say for certain a name is missing rather than merely absent
    /// this frame. Which matters because the failure is otherwise invisible —
    /// [`super::SceneStoreState::write`] refuses an unknown name, and a refused
    /// click looks exactly like a click that did nothing.
    BadStoreWrite {
        path: String,
        /// The dotted name as authored (`"dash.scrol"`).
        name: String,
        /// Why it was refused.
        detail: String,
    },
    /// `anim:` on a field the measure pass reads. Measure resolves the TARGET
    /// (see [`crate::scene::anim`]), so an animated extent is drawn at its
    /// target every frame and never moves — the `anim:` is accepted by the
    /// parser and then does nothing, which is the same silent no-op as a
    /// misspelled binding.
    AnimatedMeasuredField {
        /// Where in the tree, plus the field — `items[2].Text.w`.
        path: String,
        /// Why this axis cannot animate.
        detail: String,
    },
}

impl std::fmt::Display for LoadIssue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadIssue::BadBinding { path, src, err } => write!(
                f,
                "{path}: binding {src:?} does not compile ({err}) — it evaluates to 0, \
                 so this field draws nothing"
            ),
            LoadIssue::UnknownProperty {
                path,
                src,
                name,
                kind,
            } => write!(
                f,
                "{path}: {kind} {src:?} reads {name:?}, which nobody publishes — it resolves to 0"
            ),
            LoadIssue::KeyCollision { key, first, second } => write!(
                f,
                "{first} and {second} both register key {key} — whichever draws last \
                 takes every click aimed at the other"
            ),
            LoadIssue::InvertedAnchors { path, detail } => write!(
                f,
                "{path}: anchors invert ({detail}) — the region has no area, so the \
                 click target is unreachable"
            ),
            LoadIssue::UnknownRowField {
                path,
                name,
                model,
                have,
            } => write!(
                f,
                "{path}: {name:?} is not a field of model {model:?} (it has: {}) — \
                 it folds to 0, so the field it drives draws nothing",
                have.join(", ")
            ),
            LoadIssue::UnknownStoreProp {
                path,
                name,
                store,
                have,
            } => write!(
                f,
                "{path}: {name:?} is not a prop of store {store:?} (it has: {}) — \
                 it resolves to 0, so the field it drives draws nothing",
                have.join(", ")
            ),
            LoadIssue::BadStoreWrite { path, name, detail } => {
                write!(f, "{path}: Set {name:?} {detail} — the click does nothing")
            }
            LoadIssue::AnimatedMeasuredField { path, detail } => {
                write!(f, "{path}: `anim:` {detail}")
            }
        }
    }
}

/// Validate a card scene's items against the pool they will draw with.
pub fn validate_items(items: &[SceneItem], vals: &SceneValues) -> Vec<LoadIssue> {
    let mut v = Validator::new(vals);
    // top-level items are laid out against the live card box, whose size no
    // scene file knows, so the anchor check has no parent extent to compare
    // against here — it runs for nested items only.
    v.walk(items, "items", None);
    v.finish()
}

/// Validate every surface of a whole-shell scene, prefixing each item's path
/// with its surface name so a diagnostic names the surface to look at.
pub fn validate_shell(scene: &ShellScene, vals: &SceneValues) -> Vec<LoadIssue> {
    let mut out = Vec::new();
    for s in &scene.surfaces {
        let mut v = Validator::new(vals).with_stores(&s.stores);
        v.walk(&s.items, &format!("{}:items", s.name), None);
        out.extend(v.finish());
    }
    out
}

/// Print a scene's complaints, once, the way every other loader complaint in
/// this crate is printed (`eprintln!`, prefixed `zen:`). Silent when there is
/// nothing to say — a clean scene must not spam the log on every hot reload.
pub fn report(label: &str, path: &std::path::Path, issues: &[LoadIssue]) {
    if issues.is_empty() {
        return;
    }
    eprintln!(
        "zen: scene {label}: {} load issue(s) ({path:?})",
        issues.len()
    );
    for i in issues {
        eprintln!("zen:   - {i}");
    }
}

/// The declared inner box of a container, in base px, when the container
/// declares it as a LITERAL. `None` = the box is runtime arithmetic (a bound
/// edge) or an `auto` measure, so a child of it cannot be proved either way at
/// load and the check stands down rather than guessing.
#[derive(Clone, Copy)]
struct ParentBox {
    w: f32,
    h: f32,
}

struct Validator<'p> {
    /// `None` = do not ask about property names (a partial pool).
    props: Option<&'p SceneValues>,
    /// The pool itself, unconditionally. A model's ROW SHAPE is a different
    /// question from "is this name published", and it is answerable even for a
    /// partial pool — so it must not be gated on `props`.
    vals: &'p SceneValues,
    issues: Vec<LoadIssue>,
    /// Fingerprint of a `Hit` key → the path that claimed it, and the action it
    /// claimed it for. The fingerprint is the authored `Val` plus `key_add`, so
    /// two hits collide when they are the same constant, or the same binding
    /// with the same slot offset (the shape the bar's center strip re-keys).
    keys: Vec<(String, String, Option<super::SceneAction>)>,
    /// The stores THIS surface declares, as `store -> declared props`. A
    /// surface-local closed set of names — which is what makes a `Set` checkable
    /// without a complete pool, and what a dotted binding name can now be
    /// resolved against.
    stores: std::collections::HashMap<String, Vec<(String, super::PropValue)>>,
}

impl<'p> Validator<'p> {
    fn new(vals: &'p SceneValues) -> Self {
        Validator {
            props: vals.is_complete().then_some(vals),
            vals,
            issues: Vec::new(),
            keys: Vec::new(),
            stores: Default::default(),
        }
    }

    /// Give this validator the surface's store declarations, so `Set` targets
    /// and dotted binding names can be checked against them.
    fn with_stores(mut self, stores: &[super::StoreDecl]) -> Self {
        self.stores = stores
            .iter()
            .map(|s| (s.name.clone(), s.props.as_slice().to_vec()))
            .collect();
        self
    }

    /// The declared type of the store prop a dotted `name` addresses, if this
    /// surface declares it.
    fn store_prop(&self, name: &str) -> Option<&super::PropValue> {
        let (store, prop) = name.split_once('.')?;
        let decl: &Vec<(String, super::PropValue)> = self.stores.get(store)?;
        decl.iter().find(|(n, _)| n == prop).map(|(_, v)| v)
    }

    /// Report a `Set` whose target or value type is wrong.
    fn store_write(&mut self, path: &str, name: &str, value: &super::PropValue) {
        let (store, prop) = name.split_once('.').unwrap_or(("", name));
        // The same "one helper, both directions" rule as a scope: if the writer
        // here and the applier in `SceneStoreState::write` ever disagreed about
        // what is legal, a validated scene would still fail silently at click
        // time. `store_write_type_ok` is deliberately the SAME predicate.
        let declared = self.store_prop(name);
        let Some(declared) = declared else {
            let have: Vec<String> = self
                .stores
                .iter()
                .flat_map(|(s, props)| props.iter().map(move |(p, _)| format!("{s}.{p}")))
                .collect();
            self.issues.push(LoadIssue::BadStoreWrite {
                path: path.to_string(),
                name: name.to_string(),
                detail: format!(
                    "names no prop of store {store:?} (declared here: {})",
                    if have.is_empty() {
                        "none".to_string()
                    } else {
                        have.join(", ")
                    }
                ),
            });
            return;
        };
        if !store_write_type_ok(declared, value) {
            self.issues.push(LoadIssue::BadStoreWrite {
                path: path.to_string(),
                name: name.to_string(),
                detail: format!(
                    "is {}, but {store}.{} is declared {}",
                    value_kind(value),
                    prop,
                    declared_kind(declared)
                ),
            });
        }
    }

    fn finish(self) -> Vec<LoadIssue> {
        self.issues
    }

    /// One `Val` field: it must compile, and every name it reads must be
    /// published.
    /// `anim:` is honoured on a field the DRAW pass resolves on its own, and
    /// refused on one the MEASURE pass resolves first — because measure must
    /// read the target (see [`crate::scene::anim`]), so an animated extent
    /// would be laid out at its final size and then drawn there too, every
    /// frame. The parser accepts it; without this report the animation would
    /// simply never be visible.
    fn anim_axes(&mut self, path: &str, field: &str, v: &super::Val) {
        if !matches!(v, super::Val::Anim { .. }) {
            return;
        }
        // The measure pass's inputs, per [`super::SceneItem::want_w`] /
        // `want_h` / `fills_w` / `fills_h` / `intrinsic_w`.
        const MEASURED: &[&str] = &["w", "h", "font_size"];
        if MEASURED.contains(&field) {
            self.issues.push(LoadIssue::AnimatedMeasuredField {
                path: format!("{path}.{field}"),
                detail: format!(
                    "is on `{field}`, which the LAYOUT pass measures — measure has to read \
                     the target, so this would draw at its final value every frame and never \
                     move. Animate a position (`x` / `y`) instead, or move the motion to a \
                     container that positions it."
                ),
            });
        }
    }

    fn val(&mut self, path: &str, field: &str, v: &super::Val) {
        self.anim_axes(path, field, v);
        let src = match v.binding() {
            Some(s) => s,
            None => return,
        };
        let path = format!("{path}.{field}");
        match v.compile() {
            Err(e) => self.issues.push(LoadIssue::BadBinding {
                path,
                src: src.to_string(),
                err: e.to_string(),
            }),
            Ok(Some(tree)) => {
                let mut deps = Vec::new();
                tree.deps(&mut deps);
                let Some(props) = self.props else { return };
                for name in deps {
                    // A SCOPE name is answered by the delegate's own model row,
                    // not by the pool, so the pool cannot judge it — `item.w`
                    // is a typo-proof question only the row shape can answer,
                    // and that is a soundness question for the repeat, not an
                    // `UnknownProperty`. `index` is the same. Everything else
                    // is a pool property and is checked as one.
                    if name == "index" {
                        continue;
                    }
                    // A dotted name is one of two scope names, and they are
                    // NOT interchangeable:
                    //
                    //  - `item.<field>` — the pool cannot answer it, and the
                    //    model's row shape already has its own, better check
                    //    (`UnknownRowField`, which names the fields that DO
                    //    exist).
                    //  - `<store>.<prop>` — this surface's OWN stores are a
                    //    closed set of names, so unlike the flat pool we CAN
                    //    say for certain. This is the phase-3 survivor: the
                    //    check becomes possible precisely because a scope is
                    //    closed, and it is the one dotted case worth reporting.
                    if let Some((store, _)) = name.split_once('.') {
                        if store == "item" || self.store_prop(&name).is_some() {
                            continue;
                        }
                        if !self.stores.contains_key(store) {
                            // A dotted name under some other namespace may be a
                            // plugin's (`mod_clock.time`) or a future scope
                            // kind, and this validator has no way to know which
                            // — so it stands down rather than reporting a name
                            // that is perfectly legal. A store typo IS caught,
                            // just by the `Set` check below, which is exact.
                            continue;
                        }
                        let have: Vec<String> = self.stores[store]
                            .iter()
                            .map(|(n, _)| format!("{store}.{n}"))
                            .collect();
                        self.issues.push(LoadIssue::UnknownStoreProp {
                            path: path.clone(),
                            name: name.clone(),
                            store: store.to_string(),
                            have,
                        });
                        continue;
                    }
                    // A name may be published as a STRING and still be a
                    // legitimate read — `len(name)` on a label is how the
                    // workspace chip sizes itself — so "published at all" is
                    // the bar here, not "published as a number". Arithmetic on
                    // a string is its own (documented) 0. A name the publisher
                    // declared CONDITIONAL counts: the bar's `pill_ctr` model
                    // exists only for the widgets the user put in their bar,
                    // and a template may bind any widget's field.
                    if !props.is_published(&name) {
                        self.issues.push(LoadIssue::UnknownProperty {
                            path: path.clone(),
                            src: src.to_string(),
                            name,
                            kind: "binding",
                        });
                    }
                }
            }
            Ok(None) => {}
        }
    }

    /// A `*_var` / model-name field: a plain string naming a property. Same
    /// question as a binding, without the arithmetic.
    fn name_prop(&mut self, path: &str, field: &str, name: &str, kind: &'static str) {
        let Some(props) = self.props else { return };
        if !props.is_published(name) {
            self.issues.push(LoadIssue::UnknownProperty {
                path: format!("{path}.{field}"),
                src: name.to_string(),
                name: name.to_string(),
                kind,
            });
        }
    }

    /// The `visible` gate, read through [`SceneItem::visible_binding`] so the
    /// validator and the engine can never disagree about which items have one.
    fn gate(&mut self, path: &str, it: &SceneItem) {
        if let Some(n) = it.visible_binding() {
            self.name_prop(path, "visible", n, "visible");
        }
    }

    /// A `Repeat`'s delegate items, validated against the ROW SHAPE of the
    /// model they instantiate over.
    ///
    /// This is the soundness half of the scope: when the engine inflates a
    /// delegate it knows the model's row shape, so `item.<field>` can be
    /// checked against the ACTUAL rows instead of being waved through as a
    /// name the pool might publish. A card-local pool cannot answer that
    /// question at all (see [`SceneValues::complete`]) — a model's rows can,
    /// because they are a closed set the publisher built whole.
    ///
    /// The check is per TEMPLATE, not per row: a template is drawn for every
    /// row whose `tag` it matches, so the field has to be one the rows carry
    /// in general, and the report lists what they do carry.
    fn row_fields(&mut self, path: &str, model: &str, templates: &[super::SceneTemplate]) {
        let rows = self.vals.model(model);
        if rows.is_empty() {
            // the model published no rows this frame — that is a
            // conditional/empty case, not a typo, and standing down is the
            // only honest answer without a declared row shape
            return;
        }
        // the union of every field any row carries, read through
        // [`SceneScope::fields`] so the validator and the engine agree on what
        // a row's field names ARE. A model is allowed to be ragged (a row may
        // omit a field its template never reads), so the union is the
        // contract; a field only SOME rows carry is the model's business, and
        // the 0 it produces for the others is what a ragged model means.
        let have: std::collections::BTreeSet<String> = rows
            .iter()
            .flat_map(super::SceneScope::fields)
            .map(str::to_string)
            .collect();
        for (n, t) in templates.iter().enumerate() {
            self.row_field_deps(&format!("{path}/templates[{n}]"), model, &have, &t.items);
        }
    }

    /// Every `item.<field>` one template's subtree binds or interpolates,
    /// checked against the model's fields.
    ///
    /// The names come from [`SceneItem::scope_names`], which reads the SAME
    /// two sources the engine folds from — `Val::compile` → `Expr::deps` for
    /// arithmetic and the `{name}` tokens of an interpolated string — so the
    /// validator cannot disagree with the evaluator about what a delegate asks
    /// for. Each unknown field is reported once per template, not once per
    /// occurrence: a chip whose four children all bind `item.ink` is one
    /// mistake, not four.
    fn row_field_deps(
        &mut self,
        path: &str,
        model: &str,
        have: &std::collections::BTreeSet<String>,
        items: &[SceneItem],
    ) {
        let mut asked: std::collections::BTreeSet<String> = Default::default();
        for it in items {
            asked.extend(it.scope_names());
        }
        for name in asked {
            let Some(field) = name.strip_prefix("item.") else {
                continue;
            };
            if !have.contains(field) {
                self.issues.push(LoadIssue::UnknownRowField {
                    path: path.to_string(),
                    name,
                    model: model.to_string(),
                    have: have.iter().cloned().collect(),
                });
            }
        }
    }

    /// A `Hit`'s key fingerprint, for the collision table. The authored `Val`
    /// and the slot offset together: `Hit { key: "pill_ws_key", key_add: 5 }`
    /// and `Hit { key: "pill_ws_key" }` are different keys at runtime and must
    /// not be reported as a collision.
    ///
    /// A repeat is only an issue when the two claimants DISAGREE about what the
    /// key does. Two boxes on one key that mean the same thing are the fat
    /// target idiom — the latency card's 30×20 pill plus the invisible 36×26
    /// halo around it — and the shell's click path is built for exactly that
    /// (every region on the key resolves to the same handler). Two claimants
    /// that name DIFFERENT actions is the bug: `scene_click_actions` is keyed
    /// by the key alone, so one of the two controls is simply dead, and which
    /// one depends on draw order.
    fn hit_key(
        &mut self,
        path: &str,
        key: &super::Val,
        key_add: u32,
        action: Option<&super::SceneAction>,
    ) {
        // Built through `binding()` rather than matched, because `anim:` is a
        // WRAPPER: two animated spellings of the same binding are the same
        // key, and matching on the variant would tell them apart.
        let fingerprint = match key.binding() {
            Some(src) => format!("{src}+{key_add}"),
            None => format!("#{}", key.constant() as i64 + key_add as i64),
        };
        let at = format!("{path}.Hit.key");
        let existing = self.keys.iter().position(|(f, _, _)| *f == fingerprint);
        match existing {
            Some(i) if self.keys[i].2.as_ref() != action => {
                let (_, first, _) = self.keys[i].clone();
                let key = match key.binding() {
                    Some(src) => format!("{src:?} + {key_add}"),
                    None => format!("{}", key.constant() as u32 + key_add),
                };
                self.issues.push(LoadIssue::KeyCollision {
                    key,
                    first,
                    second: at,
                });
            }
            Some(_) => {}
            None => self.keys.push((fingerprint, at, action.cloned())),
        }
    }

    /// `anchors` against a parent whose box the scene declares: the region is
    /// `inner_w − left − right` by `inner_h − top − bottom`, so an inset pair
    /// that sums past the parent's own box leaves nothing to click.
    fn anchors(&mut self, path: &str, a: &super::Anchors, parent: Option<ParentBox>) {
        let Some(p) = parent else { return };
        let (pw, ph) = (p.w, p.h);
        let w = pw - a.left - a.right;
        let h = ph - a.top - a.bottom;
        let (bad, detail) = if w <= 0.0 {
            (
                true,
                format!("left {} + right {} ≥ parent width {}", a.left, a.right, pw),
            )
        } else if h <= 0.0 {
            (
                true,
                format!("top {} + bottom {} ≥ parent height {}", a.top, a.bottom, ph),
            )
        } else {
            return;
        };
        if bad {
            self.issues.push(LoadIssue::InvertedAnchors {
                path: format!("{path}.Hit.anchors"),
                detail,
            });
        }
    }

    /// The inner box a container hands its children, when that box is a
    /// literal. `pad` is the container's own inset.
    fn inner_box(w: &super::Val, h: &super::Val, pad: f32, auto: bool) -> Option<ParentBox> {
        if auto || w.binding().is_some() || h.binding().is_some() {
            return None;
        }
        let (w, h) = (w.constant() - pad * 2.0, h.constant() - pad * 2.0);
        if w <= 0.0 || h <= 0.0 {
            // a fill / zero-width container takes its extent from the card at
            // draw time, so there is nothing to prove against
            return None;
        }
        Some(ParentBox { w, h })
    }

    /// Walk one item list. `parent` is the box the items are laid out in, when
    /// the scene declares it.
    fn walk(&mut self, items: &[SceneItem], path: &str, parent: Option<ParentBox>) {
        for (i, it) in items.iter().enumerate() {
            self.item(it, &format!("{path}[{i}]"), parent);
        }
    }

    fn item(&mut self, it: &SceneItem, path: &str, parent: Option<ParentBox>) {
        use SceneItem as I;
        // The engine's own gate list, reused verbatim so the two cannot drift.
        self.gate(path, it);
        match it {
            I::Text {
                x,
                y,
                w,
                h,
                font_size,
                ..
            } => {
                self.val(path, "x", x);
                self.val(path, "y", y);
                self.val(path, "w", w);
                self.val(path, "h", h);
                self.val(path, "font_size", font_size);
            }
            I::When { items, .. } => self.walk(items, &format!("{path}/items"), parent),
            I::TextWrap { w, .. } => self.val(path, "w", w),
            I::Hit {
                x,
                y,
                w,
                h,
                anchors,
                key,
                key_add,
                action,
                ..
            } => {
                self.val(path, "x", x);
                self.val(path, "y", y);
                self.val(path, "w", w);
                self.val(path, "h", h);
                self.hit_key(path, key, *key_add, action.as_ref());
                if let Some(super::SceneAction::Set { name, value }) = action {
                    self.store_write(
                        &format!("{path}.action"),
                        name,
                        value,
                    );
                }
                if let Some(a) = anchors {
                    self.anchors(path, a, parent);
                }
            }
            I::Surface {
                x,
                y,
                w,
                h,
                visible,
                hover_key_var,
                ..
            } => {
                self.val(path, "x", x);
                self.val(path, "y", y);
                self.val(path, "w", w);
                self.val(path, "h", h);
                // `Surface` gates in its own draw arm, so it is not in
                // `visible_binding` — check the field here or not at all.
                if let Some(n) = visible.as_deref() {
                    self.name_prop(path, "visible", n, "visible");
                }
                if let Some(n) = hover_key_var.as_deref() {
                    self.name_prop(path, "hover_key_var", n, "property");
                }
            }
            I::Scissor { x, y, w, h, .. } => self.box4(path, x, y, w, h),
            I::Divider { x, y, w, .. } => {
                self.val(path, "x", x);
                self.val(path, "y", y);
                self.val(path, "w", w);
            }
            I::Bar {
                x, y, w, h, ..
            }
            | I::Battery {
                x, y, w, h, ..
            }
            | I::Image {
                x, y, w, h, ..
            }
            // `Ink.name` is a RUST PAINTER name ("panel", "card") resolved by
            // the drawer, NOT a property name — nothing to check but the box
            | I::Ink {
                x, y, w, h, ..
            } => self.box4(path, x, y, w, h),
            // plots are named after the series list they read; an unknown one
            // leaves the axes drawn and the line missing
            I::Spark {
                name, x, y, w, h, ..
            }
            | I::Spectrum {
                name, x, y, w, h, ..
            } => {
                self.name_prop(path, "name", name, "property");
                self.box4(path, x, y, w, h);
            }
            I::Rows {
                name,
                y,
                w,
                start,
                check,
                flash,
                karaoke,
                ..
            } => {
                // a data-driven table whose `name` nobody publishes renders an
                // EMPTY table, headers and all — the item looks deliberate
                self.name_prop(path, "name", name, "property");
                self.val(path, "y", y);
                self.val(path, "w", w);
                // NOT `more`: that one is a LABEL TEMPLATE (`"+{n} more"`),
                // not a property name. Getting that one wrong is what a
                // validator that "checks every Option<String>" would do.
                for (field, n) in [
                    ("start", start.as_deref()),
                    ("check", check.as_deref()),
                    ("flash", flash.as_deref()),
                    ("karaoke", karaoke.as_deref()),
                ] {
                    if let Some(n) = n {
                        self.name_prop(path, field, n, "property");
                    }
                }
            }
            I::Moon { x, y, d, .. } => {
                self.val(path, "x", x);
                self.val(path, "y", y);
                self.val(path, "d", d);
            }
            I::Ring { name, .. } => self.name_prop(path, "name", name, "property"),
            I::Dots { x, active, when, .. } => {
                self.val(path, "x", x);
                self.name_prop(path, "active", active, "property");
                if let Some(n) = when.as_deref() {
                    self.name_prop(path, "when", n, "property");
                }
            }
            // a gauge whose `name` nobody publishes draws an EMPTY gauge, track
            // and all — same class as `Rows`: looks deliberate, not broken
            I::Fader {
                name, x, y, w, h, ..
            }
            | I::Toggle {
                name, x, y, w, h, ..
            } => {
                self.name_prop(path, "name", name, "property");
                self.box4(path, x, y, w, h);
            }
            I::TabRow { name, y, h, sel, .. } => {
                self.name_prop(path, "name", name, "property");
                self.val(path, "y", y);
                self.val(path, "h", h);
                if let Some(n) = sel.as_deref() {
                    self.name_prop(path, "sel", n, "property");
                }
            }
            I::Composer { y, w, h, focus, .. } => {
                self.val(path, "y", y);
                self.val(path, "w", w);
                self.val(path, "h", h);
                if let Some(n) = focus.as_deref() {
                    self.name_prop(path, "focus", n, "property");
                }
            }
            I::Header { .. } => {}
            I::Strip {
                chips, x, y, w, h, sel, scroll, ..
            } => {
                self.name_prop(path, "chips", chips, "property");
                self.box4(path, x, y, w, h);
                for (field, n) in [("sel", sel.as_deref()), ("scroll", scroll.as_deref())] {
                    if let Some(n) = n {
                        self.name_prop(path, field, n, "property");
                    }
                }
            }
            I::BannerRow {
                cells,
                x,
                y,
                w,
                h,
                overflow,
                zones,
                ..
            } => {
                self.name_prop(path, "cells", cells, "property");
                self.box4(path, x, y, w, h);
                for (field, n) in [
                    ("overflow", overflow.as_deref()),
                    ("zones", zones.as_deref()),
                ] {
                    if let Some(n) = n {
                        self.name_prop(path, field, n, "property");
                    }
                }
            }
            I::Row {
                x,
                y,
                w,
                h,
                pad,
                auto,
                items,
                ..
            } => self.container(path, x, y, w, h, *pad, *auto, items, parent),
            I::Column {
                x,
                y,
                w,
                h,
                pad,
                auto,
                items,
                ..
            } => self.container(path, x, y, w, h, *pad, *auto, items, parent),
            I::Stack {
                x,
                y,
                w,
                h,
                pad,
                auto,
                items,
                ..
            } => self.container(path, x, y, w, h, *pad, *auto, items, parent),
            I::Repeat {
                name,
                model,
                templates,
                ..
            } => {
                match model.as_deref() {
                    // a MODEL repeat: the rows are the publisher's, and the
                    // template binds `item.<field>` / `index` against them.
                    // Those names are NOT pool properties, so they are exempt
                    // from the "nobody publishes this?" check (a scope name can
                    // never be a pool name) — but a scope name that is a typo
                    // is exactly the silent 0 this subsystem exists to catch,
                    // and the model value itself IS a pool property.
                    Some(m) => {
                        self.name_prop(path, "model", m, "model");
                        self.row_fields(path, m, templates);
                    }
                    None => self.name_prop(path, "name", name, "property"),
                }
                for (n, t) in templates.iter().enumerate() {
                    self.walk(&t.items, &format!("{path}/templates[{n}]"), parent);
                }
            }
            I::Grid {
                name,
                start,
                max_rows_var,
                cols_var,
                cell_var,
                ..
            } => {
                self.name_prop(path, "name", name, "property");
                for (field, n) in [
                    ("start", start.as_deref()),
                    ("max_rows_var", max_rows_var.as_deref()),
                    ("cols_var", cols_var.as_deref()),
                    ("cell_var", cell_var.as_deref()),
                ] {
                    if let Some(n) = n {
                        self.name_prop(path, field, n, "property");
                    }
                }
            }
            I::Tiles { name, .. } => self.name_prop(path, "name", name, "property"),
            I::Comp { .. } | I::ScissorEnd => {}
        }
    }

    fn box4(&mut self, path: &str, x: &super::Val, y: &super::Val, w: &super::Val, h: &super::Val) {
        self.val(path, "x", x);
        self.val(path, "y", y);
        self.val(path, "w", w);
        self.val(path, "h", h);
    }

    fn container(
        &mut self,
        path: &str,
        x: &super::Val,
        y: &super::Val,
        w: &super::Val,
        h: &super::Val,
        pad: f32,
        auto: bool,
        items: &[SceneItem],
        parent: Option<ParentBox>,
    ) {
        self.box4(path, x, y, w, h);
        let inner = Self::inner_box(w, h, pad, auto);
        // A container keeps its OWN parent box for its children only when it
        // declares one; otherwise the child inherits this item's parent (a
        // `Stack` with no declared size draws its children in the box the
        // stack itself sits in).
        let child_parent = inner.or(parent);
        self.walk(items, &format!("{path}/items"), child_parent);
    }
}

/// Every `Val` field this walker reads, as `(variant, field)`.
///
/// It is a list, not a `match` arm per field, so the sweep the whole
/// declarative rewrite is built on ("a field rename in `src/scene.rs` is not
/// done until every scene has been swept") is a TEST instead of a habit:
/// `the_walker_reads_every_val_field_in_the_schema` reads this file's own
/// source, finds every `: Val,` field on `SceneItem`, and fails if a field
/// exists that this list does not name. Adding a `Val` field therefore cannot
/// silently escape the diagnostics.
///
/// The list is the walker's DECLARED coverage, and the sweep above keeps that
/// declaration honest against the schema — it cannot prove the walker has an
/// arm for each entry. `every_declared_geometry_field_is_walked` closes the
/// part that matters most by resolving a real fixture per variant; a
/// non-geometry field added to this list with no arm in the walker stays
/// uncovered.
#[cfg(test)]
pub const WALKED_VAL_FIELDS: &[(&str, &str)] = &[
    ("Text", "x"),
    ("Text", "y"),
    ("Text", "w"),
    ("Text", "h"),
    ("Text", "font_size"),
    ("TextWrap", "w"),
    ("Hit", "x"),
    ("Hit", "y"),
    ("Hit", "w"),
    ("Hit", "h"),
    ("Hit", "key"),
    ("Surface", "x"),
    ("Surface", "y"),
    ("Surface", "w"),
    ("Surface", "h"),
    ("Scissor", "x"),
    ("Scissor", "y"),
    ("Scissor", "w"),
    ("Scissor", "h"),
    ("Divider", "x"),
    ("Divider", "y"),
    ("Divider", "w"),
    ("Bar", "x"),
    ("Bar", "y"),
    ("Bar", "w"),
    ("Bar", "h"),
    ("Battery", "x"),
    ("Battery", "y"),
    ("Battery", "w"),
    ("Battery", "h"),
    ("Rows", "y"),
    ("Rows", "w"),
    ("Spark", "x"),
    ("Spark", "y"),
    ("Spark", "w"),
    ("Spark", "h"),
    ("Spectrum", "x"),
    ("Spectrum", "y"),
    ("Spectrum", "w"),
    ("Spectrum", "h"),
    ("Moon", "x"),
    ("Moon", "y"),
    ("Moon", "d"),
    ("Fader", "x"),
    ("Fader", "y"),
    ("Fader", "w"),
    ("Fader", "h"),
    ("Toggle", "x"),
    ("Toggle", "y"),
    ("Toggle", "w"),
    ("Toggle", "h"),
    ("Dots", "x"),
    ("TabRow", "y"),
    ("TabRow", "h"),
    ("Composer", "y"),
    ("Composer", "w"),
    ("Composer", "h"),
    ("Strip", "x"),
    ("Strip", "y"),
    ("Strip", "w"),
    ("Strip", "h"),
    ("BannerRow", "x"),
    ("BannerRow", "y"),
    ("BannerRow", "w"),
    ("BannerRow", "h"),
    ("Ink", "x"),
    ("Ink", "y"),
    ("Ink", "w"),
    ("Ink", "h"),
    ("Row", "x"),
    ("Row", "y"),
    ("Row", "w"),
    ("Row", "h"),
    ("Column", "x"),
    ("Column", "y"),
    ("Column", "w"),
    ("Column", "h"),
    ("Stack", "x"),
    ("Stack", "y"),
    ("Stack", "w"),
    ("Stack", "h"),
    ("Repeat", "x"),
    ("Repeat", "y"),
    ("Repeat", "w"),
    ("Repeat", "h"),
    ("Grid", "x"),
    ("Grid", "y"),
    ("Grid", "w"),
    ("Grid", "h"),
    ("Tiles", "x"),
    ("Tiles", "y"),
    ("Tiles", "w"),
    ("Tiles", "h"),
    ("Image", "x"),
    ("Image", "y"),
    ("Image", "w"),
    ("Image", "h"),
    ("Comp", "x"),
    ("Comp", "y"),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::{expr, parse_scene_ron, CardScene, SceneValue};

    /// A pool that may be asked "does anybody publish this?" — the property
    /// checks stand down without one.
    fn full_pool() -> SceneValues {
        let mut v = SceneValues::new();
        v.set_num("chip_w", 20.0);
        v.set_num("pill_gap", 6.0);
        // a data-model name resolves through the same pool, so the geometry
        // sweep needs one to point `Rows`/`Spark`/`Fader`/... at
        v.insert("model_rows", crate::scene::SceneValue::Rows(vec![]));
        v.set_bool("pane0", true);
        v.set_str("ws_label", "3");
        v.set_num("pill_mid", 12.0);
        v.mark_complete();
        v
    }

    fn check(src: &str) -> Vec<LoadIssue> {
        let scene: CardScene = parse_scene_ron(src).expect("test scene parses");
        validate_items(&scene.items, &full_pool())
    }

    /// Validate a whole-shell scene whose surfaces carry stores, so the
    /// `Set`-target checks have their closed name set to work from.
    fn check_shell(src: &str) -> Vec<LoadIssue> {
        check_shell_with(&full_pool(), src)
    }

    fn check_shell_with(vals: &SceneValues, src: &str) -> Vec<LoadIssue> {
        let mut scene: ShellScene = parse_scene_ron(src).expect("test scene parses");
        scene.expand();
        validate_shell(&scene, vals)
    }

    #[test]
    fn a_set_to_a_declared_store_prop_is_clean() {
        assert!(check_shell(
            r#"(surfaces: [ (name: "settings",
                stores: [ (name: "nav", props: (tab: 0.0, editing: false)) ],
                items: [
                    Hit(key: 200.0, x: 0.0, y: 0.0, w: 8.0, h: 8.0,
                        action: Set(name: "nav.tab", value: 2.0)),
                    Hit(key: 201.0, x: 0.0, y: 20.0, w: 8.0, h: 8.0,
                        action: Set(name: "nav.editing", value: true)),
                ])])"#
        )
        .is_empty());
    }

    #[test]
    fn a_set_naming_no_declared_prop_is_reported() {
        // The typo this exists for: `nav.scrol` matches no store prop, so the
        // click is refused at runtime and the scroll never moves. Without the
        // load-time report the only symptom is a control that does nothing.
        let issues = check_shell(
            r#"(surfaces: [ (name: "settings",
                stores: [ (name: "nav", props: (tab: 0.0)) ],
                items: [
                    Hit(key: 200.0, x: 0.0, y: 0.0, w: 8.0, h: 8.0,
                        action: Set(name: "nav.scrol", value: 2.0)),
                ])])"#,
        );
        match issues.as_slice() {
            [LoadIssue::BadStoreWrite { name, detail, .. }] => {
                assert_eq!(name, "nav.scrol");
                // the message must list what IS declared — the fix is a typo away
                assert!(detail.contains("nav.tab"), "{detail}");
            }
            other => panic!("expected one BadStoreWrite, got {other:?}"),
        }
    }

    #[test]
    fn a_set_of_the_wrong_type_is_reported() {
        // `nav.tab` is numeric; writing a string would leave the binding
        // resolving to 0 forever (a `Text` is not a `scalar`), with no other
        // symptom.
        let issues = check_shell(
            r#"(surfaces: [ (name: "settings",
                stores: [ (name: "nav", props: (tab: 0.0)) ],
                items: [
                    Hit(key: 200.0, x: 0.0, y: 0.0, w: 8.0, h: 8.0,
                        action: Set(name: "nav.tab", value: "2")),
                ])])"#,
        );
        match issues.as_slice() {
            [LoadIssue::BadStoreWrite { name, detail, .. }] => {
                assert_eq!(name, "nav.tab");
                assert!(detail.contains("declared a number"), "{detail}");
            }
            other => panic!("expected one BadStoreWrite, got {other:?}"),
        }
    }

    #[test]
    fn a_store_prop_is_visible_only_to_its_own_surface() {
        // Two surfaces may both use key 200 and both write `nav.tab`; the write
        // must reach the store of the surface that DECLARED it, and a `Set`
        // naming a store the surface does not declare is an error even though
        // the name exists in a sibling surface.
        let issues = check_shell(
            r#"(surfaces: [
                (name: "settings",
                    stores: [ (name: "nav", props: (tab: 0.0)) ],
                    items: [ Hit(key: 200.0, w: 8.0, h: 8.0,
                        action: Set(name: "nav.tab", value: 1.0)) ]),
                (name: "osd",
                    stores: [ (name: "lvl", props: (v: 0.0)) ],
                    items: [ Hit(key: 200.0, w: 8.0, h: 8.0,
                        action: Set(name: "nav.tab", value: 1.0)) ]),
            ])"#,
        );
        match issues.as_slice() {
            [LoadIssue::BadStoreWrite { path, name, detail }] => {
                assert!(path.starts_with("osd:"), "{path}");
                assert_eq!(name, "nav.tab");
                assert!(detail.contains("lvl.v"), "{detail}");
            }
            other => panic!("expected one BadStoreWrite on osd, got {other:?}"),
        }
    }

    #[test]
    fn a_binding_to_a_store_prop_is_clean_and_a_typo_is_reported() {
        // A store prop is published into the pool under its dotted name, so a
        // binding naming one resolves like any other property. The interesting
        // half is that the validator must NOT report `nav.tab` as an unknown
        // property just because the flat pool does not carry it.
        let clean = check_shell(
            r#"(surfaces: [ (name: "settings",
                stores: [ (name: "nav", props: (tab: 0.0)) ],
                items: [ Surface(x: "nav.tab", w: 40.0, h: 4.0, color: acc) ])])"#,
        );
        assert!(clean.is_empty(), "{clean:?}");

        // `nav.tb` is the same store, so this surface's closed name set CAN
        // judge it — unlike `item.w`, where only the row shape knows. This is
        // the one dotted case worth reporting.
        let typo = check_shell(
            r#"(surfaces: [ (name: "settings",
                stores: [ (name: "nav", props: (tab: 0.0)) ],
                items: [ Surface(x: "nav.tb", w: 40.0, h: 4.0, color: acc) ])])"#,
        );
        match typo.as_slice() {
            [LoadIssue::UnknownStoreProp {
                name, store, have, ..
            }] => {
                assert_eq!(name, "nav.tb");
                assert_eq!(store, "nav");
                assert_eq!(have, &["nav.tab".to_string()]);
            }
            other => panic!("expected one UnknownStoreProp, got {other:?}"),
        }
    }

    #[test]
    fn a_binding_under_an_undeclared_namespace_is_left_alone() {
        // The deliberate limit of the check above: `mod_clock.time` is a
        // plugin's dotted name, and this validator has no way to know that, so
        // it must stand down rather than report a perfectly legal name.
        let issues = check_shell(
            r#"(surfaces: [ (name: "bar",
                stores: [ (name: "nav", props: (tab: 0.0)) ],
                items: [ Surface(x: "mod_clock.time", w: 40.0, h: 4.0, color: acc) ])])"#,
        );
        assert!(issues.is_empty(), "{issues:?}");
    }

    #[test]
    fn an_item_scope_binding_is_not_a_store_check() {
        // `item.w` is answered by the delegate's model row, not by a store. The
        // surface here DOES declare a store, so the new dotted-name arm is
        // live — and it must still stand down on `item.`, leaving
        // `UnknownRowField` (the row-shape check, which knows the answer) as the
        // only reporter.
        let issues = check_shell_with(
            &pool_with_model(),
            r#"(surfaces: [ (name: "bar",
                stores: [ (name: "nav", props: (tab: 0.0)) ],
                items: [
                    Repeat(name: "ctr_tags", model: "ctr", templates: [
                        (tag: "ws", items: [
                            Surface(x: 0.0, y: 0.0, w: "item.wn", h: 24.0, color: fg),
                        ]),
                    ]),
                ])])"#,
        );
        match issues.as_slice() {
            [LoadIssue::UnknownRowField { name, model, .. }] => {
                assert_eq!(name, "item.wn");
                assert_eq!(model, "ctr");
            }
            other => panic!("expected one UnknownRowField and no store complaint, got {other:?}"),
        }
    }

    #[test]
    fn the_walker_reads_every_val_field_in_the_schema() {
        // The sweep this project keeps re-learning: a field added to
        // `SceneItem` and not swept draws wrong in silence. Here the sweep is
        // a test — it reads this crate's own source for `: Val,` fields on
        // `SceneItem` and fails if the walker's list does not name every one.
        let src = include_str!("../scene.rs");
        let mut in_enum = false;
        let mut variant = String::new();
        let mut schema: Vec<(String, String)> = Vec::new();
        for line in src.lines() {
            if line.starts_with("pub enum SceneItem") {
                in_enum = true;
                continue;
            }
            if in_enum {
                if line == "}" {
                    break;
                }
                let trimmed = line.trim_end();
                // a variant header: `    Text {` / `    ScissorEnd,`
                if let Some(rest) = trimmed.strip_prefix("    ") {
                    let head = rest.split(['{', ',']).next().unwrap_or("").trim();
                    if head.chars().next().is_some_and(char::is_uppercase) {
                        variant = head.to_string();
                        continue;
                    }
                }
                let field = trimmed.trim_start();
                if let Some(name) = field.strip_suffix(": Val,") {
                    schema.push((variant.clone(), name.trim().to_string()));
                }
            }
        }
        assert!(
            schema.len() >= 99,
            "the source scan found {schema:?} — the enum parse broke, not the schema"
        );
        let walked: Vec<(String, String)> = WALKED_VAL_FIELDS
            .iter()
            .map(|(v, f)| (v.to_string(), f.to_string()))
            .collect();
        assert_eq!(
            walked, schema,
            "the walker's field list and the schema's `Val` fields have drifted: \
             every field a RON can BIND needs an arm in the walker, or a \
             misspelled binding in it is never reported"
        );
    }

    #[test]
    fn every_live_scene_validates_clean_against_the_shells_own_pool() {
        // The audit this whole subsystem exists for, run over the real config
        // tree with the shell's own published names. A clean run is the point:
        // it says no live binding is dead, no gate names a toggle nobody
        // publishes, no two hits share a key, and no anchors invert.
        let cfg: crate::config::Config =
            toml::from_str(crate::config::DEFAULT_SHELL_TOML).expect("default config parses");
        let shell = crate::shell::Shell::new(cfg);
        let vals = shell.scene_values();
        assert!(
            vals.is_complete(),
            "the shell's pool must be marked complete or every property check stands down"
        );

        let mut all = Vec::new();
        let cards_dir = crate::vars::card_scenes_dir();
        let mut n_cards = 0;
        if let Ok(rd) = std::fs::read_dir(&cards_dir) {
            let mut paths: Vec<std::path::PathBuf> = rd
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| p.extension().is_some_and(|e| e == "ron"))
                .collect();
            paths.sort();
            for p in paths {
                let text = std::fs::read_to_string(&p).expect("scene file readable");
                let scene: CardScene = match parse_scene_ron(&text) {
                    Ok(s) => s,
                    Err(e) => panic!("{} must parse: {e}", p.display()),
                };
                n_cards += 1;
                for i in validate_items(&scene.items, &vals) {
                    all.push(format!("{}: {i}", p.display()));
                }
            }
        }
        let shell_path = crate::vars::shell_scene_path();
        let mut n_surfaces = 0;
        if shell_path.exists() {
            let text = std::fs::read_to_string(&shell_path).expect("shell scene readable");
            let mut scene: ShellScene =
                parse_scene_ron(&text).unwrap_or_else(|e| panic!("shell.ron must parse: {e}"));
            scene.expand();
            n_surfaces = scene.surfaces.len();
            for i in validate_shell(&scene, &vals) {
                all.push(format!("shell.ron: {i}"));
            }
        }
        assert!(
            n_cards > 0 || n_surfaces > 0,
            "the live scene tree is missing — this test would pass vacuously"
        );
        assert!(
            all.is_empty(),
            "the live scene tree has {} load issue(s):\n{}",
            all.len(),
            all.join("\n")
        );
    }

    #[test]
    fn a_clean_scene_says_nothing() {
        let issues = check(
            r#"(items: [
                Hit(key: 40.0, x: 0.0, y: 0.0, w: "chip_w", h: 18.0, text: "{ws_label}"),
                Surface(x: 4.0, y: 2.0, w: 8.0, h: 8.0, color: fg, visible: "pane0"),
            ])"#,
        );
        assert_eq!(
            issues,
            Vec::<LoadIssue>::new(),
            "no complaints on a clean scene"
        );
    }

    #[test]
    fn a_binding_that_does_not_compile_is_reported_with_its_source() {
        let issues = check(r#"(items: [Hit(key: 1.0, x: 0.0, y: 0.0, w: "chip_w +", h: 4.0)])"#);
        match issues.as_slice() {
            [LoadIssue::BadBinding { path, src, .. }] => {
                assert_eq!(path, "items[0].w");
                assert_eq!(src, "chip_w +");
            }
            other => panic!("expected one BadBinding, got {other:?}"),
        }
        // and the engine's own answer for that binding is the 0 this prevents
        // anyone from reading as a width
        assert_eq!(expr::compile("chip_w +").is_err(), true);
    }

    #[test]
    fn a_binding_naming_nobody_s_value_is_reported() {
        let issues = check(r#"(items: [Text(x: "chip_widht", y: 0.0, text: "x")])"#);
        match issues.as_slice() {
            [LoadIssue::UnknownProperty {
                path, name, kind, ..
            }] => {
                assert_eq!(path, "items[0].x");
                assert_eq!(name, "chip_widht");
                assert_eq!(*kind, "binding");
            }
            other => panic!("expected one UnknownProperty, got {other:?}"),
        }
    }

    #[test]
    fn a_string_property_is_a_published_name_too() {
        // `len(ws_label)` is a legitimate read of a TEXT property: a name
        // published as a string must not be called unpublished.
        assert!(check(r#"(items: [Text(x: "len(ws_label) * 4", y: 0.0, text: "x")])"#).is_empty());
    }

    #[test]
    fn an_unknown_visible_gate_is_reported() {
        let issues = check(
            r#"(items: [Surface(x: 0.0, y: 0.0, w: 4.0, h: 4.0, color: fg, visible: "pane1")])"#,
        );
        match issues.as_slice() {
            [LoadIssue::UnknownProperty {
                path, name, kind, ..
            }] => {
                assert_eq!(path, "items[0].visible");
                assert_eq!(name, "pane1");
                assert_eq!(*kind, "visible");
            }
            other => panic!("expected one UnknownProperty on the gate, got {other:?}"),
        }
    }

    #[test]
    fn a_partial_pool_is_never_asked_about_property_names() {
        // the pool a card builds for itself knows nothing about the rest of
        // the shell, so it must not turn every other name into a complaint
        let scene: CardScene =
            parse_scene_ron(r#"(items: [Text(x: "not_mine", y: 0.0, text: "x")])"#)
                .expect("parses");
        let mut local = SceneValues::new();
        local.insert("mine", SceneValue::Text("x".into()));
        assert!(validate_items(&scene.items, &local).is_empty());
    }

    #[test]
    fn two_hits_on_one_key_doing_different_things_are_reported() {
        // `scene_click_actions` is keyed by the key alone, so the second box's
        // action is what the whole key does — one of the two is simply dead
        let issues = check(
            r#"(items: [
                Hit(key: 40.0, x: 0.0, y: 0.0, w: 8.0, h: 8.0, action: Command("wifi on")),
                Hit(key: 40.0, x: 20.0, y: 0.0, w: 8.0, h: 8.0, action: Command("bt on")),
                Hit(key: 41.0, x: 40.0, y: 0.0, w: 8.0, h: 8.0, action: Command("wifi off")),
            ])"#,
        );
        match issues.as_slice() {
            [LoadIssue::KeyCollision { key, first, second }] => {
                assert_eq!(key, "40");
                assert_eq!(first, "items[0].Hit.key");
                assert_eq!(second, "items[1].Hit.key");
            }
            other => panic!("expected exactly one collision, got {other:?}"),
        }
    }

    #[test]
    fn two_hits_on_one_key_doing_the_same_thing_are_the_fat_target_idiom() {
        // the latency card's 30x20 pill plus the invisible 36x26 halo around
        // it: every region on the key resolves to the same handler, so this is
        // how a scene widens a click target and must never be reported — the
        // same holds for two boxes carrying the SAME declared action
        assert!(check(
            r#"(items: [
                Hit(key: 31_970, x: 12.0, y: 8.0, w: 30.0, h: 20.0, surface: raised),
                Hit(key: 31_970, x: 9.0, y: 5.0, w: 36.0, h: 26.0),
                Hit(key: 42.0, x: 9.0, y: 5.0, w: 36.0, h: 26.0, action: Command("zen-shell open wifi")),
                Hit(key: 42.0, x: 1.0, y: 5.0, w: 20.0, h: 26.0, action: Command("zen-shell open wifi")),
            ])"#
        )
        .is_empty());
        // …but a second action on the same key is the bug
        assert!(
            check(
                r#"(items: [
                Hit(key: 42.0, action: Command("zen-shell open wifi")),
                Hit(key: 42.0, action: Command("zen-shell open bt")),
            ])"#
            )
            .len()
                == 1
        );
    }

    #[test]
    fn key_add_is_part_of_the_fingerprint() {
        // the bar's center strip re-keys one template into a run: `key + 0`
        // and `key + 1` are different keys, and a scene may declare both
        assert!(check(
            r#"(items: [
                Hit(key: 50.0, key_add: 0, w: 8.0, h: 8.0),
                Hit(key: 50.0, key_add: 1, w: 8.0, h: 8.0),
            ])"#
        )
        .is_empty());
    }

    #[test]
    fn a_bound_key_collides_only_with_the_same_binding_and_offset() {
        let issues = check(
            r#"(items: [
                Hit(key: "chip_w", w: 8.0, h: 8.0, action: Command("a")),
                Hit(key: "chip_w", w: 8.0, h: 8.0, action: Command("b")),
                Hit(key: "chip_w + 1", w: 8.0, h: 8.0, action: Command("c")),
                Hit(key: "chip_w", key_add: 3, w: 8.0, h: 8.0, action: Command("d")),
            ])"#,
        );
        match issues.as_slice() {
            [LoadIssue::KeyCollision { key, first, second }] => {
                assert_eq!(key, "\"chip_w\" + 0");
                assert_eq!(first, "items[0].Hit.key");
                assert_eq!(second, "items[1].Hit.key");
            }
            other => panic!("expected one collision, got {other:?}"),
        }
    }

    #[test]
    fn anchors_are_checked_against_a_declared_parent_only() {
        // 40 + 2 of insets inside a 36 px parent leaves no area
        let issues = check(
            r#"(items: [
                Row(w: 40.0, h: 20.0, pad: 2.0, items: [
                    Hit(key: 1.0, anchors: (left: 40.0, top: 0.0, right: 2.0, bottom: 0.0)),
                ]),
            ])"#,
        );
        match issues.as_slice() {
            [LoadIssue::InvertedAnchors { path, detail }] => {
                assert_eq!(path, "items[0]/items[0].Hit.anchors");
                assert!(
                    detail.contains("40"),
                    "the arithmetic names the insets: {detail}"
                );
                assert!(
                    detail.contains("36"),
                    "…and the parent's inner width: {detail}"
                );
            }
            other => panic!("expected one InvertedAnchors, got {other:?}"),
        }
    }

    #[test]
    fn anchors_fit_when_the_whole_parent_is_claimed() {
        // the bar's own chip: a 2 px halo on each side of a full-height box
        assert!(check(
            r#"(items: [
                Row(w: 100.0, h: 40.0, pad: 0.0, items: [
                    Hit(key: 1.0, anchors: (left: -2.0, top: 4.0, right: 2.0, bottom: -4.0)),
                ]),
            ])"#
        )
        .is_empty());
    }

    #[test]
    fn a_runtime_sized_parent_is_not_proved_against() {
        // `w` is a binding / `auto` / a fill: the parent's extent is whatever
        // the shell hands it, so the anchor check must stand down, not guess
        assert!(check(
            r#"(items: [
                Row(w: "chip_w", h: 20.0, items: [
                    Hit(key: 1.0, anchors: (left: 40.0, top: 0.0, right: 0.0, bottom: 0.0)),
                ]),
                Row(w: 0.0, h: 20.0, items: [
                    Hit(key: 2.0, anchors: (left: 40.0, top: 0.0, right: 0.0, bottom: 0.0)),
                ]),
                Stack(w: 40.0, h: 20.0, auto: true, items: [
                    Hit(key: 3.0, anchors: (left: 40.0, top: 0.0, right: 0.0, bottom: 0.0)),
                ]),
            ])"#
        )
        .is_empty());
    }

    #[test]
    fn a_top_level_anchor_has_no_parent_extent_to_fail_against() {
        // the card box is a runtime size, so a top-level anchors cannot be
        // proved either way at load
        assert!(check(
            r#"(items: [Hit(key: 1.0, anchors: (left: 400.0, top: 0.0, right: 0.0, bottom: 0.0))])"#
        )
        .is_empty());
    }

    #[test]
    fn nesting_carries_the_grandparents_box_down() {
        assert!(!check(
            r#"(items: [
                Row(w: 40.0, h: 20.0, pad: 0.0, items: [
                    Stack(w: 0.0, h: 0.0, items: [
                        Hit(key: 1.0, anchors: (left: 40.0, top: 0.0, right: 0.0, bottom: 0.0)),
                    ]),
                ]),
            ])"#
        )
        .is_empty());
    }

    /// A pool with one model, `ctr`, whose rows carry the fields a pill row
    /// carries. The delegate check needs rows to compare against; without them
    /// it stands down, by design.
    fn pool_with_model() -> SceneValues {
        let mut v = full_pool();
        v.insert(
            "ctr",
            SceneValue::Model(vec![
                vec![
                    ("tag".into(), SceneValue::Text("ws".into())),
                    ("key".into(), SceneValue::Ring(4090.0)),
                    (
                        "ink".into(),
                        SceneValue::Color(crate::scene::SceneColor::Raw(0xff00_00)),
                    ),
                    ("w".into(), SceneValue::Ring(18.0)),
                    ("label".into(), SceneValue::Text("3".into())),
                ],
                vec![
                    ("tag".into(), SceneValue::Text("wifi".into())),
                    ("key".into(), SceneValue::Ring(4091.0)),
                    ("w".into(), SceneValue::Ring(16.0)),
                ],
            ]),
        );
        v
    }

    fn check_model(src: &str) -> Vec<LoadIssue> {
        let scene: CardScene = parse_scene_ron(src).expect("test scene parses");
        validate_items(&scene.items, &pool_with_model())
    }

    #[test]
    fn a_delegate_binding_a_field_no_row_carries_is_reported() {
        // the one "nobody publishes this?" question a scope makes answerable:
        // a model's rows are a CLOSED set, so this is a fact, not a guess
        let issues = check_model(
            r#"(items: [
                Repeat(name: "ctr_tags", model: "ctr", templates: [
                    (tag: "ws", items: [
                        Surface(x: 0.0, y: 0.0, w: "item.wn", h: 24.0, color: fg),
                    ]),
                ]),
            ])"#,
        );
        match issues.as_slice() {
            [LoadIssue::UnknownRowField {
                path,
                name,
                model,
                have,
            }] => {
                assert_eq!(path, "items[0]/templates[0]");
                assert_eq!(name, "item.wn");
                assert_eq!(model, "ctr");
                // the fix is a typo away, so the report lists what IS there
                assert!(
                    have.contains(&"w".to_string()),
                    "the row's real fields: {have:?}"
                );
            }
            other => panic!("expected one UnknownRowField, got {other:?}"),
        }
    }

    #[test]
    fn a_clean_delegate_says_nothing() {
        // every source the engine folds from is checked: a `Val` binding, a
        // `{token}` in a string, and a `value("…")` color
        assert!(check_model(
            r#"(items: [
                Repeat(name: "ctr_tags", model: "ctr", templates: [
                    (tag: "ws", items: [
                        Stack(x: 0.0, y: 0.0, w: "item.w", h: 0.0, items: [
                            Text(x: 2.0, y: "pill_mid", text: "{item.label}",
                                 font_size: 14.0, color: value("item.ink")),
                            Surface(x: 0.0, y: 7.0, w: 16.0, h: 24.0, r: 6.0,
                                    hover_key: "item.key", color: fg),
                            Hit(key: "item.key", x: 0.0, y: 7.0, w: 16.0, h: 24.0),
                        ]),
                    ]),
                    (tag: "*", items: []),
                ]),
            ])"#
        )
        .is_empty());
    }

    #[test]
    fn a_scope_name_is_not_asked_of_the_pool() {
        // `item.wn` above is a MODEL question, not a pool question: the pool
        // cannot answer it and must not claim the name is unpublished
        assert!(
            check_model(
                r#"(items: [
                Repeat(name: "ctr_tags", model: "ctr", templates: [
                    (tag: "ws", items: [
                        Text(x: "item.wnope", y: 0.0, text: "x"),
                    ]),
                ]),
            ])"#
            )
            .len()
                == 1
        );
        // a name with no dot IS a pool question, even inside a delegate: the
        // template's own geometry still reads the pool
        let issues = check_model(
            r#"(items: [
                Repeat(name: "ctr_tags", model: "ctr", templates: [
                    (tag: "ws", items: [
                        Text(x: "chip_widht", y: 0.0, text: "x"),
                    ]),
                ]),
            ])"#,
        );
        assert!(matches!(
            issues.as_slice(),
            [LoadIssue::UnknownProperty { name, .. }] if name == "chip_widht"
        ));
    }

    #[test]
    fn a_model_with_no_rows_stands_down() {
        // a conditional model (none of its widgets enabled) is an empty case,
        // not a typo, and the loader cannot tell the two apart
        let mut v = SceneValues::new();
        v.set_num("chip_w", 20.0);
        v.insert("ctr", SceneValue::Model(vec![]));
        v.mark_complete();
        let scene: CardScene = parse_scene_ron(
            r#"(items: [
                Repeat(name: "ctr_tags", model: "ctr", templates: [
                    (tag: "ws", items: [Text(x: "item.wn", y: 0.0, text: "x")]),
                ]),
            ])"#,
        )
        .expect("parses");
        assert!(validate_items(&scene.items, &v).is_empty());
    }

    /// Every `: Val,` field in the `SceneItem` schema, as
    /// `(variant, field)` — the same scan the walker's sweep uses.
    fn scene_item_val_fields() -> Vec<(String, String)> {
        let src = include_str!("../scene.rs");
        let mut in_enum = false;
        let mut variant = String::new();
        let mut schema: Vec<(String, String)> = Vec::new();
        for line in src.lines() {
            if line.starts_with("pub enum SceneItem") {
                in_enum = true;
                continue;
            }
            if !in_enum {
                continue;
            }
            if line == "}" {
                break;
            }
            let trimmed = line.trim_end();
            if let Some(rest) = trimmed.strip_prefix("    ") {
                let head = rest.split(['{', ',']).next().unwrap_or("").trim();
                if head.chars().next().is_some_and(char::is_uppercase) {
                    variant = head.to_string();
                    continue;
                }
            }
            if let Some(name) = trimmed.trim_start().strip_suffix(": Val,") {
                schema.push((variant.clone(), name.trim().to_string()));
            }
        }
        assert!(
            schema.len() >= 99,
            "the source scan found {schema:?} — the enum parse broke, not the schema"
        );
        schema
    }

    /// The body of a named `fn` in `src/scene.rs`, by its line.
    fn scene_fn_body(fname: &str) -> String {
        let src = include_str!("../scene.rs");
        let start = src
            .lines()
            .position(|l| l.contains(&format!("fn {fname}(")))
            .unwrap_or_else(|| panic!("no fn {fname} in scene.rs"));
        let rest = &src.lines().skip(start + 1).collect::<Vec<_>>().join("\n");
        let mut body = String::new();
        let mut depth = 1i32;
        for line in rest.lines() {
            for ch in line.chars() {
                match ch {
                    '{' => depth += 1,
                    '}' => depth -= 1,
                    _ => {}
                }
            }
            if depth <= 0 {
                break;
            }
            body.push_str(line);
            body.push('\n');
        }
        body
    }

    #[test]
    fn the_scope_name_scan_covers_every_val_field() {
        // the same drift guard as the walker's sweep, for the OTHER half of the
        // delegate check: `scope_names` is what says which fields a template
        // asks for, so a `Val` field it skips is a field whose misspelled
        // binding folds to 0 with nothing reported. The scan reads this crate's
        // own source for `: Val,` fields on `SceneItem` and requires each to be
        // destructured (and read) inside `collect_scope_names`.
        let body = scene_fn_body("collect_scope_names");
        let mut missing = Vec::new();
        for (variant, field) in scene_item_val_fields() {
            // the field has to be BOUND, not merely mentioned: a pattern that
            // names it and a `..` would pass a substring check while reading
            // nothing, so require the name followed by a non-identifier char
            let bound = format!("{field},");
            let renamed = format!("{field}:");
            if !body.contains(&bound) && !body.contains(&renamed) {
                missing.push(format!("{variant}.{field}"));
            }
        }
        assert!(
            missing.is_empty(),
            "the scope scan never reads {} — a misspelled binding in them folds to 0 \
             unreported",
            missing.join(", ")
        );
    }

    #[test]
    fn a_delegate_binding_that_names_the_scope_is_folded_not_reported() {
        // the two halves must not fight: `val` exempts scope names from the
        // pool question, `scope_names` owns them, and a field the rows carry
        // must produce no issue from either
        let issues = check_model(
            r#"(items: [
                Repeat(name: "ctr_tags", model: "ctr", templates: [
                    (tag: "ws", items: [
                        Stack(x: "index", y: 0.0, w: "item.w", h: 0.0, items: [
                            Text(x: 0.0, y: 0.0, text: "{item.label}"),
                            Surface(x: 0.0, y: 0.0, w: 1.0, h: 1.0, color: value("item.ink")),
                        ]),
                    ]),
                ]),
            ])"#,
        );
        assert_eq!(
            issues,
            Vec::<LoadIssue>::new(),
            "clean delegate: {issues:?}"
        );
    }

    #[test]
    fn every_declared_geometry_field_is_walked() {
        // the fields a RON can bind, per variant, checked against the walker:
        // a `Val` field nobody reads here is a field with no diagnostics
        for (variant, fields) in [
            (
                "Hit",
                "x: \"chip_w\", y: 0.0, w: 0.0, h: 0.0, key: 7.0",
            ),
            (
                "Text",
                "x: 0.0, y: 0.0, w: \"chip_w\", h: 0.0, font_size: \"chip_w\", text: \"t\"",
            ),
            ("TextWrap", "w: \"chip_w\", text: \"t\""),
            ("Surface", "x: \"chip_w\", y: 0.0, w: 0.0, h: 0.0, color: fg"),
            ("Scissor", "x: \"chip_w\", y: 0.0, w: 0.0, h: 0.0"),
            ("Divider", "x: \"chip_w\", y: 0.0, w: 0.0"),
            ("Bar", "x: \"chip_w\", y: 0.0, w: 0.0, h: 0.0, value: \"c\", max: 1.0"),
            ("Battery", "x: \"chip_w\", y: 0.0, w: 0.0, h: 0.0, value: \"c\""),
            (
                "Rows",
                "name: \"model_rows\", y: \"chip_w\", w: \"chip_w\", row_h: 20.0, cols: [], del_color: None",
            ),
            ("Spark", "name: \"model_rows\", x: \"chip_w\", y: 0.0, w: 0.0, h: 0.0"),
            ("Spectrum", "name: \"model_rows\", kind: rounded, x: \"chip_w\", y: 0.0, w: 0.0, h: 0.0"),
            ("Moon", "phase: \"waxing\", x: \"chip_w\", y: 0.0, d: \"chip_w\""),
            ("Fader", "name: \"model_rows\", x: \"chip_w\", y: 0.0, w: 0.0, h: 0.0"),
            ("Toggle", "name: \"model_rows\", x: \"chip_w\", y: 0.0, w: 0.0, h: 0.0"),
            ("Dots", "x: \"chip_w\", active: \"model_rows\""),
            ("TabRow", "name: \"model_rows\", y: \"chip_w\", h: \"chip_w\""),
            ("Composer", "y: \"chip_w\", w: \"chip_w\", h: \"chip_w\""),
            ("Strip", "chips: \"model_rows\", x: \"chip_w\", y: 0.0, w: 0.0, h: 0.0"),
            ("BannerRow", "cells: \"model_rows\", x: \"chip_w\", y: 0.0, w: 0.0, h: 0.0"),
            ("Ink", "name: \"panel\", x: \"chip_w\", y: 0.0, w: 0.0, h: 0.0"),
            ("Row", "x: \"chip_w\", y: 0.0, w: 0.0, h: 0.0, items: []"),
            ("Column", "x: \"chip_w\", y: 0.0, w: 0.0, h: 0.0, items: []"),
            ("Stack", "x: \"chip_w\", y: 0.0, w: 0.0, h: 0.0, items: []"),
            ("Repeat", "name: \"model_rows\", x: \"chip_w\", y: 0.0, w: 0.0, h: 0.0, templates: []"),
            ("Grid", "name: \"model_rows\", x: \"chip_w\", y: 0.0, w: 0.0, h: 0.0"),
            ("Tiles", "name: \"model_rows\", x: \"chip_w\", y: 0.0, w: 0.0, h: 0.0"),
            ("Image", "key: \"icon:net\", x: \"chip_w\", y: 0.0, w: 0.0, h: 0.0"),
            ("Comp", "name: \"c\", x: \"chip_w\", y: 0.0"),
        ] {
            let scene: CardScene =
                parse_scene_ron(&format!("(items: [{variant}({fields})])")).expect("parses");
            let issues = validate_items(&scene.items, &full_pool());
            assert_eq!(
                issues,
                Vec::<LoadIssue>::new(),
                "{variant}: every bound field must resolve ({issues:?})"
            );
        }
    }
}
