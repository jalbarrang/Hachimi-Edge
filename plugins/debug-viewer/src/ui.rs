//! Floating overlay for view-transition diagnostics.

use std::ffi::c_void;
use std::panic::{self, AssertUnwindSafe};

use hachimi_plugin_sdk::{egui, ui_from_ptr, Sdk};

const OVERLAY_ID: &str = "debug_viewer";
const OVERLAY_MIN_WIDTH: f32 = 260.0;

/// Register the debug overlay panel with Hachimi's GUI.
pub fn register_ui() {
    let sdk = Sdk::get();
    let handle = sdk.register_panel(OVERLAY_ID, draw_overlay, std::ptr::null_mut());

    if handle == 0 {
        hlog_warn!(target: "debug-viewer", "Overlay panel registration declined by host");
    } else {
        hlog_info!(target: "debug-viewer", "Overlay panel registered ({})", handle);
    }
}

extern "C" fn draw_overlay(ui: *mut c_void, _userdata: *mut c_void) {
    // SAFETY: host passes its live `&mut egui::Ui` for this callback.
    let ui = unsafe { ui_from_ptr(ui) };
    if panic::catch_unwind(AssertUnwindSafe(|| draw_overlay_inner(ui))).is_err() {
        hlog_error!(target: "debug-viewer", "draw_overlay panicked");
    }
}

fn draw_overlay_inner(ui: &mut egui::Ui) {
    let snapshot = crate::state::snapshot();

    ui.set_min_width(OVERLAY_MIN_WIDTH);
    ui.vertical(|ui| {
        ui.strong("Debug Viewer");
        ui.separator();

        ui.monospace(format!("Current view:  {}", format_view(snapshot.current_view_id)));
        ui.monospace(format!("Previous view: {}", format_view(snapshot.previous_view_id)));
        ui.monospace(format!("Transitions:   {}", snapshot.sequence));

        ui.separator();
        ui.label("Recent view changes");

        if snapshot.history.is_empty() {
            ui.label(egui::RichText::new("No VIEW_CHANGE events observed yet.").small());
        } else {
            egui::Grid::new("debug_viewer_history")
                .striped(true)
                .num_columns(3)
                .show(ui, |ui| {
                    ui.small("#");
                    ui.small("t");
                    ui.small("view");
                    ui.end_row();

                    for entry in snapshot.history.iter().rev() {
                        ui.monospace(entry.sequence.to_string());
                        ui.monospace(format!("{:.1}s", entry.seconds_since_start));
                        ui.monospace(format_view(Some(entry.view_id)));
                        ui.end_row();
                    }
                });
        }
    });
}

fn format_view(view_id: Option<i32>) -> String {
    view_id.map_or_else(
        || "—".to_owned(),
        |id| match Sdk::get().view_name(id) {
            Some(name) => format!("{id} ({name})"),
            None => id.to_string(),
        },
    )
}
