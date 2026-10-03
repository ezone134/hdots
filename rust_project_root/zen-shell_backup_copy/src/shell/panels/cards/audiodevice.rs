use super::super::*;
use super::shared::*;

impl Shell {

    /// AUDIO DEVICE — sink/source rows from `wpctl status` (polled while the
    /// card is packed). Click a row to make it the default; click its mute
    /// glyph to toggle mute. The default sink carries the star.
    pub(crate) fn draw_audiodevice_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        let pad = self.scale.s(12.0);
        let hdr = self.scale.s(22.0);
        let total = self.audio_sinks.len() + self.audio_sources.len();
        let meta = if total > 0 { format!("{total} dev") } else { String::new() };
        card_header(v, pal, x, y, w, pad, "Audio", Some(ICON_VOLUME_FILL), Some((&meta, false)), self.scale, self.card_show_title, self.card_show_glyph);

        let rows: Vec<(u32, String, f32, bool, bool, bool)> = {
            let sinks: Vec<(u32, String, f32, bool, bool, bool)> = self
                .audio_sinks
                .clone()
                .into_iter()
                .map(|(id, n, v, m)| (id, n, v, m, true, id == self.audio_default))
                .collect();
            let sources: Vec<(u32, String, f32, bool, bool, bool)> = self
                .audio_sources
                .clone()
                .into_iter()
                .map(|(id, n, v, m)| (id, n, v, m, false, false))
                .collect();
            sinks.into_iter().chain(sources).collect()
        };

        if rows.is_empty() {
            card_empty(v, pal, x, y, w, h, "no audio devices", self.scale);
            return;
        }

        let mut ry = y + hdr + self.scale.s(4.0);
        let row_h = self.scale.s(21.0);
        for (i, (_id, name, vol, muted, is_sink, is_default)) in rows.iter().enumerate() {
            if ry + row_h > y + h - self.scale.s(2.0) {
                break;
            }
            let key = AUDIO_DEV_BASE + (i * 2) as u32;
            let mkey = AUDIO_DEV_BASE + (i * 2 + 1) as u32;
            let hov = self.hover_key == key;
            if hov {
                v.push(Cmd::Rect { x: x + pad, y: ry, w: w - pad * 2.0, h: row_h, r: self.scale.s(5.0), color: ui::raised_hl(&pal) });
            }
            // name — star marks the default; a small OUT arrow prefix marks
            // input devices
            let prefix = if *is_default {
                format!("{} ", ICON_STAR)
            } else if !*is_sink {
                format!("{} ", ICON_ARROW_FORWARD)
            } else {
                String::new()
            };
            let full = prefix + name;
            let name_chars = (((w - pad * 2.0 - self.scale.s(66.0)) / self.scale.s(5.6)).max(4.0)) as usize;
            let shown: String = full.chars().take(name_chars).collect();
            let is_hover_fg = hov;
            ui::caption(v, x + pad + self.scale.s(2.0), ry + self.scale.s(4.0), shown, self.scale.fs(8.5), if is_hover_fg { ui::hover_fg(&pal) } else { if *is_default { pal.fg } else { ui::fg2(&pal) } }, true);
            // vol % (dim on muted)
            let pct = format!("{:>3}%", (vol * 100.0).round() as i32);
            ui::text_r_mono(v, x + w - pad - self.scale.s(16.0), ry + self.scale.s(4.0), &pct, self.scale.fs(8.5), if *muted { ui::fg3(&pal) } else { if hov && self.hover_key != mkey { ui::hover_fg(&pal) } else { ui::fg3(&pal) } });
            // mute glyph
            let gm = if *muted { ICON_MUTE } else { ICON_VOLUME_FILL };
            let gc = if *muted { ui::DANGER } else { ui::fg3(&pal) };
            ui::text_r(v, x + w - pad, ry + self.scale.s(4.0), gm, self.scale.fs(9.0), gc, true);
            self.region(x + pad, ry - self.scale.s(2.0), w - pad * 2.0 - self.scale.s(18.0), row_h + self.scale.s(2.0), key);
            self.region(x + w - pad - self.scale.s(16.0), ry - self.scale.s(2.0), self.scale.s(16.0), row_h + self.scale.s(2.0), mkey);
            ry += row_h;
        }
    }

    /// Map a linear AUDIO_DEV card key back to its (device, is-mute-glyph)
    /// pair. Indices walk sinks first, then sources; each device occupies two
    /// adjacent keys (row → set default, +1 → toggle mute).
    pub(crate) fn audio_dev_pair(&self, i: usize) -> (Option<u32>, bool) {
        let is_mute = i % 2 == 1;
        let row = i / 2;
        let devices: Vec<u32> = self
            .audio_sinks
            .iter()
            .map(|d| d.0)
            .chain(self.audio_sources.iter().map(|d| d.0))
            .collect();
        (devices.get(row).copied(), is_mute)
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
    fn card_draws_rows_and_dev_pair() {
        let mut s = shell();
        s.audio_sinks = vec![(90, "USB-C Audio".into(), 0.8, false), (92, "HDMI / DisplayPort".into(), 1.0, false)];
        s.audio_sources = vec![(98, "Built-in Mic".into(), 0.4, true)];
        s.audio_default = 90;
        let p = pal();
        let mut v: Vec<Cmd> = Vec::new();
        s.draw_audiodevice_card(&mut v, 0.0, 0.0, 200.0, 110.0, &p);
        // row 0 → default = OK; row 1 → mute = toggle; row 2 → default its source
        let (d0, m0) = s.audio_dev_pair(0);
        assert_eq!(d0, Some(90));
        assert!(!m0, "row 0 is a default-action");
        let (d0m, _) = s.audio_dev_pair(1);
        assert_eq!(d0m, Some(90), "mute glyph of row 0 targets same device");
        let (_, m1) = s.audio_dev_pair(3);
        assert!(m1, "odd index is the mute glyph");
        let (d3, _) = s.audio_dev_pair(3);
        assert_eq!(d3, Some(92), "source follows sinks in the walk");
    }

    #[test]
    fn dev_pair_is_bounded() {
        let mut s = shell();
        s.audio_sinks = vec![(1, "a".into(), 1.0, false)];
        assert_eq!(s.audio_dev_pair(100), (None, false), "past the end → no device, is_mute reflects parity");
    }
}