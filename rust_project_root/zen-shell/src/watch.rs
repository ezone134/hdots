//! File watcher — the 0-CPU hot-reload pipe (no hooks, no stat loops).
//!
//! A background thread owns an inotify instance and BLOCKS on `read()`: the
//! kernel suspends it until a watched file changes, so idle cost is exactly
//! zero — the same mechanism the wayland fd sleeps on. Every real file event
//! maps to ONE calloop wake in the main loop:
//!
//! | file                              | action on change                    |
//! |-----------------------------------|-------------------------------------|
//! | `$states/shell_<ch>` (master cfg) | full `reload_config()`              |
//! | `$states2/s` (channel)            | full `reload_config()` (adopt)      |
//! | `$states2/shell_vars.json`        | palette-only `colors_reload()`      |
//! | `$states2/shell_vars` (legacy)    | palette-only `colors_reload()`      |
//! | `$states2/m_dummy` (dark/light)   | `colors_reload()` (dark sync)       |
//!
//! This replaces the external hook pipeline (`theme_save_hex` having to call
//! `zen-shell ipc call colors reload` etc.): SAVING the file IS the trigger
//! now. Files written straight (`>` truncate) are caught by the file's
//! `IN_CLOSE_WRITE`; atomic writers (tmp + rename) are caught on the PARENT
//! dir watch (`IN_CREATE`/`IN_MOVED_TO` re-arms the file wd, then fires). A
//! 200 ms per-kind debounce absorbs editor double-writes and rep bursts.
//!
//! Deliberate choices:
//! - std::thread + blocking read, NOT a calloop Generic fd source: the
//!   thread either blocks or sleeps forever — no polling, no wakeups, no
//!   rearm bookkeeping; the process exit closes the fd and kills it.
//! - Watch by FILE (not the whole `$states2/` dir traffic): the volatile
//!   state dir churns constantly (viz dumps, lyric caches, watch heartbeat);
//!   file wds only ever fire for the files we reload on. Dir wds exist just
//!   to re-arm file wds after replace/unlink — they fire nothing extra.
//! - Watched names are filtered on dir events: `$states/shell_*` saves for a
//!   DIFFERENT channel don't wake the reload (the channel file `s` owns
//!   adoption; its change triggers a full `reload_config()` and the watcher
//!   re-syncs its master target to the new `shell_<ch>`).

use std::collections::HashMap;
use std::ffi::{CString, OsString};
use std::os::unix::ffi::OsStrExt;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use crate::vars;

/// What a watched file maps to when it changes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    /// full config reload (`reload_config`) — master `shell_<ch>` or `s`
    Config,
    /// palette + dark/light sync (`colors_reload`) — shell_vars(.json)/m_dummy
    Colors,
}

const SUPPRESS: Duration = Duration::from_millis(200);

/// inotify masks: file wd reacts to real saves; dir wd only re-arms file wds.
const FILE_MASK: u32 = libc::IN_CLOSE_WRITE | libc::IN_ATTRIB | libc::IN_DELETE_SELF | libc::IN_MOVE_SELF;
const DIR_MASK: u32 = libc::IN_CREATE | libc::IN_MOVED_TO | libc::IN_DELETE | libc::IN_IGNORED;

/// One watched descriptor.
struct Wd {
    path: PathBuf,
    is_dir: bool,
}

/// The inotify handle owned by the watch thread.
pub struct Watcher {
    fd: i32,
    /// wd → what it watches
    wds: HashMap<i32, Wd>,
    /// watched files (target → action kind)
    files: HashMap<PathBuf, Kind>,
    /// watched parent dirs (for replace/unlink re-arm)
    dirs: HashMap<PathBuf, ()>,
    /// debounce state: last real fire per kind
    last: HashMap<Kind, Instant>,
}

impl Watcher {
    /// Blocking inotify instance (no IN_NONBLOCK: `read()` is the idle state).
    pub fn new() -> std::io::Result<Watcher> {
        let fd = unsafe { libc::inotify_init1(libc::IN_CLOEXEC) };
        if fd < 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(Watcher {
            fd,
            wds: HashMap::new(),
            files: HashMap::new(),
            dirs: HashMap::new(),
            last: HashMap::new(),
        })
    }

    /// (re)point the watch set at the CURRENT channel + the static $states2
    /// files. Called at thread start and after every Config fire (a channel
    /// swap moves the master-config target).
    pub fn sync(&mut self) {
        let s2 = vars::states2_dir();
        let mut want: Vec<(PathBuf, Kind)> = vec![
            (s2.join("s"), Kind::Config),
            (s2.join("shell_vars.json"), Kind::Colors),
            (s2.join("shell_vars"), Kind::Colors),
            (s2.join("m_dummy"), Kind::Colors),
        ];
        want.push((
            vars::per_state_shell_path(vars::read_channel()),
            Kind::Config,
        ));

        // unique parent dirs of all targets
        let mut dirs: HashMap<PathBuf, ()> = HashMap::new();
        for (t, _) in &want {
            if let Some(d) = t.parent() {
                dirs.insert(d.to_path_buf(), ());
            }
        }

        // drop file watches that aren't targets anymore
        let stale_files: Vec<PathBuf> = self
            .files
            .keys()
            .filter(|p| !want.iter().any(|(t, _)| t == *p))
            .cloned()
            .collect();
        for p in stale_files {
            self.rm_watch_file(&p);
        }
        // drop dir watches whose dir isn't referenced anymore
        let stale_dirs: Vec<PathBuf> = self
            .dirs
            .keys()
            .filter(|d| !dirs.contains_key(*d))
            .cloned()
            .collect();
        for d in stale_dirs {
            self.rm_watch_dir(&d);
        }

        self.files = want.iter().cloned().collect();
        self.dirs = dirs;
        // arm everything (idempotent: inotify reuses an existing wd per path)
        for (p, _) in &want {
            self.add_file_watch(p);
        }
        let dir_list: Vec<PathBuf> = self.dirs.keys().cloned().collect();
        for d in dir_list {
            self.add_dir_watch(&d);
        }
    }

    /// Block until real, DEBOUNCED events arrive. Returns the kinds to apply;
    /// empty list = nothing worth waking the shell for (caller keeps looping).
    pub fn blocking_all(&mut self) -> Vec<Kind> {
        loop {
            let raw = self.read_batch();
            if raw.is_empty() {
                continue; // transient read error — sleep happened inside
            }
            let mut out = Vec::with_capacity(raw.len());
            for kind in raw {
                let now = Instant::now();
                let recent = self
                    .last
                    .get(&kind)
                    .is_some_and(|t| now.duration_since(*t) < SUPPRESS);
                if recent {
                    continue;
                }
                self.last.insert(kind, now);
                out.push(kind);
            }
            if !out.is_empty() {
                return out;
            }
        }
    }

    // ── read + parse ────────────────────────────────────────────────────────

    /// One blocking read; parse ALL events in the batch. Returns the pre-
    /// debounce kinds (may contain one kind several times; the debounce in
    /// `blocking_all` collapses them).
    fn read_batch(&mut self) -> Vec<Kind> {
        // 8 KiB, u64-typed so the base is 8-aligned (inotify needs ≥ align 4)
        let mut buf = vec![0u64; 1024];
        let base = buf.as_mut_ptr().cast::<u8>();
        let cap = buf.len() * 8;
        let n = loop {
            let r = unsafe { libc::read(self.fd, base.cast::<libc::c_void>(), cap) };
            if r > 0 {
                break r as usize;
            }
            let err = std::io::Error::last_os_error();
            if err.raw_os_error() == Some(libc::EINTR) {
                continue;
            }
            eprintln!("zen: file watcher read failed: {err}");
            std::thread::sleep(Duration::from_secs(1));
            return Vec::new();
        };
        let hdr = std::mem::size_of::<libc::inotify_event>();
        if n < hdr {
            return Vec::new();
        }
        let data = unsafe { std::slice::from_raw_parts(base, n) };
        let mut off = 0usize;
        let mut fires = Vec::new();
        while off + hdr <= n {
            let ev = &unsafe { &*(data.as_ptr().add(off) as *const libc::inotify_event) };
            let (wd, mask, elen) = (ev.wd, ev.mask, ev.len as usize);
            off += hdr;
            let name = if elen > 0 && off + elen <= n {
                let nb = &data[off..off + elen];
                let s = nb.split(|&b| b == 0).next().unwrap_or(&[]);
                Some(OsString::from(std::ffi::OsStr::from_bytes(s)))
            } else {
                None
            };
            off += elen;
            self.handle(wd, mask, name.as_ref(), &mut fires);
        }
        fires
    }

    /// Route one inotify event to the firing list.
    fn handle(&mut self, wd: i32, mask: u32, name: Option<&OsString>, out: &mut Vec<Kind>) {
        if mask & libc::IN_Q_OVERFLOW != 0 {
            // the kernel dropped some events — apply everything to re-sync
            out.push(Kind::Config);
            out.push(Kind::Colors);
            return;
        }
        let Some(w) = self.wds.get(&wd) else { return };
        let w = (w.path.clone(), w.is_dir);

        if mask & libc::IN_IGNORED != 0 {
            self.wds.remove(&wd);
            // re-arm in place: the watch target still exists (this fires after
            // unlink/replace of a file/dir wd whose path is still ours)
            if w.1 {
                self.add_dir_watch(&w.0);
            } else {
                self.add_file_watch(&w.0);
            }
            return;
        }

        if w.1 {
            // PARENT dir event — atomic replace (tmp + rename) lands here.
            // These events carry a name; re-arm the file wd and fire only if
            // the name is one of our targets.
            if mask & (libc::IN_CREATE | libc::IN_MOVED_TO | libc::IN_DELETE) != 0 {
                let Some(n) = name else { return };
                let hits: Vec<(PathBuf, Kind)> = self
                    .files
                    .iter()
                    .filter(|(t, _)| {
                        t.file_name() == Some(n.as_os_str()) && t.parent() == Some(w.0.as_path())
                    })
                    .map(|(p, k)| (p.clone(), *k))
                    .collect();
                for (t, kind) in hits {
                    if mask & libc::IN_DELETE == 0 {
                        self.add_file_watch(&t);
                    }
                    out.push(kind);
                }
            }
        } else if mask & (libc::IN_CLOSE_WRITE | libc::IN_ATTRIB) != 0 {
            // plain save (`>` truncate / in-place rewrite) — the file wd owns it
            if let Some(kind) = self.files.get(&w.0) {
                out.push(*kind);
            }
        }
    }

    // ── watch plumbing ──────────────────────────────────────────────────────

    fn add_file_watch(&mut self, path: &std::path::Path) {
        self.add_watch(path, FILE_MASK, false);
    }

    fn add_dir_watch(&mut self, path: &std::path::Path) {
        self.add_watch(path, DIR_MASK, true);
    }

    fn add_watch(&mut self, path: &std::path::Path, mask: u32, is_dir: bool) {
        if !std::fs::metadata(path)
            .map(|m| if is_dir { m.is_dir() } else { m.is_file() })
            .unwrap_or(false)
        {
            return; // not there yet — the parent dir watch covers emergence
        }
        let Ok(c) = CString::new(path.as_os_str().as_bytes()) else {
            return;
        };
        // SAFETY: fd valid (owned for the thread's life), c NUL-terminated
        let wd = unsafe { libc::inotify_add_watch(self.fd, c.as_ptr(), mask) };
        if wd <= 0 {
            return;
        }
        // one wd per inode path each call — store the mapping (duplicate adds
        // for the same path re-use the same wd, so no leak)
        self.wds.insert(
            wd,
            Wd {
                path: path.to_path_buf(),
                is_dir,
            },
        );
    }

    fn rm_watch_file(&mut self, path: &std::path::Path) {
        self.rm_watch(path, false);
    }

    fn rm_watch_dir(&mut self, path: &std::path::Path) {
        self.rm_watch(path, true);
    }

    fn rm_watch(&mut self, path: &std::path::Path, is_dir: bool) {
        let victims: Vec<i32> = self
            .wds
            .iter()
            .filter(|(_, w)| w.path == path && w.is_dir == is_dir)
            .map(|(wd, _)| *wd)
            .collect();
        for wd in victims {
            unsafe { libc::inotify_rm_watch(self.fd, wd) };
            self.wds.remove(&wd);
        }
    }
}

impl Drop for Watcher {
    fn drop(&mut self) {
        unsafe { libc::close(self.fd) };
    }
}
