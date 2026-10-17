//! Shared zen theme layer — extracted from `zen-shell/src/{vars,themes}.rs`
//! so every project in `rust_project_root/` resolves the hybrid-session state
//! dirs and reads the live shell palette through one implementation.
//!
//! This crate knows **three** things:
//!
//! 1. **Session paths** ([`states_dir`], [`states2_dir`], …) — the `$states`
//!    (persisted) and `$states2` (volatile) dirs exported by the Hyprland
//!    session, plus per-state config/font and branding files.
//! 2. **The live palette** ([`read_shell_vars`]) — `$states2/shell_vars.json`
//!    (`{"fg": "#rrggbb", …}` written by `theme_save_hex`) with the legacy
//!    `key=#rrggbb` text file as fallback. This is what `gcp`, `sp`,
//!    `zen-image`, and the shell read to theme themselves.
//! 3. **Theme cards** ([`load`], [`parse`], [`ThemeCard`]) — the
//!    `$states2/themes` JSON mirror of the bash master, plus the wire types
//!    for a single card's detail view ([`ThemeDetail`]).
//!
//! Colors are `0xRRGGBBAA` throughout (see `zen-color`).

use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;

/// Re-exported so theme consumers get the color primitives without a separate
/// `zen-color` dependency.
pub use zen_color;

// ── session paths ────────────────────────────────────────────────────────

/// `$states` — persisted session state (syncs back to persist). Holds
/// per-state settings (`$states/acc_d`), the master theme file
/// (`$states/themes`), and per-state shell config (`$states/shell_{state}`).
pub fn states_dir() -> PathBuf {
    PathBuf::from(std::env::var("states").unwrap_or_else(|_| "/tmp/tw_rconf/states".into()))
}

/// `$states2` — volatile runtime state (never syncs to persist). Holds live
/// colors (`shell_vars.json`), the dark/light marker (`m_dummy`), and the
/// channel (`s`).
pub fn states2_dir() -> PathBuf {
    PathBuf::from(std::env::var("states2").unwrap_or_else(|_| "/tmp/tw_rconf/states2".into()))
}

/// Current state channel (`$states2/s`) — `n` | `d` | `l`. Unreadable → `d`.
pub fn read_channel() -> char {
    std::fs::read_to_string(states2_dir().join("s"))
        .ok()
        .and_then(|s| s.trim().chars().next())
        .unwrap_or('d')
}

/// Per-state shell config path (`$states/shell_{state}`).
pub fn per_state_shell_path(state: char) -> PathBuf {
    states_dir().join(format!("shell_{state}"))
}

/// Per-state UI font path (`$states/font_{state}`).
pub fn per_state_font_path(state: char) -> PathBuf {
    states_dir().join(format!("font_{state}"))
}

/// Branding font family (`$states2/branding_font`); missing → `"opensuse"`.
pub fn read_branding_font() -> String {
    std::fs::read_to_string(states2_dir().join("branding_font"))
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "opensuse".to_string())
}

/// Branding glyph text (`$states2/d`); missing → `"o"`.
pub fn read_branding_glyph() -> String {
    std::fs::read_to_string(states2_dir().join("d"))
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "o".to_string())
}

// ── live palette ─────────────────────────────────────────────────────────

/// Aggregated live-color file (JSON — `$states2/shell_vars.json`).
pub fn shell_vars_json_path() -> PathBuf {
    states2_dir().join("shell_vars.json")
}

/// Legacy `$states2/shell_vars` (`key=#rrggbb` lines) — read only as a
/// fallback when the JSON mirror is missing or unparseable.
pub fn shell_vars_path() -> PathBuf {
    states2_dir().join("shell_vars")
}

/// Current dark/light marker file (`$states2/m_dummy`, `d` | `l`).
pub fn m_dummy_path() -> PathBuf {
    states2_dir().join("m_dummy")
}

/// Read the live color palette into a `key → 0xRRGGBBAA` map.
///
/// Source is the JSON mirror `$states2/shell_vars.json`; falls back to the
/// legacy `key=#rrggbb` text file. Empty map when neither is readable.
pub fn read_shell_vars() -> HashMap<String, u32> {
    if let Some(m) = read_json_shell_vars() {
        return m;
    }
    let mut m = HashMap::new();
    let Ok(text) = std::fs::read_to_string(shell_vars_path()) else {
        return m;
    };
    for line in text.lines() {
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        m.insert(k.trim().to_string(), hex(v));
    }
    m
}

fn read_json_shell_vars() -> Option<HashMap<String, u32>> {
    let text = std::fs::read_to_string(shell_vars_json_path()).ok()?;
    let m = serde_json::from_str::<HashMap<String, String>>(&text).ok()?;
    Some(m.into_iter().map(|(k, v)| (k, hex(&v))).collect())
}

/// Read the dark/light marker: `l` → false (light), anything else → true.
pub fn is_dark_mode() -> bool {
    std::fs::read_to_string(m_dummy_path())
        .map(|s| !s.trim().starts_with('l'))
        .unwrap_or(true)
}

/// Parse `#rrggbb` (alpha optional) into opaque `0xRRGGBBAA`. A malformed
/// master line returns loud magenta so the breakage is visible.
pub fn hex(s: &str) -> u32 {
    zen_color::parse_hex_or_magenta(s)
}

/// Parse `#rrggbb` (or `rrggbb`) into `0xRRGGBBAA` with full alpha, falling
/// back to `default` on malformed input.
pub fn parse_color(s: &str, default: u32) -> u32 {
    zen_color::parse_hex_or(s, default)
}

// ── theme cards ──────────────────────────────────────────────────────────

/// One palette half of a theme card (the colors a card shows).
#[derive(Clone, Copy, Debug, Deserialize)]
pub struct Palette {
    pub bg: u32,
    pub fg: u32,
    pub sfg: u32,
    pub acc: u32,
    /// border / outline color (accent border for apps + the card preview ring)
    pub border: u32,
}

/// One row of the master theme file → one card in the Themes panel.
#[derive(Clone, Debug, Deserialize)]
pub struct ThemeCard {
    pub name: String,
    /// true when a dual theme (dark + light palettes)
    pub dual: bool,
    pub dark: Palette,
    /// light half of a dual theme (None for single-mode themes)
    pub light: Option<Palette>,
}

/// Wire format of `$states2/themes` — colors are `#rrggbb` strings on disk.
#[derive(Deserialize)]
struct WireCard {
    name: String,
    dual: bool,
    dark: WirePalette,
    light: Option<WirePalette>,
}

#[derive(Deserialize)]
struct WirePalette {
    bg: String,
    fg: String,
    sfg: String,
    acc: String,
    #[serde(default)]
    border: Option<String>,
}

/// Rust shell theme cards mirror (`$states2/themes`).
pub fn master_path() -> PathBuf {
    states2_dir().join("themes")
}

/// Current theme id (`$states2/cur_theme`), empty when unreadable.
pub fn cur_theme() -> String {
    std::fs::read_to_string(states2_dir().join("cur_theme"))
        .map(|s| s.trim().to_string())
        .unwrap_or_default()
}

/// Current color-scheme id (`$states2/cur_scheme`), empty when unreadable.
pub fn cur_scheme() -> String {
    std::fs::read_to_string(states2_dir().join("cur_scheme"))
        .map(|s| s.trim().to_string())
        .unwrap_or_default()
}

/// Current UI state's friendly name (`$states2/s_String`), e.g. `Split`.
pub fn channel_string() -> String {
    std::fs::read_to_string(states2_dir().join("s_String"))
        .map(|s| s.trim().to_string())
        .unwrap_or_default()
}

/// Current accent source label for this channel. When `$states/acc_source_${s}`
/// is `0` (theme-default accent) returns empty; otherwise the friendly name
/// from `$states2/acc_Source` (falling back to the flag code).
pub fn acc_source_string() -> String {
    let ch = read_channel();
    let flag = std::fs::read_to_string(states_dir().join(format!("acc_source_{ch}")))
        .map(|s| s.trim().to_string())
        .unwrap_or_default();
    if flag == "0" || flag.is_empty() {
        return String::new();
    }
    match std::fs::read_to_string(states2_dir().join("acc_Source")) {
        Ok(s) if !s.trim().is_empty() => s.trim().to_string(),
        _ => match flag.as_str() {
            "w" => "Wallpaper".to_string(),
            "c" => "Custom Accent".to_string(),
            "h" => "Defined Hex".to_string(),
            _ => String::new(),
        },
    }
}

/// Load + parse the whole `$states2/themes` JSON mirror into cards.
pub fn load() -> Vec<ThemeCard> {
    let text = match std::fs::read_to_string(master_path()) {
        Ok(t) => t,
        Err(_) => return Vec::new(),
    };
    parse(&text)
}

/// Parse `$states2/themes` (wire `#rrggbb` strings) into cards.
pub fn parse(text: &str) -> Vec<ThemeCard> {
    let Ok(cards) = serde_json::from_str::<Vec<WireCard>>(text) else {
        return Vec::new();
    };
    cards
        .into_iter()
        .map(|c| ThemeCard {
            name: c.name,
            dual: c.dual,
            dark: wire_to_palette(&c.dark),
            light: c.light.as_ref().map(wire_to_palette),
        })
        .collect()
}

fn wire_to_palette(w: &WirePalette) -> Palette {
    // border is optional in the wire format (older mirrors); fall back to a
    // neutral — sfg — so the preview ring is always drawable.
    let border = w.border.as_deref().map(hex).unwrap_or_else(|| hex(&w.sfg));
    Palette {
        bg: hex(&w.bg),
        fg: hex(&w.fg),
        sfg: hex(&w.sfg),
        acc: hex(&w.acc),
        border,
    }
}

// ── keybinds ─────────────────────────────────────────────────────────────

/// One entry of `$states2/keybinds_cache`: `{ "key": "SUPER + T", "desc": … }`.
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Keybind {
    pub key: String,
    pub desc: String,
}

/// `$states2/keybinds_cache` path.
pub fn keybinds_path() -> PathBuf {
    states2_dir().join("keybinds_cache")
}

/// Load + parse `$states2/keybinds_cache` into rows; empty when missing.
pub fn load_keybinds() -> Vec<Keybind> {
    let text = match std::fs::read_to_string(keybinds_path()) {
        Ok(t) => t,
        Err(_) => return Vec::new(),
    };
    serde_json::from_str::<Vec<Keybind>>(&text).unwrap_or_default()
}

// ── theme detail (the ">" button wire) ───────────────────────────────────

/// One side of a fetched theme (dark side, or light side of a dual), exactly
/// as `scheme_main fetch` emits it.
#[derive(Clone, Debug, Deserialize)]
pub struct WireSide {
    pub bg: String,
    pub fg: String,
    pub acc: String,
    pub sfg: String,
    pub border: String,
    pub acc_us: String,
    pub acc_hc: String,
    pub acc_ec: String,
    pub extras: WireExtras,
    #[serde(default)]
    pub series: Vec<String>,
}

/// Extras tier of a fetched theme side (fallback values when flag = 0).
#[derive(Clone, Debug, Deserialize)]
pub struct WireExtras {
    pub hover: String,
    pub focus: String,
    pub disabled: String,
    pub sfg_d: String,
    pub sfg_l: String,
}

/// One fetched theme card's full color scheme (dark side + optional light).
#[derive(Clone, Debug, Deserialize)]
pub struct ThemeDetail {
    pub name: String,
    pub mode: String,
    #[allow(dead_code)]
    pub dual: bool,
    #[serde(default)]
    pub e1: u32,
    #[serde(default)]
    pub s1: u32,
    #[serde(default)]
    pub e2: u32,
    #[serde(default)]
    pub s2: u32,
    #[serde(default)]
    pub dark: Option<WireSide>,
    #[serde(default)]
    pub light: Option<WireSide>,
}

/// Run `scheme_main fetch <id>` and parse its stdout JSON.
pub fn fetch_theme_detail(id: &str) -> Option<ThemeDetail> {
    use std::process::Command;
    let out = Command::new("scheme_main")
        .args(["fetch", id])
        .output()
        .ok()?;
    serde_json::from_slice::<ThemeDetail>(&out.stdout).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_json_mirror() {
        let json = r##"[
{"name":"hdots_d","dual":false,"dark":{"bg":"#303642","fg":"#FDFCFB","acc":"#5c636d","sfg":"#F3F4F6","border":"#e5e9f0"},"light":null},
{"name":"dracula","dual":true,"dark":{"bg":"#282a36","fg":"#f8f8f2","acc":"#a33b74","sfg":"#f3f4f6","border":"#44475a"},"light":{"bg":"#f8f8f2","fg":"#282a36","acc":"#ff5555","sfg":"#f3f4f6","border":"#6272a4"}}
]"##;
        let cards = parse(json);
        assert_eq!(cards.len(), 2);
        assert!(cards[1].dual);
        let light = cards[1].light.unwrap();
        assert_eq!((light.acc >> 8), 0xff5555);
        assert_eq!((light.border >> 8), 0x6272a4);
    }

    #[test]
    fn hex_magenta_on_garbage() {
        assert_eq!(hex("nope"), zen_color::MAGENTA);
        assert_eq!(hex("#aabbcc"), 0xaabbccff);
    }

    #[test]
    fn parses_fetch_detail_wire_single() {
        let json = r##"{"name":"a","mode":"d","dual":false,"e1":0,"s1":0,"e2":0,"s2":0,"dark":{"bg":"#141218","fg":"#e6e1e5","acc":"#385b91","sfg":"#f3f4f6","border":"#6a6d75","acc_us":"#a0a4ac","acc_hc":"#9aa0a8","acc_ec":"#181924","extras":{"hover":"#1c1a20","focus":"#6a6d75","disabled":"#6a6d75","sfg_d":"#1a1a1e","sfg_l":"#f3f4f6"},"series":["#4ea1ff"]},"light":null}"##;
        let d: ThemeDetail = serde_json::from_str(json).unwrap();
        assert!(!d.dual);
        assert!(d.light.is_none());
        assert!(d.dark.is_some());
    }
}
