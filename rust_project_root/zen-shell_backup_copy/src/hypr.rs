//! Hyprland IPC — real workspaces, active window title, live events.
//!
//! Two sockets live in `$XDG_RUNTIME_DIR/hypr/<instance>/`:
//!  - `.socket.sock`  — one-shot JSON queries (`j/<cmd>`), fresh connection per query
//!  - `.socket2.sock` — push events (`EVENT>>payload`, newline-delimited). This fd
//!    is registered with calloop, so the shell only wakes when the compositor
//!    actually changes something (0-CPU idle).

use crate::shell::Shell;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::time::Duration;

pub struct Hypr {
    /// query socket dir (kept to reconnect cheaply)
    dir: Option<PathBuf>,
    /// event socket (nonblocking, registered with calloop)
    pub event: Option<UnixStream>,
    /// partial-line buffer for event parsing
    buf: Vec<u8>,
}

fn socket_dir() -> Option<PathBuf> {
    let sig = std::env::var("HYPRLAND_INSTANCE_SIGNATURE").ok()?;
    let runtime = std::env::var("XDG_RUNTIME_DIR").ok()?;
    let dir = PathBuf::from(runtime).join("hypr").join(sig);
    if dir.join(".socket.sock").exists() && dir.join(".socket2.sock").exists() {
        Some(dir)
    } else {
        None
    }
}

impl Hypr {
    pub fn new() -> Self {
        let dir = socket_dir();
        let event = dir.as_ref().and_then(|d| UnixStream::connect(d.join(".socket2.sock")).ok()).inspect(|s| {
            let _ = s.set_nonblocking(true);
        });
        if dir.is_some() && std::env::var("ZEN_TRACE").is_ok() {
            eprintln!("zen: hyprland ipc connected");
        }
        Hypr { dir, event, buf: Vec::new() }
    }

    /// Send `j/<cmd>` on a fresh connection and return the JSON reply.
    fn query(&self, cmd: &str) -> Option<serde_json::Value> {
        let dir = self.dir.as_ref()?;
        let mut sock = UnixStream::connect(dir.join(".socket.sock")).ok()?;
        sock.set_read_timeout(Some(Duration::from_millis(200))).ok()?;
        sock.write_all(format!("j/{cmd}").as_bytes()).ok()?;
        let mut data = Vec::new();
        let mut tmp = [0u8; 4096];
        // Hyprland writes the full reply in one go; drain until silence.
        loop {
            match sock.read(&mut tmp) {
                Ok(0) => break,
                Ok(n) => data.extend_from_slice(&tmp[..n]),
                Err(_) => break, // timeout → reply complete
            }
            if data.len() > 1 << 20 {
                break; // sanity cap
            }
        }
        serde_json::from_slice(&data).ok()
    }

    /// Fire-and-forget Hyprland dispatch command (no JSON reply expected).
    /// Sends a raw `dispatch …` line to `.socket.sock`.
    fn dispatch(&self, cmd: &str) {
        let Some(dir) = self.dir.as_ref() else { return };
        let Ok(mut sock) = UnixStream::connect(dir.join(".socket.sock")) else { return };
        sock.set_read_timeout(Some(Duration::from_millis(50))).ok();
        let _ = sock.write_all(format!("dispatch {cmd}").as_bytes());
    }

    /// Switch to workspace by 1-indexed id (Hyprland convention).
    pub fn dispatch_workspace(&self, id: usize) {
        self.dispatch(&format!("workspace {id}"));
    }

    /// Global cursor position (logical px) via `j/cursorpos`.
    pub fn cursor_pos(&self) -> Option<(f64, f64)> {
        let v = self.query("cursorpos")?;
        let x = v.get("x")?.as_f64()?;
        let y = v.get("y")?.as_f64()?;
        Some((x, y))
    }

    /// Name of the focused monitor via `j/monitors` (for the wallpaper
    /// picker's `{monitor}` substitution).
    pub fn focused_monitor(&self) -> Option<String> {
        let v = self.query("monitors")?;
        let arr = v.as_array()?;
        for m in arr {
            if m.get("focused").and_then(|f| f.as_bool()).unwrap_or(false) {
                return m.get("name").and_then(|n| n.as_str()).map(|s| s.to_string());
            }
        }
        None
    }

    /// (width, height) of the focused monitor — the bar surface math needs
    /// real monitor dims when the strip is anchored to a left/right edge
    /// (vertical centering) instead of the top/bottom centering inference.
    pub fn focused_monitor_size(&self) -> Option<(f64, f64)> {
        let v = self.query("monitors")?;
        let arr = v.as_array()?;
        for m in arr {
            if m.get("focused").and_then(|f| f.as_bool()).unwrap_or(false) {
                let w = m.get("width").and_then(|v| v.as_f64())?;
                let h = m.get("height").and_then(|v| v.as_f64())?;
                return Some((w, h));
            }
        }
        None
    }

    /// Global geometry (x, y, w, h in logical px) of our own layer surface,
    /// via `j/layers` (namespace `zen-shell`). Ground truth for deciding
    /// whether the cursor is over the bar when the pointer event flow has
    /// gone stale (no focus → no enter/motion/leave).
    pub fn layer_geom(&self) -> Option<(f64, f64, f64, f64)> {
        let v = self.query("layers")?;
        let monitors = v.as_object()?;
        for info in monitors.values() {
            let levels = info.get("levels")?.as_object()?;
            for arr in levels.values() {
                for s in arr.as_array()? {
                    if s.get("namespace")?.as_str()? == "zen-shell" {
                        let x = s.get("x")?.as_f64()?;
                        let y = s.get("y")?.as_f64()?;
                        let w = s.get("w")?.as_f64()?;
                        let h = s.get("h")?.as_f64()?;
                        return Some((x, y, w, h));
                    }
                }
            }
        }
        None
    }

    /// Window titles on a workspace (most recently focused first), via
    /// `j/clients` (`j/windows` is a newer request this Hyprland rejects).
    pub fn workspace_windows(&self, ws_id: usize) -> Option<Vec<String>> {
        let v = self.query("clients")?;
        let arr = v.as_array()?;
        let mut wins: Vec<(i64, String)> = Vec::new();
        for c in arr {
            let ws = c.get("workspace")?;
            let id = ws.get("id")?.as_i64()?;
            if id != ws_id as i64 {
                continue;
            }
            let title = c
                .get("title")?
                .as_str()
                .unwrap_or("")
                .trim()
                .to_string();
            if title.is_empty() {
                continue;
            }
            // focusHistoryID 0 = most recently focused
            let focus = c.get("focusHistoryID").and_then(|f| f.as_i64()).unwrap_or(i64::MAX);
            wins.push((focus, title));
        }
        wins.sort_by_key(|(focus, _)| *focus);
        Some(wins.into_iter().map(|(_, t)| t.chars().take(48).collect()).collect())
    }

    /// Refresh the shell's workspace list + active window from Hyprland.
    /// Returns true if anything actually changed (so callers only re-render
    /// when the values differ).
    pub fn update_shell(&self, shell: &mut Shell) -> bool {
        let mut changed = false;
        // workspaces → count
        if let Some(v) = self.query("workspaces") {
            if let Some(arr) = v.as_array() {
                let count = arr.iter().filter_map(|w| w.get("id")?.as_i64()).max().map(|m| m as usize + 1).unwrap_or(1);
                if count >= 1 && count != shell.ws_count {
                    shell.ws_count = count.max(1);
                    changed = true;
                }
            }
        }
        // activeworkspace → active id
        if let Some(v) = self.query("activeworkspace") {
            if let Some(id) = v.get("id").and_then(|i| i.as_i64()) {
                let active = (id as usize).saturating_sub(1);
                if active != shell.ws_active {
                    shell.ws_active = active;
                    changed = true;
                }
            }
        }
        // activewindow → title
        if let Some(v) = self.query("activewindow") {
            let title = v.get("title").and_then(|t| t.as_str()).unwrap_or("");
            let t = title.chars().take(48).collect::<String>();
            if t != shell.title {
                shell.title = t;
                changed = true;
            }
        }
        changed
    }

    /// Drain the event socket. Returns true if anything changed.
    pub fn drain_events(&mut self, shell: &mut Shell) -> bool {
        let Some(sock) = self.event.as_mut() else { return false };
        let mut changed = false;
        let mut tmp = [0u8; 4096];
        loop {
            match sock.read(&mut tmp) {
                Ok(0) => break,
                Ok(n) => {
                    self.buf.extend_from_slice(&tmp[..n]);
                    changed = true;
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(_) => break,
            }
            if self.buf.len() > 1 << 20 {
                break;
            }
        }
        if !changed {
            return false;
        }
        // split lines, keep the trailing partial
        let mut lines: Vec<Vec<u8>> = Vec::new();
        while let Some(pos) = self.buf.iter().position(|&b| b == b'\n') {
            lines.push(self.buf.drain(..=pos).collect());
        }
        for line in lines {
            let line = String::from_utf8_lossy(&line);
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let (ev, _payload) = line.split_once(">>").unwrap_or((line, ""));
            match ev {
                "workspace" | "createworkspace" | "destroyworkspace" | "focusedmon" => {
                    self.update_shell(shell);
                }
                "activewindow" | "movewindow" | "openwindow" | "closewindow" => {
                    self.update_shell(shell);
                }
                _ => {}
            }
        }
        true
    }
}
