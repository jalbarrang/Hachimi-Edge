//! L1 About tab — about info, update check, stats, and the danger zone.

use chrono::{Datelike, Utc};
use rust_i18n::t;

use crate::core::gui::widgets;
use crate::core::gui::window::{BoxedWindow, LicenseWindow, SimpleYesNoDialog};
use crate::core::gui::Gui;
use crate::core::hachimi::{REPO_PATH, WEBSITE_URL};
use crate::core::Hachimi;
use crate::il2cpp::{
    ext::StringExt,
    hook::{umamusume::GameSystem, UnityEngine_CoreModule::Application},
    symbols::Thread,
};

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
            if widgets::secondary_button(ui, t!("about.view_license").into_owned()).clicked() {
                *show_window = Some(Box::new(LicenseWindow::new()));
            }
            if widgets::secondary_button(ui, t!("about.open_website").into_owned()).clicked() {
                Application::OpenURL(WEBSITE_URL.to_il2cpp_string());
            }
            if widgets::secondary_button(ui, t!("about.view_source_code").into_owned()).clicked() {
                Application::OpenURL(format!("https://github.com/{}", REPO_PATH).to_il2cpp_string());
            }
        });

        ui.horizontal_wrapped(|ui| {
            if widgets::primary_button(ui, t!("menu.check_for_updates").into_owned()).clicked() {
                Hachimi::instance().updater.clone().check_for_updates(|_| {});
            }
            if widgets::secondary_button(ui, t!("menu.sync_gametora_data").into_owned()).clicked() {
                Hachimi::instance().gametora_updater.clone().sync(true);
            }
        });

        widgets::section_header(ui, t!("menu.stats_heading"));
        ui.label(&self.fps_text);

        widgets::section_header(ui, t!("menu.danger_zone_heading"));
        ui.label(t!("menu.danger_zone_warning"));
        if widgets::danger_button(ui, t!("menu.soft_restart").into_owned()).clicked() {
            *show_window = Some(Box::new(SimpleYesNoDialog::new(
                &t!("confirm_dialog_title"),
                &t!("soft_restart_confirm_content"),
                |ok| {
                    if !ok {
                        return;
                    }
                    Thread::main_thread().schedule(|| {
                        GameSystem::SoftwareReset(GameSystem::instance());
                    });
                },
            )));
        }
        if widgets::secondary_button(ui, t!("menu.toggle_game_ui").into_owned()).clicked() {
            Thread::main_thread().schedule(Self::toggle_game_ui);
        }
    }
}
