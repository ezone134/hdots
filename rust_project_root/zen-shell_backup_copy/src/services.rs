//! Live system services via D-Bus — MPRIS media, upower battery, NetworkManager SSID.
//!
//! Polled on a slow timer (2 s), never blocking the wayland loop for long:
//! each `poll()` is a handful of local-socket property reads (sub-ms), and the
//! shell only wakes when the timer fires, so idle CPU stays at 0.

use crate::shell::Shell;
use zbus::blocking::Connection;
use zbus::zvariant::{OwnedObjectPath, OwnedValue, Value};

pub struct Services {
    session: Option<Connection>,
    system: Option<Connection>,
    /// active MPRIS player bus name (cached so we don't re-list every poll)
    player: Option<String>,
}

/// `org.freedesktop.DBus.Properties.Get` → the property value as OwnedValue.
fn prop(conn: &Connection, service: &str, path: &str, iface: &str, name: &str) -> Option<OwnedValue> {
    let msg = conn
        .call_method(
            Some(service),
            path,
            Some("org.freedesktop.DBus.Properties"),
            "Get",
            &(iface, name),
        )
        .ok()?;
    msg.body().deserialize::<OwnedValue>().ok()
}

/// Unwrap a variant-wrapped value (`Value::Value(boxed)` from `Properties.Get`).
fn unwrap<'a>(v: &'a Value<'a>) -> &'a Value<'a> {
    match v {
        Value::Value(b) => unwrap(b),
        other => other,
    }
}

fn as_str(v: &Value) -> Option<String> {
    match unwrap(v) {
        Value::Str(s) => Some(s.to_string()),
        _ => None,
    }
}

fn as_f64(v: &Value) -> Option<f64> {
    match unwrap(v) {
        Value::F64(f) => Some(*f),
        Value::I32(i) => Some(*i as f64),
        Value::U32(u) => Some(*u as f64),
        _ => None,
    }
}

fn as_i64(v: &Value) -> Option<i64> {
    match unwrap(v) {
        Value::I64(i) => Some(*i),
        Value::U64(u) => Some(*u as i64),
        Value::I32(i) => Some(*i as i64),
        Value::U32(u) => Some(*u as i64),
        _ => None,
    }
}

fn as_bool(v: &Value) -> Option<bool> {
    match unwrap(v) {
        Value::Bool(b) => Some(*b),
        _ => None,
    }
}

/// First element of a string array (`as`), e.g. MPRIS `xesam:artist`.
fn first_str_in_array(v: &Value) -> Option<String> {
    let v = unwrap(v);
    let Value::Array(arr) = v else { return None };
    arr.iter().find_map(|el| as_str(el))
}

impl Services {
    pub fn new() -> Self {
        let session = Connection::session().ok();
        let system = Connection::system().ok();
        Services { session, system, player: None }
    }

    /// Refresh all services into `shell`. Returns true if anything changed
    /// (caller redraws only then).
    pub fn poll(&mut self, shell: &mut Shell) -> bool {
        let mut changed = false;
        changed |= self.poll_mpris(shell);
        changed |= self.poll_network(shell);
        changed |= self.poll_wifi_networks(shell);
        changed |= self.poll_bt_devices(shell);
        changed
    }

    // ---------------------------------------------------------------- MPRIS

    fn find_player(&mut self) -> Option<String> {
        // cached player still alive?
        let conn = self.session.as_ref()?;
        if let Some(p) = &self.player {
            if prop(
                conn,
                p,
                "/org/mpris/MediaPlayer2",
                "org.mpris.MediaPlayer2.Player",
                "PlaybackStatus",
            )
            .is_some()
            {
                return Some(p.clone());
            }
        }
        let msg = conn
            .call_method(
                Some("org.freedesktop.DBus"),
                "/org/freedesktop/DBus",
                Some("org.freedesktop.DBus"),
                "ListNames",
                &(),
            )
            .ok()?;
        let names: Vec<String> = msg.body().deserialize().ok()?;
        let p = names
            .into_iter()
            .find(|n| n.starts_with("org.mpris.MediaPlayer2."))
            .unwrap_or_default();
        if p.is_empty() {
            self.player = None;
            None
        } else {
            self.player = Some(p.clone());
            Some(p)
        }
    }

    fn poll_mpris(&mut self, shell: &mut Shell) -> bool {
        let Some(player) = self.find_player() else {
            // no player → clear media if it was set
            if !shell.media_title.is_empty()
                || !shell.media_artist.is_empty()
                || !shell.media_album.is_empty()
                || shell.media_playing
                || shell.media_len != 0
                || !shell.media_art.is_empty()
            {
                shell.media_title.clear();
                shell.media_artist.clear();
                shell.media_album.clear();
                shell.media_playing = false;
                shell.media_pos = 0;
                shell.media_len = 0;
                shell.media_art.clear();
                shell.media_seek = None;
                return true;
            }
            return false;
        };
        let Some(conn) = self.session.as_ref() else { return false };
        let Some(meta) = prop(conn, &player, "/org/mpris/MediaPlayer2", "org.mpris.MediaPlayer2.Player", "Metadata") else {
            return false;
        };
        let status = prop(conn, &player, "/org/mpris/MediaPlayer2", "org.mpris.MediaPlayer2.Player", "PlaybackStatus")
            .and_then(|v| as_str(&v))
            .unwrap_or_default();
        let pos = prop(conn, &player, "/org/mpris/MediaPlayer2", "org.mpris.MediaPlayer2.Player", "Position")
            .and_then(|v| as_i64(&v))
            .unwrap_or(shell.media_pos);

        let mut title = String::new();
        let mut artist = String::new();
        let mut album = String::new();
        let mut len = 0i64;
        let mut art = String::new();
        // Metadata is `a{sv}` → iterate the dict; only `s`/`as` entries matter.
        if let Value::Dict(dict) = unwrap(&meta) {
            for (k, v) in dict.iter() {
                match as_str(k).as_deref() {
                    Some("xesam:title") => title = as_str(v).unwrap_or_default(),
                    Some("xesam:artist") => artist = first_str_in_array(v).unwrap_or_default(),
                    Some("xesam:album") => album = as_str(v).unwrap_or_default(),
                    Some("mpris:length") => len = as_i64(v).unwrap_or(0),
                    Some("mpris:artUrl") => {
                        // local art only (file://) — https needs TLS we don't
                        // ship; the dashboard falls back to a placeholder tile
                        if let Some(u) = as_str(v) {
                            if let Some(p) = u.strip_prefix("file://") {
                                art = p.to_string();
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        let playing = status == "Playing";
        let mut changed = false;
        if title != shell.media_title {
            shell.media_title = title;
            changed = true;
        }
        if artist != shell.media_artist {
            shell.media_artist = artist;
            changed = true;
        }
        if album != shell.media_album {
            shell.media_album = album;
            changed = true;
        }
        if playing != shell.media_playing {
            shell.media_playing = playing;
            changed = true;
        }
        if pos != shell.media_pos {
            shell.media_pos = pos;
            changed = true;
        }
        if len != shell.media_len {
            shell.media_len = len;
            changed = true;
        }
        if art != shell.media_art {
            shell.media_art = art;
            changed = true;
        }
        // keep the interpolation clock fresh even when nothing changed
        shell.media_pos_at = std::time::Instant::now();
        changed
    }

    // ----------------------------------------------------- wifi networks

    /// First NetworkManager wifi device path (DeviceType 2).
    fn wifi_device(&self, conn: &Connection) -> Option<OwnedObjectPath> {
        let devs = prop(
            conn,
            "org.freedesktop.NetworkManager",
            "/org/freedesktop/NetworkManager",
            "org.freedesktop.NetworkManager",
            "Devices",
        )?;
        let Value::Array(arr) = unwrap(&devs) else { return None };
        for el in arr.iter() {
            let Value::ObjectPath(p) = unwrap(el) else { continue };
            let ty = prop(
                conn,
                "org.freedesktop.NetworkManager",
                p.as_str(),
                "org.freedesktop.NetworkManager.Device",
                "DeviceType",
            )
            .and_then(|v| as_f64(&v))
            .unwrap_or(0.0);
            if ty == 2.0 {
                return Some(OwnedObjectPath::from(p.clone()));
            }
        }
        None
    }

    fn poll_wifi_networks(&mut self, shell: &mut Shell) -> bool {
        let Some(conn) = self.system.as_ref() else { return false };
        let Some(dev) = self.wifi_device(conn) else { return false };
        let active_ap = prop(
            conn,
            "org.freedesktop.NetworkManager",
            dev.as_str(),
            "org.freedesktop.NetworkManager.Device.Wireless",
            "ActiveAccessPoint",
        )
        .and_then(|v| match unwrap(&v) {
            Value::ObjectPath(p) => Some(p.to_string()),
            _ => None,
        })
        .unwrap_or_default();
        let aps = conn
            .call_method(
                Some("org.freedesktop.NetworkManager"),
                dev.as_str(),
                Some("org.freedesktop.NetworkManager.Device.Wireless"),
                "GetAccessPoints",
                &(),
            )
            .ok();
        let ap_paths: Vec<OwnedObjectPath> = aps.and_then(|m| m.body().deserialize().ok()).unwrap_or_default();
        let mut nets: Vec<(String, u8, bool, bool)> = Vec::new();
        for ap in ap_paths {
            let ssid_ay = prop(
                conn,
                "org.freedesktop.NetworkManager",
                ap.as_str(),
                "org.freedesktop.NetworkManager.AccessPoint",
                "Ssid",
            )
            .and_then(|v| match unwrap(&v) {
                Value::Array(a) => Some(
                    a.iter()
                        .filter_map(|e| match unwrap(e) {
                            Value::U8(b) => Some(*b),
                            _ => None,
                        })
                        .collect::<Vec<u8>>(),
                ),
                _ => None,
            })
            .unwrap_or_default();
            if ssid_ay.is_empty() {
                continue;
            }
            let ssid = String::from_utf8_lossy(&ssid_ay).to_string();
            let strength = prop(
                conn,
                "org.freedesktop.NetworkManager",
                ap.as_str(),
                "org.freedesktop.NetworkManager.AccessPoint",
                "Strength",
            )
            .and_then(|v| as_f64(&v))
            .unwrap_or(0.0) as u8;
            let rsn = prop(
                conn,
                "org.freedesktop.NetworkManager",
                ap.as_str(),
                "org.freedesktop.NetworkManager.AccessPoint",
                "RsnFlags",
            )
            .and_then(|v| as_f64(&v))
            .unwrap_or(0.0) as u32;
            let wpa = prop(
                conn,
                "org.freedesktop.NetworkManager",
                ap.as_str(),
                "org.freedesktop.NetworkManager.AccessPoint",
                "WpaFlags",
            )
            .and_then(|v| as_f64(&v))
            .unwrap_or(0.0) as u32;
            let secured = rsn != 0 || wpa != 0;
            let connected = ap.as_str() == active_ap;
            nets.push((ssid, strength, secured, connected));
        }
        // strongest first, connected on top, dedupe by ssid
        nets.sort_by_key(|n| (!n.3, std::cmp::Reverse(n.1)));
        let mut seen = Vec::new();
        nets.retain(|n| {
            if seen.contains(&n.0) {
                false
            } else {
                seen.push(n.0.clone());
                true
            }
        });
        if nets != shell.wifi_networks {
            shell.wifi_networks = nets;
            true
        } else {
            false
        }
    }

    /// Ask NetworkManager to rescan (call when the network menu opens).
    pub fn request_wifi_scan(&self) {
        let Some(conn) = self.system.as_ref() else { return };
        let Some(dev) = self.wifi_device(conn) else { return };
        let _ = conn.call_method(
            Some("org.freedesktop.NetworkManager"),
            dev.as_str(),
            Some("org.freedesktop.NetworkManager.Device.Wireless"),
            "RequestScan",
            &(std::collections::HashMap::<String, Value>::new(),),
        );
    }

    /// Connect to a Wi-Fi network (open networks connect directly; secured
    /// ones may fail without a saved password — the OS agent can't prompt).
    pub fn connect_wifi(&self, ssid: &str) {
        let Some(conn) = self.system.as_ref() else { return };
        let Some(dev) = self.wifi_device(conn) else { return };
        let t = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let uuid = format!(
            "{:08x}-{:04x}-{:04x}-{:04x}-{:012x}",
            (t >> 64) as u32,
            (t >> 48) as u16,
            (t >> 32) as u16,
            (t >> 16) as u16,
            t as u64 & 0xffff_ffff_ffff
        );
        let mut wireless = std::collections::HashMap::new();
        wireless.insert("ssid".to_string(), Value::from(ssid.as_bytes().to_vec()));
        wireless.insert("mode".to_string(), Value::from("infrastructure"));
        let mut connection = std::collections::HashMap::new();
        connection.insert("type".to_string(), Value::from("802-11-wireless"));
        connection.insert("id".to_string(), Value::from(ssid));
        connection.insert("uuid".to_string(), Value::from(uuid));
        let profile = [
            ("802-11-wireless".to_string(), wireless),
            ("connection".to_string(), connection),
        ]
        .into_iter()
        .collect::<std::collections::HashMap<_, _>>();
        let _ = conn.call_method(
            Some("org.freedesktop.NetworkManager"),
            "/org/freedesktop/NetworkManager",
            Some("org.freedesktop.NetworkManager"),
            "AddAndActivateConnection",
            &(profile, dev.as_str(), "/"),
        );
    }

    /// Toggle the NM wifi radio.
    pub fn set_wifi_powered(&self, on: bool) {
        let Some(conn) = self.system.as_ref() else { return };
        let _ = conn.call_method(
            Some("org.freedesktop.NetworkManager"),
            "/org/freedesktop/NetworkManager",
            Some("org.freedesktop.DBus.Properties"),
            "Set",
            &(
                "org.freedesktop.NetworkManager",
                "WirelessEnabled",
                Value::from(on),
            ),
        );
    }

    // ------------------------------------------------------- bluetooth

    fn poll_bt_devices(&mut self, shell: &mut Shell) -> bool {
        let Some(conn) = self.system.as_ref() else { return false };
        let Ok(msg) = conn.call_method(
            Some("org.bluez"),
            "/",
            Some("org.freedesktop.DBus.ObjectManager"),
            "GetManagedObjects",
            &(),
        ) else {
            return false;
        };
        let body = msg.body();
        let Ok(v) = body.deserialize::<Value>() else { return false };
        let Value::Dict(objs) = v else { return false };
        let mut on = false;
        let mut devs: Vec<(String, bool, bool)> = Vec::new();
        for (path, ifaces) in objs.iter() {
            let p = as_str(path).unwrap_or_default();
            if !p.starts_with("/org/bluez/hci") {
                continue;
            }
            let Value::Dict(ifaces) = ifaces else { continue };
            for (iface, props) in ifaces.iter() {
                let name = as_str(iface).unwrap_or_default();
                let Value::Dict(props) = props else { continue };
                let get = |k: &str| {
                    props
                        .iter()
                        .find(|(pk, _)| as_str(pk).as_deref() == Some(k))
                        .map(|(_, pv)| pv.clone())
                };
                match name.as_str() {
                    "org.bluez.Adapter1" => {
                        on = get("Powered").and_then(|v| as_bool(&v)).unwrap_or(false);
                    }
                    "org.bluez.Device1" => {
                        let dname = get("Name").and_then(|v| as_str(&v)).unwrap_or_default();
                        if dname.is_empty() {
                            continue;
                        }
                        let connected = get("Connected").and_then(|v| as_bool(&v)).unwrap_or(false);
                        let paired = get("Paired").and_then(|v| as_bool(&v)).unwrap_or(false);
                        devs.push((dname, connected, paired));
                    }
                    _ => {}
                }
            }
        }
        let mut changed = false;
        if on != shell.bt_on {
            shell.bt_on = on;
            changed = true;
        }
        if devs != shell.bt_devices {
            shell.bt_devices = devs;
            changed = true;
        }
        changed
    }

    /// Power the bluez adapter on/off.
    pub fn set_bt_powered(&self, on: bool) {
        let Some(conn) = self.system.as_ref() else { return };
        if let Ok(msg) = conn.call_method(
            Some("org.bluez"),
            "/",
            Some("org.freedesktop.DBus.ObjectManager"),
            "GetManagedObjects",
            &(),
        ) {
            let body = msg.body();
            if let Ok(v) = body.deserialize::<Value>() {
                let Value::Dict(objs) = v else { return };
                for (path, _) in objs.iter() {
                    let p = as_str(path).unwrap_or_default();
                    if p.starts_with("/org/bluez/hci") {
                        let _ = conn.call_method(
                            Some("org.bluez"),
                            p.as_str(),
                            Some("org.freedesktop.DBus.Properties"),
                            "Set",
                            &("org.bluez.Adapter1", "Powered", Value::from(on)),
                        );
                        break;
                    }
                }
            }
        }
    }

    /// Connect (or disconnect, when already connected) a bluetooth device.
    pub fn toggle_bt(&self, name: &str) {
        let Some(conn) = self.system.as_ref() else { return };
        if let Ok(msg) = conn.call_method(
            Some("org.bluez"),
            "/",
            Some("org.freedesktop.DBus.ObjectManager"),
            "GetManagedObjects",
            &(),
        ) {
            let body = msg.body();
            if let Ok(v) = body.deserialize::<Value>() {
                let Value::Dict(objs) = v else { return };
                for (path, ifaces) in objs.iter() {
                    let p = as_str(path).unwrap_or_default();
                    if !p.starts_with("/org/bluez/hci") {
                        continue;
                    }
                    let Value::Dict(ifaces) = ifaces else { continue };
                    for (iface, props) in ifaces.iter() {
                        if as_str(iface).as_deref() != Some("org.bluez.Device1") {
                            continue;
                        }
                        let Value::Dict(props) = props else { continue };
                        let dname = props
                            .iter()
                            .find(|(k, _)| as_str(k).as_deref() == Some("Name"))
                            .and_then(|(_, v)| as_str(v));
                        if dname.as_deref() != Some(name) {
                            continue;
                        }
                        let connected = props
                            .iter()
                            .find(|(k, _)| as_str(k).as_deref() == Some("Connected"))
                            .and_then(|(_, v)| as_bool(v))
                            .unwrap_or(false);
                        let method = if connected { "Disconnect" } else { "Connect" };
                        let _ = conn.call_method(
                            Some("org.bluez"),
                            p.as_str(),
                            Some("org.bluez.Device1"),
                            method,
                            &(),
                        );
                        return;
                    }
                }
            }
        }
    }

    // ------------------------------------------------------- NetworkManager

    fn poll_network(&mut self, shell: &mut Shell) -> bool {
        let Some(conn) = self.system.as_ref() else { return false };
        let wifi_on = prop(
            conn,
            "org.freedesktop.NetworkManager",
            "/org/freedesktop/NetworkManager",
            "org.freedesktop.NetworkManager",
            "WirelessEnabled",
        )
        .and_then(|v| as_bool(&v))
        .unwrap_or(true);

        // first active connection of type 802-11-wireless → its Id is the SSID
        let mut ssid = String::new();
        if let Some(conns) = prop(
            conn,
            "org.freedesktop.NetworkManager",
            "/org/freedesktop/NetworkManager",
            "org.freedesktop.NetworkManager",
            "ActiveConnections",
        )
        .and_then(|v| match unwrap(&v) {
            Value::Array(arr) => Some(
                arr.iter()
                    .filter_map(|el| match unwrap(el) {
                        Value::ObjectPath(p) => Some(OwnedObjectPath::from(p.clone())),
                        _ => None,
                    })
                    .collect::<Vec<_>>(),
            ),
            _ => None,
        }) {
            for c in conns {
                let ty = prop(
                    conn,
                    "org.freedesktop.NetworkManager",
                    c.as_str(),
                    "org.freedesktop.NetworkManager.Connection.Active",
                    "Type",
                )
                .and_then(|v| as_str(&v))
                .unwrap_or_default();
                if ty == "802-11-wireless" {
                    if let Some(id) = prop(
                        conn,
                        "org.freedesktop.NetworkManager",
                        c.as_str(),
                        "org.freedesktop.NetworkManager.Connection.Active",
                        "Id",
                    )
                    .and_then(|v| as_str(&v))
                    {
                        ssid = id;
                    }
                    break;
                }
            }
        }

        let mut changed = false;
        if wifi_on != shell.wifi_on {
            shell.wifi_on = wifi_on;
            changed = true;
        }
        if ssid != shell.ssid {
            shell.ssid = ssid;
            changed = true;
        }
        changed
    }

    /// MPRIS transport control for the media buttons.
    pub fn media_command(&self, cmd: &str) {
        let Some(conn) = self.session.as_ref() else { return };
        let Some(player) = self.player.as_ref() else { return };
        let _ = conn.call_method(
            Some(player.as_str()),
            "/org/mpris/MediaPlayer2",
            Some("org.mpris.MediaPlayer2.Player"),
            cmd,
            &(),
        );
    }

    /// MPRIS seek — jump the current track by `delta` microseconds.
    pub fn media_seek(&self, delta: i64) {
        let Some(conn) = self.session.as_ref() else { return };
        let Some(player) = self.player.as_ref() else { return };
        let _ = conn.call_method(
            Some(player.as_str()),
            "/org/mpris/MediaPlayer2",
            Some("org.mpris.MediaPlayer2.Player"),
            "Seek",
            &(delta,),
        );
    }

}

