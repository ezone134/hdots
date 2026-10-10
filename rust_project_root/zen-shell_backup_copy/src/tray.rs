//! System tray: `org.kde.StatusNotifierWatcher` (SNI) on a dedicated thread.
//!
//! zbus drives the bus fd; items are introspected once on registration and
//! their icons/pixmaps pushed to the shell over a calloop channel. The main
//! loop never polls — the daemon thread wakes on bus activity or a 10s ping
//! round-trip used only to reap dead items (a `Get` per item, negligible).

use smithay_client_toolkit::reexports::calloop;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use zbus::blocking::{Connection, Proxy};
use zbus::interface;

/// Set once the watcher connection is built (handlers need it to introspect
/// items and to call back into them).
static WCONN: OnceLock<Connection> = OnceLock::new();
/// The live item registry, shared by the watcher, the ping loop and the
/// callback dispatch.
static ITEMS: OnceLock<Arc<Mutex<HashMap<String, TrayItem>>>> = OnceLock::new();

/// An item as sent to the shell for rendering + callbacks.
#[derive(Clone, Debug)]
pub struct TrayItem {
    /// stable id (the item's bus name)
    pub id: String,
    /// bus name to call back into
    pub service: String,
    /// object path of the item
    pub path: String,
    pub title: String,
    pub tooltip: String,
    pub status: String,
    /// themed icon name ("" if none)
    pub icon_name: String,
    /// best pixmap for ~16px, RGBA byte order, premultiplied
    pub pixmap: Option<(u32, u32, Vec<u8>)>,
    /// context-menu object path ("" if none)
    pub menu_path: String,
}

impl TrayItem {
    /// `ImageStore` key: raw pixmap beats the themed name (guaranteed exact).
    pub fn image_key(&self) -> String {
        if self.pixmap.is_some() {
            format!("tray:{}", self.id)
        } else if !self.icon_name.is_empty() {
            format!("icon:{}", self.icon_name)
        } else {
            String::new()
        }
    }
}

/// Messages pushed to the main loop.
pub enum TrayMsg {
    Add(TrayItem),
    Remove(String),
    Ready,
}

/// Commands the shell sends back to the watcher thread.
pub enum TrayCmd {
    Activate(String),
    Scroll(String, i32),
}

/// Spawn the tray daemon thread. Returns the thread handle and the command
/// sender (used by the shell to activate/scroll items).
pub fn spawn(
    sender: calloop::channel::Sender<TrayMsg>,
) -> Option<(std::thread::JoinHandle<()>, std::sync::mpsc::Sender<TrayCmd>)> {
    let (cmd_tx, cmd_rx) = std::sync::mpsc::channel();
    let h = std::thread::Builder::new()
        .name("zen-tray".into())
        .spawn(move || {
            if let Err(e) = run(sender, cmd_rx) {
                eprintln!("zen: tray daemon: {e}");
            }
        })
        .ok()?;
    Some((h, cmd_tx))
}

fn run(
    sender: calloop::channel::Sender<TrayMsg>,
    cmd_rx: std::sync::mpsc::Receiver<TrayCmd>,
) -> zbus::Result<()> {
    use zbus::blocking::connection::Builder as ZBuilder;
    let items = Arc::new(Mutex::new(HashMap::<String, TrayItem>::new()));
    let _ = ITEMS.set(items.clone());
    // Registrations are forwarded from the handler (which runs on zbus's
    // executor thread and must never block) to this free thread.
    let (reg_tx, reg_rx) = std::sync::mpsc::channel::<(String, String)>(); // (service, sender unique name)
    let server = WatcherServer { reg_tx };
    let conn = ZBuilder::session()?
        .serve_at("/StatusNotifierWatcher", server)?
        .build()?;
    let _ = WCONN.set(conn.clone());
    // The watcher well-known name: another watcher may own it (e.g. a stale
    // quickshell) — non-fatal, we still answer on our unique name.
    if let Err(e) = conn.request_name("org.kde.StatusNotifierWatcher") {
        eprintln!("zen: tray: could not claim StatusNotifierWatcher: {e}");
    }
    // We are the host: claim the standard host name so items show up.
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis();
    let host = format!("org.kde.StatusNotifierHost-{}-{}", std::process::id(), stamp);
    if let Err(e) = conn.request_name(host) {
        eprintln!("zen: tray: could not claim host name: {e}");
    }
    let _ = sender.send(TrayMsg::Ready);

    let mut last_ping = Instant::now();
    loop {
        let cmd = cmd_rx.recv_timeout(Duration::from_millis(200));
        match cmd {
            Ok(TrayCmd::Activate(id)) => call_item(&id, |p| {
                let _ = p.call_method("Activate", &(0i32, 0i32));
            }),
            Ok(TrayCmd::Scroll(id, delta)) => call_item(&id, |p| {
                let _ = p.call_method("Scroll", &(delta, "vertical"));
            }),
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
        }
        while let Ok((service, sender_name)) = reg_rx.try_recv() {
            introspect(&sender, &service, &sender_name);
        }
        if last_ping.elapsed() >= Duration::from_secs(10) {
            last_ping = Instant::now();
            reap_dead(&sender);
        }
    }
    zbus::Result::Ok(())
}

/// Fetch an item's properties and push it to the shell (daemon thread only).
fn introspect(sender: &calloop::channel::Sender<TrayMsg>, service: &str, sender_name: &str) {
    let Some(conn) = WCONN.get() else { return };
    // "service" is either an object path (→ the sender's unique name) or a
    // full bus name (→ /StatusNotifierItem).
    let (dest, path) = if service.starts_with('/') {
        (sender_name.to_string(), service.to_string())
    } else {
        (service.to_string(), "/StatusNotifierItem".to_string())
    };
    let Ok(proxy) = Proxy::new(conn, dest.clone(), path.clone(), "org.kde.StatusNotifierItem")
    else {
        return;
    };
    let icon_name: String = proxy.get_property("IconName").unwrap_or_default();
    let title: String = proxy.get_property("Title").unwrap_or_default();
    let id: String = proxy.get_property("Id").unwrap_or_else(|_| dest.clone());
    let status: String = proxy.get_property("Status").unwrap_or_default();
    let tooltip_title: String = proxy.get_property("ToolTipTitle").unwrap_or_default();
    let menu_path: String = proxy.get_property("Menu").unwrap_or_default();
    let pixmaps: Vec<(i32, i32, Vec<u8>)> = proxy.get_property("IconPixmap").unwrap_or_default();

    let item = TrayItem {
        id: dest.clone(),
        service: dest,
        path,
        title: if title.is_empty() { id } else { title.clone() },
        tooltip: if tooltip_title.is_empty() { title } else { tooltip_title },
        status,
        icon_name,
        pixmap: pick_pixmap(&pixmaps, 16),
        menu_path,
    };
    if let Some(registry) = ITEMS.get() {
        registry.lock().unwrap().insert(item.id.clone(), item.clone());
    }
    let _ = sender.send(TrayMsg::Add(item));
}

/// Call a method on the item identified by `id` (its stored service+path).
fn call_item<F>(id: &str, f: F)
where
    F: FnOnce(&Proxy<'_>),
{
    let (Some(conn), Some(registry)) = (WCONN.get(), ITEMS.get()) else { return };
    let Some(item) = registry.lock().unwrap().get(id).cloned() else { return };
    if let Ok(p) = Proxy::new(conn, item.service, item.path, "org.kde.StatusNotifierItem") {
        f(&p);
    }
}

/// Ping every item once; drop the ones that stopped answering.
fn reap_dead(sender: &calloop::channel::Sender<TrayMsg>) {
    let (Some(conn), Some(registry)) = (WCONN.get(), ITEMS.get()) else { return };
    let dead: Vec<String> = {
        let map = registry.lock().unwrap();
        map.iter()
            .filter(|(_, it)| {
                match Proxy::new(
                    conn,
                    it.service.clone(),
                    it.path.clone(),
                    "org.freedesktop.DBus.Properties",
                ) {
                    Ok(p) => p.get_property::<String>("Title").is_err(),
                    Err(_) => true,
                }
            })
            .map(|(k, _)| k.clone())
            .collect()
    };
    for id in dead {
        registry.lock().unwrap().remove(&id);
        let _ = sender.send(TrayMsg::Remove(id));
    }
}

struct WatcherServer {
    /// forwards (service arg, sender's unique name) to the introspecting thread
    reg_tx: std::sync::mpsc::Sender<(String, String)>,
}

#[interface(name = "org.kde.StatusNotifierWatcher")]
impl WatcherServer {
    fn register_status_notifier_item(
        &mut self,
        service: &str,
        #[zbus(header)] header: zbus::message::Header<'_>,
    ) {
        // Runs on zbus's executor thread: never block, never do round-trips.
        // The daemon thread introspects and pushes the item.
        let sender_name = header.sender().map(|u| u.to_string()).unwrap_or_default();
        let _ = self.reg_tx.send((service.to_string(), sender_name));
    }

    #[zbus(property)]
    fn registered_status_notifier_items(&self) -> Vec<String> {
        ITEMS
            .get()
            .map(|m| m.lock().unwrap().keys().cloned().collect())
            .unwrap_or_default()
    }

    #[zbus(property)]
    fn is_status_notifier_host_registered(&self) -> bool {
        true
    }

    #[zbus(property)]
    fn protocol_version(&self) -> i32 {
        0
    }
}

/// Best pixmap for `want` px: closest to target, else the largest available.
fn pick_pixmap(list: &[(i32, i32, Vec<u8>)], want: i32) -> Option<(u32, u32, Vec<u8>)> {
    type P = (i32, i32, Vec<u8>);
    let mut best: Option<(i32, &P)> = None;
    for p in list {
        if p.0 <= 0 || p.1 <= 0 {
            continue;
        }
        let d = (p.0.max(p.1) - want).abs();
        if best.is_none_or(|(bd, _)| d < bd) {
            best = Some((d, p));
        }
    }
    let (w, h, data) = best?.1;
    if data.len() < (w * h * 4) as usize {
        return None;
    }
    // ARGB32 (0xAARRGGBB) little-endian → bytes are B,G,R,A; convert to R,G,B,A.
    // Premultiplied as delivered (matches our GL blend).
    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    for c in data[..(w * h * 4) as usize].chunks_exact(4) {
        rgba.extend_from_slice(&[c[2], c[1], c[0], c[3]]);
    }
    Some((*w as u32, *h as u32, rgba))
}
