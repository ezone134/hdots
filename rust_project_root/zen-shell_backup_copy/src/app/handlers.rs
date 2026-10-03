//! Wayland trait handlers: registry, compositor, layer shell, session
//! lock, outputs, seats, data-control dispatch.

use super::*;

impl ProvidesRegistryState for App {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    smithay_client_toolkit::registry_handlers![OutputState, SeatState];
}

impl CompositorHandler for App {
    fn scale_factor_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_factor: i32,
    ) {
    }
    fn transform_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_transform: wl_output::Transform,
    ) {
    }
    fn frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _time: u32,
    ) {
    }
    fn surface_enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
    }
    fn surface_leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
    }
}

impl LayerShellHandler for App {
    fn closed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, layer: &LayerSurface) {
        // a secondary surface closing (compositor layer restart) just unmaps
        // it; only the bar's own close ends the process
        let id = layer.wl_surface().id();
        let is_notif = self
            .notif_surf
            .as_ref()
            .map(|s| s.surface_id.id() == id)
            .unwrap_or(false);
        if is_notif {
            if let Some(s) = self.notif_surf.as_mut() {
                if s.surface_id.id() == id {
                    s.visible = false;
                }
            }
            return;
        }
        self.running.store(false, Ordering::Relaxed);
        std::process::exit(0);
    }

    fn configure(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        layer: &LayerSurface,
        configure: LayerSurfaceConfigure,
        _serial: u32,
    ) {
        let id = layer.wl_surface().id();
        let w = configure.new_size.0 as i32;
        let h = configure.new_size.1 as i32;
        // route: notification popup / the bar
        if self
            .notif_surf
            .as_ref()
            .map(|s| s.surface_id.id() == id)
            .unwrap_or(false)
        {
            self.notif_configure(w, h);
            return;
        }
        if std::env::var("ZEN_TRACE").is_ok() {
            eprintln!("zen: configure {:?}", configure.new_size);
        }
        if w > 0 && h > 0 {
            self.configured = (w, h);
            if let Some(renderer) = &mut self.renderer {
                renderer.resize(w, h);
            }
        }
        // ack received for our size commit (sctk acks internally)
        self.shell.size_pending = false;
        self.dirty = true;
        self.maybe_render();
    }
}

impl SessionLockHandler for App {
    /// The compositor granted the lock. Create the fullscreen lock surface on
    /// the first output; a configure will follow with the real size.
    fn locked(&mut self, _conn: &Connection, qh: &QueueHandle<Self>, session_lock: SessionLock) {
        if self.shell.mode != crate::shell::Mode::Lock {
            // not in the lock mode anymore (raced an IPC close) — let go
            session_lock.unlock();
            return;
        }
        let Some(output) = self.output_state.outputs().next() else {
            eprintln!("zen: session lock: no output to lock");
            session_lock.unlock();
            return;
        };
        if std::env::var("ZEN_TRACE").is_ok() {
            eprintln!("zen: session locked — creating lock surface");
        }
        let surface = self.compositor.create_surface(qh);
        let lock_surface = session_lock.create_lock_surface(surface, &output, qh);
        self.lock_surf = Some(LockSurface {
            surface: lock_surface,
            renderer: None,
            configured: (0, 0),
            dirty: false,
        });
        // first commit → the compositor replies with a configure (acked by sctk)
        self.lock_surf.as_ref().unwrap().surface.wl_surface().commit();
        let _ = self.conn.flush();
    }

    /// The compositor sized the lock surface: init the renderer and draw.
    fn configure(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        surface: SessionLockSurface,
        configure: SessionLockSurfaceConfigure,
        _serial: u32,
    ) {
        let Some(ls) = self.lock_surf.as_mut() else { return };
        if ls.surface.wl_surface().id() != surface.wl_surface().id() {
            return; // not ours
        }
        let (w, h) = (configure.new_size.0 as i32, configure.new_size.1 as i32);
        if w > 0 && h > 0 {
            ls.configured = (w, h);
            if ls.renderer.is_none() {
                let display_ptr = self.conn.backend().display_ptr() as *mut c_void;
                ls.renderer = Self::init_gl(display_ptr, ls.surface.wl_surface(), w.max(1), h.max(1))
                    .ok()
                    .map(Renderer::Gl);
            } else if let Some(r) = ls.renderer.as_mut() {
                r.resize(w, h);
            }
        }
        ls.dirty = true;
        self.render_lock();
    }

    /// The lock ended (we called `unlock()` or the compositor denied it):
    /// drop the surface and any cached backdrop, and make sure we're unlocked.
    fn finished(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _session_lock: SessionLock) {
        if std::env::var("ZEN_TRACE").is_ok() {
            eprintln!("zen: session lock finished");
        }
        self.lock_surf = None;
        self.session_lock_active = None;
        self.lock_bg_tex = None;
        self.lock_bg_loaded = None;
        self.lock_bg_size = (0, 0);
        self.lock_fade = None;
        if let Some(tok) = self.lock_fade_timer.take() {
            if let Some(h) = self.loop_handle.clone() {
                h.remove(tok);
            }
        }
        if self.shell.mode == crate::shell::Mode::Lock {
            self.shell.lock_session = false;
            self.apply_mode(crate::shell::Mode::Collapsed);
        }
        self.dirty = true;
        self.maybe_render();
    }
}

impl OutputHandler for App {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }
    fn new_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }
    fn update_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }
    fn output_destroyed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }
}

impl SeatHandler for App {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seat_state
    }
    fn new_seat(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _seat: wl_seat::WlSeat,
    ) {
    }
    fn new_capability(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        seat: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Pointer && self.pointer.is_none() {
            if let Ok(ptr) = self.seat_state.get_pointer(qh, &seat) {
                self.pointer = Some(ptr);
                // touchpad pinch-zoom for the world map (best-effort)
                self.ensure_pinch(qh);
            }
        }
        if capability == Capability::Keyboard && self.keyboard.is_none() {
            // Key repeat needs the repeat-capable keyboard (holding an arrow
            // key in the launcher auto-scrolls only then). The callback gets
            // sctk's synthesized repeat events; route them through the same
            // handler the compositor's own repeat events use.
            if let Some(handle) = self.loop_handle.clone() {
                let callback =
                    Box::new(|app: &mut App, kbd: &wl_keyboard::WlKeyboard, event: KeyEvent| {
                        let conn = app.conn.clone();
                        let qh = app.qh.clone();
                        app.repeat_key(&conn, &qh, kbd, 0, event);
                    });
                if let Ok(kbd) =
                    self.seat_state.get_keyboard_with_repeat(qh, &seat, None, handle, callback)
                {
                    self.keyboard = Some(kbd);
                }
            }
            if self.keyboard.is_none() {
                if let Ok(kbd) = self.seat_state.get_keyboard(qh, &seat, None) {
                    self.keyboard = Some(kbd);
                }
            }
            // clipboard manager observes this seat's selection
            self.clip.bind_device(qh, &seat);
        }
    }
    fn remove_capability(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _seat: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Pointer && self.pointer.is_some() {
            if let Some(p) = self.pointer.take() {
                p.release();
            }
        }
        if capability == Capability::Keyboard && self.keyboard.is_some() {
            if let Some(k) = self.keyboard.take() {
                k.release();
            }
        }
    }
    fn remove_seat(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _seat: wl_seat::WlSeat) {}
}

impl App {
    /// Bind the touchpad pinch-gesture path (zwp_pointer_gestures_v1 →
    /// zwp_pointer_gesture_pinch_v1 on the seat pointer). Composition syntax
    /// matches the wlr-data-control binding pattern: lazy, succeeds only when
    /// the compositor advertises the global (Hyprland does; others degrade to
    /// the +/− buttons).
    pub(crate) fn ensure_pinch(&mut self, qh: &QueueHandle<App>) {
        let Some(ptr) = self.pointer.clone() else { return };
        if self.gestures.is_none() {
            if let Ok(g) =
                self.registry().bind_one::<ZwpPointerGesturesV1, App, ()>(qh, 1..=3, ())
            {
                self.gestures = Some(g);
            }
        }
        if self.pinch.is_none() {
            if let Some(g) = &self.gestures {
                self.pinch = Some(g.get_pinch_gesture(&ptr, qh, ()));
            }
        }
    }
}

impl wayland_client::Dispatch<ZwlrDataControlManagerV1, ()> for App {
    fn event(
        _state: &mut App,
        _proxy: &ZwlrDataControlManagerV1,
        _event: <ZwlrDataControlManagerV1 as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<App>,
    ) {
    }
}

impl wayland_client::Dispatch<ZwpPointerGesturesV1, ()> for App {
    fn event(
        _state: &mut App,
        _proxy: &ZwpPointerGesturesV1,
        _event: <ZwpPointerGesturesV1 as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<App>,
    ) {
    }
}

impl wayland_client::Dispatch<ZwpPointerGesturePinchV1, ()> for App {
    fn event(
        state: &mut App,
        _proxy: &ZwpPointerGesturePinchV1,
        event: <ZwpPointerGesturePinchV1 as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<App>,
    ) {
        use wayland_protocols::wp::pointer_gestures::zv1::client::zwp_pointer_gesture_pinch_v1::Event;
        // touchpad pinch over the world-map body → absolute zoom. The scale
        // is relative to the gesture START, so world_zoom = base × scale,
        // clamped to the same 1×..32× band the +/− buttons use.
        match event {
            Event::Begin { fingers, .. } => {
                // capture the anchor ONCE (the point under the cursor); it
                // stays fixed on screen while the gesture zooms around it
                if fingers >= 2 {
                    if let Some(anchor) = state.shell.world_cursor_anchor() {
                        state.pinch_base_zoom = state.shell.world_zoom;
                        state.pinch_anchor = Some(anchor);
                        state.pinch_engaged = true;
                    }
                }
            }
            Event::Update { scale, .. } => {
                if !state.pinch_engaged {
                    return;
                }
                let z = (state.pinch_base_zoom * scale as f32).clamp(1.0, 32.0);
                if (state.shell.world_zoom - z).abs() > 0.0005 {
                    if let Some(anchor) = state.pinch_anchor {
                        state.shell.world_pinch_zoom(z, anchor);
                        state.dirty = true;
                        state.maybe_render();
                    }
                }
            }
            Event::End { .. } => {
                state.pinch_engaged = false;
                state.pinch_anchor = None;
            }
            _ => {} // non-exhaustive protocol enum (future versions)
        }
    }
}

impl wayland_client::Dispatch<ZwlrDataControlDeviceV1, ()> for App {
    fn event(
        state: &mut App,
        _proxy: &ZwlrDataControlDeviceV1,
        event: <ZwlrDataControlDeviceV1 as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<App>,
    ) {
        use wayland_protocols_wlr::data_control::v1::client::zwlr_data_control_device_v1::Event;
        match event {
            Event::DataOffer { id } => state.clip.on_data_offer(id),
            Event::Selection { id } => state.clip.on_selection(id),
            _ => {}
        }
    }

    // the `data_offer` event creates the offer object (opcode 0)
    wayland_client::event_created_child!(App, ZwlrDataControlDeviceV1, [
        wayland_protocols_wlr::data_control::v1::client::zwlr_data_control_device_v1::EVT_DATA_OFFER_OPCODE => (ZwlrDataControlOfferV1, ()),
    ]);
}

impl wayland_client::Dispatch<ZwlrDataControlOfferV1, ()> for App {
    fn event(
        state: &mut App,
        _proxy: &ZwlrDataControlOfferV1,
        event: <ZwlrDataControlOfferV1 as wayland_client::Proxy>::Event,
        _data: &(),
        conn: &Connection,
        _qhandle: &QueueHandle<App>,
    ) {
        use wayland_protocols_wlr::data_control::v1::client::zwlr_data_control_offer_v1::Event;
        if let Event::Offer { mime_type } = event {
            state
                .clip
                .on_offer_mime(conn, mime_type, &mut state.shell, &mut state.img);
            state.dirty = true;
            state.maybe_render();
        }
    }
}

impl wayland_client::Dispatch<ZwlrDataControlSourceV1, ()> for App {
    fn event(
        state: &mut App,
        proxy: &ZwlrDataControlSourceV1,
        event: <ZwlrDataControlSourceV1 as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<App>,
    ) {
        use wayland_protocols_wlr::data_control::v1::client::zwlr_data_control_source_v1::Event;
        if let Event::Send { mime_type, fd } = event {
            state.clip.on_source_send(proxy, &mime_type, fd);
        }
    }
}
