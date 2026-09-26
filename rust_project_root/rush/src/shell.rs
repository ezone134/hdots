//! Shell state: cwd, variables, functions, background job handles.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::process::Child;

#[derive(Default)]
pub struct Shell {
    pub cwd: PathBuf,
    /// shell + env variables. Only keys in `exported` are inherited by children.
    pub vars: HashMap<String, String>,
    /// array variables: name -> elements
    pub arrays: HashMap<String, Vec<String>>,
    pub exported: HashSet<String>,
    /// function name -> raw body source (list of statements)
    pub functions: HashMap<String, Vec<String>>,
    pub last_status: i32,
    /// positional params $1..$9, ${@}
    pub positional: Vec<String>,
    /// backgrounded child processes
    pub jobs: Vec<Child>,
    pub interactive: bool,
}

impl Shell {
    pub fn new(interactive: bool) -> Self {
        let mut shell = Shell {
            cwd: std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/")),
            vars: HashMap::new(),
            arrays: HashMap::new(),
            exported: HashSet::new(),
            functions: HashMap::new(),
            last_status: 0,
            positional: Vec::new(),
            jobs: Vec::new(),
            interactive,
        };
        // seed from process env
        for (k, v) in std::env::vars() {
            shell.vars.insert(k.clone(), v);
            shell.exported.insert(k);
        }
        shell.set_var("?");
        shell
    }

    pub fn get(&self, k: &str) -> Option<&String> {
        self.vars.get(k)
    }

    pub fn set(&mut self, k: &str, v: String) {
        self.vars.insert(k.to_string(), v);
    }

    pub fn get_array(&self, k: &str) -> Option<&Vec<String>> {
        self.arrays.get(k)
    }

    pub fn set_array(&mut self, k: &str, v: Vec<String>) {
        self.arrays.insert(k.to_string(), v);
    }

    pub fn export(&mut self, k: &str) {
        self.exported.insert(k.to_string());
    }

    /// Full env map merged from exported vars + positional/special.
    pub fn env(&self) -> HashMap<String, String> {
        let mut m = HashMap::new();
        for k in &self.exported {
            if let Some(v) = self.vars.get(k) {
                m.insert(k.clone(), v.clone());
            }
        }
        m.insert("PWD".into(), self.cwd.to_string_lossy().into_owned());
        m
    }

    pub fn set_var(&mut self, _k: &str) {
        // placeholder for special-var interning
    }
}
