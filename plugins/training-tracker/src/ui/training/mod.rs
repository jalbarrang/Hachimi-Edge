//! Training tab orchestrator.

mod footer;
mod overview;
mod stats_grid;

use hachimi_plugin_sdk::egui;

use super::snapshot;

pub(super) fn draw(ui: &mut egui::Ui) {
    let Some(snap) = snapshot::current_snapshot(ui) else {
        return;
    };

    overview::draw(ui, &snap);
    let stats = stats_grid::build_stats(&snap);
    let rec = stats_grid::score_facilities(&snap);
    let any_capped = stats_grid::draw(ui, &snap, &stats, &rec);
    footer::draw(ui, &snap, &stats, &rec, any_capped);

    super::bonds::draw_section(ui);
}
