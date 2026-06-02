//! Plugin SDK domain module shared by the host and runtime-loaded plugins.
//! `api` exposes the C ABI vtable, `types` defines shared plugin types,
//! `events` drives host→plugin event dispatch, and `overlay`, `menu`,
//! `notification` own plugin-driven GUI state.
//! `mod.rs` re-exports the public surface used by the rest of core.

use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

pub mod api;
pub mod career;
pub mod events;
pub mod menu;
pub mod notification;
pub mod overlay;
pub mod types;

pub use hachimi_plugin_abi::Vtable;
pub use hachimi_plugin_abi::API_VERSION;
pub use types::{GuiMenuCallback, GuiMenuSectionCallback, GuiUiCallback, HachimiInitFn, InitResult, Plugin};

/// Monotonic registration/subscription handle counter. 0 is reserved for "failure".
static HANDLE_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Serializes tests that touch process-global plugin state (`CURRENT_OWNER` and
/// the registration/subscription registries) so they don't interfere when the test
/// harness runs them in parallel.
#[cfg(test)]
pub(crate) static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Allocate a fresh non-zero handle for a registration or event subscription.
pub(crate) fn next_handle() -> u64 {
    HANDLE_COUNTER.fetch_add(1, Ordering::Relaxed)
}

/// Remove any GUI registration (menu item/section/overlay) by handle.
pub(crate) fn unregister(handle: u64) -> bool {
    let mut removed = menu::remove_by_handle(handle);
    removed |= overlay::remove_by_handle(handle);
    removed
}

/// Owner id of the plugin whose code is currently running (init or a host
/// callback). `0` means "host / no plugin". Registrations and event subscriptions
/// are tagged with this so a plugin's callbacks can be reclaimed before its DLL is
/// unloaded.
static CURRENT_OWNER: AtomicU32 = AtomicU32::new(0);

/// The owner id attributed to registrations made right now.
pub(crate) fn current_owner() -> u32 {
    CURRENT_OWNER.load(Ordering::Relaxed)
}

/// RAII guard that sets [`CURRENT_OWNER`] for the duration of a plugin call and
/// restores the previous owner on drop (so nested host→plugin calls compose).
///
/// Owner scoping is single-global and assumes plugin code runs on the game/render
/// threads serially; registrations from a plugin-spawned background thread would be
/// mis-attributed and are not supported.
pub(crate) struct OwnerScope(u32);

impl OwnerScope {
    pub(crate) fn enter(owner: u32) -> Self {
        Self(CURRENT_OWNER.swap(owner, Ordering::Relaxed))
    }
}

impl Drop for OwnerScope {
    fn drop(&mut self) {
        CURRENT_OWNER.store(self.0, Ordering::Relaxed);
    }
}

/// Tear down everything a plugin owns: dispatch `SHUTDOWN` to its event
/// subscriptions, then drop those subscriptions and all of its GUI registrations.
///
/// This makes it safe to release the plugin's GUI/event callbacks. It does **not**
/// remove IL2CPP hooks the plugin installed via the interceptor — the host cannot
/// know about those, so a plugin that hooks game code MUST unhook in its `SHUTDOWN`
/// handler before its DLL is unloaded.
pub(crate) fn teardown_owner(owner: u32) {
    events::shutdown_and_remove_owner(owner);
    menu::remove_by_owner(owner);
    overlay::remove_by_owner(owner);
}

/// Unload a single plugin by library name. Windows-only; returns `false` if not loaded.
#[cfg(target_os = "windows")]
pub fn unload_by_name(name: &str) -> bool {
    crate::windows::main::unload_plugin(name)
}

#[cfg(not(target_os = "windows"))]
pub fn unload_by_name(_name: &str) -> bool {
    false
}
