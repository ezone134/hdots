// engine — where answers come from.
//
// Order of preference, in one probe():
//   1. local .gguf via embedded llama.cpp  (fully offline; the default path)
//   2. an OpenAI-compatible HTTP endpoint  (user opted in via ZEN_AI_ONLINE)
//   3. nothing — callers then answer from the intent table alone
//
// Model search is one directory deep in $rconf/models (falling back to
// XDG data + ./models, then $sources/models), pick: smallest that still
// matches names we know — on this 7GB-RAM box a 7B Q4 fits, a 13B does not.
//
// The embedded llama.cpp stays inside one request context per call: create,
// decode the prompt, read the answer, drop. First call pays the model load;
// a REPL pays it once per process, everything else once per invocation.

use std::path::{Path, PathBuf};

pub const SYSTEM_PROMPT: &str = r#"You are the zen shell assistant. You answer in ONE short line.
If the user asks to change the shell or desktop, you answer with a master command
like: master <section> <args...>. Sections and verbs:
  theme status|dark|light|toggle|restore
  blur|opacity|shadow toggle|on|off|restore|reset
  scr toggle|on|off|set N|temp K|gamma-up|gamma-down
  bri get|set PCT|up|down
  audio vol set N|up|down|mute, mic ...
  caffeine on|off|toggle, pp cycle, net status
  monitors get|set, idle set NAME VALUE, wallpaper PATH|NAME
  power status, media play|pause|next|prev, shot full|region, ws goto N
If it is not a shell task, just answer briefly. No explanations, no markdown."#;

pub enum Engine {
    Local(PathBuf),
    Online(String),
    None,
}

// ── model discovery ──────────────────────────────────────────────────────────

fn gguf_search_dirs() -> Vec<PathBuf> {
    let mut v = Vec::new();
    if let Ok(rconf) = std::env::var("rconf") { v.push(PathBuf::from(rconf).join("models")); }
    if let Ok(home) = std::env::var("HOME") {
        v.push(PathBuf::from(&home).join(".local/share/zen-ai/models"));
        v.push(PathBuf::from(&home).join(".cache/zen-ai/models"));
    }
    if let Ok(sources) = std::env::var("sources") { v.push(PathBuf::from(sources).join("models")); }
    v.push(PathBuf::from("models"));
    v
}

pub fn find_model() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("ZEN_AI_MODEL") {
        let p = PathBuf::from(p);
        if p.is_file() { return Some(p); }
    }
    let mut hits: Vec<PathBuf> = Vec::new();
    for dir in gguf_search_dirs() {
        let Ok(rd) = std::fs::read_dir(&dir) else { continue };
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().and_then(|x| x.to_str()) == Some("gguf") {
                hits.push(p);
            }
        }
    }
    // smallest first (RAM-neutral)
    hits.sort_by_key(|p| p.metadata().and_then(|m| Ok(m.len())).unwrap_or(Ok(u64::MAX)));
    hits.into_iter().next()
}

pub fn probe() -> Engine {
    if let Some(m) = find_model() { return Engine::Local(m); }
    if let Ok(url) = std::env::var("ZEN_AI_ONLINE") {
        if !url.trim().is_empty() { return Engine::Online(url.trim().to_string()); }
    }
    Engine::None
}

// ── local inference (llama.cpp embedded) ─────────────────────────────────────

pub fn complete(prompt: &str) -> Result<String, String> {
    match probe() {
        Engine::Local(model) => local_complete(&model, prompt),
        Engine::Online(url) => online_complete(&url, prompt),
        Engine::None => Ok(no_engine_answer(prompt)),
    }
}

fn local_complete(model: &Path, prompt: &str) -> Result<String, String> {
    let ctx_params = llama_cpp_2::context::ContextParameters {
        n_ctx: 1024,
        n_threads: std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4),
        ..Default::default()
    };
    let backend = llama_cpp_2::llama_backend::LlamaBackend::init()
        .map_err(|e| format!("backend init: {e}"))?;
    let mparams = llama_cpp_2::model::LlamaModelParams::default();
    let model_c = llama_cpp_2::model::LlamaModel::load_from_file(&backend, model, mparams)
        .map_err(|e| format!("load {}: {e}", model.display()))?;
    let mut ctx = model_c.new_context(&backend, ctx_params)
        .map_err(|e| format!("ctx: {e}"))?;

    let full = format!("<|im_start|>system\n{SYSTEM_PROMPT}<|im_end|>\n<|im_start|>user\n{prompt}<|im_end|>\n<|im_start|>assistant\n");
    let tokens = model_c.str_to_token(&full, llama_cpp_2::model::add_bos::AddBos::Always)
        .map_err(|e| format!("tokenize: {e}"))?;

    let mut batch = llama_cpp_2::llama_batch::LlamaBatch::new(2048, 1);
    let n = tokens.len();
    for (i, t) in tokens.iter().enumerate() {
        batch.add(*t, i as i32, &[0], if i + 1 == n { true } else { false });
    }
    ctx.decode(&mut batch).map_err(|e| format!("decode: {e}"))?;

    let mut out = String::new();
    let max_new = 96usize;
    for i in 0..max_new {
        let tok = ctx.sample_token()
            .map_err(|e| format!("sample: {e}"))?;
        if ctx.is_eog_token(tok) { break; }
        out.push_str(&ctx.token_to_str(tok, llama_cpp_2::model::token::Special::Skip)
            .unwrap_or_default());
        let pos = (n + i) as i32;
        batch.clear();
        batch.add(tok, pos, &[0], true);
        ctx.decode(&mut batch).map_err(|e| format!("gen decode: {e}"))?;
    }
    Ok(out.trim().to_string())
}

// ── online fallback (OpenAI-compatible chat completions, crate-free) ──────────

fn online_complete(base: &str, prompt: &str) -> Result<String, String> {
    let url = if base.ends_with("/v1") { format!("{base}/chat/completions") } else { format!("{base}/v1/chat/completions") };
    let body = serde_json::json!({
        "messages": [
            {"role": "system", "content": SYSTEM_PROMPT},
            {"role": "user", "content": prompt}
        ],
        "max_tokens": 128, "temperature": 0.2
    });
    let key = std::env::var("ZEN_AI_KEY").unwrap_or_default();
    let mut cmd = std::process::Command::new("curl");
    cmd.args(["-sS", "-m", "30", "-X", "POST", &url, "-H", "content-type: application/json"]);
    if !key.is_empty() { cmd.args(["-H", &format!("authorization: Bearer {key}")]); }
    cmd.arg("-d").arg(serde_json::to_string(&body).unwrap());
    let out = cmd.output().map_err(|e| format!("curl spawn: {e}"))?;
    if !out.status.success() { return Err(format!("http {} from {url}", out.status)); }
    let v: serde_json::Value = serde_json::from_slice(&out.stdout)
        .map_err(|e| format!("bad json from endpoint: {e}"))?;
    let a = v["choices"][0]["message"]["content"]
        .as_str().ok_or_else(|| "no content in response".to_string())?;
    Ok(a.trim().to_string())
}

// No engine yet (no gguf, no endpoint): fall through to the keyword intent
// table so `master blur toggle` still runs — engine presence is about ANSWERS,
// not about whether the shell keeps working.
fn no_engine_answer(prompt: &str) -> String {
    prompt.to_string()
}
