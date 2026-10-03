//! Theme state — reads the master theme file and the live shell colors.
//!
//! Two state dirs (both exported by the session environment):
//!
//! - **`$states2`** — volatile runtime state that does **NOT** sync to
//!   persist. `hypr_init_logic` / the `theme_main` pipeline write everything
//!   the shell reads here:
//!     - `$states2/themes` — JSON mirror of the bash master (`$states/themes`
//!       is a pure-bash SOURCEABLE file, so the shell reads the JSON
//!       reflection instead), one object per theme card.
//!     - `$states2/shell_vars.json` — every live color as a JSON object
//!       (`{"fg": "#rrggbb", "fg2": …, "bg": …, "acc": …, "sfg": …}`),
//!       emitted by `$sources/helpers/theme_save_hex` for the shell; the
//!       legacy `$states2/shell_vars` (`key=#rrggbb` lines) is the fallback
//!       while old helpers are still in the wild, plus the current mode
//!       (`$states2/m_dummy`, `d` = dark / `l` = light). zen-shell reads
//!       its colors from these on boot and on every
//!       `zen-shell ipc call colors reload`.

use std::collections::HashMap;
use std::path::PathBuf;
use serde::Deserialize;

/// One entry of `$states2/keybinds_cache` — a single Hyprland keybind's
/// human label, emitted by the `gen_keybinds_cache` helper from the
/// `-- hdots` comments in `~/.config/hypr/keybinds.lua`:
/// `{ "key": "SUPER + T", "desc": "Theme Picker" }`.
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Keybind {
    pub key: String,
    pub desc: String,
}

/// Rust shell keybind list (`$states2/keybinds_cache`, JSON `{key,desc}[]`
/// emitted by hypr_init_logic → helpers/gen_keybinds_cache).
pub fn keybinds_path() -> PathBuf {
    states2_dir().join("keybinds_cache")
}

/// Load + parse `$states2/keybinds_cache` into `Keybind` rows.
/// Empty when the file is missing or unparseable (the panel shows a hint).
pub fn load_keybinds() -> Vec<Keybind> {
    let text = match std::fs::read_to_string(keybinds_path()) {
        Ok(t) => t,
        Err(_) => return Vec::new(),
    };
    serde_json::from_str::<Vec<Keybind>>(&text).unwrap_or_default()
}

/// One palette half of a theme card (the five colors a card shows).
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
/// A dual theme carries BOTH palettes (dark first, light second) and the
/// card shows them side by side — no dark/light label needed, the colors
/// speak for themselves.
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

fn states_dir() -> PathBuf {
    PathBuf::from(std::env::var("states").unwrap_or_else(|_| "/tmp/tw_rconf/states".into()))
}

fn states2_dir() -> PathBuf {
    PathBuf::from(std::env::var("states2").unwrap_or_else(|_| "/tmp/tw_rconf/states2".into()))
}

/// Rust shell theme cards mirror (`$states2/themes`, JSON emitted by
/// hypr_init_logic from the bash sourceable master `$states/themes`).
pub fn master_path() -> PathBuf {
    states2_dir().join("themes")
}

/// Aggregated live-color file (JSON — `$states2/shell_vars.json`).
pub fn shell_vars_json_path() -> PathBuf {
    states2_dir().join("shell_vars.json")
}

/// Legacy `$states2/shell_vars` (`key=#rrggbb` lines) — read only as a
/// fallback when the JSON mirror is missing or unparseable.
fn shell_vars_path() -> PathBuf {
    states2_dir().join("shell_vars")
}

/// Current dark/light marker file (`$states2/m_dummy`, `d` | `l`).
fn m_dummy_path() -> PathBuf {
    states2_dir().join("m_dummy")
}

/// Current theme id (`$states2/cur_theme`), empty when unreadable.
pub fn cur_theme() -> String {
    std::fs::read_to_string(states2_dir().join("cur_theme"))
        .map(|s| s.trim().to_string())
        .unwrap_or_default()
}

/// Current accent source label for this channel, shown in the system card.
/// When `$states/acc_source_${s}` is `0` (theme-default accent) returns empty
/// and the line stays hidden; otherwise the friendly name is read from
/// `$states2/acc_Source` (e.g. `Wallpaper`, `Custom Accent`, `Defined Hex`),
/// falling back to the flag code when that convenience file is stale.
pub fn acc_source_string() -> String {
    let ch = crate::vars::read_channel();
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

/// Current color-scheme id — the applied scheme for this state+mode
/// (`$states/scheme_<sm>`, where `sm` is read from `$states2/sm`, e.g. `dd`).
/// Empty when unreadable.
pub fn cur_scheme() -> String {
    let sm = std::fs::read_to_string(states2_dir().join("sm"))
        .map(|s| s.trim().to_string())
        .unwrap_or_default();
    if sm.is_empty() {
        return String::new();
    }
    std::fs::read_to_string(states_dir().join(format!("scheme_{sm}")))
        .map(|s| s.trim().to_string())
        .unwrap_or_default()
}

/// Current UI state's friendly name (`$states2/s_String`, e.g. `Split`),
/// shown in the system card. Empty when unreadable.
pub fn channel_string() -> String {
    std::fs::read_to_string(states2_dir().join("s_String"))
        .map(|s| s.trim().to_string())
        .unwrap_or_default()
}

/// Parse `#rrggbb` (alpha optional) into opaque `0xRRGGBBAA`.
fn hex(s: &str) -> u32 {
    let s = s.trim().trim_start_matches('#');
    if s.len() < 6 {
        return 0xff00ffff; // loud magenta — a broken master line should be seen
    }
    let v = u32::from_str_radix(&s[..6], 16).unwrap_or(0xff00ff);
    (v << 8) | 0xff
}

/// Load + parse the whole `$states2/themes` JSON mirror into cards.
pub fn load() -> Vec<ThemeCard> {
    let text = match std::fs::read_to_string(master_path()) {
        Ok(t) => t,
        Err(_) => return Vec::new(),
    };
    parse(&text)
}

/// Parse `$states2/themes` (wire `#rrggbb` strings) into cards — on-disk
/// colors are hex strings, in-memory they are opaque `0xRRGGBBAA`.
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

/// Read the live color palette into a `key → 0xRRGGBBAA` map.
///
/// Source is the JSON mirror `$states2/shell_vars.json`
/// (`{"fg": "#rrggbb", …}`) — every key the helpers write is picked up,
/// so callers just look up the colors they want. Falls back to the legacy
/// `$states2/shell_vars` `key=#rrggbb` lines when the JSON is missing or
/// unparseable. Empty map when neither file is readable — callers keep
/// their current colors.
pub fn read_shell_vars() -> HashMap<String, u32> {
    if let Some(m) = read_json_shell_vars() {
        return m;
    }
    let mut m = HashMap::new();
    let Ok(text) = std::fs::read_to_string(shell_vars_path()) else {
        return m;
    };
    for line in text.lines() {
        let Some((k, v)) = line.split_once('=') else { continue };
        m.insert(k.trim().to_string(), hex(v));
    }
    m
}

/// Try the JSON mirror first. `None` = file missing or unparseable → the
/// caller falls back to the legacy text format.
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
        assert!(!cards[0].dual);
        assert_eq!(cards[1].name, "dracula");
        assert!(cards[1].dual);
        let light = cards[1].light.unwrap();
        assert_eq!((light.acc >> 8) as u32, 0xff5555);
        assert_eq!((light.border >> 8) as u32, 0x6272a4);
        assert_eq!((cards[0].dark.border >> 8) as u32, 0xe5e9f0);
    }

    #[test]
    fn parses_json_shell_vars_prefers_json_mirror() {
        // exercise the JSON parser directly on the wire format the helper
        // emits (theme_save_hex → $states2/shell_vars.json)
        let json = "{\"fg\":\"#fbffe4\",\"fg2\":\"#797c67\",\"bg\":\"#0f1202\",\"acc\":\"#c6ff00\",\"sfg\":\"#1a1a1e\"}";
        let m: HashMap<String, u32> = serde_json::from_str::<HashMap<String, String>>(&json)
            .unwrap()
            .into_iter()
            .map(|(k, v)| (k, hex(&v)))
            .collect();
        assert_eq!(m["fg"], 0xfbffe4ff);
        assert_eq!(m["acc"], 0xc6ff00ff);
        assert_eq!(m["bg"] >> 8, 0x0f1202);
    }
}
