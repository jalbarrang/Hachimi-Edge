//! Training tab orchestrator.

mod footer;
mod overview;
mod stats_grid;

use hachimi_plugin_sdk::egui;

use crate::planner;

use super::snapshot;

pub(super) fn draw(ui: &mut egui::Ui) {
    let Some(snap) = snapshot::current_snapshot(ui) else {
        return;
    };

    overview::draw(ui, &snap);
    ui.add_space(16.0);

    let stats = stats_grid::build_stats(&snap);
    // Objective-aware greedy scores, then layer the multi-turn planner (energy,
    // bonds, career phase) on top before display + suggestion.
    let sctx = stats_grid::scoring_context(&snap);
    let pctx = stats_grid::plan_context(&snap);
    let base = stats_grid::score_facilities(&snap, &sctx);
    let rec = planner::adjust_scores(&base, &pctx, &planner::params());
    let any_capped = stats_grid::draw(ui, &snap, &stats, &rec);
    ui.add_space(16.0);

    footer::draw(ui, &snap, &stats, &rec, any_capped, &pctx, &sctx);
    ui.add_space(16.0);

    super::bonds::draw_section(ui);
}
