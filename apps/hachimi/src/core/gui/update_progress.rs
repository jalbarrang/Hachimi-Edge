use rust_i18n::t;

use crate::core::{tl_repo, Hachimi};

use super::scale::get_scale;
use super::Gui;

impl Gui {
    pub(crate) fn run_update_progress(&mut self) {
        let ctx = &self.context;
        let scale = get_scale(ctx);

        let progress = Hachimi::instance().tl_updater.progress().unwrap_or_else(|| {
            self.update_progress_visible = false;
            tl_repo::UpdateProgress::new(1, 1)
        });
        let ratio = progress.current as f32 / progress.total as f32;

        egui::Area::new("update_progress".into())
            .fixed_pos(egui::Pos2 {
                x: 4.0 * scale,
                y: 4.0 * scale,
            })
            .show(ctx, |ui| {
                egui::Frame::NONE
                    .fill(self.config.ui_panel_fill)
                    .inner_margin(egui::Margin::same((4.0 * scale) as i8))
                    .corner_radius(4.0 * scale)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(t!("tl_updater.title"));
                            ui.add_space(26.0 * scale);
                            ui.label(format!("{:.2}%", ratio * 100.0));
                        });
                        ui.add(
                            egui::ProgressBar::new(ratio)
                                .desired_height(4.0 * scale)
                                .desired_width(140.0 * scale),
                        );
                        ui.label(
                            egui::RichText::new(t!("tl_updater.warning"))
                                .font(egui::FontId::proportional(10.0 * scale)),
                        );
                    });
            });
    }
}
