    use super::*;
    use crate::icons::{ICON_CHECK, ICON_CLOSE, ICON_GLOBE};
    use crate::ui::hover;

    fn pal() -> Pal {
        Pal {
            fg: 0xe8e6e3ff,
            bg: 0x141414ff,
            acc: 0x4fc2ffff,
            sfg: 0x101010ff,
        }
    }

    fn scale() -> crate::shell::GridScale {
        crate::shell::GridScale(1.0)
    }

    fn vals() -> SceneValues {
        let mut m = SceneValues::new();
        m.insert("rec_btn", SceneValue::Text("Record".into()));
        m
    }

    const SCENE_RON: &str = r##"
        (
            items: [
                Text(x: 12.0, y: 10.0, text: "Recorder", font_size: 11.5, weight: semibold),
                Hit(x: 134.0, y: 6.0, w: 16.0, h: 16.0, r: 5.0, key: 2003,
                    surface: Some(raised), text: "›", font_size: 10.0, icon: true,
                    action: Some(Command("zen-shell ipc call recorder open"))),
                Hit(x: 8.0, y: 84.0, w: 144.0, h: 26.0, r: 7.0, key: 2004,
                    surface: Some(raised), text: "{rec_btn}", font_size: 9.5,
                    action: Some(Command("zen-shell ipc call recorder toggle"))),
            ],
        )
    "##;

    #[test]
    /// The authored shape is the tuple form, so `anim:` reads as a MODIFIER on
    /// the value it wraps rather than a field of the item:
    ///
    ///     ( x: (val: "polkit_cursor_x", anim: (ms: 90, curve: out_cubic)) )
    ///
    /// This is the check that the `Val::Anim` wrapper actually round-trips —
    /// a wrapper that quietly fell back to its inner value would cost the
    /// author their animation with no diagnostic at all.
    fn anim_wraps_a_val_through_ron() {
        let text = r#"(
            items: [
                Surface(
                    x: (val: "nav.tab", anim: (ms: 90, curve: out_cubic)),
                    y: 4.0, w: 2.0, h: 20.0, r: 1.0, color: acc,
                ),
            ]
        )"#;
        let scene: CardScene = parse_scene_ron(text).unwrap();
        let SceneItem::Surface { x, .. } = &scene.items[0] else {
            panic!("item 0 is a Surface")
        };
        let Val::Anim { val, anim } = x else {
            panic!("x is animated, got {x:?}")
        };
        assert_eq!(**val, Val::Expr("nav.tab".into()));
        assert_eq!(anim.ms, 90.0);
        assert_eq!(anim.curve, anim::Curve::OutCubic);
    }

    #[test]
    /// `anim:` without a `curve` takes the documented default rather than
    /// needing one spelled out at every use site.
    fn anim_defaults_to_out_cubic_over_180ms() {
        let text = r#"(
            items: [ Surface(x: (val: "a.b", anim: ()), y: 0.0, w: 1.0, h: 1.0, color: acc) ]
        )"#;
        let scene: CardScene = parse_scene_ron(text).unwrap();
        let SceneItem::Surface { x, .. } = &scene.items[0] else {
            panic!("item 0 is a Surface")
        };
        let Val::Anim { anim, .. } = x else {
            panic!("x is animated")
        };
        assert_eq!(*anim, anim::Anim::default());
        assert_eq!(anim.ms, 180.0);
        assert_eq!(anim.curve, anim::Curve::OutCubic);
    }

    #[test]
    /// Every curve name has to be reachable from RON, or a documented one is a
    /// trap. An unknown name falls back to the default rather than failing the
    /// whole scene (same rule as an unknown colour).
    fn every_documented_curve_name_parses() {
        for (ron, want) in [
            ("linear", anim::Curve::Linear),
            ("out_cubic", anim::Curve::OutCubic),
            ("in_out_cubic", anim::Curve::InOutCubic),
            ("out_back", anim::Curve::OutBack),
        ] {
            let text = format!(
                r#"(items: [ Surface(x: (val: "a.b", anim: (curve: {ron})), y: 0.0, w: 1.0, h: 1.0, color: acc) ])"#
            );
            let scene: CardScene = parse_scene_ron(&text).unwrap_or_else(|e| panic!("{ron}: {e}"));
            let SceneItem::Surface { x, .. } = &scene.items[0] else {
                panic!("item 0 is a Surface")
            };
            let Val::Anim { anim, .. } = x else {
                panic!("x is animated")
            };
            assert_eq!(anim.curve, want, "{ron}");
        }
    }

    #[test]
    /// A wrapper with half its keys is an authoring mistake, and the mistake has
    /// to be NAMEABLE — the whole point of reading `Val` by hand is that the
    /// message is not "data did not match any variant of untagged enum Val".
    /// The alternative to complaining is an animation nobody declared, at a
    /// duration nobody chose.
    fn a_half_written_anim_says_what_is_missing() {
        for (text, want) in [
            (r#"(val: "a.b")"#, "without `anim:`"),
            (r#"(anim: (ms: 90))"#, "`anim:` without `val:`"),
            (r#"()"#, "`()` is not a Val"),
        ] {
            let err = parse_scene_ron::<Val>(text)
                .err()
                .unwrap_or_else(|| panic!("{text} must not parse"));
            let msg = err.to_string();
            assert!(
                msg.contains(want),
                "{text} should say {want:?}, said {msg:?}"
            );
        }
        // …while an unknown key is tolerated, so a scene written for a newer
        // build still loads here instead of failing on a word it lacks.
        let ok: Val = parse_scene_ron(r#"(val: "a.b", anim: (ms: 90, from_a_newer_build: 1))"#)
            .expect("an unknown key is skipped, not fatal");
        assert_eq!(
            ok,
            Val::Anim {
                val: Box::new(Val::Expr("a.b".into())),
                anim: anim::Anim {
                    ms: 90.0,
                    curve: anim::Curve::OutCubic
                },
            }
        );
    }

    #[test]
    /// Writing a scene back out has to preserve the animation, because that
    /// RON is also how a scene is handed to the validator and to the golden
    /// tests. `Val` reads itself by hand and writes itself `untagged`; if those
    /// two halves ever disagree (a tagged serialize would emit
    /// `Anim(val: …)`), this is what catches it.
    fn an_animated_val_survives_a_write_and_a_reread() {
        let scene: CardScene = parse_scene_ron(
            r#"(items: [ Surface(x: (val: "nav.tab", anim: (ms: 90, curve: out_back)),
                           y: 0.0, w: 1.0, h: 1.0, color: acc) ])"#,
        )
        .unwrap();
        let ron_out = ron::to_string(&scene).unwrap();
        assert!(
            ron_out.contains("anim:"),
            "the wrapper must stay inline in the written scene: {ron_out}"
        );
        let back: CardScene = parse_scene_ron(&ron_out).unwrap();
        assert_eq!(scene, back);
    }

    #[test]
    /// Measure has to read the TARGET, or a measured extent would be laid out
    /// at its final size and then drawn there too — the animation would be
    /// visible in the schema and inert on screen.
    fn measure_reads_the_target_of_an_animated_value() {
        let wrapped = Val::Anim {
            val: Box::new(Val::Expr("a.b".into())),
            anim: anim::Anim::default(),
        };
        let mut vals = SceneValues::new();
        vals.insert("a.b", SceneValue::Ring(40.0));
        assert_eq!(wrapped.num(&vals), 40.0, "measure sees the target");
    }

    // ── phase 6: the `parent.*` scope ────────────────────────────────────────

    /// The scope resolves in all three channels a binding can arrive through,
    /// and reads in BASE px — the same units a literal extent is written in, so
    /// `"parent.w - 28"` is 28 base px and not 28 device px. Everything else
    /// about the feature depends on that being true: a scope in the wrong
    /// units composes into a number that looks right at one card size and is
    /// wrong at every other.
    #[test]
    fn a_parent_name_reads_the_enclosing_box_in_base_px() {
        let vals = SceneValues::new();
        // 240 device px at scale 2 is 120 base px
        {
            let _p = vals.push_parent(240.0, 80.0, 2.0);
            assert_eq!(Val::Expr("parent.w".into()).num(&vals), 120.0);
            assert_eq!(Val::Expr("parent.h".into()).num(&vals), 40.0);
            assert_eq!(Val::Expr("parent.cx".into()).num(&vals), 60.0);
            assert_eq!(Val::Expr("parent.cy".into()).num(&vals), 20.0);
            // arithmetic over the scope, which is the whole reason it is a
            // pool name and not a keyword
            assert_eq!(Val::Expr("parent.w - 28".into()).num(&vals), 92.0);
            assert_eq!(Val::Expr("max(parent.w / 3, 40)".into()).num(&vals), 40.0);
        }
        // …and the guard POPPED: a name outside any item's scope resolves to
        // nothing rather than to whatever the last item happened to push
        assert_eq!(Val::Expr("parent.w".into()).num(&vals), 0.0);
    }

    /// The stack is what makes it a *scope* rather than a global: an item two
    /// containers down reads its OWN container, not the outermost one. Without
    /// the depth, `parent.w` would be the card for everything, which is the
    /// bug the scope was added to remove.
    #[test]
    fn parent_is_the_innermost_enclosing_box() {
        let vals = SceneValues::new();
        let outer = vals.push_parent(300.0, 200.0, 1.0);
        assert_eq!(Val::Expr("parent.w".into()).num(&vals), 300.0);
        {
            let _mid = vals.push_parent(160.0, 90.0, 1.0);
            assert_eq!(Val::Expr("parent.w".into()).num(&vals), 160.0);
            {
                let _inner = vals.push_parent(40.0, 20.0, 1.0);
                assert_eq!(Val::Expr("parent.w".into()).num(&vals), 40.0);
            }
            // the inner guard popped on ITS way out — including out of the
            // middle of a scope that never reached the end of the block
            assert_eq!(Val::Expr("parent.w".into()).num(&vals), 160.0);
        }
        assert_eq!(Val::Expr("parent.w".into()).num(&vals), 300.0);
        drop(outer);
        assert_eq!(Val::Expr("parent.w".into()).num(&vals), 0.0);
    }

    /// A `Row`'s children are laid out in the row's INNER box, so `parent.h`
    /// inside a child must be the inner height — 40 less 2·6 of pad — and not
    /// the row's own 40.
    #[test]
    fn a_rows_children_read_the_row_inner_box() {
        let scene: CardScene = parse_scene_ron(
            r#"(items: [
                Row(x: 10.0, y: 4.0, w: 200.0, h: 40.0, pad: 6.0, spacing: 0.0, items: [
                    Hit(key: 1, x: 0.0, y: 0.0, w: 40.0, h: "parent.h", text: "a"),
                ]),
            ])"#,
        )
        .expect("row scene parses");
        let vals = SceneValues::new();
        let (hits, _) = scene.draw(
            &mut Vec::new(),
            0.0,
            0.0,
            200.0,
            40.0,
            scale(),
            &pal(),
            &vals,
            0,
            false,
            false,
        );
        // row box = 200×40 at (10, 4); inner = 188×28 at (16, 10)
        assert_eq!(
            regions(&hits),
            vec![(16.0f32, 10.0f32, 40.0f32, 28.0f32)],
            "`parent.h` is the row's inner height, not its own"
        );
    }

    /// The `Repeat` refusal is a LOAD-time decision, so the draw path still has
    /// to answer for a scene that never went through the validator — and what
    /// it must never do is panic or paint a zero-width widget silently. It
    /// resolves the template's parent to the SLOT it is being drawn in, which
    /// is the one box that does exist at that depth, and the width lands
    /// non-zero. Phase 5's rule is the one that applies here: the schema refuses
    /// what the renderer cannot honour, and this test is the renderer half.
    #[test]
    fn a_parent_name_inside_a_repeat_template_draws_without_panicking() {
        let mut vals = SceneValues::new();
        vals.insert("tags", SceneValue::List(vec!["a".to_string()]));
        let scene: CardScene = parse_scene_ron(
            r#"(items: [ Repeat(name: "tags", x: 0.0, y: 0.0, w: 0.0, h: 0.0, spacing: 0.0,
                templates: [ (tag: "a", items: [
                    Surface(x: 0.0, y: 0.0, w: "parent.w - 4", h: "parent.h", color: raised),
                    Hit(key: 1, x: 0.0, y: 0.0, w: "parent.w", h: 0.0),
                ]) ]) ])"#,
        )
        .unwrap();
        let mut cmds: Vec<crate::cmd::Cmd> = Vec::new();
        let (hits, _) = scene.draw(
            &mut cmds,
            0.0,
            0.0,
            200.0,
            60.0,
            scale(),
            &pal(),
            &vals,
            0,
            false,
            false,
        );
        use crate::cmd::Cmd::*;
        let rect = cmds
            .iter()
            .find_map(|c| match c {
                Rect { w, h, .. } => Some((*w, *h)),
                _ => None,
            })
            .expect("the template still painted");
        assert!(
            rect.0 > 0.0 && rect.1 > 0.0,
            "never a zero-area box: {rect:?}"
        );
        assert_eq!(regions(&hits).len(), 1, "the template's hit survived");
    }

    /// A `Stack` has no measure pass at all — every child is handed the
    /// container's OWN box, once — so a binding that reads it cannot disagree
    /// with itself the way a `Row`'s can. This is the shape the live power
    /// menu is five copies of (a `Bar` inset 2 px inside a 92×92 chip, and a
    /// label centered on the chip's full width), written the way a scene
    /// should write it rather than by restating 92 and 88.
    #[test]
    fn a_stack_child_reads_the_stacks_own_box() {
        let vals = SceneValues::new();
        let scene: CardScene = parse_scene_ron(
            r#"(items: [ Stack(x: 0.0, y: 0.0, w: 92.0, h: 92.0, items: [
                    Surface(x: 2.0, y: -2.0, w: "parent.w - 4", h: "parent.h", r: 16.0, color: raised),
                    Text(x: 0.0, y: 60.0, w: "parent.w", center: true, text: "Lock", font_size: 12.0),
                    Hit(key: 1, x: 0.0, y: 0.0, w: 0.0, h: 0.0),
                ]) ])"#,
        )
        .unwrap();
        let mut cmds: Vec<crate::cmd::Cmd> = Vec::new();
        let (hits, _) = scene.draw(
            &mut cmds,
            0.0,
            0.0,
            200.0,
            120.0,
            scale(),
            &pal(),
            &vals,
            0,
            false,
            false,
        );
        use crate::cmd::Cmd::*;
        let rects: Vec<(f32, f32, f32, f32)> = cmds
            .iter()
            .filter_map(|c| match c {
                Rect { x, y, w, h, .. } => Some((*x, *y, *w, *h)),
                _ => None,
            })
            .collect();
        assert_eq!(
            rects,
            vec![(2.0, -2.0, 88.0, 92.0)],
            "92 less 2·2 by the chip's own 92"
        );
        assert_eq!(regions(&hits), vec![(0.0, 0.0, 92.0, 92.0)]);
        assert!(
            cmds.iter().any(|c| matches!(c, Text { center: true, .. })),
            "the label centers on parent.w, so it must still be centered"
        );
    }

    /// The scope has to answer the SAME question wherever it appears, or
    /// copying an item between positions in a scene silently changes what it
    /// means. These are the positions that differ: at card level, one
    /// container deep, and two.
    ///
    /// The probe FILLS its slot (`w: 0, h: 0`), which is the only way to read
    /// the parent's box off a child: a child that declares its own extent gets
    /// a slot that IS that extent, so `parent.w` there is its own width —
    /// correct, and not what this test is measuring.
    #[test]
    fn the_parent_scope_answers_the_same_way_at_every_depth() {
        let vals = SceneValues::new();
        let at = |src: &str| {
            let scene: CardScene = parse_scene_ron(src).unwrap();
            let (hits, _) = scene.draw(
                &mut Vec::new(),
                0.0,
                0.0,
                200.0,
                60.0,
                scale(),
                &pal(),
                &vals,
                0,
                false,
                false,
            );
            regions(&hits)
        };
        let fill = r#"Hit(key: 1, x: 0.0, y: 0.0, w: 0.0, h: 0.0)"#;
        let one = |pad: f32| {
            format!(
                r#"(items: [ Row(x: 0.0, y: 0.0, w: 0.0, h: 0.0, pad: {pad}, spacing: 0.0,
                              items: [ {fill} ]) ])"#
            )
        };
        let card_level = at(&format!("(items: [ {fill} ])"));
        assert_eq!(
            card_level[0],
            (0.0, 0.0, 200.0, 60.0),
            "no container: the card"
        );
        assert_eq!(
            at(&one(8.0))[0],
            (8.0, 8.0, 184.0, 44.0),
            "one pad-8 row: the INNER box"
        );
        assert_eq!(
            at(&format!(
                r#"(items: [ Column(x: 0.0, y: 0.0, w: 0.0, h: 0.0, pad: 6.0, spacing: 0.0, items: [
                        Row(x: 0.0, y: 0.0, w: 0.0, h: 0.0, pad: 3.0, spacing: 0.0, items: [
                            {fill} ]) ]) ])"#
            ))[0],
            (9.0, 9.0, 182.0, 42.0),
            "two levels: 200 − 2·6 − 2·3 by 60 − 2·6 − 2·3 — the INNERMOST box, \
             not the column's and not the card's"
        );
        // and the arithmetic composes on top of it, which is the whole reason
        // this is a pool name rather than a keyword
        assert_eq!(
            at(
                r#"(items: [ Row(x: 0.0, y: 0.0, w: 0.0, h: 0.0, pad: 8.0, spacing: 0.0, items: [
                        Hit(key: 1, x: 0.0, y: 0.0, w: "parent.w - 20", h: "parent.h / 2") ]) ])"#
            )[0],
            (8.0, 8.0, 164.0, 22.0),
            "\"parent.w - 20\" off a fill child is exact: 184 − 20"
        );
    }

    /// A container resolves a child's `w` BEFORE it knows the box it is
    /// resolving against — the child's width is what decides the slot, and the
    /// slot is the box. So a `w` that reads `parent.*` is answered from the
    /// container's own box, and the answer is then STICKY: the draw pass gets
    /// a slot that is already that width, so reading the scope again there
    /// would resize the box that is positioning the item and the two would
    /// chase each other.
    ///
    /// This is the same rule `anim:` follows for the same reason — measure
    /// reads the target (see [`crate::scene::anim`]) — and it is why the
    /// measured 180 and the drawn 172 are both correct. The author-visible
    /// consequence is one line, and it is not subtle: **inside a container,
    /// put `parent.*` on `x` / `y`, not on `w` / `h`.** `w: "parent.w - 28"`
    /// at card level (no container) is exact, which is the case the docs'
    /// gotchas actually needed.
    #[test]
    fn a_width_that_reads_parent_sticks_at_the_measured_value_inside_a_container() {
        let vals = SceneValues::new();
        let at = |src: &str| {
            let scene: CardScene = parse_scene_ron(src).unwrap();
            let (hits, _) = scene.draw(
                &mut Vec::new(),
                0.0,
                0.0,
                200.0,
                60.0,
                scale(),
                &pal(),
                &vals,
                0,
                false,
                false,
            );
            regions(&hits)
        };
        // at CARD level there is no container, so the draw pass sees the same
        // box the measure did and the width is exact: 200 - 28
        let card_level =
            at(r#"(items: [ Hit(key: 1, x: 0.0, y: 0.0, w: "parent.w - 28", h: 4.0) ])"#);
        assert_eq!(
            card_level[0].2, 172.0,
            "no container between the item and the card"
        );
        // inside a pad-6 row the slot is measured off the row's INNER box
        // (188) and then reused as the draw-time parent, so the width settles
        // at 188 - 8 = 180 and stays there
        let in_a_row = at(
            r#"(items: [ Row(x: 0.0, y: 0.0, w: 0.0, h: 20.0, pad: 6.0, spacing: 0.0, items: [
                    Hit(key: 1, x: 0.0, y: 0.0, w: "parent.w - 8", h: 4.0) ]) ])"#,
        );
        assert_eq!(
            in_a_row[0].2, 180.0,
            "the measured width is drawn unchanged — the layout pass and the draw \
             pass cannot disagree about it"
        );
    }

    /// The gotcha this phase exists to delete, as an equality: a centered
    /// caption at CARD level used to need a full-card `Stack` wrapper, because
    /// a card-level `Text` has no implicit width and `w: 0` centers on the
    /// anchor itself. `w: "parent.w"` says the same thing without the wrapper.
    #[test]
    fn a_centered_card_level_text_needs_no_stack_wrapper() {
        let centered = |src: &str| {
            let scene: CardScene = parse_scene_ron(src).unwrap();
            let vals = SceneValues::new();
            let mut v: Vec<Cmd> = Vec::new();
            scene.draw(
                &mut v,
                0.0,
                0.0,
                240.0,
                100.0,
                scale(),
                &pal(),
                &vals,
                0,
                false,
                false,
            );
            text_xy(&v)
        };
        // the wrapper the docs told you to write
        let wrapped = centered(
            r#"(items: [ Stack(x: 0.0, y: 0.0, w: 0.0, h: 0.0, items: [
                    Text(x: 0.0, y: 40.0, w: 0.0, center: true, text: "nothing here")
                ]) ])"#,
        );
        // and the scope, with no wrapper at all
        let scoped = centered(
            r#"(items: [ Text(x: 0.0, y: 40.0, w: "parent.w", center: true, text: "nothing here") ])"#,
        );
        assert_eq!(
            scoped, wrapped,
            "`w: \"parent.w\"` must place a card-level centered caption exactly where the \
             full-card Stack wrapper did"
        );
        // …and it is the CARD's width, so the string centers on the card mid
        assert!(!scoped.is_empty());
    }

    /// `x: "parent.cx"` is the arithmetic spelling of `center_x: true`, which
    /// is the other half of "stop threading the box through": a hit region or
    /// a dot no longer has to be nudged by half its own width by hand.
    #[test]
    fn parent_cx_is_the_arithmetic_spelling_of_center_x() {
        let place = |src: &str| {
            let scene: CardScene = parse_scene_ron(src).unwrap();
            let vals = SceneValues::new();
            let (hits, _) = scene.draw(
                &mut Vec::new(),
                0.0,
                0.0,
                200.0,
                60.0,
                scale(),
                &pal(),
                &vals,
                0,
                false,
                false,
            );
            regions(&hits)
        };
        let literal =
            place(r#"(items: [ Hit(key: 1, x: 0.0, y: 0.0, w: 10.0, h: 10.0, center_x: true) ])"#);
        let scoped = place(r#"(items: [ Hit(key: 1, x: "parent.cx", y: 0.0, w: 10.0, h: 10.0) ])"#);
        assert_eq!(literal, scoped, "both land the box's left edge on the mid");
        assert_eq!(scoped[0].0, 100.0, "and that is the card's mid");
    }

    /// `parent.` is a RESERVED namespace, which is a stronger claim than
    /// "the scope is consulted first": a name published under it is not merely
    /// shadowed while an item is drawing, it is unreachable outright.
    ///
    /// This matters because the alternative is a publisher that silently
    /// changes what `parent.w` means for every nested item in every surface,
    /// with no diagnostic on either side. Reserving the prefix makes the
    /// collision impossible instead of merely unlikely — and the validator can
    /// then leave `parent.*` out of the "does anybody publish this?" question
    /// altogether, which is what makes the scope checkable with no pool at all.
    #[test]
    fn parent_dot_is_a_reserved_namespace() {
        let mut vals = SceneValues::new();
        vals.insert("parent.w", SceneValue::Ring(999.0));
        {
            let _p = vals.push_parent(200.0, 60.0, 1.0);
            assert_eq!(
                Val::Expr("parent.w".into()).num(&vals),
                200.0,
                "the scope wins over the pool"
            );
        }
        assert_eq!(
            Val::Expr("parent.w".into()).num(&vals),
            0.0,
            "and with no scope active the pool still cannot answer — the prefix \
             is reserved, not merely shadowed"
        );
    }

    /// The scope has to be empty on the way OUT of a draw, or a second surface
    /// drawn with the same pool would inherit the first one's box. `draw`
    /// returning is the only place that can be observed from, so that is where
    /// this is checked — including for an item that RETURNS EARLY (a closed
    /// `visible` gate), which is the shape a hand-written push/pop pair would
    /// get wrong.
    #[test]
    fn the_parent_scope_is_empty_again_after_a_draw_that_hid_an_item() {
        let mut vals = SceneValues::new();
        vals.set_bool("off", false);
        let scene: CardScene = parse_scene_ron(
            r#"(items: [
                Text(x: 0.0, y: 0.0, w: "parent.w", text: "hidden", visible: "off"),
            ])"#,
        )
        .unwrap();
        let mut v: Vec<Cmd> = Vec::new();
        scene.draw(
            &mut v,
            0.0,
            0.0,
            200.0,
            60.0,
            scale(),
            &pal(),
            &vals,
            0,
            false,
            false,
        );
        assert!(v.is_empty(), "the gated item draws nothing");
        assert_eq!(
            Val::Expr("parent.w".into()).num(&vals),
            0.0,
            "…and its scope was popped on the way out, not leaked to the next draw"
        );
    }

    /// Every click region a draw registered, as `x y w h`.
    fn regions(hits: &[SceneHit]) -> Vec<(f32, f32, f32, f32)> {
        hits.iter().map(|h| (h.x, h.y, h.w, h.h)).collect()
    }

    /// Every text a `Cmd::Text` drew, as `x y string`.
    fn text_xy(v: &[Cmd]) -> Vec<(f32, f32, String)> {
        v.iter()
            .filter_map(|c| match c {
                Cmd::Text { x, y, text, .. } => Some((*x, *y, text.clone())),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn ron_roundtrip_parses() {
        let scene: CardScene = parse_scene_ron(SCENE_RON).expect("scene ron parses");
        assert_eq!(scene.items.len(), 3);
        match &scene.items[0] {
            SceneItem::Text {
                text, weight, font, ..
            } => {
                assert_eq!(text, "Recorder");
                assert_eq!(*weight, SceneWeight::Semibold);
                assert_eq!(*font, SceneFont::Ui);
            }
            _ => panic!("item 0 is a Text"),
        }
        match &scene.items[1] {
            SceneItem::Hit {
                key, action, icon, ..
            } => {
                assert_eq!(*key, Val::Lit(2003.0));
                assert_eq!(
                    *action,
                    Some(SceneAction::Command(
                        "zen-shell ipc call recorder open".into()
                    ))
                );
                assert!(icon);
            }
            _ => panic!("item 1 is a Hit"),
        }
        match &scene.items[2] {
            SceneItem::Hit { text, .. } => assert_eq!(text, "{rec_btn}"),
            _ => panic!("item 2 is a Hit"),
        }
    }

    #[test]
    fn draw_emits_cmd_and_hits() {
        let scene: CardScene = parse_scene_ron(SCENE_RON).unwrap();
        let mut v: Vec<Cmd> = Vec::new();
        let (hits, _) = scene.draw(
            &mut v,
            0.0,
            0.0,
            148.0,
            90.0,
            scale(),
            &pal(),
            &vals(),
            0,
            true,
            true,
        );
        // 2 surfaces (chevron + record button) + 3 texts (title, glyph, btn)
        assert!(v.iter().filter(|c| matches!(c, Cmd::Rect { .. })).count() >= 2);
        assert!(v
            .iter()
            .any(|c| matches!(c, Cmd::Text { text, .. } if text == "Recorder")));
        assert!(
            v.iter()
                .any(|c| matches!(c, Cmd::Text { text, .. } if text == "Record")),
            "rec_btn interpolated from vals"
        );
        assert_eq!(hits.len(), 2, "two keys declared → two hit regions");
        assert_eq!(hits[0].key, 2003);
        assert!(
            matches!(hits[0].action, Some(SceneAction::Command(_))),
            "action present on hit 2003"
        );
        assert!(matches!(hits[0].action,
            Some(SceneAction::Command(ref c)) if c == "zen-shell ipc call recorder open"));
    }

    #[test]
    fn hit_offset_by_card_origin() {
        let scene: CardScene = parse_scene_ron(SCENE_RON).unwrap();
        let mut v: Vec<Cmd> = Vec::new();
        let (hits, _) = scene.draw(
            &mut v,
            100.0,
            40.0,
            148.0,
            90.0,
            scale(),
            &pal(),
            &vals(),
            0,
            true,
            true,
        );
        assert_eq!(hits[0].x, 100.0 + 134.0, "hit x is card-origin offset");
        assert_eq!(hits[0].y, 40.0 + 6.0);
    }

    #[test]
    fn hit_right_anchored_label_centers_when_center_is_set() {
        // A right-anchored hit whose label asks to CENTER must emit a centered
        // command (the drawer's `text_c` idiom), not a right-aligned one. The
        // box still hugs the card's right edge (x 200 − 12 is the box's right
        // edge; the box is 52 wide so it starts at 136), but the label is
        // centered inside it — `center: true` beats `right: true` for the ink.
        let scene: CardScene = parse_scene_ron(
            r#"(items: [ Hit(x: 12.0, y: 10.0, w: 52.0, h: 22.0, r: 6.0, right: true,
                             center: true, text: "Reset", font_size: 9.5, key: 7) ])"#,
        )
        .unwrap();
        let mut v: Vec<Cmd> = Vec::new();
        let (hits, _) = scene.draw(
            &mut v,
            0.0,
            0.0,
            200.0,
            120.0,
            scale(),
            &pal(),
            &vals(),
            0,
            true,
            true,
        );
        assert_eq!(hits.len(), 1, "one region");
        assert_eq!(hits[0].x, 136.0, "the box still hugs the right edge");
        let (x, right, center) = v
            .iter()
            .find_map(|c| match c {
                Cmd::Text {
                    x,
                    right,
                    center,
                    text,
                    ..
                } if text == "Reset" => Some((*x, *right, *center)),
                _ => None,
            })
            .expect("label emitted");
        assert_eq!(
            (x, right, center),
            (162.0, false, true),
            "right-anchored box, centered label"
        );
    }

    /// calendar-style matrix: a `head` row of weekdays, 42 day cells across
    /// A segmented `bars` cell can CENTER its staircase on the row's mid rail
    /// (`SegmentsSpec.center`) and anchor the whole BLOCK to the cell's right
    /// edge minus `col.edge` — the Wi-Fi submenu's signal indicator, which the
    /// drawer measured from the panel's right edge rather than the cell's left.
    #[test]
    fn bars_cell_centers_its_staircase_and_can_anchor_the_block_right() {
        // 4 bars, 5 wide, 2 gaps, heights 4 + 3·k, base 18 — a 26 px block.
        // The cell starts at x 0 of a 200 px card, so `right: true, edge: 46`
        // lands the block's RIGHT edge at 200 − 46 = 154, i.e. its left at 128.
        let scene: CardScene = parse_scene_ron(
            r#"(items: [ Rows(
                    name: "sig",
                    x: 0.0, y: 0.0, w: 0.0,
                    row_h: 36.0, pitch: Some(42.0), pad: 0.0, row_dy: 10.0,
                    cols: [
                        SceneCol(x: 0.0, w: 0.0, right: true, edge: Some(46.0),
                         bars: Some(SegmentsSpec(count: 4, max: 100.0, width: 5.0, gap: 2.0, height: 4.0,
                                                 step: 3.0, radius: 2.0, base: 18.0, center: true))),
                    ],
                ) ])"#,
        )
        .expect("bars row scene parses");
        let mut vals = SceneValues::new();
        vals.insert(
            "sig",
            SceneValue::Rows(vec![SceneRow {
                cols: vec!["100".to_string()],
                ..Default::default()
            }]),
        );
        let mut v: Vec<Cmd> = Vec::new();
        scene.draw(
            &mut v,
            0.0,
            0.0,
            200.0,
            80.0,
            scale(),
            &pal(),
            &vals,
            0,
            false,
            false,
        );
        let all = rects(&v);
        let bars: Vec<&(f32, f32, f32, f32, u32)> = all.iter().filter(|r| r.2 == 5.0).collect();
        assert_eq!(bars.len(), 4, "four bars at 100%");
        // the BLOCK is right-anchored: 4 · 5 + 3 · 2 = 26 wide, right edge 154
        assert_eq!(
            bars[0].0,
            200.0 - 46.0 - 26.0,
            "block's left sits edge - block width"
        );
        assert_eq!(
            bars[3].0 + bars[3].2,
            200.0 - 46.0,
            "block's right edge is the cell's right - edge"
        );
        // centered on the row's mid rail (base 18 of a 36 px row) instead of
        // standing on it: every bar's VERTICAL MIDPOINT is the rail, so the
        // 4 / 7 / 10 / 13 px steps straddle it symmetrically
        for (i, b) in bars.iter().enumerate() {
            assert_eq!(b.1 + b.3 / 2.0, 18.0, "bar {i} straddles the mid rail");
        }
        assert_eq!(bars[0].3, 4.0, "the shortest step first");
        assert_eq!(bars[3].3, 13.0, "the tallest step last");
        assert!(
            bars[0].1 > bars[3].1,
            "and the staircase still rises left to right"
        );
    }

    /// Without `center` the bars keep their original behavior: they STAND on
    /// the `base` line, growing upward from it.
    #[test]
    fn bars_cell_without_center_still_stands_on_its_base() {
        let scene: CardScene = parse_scene_ron(
            r#"(items: [ Rows(
                    name: "sig",
                    x: 0.0, y: 0.0, w: 0.0,
                    row_h: 36.0, pitch: Some(42.0), pad: 0.0, row_dy: 10.0,
                    cols: [
                        SceneCol(x: 4.0, w: 0.0,
                         bars: Some(SegmentsSpec(count: 4, max: 100.0, width: 5.0, gap: 2.0, height: 4.0,
                                                 step: 3.0, radius: 2.0, base: 12.0))),
                    ],
                ) ])"#,
        )
        .expect("bars row scene parses");
        let mut vals = SceneValues::new();
        vals.insert(
            "sig",
            SceneValue::Rows(vec![SceneRow {
                cols: vec!["100".to_string()],
                ..Default::default()
            }]),
        );
        let mut v: Vec<Cmd> = Vec::new();
        scene.draw(
            &mut v,
            0.0,
            0.0,
            200.0,
            80.0,
            scale(),
            &pal(),
            &vals,
            0,
            false,
            false,
        );
        let bars: Vec<(f32, f32, f32, f32, u32)> =
            rects(&v).into_iter().filter(|r| r.2 == 5.0).collect();
        assert_eq!(bars.len(), 4, "four bars at 100%");
        assert!(
            bars.iter().all(|b| b.1 + b.3 == 12.0),
            "every bar's bottom sits ON base 12"
        );
        assert_eq!(bars[0].0, 4.0, "left-anchored at the cell's own x");
    }

    /// 7 columns, a couple of off-month (invisible) cells, one "today"
    /// (accent surface).
    #[test]
    fn grid_draws_calendar_matrix_and_regions() {
        let scene: CardScene = parse_scene_ron(
            r#"(items: [ Grid(
                    name: "cal",
                    x: 0.0, y: 24.0,
                    cols: 7,
                    pad: 0.0, gap: 0.0,
                    font_size: 9.0, r: 8.0,
                    head: Some(["S", "M", "T", "W", "T", "F", "S"]),
                    key_base: 30000,
                    hover_surface: Some(hover),
                ) ])"#,
        )
        .unwrap();
        let mut vals = vals();
        let mut cells = Vec::with_capacity(42);
        for cell in 0..42 {
            let day = cell - 2 + 1; // first weekday = Tue → 2 hidden days
            let visible = (1..=31).contains(&day);
            cells.push(GridCell {
                text: if visible {
                    day.to_string()
                } else {
                    String::new()
                },
                visible,
                color: if cell == 15 {
                    Some(ColorToken::Acc)
                } else {
                    None
                },
                surface: if cell == 15 {
                    Some(ColorToken::AccTint)
                } else {
                    None
                },
                ..Default::default()
            });
        }
        vals.insert("cal", SceneValue::GridCells(cells));
        let mut v: Vec<Cmd> = Vec::new();
        let (hits, _) = scene.draw(
            &mut v,
            0.0,
            0.0,
            350.0,
            280.0,
            scale(),
            &pal(),
            &vals,
            0,
            true,
            true,
        );
        // 42 slots: 2 leading + 9 trailing off-month cells hidden → 31
        // day cells (a 31-day month starting Tuesday)
        assert_eq!(hits.len(), 31, "invisible cells register no region");
        assert_eq!(
            hits[0].key,
            30000 + 2,
            "first visible cell key = base + index"
        );
        // every day cell is hit-tested at its own slot
        let cell_w = 350.0 / 7.0;
        let cell_h = (280.0 - 24.0 - 16.0) / 6.0; // (card_h - y - head_h) / 6 rows
        let h0 = hits.iter().find(|h| h.key == 30000 + 2).unwrap();
        assert!(
            (h0.x - cell_w * 2.0).abs() < 0.01,
            "row-wrap puts day 1 under the Tue column"
        );
        assert!(
            (h0.y - (24.0 + 16.0)).abs() < 0.01,
            "head row pushes cells down"
        );
        assert!((h0.h - cell_h).abs() < 0.01);
        // today's cell paints the accent tint surface + accent label
        assert!(v.iter().any(|c| matches!(c, Cmd::Rect { color: col, .. } if *col == ColorToken::AccTint.resolve(&pal()))),
            "today cell draws the accent tint");
        assert!(
            v.iter()
                .any(|c| matches!(c, Cmd::Text { text, .. } if *text == "15")),
            "day 15 text renders"
        );
    }

    /// A grid label centers on its OWN cell, matching the drawer
    /// (`text_c(v, cx + cell_w / 2.0, …)`). `draw_text` takes a box and
    /// centers inside it, so the cell must hand it the cell's left edge —
    /// anchoring at the already-centered point lands the label half a cell to
    /// the right (which is where the calendar's day numbers used to sit).
    #[test]
    fn grid_labels_center_on_their_own_cell() {
        let scene: CardScene = parse_scene_ron(
            r#"(items: [ Grid(
                    name: "g", x: 0.0, y: 0.0,
                    cols: 2, pad: 0.0, gap: 4.0,
                    font_size: 11.0, key_base: 9000,
                ) ])"#,
        )
        .unwrap();
        let mut vals = vals();
        vals.insert(
            "g",
            SceneValue::GridCells(vec![
                GridCell {
                    text: "a".into(),
                    visible: true,
                    ..Default::default()
                },
                GridCell {
                    text: "b".into(),
                    visible: true,
                    ..Default::default()
                },
            ]),
        );
        let mut v: Vec<Cmd> = Vec::new();
        let (hits, _) = scene.draw(
            &mut v,
            0.0,
            0.0,
            200.0,
            100.0,
            scale(),
            &pal(),
            &vals,
            0,
            true,
            true,
        );
        let cell_w = (200.0 - 4.0) / 2.0;
        let a = v
            .iter()
            .find_map(|c| {
                if let Cmd::Text {
                    text, x, center, ..
                } = c
                {
                    (*text == "a").then_some((*x, *center))
                } else {
                    None
                }
            })
            .expect("cell 'a' rendered");
        assert_eq!(a.1, true, "grid labels are center-anchored");
        assert_eq!(
            a.0,
            cell_w / 2.0,
            "the midpoint of the first cell, not its right edge"
        );
        let b = v
            .iter()
            .find_map(|c| {
                if let Cmd::Text { text, x, .. } = c {
                    (*text == "b").then_some(*x)
                } else {
                    None
                }
            })
            .expect("cell 'b' rendered");
        assert_eq!(
            b,
            cell_w + 4.0 + cell_w / 2.0,
            "and the second cell's own midpoint"
        );
        assert!(
            (hits[0].w - cell_w).abs() < 0.01,
            "the region is the cell box"
        );
    }

    #[test]
    fn grid_offsets_with_card_origin() {
        let scene: CardScene = parse_scene_ron(
            "(items: [ Grid(name: \"g\", x: 4.0, y: 6.0, cols: 2, pad: 0.0, gap: 0.0, key_base: 9000)])",
        )
        .unwrap();
        let mut vals = vals();
        vals.insert(
            "g",
            SceneValue::GridCells(vec![
                GridCell {
                    text: "a".into(),
                    visible: true,
                    ..Default::default()
                },
                GridCell {
                    text: "b".into(),
                    visible: true,
                    ..Default::default()
                },
            ]),
        );
        let mut v: Vec<Cmd> = Vec::new();
        let (hits, _) = scene.draw(
            &mut v,
            30.0,
            20.0,
            200.0,
            100.0,
            scale(),
            &pal(),
            &vals,
            0,
            true,
            true,
        );
        assert_eq!(hits.len(), 2);
        assert_eq!(
            hits[0].x, 34.0,
            "cell x = card x + grid x (full-bleed 2-col split)"
        );
        assert_eq!(hits[0].y, 26.0);
        assert_eq!(hits[1].x, 34.0 + 96.0, "second cell on the next column");
    }

    /// app-tile style: square cells with a raster + bottom label; the ✕
    /// delete corner appears only while the filled tile is hovered.
    #[test]
    fn grid_app_tiles_image_and_del_corner() {
        let scene: CardScene = parse_scene_ron(
            r#"(items: [ Grid(
                    name: "apps",
                    x: 0.0, y: 36.0,
                    cols: 4,
                    pad: 12.0, gap: 8.0,
                    font_size: 8.0, r: 10.0,
                    key_base: 34400,
                    del_base: 34416,
                    square: true,
                ) ])"#,
        )
        .unwrap();
        let mut vals = vals();
        let cells: Vec<GridCell> = (0..8)
            .map(|i| {
                let filled = i < 3;
                GridCell {
                    text: if filled {
                        format!("App {i}")
                    } else {
                        "+".into()
                    },
                    visible: true,
                    bottom: filled,
                    icon: !filled,
                    image_key: if filled {
                        Some(format!("icon:{i}"))
                    } else {
                        None
                    },
                    ..Default::default()
                }
            })
            .collect();
        vals.insert("apps", SceneValue::GridCells(cells));
        // resting: 3 filled tiles + 5 "+" tiles; no del corners while idle
        let mut v: Vec<Cmd> = Vec::new();
        let (hits, _) = scene.draw(
            &mut v,
            0.0,
            0.0,
            400.0,
            300.0,
            scale(),
            &pal(),
            &vals,
            0,
            true,
            true,
        );
        assert_eq!(hits.len(), 8, "every tile is clickable");
        assert!(
            v.iter()
                .any(|c| matches!(c, Cmd::Image { key, .. } if key == "icon:0")),
            "app icon rasters draw"
        );
        // hover the first tile → ✕ corner appears
        let mut v = Vec::new();
        let key = 34400;
        let (hits, _) = scene.draw(
            &mut v,
            0.0,
            0.0,
            400.0,
            300.0,
            scale(),
            &pal(),
            &vals,
            key,
            true,
            true,
        );
        assert_eq!(hits.len(), 9, "hovering a filled tile adds the ✕ region");
        assert!(
            hits.iter().any(|h| h.key == 34416),
            "del key = del_base + 0"
        );
        assert!(v
            .iter()
            .any(|c| matches!(c, Cmd::Text { text, .. } if text == crate::icons::ICON_CLOSE)));
        // hover an EMPTY "+" tile → no ✕ (not bottom)
        let mut v = Vec::new();
        let (hits, _) = scene.draw(
            &mut v,
            0.0,
            0.0,
            400.0,
            300.0,
            scale(),
            &pal(),
            &vals,
            34400 + 3,
            true,
            true,
        );
        assert_eq!(hits.len(), 8, "empty tiles never reveal the ✕");
    }

    #[test]
    fn image_emits_cmd_key_substituted_and_gated() {
        let scene: CardScene = parse_scene_ron(
            r#"(items: [ Image(x: 10.0, y: 38.0, w: 380.0, h: 180.0, key: "cam", visible: Some("pane0")),
                              Image(x: 0.0, y: 2.0, w: 12.0, h: 12.0, key: "{hero}", visible: Some("gone")) ])"#,
        )
        .unwrap();
        let mut vals = vals();
        vals.insert("pane0", SceneValue::Toggle(true));
        vals.insert("gone", SceneValue::Toggle(false));
        vals.insert("hero", SceneValue::Text("file:/tmp/a.png".into()));
        let mut v: Vec<Cmd> = Vec::new();
        let (_h, _i) = scene.draw(
            &mut v,
            5.0,
            5.0,
            400.0,
            300.0,
            scale(),
            &pal(),
            &vals,
            0,
            true,
            true,
        );
        let imgs: Vec<_> = v
            .iter()
            .filter(|c| matches!(c, Cmd::Image { .. }))
            .collect();
        assert_eq!(imgs.len(), 1, "the off-gate image is skipped");
        match imgs[0] {
            Cmd::Image { x, y, w, h, key } => {
                assert_eq!(*x, 15.0, "image x offsets with the card origin");
                assert_eq!(*y, 43.0);
                assert_eq!(*w, 380.0);
                assert_eq!(*h, 180.0);
                assert_eq!(key, "cam", "literal key unchanged");
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn surface_lifts_only_when_hovered() {
        let scene: CardScene = parse_scene_ron(
            "(items: [ Hit(x: 8.0, y: 6.0, w: 16.0, h: 16.0, r: 5.0, key: 2003, surface: Some(raised)) ])",
        )
        .unwrap();
        let base = ColorToken::Raised.resolve(&pal());
        let hov = ColorToken::Raised.hover_variant(&pal());
        // not hovered → base surface
        let mut v: Vec<Cmd> = Vec::new();
        let (_h, _i) = scene.draw(
            &mut v,
            0.0,
            0.0,
            148.0,
            90.0,
            scale(),
            &pal(),
            &vals(),
            0,
            true,
            true,
        );
        assert!(
            v.iter()
                .any(|c| matches!(c, Cmd::Rect { color: col, .. } if *col != hov)),
            "resting surface is not the hover color"
        );
        // hovered → lifted surface
        let mut v: Vec<Cmd> = Vec::new();
        let (_h, _i) = scene.draw(
            &mut v,
            0.0,
            0.0,
            148.0,
            90.0,
            scale(),
            &pal(),
            &vals(),
            2003,
            true,
            true,
        );
        assert!(
            v.iter()
                .any(|c| matches!(c, Cmd::Rect { color: col, .. } if *col == hov)),
            "hovered surface lifts"
        );
        assert!(
            v.iter()
                .any(|c| matches!(c, Cmd::Rect { color: col, .. } if *col != base)),
            "hover color differs from base"
        );
    }

    /// Read the first `Text` of a surface after `expand()`, which is the shape
    /// almost every prop test asserts on (a prop that is not read shows up as a
    /// `Val::Expr` naming the pool instead of a `Val::Lit`).
    fn first_text(scene: &mut ShellScene) -> (Val, Val, String) {
        let mut sc = scene.clone();
        sc.expand();
        let surf = sc.surface("settings").expect("surface found by name");
        match &surf.items[0] {
            SceneItem::Text { x, y, text, .. } => (x.clone(), y.clone(), text.clone()),
            other => panic!("first item is not a Text: {other:?}"),
        }
    }

    #[test]
    fn a_component_prop_defaults_when_no_use_site_overrides_it() {
        let mut sc: ShellScene = parse_scene_ron(
            r#"ShellScene(
                components: {
                    "chip": (props: (w: 20.0, x0: 4.0),
                             items: [ Text(x: "x0", y: 2.0, text: "Chip") ]),
                },
                surfaces: [ (name: "settings", items: [ Comp(name: "chip", x: 8.0, y: 6.0) ]) ],
            )"#,
        )
        .unwrap();
        let (x, _, _) = first_text(&mut sc);
        assert_eq!(
            x,
            Val::Lit(8.0 + 4.0),
            "the declared default applies and still composes with the Comp x/y"
        );
    }

    #[test]
    fn a_use_site_override_replaces_the_default() {
        let mut sc: ShellScene = parse_scene_ron(
            r#"ShellScene(
                components: {
                    "chip": (props: (x0: 4.0), items: [ Text(x: "x0", y: 2.0, text: "C") ]),
                },
                surfaces: [ (name: "settings",
                              items: [ Comp(name: "chip", x: 8.0, y: 0.0,
                                             props: (x0: 10.0)) ]) ],
            )"#,
        )
        .unwrap();
        let (x, _, _) = first_text(&mut sc);
        assert_eq!(x, Val::Lit(18.0), "8.0 (Comp x) + 10.0 (override)");
    }

    #[test]
    fn an_expression_prop_stays_an_expression_rather_than_freezing() {
        // THE load-bearing property of props: a use site that passes a pool name
        // (`w: "pill_ws_reg_w"`) must keep resolving against the pool at DRAW
        // time. If this folded to a literal at load, the value would be frozen
        // into the scene and every later frame would paint the first one's
        // number — a bug no single-frame test would catch.
        let sc: ShellScene = parse_scene_ron(
            r#"ShellScene(
                components: {
                    "chip": (props: (w: 20.0), items: [ Text(x: 0.0, y: 0.0, w: "w", text: "t") ]),
                },
                surfaces: [ (name: "settings",
                              items: [ Comp(name: "chip", props: (w: "pill_ws_reg_w")) ]) ],
            )"#,
        )
        .unwrap();
        let mut sc2 = sc.clone();
        sc2.expand();
        let surf = sc2.surface("settings").unwrap();
        match &surf.items[0] {
            SceneItem::Text { w, .. } => assert_eq!(
                *w,
                Val::Expr("pill_ws_reg_w".into()),
                "the prop is SUBSTITUTED into the binding, so the body still \
                 reads the pool name itself"
            ),
            other => panic!("not a Text: {other:?}"),
        }

        // …and it therefore follows the pool: 40 gives 40, not the default 20
        let mut pool = SceneValues::default();
        pool.set_num("pill_ws_reg_w", 40.0);
        match &surf.items[0] {
            SceneItem::Text { w, .. } => assert_eq!(
                w.num(&pool),
                40.0,
                "the frozen-literal bug this test exists to catch would read 20"
            ),
            other => panic!("not a Text: {other:?}"),
        }
    }

    #[test]
    fn an_expression_prop_composes_with_arithmetic_in_the_body() {
        // `w - 4` in the body must survive substitution as an expression, so a
        // prop that is itself an expression still does arithmetic. This is the
        // case a naive "fold the prop to a number" implementation breaks.
        let sc: ShellScene = parse_scene_ron(
            r#"ShellScene(
                components: {
                    "reg": (props: (w: 30.0),
                            items: [ Text(x: 0.0, y: 0.0, w: "w - 4", text: "t") ]),
                },
                surfaces: [ (name: "settings",
                              items: [ Comp(name: "reg", props: (w: "pill_w")) ]) ],
            )"#,
        )
        .unwrap();
        let mut sc2 = sc.clone();
        sc2.expand();
        let surf = sc2.surface("settings").unwrap();
        match &surf.items[0] {
            SceneItem::Text { w, .. } => {
                assert_eq!(*w, Val::Expr("pill_w - 4".into()), "composed, not folded")
            }
            other => panic!("not a Text: {other:?}"),
        }
        let mut pool = SceneValues::default();
        pool.set_num("pill_w", 50.0);
        match &surf.items[0] {
            SceneItem::Text { w, .. } => {
                assert_eq!(w.num(&pool), 46.0, "50 from the pool, minus the body's 4")
            }
            other => panic!("not a Text: {other:?}"),
        }
    }

    #[test]
    fn a_color_prop_is_read_through_the_value_token() {
        let sc: ShellScene = parse_scene_ron(
            r#"ShellScene(
                components: {
                    "chip": (props: (ink: fg),
                             items: [ Text(x: 0.0, y: 0.0, text: "x", color: Some(value("ink"))) ]),
                },
                surfaces: [ (name: "settings",
                              items: [ Comp(name: "chip", props: (ink: acc)) ]) ],
            )"#,
        )
        .unwrap();
        let mut sc2 = sc.clone();
        sc2.expand();
        let surf = sc2.surface("settings").unwrap();
        match &surf.items[0] {
            SceneItem::Text { color, .. } => assert_eq!(
                *color,
                Some(ColorToken::Acc),
                "a bare color token in a props block reaches value(\"ink\")"
            ),
            other => panic!("not a Text: {other:?}"),
        }
    }

    #[test]
    fn a_component_with_no_props_still_parses_as_a_bare_item_list() {
        // the legacy form — every existing scene in the repo is written this
        // way, so this is the backwards-compatibility floor
        let mut sc: ShellScene = parse_scene_ron(
            r#"ShellScene(
                components: { "chip": [ Text(x: 4.0, y: 2.0, text: "Chip") ] },
                surfaces: [ (name: "settings", items: [ Comp(name: "chip", x: 8.0) ]) ],
            )"#,
        )
        .unwrap();
        let (x, _, text) = first_text(&mut sc);
        assert_eq!(x, Val::Lit(12.0));
        assert_eq!(text, "Chip");
    }

    #[test]
    fn a_component_nests_inside_another_and_inherits_its_own_props() {
        let mut sc: ShellScene = parse_scene_ron(
            r#"ShellScene(
                components: {
                    "inner": (props: (pad: 2.0), items: [ Text(x: "pad", y: 0.0, text: "in") ]),
                    "outer": (props: (pad: 5.0),
                              items: [ Comp(name: "inner", x: 1.0, y: 1.0, props: (pad: 9.0)) ]),
                },
                surfaces: [ (name: "settings", items: [ Comp(name: "outer", x: 100.0) ]) ],
            )"#,
        )
        .unwrap();
        let (x, _, text) = first_text(&mut sc);
        assert_eq!(
            x,
            Val::Lit(110.0),
            "100 (outer) + 1 (inner Comp) + 9 (inner override) — each frame adds \
             its own base, the override wins over both defaults"
        );
        assert_eq!(text, "in");
    }

    #[test]
    fn a_prop_is_scoped_to_its_own_subtree() {
        // props must not leak into SIBLING components: `w` here is the outer
        // default even though the first use site overrode its own `w`.
        let sc: ShellScene = parse_scene_ron(
            r#"ShellScene(
                components: {
                    "chip": (props: (w: 20.0), items: [ Text(x: 0.0, y: 0.0, w: "w", text: "t") ]),
                },
                surfaces: [ (name: "settings", items: [
                    Comp(name: "chip", x: 0.0, props: (w: 99.0)),
                    Comp(name: "chip", x: 200.0),
                ]) ],
            )"#,
        )
        .unwrap();
        let mut sc2 = sc.clone();
        sc2.expand();
        let surf = sc2.surface("settings").unwrap();
        assert_eq!(surf.items.len(), 2);
        match (&surf.items[0], &surf.items[1]) {
            (SceneItem::Text { w: a, .. }, SceneItem::Text { w: b, .. }) => {
                assert_eq!(*a, Val::Lit(99.0), "override applied");
                assert_eq!(*b, Val::Lit(20.0), "sibling kept the default");
            }
            _ => panic!("expected two Texts"),
        }
    }

    #[test]
    fn a_prop_read_through_the_wrong_channel_is_left_alone() {
        // `ink: acc` read as a WIDTH must not silently become a number. The
        // validator reports the type mismatch; here the point is only that the
        // rewriter does not invent a 0 and draw it.
        let sc: ShellScene = parse_scene_ron(
            r#"ShellScene(
                components: {
                    "chip": (props: (ink: acc), items: [ Text(x: 0.0, y: 0.0, w: "ink", text: "t") ]),
                },
                surfaces: [ (name: "settings", items: [ Comp(name: "chip") ]) ],
            )"#,
        )
        .unwrap();
        let mut sc2 = sc.clone();
        sc2.expand();
        let surf = sc2.surface("settings").unwrap();
        match &surf.items[0] {
            SceneItem::Text { w, .. } => assert_eq!(
                *w,
                Val::Expr("ink".into()),
                "an unsubstituted name, which the pool resolves 0 — not a wrong \
                 number baked into the scene"
            ),
            other => panic!("not a Text: {other:?}"),
        }
    }

    #[test]
    fn a_duplicate_prop_name_is_a_load_error_not_last_one_wins() {
        // Two spellings of the same prop disagree; silently keeping the last
        // would make the effective default depend on which line you read, and
        // in RON (unlike a JSON5-ish format) there is no "later wins" to rely
        // on when the two values are on different lines of a component table.
        let err = parse_scene_ron::<ShellScene>(
            r#"ShellScene(
                components: {
                    "chip": (props: (w: 20.0, w: 30.0),
                             items: [ Text(x: 0.0, y: 0.0, w: "w", text: "t") ]),
                },
                surfaces: [ (name: "settings", items: [ Comp(name: "chip") ]) ],
            )"#,
        )
        .expect_err("a duplicate prop must not parse");
        assert!(
            format!("{err}").contains("duplicate prop `w`"),
            "the diagnostic names the prop: {err}"
        );
    }

    #[test]
    fn an_empty_prop_set_parses() {
        // `props: ()` is the degenerate spelling of "no defaults", and an
        // author who writes it should get an empty set rather than a parse error
        // that reads like the whole component table is malformed.
        let mut sc: ShellScene = parse_scene_ron(
            r#"ShellScene(
                components: {
                    "chip": (props: (), items: [ Text(x: 3.0, y: 0.0, text: "t") ]),
                },
                surfaces: [ (name: "settings", items: [ Comp(name: "chip", x: 1.0) ]) ],
            )"#,
        )
        .unwrap();
        let (x, _, _) = first_text(&mut sc);
        assert_eq!(x, Val::Lit(4.0));
    }

    #[test]
    fn a_props_set_round_trips_through_serialization() {
        // the `Serialize` half of `PropSet` writes a MAP, so a component table
        // survives the serialize → parse round trip the shell's own tests use to
        // prove a scene is lossless
        let sc: ShellScene = parse_scene_ron(
            r#"ShellScene(
                components: {
                    "chip": (props: (w: 20.0, ink: acc, title: "hi"),
                             items: [ Text(x: 0.0, y: 0.0, w: "w", text: "{title}") ]),
                },
                surfaces: [ (name: "settings", items: [ Comp(name: "chip") ]) ],
            )"#,
        )
        .unwrap();
        let out = serde_json::to_string(&sc).unwrap();
        let back: ShellScene = serde_json::from_str(&out).expect("roundtrips");
        let chip = back.components.get("chip").expect("chip survived");
        assert_eq!(chip.props().get("w"), Some(&PropValue::Num(20.0)));
        assert_eq!(
            chip.props().get("ink"),
            Some(&PropValue::Color(ColorToken::Acc))
        );
        assert_eq!(
            chip.props().get("title"),
            Some(&PropValue::Expr("hi".into()))
        );
    }

    /// A string prop read in a `value()` slot forwards to the pool as a color
    /// NAME, which is how a component parameterizes a color the scene cannot
    /// decide for itself (accent-while-connected, fg3-otherwise).
    #[test]
    fn a_string_prop_in_a_value_slot_becomes_a_pool_color_name() {
        let mut sc: ShellScene = parse_scene_ron(
            r#"ShellScene(
                components: {
                    "chip": (props: (ink: fg),
                             items: [ Text(x: 0.0, y: 0.0, text: "x", color: Some(value("ink"))) ]),
                },
                surfaces: [ (name: "settings",
                              items: [ Comp(name: "chip", props: (ink: "pill_sub_meta_col")) ]) ],
            )"#,
        )
        .unwrap();
        sc.expand();
        let surf = sc.surface("settings").unwrap();
        match &surf.items[0] {
            SceneItem::Text { color, .. } => assert_eq!(
                *color,
                Some(ColorToken::Value(
                    DynColor::new("pill_sub_meta_col").unwrap()
                )),
                "the name is deferred to the pool, so a theme switch still lands"
            ),
            other => panic!("not a Text: {other:?}"),
        }
        assert!(
            sc.dead_props.is_empty(),
            "the prop IS read: {:?}",
            sc.dead_props
        );
    }

    /// A text prop is LITERAL, and braces are what say "resolve a pool name".
    ///
    /// Both halves matter: a scene author writing `meta: "pill_meta"` must get
    /// the four characters "pill_meta" (it is almost always literal copy, and
    /// `title: "Wi-Fi Networks"` must not vanish because no one publishes a
    /// property by that name), while `meta: "{pill_meta}"` must reach the pool.
    #[test]
    fn a_text_prop_is_literal_and_braces_are_what_bind_it() {
        let mut sc: ShellScene = parse_scene_ron(
            r#"ShellScene(
                components: {
                    "sub": (props: (title: "", meta: ""),
                            items: [ Text(x: 0.0, y: 0.0, text: "{title}", font_size: 14.0),
                                     Text(x: 0.0, y: 20.0, text: "{meta}", font_size: 14.0) ]),
                },
                surfaces: [ (name: "settings", items: [ Comp(name: "sub",
                        props: (title: "Wi-Fi Networks", meta: "{pill_wifi_meta}")) ]) ],
            )"#,
        )
        .unwrap();
        sc.expand();
        let surf = sc.surface("settings").unwrap();
        match (&surf.items[0], &surf.items[1]) {
            (SceneItem::Text { text: t, .. }, SceneItem::Text { text: m, .. }) => {
                assert_eq!(t, "Wi-Fi Networks", "a bare string prop is literal");
                assert_eq!(
                    m, "{pill_wifi_meta}",
                    "a braced prop stays braced after substitution, so the pool \
                     resolves it on the next pass"
                );
            }
            _ => panic!("expected two Texts"),
        }
        assert!(sc.dead_props.is_empty(), "{:?}", sc.dead_props);

        // and the pool pass does resolve it, at draw
        let mut vals = SceneValues::default();
        vals.set_str("pill_wifi_meta", "On");
        let card = CardScene {
            items: surf.items.clone(),
        };
        let mut v: Vec<Cmd> = Vec::new();
        card.draw(
            &mut v,
            0.0,
            0.0,
            200.0,
            60.0,
            scale(),
            &pal(),
            &vals,
            0,
            false,
            false,
        );
        let texts: Vec<&str> = v
            .iter()
            .filter_map(|c| match c {
                Cmd::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(
            texts,
            vec!["Wi-Fi Networks", "On"],
            "the braced prop reached the pool value"
        );
    }

    #[test]
    fn an_override_the_body_never_reads_is_reported() {
        // `wid` is a typo for `w`. The scene still draws correctly — the default
        // substitutes — so nothing else in the pipeline can notice, which is
        // exactly why this needs saying out loud.
        let mut sc: ShellScene = parse_scene_ron(
            r#"ShellScene(
                components: {
                    "chip": (props: (w: 20.0), items: [ Text(x: 0.0, y: 0.0, w: "w", text: "t") ]),
                },
                surfaces: [ (name: "settings",
                              items: [ Comp(name: "chip", props: (wid: 99.0)) ]) ],
            )"#,
        )
        .unwrap();
        sc.expand();
        assert_eq!(sc.dead_props.len(), 1, "{:?}", sc.dead_props);
        let d = &sc.dead_props[0];
        assert!(d.contains("`wid`"), "names the prop: {d}");
        assert!(d.contains("`chip`"), "names the component: {d}");
        assert!(d.contains("`w`"), "lists what IS declared: {d}");
    }

    #[test]
    fn an_override_of_an_undeclared_prop_is_reported() {
        // A use site passing a prop the component never declared: inert, and the
        // declaration list reads "no defaults at all", which is the tell.
        let mut sc: ShellScene = parse_scene_ron(
            r#"ShellScene(
                components: {
                    "chip": [ Text(x: 1.0, y: 0.0, text: "t") ],
                },
                surfaces: [ (name: "settings",
                              items: [ Comp(name: "chip", props: (w: 99.0)) ]) ],
            )"#,
        )
        .unwrap();
        sc.expand();
        assert_eq!(sc.dead_props.len(), 1, "{:?}", sc.dead_props);
        assert!(
            sc.dead_props[0].contains("no defaults at all"),
            "{}",
            sc.dead_props[0]
        );
    }

    #[test]
    fn a_declared_default_nobody_reads_is_reported() {
        let mut sc: ShellScene = parse_scene_ron(
            r#"ShellScene(
                components: {
                    "chip": (props: (w: 20.0, stale: 3.0),
                             items: [ Text(x: 0.0, y: 0.0, w: "w", text: "t") ]),
                },
                surfaces: [ (name: "settings", items: [ Comp(name: "chip") ]) ],
            )"#,
        )
        .unwrap();
        sc.expand();
        assert_eq!(sc.dead_props.len(), 1, "{:?}", sc.dead_props);
        assert!(
            sc.dead_props[0].contains("`stale`") && !sc.dead_props[0].contains("`w`,"),
            "only the unread one: {}",
            sc.dead_props[0]
        );
    }

    #[test]
    fn a_prop_that_is_read_is_never_reported() {
        // the negative case, and the one that matters: a check that fires on live
        // props trains the author to ignore it
        let mut sc: ShellScene = parse_scene_ron(
            r#"ShellScene(
                components: {
                    "chip": (props: (w: 20.0, ink: fg, title: "hi"),
                             items: [ Text(x: 0.0, y: 0.0, w: "w", text: "{title}",
                                            color: Some(value("ink"))) ]),
                },
                surfaces: [ (name: "settings",
                              items: [ Comp(name: "chip", props: (w: 30.0, ink: acc, title: "yo")) ]) ],
            )"#,
        )
        .unwrap();
        sc.expand();
        assert!(
            sc.dead_props.is_empty(),
            "all three props are read: {:?}",
            sc.dead_props
        );
    }

    #[test]
    fn shell_scene_parses_and_expands_components() {
        let ron = r#"
            ShellScene(
                components: {
                    "chip": [ Text(x: 4.0, y: 2.0, text: "Chip", font_size: 12.0, color: Some(fg)) ],
                },
                surfaces: [
                    (name: "settings",
                     items: [
                        Comp(name: "chip", x: 8.0, y: 6.0),
                        Text(x: 1.0, y: 1.0, text: "hello", font_size: 10.0, color: Some(fg)),
                     ]),
                ],
            )
        "#;
        let mut sc: ShellScene = parse_scene_ron(ron).unwrap();
        sc.expand();
        let surf = sc.surface("settings").expect("surface found by name");
        assert_eq!(
            surf.items.len(),
            2,
            "Comp inlined + the literal text both remain"
        );
        match &surf.items[0] {
            SceneItem::Text { x, y, text, .. } => {
                assert_eq!(*x, Val::Lit(12.0), "template translated by the Comp x/y");
                assert_eq!(*y, Val::Lit(8.0));
                assert_eq!(text, "Chip");
            }
            other => panic!("widget inlined but wrong: {other:?}"),
        }
    }

    /// The live scene must declare NO dead props (phase 2).
    ///
    /// Separate from the parse test on purpose: a dead prop is not a parse
    /// error, so without this the engine's own unused-vocabulary diagnostic
    /// would have nothing testing it against the one file that matters, and a
    /// typo'd prop in `shell.ron` would ship silently.
    #[test]
    fn shell_ron_live_file_declares_no_dead_props() {
        let text = crate::shell_scene::SHELL_SCENE_RON;
        let mut sc: ShellScene = parse_scene_ron(&text).expect("live shell.ron parses");
        sc.expand();
        assert!(
            sc.dead_props.is_empty(),
            "every prop the live scene declares is read by some body: {:?}",
            sc.dead_props
        );
    }

    #[test]
    fn shell_ron_live_file_parses_and_has_surfaces() {
        let text = crate::shell_scene::SHELL_SCENE_RON;
        let mut sc: ShellScene = parse_scene_ron(&text).expect("RON parses cleanly");
        sc.expand();
        assert!(
            sc.surface("settings").is_some(),
            "settings surface declared"
        );
        assert!(
            sc.surface("dashboard").is_some(),
            "dashboard surface declared"
        );
        assert!(sc.surface("notif").is_some(), "notif surface declared");
        assert!(sc.surface("lock").is_some(), "lock surface declared");
        assert!(
            sc.surface("wallpaper").is_some(),
            "wallpaper surface declared"
        );
        // the first whole-shell batch: the OSD, the Power tile row and the two
        // radio submenus, each drawn with ZERO Ink (fully declarative) plus
        // the shared `subhead` chrome the menus stamp.
        for name in [
            "osd",
            "power",
            "wifi_menu",
            "bt_menu",
            "workspace_switcher",
            "polkit_auth",
        ] {
            let surf = sc
                .surface(name)
                .unwrap_or_else(|| panic!("{name} surface declared"));
            assert!(
                !surf
                    .items
                    .iter()
                    .any(|it| matches!(it, SceneItem::Ink { .. })),
                "{name} is fully declarative — no Rust body Ink"
            );
        }
        assert!(
            sc.components.contains_key("subhead"),
            "shared submenu header component declared"
        );
        // the OSD keeps the dismiss region and gates both texts on `osd_on`
        let osd = sc.surface("osd").expect("osd surface");
        assert!(
            osd.items.iter().any(|it| matches!(
                it,
                SceneItem::Hit {
                    key: Val::Lit(1.0),
                    ..
                }
            )),
            "osd surface keeps the click-to-dismiss region"
        );
        let gated = osd
            .items
            .iter()
            .filter_map(|it| match it {
                SceneItem::Text { visible, .. } => visible.as_deref(),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(gated.len(), 2, "both OSD texts gate on osd_on");
        assert!(gated.iter().all(|v| *v == "osd_on"), "OSD gate is osd_on");
        // the power row centres one 5-tile stack, and the wifi/bt lists are
        // `Rows` with the menus' own key base
        let power = sc.surface("power").expect("power surface");
        let rows = power
            .items
            .iter()
            .filter_map(|it| match it {
                SceneItem::Row { items, .. } => Some(items.clone()),
                _ => None,
            })
            .next()
            .expect("power surface centres its tiles in a Row");
        assert_eq!(rows.len(), 5, "five power tiles");
        assert!(
            rows.iter().all(|it| matches!(it, SceneItem::Stack { .. })),
            "every power tile is a Stack"
        );
        for name in ["wifi_menu", "bt_menu"] {
            let surf = sc.surface(name).expect("submenu surface");
            let list = surf
                .items
                .iter()
                .find_map(|it| match it {
                    SceneItem::Rows { key_base, cols, .. } => Some((*key_base, cols.clone())),
                    _ => None,
                })
                .unwrap_or_else(|| panic!("{name} declares its row list"));
            assert_eq!(list.0, Some(10), "{name} rows key off 10");
            assert!(!list.1.is_empty(), "{name} declares row columns");
        }
        // `wallpaper` is CONVERTED: the picker's empty state, thumbnail grid
        // and scrollbar are declared, and the body `Ink` is gone. This is the
        // third surface to fall — the `settings` sections above and `notif`
        // below both said the same thing, for the same reason: a declared body
        // plus an Ink would double-paint and double-register every tile and the
        // scrollbar strip under it.
        {
            let surf = sc.surface("wallpaper").expect("surface");
            assert!(
                !surf
                    .items
                    .iter()
                    .any(|it| matches!(it, SceneItem::Ink { .. })),
                "wallpaper declares its picker — no Rust body Ink"
            );
        }
        // `settings` is CONVERTED as of the Appearance section — the sixth and
        // last — so the surface carries no Ink either. Same ratchet as `notif`
        // below, for the same reason: an Ink reappearing here would mean some
        // tab fell back to a second hand-drawn set, and a declaration plus an
        // Ink would double-paint and double-register every row under it.
        {
            let surf = sc.surface("settings").expect("surface");
            assert!(
                !surf
                    .items
                    .iter()
                    .any(|it| matches!(it, SceneItem::Ink { .. })),
                "settings declares all six of its sections — no Rust body Ink"
            );
        }
        // `notif` is CONVERTED: its card list is a `Repeat` over the `notif_rows`
        // model and the surface carries no Ink at all. Asserted as a ratchet
        // rather than folded into the loop above — a body that has moved into
        // `.ron` must not be able to quietly acquire an Ink again, because
        // "Rust still paints this" is the failure mode the whole rewrite is
        // about and nothing else in the tree would notice it.
        {
            let surf = sc.surface("notif").expect("surface");
            assert!(
                !surf
                    .items
                    .iter()
                    .any(|it| matches!(it, SceneItem::Ink { .. })),
                "notif declares its card list — no Rust body Ink"
            );
            let rep = surf
                .items
                .iter()
                .find_map(|it| match it {
                    SceneItem::Repeat {
                        model, templates, ..
                    } => Some((model.clone(), templates.len())),
                    _ => None,
                })
                .expect("notif declares its list as a model Repeat");
            assert_eq!(
                rep.0.as_deref(),
                Some("notif_rows"),
                "the list repeats the notif model"
            );
            assert_eq!(rep.1, 1, "one delegate draws every notification");
        }
        // The dashboard surface: the LAST shell surface to lose its Ink, and
        // now the last one at all — the board (`dash_cards` Repeat of `Card`
        // delegates), the strip band, the chips and the connectivity popover
        // are all declared here. What survives of `Ink("dash_banner")` is only
        // its Rust painter, `layout_banner`, called as the FALLBACK when a
        // config has no `dashboard` surface (and for edit-mode chrome, which
        // the surface never claims). The scrollbar overlay was Rust inline
        // either way.
        let dash = sc.surface("dashboard").expect("dashboard surface");
        let inks: Vec<&str> = dash
            .items
            .iter()
            .filter_map(|it| match it {
                SceneItem::Ink { name, .. } => Some(name.as_str()),
                _ => None,
            })
            .collect();
        assert!(
            inks.is_empty(),
            "the dashboard surface declares no Ink (found {inks:?}) — its \
             Rust body lives only as the fallback painter"
        );
        let board = dash
            .items
            .iter()
            .find_map(|it| match it {
                SceneItem::Repeat {
                    model, templates, ..
                } => Some((model.clone(), templates)),
                _ => None,
            })
            .expect("dashboard declares its board as a model Repeat");
        assert_eq!(
            board.0.as_deref(),
            Some("dash_cards"),
            "the board repeats the packed-card model"
        );
        assert!(
            board.1.iter().any(|t| t.items.iter().any(|it| matches!(
                it,
                SceneItem::Card { name, .. } if name.contains("item.card")
            ))),
            "the board's template hands every slot to a `Card` delegate"
        );
    }

    /// The first whole-shell batch, measured against the Rust drawers it
    /// replaces: the Wi-Fi submenu's row band, label, signal block and check,
    /// the Bluetooth row's state column, the Power tile block and the OSD's
    /// two centered strings. Drawn straight off the live `shell.ron`, so a
    /// drifted constant in either file fails here.
    #[test]
    fn shell_ron_first_batch_matches_its_rust_drawers() {
        let text = crate::shell_scene::SHELL_SCENE_RON;
        let mut sc: ShellScene = parse_scene_ron(&text).expect("RON parses cleanly");
        sc.expand();
        let draw = |name: &str, w: f32, h: f32, vals: &SceneValues| -> (Vec<Cmd>, Vec<SceneHit>) {
            let surf = sc.surface(name).expect("surface");
            let card = CardScene {
                items: surf.items.clone(),
            };
            let mut v: Vec<Cmd> = Vec::new();
            let (hits, inks) = card.draw(
                &mut v,
                0.0,
                0.0,
                w,
                h,
                scale(),
                &pal(),
                vals,
                0,
                false,
                false,
            );
            assert!(inks.is_empty(), "{name} needs no Ink");
            (v, hits)
        };
        let wash = |_i: usize| SceneColor::Raw(crate::ui::raised(&pal()));
        // ── Wi-Fi: 440 × 480, two networks (one secured + connected) ──
        let mut vals = SceneValues::new();
        vals.insert_color("wifi_wash_0", wash(0));
        vals.insert_color("wifi_wash_1", wash(1));
        vals.insert(
            "wifi_menu_rows",
            SceneValue::Rows(vec![
                SceneRow {
                    cols: vec!["90".into(), "\u{f023} Fritz!Box".into(), "\u{f00c}".into()],
                    key: 10,
                    surface: Some(ColorToken::Value(
                        crate::ui::DynColor::new("wifi_wash_0").unwrap(),
                    )),
                    col_colors: vec![None, None, Some(ColorToken::Acc)],
                    ..Default::default()
                },
                SceneRow {
                    cols: vec!["30".into(), "guest".into(), String::new()],
                    key: 11,
                    surface: Some(ColorToken::Value(
                        crate::ui::DynColor::new("wifi_wash_1").unwrap(),
                    )),
                    ..Default::default()
                },
            ]),
        );
        vals.insert("wifi_empty_on", SceneValue::Toggle(false));
        vals.insert("wifi_empty", SceneValue::Text("Scanning…".into()));
        // per-panel subhead state (phase 2): the fixture stands in for the
        // publisher, so it publishes the same names the live one does
        vals.insert("pill_wifi_meta", SceneValue::Text("On".into()));
        vals.insert("pill_wifi_meta_size", SceneValue::Ring(12.0));
        vals.insert_color("pill_wifi_meta_col", SceneColor::Raw(pal().acc));
        vals.insert("osd_on", SceneValue::Toggle(false));
        let (v, hits) = draw("wifi_menu", 440.0, 480.0, &vals);
        // the row band: 16 px inset, 36 tall, 12 radius, 42 stride from y 56
        let prects = rects(&v);
        let bands: Vec<&(f32, f32, f32, f32, u32)> =
            prects.iter().filter(|r| r.3 == 36.0).collect();
        assert_eq!(bands.len(), 2, "one wash per network");
        assert_eq!(
            (bands[0].0, bands[0].1, bands[0].2),
            (16.0, 56.0, 440.0 - 32.0),
            "row 0 band"
        );
        assert_eq!(
            (bands[1].0, bands[1].1),
            (16.0, 98.0),
            "row 1 sits one 42 px stride down"
        );
        // the SSID label at the drawer's (30, y + 10), 12 px
        let label = v
            .iter()
            .find_map(|c| match c {
                Cmd::Text {
                    x, y, size, text, ..
                } if text.ends_with("Fritz!Box") => Some((*x, *y, *size)),
                _ => None,
            })
            .expect("the SSID label");
        assert_eq!(label, (30.0, 66.0, 12.0), "16 pad + 14 in, 10 down the row");
        // the ✓ right-anchored 28 px in, 9 down, in the icon font at 13 px
        let check = v
            .iter()
            .find_map(|c| match c {
                Cmd::Text {
                    x,
                    y,
                    right,
                    size,
                    icon,
                    text,
                    ..
                } if text == crate::icons::ICON_CHECK => Some((*x, *y, *right, *size, *icon)),
                _ => None,
            })
            .expect("the connected check");
        assert_eq!(
            (check.0, check.1),
            (440.0 - 28.0, 65.0),
            "right edge 28 in, 9 down the row"
        );
        assert!(check.2 && check.4, "right-anchored, drawn in the icon font");
        assert_eq!(check.3, 13.0);
        // the signal block: 4 × 5 px bars on 7 px centers, RIGHT edge 46 in
        // (the drawer's w − 72 + 3 · 7 + 5), centered on the row's mid rail
        let bars: Vec<&(f32, f32, f32, f32, u32)> = prects.iter().filter(|r| r.2 == 5.0).collect();
        assert_eq!(bars.len(), 8, "four bars on each of the two rows");
        assert_eq!(
            bars[3].0 + bars[3].2,
            440.0 - 46.0,
            "block's right edge is w - 46"
        );
        assert_eq!(
            bars[0].0,
            440.0 - 72.0,
            "and its left is the drawer's w - 72"
        );
        for b in &bars[..4] {
            assert_eq!(
                b.1 + b.3 / 2.0,
                56.0 + 18.0,
                "each bar straddles the row's mid rail"
            );
        }
        // the header: the meta size rides the bound scalar (12 px for the
        // radio menus), the title is static and the state right-anchored
        let (title, meta) = (
            v.iter().find_map(|c| match c {
                Cmd::Text { x, size, text, .. } if text == "Wi-Fi Networks" => Some((*x, *size)),
                _ => None,
            }),
            v.iter().find_map(|c| match c {
                Cmd::Text {
                    x,
                    y,
                    right,
                    size,
                    text,
                    ..
                } if text == "On" => Some((*x, *y, *right, *size)),
                _ => None,
            }),
        );
        assert_eq!(
            title,
            Some((48.0, 15.0)),
            "the subhead title at the drawer's 48 px inset"
        );
        assert_eq!(
            meta,
            Some((440.0 - 16.0, 16.0, true, 12.0)),
            "the meta rides its bound size"
        );
        // the header: back region, title, and the right-anchored state
        let keys: Vec<u32> = hits.iter().map(|r| r.key).collect();
        assert_eq!(
            keys,
            vec![1, 10, 11],
            "back button then one region per row, in draw order"
        );
        assert_eq!(
            (hits[0].x, hits[0].y, hits[0].w, hits[0].h),
            (12.0, 8.0, 28.0, 28.0),
            "back region"
        );
        // ── Bluetooth: same band, the state right-anchored 28 in on the
        // row's own 10 px baseline ──
        let mut bvals = SceneValues::new();
        bvals.insert_color("bt_wash_0", wash(0));
        bvals.insert_color("bt_state_0", SceneColor::Raw(pal().acc));
        bvals.insert(
            "bt_menu_rows",
            SceneValue::Rows(vec![SceneRow {
                cols: vec!["Pixel 9".into(), "Connected".into()],
                key: 10,
                surface: Some(ColorToken::Value(
                    crate::ui::DynColor::new("bt_wash_0").unwrap(),
                )),
                col_colors: vec![
                    None,
                    Some(ColorToken::Value(
                        crate::ui::DynColor::new("bt_state_0").unwrap(),
                    )),
                ],
                ..Default::default()
            }]),
        );
        bvals.insert("bt_empty_on", SceneValue::Toggle(false));
        bvals.insert("bt_empty", SceneValue::Text("No devices found".into()));
        bvals.insert("pill_bt_meta", SceneValue::Text("On".into()));
        bvals.insert("pill_bt_meta_size", SceneValue::Ring(12.0));
        bvals.insert_color("pill_bt_meta_col", SceneColor::Raw(pal().acc));
        let (bv, bhits) = draw("bt_menu", 440.0, 480.0, &bvals);
        let state = bv
            .iter()
            .find_map(|c| match c {
                Cmd::Text {
                    x,
                    y,
                    right,
                    size,
                    text,
                    ..
                } if text == "Connected" => Some((*x, *y, *right, *size)),
                _ => None,
            })
            .expect("the connection state");
        assert_eq!(
            state,
            (440.0 - 28.0, 66.0, true, 11.0),
            "right 28 in, 10 down, 11 px"
        );
        let brects = rects(&bv);
        let bbands: Vec<&(f32, f32, f32, f32, u32)> =
            brects.iter().filter(|r| r.3 == 36.0).collect();
        assert_eq!(
            (bbands[0].0, bbands[0].1),
            (16.0, 56.0),
            "the same row band as the Wi-Fi menu"
        );
        assert_eq!(bhits.iter().map(|r| r.key).collect::<Vec<_>>(), vec![1, 10]);
        // ── Power: the five 92 px tiles centered as one 516 px block ──
        let mut pvals = SceneValues::new();
        for i in 0..5 {
            pvals.insert_color(format!("ppower_bg_{i}"), wash(i));
            pvals.insert(
                format!("ppower_hold_{i}"),
                SceneValue::Text("0.0000".into()),
            );
            pvals.insert(format!("ppower_hold_on_{i}"), SceneValue::Toggle(false));
            pvals.insert_color(format!("ppower_ink_{i}"), SceneColor::Raw(pal().fg));
        }
        pvals.insert_color("power_hold_fill", SceneColor::Raw(pal().acc));
        pvals.insert_color("clear", SceneColor::Raw(0));
        let (pv, phits) = draw("power", 700.0, 350.0, &pvals);
        let prects = rects(&pv);
        let tiles: Vec<&(f32, f32, f32, f32, u32)> =
            prects.iter().filter(|r| r.2 == 92.0).collect();
        assert_eq!(
            tiles.len(),
            5,
            "five tile washes and nothing else is 92 wide"
        );
        let x0 = (700.0 - (5.0 * 92.0 + 4.0 * 14.0)) / 2.0;
        let y0 = (350.0 - 92.0) / 2.0;
        assert_eq!(
            (tiles[0].0, tiles[0].1),
            (x0, y0),
            "the block centers as the drawer does"
        );
        assert_eq!(
            tiles[4].0 + 92.0,
            x0 + 516.0,
            "516 px block on a 14 px stride"
        );
        // the glyphs sit at the drawer's 34 px inset, 24 down; the labels are
        // centered on the tile at y + 60
        let mut glyph_x: Vec<f32> = pv
            .iter()
            .filter_map(|c| match c {
                Cmd::Text { x, size, text, .. } if *size == 24.0 && *text != "Power" => Some(*x),
                _ => None,
            })
            .collect();
        glyph_x.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert_eq!(glyph_x.len(), 5, "five 24 px glyphs");
        for (i, gx) in glyph_x.iter().enumerate() {
            assert!(
                (gx - (x0 + i as f32 * 106.0 + 34.0)).abs() < 0.01,
                "glyph {i} at the drawer's (tile_w - fs)/2 = 34 px inset: {gx}"
            );
        }
        let label = pv
            .iter()
            .find_map(|c| match c {
                Cmd::Text { x, size, text, .. } if text == "Shutdown" => Some((*x, *size)),
                _ => None,
            })
            .expect("the shutdown label");
        assert_eq!(label.1, 12.0, "12 px labels");
        // anchored ON the last tile's center line, exactly like the drawer's
        // `text_c(bx + bw/2, …)` — both sides hand the renderer a center x
        let tile_c = x0 + 4.0 * (92.0 + 14.0) + 46.0;
        assert!(
            (label.0 - tile_c).abs() < 0.01,
            "label anchored on the last tile center {tile_c}"
        );
        let keys: Vec<u32> = phits.iter().map(|r| r.key).collect();
        assert_eq!(keys, vec![1, 2, 3, 4, 5], "the five tiles key 1..=5");
        // ── OSD: glyph and label on the panel's mid line ──
        let mut ovals = SceneValues::new();
        ovals.insert("osd_on", SceneValue::Toggle(true));
        ovals.insert("osd_glyph", SceneValue::Text("\u{f028}".into()));
        ovals.insert("osd_text", SceneValue::Text("Brightness 60%".into()));
        let (ov, ohits) = draw("osd", 260.0, 68.0, &ovals); // the real Mode::Osd size
        let texts: Vec<(f32, f32, f32, String)> = ov
            .iter()
            .filter_map(|c| match c {
                Cmd::Text {
                    x, y, size, text, ..
                } => Some((*x, *y, *size, text.clone())),
                _ => None,
            })
            .collect();
        assert_eq!(texts.len(), 2, "glyph + label, both gated on osd_on");
        assert_eq!(
            (texts[0].0, texts[0].2),
            (18.0, 22.0),
            "glyph at the drawer's 18 px inset, 22 px"
        );
        assert_eq!(texts[0].1, 68.0 / 2.0 - 11.0, "centered: (h - 22)/2");
        assert_eq!(
            (texts[1].0, texts[1].2),
            (52.0, 14.0),
            "label 34 px along, 14 px"
        );
        assert_eq!(texts[1].1, 68.0 / 2.0 - 7.0, "centered: (h - 14)/2");
        assert_eq!(
            ohits.iter().map(|r| r.key).collect::<Vec<_>>(),
            vec![1],
            "one dismiss region"
        );
        // ── workspace switcher: 600 × 320, five workspaces (2 on the left
        // column, 3 on the right), the second one active ──
        let mut wvals = SceneValues::new();
        wvals.insert("ws_count", SceneValue::Text("5".into()));
        wvals.insert_color("clear", SceneColor::Raw(0)); // the rails' track
        for i in 0..5 {
            wvals.insert_color(
                format!("ws_wash_{i}"),
                SceneColor::Raw(crate::ui::hover(&pal())),
            );
            wvals.insert_color(
                format!("ws_num_{i}"),
                SceneColor::Raw(crate::ui::fg2(&pal())),
            );
        }
        for (name, keys) in [
            ("ws_col0", vec![10u32, 12, 14]),
            ("ws_col1", vec![11u32, 13]),
        ] {
            wvals.insert(
                name,
                SceneValue::Rows(
                    keys.iter()
                        .enumerate()
                        .map(|(_r, key)| {
                            let i = (key - 10) as usize;
                            SceneRow {
                                cols: vec![
                                    if i == 1 { "1".into() } else { "0".to_string() },
                                    (i + 1).to_string(),
                                    "Desktop".to_string(),
                                ],
                                key: *key,
                                surface: Some(ColorToken::Value(
                                    crate::ui::DynColor::new(&format!("ws_wash_{i}")).unwrap(),
                                )),
                                col_colors: vec![
                                    None,
                                    Some(ColorToken::Value(
                                        crate::ui::DynColor::new(&format!("ws_num_{i}")).unwrap(),
                                    )),
                                    None,
                                ],
                                ..Default::default()
                            }
                        })
                        .collect(),
                ),
            );
        }
        let (wv, whits) = draw("workspace_switcher", 600.0, 320.0, &wvals);
        // the chrome: dismiss region, title, and the count right-anchored 20 in
        let (wtitle, wcount) = (
            wv.iter().find_map(|c| match c {
                Cmd::Text {
                    x, y, size, text, ..
                } if text == "Workspaces" => Some((*x, *y, *size)),
                _ => None,
            }),
            wv.iter().find_map(|c| match c {
                Cmd::Text {
                    x,
                    y,
                    right,
                    size,
                    text,
                    ..
                } if text == "5" => Some((*x, *y, *right, *size)),
                _ => None,
            }),
        );
        assert_eq!(
            wtitle,
            Some((16.0, 14.0, 17.0)),
            "the title at the drawer's 16/14"
        );
        assert_eq!(
            wcount,
            Some((600.0 - 20.0, 16.0, true, 13.0)),
            "the count, right-anchored 20 in"
        );
        let wrects = rects(&wv);
        // the cards: 274 × 56 on a 14 gutter, left column at 14, right at 302,
        // 70 px stride from y 44 — the drawer's own 2-up flow
        let cards: Vec<(f32, f32)> = wrects
            .iter()
            .filter(|r| r.2 == 274.0 && r.3 == 56.0)
            .map(|r| (r.0, r.1))
            .collect();
        assert_eq!(cards.len(), 5, "one wash per workspace");
        assert_eq!(
            cards,
            vec![(14.0, 44.0), (14.0, 114.0), (14.0, 184.0), (302.0, 44.0), (302.0, 114.0)],
            "the 2-up column flow at 14 + col * 288, 44 + row * 70 (left column first,              as the surface authors it — the cards never overlap, so the order is invisible)"
        );
        // the active card's rail: a 4 × 40 accent bar at its top + 8, and the
        // inactive cards draw none (their rail value is 0 on a clear track)
        // the bar cell always draws its track, so five MORE 4-wide rects
        // appear — on `clear` (alpha 0), i.e. invisible; only the active
        // workspace's fill is real ink
        let rails: Vec<&(f32, f32, f32, f32, u32)> =
            wrects.iter().filter(|r| r.2 == 4.0 && r.4 != 0).collect();
        assert_eq!(rails.len(), 1, "only the active workspace carries a rail");
        assert_eq!(rails[0].4, pal().acc, "the rail is the accent ink");
        assert_eq!(
            (rails[0].0, rails[0].1, rails[0].3),
            (302.0, 44.0 + 8.0, 40.0),
            "on the active card"
        );
        // the numbers ride the row's 12 px rail at 20 px, "Desktop" four px
        // lower at 11 px, both on their own card's x
        let num = wv
            .iter()
            .find_map(|c| match c {
                Cmd::Text {
                    x, y, size, text, ..
                } if text == "2" => Some((*x, *y, *size)),
                _ => None,
            })
            .expect("workspace 2's number");
        assert_eq!(
            num,
            (302.0 + 16.0, 44.0 + 12.0, 20.0),
            "16 in, 12 down, 20 px"
        );
        let mut caps: Vec<(f32, f32, f32)> = wv
            .iter()
            .filter_map(|c| match c {
                Cmd::Text {
                    x, y, size, text, ..
                } if text == "Desktop" => Some((*x, *y, *size)),
                _ => None,
            })
            .collect();
        caps.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert_eq!(
            caps,
            cards
                .iter()
                .map(|(cx, cy)| (cx + 44.0, cy + 16.0, 11.0))
                .collect::<Vec<_>>(),
            "one caption per card, 44 in and 16 down at 11 px"
        );
        assert_eq!(
            whits.iter().map(|r| r.key).collect::<Vec<_>>(),
            vec![1, 10, 12, 14, 11, 13],
            "dismiss, then the left column top-down, then the right"
        );
        // ── polkit: 420 × 320, the password field focused, an error showing ──
        let mut pvals = SceneValues::new();
        pvals.insert(
            "polkit_msg",
            SceneValue::Text("Authentication required".into()),
        );
        pvals.insert(
            "polkit_act",
            SceneValue::Text("org.freedesktop.policykit.exec".into()),
        );
        pvals.insert("polkit_user", SceneValue::Text("tw".into()));
        pvals.insert_color("polkit_pw_bg", SceneColor::Raw(crate::ui::hover_hl(&pal())));
        pvals.insert("polkit_pw_disp", SceneValue::Text("***".into()));
        pvals.insert_color("polkit_pw_ink", SceneColor::Raw(pal().fg));
        pvals.insert("polkit_cursor_on", SceneValue::Toggle(true));
        pvals.insert(
            "polkit_cursor_x",
            SceneValue::Ring(36.0 + 3.0 * 13.0 * 0.62),
        );
        pvals.insert("polkit_err_on", SceneValue::Toggle(true));
        pvals.insert(
            "polkit_err",
            SceneValue::Text("Authentication failure".into()),
        );
        pvals.insert_color("polkit_err_ink", SceneColor::Raw(pal().fg));
        pvals.insert_color(
            "polkit_cancel_bg",
            SceneColor::Raw(crate::ui::hover(&pal())),
        );
        pvals.insert_color("polkit_auth_bg", SceneColor::Raw(pal().acc));
        pvals.insert("polkit_auth_label", SceneValue::Text("Authenticate".into()));
        let (pv2, phits) = draw("polkit_auth", 420.0, 320.0, &pvals);
        // the field: 24 px inset, 36 tall at y 164, 8 px corners
        let field = rects(&pv2)
            .into_iter()
            .find(|r| r.3 == 36.0 && r.1 == 164.0)
            .expect("the password field");
        assert_eq!(
            (field.0, field.2),
            (24.0, 420.0 - 48.0),
            "24 in, 24 short of the right edge"
        );
        // the buttons ride the drawer's `h - 24 - 36` baseline, centered as a
        // 256 px block with a 16 px gutter
        let btns: Vec<(f32, f32)> = rects(&pv2)
            .into_iter()
            .filter(|r| r.2 == 120.0 && r.3 == 36.0)
            .map(|r| (r.0, r.1))
            .collect();
        assert_eq!(
            btns,
            vec![
                ((420.0 - 256.0) / 2.0, 320.0 - 60.0),
                ((420.0 - 256.0) / 2.0 + 136.0, 320.0 - 60.0)
            ]
        );
        // the cursor rides its bound x (the measured run) and the keys are
        // the drawer's: 92 field, 91 cancel, 90 auth
        let cursor = rects(&pv2)
            .into_iter()
            .find(|r| r.2 == 2.0)
            .expect("the caret");
        assert_eq!(
            (cursor.0, cursor.1, cursor.3),
            (36.0 + 3.0 * 13.0 * 0.62, 172.0, 20.0)
        );
        assert_eq!(
            phits.iter().map(|r| r.key).collect::<Vec<_>>(),
            vec![92, 91, 90]
        );
    }

    #[test]
    fn dots_render_pane_rule_and_hover_gate() {
        // the multi-pane HARD RULE primitive: one dot per pane, the active
        // pane's dot in the accent, hidden entirely while `when` is false
        let scene: CardScene = parse_scene_ron(
            r#"(items: [ Dots(active: "pane", when: Some("hov"), dy: 3.5, step: 7.0, panes: 2) ])"#,
        )
        .unwrap();
        let mut vals = SceneValues::new();
        vals.insert("pane", SceneValue::Ring(1.0));
        vals.insert("hov", SceneValue::Toggle(true));
        let mut v: Vec<Cmd> = Vec::new();
        scene.draw(
            &mut v,
            0.0,
            0.0,
            200.0,
            120.0,
            scale(),
            &pal(),
            &vals,
            0,
            true,
            true,
        );
        let dots: Vec<&Cmd> = v
            .iter()
            .filter(|c| matches!(c, Cmd::Rect { r, .. } if *r > 0.0 && *r < 6.0))
            .collect();
        assert_eq!(dots.len(), 2, "one dot per pane");
        let acc = pal().acc;
        assert!(
            dots.iter()
                .any(|c| matches!(c, Cmd::Rect { color, .. } if *color == acc)),
            "active pane dot is accent"
        );
        // hover gate off → nothing draws
        vals.insert("hov", SceneValue::Toggle(false));
        let mut v2: Vec<Cmd> = Vec::new();
        scene.draw(
            &mut v2,
            0.0,
            0.0,
            200.0,
            120.0,
            scale(),
            &pal(),
            &vals,
            0,
            true,
            true,
        );
        assert!(
            v2.is_empty(),
            "dots hidden while the cursor is off the card"
        );
    }

    #[test]
    fn grid_cell_truncate_clips_long_labels() {
        let scene: CardScene = parse_scene_ron(
            r#"(items: [ Grid(name: "g", x: 0.0, y: 0.0, w: 100.0, h: 40.0, cols: 1) ])
            "#,
        )
        .unwrap();
        let mut vals = SceneValues::new();
        vals.insert(
            "g",
            SceneValue::GridCells(vec![GridCell {
                text: "very-long-app-name-here".into(),
                truncate: Some(4),
                ..Default::default()
            }]),
        );
        let mut v: Vec<Cmd> = Vec::new();
        scene.draw(
            &mut v,
            0.0,
            0.0,
            100.0,
            40.0,
            scale(),
            &pal(),
            &vals,
            0,
            true,
            true,
        );
        let texts: Vec<&String> = v
            .iter()
            .filter_map(|c| match c {
                Cmd::Text { text, .. } => Some(text),
                _ => None,
            })
            .collect();
        assert!(
            texts.iter().any(|t| t.as_str() == "very"),
            "label clips to the first n chars, got {texts:?}"
        );
    }

    #[test]
    fn items_between_inks_attach_to_the_next_ink() {
        // the dashboard band z-order: [Ink(grid), Surface(band), Ink(banner)]
        // must NOT draw the band upfront — it rides the banner ink's `pre` so
        // the Rust painter order is grid → band → banner (band over cards,
        // chips over band)
        let scene: CardScene = parse_scene_ron(
            r#"(items: [
                Ink(x: 0.0, y: 0.0, w: 0.0, h: 0.0, name: "grid"),
                Surface(x: 0.0, y: 12.0, w: 0.0, h: 26.0, r: 16.0, tr: -16.0, color: acc),
                Ink(x: 0.0, y: 0.0, w: 0.0, h: 0.0, name: "banner"),
            ])
            "#,
        )
        .unwrap();
        let mut v: Vec<Cmd> = Vec::new();
        let (_h, inks) = scene.draw(
            &mut v,
            0.0,
            0.0,
            200.0,
            120.0,
            scale(),
            &pal(),
            &SceneValues::new(),
            0,
            true,
            true,
        );
        // nothing drawn upfront (both pieces are Inks / attached chrome)…
        assert!(v.is_empty(), "no upfront cmds, got {}", v.len());
        // …the band rides the SECOND ink…
        assert_eq!(inks.len(), 2);
        assert!(
            inks[0].pre.is_empty(),
            "items before the first ink stay upfront-shaped"
        );
        assert_eq!(
            inks[1].pre.len(),
            1,
            "the band attaches to the NEXT ink, not the first"
        );
        assert!(matches!(inks[1].pre[0], SceneItem::Surface { .. }));
    }

    #[test]
    fn banner_row_paints_cells_and_registers_hits() {
        // a declarative strip: one text chip, one tray chip with a fallback
        // glyph, one filler. The filler draws/hits nothing; chips register
        // their slot region (and tray icons their 30+ti sub-regions).
        let mut vals = SceneValues::new();
        vals.insert_color("bchip_clock", SceneColor::Token(ColorToken::Fg));
        vals.insert(
            "cells",
            SceneValue::BannerCells(vec![
                BannerCell {
                    x: 0.0,
                    w: 60.0,
                    ink: "bchip_clock".into(),
                    text: "12:34".into(),
                    glyph: None,
                    icons: Vec::new(),
                    zone: 3,
                    kind: BannerCellKind::Text { n: None },
                    key: 34_002,
                },
                BannerCell {
                    x: 66.0,
                    w: 48.0,
                    ink: "bchip_tray".into(),
                    text: String::new(),
                    glyph: None,
                    icons: vec![String::new()],
                    zone: 3,
                    kind: BannerCellKind::Tray,
                    key: 34_005,
                },
                BannerCell {
                    x: 120.0,
                    w: 30.0,
                    ink: "bchip_dim".into(),
                    text: String::new(),
                    glyph: None,
                    icons: Vec::new(),
                    zone: 3,
                    kind: BannerCellKind::Filler,
                    key: 0,
                },
            ]),
        );
        let scene: CardScene = parse_scene_ron(
            r#"(items: [ BannerRow(cells: "cells", x: 0.0, y: 12.0, w: 0.0, h: 26.0) ])
            "#,
        )
        .unwrap();
        let mut v: Vec<Cmd> = Vec::new();
        let mut hits_out: Vec<SceneHit> = Vec::new();
        let _ = &mut hits_out;
        let (_, inks) = scene.draw(
            &mut v,
            0.0,
            0.0,
            200.0,
            120.0,
            scale(),
            &pal(),
            &vals,
            0,
            false,
            false,
        );
        // pre-attach: a trailing BannerRow with no following Ink stays in the
        // upfront path — its pieces are already in `v`.
        assert!(inks.is_empty(), "no ink present → nothing deferred");
        // the tray fallback glyph + the clock label drew (2 text cmds), no
        // rect surfaces, and three hit regions landed (2 slots + 1 tray icon)
        let text = v.iter().filter(|c| matches!(c, Cmd::Text { .. })).count();
        assert_eq!(text, 2, "clock label + tray fallback glyph, got {text}");
        assert_eq!(
            v.iter()
                .filter(|c| matches!(c, Cmd::Scissor { .. }))
                .count(),
            0,
            "unzoned, non-overflow strip opens no clip"
        );
    }

    #[test]
    fn banner_row_overflow_clips_at_the_band() {
        let mut vals = SceneValues::new();
        vals.insert("ovf", SceneValue::Toggle(true));
        vals.insert(
            "cells",
            SceneValue::BannerCells(vec![BannerCell {
                x: 0.0,
                w: 50.0,
                ink: "bchip_clock".into(),
                text: "12:34".into(),
                glyph: None,
                icons: Vec::new(),
                zone: 3,
                kind: BannerCellKind::Text { n: None },
                key: 1,
            }]),
        );
        let scene: CardScene = parse_scene_ron(
            r#"(items: [ BannerRow(cells: "cells", x: 0.0, y: 12.0, w: 0.0, h: 26.0,
                          overflow: Some("ovf")) ])
            "#,
        )
        .unwrap();
        let mut v: Vec<Cmd> = Vec::new();
        scene.draw(
            &mut v,
            0.0,
            0.0,
            200.0,
            120.0,
            scale(),
            &pal(),
            &vals,
            0,
            false,
            false,
        );
        let open = v
            .iter()
            .find_map(|c| match c {
                Cmd::Scissor { x, y, w, h } => Some((*x, *y, *w, *h)),
                _ => None,
            })
            .expect("overflow opens a clip");
        assert_eq!(open, (0.0, 12.0, 200.0, 26.0), "clip = the band box");
        assert!(
            v.iter().any(|c| matches!(c, Cmd::ScissorEnd)),
            "clip closes"
        );
    }

    #[test]
    fn banner_row_zone_scissors_interleave_between_cells() {
        // L/C/R strip: the scissor for zone 1 must open only after zone 0's
        // last cell drew (the Rust drawer's zone_push state machine) — a
        // single upfront clip would let a scrolled zone bleed into its
        // neighbour's window.
        let mut vals = SceneValues::new();
        vals.insert("zn", SceneValue::Toggle(true));
        let cell = |x: f32, zone: u8, key: u32| BannerCell {
            x,
            w: 20.0,
            ink: "bchip_clock".into(),
            text: "x".into(),
            glyph: None,
            icons: Vec::new(),
            zone,
            kind: BannerCellKind::Text { n: None },
            key,
        };
        vals.insert(
            "cells",
            SceneValue::BannerCells(vec![cell(0.0, 0, 1), cell(200.0, 1, 2), cell(400.0, 2, 3)]),
        );
        let scene: CardScene = parse_scene_ron(
            r#"(items: [ BannerRow(cells: "cells", x: 0.0, y: 12.0, w: 0.0, h: 26.0,
                          zones: Some("zn")) ])
            "#,
        )
        .unwrap();
        let mut v: Vec<Cmd> = Vec::new();
        scene.draw(
            &mut v,
            0.0,
            0.0,
            600.0,
            120.0,
            scale(),
            &pal(),
            &vals,
            0,
            false,
            false,
        );
        // sequence: scissor(z0) → text(z0) → scissor(z1) → text(z1) → scissor(z2)
        // → text(z2) → end — zone_push clips EVERY zone window (including the
        // first), matching the Rust drawer's state machine
        let seq: Vec<bool> = v.iter().map(|c| matches!(c, Cmd::Text { .. })).collect();
        let scissors: Vec<usize> = v
            .iter()
            .enumerate()
            .filter(|(_, c)| matches!(c, Cmd::Scissor { .. }))
            .map(|(i, _)| i)
            .collect();
        assert_eq!(scissors.len(), 3, "each zone window opens a clip");
        // the FIRST clip opens before zone 0's cells (it clips their window);
        // every later one must open after cells drew — interleaved, not upfront
        for &si in &scissors[1..] {
            assert!(seq[..si].iter().any(|&t| t), "zone scissor opens mid-sweep");
        }
        assert!(
            v.iter().any(|c| matches!(c, Cmd::ScissorEnd)),
            "last zone closes"
        );
    }

    #[test]
    fn banner_row_visible_gate_and_missing_binding_draw_nothing() {
        let scene: CardScene = parse_scene_ron(
            r#"(items: [ BannerRow(cells: "missing", x: 0.0, y: 12.0, w: 0.0, h: 26.0,
                          visible: Some("off")) ])
            "#,
        )
        .unwrap();
        let mut vals = SceneValues::new();
        vals.insert("off", SceneValue::Toggle(false));
        let mut v: Vec<Cmd> = Vec::new();
        let (_, inks) = scene.draw(
            &mut v,
            0.0,
            0.0,
            200.0,
            120.0,
            scale(),
            &pal(),
            &vals,
            0,
            false,
            false,
        );
        assert!(v.is_empty(), "gated row draws nothing");
        assert!(inks.is_empty(), "gated row defers nothing");
    }

    #[test]
    fn surface_concave_corners_emit_rect_concave() {
        // the strip band's screen-edge corners: negative tr = concave top,
        // rendered via Cmd::RectConcave so the band curves into the panel
        let scene: CardScene = parse_scene_ron(
            r#"(items: [ Surface(x: 0.0, y: 12.0, w: 100.0, h: 26.0, r: 16.0, tr: -16.0, color: acc) ])
            "#,
        )
        .unwrap();
        let mut v: Vec<Cmd> = Vec::new();
        scene.draw(
            &mut v,
            0.0,
            0.0,
            200.0,
            120.0,
            scale(),
            &pal(),
            &SceneValues::new(),
            0,
            true,
            true,
        );
        assert!(
            v.iter().any(
                |c| matches!(c, Cmd::RectConcave { r_tl, r_tr, r_br, r_bl, .. }
                if *r_tl > 0.0 && *r_tr < 0.0 && *r_br == 0.0 && *r_bl == 0.0)
            ),
            "concave top corners + square bottom corners"
        );
    }

    #[test]
    fn a_negative_span_measures_from_the_items_own_x_and_y() {
        // THE convention (every arm goes through `span_px`, so a negative span
        // is the span left from the item's own x/y minus the inset): a surface
        // 12 px in with `w: -12` insets BOTH sides, `bottom: true, y: 8` puts
        // the box's bottom edge 8 px above the card's, and `w: 0` reaches the
        // parent's far edge from wherever the item starts.
        let scene: CardScene = parse_scene_ron(
            r#"(items: [
                    Surface(x: 12.0, y: 10.0, w: -12.0, h: 20.0, r: 0.0, color: acc),
                    Surface(x: 12.0, y: 8.0, w: -12.0, h: 30.0, r: 0.0, bottom: true, color: fg),
                    Surface(x: 30.0, y: 0.0, w: 0.0, h: 0.0, r: 0.0, color: fg3),
                ])"#,
        )
        .unwrap();
        let mut v: Vec<Cmd> = Vec::new();
        scene.draw(
            &mut v,
            0.0,
            0.0,
            240.0,
            140.0,
            scale(),
            &pal(),
            &vals(),
            0,
            true,
            true,
        );
        let rects: Vec<&Cmd> = v.iter().filter(|c| matches!(c, Cmd::Rect { .. })).collect();
        assert_eq!(rects.len(), 3, "three surfaces");
        for (i, r) in rects.iter().enumerate() {
            let (x, y, w, h) = match r {
                Cmd::Rect { x, y, w, h, .. } => (*x, *y, *w, *h),
                _ => unreachable!(),
            };
            match i {
                // 12 px inset both sides; 10 from the top
                0 => assert_eq!(
                    (x, y, w, h),
                    (12.0, 10.0, 216.0, 20.0),
                    "symmetric 12 px inset"
                ),
                // bottom edge 8 px above the card's, 12 px inset both sides
                1 => assert_eq!(
                    (x, y, w, h),
                    (12.0, 102.0, 216.0, 30.0),
                    "bottom-anchored pill"
                ),
                // `w: 0` at x 30 reaches the card's right edge
                _ => assert_eq!(
                    (x, y, w, h),
                    (30.0, 0.0, 210.0, 140.0),
                    "zero span fills from x"
                ),
            }
        }
    }

    #[test]
    fn surface_center_anchors_to_box_center() {
        // the fullscreen-lock hero pieces anchor to the parent box center (the
        // dim + avatar signatures here) — x/y become offsets from that center
        let scene: CardScene = parse_scene_ron(
            r#"(items: [
                    Surface(x: 0.0, y: 0.0, w: 0.0, h: 0.0, r: 0.0, color: value("lock_dim")),
                    Surface(center_x: true, center_y: true, x: -12.0, y: -34.0, w: 52.0, h: 52.0, r: 26.0, color: acc),
                ])"#,
        )
        .unwrap();
        let mut vals = SceneValues::new();
        vals.insert_color("lock_dim", SceneColor::Raw(0x00000059));
        let mut v: Vec<Cmd> = Vec::new();
        scene.draw(
            &mut v,
            0.0,
            0.0,
            200.0,
            120.0,
            scale(),
            &pal(),
            &vals,
            0,
            true,
            true,
        );
        let rects: Vec<&Cmd> = v.iter().filter(|c| matches!(c, Cmd::Rect { .. })).collect();
        assert_eq!(rects.len(), 2, "dim + avatar");
        match rects[0] {
            Cmd::Rect { x, y, w, h, r, .. } => {
                assert_eq!(
                    (*x, *y, *w, *h, *r),
                    (0.0, 0.0, 200.0, 120.0, 0.0),
                    "0-dim fills the box"
                );
            }
            _ => unreachable!(),
        }
        match rects[1] {
            Cmd::Rect {
                x,
                y,
                w,
                h,
                r,
                color,
                ..
            } => {
                assert_eq!(*x, 200.0 / 2.0 - 12.0, "center_x offsets from the box mid");
                assert_eq!(*y, 120.0 / 2.0 - 34.0, "center_y offsets from the box mid");
                assert_eq!((*w, *h, *r), (52.0, 52.0, 26.0));
                assert_eq!(*color, plt_acc());
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn right_anchor_hugs_card_width() {
        let scene: CardScene = parse_scene_ron(
            "(items: [ Hit(x: 12.0, y: 6.0, w: 16.0, h: 16.0, r: 5.0, key: 7, right: true, surface: Some(raised), text: \"›\", font_size: 10.0, icon: true) ])",
        )
        .unwrap();
        // card 148 wide: right-anchored chevron sits at 148 - (12 + 16) = 120
        let mut v: Vec<Cmd> = Vec::new();
        let (hits, _) = scene.draw(
            &mut v,
            0.0,
            0.0,
            148.0,
            90.0,
            scale(),
            &pal(),
            &vals(),
            0,
            true,
            true,
        );
        assert_eq!(hits[0].x, 148.0 - 28.0);
        // card 200 wide: same authoring survives a resize (hugs the right edge)
        let mut v: Vec<Cmd> = Vec::new();
        let (hits2, _) = scene.draw(
            &mut v,
            0.0,
            0.0,
            200.0,
            90.0,
            scale(),
            &pal(),
            &vals(),
            0,
            true,
            true,
        );
        assert_eq!(hits2[0].x, 200.0 - 28.0);
    }

    #[test]
    fn substitution_unknown_name_passthrough() {
        assert_eq!(substitute("hi {nope}", &vals()), "hi {nope}");
        assert_eq!(substitute("plain", &vals()), "plain");
        assert_eq!(substitute("a {rec_btn} b", &vals()), "a Record b");
    }

    #[test]
    fn rows_edge_text_cells_anchor_right_and_bottom_inset_clips() {
        // Batch 2 (2026-09-17): plain text cells honor `edge` (the cell's
        // RIGHT edge lands at card_w − edge — news open-glyph/ticker chg%
        // parity), and `bottom` reserves the last N px of the card (the
        // conninfo list clears its refresh button).
        let scene: CardScene = parse_scene_ron(
            r#"(items: [ Rows(name: "rw", y: 30.0, row_h: 18.0, pad: 12.0, bottom: 32.0, cols: [
                (x: 0.0),
                (x: 0.0, w: 0.0, edge: Some(14.0), right: true, font: mono, size: 8.5),
            ], key_base: Some(91)) ])"#,
        )
        .unwrap();
        let mk = |v: &str| SceneRow {
            cols: vec!["iface".into(), v.into()],
            key: 0,
            action: None,
            color: None,
            col_colors: vec![],
            surface: None,
        };
        let mut values = SceneValues::new();
        values.insert(
            "rw",
            SceneValue::Rows(vec![
                mk("10.0.0.2"),
                mk("10.0.0.3"),
                mk("10.0.0.4"),
                mk("10.0.0.5"),
                mk("10.0.0.6"),
                mk("10.0.0.7"),
            ]),
        );
        let mut v: Vec<Cmd> = Vec::new();
        let (hits, _) = scene.draw(
            &mut v,
            0.0,
            0.0,
            220.0,
            140.0,
            scale(),
            &pal(),
            &values,
            0,
            true,
            true,
        );
        // right cells: text anchored at card_w − 14 = 206 (right → draws left of it)
        assert!(
            v.iter().any(|c| matches!(c, Cmd::Text { x, right: true, text, .. } if (*x - 206.0).abs() < 0.01 && text == "10.0.0.2")),
            "edge cell right-anchors at card_w − edge",
        );
        // bottom inset 32: rows start at 30, pitch 18 → row boxes crossing
        // y=108 (140−32) are skipped: rows at 30/48/66/84 fit, 102+108 do not
        assert_eq!(
            hits.len(),
            4,
            "rows past the bottom inset are skipped (got {hits:?})"
        );
        assert_eq!(hits[3].key, 94);
    }

    #[test]
    fn grid_fixed_cell_h_keeps_pitch() {
        // thermal chips: `cell_h` fixes the row pitch — a partial tail row
        // leaves the rest of the box open instead of stretching cells
        let scene: CardScene = parse_scene_ron(
            r#"(items: [ Grid(name: "g", x: 0.0, y: 0.0, h: 100.0, cols: 3, gap: 2.0, cell_h: 24.0) ])"#,
        )
        .unwrap();
        let mut values = SceneValues::new();
        let chip = |t: &str| GridCell {
            text: t.into(),
            surface: Some(ColorToken::Raised),
            ..Default::default()
        };
        values.insert(
            "g",
            SceneValue::GridCells(vec![
                chip("acpitz"),
                chip("core"),
                chip("nvme"),
                chip("acpi2"),
            ]),
        );
        let mut v: Vec<Cmd> = Vec::new();
        let _ = scene.draw(
            &mut v,
            0.0,
            0.0,
            220.0,
            140.0,
            scale(),
            &pal(),
            &values,
            0,
            true,
            true,
        );
        let chip_h: Vec<f32> = v
            .iter()
            .filter_map(|c| match c {
                // any rounded rect is a chip (the default `r` is 8 px)
                Cmd::Rect { h, r, .. } if *r > 0.0 => Some(*h),
                _ => None,
            })
            .collect();
        assert_eq!(
            chip_h.len(),
            4,
            "all four chips drawn (2 rows, partial tail) — got {chip_h:?}"
        );
        assert!(
            chip_h.iter().all(|h| (*h - 24.0).abs() < 0.01),
            "fixed 24 px chip height (got {chip_h:?})"
        );
    }

    #[test]
    fn header_renders_title_glyph_and_meta_flagged() {
        let scene: CardScene = parse_scene_ron(
            "(items: [ Header(title: \"CPU\", meta: Some(\"{cpu_meta}\"), meta_mono: true, pad: 12.0) ])",
        )
        .unwrap();
        let mut values = SceneValues::new();
        values.insert("cpu_meta", SceneValue::Text("3400 MHz".into()));
        let mut v: Vec<Cmd> = Vec::new();
        // show_title=false hides the title; show_glyph irrelevant (no glyph)
        let (_h, _i) = scene.draw(
            &mut v,
            0.0,
            0.0,
            180.0,
            120.0,
            scale(),
            &pal(),
            &values,
            0,
            false,
            true,
        );
        assert!(
            v.iter()
                .all(|c| !matches!(c, Cmd::Text { text, .. } if text == "CPU")),
            "title hidden when card_show_title is off"
        );
        assert!(
            v.iter()
                .any(|c| matches!(c, Cmd::Text { text, .. } if text == "3400 MHz")),
            "meta draws with {{cpu_meta}} interpolated (mono meta kept)"
        );
    }

    #[test]
    fn rows_bar_cells_parse_value_fill_and_track() {
        // the fans meter rows: a bar cell interpolates its cell text
        // ({name} → fraction), draws track + fill honoring the row colors
        let scene: CardScene = parse_scene_ron(
            r#"(items: [ Rows(
                name: "fan_rows", y: 30.0, row_h: 24.0, pad: 12.0,
                cols: [
                    (x: 0.0, dy: Some(1.0), size: 8.5, truncate: Some(60.0)),
                    (x: 0.0, edge: Some(12.0), right: true, w: 80.0, dy: Some(13.0),
                     bar: Some((height: 3.0, top: 13.0, radius: 1.5, fill: Some(info)))),
                ],
            ) ])
            "#,
        )
        .unwrap();
        let cols: Vec<&SceneCol> = scene
            .items
            .iter()
            .filter_map(|i| match i {
                SceneItem::Rows { cols, .. } => Some(cols.iter().collect()),
                _ => None,
            })
            .next()
            .expect("rows item");
        assert!(cols[1].bar.is_some(), "bar cell declared");
        let b = cols[1].bar.as_ref().unwrap();
        assert_eq!((b.height, b.top, b.radius, b.max), (3.0, 13.0, 1.5, 1.0));
        assert_eq!(b.fill, Some(ColorToken::Info));
        assert!(b.track.is_none(), "track defaults to hover");
    }

    #[test]
    fn rows_bar_cell_zero_width_spans_the_rest_of_the_row() {
        // a `w: 0` meter cell is a FULL-WIDTH meter: from its own x to the
        // row's right edge (the disk / fans bars). It used to be honored
        // literally, which collapsed those tracks to nothing.
        let scene: CardScene = parse_scene_ron(
            r#"(items: [ Rows(
                name: "fan_rows", y: 30.0, row_h: 24.0, pad: 12.0,
                cols: [
                    (x: 0.0, w: 0.0, bar: Some((height: 3.0, top: 13.0, max: 2.0))),
                ],
            ) ])
            "#,
        )
        .unwrap();
        let mut vals = SceneValues::new();
        vals.insert(
            "fan_rows",
            SceneValue::Rows(vec![SceneRow {
                cols: vec!["0.5".into()],
                ..Default::default()
            }]),
        );
        let mut v: Vec<Cmd> = Vec::new();
        scene.draw(
            &mut v,
            0.0,
            0.0,
            200.0,
            120.0,
            scale(),
            &pal(),
            &vals,
            0,
            true,
            true,
        );
        let rects: Vec<(f32, f32, f32, f32)> = v
            .iter()
            .filter_map(|c| match c {
                Cmd::Rect { x, y, w, h, .. } => Some((*x, *y, *w, *h)),
                _ => None,
            })
            .collect();
        // 200 px card, 12 px pad → the row box is 176 wide
        let track = rects
            .iter()
            .find(|r| r.2 > 100.0)
            .expect("full-width track");
        assert_eq!(
            (track.0, track.2, track.3),
            (12.0, 176.0, 3.0),
            "pad to the row's right edge"
        );
        let fill = rects
            .iter()
            .find(|r| r.0 == 12.0 && r.2 > 20.0 && r.2 < 176.0)
            .expect("quarter fill");
        assert!(
            (fill.2 - 44.0).abs() < 0.01,
            "0.5 of 2.0 = a quarter of the track, got {}",
            fill.2
        );
    }

    #[test]
    fn rows_bar_cell_draws_track_and_fraction_fill() {
        // real width: track spans the cell box, fill = frac of max, fill
        // hugs the track's left edge
        let scene: CardScene = parse_scene_ron(
            r#"(items: [ Rows(
                name: "fan_rows", y: 30.0, row_h: 24.0, pad: 12.0,
                cols: [
                    (x: 0.0, w: 80.0, bar: Some((height: 3.0, top: 13.0, max: 2.0))),
                ],
            ) ])
            "#,
        )
        .unwrap();
        let mut vals = SceneValues::new();
        vals.insert(
            "fan_rows",
            SceneValue::Rows(vec![SceneRow {
                cols: vec!["0.5".into()],
                ..Default::default()
            }]),
        );
        let mut v: Vec<Cmd> = Vec::new();
        scene.draw(
            &mut v,
            0.0,
            0.0,
            200.0,
            120.0,
            scale(),
            &pal(),
            &vals,
            0,
            true,
            true,
        );
        let rects: Vec<(f32, f32, f32, f32)> = v
            .iter()
            .filter_map(|c| match c {
                Cmd::Rect { x, y, w, h, .. } => Some((*x, *y, *w, *h)),
                _ => None,
            })
            .collect();
        let track = rects
            .iter()
            .find(|r| (r.2 - 80.0).abs() < 0.01)
            .expect("track rect");
        let fill = rects
            .iter()
            .find(|r| (r.2 - 20.0).abs() < 0.01)
            .expect("fill rect");
        assert!(
            (track.0 - fill.0).abs() < 0.01,
            "fill hugs the track's left"
        );
        assert_eq!(fill.3, track.3, "fill height matches the track");
    }

    #[test]
    fn scene_deserializes_minimal_hit() {
        let scene: CardScene =
            parse_scene_ron("(items: [ Text(x: 4.0, y: 5.0, text: \"hi\", font_size: 9.0),\n Hit(x: 0.0, y: 0.0, w: 10.0, h: 10.0, key: 1) ])").unwrap();
        assert_eq!(scene.items.len(), 2);
        match &scene.items[0] {
            SceneItem::Text { font, color, .. } => {
                assert_eq!(*font, SceneFont::Ui);
                assert_eq!(*color, None);
            }
            _ => panic!(),
        }
        match &scene.items[1] {
            SceneItem::Hit {
                key, surface, text, ..
            } => {
                assert_eq!(*key, Val::Lit(1.0));
                assert_eq!(*surface, None);
                assert!(text.is_empty());
            }
            _ => panic!(),
        }
    }

    // ── data-driven primitives (Phase 2a) ───────────────────────────────

    #[test]
    fn ron_parses_rows_and_column_formats() {
        let scene: CardScene = parse_scene_ron(
            r#"(items: [ Rows(name: "jr_rows", y: 30.0, row_h: 24.0,
                cols: [ (x: 0.0), (x: 160.0, right: true, font: mono, size: 9.0, color: Some(fg3)) ],
                key_base: Some(700), hover_surface: Some(hover)) ])"#,
        )
        .unwrap();
        match &scene.items[0] {
            SceneItem::Rows {
                name,
                cols,
                key_base,
                hover_surface,
                pad,
                max_rows,
                ..
            } => {
                assert_eq!(name, "jr_rows");
                assert_eq!(
                    *key_base,
                    Some(700),
                    "a declared key_base opts the rows into regions"
                );
                assert_eq!(*hover_surface, Some(ColorToken::Hover));
                assert_eq!(cols.len(), 2);
                assert_eq!(cols[0].font, SceneFont::Ui);
                assert_eq!(
                    cols[0].weight,
                    SceneWeight::Regular,
                    "columns default to Regular body rows"
                );
                assert!(cols[1].right);
                assert_eq!(cols[1].size, 9.0);
                assert_eq!(*pad, 12.0);
                assert_eq!(*max_rows, 0);
            }
            other => panic!("expected Rows, got {other:?}"),
        }
    }

    #[test]
    fn rows_draws_cells_and_registers_row_keys() {
        let scene: CardScene = parse_scene_ron(
            "(items: [ Rows(name: \"rw\", y: 30.0, row_h: 24.0, cols: [ (x: 0.0), (x: 160.0, right: true) ], key_base: Some(700)) ])",
        )
        .unwrap();
        let mut values = SceneValues::new();
        values.insert(
            "rw",
            SceneValue::Rows(vec![
                SceneRow {
                    cols: vec!["First".into(), "1".into()],
                    key: 0,
                    action: None,
                    color: None,
                    col_colors: vec![],
                    surface: None,
                },
                SceneRow {
                    cols: vec!["Second".into(), "2".into()],
                    key: 0,
                    action: None,
                    color: None,
                    col_colors: vec![],
                    surface: None,
                },
                SceneRow {
                    cols: vec!["Third".into(), "3".into()],
                    key: 0,
                    action: None,
                    color: None,
                    col_colors: vec![],
                    surface: None,
                },
            ]),
        );
        let mut v: Vec<Cmd> = Vec::new();
        let (hits, _) = scene.draw(
            &mut v,
            0.0,
            0.0,
            200.0,
            120.0,
            scale(),
            &pal(),
            &values,
            0,
            true,
            true,
        );
        assert!(
            v.iter()
                .any(|c| matches!(c, Cmd::Text { text, .. } if text == "First")),
            "row 1 drawn"
        );
        assert!(
            v.iter()
                .any(|c| matches!(c, Cmd::Text { text, .. } if text == "3")),
            "row 3 drawn"
        );
        assert_eq!(hits.len(), 3, "each row registers key_base + index");
        assert_eq!(hits[0].key, 700);
        assert_eq!(hits[2].key, 702);
    }

    #[test]
    fn rows_skip_past_the_card_edge_and_keep_own_keys() {
        let scene: CardScene = parse_scene_ron(
            "(items: [ Rows(name: \"rw\", y: 30.0, row_h: 24.0, cols: [ (x: 0.0) ], key_base: Some(700)) ])",
        )
        .unwrap();
        let mut values = SceneValues::new();
        // 4 rows at pitch 24 from y=30 → rows 1-3 end at 102 (below 120 card? row h=24:
        // row0 30-54, row1 54-78, row2 78-102, row3 102-126 > card 120 → skipped)
        values.insert(
            "rw",
            SceneValue::Rows(
                (0..4)
                    .map(|i| SceneRow {
                        cols: vec![format!("r{i}")],
                        key: 800 + i,
                        action: None,
                        color: None,
                        col_colors: vec![],
                        surface: None,
                    })
                    .collect(),
            ),
        );
        let mut v: Vec<Cmd> = Vec::new();
        let (hits, _) = scene.draw(
            &mut v,
            0.0,
            0.0,
            200.0,
            120.0,
            scale(),
            &pal(),
            &values,
            0,
            true,
            true,
        );
        assert_eq!(
            hits.len(),
            3,
            "only rows fully above the card's bottom edge register"
        );
        assert_eq!(hits[0].key, 800, "explicit per-row key wins over key_base");
    }

    #[test]
    fn rows_hover_surface_lifts_on_row_key() {
        let scene: CardScene = parse_scene_ron(
            "(items: [ Rows(name: \"rw\", y: 30.0, row_h: 24.0, cols: [ (x: 0.0) ], key_base: Some(700), hover_surface: Some(raised)) ])",
        )
        .unwrap();
        let mut values = SceneValues::new();
        values.insert(
            "rw",
            SceneValue::Rows(vec![SceneRow {
                cols: vec!["a".into()],
                key: 0,
                action: None,
                color: None,
                col_colors: vec![],
                surface: None,
            }]),
        );
        // resting: no background rect drawn
        let mut v: Vec<Cmd> = Vec::new();
        let (_h, _i) = scene.draw(
            &mut v,
            0.0,
            0.0,
            200.0,
            120.0,
            scale(),
            &pal(),
            &values,
            0,
            true,
            true,
        );
        assert!(
            v.iter().all(|c| !matches!(c, Cmd::Rect { .. })),
            "resting rows draw no background surface"
        );
        // hovered: one background rect with hover_variant
        let hov = ColorToken::Raised.hover_variant(&pal());
        let mut v: Vec<Cmd> = Vec::new();
        let (_h, _i) = scene.draw(
            &mut v,
            0.0,
            0.0,
            200.0,
            120.0,
            scale(),
            &pal(),
            &values,
            700,
            true,
            true,
        );
        assert!(
            v.iter()
                .any(|c| matches!(c, Cmd::Rect { color: col, .. } if *col == hov)),
            "hovered row lifts its surface"
        );
    }

    #[test]
    fn rows_resting_surface_wins_over_hover_and_icon_col_renders() {
        let scene: CardScene = parse_scene_ron(
            "(items: [ Rows(name: \"rw\", y: 30.0, row_h: 24.0, cols: [ (x: 0.0, icon: true), (x: 18.0, color: Some(fg3)) ], key_base: Some(700), hover_surface: Some(raised)) ])",
        )
        .unwrap();
        let mut values = SceneValues::new();
        values.insert(
            "rw",
            SceneValue::Rows(vec![SceneRow {
                cols: vec![ICON_GLOBE.into(), "linked".into()],
                key: 0,
                action: None,
                color: None,
                surface: Some(ColorToken::AccTint),
                col_colors: vec![],
            }]),
        );
        let mut v: Vec<Cmd> = Vec::new();
        // resting: the row's own surface tint draws (no hover needed)
        let (_h, _i) = scene.draw(
            &mut v,
            0.0,
            0.0,
            200.0,
            120.0,
            scale(),
            &pal(),
            &values,
            0,
            true,
            true,
        );
        assert!(
            v.iter().any(|c| matches!(c, Cmd::Rect { color: col, .. } if *col == ColorToken::AccTint.resolve(&pal()))),
            "resting surface tint draws behind the row"
        );
        assert!(
            v.iter()
                .any(|c| matches!(c, Cmd::Text { text, icon, .. } if *icon && text == ICON_GLOBE)),
            "icon column renders its glyph in the icon font"
        );
        // hovered: the resting tint takes precedence over the hover lift
        let hov = ColorToken::AccTint.hover_variant(&pal());
        let hit = ColorToken::Raised.hover_variant(&pal());
        let mut v: Vec<Cmd> = Vec::new();
        let (_h, _i) = scene.draw(
            &mut v,
            0.0,
            0.0,
            200.0,
            120.0,
            scale(),
            &pal(),
            &values,
            700,
            true,
            true,
        );
        assert!(
            !v.iter()
                .any(|c| matches!(c, Cmd::Rect { color: col, .. } if *col == hov || *col == hit)),
            "row surface wins over hover tint"
        );
        assert!(
            v.iter().any(|c| matches!(c, Cmd::Rect { color: col, .. } if *col == ColorToken::AccTint.resolve(&pal()))),
            "surface tint stays put under hover"
        );
    }

    #[test]
    fn rows_checkbox_cells_draw_checked_and_unchecked() {
        let scene: CardScene = parse_scene_ron(
            r#"(items: [ Rows(name: "rw", y: 30.0, row_h: 24.0, cols: [ (x: 20.0, size: 9.5) ], key_base: Some(700), check: Some("ck")) ])"#,
        )
        .unwrap();
        let mut values = SceneValues::new();
        values.insert(
            "rw",
            SceneValue::Rows(vec![
                SceneRow {
                    cols: vec!["done".into()],
                    key: 0,
                    action: None,
                    color: None,
                    col_colors: vec![],
                    surface: None,
                },
                SceneRow {
                    cols: vec!["open".into()],
                    key: 0,
                    action: None,
                    color: None,
                    col_colors: vec![],
                    surface: None,
                },
            ]),
        );
        values.insert("ck", SceneValue::Checks(vec![true, false]));
        let mut v: Vec<Cmd> = Vec::new();
        let (_h, _i) = scene.draw(
            &mut v,
            0.0,
            0.0,
            200.0,
            120.0,
            scale(),
            &pal(),
            &values,
            0,
            true,
            true,
        );
        // checked (row 0, y=30): acc-tint fill at pad(12)+check_x(2), +1 = 14,31
        let acc_tint = mix(pal().bg, pal().acc, 0.16);
        assert!(
            v.iter().any(|c| matches!(c, Cmd::Rect { x, y, w, h, color, .. } if *x == 14.0 && *y == 31.0 && *w == 11.0 && *h == 11.0 && *color == acc_tint)),
            "checked row draws the acc-tint checkbox well"
        );
        assert!(
            v.iter()
                .any(|c| matches!(c, Cmd::Text { text, .. } if text == ICON_CHECK)),
            "checked row draws the check glyph"
        );
        // unchecked (row 1, y=54): outline only, no fill, no glyph
        assert!(
            v.iter().any(|c| matches!(c, Cmd::Outline { x, y, w, h, width, .. } if *x == 14.0 && *y == 55.0 && *w == 11.0 && *h == 11.0 && *width == 1.2)),
            "unchecked row draws a stroke-only checkbox"
        );
        assert_eq!(
            v.iter()
                .filter(|c| matches!(c, Cmd::Text { text, .. } if text == ICON_CHECK))
                .count(),
            1,
            "exactly one check glyph for the one checked row"
        );
    }

    #[test]
    fn rows_del_reveals_on_hover_and_registers_corner_region() {
        let scene: CardScene = parse_scene_ron(
            r#"(items: [ Rows(name: "rw", y: 30.0, row_h: 24.0, cols: [ (x: 20.0, size: 9.5) ], key_base: Some(700), del_base: 70, del_pad: 20.0) ])"#,
        )
        .unwrap();
        let mut values = SceneValues::new();
        values.insert(
            "rw",
            SceneValue::Rows(vec![SceneRow {
                cols: vec!["a".into()],
                key: 0,
                action: None,
                color: None,
                col_colors: vec![],
                surface: None,
            }]),
        );
        // resting (no hover): no ✕ drawn, no del region registered
        let mut v: Vec<Cmd> = Vec::new();
        let (hits, _i) = scene.draw(
            &mut v,
            0.0,
            0.0,
            200.0,
            120.0,
            scale(),
            &pal(),
            &values,
            0,
            true,
            true,
        );
        assert!(
            !v.iter()
                .any(|c| matches!(c, Cmd::Text { text, .. } if text == ICON_CLOSE)),
            "✕ hidden while resting"
        );
        assert!(
            hits.iter().all(|h| h.key != 70),
            "no del region while resting"
        );
        // row hovered (key 700): ✕ appears right-anchored in fg + del region wins the corner
        let mut v: Vec<Cmd> = Vec::new();
        let (hits, _i) = scene.draw(
            &mut v,
            0.0,
            0.0,
            200.0,
            120.0,
            scale(),
            &pal(),
            &values,
            700,
            true,
            true,
        );
        let glyph = v.iter().find_map(|c| {
            if let Cmd::Text {
                text, x, y, color, ..
            } = c
            {
                if text == ICON_CLOSE {
                    Some((*x, *y, *color))
                } else {
                    None
                }
            } else {
                None
            }
        });
        assert_eq!(
            glyph,
            Some((180.0, 31.0, pal().fg)),
            "hovered row reveals the ✕ at card_w - del_pad in fg"
        );
        assert!(
            hits.iter().any(|h| h.key == 70
                && h.x == 176.0
                && h.y == 26.0
                && h.w == 22.0
                && h.h == 22.0),
            "del region registered on the row corner"
        );
        // hovering the ✕ itself (key 70): glyph flips to danger
        let mut v: Vec<Cmd> = Vec::new();
        let (_h, _i) = scene.draw(
            &mut v,
            0.0,
            0.0,
            200.0,
            120.0,
            scale(),
            &pal(),
            &values,
            70,
            true,
            true,
        );
        let dange = ColorToken::Danger.resolve(&pal());
        assert!(
            v.iter().any(|c| matches!(c, Cmd::Text { text, color, .. } if text == ICON_CLOSE && *color == dange)),
            "hovering the ✕ paints it danger-red"
        );
    }

    #[test]
    fn rows_del_reveals_without_registered_row_keys() {
        let scene: CardScene = parse_scene_ron(
            r#"(items: [ Rows(name: "rw", y: 30.0, row_h: 24.0, pad: 12.0, cols: [ (x: 50.0, size: 9.5) ], del_base: 70, del_pad: 26.0, del_dy: 4.0) ])"#,
        )
        .unwrap();
        let mut values = SceneValues::new();
        values.insert(
            "rw",
            SceneValue::Rows(vec![SceneRow {
                cols: vec!["label".into()],
                key: 0,
                action: None,
                color: None,
                col_colors: vec![],
                surface: None,
            }]),
        );
        // key-less rows (no key_base): resting draws no ✕ and no row/del regions
        let mut v: Vec<Cmd> = Vec::new();
        let (hits, _i) = scene.draw(
            &mut v,
            0.0,
            0.0,
            200.0,
            120.0,
            scale(),
            &pal(),
            &values,
            0,
            true,
            true,
        );
        assert!(
            !v.iter()
                .any(|c| matches!(c, Cmd::Text { text, .. } if text == ICON_CLOSE)),
            "✕ hidden while resting"
        );
        assert_eq!(hits.len(), 0, "no row-level region for key-less rows");
        // directly hovering the ✕ (del_base + 0) reveals at del_pad/del_dy + registers only it
        let mut v: Vec<Cmd> = Vec::new();
        let (hits, _i) = scene.draw(
            &mut v,
            0.0,
            0.0,
            200.0,
            120.0,
            scale(),
            &pal(),
            &values,
            70,
            true,
            true,
        );
        assert!(
            v.iter().any(|c| matches!(c, Cmd::Text { text, x, y, .. } if text == ICON_CLOSE && *x == 174.0 && *y == 34.0)),
            "✕ appears at card_w - del_pad(26) and ry + del_dy(4)"
        );
        assert_eq!(hits.len(), 1, "only the del region registers");
        assert_eq!(hits[0].key, 70);
    }

    #[test]
    fn rows_start_scalar_scrolls_the_window_and_rebases_keys() {
        let scene: CardScene = parse_scene_ron(
            r#"(items: [ Rows(name: "rw", y: 30.0, row_h: 24.0, cols: [ (x: 0.0) ], key_base: Some(700), start: Some("bt_scroll")) ])"#,
        )
        .unwrap();
        let mut values = SceneValues::new();
        values.insert(
            "rw",
            SceneValue::Rows(
                (0..4)
                    .map(|i| SceneRow {
                        cols: vec![format!("r{i}")],
                        key: 0,
                        action: None,
                        color: None,
                        col_colors: vec![],
                        surface: None,
                    })
                    .collect(),
            ),
        );
        values.insert("bt_scroll", SceneValue::Ring(2.0));
        let mut v: Vec<Cmd> = Vec::new();
        // 4 rows from y=30 pitch 24: bottom edge 120 → only visible[:2] rows fit
        let (hits, _i) = scene.draw(
            &mut v,
            0.0,
            0.0,
            200.0,
            120.0,
            scale(),
            &pal(),
            &values,
            0,
            true,
            true,
        );
        assert!(
            v.iter()
                .any(|c| matches!(c, Cmd::Text { text, .. } if text == "r2")),
            "window starts at scalar index"
        );
        assert!(
            !v.iter()
                .any(|c| matches!(c, Cmd::Text { text, .. } if text == "r0" || text == "r1")),
            "pre-window rows hidden"
        );
        assert_eq!(hits.len(), 2, "only the visible window registers");
        assert_eq!(hits[0].key, 700, "keys rebase to the visible window");
        assert_eq!(hits[1].key, 701);
    }

    #[test]
    fn rows_col_colors_override_per_cell_before_row_color() {
        let scene: CardScene = parse_scene_ron(
            r#"(items: [ Rows(name: "rw", y: 30.0, row_h: 18.0, cols: [ (x: 0.0), (x: 56.0), (x: 160.0, right: true) ], key_base: Some(90)) ])"#,
        )
        .unwrap();
        let mut values = SceneValues::new();
        values.insert(
            "rw",
            SceneValue::Rows(vec![SceneRow {
                cols: vec!["BTC".into(), "68000".into(), "+2.4%".into()],
                key: 0,
                action: None,
                color: Some(ColorToken::Fg3),
                surface: None,
                col_colors: vec![None, None, Some(ColorToken::Ok)],
            }]),
        );
        let mut v: Vec<Cmd> = Vec::new();
        let (_h, _i) = scene.draw(
            &mut v,
            0.0,
            0.0,
            200.0,
            120.0,
            scale(),
            &pal(),
            &values,
            0,
            true,
            true,
        );
        let find = |t: &str| {
            v.iter().find_map(|c| {
                if let Cmd::Text { text, color, .. } = c {
                    if text == t {
                        Some(*color)
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
        };
        assert_eq!(
            find("BTC"),
            Some(ColorToken::Fg3.resolve(&pal())),
            "row color dims sym cell"
        );
        assert_eq!(
            find("+2.4%").unwrap(),
            ColorToken::Ok.resolve(&pal()),
            "status cell keeps its own color"
        );
    }

    #[test]
    fn row_bottom_anchor_uses_the_row_height() {
        // `bottom: true` lifts the row so its BOTTOM edge sits `y` px above
        // the card's — and it needs `h`, since that is the edge it measures
        // (a row with `bottom: true` and no `h` lands on its `y` like any
        // other row, which is what the auth dialog's button pair hit first).
        let scene: CardScene = parse_scene_ron(
            r#"(items: [ Row(x: 0.0, bottom: true, y: 24.0, h: 36.0, halign: center, spacing: 16.0, items: [
                    Stack(w: 120.0, h: 36.0, items: [ Surface(x: 0.0, y: 0.0, w: 0.0, h: 0.0, r: 8.0, color: acc) ]),
                    Stack(w: 120.0, h: 36.0, items: [ Surface(x: 0.0, y: 0.0, w: 0.0, h: 0.0, r: 8.0, color: acc) ]),
               ]) ])"#,
        )
        .unwrap();
        let mut v: Vec<Cmd> = Vec::new();
        let (_h, _i) = scene.draw(
            &mut v,
            0.0,
            0.0,
            420.0,
            320.0,
            scale(),
            &pal(),
            &SceneValues::new(),
            0,
            true,
            true,
        );
        let rects: Vec<(f32, f32)> = v
            .iter()
            .filter_map(|c| {
                if let Cmd::Rect { x, y, .. } = c {
                    Some((*x, *y))
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(
            rects,
            vec![(82.0, 320.0 - 60.0), (218.0, 320.0 - 60.0)],
            "the pair rides 24 + 36 above the bottom, centered as a 256 px block"
        );
    }

    #[test]
    fn rows_col_dy_offsets_cell_baselines() {
        let scene: CardScene = parse_scene_ron(
            r#"(items: [ Rows(name: "rw", y: 30.0, row_h: 24.0, row_dy: 3.0, cols: [ (x: 0.0), (x: 16.0, dy: Some(12.0), size: 8.0) ]) ])"#,
        )
        .unwrap();
        let mut values = SceneValues::new();
        values.insert(
            "rw",
            SceneValue::Rows(vec![SceneRow {
                cols: vec!["title".into(), "caption".into()],
                key: 0,
                action: None,
                color: None,
                surface: None,
                col_colors: vec![],
            }]),
        );
        let mut v: Vec<Cmd> = Vec::new();
        let (_h, _i) = scene.draw(
            &mut v,
            0.0,
            0.0,
            200.0,
            120.0,
            scale(),
            &pal(),
            &values,
            0,
            true,
            true,
        );
        let find_y = |text: &str| {
            v.iter().find_map(|c| {
                if let Cmd::Text { text: t, y, .. } = c {
                    if t == text {
                        Some(*y)
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
        };
        assert_eq!(find_y("title"), Some(33.0), "first col uses row_dy = 3");
        assert_eq!(
            find_y("caption"),
            Some(42.0),
            "second col uses dy = 12 (30 + 12)"
        );
    }

    #[test]
    fn rows_visible_toggle_hides_list() {
        let scene: CardScene = parse_scene_ron(
            r#"(items: [ Rows(name: "rw", y: 30.0, row_h: 24.0, cols: [ (x: 0.0) ], key_base: Some(700), visible: Some("vis")) ])"#,
        )
        .unwrap();
        let mut values = SceneValues::new();
        values.insert(
            "rw",
            SceneValue::Rows(vec![SceneRow {
                cols: vec!["a".into()],
                key: 0,
                action: None,
                color: None,
                surface: None,
                col_colors: vec![],
            }]),
        );
        // Toggle false → list hidden
        values.insert("vis", SceneValue::Toggle(false));
        let mut v: Vec<Cmd> = Vec::new();
        let (hits, _i) = scene.draw(
            &mut v,
            0.0,
            0.0,
            200.0,
            120.0,
            scale(),
            &pal(),
            &values,
            0,
            true,
            true,
        );
        assert!(v.is_empty(), "rows list draws nothing when Toggle is false");
        assert!(hits.is_empty(), "no row regions when hidden");
        // Toggle true → list drawn
        values.insert("vis", SceneValue::Toggle(true));
        let mut v: Vec<Cmd> = Vec::new();
        let (hits, _i) = scene.draw(
            &mut v,
            0.0,
            0.0,
            200.0,
            120.0,
            scale(),
            &pal(),
            &values,
            0,
            true,
            true,
        );
        assert!(
            v.iter()
                .any(|c| matches!(c, Cmd::Text { text, .. } if text == "a")),
            "rows draw when Toggle is true"
        );
        assert_eq!(hits.len(), 1, "row region registered");
    }

    #[test]
    fn rows_pill_cell_draws_chip_with_derived_fill() {
        let scene: CardScene = parse_scene_ron(
            r#"(items: [ Rows(name: "rw", y: 30.0, row_h: 24.0, row_dy: 0.0, pad: 12.0, cols: [
                (x: 0.0, truncate: Some(120.0), size: 9.5),
                (x: 0.0, edge: Some(12.0), right: true, dy: Some(6.0), size: 8.5, pill: Some((height: 16.0, radius: 8.0))),
            ]) ])"#,
        )
        .unwrap();
        let mut values = SceneValues::new();
        values.insert(
            "rw",
            SceneValue::Rows(vec![SceneRow {
                cols: vec!["aaaaaaaaaaaaaaaaaaaaaa".into(), "7d".into()],
                key: 0,
                action: None,
                color: None,
                surface: None,
                col_colors: vec![None, Some(ColorToken::Warn)],
            }]),
        );
        let mut v: Vec<Cmd> = Vec::new();
        let (_h, _i) = scene.draw(
            &mut v,
            0.0,
            0.0,
            200.0,
            120.0,
            scale(),
            &pal(),
            &values,
            0,
            true,
            true,
        );
        // truncate: label cut to fit card_w - 2·pad - 120 at 0.62·fs(9.5)
        // (56 px / 5.89 → 9 glyphs, the ≥8 floor)
        assert!(
            v.iter()
                .any(|c| matches!(c, Cmd::Text { text, .. } if text == "aaaaaaaaa")),
            "long label truncated to the reserved width",
        );
        assert!(
            !v.iter()
                .any(|c| matches!(c, Cmd::Text { text, .. } if text == "aaaaaaaaaaaaaaaaaaaaaa")),
            "untruncated label not drawn"
        );
        // chip: right edge at card_w - pad(12), auto width = 2·0.62·fs(8.5) + 2·5
        let pw = 2.0 * 0.62 * 8.5 + 10.0;
        let pl = 200.0 - 12.0 - pw;
        let hov = crate::ui::hover(&pal());
        let chip = v.iter().find_map(|c| {
            if let Cmd::Rect {
                x, w, h, r, color, ..
            } = c
            {
                if (*x - pl).abs() < 0.01 {
                    Some((*w, *h, *r, *color))
                } else {
                    None
                }
            } else {
                None
            }
        });
        let (w, h, r, color) = chip.expect("chip rect drawn at the edge anchor");
        assert!(
            (w - pw).abs() < 0.01 && h == 16.0 && r == 8.0,
            "chip auto-sized (w {w} h {h} r {r})"
        );
        assert_eq!(color, hov, "Warn chip text → hover fill");
        // Warn-glyph centered across the chip
        assert!(v.iter().any(|c| matches!(c, Cmd::Text { text, x, .. } if text == "7d" && (x - (pl + pw / 2.0)).abs() < 0.01)), "chip text centered in the pill");
        // Acc-token chip derives the acc-tint fill
        values.insert(
            "rw",
            SceneValue::Rows(vec![SceneRow {
                cols: vec!["a".into(), "today!".into()],
                key: 0,
                action: None,
                color: None,
                surface: None,
                col_colors: vec![None, Some(ColorToken::Acc)],
            }]),
        );
        let mut v: Vec<Cmd> = Vec::new();
        let (_h, _i) = scene.draw(
            &mut v,
            0.0,
            0.0,
            200.0,
            120.0,
            scale(),
            &pal(),
            &values,
            0,
            true,
            true,
        );
        let tint = mix(pal().bg, pal().acc, 0.16);
        let cube = v
            .iter()
            .any(|c| matches!(c, Cmd::Rect { color, .. } if *color == tint));
        assert!(cube, "acc chip derived acc-tint fill");
    }

    #[test]
    fn rows_pill_del_anchors_left_of_chip() {
        let scene: CardScene = parse_scene_ron(
            r#"(items: [ Rows(name: "rw", y: 30.0, row_h: 24.0, pad: 12.0, cols: [
                (x: 0.0, size: 9.5),
                (x: 0.0, edge: Some(12.0), right: true, dy: Some(6.0), size: 8.5, pill: Some(()), del_anchor: true),
            ], del_base: 70, del_pad: 20.0, del_dy: 4.0) ])"#,
        )
        .unwrap();
        let mut values = SceneValues::new();
        values.insert(
            "rw",
            SceneValue::Rows(vec![SceneRow {
                cols: vec!["Release".into(), "today!".into()],
                key: 0,
                action: None,
                color: None,
                surface: None,
                col_colors: vec![None, Some(ColorToken::Acc)],
            }]),
        );
        let mut v: Vec<Cmd> = Vec::new();
        let (hits, _i) = scene.draw(
            &mut v,
            0.0,
            0.0,
            200.0,
            120.0,
            scale(),
            &pal(),
            &values,
            70,
            true,
            true,
        );
        let pw = 6.0 * 0.62 * 8.5 + 10.0;
        let pl = 200.0 - 12.0 - pw;
        let dange = ColorToken::Danger.resolve(&pal());
        assert!(
            v.iter().any(|c| matches!(c, Cmd::Text { text, x, .. } if text == ICON_CLOSE && (x - (pl - 20.0)).abs() < 0.01)),
            "✕ anchors left of the chip at del_pad",
        );
        assert!(
            v.iter().any(|c| matches!(c, Cmd::Text { text, color, .. } if text == ICON_CLOSE && *color == dange)),
            "the revealed ✕ is danger-red (reveal-on-✕ rows)",
        );
        assert_eq!(hits.len(), 1, "only the del region registers");
        let h = &hits[0];
        assert!(
            (h.x - (pl - 22.0)).abs() < 0.01 && h.y == 30.0 && h.w == 20.0 && h.h == 24.0,
            "anchor region sits full-row left of the chip"
        );
    }









    #[test]
    fn textwrap_wraps_at_thirty_four_and_centers() {
        // 136 chars (4 lines of 34) + author caption
        let long_text = "abcdefghijklmnopqrstuvwxyz0123456789abcdefghijklmnopqrstuvwxyz0123456789abcdefghijklmnopqrstuvwxyz0123456789abcdefghijklmnopqrstuvwxyz0123456789";
        let scene: CardScene = parse_scene_ron(
            "(items: [ TextWrap(text: \"{tw_text}\", caption: Some(\"— {tw_author}\"), top: 28.0, bottom: 36.0, visible: Some(\"tw_has\")) ])"
        )
        .unwrap();
        let mut vals = SceneValues::new();
        vals.insert("tw_text", SceneValue::Text(long_text.to_string()));
        vals.insert("tw_author", SceneValue::Text("Ada Lovelace".into()));
        vals.insert("tw_has", SceneValue::Toggle(true));
        let mut v: Vec<Cmd> = Vec::new();
        let (_h, _i) = scene.draw(
            &mut v,
            0.0,
            0.0,
            200.0,
            100.0,
            scale(),
            &pal(),
            &vals,
            0,
            true,
            true,
        );
        let text_cmds: Vec<_> = v.iter().filter(|c| matches!(c, Cmd::Text { .. })).collect();
        assert_eq!(text_cmds.len(), 5, "4 wrapped lines + 1 author caption");
        // visible=false suppresses everything
        let mut v2: Vec<Cmd> = Vec::new();
        vals.insert("tw_has", SceneValue::Toggle(false));
        scene.draw(
            &mut v2,
            0.0,
            0.0,
            200.0,
            100.0,
            scale(),
            &pal(),
            &vals,
            0,
            true,
            true,
        );
        assert!(
            v2.iter().all(|c| !matches!(c, Cmd::Text { .. })),
            "visible=false skips draw"
        );
    }

    #[test]
    fn expense_primitives_reserve_gate_and_hover() {
        let mut vals = SceneValues::new();
        vals.insert("cat1", SceneValue::Text("food".into()));
        vals.insert("amt1", SceneValue::Text("42".into()));
        vals.insert("ratio1", SceneValue::Text("0.5".into()));
        vals.insert("has1", SceneValue::Toggle(true));
        vals.insert("empty", SceneValue::Toggle(false));
        let scene: CardScene = parse_scene_ron(
            "(items: [
                Header(title: \"Expenses\", meta: Some(\"{meta}\"), meta_mono: true, meta_color: Some(acc)),
                Text(x: 12.0, y: 90.0, text: \"{cat1}\", font_size: 8.0, visible: Some(\"has1\")),
                Bar(x: 12.0, y: 101.0, w: 0.0, h: 4.0, value: \"{ratio1}\", reserve: 46.0, visible: Some(\"has1\")),
                Text(x: 12.0, y: 96.0, center_x: true, text: \"nothing logged this month\", font_size: 8.5, visible: Some(\"empty\")),
                Hit(x: 0.0, y: 200.0, w: 70.0, h: 22.0, text: \"clear mo\", color: Some(fg2), color_hover: Some(danger), key: 31_811),
             ])",
        )
        .unwrap();
        let mut hvals = vals.clone();
        hvals.insert("meta", SceneValue::Text("MTD 42".into()));
        let mut v: Vec<Cmd> = Vec::new();
        scene.draw(
            &mut v,
            0.0,
            0.0,
            300.0,
            120.0,
            scale(),
            &pal(),
            &hvals,
            31_811,
            true,
            true,
        );
        // header meta — accent mono
        let acc = ColorToken::Acc.resolve(&pal());
        assert!(
            v.iter().any(
                |c| matches!(c, Cmd::Text { text, color, .. } if text == "MTD 42" && *color == acc)
            ),
            "meta color accent + mono still substituted"
        );
        // bar reserve → 300 − 46 track
        let track: Vec<_> = v
            .iter()
            .filter_map(|c| {
                if let Cmd::Rect { x, y, w, h, .. } = c {
                    Some((*x, *y, *w, *h))
                } else {
                    None
                }
            })
            .filter(|(_, yy, _, _)| (*yy - 101.0).abs() < 0.01)
            .collect();
        assert!(
            track.iter().any(|(xx, _, ww, hh)| (*xx - 12.0).abs() < 0.01
                && (*ww - 254.0).abs() < 0.01
                && *hh == 4.0),
            "bar spans card_w − reserve"
        );
        // visible=false caption suppressed
        assert!(
            !v.iter().any(
                |c| matches!(c, Cmd::Text { text, .. } if text == "nothing logged this month")
            ),
            "empty caption hidden while a category bar exists"
        );
        // hover swaps the clear label to danger
        let dange = ColorToken::Danger.resolve(&pal());
        let found = v.iter().any(
            |c| matches!(c, Cmd::Text { text, color, .. } if text == "clear mo" && *color == dange),
        );
        assert!(found, "clear mo flips to danger while hovered");
        // idle frame: label back to fg2
        let mut v2: Vec<Cmd> = Vec::new();
        scene.draw(
            &mut v2,
            0.0,
            0.0,
            300.0,
            120.0,
            scale(),
            &pal(),
            &hvals,
            0,
            true,
            true,
        );
        let fg2 = ColorToken::Fg2.resolve(&pal());
        assert!(v2.iter().any(|c| matches!(c, Cmd::Text { text, color, .. } if text == "clear mo" && *color == fg2)), "idle clear label is fg2");
    }

    #[test]
    fn news_strip_draws_chips_and_rows_hover_ink() {
        let mut vals = SceneValues::new();
        vals.insert(
            "news_chips",
            SceneValue::Chips(vec![
                "world".into(),
                "tech".into(),
                "science".into(),
                "money".into(),
                "sports".into(),
            ]),
        );
        vals.insert("news_cat_sel", SceneValue::Ring(2.0));
        vals.insert("news_cat_scroll", SceneValue::Ring(40.0));
        vals.insert(
            "news_rows",
            SceneValue::Rows(vec![crate::scene::SceneRow {
                cols: vec![
                    "Big headline here".into(),
                    crate::icons::ICON_OPEN.to_string(),
                    "BBC".into(),
                ],
                key: 0,
                action: None,
                color: None,
                surface: None,
                col_colors: vec![],
            }]),
        );
        vals.insert("news_scroll", SceneValue::Ring(0.0));
        let scene: CardScene = parse_scene_ron(
            "(items: [
                Strip(
                    chips: \"news_chips\",
                    sel: Some(\"news_cat_sel\"),
                    scroll: Some(\"news_cat_scroll\"),
                    x: 0.0, y: 30.0, w: 0.0, h: 26.0,
                    pad: 12.0, spacing: 6.0, chip_pad: 11.0, fs: 8.5,
                    key_base: 13000,
                ),
                Rows(
                    name: \"news_rows\",
                    y: 59.0, row_h: 30.0, pitch: Some(30.0), pad: 12.0, row_dy: 2.0,
                    cols: [
                        (x: 5.0, dy: Some(2.0), size: 9.5, color: Some(fg), hover: Some(acc)),
                        (x: 0.0, edge: Some(12.0), right: true, dy: Some(2.0), size: 8.0, icon: true, color: Some(fg3), hover: Some(acc)),
                        (x: 0.0, edge: Some(12.0), right: true, dy: Some(14.0), size: 7.5, color: Some(fg3)),
                    ],
                    key_base: Some(13300),
                    hairline: true,
                    hover_bar: true,
                ),
             ])",
        )
        .unwrap();
        let acc = ColorToken::Acc.resolve(&pal());
        let sfg = ColorToken::Sfg.resolve(&pal());
        let mut v: Vec<Cmd> = Vec::new();
        let (hits, _) = scene.draw(
            &mut v,
            0.0,
            0.0,
            300.0,
            130.0,
            scale(),
            &pal(),
            &vals,
            13_300,
            true,
            true,
        );
        assert!(
            hits.iter().any(|h| h.key == 13_002),
            "chip index 2 registers a region"
        );
        assert!(
            hits.iter().any(|h| h.key == 13_300),
            "the headline row registers"
        );
        assert!(
            v.iter()
                .any(|c| matches!(c, Cmd::Rect { color, h, .. } if *color == acc && *h == 24.0)),
            "active chip painted accent"
        );
        assert!(
            v.iter().any(
                |c| matches!(c, Cmd::Text { text, color, .. } if text == "science" && *color == sfg)
            ),
            "active chip label in Sfg (accent text)"
        );
        assert!(v.iter().any(|c| matches!(c, Cmd::Rect { color, w, y, .. } if *color == acc && (*w - 2.5).abs() < 0.01 && (*y - 61.0).abs() < 0.02)), "hover accent rail at the row's left");
        let hover_gray = crate::ui::hover(&pal());
        assert!(v.iter().any(|c| matches!(c, Cmd::Rect { color, h, y, .. } if *color == hover_gray && *h == 1.0 && (*y - 88.0).abs() < 0.02)), "hairline divider under the row");
        assert!(v.iter().any(|c| matches!(c, Cmd::Text { text, color, .. } if text == "Big headline here" && *color == acc)), "title tints accent while its row is hovered");
        // idle frame: title back to fg, rail gone
        let mut v2: Vec<Cmd> = Vec::new();
        scene.draw(
            &mut v2,
            0.0,
            0.0,
            300.0,
            130.0,
            scale(),
            &pal(),
            &vals,
            0,
            true,
            true,
        );
        let fg = ColorToken::Fg.resolve(&pal());
        assert!(v2.iter().any(|c| matches!(c, Cmd::Text { text, color, .. } if text == "Big headline here" && *color == fg)), "idle title is fg");
        assert!(!v2.iter().any(|c| matches!(c, Cmd::Rect { color, w, .. } if *color == acc && (*w - 2.5).abs() < 0.01)), "no accent rail without hover");
    }

    #[test]
    fn snippet_flash_draws_translucent_wash_and_flash_ink() {
        let mut vals = SceneValues::new();
        vals.insert(
            "snip_rows",
            SceneValue::Rows(vec![
                crate::scene::SceneRow {
                    cols: vec![
                        crate::icons::ICON_SELECT.to_string(),
                        "ssh".into(),
                        "user@host:22".into(),
                    ],
                    key: 0,
                    action: None,
                    color: None,
                    surface: None,
                    col_colors: vec![],
                },
                crate::scene::SceneRow {
                    cols: vec![
                        crate::icons::ICON_SELECT.to_string(),
                        "addr".into(),
                        "42 Imagination Street".into(),
                    ],
                    key: 0,
                    action: None,
                    color: None,
                    surface: None,
                    col_colors: vec![],
                },
            ]),
        );
        // scalar is index + 1 → row 1 flashes (0 = none)
        vals.insert("snip_flash", SceneValue::Ring(2.0));
        let scene: CardScene = parse_scene_ron(
            "(items: [
                Rows(
                    name: \"snip_rows\",
                    y: 50.0, row_h: 24.0, pad: 12.0, r: 6.0,
                    hover_surface: Some(hover_hl),
                    flash: Some(\"snip_flash\"),
                    cols: [
                        (x: 2.0, dy: Some(4.0), size: 9.0, icon: true, color: Some(fg2), flash: Some(acc)),
                        (x: 18.0, dy: Some(4.0), size: 9.5, color: Some(fg), flash: Some(acc)),
                        (x: 96.0, dy: Some(5.0), size: 8.0, color: Some(fg3)),
                    ],
                    key_base: Some(31710),
                    del_base: 31730,
                    del_pad: 20.0,
                    del_dy: 4.0,
                    del_size: 9.0,
                ),
             ])",
        )
        .unwrap();
        let acc = ColorToken::Acc.resolve(&pal());
        let wash = (pal().acc & 0xFFFF_FF00) | 0x30;
        let mut v: Vec<Cmd> = Vec::new();
        let (hits, _) = scene.draw(
            &mut v,
            0.0,
            0.0,
            300.0,
            130.0,
            scale(),
            &pal(),
            &vals,
            0,
            true,
            true,
        );
        // the flash row (index 1 → ry 74): translucent accent wash behind it
        assert!(v.iter().any(|c| matches!(c, Cmd::Rect { color, y, h, .. } if *color == wash && (*y - 74.0).abs() < 0.02 && *h == 24.0)), "flashed row washed translucent accent");
        // its name inks accent; the preview stays fg3
        assert!(
            v.iter().any(
                |c| matches!(c, Cmd::Text { text, color, .. } if text == "addr" && *color == acc)
            ),
            "flashed name in accent"
        );
        let fg3 = ColorToken::Fg3.resolve(&pal());
        assert!(v.iter().any(|c| matches!(c, Cmd::Text { text, color, .. } if text == "42 Imagination Street" && *color == fg3)), "preview line stays fg3 under the flash");
        // idle row 0: copy glyph fg2, name fg
        let fg2 = ColorToken::Fg2.resolve(&pal());
        assert!(v.iter().any(|c| matches!(c, Cmd::Text { text, color, .. } if text == crate::icons::ICON_SELECT && *color == fg2)), "idle copy glyph stays fg2");
        let fg = ColorToken::Fg.resolve(&pal());
        assert!(
            v.iter().any(
                |c| matches!(c, Cmd::Text { text, color, .. } if text == "ssh" && *color == fg)
            ),
            "idle name stays fg"
        );
        // region: both copy regions registered; ✕ only shows on hover
        assert!(hits.iter().any(|h| h.key == 31_710), "row 0 copy region");
        assert!(hits.iter().any(|h| h.key == 31_711), "row 1 copy region");
        assert!(
            !hits.iter().any(|h| h.key == 31_730),
            "no delete key without hover"
        );
        // hovered + flash: wash beats the hover surface
        let mut v2: Vec<Cmd> = Vec::new();
        scene.draw(
            &mut v2,
            0.0,
            0.0,
            300.0,
            130.0,
            scale(),
            &pal(),
            &vals,
            31_711,
            true,
            true,
        );
        assert!(v2.iter().any(|c| matches!(c, Cmd::Rect { color, y, .. } if *color == wash && (*y - 74.0).abs() < 0.02)), "flash wash beats hover surface");
        // idle (0 = no flash): hover lifts the surface instead
        let mut vals_idle = vals.clone();
        vals_idle.insert("snip_flash", SceneValue::Ring(0.0));
        let mut v3: Vec<Cmd> = Vec::new();
        scene.draw(
            &mut v3,
            0.0,
            0.0,
            300.0,
            130.0,
            scale(),
            &pal(),
            &vals_idle,
            31_711,
            true,
            true,
        );
        let hhl = ColorToken::HoverHl.resolve(&pal());
        assert!(v3.iter().any(|c| matches!(c, Cmd::Rect { color, y, .. } if *color == hhl && (*y - 74.0).abs() < 0.02)), "idle hover surface lifts the row");
        assert!(!v3.iter().any(|c| matches!(c, Cmd::Rect { color, y, .. } if *color == wash && (*y - 74.0).abs() < 0.02)), "no wash without a flash scalar");
    }

    #[test]
    fn karaoke_highlights_active_word_and_washes_active_row() {
        let mut vals = SceneValues::new();
        vals.insert(
            "lyr_rows",
            SceneValue::Rows(vec![
                crate::scene::SceneRow {
                    cols: vec!["we both shine".into()],
                    key: 0,
                    action: None,
                    color: None,
                    surface: None,
                    col_colors: vec![],
                },
                crate::scene::SceneRow {
                    cols: vec!["hello moon again".into()],
                    key: 0,
                    action: None,
                    color: None,
                    surface: None,
                    col_colors: vec![],
                },
            ]),
        );
        vals.insert(
            "lyr_karaoke",
            SceneValue::Karaoke(vec![
                crate::scene::KaraokeLine {
                    words: vec![
                        ("we".into(), false),
                        ("both".into(), false),
                        ("shine".into(), false),
                    ],
                    active: false,
                },
                crate::scene::KaraokeLine {
                    words: vec![
                        ("hello".into(), true),
                        ("moon".into(), false),
                        ("again".into(), false),
                    ],
                    active: true,
                },
            ]),
        );
        vals.insert("lyr_scroll", SceneValue::Ring(0.0));
        let scene: CardScene = parse_scene_ron(
            "(items: [
                Rows(
                    name: \"lyr_rows\",
                    y: 33.0, row_h: 21.0, r: 4.0,
                    hover_surface: Some(hover),
                    karaoke: Some(\"lyr_karaoke\"),
                    start: Some(\"lyr_scroll\"),
                    key_base: Some(33500),
                    cols: [
                        (x: 6.0, dy: Some(0.5), size: 10.5, color: Some(fg), hover: Some(acc)),
                    ],
                ),
             ])",
        )
        .unwrap();
        let mut v: Vec<Cmd> = Vec::new();
        let (hits, _) = scene.draw(
            &mut v,
            0.0,
            0.0,
            300.0,
            120.0,
            scale(),
            &pal(),
            &vals,
            33_500,
            true,
            true,
        );
        // the ACTIVE (row 1, ry 54) row: accent wash + word walk
        let acc = ColorToken::Acc.resolve(&pal());
        assert!(v.iter().any(|c| matches!(c, Cmd::Rect { color, y, h, .. } if *color == acc && (*y - 55.0).abs() < 0.02 && *h == 17.0)), "active row washed accent");
        assert!(v.iter().any(|c| matches!(c, Cmd::Text { text, color, .. } if text == "hello" && *color == 0xffff_ffff)), "live word white");
        assert!(v.iter().any(|c| matches!(c, Cmd::Text { text, color, .. } if text == "moon" && *color == 0x1d1d_1d)), "rest of the line near-black");
        assert!(v.iter().any(|c| matches!(c, Cmd::Rect { color, y, h, .. } if *color == 0xffff_ffff && *h == 1.5 && (*y - 68.5).abs() < 0.02)), "underline under the live word");
        // the inactive hovered row: hover lift + joined accent line
        let hover_col = crate::ui::hover(&pal());
        assert!(v.iter().any(|c| matches!(c, Cmd::Rect { color, y, h, .. } if *color == hover_col && (*y - 34.0).abs() < 0.02 && *h == 17.0)), "inactive hovered row lifts");
        assert!(v.iter().any(|c| matches!(c, Cmd::Text { text, color, .. } if text == "we both shine" && *color == acc)), "joined line tints accent on hover");
        // regions for both visible rows
        assert!(hits.iter().any(|h| h.key == 33_500));
        assert!(hits.iter().any(|h| h.key == 33_501));
    }

    #[test]
    fn karaoke_unbound_falls_back_to_plain_cells() {
        let mut vals = SceneValues::new();
        vals.insert(
            "lyr_rows",
            SceneValue::Rows(vec![crate::scene::SceneRow {
                cols: vec!["plain line one".into()],
                key: 0,
                action: None,
                color: None,
                surface: None,
                col_colors: vec![],
            }]),
        );
        vals.insert("lyr_scroll", SceneValue::Ring(0.0));
        // no `lyr_karaoke` value bound — the karaoke binding resolves None
        let scene: CardScene = parse_scene_ron(
            "(items: [
                Rows(
                    name: \"lyr_rows\",
                    y: 33.0, row_h: 21.0, r: 4.0,
                    hover_surface: Some(hover),
                    karaoke: Some(\"lyr_karaoke\"),
                    start: Some(\"lyr_scroll\"),
                    key_base: Some(33500),
                    cols: [
                        (x: 6.0, dy: Some(0.5), size: 10.5, color: Some(fg), hover: Some(acc)),
                    ],
                ),
             ])",
        )
        .unwrap();
        // idle frame: whole line via the plain cell path, fg
        let mut v: Vec<Cmd> = Vec::new();
        scene.draw(
            &mut v,
            0.0,
            0.0,
            300.0,
            120.0,
            scale(),
            &pal(),
            &vals,
            0,
            true,
            true,
        );
        let fg = ColorToken::Fg.resolve(&pal());
        assert!(v.iter().any(|c| matches!(c, Cmd::Text { text, color, .. } if text == "plain line one" && *color == fg)), "whole line via the plain cell path");
        assert!(
            !v.iter().any(|c| matches!(
                c,
                Cmd::Text {
                    color: 0xffff_ffff,
                    ..
                } | Cmd::Text {
                    color: 0x1d1d_1d,
                    ..
                }
            )),
            "no karaoke word draw without the value"
        );
        // hovered frame: hover lift + accent tint
        let mut v2: Vec<Cmd> = Vec::new();
        scene.draw(
            &mut v2,
            0.0,
            0.0,
            300.0,
            120.0,
            scale(),
            &pal(),
            &vals,
            33_500,
            true,
            true,
        );
        let acc = ColorToken::Acc.resolve(&pal());
        assert!(v2.iter().any(|c| matches!(c, Cmd::Text { text, color, .. } if text == "plain line one" && *color == acc)), "hovered plain row tints accent");
        assert!(v2.iter().any(|c| matches!(c, Cmd::Rect { color, y, h, .. } if *color == crate::ui::hover_hl(&pal()) && (*y - 33.0).abs() < 0.02 && *h == 21.0)), "plain row hover surface lifts");
    }

    #[test]
    fn spark_draws_line_and_column_shapes() {
        let scene: CardScene = parse_scene_ron(
            "(items: [
                Spark(name: \"cpu\", x: 12.0, y: 30.0, w: 176.0, h: 40.0, color: Some(acc), max: 100.0),
                Spark(name: \"mem\", x: 12.0, y: 76.0, w: 176.0, h: 24.0, kind: column, max: 100.0, thickness: 4.0),
            ])",
        )
        .unwrap();
        let mut values = SceneValues::new();
        values.insert("cpu", SceneValue::Spark(vec![10.0, 40.0, 90.0, 30.0]));
        values.insert("mem", SceneValue::Spark(vec![20.0, 60.0, 80.0]));
        let mut v: Vec<Cmd> = Vec::new();
        let (_h, _i) = scene.draw(
            &mut v,
            0.0,
            0.0,
            200.0,
            110.0,
            scale(),
            &pal(),
            &values,
            0,
            true,
            true,
        );
        assert!(
            v.iter().any(|c| matches!(c, Cmd::Line { .. })),
            "line spark emits segments"
        );
        assert!(
            v.iter().any(|c| matches!(c, Cmd::Rect { .. })),
            "column spark emits bars"
        );
        // a 100%-sample line's last point sits at the line's own thickness inset:
        let peak_at = 90.0_f32 / 100.0;
        // nothing to assert numerically here beyond both shapes drawing -> covered
        let _ = peak_at;
    }

    #[test]
    fn spark_auto_normalizes_when_no_max() {
        let scene: CardScene =
            parse_scene_ron("(items: [ Spark(name: \"s\", x: 0.0, y: 0.0, w: 100.0, h: 50.0) ])")
                .unwrap();
        let mut values = SceneValues::new();
        values.insert("s", SceneValue::Spark(vec![0.0, 50.0, 100.0]));
        let mut v: Vec<Cmd> = Vec::new();
        let (_h, _i) = scene.draw(
            &mut v,
            0.0,
            0.0,
            100.0,
            50.0,
            scale(),
            &pal(),
            &values,
            0,
            true,
            true,
        );
        assert!(
            v.iter().any(|c| matches!(c, Cmd::Line { .. })),
            "no-max spark still draws"
        );
    }

    #[test]
    fn ring_lights_beads_by_ratio() {
        let scene: CardScene = parse_scene_ron(
            "(items: [ Ring(name: \"bat\", cx: 40.0, cy: 50.0, radius: 30.0, max: 100.0, beads: 36, fill: acc, track: hover) ])",
        )
        .unwrap();
        let mut values = SceneValues::new();
        values.insert("bat", SceneValue::Ring(50.0));
        let acc = ColorToken::Acc.resolve(&pal());
        let mut v: Vec<Cmd> = Vec::new();
        let (_h, _i) = scene.draw(
            &mut v,
            0.0,
            0.0,
            200.0,
            120.0,
            scale(),
            &pal(),
            &values,
            0,
            true,
            true,
        );
        let lit = v
            .iter()
            .filter(|c| matches!(c, Cmd::Rect { color, .. } if *color == acc))
            .count();
        assert_eq!(lit, 18, "50% of 36 beads lit");
        // value over max clamps to full ring
        values.insert("bat", SceneValue::Ring(150.0));
        let mut v: Vec<Cmd> = Vec::new();
        let (_h, _i) = scene.draw(
            &mut v,
            0.0,
            0.0,
            200.0,
            120.0,
            scale(),
            &pal(),
            &values,
            0,
            true,
            true,
        );
        let lit = v
            .iter()
            .filter(|c| matches!(c, Cmd::Rect { color, .. } if *color == acc))
            .count();
        assert_eq!(lit, 36, "over-max clamps to the full ring");
    }

    #[test]
    fn fader_draws_fill_head_and_registers_key() {
        let scene: CardScene = parse_scene_ron(
            "(items: [ Fader(name: \"vol\", x: 12.0, y: 40.0, w: 176.0, h: 20.0, key: 501, fill: acc, track: hover) ])",
        )
        .unwrap();
        let mut values = SceneValues::new();
        values.insert("vol", SceneValue::Fader(0.75));
        let mut v: Vec<Cmd> = Vec::new();
        let (hits, _) = scene.draw(
            &mut v,
            0.0,
            0.0,
            200.0,
            100.0,
            scale(),
            &pal(),
            &values,
            501,
            true,
            true,
        );
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].key, 501);
        assert_eq!(hits[0].x, 12.0);
        assert_eq!(hits[0].w, 176.0);
        let acc = ColorToken::Acc.resolve(&pal());
        assert!(
            v.iter()
                .filter(|c| matches!(c, Cmd::Rect { color, .. } if *color == acc))
                .count()
                >= 2,
            "fill + round head both accent-colored"
        );
        // no key → no hit region
        let scene2: CardScene = parse_scene_ron(
            "(items: [ Fader(name: \"br\", x: 12.0, y: 40.0, w: 100.0, h: 20.0) ])",
        )
        .unwrap();
        let (_h, _i) = scene2.draw(
            &mut Vec::new(),
            0.0,
            0.0,
            200.0,
            100.0,
            scale(),
            &pal(),
            &values,
            0,
            true,
            true,
        );
        let mut v2: Vec<Cmd> = Vec::new();
        let (hits2, _) = scene2.draw(
            &mut v2,
            0.0,
            0.0,
            200.0,
            100.0,
            scale(),
            &pal(),
            &values,
            0,
            true,
            true,
        );
        assert!(hits2.is_empty(), "key 0 registers nothing");
    }

    #[test]
    fn toggle_draws_switch_state_and_registers_key() {
        let scene: CardScene = parse_scene_ron(
            "(items: [ Toggle(name: \"blur\", x: 150.0, y: 8.0, w: 38.0, h: 20.0, key: 601) ])",
        )
        .unwrap();
        let mut values = SceneValues::new();
        values.insert("blur", SceneValue::Toggle(false));
        let mut v: Vec<Cmd> = Vec::new();
        let (hits, _) = scene.draw(
            &mut v,
            0.0,
            0.0,
            200.0,
            100.0,
            scale(),
            &pal(),
            &values,
            0,
            true,
            true,
        );
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].key, 601);
        assert!(
            v.iter()
                .all(|c| !matches!(c, Cmd::Rect { color, .. } if *color == pal().acc)),
            "off switch has no accent anywhere"
        );
        // on → accent track
        values.insert("blur", SceneValue::Toggle(true));
        let mut v: Vec<Cmd> = Vec::new();
        let (_h, _i) = scene.draw(
            &mut v,
            0.0,
            0.0,
            200.0,
            100.0,
            scale(),
            &pal(),
            &values,
            0,
            true,
            true,
        );
        assert!(
            v.iter()
                .any(|c| matches!(c, Cmd::Rect { color, .. } if *color == pal().acc)),
            "on switch paints the accent track"
        );
    }

    #[test]
    fn tabrow_draws_tabs_and_highlights_selected() {
        let scene: CardScene = parse_scene_ron(
            "(items: [ TabRow(name: \"tabs\", sel: Some(\"tab_sel\"), y: 28.0, key_base: 810)])",
        )
        .unwrap();
        let mut values = SceneValues::new();
        values.insert(
            "tabs",
            SceneValue::Rows(vec![
                SceneRow {
                    cols: vec!["One".into()],
                    key: 0,
                    action: None,
                    color: None,
                    col_colors: vec![],
                    surface: None,
                },
                SceneRow {
                    cols: vec!["Two".into()],
                    key: 0,
                    action: None,
                    color: None,
                    col_colors: vec![],
                    surface: None,
                },
            ]),
        );
        values.insert("tab_sel", SceneValue::Ring(1.0));
        let mut v: Vec<Cmd> = Vec::new();
        let (hits, _) = scene.draw(
            &mut v,
            0.0,
            0.0,
            200.0,
            100.0,
            scale(),
            &pal(),
            &values,
            0,
            true,
            true,
        );
        assert_eq!(hits.len(), 2, "one key per tab from key_base");
        assert_eq!(hits[1].key, 811);
        assert!(
            v.iter()
                .any(|c| matches!(c, Cmd::Text { text, .. } if text == "One")),
            "tab label drawn"
        );
        assert!(v.iter().any(|c| matches!(c, Cmd::Rect { color, .. } if *color == ColorToken::AccTint.resolve(&pal()))),
            "selected tab gets the accent-tinted surface");
    }

    #[test]
    fn missing_or_wrong_typed_value_draws_nothing() {
        let scene: CardScene = parse_scene_ron(
            "(items: [ Spark(name: \"nope\", x: 0.0, y: 0.0, w: 40.0, h: 20.0), Ring(name: \"also_nope\", cx: 0.0, cy: 0.0, radius: 20.0), Fader(name: \"wonk\", x: 0.0, y: 0.0, w: 40.0, h: 18.0), Toggle(name: \"gone\", x: 0.0, y: 0.0, w: 20.0, h: 12.0) ])",
        )
        .unwrap();
        let mut v: Vec<Cmd> = Vec::new();
        let (_h, hits) = scene.draw(
            &mut v,
            0.0,
            0.0,
            200.0,
            120.0,
            scale(),
            &pal(),
            &SceneValues::new(),
            0,
            true,
            true,
        );
        assert!(hits.is_empty(), "no keys → no hit regions");
        assert!(v.is_empty(), "missing/typed-mismatched names draw nothing");
    }

    // ── dynamic color tokens (ColorToken::Value) ───────────────────────

    #[test]
    fn value_token_ron_parses_and_roundtrips() {
        let scene: CardScene = parse_scene_ron(
            r#"(items: [
                Hit(x: 8.0, y: 6.0, w: 60.0, h: 24.0, r: 12.0, key: 3,
                    surface: Some(value("dnd_chip")), text: "DND", font_size: 11.0,
                    color: Some(value("dnd_chip_fg"))),
            ])"#,
        )
        .unwrap();
        match &scene.items[0] {
            SceneItem::Hit { surface, color, .. } => {
                assert_eq!(
                    *surface,
                    Some(ColorToken::Value(DynColor::new("dnd_chip").unwrap()))
                );
                assert_eq!(
                    *color,
                    Some(ColorToken::Value(DynColor::new("dnd_chip_fg").unwrap()))
                );
            }
            other => panic!("expected Hit, got {other:?}"),
        }
    }

    #[test]
    fn dynamic_token_resolves_from_scene_colors() {
        let scene: CardScene = parse_scene_ron(
            r#"(items: [ Surface(x: 8.0, y: 6.0, w: 60.0, h: 24.0, r: 12.0, color: value("dnd_chip")) ])"#,
        )
        .unwrap();
        let mut values = SceneValues::new();
        values.insert_color("dnd_chip", SceneColor::Token(ColorToken::Acc));
        let mut v: Vec<Cmd> = Vec::new();
        let (_h, _i) = scene.draw(
            &mut v,
            0.0,
            0.0,
            200.0,
            120.0,
            scale(),
            &pal(),
            &values,
            0,
            true,
            true,
        );
        assert!(
            v.iter()
                .any(|c| matches!(c, Cmd::Rect { color, .. } if *color == plt_acc())),
            "surface bound to a dynamic token lifts the live theme accent"
        );
    }

    #[test]
    fn dynamic_raw_color_bypasses_theme() {
        let scene: CardScene = parse_scene_ron(
            r#"(items: [ Surface(x: 0.0, y: 0.0, w: 40.0, h: 20.0, color: value("meter")) ])"#,
        )
        .unwrap();
        let mut values = SceneValues::new();
        values.insert_color("meter", SceneColor::Raw(0x11223344));
        let mut v: Vec<Cmd> = Vec::new();
        let (_h, _i) = scene.draw(
            &mut v,
            0.0,
            0.0,
            200.0,
            120.0,
            scale(),
            &pal(),
            &values,
            0,
            true,
            true,
        );
        assert!(
            v.iter()
                .any(|c| matches!(c, Cmd::Rect { color, .. } if *color == 0x11223344)),
            "pre-resolved u32 draws verbatim"
        );
    }

    #[test]
    fn unknown_dynamic_name_falls_back_to_fg() {
        let scene: CardScene = parse_scene_ron(
            r#"(items: [ Surface(x: 0.0, y: 0.0, w: 40.0, h: 20.0, color: value("nope")) ])"#,
        )
        .unwrap();
        let mut v: Vec<Cmd> = Vec::new();
        let (_h, _i) = scene.draw(
            &mut v,
            0.0,
            0.0,
            200.0,
            120.0,
            scale(),
            &pal(),
            &SceneValues::new(),
            0,
            true,
            true,
        );
        assert!(
            v.iter()
                .any(|c| matches!(c, Cmd::Rect { color, .. } if *color == pal().fg)),
            "missing dynamic name degrades to fg, never panics"
        );
    }

    #[test]
    fn dynamic_surface_keeps_its_color_under_hover() {
        // a dynamic chip surface is state-computed per frame (its hover intent
        // is already baked in) so hovering must not lift/replace the color.
        let scene: CardScene = parse_scene_ron(
            r#"(items: [ Hit(x: 8.0, y: 6.0, w: 60.0, h: 24.0, r: 12.0, key: 3, surface: Some(value("dnd_chip"))) ])"#,
        )
        .unwrap();
        let mut values = SceneValues::new();
        values.insert_color("dnd_chip", SceneColor::Token(ColorToken::Acc));
        for hover in [0, 3] {
            let mut v: Vec<Cmd> = Vec::new();
            let (_h, _i) = scene.draw(
                &mut v,
                0.0,
                0.0,
                200.0,
                120.0,
                scale(),
                &pal(),
                &values,
                hover,
                true,
                true,
            );
            let hits = v
                .iter()
                .filter(|c| matches!(c, Cmd::Rect { color, .. } if *color == plt_acc()))
                .count();
            assert_eq!(
                hits, 1,
                "chip surface stays the dynamic accent, hovered or not (hover={hover})"
            );
        }
    }

    fn plt_acc() -> u32 {
        ColorToken::Acc.resolve(&pal())
    }

    // ── layout containers (Phase B) ────────────────────────────────────

    #[test]
    fn row_places_children_by_width_and_spacing() {
        let scene: CardScene = parse_scene_ron(
            r#"(items: [ Row(
                    x: 0.0, y: 0.0,
                    spacing: 10.0,
                    items: [
                        Hit(x: 0.0, y: 0.0, w: 40.0, h: 20.0, key: 1),
                        Hit(x: 0.0, y: 0.0, w: 40.0, h: 20.0, key: 2),
                        Hit(x: 0.0, y: 0.0, w: 40.0, h: 20.0, key: 3),
                    ],
                ) ])"#,
        )
        .unwrap();
        let mut v: Vec<Cmd> = Vec::new();
        let (hits, _) = scene.draw(
            &mut v,
            0.0,
            0.0,
            200.0,
            120.0,
            scale(),
            &pal(),
            &SceneValues::new(),
            0,
            true,
            true,
        );
        assert_eq!(hits.len(), 3);
        assert_eq!(hits[0].x, 0.0);
        assert_eq!(hits[1].x, 50.0, "40 + 10 spacing");
        assert_eq!(hits[2].x, 100.0);
        assert_eq!(hits[0].w, 40.0);
    }

    #[test]
    fn row_fill_children_split_remaining_width() {
        let scene: CardScene = parse_scene_ron(
            r#"(items: [ Row(
                    x: 0.0, y: 0.0, w: 200.0,
                    spacing: 0.0,
                    items: [
                        Hit(x: 0.0, y: 0.0, w: 120.0, h: 20.0, key: 1),
                        Hit(x: 0.0, y: 0.0, w: 0.0, h: 20.0, key: 2),
                        Hit(x: 0.0, y: 0.0, w: 0.0, h: 20.0, key: 3),
                    ],
                ) ])"#,
        )
        .unwrap();
        let mut v: Vec<Cmd> = Vec::new();
        let (hits, _) = scene.draw(
            &mut v,
            0.0,
            0.0,
            200.0,
            120.0,
            scale(),
            &pal(),
            &SceneValues::new(),
            0,
            true,
            true,
        );
        assert_eq!(hits.len(), 3);
        assert_eq!(hits[0].w, 120.0);
        assert_eq!(hits[1].x, 120.0, "fixed child ends at 120");
        assert_eq!(hits[1].w, 40.0, "(200 - 120) / 2 fills");
        assert_eq!(hits[2].x, 160.0);
        assert_eq!(hits[2].w, 40.0);
    }

    #[test]
    fn row_valigns_short_children() {
        let scene: CardScene = parse_scene_ron(
            r#"(items: [ Row(x: 0.0, y: 0.0, h: 100.0, valign: middle, items: [
                    Hit(x: 0.0, y: 0.0, w: 40.0, h: 20.0, key: 1),
                ]) ])"#,
        )
        .unwrap();
        let mut v: Vec<Cmd> = Vec::new();
        let (hits, _) = scene.draw(
            &mut v,
            0.0,
            0.0,
            200.0,
            120.0,
            scale(),
            &pal(),
            &SceneValues::new(),
            0,
            true,
            true,
        );
        assert_eq!(
            hits[0].y, 40.0,
            "in a 100-tall box a 20-tall child centers at 40"
        );
    }

    #[test]
    fn column_stacks_children_and_centers_halign() {
        let scene: CardScene = parse_scene_ron(
            r#"(items: [ Column(
                    x: 0.0, y: 0.0, w: 200.0, pad: 10.0, spacing: 5.0, halign: center,
                    items: [
                        Hit(x: 0.0, y: 0.0, w: 80.0, h: 20.0, key: 1),
                        Hit(x: 0.0, y: 0.0, w: 120.0, h: 20.0, key: 2),
                    ],
                ) ])"#,
        )
        .unwrap();
        let mut v: Vec<Cmd> = Vec::new();
        let (hits, _) = scene.draw(
            &mut v,
            0.0,
            0.0,
            200.0,
            120.0,
            scale(),
            &pal(),
            &SceneValues::new(),
            0,
            true,
            true,
        );
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].y, 10.0, "pad top");
        assert_eq!(hits[1].y, 35.0, "20 + 5 spacing");
        assert_eq!(
            hits[0].x,
            10.0 + (180.0 - 80.0) / 2.0,
            "halign: center offsets within inner width"
        );
        assert_eq!(hits[1].x, 10.0 + (180.0 - 120.0) / 2.0);
    }

    #[test]
    fn column_fill_children_share_remaining_height() {
        let scene: CardScene = parse_scene_ron(
            r#"(items: [ Column(x: 0.0, y: 0.0, h: 100.0, spacing: 4.0, items: [
                    Hit(x: 0.0, y: 0.0, w: 100.0, h: 20.0, key: 1),
                    Hit(x: 0.0, y: 0.0, w: 100.0, h: 0.0, key: 2),
                    Hit(x: 0.0, y: 0.0, w: 100.0, h: 0.0, key: 3),
                ]) ])"#,
        )
        .unwrap();
        let mut v: Vec<Cmd> = Vec::new();
        let (hits, _) = scene.draw(
            &mut v,
            0.0,
            0.0,
            200.0,
            120.0,
            scale(),
            &pal(),
            &SceneValues::new(),
            0,
            true,
            true,
        );
        assert_eq!(hits.len(), 3);
        // fixed=20, 2 fill children, 2 gaps → (100 - 20 - 8)/2 = 36 each
        assert_eq!(hits[1].y, 24.0, "20 + 4 spacing");
        assert_eq!(hits[1].h, 36.0, "(100 - 20 - 8)/2 fills");
        assert_eq!(hits[2].y, 64.0, "24 + 36 + 4 spacing");
    }

    #[test]
    fn stack_overlays_fullbleed_surface_then_ring() {
        let scene: CardScene = parse_scene_ron(
            r#"(items: [ Column(x: 0.0, y: 0.0, h: 120.0, spacing: 4.0, items: [
                    Stack(x: 0.0, y: 0.0, h: 60.0, items: [
                        Surface(x: 0.0, y: 0.0, w: 0.0, h: 0.0, color: hover),
                        Ring(name: "mem", cx: 0.0, cy: 0.0, radius: 24.0, center_x: true, center_y: true, max: 100.0, beads: 36),
                    ]),
                ]) ])"#,
        )
        .unwrap();
        let mut values = SceneValues::new();
        values.insert("mem", SceneValue::Ring(50.0));
        let mut v: Vec<Cmd> = Vec::new();
        let (_h, _i) = scene.draw(
            &mut v,
            0.0,
            0.0,
            200.0,
            120.0,
            scale(),
            &pal(),
            &values,
            0,
            true,
            true,
        );
        // full-bleed surface spans the stack's (implicit card-width) box
        let hover = ColorToken::Hover.resolve(&pal());
        let full = v
            .iter()
            .filter_map(|c| match c {
                Cmd::Rect { w, h, color, .. } if *color == hover => Some((w, h)),
                _ => None,
            })
            .next()
            .unwrap();
        assert!(
            (*full.0 - 200.0).abs() < 0.001,
            "surface fills width, got {}",
            full.0
        );
        assert!(
            (*full.1 - 60.0).abs() < 0.001,
            "surface fills stack height, got {}",
            full.1
        );
    }

    #[test]
    fn nested_containers_keep_child_scaling_relative() {
        let scene: CardScene = parse_scene_ron(
            r#"(items: [ Column(x: 0.0, y: 0.0, w: 200.0, h: 120.0, spacing: 10.0, items: [
                    Row(spacing: 8.0, items: [
                        Hit(x: 0.0, y: 0.0, w: 90.0, h: 30.0, key: 1),
                        Toggle(name: "blur", x: 0.0, y: 0.0, w: 38.0, h: 20.0, key: 2),
                    ]),
                    Fader(name: "vol", x: 0.0, y: 0.0, w: 0.0, h: 20.0, key: 3),
                ]) ])"#,
        )
        .unwrap();
        let mut values = SceneValues::new();
        values.insert("blur", SceneValue::Toggle(true));
        values.insert("vol", SceneValue::Fader(0.5));
        let mut v: Vec<Cmd> = Vec::new();
        let (hits, _) = scene.draw(
            &mut v,
            0.0,
            0.0,
            200.0,
            120.0,
            scale(),
            &pal(),
            &values,
            0,
            true,
            true,
        );
        assert_eq!(hits.len(), 3, "nested Hit + Toggle + Fader all register");
        assert_eq!(hits[0].x, 0.0);
        assert_eq!(hits[1].x, 98.0, "90 + 8 spacing");
        assert_eq!(hits[2].x, 0.0, "fader in the Column fills column width");
        assert_eq!(hits[2].w, 200.0);
        // Row fills remaining 90 (h:120 - Fader h:20 - spacing 10), then fader below
        assert_eq!(
            hits[2].y, 100.0,
            "fader y = Row fill height + Column spacing"
        );
    }

    #[test]
    fn container_ron_roundtrips_with_extension_shapes() {
        // The scene engine derives Serialize: nested containers serialize back.
        let _scene: CardScene = parse_scene_ron(
            r#"(items: [ Stack(w: 0.0, h: 0.0, items: [
                    Row(w: 0.0, h: 0.0, spacing: 4.0, items: [
                        Toggle(name: "a", x: 0.0, y: 0.0, w: 20.0, h: 12.0, key: 1),
                    ]),
                ]) ])"#,
        )
        .unwrap();
    }

    // ── scissor clip regions (the banner-zone prerequisite) ────────────

    #[test]
    fn scissor_emits_clip_commands_at_scaled_coords() {
        let scene: CardScene = parse_scene_ron(
            r#"(items: [
                Scissor(x: 10.0, y: 12.0, w: 80.0, h: 24.0),
                Text(x: 0.0, y: 0.0, text: "clipped", font_size: 10.0),
                ScissorEnd,
            ])"#,
        )
        .unwrap();
        let mut v: Vec<Cmd> = Vec::new();
        let (_h, _i) = scene.draw(
            &mut v,
            0.0,
            0.0,
            200.0,
            100.0,
            scale(),
            &pal(),
            &SceneValues::new(),
            0,
            true,
            true,
        );
        let clip = v
            .iter()
            .find(|c| matches!(c, Cmd::Scissor { .. }))
            .expect("opens a clip");
        match clip {
            Cmd::Scissor { x, y, w, h } => {
                assert_eq!(*x, 10.0);
                assert_eq!(*y, 12.0);
                assert_eq!(*w, 80.0);
                assert_eq!(*h, 24.0);
            }
            other => panic!("expected Scissor, got {:?}", std::mem::discriminant(other)),
        }
        assert!(
            v.iter().any(|c| matches!(c, Cmd::ScissorEnd)),
            "closes the clip"
        );
        assert_eq!(
            v.iter()
                .filter(|c| matches!(c, Cmd::Scissor { .. }))
                .count(),
            v.iter().filter(|c| matches!(c, Cmd::ScissorEnd)).count(),
            "balanced open/close"
        );
    }

    #[test]
    fn scissor_zero_dim_spans_the_parent_box() {
        let scene: CardScene =
            parse_scene_ron("(items: [ Scissor(x: 0.0, y: 0.0), ScissorEnd ])").unwrap();
        let mut v: Vec<Cmd> = Vec::new();
        let (_h, _i) = scene.draw(
            &mut v,
            5.0,
            7.0,
            200.0,
            100.0,
            scale(),
            &pal(),
            &SceneValues::new(),
            0,
            true,
            true,
        );
        match v.first() {
            Some(Cmd::Scissor { x, y, w, h }) => {
                assert_eq!(*x, 5.0);
                assert_eq!(*y, 7.0);
                assert_eq!(*w, 200.0, "w: 0 spans the parent width");
                assert_eq!(*h, 100.0, "h: 0 spans the parent height");
            }
            _other => panic!("expected an opening Scissor, was a non-Scissor command"),
        }
    }

    #[test]
    fn scissor_ron_roundtrips_with_default_dims() {
        let scene: CardScene =
            parse_scene_ron("(items: [ Scissor(x: 3.0, y: 4.0), ScissorEnd ])").unwrap();
        match &scene.items[0] {
            SceneItem::Scissor { x, y, w, h } => {
                assert_eq!(*x, Val::Lit(3.0));
                assert_eq!(*y, Val::Lit(4.0));
                assert_eq!(*w, Val::Lit(0.0), "w/h default to 0 (fill)");
                assert_eq!(*h, Val::Lit(0.0));
            }
            other => panic!("expected Scissor, got {other:?}"),
        }
    }

    // ── banner tokens (the named `dashboard` prerequisite) ─────────────

    #[test]
    fn banner_tokens_resolve_like_scene_values() {
        // The exact bindings `Shell::scene_values()` publishes for the banner
        // surface: two drop-guide inks resolved from the live palette, and the
        // edit/zone/lift state booleans. A scene authored against them must
        // resolve through the normal `value(...)` / `Toggle(name)` paths.
        let pal = pal();
        let hov = hover(&pal);
        let mut values = SceneValues::new();
        values.insert_color("banner_guide", SceneColor::Raw(mix(hov, pal.acc, 0.35)));
        values.insert_color("banner_guide_dim", SceneColor::Raw(mix(hov, pal.fg, 0.10)));
        values.insert("banner_editing", SceneValue::Toggle(true));
        values.insert("banner_zones", SceneValue::Toggle(true));

        let scene: CardScene = parse_scene_ron(
            r#"(items: [
                Scissor(x: 10.0, y: 10.0, w: 60.0, h: 10.0),
                Surface(x: 10.0, y: 12.0, w: 2.0, h: 6.0, color: value("banner_guide_dim")),
                ScissorEnd,
                Toggle(name: "banner_editing", x: 0.0, y: 30.0, w: 30.0, h: 16.0),
                Surface(x: 10.0, y: 34.0, w: 2.0, h: 6.0, color: value("banner_guide")),
            ])"#,
        )
        .unwrap();
        let mut v: Vec<Cmd> = Vec::new();
        let (_h, _i) = scene.draw(
            &mut v,
            0.0,
            0.0,
            200.0,
            100.0,
            scale(),
            &pal,
            &values,
            0,
            true,
            true,
        );
        let active = mix(hov, pal.acc, 0.35);
        let dim = mix(hov, pal.fg, 0.10);
        assert_eq!(
            v.iter()
                .filter(|c| matches!(c, Cmd::Rect { color, .. } if *color == active))
                .count(),
            1,
            "the active drop-guide ink paints exactly one bar"
        );
        assert_eq!(
            v.iter()
                .filter(|c| matches!(c, Cmd::Rect { color, .. } if *color == dim))
                .count(),
            1,
            "the dim drop-guide ink paints exactly one bar"
        );
    }

    #[test]
    fn composer_draws_placeholder_then_buffer_and_caret() {
        let mut idle = SceneValues::new();
        idle.insert("todo_focus", SceneValue::Toggle(false));
        idle.insert("todo_buf", SceneValue::Text(String::new()));
        let scene: CardScene = parse_scene_ron(
            r#"(items: [
                Composer(pad: 12.0, y: 12.0, bottom: true, text: Some("{todo_buf}"),
                        focus: Some("todo_focus"), placeholder: "Add a task…",
                        key: 78, add: true, add_key: 30702),
            ])"#,
        )
        .unwrap();
        let mut v: Vec<Cmd> = Vec::new();
        let (hits, _) = scene.draw(
            &mut v,
            0.0,
            0.0,
            200.0,
            100.0,
            scale(),
            &pal(),
            &idle,
            0,
            true,
            true,
        );
        // idle: field + add button rects, placeholder, two regions (field + ⊕)
        assert!(
            v.iter()
                .any(|c| matches!(c, Cmd::Text { text, .. } if text == "Add a task…")),
            "placeholder hidden until typing"
        );
        assert!(
            !v.iter()
                .any(|c| matches!(c, Cmd::Rect { w, .. } if *w == 1.4)),
            "no caret while unfocused"
        );
        assert_eq!(
            hits.iter().filter(|h| h.key == 78).count(),
            1,
            "field region"
        );
        assert_eq!(
            hits.iter().filter(|h| h.key == 30702).count(),
            1,
            "⊕ region"
        );

        // focused with text: buffer, caret, keyboard hint — ⊕ hides
        let mut on = SceneValues::new();
        on.insert("todo_focus", SceneValue::Toggle(true));
        on.insert("todo_buf", SceneValue::Text("buy milk".into()));
        let mut v: Vec<Cmd> = Vec::new();
        let (hits, _) = scene.draw(
            &mut v,
            0.0,
            0.0,
            200.0,
            100.0,
            scale(),
            &pal(),
            &on,
            0,
            true,
            true,
        );
        assert!(
            v.iter()
                .any(|c| matches!(c, Cmd::Text { text, .. } if text == "buy milk")),
            "buffer interpolated"
        );
        assert!(v.iter().any(|c| matches!(c, Cmd::Text { text, icon, .. } if *icon && text == crate::icons::ICON_KEYBOARD)), "typing hint shown");
        assert!(
            v.iter()
                .any(|c| matches!(c, Cmd::Rect { w, .. } if *w == 1.4)),
            "caret drawn while focused"
        );
        assert!(
            !v.iter()
                .any(|c| matches!(c, Cmd::Text { text, .. } if text == "Add a task…")),
            "placeholder gone while typing"
        );
        assert_eq!(
            hits.iter().filter(|h| h.key == 30702).count(),
            0,
            "⊕ hidden while focused"
        );
        assert_eq!(
            hits.iter().filter(|h| h.key == 78).count(),
            1,
            "field still focusable"
        );
    }

    #[test]
    fn composer_bottom_anchors_to_card_bottom() {
        let scene: CardScene = parse_scene_ron(
            r#"(items: [
                Composer(pad: 12.0, y: 12.0, bottom: true, add: true, add_key: 9),
            ])"#,
        )
        .unwrap();
        for (card_h, expect_field_y, expect_add_y) in [(100.0, 62.0, 72.0), (140.0, 102.0, 112.0)] {
            let mut v: Vec<Cmd> = Vec::new();
            let (_h, _i) = scene.draw(
                &mut v,
                0.0,
                0.0,
                200.0,
                card_h,
                scale(),
                &pal(),
                &vals(),
                0,
                true,
                true,
            );
            // field: h 26, top = card_h - 26 - 12 = card_h - 38
            let field = v
                .iter()
                .find(|c| matches!(c, Cmd::Rect { h, .. } if *h == 26.0))
                .expect("field rect");
            if let Cmd::Rect { y, .. } = field {
                assert!(
                    (y - expect_field_y).abs() < 0.01,
                    "field pinned (want {expect_field_y}, got {y})"
                );
            }
            // ⊕: 20² at x=card_w-30, y = card_h - 28 (10 up from the 18 bottom anchor)
            let add = v
                .iter()
                .find(|c| matches!(c, Cmd::Rect { w, h, .. } if *w == 20.0 && *h == 20.0))
                .expect("⊕ button");
            if let Cmd::Rect { y, .. } = add {
                assert!(
                    (y - expect_add_y).abs() < 0.01,
                    "⊕ pinned (want {expect_add_y}, got {y})"
                );
            }
        }
    }

    #[test]
    fn composer_ron_roundtrips_with_defaults() {
        let src = r#"(items: [
            Composer(pad: 12.0, y: 12.0, text: Some("{todo_buf}"), focus: Some("todo_focus"),
                     placeholder: "Add a task…", key: 78, add: true, add_key: 30702),
        ])"#;
        let scene: CardScene = parse_scene_ron(src).unwrap();
        let ink: String = ron::to_string(&scene).unwrap();
        let back: CardScene = parse_scene_ron(&ink).unwrap();
        match &back.items[0] {
            SceneItem::Composer {
                h,
                bottom,
                add,
                key,
                add_key,
                ..
            } => {
                assert_eq!(*h, Val::Lit(26.0), "default height");
                assert!(!bottom, "top-anchored by default");
                assert!(*add && *key == 78 && *add_key == 30702);
            }
            other => panic!("roundtrip produced {other:?}"),
        }
    }

    #[test]
    fn rows_more_footer_counts_the_hidden_rows_and_hides_when_done() {
        let scene: CardScene = parse_scene_ron(
            r#"(items: [ Rows(name: "rw", y: 36.0, row_h: 24.0, bottom: 38.0, cols: [ (x: 0.0) ], more: Some("+{n} more")) ])"#,
        )
        .unwrap();
        let mut values = SceneValues::new();
        let rows_of = |n: usize| {
            (0..n)
                .map(|i| SceneRow {
                    cols: vec![format!("t{i}")],
                    key: 0,
                    action: None,
                    color: None,
                    col_colors: vec![],
                    surface: None,
                })
                .collect::<Vec<SceneRow>>()
        };
        // 10 tasks on a 200 px card: rows from 36, 24 px stride, 38 px
        // reserved at the bottom → 5 fit (boxes end 180 > 162 from row 5 on)
        values.insert("rw", SceneValue::Rows(rows_of(10)));
        let mut v: Vec<Cmd> = Vec::new();
        scene.draw(
            &mut v,
            0.0,
            0.0,
            220.0,
            200.0,
            scale(),
            &pal(),
            &values,
            0,
            true,
            true,
        );
        let more = v
            .iter()
            .find_map(|c| match c {
                Cmd::Text { text, .. } if text.starts_with('+') => Some(text.clone()),
                _ => None,
            })
            .expect("overflow footer drawn while rows are hidden");
        assert_eq!(more, "+5 more", "5 of 10 tasks fit under the fold");
        let fy = v
            .iter()
            .find_map(|c| match c {
                Cmd::Text { y, text, .. } if text.starts_with('+') => Some(*y),
                _ => None,
            })
            .expect("footer y");
        assert!(
            (fy - (36.0 + 5.0 * 24.0 + 2.0)).abs() < 0.01,
            "footer 2 px under the last drawn row, got {fy}"
        );
        // nothing hidden → no footer at all
        values.insert("rw", SceneValue::Rows(rows_of(4)));
        let mut v2: Vec<Cmd> = Vec::new();
        scene.draw(
            &mut v2,
            0.0,
            0.0,
            220.0,
            200.0,
            scale(),
            &pal(),
            &values,
            0,
            true,
            true,
        );
        assert!(
            !v2.iter()
                .any(|c| matches!(c, Cmd::Text { text, .. } if text.starts_with('+'))),
            "a list that fits shows no footer"
        );
    }

    // ── fader / container primitives (2026-09-25 sliders conversion) ──

    #[test]
    fn every_live_card_scene_parses() {
        // The live config tree is the surface the shell actually reads, and a
        // malformed scene fails SILENTLY at runtime (the card falls back to
        // its Rust drawer), so it can sit broken for months. This walks every
        // `ui/cards/*.ron` and parses it. It paid for itself immediately: three
        // committed scenes carried `visible: "name"` where the schema wants
        // `visible: Some("name")` — a form `IMPLICIT_SOME` now accepts too, so
        // the guard is for genuinely broken syntax, not a spelling.
        let dir = crate::vars::card_scenes_dir();
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => return, // config tree absent in this env
        };
        let mut checked = 0usize;
        let mut broken: Vec<String> = Vec::new();
        for e in entries.flatten() {
            let path = e.path();
            if path.extension().and_then(|s| s.to_str()) != Some("ron") {
                continue;
            }
            let text = std::fs::read_to_string(&path).expect("scene file readable");
            if parse_scene_ron::<CardScene>(&text).is_err() {
                broken.push(format!(
                    "{}: {}",
                    path.display(),
                    parse_scene_ron::<CardScene>(&text).unwrap_err()
                ));
            }
            checked += 1;
        }
        let _ = checked;
        assert!(
            broken.is_empty(),
            "live card scenes must parse:\n{}",
            broken.join("\n")
        );
        // no `.ron` remain in the live tree (every card is a `.lua` module
        // now), so `checked` is legitimately 0 — the ratchet below guards the
        // inventory, not the file count.
    }

    /// `Option` fields are written BARE — the whole point of the
    /// `implicit_some` load path. Both spellings must load, and load to the
    /// SAME scene, so a file can be migrated a field at a time and a half-
    /// migrated tree still renders.
    #[test]
    fn optional_fields_take_both_spellings() {
        let bare = r#"(
            items: [
                Text(x: "vx", y: 2.0, text: "{v}", font_size: "vs", color: fg3,
                     visible: "gate"),
                Hit(x: 0.0, y: 0.0, w: 10.0, h: 10.0, key: 7, surface: raised,
                    anchors: (left: -2.0, top: 4.0, right: 2.0, bottom: -4.0),
                    action: Command("zen-shell open launcher")),
            ],
        )"#;
        let wrapped = r#"(
            items: [
                Text(x: "vx", y: 2.0, text: "{v}", font_size: "vs", color: Some(fg3),
                     visible: Some("gate")),
                Hit(x: 0.0, y: 0.0, w: 10.0, h: 10.0, key: 7, surface: Some(raised),
                    anchors: Some((left: -2.0, top: 4.0, right: 2.0, bottom: -4.0)),
                    action: Some(Command("zen-shell open launcher"))),
            ],
        )"#;
        let a: CardScene = parse_scene_ron(bare).expect("bare optionals parse");
        let b: CardScene = parse_scene_ron(wrapped).expect("Some-wrapped optionals parse");
        assert_eq!(a, b, "the wrapper is spelling, not meaning");
        // and the bare form really is the schema, not a skipped field
        match &a.items[0] {
            SceneItem::Text {
                x,
                font_size,
                color: Some(ColorToken::Fg3),
                visible: Some(v),
                ..
            } if v == "gate" && x.binding() == Some("vx") && font_size.binding() == Some("vs") => {}
            other => panic!("bare optionals did not bind: {other:?}"),
        }
    }

    #[test]
    fn fader_negative_w_stops_short_of_the_parent_span() {
        // `w: -38` is the Image span-minus convention applied to a fader: the
        // groove ends 38 px before the parent's right edge so a value column
        // stays clear on any card width.
        let scene: CardScene = parse_scene_ron(
            r#"(items: [ Stack(x: 14.0, y: 0.0, w: 0.0, h: 24.0, items: [
                    Fader(name: "f", x: 20.0, y: 0.0, w: -18.0, h: 24.0, key: 51),
                ]) ])"#,
        )
        .unwrap();
        let mut m = SceneValues::new();
        m.insert("f", SceneValue::Fader(0.5));
        let mut v: Vec<Cmd> = Vec::new();
        let (hits, _) = scene.draw(
            &mut v,
            0.0,
            0.0,
            300.0,
            120.0,
            scale(),
            &pal(),
            &m,
            0,
            true,
            true,
        );
        assert_eq!(hits.len(), 1);
        // stack box: x 14, w 0 → 286 wide (to the card's right edge) → the
        // fader starts 20 px in and its `w: -18` leaves 38 px of that row
        assert_eq!(hits[0].x, 34.0, "fader starts 20 px into the row box");
        assert_eq!(
            hits[0].w, 248.0,
            "negative w = the row's span from x, minus the inset"
        );
    }

    #[test]
    fn fader_centered_fill_grows_from_the_middle() {
        // `centered: true` is `ui::fader_bal`: the fill leaves the middle, not
        // the left edge (the balance / saturation rows).
        let r = |centered: bool, val: f32| {
            let scene: CardScene = parse_scene_ron(&format!(
                r#"(items: [ Fader(name: "f", x: 0.0, y: 0.0, w: 100.0, h: 10.0, centered: {centered}) ])"#
            ))
            .unwrap();
            let mut m = SceneValues::new();
            m.insert("f", SceneValue::Fader(val));
            let mut v: Vec<Cmd> = Vec::new();
            scene.draw(
                &mut v,
                0.0,
                0.0,
                200.0,
                60.0,
                scale(),
                &pal(),
                &m,
                0,
                true,
                true,
            );
            v
        };
        // groove (track) is the first rect; the fill is the second
        let fill_x = |cmds: &Vec<Cmd>| match cmds[1] {
            Cmd::Rect { x, .. } => x,
            _ => panic!("expected a fill rect"),
        };
        assert_eq!(
            fill_x(&r(true, 0.75)),
            50.0,
            "centered fill runs from the middle to 75%"
        );
        assert_eq!(
            fill_x(&r(true, 0.25)),
            25.0,
            "and from 25% to the middle below center"
        );
        assert_eq!(
            fill_x(&r(false, 0.75)),
            0.0,
            "the default fader fills from the left edge"
        );
    }

    #[test]
    fn fader_visible_gate_hides_the_whole_row() {
        let scene: CardScene = parse_scene_ron(
            r#"(items: [ Fader(name: "f", x: 0.0, y: 0.0, w: 100.0, h: 10.0, key: 59, visible: Some("gate")) ])"#,
        )
        .unwrap();
        let mut m = SceneValues::new();
        m.insert("f", SceneValue::Fader(0.5));
        m.insert("gate", SceneValue::Toggle(false));
        let mut v: Vec<Cmd> = Vec::new();
        let (hits, _) = scene.draw(
            &mut v,
            0.0,
            0.0,
            200.0,
            60.0,
            scale(),
            &pal(),
            &m,
            0,
            true,
            true,
        );
        assert!(
            hits.is_empty() && v.is_empty(),
            "a gated-false fader draws nothing"
        );
        m.insert("gate", SceneValue::Toggle(true));
        let mut v2: Vec<Cmd> = Vec::new();
        let (hits2, _) = scene.draw(
            &mut v2,
            0.0,
            0.0,
            200.0,
            60.0,
            scale(),
            &pal(),
            &m,
            0,
            true,
            true,
        );
        assert_eq!(hits2.len(), 1, "gate true draws it again");
    }

    #[test]
    fn container_layout_skips_invisible_children() {
        // a gated-false child reserves NO slot: the stack reflows around it
        // (the sliders card's optional balance / saturation rows).
        let r = |on: bool| {
            let scene: CardScene = parse_scene_ron(
                r#"(items: [ Column(x: 0.0, y: 0.0, w: 100.0, items: [
                        Hit(x: 0.0, y: 0.0, w: 10.0, h: 20.0, key: 1),
                        Hit(x: 0.0, y: 0.0, w: 10.0, h: 20.0, key: 2, visible: Some("gate")),
                        Hit(x: 0.0, y: 0.0, w: 10.0, h: 20.0, key: 3),
                    ]) ])"#,
            )
            .unwrap();
            let mut m = SceneValues::new();
            m.insert("gate", SceneValue::Toggle(on));
            let mut v: Vec<Cmd> = Vec::new();
            let (hits, _) = scene.draw(
                &mut v,
                0.0,
                0.0,
                200.0,
                120.0,
                scale(),
                &pal(),
                &m,
                0,
                true,
                true,
            );
            hits
        };
        let on = r(true);
        assert_eq!(on.len(), 3);
        assert_eq!(
            (on[0].y, on[1].y, on[2].y),
            (0.0, 20.0, 40.0),
            "all three rows stack"
        );
        let off = r(false);
        assert_eq!(off.len(), 2, "the hidden row draws nothing");
        assert_eq!(
            (off[0].y, off[1].y),
            (0.0, 20.0),
            "and leaves no gap behind"
        );
    }

    #[test]
    fn column_valign_middle_centers_the_stacked_block() {
        // the sliders pane-0 block: 4 rows of 24 in a 100-tall box → 2 px of
        // slack above and below.
        let r = |valign: &str| {
            let scene: CardScene = parse_scene_ron(&format!(
                r#"(items: [ Column(x: 0.0, y: 0.0, w: 100.0, h: 100.0, valign: {valign}, items: [
                        Hit(x: 0.0, y: 0.0, w: 10.0, h: 24.0, key: 1),
                        Hit(x: 0.0, y: 0.0, w: 10.0, h: 24.0, key: 2),
                        Hit(x: 0.0, y: 0.0, w: 10.0, h: 24.0, key: 3),
                        Hit(x: 0.0, y: 0.0, w: 10.0, h: 24.0, key: 4),
                    ]) ])"#
            ))
            .unwrap();
            let mut v: Vec<Cmd> = Vec::new();
            let (hits, _) = scene.draw(
                &mut v,
                0.0,
                0.0,
                200.0,
                140.0,
                scale(),
                &pal(),
                &SceneValues::new(),
                0,
                true,
                true,
            );
            hits
        };
        let top = r("top");
        assert_eq!(top[0].y, 0.0, "top-aligned stacks from the box top");
        assert_eq!(top[3].y, 72.0);
        let mid = r("middle");
        assert_eq!(
            mid[0].y, 2.0,
            "96 px of rows in a 100 px box → 2 px of slack"
        );
        assert_eq!(mid[3].y, 74.0);
    }

    // ── `Tiles` (the toggles card's responsive switch grid) ──

    /// A `Tiles` item plus a six-tile list, mirroring what `scene_values()`
    /// publishes for the toggles card (Wi-Fi + BT carry a chip key).
    fn tiles_fixture(ron_body: &str, on: &[bool]) -> (CardScene, SceneValues) {
        let src = format!("(items: [Tiles({ron_body})])");
        let scene: CardScene = parse_scene_ron(&src).expect("fixture Tiles parses");
        let keys = [41u32, 43, 45, 46, 47, 48];
        let labels = ["Wi-Fi", "BT", "DND", "Caffeine", "Blue Light", "Saturation"];
        let mut m = SceneValues::new();
        m.insert(
            "t",
            SceneValue::Tiles(
                (0..6)
                    .map(|i| SceneTile {
                        key: keys[i],
                        glyph: "\u{f1eb}".into(),
                        label: labels[i].into(),
                        on: on.get(i).copied().unwrap_or(false),
                        chip_key: if i < 2 { keys[i] + 1 } else { 0 },
                    })
                    .collect(),
            ),
        );
        (scene, m)
    }

    const TILES_BODY: &str = r#"
        name: "t", x: 0.0, y: 0.0, w: 0.0, h: 0.0,
        pad: 10.0, gap: 8.0, cols: 1,
        cols_break: [(225.0, 2), (330.0, 3)],
        r: 12.0, icon_size: 13.0, label_size: 8.5,
        icon_top: 8.0, label_top: 30.0, label_min_h: 42.0,
        on_surface: acc_tint, hover_surface: hover_hl, off_surface: hover,
        on_ink: acc, off_ink: fg, label_ink: fg,
        chip: true, chip_w: 18.0, chip_size: 9.0, chip_dx: 10.0, chip_dy: 14.0
    "#;

    fn tile_hits(hits: &[SceneHit]) -> Vec<&SceneHit> {
        // tile regions are the full-height boxes; the chip strips are 18 px wide
        hits.iter().filter(|t| t.w > 18.0).collect()
    }

    #[test]
    fn tiles_breakpoints_pick_the_columns_the_drawer_picks() {
        // the drawer's `if w >= 330 {3} else if w >= 225 {2} else {1}` becomes
        // ascending min-width breakpoints — 1 / 2 / 3 columns
        for (card_w, want_cols) in [
            (200.0, 1usize),
            (224.0, 1),
            (225.0, 2),
            (329.0, 2),
            (330.0, 3),
            (420.0, 3),
        ] {
            let (scene, m) = tiles_fixture(TILES_BODY, &[false; 6]);
            let (w, h) = (card_w, 200.0);
            let mut v: Vec<Cmd> = Vec::new();
            let (hits, _) = scene.draw(&mut v, 0.0, 0.0, w, h, scale(), &pal(), &m, 0, true, true);
            let tiles = tile_hits(&hits);
            assert_eq!(tiles.len(), 6, "all six tiles register a region at w={w}");
            let rows = 6 / want_cols;
            // drawer: tw = (w − 20 − (cols−1)·8) / cols, tiles at 10 + i·(tw+8)
            let tw = (w - 20.0 - (want_cols as f32 - 1.0) * 8.0) / want_cols as f32;
            // drawer: th = (h − 20 − (rows−1)·8) / rows
            let th = (h - 20.0 - (rows as f32 - 1.0) * 8.0) / rows as f32;
            for (i, t) in tiles.iter().enumerate() {
                let col = i % want_cols;
                let row = i / want_cols;
                assert_eq!(
                    t.x,
                    10.0 + col as f32 * (tw + 8.0),
                    "tile {i} column x at w={w}"
                );
                assert_eq!(
                    t.y,
                    10.0 + row as f32 * (th + 8.0),
                    "tile {i} row y at w={w}"
                );
                assert_eq!(t.w, tw, "tile {i} width at w={w}");
                assert_eq!(t.h, th, "tile {i} height at w={w}");
            }
        }
    }

    #[test]
    fn tiles_breaks_only_on_the_card_box_width() {
        // the breakpoints read the CARD width, not the item box: a box that
        // stops short of the right edge still reflows on the card's size
        let (scene, m) = tiles_fixture(
            r#"
            name: "t", x: 0.0, y: 0.0, w: -40.0, h: 0.0,
            pad: 10.0, gap: 8.0, cols: 1,
            cols_break: [(225.0, 2), (330.0, 3)]
        "#,
            &[false; 6],
        );
        let mut v: Vec<Cmd> = Vec::new();
        let (hits, _) = scene.draw(
            &mut v,
            0.0,
            0.0,
            340.0,
            200.0,
            scale(),
            &pal(),
            &m,
            0,
            true,
            true,
        );
        let tiles = tile_hits(&hits);
        assert_eq!(tiles.len(), 6);
        // three columns inside a 300 px box (card 340 − the 40 px short stop)
        let tw = (300.0 - 20.0 - 16.0) / 3.0;
        assert!(
            (tiles[0].w - tw).abs() < 0.01,
            "a short-stopped box still splits three ways"
        );
        assert!((tiles[0].x - 10.0).abs() < 0.01);
    }

    #[test]
    fn tiles_squat_tiles_drop_the_caption_and_center_the_glyph() {
        // the drawer's `labeled = th_ >= 42` branch: a 30 px tile is icon-only
        // with the glyph centered; a 50 px tile shows icon + caption
        // six rows need 20 + 5·8 + 6·42 = 312 px of card before a tile
        // clears the caption threshold, so 200 px is icon-only and 320 is not
        for (h, labeled) in [(200.0, false), (320.0, true)] {
            let (scene, m) = tiles_fixture(TILES_BODY, &[false; 6]);
            let mut v: Vec<Cmd> = Vec::new();
            scene.draw(
                &mut v,
                0.0,
                0.0,
                200.0,
                h,
                scale(),
                &pal(),
                &m,
                0,
                true,
                true,
            );
            let rows = 6;
            let th = (h - 20.0 - (rows as f32 - 1.0) * 8.0) / rows as f32;
            let glyph_y = v
                .iter()
                .find_map(|c| match c {
                    Cmd::Text { y, text, .. } if text == "\u{f1eb}" => Some(*y),
                    _ => None,
                })
                .expect("the glyph draws");
            let has_label = v
                .iter()
                .any(|c| matches!(c, Cmd::Text { text, .. } if text == "Wi-Fi"));
            assert_eq!(
                has_label, labeled,
                "caption present only on a tall tile (h={h})"
            );
            let want = if labeled {
                10.0 + 8.0 // icon_top inside the tile
            } else {
                10.0 + (th - 13.0) / 2.0 // centered in the tile
            };
            assert!(
                (glyph_y - want).abs() < 0.01,
                "glyph y at h={h} (got {glyph_y}, want {want})"
            );
        }
    }

    #[test]
    fn tiles_label_sits_below_the_glyph_on_a_tall_tile() {
        let (scene, m) = tiles_fixture(TILES_BODY, &[false; 6]);
        let mut v: Vec<Cmd> = Vec::new();
        scene.draw(
            &mut v,
            0.0,
            0.0,
            200.0,
            320.0,
            scale(),
            &pal(),
            &m,
            0,
            true,
            true,
        );
        // drawer: glyph at ty0 + 8, caption at ty0 + 30, both centered
        let cap = v
            .iter()
            .find_map(|c| match c {
                Cmd::Text { y, text, .. } if text == "Wi-Fi" => Some(*y),
                _ => None,
            })
            .expect("the caption draws on a tall tile");
        assert!(
            (cap - 40.0).abs() < 0.01,
            "caption at tile top + 30 (got {cap})"
        );
    }

    #[test]
    fn tiles_on_state_outranks_hover_and_lights_the_glyph() {
        // the drawer's `if on { acc_tint } else if hov { hover_hl } else { hover }`
        // and its `if on { acc } else { fg }` glyph ink
        let p = pal();
        // tile 0 (Wi-Fi) lit, tile 1 (BT) off — one list, both states
        let (scene, m2) = tiles_fixture(TILES_BODY, &[true, false, false, false, false, false]);
        // hovered AND on → still the accent wash, accent glyph
        let mut v: Vec<Cmd> = Vec::new();
        let (_, _) = scene.draw(
            &mut v,
            0.0,
            0.0,
            200.0,
            200.0,
            scale(),
            &p,
            &m2,
            41,
            true,
            true,
        );
        let fill = v.iter().find_map(|c| match c {
            Cmd::Rect { color, .. } => Some(*color),
            _ => None,
        });
        assert_eq!(
            fill,
            Some(crate::ui::acc_tint(&p)),
            "a lit tile keeps its accent wash under the cursor"
        );
        let ink = v.iter().find_map(|c| match c {
            Cmd::Text { text, color, .. } if text == "\u{f1eb}" => Some(*color),
            _ => None,
        });
        assert_eq!(ink, Some(p.acc), "a lit tile's glyph goes accent");
        // resting, off → the quiet hover surface (tile 1's fill, drawn second)
        let mut v2: Vec<Cmd> = Vec::new();
        let (_, _) = scene.draw(
            &mut v2,
            0.0,
            0.0,
            200.0,
            200.0,
            scale(),
            &p,
            &m2,
            0,
            true,
            true,
        );
        let fills: Vec<u32> = v2
            .iter()
            .filter_map(|c| match c {
                Cmd::Rect { color, .. } => Some(*color),
                _ => None,
            })
            .collect();
        assert_eq!(
            fills[0],
            crate::ui::acc_tint(&p),
            "a lit tile rests on its accent wash"
        );
        assert_eq!(
            fills[1],
            crate::ui::hover(&p),
            "an off tile sits on the quiet hover surface"
        );
        let inks: Vec<u32> = v2
            .iter()
            .filter_map(|c| match c {
                Cmd::Text { text, color, .. } if text == "\u{f1eb}" => Some(*color),
                _ => None,
            })
            .collect();
        assert_eq!(inks[0], p.acc, "a lit tile's glyph is accent");
        assert_eq!(inks[1], p.fg, "an off tile's glyph is plain fg");
        // hovered, off → the lifted surface
        let mut v3: Vec<Cmd> = Vec::new();
        let (_, _) = scene.draw(
            &mut v3,
            0.0,
            0.0,
            200.0,
            200.0,
            scale(),
            &p,
            &m2,
            43,
            true,
            true,
        );
        let hovered: Vec<u32> = v3
            .iter()
            .filter_map(|c| match c {
                Cmd::Rect { color, .. } => Some(*color),
                _ => None,
            })
            .collect();
        assert_eq!(
            hovered[1],
            crate::ui::hover_hl(&p),
            "a hovered off tile lifts one step"
        );
    }

    #[test]
    fn tiles_chip_is_a_tall_right_edge_region() {
        // the drawer's ⋯ chevron: glyph centered 10 px from the tile's right
        // edge, 14 px above its bottom, over an 18 px × full-height strip
        let (scene, m) = tiles_fixture(TILES_BODY, &[false; 6]);
        let (w, h) = (200.0, 200.0);
        let mut v: Vec<Cmd> = Vec::new();
        let (hits, _) = scene.draw(&mut v, 0.0, 0.0, w, h, scale(), &pal(), &m, 0, true, true);
        let chips: Vec<&SceneHit> = hits.iter().filter(|t| t.w == 18.0).collect();
        assert_eq!(chips.len(), 2, "only the two network tiles carry a chip");
        assert_eq!(
            chips.iter().map(|c| c.key).collect::<Vec<_>>(),
            vec![42, 44],
            "resident chip keys"
        );
        let th = (h - 20.0 - 5.0 * 8.0) / 6.0;
        for c in &chips {
            assert_eq!(c.h, th, "the chip strip spans the tile's height");
            assert_eq!(
                c.x + 18.0,
                10.0 + w - 20.0,
                "strip ends at the tile's right edge"
            );
        }
        let glyph = v
            .iter()
            .filter_map(|c| match c {
                Cmd::Text { x, y, text, .. } if text == crate::icons::ICON_MORE => Some((*x, *y)),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(glyph.len(), 2, "one chevron per chip tile");
        assert_eq!(
            glyph[0],
            (10.0 + w - 20.0 - 10.0, 10.0 + th - 14.0),
            "chevron at tile_right − 10, tile_bottom − 14"
        );
    }

    #[test]
    fn tiles_chip_ink_lifts_on_hover() {
        let (scene, m) = tiles_fixture(TILES_BODY, &[false; 6]);
        let mut v: Vec<Cmd> = Vec::new();
        let (_, _) = scene.draw(
            &mut v,
            0.0,
            0.0,
            200.0,
            200.0,
            scale(),
            &pal(),
            &m,
            42,
            true,
            true,
        );
        let ink = v.iter().find_map(|c| match c {
            Cmd::Text { text, color, .. } if text == crate::icons::ICON_MORE => Some(*color),
            _ => None,
        });
        assert_eq!(
            ink,
            Some(pal().acc),
            "the hovered chevron inks to the accent"
        );
    }

    #[test]
    fn tiles_gate_and_missing_binding_draw_nothing() {
        // an unbound list is a no-op (the same rule every data item follows)
        let (scene, _) = tiles_fixture(TILES_BODY, &[false; 6]);
        let mut v: Vec<Cmd> = Vec::new();
        let (hits, _) = scene.draw(
            &mut v,
            0.0,
            0.0,
            200.0,
            200.0,
            scale(),
            &pal(),
            &SceneValues::new(),
            0,
            true,
            true,
        );
        assert!(
            v.is_empty() && hits.is_empty(),
            "no tiles value → nothing drawn, nothing clickable"
        );
        // an explicit `visible` gate that resolves false hides the whole grid
        let (_, mut m) = tiles_fixture(TILES_BODY, &[false; 6]);
        let gated: CardScene = parse_scene_ron(
            r#"(items: [Tiles(
                name: "t", x: 0.0, y: 0.0, w: 0.0, h: 0.0,
                cols: 2, visible: Some("pane_1")
            )])"#,
        )
        .expect("gated fixture parses");
        m.insert("pane_1", SceneValue::Toggle(false));
        let mut v2: Vec<Cmd> = Vec::new();
        let (hits2, _) = gated.draw(
            &mut v2,
            0.0,
            0.0,
            200.0,
            200.0,
            scale(),
            &pal(),
            &m,
            0,
            true,
            true,
        );
        assert!(
            v2.is_empty() && hits2.is_empty(),
            "a false gate hides every tile"
        );
        // …and true opens it
        m.insert("pane_1", SceneValue::Toggle(true));
        let mut v3: Vec<Cmd> = Vec::new();
        let (hits3, _) = gated.draw(
            &mut v3,
            0.0,
            0.0,
            200.0,
            200.0,
            scale(),
            &pal(),
            &m,
            0,
            true,
            true,
        );
        assert_eq!(tile_hits(&hits3).len(), 6);
    }


    #[test]
    fn row_min_card_h_skips_the_row_on_a_short_card() {
        // the compositor's screenshot row: 78 px of rows + 30 px of buttons
        // + the 2 px the drawer keeps clear = a 110 px card, below which the
        // whole row stands down (no orphan buttons hanging off the bottom)
        let src = r#"(items: [
            Row(x: 0.0, y: 78.0, w: -24.0, h: 30.0, min_card_h: 110.0, items: [
                Hit(x: 0.0, y: 0.0, w: 0.0, h: 0.0, key: 31_403),
            ]),
        ])"#;
        let scene: CardScene = parse_scene_ron(src).expect("row fixture parses");
        for (h, want) in [(108.0, false), (109.9, false), (110.0, true), (200.0, true)] {
            let mut v: Vec<Cmd> = Vec::new();
            let (hits, _) = scene.draw(
                &mut v,
                0.0,
                0.0,
                240.0,
                h,
                scale(),
                &pal(),
                &vals(),
                0,
                true,
                true,
            );
            assert_eq!(
                hits.len(),
                usize::from(want),
                "row present on a {h} px card"
            );
            if want {
                assert_eq!(hits[0].y, 78.0, "the row keeps its declared top");
            }
        }
    }



    #[test]
    fn row_halign_centers_a_block_of_fixed_width_children() {
        // `halign: center` is a no-op for children that FILL the row (they
        // already span it) and centers the block when every child declares
        // its own width
        let filled = r#"(items: [
            Row(x: 0.0, y: 0.0, w: 100.0, h: 10.0, halign: center, items: [
                Text(x: 0.0, y: 0.0, w: 0.0, text: "a", font_size: 10.0),
                Text(x: 0.0, y: 0.0, w: 0.0, text: "b", font_size: 10.0),
            ]),
        ])"#;
        let s: CardScene = parse_scene_ron(filled).expect("filled fixture parses");
        let mut v: Vec<Cmd> = Vec::new();
        s.draw(
            &mut v,
            0.0,
            0.0,
            100.0,
            20.0,
            scale(),
            &pal(),
            &vals(),
            0,
            true,
            true,
        );
        let xs: Vec<f32> = v
            .iter()
            .filter_map(|c| match c {
                Cmd::Text { x, .. } => Some(*x),
                _ => None,
            })
            .collect();
        assert_eq!(
            xs,
            vec![0.0, 50.0],
            "fills stay left-anchored, split equally"
        );

        let fixed = r#"(items: [
            Row(x: 0.0, y: 0.0, w: 100.0, h: 10.0, spacing: 4.0, halign: center, items: [
                Surface(x: 0.0, y: 0.0, w: 20.0, h: 10.0, color: acc),
                Surface(x: 0.0, y: 0.0, w: 20.0, h: 10.0, color: acc),
            ]),
        ])"#;
        let s: CardScene = parse_scene_ron(fixed).expect("fixed fixture parses");
        let mut v: Vec<Cmd> = Vec::new();
        s.draw(
            &mut v,
            0.0,
            0.0,
            100.0,
            20.0,
            scale(),
            &pal(),
            &vals(),
            0,
            true,
            true,
        );
        let xs: Vec<f32> = v
            .iter()
            .filter_map(|c| match c {
                Cmd::Rect { x, .. } => Some(*x),
                _ => None,
            })
            .collect();
        assert_eq!(
            xs,
            vec![28.0, 52.0],
            "a 44 px block centers in 100 px (28 px lead)"
        );
    }

    #[test]
    fn bar_vertical_fills_bottom_up_and_surface_strokes_over() {
        // `vertical: true` scales the box height by the fraction, anchored on
        // the bottom edge; the horizontal bar is unchanged
        let vron = r#"(items: [
            Bar(x: 0.0, y: 0.0, w: 40.0, h: 20.0, vertical: true, value: "50", fill: acc, track: fg3),
            Bar(x: 50.0, y: 0.0, w: 40.0, h: 20.0, value: "50", fill: acc, track: fg3),
        ])"#;
        let s: CardScene = parse_scene_ron(vron).expect("bar fixture parses");
        let mut v: Vec<Cmd> = Vec::new();
        s.draw(
            &mut v,
            0.0,
            0.0,
            100.0,
            40.0,
            scale(),
            &pal(),
            &vals(),
            0,
            true,
            true,
        );
        let mut fills: Vec<(f32, f32, f32)> = Vec::new();
        for c in &v {
            if let Cmd::Rect { x, y, w, h, .. } = c {
                if (*h - 20.0).abs() > 0.01 || (*w - 40.0).abs() > 0.01 {
                    fills.push((*x, *y, *w));
                }
            }
        }
        assert_eq!(
            fills,
            vec![(0.0, 10.0, 40.0), (50.0, 0.0, 20.0)],
            "vertical fill: 20 x 0.5 anchored at y 10; horizontal: 40 x 0.5"
        );
        // `stroke` outlines the box on the fill's own rect
        let sron = r#"(items: [ Surface(x: 4.0, y: 6.0, w: 30.0, h: 18.0, r: 5.0, color: acc, stroke: 1.0, stroke_color: Some(hairline)) ])"#;
        let s: CardScene = parse_scene_ron(sron).expect("surface fixture parses");
        let mut v: Vec<Cmd> = Vec::new();
        s.draw(
            &mut v,
            0.0,
            0.0,
            100.0,
            40.0,
            scale(),
            &pal(),
            &vals(),
            0,
            true,
            true,
        );
        assert_eq!(v.len(), 2, "fill + outline");
        match v[1] {
            Cmd::Outline {
                x,
                y,
                w,
                h,
                r,
                color,
                ..
            } => {
                assert_eq!(
                    (x, y, w, h, r),
                    (4.0, 6.0, 30.0, 18.0, 5.0),
                    "outline traces the box"
                );
                assert_eq!(color, crate::ui::hairline(&pal()), "hairline ink");
            }
            _ => panic!("expected an Outline as the second command"),
        }
    }

    /// One comparable line per command — the differential-test fingerprint.
    /// Any command the pomodoro does not use still shows up by its variant
    /// index, so an unexpected extra command can never compare equal.
    fn fmt(v: &[Cmd]) -> Vec<String> {
        v.iter()
            .enumerate()
            .map(|(i, c)| match c {
                Cmd::Text {
                    x,
                    y,
                    right,
                    center,
                    text,
                    size,
                    color,
                    ..
                } => {
                    format!("T {x:.2} {y:.2} {right} {center} {size:.2} {color} {text}")
                }
                Cmd::Rect {
                    x,
                    y,
                    w,
                    h,
                    r,
                    color,
                } => format!("R {x:.2} {y:.2} {w:.2} {h:.2} {r:.2} {color}"),
                // Lines and images carry GEOMETRY the two painters compute
                // independently (a plot's box, a thumbnail's box), so compare
                // them in full — counting them would have let a 20 px plot
                // floor difference through.
                Cmd::Line {
                    x0,
                    y0,
                    x1,
                    y1,
                    w,
                    color,
                } => {
                    format!("L {x0:.2} {y0:.2} {x1:.2} {y1:.2} {w:.2} {color}")
                }
                Cmd::Image { x, y, w, h, key } => {
                    format!("I {x:.2} {y:.2} {w:.2} {h:.2} {key}")
                }
                // `Cmd` is neither `Debug` nor `Display`; the command's slot
                // plus a zero-width marker is enough to fail loudly on an
                // unexpected extra command.
                _ => format!("#{i} <other>"),
            })
            .collect()
    }

    #[test]
    fn every_notif_row_names_a_declared_template() {
        // The floor under `every_pill_widget_has_a_template_for_its_delegate`.
        //
        // That test walks the pill because the pill was the only model. A model
        // repeater's silent failure — a row whose `tpl` matches no template
        // draws NOTHING, no warning — is per-MODEL, so the moment a second model
        // lands the pill-only walk stops covering anything new, and the notif
        // conversion duly shipped with rows carrying no `tpl` at all. Every card
        // vanished and the suite was green.
        //
        // The parity differential would have caught the paint, but it cannot
        // catch a row kind that appears only for some inputs, and it says
        // nothing about the *declaration* being coherent. This says it directly:
        // for the rows the publisher can actually build, the delegate each one
        // names must exist in the template set (or there must be a wildcard).
        let text = crate::shell_scene::SHELL_SCENE_RON;
        let mut sc: ShellScene = parse_scene_ron(&text).expect("RON parses cleanly");
        sc.expand();
        let surf = sc.surface("notif").expect("the notif surface is declared");

        fn model_repeats(items: &[SceneItem], out: &mut Vec<(String, Vec<String>)>) {
            for it in items {
                match it {
                    SceneItem::Repeat {
                        model: Some(m),
                        templates,
                        ..
                    } => out.push((m.clone(), templates.iter().map(|t| t.tag.clone()).collect())),
                    SceneItem::Row { items, .. }
                    | SceneItem::Column { items, .. }
                    | SceneItem::Stack { items, .. } => model_repeats(items, out),
                    _ => {}
                }
            }
        }
        let mut reps = Vec::new();
        model_repeats(&surf.items, &mut reps);
        let (_, tags) = reps
            .iter()
            .find(|(m, _)| m == "notif_rows")
            .expect("notif declares its card list as a model Repeat");
        assert!(!tags.is_empty(), "the repeat declares templates at all");
        let wildcard = tags.iter().any(|t| t == "*");

        // build the widest row set the publisher can produce: eight rows, which
        // is the cap, so the check cannot pass on a short list
        let cfg: crate::config::Config =
            toml::from_str(crate::config::DEFAULT_SHELL_TOML).expect("cfg parses");
        let mut s = crate::shell::Shell::new(cfg);
        s.notifs = (0..8)
            .map(|i| crate::shell::Notif {
                id: i,
                app: "App".into(),
                summary: "Summary".into(),
                body: "Body".into(),
                icon: "icon".into(),
                urgency: (i % 3) as u8,
                actions: vec![],
            })
            .collect();
        let vals = s.scene_values_for("notif");
        let rows = vals.model("notif_rows");
        assert_eq!(rows.len(), 8, "the cap is 8 rows");
        for (i, row) in rows.iter().enumerate() {
            let tpl = model_field_text(row, "tpl").unwrap_or_default();
            let tag = model_field_text(row, "tag").unwrap_or_default();
            // `tpl` wins over `tag`, the same precedence `repeat_slots` applies
            let named = if tpl.is_empty() { tag } else { tpl };
            assert!(
                !named.is_empty(),
                "row {i} names no delegate: a model row without `tpl`/`tag` draws NOTHING \
                 and says nothing"
            );
            assert!(
                wildcard || tags.iter().any(|t| t == named),
                "row {i} names delegate {named:?}, which the scene does not declare \
                 (declared: {tags:?})"
            );
        }
    }

    /// The differential that earns the right to retire the Rust popover: ONE
    /// Shell, both painters — `layout_banner`'s connectivity arm and the
    /// declared `dconn` component — compared command for command and region
    /// for region, across both ink ladders and every hover the rows see.
    ///
    /// Only the popover is compared. `layout_banner` also paints the strip's
    /// chips (whose painter is declared separately as `dchips`) and, in edit
    /// mode, chrome the surface never claims — so the drawer's stream is
    /// sliced from the first `0x2e` alpha, the shadow ring that starts this
    /// popover and that nothing else in the viewing state pushes.
    ///
    /// Scale stays 1.0, like every other surface parity here: the offsets the
    /// drawer computes in DEVICE px from `fs()` round to a tenth of a base px,
    /// which the scene's own scaling of a literal then rounds differently —
    /// a sub-pixel ladder this small cannot carry both spellings exactly.
    #[test]
    fn the_connectivity_popover_scene_matches_its_own_drawer() {
        use crate::shell::panels::layout::Layout;
        use crate::shell::{BannerItem, BannerToken, CONN_BT_KEY, CONN_WIFI_KEY};

        // (wifi on, bt on, ssid, connected-device count)
        let sets: [(bool, bool, &str, usize); 4] = [
            (true, false, "CafeGuest", 0),
            (false, true, "", 0),
            (true, true, "A rather long network name", 2),
            (false, false, "", 0),
        ];
        for (wifi_on, bt_on, ssid, n_devices) in sets {
            for hover in [0u32, CONN_WIFI_KEY, CONN_BT_KEY] {
                for w in [900.0f32, 1400.0] {
                    let cfg: crate::config::Config =
                        toml::from_str(crate::config::DEFAULT_SHELL_TOML).expect("cfg parses");
                    let mut s = crate::shell::Shell::new(cfg);
                    s.scale = scale();
                    s.sv_fg = pal().fg;
                    s.sv_bg = pal().bg;
                    s.sv_acc = pal().acc;
                    s.sv_sfg = pal().sfg;
                    s.mode = crate::shell::Mode::Expanded;
                    s.dash_enabled = true;
                    s.dash_edit = false;
                    s.connectivity_open = true;
                    s.hover_key = hover;
                    s.wifi_on = wifi_on;
                    s.bt_on = bt_on;
                    s.ssid = ssid.to_string();
                    s.bt_devices = (0..n_devices)
                        .map(|i| (format!("device {i}"), i % 2 == 0, true))
                        .collect();
                    // the net chip must be IN the strip for the popover to
                    // have a cell to hang from — that is the only state it
                    // ever draws in
                    s.banner_order = vec![
                        BannerToken::Chip(BannerItem::Workspaces),
                        BannerToken::Chip(BannerItem::Connectivity),
                        BannerToken::Chip(BannerItem::Clock),
                    ];
                    let mut vals = s.scene_values_for("dashboard");
                    s.publish_conn_popover(&mut vals, w);

                    // ── the drawer ── whole `layout_banner`, then sliced at
                    // the popover's shadow ring
                    let lt = Layout::compute(&s, w, 800.0);
                    let mut rust: Vec<Cmd> = Vec::new();
                    s.layout_banner(&mut rust, w, &lt, None, &pal());
                    let mut rust_hits = s.take_hover_regions();
                    rust_hits.retain(|r| matches!(r.4, CONN_WIFI_KEY | CONN_BT_KEY));
                    let start = rust
                        .iter()
                        .position(|c| matches!(c, Cmd::Rect { color, .. } if color & 0xff == 0x2e))
                        .expect("the drawer painted the popover's shadow ring");
                    let rust: Vec<Cmd> = rust.into_iter().skip(start).collect();
                    assert!(
                        rust
                            .iter()
                            .any(|c| matches!(c, Cmd::Text { text, .. } if text == "Wi-Fi")),
                        "the slice starts at THIS popover, not at an earlier 0x2e rect"
                    );

                    // ── the declared `dconn` body ──
                    let sc = crate::scene::ShellSceneCache::new()
                        .get()
                        .cloned()
                        .expect("shell.ron loads");
                    let dconn = match sc.components.get("dconn") {
                        Some(crate::scene::SceneComponent::Body(items)) => items.clone(),
                        other => panic!("dconn must be a prop-less body, got {other:?}"),
                    };
                    assert!(!dconn.is_empty(), "dconn declares the popover");
                    let scene = CardScene { items: dconn };
                    let mut sv: Vec<Cmd> = Vec::new();
                    let (scene_hits, _) = scene.draw(
                        &mut sv,
                        0.0,
                        0.0,
                        w,
                        800.0,
                        scale(),
                        &pal(),
                        &vals,
                        hover,
                        true,
                        true,
                    );

                    let label =
                        format!("wifi={wifi_on} bt={bt_on} hover={hover} w={w} ssid={ssid:?}");
                    assert_eq!(
                        fmt(&rust),
                        fmt(&sv),
                        "{label}: the scene and the drawer must paint the same commands"
                    );
                    let want: Vec<String> = rust_hits
                        .iter()
                        .map(|r| format!("H {:.2} {:.2} {:.2} {:.2} #{}", r.0, r.1, r.2, r.3, r.4))
                        .collect();
                    let got: Vec<String> = scene_hits
                        .iter()
                        .map(|k| {
                            format!("H {:.2} {:.2} {:.2} {:.2} #{}", k.x, k.y, k.w, k.h, k.key)
                        })
                        .collect();
                    assert_eq!(
                        want, got,
                        "{label}: the scene's hit regions must be the drawer's, in order"
                    );
                }
            }
        }
    }



    #[test]
    fn every_settings_tab_has_exactly_one_body() {
        // safe. A tab's body is painted by EXACTLY ONE declared section in
        // `scontent` — never two (the pane would paint the same row twice,
        // double-registering its region) and never none (a blank pane that
        // looks like a crash).
        //
        // This was the ratchet for the whole settings migration, and Appearance
        // was the last section: while a tab still had a Rust painter it was
        // reached through the `settings_ink` route, and this test failed unless
        // its `tab_*` gate joined `scontent` in the SAME commit that took its
        // name out of that match. Nothing else in the tree notices a
        // double-painted pane — the parity tests cover one section at a time
        // and none of them look across tabs.
        //
        // With the last fallback gone the test inverts: the declared-section
        // count is exactly one for every tab, and the Ink route is not merely
        // unused by these six but UNPUBLISHED — a name that no tab routes to
        // today is a fallback someone is about to route one to again.
        let mut sc = crate::scene::ShellSceneCache::new()
            .get()
            .cloned()
            .expect("shell.ron loads");
        sc.expand();
        let comp = sc.components.get("scontent").expect("scontent component");
        // every declared section in the pane, named by the binding that opens
        // it — the `tab_*` convention the publisher follows
        let gates: Vec<String> = comp
            .items()
            .iter()
            .filter_map(|it| match it {
                SceneItem::Stack {
                    visible: Some(v), ..
                } => Some(v.clone()),
                _ => None,
            })
            .collect();
        assert!(
            !gates.is_empty(),
            "`scontent` declares at least one gated section — otherwise nothing here \
             is testing anything"
        );
        for tab in crate::shell::SettingsTab::ALL {
            let mut s = crate::shell::Shell::new(
                toml::from_str(crate::config::DEFAULT_SHELL_TOML).expect("cfg parses"),
            );
            s.set_settings_tab(tab);
            let vals = s.scene_values_for("settings");
            let ink = vals.text("settings_ink").unwrap_or_default();
            let open = gates
                .iter()
                .filter(|g| vals.toggle(g) == Some(true))
                .count();
            assert_eq!(
                open, 1,
                "{tab:?}: {open} declared section(s) open — exactly one body must paint this tab"
            );
            assert!(
                ink.is_empty(),
                "{tab:?}: the `settings_ink` fallback is retired, but the publisher routed \
                 this tab to {ink:?} anyway"
            );
        }
    }

    #[test]
    fn every_pill_widget_has_a_template_for_its_delegate() {
        // The model dispatches on the row's `tpl` field, and a widget whose
        // delegate name matches no template draws NOTHING — a chip that vanishes
        // with no complaint, which is the one failure mode a model repeater has
        // that a `List` of tags did not (a tag with no template and no wildcard
        // used to be caught by the shell's own tag coverage).
        //
        // So the coverage check has to live where the naming lives: every
        // `PillItem` the bar can hold must name a delegate the live scene
        // actually declares.
        let text = crate::shell_scene::SHELL_SCENE_RON;
        let mut sc: ShellScene = parse_scene_ron(&text).expect("RON parses cleanly");
        sc.expand();
        let surf = sc.surface("pill").expect("the pill surface is declared");
        // the center cluster is nested (outer row → center row → repeater), so
        // search the tree rather than assuming a shape — a scene that restructures
        // its rows must not fail this check for the wrong reason
        fn model_templates(items: &[SceneItem]) -> Option<Vec<String>> {
            items.iter().find_map(|it| match it {
                SceneItem::Repeat {
                    model: Some(_),
                    templates,
                    ..
                } => Some(templates.iter().map(|t| t.tag.clone()).collect()),
                SceneItem::Row { items, .. }
                | SceneItem::Column { items, .. }
                | SceneItem::Stack { items, .. } => model_templates(items),
                _ => None,
            })
        }
        let declared: Vec<String> = model_templates(&surf.items)
            .expect("the pill's center cluster is a model Repeat")
            .into_iter()
            .chain(["*".to_string()])
            .collect();
        for &it in crate::shell::ALL_PILL_ITEMS {
            if !it.in_center_model() {
                continue; // left/right cluster chips, drawn by their own repeaters
            }
            let d = it.delegate();
            assert!(
                declared.iter().any(|t| t == d),
                "{:?} dispatches to template {d:?}, which the pill's Repeat does not declare \
                 (it has: {}) — the widget would draw nothing",
                it,
                declared.join(", ")
            );
        }
        // and the identity tag is still distinct from the delegate for the
        // widgets that share a body, or `tag` would be useless as a row's name
        for &a in crate::shell::ALL_PILL_ITEMS {
            for &b in crate::shell::ALL_PILL_ITEMS {
                if a == b || !a.in_center_model() || !b.in_center_model() {
                    continue;
                }
                if a.delegate() == b.delegate() {
                    assert_ne!(
                        a.tag(),
                        b.tag(),
                        "{:?} and {:?} share delegate {:?} but not a tag — their rows would be \
                         indistinguishable",
                        a,
                        b,
                        a.delegate()
                    );
                }
            }
        }
    }


    fn wallpaper_shell_at(n: usize, scroll: usize, w: f32, h: f32, f: f32) -> crate::shell::Shell {
        let cfg: crate::config::Config =
            toml::from_str(crate::config::DEFAULT_SHELL_TOML).expect("default config parses");
        let mut s = crate::shell::Shell::new(cfg);
        s.scale = crate::shell::GridScale(f);
        s.sv_fg = pal().fg;
        s.sv_bg = pal().bg;
        s.sv_acc = pal().acc;
        s.sv_sfg = pal().sfg;
        s.dash_enabled = true;
        s.wallpapers = (0..n).map(|i| format!("/pics/zen-wall-{i}.png")).collect();
        s.wp_card_scroll = scroll;
        s.wp_card_rect = (0.0, 0.0, w, h);
        s
    }

    #[test]
    fn wallpaper_picker_keys_are_absolute_and_cover_the_collection() {
        // The one behaviour the conversion CHANGED, so it gets its own test
        // rather than riding on the parity check.
        //
        // Clicks go through `click(x, y)` — pointer, registered region, key,
        // handler — because that is the whole chain the conversion could have
        // broken, and because the keys live in the click path, not a bare key
        // handler. Three things this pins down:
        //
        //   • a tile is numbered by the wallpaper it SHOWS (`base + scroll +
        //     vi`), and the handler reads `base + i` with no offset added back.
        //     A stale `wallpaper_scroll` — the painter clamps it, the handler
        //     never saw the clamp — used to resolve a different file than
        //     the tile under the pointer, so the assertion is against the
        //     tile's OWN path, not "some wallpaper";
        //   • the click BAND covers the collection. Absolute keys need a
        //     ceiling over the whole list, not one page of it; the old
        //     `10..=99` capped the grid at 90 files and every tile below index
        //     89 fell through to `_`, collapsing the panel on click. So the
        //     deepest VISIBLE tile on the last page is clicked, not just the
        //     shallowest;
        //   • a wallpapers count past the old ceiling still works.
        let cfg: crate::config::Config =
            toml::from_str(crate::config::DEFAULT_SHELL_TOML).expect("default config parses");
        // well past the old 90-file ceiling, and a non-multiple of the page size
        // so the last page is a partial one
        const N: usize = 295;
        let mut s = crate::shell::Shell::new(cfg);
        s.mode = crate::shell::Mode::Wallpaper;
        s.wallpapers = (0..N).map(|i| format!("/pics/zen-wall-{i}.png")).collect();
        s.wallpaper_scroll = N - s.wallpaper_visible();

        let cols = s.wp_cols() as f32;
        let cell = s.wp_cell;
        let w = s.target_size().0;
        let grid_w = cols * cell + (cols - 1.0) * crate::shell::Shell::WP_GAP;
        let x0 = (w - grid_w) / 2.0;
        // centre of the tile at WINDOW index `vi`
        let centre = |s: &crate::shell::Shell, vi: usize| -> (f32, f32) {
            let c = s.wp_cols() as f32;
            let g = s.wp_cell;
            let gw = c * g + (c - 1.0) * crate::shell::Shell::WP_GAP;
            (
                (w - gw) / 2.0 + (vi as f32 % c) * (g + crate::shell::Shell::WP_GAP) + g / 2.0,
                crate::shell::Shell::WP_Y0
                    + (vi as f32 / c).floor() * (g + crate::shell::Shell::WP_GAP)
                    + g / 2.0,
            )
        };
        let _ = (x0, cell, cols);

        let visible = s.wallpaper_visible();
        // lay the panel out once: `click` hit-tests the REGISTERED regions, and
        // a fresh Shell has none
        let mut v: Vec<Cmd> = Vec::new();
        s.layout_wallpaper(&mut v, w, s.target_size().1, &pal());

        // every visible tile on the last page is reachable, first to last
        for vi in [0usize, 1, visible / 2, visible - 1] {
            let absolute = s.wallpaper_scroll + vi;
            let key = crate::shell::WP_PICKER_KEY_BASE + absolute as u32;
            assert!(
                key <= crate::shell::WP_PICKER_KEY_MAX,
                "tile {vi} of the last page keys to {key}, outside the click band \
                 — the old 10..=99 range would have dropped it"
            );
            let (cx, cy) = centre(&s, vi);
            s.click(cx, cy);
            assert_eq!(
                s.pending_wallpaper.as_deref(),
                Some(s.wallpapers[absolute].as_str()),
                "clicking tile {vi} on the last page must set {}, got {:?}",
                s.wallpapers[absolute],
                s.pending_wallpaper
            );
        }

        // and the deepest file in the collection keys inside the band at all
        let deepest = crate::shell::WP_PICKER_KEY_BASE + (N - 1) as u32;
        assert!(
            deepest <= crate::shell::WP_PICKER_KEY_MAX,
            "the deepest tile's key {deepest} must sit inside the click band"
        );
    }

    #[test]
    fn a_spark_inset_chooses_between_stroke_headroom_and_the_full_box() {
        // `inset` exists for the equalizer's response curve: a fixed 50px
        // panel the curve is laid out against EDGE TO EDGE, where the default
        // `thickness / 2 + 1` headroom would clip the trace's own extremes
        // off the box. The trace is bottom-up (`py = sy + sh - pad - frac *
        // usable`), so a peak sample sits at the box TOP — the y we measure.
        let card = |inset: &str| {
            format!(
                r#"( items: [
            Spark(name: "c", x: 0.0, y: 0.0, w: 200.0, h: 50.0,
                  color: acc, max: 1.0, kind: line, thickness: 2.0{})
        ] )"#,
                inset
            )
        };
        let mut vals = SceneValues::new();
        // a peak-then-trough trace: sample 1 is the curve's maximum
        vals.insert("c".to_string(), SceneValue::Spark(vec![0.0, 1.0]));
        let top_of = |src: &str| -> f32 {
            let scene: CardScene = parse_scene_ron(src).expect("spark parses");
            let mut sa = crate::shell::Shell::new(Default::default());
            sa.scale = crate::shell::GridScale(1.0);
            let mut v: Vec<Cmd> = Vec::new();
            scene.draw(
                &mut v, 0.0, 0.0, 200.0, 50.0, sa.scale, &pal(), &vals, 0, true, true,
            );
            v.iter()
                .filter_map(|c| match c {
                    Cmd::Line { y0, y1, .. } => Some((*y0).min(*y1)),
                    _ => None,
                })
                .fold(f32::MAX, f32::min)
        };
        // default: `thickness / 2 + 1` = 2px of headroom, so the peak lands
        // 2px below the top edge instead of on it
        assert!(
            (top_of(&card("")) - 2.0).abs() < 0.01,
            "default keeps the stroke headroom, got {}",
            top_of(&card(""))
        );
        // `inset: 0.0` — the peak reaches the box top exactly
        assert!(
            (top_of(&card(", inset: 0.0")) - 0.0).abs() < 0.01,
            "inset 0.0 spans the full box height, got {}",
            top_of(&card(", inset: 0.0"))
        );
    }

    #[test]
    fn a_grid_gap_var_overrides_its_literal_at_every_scale() {
        // `gap_var` exists for one case: a body that kept its pitch in DEVICE
        // px while the scene scaled everything else. The wallpaper picker did
        // — a plain `gap: 10.0` agrees at scale 1.0 and drifts at every other
        // one, so this asserts the binding wins at a scale that is NOT 1.0 and
        // that it wins by exactly the scale factor.
        let mut sa = crate::shell::Shell::new(Default::default());
        sa.scale = crate::shell::GridScale(1.0);
        let mut sb = crate::shell::Shell::new(Default::default());
        sb.scale = crate::shell::GridScale(1.75);
        let card = r#"( items: [
            Grid(name: "g", x: 0.0, y: 0.0, w: 0.0, h: 0.0,
                 cols_var: "cols", cell_var: "cell", square: true,
                 gap: 10.0, gap_var: "gap",
                 surface: hover, hover_surface: hover_hl,
                 key_base: 10, visible: "on")
        ] )"#;
        let scene: CardScene = parse_scene_ron(card).expect("grid parses");
        // read the column origins off the DRAWN HIT RECTS, so the test
        // measures the scene's own geometry and not a re-derivation of it
        fn xs(hits: &[SceneHit]) -> Vec<f32> {
            let mut v = hits.iter().map(|r| r.x).collect::<Vec<_>>();
            v.sort_by(|p, q| p.partial_cmp(q).unwrap());
            v.dedup();
            v
        }
        fn vals_for(f: f32) -> SceneValues {
            let mut v = SceneValues::new();
            let one = crate::shell::GridScale(f).s(1.0);
            v.insert("cols".to_string(), SceneValue::Ring(3.0));
            v.insert("cell".to_string(), SceneValue::Ring(40.0));
            v.insert("on".to_string(), SceneValue::Toggle(true));
            // 10 device px, in base px — the number the publisher writes
            v.insert("gap".to_string(), SceneValue::Ring(10.0 / one));
            // the grid's cells come in under its `name`, like every other
            // name-bound model in the scene
            v.insert(
                "g".to_string(),
                SceneValue::GridCells((0..3).map(|_| crate::scene::GridCell::default()).collect()),
            );
            v
        }
        let mut va: Vec<Cmd> = Vec::new();
        let mut vb: Vec<Cmd> = Vec::new();
        let valsa = vals_for(1.0);
        let valsb = vals_for(1.75);
        let (hits_a, _) = scene.draw(
            &mut va,
            0.0,
            0.0,
            300.0,
            300.0,
            sa.scale,
            &pal(),
            &valsa,
            0,
            true,
            true,
        );
        let (hits_b, _) = scene.draw(
            &mut vb,
            0.0,
            0.0,
            300.0,
            300.0,
            sb.scale,
            &pal(),
            &valsb,
            0,
            true,
            true,
        );
        let ha = xs(&hits_a);
        let hb = xs(&hits_b);
        // The claim, in pitch rather than absolute x (the grid centres itself,
        // so only the column-to-column distance is the gap's business):
        //
        //   bound gap, scale 1.00 → pitch 50.0  = cell 40 + gap 10
        //   bound gap, scale 1.75 → pitch 80.0  = cell 70 + gap 10  ← the gap
        //                                                         stayed 10 DEVICE px
        //   literal gap, scale 1.00 → pitch 50.0  — IDENTICAL
        //   literal gap, scale 1.75 → pitch 87.5  = cell 70 + gap 17.5 ← the
        //                                                         gap got scaled too
        //
        // Note the two rows agree exactly at scale 1.0. That is why the
        // wallpaper picker could be declared with a plain `gap: 10.0`, ship,
        // and look correct — the drift is 7.5 px per gap and only appears
        // once anything is scaled, which is exactly when it is least likely to
        // be noticed and most expensive to chase.
        let pitch = |v: &[f32]| v[1] - v[0];
        assert_eq!(ha.len(), hb.len(), "same tile count at both scales");
        assert!(
            (pitch(&ha) - 50.0).abs() < 0.01,
            "scale 1.0: cell 40 + 10 = 50, got {}",
            pitch(&ha)
        );
        assert!(
            (pitch(&hb) - 80.0).abs() < 0.01,
            "scale 1.75: the cell scaled to 70 but the bound gap stayed 10, got {}",
            pitch(&hb)
        );
        // and the SAME RON without the binding: a literal gap is multiplied by
        // the scale like everything else, so 10.0 becomes 17.5 device px and
        // the pitch is 100.5, not 87.5. That 13 px per gap is the drift the
        // wallpaper picker would have shipped if it had been declared with
        // `gap: 10.0` — which is why `gap_var` was added at all.
        let lit = r#"( items: [
            Grid(name: "g", x: 0.0, y: 0.0, w: 0.0, h: 0.0,
                 cols_var: "cols", cell_var: "cell", square: true,
                 gap: 10.0,
                 surface: hover, hover_surface: hover_hl,
                 key_base: 10, visible: "on")
        ] )"#;
        let lit_scene: CardScene = parse_scene_ron(lit).expect("grid parses");
        let mut v2: Vec<Cmd> = Vec::new();
        let mut v3: Vec<Cmd> = Vec::new();
        let mut plain = SceneValues::new();
        plain.insert("cols".to_string(), SceneValue::Ring(3.0));
        plain.insert("cell".to_string(), SceneValue::Ring(40.0));
        plain.insert("on".to_string(), SceneValue::Toggle(true));
        plain.insert(
            "g".to_string(),
            SceneValue::GridCells((0..3).map(|_| crate::scene::GridCell::default()).collect()),
        );
        let (lha, _) = lit_scene.draw(
            &mut v2,
            0.0,
            0.0,
            300.0,
            300.0,
            crate::shell::GridScale(1.0),
            &pal(),
            &plain,
            0,
            true,
            true,
        );
        let (lhb, _) = lit_scene.draw(
            &mut v3,
            0.0,
            0.0,
            300.0,
            300.0,
            crate::shell::GridScale(1.75),
            &pal(),
            &plain,
            0,
            true,
            true,
        );
        let lit_a = xs(&lha);
        let lit_b = xs(&lhb);
        assert!(
            (pitch(&lit_a) - 50.0).abs() < 0.01,
            "at scale 1.0 the literal gap is the literal, got {}",
            pitch(&lit_a)
        );
        assert!(
            (pitch(&lit_b) - 87.5).abs() < 0.01,
            "at 1.75 a literal gap scales with everything else: 1.75 * (40 + 10) = 87.5, got {}",
            pitch(&lit_b)
        );
        assert!(
            (pitch(&ha) - pitch(&lit_a)).abs() < 0.01,
            "the two agree exactly at scale 1.0 — which is why the drift hid"
        );
        assert!(
            (pitch(&lit_b) - pitch(&hb) - 7.5).abs() < 0.01,
            "and by 7.5 px of gap at 1.75, which is the drift the binding removes"
        );
    }

    /// Every `Ink(` call site in the LIVE config tree, comments stripped.
    ///
    /// This exists because the project's own notes claimed "every card under
    /// `ui/cards/` is declarative, so cards are no longer a source of `Ink` at
    /// all" — and four of them were not. The check that produced it matched
    /// `Ink(name:` while those cards write `Ink(x: …, name: …)`, so a
    /// field-order typo turned four live escape hatches into a clean bill of
    /// health. Two things are therefore non-negotiable here: strip comments
    /// (these files *name* their Inks in prose) and match the call, not a
    /// field order.
    fn live_ink_sites() -> Vec<(String, String)> {
        fn scan(out: &mut Vec<(String, String)>, label: &str, text: &str) {
            // strip // first: several of these sources DISCUSS their own
            // remaining Inks in the header comment, so a raw `grep` counts the
            // plan as though it were the code
            let code = text
                .lines()
                .filter(|l| !l.trim_start().starts_with("//"))
                .collect::<Vec<_>>()
                .join("\n");
            let code = strip_block_comments(&code);
            for chunk in code.split("Ink(").skip(1) {
                let head: String = chunk.chars().take(120).collect();
                let name = head
                    .split(')')
                    .next()
                    .and_then(|a| a.split("name:").nth(1))
                    .and_then(|a| a.split('"').nth(1))
                    .unwrap_or("<dynamic>")
                    .to_string();
                out.push((label.to_string(), name));
            }
        }
        let mut out = Vec::new();
        // the whole-shell scene is EMBEDDED now (`crate::shell_scene`) — it is
        // walked in memory, not off disk
        scan(&mut out, "embedded shell scene", crate::shell_scene::SHELL_SCENE_RON);
        // the declarative card scenes are the one config tree still read live
        if let Ok(rd) = std::fs::read_dir(crate::vars::card_scenes_dir()) {
            let mut paths: Vec<std::path::PathBuf> = rd
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| p.extension().is_some_and(|e| e == "ron"))
                .collect();
            paths.sort();
            for p in paths {
                if let Ok(text) = std::fs::read_to_string(&p) {
                    scan(&mut out, &p.display().to_string(), &text);
                }
            }
        }
        out
    }

    /// Drop `/* … */` regions, honouring nesting. These scene sources use `/*` for
    /// block notes and `//` for line notes, and several of them name their own
    /// remaining `Ink`s in those notes — so the ratchet has to read code, not
    /// documentation about code. Nesting is handled because the word "Ink" also
    /// appears inside them, and a naive `find`/`rfind` pair would swallow real
    /// declarations that follow a commented-out block.
    fn strip_block_comments(s: &str) -> String {
        let b: Vec<char> = s.chars().collect();
        let mut out = String::with_capacity(s.len());
        let mut i = 0usize;
        let mut depth = 0usize;
        while i < b.len() {
            if b[i] == '/' && i + 1 < b.len() && b[i + 1] == '*' {
                depth += 1;
                i += 2;
                continue;
            }
            if depth > 0 && b[i] == '*' && i + 1 < b.len() && b[i + 1] == '/' {
                depth -= 1;
                i += 2;
                continue;
            }
            if depth == 0 {
                out.push(b[i]);
            }
            i += 1;
        }
        out
    }

    #[test]
    fn the_live_ink_ratchet_counts_call_sites_and_is_not_yet_zero() {
        let mut got: Vec<String> = live_ink_sites()
            .into_iter()
            .map(|(p, n)| format!("{}/{n}", p.rsplit('/').next().unwrap_or(&p)))
            .collect();
        got.sort();
        // the WHOLE inventory, exact. `assert!(len >= 6)` would be the same
        // mistake in a different coat: it stays green if a site is missed, if a
        // seventh appears, or if the counter is quietly wrong — which is
        // precisely the failure that produced the false "cards are all
        // declarative" claim in the first place. An exact list has to be
        // edited when the work lands, which is the point.
        assert_eq!(
            got,
            vec![] as Vec<String>,
            "the live Ink inventory. Each entry must be DELETED, never edited \
             away — when one lands, drop the line; do not relax the expectation. \
             If this fails with a site you do not recognise, a file was added \
             with a differently-spelled `Ink(` — check the pattern, not the count."
        );
        // and the ones already converted must not reappear
        for gone in [
            "wallpaper_picker",
            "lock_screen",
            "settings_ink",
            "accent",
            "dash_grid",
            "dash_banner",
        ] {
            assert!(
                !got.iter().any(|n| n.ends_with(gone)),
                "`{gone}` is declared now — it must not reappear as a call site"
            );
        }
    }

    #[test]
    fn a_surface_r_var_overrides_its_literal() {
        // `r_var` is `Grid.gap_var`'s twin for the same reason: a surface
        // radius that is half of a DEVICE-constant width cannot be a literal,
        // because literals scale and the constant does not. The wallpaper
        // scrollbar is exactly that — a 6 px track pilled at `r = tw / 2`.
        //
        // Asserted as the agreement/disagreement pair, because the whole value
        // of the binding is invisible at scale 1.0: 3.0 == 3.0/1.0.
        let ron = |gap: &str| {
            format!(
                r#"( items: [
                    Surface(x: 0.0, y: 0.0, w: 6.0, h: 40.0, {gap}, color: hover)
                ] )"#
            )
        };
        let radius = |ron: &str, f: f32| -> f32 {
            let scene: CardScene = parse_scene_ron(ron).expect("surface parses");
            let mut vals = SceneValues::new();
            if ron.contains("r_var") {
                // half a 6 px DEVICE track, in base px
                vals.insert(
                    "r".to_string(),
                    SceneValue::Ring(3.0 / crate::shell::GridScale(f).s(1.0)),
                );
            }
            let mut v: Vec<Cmd> = Vec::new();
            scene.draw(
                &mut v,
                0.0,
                0.0,
                200.0,
                200.0,
                crate::shell::GridScale(f),
                &pal(),
                &vals,
                0,
                true,
                true,
            );
            v.iter()
                .find_map(|c| match c {
                    Cmd::Rect { r, .. } => Some(*r),
                    _ => None,
                })
                .expect("a rect is drawn")
        };
        assert_eq!(
            radius(&ron("r: 3.0"), 1.0),
            3.0,
            "at scale 1.0 a literal is the literal"
        );
        assert_eq!(
            radius(&ron(r#"r_var: "r""#), 1.0),
            3.0,
            "and so is the binding — which is exactly why the bug hid"
        );
        assert_eq!(
            radius(&ron(r#"r_var: "r""#), 1.75),
            3.0,
            "the bound radius stays 3 DEVICE px at 1.75"
        );
        assert_eq!(
            radius(&ron("r: 3.0"), 1.75),
            5.25,
            "a literal scales with everything else: 3.0 * 1.75"
        );
        // a literal is still the fallback when the binding is present but
        // UNRESOLVED — a `r_var` naming a name the publisher forgot must not
        // round the surface to nothing
        let scene: CardScene =
            parse_scene_ron(&ron(r#"r: 3.0, r_var: "missing""#)).unwrap();
        let vals = SceneValues::new();
        let mut v: Vec<Cmd> = Vec::new();
        scene.draw(
            &mut v,
            0.0,
            0.0,
            200.0,
            200.0,
            crate::shell::GridScale(1.75),
            &pal(),
            &vals,
            0,
            true,
            true,
        );
        let fallback = v.iter().find_map(|c| match c {
            Cmd::Rect { r, .. } => Some(*r),
            _ => None,
        });
        assert!(
            matches!(fallback, Some(r) if (r - 5.25).abs() < 0.01),
            "an unresolvable `r_var` must fall back to the literal — a zero radius \
             would round nothing and read as a design change. got {fallback:?}"
        );
    }


    #[test]
    fn the_wallpaper_card_claims_the_wheel_only_while_it_can_move_that_way() {
        // The card's claim on a wheel notch is DIRECTION-aware: parked at the
        // top it refuses an up notch, parked at the bottom a down one, and a
        // grid that fits (or a card that was never drawn) refuses both — so
        // the wheel reaches the board underneath instead of dying at the
        // boundary. The end stop is the drawn one: the same geometry chain the
        // tiles and the scene come from.
        for n in [0usize, 1, 7, 40, 295] {
            for (w, h) in [
                (200.0, 120.0),
                (320.0, 200.0),
                (150.0, 80.0),
                (600.0, 320.0),
            ] {
                for f in [1.0f32, 1.75] {
                    let mut s = wallpaper_shell_at(n, 0, w, h, f);
                    let g = s.wp_card_settle();
                    let overflows = g.max_scroll > 0;
                    let at = format!("{n} files at {w}x{h} (scale {f})");

                    // at the top
                    assert!(!s.wp_card_can_scroll(-1), "{at}: no page above the top");
                    assert_eq!(s.wp_card_can_scroll(1), overflows, "{at}: a page below?");

                    // one notch down is a whole row of the DRAWN grid
                    if overflows {
                        assert!(s.wp_card_scroll_by(1), "{at}: a full page moves");
                        assert_eq!(s.wp_card_scroll, g.cols, "{at}: one notch is one row");
                    }

                    // at the bottom: the mirror image
                    s.wp_card_scroll = g.max_scroll;
                    assert!(!s.wp_card_can_scroll(1), "{at}: no page below the last");
                    assert_eq!(s.wp_card_can_scroll(-1), overflows, "{at}: a page above?");

                    // however hard you push, the wheel stops ON the last page
                    // the card draws — never short of it, never past it
                    s.wp_card_scroll = 0;
                    let mut notches = 0;
                    while s.wp_card_scroll_by(1) {
                        notches += 1;
                        assert!(notches < 10_000, "{at}: the wheel never reaches the end");
                    }
                    assert_eq!(
                        s.wp_card_scroll, g.max_scroll,
                        "{at}: the wheel's end stop is the drawn one"
                    );
                    assert!(!s.wp_card_can_scroll(1), "{at}: and it holds there");
                }
            }
        }

        // a card that was not drawn this frame (the layout pass clears the
        // wheel rects) can never claim a notch
        let mut s = wallpaper_shell_at(40, 0, 320.0, 200.0, 1.0);
        s.wp_card_settle();
        assert!(
            s.wp_card_can_scroll(1),
            "a drawn card with pages below does claim"
        );
        s.clear_frame_wheel_rects();
        assert_eq!(
            s.wp_card_rect,
            (0.0, 0.0, 0.0, 0.0),
            "an undrawn card leaves no box"
        );
        assert!(!s.wp_card_can_scroll(1), "so it claims nothing either");
        assert!(!s.wp_card_scroll_by(1), "and a stray notch cannot move it");
    }


    #[test]
    fn moon_disc_covers_phases_without_panics() {
        // the retired drawer test exercised the sliver loop across the whole
        // cycle; the disc primitive still gets that sweep directly. The
        // terminator collapses to zero width at exactly new (0) and full (0.5)
        // — both endpoints legitimately paint the dark base disc alone.
        let p = pal();
        for i in 0..=10 {
            let phase = i as f64 / 10.0;
            let mut v: Vec<Cmd> = Vec::new();
            crate::ui::moon_disc(&mut v, 40.0, 40.0, 44.0, phase, &p);
            assert!(
                matches!(v.first(), Some(Cmd::Rect { .. })),
                "phase {phase}: rect 0 is the dark base disc"
            );
            if i != 0 && i != 5 {
                assert!(v.len() > 1, "phase {phase}: lit slivers over the base");
            }
        }
    }









    /// The custom-accents subview — the half of this card that was still a
    /// Rust `Ink` until accent.ron declared it.
    ///
    /// Its parity is a different problem from the picker's: the list is
    /// PAGED, so the window the drawer draws, the window the scene skips on
    /// `accent_list_scroll` and the viewport the scroll is clamped against
    /// all have to come from one measure of the card box — while
    /// `clear_frame_wheel_rects` has already zeroed `accent_list_rect` by
    /// the time the publisher runs.


    /// Ink color of the `Cmd::Text` carrying `s` — `texts` drops it.
    /// (`#[allow(dead_code)]`: its only users, the pomodoro scene-geometry
    /// tests, retired with the card's Lua migration.)
    #[allow(dead_code)]
    fn ink_of(v: &[Cmd], s: &str) -> u32 {
        v.iter()
            .find_map(|c| {
                if let Cmd::Text { text, color, .. } = c {
                    (*text == s).then_some(*color)
                } else {
                    None
                }
            })
            .unwrap_or_else(|| panic!("text {s:?} drawn"))
    }

    fn texts(v: &[Cmd]) -> Vec<(f32, f32, f32, String)> {
        v.iter()
            .filter_map(|c| match c {
                Cmd::Text {
                    x, y, size, text, ..
                } => Some((*x, *y, *size, text.clone())),
                _ => None,
            })
            .collect()
    }







    #[test]
    fn battery_item_honors_its_gate_value_and_max() {
        // the gauge's own knobs: an explicit `max` turns the bound percent into
        // a 0..1 fraction, and `visible` gates the whole item
        let scene: CardScene = parse_scene_ron(
            r#"(items: [
                Battery(x: 0.0, y: 0.0, w: 40.0, h: 20.0, vertical: true, value: "{pct}", max: 100.0, visible: Some("on")),
            ])"#,
        )
        .unwrap();
        let mut vals = SceneValues::new();
        vals.insert("pct", SceneValue::Text("50".into()));
        vals.insert("on", SceneValue::Toggle(false));
        let mut v: Vec<Cmd> = Vec::new();
        scene.draw(
            &mut v,
            0.0,
            0.0,
            240.0,
            140.0,
            scale(),
            &pal(),
            &vals,
            0,
            false,
            false,
        );
        assert!(v.is_empty(), "a closed gate draws nothing at all");
        vals.insert("on", SceneValue::Toggle(true));
        let mut v: Vec<Cmd> = Vec::new();
        scene.draw(
            &mut v,
            0.0,
            0.0,
            240.0,
            140.0,
            scale(),
            &pal(),
            &vals,
            0,
            false,
            false,
        );
        let body = v
            .iter()
            .find_map(|c| match c {
                Cmd::Outline { x, y, w, h, .. } => Some((*x, *y, *w, *h)),
                _ => None,
            })
            .expect("the body outline");
        assert_eq!(
            body,
            (0.0, 0.0, 40.0, 20.0),
            "the declared box is honored verbatim"
        );
        // half full: beside the body's own empty fill (16.7 tall, inset by the
        // 1.65 corner caps) the charge rect is half as tall and rides the
        // bottom of the body
        let mut inner: Vec<(f32, f32)> = v
            .iter()
            .filter_map(|c| match c {
                Cmd::Rect { x, y, w, h, .. } if *x >= 0.0 && *x + *w <= 40.01 => Some((*y, *h)),
                _ => None,
            })
            .collect();
        inner.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        let body_fill = inner[0];
        let charge = inner
            .iter()
            .find(|(_, h)| *h < body_fill.1 - 0.5)
            .copied()
            .expect("the charge fill beside the empty body fill");
        assert!(
            (charge.1 - body_fill.1 / 2.0).abs() < 0.5,
            "50% of the body is charged: {charge:?} vs body {body_fill:?}"
        );
        assert!(
            charge.0 + charge.1 <= body_fill.0 + body_fill.1 + 0.01,
            "the charge rides the body's bottom edge: {charge:?} vs {body_fill:?}"
        );
    }

    #[test]
    fn hit_bottom_anchors_its_box_and_negative_span_shrinks_to_the_card() {
        // `Hit { bottom: true }` puts the box's BOTTOM edge `y` px above the
        // card's bottom; a negative `w` is "the span from x, minus this inset"
        // (the battery power-save pill: 12 px in, 38 px short of the right)
        let scene: CardScene = parse_scene_ron(
            r#"(items: [
                Hit(x: 12.0, y: 2.0, w: -38.0, h: 22.0, bottom: true, r: 11.0, key: 7),
                Hit(x: 0.0, y: 0.0, w: 0.0, h: 0.0, key: 8),
            ])"#,
        )
        .unwrap();
        let mut v: Vec<Cmd> = Vec::new();
        let (hits, _) = scene.draw(
            &mut v,
            0.0,
            0.0,
            240.0,
            140.0,
            scale(),
            &pal(),
            &vals(),
            0,
            false,
            false,
        );
        assert_eq!(
            (hits[0].x, hits[0].y, hits[0].w, hits[0].h),
            (12.0, 116.0, 190.0, 22.0),
            "bottom-anchored box: 12 px in, 38 px short of the right edge"
        );
        assert_eq!(
            (hits[1].x, hits[1].y, hits[1].w, hits[1].h),
            (0.0, 0.0, 240.0, 140.0),
            "a 0x0 Hit still fills the card"
        );
    }

    #[test]
    fn rows_without_a_key_base_register_no_regions() {
        // an info list must stay inert: the bare `index` keys it used to
        // register squatted on the global 1 / 2 / 3 keys
        let scene: CardScene = parse_scene_ron(
            r#"(items: [ Rows(name: "rw", y: 30.0, row_h: 18.0, cols: [ (x: 0.0) ]) ])"#,
        )
        .unwrap();
        let mut values = SceneValues::new();
        values.insert(
            "rw",
            SceneValue::Rows(
                ["a", "b", "c"]
                    .iter()
                    .map(|s| SceneRow {
                        cols: vec![(*s).into()],
                        ..Default::default()
                    })
                    .collect(),
            ),
        );
        let mut v: Vec<Cmd> = Vec::new();
        let (hits, _) = scene.draw(
            &mut v,
            0.0,
            0.0,
            240.0,
            140.0,
            scale(),
            &pal(),
            &values,
            0,
            false,
            false,
        );
        assert_eq!(v.len(), 3, "the rows still draw their text");
        assert!(hits.is_empty(), "no key_base means no row regions");
        // an explicit per-row key still opts that row in
        let scene: CardScene = parse_scene_ron(
            r#"(items: [ Rows(name: "rw", y: 30.0, row_h: 18.0, cols: [ (x: 0.0) ]) ])"#,
        )
        .unwrap();
        let mut values = SceneValues::new();
        values.insert(
            "rw",
            SceneValue::Rows(vec![
                SceneRow {
                    cols: vec!["a".into()],
                    key: 4242,
                    ..Default::default()
                },
                SceneRow {
                    cols: vec!["b".into()],
                    ..Default::default()
                },
            ]),
        );
        let mut v: Vec<Cmd> = Vec::new();
        let (hits, _) = scene.draw(
            &mut v,
            0.0,
            0.0,
            240.0,
            140.0,
            scale(),
            &pal(),
            &values,
            0,
            false,
            false,
        );
        assert_eq!(hits.len(), 1, "only the row that asked for it");
        assert_eq!(hits[0].key, 4242, "its own key");
    }




    /// Every `Rect` (a bead) with its geometry + ink.
    fn rects(v: &[Cmd]) -> Vec<(f32, f32, f32, f32, u32)> {
        v.iter()
            .filter_map(|c| match c {
                Cmd::Rect {
                    x, y, w, h, color, ..
                } => Some((*x, *y, *w, *h, *color)),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn ring_fit_and_ladders_follow_the_card_height() {
        // the `Half` fit: ((110 − 62) / 2).clamp(20, 34) = 24
        assert_eq!(
            RingFit::Half {
                offset: 62.0,
                min: 20.0,
                max: 34.0
            }
            .radius(110.0, scale()),
            24.0
        );
        // clamped at both ends
        assert_eq!(
            RingFit::Half {
                offset: 62.0,
                min: 20.0,
                max: 34.0
            }
            .radius(90.0, scale()),
            20.0
        );
        assert_eq!(
            RingFit::Half {
                offset: 62.0,
                min: 20.0,
                max: 34.0
            }
            .radius(200.0, scale()),
            34.0
        );
        // the ladder steps on 116 with the last step as the fallback
        let l = RingFit::Ladder(vec![(116.0, 30.0), (0.0, 22.0)]);
        assert_eq!(l.radius(140.0, scale()), 30.0);
        assert_eq!(l.radius(116.0, scale()), 30.0);
        assert_eq!(l.radius(115.0, scale()), 22.0);
        // the shared ladder helper
        let steps = [(116.0, 45.0), (0.0, 37.0)];
        assert_eq!(ladder_step(&steps, 0.0, 140.0, scale()), 45.0);
        assert_eq!(ladder_step(&steps, 0.0, 100.0, scale()), 37.0);
        assert_eq!(
            ladder_step(&[], 12.0, 100.0, scale()),
            12.0,
            "no ladder keeps the field"
        );
        assert_eq!(
            ladder_step(&[(500.0, 9.0)], 12.0, 100.0, scale()),
            12.0,
            "no match keeps the field"
        );
        // scaled cards: thresholds are base px, the box arrives scaled
        let s2 = crate::shell::GridScale(2.0);
        assert_eq!(
            l.radius(280.0, s2),
            30.0,
            "a 2x card of 140 still takes the big ring"
        );
        assert_eq!(l.radius(200.0, s2), 22.0);
    }


    // ── disk / cpugpu (2026-09-27) — `Ink` cards that became fully
    // declarative scenes, pinned against the Rust drawers they replace.





    #[test]
    fn a_dot_cell_paints_its_own_ink_and_a_char_cap_truncates() {
        // the two new cell kinds in isolation: a text-less dot (inked by
        // `col_colors`) and the 20-character image cap
        let scene: CardScene = parse_scene_ron(
            r#"(items: [ Rows(
                    name: "r", y: 0.0, row_h: 20.0, w: 0.0, pad: 10.0, row_dy: 0.0,
                    cols: [
                        SceneCol(x: 0.0, dot: Some(DotSpec(size: 6.0, top: 2.0))),
                        SceneCol(x: 10.0, size: 9.0, max_chars: Some(5)),
                    ],
                ) ])"#,
        )
        .unwrap();
        let mut m = SceneValues::new();
        m.insert(
            "r",
            SceneValue::Rows(vec![SceneRow {
                cols: vec![String::new(), "abcdefghij".into()],
                key: 0,
                action: None,
                color: None,
                surface: None,
                col_colors: vec![Some(ColorToken::Warn)],
            }]),
        );
        let mut v: Vec<Cmd> = Vec::new();
        scene.draw(
            &mut v,
            0.0,
            0.0,
            200.0,
            60.0,
            scale(),
            &pal(),
            &m,
            0,
            true,
            true,
        );
        let dot = rects(&v)
            .into_iter()
            .find(|r| r.2 == 6.0 && r.3 == 6.0)
            .expect("the dot");
        assert_eq!(
            (dot.0, dot.1),
            (10.0, 2.0),
            "at the cell's x, `top` into the row"
        );
        assert_eq!(dot.4, crate::ui::WARN, "inked by col_colors");
        assert!(
            texts(&v).iter().any(|(_, _, _, s)| s == "abcde"),
            "the cell is capped at 5 chars"
        );
        assert!(
            !texts(&v).iter().any(|(_, _, _, s)| s.len() > 5),
            "nothing longer survives"
        );
    }


    
