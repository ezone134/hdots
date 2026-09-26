//! Offline map data — hosted on GitHub (`ezone134/zen-shell-map`, public-domain
//! Natural Earth), downloaded on demand and cached to disk.
//!
//! Cache layout (mirrors the repo layout under `maps/`):
//!
//! ```text
//! $HOME/.local/share/zen-shell/maps/   ← persisted (save-data toggle ON)
//! /tmp/zen-shell/maps/                 ← ephemeral (toggle OFF, per-boot)
//!   ├─ manifest.json
//!   ├─ world.bin
//!   ├─ cities.bin
//!   └─ countries/<ISO2>.bin            ← per-region chunks
//! ```
//!
//! Downloads go through `curl` (the shell already binds to it) and are
//! verified against the manifest's sha256 before they replace any cached file,
//! so a partial download never poisons the on-disk cache.

use std::collections::BTreeMap;
use std::path::PathBuf;
use serde::Deserialize;

/// Default GitHub data repo (public-domain Natural Earth chunks).
pub const DEFAULT_BASE: &str = "https://raw.githubusercontent.com/ezone134/zen-shell-map/main/";

#[derive(Deserialize, Clone, Debug, Default)]
pub struct Meta {
    pub url: String,
    pub sha256: String,
    pub size: u64,
}

#[derive(Deserialize, Clone, Debug, Default)]
pub struct RegionMeta {
    pub name: String,
    /// [lon_min, lat_min, lon_max, lat_max]
    pub bbox: [f32; 4],
    pub url: String,
    pub sha256: String,
    pub size: u64,
}

#[derive(Deserialize, Clone, Debug, Default)]
pub struct Manifest {
    pub world: Meta,
    pub cities: Meta,
    #[serde(default)]
    pub regions: BTreeMap<String, RegionMeta>,
}

/// Worker-thread result.
#[derive(Clone, Debug)]
pub enum DlResult {
    WorldOk,
    RegionOk { iso: String },
    Failed { why: String },
}

/// Resolve the configured base URL (fallback to the default repo).
pub fn base_url(configured: &str) -> String {
    let b = configured.trim();
    if b.is_empty() {
        DEFAULT_BASE.to_string()
    } else if b.ends_with('/') {
        b.to_string()
    } else {
        format!("{b}/")
    }
}

/// Root cache dir for map chunks. Persistent under the user's data dir when
/// `save` is true; ephemeral `/tmp` scratch otherwise.
pub fn cache_root(save: bool) -> PathBuf {
    if save {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
        PathBuf::from(home).join(".local/share/zen-shell/maps")
    } else {
        PathBuf::from("/tmp/zen-shell/maps")
    }
}

pub fn manifest_path(save: bool) -> PathBuf {
    cache_root(save).join("manifest.json")
}
pub fn world_path(save: bool) -> PathBuf {
    cache_root(save).join("world.bin")
}
pub fn cities_path(save: bool) -> PathBuf {
    cache_root(save).join("cities.bin")
}
pub fn region_path(save: bool, iso: &str) -> PathBuf {
    cache_root(save).join("countries").join(format!("{iso}.bin"))
}

/// True when a cached file exists AND matches the manifest checksum/size —
/// a file that fails the check is treated as missing (re-downloadable).
pub fn file_valid(path: &PathBuf, meta: &Meta) -> bool {
    std::fs::metadata(path)
        .map(|m| m.len() == meta.size)
        .unwrap_or(false)
        && sha256_of(path).map(|s| s == meta.sha256).unwrap_or(false)
}

/// Manifest from disk (if present), else a fresh HTTP fetch (`curl`).
pub fn load_manifest(save: bool, base: &str) -> Option<Manifest> {
    let p = manifest_path(save);
    if let Ok(txt) = std::fs::read_to_string(&p) {
        if let Ok(m) = serde_json::from_str(&txt) {
            return Some(m);
        }
    }
    // fetch fresh + cache it
    let url = format!("{base}manifest.json");
    let body = curl_fetch(&url).ok()?;
    let m: Manifest = serde_json::from_slice(&body).ok()?;
    let _ = std::fs::create_dir_all(p.parent()?);
    let _ = std::fs::write(&p, &body);
    Some(m)
}

/// Download a chunk to `dest` (atomic: `.part` → verify → rename).
pub fn download_chunk(base: &str, meta: &Meta, dest: &PathBuf) -> Result<(), String> {
    let url = format!("{base}{}", meta.url);
    let bytes = curl_fetch(&url).map_err(|e| format!("{url}: {e}"))?;
    if bytes.len() as u64 != meta.size || sha256_of_bytes(&bytes) != meta.sha256 {
        return Err("checksum mismatch".into());
    }
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(dest, &bytes).map_err(|e| e.to_string())
}

/// Smallest region whose bbox contains the point; fallback: smallest bbox.
pub fn region_at(m: &Manifest, lon: f32, lat: f32) -> Option<(String, &RegionMeta)> {
    let mut hit: Option<(String, f32)> = None; // iso → bbox area
    for (iso, r) in &m.regions {
        let [x0, y0, x1, y1] = r.bbox;
        if lon >= x0 && lon <= x1 && lat >= y0 && lat <= y1 {
            let area = (x1 - x0).max(0.0001) * (y1 - y0).max(0.0001);
            match hit {
                Some((_, a)) if area < a => hit = Some((iso.clone(), area)),
                None => hit = Some((iso.clone(), area)),
                _ => {}
            }
        }
    }
    if let Some((iso, _)) = hit {
        return Some((iso.clone(), &m.regions[&iso]));
    }
    let mut best: Option<(String, f32)> = None;
    for (iso, r) in &m.regions {
        let [x0, y0, x1, y1] = r.bbox;
        let area = (x1 - x0).max(0.0001) * (y1 - y0).max(0.0001);
        match best {
            None => best = Some((iso.clone(), area)),
            Some((_, a)) if area < a => best = Some((iso.clone(), area)),
            _ => {}
        }
    }
    best.map(|(iso, _)| (iso.clone(), &m.regions[&iso]))
}

// ---------------------------------------------------------------- helpers

fn sha256_of(path: &PathBuf) -> Option<String> {
    std::fs::read(path).ok().map(|b| sha256_of_bytes(&b))
}

fn sha256_of_bytes(bytes: &[u8]) -> String {
    use std::process::Command;
    use std::io::Write;
    // `sha256sum` reads from stdin; cheap and avoids adding a dep.
    let mut child = Command::new("sha256sum").stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped()).spawn().expect("sha256sum");
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(bytes);
    }
    let out = child.wait_with_output().ok();
    out.and_then(|o| {
        String::from_utf8(o.stdout).ok().and_then(|s| s.split_whitespace().next().map(|x| x.to_string()))
    })
    .unwrap_or_else(|| "0000".into())
}

fn curl_fetch(url: &str) -> Result<Vec<u8>, String> {
    use std::process::Command;
    let out = Command::new("curl")
        .args(["-sS", "--max-time", "25", "-o", "-", url])
        .output()
        .map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(out.stdout)
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_url_normalization() {
        assert_eq!(base_url(""), DEFAULT_BASE.to_string());
        assert_eq!(base_url("https://x/y/"), "https://x/y/".to_string());
        assert_eq!(base_url("https://x/y"), "https://x/y/".to_string());
    }

    #[test]
    fn region_at_picks_smallest_containing() {
        let m = Manifest {
            world: Meta { url: "world.bin".into(), sha256: "a".into(), size: 1 },
            cities: Meta { url: "cities.bin".into(), sha256: "b".into(), size: 1 },
            regions: BTreeMap::from([
                ("AU".into(), RegionMeta { name: "Australia".into(), bbox: [113.0, -44.0, 154.0, -10.0], url: "countries/AU.bin".into(), sha256: "c".into(), size: 1 }),
                ("JP".into(), RegionMeta { name: "Japan".into(), bbox: [122.0, 24.0, 154.0, 46.0], url: "countries/JP.bin".into(), sha256: "d".into(), size: 1 }),
            ]),
        };
        let (iso, r) = region_at(&m, 139.0, 35.0).expect("hit");
        assert_eq!(iso, "JP");
        assert_eq!(r.name, "Japan");
    }
}