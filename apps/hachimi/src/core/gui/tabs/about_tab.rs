//! L1 About tab — about info, update check, stats, and the danger zone.

use std::borrow::Cow;

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
        show_notification: &mut Option<Cow<'_, str>>,
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

        if ui.button(t!("menu.check_for_updates")).clicked() {
            Hachimi::instance().updater.clone().check_for_updates(|_| {});
        }

        widgets::section_header(ui, t!("menu.stats_heading"));
        ui.label(&self.fps_text);

        widgets::section_header(ui, t!("menu.danger_zone_heading"));
        ui.label(t!("menu.danger_zone_warning"));
        if ui.button(t!("menu.soft_restart")).clicked() {
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
        if ui.button(t!("menu.toggle_game_ui")).clicked() {
            Thread::main_thread().schedule(Self::toggle_game_ui);
        }
        if ui.button(t!("menu.reload_plugins")).clicked() {
            let (reloaded, skipped) = crate::core::plugin::reload_all();
            *show_notification = Some(format!("Reloaded {reloaded} plugin(s), skipped {skipped}").into());
        }
    }
}
