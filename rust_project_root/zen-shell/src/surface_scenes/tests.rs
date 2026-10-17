    use super::*;

    /// Every retired surface const parses into its `SurfaceScene` and
    /// declares its own name (same serde rules `draw_shell_surface` uses).
    #[test]
    fn every_surface_const_parses() {
        for (name, js) in SURFACE_JSONS {
            let sc = serde_json::from_str::<SurfaceScene>(js)
                .unwrap_or_else(|e| panic!("{name}: {e}"));
            assert_eq!(sc.name, *name, "{name}: const declares its own name");
        }
    }
