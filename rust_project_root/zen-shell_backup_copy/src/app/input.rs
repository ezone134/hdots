//! Input: pointer frame handling (hover / press / drag / scroll) and
//! keyboard handling (panel key events, repeat).

use super::*;
use crate::shell::{BANNER_CLEAR_ALL_KEY, DASH_CLEAR_ALL_KEY};

impl PointerHandler for App {
    fn pointer_frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _pointer: &wl_pointer::WlPointer,
        events: &[PointerEvent],
    ) {
        let Some(layer) = self.layer.as_ref() else { return };
        let our_surface = layer.wl_surface().clone();
        for e in events {
            // clicks on the corner notification popup open the notifications
            // panel and hide the popup — nothing else is interactive there
            let on_notif_surf = self
                .notif_surf
                .as_ref()
                .map(|s| s.surface_id.id() == e.surface.id())
                .unwrap_or(false);
            if on_notif_surf {
                if matches!(e.kind, PointerEventKind::Press { .. }) {
                    self.notif_popup.clear();
                    if let Some(s) = self.notif_surf.as_mut() {
                        s.hide(&self.conn);
                    }
                    self.apply_mode(crate::shell::Mode::Notifications);
                }
                continue;
            }
            // the session-lock surface owns ALL input while locked: hit-test
            // its own (fullscreen) layout — power buttons, hold-to-confirm.
            let on_lock_surf = self
                .lock_surf
                .as_ref()
                .map(|s| s.surface.wl_surface().id() == e.surface.id())
                .unwrap_or(false);
            if on_lock_surf {
                self.pointer_x = e.position.0;
                self.pointer_y = e.position.1;
                match e.kind {
                    PointerEventKind::Motion { .. } => {
                        if self.shell.set_cursor(Some((e.position.0 as f32, e.position.1 as f32))) {
                            self.dirty = true;
                            self.maybe_render();
                        }
                        // sliding off a held power button cancels the hold
                        if let Some(k) = self.shell.power_hold {
                            if self.shell.hover_key != k && self.shell.cancel_power_hold() {
                                self.dirty = true;
                                self.maybe_render();
                            }
                        }
                        // sliding off the clear-all button cancels its hold
                        if self.shell.dash_clear_all_hold.is_some() && self.shell.hover_key != DASH_CLEAR_ALL_KEY {
                            self.shell.cancel_clear_all_hold();
                        }
                        if self.shell.banner_clear_all_hold.is_some() && self.shell.hover_key != BANNER_CLEAR_ALL_KEY {
                            self.shell.cancel_banner_clear_hold();
                        }
                    }
                    PointerEventKind::Press { button: 272, .. } => {
                        self.pointer_down = true;
                        // try pill-item reorder drag first (collapsed + settings)
                        if self.shell.begin_pill_drag(e.position.0 as f32, e.position.1 as f32) {
                            self.dirty = true;
                            self.maybe_render();
                        } else if let Some(mode) = self.shell.click(self.pointer_x as f32, self.pointer_y as f32) {
                            self.apply_mode(mode);
                        } else {
                            self.dirty = true;
                            self.maybe_render();
                        }
                        // a click may have started/paused the pomodoro — sync
                        // the 1 s countdown driver with the new run state
                        self.sync_pomo_timer();
                        // a power-button press starts the hold fill; a
                        // Clear-all press starts its liquid fill timer too
                        if self.shell.power_hold.is_some()
                            || self.shell.dash_clear_all_hold.is_some()
                            || self.shell.banner_clear_all_hold.is_some()
                        {
                            self.ensure_hold_timer();
                        }
                        // a click may have started/paused the pomodoro — sync
                        // the 1 s countdown driver with the new run state
                        self.sync_pomo_timer();
                    }
                    PointerEventKind::Release { button: 272, .. } => {
                        self.pointer_down = false;
                        if self.shell.cancel_power_hold() {
                            self.dirty = true;
                            self.maybe_render();
                        }
                        // check if clear-all hold completed — harmless on the lock surface
                        // but kept for symmetry with the main-surface handler
                        if self.shell.check_clear_all_hold() {
                            self.dirty = true;
                            self.maybe_render();
                        }
                        if self.shell.check_banner_clear_hold() {
                            self.dirty = true;
                            self.maybe_render();
                        }
                    }
                    _ => {}
                }
                continue;
            }
            if e.surface != our_surface {
                continue;
            }
            self.pointer_x = e.position.0;
            self.pointer_y = e.position.1;
            let inside = self.pointer_inside(e.position.0, e.position.1);
            match e.kind {
                PointerEventKind::Enter { .. } => {
                    if std::env::var("ZEN_TRACE").is_ok() {
                        eprintln!("zen: pointer enter at ({:.0},{:.0}) inside={}", e.position.0, e.position.1, inside);
                    }
                    // arriving on the surface is real movement — re-arm
                    // hover-expansion (see hover_armed)
                    self.hover_armed = true;
                    // Record the cursor, then let the ONE hover rule decide
                    // (a bogus off-surface enter simply reports an outside
                    // cursor → stays collapsed; no "stuck expanded" state).
                    if self.shell.set_cursor(Some((e.position.0 as f32, e.position.1 as f32))) {
                        self.dirty = true;
                        self.maybe_render();
                    }
                    self.reconcile_hover();
                }
                PointerEventKind::Leave { .. } => {
                    if std::env::var("ZEN_TRACE").is_ok() {
                        eprintln!("zen: pointer leave at ({:.0},{:.0})", e.position.0, e.position.1);
                    }
                    // Stop tracking the cursor; the ONE hover rule re-checks
                    // against the compositor's ground truth (reconcile skips
                    // panels entirely — they never close on hover-out).
                    if self.shell.set_cursor(None) {
                        self.dirty = true;
                        self.maybe_render();
                    }
                    // leaving the surface cancels an in-flight power hold
                    if self.shell.cancel_power_hold() {
                        self.dirty = true;
                        self.maybe_render();
                    }
                    // also cancel clear-all holds
                    self.shell.cancel_clear_all_hold();
                    self.shell.cancel_banner_clear_hold();
                    // end an in-flight drag too (the compositor's implicit
                    // grab usually still delivers Release, but if a mode
                    // change interrupted it, don't leave the drag stuck)
                    //
                    // EDIT-MODE exception: don't commit on pointer leave.
                    // When the user resizes/drags a card past the current
                    // surface edge, the pointer escapes before the canvas
                    // morph catches up — committing here would snap the
                    // card to the wrong size.  Keep the drag alive; the
                    // morph will grow the surface and the pointer re-enters.
                    let is_edit_drag = self.shell.dash_edit
                        && self.shell.edit_drag.is_some();
                    if self.pointer_down && !is_edit_drag {
                        self.pointer_down = false;
                        let (rx, ry) = e.position;
                        let relaid = self.shell.end_drag(rx as f32, ry as f32);
                        self.shell.end_pill_drag();
                        self.shell.end_wp_sb_drag();
                        self.shell.end_th_sb_drag();
                        self.shell.end_dash_sb_drag();
                        self.shell.end_world_drag();
                        self.run_pending_actions();
                        // an edit-mode drop that grew the canvas → re-morph
                        if relaid && self.shell.mode == crate::shell::Mode::Expanded {
                            self.shell.refresh_size();
                            self.dirty = true;
                            self.maybe_render();
                            continue;
                        }
                        // mirror-card visibility may have changed on a drop
                        self.sync_cam_timer();
                    }
                    self.reconcile_hover();
                }
                PointerEventKind::Motion { .. } => {
                    // real movement — re-arm hover-expansion (see hover_armed)
                    self.hover_armed = true;
                    // hover highlights — redraw only when the element under the
                    // cursor actually changes (no per-pixel redraw churn)
                    if self.shell.set_cursor(Some((e.position.0 as f32, e.position.1 as f32))) {
                        self.dirty = true;
                        self.maybe_render();
                        self.refresh_ws_preview();
                    }
                    // keep the dashboard scrollbar fade-out loop alive if a
                    // reveal is still in its window (harmless, cheap)
                    if self.shell.dash_sb_pending_frames(Instant::now()) {
                        self.ensure_morph_timer();
                    }
                    // dragging a slider (button held): live value updates. The
                    // knob tracks the cursor every event (render), but the
                    // backend write is throttled — spawning wpctl / writing
                    // sysfs on every motion event (100+ Hz) floods the loop
                    // and the value lands in laggy bursts. 30 ms ≈ 33 writes/s
                    // max; the release handler flushes the final value.
                    if self.pointer_down && self.shell.drag(e.position.0 as f32, e.position.1 as f32) {
                        self.dirty = true;
                        self.maybe_render();
                        if self.slider_flush.elapsed() >= Duration::from_millis(30) {
                            self.run_pending_actions(); // apply to backend live
                            self.slider_flush = Instant::now();
                        }
                    }
                    // pill-item reorder drag
                    if self.pointer_down && self.shell.drag_pill(e.position.0 as f32, e.position.1 as f32) {
                        self.dirty = true;
                        self.maybe_render();
                    }
                    // wallpaper scrollbar grab-drag
                    if self.pointer_down && self.shell.drag_wp_sb(e.position.1 as f32) {
                        self.dirty = true;
                        self.maybe_render();
                    }
                    // theme-grid scrollbar grab-drag
                    if self.pointer_down && self.shell.drag_th_sb(e.position.1 as f32) {
                        self.dirty = true;
                        self.maybe_render();
                    }
                    // dashboard scrollbar grab-drag
                    if self.pointer_down && self.shell.drag_dash_sb(e.position.0 as f32, e.position.1 as f32) {
                        self.dirty = true;
                        self.maybe_render();
                        // keep frames coming for the overlay fade after release
                        self.ensure_morph_timer();
                    }
                    // world-map click-drag pan: the map follows the fingers via
                    // the (cheap) stretched band texture — no raster until idle
                    if self.pointer_down && self.shell.world_drag_to() {
                        self.dirty = true;
                        self.maybe_render();
                    }
                    // sliding off the held power button cancels the hold
                    if let Some(k) = self.shell.power_hold {
                        if self.shell.hover_key != k && self.shell.cancel_power_hold() {
                            self.dirty = true;
                            self.maybe_render();
                        }
                    }
                    // edit-mode drags: keep frames coming while cards glide,
                    // and arm the morph ticker for live canvas auto-fit
                    if (self.shell.anim.is_some()
                        || (self.shell.dash_edit
                            && self.shell.edit_drag.is_some()))
                        && self.morph_timer.is_none()
                    {
                        self.ensure_morph_timer();
                    }
                    self.sync_edit_timer();
                    // the ONE hover rule: over → expand, away → collapse
                    self.reconcile_hover();
                }
                PointerEventKind::Release { button, .. } => {
                    if button == 272 {
                        self.pointer_down = false;
                        // a track-bar drag ends in a real MPRIS seek
                        if let Some(delta) = self.shell.finish_seek() {
                            self.services.media_seek(delta);
                        }
                        let (rx, ry) = e.position;
                        let relaid = self.shell.end_drag(rx as f32, ry as f32);
                        self.shell.end_pill_drag();
                        self.shell.end_wp_sb_drag();
                        self.shell.end_th_sb_drag();
                        self.shell.end_dash_sb_drag();
                        // world-map click-drag: release straight back → clip the
                        // (lat, lon); dragged → just the pan the motion already did
                        let world_click = if self.shell.world_drag_active() {
                            !self.shell.end_world_drag()
                        } else {
                            false
                        };
                        if world_click {
                            self.shell.world_click_at();
                        }
                        // flush the drag's final value (the motion handler
                        // throttles mid-drag writes, so this must land)
                        self.run_pending_actions();
                        // an edit-mode drop that changed the layout → re-morph
                        // to the new auto-trimmed canvas extent
                        if relaid && self.shell.mode == crate::shell::Mode::Expanded {
                            self.shell.refresh_size();
                            self.ensure_morph_timer();
                            self.dirty = true;
                            self.maybe_render();
                        }
                        // the drop may have parked/unparked the Mirror card →
                        // release/grab the camera to match (idempotent)
                        self.sync_cam_timer();
                        self.sync_edit_timer();
                        // releasing the mouse button cancels an in-flight power
                        // hold (the configured hold delay must be held)
                        if self.shell.cancel_power_hold() {
                            self.dirty = true;
                            self.maybe_render();
                        }
                        // a Clear-all hold STILL clears when the button is held
                        // the full 3s and released over it — release-driven,
                        // but on THIS (dashboard/banner) surface, not the lock.
                        if self.shell.check_clear_all_hold() {
                            self.shell.clear_all_cards();
                            self.ensure_morph_timer();
                            self.dirty = true;
                            self.maybe_render();
                        }
                        if self.shell.check_banner_clear_hold() {
                            self.shell.clear_all_banner_cards();
                            self.ensure_morph_timer();
                            self.dirty = true;
                            self.maybe_render();
                        }
                        // a fresh map click landed → repaint the marker pin
                        if world_click {
                            self.dirty = true;
                            self.maybe_render();
                        }
                    }
                }
                PointerEventKind::Axis { vertical, horizontal, .. } => {
                    // world-map card: wheel / two-finger scroll PANS the map
                    // (vertical → latitude, horizontal → longitude) instead of
                    // flipping panels / panning the board. Trackpads move the
                    // map under the fingers, wheels use the classic direction.
                    if self.shell.over_worldmap() {
                        // north-positive vertical, east-positive horizontal
                        let v_pan = if vertical.absolute != 0.0 {
                            vertical.absolute as f32 / 30.0
                        } else if vertical.value120 != 0 {
                            vertical.value120 as f32 / 120.0
                        } else {
                            vertical.discrete as f32
                        };
                        let h_pan = if horizontal.absolute != 0.0 {
                            -(horizontal.absolute as f32) / 30.0
                        } else if horizontal.value120 != 0 {
                            -(horizontal.value120 as f32) / 120.0
                        } else {
                            -(horizontal.discrete as f32)
                        };
                        if self.shell.world_scroll(v_pan, h_pan) {
                            self.dirty = true;
                            self.maybe_render();
                            continue;
                        }
                    }
                    // mouse wheel: one notch = one row (discrete ticks and
                    // value120). Trackpad two-finger scroll arrives as smooth
                    // `absolute` deltas — NOT sign-flipped, so fingers-down
                    // moves content the natural (touchscreen) direction.
                    let dv = if vertical.discrete != 0 {
                        -vertical.discrete as f32
                    } else if vertical.value120 != 0 {
                        -vertical.value120 as f32 / 120.0
                    } else if vertical.absolute != 0.0 {
                        vertical.absolute as f32 / 30.0
                    } else {
                        0.0
                    };
                    let dh = if horizontal.discrete != 0 {
                        -horizontal.discrete as f32
                    } else if horizontal.value120 != 0 {
                        -horizontal.value120 as f32 / 120.0
                    } else if horizontal.absolute != 0.0 {
                        horizontal.absolute as f32 / 30.0
                    } else {
                        0.0
                    };
                    self.scroll_acc += dv;
                    self.scroll_h_acc += dh;
                    let steps = self.scroll_acc as i32;
                    let hsteps = self.scroll_h_acc as i32;
                    if steps != 0 {
                        self.scroll_acc -= steps as f32;
                    }
                    if hsteps != 0 {
                        self.scroll_h_acc -= hsteps as f32;
                    }
                    if steps != 0 {
                        // launcher: wheel / two-finger scroll MOVES THE
                        // SELECTION (rofi-style) and auto-scrolls the visible
                        // window with it
                        if self.shell.mode == crate::shell::Mode::Launcher {
                            if self.shell.launcher_select_by(steps) {
                                self.dirty = true;
                                self.maybe_render();
                            }
                        }
                        // clipboard lists: wheel / two-finger moves the row
                        if matches!(
                            self.shell.mode,
                            crate::shell::Mode::Clipboard | crate::shell::Mode::ClipboardImages
                        ) {
                            if self.shell.clip_select_by(steps) {
                                self.dirty = true;
                                self.maybe_render();
                            }
                        }
                        // scroll the wallpaper picker grid (wheel / trackpad)
                        if self.shell.mode == crate::shell::Mode::Wallpaper {
                            if self.shell.wallpaper_scroll_by(steps) {
                                self.dirty = true;
                                self.maybe_render();
                            }
                        }
                        // scroll the dashboard Wallpaper card when the pointer
                        // is over it (one row per notch) — independent of the
                        // full picker's scroll offset
                        if self.shell.mode == crate::shell::Mode::Expanded {
                            let px = self.pointer_x as f32;
                            let py = self.pointer_y as f32;
                            let inside = |r: (f32, f32, f32, f32)| -> bool {
                                r.2 > 0.0 && px >= r.0 && px <= r.0 + r.2 && py >= r.1 && py <= r.1 + r.3
                            };
                            let mut card_consumed = false;
                            // wallpaper card grid
                            if inside(self.shell.wp_card_rect) {
                                card_consumed |= self.shell.wp_card_scroll_by(steps);
                            }
                            // News card list
                            if inside(self.shell.news_rect) && !self.shell.news_items.is_empty() {
                                card_consumed |= self.shell.news_scroll_by(steps);
                            }
                            // Lyrics card list
                            if inside(self.shell.lyrics_rect) && !self.shell.lyrics_lines.is_empty() {
                                card_consumed |= self.shell.lyrics_scroll_by(steps);
                            }
                            // Wi-Fi / Bluetooth / Recent / Currency / Prices lists
                            if inside(self.shell.wifi_rect) && !self.shell.wifi_networks.is_empty() {
                                card_consumed |= self.shell.wifi_scroll_by(steps);
                            }
                            if inside(self.shell.bt_rect) && !self.shell.bt_devices.is_empty() {
                                card_consumed |= self.shell.bt_scroll_by(steps);
                            }
                            if inside(self.shell.recent_rect) && !self.shell.recent_files.is_empty() {
                                card_consumed |= self.shell.recent_scroll_by(steps);
                            }
                            if inside(self.shell.currency_rect) {
                                card_consumed |= self.shell.currency_scroll_by(steps);
                            }
                            if inside(self.shell.ticker_rect) && !self.shell.ticker_items.is_empty() {
                                card_consumed |= self.shell.ticker_scroll_by(steps);
                            }
                            // dashboard list cards: clipboard text / clipboard
                            // images / to-do / notes — wheel scrolls the list
                            if inside(self.shell.clip_card_rect) {
                                card_consumed |= self.shell.clip_card_scroll_by(steps);
                            }
                            if inside(self.shell.clipimg_card_rect) {
                                card_consumed |= self.shell.clipimg_card_scroll_by(steps);
                            }
                            if inside(self.shell.todo_card_rect) {
                                card_consumed |= self.shell.todo_card_scroll_by(steps);
                            }
                            if inside(self.shell.notes_card_rect) {
                                card_consumed |= self.shell.notes_card_scroll_by(steps);
                            }
                            // accent-card custom-accents subview (one-column list)
                            if self.shell.accent_list_open && inside(self.shell.accent_list_rect) {
                                card_consumed |= self.shell.accent_list_scroll_by(steps);
                            }
                            // edit-mode tray scrollers: parked banner chips
                            // or parked dashboard cards — wheel scrolls one
                            // row at a time if the tray overflows.
                            let tray_scrolled = if self.shell.dash_edit && steps != 0 {
                                if inside(self.shell.banner_tray_rect) && self.shell.banner_tray_overflow() {
                                    self.shell.banner_tray_scroll_by(steps)
                                } else if self.shell.tray_rect.2 > 0.0
                                    && inside(self.shell.tray_rect)
                                    && self.shell.dash_tray_overflow()
                                {
                                    self.shell.dash_tray_scroll_by(steps)
                                } else {
                                    false
                                }
                            } else {
                                false
                            };
                            if card_consumed {
                                self.dirty = true;
                                self.maybe_render();
                            } else if tray_scrolled {
                                self.dirty = true;
                                self.maybe_render();
                            } else if self.shell.banner_strip_point(self.pointer_x as f32, self.pointer_y as f32)
                                && self.shell.banner_strip_scroll_by(steps)
                            {
                                // whole-strip overflow mode: the wheel pans the
                                // ENTIRE banner strip as ONE row — every chip
                                // (left, center and right) scrolls together
                                self.dirty = true;
                                self.maybe_render();
                            } else if let Some(z) = self.shell.banner_zone_at(self.pointer_x as f32, self.pointer_y as f32) {
                                // strip fits the panel but a single L / C / R
                                // third overflows → scroll just that zone
                                // (never the board beneath it)
                                if self.shell.banner_zone_scroll_by(z, steps) {
                                    self.dirty = true;
                                    self.maybe_render();
                                }
                            } else if !inside(self.shell.wp_card_rect)
                                && !(inside(self.shell.news_rect) && !self.shell.news_items.is_empty())
                            {
                                // pan the BOARD across whichever axis overflows
                                // the viewport — a tall canvas scrolls down, a
                                // wide one sideways (independent of the stack
                                // direction toggle).
                                let over_y = self.shell.dash_canvas_h() - self.shell.dash_view_h();
                                let over_x = self.shell.dash_canvas_w() - self.shell.dash_view_w();
                                let (vr, hr) = if over_y > (self.shell.card_r()) {
                                    (steps as f32, 0.0)
                                } else if over_x > (self.shell.card_r()) {
                                    (0.0, steps as f32)
                                } else {
                                    (steps as f32, 0.0)
                                };
                                if self.shell.dash_scroll(vr, hr) {
                                    self.dirty = true;
                                    self.maybe_render();
                                    // keep frames coming for the overlay fade
                                    self.ensure_morph_timer();
                                }
                            }
                        }
                        // scroll the theme card grid (one row per notch)
                        if self.shell.mode == crate::shell::Mode::Themes {
                            if self.shell.themes_scroll_by(steps) {
                                self.dirty = true;
                                self.maybe_render();
                            }
                        }
                        // scroll the keybind viewer list (one row per notch)
                        if self.shell.mode == crate::shell::Mode::Keybinds {
                            if self.shell.keybinds_scroll_by(steps) {
                                self.dirty = true;
                                self.maybe_render();
                            }
                        }
                        // scroll the clipboard history list (one row per notch)
                        if self.shell.mode == crate::shell::Mode::Clipboard {
                            if self.shell.clip_scroll_by(steps) {
                                self.dirty = true;
                                self.maybe_render();
                            }
                        }
                        // scroll the Settings Appearance pane (toggles/alpha/
                        // custom-accent dropdown/apply) as one unified region;
                        // an open dropdown scrolls its own list instead
                        if self.shell.mode == crate::shell::Mode::Settings
                            && self.shell.settings_tab == crate::shell::SettingsTab::Appearance
                        {
                            let moved = if self.shell.custom_acc_drop_open {
                                self.shell.custom_acc_drop_scroll_by(steps)
                            } else {
                                self.shell.appearance_scroll_by(steps)
                            };
                            if moved {
                                self.dirty = true;
                                self.maybe_render();
                            }
                        }
                        // scroll the Settings Pill pane (sliders/toggles/reorder
                        // rows) — same px pitch as the Appearance pane
                        if self.shell.mode == crate::shell::Mode::Settings
                            && self.shell.settings_tab == crate::shell::SettingsTab::Pill
                        {
                            if self.shell.pill_scroll_by(steps) {
                                self.dirty = true;
                                self.maybe_render();
                            }
                        }
                        // scroll over the collapsed pill → switch workspaces
                        if self.shell.mode == crate::shell::Mode::Collapsed {
                            self.shell.scroll_workspace(steps as f32);
                            if self.shell.pending_ws_dispatch.is_some() {
                                self.dirty = true;
                                self.maybe_render();
                            }
                        }
                        // scroll over a tray chip → SNI Scroll (e.g. volume)
                        if let Some(id) =
                            self.shell.tray_click(self.pointer_x as f32, self.pointer_y as f32)
                        {
                            if let Some(tx) = &self.tray_cmd_tx {
                                let _ = tx.send(TrayCmd::Scroll(id, steps));
                            }
                        }
                    }
                    // horizontal pan of the dashboard board (trackpad two-finger
                    // sideways) — runs even with no vertical wheel movement.
                    // Over the News card it scrolls the category chip strip
                    // instead of panning the whole board.
                    if hsteps != 0 && self.shell.mode == crate::shell::Mode::Expanded {
                        let px = self.pointer_x as f32;
                        let py = self.pointer_y as f32;
                        let inside = |r: (f32, f32, f32, f32)| -> bool {
                            r.2 > 0.0 && px >= r.0 && px <= r.0 + r.2 && py >= r.1 && py <= r.1 + r.3
                        };
                        let over_news = inside(self.shell.news_rect) && !self.shell.news_items.is_empty();
                        // banner strip claims horizontal pan too: swiping over
                        // the banner BAND pans the strip (whole-strip overflow,
                        // or the zone under the pointer) — same area-wide hit
                        // test as the vertical wheel, not just the chips
                        let banner_hit = (self.shell.banner_strip_point(px, py)
                            && self.shell.banner_strip_scroll_by(hsteps))
                            || self
                                .shell
                                .banner_zone_at(px, py)
                                .map_or(false, |z| self.shell.banner_zone_scroll_by(z, hsteps));
                        // multi-pane cards: horizontal swipe flips the pane
                        let mut moved = false;
                        if banner_hit {
                            moved = true;
                        } else if inside(self.shell.battery_v_rect) && self.shell.battery_v_rect.2 > 0.0 && self.shell.battery >= 0 {
                            moved |= crate::shell::Shell::card_pane_flip(&mut self.shell.battery_v_pane, hsteps);
                        } else if inside(self.shell.battery_h_rect) && self.shell.battery_h_rect.2 > 0.0 && self.shell.battery >= 0 {
                            moved |= crate::shell::Shell::card_pane_flip(&mut self.shell.battery_h_pane, hsteps);
                        } else if inside(self.shell.weather_rect) && self.shell.weather_rect.2 > 0.0 && self.shell.weather.is_some() {
                            moved |= crate::shell::Shell::card_pane_flip(&mut self.shell.weather_pane, hsteps);
                        } else if inside(self.shell.worldmap_rect) && self.shell.worldmap_rect.2 > 0.0 {
                            // world map: a right-swipe opens the overlay menu; a
                            // left-swipe (or swiping again) closes it
                            if hsteps > 0 {
                                self.shell.world_menu = true;
                                moved = true;
                            } else if self.shell.world_menu {
                                self.shell.world_menu = false;
                                moved = true;
                            }
                        } else if inside(self.shell.mirror_rect) && self.shell.mirror_rect.2 > 0.0 {
                            // mirror card: horizontal swipe flips the pane
                            // (0 = feed + photo/record, 1 = fps settings)
                            moved |= crate::shell::Shell::card_pane_flip(&mut self.shell.mirror_pane, hsteps);
                        } else if inside(self.shell.sliders_rect) && self.shell.sliders_rect.2 > 0.0 {
                            // sliders card: horizontal swipe flips the pane
                            // (0 = fader rows, 1 = show-balance / show-saturation
                            // toggles)
                            moved |= crate::shell::Shell::card_pane_flip(&mut self.shell.sliders_pane, hsteps);
                        } else if inside(self.shell.audiorec_rect) && self.shell.audiorec_rect.2 > 0.0 {
                            // audio recorder card: horizontal swipe flips the
                            // pane (0 = recorder, 1 = recordings list) — same
                            // as the chevrons; opening the list refreshes it
                            let before = self.shell.audiorec_pane;
                            moved |= crate::shell::Shell::card_pane_flip(&mut self.shell.audiorec_pane, hsteps);
                            if self.shell.audiorec_pane == 1 && before != 1 {
                                self.shell.pending_audiorec_list = true;
                            } else if self.shell.audiorec_pane == 0 && before != 0 {
                                self.shell.audiorec_del_armed = None;
                            }
                        } else if inside(self.shell.water_rect) && self.shell.water_rect.2 > 0.0 {
                            // water card: horizontal swipe adjusts the daily goal
                            // (up/right = +1 glass of goal, down/left = −1)
                            let g = self.shell.water_goal as i32 + hsteps;
                            self.shell.water_set_goal(g.clamp(1, 30) as u32);
                            moved = true;
                        } else if over_news {
                            moved |= self.shell.news_cat_scroll_by(hsteps as f32 * 8.0);
                        } else {
                            // dash_scroll_dir swaps rows ↔ columns — but a two-finger
                            // sideways swipe always pans the board HORIZONTALLY:
                            // on a horizontal (metro) stack only horizontal
                            // panning is allowed, vertical is never triggered.
                            let (vr, hr) = (0.0, hsteps as f32);
                            moved |= self.shell.dash_scroll(vr, hr);
                        };
                        if moved {
                            self.dirty = true;
                            self.maybe_render();
                            // keep frames coming for the overlay fade
                            self.ensure_morph_timer();
                        }
                    }
                }
                PointerEventKind::Press { button, .. } => {
                    if button == 272 {
                        self.pointer_down = true;
                        // pill-item reorder drag FIRST (Settings → Pill rows):
                        // a held press must START a drag, not fire the row's
                        // toggle on press — tap-without-move toggles on
                        // release instead (see end_pill_drag).
                        if self
                            .shell
                            .begin_pill_drag(self.pointer_x as f32, self.pointer_y as f32)
                        {
                            self.dirty = true;
                            self.maybe_render();
                            continue;
                        }
                        // wallpaper grid scrollbar: a press starts a grab-drag
                        // (must not fall through to click() — any unhandled
                        // key there closes the panel)
                        if self
                            .shell
                            .begin_wp_sb_drag(self.pointer_x as f32, self.pointer_y as f32)
                        {
                            self.dirty = true;
                            self.maybe_render();
                            continue;
                        }
                        // theme-grid scrollbar: same treatment — consume the
                        // press or the unhandled key in click() collapses the
                        // panel mid-drag
                        if self
                            .shell
                            .begin_th_sb_drag(self.pointer_x as f32, self.pointer_y as f32)
                        {
                            self.dirty = true;
                            self.maybe_render();
                            continue;
                        }
                        // dashboard scrollbar: press starts a grab-drag, never
                        // a click (the click handler would treat it as a noop
                        // background press and collapse the panel)
                        if self
                            .shell
                            .begin_dash_sb_drag(self.pointer_x as f32, self.pointer_y as f32)
                        {
                            self.dirty = true;
                            self.maybe_render();
                            // fade-out frames after the grab ends
                            self.ensure_morph_timer();
                            continue;
                        }
                        // world-map body press: start the click-drag pan. Pressing
                        // anywhere on the body grabs the map (like a gallery
                        // photo) — release-without-move becomes the place click.
                        if self.shell.begin_world_drag() {
                            self.dirty = true;
                            self.maybe_render();
                            continue;
                        }
                        // tray chips only exist on the pill / dashboard — the
                        // audio panel's mute buttons share the 30+ key range
                        if matches!(
                            self.shell.mode,
                            crate::shell::Mode::Collapsed | crate::shell::Mode::Expanded
                        ) {
                            // tray items first: Activate the app, never change mode
                            if let Some(id) =
                                self.shell.tray_click(self.pointer_x as f32, self.pointer_y as f32)
                            {
                                if let Some(tx) = &self.tray_cmd_tx {
                                    let _ = tx.send(TrayCmd::Activate(id));
                                }
                                continue;
                            }
                        }
                        // media transport buttons first (MPRIS)
                        if let Some(cmd) = self.shell.media_click(self.pointer_x as f32, self.pointer_y as f32) {
                            self.services.media_command(cmd);
                            continue;
                        }
                        // left click → hit-test on the current mode layout
                        if let Some(mode) = self.shell.click(self.pointer_x as f32, self.pointer_y as f32) {
                            self.apply_mode(mode);
                        } else {
                            // edit-mode buttons (grid rows/cols +/−, banner
                            // width +/−, pill toggles …) can change the target
                            // size. While idle no morph timer is armed and the
                            // compositor never learns about the new size until
                            // you exit edit mode — arm it so the board grows /
                            // shrinks LIVE under the cursor.
                            if self.shell.mode == crate::shell::Mode::Expanded && self.shell.dash_edit {
                                self.ensure_morph_timer();
                            }
                            self.dirty = true;
                            self.maybe_render();
                        }
                        // a power-button press starts the hold fill; a
                        // Clear-all press starts its liquid fill timer too
                        if self.shell.power_hold.is_some()
                            || self.shell.dash_clear_all_hold.is_some()
                            || self.shell.banner_clear_all_hold.is_some()
                        {
                            self.ensure_hold_timer();
                        }
                        // a click may have started/paused the pomodoro — sync
                        // the 1 s countdown driver with the new run state
                        self.sync_pomo_timer();
                        // queued actions (wifi/bt toggles, audio, wallpaper)
                        self.run_pending_actions();
                    } else if button == 273 {
                        // right-click: pill → Settings; dashboard → EDIT MODE
                        // (metro-style: drag to move, corner grip to resize,
                        // canvas auto-fits the content)
                        if matches!(self.shell.mode, crate::shell::Mode::Collapsed) {
                            self.apply_mode(crate::shell::Mode::Settings);
                        } else if matches!(self.shell.mode, crate::shell::Mode::Expanded) {
                            self.shell.toggle_dash_edit();
                            // canvas re-morphs to the card extent on exit
                            self.ensure_morph_timer();
                            self.dirty = true;
                            self.maybe_render();
self.sync_edit_timer();
                            self.sync_autoarrange_timer();
                        }
                    }
                }
            }
        }
    }
}

impl KeyboardHandler for App {
    fn enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &wl_keyboard::WlKeyboard,
        _surface: &wl_surface::WlSurface,
        _serial: u32,
        _raw: &[u32],
        _keysyms: &[Keysym],
    ) {
        if std::env::var("ZEN_TRACE").is_ok() {
            eprintln!("zen: keyboard enter mode={:?}", self.shell.mode);
        }
    }

    fn leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &wl_keyboard::WlKeyboard,
        _surface: &wl_surface::WlSurface,
        _serial: u32,
    ) {
        // Every modal panel grabs EXCLUSIVE keyboard; clicking any other window
        // drops it, which is the "click outside" signal — close the panel.
        // Panels never close on hover-out, so this is the only mouse-driven
        // dismissal besides a background click on the panel itself.
        if std::env::var("ZEN_TRACE").is_ok() {
            eprintln!("zen: keyboard leave mode={:?}", self.shell.mode);
        }
        match self.shell.mode {
            crate::shell::Mode::Launcher
            | crate::shell::Mode::ControlCenter
            | crate::shell::Mode::Notifications
            | crate::shell::Mode::Calendar
            | crate::shell::Mode::Settings
            | crate::shell::Mode::Power
            | crate::shell::Mode::WifiMenu
            | crate::shell::Mode::BtMenu
            | crate::shell::Mode::Audio
            | crate::shell::Mode::Wallpaper
            | crate::shell::Mode::Weather
            | crate::shell::Mode::Clipboard
            | crate::shell::Mode::ClipboardImages
            | crate::shell::Mode::Themes => {
                self.apply_mode(crate::shell::Mode::Collapsed);
            }
            _ => {
                // dashboard (on-demand keyboard): losing focus blurs the
                // to-do composer + world-clock search + the card composers so
                // stray typing never lands in them
                let blurred = self.shell.todo_input.take().is_some()
                    || self.shell.worldclock_search.take().is_some()
                    || self.shell.countdown_input.take().is_some()
                    || self.shell.alarm_input.take().is_some()
                    || self.shell.snippet_input.take().is_some()
                    || self.shell.expense_input.take().is_some();
                if blurred {
                    self.dirty = true;
                    self.maybe_render();
                }
            }
        }
    }

    fn press_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &wl_keyboard::WlKeyboard,
        _serial: u32,
        event: KeyEvent,
    ) {
        // wallpaper grid zoom: Ctrl+= / Ctrl++ grows, Ctrl+- shrinks the
        // thumbnails. Handled BEFORE shell.key — an unhandled key there
        // closes the panel. repeat_key re-enters here, so holding repeats.
        if self.shell.mode == crate::shell::Mode::Wallpaper
            && self.ctrl_down
            && matches!(event.keysym.raw(), 0x2d | 0x2b | 0x3d | 0xffab | 0xffad)
        {
            if self.shell.wallpaper_zoom(event.keysym.raw() != 0x2d && event.keysym.raw() != 0xffad) {
                self.shell.save_config();
                self.dirty = true;
                self.maybe_render();
            }
            return;
        }
        // Space over the media card (dashboard) toggles play/pause — done
        // before shell.key so the dashboard never eats it
        if event.keysym.raw() == 0x20
            && self.shell.mode == crate::shell::Mode::Expanded
            && self.shell.todo_input.is_none()
        {
            let (mx, my, mw, mh) = self.shell.media_card_rect;
            let px = self.pointer_x as f32;
            let py = self.pointer_y as f32;
            if mw > 0.0 && px >= mx && px <= mx + mw && py >= my && py <= my + mh {
                self.services.media_command("PlayPause");
                self.dirty = true;
                self.maybe_render();
                return;
            }
        }
        if let Some(mode) = self.shell.key(event.keysym.raw()) {
            self.apply_mode(mode);
        } else {
            // leaving EDIT MODE re-morphs the canvas to the committed card
            // extent — arm the ticker so the shrink animates even though no
            // pointer motion follows (Esc pressed while the mouse rests)
            self.ensure_morph_timer();
            self.dirty = true;
            self.maybe_render();
        }
        // lockscreen: Enter submitted a password — verify it through PAM
        if std::mem::take(&mut self.shell.pending_lock_auth) {
            self.verify_lock_password();
        }
        // polkit dialog: Enter submitted a password — verify it, then answer
        // the polkit authority (and the in-DB dialog) with the result.
        if std::mem::take(&mut self.shell.polkit_pending) {
            self.submit_polkit();
        }
    }

    fn repeat_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &wl_keyboard::WlKeyboard,
        _serial: u32,
        event: KeyEvent,
    ) {
        // holding Enter on the lockscreen must not re-submit the (now empty)
        // password over and over — one PAM attempt per press
        if self.shell.mode == crate::shell::Mode::Lock && event.keysym.raw() == 0xff0d {
            return;
        }
        self.press_key(_conn, _qh, _keyboard, _serial, event);
    }

    fn release_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &wl_keyboard::WlKeyboard,
        _serial: u32,
        _event: KeyEvent,
    ) {
    }

    fn update_modifiers(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &wl_keyboard::WlKeyboard,
        _serial: u32,
        _modifiers: smithay_client_toolkit::seat::keyboard::Modifiers,
        raw: smithay_client_toolkit::seat::keyboard::RawModifiers,
        _layout: u32,
    ) {
        // caps lock = xkb modifier index 1 (Lock); the lockscreen warns about it
        let caps = raw.locked & (1 << 1) != 0;
        if self.shell.caps != caps {
            self.shell.caps = caps;
            if self.shell.mode == crate::shell::Mode::Lock {
                self.dirty = true;
                self.maybe_render();
            }
        }
        // control = xkb modifier index 2 — gates the wallpaper grid zoom keys
        self.ctrl_down = raw.depressed & (1 << 2) != 0;
    }
}

smithay_client_toolkit::delegate_dispatch2!(App);
