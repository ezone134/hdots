//! Per-panel scene layouts (one `layout_*` per mode).

use super::*;
mod app_panels;
mod cards;
pub(crate) mod dashboard;
pub(crate) mod layout;
mod overlays;
mod polkit_auth;
mod session;
mod settings;
mod system_panels;

/// Transient per-frame context the `dashboard` surface's Rust body Inks need:
/// the pre-computed `Layout` (cell grid, strip zone, scroll), the fluid-drag
/// cursor, and the scene values used to draw each card's own `.ron` scene.
/// Built in `layout_expanded` the one frame it consults the surface, then
/// dropped — the dashboard never stores layout state.
pub(crate) struct DashDrawCtx<'a> {
    pub lt: &'a layout::Layout,
    pub cursor: Option<(f32, f32)>,
    pub vals: &'a crate::scene::SceneValues,
}

impl Shell {
    pub fn layout(&mut self, out_w: f32, out_h: f32) -> Vec<Cmd> {
        self.layout_for(self.mode, out_w, out_h)
    }

    /// Layout the scene for a specific mode (not necessarily the current one).
    /// The session-lock surface renders `Mode::Lock` while the bar itself
    /// renders its resting mode behind it.
    pub fn layout_for(&mut self, mode: Mode, out_w: f32, out_h: f32) -> Vec<Cmd> {
        // the moon's phase is pure date math off the wall clock: sample it ONCE
        // here, so the disc, its caption and the header meta are all the same
        // instant even if the frame straddles a second
        self.moon_epoch = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0);
        self.hover_regions.clear();
        self.scene_click_actions.clear();
        // Before any `scene_values()`: a store's declared default has to be in
        // the pool on the FIRST frame the surface draws, and the pool is built
        // per-panel below.
        self.seed_scene_stores();
        // the wheel hit-tests these card boxes, so they must describe THIS
        // frame: a card that is not on the board leaves no box behind to eat
        // notches aimed at whatever is really under the pointer
        self.clear_frame_wheel_rects();
        // LIVE colors: every fg/bg/… comes from $states2/shell_vars (read on
        // boot and refreshed by `zen-shell ipc call colors reload`); the
        // config [colors] table is only the boot-time fallback.
        let fg = self.sv_fg;
        let bg = self.sv_bg;
        // accent color is the live `$acc` from $states2/shell_vars
        let acc = self.sv_acc;
        let pal = Pal {
            fg,
            bg,
            acc,
            sfg: self.sv_sfg,
        };
        let mut v = Vec::new();
        // While a morph is in flight the surface is mid-size, but contents are
        // laid out at the mode's TARGET size: they hold still (revealed or
        // clipped by the growing/shrinking surface) instead of reflowing every
        // stepped frame — the "clock slides / contents jump" glitch during
        // expand-collapse. The frame follows the current size, so the box
        // itself still animates; the renderer clips to the buffer, so content
        // outside the current surface is simply cut off. At rest the two are
        // identical, so nothing changes.
        let (tw, th) = self.target_size();
        let (lw, lh) = if (tw - out_w).abs() < 0.5 && (th - out_h).abs() < 0.5 {
            (out_w, out_h)
        } else {
            (tw, th)
        };
        // The spring overshoot can make the surface briefly LARGER than the
        // target: shift the content to stay centered (zero at rest / while
        // the surface is smaller — then the existing clipped-reveal applies).
        self.content_dx = ((out_w - lw) / 2.0).max(0.0);
        self.content_dy = ((out_h - lh) / 2.0).max(0.0);
        // the pill is a capsule; every widget (dashboard, panels) is square-ish
        // with a generous rounded-corner radius. The corner radius itself
        // morphs with the surface (Dynamic Island: capsule → panel corners),
        // so it never snaps when the mode flips.
        let r = if mode == Mode::Collapsed {
            (out_h / 2.0).max(1.0)
        } else {
            let t = self.morph_progress();
            let cap = (out_h / 2.0).max(1.0);
            cap + (self.panel_r() - cap) * t
        };
        // When the surface sticks flush to a screen edge (Floating off) the two
        // edge corners go square so the pill/dashboard reads as a pendant
        // hanging from that edge: top edge → bottom-left + bottom-right rounded,
        // bottom edge → top-left + top-right rounded. Floating keeps all four
        // corners shaped (rounded capsule / panel).
        #[rustfmt::skip]
        let (r_tl, r_tr, r_br, r_bl) = if self.bar_floating {
            (r, r, r, r)
        } else {
            match self.bar_edge {
                crate::shell::BarEdge::Top => (0.0, 0.0, r, r),
                crate::shell::BarEdge::Bottom => (r, r, 0.0, 0.0),
                crate::shell::BarEdge::Left => (0.0, r, r, 0.0),
                crate::shell::BarEdge::Right => (r, 0.0, 0.0, r),
            }
        };
        // the lockscreen paints its own fullscreen backdrop (blurred wallpaper
        // + dim) on the lock surface — an opaque base frame would cover it
        if mode != Mode::Lock {
            // Collapsed pill AND dashboard: semi-transparent dark bg (see-through).
            // Other panels (settings, launcher, etc.) keep the opaque bg so
            // content is readable.
            // bg is 0xRRGGBBAA — alpha comes from the per-state config.
            let pill_bg = if mode == Mode::Collapsed || mode == Mode::Expanded {
                (bg & 0xffffff00) | self.pill_alpha as u32
            } else {
                bg
            };
            ui::frame_concave(&mut v, out_w, out_h, r_tl, r_tr, r_br, r_bl, pill_bg);
            // hairline outer border — flat + minimal: a 1px tone line replaces
            // the drop shadow. Only in floating mode (all corners rounded);
            // a pinned surface sits flush to the screen edge anyway.
            if self.bar_floating && mode != Mode::Collapsed {
                ui::outline(&mut v, 0.0, 0.0, out_w, out_h, r, ui::hairline(&pal));
            }
        }
        match mode {
            Mode::Collapsed => self.layout_collapsed(&mut v, lw, lh, &pal),
            Mode::Expanded => self.layout_expanded(&mut v, lw, lh, &pal),
            Mode::ControlCenter => self.layout_cc(&mut v, lw, lh, &pal),
            Mode::Launcher => self.layout_launcher(&mut v, lw, lh, &pal),
            Mode::Notifications => self.layout_notifs(&mut v, lw, lh, &pal),
            Mode::Calendar => self.layout_calendar(&mut v, lw, lh, &pal),
            Mode::Settings => self.layout_settings(&mut v, lw, lh, &pal),
            Mode::Power => self.layout_power(&mut v, lw, lh, &pal),
            Mode::WifiMenu => self.layout_wifi_menu(&mut v, lw, lh, &pal),
            Mode::BtMenu => self.layout_bt_menu(&mut v, lw, lh, &pal),
            Mode::Audio => self.layout_audio(&mut v, lw, lh, &pal),
            Mode::Wallpaper => self.layout_wallpaper(&mut v, lw, lh, &pal),
            Mode::Themes => self.layout_themes(&mut v, lw, lh, &pal),
            Mode::Keybinds => self.layout_keybinds(&mut v, lw, lh, &pal),
            Mode::Weather => self.layout_weather(&mut v, lw, lh, &pal),
            Mode::Clipboard => self.layout_clipboard(&mut v, lw, lh, &pal),
            Mode::ClipboardImages => self.layout_clipboard_images(&mut v, lw, lh, &pal),
            Mode::Osd => self.layout_osd(&mut v, lw, lh, &pal),
            Mode::WorkspaceSwitcher => self.layout_workspace_switcher(&mut v, lw, lh, &pal),
            Mode::Lock => self.layout_lock(&mut v, lw, lh, &pal),
            Mode::PolkitAuth => self.layout_polkit_auth(&mut v, lw, lh, &pal),
        }
        // Apply the spring-overshoot centering: shift every content command by
        // (content_dx, content_dy) — but never the frame (index 0, which is
        // the surface itself and always covers the whole buffer). For Lock the
        // shift is always zero (the surface never exceeds the monitor target),
        // so the fullscreen backdrop stays put.
        if self.content_dx != 0.0 || self.content_dy != 0.0 {
            for (i, cmd) in v.iter_mut().enumerate() {
                if i == 0 {
                    continue;
                }
                match cmd {
                    Cmd::Rect { x, y, .. }
                    | Cmd::RectConcave { x, y, .. }
                    | Cmd::Outline { x, y, .. }
                    | Cmd::Text { x, y, .. }
                    | Cmd::Image { x, y, .. }
                    | Cmd::Scissor { x, y, .. }
                    | Cmd::Map { x, y, .. } => {
                        *x += self.content_dx;
                        *y += self.content_dy;
                    }
                    Cmd::Line { x0, y0, x1, y1, .. } => {
                        *x0 += self.content_dx;
                        *y0 += self.content_dy;
                        *x1 += self.content_dx;
                        *y1 += self.content_dy;
                    }
                    Cmd::ScissorEnd => {}
                }
            }
        }
        v
    }

    pub(crate) fn layout_collapsed(&mut self, v: &mut Vec<Cmd>, w: f32, h: f32, pal: &Pal) {
        let g = PILL_GAP;
        let cy = (h - 14.0) / 2.0;
        // whole pill is the expand zone (matched last — the per-item regions
        // below are pushed after it and win on overlap)
        self.region(0.0, 0.0, w, h, 1);

        let mx = self.pill_margin_x;

        // ── left-cluster: clock + date + battery — drawn from `mx` ──
        let mut lx = mx;
        if self.pill_clock && !self.clock.is_empty() {
            let key = 70;
            let hov = self.hover_key == key;
            ui::text(
                v,
                lx,
                cy,
                self.clock.clone(),
                14.0,
                if hov { pal.acc } else { pal.fg },
                false,
            );
            let cw = 8.0 + self.clock.chars().count() as f32 * 14.0 * 0.62;
            self.region(lx - 2.0, 4.0, cw + 4.0, h - 8.0, key);
            lx += cw + g;
        }
        {
            let dtext = self.date_str("%a %d");
            let hov = self.hover_key == 72;
            let dw = dtext.chars().count() as f32 * 14.0 * 0.62;
            ui::text(
                v,
                lx,
                cy,
                dtext,
                14.0,
                if hov { pal.acc } else { pal.fg },
                false,
            );
            self.region(lx - 2.0, 4.0, dw + 4.0, h - 8.0, 72);
            lx += dw + g;
        }
        if self.pill_battery && self.battery >= 0 {
            let key = 71;
            let hov = self.hover_key == key;
            let low = self.battery <= 20 && !self.ac_online;
            let ic = if low {
                RED
            } else if hov {
                pal.acc
            } else {
                pal.fg
            };
            let icon_w = ui::battery_icon_w(13.0);
            let pct_text = format!("{}%", self.battery);
            let pct_w = if self.pill_battery_pct {
                pct_text.len() as f32 * 14.0 * 0.62 + 3.0
            } else {
                0.0
            };
            // vertical nerd-font battery glyph (level/charging aware)
            let glyph = ui::battery_glyph(self.battery, self.ac_online);
            ui::text(v, lx, (h - 13.0) / 2.0, glyph, 13.0, ic, true);
            if self.pill_battery_pct {
                ui::text(v, lx + icon_w + 3.0, cy, pct_text, 14.0, ic, false);
            }
            let bw = icon_w + pct_w;
            self.region(lx - 2.0, 4.0, bw + 4.0, h - 8.0, key);
        }
        let left_end = self.pill_left_end(); // right edge of the last left item

        // ── right-cluster: bell + brightness + tray — anchored so the last
        // visible item's right edge sits at `w - mx` (right margin == left). ──
        let right_edge = w - mx;
        let tray_n = if self.pill_tray {
            self.tray.len().min(6)
        } else {
            0
        };
        let mut n_right: usize = 0;
        let mut right_used = 0.0;
        if self.pill_notif {
            right_used += 22.0;
            n_right += 1;
        }
        if self.pill_brightness {
            right_used += 22.0;
            n_right += 1;
        }
        if tray_n > 0 {
            right_used += tray_n as f32 * 22.0;
            n_right += 1;
        }
        let right_left = right_edge - right_used - (n_right.saturating_sub(1)) as f32 * g;
        // invisible control-center click zone: the strip from the left edge of
        // the right cluster out to the pill's right edge. It never inflates the
        // pill width or the margins, and the visible per-item regions below
        // (registered after it) win on overlap.
        if right_left < w {
            self.region(right_left, 0.0, (w - right_left).min(w), h, 63);
        }
        let mut rx = right_left;
        // bell + notification dot (rightmost)
        if self.pill_notif {
            let bx = rx;
            if self.hover_key == 62 {
                v.push(Cmd::Rect {
                    x: bx,
                    y: 8.0,
                    w: 22.0,
                    h: 22.0,
                    r: 11.0,
                    color: ui::hover(pal),
                });
            }
            ui::text_r(v, bx + 22.0, cy, ICON_BELL, 14.0, pal.fg, true);
            self.region(bx, 0.0, 22.0, h, 62);
            if self.notif_count > 0 {
                v.push(Cmd::Rect {
                    x: bx + 2.0,
                    y: 5.0,
                    w: 6.0,
                    h: 6.0,
                    r: 3.0,
                    color: pal.acc,
                });
            }
            rx += 22.0 + g;
        }
        // brightness / dark-mode toggle (sun ⇄ moon)
        if self.pill_brightness {
            let dx = rx;
            let hov = self.hover_key == 67;
            if hov {
                v.push(Cmd::Rect {
                    x: dx,
                    y: 8.0,
                    w: 22.0,
                    h: 22.0,
                    r: 11.0,
                    color: ui::hover_hl(pal),
                });
            }
            let glyph = if self.dark_mode {
                ICON_MOON
            } else {
                ICON_BRIGHTNESS
            };
            ui::text(
                v,
                dx + 5.0,
                cy,
                glyph,
                14.0,
                if hov { pal.acc } else { pal.fg },
                true,
            );
            self.region(dx, 0.0, 22.0, h, 67);
            rx += 22.0 + g;
        }
        // tray chips: right-anchored, left of the bell/brightness
        if tray_n > 0 {
            for i in 0..tray_n {
                let key = 65 + i as u32;
                let cx2 = rx + i as f32 * 22.0;
                let hov = self.hover_key == key;
                if hov {
                    v.push(Cmd::Rect {
                        x: cx2,
                        y: 9.0,
                        w: 20.0,
                        h: 20.0,
                        r: 10.0,
                        color: ui::hover_hl(pal),
                    });
                }
                let item = &self.tray[i];
                let ik = item.image_key();
                if ik.is_empty() {
                    ui::text(v, cx2 + 6.5, 13.5, ICON_DOT, 8.0, ui::fg2(pal), true);
                } else {
                    v.push(Cmd::Image {
                        x: cx2 + 2.0,
                        y: 11.0,
                        w: 16.0,
                        h: 16.0,
                        key: ik,
                    });
                }
                self.region(cx2, 8.0, 20.0, 22.0, key);
            }
        }

        // ── center-cluster: remaining pill_order items, auto-justified in the
        // space between the left cluster and the right cluster ──
        let order = self.pill_order.clone();
        let widths: Vec<f32> = order.iter().map(|&it| self.pill_item_width(it)).collect();
        let center: Vec<(usize, PillItem, f32)> = order
            .iter()
            .enumerate()
            .filter(|(idx, &it)| {
                it != PillItem::Clock && it != PillItem::Battery && widths[*idx] > 0.0
            })
            .map(|(idx, &it)| (idx, it, widths[idx]))
            .collect();
        let center_total: f32 = center.iter().map(|&(_, _, iw)| iw).sum::<f32>()
            + (center.len().saturating_sub(1)) as f32 * g;
        // `right_left` is the left edge of the right cluster; the center
        // cluster is centered in the leftover space so the outer margins stay
        // symmetric.
        let right_start = if n_right > 0 { right_left } else { w - mx };
        let center_space = (right_start - left_end - g).max(0.0);
        let mut cx = left_end + g + ((center_space - center_total) / 2.0).max(0.0);
        for &(slot, item, iw) in &center {
            match item {
                PillItem::Workspaces | PillItem::WorkspacesShort => {
                    // only the ACTIVE workspace number — always current, works
                    // for any workspace id (8/9/10 …), never a fixed 1..3 row
                    let txt = (self.ws_active + 1).to_string();
                    let key = 50 + slot as u32 * 10;
                    let hov = self.hover_key == key;
                    ui::text(
                        v,
                        cx + 2.0,
                        cy,
                        &txt,
                        14.0,
                        if hov { pal.acc } else { pal.fg },
                        false,
                    );
                    self.region(
                        cx - 4.0,
                        8.0,
                        if txt.len() > 1 { 26.0 } else { 20.0 },
                        h - 12.0,
                        key,
                    );
                }
                PillItem::WorkspacesLong => {
                    // long form: chips 1..5 always shown + clickable (Hyprland
                    // auto-creates a workspace on switch); the active one is
                    // marked with a dot below (no background), and when the
                    // active workspace is >5 (8/9/10 …) its number is appended
                    // on the right with the same dot below it
                    for i in 0..5usize {
                        let n = i + 1;
                        let cx2 = cx + i as f32 * (18.0 + 2.0);
                        let key = 50 + slot as u32 * 10 + i as u32;
                        let hov = self.hover_key == key;
                        let is_active = n == self.ws_active + 1;
                        if hov {
                            v.push(Cmd::Rect {
                                x: cx2,
                                y: 7.0,
                                w: 18.0,
                                h: 24.0,
                                r: 6.0,
                                color: ui::hover_hl(pal),
                            });
                        }
                        ui::text(v, cx2 + 6.0, cy, n.to_string(), 14.0, pal.fg, false);
                        if is_active {
                            v.push(Cmd::Rect {
                                x: cx2 + 7.5,
                                y: 32.0,
                                w: 3.0,
                                h: 3.0,
                                r: 1.5,
                                color: pal.acc,
                            });
                        }
                        self.region(cx2, 7.0, 18.0, 24.0, key);
                    }
                    if self.ws_active + 1 > 5 {
                        let cx2 = cx + 5.0 * (18.0 + 2.0) + 2.0;
                        let txt = (self.ws_active + 1).to_string();
                        let key = 50 + slot as u32 * 10 + 5;
                        let hov = self.hover_key == key;
                        if hov {
                            v.push(Cmd::Rect {
                                x: cx2 - 2.0,
                                y: 7.0,
                                w: if txt.len() > 1 { 22.0 } else { 16.0 },
                                h: 24.0,
                                r: 6.0,
                                color: ui::hover_hl(pal),
                            });
                        }
                        ui::text(v, cx2 + 2.0, cy, &txt, 14.0, pal.fg, false);
                        let tw = txt.len() as f32 * 14.0 * 0.62;
                        v.push(Cmd::Rect {
                            x: cx2 + 2.0 + tw * 0.5 - 1.5,
                            y: cy + 14.0 + 2.0,
                            w: 3.0,
                            h: 3.0,
                            r: 1.5,
                            color: pal.acc,
                        });
                        self.region(
                            cx2 - 4.0,
                            8.0,
                            if txt.len() > 1 { 26.0 } else { 20.0 },
                            h - 12.0,
                            key,
                        );
                    }
                }
                PillItem::Wifi => {
                    let key = 50 + slot as u32 * 10;
                    let hov = self.hover_key == key;
                    let c = if hov {
                        pal.acc
                    } else if !self.ssid.is_empty() {
                        pal.acc
                    } else {
                        pal.fg
                    };
                    ui::text(v, cx + 4.0, cy, ICON_WIFI, 14.0, c, true);
                    self.region(cx - 2.0, 4.0, iw + 4.0, h - 8.0, key);
                }
                PillItem::Bluetooth => {
                    let key = 50 + slot as u32 * 10;
                    let hov = self.hover_key == key;
                    let c = if hov {
                        pal.acc
                    } else if !self.bt_devices.is_empty() {
                        pal.acc
                    } else {
                        pal.fg
                    };
                    ui::text(v, cx + 4.0, cy, ICON_BLUETOOTH, 14.0, c, true);
                    self.region(cx - 2.0, 4.0, iw + 4.0, h - 8.0, key);
                }
                PillItem::Volume => {
                    let key = 50 + slot as u32 * 10;
                    let hov = self.hover_key == key;
                    ui::text(
                        v,
                        cx + 4.0,
                        cy,
                        ICON_VOLUME,
                        14.0,
                        if hov { pal.acc } else { pal.fg },
                        true,
                    );
                    self.region(cx - 2.0, 4.0, iw + 4.0, h - 8.0, key);
                }
                PillItem::Wallpaper => {
                    let key = 50 + slot as u32 * 10;
                    let hov = self.hover_key == key;
                    ui::text(
                        v,
                        cx + 4.0,
                        cy,
                        ICON_WALLPAPER,
                        14.0,
                        if hov { pal.acc } else { pal.fg },
                        true,
                    );
                    self.region(cx - 2.0, 4.0, iw + 4.0, h - 8.0, key);
                }
                PillItem::Themes => {
                    let key = 50 + slot as u32 * 10;
                    let hov = self.hover_key == key;
                    let c = if hov { pal.acc } else { pal.fg };
                    ui::text(v, cx + 4.0, cy, ICON_PALETTE, 14.0, c, true);
                    self.region(cx - 2.0, 4.0, iw + 4.0, h - 8.0, key);
                }
                PillItem::Settings => {
                    // the gear chip: a 22 px slot like the other icon chips, so
                    // the order can drag it anywhere
                    let key = 50 + slot as u32 * 10;
                    let hov = self.hover_key == key;
                    if hov {
                        v.push(Cmd::Rect {
                            x: cx,
                            y: 8.0,
                            w: 22.0,
                            h: 22.0,
                            r: 11.0,
                            color: ui::hover_hl(pal),
                        });
                    }
                    ui::text(
                        v,
                        cx + 4.0,
                        cy,
                        ICON_SETTINGS,
                        14.0,
                        if hov { pal.acc } else { pal.fg },
                        true,
                    );
                    self.region(cx - 2.0, 4.0, iw + 4.0, h - 8.0, key);
                }

                PillItem::Branding => {
                    // display-only brand mark — no hit region, no hover; the
                    // glyph renders in the brand family, centered in its chip.
                    // Re-read here: the branding card is a zero-`Ink` scene
                    // now, so its Rust drawer (which used to refresh this
                    // field every draw) only runs when the scene is missing.
                    self.brand_glyph = crate::vars::read_branding_glyph();
                    ui::text_cw(
                        v,
                        cx + iw / 2.0,
                        cy,
                        &self.brand_glyph,
                        16.0,
                        crate::text::Ff::Brand,
                        crate::text::Fw::Regular,
                        pal.fg,
                        false,
                    );
                }
                PillItem::Watts => {
                    let key = 50 + slot as u32 * 10;
                    let hov = self.hover_key == key;
                    let c = if hov { pal.acc } else { pal.fg };
                    ui::text(v, cx, cy, ICON_BOLT, 14.0, c, true);
                    ui::text(
                        v,
                        cx + 14.0,
                        cy,
                        format!("{:.1}W", self.watts_shown),
                        14.0,
                        c,
                        false,
                    );
                    self.region(cx - 2.0, 4.0, iw + 4.0, h - 8.0, key);
                }
                PillItem::Visualizer => {
                    // bars grow from a fixed bottom line: at silence they're
                    // gone (0 height), never a stub — cava does the same
                    let n_bars = 8u32;
                    let max_h = 14.0;
                    let bar_bottom = h / 2.0 + max_h / 2.0;
                    for i in 0..n_bars {
                        let bx = cx + i as f32 * 3.0;
                        let b = self.viz.get(i as usize).copied().unwrap_or(0.0);
                        let bh = b * max_h;
                        if bh < 0.5 {
                            continue;
                        }
                        let col = if i % 4 == 0 {
                            pal.acc
                        } else {
                            mix(pal.acc, pal.fg, 0.35)
                        };
                        v.push(Cmd::Rect {
                            x: bx,
                            y: bar_bottom - bh,
                            w: 2.0,
                            h: bh,
                            r: 1.0,
                            color: col,
                        });
                    }
                    let key = 50 + slot as u32 * 10;
                    self.region(cx - 2.0, 4.0, n_bars as f32 * 3.0 + 4.0, h - 8.0, key);
                }
                PillItem::User => {
                    let key = 50 + slot as u32 * 10;
                    let hov = self.hover_key == key;
                    ui::text(
                        v,
                        cx,
                        cy,
                        &self.username,
                        14.0,
                        if hov { pal.acc } else { pal.fg },
                        false,
                    );
                    self.region(cx - 2.0, 4.0, iw + 4.0, h - 8.0, key);
                }
                _ => {}
            }
            cx += iw + g;
        }

        // drag ghost: the item being dragged follows the pointer
        if let Some(ref drag) = self.pill_drag {
            let ghost_x = drag.x - 16.0;
            v.push(Cmd::Rect {
                x: ghost_x,
                y: 2.0,
                w: 32.0,
                h: h - 4.0,
                r: (h - 4.0) / 2.0,
                color: mix(ui::hover(pal), pal.acc, 0.20),
            });
        }
    }

    /// Draw a card from its declarative scene file (if present) and register
    /// all hit regions + dispatch any `Ink` painters. Returns `true` when a
    /// scene was loaded and drew, so the caller can skip the built-in Rust
    /// draw for that card.
    pub(crate) fn draw_card_scene(
        &mut self,
        card_id: &str,
        v: &mut Vec<Cmd>,
        x: f32,
        y: f32,
        card_w: f32,
        card_h: f32,
        pal: &Pal,
        vals: &crate::scene::SceneValues,
    ) -> bool {
        let scene = self.scene_cache.get_validated(card_id, vals).cloned();
        if self.scene_cache.take_reloaded() {
            self.scene_anims.clear();
        }
        let Some(scene) = scene else {
            return false;
        };
        // A fully declarative scene (no `Ink` body) skips the Rust drawer, so
        // the card's wheel-scroll rect (normally set inside the drawer) must
        // be tracked here to keep pointer-wheel scrolling alive.
        if !scene
            .items
            .iter()
            .any(|it| matches!(it, crate::scene::SceneItem::Ink { .. }))
        {
            match card_id {
                "bluetooth" => self.bt_rect = (x, y, card_w, card_h),
                "ticker" => self.ticker_rect = (x, y, card_w, card_h),
                "currency" => self.currency_rect = (x, y, card_w, card_h),
                "recent" => {
                    self.recent_rect = (x, y, card_w, card_h);
                    // the Rust drawer normally refreshes the 30 s cache; keep
                    // it live now that the scene owns the whole card
                    self.refresh_recent_if_stale();
                }
                "news" => self.news_rect = (x, y, card_w, card_h),
                "mirror" => {
                    // the scene owns the whole card; the Rust drawer that
                    // normally sets these rects never runs
                    self.mirror_rect = (x, y, card_w, card_h);
                }
                "appshortcut" => self.app_shortcut_rect = (x, y, card_w, card_h),
                "lyrics" => {
                    // the Rust drawer normally auto-follows the sung line each
                    // frame; keep it live now that the scene owns the card
                    self.lyrics_rect = (x, y, card_w, card_h);
                    let vis = self.lyrics_card_visible();
                    self.lyrics_follow(vis);
                }
                "clipboard" => {
                    // the Rust drawer normally registers the wheel-scroll
                    // rect; keep clip scrolling alive now the scene owns it
                    self.clip_card_rect = (x, y, card_w, card_h);
                }
                "sliders" => {
                    // the scene owns the whole card; the Rust drawer that
                    // normally sets this rect (swipe hit-test + dot hover)
                    // never runs
                    self.sliders_rect = (x, y, card_w, card_h);
                }
                "batteryh" => {
                    // the swipe hit-test + dots hover read this rect
                    self.battery_h_rect = (x, y, card_w, card_h);
                }
                "batteryv" => {
                    self.battery_v_rect = (x, y, card_w, card_h);
                }
                "water" => {
                    // the midnight rollover + the swipe hit-test rect both
                    // lived in the drawer the scene now replaces
                    self.roll_water_day();
                    self.water_rect = (x, y, card_w, card_h);
                }
                "eyerest" => {
                    // the drawer's inline `er_tick()` — a (re)draw always shows
                    // the truth even if a 1 s tick was skipped
                    let _ = self.er_tick();
                }
                "branding" => {
                    // the brand mark is aspect-fit against this box, and the
                    // Rust drawer that measured it never runs now
                    self.branding_rect = (x, y, card_w, card_h);
                }
                "accent" => {
                    // the source list is centered in this box with a clamp, so
                    // the declarative rows need the height (the drawer that
                    // measured it stands down)
                    self.accent_rect = (x, y, card_w, card_h);
                }
                "clipimg" => {
                    // the scroll clamp and the visible-row count both read
                    // this box (`clipimg_card_visible`), and the Rust drawer
                    // that used to set it no longer runs
                    self.clipimg_card_rect = (x, y, card_w, card_h);
                }
                "media" => {
                    // the card box decides `wide` (the info column sits right
                    // of the art or under it) and the title/line2 truncation —
                    // both computed in `scene_values` now that the drawer stands
                    // down, and the album-art + seek-hit geometry need it too
                    self.media_card_rect = (x, y, card_w, card_h);
                }
                "system" => {
                    // the hostname truncation (`w / 8` chars) and the date
                    // chip's measured width both read the card box now that
                    // the drawer stands down
                    self.sys_card_rect = (x, y, card_w, card_h);
                }
                "moon" => {
                    // the disc's diameter, its center and all three side-caption
                    // baselines are the card's own body arithmetic, and the
                    // side column's `> s(60)` gate needs the card box — the
                    // drawer that used to set it stands down
                    self.moon_rect = (x, y, card_w, card_h);
                }
                "worldclock" => {
                    // the card box (the add row's slot arithmetic) and the
                    // frame's ONE clock — the city rows' HH:MM and the
                    // day/night glyph are the drawer's own arithmetic off this
                    // instant, and a value sampled twice could straddle a minute
                    self.worldclock_rect = (x, y, card_w, card_h);
                    self.worldclock_epoch = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs() as i64)
                        .unwrap_or(0);
                }
                "wallpaper" => {
                    // the grid's cell fit, its scroll clamp and its centered
                    // block all read the card box, and the tile keys the click
                    // handler resolves are drawn against the SAME clamped
                    // scroll — so the state is settled here, once a frame,
                    // before anything reads it (the drawer stands down)
                    self.wp_card_rect = (x, y, card_w, card_h);
                    self.wp_card_settle();
                }
                "weather" => {
                    // the centered glyph+temp pair, the forecast's row count and
                    // its centered top, and the hover gate for the pagination
                    // dots all read the card box now that the drawer stands down
                    self.weather_rect = (x, y, card_w, card_h);
                }
                "wifi" => {
                    // the row window + the wheel-scroll rect both lived in the
                    // drawer the scene now replaces
                    self.wifi_rect = (x, y, card_w, card_h);
                }
                "todo" => {
                    // the wheel-scroll rect + the height→visible-row count
                    // (`todo_card_visible`, the wheel clamp) both lived in the
                    // drawer the scene now replaces
                    self.todo_card_rect = (x, y, card_w, card_h);
                }
                _ => {}
            }
        }
        // the scene owns the title chrome when it declares a `Header`; Rust
        // card drawers then skip their own (`if !self.scene_owns_header`).
        let had_header = self.scene_owns_header;
        let had_composer = self.scene_owns_composer;
        let had_rows = self.scene_owns_rows;
        self.scene_owns_header = scene
            .items
            .iter()
            .any(|it| matches!(it, crate::scene::SceneItem::Header { .. }));
        self.scene_owns_composer = scene
            .items
            .iter()
            .any(|it| matches!(it, crate::scene::SceneItem::Composer { .. }));
        self.scene_owns_rows = scene.items.iter().any(|it| {
            matches!(
                it,
                crate::scene::SceneItem::Rows { .. } | crate::scene::SceneItem::TextWrap { .. }
            )
        });
        let (hits, inks) = scene.draw(
            v,
            x,
            y,
            card_w,
            card_h,
            self.scale,
            pal,
            vals,
            self.hover_key,
            self.card_show_title,
            self.card_show_glyph,
        );
        for hit in hits {
            self.region(hit.x, hit.y, hit.w, hit.h, hit.key);
            if let Some(action) = hit.action {
                self.scene_click_actions
                    .insert(hit.key, crate::scene::ClickedAction::new(card_id, action));
            }
            // Dashboard fader drag math (`set_dash_slider`) reads the geometry
            // the Rust sliders drawer publishes; a declared `Fader` publishes
            // the same rect from its own hit box so dragging a declarative
            // fader keeps working once the drawer stands down.
            match hit.key {
                51..=55 => {
                    self.faders[(hit.key - 51) as usize] = (hit.x, hit.y + hit.h / 2.0, hit.w)
                }
                59 => self.balance_rect = (hit.x, hit.y + hit.h / 2.0, hit.w),
                // the media card's seek bar: the drag handler reads this rect
                // (x, y, w, thickness), and the drawer's own halo is 2 px left
                // / 6 px above the bar and 4 px wider — so the bar's own box
                // is recovered from the hit, exactly as the sliders do
                23 => self.media_seek_rect = (hit.x + 2.0, hit.y + 6.0, hit.w - 4.0, 5.0),
                _ => {}
            }
        }
        self.dispatch_scene_inks(v, inks, x, y, card_w, card_h, pal, vals);
        self.scene_owns_header = had_header;
        self.scene_owns_composer = had_composer;
        self.scene_owns_rows = had_rows;
        true
    }

    /// Draw a whole-shell surface (`shell.ron`) and register all hit regions
    /// + dispatch any `Ink` painters. Returns `true` when the declared
    /// surface existed and drew, so the caller can skip the built-in Rust
    /// draw for that surface area.
    pub(crate) fn draw_shell_surface(
        &mut self,
        name: &str,
        v: &mut Vec<Cmd>,
        w: f32,
        h: f32,
        pal: &Pal,
        vals: &crate::scene::SceneValues,
        dash: Option<&DashDrawCtx>,
    ) -> bool {
        // clone the surface to release the `&mut self` borrow on
        // `shell_scene` before drawing (avoids a borrowck conflict with the
        // dispatch callback that follows).
        let scene = self
            .shell_scene
            .get_validated(vals)
            .and_then(|ss| ss.surface(name))
            .cloned();
        if self.shell_scene.take_reloaded() {
            // The scene text changed, so a binding name may now point somewhere
            // else; easing from the OLD target would drag the new one along.
            self.scene_anims.clear();
        }
        let Some(surface) = scene else {
            return false;
        };
        // The DRAWN pool is this surface's own, not the caller's. The caller
        // already built it with `scene_values_for(name)`, but taking the
        // surface's view here as well means the draw cannot accidentally run
        // against a pool that already has a sibling's stores merged in — the
        // one path where a cross-surface `nav.tab` would resolve.
        let mut own = vals.clone();
        self.scene_stores.merge_into(name, &mut own);
        let vals = &own;
        let card = crate::scene::CardScene {
            items: surface.items,
        };
        // Stamped ONCE per surface, so every animated field in this frame
        // reads the same instant and two fields easing on one item cannot
        // differ by the microseconds between their reads.
        self.scene_anims.set_now(std::time::Instant::now());
        let (hits, inks) = card.draw_with_anims(
            v,
            0.0,
            0.0,
            w,
            h,
            self.scale,
            pal,
            vals,
            self.hover_key,
            false,
            false,
            &mut self.scene_anims,
        );
        for hit in hits {
            self.region(hit.x, hit.y, hit.w, hit.h, hit.key);
            if let Some(action) = hit.action {
                self.scene_click_actions
                    .insert(hit.key, crate::scene::ClickedAction::new(name, action));
            }
        }
        self.dispatch_shell_inks(v, inks, w, h, pal, vals, dash);
        true
    }

    /// Dispatch `Ink` painters for whole-shell surfaces. Surface `Ink` names
    /// map to the Rust layout function for that surface section (using the
    /// same base-pixel coordinate system the scene was authored against).
    fn dispatch_shell_inks(
        &mut self,
        v: &mut Vec<Cmd>,
        inks: Vec<crate::scene::SceneInk>,
        w: f32,
        h: f32,
        pal: &Pal,
        vals: &crate::scene::SceneValues,
        dash: Option<&DashDrawCtx>,
    ) {
        for ink in inks {
            // declared items authored before this Ink draw at its dispatch
            // slot (the dashboard band layers over the grid this way)
            if !ink.pre.is_empty() {
                let card = crate::scene::CardScene {
                    items: ink.pre.clone(),
                };
                let ctxvals = match dash {
                    Some(d) => d.vals,
                    None => vals,
                };
                card.draw(
                    v,
                    0.0,
                    0.0,
                    w,
                    h,
                    self.scale,
                    pal,
                    ctxvals,
                    self.hover_key,
                    false,
                    false,
                );
            }
            let cx = SETTINGS_SIDEBAR;
            let drawn = match ink.name.as_str() {
                "settings_network" => {
                    self.layout_settings_network(v, w, cx, pal);
                    true
                }
                "settings_sound" => {
                    self.layout_settings_sound(v, w, cx, pal);
                    true
                }
                "settings_appearance" => {
                    self.layout_settings_appearance(v, w, cx, pal);
                    true
                }
                "settings_pill" => {
                    self.layout_settings_pill(v, w, cx, pal);
                    true
                }
                "settings_system" => {
                    self.layout_settings_system(v, w, cx, h, pal);
                    true
                }
                "settings_misc" => {
                    self.layout_settings_misc(v, w, cx, pal);
                    true
                }
                // the notif surface's body: notification cards + empty state
                // (title + Clear/DND chips are declared chrome in shell.ron)
                "notif_list" => {
                    self.layout_notifs_body(v, w, h, pal);
                    true
                }
                // the lock surface's body: password field + auth error/caps +
                // hold-power buttons + hint (hero clock/date/avatar are
                // declared chrome in shell.ron)
                "lock_screen" => {
                    self.layout_lock_body(v, w, h, pal);
                    true
                }
                // the wallpaper surface's body: empty state + thumbnail grid +
                // scrollbar (header chrome is declared in shell.ron)
                "wallpaper_picker" => {
                    self.layout_wallpaper_body(v, w, h, pal);
                    true
                }
                // ── the `dashboard` surface (LIVE in the viewing state) ──
                // both bodies route to their exact Rust painters; declared
                // chrome can layer between them. The grid Ink carries the fluid
                // drag + region side effects; the banner Ink the strip chips
                // (+ edit tray/guides when `layout_banner` runs it). Edit-mode
                // chrome stays Rust — the surface is never consulted then.
                "dash_grid" => {
                    if let Some(d) = dash {
                        self.layout_dash_grid(v, w, h, d.lt, d.cursor, d.vals, pal);
                        true
                    } else {
                        false
                    }
                }
                "dash_banner" => {
                    if let Some(d) = dash {
                        self.layout_banner(v, w, d.lt, d.cursor, pal);
                        true
                    } else {
                        false
                    }
                }
                _ => false,
            };
            if !drawn {
                eprintln!("zen: shell ink \"{}\" has no Rust painter", ink.name);
            }
        }
    }

    /// Dispatch `Ink` items to their Rust painters. Unknown names are a
    /// no-op (logged) so a scene can carry optional decoration safely.
    fn dispatch_scene_inks(
        &mut self,
        v: &mut Vec<Cmd>,
        inks: Vec<crate::scene::SceneInk>,
        cx: f32,
        cy: f32,
        cw: f32,
        ch: f32,
        pal: &Pal,
        vals: &crate::scene::SceneValues,
    ) {
        for ink in inks {
            // declared items authored before this Ink draw at its dispatch
            // slot — declared chrome BETWEEN two Rust bodies in z-order
            if !ink.pre.is_empty() {
                let card = crate::scene::CardScene {
                    items: ink.pre.clone(),
                };
                card.draw(
                    v,
                    cx,
                    cy,
                    cw,
                    ch,
                    self.scale,
                    pal,
                    vals,
                    self.hover_key,
                    self.card_show_title,
                    self.card_show_glyph,
                );
            }
            let drawn = match crate::shell::DashCard::from_id(&ink.name) {
                Some(card) => {
                    card.draw_rust(self, v, cx, cy, cw, ch, pal);
                    true
                }
                None => false,
            };
            if !drawn {
                eprintln!("zen: scene ink \"{}\" has no Rust painter", ink.name);
            }
        }
    }

    /// The live values a scene can bind by name. Shared by every card scene —
    /// a scene only references the ones it needs. Value names mirror the
    /// shell's public fields so `.ron` authors get the same naming as the
    /// Rust code. Strings interpolate via `{name}`; the typed variants feed
    /// the data-driven primitives (`Rows`/`Spark`/`Ring`/`Fader`/`Toggle`).
    /// Publish everything the declarative `pill` surface draws: the three
    /// clusters' strings, their inks, and the per-widget center keys.
    ///
    /// The keys are the Rust drawer's own scheme, not a new one — slot `i` of
    /// `pill_order` owns `50 + i·10`, plus a widget's own `key_add` for its
    /// sub-chips — so a center `Repeat` template binds `pill_key_<tag>` and
    /// ONE template serves every position in the user's ordering.
    ///
    /// Every number here is BASE px (the engine scales the scene), so the
    /// ladders the bar computes by hand stay in exactly one place: this
    /// function. The Rust drawer is the fallback for a `pill` surface that
    /// fails to load, and a parity test holds the two to the same arithmetic.
    pub(crate) fn publish_pill(&self, m: &mut crate::scene::SceneValues) {
        use crate::scene::{SceneColor, SceneValue};
        use crate::ui::DANGER;
        let h = self.cfg.bar_h();
        let pal = Pal {
            fg: self.sv_fg,
            bg: self.sv_bg,
            acc: self.sv_acc,
            sfg: self.sv_sfg,
        };
        // The drawer's own ink ladder: hover (or "connected") lights the
        // accent, everything else rides the primary ink. Published ALREADY
        // RESOLVED because it depends on `hover_key` — a token has no way to
        // see which key is live.
        let ink = |k: u32| -> SceneColor {
            SceneColor::Raw(if self.hover_key == k { pal.acc } else { pal.fg })
        };
        let ink_lit = |connected: bool, k: u32| -> SceneColor {
            SceneColor::Raw(if self.hover_key == k || connected {
                pal.acc
            } else {
                pal.fg
            })
        };
        // ── the bar's own anchors ──
        m.insert("pill_h", SceneValue::Ring(h));
        m.insert("pill_mid", SceneValue::Ring((h - 14.0) / 2.0)); // 14 px text baseline
        m.insert("pill_mid_sm", SceneValue::Ring((h - 13.0) / 2.0)); // 13 px glyphs
        m.insert("pill_gap", SceneValue::Ring(PILL_GAP));
        m.insert("pill_mx", SceneValue::Ring(self.pill_margin_x));
        // The left cluster's slot is NOT its ink: the `pill_mx` lead-in is part
        // of the drawer's `left_end`, and the center cluster is centered in
        // whatever the left slot leaves. Measuring the row to its ink alone
        // (152.58 vs the drawer's 170.58) hands the centering 24 px too much
        // room, so every center chip lands 12 px right of the drawer. Publish
        // the drawer's own `left_end` PLUS the gap that separates the two
        // clusters, and the center strip's Fill starts exactly where the
        // drawer starts measuring it.
        m.insert(
            "pill_left_w",
            SceneValue::Ring(self.pill_left_end() + PILL_GAP),
        );
        // the visualizer's plot box: bars rise from a line at `h/2 + 7`
        m.insert("pill_viz_y", SceneValue::Ring(h / 2.0 - 7.0));
        m.insert(
            "pill_viz",
            SceneValue::Spark(self.viz.iter().copied().take(8).collect()),
        );

        // ── left cluster: clock · date · battery ──
        m.insert("pill_clock", SceneValue::Text(self.clock.clone()));
        m.insert(
            "pill_clock_on",
            SceneValue::Toggle(self.pill_clock && !self.clock.is_empty()),
        );
        m.insert("pill_clock_key", SceneValue::Ring(70.0));
        // the chip's own width (`8 + chars · 14 · 0.62` — the drawer's text
        // metric, bearings included) so the cluster's auto row measures it
        // exactly as `collapsed_w` does
        m.insert(
            "pill_clock_w",
            SceneValue::Ring(self.pill_item_width(PillItem::Clock)),
        );
        m.insert_color("pill_clock_ink", ink(70));
        m.insert("pill_date", SceneValue::Text(self.date_str("%a %d")));
        m.insert(
            "pill_date_w",
            SceneValue::Ring(self.date_str("%a %d").chars().count() as f32 * 14.0 * 0.62),
        );
        m.insert("pill_date_key", SceneValue::Ring(72.0));
        m.insert_color("pill_date_ink", ink(72));
        let batt_low = self.battery <= 20 && !self.ac_online;
        m.insert(
            "pill_batt_on",
            SceneValue::Toggle(self.pill_battery && self.battery >= 0),
        );
        m.insert(
            "pill_batt_glyph",
            SceneValue::Text(crate::ui::battery_glyph(self.battery, self.ac_online).to_string()),
        );
        m.insert(
            "pill_batt_pct",
            SceneValue::Text(format!("{}%", self.battery)),
        );
        m.insert(
            "pill_batt_pct_on",
            SceneValue::Toggle(self.pill_battery_pct),
        );
        m.insert(
            "pill_batt_w",
            SceneValue::Ring(self.pill_item_width(PillItem::Battery)),
        );
        // the percentage rides the glyph's own measured width (`icon_w + 3`),
        // published so the label can never drift from `battery_icon_w`
        m.insert(
            "pill_batt_pct_x",
            SceneValue::Ring(crate::ui::battery_icon_w(13.0) + 3.0),
        );
        m.insert("pill_batt_key", SceneValue::Ring(71.0));
        m.insert_color(
            "pill_batt_ink",
            if batt_low {
                SceneColor::Raw(DANGER)
            } else {
                ink(71)
            },
        );

        // ── center cluster: the widgets the user ordered ──
        // The model is the ordered tags of every ENABLED center widget; the
        // key each one owns is its slot in the FULL `pill_order` (clock and
        // battery hold slots too and are skipped as model entries), which is
        // what the Rust click handler resolves `50 + i·10` against.
        let order = &self.pill_order;
        let widths: Vec<f32> = order.iter().map(|&it| self.pill_item_width(it)).collect();
        let center: Vec<(usize, PillItem)> = order
            .iter()
            .enumerate()
            .filter(|(i, &it)| it != PillItem::Clock && it != PillItem::Battery && widths[*i] > 0.0)
            .map(|(i, &it)| (i, it))
            .collect();
        // Quickshell-style model: one row per enabled center widget, each
        // carrying the fields its DELEGATE reads as `item.*`:
        //   `tag`  the widget's own stable tag — identity for the row, and
        //         what a future per-widget inspector or test names it by;
        //   `tpl`  which template renders it. Two widgets whose bodies are
        //         identical (the two workspace-number chips, or every plain
        //         icon chip) carry the SAME `tpl`, which is what collapses
        //         fourteen templates into a handful;
        //   `key`  the click key it owns in the user's ordering;
        //   `ink`  its own color (hover-lit, or accent while CONNECTED, which
        //         is state only the shell knows);
        //   `w`    its own width, so a delegate never hardcodes the 22 px the
        //         drawer measured;
        //   `glyph` its text, for the delegates that draw one.
        // This replaces the whole `pill_key_<tag>` / `pill_ink_<tag>` family,
        // which existed only because a `List` of tags cannot carry per-row
        // data.
        let mut pill_ctr: Vec<crate::scene::ModelRow> = Vec::new();
        for &(slot, item) in &center {
            let key = 50 + slot as u32 * 10;
            let c = match item {
                PillItem::Wifi => ink_lit(!self.ssid.is_empty(), key),
                PillItem::Bluetooth => ink_lit(!self.bt_devices.is_empty(), key),
                _ => ink(key),
            };
            pill_ctr.push(vec![
                ("tag".into(), SceneValue::Text(item.tag().into())),
                ("tpl".into(), SceneValue::Text(item.delegate().into())),
                ("key".into(), SceneValue::Ring(key as f32)),
                ("ink".into(), SceneValue::Color(c)),
                ("slot".into(), SceneValue::Ring(slot as f32)),
                ("w".into(), SceneValue::Ring(widths[slot])),
                ("glyph".into(), SceneValue::Text(item.glyph().into())),
            ]);
        }
        m.insert("pill_ctr", SceneValue::Model(pill_ctr));
        // `pill_key_<tag>` / `pill_ink_<tag>` are NOT published: the model row
        // carries both, so a delegate binds `item.key` / `value("item.ink")`
        // and one template covers every chip. The `declare_conditional` loop
        // that used to stand here is gone with them — a row that exists is a
        // widget that is on, so there is no longer a name that is published
        // only sometimes.
        // workspace chips: the number, its box, and the region that box buys
        let ws_n = (self.ws_active + 1).to_string();
        m.insert("pill_ws_n", SceneValue::Text(ws_n.clone()));
        m.insert("pill_ws_w", SceneValue::Ring(self.active_ws_w()));
        m.insert(
            "pill_ws_reg_w",
            SceneValue::Ring(if ws_n.len() > 1 { 26.0 } else { 20.0 }),
        );
        // long form: five fixed 18 px chips, each marked by a dot when active
        for i in 0..5usize {
            m.insert(
                format!("pill_ws_long_on_{}", i + 1),
                SceneValue::Toggle(ws_n == (i + 1).to_string()),
            );
        }
        let tail = self.ws_active + 1 > 5;
        m.insert(
            "pill_ws_long_w",
            SceneValue::Ring(self.pill_item_width(PillItem::WorkspacesLong)),
        );
        m.insert("pill_ws_tail_on", SceneValue::Toggle(tail));
        m.insert("pill_ws_tail_n", SceneValue::Text(ws_n.clone()));
        m.insert("pill_ws_tail_w", SceneValue::Ring(self.active_ws_w()));
        m.insert(
            "pill_ws_tail_reg_w",
            SceneValue::Ring(if ws_n.len() > 1 { 26.0 } else { 20.0 }),
        );
        m.insert(
            "pill_ws_tail_hover_w",
            SceneValue::Ring(if ws_n.len() > 1 { 22.0 } else { 16.0 }),
        );
        // the tail chip's dot, from the tail chip's own left edge: the drawer
        // draws it at `cx + 102 + 2 + w/2 − 1.5` and the chip's box starts at
        // `cx + 100`
        m.insert(
            "pill_ws_tail_dot_x",
            SceneValue::Ring(2.5 + ws_n.chars().count() as f32 * 14.0 * 0.62 * 0.5),
        );
        // …and the dot's baseline row: the drawer puts the tail chip's dot at
        // `cy + 14 + 2` (the appended number sits lower in the strip than the
        // per-chip active dots, which are a flat `y: 32`)
        m.insert(
            "pill_ws_tail_dot_y",
            SceneValue::Ring((h - 14.0) / 2.0 + 16.0),
        );
        // branding re-reads its glyph here: the branding CARD is a zero-`Ink`
        // scene, so its Rust drawer (which used to refresh this field every
        // draw) only runs when that scene is missing
        let brand = crate::vars::read_branding_glyph();
        m.insert("pill_brand_glyph", SceneValue::Text(brand.clone()));
        m.insert(
            "pill_brand_w",
            SceneValue::Ring({
                let chars = brand.chars().count().max(1) as f32;
                (chars * 16.0 * 0.62 + self.scale.s(4.0)).max(18.0)
            }),
        );
        // watts + user: plain measured strings, so their chips' widths are the
        // same `pill_item_width` the center arithmetic already uses
        m.insert(
            "pill_watts",
            SceneValue::Text(format!("{:.1}W", self.watts_shown)),
        );
        m.insert(
            "pill_watts_w",
            SceneValue::Ring(self.pill_item_width(PillItem::Watts)),
        );
        m.insert(
            "pill_user_w",
            SceneValue::Ring(self.pill_item_width(PillItem::User)),
        );

        // ── right cluster: bell · brightness · tray ──
        m.insert("pill_bell_on", SceneValue::Toggle(self.pill_notif));
        m.insert("pill_bell_key", SceneValue::Ring(62.0));
        m.insert("pill_notif_dot", SceneValue::Toggle(self.notif_count > 0));
        m.insert("pill_bright_on", SceneValue::Toggle(self.pill_brightness));
        m.insert("pill_bright_key", SceneValue::Ring(67.0));
        m.insert(
            "pill_bright_glyph",
            SceneValue::Text(
                if self.dark_mode {
                    crate::icons::ICON_MOON
                } else {
                    crate::icons::ICON_BRIGHTNESS
                }
                .to_string(),
            ),
        );
        m.insert_color("pill_bright_ink", ink(67));
        m.insert("pill_cc_key", SceneValue::Ring(63.0));
        let tray_n = if self.pill_tray {
            self.tray.len().min(6)
        } else {
            0
        };
        m.insert("pill_tray_on", SceneValue::Toggle(tray_n > 0));
        // The right cluster's SLOT in the outer row: its visible chips plus the
        // trailing margin, because the invisible control-center strip runs from
        // the cluster's left edge all the way to the pill's right edge.
        let mut right_used = 0.0f32;
        let mut n_right = 0usize;
        if self.pill_notif {
            right_used += 22.0;
            n_right += 1;
        }
        if self.pill_brightness {
            right_used += 22.0;
            n_right += 1;
        }
        if tray_n > 0 {
            right_used += tray_n as f32 * 22.0;
            n_right += 1;
        }
        m.insert(
            "pill_right_w",
            SceneValue::Ring(
                self.pill_margin_x + right_used + n_right.saturating_sub(1) as f32 * PILL_GAP,
            ),
        );
        m.insert(
            "pill_tray",
            SceneValue::List(
                self.tray
                    .iter()
                    .take(tray_n)
                    .map(|t| t.image_key())
                    .collect(),
            ),
        );
        m.insert("pill_tray_key", SceneValue::Ring(65.0));
    }

    #[cfg(test)]
    pub(crate) fn scene_values(&self) -> crate::scene::SceneValues {
        // EVERY surface's stores, merged. This is the whole-shell view: the
        // validator's and the tests' view, where "which surface is drawing" is
        // not a question being asked.
        //
        // A surface that DRAWS must not use this — it must use
        // [`Shell::scene_values_for`], which merges only its own stores. The
        // difference is not academic: a global merge means a store's prop
        // resolves in every surface, so two surfaces that both name a store
        // `nav` silently share one value, and a `nav.tab` written by the
        // settings surface moves the OSD's tab. The names are namespaced, and
        // a namespace nobody enforces is a convention.
        let mut vals = self.scene_values_raw();
        for surface in self
            .shell_scene
            .peek()
            .map(|ss| &ss.surfaces[..])
            .unwrap_or_default()
        {
            self.scene_stores.merge_into(&surface.name, &mut vals);
        }
        vals
    }

    /// The pool as the surface named `surface` sees it: Rust-published values
    /// plus ONLY that surface's own stores.
    ///
    /// Every panel layout and draw path builds its pool through here, so a
    /// store prop resolves where it was declared and nowhere else — which is
    /// what makes `Set` a closed loop: a write names `store.prop`, and the only
    /// reader that can see it is the surface that declared the store.
    ///
    /// Store props are merged LAST, so a store always wins a collision with a
    /// Rust-published name. That ordering is the point: a store is scene-owned
    /// state, and a Rust publisher that happens to use `dash.scroll` for
    /// something else must not be able to overwrite what a `Set` just wrote.
    pub(crate) fn scene_values_for(&self, surface: &str) -> crate::scene::SceneValues {
        let mut vals = self.scene_values_raw();
        self.scene_stores.merge_into(surface, &mut vals);
        vals
    }

    /// Seed every declared store from its defaults, for the surfaces already
    /// loaded. Idempotent — see [`crate::scene::SceneStoreState::seed`].
    ///
    /// Called at the TOP of the frame, before any `scene_values()`, because
    /// seeding inside the draw would make a store's default invisible on the
    /// very frame the store first appears: `scene_values()` runs first, so the
    /// merge would find nothing seeded and the surface would draw one frame
    /// with every `store.*` binding resolving to 0.
    #[cfg(test)]
    pub(crate) fn seed_scene_stores_for(&mut self, surface: &crate::scene::SurfaceScene) {
        self.scene_stores.seed(surface);
    }

    pub(crate) fn seed_scene_stores(&mut self) {
        // Ensure the shell scene is loaded (mtime-aware) even on the first
        // frame before any validated read. `get()` reloads if needed; we ignore
        // the reference and then seed from `peek()` so the frame remains
        // idempotent.
        let _ = self.shell_scene.get();
        for surface in self
            .shell_scene
            .peek()
            .map(|ss| &ss.surfaces[..])
            .unwrap_or_default()
        {
            self.scene_stores.seed(surface);
        }
    }

    /// The pool WITHOUT any store values merged in — what the store merge must
    /// read as its base, so it cannot see its own previous output.
    fn scene_values_raw(&self) -> crate::scene::SceneValues {
        use crate::scene::{
            GridCell, KaraokeLine, SceneColor, SceneRow, SceneTile, SceneValue, SceneValues,
        };
        use crate::shell::POWER_CARD_KEY_BASE;
        use crate::ui::ColorToken;
        let mut m = SceneValues::new();
        // Values published by $sources/lib modules, e.g. `mod_clock.time`.
        // Inserted FIRST on purpose: every built-in below then overwrites a
        // colliding key, so a plugin can add values but can never shadow the
        // shell's own (`clock`, `battery`, …). Plugins are expected to use a
        // dotted `mod_<module>.<name>` namespace anyway.
        merge_module_values(&mut m, crate::mods::values());
        m.insert("clock", SceneValue::Text(self.clock.clone()));
        m.insert("username", SceneValue::Text(self.username.clone()));
        m.insert("hostname", SceneValue::Text(self.hostname.clone()));
        m.insert("uptime", SceneValue::Text(self.uptime.clone()));
        m.insert("date", SceneValue::Text(self.date_str("%A, %e %b")));
        m.insert("battery", SceneValue::Text(self.battery.to_string()));
        m.insert(
            "battery_watts",
            SceneValue::Text(format!("{:.0}", self.battery_watts)),
        );
        m.insert("battery_time", SceneValue::Text(self.battery_time.clone()));
        m.insert(
            "volume",
            SceneValue::Text(format!("{:.0}", self.volume * 100.0)),
        );
        m.insert(
            "brightness",
            SceneValue::Text(format!("{:.0}", self.brightness * 100.0)),
        );
        m.insert(
            "mic_level",
            SceneValue::Text(format!("{:.0}", self.mic_level * 100.0)),
        );
        m.insert("cpu", SceneValue::Text(self.cpu.to_string()));
        m.insert("mem", SceneValue::Text(self.mem_pct.to_string()));
        m.insert("disk", SceneValue::Text(self.disk_pct.to_string()));
        m.insert(
            "cpu_temp",
            SceneValue::Text(format!("{:.0}°", self.cpu_temp)),
        );
        m.insert(
            "gpu_temp",
            SceneValue::Text(format!("{:.0}°", self.gpu_temp)),
        );
        m.insert("net_up", SceneValue::Text(self.net_up.clone()));
        m.insert("net_down", SceneValue::Text(self.net_down.clone()));
        m.insert(
            "kblayout",
            SceneValue::Text(if self.kb_layout.is_empty() {
                "Unknown".into()
            } else {
                self.kb_layout.clone()
            }),
        );
        // header metas — live strings the declarative `Header` chrome can
        // drop in for cards whose meta is dynamic (counts / state labels)
        m.insert(
            "disk_meta",
            SceneValue::Text(if self.disk_parts.is_empty() {
                String::new()
            } else {
                format!("{} mounts", self.disk_parts.len())
            }),
        );
        m.insert(
            "cpu_meta",
            SceneValue::Text(if self.cpu_freq_mhz > 0 {
                format!("{} MHz", self.cpu_freq_mhz)
            } else {
                String::new()
            }),
        );
        m.insert(
            "water_meta",
            SceneValue::Text(format!(
                "{:.1} / {:.1} L",
                self.water_glasses as f32 * 0.25,
                self.water_goal.max(1) as f32 * 0.25
            )),
        );
        m.insert(
            "pomo_meta",
            SceneValue::Text(if self.pomo_sessions > 0 {
                format!("{} done", self.pomo_sessions)
            } else {
                String::new()
            }),
        );
        m.insert(
            "speed_meta",
            SceneValue::Text(match self.speed_state {
                1 => "testing…".into(),
                2 => "done".into(),
                _ => "idle".into(),
            }),
        );
        m.insert(
            "wp_meta",
            SceneValue::Text(if !self.wallpapers.is_empty() {
                format!("{} wallpapers", self.wallpapers.len())
            } else {
                String::new()
            }),
        );
        // The wallpaper card is fully declarative (wallpaper.ron): a scrollable
        // tile grid plus its right-edge scrollbar. Every number the grid needs
        // is the card's own fit arithmetic — the adaptive cell, the column and
        // row counts, the clamped scroll, the centered block origin — so it
        // all comes from the one pure `wp_card_geometry` chain, and the scroll
        // is published as the EFFECTIVE start (clamped, unwritten: this is
        // `&self`, and the mutation belongs to the layout pass).
        {
            use crate::scene::GridCell;
            let g = self.wp_card_geometry();
            let one = self.scale.s(1.0);
            m.insert("wp_has", SceneValue::Toggle(!self.wallpapers.is_empty()));
            m.insert("wp_none", SceneValue::Toggle(self.wallpapers.is_empty()));
            m.insert("wp_cell", SceneValue::Ring(g.cell / one));
            m.insert("wp_cols", SceneValue::Ring(g.cols as f32));
            m.insert("wp_rows", SceneValue::Ring(g.rows as f32));
            m.insert("wp_scroll", SceneValue::Ring(g.start as f32));
            m.insert("wp_grid_x", SceneValue::Ring(g.x0 / one));
            m.insert("wp_grid_y", SceneValue::Ring(g.y0 / one));
            m.insert(
                "wp_cells",
                SceneValue::GridCells(
                    self.wallpapers
                        .iter()
                        .map(|p| GridCell {
                            // the tile paints the plate; the picture rides
                            // `image_inset` inside it
                            image_key: Some(format!("thumb:{p}")),
                            ..GridCell::default()
                        })
                        .collect(),
                ),
            );
            // ── the scrollbar: drawn only when the grid overflows ──
            m.insert("wp_sb", SceneValue::Toggle(g.max_scroll > 0));
            let hdr = self.wp_card_header_h();
            let track_y = hdr + self.scale.s(6.0);
            let track_h = (self.wp_card_rect.3 - hdr - self.scale.s(12.0)).max(self.scale.s(10.0));
            let total_rows = g.total_rows.max(1) as f32;
            let rows = g.rows as f32;
            let span = (g.total_rows.saturating_sub(g.rows)).max(1) as f32;
            // the thumb is `rows/total` of the track, positioned by the
            // scroll's fraction of the pages above it
            let frac = ((g.start as f32 / g.cols as f32) / span).clamp(0.0, 1.0);
            let thumb_h = (track_h * rows / total_rows).clamp(self.scale.s(10.0), track_h);
            let thumb_y = track_y + (track_h - thumb_h) * frac;
            m.insert("wp_sb_track_y", SceneValue::Ring(track_y / one));
            m.insert("wp_sb_track_h", SceneValue::Ring(track_h / one));
            m.insert("wp_sb_thumb_y", SceneValue::Ring(thumb_y / one));
            m.insert("wp_sb_thumb_h", SceneValue::Ring(thumb_h / one));
            // the page-jump bands: whatever the thumb left free above and below,
            // registered only when at least 4 px of it is left
            let up_h = (thumb_y - track_y - self.scale.s(2.0)).max(0.0);
            let down_y = (thumb_y + thumb_h + self.scale.s(2.0)).min(track_y + track_h);
            let down_h = (track_y + track_h - down_y).max(0.0);
            m.insert("wp_sb_up_y", SceneValue::Ring(track_y / one));
            m.insert("wp_sb_up_h", SceneValue::Ring(up_h / one));
            m.insert("wp_sb_up", SceneValue::Toggle(up_h >= self.scale.s(4.0)));
            m.insert("wp_sb_down_y", SceneValue::Ring(down_y / one));
            m.insert("wp_sb_down_h", SceneValue::Ring(down_h / one));
            m.insert(
                "wp_sb_down",
                SceneValue::Toggle(down_h >= self.scale.s(4.0)),
            );
            m.insert(
                "wp_sb_hover",
                SceneValue::Toggle(
                    self.hover_key == crate::shell::WP_CARD_SB_UP
                        || self.hover_key == crate::shell::WP_CARD_SB_DOWN,
                ),
            );
            // the three scrollbar inks are palette MIXES the palette cannot
            // name, so they ride per-frame colors (the `value(...)` idiom)
            let wpal = Pal {
                fg: self.sv_fg,
                bg: self.sv_bg,
                acc: self.sv_acc,
                sfg: self.sv_sfg,
            };
            m.insert_color(
                "wp_sb_track",
                SceneColor::Raw(mix(ui::hover(&wpal), wpal.fg, 0.08)),
            );
            m.insert_color(
                "wp_sb_thumb",
                SceneColor::Raw(mix(
                    ui::hover(&wpal),
                    wpal.fg,
                    if self.hover_key == crate::shell::WP_CARD_SB_UP
                        || self.hover_key == crate::shell::WP_CARD_SB_DOWN
                    {
                        0.55
                    } else {
                        0.32
                    },
                )),
            );
        }
        m.insert(
            "jr_meta",
            SceneValue::Text(if self.jr_lines.is_empty() {
                String::new()
            } else {
                format!("{} lines", self.jr_lines.len())
            }),
        );
        let sm_n = self.sm_disks.len();
        m.insert(
            "sm_meta",
            SceneValue::Text(if sm_n == 0 {
                String::new()
            } else {
                format!("{sm_n} disk{}", if sm_n == 1 { "" } else { "s" })
            }),
        );
        m.insert(
            "sm_icon",
            SceneValue::Text(if self.sm_disks.iter().any(|(_, _, ok, _)| !ok) {
                ICON_SHIELD.into()
            } else {
                ICON_CHECK.into()
            }),
        );
        m.insert(
            "auddev_meta",
            SceneValue::Text(if self.audio_sinks.len() + self.audio_sources.len() > 0 {
                format!("{} dev", self.audio_sinks.len() + self.audio_sources.len())
            } else {
                String::new()
            }),
        );
        // AUDIO DEVICE rows — sinks first, then sources, the drawer's walk
        // order. The star/arrow prefix is part of the name cell (the drawer
        // builds one `full` string and truncates it as a whole), and the
        // per-row colors are the drawer's three: name fg/fg2 (accent-bright
        // only for the default), pct always fg3, glyph danger-when-muted.
        {
            let total = self.audio_sinks.len() + self.audio_sources.len();
            let mut rows: Vec<crate::scene::SceneRow> = Vec::with_capacity(total);
            // The pct ink is published per row rather than given a
            // `SceneCol.hover` token: the drawer tests MUTED FIRST
            // (`if *muted { fg3 } else { hover… }`), so a muted row's pct never
            // tints, but a col `hover` token outranks any published color
            // during hover. The `.ron` therefore leaves the pct col's `hover`
            // unset and this chain reproduces the drawer exactly — the row key
            // test alone is enough, since hovering the mute box puts the
            // pointer on the aux key, so the row is not hovered (the drawer's
            // `hover_key != mkey` guard).
            let pct_ink = |ri: usize, muted: bool| -> crate::ui::ColorToken {
                let key = AUDIO_DEV_BASE + (ri as u32) * 2;
                if muted {
                    crate::ui::ColorToken::Fg3
                } else if self.hover_key == key {
                    crate::ui::ColorToken::HoverFg
                } else {
                    crate::ui::ColorToken::Fg3
                }
            };
            let mut ri = 0usize;
            for (id, name, vol, muted) in self.audio_sinks.clone() {
                let is_default = id == self.audio_default;
                let prefix = if is_default {
                    format!("{} ", ICON_STAR)
                } else {
                    String::new()
                };
                rows.push(crate::scene::SceneRow {
                    cols: vec![
                        format!("{prefix}{name}"),
                        format!("{:>3}%", (vol * 100.0).round() as i32),
                        if muted {
                            ICON_MUTE.into()
                        } else {
                            ICON_VOLUME_FILL.into()
                        },
                    ],
                    key: 0,
                    action: None,
                    color: None,
                    col_colors: vec![
                        Some(if is_default {
                            crate::ui::ColorToken::Fg
                        } else {
                            crate::ui::ColorToken::Fg2
                        }),
                        Some(pct_ink(ri, muted)),
                        Some(if muted {
                            crate::ui::ColorToken::Danger
                        } else {
                            crate::ui::ColorToken::Fg3
                        }),
                    ],
                    surface: None,
                });
                ri += 1;
            }
            for (_id, name, vol, muted) in self.audio_sources.clone() {
                rows.push(crate::scene::SceneRow {
                    cols: vec![
                        format!("{} {name}", ICON_ARROW_FORWARD),
                        format!("{:>3}%", (vol * 100.0).round() as i32),
                        if muted {
                            ICON_MUTE.into()
                        } else {
                            ICON_VOLUME_FILL.into()
                        },
                    ],
                    key: 0,
                    action: None,
                    color: None,
                    col_colors: vec![
                        Some(crate::ui::ColorToken::Fg2),
                        Some(pct_ink(ri, muted)),
                        Some(if muted {
                            crate::ui::ColorToken::Danger
                        } else {
                            crate::ui::ColorToken::Fg3
                        }),
                    ],
                    surface: None,
                });
                ri += 1;
            }
            let has = !rows.is_empty();
            m.insert("auddev_rows", SceneValue::Rows(rows));
            m.insert("auddev_has", SceneValue::Toggle(has));
            m.insert("auddev_empty", SceneValue::Toggle(!has));
        }
        m.insert(
            "conn_meta",
            SceneValue::Text(format!("{} addrs", self.conn_rows.len())),
        );
        // the moon's whole body is declarative (moon.ron): the disc's box is
        // the card's own `body_h`/`min`/`max` arithmetic and the captions are
        // the drawer's strings, so both publish here. ONE phase sample feeds
        // the header meta, the disc and the captions.
        {
            use crate::shell::panels::cards::moon as moonmath;
            let (phase, illum) = moonmath::moon_phase(self.moon_epoch);
            m.insert(
                "moon_meta",
                SceneValue::Text(format!("{:.0}%", illum * 100.0)),
            );
            m.insert("moon_phase", SceneValue::Ring(phase as f32));
            m.insert(
                "moon_name",
                SceneValue::Text(moonmath::moon_name(phase).to_string()),
            );
            m.insert(
                "moon_lit",
                SceneValue::Text(format!("{:.0}% lit", illum * 100.0)),
            );
            let dtf = moonmath::days_to_full(phase);
            m.insert(
                "moon_sub",
                SceneValue::Text(if dtf < 0.8 {
                    "full tonight".to_string()
                } else {
                    format!("{:.0} d to full", dtf)
                }),
            );
            let (_, _, cw, ch) = self.moon_rect;
            let s = self.scale.s(1.0);
            let pad = self.scale.s(12.0);
            let hdr = self.scale.s(24.0);
            let body_top = hdr + self.scale.s(4.0);
            // NOTE: the drawer's `body_h` is the remaining EXTENT, and `d` is
            // the min of the height room and the width room (less the 90 px
            // the side text needs), floored at 30 px
            let body_h = ch - body_top - self.scale.s(34.0);
            let d = body_h
                .min(cw - pad * 2.0 - self.scale.s(90.0))
                .max(self.scale.s(30.0));
            let cx = pad + d / 2.0 + self.scale.s(4.0);
            let cy = body_top + body_h / 2.0;
            m.insert("moon_d", SceneValue::Ring(d / s));
            m.insert("moon_cx", SceneValue::Ring(cx / s));
            m.insert("moon_cy", SceneValue::Ring(cy / s));
            let tx = cx + d / 2.0 + self.scale.s(12.0);
            // the side column only appears when the card has room for it
            let side = cw - pad - tx > self.scale.s(60.0);
            m.insert("moon_side", SceneValue::Toggle(side));
            m.insert("moon_tx", SceneValue::Ring(tx / s));
            m.insert("moon_ty_0", SceneValue::Ring((cy - self.scale.s(12.0)) / s));
            m.insert("moon_ty_1", SceneValue::Ring((cy + self.scale.s(3.0)) / s));
            m.insert("moon_ty_2", SceneValue::Ring((cy + self.scale.s(14.0)) / s));
        }
        m.insert(
            "comp_meta",
            SceneValue::Text({
                let on = [self.fx_blur_on, self.fx_shadow_on, self.fx_opacity_on]
                    .iter()
                    .filter(|b| **b)
                    .count();
                format!("{on} on")
            }),
        );
        m.insert(
            "fan_meta",
            SceneValue::Text(if self.fans_rpm.is_empty() {
                String::new()
            } else {
                format!("{} fans", self.fans_rpm.len())
            }),
        );
        m.insert(
            "er_meta",
            SceneValue::Text(if self.er_sessions > 0 {
                format!("{} rests", self.er_sessions)
            } else {
                String::new()
            }),
        );
        m.insert(
            "thermal_meta",
            SceneValue::Text(if self.thermal_zones.is_empty() {
                String::new()
            } else {
                format!("{} zones", self.thermal_zones.len())
            }),
        );
        m.insert(
            "wc_meta",
            SceneValue::Text(if self.worldclock_zones.is_empty() {
                String::new()
            } else {
                format!("{} cities", self.worldclock_zones.len())
            }),
        );
        m.insert(
            "mic_meta",
            SceneValue::Text(format!("{:>3}%", (self.mic_level * 100.0).round() as i32)),
        );
        m.insert(
            "mic_icon",
            SceneValue::Text(if self.mic_muted {
                ICON_MIC_OFF.into()
            } else {
                ICON_MIC.into()
            }),
        );
        m.insert(
            "sus_meta",
            SceneValue::Text(if self.sus_failed.is_empty() {
                String::new()
            } else {
                format!("{} failed", self.sus_failed.len())
            }),
        );
        m.insert(
            "sus_icon",
            SceneValue::Text(if self.sus_failed.is_empty() {
                ICON_CHECK.into()
            } else {
                ICON_SHIELD.into()
            }),
        );
        m.insert(
            "ws_meta",
            SceneValue::Text({
                let n = self.ws_count.max(1);
                format!("{}/{}", (self.ws_active + 1).min(n), n)
            }),
        );

        // ── typed values (data-driven primitives) —──────────────────────
        // Ring: charge gauge (0..100; -1 = no battery → 0).
        m.insert("battery_ring", SceneValue::Ring(self.battery.max(0) as f32));
        m.insert("mem_pct_f", SceneValue::Ring(self.mem_pct.max(0) as f32));
        // ── gauges / water / eyerest — the three bead-ring gauge cards.
        // The rings' geometry lives in the RON (responsive `fit`/`dot_ladder`,
        // the `When` size windows); only the live figures + the blends the
        // Rust drawers computed by hand are published here.
        m.insert("disk_pct_f", SceneValue::Ring(self.disk_pct.max(0) as f32));
        m.insert(
            "gau_mem_pct",
            SceneValue::Text(format!("{}%", self.mem_pct)),
        );
        m.insert(
            "gau_disk_pct",
            SceneValue::Text(format!("{}%", self.disk_pct)),
        );
        // below-gauge capacity line: used/total in GB, else the % again
        m.insert(
            "gau_mem_gb",
            SceneValue::Text(if self.mem_total_gb > 0.0 {
                format!("{:.0}G/{:.0}G", self.mem_used_gb, self.mem_total_gb)
            } else {
                format!("{}%", self.mem_pct)
            }),
        );
        m.insert(
            "gau_disk_gb",
            SceneValue::Text(if self.disk_total_gb > 0.0 {
                format!("{:.0}G/{:.0}G", self.disk_used_gb, self.disk_total_gb)
            } else {
                format!("{}%", self.disk_pct)
            }),
        );
        let pal = Pal {
            fg: self.sv_fg,
            bg: self.sv_bg,
            acc: self.sv_acc,
            sfg: self.sv_sfg,
        };
        m.insert_color(
            "gau_dim",
            SceneColor::Raw(mix(ui::hover(&pal), pal.fg, 0.08)),
        );
        // Pomodoro (zero-`Ink` card since 2026-09-27): the countdown figure, its
        // phase ink, the bar's fill and the four button fills are all per-frame
        // (phase + running + hover), so the shell publishes them; the frame —
        // hero, caption, bar, the `[-] 25m [+]` trio and the Start/Reset pair —
        // lives in `pomodoro.ron`. The trio's chips flank a runtime string, so
        // their center-relative x rides the drawer's own text-width estimate
        // (`pomo_dec_dx` / `pomo_inc_dx`).
        let pomo_rem = self.pomo_remaining_secs();
        let pomo_phase_c = if self.pomo_phase_focus {
            pal.fg
        } else {
            ui::OK
        };
        m.insert(
            "pomo_hero",
            SceneValue::Text(format!("{:02}:{:02}", pomo_rem / 60, pomo_rem % 60)),
        );
        m.insert_color(
            "pomo_ink",
            SceneColor::Raw(if self.pomo_running {
                pomo_phase_c
            } else {
                mix(ui::hover(&pal), pomo_phase_c, 0.25)
            }),
        );
        m.insert_color("pomo_bar_ink", SceneColor::Raw(pomo_phase_c));
        m.insert(
            "pomo_phase_lbl",
            SceneValue::Text(if self.pomo_phase_focus {
                "focus".into()
            } else {
                "break".into()
            }),
        );
        let pomo_total = if self.pomo_phase_focus {
            self.pomo_focus_min
        } else {
            self.pomo_break_min
        }
        .max(1) as f32
            * 60.0;
        m.insert(
            "pomo_frac",
            SceneValue::Text(format!(
                "{}",
                (1.0 - pomo_rem as f32 / pomo_total).clamp(0.0, 1.0)
            )),
        );
        m.insert("pomo_idle", SceneValue::Toggle(!self.pomo_running));
        let pomo_readout = format!("{}m", self.pomo_focus_min);
        // the drawer's own readout-width estimate (`0.62 · fs(8.5) · chars + 2`),
        // shared via `pomo_readout_w` so the chips never drift
        let pomo_rw = self.pomo_readout_w();
        m.insert("pomo_focus_readout", SceneValue::Text(pomo_readout));
        // the drawer's own algebra: `cx0 = w/2 − rw/2 − 18 − 6` for `[-]`, and
        // `cx0 + 18 + 6 + rw` collapses back to `w/2 + rw/2` for `[+]`
        m.insert("pomo_dec_dx", SceneValue::Ring(-pomo_rw / 2.0 - 24.0));
        m.insert("pomo_inc_dx", SceneValue::Ring(pomo_rw / 2.0));
        m.insert_color(
            "pomo_chip_fill",
            SceneColor::Raw(mix(ui::hover(&pal), pal.fg, 0.10)),
        );
        m.insert_color(
            "pomo_tog_fill",
            SceneColor::Raw((pal.acc & 0xFFFF_FF00) | 0x28),
        );
        m.insert_color(
            "pomo_tog_hover",
            SceneColor::Raw(mix(ui::hover(&pal), pal.acc, 0.25)),
        );
        m.insert(
            "pomo_tog_lbl",
            SceneValue::Text(
                if self.pomo_running {
                    "Pause"
                } else if self.pomo_paused_rem.is_some() {
                    "Resume"
                } else {
                    "Start"
                }
                .into(),
            ),
        );
        // Workspaces tile grid (zero-`Ink` card since 2026-09-27): one cell per
        // workspace, 1-based label, the active one keeping its accent wash (the
        // drawer's `(acc & 0xFFFFFF00) | 0x28` — published as a bound color so
        // the tile is a straight alpha wash, not an `acc_tint` mix) plus its
        // accent ink and its bottom-center pip. The frame (columns, caps,
        // centering, radii, the width ladder) lives in `workspaces.ron`.
        let ws_n = self.ws_count.max(1);
        m.insert_color(
            "ws_fill_act",
            SceneColor::Raw((pal.acc & 0xFFFF_FF00) | 0x28),
        );
        m.insert(
            "ws_tiles",
            SceneValue::GridCells(
                (0..ws_n)
                    .map(|i| {
                        let active = i == self.ws_active;
                        let fill = if active {
                            ColorToken::Value(crate::ui::DynColor::new("ws_fill_act").unwrap())
                        } else {
                            ColorToken::Hover
                        };
                        GridCell {
                            text: (i + 1).to_string(),
                            surface: Some(fill),
                            // the active tile KEEPS its wash under the cursor
                            // (the drawer tests `active` before hover)
                            surface_hover: if active { Some(fill) } else { None },
                            color: if active { Some(ColorToken::Acc) } else { None },
                            marker: if active { Some(ColorToken::Acc) } else { None },
                            vcenter: true,
                            ..GridCell::default()
                        }
                    })
                    .collect(),
            ),
        );
        {
            // water: glasses/goal, capped so an over-goal day keeps the ring
            // full instead of wrapping (the drawer's `min(1.0)`)
            let frac = (self.water_glasses as f32 / self.water_goal.max(1) as f32).min(1.0);
            let done = frac >= 1.0;
            m.insert("water_frac", SceneValue::Ring(frac));
            m.insert("water_done", SceneValue::Toggle(done));
            m.insert(
                "water_hero",
                SceneValue::Text(format!(
                    "{}{}",
                    self.water_glasses,
                    if done { " \u{2713}" } else { "" }
                )),
            );
            m.insert(
                "water_goal_txt",
                SceneValue::Text(format!("of {} glasses", self.water_goal)),
            );
            m.insert_color(
                "water_ring_ink",
                SceneColor::Raw(if done { ui::OK } else { ui::INFO }),
            );
            m.insert_color(
                "water_hero_ink",
                SceneColor::Raw(if done { ui::OK } else { pal.fg }),
            );
            m.insert_color(
                "water_dim",
                SceneColor::Raw(mix(ui::hover(&pal), pal.fg, 0.10)),
            );
            // − / + glass buttons: the drawer's 3-state edit-button fill
            m.insert_color(
                "water_dec_bg",
                SceneColor::Raw(if self.hover_key == crate::shell::WATER_DEC_KEY {
                    ui::hover_hl(&pal)
                } else {
                    mix(ui::hover(&pal), pal.fg, 0.10)
                }),
            );
            m.insert_color(
                "water_inc_bg",
                SceneColor::Raw(if self.hover_key == crate::shell::WATER_INC_KEY {
                    mix(ui::hover(&pal), pal.acc, 0.25)
                } else {
                    (pal.acc & 0xFFFF_FF00) | 0x28
                }),
            );
        }
        {
            // eye rest: the focus/rest fraction + the label/caption/ink ladder
            use self::cards::{ER_FOCUS_SECS, ER_REST_SECS};
            let resting = self.er_rest_until.is_some();
            let rem = self.er_remaining_secs();
            let total = if resting { ER_REST_SECS } else { ER_FOCUS_SECS };
            let frac = (total.saturating_sub(rem) as f32 / total as f32).clamp(0.0, 1.0);
            m.insert("er_frac", SceneValue::Ring(frac));
            m.insert(
                "er_hero",
                SceneValue::Text(format!("{:02}:{:02}", rem / 60, rem % 60)),
            );
            m.insert("er_resting", SceneValue::Toggle(resting));
            m.insert(
                "er_state",
                SceneValue::Text(
                    if resting {
                        "rest your eyes"
                    } else if self.er_running {
                        "focus"
                    } else if self.er_paused_rem.is_some() {
                        "paused"
                    } else {
                        "ready"
                    }
                    .into(),
                ),
            );
            m.insert(
                "er_tog_lbl",
                SceneValue::Text(
                    if resting {
                        "Skip"
                    } else if self.er_running {
                        "Pause"
                    } else if self.er_paused_rem.is_some() {
                        "Resume"
                    } else {
                        "Start"
                    }
                    .into(),
                ),
            );
            m.insert_color(
                "er_ring_ink",
                SceneColor::Raw(if resting { ui::OK } else { pal.acc }),
            );
            m.insert_color(
                "er_hero_ink",
                SceneColor::Raw(if resting { ui::OK } else { pal.fg }),
            );
            m.insert_color(
                "er_tog_bg",
                SceneColor::Raw(if self.hover_key == crate::shell::EYEREST_TOGGLE_KEY {
                    mix(ui::hover(&pal), pal.acc, 0.25)
                } else {
                    (pal.acc & 0xFFFF_FF00) | 0x28
                }),
            );
            m.insert_color(
                "er_rst_bg",
                SceneColor::Raw(if self.hover_key == crate::shell::EYEREST_RESET_KEY {
                    ui::hover_hl(&pal)
                } else {
                    mix(ui::hover(&pal), pal.fg, 0.10)
                }),
            );
        }
        // cpu / mem — declarative bead-ring gauges + metric rows (cpu.ron,
        // mem.ron). The cpu gauge + hero % follow the Rust warn/crit ladder
        // (fg → warn ≥ 70 → danger ≥ 90) via a per-frame dynamic color.
        m.insert("cpu_ring", SceneValue::Ring(self.cpu.max(0) as f32));
        m.insert_color(
            "cpu_gauge_col",
            SceneColor::Token(if self.cpu >= 90 {
                ColorToken::Danger
            } else if self.cpu >= 70 {
                ColorToken::Warn
            } else {
                ColorToken::Fg
            }),
        );
        let cpu_freq = if self.cpu_freq_mhz >= 1000 {
            format!("{:.2} GHz", self.cpu_freq_mhz as f32 / 1000.0)
        } else if self.cpu_freq_mhz > 0 {
            format!("{} MHz", self.cpu_freq_mhz)
        } else {
            String::new()
        };
        let cpu_temp_col = if self.cpu_temp > 80.0 {
            ColorToken::Danger
        } else if self.cpu_temp > 65.0 {
            ColorToken::Warn
        } else {
            ColorToken::Fg
        };
        m.insert(
            "cpu_rows",
            SceneValue::Rows(vec![
                crate::scene::SceneRow {
                    cols: vec![
                        "Load".to_string(),
                        format!(
                            "{:.2}/{:.2}/{:.2}",
                            self.cpu_load.0, self.cpu_load.1, self.cpu_load.2
                        ),
                    ],
                    key: 0,
                    action: None,
                    color: None,
                    surface: None,
                    col_colors: vec![],
                },
                crate::scene::SceneRow {
                    cols: vec![
                        "Freq".to_string(),
                        if cpu_freq.is_empty() {
                            "—".to_string()
                        } else {
                            cpu_freq.clone()
                        },
                    ],
                    key: 0,
                    action: None,
                    color: None,
                    surface: None,
                    col_colors: vec![],
                },
                crate::scene::SceneRow {
                    cols: vec![
                        "Temp".to_string(),
                        if self.cpu_temp > 0.0 {
                            format!("{:.0}°C", self.cpu_temp)
                        } else {
                            "—".to_string()
                        },
                    ],
                    key: 0,
                    action: None,
                    color: None,
                    surface: None,
                    col_colors: vec![None, Some(cpu_temp_col)],
                },
            ]),
        );
        let mem_rows = |label: &str, gb: f32| -> crate::scene::SceneRow {
            crate::scene::SceneRow {
                cols: vec![label.to_string(), format!("{gb:.1}G")],
                key: 0,
                action: None,
                color: None,
                surface: None,
                col_colors: vec![],
            }
        };
        m.insert(
            "mem_rows",
            SceneValue::Rows(vec![
                mem_rows("Total", self.mem_total_gb),
                mem_rows("Available", self.mem_avail_gb),
                mem_rows("Cached", self.mem_cached_gb),
                mem_rows("Free", self.mem_free_gb),
            ]),
        );
        // swap bar: fraction-of-100 value string + presence toggle
        m.insert("swap_frac", SceneValue::Text(self.swap_pct.to_string()));
        m.insert("swap_show", SceneValue::Toggle(self.swap_pct > 0));
        // Fader: volume / brightness / mic, all 0..1.
        m.insert("volume_fader", SceneValue::Fader(self.volume));
        m.insert("brightness_fader", SceneValue::Fader(self.brightness));
        m.insert("mic_fader", SceneValue::Fader(self.mic_level));
        // Toggle: the shell's boolean state flags.
        m.insert("wifi_on", SceneValue::Toggle(self.wifi_on));
        m.insert("bt_on", SceneValue::Toggle(self.bt_on));
        // read live: the battery cards' Rust drawers used to be the only
        // `load_power_save` callers and they no longer run
        m.insert("power_save_on", SceneValue::Toggle(self.power_save_state()));
        m.insert("fx_blur_on", SceneValue::Toggle(self.fx_blur_on));
        m.insert("fx_shadow_on", SceneValue::Toggle(self.fx_shadow_on));
        m.insert("fx_opacity_on", SceneValue::Toggle(self.fx_opacity_on));
        // Composer: the note cards' live input strip (To-Do/Notes/Snippets).
        m.insert("todo_focus", SceneValue::Toggle(self.todo_input.is_some()));
        m.insert(
            "todo_buf",
            SceneValue::Text(self.todo_input.clone().unwrap_or_default()),
        );
        // Rows: the To-Do checklist as a row list (each row carries the label
        // only — key/✕ are scene-side via `key_base`/`del_base`), plus the
        // parallel per-row checked series for the checkbox column.
        m.insert(
            "todo_rows",
            SceneValue::Rows(
                self.todos
                    .iter()
                    .map(|(text, _)| crate::scene::SceneRow {
                        cols: vec![text.clone()],
                        key: 0,
                        action: None,
                        color: Some(crate::ui::ColorToken::Fg),
                        surface: None,
                        col_colors: vec![],
                    })
                    .collect(),
            ),
        );
        m.insert(
            "todo_checks",
            SceneValue::Checks(self.todos.iter().map(|(_, done)| *done).collect()),
        );
        m.insert("todo_scroll", SceneValue::Ring(self.todo_scroll as f32));
        // the `todo` card's chrome (it is zero-`Ink` now): the done/total
        // count in the header meta, the empty-state gate, and the empty
        // state's quiet glyph ink (`mix(hover, fg, 0.10)` — a blend no token
        // spells). The "+N more" footer needs no value: the `Rows` item owns
        // the hidden-row count and formats it into its own `more` template.
        let todo_n = self.todos.len();
        let todo_done = self.todos.iter().filter(|(_, d)| *d).count();
        m.insert(
            "todo_count",
            SceneValue::Text(if todo_n > 0 {
                format!("{todo_done}/{todo_n}")
            } else {
                String::new()
            }),
        );
        m.insert("todo_empty", SceneValue::Toggle(todo_n == 0));
        m.insert_color(
            "todo_empty_ink",
            SceneColor::Raw(mix(ui::hover(&pal), pal.fg, 0.10)),
        );

        // disk card rows (declared `disk.ron`, zero `Ink`): mount · right mono
        // `used/total GB pct%` · a meter bar whose fill follows the shared
        // warn/crit ladder through the per-row `col_colors` ink.
        {
            let parts = self.disk_parts.clone();
            let has = !parts.is_empty();
            m.insert(
                "disk_rows",
                SceneValue::Rows(
                    parts
                        .iter()
                        .map(|(mount, pct, used_gb, total_gb)| crate::scene::SceneRow {
                            cols: vec![
                                mount.clone(),
                                format!("{used_gb:.0}/{total_gb:.0} GB {pct}%"),
                                pct.to_string(),
                            ],
                            key: 0,
                            action: None,
                            color: None,
                            col_colors: vec![
                                Some(ColorToken::Fg2),
                                Some(ColorToken::Fg),
                                // the shared warn/crit ladder (75 / 90), as tokens
                                Some(if *pct >= 90 {
                                    ColorToken::Danger
                                } else if *pct >= 75 {
                                    ColorToken::Warn
                                } else {
                                    ColorToken::Acc
                                }),
                            ],
                            surface: None,
                        })
                        .collect(),
                ),
            );
            m.insert("disk_live", SceneValue::Toggle(has));
            m.insert("disk_empty", SceneValue::Toggle(!has));
        }
        // gpu card (declared `gpu.ron`, zero `Ink`): util hero + bar, the temp
        // row, and the VRAM row/bar that hides when the source exposes none.
        {
            let pct = self.gpu.max(0).clamp(0, 100);
            let has = self.gpu >= 0 || self.gpu_temp > 0.0;
            let vram_total = self.gpu_vram_total_mb;
            m.insert("gpu_pct", SceneValue::Text(format!("{pct}%")));
            m.insert("gpu_frac", SceneValue::Ring(pct as f32 / 100.0));
            m.insert(
                "gpu_temp_c",
                SceneValue::Text(format!("{:.0} °C", self.gpu_temp)),
            );
            m.insert("gpu_live", SceneValue::Toggle(has));
            // the empty state hangs off the INVERSE of the live body
            m.insert("gpu_empty", SceneValue::Toggle(!has));
            m.insert("gpu_vram_live", SceneValue::Toggle(vram_total > 0));
            if vram_total > 0 {
                let used_gb = self.gpu_vram_used_mb as f32 / 1024.0;
                let total_gb = vram_total as f32 / 1024.0;
                m.insert(
                    "gpu_vram_txt",
                    SceneValue::Text(format!("VRAM {used_gb:.1}/{total_gb:.1} GB")),
                );
                m.insert(
                    "gpu_vram_frac",
                    SceneValue::Ring(self.gpu_vram_used_mb as f32 / vram_total.max(1) as f32),
                );
            }
        }
        // cpugpu card (declared `cpugpu.ron`, zero `Ink`): the two legend lines
        // and the CPU/GPU history graph adapt to the mode ($states/m: d/l) —
        // brighter in dark, darker in light, so the lines read against the
        // transparent card. CPU = accent, GPU = muted info blue.
        {
            let cpu_col = mix(pal.acc, pal.fg, if self.dark_mode { 0.35 } else { 0.30 });
            let gpu_col = mix(ui::INFO, pal.fg, if self.dark_mode { 0.10 } else { 0.40 });
            m.insert("cpugpu_cpu", SceneValue::Text(format!("CPU {}%", self.cpu)));
            m.insert(
                "cpugpu_gpu",
                SceneValue::Text(if self.gpu >= 0 {
                    format!("GPU {}%", self.gpu)
                } else {
                    "GPU n/a".into()
                }),
            );
            m.insert("cpugpu_gpu_live", SceneValue::Toggle(self.gpu >= 0));
            m.insert_color("cpugpu_cpu_ink", SceneColor::Raw(cpu_col));
            m.insert_color("cpugpu_gpu_ink", SceneColor::Raw(gpu_col));
        }
        // speedtest card (declared `speedtest.ron`, zero `Ink`): the three
        // state captions, the done-state figures, and the run pill. The
        // pill's fill + ink follow state AND hover (the Rust button's
        // `mix(acc, fg, .15)` lift), which no static token can express, so
        // the shell publishes them the same way it publishes `power_hold_fill`.
        {
            let p = &pal;
            let testing = self.speed_state == 1;
            let done = self.speed_state == 2;
            let hov = self.hover_key == crate::shell::SPEED_KEY_BASE;
            m.insert("st_idle", SceneValue::Toggle(self.speed_state == 0));
            m.insert("st_testing", SceneValue::Toggle(testing));
            m.insert("st_done", SceneValue::Toggle(done));
            m.insert("st_clickable", SceneValue::Toggle(!testing));
            m.insert(
                "st_down",
                SceneValue::Text(format!("down {}", self.speed_down)),
            );
            m.insert("st_up", SceneValue::Text(format!("up {}", self.speed_up)));
            m.insert(
                "st_ping",
                SceneValue::Text(format!("ping {} ms", self.speed_ping)),
            );
            // header meta ink: idle/testing fg3, done = the muted OK green
            m.insert_color(
                "st_state_ink",
                if done {
                    SceneColor::Raw(crate::ui::OK)
                } else {
                    SceneColor::Raw(crate::ui::fg3(p))
                },
            );
            let (fill, ink) = if testing {
                (mix(crate::ui::hover(p), p.fg, 0.10), crate::ui::fg3(p))
            } else if hov {
                (mix(p.acc, p.fg, 0.15), 0xff14_14_14)
            } else {
                (p.acc, 0xff14_14_14)
            };
            m.insert_color("st_btn_fill", SceneColor::Raw(fill));
            m.insert_color("st_btn_ink", SceneColor::Raw(ink));
            m.insert(
                "st_btn_label",
                SceneValue::Text(if testing {
                    "testing…".to_string()
                } else if done {
                    format!("{}  run again", crate::icons::ICON_REFRESH)
                } else {
                    format!("{}  run test", crate::icons::ICON_PLAY)
                }),
            );
        }
        // docker card (declared `docker.ron`, zero `Ink`): one row per
        // container — a status dot (Up = the muted OK green, anything else
        // the danger red, the drawer's own two constants), the name, and the
        // image label. The dot's ink rides `col_colors`, so the `dot` cell
        // needs no text of its own.
        {
            let rows: Vec<crate::scene::SceneRow> = self
                .docker_containers
                .iter()
                .map(|(name, status, image)| crate::scene::SceneRow {
                    cols: vec![String::new(), name.clone(), image.clone()],
                    key: 0,
                    action: None,
                    color: None,
                    surface: None,
                    col_colors: vec![Some(if status.contains("Up") {
                        crate::ui::ColorToken::Ok
                    } else {
                        crate::ui::ColorToken::Danger
                    })],
                })
                .collect();
            m.insert("docker_empty", SceneValue::Toggle(rows.is_empty()));
            m.insert("docker_rows", SceneValue::Rows(rows));
        }
        // micmeter card (declared `micmeter.ron`, zero `Ink`): the live input
        // level bar, its status caption, and the mute button. The shell owns
        // the three state-driven colors the drawer computes inline (the red
        // clip above 90%, the muted level collapse, the hover lift).
        {
            let p = &pal;
            let muted = self.mic_muted;
            let lvl = if muted {
                0.0
            } else {
                self.mic_level.clamp(0.0, 1.0)
            };
            m.insert("mic_lvl", SceneValue::Text(format!("{lvl:.4}")));
            m.insert_color(
                "mic_bar_ink",
                if muted {
                    SceneColor::Raw(crate::ui::fg3(p))
                } else if lvl > 0.9 {
                    SceneColor::Raw(crate::ui::DANGER)
                } else {
                    SceneColor::Raw(p.acc)
                },
            );
            m.insert(
                "mic_status",
                SceneValue::Text(if muted {
                    "muted".to_string()
                } else if lvl > 0.05 {
                    "live".to_string()
                } else {
                    "quiet".to_string()
                }),
            );
            m.insert_color(
                "mic_status_ink",
                if muted {
                    SceneColor::Raw(crate::ui::fg3(p))
                } else if lvl > 0.9 {
                    SceneColor::Raw(crate::ui::DANGER)
                } else {
                    SceneColor::Raw(crate::ui::fg2(p))
                },
            );
            let hov = self.hover_key == crate::shell::MIC_MUTE_KEY;
            m.insert_color(
                "mic_btn_fill",
                SceneColor::Raw(if hov {
                    crate::ui::raised_hl(p)
                } else {
                    crate::ui::raised(p)
                }),
            );
            m.insert_color(
                "mic_btn_ink",
                SceneColor::Raw(if muted { p.acc } else { crate::ui::fg2(p) }),
            );
            m.insert(
                "mic_btn_label",
                SceneValue::Text(format!(
                    "{} {}",
                    if muted { ICON_MIC_OFF } else { ICON_MIC },
                    if muted { "Unmute mic" } else { "Mute mic" }
                )),
            );
        }
        // The branding card is fully declarative (branding.ron): the mark is
        // aspect-fit inside the body (the SMALLER of a ~0.62 em-per-char width
        // slot or the full body height, times 0.95, floored at 10), so both its
        // size and its top are computed per frame and bound through `size_var`
        // / `y_var` in BASE px. The glyph is re-read here too — the drawer that
        // used to do it never runs now, and `$states2/d` can change at any time.
        {
            let (_, _, rw, rh) = self.branding_rect;
            let pad = 16.0;
            let hdr = 24.0;
            let avail_w = (self.scale.base(rw) - pad * 2.0).max(1.0);
            let avail_h = (self.scale.base(rh) - hdr - pad).max(1.0);
            let glyph = crate::vars::read_branding_glyph();
            let chars = glyph.chars().count().max(1) as f32;
            let size = ((avail_w / (chars * 0.62)).min(avail_h) * 0.95).max(10.0);
            m.insert("brand_glyph", SceneValue::Text(glyph.clone()));
            m.insert("brand_size", SceneValue::Ring(size));
            m.insert("brand_y", SceneValue::Ring(hdr + (avail_h - size) / 2.0));
            m.insert("brand_live", SceneValue::Toggle(!glyph.is_empty()));
            m.insert("brand_empty", SceneValue::Toggle(glyph.is_empty()));
        }
        // The accent card's PICKER is declarative (accent.ron): the scheme row
        // and the three accent sources. The block is centered in the card with
        // a clamp (`((h − 156) / 2).clamp(30, 48)`), so both the scheme row and
        // the list bind their y to a published anchor; the custom-accents
        // subview stays in the Rust drawer (paged scroll + the
        // `accent_list_rect` write-back) behind `accent_picker`.
        {
            let (_, _, _, rh) = self.accent_rect;
            let top = ((self.scale.base(rh) - 156.0) * 0.5).clamp(30.0, 48.0);
            let cur = self.current_acc_source();
            let cur_name = self.current_acc_name();
            let sname = if self.cur_scheme.is_empty() {
                "default"
            } else {
                self.cur_scheme.as_str()
            };
            m.insert("accent_top", SceneValue::Ring(top));
            // the three y's the drawer's picker uses off that anchor: the
            // scheme row's region (top + 28), its label (6.5 into the row),
            // the chevron (5.5 in) and the first source row (top + 58). Each
            // is bound whole — a `Text` has no `dy`, only `y_var`
            m.insert("accent_scheme_region_y", SceneValue::Ring(top + 28.0));
            m.insert("accent_scheme_text_y", SceneValue::Ring(top + 34.5));
            m.insert("accent_chev_y", SceneValue::Ring(top + 33.5));
            m.insert("accent_rows_y", SceneValue::Ring(top + 58.0));
            m.insert(
                "accent_scheme",
                SceneValue::Text(format!("Scheme: {sname}")),
            );
            m.insert_color(
                "accent_scheme_ink",
                SceneColor::Token(if self.hover_key == 28 {
                    crate::ui::ColorToken::Acc
                } else {
                    crate::ui::ColorToken::Fg
                }),
            );
            m.insert("accent_picker", SceneValue::Toggle(!self.accent_list_open));
            // ("From wallpaper", "Scheme default", "Custom accent") — the
            // drawer's own order; the active row takes the 3 × 12 rail and the
            // custom row appends the live accent name
            let mut rows: Vec<crate::scene::SceneRow> = Vec::with_capacity(3);
            for (i, name) in ["From wallpaper", "Scheme default", "Custom accent"]
                .into_iter()
                .enumerate()
            {
                let active = match i {
                    0 => cur == "w",
                    1 => cur == "0",
                    _ => cur == "c",
                };
                let hov = self.hover_key == ACC_KEY_BASE + i as u32;
                let label = if i == 2 && active && !cur_name.is_empty() {
                    format!("{name} · {cur_name}")
                } else {
                    name.to_string()
                };
                rows.push(crate::scene::SceneRow {
                    // col 0 is the rail cell — the text is empty, the `dot`
                    // cell paints from the row's ink
                    cols: vec![String::new(), label],
                    key: 0,
                    action: None,
                    color: None,
                    col_colors: vec![
                        // the rail is `acc` only on the active row; `clear` on
                        // the others (a ColorToken has no transparent variant)
                        Some(if active {
                            crate::ui::ColorToken::Acc
                        } else {
                            crate::ui::ColorToken::Value(crate::ui::DynColor::named("clear"))
                        }),
                        Some(if hov {
                            crate::ui::ColorToken::Acc
                        } else {
                            crate::ui::ColorToken::Fg
                        }),
                    ],
                    surface: None,
                });
            }
            m.insert("accent_rows", SceneValue::Rows(rows));
        }
        // The clipboard image list is fully declarative (clipimg.ron): one
        // `Rows` row per paste, each an IMAGE cell (the renderer's key is the
        // cell text, over a rounded `hover` tile) plus the truncated name.
        // The selection and hover inks are resolved HERE rather than by a
        // per-column `hover` token, because the drawer ranks them — selected
        // beats hovered — while `col.hover` would beat `col_colors`.
        {
            let entries = self.clip_images.clone();
            let visible = self.clipimg_card_visible().min(entries.len());
            let start = self
                .clipimg_scroll
                .min(entries.len().saturating_sub(visible));
            let mut rows: Vec<crate::scene::SceneRow> = Vec::new();
            // ALL entries are published and the list windows itself on
            // `clipimg_start` — publishing only the visible window AND binding
            // the scroll would apply the offset twice. Each row's key is
            // window-RELATIVE (`key_base` + its index in the window), which is
            // what `input.rs` maps back to a visible slot.
            for i in 0..entries.len() {
                // a row ABOVE the window can never be the hovered one (it has
                // no visible slot to be under the cursor)
                let in_window = i >= start;
                let key = if in_window {
                    crate::shell::CLIPIMG_KEY_BASE + (i - start) as u32
                } else {
                    0
                };
                let is_sel = self.clip_sel == i;
                let is_hov = in_window && self.hover_key == key;
                // the drawer's own ink ladder: acc on the selection,
                // hover_fg under the cursor, fg otherwise
                let name_ink = if is_sel {
                    crate::ui::ColorToken::Acc
                } else if is_hov {
                    crate::ui::ColorToken::HoverFg
                } else {
                    crate::ui::ColorToken::Fg
                };
                rows.push(crate::scene::SceneRow {
                    cols: vec![entries[i].clone(), entries[i].chars().take(28).collect()],
                    key: 0,
                    action: None,
                    color: None,
                    col_colors: vec![None, Some(name_ink)],
                    // the band behind the selected row — the SAME `hover_hl`
                    // the hovered row gets, and `row.surface` outranks
                    // `hover_surface` so a selected+hovered row keeps it
                    surface: is_sel.then_some(crate::ui::ColorToken::HoverHl),
                });
            }
            m.insert("clipimg_rows", SceneValue::Rows(rows));
            // the scroll offset as the drawer's own clamped first visible row
            m.insert("clipimg_start", SceneValue::Ring(start as f32));
            m.insert("clipimg_empty", SceneValue::Toggle(entries.is_empty()));
        }
        // Power draw (powerdraw.ron): battery watts in accent and GPU watts
        // in info blue, as two `ui::pulse` traces over the same box, with the
        // last sample of each beside its B / G dot. The series are published
        // PRE-NORMALIZED against the drawer's own shared peak (a 1.0 floor) —
        // the `Spark` item divides by `max`, so `max: 1.0` here reproduces the
        // drawer's normalized data exactly.
        {
            let hist = self.pw_history.clone();
            let last = hist.back().copied().unwrap_or((0.0, 0.0));
            m.insert("pw_empty", SceneValue::Toggle(hist.is_empty()));
            // the info column and the dots only exist once a sample has
            // landed (the drawer returns before them on an empty history)
            m.insert("pw_live", SceneValue::Toggle(!hist.is_empty()));
            // EXACTLY one sample has no trace to draw — the drawer falls back
            // to a flat midline (the latency card's `Divider` idiom). Zero
            // samples is the `pw_empty` state instead, and the drawer returns
            // before any of it, so this must not cover the empty history.
            m.insert("pw_idle", SceneValue::Toggle(hist.len() == 1));
            // both traces need TWO samples; until then `pw_idle`'s flat
            // midline stands in — the names are conditionally published, and
            // the scene binds them on every frame
            m.declare_conditional("pw_b_series");
            m.declare_conditional("pw_g_series");
            if !hist.is_empty() {
                // battery is charging when its draw rose vs the previous sample
                let charging = hist.len() >= 2
                    && hist.back().map(|c| c.0).unwrap_or(0.0)
                        > hist.get(hist.len() - 2).map(|p| p.0).unwrap_or(0.0) + 0.05;
                let arrow = if charging { "↑" } else { "" };
                m.insert("pw_b", SceneValue::Text(format!("{:.1}W {arrow}B", last.0)));
                m.insert("pw_g", SceneValue::Text(format!("{:.1}W G", last.1)));
                if hist.len() >= 2 {
                    let peak = hist.iter().fold(1.0_f32, |m, &(b, g)| m.max(b).max(g));
                    m.insert(
                        "pw_b_series",
                        SceneValue::Spark(
                            hist.iter()
                                .map(|&(b, _)| (b / peak).clamp(0.0, 1.0))
                                .collect(),
                        ),
                    );
                    m.insert(
                        "pw_g_series",
                        SceneValue::Spark(
                            hist.iter()
                                .map(|&(_, g)| (g / peak).clamp(0.0, 1.0))
                                .collect(),
                        ),
                    );
                }
            }
        }
        // The system card is fully declarative (system.ron): the identity
        // block, the control stack, and the per-state inks the drawer
        // computed inline. Its strings are published already shortened (the
        // 12-char username, the width-capped hostname, the width-measured
        // date chip) and the two state-driven inks ride per-frame colors.
        {
            let pal = Pal {
                fg: self.sv_fg,
                bg: self.sv_bg,
                acc: self.sv_acc,
                sfg: self.sv_sfg,
            };
            let (_, _, cw, _) = self.sys_card_rect;
            let s = self.scale.s(1.0);
            m.insert(
                "sys_uname",
                SceneValue::Text(self.username.chars().take(12).collect()),
            );
            // the drawer's own cap: `floor(w / 8)` chars, never fewer than 8
            m.insert(
                "sys_hname",
                SceneValue::Text(
                    self.hostname
                        .chars()
                        .take((cw / 8.0).max(8.0) as usize)
                        .collect(),
                ),
            );
            m.insert("sys_clock", SceneValue::Text(self.clock.clone()));
            m.insert("sys_clock_on", SceneValue::Toggle(!self.clock.is_empty()));
            let dtext = self.date_str("%a, %d %b");
            m.insert("sys_date", SceneValue::Text(dtext.clone()));
            // the chip's width: `chars * 12 * 0.62 + 18`, which `w_var`
            // wants in BASE px
            m.insert(
                "sys_date_w",
                SceneValue::Ring(
                    (dtext.chars().count() as f32 * self.scale.s(12.0) * 0.62 + self.scale.s(18.0))
                        / s,
                ),
            );
            let date_hover = self.hover_key == 25;
            m.insert("sys_date_hover", SceneValue::Toggle(date_hover));
            m.insert("sys_date_idle", SceneValue::Toggle(!date_hover));
            m.insert_color(
                "sys_date_bg",
                SceneColor::Raw(if date_hover {
                    ui::hover_hl(&pal)
                } else {
                    mix(ui::hover(&pal), pal.fg, 0.06)
                }),
            );
            m.insert(
                "sys_uistate",
                SceneValue::Text(format!("UI State: {}", self.cur_channel)),
            );
            m.insert(
                "sys_uistate_on",
                SceneValue::Toggle(!self.cur_channel.is_empty()),
            );
            // the ⋮ button's fill exists while hovered OR open, and its ink is
            // hover_hl in the first case, a 20%-alpha accent wash in the second
            let menu_hover = self.hover_key == crate::shell::SYS_MENU_KEY;
            m.insert(
                "sys_menu_fill",
                SceneValue::Toggle(menu_hover || self.sys_menu_open),
            );
            m.insert_color(
                "sys_menu_fill_c",
                SceneColor::Raw(if menu_hover {
                    ui::hover_hl(&pal)
                } else {
                    (pal.acc & 0xFFFF_FF00) | 0x34
                }),
            );
            m.insert_color(
                "sys_dots_c",
                SceneColor::Raw(if menu_hover || self.sys_menu_open {
                    pal.acc
                } else {
                    ui::fg2(&pal)
                }),
            );
            m.insert(
                "sys_bell_glyph",
                SceneValue::Text(
                    if self.dnd {
                        crate::icons::ICON_DND
                    } else {
                        crate::icons::ICON_BELL
                    }
                    .to_string(),
                ),
            );
            m.insert(
                "sys_notif_badge",
                SceneValue::Toggle(self.notif_count > 0 && !self.dnd),
            );
            m.insert(
                "sys_mode_glyph",
                SceneValue::Text(
                    if self.dark_mode {
                        crate::icons::ICON_MOON
                    } else {
                        crate::icons::ICON_BRIGHTNESS
                    }
                    .to_string(),
                ),
            );
            m.insert(
                "sys_uptime",
                SceneValue::Text(format!("Uptime {}", self.uptime)),
            );
            let has_batt = self.battery >= 0;
            let batt_low = self.battery <= 20 && !self.ac_online;
            m.insert("sys_batt", SceneValue::Toggle(has_batt));
            m.insert(
                "sys_batt_glyph",
                SceneValue::Text(
                    crate::ui::battery_glyph(self.battery, self.ac_online).to_string(),
                ),
            );
            m.insert_color(
                "sys_batt_c",
                SceneColor::Raw(if batt_low { crate::ui::DANGER } else { pal.fg }),
            );
            // the % sits left of the glyph, clear of it: `x_var` is a BASE px
            // offset from the card's left edge
            // the % clears the glyph by its measured width plus 4 px, and a
            // right-anchored `Text` takes the distance from the card's RIGHT
            // edge (`s(16)` gutter + the glyph + the gap), so the binding is
            // width-independent like the drawer's own geometry
            let icon_w = crate::ui::battery_icon_w(self.scale.s(13.0));
            m.insert(
                "sys_batt_pct_x",
                SceneValue::Ring((self.scale.s(16.0) + icon_w + self.scale.s(4.0)) / s),
            );
            m.insert(
                "sys_batt_pct",
                SceneValue::Text(format!("{}%", self.battery)),
            );
            m.insert(
                "sys_batt_pct_on",
                SceneValue::Toggle(has_batt && self.pill_battery_pct),
            );
            m.insert("sys_batt_time", SceneValue::Text(self.battery_time.clone()));
            m.insert(
                "sys_batt_time_on",
                SceneValue::Toggle(has_batt && !self.battery_time.is_empty()),
            );
            m.insert(
                "sys_watts",
                SceneValue::Text(format!("{:.1}W", self.battery_watts)),
            );
            m.insert(
                "sys_watts_on",
                SceneValue::Toggle(has_batt && self.battery_watts > 0.0),
            );
        }
        // The weather card is fully declarative (weather.ron): pane 0 is the
        // current conditions, pane 1 the 7-day lookahead. Its arithmetic moves
        // to here because the drawer stands down — the centered glyph+temp
        // pair needs the measured temp width, the forecast's row COUNT and
        // centered TOP are height arithmetic, and the weekday labels are the
        // drawer's own libc rollover.
        {
            let (_, _, cw, ch) = self.weather_rect;
            let s = self.scale.s(1.0);
            let hover = !self.dash_edit
                && self
                    .cursor
                    .map(|(px, py)| Shell::in_rect(px, py, self.weather_rect))
                    .unwrap_or(false);
            m.insert("wx_hover", SceneValue::Toggle(hover));
            if self.weather.is_none() {
                // no data yet: the centered placeholder, no panes — and no
                // dots, because the drawer returns before `battery_dots`
                m.insert("wx_hover", SceneValue::Toggle(false));
                m.insert("wx_none", SceneValue::Toggle(true));
                m.insert("wx_pane_0", SceneValue::Toggle(false));
                m.insert("wx_pane_1", SceneValue::Toggle(false));
                m.insert("wx_squat", SceneValue::Toggle(false));
                m.insert("wx_pane", SceneValue::Ring(0.0));
                // every interpolation the panes read MUST still exist, or the
                // engine's `{name}` lookup falls back to the raw token
                m.insert("wx_glyph", SceneValue::Text(String::new()));
                m.insert("wx_desc", SceneValue::Text(String::new()));
                m.insert("wx_temp", SceneValue::Text(String::new()));
                m.insert("wx_temp_x", SceneValue::Ring(0.0));
                m.insert("wx_city_text", SceneValue::Text(String::new()));
                m.insert("wx_pane0_city", SceneValue::Toggle(false));
                m.insert("wx_detail_text", SceneValue::Text(String::new()));
                m.insert("wx_pane0_detail", SceneValue::Toggle(false));
                m.insert("wx_rows_y", SceneValue::Ring(0.0));
                m.insert("wx_fc_rows", SceneValue::Rows(Vec::new()));
            }
            // the OTHER cards' values keep publishing either way
            if let Some(wx) = self.weather.as_ref() {
                m.insert("wx_none", SceneValue::Toggle(false));
                // the drawer only shows the forecast pane when it HAS a forecast
                let pane1 = self.weather_pane == 1 && !wx.daily.is_empty();
                // the squat card (h < 60) is the one dense line INSTEAD of the
                // hero, so the hero pane gate has to know about it
                let squat = ch < self.scale.s(60.0);
                let pane0 = !pane1 && !squat;
                m.insert("wx_pane_0", SceneValue::Toggle(pane0));
                m.insert("wx_pane_1", SceneValue::Toggle(pane1));
                m.insert("wx_pane", SceneValue::Ring(if pane1 { 1.0 } else { 0.0 }));
                let (glyph, desc) = crate::weather::describe(wx.code);
                m.insert("wx_glyph", SceneValue::Text(glyph.to_string()));
                m.insert("wx_desc", SceneValue::Text(desc.to_string()));
                let temp = format!("{:.0}\u{b0}C", wx.temp_c);
                m.insert("wx_temp", SceneValue::Text(temp.clone()));
                // the drawer's centering of the glyph+temp PAIR: the temp's
                // left edge is the pair's center shifted by half the width gap
                let glyph_w = self.scale.fs(20.0) * 1.6;
                let temp_w = temp.chars().count() as f32 * self.scale.fs(20.0) * 0.55;
                m.insert(
                    "wx_temp_x",
                    SceneValue::Ring(
                        ((cw / 2.0 - (glyph_w + temp_w) / 2.0 + glyph_w) / s).max(0.0),
                    ),
                );
                // an EMPTY string is not enough to hide a Text (the engine still
                // registers the cell), so both the city and the detail line get
                // explicit gates — the drawer's own conditions
                m.insert("wx_city_text", SceneValue::Text(self.weather_city.clone()));
                let mut detail = String::new();
                if let Some(fl) = wx.feels_like {
                    detail.push_str(&format!("feels {:.0}\u{b0}", fl));
                }
                if wx.wind_kmh > 0.0 {
                    if !detail.is_empty() {
                        detail.push_str(" \u{b7} ");
                    }
                    detail.push_str(&format!("{:.0} km/h", wx.wind_kmh));
                }
                // the detail line also needs the drawer's height gate
                let detail_tall = ch >= self.scale.s(100.0);
                m.insert("wx_detail_text", SceneValue::Text(detail.clone()));
                // a Text item carries ONE gate, so the pane gate and each
                // line's own draw condition publish as one combined Toggle
                m.insert(
                    "wx_pane0_city",
                    SceneValue::Toggle(pane0 && !self.weather_city.is_empty()),
                );
                m.insert(
                    "wx_pane0_detail",
                    SceneValue::Toggle(pane0 && !detail.is_empty() && detail_tall),
                );
                // the squat card's one dense line (glyph + temp in the icon font)
                m.insert("wx_squat_text", SceneValue::Text(format!("{glyph} {temp}")));
                // the squat line is pane 0's only line: pane 1 returns before
                // the height check, so it must not bleed onto the forecast
                m.insert("wx_squat", SceneValue::Toggle(!pane1 && squat));
                // pane 1: the drawer's row count and centered top, in base px
                let row_h = self.scale.s(24.0);
                let n_fit = (((ch - self.scale.s(16.0)) / row_h).floor() as usize)
                    .min(7)
                    .max(1);
                let top = self.scale.s(8.0).max((ch - n_fit as f32 * row_h) / 2.0);
                m.insert("wx_rows_y", SceneValue::Ring(top / s));
                let rows: Vec<SceneRow> = wx
                    .daily
                    .iter()
                    .take(n_fit)
                    .map(|d| {
                        let (g, _) = crate::weather::describe(d.code);
                        let rain = if d.precip_prob >= 5.0 {
                            format!("{}%", d.precip_prob.round() as i32)
                        } else {
                            String::new()
                        };
                        SceneRow {
                            cols: vec![
                                g.to_string(),
                                self.weekday_name(d.day_offset as i32),
                                rain,
                                format!("{:.0}\u{b0} {:.0}\u{b0}", d.tmin_c, d.tmax_c),
                            ],
                            key: 0,
                            action: None,
                            color: None,
                            surface: None,
                            col_colors: vec![],
                        }
                    })
                    .collect();
                m.insert("wx_fc_rows", SceneValue::Rows(rows));
            }
        }
        // The visualizer is fully declarative (viz.ron): three `Spectrum`
        // items over one series, gated on the `viz_style` cycle, plus the
        // silent caption. The series is clamped to the card's own `n` and
        // ZERO-PADDED here, because a `Spectrum` bar's pitch is `plot_w / n`
        // (not a line trace's `w / (n-1)`) and the count of bars IS the
        // series length — the drawer reads `viz.get(i).copied().unwrap_or(0)`
        // for the tail, so a short capture would re-pitch the whole grid.
        {
            let n = self.viz.len().min(24);
            m.insert(
                "viz_series",
                SceneValue::Spark(
                    (0..n)
                        .map(|i| self.viz.get(i).copied().unwrap_or(0.0))
                        .collect(),
                ),
            );
            let style = self.viz_style;
            m.insert("viz_bars", SceneValue::Toggle(style != 1 && style != 2));
            m.insert("viz_block", SceneValue::Toggle(style == 2));
            m.insert("viz_wave", SceneValue::Toggle(style == 1));
            m.insert(
                "viz_silent",
                SceneValue::Toggle(self.viz.iter().take(24).all(|b| *b < 0.04)),
            );
        }
        // The media card is fully declarative (media.ron): the album art (an
        // `Image` when MPRIS gave us a path, a rounded note glyph when not),
        // the title / album+artist pair, the seek bar with its time labels, and
        // the three transport buttons. Everything the drawer computed inline
        // is published here, INCLUDING the two width/height gates and the
        // drawer's own `fit()` truncation — a `Text` cell's char cap is a
        // static reserve, and this cap moves with the card box.
        {
            let pal = Pal {
                fg: self.sv_fg,
                bg: self.sv_bg,
                acc: self.sv_acc,
                sfg: self.sv_sfg,
            };
            // the drawer's own arithmetic, in the SAME px it used: the `wide`
            // and `has_room` gates are device-px breakpoints and the `fit` cap
            // is a device-px width, so comparing base px here would move both
            // at scales != 1 and the card would not match its fallback. Only
            // the values bound to `x_var` (BASE px, per the item contract) are
            // converted.
            let (_, _, w, h) = self.media_card_rect;
            let wide = w >= 210.0;
            let info_tx = if wide { 72.0 } else { 14.0 };
            let avail = (w - info_tx - 14.0).max(1.0);
            // the drawer's `fit`: floor(avail / (size · 0.62)), floored at 4
            // chars, and an ellipsis when it clips
            let fit = |s: &str, size: f32| -> String {
                let n = (avail / (size * 0.62)).floor().max(4.0) as usize;
                if s.chars().count() > n {
                    let mut t: String = s.chars().take(n.saturating_sub(1)).collect();
                    t.push('…');
                    t
                } else {
                    s.to_string()
                }
            };
            let (mtitle, martist, malbum) = if self.media_title.is_empty() {
                ("Nothing playing".to_string(), String::new(), String::new())
            } else {
                (
                    self.media_title.clone(),
                    self.media_artist.clone(),
                    self.media_album.clone(),
                )
            };
            let line2 = if !malbum.is_empty() {
                format!(
                    "{malbum}{}",
                    if !martist.is_empty() {
                        format!(" — {martist}")
                    } else {
                        String::new()
                    }
                )
            } else {
                martist.clone()
            };
            // one toggle per DRAWN variant, since `visible` binds a single
            // name: the info column's two x positions (right of the 48 px art
            // when wide, under it when not) and the line2's emptiness folded in
            let has2 = !line2.is_empty();
            m.insert("media_title_wide", SceneValue::Toggle(wide));
            m.insert("media_title_narrow", SceneValue::Toggle(!wide));
            m.insert("media_line2_wide", SceneValue::Toggle(wide && has2));
            m.insert("media_line2_narrow", SceneValue::Toggle(!wide && has2));
            m.insert("media_has_room", SceneValue::Toggle(h >= 120.0));
            m.insert("media_title", SceneValue::Text(fit(&mtitle, 12.0)));
            m.insert("media_line2", SceneValue::Text(fit(&line2, 9.5)));
            // `file:` key or empty — an empty key paints nothing, which is how
            // the placeholder takes the art's box
            m.insert(
                "media_art_key",
                SceneValue::Text(if self.media_art.is_empty() {
                    String::new()
                } else {
                    format!("file:{}", self.media_art)
                }),
            );
            m.insert(
                "media_art_empty",
                SceneValue::Toggle(self.media_art.is_empty()),
            );
            m.insert_color(
                "media_ph_ink",
                SceneColor::Raw(crate::ui::mix(crate::ui::hover(&pal), pal.fg, 0.06)),
            );
            // seek: the fraction, the hover-dependent track and the time labels
            let frac = self.media_seek.unwrap_or_else(|| self.media_frac());
            let seek_hov = self.hover_key == 23;
            m.insert("media_frac", SceneValue::Text(frac.to_string()));
            m.insert_color(
                "media_track_ink",
                SceneColor::Raw(crate::ui::mix(
                    pal.bg,
                    pal.fg,
                    if seek_hov { 0.16 } else { 0.10 },
                )),
            );
            m.insert_color(
                "media_fill_ink",
                SceneColor::Token(crate::ui::ColorToken::Acc),
            );
            let cur = self
                .media_seek
                .map(|f| (f * self.media_len.max(1) as f32) as i64)
                .unwrap_or_else(|| self.media_pos_now());
            m.insert("media_pos", SceneValue::Text(crate::shell::fmt_time(cur)));
            m.insert(
                "media_dur",
                SceneValue::Text(crate::shell::fmt_time(self.media_len)),
            );
            // transport: the play/pause glyph and the two side buttons' inks
            // (hover → accent). The PLAY button is accent either way — the
            // drawer passes `pal.acc` as its resting color too — so it needs
            // no published ink at all.
            m.insert(
                "media_play_glyph",
                SceneValue::Text(if self.media_playing {
                    ICON_PAUSE.into()
                } else {
                    ICON_PLAY.into()
                }),
            );
            m.insert("media_prev_glyph", SceneValue::Text(ICON_PREV.into()));
            m.insert("media_next_glyph", SceneValue::Text(ICON_SKIP_NEXT.into()));
            m.insert_color(
                "media_prev_ink",
                SceneColor::Token(if self.hover_key == 20 {
                    crate::ui::ColorToken::Acc
                } else {
                    crate::ui::ColorToken::Fg
                }),
            );
            m.insert_color(
                "media_next_ink",
                SceneColor::Token(if self.hover_key == 22 {
                    crate::ui::ColorToken::Acc
                } else {
                    crate::ui::ColorToken::Fg
                }),
            );
            // the three button hit boxes straddle the card's center line
            let bw = self.scale.base(w);
            m.insert("media_prev_hx", SceneValue::Ring(bw / 2.0 - 50.0));
            m.insert("media_play_hx", SceneValue::Ring(bw / 2.0 - 22.0));
            m.insert("media_next_hx", SceneValue::Ring(bw / 2.0 + 12.0));
            // the glyphs are left-anchored in the ICON face, not centered: the
            // drawer's `text(.., icon = true)` at `btn_cx - 36 / + 36`
            m.insert("media_prev_gx", SceneValue::Ring(bw / 2.0 - 36.0));
            m.insert("media_play_gx", SceneValue::Ring(bw / 2.0));
            m.insert("media_next_gx", SceneValue::Ring(bw / 2.0 + 36.0));
        }
        // The latency card is fully declarative (latency.ron): the probe
        // series (pre-normalized against the drawer's 120 ms peak floor), the
        // right-anchored hero ms number + its ink, the status word, and the
        // `lat_idle` gate that keeps the flat trace on screen until two probes
        // land. The re-probe pill's hover ink is a `Hit` surface_hover, so
        // nothing about the pointer is published here.
        {
            let (val, ink) = match self.lat_state {
                0 => ("--".to_string(), None),
                1 => ("…".to_string(), None),
                3 => ("ERR".to_string(), Some(crate::ui::DANGER)),
                _ => {
                    let cur = self.lat_current;
                    let c = if cur < 60 {
                        Some(crate::ui::OK)
                    } else if cur < 120 {
                        Some(crate::ui::WARN)
                    } else {
                        Some(crate::ui::DANGER)
                    };
                    (format!("{cur}"), c)
                }
            };
            m.insert("lat_val", SceneValue::Text(val));
            m.insert_color(
                "lat_val_ink",
                match ink {
                    Some(c) => SceneColor::Raw(c),
                    None => SceneColor::Token(ColorToken::Fg3),
                },
            );
            let status = match self.lat_state {
                0 => "idle",
                1 => "probing…",
                3 => "no reply",
                _ => {
                    if self.lat_current < 60 {
                        "smooth"
                    } else if self.lat_current < 120 {
                        "slow"
                    } else {
                        "bad"
                    }
                }
            };
            m.insert("lat_status", SceneValue::Text(status.into()));
            // the drawer's own normalization: a 120 ms peak floor, clamped
            let series: Vec<f32> = self.lat_history.iter().map(|&ms| ms as f32).collect();
            let norm = if series.len() >= 2 {
                let peak = series.iter().fold(120.0_f32, |a, &b| a.max(b)).max(1.0);
                series.iter().map(|&v| (v / peak).clamp(0.0, 1.0)).collect()
            } else {
                Vec::new()
            };
            m.insert("lat_spark", SceneValue::Spark(norm));
            m.insert("lat_idle", SceneValue::Toggle(series.len() < 2));
        }
        // The wifi card is fully declarative (wifi.ron): the count meta, the
        // list (per-row: the signal staircase value, the SSID, the padlock
        // glyph, the ✓ / … state), the scroll window, and the two empty
        // captions. The connected row's wash rides `row.surface`; the SSID and
        // the ✓ take their ink from `col_colors` exactly like the drawer.
        {
            let nets = self.wifi_networks.clone();
            m.insert(
                "wifi_meta",
                SceneValue::Text(format!("{} nets", nets.len())),
            );
            m.insert(
                "wifi_empty_on",
                SceneValue::Toggle(nets.is_empty() && self.wifi_on),
            );
            m.insert(
                "wifi_off",
                SceneValue::Toggle(nets.is_empty() && !self.wifi_on),
            );
            m.insert(
                "wifi_scroll",
                SceneValue::Ring(self.wifi_scroll as i64 as f32),
            );
            let pending = self.pending_wifi.clone();
            // the drawer's wash chain is `connected → acc_tint, else hovered →
            // hover`, and a hit key is a WINDOW index (input.rs resolves
            // `top + j`), so the hovered data row is the one at `top + j`
            let top = self
                .wifi_scroll
                .min(nets.len().saturating_sub(self.wifi_card_visible()));
            let hov_i = self
                .hover_key
                .checked_sub(crate::shell::WIFI_KEY_BASE)
                .filter(|j| *j < 100)
                .map(|j| top + j as usize);
            m.insert(
                "wifi_rows",
                SceneValue::Rows(
                    nets.iter()
                        .enumerate()
                        .map(|(d, (ssid, strength, secured, connected))| {
                            let connecting = pending.as_deref() == Some(ssid.as_str());
                            let ssid: String = ssid.chars().take(22).collect();
                            crate::scene::SceneRow {
                                cols: vec![
                                    strength.to_string(),
                                    ssid,
                                    if *secured {
                                        ICON_LOCK.to_string()
                                    } else {
                                        String::new()
                                    },
                                    if *connected {
                                        ICON_CHECK.to_string()
                                    } else {
                                        String::new()
                                    },
                                    if connecting {
                                        "…".to_string()
                                    } else {
                                        String::new()
                                    },
                                ],
                                key: 0,
                                action: None,
                                color: None,
                                surface: if *connected {
                                    Some(crate::ui::ColorToken::AccTint)
                                } else if hov_i == Some(d) {
                                    Some(crate::ui::ColorToken::Hover)
                                } else {
                                    None
                                },
                                col_colors: vec![
                                    None,
                                    Some(if *connected {
                                        crate::ui::ColorToken::Acc
                                    } else {
                                        crate::ui::ColorToken::Fg
                                    }),
                                    None,
                                    Some(if *connected {
                                        crate::ui::ColorToken::Acc
                                    } else {
                                        crate::ui::ColorToken::Fg3
                                    }),
                                    Some(crate::ui::ColorToken::Fg3),
                                ],
                            }
                        })
                        .collect(),
                ),
            );
        }
        // The quote card is fully declarative (quote.ron): the three-state
        // header meta, the wrapped body + its two swap captions, and the pill
        // pair whose save fill/ink/region follow whether a quote is loaded.
        let quote_has = self.quote_state == 2 && !self.quote_text.is_empty();
        let (qmeta, qmeta_ink) = if self.quote_state == 2 && self.quote_saved_flash > 0 {
            (format!("{} saved", ICON_CHECK), SceneColor::Raw(0x2ee6a8ff))
        } else {
            match self.quote_state {
                1 => (
                    "fetching…".to_string(),
                    SceneColor::Token(crate::ui::ColorToken::Fg3),
                ),
                2 => (
                    format!("{} saved", self.saved_quotes.len()),
                    SceneColor::Token(crate::ui::ColorToken::Fg3),
                ),
                _ => (
                    "—".to_string(),
                    SceneColor::Token(crate::ui::ColorToken::Fg3),
                ),
            }
        };
        m.insert("quote_meta", SceneValue::Text(qmeta));
        m.insert_color("quote_meta_ink", qmeta_ink);
        m.insert("quote_text", SceneValue::Text(self.quote_text.clone()));
        m.insert("quote_author", SceneValue::Text(self.quote_author.clone()));
        m.insert("quote_has", SceneValue::Toggle(quote_has));
        m.insert("quote_fetching", SceneValue::Toggle(self.quote_state == 1));
        m.insert(
            "quote_empty",
            SceneValue::Toggle(!quote_has && self.quote_state != 1),
        );
        let p = &pal;
        // the refresh pill's fill + label ink follow hover (the drawer's
        // `mix(acc, fg, .15)` lift and accent label), the save pill's follow
        // whether a quote is loaded — so both are published per frame
        let new_hov = self.hover_key == crate::shell::QUOTE_KEY_BASE;
        let save_hov = self.hover_key == crate::shell::QUOTE_SAVE_KEY;
        m.insert_color(
            "quote_new_fill",
            SceneColor::Raw(if new_hov {
                crate::ui::mix(p.acc, p.fg, 0.15)
            } else {
                crate::ui::hover_hl(p)
            }),
        );
        m.insert_color(
            "quote_new_ink",
            SceneColor::Token(if new_hov {
                crate::ui::ColorToken::Acc
            } else {
                crate::ui::ColorToken::Fg
            }),
        );
        m.insert_color(
            "quote_save_fill",
            SceneColor::Raw(if quote_has {
                if save_hov {
                    crate::ui::mix(0x2ee6a8ff, p.fg, 0.15)
                } else {
                    0x2ee6a8ff
                }
            } else if save_hov {
                crate::ui::hover_hl(p)
            } else {
                crate::ui::hover(p)
            }),
        );
        m.insert_color(
            "quote_save_ink",
            SceneColor::Raw(if quote_has {
                0xff141414
            } else {
                crate::ui::fg3(p)
            }),
        );
        // The alarms card is fully declarative (alarms.ron): the header meta,
        // the composer well (fill + label ink follow focus/emptiness) and the
        // list, whose ✕ keys are handled scene-side via
        // `Rows { del_base: ALARM_DEL_BASE }`.
        m.insert(
            "alarm_meta",
            SceneValue::Text(if self.alarm_list.is_empty() {
                String::new()
            } else {
                format!("{} set", self.alarm_list.len())
            }),
        );
        let composing = self.alarm_input.is_some();
        let buf = self.alarm_input.clone().unwrap_or_default();
        m.insert_color(
            "alarm_fill",
            SceneColor::Token(if composing {
                crate::ui::ColorToken::HoverHl
            } else {
                crate::ui::ColorToken::Hover
            }),
        );
        m.insert_color(
            "alarm_ink",
            SceneColor::Token(if buf.is_empty() {
                crate::ui::ColorToken::Fg3
            } else {
                crate::ui::ColorToken::Fg
            }),
        );
        m.insert(
            "alarm_input",
            SceneValue::Text(if buf.is_empty() {
                "add alarm — 07:30 stand up".to_string()
            } else {
                buf
            }),
        );
        m.insert(
            "alarm_rows",
            SceneValue::Rows(
                // capped at the ✕ key range: the delete handler covers
                // `ALARM_DEL_BASE..=ALARM_DEL_MAX`, so a 17th row would publish
                // a live region whose key nothing answers — and `input.rs`
                // deletes by visible index, which equals the list index here
                // (the alarms list never scrolls)
                self.alarm_list
                    .iter()
                    .take((crate::shell::ALARM_DEL_MAX - crate::shell::ALARM_DEL_BASE + 1) as usize)
                    .map(|(t, l)| crate::scene::SceneRow {
                        cols: vec![t.clone(), l.clone()],
                        key: 0,
                        action: None,
                        color: None,
                        surface: None,
                        col_colors: vec![],
                    })
                    .collect(),
            ),
        );
        // Rows: the pinned-city list — day/night glyph · 14-char city · mono
        // HH:MM · the `+5:45` offset-vs-local chip (empty, and so undrawn, for
        // the local zone). The offset cache and the shifted hour are the same
        // calls the drawer made; the `+ add city` row below the list and the
        // inline search stay in the `worldclock` Ink.
        {
            // the frame's ONE clock, sampled by the layout path — not here,
            // or the scene values and the drawer would read two instants
            let now = self.worldclock_epoch;
            let local_off = Shell::local_utc_offset_min(now);
            m.insert(
                "worldclock_rows",
                SceneValue::Rows(
                    self.worldclock_zones
                        .iter()
                        .map(|(city, _, off, _)| {
                            let day = (6..=17).contains(&Shell::shifted_hour(now, *off));
                            let d = *off - local_off;
                            let chip = if d == 0 {
                                String::new()
                            } else {
                                format!(
                                    "{}{}",
                                    if d > 0 { "+" } else { "" },
                                    cards::fmt_hhmm_min(d)
                                )
                            };
                            crate::scene::SceneRow {
                                cols: vec![
                                    if day {
                                        crate::icons::ICON_SUNNY.to_string()
                                    } else {
                                        crate::icons::ICON_MOON.to_string()
                                    },
                                    city.clone(),
                                    worldmap::world_hhmm(now, *off),
                                    chip,
                                ],
                                key: 0,
                                action: None,
                                color: None,
                                surface: None,
                                col_colors: vec![
                                    Some(if day {
                                        crate::ui::ColorToken::Warn
                                    } else {
                                        crate::ui::ColorToken::Fg3
                                    }),
                                    None,
                                    None,
                                    Some(crate::ui::ColorToken::Fg3),
                                ],
                            }
                        })
                        .collect(),
                ),
            );
            // ── the `+ add city` row ── its top is `y0 + min(n, max_rows−1)·row_h`
            // (the list's last free slot) and it only exists while that row fits
            // inside the card, so BOTH ride a bound y and a gate
            let (_, _, _, wc_h) = self.worldclock_rect;
            let row_h = self.scale.s(26.0);
            let y0 = self.scale.s(28.0);
            let max_rows = (((wc_h - self.scale.s(8.0)) - y0) / row_h).floor().max(0.0) as usize;
            let n = self.worldclock_zones.len();
            let add_y = y0 + n.min(max_rows.saturating_sub(1)) as f32 * row_h;
            m.insert(
                "wc_idle",
                SceneValue::Toggle(self.worldclock_search.is_none()),
            );
            m.insert(
                "wc_add_on",
                SceneValue::Toggle(self.worldclock_search.is_none() && add_y + row_h <= wc_h - 2.0),
            );
            m.insert("wc_add_y", SceneValue::Ring(self.scale.base(add_y)));
            let wc_pal = Pal {
                fg: self.sv_fg,
                bg: self.sv_bg,
                acc: self.sv_acc,
                sfg: self.sv_sfg,
            };
            m.insert_color(
                "wc_add_ink",
                SceneColor::Raw(if self.hover_key == crate::shell::WORLDCLOCK_ADD_KEY {
                    wc_pal.acc
                } else {
                    ui::fg3(&wc_pal)
                }),
            );
            // ── the inline search ── the input plate, the buffer (dim
            // until something is typed), the `esc` hint, the matches and the
            // empty-result caption. The match list is a `Rows` over the SAME
            // pure `worldclock_matches` the click handler resolves keys
            // through, so a row and its key cannot drift apart.
            let searching = self.worldclock_search.is_some();
            let buf = self.worldclock_search.clone().unwrap_or_default();
            let matches = self.worldclock_matches();
            m.insert("wc_srch", SceneValue::Toggle(searching));
            m.insert(
                "wc_buf",
                SceneValue::Text(if buf.is_empty() {
                    "type a city…".into()
                } else {
                    buf.clone()
                }),
            );
            m.insert_color(
                "wc_buf_ink",
                SceneColor::Raw(if buf.is_empty() {
                    ui::fg3(&wc_pal)
                } else {
                    wc_pal.fg
                }),
            );
            m.insert("wc_two", SceneValue::Toggle(buf.len() >= 2));
            m.insert(
                "wc_none",
                SceneValue::Toggle(buf.len() >= 2 && matches.is_empty()),
            );
            m.insert(
                "wc_matches",
                SceneValue::Rows(
                    matches
                        .iter()
                        .map(|(key, tz, city)| crate::scene::SceneRow {
                            cols: vec![
                                city.chars().take(16).collect(),
                                tz.chars().take(20).collect(),
                            ],
                            key: *key,
                            action: None,
                            color: None,
                            surface: None,
                            col_colors: vec![],
                        })
                        .collect(),
                ),
            );
        }
        // Rows: the notes list — bullet glyph · title · body-caption columns,
        // scroll window, and a `visible` toggle that hides the list while the
        // shared composer is open (it draws in place of the list).
        m.insert(
            "notes_rows",
            SceneValue::Rows(
                self.notes
                    .iter()
                    .map(|(title, body)| crate::scene::SceneRow {
                        cols: vec![
                            crate::icons::ICON_BULLET.to_string(),
                            title.clone(),
                            body.clone(),
                        ],
                        key: 0,
                        action: None,
                        color: None,
                        surface: None,
                        col_colors: vec![],
                    })
                    .collect(),
            ),
        );
        m.insert(
            "notes_composing",
            SceneValue::Toggle(self.notes_input.is_some()),
        );
        m.insert("notes_scroll", SceneValue::Ring(self.notes_scroll as f32));
        // Countdown chrome + composer (the card is zero-`Ink` since 2026-09-27):
        // the header meta counts the upcoming events, the composer's fill lifts
        // while it has focus, and its buffer shows the placeholder hint (dim)
        // until something is typed.
        let cd_upcoming = self
            .countdown_events
            .iter()
            .filter(|(_, d)| self.countdown_days(d) >= 0)
            .count();
        m.insert(
            "countdown_meta",
            SceneValue::Text(if self.countdown_events.is_empty() {
                String::new()
            } else {
                format!("{cd_upcoming} upcoming")
            }),
        );
        let cd_buf = self.countdown_input.clone().unwrap_or_default();
        m.insert(
            "countdown_shown",
            SceneValue::Text(if cd_buf.is_empty() {
                "add event — name 2026-12-31".to_string()
            } else {
                cd_buf.clone()
            }),
        );
        m.insert_color(
            "countdown_shown_ink",
            SceneColor::Token(if cd_buf.is_empty() {
                ColorToken::Fg3
            } else {
                ColorToken::Fg
            }),
        );
        m.insert_color(
            "countdown_input_bg",
            SceneColor::Token(if self.countdown_input.is_some() {
                ColorToken::HoverHl
            } else {
                ColorToken::Hover
            }),
        );
        // Rows: countdown events — label (reserves 120 for the chip area) ·
        // date caption · right `Nd`/`today!`/`passed` chip (per-row text token
        // drives both the chip ink and its derived fill); the del ✕ anchors
        // left of the chip via the pill col's `del_anchor`.
        m.insert(
            "countdown_rows",
            SceneValue::Rows(
                self.countdown_events
                    .iter()
                    .map(|(label, date)| {
                        let days = self.countdown_days(date);
                        let passed = days < 0;
                        let is_today = days == 0;
                        let chip = if is_today {
                            "today!".to_string()
                        } else if passed {
                            "passed".to_string()
                        } else if days == 1 {
                            "1d".to_string()
                        } else {
                            format!("{days}d")
                        };
                        let chip_c = if is_today {
                            crate::ui::ColorToken::Acc
                        } else if passed {
                            crate::ui::ColorToken::Fg3
                        } else if days <= 7 {
                            crate::ui::ColorToken::Warn
                        } else {
                            crate::ui::ColorToken::Fg2
                        };
                        crate::scene::SceneRow {
                            cols: vec![label.clone(), date.clone(), chip],
                            key: 0,
                            action: None,
                            color: None,
                            surface: None,
                            col_colors: vec![
                                if passed {
                                    Some(crate::ui::ColorToken::Fg3)
                                } else {
                                    None
                                },
                                Some(crate::ui::ColorToken::Fg3),
                                Some(chip_c),
                            ],
                        }
                    })
                    .collect(),
            ),
        );
        // TextWrap: quote body text + author + has_quote toggle — the wrap
        // block draws when `quote_has` is true; Rust keeps the state-fallback
        // messages ("fetching a quote…"/"no quote yet") and the pills/header.
        let has_quote = self.quote_state == 2 && !self.quote_text.is_empty();
        m.insert("quote_text", SceneValue::Text(self.quote_text.clone()));
        m.insert("quote_author", SceneValue::Text(self.quote_author.clone()));
        m.insert("quote_has", SceneValue::Toggle(has_quote));
        // Expenses: fully declarative scene — header MTD total (accent mono
        // meta), quick chip + composer rows, and the top-3 per-category bars
        // (caption, ratio bar, right mono amount). `exp_has_*` gates each bar
        // group against the category existing, `exp_empty` swaps the caption.
        let (exp_total, exp_per) = self.expense_month_summary();
        let exp_max = exp_per.first().map(|(_, v)| *v).unwrap_or(0.0).max(1.0);
        m.insert("exp_meta", SceneValue::Text(format!("MTD {exp_total:.0}")));
        m.insert(
            "exp_buf",
            SceneValue::Text(self.expense_input.clone().unwrap_or_default()),
        );
        m.insert(
            "exp_focus",
            SceneValue::Toggle(self.expense_input.is_some()),
        );
        m.insert("exp_empty", SceneValue::Toggle(exp_per.is_empty()));
        for i in 0..3 {
            let has = exp_per.len() > i;
            let (cat, val) = exp_per.get(i).cloned().unwrap_or_default();
            let name: String = cat.chars().take(7).collect();
            m.insert(format!("exp_has_{}", i + 1), SceneValue::Toggle(has));
            m.insert(format!("exp_cat_{}", i + 1), SceneValue::Text(name));
            m.insert(
                format!("exp_val_{}", i + 1),
                SceneValue::Text(format!("{val:.0}")),
            );
            m.insert(
                format!("exp_bar_{}", i + 1),
                SceneValue::Text(format!("{:.4}", (val / exp_max).clamp(0.0, 1.0))),
            );
        }
        // News: fully declarative — Header label (`{n} stories` or the active
        // category, empty when inert), the horizontal category `Strip`
        // (chips + active index + h-scroll), the filtered headline `Rows`
        // (title / open-glyph / source), and the centered empty/fetching
        // message. No Ink: the scene owns the whole card.
        let news_cats = self.news_cats();
        let news_label = if !self.news_items.is_empty() && news_cats.len() > 1 {
            if self.news_active_cat == 0 {
                format!("{} stories", self.news_filtered_len())
            } else {
                news_cats
                    .get(self.news_active_cat)
                    .cloned()
                    .unwrap_or_default()
            }
        } else {
            String::new()
        };
        m.insert("news_label", SceneValue::Text(news_label));
        m.insert(
            "news_msg",
            SceneValue::Text(if self.news_fetched {
                "no feeds — set [[news_feeds]]/news_url or write ~/.cache/zen-shell/news.json"
                    .to_string()
            } else {
                "fetching headlines…".to_string()
            }),
        );
        m.insert("news_chips", SceneValue::Chips(news_cats.clone()));
        m.insert(
            "news_cat_sel",
            SceneValue::Ring(self.news_active_cat as f32),
        );
        m.insert("news_cat_scroll", SceneValue::Ring(self.news_cat_scroll));
        m.insert("news_empty", SceneValue::Toggle(self.news_items.is_empty()));
        m.insert("news_scroll", SceneValue::Ring(self.news_scroll as f32));
        m.insert(
            "news_rows",
            SceneValue::Rows(
                self.news_filtered_idx()
                    .iter()
                    .filter_map(|&gi| self.news_items.get(gi))
                    .map(|item| crate::scene::SceneRow {
                        cols: vec![
                            item.title.chars().take(40).collect(),
                            if item.url.trim().is_empty() {
                                String::new()
                            } else {
                                crate::icons::ICON_OPEN.to_string()
                            },
                            item.source.chars().take(16).collect(),
                        ],
                        key: 0,
                        action: None,
                        color: None,
                        surface: None,
                        col_colors: vec![],
                    })
                    .collect(),
            ),
        );
        // Snippets: fully declarative — Header (`{n} clips`), the Composer
        // field (buffer or the placeholder; focused tint when active), and
        // click-to-copy rows (copy glyph · name · dim body preview) with the
        // 1 s "just copied" flash (`snip_flash`, scalar index + 1/0 = none)
        // and the hover ✕ delete. Keys route through the resident handler.
        let snip_n = self.snippets.len();
        let snip_flash_secs = self
            .snippet_copied_at
            .map(|t| t.elapsed().as_secs() < 1)
            .unwrap_or(false);
        m.insert(
            "snip_meta",
            SceneValue::Text(if snip_n > 0 {
                format!("{snip_n} clips")
            } else {
                String::new()
            }),
        );
        m.insert(
            "snip_buf",
            SceneValue::Text(self.snippet_input.clone().unwrap_or_default()),
        );
        m.insert(
            "snip_focus",
            SceneValue::Toggle(self.snippet_input.is_some()),
        );
        m.insert(
            "snip_flash",
            SceneValue::Ring(
                if snip_flash_secs {
                    self.snippet_copied.map(|i| i as f32 + 1.0)
                } else {
                    None
                }
                .unwrap_or(0.0),
            ),
        );
        m.insert(
            "snip_rows",
            SceneValue::Rows(
                self.snippets
                    .iter()
                    .map(|(name, body)| crate::scene::SceneRow {
                        cols: vec![
                            crate::icons::ICON_SELECT.to_string(),
                            name.chars().take(12).collect(),
                            body.chars().take(80).collect(),
                        ],
                        key: 0,
                        action: None,
                        color: None,
                        surface: None,
                        col_colors: vec![],
                    })
                    .collect(),
            ),
        );
        // Spark: CPU/GPU/latency histories are already 0..100 fractions.
        m.insert(
            "cpu_spark",
            SceneValue::Spark(self.cpu_history.iter().copied().collect()),
        );
        m.insert(
            "gpu_spark",
            SceneValue::Spark(self.gpu_history.iter().copied().collect()),
        );
        m.insert(
            "lat_spark",
            SceneValue::Spark(self.lat_history.iter().map(|&x| x as f32).collect()),
        );
        // net/disk histories are (bytes read, bytes written) pairs — expose a
        // normalized 0..1 series against the deque's own peak so a Spark
        // without an explicit max just works.
        let mut net_rows: Vec<(f32, f32)> = self
            .net_history
            .iter()
            .map(|&(d, u)| (d as f32, u as f32))
            .collect();
        let net_peak = peak_of_pairs(&net_rows);
        for (d, u) in net_rows.iter_mut() {
            *d /= net_peak;
            *u /= net_peak;
        }
        m.insert(
            "net_up_spark",
            SceneValue::Spark(net_rows.iter().map(|&(_, u)| u).collect()),
        );
        m.insert(
            "net_down_spark",
            SceneValue::Spark(net_rows.iter().map(|&(d, _)| d).collect()),
        );
        // the network graph's y-axis caption (raw bytes → bits/s, the Rust
        // drawer's exact ladder) — empty until the history has enough samples
        let net_bits = (net_peak.max(1.0) as u64).saturating_mul(8);
        m.insert(
            "net_meta",
            SceneValue::Text(if self.net_history.len() >= 2 {
                if net_bits >= 1_000_000 {
                    format!("{:.1} Mbps", net_bits as f64 / 1_000_000.0)
                } else if net_bits >= 1_000 {
                    format!("{:.0} Kbps", net_bits as f64 / 1_000.0)
                } else {
                    format!("{net_bits} bps")
                }
            } else {
                String::new()
            }),
        );
        let mut disk_rows: Vec<(f32, f32)> = self
            .disk_history
            .iter()
            .map(|&(r, w)| (r as f32, w as f32))
            .collect();
        let disk_peak = peak_of_pairs(&disk_rows);
        for (r, w) in disk_rows.iter_mut() {
            *r /= disk_peak;
            *w /= disk_peak;
        }
        m.insert(
            "disk_r_spark",
            SceneValue::Spark(disk_rows.iter().map(|&(r, _)| r).collect()),
        );
        m.insert(
            "disk_w_spark",
            SceneValue::Spark(disk_rows.iter().map(|&(_, w)| w).collect()),
        );
        // Rows: text-list cards — one data row per line / entry.
        m.insert(
            "jr_rows",
            SceneValue::Rows(
                self.jr_lines
                    .iter()
                    .map(|(msg, prio)| {
                        let color = Some(match *prio {
                            0..=3 => crate::ui::ColorToken::Danger,
                            4 => crate::ui::ColorToken::Warn,
                            _ => crate::ui::ColorToken::Fg2,
                        });
                        SceneRow {
                            cols: vec![msg.clone()],
                            key: 0,
                            action: None,
                            color,
                            col_colors: vec![],
                            surface: None,
                        }
                    })
                    .collect(),
            ),
        );
        // the journaltail CARD shows the newest-first tail (last 5, reversed)
        // — a separate binding so the full `jr_rows` list keeps its order
        m.insert(
            "jr_tail_rows",
            SceneValue::Rows(
                self.jr_lines
                    .iter()
                    .rev()
                    .take(5)
                    .map(|(msg, prio)| {
                        let color = Some(match *prio {
                            0..=3 => crate::ui::ColorToken::Danger,
                            4 => crate::ui::ColorToken::Warn,
                            _ => crate::ui::ColorToken::Fg2,
                        });
                        SceneRow {
                            cols: vec![msg.clone()],
                            key: 0,
                            action: None,
                            color,
                            col_colors: vec![],
                            surface: None,
                        }
                    })
                    .collect(),
            ),
        );
        m.insert("jr_empty", SceneValue::Toggle(self.jr_lines.is_empty()));
        m.insert("jr_live", SceneValue::Toggle(!self.jr_lines.is_empty()));
        // systemd failed units: newest-first raised chips (danger name · ✕)
        m.insert(
            "sus_rows",
            SceneValue::Rows(
                self.sus_failed
                    .iter()
                    .rev()
                    .take(5)
                    .map(|unit| SceneRow {
                        cols: vec![unit.clone(), ICON_CLOSE.into()],
                        key: 0,
                        action: None,
                        color: Some(crate::ui::ColorToken::Danger),
                        col_colors: vec![None, Some(crate::ui::ColorToken::Danger)],
                        surface: Some(crate::ui::ColorToken::Raised),
                    })
                    .collect(),
            ),
        );
        m.insert("sus_empty", SceneValue::Toggle(self.sus_failed.is_empty()));
        m.insert("sus_live", SceneValue::Toggle(!self.sus_failed.is_empty()));
        // SMART health rows: mono dev (danger when failed) · dim model ·
        // temp (warn ≥ 55°) · status glyph (ok green / danger ✕)
        m.insert(
            "sm_rows",
            SceneValue::Rows(
                self.sm_disks
                    .iter()
                    .map(|(dev, model, passed, temp)| {
                        let dev_s = dev.rsplit('/').next().unwrap_or(dev).to_string();
                        SceneRow {
                            cols: vec![
                                dev_s,
                                model.clone(),
                                format!("{temp}\u{b0}"),
                                if *passed {
                                    ICON_CHECK.into()
                                } else {
                                    ICON_CLOSE.into()
                                },
                            ],
                            key: 0,
                            action: None,
                            color: None,
                            col_colors: vec![
                                Some(if *passed {
                                    crate::ui::ColorToken::Fg2
                                } else {
                                    crate::ui::ColorToken::Danger
                                }),
                                Some(crate::ui::ColorToken::Fg3),
                                Some(if *temp >= 55 {
                                    crate::ui::ColorToken::Warn
                                } else {
                                    crate::ui::ColorToken::Fg3
                                }),
                                Some(if *passed {
                                    crate::ui::ColorToken::Ok
                                } else {
                                    crate::ui::ColorToken::Danger
                                }),
                            ],
                            surface: None,
                        }
                    })
                    .collect(),
            ),
        );
        m.insert("sm_empty", SceneValue::Toggle(self.sm_disks.is_empty()));
        m.insert("sm_live", SceneValue::Toggle(!self.sm_disks.is_empty()));
        // conninfo metric rows: interface labels fg, gateway/dns dimmed
        // (the Rust drawer's exact rule), one centered empty caption when
        // no interfaces exist
        m.insert(
            "conn_rows_v",
            SceneValue::Rows(
                self.conn_rows
                    .iter()
                    .map(|(lbl, val)| SceneRow {
                        cols: vec![lbl.clone(), val.clone()],
                        key: 0,
                        action: None,
                        color: None,
                        col_colors: vec![
                            Some(if lbl == "gateway" || lbl == "dns" {
                                crate::ui::ColorToken::Fg3
                            } else {
                                crate::ui::ColorToken::Fg
                            }),
                            Some(if lbl == "gateway" || lbl == "dns" {
                                crate::ui::ColorToken::Fg3
                            } else {
                                crate::ui::ColorToken::Fg
                            }),
                        ],
                        surface: None,
                    })
                    .collect(),
            ),
        );
        m.insert("conn_empty", SceneValue::Toggle(self.conn_rows.is_empty()));
        m.insert("conn_live", SceneValue::Toggle(!self.conn_rows.is_empty()));
        // thermal zones: the 3-wide chip grid — cell label = zone caption
        // (fg3), `sub` = the temp line with the 60°/80° warning ladder
        m.insert(
            "thermal_cells",
            SceneValue::GridCells(
                self.thermal_zones
                    .iter()
                    .map(|(zone, temp)| crate::scene::GridCell {
                        text: zone.chars().take(8).collect(),
                        sub: Some(format!("{:.0}°", temp)),
                        sub_color: Some(if *temp >= 80.0 {
                            crate::ui::ColorToken::Danger
                        } else if *temp >= 60.0 {
                            crate::ui::ColorToken::Warn
                        } else {
                            crate::ui::ColorToken::Fg2
                        }),
                        // exact Rust baselines: zone caption at cy+3, temp
                        // (fs 9) at cy+12 — the centered anchors land at
                        // cy+7 / cy+18 in a 24 px cell, so nudge both up
                        dy: Some(-4.0),
                        sub_dy: Some(-6.0),
                        sub_size: Some(9.0),
                        // the chip's resting well (the Rust drawer paints it
                        // on every zone unconditionally — hover lifts via the
                        // grid's `surface_hover`)
                        surface: Some(crate::ui::ColorToken::Hover),
                        ..Default::default()
                    })
                    .collect(),
            ),
        );
        m.insert(
            "thermal_empty",
            SceneValue::Toggle(self.thermal_zones.is_empty()),
        );
        m.insert(
            "thermal_live",
            SceneValue::Toggle(!self.thermal_zones.is_empty()),
        );
        m.insert(
            "conn_rows",
            SceneValue::Rows(
                self.conn_rows
                    .iter()
                    .map(|(k, v)| SceneRow {
                        cols: vec![k.clone(), v.clone()],
                        key: 0,
                        action: None,
                        color: None,
                        col_colors: vec![],
                        surface: None,
                    })
                    .collect(),
            ),
        );
        // procmon: label rows + per-row CPU-tick fraction of the top process
        // (the scene's bar cell parses it — peak = the first row's ticks, the
        // drawer's implicit ordering makes #1 the 100% reference)
        let proc_top_ticks = self
            .top_procs
            .first()
            .map(|(_, t, _)| *t)
            .unwrap_or(0)
            .max(1);
        m.insert(
            "top_procs_rows",
            SceneValue::Rows(
                self.top_procs
                    .iter()
                    .map(|(name, ticks, _)| {
                        let frac = (*ticks as f32 / proc_top_ticks as f32).clamp(0.0, 1.0);
                        SceneRow {
                            cols: vec![name.clone(), format!("{frac:.4}")],
                            key: 0,
                            action: None,
                            color: None,
                            col_colors: vec![],
                            surface: None,
                        }
                    })
                    .collect(),
            ),
        );
        // fans: label · rpm caption · per-fan bar fraction (slow peak decay,
        // the exact frac the Rust drawer computes) — live rows get the info
        // ink + info fill, stopped fans dim to fg3 with a quiet track
        m.insert(
            "fans_rows",
            SceneValue::Rows(
                self.fans_rpm
                    .iter()
                    .enumerate()
                    .map(|(i, (label, rpm))| {
                        let peak = self.fan_peaks.get(i).copied().unwrap_or(1.0).max(1.0);
                        let frac = (*rpm as f32 / peak).clamp(0.0, 1.0);
                        let live = *rpm > 0;
                        SceneRow {
                            cols: vec![label.clone(), format!("{rpm} rpm"), format!("{frac:.4}")],
                            key: 0,
                            action: None,
                            color: None,
                            col_colors: vec![
                                Some(ColorToken::Fg2),
                                Some(if live {
                                    ColorToken::Fg
                                } else {
                                    ColorToken::Fg3
                                }),
                                None,
                            ],
                            surface: None,
                        }
                    })
                    .collect(),
            ),
        );
        m.insert("fans_empty", SceneValue::Toggle(self.fans_rpm.is_empty()));
        m.insert("fans_live", SceneValue::Toggle(!self.fans_rpm.is_empty()));
        // sensors card: tilt row (label fg2 · right value fg), compass row
        // (label fg2 · right gauge glyph + heading in accent), gyro row (label
        // fg2 · right mono triplet) — each gated on its hardware's presence;
        // one centered empty caption when nothing is exposed
        let s = &self.sensors;
        let none = !s.has_accel && s.heading.is_none() && s.gyro.is_none();
        m.insert("sensors_none", SceneValue::Toggle(none));
        m.insert("sensors_tilt", SceneValue::Toggle(!none));
        m.insert("sensors_compass", SceneValue::Toggle(s.heading.is_some()));
        m.insert("sensors_gyro", SceneValue::Toggle(s.gyro.is_some()));
        m.insert(
            "sensor_tilt",
            SceneValue::Text(format!("pitch {:.0}°  roll {:.0}°", s.pitch, s.roll)),
        );
        if let Some(hdg) = s.heading {
            let arrow = match (0.5 + hdg / 360.0 * 8.0) as usize % 8 {
                0 => ICON_GAUGE_0,
                1 => ICON_GAUGE_1,
                2 => ICON_GAUGE_2,
                3 => ICON_GAUGE_3,
                4 => ICON_GAUGE_4,
                5 => ICON_GAUGE_5,
                6 => ICON_GAUGE_6,
                _ => ICON_GAUGE_7,
            };
            m.insert(
                "sensor_compass",
                SceneValue::Text(format!("{arrow} {:.0}°", hdg)),
            );
        } else {
            m.insert("sensor_compass", SceneValue::Text(String::new()));
        }
        m.insert(
            "sensor_gyro",
            SceneValue::Text(match s.gyro {
                Some((gx, gy, gz)) => format!("{gx:3.0} {gy:3.0} {gz:3.0}"),
                None => String::new(),
            }),
        );
        // ── batch-1 list cards: data-driven rows (scroll + per-row states) ──
        // Bluetooth: icon · name · right state (✓ connected / "unpaired").
        // Connected rows get the accent tint well + accent name/check; the
        // visible window follows `bt_scroll` so wheel scrolling still works.
        m.insert(
            "bt_rows",
            SceneValue::Rows(
                self.bt_devices
                    .iter()
                    .map(|(name, connected, paired)| {
                        let state = if *connected {
                            ICON_CHECK.to_string()
                        } else if !*paired {
                            "unpaired".into()
                        } else {
                            String::new()
                        };
                        let (name_col, state_col) = if *connected {
                            (Some(ColorToken::Acc), Some(ColorToken::Acc))
                        } else if !*paired {
                            (None, Some(ColorToken::Fg3))
                        } else {
                            (None, None)
                        };
                        SceneRow {
                            cols: vec![ICON_BLUETOOTH.to_string(), name.clone(), state],
                            key: 0,
                            action: None,
                            color: None,
                            col_colors: vec![name_col, None, state_col],
                            surface: if *connected {
                                Some(ColorToken::AccTint)
                            } else {
                                None
                            },
                        }
                    })
                    .collect(),
            ),
        );
        m.insert(
            "bt_scroll",
            SceneValue::Ring(self.bt_scroll.min(self.bt_devices.len().saturating_sub(1)) as f32),
        );
        m.insert(
            "bt_chip_on",
            SceneValue::Text(if self.bt_on {
                "on".into()
            } else {
                String::new()
            }),
        );
        m.insert(
            "bt_chip_off",
            SceneValue::Text(if self.bt_on {
                String::new()
            } else {
                "off".into()
            }),
        );
        m.insert(
            "bt_status",
            SceneValue::Text(if self.bt_devices.is_empty() {
                if self.bt_on {
                    "no paired devices".into()
                } else {
                    "bluetooth off — enable in toggles".into()
                }
            } else {
                String::new()
            }),
        );
        // Prices: sym · price · right chg% (green up / red down). The status
        // text shows only when no price data; the window follows `ticker_scroll`.
        m.insert(
            "ticker_rows",
            SceneValue::Rows(
                self.ticker_items
                    .iter()
                    .map(|(sym, price, chg)| SceneRow {
                        cols: vec![sym.clone(), price.clone(), format!("{:+.1}%", chg)],
                        key: 0,
                        action: None,
                        color: None,
                        col_colors: vec![
                            None,
                            None,
                            Some(if *chg >= 0.0 {
                                ColorToken::Ok
                            } else {
                                ColorToken::Danger
                            }),
                        ],
                        surface: None,
                    })
                    .collect(),
            ),
        );
        m.insert(
            "ticker_scroll",
            SceneValue::Ring(
                self.ticker_scroll
                    .min(self.ticker_items.len().saturating_sub(1)) as f32,
            ),
        );
        m.insert(
            "ticker_status",
            SceneValue::Text(if self.ticker_items.is_empty() {
                if self.ticker_state == 1 {
                    "loading prices…".into()
                } else {
                    "no price data".into()
                }
            } else {
                String::new()
            }),
        );
        // Currency: offline reference table, one static row per pair. The
        // currently re-based currency tints its row + sym + value accent; the
        // window follows `currency_scroll`.
        const CCY: [(&str, &str, f64); 12] = [
            ("USD", "Dollar", 1.0),
            ("EUR", "Euro", 0.9200),
            ("GBP", "Pound", 0.7900),
            ("JPY", "Yen", 149.50),
            ("INR", "Rupee", 83.10),
            ("CNY", "Yuan", 7.2400),
            ("RUB", "Ruble", 92.50),
            ("AUD", "AU Dollar", 1.5200),
            ("CAD", "CA Dollar", 1.3700),
            ("KRW", "Won", 1347.00),
            ("XAU", "Gold oz", 0.000420),
            ("XBT", "BTC", 0.0000160),
        ];
        let base = self.currency_base.as_str();
        let base_mult = CCY
            .iter()
            .find(|(s, _, _)| *s == base)
            .map(|(_, _, m)| *m)
            .unwrap_or(1.0);
        m.insert(
            "currency_rows",
            SceneValue::Rows(
                CCY.iter()
                    .map(|&(sym, label, mult)| {
                        let active = sym == base;
                        let val = mult / base_mult;
                        let vstr = if val >= 100.0 {
                            format!("{:.0}", val)
                        } else if val >= 10.0 {
                            format!("{:.1}", val)
                        } else {
                            format!("{:.2}", val)
                        };
                        SceneRow {
                            cols: vec![
                                sym.to_string(),
                                label.to_string(),
                                format!("1 {base} = {vstr} {sym}"),
                            ],
                            key: 0,
                            action: None,
                            color: None,
                            col_colors: if active {
                                vec![Some(ColorToken::Acc), None, Some(ColorToken::Acc)]
                            } else {
                                vec![None, None, None]
                            },
                            surface: if active {
                                Some(ColorToken::AccTint)
                            } else {
                                None
                            },
                        }
                    })
                    .collect(),
            ),
        );
        m.insert(
            "currency_scroll",
            SceneValue::Ring(self.currency_scroll.min(11) as f32),
        );
        m.insert(
            "currency_chip",
            SceneValue::Text(if base != "USD" {
                base.to_string()
            } else {
                String::new()
            }),
        );
        // SSH / VPN: one line per tunnel/connection; the leading glyph flips
        // globe/shield by whether the line names a VPN.
        m.insert(
            "sshvpn_rows",
            SceneValue::Rows(
                self.ssh_vpn_lines
                    .iter()
                    .map(|line| SceneRow {
                        cols: vec![
                            if line.starts_with("VPN") {
                                ICON_GLOBE.into()
                            } else {
                                ICON_SHIELD.into()
                            },
                            line.clone(),
                        ],
                        key: 0,
                        action: None,
                        color: None,
                        col_colors: vec![],
                        surface: None,
                    })
                    .collect(),
            ),
        );
        m.insert(
            "sshvpn_status",
            SceneValue::Text(if self.ssh_vpn_lines.is_empty() {
                "No connections".into()
            } else {
                String::new()
            }),
        );
        // Recent files: name · open-glyph, right side. The hovered row tints
        // both columns accent (matching the Rust drawer's per-key hover); the
        // visible window follows `recent_scroll` so wheel scrolling keeps the
        // top-edge rows keyed to `recent key_base + visible index` for the
        // open-file click plumbing.
        m.insert(
            "recent_files_n",
            SceneValue::Text(format!("{} files", self.recent_files.len())),
        );
        m.insert(
            "recent_rows",
            SceneValue::Rows(
                self.recent_files
                    .iter()
                    .enumerate()
                    .map(|(i, (name, _path))| {
                        let j = i.saturating_sub(self.recent_scroll) as u32;
                        let hov = self.hover_key == crate::shell::RECENT_KEY_BASE + j;
                        let (name_col, open_col) = if hov {
                            (Some(ColorToken::Acc), Some(ColorToken::Acc))
                        } else {
                            (None, Some(ColorToken::Fg3))
                        };
                        let nm: String = name.chars().take(24).collect();
                        SceneRow {
                            cols: vec![nm, ICON_OPEN.to_string()],
                            key: 0,
                            action: None,
                            color: None,
                            surface: None,
                            col_colors: vec![name_col, open_col],
                        }
                    })
                    .collect(),
            ),
        );
        // Lyrics: fully declarative — header (accent note glyph, dynamic
        // state chip), the track caption, the centered fallback message, and
        // the karaoke body: word-split lines bound to `Rows` whose active row
        // washes accent with the live word highlighted white, plain lyrics
        // falling back to single-column whole lines.
        let lyr_state = self.lyrics_state;
        let lyr_synced = lyr_state == 2 && crate::lyrics::has_timing(&self.lyrics_lines);
        let lyr_none = lyr_state == 3;
        let lyr_other_lbl = match lyr_state {
            1 => "fetching…".to_string(),
            2 => "plain".to_string(),
            _ => "—".to_string(),
        };
        let lyr_msg = match lyr_state {
            1 => "fetching lyrics…",
            3 => "no lyrics found",
            _ => "no track playing",
        };
        let track = if lyr_state == 2 && !self.lyrics_track.is_empty() {
            self.lyrics_track.replace(" || ", " — ")
        } else if !self.media_title.is_empty() {
            if !self.media_artist.is_empty() {
                format!("{} — {}", self.media_artist, self.media_title)
            } else {
                self.media_title.clone()
            }
        } else {
            "no track playing".to_string()
        };
        let t2: String = track.chars().take(42).collect();
        let pos = self.media_pos_now() as f64 / 1e6;
        let lyr_active = self.lyrics_active_line();
        let lyr_cur = lyr_active
            .and_then(|a| crate::lyrics::active_word(&self.lyrics_lines, pos, a))
            .unwrap_or(0);
        let lyr_karaoke: Option<Vec<KaraokeLine>> = if lyr_synced {
            Some(
                self.lyrics_lines
                    .iter()
                    .enumerate()
                    .map(|(i, l)| KaraokeLine {
                        active: lyr_active == Some(i),
                        words: l
                            .text
                            .split_whitespace()
                            .enumerate()
                            .map(|(wi, w)| {
                                (
                                    w.chars().take(120).collect(),
                                    lyr_active == Some(i) && wi == lyr_cur,
                                )
                            })
                            .collect(),
                    })
                    .collect(),
            )
        } else {
            None
        };
        m.insert("lyr_status_synced", SceneValue::Toggle(lyr_synced));
        m.insert("lyr_status_none", SceneValue::Toggle(lyr_none));
        m.insert(
            "lyr_status_other_on",
            SceneValue::Toggle(!lyr_synced && !lyr_none),
        );
        m.insert("lyr_status_other", SceneValue::Text(lyr_other_lbl));
        m.insert("lyr_track", SceneValue::Text(t2));
        m.insert("lyr_msg", SceneValue::Text(lyr_msg.to_string()));
        m.insert(
            "lyr_empty",
            SceneValue::Toggle(self.lyrics_lines.is_empty()),
        );
        m.insert("lyr_scroll", SceneValue::Ring(self.lyrics_scroll as f32));
        m.insert(
            "lyr_rows",
            SceneValue::Rows(
                self.lyrics_lines
                    .iter()
                    .map(|l| SceneRow {
                        cols: vec![l.text.chars().take(60).collect()],
                        key: 0,
                        action: None,
                        color: None,
                        surface: None,
                        col_colors: vec![],
                    })
                    .collect(),
            ),
        );
        if let Some(k) = lyr_karaoke {
            m.insert("lyr_karaoke", SceneValue::Karaoke(k));
        } else {
            // the lyrics scene binds this name and must render before the
            // words sync, so "absent" is a state it handles, not a dead
            // binding — say so where the condition lives
            m.declare_conditional("lyr_karaoke");
        }
        m.insert(
            "recent_scroll",
            SceneValue::Ring(
                self.recent_scroll
                    .min(self.recent_files.len().saturating_sub(1)) as f32,
            ),
        );
        m.insert(
            "recent_status",
            SceneValue::Text(if self.recent_files.is_empty() {
                "no recent files tracked".into()
            } else {
                String::new()
            }),
        );
        // Settings sidebar nav: glyph + label per tab. Per-frame row `surface`
        // + `col_colors` carry the selected/hoarked states (selected = accent
        // glyph on HoverHl, hover = fg on Hover, idle = fg2 on nothing),
        // mirroring the Rust setting rows glyph-for-glyph.
        m.insert(
            "settings_nav",
            SceneValue::Rows({
                let tabs: [(u32, &str, &str, crate::shell::SettingsTab); 6] = [
                    (
                        crate::shell::panels::settings::NAV_NETWORK,
                        ICON_WIFI,
                        "Network",
                        crate::shell::SettingsTab::Network,
                    ),
                    (
                        crate::shell::panels::settings::NAV_SOUND,
                        ICON_VOLUME,
                        "Sound & Display",
                        crate::shell::SettingsTab::Sound,
                    ),
                    (
                        crate::shell::panels::settings::NAV_APPEARANCE,
                        ICON_PALETTE,
                        "Appearance",
                        crate::shell::SettingsTab::Appearance,
                    ),
                    (
                        crate::shell::panels::settings::NAV_PILL,
                        ICON_PALETTE,
                        "Pill",
                        crate::shell::SettingsTab::Pill,
                    ),
                    (
                        crate::shell::panels::settings::NAV_SYSTEM,
                        ICON_MONITOR,
                        "System",
                        crate::shell::SettingsTab::System,
                    ),
                    (
                        crate::shell::panels::settings::NAV_MISC,
                        ICON_SETTINGS,
                        "Misc",
                        crate::shell::SettingsTab::Misc,
                    ),
                ];
                let mut rows: Vec<SceneRow> = Vec::with_capacity(tabs.len());
                for (key, glyph, label, tab) in tabs {
                    let sel = self.settings_tab() == tab;
                    let hov = !sel && self.hover_key == key;
                    let surface = if sel {
                        Some(ColorToken::HoverHl)
                    } else if hov {
                        Some(ColorToken::Hover)
                    } else {
                        None
                    };
                    let gcol = Some(if sel {
                        ColorToken::Acc
                    } else if hov {
                        ColorToken::Fg
                    } else {
                        ColorToken::Fg2
                    });
                    let lcol = Some(if sel || hov {
                        ColorToken::Fg
                    } else {
                        ColorToken::Fg2
                    });
                    rows.push(SceneRow {
                        cols: vec![glyph.to_string(), label.to_string()],
                        key,
                        // The sidebar row SELECTS the pane: a `Set` into the
                        // surface's own `nav.tab` store. Not a Rust handler,
                        // because a Rust handler would win the key and the
                        // scene's write would never run — the ordering that
                        // makes `scene_click_actions` the escape hatch rather
                        // than a fallback.
                        action: Some(crate::scene::SceneAction::Set {
                            name: "nav.tab".to_string(),
                            value: crate::scene::PropValue::Num(tab.index()),
                        }),
                        color: None,
                        col_colors: vec![gcol, lcol],
                        surface,
                    });
                }
                rows
            }),
        );
        m.insert(
            "settings_ink",
            SceneValue::Text(
                match self.settings_tab() {
                    crate::shell::SettingsTab::Network => "settings_network",
                    crate::shell::SettingsTab::Sound => "settings_sound",
                    crate::shell::SettingsTab::Appearance => "settings_appearance",
                    crate::shell::SettingsTab::Pill => "settings_pill",
                    crate::shell::SettingsTab::System => "settings_system",
                    crate::shell::SettingsTab::Misc => "settings_misc",
                }
                .to_string(),
            ),
        );
        // ── calendar / apps / mirror cards (declarative `Grid` scenes) ──
        // the 6×7 day matrix: off-month slots stay invisible (no region),
        // today gets the accent tint + accent number, the month name heads
        // the card. Weekday heads live in the RON `head` row.
        {
            let (first_wday, days, today, month) = self.calendar_info();
            m.insert("cal_month", SceneValue::Text(month));
            let mut cal = Vec::with_capacity(42);
            for cell in 0..42 {
                let day = cell - first_wday + 1;
                let visible = day >= 1 && day <= days;
                let is_today = self.cal_offset == 0 && day == today;
                cal.push(GridCell {
                    text: if visible {
                        day.to_string()
                    } else {
                        String::new()
                    },
                    visible,
                    color: if !visible {
                        None
                    } else if is_today {
                        Some(ColorToken::Acc)
                    } else {
                        Some(ColorToken::Fg2)
                    },
                    // day ink lifts to fg on hover (today keeps acc), exactly
                    // the Rust drawer's per-day hover
                    hover: if visible && !is_today {
                        Some(ColorToken::Fg)
                    } else {
                        None
                    },
                    surface: if visible && is_today {
                        Some(ColorToken::AccTint)
                    } else {
                        None
                    },
                    // hovered today keeps its accent wash (the Rust drawer
                    // paints hover first, acc_tint over it)
                    surface_hover: if visible && is_today {
                        Some(ColorToken::AccTint)
                    } else {
                        None
                    },
                    ..GridCell::default()
                });
            }
            m.insert("cal_cells", SceneValue::GridCells(cal));
        }
        // the launcher slate: filled app tiles (raster or Nerd fallback +
        // clipped name, bottom label) vs "+" empty pickers. Resting tile ink
        // is a per-frame blend the scene binds by name; both hover highlights
        // use the lifted HoverHl token so hover consequently lifts.
        {
            let mut apps = Vec::with_capacity(8);
            // name clip parity: the Rust drawer takes (tile_w − 4)/5.5
            // chars — the scene clips via `truncate` (card width known from
            // the rect the scene pass tracked last frame; fallback 300)
            let clip = {
                let (_, _, rw, _) = self.app_shortcut_rect;
                let w = if rw > 0.0 { rw } else { self.scale.s(300.0) };
                let cols = if w >= self.scale.s(330.0) {
                    4.0
                } else if w >= self.scale.s(250.0) {
                    3.0
                } else if w >= self.scale.s(165.0) {
                    2.0
                } else {
                    1.0
                };
                let tw = (w - self.scale.s(24.0) - (cols - 1.0) * self.scale.s(8.0)) / cols;
                (((tw - self.scale.s(4.0)) / self.scale.s(5.5)).max(3.0)) as u32
            };
            for i in 0..8 {
                let filled = i < self.app_shortcuts.len();
                if !filled {
                    apps.push(GridCell {
                        text: "+".into(),
                        icon: true,
                        size: Some(20.0),
                        // centered base already sits at cell_h/2 − 5; the
                        // Rust drawer pins the plus at cell_h/2 − 3
                        dy: Some(2.0),
                        color: Some(ColorToken::Fg3),
                        // hover turns the plus accent (the empty-slate call)
                        hover: Some(ColorToken::Acc),
                        surface: Some(ColorToken::Value(crate::ui::DynColor::named(
                            "app_tile_empty",
                        ))),
                        surface_hover: Some(ColorToken::HoverHl),
                        ..GridCell::default()
                    });
                    continue;
                }
                match self
                    .apps
                    .apps
                    .iter()
                    .find(|a| a.id == self.app_shortcuts[i])
                {
                    Some(a) => {
                        let ik = a.icon_key();
                        let has_ik = ik.is_some();
                        apps.push(GridCell {
                            image_key: ik,
                            glyph: if !has_ik {
                                Some(ICON_APPS.into())
                            } else {
                                None
                            },
                            text: a.name.clone(),
                            bottom: true,
                            color: Some(ColorToken::Fg2),
                            surface: Some(ColorToken::Value(crate::ui::DynColor::named(
                                "app_tile",
                            ))),
                            surface_hover: Some(ColorToken::HoverHl),
                            truncate: Some(clip),
                            ..GridCell::default()
                        });
                    }
                    None => {
                        apps.push(GridCell {
                            glyph: Some(ICON_APPS.into()),
                            text: self.app_shortcuts[i].clone(),
                            bottom: true,
                            color: Some(ColorToken::Fg2),
                            surface: Some(ColorToken::Value(crate::ui::DynColor::named(
                                "app_tile",
                            ))),
                            surface_hover: Some(ColorToken::HoverHl),
                            truncate: Some(clip),
                            ..GridCell::default()
                        });
                    }
                }
            }
            m.insert("app_cells", SceneValue::GridCells(apps));
        }
        // the mirror card's live frame: pane gates, camera state, the
        // capture-rate rows (active row shows the delivered rate), the rec
        // button label/icon and its per-frame dip, and the show-fps toggle.
        {
            use crate::shell::MIRROR_FPS_ROW_BASE;
            m.insert("mirror_pane_0", SceneValue::Toggle(self.mirror_pane == 0));
            m.insert("mirror_pane_1", SceneValue::Toggle(self.mirror_pane == 1));
            m.insert("mirror_cam_ok", SceneValue::Toggle(self.cam_ok));
            m.insert("mirror_cam_missing", SceneValue::Toggle(!self.cam_ok));
            m.insert("mirror_show_fps", SceneValue::Toggle(self.mirror_show_fps));
            let rec = self.mirror_rec;
            let rec_label = if rec {
                let mm = self.mirror_rec_secs / 60;
                let ss = self.mirror_rec_secs % 60;
                format!("Stop {mm:02}:{ss:02}")
            } else {
                "Record".to_string()
            };
            m.insert("mirror_rec_label", SceneValue::Text(rec_label));
            m.insert(
                "mirror_rec_icon",
                SceneValue::Text(if rec {
                    ICON_SQUARE.into()
                } else {
                    ICON_DOT.into()
                }),
            );
            let mut fps = Vec::with_capacity(crate::shell::MIRROR_FPS_OPTIONS.len());
            for (i, rate) in crate::shell::MIRROR_FPS_OPTIONS.iter().enumerate() {
                let active = self.mirror_fps == *rate;
                let mut label = format!("{rate} fps");
                if active {
                    if *rate == 60 {
                        label.push_str(" · best");
                    }
                    if self.mirror_show_fps && self.cam_rate > 0 && self.cam_rate != *rate {
                        label.push_str(&format!(" · ~{} fps", self.cam_rate));
                    }
                }
                fps.push(SceneRow {
                    cols: vec![label],
                    key: MIRROR_FPS_ROW_BASE + i as u32,
                    color: Some(if active {
                        ColorToken::Fg
                    } else {
                        ColorToken::Fg2
                    }),
                    // SelBg targets the active row; hover fills via the Rows'
                    // `hover_surface` — row surface would always paint
                    surface: if active {
                        Some(ColorToken::SelBg)
                    } else {
                        None
                    },
                    ..SceneRow::default()
                });
            }
            m.insert("mirror_fps_rows", SceneValue::Rows(fps));
        }
        // ── calendar / mirror / apps pane gates + dots (declared scenes) ──
        // mirror_pane_1 already exists above; the flip helpers + hover-dot
        // gate need the SAME toggles as plain bools
        m.insert("mirror_pane_0", SceneValue::Toggle(self.mirror_pane == 0));
        m.insert("mirror_pane_1", SceneValue::Toggle(self.mirror_pane == 1));
        m.insert("mirror_pane", SceneValue::Ring(self.mirror_pane as f32));
        m.insert("mirror_hover", SceneValue::Toggle(self.mirror_dots_hover()));
        m.insert(
            "cal_pane",
            SceneValue::Ring(self.cal_offset.max(-1).min(1) as f32),
        );
        m.insert(
            "app_slots",
            SceneValue::Ring(self.app_shortcuts.len() as f32),
        );
        // ── sliders card (declared `sliders.ron`, zero Ink) ──
        // pane 0 = the six fader rows, pane 1 = the two switches that add /
        // remove the balance + saturation rows; the flip helper + the
        // hover-gated `Dots` need the same state as plain values
        m.insert("sliders_pane_0", SceneValue::Toggle(self.sliders_pane == 0));
        m.insert("sliders_pane_1", SceneValue::Toggle(self.sliders_pane == 1));
        m.insert("sliders_pane", SceneValue::Ring(self.sliders_pane as f32));
        m.insert(
            "sliders_hover",
            SceneValue::Toggle(self.sliders_dots_hover()),
        );
        // the pane-1 switches double as the pane-0 ROW gates — a hidden row
        // reserves no slot, so the vertically centered stack reflows exactly
        // like the drawer filtering its row list
        m.insert("show_balance", SceneValue::Toggle(self.show_balance));
        m.insert("show_saturation", SceneValue::Toggle(self.show_saturation));
        // the "Smooth transition" switch is a BEHAVIOUR flag, not a row gate, so
        // it is published on its own and gates nothing: `$states/sst` lives in
        // the helper's state file (polled, never persisted here)
        m.insert("sst_on", SceneValue::Toggle(self.sst_on));
        // balance is the R-share of the L/R mix, centered at 0.5 (0.5 = silent)
        let lr = self.vol_left + self.vol_right;
        m.insert(
            "balance_fader",
            SceneValue::Fader(if lr > 0.001 {
                (self.vol_right / lr).clamp(0.0, 1.0)
            } else {
                0.5
            }),
        );
        m.insert(
            "sat_fader",
            SceneValue::Fader(self.saturation as f32 / 200.0),
        );
        // the kelvin fader rides the drawer's 1200..6500K warm ramp
        let kel_warm = ((self.sunset_kelvin as f32 - 1200.0) / 5300.0).clamp(0.0, 1.0);
        m.insert("kelvin_fader", SceneValue::Fader(kel_warm));
        m.insert("vol_muted", SceneValue::Toggle(self.volume_muted));
        // value-column labels — "Muted" replaces the percentage, as in the drawer
        m.insert(
            "bri_lbl",
            SceneValue::Text(format!("{}%", (self.brightness * 100.0).round() as i32)),
        );
        m.insert(
            "vol_lbl",
            SceneValue::Text(if self.volume_muted {
                "Muted".to_string()
            } else {
                format!("{}%", (self.volume * 100.0).round() as i32)
            }),
        );
        m.insert(
            "mic_lbl",
            SceneValue::Text(if self.mic_muted {
                "Muted".to_string()
            } else {
                format!("{}%", (self.mic_level * 100.0).round() as i32)
            }),
        );
        m.insert("sat_lbl", SceneValue::Text(format!("{}", self.saturation)));
        m.insert(
            "kel_lbl",
            SceneValue::Text(format!("{}K", self.sunset_kelvin)),
        );
        // the volume row's glyph swaps to the mute icon while muted (`mic_icon`
        // already carries the mic row's swap)
        m.insert(
            "vol_glyph",
            SceneValue::Text(if self.volume_muted {
                ICON_MUTE.into()
            } else {
                ICON_VOLUME.into()
            }),
        );
        // ── toggles card (declared `toggles.ron`, zero Ink) ── the six switch
        // tiles; each carries its own region key and the two network tiles
        // carry a corner-chip key (the drawer's "more" chevron, keys 42 / 44)
        m.insert(
            "toggles_tiles",
            SceneValue::Tiles(vec![
                SceneTile {
                    key: 41,
                    glyph: ICON_WIFI.into(),
                    label: "Wi-Fi".into(),
                    on: self.wifi_on,
                    chip_key: 42,
                },
                SceneTile {
                    key: 43,
                    glyph: ICON_BLUETOOTH.into(),
                    label: "BT".into(),
                    on: self.bt_on,
                    chip_key: 44,
                },
                SceneTile {
                    key: 45,
                    glyph: ICON_SNOW.into(),
                    label: "DND".into(),
                    on: self.dnd,
                    chip_key: 0,
                },
                SceneTile {
                    key: 46,
                    glyph: ICON_CAFFEINE.into(),
                    label: "Caffeine".into(),
                    on: self.caffeine_on,
                    chip_key: 0,
                },
                // the last two are the screen look itself, and each tile owns
                // the ONE value behind it: Blue Light is the Kelvin (off at 0
                // and at the 6500 K neutral notch), Saturation is `satu_$s` (off
                // at its 100 neutral notch) — there is no switch file left for a
                // tile to disagree with. 0 is not "off" for either: it is
                // grayscale / a cold shift, both real effects.
                SceneTile {
                    key: 47,
                    glyph: ICON_MOON.into(),
                    label: "Blue Light".into(),
                    on: self.sunset_on,
                    chip_key: 0,
                },
                SceneTile {
                    key: 48,
                    glyph: ICON_SHADER.into(),
                    label: "Saturation".into(),
                    on: self.saturation != SAT_NEUTRAL,
                    chip_key: 0,
                },
            ]),
        );
        // ── dynamic colors (`ColorToken::Value`) — state-driven single-element
        // colors, pre-resolved against the LIVE theme sv_* palette so a
        // `value("dnd_chip")` token authored in `.ron` follows this exact
        // palette (DND chip accent, chips, banner accents as they get wired).
        let pal = Pal {
            fg: self.sv_fg,
            bg: self.sv_bg,
            acc: self.sv_acc,
            sfg: self.sv_sfg,
        };
        // ── compositor card (declared `compositor.ron`, zero Ink) ── each effect
        // tile's four state inks are per-frame blends a token can't spell: a lit
        // tile fills with the accent at 16% alpha and lifts to 25% under the
        // cursor, its dot + label go accent, and the caption spells "on"/"off".
        // (The screenshot row's fills ride the same ladder, hover-only.)
        for (i, (key, on)) in [
            (COMP_BLUR_KEY, self.fx_blur_on),
            (COMP_SHADOW_KEY, self.fx_shadow_on),
            (COMP_OPACITY_KEY, self.fx_opacity_on),
            (COMP_SHOT_AREA_KEY, false),
            (COMP_SHOT_FULL_KEY, false),
        ]
        .into_iter()
        .enumerate()
        {
            let i = i as f32; // the RON binds these as `comp_tile_0` / `_1` / …
            let hov = self.hover_key == key;
            m.insert_color(
                format!("comp_tile_{i}"),
                SceneColor::Raw(if on {
                    if hov {
                        mix(ui::hover(&pal), pal.acc, 0.25)
                    } else {
                        (pal.acc & 0xFFFF_FF00) | 0x28
                    }
                } else if hov {
                    ui::hover_hl(&pal)
                } else {
                    ui::hover(&pal)
                }),
            );
            m.insert_color(
                format!("comp_dot_{i}"),
                SceneColor::Raw(if on { pal.acc } else { ui::fg3(&pal) }),
            );
            m.insert_color(
                format!("comp_ink_{i}"),
                SceneColor::Raw(if on { pal.acc } else { pal.fg }),
            );
            m.insert(
                format!("comp_state_{i}"),
                SceneValue::Text(if on {
                    "on".to_string()
                } else {
                    "off".to_string()
                }),
            );
            // the screenshot buttons are hover-only surfaces: `off` is their
            // resting state, so the same ladder paints them
            m.insert_color(
                format!("comp_shot_{i}"),
                SceneColor::Raw(if hov {
                    ui::hover_hl(&pal)
                } else {
                    ui::hover(&pal)
                }),
            );
        }
        // ── power cards (powerh/powerv) ── each of the five action buttons is
        // a declared `Surface` (its own background + hairline) holding a
        // bottom-anchored `Bar` for the hold-to-confirm liquid, the glyph and
        // the hit region. All four pieces are per-frame: the background
        // climbs raised → raised_hl → acc_tint as the pointer arrives and the
        // hold starts, the danger glyphs ride a red-tinted ink, and the liquid
        // rides the hold fraction — none of it a fixed token.
        m.insert_color("clear", SceneColor::Raw(0));
        m.insert_color(
            "power_hold_fill",
            SceneColor::Raw(mix(ui::hover(&pal), pal.acc, 0.5)),
        );
        for (i, &(_, danger)) in Self::power_card_items().iter().enumerate() {
            let key = POWER_CARD_KEY_BASE + i as u32;
            let hov = self.hover_key == key;
            let holding = self.power_hold == Some(key);
            let fill = if holding { self.power_hold_fill() } else { 0.0 };
            m.insert_color(
                format!("power_bg_{i}"),
                SceneColor::Raw(if holding {
                    ui::acc_tint(&pal)
                } else if hov {
                    ui::raised_hl(&pal)
                } else {
                    ui::raised(&pal)
                }),
            );
            m.insert(
                format!("power_hold_{i}"),
                SceneValue::Text(format!("{fill:.4}")),
            );
            // the drawer's own sliver threshold: a hold under 1.5 px of
            // liquid paints nothing at all
            m.insert(
                format!("power_hold_on_{i}"),
                SceneValue::Toggle(holding && self.scale.s(32.0) * fill >= self.scale.s(1.5)),
            );
            m.insert_color(
                format!("power_ink_{i}"),
                SceneColor::Raw(if danger {
                    mix(RED, pal.fg, 0.25)
                } else {
                    pal.fg
                }),
            );
        }
        // ── whole-shell PANEL surfaces ── the Power / Wi-Fi / Bluetooth /
        // Audio submenus and the OSD draw from the same per-frame ladders
        // their Rust drawers compute inline, so each publishes here instead
        // of hiding arithmetic in RON. Keys and the ladders are the panels'
        // OWN (the power panel reds only Shutdown, and keys its tiles 1..=5
        // rather than the cards' `POWER_CARD_KEY_BASE` ladder).
        for (i, &(_, _, danger)) in Self::power_panel_items().iter().enumerate() {
            let key = 1 + i as u32;
            let hov = self.hover_key == key;
            let holding = self.power_hold == Some(key);
            let fill = if holding { self.power_hold_fill() } else { 0.0 };
            m.insert_color(
                format!("ppower_bg_{i}"),
                SceneColor::Raw(if holding {
                    ui::acc_tint(&pal)
                } else if hov {
                    ui::raised_hl(&pal)
                } else {
                    ui::raised(&pal)
                }),
            );
            m.insert(
                format!("ppower_hold_{i}"),
                SceneValue::Text(format!("{fill:.4}")),
            );
            // the drawer's own sliver threshold, at the PANEL's 92 px tile
            m.insert(
                format!("ppower_hold_on_{i}"),
                SceneValue::Toggle(holding && self.scale.s(92.0) * fill >= self.scale.s(1.5)),
            );
            m.insert_color(
                format!("ppower_ink_{i}"),
                SceneColor::Raw(if danger { RED } else { pal.fg }),
            );
        }
        // the OSD's own two strings + presence gate (the drawer returns before
        // drawing when no OSD is live, so the scene hides both texts)
        let osd = self.osd.as_ref();
        m.insert("osd_on", SceneValue::Toggle(osd.is_some()));
        m.insert(
            "osd_glyph",
            SceneValue::Text(osd.map(|o| o.glyph).unwrap_or_default().to_string()),
        );
        m.insert(
            "osd_text",
            SceneValue::Text(osd.map(|o| o.text.clone()).unwrap_or_default().to_string()),
        );
        // The sub-panel header's per-panel STATE, hoisted out of any `match
        // self.mode` on purpose: both radios' meta text and color depend only
        // on their own on/off flag, so there is nothing mode-dependent left to
        // branch on. The panel that is not on screen simply does not read them.
        let wifi_meta = if self.wifi_on { "On" } else { "Off" };
        let wifi_meta_col = if self.wifi_on { pal.acc } else { ui::fg3(&pal) };
        let bt_meta = if self.bt_on { "On" } else { "Off" };
        let bt_meta_col = if self.bt_on { pal.acc } else { ui::fg3(&pal) };
        // The sub-panel header's PER-FRAME STATE, per panel (phase 2). What is
        // NOT here any more is the panel's TITLE: that was a Rust `match
        // self.mode` choosing UI copy, and it now lives in `shell.ron` as the
        // `title:` prop each `subhead` call site passes. What stays is exactly
        // the part the scene cannot know — whether the radio is up, and the
        // color and size that follow from it.
        //
        // The names are per-panel (`pill_wifi_meta*`, `pill_bt_meta*`) rather
        // than one shared `sub_meta*` triple precisely because each `subhead`
        // use site now points at its own binding through props; a shared name
        // would have re-created the Rust table's job in the scene file.
        //
        // Sizes are `Ring`s, not `Text`s: `font_size` binds through a `Val`.
        m.insert("pill_wifi_meta", SceneValue::Text(wifi_meta.to_string()));
        m.insert("pill_wifi_meta_size", SceneValue::Ring(12.0));
        m.insert_color("pill_wifi_meta_col", SceneColor::Raw(wifi_meta_col));
        m.insert("pill_bt_meta", SceneValue::Text(bt_meta.to_string()));
        m.insert("pill_bt_meta_size", SceneValue::Ring(12.0));
        m.insert_color("pill_bt_meta_col", SceneColor::Raw(bt_meta_col));
        // ── workspace switcher ── the grid is TWO `Rows` lists (the surface
        // has no `x` on `Rows`, so the column offset rides `pad`), filled in
        // the drawer's own column-major order: column `c` holds the
        // workspaces `c, c + cols, c + 2·cols, …`. Per workspace: the accent
        // rail's value (1.0 on the active one, 0 elsewhere — a bar cell with
        // a clear track draws nothing at 0), the number, and the two inks
        // (the wash is a 20% accent blend of the hover fill, and the number
        // ladder is accent / fg / fg2), each published under a per-workspace
        // binding name because `Rows` prefers a row's own `surface` over
        // `hover_surface` while this drawer resolves active-FIRST.
        let ws_total = self.ws_count.min(10);
        let ws_cols = 2usize.min(self.ws_count.max(1));
        m.insert("ws_count", SceneValue::Text(format!("{}", self.ws_count)));
        for c in 0..2 {
            let mut ws_rows = Vec::new();
            let mut r = 0usize;
            loop {
                let i = r * ws_cols + c;
                if i >= ws_total {
                    break;
                }
                r += 1;
                let key = 10 + i as u32;
                let hov = self.hover_key == key;
                let active = i == self.ws_active;
                m.insert_color(
                    format!("ws_wash_{i}"),
                    SceneColor::Raw(if active {
                        mix(crate::ui::hover(&pal), pal.acc, 0.20)
                    } else if hov {
                        crate::ui::hover_hl(&pal)
                    } else {
                        crate::ui::hover(&pal)
                    }),
                );
                m.insert_color(
                    format!("ws_num_{i}"),
                    SceneColor::Raw(if active {
                        pal.acc
                    } else if hov {
                        pal.fg
                    } else {
                        ui::fg2(&pal)
                    }),
                );
                ws_rows.push(SceneRow {
                    cols: vec![
                        if active { "1" } else { "0" }.to_string(),
                        (i + 1).to_string(),
                        "Desktop".to_string(),
                    ],
                    key,
                    action: None,
                    color: None,
                    surface: Some(ColorToken::Value(
                        crate::ui::DynColor::new(&format!("ws_wash_{i}")).unwrap(),
                    )),
                    col_colors: vec![
                        None,
                        Some(ColorToken::Value(
                            crate::ui::DynColor::new(&format!("ws_num_{i}")).unwrap(),
                        )),
                        None,
                    ],
                });
            }
            m.insert(format!("ws_col{c}"), SceneValue::Rows(ws_rows));
        }
        // ── polkit auth dialog ── the strings are clipped here (the drawer's
        // own `chars().take((w - 48) / (12 · 0.62))`, keyed to the dialog
        // width) and every fill that moves with hover / focus / the
        // verifying state rides a per-frame color. The cursor's x is a
        // measured run, so it is published as a bound scalar for the
        // surface's `Surface.x_var`.
        let pw_focus = self.hover_key == polkit_auth::POLKIT_KEY_PW;
        // the drawer's own clip: `((w - 48) / (12 · 0.62))` chars, with `w`
        // the LIVE dialog width (`Mode::size` — the same number the layout
        // function is handed, so scene and drawer cannot disagree).
        let pk_w = Mode::PolkitAuth.size(&self.cfg).0;
        let pk_max_chars = (((pk_w - 48.0) / (12.0 * 0.62)).max(0.0)) as usize;
        let pk_msg = if self.polkit_message.is_empty() {
            "Authentication required".to_string()
        } else {
            self.polkit_message.clone()
        };
        m.insert(
            "polkit_msg",
            SceneValue::Text(pk_msg.chars().take(pk_max_chars).collect()),
        );
        m.insert(
            "polkit_act",
            SceneValue::Text(self.polkit_action_id.chars().take(pk_max_chars).collect()),
        );
        m.insert("polkit_user", SceneValue::Text(self.username.clone()));
        m.insert_color(
            "polkit_pw_bg",
            SceneColor::Raw(if pw_focus {
                ui::hover_hl(&pal)
            } else {
                ui::hover(&pal)
            }),
        );
        let pw_empty = self.polkit_pw.is_empty();
        m.insert(
            "polkit_pw_disp",
            SceneValue::Text(if pw_empty {
                "Password".to_string()
            } else {
                "*".repeat(self.polkit_pw.len())
            }),
        );
        m.insert_color(
            "polkit_pw_ink",
            SceneColor::Raw(if pw_empty { ui::fg3(&pal) } else { pal.fg }),
        );
        m.insert("polkit_cursor_on", SceneValue::Toggle(pw_focus));
        m.insert(
            "polkit_cursor_x",
            SceneValue::Ring(24.0 + 12.0 + self.polkit_pw.len() as f32 * 13.0 * 0.62),
        );
        let pk_err = self.polkit_error.clone();
        m.insert("polkit_err_on", SceneValue::Toggle(pk_err.is_some()));
        m.insert("polkit_err", SceneValue::Text(pk_err.unwrap_or_default()));
        m.insert_color("polkit_err_ink", SceneColor::Raw(RED));
        m.insert_color(
            "polkit_cancel_bg",
            SceneColor::Raw(if self.hover_key == polkit_auth::POLKIT_KEY_CANCEL {
                ui::hover_hl(&pal)
            } else {
                ui::hover(&pal)
            }),
        );
        let auth_hov = self.hover_key == polkit_auth::POLKIT_KEY_AUTH;
        m.insert_color(
            "polkit_auth_bg",
            SceneColor::Raw(if auth_hov {
                mix(pal.acc, pal.fg, 0.15)
            } else {
                pal.acc
            }),
        );
        m.insert(
            "polkit_auth_label",
            SceneValue::Text(if self.polkit_checking {
                "Verifying…".to_string()
            } else {
                "Authenticate".to_string()
            }),
        );
        // ── Wi-Fi submenu rows ── the panel's own row geometry (36 px rows on
        // a 42 px stride) with the drawer's wash chain published per row (it
        // is hover-FIRST, unlike the card's connected-first chain, so it can't
        // ride `hover_surface`), plus the composited "{lock} {ssid}" label the
        // drawer builds itself (the padlock is a PREFIX, and the `suffix` cell
        // kind only hangs a trailing glyph).
        let mut wifi_rows = Vec::new();
        for (d, (ssid, strength, secured, connected)) in
            self.wifi_networks.iter().take(18).enumerate()
        {
            let key = 10 + d as u32;
            let hov = self.hover_key == key;
            // the drawer's own wash ladder, published under a per-row binding
            // name so the row can name it back
            publish_row_wash(&mut m, &format!("wifi_wash_{d}"), hov, *connected, &pal);
            wifi_rows.push(SceneRow {
                cols: vec![
                    strength.to_string(),
                    if *secured {
                        format!("{ICON_LOCK} {ssid}")
                    } else {
                        ssid.clone()
                    },
                    if *connected {
                        ICON_CHECK.to_string()
                    } else {
                        String::new()
                    },
                ],
                key,
                action: None,
                color: None,
                surface: Some(ColorToken::Value(
                    crate::ui::DynColor::new(&format!("wifi_wash_{d}")).unwrap(),
                )),
                col_colors: vec![None, None, Some(ColorToken::Acc)],
            });
        }
        m.insert("wifi_menu_rows", SceneValue::Rows(wifi_rows));
        let wifi_empty = self.wifi_networks.is_empty();
        m.insert("wifi_empty_on", SceneValue::Toggle(wifi_empty));
        m.insert(
            "wifi_empty",
            SceneValue::Text(if self.wifi_on {
                "Scanning…".to_string()
            } else {
                "Wi-Fi is off".to_string()
            }),
        );
        // ── Bluetooth submenu rows ── same geometry, a name column and the
        // connection state in its own ink (accent / fg3).
        let mut bt_rows = Vec::new();
        for (d, (name, connected, _paired)) in self.bt_devices.iter().take(18).enumerate() {
            let key = 10 + d as u32;
            let hov = self.hover_key == key;
            publish_row_wash(&mut m, &format!("bt_wash_{d}"), hov, *connected, &pal);
            let state_ink = format!("bt_state_{d}");
            m.insert_color(
                &state_ink,
                SceneColor::Raw(if *connected { pal.acc } else { ui::fg3(&pal) }),
            );
            bt_rows.push(SceneRow {
                cols: vec![
                    name.clone(),
                    if *connected {
                        "Connected".to_string()
                    } else {
                        "Tap to connect".to_string()
                    },
                ],
                key,
                action: None,
                color: None,
                surface: Some(ColorToken::Value(
                    crate::ui::DynColor::new(&format!("bt_wash_{d}")).unwrap(),
                )),
                col_colors: vec![
                    None,
                    Some(ColorToken::Value(
                        crate::ui::DynColor::new(&state_ink).unwrap(),
                    )),
                ],
            });
        }
        m.insert("bt_menu_rows", SceneValue::Rows(bt_rows));
        let bt_empty = self.bt_devices.is_empty();
        m.insert("bt_empty_on", SceneValue::Toggle(bt_empty));
        m.insert(
            "bt_empty",
            SceneValue::Text(if self.bt_on {
                "No devices found".to_string()
            } else {
                "Bluetooth is off".to_string()
            }),
        );
        // ── battery cards (batteryh / batteryv) ── both cards are the same
        // two-pane shape: pane 0 the vector gauge + percent, pane 1 the info
        // rows + the power-save pill. Everything is per-frame because the
        // charge, the AC bolt, the level ink and the pill's three states all
        // move; the pane gates let the scene pick the pane declaratively.
        let pct = self.battery;
        let present = pct >= 0;
        m.insert("batt_pct", SceneValue::Text(pct.to_string()));
        // "86%", plus the bolt while on AC — the drawer's own charge suffix
        m.insert(
            "batt_pct_txt",
            SceneValue::Text(if present {
                format!("{pct}%{}", if self.ac_online { " \u{f0e7}" } else { "" })
            } else {
                String::new()
            }),
        );
        m.insert_color(
            "batt_ink",
            SceneColor::Raw(crate::scene::battery_level_color(&pal, pct)),
        );
        m.insert("batt_h_pane", SceneValue::Ring(self.battery_h_pane as f32));
        m.insert("batt_v_pane", SceneValue::Ring(self.battery_v_pane as f32));
        // the pane gates fold in the no-battery guard: the drawer's
        // `battery_frame` returns early, so with no battery the body of BOTH
        // panes and the dots are all absent and only the empty caption shows
        m.insert("batt_absent", SceneValue::Toggle(!present));
        m.insert(
            "batt_h_pane_0",
            SceneValue::Toggle(present && self.battery_h_pane == 0),
        );
        m.insert(
            "batt_h_pane_1",
            SceneValue::Toggle(present && self.battery_h_pane == 1),
        );
        m.insert(
            "batt_v_pane_0",
            SceneValue::Toggle(present && self.battery_v_pane == 0),
        );
        m.insert(
            "batt_v_pane_1",
            SceneValue::Toggle(present && self.battery_v_pane == 1),
        );
        m.insert(
            "batt_h_hover",
            SceneValue::Toggle(present && self.battery_dots_hover(self.battery_h_rect)),
        );
        m.insert(
            "batt_v_hover",
            SceneValue::Toggle(present && self.battery_dots_hover(self.battery_v_rect)),
        );
        // pane 1: the four metric rows (no regions — the pill owns the click)
        let state = if self.ac_online {
            "Charging \u{26a1}"
        } else {
            "Discharging"
        };
        let draw = if self.battery_watts > 0.0 {
            format!("{:.1} W", self.battery_watts)
        } else {
            "\u{2014}".to_string()
        };
        let remaining = if self.battery_time.is_empty() {
            "\u{2014}".to_string()
        } else {
            self.battery_time.clone()
        };
        m.insert(
            "batt_info",
            SceneValue::Rows(vec![
                batt_row("State", state),
                batt_row("Battery", &format!("{pct}%")),
                batt_row("Draw", &draw),
                batt_row("Remaining", &remaining),
            ]),
        );
        // the power-save pill: 3-state background, accent label while on, and
        // the glyph+label swap the drawer spells as one string
        let psv = self.power_save_state();
        m.insert_color(
            "batt_psave_bg",
            SceneColor::Raw(if psv {
                ui::acc_tint(&pal)
            } else if self.hover_key == BATTERY_PSAVE_KEY {
                ui::raised_hl(&pal)
            } else {
                ui::raised(&pal)
            }),
        );
        m.insert_color(
            "batt_psave_ink",
            SceneColor::Raw(if psv { pal.acc } else { pal.fg }),
        );
        m.insert(
            "batt_psave_lbl",
            SceneValue::Text(format!(
                "{} {}",
                if psv {
                    ICON_REFRESH
                } else {
                    ICON_BATTERY_SAVER
                },
                if psv { "Power save on" } else { "Power save" }
            )),
        );
        // ── sliders card row inks ── a muted volume/mic row paints its fill
        // AND glyph danger-red (the drawer's `RED` branch), and the kelvin
        // fader rides warn→accent by warmth; both are per-frame blends the
        // scene can't spell as a token.
        m.insert_color(
            "vol_fill",
            SceneColor::Raw(if self.volume_muted { RED } else { pal.acc }),
        );
        m.insert_color(
            "mic_fill",
            SceneColor::Raw(if self.mic_muted { RED } else { pal.acc }),
        );
        m.insert_color(
            "kel_fill",
            SceneColor::Raw(mix(ui::WARN, pal.acc, kel_warm)),
        );
        // pane-1 switch rows tint on hover — the drawer's `hl(key, 0, hover)`,
        // which pushes NO rect at rest, so a fully transparent `0` is the
        // declared equivalent and keeps the hit region key-driven.
        m.insert_color(
            "sliders_tog_bg_0",
            SceneColor::Raw(if self.hover_key == SLIDERS_TOGGLE_BALANCE_KEY {
                ui::hover(&pal)
            } else {
                0
            }),
        );
        m.insert_color(
            "sliders_tog_bg_1",
            SceneColor::Raw(if self.hover_key == SLIDERS_TOGGLE_SAT_KEY {
                ui::hover(&pal)
            } else {
                0
            }),
        );
        m.insert_color(
            "sliders_tog_bg_2",
            SceneColor::Raw(if self.hover_key == SLIDERS_TOGGLE_SST_KEY {
                ui::hover(&pal)
            } else {
                0
            }),
        );
        m.insert_color(
            "dnd_chip",
            SceneColor::Raw(if self.dnd { pal.acc } else { ui::hover(&pal) }),
        );
        m.insert_color(
            "dnd_chip_hover",
            SceneColor::Raw(if self.dnd {
                mix(pal.acc, pal.fg, 0.15)
            } else {
                ui::hover_hl(&pal)
            }),
        );
        // label ink tracks the theme directly (sfg on the accent fill, fg
        // otherwise) — a raw value is unnecessary, so bind a live TOKEN.
        m.insert_color(
            "dnd_chip_fg",
            SceneColor::Token(if self.dnd {
                ColorToken::Sfg
            } else {
                ColorToken::Fg
            }),
        );
        // ── lock-surface values (declared chrome binds them by name) ──
        // the fullscreen dim backdrop + the avatar disc fill — both are
        // derived colors `mix` can't express as a token, so pre-resolve them.
        m.insert_color("lock_dim", SceneColor::Raw(0x00000059));
        m.insert_color(
            "lock_avatar",
            SceneColor::Raw(mix(ui::hover(&pal), pal.acc, 0.35)),
        );
        // ── app-shortcut tiles (declared `Grid` scene binds them by name) ──
        // resting tile fill is a per-frame blend the scene can't express as a
        // token (hover mixes toward fg at different ratios per tile state).
        m.insert_color(
            "app_tile_empty",
            SceneColor::Raw(mix(ui::hover(&pal), pal.fg, 0.06)),
        );
        m.insert_color(
            "app_tile",
            SceneColor::Raw(mix(ui::hover(&pal), pal.fg, 0.05)),
        );
        // mirror record button resting fill — red-tinged while recording
        // (Rust parity: mix(RED, hover_hl, 0.15) while rec, hover otherwise)
        m.insert_color(
            "mirror_rec_bg",
            SceneColor::Raw(if self.mirror_rec {
                mix(RED, ui::hover_hl(&pal), 0.15)
            } else {
                ui::hover(&pal)
            }),
        );
        // apps header count ("3/8") — the declared Header meta substitutes it
        let app_filled = self.app_shortcuts.iter().filter(|s| !s.is_empty()).count();
        m.insert("app_count", SceneValue::Text(format!("{app_filled}/8")));
        // the avatar's initial (first letter, upper-cased — the lock Rust
        // computes it identically from `auth::current_user()`)
        m.insert(
            "user_init",
            SceneValue::Text(
                self.username
                    .chars()
                    .next()
                    .map(|c| c.to_uppercase().collect())
                    .unwrap_or_else(|| "?".into()),
            ),
        );
        // the lock's hero date — `%d` zero-padded day, matching Rust exactly;
        // distinct from the shared `date` (`%e` space-padded) card scenes use.
        m.insert("lock_date", SceneValue::Text(self.date_str("%A, %d %B")));
        // ── banner tokens (the named `dashboard` prerequisite) ──
        // the strip is too state-live for static chrome: edit mode, a lifted
        // token, and the L/C/R zone split all shift the picture per frame.
        // These are the LIVE bindings a future declarative banner needs:
        //   * drop-guide tints (dim = idle column bar, bright = the column
        //     under a lifted token) — Rust computes guide geometry, scenes
        //     bind the two inks + the lift/zone toggles to show/hide them;
        //   * the zone-split + overflow/collapsed/edit flags that change the
        //     whole strip shape (zoned clip vs whole-strip scroll vs hidden).
        m.insert_color(
            "banner_guide",
            SceneColor::Raw(mix(ui::hover(&pal), pal.acc, 0.35)),
        );
        m.insert_color(
            "banner_guide_dim",
            SceneColor::Raw(mix(ui::hover(&pal), pal.fg, 0.10)),
        );
        // the viewing strip band: opaque theme bg + "strip holds chips" gate
        // (the declared band suppresses the Rust one while the surface draws)
        m.insert_color(
            "dash_opaque",
            SceneColor::Raw((self.sv_bg & 0xffffff00) | 0xff),
        );
        m.insert("banner_shown", SceneValue::Toggle(!self.banner_collapsed()));
        let has_zone = self.banner_order.iter().any(|t| t.as_zone().is_some());
        m.insert("banner_editing", SceneValue::Toggle(self.dash_edit));
        m.insert(
            "banner_drag",
            SceneValue::Toggle(self.banner_drag.is_some()),
        );
        m.insert(
            "banner_zones",
            SceneValue::Toggle(has_zone && !self.banner_strip_overflow()),
        );
        m.insert(
            "banner_collapsed",
            SceneValue::Toggle(self.banner_collapsed()),
        );
        m.insert(
            "banner_overflow",
            SceneValue::Toggle(self.banner_strip_overflow()),
        );
        // the cells themselves are stamped by the dashboard draw path (they
        // need the strip's resolved width) and only for an expanded,
        // un-collapsed strip — so this name is conditionally published while
        // the declarative `BannerRow` binding sits in the scene tree always
        m.declare_conditional("banner_cells_v");
        // ── banner viewing chip inks (the declarative `BannerRow` resolves
        // these per cell) — each mirrors its Rust drawer arm's color ladder ──
        m.insert_color("bchip_acc", SceneColor::Token(ColorToken::Acc));
        m.insert_color("bchip_dim", SceneColor::Token(ColorToken::Sfg));
        m.insert_color(
            "bchip_workspaces",
            SceneColor::Raw(if self.hover_key == BANNER_KEY_BASE {
                pal.acc
            } else {
                pal.fg
            }),
        );
        m.insert_color("bchip_title", SceneColor::Token(ColorToken::Fg));
        m.insert_color("bchip_date", SceneColor::Token(ColorToken::Fg));
        m.insert_color("bchip_clock", SceneColor::Token(ColorToken::Fg));
        m.insert_color("bchip_tray", SceneColor::Token(ColorToken::Fg));
        m.insert_color("bchip_settings", SceneColor::Token(ColorToken::Fg));
        m.insert_color("bchip_power", SceneColor::Token(ColorToken::Fg));
        m.insert_color("bchip_search", SceneColor::Token(ColorToken::Fg));
        m.insert_color(
            "bchip_wifi",
            SceneColor::Token(if self.wifi_on {
                if self.ssid.is_empty() {
                    ColorToken::Fg
                } else {
                    ColorToken::Acc
                }
            } else {
                ColorToken::Sfg
            }),
        );
        m.insert_color(
            "bchip_bt",
            SceneColor::Token(if self.bt_on {
                ColorToken::Acc
            } else {
                ColorToken::Fg
            }),
        );
        m.insert_color(
            "bchip_conn",
            SceneColor::Token(if self.wifi_on && !self.ssid.is_empty() {
                ColorToken::Acc
            } else if self.wifi_on || self.bt_on {
                ColorToken::Fg
            } else {
                ColorToken::Sfg
            }),
        );
        m.insert_color(
            "bchip_battery",
            SceneColor::Raw(if self.battery <= 20 && !self.ac_online {
                RED
            } else {
                pal.fg
            }),
        );
        // weather: dim until the service answers (glyph + temp in one ink)
        m.insert_color(
            "bchip_weather",
            SceneColor::Token(if self.banner_weather_text().is_some() {
                ColorToken::Fg
            } else {
                ColorToken::Sfg
            }),
        );
        m.insert_color(
            "bchip_cpu",
            SceneColor::Raw(if self.cpu >= 90 {
                RED
            } else if self.cpu >= 70 {
                0xf9e2afff
            } else {
                pal.fg
            }),
        );
        m.insert_color(
            "bchip_ram",
            SceneColor::Raw(if self.mem_pct >= 90 {
                RED
            } else if self.mem_pct >= 75 {
                0xf9e2afff
            } else {
                pal.fg
            }),
        );
        m.insert_color("bchip_net", SceneColor::Token(ColorToken::Acc));
        m.insert_color(
            "bchip_dnd",
            SceneColor::Token(if self.dnd {
                ColorToken::Acc
            } else {
                ColorToken::Fg
            }),
        );
        m.insert_color(
            "bchip_volume",
            SceneColor::Token(if self.volume_muted {
                ColorToken::Sfg
            } else {
                ColorToken::Fg
            }),
        );
        m.insert_color("bchip_bright", SceneColor::Token(ColorToken::Fg));
        m.insert_color(
            "bchip_vpn",
            SceneColor::Token(if self.ssh_vpn_lines.is_empty() {
                ColorToken::Sfg
            } else {
                ColorToken::Acc
            }),
        );
        m.insert_color("bchip_branding", SceneColor::Token(ColorToken::Fg));
        m.insert_color("bchip_sep", SceneColor::Token(ColorToken::Sfg));
        // activewin: twin hero states — empty (dim glyph + "No window") vs
        // live (app class · title · bottom-anchored per-app RAM pill)
        m.insert(
            "active_win_empty",
            SceneValue::Toggle(self.active_win_class.is_empty()),
        );
        m.insert(
            "active_win_has",
            SceneValue::Toggle(!self.active_win_class.is_empty()),
        );
        m.insert("aw_class", SceneValue::Text(self.active_win_class.clone()));
        m.insert(
            "aw_title",
            SceneValue::Text(self.active_win_title.chars().take(28).collect()),
        );
        m.insert("aw_has_rss", SceneValue::Toggle(self.active_win_rss_kb > 0));
        m.insert(
            "aw_rss",
            SceneValue::Text(if self.active_win_rss_kb > 0 {
                let kb = self.active_win_rss_kb;
                let (val, unit) = if kb >= 1_048_576 {
                    (kb as f32 / 1_048_576.0, "GB")
                } else {
                    (kb as f32 / 1024.0, "MB")
                };
                format!("{} {:.1} {}", ICON_GAUGE_VALUE, val, unit)
            } else {
                String::new()
            }),
        );
        // pkgupdates: hero count figure — `pkg_pending` (updates to install)
        // accents the DOWN glyph, `pkg_ok` draws the CHECK in ok green
        m.insert("pkg_pending", SceneValue::Toggle(self.pkg_update_count > 0));
        m.insert("pkg_ok", SceneValue::Toggle(self.pkg_update_count == 0));
        m.insert(
            "pkg_count",
            SceneValue::Text(self.pkg_update_count.to_string()),
        );
        // clipboard: one row per clip (32-char cap) — the previously-copied
        // row keeps the persistent hover_hl well + accent name, hover lifts
        // any row to the same well/accent (Rust parity — hover_fg maps to
        // Acc like the recent card); the wheel window follows `clip_scroll`
        // so regions stay keyed `CLIP_KEY_BASE + visible index` for the
        // click-to-copy plumbing.
        m.insert(
            "clip_rows",
            SceneValue::Rows(
                self.clip_text
                    .iter()
                    .enumerate()
                    .map(|(i, text)| {
                        let j = i.saturating_sub(self.clip_scroll) as u32;
                        let is_sel = self.clip_sel == i;
                        let is_hov = self.hover_key == crate::shell::CLIP_KEY_BASE + j;
                        let label: String = text.chars().take(32).collect();
                        SceneRow {
                            cols: vec![label],
                            key: 0,
                            action: None,
                            color: None,
                            col_colors: vec![if is_sel || is_hov {
                                Some(ColorToken::Acc)
                            } else {
                                None
                            }],
                            surface: if is_sel {
                                Some(ColorToken::HoverHl)
                            } else {
                                None
                            },
                        }
                    })
                    .collect(),
            ),
        );
        m.insert("clip_scroll", SceneValue::Ring(self.clip_scroll as f32));
        m.insert("clip_empty", SceneValue::Toggle(self.clip_text.is_empty()));
        // docker: one row per container (name, right-anchored 20-char image
        // label, and a status dot — rendered as a full 6×6 bar cell carrying
        // the Ok"Up"/Danger blob the Rust drawer paints as a colored rect)
        m.insert(
            "docker_rows",
            SceneValue::Rows(
                self.docker_containers
                    .iter()
                    .map(|(name, status, image)| SceneRow {
                        cols: vec![
                            "1.0".to_string(),
                            name.clone(),
                            image.chars().take(20).collect(),
                        ],
                        key: 0,
                        action: None,
                        color: None,
                        col_colors: vec![
                            Some(if status.contains("Up") {
                                ColorToken::Ok
                            } else {
                                ColorToken::Danger
                            }),
                            Some(ColorToken::Fg),
                            Some(ColorToken::Fg),
                        ],
                        surface: None,
                    })
                    .collect(),
            ),
        );
        m.insert(
            "docker_empty",
            SceneValue::Toggle(self.docker_containers.is_empty()),
        );
        // the resting bar's own model: its strings, its ink ladder and its
        // per-widget keys, for the `pill` surface in `shell.ron`
        self.publish_pill(&mut m);
        // This pool is the WHOLE world a declarative scene can see, so the
        // loader is allowed to ask it "does anybody publish this name?" — a
        // card that builds a partial pool for itself cannot answer that
        // question about the rest of the shell.
        m.mark_complete();
        m
    }
}

/// Peak of a `(down, up)` history deque (bytes), used to normalize net/disk
/// spark series to 0..1. 0/empty → 1 so a division never yields NaN.
fn peak_of_pairs(rows: &[(f32, f32)]) -> f32 {
    rows.iter()
        .flat_map(|&(a, b)| [a, b])
        .fold(0.0f32, f32::max)
        .max(1.0)
}

/// one label/value `SceneRow` for the battery cards' pane 1.
fn batt_row(label: &str, value: &str) -> crate::scene::SceneRow {
    crate::scene::SceneRow {
        cols: vec![label.to_string(), value.to_string()],
        ..Default::default()
    }
}

/// Publish one submenu row's background wash under a per-row binding name, so
/// the row can name it back with `value("…")` / `DynColor`. The chain is the
/// submenu drawers' own — hover FIRST (a hover on the connected row still
/// lifts), then the connected accent tint, then the plain raised fill — which
/// is why it can't ride `hover_surface`: the `Rows` arm resolves a published
/// `row.surface` before the hover token, so a connected row would lose its
/// tint under the pointer.
fn publish_row_wash(
    vals: &mut crate::scene::SceneValues,
    name: &str,
    hov: bool,
    connected: bool,
    pal: &crate::ui::Pal,
) {
    vals.insert_color(
        name,
        crate::scene::SceneColor::Raw(if hov {
            crate::ui::raised_hl(pal)
        } else if connected {
            crate::ui::acc_tint(pal)
        } else {
            crate::ui::raised(pal)
        }),
    );
}

/// Seed `vals` with the $sources/lib module values. Called at the very top of
/// [`Shell::scene_values`] so the built-ins that follow win any collision —
/// that ordering is the whole contract, so it lives in one named function with
/// a test rather than as three lines inside a 200-line builder.
fn merge_module_values(vals: &mut crate::scene::SceneValues, values: Vec<(String, String)>) {
    use crate::scene::SceneValue;
    for (k, v) in values {
        vals.insert(k, SceneValue::Text(v));
    }
}

#[cfg(test)]
mod module_value_tests {
    use super::*;

    /// A module value is reachable by `{key}` interpolation, dots and all.
    #[test]
    fn module_values_interpolate_by_their_dotted_key() {
        let mut vals = crate::scene::SceneValues::new();
        merge_module_values(
            &mut vals,
            vec![("mod_clock.time".into(), "16:24:46".into())],
        );
        assert_eq!(vals.text("mod_clock.time"), Some("16:24:46"));
        assert_eq!(
            crate::scene::substitute("{mod_clock.time} o'clock", &vals),
            "16:24:46 o'clock"
        );
    }

    /// A module cannot shadow a built-in: the built-in insert comes later and
    /// overwrites whatever the module published under that name.
    #[test]
    fn built_ins_win_a_key_collision() {
        let mut vals = crate::scene::SceneValues::new();
        merge_module_values(
            &mut vals,
            vec![
                ("clock".into(), "hijacked".into()),
                ("mod_x.v".into(), "kept".into()),
            ],
        );
        vals.insert("clock", crate::scene::SceneValue::Text("22:09:00".into()));
        assert_eq!(vals.text("clock"), Some("22:09:00"), "built-in must win");
        assert_eq!(vals.text("mod_x.v"), Some("kept"), "additive keys survive");
    }

    /// No `$sources/lib` at all is the normal case for anyone who has not
    /// installed a module — it must be a no-op, not a panic.
    #[test]
    fn no_modules_is_a_no_op() {
        let mut vals = crate::scene::SceneValues::new();
        merge_module_values(&mut vals, Vec::new());
        assert_eq!(
            crate::scene::substitute("{mod_clock.time}", &vals),
            "{mod_clock.time}"
        );
    }
}
