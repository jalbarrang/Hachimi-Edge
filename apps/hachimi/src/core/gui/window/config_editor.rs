use std::{borrow::Cow, ops::RangeInclusive, sync::Arc, thread};

use rust_i18n::t;

use crate::core::{
    hachimi::{self, Language},
    utils::get_localized_string,
    Hachimi,
};
use crate::il2cpp::hook::{
    umamusume::{
        CameraData::ShadowResolution,
        CySpringController::SpringUpdateMode,
        GraphicSettings::{GraphicsQuality, MsaaQuality},
        TimeUtil::BgSeason,
    },
    UnityEngine_CoreModule::Texture::AnisoLevel,
};

use super::super::scale::get_scale;
use super::super::Gui;
use super::{
    new_window, random_id, save_and_reload_config, simple_window_layout, LiveVocalsSwapWindow, SimpleOkDialog,
    ThemeEditorWindow, Window,
};

pub(crate) struct ConfigEditor {
    last_ptr_config: usize,
    config: hachimi::Config,
    id: egui::Id,
    current_tab: ConfigEditorTab,
}

#[derive(Eq, PartialEq, Clone, Copy)]
enum ConfigEditorTab {
    General,
    Graphics,
    Gameplay,
}

impl ConfigEditorTab {
    fn display_list() -> [(ConfigEditorTab, Cow<'static, str>); 3] {
        [
            (ConfigEditorTab::General, t!("config_editor.general_tab")),
            (ConfigEditorTab::Graphics, t!("config_editor.graphics_tab")),
            (ConfigEditorTab::Gameplay, t!("config_editor.gameplay_tab")),
        ]
    }
}

impl ConfigEditor {
    pub fn new() -> ConfigEditor {
        let handle = Hachimi::instance().config.load();
        ConfigEditor {
            last_ptr_config: Arc::as_ptr(&handle) as usize,
            config: (**Hachimi::instance().config.load()).clone(),
            id: random_id(),
            current_tab: ConfigEditorTab::General,
        }
    }

    fn restore_defaults(&mut self) {
        let current_language = self.config.language;
        self.config = hachimi::Config::default();
        self.config.language = current_language;
    }

    fn option_slider<Num: egui::emath::Numeric>(
        ui: &mut egui::Ui,
        label: &str,
        value: &mut Option<Num>,
        range: RangeInclusive<Num>,
    ) {
        let mut checked = value.is_some();
        ui.label(label);
        ui.checkbox(&mut checked, t!("enable"));
        ui.end_row();

        if checked && value.is_none() {
            *value = Some(*range.start())
        } else if !checked && value.is_some() {
            *value = None;
        }

        if let Some(num) = value.as_mut() {
            ui.label("");
            ui.add(egui::Slider::new(num, range));
            ui.end_row();
        }
    }

    fn run_options_grid(config: &mut hachimi::Config, ui: &mut egui::Ui, tab: ConfigEditorTab) {
        let scale = get_scale(ui.ctx());
        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);

        match tab {
            ConfigEditorTab::General => {
                ui.label(t!("config_editor.language"));
                let lang_changed = Gui::run_combo(ui, "language", &mut config.language, Language::CHOICES);
                if lang_changed {
                    config.language.set_locale();
                }
                ui.end_row();

                ui.label(t!("config_editor.disable_overlay"));
                if ui.checkbox(&mut config.disable_gui, "").clicked() && config.disable_gui {
                    thread::spawn(|| {
                        Gui::instance()
                            .expect("unexpected failure")
                            .lock()
                            .expect("unexpected failure")
                            .show_window(Box::new(SimpleOkDialog::new(
                                &t!("warning"),
                                &t!("config_editor.disable_overlay_warning"),
                                || {},
                            )));
                    });
                }
                ui.end_row();

                ui.label(t!("config_editor.ipv4_only"));
                ui.checkbox(&mut config.ipv4_only, "");
                ui.end_row();

                ui.label(t!("config_editor.meta_index_url"));
                let res = ui.add(egui::TextEdit::singleline(&mut config.meta_index_url).lock_focus(true));
                #[cfg(target_os = "windows")]
                if res.has_focus() {
                    ui.memory_mut(|mem| {
                        mem.set_focus_lock_filter(
                            res.id,
                            egui::EventFilter {
                                tab: true,
                                horizontal_arrows: true,
                                vertical_arrows: true,
                                escape: true,
                            },
                        )
                    });
                }
                ui.end_row();

                ui.label(t!("config_editor.gui_scale"));
                ui.add(egui::Slider::new(&mut config.gui_scale, 0.25..=2.0).step_by(0.05));
                ui.end_row();

                #[cfg(target_os = "windows")]
                {
                    ui.label(t!("config_editor.gui_landscape_ratio"));
                    ui.add(
                        egui::Slider::new(&mut config.windows.gui_landscape_ratio, 0.25..=1.0)
                            .step_by(0.05)
                            .fixed_decimals(2),
                    );
                    ui.end_row();
                }

                ui.label(t!("theme_editor.title"));
                ui.horizontal(|ui| {
                    if ui.button(t!("open")).clicked() {
                        thread::spawn(|| {
                            Gui::instance()
                                .expect("unexpected failure")
                                .lock()
                                .expect("unexpected failure")
                                .show_window(Box::new(ThemeEditorWindow::new()));
                        });
                    }
                });
                ui.end_row();

                #[cfg(target_os = "windows")]
                {
                    ui.label(t!("config_editor.discord_rpc"));
                    ui.checkbox(&mut config.windows.discord_rpc, "");
                    ui.end_row();

                    ui.label(t!("config_editor.menu_open_key"));
                    ui.horizontal(|ui| {
                        ui.label(crate::windows::utils::vk_to_display_label(config.windows.menu_open_key));
                        if ui.button(t!("config_editor.menu_open_key_set")).clicked() {
                            crate::windows::wnd_hook::start_menu_key_capture();
                            thread::spawn(|| {
                                Gui::instance()
                                    .expect("unexpected failure")
                                    .lock()
                                    .expect("unexpected failure")
                                    .show_notification(&t!("notification.press_to_set_menu_key"));
                            });
                        }
                    });
                    ui.end_row();
                }

                ui.label(t!("config_editor.debug_mode"));
                ui.checkbox(&mut config.debug_mode, "");
                ui.end_row();

                ui.label(t!("config_editor.enable_file_logging"));
                ui.checkbox(&mut config.enable_file_logging, "");
                ui.end_row();

                ui.label(t!("config_editor.apply_atlas_workaround"));
                ui.checkbox(&mut config.apply_atlas_workaround, "");
                ui.end_row();

                ui.label(t!("config_editor.translator_mode"));
                ui.checkbox(&mut config.translator_mode, "");
                ui.end_row();

                ui.label(t!("config_editor.skip_first_time_setup"));
                ui.checkbox(&mut config.skip_first_time_setup, "");
                ui.end_row();

                ui.label(t!("config_editor.lazy_translation_updates"));
                ui.checkbox(&mut config.lazy_translation_updates, "");
                ui.end_row();

                ui.label(t!("config_editor.disable_auto_update_check"));
                ui.checkbox(&mut config.disable_auto_update_check, "");
                ui.end_row();

                ui.label(t!("config_editor.disable_translations"));
                ui.checkbox(&mut config.disable_translations, "");
                ui.end_row();

                ui.label(t!("config_editor.enable_ipc"));
                ui.checkbox(&mut config.enable_ipc, "");
                ui.end_row();

                ui.label(t!("config_editor.ipc_listen_all"));
                ui.checkbox(&mut config.ipc_listen_all, "");
                ui.end_row();

                ui.label(t!("config_editor.auto_translate_stories"));
                if ui.checkbox(&mut config.auto_translate_stories, "").clicked() && config.auto_translate_stories {
                    thread::spawn(|| {
                        Gui::instance()
                            .expect("unexpected failure")
                            .lock()
                            .expect("unexpected failure")
                            .show_window(Box::new(SimpleOkDialog::new(
                                &t!("warning"),
                                &t!("config_editor.auto_tl_warning"),
                                || {},
                            )));
                    });
                }
                ui.end_row();

                ui.label(t!("config_editor.auto_translate_ui"));
                if ui.checkbox(&mut config.auto_translate_localize, "").clicked() && config.auto_translate_localize {
                    thread::spawn(|| {
                        Gui::instance()
                            .expect("unexpected failure")
                            .lock()
                            .expect("unexpected failure")
                            .show_window(Box::new(SimpleOkDialog::new(
                                &t!("warning"),
                                &t!("config_editor.auto_tl_warning"),
                                || {},
                            )));
                    });
                }
                ui.end_row();
            }

            ConfigEditorTab::Graphics => {
                Self::option_slider(ui, &t!("config_editor.target_fps"), &mut config.target_fps, 30..=1000);

                ui.label(t!("config_editor.virtual_resolution_multiplier"));
                ui.add(egui::Slider::new(&mut config.virtual_res_mult, 1.0..=4.0).step_by(0.1));
                ui.end_row();

                ui.label(t!("config_editor.ui_scale"));
                ui.add(egui::Slider::new(&mut config.ui_scale, 0.1..=10.0).step_by(0.05));
                ui.end_row();

                ui.label(t!("config_editor.ui_animation_scale"));
                ui.add(egui::Slider::new(&mut config.ui_animation_scale, 0.1..=10.0).step_by(0.1));
                ui.end_row();

                ui.label(t!("config_editor.render_scale"));
                ui.add(egui::Slider::new(&mut config.render_scale, 0.1..=10.0).step_by(0.1));
                ui.end_row();

                ui.label(t!("config_editor.msaa"));
                Gui::run_combo(
                    ui,
                    "msaa",
                    &mut config.msaa,
                    &[
                        (MsaaQuality::Disabled, &t!("default")),
                        (MsaaQuality::_2x, "2x"),
                        (MsaaQuality::_4x, "4x"),
                        (MsaaQuality::_8x, "8x"),
                    ],
                );
                ui.end_row();

                ui.label(t!("config_editor.aniso_level"));
                Gui::run_combo(
                    ui,
                    "aniso_level",
                    &mut config.aniso_level,
                    &[
                        (AnisoLevel::Default, &t!("default")),
                        (AnisoLevel::_2x, "2x"),
                        (AnisoLevel::_4x, "4x"),
                        (AnisoLevel::_8x, "8x"),
                        (AnisoLevel::_16x, "16x"),
                    ],
                );
                ui.end_row();

                ui.label(t!("config_editor.shadow_resolution"));
                Gui::run_combo(
                    ui,
                    "shadow_resolution",
                    &mut config.shadow_resolution,
                    &[
                        (ShadowResolution::Default, &t!("default")),
                        (ShadowResolution::_256, "256x"),
                        (ShadowResolution::_512, "512x"),
                        (ShadowResolution::_1024, "1K"),
                        (ShadowResolution::_2048, "2K"),
                        (ShadowResolution::_4096, "4K"),
                    ],
                );
                ui.end_row();

                ui.label(t!("config_editor.graphics_quality"));
                Gui::run_combo(
                    ui,
                    "graphics_quality",
                    &mut config.graphics_quality,
                    &[
                        (GraphicsQuality::Default, &t!("default")),
                        (GraphicsQuality::Toon1280, "Toon1280"),
                        (GraphicsQuality::Toon1280x2, "Toon1280x2"),
                        (GraphicsQuality::Toon1280x4, "Toon1280x4"),
                        (GraphicsQuality::ToonFull, "ToonFull"),
                        (GraphicsQuality::Max, "Max"),
                    ],
                );
                ui.end_row();

                #[cfg(target_os = "windows")]
                {
                    use crate::windows::hachimi_impl::{FullScreenMode, ResolutionScaling};

                    ui.label(t!("config_editor.vsync"));
                    Gui::run_vsync_combo(ui, &mut config.windows.vsync_count);
                    ui.end_row();

                    ui.label(t!("config_editor.auto_full_screen"));
                    ui.checkbox(&mut config.windows.auto_full_screen, "");
                    ui.end_row();

                    ui.label(t!("config_editor.full_screen_mode"));
                    Gui::run_combo(
                        ui,
                        "full_screen_mode",
                        &mut config.windows.full_screen_mode,
                        &[
                            (
                                FullScreenMode::ExclusiveFullScreen,
                                &t!("config_editor.full_screen_mode_exclusive"),
                            ),
                            (
                                FullScreenMode::FullScreenWindow,
                                &t!("config_editor.full_screen_mode_borderless"),
                            ),
                        ],
                    );
                    ui.end_row();

                    ui.label(t!("config_editor.block_minimize_in_full_screen"));
                    ui.checkbox(&mut config.windows.block_minimize_in_full_screen, "");
                    ui.end_row();

                    ui.label(t!("config_editor.resolution_scaling"));
                    Gui::run_combo(
                        ui,
                        "resolution_scaling",
                        &mut config.windows.resolution_scaling,
                        &[
                            (
                                ResolutionScaling::Default,
                                &t!("config_editor.resolution_scaling_default"),
                            ),
                            (
                                ResolutionScaling::ScaleToScreenSize,
                                &t!("config_editor.resolution_scaling_ssize"),
                            ),
                            (
                                ResolutionScaling::ScaleToWindowSize,
                                &t!("config_editor.resolution_scaling_wsize"),
                            ),
                        ],
                    );
                    ui.end_row();

                    ui.label(t!("config_editor.window_always_on_top"));
                    ui.checkbox(&mut config.windows.window_always_on_top, "");
                    ui.end_row();
                }
            }

            ConfigEditorTab::Gameplay => {
                ui.label(t!("config_editor.physics_update_mode"));
                Gui::run_combo(
                    ui,
                    "physics_update_mode",
                    &mut config.physics_update_mode,
                    &[
                        (None, &t!("default")),
                        (SpringUpdateMode::ModeNormal.into(), "ModeNormal"),
                        (SpringUpdateMode::Mode60FPS.into(), "Mode60FPS"),
                        (SpringUpdateMode::SkipFrame.into(), "SkipFrame"),
                        (SpringUpdateMode::SkipFramePostAlways.into(), "SkipFramePostAlways"),
                    ],
                );
                ui.end_row();

                ui.label(t!("config_editor.story_choice_auto_select_delay"));
                ui.add(egui::Slider::new(&mut config.story_choice_auto_select_delay, 0.1..=10.0).step_by(0.05));
                ui.end_row();

                ui.label(t!("config_editor.story_text_speed_multiplier"));
                ui.add(egui::Slider::new(&mut config.story_tcps_multiplier, 0.1..=10.0).step_by(0.1));
                ui.end_row();

                ui.label(t!("config_editor.force_allow_dynamic_camera"));
                ui.checkbox(&mut config.force_allow_dynamic_camera, "");
                ui.end_row();

                ui.label(t!("config_editor.live_theater_allow_same_chara"));
                ui.checkbox(&mut config.live_theater_allow_same_chara, "");
                ui.end_row();

                ui.label(t!("config_editor.live_vocals_swap"));
                ui.horizontal(|ui| {
                    if ui.button(t!("open")).clicked() {
                        thread::spawn(|| {
                            Gui::instance()
                                .expect("unexpected failure")
                                .lock()
                                .expect("unexpected failure")
                                .show_window(Box::new(LiveVocalsSwapWindow::new()));
                        });
                    }
                });
                ui.end_row();

                ui.label(t!("config_editor.skill_info_dialog"));
                ui.checkbox(&mut config.skill_info_dialog, "");
                ui.end_row();

                ui.label(t!("config_editor.homescreen_bgseason"));
                Gui::run_combo(
                    ui,
                    "homescreen_bgseason",
                    &mut config.homescreen_bgseason,
                    &[
                        (BgSeason::None, &t!("default")),
                        // Season text from TextId enum
                        (BgSeason::Spring, get_localized_string("Common0108").as_str()),
                        (BgSeason::Summer, get_localized_string("Common0109").as_str()),
                        (BgSeason::Fall, get_localized_string("Common0110").as_str()),
                        (BgSeason::Winter, get_localized_string("Common0111").as_str()),
                        (BgSeason::CherryBlossom, get_localized_string("Common0112").as_str()),
                    ],
                );
                ui.end_row();

                ui.label(t!("config_editor.disable_skill_name_translation"));
                ui.checkbox(&mut config.disable_skill_name_translation, "");
                ui.end_row();

                ui.label(t!("config_editor.hide_ingame_ui_hotkey"));
                if ui.checkbox(&mut config.hide_ingame_ui_hotkey, "").clicked() && config.hide_ingame_ui_hotkey {
                    thread::spawn(|| {
                        Gui::instance()
                            .expect("unexpected failure")
                            .lock()
                            .expect("unexpected failure")
                            .show_window(Box::new(SimpleOkDialog::new(
                                &t!("info"),
                                &t!("config_editor.hide_ingame_ui_hotkey_info"),
                                || {},
                            )));
                    });
                }
                ui.end_row();
            }
        }

        // Column widths workaround
        ui.horizontal(|ui| ui.add_space(100.0 * scale));
        ui.horizontal(|ui| ui.add_space(150.0 * scale));
        ui.end_row();
    }
}

impl Window for ConfigEditor {
    fn run(&mut self, ctx: &egui::Context) -> bool {
        let scale = get_scale(ctx);

        let mut open = true;
        let mut open2 = true;
        let global_handle = Hachimi::instance().config.load();
        let global_ptr = Arc::as_ptr(&global_handle) as usize;

        // sync config between diff windows
        if global_ptr != self.last_ptr_config {
            self.config = (**global_handle).clone();
            self.last_ptr_config = global_ptr;
        }
        let mut config = self.config.clone();
        #[cfg(target_os = "windows")]
        {
            config.windows.menu_open_key = global_handle.windows.menu_open_key;
        }
        let mut reset_clicked = false;

        new_window(ctx, self.id, t!("config_editor.title"))
            .open(&mut open)
            .show(ctx, |ui| {
                simple_window_layout(
                    ui,
                    self.id,
                    |ui| {
                        egui::ScrollArea::horizontal().id_salt("tabs_scroll").show(ui, |ui| {
                            ui.horizontal(|ui| {
                                let style = ui.style_mut();
                                style.spacing.button_padding = egui::vec2(8.0, 5.0);
                                style.spacing.item_spacing = egui::Vec2::ZERO;
                                let widgets = &mut style.visuals.widgets;
                                widgets.inactive.corner_radius = egui::CornerRadius::ZERO;
                                widgets.hovered.corner_radius = egui::CornerRadius::ZERO;
                                widgets.active.corner_radius = egui::CornerRadius::ZERO;

                                for (tab, label) in ConfigEditorTab::display_list() {
                                    if ui.selectable_label(self.current_tab == tab, label.as_ref()).clicked() {
                                        self.current_tab = tab;
                                    }
                                }
                            });
                        });

                        ui.add_space(4.0);

                        egui::ScrollArea::vertical().id_salt("body_scroll").show(ui, |ui| {
                            egui::Frame::NONE
                                .inner_margin(egui::Margin::symmetric(8, 0))
                                .show(ui, |ui| {
                                    egui::Grid::new(self.id.with("options_grid"))
                                        .striped(true)
                                        .num_columns(2)
                                        .spacing([40.0 * scale, 4.0 * scale])
                                        .show(ui, |ui| {
                                            Self::run_options_grid(&mut config, ui, self.current_tab);
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
                                    open2 = false;
                                }
                                if ui.button(t!("save")).clicked() {
                                    save_and_reload_config(self.config.clone());
                                    open2 = false;
                                }
                            });
                        });
                    },
                );
            });

        self.config = config;

        if reset_clicked {
            self.restore_defaults();
        }

        open &= open2;
        if !open {
            let config_locale = Hachimi::instance().config.load().language.locale_str();
            if config_locale != &*rust_i18n::locale() {
                rust_i18n::set_locale(config_locale);
            }
        }

        open
    }
}
