//! L1 Config tab — config actions plus the embedded config editor
//! (General / Graphics / Gameplay sub-tabs + shared Save/Revert/Restore footer).

use std::borrow::Cow;

use rust_i18n::t;

use crate::core::gui::widgets;
use crate::core::gui::window::FirstTimeSetupWindow;
use crate::core::gui::Gui;
use crate::core::Hachimi;

impl Gui {
    pub(crate) fn run_config_tab(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        show_notification: &mut Option<Cow<'_, str>>,
    ) {
        let hachimi = Hachimi::instance();

        widgets::section_header(ui, t!("config_editor.general_tab").into_owned());
        ui.horizontal_wrapped(|ui| {
            if widgets::primary_button(ui, t!("menu.reload_config").into_owned()).clicked() {
                hachimi.reload_config();
                *show_notification = Some(t!("notification.config_reloaded"));
            }
            if widgets::secondary_button(ui, t!("menu.open_first_time_setup").into_owned()).clicked() {
                self.show_window(Box::new(FirstTimeSetupWindow::new()));
            }
        });
        ui.add_space(8.0);

        self.config_editor.ui_editor(ui, ctx);
        self.config_editor.ui_footer(ui);
    }
}
