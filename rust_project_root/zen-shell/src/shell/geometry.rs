use super::*;

impl Shell {
    // ── scale-aware grid geometry ──────────────────────────────────────────
    pub(crate) fn dash_pad(&self) -> f32 { self.scale.s(self.ui_pad) }
    pub(crate) fn dash_gap(&self) -> f32 { self.scale.s(BASE_DASH_GAP) }
    /// The banner strip has collapsed (no tokens left) — the dashboard board
    /// rises to the top to take its place. The "Enable banner chips" toggle
    /// additionally collapses the whole strip band.
    pub(crate) fn banner_collapsed(&self) -> bool {
        // a strip with separators / zone markers but NO chips is still
        // "empty" — collapse the band so the cards keep only a tiny padding
        !self.banner_chips_enabled
            || !self.banner_order.iter().any(|t| matches!(t, BannerToken::Chip(_)))
    }
    /// Grid/pill Y origin: normally sits a strip-height below the banner band;
    /// when the strip is empty the grid columns move up into that space.
    pub(crate) fn dash_top(&self) -> f32 {
        if self.banner_collapsed() { self.scale.s(BASE_BANNER_Y + 2.0) } else { self.scale.s(BASE_DASH_TOP) }
    }
    pub(crate) fn tray_gap(&self) -> f32 { self.scale.s(BASE_TRAY_GAP) }
    /// Top of the banner-chips tray (edit mode): above the strip normally;
    /// at the strip's place when the strip collapsed.
    pub(crate) fn tray_at_top(&self) -> f32 {
        if self.banner_collapsed() { self.scale.s(BASE_BANNER_Y + 2.0) } else { self.scale.s(BASE_DASH_TOP + 6.0) }
    }
    pub(crate) fn card_r(&self) -> f32 {
        let px = if self.card_radius_sync { self.win_radius } else { self.card_radius };
        self.scale.s(px)
    }
    /// Window / panel corner radius in raw px (used by the surface frame
    /// which is not zoom-scaled).
    pub(crate) fn panel_r(&self) -> f32 { self.win_radius }

    pub(crate) fn dash_cell_base_w(&self, expanded_w: f32) -> f32 {
        (expanded_w - 2.0 * self.dash_pad() - (DASH_COLS - 1) as f32 * self.dash_gap()) / DASH_COLS as f32
    }
    pub(crate) fn dash_cell_h(&self, expanded_w: f32) -> f32 {
        self.dash_cell_base_w(expanded_w) // square cells
    }

    pub(crate) fn live_extent(&self) -> (u16, u16) {
        match &self.edit_preview_pack {
            Some((p, r)) => (
                p.iter().map(|l| l.x + l.w).max().unwrap_or(1),
                *r,
            ),
            None => (self.dash_cols, self.dash_rows),
        }
    }

    /// Cell-column count the canvas spans: the packed content extent, at
    /// least the edit-mode grid boost ("+" / "-" buttons). Always clamps to
    /// the 512-column ceiling so a board can grow wide however it stacks.
    pub(crate) fn canvas_cols(&self) -> u16 {
        self.live_extent().0.max(self.edit_grid_cols).max(1).min(512)
    }

    /// Cell-row count the canvas spans: content extent, at least the boost,
    /// clamped to the 1024-row ceiling.
    pub(crate) fn canvas_rows(&self) -> u16 {
        self.live_extent().1.max(self.edit_grid_rows).max(1).min(1024)
    }

    /// Hard ceiling for the edit-mode "rows +" chip and the rows readout.
    pub(crate) fn rows_cap(&self) -> u16 {
        1024
    }

    /// Hard ceiling for the edit-mode "cols +" chip and the cols readout.
    pub(crate) fn edit_cols_cap(&self) -> u16 {
        512
    }

    /// Width in columns the flow packer may use — a board grows to (at most)
    /// the canvas column count, never narrower than the 20-column card span.
    pub(crate) fn grid_max_cols(&self) -> u16 {
        self.canvas_cols().max(DASH_COLS).min(512)
    }

    /// Default size of the dashboard VIEWPORT: the surface never floats
    /// bigger than the monitor — overflow scrolls instead.
    pub(crate) fn dash_view_w(&self) -> f32 {
        (self.monitor.0 - self.scale.s(24.0)).max(320.0)
    }

    pub(crate) fn dash_view_h(&self) -> f32 {
        (self.monitor.1 - self.scale.s(24.0)).max(240.0)
    }

    /// Clamp the scroll offsets to what the current viewport allows.
    pub(crate) fn clamp_scroll(&mut self) {
        let over_x = (self.dash_canvas_w() - self.dash_view_w()).max(0.0);
        let over_y = (self.dash_canvas_h() - self.dash_view_h()).max(0.0);
        self.dash_scroll_x = self.dash_scroll_x.clamp(0.0, over_x);
        self.dash_scroll_y = self.dash_scroll_y.clamp(0.0, over_y);
    }

    /// Subtract the scroll offset from a content-space rect → surface space.
    pub(crate) fn scrolled(&self, r: (f32, f32, f32, f32)) -> (f32, f32, f32, f32) {
        (r.0 - self.dash_scroll_x, r.1 - self.dash_scroll_y, r.2, r.3)
    }

    pub(crate) fn dash_canvas_w(&self) -> f32 {
        if !self.dash_enabled || self.dash_area_collapsed() {
            // banner-only dashboard (dashboard OFF or enabled but with zero
            // cards anywhere): the strip's natural width, or the manual width
            // chosen with the edit-mode width +/− buttons (never less).
            // While editing, also never squeeze the strip-editor row below it.
            let mut w = self.banner_strip_w().max(self.banner_w as f32);
            if self.dash_edit {
                w = w.max(self.banner_ctrl_min_w());
            }
            return w;
        }
        let cw = self.dash_cell_base_w(self.cfg.expanded_w());
        let cols = self.canvas_cols() as f32;
        (self.dash_pad() * 2.0 + cols * (cw + self.dash_gap()) - self.dash_gap()).max(320.0)
    }

    /// Content height of the dashboard CARDS parking tray (edit mode) —
    /// mirrors the drawer's chip wrapping so `grid_top` and the drawing can
    /// never disagree.
    pub(crate) fn dash_tray_h(&self) -> f32 {
        if self.tray_cards.is_empty() {
            return self.scale.s(44.0);
        }
        let cw_chip = self.scale.s(96.0);
        let ch_chip = self.scale.s(44.0);
        let tray_w = self.dash_canvas_w().min(self.dash_view_w()) - self.scale.s(28.0);
        let per_row = ((tray_w - self.scale.s(24.0)) / (cw_chip + self.scale.s(12.0))).floor().max(1.0) as usize;
        let chip_rows = ((self.tray_cards.len() + per_row - 1) / per_row).max(1);
        let visible_rows = chip_rows.min(DASH_TRAY_VROWS);
        self.scale.s(26.0) + visible_rows as f32 * (ch_chip + self.scale.s(10.0)) + self.scale.s(8.0)
    }

    /// Grid Y origin: normal mode starts at DASH_TOP; edit mode pushes the
    /// grid below the top-mounted trays (banner-chips tray, then the
    /// strip-editor row, then the dashboard-cards tray when the dashboard
    /// is enabled).
    /// Dashboard has zero cards anywhere (board + parked tray): the edit
    /// chrome collapses the cards tray out of the stack so only the banner
    /// chips area shows.
    pub(crate) fn dash_area_collapsed(&self) -> bool {
        self.dash_enabled && self.dash_layout.is_empty() && self.tray_cards.is_empty()
    }

    /// No chips left to park on the banner-chips tray (only separator presets
    /// and the smart filler remain) — the whole tray collapses so the cards
    /// board (or the strip editor row) takes its space. When the "Enable
    /// banner chips" toggle is off the tray never shows either.
    pub(crate) fn banner_chips_absent(&self) -> bool {
        !self.banner_chips_enabled
            || !self
                .parked_banner_tokens()
                .iter()
                .any(|t| matches!(t, BannerToken::Chip(_)))
    }

    pub(crate) fn grid_top(&self) -> f32 {
        if self.dash_edit {
            if self.dash_enabled {
                // below the strip-editor row: the cards parking tray (the UI
                // controls row now sits ABOVE the strip-editor row instead of
                // below the cards tray, and is skipped entirely when the
                // dashboard has no cards at all)
                let cards = if self.dash_area_collapsed() {
                    0.0
                } else {
                    self.dash_tray_h() + self.tray_gap()
                };
                self.banner_ctrl_top() + self.banner_ctrl_h() + self.tray_gap() + cards
            } else {
                // dashboard off: no card grid / tray — only the banner
                // parking tray (the strip-editor row sits just below it)
                self.banner_ctrl_top()
            }
        } else {
            self.dash_top()
        }
    }

    pub(crate) fn dash_canvas_h(&self) -> f32 {
        if !self.dash_enabled || self.dash_area_collapsed() {
            // banner-only: strip height (+ the banner parking tray and the
            // strip-editor row below it while editing)
            let strip = self.banner_strip_top() + self.banner_strip_h() + self.scale.s(6.0);
            if self.dash_edit {
                return self.banner_ctrl_top() + self.banner_ctrl_h() + self.scale.s(8.0);
            }
            return strip.max(self.scale.s(48.0));
        }
        let ch = self.dash_cell_h(self.cfg.expanded_w());
        let rows = self.canvas_rows() as f32;
        let top = self.grid_top();
        (top + rows * (ch + self.dash_gap()) - self.dash_gap() + self.dash_pad()).max(160.0)
    }
}
