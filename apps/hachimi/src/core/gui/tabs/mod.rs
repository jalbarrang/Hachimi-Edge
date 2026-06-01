//! Control Center tab bodies. Each tab is an `impl Gui` method living in its own
//! file to keep `menu.rs` (the shell) small.

mod about_tab;
mod config;
mod overlay_tab;
mod plugins;
mod translations;
