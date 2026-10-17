//! Shared zen renderers — extracted from `zen-shell` so every project in
//! `rust_project_root/` renders through one implementation.
//!
//! Two backends, same scene API (rounded-rect SDF solid pass + premultiplied
//! glyph-texture pass), selected per consumer:
//!  - `gl` (default): EGL/GLES3-or-desktop-GL, via [`gl`].
//!  - `vk`: Vulkan, via [`vk`] (opt-in — pulls `ash` + `naga`).
//!
//! `Tex` is backend-neutral: `id` is a GL texture name or a Vulkan slot index
//! depending on which backend produced it.

/// Backend-neutral texture handle: `id` is backend-specific (GL texture name,
/// Vulkan texture slot) but the cache key/type is shared.
#[derive(Clone, Copy)]
pub struct Tex {
    pub id: u64,
    pub w: u32,
    pub h: u32,
}

#[cfg(feature = "gl")]
pub mod gl;

#[cfg(feature = "gl")]
pub use gl::{Gl, is_software_gl};

#[cfg(feature = "vk")]
pub mod vk;

#[cfg(feature = "vk")]
pub use vk::Vk;
