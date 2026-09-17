//! Keybinding resolution: maps (modifiers, keysym) to a [`KeyAction`].
//!
//! Bindings are data-driven from the config's `[bindings]` table (hyprland-style
//! `"Mod+Return" = "terminal"` strings). Defaults live in `config.rs::default_bindings`
//! and are merged under user entries.

use smithay::input::keyboard::{Keysym, ModifiersState};

use crate::{
    config::Config,
    overview::OverviewDirection,
    workspace::SplitSide,
};

#[derive(Debug, Clone, PartialEq)]
pub enum KeyAction {
    None,
    Quit,
    /// Spawn the configured launcher.
    Launch,
    /// Spawn a terminal, optionally as the new master on the given side.
    Terminal(Option<SplitSide>),
    ToggleOverview,
    /// Switch to a previous/next non-empty workspace.
    WorkspaceSwitch(isize),
    /// Toggle overview saturation demo (cycles 0 / 0.35 / 1).
    CycleSaturation,
    /// Spawn an arbitrary command from a `exec ...` bind.
    Exec(Vec<String>),
    /// Overview navigation (overview grabs all input while active).
    Overview(OverviewDirection),
    OverviewSelect,
    OverviewNext,
    OverviewCancel,
}

/// Resolve the mod-key bitmask from the config.
pub fn mod_mask(config: &Config) -> ModifiersState {
    let mut m = ModifiersState::default();
    match config.mod_key.to_lowercase().as_str() {
        "alt" | "mod1" => m.alt = true,
        "ctrl" | "control" => m.ctrl = true,
        "shift" => m.shift = true,
        // Default: Super / Mod4.
        _ => m.logo = true,
    }
    m
}

/// Compare only the state-modifier flags (ignores serialized layout bits).
fn mods_match(a: &ModifiersState, b: &ModifiersState) -> bool {
    a.ctrl == b.ctrl && a.alt == b.alt && a.shift == b.shift && a.logo == b.logo
}

fn no_mods(m: &ModifiersState) -> bool {
    !m.ctrl && !m.alt && !m.shift && !m.logo
}

/// Normal-mode bindings: iterate the config's `[bindings]` map, first match wins.
pub fn resolve(mods: ModifiersState, keysym: Keysym, config: &Config) -> KeyAction {
    for (key, action) in &config.bindings {
        let Some((bmods, bkey)) = parse_binding_key(key) else {
            continue;
        };
        if bkey == keysym && mods_match(&mods, &bmods) {
            return match parse_action(action) {
                Some(action) => action,
                None => {
                    tracing::warn!(%key, %action, "invalid binding action in config, ignoring");
                    KeyAction::None
                }
            };
        }
    }
    KeyAction::None
}

/// Overview-mode bindings: the overview grabs every key. Fixed set for now
/// (arrows/Return/Tab/Escape plus mod actions from the config's mod key).
pub fn resolve_overview(mods: ModifiersState, keysym: Keysym, config: &Config) -> KeyAction {
    let m = mod_mask(config);
    let directional = mods_match(&mods, &m) || no_mods(&mods);

    match keysym {
        Keysym::Left if directional => KeyAction::Overview(OverviewDirection::Left),
        Keysym::Right if directional => KeyAction::Overview(OverviewDirection::Right),
        Keysym::Up if directional => KeyAction::Overview(OverviewDirection::Up),
        Keysym::Down if directional => KeyAction::Overview(OverviewDirection::Down),
        Keysym::Return => KeyAction::OverviewSelect,
        Keysym::Tab | Keysym::ISO_Left_Tab => KeyAction::OverviewNext,
        Keysym::Escape => KeyAction::OverviewCancel,
        Keysym::q | Keysym::Q if mods_match(&mods, &m) => KeyAction::Quit,
        Keysym::space if mods_match(&mods, &m) => KeyAction::Launch,
        _ => KeyAction::None,
    }
}

/// Parse a `"Mod+Return"` / `"Ctrl+Alt+t"` style key string into (modifiers, keysym).
/// The last `+`-separated token is the key; everything before it is modifiers
/// (`Super`/`Mod4`, `Alt`/`Mod1`, `Ctrl`/`Control`, `Shift`). Returns `None` if
/// any token is unrecognized.
pub fn parse_binding_key(s: &str) -> Option<(ModifiersState, Keysym)> {
    let parts: Vec<&str> = s.split('+').map(str::trim).collect();
    let (mod_parts, key_part) = parts.split_at(parts.len().saturating_sub(1));
    let key_name = key_part.first()?;
    if key_name.is_empty() {
        return None;
    }

    let mut mods = ModifiersState::default();
    for m in mod_parts {
        match m.to_ascii_lowercase().as_str() {
            "super" | "mod" | "mod4" | "logo" => mods.logo = true,
            "alt" | "mod1" => mods.alt = true,
            "ctrl" | "control" => mods.ctrl = true,
            "shift" => mods.shift = true,
            _ => return None,
        }
    }

    Some((mods, parse_keysym(key_name)?))
}

/// Parse a key name into an xkbcommon keysym (case-insensitive).
pub fn parse_keysym(s: &str) -> Option<Keysym> {
    let lower = s.to_ascii_lowercase();
    let ks = match lower.as_str() {
        "return" | "enter" => Keysym::Return,
        "space" => Keysym::space,
        "tab" => Keysym::Tab,
        "escape" | "esc" => Keysym::Escape,
        "backspace" => Keysym::BackSpace,
        "delete" | "del" => Keysym::Delete,
        "left" => Keysym::Left,
        "right" => Keysym::Right,
        "up" => Keysym::Up,
        "down" => Keysym::Down,
        "home" => Keysym::Home,
        "end" => Keysym::End,
        "page_up" | "pageup" => Keysym::Page_Up,
        "page_down" | "pagedown" => Keysym::Page_Down,
        "comma" => Keysym::comma,
        "period" | "dot" => Keysym::period,
        "slash" => Keysym::slash,
        "minus" => Keysym::minus,
        "equal" | "equals" => Keysym::equal,
        "semicolon" => Keysym::semicolon,
        "apostrophe" => Keysym::apostrophe,
        "grave" => Keysym::grave,
        "backslash" => Keysym::backslash,
        "bracketleft" | "leftbrace" => Keysym::bracketleft,
        "bracketright" | "rightbrace" => Keysym::bracketright,
        "caps_lock" | "capslock" => Keysym::Caps_Lock,
        "shift_l" => Keysym::Shift_L,
        "shift_r" => Keysym::Shift_R,
        "ctrl_l" | "control_l" => Keysym::Control_L,
        "ctrl_r" | "control_r" => Keysym::Control_R,
        "alt_l" => Keysym::Alt_L,
        "alt_r" => Keysym::Alt_R,
        "super_l" | "meta_l" => Keysym::Super_L,
        "super_r" | "meta_r" => Keysym::Super_R,
        _ => {
            // F-keys first ("f5" must not be read as letter 'f'), then single
            // letter/digit, then anything else is unknown.
            if let Some(n) = lower.strip_prefix('f').and_then(|x| x.parse::<u32>().ok()) {
                if (1..=24).contains(&n) {
                    // XKB_KEY_F1 = 0xFFBE.
                    return Some(Keysym::from(0xFFBE + n - 1));
                }
            }
            let ch = lower.chars().next()?;
            if lower.len() == 1 && (('a'..='z').contains(&ch) || ('0'..='9').contains(&ch)) {
                return Some(Keysym::from(ch as u32));
            }
            return None;
        }
    };
    Some(ks)
}

/// Parse an action string into a [`KeyAction`]. Returns `None` for unknown actions.
pub fn parse_action(s: &str) -> Option<KeyAction> {
    let t = s.trim();
    let lower = t.to_ascii_lowercase();
    match lower.as_str() {
        "quit" | "exit" => Some(KeyAction::Quit),
        "launcher" | "launch" => Some(KeyAction::Launch),
        "terminal" => Some(KeyAction::Terminal(None)),
        "terminal left" | "terminal-left" | "master left" | "master-left" => {
            Some(KeyAction::Terminal(Some(SplitSide::Left)))
        }
        "terminal right" | "terminal-right" | "master right" | "master-right" => {
            Some(KeyAction::Terminal(Some(SplitSide::Right)))
        }
        "overview" | "toggle overview" => Some(KeyAction::ToggleOverview),
        "workspace next" | "workspace-next" | "next workspace" => Some(KeyAction::WorkspaceSwitch(1)),
        "workspace prev" | "workspace-prev" | "prev workspace" | "previous workspace" => {
            Some(KeyAction::WorkspaceSwitch(-1))
        }
        "saturation" | "cycle saturation" => Some(KeyAction::CycleSaturation),
        _ => {
            // exec <cmd args...> — split on whitespace.
            if let Some(rest) = lower.strip_prefix("exec ") {
                let argv: Vec<String> = t.split_whitespace().skip(1).map(str::to_string).collect();
                if argv.is_empty() {
                    return None;
                }
                Some(KeyAction::Exec(argv))
            } else {
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_modifier_combos() {
        let (m, k) = parse_binding_key("Mod+Return").unwrap();
        assert!(m.logo && !m.ctrl && !m.alt && !m.shift);
        assert_eq!(k, Keysym::Return);

        let (m, k) = parse_binding_key("Ctrl+Alt+t").unwrap();
        assert!(m.ctrl && m.alt && !m.logo && !m.shift);
        assert_eq!(k, Keysym::from('t' as u32));

        let (m, k) = parse_binding_key("Shift+F5").unwrap();
        assert!(m.shift && !m.logo);
        assert_eq!(k, Keysym::from(0xFFBE + 4));

        assert!(parse_binding_key("Bogus+x").is_none());
        assert!(parse_binding_key("Mod").is_none());
        assert!(parse_binding_key("Mod+").is_none());
    }

    #[test]
    fn parses_keys() {
        assert_eq!(parse_keysym("space"), Some(Keysym::space));
        assert_eq!(parse_keysym("Return"), Some(Keysym::Return));
        assert_eq!(parse_keysym("left"), Some(Keysym::Left));
        assert_eq!(parse_keysym("q"), Some(Keysym::q));
        assert_eq!(parse_keysym("F1"), Some(Keysym::from(0xFFBE)));
        assert_eq!(parse_keysym("nope"), None);
    }

    #[test]
    fn parses_actions() {
        assert_eq!(parse_action("terminal"), Some(KeyAction::Terminal(None)));
        assert_eq!(
            parse_action("terminal left"),
            Some(KeyAction::Terminal(Some(SplitSide::Left)))
        );
        assert_eq!(parse_action("workspace next"), Some(KeyAction::WorkspaceSwitch(1)));
        assert_eq!(parse_action("workspace-prev"), Some(KeyAction::WorkspaceSwitch(-1)));
        assert_eq!(parse_action("quit"), Some(KeyAction::Quit));
        assert_eq!(
            parse_action("exec /usr/bin/rofi -show drun"),
            Some(KeyAction::Exec(vec![
                "/usr/bin/rofi".to_string(),
                "-show".to_string(),
                "drun".to_string()
            ]))
        );
        assert_eq!(parse_action("bogus"), None);
    }
}
