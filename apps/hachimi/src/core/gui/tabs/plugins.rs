//! L1 Plugins tab — a sub-nav of plugin-registered pages (menu sections become
//! L1 pages). Scales to N plugins without N top-level tabs: pages are listed as
//! selectable chips, and the selected page's callback is rendered below.

use std::borrow::Cow;
use std::ffi::c_void;
use std::panic::{self, AssertUnwindSafe};

use crate::core::gui::scale::get_scale;
use crate::core::gui::Gui;
use crate::core::plugin::menu::{get_plugin_menu_icon, get_plugin_menu_items, get_plugin_menu_sections};
use crate::core::plugin::OwnerScope;

impl Gui {
    pub(crate) fn run_plugins_tab(
        &mut self,
        ui: &mut egui::Ui,
        _ctx: &egui::Context,
        _show_notification: &mut Option<Cow<'_, str>>,
    ) {
        let scale = get_scale(ui.ctx());
        let items = get_plugin_menu_items();
        let sections = get_plugin_menu_sections();

        if items.is_empty() && sections.is_empty() {
            ui.add_space(8.0);
            ui.weak("No plugins have registered any pages.");
            return;
        }

        // Plugin action buttons (menu items) at the top.
        if !items.is_empty() {
            ui.horizontal_wrapped(|ui| {
                for item in &items {
                    let clicked = match get_plugin_menu_icon(&item.label) {
                        Some(icon) => {
                            let size = 18.0 * scale;
                            ui.add(egui::Image::new((icon.uri, icon.bytes)).fit_to_exact_size(egui::Vec2::splat(size)));
                            ui.button(&item.label).clicked()
                        }
                        None => ui.button(&item.label).clicked(),
                    };
                    if clicked {
                        if let Some(callback) = item.callback {
                            let _ = panic::catch_unwind(AssertUnwindSafe(|| {
                                callback(item.userdata as *mut c_void);
                            }))
                            .inspect_err(|_| error!("plugin menu item callback panicked: {}", item.label));
                        }
                    }
                }
            });
            if !sections.is_empty() {
                ui.separator();
            }
        }

        if sections.is_empty() {
            return;
        }

        // Default selection: first section if nothing valid is selected.
        let selected_valid = self
            .plugins_selected
            .is_some_and(|h| sections.iter().any(|s| s.handle == h));
        if !selected_valid {
            self.plugins_selected = Some(sections[0].handle);
        }

        // Page sub-nav (chips).
        ui.horizontal_wrapped(|ui| {
            for (i, section) in sections.iter().enumerate() {
                let label = page_label(section.title.as_deref(), i);
                let selected = self.plugins_selected == Some(section.handle);
                if ui.selectable_label(selected, label).clicked() {
                    self.plugins_selected = Some(section.handle);
                }
            }
        });
        ui.separator();

        // Active page body.
        let Some(section) = sections.iter().find(|s| Some(s.handle) == self.plugins_selected) else {
            return;
        };

        if let Some(title) = &section.title {
            ui.horizontal(|ui| {
                if let Some(icon) = &section.icon {
                    let size = 18.0 * scale;
                    ui.add(
                        egui::Image::new((icon.uri.clone(), icon.bytes.clone()))
                            .fit_to_exact_size(egui::Vec2::splat(size)),
                    );
                }
                ui.heading(title);
            });
        }

        let _scope = OwnerScope::enter(section.owner);
        let _ = panic::catch_unwind(AssertUnwindSafe(|| {
            (section.callback)(ui as *mut egui::Ui as *mut c_void, section.userdata as *mut c_void);
        }))
        .inspect_err(|_| error!("plugin menu section callback panicked"));
    }
}

/// Label for a page chip: the section title, else a generic numbered fallback.
fn page_label(title: Option<&str>, index: usize) -> String {
    match title {
        Some(t) if !t.is_empty() => t.to_owned(),
        _ => format!("Plugin {}", index + 1),
    }
}
