// intents — map a plain sentence (or an LLM reply that ends in a master
// command) to one or more master invocations. Two layers:
//
//   1. scout the text for an explicit `master …` line (the LLM was told to
//      emit those, so a well-behaved reply lands here directly)
//   2. a keyword table for the no-engine case and for free-text answers.
//
// Both produce the same Intent { section, args } — the only thing main.rs
// ever runs. Sections are the exact names master_body's SECTIONS table maps;

pub struct Intent {
    pub section: String,
    pub args: Vec<String>,
}

// scan every line for a `master …` prefix; first hit wins (the model was told
// ONE line), but if it lists several, all run in order.
fn scout(text: &str) -> Vec<Intent> {
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        let Some(rest) = line.strip_prefix("master ") else { continue };
        let rest = rest.strip_prefix("-") .unwrap_or(rest); // tolerate "master -"
        let mut words = rest.split_whitespace();
        let Some(section) = words.next() else { continue };
        let args: Vec<String> = words.map(|w| w.trim_matches('"').to_string()).collect();
        out.push(Intent { section: section.to_string(), args });
        if out.len() >= 4 { break; } // cap runaway lists
    }
    out
}

// The no-model everyday table: phrase fragments → master verbs. Ordered,
// first match wins; each entry is (needle, section, args).
const KEYWORDS: &[(&str, &str, &[&str])] = &[
    ("turn off blur",       "blur",     &["off"]),
    ("disable blur",        "blur",     &["off"]),
    ("blur off",            "blur",     &["off"]),
    ("turn on blur",        "blur",     &["on"]),
    ("blur on",             "blur",     &["on"]),
    ("toggle blur",         "blur",     &["toggle"]),
    ("toggle opacity",      "opacity",  &["toggle"]),
    ("turn off opacity",    "opacity",  &["off"]),
    ("turn off shadows",    "shadow",   &["off"]),
    ("turn on shadows",     "shadow",   &["on"]),
    ("dark mode",           "theme",    &["dark"]),
    ("light mode",          "theme",    &["light"]),
    ("toggle theme",        "theme",    &["toggle"]),
    ("screenshot",          "shot",     &["full"]),
    ("region screenshot",   "shot",     &["region"]),
    ("wifi status",         "net",      &["wifi", "status"]),
    ("battery status",      "power",    &["status"]),
    ("power profile",       "pp",       &["cycle"]),
    ("caffeine on",         "caffeine", &["on"]),
    ("caffeine off",        "caffeine", &["off"]),
    ("reload shell",        "restart",  &[]),
 ];

pub fn parse(text: &str) -> Vec<Intent> {
    let scoped = scout(text);
    if !scoped.is_empty() { return scoped; }

    let low = text.to_lowercase();
    for (needle, section, args) in KEYWORDS {
        if low.contains(needle) {
            return vec![Intent {
                section: (*section).to_string(),
                args: args.iter().map(|s| s.to_string()).collect(),
            }];
        }
    }
    Vec::new() // a plain question → answer only, nothing to run
}
