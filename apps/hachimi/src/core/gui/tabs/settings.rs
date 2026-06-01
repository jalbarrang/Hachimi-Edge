//! L1 Settings tab — the migrated Hachimi configuration UI (formerly the body
//! of the left `SidePanel`). Stats, config, graphics, translation, danger zone.

use std::borrow::Cow;
use std::sync::atomic;

use rust_i18n::t;

use crate::core::gui::widgets;
use crate::core::gui::window::{BoxedWindow, ConfigEditor, FirstTimeSetupWindow, SimpleYesNoDialog};
use crate::core::gui::Gui;
use crate::core::Hachimi;
use crate::il2cpp::{
    hook::{
        umamusume::{GameSystem, Localize},
        UnityEngine_CoreModule::Application,
    },
    symbols::Thread,
};

impl Gui {
    pub(crate) fn run_settings_tab(
        &mut self,
        ui: &mut egui::Ui,
        _ctx: &egui::Context,
        show_window: &mut Option<BoxedWindow>,
        show_notification: &mut Option<Cow<'_, str>>,
    ) {
        let hachimi = Hachimi::instance();
        let localized_data = hachimi.localized_data.load();
        let localize_dict_count = localized_data.localize_dict.len().to_string();
        let hashed_dict_count = localized_data.hashed_dict.len().to_string();

        if ui.button(t!("menu.check_for_updates")).clicked() {
            Hachimi::instance().updater.clone().check_for_updates(|_| {});
        }

        widgets::section_header(ui, t!("menu.stats_heading"));
        ui.label(&self.fps_text);
        ui.label(t!("menu.localize_dict_entries", count = localize_dict_count));
        ui.label(t!("menu.hashed_dict_entries", count = hashed_dict_count));

        widgets::section_header(ui, t!("menu.config_heading"));
        if ui.button(t!("menu.open_config_editor")).clicked() {
            *show_window = Some(Box::new(ConfigEditor::new()));
        }
        if ui.button(t!("menu.reload_config")).clicked() {
            hachimi.reload_config();
            *show_notification = Some(t!("notification.config_reloaded"));
        }
        if ui.button(t!("menu.open_first_time_setup")).clicked() {
            *show_window = Some(Box::new(FirstTimeSetupWindow::new()));
        }

        widgets::section_header(ui, t!("menu.graphics_heading"));
        ui.horizontal(|ui| {
            ui.label(t!("menu.fps_label"));
            let res = ui.add(egui::Slider::new(&mut self.menu_fps_value, 30..=1000));
            if res.lost_focus() || res.drag_stopped() {
                hachimi.target_fps.store(self.menu_fps_value, atomic::Ordering::Relaxed);
                Thread::main_thread().schedule(|| {
                    Application::set_targetFrameRate(30);
                });
            }
        });
        #[cfg(target_os = "windows")]
        self.run_graphics_windows(ui, &hachimi);

        widgets::section_header(ui, t!("menu.translation_heading"));
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

    #[cfg(target_os = "windows")]
    fn run_graphics_windows(&mut self, ui: &mut egui::Ui, hachimi: &Hachimi) {
        use crate::il2cpp::hook::UnityEngine_CoreModule::QualitySettings;
        use crate::windows::{discord, utils::set_window_topmost, wnd_hook};

        ui.horizontal(|ui| {
            let prev_value = self.menu_vsync_value;
            ui.label(t!("menu.vsync_label"));
            Self::run_vsync_combo(ui, &mut self.menu_vsync_value);
            if prev_value != self.menu_vsync_value {
                hachimi
                    .vsync_count
                    .store(self.menu_vsync_value, atomic::Ordering::Relaxed);
                Thread::main_thread().schedule(|| {
                    QualitySettings::set_vSyncCount(1);
                });
            }
        });

        let mut top = hachimi.window_always_on_top.load(atomic::Ordering::Relaxed);
        if widgets::toggle_row(ui, t!("menu.stay_on_top"), &mut top).changed() {
            hachimi.window_always_on_top.store(top, atomic::Ordering::Relaxed);
            Thread::main_thread().schedule(|| {
                let topmost = Hachimi::instance().window_always_on_top.load(atomic::Ordering::Relaxed);
                // SAFETY: FFI / raw pointer operation required by IL2CPP interop
                unsafe {
                    _ = set_window_topmost(wnd_hook::get_target_hwnd(), topmost);
                }
            });
        }

        let mut rpc = hachimi.discord_rpc.load(atomic::Ordering::Relaxed);
        if widgets::toggle_row(ui, t!("menu.discord_rpc"), &mut rpc).changed() {
            hachimi.discord_rpc.store(rpc, atomic::Ordering::Relaxed);
            if let Err(e) = if rpc { discord::start_rpc() } else { discord::stop_rpc() } {
                error!("{}", e);
            }
        }
    }
}
