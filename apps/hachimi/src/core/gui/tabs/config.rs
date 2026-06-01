//! L1 Config tab — config actions plus the embedded config editor
//! (General / Graphics / Gameplay sub-tabs + shared Save/Revert/Restore footer).

use std::borrow::Cow;

use rust_i18n::t;

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

        ui.horizontal_wrapped(|ui| {
            if ui.button(t!("menu.reload_config")).clicked() {
                hachimi.reload_config();
                *show_notification = Some(t!("notification.config_reloaded"));
            }
            if ui.button(t!("menu.open_first_time_setup")).clicked() {
                self.show_window(Box::new(FirstTimeSetupWindow::new()));
            }
        });
        ui.separator();

        self.config_editor.ui_editor(ui, ctx);
        self.config_editor.ui_footer(ui);
    }
}
