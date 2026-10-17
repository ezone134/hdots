//! Centralized session-path resolution.
//!
//! The runtime state-dir and branding readers now live in the shared
//! **`zen-theme`** crate (`gcp`, `sp`, `zen-image`, … all read the same
//! `$states` / `$states2` layout), so this module is a thin re-export that
//! keeps the existing `crate::vars::…` call sites working. Only the few
//! shell-specific, test-only helpers stay here.
//!
//! See `zen_theme` for the full contract of `$states` (persisted) and
//! `$states2` (volatile) plus the documented fallbacks.

pub use zen_theme::{
    per_state_font_path, per_state_shell_path, read_branding_font, read_branding_glyph,
    read_channel, states2_dir, states_dir,
};

#[cfg(test)]
use std::path::PathBuf;

/// `$XDG_CONFIG_HOME/zen-shell` (default `~/.config/zen-shell`) — the
/// hand-authored declarative config root. Test-seed paths only now.
#[cfg(test)]
pub fn config_root() -> PathBuf {
    PathBuf::from(
        std::env::var("XDG_CONFIG_HOME")
            .map(|d| format!("{d}/zen-shell"))
            .unwrap_or_else(|_| "~/.config/zen-shell".into()),
    )
}

/// `~/.config/zen-shell/ui/card_scenes_dir` — the declarative card-specified
/// scene files (`ui/cards/*.ron`) the legacy reader still parses in tests.
#[cfg(test)]
pub fn card_scenes_dir() -> PathBuf {
    config_root().join("ui").join("cards")
}
