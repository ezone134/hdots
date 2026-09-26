mod vulkan;
mod wayland;
mod wgsl;

use std::fs;
use std::path::{Path, PathBuf};

use wayland_client::Connection;

use vulkan::RenderStatus;

const IMAGE_EXTS: &[&str] = &["png", "jpg", "jpeg"];

fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let dir_mode = args.first().map(|a| a == "--dir").unwrap_or(false);
    if dir_mode {
        args.remove(0);
    }

    if args.is_empty() {
        eprintln!("usage: zen-image [--dir] <image|dir>...");
        std::process::exit(1);
    }

    let mut paths = expand_args(&args);
    let mut cur = 0usize;
    if dir_mode {
        if let Some(first) = args.first() {
            let p = PathBuf::from(first);
            if !p.is_dir() {
                if let Some(parent) = p.parent().filter(|d| d.is_dir()) {
                    let all = images_in_dir(parent);
                    if !all.is_empty() {
                        cur = all.iter().position(|x| x == &p).unwrap_or(0);
                        paths = all;
                    }
                }
            }
        }
    }
    if paths.is_empty() {
        eprintln!("no images found");
        std::process::exit(1);
    }

    let img = decode(&paths[cur]);
    let (img_w, img_h) = (img.width(), img.height());
    eprintln!("{} ({}x{})", paths[cur].display(), img_w, img_h);

    let conn = Connection::connect_to_env().expect("wayland connection");
    let mut queue = conn.new_event_queue::<wayland::AppState>();
    let qh = queue.handle();
    let mut state = wayland::AppState::new(conn.clone(), qh.clone(), img_w, img_h);
    state.paths = paths;
    state.cur = cur;

    conn.display().get_registry(&qh, ());
    roundtrip(&conn, &mut queue, &mut state);

    state.create_window();
    roundtrip(&conn, &mut queue, &mut state);

    let (dptr, sptr) = state.surface_ptrs();
    let (w, h) = state.current_size();
    let bg = bg_clear();
    let mut vk = vulkan::Vulkan::new(dptr, sptr, w, h, img.as_raw(), img_w, img_h, bg);

    state.win_w = w;
    state.win_h = h;
    state.resize_pending = false;
    state.needs_render = true;

    loop {
        if let Err(e) = queue.blocking_dispatch(&mut state) {
            eprintln!("dispatch failed: {e:?}");
            break;
        }
        if state.exit {
            break;
        }

        let changed = if state.want_next {
            state.want_next = false;
            state.command_image(1)
        } else if state.want_prev {
            state.want_prev = false;
            state.command_image(-1)
        } else {
            false
        };
        if changed {
            if let Some(nimg) = decode_ok(&state.paths[state.cur]) {
                let (nw, nh) = (nimg.width(), nimg.height());
                vk.set_texture(nimg.as_raw(), nw, nh);
                state.img_w = nw;
                state.img_h = nh;
                state.reset_view();
                eprintln!("{} ({}x{})", state.paths[state.cur].display(), nw, nh);
            }
        }

        if state.resize_pending {
            let (tw, th) = (state.target_w.max(1), state.target_h.max(1));
            vk.resize(tw, th);
            state.win_w = tw;
            state.win_h = th;
            state.resize_pending = false;
            state.needs_render = true;
        }

        if state.needs_render {
            let (ew, eh) = vk.extents();
            let view = state.view(ew, eh);
            match vk.render(&view) {
                RenderStatus::Rendered => {
                    state.needs_render = false;
                }
                RenderStatus::Busy => {}
                RenderStatus::OutOfDate => {
                    vk.rebuild();
                    state.needs_render = true;
                }
            }
        }
        queue.flush().ok();
    }
}

fn roundtrip(
    conn: &Connection,
    queue: &mut wayland_client::EventQueue<wayland::AppState>,
    state: &mut wayland::AppState,
) {
    if let Err(e) = queue.roundtrip(state) {
        eprintln!("roundtrip failed: {e:?}");
        if let Some(pe) = conn.protocol_error() {
            eprintln!(
                "protocol error: object={:?} code={} msg={}",
                pe.object_id, pe.code, pe.message
            );
        }
        std::process::exit(2);
    }
}

fn expand_args(args: &[String]) -> Vec<PathBuf> {
    let mut list = Vec::new();
    for a in args {
        let p = PathBuf::from(a);
        if p.is_dir() {
            list.extend(images_in_dir(&p));
        } else if is_image(&p) {
            list.push(p);
        }
    }
    list
}

fn images_in_dir(dir: &Path) -> Vec<PathBuf> {
    match fs::read_dir(dir) {
        Ok(rd) => {
            let mut v: Vec<PathBuf> = rd
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.is_file() && is_image(p))
                .collect();
            v.sort();
            v
        }
        Err(_) => Vec::new(),
    }
}

fn is_image(p: &Path) -> bool {
    p.extension()
        .and_then(|e| e.to_str())
        .map(|e| IMAGE_EXTS.contains(&e.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}

fn decode(path: &Path) -> image::RgbaImage {
    decode_ok(path).unwrap_or_else(|| {
        eprintln!("cannot decode {}", path.display());
        std::process::exit(1);
    })
}

fn decode_ok(path: &Path) -> Option<image::RgbaImage> {
    match image::ImageReader::open(path) {
        Ok(reader) => match reader.decode() {
            Ok(img) => Some(img.to_rgba8()),
            Err(_) => None,
        },
        Err(_) => None,
    }
}

fn bg_clear() -> [f32; 4] {
    let dir = match std::env::var("states2") {
        Ok(d) if !d.is_empty() => d,
        _ => return [0.10, 0.11, 0.14, 1.0],
    };
    let path = std::path::Path::new(&dir).join("shell_vars.json");
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(_) => return [0.10, 0.11, 0.14, 1.0],
    };
    let json: serde_json::Value = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(_) => return [0.10, 0.11, 0.14, 1.0],
    };
    let fetch = |key: &str| -> Option<(f32, f32, f32)> {
        let line = json.get(key)?.as_str()?;
        let hex = line.trim_start_matches('#');
        if hex.len() != 6 {
            return None;
        }
        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
        Some((r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0))
    };
    let (r, g, b) = fetch("bg").unwrap_or((0.10, 0.11, 0.14));
    [r, g, b, 1.0]
}