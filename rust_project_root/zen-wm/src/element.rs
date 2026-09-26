use std::borrow::Cow;

use smithay::{
    backend::renderer::{
        element::{surface::WaylandSurfaceRenderElement, AsRenderElements},
        ImportAll, ImportMem, Renderer, Texture,
    },
    desktop::{space::SpaceElement, Window},
    output::Output,
    render_elements,
    reexports::wayland_server::protocol::wl_surface::WlSurface,
    utils::{IsAlive, Logical, Physical, Point, Rectangle, Scale},
    wayland::seat::WaylandFocus,
};

/// Thin wrapper around a [`Window`] so it can live inside a [`Space`].
#[derive(Debug, Clone, PartialEq)]
pub struct WindowElement(pub Window);

impl IsAlive for WindowElement {
    #[inline]
    fn alive(&self) -> bool {
        self.0.alive()
    }
}

impl SpaceElement for WindowElement {
    fn geometry(&self) -> Rectangle<i32, Logical> {
        SpaceElement::geometry(&self.0)
    }
    fn bbox(&self) -> Rectangle<i32, Logical> {
        SpaceElement::bbox(&self.0)
    }
    fn is_in_input_region(&self, point: &Point<f64, Logical>) -> bool {
        SpaceElement::is_in_input_region(&self.0, point)
    }
    fn z_index(&self) -> u8 {
        SpaceElement::z_index(&self.0)
    }
    fn set_activate(&self, activated: bool) {
        SpaceElement::set_activate(&self.0, activated);
    }
    fn output_enter(&self, output: &Output, overlap: Rectangle<i32, Logical>) {
        SpaceElement::output_enter(&self.0, output, overlap);
    }
    fn output_leave(&self, output: &Output) {
        SpaceElement::output_leave(&self.0, output);
    }
    fn refresh(&self) {
        SpaceElement::refresh(&self.0);
    }
}

impl WindowElement {
    pub fn wl_surface(&self) -> Option<Cow<'_, WlSurface>> {
        self.0.wl_surface()
    }

    pub fn with_surfaces<F: FnMut(&WlSurface, &smithay::wayland::compositor::SurfaceData)>(&self, f: F) {
        self.0.with_surfaces(f)
    }

    /// Configure this window to the given tile geometry.
    pub fn configure(&self, rect: Rectangle<i32, Logical>) {
        if let Some(toplevel) = self.0.toplevel() {
            toplevel.with_pending_state(|state| {
                state.size = Some(rect.size);
                state.bounds = Some(rect.size);
            });
            if toplevel.is_initial_configure_sent() {
                toplevel.send_pending_configure();
            } else {
                toplevel.send_configure();
            }
        }
    }

    pub fn close(&self) {
        if let Some(toplevel) = self.0.toplevel() {
            toplevel.send_close();
        }
    }
}

render_elements! {
    pub WindowRenderElement<R> where R: ImportAll + ImportMem;
    Window=WaylandSurfaceRenderElement<R>,
}

impl<R> AsRenderElements<R> for WindowElement
where
    R: Renderer + ImportAll + ImportMem,
    R::TextureId: Clone + Texture + 'static,
{
    type RenderElement = WindowRenderElement<R>;

    fn render_elements<C: From<Self::RenderElement>>(
        &self,
        renderer: &mut R,
        location: Point<i32, Physical>,
        scale: Scale<f64>,
        alpha: f32,
    ) -> Vec<C> {
        AsRenderElements::render_elements(&self.0, renderer, location, scale, alpha)
            .into_iter()
            .map(C::from)
            .collect()
    }
}
