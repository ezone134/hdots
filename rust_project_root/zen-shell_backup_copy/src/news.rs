//! Headline fetching for the News dashboard card.
//!
//! Sources, driven by config in shell.toml:
//!   - any number of named feeds (`[[news_feeds]] name=… url=…`) — each feed
//!     becomes one horizontal category chip on the card, and its stories are
//!     tagged with the feed name,
//!   - a legacy single feed (`[news_url]`) — the RSS channel title is used as
//!     the category,
//!   - a flat JSON file the card reads directly when nothing is configured:
//!     `~/.cache/zen-shell/news.json` →
//!     `[{"title": …,"source": …,"url": …,"category": …}, …]`
//!     (a scheduled script can write this; it always wins over the network).
//!
//! Feeds are pulled with `curl` (8 s timeout) on a background thread so a
//! slow upstream never blocks the bar. Parsing is deliberately lightweight
//! string-scanning (the crate has no XML parser).

use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct NewsItem {
    pub title: String,
    pub source: String,
    pub url: String,
    pub category: String,
}

/// One named feed → one category chip (`[[news_feeds]]` in shell.toml).
#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize, Default)]
pub struct NewsFeedConfig {
    pub name: String,
    pub url: String,
}

/// Message pumped back to the shell from the fetch worker thread.
pub enum NewsMsg {
    Items(Vec<NewsItem>),
}

/// Where the manual/local JSON feed lives (`~/.cache/zen-shell/news.json`).
pub fn local_news_path() -> PathBuf {
    let mut p = std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"));
    p.push(".cache/zen-shell/news.json");
    p
}

/// Parse a flat JSON feed: `[{"title":…,"source":…,"url":…,"category":…}, …]`
/// (top 64 kept).
pub fn parse_json_feed(body: &str) -> Vec<NewsItem> {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(body) else {
        return Vec::new();
    };
    let Some(arr) = v.as_array() else {
        return Vec::new();
    };
    arr.iter()
        .filter_map(|o| {
            let title = o.get("title").and_then(|t| t.as_str()).unwrap_or("").trim().to_string();
            if title.is_empty() {
                return None;
            }
            let source = o.get("source").and_then(|t| t.as_str()).unwrap_or("").trim().to_string();
            let url = o.get("url").and_then(|t| t.as_str()).unwrap_or("").trim().to_string();
            let category = o.get("category").and_then(|t| t.as_str()).unwrap_or("").trim().to_string();
            Some(NewsItem { title, source, url, category })
        })
        .take(64)
        .collect()
}

/// Pull the text out of a `<title>…</title>` (or `<title …>`) element, first
/// occurrence in `s`; CDATA-wrapped titles are unwrapped.
fn title_in(s: &str) -> Option<String> {
    let i = s.find("<title")?;
    let after = &s[i + "<title".len()..];
    let open_end = after.find('>')?;
    let body = &after[open_end + 1..];
    let end = body.find("</title")?;
    let mut t = body[..end].trim().to_string();
    if t.is_empty() {
        return None;
    }
    if let (Some(a), Some(b)) = (t.find("<![CDATA["), t.find("]]>")) {
        if b > a {
            t = t[a + "<![CDATA[".len()..b].to_string();
        }
    }
    Some(t)
}

/// The article URL of an `<item>` / `<entry>`: the `<link>…</link>` body,
/// or a `<link href="…"/>` / `<link href="…">` attribute.
fn link_in(chunk: &str) -> String {
    let Some(i) = chunk.find("<link") else { return String::new() };
    let after = &chunk[i + 5..];
    // attribute form (Atom): <link href="…" rel="alternate" type="text/html"/>
    if let Some(gt) = after.find('>') {
        let attr = &after[..gt];
        let href_off = attr.find("href");
        if let Some(h) = attr.find("href") {
            let rest = &attr[h + 5..];
            let rest = rest.trim_start();
            if let Some(q) = rest.strip_prefix('"') {
                if let Some(end) = q.find('"') {
                    let url = q[..end].trim().to_string();
                    if !url.is_empty() {
                        return url;
                    }
                }
            }
            let _ = href_off;
        }
    }
    // body form (RSS): <link>https://…</link>
    let after = &chunk[i + "<link".len()..];
    let Some(open_end) = after.find('>') else { return String::new() };
    let body = &after[open_end + 1..];
    let end = body.find("</link").unwrap_or(body.len());
    body[..end].trim().to_string()
}

/// RSS `<category>` text (rare); empty when absent.
fn category_in(chunk: &str) -> String {
    if let Some(i) = chunk.find("<category") {
        let after = &chunk[i + "<category".len()..];
        if let Some(oe) = after.find('>') {
            let body = &after[oe + 1..];
            if let Some(end) = body.find("</category") {
                let t = body[..end].trim();
                if !t.is_empty() {
                    return t.to_string();
                }
            }
        }
    }
    String::new()
}

/// Source label for an `<item>` (its `<source>` value) or an `<entry>`
/// (its `<author><name>` / `<name>`), else the feed's `<channel><title>`.
fn source_in(chunk: &str, fallback: &str) -> String {
    if let Some(i) = chunk.find("<source") {
        let after = &chunk[i + "<source".len()..];
        if let Some(oe) = after.find('>') {
            let body = &after[oe + 1..];
            if let Some(end) = body.find("</source") {
                let t = body[..end].trim();
                if !t.is_empty() {
                    return t.to_string();
                }
            }
        }
    }
    if let Some(i) = chunk.find("<author>") {
        let after = &chunk[i + "<author>".len()..];
        if let Some(end) = after.find("</author>") {
            let t = after[..end].trim().to_string();
            if !t.is_empty() {
                return t;
            }
        }
    }
    fallback.to_string()
}

/// RSS 2.0 (`<item>` records). Returns (title, source) pairs.
fn parse_rss(body: &str) -> Vec<NewsItem> {
    let channel = title_in(body).unwrap_or_default();
    let mut out = Vec::new();
    // split("<item") yields the channel head first — skip it, only `item`s count
    for chunk in body.split("<item").skip(1) {
        let Some(title) = title_in(chunk) else { continue };
        let cat = category_in(chunk);
        out.push(NewsItem {
            title,
            source: source_in(chunk, &channel),
            url: link_in(chunk),
            category: if cat.is_empty() { channel.clone() } else { cat },
        });
        if out.len() >= 64 {
            break;
        }
    }
    out
}

/// Atom (`<entry>` records).
fn parse_atom(body: &str) -> Vec<NewsItem> {
    let mut out = Vec::new();
    for chunk in body.split("<entry").skip(1) {
        let Some(title) = title_in(chunk) else { continue };
        let mut source = String::new();
        if let Some(i) = chunk.find("<name>") {
            let after = &chunk[i + "<name>".len()..];
            if let Some(end) = after.find("</name>") {
                source = after[..end].trim().to_string();
            }
        }
        out.push(NewsItem {
            title,
            source: if source.is_empty() { "Atom feed".into() } else { source },
            url: link_in(chunk),
            category: String::new(),
        });
        if out.len() >= 64 {
            break;
        }
    }
    out
}

fn parse_feed(body: &str) -> Vec<NewsItem> {
    let rss = parse_rss(body);
    if !rss.is_empty() {
        return rss;
    }
    parse_atom(body)
}

/// The full fetch: local JSON file first (direct inject), then each
/// configured feed. Feed stories get `category = feed.name` unless the feed
/// supplied its own `<category>`. Never panics, never blocks the loop — call
/// from a worker thread.
pub fn fetch_news(feeds: &[NewsFeedConfig]) -> Vec<NewsItem> {
    // 1) a local JSON feed beats the network when present
    if let Ok(body) = std::fs::read_to_string(local_news_path()) {
        let local = parse_json_feed(&body);
        if !local.is_empty() {
            return local;
        }
    }
    // 2) networked RSS/Atom feeds (each one a category)
    let mut out = Vec::new();
    for f in feeds {
        let url = f.url.trim();
        if url.is_empty() {
            continue;
        }
        let fetched = std::process::Command::new("curl")
            .args(["-sL", "--max-time", "8"])
            .arg(url)
            .output();
        let Ok(fetched) = fetched else { continue };
        if !fetched.status.success() {
            continue;
        }
        let Ok(body) = String::from_utf8(fetched.stdout) else { continue };
        let mut items = parse_feed(&body);
        if !f.name.is_empty() {
            for it in items.iter_mut() {
                if it.category.is_empty() {
                    it.category = f.name.clone();
                }
            }
        }
        out.extend(items);
        if out.len() >= 64 {
            break;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rss_titles_and_sources() {
        let body = r#"
        <rss><channel><title>Tech News</title>
        <item><title>First headline</title><source url="x">Vendor</source></item>
        <item><title><![CDATA[Second &amp; more]]></title></item>
        </channel></rss>"#;
        let items = parse_feed(body);
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].title, "First headline");
        assert_eq!(items[0].source, "Vendor");
        assert_eq!(items[1].title, "Second &amp; more");
        assert_eq!(items[1].source, "Tech News");
    }

    #[test]
    fn atom_entries() {
        let body = r#"<feed><entry><title>Atom item</title><author><name>Alice</name></author></entry></feed>"#;
        let items = parse_feed(body);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].title, "Atom item");
        assert_eq!(items[0].source, "Alice");
    }

    #[test]
    fn json_feed() {
        let items = parse_json_feed(r#"[{"title":"a","source":"src"},{"title":"b"}]"#);
        assert_eq!(items.len(), 2);
        assert_eq!(items[1].source, "");
    }

    #[test]
    fn stale_or_invalid_falls_back_empty() {
        assert!(parse_feed("<html>no feed here</html>").is_empty());
        assert!(parse_json_feed("not json").is_empty());
    }
}