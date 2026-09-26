use std::{collections::BTreeMap, path::Path};

use serde::Deserialize;

/// A command to run: either a bare program name or a full argv.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum ExecCmd {
    /// Just a program name, no arguments.
    Cmd(String),
    /// Full argv (program + arguments).
    Argv(Vec<String>),
}

impl ExecCmd {
    pub fn argv(&self) -> Vec<String> {
        match self {
            ExecCmd::Cmd(c) => vec![c.clone()],
            ExecCmd::Argv(a) => a.clone(),
        }
    }
}

/// Hyprland-style `[autostart]` section (like `exec-once`): commands run at startup.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct Autostart {
    pub exec: Vec<ExecCmd>,
}

/// zen-wm configuration, loaded from `~/.config/zen-wm/config.toml` (or `$XDG_CONFIG_HOME`).
///
/// The file is watched (mtime poll) and hot-reloaded while the compositor runs
/// (see `ZenWm::check_config_reload`): every field takes effect live.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Custom keybindings, hyprland-style: `"Mod+Return" = "terminal"`.
    /// Entries override the default binds by key; unmentioned keys keep their default.
    /// Action names: `terminal`, `terminal left/right`, `launcher`, `overview`,
    /// `workspace next/prev`, `saturation`, `quit`, or `exec <cmd args...>`.
    pub bindings: BTreeMap<String, String>,
    /// Programs to launch on compositor start (exec-once).
    pub autostart: Autostart,
    /// Modifier used for all keybindings: `"Super"`, `"Alt"`, `"Ctrl"` or `"Shift"`.
    pub mod_key: String,
    /// Window arrangement: `"scrolling"` (default, niri-flavored full-width vertical
    /// column) or `"master"` (dwm-style master/stack).
    pub layout: String,
    /// Terminal command (argv), spawned with Super+Return.
    pub terminal: Vec<String>,
    /// Launcher command (argv), spawned with Super+Space.
    pub launcher: Vec<String>,
    /// Gap between windows and screen edge, in logical px.
    pub gap: i32,
    /// Global color saturation: 0.0 = grayscale, 1.0 = normal, >1.0 = oversaturated.
    pub saturation: f32,
    /// Workspace overview grid.
    pub workspace_columns: u32,
    pub workspace_rows: u32,
    /// Total number of workspaces.
    pub workspace_count: usize,
    /// Alpha applied to the overview background + window dimming.
    pub overview_dim: f32,
    /// Colors (linear-ish 0-255 RGB).
    pub background: [u8; 3],
    pub overview_bg: [u8; 3],
    pub overview_highlight: [u8; 3],
}

impl Default for Config {
    fn default() -> Self {
        let mut s = Self {
            mod_key: "Super".to_string(),
            layout: "scrolling".to_string(),
            terminal: vec!["alacritty".to_string()],
            launcher: vec!["fuzzel".to_string()],
            gap: 4,
            saturation: 1.0,
            workspace_columns: 1,
            workspace_rows: 8,
            workspace_count: 8,
            overview_dim: 0.65,
            background: [0x11, 0x11, 0x18],
            overview_bg: [0x0c, 0x0c, 0x10],
            overview_highlight: [0x2d, 0x9c, 0xd3],
            bindings: BTreeMap::new(),
            autostart: Autostart::default(),
        };
        s.bindings = default_bindings();
        s
    }
}

/// The default hyprland-style binds (merge base for user `[bindings]` entries).
fn default_bindings() -> BTreeMap<String, String> {
    [
        ("Mod+Return", "terminal"),
        ("Mod+Left", "terminal left"),
        ("Mod+Right", "terminal right"),
        ("Mod+Space", "launcher"),
        ("Mod+Up", "workspace prev"),
        ("Mod+Down", "workspace next"),
        ("Mod+Tab", "overview"),
        ("Mod+s", "saturation"),
        ("Mod+q", "quit"),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v.to_string()))
    .collect()
}

impl Config {
    pub fn load(path: &Path) -> Self {
        let raw = match std::fs::read_to_string(path) {
            Ok(raw) => raw,
            Err(_) => {
                tracing::warn!(path = %path.display(), "no config found, using defaults");
                return Self::default();
            }
        };
        match toml::from_str::<Config>(&raw) {
            Ok(mut cfg) => {
                // Merge: user bindings win, defaults fill the rest.
                for (k, v) in default_bindings() {
                    cfg.bindings.entry(k).or_insert(v);
                }
                cfg.sanitize();
                cfg
            }
            Err(e) => {
                tracing::warn!(path = %path.display(), error = %e, "invalid config, using defaults");
                Self::default()
            }
        }
    }

    fn sanitize(&mut self) {
        self.gap = self.gap.max(0);
        self.saturation = self.saturation.clamp(0.0, 3.0);
        self.workspace_count = self.workspace_count.max(1);
        self.workspace_columns = self.workspace_columns.max(1);
        self.workspace_rows = self.workspace_rows.max(1);
        self.overview_dim = self.overview_dim.clamp(0.0, 1.0);
        if self.terminal.is_empty() {
            self.terminal = vec!["alacritty".to_string()];
        }
        if self.launcher.is_empty() {
            self.launcher = vec!["fuzzel".to_string()];
        }
        match self.layout.to_lowercase().as_str() {
            "master" | "dwm" => self.layout = "master".to_string(),
            // Default: niri-flavored scrolling layout.
            _ => self.layout = "scrolling".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merges_bindings_and_parses_autostart() {
        let tmp = std::env::temp_dir().join("zen-wm-config-test.toml");
        std::fs::write(
            &tmp,
            r#"
            [autostart]
            exec = [ "/bin/true", ["/bin/echo", "hi"] ]
            [bindings]
            "Mod+x" = "quit"
            "#,
        )
        .unwrap();
        let cfg = Config::load(&tmp);
        let _ = std::fs::remove_file(&tmp);

        // User entry wins over the default for the same key.
        assert_eq!(cfg.bindings.get("Mod+x").map(String::as_str), Some("quit"));
        // Unmentioned defaults are still present (9 defaults + 1 new = 10).
        assert_eq!(cfg.bindings.len(), 10);
        assert_eq!(
            cfg.bindings.get("Mod+Return").map(String::as_str),
            Some("terminal")
        );
        assert_eq!(cfg.bindings.get("Mod+Tab").map(String::as_str), Some("overview"));

        // Autostart: one bare string + one argv array.
        assert_eq!(cfg.autostart.exec.len(), 2);
        assert_eq!(cfg.autostart.exec[0].argv(), vec!["/bin/true".to_string()]);
        assert_eq!(
            cfg.autostart.exec[1].argv(),
            vec!["/bin/echo".to_string(), "hi".to_string()]
        );
    }
}
