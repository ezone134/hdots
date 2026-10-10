use super::super::*;

impl Shell {

    /// SNIPPETS — saved text clips: one row per snippet (name · body
    /// preview); clicking a row copies the body to the clipboard (the same
    /// `pending_clip`-style path the clipboard history uses), the row
    /// flashes accent for a second. Composer (`name body`, Enter) adds,
    /// hover ✕ deletes.
    pub(crate) fn draw_snippets_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        self.snippets_rect = (x, y, w, h);
        let pad = self.scale.s(12.0);
        let hdr = self.scale.s(24.0);
        let n = self.snippets.len();
        let meta = if n > 0 { format!("{n} clips") } else { String::new() };
        if self.card_show_title {
            ui::title(v, x + pad, y + self.scale.s(10.0), "Snippets", self.scale.fs(11.5), pal.fg);
        }
        ui::text_r(v, x + w - pad, y + self.scale.s(12.0), &meta, self.scale.fs(9.0), pal.fg, false);

        // composer row
        let row_h = self.scale.s(24.0);
        let cy0 = y + hdr + self.scale.s(2.0);
        let composing = self.snippet_input.is_some();
        let buf = self.snippet_input.clone().unwrap_or_default();
        v.push(Cmd::Rect { x: x + pad, y: cy0, w: w - pad * 2.0, h: row_h - 2.0, r: self.scale.s(6.0), color: if composing { ui::hover_hl(&pal) } else { ui::hover(&pal) } });
        let shown = if buf.is_empty() { "add snippet — name text to copy" } else { &buf };
        ui::text(v, x + pad + self.scale.s(8.0), cy0 + self.scale.s(5.0), shown, self.scale.fs(8.5), if buf.is_empty() { ui::fg3(&pal) } else { pal.fg }, false);
        self.region(x + pad, cy0, w - pad * 2.0, row_h, SNIPPET_INPUT_KEY);

        // snippet rows (snapshotted so hit-region registration can borrow mutably)
        let y0 = cy0 + row_h;
        let max_rows = (((y + h - 2.0) - y0) / row_h).floor() as usize;
        let flash = self
            .snippet_copied
            .filter(|_| self.snippet_copied_at.map(|t| t.elapsed().as_secs() < 1).unwrap_or(false));
        let rows: Vec<(usize, String, String)> = self
            .snippets
            .iter()
            .enumerate()
            .map(|(i, (n, b))| (i, n.clone(), b.clone()))
            .collect();
        for (i, name, body) in rows.iter() {
            let i = *i;
            let name = name.as_str();
            let body = body.as_str();
            if i >= max_rows {
                break;
            }
            let ry = y0 + i as f32 * row_h;
            let hov = self.hover_key == SNIPPET_COPY_BASE + i as u32;
            let flashed = flash == Some(i);
            if hov || flashed {
                v.push(Cmd::Rect {
                    x: x + pad,
                    y: ry,
                    w: w - pad * 2.0,
                    h: row_h - 2.0,
                    r: self.scale.s(6.0),
                    color: if flashed { (pal.acc & 0xFFFF_FF00) | 0x30 } else { ui::hover_hl(&pal) },
                });
            }
            // copy affordance + name
            ui::text(v, x + pad + self.scale.s(2.0), ry + self.scale.s(4.0), ICON_SELECT, self.scale.fs(9.0), if flashed { pal.acc } else { ui::fg2(&pal) }, true);
            let name: String = name.chars().take(12).collect();
            ui::text(v, x + pad + self.scale.s(18.0), ry + self.scale.s(4.0), &name, self.scale.fs(9.5), if flashed { pal.acc } else { pal.fg }, false);
            // body preview (dim, truncated)
            let used = self.scale.s(18.0) + name.chars().count() as f32 * self.scale.s(5.9) + self.scale.s(12.0);
            let avail = ((w - pad * 2.0 - used - self.scale.s(26.0)) / self.scale.s(5.4)).max(0.0) as usize;
            let prev: String = body.chars().take(avail).collect();
            ui::caption(v, x + pad + used, ry + self.scale.s(5.0), &prev, self.scale.fs(8.0), ui::fg3(&pal), false);
            // hit regions: copy row + hover ✕
            self.region(x + pad, ry, w - pad * 2.0, row_h, SNIPPET_COPY_BASE + i as u32);
            if hov {
                ui::text(v, x + w - pad - self.scale.s(14.0), ry + self.scale.s(4.0), ICON_CLOSE, self.scale.fs(9.0), RED, true);
                self.region(x + w - pad - self.scale.s(20.0), ry, self.scale.s(20.0), row_h, SNIPPET_DEL_BASE + i as u32);
            }
        }
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    fn shell() -> Shell {
        let cfg: crate::config::Config =
            toml::from_str(crate::config::DEFAULT_SHELL_TOML).expect("default config parses");
        Shell::new(cfg)
    }

    fn pal() -> Pal {
        Pal { fg: 0xe8e6e3ff, bg: 0x141414ff, acc: 0x4fc2ffff, sfg: 0x101010ff }
    }

    #[test]
    fn snippet_add_parses_name_body() {
        let mut s = shell();
        s.snippet_input = Some("addr 42 Imagination Street".into());
        s.snippet_add();
        assert_eq!(s.snippets.len(), 1);
        assert_eq!(s.snippets[0].0, "addr");
        assert_eq!(s.snippets[0].1, "42 Imagination Street");
        // name-only rejected
        s.snippet_input = Some("justname".into());
        s.snippet_add();
        assert_eq!(s.snippets.len(), 1);
    }

    #[test]
    fn snippets_card_registers_copy_regions() {
        let mut s = shell();
        s.mode = crate::shell::Mode::Expanded;
        s.snippets = vec![("a".into(), "b".into()), ("c".into(), "d".into())];
        let p = pal();
        let mut v: Vec<Cmd> = Vec::new();
        s.draw_snippets_card(&mut v, 0.0, 0.0, 300.0, 120.0, &p);
        let keys: Vec<u32> = s.hover_regions.iter().map(|r| r.4).collect();
        assert!(keys.contains(&SNIPPET_COPY_BASE), "row 0 copy registered");
        assert!(keys.contains(&(SNIPPET_COPY_BASE + 1)), "row 1 copy registered");
    }
}
