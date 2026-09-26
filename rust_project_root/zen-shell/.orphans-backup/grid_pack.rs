//! Metro grid packing: flow packer, gravity packer, ribbon packer,
//! and cell-pitch pixel helpers.

use super::types::*;

// ── default layout ──────────────────────────────────────────────────────────

pub fn default_dash_layout() -> Vec<CardLayout> {
    use DashCard::*;
    // 20x12 lattice -- each card is a multiple of the fine cell, so a 1x1
    // is a true quarter-card tile (the old 10-col grid halved twice).
    [
        (System, 0, 0, 6, 4),
        (Weather, 6, 0, 6, 4),
        (Media, 12, 0, 8, 4),
        (Network, 0, 4, 6, 4),
        (CpuGpu, 6, 4, 6, 4),
        (Gauges, 12, 4, 8, 4),
        (Toggles, 0, 8, 6, 4),
        (Todo, 6, 8, 6, 4),
        (Sliders, 12, 8, 8, 4),
        // row 4
        (Disk, 0, 12, 5, 4),
        (ProcMon, 5, 12, 5, 4),
        (Mem, 10, 12, 5, 4),
        (TopProc, 15, 12, 5, 4),
        // row 5
        (ActiveWin, 0, 16, 5, 4),
        (Clipboard, 5, 16, 5, 4),
        (Notes, 10, 16, 5, 4),
        // row 6
        (PkgUpdates, 0, 20, 5, 4),
        (SshVpn, 5, 20, 5, 4),
        (Docker, 10, 20, 5, 4),
        (KbLayout, 15, 20, 5, 4),
        // row 7 -- full-width scrollable wallpaper strip
        (Wallpaper, 0, 24, 20, 4),
        // row 8 -- news feed + calendar + accent picker
        (News, 0, 28, 10, 4),
        (Calendar, 10, 28, 5, 4),
        (Accent, 15, 28, 5, 4),
        // row 9 -- visualizer + sensors + mirror
        (Viz, 0, 32, 8, 4),
        (Sensors, 8, 32, 6, 4),
        (Mirror, 14, 32, 6, 4),
        // row 10 -- lyrics of the current track (synced when available)
        (Lyrics, 0, 36, 10, 8),
        // row 11 -- wifi / bluetooth / on-demand bandwidth test
        (Wifi, 0, 44, 6, 4),
        (Bluetooth, 6, 44, 6, 4),
        (SpeedTest, 12, 44, 8, 4),
        // row 12 -- quote + currency
        (Quote, 0, 48, 10, 3),
        (Currency, 10, 48, 8, 3),
        // row 13 -- prices ticker + recent files
        (Ticker, 0, 51, 10, 3),
        (Recent, 10, 51, 8, 3),
        // row 14 -- GPU detail + thermal-zones grid
        (Gpu, 0, 55, 6, 4),
        (Thermal, 6, 55, 12, 4),
        // row 15 -- clean clock/date + dual battery cards (vertical & horizontal)
        (Clock, 0, 60, 6, 4),
        (BatteryV, 6, 60, 5, 4),
        (BatteryH, 11, 60, 7, 4),
    ]
    .into_iter()
    .map(|(card, x, y, w, h)| CardLayout { card, x, y, w, h })
    .collect()
}

// ── flow packer ─────────────────────────────────────────────────────────────

/// Flow packer -- cards pack left->right row by row.
pub fn pack_cards(cards: &[CardLayout]) -> (Vec<CardLayout>, u16) {
    pack_cards_max(cards, DASH_COLS)
}

/// Collapse every all-empty ROW of a pinned dashboard layout so cards "fall
/// upward" into free space.
pub fn compact_rows_up(cards: &[CardLayout]) -> Vec<CardLayout> {
    let mut occ: std::collections::HashSet<u16> = std::collections::HashSet::new();
    for l in cards {
        for r in l.y..l.y + l.h {
            occ.insert(r);
        }
    }
    let mut rows: Vec<u16> = occ.into_iter().collect();
    rows.sort_unstable();
    let rank = |y: u16| rows.partition_point(|&r| r < y);
    cards
        .iter()
        .map(|l| CardLayout { card: l.card, x: l.x, y: rank(l.y) as u16, w: l.w, h: l.h })
        .collect()
}

pub fn pack_cards_max(cards: &[CardLayout], max_cols: u16) -> (Vec<CardLayout>, u16) {
    let cols = max_cols.max(1);
    let mut occ: std::collections::HashSet<(u16, u16)> = std::collections::HashSet::new();
    let mut out = Vec::with_capacity(cards.len());
    let mut maxr = 0u16;

    /// Try to place a w*h card on row `try_y`. Left-biased or right-biased.
    fn try_place(
        occ: &std::collections::HashSet<(u16, u16)>,
        try_y: u16, w: u16, h: u16,
        hi: u16, right_biased: bool,
    ) -> Option<(u16, u16)> {
        if try_y + h > DASH_MAX_ROWS { return None; }
        let iter: Box<dyn Iterator<Item = u16>> = if right_biased {
            Box::new((0..=hi).rev())
        } else {
            Box::new(0..=hi)
        };
        for x in iter {
            let mut fits = true;
            for dy in 0..h {
                for dx in 0..w {
                    if occ.contains(&(x + dx, try_y + dy)) {
                        fits = false;
                        break;
                    }
                }
                if !fits { break; }
            }
            if fits { return Some((x, try_y)); }
        }
        None
    }

    for c in cards {
        let w = c.w.clamp(1, cols);
        let h = c.h.max(1);
        let hi = cols.saturating_sub(w);
        let mut placed = false;

        // 1) current row -- left-most empty space
        if let Some((x, y)) = try_place(&occ, c.y, w, h, hi, false) {
            for dy in 0..h { for dx in 0..w { occ.insert((x + dx, y + dy)); } }
            out.push(CardLayout { card: c.card, x, y, w, h });
            maxr = maxr.max(y + h);
            continue;
        }
        // 2) jump up one row -> RIGHTMOST empty slot
        if c.y > 0 {
            let above_has_cards =
                (0..cols).any(|xc| occ.contains(&(xc, c.y - 1)));
            if above_has_cards {
                if let Some((x, y)) = try_place(&occ, c.y - 1, w, h, hi, true) {
                    for dy in 0..h { for dx in 0..w { occ.insert((x + dx, y + dy)); } }
                    out.push(CardLayout { card: c.card, x, y, w, h });
                    maxr = maxr.max(y + h);
                    continue;
                }
            }
        }
        // 3) scan down from the original row, left-filling every row
        let mut y = c.y + 1;
        loop {
            if y + h > DASH_MAX_ROWS { break; }
            if let Some((x, yy)) = try_place(&occ, y, w, h, hi, false) {
                for dy in 0..h { for dx in 0..w { occ.insert((x + dx, yy + dy)); } }
                out.push(CardLayout { card: c.card, x, y: yy, w, h });
                maxr = maxr.max(yy + h);
                placed = true;
                break;
            }
            y += 1;
        }
        if !placed {
            let base = maxr.max(c.y);
            let (x, y) = try_place(&occ, base, w, h, hi, false)
                .or_else(|| try_place(&occ, DASH_MAX_ROWS.saturating_sub(h), w, h, hi, false).into_iter().next())
                .expect("a row below all content is always free");
            for dy in 0..h { for dx in 0..w { occ.insert((x + dx, y + dy)); } }
            out.push(CardLayout { card: c.card, x, y, w, h });
            maxr = maxr.max(y + h);
        }
    }
    (out, maxr)
}

// ── ribbon packer (horizontal scroll mode) ─────────────────────────────────

/// Metro ribbon pack: cards cascade into VERTICAL columns -- a column fills
/// top->bottom, then the next column begins to the RIGHT.
fn pack_metro(cards: &[CardLayout], max_rows: u16) -> (Vec<CardLayout>, u16) {
    use std::collections::HashSet;
    let max_rows = max_rows.max(1);
    let mut occ: HashSet<(u16, u16)> = HashSet::new();
    let mut out = Vec::with_capacity(cards.len());
    let mut max_x = 0u16;
    let mut max_y = 0u16;
    for c in cards {
        let w = c.w.clamp(1, 512);
        let h = c.h.max(1);
        let mut placed = None;
        'cols: for x in 0..512u16 {
            for y in 0..=max_rows.saturating_sub(h) {
                let mut fits = true;
                for dy in 0..h {
                    for dx in 0..w {
                        if x + dx >= 512 || occ.contains(&(x + dx, y + dy)) {
                            fits = false;
                            break;
                        }
                    }
                    if !fits { break; }
                }
                if fits {
                    placed = Some((x, y));
                    break 'cols;
                }
            }
        }
        let (x, y) = placed.unwrap_or((max_x, 0));
        for dy in 0..h {
            for dx in 0..w {
                occ.insert((x + dx, y + dy));
            }
        }
        max_x = max_x.max(x + w);
        max_y = max_y.max(y + h);
        out.push(CardLayout { card: c.card, x, y, w, h });
    }
    (out, max_y.max(1))
}

#[cfg(test)]
mod metro_pack_tests {
    use super::*;

    #[test]
    fn metro_cascades_into_vertical_columns() {
        let cards: Vec<CardLayout> = (0..6)
            .map(|_| CardLayout { card: DashCard::Cpu, x: 0, y: 0, w: 2, h: 2 })
            .collect();
        let (placed, rows) = pack_metro(&cards, 4);
        let want: Vec<(u16, u16)> = vec![(0, 0), (0, 2), (2, 0), (2, 2), (4, 0), (4, 2)];
        let got: Vec<(u16, u16)> = placed.iter().map(|l| (l.x, l.y)).collect();
        assert_eq!(got, want);
        assert_eq!(rows, 4);
        let mut occ: std::collections::HashSet<(u16, u16)> = std::collections::HashSet::new();
        for l in &placed {
            for dy in 0..l.h {
                for dx in 0..l.w {
                    assert!(occ.insert((l.x + dx, l.y + dy)), "overlap at {}x{}", l.x + dx, l.y + dy);
                }
            }
        }
    }
}

// ── auto-arrange compaction ─────────────────────────────────────────────────

/// Auto-arrange compaction: shifts each card as far LEFT as fits within
/// `cols`, preserving row band, size and relative order.
pub fn compact_layout(cards: &[CardLayout], cols: u16) -> (Vec<CardLayout>, bool) {
    let cols = cols.max(1);
    let mut order: Vec<usize> = (0..cards.len()).collect();
    order.sort_by_key(|&i| (cards[i].y, cards[i].x));
    let mut occ: std::collections::HashSet<(u16, u16)> = std::collections::HashSet::new();
    let mut out: Vec<CardLayout> = Vec::with_capacity(cards.len());
    let mut moved = false;
    for i in order {
        let c = cards[i];
        let w = c.w.clamp(1, cols);
        let h = c.h.max(1);
        let mut best = c.x.min(cols.saturating_sub(w));
        'scan: for cand in 0..=best {
            if cand + w > cols {
                break;
            }
            for dy in 0..h {
                for dx in 0..w {
                    if occ.contains(&(cand + dx, c.y + dy)) {
                        continue 'scan;
                    }
                }
            }
            best = cand;
            break;
        }
        for dy in 0..h {
            for dx in 0..w {
                occ.insert((best + dx, c.y + dy));
            }
        }
        if best != c.x {
            moved = true;
        }
        out.push(CardLayout { card: c.card, x: best, y: c.y, w, h });
    }
    (out, moved)
}

/// One gravity step: the LEFTMOST card that can climb ONE row lifts UP into
/// the row above's LEFTMOST free slot its full width fits.
fn up_step(cards: &[CardLayout], cols: u16) -> (Vec<CardLayout>, bool) {
    let cols = cols.max(1);
    let mut occ: std::collections::HashSet<(u16, u16)> = std::collections::HashSet::new();
    for c in cards {
        let w = c.w.clamp(1, cols);
        for dy in 0..c.h.max(1) {
            for dx in 0..w {
                occ.insert((c.x + dx, c.y + dy));
            }
        }
    }
    let mut pick: Option<usize> = None;
    let mut pick_x = 0u16;
    for (i, c) in cards.iter().enumerate() {
        if c.y == 0 {
            continue;
        }
        let w = c.w.clamp(1, cols);
        let h = c.h.max(1);
        let mut fits_any = false;
        let mut bx = c.x;
        for cand in 0..=cols.saturating_sub(w) {
            let mut fits = true;
            for dy in 0..h {
                for dx in 0..w {
                    if occ.contains(&(cand + dx, c.y - 1 + dy)) {
                        fits = false;
                        break;
                    }
                }
                if !fits {
                    break;
                }
            }
            if fits {
                fits_any = true;
                bx = cand;
                break;
            }
        }
        if !fits_any {
            continue;
        }
        match pick {
            None => {
                pick = Some(i);
                pick_x = bx;
            }
            Some(p) => {
                let pc = &cards[p];
                if c.x < pc.x || (c.x == pc.x && c.y > pc.y) {
                    pick = Some(i);
                    pick_x = bx;
                }
            }
        }
    }
    let Some(pi) = pick else {
        return (cards.to_vec(), false);
    };
    let c = cards[pi];
    let mut out = cards.to_vec();
    out[pi] = CardLayout { card: c.card, x: pick_x, y: c.y - 1, w: c.w.clamp(1, cols), h: c.h.max(1) };
    (out, true)
}

/// Full auto-arrange pass (HARD RULES): cards gravitate toward the top-left.
pub fn gravity_pack(cards: &[CardLayout], cols: u16) -> (Vec<CardLayout>, bool) {
    let mut cur = cards.to_vec();
    let mut moved = false;
    let maxit = (cur.len() + 1) * (DASH_MAX_ROWS as usize + 1) + 1;
    for _ in 0..maxit {
        let (nu, mu) = up_step(&cur, cols);
        let (nl, ml) = compact_layout(&nu, cols);
        if mu || ml {
            moved = true;
        }
        cur = nl;
        if !mu && !ml {
            break;
        }
    }
    (cur, moved)
}

// ── pixel helpers ───────────────────────────────────────────────────────────

/// Cell pitch helpers (pixels) -- shared by the layout and input handlers.
pub fn dash_card_px(l: CardLayout, cw: f32, ch: f32, grid_top: f32, dash_pad: f32, dash_gap: f32) -> (f32, f32, f32, f32) {
    (
        dash_pad + l.x as f32 * (cw + dash_gap),
        grid_top + l.y as f32 * (ch + dash_gap),
        l.w as f32 * cw + (l.w.max(1) - 1) as f32 * dash_gap,
        l.h as f32 * ch + (l.h.max(1) - 1) as f32 * dash_gap,
    )
}

/// Predefined card sizes the resize grip cycles through (w x h in cells).
pub const CARD_PRESETS: &[(u16, u16)] = &[
    (2, 2),
    (4, 2),
    (6, 2),
    (10, 2),
    (2, 4),
    (4, 4),
    (6, 4),
    (10, 4),
];
