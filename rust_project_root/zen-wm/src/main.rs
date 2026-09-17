mod config;
mod element;
mod focus;
mod keybindings;
mod overview;
mod renderer;
mod saturation;
mod state;
mod workspace;

use std::{path::PathBuf, sync::atomic::Ordering, time::Duration};

use smithay::{
    backend::{
        renderer::{gles::GlesRenderer, ImportEgl, ImportMemWl},
        winit,
    },
    backend::SwapBuffersError,
    output::{Mode, Output, PhysicalProperties, Subpixel},
    reexports::{
        calloop::{generic::Generic, EventLoop, Interest, Mode as CalloopMode, PostAction},
        wayland_server::Display,
        winit::platform::pump_events::PumpStatus,
    },
    utils::{Point, Transform},
    wayland::socket::ListeningSocketSource,
};

use state::ZenWm;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let config_path = config_path();
    let config = config::Config::load(&config_path);
    tracing::info!(path = %config_path.display(), "loaded config");

    let mut event_loop = EventLoop::try_new().unwrap();
    let display: Display<ZenWm> = Display::new().unwrap();
    let display_handle = display.handle();

    let (mut backend, mut winit) = match winit::init::<GlesRenderer>() {
        Ok(ret) => ret,
        Err(err) => {
            tracing::error!(error = %err, "failed to initialize winit backend");
            return;
        }
    };
    let size = backend.window_size();

    // Wayland socket for clients.
    let source = ListeningSocketSource::new_auto().unwrap();
    let socket_name = source.socket_name().to_string_lossy().into_owned();
    event_loop
        .handle()
        .insert_source(source, |client_stream, _, data: &mut ZenWm| {
            if let Err(err) = data
                .display_handle
                .insert_client(client_stream, std::sync::Arc::new(state::ClientState::default()))
            {
                tracing::warn!(error = %err, "failed to add wayland client");
            }
        })
        .expect("failed to init wayland socket source");
    tracing::info!(socket = %socket_name, "listening on wayland socket");

    // Dispatch client messages whenever the display is readable.
    event_loop
        .handle()
        .insert_source(
            Generic::new(display, Interest::READ, CalloopMode::Level),
            |_, display, data: &mut ZenWm| {
                // Safety: we do not drop the display.
                unsafe { display.get_mut().dispatch_clients(data).unwrap() };
                Ok(PostAction::Continue)
            },
        )
        .expect("failed to init wayland server source");

    let mode = Mode {
        size,
        refresh: 60_000,
    };
    let output = Output::new(
        "winit".to_string(),
        PhysicalProperties {
            size: (0, 0).into(),
            subpixel: Subpixel::Unknown,
            make: "zen-wm".into(),
            model: "winit".into(),
        },
    );
    let _global = output.create_global::<ZenWm>(&display_handle);
    output.change_current_state(Some(mode), Some(Transform::Normal), None, Some(Point::from((0, 0))));
    output.set_preferred(mode);

    // Expose EGL (wl_drm) to clients so GPU clients (e.g. alacritty) can render.
    // Must happen before the display handle is moved into the state.
    match backend.renderer().bind_wl_display(&display_handle) {
        Ok(()) => tracing::info!("bound EGL display to the wayland server (wl_drm exposed)"),
        Err(err) => tracing::warn!(error = %err, "failed to bind EGL display, GPU clients will not render"),
    }

    let mut state = ZenWm::new(display_handle, event_loop.handle(), output.clone(), config);
    state.socket_name = Some(socket_name);
    // Seed the config mtime so the startup load isn't treated as a reload.
    state.config_mtime = std::fs::metadata(&config_path).ok().and_then(|m| m.modified().ok());
    state
        .shm_state
        .update_formats(backend.renderer().shm_formats().collect::<Vec<_>>());
    state.workspaces.map_output(&output);
    state.saturation = state.config.saturation;
    state.saturation_program = saturation::compile_saturation_program(backend.renderer());

    // Hyprland-style exec-once: launch [autostart].exec commands.
    state.autostart();

    tracing::info!("zen-wm started on the winit backend");

    while state.running.load(Ordering::SeqCst) {
        let status = winit.dispatch_new_events(|event| match event {
            winit::WinitEvent::Resized { size, .. } => {
                let mode = Mode {
                    size,
                    refresh: 60_000,
                };
                output.change_current_state(Some(mode), None, None, None);
                output.set_preferred(mode);
                state.workspaces.map_output(&output);
                state.request_repaint();
            }
            winit::WinitEvent::Input(event) => state.process_input_event_windowed(event),
            _ => (),
        });

        if let PumpStatus::Exit(_) = status {
            state.running.store(false, Ordering::SeqCst);
            break;
        }

        if state.needs_repaint {
            match renderer::render_once(&mut state, &mut backend) {
                Ok(()) => state.needs_repaint = false,
                Err(SwapBuffersError::ContextLost(err)) => {
                    tracing::error!(error = %err, "graphics context lost, exiting");
                    state.running.store(false, Ordering::SeqCst);
                    break;
                }
                Err(err) => tracing::warn!(error = %err, "render error"),
            }
        }

        // Live config reload: hot-applies changes to ~/.config/zen-wm/config.toml.
        state.check_config_reload(&config_path);

        let result = event_loop.dispatch(Some(Duration::from_millis(1)), &mut state);
        if result.is_err() {
            state.running.store(false, Ordering::SeqCst);
        } else {
            if state.workspaces.refresh() {
                state.request_repaint();
            }
            state.popups.cleanup();
            if let Err(err) = state.display_handle.flush_clients() {
                tracing::warn!(error = %err, "failed to flush clients");
            }
        }
    }

    tracing::info!("zen-wm exiting");
}

fn config_path() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(|p| PathBuf::from(p).join("zen-wm").join("config.toml"))
        .unwrap_or_else(|| {
            std::env::var_os("HOME")
                .map(|h| PathBuf::from(h).join(".config/zen-wm/config.toml"))
                .unwrap_or_default()
        })
}
