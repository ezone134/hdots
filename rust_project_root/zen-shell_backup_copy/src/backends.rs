//! Backend layer — every system integration is a small trait with pluggable
//! implementations selected by the `[backends]` config table. Swapping an
//! implementation (sysfs → upower, wpctl → pipewire-dbus, command → native)
//! is a one-line config change, never a code rewrite.
//!
//! ```
//! [backends]
//! power      = "sysfs"    # sysfs | upower
//! brightness = "sysfs"    # sysfs
//! audio      = "wpctl"    # wpctl (PipeWire / WirePlumber)
//! lockscreen = "native"   # native | hyprlock
//! wallpaper  = "command"  # command (spawns the [wallpaper] command)
//! ```

use crate::shell::Shell;

// ------------------------------------------------------------------ power

/// Battery + AC status. `poll` fills the shell; `ac_changed` reports a plug
/// transition *since the last poll* (for the OSD) and consumes it, so each
/// plug/unplug fires the OSD exactly once.
pub trait Power {
    fn poll(&mut self, shell: &mut Shell) -> bool;
    fn ac_changed(&mut self) -> bool {
        false
    }
}

/// Direct `/sys/class/power_supply` reads — no daemon, no D-Bus.
pub struct SysfsPower {
    /// last seen AC state (None → first read; not a transition)
    ac_was: Option<bool>,
    /// plug transition pending delivery to the OSD
    ac_changed: bool,
}

impl SysfsPower {
    pub fn new() -> Self {
        SysfsPower { ac_was: None, ac_changed: false }
    }
}

/// First file under `/sys/class/power_supply` whose name starts with `prefix`,
/// containing `file` (e.g. `BAT0` + `capacity`).
fn sysfs_value(prefix: &str, file: &str) -> Option<u64> {
    let dir = std::path::Path::new("/sys/class/power_supply");
    let rd = std::fs::read_dir(dir).ok()?;
    for e in rd.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        if name.starts_with(prefix) {
            if let Ok(v) = std::fs::read_to_string(dir.join(&name).join(file)) {
                if let Ok(n) = v.trim().parse::<u64>() {
                    return Some(n);
                }
            }
        }
    }
    None
}

impl Power for SysfsPower {
    fn poll(&mut self, shell: &mut Shell) -> bool {
        // battery % (BAT0 / BAT1 / …); -1 when no battery present
        let pct = sysfs_value("BAT", "capacity").map(|v| v.min(100) as i32).unwrap_or(-1);
        // AC online (AC0 / AC / ADP1 / …); None → desktop without an adapter
        let ac = sysfs_value("AC", "online").or_else(|| sysfs_value("ADP", "online")).map(|v| v == 1);
        // power draw in W (µW → W); 0 when absent
        let watts = sysfs_value("BAT", "power_now").map(|u| u as f32 / 1_000_000.0).unwrap_or(0.0);
        // remaining time estimate
        let time_str = if pct >= 0 && watts > 0.01 {
            // prefer energy_now / power_now (µWh / µW = hours)
            let remaining = if let Some(energy_now) = sysfs_value("BAT", "energy_now") {
                let wh = energy_now as f32 / 1_000_000.0;
                wh / watts
            } else if let Some(charge_now) = sysfs_value("BAT", "charge_now") {
                // fallback: charge_now (µAh) × voltage / power → hours
                let voltage = sysfs_value("BAT", "voltage_now").unwrap_or(11_000_000) as f32 / 1_000_000.0;
                let ah = charge_now as f32 / 1_000.0;
                (ah * voltage) / watts
            } else {
                -1.0
            };
            if remaining >= 0.0 {
                let mins = (remaining * 60.0) as u64;
                let h = mins / 60;
                let m = mins % 60;
                if h > 0 {
                    format!("{h}h {m:02}m")
                } else {
                    format!("{m}m")
                }
            } else {
                String::new()
            }
        } else if pct >= 100 || ac == Some(true) {
            "Charged".into()
        } else {
            String::new()
        };
        let mut changed = false;
        if shell.battery != pct {
            shell.battery = pct;
            changed = true;
        }
        if (shell.battery_watts - watts).abs() > 0.01 {
            shell.battery_watts = watts;
            changed = true;
        }
        if shell.battery_time != time_str {
            shell.battery_time = time_str;
            changed = true;
        }
        if let Some(on) = ac {
            if shell.ac_online != on {
                shell.ac_online = on;
                changed = true;
            }
            // first read seeds the baseline; only later flips are transitions
            if let Some(prev) = self.ac_was {
                if prev != on {
                    self.ac_changed = true;
                }
            }
            self.ac_was = Some(on);
        }
        changed
    }

    fn ac_changed(&mut self) -> bool {
        let c = self.ac_changed;
        self.ac_changed = false;
        c
    }
}

/// upower over D-Bus (fallback backend; the battery info lives in a daemon).
pub struct UPowerPower {
    conn: Option<zbus::blocking::Connection>,
    ac_was: Option<bool>,
    ac_changed: bool,
}

impl UPowerPower {
    pub fn new() -> Self {
        UPowerPower {
            conn: zbus::blocking::Connection::system().ok(),
            ac_was: None,
            ac_changed: false,
        }
    }
}

impl Power for UPowerPower {
    fn poll(&mut self, shell: &mut Shell) -> bool {
        use zbus::zvariant::Value;
        let Some(conn) = self.conn.as_ref() else { return false };
        let path = conn
            .call_method(
                Some("org.freedesktop.UPower"),
                "/org/freedesktop/UPower",
                Some("org.freedesktop.UPower"),
                "GetDisplayDevice",
                &(),
            )
            .ok()
            .and_then(|m| m.body().deserialize::<zbus::zvariant::OwnedObjectPath>().ok());
        let Some(path) = path else { return false };
        let get = |name: &str| -> Option<zbus::zvariant::OwnedValue> {
            conn.call_method(
                Some("org.freedesktop.UPower"),
                path.as_str(),
                Some("org.freedesktop.UPower.Device"),
                "org.freedesktop.DBus.Properties.Get",
                &("org.freedesktop.UPower.Device", name),
            )
            .ok()
            .and_then(|m| m.body().deserialize::<zbus::zvariant::OwnedValue>().ok())
        };
        let num = |v: Option<zbus::zvariant::OwnedValue>| -> Option<f64> {
            let v = v?;
            match &*v {
                Value::F64(f) => Some(*f),
                Value::I32(i) => Some(*i as f64),
                Value::U32(u) => Some(*u as f64),
                _ => None,
            }
        };
        let pct = num(get("Percentage")).map(|p| p.round() as i32).unwrap_or(-1);
        let present = matches!(get("IsPresent"), Some(v) if matches!(&*v, Value::Bool(b) if *b));
        let watts = num(get("EnergyRate")).unwrap_or(0.0) as f32;
        let mut changed = false;
        if present && shell.battery != pct {
            shell.battery = pct;
            changed = true;
        } else if !present && shell.battery != -1 {
            shell.battery = -1;
            changed = true;
        }
        if shell.battery_watts != watts {
            shell.battery_watts = watts;
            changed = true;
        }
        // AC: OnBattery flips when the adapter is plugged / unplugged
        let on_battery = matches!(get("OnBattery"), Some(v) if matches!(&*v, Value::Bool(b) if *b));
        let ac = !on_battery;
        if let Some(prev) = self.ac_was {
            if prev != ac {
                self.ac_changed = true;
            }
        }
        self.ac_was = Some(ac);
        changed
    }

    fn ac_changed(&mut self) -> bool {
        let c = self.ac_changed;
        self.ac_changed = false;
        c
    }
}

// -------------------------------------------------------------- brightness

/// Display backlight control (0..=1).
pub trait Brightness {
    fn get(&mut self) -> Option<f32>;
    fn set(&mut self, v: f32);
}

/// `/sys/class/backlight/*/brightness` — a plain file write (permissions: root
/// or the `video` group).
pub struct SysfsBrightness {
    dir: Option<std::path::PathBuf>,
    max: f32,
}

impl SysfsBrightness {
    /// out_dir + max_brightness of the first matching backlight, if any.
    fn probe() -> (Option<std::path::PathBuf>, f32) {
        let dir = std::path::Path::new("/sys/class/backlight");
        let mut chosen = None;
        if let Ok(rd) = std::fs::read_dir(dir) {
            for e in rd.flatten() {
                let p = e.path();
                if p.join("brightness").exists() && p.join("max_brightness").exists() {
                    chosen = Some(p);
                    break;
                }
            }
        }
        let max = chosen
            .as_ref()
            .and_then(|p| std::fs::read_to_string(p.join("max_brightness")).ok())
            .and_then(|s| s.trim().parse::<f32>().ok())
            .unwrap_or(0.0);
        (chosen, max)
    }

    pub fn new() -> Self {
        let (dir, max) = Self::probe();
        SysfsBrightness { dir, max }
    }

    /// Probe whether this user can write the backlight sysfs file. Most
    /// laptops (brightnessctl included) route writes through logind instead,
    /// so a "yes" here means SysfsBrightness is safe to use for `set()`.
    pub fn writable(&self) -> bool {
        let Some(dir) = self.dir.as_ref() else { return false };
        let Some(max) = std::fs::read_to_string(dir.join("max_brightness")).ok() else {
            return false;
        };
        let max: u64 = max.trim().parse().ok().unwrap_or(0);
        if max == 0 {
            return false;
        }
        let probe = std::fs::read_to_string(dir.join("brightness")).ok();
        let Some(cur) = probe else { return false };
        let cur: u64 = cur.trim().parse().ok().unwrap_or(0);
        // write the same value back; ok() means we're allowed (polkit/ACL).
        std::fs::write(dir.join("brightness"), format!("{cur}\n")).is_ok()
    }

    /// The backlight device name /sys/class/backlight/<name>.
    pub fn device(&self) -> Option<String> {
        self.dir.as_ref()?.file_name().map(|s| s.to_string_lossy().into_owned())
    }

    /// Maximum backlight value; 0 if none.
    pub fn max(&self) -> f32 {
        self.max
    }
}

impl Brightness for SysfsBrightness {
    fn get(&mut self) -> Option<f32> {
        let dir = self.dir.as_ref()?;
        let cur: f32 = std::fs::read_to_string(dir.join("brightness")).ok()?.trim().parse().ok()?;
        Some((cur / self.max).clamp(0.0, 1.0))
    }

    fn set(&mut self, v: f32) {
        let Some(dir) = self.dir.as_ref() else { return };
        if self.max <= 0.0 {
            return;
        }
        let raw = (v.clamp(0.0, 1.0) * self.max).round().max(1.0) as u64;
        let _ = std::fs::write(dir.join("brightness"), format!("{raw}\n"));
    }
}

/// Backlight over systemd-logind D-Bus (`Session.SetBrightness`). Used when
/// the sysfs file isn't directly writable (common on Wayland laptops — even
/// brightnessctl sends this D-Bus call; polkit lets the active session set
/// brightness without root). Reads still come from sysfs (world-readable).
pub struct LogindBrightness {
    sysfs: SysfsBrightness,
    conn: Option<zbus::blocking::Connection>,
    session: String,
}

impl LogindBrightness {
    pub fn new() -> Self {
        let sysfs = SysfsBrightness::new();
        let conn = zbus::blocking::Connection::system().ok();
        // /org/freedesktop/login1/session/auto is logind's "resolve to the
        // calling process's session" alias — the same path brightnessctl uses.
        LogindBrightness {
            sysfs,
            conn,
            session: "/org/freedesktop/login1/session/auto".to_string(),
        }
    }
}

impl Brightness for LogindBrightness {
    fn get(&mut self) -> Option<f32> {
        self.sysfs.get()
    }

    fn set(&mut self, v: f32) {
        let max = self.sysfs.max();
        if max <= 0.0 {
            return;
        }
        let raw = (v.clamp(0.0, 1.0) * max).round().max(1.0) as u32;
        let Some(dev) = self.sysfs.device() else { return };
        let Some(conn) = self.conn.as_ref() else {
            // No logind — last-ditch direct sysfs write.
            if let Some(dir) = self.sysfs.dir.as_ref() {
                let _ = std::fs::write(dir.join("brightness"), format!("{raw}\n"));
            }
            return;
        };
        let _ = conn.call_method(
            Some("org.freedesktop.login1"),
            self.session.as_str(),
            Some("org.freedesktop.login1.Session"),
            "SetBrightness",
            &("backlight", dev.as_str(), raw),
        );
    }
}

// ------------------------------------------------------------------ audio

/// Per-app + default-stream audio control. The `wpctl` impl shells out to
/// WirePlumber's CLI (PipeWire); a future pipewire-dbus impl can slot in
/// without touching the UI.
pub trait Audio {
    fn poll(&mut self, shell: &mut Shell) -> bool;
    /// default sink volume + mute (for the OSD)
    fn default_volume(&mut self) -> Option<(f32, bool)>;
    fn set_default_volume(&self, vol: f32);
    /// set the default sink's per-channel L/R volumes in one wpctl call
    /// (live balance/pan drag: both values are known in the shell, so a
    /// single spawn replaces the bash read-modify-write per tick)
    fn set_channels(&self, left: f32, right: f32);
    fn set_volume(&self, id: u32, vol: f32);
    fn toggle_mute(&self, id: u32);
    /// make a sink/source the default (`wpctl set-default`)
    fn set_default(&self, id: u32);
}

pub struct WpctlAudio;

fn wpctl(args: &[&str]) -> Option<String> {
    std::process::Command::new("wpctl")
        .args(args)
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
}

fn wpctl_spawn(args: &[&str]) {
    let _ = std::process::Command::new("wpctl")
        .args(args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
}

fn pwcli_spawn(args: &[&str]) -> bool {
    std::process::Command::new("pw-cli")
        .args(args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .is_ok()
}

impl Audio for WpctlAudio {
    fn poll(&mut self, shell: &mut Shell) -> bool {
        let Some(text) = wpctl(&["status"]) else { return false };
        let (sinks, sources, streams) = parse_wpctl_status(&text);
        let mut changed = false;
        // default sink id — the AudioDevice card stars this row
        let default = default_sink_id_from_status(&text).unwrap_or(0);
        if default != shell.audio_default {
            shell.audio_default = default;
            changed = true;
        }
        // per-stream volume + mute (one get-volume call each; bounded so the
        // 3s poll stays cheap)
        let mut fixed: Vec<(u32, String, f32, bool, bool)> = Vec::new();
        for (id, name, is_input) in streams.into_iter().take(14) {
            let out = wpctl(&["get-volume", &id.to_string()]).unwrap_or_default();
            fixed.push((id, name, vol_of(&out).unwrap_or(1.0), muted_of(&out), is_input));
        }
        if sinks != shell.audio_sinks {
            shell.audio_sinks = sinks;
            changed = true;
        }
        if sources != shell.audio_sources {
            shell.audio_sources = sources;
            changed = true;
        }
        if fixed != shell.audio_streams {
            shell.audio_streams = fixed;
            changed = true;
        }
        changed
    }

    fn default_volume(&mut self) -> Option<(f32, bool)> {
        let out = wpctl(&["get-volume", "@DEFAULT_SINK@"])?;
        Some((vol_of(&out).unwrap_or(1.0), muted_of(&out)))
    }

    fn set_default_volume(&self, vol: f32) {
        wpctl_spawn(&["set-volume", "@DEFAULT_SINK@", &format!("{vol:.2}")]);
    }

    fn set_channels(&self, left: f32, right: f32) {
        // wpctl CANNOT set per-channel volume: its `set-volume ID VOL` takes
        // exactly one value, so a second positional fraction is silently
        // dropped and the balance slider just moved the overall volume to the
        // L value. Route the write through pw-cli's low-level set-param, which
        // targets the sink's Props { channelVolumes: [L, R] } directly.
        let Some(text) = wpctl(&["status"]) else { return };
        let Some(id) = default_sink_id_from_status(&text) else { return };
        let pod = format!("{{ channelVolumes = [ {left:.2} {right:.2} ] }}");
        if !pwcli_spawn(&["set-param", &id.to_string(), "Props", &pod]) {
            // no pw-cli — at least keep the overall volume moving with the mix
            wpctl_spawn(&["set-volume", &id.to_string(), &format!("{:.2}", (left + right) * 0.5)]);
        }
    }

    fn set_volume(&self, id: u32, vol: f32) {
        wpctl_spawn(&["set-volume", &id.to_string(), &format!("{vol:.2}")]);
    }

    fn toggle_mute(&self, id: u32) {
        wpctl_spawn(&["set-mute", &id.to_string(), "toggle"]);
    }

    fn set_default(&self, id: u32) {
        wpctl_spawn(&["set-default", &id.to_string()]);
    }
}

/// Parse `wpctl status` into (sinks, sources, streams) for the Audio panel.
/// Streams come back as `(id, name, is_input)` — direction is derived from
/// the stream's `input_*` sub-channel lines (wpctl 1.6 format).
pub fn parse_wpctl_status(
    text: &str,
) -> (Vec<(u32, String, f32, bool)>, Vec<(u32, String, f32, bool)>, Vec<(u32, String, bool)>) {
    let mut sinks: Vec<(u32, String, f32, bool)> = Vec::new();
    let mut sources: Vec<(u32, String, f32, bool)> = Vec::new();
    let mut streams: Vec<(u32, String, bool)> = Vec::new(); // (id, name, is_input)
    let mut section = "";
    let mut sub = "";
    let mut cur: Option<usize> = None; // index into `streams` awaiting direction
    for line in text.lines() {
        let t = line
            .trim_start_matches(|c: char| c == ' ' || "│├└─\t".contains(c))
            .trim();
        if t.is_empty() {
            continue;
        }
        if t == "Audio" || t == "Video" || t == "Settings" {
            section = t;
            sub = "";
            cur = None;
            continue;
        }
        if section != "Audio" {
            continue;
        }
        if let Some(s) = t.strip_suffix(':') {
            if matches!(s, "Sinks" | "Sources" | "Streams" | "Devices" | "Filters") {
                sub = s;
                cur = None;
                continue;
            }
        }
        let t = t.trim_start_matches('*').trim();
        let Some(dot) = t.find('.') else { continue };
        let id: u32 = match t[..dot].trim().parse() {
            Ok(id) => id,
            Err(_) => continue,
        };
        // strip trailing annotations ("[vol: 0.40]", "[alsa]", …)
        let name = t[dot + 1..].trim();
        let name = name.split(" [").next().unwrap_or(name).trim().to_string();
        // sub-channel lines belong to the stream parsed just before; the
        // input_ ones mark that stream as a recording (input) app
        if name.starts_with("output_") || name.starts_with("input_") || name.starts_with("monitor_") {
            if let Some(idx) = cur {
                if name.starts_with("input_") {
                    streams[idx].2 = true;
                }
            }
            continue;
        }
        match sub {
            "Sinks" => sinks.push((id, name, vol_of(&t).unwrap_or(1.0), muted_of(&t))),
            "Sources" => sources.push((id, name, vol_of(&t).unwrap_or(1.0), muted_of(&t))),
            "Streams" => {
                cur = Some(streams.len());
                streams.push((id, name, false));
            }
            _ => {}
        }
    }
    (sinks, sources, streams)
}

/// Extract the numeric node id of the default (starred) sink from `wpctl
/// status` — the id pw-cli needs for per-channel writes.
pub(crate) fn default_sink_id_from_status(text: &str) -> Option<u32> {
    let mut in_sinks = false;
    for line in text.lines() {
        let t = line
            .trim_start_matches(|c: char| c == ' ' || "│├└─\t".contains(c))
            .trim();
        if t.is_empty() {
            continue;
        }
        if t == "Sinks:" {
            in_sinks = true;
            continue;
        }
        if t.ends_with(':') {
            in_sinks = false;
            continue;
        }
        if !in_sinks {
            continue;
        }
        let Some(star) = t.strip_prefix('*') else { continue };
        let star = star.trim_start();
        let Some(dot) = star.find('.') else { continue };
        return star[..dot].trim().parse().ok();
    }
    None
}

/// Parse `[vol: 0.40]` (or `Volume: 0.33` / `[MUTED]`) out of a wpctl line.
fn vol_of(line: &str) -> Option<f32> {
    let s = line.find("[vol:").or_else(|| line.find("Volume:"))?;
    let rest = &line[s..];
    let rest = if let Some(r) = rest.strip_prefix("[vol:") {
        r
    } else {
        rest.strip_prefix("Volume:")?
    };
    let rest = rest.trim_start();
    let n: String = rest.chars().take_while(|c| c.is_ascii_digit() || *c == '.').collect();
    n.parse::<f32>().ok()
}

/// Is the line / get-volume output muted?
fn muted_of(line: &str) -> bool {
    line.contains("muted: yes") || line.contains("[MUTED]")
}

// --------------------------------------------------------------- wallpaper

/// Wallpaper setter. The `command` impl spawns the configured command (the
/// user's `set_wall` wrapper, which hands off to awww) with `{path}` /
/// `{monitor}` substituted.
pub trait Wallpaper {
    fn set(&self, path: &str, monitor: Option<&str>);
}

pub struct CommandWallpaper {
    command: String,
}

impl CommandWallpaper {
    pub fn new(command: String) -> Self {
        CommandWallpaper { command }
    }
}

impl Wallpaper for CommandWallpaper {
    fn set(&self, path: &str, monitor: Option<&str>) {
        // {path} / {monitor} are substituted into a `sh -c` string, so quote
        // them — a wallpaper named "mountain lake.jpg" must survive as one
        // argument.
        let mut cmd = self.command.replace("{path}", &sh_quote(path));
        if let Some(mon) = monitor {
            cmd = cmd.replace("{monitor}", &sh_quote(mon));
        }
        let _ = std::process::Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn();
    }
}

/// Single-quote `s` for `sh -c` (embedded quotes become `'\''`), so paths
/// with spaces or shell metacharacters arrive as one literal argument.
fn sh_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

// --------------------------------------------------------------- lockscreen

/// Lockscreen backend. `native` lets the shell morph itself into a fullscreen
/// lock UI (no external program); `hyprlock` spawns the external locker.
pub trait Lock {
    fn lock(&self);
    fn unlock(&self);
}

/// The shell's own lock UI (Mode::Lock) — no external binary involved.
pub struct NativeLock;

impl Lock for NativeLock {
    fn lock(&self) {}
    fn unlock(&self) {}
}

/// Spawn `hyprlock` (external backend; the compositor may also need the
/// `misc:close_special_on_empty`-style handling for fullscreen windows).
pub struct HyprlockLock;

impl Lock for HyprlockLock {
    fn lock(&self) {
        let _ = std::process::Command::new("hyprlock")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn();
    }
    fn unlock(&self) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_default_sink_star_line() {
        // The STATUS fixture above has the default sink as the starred first
        // "Sinks:" entry (id 49) — the id per-channel writes must target.
        assert_eq!(default_sink_id_from_status(STATUS), Some(49));
        assert_eq!(default_sink_id_from_status(""), None);
        assert_eq!(
            default_sink_id_from_status(" ├─ Sinks:\n │  *   49. Ryzen [vol: 0.40]\n"),
            Some(49)
        );
        assert_eq!(
            default_sink_id_from_status(" ├─ Sinks:\n │     60. Not default [vol: 1.00]\n"),
            None
        );
    }

    #[test]
    fn logind_set_brightness_roundtrip() {
        // Smoke test for the "brightness slider doesn't work" bug: the
        // backend was serializing the target value as u64 ('t') but logind's
        // SetBrightness takes u32 ('u'), so every call was rejected and
        // silently swallowed. Set a real value through the backend, then
        // confirm the sysfs control actually echoes it back.
        let mut lb = LogindBrightness::new();
        let max = lb.sysfs.max();
        assert!(max > 0.0, "no backlight device found");
        let dev = lb.sysfs.device().expect("backlight device");

        let before = lb.sysfs.get().expect("control read");
        let new = (before + 0.05).clamp(0.02, 0.98);
        lb.set(new);
        std::thread::sleep(std::time::Duration::from_millis(300));

        let after = lb.sysfs.get().expect("control read");
        assert!(
            (after - new).abs() < 0.02,
            "brightness did not move via logind: before={before:.2} wanted={new:.2} after={after:.2} (dev {dev})"
        );
        // restore so the panel doesn't jump while the suite runs
        lb.set(before);
    }

    const STATUS: &str = "\
PipeWire 'pipewire-0' [1.6.4, tw@tw, cookie:1133014575]\n\
 └─ Clients:\n\
        32. WirePlumber                         [1.6.4, tw@tw, pid:2794]\n\
\n\
Audio\n\
 ├─ Devices:\n\
 │      41. Renoir/Cezanne HDMI/DP Audio Controller [alsa]\n\
 ├─ Sinks:\n\
 │  *   49. Ryzen HD Audio Controller Analog Stereo [vol: 0.40]\n\
 ├─ Sources:\n\
 │  *   50. Ryzen HD Audio Controller Analog Stereo [vol: 1.00]\n\
 ├─ Filters:\n\
 └─ Streams:\n\
        69. paplay                                                    \n\
             67. output_FR       > ALC257 Analog:playback_FR\t[active]\n\
             71. output_FL       > ALC257 Analog:playback_FL\t[active]\n\
        69. parecord                                                  \n\
             67. monitor_FL     \n\
             68. input_FL        < ALC257 Analog:capture_FL\t[active]\n\
             71. monitor_FR     \n\
             72. input_FR        < ALC257 Analog:capture_FR\t[active]\n\
\n\
Video\n\
 ├─ Sources:\n\
 │  *   63. Integrated Camera (V4L2)           \n";

    #[test]
    fn parses_devices_and_stream_direction() {
        let (sinks, sources, streams) = parse_wpctl_status(STATUS);
        assert_eq!(sinks.len(), 1);
        assert_eq!(sinks[0].0, 49);
        assert_eq!(sinks[0].1, "Ryzen HD Audio Controller Analog Stereo");
        assert!((sinks[0].2 - 0.40).abs() < 0.01);
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].0, 50);
        assert_eq!(streams.len(), 2);
        assert_eq!(streams[0].0, 69);
        assert_eq!(streams[0].1, "paplay");
        assert!(!streams[0].2, "paplay is an output stream");
        assert_eq!(streams[1].1, "parecord");
        assert!(streams[1].2, "parecord is an input stream");
    }
}
