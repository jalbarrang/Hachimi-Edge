use crate::core::{hachimi, Hachimi};

use super::scale::get_scale;
use super::tween::{Easing, TweenInOutWithDelay};
use super::window::random_id;
use super::Gui;

pub struct NotificationGuard(pub u32);

impl Drop for NotificationGuard {
    fn drop(&mut self) {
        if let Some(mutex) = Gui::instance() {
            if let Ok(mut gui) = mutex.lock() {
                gui.close_notification(self.0);
            }
        }
    }
}

pub(crate) struct Notification {
    id: u32,
    content: String,
    config: hachimi::Config,
    tween: TweenInOutWithDelay,
    egui_id: egui::Id,
}

impl Notification {
    pub(crate) fn new(id: u32, content: String, persistent: bool) -> Notification {
        Notification {
            id,
            content,
            config: (**Hachimi::instance().config.load()).clone(),
            tween: TweenInOutWithDelay::new(0.2, if persistent { f32::MAX } else { 3.0 }, Easing::OutQuad),
            egui_id: random_id(),
        }
    }

    const WIDTH: f32 = 150.0;

    pub(crate) fn run(&mut self, ctx: &egui::Context, offset: &mut f32) -> bool {
        let scale = get_scale(ctx);

        let Some(tween_val) = self.tween.run(ctx, self.egui_id.with("tween")) else {
            return false;
        };

        let frame_rect = egui::Area::new(self.egui_id)
            .anchor(
                egui::Align2::RIGHT_BOTTOM,
                egui::Vec2::new((Self::WIDTH * scale) * (1.0 - tween_val), *offset),
            )
            .show(ctx, |ui| {
                egui::Frame::NONE
                    .fill(self.config.ui_panel_fill)
                    .inner_margin(egui::Margin::symmetric(10, 8))
                    .show(ui, |ui| {
                        ui.set_width(Self::WIDTH * scale);
                        ui.label(&self.content);
                    })
                    .response
                    .rect
            })
            .inner;

        *offset -= (2.0 * scale) + frame_rect.height() * tween_val;
        true
    }
}

impl Gui {
    pub(crate) fn run_notifications(&mut self) {
        let mut offset: f32 = -16.0;
        self.notifications.retain_mut(|n| n.run(&self.context, &mut offset));
    }

    pub fn show_notification(&mut self, content: &str) {
        self.add_notification(content, false);
    }

    pub fn show_persistent_notification(&mut self, content: &str) -> u32 {
        self.add_notification(content, true)
    }

    fn add_notification(&mut self, content: &str, persistent: bool) -> u32 {
        let id = self.next_notification_id;
        self.notifications
            .push(Notification::new(id, content.to_owned(), persistent));
        self.next_notification_id = self.next_notification_id.wrapping_add(1);
        id
    }

    pub fn close_notification(&mut self, id: u32) {
        self.notifications.retain(|n| n.id != id);
    }
}
