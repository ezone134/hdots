use std::{
    path::Path,
    sync::{atomic::Ordering, Arc},
    time::{Duration, Instant, SystemTime},
};

use smithay::{
    backend::{
        input::{
            AbsolutePositionEvent, Axis, ButtonState, Event, InputBackend, InputEvent, KeyState,
            KeyboardKeyEvent, PointerAxisEvent, PointerButtonEvent,
        },
        renderer::{
            element::memory::MemoryRenderBuffer,
            gles::GlesTexProgram,
            utils::on_commit_buffer_handler,
        },
    },
    desktop::{
        layer_map_for_output,
        space::SpaceElement,
        LayerSurface as DesktopLayerSurface, PopupKind, PopupManager, Window,
    },
    input::{
        keyboard::{FilterResult, Keycode, Keysym, XkbConfig},
        pointer::{AxisFrame, ButtonEvent, CursorImageStatus, MotionEvent},
        Seat, SeatHandler, SeatState,
    },
    output::Output,
    reexports::{
        calloop::LoopHandle,
        wayland_server::{
            backend::ClientData,
            protocol::{wl_buffer::WlBuffer, wl_output, wl_seat, wl_surface::WlSurface},
            Client, DisplayHandle,
        },
    },
    utils::{Logical, Point, Rectangle, Serial, Size, SERIAL_COUNTER as SCOUNTER},
    wayland::{
        buffer::BufferHandler,
        compositor::{with_states, with_surface_tree_upward, CompositorClientState, CompositorHandler, CompositorState, TraversalAction},
        fractional_scale::{FractionalScaleHandler, FractionalScaleManagerState},
        keyboard_shortcuts_inhibit::{KeyboardShortcutsInhibitHandler, KeyboardShortcutsInhibitState},
        output::{OutputHandler, OutputManagerState},
        presentation::PresentationState,
        selection::{
            data_device::{ClientDndGrabHandler, DataDeviceHandler, DataDeviceState, ServerDndGrabHandler},
            primary_selection::{PrimarySelectionHandler, PrimarySelectionState},
            wlr_data_control::{DataControlHandler, DataControlState},
            SelectionHandler,
        },
        shell::wlr_layer::{
            Layer as WlrLayer, LayerSurface, LayerSurfaceConfigure, WlrLayerShellHandler, WlrLayerShellState,
        },
        shell::xdg::{
            PositionerState, PopupSurface, ToplevelSurface, XdgShellHandler, XdgShellState, XdgToplevelSurfaceData,
        },
        shm::{ShmHandler, ShmState},
        single_pixel_buffer::SinglePixelBufferState,
        viewporter::ViewporterState,
        xdg_activation::{XdgActivationHandler, XdgActivationState, XdgActivationToken, XdgActivationTokenData},
    },
};

use crate::{
    config::{Config, ExecCmd},
    element::WindowElement,
    focus::FocusTarget,
    keybindings::{self, KeyAction},
    overview::OverviewState,
    workspace::{SplitSide, Workspaces},
};

#[derive(Debug, Default)]
pub struct ClientState {
    pub compositor_state: CompositorClientState,
}

impl ClientData for ClientState {
    fn initialized(&self, _client_id: smithay::reexports::wayland_server::backend::ClientId) {}
}

/// The compositor state: owns every global, the seat, and the workspace model.
pub struct ZenWm {
    pub display_handle: DisplayHandle,
    pub event_loop_handle: LoopHandle<'static, ZenWm>,
    pub running: Arc<std::sync::atomic::AtomicBool>,
    pub socket_name: Option<String>,
    pub config: Config,
    pub saturation_program: Option<GlesTexProgram>,
    pub saturation: f32,
    pub needs_repaint: bool,

    pub compositor_state: CompositorState,
    pub shm_state: ShmState,
    pub xdg_shell_state: XdgShellState,
    pub layer_shell_state: WlrLayerShellState,
    pub output_manager_state: OutputManagerState,
    pub seat_state: SeatState<ZenWm>,
    pub viewporter_state: ViewporterState,
    pub fractional_scale_manager_state: FractionalScaleManagerState,
    pub single_pixel_buffer_state: SinglePixelBufferState,
    pub presentation_state: PresentationState,
    pub data_device_state: DataDeviceState,
    pub primary_selection_state: PrimarySelectionState,
    pub data_control_state: DataControlState,
    pub keyboard_shortcuts_inhibit_state: KeyboardShortcutsInhibitState,
    pub xdg_activation_state: XdgActivationState,
    pub popups: PopupManager,

    pub seat: Seat<ZenWm>,
    pub output: Output,
    pub workspaces: Workspaces,
    pub overview: OverviewState,
    pub pending_master_side: Option<SplitSide>,
    pub cursor_location: Point<f64, Logical>,
    pub cursor: MemoryRenderBuffer,
    pub suppressed_keys: Vec<Keysym>,
    /// mtime of the config file at last reload (for live-reload polling).
    pub config_mtime: Option<SystemTime>,
    lasrconfig_check: Instant,
}

impl ZenWm {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        display_handle: DisplayHandle,
        event_loop_handle: LoopHandle<'static, ZenWm>,
        output: Output,
        config: Config,
    ) -> Self {
        let dh = display_handle;

        let compositor_state = CompositorState::new::<Self>(&dh);
        let shm_state = ShmState::new::<Self>(&dh, vec![]);
        let xdg_shell_state = XdgShellState::new::<Self>(&dh);
        let layer_shell_state = WlrLayerShellState::new::<Self>(&dh);
        let output_manager_state = OutputManagerState::new_with_xdg_output::<Self>(&dh);
        let mut seat_state = SeatState::new();
        let viewporter_state = ViewporterState::new::<Self>(&dh);
        let fractional_scale_manager_state = FractionalScaleManagerState::new::<Self>(&dh);
        let single_pixel_buffer_state = SinglePixelBufferState::new::<Self>(&dh);
        let presentation_state = PresentationState::new::<Self>(&dh, 0);
        let data_device_state = DataDeviceState::new::<Self>(&dh);
        let primary_selection_state = PrimarySelectionState::new::<Self>(&dh);
        let data_control_state = DataControlState::new::<Self, _>(&dh, Some(&primary_selection_state), |_| true);
        let keyboard_shortcuts_inhibit_state = KeyboardShortcutsInhibitState::new::<Self>(&dh);
        let xdg_activation_state = XdgActivationState::new::<Self>(&dh);
        let popups = PopupManager::default();

        let mut seat = seat_state.new_wl_seat(&dh, "seat0");
        seat.add_pointer();
        seat.add_keyboard(XkbConfig::default(), 200, 25)
            .expect("Failed to initialize the keyboard");

        let workspaces = Workspaces::new(
            config.workspace_count,
            config.workspace_columns,
            config.workspace_rows,
        );

        ZenWm {
            display_handle: dh,
            event_loop_handle,
            running: Arc::new(std::sync::atomic::AtomicBool::new(true)),
            socket_name: None,
            config,
            saturation_program: None,
            saturation: 1.0,
            needs_repaint: true,

            compositor_state,
            shm_state,
            xdg_shell_state,
            layer_shell_state,
            output_manager_state,
            seat_state,
            viewporter_state,
            fractional_scale_manager_state,
            single_pixel_buffer_state,
            presentation_state,
            data_device_state,
            primary_selection_state,
            data_control_state,
            keyboard_shortcuts_inhibit_state,
            xdg_activation_state,
            popups,

            seat,
            output,
            workspaces,
            overview: OverviewState::new(),
            pending_master_side: None,
            cursor_location: Point::from((0.0, 0.0)),
            cursor: crate::renderer::arrow_cursor_buffer(),
            suppressed_keys: Vec::new(),
            config_mtime: None,
            lasrconfig_check: Instant::now(),
        }
    }

    // ── live config reload ──────────────────────────────────────────────────

    /// Poll the config file (rate-limited) and hot-apply it when it changes.
    /// Returns true if a reload was performed.
    pub fn check_config_reload(&mut self, path: &Path) -> bool {
        let now = Instant::now();
        if now.duration_since(self.lasrconfig_check) < Duration::from_millis(500) {
            return false;
        }
        self.lasrconfig_check = now;

        let mtime = std::fs::metadata(path).ok().and_then(|m| m.modified().ok());
        if mtime == self.config_mtime {
            return false;
        }
        self.config_mtime = mtime;

        let new_cfg = Config::load(path);
        if new_cfg == self.config {
            tracing::debug!("config file touched but contents unchanged");
            return false;
        }
        self.apply_config(new_cfg);
        true
    }

    /// Swap in a new config and hot-apply the fields that need side effects.
    ///
    /// Everything else (keybinds, terminal/launcher, overview grid, colors) is
    /// read from `self.config` at use time, so replacing the struct applies it.
    fn apply_config(&mut self, new: Config) {
        let old = &self.config;
        let mut changed: Vec<&'static str> = Vec::new();
        if new.workspace_count != old.workspace_count {
            changed.push("workspace_count");
        }
        if new.gap != old.gap {
            changed.push("gap");
        }
        if new.layout != old.layout {
            changed.push("layout");
        }
        if new.mod_key != old.mod_key {
            changed.push("mod_key");
        }
        if new.terminal != old.terminal {
            changed.push("terminal");
        }
        if new.launcher != old.launcher {
            changed.push("launcher");
        }
        if new.saturation != old.saturation {
            changed.push("saturation");
        }
        if new.workspace_columns != old.workspace_columns {
            changed.push("workspace_columns");
        }
        if new.workspace_rows != old.workspace_rows {
            changed.push("workspace_rows");
        }
        if new.overview_dim != old.overview_dim {
            changed.push("overview_dim");
        }
        if new.background != old.background {
            changed.push("background");
        }

        // Safe workspace-count change: grow, or shrink only if the tail workspaces
        // are empty (refused otherwise, and the config is clamped back to reality).
        let mut new = new;
        if !self.workspaces.resize(new.workspace_count) {
            tracing::warn!(
                requested = new.workspace_count,
                current = self.workspaces.count(),
                "workspace_count not applied (would drop non-empty workspaces)"
            );
            new.workspace_count = self.workspaces.count();
        }

        // Only apply the file's saturation if it actually changed in the file, so
        // a runtime Mod+s toggle isn't stomped by an unrelated edit.
        if new.saturation != old.saturation {
            self.saturation = new.saturation;
        }

        let gap_or_layout_changed = new.gap != old.gap || new.layout != old.layout;
        self.config = new;

        if gap_or_layout_changed {
            self.layout_active_workspace();
        }
        // Keep the overview selection inside bounds after a shrink/grow.
        if self.overview.active && self.overview.selected >= self.workspaces.count() {
            self.overview.selected = self.workspaces.count().saturating_sub(1);
        }
        self.request_repaint();

        tracing::info!(?changed, "config reloaded and applied");
    }

    pub fn request_repaint(&mut self) {
        self.needs_repaint = true;
    }

    // ── autostart (hyprland-style exec-once) ────────────────────────────────

    /// Launch every `[autostart].exec` command, with the wayland socket set.
    /// Runs once at startup; not re-run on config reload (matches `exec-once`).
    pub fn autostart(&mut self) {
        let cmds: Vec<Vec<String>> = self.config.autostart.exec.iter().map(ExecCmd::argv).collect();
        for argv in cmds {
            tracing::info!(?argv, "autostart");
            self.spawn(&argv);
        }
    }

    // ── windows ────────────────────────────────────────────────────────────

    pub fn map_window(&mut self, window: WindowElement) {
        let ws_idx = self.workspaces.active;
        if let Some(side) = self.pending_master_side.take() {
            self.workspaces.list[ws_idx].master_side = side;
        }
        self.workspaces.list[ws_idx]
            .space
            .map_element(window.clone(), Point::from((0, 0)), true);
        self.layout_active_workspace();
        self.focus_window(Some(window));
        self.request_repaint();
    }

    pub fn layout_active_workspace(&mut self) {
        let idx = self.workspaces.active;
        let output = self.output.clone();
        let gap = self.config.gap;
        let layout = self.config.layout.clone();
        self.workspaces.layout_workspace(idx, &output, gap, &layout);
    }

    pub fn focus_window(&mut self, window: Option<WindowElement>) {
        let serial = SCOUNTER.next_serial();
        for ws in &self.workspaces.list {
            for el in ws.space.elements() {
                el.set_activate(false);
            }
        }
        if let Some(w) = &window {
            w.set_activate(true);
        }
        let focus: Option<FocusTarget> = window.map(FocusTarget::Window);
        self.set_keyboard_focus(focus, serial);
    }

    fn set_keyboard_focus(&mut self, focus: Option<FocusTarget>, serial: Serial) {
        if let Some(keyboard) = self.seat.get_keyboard() {
            keyboard.set_focus(self, focus, serial);
        }
    }

    pub fn active_window(&self) -> Option<WindowElement> {
        self.workspaces.active_window()
    }

    // ── input ──────────────────────────────────────────────────────────────

    pub fn process_input_event_windowed<B: InputBackend>(&mut self, event: InputEvent<B>) {
        match event {
            InputEvent::Keyboard { event } => {
                let serial = SCOUNTER.next_serial();
                let time = Event::time_msec(&event);
                self.handle_key(event.key_code(), event.state(), serial, time);
            }
            InputEvent::PointerMotionAbsolute { event } => {
                let output_geo = self
                    .workspaces
                    .active_workspace()
                    .space
                    .output_geometry(&self.output)
                    .unwrap_or_else(|| Rectangle::from_size(Size::from((0, 0))));
                let pos = event.position_transformed(output_geo.size) + output_geo.loc.to_f64();
                let serial = SCOUNTER.next_serial();
                self.cursor_location = pos;
                let under = self.surface_under(pos);
                let pointer = self.seat.get_pointer().unwrap();
                pointer.motion(
                    self,
                    under,
                    &MotionEvent {
                        location: pos,
                        serial,
                        time: event.time_msec(),
                    },
                );
                pointer.frame(self);
                self.request_repaint();
            }
            InputEvent::PointerButton { event } => {
                let serial = SCOUNTER.next_serial();
                let button = event.button_code();
                let button_state = event.state();
                if button_state == ButtonState::Pressed {
                    self.update_keyboard_focus_from_pointer();
                }
                let pointer = self.seat.get_pointer().unwrap();
                pointer.button(
                    self,
                    &ButtonEvent {
                        button,
                        state: button_state,
                        serial,
                        time: event.time_msec(),
                    },
                );
                pointer.frame(self);
            }
            InputEvent::PointerAxis { event } => {
                let mut frame = AxisFrame::new(event.time_msec()).source(event.source());
                if let Some(h) = event.amount(Axis::Horizontal) {
                    frame = frame.value(Axis::Horizontal, h);
                }
                if let Some(v) = event.amount(Axis::Vertical) {
                    frame = frame.value(Axis::Vertical, v);
                }
                let pointer = self.seat.get_pointer().unwrap();
                pointer.axis(self, frame);
                pointer.frame(self);
            }
            _ => (),
        }
    }

    fn update_keyboard_focus_from_pointer(&mut self) {
        if self.overview.active {
            return;
        }
        let under = self.surface_under(self.cursor_location);
        if let Some((FocusTarget::Window(w), _)) = &under {
            self.focus_window(Some(w.clone()));
        }
    }

    pub fn surface_under(&self, pos: Point<f64, Logical>) -> Option<(FocusTarget, Point<f64, Logical>)> {
        if self.overview.active {
            return None;
        }
        // Top/Overlay layer surfaces first.
        let layer_map = layer_map_for_output(&self.output);
        for layer in layer_map.layers().rev() {
            if let Some(geo) = layer_map.layer_geometry(layer) {
                if geo.to_f64().contains(pos) {
                    let loc = pos - geo.loc.to_f64();
                    return Some((FocusTarget::LayerSurface(layer.clone()), loc));
                }
            }
        }
        // Windows.
        if let Some((window, loc)) = self.workspaces.active_workspace().space.element_under(pos) {
            return Some((FocusTarget::Window(window.clone()), loc.to_f64()));
        }
        None
    }

    fn handle_key(&mut self, keycode: Keycode, state: KeyState, serial: Serial, time: u32) -> bool {
        let keyboard = match self.seat.get_keyboard() {
            Some(k) => k,
            None => return false,
        };

        let action = keyboard.input(self, keycode, state, serial, time, |state_data, modifiers, handle| {
            let keysym = handle.modified_sym();

            if state == KeyState::Pressed {
                let action = if state_data.overview.active {
                    keybindings::resolve_overview(*modifiers, keysym, &state_data.config)
                } else {
                    keybindings::resolve(*modifiers, keysym, &state_data.config)
                };

                if matches!(action, KeyAction::None) {
                    // Overview grabs all keys; normal mode forwards.
                    if state_data.overview.active {
                        FilterResult::Intercept(KeyAction::None)
                    } else {
                        FilterResult::Forward
                    }
                } else {
                    FilterResult::Intercept(action)
                }
            } else if state_data.overview.active {
                // Consume key releases while in the overview.
                FilterResult::Intercept(KeyAction::None)
            } else {
                FilterResult::Forward
            }
        });

        match action {
            Some(KeyAction::None) => true,
            Some(action) => {
                self.process_action(action);
                true
            }
            None => false,
        }
    }

    pub fn process_action(&mut self, action: KeyAction) {
        match action {
            KeyAction::None => {}
            KeyAction::Quit => {
                tracing::info!("quitting");
                self.running.store(false, Ordering::SeqCst);
            }
            KeyAction::Launch => {
                let launcher = self.config.launcher.clone();
                self.spawn(&launcher);
            }
            KeyAction::Terminal(side) => {
                self.pending_master_side = side;
                let terminal = self.config.terminal.clone();
                self.spawn(&terminal);
            }
            KeyAction::Exec(argv) => {
                self.spawn(&argv);
            }
            KeyAction::ToggleOverview => {
                self.overview.toggle(self.workspaces.active);
                self.request_repaint();
            }
            KeyAction::WorkspaceSwitch(delta) => {
                if self.workspaces.switch_relative(delta) {
                    self.layout_active_workspace();
                    if let Some(w) = self.workspaces.active_window() {
                        self.focus_window(Some(w));
                    }
                    self.request_repaint();
                }
            }
            KeyAction::CycleSaturation => {
                self.saturation = match self.saturation {
                    s if s > 0.99 => 0.0,
                    s if s < 0.35 => 0.35,
                    _ => 1.0,
                };
                tracing::info!(saturation = self.saturation, "saturation");
                self.request_repaint();
            }
            KeyAction::Overview(dir) => {
                self.overview.move_selection(
                    dir,
                    self.config.workspace_columns,
                    self.config.workspace_rows,
                    self.workspaces.count(),
                );
                self.request_repaint();
            }
            KeyAction::OverviewSelect => {
                let target = self.overview.selected_workspace();
                self.overview.exit();
                self.workspaces.switch_to(target);
                self.layout_active_workspace();
                if let Some(w) = self.workspaces.active_window() {
                    self.focus_window(Some(w));
                }
                self.request_repaint();
            }
            KeyAction::OverviewNext => {
                self.overview.move_relative(1, self.workspaces.count());
                self.request_repaint();
            }
            KeyAction::OverviewCancel => {
                self.overview.exit();
                self.request_repaint();
            }
        }
    }

    fn spawn(&mut self, argv: &[String]) {
        let socket_name = self.socket_name.clone();
        let Some(argv0) = argv.first() else {
            return;
        };
        let mut cmd = std::process::Command::new(argv0);
        cmd.args(&argv[1..]);
        if let Some(socket) = socket_name {
            cmd.env("WAYLAND_DISPLAY", socket);
        }
        if let Err(e) = cmd.spawn() {
            tracing::error!(error = %e, argv = ?argv, "failed to spawn");
        }
    }
}

// ── globals / delegates ─────────────────────────────────────────────────────

impl BufferHandler for ZenWm {
    fn buffer_destroyed(&mut self, _buffer: &WlBuffer) {}
}

impl CompositorHandler for ZenWm {
    fn compositor_state(&mut self) -> &mut CompositorState {
        &mut self.compositor_state
    }

    fn client_compositor_state<'a>(&self, client: &'a Client) -> &'a CompositorClientState {
        &client.get_data::<ClientState>().unwrap().compositor_state
    }

    fn new_surface(&mut self, _surface: &WlSurface) {}

    fn commit(&mut self, surface: &WlSurface) {
        on_commit_buffer_handler::<Self>(surface);

        if !smithay::wayland::compositor::is_sync_subsurface(surface) {
            let mut root = surface.clone();
            while let Some(parent) = smithay::wayland::compositor::get_parent(&root) {
                root = parent;
            }
            if let Some(window) = self.workspaces.window_for_surface(&root) {
                window.0.on_commit();
            }
        }
        self.popups.commit(surface);
        ensure_initial_configure(surface, self);
        self.request_repaint();
    }
}

impl SeatHandler for ZenWm {
    type KeyboardFocus = FocusTarget;
    type PointerFocus = FocusTarget;
    type TouchFocus = FocusTarget;

    fn seat_state(&mut self) -> &mut SeatState<Self> {
        &mut self.seat_state
    }

    fn focus_changed(&mut self, _seat: &Seat<Self>, _focused: Option<&Self::KeyboardFocus>) {}

    fn cursor_image(&mut self, _seat: &Seat<Self>, _image: CursorImageStatus) {
        // zen-wm draws its own cursor; client cursors are ignored.
    }
}

impl ShmHandler for ZenWm {
    fn shm_state(&self) -> &ShmState {
        &self.shm_state
    }
}

impl XdgShellHandler for ZenWm {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState {
        &mut self.xdg_shell_state
    }

    fn new_toplevel(&mut self, surface: ToplevelSurface) {
        let window = WindowElement(Window::new_wayland_window(surface));
        self.map_window(window);
    }

    fn new_popup(&mut self, surface: PopupSurface, _positioner: PositionerState) {
        let popup = PopupKind::Xdg(surface);
        self.popups.track_popup(popup).unwrap();
    }

    fn grab(&mut self, surface: PopupSurface, _seat: wl_seat::WlSeat, serial: smithay::utils::Serial) {
        let popup = PopupKind::Xdg(surface);
        let root = self
            .seat
            .get_keyboard()
            .and_then(|k| k.current_focus())
            .unwrap_or(FocusTarget::Popup(popup.clone()));
        let _ = self.popups.grab_popup(root, popup, &self.seat, serial);
    }

    fn reposition_request(&mut self, surface: PopupSurface, positioner: PositionerState, _token: u32) {
        surface.with_pending_state(|state| {
            state.positioner = positioner;
        });
        surface.send_configure().unwrap();
    }
}

impl WlrLayerShellHandler for ZenWm {
    fn shell_state(&mut self) -> &mut WlrLayerShellState {
        &mut self.layer_shell_state
    }

    fn new_layer_surface(
        &mut self,
        surface: LayerSurface,
        output: Option<wl_output::WlOutput>,
        _layer: WlrLayer,
        namespace: String,
    ) {
        let output = output
            .as_ref()
            .and_then(Output::from_resource)
            .unwrap_or_else(|| self.output.clone());
        let layer = DesktopLayerSurface::new(surface, namespace);
        let mut map = layer_map_for_output(&output);
        map.map_layer(&layer).unwrap();
        if let Some(mode) = output.current_mode() {
            let size = mode.size.to_logical(1);
            layer
                .layer_surface()
                .with_pending_state(|state| state.size = Some(size));
        }
        layer.layer_surface().send_configure();
        self.request_repaint();
    }

    fn layer_destroyed(&mut self, surface: LayerSurface) {
        if let Some(layer) = layer_map_for_output(&self.output)
            .layers()
            .find(|l| l.layer_surface() == &surface)
            .cloned()
        {
            layer_map_for_output(&self.output).unmap_layer(&layer);
        }
        self.request_repaint();
    }

    fn ack_configure(&mut self, _surface: WlSurface, _configure: LayerSurfaceConfigure) {
        self.request_repaint();
    }
}

impl OutputHandler for ZenWm {}

impl XdgActivationHandler for ZenWm {
    fn activation_state(&mut self) -> &mut XdgActivationState {
        &mut self.xdg_activation_state
    }

    fn token_created(&mut self, _token: XdgActivationToken, _data: XdgActivationTokenData) -> bool {
        true
    }

    fn request_activation(&mut self, _token: XdgActivationToken, _token_data: XdgActivationTokenData, surface: WlSurface) {
        if let Some(window) = self.workspaces.window_for_surface(&surface) {
            self.focus_window(Some(window));
        }
    }
}

impl KeyboardShortcutsInhibitHandler for ZenWm {
    fn keyboard_shortcuts_inhibit_state(&mut self) -> &mut KeyboardShortcutsInhibitState {
        &mut self.keyboard_shortcuts_inhibit_state
    }
}

impl SelectionHandler for ZenWm {
    type SelectionUserData = ();
}

impl DataDeviceHandler for ZenWm {
    fn data_device_state(&self) -> &DataDeviceState {
        &self.data_device_state
    }
}

impl ClientDndGrabHandler for ZenWm {}

impl ServerDndGrabHandler for ZenWm {}

impl PrimarySelectionHandler for ZenWm {
    fn primary_selection_state(&self) -> &PrimarySelectionState {
        &self.primary_selection_state
    }
}

impl DataControlHandler for ZenWm {
    fn data_control_state(&self) -> &DataControlState {
        &self.data_control_state
    }
}

impl FractionalScaleHandler for ZenWm {}

smithay::delegate_compositor!(ZenWm);
smithay::delegate_shm!(ZenWm);
smithay::delegate_seat!(ZenWm);
smithay::delegate_xdg_shell!(ZenWm);
smithay::delegate_layer_shell!(ZenWm);
smithay::delegate_output!(ZenWm);
smithay::delegate_viewporter!(ZenWm);
smithay::delegate_single_pixel_buffer!(ZenWm);
smithay::delegate_presentation!(ZenWm);
smithay::delegate_fractional_scale!(ZenWm);
smithay::delegate_data_device!(ZenWm);
smithay::delegate_primary_selection!(ZenWm);
smithay::delegate_data_control!(ZenWm);
smithay::delegate_keyboard_shortcuts_inhibit!(ZenWm);
smithay::delegate_xdg_activation!(ZenWm);

fn ensure_initial_configure(surface: &WlSurface, state: &mut ZenWm) {
    with_surface_tree_upward(
        surface,
        (),
        |_, _, _| TraversalAction::DoChildren(()),
        |_, states, _| {
            states
                .data_map
                .insert_if_missing(|| std::cell::RefCell::new(EnsureConfigureData::default()));
        },
        |_, _, _| true,
    );

    if let Some(window) = state.workspaces.window_for_surface(surface) {
        if let Some(toplevel) = window.0.toplevel() {
            let initial_configure_sent = with_states(surface, |states| {
                states
                    .data_map
                    .get::<XdgToplevelSurfaceData>()
                    .unwrap()
                    .lock()
                    .unwrap()
                    .initial_configure_sent
            });
            if !initial_configure_sent {
                toplevel.send_configure();
            }
        }
        return;
    }

    if let Some(popup) = state.popups.find_popup(surface) {
        let popup = match popup {
            PopupKind::Xdg(p) => p,
            PopupKind::InputMethod(_) => return,
        };
        if !popup.is_initial_configure_sent() {
            popup.send_configure().expect("initial popup configure failed");
        }
    }
}

#[derive(Default)]
struct EnsureConfigureData;
