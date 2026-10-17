// master — the one bridge to the shell. Every action zen-ai takes goes
// through $sources/bin/master (the table dispatcher that fronts every
// *_main CLI), so the assistant can only ever do what master already gates.
//
// $sources discovery, in order: $ZEN_AI_SOURCES, $sources (exported by
// hypr/env.lua), then the sibling-path this repo actually lives in.

use std::path::PathBuf;
use std::process::Command;

pub fn sources_dir() -> PathBuf {
    if let Ok(p) = std::env::var("ZEN_AI_SOURCES") {
        if PathBuf::from(&p).join("bin/master").is_file() { return PathBuf::from(p); }
    }
    if let Ok(p) = std::env::var("sources") {
        if PathBuf::from(&p).join("bin/master").is_file() { return PathBuf::from(p); }
    }
    // repo-relative fallback (zen-ai sits in rust_project_root/ next to sources)
    let home = std::env::var("HOME").unwrap_or_default();
    PathBuf::from(home).join(".config/hdots/sources")
}

pub fn path() -> PathBuf {
    sources_dir().join("bin/master")
}

pub fn run(section: &str, args: &[String]) -> Result<(), String> {
    let m = path();
    if !m.is_file() {
        return Err(format!("master not found at {}", m.display()));
    }
    let mut cmd = Command::new(&m);
    cmd.arg(section).args(args);
    // master's CLIs need the runtime env ($states/$states2/$rconf/...); when
    // zen-ai runs outside the Hyprland session env, re-derive the defaults
    // env.lua uses: rbase=/tmp, tree /tmp/<user>_rconf.
    if std::env::var("states").is_err() {
        let user = std::env::var("USER").unwrap_or_else(|_| "tw".into());
        let rconf = format!("/tmp/{}_rconf", user);
        for (k, v) in [
            ("rbase", "/tmp"),
            ("rconf", rconf.as_str()),
            ("rconfig", &format!("{}/rconfig", rconf)),
            ("states", &format!("{}/states", rconf)),
            ("states2", &format!("{}/states2", rconf)),
            ("garbage", &format!("{}/garbage", rconf)),
            ("r_daemon", &format!("{}/r_daemon", rconf)),
            ("rcache", &format!("{}/rcache", rconf)),
            ("sources", sources_dir().to_string_lossy().as_ref()),
            ("dots", std::env::var("HOME").unwrap_or_default().as_str()),
        ] {
            std::env::set_var(k, v);
        }
    }
    let st = cmd.status().map_err(|e| format!("spawn {}: {e}", m.display()))?;
    if st.success() { Ok(()) } else { Err(format!("master {} exited {}", section, st)) }
}
