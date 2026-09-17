use super::*;

impl Shell {
    pub(crate) fn new(cfg: Config) -> Self {
        let (w, h) = Mode::Collapsed.size(&cfg);
        // pill behavior seeds from config; the Settings → Pill toggles flip
        // the same fields at runtime (no persistence, config is the default)
        let expand_on_hover = cfg.expand_on_hover();
        let cc_as_dashboard = cfg.cc_as_dashboard();
        let pill_clock = cfg.show_clock();
        let pill_battery = cfg.show_battery();
        let pill_battery_pct = cfg.show_battery_pct();
        let pill_ws = cfg.show_ws();
        let pill_ws_long = cfg.show_ws_long();
        let pill_ws_short = cfg.show_ws_short();
        let pill_wifi = cfg.show_wifi();
        let pill_bluetooth = cfg.show_bluetooth();
        let pill_volume = cfg.show_volume();
        let pill_wallpaper = cfg.show_wallpaper();
        let pill_themes = cfg.show_themes();
        let pill_branding = cfg.show_brand();
        let pill_tray = cfg.show_tray();
        let pill_viz = cfg.show_viz();
    let pill_settings = cfg.settings_on_pill();
    let pill_notif = cfg.show_notif();
    let pill_user = cfg.show_user();
    let pill_watts = cfg.show_watts();
    let pill_brightness = cfg.show_brightness();
        let bar_edge = cfg.bar_edge();
        let bar_floating = cfg.floating();
        let bar_reserve = cfg.reserve();
        let icon_style = cfg.fonts.icon_style.clone();
        let pill_order = cfg.pill_order();
        let banner_order = cfg.banner_order();
        let dash_enabled = cfg.dash_enabled();
        let dash_scroll_dir = cfg.dash_scroll_dir();
        let app_shortcuts = cfg.app_shortcuts();
        let banner_w = cfg.banner_w_px();
        let wp_cell = cfg.wallpaper_cell();
        let world_save_data = cfg.world.save_data;
    // boot-time palette fallbacks (the live values come from
        // $states2/shell_vars right after construction)
        let (fg0, bg0, acc0, hov0) = (cfg.fg(), cfg.bg(), cfg.acc(), cfg.hover());
        let grid_scale = cfg.grid_scale();
        // ── master config is per-channel: `$states/shell_{n,d,l}` ──
        // `cfg` already IS the channel's master config (loaded in main.rs);
        // read the current channel and pull the pill geometry straight from it.
        let ui_state = crate::vars::read_channel();
        let pill_margin_x = cfg.pill_margin_x();
        let pill_alpha = cfg.pill_alpha();
        let dash_card_alpha = cfg.dash_card_alpha();
        let power_hold_delay = cfg.power_hold_delay();
        let ui_pad = cfg.ui_pad();
        let win_radius = cfg.win_radius();
        // card rounding is permanently LINKED to the window rounding — one
        // shared radius, the config's separate card keys are ignored
        let card_radius = win_radius;
        let card_radius_sync = true;
        let viz_style = cfg.viz_style.unwrap_or(0).min(2);
        let mirror_fps = cfg.mirror_fps.unwrap_or(60).clamp(1, 60);
        let mirror_show_fps = cfg.mirror_show_fps.unwrap_or(true);
        let show_balance = cfg.show_balance.unwrap_or(true);
        let show_saturation = cfg.show_saturation.unwrap_or(true);
        let audiorec_confirm_delete = cfg.audiorec_confirm_delete.unwrap_or(true);
        let audiorec_show_mic = cfg.audiorec_show_mic.unwrap_or(true);
        let card_show_title = cfg.card_show_title.unwrap_or(true);
        let card_show_glyph = cfg.card_show_glyph.unwrap_or(true);
        let mut shell = Shell {
            cfg,
            scale: GridScale(grid_scale),
            hover: false,
            mode: Mode::Collapsed,
            prev_mode: Mode::Collapsed,
            clock: String::new(),
            ws_count: 1,
            ws_active: 0,
            battery: -1,
            battery_watts: 0.0,
            watts_shown: 0.0,
            watts_at: None,
            battery_time: String::new(),
            ssid: String::new(),
            wifi_networks: Vec::new(),
            bt_devices: Vec::new(),
            notif_count: 0,
            tray: Vec::new(),
            title: String::new(),
            apps: AppManager::new(),
            media_title: String::new(),
            media_artist: String::new(),
            media_album: String::new(),
            media_playing: false,
            media_pos: 0,
            media_len: 0,
            media_pos_at: Instant::now(),
            media_art: String::new(),
            media_seek: None,
            viz: vec![0.0; 24],
            viz_style,
            viz_peak: 0.0,
            eq: std::sync::Arc::new(std::sync::Mutex::new(crate::eq::Eq::default())),
            eq_rect: (0.0, 0.0, 0.0, 0.0),
            eq_drag: None,
            news_items: Vec::new(),
            news_rect: (0.0, 0.0, 0.0, 0.0),
            news_scroll: 0,
            news_active_cat: 0,
            news_cat_scroll: 0.0,
            news_cat_content_w: 0.0,
            pending_news_open: None,
            news_fetched: false,
            lyrics_lines: Vec::new(),
            lyrics_track: String::new(),
            lyrics_state: 0,
            lyrics_scroll: 0,
            lyrics_scroll_smooth: 0.0,
            lyrics_rect: (0.0, 0.0, 0.0, 0.0),
            wifi_rect: (0.0, 0.0, 0.0, 0.0),
            wifi_scroll: 0,
            bt_rect: (0.0, 0.0, 0.0, 0.0),
            bt_scroll: 0,
            speed_rect: (0.0, 0.0, 0.0, 0.0),
            speed_state: 0,
            speed_down: String::new(),
            speed_up: String::new(),
            speed_ping: 0,
            recent_rect: (0.0, 0.0, 0.0, 0.0),
            recent_scroll: 0,
            recent_files: Vec::new(),
            recent_loaded_at: std::time::Instant::now(),
            currency_rect: (0.0, 0.0, 0.0, 0.0),
            currency_scroll: 0,
            currency_base: String::from("USD"),
            currency_note: String::new(),
            quote_rect: (0.0, 0.0, 0.0, 0.0),
            quote_text: String::new(),
            quote_author: String::new(),
            quote_state: 0,
            quote_saved_flash: 0,
            saved_quotes: Vec::new(),
            ticker_rect: (0.0, 0.0, 0.0, 0.0),
            ticker_scroll: 0,
            ticker_items: Vec::new(),
            ticker_state: 0,
            ticker_last_at: std::time::Instant::now(),
            sensors: crate::sensors::Sensors::default(),
            cam_ok: false,
            cam_rate: 0,
            mirror_fps,
            mirror_show_fps,
            mirror_pane: 0,
            mirror_rect: (0.0, 0.0, 0.0, 0.0),
            mirror_rec: false,
            mirror_rec_secs: 0,
            show_balance,
            show_saturation,
            card_show_title,
            card_show_glyph,
            sliders_pane: 0,
            sliders_rect: (0.0, 0.0, 0.0, 0.0),
            audiorec_pane: 0,
            audiorec_rect: (0.0, 0.0, 0.0, 0.0),
            audiorec_mic_rect: (0.0, 0.0, 0.0),
            audiorec_recording: false,
            audiorec_rec_secs: 0,
            audiorec_recordings: Vec::new(),
            audiorec_confirm_delete,
            audiorec_show_mic,
            pending_audiorec_rec: false,
            pending_audiorec_list: false,
            pending_audiorec_play: None,
            pending_audiorec_del: None,
            audiorec_del_armed: None,
            query: String::new(),
            themes_query: String::new(),
            sel: 0,
            launcher_hits: Vec::new(),
            launcher_hits_query: String::from("\0"), // force refresh on first use
            launcher_scroll: 0,
            drag_key: None,
            notifs: Vec::new(),
            dnd: false,
            cal_offset: 0,
            cal_selected: -1,
            wifi_on: true,
            bt_on: false,
            volume: 0.7,
            brightness: 1.0,
            volume_muted: false,
            pending_volume: None,
            vol_left: 1.0,
            vol_right: 1.0,
            pending_vol_left: None,
            pending_vol_right: None,
            allow_over_100: true,
            max_vol: 100,
            pending_max_vol: None,
            pending_brightness: None,
            mic_level: 1.0,
            mic_muted: false,
            caffeine_on: false,
            sunset_on: false,
            sunset_kelvin: 4000,
            shader_on: false,
            saturation: 100,
            pending_dash: None,
            ac_online: true,
            caps: false,
            monitor: (1920.0, 1080.0),
            osd: None,
            power_hold: None,
            power_hold_start: Instant::now(),
            audio_sinks: Vec::new(),
            audio_sources: Vec::new(),
            audio_streams: Vec::new(),
            pending_audio_volume: None,
            pending_audio_mute: None,
            pending_audio_default: None,
            audio_default: 0,
            er_running: false,
            er_start: None,
            er_paused_rem: None,
            er_rest_until: None,
            er_sessions: 0,
            lat_history: std::collections::VecDeque::new(),
            lat_current: 0,
            lat_state: 0,
            lat_next: None,
            pw_history: std::collections::VecDeque::new(),
            pw_next: None,
            sm_disks: Vec::new(),
            sm_next: None,
            sus_failed: Vec::new(),
            sus_next: None,
            jr_lines: Vec::new(),
            jr_next: None,
            conn_rows: Vec::new(),
            wallpapers: Vec::new(),
            wallpaper_scroll: 0,
            wp_card_scroll: 0,
            wp_card_cell_eff: 0.0,
            themes: Vec::new(),
            themes_scroll: 0,
            keybinds: Vec::new(),
            keybinds_scroll: 0,
            cur_theme: String::new(),
            cur_scheme: String::new(),
            cur_channel: String::new(),
            cur_acc_src: String::new(),
            pending_theme_apply: None,
            custom_accs: Vec::new(),
            pending_accent_src: None,
            pending_pick_accent: None,
            pending_theme_cmd: None,
            appearance_scroll: 0.0,
            pill_scroll: 0.0,
            custom_acc_drop_open: false,
            accent_list_open: false,
            accent_list_scroll: 0,
            accent_list_rect: (0.0, 0.0, 0.0, 0.0),
            custom_acc_drop_scroll: 0,
            acc_flag_start_icon: false,
            acc_flag_app_border: false,
            acc_flag_hyprland: false,
            acc_flag_gtk: true,
            acc_flag_normal_app: true,
            acc_scrim: false,
            start_icon_tone: 0,
            bg_alpha: 0xff,
            acc_alpha: 0xe5,
            scrim_alpha: 0x4c,
            border_alpha: 0xbf,
            dark_mode: true,
            sv_fg: fg0,
            sv_fg2: FG2,
            sv_fg3: FG3,
            sv_bg: bg0,
            sv_acc: acc0,
            sv_sfg: SFG,
            sv_hover: hov0,
            sv_focus: 0x6a6d75ff,
            sv_disabled: 0x6a6d75ff,
            sv_series: [
                0xff42a5f5, 0xffef5350, 0xff66bb6a,
                0xffab47bc, 0xffffa726, 0xff26c6da,
            ],
            wp_cell,
            wp_sb_drag: false,
            wp_sb_grab: 0.0,
            th_sb_drag: false,
            th_sb_grab: 0.0,
            pending_wallpaper: None,
            pending_wifi: None,
            pending_bt: None,
            pending_wifi_power: None,
            pending_bt_power: None,
            pending_speed_test: false,
            pending_quote_fetch: false,
            pending_recent_open: None,
            pending_ticker_open: None,
            pending_unlock: false,
            lock_pw: String::new(),
            lock_error: None,
            pending_lock_auth: false,
            lock_checking: false,
            lock_session: false,
            polkit_action_id: String::new(),
            polkit_message: String::new(),
            polkit_icon: String::new(),
            polkit_cookie: String::new(),
            polkit_uid: 0,
            polkit_pw: String::new(),
            polkit_error: None,
            polkit_checking: false,
            polkit_pending: false,
            clip_text: Vec::new(),
            clip_images: Vec::new(),
            clip_sel: 0,
            clip_scroll: 0,
            clipimg_scroll: 0,
            todo_scroll: 0,
            notes_scroll: 0,
            clip_card_rect: (0.0, 0.0, 0.0, 0.0),
            clipimg_card_rect: (0.0, 0.0, 0.0, 0.0),
            todo_card_rect: (0.0, 0.0, 0.0, 0.0),
            notes_card_rect: (0.0, 0.0, 0.0, 0.0),
            pending_clip: None,
            pending_ws_dispatch: None,
            pending_mirror_photo: false,
            pending_mirror_rec: false,
            pending_mirror_fps: false,
            ws_prev: 0,
            cpu: 0,
            mem_pct: 0,
            disk_pct: 0,
            mem_total_gb: 0.0,
            mem_used_gb: 0.0,
            mem_avail_gb: 0.0,
            mem_cached_gb: 0.0,
            mem_free_gb: 0.0,
            disk_used_gb: 0.0,
            disk_total_gb: 0.0,
            cpu_load: (0.0, 0.0, 0.0),
            cpu_freq_mhz: 0,
            cpu_temp: 0.0,
            gpu: -1,
            gpu_temp: 0.0,
            gpu_vram_used_mb: 0,
            gpu_vram_total_mb: 0,
            thermal_zones: Vec::new(),
            pomo_phase_focus: true,
            pomo_running: false,
            pomo_paused_rem: None,
            pomo_deadline: None,
            pomo_focus_min: 25,
            pomo_break_min: 5,
            pomo_sessions: 0,
            fans_rpm: Vec::new(),
            fan_peaks: Vec::new(),
            worldclock_zones: Vec::new(),
            worldclock_search: None,
            worldclock_probed_at: None,
            worldclock_pick_map: Vec::new(),
            worldclock_rect: (0.0, 0.0, 0.0, 0.0),
            water_glasses: 0,
            water_goal: 8,
            water_day: String::new(),
            water_rect: (0.0, 0.0, 0.0, 0.0),
            fx_blur_on: false,
            fx_shadow_on: false,
            fx_opacity_on: false,
            countdown_events: Vec::new(),
            countdown_input: None,
            countdown_rect: (0.0, 0.0, 0.0, 0.0),
            pending_shot: None,
            pending_snippet_copy: None,
            alarm_list: Vec::new(),
            alarm_input: None,
            alarms_fired: Vec::new(),
            alarms_rect: (0.0, 0.0, 0.0, 0.0),
            snippets: Vec::new(),
            snippet_input: None,
            snippet_copied: None,
            snippet_copied_at: None,
            snippets_rect: (0.0, 0.0, 0.0, 0.0),
            expenses: Vec::new(),
            expense_input: None,
            expenses_rect: (0.0, 0.0, 0.0, 0.0),
            cpu_history: std::collections::VecDeque::with_capacity(48),
            gpu_history: std::collections::VecDeque::with_capacity(48),
            net_up: String::new(),
            net_down: String::new(),
            hostname: String::new(),
            username: whoami(),
            uptime: String::new(),
            net_history: std::collections::VecDeque::with_capacity(40),
            disk_history: std::collections::VecDeque::with_capacity(48),
            todos: Vec::new(),
            todo_input: None,
            disk_parts: Vec::new(),
            top_procs: Vec::new(),
            swap_pct: 0,
            active_win_title: String::new(),
            active_win_class: String::new(),
            active_win_pid: 0,
            active_win_rss_kb: 0,
            battery_v_rect: (0.0, 0.0, 0.0, 0.0),
            battery_v_pane: 0,
            battery_h_rect: (0.0, 0.0, 0.0, 0.0),
            battery_h_pane: 0,
            weather_rect: (0.0, 0.0, 0.0, 0.0),
            weather_pane: 0,
            worldmap_rect: (0.0, 0.0, 0.0, 0.0),
            worldmap_body_rect: (0.0, 0.0, 0.0, 0.0),
            world_pane: 0,
            world_center: (0.0, 0.0),
            world_zoom: 1.0,
            world_menu: false,
            world_save_data: world_save_data,
world_dl_pending: None,
            world_dl_err: None,
            world_region: None,
            world_region_rings: None,
            world_rev: 0,
            world_region_target: None,
            world_manifest: None,
            world_marker: None,
            world_drag: None,
            world_drag_moved: false,
            world_city: String::new(),
            world_tz: String::new(),
            world_abbrev: String::new(),
            world_offset_min: 0,
            pending_world_offset: None,
            world_off_at: None,
            power_save_on: false,
            pending_power_save: false,
            notes: Vec::new(),
            notes_loaded: false,
            notes_input: None,
            notes_title: String::new(),
            notes_body: String::new(),
            pkg_update_count: 0,
            ssh_vpn_lines: Vec::new(),
            docker_containers: Vec::new(),
            kb_layout: String::new(),
            dash_layout: Vec::new(),
            dash_edit: false,
            edit_drag: None,
            edit_resize: None,
            edit_drag_from_tray: false,
            edit_ghost_slot: None,
            edit_resize_cached: None,
            edit_grid_cols: 0,
            edit_grid_rows: 0,
            dash_scroll_x: 0.0,
            dash_scroll_y: 0.0,
            dash_sb_drag: None,
            dash_sb_last: None,
            tray_cards: Vec::new(),
            dash_clear_all_hold: None,
            banner_clear_all_hold: None,
            tray_rect: (0.0, 0.0, 0.0, 0.0),
            tray_content_h: 0.0,
            dash_band_via_scene: false,
            dash_chips_via_scene: false,
            banner_tray_scroll_row: 0,
            dash_tray_scroll_row: 0,
            packed_layout: Vec::new(),
            dash_cols: DASH_COLS,
            dash_rows: DASH_ROWS,
            layout_pinned: false,
            edit_preview_pack: None,
            card_anim: std::collections::HashMap::new(),
            edit_settling: false,
            drag_grace_until: std::time::Instant::now(),
            dash_w: 1120.0,
            dash_h: EXPANDED_H,
            media_seek_rect: (0.0, 0.0, 0.0, 0.0),
            media_card_rect: (0.0, 0.0, 0.0, 0.0),
            faders: [(0.0, 0.0, 0.0); 5],
            balance_rect: (0.0, 0.0, 0.0),
            wp_card_rect: (0.0, 0.0, 0.0, 0.0),
            weather: None,
            weather_city: String::new(),
            ws_preview: Vec::new(),
            ws_preview_ws: usize::MAX,
            cursor: None,
            hover_key: 0,
            hover_regions: Vec::new(),
            expand_on_hover,
            cc_as_dashboard,
            pill_clock,
            pill_battery,
            pill_battery_pct,
            pill_ws,
            pill_ws_long,
            pill_ws_short,
            pill_wifi,
            pill_bluetooth,
            pill_volume,
            pill_wallpaper,
            pill_themes,
            pill_branding,
            brand_glyph: crate::vars::read_branding_glyph(),
            pill_tray,
            pill_viz,
            pill_settings,
            pill_notif,
            pill_user,
            pill_watts,
            pill_brightness,
            pill_order,
            pill_drag: None,
            banner_order: banner_order,
            dash_enabled: dash_enabled,
            banner_chips_enabled: true,
            dash_scroll_dir: dash_scroll_dir,
            app_shortcuts,
            app_shortcut_pick: None,
            app_shortcut_rect: (0.0, 0.0, 0.0, 0.0),
            banner_drag: None,
            banner_w,
            banner_w_last: 0,
            connectivity_open: false,
            sys_menu_open: false,
            sys_menu_bp_open: false,
            strip_row_w: 0.0,
            banner_zone_scroll: [0.0; 3],
            banner_strip_scroll: 0.0,
            banner_tray_rect: (0.0, 0.0, 0.0, 0.0),
            banner_ctrl_rect: (0.0, 0.0, 0.0, 0.0),
            banner_w_slider_rect: (0.0, 0.0, 0.0, 0.0),
            ui_pad_slider_rect: (0.0, 0.0, 0.0, 0.0),
            win_rad_slider_rect: (0.0, 0.0, 0.0, 0.0),
            card_rad_slider_rect: (0.0, 0.0, 0.0, 0.0),
            bar_edge,
            pending_move: None,
            bar_floating,
            bar_reserve,
            pending_floating: None,
            pending_reserve: None,
            icon_style,
            pending_icon_style: None,
            pill_margin_x,
            pill_alpha,
            dash_card_alpha,
            power_hold_delay,
            ui_pad,
            win_radius,
            card_radius,
            card_radius_sync,
            ui_state,
            per_state_config: crate::vars::per_state_shell_path(ui_state),
            settings_tab: SettingsTab::default(),
            scene_cache: crate::scene::SceneCache::new(),
            shell_scene: crate::scene::ShellSceneCache::new(),
            scene_click_actions: std::collections::HashMap::new(),
            scene_owns_header: false,
            scene_owns_composer: false,
            scene_owns_rows: false,
            cur_w: w,
            cur_h: h,
            anim: None,
            size_pending: false,
            last_committed: (w, h),
            content_dx: 0.0,
            content_dy: 0.0,
        };
        // configs saved before a PillItem variant existed lack it — append
        // missing items so new chips are reachable from Settings
        for item in DEFAULT_PILL_ORDER {
            if !shell.pill_order.contains(item) {
                shell.pill_order.push(*item);
            }
        }
        // Dashboard layout, colors, and accent flags are loaded later in
        // seed_data() right after the first frame — no file I/O before the
        // first frame so the bar + launcher appear instantly.
        shell
    }

    /// Load the dashboard layout, todos, notification history, live colors,
    /// and accent flags that were deferred out of  so the bar +
    /// launcher appear instantly.  Called once right after the first frame
    /// is committed (alongside `seed_boot_data`).
    pub(crate) fn seed_data(&mut self) {
        // hostname / uptime (lightweight sys reads)
        self.hostname = hostname();
        self.uptime = uptime_str();
        // todo list
        self.todos = Shell::load_todos();
        // pomodoro durations + session count
        self.load_pomo();
        // pinned world-clock cities (defaults on first launch)
        self.load_worldclock_zones();
        // water tracker (goal + today's count, midnight-rolled)
        self.load_water();
        // countdown / alarms / snippets / expenses
        self.load_countdown();
        self.load_alarms();
        self.load_snippets();
        self.load_expenses();
        // eye rest session count
        self.load_eyerest();
        // connection info (cheap in-process read)
        self.poll_conninfo();
        // notification history
        self.load_notifs();
        // saved quotes from $states/quotes
        self.load_saved_quotes();
        self.recent_files = Shell::read_recent_files();
        // dashboard grid + tray
        let (layout, tray) = Shell::load_dash_layout();
        self.dash_layout = layout;
        self.tray_cards = tray;
        Shell::apply_persisted_dashboard(self);
        self.refresh_pack();
        // live colors + dark/light state from /tmp/tw_hypr_conf/states2
        self.reload_states_colors();
        self.reload_accent_flags();
        self.sync_dark_state();
    }

    // ------------------------------------------------------------ quote shelf

    /// Path of the quotes shelf (`$states/quotes` — one quote per line).
    pub(crate) fn quotes_path() -> std::path::PathBuf {
        crate::vars::states_dir().join("quotes")
    }

    /// Load previously saved quotes from `$states/quotes`.
    pub(crate) fn load_saved_quotes(&mut self) {
        self.saved_quotes = std::fs::read_to_string(Self::quotes_path())
            .map(|s| {
                s.lines()
                    .filter(|l| !l.trim().is_empty())
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default();
    }

    /// Persist `text — author` to `$states/quotes` (deduped), bumping the
    /// card's green "saved" flash.
    pub(crate) fn save_quote(&mut self, text: &str, author: &str) {
        if text.trim().is_empty() {
            return;
        }
        let full = if author.trim().is_empty() {
            text.trim().to_string()
        } else {
            format!("{} — {}", text.trim(), author.trim())
        };
        if self.saved_quotes.iter().any(|q| q == &full) {
            return;
        }
        self.saved_quotes.push(full.clone());
        let _ = std::fs::create_dir_all(crate::vars::states_dir());
        let _ = std::fs::write(Self::quotes_path(), self.saved_quotes.join("\n") + "\n");
        self.quote_saved_flash = 40;
    }

    // ------------------------------------------------------- recent files

    /// Open `path`/URL with the default handler (xdg-open, detached).
    pub(crate) fn open_path(path: &str) {
        use std::process::Stdio;
        let _ = std::process::Command::new("xdg-open")
            .arg(path)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
    }

    /// Recently opened files from `~/.local/share/recently-used.xbel`
    /// (name, absolute path), newest-first, existing files only. Empty when
    /// the desktop hasn't tracked any (DE that use XDG recent consistently).
    pub(crate) fn read_recent_files() -> Vec<(String, String)> {
        pub(crate) fn decode(s: &str) -> String {
            let b = s.as_bytes();
            let mut out = Vec::with_capacity(b.len());
            let mut i = 0;
            while i < b.len() {
                if b[i] == b'%' && i + 2 < b.len() {
                    let hi = (b[i + 1] as char).to_digit(16);
                    let lo = (b[i + 2] as char).to_digit(16);
                    if let (Some(hi), Some(lo)) = (hi, lo) {
                        out.push((hi * 16 + lo) as u8);
                        i += 3;
                        continue;
                    }
                }
                out.push(b[i]);
                i += 1;
            }
            String::from_utf8_lossy(&out).into_owned()
        }
        let home = std::env::var("HOME").unwrap_or_default();
        let path = std::path::Path::new(&home).join(".local/share/recently-used.xbel");
        let Ok(xml) = std::fs::read_to_string(path) else {
            return Vec::new();
        };
        let mut out: Vec<(String, String)> = Vec::new();
        let mut tail: Option<String> = None;
        for line in xml.lines() {
            if let Some(i) = line.find("href=\"file://") {
                let rest = &line[i + "href=\"file://".len()..];
                if let Some(end) = rest.find('"') {
                    let full = decode(&rest[..end]);
                    out.push((String::new(), full));
                    tail = out.last().map(|(_, p)| p.clone());
                    continue;
                }
            }
            if let Some(tp) = line.trim().strip_prefix("<title>") {
                let name = tp.trim_end_matches("</title>").trim().to_string();
                if let Some(last) = out.last_mut() {
                    last.0 = name;
                }
                tail = None;
            } else if tail.is_some() && line.contains("</bookmark>") {
                tail = None;
            }
        }
        out.reverse(); // xbel lists oldest-first
        out.retain(|(n, p)| {
            if n.trim().is_empty() {
                return false;
            }
            let path = std::path::Path::new(p);
            path.is_file()
        });
        out.truncate(24);
        out
    }

    // ------------------------------------------- background fetchers (pure)

    /// (download Mbps, upload Mbps, ping ms) from Cloudflare's speed-test
    /// endpoints via curl subprocesses. Runs on a worker thread — call from
    /// app.rs, never from the draw path.
    pub(crate) fn speedtest_probe() -> (f32, f32, u32) {
        let down = || -> f32 {
            pub(crate) const MB: usize = 16 * 1024 * 1024;
            let t = std::time::Instant::now();
            let ok = std::process::Command::new("curl")
                .args([
                    "-sL",
                    "--max-time",
                    "20",
                    "-o",
                    "/dev/null",
                    "https://speed.cloudflare.com/__down?bytes=16777216",
                ])
                .status()
                .map(|s| s.success())
                .unwrap_or(false);
            if !ok {
                return 0.0;
            }
            let secs = t.elapsed().as_secs_f32();
            if secs <= 0.0 {
                return 0.0;
            }
            MB as f32 * 8.0 / 1_000_000.0 / secs
        };
        let ping = || -> u32 {
            let o = std::process::Command::new("curl")
                .args([
                    "-sL",
                    "--max-time",
                    "8",
                    "-o",
                    "/dev/null",
                    "-w",
                    "%{time_starttransfer}",
                    "https://speed.cloudflare.com/__down?bytes=0",
                ])
                .output();
            if let Ok(o) = o {
                if let Ok(ms) = String::from_utf8_lossy(&o.stdout).trim().parse::<f32>() {
                    if ms > 0.0 {
                        return (ms * 1000.0) as u32;
                    }
                }
            }
            0
        };
        let up = || -> f32 {
            pub(crate) const MB: usize = 8 * 1024 * 1024;
            let dir = crate::vars::states_dir();
            let p = dir.join("speed_up.bin");
            let _ = std::fs::create_dir_all(&dir);
            let _ = std::fs::write(&p, vec![0u8; MB]);
            let o = std::process::Command::new("curl")
                .args(["-sL", "--max-time", "20", "-o", "/dev/null", "-w", "%{speed_upload}", "--upload-file"])
                .arg(&p)
                .arg("https://speed.cloudflare.com/__up")
                .output();
            let _ = std::fs::remove_file(&p);
            if let Ok(o) = o {
                if let Ok(bps) = String::from_utf8_lossy(&o.stdout).trim().parse::<f32>() {
                    if bps > 0.0 {
                        return bps * 8.0 / 1_000_000.0;
                    }
                }
            }
            0.0
        };
        (down(), up(), ping())
    }

    /// A random quote `(text, author)` — quotable.io first, zenquotes as the
    /// fallback. Worker-thread only (curl, 8 s cap).
    pub(crate) fn fetch_quote() -> Option<(String, String)> {
        let quotable = std::process::Command::new("curl")
            .args([
                "-sL",
                "--max-time",
                "8",
                "https://api.quotable.io/random?maxLength=180",
            ])
            .output();
        if let Ok(o) = quotable {
            if let Ok(v) = serde_json::from_slice::<serde_json::Value>(&o.stdout) {
                if let (Some(c), Some(a)) = (
                    v.get("content").and_then(|x| x.as_str()),
                    v.get("author").and_then(|x| x.as_str()),
                ) {
                    if !c.trim().is_empty() {
                        return Some((c.to_string(), a.to_string()));
                    }
                }
            }
        }
        if let Ok(o) = std::process::Command::new("curl")
            .args(["-sL", "--max-time", "8", "https://zenquotes.io/api/random"])
            .output()
        {
            if let Ok(v) = serde_json::from_slice::<serde_json::Value>(&o.stdout) {
                if let Some(f) = v.as_array().and_then(|a| a.first()) {
                    if let (Some(q), Some(a)) = (
                        f.get("q").and_then(|x| x.as_str()),
                        f.get("a").and_then(|x| x.as_str()),
                    ) {
                        if !q.trim().is_empty() {
                            return Some((q.to_string(), a.to_string()));
                        }
                    }
                }
            }
        }
        None
    }

    /// Crypto ticker rows `(SYMBOL, "$price", 24h change %)` from CoinGecko's
    /// free `simple/price` endpoint. Worker-thread only; empty = fetch failed.
    pub(crate) fn fetch_ticker() -> Vec<(String, String, f32)> {
        pub(crate) const IDS: [&str; 7] =
            ["bitcoin", "ethereum", "solana", "cardano", "dogecoin", "litecoin", "monero"];
        let url = format!(
            "https://api.coingecko.com/api/v3/simple/price?ids={}&vs_currencies=usd&include_24hr_change=true",
            IDS.join(",")
        );
        let Ok(o) = std::process::Command::new("curl")
            .args(["-sL", "--max-time", "10"])
            .arg(url)
            .output()
        else {
            return Vec::new();
        };
        let Ok(v) = serde_json::from_slice::<serde_json::Value>(&o.stdout) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for id in IDS {
            let Some(coin) = v.get(id) else { continue };
            let Some(price) = coin.get("usd").and_then(|x| x.as_f64()) else {
                continue;
            };
            let chg = coin.get("usd_24h_change").and_then(|x| x.as_f64()).unwrap_or(0.0);
            let pstr = if price >= 1000.0 {
                format!("${:.0}", price)
            } else if price >= 1.0 {
                format!("${:.2}", price)
            } else {
                format!("${:.4}", price)
            };
            out.push((id.to_uppercase().to_string(), pstr, chg as f32));
        }
        out
    }

    // ---------------------------------------------------------------- mode

    pub(crate) fn set_mode(&mut self, mode: Mode) {
        if self.mode == mode {
            return;
        }
        self.prev_mode = self.mode;
        self.mode = mode;
        // leaving the dashboard resets its pan — the next open starts at the
        // top-left and the scrollbars reset to "not dragging"
        if mode != Mode::Expanded {
            self.dash_scroll_x = 0.0;
            self.dash_scroll_y = 0.0;
            self.dash_sb_drag = None;
        }
        if mode == Mode::Launcher {
            self.query.clear();
            self.sel = 0;
            self.invalidate_launcher_hits();
        }
        // fresh theme search each time the picker opens
        if mode == Mode::Themes {
            self.themes_query.clear();
            self.themes_scroll = 0;
        }
        // the lockscreen starts with a clean password field
        if mode == Mode::Lock {
            self.lock_pw.clear();
            self.lock_error = None;
            self.pending_lock_auth = false;
            self.lock_checking = false;
        }
        // leaving the power menu cancels any in-flight hold-to-confirm
        if mode != Mode::Power {
            self.power_hold = None;
        }
        // entering the wallpaper picker re-lists the thumbnails and starts
        // the grid at the top
        if mode == Mode::Wallpaper {
            self.wallpaper_scroll = 0;
            self.wp_sb_drag = false;
            self.refresh_wallpapers();
        }
        // entering the theme picker re-reads the master file + current id;
        // opening the dashboard re-syncs the dark/light toggle state
        if mode == Mode::Themes {
            self.themes_scroll = 0;
            self.refresh_themes();
        }
        // entering the keybind viewer re-reads $states2/keybinds_cache
        if mode == Mode::Keybinds {
            self.keybinds_scroll = 0;
            self.refresh_keybinds();
        }
        if mode == Mode::Clipboard {
            self.clip_scroll = 0;
        }
        if mode == Mode::Expanded {
            self.sync_dark_state();
            // re-list the wallpaper thumbnails so the dashboard Wallpaper
            // card is current each time the bar expands
            self.refresh_wallpapers();
            // Pack cards BEFORE target_size() so dash_rows / dash_cols are
            // fresh — otherwise the morph targets a stale (tiny) canvas
            // height, causing the first expand to render a cramped strip
            // before a second morph corrects it.
            self.refresh_pack();
        }
        // the ext-session-lock lockscreen owns a separate fullscreen surface,
        // so the bar itself must NOT morph — it stays at its resting size
        // behind the lock surface and pops back on unlock.
        if mode == Mode::Lock && self.lock_session {
            return;
        }
        let (tw, th) = self.target_size();
        self.start_morph(tw, th);
        self.recompute_hover();
    }

    /// The panel that hover / click on the resting pill opens: the dashboard,
    /// or the Control Center when Settings → "Expand to Control Center" is on.
    pub(crate) fn dash_mode(&self) -> Mode {
        if self.cc_as_dashboard {
            Mode::ControlCenter
        } else {
            Mode::Expanded
        }
    }

    pub(crate) fn set_hover(&mut self, h: bool) {
        if self.hover == h {
            return;
        }
        // The dashboard NEVER hover-collapses:
        //   • in EDIT MODE — only Esc / right-click / background-click exits;
        //     a wandering cursor must never yank the canvas away mid-edit
        //   • while a slider drag is live — the implicit grab owns the surface
        //     until release (the knob can cross the surface edge)
        if !h && (self.dash_edit || self.drag_key.is_some()) {
            return;
        }
        self.hover = h;
        // hover only drives the Collapsed ⇄ dashboard morph; panels stay put
        let dash = self.dash_mode();
        if self.mode == Mode::Collapsed && h {
            self.set_mode(dash);
        } else if self.mode == dash && !h {
            self.set_mode(Mode::Collapsed);
        }
    }

    /// Store an incoming notification; replaces by id. Callers must respect
    /// DND *before* calling (nothing is stored/surfaced).
    pub(crate) fn push_notif(&mut self, n: Notif) {
        self.notifs.retain(|x| x.id != n.id);
        self.notifs.push(n);
        let max = self.cfg.notifications.max_history.max(1);
        while self.notifs.len() > max {
            self.notifs.remove(0);
        }
        self.notif_count = self.notifs.len() as u32;
        self.save_notifs();
    }

    /// Remove a notification (notification UI is preserved; no daemon feeds it
    /// since mako owns the D-Bus name, so this currently has no caller).
    #[allow(dead_code)]
    pub(crate) fn remove_notif(&mut self, id: u32) {
        self.notifs.retain(|x| x.id != u64::from(id));
        self.notif_count = self.notifs.len() as u32;
        self.save_notifs();
    }

    /// Reload persisted notifications (history survives a bar restart).
    pub(crate) fn load_notifs(&mut self) {
        if let Ok(s) = std::fs::read_to_string(notif_path()) {
            if let Ok(v) = serde_json::from_str::<Vec<Notif>>(&s) {
                self.notifs = v.into_iter().take(64).collect();
                self.notif_count = self.notifs.len() as u32;
            }
        }
    }

    pub(crate) fn save_notifs(&self) {
        let Ok(json) = serde_json::to_string(&self.notifs) else { return };
        let p = notif_path();
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(p, json);
    }

    /// Settings → Appearance → Icon font: remember the chosen flavor, persist
    /// it to the channel master, and queue a runtime text-engine swap.
    pub(crate) fn set_icon_style(&mut self, tag: &str) {
        let tag = crate::icons::IconStyle::parse(tag).tag();
        self.icon_style = tag.to_string();
        self.save_config();
        self.pending_icon_style = Some(tag.to_string());
    }

    /// Persist current runtime toggles back to the master config file
    /// `$states/shell_{ui_state}` (the channel read from `$states2/s`).
    /// `sync_back` mirrors `$states/` to `$hdots/states/` at session end, so
    /// every change here survives a reboot. ONE file holds everything — base
    /// toggles, fonts, weather, and the pill geometry — per channel.
    pub(crate) fn save_config(&self) {
        let mut cfg = self.cfg.clone();
        // sync runtime toggles → config
        cfg.bar.expand_on_hover = Some(self.expand_on_hover);
        cfg.bar.cc_instead_of_dash = Some(self.cc_as_dashboard);
        cfg.bar.floating = Some(self.bar_floating);
        cfg.bar.reserve = Some(self.bar_reserve);
        cfg.bar.show_clock = Some(self.pill_clock);
        cfg.bar.show_battery = Some(self.pill_battery);
        cfg.bar.show_battery_pct = Some(self.pill_battery_pct);
        cfg.bar.show_ws = Some(self.pill_ws);
        cfg.bar.show_ws_long = Some(self.pill_ws_long);
        cfg.bar.show_ws_short = Some(self.pill_ws_short);
        cfg.bar.show_wifi = Some(self.pill_wifi);
        cfg.bar.show_bluetooth = Some(self.pill_bluetooth);
        cfg.bar.show_volume = Some(self.pill_volume);
        cfg.bar.show_wallpaper = Some(self.pill_wallpaper);
        cfg.bar.show_themes = Some(self.pill_themes);
        cfg.bar.show_brand = Some(self.pill_branding);
        cfg.bar.show_tray = Some(self.pill_tray);
        cfg.bar.show_viz = Some(self.pill_viz);
        cfg.bar.settings_on_pill = Some(self.pill_settings);
        cfg.bar.show_notif = Some(self.pill_notif);
        cfg.bar.show_user = Some(self.pill_user);
        cfg.bar.show_watts = Some(self.pill_watts);
        cfg.bar.show_brightness = Some(self.pill_brightness);
        cfg.bar.anchor = Some(self.bar_edge.name().into());
        cfg.bar.pill_margin_x = Some(self.pill_margin_x);
        cfg.bar.pill_alpha = Some(self.pill_alpha);
        cfg.bar.dash_card_alpha = Some(self.dash_card_alpha);
        cfg.bar.power_hold_delay = Some(self.power_hold_delay);
        cfg.bar.allow_over_100 = Some(self.allow_over_100);
        cfg.bar.max_vol = Some(self.max_vol);
        // icon-font flavor (Settings → Appearance → "Icon font")
        cfg.fonts.icon_style = self.icon_style.clone();
        // visualizer bar style (Viz card button)
        cfg.viz_style = Some(self.viz_style);
        // mirror-card capture rate (fps button)
        cfg.mirror_fps = Some(self.mirror_fps);
        // mirror-card "show actual fps" readout
        cfg.mirror_show_fps = Some(self.mirror_show_fps);
        // sliders-card "show balance" / "show saturation" toggles
        cfg.show_balance = Some(self.show_balance);
        cfg.show_saturation = Some(self.show_saturation);
        // audio recorder card toggles (confirm-delete / show-mic-slider)
        cfg.audiorec_confirm_delete = Some(self.audiorec_confirm_delete);
        cfg.audiorec_show_mic = Some(self.audiorec_show_mic);
        // dashboard card-header visibility (edit-mode strip-editor toggles)
        cfg.card_show_title = Some(self.card_show_title);
        cfg.card_show_glyph = Some(self.card_show_glyph);
        // dashboard grid zoom — every layout metric derives from this, so it
        // must land in the same channel master as the pill layout
        cfg.bar.grid_scale = self.scale.0;
        // picker thumbnail zoom is remembered too
        cfg.wallpaper.cell = Some(self.wp_cell);
        cfg.bar.pill_order = Some(
            self.pill_order
                .iter()
                .map(|item| match item {
                    PillItem::Workspaces => "workspaces",
                    PillItem::Clock => "clock",
                    PillItem::Battery => "battery",
                    PillItem::Watts => "watts",
                    PillItem::Visualizer => "visualizer",
                    PillItem::Settings => "settings",
                    PillItem::Bell => "bell",
                    PillItem::Tray => "tray",
                    PillItem::User => "user",
                    PillItem::WorkspacesLong => "workspaces_long",
                    PillItem::WorkspacesShort => "workspaces_short",
                    PillItem::Wifi => "wifi",
                    PillItem::Bluetooth => "bluetooth",
                    PillItem::Volume => "volume",
                    PillItem::Wallpaper => "wallpaper",
                    PillItem::Themes => "themes",
                    PillItem::Branding => "branding",
                })
                .map(String::from)
                .collect(),
        );
        // expanded-banner strip order + dashboard on/off (persisted runtime)
        cfg.bar.banner_order = Some(
            self.banner_order.iter().map(|t| t.to_str()).collect(),
        );
        cfg.bar.dash_enabled = Some(self.dash_enabled);
        cfg.bar.dash_scroll_dir = Some(self.dash_scroll_dir);
        // dashboard edit-mode steppers (card radius linked to window radius)
        cfg.bar.ui_pad = Some(self.ui_pad);
        cfg.bar.win_radius = Some(self.win_radius);
        cfg.bar.card_radius = Some(self.win_radius);
        cfg.bar.card_radius_sync = Some(true);
        // app-shortcut card: pinned app ids (persisted per channel)
        cfg.app_shortcuts = Some(self.app_shortcuts.clone());
        // manual banner-only strip width (0 = auto-fit); None hides the key
        cfg.bar.banner_width = if self.banner_w > 0 { Some(self.banner_w) } else { None };
        // dashboard cards: remember the ENABLED cards with their exact packed
        // size + position (cells) plus the cards parked in the tray — all
        // per channel, so each of n/d/l keeps its own dashboard.
        cfg.dash_cards = self
            .packed_layout
            .iter()
            .map(|l| crate::config::DashCardEntry {
                card: l.card.id().to_string(),
                x: l.x,
                y: l.y,
                w: l.w,
                h: l.h,
            })
            .collect();
        cfg.dash_tray = self.tray_cards.iter().map(|c| c.id().to_string()).collect();
        // edit-mode grid boost — remembered with the per-channel dashboard
        cfg.dash_grid_cols = if self.edit_grid_cols > 0 { Some(self.edit_grid_cols) } else { None };
        cfg.dash_grid_rows = if self.edit_grid_rows > 0 { Some(self.edit_grid_rows) } else { None };
        // the whole config — base + pill geometry — lands in the channel master
        cfg.save(&self.per_state_config);
    }

    // ------------------------------------------------------------------ to-do

    /// to-do store: `~/.local/share/zen-shell/todos.txt`, one task per line
    /// as `1 text` / `0 text` (done flag + single-line text).
    pub(crate) fn todos_path() -> std::path::PathBuf {
        let base = std::env::var("XDG_DATA_HOME")
            .ok()
            .filter(|s| !s.is_empty())
            .map(std::path::PathBuf::from)
            .or_else(|| std::env::var("HOME").ok().map(|h| std::path::Path::new(&h).join(".local/share")))
            .unwrap_or_else(|| std::path::PathBuf::from("/tmp"));
        base.join("zen-shell").join("todos.txt")
    }

    /// Pomodoro persistence: `focus <min>` / `break <min>` / `done <n>` lines.
    pub(crate) fn pomo_path() -> std::path::PathBuf {
        Self::todos_path().with_file_name("pomo.txt")
    }

    /// World-clock persistence: one `city\ttz` line per pinned zone.
    pub(crate) fn worldclock_path() -> std::path::PathBuf {
        Self::todos_path().with_file_name("zones.txt")
    }

    /// Water tracker persistence: `goal <n>` + `YYYY-MM-DD <count>` lines.
    pub(crate) fn water_path() -> std::path::PathBuf {
        Self::todos_path().with_file_name("water.txt")
    }

    /// Today's `YYYY-MM-DD` (local) — the water card's day key.
    pub(crate) fn water_today() -> String {
        let epoch = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let mut buf = [0u8; 16];
        unsafe {
            let t = epoch as libc::time_t;
            let mut tm: libc::tm = std::mem::zeroed();
            libc::localtime_r(&t, &mut tm);
            let n = libc::strftime(buf.as_mut_ptr() as *mut libc::c_char, buf.len(), c"%Y-%m-%d".as_ptr(), &tm);
            String::from_utf8_lossy(&buf[..n]).into_owned()
        }
    }

    /// Load the water log, rolling over yesterday's count (midnight reset).
    pub(crate) fn load_water(&mut self) {
        let today = Self::water_today();
        let Ok(s) = std::fs::read_to_string(Self::water_path()) else {
            self.water_day = today;
            return;
        };
        for l in s.lines() {
            let Some((k, v)) = l.split_once(' ') else { continue };
            match k {
                "goal" => self.water_goal = v.parse().unwrap_or(8).clamp(1, 30),
                "day" => self.water_day = v.to_string(),
                "count" => self.water_glasses = v.parse().unwrap_or(0),
                _ => {}
            }
        }
        if self.water_day != today {
            // new day → fresh count (yesterday's number is not kept)
            self.water_glasses = 0;
            self.water_day = today;
            self.save_water();
        }
    }

    pub(crate) fn save_water(&self) {
        let p = Self::water_path();
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(p, format!("goal {}\nday {}\ncount {}\n", self.water_goal, self.water_day, self.water_glasses));
    }

    /// Log one glass (250 ml). Caps at 3× the goal so a stuck click can't
    /// run the count away.
    pub(crate) fn water_add(&mut self) {
        self.roll_water_day();
        if self.water_glasses < self.water_goal.saturating_mul(3).max(30) {
            self.water_glasses += 1;
            self.save_water();
        }
    }

    /// Undo one glass.
    pub(crate) fn water_sub(&mut self) {
        self.roll_water_day();
        self.water_glasses = self.water_glasses.saturating_sub(1);
        self.save_water();
    }

    /// Adjust the daily goal in whole glasses (clamped 1..30).
    pub(crate) fn water_set_goal(&mut self, goal: u32) {
        let g = goal.clamp(1, 30);
        if g != self.water_goal {
            self.water_goal = g;
            self.save_water();
        }
    }

    /// Midnight rollover: if the stored day ≠ today, reset the count.
    pub(crate) fn roll_water_day(&mut self) {
        let today = Self::water_today();
        if self.water_day != today {
            self.water_day = today;
            self.water_glasses = 0;
            self.save_water();
        }
    }

    /// Countdown persistence: `label\tdate` lines sorted by date.
    pub(crate) fn countdown_path() -> std::path::PathBuf {
        Self::todos_path().with_file_name("countdown.txt")
    }

    pub(crate) fn load_countdown(&mut self) {
        let Ok(s) = std::fs::read_to_string(Self::countdown_path()) else { return };
        self.countdown_events = s
            .lines()
            .filter_map(|l| {
                let (label, date) = l.split_once('\t')?;
                Some((label.to_string(), date.to_string()))
            })
            .collect();
        self.sort_countdown();
    }

    pub(crate) fn save_countdown(&self) {
        let p = Self::countdown_path();
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let mut out = String::new();
        for (label, date) in &self.countdown_events {
            out.push_str(label);
            out.push('\t');
            out.push_str(date);
            out.push('\n');
        }
        let _ = std::fs::write(p, out);
    }

    pub(crate) fn sort_countdown(&mut self) {
        self.countdown_events.sort_by(|a, b| a.1.cmp(&b.1));
    }

    /// Add a countdown event from `Name YYYY-MM-DD` (composer commit).
    pub(crate) fn countdown_add(&mut self) {
        let Some(buf) = self.countdown_input.as_mut() else { return };
        let text = buf.trim().to_string();
        buf.clear();
        // last token = date, rest = label
        let Some((label, date)) = text.rsplit_once(' ') else { return };
        let date = date.trim();
        let db = date.as_bytes();
        if label.is_empty() || date.len() != 10 || db[4] != b'-' || db[7] != b'-' {
            return;
        }
        if self.countdown_events.len() >= 16 {
            return;
        }
        self.countdown_events.push((label.trim().to_string(), date.to_string()));
        self.sort_countdown();
        self.save_countdown();
    }

    /// Days from today until `date` (negative = passed). Pure libc math.
    pub(crate) fn countdown_days(&self, date: &str) -> i64 {
        let b = date.as_bytes();
        if date.len() != 10 || b[4] != b'-' || b[7] != b'-' {
            return 0;
        }
        let (y, m, d) = (
            date[0..4].parse::<i32>().unwrap_or(0),
            date[5..7].parse::<i32>().unwrap_or(1),
            date[8..10].parse::<i32>().unwrap_or(1),
        );
        let now = unsafe { libc::time(std::ptr::null_mut()) };
        let mut tm: libc::tm = unsafe { std::mem::zeroed() };
        unsafe { libc::localtime_r(&now, &mut tm) };
        // +1 so today's day-number is 1-based, matching `doy` below.
        let today_days = (tm.tm_year as i64 + 1900) * 365 + tm.tm_yday as i64 + 1;
        // naive day-number for the target (good enough for day-diffs)
        let leap = |y: i32| (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
        let mdays = [31, if leap(y) { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
        let mut doy = d as i64;
        for mm in 1..m.clamp(1, 12) {
            doy += mdays[(mm - 1) as usize] as i64;
        }
        y as i64 * 365 + doy - today_days
    }

    /// Alarms persistence: `HH:MM\tlabel` lines.
    pub(crate) fn alarms_path() -> std::path::PathBuf {
        Self::todos_path().with_file_name("alarms.txt")
    }

    pub(crate) fn load_alarms(&mut self) {
        let Ok(s) = std::fs::read_to_string(Self::alarms_path()) else { return };
        self.alarm_list = s
            .lines()
            .filter_map(|l| {
                let (time, label) = l.split_once('\t')?;
                Some((time.to_string(), label.to_string()))
            })
            .collect();
        self.alarm_list.sort_by(|a, b| a.0.cmp(&b.0));
    }

    pub(crate) fn save_alarms(&self) {
        let p = Self::alarms_path();
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let mut out = String::new();
        for (time, label) in &self.alarm_list {
            out.push_str(time);
            out.push('\t');
            out.push_str(label);
            out.push('\n');
        }
        let _ = std::fs::write(p, out);
    }

    /// Commit an alarm from `HH:MM label` (composer commit).
    pub(crate) fn alarm_add(&mut self) {
        let Some(buf) = self.alarm_input.as_mut() else { return };
        let text = buf.trim().to_string();
        buf.clear();
        let Some((time, label)) = text.split_once(' ') else { return };
        let tb = time.as_bytes();
        let valid = time.len() == 5
            && tb[0].is_ascii_digit()
            && tb[1].is_ascii_digit()
            && tb[2] == b':'
            && tb[3].is_ascii_digit()
            && tb[4].is_ascii_digit()
            && time[0..2].parse::<u32>().map(|h| h < 24).unwrap_or(false)
            && time[3..5].parse::<u32>().map(|m| m < 60).unwrap_or(false);
        if !valid || self.alarm_list.len() >= 16 {
            return;
        }
        self.alarm_list.push((time.to_string(), label.trim().to_string()));
        self.alarm_list.sort_by(|a, b| a.0.cmp(&b.0));
        self.save_alarms();
    }

    /// Check fired alarms (1 s pomo-timer tick): fire the popup once per
    /// day per alarm. Returns (label, time) to fire, if any.
    pub(crate) fn alarms_due(&mut self) -> Option<(String, String)> {
        let now = unsafe { libc::time(std::ptr::null_mut()) };
        let mut tm: libc::tm = unsafe { std::mem::zeroed() };
        unsafe { libc::localtime_r(&now, &mut tm) };
        let mut buf = [0u8; 8];
        let n = unsafe {
            libc::strftime(buf.as_mut_ptr() as *mut libc::c_char, buf.len(), c"%H:%M".as_ptr(), &tm)
        };
        let hhmm = String::from_utf8_lossy(&buf[..n]).into_owned();
        let mut ymd = [0u8; 12];
        let n2 = unsafe {
            libc::strftime(ymd.as_mut_ptr() as *mut libc::c_char, ymd.len(), c"%Y-%m-%d".as_ptr(), &tm)
        };
        let day = String::from_utf8_lossy(&ymd[..n2]).into_owned();
        // reset the fired list on a new day
        if self.alarms_fired.len() > 1 && !self.alarms_fired[0].starts_with(&day) {
            self.alarms_fired.clear();
        }
        if let Some((_, label)) = self.alarm_list.iter().find(|(t, _)| *t == hhmm) {
            let marker = format!("{day} {hhmm}");
            if !self.alarms_fired.contains(&marker) {
                self.alarms_fired.push(marker);
                return Some((label.clone(), hhmm));
            }
        }
        None
    }

    /// Snippets persistence: `name\tbody` lines.
    pub(crate) fn snippets_path() -> std::path::PathBuf {
        Self::todos_path().with_file_name("snippets.txt")
    }

    pub(crate) fn load_snippets(&mut self) {
        let Ok(s) = std::fs::read_to_string(Self::snippets_path()) else { return };
        self.snippets = s
            .lines()
            .filter_map(|l| {
                let (name, body) = l.split_once('\t')?;
                Some((name.to_string(), body.to_string()))
            })
            .collect();
    }

    pub(crate) fn save_snippets(&self) {
        let p = Self::snippets_path();
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let mut out = String::new();
        for (name, body) in &self.snippets {
            out.push_str(name);
            out.push('\t');
            out.push_str(body);
            out.push('\n');
        }
        let _ = std::fs::write(p, out);
    }

    /// Commit a snippet from `name body…` (composer commit; body may hold
    /// spaces, name may not).
    pub(crate) fn snippet_add(&mut self) {
        let Some(buf) = self.snippet_input.as_mut() else { return };
        let text = buf.trim().to_string();
        buf.clear();
        let Some((name, body)) = text.split_once(' ') else { return };
        if name.is_empty() || body.trim().is_empty() || self.snippets.len() >= 32 {
            return;
        }
        self.snippets.push((name.to_string(), body.trim().to_string()));
        self.save_snippets();
    }

    /// Expenses persistence: `cents\tcategory\tdate` lines.
    pub(crate) fn expenses_path() -> std::path::PathBuf {
        Self::todos_path().with_file_name("expenses.txt")
    }

    pub(crate) fn load_expenses(&mut self) {
        let Ok(s) = std::fs::read_to_string(Self::expenses_path()) else { return };
        self.expenses = s
            .lines()
            .filter_map(|l| {
                let (cents, rest) = l.split_once('\t')?;
                let (cat, date) = rest.split_once('\t')?;
                Some((cents.parse().ok()?, cat.to_string(), date.to_string()))
            })
            .collect();
    }

    pub(crate) fn save_expenses(&self) {
        let p = Self::expenses_path();
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let mut out = String::new();
        for (cents, cat, date) in &self.expenses {
            out.push_str(&cents.to_string());
            out.push('\t');
            out.push_str(cat);
            out.push('\t');
            out.push_str(date);
            out.push('\n');
        }
        let _ = std::fs::write(p, out);
    }

    /// Log `cents` in `cat` today. Caps the list so the file stays small.
    pub(crate) fn expense_add(&mut self, cents: u32, cat: &str) {
        if cents == 0 || self.expenses.len() >= 2000 {
            return;
        }
        self.expenses.push((cents, cat.to_string(), Self::water_today()));
        self.save_expenses();
    }

    /// Commit a custom expense from `12.50 category` (composer commit; a
    /// missing category falls back to "misc").
    pub(crate) fn expense_commit(&mut self) {
        let Some(buf) = self.expense_input.as_mut() else { return };
        let text = buf.trim().to_string();
        buf.clear();
        let mut it = text.splitn(2, ' ');
        let amount = it.next().unwrap_or("");
        let cat = it.next().unwrap_or("misc").trim();
        let cat = if cat.is_empty() { "misc" } else { cat };
        // accept "12" or "12.50" / "12,50"
        let norm = amount.replace(',', ".");
        let Ok(v) = norm.parse::<f32>() else { return };
        let cents = (v * 100.0).round() as u32;
        self.expense_add(cents, cat);
    }

    /// Month-to-date total (major units) + top categories.
    pub(crate) fn expense_month_summary(&self) -> (f32, Vec<(String, f32)>) {
        let today = Self::water_today();
        let month = &today[..7];
        let mut per: Vec<(String, f32)> = Vec::new();
        let mut total = 0.0f32;
        for (cents, cat, date) in &self.expenses {
            if !date.starts_with(month) {
                continue;
            }
            let v = *cents as f32 / 100.0;
            total += v;
            match per.iter_mut().find(|(c, _)| c == cat) {
                Some((_, s)) => *s += v,
                None => per.push((cat.clone(), v)),
            }
        }
        per.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        (total, per)
    }

    pub(crate) fn load_worldclock_zones(&mut self) {
        let Ok(s) = std::fs::read_to_string(Self::worldclock_path()) else {
            // first launch → the classic four
            self.worldclock_zones = vec![
                ("UTC".into(), "UTC".into(), 0, "UTC".into()),
                ("New York".into(), "America/New_York".into(), -5 * 60, "EST".into()),
                ("London".into(), "Europe/London".into(), 0, "GMT".into()),
                ("Tokyo".into(), "Asia/Tokyo".into(), 9 * 60, "JST".into()),
            ];
            return;
        };
        self.worldclock_zones = s
            .lines()
            .filter_map(|l| {
                let (city, tz) = l.split_once('\t')?;
                Some((city.to_string(), tz.to_string(), 0, String::new()))
            })
            .collect();
        self.worldclock_probed_at = None; // offsets refresh on the next probe
    }

    pub(crate) fn save_worldclock_zones(&self) {
        let p = Self::worldclock_path();
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let mut out = String::new();
        for (city, tz, _, _) in &self.worldclock_zones {
            out.push_str(city);
            out.push('\t');
            out.push_str(tz);
            out.push('\n');
        }
        let _ = std::fs::write(p, out);
    }

    pub(crate) fn load_pomo(&mut self) {
        let Ok(s) = std::fs::read_to_string(Self::pomo_path()) else { return };
        for l in s.lines() {
            let Some((k, v)) = l.split_once(' ') else { continue };
            match k {
                "focus" => self.pomo_focus_min = v.parse().unwrap_or(25).clamp(1, 180),
                "break" => self.pomo_break_min = v.parse().unwrap_or(5).clamp(1, 60),
                "done" => self.pomo_sessions = v.parse().unwrap_or(0),
                _ => {}
            }
        }
    }

    pub(crate) fn save_pomo(&self) {
        let p = Self::pomo_path();
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(p, format!("focus {}\nbreak {}\ndone {}\n", self.pomo_focus_min, self.pomo_break_min, self.pomo_sessions));
    }

    /// Remaining seconds of the current pomodoro phase.
    pub(crate) fn pomo_remaining_secs(&self) -> u64 {
        if let Some(dl) = self.pomo_deadline {
            dl.saturating_duration_since(Instant::now()).as_secs()
        } else if let Some(r) = self.pomo_paused_rem {
            r
        } else if self.pomo_phase_focus {
            self.pomo_focus_min as u64 * 60
        } else {
            self.pomo_break_min as u64 * 60
        }
    }

    /// Advance the pomodoro: flip phase on deadline, bump the session
    /// counter after each focus phase, auto-start the next phase. Returns
    /// true when the phase flipped (caller re-renders).
    pub(crate) fn pomo_tick(&mut self) -> bool {
        if !self.pomo_running {
            return false;
        }
        let Some(dl) = self.pomo_deadline else { return false };
        if Instant::now() < dl {
            return false;
        }
        if self.pomo_phase_focus {
            self.pomo_sessions += 1;
        }
        self.pomo_phase_focus = !self.pomo_phase_focus;
        let mins = if self.pomo_phase_focus { self.pomo_focus_min } else { self.pomo_break_min };
        self.pomo_deadline = Some(Instant::now() + std::time::Duration::from_secs(mins as u64 * 60));
        self.save_pomo();
        true
    }

    /// Start / pause / resume the timer.
    pub(crate) fn pomo_toggle(&mut self) {
        if self.pomo_running {
            self.pomo_paused_rem = Some(self.pomo_remaining_secs());
            self.pomo_deadline = None;
            self.pomo_running = false;
        } else {
            let rem = self.pomo_paused_rem.take().unwrap_or_else(|| self.pomo_remaining_secs());
            self.pomo_deadline = Some(Instant::now() + std::time::Duration::from_secs(rem));
            self.pomo_running = true;
        }
    }

    /// Reset to a fresh focus phase (stops the timer).
    pub(crate) fn pomo_reset(&mut self) {
        self.pomo_running = false;
        self.pomo_paused_rem = None;
        self.pomo_deadline = None;
        self.pomo_phase_focus = true;
    }

    /// Adjust the focus length (steppers); only while idle.
    pub(crate) fn pomo_adjust_focus(&mut self, delta: i32) {
        if self.pomo_running {
            return;
        }
        let next = (self.pomo_focus_min as i32 + delta).clamp(1, 180) as u32;
        if next != self.pomo_focus_min {
            self.pomo_focus_min = next;
            self.save_pomo();
        }
    }

    pub(crate) fn load_todos() -> Vec<(String, bool)> {
        let Ok(s) = std::fs::read_to_string(Self::todos_path()) else { return Vec::new() };
        s.lines()
            .filter_map(|l| {
                let (flag, text) = l.split_once(' ')?;
                let text = text.trim();
                if text.is_empty() {
                    return None;
                }
                Some((text.to_string(), flag == "1"))
            })
            .collect()
    }

    pub(crate) fn save_todos(&self) {
        let p = Self::todos_path();
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let mut out = String::new();
        for (text, done) in &self.todos {
            out.push_str(if *done { "1 " } else { "0 " });
            out.push_str(text);
            out.push('\n');
        }
        let _ = std::fs::write(p, out);
    }

    /// Toggle the done flag of the visible to-do row `i`.
    pub(crate) fn todo_toggle(&mut self, i: usize) {
        if i < self.todos.len() {
            self.todos[i].1 = !self.todos[i].1;
            self.save_todos();
        }
    }

    /// Delete visible to-do row `i` and close any composer focus above it.
    pub(crate) fn todo_delete(&mut self, i: usize) {
        if i < self.todos.len() {
            self.todos.remove(i);
            self.save_todos();
        }
    }

    /// Commit the composer buffer as a new task (keeps focus for chaining).
    pub(crate) fn todo_commit(&mut self) {
        let Some(buf) = self.todo_input.as_mut() else { return };
        let text = buf.trim().to_string();
        buf.clear();
        if text.is_empty() || self.todos.len() >= 50 {
            return;
        }
        self.todos.push((text, false));
        self.save_todos();
    }

    // ------------------------------------------------------- metro grid

    /// Layout store: same dir as todos — one line per card `name x y w h`.
    pub(crate) fn layout_path() -> std::path::PathBuf {
        Self::todos_path().with_file_name("dash_layout.txt")
    }

    /// Load the saved grid. v5/v4 file = marker line, an optional
    /// `tray <id>…` line (cards parked off-dashboard), then `name w h`
    /// lines in PACK ORDER (positions are derived by the flow packer).
    /// Missing cards are appended from the defaults; unknown names / bad
    /// lines are skipped. v4 files predate the fine 20-col lattice — their
    /// spans are doubled on load. Older formats fall back to defaults.
    pub(crate) fn load_dash_layout() -> (Vec<CardLayout>, Vec<DashCard>) {
        let mut out: Vec<CardLayout> = Vec::new();
        let mut tray: Vec<DashCard> = Vec::new();
        let Ok(s) = std::fs::read_to_string(Self::layout_path()) else {
            return (default_dash_layout(), Vec::new());
        };
        let mut lines = s.lines();
        // v4 spans live on the old coarse 10-col lattice — double them
        let mut upscale = 1u16;
        match lines.next() {
            Some(marker) if marker.starts_with("zen-layout-v5") => {}
            Some(marker) if marker.starts_with("zen-layout-v4") => upscale = 2,
            _ => return (default_dash_layout(), Vec::new()),
        }
        for line in lines {
            if let Some(rest) = line.strip_prefix("tray ") {
                for name in rest.split_whitespace() {
                    if let Some(card) = DashCard::from_id(name) {
                        if !tray.contains(&card) {
                            tray.push(card);
                        }
                    }
                }
                continue;
            }
            let mut it = line.split_whitespace();
            let (Some(name), Some(w), Some(h)) =
                (it.next(), it.next(), it.next())
            else {
                continue;
            };
            let Some(card) = DashCard::from_id(name) else { continue };
            let Ok((w, h)) = w.parse::<u16>().and_then(|w| h.parse::<u16>().map(|h| (w, h)))
            else {
                continue;
            };
            if !out.iter().any(|l| l.card == card) && !tray.contains(&card) {
                out.push(CardLayout { card, x: 0, y: 0, w: w * upscale, h: h * upscale });
            }
        }
        // append any cards the file didn't mention at all — new cards must
        // surface on the dashboard even for old files
        for d in default_dash_layout() {
            if !out.iter().any(|l| l.card == d.card) && !tray.contains(&d.card) {
                out.push(d);
            }
        }
        (out, tray)
    }

    pub(crate) fn save_dash_layout(&self) {
        let p = Self::layout_path();
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let mut out = String::from("zen-layout-v5\n");
        if !self.tray_cards.is_empty() {
            out.push_str("tray");
            for c in &self.tray_cards {
                out.push(' ');
                out.push_str(c.id());
            }
            out.push('\n');
        }
        for l in &self.dash_layout {
            out.push_str(&format!("{} {} {}\n", l.card.id(), l.w, l.h));
        }
        let _ = std::fs::write(p, out);
    }

    /// Overlay the per-channel dashboard memory (`$states/shell_{state}`) on
    /// top of the legacy order+spans layout file. The config entries carry
    /// each ENABLED card's exact packed size + position, so we adopt them
    /// verbatim and PIN the layout so `refresh_pack` renders them exactly
    /// where they were saved (no re-packing drift) until the user edits.
    /// Cards in `dash_tray` sit parked. Unknown ids, out-of-bounds spans and
    /// already-present tray entries are skipped.
    pub(crate) fn apply_persisted_dashboard(shell: &mut Shell) {
        let mut applied = false;
        // apply per-card edits onto the existing dash_layout (order authority)
        for e in &shell.cfg.dash_cards {
            let Some(card) = DashCard::from_id(&e.card) else { continue };
            if e.w == 0
                || e.h == 0
                || e.w > DASH_COLS
                || e.h > DASH_MAX_ROWS
                || e.x >= DASH_COLS
                || e.y >= DASH_MAX_ROWS
            {
                continue;
            }
            applied = true;
            if let Some(l) = shell.dash_layout.iter_mut().find(|l| l.card == card) {
                *l = CardLayout { card, x: e.x, y: e.y, w: e.w, h: e.h };
            } else {
                shell.dash_layout.push(CardLayout { card, x: e.x, y: e.y, w: e.w, h: e.h });
            }
        }
        // parked-cards tray from the channel config
        if !shell.cfg.dash_tray.is_empty() {
            let mut tray: Vec<DashCard> = Vec::new();
            for id in &shell.cfg.dash_tray {
                if let Some(card) = DashCard::from_id(id) {
                    if !tray.contains(&card) {
                        tray.push(card);
                    }
                }
            }
            if !tray.is_empty() {
                // parked cards must not also be on the grid
                for c in &tray {
                    shell.dash_layout.retain(|l| l.card != *c);
                }
                shell.tray_cards = tray;
            }
        }
        if applied {
            shell.layout_pinned = true;
        }
        // edit-mode grid boost ("+" / "-" buttons) — remembered per channel
        shell.edit_grid_cols = shell.cfg.dash_grid_cols.unwrap_or(0).min(512);
        shell.edit_grid_rows = shell.cfg.dash_grid_rows.unwrap_or(0).min(shell.rows_cap());
    }

    /// Build the dashboard grid + parked tray from the declarative `[dashboard]`
    /// spec when its `cards.run` list is declared (authority over the
    /// remembered per-channel layout). Cards keep their default span and pack
    /// in spec order; the rest of the grid is dropped. `cards.parked` sets the
    /// tray. Called on every config sync so hot edits reapply.
    pub(crate) fn apply_dashboard_spec_layout(shell: &mut Shell) {
        use crate::shell::{CardLayout, DashCard};
        let run = &shell.cfg.dashboard.cards.run;
        let parked = &shell.cfg.dashboard.cards.parked;
        let defaults = crate::shell::default_dash_layout();
        let mut layout: Vec<CardLayout> = Vec::new();
        let mut tray: Vec<DashCard> = Vec::new();
        let mut have = std::collections::HashSet::new();
        // grid cards in spec order, spans from the canonical defaults
        for id in run {
            let Some(card) = DashCard::from_id(id) else { continue };
            if !have.insert(card) {
                continue;
            }
            let span = defaults.iter().find(|d| d.card == card);
            let (w, h) = span.map(|d| (d.w, d.h)).unwrap_or((6, 4));
            layout.push(CardLayout { card, x: 0, y: 0, w, h });
        }
        // parked tray from spec (dedup + skip anything already on the grid)
        if !parked.is_empty() {
            for id in parked {
                let Some(card) = DashCard::from_id(id) else { continue };
                if have.contains(&card) {
                    continue;
                }
                have.insert(card);
                tray.push(card);
            }
        }
        if !layout.is_empty() {
            shell.dash_layout = layout;
            shell.tray_cards = tray;
            shell.layout_pinned = false;
        }
    }

    /// Pack the current card ORDER into placements. While an edit drag is
    /// live the PREVIEW pack wins (others visibly slide aside); otherwise a
    /// fresh pack of the committed order. When `layout_pinned` is set (a
    /// per-channel dashboard was just restored) it trusts the exact saved
    /// x/y in `dash_layout` instead of re-packing, so cards render exactly
    /// where they were remembered. Refreshes `packed_layout` + extents.
    pub(crate) fn refresh_pack(&mut self) {
        let wmax = self.grid_max_cols();
        let packed = match &self.edit_preview_pack {
            Some((p, _)) => p.clone(),
            None if self.layout_pinned => {
                // When layout_pinned, trust the saved positions — but
                // validate first: if any two cards share a cell (corrupt
                // config / migration from old format), discard the pin and
                // re-pack so cards never overlap.
                let mut occ = std::collections::HashSet::new();
                let mut overlaps = false;
                for l in &self.dash_layout {
                    for dy in 0..l.h {
                        for dx in 0..l.w {
                            if !occ.insert((l.x + dx, l.y + dy)) {
                                overlaps = true;
                                break;
                            }
                        }
                        if overlaps { break; }
                    }
                    if overlaps { break; }
                }
                if overlaps {
                    self.layout_pinned = false;
                    pack_cards_max(&self.dash_layout, wmax).0
                } else {
                    // collapse all-empty rows → cards fall up into free space
                    compact_rows_up(&self.dash_layout)
                }
            },
            None if self.dash_scroll_dir => {
                // Metro ribbon (horizontal scroll): column-major cascade.
                // The ribbon auto-grows RIGHTWARD, overflowing the viewport
                // so the bottom scroller pans it. Column height follows the
                // "rows" edit-chip.
                let col_rows = self.edit_grid_rows.max(DASH_ROWS).min(self.rows_cap());
                pack_metro(&self.dash_layout, col_rows).0
            }
            None => pack_cards_max(&self.dash_layout, wmax).0,
        };
        self.dash_cols = if self.dash_scroll_dir {
            // horizontal ribbon: let the canvas extend past the lattice so
            // the board overflows and scrolls (metro width is content width)
            packed.iter().map(|l| l.x + l.w).max().unwrap_or(1).min(512)
        } else {
            // vertical stack: board width is the card extent, hard-capped at
            // the 16-column / strip-width clamp (`wmax`)
            packed.iter().map(|l| l.x + l.w).max().unwrap_or(1).clamp(1, wmax)
        };
        self.dash_rows = packed.iter().map(|l| l.y + l.h).max().unwrap_or(1).max(1);
        self.packed_layout = packed;
        self.clamp_scroll();
    }

    /// True when `card` sits anywhere on the (packed) dashboard grid. The
    /// 3 s services timer uses this to poll per-card background data only
    /// while the card is actually on screen — 0 fetches otherwise.
    pub(crate) fn card_packed(&self, card: DashCard) -> bool {
        self.packed_layout.iter().any(|l| l.card == card)
    }

    /// Edit-mode auto-arrange (HARD RULES — see README): cards gravitate
    /// toward the top-left. Ran immediately after a ✕-delete and every ~3 s
    /// while editing. Two moves only: a card shifts LEFT to fill a gap (right
    /// → left cascade), and when a row above has room the LEFTMOST card of
    /// the row below climbs INTO the RIGHTMOST free slot of the row above
    /// (bottom → top). Nothing else can relocate a card. Returns true when
    /// anything moved (cards then glide via the edit-mode easing pass).
    pub(crate) fn autoarrange(&mut self) -> bool {
        // never fight a live drag/resize — the pointer owns the ghost slot
        if self.edit_drag.is_some() || self.edit_resize.is_some() {
            return false;
        }
        // horizontal Metro ribbon packs each card itself — no 2D gravity
        // re-flow (rows were re-defined as column height)
        if self.dash_scroll_dir {
            return false;
        }
        if self.dash_layout.is_empty() {
            return false;
        }
        let cols = self.grid_max_cols();
        let (compacted, moved) = gravity_pack(&self.dash_layout, cols);
        if !moved {
            return false;
        }
        // commit into both the persisted layout and the rendered pack so the
        // easing pass glides cards (no flow-packer re-jumps)
        self.dash_layout = compacted.clone();
        self.dash_cols = compacted.iter().map(|l| l.x + l.w).max().unwrap_or(1).clamp(1, cols);
        self.dash_rows = compacted.iter().map(|l| l.y + l.h).max().unwrap_or(1).max(1);
        self.packed_layout = compacted;
        self.save_dash_layout();
        self.save_config();
        self.clamp_scroll();
        true
    }

    // ---- launcher -------------------------------------------------------

    pub(crate) fn invalidate_launcher_hits(&mut self) {
        self.launcher_hits.clear();
        self.launcher_hits_query = String::from("\0");
    }

    pub(crate) fn ensure_launcher_hits(&mut self) {
        if self.launcher_hits_query == self.query {
            return;
        }
        self.launcher_hits = self.apps.search_indices(&self.query, 20);
        self.launcher_hits_query = self.query.clone();
        if self.sel >= self.launcher_hits.len() {
            self.sel = self.launcher_hits.len().saturating_sub(1);
        }
        self.launcher_scroll = self.launcher_scroll.min(self.launcher_max_scroll());
    }

    /// How many launcher rows fit on screen at once.
    pub(crate) fn launcher_visible(&self) -> usize {
        7
    }

    /// Highest allowed scroll offset for the current hit list.
    pub(crate) fn launcher_max_scroll(&self) -> usize {
        self.launcher_hits.len().saturating_sub(self.launcher_visible())
    }

    /// Wheel / two-finger trackpad scroll MOVES THE SELECTION through the hit
    /// list (rofi-style) — the highlighted row steps up/down and the visible
    /// window auto-scrolls to keep it in view. Returns true when it moved.
    pub(crate) fn launcher_select_by(&mut self, delta: i32) -> bool {
        self.ensure_launcher_hits();
        let n = self.launcher_hits.len();
        if n == 0 || delta == 0 {
            return false;
        }
        let old = self.sel;
        let vis = self.launcher_visible();
        self.sel = (self.sel as i32 + delta).clamp(0, n as i32 - 1) as usize;
        // keep the selection inside the visible window (same as arrow keys)
        if self.sel < self.launcher_scroll {
            self.launcher_scroll = self.sel;
        }
        let max = self.launcher_max_scroll();
        if self.sel > self.launcher_scroll + vis - 1 {
            self.launcher_scroll = (self.sel + 1 - vis).min(max);
        }
        old != self.sel
    }

    /// Wheel / two-finger scroll moves the clipboard list's highlighted row
    /// (same feel as the launcher). Returns true when it moved.
    pub(crate) fn clip_select_by(&mut self, delta: i32) -> bool {
        let is_img = self.mode == Mode::ClipboardImages;
        let n = if is_img { self.clip_images.len() } else { self.clip_text.len() };
        if n == 0 || delta == 0 {
            return false;
        }
        let old = self.clip_sel;
        self.clip_sel = (self.clip_sel as i32 + delta).clamp(0, n as i32 - 1) as usize;
        old != self.clip_sel
    }

    // ---- themes panel + $states2 sync ------------------------------------

    /// Re-read the theme cards mirror ($states2/themes) into cards.
    pub(crate) fn refresh_themes(&mut self) {
        let mut cards = crate::themes::load();
        // Sort by name so a dual theme sits inline with its family (e.g.
        // `dracula` right next to `dracula_dark`/`dracula_light`) instead of
        // the file order which buries the dual cards at the very end — those
        // were otherwise unreachable without scrolling past 100+ singles.
        cards.sort_by(|a, b| a.name.cmp(&b.name));
        self.themes = cards;
        self.cur_theme = crate::themes::cur_theme();
        self.themes_scroll = self.themes_scroll.min(self.themes_max_scroll());
    }

    /// Re-read the keybind list mirror ($states2/keybinds_cache) into rows.
    pub(crate) fn refresh_keybinds(&mut self) {
        self.keybinds = crate::themes::load_keybinds();
        self.keybinds_scroll = self.keybinds_scroll.min(self.keybinds_max_scroll());
    }

    /// Mirror $states2/m_dummy (`d` = dark / `l` = light) into `dark_mode`.
    /// Returns true when the state changed.
    pub(crate) fn sync_dark_state(&mut self) -> bool {
        let dark = crate::themes::is_dark_mode();
        let changed = dark != self.dark_mode;
        self.dark_mode = dark;
        changed
    }

    /// Re-read every color from $states2/shell_vars. Keys that are missing
    /// keep their current value (config [colors] / built-in fallbacks).
    /// Returns true when anything changed.
    pub(crate) fn reload_states_colors(&mut self) -> bool {
        let m = crate::themes::read_shell_vars();
        if m.is_empty() {
            return false;
        }
        let get = |k: &str| m.get(k).copied();
        let mut changed = false;
        let mut set = |cur: &mut u32, v: Option<u32>| {
            if let Some(v) = v {
                if v != *cur {
                    *cur = v;
                    changed = true;
                }
            }
        };
        set(&mut self.sv_fg, get("fg"));
        set(&mut self.sv_fg2, get("fg2"));
        set(&mut self.sv_fg3, get("fg3"));
        set(&mut self.sv_bg, get("bg"));
        set(&mut self.sv_acc, get("acc"));
        set(&mut self.sv_sfg, get("sfg"));
        set(&mut self.sv_hover, get("hover"));
        set(&mut self.sv_focus, get("focus"));
        set(&mut self.sv_disabled, get("disabled"));
        for (i, name) in ["series1", "series2", "series3", "series4", "series5", "series6"]
            .iter()
            .enumerate()
        {
            if let Some(v) = get(name) {
                if v != self.sv_series[i] {
                    self.sv_series[i] = v;
                    changed = true;
                }
            }
        }
        changed
    }

    /// Re-read the per-channel accent-toggles / alpha flags from `$states`
    /// for the current channel. These drive the Settings → Appearance toggles;
    /// theme_body is the consumer that applies them. Called on boot and every
    /// `colors reload` so the UI tracks the live flags.
    pub(crate) fn reload_accent_flags(&mut self) {
        let ch = crate::vars::read_channel();
        let dir = crate::vars::states_dir();
        let read_u32 = |name: &str, def: u32| -> u32 {
            std::fs::read_to_string(dir.join(format!("{name}_{ch}")))
                .ok()
                .and_then(|s| s.trim().parse::<u32>().ok())
                .unwrap_or(def)
        };
        let read_bool = |name: &str, def: bool| -> bool { read_u32(name, def as u32) != 0 };
        let read_byte = |name: &str, def: u8| -> u8 {
            std::fs::read_to_string(dir.join(format!("{name}_{ch}")))
                .ok()
                .and_then(|s| u8::from_str_radix(s.trim(), 16).ok())
                .unwrap_or(def)
        };
        self.acc_flag_start_icon = read_bool("custom_acc_start_icon", false);
        self.acc_flag_app_border = read_bool("custom_acc_app_border", false);
        self.acc_flag_hyprland = read_bool("custom_acc_hyprland", false);
        self.acc_flag_gtk = read_bool("custom_acc_gtk", true);
        self.acc_flag_normal_app = read_bool("custom_acc_normal_app", true);
        self.acc_scrim = read_bool("acc_scrim", false);
        self.start_icon_tone = read_u32("start_icon_tone", 0).min(900);
        self.bg_alpha = read_byte("bg_alpha", 0xff);
        self.acc_alpha = read_byte("acc_alpha", 0xe5);
        self.scrim_alpha = read_byte("scrim_alpha", 0x4c);
        self.border_alpha = read_byte("border_alpha", 0xbf);
    }

    /// Write `$states/<base>_<ch>` for the current channel (used by the
    /// Settings → Appearance toggles/alpha sliders). The "Apply changes"
    /// button then runs `theme_main restore` to apply them to the live theme.
    pub(crate) fn write_state_flag(&self, base: &str, val: &str) {
        let ch = crate::vars::read_channel();
        let path = crate::vars::states_dir().join(format!("{base}_{ch}"));
        let _ = std::fs::write(&path, format!("{val}\n"));
    }

    /// Toggle a Settings → Appearance accent flag, persist the new value to
    /// `$states/custom_acc_<flag>_<ch>`, and return it for the caller to cache.
    /// The "Apply changes" button runs `theme_main restore` (no `acc_changed`).
    pub(crate) fn toggle_acc_flag(&self, flag: &str, cur: bool) -> bool {
        let new = !cur;
        self.write_state_flag(&format!("custom_acc_{flag}"), if new { "1" } else { "0" });
        new
    }

    /// Re-read the channel flag (`$states2/s`) and adopt that channel's
    /// master shell config (`$states/shell_{n,d,l}`) wholesale — pill toggles,
    /// geometry and bar anchor/floating/reserve. Called on every `colors
    /// reload` IPC. Returns true when anything changed (switched channel, that
    /// channel's file was edited, or the bar geometry changed).
    pub(crate) fn sync_per_state_config(&mut self) -> bool {
        let new_state = crate::vars::read_channel();
        let mut changed = false;
        if new_state != self.ui_state {
            self.ui_state = new_state;
            self.per_state_config = crate::vars::per_state_shell_path(new_state);
            changed = true;
        }
        let cfg = crate::config::Config::load(&self.per_state_config);
        // adopt the channel's pill layout (toggles + geometry) from its master
        self.pill_clock = cfg.show_clock();
        self.pill_battery = cfg.show_battery();
        self.pill_battery_pct = cfg.show_battery_pct();
        self.pill_ws = cfg.show_ws();
        self.pill_ws_long = cfg.show_ws_long();
        self.pill_ws_short = cfg.show_ws_short();
        self.pill_wifi = cfg.show_wifi();
        self.pill_bluetooth = cfg.show_bluetooth();
        self.pill_volume = cfg.show_volume();
        self.pill_wallpaper = cfg.show_wallpaper();
        self.pill_themes = cfg.show_themes();
        self.pill_branding = cfg.show_brand();
        self.pill_tray = cfg.show_tray();
        self.pill_viz = cfg.show_viz();
        self.pill_settings = cfg.settings_on_pill();
        self.pill_notif = cfg.show_notif();
        self.pill_user = cfg.show_user();
        self.pill_watts = cfg.show_watts();
        self.pill_brightness = cfg.show_brightness();
        self.expand_on_hover = cfg.expand_on_hover();
        self.cc_as_dashboard = cfg.cc_as_dashboard();
        self.pill_margin_x = cfg.pill_margin_x();
        self.pill_alpha = cfg.pill_alpha();
        self.dash_card_alpha = cfg.dash_card_alpha();
        self.dash_scroll_dir = cfg.dash_scroll_dir();
        self.allow_over_100 = cfg.allow_over_100();
        self.max_vol = cfg.max_vol();
        // bar anchor / floating / reserve drive the layer geometry (applied by
        // the IPC caller when changed) — a channel switch or an edited master
        // file must not leave the bar hanging at the old anchor/offset.
        if self.bar_edge != cfg.bar_edge() {
            self.bar_edge = cfg.bar_edge();
            changed = true;
        }
        if self.bar_floating != cfg.floating() {
            self.bar_floating = cfg.floating();
            changed = true;
        }
        if self.bar_reserve != cfg.reserve() {
            self.bar_reserve = cfg.reserve();
            changed = true;
        }
        self.cfg = cfg;
        // each channel keeps its OWN dashboard — reload the remembered cards
        // (sizes + positions + parked tray) when the channel switches
        if changed {
            let (layout, tray) = Shell::load_dash_layout();
            self.dash_layout = layout;
            self.tray_cards = tray;
            Self::apply_persisted_dashboard(self);
        }
        // the declarative [dashboard] spec is authoritative when it declares
        // cards — reconcile grid order + parked tray from it on every sync
        // (covers channel switches AND same-channel hot reloads)
        if !self.cfg.dashboard.cards.run.is_empty() {
            Self::apply_dashboard_spec_layout(self);
        }
        // A same-channel `colors reload` can still change size-affecting
        // master values (bar height, expanded width, grid scale, pill layout).
        // Always re-pack + re-fit — both are no-ops when nothing actually
        // changed size — so the surface follows the master instead of staying
        // stuck at the old dimensions until a hover happens to re-morph it.
        self.refresh_pack();
        self.refresh_size();
        changed
    }

    // ---- theme card grid metrics (adaptive, mirrors the wallpaper grid) ----
    // Sizes derive from the live panel size so the grid fills the surface
    // edge to edge and shows many cards at once (≈3-4 cols × 3+ rows).
    pub(crate) const TH_GAP: f32 = 12.0;
    pub(crate) const TH_Y0: f32 = 96.0;
    /// smallest target card width used to decide how many columns fit
    pub(crate) const TH_W_MIN: f32 = 170.0;
    /// compact card height so up to ~3 rows fit the panel
    pub(crate) const TH_H: f32 = 92.0;

    /// Grid columns for the live panel width — derived so the grid fills the
    /// card edge to edge (aims for ~3-4 columns).
    pub(crate) fn th_cols(&self) -> usize {
        let w = self.target_size().0 - 32.0;
        let cols = (((w + Self::TH_GAP) / (Self::TH_W_MIN + Self::TH_GAP)).floor() as usize).max(1);
        cols.min(4)
    }

    /// Card width that fills the panel given the column count.
    pub(crate) fn th_card_w(&self) -> f32 {
        let w = self.target_size().0 - 32.0;
        let cols = self.th_cols() as f32;
        (w - (cols - 1.0) * Self::TH_GAP) / cols
    }

    /// Indices into `self.themes` matching the current search query
    /// (case-insensitive substring on the theme name). Empty query = all themes.
    pub(crate) fn filtered_themes(&self) -> Vec<usize> {
        let q = self.themes_query.trim().to_lowercase();
        self.themes.iter().enumerate()
            .filter(|(_, c)| q.is_empty() || c.name.to_lowercase().contains(&q))
            .map(|(i, _)| i)
            .collect()
    }

    pub(crate) fn th_rows(&self) -> usize {
        let h = self.target_size().1 - Self::TH_Y0 - 14.0;
        (((h + Self::TH_GAP) / (Self::TH_H + Self::TH_GAP)).floor() as usize).max(1)
    }

    pub(crate) fn themes_visible(&self) -> usize {
        self.th_cols() * self.th_rows()
    }

    pub(crate) fn themes_max_scroll(&self) -> usize {
        // max *item* index the grid can start at so the LAST theme is reachable
        // (row-aligned stepping in themes_scroll_by keeps rows intact)
        self.filtered_themes().len().saturating_sub(self.themes_visible())
    }

    /// Wheel scroll for the theme grid — one row of cards per notch.
    pub(crate) fn themes_scroll_by(&mut self, notches: i32) -> bool {
        let old = self.themes_scroll;
        let cols = self.th_cols() as i32;
        let next = self.themes_scroll as i32 + notches * cols;
        self.themes_scroll = next.clamp(0, self.themes_max_scroll() as i32) as usize;
        old != self.themes_scroll
    }

    /// Scrollbar track rect for the theme panel: (x, y, w, h). It spans
    /// exactly the visible grid height. 8 px wide so the dragger is an
    /// easy mouse target, not a 4 px hairline.
    pub(crate) fn th_track(&self) -> (f32, f32, f32, f32) {
        let track_h = self.th_rows() as f32 * (Self::TH_H + Self::TH_GAP) - Self::TH_GAP;
        (self.target_size().0 - 12.0, Self::TH_Y0, 8.0, track_h)
    }

    /// Theme scrollbar thumb rect at the current scroll: (y offset into the
    /// track, h). Min height is bigger than the wheel-only 16 px so the head
    /// is grabbable with a mouse.
    pub(crate) fn th_thumb(&self, track_h: f32) -> (f32, f32) {
        let n = self.filtered_themes().len().max(1) as f32;
        let total_rows = (n / self.th_cols() as f32).ceil().max(1.0);
        let thumb_h = (track_h / total_rows * self.th_rows() as f32).max(self.scale.s(20.0));
        let scroll_row = self.themes_scroll as f32 / self.th_cols() as f32;
        let t = (scroll_row / (total_rows - self.th_rows() as f32).max(1.0)).clamp(0.0, 1.0);
        ((track_h - thumb_h) * t, thumb_h)
    }

    /// Last valid grid-row index a drag can scroll to.
    pub(crate) fn max_th_row(&self) -> i32 {
        let total_rows = (self.filtered_themes().len() + self.th_cols() - 1) / self.th_cols();
        (total_rows as i32 - self.th_rows() as i32).max(0)
    }

    /// Press on the theme-grid scrollbar strip: begin a grab-drag. A press on
    /// the thumb keeps its in-thumb offset; a press on the empty track first
    /// page-jumps so the thumb centers under the cursor. Returning true also
    /// CONSUMES the press — without it the hit-test falls through to
    /// `click()` whose unhandled key collapses the panel (the bug where
    /// grabbing the scrollbar "collapsed" the theme browser).
    pub(crate) fn begin_th_sb_drag(&mut self, x: f32, y: f32) -> bool {
        if self.mode != Mode::Themes || self.filtered_themes().len() <= self.themes_visible() {
            return false;
        }
        let (tx, ty, tw, th) = self.th_track();
        if x < tx - 8.0 || x > tx + tw + 8.0 || y < ty || y > ty + th {
            return false;
        }
        let (rel, thumb_h) = self.th_thumb(th);
        self.th_sb_grab = if y >= ty + rel && y <= ty + rel + thumb_h {
            // thumb grab: remember where inside the thumb it was caught
            (y - ty - rel).clamp(0.0, thumb_h)
        } else {
            // track jump: center the thumb under the pointer, then drag
            let usable = (th - thumb_h).max(1.0);
            let t = ((y - ty - thumb_h / 2.0) / usable).clamp(0.0, 1.0);
            let row = ((t * self.max_th_row() as f32).round() as i32).clamp(0, self.max_th_row());
            self.themes_scroll = (row as usize * self.th_cols()).min(self.themes_max_scroll());
            thumb_h / 2.0
        };
        self.th_sb_drag = true;
        true
    }

    /// Move a theme-scrollbar drag: map pointer y through the track into a
    /// whole-row scroll offset. Returns true when the offset moved.
    pub(crate) fn drag_th_sb(&mut self, y: f32) -> bool {
        if !self.th_sb_drag || self.max_th_row() == 0 {
            return false;
        }
        let (_, ty, _, th) = self.th_track();
        let (_, thumb_h) = self.th_thumb(th);
        let usable = (th - thumb_h).max(1.0);
        let t = ((y - ty - self.th_sb_grab) / usable).clamp(0.0, 1.0);
        let row = ((t * self.max_th_row() as f32).round() as i32).clamp(0, self.max_th_row());
        let next = (row as usize * self.th_cols()).min(self.themes_max_scroll());
        if next != self.themes_scroll {
            self.themes_scroll = next;
            true
        } else {
            false
        }
    }

    /// End a theme-scrollbar drag (pointer released).
    pub(crate) fn end_th_sb_drag(&mut self) {
        self.th_sb_drag = false;
    }
}

// ================================================================ state
impl Shell {
    pub(crate) fn scroll_workspace(&mut self, delta: f32) {
        if self.ws_count == 0 || delta == 0.0 {
            return;
        }
        let steps = delta as i64;
        let next = ((self.ws_active as i64 + steps).rem_euclid(self.ws_count as i64)) as usize;
        if next == self.ws_active {
            return;
        }
        self.ws_prev = self.ws_active;
        self.ws_active = next;
        self.pending_ws_dispatch = Some(next + 1);
    }

    pub(crate) fn push_tray(&mut self, item: TrayItem) {
        self.tray.insert(0, item);
    }

    pub(crate) fn remove_tray(&mut self, id: &str) {
        self.tray.retain(|t| t.id != id);
    }

    pub(crate) fn collapsed_w(&self) -> f32 {
        let g = PILL_GAP;
        let mut left_w = 0.0;
        let mut n_left = 0usize;
        let cw = self.pill_item_width(PillItem::Clock);
        if cw > 0.0 {
            left_w += cw;
            n_left += 1;
        }
        left_w += self.date_str("%a %d").chars().count() as f32 * 14.0 * 0.62;
        n_left += 1;
        let bw = self.pill_item_width(PillItem::Battery);
        if bw > 0.0 {
            left_w += bw;
            n_left += 1;
        }
        let mut center_w = 0.0;
        let mut n_center = 0usize;
        for it in self.pill_order.iter().copied() {
            if it == PillItem::Clock || it == PillItem::Battery {
                continue;
            }
            let iw = self.pill_item_width(it);
            if iw > 0.0 {
                center_w += iw;
                n_center += 1;
            }
        }
        let tray_n = if self.pill_tray { self.tray.len().min(6) } else { 0 };
        let mut right_w = 0.0;
        let mut n_right = 0usize;
        if self.pill_notif {
            right_w += 22.0;
            n_right += 1;
        }
        if self.pill_brightness {
            right_w += 22.0;
            n_right += 1;
        }
        if tray_n > 0 {
            right_w += tray_n as f32 * 22.0;
            n_right += 1;
        }
        let w = self.pill_margin_x
            + left_w + n_left as f32 * g
            + g
            + center_w + n_center as f32 * g
            + (n_right.saturating_sub(1)) as f32 * g
            + right_w
            + self.pill_margin_x;
        w.max(1.0)
    }

    pub(crate) fn media_pos_now(&self) -> i64 {
        if let Some(f) = self.media_seek {
            return (f * self.media_len.max(1) as f32) as i64;
        }
        let pos = if self.media_playing {
            self.media_pos + self.media_pos_at.elapsed().as_micros() as i64
        } else {
            self.media_pos
        };
        pos.max(0).min(self.media_len.max(1))
    }

    pub(crate) fn media_frac(&self) -> f32 {
        if self.media_len <= 0 {
            return 0.0;
        }
        (self.media_pos_now() as f32 / self.media_len as f32).clamp(0.0, 1.0)
    }

    pub(crate) fn finish_seek(&mut self) -> Option<i64> {
        let Some(frac) = self.media_seek else {
            return None;
        };
        self.media_seek = None;
        let len = self.media_len.max(1);
        let target = (frac * len as f32) as i64;
        Some(target - self.media_pos_now())
    }

    pub(crate) fn target_size(&self) -> (f32, f32) {
        if self.mode == Mode::Lock {
            return self.monitor;
        }
        // The resting pill width follows its ACTIVE chips: the estimator in
        // `collapsed_w()` mirrors `layout_collapsed` item-for-item (clock /
        // date / battery / the center items / bell / brightness / tray), so
        // the bar auto-sizes to what it actually draws. A manual width
        // (`collapsed_width > 0` in config) still wins and pins the pill.
        if self.mode == Mode::Collapsed {
            let w = if self.cfg.bar.collapsed_width > 0 {
                self.cfg.collapsed_w()
            } else {
                self.collapsed_w()
            };
            return (w, self.cfg.bar_h());
        }
        let (tw, th) = self.mode.size(&self.cfg);
        // Dashboard EDIT MODE stacks chrome trays above the card grid; grow the
        // Expanded surface so the whole board (all rows × cols) fits below them —
        // a wide/tall grid expands the surface up to the monitor size; bigger
        // grids still scroll. When the dashboard is OFF — or enabled but with
        // ZERO cards anywhere — the surface collapses to banner-only, exactly
        // as wide/high as the banner chrome (edit) or the strip band shows.
        if self.mode == Mode::Expanded {
            if !self.dash_enabled || self.dash_area_collapsed() {
                // banner-only dashboard: exactly the strip's natural width
                // (or the manual width slider when the dashboard is off) —
                // never the full expanded width.
                return (
                    self.dash_canvas_w(),
                    self.dash_canvas_h().min(self.dash_view_h()),
                );
            }
            let need_w = self.dash_canvas_w();
            let need_h = self.dash_canvas_h();
            if self.dash_edit {
                return (
                    tw.max(need_w).min(self.dash_view_w()),
                    th.max(need_h).min(self.dash_view_h()),
                );
            }
            // normal mode: the surface follows the dashboard GRID — fewer
            // columns ⇒ a shorter panel, fewer rows ⇒ a shorter height.
            // Never floats wider/taller than the monitor; bigger boards
            // still scroll inside the viewport instead.
            return (
                need_w.min(self.dash_view_w()),
                need_h.min(self.dash_view_h()),
            );
        }
        (tw, th)
    }

    pub(crate) fn refresh_size(&mut self) {
        let (tw, th) = self.target_size();
        if (self.cur_w - tw).abs() < 0.5 && (self.cur_h - th).abs() < 0.5 {
            return;
        }
        self.start_morph(tw, th);
    }

    pub(crate) fn start_morph(&mut self, tw: f32, th: f32) {
        self.anim = Some(Anim {
            from_w: self.cur_w,
            from_h: self.cur_h,
            to_w: tw,
            to_h: th,
            start: Instant::now(),
            dur: 0.25,
        });
    }

    pub(crate) fn tick(&mut self, now: Instant) -> Option<(f32, f32)> {
        let (from_w, from_h, to_w, to_h, start, dur) = match self.anim.as_ref() {
            Some(a) => (a.from_w, a.from_h, a.to_w, a.to_h, a.start, a.dur),
            None => return None,
        };
        let el = now.duration_since(start).as_secs_f32();
        let t = ease_out_cubic((el / dur.max(0.001)).clamp(0.0, 1.0));
        self.cur_w = from_w + (to_w - from_w) * t;
        self.cur_h = from_h + (to_h - from_h) * t;
        if el >= dur {
            self.cur_w = to_w;
            self.cur_h = to_h;
            if self.size_pending {
                return None;
            }
            self.anim = None;
            return Some((to_w, to_h));
        }
        if self.size_pending {
            return None;
        }
        Some((self.cur_w, self.cur_h))
    }

    pub(crate) fn note_committed(&mut self, w: f32, h: f32) {
        self.last_committed = (w, h);
        self.cur_w = w;
        self.cur_h = h;
    }

    pub(crate) fn at_target(&self) -> bool {
        if self.anim.is_some() {
            return false;
        }
        let (tw, th) = self.target_size();
        (self.cur_w - tw).abs() < 0.5 && (self.cur_h - th).abs() < 0.5
    }

    pub(crate) fn morph_progress(&self) -> f32 {
        match self.anim.as_ref() {
            Some(a) => ease_out_cubic((a.start.elapsed().as_secs_f32() / a.dur.max(0.001)).clamp(0.0, 1.0)),
            None => 1.0,
        }
    }

    pub(crate) fn pill_item_width(&self, item: PillItem) -> f32 {
        match item {
            PillItem::Workspaces => {
                if !self.pill_ws {
                    return 0.0;
                }
                self.active_ws_w()
            }
            PillItem::WorkspacesLong => {
                if !self.pill_ws_long {
                    return 0.0;
                }
                // five 18 px chips + four 2 px gaps, plus the active-number
                // chip appended on the right when active workspace is >5
                let base = 5.0 * 18.0 + 4.0 * 2.0;
                if self.ws_active + 1 > 5 { base + 2.0 + self.active_ws_w() } else { base }
            }
            PillItem::WorkspacesShort => {
                if !self.pill_ws_short {
                    return 0.0;
                }
                self.active_ws_w()
            }
            PillItem::Wifi => self.chip_w(PillItem::Wifi),
            PillItem::Bluetooth => self.chip_w(PillItem::Bluetooth),
            PillItem::Volume => self.chip_w(PillItem::Volume),
            PillItem::Wallpaper => self.chip_w(PillItem::Wallpaper),
            PillItem::Themes => self.chip_w(PillItem::Themes),
            PillItem::Clock => {
                if !self.pill_clock || self.clock.is_empty() {
                    return 0.0;
                }
                8.0 + self.clock.chars().count() as f32 * 14.0 * 0.62
            }
            PillItem::Battery => {
                if !self.pill_battery || self.battery < 0 {
                    return 0.0;
                }
                let icon_w = ui::battery_icon_w(13.0);
                let pct_w = if self.pill_battery_pct {
                    format!("{}%", self.battery).len() as f32 * 14.0 * 0.62 + 3.0
                } else {
                    0.0
                };
                icon_w + pct_w
            }
            PillItem::Watts => {
                if !self.pill_watts {
                    return 0.0;
                }
                let txt = format!("{:.1}W", self.watts_shown);
                14.0 + txt.chars().count() as f32 * 14.0 * 0.62
            }
            PillItem::Visualizer => {
                if self.pill_viz {
                    8.0 * 3.0
                } else {
                    0.0
                }
            }
            PillItem::Settings => {
                if self.pill_settings {
                    22.0
                } else {
                    0.0
                }
            }
            PillItem::Bell | PillItem::Tray => 0.0,
            PillItem::User => {
                if self.pill_user {
                    self.username.chars().count() as f32 * 14.0 * 0.62
                } else {
                    0.0
                }
            }
            PillItem::Branding => {
                if !self.pill_branding {
                    return 0.0;
                }
                let chars = self.brand_glyph.chars().count().max(1) as f32;
                (chars * 16.0 * 0.62 + self.scale.s(4.0)).max(18.0)
            }
        }
    }

    /// Fixed-width icon chip (22 px) for the resting pill. Returns 0 when the
    /// chip's toggle is off so the pill auto-sizes to what it draws.
    fn chip_w(&self, item: PillItem) -> f32 {
        let on = match item {
            PillItem::Wifi => self.pill_wifi,
            PillItem::Bluetooth => self.pill_bluetooth,
            PillItem::Volume => self.pill_volume,
            PillItem::Wallpaper => self.pill_wallpaper,
            PillItem::Themes => self.pill_themes,
            PillItem::Branding => self.pill_branding,
            _ => false,
        };
        if on { 22.0 } else { 0.0 }
    }

    /// Width of the "active workspace number" chip: just enough for the
    /// current workspace id (fits 1 or 2 digits, e.g. 8 / 9 / 10).
    fn active_ws_w(&self) -> f32 {
        if self.ws_active + 1 >= 10 { 26.0 } else { 18.0 }
    }

    pub(crate) fn date_str(&self, fmt: &str) -> String {
        unsafe {
            let now = libc::time(std::ptr::null_mut());
            let mut tm: libc::tm = std::mem::zeroed();
            libc::localtime_r(&now, &mut tm);
            let fmts = fmt.bytes().chain(std::iter::once(0)).collect::<Vec<u8>>();
            let mut buf = [0u8; 64];
            let n = libc::strftime(buf.as_mut_ptr() as *mut libc::c_char, buf.len(), fmts.as_ptr() as *const libc::c_char, &tm);
            String::from_utf8_lossy(&buf[..n]).into_owned()
        }
    }

    pub(crate) fn calendar_info(&self) -> (i32, i32, i32, String) {
        unsafe {
            let now = libc::time(std::ptr::null_mut());
            let mut today_tm: libc::tm = std::mem::zeroed();
            libc::localtime_r(&now, &mut today_tm);
            let abs = today_tm.tm_year as i64 * 12 + today_tm.tm_mon as i64 + self.cal_offset as i64;
            let (y, m) = (abs.div_euclid(12), abs.rem_euclid(12));
            let mut first: libc::tm = std::mem::zeroed();
            first.tm_year = y as i32;
            first.tm_mon = m as i32;
            first.tm_mday = 1;
            first.tm_hour = 12;
            first.tm_isdst = -1;
            let _ = libc::mktime(&mut first);
            let first_wday = first.tm_wday as i32;
            let mut last: libc::tm = std::mem::zeroed();
            last.tm_year = y as i32;
            last.tm_mon = m as i32;
            last.tm_mday = 0;
            last.tm_hour = 12;
            last.tm_isdst = -1;
            let _ = libc::mktime(&mut last);
            let days = last.tm_mday as i32;
            let mut buf = [0u8; 64];
            let n = libc::strftime(buf.as_mut_ptr() as *mut libc::c_char, buf.len(), c"%B %Y".as_ptr(), &first);
            let month = String::from_utf8_lossy(&buf[..n]).into_owned();
            (first_wday, days, today_tm.tm_mday as i32, month)
        }
    }

    pub(crate) fn load_custom_accs(&mut self) {
        self.custom_accs = std::fs::read_to_string(crate::vars::states2_dir().join("custom_acc.json"))
            .ok()
            .and_then(|s| serde_json::from_str::<Vec<CustomAcc>>(&s).ok())
            .unwrap_or_default();
    }

    pub(crate) fn current_acc_source(&self) -> String {
        let ch = crate::vars::read_channel();
        std::fs::read_to_string(crate::vars::states_dir().join(format!("acc_source_{ch}")))
            .map(|s| s.trim().to_string())
            .unwrap_or_default()
    }

    pub(crate) fn current_acc_name(&self) -> String {
        if self.current_acc_source() != "c" {
            return String::new();
        }
        let ch = crate::vars::read_channel();
        std::fs::read_to_string(crate::vars::states_dir().join(format!("custom_acc_{ch}")))
            .map(|s| s.trim().to_string())
            .unwrap_or_default()
    }

    pub(crate) fn accent_display(&self, acc: &CustomAcc) -> u32 {
        let s = acc.hex.trim().trim_start_matches('#');
        let v = u32::from_str_radix(&s[..s.len().min(6)], 16).unwrap_or(0xff00ff);
        (v << 8) | 0xff
    }

    pub(crate) fn held_power_action(&self, key: u32) -> String {
        let k = if (POWER_CARD_KEY_BASE..=POWER_CARD_KEY_MAX).contains(&key) {
            key - POWER_CARD_KEY_BASE + 1
        } else {
            key
        };
        let action = match k {
            1 => "lock",
            2 => "logout",
            3 => "sleep",
            4 => "restart",
            5 => "shutdown",
            _ => "",
        };
        action.to_string()
    }

    pub(crate) fn power_hold_fill(&self) -> f32 {
        match self.power_hold {
            Some(_) => (self.power_hold_start.elapsed().as_secs_f32() / self.power_hold_delay.max(0.001)).clamp(0.0, 1.0),
            None => 0.0,
        }
    }

    pub(crate) fn cancel_power_hold(&mut self) -> bool {
        let active = self.power_hold.is_some();
        self.power_hold = None;
        active
    }

    pub(crate) fn cancel_clear_all_hold(&mut self) -> bool {
        let active = self.dash_clear_all_hold.is_some();
        self.dash_clear_all_hold = None;
        active
    }

    pub(crate) fn cancel_banner_clear_hold(&mut self) -> bool {
        let active = self.banner_clear_all_hold.is_some();
        self.banner_clear_all_hold = None;
        active
    }

    pub(crate) fn check_banner_clear_hold(&mut self) -> bool {
        match self.banner_clear_all_hold.take() {
            Some(start) => start.elapsed().as_secs_f32() >= 3.0,
            None => false,
        }
    }

    pub(crate) fn check_clear_all_hold(&mut self) -> bool {
        match self.dash_clear_all_hold.take() {
            Some(start) => start.elapsed().as_secs_f32() >= 3.0,
            None => false,
        }
    }

    /// Move every board card into the parked tray, preserving cards already
    /// parked — a card is never in both places.
    pub(crate) fn park_all_cards(&mut self) {
        for l in &self.dash_layout {
            if !self.tray_cards.contains(&l.card) {
                self.tray_cards.push(l.card);
            }
        }
    }

    /// Clear-all (3s hold): empty the board and reset the edit grid — cols
    /// and rows drop to auto (content-fit).
    pub(crate) fn clear_all_cards(&mut self) {
        self.park_all_cards();
        self.dash_layout.clear();
        self.dash_clear_all_hold = None;
        self.edit_grid_cols = 0;
        self.edit_grid_rows = 0;
        self.refresh_pack();
        self.save_config();
        self.refresh_size();
    }

    /// Banner strip "clear all" (3s hold): empty the whole strip back into the
    /// parked tray — every chip and separator leaves `banner_order` (zone
    /// markers and fillers stay so the L/C/R sections survive).
    pub(crate) fn clear_all_banner_cards(&mut self) {
        self.banner_order.retain(|t| !matches!(t, BannerToken::Chip(_) | BannerToken::Sep(_)));
        self.banner_clear_all_hold = None;
        self.save_config();
    }

    pub(crate) fn take_app_shortcut_pick(&mut self, id: &str) -> bool {
        match self.app_shortcut_pick.take() {
            Some(slot) if slot < self.app_shortcuts.len() => {
                self.app_shortcuts[slot] = id.to_string();
                self.save_config();
                true
            }
            _ => false,
        }
    }

    pub(crate) fn run_power(&mut self, action: String) {
        let (cmd, args): (&str, &[&str]) = match action.as_str() {
            "lock" => ("loginctl", &["lock-session"]),
            "logout" => ("hyprctl", &["dispatch", "exit"]),
            "sleep" => ("systemctl", &["suspend"]),
            "restart" => ("systemctl", &["reboot"]),
            "shutdown" => ("systemctl", &["poweroff"]),
            _ => return,
        };
        let _ = std::process::Command::new(cmd)
            .args(args)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn();
    }
}
