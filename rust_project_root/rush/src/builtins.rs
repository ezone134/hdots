//! Builtin commands. Includes forkless fs ops via `rsys` (no external binary).
//!
//! Each builtin returns exit status. `sh`-mutating builtins (cd, export,
//! unset) update shell state. Fs builtins call `rsys::execute` which performs
//! the syscall directly in this process.

use std::io::Write;
use crate::shell::Shell;

pub fn is_builtin(name: &str) -> bool {
    matches!(
        name,
        "cd" | "pwd" | "echo" | "printf" | "export" | "unset"
            | "exit" | "." | "source" | "set" | "type" | "true" | "false"
            | "mkdir" | "rm" | "rmdir" | "chmod" | "cp" | "mv"
            | "wait" | "jobs" | "kill" | "sleep-millis" | ":"
    )
}

/// Run a builtin. `argv[0]` is the name. Returns status and may print to stdout.
pub fn run_builtin(sh: &mut Shell, argv: &[String]) -> i32 {
    let name = &argv[0];
    match name.as_str() {
        "true" | ":" => 0,
        "false" => 1,
        "pwd" => {
            println!("{}", sh.cwd.display());
            0
        }
        "cd" => {
            let target = if argv.len() > 1 {
                let t = &argv[1];
                if t == "-" {
                    match sh.vars.get("OLDPWD") {
                        Some(o) => o.clone(),
                        None => { eprintln!("cd: no OLDPWD"); return 1; }
                    }
                } else if t.starts_with('~') {
                    let home = sh.get("HOME").cloned().unwrap_or_default();
                    format!("{home}{}", &t[1..])
                } else {
                    t.clone()
                }
            } else {
                sh.get("HOME").cloned().unwrap_or_default()
            };
            let path = std::path::Path::new(&target);
            let p = if path.is_absolute() {
                path.to_path_buf()
            } else {
                sh.cwd.join(path)
            };
            if p.is_dir() {
                sh.vars.insert("OLDPWD".into(), sh.cwd.to_string_lossy().into_owned());
                sh.cwd = p;
                if let Err(e) = std::env::set_current_dir(&sh.cwd) {
                    eprintln!("cd: {e}");
                    return 1;
                }
                0
            } else {
                eprintln!("cd: no such directory: {target}");
                1
            }
        }
        "echo" => {
            // flags -n and -e are partly honored
            let mut newline = true;
            let mut rest = false;
            let mut i = 1;
            while i < argv.len() {
                match argv[i].as_str() {
                    "-n" => { newline = false; i += 1; }
                    "-e" => { rest = true; i += 1; }
                    "-E" => { i += 1; }
                    "--" => { i += 1; break; }
                    _ => break,
                }
            }
            let body: Vec<&str> = argv[i..].iter().map(String::as_str).collect();
            let joined = match rest {
                true => expand_escapes(&body.join(" ")),
                false => body.join(" "),
            };
            if newline { println!("{joined}"); } else { print!("{joined}"); }
            0
        }
        "printf" => {
            let fmt = argv.get(1).map(String::as_str).unwrap_or("");
            let args = &argv[2..];
            let s = printf_fmt(fmt, args);
            print!("{s}");
            0
        }
        "export" => {
            if argv.len() == 1 {
                for k in &sh.exported {
                    if let Some(v) = sh.get(k) {
                        println!("export {k}={v}");
                    }
                }
                return 0;
            }
            for a in &argv[1..] {
                if let Some((k, v)) = a.split_once('=') {
                    sh.vars.insert(k.into(), v.into());
                    sh.export(k);
                } else {
                    sh.export(a);
                }
            }
            0
        }
        "unset" => {
            for a in &argv[1..] {
                sh.vars.remove(a);
                sh.exported.remove(a);
            }
            0
        }
        "exit" => {
            let code = argv.get(1).and_then(|s| s.parse().ok()).unwrap_or(sh.last_status);
            std::process::exit(code);
        }
        "set" => {
            // minimal: set -e / +e etc are no-ops; treat remaining as positional-ish
            0
        }
        "type" => {
            for a in &argv[1..] {
                if is_builtin(a) { println!("{a} is a shell builtin"); }
                else if sh.functions.contains_key(a) { println!("{a} is a shell function"); }
                else if let Ok(p) = which(a) { println!("{a} is {p}"); }
                else { println!("type: {a}: not found"); }
            }
            0
        }
        "wait" => {
            for mut c in sh.jobs.drain(..) {
                let _ = c.wait();
            }
            0
        }
        "jobs" => {
            for (i, _c) in sh.jobs.iter().enumerate() {
                println!("[{}] running", i + 1);
            }
            0
        }
        "kill" => {
            // kill PID(s)
            for a in &argv[1..] {
                if a.starts_with('-') {
                    continue;
                }
                if let Ok(pid) = a.parse::<i32>() {
                    let _ = std::process::Command::new("kill").arg("-s").arg("15").arg(pid.to_string()).status();
                }
            }
            0
        }
        "sleep-millis" => {
            if let Some(ms) = argv.get(1).and_then(|s| s.parse::<u64>().ok()) {
                std::thread::sleep(std::time::Duration::from_millis(ms));
            }
            0
        }
        "." | "source" => {
            // handled by executor (needs script parsing); return 1 if no file
            eprintln!("{name}: no file (handled by executor)");
            1
        }
        // ── forkless fs builtins ────────────────────────────────────────────
        "mkdir" | "rm" | "rmdir" | "chmod" | "cp" | "mv" => {
            let argv_owned: Vec<String> = argv.to_vec();
            let op = match rsys::argv_to_op(&argv_owned) {
                Ok(op) => op,
                Err(e) => { eprintln!("{name}: {e}"); return 1; }
            };
            let mut out = Vec::new();
            match rsys::execute(&op, &mut out) {
                Ok(()) => {
                    let _ = std::io::stdout().write_all(&out);
                    let _ = std::io::stdout().flush();
                    0
                }
                Err(e) => {
                    eprintln!("{name}: {e}");
                    1
                }
            }
        }
        _ => {
            eprintln!("rush: builtin not implemented: {name}");
            1
        }
    }
}

fn expand_escapes(s: &str) -> String {
    let mut out = String::new();
    let mut it = s.chars();
    while let Some(c) = it.next() {
        if c == '\\' {
            match it.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('\\') => out.push('\\'),
                Some(o) => { out.push('\\'); out.push(o); }
                None => {}
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn printf_fmt(fmt: &str, args: &[String]) -> String {
    // %s, %d, %\n etc.
    let mut out = String::new();
    let mut ai = 0;
    let mut it = fmt.chars();
    while let Some(c) = it.next() {
        if c == '%' {
            match it.next() {
                Some('%') => out.push('%'),
                Some('s') => { if let Some(a) = args.get(ai) { out.push_str(a); ai += 1; } }
                Some('d') => { if let Some(a) = args.get(ai) { out.push_str(a); ai += 1; } }
                Some('\n') => out.push('\n'),
                Some(c) => { out.push('%'); out.push(c); }
                None => out.push('%'),
            }
        } else if c == '\\' {
            match it.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some(o) => { out.push(o); }
                None => {}
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn which(cmd: &str) -> std::io::Result<String> {
    if cmd.contains('/') {
        let p = std::path::Path::new(cmd);
        if p.exists() { return Ok(cmd.into()); }
        return Err(std::io::Error::new(std::io::ErrorKind::NotFound, "no such file"));
    }
    let path = std::env::var("PATH").unwrap_or_default();
    for dir in path.split(':') {
        let p = std::path::Path::new(dir).join(cmd);
        if p.is_file() {
            return Ok(p.to_string_lossy().into_owned());
        }
    }
    Err(std::io::Error::new(std::io::ErrorKind::NotFound, "not found"))
}
