//! L2 overlay shell: tracking toggle, tab bar, scroll helper.

use hachimi_plugin_sdk::{egui, Sdk};

use crate::memory_reader;

use super::constants::LIST_MAX_HEIGHT;
use super::state::{selected_tab, set_selected_tab, Tab};

/// Apply overlay chrome and draw tracking toggle + tab bar when tracking is on.
pub(super) fn draw_shell(ui: &mut egui::Ui, tracking: bool) -> bool {
    draw_tracking_toggle(ui, tracking);

    if !tracking {
        draw_start_hint(ui);
        return false;
    }

    ui.separator();
    draw_tab_bar(ui);
    ui.separator();
    true
}

/// Hint shown when memory tracking is off.
fn draw_start_hint(ui: &mut egui::Ui) {
    ui.small("\u{1f3cb} Training Tracker");
    ui.small("Memory tracking is off — press Start Tracking above.");
}

/// Horizontal tab bar (text labels).
fn draw_tab_bar(ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        for (tab, label) in [
            (Tab::Training, "Training"),
            (Tab::Skills, "Skills"),
            (Tab::Shop, "Shop"),
            (Tab::Scenario, "Scenario"),
        ] {
            if ui.selectable_label(selected_tab() == tab, label).clicked() {
                set_selected_tab(tab);
            }
        }
    });
}

/// Compact Start/Stop memory-tracking button for the overlay (above the tabs).
fn draw_tracking_toggle(ui: &mut egui::Ui, tracking: bool) {
    let sdk = Sdk::get();
    if tracking {
        if ui.button("\u{23f9} Stop Tracking").clicked() {
            memory_reader::stop_tracking();
            sdk.show_notification("Memory tracking stopped");
        }
    } else if ui.button("\u{25b6} Start Tracking").clicked() {
        match memory_reader::start_tracking() {
            Ok(()) => {
                sdk.show_notification("Memory tracking started!");
            }
            Err(e) => {
                sdk.show_notification(&format!("Failed: {}", e));
                hlog_error!("start_tracking failed: {}", e);
            }
        }
    }
}

pub(super) fn scroll_list(ui: &mut egui::Ui, body: impl FnOnce(&mut egui::Ui)) {
    egui::ScrollArea::vertical()
        .max_height(LIST_MAX_HEIGHT)
        .auto_shrink([false, true])
        .show(ui, body);
}
