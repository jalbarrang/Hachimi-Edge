//! Host event subscriptions for view-transition recording.

use std::ffi::c_void;
use std::panic::{self, AssertUnwindSafe};

use hachimi_plugin_abi::{event, ViewChangeEvent};
use hachimi_plugin_sdk::Sdk;

extern "C" fn on_view_change(event_id: u32, data: *const c_void, _userdata: *mut c_void) {
    if event_id != event::VIEW_CHANGE {
        return;
    }

    if panic::catch_unwind(AssertUnwindSafe(|| handle_view_change(data))).is_err() {
        hlog_error!(target: "debug-viewer", "VIEW_CHANGE callback panicked");
    }
}

fn handle_view_change(data: *const c_void) {
    if data.is_null() {
        hlog_warn!(target: "debug-viewer", "VIEW_CHANGE had null payload");
        return;
    }

    // SAFETY: for `event::VIEW_CHANGE`, the host passes a pointer to
    // `ViewChangeEvent` that is valid for the callback duration. Copy it now.
    let payload = unsafe { *data.cast::<ViewChangeEvent>() };
    let update = crate::state::record_view_change(payload.view_id);

    let name = Sdk::get().view_name(update.current_view_id).unwrap_or("uncatalogued");
    hlog_info!(
        target: "debug-viewer",
        "view transition #{}: {:?} -> {} ({})",
        update.sequence,
        update.previous_view_id,
        update.current_view_id,
        name
    );
}

extern "C" fn on_shutdown(event_id: u32, _data: *const c_void, _userdata: *mut c_void) {
    if event_id != event::SHUTDOWN {
        return;
    }

    if panic::catch_unwind(AssertUnwindSafe(crate::state::reset)).is_err() {
        hlog_error!(target: "debug-viewer", "SHUTDOWN callback panicked");
        return;
    }

    hlog_info!(target: "debug-viewer", "Shutdown: recorded state reset");
}

/// Subscribe to host events used by the debug viewer.
pub fn subscribe_events() -> bool {
    let sdk = Sdk::get();
    if !sdk.has_capability(hachimi_plugin_abi::capability::EVENTS) {
        hlog_warn!(target: "debug-viewer", "Host does not advertise EVENTS capability");
        return false;
    }

    let view_handle = sdk.on(event::VIEW_CHANGE, on_view_change, std::ptr::null_mut());
    let shutdown_handle = sdk.on(event::SHUTDOWN, on_shutdown, std::ptr::null_mut());

    if view_handle == 0 || shutdown_handle == 0 {
        hlog_warn!(
            target: "debug-viewer",
            "Event subscription failed (VIEW_CHANGE={}, SHUTDOWN={})",
            view_handle,
            shutdown_handle
        );
        return false;
    }

    hlog_info!(
        target: "debug-viewer",
        "Event subscriptions registered (VIEW_CHANGE={}, SHUTDOWN={})",
        view_handle,
        shutdown_handle
    );
    true
}
