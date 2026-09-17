//! REPL: read a command line, execute it, print the prompt.

use std::io::{self, BufRead, Write};
use crate::shell::Shell;
use crate::exec;

pub fn interactive(sh: &mut Shell) {
    let stdin = io::stdin();
    loop {
        print_prompt(sh);
        let _ = io::stdout().flush();
        let mut line = String::new();
        match stdin.lock().read_line(&mut line) {
            Ok(0) => break, // EOF
            Ok(_) => {
                let line = line.trim_end();
                if line.is_empty() { continue; }
                // exit / single builtins
                exec::run(sh, line);
            }
            Err(_) => break,
        }
    }
}

fn print_prompt(sh: &Shell) {
    if sh.interactive {
        let user = sh.get("USER").map(String::as_str).unwrap_or("user");
        let host = sh.get("HOSTNAME").map(String::as_str).unwrap_or("rush");
        let dir = sh.cwd.to_string_lossy();
        let prompt = format!("{user}@{host}:{dir}$ ");
        print!("{prompt}");
    }
}
