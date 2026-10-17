use super::super::*;

impl Shell {
    /// Whether the mirror card's pagination dots should show right now —
    /// shared by the Rust drawer and the declared-scene `Dots.when` binding
    /// (cursor rests on the card, edit mode off).
    pub(crate) fn mirror_dots_hover(&self) -> bool {
        let (x, y, w, h) = self.mirror_rect;
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
    use crate::config::Config;
    use crate::shell::Shell;

    fn newg() -> Shell {
        let cfg: Config =
            toml::from_str(crate::config::DEFAULT_SHELL_TOML).expect("default parses");
        let mut shell = Shell::new(cfg);
        shell.mode = Mode::Expanded;
        shell.dash_enabled = true;
        shell.cam_ok = true;
        shell
    }

    #[test]
    fn fps_row_sets_rate_and_persists() {
        let mut s = newg();
        s.mirror_fps = 60;
        let key = MIRROR_FPS_ROW_BASE + 1;
        s.region(100.0, 100.0, 50.0, 50.0, key);
        s.set_cursor(Some((120.0, 120.0)));
        let _ = s.click(120.0, 120.0);
        assert_eq!(s.mirror_fps, 30, "row click should set the rate");
        assert!(s.pending_mirror_fps, "pending flag should be set");
    }

    #[test]
    fn fps_show_toggle_click_flips_flag() {
        // the show-fps switch (30704) is an ordinary Rust click arm now
        let mut s = newg();
        assert!(s.mirror_show_fps, "defaults on");
        s.region(90.0, 90.0, 60.0, 30.0, 30704);
        s.set_cursor(Some((100.0, 100.0)));
        let _ = s.click(100.0, 100.0);
        assert!(!s.mirror_show_fps, "toggle should flip to off");
        let _ = s.click(100.0, 100.0);
        assert!(s.mirror_show_fps, "toggle should flip back on");
    }
}
