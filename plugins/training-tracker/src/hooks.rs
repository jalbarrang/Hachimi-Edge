//! Host lifecycle subscription.
//!
//! The plugin reads career state directly from game memory (see `memory_reader`),
//! so it no longer tracks training commands or career start/end. The only event we
//! still need is [`event::SHUTDOWN`], to remove the IL2CPP hooks we installed so the
//! host can safely unload this DLL (UNLOADABLE).

use std::ffi::c_void;

use hachimi_plugin_abi::event;
use hachimi_plugin_sdk::Sdk;

/// Fired before the host unloads this plugin (or on process detach). Remove every
/// IL2CPP hook we installed so the host can safely free the DLL (UNLOADABLE).
extern "C" fn on_shutdown(_event_id: u32, _data: *const c_void, _userdata: *mut c_void) {
    crate::memory_reader::stop_tracking();
    crate::overlay_cache::shutdown();
    crate::shop_hooks::uninstall_shop_hooks();
    hlog_info!("Shutdown: tracking stopped, hooks removed");
}

/// Subscribe to the host events we need. Returns `true` if the host advertises the
/// events capability (required for the shutdown teardown).
pub fn subscribe_events() -> bool {
    let sdk = Sdk::get();
    if !sdk.has_capability(hachimi_plugin_abi::capability::EVENTS) {
        hlog_warn!("Host does not advertise the EVENTS capability");
        return false;
    }
    sdk.on(event::SHUTDOWN, on_shutdown, std::ptr::null_mut());
    true
}
