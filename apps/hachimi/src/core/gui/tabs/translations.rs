//! L1 Translations tab — translation actions, dictionary stats, and the
//! translation-related config settings (sharing the config editor's working copy).

use std::borrow::Cow;

use rust_i18n::t;

use crate::core::gui::widgets;
use crate::core::gui::Gui;
use crate::core::Hachimi;
use crate::il2cpp::{hook::umamusume::Localize, symbols::Thread};

impl Gui {
    pub(crate) fn run_translations_tab(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        show_notification: &mut Option<Cow<'_, str>>,
    ) {
        let hachimi = Hachimi::instance();
        let localized_data = hachimi.localized_data.load();
        let localize_dict_count = localized_data.localize_dict.len().to_string();
        let hashed_dict_count = localized_data.hashed_dict.len().to_string();

        ui.horizontal_wrapped(|ui| {
            if ui.button(t!("menu.reload_localized_data")).clicked() {
                hachimi.load_localized_data();
                *show_notification = Some(t!("notification.localized_data_reloaded"));
            }
            if ui.button(t!("menu.tl_check_for_updates")).clicked() {
                hachimi.tl_updater.clone().check_for_updates(false);
            }
            if ui.button(t!("menu.tl_check_for_updates_pedantic")).clicked() {
                hachimi.tl_updater.clone().check_for_updates(true);
            }
            if hachimi.config.load().translator_mode && ui.button(t!("menu.dump_localize_dict")).clicked() {
                Thread::main_thread().schedule(|| {
                    let data = Localize::dump_strings();
                    let dict_path = Hachimi::instance().get_data_path("localize_dump.json");
                    let mut gui = Gui::instance()
                        .expect("unexpected failure")
                        .lock()
                        .expect("lock poisoned");
                    if let Err(e) = crate::core::utils::write_json_file(&data, dict_path) {
                        gui.show_notification(&e.to_string())
                    } else {
                        gui.show_notification(&t!("notification.saved_localize_dump"))
                    }
                })
            }
        });

        widgets::section_header(ui, t!("menu.stats_heading"));
        ui.label(t!("menu.localize_dict_entries", count = localize_dict_count));
        ui.label(t!("menu.hashed_dict_entries", count = hashed_dict_count));

        widgets::section_header(ui, t!("translations.settings_heading"));
        self.config_editor.ui_translations(ui, ctx);
        self.config_editor.ui_footer(ui);
    }
}
