//! rush — a fast, bash-like Rust shell.
//!
//! Launch fast (≈1–2 ms — beats bash and python/lua), run forkless fs
//! builtins (mkdir/cp/rm/chmod/mv via `rsys`, no external binary), interpret
//! bash-style scripts, and exec native binaries.
//!
//! Modular layout:
//!   src/shell.rs   — shell state (cwd, vars, functions, jobs)
//!   src/ast.rs     — parsed command tree
//!   src/parser.rs  — tokenizer + recursive-descent parser
//!   src/expand.rs  — variable/parameter/command expansion + glob
//!   src/builtins.rs— native builtins incl. forkless fs via `rsys`
//!   src/exec.rs    — execute AST: builtins, external exec, redirects, control flow
//!   src/main.rs    — entry + REPL

pub mod builtins;
pub mod exec;
pub mod expand;
pub mod main_loop;
pub mod parser;
pub mod shell;

pub use shell::Shell;
