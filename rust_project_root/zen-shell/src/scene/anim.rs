//! Per-binding easing for `Val`s that declare `anim:` (QUICKSHELL_MODEL §8).
//!
//! The table is keyed by the *binding source string*, not by an item path, and
//! that is deliberate. Two items reading `nav.tab` are tracking one value, so
//! they must move together — a per-item key would let them disagree for a
//! frame. And inside a `Repeat`, substitution already rewrote every row's
//! source to its own `item.<field>` spelling, so rows are automatically
//! distinct without threading a path through `draw_piece`.
//!
//! Two rules the rest of the engine depends on:
//!
//! 1. **A target change restarts from the current value**, not from the
//!    original. An interrupted animation therefore never snaps backwards.
//! 2. **The first frame of a key is its target.** There is no "from" to
//!    animate out of — a value that appears already rests where it belongs,
//!    and only *subsequent* target changes animate.
//!
//! Measure never consults this table (see [`crate::scene::Val::num_anim`]).
//! `want_w`/`intrinsic_w` must see the *target*, because an item whose measured
//! width eased toward its target would resize the box that positions it.

use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Targets closer than this are the same value. Without it a binding that
/// wobbles by float noise would retarget every frame and never finish.
const EPS: f32 = 1e-4;

/// The easing shape. `linear` and the `out_*`/`in_out_*` family cover the
/// settle/expand cases; `out_back` overshoots for a springboard snap.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Curve {
    Linear,
    OutCubic,
    InOutCubic,
    OutBack,
}

impl Curve {
    /// Map already-clamped elapsed fraction `[0, 1]` to eased progress.
    pub fn ease(self, t: f32) -> f32 {
        match self {
            Curve::Linear => t,
            Curve::OutCubic => 1.0 - (1.0 - t).powi(3),
            Curve::InOutCubic => {
                if t < 0.5 {
                    4.0 * t * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
                }
            }
            // overshoot ~10% then settle; c1 = 2.70158 is the classic value
            Curve::OutBack => {
                let c1 = 2.70158;
                let c3 = c1 + 1.0;
                1.0 + c3 * (t - 1.0).powi(3) + c1 * (t - 1.0).powi(2)
            }
        }
    }
}

impl Default for Curve {
    fn default() -> Self {
        Curve::OutCubic
    }
}

/// How one animated value moves: `anim: (ms: 180, curve: out_cubic)`.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Anim {
    /// Duration in milliseconds. `0` means "snap", which is the honest way to
    /// spell "no animation" and keeps `anim:` usable on a field whose motion
    /// should sometimes be instant.
    #[serde(default = "Anim::default_ms")]
    pub ms: f32,
    #[serde(default)]
    pub curve: Curve,
}

impl Anim {
    fn default_ms() -> f32 {
        180.0
    }

    pub fn duration(&self) -> Duration {
        Duration::from_secs_f32(self.ms.max(0.0) / 1000.0)
    }
}

impl Default for Anim {
    fn default() -> Self {
        Anim {
            ms: Self::default_ms(),
            curve: Curve::OutCubic,
        }
    }
}

/// One value in flight.
#[derive(Clone, Debug)]
struct Cell {
    /// Where the animation started (its previous target, or where it was when
    /// a mid-flight retarget happened).
    from: f32,
    to: f32,
    start: Instant,
    dur: Duration,
    curve: Curve,
}

impl Cell {
    /// The eased value at `now`, clamped at both ends so a frame that arrives
    /// late still paints the target rather than overshooting into nonsense.
    fn value_at(&self, now: Instant) -> f32 {
        if self.dur.is_zero() {
            return self.to;
        }
        let el = now.saturating_duration_since(self.start).as_secs_f32();
        let t = (el / self.dur.as_secs_f32()).clamp(0.0, 1.0);
        self.from + (self.to - self.from) * self.curve.ease(t)
    }

    fn done(&self, now: Instant) -> bool {
        now.saturating_duration_since(self.start) >= self.dur
    }
}

/// One binding's resting target, plus its animation while one is running.
#[derive(Clone, Debug)]
struct Entry {
    /// The last target this binding was seen resting at. Kept even while
    /// settled, because it is the only way to tell "first frame" from "the
    /// value just changed" — without it every frame would look like a first
    /// sighting and nothing would ever animate.
    target: f32,
    cell: Option<Cell>,
}

/// Every animated binding the scene has read, owned by the shell so it
/// survives the frame boundary that rebuilds the scene pool.
///
/// Bounded by the bindings a scene names, not by frames: one entry per distinct
/// source string, retired by [`Self::clear`] when the scene reloads.
#[derive(Clone, Debug)]
pub struct AnimTable {
    entries: HashMap<String, Entry>,
    /// The frame's clock. Set once per frame by the draw entry point so
    /// `draw_piece` has to thread one extra parameter rather than two, and so
    /// every animated field in a frame is stamped with the SAME instant —
    /// otherwise two fields easing on one item would differ by the microseconds
    /// between their reads.
    now: Instant,
}

impl Default for AnimTable {
    fn default() -> Self {
        AnimTable {
            entries: HashMap::new(),
            now: Instant::now(),
        }
    }
}

impl AnimTable {
    /// Stamp this frame. Everything stepped after this call reads `now`.
    pub fn set_now(&mut self, now: Instant) {
        self.now = now;
    }

    /// The animated value of `key`, easing toward `target`.
    ///
    /// See the module docs for the two rules that make the first frame and a
    /// retarget both behave.
    pub fn step(&mut self, key: &str, target: f32, anim: &Anim) -> f32 {
        let now = self.now;
        let entry = self
            .entries
            .entry(key.to_string())
            .or_insert(Entry { target, cell: None });
        if (entry.target - target).abs() <= EPS {
            // Unchanged: either still running, or resting. A resting entry
            // creates no cell, so a value that never moves costs one map slot
            // and nothing per frame.
            match &entry.cell {
                None => target,
                Some(c) => {
                    if c.done(now) {
                        entry.cell = None;
                        target
                    } else {
                        c.value_at(now)
                    }
                }
            }
        } else {
            // Retarget from where we are *now*, so interrupting an animation
            // never produces a visible backwards jump.
            let from = entry
                .cell
                .as_ref()
                .map_or(entry.target, |c| c.value_at(now));
            entry.target = target;
            entry.cell = Some(Cell {
                from,
                to: target,
                start: now,
                dur: anim.duration(),
                curve: anim.curve,
            });
            from
        }
    }

    /// True while any binding is still moving — the declarative replacement for
    /// the hand-rolled `edit_settling` flag, which existed only to keep the
    /// render loop awake until a hand-computed lerp converged.
    pub fn settling(&self) -> bool {
        self.entries.values().any(|e| e.cell.is_some())
    }

    /// Drop every binding. Called when the scene is reloaded, so a removed
    /// item's entry cannot keep the render loop awake forever.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// How many bindings still have an animation running. Debugging aid; the
    /// render loop uses [`Self::settling`], which cannot be defeated by a table
    /// that holds completed entries.
    #[cfg(test)]
    pub fn in_flight(&self) -> usize {
        self.entries.values().filter(|e| e.cell.is_some()).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(base: Instant, ms: u64) -> Instant {
        base + Duration::from_millis(ms)
    }

    /// The tests drive time explicitly rather than through `set_now`, so each
    /// one can place its own frames without the table's stamp getting in the
    /// way. `step` is the same code path either way.
    impl AnimTable {
        fn step_at(&mut self, key: &str, target: f32, anim: &Anim, now: Instant) -> f32 {
            self.set_now(now);
            self.step(key, target, anim)
        }
    }

    #[test]
    fn a_curve_maps_the_endpoints_to_zero_and_one() {
        for c in [
            Curve::Linear,
            Curve::OutCubic,
            Curve::InOutCubic,
            Curve::OutBack,
        ] {
            assert!((c.ease(0.0)).abs() < 1e-6, "{c:?} must start at 0");
            assert!((c.ease(1.0) - 1.0).abs() < 1e-6, "{c:?} must end at 1");
        }
    }

    #[test]
    fn out_cubic_leads_a_linear_line_and_never_overshoots() {
        let lin = Curve::Linear.ease(0.5);
        let out = Curve::OutCubic.ease(0.5);
        assert!(out > lin, "ease-out is ahead of linear at the midpoint");
        for i in 0..=100 {
            let t = i as f32 / 100.0;
            let v = Curve::OutCubic.ease(t);
            assert!((0.0..=1.0).contains(&v), "{t} eased out of range: {v}");
        }
    }

    #[test]
    fn out_back_overshoots_then_settles() {
        assert!(
            Curve::OutBack.ease(0.7) > 1.0,
            "a springboard overshoots before it lands"
        );
        assert!((Curve::OutBack.ease(1.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn the_first_frame_of_a_key_rests_at_its_target() {
        let now = Instant::now();
        let mut t = AnimTable::default();
        assert_eq!(t.step_at("a", 10.0, &Anim::default(), now), 10.0);
        // and it left nothing in flight, because nothing moved
        assert!(!t.settling());
    }

    #[test]
    fn a_target_change_eases_over_the_declared_duration() {
        let base = Instant::now();
        let anim = Anim {
            ms: 100.0,
            curve: Curve::Linear,
        };
        let mut t = AnimTable::default();
        t.step_at("a", 0.0, &anim, base);
        t.step_at("a", 100.0, &anim, base); // starts the move

        assert!((t.step_at("a", 100.0, &anim, at(base, 50)) - 50.0).abs() < 0.01);
        assert!(
            (t.step_at("a", 100.0, &anim, at(base, 100)) - 100.0).abs() < 0.01,
            "a late frame clamps at the target instead of overshooting"
        );
    }

    #[test]
    fn a_retarget_starts_from_the_current_value() {
        let base = Instant::now();
        let anim = Anim {
            ms: 100.0,
            curve: Curve::Linear,
        };
        let mut t = AnimTable::default();
        t.step_at("a", 0.0, &anim, base);
        t.step_at("a", 100.0, &anim, base);
        let mid = t.step_at("a", 100.0, &anim, at(base, 50));
        assert!((mid - 50.0).abs() < 0.01);

        // interrupt toward a new target: the value must continue from `mid`,
        // not jump back to 0 and not jump to the new target
        let after = t.step_at("a", 0.0, &anim, at(base, 50));
        assert!((after - mid).abs() < 0.01, "continuity across a retarget");
        assert!(t.settling());
    }

    #[test]
    fn a_finished_animation_retires_its_cell() {
        let base = Instant::now();
        let anim = Anim {
            ms: 100.0,
            curve: Curve::Linear,
        };
        let mut t = AnimTable::default();
        t.step_at("a", 0.0, &anim, base);
        t.step_at("a", 100.0, &anim, base);
        assert_eq!(t.in_flight(), 1);
        t.step_at("a", 100.0, &anim, at(base, 150));
        assert_eq!(t.in_flight(), 0, "a settled binding leaves no cell behind");
        assert!(!t.settling());
    }

    #[test]
    fn float_noise_is_not_a_target_change() {
        let base = Instant::now();
        let anim = Anim {
            ms: 100.0,
            curve: Curve::Linear,
        };
        let mut t = AnimTable::default();
        t.step_at("a", 0.0, &anim, base);
        t.step_at("a", 100.0, &anim, base);
        let at10 = t.step_at("a", 100.0, &anim, at(base, 10));
        let at20 = t.step_at("a", 100.0 + 1e-6, &anim, at(base, 20));
        assert!((at10 - 10.0).abs() < 0.01);
        // progress is measured from the animation's own start, so 20 ms in is
        // still 20% — a restart would put it back at ~0
        assert!(
            (at20 - 20.0).abs() < 0.01,
            "a sub-epsilon wobble must not restart the animation, got {at20}"
        );
    }

    #[test]
    fn two_bindings_are_independent() {
        let base = Instant::now();
        let anim = Anim {
            ms: 100.0,
            curve: Curve::Linear,
        };
        let mut t = AnimTable::default();
        t.step_at("a", 0.0, &anim, base);
        t.step_at("b", 0.0, &anim, base);
        t.step_at("a", 100.0, &anim, base);
        assert!((t.step_at("b", 0.0, &anim, at(base, 50))).abs() < 1e-6);
        assert_eq!(t.in_flight(), 1, "only `a` is in flight");
    }

    #[test]
    fn one_binding_shared_by_two_items_moves_together() {
        // This is why the key is the source string: two items reading the
        // same value must not disagree mid-animation.
        let base = Instant::now();
        let anim = Anim {
            ms: 100.0,
            curve: Curve::Linear,
        };
        let mut t = AnimTable::default();
        t.step_at("nav.tab", 0.0, &anim, base);
        t.step_at("nav.tab", 100.0, &anim, base);
        let first_item = t.step_at("nav.tab", 100.0, &anim, at(base, 50));
        let second_item = t.step_at("nav.tab", 100.0, &anim, at(base, 50));
        assert_eq!(first_item, second_item);
    }

    #[test]
    fn a_zero_duration_anim_snaps() {
        let base = Instant::now();
        let anim = Anim {
            ms: 0.0,
            curve: Curve::OutBack,
        };
        let mut t = AnimTable::default();
        t.step_at("a", 0.0, &anim, base);
        t.step_at("a", 50.0, &anim, base);
        assert_eq!(t.step_at("a", 50.0, &anim, base), 50.0);
        assert!(!t.settling());
    }

    #[test]
    fn clear_drops_everything_in_flight() {
        let base = Instant::now();
        let anim = Anim::default();
        let mut t = AnimTable::default();
        t.step_at("a", 0.0, &anim, base);
        t.step_at("a", 100.0, &anim, base);
        assert!(t.settling());
        t.clear();
        assert!(!t.settling());
        assert_eq!(t.in_flight(), 0);
    }

    #[test]
    fn anim_deserializes_bare_curve_and_both_fields() {
        let a: Anim = ron::from_str("(ms: 90)").unwrap();
        assert_eq!(a.ms, 90.0);
        assert_eq!(a.curve, Curve::OutCubic, "curve defaults");

        let b: Anim = ron::from_str("(curve: out_back)").unwrap();
        assert_eq!(b.ms, 180.0, "ms defaults");
        assert_eq!(b.curve, Curve::OutBack);

        let c: Anim = ron::from_str("(ms: 250, curve: in_out_cubic)").unwrap();
        assert_eq!(c.ms, 250.0);
        assert_eq!(c.curve, Curve::InOutCubic);
    }
}
