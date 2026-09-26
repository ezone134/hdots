//! IPC — quickshell-style `zen-shell ipc call <handler>` over a unix socket.
//!
//! The running bar binds `$XDG_RUNTIME_DIR/zen-shell.sock` and serves simple
//! line commands (`call open launcher`). The same binary, invoked with an
//! argument, acts as the client — this is the Rust answer to quickshell's
//! `qs ipc call`: a Hyprland keybind or any external tool can do
//!
//! ```sh
//! zen-shell open launcher          # open the app launcher
//! zen-shell open cc                # control center
//! zen-shell open notifications     # notifications panel
//! zen-shell open calendar
//! zen-shell open power             # power menu (hold 3s to act)
//! zen-shell open audio             # per-app volume
//! zen-shell open wallpaper         # wallpaper picker
//! zen-shell open weather           # detailed weather
//! zen-shell open expanded          # the dashboard (as if hovered)
//! zen-shell close                  # collapse back to the pill
//! zen-shell toggle launcher        # open / close
//! zen-shell ws                     # toggle workspace overview
//! zen-shell reload                 # hot-reload ALL configs (per-channel)
//! zen-shell reload colors          # palette-only hot reload ($states2/shell_vars.json)
//! ```
//!
//! The quickshell-flavored forms work too: `zen-shell ipc call open launcher`,
//! `zen-shell ipc call close`.
//!
//! See `docs/ipc.md` for the full command reference (every target, aliases,
//! Hyprland keybind examples).

use crate::shell::Mode;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::time::Duration;

/// The socket the running bar listens on.
pub fn socket_path() -> PathBuf {
    let dir = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".into());
    PathBuf::from(dir).join("zen-shell.sock")
}

/// A command decoded from a client line.
pub enum Cmd {
    Open(Mode),
    Close,
    Toggle(Mode),
    /// `wallpaper set <path>` — set the native wallpaper daemon background
    WallpaperSet(String),
    /// `colors reload` / `reload colors` — re-read the live palette from
    /// $states2/shell_vars.json (palette-only hot swap; call after a theme
    /// pipeline run so the bar picks up the colors)
    ColorsReload,
    /// `mode sync` — re-read the dark/light state from $states2/m_dummy
    ModeSync,
    /// `reload` / `reload config` — hot-reload the entire per-channel
    /// shell.toml config (geometry, pill toggles, colors, animation, etc.)
    /// without restarting the bar. `reload colors` (and `colors reload`)
    /// only re-read the live palette from `$states2/shell_vars.json`.
    ConfigReload,
    /// `mods` — report the $sources/lib modules: what loaded, what each one
    /// publishes, and why anything failed. Read-only, so it is safe to run
    /// from a keybind or a terminal while debugging a plugin.
    Mods,
}

/// Parse a client line (`call <handler> [arg]`).
pub fn parse(line: &str) -> Result<Cmd, String> {
    let mut it = line.split_whitespace();
    if it.next() != Some("call") {
        return Err(format!("expected `call <handler>` (got `{line}`)"));
    }
    let handler = it.next().ok_or("missing handler")?;
    let arg = it.next();
    Ok(match handler {
        "open" => Cmd::Open(open_target(arg)?),
        "close" | "collapse" | "pill" => Cmd::Close,
        "toggle" => Cmd::Toggle(open_target(arg)?),
        // quickshell-style single-name handlers: `ipc call launcher`
        "launcher" => Cmd::Open(Mode::Launcher),
        "cc" | "control_center" => Cmd::Open(Mode::ControlCenter),
        "settings" => Cmd::Open(Mode::Settings),
        "notifications" | "notif" => Cmd::Open(Mode::Notifications),
        "calendar" | "cal" => Cmd::Open(Mode::Calendar),
        "power" => Cmd::Open(Mode::Power),
        "wifi" | "networks" => Cmd::Open(Mode::WifiMenu),
        "bluetooth" | "bt" => Cmd::Open(Mode::BtMenu),
        "audio" | "volume" => Cmd::Open(Mode::Audio),
        "wallpaper" | "wallpapers" => {
            if arg == Some("set") {
                let path = it.next().ok_or("wallpaper set <path>: missing path")?;
                Cmd::WallpaperSet(path.to_string())
            } else {
                Cmd::Open(Mode::Wallpaper)
            }
        }
        "weather" => Cmd::Open(Mode::Weather),
        "clipboard" | "clip" => Cmd::Open(Mode::Clipboard),
        "clipboard_images" | "clipimg" => Cmd::Open(Mode::ClipboardImages),
        "themes" | "theme" => Cmd::Open(Mode::Themes),
        "keybinds" | "kb" => Cmd::Open(Mode::Keybinds),
        // live-state sync: theme scripts (or a terminal) poke these after the
        // $states2 files change — no bar restart, colors just flip
        "colors" | "reload_colors" | "sync_colors" => Cmd::ColorsReload,
        "mode" | "mode_sync" | "sync_mode" => Cmd::ModeSync,
        "reload" | "config_reload" | "configreload" => match arg {
            // `reload colors` — palette-only hot swap (the same as `colors
            // reload`); `reload` / `reload config` — full per-channel config
            Some("colors") => Cmd::ColorsReload,
            _ => Cmd::ConfigReload,
        },
        "mods" | "mod" | "modules" => Cmd::Mods,
        "lock" | "lockscreen" => Cmd::Open(Mode::Lock),
        "expanded" => Cmd::Open(Mode::Expanded),
        "ws" | "ws_switch" | "workspaces" => Cmd::Open(Mode::WorkspaceSwitcher),
        other => return Err(format!("unknown handler `{other}`")),
    })
}

fn open_target(arg: Option<&str>) -> Result<Mode, String> {
    Ok(match arg {
        None | Some("launcher") => Mode::Launcher,
        Some("cc") | Some("control_center") => Mode::ControlCenter,
        Some("settings") => Mode::Settings,
        Some("notifications") | Some("notif") => Mode::Notifications,
        Some("calendar") | Some("cal") => Mode::Calendar,
        Some("power") => Mode::Power,
        Some("wifi") | Some("networks") => Mode::WifiMenu,
        Some("bluetooth") | Some("bt") => Mode::BtMenu,
        Some("audio") | Some("volume") => Mode::Audio,
        Some("wallpaper") | Some("wallpapers") => Mode::Wallpaper,
        Some("weather") => Mode::Weather,
        Some("clipboard") | Some("clip") => Mode::Clipboard,
        Some("clipboard_images") | Some("clipimg") => Mode::ClipboardImages,
        Some("themes") | Some("theme") => Mode::Themes,
        Some("keybinds") | Some("kb") => Mode::Keybinds,
        Some("lock") | Some("lockscreen") => Mode::Lock,
        Some("expanded") => Mode::Expanded,
        Some("ws") | Some("ws_switch") | Some("workspaces") => Mode::WorkspaceSwitcher,
        Some(other) => return Err(format!("unknown target `{other}`")),
    })
}

/// The socket protocol line for a client invocation (the `match` block below
/// try_client — kept here so tests can lock the CLI→protocol mapping without
/// touching a real socket). `None` = args don't look like an IPC call.
fn client_line(first: &str, rest: &[&String]) -> Option<String> {
    let line: String = match (first, rest) {
        // quickshell form: `zen-shell ipc call <handler> [arg]`
        ("ipc", words) => {
            let mut w = words.iter().map(|s| s.as_str());
            let first = w.next().unwrap_or("");
            let rest: Vec<&str> = w.collect();
            if first == "call" {
                // `ipc call open launcher` → `call open launcher`
                format!("call {}", rest.join(" "))
            } else {
                // `ipc open launcher` / `ipc launcher` → `call …`
                format!("call {}", std::iter::once(first).chain(rest).collect::<Vec<_>>().join(" "))
            }
        }
        ("open", [name]) => format!("call open {name}"),
        ("open", []) => "call open".to_string(),
        ("close", []) => "call close".to_string(),
        ("toggle", [name]) => format!("call toggle {name}"),
        ("ws", []) | ("ws_switch", []) | ("workspaces", []) => "call open ws".to_string(),
        ("wallpaper", [verb, path]) if verb.as_str() == "set" => format!("call wallpaper set {path}"),
        // terminal-friendly sync forms
        ("colors", []) => "call colors reload".to_string(),
        ("colors", [w]) if w.as_str() == "reload" => "call colors reload".to_string(),
        ("mode", [w]) if w.as_str() == "sync" => "call mode sync".to_string(),
        ("reload", []) => "call reload".to_string(),
        ("reload", [w]) if w.as_str() == "colors" => "call colors reload".to_string(),
        ("reload", [w]) if w.as_str() == "config" || w.as_str() == "all" => "call reload".to_string(),
        ("config", [w]) if w.as_str() == "reload" => "call reload".to_string(),
        ("mods", []) | ("modules", []) => "call mods".to_string(),
        _ => return None,
    };
    Some(line)
}

/// Client mode. Returns `Some(exit_code)` if `args` look like an IPC
/// invocation (the bar is running elsewhere), `None` if zen-shell should just
/// start the bar. This runs BEFORE the single-instance lock so a caller never
/// needs to own the bar's lock to poke it.
pub fn try_client(args: &[String]) -> Option<i32> {
    let mut it = args.iter();
    let first = it.next()?;
    let rest: Vec<&String> = it.collect();
    let line = client_line(first, &rest)?;
    let path = socket_path();
    let mut sock = match UnixStream::connect(&path) {
        Ok(s) => s,
        Err(_) => {
            eprintln!(
                "zen-shell: no bar running ({} missing) — start zen-shell first",
                path.display()
            );
            return Some(1);
        }
    };
    sock.set_read_timeout(Some(Duration::from_millis(500))).ok();
    let _ = sock.write_all(line.as_bytes());
    let mut reply = String::new();
    let _ = sock.read_to_string(&mut reply);
    print!("{reply}");
    // server replies `error: …` for bad handlers — surface it as a failure
    Some(if reply.starts_with("error") { 1 } else { 0 })
}

/// Fire-and-forget client: send `line` to the running bar, ignore errors.
/// Used by zen-shell itself (the post-theme-apply delayed colors reload).
pub fn poke(line: &str) {
    if let Ok(mut sock) = UnixStream::connect(socket_path()) {
        let _ = sock.write_all(line.as_bytes());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reload_config_vs_reload_colors_are_distinct() {
        // `reload` (no arg / "config") → full config reload; `reload colors`
        // and `colors reload` → palette-only hot swap
        assert!(matches!(parse("call reload").unwrap(), Cmd::ConfigReload));
        assert!(matches!(parse("call reload config").unwrap(), Cmd::ConfigReload));
        assert!(matches!(parse("call reload all").unwrap(), Cmd::ConfigReload));
        assert!(matches!(parse("call reload colors").unwrap(), Cmd::ColorsReload));
        assert!(matches!(parse("call colors reload").unwrap(), Cmd::ColorsReload));
        assert!(matches!(parse("call colors").unwrap(), Cmd::ColorsReload));
    }

    #[test]
    fn mods_is_a_read_only_report_command() {
        assert!(matches!(parse("call mods").unwrap(), Cmd::Mods));
        assert!(matches!(parse("call mod").unwrap(), Cmd::Mods));
        assert!(matches!(parse("call modules").unwrap(), Cmd::Mods));
        // `zen-shell mods` → the client rewrites the bare word onto the socket
        assert_eq!(client_line("mods", &[]).unwrap(), "call mods");
        assert_eq!(client_line("modules", &[]).unwrap(), "call mods");
    }

    fn reffed(v: &[String]) -> Vec<&String> {
        v.iter().collect()
    }

    #[test]
    fn client_forms_map_to_the_same_lines() {
        // the CLI sugar is just a rewrite onto the socket protocol
        let own = |args: &[&str]| args.iter().map(|s| s.to_string()).collect::<Vec<String>>();
        assert_eq!(client_line("reload", &reffed(&own(&["colors"]))).unwrap(), "call colors reload");
        assert_eq!(client_line("reload", &reffed(&own(&["config"]))).unwrap(), "call reload");
        assert_eq!(client_line("reload", &reffed(&own(&["all"]))).unwrap(), "call reload");
        assert_eq!(client_line("reload", &[]).unwrap(), "call reload");
        assert_eq!(client_line("colors", &reffed(&own(&["reload"]))).unwrap(), "call colors reload");
        assert_eq!(client_line("config", &reffed(&own(&["reload"]))).unwrap(), "call reload");
        // `mode sync` and bare `colors` round-trip too
        assert_eq!(client_line("colors", &[]).unwrap(), "call colors reload");
        assert_eq!(client_line("mode", &reffed(&own(&["sync"]))).unwrap(), "call mode sync");
    }
}
