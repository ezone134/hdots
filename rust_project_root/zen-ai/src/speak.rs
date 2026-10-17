// speak — voice out. Tool, picked once: spd-say (speech-dispatcher, the
// PipeWire-friendly path) > espeak-ng > espeak. No tool → silent; voice is
// an enhancement, never a requirement, so every path here is best-effort.

use std::process::{Command, Stdio};

pub fn tts_tool() -> String {
    for t in ["spd-say", "espeak-ng", "espeak"] {
        if which::which(t).is_ok() { return t.to_string(); }
    }
    "none".to_string()
}

pub fn say(text: &str) {
    match tts_tool().as_str() {
        "none" => {} // silently skip
        t => { let _ = Command::new(t).arg("-w").arg(text).stdout(Stdio::null()).stderr(Stdio::null()).status(); }
    }
}

// short status lines only; the user hears results, not the master command
pub fn say_done(intents: &[crate::intents::Intent]) {
    if let Some(i) = intents.first() {
        let what = match i.section.as_str() {
            "blur" | "opacity" | "shadow" | "scr" => "screen look",
            "theme" => "theme",
            "bri" => "brightness",
            "audio" => "volume",
            "caffeine" => "caffeine",
            "power" | "pp" => "power",
            "shot" => "screenshot",
            _ => i.section.as_str(),
        };
        say(&format!("Done. {}", what));
    }
}

pub fn say_reminder(_reply: &str) { /* no engine: stay quiet rather than read raw text */ }
