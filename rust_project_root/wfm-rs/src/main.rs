mod actions;
mod app;
mod config;
mod entry;
mod fs;
mod icons;
mod layout;
mod menu;
mod render;
mod tab;
mod text;
mod theme;
mod toolbar;
mod wl;
mod worker;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use app::{App, InputMode};

const VERSION: &str = "0.5.0";

/// Set by the SIGUSR1 handler; the event loop picks it up and hot-reloads.
static RELOAD: AtomicBool = AtomicBool::new(false);

extern "C" fn on_sigusr1(_: libc::c_int) {
    RELOAD.store(true, Ordering::SeqCst);
}

pub fn install_reload_handler() {
    unsafe {
        let mut sa: libc::sigaction = std::mem::zeroed();
        sa.sa_sigaction = (on_sigusr1 as extern "C" fn(libc::c_int)) as usize;
        sa.sa_flags = libc::SA_RESTART;
        libc::sigaction(libc::SIGUSR1, &sa, std::ptr::null_mut());
    }
}

pub fn reload_requested() -> bool {
    RELOAD.swap(false, Ordering::SeqCst)
}

fn usage() {
    eprintln!(
        "wfm {} — pure Wayland file manager (Rust)\n\
         usage: wfm [options] [DIR]\n\
         options:\n\
           --open DIR     open DIR (same as a positional DIR)\n\
           --new-window   force a fresh window (used by Ctrl+N)\n\
           --daemon       start in the background (control socket)\n\
           --ping         check the daemon is alive\n\
           --quit         ask the daemon to exit\n\
           --config PATH  use an explicit config file\n\
           --theme NAME   theme: `dark`, `light`, or a theme-file path\n\
           --reload       hot-reload config/theme in running wfm instance(s)\n\
           -h, --help     this help",
        VERSION
    );
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    let mut start_dir: Option<PathBuf> = None;
    let mut want_daemon = false;
    let mut force_new = false;
    let mut cfg_given = false;
    let mut cfg_path = config::config_path_default();
    let mut theme_override: Option<String> = None;

    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        match a.as_str() {
            "-h" | "--help" => {
                usage();
                return;
            }
            "--daemon" => want_daemon = true,
            "--new-window" => force_new = true,
            "--open" => {
                if let Some(d) = args.get(i + 1) {
                    start_dir = Some(PathBuf::from(d));
                    i += 1;
                }
            }
            "--config" => {
                if let Some(c) = args.get(i + 1) {
                    cfg_path = PathBuf::from(c);
                    cfg_given = true;
                    i += 1;
                }
            }
            "--theme" => {
                if let Some(t) = args.get(i + 1) {
                    theme_override = Some(t.clone());
                    i += 1;
                }
            }
            "--ping" => {
                std::process::exit(daemon_ping());
            }
            "--quit" => {
                std::process::exit(daemon_quit());
            }
            "--reload" => {
                std::process::exit(reload_instances());
            }
            _ if a.starts_with("--") => {
                eprintln!("wfm: unknown option {a}");
                usage();
                std::process::exit(2);
            }
            _ => start_dir = Some(PathBuf::from(a)),
        }
        i += 1;
    }

    // single-instance forward (unless daemon/new-window/explicit config)
    let dir_arg = start_dir.clone().unwrap_or_else(home_dir);
    if !cfg_given && !want_daemon && !force_new && daemon_alive() {
        let _ = std::process::Command::new("sh")
            .arg("-c")
            .arg(format!("printf 'cd {}' | nc -U {}", shell_quote(&dir_arg), daemon_socket().display()))
            .status();
        return;
    }

    let mut cfg = config::Config::default();
    config::load(&mut cfg, &cfg_path);
    // surface optional font keys in the config file (empty font_name = the
    // internal system-default fallback chain; font_size default 15)
    config::ensure_keys(&cfg_path, &[
        ("font_name", String::new()),
        ("font_size", cfg.font_size.to_string()),
        (
            "toolbar",
            crate::toolbar::toolbar_to_str(&cfg.toolbar),
        ),
        ("toolbar_hidden", String::new()),
        ("show_ext", "true".to_string()),
    ]);
    if let Some(t) = theme_override {
        cfg.theme = t;
    }
    theme::resolve(&cfg.theme).apply_to_cfg(&mut cfg);

    let mut app = App::new(cfg);
    app.cfg_path = Some(cfg_path.clone());

    // restore session state (view/sort/panels/last dir)
    let state = config::State::load(&config::state_path());
    if start_dir.is_none() {
        if let Some(ld) = &state.last_dir {
            if ld.is_dir() {
                start_dir = Some(ld.clone());
            }
        }
    }
    let start_dir = start_dir.unwrap_or_else(home_dir);
    app.rebuild_text();
    app.load_bookmarks();
    let mut tab = tab::Tab::new(0, start_dir.clone());
    tab.view = app.cfg.view; // config `view` key is the default; state overrides below
    app.tabs.push(tab);
    app.apply_state(&state);
    app.cd(start_dir.clone());
    // a restored split pane is embedded in the tab; load its listing too
    if let Some(pane_id) = app.tab().pane.as_ref().map(|p| p.id) {
        app.reload_pane_id(pane_id);
    }

    if app.input_mode != InputMode::None {
        app.input_mode = InputMode::None;
    }

    let title = format!("wfm — {}", start_dir.display());
    write_pidfile();
    let _ = wl::run(app, &title);
    remove_pidfile();
}

fn home_dir() -> PathBuf {
    std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/"))
}

fn daemon_socket() -> PathBuf {
    let rt = std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/tmp"));
    rt.join("wfm.sock")
}

fn daemon_alive() -> bool {
    std::os::unix::net::UnixStream::connect(daemon_socket()).is_ok()
}

fn daemon_ping() -> i32 {
    if daemon_alive() {
        println!("wfm: daemon alive");
        0
    } else {
        eprintln!("wfm: no daemon");
        1
    }
}

fn daemon_quit() -> i32 {
    if daemon_alive() {
        if let Ok(mut s) = std::os::unix::net::UnixStream::connect(daemon_socket()) {
            let _ = s.write_all(b"quit\n");
        }
        return 0;
    }
    eprintln!("wfm: no daemon");
    1
}

use std::path::Path;

fn shell_quote(s: &Path) -> String {
    s.display().to_string().replace('\'', "'\\''")
}

use std::io::Write;

/// `$XDG_RUNTIME_DIR/wfm/<pid>.pid` — one file per running window, so
/// `wfm --reload` can find and signal every instance.
fn run_dir() -> PathBuf {
    let rt = std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/tmp"));
    rt.join("wfm")
}

fn write_pidfile() {
    let Ok(dir) = std::fs::create_dir_all(run_dir()) else {
        return;
    };
    let _ = dir;
    let path = run_dir().join(format!("{}.pid", std::process::id()));
    let _ = std::fs::File::create(path).and_then(|mut f| write!(f, "{}", std::process::id()));
}

fn remove_pidfile() {
    let path = run_dir().join(format!("{}.pid", std::process::id()));
    let _ = std::fs::remove_file(path);
}

fn is_wfm_pid(pid: i32) -> bool {
    if let Ok(bytes) = std::fs::read(format!("/proc/{pid}/cmdline")) {
        if let Some(first) = bytes.split(|&b| b == 0).next() {
            if let Ok(s) = std::str::from_utf8(first) {
                let base = std::path::Path::new(s)
                    .file_name()
                    .map(|b| b.to_string_lossy().to_string())
                    .unwrap_or_default();
                if base == "wfm" {
                    return true;
                }
            }
        }
    }
    false
}

/// Send SIGUSR1 (hot reload) to every running wfm instance via its pidfile.
fn reload_instances() -> i32 {
    let rd = match std::fs::read_dir(run_dir()) {
        Ok(r) => r,
        Err(_) => {
            eprintln!("wfm: no running instances");
            return 1;
        }
    };
    let me = std::process::id();
    let mut sent = 0u32;
    for ent in rd.flatten() {
        let Some(name) = ent.file_name().to_str().map(|s| s.to_string()) else {
            continue;
        };
        let Some(pid_s) = name.strip_suffix(".pid") else {
            continue;
        };
        let Ok(pid) = pid_s.parse::<i32>() else {
            continue;
        };
        if pid as u32 == me {
            continue;
        }
        if is_wfm_pid(pid) {
            unsafe {
                libc::kill(pid, libc::SIGUSR1);
            }
            sent += 1;
        } else {
            let _ = std::fs::remove_file(ent.path()); // stale pidfile
        }
    }
    if sent > 0 {
        println!("wfm: sent reload to {sent} instance(s)");
        0
    } else {
        eprintln!("wfm: no running instances");
        1
    }
}
