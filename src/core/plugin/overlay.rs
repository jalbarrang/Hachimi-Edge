//! Plugin overlay registration and shared overlay state.
//! Overlays are registered once, stored behind `Lazy<Mutex<_>>`, and may be queued from plugin init.
//! The render side clones a snapshot via `get_plugin_overlays()` before invoking callbacks.
//! This snap-and-render pattern keeps lock scope short on the render thread.
//!
//! Visibility is tracked in a separate map so the render thread can toggle it
//! (via egui::Window close button) without holding the registration lock.

use std::{collections::HashMap, ffi::c_void, sync::Mutex};

use once_cell::sync::Lazy;

use super::types::GuiMenuSectionCallback;

#[derive(Clone)]
pub(crate) struct PluginOverlay {
    pub(crate) id: String,
    pub(crate) callback: GuiMenuSectionCallback,
    pub(crate) userdata: usize,
}

pub(crate) static PLUGIN_OVERLAYS: Lazy<Mutex<Vec<PluginOverlay>>> = Lazy::new(|| Mutex::new(Vec::new()));

/// Per-overlay visibility state, keyed by overlay ID.
/// Defaults to `true` (visible) when an overlay is first registered.
static OVERLAY_VISIBILITY: Lazy<Mutex<HashMap<String, bool>>> = Lazy::new(|| Mutex::new(HashMap::new()));

pub fn register_plugin_overlay(id: String, callback: GuiMenuSectionCallback, userdata: *mut c_void) {
    OVERLAY_VISIBILITY
        .lock()
        .expect("lock poisoned")
        .entry(id.clone())
        .or_insert(true);
    PLUGIN_OVERLAYS.lock().expect("lock poisoned").push(PluginOverlay {
        id,
        callback,
        userdata: userdata as usize,
    });
}

pub(crate) fn get_plugin_overlays() -> Vec<PluginOverlay> {
    PLUGIN_OVERLAYS.lock().expect("lock poisoned").clone()
}

pub(crate) fn has_plugin_overlays() -> bool {
    !PLUGIN_OVERLAYS.lock().map_or(true, |o| o.is_empty())
}

/// Get the visibility state for an overlay. Returns `true` if unknown.
pub(crate) fn is_overlay_visible(id: &str) -> bool {
    OVERLAY_VISIBILITY.lock().map_or(true, |m| *m.get(id).unwrap_or(&true))
}

/// Set visibility for an overlay by ID (used by host close-button and plugin vtable call).
pub fn set_overlay_visible(id: &str, visible: bool) {
    OVERLAY_VISIBILITY
        .lock()
        .expect("lock poisoned")
        .insert(id.to_owned(), visible);
}

#[cfg(test)]
mod tests {
    use super::*;

    static TEST_MUTEX: Mutex<()> = Mutex::new(());

    extern "C" fn overlay_callback(_: *mut c_void, _: *mut c_void) {}

    #[test]
    fn has_plugin_overlays_reflects_registration_state() {
        let _guard = TEST_MUTEX.lock().expect("lock poisoned");

        {
            let mut overlays = PLUGIN_OVERLAYS.lock().expect("lock poisoned");
            overlays.clear();
        }

        assert!(!has_plugin_overlays());

        register_plugin_overlay("test".to_owned(), overlay_callback, std::ptr::null_mut());
        assert!(has_plugin_overlays());

        {
            let mut overlays = PLUGIN_OVERLAYS.lock().expect("lock poisoned");
            overlays.clear();
        }
    }
}
