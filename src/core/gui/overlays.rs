use std::os::raw::c_void;
use std::panic::{self, AssertUnwindSafe};

use crate::core::plugin::overlay;

use super::scale::get_scale;
use super::Gui;

impl Gui {
    pub(crate) fn run_overlays(&mut self) {
        let overlays = overlay::get_plugin_overlays();
        if overlays.is_empty() {
            return;
        }

        let ctx = &self.context;
        let scale = get_scale(ctx);

        for ov in overlays.iter() {
            let mut visible = overlay::is_overlay_visible(&ov.id);
            if !visible {
                continue;
            }

            let title: String = ov
                .id
                .split('_')
                .map(|w| {
                    let mut c = w.chars();
                    match c.next() {
                        Some(first) => first.to_uppercase().collect::<String>() + c.as_str(),
                        None => String::new(),
                    }
                })
                .collect::<Vec<_>>()
                .join(" ");

            egui::Window::new(egui::RichText::new(&title).size(12.0 * scale))
                .id(egui::Id::new("plugin_overlay").with(&ov.id))
                .open(&mut visible)
                .default_pos(egui::pos2(
                    ctx.input(|i| i.viewport_rect().right()) - 300.0 * scale,
                    8.0 * scale,
                ))
                .resizable(false)
                .collapsible(true)
                .show(ctx, |ui| {
                    let _ = panic::catch_unwind(AssertUnwindSafe(|| {
                        (ov.callback)(ui as *mut egui::Ui as *mut c_void, ov.userdata as *mut c_void);
                    }))
                    .inspect_err(|_| {
                        error!("plugin overlay callback panicked: {}", ov.id);
                    });
                });

            if !visible {
                overlay::set_overlay_visible(&ov.id, false);
            }
        }
    }
}
