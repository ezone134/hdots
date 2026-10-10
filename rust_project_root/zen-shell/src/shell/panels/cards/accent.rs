use super::super::*;

impl Shell {
    /// The accent card's content anchor — `((h − s(156)) / 2).clamp(s(30),
    /// s(48))` in DEVICE px.
    ///
    /// ONE formula for the card's height-dependent top: the drawer that
    /// draws the block, `draw_card_scene`'s stamp, and the `scene_values`
    /// pool (which unscales this back to base for the scene's published
    /// y's) all come here. A second copy is how the drawer and the scene
    /// end up measuring two different cards.
    pub(crate) fn accent_top(&self, h: f32) -> f32 {
        ((h - self.scale.s(156.0)) * 0.5).clamp(self.scale.s(30.0), self.scale.s(48.0))
    }

    /// The paged custom-accents list's box inside a card box — the drawer's
    /// `x0 / ly / ww / lh`, in DEVICE px.
    ///
    /// Three publishers of this geometry call THIS: the drawer that draws
    /// the list, `draw_card_scene`'s `accent_list_rect` stamp (the wheel and
    /// the row clicks read that), and `scene_values`, which clamps the
    /// published scroll against the same viewport. A second copy would let
    /// the window that scrolls drift from the window that draws — the split
    /// that silently clicks the wrong accent.
    pub(crate) fn accent_list_box_in(&self, x: f32, y: f32, w: f32, h: f32) -> (f32, f32, f32, f32) {
        let ly = y + self.accent_top(h) + self.scale.s(122.0);
        let lh = (y + h - self.scale.s(4.0) - ly).max(8.0);
        (x + self.scale.s(10.0), ly, w - self.scale.s(20.0), lh)
    }

    /// How many 26 px rows fit a `lh`-tall list viewport — never fewer
    /// than 1, the drawer's `((lh − s(4)) / row_h).floor().max(1)`.
    pub(crate) fn accent_list_visible_in(&self, lh: f32) -> usize {
        ((lh - self.scale.s(4.0)) / self.scale.s(26.0))
            .floor()
            .max(1.0) as usize
    }

    /// The window's first row: the scroll offset clamped so the window
    /// never runs past the accents that exist.
    pub(crate) fn accent_list_start_in(&self, lh: f32) -> usize {
        let visible = self.accent_list_visible_in(lh);
        self.accent_list_scroll
            .min(self.custom_accs.len().saturating_sub(visible))
    }

    /// ACCENT — pick the accent source (from wallpaper / scheme default /
    /// custom accent) and page the one-column custom-accents subview.
    /// Mirrors the Settings Appearance block — defined-hex is merged into
    /// the custom-acc logic, so only three sources are offered.
    ///
    /// `accent.ron` owns both halves now; this is the no-scene fallback and
    /// the geometry of record its parity test draws against.
    pub(crate) fn draw_accent_card(
        &mut self,
        v: &mut Vec<Cmd>,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        pal: &Pal,
    ) {
        // the card box every published anchor is measured from — stamped
        // here too, because the scene's own stamp only runs when the card
        // HAS a scene (the no-scene fallback would leave it stale)
        self.accent_rect = (x, y, w, h);
        // header pinned to the card's TOP-LEFT (uniform card chrome) — was
        // centered mid-card with the content block
        if !self.scene_owns_header {
            self.card_head(
                v,
                pal,
                x,
                y,
                w,
                self.scale.s(12.0),
                "Theme & Accent",
                Some(ICON_STICKY),
                None,
            );
        }
        // both halves are declarative; this drawer runs under them only
        if self.scene_owns_rows && !self.accent_list_open {
            self.accent_list_rect = (0.0, 0.0, 0.0, 0.0);
            return;
        }
        // vertically center the content block in the remaining space below
        // the header; capped so the rows never clip the lower edge
        let top = self.accent_top(h);
        let y0 = y + top;
        // scheme row: the first of the four list items, pitched identically to the
        // accent-source rows below it (uniform spacing, no big gap). The scene
        // owns this row (and the three sources below it) once accent.ron
        // declares them; only the un-declared path draws it here.
        let sname = if self.cur_scheme.is_empty() {
            "default".to_string()
        } else {
            self.cur_scheme.clone()
        };
        let ctext = format!("Scheme: {sname}");
        let chov = self.hover_key == 28;
        let scheme_y = y0 + self.scale.s(28.0);
        if !self.scene_owns_rows {
            ui::text(
                v,
                x + self.scale.s(14.0),
                scheme_y + self.scale.s(6.5),
                ctext,
                self.scale.fs(10.0),
                if chov { pal.acc } else { pal.fg },
                false,
            );
            ui::text_r(
                v,
                x + w - self.scale.s(18.0),
                scheme_y + self.scale.s(5.5),
                ICON_FORWARD,
                self.scale.fs(11.0),
                ui::fg2(pal),
                true,
            );
            self.region(
                x + self.scale.s(10.0),
                scheme_y,
                w - self.scale.s(20.0),
                self.scale.s(30.0),
                28,
            );
        }
        // accent source picker (below the scheme chip). Tapping the `>`
        // chevron on "Custom accent" opens the one-column custom-accents
        // subview with a `<` back button.
        let cur = self.current_acc_source();
        let cur_name = self.current_acc_name();
        let rh = self.scale.s(30.0);
        // scheme row sits at y0+28 with the same 30 px pitch → rows start y0+58
        let rows_top = y0 + self.scale.s(58.0);
        if self.accent_list_open {
            // NOTE: no `load_custom_accs()` here. The chevron that opens this
            // subview already reloads it (input.rs), and a draw that mutated
            // the model would draw rows the publisher snapshotted one call
            // earlier — a one-frame skew no parity test could pin down.
            // back row: `<` + title
            let back_key = crate::shell::ACC_LIST_BACK_KEY;
            let back_hov = self.hover_key == back_key;
            ui::text(
                v,
                x + self.scale.s(14.0),
                rows_top,
                ICON_BACK,
                self.scale.fs(10.0),
                if back_hov { pal.acc } else { pal.fg },
                true,
            );
            ui::text(
                v,
                x + self.scale.s(28.0),
                rows_top,
                "Custom accents",
                self.scale.fs(10.0),
                pal.fg,
                false,
            );
            ui::text_r(
                v,
                x + w - self.scale.s(18.0),
                rows_top,
                format!("{}", self.custom_accs.len()),
                self.scale.fs(9.0),
                ui::fg3(pal),
                false,
            );
            self.region(
                x + self.scale.s(10.0),
                rows_top - self.scale.s(2.0),
                w - self.scale.s(20.0),
                self.scale.s(30.0),
                back_key,
            );
            // "custom color" picker row — opens the gcp GUI (never an
            // `all`-flag operation; always targets the current state)
            let ck = crate::shell::ACC_LIST_COLOR_KEY;
            let chov = self.hover_key == ck;
            let cy = rows_top + self.scale.s(32.0);
            ui::text(
                v,
                x + self.scale.s(14.0),
                cy + self.scale.s(6.5),
                ICON_DROP,
                self.scale.fs(9.5),
                if chov { pal.acc } else { pal.fg },
                true,
            );
            ui::text(
                v,
                x + self.scale.s(30.0),
                cy + self.scale.s(6.5),
                "Pick from color wheel",
                self.scale.fs(9.5),
                if chov { pal.acc } else { pal.fg },
                false,
            );
            self.region(
                x + self.scale.s(10.0),
                cy - self.scale.s(2.0),
                w - self.scale.s(20.0),
                self.scale.s(30.0),
                ck,
            );
            // one-column paged list — `ly` is `rows_top + 64`, i.e. the
            // block anchor + 122; measured by the shared helper so the
            // publisher's scroll clamp and the wheel's stamp agree with
            // the box this loop draws in
            let (x0, ly, ww, lh) = self.accent_list_box_in(x, y, w, h);
            let row_h = self.scale.s(26.0);
            let visible = self.accent_list_visible_in(lh);
            let top = self.accent_list_start_in(lh);
            for j in 0..visible {
                let i = top + j;
                let Some(a) = self.custom_accs.get(i) else {
                    break;
                };
                let rj = ly + j as f32 * row_h;
                let key = crate::shell::ACC_LIST_KEY_BASE + j as u32;
                let hov = self.hover_key == key;
                let sel = cur == "c" && !cur_name.is_empty() && a.name == cur_name;
                if sel {
                    // active item: short accent bar on the left edge (no row bg)
                    v.push(Cmd::Rect {
                        x: x0,
                        y: rj + row_h / 2.0 - self.scale.s(6.0),
                        w: self.scale.s(3.0),
                        h: self.scale.s(12.0),
                        r: self.scale.s(1.5),
                        color: pal.acc,
                    });
                }
                let shown: String = a.name.chars().take(28).collect();
                ui::text(
                    v,
                    x0 + self.scale.s(14.0),
                    rj + self.scale.s(6.5),
                    shown,
                    self.scale.fs(9.5),
                    if hov { pal.acc } else { pal.fg },
                    false,
                );
                self.region(x0, rj, ww, row_h, key);
            }
            self.accent_list_rect = (x0, ly, ww, lh);
            if self.custom_accs.is_empty() {
                ui::text(
                    v,
                    x + self.scale.s(20.0),
                    rows_top + self.scale.s(54.0),
                    "no custom accents — run gen_custom_acc_files",
                    self.scale.fs(8.5),
                    ui::fg3(pal),
                    false,
                );
            }
            return;
        }
        self.accent_list_rect = (0.0, 0.0, 0.0, 0.0);
        if self.scene_owns_rows {
            return;
        }
        let rows: [(&str, &str, bool); 3] = [
            ("w", "From wallpaper", cur == "w"),
            ("0", "Scheme default", cur == "0"),
            ("c", "Custom accent", cur == "c"),
        ];
        let mut ry = rows_top;
        for (i, &(key_src, name, active)) in rows.iter().enumerate() {
            if ry + rh > y + h - self.scale.s(3.0) {
                break;
            }
            let key = crate::shell::ACC_KEY_BASE + i as u32;
            let hov = self.hover_key == key;
            let row_x = x + self.scale.s(10.0);
            if active {
                // active item: short accent bar on the left edge (no row bg)
                v.push(Cmd::Rect {
                    x: row_x,
                    y: ry + (rh - self.scale.s(4.0)) / 2.0 - self.scale.s(6.0),
                    w: self.scale.s(3.0),
                    h: self.scale.s(12.0),
                    r: self.scale.s(1.5),
                    color: pal.acc,
                });
            }
            let label = if key_src == "c" && active && !cur_name.is_empty() {
                format!("{name} · {cur_name}")
            } else {
                name.to_string()
            };
            ui::text(
                v,
                x + self.scale.s(18.0),
                ry + self.scale.s(6.5),
                label,
                self.scale.fs(10.0),
                if hov { pal.acc } else { pal.fg },
                false,
            );
            self.region(
                row_x,
                ry,
                w - self.scale.s(20.0),
                rh - self.scale.s(4.0),
                key,
            );
            // the `>` on "Custom accent" — the entry point to the
            // custom-accents subview. Its zone sits INSIDE the row's own
            // box at the right edge and is registered after the row, so it
            // wins the overlap (the hit test walks regions backwards); the
            // glyph mirrors the scheme row's chevron
            if key_src == "c" {
                ui::text_r(
                    v,
                    x + w - self.scale.s(18.0),
                    ry + self.scale.s(5.5),
                    ICON_FORWARD,
                    self.scale.fs(11.0),
                    ui::fg2(pal),
                    true,
                );
                self.region(
                    x + w - self.scale.s(60.0),
                    ry,
                    self.scale.s(50.0),
                    rh - self.scale.s(4.0),
                    crate::shell::ACC_LIST_CHEV_KEY,
                );
            }
            ry += rh;
        }
    }
}
