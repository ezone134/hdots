    use super::*;

    /// A module value is reachable by `{key}` interpolation, dots and all.
    #[test]
    fn module_values_interpolate_by_their_dotted_key() {
        let mut vals = crate::scene::SceneValues::new();
        merge_module_values(
            &mut vals,
            vec![("mod_clock.time".into(), "16:24:46".into())],
        );
        assert_eq!(vals.text("mod_clock.time"), Some("16:24:46"));
        assert_eq!(
            crate::scene::substitute("{mod_clock.time} o'clock", &vals),
            "16:24:46 o'clock"
        );
    }

    /// A module cannot shadow a built-in: the built-in insert comes later and
    /// overwrites whatever the module published under that name.
    #[test]
    fn built_ins_win_a_key_collision() {
        let mut vals = crate::scene::SceneValues::new();
        merge_module_values(
            &mut vals,
            vec![
                ("clock".into(), "hijacked".into()),
                ("mod_x.v".into(), "kept".into()),
            ],
        );
        vals.insert("clock", crate::scene::SceneValue::Text("22:09:00".into()));
        assert_eq!(vals.text("clock"), Some("22:09:00"), "built-in must win");
        assert_eq!(vals.text("mod_x.v"), Some("kept"), "additive keys survive");
    }

    /// No `$sources/lib` at all is the normal case for anyone who has not
    /// installed a module — it must be a no-op, not a panic.
    #[test]
    fn no_modules_is_a_no_op() {
        let mut vals = crate::scene::SceneValues::new();
        merge_module_values(&mut vals, Vec::new());
        assert_eq!(
            crate::scene::substitute("{mod_clock.time}", &vals),
            "{mod_clock.time}"
        );
    }
