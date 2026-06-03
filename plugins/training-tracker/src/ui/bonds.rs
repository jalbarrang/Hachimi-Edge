//! Bonds section: bond names + progress (scrollable).
//! Rendered inside the Training tab under a "Bonds" heading.

use hachimi_plugin_sdk::egui;

use crate::bond_progress;
use crate::gametora_data;
use crate::overlay_cache;

use super::constants::OVERLAY_FONT_SIZE;
use super::overlay;
use super::util::bond_color;

/// Draw the "Bonds" heading (h2) followed by the bond list on the next rows.
pub(super) fn draw_section(ui: &mut egui::Ui) {
    overlay_cache::maybe_request_refresh();
    ui.add_space(8.0);
    ui.label(egui::RichText::new("Bonds").size(OVERLAY_FONT_SIZE * 1.4).strong());
    ui.add_space(4.0);
    overlay::scroll_list(ui, draw_panel);
}

/// Names for scenario-specific NPC bonds (not real support cards, so absent from
/// the catalog). Keyed by the scenario's speed-slot command base + `target_id`.
fn scenario_npc_name(scenario_id: i32, target_id: i32) -> Option<&'static str> {
    match (scenario_id, target_id) {
        // Trackblazer (Make a New Track) — scenario_id 4
        (4, 102) => Some("Director Akikawa"),
        (4, 103) => Some("Etsuko Otonashi"),
        _ => None,
    }
}

fn draw_panel(ui: &mut egui::Ui) {
    let evals = overlay_cache::evaluations();

    if evals.is_empty() {
        ui.small("No bond data available");
        return;
    }

    // Active scenario id gates scenario-NPC name labels.
    let scenario_id = overlay_cache::snapshot().map_or(0, |s| s.scenario_id);
    // Deck slot (Evaluation.target_id) -> support_card_id, read safely once per career.
    let deck = overlay_cache::equipped_support_ids();

    // Three columns (name | bond | chain) aligned like the stats table, 12px gap.
    egui::Grid::new("tt_bonds")
        .num_columns(3)
        .spacing([12.0, 4.0])
        .striped(true)
        .show(ui, |ui| {
            ui.strong("Support");
            ui.strong("Bond");
            ui.strong("Events");
            ui.end_row();

            for eval in &evals {
                if !eval.is_appear {
                    continue;
                }

                let support_id = deck
                    .iter()
                    .find(|(slot, _)| *slot == eval.target_id)
                    .map(|(_, id)| *id as i64)
                    .filter(|id| *id > 0);

                // Name: GameTora character name when the card is mapped + catalogued.
                let name = support_id
                    .and_then(gametora_data::support_card_name)
                    .map(str::to_owned)
                    .filter(|n| !n.is_empty())
                    .or_else(|| (!eval.name.is_empty()).then(|| eval.name.clone()))
                    .or_else(|| scenario_npc_name(scenario_id, eval.target_id).map(str::to_owned))
                    .unwrap_or_else(|| format!("#{}", eval.target_id));

                let color = {
                    let (r, g, b) = bond_color(eval.value);
                    egui::Color32::from_rgb(r, g, b)
                };
                ui.colored_label(color, name);
                ui.colored_label(color, format!("{}/100", eval.value));

                // Events column: [-] X/Y [+] for cards with a known chain max.
                match support_id.and_then(|id| gametora_data::max_chain_steps(id).map(|m| (id, m))) {
                    Some((id, max)) if max > 0 => {
                        let done = bond_progress::count(id).min(max);
                        ui.horizontal(|ui| {
                            if ui.small_button("-").clicked() {
                                bond_progress::adjust(id, -1, max);
                            }
                            ui.colored_label(color, format!("{}/{}", done, max));
                            if ui.small_button("+").clicked() {
                                bond_progress::adjust(id, 1, max);
                            }
                        });
                    }
                    _ => {
                        ui.label("");
                    }
                }
                ui.end_row();
            }
        });
}
