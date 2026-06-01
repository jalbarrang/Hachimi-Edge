//! L2 floating HUD render side. Each registered overlay is drawn as a draggable
//! `egui::Window` that toggles between a compact **badge** (collapsed) and a full
//! **panel** (expanded). A global lock makes panels non-interactive (click-through,
//! handled by the input gate). Positions/collapse/visibility persist via the
//! overlay registry.

use std::os::raw::c_void;
use std::panic::{self, AssertUnwindSafe};

use crate::core::plugin::overlay;

use super::scale::get_scale;
use super::Gui;

impl Gui {
    pub(crate) fn run_overlays(&mut self) {
        // Fire the per-frame event for subscribed plugins (independent of overlays).
        crate::core::plugin::events::dispatch_frame();

        let overlays = overlay::get_plugin_overlays();
        if overlays.is_empty() {
            return;
        }

        let ctx = &self.context;
        let scale = get_scale(ctx);
        let locked = overlay::is_locked();
        let opacity = overlay::opacity();

        for ov in overlays.iter() {
            let state = overlay::panel_state(&ov.id);
            if !state.visible {
                continue;
            }

            let title = overlay::display_title(&ov.id);
            let win_id = egui::Id::new("plugin_overlay").with(&ov.id);
            let default_pos = state.pos.map_or_else(
                || egui::pos2(ctx.input(|i| i.viewport_rect().right()) - 300.0 * scale, 8.0 * scale),
                |p| egui::pos2(p[0], p[1]),
            );

            let force_reset = overlay::take_reset(&ov.id);
            let mut window = egui::Window::new(&title)
                .id(win_id)
                .title_bar(false)
                .resizable(false)
                .movable(!locked)
                .interactable(!locked)
                .frame(panel_frame(ctx, opacity));
            window = if force_reset {
                window.current_pos(default_pos)
            } else {
                window.default_pos(default_pos)
            };
            let response = window.show(ctx, |ui| {
                ui.set_opacity(opacity);
                if state.collapsed {
                    draw_badge(ui, &ov.id, &title, scale);
                } else {
                    draw_panel(ui, ov, &title, scale);
                }
            });

            // Persist live position (flushed to disk on pointer release).
            if let Some(inner) = response {
                let pos = inner.response.rect.min;
                overlay::set_panel_pos(&ov.id, [pos.x, pos.y]);
            }
        }

        // Flush any position changes once the user releases the mouse.
        if ctx.input(|i| i.pointer.any_released()) {
            overlay::persist_if_dirty();
        }
    }
}

/// Window frame with the configured opacity applied to the background.
fn panel_frame(ctx: &egui::Context, opacity: f32) -> egui::Frame {
    let mut frame = egui::Frame::window(&ctx.style());
    frame.fill = frame.fill.linear_multiply(opacity);
    frame
}

/// Collapsed state: a compact badge that expands on click.
fn draw_badge(ui: &mut egui::Ui, id: &str, title: &str, scale: f32) {
    let text = egui::RichText::new(format!("\u{f0c9} {title}")).size(12.0 * scale);
    if ui.button(text).on_hover_text("Click to expand").clicked() {
        overlay::set_panel_collapsed(id, false);
    }
}

/// Expanded state: header row (title + collapse + close) followed by plugin content.
fn draw_panel(ui: &mut egui::Ui, ov: &overlay::PluginOverlay, title: &str, scale: f32) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(title).size(12.0 * scale).strong());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.small_button("\u{f00d}").on_hover_text("Hide").clicked() {
                overlay::set_overlay_visible(&ov.id, false);
            }
            if ui.small_button("\u{f068}").on_hover_text("Collapse").clicked() {
                overlay::set_panel_collapsed(&ov.id, true);
            }
        });
    });
    ui.separator();

    let _scope = crate::core::plugin::OwnerScope::enter(ov.owner);
    let _ = panic::catch_unwind(AssertUnwindSafe(|| {
        (ov.callback)(ui as *mut egui::Ui as *mut c_void, ov.userdata as *mut c_void);
    }))
    .inspect_err(|_| {
        error!("plugin overlay callback panicked: {}", ov.id);
    });
}
