use std::fs;
use std::path::PathBuf;

/// Parse a `#rrggbb` / `0xrrggbb` color (alpha forced to 0xFF). An 8-digit
/// `#rrggbbaa` form keeps the RGB and drops the alpha byte.
pub fn parse_color(s: &str) -> u32 {
    let s = s.trim().trim_start_matches('#');
    let s = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")).unwrap_or(s);
    let v = u32::from_str_radix(s, 16).unwrap_or(0);
    if s.len() == 8 {
        v >> 8
    } else {
        v & 0xFFFFFF
    }
}

pub fn parse_bool(s: &str) -> bool {
    let s = s.trim().to_ascii_lowercase();
    matches!(s.as_str(), "true" | "yes" | "on" | "1")
}

pub fn trim(s: &str) -> &str {
    s.trim_matches(|c| c == ' ' || c == '\t' || c == '\r' || c == '\n')
}

#[derive(Clone)]
pub struct Config {
    /* theme */
    pub bg: u32,
    pub fg: u32,
    pub sel_bg: u32,
    pub sel_fg: u32,
    pub dir: u32,
    pub dim: u32,
    pub thumb_c: u32,
    pub status_c: u32,
    pub tab_active: u32,
    pub tab_idle: u32,
    pub input_bg: u32,
    pub type_dir: u32,
    pub type_file: u32,
    pub type_img: u32,
    pub type_arc: u32,
    /* icon colors (default = type colors; overridable per theme) */
    pub icon_dir: u32,
    pub icon_file: u32,
    pub icon_img: u32,
    pub icon_arc: u32,
    pub icon_link: u32,
    pub icon_hardlink: u32,
    /* theme selection */
    pub theme: String,
    /* ui */
    /// Font family name ("" = auto/system default).
    pub font_name: String,
    pub font_size: i32,
    pub padding: i32,
    pub grid_cell: i32,
    pub icon_size: i32,
    pub view: crate::tab::ViewMode,
    pub show_hidden: bool,
    pub dirs_first: bool,
    pub thumb_size: i32,
    pub sidebar: bool,
    pub sidebar_width: i32,
    pub show_preview: bool,
    pub preview_width: i32,
    pub confirm_close: bool,
    /// Ordered toolbar items (see `toolbar::default_toolbar`).
    pub toolbar: Vec<crate::toolbar::ToolbarItem>,
    /// Keys of toolbar items currently unchecked (hidden) in the dialog.
    pub toolbar_hidden: Vec<String>,
    /// Show file extensions in the file pane (Thunar-style toggle).
    pub show_ext: bool,
    /* behaviour */
    pub opener: String,
    pub extract: String,
    pub terminal: String,
    pub enable_copy_basename: bool,
    pub enable_copy_fullpath: bool,
    pub enable_copy_parent: bool,
    pub enable_copy_stem: bool,
    pub enable_templates: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            bg: 0x1e1e2e,
            fg: 0xcdd6f4,
            sel_bg: 0x89b4fa,
            sel_fg: 0x11111b,
            dir: 0x89dceb,
            dim: 0x181825,
            thumb_c: 0x45475a,
            status_c: 0xa6adc8,
            tab_active: 0x45475a,
            tab_idle: 0x313244,
            input_bg: 0x11111b,
            type_dir: 0x89b4fa,
            type_file: 0x6c7086,
            type_img: 0xa6e3a1,
            type_arc: 0xfab387,
            icon_dir: 0x89b4fa,
            icon_file: 0x6c7086,
            icon_img: 0xa6e3a1,
            icon_arc: 0xfab387,
            icon_link: 0xffffff,
            icon_hardlink: 0xffffff,
            theme: "dark".to_string(),
            font_name: String::new(),
            font_size: 15,
            padding: 8,
            grid_cell: 76,
            icon_size: 0,
            view: crate::tab::ViewMode::List,
            show_hidden: false,
            dirs_first: true,
            thumb_size: 48,
            sidebar: true,
            sidebar_width: 160,
            show_preview: true,
            preview_width: 220,
            confirm_close: true,
            toolbar: crate::toolbar::default_toolbar(),
            toolbar_hidden: Vec::new(),
            show_ext: true,
            opener: "xdg-open".to_string(),
            extract: String::new(),
            terminal: String::new(),
            enable_copy_basename: true,
            enable_copy_fullpath: true,
            enable_copy_parent: true,
            enable_copy_stem: true,
            enable_templates: true,
        }
    }
}

pub fn config_path_default() -> PathBuf {
    let xdg = std::env::var_os("XDG_CONFIG_HOME");
    let home = std::env::var_os("HOME");
    if let Some(x) = xdg {
        if !x.is_empty() {
            return PathBuf::from(x).join("wfm/config");
        }
    }
    if let Some(h) = home {
        return PathBuf::from(h).join(".config/wfm/config");
    }
    PathBuf::from("/tmp/wfm.config")
}

/// Session state (view mode, sort, panels, last dir) — remembered across
/// restarts, separate from the hand-edited config file.
pub fn state_path() -> PathBuf {
    let xdg = std::env::var_os("XDG_CONFIG_HOME");
    let home = std::env::var_os("HOME");
    if let Some(x) = xdg {
        if !x.is_empty() {
            return PathBuf::from(x).join("wfm/state");
        }
    }
    if let Some(h) = home {
        return PathBuf::from(h).join(".config/wfm/state");
    }
    PathBuf::from("/tmp/wfm.state")
}

/// Where user theme files live (`*.theme`).
pub fn themes_dir() -> PathBuf {
    let xdg = std::env::var_os("XDG_CONFIG_HOME");
    let home = std::env::var_os("HOME");
    if let Some(x) = xdg {
        if !x.is_empty() {
            return PathBuf::from(x).join("wfm/themes");
        }
    }
    if let Some(h) = home {
        return PathBuf::from(h).join(".config/wfm/themes");
    }
    PathBuf::from("/tmp/wfm.themes")
}

#[derive(Clone, Default)]
pub struct State {
    pub view: Option<crate::tab::ViewMode>,
    pub sort_key: Option<crate::tab::SortKey>,
    pub sort_desc: bool,
    pub sidebar: bool,
    pub preview: bool,
    /// Whether dual-pane split mode was active when the window closed.
    pub split: bool,
    /// Dragged pane widths, remembered across restarts.
    pub sidebar_width: Option<i32>,
    pub preview_width: Option<i32>,
    /// Width of the left split pane (0 = default half).
    pub split_width: Option<i32>,
    /// Default place shortcuts the user hid from the sidebar
    /// (keys: home, documents, downloads, music, videos, trash).
    pub hidden_places: Vec<String>,
    pub last_dir: Option<PathBuf>,
}

impl State {
    pub fn load(path: &PathBuf) -> State {
        let mut s = State::default();
        let Ok(text) = fs::read_to_string(path) else {
            return s;
        };
        for line in text.lines() {
            let p = line.trim_start_matches([' ', '\t']);
            if p.is_empty() || p.starts_with('#') {
                continue;
            }
            let Some(eq) = p.find('=') else { continue };
            let k = trim(&p[..eq]).to_ascii_lowercase();
            let v = trim(&p[eq + 1..]);
            match k.as_str() {
                "view" => s.view = match v {
                    "grid" => Some(crate::tab::ViewMode::Grid),
                    "compact" => Some(crate::tab::ViewMode::Compact),
                    _ => Some(crate::tab::ViewMode::List),
                },
                "sort" => s.sort_key = match v {
                    "size" => Some(crate::tab::SortKey::Size),
                    "mtime" => Some(crate::tab::SortKey::Mtime),
                    "type" => Some(crate::tab::SortKey::Type),
                    _ => Some(crate::tab::SortKey::Name),
                },
                "sort_desc" => s.sort_desc = parse_bool(v),
                "sidebar" => s.sidebar = parse_bool(v),
                "preview" => s.preview = parse_bool(v),
                "split" => s.split = parse_bool(v),
                "sidebar_width" => s.sidebar_width = v.trim().parse::<i32>().ok(),
                "preview_width" => s.preview_width = v.trim().parse::<i32>().ok(),
                "split_width" => s.split_width = v.trim().parse::<i32>().ok(),
                "hidden_places" => {
                    s.hidden_places = v
                        .split(',')
                        .map(|p| trim(p).to_string())
                        .filter(|p| !p.is_empty())
                        .collect();
                }
                "last_dir" => s.last_dir = Some(PathBuf::from(v)),
                _ => {}
            }
        }
        s
    }

    pub fn save(&self, path: &PathBuf) {
        let mut out = String::new();
        if let Some(v) = self.view {
            out.push_str(&format!(
                "view = {}\n",
                match v {
                    crate::tab::ViewMode::Grid => "grid",
                    crate::tab::ViewMode::Compact => "compact",
                    _ => "list",
                }
            ));
        }
        if let Some(k) = self.sort_key {
            out.push_str(&format!(
                "sort = {}\n",
                match k {
                    crate::tab::SortKey::Size => "size",
                    crate::tab::SortKey::Mtime => "mtime",
                    crate::tab::SortKey::Type => "type",
                    crate::tab::SortKey::Name => "name",
                }
            ));
        }
        out.push_str(&format!("sort_desc = {}\n", self.sort_desc));
        out.push_str(&format!("sidebar = {}\n", self.sidebar));
        out.push_str(&format!("preview = {}\n", self.preview));
        out.push_str(&format!("split = {}\n", self.split));
        if let Some(w) = self.sidebar_width {
            out.push_str(&format!("sidebar_width = {w}\n"));
        }
        if let Some(w) = self.preview_width {
            out.push_str(&format!("preview_width = {w}\n"));
        }
        if let Some(w) = self.split_width {
            out.push_str(&format!("split_width = {w}\n"));
        }
        if !self.hidden_places.is_empty() {
            out.push_str(&format!("hidden_places = {}\n", self.hidden_places.join(",")));
        }
        if let Some(d) = &self.last_dir {
            out.push_str(&format!("last_dir = {}\n", d.display()));
        }
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(path, out);
    }
}

/// Persist a `theme =` setting into the config file (creating/updating the
/// key while preserving the rest of the file).
pub fn set_theme(path: &PathBuf, spec: &str) {
    set_keys(path, &[("theme", spec.to_string())]);
}

/// Rewrite several `key = value` settings in the config file at once, creating
/// missing keys and preserving every other line (comments/format intact).
pub fn set_keys(path: &PathBuf, keys: &[(&str, String)]) {
    let text = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(_) => String::new(),
    };
    let want: Vec<(String, &str)> = keys
        .iter()
        .map(|(k, v)| (k.to_ascii_lowercase(), v.as_str()))
        .collect();
    let mut replaced = vec![false; want.len()];
    let mut out = String::new();
    for line in text.lines() {
        let p = trim(line.trim_start_matches([' ', '\t']));
        if !p.starts_with('#') {
            let k = trim(p.split('=').next().unwrap_or("")).to_ascii_lowercase();
            if let Some(i) = want.iter().position(|(wk, _)| *wk == k) {
                let indent = &line[..line.len() - line.trim_start_matches([' ', '\t']).len()];
                out.push_str(&format!("{indent}{} = {}\n", want[i].0, want[i].1));
                replaced[i] = true;
                continue;
            }
        }
        out.push_str(line);
        out.push('\n');
    }
    for i in 0..want.len() {
        if !replaced[i] {
            out.push_str(&format!("{} = {}\n", want[i].0, want[i].1));
        }
    }
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(path, out);
}

/// Add `key = value` lines to the config file for keys that are *missing*,
/// preserving every existing key and the rest of the file. Used to surface
/// optional keys like `font_name` / `font_size` on first run without
/// clobbering values the user already set.
pub fn ensure_keys(path: &PathBuf, keys: &[(&str, String)]) {
    let text = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(_) => String::new(),
    };
    let present: Vec<String> = text
        .lines()
        .map(|l| {
            let p = trim(l.trim_start_matches([' ', '\t']));
            if p.starts_with('#') {
                String::new()
            } else {
                trim(p.split('=').next().unwrap_or("")).to_ascii_lowercase()
            }
        })
        .filter(|k| !k.is_empty())
        .collect();
    let mut out = text;
    if !out.is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }
    let mut added = false;
    for (k, v) in keys {
        let lk = k.to_ascii_lowercase();
        if !present.iter().any(|p| *p == lk) {
            out.push_str(&format!("{k} = {v}\n"));
            added = true;
        }
    }
    if added {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(path, out);
    }
}

/// Load a single-file INI config over `cfg`. Missing file = no-op.
pub fn load(cfg: &mut Config, path: &PathBuf) {
    let text = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(_) => return,
    };
    for line in text.lines() {
        let p = line.trim_start_matches([' ', '\t']);
        if p.is_empty() || p.starts_with('#') || p.starts_with('[') {
            continue;
        }
        // strip inline comments (`#` preceded by whitespace and followed by
        // whitespace/EOL, so color values like `#123456` are left intact)
        let mut p = p;
        for (i, b) in p.bytes().enumerate() {
            if b == b'#' && (i == 0 || p.as_bytes()[i - 1].is_ascii_whitespace())
                && p[i + 1..].trim_start().is_empty()
            {
                p = &p[..i];
                break;
            }
        }
        let Some(eq) = p.find('=') else { continue };
        let k = trim(&p[..eq]).to_ascii_lowercase();
        let v = trim(&p[eq + 1..]);
        if k.is_empty() || v.is_empty() {
            continue;
        }
        apply(cfg, &k, v);
    }
}

fn apply(c: &mut Config, k: &str, v: &str) {
    macro_rules! int_range {
        ($field:ident, $min:expr, $max:expr) => {
            if let Ok(n) = v.trim().parse::<i32>() {
                if ($min..=$max).contains(&n) {
                    c.$field = n;
                }
            }
        };
    }
    macro_rules! str_field {
        ($field:ident) => {{
            c.$field = v.to_string();
        }};
    }
    match k {
        "bg" => c.bg = parse_color(v),
        "fg" => c.fg = parse_color(v),
        "sel_bg" => c.sel_bg = parse_color(v),
        "sel_fg" => c.sel_fg = parse_color(v),
        "dir" => c.dir = parse_color(v),
        "dim" => c.dim = parse_color(v),
        "thumb" => c.thumb_c = parse_color(v),
        "status" => c.status_c = parse_color(v),
        "tab_active" => c.tab_active = parse_color(v),
        "tab_idle" => c.tab_idle = parse_color(v),
        "input_bg" => c.input_bg = parse_color(v),
        "type_dir" => c.type_dir = parse_color(v),
        "type_file" => c.type_file = parse_color(v),
        "type_img" => c.type_img = parse_color(v),
        "type_arc" => c.type_arc = parse_color(v),
        "icon_dir" => c.icon_dir = parse_color(v),
        "icon_file" => c.icon_file = parse_color(v),
        "icon_img" => c.icon_img = parse_color(v),
        "icon_arc" => c.icon_arc = parse_color(v),
        "icon_link" => c.icon_link = parse_color(v),
        "icon_hardlink" | "icon_hard" => c.icon_hardlink = parse_color(v),
        "theme" => c.theme = v.to_string(),
        "view" => {
            c.view = match v.trim().to_ascii_lowercase().as_str() {
                "grid" | "icon" => crate::tab::ViewMode::Grid,
                "compact" | "tiles" => crate::tab::ViewMode::Compact,
                _ => crate::tab::ViewMode::List,
            }
        }
        "font_name" => c.font_name = v.to_string(),
        "font_size" => int_range!(font_size, 8, 96),
        "padding" => int_range!(padding, 0, 64),
        "grid_cell" => int_range!(grid_cell, 48, 200),
        "icon_size" => int_range!(icon_size, 0, 64),
        "thumb_size" => int_range!(thumb_size, 16, 128),
        "sidebar_width" => int_range!(sidebar_width, 80, 600),
        "preview_width" => int_range!(preview_width, 100, 800),
        "show_hidden" => c.show_hidden = parse_bool(v),
        "dirs_first" => c.dirs_first = parse_bool(v),
        "sidebar" => c.sidebar = parse_bool(v),
        "show_preview" | "preview" => c.show_preview = parse_bool(v),
        "confirm_close" => c.confirm_close = parse_bool(v),
        "toolbar" => {
            let items = crate::toolbar::parse_toolbar(v);
            if !items.is_empty() {
                c.toolbar = items;
            }
        }
        "toolbar_hidden" => {
            c.toolbar_hidden = v
                .split(';')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect();
        }
        "show_ext" | "show_extensions" => c.show_ext = parse_bool(v),
        "opener" => str_field!(opener),
        "extract" => str_field!(extract),
        "terminal" => str_field!(terminal),
        "enable_copy_basename" => c.enable_copy_basename = parse_bool(v),
        "enable_copy_fullpath" => c.enable_copy_fullpath = parse_bool(v),
        "enable_copy_parent" => c.enable_copy_parent = parse_bool(v),
        "enable_copy_stem" => c.enable_copy_stem = parse_bool(v),
        "enable_templates" => c.enable_templates = parse_bool(v),
        _ => {}
    }
}

/// Format a size like the C `wfm_fmt_size` ("1.2 K", "3.4 M", "5.6 G").
pub fn fmt_size(sz: i64) -> String {
    if sz < 1024 {
        format!("{sz} B")
    } else if sz < 1048576 {
        format!("{:.1} K", sz as f64 / 1024.0)
    } else if sz < 1073741824 {
        format!("{:.1} M", sz as f64 / 1048576.0)
    } else {
        format!("{:.1} G", sz as f64 / 1073741824.0)
    }
}

pub fn has_suffix_ci(name: &str, suffix: &str) -> bool {
    let nl = name.len();
    let sl = suffix.len();
    if sl > nl {
        return false;
    }
    name[nl - sl..].eq_ignore_ascii_case(suffix)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colors() {
        assert_eq!(parse_color("#ff8040"), 0xff8040);
        assert_eq!(parse_color("0x123456"), 0x123456);
        assert_eq!(parse_color("aabbcc"), 0xaabbcc);
        assert_eq!(parse_color("#ff8040ff"), 0xff8040);
    }

    #[test]
    fn bools() {
        assert!(parse_bool("true"));
        assert!(parse_bool("1"));
        assert!(parse_bool("on"));
        assert!(!parse_bool("false"));
        assert!(!parse_bool("0"));
    }

    #[test]
    fn size_fmt() {
        assert_eq!(fmt_size(512), "512 B");
        assert_eq!(fmt_size(1536), "1.5 K");
        assert_eq!(fmt_size(1048576 * 2), "2.0 M");
        assert_eq!(fmt_size(1073741824 * 4), "4.0 G");
    }

    #[test]
    fn ini_load() {
        let path = std::env::temp_dir().join("wfm_test.conf");
        std::fs::write(&path, "# t\nfont_size = 22\nbg = #123456\ndirs_first = false\n").unwrap();
        let mut c = Config::default();
        load(&mut c, &path);
        std::fs::remove_file(&path).unwrap();
        assert_eq!(c.font_size, 22);
        assert_eq!(c.bg, 0x123456);
        assert!(!c.dirs_first);
    }

    #[test]
    fn font_name_parses() {
        let path = std::env::temp_dir().join("wfm_test_font.conf");
        std::fs::write(&path, "font_name = JetBrains Mono\nfont_size = 18\n").unwrap();
        let mut c = Config::default();
        load(&mut c, &path);
        std::fs::remove_file(&path).unwrap();
        assert_eq!(c.font_name, "JetBrains Mono");
        assert_eq!(c.font_size, 18);
    }

    #[test]
    fn ensure_keys_adds_only_missing() {
        let path = std::env::temp_dir().join("wfm_test_ensure.conf");
        std::fs::write(&path, "theme = dark\nfont_size = 18\n").unwrap();
        ensure_keys(&path, &[
            ("font_name", String::new()),
            ("font_size", "15".to_string()),
            ("grid_cell", "200".to_string()),
        ]);
        let text = std::fs::read_to_string(&path).unwrap();
        std::fs::remove_file(&path).unwrap();
        // existing keys keep their value, only missing ones are appended
        assert!(text.contains("theme = dark"));
        assert!(text.contains("font_size = 18"));
        assert!(text.contains("font_name = "));
        assert!(text.contains("grid_cell = 200"));
        // no duplicate font_size
        assert_eq!(text.matches("font_size").count(), 1);
    }

    #[test]
    fn state_roundtrip() {
        let path = std::env::temp_dir().join("wfm_test.state");
        let s = State {
            view: Some(crate::tab::ViewMode::Grid),
            sort_key: Some(crate::tab::SortKey::Size),
            sort_desc: true,
            sidebar: false,
            preview: true,
            split: true,
            sidebar_width: Some(200),
            preview_width: Some(260),
            split_width: Some(420),
            hidden_places: vec!["documents".into(), "music".into()],
            last_dir: Some(std::path::PathBuf::from("/home/tw")),
        };
        s.save(&path);
        let l = State::load(&path);
        std::fs::remove_file(&path).unwrap();
        assert_eq!(l.view, Some(crate::tab::ViewMode::Grid));
        assert_eq!(l.sort_key, Some(crate::tab::SortKey::Size));
        assert!(l.sort_desc);
        assert!(!l.sidebar);
        assert!(l.preview);
        assert!(l.split);
        assert_eq!(l.sidebar_width, Some(200));
        assert_eq!(l.preview_width, Some(260));
        assert_eq!(l.split_width, Some(420));
        assert_eq!(l.hidden_places, vec!["documents".to_string(), "music".to_string()]);
        assert_eq!(l.last_dir, Some(std::path::PathBuf::from("/home/tw")));
    }

    #[test]
    fn set_theme_preserves_rest() {
        let path = std::env::temp_dir().join("wfm_test.theme_set");
        std::fs::write(&path, "# keep\nfont_size = 18\nbg = #112233\n").unwrap();
        set_theme(&path, "light");
        let text = std::fs::read_to_string(&path).unwrap();
        std::fs::remove_file(&path).unwrap();
        assert!(text.contains("theme = light"));
        assert!(text.contains("font_size = 18"));
        assert!(text.contains("bg = #112233"));
        // no duplicate theme lines
        assert_eq!(text.lines().filter(|l| l.starts_with("theme =")).count(), 1);
    }

    #[test]
    fn set_keys_updates_and_appends() {
        let path = std::env::temp_dir().join("wfm_test.set_keys");
        std::fs::write(&path, "# c\ngrid_cell = 76\n").unwrap();
        set_keys(&path, &[("grid_cell", "100".into()), ("icon_size", "24".into())]);
        let text = std::fs::read_to_string(&path).unwrap();
        std::fs::remove_file(&path).unwrap();
        assert!(text.contains("grid_cell = 100"));
        assert!(text.contains("icon_size = 24"));
        assert_eq!(text.lines().filter(|l| l.starts_with("grid_cell")).count(), 1);
    }

    #[test]
    fn confirm_close_defaults_on() {
        let c = Config::default();
        assert!(c.confirm_close);
    }
}
