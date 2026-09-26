# Handoff — Material Symbols icon migration (zen-shell)

Date: 2026-09-11. Work in progress, continue tomorrow where id.

## Goal
The bar icons (wifi / connectivity / volume) looked clipped. Root cause was
**two** things, both now fixed:

1. The icon font was **never loaded**:
   - The font scanner only walked **2 directory levels**, but the Material
     Symbols fonts sit 3 deep
     (`~/.local/share/fonts/fonts_collection/google/*.ttf`) → not scanned.
   - Requested family is `"Material Symbols"` but installed families are
     `Material Symbols Rounded/Outlined/Sharp` and the match was **exact** → failed.
2. Every icon site used **Nerd-Font PUA codepoints** (`\u{f1eb}` etc.) that do
   not exist in Material Symbols → 7px `.notdef` box.

Fixed in `src/text.rs`:
- `load_configured_families()` → depth-limited recursive walk (max depth 4) via
  new `load_font_dir()`.
- new `family_satisfies(f, w)`: exact name, or prefix/short-name match
  ("Material Symbols" matches "... Rounded"). Never lets a longer *requested*
  name wildcard (e.g. "Material Symbols Rounded" won't grab plain "Material").
- `family_matches()` deleted (was exact-only).

Verified via temporary test: with family "Material Symbols" + correct
codepoint `\u{e63e}` the engine measures **13px wide** (real icon), versus 7px
box before.

## DONE
- `src/text.rs` font load fix (recursive scan + prefix match). Build clean,
  63/63 tests pass (after removing temp tests).
- `src/shell/mod.rs` `BannerItem::glyph()` → all Material codepoints.
- `src/ui.rs` `battery_glyph()` → Material battery set.
- `src/shell/panels/dashboard.rs` fully remapped (bar chips, tray hint,
  connectivity popover, edit-row toggles).

## Codepoint reference (VERIFIED against Material Symbols Rounded)

### Applied mappings
```
mod.rs glyph():      f108→e9b0  f2d2→e30c f073→e935  f017→efd6 f002→e8b6  f0c0→e5c3
                     f1eb→e63e f293→e1a7 f0c1→e250  f240→e1a5 f6c4→f172  f085→e9e4
                     f1c0→e322 f0ec→e8d5 f1f6→e7f6  f028→e050 f185→e430  f023→e899
                     f013→e8b8 f011→e646 f00a→e9b0
ui.rs battery_glyph: f0084→e1a3 (charging)  f007a→ebdc  f007c→f09e  f007e→f0a0
                     f0080→f0a1  f0079→e1a5
dashboard.rs:        f00d→e5cd  f0ae→e894  f019→f090  f07e→e5db  f07d→e5d8
                     f111→e061  f013→e8b8  f011→e646 f002→e8b6  f1eb→e63e
                     f293→e1a7  f0c1→e250  f042→e798  f085→e9e4  f1c0→e322
                     f1f6→e7f6  f0f3→e7f5  f6a9→e04e  f028→e050  f185→e430
                     f023→e899  f054→e5cc
```

### Verified name→codepoint (for the remaining sweep)
```
add e145          apps e5c3      arrow_back e5c4  arrow_forward e5c8
arrow_downward e5db  arrow_upward e5d8
battery_alert e19c  battery_charging_full e1a3  battery_full e1a5
battery_0_bar ebdc  battery_1_bar f09c  battery_2_bar f09d  battery_3_bar f09e
battery_4_bar f09f  battery_5_bar f0a0  battery_6_bar f0a1  battery_low f155
block f08c        bluetooth e1a7  bluetooth_disabled e1a9  bolt ea0b
brightness_6 e3ab brightness_medium e1ae  cable efe6  calendar_today e935
cell_tower ebba  check e5ca  check_circle f0be  chevron_left e5cb
chevron_right e5cc  circle ef4a  close e5cd  cloud f15c  cloud_off e2c1
dark_mode e51c   dehaze e3c7   desktop_windows e30c  do_not_disturb_on f08f
download f090   edit f097   error f8b6   event e878   expansion e87b
favorite e87e  fiber_manual_record e061  grid_view e9b0  headphones f01f
home e9b2  info e88e  keyboard_tab e31c  key e73c  language e894  light_mode e518
link e250  list_alt e0ee  lock e899  lock_clock ef57  lock_open e898  logout e9ba
login ea77  memory e322  menu e5d2  mic e31d  mic_off e02b  monitor ef5b
more_horiz e5d3  music_note e405  network_check e640  nightlight f03d
north f1e0  notifications e7f5  notifications_off e7f6
open_in_new e89e  partly_cloudy_day f172  partly_cloudy_night f174
pause e034  person f0d3  play_arrow e037  power e63c  power_off e646
power_settings_new f8c7  public e80b  radio_button_checked e837  refresh e5d5
remove e15b  router e328  schedule efd6  search e8b6  settings e8b8
settings_input_antenna e8bf  settings_power e8c6  signal_cellular_4_bar e1c8
signal_wifi_4_bar f065  sim_card e32b  skip_next e044  skip_previous e045
speed e9e4  star f09a  storage e1db  sunny e81a  swap_horiz e8d4  swap_vert e8d5
sync e627  tablet e32f  timer e425  tv e63b  usb e1e0  volume_mute e04e
volume_off e04f  volume_up e050  vpn_key e0da  water_drop e798  wb_sunny e430
west f1e6  wifi e63e  wifi_off e648  east f1df  south f1e3  north_east f1e1
south_west f1e5  expand_more e5cf  expand_less e5ce  ac_unit eb3b  smart_button f1c1
```

## REMAINING sweep (still Nerd PUA → boxes)
Files (skip `.bak` and `src/bin/font_diag.rs` — diag tool intentionally uses
Nerd glyphs):
`src/app/mod.rs`, `src/weather.rs`,
`src/shell/panels/settings.rs`, `app_panels.rs`, `system_panels.rs`,
`session.rs`, `polkit_auth.rs`, `mod.rs`,
`cards/{activewin,apps,batteryh,batteryv,bluetooth,calendar,currency,eq,gpu,`
`kblayout,lyrics,media,mirror,network,news,notes,pkgupdates,powerh,powerv,`
`quote,recent,sensors,shared,sliders,speedtest,sshvpn,system,thermal,ticker,`
`todo,toggles,viz,wallpaper,weather,wifi}.rs`

Codes still present (only non-.bak, non-font_diag files):
```
a. weather.rs:        f0c2, f043, f2dc, f185, f0e7  (+ f140..f147 in sensors.rs
                      are Weather-Icons WMO glyphs — review before mapping)
b. system_panels.rs:  f053,f028,f04c,f023,f6a9,f2dc,f293,f1eb,f186,f185,f105,
                      f08b,f04b,f048,f03e,f021,f011,f00c
c. settings.rs:       f303,f0b2,f108,f06e,f5d4,f573,f2d0,f293,f1fb,f1cc,f0f3,
                      f0ec,f0e2,f0d8,f0d7,f0c9,f0c7,f078,f077,f065,f05a,f047,
                      f042,f025,f017,f00c,f009,f007,f004,f001,f240,f1eb,f185,
                      f0e7,f0c0,f043,f028,f013, f00f, f132
d. app/mod.rs:        f6a9,f028,f185,f0e7,f0ac,f011
e. session.rs:        f11c,f08b,f054,f053,f021,f011
f. app_panels.rs:     f053,f002,f489,f0c5
g. polkit_auth.rs:    f132
h. cards/*: see grep list from the session (f00c check f489 apps f0e7 bolt
   f186 brightness-low f023 lock f021 refresh f6a9 muted f042 drop f0f3 bell
   f013 gear f240 battery f11c keyboard f0e2 exchange f0b2 sliders f303 gear
   f108 desktop f06e folder f2dc moon f08b restart f04b play f04c pause
   f048 skip-prev f051 skip-next f00d X f063 arrow-up f062 arrow-up f0ac
   lock-open f05b shield f0d0 dash f03e/moon f0f4 moon f0f6 note f067 plus
   f065 wifi f078 chevron-down f077 chevron-up f2d0 window f2db restore f132
   key f1fc contrast f1fb fingerprint f1cc ? f2c9 chip f554 gauge f6c7 temp
   f256 checkbox f15c folder-open f155 currency f0c7 comment f10d quote f1ea
   newspaper f08e warning f001 music f131,f130 ?
```

## TODO (tomorrow)
1. Re-verify remaining-code inventory (`rg -o '\\u\{f[0-9a-f]+\}' src/`) after
   the dashboard sweep.
2. Sweep the panels/cards with the reference table; view each site for context
   where semantics are unclear (f0e2, f131/f130, f1cc, weather WMO f140..f147).
   Use the temp `dump2` test (currently in `src/text.rs`) to look up new
   Material names; add names to its `wanted` array, `cargo test dump2 -- --nocapture`.
   Known gaps to look up: swap_vert done, "play_arrow" done; remaining lookups
   e.g. `battery_charging` done; add as needed.
3. REMOVE the temporary `#[cfg(test)] mod dump_t` (panicking `dump2` test) from
   `src/text.rs` when done.
4. `cargo build` (expect only the 3 pre-existing warnings: BANNER_RESIZE_KEY_BASE,
   banner_chip_widths/banner_resize never read, unreachable pattern) and
   `cargo test` (expect 63 pass).
5. Runtime sanity: icons in the bar/banner should now render real Material
   Symbols glyphs (13px fixtures verified in measurement).

## Notes
- The earlier "clipped" look was the 7px `.notdef` box, not layout overflow
  (strip natural width 1049.6 < 1120; all chips inside panel) — no scissor fix
  needed.
- Do NOT touch `src/shell/panels/dashcards.rs.bak`.
- Battery cards (batteryh/batteryv/powerh/powerv) use `f0e7` (bolt) → `ea0b`.