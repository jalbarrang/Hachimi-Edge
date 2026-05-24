//! Plugin SDK domain module shared by the host and runtime-loaded plugins.
//! `api` exposes the C ABI vtable, `types` defines shared plugin types,
//! and `overlay`, `menu`, and `notification` own plugin-driven GUI state.
//! `mod.rs` re-exports the public surface used by the rest of core.

pub mod api;
pub mod menu;
pub mod notification;
pub mod overlay;
pub mod types;

pub use hachimi_plugin_abi::Vtable;
pub use hachimi_plugin_abi::API_VERSION;
pub use types::{GuiMenuCallback, GuiMenuSectionCallback, GuiUiCallback, HachimiInitFn, InitResult, Plugin};
