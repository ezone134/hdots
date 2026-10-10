use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

use crate::entry::{Entry, EntryType, MAX_ENTRIES};
use crate::tab::{SortKey, Tab};

const IMAGE_EXTS: &[&str] = &[".png", ".jpg", ".jpeg", ".gif", ".bmp", ".webp", ".tga"];
const ARCHIVE_EXTS: &[&str] = &[
    ".zip", ".tar", ".tar.gz", ".tgz", ".tar.bz2", ".tbz2", ".tar.xz", ".txz",
    ".gz", ".bz2", ".xz", ".7z", ".rar", ".zst",
];

pub fn classify(name: &str, is_dir: bool) -> EntryType {
    if is_dir {
        return EntryType::Dir;
    }
    for ext in IMAGE_EXTS {
        if crate::config::has_suffix_ci(name, ext) {
            return EntryType::Image;
        }
    }
    for ext in ARCHIVE_EXTS {
        if crate::config::has_suffix_ci(name, ext) {
            return EntryType::Archive;
        }
    }
    EntryType::File
}

/// Sort entries in place. Port of `fs_sort_entries` (dirs-first, then the
/// sort key, then a case-insensitive name tie-break).
pub fn sort_entries(entries: &mut [Entry], dirs_first: bool, key: SortKey, desc: bool) {
    entries.sort_by(|a, b| {
        let mut ord = std::cmp::Ordering::Equal;
        if dirs_first && a.is_dir != b.is_dir {
            ord = if a.is_dir { std::cmp::Ordering::Less } else { std::cmp::Ordering::Greater };
            return if desc { ord.reverse() } else { ord };
        }
        match key {
            SortKey::Size => ord = a.size.cmp(&b.size),
            SortKey::Mtime => ord = a.mtime.cmp(&b.mtime),
            SortKey::Type => ord = ext_of(&a.name).cmp(ext_of(&b.name)),
            SortKey::Name => {}
        }
        if ord == std::cmp::Ordering::Equal {
            ord = a.name.to_lowercase().cmp(&b.name.to_lowercase());
        }
        if desc {
            ord.reverse()
        } else {
            ord
        }
    });
}

/// Extension (after the last dot, excluding a leading-dot name like `.git`)
/// used by `SortKey::Type`.
fn ext_of(name: &str) -> &str {
    match name.rfind('.') {
        Some(i) if i > 0 && i + 1 < name.len() => &name[i + 1..],
        _ => "",
    }
}

/// Build a single entry from a directory entry. Shared by list_dir and the
/// async worker so both produce identical metadata.
fn entry_from_dir_ent(ent: &fs::DirEntry) -> Option<Entry> {
    let name = ent.file_name().to_string_lossy().to_string();
    if name == "." || name == ".." {
        return None;
    }
    let full = ent.path();
    let lmeta = fs::symlink_metadata(&full);
    let mut e = Entry { name: name.clone(), ..Default::default() };
    if let Ok(lm) = lmeta {
        if lm.file_type().is_symlink() {
            e.kind = EntryType::Link;
            e.is_link = true;
            // `ent.metadata()` does not follow the link, so stat the target
            // to tell a folder symlink from a file symlink (like Dolphin).
            if let Ok(m) = fs::metadata(&full) {
                e.is_dir = m.is_dir();
                e.size = if e.is_dir { 0 } else { lm.size() };
                if !e.is_dir {
                    e.is_exec = m.mode() & 0o111 != 0;
                }
            }
            e.mtime = lm.mtime();
        } else {
            e.is_dir = lm.is_dir();
            e.kind = classify(&name, e.is_dir);
            e.size = lm.size();
            e.mtime = lm.mtime();
            // hard link detection (nlink > 1); dirs always have nlink > 1
            // from subdirs, so only flag regular files
            if !e.is_dir && lm.nlink() > 1 {
                e.is_hardlink = true;
            }
            if !e.is_dir {
                e.is_exec = lm.mode() & 0o111 != 0;
            }
        }
    } else {
        e.kind = EntryType::File;
    }
    Some(e)
}

/// Thread-safe pure listing. Used by the async worker; returns a fresh vector.
pub fn read_entries(
    dir: &Path,
    dirs_first: bool,
    show_hidden: bool,
    sort_key: SortKey,
    sort_desc: bool,
) -> Vec<Entry> {
    let mut entries = Vec::new();
    let Ok(read) = fs::read_dir(dir) else {
        return entries;
    };
    for ent in read.flatten() {
        if entries.len() >= MAX_ENTRIES {
            break;
        }
        let Some(name) = ent.file_name().to_str().map(|s| s.to_string()) else {
            continue;
        };
        if name.starts_with('.') && !show_hidden {
            continue;
        }
        if let Some(e) = entry_from_dir_ent(&ent) {
            entries.push(e);
        }
    }
    sort_entries(&mut entries, dirs_first, sort_key, sort_desc);
    entries
}

/// Synchronous directory listing. Port of `fs_list_dir` (main-thread or
/// worker; operates on the caller's own state).
pub fn list_dir(t: &mut Tab, dirs_first: bool, show_hidden: bool) {
    t.busy = true;
    t.entries = read_entries(&t.cwd, dirs_first, show_hidden, t.sort_key, t.sort_desc);
    t.busy = false;
}

/// Filter matching, port of `fs_match_filter`.
pub fn match_filter(
    filter: &str,
    regex_mode: bool,
    re: &Option<regex::Regex>,
    name: &str,
) -> bool {
    if filter.is_empty() {
        return true;
    }
    if regex_mode {
        match re {
            Some(r) => r.is_match(name),
            None => false,
        }
    } else {
        name.to_lowercase().contains(&filter.to_lowercase())
    }
}

pub fn filter_set(t: &mut Tab, filter: &str, regex_mode: bool) {
    t.filter = filter.chars().take(crate::entry::WFM_MAX_INPUT - 1).collect();
    t.filter_regex = regex_mode;
    t.filter_re = None;
    if regex_mode && !filter.is_empty() {
        if let Ok(r) = regex::RegexBuilder::new(filter)
            .case_insensitive(true)
            .build()
        {
            t.filter_re = Some(r);
        }
    }
}

/// Parent dir, port of `fs_parent`.
pub fn parent(cwd: &Path) -> PathBuf {
    match cwd.parent() {
        Some(p) if !p.as_os_str().is_empty() => p.to_path_buf(),
        _ => PathBuf::from("/"),
    }
}

/// Resolve a symlink to its (absolute) target path. Returns None on error.
pub fn resolve_link(path: &Path) -> Option<PathBuf> {
    let target = std::fs::read_link(path).ok()?;
    if target.is_absolute() {
        Some(target)
    } else {
        let base = path.parent().unwrap_or_else(|| Path::new("/"));
        Some(base.join(target))
    }
}

pub fn full_path(cwd: &Path, name: &str) -> PathBuf {
    let p = Path::new(name);
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        cwd.join(name)
    }
}

/// text/uri-list → local path. Port of `fs_uri_to_path`. (DnD milestone)
#[allow(dead_code)]
pub fn uri_to_path(uri: &str) -> Option<PathBuf> {
    let uri = uri.strip_prefix("file://")?;
    let path = if let Some(host_end) = uri.find('/') {
        let host = &uri[..host_end];
        if !host.is_empty() {
            return None; // remote host, reject
        }
        &uri[host_end..]
    } else {
        uri
    };
    Some(PathBuf::from(percent_decode(path)))
}

/// local path → file:// URI (percent-encoded). Port of `fs_path_to_uri`. (DnD milestone)
#[allow(dead_code)]
pub fn path_to_uri(path: &str) -> String {
    let mut out = String::from("file://");
    for b in path.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'/' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            _ => {
                out.push_str(&format!("%{:02X}", b));
            }
        }
    }
    out
}

#[allow(dead_code)]
fn hexval(c: u8) -> Option<u32> {
    match c {
        b'0'..=b'9' => Some((c - b'0') as u32),
        b'a'..=b'f' => Some((c - b'a' + 10) as u32),
        b'A'..=b'F' => Some((c - b'A' + 10) as u32),
        _ => None,
    }
}

#[allow(dead_code)]
fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() + 1 && i + 2 < bytes.len() + 1 {
            let hi = hexval(bytes[i + 1]);
            let lo = hexval(bytes[i + 2]);
            if let (Some(h), Some(l)) = (hi, lo) {
                out.push(((h << 4) | l) as u8);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}


/// A real (user-visible) mount: mount point + source device + filesystem.
#[derive(Clone, Debug)]
pub struct MountInfo {
    pub mp: PathBuf,
    pub dev: String,
    pub fstype: String,
}

/// OS-level pseudo filesystems to never show as user partitions.
const SKIP_FSTYPES: &[&str] = &[
    "proc", "sysfs", "devtmpfs", "devpts", "tmpfs", "cgroup", "cgroup2", "pstore",
    "bpf", "debugfs", "tracefs", "securityfs", "hugetlbfs", "mqueue", "fusectl",
    "configfs", "rpc_pipefs", "binfmt_misc", "autofs", "overlay", "squashfs",
    "fuse.portal", "fuse.snapfuse", "ramfs",
];

/// Mount points under these trees are kernel/OS internals, not user
/// partitions (e.g. /sys, /dev, /proc).
fn is_os_tree(mp: &str) -> bool {
    const TREES: &[&str] = &["/sys", "/dev", "/proc", "/sys/", "/dev/", "/proc/"];
    TREES.iter().any(|t| mp == *t || mp.starts_with(t))
}

/// Boot/EFI mount points never shown as user volumes (Dolphin hides the ESP
/// and boot partitions): /boot, /boot/efi, /efi and their subtrees.
fn is_boot_mount(mp: &str) -> bool {
    mp == "/boot" || mp == "/efi" || mp.starts_with("/boot/") || mp.starts_with("/efi/")
}

/// GPT partition type GUIDs the sidebar hides, mirroring Dolphin/Solid's
/// system-partition blocklist: EFI System, Microsoft Reserved, Windows
/// Recovery, BIOS Boot (grub) and Linux swap.
const HIDDEN_PART_GUIDS: &[&str] = &[
    "c12a7328-f81f-11d2-ba4b-00a0c93ec93b", // EFI System Partition
    "e3c9e316-0b5c-4db8-817d-f92df00215ae", // Microsoft Reserved
    "de94bba4-06d1-4d40-a16a-bfd50179d6ac", // Windows Recovery
    "21686148-6449-6e6f-744e-656564454649", // BIOS Boot (grub)
    "0657fd6d-a4ab-43c4-84e5-0933c84b4f4f", // Linux swap
];

/// Read the GPT partition-type GUID (canonical lowercase, mixed-endian
/// decoded) for partition `part_num` (1-based) of `disk` (a block device
/// path like /dev/nvme0n1). None if not GPT / unreadable / out of range.
fn read_partition_type_guid(disk: &Path, part_num: u32, sector: u64) -> Option<String> {
    use std::io::{Read, Seek, SeekFrom};
    if part_num == 0 {
        return None;
    }
    let mut f = std::fs::File::open(disk).ok()?;
    let mut hdr = [0u8; 92];
    f.seek(SeekFrom::Start(sector)).ok()?; // GPT header at LBA 1
    f.read_exact(&mut hdr).ok()?;
    if &hdr[0..8] != b"EFI PART" {
        return None;
    }
    let entries_lba = u64::from_le_bytes(hdr[72..80].try_into().ok()?);
    let entry_size = u32::from_le_bytes(hdr[84..88].try_into().ok()?);
    let off = entries_lba
        .saturating_mul(sector)
        .saturating_add((part_num as u64 - 1).saturating_mul(entry_size as u64));
    let mut ent = [0u8; 16];
    f.seek(SeekFrom::Start(off)).ok()?;
    f.read_exact(&mut ent).ok()?;
    if ent.iter().all(|&b| b == 0) {
        return None; // empty partition slot
    }
    Some(guid_string(&ent))
}

/// GPT GUIDs are stored mixed-endian (first three fields little-endian);
/// decode the 16 raw bytes into a canonical lowercase string.
fn guid_string(raw: &[u8; 16]) -> String {
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        raw[3], raw[2], raw[1], raw[0], // Data1 (LE)
        raw[5], raw[4],                 // Data2 (LE)
        raw[7], raw[6],                 // Data3 (LE)
        raw[8], raw[9],                 // Data4
        raw[10], raw[11], raw[12], raw[13], raw[14], raw[15], // Data5..8
    )
}

/// Resolve a partition device (e.g. /dev/nvme0n1p1) to its (disk device
/// path, partition number, sector size) via /sys/block. None if not a
/// partition or the sysfs entry is missing.
fn partition_info(dev: &Path) -> Option<(PathBuf, u32, u64)> {
    let name = dev.file_name()?.to_str()?;
    let rd = std::fs::read_dir("/sys/block").ok()?;
    for disk in rd.flatten() {
        let part_dir = disk.path().join(name);
        if !part_dir.is_dir() {
            continue;
        }
        let num: u32 = std::fs::read_to_string(part_dir.join("partition"))
            .ok()?
            .trim()
            .parse()
            .ok()?;
        let sector: u64 =
            std::fs::read_to_string(disk.path().join("queue/logical_block_size"))
                .ok()
                .and_then(|s| s.trim().parse().ok())
                .unwrap_or(512);
        let dname = disk.file_name().to_string_lossy().to_string();
        return Some((Path::new("/dev").join(&dname), num, sector));
    }
    None
}

/// Is `dev` (e.g. /dev/nvme0n1p1) a hidden system partition? Resolves the
/// sysfs partition entry to find its disk + number, then reads the GPT type
/// GUID. Unreadable devices (no permission, non-GPT) are shown, not hidden.
fn is_hidden_partition(dev: &Path) -> bool {
    let Some((disk_dev, num, sector)) = partition_info(dev) else {
        return false;
    };
    match read_partition_type_guid(&disk_dev, num, sector) {
        Some(g) => HIDDEN_PART_GUIDS.contains(&g.as_str()),
        None => false,
    }
}

/// GPT partition name (PARTNAME, e.g. "Media", "common") of a partition
/// device, decoded from the GPT entry's UTF-16LE field. Empty when unknown.
pub fn vol_partname(dev: &Path) -> String {
    let Some((disk_dev, num, sector)) = partition_info(dev) else {
        return String::new();
    };
    read_partition_name(&disk_dev, num, sector).unwrap_or_default()
}

/// Read the GPT partition *name* (bytes 56..128 of the entry, UTF-16LE,
/// null-terminated) for partition `part_num` of `disk`.
fn read_partition_name(disk: &Path, part_num: u32, sector: u64) -> Option<String> {
    use std::io::{Read, Seek, SeekFrom};
    if part_num == 0 {
        return None;
    }
    let mut f = std::fs::File::open(disk).ok()?;
    let mut hdr = [0u8; 92];
    f.seek(SeekFrom::Start(sector)).ok()?;
    f.read_exact(&mut hdr).ok()?;
    if &hdr[0..8] != b"EFI PART" {
        return None;
    }
    let entries_lba = u64::from_le_bytes(hdr[72..80].try_into().ok()?);
    let entry_size = u32::from_le_bytes(hdr[84..88].try_into().ok()?);
    let off = entries_lba
        .saturating_mul(sector)
        .saturating_add((part_num as u64 - 1).saturating_mul(entry_size as u64));
    let mut ent = [0u8; 128];
    f.seek(SeekFrom::Start(off)).ok()?;
    f.read_exact(&mut ent).ok()?;
    if ent[..16].iter().all(|&b| b == 0) {
        return None; // empty partition slot
    }
    let mut name = String::new();
    let mut i = 56;
    while i + 1 < 128 {
        let u = u16::from_le_bytes([ent[i], ent[i + 1]]);
        if u == 0 {
            break;
        }
        if let Some(c) = char::from_u32(u as u32) {
            name.push(c);
        }
        i += 2;
    }
    let name = name.trim().to_string();
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

/// Filesystem type of a block device by reading its superblock magic
/// (ext2/3/4, btrfs, xfs, f2fs, vfat, exfat, ntfs, swap, LUKS, iso9660).
/// Empty string when unknown or the device can't be read.
pub fn vol_fstype(dev: &str) -> String {
    let Some(name) = std::path::Path::new(dev)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
    else {
        return String::new();
    };
    let Ok(mut f) = std::fs::File::open(format!("/dev/{name}")) else {
        return String::new();
    };
    use std::io::Read;
    let mut buf = vec![0u8; 0x10050]; // covers the btrfs superblock at 0x10040
    let mut n = 0usize;
    loop {
        match f.read(&mut buf[n..]) {
            Ok(0) => break,
            Ok(k) => n += k,
            Err(_) => break,
        }
        if n >= buf.len() {
            break;
        }
    }
    let b = &buf[..n];
    let magic = |off: usize, m: &[u8]| n >= off + m.len() && &b[off..off + m.len()] == m;
    if magic(0x10040, b"_BHRfS_M") {
        return "btrfs".into();
    }
    if n >= 0x43A && b[0x438] == 0x53 && b[0x439] == 0xEF {
        return "ext4".into(); // 0xEF53 LE
    }
    if magic(0, b"XFSB") {
        return "xfs".into();
    }
    if magic(0x400, b"F2FS") {
        return "f2fs".into();
    }
    if magic(0x6, b"LUKS") {
        return "crypto_LUKS".into();
    }
    if magic(0x3, b"NTFS") {
        return "ntfs".into();
    }
    if magic(0x3, b"EXFAT") {
        return "exfat".into();
    }
    if magic(0x36, b"FAT12") || magic(0x36, b"FAT16") || magic(0x36, b"FAT32")
        || magic(0x52, b"FAT32")
    {
        return "vfat".into();
    }
    if magic(0xFF6, b"SWAPSPACE2") || magic(0xFF6, b"SWAPSPACE")
        || magic(0xFF6, b"SWAP-SPACE")
    {
        return "swap".into();
    }
    if n >= 0x8006 && &b[0x8001..0x8006] == b"CD001" {
        return "iso9660".into();
    }
    String::new()
}

/// Parse /proc/mounts into real user-visible mounts (skip virtual fs,
/// OS-level trees like /sys, /dev, /proc and boot/EFI system partitions).
/// Port of `fs_mount_points`.
pub fn mounts(max: usize) -> Vec<MountInfo> {
    let Ok(s) = std::fs::read_to_string("/proc/mounts") else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for line in s.lines() {
        let mut parts = line.split_whitespace();
        let Some(src) = parts.next() else { continue };
        let Some(mp) = parts.next() else { continue };
        let Some(fstype) = parts.next() else { continue };
        if SKIP_FSTYPES.contains(&fstype) || is_os_tree(mp) || is_boot_mount(mp) {
            continue;
        }
        // hide mounted EFI/boot system partitions by GPT type (Dolphin-style)
        if src.starts_with("/dev/") && is_hidden_partition(Path::new(src)) {
            continue;
        }
        let path = PathBuf::from(mp.replace("\\040", " "));
        if !out.iter().any(|m: &MountInfo| m.mp == path) {
            out.push(MountInfo {
                mp: path,
                dev: src.to_string(),
                fstype: fstype.to_string(),
            });
        }
        if out.len() >= max {
            break;
        }
    }
    out
}

/// Filesystem label of a device (via /dev/disk/by-label symlinks), or "" if
/// the device has no label. `dev` may be "/dev/nvme0n1p2", "UUID=…" or a
/// mount point.
pub fn vol_label(dev: &str) -> String {
    let Ok(rd) = std::fs::read_dir("/dev/disk/by-label") else {
        return String::new();
    };
    let want = std::path::Path::new(dev)
        .file_name()
        .map(|s| s.to_string_lossy().to_string());
    for ent in rd.flatten() {
        let name = ent.file_name().to_string_lossy().to_string();
        let Ok(target) = std::fs::read_link(ent.path()) else {
            continue;
        };
        if let Some(w) = &want {
            if target.to_string_lossy().ends_with(w) {
                return name;
            }
        }
    }
    String::new()
}

/// Size of a block device in bytes (read from /sys/class/block/<name>/size,
/// 512-byte sectors). 0 when the device can't be read.
pub fn vol_size(dev: &str) -> i64 {
    let Some(name) = std::path::Path::new(dev).file_name() else {
        return 0;
    };
    let Ok(s) = std::fs::read_to_string(format!(
        "/sys/class/block/{}/size",
        name.to_string_lossy()
    )) else {
        return 0;
    };
    let sectors: i64 = s.trim().parse().unwrap_or(0);
    sectors.saturating_mul(512)
}

/// Whether a block device is removable (USB stick, card reader, …). For a
/// partition this checks the removable flag of its parent disk
/// (nvme0n1p1 → nvme0n1, sda1 → sda).
pub fn is_removable(dev: &str) -> bool {
    let Some(name) = std::path::Path::new(dev).file_name() else {
        return false;
    };
    let name = name.to_string_lossy().to_string();
    let parent = parent_disk_name(&name);
    for c in [name, parent] {
        let p = format!("/sys/class/block/{c}/removable");
        if let Ok(v) = std::fs::read_to_string(p) {
            return v.trim() == "1";
        }
    }
    false
}

/// nvme0n1p1 → nvme0n1, mmcblk0p1 → mmcblk0, sda1 → sda; unchanged otherwise.
pub fn parent_disk_name(name: &str) -> String {
    // sd/hd/vd/xvd-style disks number partitions with a bare suffix (sda1);
    // everything else (nvme0n1p1, mmcblk0p1, …) uses a `p` separator, so a
    // whole nvme disk like `nvme0n1` must never lose its trailing digit.
    if name.starts_with("sd")
        || name.starts_with("hd")
        || name.starts_with("vd")
        || name.starts_with("xvd")
    {
        let digits = name.bytes().rev().take_while(|b| b.is_ascii_digit()).count();
        if digits > 0 && digits < name.len() {
            return name[..name.len() - digits].to_string();
        }
        return name.to_string();
    }
    if let Some(i) = name.rfind('p') {
        let tail = &name[i + 1..];
        if !tail.is_empty() && tail.bytes().all(|b| b.is_ascii_digit()) {
            return name[..i].to_string();
        }
    }
    name.to_string()
}

/// "5 GB", "1.5 GB", "800 MB" — Thunar-style volume size string.
pub fn vol_size_str(sz: i64) -> String {
    const GB: i64 = 1073741824;
    const MB: i64 = 1048576;
    if sz >= GB {
        let g = sz as f64 / GB as f64;
        let r = g.round();
        if (g - r).abs() < 0.05 {
            format!("{} GB", r as i64)
        } else {
            format!("{:.1} GB", g)
        }
    } else {
        format!("{} MB", (sz / MB).max(1))
    }
}

/// Thunar-style label for an unmounted volume. Resolution order:
/// 1. the label itself when the partition has one — filesystem label
///    (by-label) or GPT partition name (PARTNAME), e.g. "grub", "tw-cli",
///    "data-hdd";
/// 2. "{size} {fstype} volume" (e.g. "5 GB btrfs volume") when unlabeled;
/// 3. "{size} volume" when the type is unknown;
/// 4. the plain device name.
///
/// A " (Removable)" suffix is appended for removable devices.
pub fn unmounted_volume_label(dev: &str) -> String {
    let p = std::path::Path::new(dev);
    let base = p
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| dev.to_string());
    let label = vol_label(dev);
    let label = if label.is_empty() { vol_partname(p) } else { label };
    let sz = vol_size(dev);
    let size = if sz > 0 { vol_size_str(sz) } else { String::new() };
    let mut s = if !label.is_empty() {
        label
    } else {
        let fst = vol_fstype(dev);
        if !fst.is_empty() {
            if size.is_empty() {
                format!("{fst} volume")
            } else {
                format!("{size} {fst} volume")
            }
        } else if !size.is_empty() {
            format!("{size} volume")
        } else {
            base
        }
    };
    if is_removable(dev) {
        s.push_str(" (Removable)");
    }
    s
}

/// Batch version of [`unmounted_volume_label`] for a list of devices (used
/// once per sidebar refresh; device reads are cached in the App).
pub fn unmounted_volume_labels(devs: &[PathBuf]) -> Vec<String> {
    devs.iter()
        .map(|d| unmounted_volume_label(&d.display().to_string()))
        .collect()
}

fn is_mounted_dev(dev: &str) -> bool {
    let Ok(s) = std::fs::read_to_string("/proc/mounts") else {
        return false;
    };
    for line in s.lines() {
        if let Some(src) = line.split_whitespace().next() {
            if src == dev {
                return true;
            }
        }
    }
    false
}

/// Unmounted block devices via /sys/block, at *partition* level: if a disk
/// has partitions (nvme0n1p1, sda1, …) the unmounted partitions are listed;
/// a partitionless disk is listed itself. Port of `fs_unmounted_volumes`.
pub fn unmounted_volumes(max: usize) -> Vec<PathBuf> {
    let Ok(rd) = std::fs::read_dir("/sys/block") else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for ent in rd.flatten() {
        let name = ent.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        // skip loop/ram/dm devices
        if name.starts_with("loop") || name.starts_with("ram") || name.starts_with("dm-") {
            continue;
        }
        // partition-level: children are named {disk}p1.. or {disk}1..
        let mut parts: Vec<String> = Vec::new();
        if let Ok(pd) = std::fs::read_dir(ent.path()) {
            for p in pd.flatten() {
                let pn = p.file_name().to_string_lossy().to_string();
                if pn == name {
                    continue;
                }
                let is_part = pn.strip_prefix(&name).map(|s| {
                    s.starts_with('p') || s.chars().all(|c| c.is_ascii_digit())
                }).unwrap_or(false);
                if is_part {
                    parts.push(pn);
                }
            }
        }
        if !parts.is_empty() {
            for pn in parts {
                let dev = format!("/dev/{pn}");
                if !is_mounted_dev(&dev) && !is_hidden_partition(Path::new(&dev)) {
                    out.push(PathBuf::from(dev));
                }
                if out.len() >= max {
                    break;
                }
            }
        } else {
            let dev = format!("/dev/{name}");
            if !is_mounted_dev(&dev) {
                out.push(PathBuf::from(dev));
            }
        }
        if out.len() >= max {
            break;
        }
    }
    out
}

/// Free + total bytes of a filesystem (statvfs). Returns (free, total).
pub fn fs_usage(path: &Path) -> (i64, i64) {
    use std::os::unix::ffi::OsStrExt;
    let c_path = match std::ffi::CString::new(path.as_os_str().as_bytes()) {
        Ok(c) => c,
        Err(_) => return (-1, -1),
    };
    let mut st: libc::statvfs = unsafe { std::mem::zeroed() };
    let r = unsafe { libc::statvfs(c_path.as_ptr(), &mut st) };
    if r != 0 {
        return (-1, -1);
    }
    let free = (st.f_bavail as i64) * (st.f_bsize as i64);
    let total = (st.f_blocks as i64) * (st.f_bsize as i64);
    (free, total)
}

/// Unmount a mount point (polkit prompt when needed). Port of the C `umount`.
pub fn unmount(mp: &Path) -> bool {
    let status = std::process::Command::new("pkexec")
        .arg("umount")
        .arg(mp)
        .status();
    match status {
        Ok(s) => s.success(),
        Err(_) => std::process::Command::new("umount").arg(mp).status().map(|s| s.success()).unwrap_or(false),
    }
}

/// Mount via pkexec (xdg-style). Port of `fs_mount_with_polkit`.
pub fn mount_with_polkit(dev: &Path, mp: &Path) -> bool {
    let _ = std::fs::create_dir_all(mp);
    let status = std::process::Command::new("pkexec")
        .arg("mount")
        .arg(dev)
        .arg(mp)
        .status();
    matches!(status, Ok(s) if s.success())
}

/// Virtual listing kinds shown in the sidebar / content area.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum VirtualKind {
    /// No longer a default sidebar place (still fully implemented — kept for
    /// a possible future toggle).
    #[allow(dead_code)]
    Recent,
    Trash,
}

fn xdg_data_home() -> PathBuf {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
        .unwrap_or_else(|| PathBuf::from("/tmp"))
}

/// `~/.local/share/recently-used.xbel`
pub fn virtual_dir() -> PathBuf {
    xdg_data_home().join("recently-used.xbel")
}

/// `$XDG_DATA_HOME/Trash/files`
pub fn trash_dir() -> PathBuf {
    xdg_data_home().join("Trash/files")
}

/// Port of `fs_trash_path` (gio). Use the freedesktop trash dir when mounted
/// on the same device, otherwise fall back to the XDG data trash.
#[allow(dead_code)]
pub fn trash_path(path: &Path) -> Option<PathBuf> {
    let Ok(meta) = std::fs::metadata(path) else { return None };
    let dev = meta.dev();
    let home = std::env::var_os("HOME").map(PathBuf::from)?;
    let home_meta = std::fs::metadata(&home).ok()?;
    let home_dev = home_meta.dev();
    let mut candidates: Vec<PathBuf> = Vec::new();
    if home_dev == dev {
        candidates.push(home.join(".Trash"));
        candidates.push(home.join(".local/share/Trash"));
    }
    candidates.push(xdg_data_home().join("Trash"));
    for dir in candidates {
        let _ = std::fs::create_dir_all(dir.join("files"));
        let _ = std::fs::create_dir_all(dir.join("info"));
        if std::fs::metadata(dir.join("files")).map(|m| m.dev()).unwrap_or(u64::MAX) == dev {
            return Some(dir.join("files"));
        }
    }
    None
}

/// Move a path into the trash (best effort, free space). Returns the trash
/// destination on success.
#[allow(dead_code)]
pub fn move_to_trash(path: &Path) -> Option<PathBuf> {
    let trash = trash_path(path)?;
    let name = path.file_name()?.to_string_lossy().to_string();
    let dst = unique_copy_name(&trash, &name);
    if move_path(path, &dst).is_ok() {
        Some(dst)
    } else {
        None
    }
}

/// Entries for a virtual listing (Recent/Trash). Recent parses
/// recently-used.xbel; Trash lists `Trash/files`.
pub fn virtual_entries(kind: VirtualKind) -> Vec<Entry> {
    match kind {
        VirtualKind::Trash => {
            let mut entries = Vec::new();
            let Ok(read) = fs::read_dir(trash_dir()) else { return entries };
            for ent in read.flatten() {
                if entries.len() >= MAX_ENTRIES {
                    break;
                }
                if let Some(mut e) = entry_from_dir_ent(&ent) {
                    e.real_path = Some(ent.path());
                    entries.push(e);
                }
            }
            sort_entries(&mut entries, true, SortKey::Name, false);
            entries
        }
        VirtualKind::Recent => recent_entries(),
    }
}

fn extract_attr(s: &str, key: &str) -> Option<String> {
    let k = format!("{key}=\"");
    let i = s.find(&k)? + k.len();
    let rest = &s[i..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

/// "YYYY-MM-DDTHH:MM:SSZ" → unix seconds. Port of the C `xbel` time parse.
fn xbel_time(s: &str) -> Option<i64> {
    if s.len() < 19 {
        return None;
    }
    let b = s.as_bytes();
    let y: i64 = s[0..4].parse().ok()?;
    let mo: i64 = s[5..7].parse().ok()?;
    let d: i64 = s[8..10].parse().ok()?;
    let h: i64 = s[11..13].parse().ok()?;
    let mi: i64 = s[14..16].parse().ok()?;
    let se: i64 = s[17..19].parse().ok()?;
    let _ = b;
    Some(civil_to_days(y, mo, d) * 86400 + h * 3600 + mi * 60 + se)
}

/// Days since 1970-01-01 from a civil date (Howard Hinnant inverse).
fn civil_to_days(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = if m > 2 { m - 3 } else { m + 9 };
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

fn recent_entries() -> Vec<Entry> {
    let mut out = Vec::new();
    let Ok(s) = std::fs::read_to_string(virtual_dir()) else {
        return out;
    };
    for line in s.lines() {
        let t = line.trim_start();
        if !t.starts_with("<bookmark") {
            continue;
        }
        let Some(href) = extract_attr(t, "href") else { continue };
        let Some(path) = uri_to_path(&href) else { continue };
        let mtime = extract_attr(t, "modified")
            .and_then(|m| xbel_time(&m))
            .unwrap_or(0);
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.display().to_string());
        let meta = fs::symlink_metadata(&path).ok();
        let is_dir = meta.as_ref().map(|m| m.is_dir()).unwrap_or(false);
        let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
        let mut e = Entry {
            name,
            size,
            mtime,
            is_dir,
            is_link: false,
            is_exec: !is_dir && meta.as_ref().map(|m| m.mode() & 0o111 != 0).unwrap_or(false),
            real_path: Some(path),
            ..Default::default()
        };
        e.kind = if is_dir { EntryType::Dir } else { classify(&e.name, false) };
        out.push(e);
    }
    out.sort_by(|a, b| b.mtime.cmp(&a.mtime));
    out
}

/// Append a `file://` bookmark to recently-used.xbel. Port of `fs_recent_add`.
pub fn recent_add(path: &Path) {
    let file = virtual_dir();
    let uri = path_to_uri(&path.display().to_string());
    let existing = std::fs::read_to_string(&file).unwrap_or_default();
    if existing.contains(&uri) {
        return;
    }
    let stamp = iso8601_now();
    let entry = format!(
        "  <bookmark href=\"{uri}\" added=\"{stamp}\" modified=\"{stamp}\" visited=\"{stamp}\">\n    <info>\n      <metadata owner=\"http://freedesktop.org\"/>\n    </info>\n  </bookmark>\n"
    );
    let content = if existing.trim().is_empty() {
        format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<xbel version=\"1.0\"\n      xmlns:bookmark=\"http://www.freedesktop.org/standards/desktop-bookmarks\"\n      xmlns:mime=\"http://www.freedesktop.org/standards/shared-mime-info\">\n{entry}</xbel>\n"
        )
    } else if let Some(i) = existing.rfind("</xbel>") {
        format!("{}{}{}", &existing[..i], entry, &existing[i..])
    } else {
        return;
    };
    let _ = std::fs::write(file, content);
}

/// Current UTC time as ISO-8601 "YYYY-MM-DDTHH:MM:SSZ".
fn iso8601_now() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let days = secs.div_euclid(86400);
    let rem = secs.rem_euclid(86400);
    let (y, m, d) = civil_from_days(days);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        y,
        m,
        d,
        rem / 3600,
        (rem % 3600) / 60,
        rem % 60
    )
}

/// Civil date from days since 1970-01-01.
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// Network mounts from /proc/mounts (nfs/cifs/sshfs/fuse...). Port of the
/// sidebar "Network" section.
pub fn network_mounts(max: usize) -> Vec<PathBuf> {
    let Ok(s) = std::fs::read_to_string("/proc/mounts") else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let net = [
        "nfs", "nfs4", "cifs", "smbfs", "sshfs", "fuse.sshfs", "fuse.cifs", "davfs",
        "fuse.davfs", "fuse.rclone",
    ];
    for line in s.lines() {
        let mut parts = line.split_whitespace();
        let _src = parts.next();
        let Some(mp) = parts.next() else { continue };
        let Some(fstype) = parts.next() else { continue };
        if !net.contains(&fstype) {
            continue;
        }
        let path = PathBuf::from(mp.replace("\\040", " "));
        if !out.iter().any(|p: &PathBuf| p == &path) {
            out.push(path);
        }
        if out.len() >= max {
            break;
        }
    }
    out
}

/// Create a symlink at `dst` pointing to `src` (absolute or relative target
/// as given). DnD "Link Here" milestone.
pub fn symlink_path(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(src, dst)
}

/// Hard link (fails across devices or for directories).
pub fn hardlink_path(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::fs::hard_link(src, dst)
}

/// Relative path from `from` (a directory) to `to`, for relative symlinks.
pub fn relative_path(from: &Path, to: &Path) -> PathBuf {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));
    let f_abs = if from.is_absolute() { from.to_path_buf() } else { cwd.join(from) };
    let t_abs = if to.is_absolute() { to.to_path_buf() } else { cwd.join(to) };
    let f: Vec<_> = f_abs.components().collect();
    let t: Vec<_> = t_abs.components().collect();
    let mut i = 0;
    while i < f.len() && i < t.len() && f[i] == t[i] {
        i += 1;
    }
    let mut out = PathBuf::new();
    for _ in i..f.len() {
        out.push("..");
    }
    for c in &t[i..] {
        out.push(c);
    }
    out
}

/// First bytes of a small text file for the preview pane.
pub fn read_preview_text(path: &Path, max: usize) -> String {
    use std::io::Read;
    let Ok(mut f) = std::fs::File::open(path) else {
        return String::new();
    };
    let mut buf = vec![0u8; max];
    let n = f.read(&mut buf).unwrap_or(0);
    buf.truncate(n);
    // reject obvious binary
    if buf.contains(&0) {
        return String::new();
    }
    String::from_utf8_lossy(&buf).to_string()
}

pub fn free_bytes(path: &Path) -> i64 {
    use std::os::unix::ffi::OsStrExt;
    let c_path = match std::ffi::CString::new(path.as_os_str().as_bytes()) {
        Ok(c) => c,
        Err(_) => return -1,
    };
    let mut st: libc::statvfs = unsafe { std::mem::zeroed() };
    let r = unsafe { libc::statvfs(c_path.as_ptr(), &mut st) };
    if r != 0 {
        return -1;
    }
    (st.f_bavail as i64) * (st.f_bsize as i64)
}

pub fn rename_path(old: &Path, new_name: &str) -> std::io::Result<()> {
    let new = old.with_file_name(new_name);
    std::fs::rename(old, &new)
}

pub fn mkdir_path(p: &Path) -> std::io::Result<()> {
    std::fs::create_dir(p)
}

pub fn create_file(p: &Path) -> std::io::Result<()> {
    std::fs::OpenOptions::new().create_new(true).write(true).open(p).map(|_| ())
}

/// Recursively remove a path (files, symlinks, dirs). Delete-key milestone.
#[allow(dead_code)]
pub fn delete_path(p: &Path) -> std::io::Result<()> {
    let meta = std::fs::symlink_metadata(p)?;
    if meta.file_type().is_symlink() {
        std::fs::remove_file(p)
    } else if meta.is_dir() {
        std::fs::remove_dir_all(p)
    } else {
        std::fs::remove_file(p)
    }
}

/// Recursive copy (files/dirs/symlinks as new copies of content).
pub fn copy_path(src: &Path, dst: &Path) -> std::io::Result<()> {
    let meta = std::fs::symlink_metadata(src)?;
    if meta.file_type().is_symlink() {
        let target = std::fs::read_link(src)?;
        let _ = std::fs::remove_file(dst);
        std::os::unix::fs::symlink(target, dst)?;
        return Ok(());
    }
    if meta.is_dir() {
        std::fs::create_dir_all(dst)?;
        for ent in std::fs::read_dir(src)? {
            let ent = ent?;
            let name = ent.file_name();
            copy_path(&ent.path(), &dst.join(name))?;
        }
        return Ok(());
    }
    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::copy(src, dst)?;
    Ok(())
}

/// Move with copy+delete fallback across devices.
pub fn move_path(src: &Path, dst: &Path) -> std::io::Result<()> {
    match std::fs::rename(src, dst) {
        Ok(()) => Ok(()),
        Err(e) if e.raw_os_error() == Some(18) /* EXDEV */ => {
            copy_path(src, dst)?;
            delete_path(src)
        }
        Err(e) => Err(e),
    }
}

/// Unique path like `name (copy).ext` / `name (copy 2).ext`.
pub fn unique_copy_name(dir: &Path, name: &str) -> PathBuf {
    let p = Path::new(name);
    let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or(name);
    let ext = p.extension().and_then(|s| s.to_str());
    let mk = |n: &str| -> PathBuf {
        match ext {
            Some(e) => dir.join(format!("{n}.{e}")),
            None => dir.join(n),
        }
    };
    let candidate = mk(&format!("{stem} (copy)"));
    if !candidate.exists() {
        return candidate;
    }
    for i in 2..1000 {
        let c = mk(&format!("{stem} (copy {i})"));
        if !c.exists() {
            return c;
        }
    }
    mk(&format!("{stem} (copy {})", std::process::id()))
}

pub fn probe_terminal() -> String {
    const CANDS: &[&str] = &[
        "kitty", "alacritty", "foot", "wezterm", "ghostty", "kgx", "gnome-terminal",
        "konsole", "xfce4-terminal", "xterm",
    ];
    for c in CANDS {
        if which(c) {
            return (*c).to_string();
        }
    }
    String::new()
}

fn which(bin: &str) -> bool {
    let Ok(path) = std::env::var("PATH") else { return false };
    for d in path.split(':') {
        let p = Path::new(d).join(bin);
        if p.is_file() {
            return true;
        }
    }
    false
}

/// Best-effort copy text to clipboard via wl-copy (Wayland).
pub fn clipboard_set_text(s: &str) {
    let _ = std::process::Command::new("wl-copy").arg("--").arg(s).spawn();
}

/// Best-effort read text from the clipboard via wl-paste (Wayland).
pub fn clipboard_get_text() -> String {
    let Ok(out) = std::process::Command::new("wl-paste").output() else {
        return String::new();
    };
    if !out.status.success() {
        return String::new();
    }
    String::from_utf8_lossy(&out.stdout)
        .trim_end_matches(['\n', '\r'])
        .to_string()
}

/// Compute MD5 / SHA-1 / SHA-256 of a file via the coreutils tools. Each hash
/// is computed independently so a missing tool degrades to `n/a` for just that
/// algorithm. Run on the background checksum thread, never the UI loop.
pub fn checksums(path: &Path) -> (String, String, String) {
    (
        checksum_of(path, "md5sum"),
        checksum_of(path, "sha1sum"),
        checksum_of(path, "sha256sum"),
    )
}

fn checksum_of(path: &Path, tool: &str) -> String {
    let Ok(out) = std::process::Command::new(tool).arg(path).output() else {
        return "n/a".into();
    };
    if !out.status.success() {
        return "n/a".into();
    }
    String::from_utf8_lossy(&out.stdout)
        .split_whitespace()
        .next()
        .unwrap_or("n/a")
        .to_string()
}

/// Human-readable `-rw-r--r--`-style mode string for a st_mode value.
pub fn fmt_mode(mode: u32, is_dir: bool) -> String {
    let mut s = String::new();
    if mode & 0o170000 == 0o120000 {
        s.push('l');
    } else if is_dir {
        s.push('d');
    } else {
        s.push('-');
    }
    const BITS: [(u32, char); 9] = [
        (0o400, 'r'),
        (0o200, 'w'),
        (0o100, 'x'),
        (0o040, 'r'),
        (0o020, 'w'),
        (0o010, 'x'),
        (0o004, 'r'),
        (0o002, 'w'),
        (0o001, 'x'),
    ];
    for (b, c) in BITS {
        s.push(if mode & b != 0 { c } else { '-' });
    }
    s
}

fn id_name(file: &str, id: u32) -> Option<String> {
    let body = std::fs::read_to_string(file).ok()?;
    for line in body.lines() {
        let mut it = line.split(':');
        let name = it.next()?.to_string();
        let _pass = it.next()?;
        if it.next()?.parse::<u32>().ok()? == id {
            return Some(name);
        }
    }
    None
}

/// User name for a uid (falls back to the numeric id).
pub fn user_name(uid: u32) -> String {
    id_name("/etc/passwd", uid).unwrap_or_else(|| uid.to_string())
}

/// Group name for a gid (falls back to the numeric id).
pub fn group_name(gid: u32) -> String {
    id_name("/etc/group", gid).unwrap_or_else(|| gid.to_string())
}

// ------------------------------------------------------------------
// mime types, .desktop apps, "open with" + per-type default openers
// ------------------------------------------------------------------

/// Coarse extension-based mime type (no shared-mime-info dependency). Used to
/// pick candidate apps and to look up the per-type default opener.
pub fn mime_for_path(path: &Path) -> String {
    let name = path.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
    let ext = path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    let e = ext.as_str();
    let mut m = match e {
        "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp" | "tga" | "ico" | "avif" => {
            format!("image/{e}")
        }
        "svg" => "image/svg+xml".to_string(),
        "txt" | "log" | "conf" | "ini" | "cfg" | "md" => "text/plain".to_string(),
        "c" | "h" | "py" | "rs" | "go" | "js" | "ts" | "sh" | "css" | "json"
        | "toml" | "yaml" | "yml" | "xml" | "html" | "htm" => "text/plain".to_string(),
        "pdf" => "application/pdf".to_string(),
        "zip" => "application/zip".to_string(),
        "tar" | "gz" | "bz2" | "xz" | "7z" | "rar" | "zst" | "tgz" | "txz" | "tbz2" => {
            "application/x-archive".to_string()
        }
        "mp3" | "flac" | "wav" | "ogg" | "opus" | "m4a" => "audio/mpeg".to_string(),
        "mp4" | "mkv" | "webm" | "avi" | "mov" | "flv" | "mpg" | "mpeg" => {
            "video/mp4".to_string()
        }
        "doc" | "docx" => "application/msword".to_string(),
        "xls" | "xlsx" => "application/vnd.ms-excel".to_string(),
        "ppt" | "pptx" => "application/vnd.ms-powerpoint".to_string(),
        "ods" | "odt" | "odp" => "application/vnd.oasis.opendocument.text".to_string(),
        "deb" | "rpm" | "apk" | "iso" => "application/x-archive".to_string(),
        _ => {
            // dirs & binaries get the generic type
            "application/octet-stream".to_string()
        }
    };
    if name.starts_with('.') && ext.is_empty() && !path.is_dir() {
        m = "text/plain".to_string();
    }
    m
}

/// A parsed `.desktop` file ([Desktop Entry] section only).
pub struct DesktopApp {
    pub id: String,
    pub name: String,
    pub exec: String,
    pub terminal: bool,
    pub mimes: Vec<String>,
}

/// The application dirs to scan for `.desktop` files and `mimeinfo.cache`,
/// in precedence order: `$XDG_DATA_HOME/applications`, then every
/// `$XDG_DATA_DIRS` element's `applications` subdir, then the legacy
/// `/usr/local/share/applications` and `/usr/share/applications` fallbacks.
fn app_data_dirs() -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = Vec::new();
    if let Some(d) = dirs::data_dir() {
        dirs.push(d.join("applications"));
    }
    if let Some(h) = std::env::var_os("HOME") {
        dirs.push(PathBuf::from(h).join(".local/share/applications"));
    }
    if let Some(raw) = std::env::var_os("XDG_DATA_DIRS") {
        for p in std::env::split_paths(&raw) {
            dirs.push(p.join("applications"));
        }
    }
    dirs.push(PathBuf::from("/usr/local/share/applications"));
    dirs.push(PathBuf::from("/usr/share/applications"));
    dirs
}

/// Parse a `mimeinfo.cache` file body into `mime -> desktop-file ids`
/// (only the `[MIME Cache]` section; group lines are ignored).
fn parse_mimeinfo_cache(text: &str) -> std::collections::HashMap<String, Vec<String>> {
    let mut out: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
    let mut in_cache = false;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_cache = line == "[MIME Cache]";
            continue;
        }
        if !in_cache || line.is_empty() {
            continue;
        }
        let Some((mime, rest)) = line.split_once('=') else { continue };
        let ids: Vec<String> =
            rest.split(';').filter(|s| !s.is_empty()).map(|s| s.to_string()).collect();
        if !ids.is_empty() {
            out.insert(mime.trim().to_string(), ids);
        }
    }
    out
}

fn mimeinfo_cache() -> std::collections::HashMap<String, Vec<String>> {
    let mut out: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
    for d in app_data_dirs() {
        let p = d.join("mimeinfo.cache");
        let Ok(text) = fs::read_to_string(&p) else { continue };
        for (mime, ids) in parse_mimeinfo_cache(&text) {
            out.entry(mime).or_insert(ids);
        }
    }
    out
}

/// Scan the standard application dirs for `.desktop` files.
pub fn desktop_apps() -> Vec<DesktopApp> {
    let mut out = Vec::new();
    for d in app_data_dirs() {
        let Ok(rd) = fs::read_dir(&d) else { continue };
        for ent in rd.flatten() {
            let p = ent.path();
            if p.extension().map(|e| e == "desktop").unwrap_or(false) {
                if let Some(a) = parse_desktop(&p) {
                    if !out.iter().any(|o: &DesktopApp| o.id == a.id) {
                        out.push(a);
                    }
                }
            }
        }
    }
    out
}

fn parse_desktop(p: &Path) -> Option<DesktopApp> {
    let text = fs::read_to_string(p).ok()?;
    let mut in_entry = false;
    let mut name = String::new();
    let mut exec = String::new();
    let mut terminal = false;
    let mut mimes: Vec<String> = Vec::new();
    let mut hidden = false;
    let mut no_display = false;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_entry = line == "[Desktop Entry]";
            continue;
        }
        if !in_entry || line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some(eq) = line.find('=') else { continue };
        let k = line[..eq].trim();
        let v = line[eq + 1..].trim();
        match k {
            "Name" if name.is_empty() => name = v.to_string(),
            "Exec" if exec.is_empty() => exec = v.to_string(),
            "Terminal" => terminal = crate::config::parse_bool(v),
            "MimeType" => {
                mimes = v.split(';').filter(|s| !s.is_empty()).map(|s| s.to_string()).collect();
            }
            "Hidden" => hidden = crate::config::parse_bool(v),
            "NoDisplay" => no_display = crate::config::parse_bool(v),
            _ => {}
        }
    }
    if hidden || no_display || name.is_empty() || exec.is_empty() {
        return None;
    }
    Some(DesktopApp {
        id: p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(),
        name,
        exec,
        terminal,
        mimes,
    })
}

/// Does a mime string match another (exact or `type/*` wildcard on either
/// side)? e.g. `mime_matches("image/png", "image/*")` and the reverse both
/// match.
pub fn mime_matches(want: &str, have: &str) -> bool {
    if want == have {
        return true;
    }
    let wild = |a: &str, b: &str| -> bool {
        if let Some((t, sub)) = a.split_once('/') {
            if sub == "*" {
                return b.split_once('/').map(|(bt, _)| bt == t).unwrap_or(false);
            }
        }
        false
    };
    wild(have, want) || wild(want, have)
}

/// Candidate apps (name, exec, terminal) for a mime type, in priority order:
/// 1. the system `mimeinfo.cache` registry (respects its ordering),
/// 2. any remaining `.desktop` files that declare the mime in `MimeType`,
/// 3. a small curated fallback filtered by what's actually on PATH,
/// 4. every known app, so the chooser is never empty.
pub fn apps_for_mime(mime: &str) -> Vec<(String, String, bool)> {
    let apps = desktop_apps();
    let mut out: Vec<(String, String, bool)> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    // 1) mimeinfo.cache order, plus `mime/*` entries that cover the type
    let cache = mimeinfo_cache();
    let mut ids: Vec<String> = Vec::new();
    if let Some(list) = cache.get(mime) {
        ids.extend(list.iter().cloned());
    }
    if let Some((t, _)) = mime.split_once('/') {
        let wild = format!("{t}/*");
        if let Some(list) = cache.get(&wild) {
            for id in list {
                if !ids.contains(id) {
                    ids.push(id.clone());
                }
            }
        }
    }
    for id in ids {
        if let Some(a) = apps.iter().find(|a| a.id == id) {
            if seen.insert(a.id.clone()) {
                out.push((a.name.clone(), a.exec.clone(), a.terminal));
            }
        }
    }
    if !out.is_empty() {
        return out;
    }
    // 2) direct MimeType match
    for a in &apps {
        if a.mimes.iter().any(|m| mime_matches(mime, m)) && seen.insert(a.id.clone()) {
            out.push((a.name.clone(), a.exec.clone(), a.terminal));
        }
    }
    if !out.is_empty() {
        return out;
    }
    // 3) curated fallback, filtered by PATH
    let (t, _) = mime.split_once('/').unwrap_or(("", ""));
    let curated: &[&str] = match t {
        "image" => &["gwenview", "feh", "eog", "loupe", "imv", "swayimg", "viewnior"],
        "text" => &["gedit", "kate", "mousepad", "geany", "code", "nvim", "nano"],
        "audio" => &["vlc", "mpv", "audacious", "elisa"],
        "video" => &["vlc", "mpv", "celluloid", "totem"],
        "application" if mime == "application/pdf" => &["zathura", "okular", "evince", "mupdf", "firefox"],
        "application" => &["file-roller", "xarchiver", "ark", "engrampa"],
        _ => &["xdg-open"],
    };
    for c in curated {
        if cmd_in_path(c) {
            out.push((c.to_string(), c.to_string(), false));
        }
    }
    if !out.is_empty() {
        return out;
    }
    // 4) every known app
    for a in &apps {
        out.push((a.name.clone(), a.exec.clone(), a.terminal));
    }
    out
}

/// Is `name` an executable somewhere on PATH?
pub fn cmd_in_path(name: &str) -> bool {
    let Ok(path) = std::env::var("PATH") else { return false };
    for dir in path.split(':') {
        if dir.is_empty() {
            continue;
        }
        let p = PathBuf::from(dir).join(name);
        if p.is_file() {
            if let Ok(m) = p.metadata() {
                use std::os::unix::fs::PermissionsExt;
                if m.permissions().mode() & 0o111 != 0 {
                    return true;
                }
            }
        }
    }
    false
}

/// Wrap a path in single quotes for `sh -c` (C `shell_quote`).
pub fn shell_quote_path(path: &Path) -> String {
    format!("'{}'", path.display().to_string().replace('\'', "'\\''"))
}

/// Run `cmd` on `path`: desktop-Exec style `%f/%F/%u/%U` placeholders are
/// substituted with the shell-quoted path; commands without placeholders get
/// the path appended (mirrors C `fs_open_with`). Launched detached via sh.
pub fn open_with_cmd(path: &Path, cmd: &str) -> std::io::Result<()> {
    let quoted = shell_quote_path(path);
    let mut c = cmd.trim().to_string();
    if c.is_empty() {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "empty command"));
    }
    if c.contains("%f") || c.contains("%F") || c.contains("%u") || c.contains("%U") {
        c = c.replace("%f", &quoted).replace("%F", &quoted).replace("%u", &quoted).replace("%U", &quoted);
        // drop field codes we don't fill (icon, name, key, desktop file)
        let mut cleaned = String::new();
        let bytes = c.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'%' && i + 1 < bytes.len() {
                let code = bytes[i + 1] as char;
                if matches!(code, 'i' | 'c' | 'k' | 'd' | 'D' | 'n' | 'N' | 'v' | 'm') {
                    i += 2;
                    continue;
                }
            }
            cleaned.push(bytes[i] as char);
            i += 1;
        }
        c = cleaned;
    } else {
        c.push(' ');
        c.push_str(&quoted);
    }
    std::process::Command::new("sh").arg("-c").arg(&c).spawn().map(|_| ())
}

/// `~/.config/wfm/defaults` — per-mime default openers (one `mime = cmd` per
/// line), separate from the hand-edited config.
pub fn defaults_path() -> PathBuf {
    dirs::config_dir()
        .map(|p| p.join("wfm/defaults"))
        .unwrap_or_else(|| PathBuf::from("/tmp/wfm-defaults"))
}

pub fn default_app_for(mime: &str) -> Option<String> {
    let text = fs::read_to_string(defaults_path()).ok()?;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some(eq) = line.find('=') else { continue };
        let k = crate::config::trim(&line[..eq]);
        if k.eq_ignore_ascii_case(mime) {
            let v = crate::config::trim(&line[eq + 1..]);
            if !v.is_empty() {
                return Some(v.to_string());
            }
        }
    }
    None
}

pub fn set_default_app(mime: &str, cmd: &str) {
    let path = defaults_path();
    let text = fs::read_to_string(&path).unwrap_or_default();
    let mut replaced = false;
    let mut out = String::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with('#') {
            let k = crate::config::trim(trimmed.split('=').next().unwrap_or(""));
            if k.eq_ignore_ascii_case(mime) {
                out.push_str(&format!("{} = {}\n", mime, cmd));
                replaced = true;
                continue;
            }
        }
        out.push_str(line);
        out.push('\n');
    }
    if !replaced {
        out.push_str(&format!("{} = {}\n", mime, cmd));
    }
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(path, out);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mounts_hide_os_trees() {
        let m = mounts(64);
        for mo in &m {
            assert!(
                !mo.mp.starts_with("/sys") && !mo.mp.starts_with("/dev") && !mo.mp.starts_with("/proc"),
                "OS-level mount leaked: {}",
                mo.mp.display()
            );
        }
    }

    #[test]
    fn fs_usage_reports_sane_numbers() {
        let (free, total) = fs_usage(Path::new("/"));
        assert!(total > 0, "total should be positive");
        assert!(free >= 0 && free <= total, "free should be within [0, total]");
    }

    #[test]
    fn unmounted_volumes_do_not_crash() {
        let v = unmounted_volumes(8);
        // must never include mounted devices or whole-disk dupes of partitions
        for dev in &v {
            assert!(!is_mounted_dev(&dev.to_string_lossy()), "mounted device in unmounted list: {dev:?}");
        }
    }

    #[test]
    fn parent_disk_name_mapping() {
        assert_eq!(parent_disk_name("nvme0n1p1"), "nvme0n1");
        assert_eq!(parent_disk_name("mmcblk0p1"), "mmcblk0");
        assert_eq!(parent_disk_name("sda1"), "sda");
        assert_eq!(parent_disk_name("sda"), "sda");
        assert_eq!(parent_disk_name("nvme0n1"), "nvme0n1");
    }

    #[test]
    fn vol_size_str_formats() {
        assert_eq!(vol_size_str(5 * 1073741824), "5 GB");
        assert_eq!(vol_size_str((5 * 1073741824) + (1073741824 / 2)), "5.5 GB");
        assert_eq!(vol_size_str(800 * 1048576), "800 MB");
        assert_eq!(vol_size_str(512), "1 MB");
    }

    #[test]
    fn unmounted_label_falls_back_to_device_name() {
        // Nothing is readable for a device that doesn't exist.
        assert_eq!(
            unmounted_volume_label("/dev/wfm-nonexistent-xyz"),
            "wfm-nonexistent-xyz"
        );
    }

    #[test]
    fn classify_works() {
        assert_eq!(classify("a.png", false), EntryType::Image);
        assert_eq!(classify("a.tar.gz", false), EntryType::Archive);
        assert_eq!(classify("a.txt", false), EntryType::File);
        assert_eq!(classify("dir", true), EntryType::Dir);
    }

    #[test]
    fn uri_roundtrip() {
        let p = "/home/user/a b/café.txt";
        let uri = path_to_uri(p);
        let back = uri_to_path(&uri).unwrap();
        assert_eq!(back.to_str().unwrap(), p);
        assert!(uri_to_path("http://x/y").is_none());
    }

    #[test]
    fn parent_dirs() {
        assert_eq!(parent(Path::new("/a/b")), PathBuf::from("/a"));
        assert_eq!(parent(Path::new("/a")), PathBuf::from("/"));
    }

    #[test]
    fn relative_path_works() {
        assert_eq!(relative_path(Path::new("/a/b"), Path::new("/a/b/c.txt")),
                   PathBuf::from("c.txt"));
        assert_eq!(relative_path(Path::new("/a/b"), Path::new("/a/x/y")),
                   PathBuf::from("../x/y"));
        assert_eq!(relative_path(Path::new("/mnt/vol"), Path::new("/home/u/f.txt")),
                   PathBuf::from("../../home/u/f.txt"));
    }

    #[test]
    fn hardlink_detected() {
        use std::fs;
        let dir = std::env::temp_dir().join(format!("wfm-hl-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let a = dir.join("a.txt");
        let b = dir.join("b.txt");
        fs::write(&a, "hello").unwrap();
        fs::hard_link(&a, &b).unwrap();
        let mut found = 0;
        for ent in fs::read_dir(&dir).unwrap() {
            let ent = ent.unwrap();
            let e = entry_from_dir_ent(&ent).unwrap();
            if e.is_hardlink {
                found += 1;
            }
        }
        fs::remove_dir_all(&dir).ok();
        assert_eq!(found, 2, "both hard-linked names must be flagged");
    }

    #[test]
    fn filter_matches() {
        assert!(match_filter("png", false, &None, "a.PnG"));
        assert!(!match_filter("png", false, &None, "a.txt"));
    }

    #[test]
    fn mime_types() {
        assert_eq!(mime_for_path(Path::new("/a/b.png")), "image/png");
        assert_eq!(mime_for_path(Path::new("/a/b.JPG")), "image/jpg");
        assert_eq!(mime_for_path(Path::new("/a/notes.md")), "text/plain");
        assert_eq!(mime_for_path(Path::new("/a/doc.pdf")), "application/pdf");
        assert_eq!(mime_for_path(Path::new("/a/b.zip")), "application/zip");
        assert_eq!(mime_for_path(Path::new("/a/b.tar.gz")), "application/x-archive");
        assert_eq!(mime_for_path(Path::new("/a/noext")), "application/octet-stream");
    }

    #[test]
    fn mime_wildcards() {
        assert!(mime_matches("image/png", "image/*"));
        assert!(mime_matches("image/*", "image/png"));
        assert!(mime_matches("text/plain", "text/plain"));
        assert!(!mime_matches("audio/mpeg", "image/*"));
        assert!(!mime_matches("application/pdf", "application/zip"));
    }

    #[test]
    fn default_app_roundtrip() {
        let path = defaults_path();
        // back up an existing user defaults file, restore it afterwards
        let orig = std::fs::read_to_string(&path).ok();
        let _ = std::fs::remove_file(&path);
        assert_eq!(default_app_for("image/png"), None);
        set_default_app("image/png", "feh");
        set_default_app("text/plain", "nvim");
        assert_eq!(default_app_for("image/png"), Some("feh".to_string()));
        assert_eq!(default_app_for("text/plain"), Some("nvim".to_string()));
        set_default_app("image/png", "swayimg"); // replace, not duplicate
        assert_eq!(default_app_for("image/png"), Some("swayimg".to_string()));
        assert_eq!(default_app_for("video/mp4"), None);
        match orig {
            Some(t) => std::fs::write(&path, t).ok(),
            None => std::fs::remove_file(&path).ok(),
        };
    }

    #[test]
    fn shell_quote_path_works() {
        assert_eq!(shell_quote_path(Path::new("/a b/c'd.txt")), "'/a b/c'\\''d.txt'");
    }

    #[test]
    fn boot_mounts_are_filtered() {
        assert!(is_boot_mount("/boot"));
        assert!(is_boot_mount("/boot/efi"));
        assert!(is_boot_mount("/efi"));
        assert!(is_boot_mount("/boot/grub"));
        assert!(!is_boot_mount("/"));
        assert!(!is_boot_mount("/home"));
        assert!(!is_boot_mount("/bootloader")); // prefix, not path segment
    }

    #[test]
    fn guid_mixed_endian_decodes() {
        // EFI System Partition, raw GPT bytes (LE first three fields)
        let esp: [u8; 16] = [
            0x28, 0x73, 0x2a, 0xc1, 0x1f, 0xf8, 0xd2, 0x11, 0xba, 0x4b, 0x00, 0xa0, 0xc9, 0x3e,
            0xc9, 0x3b,
        ];
        assert_eq!(
            guid_string(&esp),
            "c12a7328-f81f-11d2-ba4b-00a0c93ec93b"
        );
        assert!(HIDDEN_PART_GUIDS.contains(&"c12a7328-f81f-11d2-ba4b-00a0c93ec93b"));
        // Linux filesystem GUID should NOT be hidden
        let linux: [u8; 16] = [
            0xaf, 0x3d, 0xc6, 0x0f, 0x83, 0x84, 0x72, 0x47, 0x8e, 0x79, 0x3d, 0x69, 0xd8, 0x47,
            0x7d, 0xe4,
        ];
        assert_eq!(guid_string(&linux), "0fc63daf-8483-4772-8e79-3d69d8477de4");
        assert!(!HIDDEN_PART_GUIDS.contains(&"0fc63daf-8483-4772-8e79-3d69d8477de4"));
    }

    #[test]
    fn gpt_type_guid_reads_from_synthetic_disk() {
        use std::io::{Seek, SeekFrom, Write};
        let dir = std::env::temp_dir().join(format!("wfm-gpt-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let disk = dir.join("disk.img");
        let mut f = std::fs::File::create(&disk).unwrap();
        // zero-fill 3 sectors (MBR LBA0, header LBA1, entries LBA2)
        f.set_len(512 * 3).unwrap();
        // GPT header at LBA 1
        let mut hdr = [0u8; 92];
        hdr[0..8].copy_from_slice(b"EFI PART");
        hdr[72..80].copy_from_slice(&2u64.to_le_bytes()); // entries at LBA 2
        hdr[84..88].copy_from_slice(&128u32.to_le_bytes()); // entry size 128
        f.seek(SeekFrom::Start(512)).unwrap();
        f.write_all(&hdr).unwrap();
        // entry 1: ESP (hidden), entry 2: linux fs (shown)
        let esp: [u8; 16] = [
            0x28, 0x73, 0x2a, 0xc1, 0x1f, 0xf8, 0xd2, 0x11, 0xba, 0x4b, 0x00, 0xa0, 0xc9, 0x3e,
            0xc9, 0x3b,
        ];
        let linux: [u8; 16] = [
            0xaf, 0x3d, 0xc6, 0x0f, 0x83, 0x84, 0x72, 0x47, 0x8e, 0x79, 0x3d, 0x69, 0xd8, 0x47,
            0x7d, 0xe4,
        ];
        f.seek(SeekFrom::Start(512 * 2)).unwrap();
        f.write_all(&esp).unwrap();
        f.seek(SeekFrom::Start(512 * 2 + 128)).unwrap();
        f.write_all(&linux).unwrap();
        // partition names (UTF-16LE at entry bytes 56..128):
        // entry 1 "EFI system partition", entry 2 "common"
        let name_esp = b"EFI system partition";
        let mut name1 = [0u8; 128];
        for (i, ch) in name_esp.iter().enumerate() {
            name1[56 + i * 2] = *ch;
        }
        let name_linux = b"common";
        let mut name2 = [0u8; 128];
        for (i, ch) in name_linux.iter().enumerate() {
            name2[56 + i * 2] = *ch;
        }
        // names live at entry bytes 56..128; writing at the entry start
        // would clobber the type GUIDs at bytes 0..16
        f.seek(SeekFrom::Start(512 * 2 + 56)).unwrap();
        f.write_all(&name1[56..]).unwrap();
        f.seek(SeekFrom::Start(512 * 2 + 128 + 56)).unwrap();
        f.write_all(&name2[56..]).unwrap();
        drop(f);
        assert_eq!(
            read_partition_type_guid(&disk, 1, 512).as_deref(),
            Some("c12a7328-f81f-11d2-ba4b-00a0c93ec93b")
        );
        assert_eq!(
            read_partition_type_guid(&disk, 2, 512).as_deref(),
            Some("0fc63daf-8483-4772-8e79-3d69d8477de4")
        );
        assert_eq!(read_partition_type_guid(&disk, 3, 512), None); // empty slot
        assert_eq!(read_partition_type_guid(&disk, 0, 512), None);
        assert_eq!(
            read_partition_name(&disk, 1, 512).as_deref(),
            Some("EFI system partition")
        );
        assert_eq!(read_partition_name(&disk, 2, 512).as_deref(), Some("common"));
        assert_eq!(read_partition_name(&disk, 3, 512), None); // empty slot
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn type_sort_groups_by_extension() {
        fn mk(name: &str) -> Entry {
            Entry {
                name: name.to_string(),
                kind: EntryType::File,
                size: 0,
                mtime: 0,
                is_dir: false,
                is_link: false,
                is_hardlink: false,
                is_exec: false,
                real_path: None,
                thumb: None,
                thumb_w: 0,
                thumb_h: 0,
                thumb_pending: false,
                selected: false,
            }
        }
        let mut e = vec![mk("b.txt"), mk("a.rs"), mk("c.txt"), mk("z")];
        sort_entries(&mut e, true, SortKey::Type, false);
        let names: Vec<&str> = e.iter().map(|x| x.name.as_str()).collect();
        // "z" has no extension and sorts first among files by extension
        assert_eq!(names, vec!["z", "a.rs", "b.txt", "c.txt"]);
        // descending flips within the same extension class
        sort_entries(&mut e, true, SortKey::Type, true);
        let names: Vec<&str> = e.iter().map(|x| x.name.as_str()).collect();
        assert_eq!(names, vec!["c.txt", "b.txt", "a.rs", "z"]);
    }

    #[cfg(unix)]
    #[test]
    fn symlink_to_dir_shows_as_dir() {
        let dir = std::env::temp_dir().join(format!("wfm_symtest_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("real")).unwrap();
        std::os::unix::fs::symlink("real", dir.join("lnk")).unwrap();
        let mut entries = read_entries(&dir, true, false, SortKey::Name, false);
        entries.retain(|e| e.name == "lnk");
        let lnk = entries.pop().expect("symlink entry present");
        assert!(lnk.is_link);
        assert!(lnk.is_dir, "symlink to a dir must show as a directory");
        // and it sorts with the real directories (dirs-first)
        let all = read_entries(&dir, true, false, SortKey::Name, false);
        let first = &all[0];
        assert!(first.is_dir, "dirs-first: symlink-to-dir comes before files");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn mimeinfo_cache_parsing() {
        let cache = parse_mimeinfo_cache(
            "[MIME Cache]\ntext/plain=gedit.desktop;kate.desktop;\nimage/png=swayimg.desktop;\n[Groups]\nfoo=bar;\n",
        );
        assert_eq!(cache.get("text/plain").map(|v| v.len()), Some(2));
        assert_eq!(cache.get("image/png").map(|v| v.as_slice()), Some(&["swayimg.desktop".to_string()][..]));
        assert!(cache.get("foo").is_none(), "group sections are ignored");
    }
}
