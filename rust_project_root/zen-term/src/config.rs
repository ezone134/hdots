//! zen-term configuration: a single TOML file, read once at startup.
//!
//! Hot reload is signal-driven only (`zen-term --reload` sends SIGUSR1 to the
//! running instance): the config is NEVER watched or polled, so an idle
//! terminal costs 0 CPU and does no file I/O.

use std::path::PathBuf;

use alacritty_terminal::vte::ansi::Rgb;

/// Default ANSI palette (the classic xterm-256color ramp).
pub const DEFAULT_NORMAL: [&str; 8] = [
    "#1e1e2e", "#f38ba8", "#a6e3a1", "#f9e2af", "#89b4fa", "#cba6f7", "#94e2d5", "#bac2de",
];
pub const DEFAULT_BRIGHT: [&str; 8] = [
    "#585b70", "#f38ba8", "#a6e3a1", "#f9e2af", "#89b4fa", "#cba6f7", "#94e2d5", "#cdd6f4",
];
pub const DEFAULT_BG: &str = "#1e1e2e";
pub const DEFAULT_FG: &str = "#cdd6f4";
pub const DEFAULT_CURSOR: &str = "#f5e0dc";

fn default_normal() -> [String; 8] {
    DEFAULT_NORMAL.map(|s| s.to_string())
}
fn default_bright() -> [String; 8] {
    DEFAULT_BRIGHT.map(|s| s.to_string())
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct ColorsCfg {
    pub background: String,
    pub foreground: String,
    pub cursor: String,
    /// Text color drawn inside a block cursor. Empty string = auto (inverted).
    pub cursor_text: String,
    /// Selection highlight background. Empty string = auto (foreground).
    pub selection_bg: String,
    /// Selection text color. Empty string = auto (background).
    pub selection_fg: String,
    pub normal: [String; 8],
    pub bright: [String; 8],
}

impl Default for ColorsCfg {
    fn default() -> Self {
        Self {
            background: DEFAULT_BG.into(),
            foreground: DEFAULT_FG.into(),
            cursor: DEFAULT_CURSOR.into(),
            cursor_text: String::new(),
            selection_bg: String::new(),
            selection_fg: String::new(),
            normal: default_normal(),
            bright: default_bright(),
        }
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Config {
    /// Font family. Empty string = system monospace default.
    pub font_family: String,
    /// Font size in pixels.
    pub font_size: f32,
    /// Padding around the grid, in pixels.
    pub padding: f32,
    /// Background opacity, 0.0..=1.0 (1.0 = fully opaque).
    pub background_opacity: f32,
    /// Scrollback history size in lines.
    pub scrollback_lines: usize,
    /// Shell to run; None = $SHELL (or /bin/sh).
    pub shell: Option<String>,
    /// Arguments passed to the shell.
    pub shell_args: Vec<String>,
    /// Working directory for the shell; None = current directory.
    pub working_directory: Option<String>,
    /// vsync on/off. Off disables the swap interval (maximum throughput).
    pub vsync: bool,
    /// GPU backend: "gl" (default). "vulkan" is reserved for the future ash
    /// backend and currently falls back to GL with a warning.
    pub gpu: String,
    pub colors: ColorsCfg,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            font_family: String::new(),
            font_size: 14.0,
            padding: 4.0,
            background_opacity: 1.0,
            scrollback_lines: 10_000,
            shell: None,
            shell_args: Vec::new(),
            working_directory: None,
            vsync: true,
            gpu: "gl".into(),
            colors: ColorsCfg::default(),
        }
    }
}

/// Parse a `#rrggbb` color string. Invalid input falls back to black.
pub fn parse_rgb(s: &str) -> Rgb {
    let s = s.trim().trim_start_matches('#');
    let v = u32::from_str_radix(s, 16).unwrap_or(0);
    Rgb {
        r: ((v >> 16) & 0xFF) as u8,
        g: ((v >> 8) & 0xFF) as u8,
        b: (v & 0xFF) as u8,
    }
}

/// Default config file path: `$XDG_CONFIG_HOME/zen-term/zen-term.toml`.
pub fn config_path_default() -> PathBuf {
    if let Ok(x) = std::env::var("XDG_CONFIG_HOME") {
        if !x.is_empty() {
            return PathBuf::from(x).join("zen-term").join("zen-term.toml");
        }
    }
    if let Ok(h) = std::env::var("HOME") {
        if !h.is_empty() {
            return PathBuf::from(h).join(".config").join("zen-term").join("zen-term.toml");
        }
    }
    PathBuf::from("zen-term.toml")
}

/// Load config from `path`, merging over the defaults. A missing or invalid
/// file leaves the defaults in place (the file is rewritten on exit/next run).
pub fn load(cfg: &mut Config, path: &PathBuf) {
    let Ok(text) = std::fs::read_to_string(path) else {
        return;
    };
    if let Ok(parsed) = toml::from_str::<Config>(&text) {
        *cfg = parsed;
    }
}

/// Write a full default config file if none exists (creates the directory).
pub fn ensure_default_file(cfg: &Config, path: &PathBuf) {
    if path.exists() {
        return;
    }
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let toml = toml::to_string_pretty(cfg).unwrap_or_default();
    let _ = std::fs::write(path, format!("{toml}\n"));
}

impl Config {
    /// The terminal emulation config handed to `alacritty_terminal::Term`.
    pub fn term_config(&self) -> alacritty_terminal::term::Config {
        alacritty_terminal::term::Config {
            scrolling_history: self.scrollback_lines,
            ..Default::default()
        }
    }

    /// The PTY options for `alacritty_terminal::tty`.
    pub fn tty_options(&self) -> alacritty_terminal::tty::Options {
        use std::collections::HashMap;
        let shell = self.shell.clone().map(|program| {
            alacritty_terminal::tty::Shell::new(program, self.shell_args.clone())
        });
        let working_directory = self
            .working_directory
            .as_ref()
            .map(PathBuf::from)
            .filter(|p| p.is_dir());
        let mut env = HashMap::new();
        env.insert("TERM".to_string(), "xterm-256color".to_string());
        env.insert("COLORTERM".to_string(), "truecolor".to_string());
        alacritty_terminal::tty::Options {
            shell,
            working_directory,
            drain_on_exit: true,
            env,
        }
    }
}
