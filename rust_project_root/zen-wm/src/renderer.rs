//! Custom render loop for zen-wm.
//!
//! We do *not* use `OutputDamageTracker`/`space::render_output` because those own
//! the frame and would prevent injecting the saturation shader override. Instead
//! we replicate the damage-tracker's draw order: elements come back-to-front from
//! `space_render_elements` (upper layer surfaces, windows, lower layer surfaces)
//! so we draw them in `.rev()`.
//!
//! V1 strategy: full-window redraw on every `needs_repaint`, then present.

use smithay::{
    backend::{
        allocator::Fourcc,
        renderer::{
            element::{
                memory::{MemoryRenderBuffer, MemoryRenderBufferRenderElement},
                solid::SolidColorRenderElement,
                Element, Kind, RenderElement,
            },
            gles::{GlesError, GlesFrame, GlesRenderer, GlesTexture},
            utils::{import_surface_tree, with_renderer_surface_state},
            Color32F, Frame, ImportAll, ImportMem, Renderer, Texture,
        },
        winit::WinitGraphicsBackend,
        SwapBuffersError,
    },
    desktop::space::{space_render_elements, SpaceRenderElements},
    render_elements,
    utils::{Logical, Physical, Point, Rectangle, Scale, Size, Transform},
};

use crate::{
    element::WindowRenderElement,
    saturation::apply_saturation,
    state::ZenWm,
};

render_elements! {
    pub ZenRenderElement<R> where R: ImportAll + ImportMem;
    Window=WindowRenderElement<R>,
    Solid=SolidColorRenderElement,
    Memory=MemoryRenderBufferRenderElement<R>,
}

type ZenSpaceElements<'a> = SpaceRenderElements<GlesRenderer, WindowRenderElement<GlesRenderer>>;

/// Build a 16x16 diagonal-arrow cursor (white with a 1px black outline).
pub fn arrow_cursor_buffer() -> MemoryRenderBuffer {
    const SIZE: i32 = 16;
    let mut bgra = vec![0u8; (SIZE * SIZE * 4) as usize];
    for y in 0..SIZE {
        for x in 0..SIZE {
            let diag = x == y;
            let top = y == 0;
            let left = x == 0;
            let inner = top || left || diag;
            // Black outline just below/right of the white shape.
            let outline = (x == 1 && y >= 1) || (y == 1 && x >= 1) || (x == y + 1 && x >= 2);
            let idx = ((y * SIZE + x) * 4) as usize;
            if outline {
                bgra[idx] = 0;
                bgra[idx + 1] = 0;
                bgra[idx + 2] = 0;
                bgra[idx + 3] = 255;
            } else if inner {
                bgra[idx] = 255;
                bgra[idx + 1] = 255;
                bgra[idx + 2] = 255;
                bgra[idx + 3] = 255;
            }
        }
    }
    MemoryRenderBuffer::from_slice(&bgra, Fourcc::Abgr8888, (SIZE, SIZE), 1, Transform::Normal, None)
}

fn to_swap_error(e: GlesError) -> SwapBuffersError {
    SwapBuffersError::TemporaryFailure(Box::new(e))
}

pub fn render_once(
    state: &mut ZenWm,
    backend: &mut WinitGraphicsBackend<GlesRenderer>,
) -> Result<(), SwapBuffersError> {
    let output = state.output.clone();
    let scale = Scale::from(output.current_scale().fractional_scale());
    let transform = output.current_transform();
    let output_size = backend.window_size();
    let overview_active = state.overview.active;
    tracing::debug!(
        ?output_size,
        overview_active,
        sat = state.saturation,
        "render_once"
    );

    // ── build elements + draw (bind borrows `backend`; dropped at scope end) ──
    {
        let (renderer, mut fb) = backend.bind()?;
        let mut elements: Vec<ZenSpaceElements<'_>> = Vec::new();

    if !overview_active {
        let ws = state.workspaces.active_workspace();
        let space_elements: Vec<ZenSpaceElements<'_>> =
            space_render_elements(renderer, std::slice::from_ref(&ws.space), &output, 1.0)
                .map_err(|_| SwapBuffersError::TemporaryFailure("output not mapped".into()))?;
        elements.extend(space_elements);
    } else {
        // Import window textures for the overview grid thumbnails.
        for ws in &state.workspaces.list {
            for w in ws.space.elements() {
                if let Some(surface) = w.wl_surface() {
                    let surface = surface.into_owned();
                    import_surface_tree(renderer, &surface).map_err(to_swap_error)?;
                }
            }
        }
    }

    // Cursor (drawn on top of everything, last).
    let cursor_loc: Point<f64, Physical> = state.cursor_location.to_physical(scale);
    let cursor_pos = cursor_loc - Point::from((1.0, 1.0));
    let cursor = MemoryRenderBufferRenderElement::from_buffer(
        renderer,
        cursor_pos,
        &state.cursor,
        Some(1.0),
        None,
        None,
        Kind::Unspecified,
    )
    .map_err(to_swap_error)?;

    // ── frame ──
    let mut frame = renderer.render(&mut fb, output_size, transform).map_err(to_swap_error)?;

    let damage = vec![Rectangle::from_size(output_size)];
    // Background from config (hot-reloadable).
    let bg = state.config.background;
    frame
        .clear(
            Color32F::new(bg[0] as f32 / 255.0, bg[1] as f32 / 255.0, bg[2] as f32 / 255.0, 1.0),
            &damage,
        )
        .map_err(to_swap_error)?;

    if let Some(program) = &state.saturation_program {
        apply_saturation(&mut frame, program, state.saturation);
    }

    // Draw elements back-to-front (elements were built front-to-back).
    for element in elements.iter().rev() {
        let geometry = element.geometry(scale);
        let src = element.src();
        element
            .draw(&mut frame, src, geometry, &damage, &[])
            .map_err(to_swap_error)?;
    }

    if overview_active {
        draw_overview(&mut frame, state, scale, output_size).map_err(to_swap_error)?;
    }

    cursor
        .draw(
            &mut frame,
            cursor.src(),
            cursor.geometry(scale),
            &damage,
            &[],
        )
        .map_err(to_swap_error)?;

    let _ = frame.finish().map_err(to_swap_error)?;
    }
    let damage = vec![Rectangle::from_size(output_size)];
    backend.submit(Some(&damage))?;
    Ok(())
}

/// Draw the workspace overview grid: a dim background, scaled window thumbnails
/// per workspace, and a highlight on the selected tile.
fn draw_overview(
    frame: &mut GlesFrame<'_, '_>,
    state: &ZenWm,
    scale: Scale<f64>,
    output_size: Size<i32, Physical>,
) -> Result<(), GlesError> {
    let output_logical: Size<f64, Logical> = output_size.to_f64().to_logical(scale);
    let count = state.workspaces.count();
    if count == 0 {
        return Ok(());
    }
    let cols = state.config.workspace_columns.max(1) as usize;
    let rows = state.config.workspace_rows.max(1) as usize;

    let margin = (output_logical.w * 0.08).min(96.0);
    let grid_w = (output_logical.w - 2.0 * margin).max(1.0);
    let grid_h = (output_logical.h - 2.0 * margin).max(1.0);
    let cell_w = grid_w / cols as f64;
    let cell_h = grid_h / rows as f64;

    // Dim everything behind the grid.
    let dim = state.config.overview_dim * 0.6;
    let dim_color = Color32F::new(0.0, 0.0, 0.0, dim);
    frame.draw_solid(Rectangle::from_size(output_size), &[Rectangle::from_size(output_size)], dim_color)?;

    for i in 0..count {
        let col = i % cols;
        let row = i / cols;
        let tile_loc = Point::from((margin + col as f64 * cell_w, margin + row as f64 * cell_h));
        let tile = Rectangle::new(
            tile_loc.to_physical(scale).to_i32_round(),
            Size::from((cell_w, cell_h)).to_physical(scale).to_i32_round(),
        );

        let bg = if i == state.overview.selected {
            Color32F::new(
                state.config.overview_highlight[0] as f32 / 255.0,
                state.config.overview_highlight[1] as f32 / 255.0,
                state.config.overview_highlight[2] as f32 / 255.0,
                1.0,
            )
        } else {
            Color32F::new(
                state.config.overview_bg[0] as f32 / 255.0,
                state.config.overview_bg[1] as f32 / 255.0,
                state.config.overview_bg[2] as f32 / 255.0,
                1.0,
            )
        };
        frame.draw_solid(tile, &[tile], bg)?;

        // Frontmost window thumbnail, letterboxed inside the tile.
        if let Some(window) = state.workspaces.list[i].space.elements().next_back() {
            if let Some(surface) = window.wl_surface() {
                let texture = with_renderer_surface_state(&surface, |s| {
                    s.texture::<GlesTexture>(frame.context_id()).cloned()
                })
                .flatten();
                if let Some(texture) = texture {
                    let tex_w = texture.width() as f64;
                    let tex_h = texture.height() as f64;
                    if tex_w > 0.0 && tex_h > 0.0 {
                        let inner_pad = 4;
                        let avail_w = tile.size.w - 2 * inner_pad;
                        let avail_h = tile.size.h - 2 * inner_pad;
                        let scale_f = (avail_w as f64 / tex_w).min(avail_h as f64 / tex_h).min(1.0);
                        let draw_w = (tex_w * scale_f) as i32;
                        let draw_h = (tex_h * scale_f) as i32;
                        let draw_loc = Point::from((
                            tile.loc.x + inner_pad + (avail_w - draw_w) / 2,
                            tile.loc.y + inner_pad + (avail_h - draw_h) / 2,
                        ));
                        let dst = Rectangle::new(
                            Point::from(draw_loc),
                            Size::from((draw_w, draw_h)),
                        );
                        let src = Rectangle::from_size(Size::from((tex_w, tex_h)).to_f64());
                        frame.render_texture_from_to(
                            &texture,
                            src,
                            dst,
                            &[dst],
                            &[],
                            Transform::Normal,
                            1.0,
                            None,
                            &[],
                        )?;
                    }
                }
            }
        }
    }

    Ok(())
}
