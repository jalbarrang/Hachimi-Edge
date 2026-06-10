//! Host event subscriptions: teardown on shutdown.
//!
//! The 100ms live cadence lives in the RaceManager hook (see `capture.rs`).
//! Resetting the overlay is manual (the Reset button); there is no auto-reset.

use std::ffi::c_void;
use std::panic::{self, AssertUnwindSafe};

use hachimi_plugin_abi::event;
use hachimi_plugin_sdk::Sdk;

extern "C" fn on_shutdown(event_id: u32, _data: *const c_void, _userdata: *mut c_void) {
    if event_id != event::SHUTDOWN {
        return;
    }
    let _ = panic::catch_unwind(AssertUnwindSafe(|| {
        crate::capture::uninstall();
        hachimi_telemetry::shutdown();
        hlog_info!(target: "race-hud", "Shutdown: hooks removed, state cleared");
    }));
}

/// Subscribe to the host events the plugin needs. Returns `true` if the host
/// advertises the EVENTS capability.
pub fn subscribe_events() -> bool {
    let sdk = Sdk::get();
    if !sdk.has_capability(hachimi_plugin_abi::capability::EVENTS) {
        hlog_warn!(target: "race-hud", "Host does not advertise EVENTS capability");
        return false;
    }
    sdk.on(event::SHUTDOWN, on_shutdown, std::ptr::null_mut());
    true
}
