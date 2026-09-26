//! Per-panel scene layouts (one `layout_*` per mode).

use super::*;
mod app_panels;
mod cards;
mod dashboard;
pub(crate) mod layout;
mod overlays;
mod polkit_auth;
mod session;
mod settings;
mod system_panels;

impl Shell {

    pub fn layout(&mut self, out_w: f32, out_h: f32) -> Vec<Cmd> {
        self.layout_for(self.mode, out_w, out_h)
    }

    /// Layout the scene for a specific mode (not necessarily the current one).
    /// The session-lock surface renders `Mode::Lock` while the bar itself
    /// renders its resting mode behind it.
    pub fn layout_for(&mut self, mode: Mode, out_w: f32, out_h: f32) -> Vec<Cmd> {
        self.hover_regions.clear();
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
                    Cmd::Rect { x, y, .. } | Cmd::RectConcave { x, y, .. } | Cmd::Outline { x, y, .. } | Cmd::Text { x, y, .. } | Cmd::Image { x, y, .. } | Cmd::Scissor { x, y, .. } | Cmd::Map { x, y, .. } => {
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
            ui::text(v, lx, cy, self.clock.clone(), 14.0, if hov { pal.acc } else { pal.fg }, false);
            let cw = 8.0 + self.clock.chars().count() as f32 * 14.0 * 0.62;
            self.region(lx - 2.0, 4.0, cw + 4.0, h - 8.0, key);
            lx += cw + g;
        }
        {
            let dtext = self.date_str("%a %d");
            let hov = self.hover_key == 72;
            let dw = dtext.chars().count() as f32 * 14.0 * 0.62;
            ui::text(v, lx, cy, dtext, 14.0, if hov { pal.acc } else { pal.fg }, false);
            self.region(lx - 2.0, 4.0, dw + 4.0, h - 8.0, 72);
            lx += dw + g;
        }
        if self.pill_battery && self.battery >= 0 {
            let key = 71;
            let hov = self.hover_key == key;
            let low = self.battery <= 20 && !self.ac_online;
            let ic = if low { RED } else if hov { pal.acc } else { pal.fg };
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
            lx += bw + g;
        }
        let left_end = lx - g; // right edge of the last left item

        // ── right-cluster: bell + brightness + tray — anchored so the last
        // visible item's right edge sits at `w - mx` (right margin == left). ──
        let right_edge = w - mx;
        let tray_n = if self.pill_tray { self.tray.len().min(6) } else { 0 };
        let mut n_right: usize = 0;
        let mut right_used = 0.0;
        if self.pill_notif { right_used += 22.0; n_right += 1; }
        if self.pill_brightness { right_used += 22.0; n_right += 1; }
        if tray_n > 0 { right_used += tray_n as f32 * 22.0; n_right += 1; }
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
                v.push(Cmd::Rect { x: bx, y: 8.0, w: 22.0, h: 22.0, r: 11.0, color: ui::hover(&pal) });
            }
            ui::text_r(v, bx + 22.0, cy, ICON_BELL, 14.0, pal.fg, true);
            self.region(bx, 0.0, 22.0, h, 62);
            if self.notif_count > 0 {
                v.push(Cmd::Rect { x: bx + 2.0, y: 5.0, w: 6.0, h: 6.0, r: 3.0, color: pal.acc });
            }
            rx += 22.0 + g;
        }
        // brightness / dark-mode toggle (sun ⇄ moon)
        if self.pill_brightness {
            let dx = rx;
            let hov = self.hover_key == 67;
            if hov {
                v.push(Cmd::Rect { x: dx, y: 8.0, w: 22.0, h: 22.0, r: 11.0, color: ui::hover_hl(&pal) });
            }
            let glyph = if self.dark_mode { ICON_MOON } else { ICON_BRIGHTNESS };
            ui::text(v, dx + 5.0, cy, glyph, 14.0, if hov { pal.acc } else { pal.fg }, true);
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
                    v.push(Cmd::Rect { x: cx2, y: 9.0, w: 20.0, h: 20.0, r: 10.0, color: ui::hover_hl(&pal) });
                }
                let item = &self.tray[i];
                let ik = item.image_key();
                if ik.is_empty() {
                    ui::text(v, cx2 + 6.5, 13.5, ICON_DOT, 8.0, ui::fg2(&pal), true);
                } else {
                    v.push(Cmd::Image { x: cx2 + 2.0, y: 11.0, w: 16.0, h: 16.0, key: ik });
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
            .filter(|(idx, &it)| it != PillItem::Clock && it != PillItem::Battery && widths[*idx] > 0.0)
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
                PillItem::Workspaces
        | PillItem::WorkspacesShort => {
                    // only the ACTIVE workspace number — always current, works
                    // for any workspace id (8/9/10 …), never a fixed 1..3 row
                    let txt = (self.ws_active + 1).to_string();
                    let key = 50 + slot as u32 * 10;
                    let hov = self.hover_key == key;
                    ui::text(v, cx + 2.0, cy, &txt, 14.0, if hov { pal.acc } else { pal.fg }, false);
                    self.region(cx - 4.0, 8.0, if txt.len() > 1 { 26.0 } else { 20.0 }, h - 12.0, key);
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
                        let is_active = n as usize == self.ws_active + 1;
                        if hov {
                            v.push(Cmd::Rect { x: cx2, y: 7.0, w: 18.0, h: 24.0, r: 6.0, color: ui::hover_hl(&pal) });
                        }
                        ui::text(v, cx2 + 6.0, cy, &n.to_string(), 14.0, pal.fg, false);
                        if is_active {
                            v.push(Cmd::Rect { x: cx2 + 7.5, y: 32.0, w: 3.0, h: 3.0, r: 1.5, color: pal.acc });
                        }
                        self.region(cx2, 7.0, 18.0, 24.0, key);
                    }
                    if self.ws_active + 1 > 5 {
                        let cx2 = cx + 5.0 * (18.0 + 2.0) + 2.0;
                        let txt = (self.ws_active + 1).to_string();
                        let key = 50 + slot as u32 * 10 + 5;
                        let hov = self.hover_key == key;
                        if hov {
                            v.push(Cmd::Rect { x: cx2 - 2.0, y: 7.0, w: if txt.len() > 1 { 22.0 } else { 16.0 }, h: 24.0, r: 6.0, color: ui::hover_hl(&pal) });
                        }
                        ui::text(v, cx2 + 2.0, cy, &txt, 14.0, pal.fg, false);
                        let tw = txt.len() as f32 * 14.0 * 0.62;
                        v.push(Cmd::Rect { x: cx2 + 2.0 + tw * 0.5 - 1.5, y: cy + 14.0 + 2.0, w: 3.0, h: 3.0, r: 1.5, color: pal.acc });
                        self.region(cx2 - 4.0, 8.0, if txt.len() > 1 { 26.0 } else { 20.0 }, h - 12.0, key);
                    }
                }
                PillItem::Wifi => {
                    let key = 50 + slot as u32 * 10;
                    let hov = self.hover_key == key;
                    let c = if hov { pal.acc } else if !self.ssid.is_empty() { pal.acc } else { pal.fg };
                    ui::text(v, cx + 4.0, cy, ICON_WIFI, 14.0, c, true);
                    self.region(cx - 2.0, 4.0, iw + 4.0, h - 8.0, key);
                }
                PillItem::Bluetooth => {
                    let key = 50 + slot as u32 * 10;
                    let hov = self.hover_key == key;
                    let c = if hov { pal.acc } else if !self.bt_devices.is_empty() { pal.acc } else { pal.fg };
                    ui::text(v, cx + 4.0, cy, ICON_BLUETOOTH, 14.0, c, true);
                    self.region(cx - 2.0, 4.0, iw + 4.0, h - 8.0, key);
                }
                PillItem::Volume => {
                    let key = 50 + slot as u32 * 10;
                    let hov = self.hover_key == key;
                    ui::text(v, cx + 4.0, cy, ICON_VOLUME, 14.0, if hov { pal.acc } else { pal.fg }, true);
                    self.region(cx - 2.0, 4.0, iw + 4.0, h - 8.0, key);
                }
                PillItem::Wallpaper => {
                    let key = 50 + slot as u32 * 10;
                    let hov = self.hover_key == key;
                    ui::text(v, cx + 4.0, cy, ICON_WALLPAPER, 14.0, if hov { pal.acc } else { pal.fg }, true);
                    self.region(cx - 2.0, 4.0, iw + 4.0, h - 8.0, key);
                }
                PillItem::Themes => {
                    let key = 50 + slot as u32 * 10;
                    let hov = self.hover_key == key;
                    ui::text(v, cx + 4.0, cy, ICON_PALETTE, 14.0, if hov { pal.acc } else { pal.fg }, true);
                    self.region(cx - 2.0, 4.0, iw + 4.0, h - 8.0, key);
                }
                PillItem::Branding => {
                    // display-only brand mark — no hit region, no hover; the
                    // glyph renders in the brand family, centered in its chip
                    ui::text_cw(v, cx + iw / 2.0, cy, &self.brand_glyph, 16.0, crate::text::Ff::Brand, crate::text::Fw::Regular, pal.fg, false);
                }
                PillItem::Watts => {
                    let key = 50 + slot as u32 * 10;
                    let hov = self.hover_key == key;
                    let c = if hov { pal.acc } else { pal.fg };
                    ui::text(v, cx, cy, ICON_BOLT, 14.0, c, true);
                    ui::text(v, cx + 14.0, cy, format!("{:.1}W", self.watts_shown), 14.0, c, false);
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
                        let col = if i % 4 == 0 { pal.acc } else { mix(pal.acc, pal.fg, 0.35) };
                        v.push(Cmd::Rect { x: bx, y: bar_bottom - bh, w: 2.0, h: bh, r: 1.0, color: col });
                    }
                    let key = 50 + slot as u32 * 10;
                    self.region(cx - 2.0, 4.0, n_bars as f32 * 3.0 + 4.0, h - 8.0, key);
                }
                PillItem::User => {
                    let key = 50 + slot as u32 * 10;
                    let hov = self.hover_key == key;
                    ui::text(v, cx, cy, &self.username, 14.0, if hov { pal.acc } else { pal.fg }, false);
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
                color: mix(ui::hover(&pal), pal.acc, 0.20),
            });
        }
    }
}
