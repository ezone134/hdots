    use super::*;

    #[test]
    fn banner_drop_from_strip_reorders() {
        // n0=5, dragging token from idx 2 and dropping BEFORE idx 0 (k=0):
        // after removing idx2 the target is still 0 → the token lands first
        assert_eq!(Shell::banner_drop_target(2, 0, 5, false), 0);
        // dropping before the ORIGINAL idx 3 (k=3): removal shifted it one
        // left, so it must be inserted at 2
        assert_eq!(Shell::banner_drop_target(2, 3, 5, false), 2);
        // dropping at the tail (k=n0) after removal → insert at 4
        assert_eq!(Shell::banner_drop_target(2, 5, 5, false), 4);
        // dropping at its OWN slot (k=from) → same position, no-op
        assert_eq!(Shell::banner_drop_target(2, 2, 5, false), 2);
    }

    #[test]
    fn banner_drop_from_tray_inserts_in_place() {
        // tray chip (not on the strip): no removal, so the column is used
        // verbatim — k=3 of 5 inserts BEFORE original slot 3
        assert_eq!(Shell::banner_drop_target(0, 3, 5, true), 3);
        // clicking (k=n0) appends at the tail
        assert_eq!(Shell::banner_drop_target(0, 5, 5, true), 5);
        // out-of-range columns clamp to the ends
        assert_eq!(Shell::banner_drop_target(0, 99, 5, true), 5);
        assert_eq!(Shell::banner_drop_target(7, 0, 5, true), 0);
    }

    #[test]
    fn banner_drop_edge_lists() {
        // empty strip: only the tail exists
        assert_eq!(Shell::banner_drop_target(0, 0, 0, false), 0);
        assert_eq!(Shell::banner_drop_target(0, 0, 0, true), 0);
        // single-token strip, move itself around
        assert_eq!(Shell::banner_drop_target(0, 0, 1, false), 0);
        assert_eq!(Shell::banner_drop_target(0, 1, 1, false), 0);
        // overshoot from_idx clamps to the end
        assert_eq!(Shell::banner_drop_target(9, 9, 5, false), 4);
    }

    #[test]
    fn up_step_sits_top_left_in_freed_row() {
        // the documented hard-rule step, in isolation: the card below climbs
        // one row and settles into the TOP-LEFT slot of the row above
        let cards = [CardLayout {
            card: DashCard::Weather,
            x: 0,
            y: 1,
            w: 1,
            h: 1,
        }];
        let (out, moved) = up_step(&cards, 2);
        assert!(moved);
        let b = out[0];
        assert_eq!(
            (b.x, b.y),
            (0, 0),
            "climbs into the row above's top-left slot"
        );
    }

    #[test]
    fn up_step_leftmost_climbs_even_if_column_above_is_taken() {
        // the LEFTMOST card below must climb when the row above has ANY slot
        // that fits — even when its own column above is occupied; it lands at
        // the row above's RIGHTMOST free position
        let cards: Vec<CardLayout> = [
            (DashCard::System, 0, 0, 2, 1),
            (DashCard::Weather, 2, 0, 2, 1),
            (DashCard::CpuGpu, 0, 1, 2, 1),
            (DashCard::Gauges, 4, 1, 2, 1),
        ]
        .map(|(c, x, y, w, h)| CardLayout {
            card: c,
            x,
            y,
            w,
            h,
        })
        .to_vec();
        // one step: row 0 has System + Weather at cols 0..3, cols 4..5 free;
        // CpuGpu (x=0, leftmost below) climbs into the rightmost fitting slot
        let (out, moved) = up_step(&cards, 6);
        assert!(moved);
        let cg = out.iter().find(|l| l.card == DashCard::CpuGpu).unwrap();
        assert_eq!(
            (cg.x, cg.y),
            (4, 0),
            "leftmost-below → rightmost of the row above"
        );
    }

    #[test]
    fn gravity_closes_top_gap_from_below() {
        // delete the top card → the cards below fall UP to refill: the
        // leftmost-from-below card lands in the row-above's RIGHTMOST slot,
        // everything cascades left, and the result is a hole-free pack that
        // fills the top rows before any row below them
        let cards: Vec<CardLayout> = [
            (DashCard::Weather, 0, 1, 1, 1),
            (DashCard::CpuGpu, 0, 2, 1, 1),
            (DashCard::Gauges, 0, 3, 1, 1),
        ]
        .map(|(c, x, y, w, h)| CardLayout {
            card: c,
            x,
            y,
            w,
            h,
        })
        .to_vec();
        let (out, moved) = gravity_pack(&cards, 2);
        assert!(moved);
        let b = out.iter().find(|l| l.card == DashCard::Weather).unwrap();
        let cg = out.iter().find(|l| l.card == DashCard::CpuGpu).unwrap();
        let g = out.iter().find(|l| l.card == DashCard::Gauges).unwrap();
        // top row fills fully (cols 0 and 1) before card 3 goes to row 1
        assert_eq!((b.x, b.y), (0, 0), "leftmost raised card sits top-left");
        assert_eq!(
            (cg.x, cg.y),
            (1, 0),
            "next card lands in the row-above's RIGHT slot"
        );
        assert_eq!((g.x, g.y), (0, 1), "remaining card fills the next row");
        // hole-free: no empty row between cards, nothing below the used rows
        let rows: std::collections::HashSet<u16> = out.iter().map(|l| l.y).collect();
        assert_eq!(rows, [0u16, 1].into_iter().collect());
    }

    #[test]
    fn gravity_left_packs_and_never_overlaps() {
        // gap on row 0 is closed by sliding the right card left; then the
        // below cards fall UP into the freed cells — filling every cell of
        // the top rows — and nothing is ever allowed to overlap
        let cards: Vec<CardLayout> = [
            (DashCard::System, 0, 0, 2, 1),
            (DashCard::Weather, 6, 0, 2, 1),
            (DashCard::CpuGpu, 4, 1, 2, 1),
            (DashCard::Gauges, 6, 1, 2, 1),
        ]
        .map(|(c, x, y, w, h)| CardLayout {
            card: c,
            x,
            y,
            w,
            h,
        })
        .to_vec();
        let (out, moved) = gravity_pack(&cards, 8);
        assert!(moved);
        let mut occ = std::collections::HashSet::new();
        for l in &out {
            for dy in 0..l.h {
                for dx in 0..l.w {
                    assert!(occ.insert((l.x + dx, l.y + dy)), "overlap: {:?}", l.card);
                }
            }
        }
        // everything falls up into row 0 (2-wide each, cols 0-7 in order)
        let s = out.iter().find(|l| l.card == DashCard::System).unwrap();
        let wth = out.iter().find(|l| l.card == DashCard::Weather).unwrap();
        let cg = out.iter().find(|l| l.card == DashCard::CpuGpu).unwrap();
        let g = out.iter().find(|l| l.card == DashCard::Gauges).unwrap();
        assert_eq!((s.x, s.y), (0, 0));
        assert_eq!(
            (cg.x, cg.y),
            (2, 0),
            "below card rose into the row-above gap"
        );
        assert_eq!(
            (wth.x, wth.y),
            (4, 0),
            "right card slid left to stay in order"
        );
        assert_eq!((g.x, g.y), (6, 0), "last card filled the rightmost cell");
        let rows: std::collections::HashSet<u16> = out.iter().map(|l| l.y).collect();
        assert_eq!(
            rows,
            [0u16].into_iter().collect(),
            "everything packed onto row 0"
        );
    }

    #[test]
    fn gravity_is_idempotent_when_packed() {
        let cards: Vec<CardLayout> = [
            (DashCard::System, 0, 0, 2, 1),
            (DashCard::Weather, 2, 0, 2, 1),
        ]
        .map(|(c, x, y, w, h)| CardLayout {
            card: c,
            x,
            y,
            w,
            h,
        })
        .to_vec();
        let (out, moved) = gravity_pack(&cards, 6);
        assert!(!moved, "already packed tight");
        assert_eq!(out.len(), 2);
    }

    #[test]
    fn compact_shifts_left_into_gap() {
        // gap at columns 2-3 on row 0; the right-hand card must slide left
        // into it, others staying put
        let cards: Vec<CardLayout> = [
            (DashCard::System, 0, 0, 2, 1),
            (DashCard::Weather, 4, 0, 2, 1),
            (DashCard::CpuGpu, 2, 1, 2, 1),
        ]
        .map(|(c, x, y, w, h)| CardLayout {
            card: c,
            x,
            y,
            w,
            h,
        })
        .to_vec();
        let (out, moved) = compact_layout(&cards, 6);
        assert!(moved);
        let w = out.iter().find(|l| l.card == DashCard::Weather).unwrap();
        assert_eq!(w.x, 2, "fills the gap left of it");
        let cg = out.iter().find(|l| l.card == DashCard::CpuGpu).unwrap();
        assert_eq!(cg.x, 0, "fills its own (empty) row band's leftmost cells");
        assert_eq!(cg.y, 1, "row band is preserved");
    }

    #[test]
    fn compact_left_is_idempotent() {
        let cards: Vec<CardLayout> = [
            (DashCard::System, 0, 0, 2, 1),
            (DashCard::Weather, 2, 0, 2, 1),
        ]
        .map(|(c, x, y, w, h)| CardLayout {
            card: c,
            x,
            y,
            w,
            h,
        })
        .to_vec();
        let (out, moved) = compact_layout(&cards, 6);
        assert!(!moved, "already packed tight");
        assert!(out
            .iter()
            .all(|l| l.x == if l.card == DashCard::System { 0 } else { 2 }));
    }

    #[test]
    fn compact_tall_card_keeps_its_band() {
        // tall card spans rows 0-1; a gap under/left must never make another
        // card try to occupy its cells
        let cards: Vec<CardLayout> = [
            (DashCard::System, 0, 0, 2, 2),
            (DashCard::Weather, 4, 0, 2, 1),
        ]
        .map(|(c, x, y, w, h)| CardLayout {
            card: c,
            x,
            y,
            w,
            h,
        })
        .to_vec();
        let (out, moved) = compact_layout(&cards, 6);
        assert!(moved);
        let w = out.iter().find(|l| l.card == DashCard::Weather).unwrap();
        assert_eq!(w.x, 2);
        let s = out.iter().find(|l| l.card == DashCard::System).unwrap();
        assert_eq!(s.x, 0);
    }

    #[test]
    fn default_layout_preserved() {
        let layout = default_dash_layout();
        let (packed, _) = pack_cards_max(&layout, DASH_COLS);
        assert_eq!(packed.len(), layout.len(), "card count preserved");
        for (orig, packed_l) in layout.iter().zip(packed.iter()) {
            assert_eq!(orig.card, packed_l.card);
            assert_eq!(orig.x, packed_l.x, "x changed for {:?}", orig.card);
            assert_eq!(orig.y, packed_l.y, "y changed for {:?}", orig.card);
            assert_eq!(orig.w, packed_l.w);
            assert_eq!(orig.h, packed_l.h);
        }
    }

    #[test]
    fn always_fills_leftmost_gap() {
        // Packing hard rule: a card always moves into the LEFT-MOST empty
        // space big enough for it — it never hangs to the right of a hole.
        // Row 0 is fully packed 0..19; a lone card on row 4 slides straight to
        // the far-left first empty cell (x=0), not just beside its committed x.
        let mut cards: Vec<CardLayout> = [
            DashCard::System,
            DashCard::Weather,
            DashCard::Media,
            DashCard::Network,
        ]
        .iter()
        .enumerate()
        .map(|(i, c)| CardLayout {
            card: *c,
            x: i as u16 * 4,
            y: 0,
            w: 4,
            h: 4,
        })
        .collect();
        cards.push(CardLayout {
            card: DashCard::CpuGpu,
            x: 10,
            y: 4,
            w: 2,
            h: 4,
        });
        let (packed, _) = pack_cards_max(&cards, DASH_COLS);
        let cg = packed.iter().find(|l| l.card == DashCard::CpuGpu).unwrap();
        assert_eq!(cg.y, 4, "fits its own (empty) row");
        assert_eq!(cg.x, 0, "fills the left-most empty space of the row");
    }

    #[test]
    fn fills_gap_above_rightmost_slot() {
        // A card whose own row is FULL pops up one row into its RIGHT-most
        // empty slot: row 0 holds cards 0..14 (right gap 15..19), row 1 is
        // packed 0..19 by the order below. The last w=4 card is committed to
        // row 1, can't fit it, so it jumps to row 0's rightmost gap (x=16).
        let mut cards = vec![
            CardLayout {
                card: DashCard::System,
                x: 0,
                y: 0,
                w: 15,
                h: 1,
            },
            CardLayout {
                card: DashCard::Weather,
                x: 0,
                y: 1,
                w: 15,
                h: 1,
            },
            CardLayout {
                card: DashCard::Network,
                x: 15,
                y: 1,
                w: 5,
                h: 1,
            },
        ];
        cards.push(CardLayout {
            card: DashCard::Media,
            x: 0,
            y: 1,
            w: 4,
            h: 1,
        });
        let (packed, _) = pack_cards_max(&cards, DASH_COLS);
        let m = packed.iter().find(|l| l.card == DashCard::Media).unwrap();
        assert_eq!(m.y, 0, "jumped up into the row above");
        assert_eq!(m.x, 16, "landed in the RIGHT-most empty slot of row 0");
    }

    #[test]
    fn packs_within_custom_width() {
        // pack_cards_max packs within a caller-chosen column count.
        let c = CardLayout {
            card: DashCard::System,
            x: 0,
            y: 0,
            w: 30,
            h: 1,
        };
        let (packed, maxr) = pack_cards_max(&[c], 40);
        assert_eq!(packed[0].x, 0);
        assert_eq!(packed[0].w, 30);
        assert_eq!(maxr, 1);
        let (packed_std, _) = pack_cards_max(&[c], DASH_COLS);
        assert_eq!(packed_std[0].w, 20, "standard packer clamps to DASH_COLS");
    }
