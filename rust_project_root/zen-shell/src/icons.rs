//! Icon font — the single source of truth for which font renders the icon
//! glyphs, the mapping tag, and EVERY glyph codepoint the shell draws.
//!
//! # Changing the icon font
//! 1. Pick a style in Settings → Appearance → "Icon font" (runtime apply,
//!    no restart, no recompile). The choice is persisted to the per-channel
//!    config (`fonts.icon_style`).
//! 2. The glyph SLOTS per font live **only** in `config/icons/<style>.conf`
//!    (Google / Caskaydia / JetBrains / Phosphor) — bash-sourceable
//!    `ICON_NAME=hex` lines. A slot table is embedded from the chosen conf at
//!    build time (`IconStyle::table`) and applied at draw time: every `ICON_*`
//!    constant keeps its canonical codepoint in Rust; `slot_map` translates it
//!    to the active font's slot when text is rasterized (text.rs).
//! 3. `config/icons/current.conf` records the codepoints currently hardcoded
//!    below (the draw-time recognition keys). Regenerate it with
//!    `python3 /tmp/opencode/build_conf.py` whenever the constants change.
//!
//! The glyphs are split into two groups, mirroring how the shell uses them:
//!  - **outline** (U+F…) — the default (unfilled) set.
//!  - **filled** (U+E…) — the filled variant (banner strip + edit UI).
//! A "…_FILL" suffix marks the filled variant of an otherwise-identical icon.

/// Font-flavored glyph-set (PUA codepoints), one per supported icon font.
/// All SLOT DATA lives in `config/icons/<style>.conf` (sourcable files) —
/// this enum only names a style; it contains no codepoints.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconStyle {
    Google,
    Caskaydia,
    JetBrains,
    Phosphor,
}

impl IconStyle {
    pub fn parse(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "caskadia" | "caskaydia" | "caskaydia_cove" => IconStyle::Caskaydia,
            "jetbrains" | "jetbrains_mono" | "jetbrainsmono" => IconStyle::JetBrains,
            "phosphor" => IconStyle::Phosphor,
            _ => IconStyle::Google,
        }
    }

    /// The active slot table, keyed by icon NAME (`ICON_NOTE` → codepoint).
    /// Embedded from the sourcable `config/icons/<style>.conf` at compile time
    /// via `include_str!` — edit the conf file, rebuild, done. No codepoint
    /// lives in Rust source.
    pub fn table(self) -> std::collections::HashMap<String, char> {
        parse_slot_conf(match self {
            IconStyle::Google => include_str!("../config/icons/google.conf"),
            IconStyle::Caskaydia => include_str!("../config/icons/caskadia.conf"),
            IconStyle::JetBrains => include_str!("../config/icons/jetbrains.conf"),
            IconStyle::Phosphor => include_str!("../config/icons/phosphor.conf"),
        })
    }

    /// Which font family the text engine must resolve this style's glyphs
    /// against. "Material Symbols" generically matches the Rounded/… variants.
    pub fn family(self) -> &'static str {
        match self {
            IconStyle::Google => "Material Symbols",
            IconStyle::Caskaydia => "CaskaydiaCove Nerd Font",
            IconStyle::JetBrains => "JetBrainsMono Nerd Font",
            IconStyle::Phosphor => "Phosphor",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            IconStyle::Google => "Google / Material Symbols",
            IconStyle::Caskaydia => "Caskaydia Cove Nerd",
            IconStyle::JetBrains => "JetBrains Mono Nerd",
            IconStyle::Phosphor => "Phosphor",
        }
    }

    pub fn tag(self) -> &'static str {
        match self {
            IconStyle::Google => "google",
            IconStyle::Caskaydia => "caskadia",
            IconStyle::JetBrains => "jetbrains",
            IconStyle::Phosphor => "phosphor",
        }
    }
}

/// Translate a drawn icon codepoint (one of this file's `ICON_*` constants)
/// to the equivalent glyph slot in `out`'s icon font.
///
/// The map is built by joining the ACTIVE style's table
/// (`<style>.conf`: name → slot) with the CURRENT constants' table
/// (`current.conf`: name → the slot hardcoded in this file). Styles reuse the
/// recognition key (the constant name) so the shell can swap fonts without
/// touching a single call site.
pub fn slot_map(out: IconStyle) -> std::collections::HashMap<char, char> {
    let current = parse_slot_conf(include_str!("../config/icons/current.conf"));
    let target = out.table();
    current
        .into_iter()
        .filter_map(|(name, from)| target.get(&name).map(|&to| (from, to)))
        .collect()
}

fn parse_slot_conf(text: &str) -> std::collections::HashMap<String, char> {
    text.lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                return None;
            }
            let (name, hex) = line.split_once('=')?;
            let cp = u32::from_str_radix(hex.trim(), 16).ok()?;
            Some((name.trim().to_string(), char::from_u32(cp)?))
        })
        .collect()
}

// ── outline set (U+F…) ───────────────────────────────────────────────────────

/// music_note — Viz / Lyrics / Media card, "Visualizer" chip, "Music" chip.
pub const ICON_NOTE: &'static str = "\u{f001}";
/// search — launcher / app-panel search box.
pub const ICON_SEARCH: &'static str = "\u{f002}";
/// accent "Start icon" row glyph (accent picker).
pub const ICON_SPARKLE: &'static str = "\u{f004}";
/// person — "User info" pill chip.
pub const ICON_USER: &'static str = "\u{f007}";
/// "Open Control Center" chip (rows glyph).
pub const ICON_ROWS: &'static str = "\u{f009}";
/// check — lock confirm, saved/ok, wifi/bt connected, pkg-updates done.
pub const ICON_CHECK: &'static str = "\u{f00c}";
/// close (×) — remove app, todos, list headers.
pub const ICON_CLOSE: &'static str = "\u{f00d}";
/// power_settings_new — Shutdown button / power-off glyph, AC-unplugged OSD.
pub const ICON_POWER: &'static str = "\u{f011}";
/// settings (gear) — Misc nav, "Settings chip", GPU card header.
pub const ICON_SETTINGS: &'static str = "\u{f013}";
/// schedule — "Clock" pill chip.
pub const ICON_CLOCK: &'static str = "\u{f017}";
/// recycle-ish — Quote card "…  new" button.
pub const ICON_RECYCLE: &'static str = "\u{f01e}";
/// refresh — Restart, power-save on, speedtest "run again".
pub const ICON_REFRESH: &'static str = "\u{f021}";
/// lock — Lock button, secured entries, wifi auth, VPN.
pub const ICON_LOCK: &'static str = "\u{f023}";
/// toggle-off state glyph (chip where the ON glyph is ICON_SUNNY).
pub const ICON_TOGGLE_OFF: &'static str = "\u{f025}";
/// volume_up — volume everywhere (tiles, sliders, cards, OSD).
pub const ICON_VOLUME: &'static str = "\u{f028}";
/// videocam — Mirror (camera) card header.
pub const ICON_CAMERA: &'static str = "\u{f03d}";
/// photo — Wallpaper card header / wallpaper tile.
pub const ICON_WALLPAPER: &'static str = "\u{f03e}";
/// wb_sunny — weather sun glyph, toggle-on chip glyph.
pub const ICON_SUNNY: &'static str = "\u{f042}";
/// opacity drop — weather rain/drizzle/snow mix, accent-alpha, saturation slider.
pub const ICON_DROP: &'static str = "\u{f043}";
/// "Floating" pill chip.
pub const ICON_FLOATING: &'static str = "\u{f047}";
/// skip_previous — media previous button.
pub const ICON_PREV: &'static str = "\u{f048}";
/// play_arrow — play button, speedtest "run test".
pub const ICON_PLAY: &'static str = "\u{f04b}";
/// pause — pause button / playing state.
pub const ICON_PAUSE: &'static str = "\u{f04c}";
/// skip_next — media next button.
pub const ICON_SKIP_NEXT: &'static str = "\u{f051}";
/// chevron_left — back affordance (panels, calendar previous, app panels).
pub const ICON_BACK: &'static str = "\u{f053}";
/// chevron_right — forward affordance (calendar next, session, accent rows).
pub const ICON_FORWARD: &'static str = "\u{f054}";
/// info — info glyph (settings dialogs).
pub const ICON_INFO: &'static str = "\u{f05a}";
/// power-save off-state glyph (Shared power-save card).
pub const ICON_BATTERY_SAVER: &'static str = "\u{f05b}";
/// arrow up — network upload.
pub const ICON_UP: &'static str = "\u{f062}";
/// arrow down — network download / pkg-updates available.
pub const ICON_DOWN: &'static str = "\u{f063}";
/// "Expand on hover" chip (arrows-out glyph).
pub const ICON_EXPAND: &'static str = "\u{f065}";
/// add (+) — notes "new note" button.
pub const ICON_ADD: &'static str = "\u{f067}";
/// visibility (eye) — balance / info side labels.
pub const ICON_VISIBILITY: &'static str = "\u{f06e}";
/// calendar_today — reserved: no current call site (kept so the full set is
/// documented; the banner Date / "Calendar" chip use the filled variant
/// `ICON_CALENDAR_FILL`).
#[allow(dead_code)]
pub const ICON_CALENDAR: &'static str = "\u{f073}";
/// bar anchor "top" glyph (bar-anchor toggle, top state).
pub const ICON_ANCHOR_UP: &'static str = "\u{f077}";
/// bar anchor "bottom" glyph (bar-anchor toggle, bottom state).
pub const ICON_ANCHOR_DOWN: &'static str = "\u{f078}";
/// logout — Logout button.
pub const ICON_LOGOUT: &'static str = "\u{f08b}";
/// open-in-new-ish — Recent / News "open" affordance.
pub const ICON_OPEN: &'static str = "\u{f08e}";
/// dashboard edit "arrange / gravity" button.
pub const ICON_ARRANGE: &'static str = "\u{f090}";
/// battery_3_bar — vertical battery ~30%.
pub const ICON_BATTERY_3: &'static str = "\u{f09e}";
/// battery_5_bar — vertical battery ~50%.
pub const ICON_BATTERY_5: &'static str = "\u{f0a0}";
/// battery_6_bar — vertical battery ~70%.
pub const ICON_BATTERY_6: &'static str = "\u{f0a1}";
/// public/globe — Workspace OSD, VPN/SSH card.
pub const ICON_GLOBE: &'static str = "\u{f0ac}";
/// square glyph — Reserve space, App border, accent alpha, settings rows.
pub const ICON_SQUARE: &'static str = "\u{f0b2}";
/// app/tiling glyph — Tray icons chip, Hyprland border, normal-app bg.
pub const ICON_WINDOWS: &'static str = "\u{f0c0}";
/// cloud — weather cloudy/fog states.
pub const ICON_CLOUD: &'static str = "\u{f0c2}";
/// selected-app marker (app panels list).
pub const ICON_SELECT: &'static str = "\u{f0c5}";
/// save / folder — Quote "… save", notes-save glyph.
pub const ICON_SAVE: &'static str = "\u{f0c7}";
/// normal-app background glyph (accent picker).
pub const ICON_APP: &'static str = "\u{f0c9}";
/// Shader toggle chip.
pub const ICON_SHADER: &'static str = "\u{f0d0}";
/// chevron up — collapsible section "open" state.
pub const ICON_CHEVRON_UP: &'static str = "\u{f0d7}";
/// chevron down — collapsible section "closed" state.
pub const ICON_CHEVRON_DOWN: &'static str = "\u{f0d8}";
/// slider handle / "…Reset" glyph (sliders card + settings).
pub const ICON_SLIDER: &'static str = "\u{f0e2}";
/// bolt — AC-plugged, thunder/rain storm, PWR card, battery-charging suffix.
pub const ICON_BOLT: &'static str = "\u{f0e7}";
/// grade (star) — balance star glyph.
pub const ICON_STAR: &'static str = "\u{f0ec}";
/// notifications (bell) — Notifications card/chip.
pub const ICON_BELL: &'static str = "\u{f0f3}";
/// local_cafe — Caffeine toggle chip.
pub const ICON_CAFFEINE: &'static str = "\u{f0f4}";
/// bullet glyph — Notes card row bullets.
pub const ICON_BULLET: &'static str = "\u{f0f6}";
/// chevron affordance — tile "tap for details", toggles next-arrow.
pub const ICON_MORE: &'static str = "\u{f105}";
/// desktop/monitor — System nav, GTK app bg, Workspaces chip.
pub const ICON_MONITOR: &'static str = "\u{f108}";
/// format_quote — Quote card header.
pub const ICON_QUOTE: &'static str = "\u{f10d}";
/// dot — workspace dot (dashboard).
pub const ICON_DOT: &'static str = "\u{f111}";
/// keyboard — key-layout card, CAPS-LOCK notice, todo pin.
pub const ICON_KEYBOARD: &'static str = "\u{f11c}";
/// mic — mic level slider (unmuted).
pub const ICON_MIC: &'static str = "\u{f130}";
/// mic_off — mic level slider (muted).
pub const ICON_MIC_OFF: &'static str = "\u{f131}";
/// shield — polkit auth dialog, SSH/VPN card.
pub const ICON_SHIELD: &'static str = "\u{f132}";
/// temperature gauge, level n (Sensors card 0..7).
pub const ICON_GAUGE_0: &'static str = "\u{f140}";
pub const ICON_GAUGE_1: &'static str = "\u{f141}";
pub const ICON_GAUGE_2: &'static str = "\u{f142}";
pub const ICON_GAUGE_3: &'static str = "\u{f143}";
pub const ICON_GAUGE_4: &'static str = "\u{f144}";
pub const ICON_GAUGE_5: &'static str = "\u{f145}";
pub const ICON_GAUGE_6: &'static str = "\u{f146}";
pub const ICON_GAUGE_7: &'static str = "\u{f147}";
/// attach_money — Currency card header.
pub const ICON_MONEY: &'static str = "\u{f155}";
/// clock-replay — Recent apps card header.
pub const ICON_RECENT: &'static str = "\u{f15c}";
/// partly_cloudy_day — banner Weather chip.
pub const ICON_PARTLY_CLOUDY: &'static str = "\u{f172}";
/// sunny — brightness sun (brightness chip, slider, OSD, light mode).
pub const ICON_BRIGHTNESS: &'static str = "\u{f185}";
/// moon — dark mode / Sleep mission / Sunset toggle.
pub const ICON_MOON: &'static str = "\u{f186}";
/// shopping_bag — accent "From wallpaper" source.
pub const ICON_BAG: &'static str = "\u{f1cc}";
/// import_contacts — News card header.
pub const ICON_NEWS: &'static str = "\u{f1ea}";
/// wifi — Wi-Fi everywhere (tiles, tiles card, toggles, banner).
pub const ICON_WIFI: &'static str = "\u{f1eb}";
/// do_not_disturb — System card DND glyph.
pub const ICON_DND: &'static str = "\u{f1f6}";
/// accent scheme selector glyph.
pub const ICON_SCHEME: &'static str = "\u{f1fb}";
/// sticky_note_2 — Accent card header.
pub const ICON_STICKY: &'static str = "\u{f1fc}";
/// trending — Ticker card header.
pub const ICON_TICKER: &'static str = "\u{f201}";
/// battery_full — Battery % chip / battery rows.
pub const ICON_BATTERY: &'static str = "\u{f240}";
/// todo-row check glyph (Todos card).
pub const ICON_TODO: &'static str = "\u{f256}";
/// bluetooth — Bluetooth everywhere.
pub const ICON_BLUETOOTH: &'static str = "\u{f293}";
/// thermostat — temperature (Sensors/GPU/thermal card, color-temp slider).
pub const ICON_THERMAL: &'static str = "\u{f2c9}";
/// active-window dots — Active window card, scrim alpha.
pub const ICON_ACTIVE: &'static str = "\u{f2d0}";
/// value readout glyph (Active window card value line).
pub const ICON_GAUGE_VALUE: &'static str = "\u{f2db}";
/// snow — Snow weather, DND chip/taggles.
pub const ICON_SNOW: &'static str = "\u{f2dc}";
/// palette — Appearance/Pill nav, background alpha, scheme default.
pub const ICON_PALETTE: &'static str = "\u{f303}";
/// grid/apps — Apps card header, launcher grid.
pub const ICON_APPS: &'static str = "\u{f489}";
/// Speedtest card header (gauges).
pub const ICON_SPEEDTEST: &'static str = "\u{f554}";
/// timer — Power-menu hold-delay row.
pub const ICON_TIMER: &'static str = "\u{f573}";
/// grip handle — pill-order drag grips.
pub const ICON_GRIP: &'static str = "\u{f5d4}";
/// volume_off — muted volume everywhere.
pub const ICON_MUTE: &'static str = "\u{f6a9}";
/// sensors — Sensors card header.
pub const ICON_SENSORS: &'static str = "\u{f6c7}";

// ── filled set (U+E…) — banner strip + edit UI ───────────────────────────────

/// volume_off (filled) — dash volume chip muted state.
pub const ICON_VOLUME_OFF_FILL: &'static str = "\u{e04e}";
/// volume_up (filled) — dash volume chip.
pub const ICON_VOLUME_FILL: &'static str = "\u{e050}";
/// tray placeholder (SNI without an image).
pub const ICON_TRAY_FILL: &'static str = "\u{e061}";
/// battery_charging_full — vertical battery while charging.
pub const ICON_BATTERY_CHARGING: &'static str = "\u{e1a3}";
/// battery_full (filled) — battery ≥81% + banner Battery chip.
pub const ICON_BATTERY_FULL: &'static str = "\u{e1a5}";
/// bluetooth (filled) — banner Bluetooth chip.
pub const ICON_BLUETOOTH_FILL: &'static str = "\u{e1a7}";
/// link — banner Connectivity chip.
pub const ICON_LINK: &'static str = "\u{e250}";
/// desktop_windows — banner "focused window" chip.
pub const ICON_DESKTOP_WINDOW: &'static str = "\u{e30c}";
/// memory — banner RAM chip + dash RAM chip.
pub const ICON_MEMORY: &'static str = "\u{e322}";
/// wb_sunny (filled) — banner brightness chip + dash brightness chip.
pub const ICON_BRIGHTNESS_FILL: &'static str = "\u{e430}";
/// apps (filled) — banner tray chip.
pub const ICON_APPS_FILL: &'static str = "\u{e5c3}";
/// arrow_forward — banner chip trailing affordance.
pub const ICON_ARROW_FORWARD: &'static str = "\u{e5cc}";
/// close (filled) — edit-mode close button.
pub const ICON_CLOSE_FILL: &'static str = "\u{e5cd}";
/// swap_vert (filled) — dash scroll rows state.
pub const ICON_SWAP_VERT: &'static str = "\u{e5d8}";
/// swap_horiz (filled) — dash scroll columns state.
pub const ICON_SWAP_HORIZ: &'static str = "\u{e5db}";
/// wifi (filled) — banner wifi chip, dash wifi entries.
pub const ICON_WIFI_FILL: &'static str = "\u{e63e}";
/// power_off (filled) — banner Power chip.
pub const ICON_POWER_FILL: &'static str = "\u{e646}";
/// thermostat — dash temperature chip placeholder.
pub const ICON_THERMOSTAT: &'static str = "\u{e798}";
/// notifications (filled) — dash DND chip off-state.
pub const ICON_BELL_FILL: &'static str = "\u{e7f5}";
/// notifications_off (filled) — dash DND chip on-state.
pub const ICON_BELL_OFF_FILL: &'static str = "\u{e7f6}";
/// widgets — banner-chips parking tray title glyph.
pub const ICON_WIDGETS: &'static str = "\u{e894}";
/// lock (filled) — banner VPN chip + dash VPN chip.
pub const ICON_LOCK_FILL: &'static str = "\u{e899}";
/// search (filled) — banner AppSearch chip.
pub const ICON_SEARCH_FILL: &'static str = "\u{e8b6}";
/// settings (filled) — banner Settings chip + edit-grid settings button.
pub const ICON_SETTINGS_FILL: &'static str = "\u{e8b8}";
/// swap_vert — banner NetSpeed chip (up/down).
pub const ICON_SWAP: &'static str = "\u{e8d5}";
/// calendar_today (filled) — banner Date chip.
pub const ICON_CALENDAR_FILL: &'static str = "\u{e935}";
/// grid_view — banner Workspaces chip + DashToggle chip.
pub const ICON_GRID: &'static str = "\u{e9b0}";
/// speed (filled) — banner CPU chip + dash CPU chip.
pub const ICON_SPEED_FILL: &'static str = "\u{e9e4}";
/// battery_0_bar — vertical battery ≤10%.
pub const ICON_BATTERY_0: &'static str = "\u{ebdc}";
/// schedule (filled) — banner Clock chip.
pub const ICON_SCHEDULE: &'static str = "\u{efd6}";

#[cfg(test)]
mod tests {
    use super::*;

    /// Every style's family string must resolve to a family name that text.rs
    /// can match (the generic names text.rs matches against installed fonts).
    #[test]
    fn style_families_are_nonempty() {
        for s in [IconStyle::Google, IconStyle::Caskaydia, IconStyle::JetBrains, IconStyle::Phosphor] {
            assert!(!s.family().is_empty(), "each style needs a family");
            assert_eq!(IconStyle::parse(s.tag()), s, "tag round-trips to its style");
        }
        assert_eq!(IconStyle::parse(""), IconStyle::Google, "empty/unset → Google");
        assert_eq!(IconStyle::parse("garbage"), IconStyle::Google, "unknown → Google");
    }

    /// The draw-time translation must land every canonical constant on the
    /// active font's (verified) slot. Keeps current.conf + the style tables
    /// honest at runtime.
    #[test]
    fn slot_map_translates_every_style() {
        let g = slot_map(IconStyle::Google);
        let c = slot_map(IconStyle::Caskaydia);
        let j = slot_map(IconStyle::JetBrains);
        let p = slot_map(IconStyle::Phosphor);
        for s in [&g, &c, &j, &p] {
            assert_eq!(s.len(), 130, "all 130 constants load for every style");
        }
        // outline (recognition key = FA slot) vs filled (key = MS slot)
        let note = '\u{f001}';
        assert_eq!(g.get(&note), Some(&'\u{e3a1}'), "google music_note");
        assert_eq!(c.get(&note), Some(&note), "caskadia keeps FA music");
        assert_eq!(p.get(&note), Some(&'\u{e340}'), "phosphor music-notes");
        let search = '\u{f002}';
        assert_eq!(g.get(&search), Some(&'\u{e8b6}'), "google search");
        assert_eq!(c.get(&search), Some(&search), "caskadia keeps FA search");
        assert_eq!(p.get(&search), Some(&'\u{e30c}'), "phosphor magnifying-glass");
        // MUTE U+F6A9 is not a real FA/Material slot — it must remap everywhere
        let mute = '\u{f6a9}';
        assert_eq!(g.get(&mute), Some(&'\u{e04f}'), "google volume_off");
        assert_eq!(c.get(&mute), Some(&'\u{f026}'), "caskadia fa-volume_off");
        assert_eq!(p.get(&mute), Some(&'\u{e45a}'), "phosphor speaker-slash");
        // filled constants keep their MS identity on google, remap on nerd
        assert_eq!(g.get(&'\u{ebdc}'), Some(&'\u{ebdc}'), "google battery_0 identity");
        assert_eq!(c.get(&'\u{ebdc}'), Some(&'\u{f244}'), "caskadia fa-battery_empty");
        assert_eq!(p.get(&'\u{ebdc}'), Some(&'\u{e0be}'), "phosphor battery-empty");
    }

    /// Every icon constant must hold a valid codepoint (no stray 0 / garbage).
    #[test]
    fn codepoints_are_private_use() {
        let all = [
            ICON_NOTE, ICON_SEARCH, ICON_SPARKLE, ICON_USER, ICON_ROWS, ICON_CHECK,
            ICON_CLOSE, ICON_POWER, ICON_SETTINGS, ICON_CLOCK, ICON_RECYCLE, ICON_REFRESH,
            ICON_LOCK, ICON_TOGGLE_OFF, ICON_VOLUME, ICON_CAMERA, ICON_WALLPAPER, ICON_SUNNY,
            ICON_DROP, ICON_FLOATING, ICON_PREV, ICON_PLAY, ICON_PAUSE, ICON_SKIP_NEXT,
            ICON_BACK, ICON_FORWARD, ICON_INFO, ICON_BATTERY_SAVER, ICON_UP, ICON_DOWN,
            ICON_EXPAND, ICON_ADD, ICON_VISIBILITY, ICON_CALENDAR, ICON_ANCHOR_UP,
            ICON_ANCHOR_DOWN, ICON_LOGOUT, ICON_OPEN, ICON_ARRANGE, ICON_BATTERY_3,
            ICON_BATTERY_5, ICON_BATTERY_6, ICON_GLOBE, ICON_SQUARE, ICON_WINDOWS, ICON_CLOUD,
            ICON_SELECT, ICON_SAVE, ICON_APP, ICON_SHADER, ICON_CHEVRON_UP, ICON_CHEVRON_DOWN,
            ICON_SLIDER, ICON_BOLT, ICON_STAR, ICON_BELL, ICON_CAFFEINE, ICON_BULLET, ICON_MORE,
            ICON_MONITOR, ICON_QUOTE, ICON_DOT, ICON_KEYBOARD, ICON_MIC, ICON_MIC_OFF,
            ICON_SHIELD, ICON_GAUGE_0, ICON_GAUGE_1, ICON_GAUGE_2, ICON_GAUGE_3, ICON_GAUGE_4,
            ICON_GAUGE_5, ICON_GAUGE_6, ICON_GAUGE_7, ICON_MONEY, ICON_RECENT,
            ICON_PARTLY_CLOUDY, ICON_BRIGHTNESS, ICON_MOON, ICON_BAG, ICON_NEWS, ICON_WIFI,
            ICON_DND, ICON_SCHEME, ICON_STICKY, ICON_TICKER, ICON_BATTERY, ICON_TODO,
            ICON_BLUETOOTH, ICON_THERMAL, ICON_ACTIVE, ICON_GAUGE_VALUE, ICON_SNOW,
            ICON_PALETTE, ICON_APPS, ICON_SPEEDTEST, ICON_TIMER, ICON_GRIP, ICON_MUTE,
            ICON_SENSORS, ICON_VOLUME_OFF_FILL, ICON_VOLUME_FILL, ICON_TRAY_FILL,
            ICON_BATTERY_CHARGING, ICON_BATTERY_FULL, ICON_BLUETOOTH_FILL, ICON_LINK,
            ICON_DESKTOP_WINDOW, ICON_MEMORY, ICON_BRIGHTNESS_FILL, ICON_APPS_FILL,
            ICON_ARROW_FORWARD, ICON_CLOSE_FILL, ICON_SWAP_VERT, ICON_SWAP_HORIZ,
            ICON_WIFI_FILL, ICON_POWER_FILL, ICON_THERMOSTAT, ICON_BELL_FILL,
            ICON_BELL_OFF_FILL, ICON_WIDGETS, ICON_LOCK_FILL, ICON_SEARCH_FILL,
            ICON_SETTINGS_FILL, ICON_SWAP, ICON_CALENDAR_FILL, ICON_GRID, ICON_SPEED_FILL,
            ICON_BATTERY_0, ICON_SCHEDULE,
        ];
        for s in all {
            let c = s.chars().next().unwrap();
            assert!(
                (0xE000..=0xF8FF).contains(&(c as u32)),
                "icon codepoint U+{:04X} is outside the private-use area",
                c as u32
            );
        }
    }
}