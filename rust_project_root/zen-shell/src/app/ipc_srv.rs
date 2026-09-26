//! //! Unix-socket IPC server + Hyprland ground-truth queries.

use super::*;

impl App {
        pub(super) fn install_ipc(&mut self, handle: &LoopHandle<'static, App>) -> Result<(), String> {
        use smithay_client_toolkit::reexports::calloop::generic::Generic;
        use smithay_client_toolkit::reexports::calloop::{Interest, Mode, PostAction};
        let path = ipc::socket_path();
        // We hold the single-instance flock, so any existing socket file is
        // a stale leftover from a crashed instance — safe to reclaim.
        if path.exists() {
            let _ = std::fs::remove_file(&path);
        }
        let listener = std::os::unix::net::UnixListener::bind(&path)
            .map_err(|e| format!("bind {}: {e}", path.display()))?;
        listener.set_nonblocking(true).ok();
        // register a clone; the original stays owned by App (fd must outlive
        // the source)
        let reg = listener.try_clone().map_err(|e| format!("ipc clone: {e}"))?;
        let src = Generic::new(reg, Interest::READ, Mode::Level);
        let _ = handle.insert_source(src, |_, _, app: &mut App| {
            app.ipc_accept();
            Ok(PostAction::Continue)
        });
        self._ipc_listener = Some(listener);
        Ok(())
    }

        pub(super) fn ipc_accept(&mut self) {
        use std::io::{Read, Write};
        // owned clone so `self` can be mutably borrowed while accepting
        let Some(listener) = self._ipc_listener.as_ref().and_then(|l| l.try_clone().ok()) else { return };
        while let Ok((mut stream, _)) = listener.accept() {
            stream.set_nonblocking(true).ok();
            let mut buf = [0u8; 512];
            // A client that connected microseconds ago may not have its command
            // bytes queued yet — poll briefly instead of dropping the
            // connection (a dropped command reads as "reload did nothing").
            let n = {
                let mut retries = 40u32;
                loop {
                    match stream.read(&mut buf) {
                        Ok(0) => break 0,
                        Ok(n) => break n,
                        Err(_) if retries > 0 => {
                            retries -= 1;
                            std::thread::sleep(std::time::Duration::from_micros(250));
                        }
                        Err(_) => break 0,
                    }
                }
            };
            if n == 0 {
                continue; // nothing readable — client vanished before writing
            }
            let line = String::from_utf8_lossy(&buf[..n]).trim().to_string();
            if std::env::var("ZEN_TRACE").is_ok() {
                eprintln!("zen: ipc << {line}");
            }
            let reply = match ipc::parse(&line) {
                // MASTER HARD RULE (PROJECT_GUIDE.md #10): an IPC command can
                // NEVER kill the shell. Every handler runs under a panic net —
                // if one panics the process stays alive, logs loudly, and
                // answers `error: internal` instead of aborting the bar.
                Ok(cmd) => match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    self.run_ipc(&cmd)
                })) {
                    Ok(reply) => reply,
                    Err(_) => {
                        eprintln!("zen: ipc command `{line}` panicked — shell kept alive");
                        "error: internal\n".to_string()
                    }
                },
                Err(e) => format!("error: {e}\n"),
            };
            let _ = stream.write_all(reply.as_bytes());
        }
    }

    /// Execute one parsed IPC command. Extracted from `ipc_accept` so every
    /// handler runs inside the catch_unwind panic net — see PROJECT_GUIDE #10.
    fn run_ipc(&mut self, cmd: &ipc::Cmd) -> String {
        use crate::shell::Mode;
        match cmd {
            // `open <target>` toggles: open it, or collapse back to the
            // pill if it's already the current widget (same as `toggle`).
            // The lockscreen is the one exception — a real lock never
            // dismisses itself, so a second `open lock` just stays locked.
            ipc::Cmd::Open(mode) => {
                if self.shell.mode == *mode && *mode != Mode::Lock {
                    self.apply_mode(Mode::Collapsed);
                } else {
                    self.apply_mode(*mode);
                }
                "ok\n".to_string()
            }
            ipc::Cmd::Close => {
                self.apply_mode(Mode::Collapsed);
                "ok\n".to_string()
            }
            // `zen-shell mods` — what $sources/lib is offering right now.
            // Pure reporting: no state, no render, so it is safe to fire from
            // a keybind while a plugin is misbehaving.
            ipc::Cmd::Mods => crate::mods::status(),
            ipc::Cmd::WallpaperSet(path) => {
                let mon = self.hypr.focused_monitor();
                self.wallpaper.set(path, mon.as_deref());
                "ok\n".to_string()
            }
            // like `open`, but `toggle lock` cannot dismiss the lock
            // either (password is the only way out)
            ipc::Cmd::Toggle(mode) => {
                if self.shell.mode == *mode && *mode != Mode::Lock {
                    self.apply_mode(Mode::Collapsed);
                } else {
                    self.apply_mode(*mode);
                }
                "ok\n".to_string()
            }
            // HOT COLOR RELOAD — waybar-style: palette only, nothing
            // else. The entire point of `colors reload` is that it just
            // swaps colors in place, reliably and fast. It must NEVER
            // re-read the channel config, re-apply bar geometry, touch
            // the dashboard layout or start a morph — every one of those
            // belongs to `reload` (full config reload) and would let a
            // routine palette flip disturb a live session ("after a
            // reload the shell breaks and I have to restart it").
            ipc::Cmd::ColorsReload => {
                self.shell.reload_states_colors();
                self.shell.reload_accent_flags();
                self.shell.sync_dark_state();
                self.dirty = true;
                self.maybe_render();
                "ok\n".to_string()
            }
            // re-read the dark/light marker ($states2/m_dummy) so the
            // dashboard toggle chip reflects an external switch
            ipc::Cmd::ModeSync => {
                if self.shell.sync_dark_state() {
                    self.dirty = true;
                    self.maybe_render();
                }
                "ok\n".to_string()
            }
            // hot-reload the entire per-channel shell.toml config: re-reads
            // channel + pill toggles + geometry + colors + the channel's
            // dashboard, and re-applies bar anchor/margin/reserve if changed
            ipc::Cmd::ConfigReload => {
                self.reload_config();
                "ok\n".to_string()
            }
        }
    }

        /// Hot-reload the entire shell.toml config. Re-reads the channel +
        /// per-state config file, syncs all cached pill/geometry/color/
        /// animation fields, re-applies bar anchor/margin/reserve if they
        /// changed, and triggers a full redraw. Fonts and backends are NOT
        /// reloaded (they require a restart) — but everything else (height,
        /// widths, floating, colors, animation, pill toggles, notifications,
        /// wallpaper, weather, grid scale, etc.) picks up immediately.
        pub(super) fn reload_config(&mut self) {
            // 1. channel + per-state config adoption. sync_per_state_config
            //    re-reads $states2/s, reloads that channel's shell
            //    config, syncs pill toggles / geometry / bar anchor /
            //    floating / reserve, RESTORES that channel's remembered
            //    dashboard, and re-packs — it is the one sink for "adopt the
            //    master". Lives here (full config reload), NOT in colors
            //    reload, which must stay a palette-only hot swap.
            let changed = self.shell.sync_per_state_config();

            // 2. cached fields the channel sync doesn't cover
            let cfg = &self.shell.cfg;
            self.shell.pill_order = cfg.pill_order();
            self.shell.banner_order = cfg.banner_order();
            self.shell.wp_cell = cfg.wallpaper_cell();
            self.shell.scale = crate::shell::GridScale(cfg.grid_scale());

            // 3. bar geometry (anchor/margin/reserve) re-apply when the
            //    channel config changed any of them
            if changed {
                self.apply_bar_geometry();
            }

            // 4. color palette + accent flags from $states2
            self.shell.reload_states_colors();
            self.shell.reload_accent_flags();
            self.shell.sync_dark_state();

            // 5. refresh size (collapsed/expanded dimensions may have changed)
            self.shell.refresh_size();
            self.ensure_morph_timer();

            // 6. dirty + render
            self.dirty = true;
            self.maybe_render();

            if std::env::var("ZEN_TRACE").is_ok() {
                eprintln!("zen: config reloaded from {}", self.shell.per_state_config.display());
            }
        }

        pub(super) fn refresh_ws_preview(&mut self) {
        use crate::shell::Mode;
        if self.shell.mode != Mode::Expanded || self.shell.hover_key != 1 {
            return;
        }
        let ws = self.shell.ws_active + 1; // hyprland workspace ids are 1-based
        if self.shell.ws_preview_ws == ws && !self.shell.ws_preview.is_empty() {
            return;
        }
        if let Some(wins) = self.hypr.workspace_windows(ws) {
            self.shell.ws_preview = wins;
            self.shell.ws_preview_ws = ws;
            self.dirty = true;
            self.maybe_render();
        }
    }

        pub(super) fn reconcile_hover(&mut self) {
        use crate::shell::Mode;
        // The Control Center participates in the hover rule only when it is
        // the hover-driven "dashboard" (Settings → Open Control Center).
        let cc_is_dash = self.shell.mode == Mode::ControlCenter && self.shell.cc_as_dashboard;
        if !matches!(self.shell.mode, Mode::Collapsed | Mode::Expanded) && !cc_is_dash {
            return;
        }
        // "Expand on hover" off (Settings → Pill): the cursor never morphs the
        // pill — the dashboard is opened by a click / IPC and dismissed by a
        // background click / Esc, and right-click on the pill opens Settings.
        if !self.shell.expand_on_hover {
            return;
        }
        // NEVER hover-collapse:
        //   • EDIT MODE — the dashboard only leaves editing via Esc /
        //     right-click; a wandering cursor must never close it
        //   • while a drag is live (slider knob / card move-resize) — the
        //     implicit grab owns the surface until release
        //   • during the post-gesture grace window (saturation / Kelvin apply
        //     scripts notify-send; the popup mapping can bounce pointer focus
        //     for a moment and must not read as "cursor left")
        let now = std::time::Instant::now();
        if self.shell.dash_edit
            || (self.pointer_down && self.shell.drag_key.is_some())
            || now < self.shell.drag_grace_until
        {
            return;
        }
        // Disarmed after an explicit dismissal: never expand while the cursor
        // merely rests over the bar — only real movement re-arms expansion.
        // Collapsing stays allowed (a stale-flow check must still shrink).
        if !self.hover_armed {
            self.shell.set_hover(false);
            // may have collapsed a click-opened dashboard — sync the
            // dashboard-gated timers (camera/sensors/viz) off with it
            self.sync_viz_timer();
            self.sync_sensor_timer();
            self.sync_cam_timer();
            return;
        }
        let inside = match self.shell.cursor {
            // events live: exact test against the last-acked surface bounds
            // (identical to the target at rest)
            Some((lx, ly)) => self.pointer_inside(lx as f64, ly as f64),
            // stale event flow: ask the compositor where the cursor is and
            // test the TARGET-projected rect, so a surface growing toward the
            // cursor is never collapsed mid-morph
            None => {
                let Some((cx, cy)) = self.hypr.cursor_pos() else { return };
                self.cursor_over_target(cx, cy)
            }
        };
        if std::env::var("ZEN_TRACE").is_ok() {
            eprintln!(
                "zen: hover rule inside={inside} mode={:?} cursor={:?}",
                self.shell.mode, self.shell.cursor
            );
        }
        self.shell.set_hover(inside);
        // set_hover morphs the pill via Shell::set_mode, which bypasses
        // apply_mode — so hand-sync the dashboard-gated timers here. Without
        // this the camera/sensors never start on a hover-expand (the mirror
        // card sits on "no camera" for the whole session) and never stop on a
        // hover-collapse. All three are idempotent: no-op unless the timer
        // state disagrees with the mode.
        self.sync_viz_timer();
        self.sync_sensor_timer();
        self.sync_cam_timer();
        self.ensure_morph_timer();
    }

        pub(super) fn pointer_inside(&self, x: f64, y: f64) -> bool {
        let (cw, ch) = self.configured;
        x >= 0.0 && y >= 0.0 && x < cw as f64 && y < ch as f64
    }
}
