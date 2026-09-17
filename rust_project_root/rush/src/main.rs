//! rush entry point.

use std::io::{self, Read, Write};
use rush::{Shell, exec};

const VERSION: &str = "0.1.0";

fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).collect();

    let mut interactive = false;
    let mut script: Option<String> = None;

    // parse flags
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-c" => {
                // read command from next arg
                if i + 1 < args.len() {
                    let code = args[i + 1].clone();
                    let mut sh = Shell::new(false);
                    sh.positional = args[i + 2..].to_vec();
                    let status = exec::run(&mut sh, &code);
                    std::process::exit(status);
                } else {
                    eprintln!("rush: -c requires an argument");
                    std::process::exit(2);
                }
            }
            "-i" | "--interactive" => interactive = true,
            "-v" | "--version" => {
                println!("rush {VERSION}");
                std::process::exit(0);
            }
            "--" => { args.drain(..=i); break; }
            s if s.starts_with('-') && s.len() > 1 => {
                // unknown flag
                eprintln!("rush: unknown option: {s}");
                std::process::exit(2);
            }
            _ => break, // first non-flag = script path
        }
        i += 1;
    }

    if let Some(first) = args.first() {
        script = Some(first.clone());
    }

    if let Some(path) = script {
        // run a script file
        match std::fs::read_to_string(&path) {
            Ok(content) => {
                let mut sh = Shell::new(false);
                sh.positional = args[1..].to_vec();
                let lines: Vec<String> = content.lines().map(String::from).collect();
                let status = exec::run_script_lines(&mut sh, &lines);
                std::process::exit(status);
            }
            Err(e) => {
                eprintln!("rush: {path}: {e}");
                std::process::exit(127);
            }
        }
    }

    // interactive or read-from-stdin
    if !interactive {
        // check if stdin is a TTY; if piped, read lines
        use std::io::IsTerminal;
        if std::io::stdin().is_terminal() {
            interactive = true;
        } else {
            let mut sh = Shell::new(false);
            let mut line = String::new();
            loop {
                line.clear();
                match std::io::stdin().read_line(&mut line) {
                    Ok(0) => break,
                    Ok(_) => {
                        let t = line.trim_end().to_string();
                        if !t.is_empty() { exec::run(&mut sh, &t); }
                    }
                    Err(_) => break,
                }
            }
            return;
        }
    }

    let mut sh = Shell::new(interactive);
    println!("rush {VERSION} — fast bash-like shell. Type 'exit' to leave.");
    let _ = io::stdout().flush();
    rush::main_loop::interactive(&mut sh);
}
