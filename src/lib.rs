pub mod autorun;
pub mod config;
pub mod core;
pub mod hotkeys;
pub mod ipc;
pub mod utils;

// Windows-only modules
#[cfg(windows)]
pub mod overlay;
#[cfg(windows)]
pub mod tray;
#[cfg(windows)]
pub mod ui;
#[cfg(windows)]
pub mod vd;
