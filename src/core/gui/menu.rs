use std::{
    borrow::Cow,
    os::raw::c_void,
    panic::{self, AssertUnwindSafe},
    sync::atomic,
    time::Instant,
};

use rust_i18n::t;

use crate::core::plugin::{
    menu::{get_plugin_menu_icon, get_plugin_menu_items, get_plugin_menu_sections},
    notification,
};
use crate::core::utils::{self, SendPtr};
use crate::core::Hachimi;
use crate::il2cpp::{
    hook::{
        umamusume::{GameSystem, Localize},
        UnityEngine_CoreModule::Application,
    },
    symbols::Thread,
};

#[cfg(target_os = "android")]
use crate::il2cpp::hook::umamusume::WebViewManager;

#[cfg(target_os = "windows")]
use crate::il2cpp::hook::UnityEngine_CoreModule::QualitySettings;

use super::scale::get_scale;
use super::window::{AboutWindow, BoxedWindow, ConfigEditor, FirstTimeSetupWindow, SimpleYesNoDialog};
use super::{Gui, DISABLED_GAME_UIS};

#[cfg(target_os = "android")]
use super::android_keyboard::ime_scroll_padding;

impl Gui {
    pub(crate) fn run_menu(&mut self) {
        let hachimi = Hachimi::instance();
        let localized_data = hachimi.localized_data.load();
        let localize_dict_count = localized_data.localize_dict.len().to_string();
        let hashed_dict_count = localized_data.hashed_dict.len().to_string();

        let mut show_notification: Option<Cow<'_, str>> = None;
        let mut show_window: Option<BoxedWindow> = None;
        {
            let ctx = &self.context;
            let scale = get_scale(ctx);
            let salt = self.finalized_scale;
            egui::SidePanel::left(egui::Id::new("hachimi_menu").with(salt.to_bits()))
                .min_width(96.0 * scale)
                .default_width(200.0 * scale)
                .show_animated(ctx, self.show_menu, |ui| {
                    ui.with_layout(egui::Layout::top_down_justified(egui::Align::TOP), |ui| {
                        #[cfg(target_os = "windows")]
                        {
                            ui.horizontal(|ui| {
                                ui.add(Self::icon(ctx));
                                ui.heading(t!("hachimi"));
                                if ui.button(" \u{f29c} ").clicked() {
                                    show_window = Some(Box::new(AboutWindow::new()));
                                }
                            });
                            ui.label(env!("HACHIMI_DISPLAY_VERSION"));
                            if ui.button(t!("menu.close_menu")).clicked() {
                                self.show_menu = false;
                                self.menu_anim_time = None;
                            }
                        }
                        // did this because android phones have a notch
                        #[cfg(target_os = "android")]
                        {
                            ui.horizontal(|ui| {
                                ui.add(Self::icon(ctx));
                                ui.heading(t!("hachimi"));
                            });
                            ui.label(env!("HACHIMI_DISPLAY_VERSION"));
                            ui.horizontal(|ui| {
                                if ui.button(t!("menu.close_menu")).clicked() {
                                    self.show_menu = false;
                                    self.menu_anim_time = None;
                                }
                                if ui.button(" \u{f29c} ").clicked() {
                                    show_window = Some(Box::new(AboutWindow::new()));
                                }
                            });
                        }
                        if ui.button(t!("menu.check_for_updates")).clicked() {
                            Hachimi::instance().updater.clone().check_for_updates(|_| {});
                        }
                        ui.separator();

                        egui::ScrollArea::vertical().show(ui, |ui| {
                            ui.heading(t!("menu.stats_heading"));
                            ui.label(&self.fps_text);
                            ui.label(t!("menu.localize_dict_entries", count = localize_dict_count));
                            ui.label(t!("menu.hashed_dict_entries", count = hashed_dict_count));
                            ui.separator();

                            ui.heading(t!("menu.config_heading"));
                            if ui.button(t!("menu.open_config_editor")).clicked() {
                                show_window = Some(Box::new(ConfigEditor::new()));
                            }
                            if ui.button(t!("menu.reload_config")).clicked() {
                                hachimi.reload_config();
                                show_notification = Some(t!("notification.config_reloaded"));
                            }
                            if ui.button(t!("menu.open_first_time_setup")).clicked() {
                                show_window = Some(Box::new(FirstTimeSetupWindow::new()));
                            }
                            ui.separator();

                            ui.heading(t!("menu.graphics_heading"));
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
                            {
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
                                ui.horizontal(|ui| {
                                    let mut value = hachimi.window_always_on_top.load(atomic::Ordering::Relaxed);

                                    ui.label(t!("menu.stay_on_top"));
                                    if ui.checkbox(&mut value, "").changed() {
                                        hachimi.window_always_on_top.store(value, atomic::Ordering::Relaxed);
                                        Thread::main_thread().schedule(|| {
                                            let topmost = Hachimi::instance()
                                                .window_always_on_top
                                                .load(atomic::Ordering::Relaxed);
                                            // SAFETY: FFI / raw pointer operation required by IL2CPP interop
                                            unsafe {
                                                _ = set_window_topmost(wnd_hook::get_target_hwnd(), topmost);
                                            }
                                        });
                                    }
                                });
                                ui.horizontal(|ui| {
                                    let mut value = hachimi.discord_rpc.load(atomic::Ordering::Relaxed);

                                    ui.label(t!("menu.discord_rpc"));
                                    if ui.checkbox(&mut value, "").changed() {
                                        hachimi.discord_rpc.store(value, atomic::Ordering::Relaxed);
                                        if let Err(e) = if value {
                                            discord::start_rpc()
                                        } else {
                                            discord::stop_rpc()
                                        } {
                                            error!("{}", e);
                                        }
                                    }
                                });
                            }
                            ui.separator();

                            ui.heading(t!("menu.translation_heading"));
                            if ui.button(t!("menu.reload_localized_data")).clicked() {
                                hachimi.load_localized_data();
                                show_notification = Some(t!("notification.localized_data_reloaded"));
                            }
                            if ui.button(t!("menu.tl_check_for_updates")).clicked() {
                                hachimi.tl_updater.clone().check_for_updates(false);
                            }
                            if ui.button(t!("menu.tl_check_for_updates_pedantic")).clicked() {
                                hachimi.tl_updater.clone().check_for_updates(true);
                            }
                            if hachimi.config.load().translator_mode
                                && ui.button(t!("menu.dump_localize_dict")).clicked()
                            {
                                Thread::main_thread().schedule(|| {
                                    let data = Localize::dump_strings();
                                    let dict_path = Hachimi::instance().get_data_path("localize_dump.json");
                                    let mut gui = Gui::instance()
                                        .expect("unexpected failure")
                                        .lock()
                                        .expect("lock poisoned");
                                    if let Err(e) = utils::write_json_file(&data, dict_path) {
                                        gui.show_notification(&e.to_string())
                                    } else {
                                        gui.show_notification(&t!("notification.saved_localize_dump"))
                                    }
                                })
                            }
                            ui.separator();

                            let plugin_items = get_plugin_menu_items();
                            if !plugin_items.is_empty() {
                                ui.heading("Plugins");
                                for item in plugin_items {
                                    let icon = get_plugin_menu_icon(&item.label);
                                    let clicked = if let Some(icon) = icon {
                                        let size = 18.0 * scale;
                                        ui.horizontal(|ui| {
                                            ui.add(
                                                egui::Image::new((icon.uri, icon.bytes))
                                                    .fit_to_exact_size(egui::Vec2::splat(size)),
                                            );
                                            ui.button(&item.label).clicked()
                                        })
                                        .inner
                                    } else {
                                        ui.button(&item.label).clicked()
                                    };
                                    if clicked {
                                        if let Some(callback) = item.callback {
                                            let _ = panic::catch_unwind(AssertUnwindSafe(|| {
                                                callback(item.userdata as *mut c_void);
                                            }))
                                            .inspect_err(|_| {
                                                error!("plugin menu item callback panicked: {}", item.label);
                                            });
                                        }
                                    }
                                }
                                ui.separator();
                            }

                            let plugin_sections = get_plugin_menu_sections();
                            if !plugin_sections.is_empty() {
                                for section in plugin_sections {
                                    if let Some(title) = section.title.clone() {
                                        let icon = section.icon.clone();
                                        let size = 18.0 * scale;
                                        ui.horizontal(|ui| {
                                            if let Some(icon) = icon {
                                                ui.add(
                                                    egui::Image::new((icon.uri, icon.bytes))
                                                        .fit_to_exact_size(egui::Vec2::splat(size)),
                                                );
                                            }
                                            ui.heading(title);
                                        });
                                    }
                                    let _ = panic::catch_unwind(AssertUnwindSafe(|| {
                                        (section.callback)(
                                            ui as *mut _ as *mut c_void,
                                            section.userdata as *mut c_void,
                                        );
                                    }))
                                    .inspect_err(|_| {
                                        error!("plugin menu section callback panicked");
                                    });
                                }
                                ui.separator();
                            }

                            ui.heading(t!("menu.danger_zone_heading"));
                            ui.vertical(|ui| {
                                ui.label(t!("menu.danger_zone_warning"));
                            });
                            if ui.button(t!("menu.soft_restart")).clicked() {
                                show_window = Some(Box::new(SimpleYesNoDialog::new(
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
                            #[cfg(not(target_os = "windows"))]
                            if ui.button(t!("menu.open_in_game_browser")).clicked() {
                                show_window = Some(Box::new(SimpleYesNoDialog::new(
                                    &t!("confirm_dialog_title"),
                                    &t!("in_game_browser_confirm_content"),
                                    |ok| {
                                        if !ok {
                                            return;
                                        }
                                        Thread::main_thread().schedule(|| {
                                            WebViewManager::quick_open(
                                                &t!("browser_dialog_title"),
                                                &Hachimi::instance().config.load().open_browser_url,
                                            );
                                        });
                                    },
                                )));
                            }
                            if ui.button(t!("menu.toggle_game_ui")).clicked() {
                                Thread::main_thread().schedule(Self::toggle_game_ui);
                            }

                            #[cfg(target_os = "android")]
                            {
                                let padding = ime_scroll_padding(ui.ctx());
                                if padding > 0.0 {
                                    ui.add_space(padding);
                                }
                            }
                        });
                    });
                });
        }

        for message in notification::drain() {
            self.show_notification(&message);
        }

        if !self.show_menu {
            if let Some(time) = self.menu_anim_time {
                if time.elapsed().as_secs_f32() >= self.context.style().animation_time {
                    self.menu_visible = false;
                }
            } else {
                self.menu_anim_time = Some(Instant::now());
            }
        }

        if let Some(content) = show_notification {
            self.show_notification(content.as_ref());
        }

        if let Some(window) = show_window {
            self.show_window(window);
        }
    }

    pub fn toggle_game_ui() {
        use crate::il2cpp::hook::{
            Plugins::AnimateToUnity::AnRoot,
            UnityEngine_CoreModule::{Behaviour, GameObject, Object},
            UnityEngine_UIModule::Canvas,
        };

        let canvas_array = Object::FindObjectsOfType(Canvas::type_object(), true);
        let an_root_array = Object::FindObjectsOfType(AnRoot::type_object(), true);
        // SAFETY: FFI / raw pointer operation required by IL2CPP interop
        let canvas_iter = unsafe { canvas_array.as_slice().iter() };
        // SAFETY: FFI / raw pointer operation required by IL2CPP interop
        let an_root_iter = unsafe { an_root_array.as_slice().iter() };

        let mut disabled_uis = DISABLED_GAME_UIS.lock().expect("lock poisoned");

        if disabled_uis.is_empty() {
            for canvas in canvas_iter {
                if Behaviour::get_enabled(*canvas) {
                    Behaviour::set_enabled(*canvas, false);
                    disabled_uis.insert(SendPtr(*canvas));
                }
            }
            for an_root in an_root_iter {
                let top_object = AnRoot::get__topObject(*an_root);
                if GameObject::get_activeSelf(top_object) {
                    GameObject::SetActive(top_object, false);
                    disabled_uis.insert(SendPtr(top_object));
                }
            }
        } else {
            for canvas in canvas_iter {
                if disabled_uis.contains(&SendPtr(*canvas)) {
                    Behaviour::set_enabled(*canvas, true);
                }
            }
            for an_root in an_root_iter {
                let top_object = AnRoot::get__topObject(*an_root);
                if disabled_uis.contains(&SendPtr(top_object)) {
                    GameObject::SetActive(top_object, true);
                }
            }
            disabled_uis.clear();
        }
    }

    #[cfg(target_os = "windows")]
    pub(crate) fn run_vsync_combo(ui: &mut egui::Ui, value: &mut i32) {
        Self::run_combo(
            ui,
            "vsync_combo",
            value,
            &[
                (-1, &t!("default")),
                (0, &t!("off")),
                (1, &t!("on")),
                (2, "1/2"),
                (3, "1/3"),
                (4, "1/4"),
            ],
        );
    }

    pub(crate) fn run_combo<T: PartialEq + Copy>(
        ui: &mut egui::Ui,
        id_child: impl std::hash::Hash,
        value: &mut T,
        choices: &[(T, &str)],
    ) -> bool {
        let mut selected = "Unknown";
        for choice in choices.iter() {
            if *value == choice.0 {
                selected = choice.1;
            }
        }

        let mut changed = false;
        egui::ComboBox::new(ui.id().with(id_child), "")
            .wrap_mode(egui::TextWrapMode::Wrap)
            .selected_text(selected)
            .show_ui(ui, |ui| {
                for choice in choices.iter() {
                    changed |= ui.selectable_value(value, choice.0, choice.1).changed();
                }
            });

        changed
    }

    pub(crate) fn run_combo_menu<T: PartialEq + Copy>(
        ui: &mut egui::Ui,
        id_salt: impl std::hash::Hash,
        value: &mut T,
        choices: &[(T, &str)],
        search_term: &mut String,
    ) -> bool {
        let mut changed = false;
        let scale = get_scale(ui.ctx());
        let fixed_width = 145.0 * scale;
        let row_height = 24.0 * scale;
        let padding = ui.spacing().button_padding;

        let button_id = ui.make_persistent_id(id_salt);
        let popup_id = button_id.with("popup");

        let selected_text = choices.iter().find(|(v, _)| v == value).map_or("Unknown", |(_, s)| *s);

        let (rect, _) = ui.allocate_exact_size(egui::vec2(fixed_width, row_height), egui::Sense::hover());
        let button_res = ui.interact(rect, button_id, egui::Sense::click());

        if ui.is_rect_visible(rect) {
            let is_open = egui::Popup::is_id_open(ui.ctx(), popup_id);
            let visuals = if is_open {
                &ui.visuals().widgets.open
            } else {
                ui.style().interact(&button_res)
            };

            ui.painter().rect(
                rect.expand(visuals.expansion),
                visuals.corner_radius,
                visuals.weak_bg_fill,
                visuals.bg_stroke,
                egui::epaint::StrokeKind::Inside,
            );

            let icon_size = 12.0 * scale;
            let icon_rect = egui::Rect::from_center_size(
                egui::pos2(rect.right() - padding.x - icon_size / 2.0, rect.center().y),
                egui::vec2(icon_size, icon_size),
            );
            Self::down_triangle_icon(ui.painter(), icon_rect, visuals);

            let galley = ui.painter().layout_no_wrap(
                selected_text.to_owned(),
                egui::TextStyle::Button.resolve(ui.style()),
                visuals.text_color(),
            );

            let text_pos = egui::pos2(rect.left() + padding.x, rect.center().y - galley.size().y / 2.0);
            ui.painter().galley(text_pos, galley, visuals.text_color());
        }

        egui::Popup::menu(&button_res)
            .id(popup_id)
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
            .show(|ui| {
                ui.set_width(fixed_width);
                ui.set_max_width(fixed_width);

                ui.horizontal(|ui| {
                    let _res = ui.add_sized(
                        [ui.available_width() - 30.0 * scale, row_height],
                        egui::TextEdit::singleline(search_term).hint_text(t!("search_filter")),
                    );
                    #[cfg(target_os = "android")]
                    handle_android_keyboard(&_res, search_term);

                    if ui.button("X").clicked() {
                        search_term.clear();
                    }
                });

                ui.separator();

                egui::ScrollArea::vertical()
                    .max_height(250.0 * scale)
                    .hscroll(false)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);

                        ui.with_layout(egui::Layout::top_down_justified(egui::Align::Min), |ui| {
                            for (choice_val, label) in choices {
                                if !search_term.is_empty()
                                    && !label.to_lowercase().contains(&search_term.to_lowercase())
                                {
                                    continue;
                                }

                                let is_selected = value == choice_val;
                                if ui.add(egui::Button::selectable(is_selected, *label)).clicked() {
                                    *value = *choice_val;
                                    changed = true;
                                    egui::Popup::close_id(ui.ctx(), popup_id);
                                    search_term.clear();
                                }
                            }
                        });
                    });
            });

        changed
    }

    // egui's code originally (https://github.com/emilk/egui/blob/main/crates/egui/src/containers/combo_box.rs)
    pub(crate) fn down_triangle_icon(painter: &egui::Painter, rect: egui::Rect, visuals: &egui::style::WidgetVisuals) {
        let rect = egui::Rect::from_center_size(rect.center(), egui::vec2(rect.width() * 0.7, rect.height() * 0.45));

        painter.add(egui::Shape::convex_polygon(
            vec![rect.left_top(), rect.right_top(), rect.center_bottom()],
            visuals.fg_stroke.color,
            visuals.fg_stroke,
        ));
    }
}
