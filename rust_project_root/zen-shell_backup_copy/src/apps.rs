//! Installed-app index: parses `*.desktop` entries, fuzzy-search, launch.

use std::path::Path;

#[derive(Clone, Debug)]
pub struct App {
    pub id: String,
    pub name: String,
    pub exec: String,
    pub icon: String,
    pub category: String,
}

impl App {
    /// `ImageStore` key for this app's icon: absolute path → `file:<path>`,
    /// themed name → `icon:<name>`. `None` when the entry has no icon.
    pub fn icon_key(&self) -> Option<String> {
        if self.icon.is_empty() {
            return None;
        }
        let path = Path::new(&self.icon);
        if path.is_absolute() {
            Some(format!("file:{}", path.display()))
        } else {
            Some(format!("icon:{}", self.icon))
        }
    }
}

pub struct AppManager {
    pub apps: Vec<App>,
}

impl AppManager {
    /// Empty manager — used for instant startup; call `rescan` or use
    /// `rescan_async` to populate in the background.
    pub fn new() -> Self {
        AppManager { apps: Vec::new() }
    }

    /// Scan .desktop files on a background thread; sends the result back
    /// through a calloop-compatible `std::sync::mpsc` channel. The caller
    /// should `.try_recv()` in the event loop tick.
    pub fn rescan_async(&self) -> std::sync::mpsc::Receiver<Vec<App>> {
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let mut apps = Vec::new();
            let mut dirs: Vec<String> = vec![
                "/usr/share/applications".into(),
                "/usr/local/share/applications".into(),
            ];
            if let Ok(home) = std::env::var("HOME") {
                dirs.push(format!("{home}/.local/share/applications"));
            }
            for dir in dirs {
                Self::scan_dir_into(&mut apps, &dir);
            }
            apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
            if std::env::var("ZEN_TRACE").is_ok() {
                eprintln!("zen: {} apps indexed (async)", apps.len());
            }
            let _ = tx.send(apps);
        });
        rx
    }

    fn scan_dir_into(out: &mut Vec<App>, dir: &str) {
        let Ok(rd) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in rd.flatten() {
            let path = entry.path();
            if path
                .extension()
                .and_then(|e| e.to_str())
                != Some("desktop")
            {
                continue;
            }
            if let Some(app) = parse_desktop(&path) {
                out.push(app);
            }
        }
    }

    /// Subsequence fuzzy match (query already lowercased; name compared case-insensitively).
    fn score(name: &str, q_lower: &str) -> Option<u32> {
        if q_lower.is_empty() {
            return Some(0);
        }
        let mut qi = q_lower.chars();
        let mut need = qi.next()?;
        let mut first = 0usize;
        let mut matched = 0u32;
        for (i, c) in name.chars().enumerate() {
            let c = c.to_ascii_lowercase();
            if c == need {
                if matched == 0 {
                    first = i;
                }
                matched += 1;
                match qi.next() {
                    Some(n) => need = n,
                    None => {
                        // prefer early, tight matches
                        return Some(
                            (first as u32).saturating_add(i as u32 - first as u32),
                        );
                    }
                }
            }
        }
        None
    }

    /// Indices into `self.apps` for the top `max` matches (stable for caching).
    pub fn search_indices(&self, query: &str, max: usize) -> Vec<usize> {
        let q = query.to_lowercase();
        let mut scored: Vec<(u32, usize)> = self
            .apps
            .iter()
            .enumerate()
            .filter_map(|(i, a)| AppManager::score(&a.name, &q).map(|s| (s, i)))
            .collect();
        scored.sort_by_key(|(s, _)| *s);
        scored.truncate(max);
        scored.into_iter().map(|(_, i)| i).collect()
    }

    pub fn launch(&self, id: &str) {
        let Some(app) = self.apps.iter().find(|a| a.id == id) else {
            return;
        };
        if app.exec.is_empty() {
            return;
        }
        // strip field codes (%f %u etc.) and spawn detached
        let exec: String = app
            .exec
            .split_whitespace()
            .filter(|t| !t.starts_with('%'))
            .collect::<Vec<_>>()
            .join(" ");
        if exec.is_empty() {
            return;
        }
        let _ = std::process::Command::new("/bin/sh")
            .arg("-c")
            .arg(&exec)
            .env("GIO_LAUNCHED_DESKTOP_FILE", "")
            .spawn();
    }
}

fn parse_desktop(path: &Path) -> Option<App> {
    let content = std::fs::read_to_string(path).ok()?;
    let mut in_entry = false;
    let mut name = String::new();
    let mut exec = String::new();
    let mut icon = String::new();
    let mut category = String::new();
    let mut no_display = false;
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_entry = line == "[Desktop Entry]";
            continue;
        }
        if !in_entry || line.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            match k.trim() {
                "Name" if name.is_empty() => name = v.trim().into(),
                "Exec" => exec = v.trim().into(),
                "Icon" => icon = v.trim().into(),
                "Categories" => category = v.split(';').next().unwrap_or("").into(),
                "NoDisplay" => no_display = v.trim() == "true",
                "Hidden" => no_display |= v.trim() == "true",
                _ => {}
            }
        }
    }
    if name.is_empty() || exec.is_empty() || no_display {
        return None;
    }
    let id = path.file_stem()?.to_string_lossy().into_owned();
    Some(App {
        id,
        name,
        exec,
        icon,
        category,
    })
}
