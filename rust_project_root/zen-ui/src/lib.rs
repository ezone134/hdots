//! Shared zen UI libs — extracted from `zen-shell` so every project in
//! `rust_project_root/` speaks the same primitives (`Cmd`, `Pal`,
//! `TextEngine`, widget `Node`s, the icon constants).

pub mod cmd;
pub mod icons;
pub mod text;
pub mod ui;
pub mod widgets;

/// Re-exported so downstream zen projects can build a [`text::TextEngine`] or
/// call [`text::rasterize_family`] without pinning their own `cosmic-text`.
pub use cosmic_text;

/// Shared color primitives (hex parsing, blending, HSV) — re-exported so UI
/// consumers get one implementation and need not pin their own `zen-color`.
pub use zen_color;
