//! Centralized session-path resolution.
//!
//! Everything that touches the hybrid-session state lives here so the paths
//! can be changed in ONE place — no scattered env reads, no hardcoded fallback
//! dirs scattered across the shell. The `states` and `states2` env vars are
//! ALWAYS set by the Hyprland session — never verify these directories here;
//! treat them as present and write/read directly.
//!
//! Two state dirs are exported by the session environment:
//!
//! - **`$states`** — persisted session state (**syncs back to persist when the
//!   session ends**). Holds per-state settings (files suffixed `_d` / `_l` /
//!   `_n`, e.g. `$states/acc_d`), the master theme file `$states/themes`, and
//!   this shell's per-state config `$states/shell_{state}`.
//! - **`$states2`** — volatile runtime state that does **NOT** sync to
//!   persist (pure runtime, reset each session). `theme_main` writes live
//!   colors (`$states2/shell_vars`), the dark/light marker
//!   (`$states2/m_dummy`), and the current channel (`$states2/s`).
//!
//! Both fall back to `/tmp/tw_rconf/{states,states2}` when the env vars
//! are unset (the session's default location — not normally reached).

use std::path::PathBuf;

/// `$XDG_CONFIG_HOME/zen-shell` (default `~/.config/zen-shell`) — the
/// hand-authored declarative config root (hybrid): scenes, card specs and
/// other authored UI files live here PERMANENTLY, while runtime settings
/// (toggles, dashboard layout) keep auto-saving to `$states/shell_{state}`.
pub fn config_root() -> PathBuf {
    std::env::var("XDG_CONFIG_HOME")
        .ok()
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/home/tw".into());
            PathBuf::from(home).join(".config")
        })
        .join("zen-shell")
}

/// `~/.config/zen-shell/ui/cards/` — the committed per-card scene directory.
pub fn card_scenes_dir() -> PathBuf {
    config_root().join("ui").join("cards")
}

/// `~/.config/zen-shell/ui/cards/<id>.ron` — the declarative scene file for a
/// card. Missing file → the card falls back to its built-in Rust draw.
pub fn card_scene_path(card_id: &str) -> PathBuf {
    card_scenes_dir().join(format!("{card_id}.ron"))
}

/// `~/.config/zen-shell/shell.ron` — the whole-shell declarative scene
/// (named surfaces + reusable component templates). Missing file → every
/// surface falls back to its built-in Rust draw.
pub fn shell_scene_path() -> PathBuf {
    config_root().join("shell.ron")
}

/// `$states` — persisted session state (syncs back to persist).
pub fn states_dir() -> PathBuf {
    PathBuf::from(std::env::var("states").unwrap_or_else(|_| "/tmp/tw_rconf/states".into()))
}

/// `$states2` — volatile runtime state (never syncs to persist).
pub fn states2_dir() -> PathBuf {
    PathBuf::from(std::env::var("states2").unwrap_or_else(|_| "/tmp/tw_rconf/states2".into()))
}

/// Current state channel (`$states2/s`) — `n` | `d` | `l`.
/// This is the shell's source of truth for which per-state config to load.
/// Unreadable defaults to `d` (dark).
pub fn read_channel() -> char {
    std::fs::read_to_string(states2_dir().join("s"))
        .ok()
        .and_then(|s| s.trim().chars().next())
        .unwrap_or('d')
}

/// Path to the per-state shell config (`$states/shell_{state}`) — the shell's
/// MASTER config for that channel (dark/light/night). The shell reads the
/// current channel from `$states2/s` and loads/saves ALL settings here.
/// `$states` is mirrored to `$hdots/states/` by `sync_back` when the session
/// ends, so edits survive a reboot. `$states` always exists; the file is
/// written directly on first save.
pub fn per_state_shell_path(state: char) -> PathBuf {
    states_dir().join(format!("shell_{state}"))
}

/// Path to the per-state UI font file (`$states/font_{state}`). Contains the
/// UI (text) family name — one line, e.g. "Satoshi Variable". The icon family
/// derives from the icon style (`fonts.icon_style`), overridable via config
/// `fonts.icon`; only the UI font lives here.
pub fn per_state_font_path(state: char) -> PathBuf {
    states_dir().join(format!("font_{state}"))
}

/// Branding font family name — `$states2/branding_font` (volatile runtime,
/// never synced to persist). Renders the brand glyph on the Branding card /
/// banner chip / resting pill. Missing file → "opensuse" (the session theme's
/// brand font).
pub fn read_branding_font() -> String {
    std::fs::read_to_string(states2_dir().join("branding_font"))
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "opensuse".to_string())
}

/// Branding glyph text — `$states2/d` (the small word the brand font renders,
/// e.g. "o"). Missing file → "o".
pub fn read_branding_glyph() -> String {
    std::fs::read_to_string(states2_dir().join("d"))
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "o".to_string())
}
