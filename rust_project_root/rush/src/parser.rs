//! Lexer: turn a source line into a token stream that respects shell quoting,
//! escapes, comments and operators. Compound structures (if/for/while/case,
//! functions, `[[ ]]`) are evaluated at a higher level.

use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Tok {
    /// A word. May contain unexpanded `$var`, `$(...)`, quotes are decoded away.
    Word(String),
    /// `|`
    Pipe,
    /// `||`
    OrIf,
    /// `&&`
    AndIf,
    /// `;`
    Semi,
    /// `&`
    Amp,
    /// `(`
    LParen,
    /// `)`
    RParen,
    /// `>` (truncate stdout)
    Gt,
    /// `>>` (append stdout)
    GtGt,
    /// `<`
    Lt,
    /// `<<<` here-string
    HereStr,
    /// `2>` stderr
    ErrGt,
    /// `\n` logical separator (from `;` or newline)
    Newline,
    /// reserved keywords we care about (already lowercase)
    Kw(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub tok: Tok,
    pub src: String,
}

impl fmt::Display for Tok {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

fn is_word_char(c: char) -> bool {
    !c.is_whitespace() && !matches!(c, '|' | '&' | ';' | '(' | ')' | '<' | '>' | '#')
}

/// Lex a line into tokens. `full` = whole logical statement (may span the
/// entire input); used by the executor to detect keywords.
pub fn lex(line: &str) -> Result<Vec<Token>, String> {
    let mut toks = Vec::new();
    let mut chars = line.chars().peekable();
    let mut i = 0usize; // byte index for src spans

    while let Some(c) = chars.peek().copied() {
        // skip blanks
        if c == ' ' || c == '\t' {
            chars.next();
            i += 1;
            continue;
        }
        // comment: '#' at start of a word position ends the line
        if c == '#' {
            break;
        }
        // newline
        if c == '\n' {
            toks.push(Token { tok: Tok::Newline, src: "\n".into() });
            chars.next();
            i += 1;
            continue;
        }

        // operators
        match c {
            '|' => {
                chars.next(); i += 1;
                if chars.peek() == Some(&'|') {
                    chars.next(); i += 1;
                    toks.push(op("||", Tok::OrIf));
                } else {
                    toks.push(op("|", Tok::Pipe));
                }
                continue;
            }
            '&' => {
                chars.next(); i += 1;
                if chars.peek() == Some(&'&') {
                    chars.next(); i += 1;
                    toks.push(op("&&", Tok::AndIf));
                } else {
                    toks.push(op("&", Tok::Amp));
                }
                continue;
            }
            ';' => {
                chars.next(); i += 1;
                toks.push(op(";", Tok::Semi));
                continue;
            }
            '(' => {
                chars.next(); i += 1;
                toks.push(op("(", Tok::LParen));
                continue;
            }
            ')' => {
                chars.next(); i += 1;
                toks.push(op(")", Tok::RParen));
                continue;
            }
            '<' => {
                chars.next(); i += 1;
                if chars.peek() == Some(&'<') {
                    chars.next(); i += 1;
                    if chars.peek() == Some(&'<') {
                        chars.next(); i += 1;
                        toks.push(op("<<<", Tok::HereStr));
                    } else {
                        return Err("unsupported <<".into());
                    }
                } else {
                    toks.push(op("<", Tok::Lt));
                }
                continue;
            }
            '>' => {
                chars.next(); i += 1;
                if chars.peek() == Some(&'>') {
                    chars.next(); i += 1;
                    toks.push(op(">>", Tok::GtGt));
                } else {
                    toks.push(op(">", Tok::Gt));
                }
                continue;
            }
            _ => {}
        }

        // '2>' etc → ErrGt (we special-case the leading digit 2 followed by >)
        if c == '2' {
            // detect if next non-space is '>'
            let mut save = chars.clone();
            save.next();
            while let Some(sc) = save.peek() {
                if *sc == ' ' || *sc == '\t' {
                    save.next();
                    continue;
                }
                break;
            }
            if save.peek() == Some(&'>') {
                // consume '2', then '>'
                chars.next(); i += 1;
                chars.next(); i += 1;
                toks.push(op("2>", Tok::ErrGt));
                continue;
            }
        }

        // otherwise: a word. Build until an operator / whitespace / quote end.
        let start = i;
        let mut word = String::new();
        let mut quote = None; // Some(') or Some(")
        loop {
            let Some(&ch) = chars.peek() else { break };
            if let Some(q) = quote {
                // inside quotes: only the matching quote or backslash/a dollar end matter
                if ch == '\\' {
                    chars.next(); i += 1;
                    if let Some(n) = chars.next() { word.push(n); i += 1; }
                    continue;
                }
                if ch == q {
                    chars.next(); i += 1;
                    quote = None;
                    continue;
                }
                chars.next(); i += 1;
                word.push(ch);
                continue;
            } else {
                if ch == '\'' || ch == '"' {
                    quote = Some(ch);
                    chars.next(); i += 1;
                    continue;
                }
                if ch == '\\' {
                    chars.next(); i += 1;
                    if let Some(n) = chars.next() { word.push(n); i += 1; }
                    continue;
                }
                if ch.is_whitespace() || matches!(ch, '|'|'&'|';'|'('|')'|'<'|'>'|'#') {
                    break;
                }
                chars.next(); i += 1;
                word.push(ch);
                // Handle $(...) and ${...} inline so the parens stay in the word
                if ch == '$' {
                    if let Some(&nc) = chars.peek() {
                        if nc == '(' {
                            // consume $( ... ) with nesting
                            chars.next(); i += 1;
                            word.push('(');
                            let mut depth = 1u32;
                            while let Some(&c) = chars.peek() {
                                chars.next(); i += 1;
                                word.push(c);
                                if c == '(' { depth += 1; }
                                else if c == ')' {
                                    depth -= 1;
                                    if depth == 0 { break; }
                                }
                            }
                            continue;
                        }
                        if nc == '{' {
                            // consume ${ ... }
                            chars.next(); i += 1;
                            word.push('{');
                            while let Some(&c) = chars.peek() {
                                chars.next(); i += 1;
                                word.push(c);
                                if c == '}' { break; }
                            }
                            continue;
                        }
                    }
                }
            }
        }

        let _ = start;
        if word.is_empty() {
            // this can happen for a dangling quote; just skip
            continue;
        }
        toks.push(Token { tok: Tok::Word(word.clone()), src: word });
        let _ = i;
    }

    // Decode keyword words into Kw (lowercased) when at command-start positions.
    // To keep it simple, the executor decides keywords by comparing Word text,
    // so we do NOT relabel here.
    Ok(toks)
}

fn op(src: &str, t: Tok) -> Token {
    Token { tok: t, src: src.into() }
}

/// Minimal formatter: join tokens with spaces into a printable line.
pub fn tokens_to_line(toks: &[Token]) -> String {
    let mut s = String::new();
    for t in toks {
        match &t.tok {
            Tok::Newline => s.push(';'),
            Tok::Word(w) => {
                if !s.is_empty() && !s.ends_with(';') { s.push(' '); }
                s.push_str(w);
            }
            other => {
                if !s.is_empty() && !s.ends_with(';') { s.push(' '); }
                s.push_str(&other.to_string());
            }
        }
    }
    if s.is_empty() { s.push(';'); }
    s
}

// re-export for other modules
pub use Tok as TokKind;
