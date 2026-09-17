//! Custom context-menu actions in wfm's own format.
//!
//! Actions live in `~/.config/wfm/custom-actions` (a plain file with **no
//! extension**, one `[action]` block per action — see `serialize_actions`
//! for the exact layout). Commands expand Thunar-style codes
//! (`%f %F %n %N %d %D %p`, `%%` escapes a literal `%`) and are run from the
//! first selected file's directory.
//!
//! The legacy `~/.config/Thunar/uca.xml` is *not* read directly anymore;
//! there is a temporary "Import Thunar Custom Actions…" menu option that
//! converts it into wfm's own file (with a warning when current actions
//! would be replaced). That option will be removed in a future release.

use std::path::{Path, PathBuf};

use crate::entry::Entry;

#[derive(Clone)]
pub struct CustomAction {
    pub name: String,
    pub command: String,
    /// Space-separated glob patterns (e.g. `*.png *.jpg`); empty = everything.
    pub patterns: Vec<String>,
    pub directories: bool,
    pub audio_files: bool,
    pub image_files: bool,
    pub other_files: bool,
    pub text_files: bool,
    pub video_files: bool,
}

/// Rough file category used to match the category flags.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FileCat {
    Directory,
    Audio,
    Image,
    Other,
    Text,
    Video,
}

/// `~/.config/wfm/custom-actions` — wfm's own action file (no extension).
pub fn actions_path() -> PathBuf {
    dirs::config_dir()
        .map(|p| p.join("wfm/custom-actions"))
        .unwrap_or_else(|| PathBuf::from("/tmp/wfm-custom-actions"))
}

/// `~/.config/Thunar/uca.xml` — read only by the temporary import option.
pub fn thunar_path() -> PathBuf {
    dirs::config_dir()
        .map(|p| p.join("Thunar/uca.xml"))
        .unwrap_or_else(|| PathBuf::from("/tmp/uca.xml"))
}

/// Load every custom action from wfm's own file. Returns an empty list when
/// the file is missing or unreadable.
pub fn load_custom_actions() -> Vec<CustomAction> {
    let Ok(text) = std::fs::read_to_string(actions_path()) else {
        return Vec::new();
    };
    parse_actions(&text)
}

/// Overwrite wfm's own action file with `actions` (used by the Thunar
/// import and by `ensure_default_actions`).
pub fn write_actions(actions: &[CustomAction]) -> std::io::Result<()> {
    let path = actions_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::write(&path, serialize_actions(actions))
}

// ------------------------------------------------------------------
// wfm's own file format (INI-style blocks, no XML)
// ------------------------------------------------------------------
//
//   # comment lines start with '#'
//   [action]
//   name = Open Terminal Here
//   command = exo-open --working-directory %f --launch TerminalEmulator
//   patterns = *                 (optional; space-separated globs)
//   categories = directories     (optional; space-separated subset of
//                                  directories audio image other text video;
//                                  missing = applies to everything)
//
// Unknown keys are ignored; an action without a name or command is dropped.

/// Parse wfm's own action file text into actions.
pub fn parse_actions(text: &str) -> Vec<CustomAction> {
    let mut out = Vec::new();
    let mut cur: Option<CustomAction> = None;
    let flush = |cur: &mut Option<CustomAction>, out: &mut Vec<CustomAction>| {
        if let Some(a) = cur.take() {
            if !a.name.is_empty() && !a.command.is_empty() {
                out.push(a);
            }
        }
    };
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line == "[action]" {
            flush(&mut cur, &mut out);
            cur = Some(CustomAction {
                name: String::new(),
                command: String::new(),
                patterns: Vec::new(),
                directories: false,
                audio_files: false,
                image_files: false,
                other_files: false,
                text_files: false,
                video_files: false,
            });
            continue;
        }
        let Some(eq) = line.find('=') else { continue };
        let k = line[..eq].trim();
        let v = line[eq + 1..].trim();
        let Some(a) = cur.as_mut() else { continue };
        match k {
            "name" => a.name = v.to_string(),
            "command" => a.command = v.to_string(),
            "patterns" => {
                a.patterns = v.split_whitespace().map(|s| s.to_string()).collect();
            }
            "categories" => {
                for c in v.split_whitespace() {
                    match c {
                        "directories" => a.directories = true,
                        "audio" => a.audio_files = true,
                        "image" => a.image_files = true,
                        "other" => a.other_files = true,
                        "text" => a.text_files = true,
                        "video" => a.video_files = true,
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
    flush(&mut cur, &mut out);
    out
}

fn cats_vec(a: &CustomAction) -> Vec<&'static str> {
    let mut v = Vec::new();
    if a.directories {
        v.push("directories");
    }
    if a.audio_files {
        v.push("audio");
    }
    if a.image_files {
        v.push("image");
    }
    if a.other_files {
        v.push("other");
    }
    if a.text_files {
        v.push("text");
    }
    if a.video_files {
        v.push("video");
    }
    v
}

/// Serialize actions into wfm's own file format.
pub fn serialize_actions(actions: &[CustomAction]) -> String {
    let mut out = String::new();
    out.push_str("# wfm custom actions — one [action] block per action.\n");
    out.push_str("# name = display name\n");
    out.push_str("# command = shell command (%f %F %n %N %d %D %p supported)\n");
    out.push_str("# patterns = space-separated globs (missing = all files)\n");
    out.push_str("# categories = directories audio image other text video (missing = all)\n\n");
    for a in actions {
        out.push_str("[action]\n");
        out.push_str(&format!("name = {}\n", a.name));
        out.push_str(&format!("command = {}\n", a.command));
        if !a.patterns.is_empty() {
            out.push_str(&format!("patterns = {}\n", a.patterns.join(" ")));
        }
        let cats = cats_vec(a);
        if !cats.is_empty() {
            out.push_str(&format!("categories = {}\n", cats.join(" ")));
        }
        out.push('\n');
    }
    out
}

// ------------------------------------------------------------------
// legacy Thunar uca.xml parsing (temporary import only)
// ------------------------------------------------------------------

fn strip_comments(s: &str) -> String {
    let mut out = String::new();
    let mut rest = s;
    while let Some(pos) = rest.find("<!--") {
        out.push_str(&rest[..pos]);
        match rest[pos..].find("-->") {
            Some(end) => rest = &rest[pos + 4 + end..],
            None => break,
        }
    }
    out.push_str(rest);
    out
}

fn decode_entities(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
}

/// Parse a Thunar `uca.xml` document into `CustomAction`s (XML parser kept
/// only for the temporary import option).
pub fn parse_uca_xml(text: &str) -> Vec<CustomAction> {
    let text = strip_comments(text);
    let mut out = Vec::new();
    let mut rest = text.as_str();
    while let Some(start) = rest.find("<action") {
        let after = &rest[start..];
        let open_end = after.find('>').map(|i| i + 1).unwrap_or(after.len());
        let block_start = open_end;
        let Some(close_rel) = after[block_start..].find("</action>") else {
            break;
        };
        let block = &after[block_start..block_start + close_rel];
        if let Some(a) = parse_uca_block(block) {
            out.push(a);
        }
        rest = &after[block_start + close_rel..];
    }
    out
}

fn child_text(block: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let s = block.find(&open)? + open.len();
    let e = block[s..].find(&close)? + s;
    let text = block[s..e].trim();
    if text.is_empty() {
        None
    } else {
        Some(decode_entities(text))
    }
}

fn has_flag(block: &str, tag: &str) -> bool {
    block.contains(&format!("<{tag}/>"))
        || block.contains(&format!("<{tag} />"))
        || block.contains(&format!("<{tag}>"))
}

fn parse_uca_block(block: &str) -> Option<CustomAction> {
    let name = child_text(block, "name")?;
    let command = child_text(block, "command")?;
    let patterns = child_text(block, "patterns")
        .map(|p| p.split_whitespace().map(|s| s.to_string()).collect())
        .unwrap_or_default();
    Some(CustomAction {
        name,
        command,
        patterns,
        directories: has_flag(block, "directories"),
        audio_files: has_flag(block, "audio-files"),
        image_files: has_flag(block, "image-files"),
        other_files: has_flag(block, "other-files"),
        text_files: has_flag(block, "text-files"),
        video_files: has_flag(block, "video-files"),
    })
}

/// Read Thunar's uca.xml and return its actions (temporary import path).
/// `Err` carries a user-facing message when the file is missing/unreadable
/// or contains no actions.
pub fn import_from_thunar() -> Result<Vec<CustomAction>, String> {
    let path = thunar_path();
    let text = std::fs::read_to_string(&path)
        .map_err(|_| format!("no Thunar custom actions found at {}", path.display()))?;
    let actions = parse_uca_xml(&text);
    if actions.is_empty() {
        return Err(format!("{} contains no custom actions", path.display()));
    }
    Ok(actions)
}

// ------------------------------------------------------------------
// matching + expansion
// ------------------------------------------------------------------

/// Coarse category of an entry (via its mime type) for the category flags.
pub fn file_category(e: &Entry) -> FileCat {
    if e.is_dir {
        return FileCat::Directory;
    }
    let m = crate::fs::mime_for_path(Path::new(&e.name));
    if m.starts_with("image/") {
        FileCat::Image
    } else if m.starts_with("audio/") {
        FileCat::Audio
    } else if m.starts_with("video/") {
        FileCat::Video
    } else if m == "text/plain" {
        FileCat::Text
    } else {
        FileCat::Other
    }
}

/// True when the action applies to the given selection: every selected file
/// must fall into an enabled category (all categories allowed when none are
/// flagged) and match at least one `<patterns>` glob.
pub fn action_applies(a: &CustomAction, cats: &[FileCat], names: &[String]) -> bool {
    let flags_set = a.directories
        || a.audio_files
        || a.image_files
        || a.other_files
        || a.text_files
        || a.video_files;
    if flags_set {
        for c in cats {
            let ok = match c {
                FileCat::Directory => a.directories,
                FileCat::Audio => a.audio_files,
                FileCat::Image => a.image_files,
                FileCat::Other => a.other_files,
                FileCat::Text => a.text_files,
                FileCat::Video => a.video_files,
            };
            if !ok {
                return false;
            }
        }
    }
    if a.patterns.is_empty() {
        return true;
    }
    names.iter().all(|n| a.patterns.iter().any(|p| glob_match(p, n)))
}

/// Simple fnmatch-style glob: `*` = any run, `?` = one char. Case-sensitive.
pub fn glob_match(pattern: &str, name: &str) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    let n: Vec<char> = name.chars().collect();
    fn rec(p: &[char], n: &[char]) -> bool {
        if p.is_empty() {
            return n.is_empty();
        }
        match p[0] {
            '*' => rec(&p[1..], n) || (!n.is_empty() && rec(p, &n[1..])),
            '?' => !n.is_empty() && rec(&p[1..], &n[1..]),
            c => !n.is_empty() && n[0] == c && rec(&p[1..], &n[1..]),
        }
    }
    rec(&p, &n)
}

fn quote(s: &str) -> String {
    crate::fs::shell_quote_path(Path::new(s))
}

fn base_name(p: &Path) -> String {
    p.file_name()
        .map(|b| b.to_string_lossy().to_string())
        .unwrap_or_default()
}

/// Expand Thunar's %-codes in a command: `%f %F %n %N %d %D %p` (paths and
/// names are shell-quoted; `%%` escapes a literal `%`).
pub fn expand_command(cmd: &str, paths: &[PathBuf]) -> String {
    if paths.is_empty() {
        return cmd.to_string();
    }
    let first = &paths[0];
    let all_paths = |ps: &[PathBuf]| ps.iter().map(|p| quote(&p.display().to_string())).collect::<Vec<_>>().join(" ");
    let stem = |p: &Path| {
        p.file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default()
    };
    let mut out = String::new();
    let chars: Vec<char> = cmd.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '%' && i + 1 < chars.len() {
            let code = chars[i + 1];
            match code {
                '%' => {
                    out.push('%');
                    i += 2;
                    continue;
                }
                'f' => {
                    out.push_str(&quote(&first.display().to_string()));
                    i += 2;
                    continue;
                }
                'F' => {
                    out.push_str(&all_paths(paths));
                    i += 2;
                    continue;
                }
                'n' => {
                    out.push_str(&quote(&base_name(first)));
                    i += 2;
                    continue;
                }
                'N' => {
                    out.push_str(&paths
                        .iter()
                        .map(|p| quote(&base_name(p)))
                        .collect::<Vec<_>>()
                        .join(" "));
                    i += 2;
                    continue;
                }
                'd' => {
                    if let Some(d) = first.parent() {
                        out.push_str(&quote(&d.display().to_string()));
                    }
                    i += 2;
                    continue;
                }
                'D' => {
                    out.push_str(&paths
                        .iter()
                        .filter_map(|p| p.parent())
                        .map(|d| quote(&d.display().to_string()))
                        .collect::<Vec<_>>()
                        .join(" "));
                    i += 2;
                    continue;
                }
                'p' => {
                    out.push_str(&quote(&first.with_file_name(stem(first)).display().to_string()));
                    i += 2;
                    continue;
                }
                _ => {}
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

/// Write a starter wfm custom-actions file with one commented example if
/// none exists yet. Returns the path.
pub fn ensure_default_actions() -> PathBuf {
    let path = actions_path();
    if path.exists() {
        return path;
    }
    let tpl = "# wfm custom actions — one [action] block per action.\n\
               # name = display name\n\
               # command = shell command (%f %F %n %N %d %D %p supported)\n\
               # patterns = space-separated globs (missing = all files)\n\
               # categories = directories audio image other text video (missing = all)\n\n\
               # [action]\n\
               # name = Open Terminal Here\n\
               # command = exo-open --working-directory %f --launch TerminalEmulator\n\
               # patterns = *\n\
               # categories = directories\n";
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(&path, tpl);
    path
}

#[cfg(test)]
mod tests {
    use super::*;

    const UCA_SAMPLE: &str = r#"<?xml version="1.0"?>
<!-- a comment -->
<actions>
<action>
	<icon>utilities-terminal</icon>
	<name>Open Terminal Here</name>
	<command>exo-open --working-directory %f --launch TerminalEmulator</command>
	<patterns>*</patterns>
	<directories/>
</action>
<action>
	<name>Open in GIMP</name>
	<command>gimp %F</command>
	<patterns>*.png *.jpg *.jpeg</patterns>
	<image-files/>
</action>
</actions>
"#;

    const OWN_SAMPLE: &str = r#"# wfm custom actions
[action]
name = Open Terminal Here
command = exo-open --working-directory %f --launch TerminalEmulator
patterns = *
categories = directories

[action]
name = Open in GIMP
command = gimp %F
patterns = *.png *.jpg *.jpeg
categories = image
"#;

    #[test]
    fn parses_own_format() {
        let a = parse_actions(OWN_SAMPLE);
        assert_eq!(a.len(), 2);
        assert_eq!(a[0].name, "Open Terminal Here");
        assert_eq!(a[0].command, "exo-open --working-directory %f --launch TerminalEmulator");
        assert!(a[0].directories);
        assert!(!a[0].image_files);
        assert_eq!(a[0].patterns, vec!["*".to_string()]);
        assert_eq!(a[1].patterns, vec!["*.png".to_string(), "*.jpg".to_string(), "*.jpeg".to_string()]);
        assert!(a[1].image_files);
    }

    #[test]
    fn parses_uca_xml() {
        let a = parse_uca_xml(UCA_SAMPLE);
        assert_eq!(a.len(), 2);
        assert_eq!(a[0].name, "Open Terminal Here");
        assert!(a[0].directories);
        assert_eq!(a[1].patterns, vec!["*.png".to_string(), "*.jpg".to_string(), "*.jpeg".to_string()]);
        assert!(a[1].image_files);
    }

    #[test]
    fn own_format_roundtrip() {
        let a = parse_actions(OWN_SAMPLE);
        let serialized = serialize_actions(&a);
        let b = parse_actions(&serialized);
        assert_eq!(a.len(), b.len());
        for (x, y) in a.iter().zip(b.iter()) {
            assert_eq!(x.name, y.name);
            assert_eq!(x.command, y.command);
            assert_eq!(x.patterns, y.patterns);
            assert_eq!(x.directories, y.directories);
            assert_eq!(x.audio_files, y.audio_files);
            assert_eq!(x.image_files, y.image_files);
            assert_eq!(x.other_files, y.other_files);
            assert_eq!(x.text_files, y.text_files);
            assert_eq!(x.video_files, y.video_files);
        }
    }

    #[test]
    fn uca_to_own_roundtrip() {
        // Thunar import path: uca.xml → CustomAction → own format → same actions
        let a = parse_uca_xml(UCA_SAMPLE);
        let serialized = serialize_actions(&a);
        let b = parse_actions(&serialized);
        assert_eq!(a.len(), b.len());
        for (x, y) in a.iter().zip(b.iter()) {
            assert_eq!(x.name, y.name);
            assert_eq!(x.command, y.command);
            assert_eq!(x.patterns, y.patterns);
            assert_eq!(x.directories, y.directories);
            assert_eq!(x.image_files, y.image_files);
        }
    }

    #[test]
    fn malformed_lines_are_skipped() {
        let text = "garbage without equals\n[action]\nname = Only Name\n[action]\ncommand = lone-cmd\n";
        let a = parse_actions(text);
        // only the fully-formed action survives (none here) — both dropped
        assert!(a.is_empty());
        let text = "[action]\nname = ok\ncommand = run %f\n";
        let a = parse_actions(text);
        assert_eq!(a.len(), 1);
        assert_eq!(a[0].name, "ok");
    }

    #[test]
    fn glob_matches() {
        assert!(glob_match("*.png", "a.PNG.png"));
        assert!(glob_match("*", "anything"));
        assert!(glob_match("*.png", "x.png"));
        assert!(!glob_match("*.png", "x.jpg"));
        assert!(glob_match("?at", "cat"));
        assert!(!glob_match("?at", "chat"));
        assert!(glob_match("", ""));
    }

    #[test]
    fn applies_by_category_and_pattern() {
        let a = CustomAction {
            name: "img".into(),
            command: "gimp %F".into(),
            patterns: vec!["*.png".into()],
            directories: false,
            audio_files: false,
            image_files: true,
            other_files: false,
            text_files: false,
            video_files: false,
        };
        assert!(action_applies(&a, &[FileCat::Image], &["a.png".into()]));
        assert!(!action_applies(&a, &[FileCat::Image], &["a.jpg".into()]));
        assert!(!action_applies(&a, &[FileCat::Text], &["a.png".into()]));
        // no flags = applies to any category
        let mut b = a.clone();
        b.image_files = false;
        b.patterns.clear();
        assert!(action_applies(&b, &[FileCat::Text], &["a.txt".into()]));
    }

    #[test]
    fn expands_codes() {
        let paths = vec![PathBuf::from("/home/u/a b.png"), PathBuf::from("/home/u/c.txt")];
        let out = expand_command("view %f | cat %N", &paths);
        assert_eq!(out, "view '/home/u/a b.png' | cat 'a b.png' 'c.txt'");
        let out = expand_command("open %F in %d", &paths);
        assert_eq!(out, "open '/home/u/a b.png' '/home/u/c.txt' in '/home/u'");
        let out = expand_command("edit %p", &paths);
        assert_eq!(out, "edit '/home/u/a b'");
        let out = expand_command("100%%", &paths);
        assert_eq!(out, "100%");
    }

    #[test]
    fn categories() {
        use crate::entry::EntryType;
        let mut e = Entry { name: "x.png".into(), kind: EntryType::Image, ..Default::default() };
        assert_eq!(file_category(&e), FileCat::Image);
        e.is_dir = true;
        assert_eq!(file_category(&e), FileCat::Directory);
        e.is_dir = false;
        e.name = "song.mp3".into();
        assert_eq!(file_category(&e), FileCat::Audio);
        e.name = "doc.txt".into();
        assert_eq!(file_category(&e), FileCat::Text);
        e.name = "file.bin".into();
        assert_eq!(file_category(&e), FileCat::Other);
    }
}
