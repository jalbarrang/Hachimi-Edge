use rust_i18n::t;

use super::super::scale::get_scale;
use super::super::Gui;
use super::{new_window, random_id, save_and_reload_config, simple_window_layout, Window};
use crate::core::{hachimi, Hachimi};

pub(crate) struct LiveVocalsSwapWindow {
    id: egui::Id,
    config: hachimi::Config,
    chara_choices: Vec<(i32, String)>,
    search_term: String,
}

impl LiveVocalsSwapWindow {
    pub(crate) fn new() -> LiveVocalsSwapWindow {
        let hachimi = Hachimi::instance();
        let mut chara_choices: Vec<(i32, String)> = Vec::new();
        chara_choices.push((0, t!("default").into_owned()));

        let data = hachimi.chara_data.load();
        for &id in &data.chara_ids {
            chara_choices.push((id, data.get_name(id)));
        }
        chara_choices.sort_by_key(|choice| choice.0);

        LiveVocalsSwapWindow {
            id: random_id(),
            config: (**hachimi.config.load()).clone(),
            chara_choices,
            search_term: String::new(),
        }
    }
}

impl Window for LiveVocalsSwapWindow {
    fn run(&mut self, ctx: &egui::Context) -> bool {
        let scale = get_scale(ctx);
        let mut open = true;
        let mut open2 = true;
        let mut save_clicked = false;

        let combo_items: Vec<(i32, &str)> = self
            .chara_choices
            .iter()
            .map(|&(id, ref name)| (id, name.as_str()))
            .collect();

        new_window(ctx, self.id, t!("config_editor.live_vocals_swap"))
            .open(&mut open)
            .show(ctx, |ui| {
                simple_window_layout(
                    ui,
                    self.id,
                    |ui| {
                        egui::Frame::NONE
                            .inner_margin(egui::Margin::symmetric(8, 0))
                            .show(ui, |ui| {
                                egui::Grid::new(self.id.with("live_vocals_swap_grid"))
                                    .striped(true)
                                    .num_columns(2)
                                    .spacing([40.0 * scale, 4.0 * scale])
                                    .show(ui, |ui| {
                                        for i in 0..6 {
                                            ui.label(t!("config_editor.live_vocals_swap_character_n", index = i + 1));
                                            Gui::run_combo_menu(
                                                ui,
                                                egui::Id::new("vocals_swap").with(i),
                                                &mut self.config.live_vocals_swap[i],
                                                &combo_items,
                                                &mut self.search_term,
                                            );
                                            ui.end_row();
                                        }
                                    });
                            });
                    },
                    |ui| {
                        ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |ui| {
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                                if ui.button(t!("cancel")).clicked() {
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

        if save_clicked {
            save_and_reload_config(self.config.clone());
        }

        open &= open2;
        open
    }
}
