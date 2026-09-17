//! Executor: interpret token streams, run builtins, spawn external binaries,
//! handle redirects, functions, conditionals and loops.

use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use crate::builtins;
use crate::expand;
use crate::parser::{lex, Tok, Token};
use crate::shell::Shell;

/// Execute a source string (one logical statement or a whole script block).
/// Returns nothing; shell.last_status is updated by commands.
pub fn run(sh: &mut Shell, src: &str) -> i32 {
    let toks = match lex(src) {
        Ok(t) => t,
        Err(e) => { eprintln!("rush: parse error: {e}"); sh.last_status = 2; return 2; }
    };
    let status = run_tokens(sh, &toks, 0).0;
    sh.last_status = status;
    status
}

/// Run a token stream from index `i`. Returns (status, next index).
/// Handles simple commands, functions, if/for/while/case, pipelines, and/or.
fn run_tokens(sh: &mut Shell, toks: &[Token], mut i: usize) -> (i32, usize) {
    let mut last_status = 0;
    while i < toks.len() {
        // skip separators
        if matches!(toks[i].tok, Tok::Semi | Tok::Newline) { i += 1; continue; }

        // grouping: `( ... )` subshell — we run inline
        if matches!(&toks[i].tok, Tok::LParen) {
            let (status, ni) = run_group(sh, toks, i);
            last_status = status;
            sh.last_status = status;
            i = ni;
            continue;
        }

        // keyword dispatch
        if let Tok::Word(w) = &toks[i].tok {
            let lower = w.to_ascii_lowercase();
            match lower.as_str() {
                "if" => { let (s, ni) = run_if(sh, toks, i); last_status = s; sh.last_status = s; i = ni; continue; }
                "for" => { let (s, ni) = run_for(sh, toks, i); last_status = s; sh.last_status = s; i = ni; continue; }
                "while" => { let (s, ni) = run_while(sh, toks, i); last_status = s; sh.last_status = s; i = ni; continue; }
                "case" => { let (s, ni) = run_case(sh, toks, i); last_status = s; sh.last_status = s; i = ni; continue; }
                "function" => { let (s, ni) = run_funcdef(sh, toks, i, true); last_status = s; sh.last_status = s; i = ni; continue; }
                "[" => { /* handled as simple command */ }
                _ => {}
            }

            // function definition: name ( ) { ... }
            if i + 1 < toks.len() && is_valid_var_name(w) {
                if let Tok::LParen = toks[i + 1].tok {
                    let (s, ni) = run_funcdef(sh, toks, i, false);
                    last_status = s; sh.last_status = s; i = ni; continue;
                }
            }
        }

        // Otherwise: a command list item (pipeline + and/or/background)
        let (status, ni) = run_command_list(sh, toks, i);
        last_status = status;
        sh.last_status = status;
        i = ni;
    }
    (last_status, i)
}

/// Run `( ... )` group. Returns (status, index after matching ')').
fn run_group(sh: &mut Shell, toks: &[Token], start: usize) -> (i32, usize) {
    // start points at '('; find matching ')'
    let mut depth = 0;
    let mut j = start;
    while j < toks.len() {
        match toks[j].tok {
            Tok::LParen => depth += 1,
            Tok::RParen => { depth -= 1; if depth == 0 { break; } }
            _ => {}
        }
        j += 1;
    }
    if j >= toks.len() {
        eprintln!("rush: syntax error: missing )");
        return (2, start + 1);
    }
    // run the inner tokens (does NOT create a subshell for vars for simplicity)
    let (status, _) = run_tokens(sh, &toks[start + 1..j], 0);
    (status, j + 1)
}

/// Run a command list item: parse a pipeline followed by `&&`, `||`, `;`, `&`,
/// `|`. Returns (status, next index).
fn run_command_list(sh: &mut Shell, toks: &[Token], start: usize) -> (i32, usize) {
    // Execute a list `cmd1 && cmd2 || cmd3 ; cmd4`. Iteratively process
    // segments left-to-right, honoring `&&`/`||` short-circuits.
    let mut i = start;
    let mut status = 0;
    loop {
        // find end of this segment (until |, ||, &&, ;, &, newline, EOF, ')')
        let mut j = i;
        let mut op_tok: Option<&Token> = None;
        while j < toks.len() {
            match toks[j].tok {
                Tok::Pipe | Tok::OrIf | Tok::AndIf | Tok::Semi
                | Tok::Amp | Tok::Newline | Tok::RParen => { op_tok = Some(&toks[j]); break; }
                _ => j += 1,
            }
        }
        let seg_end = j;
        if seg_end == i && matches!(toks.get(i).map(|t| &t.tok), Some(Tok::Semi | Tok::Newline)) {
            // nothing before a separator → just advance past it
            status = 0;
            if let Some(t) = op_tok {
                return (status, j + 1);
            }
            return (status, j);
        }

        if seg_end > i {
            status = run_simple_or_pipe(sh, &toks[i..seg_end]);
        }

        let Some(t) = op_tok else {
            return (status, toks.len());
        };
        match t.tok {
            Tok::Pipe => {
                // pipeline: the stages were already split/run by run_simple_or_pipe.
                i = j + 1;
            }
            Tok::OrIf => {
                if status != 0 {
                    i = j + 1; // run the right-hand side
                } else {
                    // skip the right-hand side up to the next &&/||/separator
                    i = skip_to_sep(toks, j + 1);
                    // If we stopped at another operator, keep processing it.
                    if i >= toks.len() { return (status, i); }
                }
            }
            Tok::AndIf => {
                if status == 0 {
                    i = j + 1;
                } else {
                    i = skip_to_sep(toks, j + 1);
                    if i >= toks.len() { return (status, i); }
                }
            }
            Tok::Amp => {
                return (status, j + 1);
            }
            Tok::Semi | Tok::Newline | Tok::RParen => {
                return (status, j + 1);
            }
            _ => {
                return (status, j + 1);
            }
        }
    }
}

fn skip_to_sep(toks: &[Token], mut i: usize) -> usize {
    while i < toks.len() {
        match toks[i].tok {
            Tok::Semi | Tok::Newline | Tok::RParen | Tok::OrIf | Tok::AndIf => break,
            Tok::Amp => { i += 1; break; }
            _ => i += 1,
        }
    }
    i
}

/// Run one simple command (possibly a pipeline via `|`). `seg` contains words
/// and redirects, plus `|` tokens.
fn run_simple_or_pipe(sh: &mut Shell, seg: &[Token]) -> i32 {
    // Split seg on Pipe into stages; each stage is argv+redirects.
    let mut stages: Vec<&[Token]> = Vec::new();
    let mut cur_start = 0;
    for (idx, t) in seg.iter().enumerate() {
        if matches!(t.tok, Tok::Pipe) {
            stages.push(&seg[cur_start..idx]);
            cur_start = idx + 1;
        }
    }
    stages.push(&seg[cur_start..]);

    let mut last_status = 0;
    for (i, stage) in stages.iter().enumerate() {
        // for real pipelines we'd connect stdout→stdin; v1 runs stages
        // independently, final status is last. Good enough for most scripts.
        if i == 0 && stages.len() == 1 {
            return run_stage(sh, stage);
        }
        let s = run_stage(sh, stage);
        last_status = s;
        let _ = i;
    }
    last_status
}

/// Run one stage's argv + redirects as a builtin, external command, or script.
/// If `stage` is an array assignment `name=(elem elem ...)`, store it and
/// return Some(status). Otherwise return None.
fn do_array_assignment(sh: &mut Shell, stage: &[Token]) -> Option<i32> {
    // first token must be Word ending in '=' with no other content
    let first = stage.first()?;
    let Tok::Word(w0) = &first.tok else { return None };
    let Some(eq) = w0.find('=') else { return None };
    if eq + 1 != w0.len() { return None; } // must be exactly `name=`
    let name = &w0[..eq];
    if !is_valid_var_name(name) { return None; }

    // next token must be LParen
    let mut i = 1;
    while i < stage.len() && matches!(stage[i].tok, Tok::Semi | Tok::Newline) { i += 1; }
    if i >= stage.len() || !matches!(stage[i].tok, Tok::LParen) { return None; }
    i += 1;

    let mut elems: Vec<String> = Vec::new();
    let mut ok = true;
    while i < stage.len() {
        match &stage[i].tok {
            Tok::RParen => { i += 1; break; }
            Tok::Word(w) => {
                let expanded = expand::expand_word(w, sh, &|c| run_subshell(c));
                let words = expand::split_expanded(&expanded, sh, &|c| run_subshell(c));
                for w2 in words { elems.push(w2); }
                i += 1;
            }
            _ => { i += 1; }
        }
    }
    let _ = ok;
    sh.set_array(name, elems);
    sh.last_status = 0;
    Some(0)
}

fn run_stage(sh: &mut Shell, stage: &[Token]) -> i32 {
    // Array assignment: `name=( ... )`. The lexer yields Word("name=") LParen words RParen.
    if let Some(rc) = do_array_assignment(sh, stage) {
        return rc;
    }
    let mut argv: Vec<String> = Vec::new();
    let mut redirects: Vec<(String, String)> = Vec::new(); // (op, target)
    let mut i = 0;
    while i < stage.len() {
        match &stage[i].tok {
            Tok::Word(w) => {
                let expanded = expand::expand_word(w, sh, &|c| run_subshell(c));
                let words = expand::split_expanded(&expanded, sh, &|c| run_subshell(c));
                for w2 in words { argv.push(w2); }
                i += 1;
            }
            Tok::Gt => { if let Ok(t) = next_word(stage, &mut i) { redirects.push((">".into(), t)); } }
            Tok::GtGt => { if let Ok(t) = next_word(stage, &mut i) { redirects.push((">>".into(), t)); } }
            Tok::Lt => { if let Ok(t) = next_word(stage, &mut i) { redirects.push(("<".into(), t)); } }
            Tok::HereStr => { if let Ok(t) = next_word(stage, &mut i) { redirects.push(("<<<".into(), t)); } }
            Tok::ErrGt => { if let Ok(t) = next_word(stage, &mut i) { redirects.push(("2>".into(), t)); } }
            Tok::Amp => {
                // trailing `&` → background. We run current argv in a thread.
                if argv.is_empty() { i += 1; continue; }
                let bg_argv = argv.clone();
                let bg_redirects = redirects.clone();
                let env = make_env(sh);
                let cwd = sh.cwd.clone();
                let _ = std::thread::spawn(move || run_external(&bg_argv, &bg_redirects, &env, &cwd));
                argv.clear();
                i += 1;
                continue;
            }
            other => {
                eprintln!("rush: unexpected token in command: {other:?}");
                i += 1;
            }
        }
    }

    if argv.is_empty() {
        // Bare redirect with no command: create/truncate the target file(s).
        let mut ok = true;
        for (op, target) in &redirects {
            match op.as_str() {
                ">" => {
                    if std::fs::File::create(target).is_err() { ok = false; }
                }
                _ => {}
            }
        }
        sh.last_status = if ok { 0 } else { 1 };
        return sh.last_status;
    }
    let _ = apply_redirects(&redirects);

    // variable assignments: `NAME=value` prefix (and/or bare assignments)
    let mut prefix_env: Vec<(String, String)> = Vec::new();
    let mut ci = 0;
    while ci < argv.len() {
        let a = &argv[ci];
        if let Some(eq) = a.find('=') {
            let name = &a[..eq];
            if is_valid_var_name(name) {
                let val = expand::expand_word(&a[eq + 1..], sh, &|cmd| {
                    run_subshell(cmd)
                });
                prefix_env.push((name.to_string(), val));
                ci += 1;
                continue;
            }
        }
        break;
    }

    // if everything was assignments (no command), persist in shell
    if ci == argv.len() {
        for (k, v) in prefix_env {
            sh.vars.insert(k, v);
        }
        sh.last_status = 0;
        return 0;
    }

    // fold assignments into the command argv tail
    argv.drain(..ci);
    let prefix_vars = prefix_env;

    let name = argv[0].clone();

    // function?
    if sh.functions.contains_key(&name) {
        let saved_pos = sh.positional.clone();
        sh.positional = argv[1..].to_vec();
        let body = sh.functions.get(&name).cloned().unwrap_or_default();
        let status = run_script_lines(sh, &body);
        sh.positional = saved_pos;
        sh.last_status = status;
        return status;
    }

    // builtin?
    if builtins::is_builtin(&name) {
        if name == "." || name == "source" {
            if argv.len() < 2 { eprintln!("{name}: filename argument required"); return 2; }
            let file = &argv[1];
            let path = resolve(sh, file);
            match std::fs::read_to_string(&path) {
                Ok(content) => {
                    let status = run_script_lines(sh, &content.lines().map(String::from).collect::<Vec<_>>());
                    sh.last_status = status;
                    return status;
                }
                Err(e) => { eprintln!("{name}: {}: {e}", path.display()); return 1; }
            }
        }
        if name == "cd" || name == "export" || name == "unset" || name == "set"
            || name == "wait" || name == "jobs" || name == "kill" {
            return builtins::run_builtin(sh, &argv);
        }
        // fs + lightweight builtins can run without full state mutation; still
        // pass sh for cd-like; run_builtin handles mutation itself.
        return builtins::run_builtin(sh, &argv);
    }

    // a script/binary: fork+exec
    let mut env = make_env(sh);
    for (k, v) in &prefix_vars {
        env.insert(k.clone(), v.clone());
    }
    let cwd = sh.cwd.clone();
    match run_external(&argv, &redirects, &env, &cwd) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("rush: {name}: {e}");
            127
        }
    }
}

fn is_valid_var_name(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn make_env(sh: &Shell) -> std::collections::HashMap<String, String> {
    sh.env()
}

fn next_word(stage: &[Token], i: &mut usize) -> Result<String, ()> {
    *i += 1;
    if *i < stage.len() {
        if let Tok::Word(w) = &stage[*i].tok {
            *i += 1;
            return Ok(w.clone());
        }
    }
    Err(())
}

fn apply_redirects(redirects: &[(String, String)]) -> Result<(), i32> {
    // v1: for builtins we mostly ignore file redirects to keep things simple,
    // but `>` needs to work for `echo x > file`. We implement `>` by appending.
    for (op, target) in redirects {
        match op.as_str() {
            ">" => {
                use std::io::Write;
                // this builtin simply errors out because we can't easily
                // redirect Rust stdout per-command. Return error to signal.
                let _ = target;
                return Err(2);
            }
            _ => {}
        }
    }
    Ok(())
}

/// Run external binary. `redirects` applied via std::process::Command Stdio.
fn run_external(
    argv: &[String],
    redirects: &[(String, String)],
    env: &std::collections::HashMap<String, String>,
    cwd: &std::path::Path,
) -> Result<i32, String> {
    let mut iter = argv.iter();
    let prog = iter.next().map(String::as_str).unwrap_or("").to_string();
    let args: Vec<&String> = iter.collect();

    let mut cmd = std::process::Command::new(&prog);
    cmd.args(&args);
    cmd.current_dir(cwd);
    for (k, v) in env {
        cmd.env(k, v);
    }
    // redirects
    use std::process::Stdio;
    for (op, target) in redirects {
        match op.as_str() {
            ">" => {
                let f = std::fs::File::create(target)
                    .map_err(|e| format!("{target}: {e}"))?;
                cmd.stdout(Stdio::from(f));
            }
            ">>" => {
                let f = std::fs::OpenOptions::new().create(true).append(true).open(target)
                    .map_err(|e| format!("{target}: {e}"))?;
                cmd.stdout(Stdio::from(f));
            }
            "<" => {
                let f = std::fs::File::open(target).map_err(|e| format!("{target}: {e}"))?;
                cmd.stdin(Stdio::from(f));
            }
            "2>" => {
                let f = std::fs::File::create(target).map_err(|e| format!("{target}: {e}"))?;
                cmd.stderr(Stdio::from(f));
            }
            "<<" => {}
            _ => {}
        }
    }
    match cmd.status() {
        Ok(s) => Ok(s.code().unwrap_or(1)),
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                Err(format!("command not found"))
            } else {
                Err(e.to_string())
            }
        }
    }
}

fn run_subshell(cmd: &str) -> String {
    // Best-effort `$(cmd)` for the common external case: capture stdout.
    if let Ok(output) = std::process::Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .output()
    {
        return String::from_utf8_lossy(&output.stdout).trim().to_string();
    }
    String::new()
}

fn resolve(sh: &Shell, file: &str) -> std::path::PathBuf {
    let p = std::path::Path::new(file);
    if p.is_absolute() { p.to_path_buf() } else { sh.cwd.join(p) }
}

/// Run a block of source lines (from a function body / then-branch / loop body).
pub fn run_script_lines(sh: &mut Shell, lines: &[String]) -> i32 {
    let joined = lines.join("\n");
    run(sh, &joined)
}

// ─────────── compound constructs ───────────────────────────────────────────

/// Find the next Word whose text matches `word`. Returns its index, or toks.len().
fn find_word(toks: &[Token], start: usize, word: &str) -> usize {
    let mut i = start;
    while i < toks.len() {
        if let Tok::Word(w) = &toks[i].tok {
            if w == word { return i; }
        }
        i += 1;
    }
    toks.len()
}

/// Find the next Word whose text is any of `words`. Returns (index, text), or toks.len().
fn find_any_word<'a>(toks: &[Token], start: usize, words: &[&'a str]) -> (usize, &'a str) {
    let mut i = start;
    while i < toks.len() {
        if let Tok::Word(w) = &toks[i].tok {
            for kw in words {
                if w == *kw { return (i, kw); }
            }
        }
        i += 1;
    }
    (toks.len(), "")
}

/// Skip past any Semi tokens starting at `i`. Returns new index.
fn skip_semi(toks: &[Token], mut i: usize) -> usize {
    while i < toks.len() && matches!(toks[i].tok, Tok::Semi | Tok::Newline) {
        i += 1;
    }
    i
}

/// Run a token sub-slice as its own statement list.
fn run_subtokens(sh: &mut Shell, toks: &[Token]) -> i32 {
    let (status, _) = run_tokens(sh, toks, 0);
    status
}

/// `if COND; then A; elif B; then C; else D; fi`
fn run_if(sh: &mut Shell, toks: &[Token], start: usize) -> (i32, usize) {
    let then_idx = find_word(toks, start + 1, "then");
    if then_idx >= toks.len() {
        eprintln!("rush: if: missing 'then'");
        return (2, toks.len());
    }
    // condition = tokens between "if" and "then" (skip any trailing Semi/AndIf/OrIf)
    let mut cond_end = then_idx;
    while cond_end > start + 1 && matches!(toks[cond_end - 1].tok, Tok::Semi | Tok::Newline | Tok::AndIf | Tok::OrIf) {
        cond_end -= 1;
    }
    let cond_true = eval_condition(sh, &toks[start + 1..cond_end]);

    // find matching elif/else/fi for the body
    let body_start = then_idx + 1;
    let (elif_idx, _kw) = find_any_word(toks, body_start, &["elif", "else", "fi"]);

    if cond_true {
        // run body: tokens[body_start..elif_idx]
        let body_end = elif_idx;
        let body = &toks[body_start..body_end];
        let status = run_subtokens(sh, body);
        // skip past "fi"
        let fi_idx = find_word(toks, elif_idx + 1, "fi");
        return (status, fi_idx + 1);
    }

    // condition false: skip branches
    let mut i = elif_idx;
    while i < toks.len() {
        let (next, kw) = find_any_word(toks, i, &["elif", "else", "fi"]);
        match kw {
            "elif" => {
                // find then after elif condition
                let then2 = find_word(toks, next + 1, "then");
                let mut ce2 = then2;
                while ce2 > next + 1 && matches!(toks[ce2 - 1].tok, Tok::Semi | Tok::Newline) { ce2 -= 1; }
                let c2 = eval_condition(sh, &toks[next + 1..ce2]);
                if c2 {
                    let bs = then2 + 1;
                    let (be, _) = find_any_word(toks, bs, &["elif", "else", "fi"]);
                    let status = run_subtokens(sh, &toks[bs..be]);
                    let fi_idx = find_word(toks, be + 1, "fi");
                    return (status, fi_idx + 1);
                }
                // skip this elif block
                let (next2, _) = find_any_word(toks, then2 + 1, &["elif", "else", "fi"]);
                i = next2;
            }
            "else" => {
                let fi_idx = find_word(toks, next + 1, "fi");
                let status = run_subtokens(sh, &toks[next + 1..fi_idx]);
                return (status, fi_idx + 1);
            }
            "fi" => return (0, next + 1),
            _ => i = next + 1,
        }
    }
    (0, toks.len())
}

/// `for var in words; do ...; done`
fn run_for(sh: &mut Shell, toks: &[Token], start: usize) -> (i32, usize) {
    let var = match toks.get(start + 1) {
        Some(Token { tok: Tok::Word(w), .. }) => w.clone(),
        _ => { eprintln!("rush: for: missing variable name"); return (2, start + 1); }
    };
    let mut words: Vec<String> = Vec::new();
    let mut body_start;

    let (in_idx, _) = find_any_word(toks, start + 2, &["in", "do"]);
    if in_idx < toks.len() && matches!(&toks[in_idx].tok, Tok::Word(w) if w == "in") {
        let (do_idx, _) = find_any_word(toks, in_idx + 1, &["do"]);
        for t in &toks[in_idx + 1..do_idx] {
            if let Tok::Word(w) = &t.tok {
                let cmd_run = |c: &str| run_subshell(c);
                let exp = expand::expand_word(w, sh, &cmd_run);
                words.extend(expand::split_expanded(&exp, sh, &cmd_run));
            }
        }
        body_start = do_idx + 1;
    } else {
        // no `in` → use positional params
        words = sh.positional.clone();
        body_start = in_idx + 1; // skip "do"
    }

    let (done_idx, _) = find_any_word(toks, body_start, &["done"]);
    let mut status = 0;
    for w in words {
        sh.vars.insert(var.clone(), w);
        status = run_subtokens(sh, &toks[body_start..done_idx]);
    }
    (status, done_idx + 1)
}

/// `while cond; do body; done`
fn run_while(sh: &mut Shell, toks: &[Token], start: usize) -> (i32, usize) {
    let (do_idx, _) = find_any_word(toks, start + 1, &["do"]);
    let mut cond_end = do_idx;
    while cond_end > start + 1 && matches!(toks[cond_end - 1].tok, Tok::Semi | Tok::Newline) { cond_end -= 1; }
    let body_start = do_idx + 1;
    let (done_idx, _) = find_any_word(toks, body_start, &["done"]);

    let mut status = 0;
    loop {
        if !eval_condition(sh, &toks[start + 1..cond_end]) { break; }
        status = run_subtokens(sh, &toks[body_start..done_idx]);
    }
    (status, done_idx + 1)
}

/// `case word in pat) body;; pat) body;; esac`
fn run_case(sh: &mut Shell, toks: &[Token], start: usize) -> (i32, usize) {
    let word = match toks.get(start + 1) {
        Some(Token { tok: Tok::Word(w), .. }) => {
            let cmd_run = |c: &str| run_subshell(c);
            expand::expand_word(w, sh, &cmd_run)
        }
        _ => String::new(),
    };
    let (in_idx, _) = find_any_word(toks, start + 2, &["in"]);
    let arm_start = in_idx + 1;
    let (esac_idx, _) = find_any_word(toks, arm_start, &["esac"]);

    let mut pos = arm_start;
    let mut matched = false;
    let mut status = 0;
    while pos < esac_idx {
        // collect pattern until RParen
        let mut pattern = String::new();
        while pos < esac_idx && !matches!(toks[pos].tok, Tok::RParen) {
            if let Tok::Word(w) = &toks[pos].tok {
                if !pattern.is_empty() { pattern.push(' '); }
                pattern.push_str(w);
            }
            pos += 1;
        }
        pos += 1; // skip ')'
        // collect body until ;;
        let body_start = pos;
        while pos < esac_idx {
            if let Tok::Word(w) = &toks[pos].tok { if w == ";;" { break; } }
            pos += 1;
        }
        let body = &toks[body_start..pos];
        if !matched && pattern_matches(&pattern, &word) {
            matched = true;
            status = run_subtokens(sh, body);
        }
        pos += 1; // skip ;;
        if pos < esac_idx && matches!(toks[pos].tok, Tok::Semi) { pos += 1; }
    }
    (status, esac_idx + 1)
}

fn pattern_matches(pat: &str, word: &str) -> bool {
    for p in pat.split('|') {
        if expand::wild_match(word, p.trim()) { return true; }
    }
    false
}

/// `function name { body }` or `name() { body }`
fn run_funcdef(sh: &mut Shell, toks: &[Token], start: usize, has_function_kw: bool) -> (i32, usize) {
    let name_idx = if has_function_kw { start + 1 } else { start };
    let name = match toks.get(name_idx) {
        Some(Token { tok: Tok::Word(w), .. }) => w.clone(),
        _ => { eprintln!("rush: function: bad name"); return (2, name_idx); }
    };
    // find '{'
    let mut brace_idx = name_idx + 1;
    while brace_idx < toks.len() {
        if matches!(&toks[brace_idx].tok, Tok::Word(w) if w == "{") { break; }
        if matches!(&toks[brace_idx].tok, Tok::LParen) { brace_idx += 1; continue; }
        brace_idx += 1;
    }
    let body_start = brace_idx + 1;
    // find matching '}' using simple depth
    let mut depth = 0;
    let mut j = body_start;
    let mut body_end = toks.len();
    while j < toks.len() {
        match toks[j].tok {
            Tok::LParen => depth += 1,
            Tok::RParen => { if depth > 0 { depth -= 1; } }
            _ => {}
        }
        if matches!(&toks[j].tok, Tok::Word(w) if w == "}") && depth == 0 {
            body_end = j;
            break;
        }
        j += 1;
    }
    // store body tokens as source strings for later re-execution,
    // reconstructing the source: join consecutive words with a space,
    // preserve statement separators so multi-line bodies survive re-lexing.
    let mut body_src: Vec<String> = Vec::new();
    let mut cur_statement = String::new();
    let mut first = true;
    for t in &toks[body_start..body_end] {
        match &t.tok {
            Tok::Word(_) => {
                if !cur_statement.is_empty() { cur_statement.push(' '); }
                let _ = first;
                cur_statement.push_str(&t.src);
            }
            Tok::Semi | Tok::Newline => {
                if !cur_statement.is_empty() {
                    body_src.push(std::mem::take(&mut cur_statement));
                }
            }
            _ => {
                if !cur_statement.is_empty() { cur_statement.push(' '); }
                cur_statement.push_str(&t.src);
            }
        }
    }
    if !cur_statement.is_empty() { body_src.push(cur_statement); }
    sh.functions.insert(name, body_src);
    (0, body_end + 1)
}

// ─────────── condition evaluation ──────────────────────────────────────────

/// Evaluate a `[[ ... ]]` condition or simple test. Returns bool.
fn eval_condition(sh: &Shell, toks: &[Token]) -> bool {
    if toks.is_empty() { return true; }
    // strip leading `[[` and trailing `]]`
    let mut cond: Vec<&Token> = toks.iter().collect();
    while let Some(first) = cond.first() {
        if let Tok::Word(w) = &first.tok { if w == "[[" { cond.remove(0); continue; } }
        break;
    }
    while let Some(last) = cond.last() {
        if let Tok::Word(w) = &last.tok { if w == "]]" { cond.pop(); continue; } }
        break;
    }
    // also handle `[ ... ]` (single bracket)
    while let Some(first) = cond.first() {
        if let Tok::Word(w) = &first.tok { if w == "[" { cond.remove(0); continue; } }
        break;
    }
    while let Some(last) = cond.last() {
        if let Tok::Word(w) = &last.tok { if w == "]" { cond.pop(); continue; } }
        break;
    }
    // build a condition string parts
    let mut parts: Vec<String> = Vec::new();
    for t in &cond {
        if let Tok::Word(w) = &t.tok {
            parts.push(expand::expand_word(w, sh, &|_| String::new()));
        } else {
            // operators like && || become parts too
            parts.push(t.src.clone());
        }
    }
    eval_cond_parts(&parts)
}

fn eval_cond_parts(parts: &[String]) -> bool {
    if parts.is_empty() { return false; }
    // Simple two-operand or three-operand test
    if parts.len() == 1 {
        return !parts[0].is_empty() && parts[0] != "0";
    }
    if parts.len() == 2 {
        let (op, a) = (&parts[0], &parts[1]);
        return match op.as_str() {
            "-e" | "-f" | "-d" | "-x" | "-L" => {
                let p = std::path::Path::new(a);
                match op.as_str() {
                    "-e" => p.exists(),
                    "-f" => p.is_file(),
                    "-d" => p.is_dir(),
                    "-x" => std::fs::metadata(a).map(|m| m.permissions().mode() & 0o111 != 0).unwrap_or(false),
                    "-L" => std::fs::symlink_metadata(a).map(|m| m.file_type().is_symlink()).unwrap_or(false),
                    _ => false,
                }
            }
            "-z" => a.is_empty(),
            "-n" => !a.is_empty(),
            "!" => a.is_empty() || a == "0",
            _ => !a.is_empty(),
        };
    }
    if parts.len() == 3 {
        let (a, op, b) = (&parts[0], &parts[1], &parts[2]);
        return match op.as_str() {
            "=" | "==" => a == b,
            "!=" => a != b,
            "-eq" => a.parse::<i64>().ok() == b.parse::<i64>().ok(),
            "-ne" => a.parse::<i64>().ok() != b.parse::<i64>().ok(),
            "-gt" => a.parse::<i64>().unwrap_or(0) > b.parse::<i64>().unwrap_or(0),
            "-lt" => a.parse::<i64>().unwrap_or(0) < b.parse::<i64>().unwrap_or(0),
            "-ge" => a.parse::<i64>().unwrap_or(0) >= b.parse::<i64>().unwrap_or(0),
            "-le" => a.parse::<i64>().unwrap_or(0) <= b.parse::<i64>().unwrap_or(0),
            _ => !a.is_empty(),
        };
    }
    // longer expressions: try && / || parsing
    eval_complex_cond(parts)
}

fn eval_complex_cond(parts: &[String]) -> bool {
    // split on || and && (simplistic left-to-right)
    let mut result = true;
    let mut current_op = "&&"; // implicit AND between sub-expressions
    let mut i = 0;
    while i < parts.len() {
        match parts[i].as_str() {
            "&&" => { current_op = "&&"; i += 1; continue; }
            "||" => { current_op = "||"; i += 1; continue; }
            "!" => {
                i += 1;
                let sub = eval_cond_parts(&parts[i..i + 1.min(parts.len() - i)]);
                result = if current_op == "&&" { result && !sub } else { result || !sub };
                i += 1;
                continue;
            }
            _ => {}
        }
        // try to consume a 2 or 3 token sub-expression
        let remaining = &parts[i..];
        if remaining.len() >= 3 {
            let sub = eval_cond_parts(&remaining[..3]);
            result = if current_op == "&&" { result && sub } else { result || sub };
            i += 3;
        } else if remaining.len() >= 2 {
            let sub = eval_cond_parts(&remaining[..2]);
            result = if current_op == "&&" { result && sub } else { result || sub };
            i += 2;
        } else {
            let sub = eval_cond_parts(&remaining[..1]);
            result = if current_op == "&&" { result && sub } else { result || sub };
            i += 1;
        }
    }
    result
}
