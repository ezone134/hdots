//! zen-shell — zero-overhead Wayland shell bar in Rust.
//!
//! Event-driven: epoll sleeps on the wayland fd when idle (0% CPU), draws
//! only on configure acks, and morphs with ack-gated eased size commits.

mod app;
mod apps;
mod auth;
mod backends;
mod cliphist;
mod clipboard;
mod config;
mod eq;
mod hypr;
mod icons;
mod img;
mod ipc;
mod lyrics;
mod news;
mod polkit;
mod render;
#[cfg(feature = "vk")]
mod render_vk;
mod sensors;
mod services;
mod shell;
mod stats;
mod text;
mod themes;
mod tray;
mod ui;
mod vars;
mod viz;
mod weather;

use app::App;
use config::Config;
use std::path::Path;

/// Refuse to start if another zen-shell is already running. Two bars at the
/// same anchor overlap and steal pointer focus from each other — the one that
/// loses focus can get stuck expanded ("bar stays at the dashboard size when
/// not hovered"). A stale lock file is fine: flock is released on exit.
fn claim_single_instance() {
    let lock_path = std::env::var("XDG_RUNTIME_DIR")
        .map(|d| Path::new(&d).join("zen-shell.lock"))
        .unwrap_or_else(|_| Path::new("/tmp/zen-shell.lock").to_path_buf());
    let Ok(file) = std::fs::OpenOptions::new().create(true).write(true).open(&lock_path) else {
        eprintln!("zen-shell: cannot open lock {} — continuing without single-instance guard", lock_path.display());
        return;
    };
    use std::os::fd::AsRawFd;
    if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
        eprintln!("zen-shell: another instance is already running (lock {}); exiting", lock_path.display());
        std::process::exit(0);
    }
    // Keep the fd open for the process lifetime so the lock is held.
    std::mem::forget(file);
}

/// Install a panic hook that appends the message + location + backtrace to
/// `$states2/zen-shell-panic.log`. Launched by the compositor our stderr is
/// `/dev/null`, so a panic would otherwise be invisible — the watchdog would
/// just quietly restart us. The default hook still runs too (for manual runs
/// that have a real stderr). The hook itself must never panic (a panic inside
/// the hook aborts the process): every fallible call is ignored.
fn install_panic_hook() {
    use std::io::Write;
    use std::backtrace::Backtrace;
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        default(info);
        let loc = info.location().map(|l| l.to_string()).unwrap_or_default();
        let msg = if let Some(s) = info.payload().downcast_ref::<&str>() {
            (*s).to_string()
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "non-string panic payload".to_string()
        };
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs().to_string())
            .unwrap_or_default();
        let path = vars::states2_dir().join("zen-shell-panic.log");
        let entry = format!(
            "=== zen-shell panic {now} ===\nlocation: {loc}\nmessage: {msg}\nbacktrace:\n{backtrace}\n",
            backtrace = Backtrace::force_capture(),
        );
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
            let _ = f.write_all(entry.as_bytes());
        }
    }));
}

fn main() {
    // Catch panics on the live bar (stderr is /dev/null under the compositor)
    // BEFORE the single-instance lock / IPC client dispatch.
    install_panic_hook();
    // `zen-shell open launcher` / `zen-shell ipc call …` → talk to a running
    // bar (quickshell's `qs ipc call` equivalent); anything else starts the
    // bar itself. Runs before the single-instance lock: a caller poking the
    // bar must not need the bar's own lock.
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Some(code) = ipc::try_client(&args) {
        std::process::exit(code);
    }
    claim_single_instance();
    // Master config = `$states/shell_{channel}` (channel read from
    // `$states2/s`), the file `sync_back` mirrors to `$hdots/states/` at
    // session end. config/shell.toml is NEVER read — on first run when the
    // per-state file is missing, fall back to built-in defaults; the first
    // setting change writes the file out.
    let cfg = {
        let master = vars::per_state_shell_path(vars::read_channel());
        if master.exists() {
            Config::load(&master)
        } else {
            Config::default()
        }
    };
    if let Err(e) = App::run(cfg) {
        eprintln!("zen-shell: {e}");
        std::process::exit(1);
    }
}
