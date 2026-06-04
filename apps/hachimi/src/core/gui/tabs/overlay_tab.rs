//! L1 Overlay tab — manage L2 floating panels: global lock, opacity, and
//! per-panel show/hide + reset position.

use crate::core::gui::widgets;
use crate::core::gui::Gui;
use crate::core::plugin::overlay;

impl Gui {
    pub(crate) fn run_overlay_settings_tab(&mut self, ui: &mut egui::Ui) {
        widgets::section_header(ui, "Floating overlays");

        // Global lock.
        let mut locked = overlay::is_locked();
        if widgets::toggle_row(ui, "Lock overlays (click-through)", &mut locked).changed() {
            overlay::set_locked(locked);
        }

        // Global opacity.
        let mut opacity = overlay::opacity();
        ui.horizontal(|ui| {
            ui.label("Opacity");
            if ui
                .add(egui::Slider::new(&mut opacity, 0.1..=1.0).fixed_decimals(2))
                .changed()
            {
                overlay::set_opacity(opacity);
            }
        });

        ui.add_space(6.0);

        let overlays = overlay::get_plugin_overlays();
        if overlays.is_empty() {
            ui.weak("No plugins have registered any overlays.");
            return;
        }

        widgets::section_header(ui, "Panels");
        for ov in &overlays {
            let title = overlay::display_title(&ov.id);
            let mut visible = overlay::is_overlay_visible(&ov.id);
            ui.horizontal(|ui| {
                if widgets::toggle_ui(ui, &mut visible).changed() {
                    overlay::set_overlay_visible(&ov.id, visible);
                }
                ui.label(&title);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if widgets::ghost_button(ui, "Reset")
                        .on_hover_text("Reset position and size")
                        .clicked()
                    {
                        overlay::reset_panel(&ov.id);
                    }
                });
            });
        }
    }
}
