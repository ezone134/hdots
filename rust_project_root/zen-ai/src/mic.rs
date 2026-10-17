// mic — record from the default capture device and turn it into text.
//
// NO cpal (its alsa-sys needs alsa-lib-devel; this box ships without it).
// Capture is a recorder-hook instead: pw-record (PipeWire, present on this
// shell) > arecord. Same job, system tools zen-shell already relies on.
//
// STT tool, picked once (stt_tool()): whisper-cli (whisper.cpp installed)
// > whisper > vosk-cli. None → listen returns a clear error instead of
// pretending. 5 s max of 16 kHz mono, written to /tmp/zen-ai-record.wav.

use std::path::PathBuf;
use std::process::{Command, Stdio};

pub fn stt_tool() -> String {
    for t in ["whisper-cli", "whisper", "vosk-cli"] {
        if which::which(t).is_ok() { return t.to_string(); }
    }
    "none".to_string()
}

pub fn recorder_tool() -> String {
    for t in ["pw-record", "parecord", "arecord"] {
        if which::which(t).is_ok() { return t.to_string(); }
    }
    "none".to_string()
}

pub fn capture_to_text(hotword: bool) -> Result<String, String> {
    let _ = hotword; // hotword gate is a TODO: records immediately for now
    let wav = capture_wav()?;
    transcribe(&wav)
}

fn capture_wav() -> Result<PathBuf, String> {
    let path = PathBuf::from("/tmp/zen-ai-record.wav");
    let secs = "5"; // whisper wants files anyway; short is polite
    let rec = recorder_tool();
    let st = match rec.as_str() {
        "none" => return Err("no recorder (pw-record/parecord/arecord found)".into()),
        "pw-record" => Command::new("pw-record").args(["--rate", "16000", "--channels", "1",
            "--format", "s16", "-d", secs]).arg(&path).status(),
        "parecord" => Command::new("parecord").args(["--rate=16000", "--channels=1",
            "--format=s16le", "--raw=false"]).arg(&path).status(),
        "arecord" => Command::new("arecord").args(["-r", "16000", "-c", "1", "-f", "S16_LE",
            "-d", secs]).arg(&path).status(),
        _ => unreachable!(),
    }.map_err(|e| e.to_string())?;
    if !st.success() { return Err(format!("{} exited {}", rec, st)); }
    if !path.is_file() { return Err("recorder wrote nothing".into()); }
    Ok(path)
}

fn transcribe(wav: &PathBuf) -> Result<String, String> {
    let tool = stt_tool();
    match tool.as_str() {
        "none" => Err(format!(
            "no STT tool; install whisper-cli (whisper.cpp) — model dir: $rconf/models")),
        t => {
            let model = whisper_model().ok_or("no whisper .bin model in $rconf/models")?;
            let out = Command::new(t).args(["-m"]).arg(&model).arg("-f").arg(wav)
                .args(["-nt"]).stdout(Stdio::piped()).output().map_err(|e| e.to_string())?;
            if !out.status.success() { return Err(format!("{} failed: {}", t, out.status)); }
            Ok(String::from_utf8_lossy(&out.stdout).to_string())
        }
    }
}

fn whisper_model() -> Option<PathBuf> {
    let dirs: Vec<PathBuf> = std::env::var("rconf")
        .map(|r| vec![PathBuf::from(r).join("models")])
        .unwrap_or_default();
    for d in dirs {
        for e in std::fs::read_dir(&d).ok()?.flatten() {
            let p = e.path();
            if p.extension().and_then(|x| x.to_str()) == Some("bin") { return Some(p); }
        }
    }
    None
}
