//! worker.rs — background thread for directory listing and image thumbnail
//! decoding so the UI thread never blocks on disk I/O.
//!
//! The main thread enqueues [`Job`]s and the worker pushes [`WResult`]s back
//! through a calloop channel that is drained each event loop iteration.

use std::path::{Path, PathBuf};
use std::sync::mpsc;

use crate::entry::Entry;
use crate::fs;
use crate::tab::SortKey;

pub enum Job {
    List {
        tab_id: i32,
        cwd: PathBuf,
        dirs_first: bool,
        show_hidden: bool,
        sort_key: SortKey,
        sort_desc: bool,
    },
    Thumb {
        tab_id: i32,
        idx: usize,
        path: PathBuf,
        name: String,
        box_size: u32,
    },
}

pub enum WResult {
    List {
        tab_id: i32,
        cwd: PathBuf,
        entries: Vec<Entry>,
    },
    Thumb {
        tab_id: i32,
        idx: usize,
        name: String,
        rgba: Vec<u8>,
        w: u32,
        h: u32,
    },
}

/// Result of an asynchronous checksum computation for the properties dialog.
pub struct ChecksumResult {
    pub path: String,
    pub md5: String,
    pub sha1: String,
    pub sha256: String,
}

pub struct Worker {
    pub res_rx: calloop::channel::Channel<WResult>,
}

impl Worker {
    pub fn new() -> (Worker, mpsc::Sender<Job>) {
        let (job_tx, job_rx) = mpsc::channel::<Job>();
        let (res_tx, res_rx) = calloop::channel::channel::<WResult>();
        std::thread::Builder::new()
            .name("wfm-worker".into())
            .spawn(move || {
                while let Ok(job) = job_rx.recv() {
                    match job {
                        Job::List {
                            tab_id,
                            cwd,
                            dirs_first,
                            show_hidden,
                            sort_key,
                            sort_desc,
                        } => {
                            let entries = fs::read_entries(
                                &cwd,
                                dirs_first,
                                show_hidden,
                                sort_key,
                                sort_desc,
                            );
                            let _ = res_tx.send(WResult::List { tab_id, cwd, entries });
                        }
                        Job::Thumb { tab_id, idx, path, name, box_size } => {
                            if let Some((rgba, w, h)) = make_thumb(&path, box_size) {
                                let _ = res_tx.send(WResult::Thumb {
                                    tab_id,
                                    idx,
                                    name,
                                    rgba,
                                    w,
                                    h,
                                });
                            }
                        }
                    }
                }
            })
            .expect("failed to spawn wfm worker thread");
        let worker = Worker { res_rx };
        (worker, job_tx)
    }
}

fn cache_dir() -> PathBuf {
    let base = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))
        .unwrap_or_else(|| PathBuf::from("/tmp"));
    let dir = base.join("wfm/thumbs");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

fn cache_key(path: &Path, mtime: i64, box_size: u32) -> String {
    let mut h: u64 = 14695981039346656037;
    for b in path.to_string_lossy().bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(1099511628211);
    }
    h ^= (mtime as u64).wrapping_mul(0x9E3779B97F4A7C15);
    h ^= box_size as u64;
    format!("{:016x}", h)
}

/// Decode + scale an image to fit `box_size` px, with a simple disk cache
/// keyed on (path, mtime, size). Port of the C thumbnail worker.
fn make_thumb(path: &Path, box_size: u32) -> Option<(Vec<u8>, u32, u32)> {
    let meta = std::fs::metadata(path).ok()?;
    let mtime = meta.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs() as i64;
    let key = cache_key(path, mtime, box_size);
    let file = cache_dir().join(format!("{key}.rgba"));
    if let Ok(raw) = std::fs::read(&file) {
        if raw.len() >= 8 {
            let w = u32::from_le_bytes(raw[0..4].try_into().ok()?);
            let h = u32::from_le_bytes(raw[4..8].try_into().ok()?);
            if raw.len() == 8 + (w as usize) * (h as usize) * 4 {
                return Some((raw[8..].to_vec(), w, h));
            }
        }
    }

    let img = image::ImageReader::open(path).ok()?.with_guessed_format().ok()?.decode().ok()?;
    let (w, h) = (img.width(), img.height());
    if w == 0 || h == 0 {
        return None;
    }
    let (tw, th) = if w >= h {
        (box_size, (box_size * h) / w)
    } else {
        ((box_size * w) / h, box_size)
    };
    let tw = tw.max(1);
    let th = th.max(1);
    let thumb = img.thumbnail(tw, th);
    let rgba = thumb.to_rgba8();
    let (w, h) = (rgba.width(), rgba.height());
    let mut out = Vec::with_capacity(8 + w as usize * h as usize * 4);
    out.extend_from_slice(&w.to_le_bytes());
    out.extend_from_slice(&h.to_le_bytes());
    out.extend_from_slice(&rgba);
    let _ = std::fs::write(&file, &out);
    Some((rgba.into_raw(), w, h))
}
