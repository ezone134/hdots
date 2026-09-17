//! Expansion: variable/parameter expansion, command substitution, word
//! splitting and globbing.

use crate::shell::Shell;

/// Expand a word. `cmd_sub` runs command substitution by capturing external
/// command stdout or shell-builtin stdout (for our purposes: run a subshell).
/// We support `$(cmd)` and `$(<file)`.
pub fn expand_word(word: &str, sh: &Shell, cmd_run: &impl Fn(&str) -> String) -> String {
    let mut out = String::new();
    let chars: Vec<char> = word.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '$' && i + 1 < chars.len() {
            // $((expr)) arithmetic expansion
            if chars[i + 1] == '(' && i + 2 < chars.len() && chars[i + 2] == '(' {
                let mut depth = 2u32;
                let mut j = i + 3;
                while j < chars.len() {
                    if chars[j] == '(' { depth += 1; }
                    else if chars[j] == ')' {
                        depth -= 1;
                        if depth == 0 { j += 1; break; }
                    }
                    j += 1;
                }
                let expr: String = chars[(i + 3)..(j - 1)].iter().collect();
                let result = eval_arithmetic(&expr, sh);
                out.push_str(&result.to_string());
                i = j;
                continue;
            }
            // $(...) command substitution
            if chars[i + 1] == '(' {
                // find matching close paren (naive: nest, respect quotes)
                let (body, end) = find_cmdsub_body(&chars[i..]);
                let cmd = body.iter().collect::<String>();
                // $(<file) reading a file
                if let Some(path) = cmd.strip_prefix('<') {
                    if let Ok(s) = std::fs::read_to_string(path.trim()) {
                        out.push_str(s.trim_end());
                    }
                } else {
                    out.push_str(&cmd_run(&cmd));
                }
                i += end;
                continue;
            }
            // ${var...}
            if chars[i + 1] == '{' {
                let (expr, end) = read_braced(&chars[i..]);
                out.push_str(&expand_braced_var(&expr, sh, cmd_run));
                i += end;
                continue;
            }
            // $var, $1, $? ...
            if is_name_start(chars[i + 1]) || chars[i + 1] == '?' || chars[i + 1].is_ascii_digit() {
                let mut j = i + 1;
                if chars[j] == '?' {
                    out.push_str(&sh.last_status.to_string());
                    i = j + 1;
                    continue;
                }
                if chars[j].is_ascii_digit() {
                    let idx: usize = chars[j].to_digit(10).unwrap() as usize;
                    if idx >= 1 {
                        out.push_str(sh.positional.get(idx - 1).map(String::as_str).unwrap_or(""));
                    }
                    i = j + 1;
                    continue;
                }
                while j < chars.len() && is_name_char(chars[j]) {
                    j += 1;
                }
                let name: String = chars[(i + 1)..j].iter().collect();
                if let Some(v) = sh.get(&name) {
                    out.push_str(v);
                }
                i = j;
                continue;
            }
        }
        if c == '~' && out.is_empty() {
            // tilde at word start → $HOME
            if let Some(h) = sh.get("HOME") {
                out.push_str(h);
            }
            i += 1;
            continue;
        }
        out.push(c);
        i += 1;
    }
    out
}

fn is_name_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}
fn is_name_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// Find the matching close paren for `$(`, returning (inner body, total len used including `$()`).
fn find_cmdsub_body(chars: &[char]) -> (Vec<char>, usize) {
    // chars[0..] starts with "$("
    let mut depth = 1;
    let mut body = Vec::new();
    let mut i = 2;
    while i < chars.len() {
        let c = chars[i];
        if c == '(' {
            depth += 1;
        } else if c == ')' {
            depth -= 1;
            if depth == 0 {
                return (body, i + 1);
            }
        }
        body.push(c);
        i += 1;
    }
    (body, chars.len())
}

/// Read `${...}` up to the matching `}`, returning (inner expr, total len used incl `${}`.
fn read_braced(chars: &[char]) -> (String, usize) {
    let mut expr = String::new();
    let mut i = 2; // skip ${
    while i < chars.len() && chars[i] != '}' {
        expr.push(chars[i]);
        i += 1;
    }
    if i < chars.len() { i += 1; }
    (expr, i)
}

fn expand_braced_var(expr: &str, sh: &Shell, cmd_run: &impl Fn(&str) -> String) -> String {
    // Array forms:
    //   ${#array[@]} — element count
    //   ${array[@]}   — join all elements (space-separated)
    //   ${array[i]}   — single element
    if let Some(rest) = expr.strip_prefix('#') {
        // ${#...} — could be array count or string length
        if let Some((name, _)) = split_array_index(rest) {
            if let Some(el) = sh.get_array(name) {
                return el.len().to_string();
            }
        }
        let value = expand_var_by_name(rest, sh);
        return value.chars().count().to_string();
    }
    if let Some((name, idx)) = split_array_index(expr) {
        let arr = sh.get_array(name).cloned().unwrap_or_default();
        if idx == "@" || idx == "*" {
            // join all elements with a space
            return arr.join(" ");
        }
        if let Ok(n) = idx.parse::<usize>() {
            if let Some(el) = arr.get(n) {
                return el.clone();
            }
        }
        return String::new();
    }
    // forms: name, name:-default, name#pat, name##pat, name/%/rep
    if let Some(pos) = expr.find(":-") {
        let name = &expr[..pos];
        let default = &expr[pos + 2..];
        let v = expand_var_by_name(name, sh);
        if v.is_empty() {
            return expand_word(default, sh, cmd_run);
        }
        return v;
    }
    if let Some(pos) = expr.find("##") {
        let name = &expr[..pos];
        let pat = &expr[pos + 2..];
        let v = expand_var_by_name(name, sh);
        // longest removal of leading pattern
        return strip_longest_prefix(&v, pat);
    }
    if let Some(pos) = expr.find("#") {
        let name = &expr[..pos];
        let pat = &expr[pos + 1..];
        let v = expand_var_by_name(name, sh);
        return strip_shortest_prefix(&v, pat);
    }
    if let Some(pos) = expr.find("%%") {
        // ${var%%pattern} remove longest suffix matching pattern
        let name = &expr[..pos];
        let pat = &expr[pos + 2..];
        let v = expand_var_by_name(name, sh);
        return strip_longest_suffix(&v, pat);
    }
    if let Some(pos) = expr.find('%') {
        // ${var%pattern} remove shortest suffix matching pattern
        let name = &expr[..pos];
        let pat = &expr[pos + 1..];
        let v = expand_var_by_name(name, sh);
        return strip_shortest_suffix(&v, pat);
    }
    if let Some(pos) = expr.find('/') {
        let name = &expr[..pos];
        let rest = &expr[pos + 1..];
        let (pat, rep) = match rest.find('/') {
            Some(p) => (&rest[..p], &rest[p + 1..]),
            None => (rest, ""),
        };
        let v = expand_var_by_name(name, sh);
        let rep = expand_word(rep, sh, cmd_run);
        return match replace_first(&v, pat, &rep) {
            Some(r) => r,
            None => v,
        };
    }
    // plain ${name} — resolve the variable
    expand_var_by_name(expr, sh)
}

fn strip_shortest_prefix(s: &str, pat: &str) -> String {
    // posix: ${var#pattern} removes the SHORTEST matching leading pattern
    let chars: Vec<char> = s.chars().collect();
    for i in 0..=chars.len() {
        let prefix: String = chars[..i].iter().collect();
        if wild_match(&prefix, pat) {
            return chars[i..].iter().collect();
        }
    }
    s.to_string()
}

fn strip_longest_prefix(s: &str, pat: &str) -> String {
    // ${var##pattern} removes the LONGEST matching leading pattern
    let chars: Vec<char> = s.chars().collect();
    let mut best: Option<usize> = None;
    for i in 0..=chars.len() {
        let prefix: String = chars[..i].iter().collect();
        if wild_match(&prefix, pat) {
            best = Some(i);
        }
    }
    match best {
        Some(i) => chars[i..].iter().collect(),
        None => s.to_string(),
    }
}

fn strip_shortest_suffix(s: &str, pat: &str) -> String {
    // ${var%pattern} removes the SHORTEST matching trailing pattern
    let chars: Vec<char> = s.chars().collect();
    for i in (0..=chars.len()).rev() {
        let suffix: String = chars[i..].iter().collect();
        if wild_match(&suffix, pat) {
            return chars[..i].iter().collect();
        }
    }
    s.to_string()
}

fn strip_longest_suffix(s: &str, pat: &str) -> String {
    // ${var%%pattern} removes the LONGEST matching trailing pattern
    let chars: Vec<char> = s.chars().collect();
    let mut best: Option<usize> = None;
    for i in 0..=chars.len() {
        let suffix: String = chars[i..].iter().collect();
        if wild_match(&suffix, pat) {
            best = Some(i);
        }
    }
    match best {
        Some(i) => chars[..i].iter().collect(),
        None => s.to_string(),
    }
}

/// If `s` looks like `name[idx]` (array element access), return (name, index).
/// Index is `@`/`*` for whole-array, or a numeric string.
fn split_array_index(s: &str) -> Option<(&str, String)> {
    let open = s.find('[')?;
    if !s.ends_with(']') || open + 1 >= s.len().saturating_sub(1) { return None; }
    let name = &s[..open];
    let idx_str = &s[open + 1..s.len() - 1];
    if idx_str == "@" || idx_str == "*" {
        return Some((name, idx_str.to_string()));
    }
    let _: usize = idx_str.parse().ok()?;
    Some((name, idx_str.to_string()))
}

/// Resolve a bare variable name (used by `${name}`). Returns its value or "".
fn expand_var_by_name(name: &str, sh: &Shell) -> String {
    if name.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
        let idx: usize = name.parse().unwrap_or(1);
        if idx >= 1 {
            return sh.positional.get(idx - 1).cloned().unwrap_or_else(String::new);
        }
    }
    if let Some(v) = sh.get(name) {
        v.to_string()
    } else {
        String::new()
    }
}

fn replace_first(hay: &str, pat: &str, rep: &str) -> Option<String> {
    // match pattern against prefixes
    let chars: Vec<char> = hay.chars().collect();
    for i in 0..=chars.len() {
        let rest: String = chars[i..].iter().collect();
        if wild_match(&rest, pat) {
            let prefix: String = chars[..i].iter().collect();
            return Some(format!("{prefix}{rep}"));
        }
    }
    None
}

/// Globbing pattern matcher: supports `*`, `?`, `[...]`.
pub fn wild_match(s: &str, pat: &str) -> bool {
    let s: Vec<char> = s.chars().collect();
    let p: Vec<char> = pat.chars().collect();
    wild_match_inner(&s, &p)
}

fn wild_match_inner(s: &[char], p: &[char]) -> bool {
    let (mut si, mut pi) = (0, 0);
    let (mut star_si, mut star_pi) = (usize::MAX, usize::MAX);
    while si < s.len() {
        if pi < p.len() && (p[pi] == '?' || p[pi] == s[si]) {
            si += 1; pi += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star_si = si; star_pi = pi; pi += 1;
        } else if star_pi != usize::MAX {
            star_si += 1;
            si = star_si; pi = star_pi + 1;
        } else {
            return false;
        }
    }
    while pi < p.len() && p[pi] == '*' { pi += 1; }
    pi == p.len()
}

/// Expand a word into argv pieces: perform expansion, then split on
/// whitespace (unless the original word was quoted; we approximate by always
/// splitting). Command substitution output IS split on whitespace.
pub fn split_expanded(word: &str, sh: &Shell, cmd_run: &impl Fn(&str) -> String) -> Vec<String> {
    let mut words = Vec::new();
    let mut cur = String::new();
    let mut chars = word.chars().peekable();
    let mut in_word = false;
    while let Some(c) = chars.next() {
        if c.is_whitespace() {
            if in_word {
                words.push(std::mem::take(&mut cur));
                in_word = false;
            }
        } else {
            // handle glob: if any * present, we should glob-expand later.
            cur.push(c);
            in_word = true;
        }
    }
    if in_word { words.push(cur); }
    if words.is_empty() { words.push(String::new()); }
    words
        .into_iter()
        .map(|w| glob_one(&w))
        .flatten()
        .collect()
}

/// Expand every `*`/`?` token against the filesystem; if no metachars, return as-is.
pub fn glob_one(word: &str) -> Vec<String> {
    if !word.contains('*') && !word.contains('?') && !word.contains('[') {
        return vec![word.to_string()];
    }
    // naive: glob against CWD with the pattern as a relative path
    let cwd = std::env::current_dir().unwrap_or_default();
    let mut out = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&cwd) {
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if wild_match(&name, word) {
                out.push(name);
            }
        }
    }
    if out.is_empty() {
        vec![word.to_string()]
    } else {
        out
    }
}

/// Evaluate a bash arithmetic expression. Supports +, -, *, /, %, **,
/// parentheses, variable references, and comparison operators.
/// Variables are read from the shell; unknown vars default to 0.
pub fn eval_arithmetic(expr: &str, sh: &Shell) -> i64 {
    let tokens = arith_tokenize(expr, sh);
    let (val, _) = arith_parse_expr(&tokens, 0);
    val
}

#[derive(Debug, Clone)]
enum ATok {
    Num(i64),
    Op(String),
    LParen,
    RParen,
}

fn arith_tokenize(expr: &str, sh: &Shell) -> Vec<ATok> {
    let mut toks = Vec::new();
    let chars: Vec<char> = expr.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() { i += 1; continue; }
        if c.is_ascii_digit() {
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '_') { i += 1; }
            let s: String = chars[start..i].iter().collect::<String>().replace('_', "");
            toks.push(ATok::Num(s.parse::<i64>().unwrap_or(0)));
            continue;
        }
        if c.is_ascii_alphabetic() || c == '_' {
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') { i += 1; }
            let name: String = chars[start..i].iter().collect();
            match name.as_str() {
                "eq" | "ne" | "gt" | "lt" | "ge" | "le" => {
                    toks.push(ATok::Op(name));
                }
                _ => {
                    // Resolve variable to its integer value (0 if unset or non-numeric)
                    let val = sh.get(&name)
                        .and_then(|v| v.parse::<i64>().ok())
                        .unwrap_or(0);
                    toks.push(ATok::Num(val));
                }
            }
            continue;
        }
        if c == '(' { toks.push(ATok::LParen); i += 1; continue; }
        if c == ')' { toks.push(ATok::RParen); i += 1; continue; }
        // multi-char operators
        if c == '*' && i + 1 < chars.len() && chars[i + 1] == '*' {
            toks.push(ATok::Op("**".into())); i += 2; continue;
        }
        if (c == '=' || c == '!' || c == '<' || c == '>') && i + 1 < chars.len() && chars[i + 1] == '=' {
            let op = format!("{}=", c);
            toks.push(ATok::Op(op)); i += 2; continue;
        }
        // single-char operators
        if matches!(c, '+' | '-' | '*' | '/' | '%' | '|' | '&' | '^' | '~' | '<' | '>' | '=' | '!') {
            toks.push(ATok::Op(c.to_string())); i += 1; continue;
        }
        i += 1; // skip unknown
    }
    toks
}

/// Simple recursive-descent arithmetic parser. Returns (value, next_pos).
fn arith_parse_expr(toks: &[ATok], pos: i64) -> (i64, i64) {
    let mut pos = pos as usize;
    let (mut val, mut p) = arith_parse_comparison(toks, pos);
    pos = p;
    // ternary: a ? b : c
    while pos < toks.len() {
        if let ATok::Op(op) = &toks[pos] {
            if op == "?" {
                pos += 1;
                let (t_val, p2) = arith_parse_comparison(toks, pos);
                pos = p2;
                if pos < toks.len() && matches!(&toks[pos], ATok::Op(op2) if op2 == ":") {
                    pos += 1;
                    let (f_val, p3) = arith_parse_comparison(toks, pos);
                    pos = p3;
                    val = if val != 0 { t_val } else { f_val };
                    continue;
                }
            }
        }
        break;
    }
    (val, pos as i64)
}

fn arith_parse_comparison(toks: &[ATok], mut pos: usize) -> (i64, usize) {
    let (mut val, mut p) = arith_parse_or(toks, pos);
    pos = p;
    while pos < toks.len() {
        if let ATok::Op(op) = &toks[pos] {
            match op.as_str() {
                "==" | "eq" => { pos += 1; let (r, p2) = arith_parse_or(toks, pos); pos = p2; val = if val == r { 1 } else { 0 }; }
                "!=" | "ne" => { pos += 1; let (r, p2) = arith_parse_or(toks, pos); pos = p2; val = if val != r { 1 } else { 0 }; }
                "<" | "lt" => { pos += 1; let (r, p2) = arith_parse_or(toks, pos); pos = p2; val = if val < r { 1 } else { 0 }; }
                ">" | "gt" => { pos += 1; let (r, p2) = arith_parse_or(toks, pos); pos = p2; val = if val > r { 1 } else { 0 }; }
                "<=" | "le" => { pos += 1; let (r, p2) = arith_parse_or(toks, pos); pos = p2; val = if val <= r { 1 } else { 0 }; }
                ">=" | "ge" => { pos += 1; let (r, p2) = arith_parse_or(toks, pos); pos = p2; val = if val >= r { 1 } else { 0 }; }
                _ => break,
            }
        } else { break; }
    }
    (val, pos)
}

fn arith_parse_or(toks: &[ATok], mut pos: usize) -> (i64, usize) {
    let (mut val, mut p) = arith_parse_and(toks, pos);
    pos = p;
    while pos < toks.len() {
        if let ATok::Op(op) = &toks[pos] {
            if op == "|" {
                pos += 1;
                let (r, p2) = arith_parse_and(toks, pos);
                pos = p2;
                val |= r;
                continue;
            }
        }
        break;
    }
    (val, pos)
}

fn arith_parse_and(toks: &[ATok], mut pos: usize) -> (i64, usize) {
    let (mut val, mut p) = arith_parse_xor(toks, pos);
    pos = p;
    while pos < toks.len() {
        if let ATok::Op(op) = &toks[pos] {
            if op == "&" {
                pos += 1;
                let (r, p2) = arith_parse_xor(toks, pos);
                pos = p2;
                val &= r;
                continue;
            }
        }
        break;
    }
    (val, pos)
}

fn arith_parse_xor(toks: &[ATok], mut pos: usize) -> (i64, usize) {
    let (mut val, mut p) = arith_parse_shift(toks, pos);
    pos = p;
    while pos < toks.len() {
        if let ATok::Op(op) = &toks[pos] {
            if op == "^" {
                pos += 1;
                let (r, p2) = arith_parse_shift(toks, pos);
                pos = p2;
                val ^= r;
                continue;
            }
        }
        break;
    }
    (val, pos)
}

fn arith_parse_shift(toks: &[ATok], mut pos: usize) -> (i64, usize) {
    let (mut val, mut p) = arith_parse_addsub(toks, pos);
    pos = p;
    while pos < toks.len() {
        if let ATok::Op(op) = &toks[pos] {
            if op == "<<" { pos += 1; let (r, p2) = arith_parse_addsub(toks, pos); pos = p2; val <<= r; continue; }
            if op == ">>" { pos += 1; let (r, p2) = arith_parse_addsub(toks, pos); pos = p2; val >>= r; continue; }
        }
        break;
    }
    (val, pos)
}

fn arith_parse_addsub(toks: &[ATok], mut pos: usize) -> (i64, usize) {
    let (mut val, mut p) = arith_parse_muldiv(toks, pos);
    pos = p;
    while pos < toks.len() {
        if let ATok::Op(op) = &toks[pos] {
            match op.as_str() {
                "+" => { pos += 1; let (r, p2) = arith_parse_muldiv(toks, pos); pos = p2; val += r; }
                "-" => { pos += 1; let (r, p2) = arith_parse_muldiv(toks, pos); pos = p2; val -= r; }
                _ => break,
            }
        } else { break; }
    }
    (val, pos)
}

fn arith_parse_muldiv(toks: &[ATok], mut pos: usize) -> (i64, usize) {
    let (mut val, mut p) = arith_parse_power(toks, pos);
    pos = p;
    while pos < toks.len() {
        if let ATok::Op(op) = &toks[pos] {
            match op.as_str() {
                "*" => { pos += 1; let (r, p2) = arith_parse_power(toks, pos); pos = p2; val = val.wrapping_mul(r); }
                "/" => { pos += 1; let (r, p2) = arith_parse_power(toks, pos); pos = p2; val = if r != 0 { val / r } else { 0 }; }
                "%" => { pos += 1; let (r, p2) = arith_parse_power(toks, pos); pos = p2; val = if r != 0 { val % r } else { 0 }; }
                _ => break,
            }
        } else { break; }
    }
    (val, pos)
}

fn arith_parse_power(toks: &[ATok], mut pos: usize) -> (i64, usize) {
    let (val, mut p) = arith_parse_unary(toks, pos);
    pos = p;
    if pos < toks.len() {
        if let ATok::Op(ref op) = toks[pos] {
            if op == "**" {
                pos += 1;
                let (r, p2) = arith_parse_power(toks, pos); // right-assoc
                pos = p2;
                return (val.pow(r as u32), pos);
            }
        }
    }
    (val, pos)
}

fn arith_parse_unary(toks: &[ATok], mut pos: usize) -> (i64, usize) {
    if pos < toks.len() {
        if let ATok::Op(ref op) = toks[pos] {
            if op == "-" {
                pos += 1;
                let (val, p) = arith_parse_unary(toks, pos);
                return (-val, p);
            }
            if op == "+" {
                pos += 1;
                return arith_parse_unary(toks, pos);
            }
            if op == "~" {
                pos += 1;
                let (val, p) = arith_parse_unary(toks, pos);
                return (!val, p);
            }
            if op == "!" {
                pos += 1;
                let (val, p) = arith_parse_unary(toks, pos);
                return (if val == 0 { 1 } else { 0 }, p);
            }
        }
    }
    arith_parse_primary(toks, pos)
}

fn arith_parse_primary(toks: &[ATok], pos: usize) -> (i64, usize) {
    if pos >= toks.len() {
        return (0, pos);
    }
    match &toks[pos] {
        ATok::Num(n) => (*n, pos + 1),
        ATok::LParen => {
            let (val, p) = arith_parse_expr(toks, pos as i64 + 1);
            let mut p = p as usize;
            if p < toks.len() && matches!(&toks[p], ATok::RParen) { p += 1; }
            (val, p)
        }
        _ => (0, pos + 1),
    }
}
