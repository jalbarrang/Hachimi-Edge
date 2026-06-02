//! Bonds section: bond names + progress (scrollable).
//! Rendered inside the Training tab under a "Bonds" heading.

use hachimi_plugin_sdk::egui;

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

fn draw_panel(ui: &mut egui::Ui) {
    let evals = overlay_cache::evaluations();

    if evals.is_empty() {
        ui.small("No bond data available");
        return;
    }

    for eval in &evals {
        if !eval.is_appear {
            continue;
        }

        let (r, g, b) = bond_color(eval.value);
        let name = if eval.name.is_empty() {
            format!("#{}", eval.target_id)
        } else {
            eval.name.clone()
        };
        ui.colored_label(
            egui::Color32::from_rgb(r, g, b),
            format!("{} - {}/100", name, eval.value),
        );
    }
}
