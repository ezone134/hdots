//! Aux layer surfaces (notification popup, lock backdrop), their render
//! passes and image helpers.

use super::*;

pub(super) struct AuxSurface {
    pub(super) surface_id: wl_surface::WlSurface,
    pub(super) layer: LayerSurface,
    pub(super) renderer: Option<Renderer>,
    /// last size the compositor configured us at
    pub(super) configured: (i32, i32),
    pub(super) size_pending: bool,
    pub(super) dirty: bool,
    pub(super) visible: bool,
}

impl AuxSurface {
    pub(super) fn create(
        compositor: &CompositorState,
        layer_shell: &LayerShell,
        qh: &QueueHandle<App>,
        display_ptr: *mut c_void,
        layer_kind: Layer,
        anchors: Anchor,
        name: &str,
        margin: (i32, i32, i32, i32),
        w: i32,
        h: i32,
    ) -> Option<Self> {
        let surface = compositor.create_surface(qh);
        let layer = layer_shell.create_layer_surface(qh, surface.clone(), layer_kind, Some(name), None);
        layer.set_anchor(anchors);
        layer.set_exclusive_zone(-1); // floats over content; reserves nothing
        layer.set_keyboard_interactivity(KeyboardInteractivity::None);
        layer.set_margin(margin.0, margin.1, margin.2, margin.3);
        // a 0-size layer surface whose anchor doesn't span the dimension is a
        // protocol error ("x == 0 but anchor doesn't have left and right"),
        // so partial-anchor surfaces need a concrete size from the start
        if w > 0 && h > 0 {
            layer.set_size(w as u32, h as u32);
        }
        layer.commit(); // flushed with the rest of the initial roundtrip
        let renderer = App::init_gl(display_ptr, layer.wl_surface(), w.max(1), h.max(1))
            .ok()
            .map(Renderer::Gl);
        Some(AuxSurface {
            surface_id: surface,
            layer,
            renderer,
            configured: (0, 0),
            size_pending: false,
            dirty: false,
            visible: true,
        })
    }

    fn commit_size(&mut self, conn: &Connection, w: i32, h: i32) {
        if self.size_pending {
            return;
        }
        self.layer.set_size(w.max(1) as u32, h.max(1) as u32);
        self.layer.commit();
        self.size_pending = true;
        let _ = conn.flush(); // get the size commit out NOW (C++ lesson)
    }

    pub(super) fn hide(&mut self, conn: &Connection) {
        if !self.visible {
            return;
        }
        self.visible = false;
        // a buffer-less commit unmaps the surface; it re-appears on the next
        // sized + drawn commit
        self.surface_id.attach(None, 0, 0);
        self.surface_id.commit();
        let _ = conn.flush();
    }
}

pub(super) fn first_wallpaper(dir: &std::path::Path) -> Option<String> {
    if dir.as_os_str().is_empty() || !dir.is_dir() {
        return None;
    }
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .filter(|e| e.path().is_file())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    for n in names {
        let p = dir.join(&n);
        if crate::img::supported_image(&p) {
            return Some(p.to_string_lossy().into_owned());
        }
    }
    None
}

impl App {
        pub(super) fn notif_configure(&mut self, w: i32, h: i32) {
        if std::env::var("ZEN_TRACE").is_ok() {
            eprintln!("zen: notif surface configure {w}x{h}");
        }
        let Some(surf) = self.notif_surf.as_mut() else { return };
        if w > 0 && h > 0 {
            surf.configured = (w, h);
            if let Some(r) = surf.renderer.as_mut() {
                r.resize(w, h);
            }
        }
        surf.size_pending = false;
        surf.dirty = true;
        self.render_notifs();
    }

        pub(super) fn lock_wallpaper_path(&self) -> Option<String> {
        first_wallpaper(&self.shell.cfg.wallpaper_dir())
    }

        pub(super) fn lock_bg_upload(
        renderer: Option<&mut Renderer>,
        path: &str,
        w: i32,
        h: i32,
    ) -> Result<Option<Tex>, String> {
        let img = image::open(path).map_err(|e| format!("open {path}: {e}"))?.to_rgba8();
        let (iw, ih) = img.dimensions();
        let (cw, ch) = (w.max(1) as u32, h.max(1) as u32);
        // cover: scale so the image fills the box, then crop the overflow center
        let scale = (cw as f32 / iw as f32).max(ch as f32 / ih as f32).max(0.001);
        let (dw, dh) = ((iw as f32 * scale).round().max(1.0) as u32, (ih as f32 * scale).round().max(1.0) as u32);
        let resized = image::imageops::resize(&img, dw, dh, image::imageops::FilterType::Triangle);
        let ox = dw.saturating_sub(cw) / 2;
        let oy = dh.saturating_sub(ch) / 2;
        let cropped = image::imageops::crop_imm(&resized, ox, oy, cw, ch).to_image();
        // blur at half res, draw stretched — the upscale is invisible once
        // the image is blurred, and it keeps the one-time decode cheap
        let (bw, bh) = ((cw / 2).max(1), (ch / 2).max(1));
        let small = image::imageops::resize(&cropped, bw, bh, image::imageops::FilterType::Triangle);
        let blurred = image::imageops::blur(&small, 6.0);
        let Some(r) = renderer else { return Ok(None) };
        let t = r.upload_texture(bw, bh, &blurred.into_raw())?;
        Ok(Some(t))
    }

        pub(super) fn render_lock(&mut self) {
        // the backdrop fade is only animatable once the loop is up; once the
        // 0.25 s elapsed the fade is done and the timer must stop (0 CPU)
        if let Some(t0) = self.lock_fade {
            if t0.elapsed() >= Duration::from_millis(250) || self.loop_handle.is_none() {
                self.lock_fade = None;
            }
        }
        let (cw, ch, dirty) = match self.lock_surf.as_ref() {
            Some(ls) => (ls.configured.0, ls.configured.1, ls.dirty),
            None => return,
        };
        if !dirty || cw <= 0 || ch <= 0 {
            return;
        }
        // (re)decode + blur the backdrop whenever the wallpaper or size changed
        let path = self.lock_wallpaper_path();
        if self.lock_bg_loaded.as_deref() != path.as_deref() || self.lock_bg_size != (cw, ch) {
            self.lock_bg_loaded = path.clone();
            self.lock_bg_tex = None;
            if let Some(p) = path {
                let r = self.lock_surf.as_mut().and_then(|ls| ls.renderer.as_mut());
                self.lock_bg_tex = Self::lock_bg_upload(r, &p, cw, ch).ok().flatten();
            }
            self.lock_bg_size = (cw, ch);
            self.lock_fade = Some(Instant::now());
        }
        let mut renderer = self.lock_surf.as_mut().and_then(|ls| ls.renderer.take());
        if let Some(r) = renderer.as_mut() {
            r.make_current();
            r.begin_frame();
            match self.lock_bg_tex {
                Some(tex) => {
                    // fade in over ~0.25 s (same dance as the wallpaper daemon)
                    let a = match self.lock_fade {
                        Some(t0) => (t0.elapsed().as_secs_f32() / 0.25).clamp(0.0, 1.0),
                        None => 1.0,
                    };
                    // 0xRRGGBBAA: white RGB + alpha in the low byte (same
                    // fix as the wallpaper daemon's fade tint)
                    let tint = 0xffffff00u32 | ((a * 255.0) as u32 & 0xff);
                    r.text_quad_tinted(&tex, 0.0, 0.0, cw as f32, ch as f32, tint);
                }
                None => {
                    // no wallpaper anywhere — plain base color, dimmed by the scene
                    r.rect(0.0, 0.0, cw as f32, ch as f32, 0.0, self.shell.cfg.bg());
                }
            }
            let scene = self
                .shell
                .layout_for(crate::shell::Mode::Lock, cw as f32, ch as f32);
            self.draw_cmds(r, &scene);
            r.flush();
        }
        if let Some(ls) = self.lock_surf.as_mut() {
            ls.renderer = renderer;
            ls.dirty = false;
        }
        // keep animating while the backdrop fades in, then stop
        if self.lock_fade.is_some() {
            self.ensure_lock_fade_timer();
        } else if let Some(tok) = self.lock_fade_timer.take() {
            if let Some(h) = self.loop_handle.clone() {
                h.remove(tok);
            }
        }
    }

        pub(super) fn ensure_lock_fade_timer(&mut self) {
        if self.lock_fade_timer.is_some() {
            return;
        }
        let Some(handle) = self.loop_handle.clone() else { return };
        let timer = calloop::timer::Timer::from_duration(Duration::from_millis(33));
        let token = handle
            .insert_source(timer, move |_, _, app: &mut App| {
                if app.lock_fade.is_some() {
                    // re-mark dirty each tick — the first frame cleared it and
                    // the backdrop fade would otherwise stall at alpha≈0
                    if let Some(ls) = app.lock_surf.as_mut() {
                        ls.dirty = true;
                    }
                    app.render_lock();
                    if app.lock_fade.is_some() {
                        return calloop::timer::TimeoutAction::ToDuration(Duration::from_millis(33));
                    }
                }
                if let Some(tok) = app.lock_fade_timer.take() {
                    if let Some(h) = app.loop_handle.clone() {
                        h.remove(tok);
                    }
                }
                calloop::timer::TimeoutAction::Drop
            })
            .expect("lock fade timer");
        self.lock_fade_timer = Some(token);
    }

        pub(super) fn show_notif_popup(&mut self) {
        let n = self.notif_popup.len().min(3);
        if n == 0 {
            if let Some(s) = self.notif_surf.as_mut() {
                s.hide(&self.conn);
            }
            return;
        }
        // Position below the bar so the toast never overlaps the pill —
        // when the bar is at the top, push the popup under it; otherwise the
        // default top-right placement is clear (bottom/left/right edges).
        if let Some(s) = self.notif_surf.as_mut() {
            let top_margin = if self.shell.bar_edge == crate::shell::BarEdge::Top {
                let bar_h = self.shell.cfg.bar_h() as i32;
                let float_off = if self.shell.bar_floating {
                    self.shell.cfg.bar.floating_offset
                } else {
                    0
                };
                float_off + bar_h + 12
            } else {
                12
            };
            s.layer.set_margin(top_margin, 12, 0, 0);
            s.layer.commit();
            let _ = self.conn.flush();
        }
        // single source of truth shared with the layout drawer
        let (w, h) = crate::shell::notif_popup_size(&self.notif_popup);
        let (was_hidden, cw, ch) = match self.notif_surf.as_ref() {
            Some(s) => (!s.visible, s.configured.0, s.configured.1),
            None => return,
        };
        if let Some(s) = self.notif_surf.as_mut() {
            s.visible = true;
            // commit a size change only when it's actually new — a same-size
            // commit after a hide may never be acked, wedging the popup; the
            // draw itself re-attaches a buffer and re-maps the surface
            if was_hidden || (cw, ch) != (w, h) {
                s.commit_size(&self.conn, w, h);
            }
        }
        self.render_notifs();
    }

        pub(super) fn arm_notif_timer(&mut self) {
        if let Some(tok) = self.notif_timer.take() {
            if let Some(h) = self.loop_handle.clone() {
                h.remove(tok);
            }
        }
        let Some(handle) = self.loop_handle.clone() else { return };
        let timeout_ms = self.shell.cfg.notifications.timeout_ms.max(0) as u64;
        if timeout_ms == 0 {
            return; // 0 = until dismissed manually
        }
        let timer = calloop::timer::Timer::from_duration(Duration::from_millis(timeout_ms));
        let token = handle
            .insert_source(timer, |_, _, app: &mut App| {
                if let Some(tok) = app.notif_timer.take() {
                    if let Some(h) = app.loop_handle.clone() {
                        h.remove(tok);
                    }
                }
                app.notif_popup.clear();
                if let Some(s) = app.notif_surf.as_mut() {
                    s.hide(&app.conn);
                }
                calloop::timer::TimeoutAction::Drop
            })
            .expect("notif timer");
        self.notif_timer = Some(token);
    }

        pub(super) fn render_notifs(&mut self) {
        let (cw, ch, dirty, pending) = match self.notif_surf.as_ref() {
            Some(s) => (s.configured.0, s.configured.1, s.dirty, s.size_pending),
            None => return,
        };
        if !dirty || pending || cw <= 0 || ch <= 0 {
            return;
        }
        // nothing to show (boot configure, dismissed popup): stay unmapped —
        // never paint an empty card
        if self.notif_popup.is_empty() {
            if let Some(s) = self.notif_surf.as_mut() {
                s.dirty = false;
            }
            return;
        }
        let scene = self.shell.layout_notif_popup(cw as f32, ch as f32, &self.notif_popup);
        let mut renderer = self.notif_surf.as_mut().unwrap().renderer.take();
        if let Some(r) = renderer.as_mut() {
            r.make_current();
            r.begin_frame();
            self.draw_cmds(r, &scene);
            r.flush();
        }
        let s = self.notif_surf.as_mut().unwrap();
        s.renderer = renderer;
        s.dirty = false;
    }
}

pub(super) extern "C" fn sig_handler(_: libc::c_int) {
    // minimal: leave the compositor cleanly
    std::process::exit(0);
}

// ----------------------------------------------------------------------
// sctk handler impls
// ----------------------------------------------------------------------

delegate_registry!(App);
