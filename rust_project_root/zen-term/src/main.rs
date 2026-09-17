//! zen-term — pure Wayland GPU terminal.
//!
//!   - raw GL (EGL) renderer today, raw Vulkan (ash) behind the same
//!     `GpuBackend` interface next
//!   - signal-driven config hot reload (`zen-term --reload` → SIGUSR1): no
//!     file watching, no polling — idle cost is 0 CPU / 0 RAM growth

mod atlas;
mod config;
mod font;
mod input;
mod render;
mod render_gl;
mod term;
mod wl;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use calloop::channel::{channel, Sender};
use config::Config;
use term::UiMsg;

const VERSION: &str = "0.1.0";

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

fn run_dir() -> PathBuf {
    std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp"))
}

fn write_pidfile() {
    let _ = std::fs::create_dir_all(run_dir());
    let path = run_dir().join(format!("zen-term-{}.pid", std::process::id()));
    let _ = std::fs::File::create(path)
        .and_then(|mut f| std::io::Write::write_all(&mut f, std::process::id().to_string().as_bytes()));
}

fn remove_pidfile() {
    let path = run_dir().join(format!("zen-term-{}.pid", std::process::id()));
    let _ = std::fs::remove_file(path);
}

fn is_zen_term_pid(pid: i32) -> bool {
    if let Ok(bytes) = std::fs::read(format!("/proc/{pid}/cmdline")) {
        if let Some(first) = bytes.split(|&b| b == 0).next() {
            if let Ok(s) = std::str::from_utf8(first) {
                let base = std::path::Path::new(s)
                    .file_name()
                    .map(|b| b.to_string_lossy().to_string())
                    .unwrap_or_default();
                return base == "zen-term";
            }
        }
    }
    false
}

/// `zen-term --reload`: send SIGUSR1 to every running zen-term instance.
fn reload_instances() -> i32 {
    let rd = match std::fs::read_dir(run_dir()) {
        Ok(r) => r,
        Err(_) => {
            eprintln!("zen-term: no running instances");
            return 1;
        }
    };
    let me = std::process::id() as i32;
    let mut sent = 0u32;
    for ent in rd.flatten() {
        let Some(name) = ent.file_name().to_str().map(|s| s.to_string()) else {
            continue;
        };
        let Some(pid) = name.strip_prefix("zen-term-").and_then(|p| p.strip_suffix(".pid")).and_then(|p| p.parse::<i32>().ok()) else {
            continue;
        };
        if pid == me || !is_zen_term_pid(pid) {
            continue;
        }
        unsafe {
            libc::kill(pid, libc::SIGUSR1);
        }
        sent += 1;
    }
    if sent == 0 {
        eprintln!("zen-term: no running instances");
        return 1;
    }
    println!("zen-term: reload signal sent to {sent} instance(s)");
    0
}

fn usage() {
    eprintln!(
        "zen-term {} — pure Wayland GPU terminal (Rust)\n\
         usage: zen-term [options]\n\
         options:\n\
           --reload       hot-reload config in running zen-term instance(s)\n\
           --config PATH  use an explicit config file\n\
           --version      print version\n\
           -h, --help     this help",
        VERSION
    );
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut cfg_path = config::config_path_default();

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => {
                usage();
                return;
            }
            "--version" => {
                println!("zen-term {VERSION}");
                return;
            }
            "--reload" => {
                std::process::exit(reload_instances());
            }
            "--config" => {
                if let Some(c) = args.get(i + 1) {
                    cfg_path = PathBuf::from(c);
                    i += 1;
                }
            }
            _ if args[i].starts_with("--") => {
                eprintln!("zen-term: unknown option {}", args[i]);
                usage();
                std::process::exit(2);
            }
            _ => {
                eprintln!("zen-term: unexpected argument {}", args[i]);
                usage();
                std::process::exit(2);
            }
        }
        i += 1;
    }

    let mut cfg = Config::default();
    config::load(&mut cfg, &cfg_path);
    config::ensure_default_file(&cfg, &cfg_path);

    if cfg.gpu.as_str() != "gl" {
        eprintln!("zen-term: gpu backend \"{}\" not implemented yet; using GL", cfg.gpu);
    }

    // UI channel (pty thread → event loop).
    let (ui_tx, ui_rx) = channel::<UiMsg>();
    let _: Sender<UiMsg> = ui_tx;

    let terminal = match term::Terminal::spawn(&cfg, ui_tx, 80, 24, (8.0, 16.0)) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("zen-term: failed to start shell: {e}");
            std::process::exit(1);
        }
    };

    write_pidfile();
    let result = wl::run(cfg, cfg_path, terminal, ui_rx);
    remove_pidfile();
    if let Err(e) = result {
        eprintln!("zen-term: {e}");
        std::process::exit(1);
    }
}
