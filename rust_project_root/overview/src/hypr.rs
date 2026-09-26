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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Win {
    pub title: String,
    pub class: String,
    pub addr: String,
    pub workspace: i64,
    pub focus_history: i64,
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

    /// Fire-and-forget dispatch.
    pub fn dispatch(&self, cmd: &str) {
        let Some(dir) = self.dir.as_ref() else { return };
        let Ok(mut sock) = UnixStream::connect(dir.join(".socket.sock")) else { return };
        sock.set_read_timeout(Some(Duration::from_millis(50))).ok();
        let _ = sock.write_all(format!("dispatch {cmd}").as_bytes());
    }

    pub fn focus_window(&self, addr: &str) {
        self.dispatch(&format!("focuswindow address:0x{}", addr.trim_start_matches("0x")));
    }

    pub fn switch_workspace(&self, id: i64) {
        self.dispatch(&format!("workspace {id}"));
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
                };
                if let Some(ws_node) = snap.workspaces.iter_mut().find(|w| w.id == ws_id) {
                    ws_node.wins.push(win);
                }
            }
        }

        // sort wins by focus history (0 = most recent first) within each ws
        for ws_node in snap.workspaces.iter_mut() {
            ws_node.wins.sort_by_key(|w| w.focus_history);
        }
        // workspaces sorted by id, active first
        snap.workspaces.sort_by(|a, b| {
            let aa = if a.active { 0 } else { 1 };
            let bb = if b.active { 0 } else { 1 };
            aa.cmp(&bb).then(a.id.cmp(&b.id))
        });

        Some(snap)
    }

    /// Drain the event socket. Returns the raw list of event names that fired.
    pub fn drain_events(&mut self, sink: &mut Vec<String>) {
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
            let (ev, _payload) = line.split_once(">>").unwrap_or((line, ""));
            sink.push(ev.to_string());
        }
    }
}
