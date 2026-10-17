    use super::*;

    /// A shell in edit mode with a single 2×2 card parked at the grid origin.
    fn edit_shell() -> Shell {
        let cfg: Config =
            toml::from_str(crate::config::DEFAULT_SHELL_TOML).expect("default config parses");
        let mut s = Shell::new(cfg);
        s.dash_edit = true;
        s.dash_enabled = true;
        s.dash_layout = vec![CardLayout {
            card: DashCard::System,
            x: 0,
            y: 0,
            w: 2,
            h: 2,
        }];
        s.refresh_pack();
        s
    }

    fn cell_metrics(s: &Shell) -> (f32, f32, f32, f32, f32) {
        let ew = s.cfg.expanded_w();
        (
            s.dash_cell_base_w(ew),
            s.dash_cell_h(ew),
            s.dash_pad(),
            s.dash_gap(),
            s.grid_top(),
        )
    }

    #[test]
    fn press_then_motion_moves_card_and_commits() {
        let mut s = edit_shell();
        let (cw, ch, pad, gap, gt) = cell_metrics(&s);
        // press the card body at its exact top-left corner → zero offsets, so
        // the pointer maps back to whole-cell slots cleanly
        s.edit_press(pad, gt);
        let drag = s
            .edit_drag
            .expect("press on the card body starts a move-drag");
        assert_eq!(drag.0, DashCard::System);
        assert_eq!(drag.1, 0.0, "off_x is 0 (corner press)");
        assert_eq!(drag.2, 0.0, "off_y is 0 (corner press)");
        // drag to the cell at slot (4, 3)
        let nx = pad + 4.0 * (cw + gap);
        let ny = gt + 3.0 * (ch + gap);
        assert!(
            s.edit_motion(nx, ny),
            "motion over a new slot reports a change"
        );
        let l = s
            .dash_layout
            .iter()
            .find(|l| l.card == DashCard::System)
            .unwrap();
        assert_eq!(l.x, 4, "card snaps to the column under the pointer");
        assert_eq!(l.y, 3, "card snaps to the row under the pointer");
        assert!(
            s.edit_ghost_slot.is_some(),
            "a ghost slot previews the move"
        );
        assert!(s.edit_release(nx, ny), "release reports a committed change");
        assert_eq!(s.edit_drag, None, "drag cleared on release");
        assert_eq!(s.edit_ghost_slot, None, "ghost cleared on release");
        let l = s
            .dash_layout
            .iter()
            .find(|l| l.card == DashCard::System)
            .unwrap();
        assert_eq!((l.x, l.y), (4, 3), "position is committed after release");
    }

    #[test]
    fn motion_to_same_slot_is_not_a_change() {
        let mut s = edit_shell();
        let (_, _, pad, _, gt) = cell_metrics(&s);
        s.edit_press(pad, gt);
        // first motion seeds the ghost slot (None → Some) …
        let nx = pad; // stuck on (0,0) — the card's home slot
        let ny = gt;
        assert!(s.edit_motion(nx, ny), "first motion establishes the ghost");
        // … a repeated motion over the SAME slot is then a no-op
        assert!(!s.edit_motion(nx, ny), "same-slot motion reports no change");
        assert_eq!(s.edit_ghost_slot, Some((0, 0)));
    }

    #[test]
    fn close_button_parks_card_on_tray() {
        let mut s = edit_shell();
        let (cw, ch, pad, gap, gt) = cell_metrics(&s);
        let l = s.dash_layout[0];
        let (gx, gy, gw, gh) = Shell::card_close_px(l, cw, ch, gt, pad, gap, s.scale);
        s.edit_press(gx + gw / 2.0, gy + gh / 2.0);
        assert!(
            s.dash_layout.is_empty(),
            "close removes the card from the grid"
        );
        assert_eq!(
            s.tray_cards,
            vec![DashCard::System],
            "and parks it on the tray"
        );
        assert!(s.edit_drag.is_none(), "no drag started by a close");
    }

    #[test]
    fn close_button_hit_rect_follows_board_scroll() {
        // Regression: the ✕ rect used to be computed without the board's
        // scroll offset — with the board panned, the drawn ✕ and its hit
        // rect disagreed and a click started a DRAG instead of parking.
        let mut s = edit_shell();
        let (cw, ch, pad, gap, gt) = cell_metrics(&s);
        s.dash_scroll_x = 11.0;
        s.dash_scroll_y = 37.0;
        let l = s.dash_layout[0];
        let (gx, gy, gw, gh) = s.scrolled(Shell::card_close_px(l, cw, ch, gt, pad, gap, s.scale));
        s.edit_press(gx + gw / 2.0, gy + gh / 2.0);
        assert!(
            s.dash_layout.is_empty(),
            "close parks the card at the scrolled position"
        );
        assert_eq!(
            s.tray_cards,
            vec![DashCard::System],
            "and parks it on the tray"
        );
        assert!(s.edit_drag.is_none(), "no drag started by a close");
    }

    #[test]
    fn grip_press_resizes_and_clamps_to_grid() {
        let mut s = edit_shell();
        let (cw, ch, pad, gap, gt) = cell_metrics(&s);
        let l = s.dash_layout[0];
        let grip = s.grip_px(l);
        s.edit_press(grip.0 + grip.2 / 2.0, grip.1 + grip.3 / 2.0);
        let res = s.edit_resize.expect("grip press starts a resize");
        assert_eq!(res.0, DashCard::System);
        // drag far beyond the grid edge: the card must grow but stay clamped
        let nx = pad + 40.0 * (cw + gap);
        let ny = gt + 40.0 * (ch + gap);
        assert!(s.edit_motion(nx, ny));
        let l = s
            .dash_layout
            .iter()
            .find(|l| l.card == DashCard::System)
            .unwrap();
        assert!(l.w >= 2 && l.h >= 2, "resize grows the card");
        assert!(l.w <= s.grid_max_cols(), "width clamps to the visible grid");
        assert!(s.edit_release(nx, ny));
        assert_eq!(s.edit_resize, None, "resize cleared on release");
    }

    #[test]
    fn release_with_nothing_in_flight_is_noop() {
        let mut s = edit_shell();
        assert!(!s.edit_release(0.0, 0.0), "bare release changes nothing");
        assert!(s.edit_drag.is_none());
        assert!(s.edit_resize.is_none());
    }
