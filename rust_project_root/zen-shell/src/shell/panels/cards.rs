//! Per-card behaviour helpers: input state machines, backend pollers and
//! pure math. One file — each former `shell/panels/cards/<name>.rs` lives here
//! as an inline `mod` so its logical path
//! (`crate::shell::panels::cards::<name>`) is unchanged and no `use` moved.

mod accent;

mod mirror;

mod shared;

mod sliders;

mod audiodevice;

mod audiorec;

pub(crate) mod latency;

mod powerdraw;

mod weather;

mod worldclock;

pub(crate) use worldclock::fmt_hhmm_min;

mod worldmap;
