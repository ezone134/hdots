use super::super::*;
use std::net::Ipv4Addr;

/// journalctl tail throttle (s).
pub(crate) const JR_MS: u64 = 15_000;
/// max messages pulled per tail
pub(crate) const JR_MAX: usize = 30;
/// systemctl scan throttle (s).
pub(crate) const SUS_MS: u64 = 20_000;
/// SMART scan throttle (s). A scan runs one `smartctl -j -a` per disk; 30 s
/// keeps the card fresh on the dashboard without nagging the drives.
pub(crate) const SM_MIN_MS: u64 = 30_000;

impl Shell {
    /// Refresh the connection rows in-process. Returns true when the display
    /// data changed (caller re-renders).
    pub(crate) fn poll_conninfo(&mut self) -> bool {
        let mut rows: Vec<(String, String)> = Vec::new();
        for (iface, ip) in local_addresses().into_iter().take(8) {
            rows.push((iface, ip));
        }
        let gw = default_gateway();
        if !gw.is_empty() {
            rows.push(("gateway".into(), gw));
        }
        let dns = dns_servers();
        if !dns.is_empty() {
            rows.push(("dns".into(), dns.join(" ")));
        }
        if rows == self.conn_rows {
            return false;
        }
        self.conn_rows = rows;
        true
    }

    /// Pull the latest warning+ journal lines (`journalctl -o json -p
    /// warning`), throttled. Returns true when the tail changed.
    pub(crate) fn poll_journal(&mut self) -> bool {
        if let Some(next) = self.jr_next {
            if Instant::now() < next {
                return false;
            }
        }
        self.jr_next = Some(Instant::now() + std::time::Duration::from_millis(JR_MS));
        let mut lines: Vec<(String, u8)> = Vec::new();
        if let Ok(out) = std::process::Command::new("journalctl")
            .args(["-o", "json", "-p", "warning", "-n", "-30", "--no-pager"])
            .output()
        {
            for ln in String::from_utf8_lossy(&out.stdout).lines().take(JR_MAX) {
                let Ok(j) = serde_json::from_str::<serde_json::Value>(ln) else { continue };
                let msg = j["MESSAGE"].as_str().unwrap_or("").trim();
                if msg.is_empty() {
                    continue;
                }
                let prio = j["PRIORITY"].as_u64().unwrap_or(4) as u8;
                lines.push((msg.to_string(), prio));
            }
        }
        if lines != self.jr_lines {
            self.jr_lines = lines;
            true
        } else {
            false
        }
    }

    /// List failed units (system then user manager), throttled. Returns true
    /// when the list changed.
    pub(crate) fn poll_systemd(&mut self) -> bool {
        if let Some(next) = self.sus_next {
            if Instant::now() < next {
                return false;
            }
        }
        self.sus_next = Some(Instant::now() + std::time::Duration::from_millis(SUS_MS));
        let mut failed: Vec<String> = Vec::new();
        for extra in [Vec::<&str>::new(), vec!["--user"]] {
            let mut cmd = std::process::Command::new("systemctl");
            cmd.args(&extra);
            cmd.args([
                "list-units",
                "--type=service",
                "--state=failed",
                "--no-legend",
                "--no-pager",
                "-o",
                "json",
            ]);
            let Ok(out) = cmd.output() else { continue };
            let j: serde_json::Value =
                serde_json::from_slice(&out.stdout).unwrap_or(serde_json::Value::Null);
            if let Some(arr) = j.as_array() {
                for u in arr {
                    if let Some(name) = u["unit"].as_str() {
                        failed.push(name.to_string());
                    }
                }
            }
        }
        // dedupe across scopes, keep order
        let mut seen = std::collections::HashSet::new();
        failed.retain(|u| seen.insert(u.clone()));
        if failed != self.sus_failed {
            self.sus_failed = failed;
            true
        } else {
            false
        }
    }

    /// Scan disk SMART data (+ temp), throttled. Returns true when the list
    /// changed (caller re-renders).
    pub(crate) fn poll_smart(&mut self) -> bool {
        if let Some(next) = self.sm_next {
            if Instant::now() < next {
                return false;
            }
        }
        self.sm_next = Some(Instant::now() + std::time::Duration::from_millis(SM_MIN_MS));
        let mut disks: Vec<(String, String, bool, i32)> = Vec::new();
        if let Ok(blocks) = std::fs::read_dir("/sys/block") {
            let mut names: Vec<String> = Vec::new();
            for e in blocks.flatten() {
                let name = e.file_name().to_string_lossy().into_owned();
                if name.starts_with("sd") || name.starts_with("nvme") {
                    names.push(name);
                }
            }
            names.sort();
            for name in names.into_iter().take(4) {
                if let Some(row) = smart_row(&format!("/dev/{name}")) {
                    disks.push(row);
                }
            }
        }
        if disks != self.sm_disks {
            self.sm_disks = disks;
            true
        } else {
            false
        }
    }
}

/// One `smartctl -j -a` row: (dev path, model, health passed, temp °C).
pub(crate) fn smart_row(dev: &str) -> Option<(String, String, bool, i32)> {
    let out = std::process::Command::new("smartctl").args(["-j", "-a", dev]).output().ok()?;
    let j: serde_json::Value = serde_json::from_slice(&out.stdout).ok()?;
    let name = j["device"]["name"].as_str().unwrap_or("").to_string();
    let model = j["model_name"].as_str().unwrap_or("").to_string();
    let passed = j["smart_status"]["passed"].as_bool().unwrap_or(false);
    let temp = j["temperature"]["current"].as_i64().unwrap_or(0) as i32;
    if name.is_empty() && model.is_empty() {
        return None;
    }
    Some((name, model, passed, temp))
}

/// Up to `limit` non-loopback interface addresses as (iface, ip) strings.
fn local_addresses() -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    unsafe {
        let mut ifap: *mut libc::ifaddrs = std::ptr::null_mut();
        if libc::getifaddrs(&mut ifap) != 0 {
            return out;
        }
        let mut cur = ifap;
        while !cur.is_null() {
            let ifa = &*cur;
            if !ifa.ifa_addr.is_null() {
                let name = std::ffi::CStr::from_ptr(ifa.ifa_name).to_string_lossy().into_owned();
                let fam = (*ifa.ifa_addr).sa_family as i32;
                match fam {
                    libc::AF_INET if name != "lo" => {
                        let sin = &*(ifa.ifa_addr as *const libc::sockaddr_in);
                        let ip = Ipv4Addr::from(u32::from_be(sin.sin_addr.s_addr)).to_string();
                        out.push((name.clone(), ip));
                    }
                    libc::AF_INET6 if name != "lo" => {
                        let sin6 = &*(ifa.ifa_addr as *const libc::sockaddr_in6);
                        let ip = std::net::Ipv6Addr::from(sin6.sin6_addr.s6_addr).to_string();
                        out.push((name.clone(), ip));
                    }
                    _ => {}
                }
            }
            cur = ifa.ifa_next;
        }
        libc::freeifaddrs(ifap);
    }
    out
}

/// Default gateway from `/proc/net/route` — the row with dest `00000000`.
fn default_gateway() -> String {
    if let Ok(rt) = std::fs::read_to_string("/proc/net/route") {
        for line in rt.lines().skip(1) {
            let mut it = line.split_whitespace();
            let dev = it.next().unwrap_or("");
            let dest = it.next().unwrap_or("");
            let gate = it.next().unwrap_or("");
            if dest == "00000000" && gate != "00000000" {
                if let Ok(g) = u32::from_str_radix(gate, 16) {
                    return format!("{} via {}", Ipv4Addr::from(g), dev);
                }
            }
        }
    }
    String::new()
}

/// DNS resolvers from `/etc/resolv.conf` (first three).
fn dns_servers() -> Vec<String> {
    let mut dns: Vec<String> = Vec::new();
    if let Ok(c) = std::fs::read_to_string("/etc/resolv.conf") {
        for line in c.lines() {
            let t = line.trim();
            if let Some(ns) = t.strip_prefix("nameserver") {
                let ns = ns.trim();
                if !ns.is_empty() && !ns.starts_with('#') {
                    dns.push(ns.to_string());
                }
            }
            if dns.len() >= 3 {
                break;
            }
        }
    }
    dns
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shell() -> Shell {
        let cfg: crate::config::Config =
            toml::from_str(crate::config::DEFAULT_SHELL_TOML).expect("default config parses");
        Shell::new(cfg)
    }

    #[test]
    fn poll_builds_rows_idempotently() {
        let mut s = shell();
        let _ = s.poll_conninfo();
        let snapshot = s.conn_rows.clone();
        let again = s.poll_conninfo();
        assert!(!again, "no change between identical polls");
        assert_eq!(s.conn_rows, snapshot);
    }

    #[test]
    fn journal_polling_is_throttled() {
        let mut s = shell();
        let _ = s.poll_journal();
        assert!(!s.poll_journal(), "second call within window is a no-op");
    }

    #[test]
    fn systemd_polling_is_throttled() {
        let mut s = shell();
        let _ = s.poll_systemd();
        assert!(!s.poll_systemd(), "second call within window is a no-op");
    }

    #[test]
    fn smart_polling_is_throttled() {
        let mut s = shell();
        let _ = s.poll_smart();
        assert!(!s.poll_smart(), "second call within window is a no-op");
    }

    #[test]
    fn smart_row_never_panics() {
        // missing smartctl / non-existent device → helper returns quietly
        let _ = smart_row("/dev/nonexistent0");
    }
}
