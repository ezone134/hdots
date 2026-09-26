//! Minimal read-only parser for cliphist's database (a bbolt file).
//!
//! cliphist stores its history in `go.etcd.io/bbolt`: a single bucket `b`
//! mapping an 8-byte big-endian sequence id to the raw clipboard bytes.
//! Reading it here (instead of shelling out to `cliphist`) keeps the bar
//! self-contained — this module walks the B+tree directly.
//!
//! On-disk layout (matches bbolt's refactored `internal/common` page format,
//! which the shipped cliphist uses — verified against real dbs):
//!   - every page: 16-byte header `{id u64, flags u16, count u16, overflow
//!     u32}` followed immediately by `count` 16-byte elements, then the
//!     key/value bytes written contiguously after them.
//!   - leaf element   = `{flags u32, pos u32, ksize u32, vsize u32}`
//!   - branch element = `{pos u32, ksize u32, pgid u64}`
//!   - `pos` is relative to the element's own start; the key is at
//!     `element + pos`, the value right after it.
//!   - meta lives in page 0 (offset 0) and page 1 (offset pageSize): a
//!     16-byte page header + a 64-byte meta struct; the meta with the
//!     higher txid is current.
//!   - values may span page boundaries (overflow) — the bytes are stored
//!     contiguously, so a flat slice read handles it.

use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

/// bbolt file magic (`0xED0CDAED`).
const MAGIC: u32 = 0xED0CDAED;
const BRANCH_FLAG: u16 = 0x01;
const LEAF_FLAG: u16 = 0x02;

/// cliphist's text db: `$XDG_CACHE_HOME/cliphist/db`.
pub fn text_db_path() -> PathBuf {
    cache_dir().join("cliphist").join("db")
}

/// The image db used by this setup (`cliphist -db-path … store` watchers):
/// `$XDG_CACHE_HOME/cliphist_image/db` (a separate db for image entries).
pub fn image_db_path() -> PathBuf {
    cache_dir().join("cliphist_image").join("db")
}

fn cache_dir() -> PathBuf {
    std::env::var("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|_| std::env::var("HOME").map(|h| PathBuf::from(h).join(".cache")))
        .unwrap_or_else(|_| PathBuf::from("/tmp"))
}

/// One clipboard-history entry: its sequence id + the raw clipboard bytes.
pub struct Entry {
    pub id: u64,
    pub data: Vec<u8>,
}

/// Read a cliphist (bbolt) db, newest entry first. `None` if the file is
/// missing, unreadable, or not a bbolt db.
pub fn read_db(path: &Path) -> Option<Vec<Entry>> {
    let mut f = File::open(path).ok()?;
    let mut data = Vec::new();
    f.read_to_end(&mut data).ok()?;
    if data.len() < 4096 * 2 {
        return None;
    }

    // meta page 0 is at offset 0; page 1 at offset pageSize. Pick the newer.
    let (page_size, root0, txid0) = parse_meta(&data, 0)?;
    let (root, _txid) = match parse_meta(&data, page_size) {
        Some((_, r, t)) if t > txid0 => (r, t),
        _ => (root0, txid0),
    };

    // the top-level bucket holds one entry: key "b" → the nested bucket,
    // either a root pgid or an inline bucket (root == 0 → the page is
    // embedded in the bucket value).
    let mut pairs: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();
    walk(&data, page_size, root, &mut pairs)?;
    let mut bucket_root = None;
    let mut inline = None;
    for (k, v) in pairs {
        if k == b"b" {
            let r = u64::from_le_bytes(v.get(..8)?.try_into().ok()?);
            if r == 0 {
                inline = Some(v);
            } else {
                bucket_root = Some(r);
            }
        }
    }

    let mut out = Vec::new();
    if let Some(root) = bucket_root {
        let mut pairs = Vec::new();
        walk(&data, page_size, root, &mut pairs)?;
        for (k, v) in pairs {
            if let Ok(id) = <[u8; 8]>::try_from(&k[..]) {
                out.push(Entry { id: u64::from_be_bytes(id), data: v });
            }
        }
    } else if let Some(v) = inline {
        walk_inline(&v, &mut out);
    }
    out.sort_by(|a, b| b.id.cmp(&a.id));
    Some(out)
}

/// Parse a meta page (the 64-byte meta struct sits right after the page
/// header) → (pageSize, root pgid, txid).
fn parse_meta(data: &[u8], page_off: usize) -> Option<(usize, u64, u64)> {
    let o = page_off + 16;
    if data.get(o..o + 72).is_none() || u32::from_le_bytes(data.get(o..o + 4)?.try_into().ok()?) != MAGIC {
        return None;
    }
    let page_size = u32::from_le_bytes(data.get(o + 8..o + 12)?.try_into().ok()?) as usize;
    let root = u64::from_le_bytes(data.get(o + 16..o + 24)?.try_into().ok()?);
    let txid = u64::from_le_bytes(data.get(o + 48..o + 56)?.try_into().ok()?);
    Some((page_size, root, txid))
}

/// Walk a B+tree of regular pages, collecting raw (key, value) pairs.
fn walk(data: &[u8], page_size: usize, pgid: u64, out: &mut Vec<(Vec<u8>, Vec<u8>)>) -> Option<()> {
    let off = pgid as usize * page_size;
    let hdr = data.get(off..off + 16)?;
    let flags = u16::from_le_bytes([hdr[8], hdr[9]]);
    let count = u16::from_le_bytes([hdr[10], hdr[11]]) as usize;
    if flags & BRANCH_FLAG != 0 {
        for i in 0..count {
            let e = off + 16 + i * 16;
            let _pos = u32::from_le_bytes(data.get(e..e + 4)?.try_into().ok()?) as usize;
            let _ksize = u32::from_le_bytes(data.get(e + 4..e + 8)?.try_into().ok()?) as usize;
            let child = u64::from_le_bytes(data.get(e + 8..e + 16)?.try_into().ok()?);
            walk(data, page_size, child, out)?;
        }
    } else if flags & LEAF_FLAG != 0 {
        for i in 0..count {
            let e = off + 16 + i * 16;
            // leaf element = {flags u32, pos u32, ksize u32, vsize u32}
            let _lflags = u32::from_le_bytes(data.get(e..e + 4)?.try_into().ok()?);
            let pos = u32::from_le_bytes(data.get(e + 4..e + 8)?.try_into().ok()?) as usize;
            let ksize = u32::from_le_bytes(data.get(e + 8..e + 12)?.try_into().ok()?) as usize;
            let vsize = u32::from_le_bytes(data.get(e + 12..e + 16)?.try_into().ok()?) as usize;
            let key = data.get(e + pos..e + pos + ksize)?.to_vec();
            let val = data.get(e + pos + ksize..e + pos + ksize + vsize)?.to_vec();
            out.push((key, val));
        }
    }
    Some(())
}

/// Walk an inline bucket: the whole (small) page lives inside the parent
/// leaf's value — page header at offset 16, elements after it.
fn walk_inline(v: &[u8], out: &mut Vec<Entry>) {
    let Some(hdr) = v.get(16..32) else { return };
    let flags = u16::from_le_bytes([hdr[8], hdr[9]]);
    if flags & LEAF_FLAG == 0 {
        return;
    }
    let count = u16::from_le_bytes([hdr[10], hdr[11]]) as usize;
    for i in 0..count {
        let e = 16 + 16 + i * 16;
        if e + 16 > v.len() {
            return;
        }
        // leaf element = {flags u32, pos u32, ksize u32, vsize u32}
        let _lflags = u32::from_le_bytes(<[u8; 4]>::try_from(&v[e..e + 4]).unwrap_or([0; 4]));
        let pos = u32::from_le_bytes(<[u8; 4]>::try_from(&v[e + 4..e + 8]).unwrap_or([0; 4])) as usize;
        let ksize = u32::from_le_bytes(<[u8; 4]>::try_from(&v[e + 8..e + 12]).unwrap_or([0; 4])) as usize;
        let vsize = u32::from_le_bytes(<[u8; 4]>::try_from(&v[e + 12..e + 16]).unwrap_or([0; 4])) as usize;
        let (Some(key), Some(val)) = (
            v.get(e + pos..e + pos + ksize),
            v.get(e + pos + ksize..e + pos + ksize + vsize),
        ) else {
            continue;
        };
        if key.len() == 8 {
            let id = u64::from_be_bytes(<[u8; 8]>::try_from(&key[..]).unwrap_or([0; 8]));
            out.push(Entry { id, data: val.to_vec() });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timing_boot_heavy_reads() {
        let t0 = std::time::Instant::now();
        let entries = read_db(&text_db_path());
        println!("cliphist text db: {:?} → {:?} entries in {:?}", text_db_path(), entries.as_ref().map(|e| e.len()), t0.elapsed());
        let mut imgs = 0usize;
        let mut total = 0usize;
        let t1 = std::time::Instant::now();
        if let Some(es) = entries {
            for e in es {
                // mimic put_thumbnail: load + thumbnail(64)
                if let Ok(im) = image::load_from_memory(&e.data) {
                    let im = im.thumbnail(64, 64).to_rgba8();
                    total += im.len();
                    imgs += 1;
                }
            }
        }
        println!("rasterized {imgs} images ({} bytes) in {:?}", total, t1.elapsed());
        let _ = read_db(&image_db_path());
        println!("image db read done");
    }

    /// Smoke-test against the real cliphist dbs when present (this machine
    /// has them; elsewhere they're skipped).
    #[test]
    fn reads_real_cliphist_dbs() {
        let mut checked = 0;
        for (p, must_have) in [(text_db_path(), true), (image_db_path(), false)] {
            let Some(entries) = read_db(&p) else { continue };
            checked += 1;
            if must_have {
                assert!(!entries.is_empty(), "{} read 0 entries", p.display());
            }
            // newest first
            for w in entries.windows(2) {
                assert!(w[0].id >= w[1].id, "{} not newest-first", p.display());
            }
            // entries carry data
            for e in entries.iter().take(3) {
                assert!(!e.data.is_empty(), "{} entry {} empty", p.display(), e.id);
            }
            eprintln!("cliphist: {} → {} entries", p.display(), entries.len());
        }
        // at least the text db should exist on a machine that runs cliphist
        assert!(checked >= 1, "no cliphist dbs found — nothing to validate");
    }
}
