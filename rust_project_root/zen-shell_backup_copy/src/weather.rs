//! Weather via Open-Meteo (no API key, plain HTTP — no TLS deps).
//!
//! A detached thread fetches `current_weather` every `interval` and pushes a
//! `WeatherMsg` over a calloop channel; the shell wakes only when a message
//! arrives (0 CPU while idle). Failures are silent — the last known weather
//! stays on screen and the next tick retries.

use crate::icons::*;
use smithay_client_toolkit::reexports::calloop;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

pub const HOST: &str = "api.open-meteo.com";

#[derive(Clone, Debug, PartialEq)]
pub struct WeatherMsg {
    pub temp_c: f32,
    /// WMO weather code (0 clear … 99 thunderstorm)
    pub code: u8,
    pub wind_kmh: f32,
    /// relative humidity % (None if the detail query failed)
    pub humidity: Option<f32>,
    /// apparent (feels-like) temperature °C
    pub feels_like: Option<f32>,
    /// precipitation in mm (past hour)
    pub precip_mm: Option<f32>,
    /// geolocated city name (Some only when lat/lon were auto-detected)
    pub city: Option<String>,
    /// 7-day forecast lookahead (`daily` block). Empty on the first framing /
    /// while the daily block is missing — cards render a placeholder.
    pub daily: Vec<DailyFc>,
}

/// One day of the `daily` forecast block (per-day high/low + weather code +
/// precipitation probability).
#[derive(Clone, Debug, PartialEq)]
pub struct DailyFc {
    pub code: u8,
    pub tmax_c: f32,
    pub tmin_c: f32,
    pub precip_prob: f32,
    /// 0 = today; Open-Meteo returns the daily block starting today.
    pub day_offset: u8,
}

/// Minimal HTTP/1.1 GET (Connection: close). No TLS, no redirects, but it
/// does decode chunked transfer-encoding (Open-Meteo uses it).
fn http_get(host: &str, path: &str, timeout: Duration) -> Result<String, String> {
    let mut sock = TcpStream::connect((host, 80)).map_err(|e| format!("connect: {e}"))?;
    sock.set_read_timeout(Some(timeout)).ok();
    sock.set_write_timeout(Some(timeout)).ok();
    let req = format!(
        "GET {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\nUser-Agent: zen-shell/0.1\r\nAccept: application/json\r\n\r\n"
    );
    sock.write_all(req.as_bytes()).map_err(|e| format!("write: {e}"))?;
    let mut resp = Vec::new();
    sock.read_to_end(&mut resp).map_err(|e| format!("read: {e}"))?;
    let text = String::from_utf8_lossy(&resp).into_owned();
    let (head, body) = text.split_once("\r\n\r\n").ok_or_else(|| "malformed response".to_string())?;
    if head.to_ascii_lowercase().contains("transfer-encoding: chunked") {
        decode_chunked(body).ok_or_else(|| "bad chunked body".to_string())
    } else {
        Ok(body.to_string())
    }
}

/// `1da\r\n<bytes>\r\n0\r\n\r\n` framing → concatenated payload.
fn decode_chunked(body: &str) -> Option<String> {
    let mut out = String::new();
    let mut rest = body;
    loop {
        let line_end = rest.find('\n')?;
        let size_line = rest[..line_end].trim_end_matches('\r');
        // strip chunk extensions (`1da;foo=bar`)
        let size_str = size_line.split(';').next().unwrap_or("").trim();
        let size = usize::from_str_radix(size_str, 16).ok()?;
        rest = &rest[line_end + 1..];
        if size == 0 {
            return Some(out);
        }
        if rest.len() < size + 2 {
            return None;
        }
        out.push_str(&rest[..size]);
        rest = &rest[size..];
        rest = rest.strip_prefix("\r\n").or_else(|| rest.strip_prefix('\n'))?;
    }
}

fn fetch(lat: f64, lon: f64, timeout: Duration) -> Result<WeatherMsg, String> {
    // `current_weather=true` keeps the legacy fields (temp/code/wind); the
    // `current=` vars add humidity / feels-like / precipitation for the
    // detailed panel. Both blocks come back in one response.
    let path = format!(
        "/v1/forecast?latitude={lat}&longitude={lon}&current_weather=true\
         &current=temperature_2m,relative_humidity_2m,apparent_temperature,\
         precipitation,weather_code,wind_speed_10m\
         &daily=weather_code,temperature_2m_max,temperature_2m_min,\
         precipitation_probability_max&timezone=auto&forecast_days=7"
    );
    let body = http_get(HOST, &path, timeout)?;
    let v: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| format!("json: {e}"))?;
    let cw = v.get("current_weather").ok_or("no current_weather")?;
    let temp_c = cw.get("temperature").and_then(|t| t.as_f64()).ok_or("no temp")? as f32;
    let code = cw.get("weathercode").and_then(|c| c.as_u64()).ok_or("no code")? as u8;
    let wind_kmh = cw.get("windspeed").and_then(|w| w.as_f64()).ok_or("no wind")? as f32;
    // Detail block is optional — a missing field just hides that row
    let cur = v.get("current").cloned().unwrap_or_default();
    let f = |k: &str| cur.get(k).and_then(|x| x.as_f64()).map(|x| x as f32);
    // Daily lookahead is optional too — cards degrade to a placeholder
    let daily = {
        let d = v.get("daily").cloned().unwrap_or_default();
        let arr = |k: &str| d.get(k).and_then(|x| x.as_array()).cloned().unwrap_or_default();
        let codes = arr("weather_code");
        let tmax = arr("temperature_2m_max");
        let tmin = arr("temperature_2m_min");
        let precip = arr("precipitation_probability_max");
        let day = |a: &Vec<serde_json::Value>, i: usize| a.get(i).and_then(|x| x.as_f64()).map(|x| x as f32);
        let n = codes.len().max(tmax.len()).max(tmin.len()).max(precip.len());
        (0..n)
            .map(|i| DailyFc {
                code: codes.get(i).and_then(|x| x.as_f64()).map(|x| x as u8).unwrap_or(0),
                tmax_c: day(&tmax, i).unwrap_or(0.0),
                tmin_c: day(&tmin, i).unwrap_or(0.0),
                precip_prob: day(&precip, i).unwrap_or(0.0),
                day_offset: i.min(255) as u8,
            })
            .collect()
    };
    Ok(WeatherMsg {
        temp_c,
        code,
        wind_kmh,
        humidity: f("relative_humidity_2m"),
        feels_like: f("apparent_temperature"),
        precip_mm: f("precipitation"),
        city: None,
        daily,
    })
}

/// Geolocate via ip-api.com (plain HTTP, same provider the quickshell widget
/// uses). Returns (lat, lon, city); `None` on failure → retried later.
fn locate() -> Option<(f64, f64, String)> {
    let body = http_get("ip-api.com", "/json/?fields=status,lat,lon,city", Duration::from_secs(5)).ok()?;
    let v: serde_json::Value = serde_json::from_str(&body).ok()?;
    if v.get("status").and_then(|s| s.as_str()) != Some("success") {
        return None;
    }
    let lat = v.get("lat").and_then(|x| x.as_f64())?;
    let lon = v.get("lon").and_then(|x| x.as_f64())?;
    let city = v
        .get("city")
        .and_then(|c| c.as_str())
        .map(|c| c.to_string())
        .unwrap_or_else(|| format!("{lat:.2}, {lon:.2}"));
    Some((lat, lon, city))
}

/// One-shot weather fetch for the broker: geolocate if needed, fetch once,
/// and send the result. Returns `false` if geolocation is still unresolved
/// (the broker will retry on its next tick). Run on a detached thread.
pub fn fetch_once(
    tx: calloop::channel::Sender<WeatherMsg>,
    lat: f64,
    lon: f64,
) -> bool {
    std::thread::spawn(move || {
        let (mut lat, mut lon, mut city) = (lat, lon, String::new());
        if lat == 0.0 && lon == 0.0 {
            let Some((l, o, c)) = locate() else { return };
            lat = l;
            lon = o;
            city = c;
        }
        if let Ok(mut w) = fetch(lat, lon, Duration::from_secs(5)) {
            if !city.is_empty() {
                w.city = Some(city.clone());
            }
            let _ = tx.send(w);
        }
    });
    true
}

/// WMO weather code → (nerd-font glyph, short description).
pub fn describe(code: u8) -> (&'static str, &'static str) {
    match code {
        0 => (ICON_BRIGHTNESS, "Clear"),
        1 => (ICON_CLOUD, "Mostly clear"),
        2 | 3 => (ICON_CLOUD, "Cloudy"),
        45 | 48 => (ICON_CLOUD, "Fog"),
        51..=57 => (ICON_DROP, "Drizzle"),
        61..=67 => (ICON_DROP, "Rain"),
        71..=77 => (ICON_SNOW, "Snow"),
        80..=82 => (ICON_DROP, "Showers"),
        95..=99 => (ICON_BOLT, "Thunder"),
        _ => (ICON_CLOUD, "Cloudy"),
    }
}
