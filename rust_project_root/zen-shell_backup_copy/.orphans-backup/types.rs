//! All enums, constants, data structs, and free helper functions
//! previously scattered through the top of `mod.rs`.

use crate::config::Config;
use crate::ui::{self, mix, Pal};
use std::time::Instant;

// ── PillItem ───────────────────────────────────────────────────────────────

/// A collapsible pill-item identifier — used for the drag-reorder config.
/// Each variant maps to one visible element on the collapsed pill.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PillItem {
    Workspaces,
    Clock,
    Battery,
    /// battery power draw in watts (refreshes once a minute)
    Watts,
    Visualizer,
    Settings,
    Bell,
    Tray,
    User,
}

/// Default pill layout order (left → right). Items not present are skipped.
impl Default for PillItem {
    fn default() -> Self {
        PillItem::Workspaces
    }
}

pub const DEFAULT_PILL_ORDER: &[PillItem] = &[
    PillItem::Workspaces,
    PillItem::Clock,
    PillItem::Battery,
    PillItem::Watts,
    PillItem::Visualizer,
    PillItem::Settings,
    PillItem::Bell,
    PillItem::Tray,
    PillItem::User,
];

// ── BannerItem ─────────────────────────────────────────────────────────────

/// One chip on the expanded-dashboard TOP BANNER (reorderable / removable
/// in edit mode). Mirrors the collapsed-pill items: a configurable strip
/// of chips that persists across sessions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BannerItem {
    /// workspace number
    Workspaces,
    /// focused window title
    Title,
    /// date string ("%a %d")
    Date,
    /// clock ("%H:%M")
    Clock,
    /// app launcher chip (opens "Search apps")
    AppSearch,
    /// SNI tray icons
    Tray,
    /// Wi-Fi status → opens the network menu
    Wifi,
    /// Bluetooth status → opens the devices menu
    Bluetooth,
    /// aggregate connectivity chip ("net") → small popover with
    /// Wi-Fi / Bluetooth menu shortcuts
    Connectivity,
    /// battery level (%)
    Battery,
    /// outdoor temperature (live from the weather service)
    Weather,
    /// CPU load percent
    Cpu,
    /// memory usage percent
    Ram,
    /// live network throughput (↓/↑)
    NetSpeed,
    /// do-not-disturb bell toggle
    Dnd,
    /// system volume
    Volume,
    /// screen brightness
    Brightness,
    /// VPN / tunnel status
    Vpn,
    /// settings gear → Settings
    Settings,
    /// power icon → Power menu
    Power,
    /// dashboard on/off toggle — rendered ONLY in edit mode
    DashToggle,
}

impl Default for BannerItem {
    fn default() -> Self {
        BannerItem::Workspaces
    }
}

/// Display name of a banner chip (config names + parked-tray labels).
impl BannerItem {
    pub fn name(&self) -> &'static str {
        match self {
            BannerItem::Workspaces => "workspaces",
            BannerItem::Title => "title",
            BannerItem::Date => "date",
            BannerItem::Clock => "clock",
            BannerItem::AppSearch => "app_search",
            BannerItem::Tray => "tray",
            BannerItem::Wifi => "wifi",
            BannerItem::Bluetooth => "bluetooth",
            BannerItem::Connectivity => "connectivity",
            BannerItem::Battery => "battery",
            BannerItem::Weather => "weather",
            BannerItem::Cpu => "cpu",
            BannerItem::Ram => "ram",
            BannerItem::NetSpeed => "net_speed",
            BannerItem::Dnd => "dnd",
            BannerItem::Volume => "volume",
            BannerItem::Brightness => "brightness",
            BannerItem::Vpn => "vpn",
            BannerItem::Settings => "settings",
            BannerItem::Power => "power",
            BannerItem::DashToggle => "dash_toggle",
        }
    }
}

/// Default banner layout left → right. The dashboard on/off lives on the
/// strip-editor row in edit mode, so no dedicated toggle chip is placed here.
/// Clock / date / app-search are parked here too, so they can be dragged onto
/// the strip from the edit-mode parked tray.
pub const DEFAULT_BANNER_ORDER: &[BannerItem] = &[
    BannerItem::Workspaces,
    BannerItem::Title,
    BannerItem::Date,
    BannerItem::Clock,
    BannerItem::AppSearch,
    BannerItem::Tray,
    BannerItem::Settings,
    BannerItem::Power,
    BannerItem::Wifi,
    BannerItem::Bluetooth,
    BannerItem::Connectivity,
    BannerItem::Battery,
    BannerItem::Weather,
    BannerItem::Cpu,
    BannerItem::Ram,
    BannerItem::NetSpeed,
    BannerItem::Dnd,
    BannerItem::Volume,
    BannerItem::Brightness,
    BannerItem::Vpn,
];

// ── BannerToken ────────────────────────────────────────────────────────────

/// One slot on the expanded top-banner strip. The strip is an ordered list
/// of these: chips show live info, a normal separator draws a small glyph,
/// and a smart filler stretches to push whatever sits left of it to the left
/// edge and whatever sits right of it to the right edge (several fillers
/// split the leftover space evenly).
#[derive(Clone, Debug, PartialEq)]
pub enum BannerToken {
    Chip(BannerItem),
    Sep(String),
    Filler,
    /// An L/C/R section marker (0 left, 1 center, 2 right). Everything after
    /// it until the next marker belongs to that section. Draws nothing —
    /// it only steers `banner_cells` into the three aligned zones.
    Zone(u8),
}

/// Separator glyph presets offered by the edit-mode strip editor row. Add
/// more to grow the picker — each is inserted as its own `sep:<glyph>` slot.
pub const BANNER_SEP_PRESETS: &[&str] = &["|", ":", ": |", "·"];

impl BannerToken {
    /// Parse a persisted strip entry: chip name, `sep:<glyph>`, `filler`,
    /// `zone:0|1|2`.
    pub fn parse(s: &str) -> Option<BannerToken> {
        if s == "filler" {
            return Some(BannerToken::Filler);
        }
        if let Some(z) = s.strip_prefix("zone:") {
            let n: u8 = z.parse().ok()?;
            return Some(BannerToken::Zone(n.min(2)));
        }
        if let Some(g) = s.strip_prefix("sep:") {
            if !g.is_empty() {
                return Some(BannerToken::Sep(g.to_string()));
            }
            return None;
        }
        let chip = match s {
            "workspaces" => BannerItem::Workspaces,
            "title" => BannerItem::Title,
            "date" => BannerItem::Date,
            "clock" => BannerItem::Clock,
            "app_search" => BannerItem::AppSearch,
            "tray" => BannerItem::Tray,
            "wifi" => BannerItem::Wifi,
            "bluetooth" => BannerItem::Bluetooth,
            "connectivity" => BannerItem::Connectivity,
            "battery" => BannerItem::Battery,
            "weather" => BannerItem::Weather,
            "cpu" => BannerItem::Cpu,
            "ram" => BannerItem::Ram,
            "net_speed" => BannerItem::NetSpeed,
            "dnd" => BannerItem::Dnd,
            "volume" => BannerItem::Volume,
            "brightness" => BannerItem::Brightness,
            "vpn" => BannerItem::Vpn,
            "settings" => BannerItem::Settings,
            "power" => BannerItem::Power,
            "dash_toggle" => BannerItem::DashToggle,
            _ => return None,
        };
        Some(BannerToken::Chip(chip))
    }

    /// Persist this slot back to the config string list.
    pub fn to_str(&self) -> String {
        match self {
            BannerToken::Chip(c) => c.name().to_string(),
            BannerToken::Sep(g) => format!("sep:{g}"),
            BannerToken::Filler => "filler".to_string(),
            BannerToken::Zone(z) => format!("zone:{}", *z.min(&2)),
        }
    }

    /// The chip inside this slot (None for separators / fillers / zones).
    pub fn chip(&self) -> Option<BannerItem> {
        match self {
            BannerToken::Chip(c) => Some(*c),
            _ => None,
        }
    }

    /// Which L/C/R section this token starts (None otherwise).
    pub fn as_zone(&self) -> Option<u8> {
        match self {
            BannerToken::Zone(z) => Some(*z),
            _ => None,
        }
    }

    /// Short label used by the drag ghost / tray tools.
    pub fn label(&self) -> String {
        match self {
            BannerToken::Chip(c) => c.name().to_string(),
            BannerToken::Sep(g) => g.clone(),
            BannerToken::Filler => "spacer".to_string(),
            BannerToken::Zone(z) => match z {
                1 => "center".to_string(),
                2 => "right".to_string(),
                _ => "left".to_string(),
            },
        }
    }
}

// ── Small data structs ─────────────────────────────────────────────────────

/// Uniform gap between collapsed-pill modules (px) — shared by the width
/// estimator (`collapsed_w`) and the drawer (`layout_collapsed`) so the pill is
/// sized exactly to what it draws.
pub const PILL_GAP: f32 = 6.0;

/// One custom accent from `$states2/custom_acc.json`: `{ name, hex }` where
/// `hex` is the accent color (lowercase, no '#').
#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
pub struct CustomAcc {
    #[allow(dead_code)]
    pub name: String,
    #[serde(default)]
    pub hex: String,
}

#[cfg(test)]
mod custom_acc_tests {
    use super::*;

    #[test]
    fn parses_custom_acc_json() {
        // shape emitted by gen_launcher_cache → $states2/custom_acc.json:
        // [{name,hex}] where hex is a lowercase hex string (no '#').
        let raw = r#"[{"name":"blue","hex":"89b4fa"},{"name":"mauve","hex":"cba6f7"}]"#;
        let list: Vec<CustomAcc> = serde_json::from_str(raw).unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].name, "blue");
        assert_eq!(list[0].hex, "89b4fa");
        assert_eq!(list[1].name, "mauve");
    }
}

/// Width of the Settings app's left navigation sidebar (two-pane layout).
/// The content pane starts at this x and runs to the panel's right edge.
pub const SETTINGS_SIDEBAR: f32 = 180.0;
/// Height added to the Settings surface while the Pill section shows its
/// "expand on hover" info tip (kept in sync with the band in settings.rs).
pub const SETTINGS_EXPAND_TIP_H: f32 = 44.0;

// ── Category tabs for Settings ─────────────────────────────────────────────

/// Category tabs for the two-pane Settings app. The left sidebar lists these;
/// the right pane renders the selected category's settings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsTab {
    Network,
    Sound,
    Appearance,
    Pill,
    System,
    Misc,
}
impl Default for SettingsTab {
    fn default() -> Self {
        SettingsTab::Network
    }
}

// ── Drag state structs ─────────────────────────────────────────────────────

/// Drag state for collapsed-pill item reordering.
pub struct PillDrag {
    /// the item currently being dragged
    pub item: PillItem,
    /// original slot index (for snapping back on cancel)
    pub from_idx: usize,
    /// pointer x in surface-local coords (updated on motion)
    pub x: f32,
    /// pointer y in surface-local coords (used for settings vertical drag)
    pub y: f32,
    /// pointer position where the drag began — gates the follow-cursor
    /// lift until the pointer actually moves (tap vs. drag)
    pub sx: f32,
    pub sy: f32,
    /// true when at least one swap happened during this drag
    pub swapped: bool,
}

/// Drag state for top-banner strip reordering (edit mode, single row).
#[derive(Clone)]
pub struct BannerDrag {
    /// the strip slot being dragged (chip, separator or filler)
    pub item: BannerToken,
    /// index in `banner_order` where the drag began
    pub from_idx: usize,
    /// true when the drag started from the parked banner tray (drag-in)
    pub from_tray: bool,
    /// live insertion slot while dragging in from the tray (None outside a
    /// drag-in, or before the pointer crossed the banner)
    pub over_idx: Option<usize>,
    /// pointer position (surface-local, updated on motion)
    pub x: f32,
    pub y: f32,
    /// where the drag began — gates lift (tap vs. drag)
    pub sx: f32,
    pub sy: f32,
    /// true when at least one swap happened
    pub swapped: bool,
}

// ── Geometry constants ─────────────────────────────────────────────────────

pub const EXPANDED_H: f32 = 480.0;

// ── corner notification popup ────────────────────────────────────────────
pub const NOTIF_POPUP_W: i32 = 470;
pub const NOTIF_PAD: f32 = 12.0;
pub const NOTIF_GAP: f32 = 10.0;
pub const NOTIF_CARD_H: f32 = 92.0;
pub const NOTIF_CARD_H_ACTIONS: f32 = 122.0;

// ── Dashboard metro grid ────────────────────────────────────────────────
pub const DASH_COLS: u16 = 20;
pub const DASH_ROWS: u16 = 12;
pub(crate) const BASE_DASH_GAP: f32 = 6.0;
pub(crate) const BASE_DASH_TOP: f32 = 48.0;
pub(crate) const BASE_TRAY_STRIP_H: f32 = 120.0;
pub(crate) const BASE_TRAY_GAP: f32 = 18.0;
pub(crate) const BASE_BANNER_Y: f32 = 12.0;
pub(crate) const BASE_BANNER_H: f32 = 26.0;
const DASH_MAX_ROWS: u16 = 4096;

/// Uniform zoom factor for the dashboard grid.
#[derive(Clone, Copy)]
pub struct GridScale(pub f32);
impl GridScale {
    pub fn s(self, px: f32) -> f32 { px * self.0 }
    /// Scale a font size — round to 1 decimal to avoid subpixel jitter.
    pub fn fs(self, size: f32) -> f32 { (size * self.0 * 10.0).round() / 10.0 }
}

// ── DashCard ───────────────────────────────────────────────────────────────

/// Logical cards — every dashboard card is one of these.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum DashCard {
    System,
    Weather,
    Media,
    Network,
    CpuGpu,
    /// CPU-only card: load avg, live clock speed, package temp, usage %.
    Cpu,
    /// Copied-image history: thumbnail + name list, click copies the image.
    ClipImg,
    Gauges,
    Todo,
    Toggles,
    Sliders,
    Disk,
    ProcMon,
    Mem,
    TopProc,
    ActiveWin,
    Clipboard,
    Notes,
    PkgUpdates,
    SshVpn,
    Docker,
    KbLayout,
    /// Scrollable wallpaper thumbnail grid — lives on the dashboard grid
    /// like any other card (thumbnail click → set_wall, wheel → scroll).
    Wallpaper,
    /// RSS/JSON headline feed (fetched by curl in the background).
    News,
    /// Mini calendar month — click a day opens the full calendar panel.
    Calendar,
    /// Accent source picker (from wallpaper / scheme default / custom accent /
    /// defined hex) — mirrors the Settings → Appearance block.
    Accent,
    /// Music visualizer bars (cava-fed) + bar-style toggle.
    Viz,
    /// Live sensor readout (accelerometer pitch/roll; gyro + compass when
    /// the hardware exposes them).
    Sensors,
    /// Front-camera mirror feed (ffmpeg MJPEG → decoded thumb).
    Mirror,
    /// Power actions (lock / logout / sleep / restart / shutdown) as tiles —
    /// danger actions use hold-to-confirm like the power menu. Vertical
    /// variant: the actions stack as a narrow column of rows.
    PowerV,
    /// Power actions laid out as a wide row of tiles (fit-to-card).
    PowerH,
    /// Disk I/O read/write speed — dual-line history graph (read = blue,
    /// write = accent), normalised against the read peak as 100%.
    DiskIo,
    /// Lyrics of the currently playing track (LRCLib + local cache);
    /// synced lines highlight/auto-scroll with the playhead.
    Lyrics,
    /// Nearby Wi-Fi networks: signal bars, lock/connected badges; row click
    /// connects (open networks instantly, saved-secured via the OS agent).
    Wifi,
    /// Bluetooth controller + device list; row click connects / disconnects.
    Bluetooth,
    /// On-demand bandwidth test (download + upload + ping, Cloudflare edge).
    SpeedTest,
    /// Recently opened files (xdg `recently-used.xbel`); row click opens.
    Recent,
    /// Currency converter (built-in static reference rates, click to re-base).
    Currency,
    /// Quote of the day with save-to-$states + refresh buttons.
    Quote,
    /// Crypto/stock price ticker rows (numeric, like News).
    Ticker,
    /// Dedicated GPU card: util % + temp + VRAM used/total (amdgpu sysfs or
    /// nvidia-smi when present; otherwise shows util only).
    Gpu,
    /// All hwmon thermal zones in a compact grid (°C, per-zone highlight).
    Thermal,
    /// Clean clock + date only card (divider-friendly: nothing but the time,
    /// the date, and an optional seconds readout).
    Clock,
    /// Battery card with a VERTICAL vector battery icon.
    BatteryV,
    /// Battery card with a HORIZONTAL vector battery icon (same panes).
    BatteryH,
    /// 10-band parametric equalizer with frequency response curve and presets.
    Eq,
    /// User-pinned app launcher card.
    AppShortcut,
}

impl DashCard {
    pub fn id(self) -> &'static str {
        match self {
            DashCard::System => "system",
            DashCard::Weather => "weather",
            DashCard::Media => "media",
            DashCard::Network => "network",
            DashCard::CpuGpu => "cpugpu",
            DashCard::Cpu => "cpu",
            DashCard::ClipImg => "clipimg",
            DashCard::Gauges => "gauges",
            DashCard::Todo => "todo",
            DashCard::Toggles => "toggles",
            DashCard::Sliders => "sliders",
            DashCard::Disk => "disk",
            DashCard::ProcMon => "procmon",
            DashCard::Mem => "mem",
            DashCard::TopProc => "topproc",
            DashCard::ActiveWin => "activewin",
            DashCard::Clipboard => "clipboard",
            DashCard::Notes => "notes",
            DashCard::PkgUpdates => "pkgupdates",
            DashCard::SshVpn => "sshvpn",
            DashCard::Docker => "docker",
            DashCard::KbLayout => "kblayout",
            DashCard::Wallpaper => "wallpaper",
            DashCard::News => "news",
            DashCard::Calendar => "calendar",
            DashCard::Accent => "accent",
            DashCard::Viz => "viz",
            DashCard::Sensors => "sensors",
            DashCard::Mirror => "mirror",
            DashCard::PowerV => "powerv",
            DashCard::PowerH => "powerh",
            DashCard::DiskIo => "diskio",
            DashCard::Lyrics => "lyrics",
            DashCard::Wifi => "wifi",
            DashCard::Bluetooth => "bluetooth",
            DashCard::SpeedTest => "speedtest",
            DashCard::Recent => "recent",
            DashCard::Currency => "currency",
            DashCard::Quote => "quote",
            DashCard::Ticker => "ticker",
            DashCard::Gpu => "gpu",
            DashCard::Thermal => "thermal",
            DashCard::Clock => "clock",
            DashCard::BatteryV => "batteryv",
            DashCard::BatteryH => "batteryh",
            DashCard::Eq => "eq",
            DashCard::AppShortcut => "appshortcut",
        }
    }
    pub(crate) fn from_id(s: &str) -> Option<Self> {
        Some(match s {
            "system" => DashCard::System,
            "weather" => DashCard::Weather,
            "media" => DashCard::Media,
            "network" => DashCard::Network,
            "cpugpu" => DashCard::CpuGpu,
            "cpu" => DashCard::Cpu,
            "clipimg" => DashCard::ClipImg,
            "gauges" => DashCard::Gauges,
            "todo" => DashCard::Todo,
            "toggles" => DashCard::Toggles,
            "sliders" => DashCard::Sliders,
            "disk" => DashCard::Disk,
            "procmon" => DashCard::ProcMon,
            "mem" => DashCard::Mem,
            "topproc" => DashCard::TopProc,
            "activewin" => DashCard::ActiveWin,
            "clipboard" => DashCard::Clipboard,
            "notes" => DashCard::Notes,
            "pkgupdates" => DashCard::PkgUpdates,
            "sshvpn" => DashCard::SshVpn,
            "docker" => DashCard::Docker,
            "kblayout" => DashCard::KbLayout,
            "wallpaper" => DashCard::Wallpaper,
            "news" => DashCard::News,
            "calendar" => DashCard::Calendar,
            "accent" => DashCard::Accent,
            "viz" => DashCard::Viz,
            "sensors" => DashCard::Sensors,
            "mirror" => DashCard::Mirror,
            "powerv" => DashCard::PowerV,
            "powerh" => DashCard::PowerH,
            "diskio" => DashCard::DiskIo,
            "lyrics" => DashCard::Lyrics,
            "wifi" => DashCard::Wifi,
            "bluetooth" => DashCard::Bluetooth,
            "speedtest" => DashCard::SpeedTest,
            "recent" => DashCard::Recent,
            "currency" => DashCard::Currency,
            "quote" => DashCard::Quote,
            "ticker" => DashCard::Ticker,
            "gpu" => DashCard::Gpu,
            "thermal" => DashCard::Thermal,
            "clock" => DashCard::Clock,
            "batteryv" => DashCard::BatteryV,
            "batteryh" => DashCard::BatteryH,
            "eq" => DashCard::Eq,
            "appshortcut" => DashCard::AppShortcut,
            _ => return None,
        })
    }

    /// Dispatch to the card's drawer function.
    pub(crate) fn draw(&self, shell: &mut super::Shell, v: &mut Vec<super::Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        match self {
            DashCard::System => shell.draw_system_card(v, x, y, w, h, pal),
            DashCard::Weather => shell.draw_weather_card(v, x, y, w, h, pal),
            DashCard::Media => shell.draw_media_card(v, x, y, w, h, pal),
            DashCard::Network => shell.draw_network_card(v, x, y, w, h, pal),
            DashCard::CpuGpu => shell.draw_cpugpu_card(v, x, y, w, h, pal),
            DashCard::Cpu => shell.draw_cpu_card(v, x, y, w, h, pal),
            DashCard::ClipImg => shell.draw_clipimg_card(v, x, y, w, h, pal),
            DashCard::Gauges => shell.draw_gauges_card(v, x, y, w, h, pal),
            DashCard::Todo => shell.draw_todo_card(v, x, y, w, h, pal),
            DashCard::Toggles => shell.draw_toggles_card(v, x, y, w, h, pal),
            DashCard::Sliders => shell.draw_sliders_card(v, x, y, w, h, pal),
            DashCard::Disk => shell.draw_disk_card(v, x, y, w, h, pal),
            DashCard::ProcMon => shell.draw_procmon_card(v, x, y, w, h, pal),
            DashCard::Mem => shell.draw_mem_card(v, x, y, w, h, pal),
            DashCard::TopProc => shell.draw_topproc_card(v, x, y, w, h, pal),
            DashCard::ActiveWin => shell.draw_activewin_card(v, x, y, w, h, pal),
            DashCard::Clipboard => shell.draw_clipboard_card(v, x, y, w, h, pal),
            DashCard::Notes => shell.draw_notes_card(v, x, y, w, h, pal),
            DashCard::PkgUpdates => shell.draw_pkgupdates_card(v, x, y, w, h, pal),
            DashCard::SshVpn => shell.draw_sshvpn_card(v, x, y, w, h, pal),
            DashCard::Docker => shell.draw_docker_card(v, x, y, w, h, pal),
            DashCard::KbLayout => shell.draw_kblayout_card(v, x, y, w, h, pal),
            DashCard::Wallpaper => shell.draw_wallpaper_card(v, x, y, w, h, pal),
            DashCard::News => shell.draw_news_card(v, x, y, w, h, pal),
            DashCard::Calendar => shell.draw_calendar_card(v, x, y, w, h, pal),
            DashCard::Accent => shell.draw_accent_card(v, x, y, w, h, pal),
            DashCard::Viz => shell.draw_viz_card(v, x, y, w, h, pal),
            DashCard::Sensors => shell.draw_sensors_card(v, x, y, w, h, pal),
            DashCard::Mirror => shell.draw_mirror_card(v, x, y, w, h, pal),
            DashCard::PowerV => shell.draw_power_v_card(v, x, y, w, h, pal),
            DashCard::PowerH => shell.draw_power_h_card(v, x, y, w, h, pal),
            DashCard::DiskIo => shell.draw_diskio_card(v, x, y, w, h, pal),
            DashCard::Lyrics => shell.draw_lyrics_card(v, x, y, w, h, pal),
            DashCard::Wifi => shell.draw_wifi_card(v, x, y, w, h, pal),
            DashCard::Bluetooth => shell.draw_bt_card(v, x, y, w, h, pal),
            DashCard::SpeedTest => shell.draw_speedtest_card(v, x, y, w, h, pal),
            DashCard::Recent => shell.draw_recent_card(v, x, y, w, h, pal),
            DashCard::Currency => shell.draw_currency_card(v, x, y, w, h, pal),
            DashCard::Quote => shell.draw_quote_card(v, x, y, w, h, pal),
            DashCard::Ticker => shell.draw_ticker_card(v, x, y, w, h, pal),
            DashCard::Gpu => shell.draw_gpu_card(v, x, y, w, h, pal),
            DashCard::Thermal => shell.draw_thermal_card(v, x, y, w, h, pal),
            DashCard::Clock => shell.draw_clock_card(v, x, y, w, h, pal),
            DashCard::BatteryV => shell.draw_battery_v_card(v, x, y, w, h, pal),
            DashCard::BatteryH => shell.draw_battery_h_card(v, x, y, w, h, pal),
            DashCard::Eq => shell.draw_eq_card(v, x, y, w, h, pal),
            DashCard::AppShortcut => shell.draw_apps_card(v, x, y, w, h, pal),
        }
    }
}

/// A card's placement on the grid (in cells).
#[derive(Clone, Copy, Debug)]
pub struct CardLayout {
    pub card: DashCard,
    pub x: u16,
    pub y: u16,
    pub w: u16,
    pub h: u16,
}

// ── All pub(crate) key-range constants ──────────────────────────────────────

/// Base key for custom-accent grid circle hit-regions (Settings → Appearance).
pub const CUSTOM_ACC_KEY_BASE: u32 = 100_000;

pub(crate) const WP_CARD_KEY_BASE: u32 = 2000;
pub(crate) const WP_CARD_KEY_MAX: u32 = 9999;

pub(crate) const CAL_KEY_BASE: u32 = 30_000;
pub(crate) const CAL_KEY_BASE_PREV: u32 = 30_100;
pub(crate) const CAL_KEY_BASE_NEXT: u32 = 30_101;
pub(crate) const CAL_KEY_BASE_MAX: u32 = CAL_KEY_BASE + 41;

pub(crate) const ACC_KEY_BASE: u32 = 32_000;
pub(crate) const ACC_KEY_BASE_MAX: u32 = ACC_KEY_BASE + 3;
pub(crate) const ACC_LIST_BACK_KEY: u32 = 32_100;
pub(crate) const ACC_LIST_CHEV_KEY: u32 = 32_101;
pub(crate) const ACC_LIST_KEY_BASE: u32 = 32_200;
pub(crate) const ACC_LIST_KEY_MAX: u32 = ACC_LIST_KEY_BASE + 99;

pub(crate) const NOTES_KEY_INPUT: u32 = 32_120;
pub(crate) const BATTERY_PSAVE_KEY: u32 = 32_130;
pub(crate) const VIZ_KEY_BASE: u32 = 33_000;
pub(crate) const LYRICS_KEY_BASE: u32 = 33_500;

pub(crate) const WIFI_KEY_BASE: u32 = 14_000;
pub(crate) const BT_KEY_BASE: u32 = 14_100;
pub(crate) const SPEED_KEY_BASE: u32 = 14_200;
pub(crate) const RECENT_KEY_BASE: u32 = 14_300;
pub(crate) const CURRENCY_KEY_BASE: u32 = 14_400;
pub(crate) const QUOTE_KEY_BASE: u32 = 14_500;
pub(crate) const QUOTE_SAVE_KEY: u32 = 14_501;
pub(crate) const TICKER_KEY_BASE: u32 = 14_600;

/// Speed-test probe result from the worker thread.
pub enum SpeedMsg {
    /// down Mbps, up Mbps, ping ms
    Done { down_mbps: f32, up_mbps: f32, ping_ms: u32 },
}

/// Quote fetch result from the worker thread.
pub enum QuoteMsg {
    Quote { text: String, author: String },
    Failed,
}

/// Ticker fetch result from the worker thread.
pub enum TickerMsg {
    Items { rows: Vec<(String, String, f32)> },
}

pub(crate) const NEWS_CAT_KEY_BASE: u32 = 13_000;
pub(crate) const NEWS_CAT_KEY_MAX: u32 = NEWS_CAT_KEY_BASE + 199;
pub(crate) const NEWS_HEAD_KEY_BASE: u32 = 13_300;
pub(crate) const NEWS_HEAD_KEY_MAX: u32 = NEWS_HEAD_KEY_BASE + 199;

pub(crate) const POWER_CARD_KEY_BASE: u32 = 12_000;
pub(crate) const POWER_CARD_KEY_MAX: u32 = POWER_CARD_KEY_BASE + 4;

pub(crate) const CLIPIMG_KEY_BASE: u32 = 140;
pub(crate) const CLIPIMG_KEY_MAX: u32 = CLIPIMG_KEY_BASE + 19;

pub(crate) const CLIP_KEY_BASE: u32 = 80;
pub(crate) const CLIP_KEY_MAX: u32 = CLIP_KEY_BASE + 19;

pub(crate) const WP_CARD_SB_UP: u32 = 10_000;
pub(crate) const WP_CARD_SB_DOWN: u32 = 10_001;

pub(crate) const BANNER_KEY_BASE: u32 = 34_000;
pub(crate) const BANNER_X_KEY_BASE: u32 = 34_100;
pub(crate) const BANNER_TRAY_KEY_BASE: u32 = 34_200;
pub(crate) const BANNER_CTRL_KEY_BASE: u32 = 34_300;
pub(crate) const BANNER_W_MINUS_KEY: u32 = 34_330;
pub(crate) const BANNER_W_PLUS_KEY: u32 = 34_331;
pub(crate) const BANNER_W_AUTO_KEY: u32 = 34_332;
pub(crate) const BANNER_W_SLIDER_KEY: u32 = 34_333;
pub(crate) const CONN_WIFI_KEY: u32 = 34_360;
pub(crate) const CONN_BT_KEY: u32 = 34_361;
pub(crate) const UI_PAD_SLIDER_KEY: u32 = 34_380;
pub(crate) const WIN_RAD_SLIDER_KEY: u32 = 34_381;
pub(crate) const CARD_RAD_SLIDER_KEY: u32 = 34_382;
pub(crate) const CARD_RAD_SYNC_KEY: u32 = 34_383;
pub(crate) const BANNER_TRAY_VROWS: usize = 2;
pub(crate) const DASH_TRAY_VROWS: usize = 3;
pub(crate) const DASH_SCROLL_DIR_KEY: u32 = 34_340;
pub(crate) const DASH_CLEAR_ALL_KEY: u32 = 34_341;
pub(crate) const BANNER_CLEAR_ALL_KEY: u32 = 34_350;
pub(crate) const DASH_CARD_KEY: u32 = 34_480;
pub(crate) const APP_SHORTCUT_KEY_BASE: u32 = 34_400;
pub(crate) const APP_SHORTCUT_KEY_MAX: u32 = APP_SHORTCUT_KEY_BASE + 7;
pub(crate) const APP_SHORTCUT_REMOVE_BASE: u32 = APP_SHORTCUT_KEY_BASE + 16;
pub(crate) const APP_SHORTCUT_REMOVE_MAX: u32 = APP_SHORTCUT_REMOVE_BASE + 7;
pub(crate) const EQ_KEY_BASE: u32 = 34_350;
pub(crate) const EQ_TOGGLE_KEY: u32 = EQ_KEY_BASE + 20;
pub(crate) const EQ_PRESET_BASE: u32 = EQ_KEY_BASE + 21;
pub(crate) const EQ_KEY_MAX: u32 = EQ_KEY_BASE + 9;
pub(crate) const EQ_PRESET_MAX: u32 = EQ_PRESET_BASE + 7;

pub(crate) const TODO_KEY_TOGGLE: u32 = 60;
pub(crate) const TODO_KEY_DELETE: u32 = 70;
pub(crate) const TODO_KEY_INPUT: u32 = 78;

// ── DashCmd ────────────────────────────────────────────────────────────────

/// A script-backed dashboard action queued by a toggle tile or vertical
/// slider; app.rs consumes these and spawns the matching *_main helper.
#[derive(Clone, Copy, Debug)]
pub enum DashCmd {
    /// brightness_main set N (0..100)
    Brightness(u32),
    /// theme_main switch — dark/light toggle on the identity card
    ThemeSwitch,
    /// audio_main vol set N (0..100)
    Volume(u32),
    /// audio_main mic set N (0..100)
    Mic(u32),
    /// shader_main set N (saturation 0..200, 100 = normal)
    Saturation(i32),
    /// hs_main set K (Kelvin)
    Kelvin(i32),
    CaffeineToggle,
    SunsetToggle,
    ShaderToggle,
}

// ── Catppuccin Mocha palette ───────────────────────────────────────────────

pub(crate) const FG2: u32 = 0xa6adc8ff;
pub(crate) const FG3: u32 = 0x7f849cff;
pub(crate) const SFG: u32 = 0x1e1e2eff;
pub(crate) const RED: u32 = 0xf38ba8ff;
pub(crate) const GREEN: u32 = 0xa6e3a1ff;

// ── Mode ───────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    Collapsed,
    Expanded,
    ControlCenter,
    Launcher,
    Notifications,
    Calendar,
    Settings,
    Power,
    WifiMenu,
    BtMenu,
    Audio,
    Wallpaper,
    Weather,
    /// clipboard history — text entries (SUPER+V)
    Clipboard,
    /// clipboard history — image entries (SUPER+SHIFT+V)
    ClipboardImages,
    /// theme picker — cards from $states2/themes (JSON mirror of the master)
    Themes,
    /// keybind viewer — read-only list from $states2/keybinds_cache (SUPER+SHIFT+/)
    Keybinds,
    /// transient OSD (morphs the pill into a small level/status box)
    Osd,
    /// workspace switcher — Tide Island–inspired grid of workspace cards
    WorkspaceSwitcher,
    /// native lockscreen — the bar morphs to fullscreen and grabs the keyboard
    Lock,
    /// polkit authentication dialog (replaces mate-polkit / hyprpolkitagent)
    PolkitAuth,
}

impl Mode {
    pub fn size(&self, cfg: &Config) -> (f32, f32) {
        use Mode::*;
        match self {
            Collapsed => (cfg.collapsed_w(), cfg.bar_h()),
            Expanded => (cfg.expanded_w(), EXPANDED_H),
            ControlCenter => (380.0, 380.0),
            Launcher => (560.0, 460.0),
            Notifications => (420.0, 480.0),
            Calendar => (400.0, 360.0),
            Settings => (760.0, 880.0),
            Power => (700.0, 350.0),
            WifiMenu => (440.0, 480.0),
            BtMenu => (440.0, 480.0),
            Audio => (440.0, 480.0),
            Wallpaper => (620.0, 540.0),
            Weather => (420.0, 360.0),
            Clipboard => (560.0, 460.0),
            ClipboardImages => (560.0, 420.0),
            Themes => (620.0, 540.0),
            Keybinds => (560.0, 540.0),
            Osd => (260.0, 68.0),
            WorkspaceSwitcher => (600.0, 320.0),
            Lock => (0.0, 0.0),
            PolkitAuth => (420.0, 320.0),
        }
    }
}

// ── OsdInfo, Notif, Cmd, Anim ──────────────────────────────────────────────

/// A transient OSD notification (AC plug, volume, brightness, mute).
#[derive(Clone, Debug, PartialEq)]
pub struct OsdInfo {
    /// nerd-font glyph
    pub glyph: &'static str,
    /// main text, e.g. "80%" / "AC plugged" / "Muted"
    pub text: String,
    /// mode to morph back to when the OSD dismisses
    pub prev: Mode,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Notif {
    pub id: u64,
    pub app: String,
    pub summary: String,
    pub body: String,
    /// app icon (name or absolute path); empty → no icon
    pub icon: String,
    /// 0 low, 1 normal, 2 critical
    pub urgency: u8,
    /// action buttons: (action_id, display_label)
    #[serde(default)]
    pub actions: Vec<(String, String)>,
}

impl Notif {
    /// `ImageStore` key for the app icon, if any.
    pub fn icon_key(&self) -> Option<String> {
        if self.icon.is_empty() {
            return None;
        }
        let path = std::path::Path::new(&self.icon);
        if path.is_absolute() {
            Some(format!("file:{}", path.display()))
        } else {
            Some(format!("icon:{}", self.icon))
        }
    }
}

pub enum Cmd {
    Rect { x: f32, y: f32, w: f32, h: f32, r: f32, color: u32 },
    /// Rounded rect with per-corner radii — negative = concave.
    RectConcave { x: f32, y: f32, w: f32, h: f32, r_tl: f32, r_tr: f32, r_br: f32, r_bl: f32, color: u32 },
    /// Stroked (outline-only) rounded rect.
    Outline { x: f32, y: f32, w: f32, h: f32, r: f32, width: f32, color: u32 },
    /// Thick line segment (capsule).
    Line { x0: f32, y0: f32, x1: f32, y1: f32, w: f32, color: u32 },
    Text { x: f32, y: f32, right: bool, center: bool, text: String, size: f32, color: u32, icon: bool },
    /// Rasterized image centered in a `size`×`size` box.
    Image { x: f32, y: f32, size: f32, key: String },
}

pub struct Anim {
    pub from_w: f32,
    pub from_h: f32,
    pub to_w: f32,
    pub to_h: f32,
    pub start: Instant,
    pub dur: f32,
}

// ── Free helper functions ───────────────────────────────────────────────────

/// Popup surface size for these notifications (≤ 3 visible).
pub fn notif_popup_size(notifs: &[Notif]) -> (i32, i32) {
    if notifs.is_empty() {
        return (0, 0);
    }
    let mut h = NOTIF_PAD;
    for n in notifs.iter().take(3) {
        h += if n.actions.is_empty() { NOTIF_CARD_H } else { NOTIF_CARD_H_ACTIONS } + NOTIF_GAP;
    }
    h -= NOTIF_GAP - NOTIF_PAD;
    (NOTIF_POPUP_W, h as i32)
}

/// System hostname from `/etc/hostname` or `gethostname`.
pub(crate) fn hostname() -> String {
    std::fs::read_to_string("/etc/hostname")
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| {
            use std::os::unix::io::AsRawFd;
            let mut buf = [0u8; 256];
            unsafe { libc::gethostname(buf.as_mut_ptr() as *mut libc::c_char, buf.len()); }
            let n = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
            String::from_utf8_lossy(&buf[..n]).to_string()
        })
}

/// Current username from `$USER` or `getlogin`.
pub(crate) fn whoami() -> String {
    std::env::var("USER").unwrap_or_else(|_| {
        unsafe {
            let ptr = libc::getlogin();
            if ptr.is_null() {
                "user".into()
            } else {
                std::ffi::CStr::from_ptr(ptr).to_string_lossy().to_string()
            }
        }
    })
}

/// System uptime as a human-readable string ("3d 2h 15m" / "42m" / "5s").
pub(crate) fn uptime_str() -> String {
    let Ok(s) = std::fs::read_to_string("/proc/uptime") else {
        return String::new();
    };
    let total = s.split_whitespace().next()
        .and_then(|t| t.parse::<f64>().ok())
        .unwrap_or(0.0) as u64;
    let d = total / 86400;
    let h = (total % 86400) / 3600;
    let m = (total % 3600) / 60;
    if d > 0 {
        format!("{d}d {h}h {m}m")
    } else if h > 0 {
        format!("{h}h {m}m")
    } else {
        format!("{m}m")
    }
}

/// Notification history file: `$XDG_STATE_HOME/zen-shell/notifs.json`
pub fn notif_path() -> std::path::PathBuf {
    let base = std::env::var("XDG_STATE_HOME")
        .map(|d| std::path::PathBuf::from(d))
        .or_else(|_| std::env::var("HOME").map(|h| std::path::PathBuf::from(h).join(".local/state")))
        .unwrap_or_else(|_| std::path::PathBuf::from("/tmp"));
    base.join("zen-shell").join("notifs.json")
}

/// `123456789` µs → `m:ss` (MPRIS track times).
pub fn fmt_time(micros: i64) -> String {
    let s = (micros / 1_000_000).max(0);
    format!("{}:{:02}", s / 60, s % 60)
}

/// Plain cubic ease-out — smooth settle, no overshoot.
pub(crate) fn ease_out_cubic(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(3)
}

// ── Banner zone helpers ─────────────────────────────────────────────────────

/// Default L/C/R section for a banner chip in the strip.
pub(crate) fn default_banner_zone(item: &BannerItem) -> u8 {
    match item {
        BannerItem::Workspaces | BannerItem::Title | BannerItem::Date | BannerItem::Clock
        | BannerItem::AppSearch => 1,   // center
        _ => 0,                          // left
    }
}

/// Insertion index for a banner chip with the default zone in `banner_order`.
pub(crate) fn banner_home_zone(item: &BannerItem, banner_order: &[BannerToken]) -> usize {
    let zone = default_banner_zone(item);
    let mut last = 0;
    for (i, t) in banner_order.iter().enumerate() {
        if let Some(z) = t.as_zone() {
            if z == zone {
                last = i + 1;
            }
        }
    }
    last
}

/// Insertion index for a drop onto a banner strip (pointer position → slot).
pub(crate) fn banner_drop_target(x: f32, strip_row_w: f32, banner_order: &[BannerToken]) -> usize {
    if strip_row_w <= 0.0 || banner_order.is_empty() {
        return banner_order.len();
    }
    let slot_w = strip_row_w / banner_order.len() as f32;
    ((x / slot_w).round() as usize).min(banner_order.len())
}
