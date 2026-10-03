//! The scene binding language — what a `Val` expression means.
//!
//! A binding is a string, and the string is a tiny expression over the names
//! the shell publishes each frame ([`SceneValues`]). A bare name is the common
//! case and reads like Quickshell's `shell.ws_reg_w`; the rest is arithmetic
//! so a scene can own a value that used to have to be computed in Rust and
//! published as a literal — the workspace chip's click region is
//! `ws_n > 1 ? 26 : 20`, which was `pill_ws_reg_w` and a `w_var` until this
//! landed.
//!
//! Deliberately small: numbers, names, `+ - * /`, comparisons, `&& || !`,
//! `?:`, and `min / max / clamp / abs / round / floor / ceil / len`. No
//! assignment, no calls into anything but that list, no way to fail at draw
//! time — a name that resolves to nothing is 0, exactly as `vals.scalar(name)`
//! returning `None` and the caller falling back was. Grammar:
//!
//! ```text
//! expr    := ternary
//! ternary := logic ('?' expr ':' expr)?
//! logic   := cmp (('&&' | '||') cmp)*
//! cmp     := sum (('<' | '<=' | '>' | '>=' | '==' | '!=') sum)?
//! sum     := term (('+' | '-') term)*
//! term    := unary (('*' | '/') unary)*
//! unary   := ('-' | '!') unary | primary
//! primary := number | ident | ident '(' expr (',' expr)* ')' | '(' expr ')'
//! ```
//!
//! Compiled once per distinct binding string and cached for the process, so
//! a frame pays a walk over a small tree, never a parse.

use super::SceneValues;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

/// A parsed binding. Public so a diagnostic can name the node that failed.
#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    Num(f32),
    /// A published property name (also the argument of `len`).
    Name(String),
    Neg(Box<Expr>),
    Not(Box<Expr>),
    Arith(&'static str, Box<Expr>, Box<Expr>),
    Cmp(&'static str, Box<Expr>, Box<Expr>),
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
    Ternary(Box<Expr>, Box<Expr>, Box<Expr>),
    Call(&'static str, Vec<Expr>),
}

/// A binding that would not parse, with the character offset where it stopped
/// making sense. Rendered by the load-time validator and by
/// `Val::compile`'s caller.
#[derive(Clone, Debug, PartialEq)]
pub struct SyntaxError {
    pub at: usize,
    pub what: String,
}

impl std::fmt::Display for SyntaxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} (at char {})", self.what, self.at)
    }
}

// ── the parser ────────────────────────────────────────────────────────────────

struct P<'a> {
    s: &'a [u8],
    i: usize,
}

impl<'a> P<'a> {
    fn new(s: &'a str) -> Self {
        P {
            s: s.as_bytes(),
            i: 0,
        }
    }

    fn err<T>(&self, what: &str) -> Result<T, SyntaxError> {
        Err(SyntaxError {
            at: self.i,
            what: what.into(),
        })
    }

    /// Spaces and tabs, and a `_` as a digit separator (`1_000`). RON spells
    /// numbers the same way, so a binding can be lifted out of a file.
    fn ws(&mut self) {
        while self.i < self.s.len() {
            match self.s[self.i] {
                b' ' | b'\t' | b'\n' | b'\r' => self.i += 1,
                b'_' if self.i + 1 < self.s.len() && self.s[self.i + 1].is_ascii_digit() => {
                    self.i += 1
                }
                _ => break,
            }
        }
    }

    fn eat(&mut self, two: &str) -> bool {
        self.ws();
        let t = two.as_bytes();
        if self.s.len() - self.i >= t.len() && &self.s[self.i..self.i + t.len()] == t {
            self.i += t.len();
            true
        } else {
            false
        }
    }

    /// A two-char operator first, so `<=` never parses as `<` then `=`.
    fn eat2(&mut self, op: &str) -> bool {
        let t = op.as_bytes();
        if self.s.len() - self.i >= t.len() && &self.s[self.i..self.i + t.len()] == t {
            self.i += t.len();
            true
        } else {
            false
        }
    }

    fn peek(&mut self) -> Option<u8> {
        self.ws();
        self.s.get(self.i).copied()
    }

    fn expr(&mut self) -> Result<Expr, SyntaxError> {
        let c = self.ternary()?;
        // `?:` is right-associative and its arms are full expressions
        if self.eat("?") {
            let a = self.expr()?;
            if !self.eat(":") {
                return self.err("expected `:` in `? :`");
            }
            let b = self.expr()?;
            return Ok(Expr::Ternary(Box::new(c), Box::new(a), Box::new(b)));
        }
        Ok(c)
    }

    fn ternary(&mut self) -> Result<Expr, SyntaxError> {
        self.logic()
    }

    fn logic(&mut self) -> Result<Expr, SyntaxError> {
        let mut lhs = self.cmp()?;
        loop {
            if self.eat("&&") {
                let r = self.cmp()?;
                lhs = Expr::And(Box::new(lhs), Box::new(r));
            } else if self.eat("||") {
                let r = self.cmp()?;
                lhs = Expr::Or(Box::new(lhs), Box::new(r));
            } else {
                return Ok(lhs);
            }
        }
    }

    fn cmp(&mut self) -> Result<Expr, SyntaxError> {
        let lhs = self.sum()?;
        for op in ["<=", ">=", "==", "!=", "<", ">"] {
            if self.eat2(op) {
                let rhs = self.sum()?;
                return Ok(Expr::Cmp(op, Box::new(lhs), Box::new(rhs)));
            }
        }
        Ok(lhs)
    }

    fn sum(&mut self) -> Result<Expr, SyntaxError> {
        let mut lhs = self.term()?;
        loop {
            self.ws();
            let op = match self.s.get(self.i) {
                Some(b'+') => "+",
                Some(b'-') => "-",
                _ => return Ok(lhs),
            };
            self.i += 1;
            let rhs = self.term()?;
            lhs = Expr::Arith(op, Box::new(lhs), Box::new(rhs));
        }
    }

    fn term(&mut self) -> Result<Expr, SyntaxError> {
        let mut lhs = self.unary()?;
        loop {
            self.ws();
            let op = match self.s.get(self.i) {
                Some(b'*') => "*",
                Some(b'/') => "/",
                _ => return Ok(lhs),
            };
            self.i += 1;
            let rhs = self.unary()?;
            lhs = Expr::Arith(op, Box::new(lhs), Box::new(rhs));
        }
    }

    fn unary(&mut self) -> Result<Expr, SyntaxError> {
        self.ws();
        match self.s.get(self.i) {
            Some(b'-') => {
                self.i += 1;
                Ok(Expr::Neg(Box::new(self.unary()?)))
            }
            Some(b'!') => {
                self.i += 1;
                Ok(Expr::Not(Box::new(self.unary()?)))
            }
            _ => self.primary(),
        }
    }

    fn primary(&mut self) -> Result<Expr, SyntaxError> {
        self.ws();
        let Some(&c) = self.s.get(self.i) else {
            return self.err("expected a value, found end of binding");
        };
        if c == b'(' {
            self.i += 1;
            let e = self.expr()?;
            if !self.eat(")") {
                return self.err("expected `)`");
            }
            return Ok(e);
        }
        if c.is_ascii_digit() || c == b'.' {
            return self.number();
        }
        if c.is_ascii_alphabetic() || c == b'_' {
            return self.ident();
        }
        self.err(&format!(
            "expected a number or a name, found `{}`",
            c as char
        ))
    }

    fn number(&mut self) -> Result<Expr, SyntaxError> {
        let start = self.i;
        while self.i < self.s.len() && (self.s[self.i].is_ascii_digit() || self.s[self.i] == b'.') {
            self.i += 1;
        }
        if self.i < self.s.len() && (self.s[self.i] | 0x20) == b'e' {
            let sign = self.i;
            self.i += 1;
            if self.i < self.s.len() && (self.s[self.i] == b'+' || self.s[self.i] == b'-') {
                self.i += 1;
            }
            if self.i < self.s.len() && self.s[self.i].is_ascii_digit() {
                while self.i < self.s.len() && self.s[self.i].is_ascii_digit() {
                    self.i += 1;
                }
            } else {
                self.i = sign; // a trailing `e` is not an exponent
            }
        }
        let text: String = std::str::from_utf8(&self.s[start..self.i])
            .unwrap_or("")
            .chars()
            .filter(|c| *c != '_')
            .collect();
        match text.parse::<f32>() {
            Ok(v) => Ok(Expr::Num(v)),
            Err(_) => self.err(&format!("`{text}` is not a number")),
        }
    }

    fn ident(&mut self) -> Result<Expr, SyntaxError> {
        let start = self.i;
        while self.i < self.s.len()
            && (self.s[self.i].is_ascii_alphanumeric() || self.s[self.i] == b'_')
        {
            self.i += 1;
        }
        // A DOTTED name is one name, not a name followed by garbage. The only
        // dotted names that exist are scope names — `item.<field>` of a
        // delegate's model row — and without this the parser reads `item` and
        // then chokes on the dot, which would make every delegate binding a
        // load-time `BadBinding` while still resolving correctly at draw (the
        // scope folds the field away before the expression is ever evaluated).
        // A trailing dot is NOT part of the name, so `item.` stays an error
        // rather than becoming a name no row can carry.
        while self.s.get(self.i) == Some(&b'.')
            && self
                .s
                .get(self.i + 1)
                .is_some_and(|c| c.is_ascii_alphabetic() || *c == b'_')
        {
            self.i += 1;
            while self.i < self.s.len()
                && (self.s[self.i].is_ascii_alphanumeric() || self.s[self.i] == b'_')
            {
                self.i += 1;
            }
        }
        let name = std::str::from_utf8(&self.s[start..self.i])
            .unwrap_or("")
            .to_string();
        self.ws();
        if self.s.get(self.i) != Some(&b'(') {
            return Ok(Expr::Name(name));
        }
        self.i += 1;
        let Some(fn_name) = FNS.iter().find(|(n, _, _)| *n == name) else {
            return self.err(&format!("`{name}` is not a binding function"));
        };
        let mut args = Vec::new();
        if self.peek() != Some(b')') {
            loop {
                args.push(self.expr()?);
                if !self.eat(",") {
                    break;
                }
            }
        }
        if !self.eat(")") {
            return self.err(&format!("expected `)` closing `{name}("));
        }
        if args.len() != fn_name.2 {
            return self.err(&format!(
                "`{name}` takes {} argument(s), got {}",
                fn_name.2,
                args.len()
            ));
        }
        Ok(Expr::Call(fn_name.1, args))
    }
}

/// The callables, with their arity — the single list a `primary` accepts, so
/// an unknown name is a load-time error rather than a silent 0.
const FNS: &[(&str, &'static str, usize)] = &[
    ("min", "min", 2),
    ("max", "max", 2),
    ("clamp", "clamp", 3),
    ("abs", "abs", 1),
    ("round", "round", 1),
    ("floor", "floor", 1),
    ("ceil", "ceil", 1),
    ("len", "len", 1),
];

/// Parse a binding. `Err` carries the offset so a diagnostic can point at it.
pub fn parse(src: &str) -> Result<Expr, SyntaxError> {
    let mut p = P::new(src);
    let e = p.expr()?;
    p.ws();
    if p.i < p.s.len() {
        return p.err("trailing characters after the expression");
    }
    Ok(e)
}

/// Fold scope-provided names into a binding's source text. `None` when the
/// binding reads nothing the scope could answer, so an unscoped item never
/// reallocates and keeps its one shared compiled tree.
///
/// This is the delegate-instantiation path (`QUICKSHELL_MODEL.md` §2): a row's
/// fields are concrete numbers this frame, so they are folded into the
/// delegate's binding once per row instead of being resolved through a pool at
/// draw time. A name the scope does not know is left alone, so a mixed binding
/// (`"item.w - pill_gap"`) keeps `pill_gap` for the frame's values.
///
/// `Some` means **the binding TOUCHES the scope**, which is the question the
/// caller actually needs answered: a rewritten name certainly does, and so
/// does a scope-ONLY name the row does not carry (`item.h` on a row with no
/// `h`), because that is a field the delegate asked for and the model does not
/// have — the diagnostic the caller reports rather than a silent 0. A bare name
/// is the other half of the pool's namespace, so not knowing it says nothing
/// about the scope and leaves the binding alone.
///
/// Exact rather than approximate because the grammar has no string literals:
/// an `[A-Za-z_][A-Za-z0-9_]*` run — with `.`-joined segments, so `item.w` is
/// ONE name and not `item` `.` `w` — is always a name, and the only rewrite is
/// name → number. `min(…)` and friends survive because a function name is
/// never in a scope's map.
pub fn fold(src: &str, resolve: impl Fn(&str) -> Option<f32>) -> Option<String> {
    rewrite(src, |name| match resolve(name) {
        // a non-finite value would fold to `NaN` / `inf`, which this grammar has
        // no token for — so the WHOLE rewrite stands down and every name is left
        // for the pool, which resolves it 0 like any other name nobody publishes.
        // Aborting the entire rewrite rather than skipping that one name is
        // load-bearing: a half-substituted binding would draw a plausible number
        // computed from a `NaN`.
        Some(v) if !v.is_finite() => Step::Abort,
        Some(v) => Step::Replace(format!("{v:?}")),
        None => Step::Keep,
    })
}

/// [`fold`] generalized from a *number* to an arbitrary replacement **string**.
///
/// The split exists because the two scope kinds substitute differently, and
/// conflating them would be a bug in one of them:
///
/// - a model row's field is a concrete value, so it folds to a number — the
///   row IS this frame's answer;
/// - a component's prop is a [`crate::scene::Val`], and a prop that is itself
///   an expression (`w: "pill_ws_reg_w"`) must stay an expression. Substituting
///   a resolved number would freeze one frame's value into the scene and every
///   later frame would keep drawing it.
///
/// Keeping both on one name-walker means a prop gets the *same* coverage as a
/// row for free — including the compiler-consistent name runs — instead of a
/// second hand-written field walk that would have to be kept in sync.
pub fn subst(src: &str, resolve: impl Fn(&str) -> Option<String>) -> Option<String> {
    rewrite(src, |name| match resolve(name) {
        Some(rep) => Step::Replace(rep),
        None => Step::Keep,
    })
}

/// What a name-walker does with one name it found.
enum Step {
    /// Leave the name exactly as written — it belongs to the pool.
    Keep,
    /// Substitute this text in its place.
    Replace(String),
    /// Abandon the entire rewrite and leave the binding untouched.
    Abort,
}

/// The single name-walker every scope rewrite goes through: substitute names a
/// scope answers, leave the rest verbatim, and report whether anything was
/// touched.
///
/// One walker because the three callers must agree on what a NAME is. If
/// `fold` and `subst` scanned names differently, a prop named `w` would be
/// substituted inside `max(w, 8)`'s function name by one and not the other —
/// the class of bug where the compiler and the rewriter disagree about the
/// grammar, which is exactly what the validator exists to prevent.
fn rewrite(src: &str, mut step: impl FnMut(&str) -> Step) -> Option<String> {
    let mut out = String::with_capacity(src.len());
    let mut rest = src;
    let mut touched = false;
    while !rest.is_empty() {
        let c = rest.chars().next().unwrap();
        if c.is_ascii_alphabetic() || c == '_' {
            let mut end = rest
                .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                .unwrap_or(rest.len());
            // a scope name is `item.<field>`: take the dotted tail with it, or
            // `item.w` would fold as the bare name `item` and leave `.w` behind
            while rest[end..].starts_with('.')
                && rest[end + 1..]
                    .chars()
                    .next()
                    .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
            {
                end += 1 + rest[end + 1..]
                    .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                    .unwrap_or(rest.len() - end - 1);
            }
            let (name, tail) = rest.split_at(end);
            match step(name) {
                Step::Abort => return None,
                Step::Replace(rep) => {
                    out.push_str(&rep);
                    touched = true;
                }
                Step::Keep => {
                    // a dotted name can only have come from a scope (the flat
                    // pool has no dotted names), so the binding is scope-bound
                    // whether or not this row carries the field
                    touched |= name.contains('.');
                    out.push_str(name);
                }
            }
            rest = tail;
        } else {
            out.push(c);
            rest = &rest[c.len_utf8()..];
        }
    }
    if touched {
        Some(out)
    } else {
        None
    }
}

// ── compilation cache ─────────────────────────────────────────────────────────

fn cache() -> &'static Mutex<HashMap<String, Result<Arc<Expr>, SyntaxError>>> {
    static C: OnceLock<Mutex<HashMap<String, Result<Arc<Expr>, SyntaxError>>>> = OnceLock::new();
    C.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Parse `src` and keep the tree. Distinct binding strings are few (one per
/// declared field in the live scenes) and never change, so the parse cost is
/// paid once per process and a frame only walks the tree. A failure is cached
/// too: a broken binding must not re-parse 60 times a second.
pub fn compile(src: &str) -> Result<Arc<Expr>, SyntaxError> {
    let mut c = cache().lock().unwrap_or_else(|e| e.into_inner());
    if let Some(hit) = c.get(src) {
        return hit.clone();
    }
    let built = parse(src).map(Arc::new);
    c.insert(src.to_string(), built.clone());
    built
}

// ── evaluation ────────────────────────────────────────────────────────────────

impl Expr {
    /// Walk the tree against one frame's values. A name that resolves to
    /// nothing is 0 — a binding is never allowed to fail a frame, and
    /// `Val::num` falls back to the field's literal on top of that.
    pub fn eval(&self, vals: &SceneValues) -> f32 {
        match self {
            Expr::Num(v) => *v,
            Expr::Name(n) => vals.scalar(n).unwrap_or(0.0),
            Expr::Neg(a) => -a.eval(vals),
            Expr::Not(a) => num(a.eval(vals) == 0.0),
            Expr::Arith(op, a, b) => {
                let (a, b) = (a.eval(vals), b.eval(vals));
                match *op {
                    "+" => a + b,
                    "-" => a - b,
                    "*" => a * b,
                    // a 0 divisor yields 0, not an infinity that poisons every
                    // rect downstream
                    "/" => {
                        if b == 0.0 {
                            0.0
                        } else {
                            a / b
                        }
                    }
                    _ => 0.0,
                }
            }
            Expr::Cmp(op, a, b) => {
                let (a, b) = (a.eval(vals), b.eval(vals));
                num(match *op {
                    "<" => a < b,
                    "<=" => a <= b,
                    ">" => a > b,
                    ">=" => a >= b,
                    "==" => a == b,
                    "!=" => a != b,
                    _ => false,
                })
            }
            Expr::And(a, b) => num(a.eval(vals) != 0.0 && b.eval(vals) != 0.0),
            Expr::Or(a, b) => num(a.eval(vals) != 0.0 || b.eval(vals) != 0.0),
            Expr::Ternary(c, a, b) => {
                if c.eval(vals) != 0.0 {
                    a.eval(vals)
                } else {
                    b.eval(vals)
                }
            }
            Expr::Call(f, args) => {
                let mut a = args.iter().map(|e| e.eval(vals));
                match *f {
                    "min" => a.next().unwrap_or(0.0).min(a.next().unwrap_or(0.0)),
                    "max" => a.next().unwrap_or(0.0).max(a.next().unwrap_or(0.0)),
                    "clamp" => {
                        let v = a.next().unwrap_or(0.0);
                        let lo = a.next().unwrap_or(0.0);
                        let hi = a.next().unwrap_or(0.0);
                        v.max(lo).min(hi)
                    }
                    "abs" => a.next().unwrap_or(0.0).abs(),
                    "round" => a.next().unwrap_or(0.0).round(),
                    "floor" => a.next().unwrap_or(0.0).floor(),
                    "ceil" => a.next().unwrap_or(0.0).ceil(),
                    // `len` reads a STRING property — the one place a binding
                    // sees text, so a scene can branch on label length (the
                    // workspace chip's region is wider for a 2-digit number)
                    "len" => match args.first() {
                        Some(Expr::Name(n)) => vals
                            .scalar(n)
                            .map(|v| v.trunc().abs())
                            .or_else(|| vals.text(n).map(|s| s.chars().count() as f32))
                            .unwrap_or(0.0),
                        Some(other) => other.eval(vals).trunc().abs(),
                        None => 0.0,
                    },
                    _ => 0.0,
                }
            }
        }
    }

    /// Every property name this expression reads, so the validator can ask
    /// "does anyone publish this?" without a second interpreter.
    pub fn deps(&self, out: &mut Vec<String>) {
        match self {
            Expr::Num(_) => {}
            Expr::Name(n) => out.push(n.clone()),
            Expr::Neg(a) | Expr::Not(a) => a.deps(out),
            Expr::Arith(_, a, b) | Expr::Cmp(_, a, b) | Expr::And(a, b) | Expr::Or(a, b) => {
                a.deps(out);
                b.deps(out);
            }
            Expr::Ternary(a, b, c) => {
                a.deps(out);
                b.deps(out);
                c.deps(out);
            }
            Expr::Call(_, args) => args.iter().for_each(|a| a.deps(out)),
        }
    }
}

fn num(b: bool) -> f32 {
    if b {
        1.0
    } else {
        0.0
    }
}

#[cfg(test)]
mod fold_tests {
    use super::fold;

    /// a scope of `item.w = 44`, `index = 2`
    fn scope(name: &str) -> Option<f32> {
        match name {
            "item.w" => Some(44.0),
            "index" => Some(2.0),
            _ => None,
        }
    }

    #[test]
    fn a_scope_name_becomes_a_literal_and_the_rest_of_the_binding_survives() {
        assert_eq!(fold("item.w - 2", scope).as_deref(), Some("44.0 - 2"));
        assert_eq!(
            fold("index * 10 + 5", scope).as_deref(),
            Some("2.0 * 10 + 5")
        );
        assert_eq!(
            fold("max(item.w, 8)", scope).as_deref(),
            Some("max(44.0, 8)")
        );
    }

    #[test]
    fn a_binding_the_scope_does_not_know_is_left_exactly_as_it_was() {
        // no allocation, no rewrite — and the pool still answers it
        assert_eq!(fold("pill_gap - 2", scope), None);
        assert_eq!(
            fold("item.h - pill_gap", scope).as_deref(),
            Some("item.h - pill_gap")
        );
        assert_eq!(fold("44.0", scope), None);
        assert_eq!(fold("", scope), None);
    }

    #[test]
    fn a_folded_binding_still_parses_to_the_same_number() {
        // the point of the fold: a delegate's arithmetic is decided ONCE, at
        // instantiation, from the row — the folded text must still evaluate
        for (src, want) in [
            ("item.w - 2", 42.0),
            ("index * 10", 20.0),
            ("max(item.w, 8)", 44.0),
            ("item.w / 2 + index", 24.0),
        ] {
            let folded = fold(src, scope).expect("scope reads this binding");
            assert_eq!(
                super::parse(&folded)
                    .expect("folded source parses")
                    .eval(&Default::default()),
                want,
                "{src} -> {folded}"
            );
        }
    }

    #[test]
    fn a_non_finite_row_field_is_left_alone_rather_than_written_as_nan() {
        // `NaN` / `inf` are not tokens in this grammar, so folding them would
        // turn a readable "unresolved" into a parse error
        assert_eq!(fold("item.w", |_| Some(f32::NAN)), None);
        assert_eq!(fold("item.w", |_| Some(f32::INFINITY)), None);
    }

    #[test]
    fn a_dotted_scope_name_parses_as_one_name() {
        // The pair the loader's diagnostics depend on: `fold` leaves a name the
        // row does not carry in place, and the PARSER has to read it as the same
        // single name. When it read `item` and stopped at the dot, every
        // delegate binding whose row lacked the field was reported as a syntax
        // error instead of as the missing field it actually was.
        let deps = |src: &str| {
            let mut d = Vec::new();
            super::parse(src)
                .unwrap_or_else(|e| panic!("{src} must parse: {e}"))
                .deps(&mut d);
            d
        };
        assert_eq!(deps("item.w"), vec!["item.w".to_string()]);
        assert_eq!(
            deps("item.w + pill_gap"),
            vec!["item.w".to_string(), "pill_gap".to_string()]
        );
        assert_eq!(
            deps("max(item.w, index)"),
            vec!["item.w".to_string(), "index".to_string()]
        );
        // a dot with no field after it is still a syntax error, not a name
        assert!(super::parse("item.").is_err());
        // and a folded name is a number, so a decimal point is unaffected
        assert_eq!(deps("1.5 + 2"), Vec::<String>::new());
    }

    #[test]
    fn underscores_and_an_identifier_starting_with_one_are_names_not_numbers() {
        // the lexer reads `31_970` as a NUMBER (RON's digit separator) but a
        // binding cannot start one, so `_w` is a name and must survive
        assert_eq!(
            fold("_w + 1", |n| (n == "_w").then_some(3.0)).as_deref(),
            Some("3.0 + 1")
        );
        assert_eq!(
            fold("item_w_2", |n| (n == "item_w_2").then_some(9.0)).as_deref(),
            Some("9.0")
        );
    }
}
