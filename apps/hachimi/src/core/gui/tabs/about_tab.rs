//! L1 About tab — reuses the existing about content inline in the modal.

use chrono::{Datelike, Utc};
use rust_i18n::t;

use crate::core::gui::window::{BoxedWindow, LicenseWindow};
use crate::core::gui::Gui;
use crate::core::hachimi::{REPO_PATH, WEBSITE_URL};
use crate::il2cpp::{ext::StringExt, hook::UnityEngine_CoreModule::Application};

impl Gui {
    pub(crate) fn run_about_tab(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        show_window: &mut Option<BoxedWindow>,
    ) {
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.add(Self::icon_2x(ctx));
            ui.vertical(|ui| {
                ui.heading(t!("hachimi"));
                ui.label(env!("HACHIMI_DISPLAY_VERSION"));
            });
        });
        ui.label(t!("about.copyright", year = Utc::now().year()));
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            if ui.button(t!("about.view_license")).clicked() {
                *show_window = Some(Box::new(LicenseWindow::new()));
            }
            if ui.button(t!("about.open_website")).clicked() {
                Application::OpenURL(WEBSITE_URL.to_il2cpp_string());
            }
            if ui.button(t!("about.view_source_code")).clicked() {
                Application::OpenURL(format!("https://github.com/{}", REPO_PATH).to_il2cpp_string());
            }
        });
    }
}
