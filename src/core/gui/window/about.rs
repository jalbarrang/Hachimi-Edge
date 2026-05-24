use std::thread;

use chrono::{Datelike, Utc};
use rust_i18n::t;

use super::super::scale::get_scale;
use super::super::Gui;
use super::{new_window, random_id, LicenseWindow, Window};
use crate::core::hachimi::{REPO_PATH, WEBSITE_URL};
use crate::il2cpp::{ext::StringExt, hook::UnityEngine_CoreModule::Application};

pub(crate) struct AboutWindow {
    id: egui::Id,
}

impl AboutWindow {
    pub(crate) fn new() -> AboutWindow {
        AboutWindow { id: random_id() }
    }
}

impl Window for AboutWindow {
    fn run(&mut self, ctx: &egui::Context) -> bool {
        let scale = get_scale(ctx);
        let mut open = true;

        new_window(ctx, self.id, t!("about.title"))
            .max_width(310.0 * scale)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.add(Gui::icon_2x(ctx));
                    ui.vertical(|ui| {
                        ui.heading(t!("hachimi"));
                        ui.label(env!("HACHIMI_DISPLAY_VERSION"));
                    });
                });
                ui.label(t!("about.copyright", year = Utc::now().year()));
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;

                    if ui.button(t!("about.view_license")).clicked() {
                        thread::spawn(|| {
                            Gui::instance()
                                .expect("unexpected failure")
                                .lock()
                                .expect("unexpected failure")
                                .show_window(Box::new(LicenseWindow::new()));
                        });
                    }
                    ui.end_row();

                    if ui.button(t!("about.open_website")).clicked() {
                        Application::OpenURL(WEBSITE_URL.to_il2cpp_string());
                    }

                    if ui.button(t!("about.view_source_code")).clicked() {
                        Application::OpenURL(format!("https://github.com/{}", REPO_PATH).to_il2cpp_string());
                    }
                });
            });

        open
    }
}
