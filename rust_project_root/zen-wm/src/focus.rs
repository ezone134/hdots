use std::borrow::Cow;

use smithay::{
    desktop::{LayerSurface, PopupKind},
    input::{
        keyboard::{KeyboardTarget, KeysymHandle, ModifiersState},
        pointer::{AxisFrame, ButtonEvent, MotionEvent, PointerTarget, RelativeMotionEvent},
        touch::TouchTarget,
        Seat,
    },
    reexports::wayland_server::{
        backend::ObjectId,
        protocol::wl_surface::WlSurface,
        Resource,
    },
    utils::{IsAlive, Serial},
    wayland::seat::WaylandFocus,
};

use crate::{element::WindowElement, state::ZenWm};

/// Target of the keyboard/pointer/touch focus.
#[derive(Debug, Clone, PartialEq)]
pub enum FocusTarget {
    Window(WindowElement),
    LayerSurface(LayerSurface),
    Popup(PopupKind),
}

impl FocusTarget {
    pub fn wl_surface(&self) -> Option<Cow<'_, WlSurface>> {
        match self {
            FocusTarget::Window(w) => w.wl_surface(),
            FocusTarget::LayerSurface(l) => Some(Cow::Borrowed(l.wl_surface())),
            FocusTarget::Popup(p) => Some(Cow::Borrowed(p.wl_surface())),
        }
    }

    pub fn object_id(&self) -> ObjectId {
        self.wl_surface().unwrap().id()
    }
}

impl IsAlive for FocusTarget {
    #[inline]
    fn alive(&self) -> bool {
        match self {
            FocusTarget::Window(w) => w.alive(),
            FocusTarget::LayerSurface(l) => l.alive(),
            FocusTarget::Popup(p) => p.alive(),
        }
    }
}

impl WaylandFocus for FocusTarget {
    fn wl_surface(&self) -> Option<Cow<'_, WlSurface>> {
        FocusTarget::wl_surface(self)
    }
}

impl From<PopupKind> for FocusTarget {
    fn from(popup: PopupKind) -> Self {
        FocusTarget::Popup(popup)
    }
}

impl From<FocusTarget> for WlSurface {
    #[inline]
    fn from(target: FocusTarget) -> Self {
        target.wl_surface().unwrap().into_owned()
    }
}

impl PointerTarget<ZenWm> for FocusTarget {
    fn enter(&self, seat: &Seat<ZenWm>, data: &mut ZenWm, event: &MotionEvent) {
        if let Some(surface) = self.wl_surface() {
            PointerTarget::enter(&*surface, seat, data, event);
        }
    }
    fn motion(&self, seat: &Seat<ZenWm>, data: &mut ZenWm, event: &MotionEvent) {
        if let Some(surface) = self.wl_surface() {
            PointerTarget::motion(&*surface, seat, data, event);
        }
    }
    fn relative_motion(&self, seat: &Seat<ZenWm>, data: &mut ZenWm, event: &RelativeMotionEvent) {
        if let Some(surface) = self.wl_surface() {
            PointerTarget::relative_motion(&*surface, seat, data, event);
        }
    }
    fn button(&self, seat: &Seat<ZenWm>, data: &mut ZenWm, event: &ButtonEvent) {
        if let Some(surface) = self.wl_surface() {
            PointerTarget::button(&*surface, seat, data, event);
        }
    }
    fn axis(&self, seat: &Seat<ZenWm>, data: &mut ZenWm, frame: AxisFrame) {
        if let Some(surface) = self.wl_surface() {
            PointerTarget::axis(&*surface, seat, data, frame);
        }
    }
    fn frame(&self, seat: &Seat<ZenWm>, data: &mut ZenWm) {
        if let Some(surface) = self.wl_surface() {
            PointerTarget::frame(&*surface, seat, data);
        }
    }
    fn leave(&self, seat: &Seat<ZenWm>, data: &mut ZenWm, serial: Serial, time: u32) {
        if let Some(surface) = self.wl_surface() {
            PointerTarget::leave(&*surface, seat, data, serial, time);
        }
    }
    fn gesture_swipe_begin(&self, seat: &Seat<ZenWm>, data: &mut ZenWm, event: &smithay::input::pointer::GestureSwipeBeginEvent) {
        if let Some(surface) = self.wl_surface() {
            PointerTarget::gesture_swipe_begin(&*surface, seat, data, event);
        }
    }
    fn gesture_swipe_update(&self, seat: &Seat<ZenWm>, data: &mut ZenWm, event: &smithay::input::pointer::GestureSwipeUpdateEvent) {
        if let Some(surface) = self.wl_surface() {
            PointerTarget::gesture_swipe_update(&*surface, seat, data, event);
        }
    }
    fn gesture_swipe_end(&self, seat: &Seat<ZenWm>, data: &mut ZenWm, event: &smithay::input::pointer::GestureSwipeEndEvent) {
        if let Some(surface) = self.wl_surface() {
            PointerTarget::gesture_swipe_end(&*surface, seat, data, event);
        }
    }
    fn gesture_pinch_begin(&self, seat: &Seat<ZenWm>, data: &mut ZenWm, event: &smithay::input::pointer::GesturePinchBeginEvent) {
        if let Some(surface) = self.wl_surface() {
            PointerTarget::gesture_pinch_begin(&*surface, seat, data, event);
        }
    }
    fn gesture_pinch_update(&self, seat: &Seat<ZenWm>, data: &mut ZenWm, event: &smithay::input::pointer::GesturePinchUpdateEvent) {
        if let Some(surface) = self.wl_surface() {
            PointerTarget::gesture_pinch_update(&*surface, seat, data, event);
        }
    }
    fn gesture_pinch_end(&self, seat: &Seat<ZenWm>, data: &mut ZenWm, event: &smithay::input::pointer::GesturePinchEndEvent) {
        if let Some(surface) = self.wl_surface() {
            PointerTarget::gesture_pinch_end(&*surface, seat, data, event);
        }
    }
    fn gesture_hold_begin(&self, seat: &Seat<ZenWm>, data: &mut ZenWm, event: &smithay::input::pointer::GestureHoldBeginEvent) {
        if let Some(surface) = self.wl_surface() {
            PointerTarget::gesture_hold_begin(&*surface, seat, data, event);
        }
    }
    fn gesture_hold_end(&self, seat: &Seat<ZenWm>, data: &mut ZenWm, event: &smithay::input::pointer::GestureHoldEndEvent) {
        if let Some(surface) = self.wl_surface() {
            PointerTarget::gesture_hold_end(&*surface, seat, data, event);
        }
    }
}

impl KeyboardTarget<ZenWm> for FocusTarget {
    fn enter(&self, seat: &Seat<ZenWm>, data: &mut ZenWm, keys: Vec<KeysymHandle<'_>>, serial: Serial) {
        if let Some(surface) = self.wl_surface() {
            KeyboardTarget::enter(&*surface, seat, data, keys, serial);
        }
    }
    fn leave(&self, seat: &Seat<ZenWm>, data: &mut ZenWm, serial: Serial) {
        if let Some(surface) = self.wl_surface() {
            KeyboardTarget::leave(&*surface, seat, data, serial);
        }
    }
    fn key(&self, seat: &Seat<ZenWm>, data: &mut ZenWm, key: KeysymHandle<'_>, state: smithay::backend::input::KeyState, serial: Serial, time: u32) {
        if let Some(surface) = self.wl_surface() {
            KeyboardTarget::key(&*surface, seat, data, key, state, serial, time);
        }
    }
    fn modifiers(&self, seat: &Seat<ZenWm>, data: &mut ZenWm, modifiers: ModifiersState, serial: Serial) {
        if let Some(surface) = self.wl_surface() {
            KeyboardTarget::modifiers(&*surface, seat, data, modifiers, serial);
        }
    }
}

impl TouchTarget<ZenWm> for FocusTarget {
    fn down(&self, seat: &Seat<ZenWm>, data: &mut ZenWm, event: &smithay::input::touch::DownEvent, seq: Serial) {
        if let Some(surface) = self.wl_surface() {
            TouchTarget::down(&*surface, seat, data, event, seq);
        }
    }
    fn up(&self, seat: &Seat<ZenWm>, data: &mut ZenWm, event: &smithay::input::touch::UpEvent, seq: Serial) {
        if let Some(surface) = self.wl_surface() {
            TouchTarget::up(&*surface, seat, data, event, seq);
        }
    }
    fn motion(&self, seat: &Seat<ZenWm>, data: &mut ZenWm, event: &smithay::input::touch::MotionEvent, seq: Serial) {
        if let Some(surface) = self.wl_surface() {
            TouchTarget::motion(&*surface, seat, data, event, seq);
        }
    }
    fn frame(&self, seat: &Seat<ZenWm>, data: &mut ZenWm, seq: Serial) {
        if let Some(surface) = self.wl_surface() {
            TouchTarget::frame(&*surface, seat, data, seq);
        }
    }
    fn cancel(&self, seat: &Seat<ZenWm>, data: &mut ZenWm, seq: Serial) {
        if let Some(surface) = self.wl_surface() {
            TouchTarget::cancel(&*surface, seat, data, seq);
        }
    }
    fn shape(&self, seat: &Seat<ZenWm>, data: &mut ZenWm, event: &smithay::input::touch::ShapeEvent, seq: Serial) {
        if let Some(surface) = self.wl_surface() {
            TouchTarget::shape(&*surface, seat, data, event, seq);
        }
    }
    fn orientation(&self, seat: &Seat<ZenWm>, data: &mut ZenWm, event: &smithay::input::touch::OrientationEvent, seq: Serial) {
        if let Some(surface) = self.wl_surface() {
            TouchTarget::orientation(&*surface, seat, data, event, seq);
        }
    }
}
