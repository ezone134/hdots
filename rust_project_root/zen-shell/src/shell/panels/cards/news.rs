use super::super::*;

impl Shell {

    /// NEWS — clickable headline list with a horizontal feed-category strip.
    /// The strip scrolls horizontally (trackpad two-finger / horizontal wheel)
    /// and each chip filters the vertical list; clicking a headline opens the
    /// article (`xdg-open`). Fed by the app's background fetcher
    /// (`self.news_items`, one category per `[[news_feeds]]`).
    pub(crate) fn draw_news_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        self.news_rect = (x, y, w, h);
        let pad = self.scale.s(12.0);
        let hdr = self.scale.s(Self::NEWS_HDR);
if self.card_show_glyph { ui::text(v, x + pad, y + self.scale.s(8.0), ICON_NEWS, self.scale.fs(12.0), pal.acc, true); }
if !self.scene_owns_header && self.card_show_title { ui::title(v, if self.card_show_glyph { x + pad + self.scale.s(18.0) } else { x + pad }, y + self.scale.s(9.0), "News", self.scale.fs(12.0), pal.fg); }
        if !self.news_items.is_empty() && self.news_items.len() > 1 {
            let cats = self.news_cats();
            let label = if self.news_active_cat == 0 {
                format!("{} stories", self.news_filtered_len())
            } else {
                cats.get(self.news_active_cat).cloned().unwrap_or_default()
            };
            ui::text_r(v, x + w - pad, y + self.scale.s(9.0), label, self.scale.fs(8.5), ui::fg3(&pal), false);
        }

        if self.news_items.is_empty() {
            let msg = if self.news_fetched {
                "no feeds — set [[news_feeds]]/news_url or write ~/.cache/zen-shell/news.json"
            } else {
                "fetching headlines…"
            };
            ui::text_c(v, x + w / 2.0, y + h / 2.0 - self.scale.s(6.0), msg, self.scale.fs(9.0), ui::fg3(&pal), false);
            return;
        }

        // ── horizontal category strip (chips) ────────────────────────────
        let cats = self.news_cats();
        let cat_h = if cats.len() > 1 { self.scale.s(Self::NEWS_CAT_H) } else { 0.0 };
        if cat_h > 0.0 {
            let fs = self.scale.fs(8.5);
            let chip_pad = self.scale.s(Self::NEWS_CAT_PAD);
            let sep = self.scale.s(6.0);
            // estimate per-chip width: label glyph-width + padding
            let widths: Vec<f32> = cats
                .iter()
                .map(|c| chip_pad + c.chars().count() as f32 * fs * 0.62 + chip_pad)
                .collect();
            let total: f32 = widths.iter().sum::<f32>() + sep * (widths.len() as f32 - 1.0);
            self.news_cat_content_w = total;
            let avail = (w - pad * 2.0).max(0.0);
            let max_cs = (total - avail).max(0.0);
            self.news_cat_scroll = self.news_cat_scroll.clamp(0.0, max_cs);

            let cy = y + hdr + self.scale.s(1.0);
            let ch = cat_h - self.scale.s(2.0);
            let mut cx = x + pad - self.news_cat_scroll;
            for (i, cat) in cats.iter().enumerate() {
                let cw = widths[i];
                if cx + cw < x + pad - 1.0 || cx > x + w - pad + 1.0 {
                    cx += cw + sep;
                    continue;
                }
                let active = self.news_active_cat == i;
                let key = crate::shell::NEWS_CAT_KEY_BASE + i as u32;
                let hov = self.hover_key == key;
                let bg = if active { pal.acc } else if hov { ui::hover_hl(&pal) } else { ui::hover(&pal) };
                v.push(Cmd::Rect { x: cx, y: cy, w: cw, h: ch, r: ch / 2.0, color: bg });
                let tc = if active { 0xff141414 } else { pal.fg };
                ui::text_c(v, cx + cw / 2.0, cy + ch / 2.0 - self.scale.s(4.0), cat, fs, tc, false);
                self.region(cx, cy, cw, ch, key);
                cx += cw + sep;
            }
        }

        // ── vertical headline list (filtered by active category) ─────────
        let row_h = Self::NEWS_ROW_H;
        let x0 = x + pad;
        let ww = (w - pad * 2.0).max(20.0);
        let visible = self.news_card_visible();
        let len = self.news_filtered_len();
        let top = self.news_scroll.min(len.saturating_sub(visible));
        let y0 = y + hdr + cat_h + self.scale.s(3.0);

        for j in 0..visible {
            let i = top + j;
            if i >= len {
                break;
            }
            let rj = y0 + j as f32 * row_h;
            let key = crate::shell::NEWS_HEAD_KEY_BASE + j as u32;
            let hov = self.hover_key == key;
            if let Some(item) = self.news_item_at(i) {
                // hover accent bar + title tint
                if hov {
                    v.push(Cmd::Rect { x: x0, y: rj + self.scale.s(2.0), w: self.scale.s(2.5), h: row_h - self.scale.s(6.0), r: 1.0, color: pal.acc });
                }
                let title: String = item.title.chars().take(40).collect();
                ui::text(v, x0 + self.scale.s(5.0), rj + self.scale.s(2.0), title, self.scale.fs(9.5), if hov { pal.acc } else { pal.fg }, false);
                // right side: open-glyph when clickable + source label
                let mut rx = x + w - pad;
                if !item.url.trim().is_empty() {
                    rx -= self.scale.s(13.0);
                    ui::text(v, rx, rj + self.scale.s(2.0), ICON_OPEN, self.scale.fs(8.0), if hov { pal.acc } else { ui::fg3(&pal) }, false);
                }
                if !item.source.is_empty() {
                    let src: String = item.source.chars().take(16).collect();
                    ui::text_r(v, rx, rj + self.scale.s(14.0), &src, self.scale.fs(7.5), ui::fg3(&pal), false);
                }
                v.push(Cmd::Rect { x: x0, y: rj + row_h - 1.0, w: ww, h: 1.0, r: 0.5, color: ui::hover(&pal) });
                self.region(x0, rj, ww, row_h, key);
            }
        }
    }

}
