use super::super::*;

impl Shell {
    /// Two-step delete for the recordings list: with `confirm_delete` on, the
    /// first click ARMS the row (a second click deletes); with it off, one
    /// click deletes straight away. The paint lives in `audiorec.lua`; this is
    /// the input-side state machine only.
    pub(crate) fn audiorec_del_click(&mut self, i: usize) {
        if self.audiorec_confirm_delete {
            if self.audiorec_del_armed == Some(i) {
                self.audiorec_del_armed = None;
                self.pending_audiorec_del = Some(i);
            } else {
                self.audiorec_del_armed = Some(i);
            }
        } else {
            self.pending_audiorec_del = Some(i);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shell() -> Shell {
        let cfg: crate::config::Config =
            toml::from_str(crate::config::DEFAULT_SHELL_TOML).expect("default config parses");
        let mut s = Shell::new(cfg);
        s.mode = crate::shell::Mode::Expanded;
        s.dash_enabled = true;
        s.audiorec_recordings = vec![
            ("rec-3".into(), "/tmp/rec-3.wav".into()),
            ("rec-2".into(), "/tmp/rec-2.wav".into()),
            ("rec-1".into(), "/tmp/rec-1.wav".into()),
        ];
        s
    }

    /// Place the card, run the full shell fan-out (which loads the live
    /// `audiorec.lua` module), and return the frame's commands.
    fn frame(shell: &mut Shell) -> Vec<Cmd> {
        shell.dash_layout.push(CardLayout {
            card: DashCard::AudioRec,
            x: 0,
            y: 0,
            w: 10,
            h: 9,
        });
        shell.refresh_pack();
        shell.layout(900.0, 340.0)
    }

    fn keys(shell: &Shell) -> Vec<u32> {
        shell.hover_regions.iter().map(|r| r.4).collect()
    }

    fn texts(v: &[Cmd]) -> Vec<String> {
        v.iter()
            .filter_map(|c| match c {
                Cmd::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn recorder_pane_draws_through_the_module_and_publishes_rect() {
        let mut s = shell();
        let v = frame(&mut s);
        assert!(s.audiorec_rect.2 > 0.0, "the module path stores the card rect");
        assert!(
            texts(&v).iter().any(|t| t.contains("Record")),
            "the recorder header / button label must render"
        );
        let k = keys(&s);
        assert!(k.contains(&crate::shell::AUDIO_REC_REC_KEY), "record button");
        assert!(k.contains(&crate::shell::AUDIO_REC_PANE_KEY), "pane chevron");
    }

    #[test]
    fn recordings_pane_draws_rows_and_toggles() {
        let mut s = shell();
        s.audiorec_pane = 1;
        let v = frame(&mut s);
        assert!(
            texts(&v).iter().any(|t| t.contains("Recordings")),
            "the recordings pane header must render"
        );
        let k = keys(&s);
        assert!(k.contains(&crate::shell::AUDIO_REC_BACK_KEY), "back chevron");
        assert!(
            k.contains(&crate::shell::AUDIO_REC_CONFIRM_KEY),
            "confirm-delete toggle"
        );
        assert!(
            k.contains(&crate::shell::AUDIO_REC_SHOWMIC_KEY),
            "show-mic toggle"
        );
        assert!(
            k.iter().any(|&x| (crate::shell::AUDIO_REC_PLAY_BASE
                ..=crate::shell::AUDIO_REC_PLAY_MAX)
                .contains(&x)),
            "per-row play key"
        );
    }

    #[test]
    fn mic_fader_rect_published_only_when_shown() {
        let mut s = shell();
        let _ = frame(&mut s);
        assert!(
            s.audiorec_mic_rect.2 > 0.0,
            "the mic fader rect is published for the drag handler"
        );
        s.audiorec_show_mic = false;
        let _ = frame(&mut s);
        assert_eq!(
            s.audiorec_mic_rect,
            (0.0, 0.0, 0.0),
            "hidden slider publishes an empty rect"
        );
    }

    #[test]
    fn delete_confirm_two_step() {
        let mut s = shell();
        s.audiorec_confirm_delete = true;
        s.audiorec_del_click(1);
        assert_eq!(s.audiorec_del_armed, Some(1), "first click arms the row");
        assert!(
            s.pending_audiorec_del.is_none(),
            "no delete on the arm click"
        );
        s.audiorec_del_click(1);
        assert_eq!(s.pending_audiorec_del, Some(1), "second click deletes");
        assert_eq!(s.audiorec_del_armed, None, "armed state clears");
        // confirm off → one click deletes straight away
        s.audiorec_confirm_delete = false;
        s.audiorec_del_click(0);
        assert_eq!(
            s.pending_audiorec_del,
            Some(0),
            "confirm-off deletes in one click"
        );
    }

    #[test]
    fn spec_defaults_resolve_on_theme() {
        // the default [cards.audiorec] spec (no toml section present) still
        // resolves every token against the live theme
        let s = shell();
        let p = crate::ui::Pal {
            fg: 0xe8e6e3ff,
            bg: 0x141414ff,
            acc: 0x4fc2ffff,
            sfg: 0x101010ff,
        };
        let spec = s.cfg.audiorec();
        assert_eq!(
            spec.del_armed.resolve(&p),
            crate::ui::DANGER,
            "del_armed default is danger"
        );
        assert_eq!(
            spec.row_ink.resolve(&p),
            crate::ui::fg2(&p),
            "row_ink default is fg2"
        );
        assert_eq!(
            spec.rec_active_surface.resolve(&p),
            crate::ui::DANGER,
            "record active = danger"
        );
        assert_eq!(spec.row_h, 24.0, "default row height");
    }

    #[test]
    fn spec_overrides_apply() {
        let toml = r#"
            [cards.audiorec]
            rec_surface = "acc"
            rec_active_surface = "ok"
            row_h = 30
        "#;
        let cfg: crate::config::Config = toml::from_str(toml).expect("overrides parse");
        let spec = cfg.audiorec();
        let p = crate::ui::Pal {
            fg: 0xe8e6e3ff,
            bg: 0x141414ff,
            acc: 0x4fc2ffff,
            sfg: 0x101010ff,
        };
        assert_eq!(
            spec.rec_surface.resolve(&p),
            p.acc,
            "rec_surface overridden to acc"
        );
        assert_eq!(
            spec.rec_active_surface.resolve(&p),
            crate::ui::OK,
            "rec_active overridden to ok"
        );
        assert_eq!(spec.row_h, 30.0, "row height overridden");
        // untouched fields keep defaults
        assert_eq!(spec.rec_dot.resolve(&p), crate::ui::DANGER);
    }
}
