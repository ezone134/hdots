mod color;
mod wayland;

use color::{RenderContext, ShmPool};
use wayland_client::protocol::wl_shm;
use wayland_client::Connection;
use wayland::AppState;
use wayland::{HEIGHT, WIDTH};
use wayland_protocols_wlr::layer_shell::v1::client::{zwlr_layer_shell_v1, zwlr_layer_surface_v1};

fn main() {
    let conn = Connection::connect_to_env().expect("failed to connect to wayland");
    let mut event_queue = conn.new_event_queue::<AppState>();
    let qh = event_queue.handle();
    let mut state = AppState::new(conn.clone(), qh.clone());
    apply_theme(&mut state);

    conn.display().get_registry(&qh, ());
    roundtrip(&conn, &mut event_queue, &mut state, "registry read");

    create_window(&mut state);
    roundtrip(&conn, &mut event_queue, &mut state, "layer-surface configure");

    state.snapshot_clipboard();
    ensure_data_device(&mut state);
    event_queue.flush().ok();

    let clip_slot = state.clipboard.clone();
    std::thread::spawn(move || crate::wayland::clipboard_loop(clip_slot));

    let mut frame_counter = 0u64;

    while !state.exit {
        if let Err(e) = event_queue.blocking_dispatch(&mut state) {
            report_err(&conn, "blocking_dispatch", e);
            break;
        }
        state.sync_clipboard();
        if let Some(down) = state.backspace_down {
            let now = std::time::Instant::now();
            if now.duration_since(down).as_millis() > 350
                && now.duration_since(state.backspace_last).as_millis() >= 45
            {
                state.backspace_hex();
                state.backspace_last = now;
            }
        }

        if state.exit {
            break;
        }
        if state.surface_ready() && state.needs_render && !state.waiting_for_frame {
            let (w, h) = (WIDTH as usize, HEIGHT as usize);
            let idx = (frame_counter % 2) as usize;
            frame_counter += 1;

            let mut buf = vec![0u32; w * h];
            render_frame(&state, &mut buf, w, h);

            let pool = state.pool.as_mut().unwrap();
            let offset = idx * WIDTH as usize * HEIGHT as usize * 4;
            pool.write_pixels(offset, w, h, &buf);

            let surface = state.surface.as_ref().unwrap();
            let buffer = state.buffers[idx].as_ref().unwrap().clone();
            surface.attach(Some(&buffer), 0, 0);
            surface.damage_buffer(0, 0, WIDTH, HEIGHT);

            let cb = surface.frame(&state.qh, ());
            state.frame_callback = Some(cb.clone());
            state.waiting_for_frame = true;

            surface.commit();
            state.needs_render = false;
        }
    }

    if state.output_hex {
        println!("{}", state.color.hex());
    }
}

fn apply_theme(state: &mut AppState) {
    let dir = match std::env::var("states2") {
        Ok(d) if !d.is_empty() => d,
        _ => return,
    };
    let path = std::path::Path::new(&dir).join("shell_vars.json");
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(_) => return,
    };
    let json: serde_json::Value = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(_) => return,
    };
    let fetch = |key: &str, fallback: u32| -> u32 {
        json.get(key)
            .and_then(|v| v.as_str())
            .and_then(|s| color::parse_hex(s))
            .map(|c| c.pixel())
            .unwrap_or(fallback)
    };
    state.theme_fg = fetch("fg", 0xFFF8F8F2);
    state.theme_bg = fetch("bg", 0xFF282A36);
}

fn roundtrip(conn: &Connection, queue: &mut wayland_client::EventQueue<AppState>, state: &mut AppState, step: &str) {
    if let Err(e) = queue.roundtrip(state) {
        report_err(conn, &format!("roundtrip during {step}"), e);
        std::process::exit(2);
    }
}

fn report_err(conn: &Connection, step: &str, e: wayland_client::DispatchError) {
    eprintln!("{step} failed: {e:?}");
    if let Some(pe) = conn.protocol_error() {
        eprintln!("protocol error: object={:?} code={} msg={}", pe.object_id, pe.code, pe.message);
    }
    eprintln!(
        "WAYLAND_DISPLAY={:?} XDG_RUNTIME_DIR={:?}",
        std::env::var("WAYLAND_DISPLAY"),
        std::env::var("XDG_RUNTIME_DIR")
    );
}

fn create_window(state: &mut AppState) {
    let layer_shell = state
        .layer_shell
        .as_ref()
        .expect("no zwlr_layer_shell_v1 global (need Hyprland or a wlr-layer-shell compositor)");

    let surface = state.compositor.as_ref().expect("no compositor").create_surface(&state.qh, ());
    let layer_surface = layer_shell.get_layer_surface(
        &surface,
        None,
        zwlr_layer_shell_v1::Layer::Overlay,
        "color-chooser".to_string(),
        &state.qh,
        (),
    );
    layer_surface.set_size(WIDTH as u32, HEIGHT as u32);
    layer_surface.set_margin(32, 32, 32, 32);
    layer_surface.set_exclusive_zone(0);
    layer_surface.set_keyboard_interactivity(
        zwlr_layer_surface_v1::KeyboardInteractivity::Exclusive,
    );
    surface.commit();

    let total = WIDTH as usize * HEIGHT as usize * 4;
    let pool = ShmPool::new(total * 2);

    let shm = state.shm.as_ref().expect("no shm");
    let shm_pool = shm.create_pool(pool.fd(), (total * 2) as i32, &state.qh, ());
    let buf1 = shm_pool.create_buffer(
        0,
        WIDTH,
        HEIGHT,
        WIDTH * 4,
        wl_shm::Format::Argb8888,
        &state.qh,
        (),
    );
    let buf2 = shm_pool.create_buffer(
        total as i32,
        WIDTH,
        HEIGHT,
        WIDTH * 4,
        wl_shm::Format::Argb8888,
        &state.qh,
        (),
    );
    drop(shm_pool);

    state.surface = Some(surface);
    state.layer_surface = Some(layer_surface);
    state.pool = Some(pool);
    state.buffers[0] = Some(buf1);
    state.buffers[1] = Some(buf2);
    state.needs_render = true;
}

fn render_frame(state: &AppState, buf: &mut [u32], w: usize, h: usize) {
    let s = 2usize;
    let bw = w * s;
    let bh = h * s;
    let mut big = vec![0u32; bw * bh];
    {
        let bg = state.theme_bg;
        let fg = state.theme_fg;
        let mut ctx = RenderContext { buf: &mut big, width: bw, height: bh, scale: s as i32 };
        ctx.clear(bg);

        let cx = state.center().0;
        let cy = state.center().1;
        ctx.render_wheel(&state.wheel, cx - 128, cy - 128, state.color.val);

        let ang = state.color.hue.to_radians();
        let r = (state.color.sat * state.wheel_radius()) as i32;
        ctx.render_marker(cx + (ang.cos() * r as f32) as i32, cy + (ang.sin() * r as f32) as i32);

        let (rr, gg, bb) = state.color.rgb();
        let values = [state.color.val, rr as f32 / 255.0, gg as f32 / 255.0, bb as f32 / 255.0];
        for i in 0..4 {
            let (sx, sy, sw, sh) = state.slider_rect(i);
            let label = match i {
                0 => "V",
                1 => "R",
                2 => "G",
                _ => "B",
            };
            let label_color = match i {
                0 => fg,
                1 => 0xFFFF0000,
                2 => 0xFF00B500,
                _ => 0xFF0078FF,
            };
            ctx.render_hex_text(label, 8, sy, label_color);
            ctx.render_slider(sx, sy, sw, sh, values[i], 0xFFFFFFFF, fg);
        }

        let (fx, fy, fw, fh) = wayland::hex_field();
        ctx.fill_rect(fx, fy, fw, fh, bg);
        ctx.fill_rect(fx, fy, fw, 1, fg);
        ctx.fill_rect(fx, fy + fh - 1, fw, 1, fg);
        ctx.fill_rect(fx, fy, 1, fh, fg);
        ctx.fill_rect(fx + fw - 1, fy, 1, fh, fg);
        let field_text = format!("#{}", state.hex_input);
        ctx.render_hex_text(&field_text, fx + 8, fy + 9, fg);

        let (px, py, pw, ph) = wayland::btn_paste();
        let paste_tx = px + ((pw - 50) / 2);
        ctx.fill_rect(px, py, pw, ph, bg);
        ctx.fill_rect(px, py, pw, 1, fg);
        ctx.fill_rect(px, py + ph - 1, pw, 1, fg);
        ctx.fill_rect(px, py, 1, ph, fg);
        ctx.fill_rect(px + pw - 1, py, 1, ph, fg);
        ctx.render_hex_text(wayland::BTN_PASTE_LABEL, paste_tx, py + 9, fg);

        ctx.fill_circle(300, 417, 15, state.color.pixel());
        ctx.stroke_circle(300, 417, 15, 1, fg);

        let (ox, oy, ow, oh) = wayland::btn_ok(state.show_paste);
        ctx.fill_rect(ox, oy, ow, oh, bg);
        ctx.fill_rect(ox, oy, ow, 1, fg);
        ctx.fill_rect(ox, oy + oh - 1, ow, 1, fg);
        ctx.fill_rect(ox, oy, 1, oh, fg);
        ctx.fill_rect(ox + ow - 1, oy, 1, oh, fg);
        ctx.render_hex_text(wayland::BTN_OK_LABEL, ox + ((ow - 20) / 2), oy + 11, fg);

        let (bcx, bcy, bcw, bch) = wayland::btn_copy(state.show_paste);
        let copy_border = if state.copy_flash_active() { 0xFF00FF00 } else { fg };
        ctx.fill_rect(bcx, bcy, bcw, bch, bg);
        ctx.fill_rect(bcx, bcy, bcw, 1, copy_border);
        ctx.fill_rect(bcx, bcy + bch - 1, bcw, 1, copy_border);
        ctx.fill_rect(bcx, bcy, 1, bch, copy_border);
        ctx.fill_rect(bcx + bcw - 1, bcy, 1, bch, copy_border);
        ctx.render_hex_text(wayland::BTN_COPY_LABEL, bcx + ((bcw - 40) / 2), bcy + 11, fg);

        let (cx, cy, cw, ch) = wayland::btn_close();
        ctx.fill_rect(cx, cy, cw, ch, bg);
        ctx.fill_rect(cx, cy, cw, 1, fg);
        ctx.fill_rect(cx, cy + ch - 1, cw, 1, fg);
        ctx.fill_rect(cx, cy, 1, ch, fg);
        ctx.fill_rect(cx + cw - 1, cy, 1, ch, fg);
        ctx.render_hex_text("Close", cx + ((cw - 50) / 2), cy + 6, fg);
    }

    for y in 0..h {
        for x in 0..w {
            let mut a = 0u32;
            let mut r = 0u32;
            let mut g = 0u32;
            let mut bl = 0u32;
            for dy in 0..s {
                for dx in 0..s {
                    let p = big[(y * s + dy) * bw + (x * s + dx)];
                    a += (p >> 24) & 0xFF;
                    r += (p >> 16) & 0xFF;
                    g += (p >> 8) & 0xFF;
                    bl += p & 0xFF;
                }
            }
            let n = (s * s) as u32;
            buf[y * w + x] = ((a / n) << 24) | ((r / n) << 16) | ((g / n) << 8) | (bl / n);
        }
    }
}

fn ensure_data_device(state: &mut AppState) {
    if state.data_device.is_none() {
        if let (Some(mgr), Some(seat)) = (state.data_device_manager.as_ref(), state.seat.as_ref()) {
            let dd = mgr.get_data_device(seat, &state.qh, ());
            state.data_device = Some(dd);
        }
    }
}