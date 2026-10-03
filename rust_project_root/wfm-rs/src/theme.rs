use crate::config::{parse_color, trim};

/// Theme palette + icon colors. `resolve()` builds one from a spec that is
/// either a builtin name (`dark`/`light`) or a path to an external theme file.
#[derive(Clone)]
pub struct Theme {
    pub name: String,
    pub base: Base,
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
    /// colors the procedural icons are drawn in (default = type colors)
    pub icon_dir: u32,
    pub icon_file: u32,
    pub icon_img: u32,
    pub icon_arc: u32,
    pub icon_link: u32,
    pub icon_hardlink: u32,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Base {
    Dark,
    Light,
}

impl Theme {
    fn new(name: &str, base: Base) -> Self {
        let mut t = if base == Base::Light {
            Theme::light_palette()
        } else {
            Theme::dark_palette()
        };
        t.name = name.to_string();
        t.base = base;
        t
    }

    fn dark_palette() -> Self {
        Theme {
            name: String::new(),
            base: Base::Dark,
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
        }
    }

    fn light_palette() -> Self {
        Theme {
            name: String::new(),
            base: Base::Light,
            bg: 0xeff1f5,
            fg: 0x4c4f69,
            sel_bg: 0x7287fd,
            sel_fg: 0xffffff,
            dir: 0x209fb5,
            dim: 0xe6e9ef,
            thumb_c: 0xbcc0cc,
            status_c: 0x8c8fa1,
            tab_active: 0xbcc0cc,
            tab_idle: 0xccd0da,
            input_bg: 0xe6e9ef,
            type_dir: 0x1e66f5,
            type_file: 0x7c7f93,
            type_img: 0x40a02b,
            type_arc: 0xfe640b,
            icon_dir: 0x1e66f5,
            icon_file: 0x7c7f93,
            icon_img: 0x40a02b,
            icon_arc: 0xfe640b,
            icon_link: 0x4c4f69,
            icon_hardlink: 0x4c4f69,
        }
    }

    /// Write every palette + icon color into `cfg`, so render keeps reading
    /// `app.cfg.*` unchanged.
    pub fn apply_to_cfg(&self, c: &mut crate::config::Config) {
        c.bg = self.bg;
        c.fg = self.fg;
        c.sel_bg = self.sel_bg;
        c.sel_fg = self.sel_fg;
        c.dir = self.dir;
        c.dim = self.dim;
        c.thumb_c = self.thumb_c;
        c.status_c = self.status_c;
        c.tab_active = self.tab_active;
        c.tab_idle = self.tab_idle;
        c.input_bg = self.input_bg;
        c.type_dir = self.type_dir;
        c.type_file = self.type_file;
        c.type_img = self.type_img;
        c.type_arc = self.type_arc;
        c.icon_dir = self.icon_dir;
        c.icon_file = self.icon_file;
        c.icon_img = self.icon_img;
        c.icon_arc = self.icon_arc;
        c.icon_link = self.icon_link;
        c.icon_hardlink = self.icon_hardlink;
    }

    /// INI-style theme file keys; `base = dark|light` picks the layer preset.
    pub fn apply_key(&mut self, k: &str, v: &str) {
        match k {
            "base" => match v.to_ascii_lowercase().as_str() {
                "light" => *self = Theme::new("", Base::Light),
                _ => *self = Theme::new("", Base::Dark),
            },
            "bg" => self.bg = parse_color(v),
            "fg" => self.fg = parse_color(v),
            "sel_bg" => self.sel_bg = parse_color(v),
            "sel_fg" => self.sel_fg = parse_color(v),
            "dir" => self.dir = parse_color(v),
            "dim" => self.dim = parse_color(v),
            "thumb" | "thumb_c" => self.thumb_c = parse_color(v),
            "status" | "status_c" => self.status_c = parse_color(v),
            "tab_active" => self.tab_active = parse_color(v),
            "tab_idle" => self.tab_idle = parse_color(v),
            "input_bg" => self.input_bg = parse_color(v),
            "type_dir" => {
                self.type_dir = parse_color(v);
                if self.icon_dir == self.type_dir {
                    self.icon_dir = self.type_dir;
                }
            }
            "type_file" => {
                self.type_file = parse_color(v);
                self.icon_file = self.type_file;
            }
            "type_img" => {
                self.type_img = parse_color(v);
                self.icon_img = self.type_img;
            }
            "type_arc" => {
                self.type_arc = parse_color(v);
                self.icon_arc = self.type_arc;
            }
            "icon_dir" => self.icon_dir = parse_color(v),
            "icon_file" => self.icon_file = parse_color(v),
            "icon_img" => self.icon_img = parse_color(v),
            "icon_arc" => self.icon_arc = parse_color(v),
            "icon_link" => self.icon_link = parse_color(v),
            "icon_hardlink" | "icon_hard" => self.icon_hardlink = parse_color(v),
            _ => {}
        }
    }
}

fn default_spec(spec: &str) -> bool {
    match spec.to_ascii_lowercase().as_str() {
        "" | "dark" => true,
        _ => false,
    }
}

/// Resolve a theme spec: `dark`/`light` → builtin preset, anything else is
/// treated as a path to an external theme file layered over the dark (or a
/// `base = light` file's light) palette. Unreadable → dark preset.
pub fn resolve(spec: &str) -> Theme {
    let lower = spec.to_ascii_lowercase();
    match lower.as_str() {
        "" | "dark" => return Theme::new("dark", Base::Dark),
        "light" => return Theme::new("light", Base::Light),
        _ => {}
    }
    let mut t = Theme::new(&lower, Base::Dark);
    match std::fs::read_to_string(spec) {
        Ok(text) => {
            for line in text.lines() {
                let p = trim(line.trim_start_matches([' ', '\t']));
                if p.is_empty() || p.starts_with('#') || p.starts_with('[') {
                    continue;
                }
                let Some(eq) = p.find('=') else { continue };
                let k = trim(&p[..eq]).to_ascii_lowercase();
                let v = trim(&p[eq + 1..]);
                if k.is_empty() || v.is_empty() {
                    continue;
                }
                t.apply_key(&k, v);
            }
            t.name = spec.to_string();
            t
        }
        Err(_) => {
            if default_spec(spec) {
                Theme::new("dark", Base::Dark)
            } else {
                eprintln!("wfm: cannot read theme file `{spec}`, falling back to dark");
                Theme::new("dark", Base::Dark)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_dark() {
        let t = resolve("dark");
        assert_eq!(t.base, Base::Dark);
        assert_eq!(t.bg, 0x1e1e2e);
        assert!(t.fg > t.bg);
    }

    #[test]
    fn builtin_light() {
        let t = resolve("light");
        assert_eq!(t.base, Base::Light);
        assert_eq!(t.bg, 0xeff1f5);
        assert!(t.bg > t.fg);
    }

    #[test]
    fn empty_is_dark() {
        assert_eq!(resolve("").bg, resolve("dark").bg);
    }

    #[test]
    fn unknown_falls_back_to_dark() {
        let t = resolve("/nonexistent/theme.file");
        assert_eq!(t.base, Base::Dark);
    }

    #[test]
    fn external_file_layers_on_base() {
        let path = std::env::temp_dir().join("wfm_test.theme");
        std::fs::write(&path, "# my theme\nbase = light\nbg = #ff0000\nicon_link = #111111\n").unwrap();
        let t = resolve(path.to_str().unwrap());
        std::fs::remove_file(&path).unwrap();
        assert_eq!(t.base, Base::Light);
        assert_eq!(t.bg, 0xff0000);
        assert_eq!(t.fg, 0x4c4f69); // untouched light palette
        assert_eq!(t.icon_link, 0x111111);
        // type key that differs from default also moves its icon color
        assert_eq!(t.type_dir, 0x1e66f5);
        assert_eq!(t.icon_dir, 0x1e66f5);
    }
}
