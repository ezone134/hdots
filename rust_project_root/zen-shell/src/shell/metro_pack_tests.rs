    use super::*;

    #[test]
    fn metro_cascades_into_vertical_columns() {
        let cards: Vec<CardLayout> = (0..6)
            .map(|_| CardLayout {
                card: DashCard::Cpu,
                x: 0,
                y: 0,
                w: 2,
                h: 2,
            })
            .collect();
        let (placed, rows) = pack_metro(&cards, 4);
        // col height 4 fits exactly two 2-high cards per column; the ribbon
        // then extends RIGHT: (0,0),(0,2) | (2,0),(2,2) | (4,0),(4,2)
        let want: Vec<(u16, u16)> = vec![(0, 0), (0, 2), (2, 0), (2, 2), (4, 0), (4, 2)];
        let got: Vec<(u16, u16)> = placed.iter().map(|l| (l.x, l.y)).collect();
        assert_eq!(got, want);
        assert_eq!(rows, 4);
        // hard rule: cards never overlap
        let mut occ: std::collections::HashSet<(u16, u16)> = std::collections::HashSet::new();
        for l in &placed {
            for dy in 0..l.h {
                for dx in 0..l.w {
                    assert!(
                        occ.insert((l.x + dx, l.y + dy)),
                        "overlap at {}x{}",
                        l.x + dx,
                        l.y + dy
                    );
                }
            }
        }
    }
