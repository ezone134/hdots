//! Lyrics for the currently playing track.
//!
//! Sources, in order:
//!   - a local cache at `~/.cache/zen-shell/lyrics/<artist>__<title>.lrc`
//!     (written on every successful fetch, so repeat plays are offline-safe),
//!   - the LRCLib API (`https://lrclib.net/api/get?artist_name=…&track_name=…`)
//!     — no key, generous terms, covers virtually all mainstream tracks,
//!   - LRCLib fuzzy search (`/api/search?q=…`) when the exact title 404s —
//!     picks the closest artist/track match and uses its synced lyrics,
//!   - lyrics.ovh (`https://api.lyrics.ovh/v1/…`) — a plain-text fallback
//!     last resort when neither LRCLib route finds anything.
//!
//! `fetch_lyrics` blocks (curl, 8 s timeout per attempt) and is therefore
//! called from a worker thread in the app; the result is pumped back over a
//! channel like the News card. Parsing is lightweight string-scanning — the
//! crate has no XML parser and LRC is trivial to scan by hand.

use std::path::PathBuf;

/// One lyric line. `time` is the track position in seconds (>= 0 when the
/// source carries LRC timestamps, -1.0 for plain / unsynced text).
#[derive(Clone, Debug, PartialEq)]
pub struct LyricLine {
    pub time: f64,
    pub text: String,
}

/// Message pumped back to the shell from the fetch worker thread.
pub enum LyricsMsg {
    /// `(artist, title)` the result belongs to + the lines (empty = none).
    Result(String, String, Vec<LyricLine>),
}

/// True when the lines carry LRC timestamps (synced highlighting is possible).
pub fn has_timing(lines: &[LyricLine]) -> bool {
    lines.iter().any(|l| l.time >= 0.0)
}

/// Index of the word the playhead is on inside `line` (0-based), estimated
/// by splitting the line's time window (start of this line up to the next
/// timed line) evenly across its words. Words are weighted by character count
/// so longer words (e.g. "Despacito") get proportionally more time.
/// `None` for unsynced lines.
pub fn active_word(lines: &[LyricLine], pos: f64, line: usize) -> Option<usize> {
    let l = lines.get(line)?;
    if l.time < 0.0 {
        return None;
    }
    let t0 = l.time;
    let t1 = lines[line + 1..]
        .iter()
        .find(|x| x.time >= 0.0)
        .map(|x| x.time)
        .unwrap_or(t0 + 8.0);
    let words: Vec<&str> = l.text.split_whitespace().collect();
    if words.is_empty() || t1 <= t0 {
        return Some(0);
    }
    let weights: Vec<f64> = words.iter().map(|w| w.chars().count().max(1) as f64).collect();
    let total: f64 = weights.iter().sum();
    let progress = ((pos - t0) / (t1 - t0)).clamp(0.0, 1.0);
    let mut acc = 0.0;
    for (i, &w) in weights.iter().enumerate() {
        acc += w / total;
        if progress <= acc {
            return Some(i);
        }
    }
    Some(words.len() - 1)
}

/// Blocking fetch for `(artist, title)`. Returns the lines, or an empty vec
/// when nothing was found (or the network is unavailable). Tries the exact
/// LRCLib lookup first, then LRCLib fuzzy search, then lyrics.ovh.
pub fn fetch_lyrics(artist: &str, title: &str) -> Vec<LyricLine> {
    if title.trim().is_empty() {
        return Vec::new();
    }
    if let Some(cached) = read_cache(artist, title) {
        if !cached.is_empty() {
            return cached;
        }
    }
    // Try with original names first
    let mut lines = fetch_exact(artist, title);
    if lines.is_empty() {
        lines = fetch_search(artist, title);
    }
    // If exact failed, try stripped versions (ft./feat./remix/etc.)
    if lines.is_empty() {
        let stripped_artist = strip_artist_suffix(artist);
        let stripped_title = strip_title_suffix(title);
        if stripped_artist != artist || stripped_title != title {
            lines = fetch_exact(&stripped_artist, &stripped_title);
            if lines.is_empty() {
                lines = fetch_search(&stripped_artist, &stripped_title);
            }
        }
    }
    if lines.is_empty() {
        lines = fetch_ovh(artist, title);
    }
    if lines.is_empty() {
        lines = fetch_duckduckgo(artist, title);
    }
    if !lines.is_empty() {
        write_cache(artist, title, &lines);
    }
    lines
}

/// Strip common artist suffixes like "feat. X", "ft. X", "vs. X", "x X",
/// "with X" etc. to improve exact match success.
fn strip_artist_suffix(artist: &str) -> String {
    let lower = artist.to_lowercase();
    // Try each separator in order of specificity
    for sep in &[" feat. ", " feat ", " ft. ", " ft ", " x ", " vs. ", " vs ", " with ", " & "] {
        if let Some(pos) = lower.find(sep) {
            let trimmed = artist[..pos].trim();
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }
    }
    artist.to_string()
}

/// Strip common title suffixes like "(Remix)", "[Live]", "- Single", etc.
fn strip_title_suffix(title: &str) -> String {
    let mut s = title.to_string();
    // Remove parenthetical/bracket suffixes
    for open in ['(', '[', '{'] {
        let close = match open { '(' => ')', '[' => ']', '{' => '}', _ => ')' };
        while let Some(start) = s.rfind(open) {
            if let Some(end) = s[start..].find(close) {
                let inner = s[start+1..start+end].trim().to_lowercase();
                // Only strip if it looks like a version tag, not part of the title
                if inner.starts_with("remix") || inner.starts_with("live") || inner.starts_with("acoustic")
                    || inner.starts_with("deluxe") || inner.starts_with("single")
                    || inner.starts_with("explicit") || inner.starts_with("clean")
                    || inner.starts_with("radio edit") || inner.starts_with("album version")
                    || inner.starts_with("from") || inner.starts_with("version") {
                    s = format!("{}{}", &s[..start], &s[start+end+1..]).trim().to_string();
                } else {
                    break;
                }
            } else {
                break;
            }
        }
    }
    // Strip trailing " - Single", " - EP", etc.
    for suffix in [" - Single", " - EP", " - Album", " - Deluxe Edition"] {
        if let Some(pos) = s.rfind(suffix) {
            s = s[..pos].trim().to_string();
        }
    }
    s
}

/// LRCLib exact `artist_name` + `track_name` lookup.
fn fetch_exact(artist: &str, title: &str) -> Vec<LyricLine> {
    let url = format!(
        "https://lrclib.net/api/get?artist_name={}&track_name={}",
        enc(artist.trim()),
        enc(title.trim())
    );
    http_get(&url).map(|b| parse(&b)).unwrap_or_default()
}

/// LRCLib fuzzy search: query `q=artist title`, score the candidates, keep
/// the closest one that actually carries lyrics.
fn fetch_search(artist: &str, title: &str) -> Vec<LyricLine> {
    let q = format!("{} {}", title.trim(), artist.trim());
    let Some(body) = http_get(&format!("https://lrclib.net/api/search?q={}", enc(&q))) else {
        return Vec::new();
    };
    let Ok(v) = serde_json::from_str::<serde_json::Value>(&body) else {
        return Vec::new();
    };
    let Some(arr) = v.as_array() else { return Vec::new() };
    if arr.is_empty() {
        return Vec::new();
    }
    // score = artist-distance mostly, track-distance secondary; prefer the
    // best candidate that has lyrics at all.  Strip artist/title suffixes
    // before scoring so "Luis Fonsi ft. Daddy Yankee" matches "Luis Fonsi".
    let norm_artist = normalize(&strip_artist_suffix(artist));
    let norm_title = normalize(&strip_title_suffix(title));
    let mut best: Option<(f64, &serde_json::Value)> = None;
    for cand in arr {
        let ca = cand.get("artistName").and_then(|x| x.as_str()).unwrap_or("").to_string();
        let ct = cand.get("trackName").and_then(|x| x.as_str()).unwrap_or("").to_string();
        let has_lyrics = cand.get("syncedLyrics").and_then(|x| x.as_str()).map_or(false, |s| !s.trim().is_empty())
            || cand.get("plainLyrics").and_then(|x| x.as_str()).map_or(false, |s| !s.trim().is_empty());
        if !has_lyrics {
            continue;
        }
        let cand_norm_artist = normalize(&strip_artist_suffix(&ca));
        let cand_norm_title = normalize(&strip_title_suffix(&ct));
        let score = lev(&cand_norm_artist, &norm_artist) * 1.2 + lev(&cand_norm_title, &norm_title);
        if best.map_or(true, |(s, _)| score < s) {
            best = Some((score, cand));
        }
    }
    let Some((_, cand)) = best else { return Vec::new() };
    if let Some(s) = cand.get("syncedLyrics").and_then(|x| x.as_str()) {
        let l = parse_lrc(s);
        if !l.is_empty() {
            return l;
        }
    }
    if let Some(p) = cand.get("plainLyrics").and_then(|x| x.as_str()) {
        return parse_plain(p);
    }
    Vec::new()
}

/// lyrics.ovh plain-text fallback (no synced timestamps).
fn fetch_ovh(artist: &str, title: &str) -> Vec<LyricLine> {
    let url = format!(
        "https://api.lyrics.ovh/v1/{}/{}",
        enc(artist.trim()),
        enc(title.trim())
    );
    let Some(body) = http_get(&url) else { return Vec::new() };
    let Ok(v) = serde_json::from_str::<serde_json::Value>(&body) else { return Vec::new() };
    let Some(l) = v.get("lyrics").and_then(|x| x.as_str()) else { return Vec::new() };
    parse_plain(l)
}

/// DuckDuckGo "lite" HTML lyrics search — extracts the first lyrics block
/// from the search results. This is a scraper-of-last-resort; it's fragile
/// but catches tracks the structured APIs miss.
fn fetch_duckduckgo(artist: &str, title: &str) -> Vec<LyricLine> {
    let q = format!("{} {} lyrics", artist.trim(), title.trim());
    let url = format!("https://lite.duckduckgo.com/lite/?q={}", enc(&q));
    let Some(body) = http_get(&url) else { return Vec::new() };
    // The lite HTML has lyrics in plain text between <td> tags.
    // Look for multi-line blocks that look like lyrics (contain common patterns).
    let mut lyrics_lines = Vec::new();
    let mut consecutive_short = 0u32;
    for line in body.lines() {
        let trimmed = line.trim();
        // Skip HTML tags
        if trimmed.starts_with('<') || trimmed.is_empty() {
            if !lyrics_lines.is_empty() {
                consecutive_short += 1;
                if consecutive_short > 3 {
                    break; // end of lyrics block
                }
            }
            continue;
        }
        // Look for lines that look like lyrics (not navigation, not too short)
        if trimmed.len() > 3
            && !trimmed.contains("<!--")
            && !trimmed.contains("function")
            && !trimmed.contains("var ")
            && !trimmed.contains("cookie")
            && !trimmed.contains("privacy")
            && !trimmed.contains("Settings")
            && !trimmed.contains("About")
            && !trimmed.contains("Help")
            && !trimmed.contains("Feedback")
        {
            // Decode basic HTML entities
            let decoded = trimmed
                .replace("&amp;", "&")
                .replace("&lt;", "<")
                .replace("&gt;", ">")
                .replace("&#39;", "'")
                .replace("&quot;", "\"");
            lyrics_lines.push(decoded);
            consecutive_short = 0;
        }
    }
    if lyrics_lines.len() >= 3 {
        parse_plain(&lyrics_lines.join("\n"))
    } else {
        Vec::new()
    }
}

/// `curl -sL` a URL (timeout-bounded) → body, None on any network failure.
fn http_get(url: &str) -> Option<String> {
    let out = std::process::Command::new("curl")
        .args(["-sL", "--max-time", "8", "--connect-timeout", "4", "-A", "zen-shell"])
        .arg(url)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Fold a name down to comparable form: lowercase, drop non-alphanumerics,
/// collapse spaces, strip common "extra" suffixes that break exact matches.
fn normalize(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.to_lowercase().chars() {
        if c.is_alphanumeric() {
            out.push(c);
        } else if !out.is_empty() && !out.ends_with(' ') && !out.ends_with('_') {
            out.push(' ');
        }
    }
    let t = out.trim();
    for suf in ["official music video", "official video", "official audio", "official lyric video", "lyrics", "hq audio", "audio", "remastered"] {
        if t.len() > suf.len() && t[t.len() - suf.len()..].trim() == suf && !t[..t.len() - suf.len()].trim().is_empty() {
            return normalize(t[..t.len() - suf.len()].trim());
        }
    }
    t.replace(' ', "_")
}

/// Classic Levenshtein distance (small strings only — lyric titles/artists).
fn lev(a: &str, b: &str) -> f64 {
    let (n, m) = (a.len(), b.len());
    if n == 0 { return m as f64; }
    if m == 0 { return n as f64; }
    let mut prev: Vec<usize> = (0..=m).collect();
    let mut cur = vec![0usize; m + 1];
    for (i, ca) in a.bytes().enumerate() {
        cur[0] = i + 1;
        for (j, cb) in b.bytes().enumerate() {
            cur[j + 1] = if ca == cb {
                prev[j]
            } else {
                1 + prev[j].min(prev[j + 1]).min(cur[j])
            };
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[m] as f64
}

/// Percent-encode a URL query component (curl-safe subset kept literal).
fn enc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// Parse an LRCLib JSON body first, then fall back to raw LRC/plain text.
fn parse(body: &str) -> Vec<LyricLine> {
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(body) {
        if v.is_object() {
            if let Some(s) = v.get("syncedLyrics").and_then(|x| x.as_str()) {
                let l = parse_lrc(s);
                if !l.is_empty() {
                    return l;
                }
            }
            if let Some(p) = v.get("plainLyrics").and_then(|x| x.as_str()) {
                return parse_plain(p);
            }
        }
        // JSON but not a lyrics object (error / 404 body) → nothing.
        return Vec::new();
    }
    parse_lrc(body)
}

/// Parse LRC: leading `[mm:ss.xx]` (repeatable) timestamps per line.
fn parse_lrc(s: &str) -> Vec<LyricLine> {
    let mut out = Vec::new();
    for raw in s.lines() {
        let mut rest = raw.trim();
        if rest.is_empty() {
            continue;
        }
        let mut times = Vec::new();
        loop {
            let Some(t) = rest.strip_prefix('[') else { break };
            let Some(close) = t.find(']') else { break };
            if let Some(secs) = parse_ts(&t[..close]) {
                times.push(secs);
            }
            rest = t[close + 1..].trim_start();
        }
        let text = rest.to_string();
        if text.is_empty() {
            continue;
        }
        if times.is_empty() {
            out.push(LyricLine { time: -1.0, text });
        } else {
            for t in times {
                out.push(LyricLine { time: t, text: text.clone() });
            }
        }
    }
    out
}

/// Parse an `[mm:ss]` / `[mm:ss.xx]` / `[mm:ss.xxx]` timestamp to seconds.
fn parse_ts(tag: &str) -> Option<f64> {
    let (m, s) = tag.split_once(':')?;
    let m: f64 = m.trim().parse().ok()?;
    let s: f64 = s.trim().parse().ok()?;
    Some(m * 60.0 + s)
}

fn parse_plain(s: &str) -> Vec<LyricLine> {
    s.lines()
        .map(|l| LyricLine { time: -1.0, text: l.trim().to_string() })
        .filter(|l| !l.text.is_empty())
        .collect()
}

/// Sanitize a track/artist name into a single filesystem-safe token.
fn safe(s: &str) -> String {
    let mut out = String::with_capacity(s.len().min(80));
    for c in s.trim().chars().take(80) {
        if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | ' ') {
            out.push(if c == ' ' { '_' } else { c });
        } else {
            out.push('_');
        }
    }
    out
}

fn cache_dir() -> Option<PathBuf> {
    std::env::var("HOME")
        .ok()
        .map(|h| PathBuf::from(h).join(".cache/zen-shell/lyrics"))
}

fn cache_path(artist: &str, title: &str) -> Option<PathBuf> {
    cache_dir().map(|d| d.join(format!("{}__{}.lrc", safe(artist), safe(title))))
}

fn read_cache(artist: &str, title: &str) -> Option<Vec<LyricLine>> {
    let p = cache_path(artist, title)?;
    if !p.exists() {
        return None;
    }
    let body = std::fs::read_to_string(p).ok()?;
    let l = parse_lrc(&body);
    if !l.is_empty() {
        Some(l)
    } else {
        Some(parse_plain(&body))
    }
}

fn write_cache(artist: &str, title: &str, lines: &[LyricLine]) {
    let Some(p) = cache_path(artist, title) else { return };
    if let Some(parent) = p.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let body = lines
        .iter()
        .map(|l| {
            if l.time >= 0.0 {
                format!("[{:02}:{:05.2}] {}", (l.time / 60.0) as u64, l.time % 60.0, l.text)
            } else {
                l.text.clone()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    let _ = std::fs::write(p, body);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_synced_lrc() {
        let lines = parse_lrc("[00:00.00]Verse one\n[00:04.5]Verse two\n[00:08.00][00:12.00]Chorus\n");
        assert_eq!(lines.len(), 4);
        assert_eq!(lines[0].time, 0.0);
        assert_eq!(lines[1].text, "Verse two");
        assert!((lines[1].time - 4.5).abs() < 1e-9);
        assert_eq!(lines[2].text, "Chorus");
        assert_eq!(lines[3].time, 12.0);
        assert!(has_timing(&lines));
    }

    #[test]
    fn parses_plain_lines_as_unsynced() {
        let lines = parse_plain("  first line \n\nsecond line\n");
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].text, "first line");
        assert_eq!(lines[0].time, -1.0);
        assert!(!has_timing(&lines));
    }

    #[test]
    fn lrclib_json_preferred_over_raw() {
        let body = r#"{"syncedLyrics":"[00:01.00]hi","plainLyrics":"plain fallback"}"#;
        let lines = parse(body);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].text, "hi");
        assert_eq!(lines[0].time, 1.0);
    }

    #[test]
    fn error_body_is_not_lyrics() {
        // JSON 404 → nothing
        assert!(parse(r#"{"statusCode":404,"message":"Track not found"}"#).is_empty());
        // a bare string has no timestamps → one unsynced (plain) line
        let lines = parse("just some random text without brackets");
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].time, -1.0);
    }

    #[test]
    fn normalize_squashes_and_drops_suffixes() {
        assert_eq!(normalize("  Limp  Bizkit "), normalize("Limp Bizkit"));
        assert_eq!(normalize("Numb (Official Video)"), normalize("Numb"));
        assert!(!normalize("Rollin' (Air Raid Vehicle)").is_empty());
    }

    #[test]
    fn lev_scores_small_edits() {
        assert_eq!(lev("abc", "abc"), 0.0);
        assert_eq!(lev("abc", "abd"), 1.0);
        assert_eq!(lev("", "xyz"), 3.0);
    }

    #[test]
    fn active_word_tracks_progress() {
        let lines = [
            LyricLine { time: 0.0, text: "one two three four".into() },
            LyricLine { time: 8.0, text: "next line".into() },
        ];
        assert_eq!(active_word(&lines, 0.0, 0), Some(0));
        assert_eq!(active_word(&lines, 2.0, 0), Some(1));
        assert_eq!(active_word(&lines, 4.9, 0), Some(2));
        assert_eq!(active_word(&lines, 8.0, 1), Some(0));
    }
}