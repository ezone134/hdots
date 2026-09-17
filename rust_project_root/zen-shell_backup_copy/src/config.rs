//! Config parsing — mirrors the C++ raze-shell `config/shell.toml`.

use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::shell::{BannerToken, PillItem};

/// Default config written to `$XDG_CONFIG_HOME/zen-shell/shell.toml` when no
/// config file exists anywhere (first run / installed binary).
///
/// Prod first-run uses `Config::default()` (main.rs); this template is the
/// reference default the parse tests pin against.
#[cfg_attr(not(test), allow(dead_code))]
pub const DEFAULT_SHELL_TOML: &str = r##"# zen-shell configuration (auto-created default)

# News dashboard card: RSS/Atom feed(s) fetched by curl (8 s timeout).
# Each [[news_feeds]] entry becomes a clickable category chip on the card.
# Unset → the card reads ~/.cache/zen-shell/news.json ([{"title":…,"source":…,…},…]).
# news_url = "https://cyber.harvard.edu/rss/rss.html"
# [[news_feeds]]
# name = "Tech"
# url = "https://example.com/feed.xml"

# Mirror card camera device (/dev/videoN). Unset → first camera found.
# camera_dev = "/dev/video0"

# Visualizer bar style on the Viz card: 0 = bars, 1 = wave, 2 = blocks.
# viz_style = 0

[fonts]
family = "Inter"
# display family for hero numerals (big temps / % / clock digits) — tighter
# tracking than the UI family, designed for large sizes. Falls back to `family`.
# font_display = "Inter Display"
# mono family for technical / tabular figures (rates, sizes, sensor reads).
# Falls back to `family` when the mono family isn't installed.
# font_mono = "Geist Mono"
# icon font name — the family default comes from the active icon style
# (Settings → "Icon font" → fonts.icon_style); setting it here overrides.
# Glyph SLOTS per style live in config/icons/<style>.conf, translated at
# draw time — the constants in src/icons.rs keep their canonical codepoints.
icon = "Material Symbols Rounded"
# icon style = slot flavor: "google" | "caskadia" | "jetbrains" | "phosphor"
# icon_style = "google"

[bar]
height = 38
floating = true
expanded = false
floating_offset = 25
# resting pill width: 0 = auto (sizes to the active pill chips)
collapsed_width = 0
expanded_width = 1120
# dashboard grid zoom: 1.0 = default, 0.5 = half, 2.0 = double
grid_scale = 1.0
# horizontal padding inside the collapsed pill (px)
pill_margin_x = 18
# resting pill background alpha (0 = transparent, 255 = opaque; default 176 ≈ 69%)
pill_alpha = 176
# dashboard card background alpha (0 = transparent, 255 = opaque; default 204 ≈ 80%)
dash_card_alpha = 204
# power menu / lockscreen hold-to-confirm delay (s); 0.5..=3.0, default 1.5
power_hold_delay = 1.5
# Floating pill: hovers with floating_offset margin over content. Off → the
# pill/dashboard sticks flush to the top/bottom edge.
# Reserve screen space for the resting pill (push windows below/above)
reserve = false
# Hovering the pill expands it into the dashboard (off → click to open,
# right-click on the pill always opens Settings)
expand_on_hover = true
# gear chip on the collapsed pill → Settings
settings_on_pill = true
# "bottom" (default) | "top" — where the bar floats; also movable from the
# dashboard's ▲/▼ arrows while running
anchor = "bottom"
# collapsed-pill widget visibility (Settings → Pill toggles; all default on)
# show_clock = true
# show_battery = true
# show_battery_pct = true  # % text outside the icon, not inside
# show_ws = true
# show_tray = true
# show_viz = false

[animation]
# Springy Dynamic-Island-style morph: expansions pop with a soft overshoot,
# collapses settle smoothly. Tune this to taste (ms).
duration_ms = 240.0

[backends]
power = "sysfs"
brightness = "sysfs"
audio = "wpctl"
lockscreen = "native"
wallpaper = "command"

[weather]
# Open-Meteo (no API key); unset lat/lon hides the widget
latitude = 0.0
longitude = 0.0
city = ""
interval_s = 600

[world]
# World-map card offline data. Chunks are hosted on GitHub (public-domain
# Natural Earth); data_url overrides the default repo. save_data ON → cache
# under ~/.local/share/zen-shell/maps, OFF → /tmp/zen-shell/maps.
# data_url = "https://raw.githubusercontent.com/ezone134/zen-shell-map/main/"
save_data = false

[wallpaper]
# Wallpaper picker directory + set command ({path} / {monitor} placeholders).
# Unset → defaults to $HOME/Wallpapers.
dir = "~/Wallpapers"
command = "set_wall {path}"

[colors]
fg = "#cdd6f4"
bg = "#1e1e2e"
acc = "#89b4fa"
hover = "#313244"

[notifications]
# "top-right" | "top-left" | "bottom-right" | "bottom-left"
position = "top-right"
# popup timeout in ms (0 = until dismissed)
timeout_ms = 5000
# max visible popups at once
max_visible = 5
# max notifications kept in history
max_history = 64
# show popups while the lockscreen is active
show_on_lock = false
# Per-app rules — override timeout or block an app entirely:
# [notifications.per_app."discord"]
# block = true
# [notifications.per_app."slack"]
# timeout_ms = 10000
"##;

/// A dashboard card persisted into the per-state config — its id plus its
/// exact grid placement (x/y origin + w/h span in cells). x/y are the ACTUAL
/// packed positions the shell renders, so cards come back exactly where the
/// user left them. Only cards currently enabled (on the dashboard) are listed.
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct DashCardEntry {
    pub card: String,
    pub x: u16,
    pub y: u16,
    pub w: u16,
    pub h: u16,
}

#[derive(Deserialize, Serialize, Clone, Default)]
#[serde(default)]
pub struct Config {
    pub fonts: Fonts,
    pub bar: Bar,
    pub animation: Animation,
    pub colors: Colors,
    pub weather: Weather,
    pub wallpaper: Wallpaper,
    pub world: World,
    pub backends: Backends,
    #[serde(default)]
    pub notifications: Notifications,
    /// enabled dashboard cards with their remembered sizes + positions
    /// (persisted per channel into `$states/shell_{n,d,l}`).
    #[serde(default)]
    pub dash_cards: Vec<DashCardEntry>,
    /// dashboard cards parked in the edit-mode tray (id order). They stay
    /// enabled-config-agnostic: hidden off-dashboard but remembered.
    #[serde(default)]
    pub dash_tray: Vec<String>,
    /// edit-mode dashboard grid boost (the rows/cols "+"/"−" buttons): MIN
    /// columns the canvas must span (0 = auto = packed content). A bigger
    /// value makes the dashboard wider. Persisted per channel.
    #[serde(default)]
    pub dash_grid_cols: Option<u16>,
    /// edit-mode grid boost: MIN rows (0 = auto) — makes the dashboard taller.
    #[serde(default)]
    pub dash_grid_rows: Option<u16>,
    /// RSS/Atom (or flat JSON) feed shown by the News dashboard card.
    /// Empty → the card reads `~/.cache/zen-shell/news.json` instead
    /// (`[{"title": …,"source": …},…]`). `curl` must be available for
    /// network feeds.
    #[serde(default)]
    pub news_url: Option<String>,
    /// Named news feeds — each becomes one horizontal category chip on the
    /// News card, filtering its stories. Overrides `news_url` when set.
    #[serde(default)]
    pub news_feeds: Vec<crate::news::NewsFeedConfig>,
    /// front camera used by the Mirror card (`/dev/videoN`); empty → the
    /// first camera the card can open.
    #[serde(default)]
    pub camera_dev: Option<String>,
    /// requested capture rate for the Mirror card (fps). The card cycles
    /// 15/30/60; the device delivers what it can. Unset → 60.
    #[serde(default)]
    pub mirror_fps: Option<u32>,
    /// show the actual delivered capture rate next to the fps label on the
    /// Mirror card (when the device can't hit the requested rate). Unset → on.
    #[serde(default)]
    pub mirror_show_fps: Option<bool>,
    /// show/hide the audio balance slider on the Sliders card (pane-2 toggle
    /// "Show balance"). Unset → on.
    #[serde(default)]
    pub show_balance: Option<bool>,
    /// show/hide the saturation slider on the Sliders card (pane-2 toggle
    /// "Show saturation"). Unset → on.
    #[serde(default)]
    pub show_saturation: Option<bool>,
    /// audio recorder card pane-1 toggle "Confirm delete" — demand a confirm
    /// click before removing a recording. Unset → on.
    #[serde(default)]
    pub audiorec_confirm_delete: Option<bool>,
    /// audio recorder card pane-1 toggle "Show mic slider" — hide the mic
    /// level fader on pane 0. Unset → on.
    #[serde(default)]
    pub audiorec_show_mic: Option<bool>,
    /// show/hide the TITLE text on dashboard card headers (edit-mode toggle
    /// "Card titles"). Unset → on.
    #[serde(default)]
    pub card_show_title: Option<bool>,
    /// show/hide the header GLYPH (icon) on dashboard cards (edit-mode toggle
    /// "Card glyphs"). Unset → on.
    #[serde(default)]
    pub card_show_glyph: Option<bool>,
    /// visualizer bar style (0 = rounded bars, 1 = wave, 2 = blocks), toggled
    /// by the style button on the Viz card.
    #[serde(default)]
    pub viz_style: Option<u8>,
    /// App-shortcut card: pinned app ids (order = slot order; max 8 slots).
    /// Empty → the card renders "+"-tiles the user fills from the launcher.
    #[serde(default)]
    pub app_shortcuts: Option<Vec<String>>,
}

/// Backend selection — each subsystem is a pluggable implementation (see
/// `src/backends.rs`). Unknown names fall back to the defaults below.
#[derive(Deserialize, Serialize, Clone, Default)]
#[serde(default)]
pub struct Backends {
    pub power: String,
    pub brightness: String,
    pub audio: String,
    pub lockscreen: String,
    pub wallpaper: String,
}

/// Wallpaper picker. `dir` lists the images shown as thumbnails; `command`
/// (with `{path}` and `{monitor}` placeholders) is spawned when one is
/// clicked. Unset dir → defaults to `$HOME/Wallpapers`.
#[derive(Deserialize, Serialize, Clone, Default)]
#[serde(default)]
pub struct Wallpaper {
    pub dir: String,
    pub command: String,
    /// picker thumbnail size in px (Ctrl+= / Ctrl+- zooms the grid live)
    pub cell: Option<f32>,
}

/// Optional Open-Meteo weather widget. No API key; plain HTTP GET from a
/// background thread every `interval_s` seconds. Lat/lon unset → hidden.
#[derive(Deserialize, Serialize, Clone, Default)]
#[serde(default)]
pub struct Weather {
    pub latitude: f64,
    pub longitude: f64,
    pub city: String,
    pub interval_s: u64,
}

/// Offline map data (world-map card). Chunks live on GitHub (public domain
/// Natural Earth); `data_url` overrides the default repo base. `default OFF (matches the card's menu toggle). `save_data`
/// ON → cached under `~/.local/share/zen-shell/maps`, OFF → `/tmp/zen-shell/maps`.
#[derive(Deserialize, Serialize, Clone)]
#[serde(default)]
pub struct World {
    pub data_url: String,
    pub save_data: bool,
}

impl Default for World {
    fn default() -> Self {
        World { data_url: String::new(), save_data: false }
    }
}

#[derive(Deserialize, Serialize, Clone, Default)]
#[serde(default)]
pub struct Fonts {
    pub family: String,
    /// Display family for hero numerals (default "Inter Display").
    pub display: String,
    /// Mono family for technical figures (default "Geist Mono").
    pub mono: String,
    pub icon: String,
    /// Icon slot flavor: "google" | "caskadia" | "jetbrains" | "phosphor".
    /// Empty/unset → Google. Dials which `config/icons/<style>.conf` table is
    /// translated at draw time + which family renders the glyphs.
    pub icon_style: String,
}

#[derive(Deserialize, Serialize, Clone, Default)]
#[serde(default)]
pub struct Bar {
    pub height: i32,
    pub floating: Option<bool>,
    pub expanded: bool,
    pub floating_offset: i32,
    pub collapsed_width: i32,
    pub expanded_width: i32,
    /// reserve screen space (exclusive zone) so the resting pill pushes
    /// windows below/above instead of floating over them
    pub reserve: Option<bool>,
    /// hover expands the pill → dashboard (Settings toggle; runtime).
    /// `Option` so a missing key keeps the `true` default.
    pub expand_on_hover: Option<bool>,
    /// hover / click on the resting pill opens the Control Center instead of
    /// the dashboard (Settings toggle)
    pub cc_instead_of_dash: Option<bool>,
    /// collapsed-pill widget visibility (Settings → Pill toggles; runtime).
    pub show_clock: Option<bool>,
    pub show_battery: Option<bool>,
    /// show battery % as text OUTSIDE the icon (collapsed pill + system card)
    /// rather than inside the battery body (Settings toggle; runtime).
    pub show_battery_pct: Option<bool>,
    pub show_ws: Option<bool>,
    /// collapsed pill: all workspace numbers (1..5) always visible
    pub show_ws_long: Option<bool>,
    /// collapsed pill: only the active workspace number
    pub show_ws_short: Option<bool>,
    /// collapsed pill: Wi-Fi chip → Wifi menu
    pub show_wifi: Option<bool>,
    /// collapsed pill: Bluetooth chip → BT menu
    pub show_bluetooth: Option<bool>,
    /// collapsed pill: volume chip → Sound panel
    pub show_volume: Option<bool>,
    /// collapsed pill: wallpaper chip → Wallpaper panel
    pub show_wallpaper: Option<bool>,
    /// collapsed pill: themes chip → Themes picker
    pub show_themes: Option<bool>,
    /// collapsed pill: brand glyph chip (display-only)
    pub show_brand: Option<bool>,
    pub show_tray: Option<bool>,
    /// music visualizer bars on the collapsed pill
    pub show_viz: Option<bool>,
    /// gear chip on the collapsed pill → Settings
    pub settings_on_pill: Option<bool>,
    /// notification bell on the collapsed pill
    pub show_notif: Option<bool>,
    /// user info chip on the collapsed pill
    pub show_user: Option<bool>,
    /// battery power draw (watts) chip on the collapsed pill
    pub show_watts: Option<bool>,
    /// brightness / dark-mode chip (sun ⇄ moon) on the collapsed pill
    pub show_brightness: Option<bool>,
    /// collapsed-pill item order (left → right); unknown names are skipped
    pub pill_order: Option<Vec<String>>,
    /// "bottom" (default) | "top" — where the bar floats
    pub anchor: Option<String>,
    /// dashboard grid zoom: 1.0 = default, 0.5 = half, 2.0 = double
    /// scales cells, padding, gaps, fonts, corner radii — full uniform zoom
    #[serde(default = "default_one")]
    pub grid_scale: f32,
    /// horizontal (left/right) margin inside the collapsed pill (px).
    /// 0 = edge-to-edge; default 8.
    #[serde(default)]
    pub pill_margin_x: Option<f32>,
    /// background alpha of the resting pill (0 = fully transparent,
    /// 255 = fully opaque). 0xFF when unset (opaque).
    #[serde(default)]
    pub pill_alpha: Option<u8>,
    /// background alpha of dashboard cards (0 = fully transparent,
    /// 255 = fully opaque). 0xCC when unset (≈80%).
    #[serde(default)]
    pub dash_card_alpha: Option<u8>,
    /// power menu / lockscreen hold-to-confirm delay in seconds (0.5..=3.0).
    /// Default 1.5.
    #[serde(default)]
    pub power_hold_delay: Option<f32>,
    /// allow the main mixer volume to exceed 100% (up to 200%); default on.
    /// Written to the per-state shell config by Settings → Sound.
    #[serde(default)]
    pub allow_over_100: Option<bool>,
    /// max volume cap in percent (0..200); default 100. Written to the
    /// per-state shell config by Settings → Sound max-volume slider.
    #[serde(default)]
    pub max_vol: Option<i32>,
    /// expanded-dashboard top-banner strip (left → right). Entries are:
    /// plain chip names (`workspaces`, `title`, …), `sep:<glyph>` for a
    /// normal separator (e.g. `sep:|`, `sep::`), or `filler` for a smart
    /// filler that pushes its neighbours to the strip edges. Unknown names
    /// are skipped; missing / empty → `DEFAULT_BANNER_ORDER`.
    pub banner_order: Option<Vec<String>>,
    /// dashboard cards ON (default true). When false the expanded panel
    /// shows only the top banner strip (it acts as the expanded bar) —
    /// cards are kept in the config but not rendered.
    pub dash_enabled: Option<bool>,
    /// scroll direction toggle for dashboard: false = vertical (rows),
    /// true = horizontal (columns). Default false.
    pub dash_scroll_dir: Option<bool>,
    /// manual width (px) of the banner-only strip when the dashboard is
    /// OFF. 0 / unset = auto-fit the strip to its chips. Grows the strip
    /// with the edit-mode “width +/−” buttons.
    pub banner_width: Option<i32>,
    /// extra space (px) between the window edges and the dashboard cards
    /// (left/right/bottom). Default 12 (resolved from the base pad).
    pub ui_pad: Option<f32>,
    /// window (panel / dashboard surface) corner roundness in px. 0..=20,
    /// default 24 → treated as 20 when over the slider range.
    pub win_radius: Option<f32>,
    /// dashboard card corner roundness in px. 0..=20, default 16.
    pub card_radius: Option<f32>,
    /// when true (default) cards follow the window roundness instead of
    /// their own value.
    pub card_radius_sync: Option<bool>,
}
fn default_one() -> f32 { 1.0 }

#[derive(Deserialize, Serialize, Clone, Default)]
#[serde(default)]
pub struct Animation {
    pub duration_ms: f32,
    /// ms to wait after pointer leave before collapsing Expanded → pill
    pub collapse_delay_ms: f32,
}

#[derive(Deserialize, Serialize, Clone, Default)]
#[serde(default)]
pub struct Colors {
    pub fg: String,
    pub bg: String,
    pub acc: String,
    pub hover: String,
}

/// Per-app notification rule: override global timeout or block notifications
/// from a specific app entirely.
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct NotifAppRule {
    /// Block all notifications from this app (default false).
    #[serde(default)]
    pub block: bool,
    /// Override the global timeout for this app (ms); None → use global.
    pub timeout_ms: Option<i32>,
}

/// Notification daemon configuration — mako-style.
#[derive(Deserialize, Serialize, Clone)]
pub struct Notifications {
    /// Where the popup appears: "top-right" (default) | "top-left" |
    /// "bottom-right" | "bottom-left".
    #[serde(default = "default_notif_position")]
    pub position: String,
    /// How long each popup stays on screen (ms). 0 = until dismissed.
    #[serde(default = "default_notif_timeout")]
    pub timeout_ms: i32,
    /// Max visible popup notifications at once.
    #[serde(default = "default_notif_max_visible")]
    pub max_visible: usize,
    /// Max notifications kept in history (bell panel / persistence).
    #[serde(default = "default_notif_max_history")]
    pub max_history: usize,
    /// Show popup notifications while the lockscreen is active.
    #[serde(default)]
    pub show_on_lock: bool,
    /// Per-app rules (keyed by lowercased app name).
    #[serde(default)]
    pub per_app: std::collections::HashMap<String, NotifAppRule>,
}

impl Default for Notifications {
    fn default() -> Self {
        Self {
            position: default_notif_position(),
            timeout_ms: default_notif_timeout(),
            max_visible: default_notif_max_visible(),
            max_history: default_notif_max_history(),
            show_on_lock: false,
            per_app: std::collections::HashMap::new(),
        }
    }
}

fn default_notif_position() -> String { "top-right".into() }
fn default_notif_timeout() -> i32 { 5000 }
fn default_notif_max_visible() -> usize { 5 }
fn default_notif_max_history() -> usize { 64 }

/// Parse `#rrggbb` (or `rrggbb`) into 0xAARRGGBB with full alpha.
pub fn parse_color(s: &str, default: u32) -> u32 {
    let t = s.trim_start_matches('#');
    if t.len() == 6 {
        if let Ok(v) = u32::from_str_radix(t, 16) {
            return (v << 8) | 0xff;
        }
    }
    default
}

impl Config {
    pub fn load(path: &Path) -> Self {
        match std::fs::read_to_string(path) {
            Ok(s) => toml::from_str(&s).unwrap_or_default(),
            Err(_) => Default::default(),
        }
    }

    /// Atomically persist the current config to `path` (write + rename).
    pub fn save(&self, path: &Path) {
        let Ok(s) = toml::to_string_pretty(self) else { return };
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let tmp = path.with_extension("toml.tmp");
        if std::fs::write(&tmp, &s).is_ok() {
            let _ = std::fs::rename(&tmp, path);
        }
    }

    pub fn family(&self) -> String {
        // $states/font_{s} is the per-state UI font master file. A single
        // line (e.g. "Satoshi Variable") wins over the config table, which
        // in turn wins over the built-in default.
        let ch = crate::vars::read_channel();
        let font_path = crate::vars::per_state_font_path(ch);
        if let Ok(text) = std::fs::read_to_string(&font_path) {
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }
        if !self.fonts.family.is_empty() {
            return self.fonts.family.clone();
        }
        "Inter".into()
    }

    pub fn icon_style(&self) -> crate::icons::IconStyle {
        crate::icons::IconStyle::parse(&self.fonts.icon_style)
    }

    pub fn icon_family(&self) -> String {
        if !self.fonts.icon.is_empty() {
            return self.fonts.icon.clone();
        }
        self.icon_style().family().into()
    }

    /// Display family for hero numerals (big temps, %, clock digits) — tighter
    /// tracking than the UI family, designed for large sizes. Inter Display by
    /// default; falls back to the UI family when missing.
    pub fn font_display(&self) -> String {
        if !self.fonts.display.is_empty() {
            return self.fonts.display.clone();
        }
        "Inter Display".into()
    }

    /// Mono family for technical / tabular figures (rates, sizes, sensor reads).
    /// Geist Mono by default; falls back to the UI family when missing.
    pub fn font_mono(&self) -> String {
        if !self.fonts.mono.is_empty() {
            return self.fonts.mono.clone();
        }
        "Geist Mono".into()
    }

    pub fn collapsed_w(&self) -> f32 {
        if self.bar.collapsed_width > 0 {
            self.bar.collapsed_width as f32
        } else {
            165.0
        }
    }
    pub fn expanded_w(&self) -> f32 {
        if self.bar.expanded_width > 0 {
            self.bar.expanded_width as f32
        } else {
            1120.0
        }
    }
    pub fn bar_h(&self) -> f32 {
        if self.bar.height > 0 {
            self.bar.height as f32
        } else {
            38.0
        }
    }
    /// Dashboard grid zoom. The persisted per-state config writes `0.0` as an
    /// "unset" sentinel (same as height/width) — treat it as the 1.0 default,
    /// otherwise `GridScale(0.0)` renders the whole dashboard at zero size.
    pub fn grid_scale(&self) -> f32 {
        if self.bar.grid_scale > 0.0 {
            self.bar.grid_scale
        } else {
            1.0
        }
    }

    /// Hovering the pill expands it into the dashboard (default on).
    pub fn expand_on_hover(&self) -> bool {
        self.bar.expand_on_hover.unwrap_or(true)
    }

    /// Hover / click on the resting pill opens the Control Center instead of
    /// the dashboard (default off).
    pub fn cc_as_dashboard(&self) -> bool {
        self.bar.cc_instead_of_dash.unwrap_or(false)
    }

    /// Floating pill: hovers with `floating_offset` margin over content.
    /// Off → the pill/dashboard sticks flush to the top/bottom edge.
    pub fn floating(&self) -> bool {
        self.bar.floating.unwrap_or(true)
    }

    /// Reserve screen space for the resting pill (push windows), default off.
    pub fn reserve(&self) -> bool {
        self.bar.reserve.unwrap_or(false)
    }

    /// Show the clock in the collapsed pill (default on).
    pub fn show_clock(&self) -> bool {
        self.bar.show_clock.unwrap_or(true)
    }

    /// Show the battery % in the collapsed pill (default on).
    pub fn show_battery(&self) -> bool {
        self.bar.show_battery.unwrap_or(true)
    }

    /// Show the battery percentage as text beside (outside) the icon rather
    /// than inside the battery body (default on).
    pub fn show_battery_pct(&self) -> bool {
        self.bar.show_battery_pct.unwrap_or(true)
    }

    /// Show the workspace pills in the collapsed pill (default on).
    pub fn show_ws(&self) -> bool {
        self.bar.show_ws.unwrap_or(true)
    }

    /// Show all workspace numbers (1..5) on the collapsed pill (default off).
    pub fn show_ws_long(&self) -> bool {
        self.bar.show_ws_long.unwrap_or(false)
    }

    /// Show only the active workspace number on the collapsed pill (default off).
    pub fn show_ws_short(&self) -> bool {
        self.bar.show_ws_short.unwrap_or(false)
    }

    /// Wi-Fi chip on the collapsed pill (default off).
    pub fn show_wifi(&self) -> bool {
        self.bar.show_wifi.unwrap_or(false)
    }

    /// Bluetooth chip on the collapsed pill (default off).
    pub fn show_bluetooth(&self) -> bool {
        self.bar.show_bluetooth.unwrap_or(false)
    }

    /// Volume chip on the collapsed pill (default off).
    pub fn show_volume(&self) -> bool {
        self.bar.show_volume.unwrap_or(false)
    }

    /// Wallpaper chip on the collapsed pill (default off).
    pub fn show_wallpaper(&self) -> bool {
        self.bar.show_wallpaper.unwrap_or(false)
    }

    /// Themes chip on the collapsed pill (default off).
    pub fn show_themes(&self) -> bool {
        self.bar.show_themes.unwrap_or(false)
    }

    /// Brand glyph chip on the collapsed pill (default off).
    pub fn show_brand(&self) -> bool {
        self.bar.show_brand.unwrap_or(false)
    }

    /// Show tray icons in the collapsed pill (default on).
    pub fn show_tray(&self) -> bool {
        self.bar.show_tray.unwrap_or(true)
    }

    /// Show music visualizer bars in the collapsed pill (default off).
    pub fn show_viz(&self) -> bool {
        self.bar.show_viz.unwrap_or(false)
    }

    /// Gear chip on the collapsed pill → Settings (default on).
    pub fn settings_on_pill(&self) -> bool {
        self.bar.settings_on_pill.unwrap_or(true)
    }

    /// Notification bell on the collapsed pill (default on).
    pub fn show_notif(&self) -> bool {
        self.bar.show_notif.unwrap_or(true)
    }

    /// User info chip on the collapsed pill (default off).
    pub fn show_user(&self) -> bool {
        self.bar.show_user.unwrap_or(false)
    }

    /// Battery power draw chip on the collapsed pill (default off).
    pub fn show_watts(&self) -> bool {
        self.bar.show_watts.unwrap_or(false)
    }

    /// Brightness / dark-mode chip (sun ⇄ moon) on the collapsed pill (default on).
    pub fn show_brightness(&self) -> bool {
        self.bar.show_brightness.unwrap_or(true)
    }

    /// Parsed collapsed-pill item order (left → right). Unknown names are
    /// silently skipped; missing / empty → `DEFAULT_PILL_ORDER`.
    pub fn pill_order(&self) -> Vec<PillItem> {
        self.bar
            .pill_order
            .as_ref()
            .map(|v| {
                v.iter()
                    .filter_map(|s| match s.as_str() {
                        "workspaces" => Some(PillItem::Workspaces),
                        "workspaces_long" => Some(PillItem::WorkspacesLong),
                        "workspaces_short" => Some(PillItem::WorkspacesShort),
                        "clock" => Some(PillItem::Clock),
                        "battery" => Some(PillItem::Battery),
                        "visualizer" => Some(PillItem::Visualizer),
                        "settings" => Some(PillItem::Settings),
                        "bell" => Some(PillItem::Bell),
                        "tray" => Some(PillItem::Tray),
                        "user" => Some(PillItem::User),
                        "watts" => Some(PillItem::Watts),
                        "wifi" => Some(PillItem::Wifi),
                        "bluetooth" => Some(PillItem::Bluetooth),
                        "volume" => Some(PillItem::Volume),
                        "wallpaper" => Some(PillItem::Wallpaper),
                        "themes" => Some(PillItem::Themes),
                        "branding" => Some(PillItem::Branding),
                        _ => None,
                    })
                    .collect()
            })
            .filter(|v: &Vec<PillItem>| !v.is_empty())
            .unwrap_or_else(|| crate::shell::DEFAULT_PILL_ORDER.to_vec())
    }

    /// Parsed expanded-banner strip (left → right). Entries are chips,
    /// separators (`sep:<glyph>`) or smart fillers (`filler`); unknown
    /// names and the legacy `dash_toggle` chip are silently skipped (the
    /// dashboard on/off now lives on the edit-mode strip-editor row);
    /// missing / empty → `DEFAULT_BANNER_ORDER`.
    pub fn banner_order(&self) -> Vec<BannerToken> {
        self.bar
            .banner_order
            .as_ref()
            .map(|v| {
                v.iter()
                    .filter_map(|s| BannerToken::parse(s))
                    .filter(|t| !matches!(t, BannerToken::Chip(crate::shell::BannerItem::DashToggle)))
                    .collect()
            })
            .filter(|v: &Vec<BannerToken>| !v.is_empty())
            .unwrap_or_else(|| {
                use crate::shell::BannerItem;
                // default L/C/R layout: workspaces · title · date · clock
                // hug the LEFT; app_search · tray center themselves; settings
                // · power · connectivity · wifi · bluetooth fall to the RIGHT.
                vec![
                    BannerToken::Chip(BannerItem::Workspaces),
                    BannerToken::Chip(BannerItem::Title),
                    BannerToken::Chip(BannerItem::Date),
                    BannerToken::Chip(BannerItem::Clock),
                    BannerToken::Zone(1),
                    BannerToken::Chip(BannerItem::AppSearch),
                    BannerToken::Chip(BannerItem::Tray),
                    BannerToken::Zone(2),
                    BannerToken::Chip(BannerItem::Settings),
                    BannerToken::Chip(BannerItem::Power),
                    BannerToken::Chip(BannerItem::Connectivity),
                    BannerToken::Chip(BannerItem::Wifi),
                    BannerToken::Chip(BannerItem::Bluetooth),
                ]
            })
    }

    /// Manual width (px) of the banner-only strip when the dashboard is off;
    /// 0 = auto-fit to the strip's chips.
    pub fn banner_w_px(&self) -> i32 {
        self.bar.banner_width.unwrap_or(0).max(0)
    }

    /// Extra space between the window edges and the dashboard cards.
    /// Default 12 px (the base pad resolved across zoom).
    pub fn ui_pad(&self) -> f32 {
        self.bar.ui_pad.unwrap_or(12.0).max(0.0)
    }

    /// Window (panel / dashboard surface) corner roundness in px; the
    /// slider spans 0..=20. Legacy default 24 is clamped to the range.
    pub fn win_radius(&self) -> f32 {
        self.bar.win_radius.unwrap_or(24.0).clamp(0.0, 20.0)
    }

    // NOTE: `card_radius` / `card_radius_sync` config accessors were removed —
    // card rounding is permanently LINKED to the window rounding (the config
    // keys are still parsed + re-saved as win_radius for config compatibility).

    /// Dashboard cards visible (default on). When off the expanded panel
    /// shows only the top banner strip.
    pub fn dash_enabled(&self) -> bool {
        self.bar.dash_enabled.unwrap_or(true)
    }

    /// Scroll direction toggle for dashboard cards: false = vertical (rows),
    /// true = horizontal (columns). Default false.
    pub fn dash_scroll_dir(&self) -> bool {
        self.bar.dash_scroll_dir.unwrap_or(false)
    }

    /// Pinned app ids for the AppShortcut card (persisted per channel).
    pub fn app_shortcuts(&self) -> Vec<String> {
        self.app_shortcuts.clone().unwrap_or_default()
    }

    /// Screen edge the bar strip is anchored to (`bar.anchor`: "left" /
    /// "top" / "right" / "bottom"). Default bottom keeps the legacy default.
    pub fn bar_edge(&self) -> crate::shell::BarEdge {
        self.bar
            .anchor
            .as_deref()
            .and_then(crate::shell::BarEdge::from_name)
            .unwrap_or(crate::shell::BarEdge::Bottom)
    }

    pub fn fg(&self) -> u32 {
        parse_color(&self.colors.fg, 0xcdd6f4ff)
    }
    pub fn bg(&self) -> u32 {
        parse_color(&self.colors.bg, 0x1e1e2eff)
    }
    pub fn acc(&self) -> u32 {
        parse_color(&self.colors.acc, 0x89b4faff)
    }
    pub fn hover(&self) -> u32 {
        parse_color(&self.colors.hover, 0x313244ff)
    }

    /// Horizontal padding inside the collapsed pill (default 18 px).
    pub fn pill_margin_x(&self) -> f32 {
        self.bar.pill_margin_x.unwrap_or(18.0).max(0.0)
    }

    /// Background alpha of the resting pill (0=transparent … 255=opaque).
    pub fn pill_alpha(&self) -> u8 {
        self.bar.pill_alpha.unwrap_or(0xb0)
    }

    /// Background alpha of dashboard cards (0=transparent … 255=opaque).
    pub fn dash_card_alpha(&self) -> u8 {
        self.bar.dash_card_alpha.unwrap_or(0xcc)
    }

    /// Hold-to-confirm delay for the power menu and lockscreen (s).
    /// Default 1.5, clamped to 0.5..=3.0.
    pub fn power_hold_delay(&self) -> f32 {
        self.bar.power_hold_delay.unwrap_or(1.5).clamp(0.5, 3.0)
    }

    /// Allow the main mixer volume to exceed 100% (up to 200%). Default on.
    pub fn allow_over_100(&self) -> bool {
        self.bar.allow_over_100.unwrap_or(true)
    }

    /// Max volume cap in percent (0..200). Default 100.
    pub fn max_vol(&self) -> i32 {
        self.bar.max_vol.unwrap_or(100).clamp(0, 200)
    }

    /// Wallpaper thumbnail directory (expanded `~` / `$HOME`); unset →
    /// defaults to `$HOME/Wallpapers`.
    /// Picker thumbnail size (px); unset → 96. Clamped to sane bounds.
    pub fn wallpaper_cell(&self) -> f32 {
        self.wallpaper
            .cell
            .unwrap_or(96.0)
            .clamp(56.0, 160.0)
    }

    pub fn wallpaper_dir(&self) -> std::path::PathBuf {
        let d = self.wallpaper.dir.trim();
        let d = if d.is_empty() { "~/Wallpapers" } else { d };
        std::path::PathBuf::from(shellexpand(d))
    }

    /// Wallpaper set command template; `{path}` / `{monitor}` are substituted.
    pub fn wallpaper_command(&self) -> String {
        let c = self.wallpaper.command.trim();
        if c.is_empty() {
            "set_wall {path}".to_string()
        } else {
            c.to_string()
        }
    }

    /// Power backend ("sysfs" | "upower").
    pub fn power_backend(&self) -> String {
        let b = self.backends.power.trim();
        if b.is_empty() {
            "sysfs".to_string()
        } else {
            b.to_string()
        }
    }

    /// Brightness backend ("sysfs" today).
    pub fn brightness_backend(&self) -> String {
        let b = self.backends.brightness.trim();
        if b.is_empty() {
            "sysfs".to_string()
        } else {
            b.to_string()
        }
    }

    /// Audio backend ("wpctl" today).
    pub fn audio_backend(&self) -> String {
        let b = self.backends.audio.trim();
        if b.is_empty() {
            "wpctl".to_string()
        } else {
            b.to_string()
        }
    }

    /// Lockscreen backend ("native" | "hyprlock").
    pub fn lockscreen_backend(&self) -> String {
        let b = self.backends.lockscreen.trim();
        if b.is_empty() {
            "native".to_string()
        } else {
            b.to_string()
        }
    }

    /// Check if a notification from `app` should be blocked by per-app rules.
    pub fn notif_blocked(&self, app: &str) -> bool {
        self.notifications
            .per_app
            .get(app.to_lowercase().as_str())
            .map(|r| r.block)
            .unwrap_or(false)
    }

    /// Popup position parsed as (x_anchor, y_anchor).
    /// Returns (true, true) for top-right (default).
    pub fn notif_position(&self) -> (bool, bool) {
        match self.notifications.position.as_str() {
            "top-left" => (false, true),
            "bottom-right" => (true, false),
            "bottom-left" => (false, false),
            _ => (true, true), // "top-right"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The auto-created default config must parse into a working Config.
    #[test]
    fn default_template_parses() {
        let cfg = toml::from_str::<Config>(DEFAULT_SHELL_TOML).expect("default template must parse");
        assert_eq!(cfg.lockscreen_backend(), "native");
        assert!(!cfg.family().is_empty());
        assert_eq!(cfg.bar_h(), 38.0);
        // The shipped template points at $HOME/Wallpapers.
        let d = cfg.wallpaper_dir();
        if let Ok(home) = std::env::var("HOME") {
            assert_eq!(d, std::path::PathBuf::from(home).join("Wallpapers"));
        } else {
            assert_eq!(d, std::path::PathBuf::from("~/Wallpapers"));
        }
    }

    /// Unset `[wallpaper] dir` falls back to `$HOME/Wallpapers`.
    #[test]
    fn wallpaper_dir_defaults_to_home() {
        let cfg = Config::default();
        let d = cfg.wallpaper_dir();
        if let Ok(home) = std::env::var("HOME") {
            assert_eq!(d, std::path::PathBuf::from(home).join("Wallpapers"));
        } else {
            assert_eq!(d, std::path::PathBuf::from("~/Wallpapers"));
        }
    }

    /// Dashboard card memory (`dash_cards` + `dash_tray`) round-trips through
    /// the per-state TOML exactly — sizes and positions are preserved.
    #[test]
    fn dashboard_cards_roundtrip() {
        let mut cfg = Config::default();
        cfg.dash_cards = vec![
            DashCardEntry { card: "system".into(), x: 0, y: 0, w: 6, h: 4 },
            DashCardEntry { card: "weather".into(), x: 6, y: 0, w: 6, h: 4 },
            DashCardEntry { card: "media".into(), x: 12, y: 0, w: 8, h: 4 },
        ];
        cfg.dash_tray = vec!["docker".into(), "sshvpn".into()];

        let toml = toml::to_string_pretty(&cfg).expect("serialize");
        let back = toml::from_str::<Config>(&toml).expect("parse back");
        assert_eq!(back.dash_cards.len(), 3);
        assert_eq!(back.dash_cards[0].card, "system");
        assert_eq!(back.dash_cards[0].x, 0);
        assert_eq!(back.dash_cards[0].w, 6);
        assert_eq!(back.dash_cards[2].card, "media");
        assert_eq!(back.dash_cards[2].x, 12);
        assert_eq!(back.dash_cards[2].y, 0);
        assert_eq!(back.dash_cards[2].w, 8);
        assert_eq!(back.dash_cards[2].h, 4);
        assert_eq!(back.dash_tray, vec!["docker".to_string(), "sshvpn".to_string()]);
        // cards absent from a default config stay empty (older files parse fine)
        assert!(Config::default().dash_cards.is_empty());
    }
}

/// Expand a leading `~` (or `$HOME`) in a path string.
fn shellexpand(s: &str) -> String {
    if let Some(rest) = s.strip_prefix("~").or_else(|| s.strip_prefix("$HOME")) {
        if let Ok(home) = std::env::var("HOME") {
            let rest = rest.strip_prefix('/').unwrap_or(rest);
            return format!("{home}/{rest}");
        }
    }
    s.to_string()
}
