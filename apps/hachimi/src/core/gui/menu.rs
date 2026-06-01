//! L1 "Control Center": a hotkey-toggled `egui::Modal` with fixed top tabs
//! (Settings · Plugins · Overlay · About). Replaces the old left `SidePanel`.
//! Tab bodies live in `gui/tabs/`; this module owns the modal shell + tab bar
//! plus shared combo helpers and the game-UI toggle.

use std::borrow::Cow;

use rust_i18n::t;

use crate::core::utils::SendPtr;

use super::scale::get_scale;
use super::window::BoxedWindow;
use super::{Gui, DISABLED_GAME_UIS};

/// Fixed top-level tabs of the Control Center.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum ControlTab {
    #[default]
    Settings,
    Plugins,
    Overlay,
    About,
}

impl Gui {
    pub(crate) fn run_menu(&mut self) {
        // Notifications can be queued from anywhere; drain them every frame.
        for message in crate::core::plugin::notification::drain() {
            self.show_notification(&message);
        }

        if self.show_menu {
            self.run_control_center();
        }

        // The modal has no slide-out animation, so release input as soon as it closes.
        if !self.show_menu {
            self.menu_visible = false;
        }
    }

    /// Draw the modal shell with the top tab bar and dispatch to the active tab.
    fn run_control_center(&mut self) {
        let ctx = self.context.clone();
        let scale = get_scale(&ctx);

        let mut show_notification: Option<Cow<'_, str>> = None;
        let mut show_window: Option<BoxedWindow> = None;
        let mut keep_open = true;

        let response = egui::Modal::new(egui::Id::new("hachimi_control_center")).show(&ctx, |ui| {
            ui.set_width(550.0 * scale);
            ui.set_max_height(ctx.input(|i| i.viewport_rect().height()) * 0.85);

            // Header row: icon + title + version, close button on the right.
            ui.horizontal(|ui| {
                ui.add(Self::icon(&ctx));
                ui.heading(t!("hachimi"));
                ui.label(env!("HACHIMI_DISPLAY_VERSION"));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("\u{f00d}").on_hover_text(t!("menu.close_menu")).clicked() {
                        keep_open = false;
                    }
                });
            });

            // Fixed top tab bar.
            ui.horizontal(|ui| {
                self.tab_button(ui, ControlTab::Settings, "\u{f013} Settings");
                self.tab_button(ui, ControlTab::Plugins, "\u{f12e} Plugins");
                self.tab_button(ui, ControlTab::Overlay, "\u{f2d0} Overlay");
                self.tab_button(ui, ControlTab::About, "\u{f129} About");
            });
            ui.separator();

            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| match self.menu_tab {
                    ControlTab::Settings => self.run_settings_tab(ui, &ctx, &mut show_window, &mut show_notification),
                    ControlTab::Plugins => self.run_plugins_tab(ui, &ctx, &mut show_notification),
                    ControlTab::Overlay => self.run_overlay_settings_tab(ui),
                    ControlTab::About => self.run_about_tab(ui, &ctx, &mut show_window),
                });
        });

        // Close on backdrop click / Escape, or via the header button.
        if response.should_close() || !keep_open {
            self.show_menu = false;
            self.menu_anim_time = None;
        }

        if let Some(content) = show_notification {
            self.show_notification(content.as_ref());
        }
        if let Some(window) = show_window {
            self.show_window(window);
        }
    }

    fn tab_button(&mut self, ui: &mut egui::Ui, tab: ControlTab, label: &str) {
        if ui.selectable_label(self.menu_tab == tab, label).clicked() {
            self.menu_tab = tab;
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
                    ui.add_sized(
                        [ui.available_width() - 30.0 * scale, row_height],
                        egui::TextEdit::singleline(search_term).hint_text(t!("search_filter")),
                    );

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
