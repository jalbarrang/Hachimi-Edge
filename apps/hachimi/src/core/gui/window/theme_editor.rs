use rust_i18n::t;

use super::super::scale::get_scale;
use super::super::theme_preview::enqueue_theme_preview;
use super::{new_window, random_id, save_and_reload_config, simple_window_layout, Window};
use crate::core::{hachimi, Hachimi};

pub(crate) struct ThemeEditorWindow {
    id: egui::Id,
    config: hachimi::Config,
    old_config: hachimi::Config,
}

impl ThemeEditorWindow {
    pub(crate) fn new() -> ThemeEditorWindow {
        let current_cfg = (**Hachimi::instance().config.load()).clone();
        ThemeEditorWindow {
            id: random_id(),
            config: current_cfg.clone(),
            old_config: current_cfg,
        }
    }
}

fn theme_color_row(ui: &mut egui::Ui, label: &str, color: &mut egui::Color32) -> bool {
    let mut changed = false;

    ui.columns(2, |cols| {
        cols[0].with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
            ui.label(label);
        });

        cols[1].with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.color_edit_button_srgba(color).changed() {
                changed = true;
            }
        });
    });
    ui.end_row();

    changed
}

impl Window for ThemeEditorWindow {
    fn run(&mut self, ctx: &egui::Context) -> bool {
        let scale = get_scale(ctx);
        let mut open = true;
        let mut open2 = true;
        let mut theme_changed = false;
        let mut cancel_clicked = false;
        let mut save_clicked = false;
        let mut reset_clicked = false;

        new_window(ctx, self.id, t!("theme_editor.title"))
            .open(&mut open)
            .show(ctx, |ui| {
                simple_window_layout(
                    ui,
                    self.id,
                    |ui| {
                        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);

                        egui::Frame::NONE
                            .inner_margin(egui::Margin::symmetric(8, 0))
                            .show(ui, |ui| {
                                egui::Grid::new(self.id.with("theme_editor_grid"))
                                    .striped(true)
                                    .num_columns(2)
                                    .spacing([40.0 * scale, 4.0 * scale])
                                    .show(ui, |ui| {
                                        ui.vertical(|ui| {
                                            theme_changed |= theme_color_row(
                                                ui,
                                                &t!("theme_editor.ui_accent_color"),
                                                &mut self.config.ui_accent_color,
                                            );
                                            theme_changed |= theme_color_row(
                                                ui,
                                                &t!("theme_editor.ui_window_fill"),
                                                &mut self.config.ui_window_fill,
                                            );
                                            theme_changed |= theme_color_row(
                                                ui,
                                                &t!("theme_editor.ui_panel_fill"),
                                                &mut self.config.ui_panel_fill,
                                            );
                                            theme_changed |= theme_color_row(
                                                ui,
                                                &t!("theme_editor.ui_extreme_bg_color"),
                                                &mut self.config.ui_extreme_bg_color,
                                            );
                                            theme_changed |= theme_color_row(
                                                ui,
                                                &t!("theme_editor.ui_text_color"),
                                                &mut self.config.ui_text_color,
                                            );

                                            ui.horizontal(|ui| {
                                                ui.label(t!("theme_editor.ui_window_rounding"));
                                                if ui
                                                    .add(egui::Slider::new(
                                                        &mut self.config.ui_window_rounding,
                                                        0.0..=20.0,
                                                    ))
                                                    .changed()
                                                {
                                                    theme_changed = true;
                                                }
                                            });
                                        });
                                    });
                            });
                    },
                    |ui| {
                        ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |ui| {
                            if ui.button(t!("config_editor.restore_defaults")).clicked() {
                                reset_clicked = true;
                            }

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                                if ui.button(t!("cancel")).clicked() {
                                    cancel_clicked = true;
                                    open2 = false;
                                }
                                if ui.button(t!("save")).clicked() {
                                    save_clicked = true;
                                    open2 = false;
                                }
                            });
                        });
                    },
                );
            });

        if theme_changed {
            enqueue_theme_preview(self.config.clone());
        }

        if cancel_clicked {
            enqueue_theme_preview(self.old_config.clone());
            open2 = false;
        }

        if save_clicked {
            enqueue_theme_preview(self.config.clone());
            save_and_reload_config(self.config.clone());
        }

        if reset_clicked {
            let mut config = self.config.clone();
            config.ui_accent_color = hachimi::Config::default_ui_accent();
            config.ui_window_fill = hachimi::Config::default_window_fill();
            config.ui_panel_fill = hachimi::Config::default_panel_fill();
            config.ui_extreme_bg_color = hachimi::Config::default_extreme_bg();
            config.ui_text_color = hachimi::Config::default_text_color();
            config.ui_window_rounding = hachimi::Config::default_window_rounding();

            self.config = config;
            enqueue_theme_preview(self.config.clone());
        }

        open &= open2;
        open
    }
}
