//! System stats — CPU %, RAM %, disk %, GPU % and net up/down rates.
//!
//! Pure `/proc` + `statvfs` reads (no deps, sub-ms), polled on the existing 3s
//! services timer. CPU and net are deltas between polls. The caller gates
//! `Stats::poll` to the dashboard / settings, where the numbers are shown;
//! in every other mode the /proc reads sleep.

use crate::shell::Shell;
use std::path::Path;
use std::sync::OnceLock;

pub struct Stats {
    last_idle: u64,
    last_total: u64,
    last_rx: u64,
    last_tx: u64,
    /// default-route interface name (cached)
    iface: Option<String>,
    /// cumulative sectors read/written across every block device
    last_disk_r: u64,
    last_disk_w: u64,
    /// last pushed read/write bytes-per-second, for change detection
    last_disk_bps: (u64, u64),
}

/// Where GPU busy % comes from — detected once, then cached for the
/// process lifetime. amdgpu exposes a free sysfs node; NVIDIA needs the
/// `nvidia-smi` subprocess.
enum GpuSrc {
    /// path to a `gpu_busy_percent` sysfs node (amdgpu / panfrost / …)
    Sysfs(std::path::PathBuf),
    Nvidia,
}

fn gpu_src() -> &'static Option<GpuSrc> {
    static S: OnceLock<Option<GpuSrc>> = OnceLock::new();
    S.get_or_init(|| {
        // prefer a sysfs node: /sys/class/drm/cardN/device/gpu_busy_percent
        if let Ok(rd) = std::fs::read_dir("/sys/class/drm") {
            let mut cards: Vec<_> = rd.flatten().collect();
            cards.sort_by_key(|e| e.file_name());
            for e in cards {
                let name = e.file_name();
                let name = name.to_string_lossy();
                if !name.starts_with("card") {
                    continue;
                }
                let p = e.path().join("device/gpu_busy_percent");
                if p.exists() {
                    return Some(GpuSrc::Sysfs(p));
                }
            }
        }
        // fallback: NVIDIA blob
        if std::process::Command::new("nvidia-smi")
            .arg("-L")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
        {
            return Some(GpuSrc::Nvidia);
        }
        None
    })
}

/// GPU busy percentage (0..=100), or None when no source exists.
fn gpu_usage() -> Option<i32> {
    match gpu_src() {
        Some(GpuSrc::Sysfs(p)) => std::fs::read_to_string(p).ok()?.trim().parse().ok(),
        Some(GpuSrc::Nvidia) => {
            let o = std::process::Command::new("nvidia-smi")
                .args(["--query-gpu=utilization.gpu", "--format=csv,noheader,nounits"])
                .output()
                .ok()?;
            String::from_utf8_lossy(&o.stdout).trim().parse().ok()
        }
        None => None,
    }
}

/// GPU temperature in °C from the same detected source (amdgpu hwmon node
/// or nvidia-smi). 0 when the source exposes none.
fn gpu_temp_c() -> f32 {
    let Some(src) = gpu_src() else { return 0.0 };
    match src {
        GpuSrc::Sysfs(p) => {
            // p = …/device/gpu_busy_percent → scan …/device/hwmon/*/temp1_input
            let Some(devdir) = p.parent() else { return 0.0 };
            let Ok(rd) = std::fs::read_dir(devdir.join("hwmon")) else {
                return 0.0;
            };
            for e in rd.flatten() {
                let Ok(s) = std::fs::read_to_string(e.path().join("temp1_input")) else {
                    continue;
                };
                if let Ok(milli_c) = s.trim().parse::<f32>() {
                    let c = milli_c / 1000.0;
                    if c > 0.0 && c < 200.0 {
                        return c;
                    }
                }
            }
            0.0
        }
        GpuSrc::Nvidia => {
            let o = std::process::Command::new("nvidia-smi")
                .args(["--query-gpu=temperature.gpu", "--format=csv,noheader,nounits"])
                .output();
            let Ok(o) = o else { return 0.0 };
            String::from_utf8_lossy(&o.stdout).trim().parse().ok().unwrap_or(0.0)
        }
    }
}

/// GPU VRAM used/total in MiB from the detected source. None when the
/// source exposes none (total stays 0 → card hides the VRAM row).
fn gpu_vram_mb() -> Option<(u64, u64)> {
    let Some(src) = gpu_src() else { return None };
    let parse_pair = |s: &str| -> Option<(u64, u64)> {
        let mut it = s.split(',').map(|v| v.trim().parse::<u64>().ok());
        Some((it.next()??, it.next()??))
    };
    match src {
        GpuSrc::Sysfs(p) => {
            let devdir = p.parent()?;
            let u = std::fs::read_to_string(devdir.join("mem_info_vram_used")).ok()?;
            let t = std::fs::read_to_string(devdir.join("mem_info_vram_total")).ok()?;
            let mb = |b: u64| b / (1024 * 1024);
            let (u, t) = (u.trim().parse().ok()?, t.trim().parse().ok()?);
            if t == 0 { return None; }
            Some((mb(u), mb(t)))
        }
        GpuSrc::Nvidia => {
            let o = std::process::Command::new("nvidia-smi")
                .args(["--query-gpu=memory.used,memory.total", "--format=csv,noheader,nounits"])
                .output()
                .ok()?;
            parse_pair(&String::from_utf8_lossy(&o.stdout))
        }
    }
}

/// All hwmon thermal zones as `(type, °C)` — e.g. `("x86_pkg_temp", 81.0)`.
/// Zones above 200 °C are bogus and skipped. Named in directory order.
/// Path of the persistent sensor map (`$states2/thermal_map.txt`): one line
/// per sensor, `label<TAB>temp_absolute_path`. Built once by a directory
/// scan (see [`scan_thermal_sensors`]) and reused on later polls and across
/// restarts so the per-tick work is a batch of tiny file reads instead of
/// repeated directory traversals.
fn thermal_map_path() -> std::path::PathBuf {
    crate::vars::states2_dir().join("thermal_map.txt")
}

/// Scan `/sys/class/thermal` (ACPI zones) and every `/sys/class/hwmon` so
/// SoC / NVMe / GPU sensors that are not re-exported as thermal zones are
/// covered too. Label = the zone `type` or the hwmon `temp*_label`; exact
/// duplicate temp paths are dropped (hwmon re-exports of a zone).
fn scan_thermal_sensors() -> Vec<(String, String)> {
    let mut map: Vec<(String, String)> = Vec::new();
    // ACPI / kernel thermal zones
    if let Ok(rd) = std::fs::read_dir("/sys/class/thermal") {
        for e in rd.flatten() {
            let name = e.file_name();
            let name = name.to_string_lossy();
            if !name.starts_with("thermal_zone") {
                continue;
            }
            let typ = std::fs::read_to_string(e.path().join("type"))
                .map(|t| t.trim().to_string())
                .unwrap_or_default();
            let tp = e.path().join("temp");
            if !typ.is_empty() && tp.exists() {
                map.push((typ, tp.to_string_lossy().into_owned()));
            }
        }
    }
    // raw hwmon sensors (nvme, GPU VRM, soc, …)
    if let Ok(rd) = std::fs::read_dir("/sys/class/hwmon") {
        let mut hwmons: Vec<_> = rd.flatten().collect();
        hwmons.sort_by_key(|e| e.file_name());
        for e in hwmons {
            let dir = e.path();
            let hname = std::fs::read_to_string(dir.join("name"))
                .map(|s| s.trim().to_string())
                .unwrap_or_default();
            let mut temps: Vec<String> = Vec::new();
            if let Ok(tr) = std::fs::read_dir(&dir) {
                temps = tr
                    .flatten()
                    .map(|f| f.file_name())
                    .filter_map(|n| {
                        let n = n.to_string_lossy().into_owned();
                        (n.starts_with("temp") && n.ends_with("_input")).then_some(n)
                    })
                    .collect();
                temps.sort();
            }
            for t in temps {
                let num = t.trim_start_matches("temp").trim_end_matches("_input");
                let tp = dir.join(&t);
                let tps = tp.to_string_lossy().into_owned();
                if map.iter().any(|(_, p)| *p == tps) {
                    continue; // hwmon re-export of an already-mapped zone
                }
                let label = std::fs::read_to_string(dir.join(format!("temp{num}_label")))
                    .map(|s| s.trim().to_string())
                    .unwrap_or_else(|_| format!("{} temp{num}", hname));
                let label = if hname.is_empty() {
                    label
                } else if label.contains(&hname) {
                    label
                } else {
                    format!("{hname} {label}")
                };
                map.push((label, tps));
            }
        }
    }
    map
}

/// `(label, temp_path)` sensor map, cached in `$states2/thermal_map.txt`.
/// Reads the persisted file when present (no scan), else discovers the
/// sensors once, persists the map, and returns it.
fn thermal_sensor_map() -> Vec<(String, String)> {
    if let Ok(s) = std::fs::read_to_string(thermal_map_path()) {
        let map: Vec<(String, String)> = s
            .lines()
            .filter_map(|l| {
                let (label, path) = l.split_once('\t')?;
                Some((label.to_string(), path.to_string()))
            })
            .collect();
        if !map.is_empty() {
            return map;
        }
    }
    let map = scan_thermal_sensors();
    let text = map
        .iter()
        .map(|(l, p)| format!("{l}\t{p}"))
        .collect::<Vec<_>>()
        .join("\n");
    let _ = std::fs::create_dir_all(crate::vars::states2_dir());
    let _ = std::fs::write(thermal_map_path(), text);
    map
}

/// Read every 100 m°C sensor from the persisted map. When the map is stale
/// (every sensor gone — device unplugged) it is re-scanned and persisted.
fn thermal_zones() -> Vec<(String, f32)> {
    let map = thermal_sensor_map();
    let mut out: Vec<(String, f32)> = Vec::new();
    for (label, tpath) in &map {
        let Ok(s) = std::fs::read_to_string(tpath) else { continue };
        let Ok(milli_c) = s.trim().parse::<f32>() else { continue };
        let c = milli_c / 1000.0;
        if c > 0.0 && c < 200.0 {
            out.push((label.clone(), c));
        }
    }
    if !out.is_empty() {
        return out;
    }
    // stale map — rediscover and rewrite
    let fresh = scan_thermal_sensors();
    let text = fresh
        .iter()
        .map(|(l, p)| format!("{l}\t{p}"))
        .collect::<Vec<_>>()
        .join("\n");
    let _ = std::fs::write(thermal_map_path(), text);
    for (label, tpath) in &fresh {
        let Ok(s) = std::fs::read_to_string(tpath) else { continue };
        let Ok(milli_c) = s.trim().parse::<f32>() else { continue };
        let c = milli_c / 1000.0;
        if c > 0.0 && c < 200.0 {
            out.push((label.clone(), c));
        }
    }
    out
}

/// Live fan speeds: every hwmon `fanN_input` (RPM) across all chips, as
/// `("fanN (chip)", rpm)`. Zero-RPM (stopped) fans are still listed — the
/// card dims them.
fn fan_rpms() -> Vec<(String, u32)> {
    let mut out: Vec<(String, u32)> = Vec::new();
    let Ok(chips) = std::fs::read_dir("/sys/class/hwmon") else { return out };
    let mut chips: Vec<_> = chips.flatten().collect();
    chips.sort_by_key(|e| e.file_name());
    for chip in chips {
        let chip_name = std::fs::read_to_string(chip.path().join("name"))
            .unwrap_or_else(|_| chip.file_name().to_string_lossy().to_string())
            .trim()
            .to_string();
        let Ok(rd) = std::fs::read_dir(chip.path()) else { continue };
        let mut entries: Vec<_> = rd.flatten().collect();
        entries.sort_by_key(|e| e.file_name());
        for e in entries {
            let fname = e.file_name();
            let fname = fname.to_string_lossy();
            if !fname.starts_with("fan") || !fname.ends_with("_input") {
                continue;
            }
            if let Ok(s) = std::fs::read_to_string(e.path()) {
                if let Ok(rpm) = s.trim().parse::<u32>() {
                    let idx = fname.trim_start_matches("fan").trim_end_matches("_input");
                    out.push((format!("fan{idx} ({chip_name})"), rpm));
                }
            }
        }
    }
    out
}

/// Current CPU frequency in MHz — the max `scaling_cur_freq` across online
/// cores (`/sys/devices/system/cpu/cpuN/cpufreq/…`, kHz), with a
/// `cpuinfo_cur_freq` fallback. 0 when no cpufreq driver exposes it.
fn cpu_freq_mhz() -> u64 {
    let mut best = 0u64;
    if let Ok(rd) = std::fs::read_dir("/sys/devices/system/cpu") {
        let mut cpus: Vec<_> = rd.flatten().collect();
        cpus.sort_by_key(|e| e.file_name());
        for e in cpus {
            let name = e.file_name();
            let name = name.to_string_lossy();
            if !name.starts_with("cpu") || !name[3..].chars().all(|c| c.is_ascii_digit()) {
                continue;
            }
            let base = e.path().join("cpufreq");
            for f in ["scaling_cur_freq", "cpuinfo_cur_freq"] {
                let p = base.join(f);
                if let Ok(s) = std::fs::read_to_string(p) {
                    if let Ok(khz) = s.trim().parse::<u64>() {
                        best = best.max(khz / 1000);
                        break;
                    }
                }
            }
        }
    }
    best
}

/// CPU package temperature in °C — the max over `/sys/class/thermal/
/// thermal_zone*/temp` (standard ABI: millidegrees). 0 when unavailable.
fn thermal_max_c() -> f32 {
    let Ok(rd) = std::fs::read_dir("/sys/class/thermal") else {
        return 0.0;
    };
    let mut best = 0.0f32;
    for e in rd.flatten() {
        let name = e.file_name();
        let name = name.to_string_lossy();
        if !name.starts_with("thermal_zone") {
            continue;
        }
        let Ok(s) = std::fs::read_to_string(e.path().join("temp")) else {
            continue;
        };
        if let Ok(milli_c) = s.trim().parse::<f32>() {
            let c = milli_c / 1000.0;
            if c > best && c < 200.0 {
                best = c;
            }
        }
    }
    best
}

impl Stats {
    pub fn new() -> Self {
        Stats {
            last_idle: 0,
            last_total: 0,
            last_rx: 0,
            last_tx: 0,
            iface: None,
            last_disk_r: 0,
            last_disk_w: 0,
            last_disk_bps: (u64::MAX, u64::MAX),
        }
    }

    /// Refresh stats into `shell`. Returns true if anything changed.
    pub fn poll(&mut self, shell: &mut Shell) -> bool {
        let mut changed = false;
        if let Some((cpu, mem, disk)) = self.cpu_mem_disk() {
            if cpu != shell.cpu || mem != shell.mem_pct || disk != shell.disk_pct {
                shell.cpu = cpu;
                shell.mem_pct = mem;
                shell.disk_pct = disk;
                changed = true;
            }
        }
        // ── memory detail (mem card) — total/used/available/cached/free ──
        if let Ok(mi) = std::fs::read_to_string("/proc/meminfo") {
            let kb = |key: &str| -> u64 {
                mi.lines().find_map(|l| {
                    let mut it = l.split_whitespace();
                    if it.next() == Some(key) {
                        it.next().and_then(|v| v.parse().ok())
                    } else {
                        None
                    }
                }).unwrap_or(0)
            };
            let total = kb("MemTotal:");
            let avail = kb("MemAvailable:");
            let gb = |k: u64| (k as f32 / 1_048_576.0 * 10.0).round() / 10.0;
            // used mirrors mem_pct (total − available, the way tools show it)
            let (tg, ug, ag, cg, fg) =
                (gb(total), gb(total.saturating_sub(avail)), gb(avail), gb(kb("Cached:")), gb(kb("MemFree:")));
            if tg != shell.mem_total_gb
                || ug != shell.mem_used_gb
                || ag != shell.mem_avail_gb
                || cg != shell.mem_cached_gb
                || fg != shell.mem_free_gb
            {
                shell.mem_total_gb = tg;
                shell.mem_used_gb = ug;
                shell.mem_avail_gb = ag;
                shell.mem_cached_gb = cg;
                shell.mem_free_gb = fg;
                changed = true;
            }
        }
        // ── CPU detail (cpu card) — load avg / freq / temp ──
        if let Ok(la) = std::fs::read_to_string("/proc/loadavg") {
            let mut it = la.split_whitespace().filter_map(|v| v.parse::<f32>().ok());
            let t = (it.next().unwrap_or(0.0), it.next().unwrap_or(0.0), it.next().unwrap_or(0.0));
            if t != shell.cpu_load {
                shell.cpu_load = t;
                changed = true;
            }
        }
        let mhz = cpu_freq_mhz();
        if mhz != shell.cpu_freq_mhz {
            shell.cpu_freq_mhz = mhz;
            changed = true;
        }
        let tc = thermal_max_c();
        if (tc - shell.cpu_temp).abs() > 0.05 {
            shell.cpu_temp = tc;
            changed = true;
        }
        // ── disk detail (gauges card) — / used + total, decimal GB ──
        let mut st: libc::statvfs = unsafe { std::mem::zeroed() };
        if unsafe { libc::statvfs(b"/\0".as_ptr() as *const libc::c_char, &mut st) } == 0 {
            let gb = |b: u64| (b as f32 / 1_000_000_000.0 * 10.0).round() / 10.0;
            let (tg, ug) = (
                gb(st.f_blocks as u64 * st.f_frsize as u64),
                gb((st.f_blocks - st.f_bavail) as u64 * st.f_frsize as u64),
            );
            if tg != shell.disk_total_gb || ug != shell.disk_used_gb {
                shell.disk_total_gb = tg;
                shell.disk_used_gb = ug;
                changed = true;
            }
        }
        // GPU: -1 when no source; the chart hides the GPU line then
        let gpu = gpu_usage().unwrap_or(-1);
        if gpu != shell.gpu {
            shell.gpu = gpu;
            changed = true;
        }
        // GPU detail (gpu card) — temperature + VRAM from the same source
        let gt = gpu_temp_c();
        if (gt - shell.gpu_temp).abs() > 0.05 {
            shell.gpu_temp = gt;
            changed = true;
        }
        if let Some((u, t)) = gpu_vram_mb() {
            if u != shell.gpu_vram_used_mb || t != shell.gpu_vram_total_mb {
                shell.gpu_vram_used_mb = u;
                shell.gpu_vram_total_mb = t;
                changed = true;
            }
        }
        // thermal zones (thermal card) — copy-on-change keeps draw cheap
        let tz = thermal_zones();
        if tz != shell.thermal_zones {
            shell.thermal_zones = tz;
            changed = true;
        }
        // fans (fans card) — hwmon fan*_input RPMs; label "fanN (chip)"
        let fans = fan_rpms();
        if fans != shell.fans_rpm {
            shell.fans_rpm = fans;
            changed = true;
        }
        // slow-decay per-fan peaks so bars show relative spin (never decays
        // below the current rpm → bars never exceed 1.0)
        shell.fan_peaks.resize(shell.fans_rpm.len(), 1.0);
        for (i, (_, rpm)) in shell.fans_rpm.iter().enumerate() {
            let cur = shell.fan_peaks[i];
            let target = *rpm as f32;
            let next = if target >= cur { target } else { cur * 0.995 + target * 0.005 };
            shell.fan_peaks[i] = next.max(target).max(1.0);
        }
        // load histories for the dashboard's dual-line CPU/GPU chart —
        // pushed every poll so the line scrolls even at constant load
        shell.cpu_history.push_back(shell.cpu.clamp(0, 100) as f32 / 100.0);
        if shell.cpu_history.len() > 48 {
            shell.cpu_history.pop_front();
        }
        shell.gpu_history.push_back(shell.gpu.max(0).clamp(0, 100) as f32 / 100.0);
        if shell.gpu_history.len() > 48 {
            shell.gpu_history.pop_front();
        }
        if let Some((up, down, rx_bps, tx_bps)) = self.net_rates() {
            if up != shell.net_up || down != shell.net_down {
                shell.net_up = up;
                shell.net_down = down;
                changed = true;
            }
            shell.net_history.push_back((rx_bps, tx_bps));
            if shell.net_history.len() > 40 {
                shell.net_history.pop_front();
            }
        }
        // ── disk I/O — read/write bytes/s summed across all /sys/block/* ──
        if let Some((r_bps, w_bps)) = self.disk_rates() {
            shell.disk_history.push_back((r_bps, w_bps));
            if shell.disk_history.len() > 48 {
                shell.disk_history.pop_front();
            }
            if (r_bps, w_bps) != self.last_disk_bps {
                self.last_disk_bps = (r_bps, w_bps);
                changed = true;
            }
        }
        changed
    }

    fn cpu_mem_disk(&mut self) -> Option<(i32, i32, i32)> {
        // CPU: /proc/stat first line — user nice system idle iowait irq softirq steal
        let stat = std::fs::read_to_string("/proc/stat").ok()?;
        let line = stat.lines().next()?;
        let vals: Vec<u64> = line
            .split_whitespace()
            .skip(1)
            .filter_map(|s| s.parse().ok())
            .collect();
        let idle = vals.get(3).copied().unwrap_or(0) + vals.get(4).copied().unwrap_or(0);
        let total: u64 = vals.iter().sum();
        let mut cpu = 0;
        if self.last_total != 0 && total > self.last_total {
            let d_total = total - self.last_total;
            let d_idle = idle.saturating_sub(self.last_idle);
            cpu = (100 - d_idle * 100 / d_total) as i32;
        }
        self.last_idle = idle;
        self.last_total = total;

        // RAM: MemTotal / MemAvailable
        let meminfo = std::fs::read_to_string("/proc/meminfo").ok()?;
        let mut total_kb = 0u64;
        let mut avail_kb = 0u64;
        for l in meminfo.lines() {
            let mut it = l.split_whitespace();
            let key = it.next().unwrap_or("");
            let val: u64 = it.next().and_then(|v| v.parse().ok()).unwrap_or(0);
            match key {
                "MemTotal:" => total_kb = val,
                "MemAvailable:" => avail_kb = val,
                _ => {}
            }
        }
        let mem = if total_kb > 0 {
            (100 - avail_kb * 100 / total_kb) as i32
        } else {
            0
        };

        // Disk: statvfs("/") — used = total - available (bavail, non-root view)
        let mut st: libc::statvfs = unsafe { std::mem::zeroed() };
        let disk = if unsafe { libc::statvfs(b"/\0".as_ptr() as *const libc::c_char, &mut st) } == 0
        {
            let total = st.f_blocks as u64 * st.f_frsize as u64;
            let avail = st.f_bavail as u64 * st.f_frsize as u64;
            if total > 0 {
                (100 - avail * 100 / total) as i32
            } else {
                0
            }
        } else {
            0
        };
        Some((cpu, mem, disk))
    }

    /// Default-route interface from /proc/net/route (dest 00000000).
    fn default_iface(&mut self) -> Option<String> {
        if let Some(i) = &self.iface {
            if Path::new(&format!("/sys/class/net/{i}/statistics/rx_bytes")).exists() {
                return Some(i.clone());
            }
        }
        let route = std::fs::read_to_string("/proc/net/route").ok()?;
        for l in route.lines().skip(1) {
            let mut it = l.split_whitespace();
            let iface = it.next()?;
            let dest = it.next().unwrap_or("");
            if dest == "00000000" && iface != "lo" {
                self.iface = Some(iface.to_string());
                return self.iface.clone();
            }
        }
        None
    }

    fn net_rates(&mut self) -> Option<(String, String, u64, u64)> {
        let iface = self.default_iface()?;
        let rx = std::fs::read_to_string(format!("/sys/class/net/{iface}/statistics/rx_bytes"))
            .ok()?
            .trim()
            .parse::<u64>()
            .ok()?;
        let tx = std::fs::read_to_string(format!("/sys/class/net/{iface}/statistics/tx_bytes"))
            .ok()?
            .trim()
            .parse::<u64>()
            .ok()?;
        let (up, down, rx_bps, tx_bps) = if self.last_rx != 0 {
            // poll cadence is 2s; deltas per second
            let d = rx.saturating_sub(self.last_rx) / 2;
            let u = tx.saturating_sub(self.last_tx) / 2;
            (fmt_rate(u), fmt_rate(d), u, d)
        } else {
            ("0 Kbps".to_string(), "0 Kbps".to_string(), 0, 0)
        };
        self.last_rx = rx;
        self.last_tx = tx;
        Some((up, down, rx_bps, tx_bps))
    }

    /// Whole-system disk I/O as bytes-per-second, summed across every
    /// `/sys/block/*` device (sectors read/written, 512 B each). Pure procfs
    /// counters — no subprocess. Poll cadence is 2 s → per-second deltas.
    fn disk_rates(&mut self) -> Option<(u64, u64)> {
        let mut r = 0u64;
        let mut w = 0u64;
        for e in std::fs::read_dir("/sys/block").ok()?.flatten() {
            let s = std::fs::read_to_string(e.path().join("stat")).ok()?;
            let mut it = s.split_whitespace();
            let _reads = it.next()?; // reads completed
            let _merged = it.next()?; // reads merged
            let sec_r: u64 = it.next()?.parse().ok()?; // sectors read
            let _ = it.nth(1); // time reading, writes completed
            let _merged_w = it.next()?; // writes merged
            let sec_w: u64 = it.next()?.parse().ok()?; // sectors written
            r = r.saturating_add(sec_r);
            w = w.saturating_add(sec_w);
        }
        if self.last_disk_r != 0 {
            let dr = r.saturating_sub(self.last_disk_r) * 512 / 2;
            let dw = w.saturating_sub(self.last_disk_w) * 512 / 2;
            self.last_disk_r = r;
            self.last_disk_w = w;
            Some((dr, dw))
        } else {
            self.last_disk_r = r;
            self.last_disk_w = w;
            Some((0, 0))
        }
    }
}

/// Byte rate → bits-per-second readout: "12.5 Mbps" / "340 Kbps" / "80 bps".
/// (Rates shown to the user are bits/s — the net-history graph data stays
/// in bytes.)
fn fmt_rate(bytes_per_s: u64) -> String {
    let bits_per_s = bytes_per_s.saturating_mul(8);
    if bits_per_s >= 1_000_000 {
        format!("{:.1} Mbps", bits_per_s as f64 / 1_000_000.0)
    } else if bits_per_s >= 10_000 {
        format!("{:.0} Kbps", bits_per_s as f64 / 1_000.0)
    } else if bits_per_s >= 1_000 {
        format!("{:.1} Kbps", bits_per_s as f64 / 1_000.0)
    } else {
        format!("{bits_per_s} bps")
    }
}

// ── extra card data polling ──

impl Stats {
    /// Poll disk partitions from /proc/mounts + statvfs.
    pub fn poll_disk_parts(shell: &mut Shell) {
        let mut parts = Vec::new();
        if let Ok(mounts) = std::fs::read_to_string("/proc/mounts") {
            for l in mounts.lines() {
                let mut it = l.split_whitespace();
                let dev = it.next().unwrap_or("");
                let mount = it.next().unwrap_or("");
                let fstype = it.next().unwrap_or("");
                // skip pseudo/overlay/tmpfs/etc
                if !dev.starts_with("/dev/") { continue; }
                if !["ext4","btrfs","xfs","f2fs","ntfs","vfat"].iter().any(|&t| fstype == t) { continue; }
                let mut st: libc::statvfs = unsafe { std::mem::zeroed() };
                if unsafe { libc::statvfs(
                    std::ffi::CString::new(mount).unwrap().as_ptr(),
                    &mut st
                ) } != 0 { continue; }
                let total = st.f_blocks as u64 * st.f_frsize as u64;
                let avail = st.f_bavail as u64 * st.f_frsize as u64;
                if total == 0 { continue; }
                let used = total - avail;
                let pct = (used * 100 / total) as i32;
                let total_gb = total as f32 / 1_073_741_824.0;
                let used_gb = used as f32 / 1_073_741_824.0;
                parts.push((mount.to_string(), pct, used_gb, total_gb));
            }
        }
        parts.sort_by(|a, b| b.1.cmp(&a.1)); // most used first
        shell.disk_parts = parts;
    }

    /// Poll swap usage from /proc/meminfo.
    pub fn poll_swap(shell: &mut Shell) {
        if let Ok(mi) = std::fs::read_to_string("/proc/meminfo") {
            let mut total = 0u64;
            let mut free = 0u64;
            for l in mi.lines() {
                let mut it = l.split_whitespace();
                let key = it.next().unwrap_or("");
                let val: u64 = it.next().and_then(|v| v.parse().ok()).unwrap_or(0);
                match key {
                    "SwapTotal:" => total = val,
                    "SwapFree:" => free = val,
                    _ => {}
                }
            }
            shell.swap_pct = if total > 0 { ((total - free) * 100 / total) as i32 } else { 0 };
        }
    }

    /// Top 5 processes by CPU% — reads /proc/[pid]/stat.
    pub fn poll_top_procs(shell: &mut Shell) {
        let mut procs: Vec<(String, i32, i32)> = Vec::new();
        if let Ok(rd) = std::fs::read_dir("/proc") {
            for e in rd.flatten() {
                let name = e.file_name();
                let pid: u32 = match name.to_string_lossy().parse() { Ok(v) => v, Err(_) => continue };
                let stat_path = format!("/proc/{pid}/stat");
                let comm_path = format!("/proc/{pid}/comm");
                let stat = match std::fs::read_to_string(&stat_path) { Ok(v) => v, Err(_) => continue };
                let comm = std::fs::read_to_string(&comm_path).unwrap_or_default();
                let comm = comm.trim().to_string();
                if comm.is_empty() { continue; }
                // parse utime + stime (fields 14,15 after comm which may contain parens)
                let after_paren = stat.rfind(')').map(|i| i + 2).unwrap_or(0);
                let fields: Vec<&str> = stat[after_paren..].split_whitespace().collect();
                let utime: u64 = fields.get(11).and_then(|v| v.parse().ok()).unwrap_or(0);
                let stime: u64 = fields.get(12).and_then(|v| v.parse().ok()).unwrap_or(0);
                let cpu_ticks = utime + stime;
                procs.push((comm, cpu_ticks as i32, 0));
            }
        }
        // sort by cpu ticks descending, take top 5
        procs.sort_by(|a, b| b.1.cmp(&a.1));
        procs.truncate(5);
        shell.top_procs = procs;
    }

    /// Poll active window via hyprctl.
    pub fn poll_active_win(shell: &mut Shell) {
        if let Ok(o) = std::process::Command::new("hyprctl")
            .args(["activewindow", "-j"])
            .output()
        {
            let j: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap_or(serde_json::Value::Null);
            shell.active_win_title = j["title"].as_str().unwrap_or("").to_string();
            shell.active_win_class = j["class"].as_str().unwrap_or("").to_string();
            shell.active_win_pid = j["pid"].as_i64().map(|p| p as i32).unwrap_or(0);
            shell.active_win_rss_kb = 0;
            if !shell.active_win_class.is_empty() && shell.active_win_pid > 0 {
                // per-process resident set from /proc — cheaper than running
                // another tool, no shelling out per tick
                if let Ok(status) = std::fs::read_to_string(format!("/proc/{}/status", shell.active_win_pid)) {
                    for line in status.lines() {
                        if let Some(rest) = line.strip_prefix("VmRSS:") {
                            shell.active_win_rss_kb = rest
                                .split_whitespace()
                                .next()
                                .and_then(|v| v.parse().ok())
                                .unwrap_or(0);
                            break;
                        }
                    }
                }
            }
        }
    }

    /// Poll available package updates count.
    pub fn poll_pkg_updates(shell: &mut Shell) {
        // try checkupdates (pacman), then apt
        let count = std::process::Command::new("sh")
            .args(["-c", "which checkupdates >/dev/null 2>&1 && checkupdates 2>/dev/null | wc -l || (which apt >/dev/null 2>&1 && apt list --upgradable 2>/dev/null | grep -c upgradable || echo 0)"])
            .output()
            .ok()
            .and_then(|o| String::from_utf8_lossy(&o.stdout).trim().parse::<i32>().ok())
            .unwrap_or(0);
        shell.pkg_update_count = count;
    }

    /// Poll SSH/VPN status.
    pub fn poll_ssh_vpn(shell: &mut Shell) {
        let mut lines = Vec::new();
        // check for tun/tap interfaces (VPN)
        if let Ok(sys_net) = std::fs::read_dir("/sys/class/net") {
            for e in sys_net.flatten() {
                let name = e.file_name().to_string_lossy().to_string();
                if name.starts_with("tun") || name.starts_with("tap") || name.starts_with("wg") {
                    lines.push(format!("VPN: {name}"));
                }
            }
        }
        // check ssh agent connections
        if let Ok(o) = std::process::Command::new("sh")
            .args(["-c", "ss -tnp 2>/dev/null | grep -c ':22 '"])
            .output()
        {
            let n: i32 = String::from_utf8_lossy(&o.stdout).trim().parse().unwrap_or(0);
            if n > 0 {
                lines.push(format!("SSH: {n} connection{}", if n == 1 { "" } else { "s" }));
            }
        }
        shell.ssh_vpn_lines = lines;
    }

    /// Poll Docker containers.
    pub fn poll_docker(shell: &mut Shell) {
        if let Ok(o) = std::process::Command::new("docker")
            .args(["ps", "--format", "{{.Names}}\t{{.Status}}\t{{.Image}}"])
            .output()
        {
            shell.docker_containers = String::from_utf8_lossy(&o.stdout)
                .lines()
                .filter_map(|l| {
                    let mut it = l.split('\t');
                    let name = it.next()?.to_string();
                    let status = it.next()?.to_string();
                    let image = it.next()?.to_string();
                    Some((name, status, image))
                })
                .collect();
        }
    }

    /// Poll keyboard layout via hyprctl.
    pub fn poll_kb_layout(shell: &mut Shell) {
        if let Ok(o) = std::process::Command::new("hyprctl")
            .args(["devices", "-j"])
            .output()
        {
            let j: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap_or(serde_json::Value::Null);
            if let Some(keyboards) = j["keyboards"].as_array() {
                if let Some(first) = keyboards.first() {
                    shell.kb_layout = first["active_keymap"].as_str().unwrap_or("").to_string();
                }
            }
        }
    }
}
