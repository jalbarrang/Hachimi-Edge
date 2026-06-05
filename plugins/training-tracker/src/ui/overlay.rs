//! L2 overlay shell: tracking toggle, tab bar, scroll helper, content scaling.

use std::sync::atomic::{AtomicU32, Ordering};

use hachimi_plugin_sdk::{egui, Sdk};

use crate::memory_reader;

use super::constants::{MIN_LIST_HEIGHT, OVERLAY_FONT_SIZE, OVERLAY_MIN_WIDTH};
use crate::tabs::{self, selected_tab, set_selected_tab, Tab};

/// Current overlay content scale (font + spacing multiplier), driven by the
/// panel's width so resizing the window zooms the whole panel uniformly.
static OVERLAY_SCALE: AtomicU32 = AtomicU32::new(0x3f80_0000); // 1.0f32 bits

/// Largest content zoom; keeps very wide panels from becoming comically large.
const MAX_SCALE: f32 = 3.0;
const MIN_SCALE: f32 = 0.85;

/// Compute the content scale from the available panel width, apply it to `ui`
/// (font size + spacing), and store it for size-dependent callers. Returns it.
pub(super) fn apply_scale(ui: &mut egui::Ui) -> f32 {
    let scale = (ui.available_width() / OVERLAY_MIN_WIDTH).clamp(MIN_SCALE, MAX_SCALE);
    OVERLAY_SCALE.store(scale.to_bits(), Ordering::Relaxed);

    ui.style_mut().override_font_id = Some(egui::FontId::proportional(OVERLAY_FONT_SIZE * scale));
    let sp = ui.spacing_mut();
    sp.item_spacing *= scale;
    sp.button_padding *= scale;
    sp.interact_size *= scale;
    sp.indent *= scale;
    scale
}

/// The current overlay content scale.
pub(super) fn scale() -> f32 {
    f32::from_bits(OVERLAY_SCALE.load(Ordering::Relaxed))
}

/// Scaled base font size for callers that set an explicit text size.
pub(super) fn font_size() -> f32 {
    OVERLAY_FONT_SIZE * scale()
}

/// Add vertical space that scales with the panel.
pub(super) fn space(ui: &mut egui::Ui, base: f32) {
    ui.add_space(base * scale());
}

/// Apply overlay chrome and draw tracking toggle + tab bar when tracking is on.
pub(super) fn draw_shell(ui: &mut egui::Ui, tracking: bool) -> bool {
    draw_tracking_toggle(ui, tracking);

    if !tracking {
        draw_start_hint(ui);
        return false;
    }

    ui.separator();
    // Hide the tab row when only one tab is enabled — the overlay becomes a single
    // clean panel showing just that tab's body.
    if tabs::enabled_count() > 1 {
        draw_tab_bar(ui);
        ui.separator();
    }
    true
}

/// Hint shown when memory tracking is off.
fn draw_start_hint(ui: &mut egui::Ui) {
    ui.small("\u{1f3cb} Training Tracker");
    ui.small("Memory tracking is off — press Start Tracking above.");
}

/// Horizontal tab bar (text labels) — only the user-enabled tabs are shown.
fn draw_tab_bar(ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        for (tab, label) in Tab::ALL {
            if !tabs::is_enabled(tab) {
                continue;
            }
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
    // Fill the remaining height of the (resizable) panel so vertical resizing is
    // meaningful; fall back to a small minimum when the panel is tiny.
    let max_height = ui.available_height().max(MIN_LIST_HEIGHT);
    egui::ScrollArea::vertical()
        .max_height(max_height)
        .auto_shrink([false, false])
        .show(ui, body);
}
