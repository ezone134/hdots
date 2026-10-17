//! Hyprland IPC — workspaces + windows for the overview tape.
//!
//! Two sockets in `$XDG_RUNTIME_DIR/hypr/<instance>/`:
//!  - `.socket.sock`  — one-shot JSON queries (`j/<cmd>`)
//!  - `.socket2.sock` — push events (`EVENT>>payload`), kept nonblocking and
//!    drained by the caller (registered with calloop as a generic source).

use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::time::Duration;

use smithay_client_toolkit::seat::keyboard::Keysym;

/// What a key press should do while the overview is up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyAction {
    /// Consume the key but do nothing.
    Ignore,
    /// Tear the overlay down and hand input back to the focused client.
    Hide,
    /// Scroll the tape so this workspace row is in view.
    ScrollToWs(i64),
    /// Move the selection cursor by N tiles (-1 = left, +1 = right).
    MoveSel(i32),
    /// Move the selection cursor by N rows (-1 = up, +1 = down).
    MoveRow(i32),
    /// Focus the currently selected window and dismiss the overview.
    Activate,
}

impl KeyAction {
    /// The overlay holds an exclusive keyboard grab while visible, so rofi-style
    /// dismissal has to happen in-process: Hyprland never sees the key.
    /// Escape closes, rofi-style, with no modifier required.
    pub fn resolve(keysym: Keysym, visible: bool) -> Self {
        if !visible {
            return KeyAction::Ignore;
        }
        if keysym == Keysym::Escape {
            return KeyAction::Hide;
        }
        if keysym >= Keysym::_1 && keysym <= Keysym::_9 {
            return KeyAction::ScrollToWs((keysym.raw() - Keysym::_1.raw() + 1) as i64);
        }
        // Arrow keys walk the window tiles horizontally, rofi-style.
        match keysym {
            Keysym::Left => return KeyAction::MoveSel(-1),
            Keysym::Right => return KeyAction::MoveSel(1),
            Keysym::Up => return KeyAction::MoveRow(-1),
            Keysym::Down => return KeyAction::MoveRow(1),
            Keysym::Return | Keysym::KP_Enter | Keysym::space => return KeyAction::Activate,
            _ => {}
        }
        KeyAction::Ignore
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Win {
    pub title: String,
    pub class: String,
    pub addr: String,
    pub workspace: i64,
    pub focus_history: i64,
    /// On-screen top-left position — used to order cards left→right like the
    /// actual layout (scrolling columns: x is the visual order).
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Default)]
pub struct Workspace {
    pub id: i64,
    pub name: String,
    pub active: bool,
    pub wins: Vec<Win>,
}

#[derive(Debug, Clone, Default)]
pub struct Snapshot {
    pub workspaces: Vec<Workspace>,
    /// workspace id currently focused
    pub active_id: i64,
}

pub struct Hypr {
    dir: Option<PathBuf>,
    pub event: Option<UnixStream>,
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
        let event = dir
            .as_ref()
            .and_then(|d| UnixStream::connect(d.join(".socket2.sock")).ok())
            .inspect(|s| {
                let _ = s.set_nonblocking(true);
            });
        Hypr { dir, event, buf: Vec::new() }
    }

    pub fn available(&self) -> bool {
        self.dir.is_some()
    }

    /// Send `j/<cmd>` on a fresh connection and return the JSON reply.
    fn query(&self, cmd: &str) -> Option<serde_json::Value> {
        let dir = self.dir.as_ref()?;
        let mut sock = UnixStream::connect(dir.join(".socket.sock")).ok()?;
        sock.set_read_timeout(Some(Duration::from_millis(200))).ok()?;
        sock.write_all(format!("j/{cmd}").as_bytes()).ok()?;
        let mut data = Vec::new();
        let mut tmp = [0u8; 4096];
        loop {
            match sock.read(&mut tmp) {
                Ok(0) => break,
                Ok(n) => data.extend_from_slice(&tmp[..n]),
                Err(_) => break,
            }
            if data.len() > 1 << 20 {
                break;
            }
        }
        serde_json::from_slice(&data).ok()
    }

    /// Fire-and-forget raw request (e.g. `keyword …`, `submap …`).
    pub fn send(&self, req: &str) {
        let Some(dir) = self.dir.as_ref() else { return };
        let Ok(mut sock) = UnixStream::connect(dir.join(".socket.sock")) else { return };
        sock.set_read_timeout(Some(Duration::from_millis(50))).ok();
        let _ = sock.write_all(req.as_bytes());
    }

    /// Fire-and-forget dispatch.
    pub fn dispatch(&self, cmd: &str) {
        self.send(&format!("dispatch {cmd}"));
    }

    /// Set a config keyword at runtime.
    pub fn keyword(&self, k: &str) {
        self.send(&format!("keyword {k}"));
    }

    pub fn focus_window(&self, addr: &str) {
        // This Hyprland build evaluates socket requests through its Lua config
        // bridge (`return hl.dispatch(<args>)`), so raw `dispatch focuswindow
        // address:0x…` fails to parse. Dispatcher objects like
        // `hl.dsp.focus({ window = "address:0x…" })` are accepted. Sent as a
        // plain dispatch request — NOT through keyword().
        let addr = format!("address:0x{}", addr.trim_start_matches("0x"));
        self.send(&format!(
            "dispatch hl.dsp.focus({{ window = \"{addr}\" }})"
        ));
    }

    pub fn switch_workspace(&self, id: i64) {
        // hl.dsp.focus({ workspace = N }) switches to workspace N and focuses
        // its last-used window (same dispatcher family as the keybinds).
        self.send(&format!("dispatch hl.dsp.focus({{ workspace = {id} }})"));
    }

    /// Address (without 0x) of the currently focused window, from a direct
    /// `activewindow` query. More reliable than focusHistoryID: that counter is
    /// a global MRU rank and transiently has no zero entry mid-switch.
    pub fn active_window_addr(&self) -> Option<String> {
        let v = self.query("activewindow")?;
        let a = v.get("address")?.as_str()?;
        Some(a.trim_start_matches("0x").to_string())
    }

    /// Fetch the full workspaces + windows snapshot.
    pub fn snapshot(&self) -> Option<Snapshot> {
        let ws = self.query("workspaces")?;
        let clients = self.query("clients")?;
        let mut snap = Snapshot::default();

        // active workspace id
        if let Some(v) = self.query("activeworkspace") {
            snap.active_id = v.get("id").and_then(|i| i.as_i64()).unwrap_or(1);
        }

        let ws_arr = ws.as_array()?;
        for w in ws_arr {
            let id = w.get("id")?.as_i64().unwrap_or(0);
            let name = w.get("name").and_then(|n| n.as_str()).unwrap_or("").to_string();
            let active = id == snap.active_id;
            snap.workspaces.push(Workspace { id, name, active, wins: Vec::new() });
        }

        // attach windows: only "normal" windows (skip reserved like bar), any
        // workspace overlap allowed.
        if let Some(arr) = clients.as_array() {
            for c in arr {
                let ws_info = match c.get("workspace") {
                    Some(w) => w,
                    None => continue,
                };
                let ws_id = ws_info.get("id").and_then(|i| i.as_i64()).unwrap_or(i64::MAX);

                let class = c.get("class").and_then(|x| x.as_str()).unwrap_or("").to_string();
                let title = c.get("title").and_then(|x| x.as_str()).unwrap_or("").trim().to_string();
                let addr = c.get("address").and_then(|x| x.as_str()).unwrap_or("").trim_start_matches("0x").to_string();
                let focus = c.get("focusHistoryID").and_then(|f| f.as_i64()).unwrap_or(i64::MAX);
                let (x, y) = c
                    .get("at")
                    .and_then(|a| a.as_array())
                    .map(|a| {
                        (
                            a.first().and_then(|v| v.as_i64()).unwrap_or(i64::MAX) as i32,
                            a.get(1).and_then(|v| v.as_i64()).unwrap_or(i64::MAX) as i32,
                        )
                    })
                    .unwrap_or((i32::MAX, i32::MAX));

                // Skip helper windows that clutter the overview.
                let cls = class.to_lowercase();
                if cls.contains("zen-shell") || cls.contains("hyprland") || cls.is_empty() {
                    // keep empty-class only if it has a title
                    if cls.is_empty() && title.is_empty() {
                        continue;
                    }
                    if !cls.is_empty() {
                        continue;
                    }
                }

                let win = Win {
                    title,
                    class,
                    addr,
                    workspace: ws_id,
                    focus_history: focus,
                    x,
                    y,
                };
                if let Some(ws_node) = snap.workspaces.iter_mut().find(|w| w.id == ws_id) {
                    ws_node.wins.push(win);
                }
            }
        }

        // Order each row like the screen: leftmost window first, then
        // rightward (scrolling-columns layout ⇒ x is the visual order; y
        // breaks ties for stacked/floating windows). The clients query itself
        // returns mapping order, which does not match the layout.
        // Workspaces always shown in id order: 1 first, 2 second, and so on.
        // The active row is highlighted but never re-ordered, so the tape is
        // deterministic — workspace N is always row N.
        for ws_node in snap.workspaces.iter_mut() {
            ws_node.wins.sort_by_key(|w| (w.x, w.y));
        }
        snap.workspaces.sort_by_key(|w| w.id);

        Some(snap)
    }

    /// Drain the event socket. Returns (event name, payload) pairs.
    /// Payloads matter: `workspace>>2` names the target row, and
    /// `activewindowv2>>0x…` names the newly focused window address.
    pub fn drain_events(&mut self, sink: &mut Vec<(String, String)>) {
        let Some(sock) = self.event.as_mut() else { return };
        let mut tmp = [0u8; 4096];
        loop {
            match sock.read(&mut tmp) {
                Ok(0) => break,
                Ok(n) => self.buf.extend_from_slice(&tmp[..n]),
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(_) => break,
            }
            if self.buf.len() > 1 << 20 {
                break;
            }
        }
        while let Some(pos) = self.buf.iter().position(|&b| b == b'\n') {
            let line: Vec<u8> = self.buf.drain(..=pos).collect();
            let line = String::from_utf8_lossy(&line);
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let (ev, payload) = line.split_once(">>").unwrap_or((line, ""));
            sink.push((ev.to_string(), payload.to_string()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_always_hides_while_visible() {
        // Hard rule: Escape closes the overview, rofi-style, no modifiers.
        assert_eq!(KeyAction::resolve(Keysym::Escape, true), KeyAction::Hide);
    }

    #[test]
    fn tab_does_nothing() {
        // Toggle handling was removed; Tab is inert.
        assert_eq!(KeyAction::resolve(Keysym::Tab, true), KeyAction::Ignore);
    }

    #[test]
    fn digits_scroll_to_workspace() {
        assert_eq!(KeyAction::resolve(Keysym::_1, true), KeyAction::ScrollToWs(1));
        assert_eq!(KeyAction::resolve(Keysym::_9, true), KeyAction::ScrollToWs(9));
    }

    #[test]
    fn nothing_fires_while_hidden() {
        for ks in [Keysym::Escape, Keysym::Tab, Keysym::_1] {
            assert_eq!(KeyAction::resolve(ks, false), KeyAction::Ignore);
        }
    }

    #[test]
    fn arrows_move_selection() {
        assert_eq!(KeyAction::resolve(Keysym::Left, true), KeyAction::MoveSel(-1));
        assert_eq!(KeyAction::resolve(Keysym::Right, true), KeyAction::MoveSel(1));
    }

    #[test]
    fn enter_activates_selection() {
        assert_eq!(KeyAction::resolve(Keysym::Return, true), KeyAction::Activate);
        assert_eq!(KeyAction::resolve(Keysym::space, true), KeyAction::Activate);
    }

    #[test]
    fn unbound_keys_ignored() {
        assert_eq!(KeyAction::resolve(Keysym::Home, true), KeyAction::Ignore);
        assert_eq!(KeyAction::resolve(Keysym::_0, true), KeyAction::Ignore);
    }

    #[test]
    fn up_down_move_between_rows() {
        assert_eq!(KeyAction::resolve(Keysym::Up, true), KeyAction::MoveRow(-1));
        assert_eq!(KeyAction::resolve(Keysym::Down, true), KeyAction::MoveRow(1));
    }

    #[test]
    fn arrows_inert_while_hidden() {
        assert_eq!(KeyAction::resolve(Keysym::Left, false), KeyAction::Ignore);
        assert_eq!(KeyAction::resolve(Keysym::Return, false), KeyAction::Ignore);
    }
}
