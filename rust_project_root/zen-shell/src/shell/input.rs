use super::*;

impl Shell {
    /// Re-anchor the bar strip to an edge and persist it. The app consumes
    /// `pending_move` and re-applies the layer surface geometry.
    pub(crate) fn set_bar_edge(&mut self, e: crate::shell::BarEdge) -> Option<Mode> {
        self.bar_edge = e;
        self.pending_move = Some(e);
        self.save_config();
        self.sys_menu_open = false;
        self.sys_menu_bp_open = false;
        None
    }

    /// Corner resize grip rect of a card (pixels, SURFACE space — scrolled).
    /// Offsets are scaled to match the drawn grip exactly.
    pub(crate) fn grip_px(&self, l: CardLayout) -> (f32, f32, f32, f32) {
        let cw = self.dash_cell_base_w(self.cfg.expanded_w());
        let ch = self.dash_cell_h(self.cfg.expanded_w());
        let (x, y, w, h) = self.scrolled(dash_card_px(l, cw, ch, self.grid_top(), self.dash_pad(), self.dash_gap()));
        (x + w - self.scale.s(18.0), y + h - self.scale.s(18.0), 16.0, 16.0)
    }

    pub(crate) fn in_rect(px: f32, py: f32, r: (f32, f32, f32, f32)) -> bool {
        px >= r.0 && px <= r.0 + r.2 && py >= r.1 && py <= r.1 + r.3
    }

    /// Enter / leave EDIT MODE. Leaving discards any in-flight drag and
    /// re-morphs the surface to the committed content extent (the canvas
    /// auto-trims — exiting edit mode must never leave a stale-sized bg).
    pub(crate) fn toggle_dash_edit(&mut self) {
        self.dash_edit = !self.dash_edit;
        self.edit_drag = None;
        self.edit_resize = None;
        self.edit_drag_from_tray = false;
        self.edit_preview_pack = None;
        self.edit_ghost_slot = None;
        self.edit_resize_cached = None;
        self.banner_drag = None;
        self.drag_key = None;
        self.card_anim.clear();
        self.edit_settling = false;
        self.todo_input = None;
        // entering edit mode hands the grid to the LIVE packer so cards
        // compact / fill empty spaces (the restored exact positions only
        // stick while NOT editing)
        self.layout_pinned = false;
        self.refresh_pack();
        if self.dash_edit {
            // hard rules on entry: cards cascade left / climb to fill any
            // space the restored layout left (deleted cards, parked cards)
            self.autoarrange();
            // reset tray pagers so they start at the top
            self.banner_tray_scroll_row = 0;
            self.dash_tray_scroll_row = 0;
        }
        // edit mode stacks the chrome trays above the board — re-fit the
        // surface so the first card row stays visible (and shrink back on
        // exit)
        self.refresh_size();
    }

    /// ✕ close-button rect of a card in EDIT MODE (top-right corner) —
    /// clicking parks the card into the tray. CONTENT-space rect (apply
    /// `scrolled()` at the call site) — offsets/size scaled to match the
    /// drawn button exactly.
    pub(crate) fn card_close_px(l: CardLayout, cw: f32, ch: f32, grid_top: f32, dash_pad: f32, dash_gap: f32, scale: GridScale) -> (f32, f32, f32, f32) {
        let (x, y, w, _) = dash_card_px(l, cw, ch, grid_top, dash_pad, dash_gap);
        (x + w - scale.s(27.0), y + scale.s(7.0), scale.s(20.0), scale.s(20.0))
    }

    /// Pixel rect of tray slot `i` — shared by the drawer and input so they
    /// can never disagree. Chips flow left→right inside `tray_rect`, below
    /// its label row, wrapping at the strip's right edge. Every column
    /// STRETCHES to fill the tray's full width (no dead space on wide
    /// windows).
    pub(crate) fn tray_chip_px(&self, i: usize) -> (f32, f32, f32, f32) {
        pub(crate) const MIN_CW: f32 = 96.0;
        pub(crate) const CH: f32 = 44.0;
        let (tx, ty, tw, _) = self.tray_rect;
        let per_row = (((tw - 24.0) / (MIN_CW + 12.0)).floor() as usize).max(1);
        let cw = ((tw - 24.0 - (per_row as f32 - 1.0) * 12.0) / per_row as f32).max(MIN_CW);
        let col = i % per_row;
        let row = i / per_row;
        (
            tx + 12.0 + col as f32 * (cw + 12.0),
            ty + 26.0 + (row as i32 - self.dash_tray_scroll_row) as f32 * (CH + 10.0),
            cw,
            CH,
        )
    }

    /// Regions are stored bottom-to-top; later pushes win (checked in reverse).
    pub(crate) fn hover_key_at(&self, x: f32, y: f32) -> u32 {
        for (rx, ry, rw, rh, k) in self.hover_regions.iter().rev() {
            if x >= *rx && x < rx + rw && y >= *ry && y < ry + rh {
                return *k;
            }
        }
        0
    }

    /// Track the pointer. Returns true when the hovered element changed, so
    /// the caller only redraws on an actual highlight transition.
    pub(crate) fn set_cursor(&mut self, pos: Option<(f32, f32)>) -> bool {
        self.cursor = pos;
        // EDIT MODE: suppress per-element highlights — cards highlight as a
        // whole instead (the drawers key off self.dash_edit)
        let k = if self.dash_edit {
            0
        } else {
            pos.map(|(x, y)| self.hover_key_at(x, y)).unwrap_or(0)
        };
        if k != self.hover_key {
            self.hover_key = k;
            true
        } else {
            false
        }
    }

    /// Re-derive the hover key (called when the mode / scene changes).
    pub(crate) fn recompute_hover(&mut self) {
        self.hover_key = self
            .cursor
            .map(|(x, y)| self.hover_key_at(x, y))
            .unwrap_or(0);
    }

    /// Register a hot rect for the current layout pass. Offset by the current
    /// content shift so hit-testing tracks the drawn content exactly (during
    /// a spring overshoot the content is centered, not at x=0).
    pub(crate) fn region(&mut self, x: f32, y: f32, w: f32, h: f32, key: u32) {
        self.hover_regions.push((x + self.content_dx, y + self.content_dy, w, h, key));
    }

    /// Highlight color: `over` when the element under the cursor matches `key`.
    pub(crate) fn hl(&self, key: u32, base: u32, over: u32) -> u32 {
        if self.hover_key == key {
            over
        } else {
            base
        }
    }

    // ---------------------------------------------------------------- clock

    pub(crate) fn update_clock(&mut self) -> bool {
        let mut buf = [0u8; 32];
        unsafe {
            let now = libc::time(std::ptr::null_mut());
            let mut tm: libc::tm = std::mem::zeroed();
            libc::localtime_r(&now, &mut tm);
            let n = libc::strftime(buf.as_mut_ptr() as *mut libc::c_char, buf.len(), c"%H:%M".as_ptr(), &tm);
            let s = String::from_utf8_lossy(&buf[..n]).into_owned();
            if s != self.clock {
                self.clock = s;
                return true;
            }
            false
        }
    }

    /// Sample `battery_watts` into the pill display value at most once a
    /// minute (called from the per-second tick). Returns true on refresh.
    pub(crate) fn tick_watts(&mut self) -> bool {
        if self.battery_watts <= 0.0 {
            return false;
        }
        let due = match self.watts_at {
            None => true,
            Some(t) => t.elapsed().as_secs() >= 60,
        };
        if due && (self.watts_shown - self.battery_watts).abs() > f32::EPSILON {
            self.watts_shown = self.battery_watts;
            self.watts_at = Some(Instant::now());
            true
        } else {
            if due {
                self.watts_at = Some(Instant::now());
            }
            false
        }
    }

    /// click regions and hover highlights can never drift apart.
    pub(crate) fn click(&mut self, x: f32, y: f32) -> Option<Mode> {
        let key = self.hover_key_at(x, y);
        // tray chips: handled by the app (Activate); never switch mode. Only
        // the pill / dashboard layouts register 30+ keys (the audio panel's
        // mute buttons share that range but only while Audio is open).
        if let Some(t) = key.checked_sub(30) {
            if (t as usize) < self.tray.len() {
                return None;
            }
        }
        // content is laid out at the target size while a morph is in flight,
        // so click math must use the target width too (equal at rest)
        let w = self.target_size().0;
        match self.mode {
            Mode::Collapsed => match key {
                // left-cluster fixed items: clock (70) → calendar, battery (71) → noop
                70 => Some(Mode::Calendar),
                71 => None,
                // center-cluster reorderable items: keys 50+i*10+j (slot=i, sub=j)
                k if k >= 50 && k < 60 => {
                    let slot = ((k - 50) / 10) as usize;
                    let sub = (k - 50) % 10;
                    if let Some(&item) = self.pill_order.get(slot) {
                        match item {
                            // single-number forms: click opens the switcher
                            PillItem::Workspaces | PillItem::WorkspacesShort => {
                                return Some(Mode::WorkspaceSwitcher)
                            }
                            // long form: chips 1..5 switch directly
                            PillItem::WorkspacesLong => {
                                let ws = sub as usize;
                                if ws < self.ws_count {
                                    self.ws_prev = self.ws_active;
                                    self.ws_active = ws;
                                    self.pending_ws_dispatch = Some(ws + 1); // 1-indexed
                                }
                            }
                            PillItem::Wifi => return Some(Mode::WifiMenu),
                            PillItem::Bluetooth => return Some(Mode::BtMenu),
                            PillItem::Volume => return Some(Mode::Audio),
                            PillItem::Wallpaper => return Some(Mode::Wallpaper),
                            PillItem::Themes => return Some(Mode::Themes),
                            PillItem::Clock => return Some(Mode::Calendar),
                            _ => {}
                        }
                    }
                    None
                }
                62 => Some(Mode::Notifications),  // bell
                63 => Some(Mode::ControlCenter),   // right cluster
                64 => Some(Mode::Settings),        // settings gear chip
                67 => {
                    self.dark_mode = !self.dark_mode;
                    self.pending_dash = Some(DashCmd::ThemeSwitch);
                    None
                }
                _ => Some(self.dash_mode()),
            },
            Mode::Expanded => {
                // EDIT MODE intercepts everything: grip press starts a
                // resize, card body starts a move-drag, background is a noop
                if self.dash_edit {
                    self.edit_press(x, y);
                    return None;
                }
                match key {
                // app-shortcut card slots: a filled slot launches the app; an
                // empty "+" slot opens the launcher in pick mode for it. The
                // hover ✕ sub-keys remove a slot. Registered before the banner
                // band (34_000) so the elevated range never leaks into it.
                k @ APP_SHORTCUT_KEY_BASE..=APP_SHORTCUT_KEY_MAX => {
                    let i = (k - APP_SHORTCUT_KEY_BASE) as usize;
                    if i >= self.app_shortcuts.len() {
                        self.app_shortcut_pick = Some(i);
                        return Some(Mode::Launcher);
                    }
                    let id = self.app_shortcuts[i].clone();
                    if !id.is_empty() {
                        self.apps.launch(&id);
                    }
                    Some(Mode::Collapsed)
                }
                k @ APP_SHORTCUT_REMOVE_BASE..=APP_SHORTCUT_REMOVE_MAX => {
                    let i = (k - APP_SHORTCUT_REMOVE_BASE) as usize;
                    if i < self.app_shortcuts.len() {
                        self.app_shortcuts.remove(i);
                        self.save_config();
                    }
                    None
                }
                // connectivity-popover rows (aggregate "net" chip): jump
                // straight into the Wi-Fi / Bluetooth menus
                CONN_WIFI_KEY => {
                    self.connectivity_open = false;
                    Some(Mode::WifiMenu)
                }
                CONN_BT_KEY => {
                    self.connectivity_open = false;
                    Some(Mode::BtMenu)
                }
                // system-card ⋮ menu: toggles the popover, expands the
                // Bar-position edge submenu, or jumps to Settings
                SYS_MENU_KEY => {
                    self.sys_menu_open = !self.sys_menu_open;
                    self.sys_menu_bp_open = false;
                    self.connectivity_open = false;
                    None
                }
                SYS_MENU_BP_KEY => {
                    self.sys_menu_bp_open = !self.sys_menu_bp_open;
                    None
                }
                SYS_MENU_SETTINGS_KEY => {
                    self.sys_menu_open = false;
                    self.sys_menu_bp_open = false;
                    self.connectivity_open = false;
                    Some(Mode::Settings)
                }
                // ⋮ Bar-position submenu rows (and the Settings → Pill row):
                // re-anchor the bar to the chosen screen edge, persist it
                SYS_EDGE_L_KEY => self.set_bar_edge(crate::shell::BarEdge::Left),
                SYS_EDGE_T_KEY => self.set_bar_edge(crate::shell::BarEdge::Top),
                SYS_EDGE_R_KEY => self.set_bar_edge(crate::shell::BarEdge::Right),
                SYS_EDGE_B_KEY => self.set_bar_edge(crate::shell::BarEdge::Bottom),
                // top banner strip (ordered slots): chip presses open their
                // panels (settings / power / date); the aggregate connectivity
                // chip expands its Wi-Fi / Bluetooth jump menu in place;
                // separators and fillers are display-only outside edit mode
                k if k >= BANNER_KEY_BASE => {
                    let i = (k - BANNER_KEY_BASE) as usize;
                    if i < self.banner_order.len() {
                        match self.banner_order[i].clone() {
                            BannerToken::Chip(BannerItem::Settings) => { self.connectivity_open = false; Some(Mode::Settings) }
                            BannerToken::Chip(BannerItem::Power) => { self.connectivity_open = false; Some(Mode::Power) }
                            BannerToken::Chip(BannerItem::Date) => { self.connectivity_open = false; Some(Mode::Calendar) }
                            BannerToken::Chip(BannerItem::AppSearch) => { self.connectivity_open = false; Some(Mode::Launcher) }
                            BannerToken::Chip(BannerItem::Wifi) => { self.connectivity_open = false; Some(Mode::WifiMenu) }
                            BannerToken::Chip(BannerItem::Bluetooth) => { self.connectivity_open = false; Some(Mode::BtMenu) }
                            BannerToken::Chip(BannerItem::Connectivity) => {
                                self.connectivity_open = !self.connectivity_open;
                                None
                            }
                            BannerToken::Chip(BannerItem::Weather) => {
                                self.connectivity_open = false;
                                if self.weather.is_some() { Some(Mode::Weather) } else { None }
                            }
                            BannerToken::Chip(BannerItem::Dnd) => {
                                self.connectivity_open = false;
                                self.dnd = !self.dnd;
                                None
                            }
                            BannerToken::Chip(BannerItem::Volume) => {
                                self.connectivity_open = false;
                                Some(Mode::ControlCenter)
                            }
                            BannerToken::Chip(BannerItem::Brightness) => {
                                self.connectivity_open = false;
                                Some(Mode::ControlCenter)
                            }
                            BannerToken::Chip(BannerItem::Battery)
                            | BannerToken::Chip(BannerItem::Cpu)
                            | BannerToken::Chip(BannerItem::Ram)
                            | BannerToken::Chip(BannerItem::NetSpeed)
                            | BannerToken::Chip(BannerItem::Vpn) => {
                                self.connectivity_open = false;
                                None
                            }
                            _ => { self.connectivity_open = false; None }
                        }
                    } else {
                        self.connectivity_open = false;
                        None
                    }
                }
                // row 1 keeps only settings + power chips; the bell lives on
                // the system-info card (24) and the date chip opens the
                // calendar (25)
                14 => Some(Mode::Settings),
                15 => Some(Mode::Power),
                24 => Some(Mode::Notifications),
                25 => Some(Mode::Calendar),
                // accent-card scheme chip → theme picker
                28 => Some(Mode::Themes),
                // identity-card dark/light chip: fires `theme_main switch`;
                // the real state comes back via $states2/m_dummy
                26 => {
                    self.dark_mode = !self.dark_mode;
                    self.pending_dash = Some(DashCmd::ThemeSwitch);
                    None
                }
                // weather / stats line → detailed weather (when available)
                19 => {
                    if self.weather.is_some() {
                        Some(Mode::Weather)
                    } else {
                        None
                    }
                }
                // quick-toggle strip (Wi-Fi / BT / DND / Caffeine / Sunset /
                // Shader); Wi-Fi & BT carry a chevron sub-region → menus
                41 => {
                    self.wifi_on = !self.wifi_on;
                    self.pending_wifi_power = Some(self.wifi_on);
                    None
                }
                42 => Some(Mode::WifiMenu), // > chevron
                43 => {
                    self.bt_on = !self.bt_on;
                    self.pending_bt_power = Some(self.bt_on);
                    None
                }
                44 => Some(Mode::BtMenu), // > chevron
                45 => {
                    self.dnd = !self.dnd;
                    None
                }
                46 => {
                    self.caffeine_on = !self.caffeine_on;
                    self.pending_dash = Some(DashCmd::CaffeineToggle);
                    None
                }
                47 => {
                    self.sunset_on = !self.sunset_on;
                    self.pending_dash = Some(DashCmd::SunsetToggle);
                    None
                }
                48 => {
                    self.shader_on = !self.shader_on;
                    self.pending_dash = Some(DashCmd::ShaderToggle);
                    None
                }
                // dashboard faders: 51 brightness · 52 volume · 59 balance ·
                // 53 mic · 54 saturation · 55 color temperature
                58 => {
                    // balance reset chip (dashboard) — move the head back to center
                    self.vol_left = self.volume;
                    self.vol_right = self.volume;
                    None
                }
                79 => {
                    // shader reset chip (dashboard) — back to 100 (normal saturation)
                    self.saturation = 100;
                    self.pending_dash = Some(DashCmd::Saturation(100));
                    None
                }
                key @ 51..=59 => {
                    self.set_dash_slider(key, x, y);
                    None
                }
                // audio recorder card: own mic fader (its geometry is stored in
                // `audiorec_mic_rect`, not the shared `faders` array), record
                // button, pane chevrons, toggles, recording rows
                AUDIO_REC_MIC_KEY => {
                    self.set_dash_slider(AUDIO_REC_MIC_KEY, x, y);
                    None
                }
                AUDIO_REC_REC_KEY => {
                    self.pending_audiorec_rec = true;
                    None
                }
                AUDIO_REC_PANE_KEY => {
                    self.audiorec_pane = 1;
                    self.audiorec_del_armed = None;
                    self.pending_audiorec_list = true;
                    None
                }
                AUDIO_REC_BACK_KEY => {
                    self.audiorec_pane = 0;
                    self.audiorec_del_armed = None;
                    None
                }
                AUDIO_REC_CONFIRM_KEY => {
                    self.audiorec_confirm_delete = !self.audiorec_confirm_delete;
                    self.save_config();
                    None
                }
                AUDIO_REC_SHOWMIC_KEY => {
                    self.audiorec_show_mic = !self.audiorec_show_mic;
                    self.save_config();
                    None
                }
                k @ AUDIO_REC_PLAY_BASE..=AUDIO_REC_PLAY_MAX => {
                    let i = (k - AUDIO_REC_PLAY_BASE) as usize;
                    if i < self.audiorec_recordings.len() {
                        self.pending_audiorec_play = Some(i);
                    }
                    None
                }
                k @ AUDIO_REC_DEL_BASE..=AUDIO_REC_DEL_MAX => {
                    let i = (k - AUDIO_REC_DEL_BASE) as usize;
                    if i < self.audiorec_recordings.len() {
                        self.audiorec_del_click(i);
                    }
                    None
                }
                // track bar: start a seek drag at the clicked fraction
                // (rect is written by the media drawer each frame)
                23 => {
                    let (sx, _, sw, _) = self.media_seek_rect;
                    if sw > 1.0 {
                        self.media_seek = Some(((x - sx) / sw).clamp(0.0, 1.0));
                        self.drag_key = Some(23);
                    }
                    None
                }
                // to-do column: 60+i toggle done · 70+i delete · 78 composer
                k @ TODO_KEY_TOGGLE..=67 => {
                    let j = (k - TODO_KEY_TOGGLE) as usize;
                    let i = self.todo_scroll.min(self.todos.len().saturating_sub(self.todo_card_visible())) + j;
                    self.todo_toggle(i);
                    None
                }
                k @ TODO_KEY_DELETE..=77 => {
                    let j = (k - TODO_KEY_DELETE) as usize;
                    let i = self.todo_scroll.min(self.todos.len().saturating_sub(self.todo_card_visible())) + j;
                    self.todo_delete(i);
                    None
                }
                TODO_KEY_INPUT => {
                    if self.todo_input.is_none() {
                        self.todo_input = Some(String::new());
                    }
                    None
                }
                // todo card: `+` add button — same as clicking the input row
                TODO_KEY_ADD => {
                    if self.todo_input.is_none() {
                        self.todo_input = Some(String::new());
                    }
                    None
                }
                // pomodoro card: start/pause · reset · focus-length steppers
                POMODORO_TOGGLE_KEY => {
                    self.pomo_toggle();
                    None
                }
                POMODORO_RESET_KEY => {
                    self.pomo_reset();
                    None
                }
                POMODORO_INC_KEY => {
                    self.pomo_adjust_focus(5);
                    None
                }
                POMODORO_DEC_KEY => {
                    self.pomo_adjust_focus(-5);
                    None
                }
                // workspaces card tiles: click → dispatch switch to Hyprland
                k @ WORKSPACE_KEY_BASE..=WORKSPACE_KEY_MAX => {
                    let ws = (k - WORKSPACE_KEY_BASE) as usize + 1; // 1-indexed
                    if ws <= self.ws_count.max(1) {
                        self.ws_prev = self.ws_active;
                        self.ws_active = ws - 1;
                        self.pending_ws_dispatch = Some(ws);
                    }
                    None
                }
                // world-clock card: row ✕ · "+" opens search · result pick
                k @ WORLDCLOCK_DEL_BASE..=WORLDCLOCK_DEL_MAX => {
                    self.worldclock_remove((k - WORLDCLOCK_DEL_BASE) as usize);
                    None
                }
                WORLDCLOCK_ADD_KEY => {
                    if self.worldclock_search.is_none() {
                        self.worldclock_search = Some(String::new());
                    }
                    None
                }
                // water tracker: + glass · − undo
                WATER_INC_KEY => {
                    self.water_add();
                    None
                }
                WATER_DEC_KEY => {
                    self.water_sub();
                    None
                }
                // compositor card: effect tiles + screenshot buttons
                COMP_BLUR_KEY => {
                    self.pending_dash = Some(DashCmd::BlurToggle);
                    None
                }
                COMP_SHADOW_KEY => {
                    self.pending_dash = Some(DashCmd::ShadowToggle);
                    None
                }
                COMP_OPACITY_KEY => {
                    self.pending_dash = Some(DashCmd::OpacityToggle);
                    None
                }
                COMP_SHOT_AREA_KEY => {
                    self.pending_shot = Some("region".to_string());
                    None
                }
                COMP_SHOT_FULL_KEY => {
                    self.pending_shot = Some("full".to_string());
                    None
                }
                // countdown card: composer focus · row delete
                COUNTDOWN_INPUT_KEY => {
                    if self.countdown_input.is_none() {
                        self.countdown_input = Some(String::new());
                    }
                    None
                }
                k @ COUNTDOWN_DEL_BASE..=COUNTDOWN_DEL_MAX => {
                    let i = (k - COUNTDOWN_DEL_BASE) as usize;
                    if i < self.countdown_events.len() {
                        self.countdown_events.remove(i);
                        self.save_countdown();
                    }
                    None
                }
                // alarms card: composer focus · row delete
                ALARM_INPUT_KEY => {
                    if self.alarm_input.is_none() {
                        self.alarm_input = Some(String::new());
                    }
                    None
                }
                k @ ALARM_DEL_BASE..=ALARM_DEL_MAX => {
                    let i = (k - ALARM_DEL_BASE) as usize;
                    if i < self.alarm_list.len() {
                        self.alarm_list.remove(i);
                        self.save_alarms();
                    }
                    None
                }
                // snippets card: composer focus · row copy · row delete
                SNIPPET_INPUT_KEY => {
                    if self.snippet_input.is_none() {
                        self.snippet_input = Some(String::new());
                    }
                    None
                }
                k @ SNIPPET_COPY_BASE..=SNIPPET_COPY_MAX => {
                    let i = (k - SNIPPET_COPY_BASE) as usize;
                    if i < self.snippets.len() {
                        self.pending_snippet_copy = Some(i);
                        self.snippet_copied = Some(i);
                        self.snippet_copied_at = Some(Instant::now());
                    }
                    None
                }
                k @ SNIPPET_DEL_BASE..=SNIPPET_DEL_MAX => {
                    let i = (k - SNIPPET_DEL_BASE) as usize;
                    if i < self.snippets.len() {
                        self.snippets.remove(i);
                        self.save_snippets();
                    }
                    None
                }
                // expenses card: quick chips · composer · clear month
                k @ EXPENSE_CHIP_BASE..=EXPENSE_CHIP_MAX => {
                    let amounts = [1u32, 5, 10, 20, 50, 100];
                    let i = (k - EXPENSE_CHIP_BASE) as usize;
                    if let Some(a) = amounts.get(i) {
                        self.expense_add(a * 100, "misc");
                    }
                    None
                }
                EXPENSE_INPUT_KEY => {
                    if self.expense_input.is_none() {
                        self.expense_input = Some(String::new());
                    }
                    None
                }
                EXPENSE_CLEAR_KEY => {
                    let today = Self::water_today();
                    let month = today[..7].to_string();
                    self.expenses.retain(|(_, _, d)| !d.starts_with(&month));
                    self.save_expenses();
                    None
                }
                // eye rest card: toggle pause/resume · reset
                EYEREST_TOGGLE_KEY => {
                    self.er_toggle();
                    None
                }
                EYEREST_RESET_KEY => {
                    self.er_reset();
                    None
                }
                // mic meter card: mute/unmute the input
                MIC_MUTE_KEY => {
                    self.pending_dash = Some(DashCmd::MicToggle);
                    None
                }
                // audio device card: row = set default · mute glyph = toggle
                k @ AUDIO_DEV_BASE..=AUDIO_DEV_MAX => {
                    let i = (k - AUDIO_DEV_BASE) as usize;
                    let (dev, is_mute) = self.audio_dev_pair(i);
                    match dev {
                        Some(id) if is_mute => self.pending_audio_mute = Some(id),
                        Some(id) => self.pending_audio_default = Some(id),
                        None => {}
                    }
                    None
                }
                // conninfo card: refresh now
                CONNINFO_REFRESH_KEY => {
                    self.poll_conninfo();
                    None
                }
                // latency card: re-probe immediately
                LATENCY_RUN_KEY => {
                    self.lat_state = 1;
                    self.lat_next = None;
                    None
                }
                // smarthealth card: refresh now
                SMART_REFRESH_KEY => {
                    self.sm_next = None;
                    None
                }
                // systemd units card: refresh now
                SYSTEMD_REFRESH_KEY => {
                    self.sus_next = None;
                    None
                }
                // journal tail card: refresh now
                JOURNAL_REFRESH_KEY => {
                    self.jr_next = None;
                    None
                }
                k @ WORLDCLOCK_PICK_BASE..=WORLDCLOCK_PICK_MAX => {
                    let pick = self.worldclock_pick_map.iter().find(|(key, _, _)| *key == k).cloned();
                    if let Some((_, tz, city)) = pick {
                        self.worldclock_pin(&tz, &city);
                        self.worldclock_search = None;
                    }
                    None
                }
                // calendar card: month paging + day pick → opens the full panel
                CAL_KEY_BASE_PREV => {
                    self.cal_offset -= 1;
                    self.cal_selected = -1;
                    None
                }
                CAL_KEY_BASE_NEXT => {
                    self.cal_offset += 1;
                    self.cal_selected = -1;
                    None
                }
                k @ CAL_KEY_BASE..=CAL_KEY_BASE_MAX => {
                    let cell = (k - CAL_KEY_BASE) as i32;
                    let (first_wday, days, _today, _) = self.calendar_info();
                    let day = cell - first_wday + 1;
                    if day >= 1 && day <= days {
                        self.cal_selected = day;
                        Some(Mode::Calendar)
                    } else {
                        None
                    }
                }
                // accent source card: pick the source like the Settings block
                ACC_LIST_BACK_KEY => {
                    self.accent_list_open = false;
                    None
                }
                ACC_LIST_CHEV_KEY => {
                    self.load_custom_accs();
                    self.accent_list_open = true;
                    self.accent_list_scroll = 0;
                    None
                }
                k @ ACC_LIST_KEY_BASE..=ACC_LIST_KEY_MAX => {
                    let j = (k - ACC_LIST_KEY_BASE) as usize;
                    let (_, _, _, hh) = self.accent_list_rect;
                    let row_h = self.scale.s(26.0);
                    let visible = ((hh - self.scale.s(4.0)) / row_h).floor().max(1.0) as usize;
                    let top = self.accent_list_scroll.min(self.custom_accs.len().saturating_sub(visible));
                    if let Some(a) = self.custom_accs.get(top + j) {
                        self.pending_accent_src = Some(("custom".into(), a.name.clone()));
                    }
                    self.accent_list_open = false;
                    None
                }
                crate::shell::ACC_LIST_COLOR_KEY => {
                    // "Custom color" in the accent-card subview → gcp GUI
                    self.pending_custom_color = true;
                    self.accent_list_open = false;
                    None
                }
                k @ ACC_KEY_BASE..=ACC_KEY_BASE_MAX => {
                    // Dashboard theme&accent card rows → `scheme_main accent
                    // wall|default|custom`. Never passes the `all` flag here —
                    // that is Settings-accent-source-only (and custom never
                    // takes `all` at all).
                    let src = match k - ACC_KEY_BASE {
                        0 => "wall",
                        1 => "default",
                        _ => "custom",
                    };
                    self.pending_accent_src = Some((src.to_string(), String::new()));
                    None
                }
                // custom-accent browser card: list/grid view toggle
                CUSTOMACC_VIEW_KEY => {
                    self.customacc_list_view = !self.customacc_list_view;
                    self.customacc_scroll = 0;
                    None
                }
                // custom-accent browser card: search bar → focus (keep query)
                CUSTOMACC_SEARCH_KEY => {
                    self.load_custom_accs();
                    if self.customacc_search.is_none() {
                        self.customacc_search = Some(String::new());
                    }
                    None
                }
                // custom-accent browser card: accent click → activate
                k @ CUSTOMACC_KEY_BASE..=CUSTOMACC_KEY_MAX => {
                    let i = (k - CUSTOMACC_KEY_BASE) as usize;
                    if let Some(&ai) = self.customacc_filtered().get(i) {
                        let a = &self.custom_accs[ai];
                        self.pending_accent_src = Some(("custom".into(), a.name.clone()));
                    }
                    None
                }
                // notes card: `+` opens the title/body composer
                NOTES_KEY_INPUT => {
                    if self.notes_input.is_none() {
                        self.load_notes_if_needed();
                        self.notes_input = Some(0);
                    }
                    None
                }
                // notes composer save button (only while the body is focused)
                NOTES_KEY_SAVE => {
                    if self.notes_input == Some(1) {
                        self.notes_commit();
                    }
                    None
                }
                // battery card: power-save toggle (CPU governor)
                BATTERY_PSAVE_KEY => {
                    self.pending_power_save = true;
                    None
                }
                // world-map card: click → (lat, lon) → nearest zone.tab
                // timezone → schedule an offset probe (`TZ=<zone> date`)
                crate::shell::WORLD_MAP_KEY => {
                    let (mx, my, mw, mh) = self.worldmap_body_rect;
                    if mw > 1.0 && mh > 1.0 {
                        let win = crate::shell::worldmap::win_for(self.world_center.0, self.world_center.1, self.world_zoom);
                        let (lon, lat) = crate::shell::worldmap::unproj(mx, my, mw, mh, &win, x, y);
                        self.world_marker = Some((lat, lon));
                        if self.world_zoom > 1.0 {
                            // zoom is anchored to the pinned marker
                            self.world_center = (lon, lat);
                        }
                        if let Some(z) = crate::shell::worldmap::nearest_zone(lat, lon).cloned() {
                            self.world_city = z.city;
                            self.world_tz = z.tz.clone();
                            self.world_abbrev.clear();
                            self.pending_world_offset = Some(z.tz);
                        }
                    }
                    None
                }
                // world-map card: zoom in/out — centered on the pinned marker, or AT the
                // cursor point when the pointer is over the map body
                crate::shell::WORLD_ZOOM_IN_KEY => {
                    if !self.over_worldmap() {
                        if let Some((lat, lon)) = self.world_marker {
                            self.world_center = (lon, lat);
                        }
                    }
                    self.world_zoom_to(self.world_zoom * 2.0);
                    None
                }
                crate::shell::WORLD_ZOOM_OUT_KEY => {
                    if !self.over_worldmap() {
                        if let Some((lat, lon)) = self.world_marker {
                            self.world_center = (lon, lat);
                        }
                    }
                    self.world_zoom_to(self.world_zoom / 2.0);
                    None
                }
                // world-map card: base download (only meaningful while the
                // embedded fallback is on screen — a real dataset replaces it)
                crate::shell::WORLD_DL_KEY => {
                    if crate::shell::worldmap::using_fallback() {
                        self.world_dl_pending = Some(crate::shell::WorldDl::Base);
                        self.world_dl_err = None;
                    }
                    None
                }
                // world-map card: download the active country chunk
                crate::shell::WORLD_REGION_DL_KEY => {
                    if let Some(iso) = self.world_region_target.clone() {
                        self.world_dl_pending = Some(crate::shell::WorldDl::Region(iso));
                        self.world_dl_err = None;
                    }
                    None
                }
                // world-map card: menu backdrop closes the overlay
                crate::shell::WORLD_MENU_KEY => {
                    self.world_menu = false;
                    None
                }
                // world-map card: save-downloads toggle (persisted)
                crate::shell::WORLD_MENU_TOGGLE_KEY => {
                    self.world_save_data = !self.world_save_data;
                    // the cache dir changed; drop stale in-memory data and
                    // region rings from the old location until they reload
                    worldmap::clear();
                    self.world_region = None;
                    self.world_region_rings = None;
                    self.world_rev = self.world_rev.wrapping_add(1);
                    self.world_region_target = None;
                    self.world_dl_pending = None;
                    self.world_dl_err = None;
                    self.save_config();
                    None
                }
                // world-map card: menu style rows
                k if (crate::shell::WORLD_STYLE_KEY_BASE..crate::shell::WORLD_STYLE_KEY_BASE + crate::shell::WORLD_PANES as u32).contains(&k) => {
                    self.world_pane = (k - crate::shell::WORLD_STYLE_KEY_BASE) as usize;
                    self.world_menu = false;
                    None
                }
                // viz card: cycle the bar style + persist it
                VIZ_KEY_BASE => {
                    self.viz_style = (self.viz_style + 1) % 3;
                    self.save_config();
                    None
                }
                // EQ card: toggle on/off
                EQ_TOGGLE_KEY => {
                    let mut eq = self.eq.lock().unwrap();
                    eq.toggle();
                    None
                }
                // EQ card: apply preset
                k @ EQ_PRESET_BASE..=EQ_PRESET_MAX => {
                    let idx = (k - EQ_PRESET_BASE) as usize;
                    let mut eq = self.eq.lock().unwrap();
                    eq.apply_preset(idx);
                    None
                }
                // EQ card: band slider (vertical drag — we use click-to-center)
                k @ EQ_KEY_BASE..=EQ_KEY_MAX => {
                    let band = (k - EQ_KEY_BASE) as usize;
                    // Calculate gain from y position relative to the card
                    let (_, eq_y, _, eq_h) = self.eq_rect;
                    let gain_range = crate::eq::GAIN_MAX - crate::eq::GAIN_MIN;
                    let frac = ((y - eq_y) / eq_h).clamp(0.0, 1.0);
                    let gain = crate::eq::GAIN_MAX - frac * gain_range;
                    let mut eq = self.eq.lock().unwrap();
                    eq.set_band(band, gain);
                    self.eq_drag = Some(band);
                    self.drag_key = Some(k);
                    None
                }
                // wallpapers card: click a thumbnail → set_wall, keep the
                // dashboard open (scrollbar page-jump regions ride above the
                // thumbnail band)
                WP_CARD_SB_UP => {
                    self.wp_card_scroll_by(-self.wp_card_page());
                    None
                }
                WP_CARD_SB_DOWN => {
                    self.wp_card_scroll_by(self.wp_card_page());
                    None
                }
                // power cards (V + H): start the hold-to-confirm; the app
                // timer fires the action when the fill completes
                k @ POWER_CARD_KEY_BASE..=POWER_CARD_KEY_MAX => {
                    self.power_hold = Some(k);
                    self.power_hold_start = Instant::now();
                    None
                }
                k @ WP_CARD_KEY_BASE..=WP_CARD_KEY_MAX => {
                    let i = (k - WP_CARD_KEY_BASE) as usize;
                    if let Some(name) = self.wallpapers.get(i) {
                        // pass the cached basename as-is to `set_wall {path}`
                        self.pending_wallpaper = Some(name.clone());
                    }
                    None
                }
                // clipboard text card: click a row → copy it back to the
                // clipboard (card stays open so you can grab several)
                k @ CLIP_KEY_BASE..=CLIP_KEY_MAX => {
                    let j = (k - CLIP_KEY_BASE) as usize;
                    let i = self.clip_scroll.min(self.clip_text.len().saturating_sub(self.clip_card_visible())) + j;
                    if i < self.clip_text.len() {
                        self.pending_clip = Some((i, false));
                    }
                    None
                }
                // copied-image card: click a thumbnail/name → copy that image
                k @ CLIPIMG_KEY_BASE..=CLIPIMG_KEY_MAX => {
                    let j = (k - CLIPIMG_KEY_BASE) as usize;
                    let i = self.clipimg_scroll.min(self.clip_images.len().saturating_sub(self.clipimg_card_visible())) + j;
                    if i < self.clip_images.len() {
                        self.pending_clip = Some((i, true));
                    }
                    None
                }
                // news card: category chip → activate it (clicking the active chip returns
                // to "All"); the active channel filters the headline list
                k @ NEWS_CAT_KEY_BASE..=NEWS_CAT_KEY_MAX => {
                    self.news_activate_cat((k - NEWS_CAT_KEY_BASE) as usize);
                    None
                }
                // news card: headline row → open the article (xdg-open); the
                // card stays open so you can keep clicking stories
                k @ NEWS_HEAD_KEY_BASE..=NEWS_HEAD_KEY_MAX => {
                    let j = (k - NEWS_HEAD_KEY_BASE) as usize;
                    let visible = self.news_card_visible();
                    let len = self.news_filtered_len();
                    let top = self.news_scroll.min(len.saturating_sub(visible));
                    self.news_open(top + j);
                    None
                }
                // wifi card: row → connect via the services plumbing
                k if (WIFI_KEY_BASE..=WIFI_KEY_BASE + 99).contains(&k) => {
                    let j = (k - WIFI_KEY_BASE) as usize;
                    let len = self.wifi_networks.len();
                    let top = self.wifi_scroll.min(len.saturating_sub(self.wifi_card_visible()));
                    if let Some((ssid, _, _, _)) = self.wifi_networks.get(top + j) {
                        self.pending_wifi = Some(ssid.clone());
                    }
                    None
                }
                // bluetooth card: row → connect/disconnect via bt plumbing
                k if (BT_KEY_BASE..=BT_KEY_BASE + 99).contains(&k) => {
                    let j = (k - BT_KEY_BASE) as usize;
                    let len = self.bt_devices.len();
                    let top = self.bt_scroll.min(len.saturating_sub(self.bt_card_visible()));
                    if let Some((name, _, _)) = self.bt_devices.get(top + j) {
                        self.pending_bt = Some(name.clone());
                    }
                    None
                }
                // speedtest card: run button → app.rs spawns the probe
                SPEED_KEY_BASE => {
                    if self.speed_state != 1 {
                        self.speed_state = 1;
                        self.pending_speed_test = true;
                    }
                    None
                }
                // recent card: row → open the file
                k if (RECENT_KEY_BASE..=RECENT_KEY_BASE + 99).contains(&k) => {
                    let j = (k - RECENT_KEY_BASE) as usize;
                    let len = self.recent_files.len();
                    let top = self.recent_scroll.min(len.saturating_sub(self.recent_card_visible()));
                    if let Some((_, path)) = self.recent_files.get(top + j) {
                        self.pending_recent_open = Some(path.clone());
                    }
                    None
                }
                // currency card: row → re-base to that currency
                k if (CURRENCY_KEY_BASE..=CURRENCY_KEY_BASE + 99).contains(&k) => {
                    let j = (k - CURRENCY_KEY_BASE) as usize;
                    pub(crate) const TBL: [(&str, &str, f64); 12] = [
                        ("USD", "Dollar", 1.0), ("EUR", "Euro", 0.9200), ("GBP", "Pound", 0.7900),
                        ("JPY", "Yen", 149.50), ("INR", "Rupee", 83.10), ("CNY", "Yuan", 7.2400),
                        ("RUB", "Ruble", 92.50), ("AUD", "AU Dollar", 1.5200), ("CAD", "CA Dollar", 1.3700),
                        ("KRW", "Won", 1347.00), ("XAU", "Gold oz", 0.000420), ("XBT", "BTC", 0.0000160),
                    ];
                    let len = TBL.len();
                    let top = self.currency_scroll.min(len.saturating_sub(self.currency_card_visible()));
                    if let Some((sym, _, _)) = TBL.get(top + j) {
                        self.currency_base = sym.to_string();
                    }
                    None
                }
                // quote card: refresh pill → kick a new fetch · save pill →
                // persist to $states/quotes with a green flash
                QUOTE_KEY_BASE => {
                    if self.quote_state != 1 {
                        self.quote_state = 1;
                        self.pending_quote_fetch = true;
                    }
                    None
                }
                QUOTE_SAVE_KEY => {
                    let text = self.quote_text.clone();
                    let author = self.quote_author.clone();
                    if !text.is_empty() {
                        self.save_quote(&text, &author);
                    }
                    None
                }
                // ticker card: row → open the asset's market page
                k if (TICKER_KEY_BASE..=TICKER_KEY_BASE + 99).contains(&k) => {
                    let j = (k - TICKER_KEY_BASE) as usize;
                    let len = self.ticker_items.len();
                    let top = self.ticker_scroll.min(len.saturating_sub(self.ticker_card_visible()));
                    if let Some((sym, _, _)) = self.ticker_items.get(top + j) {
                        let syml = sym.to_lowercase();
                        self.pending_ticker_open = Some(format!("https://www.coingecko.com/en/coins/{syml}"));
                    }
                    None
                }
                // mirror card: take-photo / record-toggle / fps-cycle buttons
                MIRROR_PHOTO_KEY => {
                    self.pending_mirror_photo = true;
                    None
                }
                MIRROR_REC_KEY => {
                    self.pending_mirror_rec = true;
                    None
                }
                // mirror-card fps pane: "show actual fps" switch (persisted)
                MIRROR_FPS_SHOW_KEY => {
                    self.mirror_show_fps = !self.mirror_show_fps;
                    self.save_config();
                    None
                }
                // mirror-card fps pane: 15/30/60 rows (restart capture + persist)
                k if k >= MIRROR_FPS_ROW_BASE && k < MIRROR_FPS_ROW_BASE + MIRROR_FPS_OPTIONS.len() as u32 => {
                    self.mirror_fps = MIRROR_FPS_OPTIONS[(k - MIRROR_FPS_ROW_BASE) as usize];
                    self.pending_mirror_fps = true;
                    None
                }
                // sliders-card pane-2 toggles: "Show balance" / "Show saturation"
                SLIDERS_TOGGLE_BALANCE_KEY => {
                    self.show_balance = !self.show_balance;
                    self.save_config();
                    None
                }
                SLIDERS_TOGGLE_SAT_KEY => {
                    self.show_saturation = !self.show_saturation;
                    self.save_config();
                    None
                }
                // ANY click inside the dashboard range is a noop — the
                // scene-declared click actions (from `~/.config/zen-shell/
                // ui/cards/*.ron` Hit items): a `Command` fires when the key
                // has NO Rust handler above — the declarative escape hatch for
                // scripts / IPC calls without a rebuild.
                k if self.scene_click_actions.contains_key(&k) => {
                    if let Some(crate::scene::SceneAction::Command(cmd)) =
                        self.scene_click_actions.get(&k)
                    {
                        let _ = std::process::Command::new("sh")
                            .args(["-c", cmd])
                            .env("PATH", "/usr/bin:/usr/local/bin:/bin")
                            .spawn();
                    }
                    None
                }
                // dashboard only ever closes via hover-out, Esc, the pill,
                // or IPC. Any such press also dismisses the connectivity
                // jump menu.
                _ => {
                    self.connectivity_open = false;
                    self.sys_menu_open = false;
                    self.sys_menu_bp_open = false;
                    None
                }
            }
            }
            Mode::ControlCenter => match key {
                1 => Some(Mode::Collapsed),       // close
                2 => {
                    self.wifi_on = !self.wifi_on;
                    self.pending_wifi_power = Some(self.wifi_on);
                    None
                }
                3 => {
                    self.bt_on = !self.bt_on;
                    self.pending_bt_power = Some(self.bt_on);
                    None
                }
                6 => Some(Mode::WifiMenu),        // > chevron → network list
                7 => Some(Mode::BtMenu),          // > chevron → device list
                8 => Some(Mode::Audio),           // per-app volume
                9 => Some(Mode::Wallpaper),       // wallpaper picker
                4 => {
                    self.volume = ((x - 16.0) / (w - 32.0)).clamp(0.0, 1.0);
                    self.drag_key = Some(4);
                    self.pending_volume = Some(self.volume);
                    None
                }
                5 => {
                    self.brightness = ((x - 16.0) / (w - 32.0)).clamp(0.0, 1.0);
                    self.drag_key = Some(5);
                    self.pending_brightness = Some(self.brightness);
                    None
                }
                // background click → collapse (C++ parity)
                _ => Some(Mode::Collapsed),
            },
            Mode::Launcher => {
                self.ensure_launcher_hits();
                // visible window is scrolled: key 10..=16 → hits[scroll + i]
                if (10..=16).contains(&key) {
                    let i = (key - 10) as usize;
                    if let Some(&idx) = self.launcher_hits.get(self.launcher_scroll + i) {
                        if let Some(app) = self.apps.apps.get(idx) {
                            let id = app.id.clone();
                            if !self.take_app_shortcut_pick(&id) {
                                self.apps.launch(&id);
                            }
                        }
                    }
                    return Some(Mode::Collapsed);
                }
                Some(Mode::Collapsed)
            }
            Mode::Notifications => match key {
                2 => {
                    self.notifs.clear();
                    self.notif_count = 0;
                    None
                }
                3 => {
                    self.dnd = !self.dnd;
                    None
                }
                10..=17 => {
                    // cards render newest-first; remove the card actually clicked
                    let i = (key - 10) as usize;
                    if i < self.notifs.len() {
                        let idx = self.notifs.len() - 1 - i;
                        self.notifs.remove(idx);
                        self.notif_count = self.notifs.len() as u32;
                    }
                    None
                }
                _ => Some(Mode::Collapsed),
            },
            Mode::Calendar => match key {
                1 => {
                    self.cal_offset -= 1;
                    self.cal_selected = -1;
                    None
                }
                2 => {
                    self.cal_offset += 1;
                    self.cal_selected = -1;
                    None
                }
                10..=51 => {
                    let cell = (key - 10) as i32;
                    let (first_wday, days, _today, _) = self.calendar_info();
                    let day = cell - first_wday + 1;
                    if day >= 1 && day <= days {
                        self.cal_selected = day;
                    }
                    None
                }
                _ => Some(Mode::Collapsed),
            },            Mode::Power => match key {
                // hold-to-confirm: the press starts a 3 s liquid fill; the app
                // timer fires the action when the fill completes, and a release
                // (or moving off the button) cancels it
                1..=5 => {
                    self.power_hold = Some(key);
                    self.power_hold_start = Instant::now();
                    None
                }
                _ => Some(Mode::Collapsed),
            },
            Mode::Osd => match key {
                // click the OSD → dismiss immediately
                _ => Some(Mode::Collapsed),
            },
            Mode::WorkspaceSwitcher => match key {
                // workspace cards: keys 10..19 → switch to workspace
                10..=19 => {
                    let ws = (key - 10) as usize;
                    if ws < self.ws_count {
                        self.ws_prev = self.ws_active;
                        self.ws_active = ws;
                        self.pending_ws_dispatch = Some(ws + 1); // 1-indexed
                    }
                    Some(Mode::Collapsed)
                }
                // background click → collapse
                _ => Some(Mode::Collapsed),
            },
            Mode::Lock => match key {
                // hold logout / restart / shutdown 3s to confirm; the password
                // field (key 1) is keyboard-driven and ignores clicks
                2..=4 => {
                    self.power_hold = Some(key);
                    self.power_hold_start = Instant::now();
                    None
                }
                // background clicks are ignored — the lock stays up
                _ => None,
            },
            Mode::PolkitAuth => match key {
                // Authenticate button (key 90)
                90 => {
                    if !self.polkit_pw.is_empty() && !self.polkit_checking {
                        self.polkit_pending = true;
                        None // app.rs handles verification
                    } else {
                        None
                    }
                }
                // Cancel button (key 91) or Esc
                91 => Some(Mode::Collapsed),
                // Password field focus (key 92) — just track focus
                92 => None,
                _ => None,
            },
            Mode::WifiMenu => match key {
                1 => Some(Mode::ControlCenter), // back
                10..=29 => {
                    let i = (key - 10) as usize;
                    if i < self.wifi_networks.len() {
                        let (ssid, _, _, _) = self.wifi_networks[i].clone();
                        self.pending_wifi = Some(ssid);
                    }
                    None
                }
                _ => None,
            },
            Mode::BtMenu => match key {
                1 => Some(Mode::ControlCenter), // back
                10..=29 => {
                    let i = (key - 10) as usize;
                    if i < self.bt_devices.len() {
                        let (name, _, _) = self.bt_devices[i].clone();
                        self.pending_bt = Some(name);
                    }
                    None
                }
                _ => None,
            },
            Mode::Weather => match key {
                // close / back is the only interaction; anything else collapses
                _ => Some(Mode::Collapsed),
            },
            Mode::Audio => match key {
                1 => Some(Mode::Collapsed), // close
                // per-app / device volume slider rows: 10..=29 → row index
                10..=29 => {
                    let i = (key - 10) as usize;
                    if let Some((id, ..)) = self.audio_rows().get(i) {
                        let sx = 44.0;
                        let sw = (w - 44.0 - 64.0).max(16.0);
                        self.pending_audio_volume = Some((*id, ((x - sx) / sw).clamp(0.0, 1.0)));
                        self.drag_key = Some(key);
                    }
                    None
                }
                // mute toggles: 40..=59 → row index
                40..=59 => {
                    let i = (key - 40) as usize;
                    if let Some((id, ..)) = self.audio_rows().get(i) {
                        self.pending_audio_mute = Some(*id);
                    }
                    None
                }
                _ => Some(Mode::Collapsed),
            },
            Mode::Wallpaper => match key {
                // thumbnail grid: click sets + collapses. Keys 10..=99 are the
                // visible window (size depends on the zoomable cell size);
                // the scroll offset picks the actual file.
                10..=99 => {
                    let i = self.wallpaper_scroll + (key - 10) as usize;
                    if let Some(name) = self.wallpapers.get(i) {
                        // cache basename (spaces and all) → pass as-is to `set_wall {path}`
                        self.pending_wallpaper = Some(name.clone());
                    }
                    Some(Mode::Collapsed)
                }
                _ => Some(Mode::Collapsed),
            },
            Mode::Themes => match key {
                // color-scheme detail subview: `<` returns to the grid
                // (stays inside Mode::Themes), anything else closes
                1 if self.theme_detail.is_some() => {
                    self.theme_detail = None;
                    Some(Mode::Themes)
                }
                // theme cards: click runs scheme_main apply <id>; the delayed
                // colors-reload picks the new palette up
                10..=99 => {
                    let hits = self.filtered_themes();
                    let i = self.themes_scroll + (key - 10) as usize;
                    if let Some(&idx) = hits.get(i) {
                        if let Some(card) = self.themes.get(idx) {
                            self.pending_theme_apply = Some(card.name.clone());
                        }
                    }
                    Some(Mode::Collapsed)
                }
                // ">" chevron: fetch + show the card's full color scheme,
                // staying open inside Mode::Themes
                100..=199 => {
                    let hits = self.filtered_themes();
                    let i = self.themes_scroll + (key - 100) as usize;
                    if let Some(&idx) = hits.get(i) {
                        if let Some(card) = self.themes.get(idx) {
                            self.pending_theme_fetch = Some(card.name.clone());
                        }
                    }
                    Some(Mode::Themes)
                }
                _ => Some(Mode::Collapsed),
            },
            Mode::Keybinds => Some(Mode::Collapsed), // read-only viewer — any click closes
            Mode::Clipboard => match key {
                // rows: click copies + collapses
                10..=49 => {
                    let i = self.clip_scroll + (key - 10) as usize;
                    if i < self.clip_text.len() {
                        self.pending_clip = Some((i, false));
                    }
                    Some(Mode::Collapsed)
                }
                _ => Some(Mode::Collapsed),
            },
            Mode::ClipboardImages => match key {
                // thumbnail grid: click copies + collapses
                10..=49 => {
                    let i = (key - 10) as usize;
                    if i < self.clip_images.len() {
                        self.pending_clip = Some((i, true));
                    }
                    Some(Mode::Collapsed)
                }
                _ => Some(Mode::Collapsed),
            },
            Mode::Settings => match key {
                1 => Some(Mode::Collapsed), // close
                // Android-style rows: tapping the row body opens the subpage,
                // tapping the switch powers the radio
                2 => Some(Mode::WifiMenu),
                3 => Some(Mode::BtMenu),
                12 => {
                    self.wifi_on = !self.wifi_on;
                    self.pending_wifi_power = Some(self.wifi_on);
                    None
                }
                13 => {
                    self.bt_on = !self.bt_on;
                    self.pending_bt_power = Some(self.bt_on);
                    None
                }
                4 => {
                    self.volume = self.slider_val(x, w);
                    self.drag_key = Some(4);
                    self.pending_volume = Some(self.volume);
                    None
                }
                5 => {
                    self.brightness = self.slider_val(x, w);
                    self.drag_key = Some(5);
                    self.pending_brightness = Some(self.brightness);
                    None
                }
                // pill margin slider (0..30 px)
                36 => {
                    self.pill_margin_x = self.slider_val(x, w) * 30.0;
                    self.drag_key = Some(36);
                    self.save_config();
                    self.refresh_size();
                    None
                }
                // pill transparency slider (0..255 alpha)
                37 => {
                    self.pill_alpha = (self.slider_val(x, w) * 255.0) as u8;
                    self.drag_key = Some(37);
                    self.save_config();
                    None
                }
                // dashboard card transparency slider (0..255 alpha)
                38 => {
                    self.dash_card_alpha = (self.slider_val(x, w) * 255.0) as u8;
                    self.drag_key = Some(38);
                    self.save_config();
                    None
                }
                // Max volume toggle (allow up to 200%)
                14 => {
                    self.allow_over_100 = !self.allow_over_100;
                    self.save_config();
                    None
                }
                // Max volume slider (0..200)
                15 => {
                    let v = (self.slider_val(x, w) * 200.0).round().clamp(0.0, 200.0) as i32;
                    if self.max_vol != v {
                        self.max_vol = v;
                        self.drag_key = Some(15);
                        self.save_config();
                    }
                    None
                }
                // balance slider reset button (dashboard + settings)
                58 => {
                    self.vol_left = self.volume;
                    self.vol_right = self.volume;
                    self.save_config();
                    None
                }
                // balance slider (settings) — same formula as dashboard
                59 => {
                    let frac = self.slider_val(x, w);
                    let mid = (self.vol_left + self.vol_right) * 0.5;
                    self.vol_left = (mid * (2.0 - 2.0 * frac)).clamp(0.0, 2.0);
                    self.vol_right = (mid * (2.0 * frac)).clamp(0.0, 2.0);
                    self.drag_key = Some(59);
                    None
                }
                // Settings → Misc: power menu hold delay slider (0.5..3.0 s)
                220 => {
                    let sx = SETTINGS_SIDEBAR + 64.0;
                    let sw = (w - SETTINGS_SIDEBAR - 96.0).max(16.0);
                    let frac = ((x - sx) / sw).clamp(0.0, 1.0);
                    self.power_hold_delay = (frac * 2.5 + 0.5).round() * 0.1;
                    self.drag_key = Some(220);
                    self.save_config();
                    None
                }
                6..=11 => {
                    // Preset circles → pick this hex as the custom accent.
                    // Routed through `pick_accent_main <hex>` (no hyprpicker)
                    // which stores it as `$states/custom_acc_<ch>` + source c,
                    // sets `$states2/acc_changed`, then restores the theme.
                    let i = (key - 6) as usize;
                    if let Some(&c) = self.sv_series.get(i) {
                        let hex = format!("#{:06x}", (c >> 8) & 0xffffff);
                        self.pending_pick_accent = Some(hex);
                    }
                    None
                }
                // Theme-accent source buttons (Appearance): from-wallpaper /
                // scheme-default. Handled by app.rs via `scheme_main accent`.
                // wall & default honor the this-state / all-states toggle: when
                // all-states is on, pass the `all` flag so scheme_main writes
                // every channel's acc_source. Custom accent NEVER takes `all`.
                60 => {
                    let all = if self.acc_all_states { "all" } else { "" };
                    self.pending_accent_src = Some(("wall".into(), all.into()));
                    None
                }
                61 => {
                    let all = if self.acc_all_states { "all" } else { "" };
                    self.pending_accent_src = Some(("default".into(), all.into()));
                    None
                }
                // Custom accent → acc_source = c (never the `all` flag — a
                // custom accent always targets the current state only)
                223 => {
                    self.pending_accent_src = Some(("custom".into(), String::new()));
                    None
                }
                // "Pick from screen" → spawn hyprpicker (discard on Esc)
                205 => {
                    self.pending_pick_accent = Some(String::new());
                    None
                }
                // custom accents from $states2/custom_acc.json (grid keys 62..)
                62..=199 => {
                    let i = (key - 62) as usize;
                    if let Some(a) = self.custom_accs.get(i) {
                        self.pending_accent_src = Some(("custom".into(), a.name.clone()));
                    }
                    None
                }
                // Settings → Appearance: accent toggles → write $states flag
                // (applied live by the "Apply changes" button → theme_main restore)
                206 => {
                    self.acc_flag_start_icon = self.toggle_acc_flag("start_icon", self.acc_flag_start_icon);
                    None
                }
                207 => {
                    self.acc_flag_app_border = self.toggle_acc_flag("app_border", self.acc_flag_app_border);
                    None
                }
                208 => {
                    self.acc_flag_hyprland = self.toggle_acc_flag("hyprland", self.acc_flag_hyprland);
                    None
                }
                209 => {
                    self.acc_flag_gtk = self.toggle_acc_flag("gtk", self.acc_flag_gtk);
                    None
                }
                210 => {
                    self.acc_flag_normal_app = self.toggle_acc_flag("normal_app", self.acc_flag_normal_app);
                    None
                }
                216 => {
                    self.acc_scrim = !self.acc_scrim;
                    self.write_state_flag("acc_scrim", if self.acc_scrim { "1" } else { "0" });
                    None
                }
                // Appearance slider rows: begin a drag (tone 211, alpha 212..=215)
                211..=215 => {
                    self.drag_key = Some(key);
                    None
                }
                // Apply changes → theme_main restore to push all edits live
                217 => {
                    self.pending_theme_cmd = Some(vec!["restore".to_string()]);
                    None
                }
                // Settings → Appearance → Icon font: pick a slot flavor.
                // app.rs swaps the text engine (family + slot table) live.
                224 => { self.set_icon_style("google"); None }
                225 => { self.set_icon_style("caskadia"); None }
                226 => { self.set_icon_style("jetbrains"); None }
                227 => { self.set_icon_style("phosphor"); None }
                // Accent-source scope (ACCENT SOURCE header): this state (228)
                // vs all states (229). Only shown when the current channel is
                // not `n`. Gates the `all` flag for wall/default rows only.
                228 => { self.acc_all_states = false; None }
                229 => { self.acc_all_states = true; None }
                // Custom-color pick: run the gcp picker GUI (via
                // `custom_color_main` wrapper). The picked hex becomes the
                // custom accent for the current state — never an `all`-flag
                // operation.
                230 => {
                    self.pending_custom_color = true;
                    None
                }
                // Custom-accent dropdown: toggle open/closed
                218 => {
                    self.custom_acc_drop_open = !self.custom_acc_drop_open;
                    if !self.custom_acc_drop_open {
                        self.custom_acc_drop_scroll = 0;
                    }
                    None
                }
                // Dropdown item: select + apply. `scheme_main accent custom
                // <name>` turns custom accent on and restores (with the
                // previously-selected value applied) + acc_changed GTK reload.
                k if k >= crate::shell::CUSTOM_ACC_KEY_BASE => {
                    let i = (k - crate::shell::CUSTOM_ACC_KEY_BASE) as usize;
                    if let Some(a) = self.custom_accs.get(i) {
                        self.custom_acc_drop_open = false;
                        self.custom_acc_drop_scroll = 0;
                        self.pending_accent_src = Some(("custom".into(), a.name.clone()));
                    }
                    None
                }
                // Pill section toggles (runtime; the collapsed pill + hover
                // behavior reflect them immediately)
                20 => {
                    self.expand_on_hover = !self.expand_on_hover;
                    self.save_config();
                    // Settings surface height follows the tip visibility
                    self.refresh_size();
                    None
                }
                // hover / click on the resting pill opens the Control Center
                // instead of the dashboard
                35 => {
                    self.cc_as_dashboard = !self.cc_as_dashboard;
                    self.save_config();
                    None
                }
                39 => {
                    self.pill_battery_pct = !self.pill_battery_pct;
                    self.save_config();
                    self.refresh_size();
                    None
                }
                21 => {
                    self.pill_ws = !self.pill_ws;
                    self.save_config();
                    self.refresh_size();
                    None
                }
                22 => {
                    self.pill_clock = !self.pill_clock;
                    self.save_config();
                    self.refresh_size();
                    None
                }
                23 => {
                    self.pill_battery = !self.pill_battery;
                    self.save_config();
                    self.refresh_size();
                    None
                }
                34 => {
                    self.pill_watts = !self.pill_watts;
                    self.save_config();
                    self.refresh_size();
                    None
                }
                27 => {
                    self.pill_brightness = !self.pill_brightness;
                    self.save_config();
                    self.refresh_size();
                    None
                }
                // ▲/▼ arrows removed — reorder is drag-only in Settings → Pill
                24 => {
                    self.pill_tray = !self.pill_tray;
                    self.save_config();
                    self.refresh_size();
                    None
                }
                25 => {
                    self.pill_settings = !self.pill_settings;
                    self.save_config();
                    self.refresh_size();
                    None
                }
                31 => {
                    self.pill_viz = !self.pill_viz;
                    self.save_config();
                    self.refresh_size();
                    None
                }
                32 => {
                    self.pill_notif = !self.pill_notif;
                    self.save_config();
                    self.refresh_size();
                    None
                }
                33 => {
                    self.pill_user = !self.pill_user;
                    self.save_config();
                    self.refresh_size();
                    None
                }
                // Floating: hovers with a margin vs. sticks flush to the edge
                28 => {
                    self.bar_floating = !self.bar_floating;
                    self.pending_floating = Some(self.bar_floating);
                    self.save_config();
                    None
                }
                // Reserve space: a persistent strip (pill height + floating
                // offset) stays reserved in every mode — no window bouncing
                29 => {
                    self.bar_reserve = !self.bar_reserve;
                    self.pending_reserve = Some(self.bar_reserve);
                    self.save_config();
                    None
                }
                // Position row: choose the screen edge the bar sits on
                SYS_EDGE_L_KEY => self.set_bar_edge(crate::shell::BarEdge::Left),
                SYS_EDGE_T_KEY => self.set_bar_edge(crate::shell::BarEdge::Top),
                SYS_EDGE_R_KEY => self.set_bar_edge(crate::shell::BarEdge::Right),
                SYS_EDGE_B_KEY => self.set_bar_edge(crate::shell::BarEdge::Bottom),
                // two-pane nav sidebar category selection
                200 => { self.settings_tab = SettingsTab::Network; None }
                201 => { self.settings_tab = SettingsTab::Sound; None }
                202 => { self.settings_tab = SettingsTab::Appearance; None }
                203 => { self.settings_tab = SettingsTab::Pill; None }
                204 => { self.settings_tab = SettingsTab::System; None }
                219 => { self.settings_tab = SettingsTab::Misc; None }
                _ => Some(Mode::Collapsed),
            },
        }
    }

    /// Slider geometry → value for the current mode. Settings rows start the
    /// track after the icon square (64px in); CC keeps the old full-width
    /// track. Kept in one place so click and drag agree with the layout.
    pub(crate) fn slider_val(&self, x: f32, w: f32) -> f32 {
        let (sx, sw) = if self.mode == Mode::Settings {
            // sliders live in the content pane, offset by the sidebar
            (SETTINGS_SIDEBAR + 64.0, (w - SETTINGS_SIDEBAR - 80.0).max(16.0))
        } else {
            (16.0, (w - 32.0).max(16.0))
        };
        ((x - sx) / sw).clamp(0.0, 1.0)
    }

    /// Map a press/drag position onto a dashboard EQ-fader slider
    /// (51..=55, horizontal): updates the local preview value and queues the
    /// script action for app.rs. Plus balance slider (59) + reset button (58).
    ///
    /// Main volume slider (52): moving it resets the L/R channel volumes to
    /// the new main volume so the mix stays balanced.
    pub(crate) fn set_dash_slider(&mut self, key: u32, x: f32, _y: f32) -> bool {
        let (fx, _fy, fw) = match key {
            59 => self.balance_rect,
            51..=55 => self.faders[(key - 51) as usize],
            AUDIO_REC_MIC_KEY => self.audiorec_mic_rect,
            _ => return false,
        };
        if fw <= 1.0 {
            return false; // sliders card not on screen
        }
        let frac = ((x - fx) / fw).clamp(0.0, 1.0);
        let cmd = match key {
            51 => {
                if (self.brightness - frac).abs() <= 0.002 {
                    return false;
                }
                self.brightness = frac;
                DashCmd::Brightness((frac * 100.0).round() as u32)
            }
            AUDIO_REC_MIC_KEY => {
                if (self.mic_level - frac).abs() <= 0.002 {
                    return false;
                }
                self.mic_level = frac;
                DashCmd::Mic((frac * 100.0).round() as u32)
            }
            52 => {
                if (self.volume - frac).abs() <= 0.002 {
                    return false;
                }
                self.volume = frac;
                // reset L/R channel sliders to the new main volume
                self.vol_left = frac;
                self.vol_right = frac;
                // periodic live flush via native wpctl (no bash per tick);
                // end_drag() syncs the $states2/vol mirror once on release
                self.pending_volume = Some(self.volume);
                self.drag_key = Some(key);
                return true; // live — flushed by run_pending_actions()
            }
            56 => {
                // left channel — moving it sets main vol = (L+R)/2 (the average)
                if (self.vol_left - frac).abs() <= 0.002 {
                    return false;
                }
                self.vol_left = frac;
                self.volume = (self.vol_left + self.vol_right) * 0.5;
                // periodic live flush — native wpctl channel-set, both sides known
                self.pending_vol_left = Some(self.vol_left);
                self.pending_vol_right = Some(self.vol_right);
                self.pending_volume = Some(self.volume);
                self.drag_key = Some(key);
                return true; // deferred — flushed by end_drag()
            }
            57 => {
                // right channel — moving it sets main vol = (L+R)/2 (the average)
                if (self.vol_right - frac).abs() <= 0.002 {
                    return false;
                }
                self.vol_right = frac;
                self.volume = (self.vol_left + self.vol_right) * 0.5;
                // periodic live flush — native wpctl channel-set, both sides known
                self.pending_vol_left = Some(self.vol_left);
                self.pending_vol_right = Some(self.vol_right);
                self.pending_volume = Some(self.volume);
                self.drag_key = Some(key);
                return true; // deferred — flushed by end_drag()
            }
            59 => {
                // balance slider: 0..1 centered at 0.5 → L = base*(2-2b), R = base*(2b)
                // keep the midpoint (L+R)/2 constant, skew the split. NOTE: the
                // multiplier range is 0..2 — clamping it to ≤1 capped the loud
                // channel at `mid` and the pan never became a true full pan.
                let mid = (self.vol_left + self.vol_right) * 0.5;
                let new_l = (mid * (2.0 - 2.0 * frac)).clamp(0.0, 2.0);
                let new_r = (mid * (2.0 * frac)).clamp(0.0, 2.0);
                if (self.vol_left - new_l).abs() <= 0.002 && (self.vol_right - new_r).abs() <= 0.002 {
                    return false;
                }
                self.vol_left = new_l;
                self.vol_right = new_r;
                // periodic live flush — motion handler re-applies every 30 ms
                self.pending_vol_left = Some(self.vol_left);
                self.pending_vol_right = Some(self.vol_right);
                self.drag_key = Some(key);
                return true; // deferred — flushed by end_drag()
            }
            53 => {
                if (self.mic_level - frac).abs() <= 0.002 {
                    return false;
                }
                self.mic_level = frac;
                DashCmd::Mic((frac * 100.0).round() as u32)
            }
            54 => {
                let n = ((frac * 200.0).round().clamp(0.0, 200.0)) as i32;
                if self.saturation == n && self.drag_key != Some(key) {
                    return false;
                }
                self.saturation = n;
                self.drag_key = Some(key); // release (end_drag) applies it
                return true; // deferred — flushed by end_drag()
            }
            55 => {
                let k = ((1200.0 + frac * 5300.0).round() as i32).clamp(1200, 6500);
                if self.sunset_kelvin == k && self.drag_key != Some(key) {
                    return false;
                }
                self.sunset_kelvin = k;
                self.drag_key = Some(key); // release (end_drag) applies it
                return true; // deferred — flushed by end_drag()
            }
            _ => return false,
        };
        self.drag_key = Some(key);
        self.pending_dash = Some(cmd);
        true
    }

    /// Drag a slider while the pointer button is held (motion during a
    /// press). Uses the key captured at press time so the drag keeps working
    /// even if the cursor wanders off the thin slider row. Returns true when
    /// a value changed (caller redraws + applies to the backend).
    pub(crate) fn drag(&mut self, x: f32, y: f32) -> bool {
        let key = self.drag_key.unwrap_or_else(|| self.hover_key_at(x, y));
        let w = self.target_size().0;
        match self.mode {
            Mode::ControlCenter | Mode::Settings => match key {
                4 => {
                    let v = self.slider_val(x, w);
                    if (self.volume - v).abs() > 0.002 {
                        self.volume = v;
                        // reset L/R channels when main volume moves
                        self.vol_left = v;
                        self.vol_right = v;
                        self.pending_volume = Some(v);
                        true
                    } else {
                        false
                    }
                }
                5 => {
                    let v = self.slider_val(x, w);
                    if (self.brightness - v).abs() > 0.002 {
                        self.brightness = v;
                        self.pending_brightness = Some(v);
                        true
                    } else {
                        false
                    }
                }
                // pill margin slider drag
                36 => {
                    let new_val = self.slider_val(x, w) * 30.0;
                    if (self.pill_margin_x - new_val).abs() > 0.1 {
                        self.pill_margin_x = new_val;
                        self.save_config();
                        self.refresh_size();
                        true
                    } else {
                        false
                    }
                }
                // pill transparency slider drag
                37 => {
                    let new_val = (self.slider_val(x, w) * 255.0) as u8;
                    if self.pill_alpha != new_val {
                        self.pill_alpha = new_val;
                        self.save_config();
                        true
                    } else {
                        false
                    }
                }
                // max volume slider drag (0..200)
                15 => {
                    let new_val = (self.slider_val(x, w) * 200.0).round().clamp(0.0, 200.0) as i32;
                    if self.max_vol != new_val {
                        self.max_vol = new_val;
                        self.save_config();
                        true
                    } else {
                        false
                    }
                }
                // dashboard card transparency slider drag
                38 => {
                    let new_val = (self.slider_val(x, w) * 255.0) as u8;
                    if self.dash_card_alpha != new_val {
                        self.dash_card_alpha = new_val;
                        self.save_config();
                        true
                    } else {
                        false
                    }
                }
                // balance slider drag (settings + dashboard)
                59 => {
                    let frac = self.slider_val(x, w);
                    let mid = (self.vol_left + self.vol_right) * 0.5;
                    let new_l = (mid * (2.0 - 2.0 * frac)).clamp(0.0, 2.0);
                    let new_r = (mid * (2.0 * frac)).clamp(0.0, 2.0);
                    if (self.vol_left - new_l).abs() > 0.002 || (self.vol_right - new_r).abs() > 0.002 {
                        self.vol_left = new_l;
                        self.vol_right = new_r;
                        // periodic live flush — motion handler re-applies every 30 ms
                        self.pending_vol_left = Some(self.vol_left);
                        self.pending_vol_right = Some(self.vol_right);
                        true
                    } else {
                        false
                    }
                }
                // Settings → Misc: power menu hold delay slider drag (0.5..3.0 s)
                220 => {
                    let sx = SETTINGS_SIDEBAR + 64.0;
                    let sw = (w - SETTINGS_SIDEBAR - 96.0).max(16.0);
                    let frac = ((x - sx) / sw).clamp(0.0, 1.0);
                    let new = (frac * 2.5 + 0.5).round() * 0.1;
                    if (self.power_hold_delay - new).abs() > 0.001 {
                        self.power_hold_delay = new;
                        self.save_config();
                        true
                    } else {
                        false
                    }
                }
                // Settings → Appearance slider rows (tone 211, alpha 212..=215)
                // share the content-pane geometry: track x = sidebar+64,
                // width = w − sidebar − 96 (mirrors layout_settings_appearance)
                211 => {
                    let sx = SETTINGS_SIDEBAR + 64.0;
                    let sw = (w - SETTINGS_SIDEBAR - 96.0).max(16.0);
                    let frac = ((x - sx) / sw).clamp(0.0, 1.0);
                    let new = (frac * 900.0).round() as u32;
                    if self.start_icon_tone != new {
                        self.start_icon_tone = new;
                        self.write_state_flag("start_icon_tone", &new.to_string());
                        true
                    } else {
                        false
                    }
                }
                212 => {
                    let sx = SETTINGS_SIDEBAR + 64.0;
                    let sw = (w - SETTINGS_SIDEBAR - 96.0).max(16.0);
                    let new = ((x - sx) / sw).clamp(0.0, 1.0);
                    let new = (new * 255.0).round() as u8;
                    if self.bg_alpha != new {
                        self.bg_alpha = new;
                        self.write_state_flag("bg_alpha", &format!("{:02x}", new));
                        true
                    } else {
                        false
                    }
                }
                213 => {
                    let sx = SETTINGS_SIDEBAR + 64.0;
                    let sw = (w - SETTINGS_SIDEBAR - 96.0).max(16.0);
                    let new = ((x - sx) / sw).clamp(0.0, 1.0);
                    let new = (new * 255.0).round() as u8;
                    if self.acc_alpha != new {
                        self.acc_alpha = new;
                        self.write_state_flag("acc_alpha", &format!("{:02x}", new));
                        true
                    } else {
                        false
                    }
                }
                214 => {
                    let sx = SETTINGS_SIDEBAR + 64.0;
                    let sw = (w - SETTINGS_SIDEBAR - 96.0).max(16.0);
                    let new = ((x - sx) / sw).clamp(0.0, 1.0);
                    let new = (new * 255.0).round() as u8;
                    if self.scrim_alpha != new {
                        self.scrim_alpha = new;
                        self.write_state_flag("scrim_alpha", &format!("{:02x}", new));
                        true
                    } else {
                        false
                    }
                }
                215 => {
                    let sx = SETTINGS_SIDEBAR + 64.0;
                    let sw = (w - SETTINGS_SIDEBAR - 96.0).max(16.0);
                    let new = ((x - sx) / sw).clamp(0.0, 1.0);
                    let new = (new * 255.0).round() as u8;
                    if self.border_alpha != new {
                        self.border_alpha = new;
                        self.write_state_flag("border_alpha", &format!("{:02x}", new));
                        true
                    } else {
                        false
                    }
                }
                _ => false,
            },
            Mode::Audio => {
                if (10..=29).contains(&key) {
                    let i = (key - 10) as usize;
                    if let Some((id, ..)) = self.audio_rows().get(i) {
                        let sx = 44.0;
                        let sw = (w - 44.0 - 64.0).max(16.0);
                        let v = ((x - sx) / sw).clamp(0.0, 1.0);
                        self.pending_audio_volume = Some((*id, v));
                        return true;
                    }
                }
                false
            }
            // dashboard track bar: live seek preview while dragging; the
            // vertical iOS sliders share the same press-drag flow
            Mode::Expanded => {
                // EDIT MODE: move / resize follow the cursor — the canvas
                // auto-fits (grows live, trims on commit). Also drives the
                // banner-chip reorder drag.
                if self.dash_edit && (self.edit_drag.is_some() || self.edit_resize.is_some() || self.banner_drag.is_some()) {
                    return self.edit_motion(x, y);
                }
                // manual banner-width slider (edit-mode strip-editor row):
                // live width follows the cursor across the track
                if key == BANNER_W_SLIDER_KEY {
                    let r = self.banner_w_slider_rect;
                    if r.2 <= 1.0 {
                        return false;
                    }
                    let v = ((x - r.0) / r.2).clamp(0.0, 1.0);
                    let nw = (v * 6000.0).round() as i32;
                    if nw != self.banner_w {
                        self.banner_w = nw;
                        self.save_config();
                        self.refresh_size();
                        return true;
                    }
                    return false;
                }
                if key == 23 {
                    let (sx, _, sw, _) = self.media_seek_rect;
                    if sw > 1.0 {
                        let frac = ((x - sx) / sw).clamp(0.0, 1.0);
                        if (self.media_seek.unwrap_or(0.0) - frac).abs() > 0.002 {
                            self.media_seek = Some(frac);
                            return true;
                        }
                    }
                    return false;
                }
                if (51..=59).contains(&key) {
                    return self.set_dash_slider(key, x, y);
                }
                // audio recorder card mic fader — own geometry / mic level
                if key == AUDIO_REC_MIC_KEY {
                    return self.set_dash_slider(AUDIO_REC_MIC_KEY, x, y);
                }
                // EQ band vertical drag
                if (EQ_KEY_BASE..=EQ_KEY_MAX).contains(&key) {
                    let band = (key - EQ_KEY_BASE) as usize;
                    let (_, eq_y, _, eq_h) = self.eq_rect;
                    if eq_h > 1.0 {
                        let gain_range = crate::eq::GAIN_MAX - crate::eq::GAIN_MIN;
                        let frac = ((y - eq_y) / eq_h).clamp(0.0, 1.0);
                        let gain = crate::eq::GAIN_MAX - frac * gain_range;
                        let mut eq = self.eq.lock().unwrap();
                        eq.set_band(band, gain);
                        return true;
                    }
                }
                false
            }
            _ => false,
        }
    }

    /// End a slider drag (pointer released at x/y). Flushes the deferred
    /// backend command for the saturation / Kelvin sliders (one apply per
    /// gesture). Returns true when an edit-mode drop changed the layout
    /// (caller should re-morph to the new auto-trimmed canvas extent).
    pub(crate) fn end_drag(&mut self, x: f32, y: f32) -> bool {
        // edit-mode drags commit (or snap back) on release
        if self.dash_edit && (self.edit_drag.is_some() || self.edit_resize.is_some() || self.banner_drag.is_some()) {
            return self.edit_release(x, y);
        }
        match self.drag_key {
            Some(54) => {
                self.pending_dash = Some(DashCmd::Saturation(self.saturation));
                // shader_main swaps the Hyprland shader AND notify-sends —
                // grant a collapse grace while that settles
                self.drag_grace_until = std::time::Instant::now() + std::time::Duration::from_millis(2500);
            }
            Some(55) => {
                self.pending_dash = Some(DashCmd::Kelvin(self.sunset_kelvin));
                // hs_main restarts hyprsunset AND notify-sends
                self.drag_grace_until = std::time::Instant::now() + std::time::Duration::from_millis(2500);
            }
            Some(52) => {
                // main volume released — sync $states2/vol (and clamp) once
                // via audio_main, matching the Settings slider's release.
                self.pending_dash = Some(DashCmd::Volume(
                    (self.volume * 100.0).round() as u32,
                ));
            }
            Some(56) => {
                // left channel released — flush main volume (L+R)/2 + the L channel
                self.pending_volume = Some(self.volume);
                self.pending_vol_left = Some(self.vol_left);
            }
            Some(57) => {
                // right channel released — flush main volume (L+R)/2 + the R channel
                self.pending_volume = Some(self.volume);
                self.pending_vol_right = Some(self.vol_right);
            }
            Some(59) => {
                // balance slider released — flush L + R channels + main vol
                self.pending_volume = Some(self.volume);
                self.pending_vol_left = Some(self.vol_left);
                self.pending_vol_right = Some(self.vol_right);
            }
            Some(k) if (EQ_KEY_BASE..=EQ_KEY_MAX).contains(&k) => {
                self.eq_drag = None;
            }
            _ => {}
        }
        self.drag_key = None;
        false
    }

    /// Begin a pill-item reorder drag. `x` is the pointer x in surface-local
    /// coords; the method checks whether it falls on a left-cluster item and
    /// starts tracking the drag. Returns true when a drag begins.
    pub(crate) fn begin_pill_drag(&mut self, x: f32, y: f32) -> bool {
        // Reordering lives in Settings only (vertical rows + ▲/▼ arrows);
        // the collapsed pill itself is never draggable.
        if self.mode == Mode::Settings {
            let key = self.hover_key_at(x, y);
            // settings pill-item toggle keys → find in pill_order
            let Some(item) = Self::settings_key_to_pill(key) else {
                return false;
            };
            // slot may be absent (e.g. a new item never in the order yet) —
            // still begin the drag so a click-without-move toggles it on and
            // appends it to the order.
            let slot = self.pill_order.iter().position(|&p| p == item).unwrap_or(usize::MAX);
            self.pill_drag = Some(PillDrag { item, from_idx: slot, x, y, sx: x, sy: y, swapped: false });
            self.drag_key = Some(99);
            true
        } else {
            false
        }
    }

    /// Map a settings toggle key to the PillItem it represents (only
    /// reorderable pill items; non-pill keys return None).
    pub(crate) fn settings_key_to_pill(key: u32) -> Option<PillItem> {
        match key {
            21 => Some(PillItem::Workspaces),
            22 => Some(PillItem::Clock),
            23 => Some(PillItem::Battery),
            24 => Some(PillItem::Tray),
            31 => Some(PillItem::Visualizer),
            25 => Some(PillItem::Settings),
            32 => Some(PillItem::Bell),
            33 => Some(PillItem::User),
            34 => Some(PillItem::Watts),
            40 => Some(PillItem::WorkspacesLong),
            41 => Some(PillItem::WorkspacesShort),
            42 => Some(PillItem::Wifi),
            43 => Some(PillItem::Bluetooth),
            44 => Some(PillItem::Volume),
            45 => Some(PillItem::Wallpaper),
            46 => Some(PillItem::Themes),
            47 => Some(PillItem::Branding),
            _ => None,
        }
    }

    /// Settings rows show pill items in this visual order (top → bottom);
    /// shared by row rendering, drag-reorder and the ▲/▼ arrows.
    pub(crate) const PILL_ROW_ORDER: [PillItem; 17] = [
        PillItem::Workspaces,
        PillItem::Clock,
        PillItem::Battery,
        PillItem::Tray,
        PillItem::Visualizer,
        PillItem::Settings,
        PillItem::Bell,
        PillItem::User,
        PillItem::Watts,
        PillItem::WorkspacesLong,
        PillItem::WorkspacesShort,
        PillItem::Wifi,
        PillItem::Bluetooth,
        PillItem::Volume,
        PillItem::Wallpaper,
        PillItem::Themes,
        PillItem::Branding,
    ];

    /// Update a pill-item reorder drag. Swaps the dragged item in
    /// `pill_order` when the pointer crosses an adjacent slot boundary.
    /// Returns true when the order changed (caller redraws).
    pub(crate) fn drag_pill(&mut self, x: f32, y: f32) -> bool {
        let Some(ref mut drag) = self.pill_drag else { return false };
        drag.x = x;
        drag.y = y;
        let from_idx = drag.from_idx;

        if self.mode == Mode::Settings {
            // Pill reorderable rows are drawn at y = 422 - pill_scroll + row*40
            // in the two-pane content pane (see layout_settings_pill; the −6
            // header shift and the expand-tip offset are included there). Map
            // the visual row to a PillItem, then find it in pill_order.
            pub(crate) const PILL_TOGGLE_START: f32 = 422.0; // first pill-item toggle y (unscrolled)
            let pill_toggles: [PillItem; 17] = Self::PILL_ROW_ORDER;
            let ry0 = PILL_TOGGLE_START - self.pill_scroll
                + if self.expand_on_hover { SETTINGS_EXPAND_TIP_H } else { 0.0 };
            let target = (0..17u32).find(|&i| {
                let ry = ry0 + i as f32 * 40.0;
                y >= ry - 2.0 && y < ry + 36.0
            });
            if let Some(ti) = target {
                let target_item = pill_toggles[ti as usize];
                if let Some(to_idx) = self.pill_order.iter().position(|&p| p == target_item) {
                    if from_idx != to_idx && from_idx < self.pill_order.len() {
                        self.pill_order.swap(from_idx, to_idx);
                        drag.from_idx = to_idx;
                        drag.swapped = true;
                        return true;
                    }
                }
            }
        }
        false
    }

    /// End a pill-item reorder drag. Clears the drag state. If no swap
    /// happened during the drag (it was just a click), toggle the pill item
    /// on/off instead. Always persists the config.
    pub(crate) fn end_pill_drag(&mut self) {
        let drag = self.pill_drag.take();
        self.drag_key = None;
        if let Some(ref d) = drag {
            let moved = {
                let dx = d.x - d.sx;
                let dy = d.y - d.sy;
                dx * dx + dy * dy > 16.0
            };
            // click without drag → toggle the pill item (a real drag that
            // crossed no slot boundary is reorder-only, never a toggle)
            if !d.swapped && !moved && self.mode == Mode::Settings {
                match d.item {
                    PillItem::Workspaces => self.pill_ws = !self.pill_ws,
                    PillItem::Clock => self.pill_clock = !self.pill_clock,
                    PillItem::Battery => self.pill_battery = !self.pill_battery,
                    PillItem::Watts => self.pill_watts = !self.pill_watts,
                    PillItem::Tray => self.pill_tray = !self.pill_tray,
                    PillItem::Visualizer => self.pill_viz = !self.pill_viz,
                    PillItem::Settings => self.pill_settings = !self.pill_settings,
                    PillItem::Bell => self.pill_notif = !self.pill_notif,
                    PillItem::User => self.pill_user = !self.pill_user,
                    PillItem::WorkspacesLong => self.pill_ws_long = !self.pill_ws_long,
                    PillItem::WorkspacesShort => self.pill_ws_short = !self.pill_ws_short,
                    PillItem::Wifi => self.pill_wifi = !self.pill_wifi,
                    PillItem::Bluetooth => self.pill_bluetooth = !self.pill_bluetooth,
                    PillItem::Volume => self.pill_volume = !self.pill_volume,
                    PillItem::Wallpaper => self.pill_wallpaper = !self.pill_wallpaper,
                    PillItem::Themes => self.pill_themes = !self.pill_themes,
                    PillItem::Branding => {
                        self.pill_branding = !self.pill_branding;
                        if self.pill_branding && !self.pill_order.contains(&PillItem::Branding) {
                            self.pill_order.push(PillItem::Branding);
                        }
                    }
                }
            }
        }
        self.save_config();
    }

    /// when the offset actually moved.
    pub(crate) fn dash_scroll(&mut self, vr: f32, hr: f32) -> bool {
        let (cw, chh) = (self.dash_cell_base_w(self.cfg.expanded_w()), self.dash_cell_h(self.cfg.expanded_w()));
        let before = (self.dash_scroll_x, self.dash_scroll_y);
        self.dash_scroll_y += vr * (chh + self.dash_gap());
        self.dash_scroll_x += hr * (cw + self.dash_gap());
        self.clamp_scroll();
        let moved = before != (self.dash_scroll_x, self.dash_scroll_y);
        if moved {
            self.dash_sb_last = Some(std::time::Instant::now());
        }
        moved
    }

    /// Current opacity of the dashboard scrollbar overlay, purely from state —
    /// no side effects (computed at frame time). Visible while a grab-drag is
    /// active, fading out DASH_SB_KEEP seconds after the last scroll.
    pub(crate) fn dash_sb_alpha(&self, now: std::time::Instant) -> f32 {
        if self.dash_sb_drag.is_some() {
            return 1.0;
        }
        let Some(t0) = self.dash_sb_last else { return 0.0 };
        let el = now.duration_since(t0).as_secs_f64();
        if el < DASH_SB_KEEP {
            1.0
        } else if el < DASH_SB_KEEP + DASH_SB_FADE {
            (1.0 - (el - DASH_SB_KEEP) / DASH_SB_FADE) as f32
        } else {
            0.0
        }
    }

    /// True while the scrollbar overlay still needs frames: the reveal window
    /// is open (or fading) and nothing forces it visible. The app keeps its
    /// 16 ms tick alive during this so the fade-out animates instead of
    /// stalling on the last event-driven frame.
    pub(crate) fn dash_sb_pending_frames(&self, now: std::time::Instant) -> bool {
        if self.mode != Mode::Expanded || self.dash_sb_drag.is_some() {
            return false;
        }
        let Some(t0) = self.dash_sb_last else { return false };
        now.duration_since(t0).as_secs_f64() < DASH_SB_KEEP + DASH_SB_FADE
    }

    /// Press on the dashboard scrollbar strip: begin a grab-drag in the
    /// matching axis. Returns false when the press missed both strips
    /// (normal click handling proceeds).
    pub(crate) fn begin_dash_sb_drag(&mut self, x: f32, y: f32) -> bool {
        if self.mode != Mode::Expanded {
            return false;
        }
        let over_x = (self.dash_canvas_w() - self.dash_view_w()).max(0.0);
        let over_y = (self.dash_canvas_h() - self.dash_view_h()).max(0.0);
        // vertical strip: right edge of the viewport
        if over_y > 1.0 {
            let vw = self.dash_view_w();
            let vh = self.dash_view_h();
            let tw = self.scale.s(6.0);
            let tx = vw - tw - self.scale.s(4.0);
            let ty = (self.grid_top() + self.scale.s(4.0)).min(vh - tw - self.scale.s(8.0));
            let th = (vh - ty - self.scale.s(6.0)).max(20.0);
            if x >= tx - self.scale.s(5.0) && x <= tx + tw + self.scale.s(5.0) && y >= ty && y <= ty + th {
                self.dash_sb_last = Some(std::time::Instant::now());
                self.dash_sb_drag = Some((0, self.dash_scroll_y));
                return true;
            }
        }
        // horizontal strip: bottom edge of the viewport
        if over_x > 1.0 {
            let vw = self.dash_view_w();
            let vh = self.dash_view_h();
            let thh = self.scale.s(6.0);
            let tyy = vh - thh - self.scale.s(4.0);
            let tx = self.scale.s(6.0);
            let tw = (vw - self.scale.s(16.0)).max(20.0);
            if x >= tx && x <= tx + tw && y >= tyy - self.scale.s(5.0) && y <= tyy + thh + self.scale.s(5.0) {
                self.dash_sb_last = Some(std::time::Instant::now());
                self.dash_sb_drag = Some((1, self.dash_scroll_x));
                return true;
            }
        }
        false
    }

    /// Move a dashboard scrollbar drag: map the pointer through the track
    /// into a scroll offset. Returns true when the offset moved.
    pub(crate) fn drag_dash_sb(&mut self, x: f32, y: f32) -> bool {
        let Some((axis, _)) = self.dash_sb_drag else { return false };
        let over_x = (self.dash_canvas_w() - self.dash_view_w()).max(0.0);
        let over_y = (self.dash_canvas_h() - self.dash_view_h()).max(0.0);
        let before = (self.dash_scroll_x, self.dash_scroll_y);
        if axis == 0 && over_y > 1.0 {
            let vh = self.dash_view_h();
            let tw = self.scale.s(6.0);
            let ty = (self.grid_top() + self.scale.s(4.0)).min(vh - tw - self.scale.s(8.0));
            let th = (vh - ty - self.scale.s(6.0)).max(20.0);
            let thumb_h = (th * (vh / self.dash_canvas_h()).min(1.0)).max(20.0);
            let t = ((y - ty - thumb_h / 2.0) / (th - thumb_h).max(1.0)).clamp(0.0, 1.0);
            self.dash_scroll_y = t * over_y;
        } else if axis == 1 && over_x > 1.0 {
            let vw = self.dash_view_w();
            let tx = self.scale.s(6.0);
            let tw = (vw - self.scale.s(16.0)).max(20.0);
            let thumb_w = (tw * (vw / self.dash_canvas_w()).min(1.0)).max(30.0);
            let t = ((x - tx - thumb_w / 2.0) / (tw - thumb_w).max(1.0)).clamp(0.0, 1.0);
            self.dash_scroll_x = t * over_x;
        }
        let moved = before != (self.dash_scroll_x, self.dash_scroll_y);
        if moved {
            self.dash_sb_last = Some(std::time::Instant::now());
        }
        moved
    }

    /// End a dashboard scrollbar drag (pointer released).
    pub(crate) fn end_dash_sb_drag(&mut self) {
        self.dash_sb_drag = None;
    }

    /// Handle a key. `keysym` is the xkb keysym (ASCII chars for printables).
    pub(crate) fn key(&mut self, keysym: u32) -> Option<Mode> {
        // lockscreen: password entry. A real lock never dismisses on Esc — it
        // only clears the field; Enter hands the password to the app to verify.
        if self.mode == Mode::Lock {
            match keysym {
                0xff1b => self.lock_pw.clear(),
                0xff08 => {
                    self.lock_pw.pop();
                }
                // Enter hands the password to the app — but only when a check
                // isn't already in flight (no double-submit while PAM runs)
                0xff0d => {
                    if !self.lock_checking {
                        self.pending_lock_auth = true;
                    }
                }
                32..=126 => {
                    if !self.lock_checking && self.lock_pw.len() < 128 {
                        self.lock_pw.push(char::from_u32(keysym).unwrap_or(' '));
                    }
                }
                _ => {}
            }
            return None;
        }
        // polkit password dialog: same keyboard flow as the lockscreen — Esc
        // clears the field, Enter hands the password to the app when a check
        // isn't already in flight (no double-submit while the worker runs).
        if self.mode == Mode::PolkitAuth {
            match keysym {
                0xff1b => self.polkit_pw.clear(),
                0xff08 => {
                    self.polkit_pw.pop();
                }
                0xff0d => {
                    if !self.polkit_checking {
                        self.polkit_pending = true;
                    }
                }
                32..=126 => {
                    if !self.polkit_checking && self.polkit_pw.len() < 128 {
                        self.polkit_pw.push(char::from_u32(keysym).unwrap_or(' '));
                    }
                }
                _ => {}
            }
            return None;
        }
        // dashboard notes composer: two stages — Some(0) title, Some(1) body.
        // Enter on the title advances to the body; Enter on the body commits
        // the note and saves; Esc blurs. While focused it swallows typing.
        if self.mode == Mode::Expanded && self.notes_input.is_some() {
            match keysym {
                0xff1b => self.notes_blur(),
                0xff0d => {
                    if self.notes_input == Some(0) {
                        if !self.notes_title.trim().is_empty() {
                            self.notes_input = Some(1);
                        }
                    } else {
                        self.notes_commit();
                    }
                }
                0xff08 => {
                    let b = if self.notes_input == Some(0) { &mut self.notes_title } else { &mut self.notes_body };
                    b.pop();
                }
                32..=126 => {
                    let (b, max) = if self.notes_input == Some(0) {
                        (&mut self.notes_title, 60)
                    } else {
                        (&mut self.notes_body, 160)
                    };
                    if b.len() < max {
                        b.push(char::from_u32(keysym).unwrap_or(' '));
                    }
                }
                _ => {}
            }
            return None;
        }
        // dashboard world-clock search: while focused it swallows typing
        // (Esc closes, printable keys feed the filter)
        if self.mode == Mode::Expanded && self.worldclock_search.is_some() {
            match keysym {
                0xff1b => self.worldclock_search = None, // close
                0xff08 => {
                    if let Some(buf) = self.worldclock_search.as_mut() {
                        buf.pop();
                    }
                }
                32..=126 => {
                    if let Some(buf) = self.worldclock_search.as_mut() {
                        if buf.len() < 48 {
                            buf.push(char::from_u32(keysym).unwrap_or(' '));
                        }
                    }
                }
                _ => {}
            }
            return None;
        }
        // dashboard custom-accent search: while focused it swallows typing
        // (Esc blurs, Backspace pops, printable keys live-filter the list)
        if self.mode == Mode::Expanded && self.customacc_search.is_some() {
            match keysym {
                0xff1b => {
                    self.customacc_search = None;
                    self.customacc_scroll = 0;
                }
                0xff08 => {
                    if let Some(buf) = self.customacc_search.as_mut() {
                        buf.pop();
                        self.customacc_scroll = 0;
                    }
                }
                32..=126 => {
                    if let Some(buf) = self.customacc_search.as_mut() {
                        if buf.len() < 60 {
                            buf.push(char::from_u32(keysym).unwrap_or(' '));
                            self.customacc_scroll = 0;
                        }
                    }
                }
                _ => {}
            }
            return None;
        }
        // dashboard text composers (countdown / alarms / snippets / expenses):
        // while focused each swallows typing — Enter commits + blurs, Esc blurs
        if self.mode == Mode::Expanded {
            macro_rules! composer {
                ($field:ident, $commit:ident) => {
                    if self.$field.is_some() {
                        match keysym {
                            0xff1b => self.$field = None,
                            0xff0d => {
                                self.$commit();
                                self.$field = None;
                            }
                            0xff08 => {
                                if let Some(buf) = self.$field.as_mut() {
                                    buf.pop();
                                }
                            }
                            32..=126 => {
                                if let Some(buf) = self.$field.as_mut() {
                                    if buf.len() < 80 {
                                        buf.push(char::from_u32(keysym).unwrap_or(' '));
                                    }
                                }
                            }
                            _ => {}
                        }
                        return None;
                    }
                };
            }
            composer!(countdown_input, countdown_add);
            composer!(alarm_input, alarm_add);
            composer!(snippet_input, snippet_add);
            composer!(expense_input, expense_commit);
        }
        // dashboard to-do composer: while focused it swallows all typing
        // (Enter commits and keeps focus for chaining; Esc blurs)
        if self.mode == Mode::Expanded && self.todo_input.is_some() {
            match keysym {
                0xff1b => self.todo_input = None, // blur
                0xff0d => self.todo_commit(),
                0xff08 => {
                    if let Some(buf) = self.todo_input.as_mut() {
                        buf.pop();
                    }
                }
                32..=126 => {
                    if let Some(buf) = self.todo_input.as_mut() {
                        if buf.len() < 64 {
                            buf.push(char::from_u32(keysym).unwrap_or(' '));
                        }
                    }
                }
                _ => {}
            }
            return None;
        }
        // Escape collapses any widget / panel / the expanded dashboard back to
        // the pill (C++ parity: "Esc collapses, hover state stays"). In
        // dashboard EDIT MODE it leaves editing first.
        if keysym == 0xff1b {
            if self.mode == Mode::Expanded && self.dash_edit {
                self.toggle_dash_edit();
                return None;
            }
            // theme color-detail subview: Esc returns to the grid first
            if self.mode == Mode::Themes && self.theme_detail.is_some() {
                self.theme_detail = None;
                return Some(Mode::Themes);
            }
            return match self.mode {
                Mode::Collapsed => None,
                _ => {
                    if self.mode == Mode::Launcher {
                        self.app_shortcut_pick = None;
                    }
                    Some(Mode::Collapsed)
                }
            };
        }
        // Everything below is text-entry / navigation for the search panels
        // and the clipboard lists.
        match self.mode {
            Mode::Launcher | Mode::Clipboard | Mode::ClipboardImages | Mode::Themes => {}
            _ => return None,
        }
        let is_clip = matches!(self.mode, Mode::Clipboard | Mode::ClipboardImages);
        let is_themes = self.mode == Mode::Themes;
        match keysym {
            0xff0d => {
                // Return — launch (launcher), run (command), copy (clipboard)
                if is_themes && self.theme_detail.is_some() {
                    // color-detail subview: Enter returns to the grid
                    self.theme_detail = None;
                    return Some(Mode::Themes);
                }
                if is_themes {
                    // apply the first filtered theme and close
                    let hits = self.filtered_themes();
                    if let Some(&idx) = hits.first() {
                        if let Some(card) = self.themes.get(idx) {
                            self.pending_theme_apply = Some(card.name.clone());
                        }
                    }
                    return Some(Mode::Collapsed);
                }
                if is_clip {
                    let is_img = self.mode == Mode::ClipboardImages;
                    let n = if is_img { self.clip_images.len() } else { self.clip_text.len() };
                    if self.clip_sel < n {
                        self.pending_clip = Some((self.clip_sel, is_img));
                    }
                    return Some(Mode::Collapsed);
                }
                // Enter — launch the selected app
                self.ensure_launcher_hits();
                if let Some(&idx) = self.launcher_hits.get(self.sel) {
                    if let Some(app) = self.apps.apps.get(idx) {
                        let id = app.id.clone();
                        if !self.take_app_shortcut_pick(&id) {
                            self.apps.launch(&id);
                        }
                    }
                }
                return Some(Mode::Collapsed);
            }
            0xff52 => {
                // Up — move the selection (launcher list / clipboard list)
                if is_clip {
                    if self.clip_sel > 0 {
                        self.clip_sel -= 1;
                    }
                } else {
                    self.ensure_launcher_hits();
                    if self.sel > 0 {
                        self.sel -= 1;
                    }
                    if self.sel < self.launcher_scroll {
                        self.launcher_scroll = self.sel;
                    }
                }
            }
            0xff54 => {
                // Down — move the selection
                if is_clip {
                    let n = if self.mode == Mode::ClipboardImages {
                        self.clip_images.len()
                    } else {
                        self.clip_text.len()
                    };
                    if n > 0 && self.clip_sel + 1 < n {
                        self.clip_sel += 1;
                    }
                } else {
                    self.ensure_launcher_hits();
                    let n = self.launcher_hits.len();
                    if n > 0 && self.sel + 1 < n {
                        self.sel += 1;
                    }
                    let max = self.launcher_max_scroll();
                    if self.sel > self.launcher_scroll + self.launcher_visible() - 1 {
                        self.launcher_scroll = (self.sel + 1 - self.launcher_visible()).min(max);
                    }
                }
            }
            0xff08 => {
                if is_clip {
                    return Some(Mode::Collapsed); // Backspace → close
                }
                if is_themes {
                    self.themes_query.pop();
                    self.themes_scroll = self.themes_scroll.min(self.themes_max_scroll());
                } else {
                    self.query.pop();
                    self.sel = 0;
                    self.launcher_scroll = 0;
                    self.invalidate_launcher_hits();
                }
            }
            32..=126 => {
                if is_clip {
                    return None; // no text entry in the clipboard lists
                }
                if is_themes {
                    if self.themes_query.len() < 128 {
                        self.themes_query.push(char::from_u32(keysym).unwrap_or(' '));
                    }
                    self.themes_scroll = self.themes_scroll.min(self.themes_max_scroll());
                } else {
                    if self.query.len() < 128 {
                        self.query.push(char::from_u32(keysym).unwrap_or(' '));
                    }
                    self.sel = 0;
                    self.launcher_scroll = 0;
                    self.invalidate_launcher_hits();
                }
            }
            _ => {}
        }
        None
    }

    pub(crate) fn media_click(&self, px: f32, py: f32) -> Option<&'static str> {
        let (x, y, w, h) = self.media_card_rect;
        if !Self::in_rect(px, py, (x, y, w, h)) {
            return None;
        }
        let tl_y = y + self.scale.s(80.0) + self.scale.s(8.0);
        let btn_y = tl_y + self.scale.s(16.0);
        let btn_cx = x + w / 2.0;
        if Self::in_rect(px, py, (btn_cx - self.scale.s(50.0), btn_y - self.scale.s(10.0), self.scale.s(38.0), self.scale.s(36.0))) {
            return Some("Previous");
        }
        if Self::in_rect(px, py, (btn_cx - self.scale.s(22.0), btn_y - self.scale.s(12.0), self.scale.s(44.0), self.scale.s(40.0))) {
            return Some("PlayPause");
        }
        if Self::in_rect(px, py, (btn_cx + self.scale.s(12.0), btn_y - self.scale.s(10.0), self.scale.s(38.0), self.scale.s(36.0))) {
            return Some("Next");
        }
        None
    }

    pub(crate) fn tray_click(&self, px: f32, py: f32) -> Option<String> {
        let order = &self.pill_order;
        let mut cx = self.scale.s(self.pill_margin_x) + self.scale.s(6.0);
        let ch = self.scale.s(self.cfg.bar_h() - 4.0);
        let cy = (self.cfg.bar_h() - ch) / 2.0;
        for &item in order {
            match item {
                PillItem::Tray => {
                    let pw = self.scale.s(22.0);
                    if px >= cx && px <= cx + pw && py >= cy && py <= cy + ch {
                        return None;
                    }
                    cx += pw + self.scale.s(6.0);
                }
                _ => {
                    let w = self.pill_item_width(item);
                    cx += w + self.scale.s(PILL_GAP);
                }
            }
        }
        for (i, t) in self.tray.iter().enumerate() {
            let tw = self.scale.s(22.0);
            let tx = self.scale.s(self.pill_margin_x) + self.scale.s(6.0)
                + i as f32 * (tw + self.scale.s(2.0));
            if px >= tx && px <= tx + tw && py >= cy && py <= cy + ch {
                return Some(t.id.clone());
            }
        }
        None
    }

    pub(crate) fn edit_press(&mut self, x: f32, y: f32) {
        let cw = self.dash_cell_base_w(self.cfg.expanded_w());
        let ch = self.dash_cell_h(self.cfg.expanded_w());
        let pad = self.dash_pad();
        let gap = self.dash_gap();
        let gt = self.grid_top();

        if self.dash_enabled {
            for l in self.packed_layout.iter().rev() {
                // hit rects in SURFACE space (the draw loop scrolls content by
                // dash_scroll_x/y): the ✕ and grip via scrolled(), the body
                // inline. Mismatched spaces made the visible ✕ (and any
                // x-scrolled body) click through to the drag handler.
                let lx = pad + l.x as f32 * (cw + gap) - self.dash_scroll_x;
                let ly = gt + l.y as f32 * (ch + gap) - self.dash_scroll_y;
                let lw = l.w as f32 * cw + (l.w as f32 - 1.0) * gap;
                let lh = l.h as f32 * ch + (l.h as f32 - 1.0) * gap;
                let (gx, gy, gw, gh) = self.scrolled(Self::card_close_px(*l, cw, ch, gt, pad, gap, self.scale));
                if Self::in_rect(x, y, (gx, gy, gw, gh)) {
                    self.dash_layout.retain(|c| c.card != l.card);
                    self.tray_cards.push(l.card);
                    self.refresh_pack();
                    return;
                }
                let grip = self.grip_px(*l);
                if Self::in_rect(x, y, grip) {
                    self.edit_resize = Some((l.card, x - lx, y - ly, l.w, l.h));
                    return;
                }
                if Self::in_rect(x, y, (lx, ly, lw, lh)) {
                    self.edit_drag = Some((l.card, x - lx, y - ly));
                    self.edit_drag_from_tray = false;
                    self.edit_ghost_slot = None;
                    return;
                }
            }
        }

        if self.banner_tray_h() > 0.0 {
            let tray_top = self.banner_tray_top();
            if y >= tray_top && y <= tray_top + self.banner_tray_h()
                && x >= self.banner_tray_rect.0
                && x <= self.banner_tray_rect.0 + self.banner_tray_rect.2
            {
                let parked = self.parked_banner_tokens();
                for (i, tok) in parked.iter().enumerate() {
                    if !self.banner_tray_row_vis(i) { continue; }
                    let (rx, ry, tw, rh) = self.banner_tray_chip_px(i);
                    if x >= rx && x <= rx + tw && y >= ry && y <= ry + rh {
                        let idx = self.banner_order.iter().position(|t| t == tok).unwrap_or(0);
                        let drag = BannerDrag {
                            item: tok.clone(),
                            from_idx: idx,
                            from_tray: true,
                            over_idx: None,
                            x,
                            y,
                            swapped: false,
                        };
                        self.banner_drag = Some(drag);
                        return;
                    }
                }
            }
        }

        if self.dash_enabled && !self.dash_area_collapsed() {
            let parked = self.tray_cards.clone();
            for (i, card) in parked.iter().enumerate() {
                let (rx, ry, tw, th) = self.tray_chip_px(i);
                if x >= rx && x <= rx + tw && y >= ry && y <= ry + th {
                    // clicking a parked card ADDS it to the board BELOW the
                    // current content, auto-growing a new row when needed
                    // (metro cascades it rightward instead). The same press
                    // still arms the drag so a pull repositions it live.
                    let (sw, sh) = Self::default_span(*card);
                    let (gx, gy) = if self.dash_scroll_dir {
                        (self.live_extent().0, 0)
                    } else if self.dash_layout.is_empty() {
                        (0, 0)
                    } else {
                        (0, self.live_extent().1)
                    };
                    self.dash_layout.push(CardLayout { card: *card, x: gx, y: gy, w: sw, h: sh });
                    self.tray_cards.retain(|c| *c != *card);
                    self.dash_tray_scroll_row = self.dash_tray_scroll_row.min(self.dash_tray_max_scroll());
                    self.refresh_pack();
                    self.refresh_size();
                    self.edit_drag = Some((*card, x - rx, y - ry));
                    self.edit_drag_from_tray = false;
                    self.edit_ghost_slot = None;
                    return;
                }
            }
        }

        // strip-editor ctrl row buttons (dashboard on/off + banner-width
        // auto/manual) — registered hit rects resolve the exact buttons.
        // The row only exists while the dashboard is toggled in (banner
        // width ctrl is hidden when the dashboard is ON).
        match self.hover_key_at(x, y) {
            // park-minus badge on a strip chip ("−", top-right corner): the
            // chip leaves the strip and returns to the parked tray.
            k @ BANNER_PARK_KEY_BASE..=BANNER_PARK_KEY_MAX => {
                let i = (k - BANNER_PARK_KEY_BASE) as usize;
                if i < self.banner_order.len() {
                    self.banner_order.remove(i);
                    self.banner_drag = None;
                    self.save_config();
                    self.refresh_pack();
                    self.refresh_size();
                }
                return;
            }
            // move grip on a strip chip (edit mode): lifts the chip into a
            // banner drag so it can be re-dropped anywhere on the strip
            // (left / center / right zones) — it never parks. Dropping it
            // back in the same slot is a no-op; the park-minus badge is the
            // only way a chip leaves the strip. The grip's ledge region is
            // registered after the slot region, so it always wins the press.
            k @ BANNER_GRIP_KEY_BASE..=BANNER_GRIP_KEY_MAX => {
                let i = (k - BANNER_GRIP_KEY_BASE) as usize;
                if i < self.banner_order.len() {
                    let item = self.banner_order[i].clone();
                    let drag = BannerDrag {
                        item,
                        from_idx: i,
                        from_tray: false,
                        over_idx: None,
                        x,
                        y,
                        swapped: false,
                    };
                    self.banner_drag = Some(drag);
                }
                return;
            }
            // Clear-all buttons — the press starts the 3s liquid hold; the
            // completion is release-driven (clears board / empties strip).
            DASH_CLEAR_ALL_KEY => {
                self.dash_clear_all_hold = Some(std::time::Instant::now());
                return;
            }
            BANNER_CLEAR_ALL_KEY => {
                self.banner_clear_all_hold = Some(std::time::Instant::now());
                return;
            }
            BANNER_CTRL_KEY_BASE => {
                self.dash_enabled = !self.dash_enabled;
                self.save_config();
                self.refresh_pack();
                return;
            }
            BANNER_CHIPS_TOGGLE_KEY => {
                self.banner_chips_enabled = !self.banner_chips_enabled;
                self.banner_drag = None;
                self.refresh_size();
                return;
            }
            BANNER_W_AUTO_KEY => {
                if self.banner_w == 0 {
                    // auto -> manual: restore the last free width
                    self.banner_w = self.banner_w_last.max(60);
                } else {
                    // manual -> auto: remember the width for later
                    self.banner_w_last = self.banner_w;
                    self.banner_w = 0;
                }
                self.save_config();
                self.refresh_size();
                return;
            }
            DASH_SCROLL_DIR_KEY => {
                self.dash_scroll_dir = !self.dash_scroll_dir;
                self.save_config();
                self.refresh_pack();
                self.refresh_size();
                return;
            }
            // grid-size toolbar: rows −(504) +(505) · cols −(506) +(507)
            504 => {
                self.edit_grid_rows = self.edit_grid_rows.saturating_sub(1);
                self.refresh_pack();
                self.refresh_size();
                return;
            }
            505 => {
                self.edit_grid_rows = (self.edit_grid_rows as u32 + 1).min(self.rows_cap() as u32) as u16;
                self.refresh_pack();
                self.refresh_size();
                return;
            }
            506 => {
                self.edit_grid_cols = self.edit_grid_cols.saturating_sub(1);
                self.refresh_pack();
                self.refresh_size();
                return;
            }
            507 => {
                let max_cols = self.edit_cols_cap();
                self.edit_grid_cols = (self.edit_grid_cols as u32 + 1).min(max_cols as u32) as u16;
                self.refresh_pack();
                self.refresh_size();
                return;
            }
            // UI-controls −/+ steppers (pad / win / card): each press nudges
            // the value by 1 px and persists. Card radius is LINKED to the
            // window radius — the card stepper's −/+ drive the same shared
            // value.
            UI_PAD_MINUS_KEY => {
                self.ui_pad = (self.ui_pad - 1.0).max(0.0);
                self.save_config();
                self.refresh_size();
                return;
            }
            UI_PAD_PLUS_KEY => {
                self.ui_pad = (self.ui_pad + 1.0).min(40.0);
                self.save_config();
                self.refresh_size();
                return;
            }
            WIN_RAD_MINUS_KEY | CARD_RAD_MINUS_KEY => {
                self.win_radius = (self.win_radius - 1.0).max(0.0);
                self.card_radius = self.win_radius;
                self.save_config();
                self.refresh_size();
                return;
            }
            WIN_RAD_PLUS_KEY | CARD_RAD_PLUS_KEY => {
                self.win_radius = (self.win_radius + 1.0).min(20.0);
                self.card_radius = self.win_radius;
                self.save_config();
                self.refresh_size();
                return;
            }
            // one reset: padding 12 px · window rounding 16 px · card = window
            UI_CTRL_RESET_KEY => {
                self.ui_pad = 12.0;
                self.win_radius = 16.0;
                self.card_radius = self.win_radius;
                self.save_config();
                self.refresh_size();
                return;
            }
            // card-header visibility toggles: hide/show every card's header
            // TITLE text / header GLYPH (both persist; default ON)
            CARD_TITLE_TOGGLE_KEY => {
                self.card_show_title = !self.card_show_title;
                self.save_config();
                return;
            }
            CARD_GLYPH_TOGGLE_KEY => {
                self.card_show_glyph = !self.card_show_glyph;
                self.save_config();
                return;
            }
            _ => {}
        }
    }

    pub(crate) fn edit_motion(&mut self, x: f32, y: f32) -> bool {
        let cw = self.dash_cell_base_w(self.cfg.expanded_w());
        let ch = self.dash_cell_h(self.cfg.expanded_w());
        let pad = self.dash_pad();
        let gap = self.dash_gap();
        let gt = self.grid_top();

        if let Some((card, off_x, off_y)) = self.edit_drag {
            let cx = (x - off_x - pad + self.dash_scroll_x) / (cw + gap);
            let cy = (y - off_y - gt + self.dash_scroll_y) / (ch + gap);
            let gx = (cx.round() as i32).max(0) as u16;
            let gy = (cy.round() as i32).max(0) as u16;
            let slot = (gx, gy);
            if self.edit_ghost_slot != Some(slot) {
                self.edit_ghost_slot = Some(slot);
                let w = self.dash_layout.iter().find(|l| l.card == card).map(|l| l.w).unwrap_or(1);
                let h = self.dash_layout.iter().find(|l| l.card == card).map(|l| l.h).unwrap_or(1);
                if self.edit_drag_from_tray {
                    self.dash_layout.push(CardLayout { card, x: gx, y: gy, w, h });
                    self.tray_cards.retain(|c| *c != card);
                    self.edit_drag_from_tray = false;
                } else {
                    if let Some(l) = self.dash_layout.iter_mut().find(|l| l.card == card) {
                        l.x = gx;
                        l.y = gy;
                    }
                }
                self.refresh_pack();
                return true;
            }
        }

        if let Some((card, off_x, off_y, _ow, _oh)) = self.edit_resize {
            // off_x/off_y were captured in surface space (see edit_press); add
            // the scroll back so the pointer maps to content cells (the drag
            // branch above does the same)
            let lx = x - off_x + self.dash_scroll_x;
            let ly = y - off_y + self.dash_scroll_y;
            let new_w = (((lx - pad) / (cw + gap) + 1.0).round() as u16).max(1).min(self.grid_max_cols());
            let new_h = (((ly - gt) / (ch + gap) + 1.0).round() as u16).max(1);
            if Some((new_w, new_h)) != self.edit_resize_cached {
                self.edit_resize_cached = Some((new_w, new_h));
                if let Some(l) = self.dash_layout.iter_mut().find(|l| l.card == card) {
                    l.w = new_w;
                    l.h = new_h;
                }
                self.refresh_pack();
                return true;
            }
        }

        if self.banner_drag.is_some() {
            let mut drag = self.banner_drag.take().unwrap();
            drag.x = x;
            drag.y = y;
            let n = self.banner_order.len();
            // The strip is split into three L/C/R thirds (see banner_cells);
            // a drop must land in the zone under the cursor, even when that
            // zone is currently empty. Zone markers in `banner_order` define
            // each section's insertion range [lo, hi].
            let row_w = self.strip_row_w.max(1.0);
            let margin = self.scale.s(10.0);
            let avail = (row_w - margin * 2.0).max(0.0);
            let zw = avail / 3.0;
            let zone = if x >= margin + 2.0 * zw {
                2
            } else if x >= margin + zw {
                1
            } else {
                0
            };
            let mut z_markers = Vec::new();
            for (i, t) in self.banner_order.iter().enumerate() {
                if t.as_zone().is_some() {
                    z_markers.push(i);
                }
            }
            let z1 = z_markers.first().copied();
            let z2 = z_markers.get(1).copied();
            let (lo, hi) = match zone {
                0 => (0, z1.unwrap_or(n)),
                1 => (z1.map_or(0, |m| m + 1), z2.unwrap_or(n).max(z1.map_or(0, |m| m + 1))),
                _ => (z2.unwrap_or_else(|| z1.map_or(0, |m| m + 1)), n),
            };
            // Actual cell positions (respecting zone alignment) pick the
            // nearest slot; the zone range above pins it to that zone.
            let cells = self.banner_cells(row_w);
            let mut new_idx = n;
            for (i, cell) in cells.iter().enumerate() {
                if let Some((cx, cw)) = cell {
                    if x < cx + cw / 2.0 {
                        new_idx = i;
                        break;
                    }
                }
            }
            let new_idx = new_idx.clamp(lo, hi);
            let from = drag.from_idx;
            if !drag.from_tray {
                let cur_idx = self.banner_order.iter().position(|t| *t == drag.item).unwrap_or(from);
                if new_idx != cur_idx && new_idx != cur_idx + 1 {
                    self.banner_order.remove(cur_idx);
                    let insert = if new_idx > cur_idx { new_idx - 1 } else { new_idx };
                    self.banner_order.insert(insert, drag.item.clone());
                    drag.from_idx = insert;
                    drag.swapped = true;
                    self.banner_drag = Some(drag);
                    return true;
                }
            }
            drag.over_idx = Some(new_idx);
            self.banner_drag = Some(drag);
        }

        false
    }

    pub(crate) fn edit_release(&mut self, _x: f32, _y: f32) -> bool {
        let mut changed = false;

        if self.edit_drag.take().is_some() || self.edit_resize.take().is_some() {
            self.edit_ghost_slot = None;
            self.edit_resize_cached = None;
            self.refresh_pack();
            changed = true;
        }

        if let Some(drag) = self.banner_drag.take() {
            if drag.from_tray {
                // a plain click (no motion) never computed a drop column —
                // append at the strip's tail so "click to add" works too
                let idx = drag.over_idx.unwrap_or_else(|| self.banner_order.len());
                let tok = drag.item.clone();
                self.banner_order.insert(idx, tok);
                self.save_config();
                self.refresh_pack();
                self.refresh_size();
            } else if drag.swapped {
                // strip chip dragged to a new home (move handle) — persist
                // the new order
                self.save_config();
                self.refresh_pack();
                self.refresh_size();
            }
            changed = true;
        }

        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn balance_slider_reaches_extremes() {
        let mut shell = crate::shell::Shell::new(crate::config::Config::default());
        shell.balance_rect = (100.0, 200.0, 300.0);
        shell.vol_left = 0.72;
        shell.vol_right = 0.72;
        // full left: pointer at fader start → L carries the whole 2*mid sum
        assert!(shell.set_dash_slider(59, 100.0, 200.0));
        assert!((shell.vol_left - 1.44).abs() < 0.01, "vol_left {}", shell.vol_left);
        assert!(shell.vol_right.abs() < 0.01, "vol_right {}", shell.vol_right);
        // full right: pointer at fader end → R carries the whole sum
        assert!(shell.set_dash_slider(59, 400.0, 200.0));
        assert!(shell.vol_left.abs() < 0.01, "vol_left {}", shell.vol_left);
        assert!((shell.vol_right - 1.44).abs() < 0.01, "vol_right {}", shell.vol_right);
        // balanced center: frac 0.5 → both stay at mid
        assert!(shell.set_dash_slider(59, 250.0, 200.0));
        assert!((shell.vol_left - 0.72).abs() < 0.01, "vol_left {}", shell.vol_left);
        assert!((shell.vol_right - 0.72).abs() < 0.01, "vol_right {}", shell.vol_right);
    }
}
