    use super::*;
    use crate::app::App;
    use std::time::Duration;

    /// The hover-expand deadlock (2026-09-14): the final tween commit
    /// 1009.79×494.79 truncates to the acked configure 1009×494 — the
    /// compositor already has that size, so re-sending set_size is a no-op
    /// it never acks. `commit_size` must classify it as already-configured.
    #[test]
    fn final_fractional_tween_size_counts_as_configured() {
        assert!(App::size_is_configured((1009, 494), 1009.7905, 494.79385));
        assert!(App::size_is_configured((302, 38), 302.58002, 38.0));
        assert!(App::size_is_configured((0, 0), 0.4, 0.9));
    }

    /// A genuinely different size must still go out as a real commit.
    #[test]
    fn different_size_is_norconfigured() {
        assert!(!App::size_is_configured((1009, 494), 1010.0, 494.79));
        assert!(!App::size_is_configured((302, 38), 1010.0, 495.0));
    }

    /// Shell side: with `size_pending` set (the never-acked commit),
    /// `tick()` must NOT clear the anim — the surface hasn't been told the
    /// final size yet. This is what stranded the pill/dashboard mid-morph:
    /// anim stuck + every render dropped while pending.
    #[test]
    fn tick_holds_anim_while_size_pending() {
        let cfg: Config =
            toml::from_str(crate::config::DEFAULT_SHELL_TOML).expect("default config parses");
        let mut s = Shell::new(cfg);
        s.start_morph(1009.79, 494.79);
        s.size_pending = true;
        // run well past the 250 ms duration
        let later = Instant::now() + Duration::from_millis(500);
        assert!(s.tick(later).is_none(), "pending commit holds the anim");
        assert!(s.anim.is_some(), "anim must not complete while pending");
        // once the pending clears, the same tick completes and offers the
        // final size — the app layer then classifies it as already-configured
        // and un-sticks instead of re-sending a no-op commit.
        s.size_pending = false;
        assert_eq!(s.tick(later), Some((1009.79, 494.79)));
        assert!(s.anim.is_none(), "anim completes once the commit lands");
    }
