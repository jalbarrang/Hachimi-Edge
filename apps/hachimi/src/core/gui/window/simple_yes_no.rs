use rust_i18n::t;

use super::{centered_and_wrapped_text, new_window, random_id, Window};

pub struct SimpleYesNoDialog {
    title: String,
    content: String,
    callback: fn(bool),
    id: egui::Id,
}

impl SimpleYesNoDialog {
    pub fn new(title: &str, content: &str, callback: fn(bool)) -> SimpleYesNoDialog {
        SimpleYesNoDialog {
            title: title.to_owned(),
            content: content.to_owned(),
            callback,
            id: random_id(),
        }
    }
}

impl Window for SimpleYesNoDialog {
    fn run(&mut self, ctx: &egui::Context) -> bool {
        let mut open = true;
        let mut open2 = true;
        let mut result = false;

        new_window(ctx, self.id, &self.title).open(&mut open).show(ctx, |ui| {
            egui::TopBottomPanel::bottom(self.id.with("bottom_panel")).show_inside(ui, |ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                    if ui.button(t!("no")).clicked() {
                        open2 = false;
                    }
                    if ui.button(t!("yes")).clicked() {
                        result = true;
                        open2 = false;
                    }
                })
            });

            egui::CentralPanel::default()
                .frame(egui::Frame::NONE)
                .show_inside(ui, |ui| {
                    centered_and_wrapped_text(ui, &self.content);
                });
        });

        if open && open2 {
            true
        } else {
            (self.callback)(result);
            false
        }
    }
}
