// zen-ai - offline/online voice AI assistant for the zen shell.
// Runs llama.cpp in-process (a .gguf in $states2/../models), falls back to an
// OpenAI-compatible endpoint when asked, and turns every answer it can map
// into `master <section> <args…>` - the same single entry point that fronts
// every shell CLI in sources/bin.
//
// usage:
//   zen-ai ask "turn off blur"        text -> intents -> master run
//   zen-ai say  "what's the theme"    text -> answer only (no side effects)
//   zen-ai listen                     hotword -> mic -> ask
//   zen-ai mic                        mic -> ask directly (no hotword)
//   zen-ai repl                       streaming chat session
//   zen-ai status                     engine + model + master status line
//   zen-ai intent <text>              print parsed intents, run nothing
//
// env (all optional):
//   ZEN_AI_MODEL      path to .gguf                (default: search $rconf/models)
//   ZEN_AI_ONLINE     base url of an OpenAI-compatible /v1/chat/completions
//   ZEN_AI_KEY        bearer key for that endpoint
//   ZEN_AI_SOURCES    override $sources discovery

mod engine;
mod intents;
mod master;
mod mic;
mod speak;

use std::io::Write as _;

fn eprint_usage() {
    eprintln!("zen-ai: use ask <text>|say <text>|listen|mic|repl|status|intent <text>");
}

fn main() {
    let arg0 = std::env::args().nth(1);
    let rest: Vec<String> = std::env::args().skip(2).collect();

    let code = match arg0.as_deref() {
        Some("ask") => cmd_ask(&rest, true),
        Some("say") => cmd_ask(&rest, false),
        Some("intent") => cmd_intent(&rest),
        Some("listen") => cmd_listen(true),
        Some("mic") => cmd_listen(false),
        Some("repl") => cmd_repl(),
        Some("status") => cmd_status(),
        _ => {
            eprint_usage();
            2
        }
    };
    std::process::exit(code);
}

// the engine line is shared by ask/say/listen/status
fn engine_blurb() -> String {
    match engine::probe() {
        engine::Engine::Local(m) => format!("local: {}", m.display()),
        engine::Engine::Online(url) => format!("online: {}", url),
        engine::Engine::None => "none (no model file, no endpoint)".into(),
    }
}

fn cmd_ask(rest: &[String], run: bool) -> i32 {
    if rest.is_empty() {
        eprint_usage();
        return 2;
    }
    let text = rest.join(" ");
    let reply = match engine::complete(&text) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("zen-ai: engine: {}", e);
            return 1;
        }
    };
    println!("{}", reply);

    let parsed = intents::parse(&reply);
    if parsed.is_empty() {
        if run { eprintln!("zen-ai: nothing to run"); }
        speak::say_reminder(&reply);
        return 0;
    }
    for intent in &parsed {
        println!("-> {} {}", intent.section, intent.args.join(" "));
        if run {
            if let Err(e) = master::run(&intent.section, &intent.args) {
                eprintln!("zen-ai: master {} failed: {}", intent.section, e);
            }
        }
    }
    speak::say_done(&parsed);
    0
}

fn cmd_intent(rest: &[String]) -> i32 {
    if rest.is_empty() { eprint_usage(); return 2; }
    let text = rest.join(" ");
    for intent in intents::parse(&text) {
        println!("{} {}", intent.section, intent.args.join(" "));
    }
    0
}

fn cmd_listen(hotword: bool) -> i32 {
    let text = match mic::capture_to_text(hotword) {
        Ok(t) if !t.trim().is_empty() => t,
        Ok(_) => { eprintln!("zen-ai: heard nothing"); return 1; }
        Err(e) => { eprintln!("zen-ai: mic: {}", e); return 1; }
    };
    println!("heard: {}", text);
    cmd_ask(&[text], true)
}

fn cmd_repl() -> i32 {
    // straight streaming loop: one line in, reply + intents out, no history
    // state - the shell pipeline is stateless by design, so is the chat.
    let mut line = String::new();
    loop {
        print!("zen-ai> ");
        let _ = std::io::stdout().flush();
        line.clear();
        if std::io::stdin().read_line(&mut line).unwrap_or(0) == 0 { return 0; }
        let t = line.trim();
        if t.is_empty() { continue; }
        if t == "quit" || t == "exit" { return 0; }
        let v: Vec<String> = vec![t.to_string()];
        cmd_ask(&v, true);
    }
}

fn cmd_status() -> i32 {
    println!("engine  {}", engine_blurb());
    println!("master  {}", master::path().display());
    println!("stt     {}", mic::stt_tool());
    println!("tts     {}", speak::tts_tool());
    0
}
