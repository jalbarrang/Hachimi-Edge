//! GUI rendering via the Hachimi plugin menu system.
//!
//! With API v9 the host hands plugins the real `egui::Ui`, so we draw with egui
//! directly. Registers a menu section and an overlay that display:
//! - Live career stats read directly from game memory (memory-read mode)

use std::ffi::c_void;
use std::panic::{self, AssertUnwindSafe};
use std::sync::atomic::Ordering;

use hachimi_plugin_sdk::{egui, ui_from_ptr, Sdk};

use crate::memory_reader;

mod bonds;
mod constants;
mod menu;
mod overlay;
mod scenario;
mod skill_shop_tab;
mod skills;
mod snapshot;
mod training;
mod util;

// Public API re-export (unused within crate; kept for external callers).
#[allow(unused_imports)]
pub use util::bond_color;

/// Register the plugin's UI components with the Hachimi GUI.
pub fn register_ui() {
    let sdk = Sdk::get();

    sdk.register_page(draw_menu_section, std::ptr::null_mut());

    if sdk.register_panel(constants::OVERLAY_ID, draw_overlay, std::ptr::null_mut()) != 0 {
        hlog_info!(target: "training-tracker", "UI registered (L1 page + L2 panel)");
    } else {
        hlog_warn!(
            target: "training-tracker",
            "L1 page registered; L2 panel registration declined by host"
        );
    }
}

extern "C" fn draw_menu_section(ui: *mut c_void, _userdata: *mut c_void) {
    // SAFETY: host passes its live `&mut egui::Ui` for this callback.
    let ui = unsafe { ui_from_ptr(ui) };
    if panic::catch_unwind(AssertUnwindSafe(|| menu::draw(ui))).is_err() {
        hlog_error!("draw_menu_section PANICKED");
    }
}

extern "C" fn draw_overlay(ui: *mut c_void, _userdata: *mut c_void) {
    // SAFETY: host passes its live `&mut egui::Ui` for this callback.
    let ui = unsafe { ui_from_ptr(ui) };
    if panic::catch_unwind(AssertUnwindSafe(|| draw_overlay_inner(ui))).is_err() {
        hlog_error!("draw_overlay PANICKED");
    }
}

fn draw_overlay_inner(ui: &mut egui::Ui) {
    let tracking = memory_reader::TRACKING.load(Ordering::Relaxed);

    // Drive a uniform content zoom from the panel width: resizing the window
    // scales the whole panel (font + spacing) instead of scrolling/reflowing.
    overlay::apply_scale(ui);
    ui.set_min_width(constants::OVERLAY_MIN_WIDTH);

    if !overlay::draw_shell(ui, tracking) {
        return;
    }

    match crate::tabs::selected_tab() {
        crate::tabs::Tab::Training => training::draw(ui),
        crate::tabs::Tab::Skills => skills::draw(ui),
        crate::tabs::Tab::Shop => skill_shop_tab::draw(ui),
        crate::tabs::Tab::Scenario => scenario::draw(ui),
    }
}
