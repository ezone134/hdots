//! Minimal StatusNotifierItem used to test zen-shell's tray.
//! Run: `cargo run --example tray_item` (background it), then watch zen.log.

use std::sync::atomic::{AtomicU32, Ordering};
use zbus::interface;

struct Item {
    clicks: AtomicU32,
}

#[interface(name = "org.kde.StatusNotifierItem")]
impl Item {
    fn activate(&self, x: i32, y: i32) {
        println!("[item] Activate({x}, {y})");
        let _ = self.clicks.fetch_add(1, Ordering::Relaxed);
    }
    fn secondary_activate(&self, x: i32, y: i32) {
        println!("[item] SecondaryActivate({x}, {y})");
    }
    fn scroll(&self, delta: i32, orientation: &str) {
        println!("[item] Scroll({delta}, {orientation})");
    }
    #[zbus(property)]
    fn category(&self) -> &str {
        "ApplicationStatus"
    }
    #[zbus(property)]
    fn id(&self) -> &str {
        "zen-test-item"
    }
    #[zbus(property)]
    fn title(&self) -> &str {
        "Zen Test Tray"
    }
    #[zbus(property)]
    fn status(&self) -> &str {
        "Active"
    }
    #[zbus(property)]
    fn icon_name(&self) -> &str {
        "foot"
    }
    #[zbus(property)]
    fn tool_tip(&self) -> (String, Vec<(String, String)>, String, String) {
        (
            "zen-test-item".into(),
            vec![("title".into(), "Zen Test Tray".into())],
            "icon".into(),
            "foot".into(),
        )
    }
}

fn main() -> zbus::Result<()> {
    let conn = zbus::blocking::connection::Builder::session()?.build()?;
    let item = Item { clicks: AtomicU32::new(0) };
    let _ = conn.object_server().at("/StatusNotifierItem", item)?;
    let name = format!("org.kde.StatusNotifierItem-{}-1", std::process::id());
    conn.request_name(name.clone())?;
    let watcher = zbus::blocking::Proxy::new(
        &conn,
        "org.kde.StatusNotifierWatcher",
        "/StatusNotifierWatcher",
        "org.kde.StatusNotifierWatcher",
    )?;
    println!("[item] registering with watcher");
    let _reply = watcher.call_method("RegisterStatusNotifierItem", &(name.as_str()))?;
    println!("[item] registered, waiting...");
    std::thread::park();
    Ok(())
}
