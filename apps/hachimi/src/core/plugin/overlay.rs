//! Plugin overlay (L2 floating HUD) registration and persisted UI state.
//!
//! Overlays are registered once, stored behind `Lazy<Mutex<_>>`, and may be queued
//! from plugin init. The render side clones a snapshot via `get_plugin_overlays()`
//! before invoking callbacks, keeping lock scope short on the render thread.
//!
//! Per-panel UI state (visibility, collapsed/badge, position) and the global lock +
//! opacity live in [`OverlayUiState`], persisted to `overlay_state.json` so panel
//! placement survives restarts. The visibility map is keyed by overlay ID so the
//! render thread can toggle it without holding the registration lock.

use std::{
    collections::{HashMap, HashSet},
    ffi::c_void,
    sync::{atomic::AtomicBool, atomic::Ordering, Mutex},
};

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

use super::types::GuiMenuSectionCallback;

#[derive(Clone)]
pub(crate) struct PluginOverlay {
    pub(crate) handle: u64,
    pub(crate) owner: u32,
    pub(crate) id: String,
    pub(crate) callback: GuiMenuSectionCallback,
    pub(crate) userdata: usize,
}

/// Persisted per-panel state.
#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct PanelState {
    #[serde(default = "default_true")]
    pub(crate) visible: bool,
    #[serde(default)]
    pub(crate) collapsed: bool,
    #[serde(default)]
    pub(crate) pos: Option<[f32; 2]>,
}

impl Default for PanelState {
    fn default() -> Self {
        Self {
            visible: true,
            collapsed: false,
            pos: None,
        }
    }
}

/// Global L2 state plus the per-panel map. Serialized as `overlay_state.json`.
#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct OverlayUiState {
    #[serde(default)]
    pub(crate) locked: bool,
    #[serde(default = "default_opacity")]
    pub(crate) opacity: f32,
    #[serde(default)]
    pub(crate) panels: HashMap<String, PanelState>,
}

impl Default for OverlayUiState {
    fn default() -> Self {
        Self {
            locked: false,
            opacity: default_opacity(),
            panels: HashMap::new(),
        }
    }
}

fn default_true() -> bool {
    true
}
fn default_opacity() -> f32 {
    1.0
}

pub(crate) static PLUGIN_OVERLAYS: Lazy<Mutex<Vec<PluginOverlay>>> = Lazy::new(|| Mutex::new(Vec::new()));

static OVERLAY_UI: Lazy<Mutex<OverlayUiState>> = Lazy::new(|| Mutex::new(load_state()));

/// Set when a panel position changed in memory but hasn't been flushed to disk yet.
static POS_DIRTY: AtomicBool = AtomicBool::new(false);

/// Panels whose position should be force-reset to default on the next frame.
static RESET_QUEUE: Lazy<Mutex<HashSet<String>>> = Lazy::new(|| Mutex::new(HashSet::new()));

const STATE_FILE: &str = "overlay_state.json";

fn load_state() -> OverlayUiState {
    if !crate::core::Hachimi::is_initialized() {
        return OverlayUiState::default();
    }
    let path = crate::core::Hachimi::instance().get_data_path(STATE_FILE);
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

/// Persist current state to disk (best-effort; no-op before host init).
fn save_state(state: &OverlayUiState) {
    if !crate::core::Hachimi::is_initialized() {
        return;
    }
    let path = crate::core::Hachimi::instance().get_data_path(STATE_FILE);
    if let Err(e) = crate::core::utils::write_json_file(state, path) {
        error!("failed to save overlay state: {}", e);
    }
}

pub fn register_plugin_overlay(id: String, callback: GuiMenuSectionCallback, userdata: *mut c_void) -> u64 {
    OVERLAY_UI
        .lock()
        .expect("lock poisoned")
        .panels
        .entry(id.clone())
        .or_default();
    let handle = super::next_handle();
    PLUGIN_OVERLAYS.lock().expect("lock poisoned").push(PluginOverlay {
        handle,
        owner: super::current_owner(),
        id,
        callback,
        userdata: userdata as usize,
    });
    handle
}

/// Remove all overlays owned by `owner`.
pub(crate) fn remove_by_owner(owner: u32) {
    PLUGIN_OVERLAYS
        .lock()
        .expect("lock poisoned")
        .retain(|o| o.owner != owner);
}

/// Remove an overlay by handle. Returns whether anything was removed.
pub(crate) fn remove_by_handle(handle: u64) -> bool {
    let mut overlays = PLUGIN_OVERLAYS.lock().expect("lock poisoned");
    let before = overlays.len();
    overlays.retain(|o| o.handle != handle);
    overlays.len() != before
}

pub(crate) fn get_plugin_overlays() -> Vec<PluginOverlay> {
    PLUGIN_OVERLAYS.lock().expect("lock poisoned").clone()
}

pub(crate) fn has_plugin_overlays() -> bool {
    !PLUGIN_OVERLAYS.lock().map_or(true, |o| o.is_empty())
}

// ── Per-panel + global UI state ──

/// Get the visibility state for an overlay. Returns `true` if unknown.
pub(crate) fn is_overlay_visible(id: &str) -> bool {
    OVERLAY_UI
        .lock()
        .map_or(true, |s| s.panels.get(id).is_none_or(|p| p.visible))
}

/// Set visibility for an overlay by ID (host close-button + plugin vtable call).
pub fn set_overlay_visible(id: &str, visible: bool) {
    let mut state = OVERLAY_UI.lock().expect("lock poisoned");
    state.panels.entry(id.to_owned()).or_default().visible = visible;
    save_state(&state);
}

/// Snapshot of a panel's persisted state (defaults if unknown).
pub(crate) fn panel_state(id: &str) -> PanelState {
    OVERLAY_UI
        .lock()
        .expect("lock poisoned")
        .panels
        .get(id)
        .cloned()
        .unwrap_or_default()
}

/// Forget a panel's saved position so it returns to its default spot next frame.
pub(crate) fn reset_panel_pos(id: &str) {
    let mut state = OVERLAY_UI.lock().expect("lock poisoned");
    state.panels.entry(id.to_owned()).or_default().pos = None;
    save_state(&state);
    drop(state);
    RESET_QUEUE.lock().expect("lock poisoned").insert(id.to_owned());
}

/// Consume a pending position-reset request for `id` (true = force to default this frame).
pub(crate) fn take_reset(id: &str) -> bool {
    RESET_QUEUE.lock().expect("lock poisoned").remove(id)
}

pub(crate) fn set_panel_collapsed(id: &str, collapsed: bool) {
    let mut state = OVERLAY_UI.lock().expect("lock poisoned");
    state.panels.entry(id.to_owned()).or_default().collapsed = collapsed;
    save_state(&state);
}

/// Update a panel's position in memory (cheap; call [`persist`] to flush on drag-stop).
pub(crate) fn set_panel_pos(id: &str, pos: [f32; 2]) {
    let mut state = OVERLAY_UI.lock().expect("lock poisoned");
    let entry = state.panels.entry(id.to_owned()).or_default();
    if entry.pos != Some(pos) {
        entry.pos = Some(pos);
        POS_DIRTY.store(true, Ordering::Relaxed);
    }
}

pub(crate) fn is_locked() -> bool {
    OVERLAY_UI.lock().is_ok_and(|s| s.locked)
}

pub(crate) fn set_locked(locked: bool) {
    let mut state = OVERLAY_UI.lock().expect("lock poisoned");
    state.locked = locked;
    save_state(&state);
}

pub(crate) fn opacity() -> f32 {
    OVERLAY_UI.lock().map_or(1.0, |s| s.opacity)
}

pub(crate) fn set_opacity(value: f32) {
    let mut state = OVERLAY_UI.lock().expect("lock poisoned");
    state.opacity = value.clamp(0.1, 1.0);
    save_state(&state);
}

/// Turn an overlay id like `training_tracker_overlay` into `Training Tracker Overlay`.
pub(crate) fn display_title(id: &str) -> String {
    id.split('_')
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                Some(first) => first.to_uppercase().collect::<String>() + c.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Flush pending position changes to disk (call on pointer release).
pub(crate) fn persist_if_dirty() {
    if POS_DIRTY.swap(false, Ordering::Relaxed) {
        let state = OVERLAY_UI.lock().expect("lock poisoned");
        save_state(&state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use super::super::TEST_LOCK;

    extern "C" fn overlay_callback(_: *mut c_void, _: *mut c_void) {}

    #[test]
    fn has_plugin_overlays_reflects_registration_state() {
        let _guard = TEST_LOCK.lock().expect("lock poisoned");

        {
            let mut overlays = PLUGIN_OVERLAYS.lock().expect("lock poisoned");
            overlays.clear();
        }

        assert!(!has_plugin_overlays());

        let _ = register_plugin_overlay("test".to_owned(), overlay_callback, std::ptr::null_mut());
        assert!(has_plugin_overlays());
        assert!(is_overlay_visible("test"));

        {
            let mut overlays = PLUGIN_OVERLAYS.lock().expect("lock poisoned");
            overlays.clear();
        }
    }

    #[test]
    fn remove_by_owner_only_drops_matching() {
        let _guard = TEST_LOCK.lock().expect("lock poisoned");
        PLUGIN_OVERLAYS.lock().expect("lock poisoned").clear();

        {
            let _s = super::super::OwnerScope::enter(7);
            let _ = register_plugin_overlay("a".to_owned(), overlay_callback, std::ptr::null_mut());
        }
        {
            let _s = super::super::OwnerScope::enter(8);
            let _ = register_plugin_overlay("b".to_owned(), overlay_callback, std::ptr::null_mut());
        }

        remove_by_owner(7);
        let overlays = get_plugin_overlays();
        assert_eq!(overlays.len(), 1);
        assert_eq!(overlays[0].owner, 8);
        PLUGIN_OVERLAYS.lock().expect("lock poisoned").clear();
    }
}
