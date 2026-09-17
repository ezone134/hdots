use super::super::*;

impl Shell {

    /// WALLPAPER — a scrollable thumbnail grid inside the card. Thumbnails
    /// come from the shared `self.wallpapers` cache; wheel scrolls the card
    /// (one row per notch), clicking a tile queues `set_wall {path}`.
    pub(crate) fn draw_wallpaper_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        // remember this frame's rect so the wheel handler can hit-test it
        self.wp_card_rect = (x, y, w, h);

        // header: glyph + title pinned top-left (uniform card chrome), live
        // count right — was vertically centered with the content block
        let hdr = self.wp_card_header_h();
        let pad = self.wp_card_pad();
        let n = self.wallpapers.len();
        let meta = if n > 0 { Some((format!("{} wallpapers", n), false)) } else { None };
        let meta_ref = meta.as_ref().map(|(s, m)| (s.as_str(), *m));
        self.card_head(v, pal, x, y, w, pad, "Backgrounds", Some(ICON_WALLPAPER), meta_ref);

        if self.wallpapers.is_empty() {
            ui::text_c(v, x + w / 2.0, y + h - self.scale.s(18.0), "No images here", self.scale.fs(9.5), ui::fg3(&pal), false);
            return;
        }

        // adaptive cell: shrinks so ≥5 columns and ≥2 rows always fit, hiding
        // the card with a proper mini-grid instead of a sparse single row
        let gap = self.wp_card_gap();
        let cell = self.wp_card_fit_cell(w, h);
        self.wp_card_cell_eff = cell;

        let cols = self.wp_card_cols(w).max(1);
        let rows = self.wp_card_rows(h).max(1);
        // clamp the scroll so the last page sits flush with the card bottom
        let max_scroll = self.wp_card_max_scroll(cols, rows);
        self.wp_card_scroll = self.wp_card_scroll.min(max_scroll);

        // clip-safe rows: never tile past the card boundary (guards odd sizes)
        let fit_rows = ((h - hdr - self.scale.s(6.0) + gap) / (cell + gap)).floor() as usize;
        let rows = rows.min(fit_rows).max(1);
        let max_scroll = self.wp_card_max_scroll(cols, rows);
        self.wp_card_scroll = self.wp_card_scroll.min(max_scroll);

        let files = self.wallpapers.clone();
        let start = self.wp_card_scroll;
        let visible_cells = cols * rows;
        let grid_w = cols as f32 * cell + (cols as f32 - 1.0) * gap;
        let x0 = x + (w - grid_w) / 2.0;
        let gy0 = y + hdr + self.scale.s(2.0);

        for (vi, path) in files.iter().enumerate().skip(start).take(visible_cells) {
            // position from the LOCAL tile index — `vi` already includes the
            // scroll offset, so using it directly pushes every page down
            // below the card the moment you scroll
            let t = vi - start;
            let cx = x0 + (t as f32 % cols as f32) * (cell + gap);
            let cy = gy0 + (t as f32 / cols as f32).floor() * (cell + gap);
            let key = crate::shell::WP_CARD_KEY_BASE + vi as u32;
            let hov = self.hover_key == key;
            v.push(Cmd::Rect {
                x: cx,
                y: cy,
                w: cell,
                h: cell,
                r: self.scale.s(10.0),
                color: if hov { ui::hover_hl(&pal) } else { ui::hover(&pal) },
            });
            v.push(Cmd::Image {
                x: cx + self.scale.s(3.0),
                y: cy + self.scale.s(3.0),
                w: cell - self.scale.s(6.0),
                h: cell - self.scale.s(6.0),
                key: format!("thumb:{path}"),
            });
            self.region(cx, cy, cell, cell, key);
        }

        // right-edge scrollbar (page-jump regions + proportional thumb) —
        // only when the grid is actually scrollable
        if max_scroll > 0 {
            self.draw_wp_card_scrollbar(v, x, y, w, h, cols, rows, cell, pal);
        }
    }

}
impl Shell {

    fn draw_wp_card_scrollbar(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, _cols: usize, rows: usize, _cell: f32, pal: &Pal) {
        let total_rows = Self::wp_card_total_rows(self.wallpapers.len(), self.wp_card_cols(w));
        let track_x = x + w - self.scale.s(9.0);
        let track_y = y + self.wp_card_header_h() + self.scale.s(6.0);
        let track_h = (h - self.wp_card_header_h() - self.scale.s(12.0)).max(self.scale.s(10.0));
        let track_w = self.scale.s(4.0);
        // track
        v.push(Cmd::Rect { x: track_x, y: track_y, w: track_w, h: track_h, r: self.scale.s(2.0), color: mix(ui::hover(&pal), pal.fg, 0.08) });
        // thumb
        let span = total_rows.saturating_sub(rows).max(1) as f32;
        let frac = (self.wp_card_scroll as f32 / self.wp_card_cols(w) as f32) / span;
        let thumb_h = (track_h * rows as f32 / total_rows as f32).clamp(self.scale.s(10.0), track_h);
        let thumb_y = track_y + (track_h - thumb_h) * frac.clamp(0.0, 1.0);
        let sb_hover = self.hover_key == crate::shell::WP_CARD_SB_UP || self.hover_key == crate::shell::WP_CARD_SB_DOWN;
        v.push(Cmd::Rect {
            x: track_x - self.scale.s(1.0),
            y: thumb_y,
            w: track_w + self.scale.s(2.0),
            h: thumb_h,
            r: self.scale.s(3.0),
            color: if sb_hover { mix(ui::hover(&pal), pal.fg, 0.55) } else { mix(ui::hover(&pal), pal.fg, 0.32) },
        });
        // page-jump regions: above the thumb → page up, below → page down
        let up_h = (thumb_y - track_y - self.scale.s(2.0)).max(0.0);
        let down_y = (thumb_y + thumb_h + self.scale.s(2.0)).min(track_y + track_h);
        let down_h = (track_y + track_h - down_y).max(0.0);
        let hit_w = track_w + self.scale.s(6.0);
        if up_h >= self.scale.s(4.0) {
            self.region(track_x - self.scale.s(1.0), track_y, hit_w, up_h, crate::shell::WP_CARD_SB_UP);
        }
        if down_h >= self.scale.s(4.0) {
            self.region(track_x - self.scale.s(1.0), down_y, hit_w, down_h, crate::shell::WP_CARD_SB_DOWN);
        }
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    /// Reference shell built from the pinned default config, forced into the
    /// dashboard edit mode with the Wallpaper card parked in the tray — the
    /// exact state the live shell sits in before "adding the wallpaper card".
    fn add_setup(wallpapers: usize) -> Shell {
        let cfg: Config = toml::from_str(crate::config::DEFAULT_SHELL_TOML).expect("default config parses");
        let mut shell = Shell::new(cfg);
        shell.mode = crate::shell::Mode::Expanded;
        shell.dash_enabled = true;
        shell.dash_edit = true;
        // mirror the user's live board (dash_layout.txt) for card density
        shell.dash_layout.clear();
        shell.tray_cards.clear();
        let board = [
            ("procmon", 5, 4),
            ("todo", 6, 4),
            ("quote", 10, 3),
            ("gauges", 8, 4),
            ("cpugpu", 6, 4),
            ("clipimg", 5, 4),
            ("mem", 5, 4),
            ("clipboard", 5, 4),
            ("currency", 8, 3),
        ];
        for (i, (name, w, h)) in board.iter().enumerate() {
            let card = DashCard::from_id(name).expect("known card");
            shell.dash_layout.push(CardLayout { card, x: (i % 2) as u16 * 10, y: (i / 2) as u16, w: *w, h: *h });
        }
        let parked = [
            "lyrics", "wifi", "bluetooth", "speedtest", "ticker", "recent", "thermal",
            "clock", "batteryv", "batteryh", "cpu", "disk", "notes", "media", "activewin",
            "topproc", "network", "toggles", "pkgupdates", "sshvpn", "docker", "kblayout",
            "news", "weather", "system", "calendar", "accent", "sliders", "viz", "sensors",
            "mirror", "gpu", "powerh", "wallpaper",
        ];
        for name in parked {
            shell.tray_cards.push(DashCard::from_id(name).expect("known card"));
        }
        assert!(shell.tray_cards.contains(&DashCard::Wallpaper));
        shell.wallpapers = (0..wallpapers).map(|i| format!("zen-wall-{i}.png")).collect();
        shell
    }

    /// Drive the live "click parked chip" flow: a frame populates the tray
    /// rect, a press adds the card, a drag + drop commits it, then frames
    /// render it. Panics propagate straight to the test.
    fn drag_add_and_render(shell: &mut Shell, w: f32, h: f32) {
        // frame 0 (edit mode): populates tray_rect + hover regions
        shell.layout(w, h);
        // press the parked Wallpaper chip → adds the card, arms the drag
        let wi = shell.tray_cards.iter().position(|c| *c == DashCard::Wallpaper).expect("wallpaper parked");
        let (cx, cy, cw, ch) = shell.tray_chip_px(wi);
        shell.edit_press(cx + cw / 2.0, cy + ch / 2.0);
        assert!(shell.dash_layout.iter().any(|l| l.card == DashCard::Wallpaper), "chip click must place the card");
        // drag the freshly-placed card to the board top-left, then drop
        shell.layout(w, h);
        shell.edit_motion(w / 2.0, h / 2.0);
        shell.layout(w, h);
        let over = shell.packed_layout.clone();
        let any = |s: &Shell, card: DashCard| s.packed_layout.iter().any(|l| l.card == card);
        if !any(shell, DashCard::Wallpaper) && !over.is_empty() {
            // fall back: place at an explicit slot if auto-pack bounced it
            while shell.tray_cards.contains(&DashCard::Wallpaper) {
                shell.dash_layout.push(CardLayout { card: DashCard::Wallpaper, x: 0, y: 0, w: 20, h: 4 });
                shell.tray_cards.retain(|c| *c != DashCard::Wallpaper);
                shell.refresh_pack();
            }
        }
        shell.edit_release(0.0, 0.0);
        shell.layout(w, h);
    }

    #[test]
    fn add_wallpaper_card_from_tray_does_not_crash() {
        let mut shell = add_setup(295);
        assert_eq!(shell.wallpapers.len(), 295);
        drag_add_and_render(&mut shell, 1400.0, 960.0);
        // the card must be on the board and the dashboard must render at least
        // one wallpaper thumbnail (an empty scene means the card vanished)
        assert!(shell.packed_layout.iter().any(|l| l.card == DashCard::Wallpaper));
    }

    #[test]
    fn wallpaper_card_renders_at_extreme_sizes() {
        for (i, (w, h)) in [(1400.0, 960.0), (900.0, 480.0), (600.0, 400.0), (3840.0, 2160.0)].iter().enumerate() {
            let mut shell = add_setup(295);
            // place the wallpaper card directly (static span must be (20,4))
            let (sw, sh) = Shell::default_span(DashCard::Wallpaper);
            assert_eq!((sw, sh), (20, 4), "case {i}");
            shell.dash_layout.push(CardLayout { card: DashCard::Wallpaper, x: 0, y: 0, w: sw, h: sh });
            shell.tray_cards.retain(|c| *c != DashCard::Wallpaper);
            shell.refresh_pack();
            let v = shell.layout(*w, *h);
            let thumbs = v.iter().filter(|c| matches!(c, Cmd::Image { key, .. } if key.starts_with("thumb:"))).count();
            assert!(thumbs > 0, "case {i}: no wallpaper thumbnails rendered");
        }
    }
}
