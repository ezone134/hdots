use super::super::*;

impl Shell {
    /// SLIDERS — two-pane card. Pane 0: the six fader rows; pane 1: the
    /// "show balance" / "show saturation" switches that add/remove rows
    /// from pane 0. The whole body is `sliders.ron` (zero Ink) — this
    /// helper only answers the `Dots` hover gate, and the Rust drawer
    /// below stays the fallback for when no scene is authored.
    pub(crate) fn sliders_dots_hover(&self) -> bool {
        let (x, y, w, h) = self.sliders_rect;
        !self.dash_edit
            && w > 0.0
            && self
                .cursor
                .map(|(px, py)| Self::in_rect(px, py, (x, y, w, h)))
                .unwrap_or(false)
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    fn newg() -> crate::shell::Shell {
        let cfg: crate::config::Config =
            toml::from_str(crate::config::DEFAULT_SHELL_TOML).expect("default parses");
        let mut shell = crate::shell::Shell::new(cfg);
        shell.mode = crate::shell::Mode::Expanded;
        shell.dash_enabled = true;
        shell
    }


    #[test]
    fn lua_card_drag_geometry_is_published() {
        // `set_dash_slider` drags read the geometry that the DRAW writes into
        // `faders` / `balance_rect`. The sliders card is a Lua module now, and
        // the Lua draw path (unlike `draw_card_scene`) must still publish that
        // cache or the sliders all come out dead.
        let mut s = newg();
        let pal = crate::ui::Pal {
            fg: 0xe8e6e3ff,
            bg: 0x141414ff,
            acc: 0x4fc2ffff,
            sfg: 0x101010ff,
        };
        let mut v: Vec<Cmd> = Vec::new();
        let vals = crate::scene::SceneValues::new();
        let drew =
            s.draw_lua_if_any("sliders", &mut v, 10.0, 10.0, 300.0, 280.0, &pal, &vals);
        assert!(drew, "sliders card has a Lua module");
        assert!(!v.is_empty(), "the module drew something");
        assert!(
            s.faders[0].2 > 1.0,
            "brightness groove registered: {:?}",
            s.faders
        );
        assert!(
            s.faders[4].2 > 1.0,
            "kelvin groove registered: {:?}",
            s.faders
        );
        assert!(
            s.balance_rect.2 > 1.0,
            "balance groove registered: {:?}",
            s.balance_rect
        );
    }

    #[test]
    fn sliders_toggles_click_flip_and_queue_scr_main() {
        // the pane-2 toggles (30720..30722) are ordinary Rust click arms now
        let mut s = newg();
        assert!(s.show_balance && s.show_saturation && s.sst_on, "defaults on");
        for key in [30720, 30721, 30722] {
            s.region(80.0, 80.0, 60.0, 30.0, key);
            s.set_cursor(Some((100.0, 100.0)));
            let _ = s.click(100.0, 100.0);
        }
        assert!(!s.show_balance, "toggle should flip off");
        assert!(!s.show_saturation, "toggle should flip off");
        assert!(
            !s.sst_on,
            "the switch flips optimistically"
        );
        assert!(
            matches!(s.pending_dash, Some(DashCmd::SstToggle)),
            "the file write is scr_main's job, not a config key"
        );
        // and there is genuinely nowhere in this shell's own config for it to
        // hide: a key added later would be a second copy of the flag to drift
        let dumped = toml::to_string(&s.cfg).unwrap_or_default();
        assert!(
            !dumped.contains("sst"),
            "no sst key in the shell config: {dumped:?}"
        );
    }

}
