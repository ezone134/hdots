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
        self.hover_regions.clear();
        self.scene_click_actions.clear();
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
        let scene = self.scene_cache.get(card_id).cloned();
        let Some(scene) = scene else {
            return false;
        };
        // A fully declarative scene (no `Ink` body) skips the Rust drawer, so
        // the card's wheel-scroll rect (normally set inside the drawer) must
        // be tracked here to keep pointer-wheel scrolling alive.
        if !scene.items.iter().any(|it| matches!(it, crate::scene::SceneItem::Ink { .. })) {
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
        self.scene_owns_rows = scene
            .items
            .iter()
            .any(|it| matches!(it, crate::scene::SceneItem::Rows { .. } | crate::scene::SceneItem::TextWrap { .. }));
        let (hits, inks) = scene.draw(v, x, y, card_w, card_h, self.scale, pal, vals, self.hover_key, self.card_show_title, self.card_show_glyph);
        for hit in hits {
            self.region(hit.x, hit.y, hit.w, hit.h, hit.key);
            if let Some(action) = hit.action {
                self.scene_click_actions.insert(hit.key, action);
            }
            // Dashboard fader drag math (`set_dash_slider`) reads the geometry
            // the Rust sliders drawer publishes; a declared `Fader` publishes
            // the same rect from its own hit box so dragging a declarative
            // fader keeps working once the drawer stands down.
            match hit.key {
                51..=55 => self.faders[(hit.key - 51) as usize] = (hit.x, hit.y + hit.h / 2.0, hit.w),
                59 => self.balance_rect = (hit.x, hit.y + hit.h / 2.0, hit.w),
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
        let scene = self.shell_scene.get().and_then(|ss| ss.surface(name)).cloned();
        let Some(surface) = scene else {
            return false;
        };
        let card = crate::scene::CardScene { items: surface.items };
        let (hits, inks) = card.draw(
            v, 0.0, 0.0, w, h, self.scale, pal, vals, self.hover_key, false, false,
        );
        for hit in hits {
            self.region(hit.x, hit.y, hit.w, hit.h, hit.key);
            if let Some(action) = hit.action {
                self.scene_click_actions.insert(hit.key, action);
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
                let card = crate::scene::CardScene { items: ink.pre.clone() };
                let ctxvals = match dash {
                    Some(d) => d.vals,
                    None => vals,
                };
                card.draw(v, 0.0, 0.0, w, h, self.scale, pal, ctxvals, self.hover_key, false, false);
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
        cx: f32, cy: f32, cw: f32, ch: f32,
        pal: &Pal,
        vals: &crate::scene::SceneValues,
    ) {
        for ink in inks {
            // declared items authored before this Ink draw at its dispatch
            // slot — declared chrome BETWEEN two Rust bodies in z-order
            if !ink.pre.is_empty() {
                let card = crate::scene::CardScene { items: ink.pre.clone() };
                card.draw(v, cx, cy, cw, ch, self.scale, pal, vals, self.hover_key, self.card_show_title, self.card_show_glyph);
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
    pub(crate) fn scene_values(&self) -> crate::scene::SceneValues {
        use crate::scene::{GridCell, KaraokeLine, SceneColor, SceneRow, SceneTile, SceneValue, SceneValues};
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
        m.insert("battery_watts", SceneValue::Text(format!("{:.0}", self.battery_watts)));
        m.insert("battery_time", SceneValue::Text(self.battery_time.clone()));
        m.insert("volume", SceneValue::Text(format!("{:.0}", self.volume * 100.0)));
        m.insert("brightness", SceneValue::Text(format!("{:.0}", self.brightness * 100.0)));
        m.insert("mic_level", SceneValue::Text(format!("{:.0}", self.mic_level * 100.0)));
        m.insert("cpu", SceneValue::Text(self.cpu.to_string()));
        m.insert("mem", SceneValue::Text(self.mem_pct.to_string()));
        m.insert("disk", SceneValue::Text(self.disk_pct.to_string()));
        m.insert("cpu_temp", SceneValue::Text(format!("{:.0}°", self.cpu_temp)));
        m.insert("gpu_temp", SceneValue::Text(format!("{:.0}°", self.gpu_temp)));
        m.insert("net_up", SceneValue::Text(self.net_up.clone()));
        m.insert("net_down", SceneValue::Text(self.net_down.clone()));
        m.insert("kblayout", SceneValue::Text(if self.kb_layout.is_empty() { "Unknown".into() } else { self.kb_layout.clone() }));
        // header metas — live strings the declarative `Header` chrome can
        // drop in for cards whose meta is dynamic (counts / state labels)
        m.insert(
            "disk_meta",
            SceneValue::Text(if self.disk_parts.is_empty() { String::new() } else { format!("{} mounts", self.disk_parts.len()) }),
        );
        m.insert(
            "cpu_meta",
            SceneValue::Text(if self.cpu_freq_mhz > 0 { format!("{} MHz", self.cpu_freq_mhz) } else { String::new() }),
        );
        m.insert(
            "water_meta",
            SceneValue::Text(format!("{:.1} / {:.1} L", self.water_glasses as f32 * 0.25, self.water_goal.max(1) as f32 * 0.25)),
        );
        m.insert(
            "pomo_meta",
            SceneValue::Text(if self.pomo_sessions > 0 { format!("{} done", self.pomo_sessions) } else { String::new() }),
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
            SceneValue::Text(if self.wallpapers.len() > 0 { format!("{} wallpapers", self.wallpapers.len()) } else { String::new() }),
        );
        m.insert(
            "jr_meta",
            SceneValue::Text(if self.jr_lines.is_empty() { String::new() } else { format!("{} lines", self.jr_lines.len()) }),
        );
        let sm_n = self.sm_disks.len();
        m.insert(
            "sm_meta",
            SceneValue::Text(if sm_n == 0 { String::new() } else { format!("{sm_n} disk{}", if sm_n == 1 { "" } else { "s" }) }),
        );
        m.insert(
            "sm_icon",
            SceneValue::Text(if self.sm_disks.iter().any(|(_, _, ok, _)| !ok) { ICON_SHIELD.into() } else { ICON_CHECK.into() }),
        );
        m.insert(
            "auddev_meta",
            SceneValue::Text(if self.audio_sinks.len() + self.audio_sources.len() > 0 {
                format!("{} dev", self.audio_sinks.len() + self.audio_sources.len())
            } else {
                String::new()
            }),
        );
        m.insert("conn_meta", SceneValue::Text(format!("{} addrs", self.conn_rows.len())));
        m.insert(
            "moon_meta",
            SceneValue::Text({
                let epoch = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0);
                let days = (epoch - 947_182_440.0) / 86_400.0;
                let phase = (days % 29.530_59 / 29.530_59).rem_euclid(1.0);
                let illum = (1.0 - (2.0 * std::f64::consts::PI * phase).cos()) / 2.0;
                format!("{:.0}%", illum * 100.0)
            }),
        );
        m.insert(
            "comp_meta",
            SceneValue::Text({
                let on = [self.fx_blur_on, self.fx_shadow_on, self.fx_opacity_on].iter().filter(|b| **b).count();
                format!("{on} on")
            }),
        );
        m.insert(
            "fan_meta",
            SceneValue::Text(if self.fans_rpm.is_empty() { String::new() } else { format!("{} fans", self.fans_rpm.len()) }),
        );
        m.insert(
            "er_meta",
            SceneValue::Text(if self.er_sessions > 0 { format!("{} rests", self.er_sessions) } else { String::new() }),
        );
        m.insert(
            "thermal_meta",
            SceneValue::Text(if self.thermal_zones.is_empty() { String::new() } else { format!("{} zones", self.thermal_zones.len()) }),
        );
        m.insert(
            "wc_meta",
            SceneValue::Text(if self.worldclock_zones.is_empty() { String::new() } else { format!("{} cities", self.worldclock_zones.len()) }),
        );
        m.insert("mic_meta", SceneValue::Text(format!("{:>3}%", (self.mic_level * 100.0).round() as i32)));
        m.insert(
            "mic_icon",
            SceneValue::Text(if self.mic_muted { ICON_MIC_OFF.into() } else { ICON_MIC.into() }),
        );
        m.insert(
            "sus_meta",
            SceneValue::Text(if self.sus_failed.is_empty() { String::new() } else { format!("{} failed", self.sus_failed.len()) }),
        );
        m.insert(
            "sus_icon",
            SceneValue::Text(if self.sus_failed.is_empty() { ICON_CHECK.into() } else { ICON_SHIELD.into() }),
        );
        m.insert(
            "ws_meta",
            SceneValue::Text({
                let n = self.ws_count.max(1);
                format!("{}/{n}", (self.ws_active + 1).min(n))
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
        m.insert("gau_mem_pct", SceneValue::Text(format!("{}%", self.mem_pct)));
        m.insert("gau_disk_pct", SceneValue::Text(format!("{}%", self.disk_pct)));
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
        let pal = Pal { fg: self.sv_fg, bg: self.sv_bg, acc: self.sv_acc, sfg: self.sv_sfg };
        m.insert_color("gau_dim", SceneColor::Raw(mix(ui::hover(&pal), pal.fg, 0.08)));
        {
            // water: glasses/goal, capped so an over-goal day keeps the ring
            // full instead of wrapping (the drawer's `min(1.0)`)
            let frac = (self.water_glasses as f32 / self.water_goal.max(1) as f32).min(1.0);
            let done = frac >= 1.0;
            m.insert("water_frac", SceneValue::Ring(frac));
            m.insert("water_done", SceneValue::Toggle(done));
            m.insert(
                "water_hero",
                SceneValue::Text(format!("{}{}", self.water_glasses, if done { " \u{2713}" } else { "" })),
            );
            m.insert("water_goal_txt", SceneValue::Text(format!("of {} glasses", self.water_goal)));
            m.insert_color(
                "water_ring_ink",
                SceneColor::Raw(if done { ui::OK } else { ui::INFO }),
            );
            m.insert_color(
                "water_hero_ink",
                SceneColor::Raw(if done { ui::OK } else { pal.fg }),
            );
            m.insert_color("water_dim", SceneColor::Raw(mix(ui::hover(&pal), pal.fg, 0.10)));
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
            m.insert("er_hero", SceneValue::Text(format!("{:02}:{:02}", rem / 60, rem % 60)));
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
                        format!("{:.2}/{:.2}/{:.2}", self.cpu_load.0, self.cpu_load.1, self.cpu_load.2),
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
                        if cpu_freq.is_empty() { "—".to_string() } else { cpu_freq.clone() },
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
                        if self.cpu_temp > 0.0 { format!("{:.0}°C", self.cpu_temp) } else { "—".to_string() },
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
        m.insert("todo_buf", SceneValue::Text(self.todo_input.clone().unwrap_or_default()));
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
        // Rows: the alarms list — time (mono, accent) + label columns, ✕ keys
        // handled scene-side via `Rows { del_base: ALARM_DEL_BASE }`.
        m.insert(
            "alarm_rows",
            SceneValue::Rows(
                self.alarm_list
                    .iter()
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
        // Rows: the notes list — bullet glyph · title · body-caption columns,
        // scroll window, and a `visible` toggle that hides the list while the
        // shared composer is open (it draws in place of the list).
        m.insert(
            "notes_rows",
            SceneValue::Rows(
                self.notes
                    .iter()
                    .map(|(title, body)| crate::scene::SceneRow {
                        cols: vec![crate::icons::ICON_BULLET.to_string(), title.clone(), body.clone()],
                        key: 0,
                        action: None,
                        color: None,
                        surface: None,
                        col_colors: vec![],
                    })
                    .collect(),
            ),
        );
        m.insert("notes_composing", SceneValue::Toggle(self.notes_input.is_some()));
        m.insert("notes_scroll", SceneValue::Ring(self.notes_scroll as f32));
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
                                if passed { Some(crate::ui::ColorToken::Fg3) } else { None },
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
        m.insert("exp_buf", SceneValue::Text(self.expense_input.clone().unwrap_or_default()));
        m.insert("exp_focus", SceneValue::Toggle(self.expense_input.is_some()));
        m.insert("exp_empty", SceneValue::Toggle(exp_per.is_empty()));
        for i in 0..3 {
            let has = exp_per.len() > i;
            let (cat, val) = exp_per.get(i).cloned().unwrap_or_default();
            let name: String = cat.chars().take(7).collect();
            m.insert(format!("exp_has_{}", i + 1), SceneValue::Toggle(has));
            m.insert(format!("exp_cat_{}", i + 1), SceneValue::Text(name));
            m.insert(format!("exp_val_{}", i + 1), SceneValue::Text(format!("{val:.0}")));
            m.insert(format!("exp_bar_{}", i + 1), SceneValue::Text(format!("{:.4}", (val / exp_max).clamp(0.0, 1.0))));
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
                news_cats.get(self.news_active_cat).cloned().unwrap_or_default()
            }
        } else {
            String::new()
        };
        m.insert("news_label", SceneValue::Text(news_label));
        m.insert("news_msg", SceneValue::Text(if self.news_fetched {
            "no feeds — set [[news_feeds]]/news_url or write ~/.cache/zen-shell/news.json".to_string()
        } else {
            "fetching headlines…".to_string()
        }));
        m.insert("news_chips", SceneValue::Chips(news_cats.clone()));
        m.insert("news_cat_sel", SceneValue::Ring(self.news_active_cat as f32));
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
            SceneValue::Text(if snip_n > 0 { format!("{snip_n} clips") } else { String::new() }),
        );
        m.insert("snip_buf", SceneValue::Text(self.snippet_input.clone().unwrap_or_default()));
        m.insert("snip_focus", SceneValue::Toggle(self.snippet_input.is_some()));
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
        m.insert("cpu_spark", SceneValue::Spark(self.cpu_history.iter().copied().collect()));
        m.insert("gpu_spark", SceneValue::Spark(self.gpu_history.iter().copied().collect()));
        m.insert("lat_spark", SceneValue::Spark(self.lat_history.iter().map(|&x| x as f32).collect()));
        // net/disk histories are (bytes read, bytes written) pairs — expose a
        // normalized 0..1 series against the deque's own peak so a Spark
        // without an explicit max just works.
        let mut net_rows: Vec<(f32, f32)> = self.net_history.iter().map(|&(d, u)| (d as f32, u as f32)).collect();
        let net_peak = peak_of_pairs(&net_rows);
        for (d, u) in net_rows.iter_mut() {
            *d /= net_peak;
            *u /= net_peak;
        }
        m.insert("net_up_spark", SceneValue::Spark(net_rows.iter().map(|&(_, u)| u).collect()));
        m.insert("net_down_spark", SceneValue::Spark(net_rows.iter().map(|&(d, _)| d).collect()));
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
        let mut disk_rows: Vec<(f32, f32)> = self.disk_history.iter().map(|&(r, w)| (r as f32, w as f32)).collect();
        let disk_peak = peak_of_pairs(&disk_rows);
        for (r, w) in disk_rows.iter_mut() {
            *r /= disk_peak;
            *w /= disk_peak;
        }
        m.insert("disk_r_spark", SceneValue::Spark(disk_rows.iter().map(|&(r, _)| r).collect()));
        m.insert("disk_w_spark", SceneValue::Spark(disk_rows.iter().map(|&(_, w)| w).collect()));
        // Rows: text-list cards — one data row per line / entry.
        m.insert(
            "jr_rows",
            SceneValue::Rows(
                self.jr_lines.iter().map(|(msg, prio)| {
                    let color = Some(match *prio {
                        0..=3 => crate::ui::ColorToken::Danger,
                        4 => crate::ui::ColorToken::Warn,
                        _ => crate::ui::ColorToken::Fg2,
                    });
                    SceneRow { cols: vec![msg.clone()], key: 0, action: None, color, col_colors: vec![], surface: None }
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
                        SceneRow { cols: vec![msg.clone()], key: 0, action: None, color, col_colors: vec![], surface: None }
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
                                if *passed { ICON_CHECK.into() } else { ICON_CLOSE.into() },
                            ],
                            key: 0,
                            action: None,
                            color: None,
                            col_colors: vec![
                                Some(if *passed { crate::ui::ColorToken::Fg2 } else { crate::ui::ColorToken::Danger }),
                                Some(crate::ui::ColorToken::Fg3),
                                Some(if *temp >= 55 { crate::ui::ColorToken::Warn } else { crate::ui::ColorToken::Fg3 }),
                                Some(if *passed { crate::ui::ColorToken::Ok } else { crate::ui::ColorToken::Danger }),
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
                            Some(if lbl == "gateway" || lbl == "dns" { crate::ui::ColorToken::Fg3 } else { crate::ui::ColorToken::Fg }),
                            Some(if lbl == "gateway" || lbl == "dns" { crate::ui::ColorToken::Fg3 } else { crate::ui::ColorToken::Fg }),
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
                        sub_color: Some(
                            if *temp >= 80.0 {
                                crate::ui::ColorToken::Danger
                            } else if *temp >= 60.0 {
                                crate::ui::ColorToken::Warn
                            } else {
                                crate::ui::ColorToken::Fg2
                            },
                        ),
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
        m.insert("thermal_empty", SceneValue::Toggle(self.thermal_zones.is_empty()));
        m.insert("thermal_live", SceneValue::Toggle(!self.thermal_zones.is_empty()));
        m.insert(
            "conn_rows",
            SceneValue::Rows(self.conn_rows.iter().map(|(k, v)| SceneRow { cols: vec![k.clone(), v.clone()], key: 0, action: None, color: None, col_colors: vec![], surface: None }).collect()),
        );
        // procmon: label rows + per-row CPU-tick fraction of the top process
        // (the scene's bar cell parses it — peak = the first row's ticks, the
        // drawer's implicit ordering makes #1 the 100% reference)
        let proc_top_ticks = self.top_procs.first().map(|(_, t, _)| *t).unwrap_or(0).max(1);
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
                                Some(if live { ColorToken::Fg } else { ColorToken::Fg3 }),
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
        m.insert("sensor_tilt", SceneValue::Text(format!("pitch {:.0}°  roll {:.0}°", s.pitch, s.roll)));
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
            m.insert("sensor_compass", SceneValue::Text(format!("{arrow} {:.0}°", hdg)));
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
                self.bt_devices.iter().map(|(name, connected, paired)| {
                    let state = if *connected { ICON_CHECK.to_string() } else if !*paired { "unpaired".into() } else { String::new() };
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
                        surface: if *connected { Some(ColorToken::AccTint) } else { None },
                    }
                })
                .collect(),
            ),
        );
        m.insert(
            "bt_scroll",
            SceneValue::Ring(self.bt_scroll.min(self.bt_devices.len().saturating_sub(1)) as f32),
        );
        m.insert("bt_chip_on", SceneValue::Text(if self.bt_on { "on".into() } else { String::new() }));
        m.insert("bt_chip_off", SceneValue::Text(if self.bt_on { String::new() } else { "off".into() }));
        m.insert(
            "bt_status",
            SceneValue::Text(if self.bt_devices.is_empty() {
                if self.bt_on { "no paired devices".into() } else { "bluetooth off — enable in toggles".into() }
            } else {
                String::new()
            }),
        );
        // Prices: sym · price · right chg% (green up / red down). The status
        // text shows only when no price data; the window follows `ticker_scroll`.
        m.insert(
            "ticker_rows",
            SceneValue::Rows(
                self.ticker_items.iter().map(|(sym, price, chg)| SceneRow {
                    cols: vec![sym.clone(), price.clone(), format!("{:+.1}%", chg)],
                    key: 0,
                    action: None,
                    color: None,
                    col_colors: vec![None, None, Some(if *chg >= 0.0 { ColorToken::Ok } else { ColorToken::Danger })],
                    surface: None,
                })
                .collect(),
            ),
        );
        m.insert(
            "ticker_scroll",
            SceneValue::Ring(self.ticker_scroll.min(self.ticker_items.len().saturating_sub(1)) as f32),
        );
        m.insert(
            "ticker_status",
            SceneValue::Text(if self.ticker_items.is_empty() {
                if self.ticker_state == 1 { "loading prices…".into() } else { "no price data".into() }
            } else {
                String::new()
            }),
        );
        // Currency: offline reference table, one static row per pair. The
        // currently re-based currency tints its row + sym + value accent; the
        // window follows `currency_scroll`.
        const CCY: [(&str, &str, f64); 12] = [
            ("USD", "Dollar", 1.0), ("EUR", "Euro", 0.9200), ("GBP", "Pound", 0.7900),
            ("JPY", "Yen", 149.50), ("INR", "Rupee", 83.10), ("CNY", "Yuan", 7.2400),
            ("RUB", "Ruble", 92.50), ("AUD", "AU Dollar", 1.5200), ("CAD", "CA Dollar", 1.3700),
            ("KRW", "Won", 1347.00), ("XAU", "Gold oz", 0.000420), ("XBT", "BTC", 0.0000160),
        ];
        let base = self.currency_base.as_str();
        let base_mult = CCY.iter().find(|(s, _, _)| *s == base).map(|(_, _, m)| *m).unwrap_or(1.0);
        m.insert(
            "currency_rows",
            SceneValue::Rows(
                CCY.iter()
                    .map(|&(sym, label, mult)| {
                        let active = sym == base;
                        let val = mult / base_mult;
                        let vstr = if val >= 100.0 { format!("{:.0}", val) } else if val >= 10.0 { format!("{:.1}", val) } else { format!("{:.2}", val) };
                        SceneRow {
                            cols: vec![sym.to_string(), label.to_string(), format!("1 {base} = {vstr} {sym}")],
                            key: 0,
                            action: None,
                            color: None,
                            col_colors: if active {
                                vec![Some(ColorToken::Acc), None, Some(ColorToken::Acc)]
                            } else {
                                vec![None, None, None]
                            },
                            surface: if active { Some(ColorToken::AccTint) } else { None },
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
            SceneValue::Text(if base != "USD" { base.to_string() } else { String::new() }),
        );
        // SSH / VPN: one line per tunnel/connection; the leading glyph flips
        // globe/shield by whether the line names a VPN.
        m.insert(
            "sshvpn_rows",
            SceneValue::Rows(
                self.ssh_vpn_lines.iter().map(|line| SceneRow {
                    cols: vec![if line.starts_with("VPN") { ICON_GLOBE.into() } else { ICON_SHIELD.into() }, line.clone()],
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
            SceneValue::Text(if self.ssh_vpn_lines.is_empty() { "No connections".into() } else { String::new() }),
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
                            .map(|(wi, w)| (w.chars().take(120).collect(), lyr_active == Some(i) && wi == lyr_cur))
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
        m.insert("lyr_empty", SceneValue::Toggle(self.lyrics_lines.is_empty()));
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
        }
        m.insert(
            "recent_scroll",
            SceneValue::Ring(self.recent_scroll.min(self.recent_files.len().saturating_sub(1)) as f32),
        );
        m.insert(
            "recent_status",
            SceneValue::Text(if self.recent_files.is_empty() { "no recent files tracked".into() } else { String::new() }),
        );
        // Settings sidebar nav: glyph + label per tab. Per-frame row `surface`
        // + `col_colors` carry the selected/hoarked states (selected = accent
        // glyph on HoverHl, hover = fg on Hover, idle = fg2 on nothing),
        // mirroring the Rust setting rows glyph-for-glyph.
        m.insert(
            "settings_nav",
            SceneValue::Rows({
                let tabs: [(u32, &str, &str, crate::shell::SettingsTab); 6] = [
                    (crate::shell::panels::settings::NAV_NETWORK, ICON_WIFI, "Network", crate::shell::SettingsTab::Network),
                    (crate::shell::panels::settings::NAV_SOUND, ICON_VOLUME, "Sound & Display", crate::shell::SettingsTab::Sound),
                    (crate::shell::panels::settings::NAV_APPEARANCE, ICON_PALETTE, "Appearance", crate::shell::SettingsTab::Appearance),
                    (crate::shell::panels::settings::NAV_PILL, ICON_PALETTE, "Pill", crate::shell::SettingsTab::Pill),
                    (crate::shell::panels::settings::NAV_SYSTEM, ICON_MONITOR, "System", crate::shell::SettingsTab::System),
                    (crate::shell::panels::settings::NAV_MISC, ICON_SETTINGS, "Misc", crate::shell::SettingsTab::Misc),
                ];
                let mut rows: Vec<SceneRow> = Vec::with_capacity(tabs.len());
                for (key, glyph, label, tab) in tabs {
                    let sel = self.settings_tab == tab;
                    let hov = !sel && self.hover_key == key;
                    let surface = if sel { Some(ColorToken::HoverHl) } else if hov { Some(ColorToken::Hover) } else { None };
                    let gcol = Some(if sel { ColorToken::Acc } else if hov { ColorToken::Fg } else { ColorToken::Fg2 });
                    let lcol = Some(if sel || hov { ColorToken::Fg } else { ColorToken::Fg2 });
                    rows.push(SceneRow {
                        cols: vec![glyph.to_string(), label.to_string()],
                        key,
                        action: None,
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
                match self.settings_tab {
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
                    text: if visible { day.to_string() } else { String::new() },
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
                    hover: if visible && !is_today { Some(ColorToken::Fg) } else { None },
                    surface: if visible && is_today { Some(ColorToken::AccTint) } else { None },
                    // hovered today keeps its accent wash (the Rust drawer
                    // paints hover first, acc_tint over it)
                    surface_hover: if visible && is_today { Some(ColorToken::AccTint) } else { None },
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
                        surface: Some(ColorToken::Value(crate::ui::DynColor::named("app_tile_empty"))),
                        surface_hover: Some(ColorToken::HoverHl),
                        ..GridCell::default()
                    });
                    continue;
                }
                match self.apps.apps.iter().find(|a| a.id == self.app_shortcuts[i]) {
                    Some(a) => {
                        let ik = a.icon_key();
                        let has_ik = ik.is_some();
                        apps.push(GridCell {
                            image_key: ik,
                            glyph: if !has_ik { Some(ICON_APPS.into()) } else { None },
                            text: a.name.clone(),
                            bottom: true,
                            color: Some(ColorToken::Fg2),
                            surface: Some(ColorToken::Value(crate::ui::DynColor::named("app_tile"))),
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
                            surface: Some(ColorToken::Value(crate::ui::DynColor::named("app_tile"))),
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
            m.insert("mirror_rec_icon", SceneValue::Text(if rec { ICON_SQUARE.into() } else { ICON_DOT.into() }));
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
                    color: Some(if active { ColorToken::Fg } else { ColorToken::Fg2 }),
                    // SelBg targets the active row; hover fills via the Rows'
                    // `hover_surface` — row surface would always paint
                    surface: if active { Some(ColorToken::SelBg) } else { None },
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
        m.insert("cal_pane", SceneValue::Ring(self.cal_offset.max(-1).min(1) as f32));
        m.insert("app_slots", SceneValue::Ring(self.app_shortcuts.len() as f32));
        // ── sliders card (declared `sliders.ron`, zero Ink) ──
        // pane 0 = the six fader rows, pane 1 = the two switches that add /
        // remove the balance + saturation rows; the flip helper + the
        // hover-gated `Dots` need the same state as plain values
        m.insert("sliders_pane_0", SceneValue::Toggle(self.sliders_pane == 0));
        m.insert("sliders_pane_1", SceneValue::Toggle(self.sliders_pane == 1));
        m.insert("sliders_pane", SceneValue::Ring(self.sliders_pane as f32));
        m.insert("sliders_hover", SceneValue::Toggle(self.sliders_dots_hover()));
        // the pane-1 switches double as the pane-0 ROW gates — a hidden row
        // reserves no slot, so the vertically centered stack reflows exactly
        // like the drawer filtering its row list
        m.insert("show_balance", SceneValue::Toggle(self.show_balance));
        m.insert("show_saturation", SceneValue::Toggle(self.show_saturation));
        // balance is the R-share of the L/R mix, centered at 0.5 (0.5 = silent)
        let lr = self.vol_left + self.vol_right;
        m.insert(
            "balance_fader",
            SceneValue::Fader(if lr > 0.001 { (self.vol_right / lr).clamp(0.0, 1.0) } else { 0.5 }),
        );
        m.insert("sat_fader", SceneValue::Fader(self.saturation as f32 / 200.0));
        // the kelvin fader rides the drawer's 1200..6500K warm ramp
        let kel_warm = ((self.sunset_kelvin as f32 - 1200.0) / 5300.0).clamp(0.0, 1.0);
        m.insert("kelvin_fader", SceneValue::Fader(kel_warm));
        m.insert("vol_muted", SceneValue::Toggle(self.volume_muted));
        // value-column labels — "Muted" replaces the percentage, as in the drawer
        m.insert("bri_lbl", SceneValue::Text(format!("{}%", (self.brightness * 100.0).round() as i32)));
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
        m.insert("kel_lbl", SceneValue::Text(format!("{}K", self.sunset_kelvin)));
        // the volume row's glyph swaps to the mute icon while muted (`mic_icon`
        // already carries the mic row's swap)
        m.insert(
            "vol_glyph",
            SceneValue::Text(if self.volume_muted { ICON_MUTE.into() } else { ICON_VOLUME.into() }),
        );
        // ── toggles card (declared `toggles.ron`, zero Ink) ── the six switch
        // tiles; each carries its own region key and the two network tiles
        // carry a corner-chip key (the drawer's "more" chevron, keys 42 / 44)
        m.insert(
            "toggles_tiles",
            SceneValue::Tiles(vec![
                SceneTile { key: 41, glyph: ICON_WIFI.into(), label: "Wi-Fi".into(), on: self.wifi_on, chip_key: 42 },
                SceneTile { key: 43, glyph: ICON_BLUETOOTH.into(), label: "BT".into(), on: self.bt_on, chip_key: 44 },
                SceneTile { key: 45, glyph: ICON_SNOW.into(), label: "DND".into(), on: self.dnd, chip_key: 0 },
                SceneTile { key: 46, glyph: ICON_CAFFEINE.into(), label: "Caffeine".into(), on: self.caffeine_on, chip_key: 0 },
                SceneTile { key: 47, glyph: ICON_MOON.into(), label: "Sunset".into(), on: self.sunset_on, chip_key: 0 },
                SceneTile { key: 48, glyph: ICON_SHADER.into(), label: "Shader".into(), on: self.shader_on, chip_key: 0 },
            ]),
        );
        // ── dynamic colors (`ColorToken::Value`) — state-driven single-element
        // colors, pre-resolved against the LIVE theme sv_* palette so a
        // `value("dnd_chip")` token authored in `.ron` follows this exact
        // palette (DND chip accent, chips, banner accents as they get wired).
        let pal = Pal { fg: self.sv_fg, bg: self.sv_bg, acc: self.sv_acc, sfg: self.sv_sfg };
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
                    if hov { mix(ui::hover(&pal), pal.acc, 0.25) } else { (pal.acc & 0xFFFF_FF00) | 0x28 }
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
                SceneValue::Text(if on { "on".to_string() } else { "off".to_string() }),
            );
            // the screenshot buttons are hover-only surfaces: `off` is their
            // resting state, so the same ladder paints them
            m.insert_color(
                format!("comp_shot_{i}"),
                SceneColor::Raw(if hov { ui::hover_hl(&pal) } else { ui::hover(&pal) }),
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
                SceneColor::Raw(if danger { mix(RED, pal.fg, 0.25) } else { pal.fg }),
            );
        }
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
        m.insert("batt_h_pane_0", SceneValue::Toggle(present && self.battery_h_pane == 0));
        m.insert("batt_h_pane_1", SceneValue::Toggle(present && self.battery_h_pane == 1));
        m.insert("batt_v_pane_0", SceneValue::Toggle(present && self.battery_v_pane == 0));
        m.insert("batt_v_pane_1", SceneValue::Toggle(present && self.battery_v_pane == 1));
        m.insert(
            "batt_h_hover",
            SceneValue::Toggle(present && self.battery_dots_hover(self.battery_h_rect)),
        );
        m.insert(
            "batt_v_hover",
            SceneValue::Toggle(present && self.battery_dots_hover(self.battery_v_rect)),
        );
        // pane 1: the four metric rows (no regions — the pill owns the click)
        let state = if self.ac_online { "Charging \u{26a1}" } else { "Discharging" };
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
                if psv { ICON_REFRESH } else { ICON_BATTERY_SAVER },
                if psv { "Power save on" } else { "Power save" }
            )),
        );
        // ── sliders card row inks ── a muted volume/mic row paints its fill
        // AND glyph danger-red (the drawer's `RED` branch), and the kelvin
        // fader rides warn→accent by warmth; both are per-frame blends the
        // scene can't spell as a token.
        m.insert_color("vol_fill", SceneColor::Raw(if self.volume_muted { RED } else { pal.acc }));
        m.insert_color("mic_fill", SceneColor::Raw(if self.mic_muted { RED } else { pal.acc }));
        m.insert_color("kel_fill", SceneColor::Raw(mix(ui::WARN, pal.acc, kel_warm)));
        // pane-1 switch rows tint on hover — the drawer's `hl(key, 0, hover)`,
        // which pushes NO rect at rest, so a fully transparent `0` is the
        // declared equivalent and keeps the hit region key-driven.
        m.insert_color(
            "sliders_tog_bg_0",
            SceneColor::Raw(if self.hover_key == SLIDERS_TOGGLE_BALANCE_KEY { ui::hover(&pal) } else { 0 }),
        );
        m.insert_color(
            "sliders_tog_bg_1",
            SceneColor::Raw(if self.hover_key == SLIDERS_TOGGLE_SAT_KEY { ui::hover(&pal) } else { 0 }),
        );
        m.insert_color("dnd_chip", SceneColor::Raw(if self.dnd { pal.acc } else { ui::hover(&pal) }));
        m.insert_color(
            "dnd_chip_hover",
            SceneColor::Raw(if self.dnd { mix(pal.acc, pal.fg, 0.15) } else { ui::hover_hl(&pal) }),
        );
        // label ink tracks the theme directly (sfg on the accent fill, fg
        // otherwise) — a raw value is unnecessary, so bind a live TOKEN.
        m.insert_color(
            "dnd_chip_fg",
            SceneColor::Token(if self.dnd { ColorToken::Sfg } else { ColorToken::Fg }),
        );
        // ── lock-surface values (declared chrome binds them by name) ──
        // the fullscreen dim backdrop + the avatar disc fill — both are
        // derived colors `mix` can't express as a token, so pre-resolve them.
        m.insert_color("lock_dim", SceneColor::Raw(0x00000059));
        m.insert_color("lock_avatar", SceneColor::Raw(mix(ui::hover(&pal), pal.acc, 0.35)));
        // ── app-shortcut tiles (declared `Grid` scene binds them by name) ──
        // resting tile fill is a per-frame blend the scene can't express as a
        // token (hover mixes toward fg at different ratios per tile state).
        m.insert_color(
            "app_tile_empty",
            SceneColor::Raw(mix(ui::hover(&pal), pal.fg, 0.06)),
        );
        m.insert_color("app_tile", SceneColor::Raw(mix(ui::hover(&pal), pal.fg, 0.05)));
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
        m.insert_color("banner_guide", SceneColor::Raw(mix(ui::hover(&pal), pal.acc, 0.35)));
        m.insert_color("banner_guide_dim", SceneColor::Raw(mix(ui::hover(&pal), pal.fg, 0.10)));
        // the viewing strip band: opaque theme bg + "strip holds chips" gate
        // (the declared band suppresses the Rust one while the surface draws)
        m.insert_color("dash_opaque", SceneColor::Raw((self.sv_bg & 0xffffff00) | 0xff));
        m.insert("banner_shown", SceneValue::Toggle(!self.banner_collapsed()));
        let has_zone = self.banner_order.iter().any(|t| t.as_zone().is_some());
        m.insert("banner_editing", SceneValue::Toggle(self.dash_edit));
        m.insert("banner_drag", SceneValue::Toggle(self.banner_drag.is_some()));
        m.insert("banner_zones", SceneValue::Toggle(has_zone && !self.banner_strip_overflow()));
        m.insert("banner_collapsed", SceneValue::Toggle(self.banner_collapsed()));
        m.insert("banner_overflow", SceneValue::Toggle(self.banner_strip_overflow()));
        // ── banner viewing chip inks (the declarative `BannerRow` resolves
        // these per cell) — each mirrors its Rust drawer arm's color ladder ──
        m.insert_color("bchip_acc", SceneColor::Token(ColorToken::Acc));
        m.insert_color("bchip_dim", SceneColor::Token(ColorToken::Sfg));
        m.insert_color(
            "bchip_workspaces",
            SceneColor::Raw(if self.hover_key == BANNER_KEY_BASE { pal.acc } else { pal.fg }),
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
                if self.ssid.is_empty() { ColorToken::Fg } else { ColorToken::Acc }
            } else {
                ColorToken::Sfg
            }),
        );
        m.insert_color(
            "bchip_bt",
            SceneColor::Token(if self.bt_on { ColorToken::Acc } else { ColorToken::Fg }),
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
            SceneColor::Raw(if self.battery <= 20 && !self.ac_online { RED } else { pal.fg }),
        );
        // weather: dim until the service answers (glyph + temp in one ink)
        m.insert_color(
            "bchip_weather",
            SceneColor::Token(if self.banner_weather_text().is_some() { ColorToken::Fg } else { ColorToken::Sfg }),
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
            SceneColor::Token(if self.dnd { ColorToken::Acc } else { ColorToken::Fg }),
        );
        m.insert_color(
            "bchip_volume",
            SceneColor::Token(if self.volume_muted { ColorToken::Sfg } else { ColorToken::Fg }),
        );
        m.insert_color("bchip_bright", SceneColor::Token(ColorToken::Fg));
        m.insert_color(
            "bchip_vpn",
            SceneColor::Token(if self.ssh_vpn_lines.is_empty() { ColorToken::Sfg } else { ColorToken::Acc }),
        );
        m.insert_color("bchip_branding", SceneColor::Token(ColorToken::Fg));
        m.insert_color("bchip_sep", SceneColor::Token(ColorToken::Sfg));
        // activewin: twin hero states — empty (dim glyph + "No window") vs
        // live (app class · title · bottom-anchored per-app RAM pill)
        m.insert("active_win_empty", SceneValue::Toggle(self.active_win_class.is_empty()));
        m.insert("active_win_has", SceneValue::Toggle(!self.active_win_class.is_empty()));
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
        m.insert("pkg_count", SceneValue::Text(self.pkg_update_count.to_string()));
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
                            col_colors: vec![if is_sel || is_hov { Some(ColorToken::Acc) } else { None }],
                            surface: if is_sel { Some(ColorToken::HoverHl) } else { None },
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
        m.insert("docker_empty", SceneValue::Toggle(self.docker_containers.is_empty()));
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
    crate::scene::SceneRow { cols: vec![label.to_string(), value.to_string()], ..Default::default() }
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
        merge_module_values(&mut vals, vec![("mod_clock.time".into(), "16:24:46".into())]);
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
            vec![("clock".into(), "hijacked".into()), ("mod_x.v".into(), "kept".into())],
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
        assert_eq!(crate::scene::substitute("{mod_clock.time}", &vals), "{mod_clock.time}");
    }
}
