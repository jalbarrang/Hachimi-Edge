//! L1 menu page (Plugins tab section).

use std::sync::atomic::Ordering;

use hachimi_plugin_sdk::{egui, Sdk};

use crate::class_dump;
use crate::memory_reader;
use crate::overlay_cache;
use crate::stat_targets;

use super::constants::OVERLAY_ID;

/// Page title — h2 (theme heading size).
fn heading_h2(ui: &mut egui::Ui, text: impl Into<egui::RichText>) {
    ui.heading(text);
}

/// Section title — h3 (between body and heading).
fn heading_h3(ui: &mut egui::Ui, text: impl Into<egui::RichText>) {
    let style = ui.style();
    let heading_size = egui::TextStyle::Heading.resolve(style).size;
    let body_size = egui::TextStyle::Body.resolve(style).size;
    let size = body_size + (heading_size - body_size) * 0.55;
    ui.label(text.into().size(size).strong());
}

pub(super) fn draw(ui: &mut egui::Ui) {
    let sdk = Sdk::get();

    heading_h2(ui, "\u{1f3cb} Training Tracker");
    ui.add_space(8.0);

    draw_tracking_controls(ui);

    ui.add_space(12.0);
    ui.separator();
    ui.add_space(8.0);
    draw_stat_targets(ui);

    ui.add_space(12.0);
    ui.separator();
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        if ui.button("\u{1f4ca} Show Training Overlay").clicked() {
            if sdk.overlay_set_visible(OVERLAY_ID, true) {
                sdk.show_notification("Training overlay shown");
            } else {
                hlog_warn!(target: "training-tracker", "Host declined overlay_set_visible");
            }
        }
        if ui.button("\u{1f4cb} Dump All IL2CPP Classes").clicked() {
            class_dump::dump_all_classes();
            sdk.show_notification("Class dump complete — see il2cpp_classes.txt");
        }
    });
}

/// Draw start/stop button and brief status in the menu.
fn draw_tracking_controls(ui: &mut egui::Ui) {
    let sdk = Sdk::get();
    let tracking = memory_reader::TRACKING.load(Ordering::Relaxed);

    if !tracking {
        if ui.button("\u{25b6} Start Memory Tracking").clicked() {
            match memory_reader::start_tracking() {
                Ok(()) => sdk.show_notification("Memory tracking started!"),
                Err(e) => {
                    sdk.show_notification(&format!("Failed: {}", e));
                    hlog_error!("start_tracking failed: {}", e);
                    false
                }
            };
        }
        ui.small("Reads stats directly from game memory via IL2CPP");
        return;
    }

    if ui.button("\u{23f9} Stop Memory Tracking").clicked() {
        memory_reader::stop_tracking();
        sdk.show_notification("Memory tracking stopped");
        return;
    }

    overlay_cache::maybe_request_refresh();
    let status = match overlay_cache::snapshot() {
        Some(snap) if snap.is_playing => format!(
            "\u{2705} Tracking • Turn {} • Total {}",
            snap.current_turn, snap.total_stats
        ),
        Some(_) => "\u{23f8} No active career".to_owned(),
        None => "\u{26a0} Waiting for data…".to_owned(),
    };
    ui.small(status);
}

/// Per-stat target editor. 0 = use the game cap; a positive value warns earlier.
fn draw_stat_targets(ui: &mut egui::Ui) {
    heading_h3(ui, "\u{1f3af} Stat Targets");
    ui.small("0 = game cap");
    ui.add_space(4.0);
    let mut t = stat_targets::targets();
    let mut changed = false;
    let mut commit = false;
    egui::Grid::new("tt_targets")
        .num_columns(stat_targets::LABELS.len())
        .striped(true)
        .spacing([8.0, 4.0])
        .show(ui, |ui| {
            for name in stat_targets::LABELS.iter() {
                ui.label(*name);
            }
            ui.end_row();
            for i in 0..stat_targets::LABELS.len() {
                let resp = ui.add(
                    egui::DragValue::new(&mut t[i])
                        .speed(10.0)
                        .range(0..=stat_targets::MAX_TARGET),
                );
                changed |= resp.changed();
                commit |= resp.drag_stopped() || resp.lost_focus();
            }
            ui.end_row();
        });
    ui.add_space(4.0);
    if changed {
        stat_targets::set_targets(t);
    }
    if commit {
        stat_targets::persist();
    }
    if ui.small_button("Clear targets").clicked() {
        stat_targets::set_targets([0; 5]);
        stat_targets::persist();
    }
}
