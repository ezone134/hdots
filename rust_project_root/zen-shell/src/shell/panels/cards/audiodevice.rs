use super::super::*;

impl Shell {
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



    #[test]
    fn dev_pair_is_bounded() {
        let mut s = shell();
        s.audio_sinks = vec![(1, "a".into(), 1.0, false)];
        assert_eq!(
            s.audio_dev_pair(100),
            (None, false),
            "past the end → no device, is_mute reflects parity"
        );
    }
}
