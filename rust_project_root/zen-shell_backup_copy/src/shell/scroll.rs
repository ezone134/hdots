use super::*;

impl Shell {
    pub(crate) const CLIP_ROW_H: f32 = 36.0;
    pub(crate) const CLIP_Y0: f32 = 52.0;

    pub(crate) fn clip_visible(&self) -> usize {
        let h = self.target_size().1 - Self::CLIP_Y0 - 14.0;
        ((h / Self::CLIP_ROW_H).floor() as usize).max(1)
    }

    /// Highest allowed first-row index so the last entry is reachable.
    pub(crate) fn clip_max_scroll(&self) -> usize {
        self.clip_text.len().saturating_sub(self.clip_visible())
    }

    /// Wheel / two-finger scroll the clipboard history — one row per notch.
    pub(crate) fn clip_scroll_by(&mut self, notches: i32) -> bool {
        let old = self.clip_scroll;
        self.clip_scroll =
            (self.clip_scroll as i32 + notches).clamp(0, self.clip_max_scroll() as i32) as usize;
        old != self.clip_scroll
    }

    // ---- dashboard list-card scrolling --------------------------------
    // Each scrollable dashboard card keeps its own offset + last-drawn rect;
    // wheel over the card scrolls ITS list instead of panning the board.
    // Row pitch must mirror the draw functions in dashcards.rs.

    pub(crate) fn list_scroll_by(scroll: &mut usize, len: usize, visible: usize, notches: i32) -> bool {
        let max = len.saturating_sub(visible);
        let old = *scroll;
        *scroll = (*scroll as i32 + notches).clamp(0, max as i32) as usize;
        old != *scroll
    }

    pub(crate) fn clip_card_visible(&self) -> usize {
        let (_, _, _, h) = self.clip_card_rect;
        (((h - self.scale.s(36.0)) / self.scale.s(22.0)).floor() as usize).max(1)
    }
    pub(crate) fn clipimg_card_visible(&self) -> usize {
        let (_, _, _, h) = self.clipimg_card_rect;
        (((h - self.scale.s(36.0)) / self.scale.s(28.0)).floor() as usize).max(1)
    }
    pub(crate) fn todo_card_visible(&self) -> usize {
        let (_, _, _, h) = self.todo_card_rect;
        (((h - self.scale.s(76.0)) / self.scale.s(24.0)).floor() as usize).clamp(0, 8)
    }
    pub(crate) fn notes_card_visible(&self) -> usize {
        let (_, _, _, h) = self.notes_card_rect;
        (((h - self.scale.s(30.0) - self.scale.s(34.0)) / self.scale.s(24.0)).floor() as usize).max(1)
    }

    pub(crate) fn clip_card_scroll_by(&mut self, notches: i32) -> bool {
        let visible = self.clip_card_visible();
        let len = self.clip_text.len();
        Self::list_scroll_by(&mut self.clip_scroll, len, visible, notches)
    }
    pub(crate) fn clipimg_card_scroll_by(&mut self, notches: i32) -> bool {
        let visible = self.clipimg_card_visible();
        let len = self.clip_images.len();
        Self::list_scroll_by(&mut self.clipimg_scroll, len, visible, notches)
    }
    pub(crate) fn todo_card_scroll_by(&mut self, notches: i32) -> bool {
        let visible = self.todo_card_visible();
        let len = self.todos.len();
        Self::list_scroll_by(&mut self.todo_scroll, len, visible, notches)
    }
    pub(crate) fn notes_card_scroll_by(&mut self, notches: i32) -> bool {
        let visible = self.notes_card_visible();
        let len = self.notes.len();
        Self::list_scroll_by(&mut self.notes_scroll, len, visible, notches)
    }

    /// Wheel scrolling for the accent-card custom-accents subview.
    pub(crate) fn accent_list_scroll_by(&mut self, notches: i32) -> bool {
        let (_, _, _, hh) = self.accent_list_rect;
        let row_h = self.scale.s(26.0);
        let visible = ((hh - self.scale.s(4.0)) / row_h).floor().max(1.0) as usize;
        let len = self.custom_accs.len();
        Self::list_scroll_by(&mut self.accent_list_scroll, len, visible, notches)
    }

    // ---- notes persistence + composer ----
    pub(crate) fn notes_path() -> std::path::PathBuf {
        crate::vars::states_dir().join("notes")
    }
    /// Lazy one-shot load of `$states/notes` (one note per line,
    /// `title\x1fbody`).
    pub(crate) fn load_notes_if_needed(&mut self) {
        if self.notes_loaded {
            return;
        }
        self.notes_loaded = true;
        self.notes.clear();
        let Ok(s) = std::fs::read_to_string(Self::notes_path()) else {
            return;
        };
        for line in s.lines() {
            if line.is_empty() {
                continue;
            }
            let (t, b) = line
                .split_once('\x1f')
                .map(|(t, b)| (t.to_string(), b.to_string()))
                .unwrap_or_else(|| (line.to_string(), String::new()));
            self.notes.push((t, b));
        }
    }
    pub(crate) fn save_notes(&self) {
        let text = self
            .notes
            .iter()
            .map(|(t, b)| format!("{t}\u{1f}{b}"))
            .collect::<Vec<_>>()
            .join("\n");
        let _ = std::fs::write(Self::notes_path(), text);
    }
    /// Close the composer and clear both buffers.
    pub(crate) fn notes_blur(&mut self) {
        self.notes_input = None;
        self.notes_title.clear();
        self.notes_body.clear();
    }
    /// Commit the composer buffers as a note (newest-first), then blur.
    pub(crate) fn notes_commit(&mut self) {
        let title = self.notes_title.trim().to_string();
        let body = self.notes_body.trim().to_string();
        self.notes_blur();
        if title.is_empty() {
            return;
        }
        self.notes.insert(0, (title, body));
        self.notes.truncate(200);
        self.save_notes();
    }

    // ---- multi-pane cards: pane paging + CPU-governor power-save ----
    /// Horizontal-scroll pane flip on a swipeable card (page 0 ↔ page 1).
    /// Flip a horizontal-swipe card pane. HARD RULE: never wraps — when the
    /// last pane is showing, keep swiping right does nothing (clamped), so a
    /// card never jumps back to its first pane mid-swipe.
    pub(crate) fn card_pane_flip(pane: &mut usize, notches: i32) -> bool {
        Self::card_pane_flip_n(pane, notches, 1)
    }

    /// N-pane variant of [`card_pane_flip`] (world-map card has 4 styles).
    pub(crate) fn card_pane_flip_n(pane: &mut usize, notches: i32, max: usize) -> bool {
        if notches == 0 || max == 0 {
            return false;
        }
        let step = if notches > 0 { 1 } else { -1 };
        let nxt = ((*pane as i32 + step).clamp(0, max as i32)) as usize;
        if nxt == *pane {
            return false;
        }
        *pane = nxt;
        true
    }
    /// Last power-save state per channel (`$states/power_save_<ch>`).
    pub(crate) fn load_power_save(&mut self) {
        let ch = crate::vars::read_channel();
        self.power_save_on = std::fs::read_to_string(crate::vars::states_dir().join(format!("power_save_{ch}")))
            .map(|s| s.trim() == "1")
            .unwrap_or(false);
    }
    /// Toggle the CPU frequency-scaling governor (performance ↔ powersave)
    /// across all online cores. Direct sysfs write first; falls back to
    /// `pkexec` when the session user lacks permission. Persists the result.
    pub(crate) fn toggle_power_save(&mut self) -> bool {
        let cur = std::fs::read_to_string("/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor").unwrap_or_default();
        let next = if cur.trim() == "powersave" { "performance" } else { "powersave" };
        let mut govs: Vec<std::path::PathBuf> = Vec::new();
        if let Ok(rd) = std::fs::read_dir("/sys/devices/system/cpu") {
            for e in rd.flatten() {
                let n = e.file_name();
                let n = n.to_string_lossy();
                if !n.starts_with("cpu") || !n[3..].chars().all(|c| c.is_ascii_digit()) {
                    continue;
                }
                let p = e.path().join("cpufreq").join("scaling_governor");
                if p.exists() {
                    govs.push(p);
                }
            }
        }
        if govs.is_empty() {
            return false;
        }
        let mut ok = true;
        for p in &govs {
            if std::fs::write(p, format!("{next}\n")).is_err() {
                ok = false;
            }
        }
        if !ok {
            let script = govs
                .iter()
                .map(|p| format!("printf '{next}\\n' > {}", p.display()))
                .collect::<Vec<_>>()
                .join("; ");
            ok = std::process::Command::new("pkexec")
                .args(["sh", "-c", &script])
                .status()
                .map(|s| s.success())
                .unwrap_or(false);
        }
        if ok {
            self.power_save_on = next == "powersave";
            let ch = crate::vars::read_channel();
            let _ = std::fs::write(
                crate::vars::states_dir().join(format!("power_save_{ch}")),
                if self.power_save_on { "1" } else { "0" },
            );
        }
        ok
    }

    // ---- keybind viewer metrics (shared with the panel layout) ----
    pub(crate) const KB_ROW_H: f32 = 34.0;
    pub(crate) const KB_Y0: f32 = 52.0;
    pub(crate) const KB_GAP: f32 = 4.0;

    /// How many keybind rows fit under the header.
    pub(crate) fn keybinds_visible(&self) -> usize {
        let h = self.target_size().1 - Self::KB_Y0 - 14.0;
        (((h + Self::KB_GAP) / (Self::KB_ROW_H + Self::KB_GAP)).floor() as usize).max(1)
    }

    /// Highest allowed first-row index so the last bind is reachable.
    pub(crate) fn keybinds_max_scroll(&self) -> usize {
        self.keybinds.len().saturating_sub(self.keybinds_visible())
    }

    /// Wheel / two-finger scroll the keybind list — one row per notch.
    pub(crate) fn keybinds_scroll_by(&mut self, notches: i32) -> bool {
        let old = self.keybinds_scroll;
        self.keybinds_scroll =
            (self.keybinds_scroll as i32 + notches).clamp(0, self.keybinds_max_scroll() as i32) as usize;
        old != self.keybinds_scroll
    }

    /// Scroll the Settings → Appearance content pane by `notches` (px pitch).
    /// Clamps to the pane's content height. Returns true when the offset moved.
    pub(crate) fn appearance_scroll_by(&mut self, notches: i32) -> bool {
        let old = self.appearance_scroll;
        let max = self.appearance_max_scroll().max(0.0);
        let next = self.appearance_scroll + notches as f32 * 40.0;
        self.appearance_scroll = next.clamp(0.0, max);
        old != self.appearance_scroll
    }

    /// Max px the Appearance pane can scroll (content bottom − visible bottom).
    pub(crate) fn appearance_max_scroll(&self) -> f32 {
        (self.appearance_content_h() - (self.target_size().1 - 150.0 - 14.0)).max(0.0)
    }

    /// Scroll the Settings → Pill content pane by `notches` (px pitch).
    /// Returns true when the offset moved.
    pub(crate) fn pill_scroll_by(&mut self, notches: i32) -> bool {
        let old = self.pill_scroll;
        let max = self.pill_max_scroll().max(0.0);
        let next = self.pill_scroll + notches as f32 * 40.0;
        self.pill_scroll = next.clamp(0.0, max);
        old != self.pill_scroll
    }

    /// Max px the Pill pane can scroll (content bottom − visible bottom).
    /// Mirrors `layout_settings_pill`: battery-% row sits at 422 + tip +
    /// 17·40 and the Position row adds ≈76px below it.
    pub(crate) fn pill_max_scroll(&self) -> f32 {
        let tip = if self.expand_on_hover { SETTINGS_EXPAND_TIP_H } else { 0.0 };
        let content_h = 422.0 + tip + 17.0 * 40.0 + 76.0;
        (content_h - (self.target_size().1 - 150.0 - 14.0)).max(0.0)
    }

    /// Total height (px) of the scrollable Appearance content, from its top
    /// (`SCROLL_TOP` ≈ 150) down. Mirrors `layout_settings_appearance`. The
    /// custom-accent dropdown (header + row + popup allowance) replaces the
    /// old static grid.
    pub(crate) fn appearance_content_h(&self) -> f32 {
        22.0                      // ACCENT TOGGLES header
        + 5.0 * 52.0              // 5 toggle rows
        + 52.0                    // start-icon tone slider
        + 22.0                    // ALPHA header
        + 4.0 * 44.0              // 4 alpha sliders
        + 52.0                    // Accent scrim toggle
        + 22.0                    // ACCENT SOURCE header
        + 5.0 * 50.0              // wall + scheme + defined hex + custom accent + Pick
        + 22.0                    // CUSTOM ACCENT header
        + 40.0                    // dropdown row
        + 240.0                   // open dropdown popup (≈6 rows)
        + 22.0                    // ICON FONT header
        + 4.0 * 50.0              // 4 icon-font option rows
        + 50.0                    // Apply changes button
    }

    /// Scroll the open custom-accent circle grid by `notches` rows.
    /// Returns true when the window moved.
    pub(crate) fn custom_acc_drop_scroll_by(&mut self, notches: i32) -> bool {
        if !self.custom_acc_drop_open {
            return false;
        }
        let n = self.custom_accs.len();
        if n == 0 {
            return false;
        }
        // mirror the grid metrics from layout_settings_appearance
        pub(crate) const CIRCLE: f32 = 40.0;
        pub(crate) const GAP_X: f32 = 12.0;
        pub(crate) const VISIBLE_ROWS: usize = 3;
        let cxx = crate::shell::SETTINGS_SIDEBAR;
        let roww = self.target_size().0 - cxx - 16.0;
        let cols = (roww / (CIRCLE + GAP_X)).floor().max(1.0) as usize;
        let rows = (n + cols - 1) / cols;
        if rows <= VISIBLE_ROWS {
            self.custom_acc_drop_scroll = self.custom_acc_drop_scroll.min(cols - 1);
            return false;
        }
        let old = self.custom_acc_drop_scroll;
        let top_row = old / cols;
        let max_row = (rows - VISIBLE_ROWS) as isize;
        let next_row = (top_row as isize + notches as isize).clamp(0, max_row);
        self.custom_acc_drop_scroll = (next_row as usize) * cols;
        old / cols != next_row as usize
    }

    // ---- wallpaper grid metrics (shared with the panel layout) ----
    pub(crate) const WP_GAP: f32 = 10.0;
    pub(crate) const WP_Y0: f32 = 52.0;
    pub(crate) const WP_CELL_MIN: f32 = 56.0;
    pub(crate) const WP_CELL_MAX: f32 = 160.0;

    // ── dashboard Wallpaper-card grid ─────────────────────────────────────
    // A scrollable thumbnail grid inside the card (independent of the full
    // picker panel). Thumbnail cell is one third the default card cell pitch.
    pub(crate) const WP_CARD_CELL: f32 = 72.0;
    pub(crate) const WP_CARD_GAP: f32 = 8.0;
    pub(crate) const WP_CARD_PAD: f32 = 12.0;
    pub(crate) const WP_CARD_HEADER_H: f32 = 30.0;

    // ── dashboard News-card list ─────────────────────────────────────────
    /// row height for one headline
    pub(crate) const NEWS_ROW_H: f32 = 30.0;
    /// News-card header strip height
    pub(crate) const NEWS_HDR: f32 = 30.0;
    /// News-card horizontal category-strip height (chips)
    pub(crate) const NEWS_CAT_H: f32 = 26.0;
    pub(crate) const NEWS_CAT_PAD: f32 = 11.0;

    // ── dashboard Lyrics-card list ──────────────────────────────────────
    /// Lyrics-card header strip height
    pub(crate) const LYRICS_HDR: f32 = 30.0;
    /// row height for one lyric line
    pub(crate) const LYRICS_ROW_H: f32 = 21.0;

    /// Columns that fit in a Wallpaper-card of pixel width `card_w`.
    pub(crate) fn wp_card_cols(&self, card_w: f32) -> usize {
        let cell = self.wp_card_cell_now();
        let gap = self.wp_card_gap();
        let pad = self.wp_card_pad();
        // reserve 14px for the right-edge scrollbar so tiles never slide under it
        let cw = (card_w - 14.0).max(10.0);
        (((cw - 2.0 * pad + gap) / (cell + gap)).floor() as usize).max(1)
    }

    /// Preferred (pre-scale) tile cell; the adaptive cell only shrinks it.
    pub(crate) fn wp_card_cell(&self) -> f32 { self.scale.s(Self::WP_CARD_CELL) }
    /// The cell the card is actually drawn at this frame (`wp_card_cell_eff`),
    /// falling back to the preferred cell before the first draw.
    pub(crate) fn wp_card_cell_now(&self) -> f32 {
        if self.wp_card_cell_eff > 0.0 {
            self.wp_card_cell_eff
        } else {
            self.wp_card_cell()
        }
    }
    pub(crate) fn wp_card_gap(&self) -> f32 { self.scale.s(Self::WP_CARD_GAP) }
    pub(crate) fn wp_card_pad(&self) -> f32 { self.scale.s(Self::WP_CARD_PAD) }

    /// Tile cell size that fills the card: the preferred cell, shrunk so at
    /// least five columns and two rows fit horizontally / vertically. Guarantees
    /// the grid never traps a handful of tiles in an empty card.
    pub(crate) fn wp_card_fit_cell(&self, card_w: f32, card_h: f32) -> f32 {
        let gap = self.wp_card_gap();
        let pad = self.wp_card_pad();
        let avail_h = (card_h - self.wp_card_header_h() - pad).max(0.0);
        let max_by_w = (card_w - 14.0 - 2.0 * pad + gap) / 5.0;
        let max_by_h = (avail_h - gap) / 2.0;
        self.wp_card_cell().min(max_by_w).min(max_by_h).max(1.0)
    }

    /// Visible rows that fit in a Wallpaper-card of pixel height `card_h`
    /// (accounting for the header row).
    pub(crate) fn wp_card_rows(&self, card_h: f32) -> usize {
        let avail = card_h - self.wp_card_header_h() - self.wp_card_pad();
        (((avail + self.wp_card_gap()) / (self.wp_card_cell_now() + self.wp_card_gap())).floor() as usize).max(1)
    }

    /// One viewport of rows (used by the scrollbar page-jump regions).
    pub(crate) fn wp_card_page(&self) -> i32 {
        let (_, _, w, h) = self.wp_card_rect;
        if w <= 0.0 || h <= 0.0 {
            1
        } else {
            self.wp_card_rows(h).max(1) as i32 * self.wp_card_cols(w).max(1) as i32
        }
    }

    pub(crate) fn wp_card_header_h(&self) -> f32 { self.scale.s(Self::WP_CARD_HEADER_H) }

    /// Highest card-local scroll offset — whole rows, so the last page sits
    /// flush with the card bottom. `cols`/`rows` come from the card geometry.
    pub(crate) fn wp_card_max_scroll(&self, cols: usize, rows: usize) -> usize {
        let total = Self::wp_card_total_rows(self.wallpapers.len(), cols);
        total.saturating_sub(rows) * cols
    }

    pub(crate) fn wp_card_total_rows(n: usize, cols: usize) -> usize {
        (n + cols - 1).max(0) / cols.max(1)
    }

    /// Category names shown on the News-card chip strip, "All" first. Each
    /// distinct `NewsItem.category` (one per configured `[[news_feeds]]`)
    /// becomes its own chip; stories without a category stick to "All".
    pub(crate) fn news_cats(&self) -> Vec<String> {
        let mut cats: Vec<String> = Vec::new();
        for it in &self.news_items {
            let c = it.category.trim();
            if !c.is_empty() && !cats.iter().any(|x| x == c) {
                cats.push(c.to_string());
            }
        }
        if cats.len() > 1 {
            let mut full = Vec::with_capacity(cats.len() + 1);
            full.push("All".to_string());
            full.extend(cats);
            full
        } else {
            cats // single category → no chip strip at all
        }
    }

    /// Indices (into `news_items`) of stories shown for the active category.
    /// `news_active_cat == 0` → All, else `news_cats()[active]`.
    pub(crate) fn news_filtered_idx(&self) -> Vec<usize> {
        let cats = self.news_cats();
        if cats.len() <= 1 {
            return (0..self.news_items.len()).collect();
        }
        let want = cats.get(self.news_active_cat).map(|s| s.as_str());
        if want.is_none() || matches!(want, Some("All")) {
            return (0..self.news_items.len()).collect();
        }
        let want = want.unwrap();
        self.news_items
            .iter()
            .enumerate()
            .filter_map(|(i, it)| if it.category == want { Some(i) } else { None })
            .collect()
    }

    pub(crate) fn news_filtered_len(&self) -> usize {
        self.news_filtered_idx().len()
    }

    pub(crate) fn news_item_at(&self, i: usize) -> Option<&crate::news::NewsItem> {
        self.news_filtered_idx().get(i).and_then(|&g| self.news_items.get(g))
    }

    /// How many headline rows fit under the header + category strip.
    pub(crate) fn news_card_visible(&self) -> usize {
        let (_, _, w, h) = self.news_rect;
        if w <= 0.0 || h <= 0.0 {
            return 1;
        }
        let cat_h = if self.news_cats().len() > 1 { self.scale.s(Self::NEWS_CAT_H) } else { 0.0 };
        let avail = h - self.scale.s(Self::NEWS_HDR) - cat_h - self.scale.s(6.0);
        ((avail / self.scale.s(Self::NEWS_ROW_H)).floor() as usize).max(1)
    }

    /// Scroll the News card list by `notches` (lines) over the active
    /// category; returns true when the offset moved.
    pub(crate) fn news_scroll_by(&mut self, notches: i32) -> bool {
        let visible = self.news_card_visible();
        let len = self.news_filtered_len();
        Self::list_scroll_by(&mut self.news_scroll, len, visible, notches)
    }

    /// Scroll the News-card category chip strip horizontally by `px` (wheel
    /// / trackpad two-finger); returns true when the offset moved.
    pub(crate) fn news_cat_scroll_by(&mut self, px: f32) -> bool {
        let (_, _, w, h) = self.news_rect;
        if w <= 0.0 || h <= 0.0 {
            return false;
        }
        let avail = (w - self.scale.s(24.0)).max(0.0);
        let max = (self.news_cat_content_w - avail).max(0.0);
        let old = self.news_cat_scroll;
        self.news_cat_scroll = (self.news_cat_scroll + px).clamp(0.0, max);
        old != self.news_cat_scroll
    }

    /// Activate a category chip (`0` = All; clicking the active chip again
    /// returns to All).
    pub(crate) fn news_activate_cat(&mut self, chip: usize) {
        let n = self.news_cats().len();
        if n <= 1 {
            self.news_active_cat = 0;
            self.news_scroll = 0;
            return;
        }
        if chip >= n {
            self.news_scroll = 0;
            return;
        }
        self.news_active_cat = (chip == self.news_active_cat).then_some(0).unwrap_or(chip);
        self.news_scroll = 0;
    }

    /// Open the story at filtered index `i` in the browser (headline click).
    pub(crate) fn news_open(&mut self, i: usize) {
        if let Some(it) = self.news_item_at(i) {
            if !it.url.trim().is_empty() {
                self.pending_news_open = Some(it.url.trim().to_string());
            }
        }
    }

    // ── dashboard Lyrics card ───────────────────────────────────────────

    /// How many lyric lines the card can show (geometry from the last draw).
    pub(crate) fn lyrics_card_visible(&self) -> usize {
        let (_, _, w, h) = self.lyrics_rect;
        if w <= 0.0 || h <= 0.0 {
            return 0;
        }
        let pad = self.scale.s(Self::LYRICS_HDR) + self.scale.s(6.0);
        (((h - pad) / self.scale.s(Self::LYRICS_ROW_H)).floor().max(1.0) as usize).min(200)
    }

    /// Index of the line the playhead has reached, if the loaded lines carry
    /// LRC timestamps. A paused track keeps its current line lit (the
    /// interpolated playhead just stops advancing).
    pub(crate) fn lyrics_active_line(&self) -> Option<usize> {
        let lines = &self.lyrics_lines;
        if lines.is_empty() || !crate::lyrics::has_timing(lines) {
            return None;
        }
        let pos = self.media_pos_now() as f64 / 1e6;
        let mut active = None;
        for (i, l) in lines.iter().enumerate() {
            if l.time < 0.0 {
                continue;
            }
            if l.time > pos {
                break;
            }
            active = Some(i);
        }
        active
    }

    /// Auto-follow the singing line while playing. Uses smooth interpolation
    /// for buttery scrolling — the target is 33% from the top, and the actual
    /// scroll position eases toward it each frame.
    pub(crate) fn lyrics_follow(&mut self, visible: usize) {
        let Some(active) = self.lyrics_active_line() else { return };
        if !self.media_playing || visible == 0 {
            return;
        }
        let max = self.lyrics_lines.len().saturating_sub(visible);
        let target = (active as f32 - visible as f32 * 0.33).max(0.0).min(max as f32);
        // Smooth easing toward the target (lerp factor tuned for 60fps feel)
        let diff = target - self.lyrics_scroll_smooth;
        if diff.abs() < 0.05 {
            self.lyrics_scroll_smooth = target;
        } else {
            self.lyrics_scroll_smooth += diff * 0.18;
        }
        self.lyrics_scroll = (self.lyrics_scroll_smooth.round() as usize).min(max);
    }

    /// Wheel-scroll the lyrics card by `notches` rows; returns true when moved.
    pub(crate) fn lyrics_scroll_by(&mut self, notches: i32) -> bool {
        let (_, _, w, h) = self.lyrics_rect;
        if w <= 0.0 || h <= 0.0 {
            return false;
        }
        let visible = self.lyrics_card_visible();
        let max = self.lyrics_lines.len().saturating_sub(visible);
        let old = self.lyrics_scroll;
        self.lyrics_scroll = (self.lyrics_scroll as i32 + notches).clamp(0, max as i32) as usize;
        // Sync smooth scroll to manual position so auto-follow resumes from here
        self.lyrics_scroll_smooth = self.lyrics_scroll as f32;
        old != self.lyrics_scroll
    }

    // ----------------------------------------------------- card row helpers

    /// Rows visible in the Wi-Fi card (mirrors `draw_wifi_card`).
    pub(crate) fn wifi_card_visible(&self) -> usize {
        if self.wifi_rect.3 <= 0.0 { return 0; }
        (((self.wifi_rect.3 - self.scale.s(28.0)) / self.scale.s(19.0)).floor().max(1.0) as usize)
            .min(200)
    }
    /// Rows visible in the Bluetooth card.
    pub(crate) fn bt_card_visible(&self) -> usize {
        if self.bt_rect.3 <= 0.0 { return 0; }
        (((self.bt_rect.3 - self.scale.s(28.0)) / self.scale.s(19.0)).floor().max(1.0) as usize)
            .min(200)
    }
    /// Rows visible in the Recent-files card.
    pub(crate) fn recent_card_visible(&self) -> usize {
        if self.recent_rect.3 <= 0.0 { return 0; }
        (((self.recent_rect.3 - self.scale.s(28.0)) / self.scale.s(19.0)).floor().max(1.0) as usize)
            .min(200)
    }
    /// Rows visible in the Currency card (footer note eats a row).
    pub(crate) fn currency_card_visible(&self) -> usize {
        if self.currency_rect.3 <= 0.0 { return 0; }
        (((self.currency_rect.3 - self.scale.s(42.0)) / self.scale.s(20.0)).floor().max(1.0) as usize)
            .min(200)
    }
    /// Rows visible in the Prices (ticker) card.
    pub(crate) fn ticker_card_visible(&self) -> usize {
        if self.ticker_rect.3 <= 0.0 { return 0; }
        (((self.ticker_rect.3 - self.scale.s(28.0)) / self.scale.s(18.0)).floor().max(1.0) as usize)
            .min(200)
    }

    /// Wheel the Wi-Fi card by `notches` rows; true when moved.
    pub(crate) fn wifi_scroll_by(&mut self, notches: i32) -> bool {
        let old = self.wifi_scroll;
        let max = self.wifi_networks.len().saturating_sub(self.wifi_card_visible());
        self.wifi_scroll = (self.wifi_scroll as i32 + notches).clamp(0, max as i32) as usize;
        old != self.wifi_scroll
    }
    /// Wheel the Bluetooth card by `notches` rows; true when moved.
    pub(crate) fn bt_scroll_by(&mut self, notches: i32) -> bool {
        let old = self.bt_scroll;
        let max = self.bt_devices.len().saturating_sub(self.bt_card_visible());
        self.bt_scroll = (self.bt_scroll as i32 + notches).clamp(0, max as i32) as usize;
        old != self.bt_scroll
    }
    /// Wheel the Recent-files card by `notches` rows; true when moved.
    pub(crate) fn recent_scroll_by(&mut self, notches: i32) -> bool {
        let old = self.recent_scroll;
        let max = self.recent_files.len().saturating_sub(self.recent_card_visible());
        self.recent_scroll = (self.recent_scroll as i32 + notches).clamp(0, max as i32) as usize;
        old != self.recent_scroll
    }
    /// Wheel the Currency card by `notches` rows; true when moved.
    pub(crate) fn currency_scroll_by(&mut self, notches: i32) -> bool {
        let old = self.currency_scroll;
        let max = 12usize.saturating_sub(self.currency_card_visible());
        self.currency_scroll = (self.currency_scroll as i32 + notches).clamp(0, max as i32) as usize;
        old != self.currency_scroll
    }
    /// Wheel the Prices card by `notches` rows; true when moved.
    pub(crate) fn ticker_scroll_by(&mut self, notches: i32) -> bool {
        let old = self.ticker_scroll;
        let max = self.ticker_items.len().saturating_sub(self.ticker_card_visible());
        self.ticker_scroll = (self.ticker_scroll as i32 + notches).clamp(0, max as i32) as usize;
        old != self.ticker_scroll
    }

    /// Scroll the card grid by `notches` (one step = one whole row). Uses the
    /// last-drawn card rect geometry; returns true when the offset moved.
    pub(crate) fn wp_card_scroll_by(&mut self, notches: i32) -> bool {
        let (_, _, w, h) = self.wp_card_rect;
        if w <= 0.0 || h <= 0.0 {
            return false;
        }
        let cols = self.wp_card_cols(w);
        let rows = self.wp_card_rows(h);
        let max = self.wp_card_max_scroll(cols, rows);
        let old = self.wp_card_scroll;
        let next = self.wp_card_scroll as i32 + notches * cols as i32;
        self.wp_card_scroll = next.clamp(0, max as i32) as usize;
        old != self.wp_card_scroll
    }

    /// Grid columns for the current cell size — derived from the panel
    /// width so the grid always fills the card edge to edge.
    pub(crate) fn wp_cols(&self) -> usize {
        let w = self.target_size().0 - 24.0;
        (((w + Self::WP_GAP) / (self.wp_cell + Self::WP_GAP)).floor() as usize).max(1)
    }

    /// Grid rows that fit under the picker header.
    pub(crate) fn wp_rows(&self) -> usize {
        let h = self.target_size().1 - Self::WP_Y0 - 14.0;
        (((h + Self::WP_GAP) / (self.wp_cell + Self::WP_GAP)).floor() as usize).max(1)
    }

    /// How many wallpaper thumbnails fit on screen.
    pub(crate) fn wallpaper_visible(&self) -> usize {
        self.wp_cols() * self.wp_rows()
    }

    /// Total grid rows for the current wallpaper list.
    pub(crate) fn wp_total_rows(&self) -> usize {
        (self.wallpapers.len() + self.wp_cols() - 1) / self.wp_cols()
    }

    /// Highest allowed scroll offset — whole rows, so the final page sits
    /// flush with the bottom of the window.
    pub(crate) fn wallpaper_max_scroll(&self) -> usize {
        self.wp_total_rows().saturating_sub(self.wp_rows()) * self.wp_cols()
    }

    /// Scroll the wallpaper grid by `delta` wheel steps — one step is a whole
    /// grid row (cols thumbnails). Returns true when the offset moved.
    pub(crate) fn wallpaper_scroll_by(&mut self, delta: i32) -> bool {
        let old = self.wallpaper_scroll;
        let next = self.wallpaper_scroll as i32 + delta * self.wp_cols() as i32;
        self.wallpaper_scroll = next.clamp(0, self.wallpaper_max_scroll() as i32) as usize;
        old != self.wallpaper_scroll
    }

    /// Ctrl+= / Ctrl+- : grow / shrink the picker thumbnails. The size is
    /// remembered via `[wallpaper] cell` in shell.toml. True when changed.
    pub(crate) fn wallpaper_zoom(&mut self, grow: bool) -> bool {
        let step = if grow { 8.0 } else { -8.0 };
        let next = (self.wp_cell + step).clamp(Self::WP_CELL_MIN, Self::WP_CELL_MAX);
        if (next - self.wp_cell).abs() < f32::EPSILON {
            return false;
        }
        self.wp_cell = next;
        // keep the visible window valid for the new cell metrics
        self.wallpaper_scroll = self.wallpaper_scroll.min(self.wallpaper_max_scroll());
        true
    }

    /// Scrollbar track rect for the wallpaper panel: (x, y, w, h). It spans
    /// exactly the current grid height.
    pub(crate) fn wp_track(&self) -> (f32, f32, f32, f32) {
        let track_h = self.wp_rows() as f32 * (self.wp_cell + Self::WP_GAP) - Self::WP_GAP;
        (self.target_size().0 - 12.0, Self::WP_Y0, 6.0, track_h)
    }

    /// Scrollbar thumb rect at the current scroll: (y offset into the track, h).
    pub(crate) fn wp_thumb(&self, track_h: f32) -> (f32, f32) {
        let total_rows = self.wp_total_rows().max(1) as f32;
        let thumb_h = (track_h / total_rows).max(20.0);
        let vis_rows = self.wp_rows() as f32;
        let scroll_row = self.wallpaper_scroll as f32 / self.wp_cols() as f32;
        let t = (scroll_row / (total_rows - vis_rows).max(1.0)).clamp(0.0, 1.0);
        ((track_h - thumb_h) * t, thumb_h)
    }

    /// Press on the wallpaper scrollbar strip: begin a grab-drag. A press on
    /// the thumb keeps its in-thumb offset; a press on the empty track first
    /// page-jumps so the thumb centers under the cursor. Returns false when
    /// the press missed the strip (normal click handling proceeds).
    pub(crate) fn begin_wp_sb_drag(&mut self, x: f32, y: f32) -> bool {
        if self.mode != Mode::Wallpaper || self.wallpapers.len() <= self.wallpaper_visible() {
            return false;
        }
        let (tx, ty, tw, th) = self.wp_track();
        if x < tx - 8.0 || x > tx + tw + 8.0 || y < ty || y > ty + th {
            return false;
        }
        let (rel, thumb_h) = self.wp_thumb(th);
        self.wp_sb_grab = if y >= ty + rel && y <= ty + rel + thumb_h {
            // thumb grab: remember where inside the thumb it was caught
            (y - ty - rel).clamp(0.0, thumb_h)
        } else {
            // track jump: center the thumb under the pointer, then drag
            let usable = (th - thumb_h).max(1.0);
            let t = ((y - ty - thumb_h / 2.0) / usable).clamp(0.0, 1.0);
            let max_row = self.max_wp_row();
            let row = ((t * max_row as f32).round() as i32).clamp(0, max_row);
            self.wallpaper_scroll = row as usize * self.wp_cols();
            thumb_h / 2.0
        };
        self.wp_sb_drag = true;
        true
    }

    /// Last valid grid-row index a drag can scroll to.
    pub(crate) fn max_wp_row(&self) -> i32 {
        (self.wp_total_rows() as i32 - self.wp_rows() as i32).max(0)
    }

    /// Move a wallpaper-scrollbar drag: map pointer y through the track into
    /// a whole-row scroll offset. Returns true when the offset moved.
    pub(crate) fn drag_wp_sb(&mut self, y: f32) -> bool {
        if !self.wp_sb_drag || self.max_wp_row() == 0 {
            return false;
        }
        let (_, ty, _, th) = self.wp_track();
        let (_, thumb_h) = self.wp_thumb(th);
        let usable = (th - thumb_h).max(1.0);
        let t = ((y - ty - self.wp_sb_grab) / usable).clamp(0.0, 1.0);
        let row = ((t * self.max_wp_row() as f32).round() as i32).clamp(0, self.max_wp_row());
        let next = row as usize * self.wp_cols();
        if next != self.wallpaper_scroll {
            self.wallpaper_scroll = next;
            true
        } else {
            false
        }
    }

    /// End a wallpaper-scrollbar drag (pointer released).
    pub(crate) fn end_wp_sb_drag(&mut self) {
        self.wp_sb_drag = false;
    }

}
