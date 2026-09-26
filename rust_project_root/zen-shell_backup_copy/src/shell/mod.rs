//! The pill bar — state machine for all modes, mirroring the C++
//! raze-shell. Pure logic: no wayland / GL. Emits `Cmd` scenes.
//! Panel layouts live in the child module `panels`.

mod geometry;
mod banner;
mod input;
mod scroll;
mod state;
mod panels;
pub mod worldmap;

/// Offline map data (GitHub-hosted chunks): manifest, cache dirs by save
/// toggle, curl+sha256 downloads. `worldmap` holds the runtime data store.
pub mod mapdata;

use crate::apps::AppManager;
use crate::config::Config;
use crate::icons::*;
use crate::tray::TrayItem;
use std::time::Instant;

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
    /// resting pill: ALL workspace numbers (1..5) always visible
    WorkspacesLong,
    /// resting pill: only the active workspace number
    WorkspacesShort,
    /// resting pill: Wi-Fi chip → Wifi menu
    Wifi,
    /// resting pill: Bluetooth chip → BT menu
    Bluetooth,
    /// resting pill: volume chip → Sound panel
    Volume,
    /// resting pill: wallpaper chip → Wallpaper panel
    Wallpaper,
    /// resting pill: themes chip → Themes picker
    Themes,
    /// resting pill: brand glyph (`$states2/d` in `$states2/branding_font`)
    Branding,
}

/// Default pill layout order (left → right). Items not present are skipped.
impl Default for PillItem {
    fn default() -> Self {
        PillItem::Workspaces
    }
}

/// Background resources a RESTING pill consumes. The broker keeps these
/// fetching while the pill is on-screen, even with the dashboard closed.
impl PillItem {
    pub fn needs(self) -> &'static [(Res, Rate)] {
        &[] // none of the resting pills consume a broker-managed fetch today
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
    PillItem::WorkspacesLong,
    PillItem::WorkspacesShort,
    PillItem::Wifi,
    PillItem::Bluetooth,
    PillItem::Volume,
    PillItem::Wallpaper,
    PillItem::Themes,
    PillItem::Branding,
];

/// Screen edge the bar strip is anchored to. The pill stays a horizontal
/// capsule on every edge — left/right placements just re-anchor the surface
/// (flush against the edge, vertically centered).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BarEdge {
    Left,
    Top,
    Right,
    Bottom,
}

impl BarEdge {
    /// Persisted `bar.anchor` value.
    pub fn name(self) -> &'static str {
        match self {
            BarEdge::Left => "left",
            BarEdge::Top => "top",
            BarEdge::Right => "right",
            BarEdge::Bottom => "bottom",
        }
    }

    pub fn from_name(s: &str) -> Option<BarEdge> {
        match s {
            "left" => Some(BarEdge::Left),
            "top" => Some(BarEdge::Top),
            "right" => Some(BarEdge::Right),
            "bottom" => Some(BarEdge::Bottom),
            _ => None,
        }
    }
}

/// One chip on the expanded-dashboard TOP BANNER (reorderable / removable
/// in edit mode). Mirrors the collapsed-pill items: a configurable strip
/// of chips that persists across sessions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
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
    /// brand glyph (`$states2/d` in `$states2/branding_font`) — display-only
    Branding,
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
            BannerItem::Branding => "branding",
        }
    }

    /// Nerd Font glyph shown for this chip in EDIT MODE (in place of its
    /// word label). Icons mirror the live-chip glyphs so the parked tray and
    /// the strip read at a glance; chips without a natural single glyph get
    /// a stable representative icon.
    pub fn glyph(&self) -> &'static str {
        match self {
            BannerItem::Workspaces => ICON_GRID, // grid_view (workspaces)
            BannerItem::Title => ICON_DESKTOP_WINDOW,      // desktop_windows (focused window)
            BannerItem::Date => ICON_CALENDAR_FILL,       // calendar_today
            BannerItem::Clock => ICON_SCHEDULE,      // schedule (clock)
            BannerItem::AppSearch => ICON_SEARCH_FILL,  // search (magnifying glass)
            BannerItem::Tray => ICON_APPS_FILL,       // apps (tray icons)
            BannerItem::Wifi => ICON_WIFI_FILL,       // wifi
            BannerItem::Bluetooth => ICON_BLUETOOTH_FILL,  // bluetooth
            BannerItem::Connectivity => ICON_LINK, // link (connectivity)
            BannerItem::Battery => ICON_BATTERY_FULL,    // battery_full
            BannerItem::Weather => ICON_PARTLY_CLOUDY,    // partly_cloudy_day
            BannerItem::Cpu => ICON_SPEED_FILL,        // speed
            BannerItem::Ram => ICON_MEMORY,        // memory
            BannerItem::NetSpeed => ICON_SWAP,   // swap_vert (up/down)
            BannerItem::Dnd => ICON_BELL_OFF_FILL,        // notifications_off
            BannerItem::Volume => ICON_VOLUME_FILL,     // volume_up
            BannerItem::Brightness => ICON_BRIGHTNESS_FILL, // wb_sunny (brightness)
            BannerItem::Vpn => ICON_LOCK_FILL,        // lock
            BannerItem::Settings => ICON_SETTINGS_FILL,   // settings (gear)
            BannerItem::Power => ICON_POWER_FILL,      // power_off
            BannerItem::DashToggle => ICON_GRID, // grid_view (kept word-only in edit)
            BannerItem::Branding => ICON_SPARKLE, // sparkles (brand mark)
        }
    }

    /// Background resources a visible banner chip needs. The broker treats
    /// each rendered banner chip as a permanent fetch client (they're
    /// on-screen whenever the strip renders).
    pub fn needs(self) -> &'static [(Res, Rate)] {
        use Rate::*;
        match self {
            BannerItem::Weather => &[(Res::Weather, Slow)],
            _ => &[],
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
    BannerItem::Branding,
];

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
            "branding" => BannerItem::Branding,
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
    ///
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

/// Uniform gap between collapsed-pill modules (px) — shared by the width
/// estimator (`collapsed_w`) and the drawer (`layout_collapsed`) so the pill is
/// sized exactly to what it draws.
pub const PILL_GAP: f32 = 6.0;

/// Dashboard scrollbar overlay timing (seconds): how long the bars stay
/// fully visible after the last scroll / drag, then how long the fade-out
/// takes. They render only while this window is live.
pub(crate) const DASH_SB_KEEP: f64 = 3.0;
pub(crate) const DASH_SB_FADE: f64 = 0.5;

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

/// Base key for custom-accent grid circle hit-regions (Settings → Appearance).
/// Set far above every other Settings-mode key (1..218, 300) so all accents —
/// which can number in the hundreds — selectable without colliding. A click
/// region key is `CUSTOM_ACC_KEY_BASE + accent_index`.
pub const CUSTOM_ACC_KEY_BASE: u32 = 100_000;

/// Base key for dashboard Wallpaper-card thumbnails. Each visible thumbnail
/// is `WP_CARD_KEY_BASE + visible_index` (its file is picked via the
/// card-local scroll offset + column layout). Set far above every other
/// Expanded-mode key (1..79, 2000-range clearance).
pub(crate) const WP_CARD_KEY_BASE: u32 = 2000;
/// Upper bound for Wallpaper-card thumbnail keys — set far below the
/// custom-accent keys so the click ranges never overlap.
pub(crate) const WP_CARD_KEY_MAX: u32 = 9999;

/// Base key for Calendar-card day cells (Expanded mode): 30_000 + cell.
/// Next to (never inside) the wallpaper band; far below custom-accent keys.
pub(crate) const CAL_KEY_BASE: u32 = 30_000;
/// prev / next month on the Calendar card.
pub(crate) const CAL_KEY_BASE_PREV: u32 = 30_100;
pub(crate) const CAL_KEY_BASE_NEXT: u32 = 30_101;
/// top Calendar-card day-cell key (30_000 + cell → fixed pattern bound).
pub(crate) const CAL_KEY_BASE_MAX: u32 = CAL_KEY_BASE + 41;
/// Base key for Accent-card source rows: 32_000 + row_index.
pub(crate) const ACC_KEY_BASE: u32 = 32_000;
/// top Accent-card row key (fixed pattern bound).
pub(crate) const ACC_KEY_BASE_MAX: u32 = ACC_KEY_BASE + 3;
/// Back button (`<`) on the Accent-card custom-accents subview: 32_100.
pub(crate) const ACC_LIST_BACK_KEY: u32 = 32_100;
/// Chevron (`>`) on the Accent-card "Custom accent" row — opens the subview.
pub(crate) const ACC_LIST_CHEV_KEY: u32 = 32_101;
/// Base key for custom-accent rows in the Accent-card subview:
/// 32_200 + visible row.
pub(crate) const ACC_LIST_KEY_BASE: u32 = 32_200;
/// top custom-accent row key (fixed pattern bound).
pub(crate) const ACC_LIST_KEY_MAX: u32 = ACC_LIST_KEY_BASE + 99;
/// `+` composer button on the Notes card: 32_120.
pub(crate) const NOTES_KEY_INPUT: u32 = 32_120;
/// Notes composer save `+` button: 32_121.
pub(crate) const NOTES_KEY_SAVE: u32 = 32_121;
/// Mirror-card "take photo" button: 30_700 (below NOTES_KEY_INPUT range).
pub(crate) const MIRROR_PHOTO_KEY: u32 = 30_700;
/// Mirror-card "record / stop" button: 30_701.
pub(crate) const MIRROR_REC_KEY: u32 = 30_701;
/// Mirror-card "show actual fps" switch (inside the fps pane).
pub(crate) const MIRROR_FPS_SHOW_KEY: u32 = 30_704;
/// Mirror-card fps menu rows (15/30/60) — base of a 3-value range.
pub(crate) const MIRROR_FPS_ROW_BASE: u32 = 30_710;
/// Mirror-card capture-rate options, low→high (index = row key − row base).
pub(crate) const MIRROR_FPS_OPTIONS: [u32; 3] = [15, 30, 60];
/// Sliders-card pane-2 toggle: "Show balance" slider — 30_720.
pub(crate) const SLIDERS_TOGGLE_BALANCE_KEY: u32 = 30_720;
/// Sliders-card pane-2 toggle: "Show saturation" slider — 30_721.
pub(crate) const SLIDERS_TOGGLE_SAT_KEY: u32 = 30_721;
/// CPU-governor power-save toggle on the battery card: 32_130.
pub(crate) const BATTERY_PSAVE_KEY: u32 = 32_130;
/// Key for the Viz-card bar-style cycle button.
pub(crate) const VIZ_KEY_BASE: u32 = 33_000;
/// Base key for Lyrics-card lines: 33_500 + visible row index. Clicking a
/// line is a noop; the band exists so rows get hover highlighting. Kept below
/// the banner band (34_000), which swallows every key above it.
pub(crate) const LYRICS_KEY_BASE: u32 = 33_500;
/// Base key for Wifi-card network rows: 14_000 + visible row.
pub(crate) const WIFI_KEY_BASE: u32 = 14_000;
/// Base key for Bluetooth-card device rows: 14_100 + visible row.
pub(crate) const BT_KEY_BASE: u32 = 14_100;
/// Key for the SpeedTest-card "run test" button: 14_200.
pub(crate) const SPEED_KEY_BASE: u32 = 14_200;
/// Base key for Recent-files card rows: 14_300 + visible row.
pub(crate) const RECENT_KEY_BASE: u32 = 14_300;
/// Keys for the Currency card: 14_400 + visible row (row click re-bases).
pub(crate) const CURRENCY_KEY_BASE: u32 = 14_400;
/// Keys for the Quote card: 14_500 = refresh · 14_501 = save.
pub(crate) const QUOTE_KEY_BASE: u32 = 14_500;
/// Save pill on the Quote card: 14_501.
pub(crate) const QUOTE_SAVE_KEY: u32 = 14_501;
/// Base key for Ticker-card rows: 14_600 + visible row (row click opens the
/// asset's market page).
pub(crate) const TICKER_KEY_BASE: u32 = 14_600;

/// World-map card: the whole map body is one clickable region — a click
/// resolves to (lat, lon) → nearest zone.tab timezone. Lives above the
/// 14_xxx row-card range and below the browse/panel ranges.
pub(crate) const WORLD_MAP_KEY: u32 = 14_900;
/// World-map card internal buttons (registered over the card body):
pub(crate) const WORLD_DL_KEY: u32 = 14_901; // download base world map
pub(crate) const WORLD_REGION_DL_KEY: u32 = 14_902; // download the shown region
pub(crate) const WORLD_ZOOM_IN_KEY: u32 = 14_903; // "+" zoom button
pub(crate) const WORLD_ZOOM_OUT_KEY: u32 = 14_904; // "−" zoom button
pub(crate) const WORLD_MENU_KEY: u32 = 14_905; // style/settings menu backdrop
pub(crate) const WORLD_MENU_TOGGLE_KEY: u32 = 14_906; // "save downloads" switch
/// Style rows inside the map menu: 14_910 + style index.
pub(crate) const WORLD_STYLE_KEY_BASE: u32 = 14_910;

/// Style panes on the world-map card, chosen from the swipe menu.
pub(crate) const WORLD_PANES: usize = 4;

/// A download that has been requested but not yet started (app services timer
/// spawns the worker + drains the result channel).
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum WorldDl {
    /// fetch manifest + world.bin + cities.bin
    Base,
    /// fetch one country chunk
    Region(String),
}

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
/// Base key for News-card category chips (Expanded mode): 13_000 + chip index
/// (0 = "All"). 0 = All · 1..N = the N configured feed categories.
pub(crate) const NEWS_CAT_KEY_BASE: u32 = 13_000;
pub(crate) const NEWS_CAT_KEY_MAX: u32 = NEWS_CAT_KEY_BASE + 199;
/// Base key for News-card headline rows: 13_300 + visible row index. The
/// row resolves to a story via the card-local vertical scroll + category.
/// The head/CAT bands must NOT overlap — the category arm matches first.
pub(crate) const NEWS_HEAD_KEY_BASE: u32 = 13_300;
pub(crate) const NEWS_HEAD_KEY_MAX: u32 = NEWS_HEAD_KEY_BASE + 199;

/// Base key for dashboard Power-card action tiles (Expanded mode). Each tile
/// is `POWER_CARD_KEY_BASE + i` (0 lock · 1 logout · 2 sleep · 3 restart ·
/// 4 shutdown) and runs the same hold-to-confirm flow as the power menu.
/// Kept BELOW the banner band (34_000); the banner arm catches every key
/// `>= BANNER_KEY_BASE`, which would swallow anything above it.
pub(crate) const POWER_CARD_KEY_BASE: u32 = 12_000;
pub(crate) const POWER_CARD_KEY_MAX: u32 = POWER_CARD_KEY_BASE + 4;

/// Keys for the copied-image card, one per thumbnail/name row.
/// Small generic range (140..159) — clear of the media/toggle/slider/todo
/// bands used in Expanded mode.
pub(crate) const CLIPIMG_KEY_BASE: u32 = 140;
pub(crate) const CLIPIMG_KEY_MAX: u32 = CLIPIMG_KEY_BASE + 19;

/// Keys for the clipboard text card, one per history row.
pub(crate) const CLIP_KEY_BASE: u32 = 80;
pub(crate) const CLIP_KEY_MAX: u32 = CLIP_KEY_BASE + 19;

/// Keys for the Wallpaper-card scrollbar (page up / page down). Placed above
/// the thumbnail band (2000..9999) so they never collide with tile keys.
pub(crate) const WP_CARD_SB_UP: u32 = 10_000;
pub(crate) const WP_CARD_SB_DOWN: u32 = 10_001;

/// Base key for top-banner strip slots (Expanded mode). Strip slot i (an
/// index into `banner_order`) is `BANNER_KEY_BASE + i`; the move grip below
/// each edit-mode slot is `BANNER_GRIP_KEY_BASE + i` (a press lifts the chip
/// into a drag so it can be re-dropped anywhere on the strip — left / center
/// / right); parked banner tokens on the banner-chips tray are
/// `BANNER_TRAY_KEY_BASE + i` (chips, separators AND the smart filler all
/// park there; click appends to the strip). The strip-editor control row
/// (edit mode) sits right below the banner tray: `BANNER_CTRL_KEY_BASE`
/// toggles the dashboard on, and the width −/+ keys grow / shrink the
/// banner-only strip (dashboard OFF only).
pub(crate) const BANNER_KEY_BASE: u32 = 34_000;
    /// `BANNER_PARK_KEY_BASE + i` — the EDIT-MODE "park" minus badge on the
    /// top-right corner of strip chip i (the cursor is in the cell). Clicking
    /// it sends the chip back to the parked tray — the ONLY way a strip chip
    /// is parked now (the bottom grip is the move handle instead). Never
    /// drawn or hit-testable outside edit mode. Lives above the slot keys
    /// (`BANNER_KEY_BASE + i`; the strip can never grow past ~30 tokens) and
    /// below the edit grips (34_100).
    pub(crate) const BANNER_PARK_KEY_BASE: u32 = 34_050;
    pub(crate) const BANNER_PARK_KEY_MAX: u32 = 34_099;
    pub(crate) const BANNER_GRIP_KEY_BASE: u32 = 34_100;
    pub(crate) const BANNER_GRIP_KEY_MAX: u32 = 34_199;
pub(crate) const BANNER_TRAY_KEY_BASE: u32 = 34_200;
pub(crate) const BANNER_CTRL_KEY_BASE: u32 = 34_300;
pub(crate) const BANNER_W_AUTO_KEY: u32 = 34_332;
pub(crate) const BANNER_W_SLIDER_KEY: u32 = 34_333;
/// Connectivity-popover menu rows (aggregate "net" chip): jump to the
/// Wi-Fi / Bluetooth menus.
pub(crate) const CONN_WIFI_KEY: u32 = 34_360;
    pub(crate) const CONN_BT_KEY: u32 = 34_361;
    /// System-card hidden menu (⋮ top-right): row keys — "Bar position"
    /// expands a 4-edge submenu, "Open settings" jumps to the Settings panel.
pub(crate) const SYS_MENU_KEY: u32 = 34_390;
    pub(crate) const SYS_MENU_BP_KEY: u32 = 34_391;
    pub(crate) const SYS_MENU_SETTINGS_KEY: u32 = 34_392;
    /// Bar-edge chips in the ⋮ submenu AND the Settings → Pill position row.
    /// The active edge is accented; clicking re-anchors the bar live.
    pub(crate) const SYS_EDGE_L_KEY: u32 = 34_393;
    pub(crate) const SYS_EDGE_T_KEY: u32 = 34_394;
    pub(crate) const SYS_EDGE_R_KEY: u32 = 34_395;
    pub(crate) const SYS_EDGE_B_KEY: u32 = 34_396;
    /// Edit-mode UI controls row (below parked-cards tray): −/+ steppers
    /// for dashboard padding and window/card roundness (card is LINKED to
    /// the window radius) plus one reset button.
    pub(crate) const UI_PAD_MINUS_KEY: u32 = 34_380;
    pub(crate) const UI_PAD_PLUS_KEY: u32 = 34_381;
    pub(crate) const WIN_RAD_MINUS_KEY: u32 = 34_382;
    pub(crate) const WIN_RAD_PLUS_KEY: u32 = 34_383;
    pub(crate) const CARD_RAD_MINUS_KEY: u32 = 34_384;
    pub(crate) const CARD_RAD_PLUS_KEY: u32 = 34_385;
    pub(crate) const UI_CTRL_RESET_KEY: u32 = 34_386;
    /// Edit-mode strip-editor row: card-header visibility toggles ("Card
    /// titles" / "Card glyphs"). Off hides every card's header title text /
    /// header icon; both default ON.
    pub(crate) const CARD_TITLE_TOGGLE_KEY: u32 = 34_387;
    pub(crate) const CARD_GLYPH_TOGGLE_KEY: u32 = 34_388;
    /// Tray pager: visible rows shown at once for the banner-chips tray
    /// and dashboard-cards tray (edit mode).
    pub(crate) const BANNER_TRAY_VROWS: usize = 2;
    pub(crate) const DASH_TRAY_VROWS: usize = 3;
/// Scroll direction toggle button in edit-mode tray header
pub(crate) const DASH_SCROLL_DIR_KEY: u32 = 34_340;
/// Clear all cards button in edit-mode tray (hold-to-confirm)
pub(crate) const DASH_CLEAR_ALL_KEY: u32 = 34_341;
/// "Clear all" on the top BANNER strip (edit mode): hold 3 s to move every
/// chip back to the parked tray.
pub(crate) const BANNER_CLEAR_ALL_KEY: u32 = 34_350;
/// "Enable banner chips" toggle in the strip-editor row (right side): hides
/// every item on the top banner strip (band collapses).
pub(crate) const BANNER_CHIPS_TOGGLE_KEY: u32 = 34_351;
/// Hover regions for dashboard cards (one per packed slot, in layout order).
    /// Not a real interaction slot — flipping `hover_key` when the pointer crosses
    /// a card lets hover-only chrome (e.g. multi-pane pagination dots) repaint.
    pub(crate) const DASH_CARD_KEY: u32 = 34_480;
/// AppShortcut dashboard card keys: 34_400..34_40F = the 8 shortcut slots
/// (click launches a filled slot / opens the launcher picker on an empty
/// "+" slot); 34_410..34_41F = the hover-only ✕ that removes a slot.
pub(crate) const APP_SHORTCUT_KEY_BASE: u32 = 34_400;
pub(crate) const APP_SHORTCUT_KEY_MAX: u32 = APP_SHORTCUT_KEY_BASE + 7;
pub(crate) const APP_SHORTCUT_REMOVE_BASE: u32 = APP_SHORTCUT_KEY_BASE + 16;
pub(crate) const APP_SHORTCUT_REMOVE_MAX: u32 = APP_SHORTCUT_REMOVE_BASE + 7;
/// EQ card key range: 34_350..34_369 (band sliders + preset chips + toggle)
pub(crate) const EQ_KEY_BASE: u32 = 34_350;
/// EQ toggle on/off (key EQ_KEY_BASE + 20)
pub(crate) const EQ_TOGGLE_KEY: u32 = EQ_KEY_BASE + 20;
/// EQ preset chips start at EQ_KEY_BASE + 21
pub(crate) const EQ_PRESET_BASE: u32 = EQ_KEY_BASE + 21;
/// Last EQ band slider key
pub(crate) const EQ_KEY_MAX: u32 = EQ_KEY_BASE + 9;
/// Last EQ preset key
pub(crate) const EQ_PRESET_MAX: u32 = EQ_PRESET_BASE + 7;

/// Width of the Settings app's left navigation sidebar (two-pane layout).
/// The content pane starts at this x and runs to the panel's right edge.
pub const SETTINGS_SIDEBAR: f32 = 180.0;
/// Height added to the Settings surface while the Pill section shows its
/// "expand on hover" info tip (kept in sync with the band in settings.rs).
pub const SETTINGS_EXPAND_TIP_H: f32 = 44.0;

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
    /// true when at least one swap happened
    pub swapped: bool,
}

pub const EXPANDED_H: f32 = 480.0;

// ── corner notification popup ────────────────────────────────────────────
// Shared by the surface sizer (app.rs) and the layout drawer (panels.rs) so
// the surface is always exactly as tall as its content.
pub const NOTIF_POPUP_W: i32 = 470;
pub const NOTIF_PAD: f32 = 12.0;
pub const NOTIF_GAP: f32 = 10.0;
pub const NOTIF_CARD_H: f32 = 92.0;
pub const NOTIF_CARD_H_ACTIONS: f32 = 122.0;

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

// ── Dashboard metro grid ────────────────────────────────────────────────
// Cards are N×M spans on a fine cell lattice (Windows-10-start style).
// Positions are DERIVED by a first-fit flow packer in card order — growing
// or moving one card PUSHES the others aside instead of blocking, and the
// canvas grows taller when the packed content needs another row. Only the
// order + spans persist (~/.local/share/zen-shell/dash_layout.txt).
pub const DASH_COLS: u16 = 20;
pub const DASH_ROWS: u16 = 12;
/// Base (unscaled) grid geometry — multiply by GridScale at runtime.
pub(crate) const BASE_DASH_GAP: f32 = 6.0;
pub(crate) const BASE_DASH_TOP: f32 = 48.0;
pub(crate) const BASE_TRAY_GAP: f32 = 8.0;
/// Banner chip strip: chip y offset + chip height (unscaled), below a small
/// top margin — shared by the expanded banner and the banner parked tray.
pub(crate) const BASE_BANNER_Y: f32 = 12.0;
pub(crate) const BASE_BANNER_H: f32 = 26.0;
/// hard safety cap on packed rows. The canvas AUTO-GROWS when new cards are
/// added and won't fit the current rows — this is only a sanity ceiling so a
/// corrupt layout can't spin the canvas absurdly large. 4096 rows ≈ 100s of
/// stacked cards; no realistic grid ever reaches it.
const DASH_MAX_ROWS: u16 = 4096;

/// Uniform zoom factor for the dashboard grid.  `s(px)` scales any pixel
/// value; `fs(size)` scales font sizes with 1-decimal precision.
#[derive(Clone, Copy)]
pub struct GridScale(pub f32);
impl GridScale {
    pub fn s(self, px: f32) -> f32 { px * self.0 }
    /// Scale a font size — round to 1 decimal to avoid subpixel jitter.
    pub fn fs(self, size: f32) -> f32 { (size * self.0 * 10.0).round() / 10.0 }
}
/// Predefined card sizes the resize grip cycles through (w × h in cells).
/// Each click advances to the next size; after the last, wraps to the first.
#[allow(dead_code)]
const CARD_SIZES: &[(u16, u16)] = &[
    (1, 1),
    (4, 2),
    (6, 2),
    (10, 2),
    (2, 4),
    (4, 4),
    (6, 4),
    (10, 4),
];

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
    /// Real-audio visualizer bars (sink monitor → FFT) + bar-style toggle.
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
    /// Battery card with a VERTICAL vector battery icon. Pane 0 = icon +
    /// percent, pane 1 = extra info + CPU-governor power-save toggle.
    /// Horizontal scroll switches panes; dots below mark the active pane.
    BatteryV,
    /// Battery card with a HORIZONTAL vector battery icon (same panes).
    BatteryH,
    /// 10-band parametric equalizer with frequency response curve and presets.
    Eq,
    /// User-pinned app launcher card: empty "+" tiles the user fills from the
    /// launcher; clicking a filled slot launches the app.
    AppShortcut,
    /// Interactive world map (vector, Natural Earth 110m). Clicking a spot
    /// pins a marker and shows that place's local time; the card has four
    /// style panes — 0 = real, 1 = filled, 2 = dotted, 3 = stroked — flipped
    /// by horizontal scroll.
    WorldMap,
    /// Minimalist weather card: light "paper" surface with a left hero
    /// (Today / city / large temp / high-low) and a right 5-row forecast
    /// strip (glyph + temp per day).
    WeatherV2,
    /// Pomodoro focus timer — mm:ss countdown, start/pause/reset, focus
    /// length steppers; sessions persist to $XDG_DATA_HOME/zen-shell/pomo.txt.
    Pomodoro,
    /// Fan speeds from hwmon `fan*_input` (RPM) — per-fan rows + bars.
    Fans,
    /// Hyprland workspace tiles — click to switch; active tile accent-tinted.
    Workspaces,
    /// World clock — pinned cities with local times, day/night glyphs and
    /// offset-vs-local chips; inline zone.tab search to add cities.
    WorldClock,
    /// Water tracker — daily hydration log: bead-ring gauge, +glass/−undo,
    /// auto-resets at midnight; persisted to $XDG_DATA_HOME/zen-shell/water.txt.
    Water,
    /// Moon phase — vector shaded disc computed from the synodic cycle,
    /// with phase name + illumination % + days-until-full.
    Moon,
    /// Compositor / effects — live blur/shadow/opacity tiles + area/full
    /// screenshot buttons, wired to the `*_main` helper scripts.
    Compositor,
    /// Countdown — dated events sorted by soonest with days-until chips;
    /// past events gray out (manual delete).
    Countdown,
    /// Alarms — HH:MM reminders that fire the shell's notification popup.
    Alarms,
    /// Snippets — saved text clips; click copies to the clipboard.
    Snippets,
    /// Expenses — quick-amount chips + month total + per-category bars.
    Expenses,
    /// Eye rest — the 20-20-20 rule: ring + mm:ss focus countdown, 20 s
    /// rest phase after each stretch, sessions persisted to
    /// `$XDG_DATA_HOME/zen-shell/eyerest.txt`.
    EyeRest,
    /// Mic meter — live input level bar + mute toggle (reads the same
    /// `mic_level`/`mic_muted` the dashboard faders use).
    MicMeter,
    /// Audio device picker — sink/source rows (wpctl status). Row click
    /// makes the device default; the mute glyph toggles it.
    AudioDevice,
    /// Audio recorder — mic volume slider + record/stop, chevron opens pane 1
    /// with the saved recordings list (play / delete) and the confirm-delete /
    /// mic-slider toggles.
    AudioRec,
    /// Connection info — local interface IPs, default gateway and DNS
    /// resolvers, all read in-process (/proc/net, /etc/resolv.conf,
    /// getifaddrs).
    ConnInfo,
    /// Latency monitor — continuous `ping` sparkline to a probe host with a
    /// live ms readout and a manual re-probe button.
    Latency,
    /// Power draw — battery watts (+ GPU watts when hwmon exposes them) as
    /// a dual-line history graph.
    PowerDraw,
    /// Disk health (SMART) — per-disk model / temp / PASS·FAIL from
    /// `smartctl -j -a`, one row per disk.
    SmartHealth,
    /// Failed systemd units — red count + unit rows; empty = all green.
    SystemdUnits,
    /// System log tail — last N warning+ journal lines, severity-colored.
    JournalTail,
    /// Brand sign — the brand glyph (`$states2/d` in `$states2/branding_font`)
    /// rendered aspect-fit inside the card (never overflow / stretch).
    Branding,
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
            DashCard::WorldMap => "worldmap",
            DashCard::WeatherV2 => "weatherv2",
            DashCard::Pomodoro => "pomodoro",
            DashCard::Fans => "fans",
            DashCard::Workspaces => "workspaces",
            DashCard::WorldClock => "worldclock",
            DashCard::Water => "water",
            DashCard::Moon => "moon",
            DashCard::Compositor => "compositor",
            DashCard::Countdown => "countdown",
            DashCard::Alarms => "alarms",
            DashCard::Snippets => "snippets",
            DashCard::Expenses => "expenses",
            DashCard::EyeRest => "eyerest",
            DashCard::MicMeter => "micmeter",
            DashCard::AudioDevice => "audiodevice",
            DashCard::AudioRec => "audiorec",
            DashCard::ConnInfo => "conninfo",
            DashCard::Latency => "latency",
            DashCard::PowerDraw => "powerdraw",
            DashCard::SmartHealth => "smarthealth",
            DashCard::SystemdUnits => "systemdunits",
            DashCard::JournalTail => "journaltail",
            DashCard::Branding => "branding",
        }
    }
    fn from_id(s: &str) -> Option<Self> {
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
            "worldmap" => DashCard::WorldMap,
            "weatherv2" => DashCard::WeatherV2,
            "pomodoro" => DashCard::Pomodoro,
            "fans" => DashCard::Fans,
            "workspaces" => DashCard::Workspaces,
            "worldclock" => DashCard::WorldClock,
            "water" => DashCard::Water,
            "moon" => DashCard::Moon,
            "compositor" => DashCard::Compositor,
            "countdown" => DashCard::Countdown,
            "alarms" => DashCard::Alarms,
            "snippets" => DashCard::Snippets,
            "expenses" => DashCard::Expenses,
            "eyerest" => DashCard::EyeRest,
            "micmeter" => DashCard::MicMeter,
            "audiodevice" => DashCard::AudioDevice,
            "audiorec" => DashCard::AudioRec,
            "conninfo" => DashCard::ConnInfo,
            "latency" => DashCard::Latency,
            "powerdraw" => DashCard::PowerDraw,
            "smarthealth" => DashCard::SmartHealth,
            "systemdunits" => DashCard::SystemdUnits,
            "journaltail" => DashCard::JournalTail,
            "branding" => DashCard::Branding,
            _ => return None,
        })
    }

    /// Dispatch to the card's drawer function. Replaces the 40-arm match
    /// in `layout_expanded` — call `card.draw(shell, v, x, y, w, h, pal)`.
    pub(crate) fn draw(&self, shell: &mut Shell, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
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
            DashCard::WorldMap => shell.draw_worldmap_card(v, x, y, w, h, pal),
            DashCard::WeatherV2 => shell.draw_weather_v2_card(v, x, y, w, h, pal),
            DashCard::Pomodoro => shell.draw_pomodoro_card(v, x, y, w, h, pal),
            DashCard::Fans => shell.draw_fans_card(v, x, y, w, h, pal),
            DashCard::Workspaces => shell.draw_workspaces_card(v, x, y, w, h, pal),
            DashCard::WorldClock => shell.draw_worldclock_card(v, x, y, w, h, pal),
            DashCard::Water => shell.draw_water_card(v, x, y, w, h, pal),
            DashCard::Moon => shell.draw_moon_card(v, x, y, w, h, pal),
            DashCard::Compositor => shell.draw_compositor_card(v, x, y, w, h, pal),
            DashCard::Countdown => shell.draw_countdown_card(v, x, y, w, h, pal),
            DashCard::Alarms => shell.draw_alarms_card(v, x, y, w, h, pal),
            DashCard::Snippets => shell.draw_snippets_card(v, x, y, w, h, pal),
            DashCard::Expenses => shell.draw_expenses_card(v, x, y, w, h, pal),
            DashCard::EyeRest => shell.draw_eyerest_card(v, x, y, w, h, pal),
            DashCard::MicMeter => shell.draw_micmeter_card(v, x, y, w, h, pal),
            DashCard::AudioDevice => shell.draw_audiodevice_card(v, x, y, w, h, pal),
            DashCard::AudioRec => shell.draw_audiorec_card(v, x, y, w, h, pal),
            DashCard::ConnInfo => shell.draw_conninfo_card(v, x, y, w, h, pal),
            DashCard::Latency => shell.draw_latency_card(v, x, y, w, h, pal),
            DashCard::PowerDraw => shell.draw_powerdraw_card(v, x, y, w, h, pal),
            DashCard::SmartHealth => shell.draw_smarthealth_card(v, x, y, w, h, pal),
            DashCard::SystemdUnits => shell.draw_systemdunits_card(v, x, y, w, h, pal),
            DashCard::JournalTail => shell.draw_journaltail_card(v, x, y, w, h, pal),
            DashCard::Branding => shell.draw_branding_card(v, x, y, w, h, pal),
        }
    }

    /// The background resources this card needs while it's ON the grid.
    /// Empty = no background fetching (pure local draw). The broker unions
    /// these over every on-screen card + banner chip + resting pill, and only
    /// fetches the resources with ≥1 consumer.
    pub fn needs(self) -> &'static [(Res, Rate)] {
        use Rate::*;
        match self {
            DashCard::Weather => &[(Res::Weather, Slow)],
            DashCard::WeatherV2 => &[(Res::Weather, Slow)],
            DashCard::News => &[(Res::News, Steady)],
            DashCard::Ticker => &[(Res::Ticker, Steady)],
            DashCard::Quote => &[(Res::Quote, Steady)],
            DashCard::Lyrics => &[(Res::Lyrics, Manual)],
            DashCard::SpeedTest => &[(Res::SpeedTest, Manual)],
            _ => &[],
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

/// A fetchable background data source. The broker only fetches resources
/// that at least one ON-SCREEN consumer (packed card, banner chip, resting
/// pill) declares via `needs()` — zero consumers ⇒ zero fetches.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Res {
    /// Open-Meteo weather (thread, geolocation-cached).
    Weather,
    /// RSS/JSON headline feeds.
    News,
    /// Crypto/stock price rows.
    Ticker,
    /// Quote of the day.
    Quote,
    /// Track lyrics (LRCLib + cache).
    Lyrics,
    /// On-demand bandwidth probe (button only — Manual rate).
    SpeedTest,
}

impl Res {
    pub fn name(self) -> &'static str {
        match self {
            Res::Weather => "weather",
            Res::News => "news",
            Res::Ticker => "ticker",
            Res::Quote => "quote",
            Res::Lyrics => "lyrics",
            Res::SpeedTest => "speedtest",
        }
    }
}

/// Refresh cadence a consumer wants for a resource. The broker fetches each
/// resource at the FASTEST requested rate among its current on-screen
/// consumers. `Manual` never auto-fetches — it only fires on an explicit
/// trigger (a button press, a track change).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Rate {
    /// ~25 s — steady values (news, quotes).
    Steady,
    /// ~15 min — slow values (weather).
    Slow,
    /// never auto — button- or event-driven only.
    Manual,
}

impl Rate {
    /// TTL for this cadence. `Manual` has none — it never self-refreshes.
    pub fn ttl(self) -> Option<std::time::Duration> {
        use std::time::Duration;
        match self {
            Rate::Steady => Some(Duration::from_secs(25)),
            Rate::Slow => Some(Duration::from_secs(900)),
            Rate::Manual => None,
        }
    }
}

/// Metro-tile defaults at the fine 10×6 lattice — each card is 3-4 cells
/// wide and 2 cells tall, filling the grid exactly with zero holes.
pub fn default_dash_layout() -> Vec<CardLayout> {
    use DashCard::*;
    // 20×12 lattice — each card is a multiple of the fine cell, so a 1×1
    // is a true quarter-card tile (the old 10-col grid halved twice).
    [
        (System, 0, 0, 6, 4),
        (Weather, 6, 0, 6, 4),
        (Media, 12, 0, 8, 4),
        (Network, 0, 4, 6, 4),
        (CpuGpu, 6, 4, 6, 4),
        (Gauges, 12, 4, 8, 4),
        (Toggles, 0, 8, 6, 4),
        (Todo, 6, 8, 6, 4),
        (Sliders, 12, 8, 8, 4),
        // row 4
        (Disk, 0, 12, 5, 4),
        (ProcMon, 5, 12, 5, 4),
        (Mem, 10, 12, 5, 4),
        (TopProc, 15, 12, 5, 4),
        // row 5
        (ActiveWin, 0, 16, 5, 4),
        (Clipboard, 5, 16, 5, 4),
        (Notes, 10, 16, 5, 4),
        // row 6
        (PkgUpdates, 0, 20, 5, 4),
        (SshVpn, 5, 20, 5, 4),
        (Docker, 10, 20, 5, 4),
        (KbLayout, 15, 20, 5, 4),
        // row 7 — full-width scrollable wallpaper strip
        (Wallpaper, 0, 24, 20, 4),
        // row 8 — news feed + calendar + accent picker
        (News, 0, 28, 10, 4),
        (Calendar, 10, 28, 5, 4),
        (Accent, 15, 28, 5, 4),
        // row 9 — visualizer + sensors + mirror
        (Viz, 0, 32, 8, 4),
        (Sensors, 8, 32, 6, 4),
        (Mirror, 14, 32, 6, 4),
        // row 10 — lyrics of the current track (synced when available)
        (Lyrics, 0, 36, 10, 8),
        // row 11 — wifi / bluetooth / on-demand bandwidth test
        (Wifi, 0, 44, 6, 4),
        (Bluetooth, 6, 44, 6, 4),
        (SpeedTest, 12, 44, 8, 4),
        // row 12 — quote + currency
        (Quote, 0, 48, 10, 3),
        (Currency, 10, 48, 8, 3),
        // row 13 — prices ticker + recent files
        (Ticker, 0, 51, 10, 3),
        (Recent, 10, 51, 8, 3),
        // row 14 — GPU detail + thermal-zones grid
        (Gpu, 0, 55, 6, 4),
        (Thermal, 6, 55, 12, 4),
        // row 15 — clean clock/date + dual battery cards (vertical & horizontal)
        (Clock, 0, 60, 6, 4),
        (BatteryV, 6, 60, 5, 4),
        (BatteryH, 11, 60, 7, 4),
        // row 16 — world map (style panes: real/filled/dotted/stroked). The
        // map's 2.1:1 aspect needs height more than width (12×6 cells are
        // ~6:1), so it sits as a taller-than-wide card, fully swipeable.
        (WorldMap, 0, 64, 10, 9),
        // row 17 — minimalist light "paper" weather card (hero + strip).
        (WeatherV2, 0, 80, 8, 6),
        // row 18 — focus timer + fans + workspace tiles
        (Pomodoro, 8, 80, 6, 4),
        (Fans, 14, 80, 6, 4),
        (Workspaces, 0, 86, 6, 4),
        // row 19 — world clock (pinned cities + inline search)
        (WorldClock, 6, 86, 6, 4),
        // row 20 — water tracker + moon phase
        (Water, 12, 86, 4, 4),
        (Moon, 16, 86, 4, 4),
        // row 21 — compositor effects + countdown + alarms
        (Compositor, 0, 90, 6, 4),
        (Countdown, 6, 90, 7, 4),
        (Alarms, 13, 90, 7, 4),
        // row 22 — snippets + expenses
        (Snippets, 0, 94, 10, 4),
        (Expenses, 10, 94, 10, 4),
        // row 23 — eye rest + mic meter + audio device picker
        (EyeRest, 0, 98, 6, 4),
        (MicMeter, 6, 98, 4, 4),
        (AudioDevice, 10, 98, 10, 4),
        // row 24 — connection info + latency + power draw + disk health
        (ConnInfo, 0, 102, 5, 4),
        (Latency, 5, 102, 5, 4),
        (PowerDraw, 10, 102, 5, 4),
        (SmartHealth, 15, 102, 5, 4),
        // row 25 — failed units + journal tail
        (SystemdUnits, 0, 106, 7, 4),
        (JournalTail, 7, 106, 13, 4),
        // row 26 — brand sign (glyph from $states2, wide landscape card)
        (Branding, 0, 110, 20, 4),
    ]
    .into_iter()
    .map(|(card, x, y, w, h)| CardLayout { card, x, y, w, h })
    .collect()
}

/// Flow packer — cards pack left→right row by row. Compaction rules keep the
/// grid tidy without scrambling card order:
///
/// - **Always fill left.** Within its own row and the rows below, a card moves
///   as far LEFT as the first empty slot big enough for it — it is never kept
///   away from empty space it can fit into.
/// - **Jump up one row → rightmost slot.** Only *when its own row can't hold
///   it* (left fill failed), a lower card fills a gap in the row above (max
///   1 row up), landing in the *rightmost* empty slot — the leftmost card of
/// row 2 pops into the rightmost gap of row 1.
///
/// Collapse every all-empty ROW of a pinned dashboard layout so cards "fall
/// upward" into free space, keeping each card's column and vertical order.
/// Only rows with zero cards disappear — cards never shift sideways, never
/// overlap, and never move down.
pub fn compact_rows_up(cards: &[CardLayout]) -> Vec<CardLayout> {
    let mut occ: std::collections::HashSet<u16> = std::collections::HashSet::new();
    for l in cards {
        for r in l.y..l.y + l.h {
            occ.insert(r);
        }
    }
    let mut rows: Vec<u16> = occ.into_iter().collect();
    rows.sort_unstable();
    let rank = |y: u16| rows.partition_point(|&r| r < y);
    cards
        .iter()
        .map(|l| CardLayout { card: l.card, x: l.x, y: rank(l.y) as u16, w: l.w, h: l.h })
        .collect()
}

/// First-fit packer within a grid of `max_cols` columns (the dashboard uses
/// [`DASH_COLS`]; edit-mode "＋" grows it wider). Returns the packed
/// placements plus the row count the contents grew to.
pub fn pack_cards_max(cards: &[CardLayout], max_cols: u16) -> (Vec<CardLayout>, u16) {
    let cols = max_cols.max(1);
    let mut occ: std::collections::HashSet<(u16, u16)> = std::collections::HashSet::new();
    let mut out = Vec::with_capacity(cards.len());
    let mut maxr = 0u16;

    /// Try to place a w×h card on row `try_y`. Left-biased: scans columns
    /// 0..=hi and the LEFT-most free slot that fits wins. When `right_biased`,
    /// the scan goes hi..=0 and the RIGHT-most free slot wins.
    fn try_place(
        occ: &std::collections::HashSet<(u16, u16)>,
        try_y: u16, w: u16, h: u16,
        hi: u16, right_biased: bool,
    ) -> Option<(u16, u16)> {
        if try_y + h > DASH_MAX_ROWS { return None; }
        let iter: Box<dyn Iterator<Item = u16>> = if right_biased {
            Box::new((0..=hi).rev())
        } else {
            Box::new(0..=hi)
        };
        for x in iter {
            let mut fits = true;
            for dy in 0..h {
                for dx in 0..w {
                    if occ.contains(&(x + dx, try_y + dy)) {
                        fits = false;
                        break;
                    }
                }
                if !fits { break; }
            }
            if fits { return Some((x, try_y)); }
        }
        None
    }

    for c in cards {
        let w = c.w.clamp(1, cols);
        let h = c.h.max(1);
        let hi = cols.saturating_sub(w);
        let mut placed = false;

        // 1) current row — always fill the LEFT-most empty space
        if let Some((x, y)) = try_place(&occ, c.y, w, h, hi, false) {
            for dy in 0..h { for dx in 0..w { occ.insert((x + dx, y + dy)); } }
            out.push(CardLayout { card: c.card, x, y, w, h });
            maxr = maxr.max(y + h);
            continue;
        }
        // 2) jump up one row → RIGHTMOST empty slot (rightmost bias).
        //    Only fired when the current row is full (case 1 failed), and only
        //    into a row that already holds cards — never an empty band
        //    between card rows (which would scatter cards to a far side).
        if c.y > 0 {
            let above_has_cards =
                (0..cols).any(|xc| occ.contains(&(xc, c.y - 1)));
            if above_has_cards {
                if let Some((x, y)) = try_place(&occ, c.y - 1, w, h, hi, true) {
                    for dy in 0..h { for dx in 0..w { occ.insert((x + dx, y + dy)); } }
                    out.push(CardLayout { card: c.card, x, y, w, h });
                    maxr = maxr.max(y + h);
                    continue;
                }
            }
        }
        // 3) scan down from the original row, left-filling every row
        let mut y = c.y + 1;
        loop {
            if y + h > DASH_MAX_ROWS { break; }
            if let Some((x, yy)) = try_place(&occ, y, w, h, hi, false) {
                for dy in 0..h { for dx in 0..w { occ.insert((x + dx, yy + dy)); } }
                out.push(CardLayout { card: c.card, x, y: yy, w, h });
                maxr = maxr.max(yy + h);
                placed = true;
                break;
            }
            y += 1;
        }
        if !placed {
            // NEVER overlap (hard rule): falling back to `c.clone()` here
            // would paint the card on cells another card already holds. The
            // row immediately below the tallest content is always empty, so
            // place the card there — this is also how the canvas auto-grows
            // a new row when cards are added beyond the current extent.
            let base = maxr.max(c.y);
            let (x, y) = try_place(&occ, base, w, h, hi, false)
                .or_else(|| try_place(&occ, DASH_MAX_ROWS.saturating_sub(h), w, h, hi, false).into_iter().next())
                .expect("a row below all content is always free");
            for dy in 0..h { for dx in 0..w { occ.insert((x + dx, y + dy)); } }
            out.push(CardLayout { card: c.card, x, y, w, h });
            maxr = maxr.max(y + h);
        }
    }
    (out, maxr)
}

/// Metro ribbon pack (horizontal scroll mode): cards cascade into VERTICAL
/// columns — a column fills top→bottom, then the next column begins to the
/// RIGHT. Leftmost-fit scan (columns left→right, rows top→bottom inside a
/// column) produces a dense Windows-8-style Start-screen ribbon; the canvas
/// width grows with the ribbon, so horizontal mode overflows the viewport
/// and the bottom scroller pans it. Never overlaps.
fn pack_metro(cards: &[CardLayout], max_rows: u16) -> (Vec<CardLayout>, u16) {
    use std::collections::HashSet;
    let max_rows = max_rows.max(1);
    let mut occ: HashSet<(u16, u16)> = HashSet::new();
    let mut out = Vec::with_capacity(cards.len());
    let mut max_x = 0u16;
    let mut max_y = 0u16;
    for c in cards {
        let w = c.w.clamp(1, 512);
        let h = c.h.max(1);
        let mut placed = None;
        'cols: for x in 0..512u16 {
            for y in 0..=max_rows.saturating_sub(h) {
                let mut fits = true;
                for dy in 0..h {
                    for dx in 0..w {
                        if x + dx >= 512 || occ.contains(&(x + dx, y + dy)) {
                            fits = false;
                            break;
                        }
                    }
                    if !fits { break; }
                }
                if fits {
                    placed = Some((x, y));
                    break 'cols;
                }
            }
        }
        let (x, y) = placed.unwrap_or((max_x, 0));
        for dy in 0..h {
            for dx in 0..w {
                occ.insert((x + dx, y + dy));
            }
        }
        max_x = max_x.max(x + w);
        max_y = max_y.max(y + h);
        out.push(CardLayout { card: c.card, x, y, w, h });
    }
    (out, max_y.max(1))
}

#[cfg(test)]
mod metro_pack_tests {
    use super::*;

    #[test]
    fn metro_cascades_into_vertical_columns() {
        let cards: Vec<CardLayout> = (0..6)
            .map(|_| CardLayout { card: DashCard::Cpu, x: 0, y: 0, w: 2, h: 2 })
            .collect();
        let (placed, rows) = pack_metro(&cards, 4);
        // col height 4 fits exactly two 2-high cards per column; the ribbon
        // then extends RIGHT: (0,0),(0,2) | (2,0),(2,2) | (4,0),(4,2)
        let want: Vec<(u16, u16)> = vec![(0, 0), (0, 2), (2, 0), (2, 2), (4, 0), (4, 2)];
        let got: Vec<(u16, u16)> = placed.iter().map(|l| (l.x, l.y)).collect();
        assert_eq!(got, want);
        assert_eq!(rows, 4);
        // hard rule: cards never overlap
        let mut occ: std::collections::HashSet<(u16, u16)> = std::collections::HashSet::new();
        for l in &placed {
            for dy in 0..l.h {
                for dx in 0..l.w {
                    assert!(occ.insert((l.x + dx, l.y + dy)), "overlap at {}x{}", l.x + dx, l.y + dy);
                }
            }
        }
    }
}

/// Auto-arrange compaction: shifts each card as far LEFT as fits within
/// `cols`, preserving row band, size and relative order (never moves a card
/// up/down, never reorders). Pure + deterministic — every card is placed into
/// the leftmost empty column of its own rows when a gap opens up. Returns the
/// compacted placements and whether anything moved.
pub fn compact_layout(cards: &[CardLayout], cols: u16) -> (Vec<CardLayout>, bool) {
    let cols = cols.max(1);
    let mut order: Vec<usize> = (0..cards.len()).collect();
    order.sort_by_key(|&i| (cards[i].y, cards[i].x));
    let mut occ: std::collections::HashSet<(u16, u16)> = std::collections::HashSet::new();
    let mut out: Vec<CardLayout> = Vec::with_capacity(cards.len());
    let mut moved = false;
    for i in order {
        let c = cards[i];
        let w = c.w.clamp(1, cols);
        let h = c.h.max(1);
        // leftmost column 0..=c.x where the card's own-row rectangle fits
        let mut best = c.x.min(cols.saturating_sub(w));
        'scan: for cand in 0..=best {
            if cand + w > cols {
                break;
            }
            for dy in 0..h {
                for dx in 0..w {
                    if occ.contains(&(cand + dx, c.y + dy)) {
                        continue 'scan;
                    }
                }
            }
            best = cand;
            break;
        }
        for dy in 0..h {
            for dx in 0..w {
                occ.insert((best + dx, c.y + dy));
            }
        }
        if best != c.x {
            moved = true;
        }
        out.push(CardLayout { card: c.card, x: best, y: c.y, w, h });
    }
    (out, moved)
}

/// One gravity step: the LEFTMOST card that can climb ONE row lifts UP into
/// the row above's LEFTMOST free slot its full width fits (leftmost-below →
/// top-left-above, even if the slot isn't in the card's own column — a card
/// NEVER sits to the right of a free cell it could occupy). Exactly one card
/// moves per call — the outer relaxation loops this until stable so the
/// motion cascades bottom→top like a gravity feed.
fn up_step(cards: &[CardLayout], cols: u16) -> (Vec<CardLayout>, bool) {
    let cols = cols.max(1);
    let mut occ: std::collections::HashSet<(u16, u16)> = std::collections::HashSet::new();
    for c in cards {
        let w = c.w.clamp(1, cols);
        for dy in 0..c.h.max(1) {
            for dx in 0..w {
                occ.insert((c.x + dx, c.y + dy));
            }
        }
    }
    // pick the best candidate: the LEFTMOST card (lowest x; among equal x the
    // one further BELOW) that has a slot free in the row above
    let mut pick: Option<usize> = None;
    let mut pick_x = 0u16;
    for (i, c) in cards.iter().enumerate() {
        if c.y == 0 {
            continue;
        }
        let w = c.w.clamp(1, cols);
        let h = c.h.max(1);
        // LEFTMOST x in the row above where this card's whole band fits
        let mut fits_any = false;
        let mut bx = c.x;
        for cand in 0..=cols.saturating_sub(w) {
            let mut fits = true;
            for dy in 0..h {
                for dx in 0..w {
                    if occ.contains(&(cand + dx, c.y - 1 + dy)) {
                        fits = false;
                        break;
                    }
                }
                if !fits {
                    break;
                }
            }
            if fits {
                fits_any = true;
                bx = cand;
                break;
            }
        }
        if !fits_any {
            continue;
        }
        match pick {
            None => {
                pick = Some(i);
                pick_x = bx;
            }
            Some(p) => {
                let pc = &cards[p];
                if c.x < pc.x || (c.x == pc.x && c.y > pc.y) {
                    pick = Some(i);
                    pick_x = bx;
                }
            }
        }
    }
    let Some(pi) = pick else {
        return (cards.to_vec(), false);
    };
    let c = cards[pi];
    let mut out = cards.to_vec();
    out[pi] = CardLayout { card: c.card, x: pick_x, y: c.y - 1, w: c.w.clamp(1, cols), h: c.h.max(1) };
    (out, true)
}

/// Full auto-arrange pass (HARD RULES): cards gravitate toward the top-left.
/// Alternates up-fill (bottom → top, leftmost-below → rightmost-above) with
/// left-fill (right → left cascade) to a fixpoint. A card never moves right
/// into empty space, never jumps down, never hops to an arbitrary cell.
pub fn gravity_pack(cards: &[CardLayout], cols: u16) -> (Vec<CardLayout>, bool) {
    let mut cur = cards.to_vec();
    let mut moved = false;
    // monotone: y only decreases, so the relaxation is bounded
    let maxit = (cur.len() + 1) * (DASH_MAX_ROWS as usize + 1) + 1;
    for _ in 0..maxit {
        // climb FIRST (bottom→top), then slide-left compact the NEW state so
        // the up-fill is never clobbered by the previous row positions
        let (nu, mu) = up_step(&cur, cols);
        let (nl, ml) = compact_layout(&nu, cols);
        if mu || ml {
            moved = true;
        }
        cur = nl;
        if !mu && !ml {
            break;
        }
    }
    (cur, moved)
}

/// Cell pitch helpers (pixels) — shared by the layout and input handlers.
/// Cell SIZE is anchored to the CONFIGURED dashboard width, NOT the live
/// canvas width: the canvas is auto-trimmed/grown to the packed content, so
/// deriving cells from it would feed back (shrink → reshrink → …). With a
/// fixed anchor the lattice never stretches — full-width layouts reproduce
/// exactly the configured geometry.
/// Pixel rect of a card placement (delegates to Shell method).
pub fn dash_card_px(l: CardLayout, cw: f32, ch: f32, grid_top: f32, dash_pad: f32, dash_gap: f32) -> (f32, f32, f32, f32) {
    (
        dash_pad + l.x as f32 * (cw + dash_gap),
        grid_top + l.y as f32 * (ch + dash_gap),
        l.w as f32 * cw + (l.w.max(1) - 1) as f32 * dash_gap,
        l.h as f32 * ch + (l.h.max(1) - 1) as f32 * dash_gap,
    )
}
/// to-do hit keys: 60+i toggle done · 70+i delete · 78 focus the composer
pub(crate) const TODO_KEY_TOGGLE: u32 = 60;
pub(crate) const TODO_KEY_DELETE: u32 = 70;
pub(crate) const TODO_KEY_INPUT: u32 = 78;
/// `+` add button on the To-Do card (below the clip keys which start at 80).
pub(crate) const TODO_KEY_ADD: u32 = 79;
/// pomodoro card hit keys
pub(crate) const POMODORO_TOGGLE_KEY: u32 = 30_730;
pub(crate) const POMODORO_RESET_KEY: u32 = 30_731;
pub(crate) const POMODORO_INC_KEY: u32 = 30_732;
pub(crate) const POMODORO_DEC_KEY: u32 = 30_733;
/// workspaces card tiles: base + 0-based workspace index
pub(crate) const WORKSPACE_KEY_BASE: u32 = 30_800;
pub(crate) const WORKSPACE_KEY_MAX: u32 = WORKSPACE_KEY_BASE + 63;
/// world-clock card: row ✕ remove · "+" add · search-result pick base
pub(crate) const WORLDCLOCK_DEL_BASE: u32 = 31_000;
pub(crate) const WORLDCLOCK_DEL_MAX: u32 = WORLDCLOCK_DEL_BASE + 15;
pub(crate) const WORLDCLOCK_ADD_KEY: u32 = 31_100;
pub(crate) const WORLDCLOCK_PICK_BASE: u32 = 31_200;
pub(crate) const WORLDCLOCK_PICK_MAX: u32 = WORLDCLOCK_PICK_BASE + 15;
/// water tracker card: + glass · − undo
pub(crate) const WATER_INC_KEY: u32 = 31_300;
pub(crate) const WATER_DEC_KEY: u32 = 31_301;
/// compositor card: blur · shadow · opacity tiles + shot area/full
pub(crate) const COMP_BLUR_KEY: u32 = 31_400;
pub(crate) const COMP_SHADOW_KEY: u32 = 31_401;
pub(crate) const COMP_OPACITY_KEY: u32 = 31_402;
pub(crate) const COMP_SHOT_AREA_KEY: u32 = 31_403;
pub(crate) const COMP_SHOT_FULL_KEY: u32 = 31_404;
/// countdown card: composer focus · row delete (base+i) · row click cycle
pub(crate) const COUNTDOWN_INPUT_KEY: u32 = 31_500;
pub(crate) const COUNTDOWN_DEL_BASE: u32 = 31_510;
pub(crate) const COUNTDOWN_DEL_MAX: u32 = COUNTDOWN_DEL_BASE + 15;
/// alarms card: composer focus · row delete
pub(crate) const ALARM_INPUT_KEY: u32 = 31_600;
pub(crate) const ALARM_DEL_BASE: u32 = 31_610;
pub(crate) const ALARM_DEL_MAX: u32 = ALARM_DEL_BASE + 15;
/// snippets card: composer focus · row copy · row delete
pub(crate) const SNIPPET_INPUT_KEY: u32 = 31_700;
pub(crate) const SNIPPET_COPY_BASE: u32 = 31_710;
pub(crate) const SNIPPET_COPY_MAX: u32 = SNIPPET_COPY_BASE + 15;
pub(crate) const SNIPPET_DEL_BASE: u32 = 31_730;
pub(crate) const SNIPPET_DEL_MAX: u32 = SNIPPET_DEL_BASE + 15;
/// expenses card: quick-add amount chips · composer (custom amount)
pub(crate) const EXPENSE_CHIP_BASE: u32 = 31_800;
pub(crate) const EXPENSE_CHIP_MAX: u32 = EXPENSE_CHIP_BASE + 7;
pub(crate) const EXPENSE_INPUT_KEY: u32 = 31_810;
pub(crate) const EXPENSE_CLEAR_KEY: u32 = 31_811;

/// eye rest card: pause/resume · reset timer
pub(crate) const EYEREST_TOGGLE_KEY: u32 = 31_900;
pub(crate) const EYEREST_RESET_KEY: u32 = 31_901;
/// mic meter card: click glyph mutes/unmutes the input
pub(crate) const MIC_MUTE_KEY: u32 = 31_910;
/// audio device card: row = set default, right mute glyph = toggle mute
/// (base + 2*row, base + 2*row + 1)
pub(crate) const AUDIO_DEV_BASE: u32 = 31_920;
pub(crate) const AUDIO_DEV_MAX: u32 = AUDIO_DEV_BASE + 24;
/// conninfo card: refresh
pub(crate) const CONNINFO_REFRESH_KEY: u32 = 31_960;
/// latency card: re-probe now
pub(crate) const LATENCY_RUN_KEY: u32 = 31_970;
/// smarthealth card: refresh smart data
pub(crate) const SMART_REFRESH_KEY: u32 = 31_980;
/// systemd units card: refresh failed list
pub(crate) const SYSTEMD_REFRESH_KEY: u32 = 31_990;
/// journal tail card: refresh
pub(crate) const JOURNAL_REFRESH_KEY: u32 = 31_995;

// ── audio recorder card (BASE range 32_300) ──
/// mic level fader (drag key, uses `audiorec_mic_rect` geometry)
pub(crate) const AUDIO_REC_MIC_KEY: u32 = 32_300;
/// record/stop button
pub(crate) const AUDIO_REC_REC_KEY: u32 = 32_301;
/// chevron "›" — open the recordings pane (pane 1)
pub(crate) const AUDIO_REC_PANE_KEY: u32 = 32_302;
/// chevron "‹" — back to the recorder pane (pane 0)
pub(crate) const AUDIO_REC_BACK_KEY: u32 = 32_303;
/// confirm-delete toggle row (pane 1)
pub(crate) const AUDIO_REC_CONFIRM_KEY: u32 = 32_304;
/// show-mic-slider toggle row (pane 1)
pub(crate) const AUDIO_REC_SHOWMIC_KEY: u32 = 32_305;
/// recordings list rows: play (base+i) · delete (base+0x20+i)
pub(crate) const AUDIO_REC_PLAY_BASE: u32 = 32_310;
pub(crate) const AUDIO_REC_PLAY_MAX: u32 = AUDIO_REC_PLAY_BASE + 15;
pub(crate) const AUDIO_REC_DEL_BASE: u32 = 32_330;
pub(crate) const AUDIO_REC_DEL_MAX: u32 = AUDIO_REC_DEL_BASE + 15;

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
    /// audio_main mic toggle — mute/unmute the input
    MicToggle,
    /// shader_main set N (saturation 0..200, 100 = normal)
    Saturation(i32),
    /// hs_main set K (Kelvin)
    Kelvin(i32),
    CaffeineToggle,
    SunsetToggle,
    ShaderToggle,
    /// blur_main toggle (effects card)
    BlurToggle,
    /// shadow_main toggle (effects card)
    ShadowToggle,
    /// opacity_main toggle (effects card)
    OpacityToggle,
}

// Catppuccin Mocha derived shades (same as the C++ palette)
const FG2: u32 = 0xa6adc8ff;
const FG3: u32 = 0x7f849cff;
const SFG: u32 = 0x1e1e2eff; // text on acc
const RED: u32 = ui::DANGER; // low battery / destructive
const GREEN: u32 = ui::OK;   // charging / ok

use crate::ui::{self, mix, Pal};

/// Plain cubic ease-out — smooth settle, no overshoot. Used for both the
/// grow (expand) and shrink (collapse) morphs so neither direction bounces.
fn ease_out_cubic(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(3)
}

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
            Lock => (0.0, 0.0), // real size = monitor, set in target_size()
            PolkitAuth => (420.0, 320.0),
        }
    }
}

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

/// System hostname from `/etc/hostname` or `gethostname`.
fn hostname() -> String {
    std::fs::read_to_string("/etc/hostname")
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| {
            let mut buf = [0u8; 256];
            unsafe { libc::gethostname(buf.as_mut_ptr() as *mut libc::c_char, buf.len()); }
            let n = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
            String::from_utf8_lossy(&buf[..n]).to_string()
        })
}

/// Current username from `$USER` or `getlogin`.
fn whoami() -> String {
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
fn uptime_str() -> String {
    // /proc/uptime: "<idle> <total>" in seconds
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
/// (fallback `~/.local/state/zen-shell/notifs.json`).
pub fn notif_path() -> std::path::PathBuf {
    let base = std::env::var("XDG_STATE_HOME")
        .map(|d| std::path::PathBuf::from(d))
        .or_else(|_| std::env::var("HOME").map(|h| std::path::PathBuf::from(h).join(".local/state")))
        .unwrap_or_else(|_| std::path::PathBuf::from("/tmp"));
    base.join("zen-shell").join("notifs.json")
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
    /// Rounded rect with per-corner radii — negative = concave (Noctalia-style screen-edge carve).
    RectConcave { x: f32, y: f32, w: f32, h: f32, r_tl: f32, r_tr: f32, r_br: f32, r_bl: f32, color: u32 },
    /// Stroked (outline-only) rounded rect — `width` is the stroke thickness
    /// centered on the edge. Vector icons (battery body, drag outlines).
    Outline { x: f32, y: f32, w: f32, h: f32, r: f32, width: f32, color: u32 },
    /// Thick line segment (capsule) from (x0,y0) to (x1,y1), `w` px thick —
    /// heartbeat-style graph lines.
    Line { x0: f32, y0: f32, x1: f32, y1: f32, w: f32, color: u32 },
    Text {
        x: f32, y: f32, right: bool, center: bool,
        text: String, size: f32, color: u32, icon: bool,
        f: crate::text::Ff, w: crate::text::Fw,
    },
    /// Rasterized image (app icon, tray pixmap) fit inside a `w`×`h` box
    /// (aspect-preserving, downscale-only), centered in that box.
    /// `key` is an `ImageStore` key (`icon:<name>` / `file:<path>` / tray id).
    Image { x: f32, y: f32, w: f32, h: f32, key: String },
    /// World-map texture: ONE cached band raster drawn as a UV-windowed quad
    /// fitted to the body rect. Pan/zoom only moves the UV window (4 floats)
    /// — the heavy raster is re-baked only when the view leaves the band.
    /// `colors` = `[raised, land, rings, hairline, dots, region]`.
    Map {
        x: f32, y: f32, w: f32, h: f32,
        lon0: f32, lat0: f32, lon1: f32, lat1: f32,
        zoom: f32,
        style: u8,
        colors: [u32; 6],
        region: bool,
        version: u64,
    },
    /// Restrict all subsequent drawing to a pixel rect (scene coords). Clips
    /// overflow inside a region — per-zone banner strip scrolling clips each
    /// L/C/R zone so scrolled-out chips never bleed into their neighbours.
    Scissor { x: f32, y: f32, w: f32, h: f32 },
    /// End the current scissor; drawing is unrestricted again.
    ScissorEnd,
}

pub struct Anim {
    pub from_w: f32,
    pub from_h: f32,
    pub to_w: f32,
    pub to_h: f32,
    pub start: Instant,
    pub dur: f32,
}

pub struct Shell {
    pub cfg: Config,
    /// uniform zoom factor — scales all grid geometry, fonts, radii
    pub scale: GridScale,
    pub hover: bool,
    pub mode: Mode,
    pub prev_mode: Mode,

    // pointer hover (updated on motion; drives per-element highlight)
    pub cursor: Option<(f32, f32)>,
    /// key of the interactive element under the cursor (0 = none)
    pub hover_key: u32,
    /// hot rects from the last layout pass — shared by hover, click, media
    hover_regions: Vec<(f32, f32, f32, f32, u32)>,

    // live state (services)
    pub clock: String,
    pub ws_count: usize,
    pub ws_active: usize,
    pub battery: i32,
    /// battery power draw in watts (upower EnergyRate); 0 / absent → hidden
    pub battery_watts: f32,
    /// watts value shown on the pill (battery_watts sampled once a minute)
    pub watts_shown: f32,
    /// last time `watts_shown` was refreshed
    pub watts_at: Option<Instant>,
    /// estimated remaining time ("2h 15m" / "Charged" / "")
    pub battery_time: String,
    pub ssid: String,
    /// available Wi-Fi networks (ssid, strength 0-100, secured, connected)
    pub wifi_networks: Vec<(String, u8, bool, bool)>,
    /// bluetooth devices (name, connected, paired)
    pub bt_devices: Vec<(String, bool, bool)>,
    pub notif_count: u32,
    /// system tray items (StatusNotifier), left→right = right→left visually
    pub tray: Vec<TrayItem>,
    pub title: String,
    pub apps: AppManager,

    // media (MPRIS)
    pub media_title: String,
    pub media_artist: String,
    pub media_album: String,
    pub media_playing: bool,
    /// track position in µs (sampled by the poll; interpolated while playing)
    pub media_pos: i64,
    /// track length in µs (0 = unknown / no track)
    pub media_len: i64,
    /// when `media_pos` was sampled (drives the interpolation)
    pub media_pos_at: Instant,
    /// local album-art path (from `mpris:artUrl`, file:// only); empty → none
    pub media_art: String,
    /// seek-drag fraction 0..=1 while the user drags the track bar (None at
    /// rest — the bar then shows the interpolated playhead)
    pub media_seek: Option<f32>,
    /// animated spectrum bars (0..=1), ticked by the app off the live capture
    pub viz: Vec<f32>,
    /// visualizer bar style (0 = rounded bars, 1 = wave, 2 = blocks) — the
    /// Viz card's style button cycles it; persisted via `cfg.viz_style`
    pub viz_style: u8,
    /// Slow-decaying peak magnitude used by the auto-gain in viz_tick so
    /// bars consistently span 0..1 even at lower volumes (cava autosens).
    pub viz_peak: f32,
    /// audio equalizer state (Arc<Mutex<…>> for cross-thread access)
    pub eq: crate::eq::EqState,
    /// last frame's EQ-card rect (hit-testing)
    pub eq_rect: (f32, f32, f32, f32),
    /// EQ card: dragged band index (None = idle)
    pub eq_drag: Option<usize>,
    /// latest headlines for the News card (pumped in from the app worker)
    pub news_items: Vec<crate::news::NewsItem>,
    /// last frame's News-card rect (wheel hit-testing for card-local scroll)
    pub news_rect: (f32, f32, f32, f32),
    /// News-card headline scroll offset (rows, 0 = top)
    pub news_scroll: usize,
    /// active News category chip: 0 = All, 1..N = feed category
    pub news_active_cat: usize,
    /// horizontal (category strip) scroll offset in px
    pub news_cat_scroll: f32,
    /// total width of the rendered category chips (clamp target)
    pub news_cat_content_w: f32,
    /// story URL to open in the browser (clicked a headline)
    pub pending_news_open: Option<String>,
    /// true once the first fetch landed (distinguishes "fetching" from "off")
    pub news_fetched: bool,
    /// lyric lines for the current track (empty = none yet / not found)
    pub lyrics_lines: Vec<crate::lyrics::LyricLine>,
    /// `artist || title` the loaded lyrics belong to — a new track resets it
    /// and schedules a fresh background fetch
    pub lyrics_track: String,
    /// 0 = idle/unknown · 1 = fetching · 2 = loaded · 3 = not found
    pub lyrics_state: u8,
    /// Lyrics-card scroll offset (rows, 0 = top)
    pub lyrics_scroll: usize,
    /// Smooth scroll target for lyrics auto-follow (fractional rows)
    pub lyrics_scroll_smooth: f32,
    /// last frame's Lyrics-card rect (wheel hit-testing for card-local scroll)
    pub lyrics_rect: (f32, f32, f32, f32),

    /// wifi card: scrolled network rows + hit rect
    pub wifi_rect: (f32, f32, f32, f32),
    pub wifi_scroll: usize,
    /// bluetooth card: scrolled device rows + hit rect
    pub bt_rect: (f32, f32, f32, f32),
    pub bt_scroll: usize,
    /// bandwidth-test card: hit rect, 0=idle 1=running 2=done,
    /// string results ("x.xx Mbps" / "-"), ping ms
    pub speed_rect: (f32, f32, f32, f32),
    pub speed_state: u8,
    pub speed_down: String,
    pub speed_up: String,
    pub speed_ping: u32,
    /// recent-files card: scrolled rows + cached xbel listing (name, path)
    pub recent_rect: (f32, f32, f32, f32),
    pub recent_scroll: usize,
    pub recent_files: Vec<(String, String)>,
    pub recent_loaded_at: std::time::Instant,
    /// currency card: hit rect, scroll, active base symbol + stale-rates note
    pub currency_rect: (f32, f32, f32, f32),
    pub currency_scroll: usize,
    pub currency_base: String,
    pub currency_note: String,
    /// quote card: hit rect, current text/author, 0=idle 1=fetching 2=ok 3=err,
    /// save-flash frame counter + persisted quotes from $states
    pub quote_rect: (f32, f32, f32, f32),
    pub quote_text: String,
    pub quote_author: String,
    pub quote_state: u8,
    pub quote_saved_flash: u32,
    pub saved_quotes: Vec<String>,
    /// ticker card: hit rect, scroll, (symbol, price, change-pct) rows, state
    pub ticker_rect: (f32, f32, f32, f32),
    pub ticker_scroll: usize,
    pub ticker_items: Vec<(String, String, f32)>,
    pub ticker_state: u8,
    /// when the ticker was last fetched (auto-refresh cadence in app.rs)
    pub ticker_last_at: std::time::Instant,
    /// latest IIO sensor sample (accelerometer / gyro / compass)
    pub sensors: crate::sensors::Sensors,
    /// true while the mirror card has a live camera frame to show
    pub cam_ok: bool,
    /// active mirror-camera capture rate (fps) — 0 when the feed is down
    pub cam_rate: u32,
    /// requested mirror-camera capture rate (fps) — the Mirror card's fps
    /// button cycles 15/30/60; persisted via `cfg.mirror_fps`
    pub mirror_fps: u32,
    /// show the actual delivered fps next to the fps label (persisted via
    /// `cfg.mirror_show_fps`) — on by default
    pub mirror_show_fps: bool,
    /// active pane of the Mirror card (0 = feed + photo/record, 1 = fps
    /// settings rows + show-real-fps toggle); horizontal swipe flips
    pub mirror_pane: usize,
    /// last drawn Mirror-card bounds (x, y, w, h) — for the swipe hit-test
    pub mirror_rect: (f32, f32, f32, f32),
    /// true while the mirror card is recording the live feed to a video file
    pub mirror_rec: bool,
    /// elapsed recording time in seconds (drives the "● Stop 00:12" button)
    pub mirror_rec_secs: u32,
    /// show the audio balance slider row on the Sliders card (pane-2 toggle,
    /// persisted via `cfg.show_balance`) — on by default
    pub show_balance: bool,
    /// show the saturation slider row on the Sliders card (pane-2 toggle,
    /// persisted via `cfg.show_saturation`) — on by default
    pub show_saturation: bool,
    /// show the TITLE text on dashboard card headers (edit-mode strip-editor
    /// toggle "Card titles", persisted via `cfg.card_show_title`) — on by
    /// default
    pub card_show_title: bool,
    /// show the header GLYPH (icon) on dashboard cards (edit-mode
    /// strip-editor toggle "Card glyphs", persisted via `cfg.card_show_glyph`)
    /// — on by default
    pub card_show_glyph: bool,
    /// active pane of the Sliders card (0 = fader rows, 1 = show-balance /
    /// show-saturation toggles); horizontal swipe flips
    pub sliders_pane: usize,
    /// last drawn Sliders-card bounds (x, y, w, h) — for the swipe hit-test
    pub sliders_rect: (f32, f32, f32, f32),
    // ── audio recorder card ──
    /// active pane (0 = recorder, 1 = recordings list); chevron / swipe flips
    pub audiorec_pane: usize,
    /// last drawn AudioRec-card bounds (x, y, w, h) — for the swipe hit-test
    pub audiorec_rect: (f32, f32, f32, f32),
    /// mic fader track geometry (x, cy, w) — used by `set_dash_slider` for the
    /// recorder card's own mic slider drag key AUDIO_REC_MIC_KEY
    pub audiorec_mic_rect: (f32, f32, f32),
    /// true while the app's pw-record child is running
    pub audiorec_recording: bool,
    /// elapsed recording time in seconds (drives the live "● Stop mm:ss")
    pub audiorec_rec_secs: u32,
    /// saved recordings as `path -> short name` (short name = file stem),
    /// listed newest first by the app
    pub audiorec_recordings: Vec<(String, String)>,
    /// pane-1 toggle: demand a confirm step before deleting a file (default on)
    pub audiorec_confirm_delete: bool,
    /// pane-1 toggle: show the mic level slider on pane 0 (default on)
    pub audiorec_show_mic: bool,
    /// pending record button pressed — the app toggles the pw-record child
    pub pending_audiorec_rec: bool,
    /// pending "refresh the recordings list" (pane opened / after stop) —
    /// the app re-scans the recordings dir
    pub pending_audiorec_list: bool,
    /// pending play of `audiorec_recordings[i]` (app spawns pw-play)
    pub pending_audiorec_play: Option<usize>,
    /// pending delete of `audiorec_recordings[i]` (app removes the file)
    pub pending_audiorec_del: Option<usize>,
    /// recordings row armed for delete (index into `audiorec_recordings`);
    /// a second delete click on a confirmed-delete card deletes the file.
    pub audiorec_del_armed: Option<usize>,

    // panels
    pub query: String,
    /// theme-picker search filter (Mode::Themes)
    pub themes_query: String,
    pub sel: usize,
    /// Cached launcher hits (`apps.apps` indices); refreshed only when query changes.
    launcher_hits: Vec<usize>,
    launcher_hits_query: String,
    /// scroll offset into the launcher hit list (wheel / arrows)
    launcher_scroll: usize,
    /// slider being dragged (set on press, cleared on release)
    pub drag_key: Option<u32>,
    pub notifs: Vec<Notif>,
    pub dnd: bool,
    pub cal_offset: i32,
    pub cal_selected: i32,
    pub wifi_on: bool,
    pub bt_on: bool,
    pub volume: f32,
    pub brightness: f32,
    /// default-sink muted (for the OSD / CC display)
    pub volume_muted: bool,
    /// set by the CC/Settings volume slider; app.rs applies it to the backend
    pub pending_volume: Option<f32>,
    /// per-channel (L/R) volume relative to the main mixer — 0..=1, mirrored
    /// when the main slider moves unless the user has adjusted one channel
    pub vol_left: f32,
    pub vol_right: f32,
    /// set by the sliders-card L/R channel sliders; app.rs applies to backend
    pub pending_vol_left: Option<f32>,
    pub pending_vol_right: Option<f32>,
    /// allow the main mixer volume to exceed 1.0 (100%); backed by wpctl's
    /// fractional set-volume which accepts values > 1.0
    pub allow_over_100: bool,
    /// max volume cap in hundredths (0..200 = 0%..200%); written to
    /// `$states/max_vol` by the Settings max-volume slider; read by the
    /// bash `audio_body` `vol_set`/`vol_step` to clamp the user-visible value
    pub max_vol: i32,
    /// set by the Settings max-volume slider; app.rs writes `$states/max_vol`
    pub pending_max_vol: Option<i32>,
    /// set by the CC/Settings brightness slider; app.rs writes it to the backend
    pub pending_brightness: Option<f32>,
    /// default-microphone level 0..=1 (dashboard vertical slider; dash_status poll)
    pub mic_level: f32,
    /// default microphone muted (dash_status poll)
    pub mic_muted: bool,
    /// caffeine / idle-inhibit active (caffeine_main)
    pub caffeine_on: bool,
    /// hyprsunset running (state a|m from hs_main status)
    pub sunset_on: bool,
    /// manual color temperature in Kelvin (hs_main)
    pub sunset_kelvin: i32,
    /// vibrance shader active (shader_main status)
    pub shader_on: bool,
    /// saturation level 0..200 (shader_main set N; 100 = normal)
    pub saturation: i32,
    /// script-backed dashboard actions queued for app.rs to run
    pub pending_dash: Option<DashCmd>,
    /// AC adapter online (from the power backend; OSD on change)
    pub ac_online: bool,
    /// caps-lock state (from xkb locked modifiers; shown on the lockscreen)
    pub caps: bool,
    /// monitor size in logical px (set by the app before locking)
    pub monitor: (f32, f32),
    /// active OSD, if any
    pub osd: Option<OsdInfo>,

    // power menu hold-to-confirm (configurable fill)
    /// power button key (1..=5) currently held, if any
    pub power_hold: Option<u32>,
    /// when the hold started (drives the fill)
    pub power_hold_start: Instant,

    // audio (wpctl, polled only while the Audio panel is open)
    /// output devices: (wpctl id, name, volume 0-1, muted)
    pub audio_sinks: Vec<(u32, String, f32, bool)>,
    /// input devices: (wpctl id, name, volume 0-1, muted)
    pub audio_sources: Vec<(u32, String, f32, bool)>,
    /// per-app streams: (wpctl id, name, volume 0-1, muted, is_input)
    pub audio_streams: Vec<(u32, String, f32, bool, bool)>,
    /// set by an Audio row click; app.rs runs wpctl (id, volume 0-1)
    pub pending_audio_volume: Option<(u32, f32)>,
    /// set by an Audio mute click; app.rs runs wpctl (id)
    pub pending_audio_mute: Option<u32>,
    /// set by an AudioDevice card row click; app.rs runs wpctl set-default
    pub pending_audio_default: Option<u32>,
    /// id of the starred default sink (wpctl status "Sinks:" list) — the
    /// AudioDevice card draws the star next to it.
    pub audio_default: u32,

    // eye rest (20-20-20) ------------------------------------------------
    /// timer running (focus phase) or paused
    pub er_running: bool,
    /// when the current focus stretch started
    pub er_start: Option<Instant>,
    /// remaining focus seconds while paused (None when running/resting)
    pub er_paused_rem: Option<u64>,
    /// while a rest break is active, when it ends (None otherwise)
    pub er_rest_until: Option<Instant>,
    /// completed rest breaks, persisted to `eyerest.txt`
    pub er_sessions: u64,

    // latency monitor ------------------------------------------------------
    /// ring buffer of probe times (ms), newest last
    pub lat_history: std::collections::VecDeque<u32>,
    /// last probe result in ms
    pub lat_current: u32,
    /// 0 idle · 1 probing · 2 ok · 3 no reply/error
    pub lat_state: u8,
    /// throttle: only re-probe past this instant (set by poll_latency)
    pub lat_next: Option<Instant>,

    // power draw -----------------------------------------------------------
    /// (battery watts, gpu watts) samples, newest last
    pub pw_history: std::collections::VecDeque<(f32, f32)>,
    /// throttle for the next power-draw sample
    pub pw_next: Option<Instant>,

    // disk health (smartctl) ------------------------------------------------
    /// (dev, model, passed, temp C)
    pub sm_disks: Vec<(String, String, bool, i32)>,
    /// throttle for the next smartctl scan
    pub sm_next: Option<Instant>,

    // failed systemd units ---------------------------------------------------
    pub sus_failed: Vec<String>,
    /// throttle for the next systemctl scan
    pub sus_next: Option<Instant>,

    // journal tail -----------------------------------------------------------
    /// (message, priority 3=err 4=warn 5=notice), oldest first
    pub jr_lines: Vec<(String, u8)>,
    /// throttle for the next journalctl read
    pub jr_next: Option<Instant>,

    // connection info ---------------------------------------------------------
    /// label/value rows: interface IPs, default gateway, DNS resolvers
    pub conn_rows: Vec<(String, String)>,

    // wallpaper picker (refreshed when the panel opens)
    /// absolute paths of the wallpapers in `cfg.wallpaper.dir`
    pub wallpapers: Vec<String>,
    /// scroll offset into the wallpaper grid (wheel / arrows / scrollbar)
    pub wallpaper_scroll: usize,
    /// card-local scroll offset for the dashboard Wallpaper card grid
    /// (independent of the full picker's `wallpaper_scroll`)
    pub wp_card_scroll: usize,
    /// effective tile cell for the dashboard Wallpaper card, recomputed every
    /// frame so the grid always fills the card (0 = not drawn yet → helper
    /// falls back to the preferred `WP_CARD_CELL`)
    pub wp_card_cell_eff: f32,
    /// picker thumbnail cell size (Ctrl+= / Ctrl+- zooms; persisted)
    pub wp_cell: f32,
    /// wallpaper scrollbar grab-drag active (press started on the strip)
    pub wp_sb_drag: bool,
    /// where inside the thumb the grab caught (px from the thumb top)
    pub wp_sb_grab: f32,
    /// theme-grid scrollbar grab-drag active (press started on the strip)
    pub th_sb_drag: bool,
    /// where inside the theme thumb the grab caught (px from the thumb top)
    pub th_sb_grab: f32,
    /// set by a thumbnail click; app.rs spawns the set command
    pub pending_wallpaper: Option<String>,

    // themes panel + live $states2 colors ---------------------------------
    /// cards parsed from $states2/themes JSON mirror (refreshed on open)
    pub themes: Vec<crate::themes::ThemeCard>,
    /// scroll offset into the theme card grid (wheel)
    pub themes_scroll: usize,
    /// keybind viewer rows parsed from $states2/keybinds_cache (SUPER+SHIFT+/)
    pub keybinds: Vec<crate::themes::Keybind>,
    /// first visible row index into `keybinds` (wheel scroll)
    pub keybinds_scroll: usize,
    /// theme id currently active ($states2/cur_theme, for the card marker)
    pub cur_theme: String,
    /// current color-scheme id shown in the system card — read from
    /// $states/scheme_<cm> (cm = $states2/sm) while the dashboard is open
    pub cur_scheme: String,
    /// current UI state label shown in the system card ($states2/s_String)
    pub cur_channel: String,
    /// current accent source label shown in the system card ($states2/acc_Source)
    pub cur_acc_src: String,
    /// set by a theme card click; app.rs runs `scheme_main apply <name>`
    pub pending_theme_apply: Option<String>,
    /// custom accents parsed from `$states2/custom_acc.json` (`[{name,d,l}]`),
    /// shown in Settings → Appearance. Refreshed on Settings open.
    pub custom_accs: Vec<crate::shell::CustomAcc>,
    /// set by a Theme Accent row click; app.rs runs `scheme_main accent <kind> [payload]`
    /// (kind ∈ custom_acc / acc_from_wall / theme_default). Writes the
    /// `$states/acc_source_${s}` flag + GTK hot-reload via scheme_main_body.
    pub pending_accent_src: Option<(String, String)>,
    /// A new custom-accent hex queued for `pick_accent_main [hex]` (the screen
    /// picker, or a preset circle). app.rs spawns pick_accent_main, which
    /// stores it as `$states/custom_acc_${s}` + source `c`, marks
    /// `$states2/acc_changed`, then restores so shell + GTK pick it up.
    pub pending_pick_accent: Option<String>,
    /// Set by Settings → Appearance → Accent toggles / Alpha; app.rs runs
    /// `theme_main <sub> <args...>` where sub ∈ toggle_acc / start_icon_tone /
    /// alpha / acc_scrim (all added to theme_main_body).
    pub pending_theme_cmd: Option<Vec<String>>,
    /// Accent-card subview: open when the `>` chevron is tapped; shows every
    /// custom accent as a one-column paged list with a `<` back button.
    pub accent_list_open: bool,
    /// first visible index into `custom_accs` in the accent-card subview
    pub accent_list_scroll: usize,
    /// list viewport of the accent-card subview (x, y, w, h); wheel-driven
    pub accent_list_rect: (f32, f32, f32, f32),
    /// vertical scroll (px) for the Settings → Appearance content pane
    pub appearance_scroll: f32,
    /// vertical scroll (px) for the Settings → Pill content pane
    pub pill_scroll: f32,
    /// Custom-accent dropdown open state (Settings → Appearance)
    pub custom_acc_drop_open: bool,
    /// first visible index into `custom_accs` for the open dropdown list
    pub custom_acc_drop_scroll: usize,
    /// Settings → Appearance → Accent toggles / Alpha: live per-channel accent
    /// flags, read from `$states` on boot + `colors reload`. theme_body
    /// consumes these in init_global_vars to tint app backgrounds / borders.
    pub acc_flag_start_icon: bool,
    pub acc_flag_app_border: bool,
    pub acc_flag_hyprland: bool,
    pub acc_flag_gtk: bool,
    pub acc_flag_normal_app: bool,
    pub acc_scrim: bool,
    /// start-icon contrast tone slider (0..900)
    pub start_icon_tone: u32,
    /// lowercase hex alpha bytes: bg / acc / scrim / border
    pub bg_alpha: u8,
    pub acc_alpha: u8,
    pub scrim_alpha: u8,
    pub border_alpha: u8,
    /// dark/light state mirrored from $states2/m_dummy (`d` / `l`)
    pub dark_mode: bool,
    /// LIVE palette read from $states2/shell_vars — the shell takes every
    /// fg/bg/… from here from now on (config [colors] is only the fallback).
    /// Refreshed on boot and by `zen-shell ipc call colors reload`.
    pub sv_fg: u32,
    pub sv_fg2: u32,
    pub sv_fg3: u32,
    pub sv_bg: u32,
    pub sv_acc: u32,
    pub sv_sfg: u32,
    pub sv_hover: u32,
    pub sv_focus: u32,
    pub sv_disabled: u32,
    pub sv_series: [u32; 6],
    /// set by a WifiMenu row click; app.rs performs the NetworkManager connect
    pub pending_wifi: Option<String>,
    /// set by a BtMenu row click; app.rs performs the bluez connect/disconnect
    pub pending_bt: Option<String>,
    /// set by the CC/Settings wifi toggle; app.rs powers the NM radio
    pub pending_wifi_power: Option<bool>,
    /// set by the CC/Settings bt toggle; app.rs powers the bluez adapter
    pub pending_bt_power: Option<bool>,
    /// set by the SpeedTest-card "run" button; app.rs spawns the timed
    /// Cloudflare probe and reports back over the speed channel
    pub pending_speed_test: bool,
    /// set by the Quote-card refresh button; app.rs kicks the next quote fetch
    pub pending_quote_fetch: bool,
    /// set by a Recent-card row click; app.rs opens the path via xdg-open
    pub pending_recent_open: Option<String>,
    /// set by a Ticker-card row click; app.rs opens the asset's market page
    pub pending_ticker_open: Option<String>,
    /// set when the lockscreen password verifies; app.rs signals the lock
    /// backend (no-op for the native backend — collapsing is the unlock)
    pub pending_unlock: bool,
    /// lockscreen: password typed so far (dots in the UI)
    pub lock_pw: String,
    /// lockscreen: last auth error to show (wrong password / PAM unavailable)
    pub lock_error: Option<String>,
    /// set when Enter is pressed on the lockscreen; app.rs runs the PAM check
    pub pending_lock_auth: bool,
    /// lockscreen: a PAM check is in flight (Enter is ignored, UI says so)
    pub lock_checking: bool,
    /// native lock via ext-session-lock (bar stays put; a fullscreen lock
    /// surface owns input + blocks compositor keybinds). False → fallback:
    /// the bar morphs to fullscreen and grabs exclusive keyboard.
    pub lock_session: bool,

    // polkit auth dialog (replaces mate-polkit / hyprpolkitagent)
    /// action_id from the polkit BeginAuthentication call
    pub polkit_action_id: String,
    /// human-readable message from polkit ("Authentication is required to...")
    pub polkit_message: String,
    /// icon name from polkit
    pub polkit_icon: String,
    /// cookie from polkit (used to respond)
    pub polkit_cookie: String,
    /// uid of the user being authenticated
    pub polkit_uid: u32,
    /// password typed so far (masked in UI)
    pub polkit_pw: String,
    /// auth error message (wrong password, etc.)
    pub polkit_error: Option<String>,
    /// true while PAM verification is in flight
    pub polkit_checking: bool,
    /// set by Enter when a check isn't running — the app resolves it
    pub polkit_pending: bool,

    // clipboard history (synced by src/clipboard.rs, most recent first)
    /// text entry previews
    pub clip_text: Vec<String>,
    /// image entries as img-store keys (`clip:<n>`)
    pub clip_images: Vec<String>,
    /// keyboard-selected clipboard row (arrow keys)
    pub clip_sel: usize,
    /// scroll offset for the clipboard text panel (wheel, one row per notch)
    pub clip_scroll: usize,
    /// scroll offsets for the dashboard list cards (wheel over the card)
    pub clipimg_scroll: usize,
    pub todo_scroll: usize,
    pub notes_scroll: usize,
    /// last-rendered bounds of the dashboard list cards (wheel hit-testing)
    pub clip_card_rect: (f32, f32, f32, f32),
    pub clipimg_card_rect: (f32, f32, f32, f32),
    pub todo_card_rect: (f32, f32, f32, f32),
    pub notes_card_rect: (f32, f32, f32, f32),
    /// set by a clipboard row click / Enter; app.rs re-copies (idx, is_image)
    pub pending_clip: Option<(usize, bool)>,
    /// 1-indexed workspace id to dispatch to Hyprland (consumed by app.rs)
    pub pending_ws_dispatch: Option<usize>,
    /// compositor card screenshot button pressed: "region" | "full"
    /// (consumed by app.rs → shot_main)
    pub pending_shot: Option<String>,
    /// snippets row clicked: copy index (consumed by app.rs → clipboard)
    pub pending_snippet_copy: Option<usize>,
    /// mirror-card "take photo" pressed (consumed by app.rs)
    pub pending_mirror_photo: bool,
    // ── new cards: pomodoro / fans / workspaces ──
    /// focus phase when true, break phase when false
    pub pomo_phase_focus: bool,
    /// timer running (counting toward the phase deadline)
    pub pomo_running: bool,
    /// remaining seconds while paused (None = full duration / running)
    pub pomo_paused_rem: Option<u64>,
    /// deadline of the running phase
    pub pomo_deadline: Option<Instant>,
    /// focus length in minutes (stepper-adjustable, persisted)
    pub pomo_focus_min: u32,
    /// break length in minutes (persisted)
    pub pomo_break_min: u32,
    /// completed focus sessions this session
    pub pomo_sessions: u32,
    /// live fan RPMs as (label, rpm) from hwmon
    pub fans_rpm: Vec<(String, u32)>,
    /// slow-decaying per-fan RPM peak for relative bars
    pub fan_peaks: Vec<f32>,
    // ── world-clock card ──
    /// pinned zones as (city, iana tz, utc-offset minutes, abbreviation);
    /// offset/abbrev are cache — refreshed by the app's batched tz probe
    pub worldclock_zones: Vec<(String, String, i32, String)>,
    /// inline zone search state (Some = focused + buffer)
    pub worldclock_search: Option<String>,
    /// last time the pinned zones' offsets were batch-probed
    pub worldclock_probed_at: Option<Instant>,
    /// search-result rows drawn last frame: (hit key, tz, city) — the click
    /// handler resolves pick keys against this map
    pub worldclock_pick_map: Vec<(u32, String, String)>,
    /// last-rendered bounds of the world-clock card (wheel/blur hit-testing)
    pub worldclock_rect: (f32, f32, f32, f32),
    // ── water tracker card ──
    /// glasses logged today (1 glass = 250 ml)
    pub water_glasses: u32,
    /// daily goal in glasses (persisted; default 8 ≈ 2 L)
    pub water_goal: u32,
    /// `YYYY-MM-DD` the count belongs to (midnight reset check)
    pub water_day: String,
    /// last-rendered bounds of the water card (wheel goal-adjust hit-testing)
    pub water_rect: (f32, f32, f32, f32),
    // ── compositor / effects card ──
    /// live blur state from `dash_status` extension (blur_main status)
    pub fx_blur_on: bool,
    /// live shadow state (shadow_main status)
    pub fx_shadow_on: bool,
    /// live opacity state (opacity_main status)
    pub fx_opacity_on: bool,
    // ── countdown card ──
    /// events as (label, YYYY-MM-DD); sorted by date at load/save
    pub countdown_events: Vec<(String, String)>,
    /// inline composer buffer (Some = focused): `Name YYYY-MM-DD`
    pub countdown_input: Option<String>,
    /// last-rendered bounds of the countdown card
    pub countdown_rect: (f32, f32, f32, f32),
    // ── alarms card ──
    /// alarms as (HH:MM, label) — fire once per day when clock matches
    pub alarm_list: Vec<(String, String)>,
    /// composer buffer: `HH:MM label`
    pub alarm_input: Option<String>,
    /// `HH:MM` entries already fired today (dedup re-fires on the same minute)
    pub alarms_fired: Vec<String>,
    /// last-rendered bounds of the alarms card
    pub alarms_rect: (f32, f32, f32, f32),
    // ── snippets card ──
    /// saved clips as (name, body) — click a row to copy the body
    pub snippets: Vec<(String, String)>,
    /// composer buffer: `name body`
    pub snippet_input: Option<String>,
    /// last snippet copied (flash the row) + when
    pub snippet_copied: Option<usize>,
    pub snippet_copied_at: Option<Instant>,
    /// last-rendered bounds of the snippets card
    pub snippets_rect: (f32, f32, f32, f32),
    // ── expenses card ──
    /// entries as (amount_cents, category, YYYY-MM-DD)
    pub expenses: Vec<(u32, String, String)>,
    /// composer buffer (custom amount) — Some = focused
    pub expense_input: Option<String>,
    /// last-rendered bounds of the expenses card
    pub expenses_rect: (f32, f32, f32, f32),
    /// mirror-card record toggle pressed (consumed by app.rs)
    pub pending_mirror_rec: bool,
    /// mirror-card fps button pressed — app restarts capture at the new rate
    pub pending_mirror_fps: bool,
    /// workspace we were on before the current switch (for OSD label)
    pub ws_prev: usize,

    // system stats (stats.rs, 2s timer)
    pub cpu: i32,
    pub mem_pct: i32,
    pub disk_pct: i32,
    /// memory detail (GB, mem card): total / used (=total−available) /
    /// available / cached / free from /proc/meminfo
    pub mem_total_gb: f32,
    pub mem_used_gb: f32,
    pub mem_avail_gb: f32,
    pub mem_cached_gb: f32,
    pub mem_free_gb: f32,
    /// disk detail (gauges card): / root used + total in (decimal) GB
    pub disk_used_gb: f32,
    pub disk_total_gb: f32,
    /// CPU detail (cpu card): load averages (/proc/loadavg 1/5/15 min)
    pub cpu_load: (f32, f32, f32),
    /// CPU detail (cpu card): current frequency in MHz (max over online cores)
    pub cpu_freq_mhz: u64,
    /// CPU detail (cpu card): package temp °C (max thermal zone); 0 = none
    pub cpu_temp: f32,
    /// GPU busy % (amdgpu sysfs / nvidia-smi); -1 = no GPU source found
    pub gpu: i32,
    /// GPU temperature °C (gpu card); 0 = source exposes none
    pub gpu_temp: f32,
    /// GPU VRAM used/total in MiB; total 0 = not exposed → card hides it
    pub gpu_vram_used_mb: u64,
    pub gpu_vram_total_mb: u64,
    /// hwmon thermal zones as `(type, °C)` (thermal card)
    pub thermal_zones: Vec<(String, f32)>,
    /// CPU load history (0..=1) for the dashboard dual-line chart
    pub cpu_history: std::collections::VecDeque<f32>,
    /// GPU load history (0..=1) — same chart, second line
    pub gpu_history: std::collections::VecDeque<f32>,
    pub net_up: String,
    pub net_down: String,

    // system identity (populated once at startup)
    pub hostname: String,
    pub username: String,
    /// system uptime as a human string ("3d 2h 15m")
    pub uptime: String,
    /// network traffic history for the sparkline (rx_bps, tx_bps), last 40 samples
    pub net_history: std::collections::VecDeque<(u64, u64)>,
    /// disk I/O history (read_bytes/s, write_bytes/s), last 48 samples — feeds
    /// the Disk I/O card's dual-line graph (summed across /sys/block/*)
    pub disk_history: std::collections::VecDeque<(u64, u64)>,
    /// to-do list shown in the dashboard's right column — (text, done),
    /// persisted to `~/.local/share/zen-shell/todos.txt`
    pub todos: Vec<(String, bool)>,
    /// `Some(buffer)` while the to-do composer holds keyboard focus
    /// (typed characters land here; Enter commits, Esc blurs)
    pub todo_input: Option<String>,

    // ── new card data fields ──
    /// disk partitions: (mount, used_pct, used_gb, total_gb)
    pub disk_parts: Vec<(String, i32, f32, f32)>,
    /// top processes: (name, cpu%, mem%)
    pub top_procs: Vec<(String, i32, i32)>,
    /// swap usage percent
    pub swap_pct: i32,
    /// active window title
    pub active_win_title: String,
    /// active window class
    pub active_win_class: String,
    /// active window pid (from `hyprctl activewindow -j`) — for /proc lookup
    pub active_win_pid: i32,
    /// RSS of the active window's process in KiB (`/proc/<pid>/status`)
    pub active_win_rss_kb: u64,
    /// vertical battery card viewport rect (x, y, w, h)
    pub battery_v_rect: (f32, f32, f32, f32),
    /// vertical battery card pane: 0 = icon, 1 = extra info
    pub battery_v_pane: usize,
    /// horizontal battery card viewport rect (x, y, w, h)
    pub battery_h_rect: (f32, f32, f32, f32),
    /// horizontal battery card pane: 0 = icon, 1 = extra info
    pub battery_h_pane: usize,
    /// weather card viewport rect (x, y, w, h)
    pub weather_rect: (f32, f32, f32, f32),
    /// weather card pane: 0 = current, 1 = 7-day forecast
    pub weather_pane: usize,
    /// world-map card viewport rect (x, y, w, h) — whole card, for swiping
    pub worldmap_rect: (f32, f32, f32, f32),
    /// world-map map-body rect (x, y, w, h) — the clickable/geo resolvable area
    pub worldmap_body_rect: (f32, f32, f32, f32),
    /// world-map pane: 0 = real, 1 = filled, 2 = dotted, 3 = stroked
    pub world_pane: usize,
    /// view center in (lon, lat) degrees (zoom target, defaults 0,0)
    pub world_center: (f32, f32),
    /// view zoom, 1.0 = whole world (button/wheel steps, clamped 1..=32)
    pub world_zoom: f32,
    /// swipe-right style/settings menu open (overlays the map body)
    pub world_menu: bool,
    /// "save downloaded data" — true → persistent cache, false → /tmp scratch
    pub world_save_data: bool,
    /// pending download (requested by a click; app services timer spawns it)
    pub world_dl_pending: Option<crate::shell::WorldDl>,
    /// last download outcome (None → idle; Some(err) → show a toast on card)
    pub world_dl_err: Option<String>,
    /// region being shown at high zoom: ISO2 + its loaded fine rings (None)
    pub world_region: Option<String>,
    pub world_region_rings: Option<Vec<Vec<(f32, f32)>>>,
    /// monotonically increases whenever the world geometry/regions change —
    /// invalidates the App's cached band texture so the map re-bakes
    pub world_rev: u64,
    /// iso the download pill currently refers to (set during card draw)
    pub world_region_target: Option<String>,
    /// parsed repo manifest (region lookup) — set after the base download
    pub world_manifest: Option<std::sync::Arc<crate::shell::mapdata::Manifest>>,
    /// last map click in (lat, lon) degrees — the marker; None before any click
    pub world_marker: Option<(f32, f32)>,
    /// live map click-drag: (press content_x, press content_y, center lon,
    /// center lat at press) — panning moves these, re-bakes only on idle
    pub world_drag: Option<(f32, f32, f32, f32)>,
    /// a press+move that crossed the drag threshold (≈3px, like a gallery
    /// photo) — suppresses the click on release
    pub world_drag_moved: bool,
    /// display name for the selected place (city from zone.tab)
    pub world_city: String,
    /// selected IANA zone (e.g. "Asia/Tokyo")
    pub world_tz: String,
    /// timezone abbreviation (from `date +%Z`, e.g. "JST")
    pub world_abbrev: String,
    /// signed UTC offset in minutes (from `date +%z`)
    pub world_offset_min: i32,
    /// zone awaiting an offset probe: app spawns `TZ=<zone> date +%z %Z`
    pub pending_world_offset: Option<String>,
    /// last offset-probe instant (re-probe on DST change at most ~5 min apart)
    pub world_off_at: Option<std::time::Instant>,
    /// CPU governor power-save mode is active (persisted per channel)
    pub power_save_on: bool,
    /// set by the battery-card button; app.rs toggles the governor
    pub pending_power_save: bool,
    /// saved notes, newest-first; each (title, body). Persisted at
    /// `$states/notes`, one line per note `title\x1fbody`.
    pub notes: Vec<(String, String)>,
    /// lazy-load guard for `self.notes`
    pub notes_loaded: bool,
    /// notes composer state: None idle, Some(0) editing title, Some(1) body
    pub notes_input: Option<u8>,
    /// composer buffers
    pub notes_title: String,
    pub notes_body: String,
    /// available package updates count
    pub pkg_update_count: i32,
    /// ssh/vpn status lines
    pub ssh_vpn_lines: Vec<String>,
    /// docker containers: (name, status, image)
    pub docker_containers: Vec<(String, String, String)>,
    /// keyboard layout name
    pub kb_layout: String,

    // ── metro grid layout (DashCard spans) ──
    /// card placements; persists to `~/.local/share/zen-shell/dash_layout.txt`
    pub dash_layout: Vec<CardLayout>,
    /// EDIT MODE (right-click): cards drag to move, corner grip resizes;
    /// the flow packer pushes the others aside live and the canvas grows
    pub dash_edit: bool,
    /// active move-drag: card + grab offset in pixels
    pub(crate) edit_drag: Option<(DashCard, f32, f32)>,
    /// active resize-drag from corner grip: (card, grab_offset_x, grab_offset_y, orig_w, orig_h)
    pub(crate) edit_resize: Option<(DashCard, f32, f32, u16, u16)>,
    /// the active move-drag started from the PARKING TRAY (not the grid)
    pub(crate) edit_drag_from_tray: bool,
    /// cached ghost grid slot for move-drag — skip repack when unchanged
    edit_ghost_slot: Option<(u16, u16)>,
    /// cached card span for resize-drag — skip repack when unchanged
    edit_resize_cached: Option<(u16, u16)>,
    /// edit-mode grid boost (rows/cols "+" "-" buttons): MIN columns the
    /// canvas must span (0 = auto = packed content extent). A bigger value
    /// makes the dashboard WIDER even when cards don't fill it.
    pub edit_grid_cols: u16,
    /// edit-mode grid boost: MIN rows (0 = auto) — makes it TALLER.
    pub edit_grid_rows: u16,
    /// dashboard viewport scroll offset (px). The surface is capped to the
    /// monitor, so a taller/wider canvas scrolls: wheel, scrollbars, or the
    /// edit-mode "+" buttons pan the board.
    pub dash_scroll_x: f32,
    pub dash_scroll_y: f32,
    /// active dashboard scrollbar drag: (0 vertical, 1 horizontal, grab px)
    pub dash_sb_drag: Option<(u8, f32)>,
    /// last dashboard scrollbar activity (scroll / drag). The bars stay
    /// shown for `DASH_SB_KEEP + DASH_SB_FADE` after it, then auto-hide
    /// (overlay style). `None` = never interacted → hidden.
    pub dash_sb_last: Option<Instant>,
    /// scroll direction toggle: false = vertical (wheel scrolls rows),
    /// true = horizontal (wheel scrolls columns)
    pub dash_scroll_dir: bool,
    /// App-shortcut card: ordered pinned app ids (card "Apps"). Empty slots
    /// show a "+" tile; clicking it opens the launcher in pick mode.
    pub app_shortcuts: Vec<String>,
    /// When set, the launcher runs in "pick for app-shortcut slot" mode —
    /// the next app selected is stored into `app_shortcuts[slot]` instead of
    /// launched. None = normal launcher.
    pub app_shortcut_pick: Option<usize>,
    /// Apps-card rect this frame (set by the drawer, read by input).
    pub app_shortcut_rect: (f32, f32, f32, f32),
    /// cards parked in the EDIT-MODE tray — off-dashboard storage you can
    /// drag back onto the grid (persisted in the layout file)
    pub tray_cards: Vec<DashCard>,
    /// hold-to-confirm state for "Clear all cards" button: start time when held
    pub dash_clear_all_hold: Option<std::time::Instant>,
    pub banner_clear_all_hold: Option<std::time::Instant>,
    /// parking-tray strip rect this frame (set by the drawer, read by input)
    pub tray_rect: (f32, f32, f32, f32),
    /// dynamic height of the tray content (chip rows), set by drawer each frame
    pub tray_content_h: f32,
    /// banner-chips tray row scroll offset (whole rows; 0 = first visible).
    pub banner_tray_scroll_row: i32,
    /// dashboard-cards tray row scroll offset (whole rows; 0 = first visible).
    pub dash_tray_scroll_row: i32,
    /// placements actually drawn/hit this frame (packed; ghost while editing)
    pub packed_layout: Vec<CardLayout>,
    /// committed column extent the packed cards reach — the canvas is
    /// AUTO-TRIMMED to exactly this (no empty strips on the right)
    dash_cols: u16,
    /// committed row extent of the canvas (grows AND shrinks with content)
    dash_rows: u16,
    /// when set (right after restoring a persisted per-channel dashboard),
    /// `refresh_pack` trusts the exact x/y in `dash_layout` instead of re-packing
    /// — so cards render exactly where they were saved. Cleared on the first edit.
    layout_pinned: bool,
    /// live re-packed preview + its row extent while dragging in edit mode
    edit_preview_pack: Option<(Vec<CardLayout>, u16)>,
    /// last drawn pixel position per card — cards GLIDE to their packed slot
    /// while others are dragged/resized instead of snapping (edit-mode fluidity)
    card_anim: std::collections::HashMap<DashCard, (f32, f32)>,
    /// true while the settle glide still has moving cards (app keeps rendering)
    pub edit_settling: bool,
    /// hover-collapse grace after a heavy slider gesture (saturation / Kelvin):
    /// their backend scripts notify-send a popup which can steal pointer focus
    /// right at release — the dashboard must not read that as "cursor left"
    pub drag_grace_until: std::time::Instant,
    // runtime geometry written by the drawers each frame, read by input —
    // cards move freely on the grid so absolute consts can't exist anymore
    pub dash_w: f32,
    pub dash_h: f32,
    /// media seek bar pixel rect (x, y, w, h) inside the dashboard
    pub media_seek_rect: (f32, f32, f32, f32),
    /// full bounds of the media card (last frame) — Space plays/pauses over it
    pub media_card_rect: (f32, f32, f32, f32),
    /// horizontal EQ-fader geometry per slider slot: (x, y_center, width)
    pub faders: [(f32, f32, f32); 5],
    /// horizontal geometry of the balance slider row: (x, y_center, width)
    pub balance_rect: (f32, f32, f32),
    /// pixel rect of the dashboard Wallpaper card this frame — the wheel
    /// handler scrolls the card's grid only when the pointer is over it
    pub wp_card_rect: (f32, f32, f32, f32),
    // weather (weather.rs thread → channel)
    pub weather: Option<crate::weather::WeatherMsg>,
    pub weather_city: String,

    // workspace preview (hyprctl clients, on hover)
    pub ws_preview: Vec<String>,
    /// which workspace the preview was fetched for (active id)
    pub ws_preview_ws: usize,

    // pill behavior toggles (Settings → Pill; runtime, seeded from config)
    /// hover expands the pill → dashboard
    pub expand_on_hover: bool,
    /// hover / click on the resting pill opens the Control Center instead
    /// of the dashboard
    pub cc_as_dashboard: bool,
    /// floating pill: hovers with floating_offset margin over content; off →
    /// sticks flush to the top/bottom edge
    pub bar_floating: bool,
    /// reserve screen space (exclusive zone) so the resting pill pushes
    /// windows below/above instead of floating over them
    pub bar_reserve: bool,
    /// set by the Settings Floating / Reserve toggles; app.rs re-applies
    /// anchor / margin / exclusive zone
    pub pending_floating: Option<bool>,
    pub pending_reserve: Option<bool>,
    /// Settings → Appearance → "Icon font": the live slot-flavor
    /// ("google"|"caskadia"|"jetbrains"|"phosphor"), seeded from config.
    /// Queued for app.rs, which swaps the text engine's slot table + family.
    pub icon_style: String,
    pub pending_icon_style: Option<String>,
    /// collapsed-pill widget visibility
    pub pill_clock: bool,
    pub pill_battery: bool,
    /// show battery % as text outside the icon (collapsed pill + system card)
    pub pill_battery_pct: bool,
    pub pill_ws: bool,
    pub pill_tray: bool,
    /// music visualizer bars on the collapsed pill
    pub pill_viz: bool,
    /// gear chip on the collapsed pill → Settings
    pub pill_settings: bool,
    /// notification bell on the collapsed pill
    pub pill_notif: bool,
    /// user info chip on the collapsed pill
    pub pill_user: bool,
    /// battery watts chip on the collapsed pill
    pub pill_watts: bool,
    /// brightness / dark-mode chip (sun ⇄ moon) on the collapsed pill
    pub pill_brightness: bool,
    /// resting-pill: workspace numbers 1..5 always visible
    pub pill_ws_long: bool,
    /// resting-pill: only the active workspace number
    pub pill_ws_short: bool,
    /// resting-pill: Wi-Fi chip → Wifi menu
    pub pill_wifi: bool,
    /// resting-pill: Bluetooth chip → BT menu
    pub pill_bluetooth: bool,
    /// resting-pill: volume chip → Sound panel
    pub pill_volume: bool,
    /// resting-pill: wallpaper chip → Wallpaper panel
    pub pill_wallpaper: bool,
    /// resting-pill: themes chip → Themes picker
    pub pill_themes: bool,
    /// resting-pill: brand glyph chip (display-only)
    pub pill_branding: bool,
    /// brand glyph text (`$states2/d`) rendered in `$states2/branding_font`
    pub brand_glyph: String,
    /// collapsed-pill item order (left → right)
    pub pill_order: Vec<PillItem>,
    /// active drag state for pill-item reorder (None at rest)
    pub pill_drag: Option<PillDrag>,
    /// expanded top-banner strip order (left → right): chips, separators,
    /// smart fillers
    pub banner_order: Vec<BannerToken>,
    /// dashboard cards on/off — off shows only the top banner strip
    pub dash_enabled: bool,
    /// banner chips on/off — off hides every item on the top banner strip
    /// (the band collapses; only the dashboard / strip-editor chrome remains).
    /// Runtime toggle from the strip editor; not persisted.
    pub banner_chips_enabled: bool,
    /// active drag state for banner strip reorder (None at rest)
    pub banner_drag: Option<BannerDrag>,
    /// manual width (px) of the banner-only strip when the dashboard is off;
    /// 0 = auto-fit to content
    pub banner_w: i32,
    /// last manual width kept across the auto <=> manual width toggle
    /// (restored when turning auto off again)
    pub banner_w_last: i32,
    /// slider track rect (surface coords) of the manual banner-width slider;
    /// zeroed while the slider is hidden (auto / dashboard on)
    pub banner_w_slider_rect: (f32, f32, f32, f32),
    /// edit-mode UI slider track rects (dashboard padding / window roundness /
    /// card roundness); the card rect is zeroed while sync is on
    pub ui_pad_slider_rect: (f32, f32, f32, f32),
    pub win_rad_slider_rect: (f32, f32, f32, f32),
    pub card_rad_slider_rect: (f32, f32, f32, f32),
    /// the aggregate connectivity chip is expanded (Wi-Fi / Bluetooth rows)
    pub connectivity_open: bool,
    /// the system-card ⋮ menu is open (Bar position / Open settings rows)
    pub sys_menu_open: bool,
    /// the ⋮ menu is showing its Bar-position 4-edge submenu
    pub sys_menu_bp_open: bool,
    /// strip row width used by the last layout pass (fillers split this) —
    /// lets input hit-testing agree with the drawn cells
    pub strip_row_w: f32,
    /// Horizontal scroll offset per banner zone (px) — L / C / R scroll
    /// independently when a zone overflows its third. Clamped against each
    /// zone's live overflow on every layout pass.
    pub banner_zone_scroll: [f32; 3],
    /// Horizontal scroll offset of the WHOLE banner strip (px) — used when
    /// the strip has no L/C/R zone markers and its natural width exceeds the
    /// panel: the strip becomes one horizontally-scrollable row. Clamped
    /// against the strip overflow on every layout pass.
    pub banner_strip_scroll: f32,
    /// pixels rect of the edit-mode "banner chips" parking tray strip
    pub banner_tray_rect: (f32, f32, f32, f32),
    /// pixels rect of the edit-mode strip-editor row (dashboard off)
    pub banner_ctrl_rect: (f32, f32, f32, f32),
    /// screen edge the bar strip is anchored to (left/top/right/bottom)
    pub bar_edge: BarEdge,
    /// set by the bar-position controls (⋮ menu / settings); the app
    /// re-anchors the layer surface on each move
    pub pending_move: Option<BarEdge>,
    /// horizontal padding inside the collapsed pill (px)
    pub pill_margin_x: f32,
    /// background alpha of the resting pill (0..=255)
    pub pill_alpha: u8,
    /// background alpha of dashboard cards (0..=255)
    pub dash_card_alpha: u8,
    /// power menu / lockscreen hold-to-confirm delay (s; 0.5..=3.0)
    pub power_hold_delay: f32,
    /// extra space between the window edges and the dashboard cards (px)
    pub ui_pad: f32,
    /// window / panel surface corner roundness (px, 0..=20)
    pub win_radius: f32,
    /// dashboard card corner roundness (px, 0..=20)
    pub card_radius: f32,
    /// cards follow the window roundness
    pub card_radius_sync: bool,
    /// current ui_state char (d/l/n) for per-state config saving
    pub ui_state: char,
    /// per-state config path (e.g. ~/.config/zen-shell/user/shell_d)
    pub(crate) per_state_config: std::path::PathBuf,
    /// active category in the two-pane Settings app (which pane is shown)
    pub settings_tab: SettingsTab,

    // morph
    pub cur_w: f32,
    pub cur_h: f32,
    pub anim: Option<Anim>,
    pub size_pending: bool,
    pub last_committed: (f32, f32),
    /// content centering while the spring overshoots the target size (the
    /// surface can be briefly LARGER than the layout — content shifts to stay
    /// centered instead of leaving an empty strip on one side)
    content_dx: f32,
    content_dy: f32,
}

/// `123456789` µs → `m:ss` (MPRIS track times).
pub fn fmt_time(micros: i64) -> String {
    let s = (micros / 1_000_000).max(0);
    format!("{}:{:02}", s / 60, s % 60)
}


#[cfg(test)]
mod pack_tests {
    use super::*;

    #[test]
    fn banner_drop_from_strip_reorders() {
        // n0=5, dragging token from idx 2 and dropping BEFORE idx 0 (k=0):
        // after removing idx2 the target is still 0 → the token lands first
        assert_eq!(Shell::banner_drop_target(2, 0, 5, false), 0);
        // dropping before the ORIGINAL idx 3 (k=3): removal shifted it one
        // left, so it must be inserted at 2
        assert_eq!(Shell::banner_drop_target(2, 3, 5, false), 2);
        // dropping at the tail (k=n0) after removal → insert at 4
        assert_eq!(Shell::banner_drop_target(2, 5, 5, false), 4);
        // dropping at its OWN slot (k=from) → same position, no-op
        assert_eq!(Shell::banner_drop_target(2, 2, 5, false), 2);
    }

    #[test]
    fn banner_drop_from_tray_inserts_in_place() {
        // tray chip (not on the strip): no removal, so the column is used
        // verbatim — k=3 of 5 inserts BEFORE original slot 3
        assert_eq!(Shell::banner_drop_target(0, 3, 5, true), 3);
        // clicking (k=n0) appends at the tail
        assert_eq!(Shell::banner_drop_target(0, 5, 5, true), 5);
        // out-of-range columns clamp to the ends
        assert_eq!(Shell::banner_drop_target(0, 99, 5, true), 5);
        assert_eq!(Shell::banner_drop_target(7, 0, 5, true), 0);
    }

    #[test]
    fn banner_drop_edge_lists() {
        // empty strip: only the tail exists
        assert_eq!(Shell::banner_drop_target(0, 0, 0, false), 0);
        assert_eq!(Shell::banner_drop_target(0, 0, 0, true), 0);
        // single-token strip, move itself around
        assert_eq!(Shell::banner_drop_target(0, 0, 1, false), 0);
        assert_eq!(Shell::banner_drop_target(0, 1, 1, false), 0);
        // overshoot from_idx clamps to the end
        assert_eq!(Shell::banner_drop_target(9, 9, 5, false), 4);
    }

    #[test]
    fn up_step_sits_top_left_in_freed_row() {
        // the documented hard-rule step, in isolation: the card below climbs
        // one row and settles into the TOP-LEFT slot of the row above
        let cards = [CardLayout { card: DashCard::Weather, x: 0, y: 1, w: 1, h: 1 }];
        let (out, moved) = up_step(&cards, 2);
        assert!(moved);
        let b = out[0];
        assert_eq!((b.x, b.y), (0, 0), "climbs into the row above's top-left slot");
    }

    #[test]
    fn up_step_leftmost_climbs_even_if_column_above_is_taken() {
        // the LEFTMOST card below must climb when the row above has ANY slot
        // that fits — even when its own column above is occupied; it lands at
        // the row above's RIGHTMOST free position
        let cards: Vec<CardLayout> = [
            (DashCard::System, 0, 0, 2, 1),
            (DashCard::Weather, 2, 0, 2, 1),
            (DashCard::CpuGpu, 0, 1, 2, 1),
            (DashCard::Gauges, 4, 1, 2, 1),
        ]
        .map(|(c, x, y, w, h)| CardLayout { card: c, x, y, w, h })
        .to_vec();
        // one step: row 0 has System + Weather at cols 0..3, cols 4..5 free;
        // CpuGpu (x=0, leftmost below) climbs into the rightmost fitting slot
        let (out, moved) = up_step(&cards, 6);
        assert!(moved);
        let cg = out.iter().find(|l| l.card == DashCard::CpuGpu).unwrap();
        assert_eq!((cg.x, cg.y), (4, 0), "leftmost-below → rightmost of the row above");
    }

    #[test]
    fn gravity_closes_top_gap_from_below() {
        // delete the top card → the cards below fall UP to refill: the
        // leftmost-from-below card lands in the row-above's RIGHTMOST slot,
        // everything cascades left, and the result is a hole-free pack that
        // fills the top rows before any row below them
        let cards: Vec<CardLayout> = [
            (DashCard::Weather, 0, 1, 1, 1),
            (DashCard::CpuGpu, 0, 2, 1, 1),
            (DashCard::Gauges, 0, 3, 1, 1),
        ]
        .map(|(c, x, y, w, h)| CardLayout { card: c, x, y, w, h })
        .to_vec();
        let (out, moved) = gravity_pack(&cards, 2);
        assert!(moved);
        let b = out.iter().find(|l| l.card == DashCard::Weather).unwrap();
        let cg = out.iter().find(|l| l.card == DashCard::CpuGpu).unwrap();
        let g = out.iter().find(|l| l.card == DashCard::Gauges).unwrap();
        // top row fills fully (cols 0 and 1) before card 3 goes to row 1
        assert_eq!((b.x, b.y), (0, 0), "leftmost raised card sits top-left");
        assert_eq!((cg.x, cg.y), (1, 0), "next card lands in the row-above's RIGHT slot");
        assert_eq!((g.x, g.y), (0, 1), "remaining card fills the next row");
        // hole-free: no empty row between cards, nothing below the used rows
        let rows: std::collections::HashSet<u16> = out.iter().map(|l| l.y).collect();
        assert_eq!(rows, [0u16, 1].into_iter().collect());
    }

    #[test]
    fn gravity_left_packs_and_never_overlaps() {
        // gap on row 0 is closed by sliding the right card left; then the
        // below cards fall UP into the freed cells — filling every cell of
        // the top rows — and nothing is ever allowed to overlap
        let cards: Vec<CardLayout> = [
            (DashCard::System, 0, 0, 2, 1),
            (DashCard::Weather, 6, 0, 2, 1),
            (DashCard::CpuGpu, 4, 1, 2, 1),
            (DashCard::Gauges, 6, 1, 2, 1),
        ]
        .map(|(c, x, y, w, h)| CardLayout { card: c, x, y, w, h })
        .to_vec();
        let (out, moved) = gravity_pack(&cards, 8);
        assert!(moved);
        let mut occ = std::collections::HashSet::new();
        for l in &out {
            for dy in 0..l.h {
                for dx in 0..l.w {
                    assert!(occ.insert((l.x + dx, l.y + dy)), "overlap: {:?}", l.card);
                }
            }
        }
        // everything falls up into row 0 (2-wide each, cols 0-7 in order)
        let s = out.iter().find(|l| l.card == DashCard::System).unwrap();
        let wth = out.iter().find(|l| l.card == DashCard::Weather).unwrap();
        let cg = out.iter().find(|l| l.card == DashCard::CpuGpu).unwrap();
        let g = out.iter().find(|l| l.card == DashCard::Gauges).unwrap();
        assert_eq!((s.x, s.y), (0, 0));
        assert_eq!((cg.x, cg.y), (2, 0), "below card rose into the row-above gap");
        assert_eq!((wth.x, wth.y), (4, 0), "right card slid left to stay in order");
        assert_eq!((g.x, g.y), (6, 0), "last card filled the rightmost cell");
        let rows: std::collections::HashSet<u16> = out.iter().map(|l| l.y).collect();
        assert_eq!(rows, [0u16].into_iter().collect(), "everything packed onto row 0");
    }

    #[test]
    fn gravity_is_idempotent_when_packed() {
        let cards: Vec<CardLayout> = [
            (DashCard::System, 0, 0, 2, 1),
            (DashCard::Weather, 2, 0, 2, 1),
        ]
        .map(|(c, x, y, w, h)| CardLayout { card: c, x, y, w, h })
        .to_vec();
        let (out, moved) = gravity_pack(&cards, 6);
        assert!(!moved, "already packed tight");
        assert_eq!(out.len(), 2);
    }

    #[test]
    fn compact_shifts_left_into_gap() {
        // gap at columns 2-3 on row 0; the right-hand card must slide left
        // into it, others staying put
        let cards: Vec<CardLayout> = [
            (DashCard::System, 0, 0, 2, 1),
            (DashCard::Weather, 4, 0, 2, 1),
            (DashCard::CpuGpu, 2, 1, 2, 1),
        ]
        .map(|(c, x, y, w, h)| CardLayout { card: c, x, y, w, h })
        .to_vec();
        let (out, moved) = compact_layout(&cards, 6);
        assert!(moved);
        let w = out.iter().find(|l| l.card == DashCard::Weather).unwrap();
        assert_eq!(w.x, 2, "fills the gap left of it");
        let cg = out.iter().find(|l| l.card == DashCard::CpuGpu).unwrap();
        assert_eq!(cg.x, 0, "fills its own (empty) row band's leftmost cells");
        assert_eq!(cg.y, 1, "row band is preserved");
    }

    #[test]
    fn compact_left_is_idempotent() {
        let cards: Vec<CardLayout> = [
            (DashCard::System, 0, 0, 2, 1),
            (DashCard::Weather, 2, 0, 2, 1),
        ]
        .map(|(c, x, y, w, h)| CardLayout { card: c, x, y, w, h })
        .to_vec();
        let (out, moved) = compact_layout(&cards, 6);
        assert!(!moved, "already packed tight");
        assert!(out.iter().all(|l| l.x == if l.card == DashCard::System { 0 } else { 2 }));
    }

    #[test]
    fn compact_tall_card_keeps_its_band() {
        // tall card spans rows 0-1; a gap under/left must never make another
        // card try to occupy its cells
        let cards: Vec<CardLayout> = [
            (DashCard::System, 0, 0, 2, 2),
            (DashCard::Weather, 4, 0, 2, 1),
        ]
        .map(|(c, x, y, w, h)| CardLayout { card: c, x, y, w, h })
        .to_vec();
        let (out, moved) = compact_layout(&cards, 6);
        assert!(moved);
        let w = out.iter().find(|l| l.card == DashCard::Weather).unwrap();
        assert_eq!(w.x, 2);
        let s = out.iter().find(|l| l.card == DashCard::System).unwrap();
        assert_eq!(s.x, 0);
    }

    #[test]
    fn default_layout_preserved() {
        let layout = default_dash_layout();
        let (packed, _) = pack_cards_max(&layout, DASH_COLS);
        assert_eq!(packed.len(), layout.len(), "card count preserved");
        for (orig, packed_l) in layout.iter().zip(packed.iter()) {
            assert_eq!(orig.card, packed_l.card);
            assert_eq!(orig.x, packed_l.x, "x changed for {:?}", orig.card);
            assert_eq!(orig.y, packed_l.y, "y changed for {:?}", orig.card);
            assert_eq!(orig.w, packed_l.w);
            assert_eq!(orig.h, packed_l.h);
        }
    }

    #[test]
    fn always_fills_leftmost_gap() {
        // Packing hard rule: a card always moves into the LEFT-MOST empty
        // space big enough for it — it never hangs to the right of a hole.
        // Row 0 is fully packed 0..19; a lone card on row 4 slides straight to
        // the far-left first empty cell (x=0), not just beside its committed x.
        let mut cards: Vec<CardLayout> = [
            DashCard::System, DashCard::Weather, DashCard::Media, DashCard::Network,
        ]
        .iter()
        .enumerate()
        .map(|(i, c)| CardLayout { card: *c, x: i as u16 * 4, y: 0, w: 4, h: 4 })
        .collect();
        cards.push(CardLayout { card: DashCard::CpuGpu, x: 10, y: 4, w: 2, h: 4 });
        let (packed, _) = pack_cards_max(&cards, DASH_COLS);
        let cg = packed.iter().find(|l| l.card == DashCard::CpuGpu).unwrap();
        assert_eq!(cg.y, 4, "fits its own (empty) row");
        assert_eq!(cg.x, 0, "fills the left-most empty space of the row");
    }

    #[test]
    fn fills_gap_above_rightmost_slot() {
        // A card whose own row is FULL pops up one row into its RIGHT-most
        // empty slot: row 0 holds cards 0..14 (right gap 15..19), row 1 is
        // packed 0..19 by the order below. The last w=4 card is committed to
        // row 1, can't fit it, so it jumps to row 0's rightmost gap (x=16).
        let mut cards = vec![
            CardLayout { card: DashCard::System, x: 0, y: 0, w: 15, h: 1 },
            CardLayout { card: DashCard::Weather, x: 0, y: 1, w: 15, h: 1 },
            CardLayout { card: DashCard::Network, x: 15, y: 1, w: 5, h: 1 },
        ];
        cards.push(CardLayout { card: DashCard::Media, x: 0, y: 1, w: 4, h: 1 });
        let (packed, _) = pack_cards_max(&cards, DASH_COLS);
        let m = packed.iter().find(|l| l.card == DashCard::Media).unwrap();
        assert_eq!(m.y, 0, "jumped up into the row above");
        assert_eq!(m.x, 16, "landed in the RIGHT-most empty slot of row 0");
    }

    #[test]
    fn packs_within_custom_width() {
        // pack_cards_max packs within a caller-chosen column count.
        let c = CardLayout { card: DashCard::System, x: 0, y: 0, w: 30, h: 1 };
        let (packed, maxr) = pack_cards_max(&[c], 40);
        assert_eq!(packed[0].x, 0);
        assert_eq!(packed[0].w, 30);
        assert_eq!(maxr, 1);
        let (packed_std, _) = pack_cards_max(&[c], DASH_COLS);
        assert_eq!(packed_std[0].w, 20, "standard packer clamps to DASH_COLS");
    }
}

/// Edit-mode drag plumbing: `edit_press` / `edit_motion` / `edit_release`
/// drive card move, resize, close-to-tray and banner-chip reorder. These
/// reconstruct ~350 lines of input handlers, so they get structure tests here
/// (pure shell logic) in addition to the live-session smoke check.
#[cfg(test)]
mod edit_drag_tests {
    use super::*;

    /// A shell in edit mode with a single 2×2 card parked at the grid origin.
    fn edit_shell() -> Shell {
        let cfg: Config =
            toml::from_str(crate::config::DEFAULT_SHELL_TOML).expect("default config parses");
        let mut s = Shell::new(cfg);
        s.dash_edit = true;
        s.dash_enabled = true;
        s.dash_layout = vec![CardLayout { card: DashCard::System, x: 0, y: 0, w: 2, h: 2 }];
        s.refresh_pack();
        s
    }

    fn cell_metrics(s: &Shell) -> (f32, f32, f32, f32, f32) {
        let ew = s.cfg.expanded_w();
        (s.dash_cell_base_w(ew), s.dash_cell_h(ew), s.dash_pad(), s.dash_gap(), s.grid_top())
    }

    #[test]
    fn press_then_motion_moves_card_and_commits() {
        let mut s = edit_shell();
        let (cw, ch, pad, gap, gt) = cell_metrics(&s);
        // press the card body at its exact top-left corner → zero offsets, so
        // the pointer maps back to whole-cell slots cleanly
        s.edit_press(pad, gt);
        let drag = s.edit_drag.expect("press on the card body starts a move-drag");
        assert_eq!(drag.0, DashCard::System);
        assert_eq!(drag.1, 0.0, "off_x is 0 (corner press)");
        assert_eq!(drag.2, 0.0, "off_y is 0 (corner press)");
        // drag to the cell at slot (4, 3)
        let nx = pad + 4.0 * (cw + gap);
        let ny = gt + 3.0 * (ch + gap);
        assert!(s.edit_motion(nx, ny), "motion over a new slot reports a change");
        let l = s.dash_layout.iter().find(|l| l.card == DashCard::System).unwrap();
        assert_eq!(l.x, 4, "card snaps to the column under the pointer");
        assert_eq!(l.y, 3, "card snaps to the row under the pointer");
        assert!(s.edit_ghost_slot.is_some(), "a ghost slot previews the move");
        assert!(s.edit_release(nx, ny), "release reports a committed change");
        assert_eq!(s.edit_drag, None, "drag cleared on release");
        assert_eq!(s.edit_ghost_slot, None, "ghost cleared on release");
        let l = s.dash_layout.iter().find(|l| l.card == DashCard::System).unwrap();
        assert_eq!((l.x, l.y), (4, 3), "position is committed after release");
    }

    #[test]
    fn motion_to_same_slot_is_not_a_change() {
        let mut s = edit_shell();
        let (_, _, pad, _, gt) = cell_metrics(&s);
        s.edit_press(pad, gt);
        // first motion seeds the ghost slot (None → Some) …
        let nx = pad; // stuck on (0,0) — the card's home slot
        let ny = gt;
        assert!(s.edit_motion(nx, ny), "first motion establishes the ghost");
        // … a repeated motion over the SAME slot is then a no-op
        assert!(!s.edit_motion(nx, ny), "same-slot motion reports no change");
        assert_eq!(s.edit_ghost_slot, Some((0, 0)));
    }

    #[test]
    fn close_button_parks_card_on_tray() {
        let mut s = edit_shell();
        let (cw, ch, pad, gap, gt) = cell_metrics(&s);
        let l = s.dash_layout[0];
        let (gx, gy, gw, gh) = Shell::card_close_px(l, cw, ch, gt, pad, gap, s.scale);
        s.edit_press(gx + gw / 2.0, gy + gh / 2.0);
        assert!(s.dash_layout.is_empty(), "close removes the card from the grid");
        assert_eq!(s.tray_cards, vec![DashCard::System], "and parks it on the tray");
        assert!(s.edit_drag.is_none(), "no drag started by a close");
    }

    #[test]
    fn close_button_hit_rect_follows_board_scroll() {
        // Regression: the ✕ rect used to be computed without the board's
        // scroll offset — with the board panned, the drawn ✕ and its hit
        // rect disagreed and a click started a DRAG instead of parking.
        let mut s = edit_shell();
        let (cw, ch, pad, gap, gt) = cell_metrics(&s);
        s.dash_scroll_x = 11.0;
        s.dash_scroll_y = 37.0;
        let l = s.dash_layout[0];
        let (gx, gy, gw, gh) = s.scrolled(Shell::card_close_px(l, cw, ch, gt, pad, gap, s.scale));
        s.edit_press(gx + gw / 2.0, gy + gh / 2.0);
        assert!(s.dash_layout.is_empty(), "close parks the card at the scrolled position");
        assert_eq!(s.tray_cards, vec![DashCard::System], "and parks it on the tray");
        assert!(s.edit_drag.is_none(), "no drag started by a close");
    }

    #[test]
    fn grip_press_resizes_and_clamps_to_grid() {
        let mut s = edit_shell();
        let (cw, ch, pad, gap, gt) = cell_metrics(&s);
        let l = s.dash_layout[0];
        let grip = s.grip_px(l);
        s.edit_press(grip.0 + grip.2 / 2.0, grip.1 + grip.3 / 2.0);
        let res = s.edit_resize.expect("grip press starts a resize");
        assert_eq!(res.0, DashCard::System);
        // drag far beyond the grid edge: the card must grow but stay clamped
        let nx = pad + 40.0 * (cw + gap);
        let ny = gt + 40.0 * (ch + gap);
        assert!(s.edit_motion(nx, ny));
        let l = s.dash_layout.iter().find(|l| l.card == DashCard::System).unwrap();
        assert!(l.w >= 2 && l.h >= 2, "resize grows the card");
        assert!(l.w <= s.grid_max_cols(), "width clamps to the visible grid");
        assert!(s.edit_release(nx, ny));
        assert_eq!(s.edit_resize, None, "resize cleared on release");
    }

    #[test]
    fn release_with_nothing_in_flight_is_noop() {
        let mut s = edit_shell();
        assert!(!s.edit_release(0.0, 0.0), "bare release changes nothing");
        assert!(s.edit_drag.is_none());
        assert!(s.edit_resize.is_none());
    }
}

#[cfg(test)]
mod morph_size_protocol_tests {
    use super::*;
    use crate::app::App;
    use std::time::Duration;

    /// The hover-expand deadlock (2026-09-14): the final tween commit
    /// 1009.79×494.79 truncates to the acked configure 1009×494 — the
    /// compositor already has that size, so re-sending set_size is a no-op
    /// it never acks. `commit_size` must classify it as already-configured.
    #[test]
    fn final_fractional_tween_size_counts_as_configured() {
        assert!(App::size_is_configured((1009, 494), 1009.7905, 494.79385));
        assert!(App::size_is_configured((302, 38), 302.58002, 38.0));
        assert!(App::size_is_configured((0, 0), 0.4, 0.9));
    }

    /// A genuinely different size must still go out as a real commit.
    #[test]
    fn different_size_is_norconfigured() {
        assert!(!App::size_is_configured((1009, 494), 1010.0, 494.79));
        assert!(!App::size_is_configured((302, 38), 1010.0, 495.0));
    }

    /// Shell side: with `size_pending` set (the never-acked commit),
    /// `tick()` must NOT clear the anim — the surface hasn't been told the
    /// final size yet. This is what stranded the pill/dashboard mid-morph:
    /// anim stuck + every render dropped while pending.
    #[test]
    fn tick_holds_anim_while_size_pending() {
        let cfg: Config =
            toml::from_str(crate::config::DEFAULT_SHELL_TOML).expect("default config parses");
        let mut s = Shell::new(cfg);
        s.start_morph(1009.79, 494.79);
        s.size_pending = true;
        // run well past the 250 ms duration
        let later = Instant::now() + Duration::from_millis(500);
        assert!(s.tick(later).is_none(), "pending commit holds the anim");
        assert!(s.anim.is_some(), "anim must not complete while pending");
        // once the pending clears, the same tick completes and offers the
        // final size — the app layer then classifies it as already-configured
        // and un-sticks instead of re-sending a no-op commit.
        s.size_pending = false;
        assert_eq!(s.tick(later), Some((1009.79, 494.79)));
        assert!(s.anim.is_none(), "anim completes once the commit lands");
    }
}
