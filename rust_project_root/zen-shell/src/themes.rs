//! Theme state — reads the master theme file and the live shell colors.
//!
//! The implementation now lives in the shared **`zen-theme`** crate (the live
//! `$states2/shell_vars.json` palette reader is consumed by `gcp`, `sp`,
//! `zen-image`, and the shell alike), so this module is a thin re-export that
//! keeps the existing `crate::themes::…` call sites working.
//!
//! Colors are `0xRRGGBBAA` throughout (see `zen-color`).

pub use zen_theme::*;
