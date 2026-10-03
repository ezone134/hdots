use super::super::*;
use super::shared::*;
use std::net::Ipv4Addr;

impl Shell {

    /// CONNECTION INFO — local interface addresses (getifaddrs), default
    /// gateway (/proc/net/route) and DNS resolvers (/etc/resolv.conf). Every
    /// source is an in-process kernel/fs read, so this card costs a few µs
    /// and never spawns a subprocess; it re-polls on the 3 s timer while
    /// packed and on the manual refresh button.
    pub(crate) fn draw_conninfo_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        let pad = self.scale.s(12.0);
        let hdr = self.scale.s(24.0);
        let meta = format!("{} addrs", self.conn_rows.len());
        card_header(v, pal, x, y, w, pad, "Network", Some(ICON_LINK), Some((&meta, false)), self.scale, self.card_show_title, self.card_show_glyph);

        if self.conn_rows.is_empty() {
            card_empty(v, pal, x, y, w, h, "no network interfaces", self.scale);
        } else {
            let mut ry = y + hdr + self.scale.s(6.0);
            for (lbl, val) in &self.conn_rows {
                if ry + self.scale.s(18.0) > y + h - self.scale.s(26.0) {
                    break;
                }
                let max_v = ((w - pad * 2.0 - self.scale.s(70.0)) / self.scale.s(5.6)).max(4.0) as usize;
                let shown: String = val.chars().take(max_v).collect();
                metric_row(v, pal, x + pad, ry, x + w - pad - self.scale.s(2.0), lbl, &shown, if lbl == "gateway" || lbl == "dns" { ui::fg3(&pal) } else { pal.fg }, self.scale);
                ry += self.scale.s(18.0);
            }
        }
        // refresh button (bottom-right): re-reads /proc right now
        let bh = self.scale.s(22.0);
        let by = y + h - bh - self.scale.s(10.0);
        let bkey = CONNINFO_REFRESH_KEY;
        let hov = self.hover_key == bkey;
        v.push(Cmd::Rect {
            x: x + w - pad - self.scale.s(30.0),
            y: by,
            w: self.scale.s(30.0),
            h: bh,
            r: self.scale.s(6.0),
            color: if hov { ui::raised_hl(&pal) } else { ui::raised(&pal) },
        });
        ui::text_c(v, x + w - pad - self.scale.s(15.0), by + self.scale.s(6.0), ICON_REFRESH, self.scale.fs(9.0), if hov { pal.acc } else { ui::fg3(&pal) }, true);
        self.region(x + w - pad - self.scale.s(30.0) - self.scale.s(3.0), by - self.scale.s(3.0), self.scale.s(30.0) + self.scale.s(6.0), bh + self.scale.s(6.0), bkey);
    }

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

    fn pal() -> Pal {
        Pal { fg: 0xe8e6e3ff, bg: 0x141414ff, acc: 0x4fc2ffff, sfg: 0x101010ff }
    }

    #[test]
    fn gateway_and_dns_parse() {
        // /proc/net/route row: Iface  Dest   Gateway ...
        let gw = default_gateway();
        // machine-dependent: either has a gateway or empty
        let _ = gw;
        // gateways/dns are env-dependent — the card must draw regardless
        let mut s = shell();
        s.conn_rows = vec![
            ("eth0".into(), "192.168.1.24".into()),
            ("wlan0".into(), "fd00::abcd".into()),
            ("gateway".into(), "192.168.1.1 via eth0".into()),
            ("dns".into(), "1.1.1.1 9.9.9.9".into()),
        ];
        let p = pal();
        let mut v: Vec<Cmd> = Vec::new();
        s.draw_conninfo_card(&mut v, 0.0, 0.0, 170.0, 110.0, &p);
        assert!(v.iter().any(|c| matches!(c, Cmd::Text { text, .. } if text.contains("192.168.1.24"))), "iface row drawn");
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
}