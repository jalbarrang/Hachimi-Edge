mod about;
mod config_editor;
mod first_time_setup;
mod license;
mod live_vocals_swap;
mod persistent_message;
mod simple_ok;
mod simple_yes_no;
mod theme_editor;

use std::sync::Arc;

use rust_i18n::t;

use crate::core::http::AsyncRequest;

use super::scale::{get_scale, get_scale_salt};

pub(crate) use about::AboutWindow;
pub(crate) use config_editor::ConfigEditor;
pub(crate) use first_time_setup::FirstTimeSetupWindow;
pub(crate) use license::LicenseWindow;
pub(crate) use live_vocals_swap::LiveVocalsSwapWindow;
pub use persistent_message::PersistentMessageWindow;
pub use simple_ok::SimpleOkDialog;
pub use simple_yes_no::SimpleYesNoDialog;
pub(crate) use theme_editor::ThemeEditorWindow;

pub(crate) type BoxedWindow = Box<dyn Window + Send + Sync>;

pub trait Window {
    fn run(&mut self, ctx: &egui::Context) -> bool;
}

pub(crate) fn random_id() -> egui::Id {
    egui::Id::new(egui::epaint::ahash::RandomState::new().hash_one(0))
}

pub(crate) fn new_window<'a>(
    ctx: &egui::Context,
    id: egui::Id,
    title: impl Into<egui::WidgetText>,
) -> egui::Window<'a> {
    let scale = get_scale(ctx);
    let salt = get_scale_salt(ctx);

    egui::Window::new(title)
        .id(id.with(salt.to_bits()))
        .pivot(egui::Align2::CENTER_CENTER)
        .fixed_pos(ctx.viewport_rect().max / 2.0)
        .min_width(96.0 * scale)
        .max_width(320.0 * scale)
        .max_height(250.0 * scale)
        .collapsible(false)
        .resizable(false)
}

pub(crate) fn simple_window_layout(
    ui: &mut egui::Ui,
    id: egui::Id,
    add_contents: impl FnOnce(&mut egui::Ui),
    add_buttons: impl FnOnce(&mut egui::Ui),
) {
    let builder = egui::UiBuilder::new()
        .id(id)
        .layout(egui::Layout::top_down(egui::Align::Center).with_cross_justify(true));

    ui.scope_builder(builder, |ui| {
        ui.with_layout(egui::Layout::top_down(egui::Align::Min), add_contents);

        ui.separator();

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), add_buttons);
    });
}

pub(crate) fn centered_and_wrapped_text(ui: &mut egui::Ui, text: &str) {
    let rect = ui.available_rect_before_wrap();

    let text_style = egui::TextStyle::Body;
    let text_font = ui.style().text_styles.get(&text_style).cloned().unwrap_or_default();
    let text_color = ui.style().visuals.text_color();

    let mut job = egui::text::LayoutJob::simple(text.to_owned(), text_font, text_color, rect.width());
    job.halign = egui::Align::Center;

    let galley = ui.painter().layout_job(job);

    let text_rect = galley.rect;
    let text_size = text_rect.size();

    let center_pos = rect.min + (rect.size() - text_size) / 2.0;

    let paint_pos = center_pos - text_rect.min.to_vec2();
    ui.painter().galley(paint_pos, galley, text_color);
}

pub(crate) fn paginated_window_layout(
    ui: &mut egui::Ui,
    id: egui::Id,
    i: &mut usize,
    page_count: usize,
    allow_next: bool,
    add_page_content: impl FnOnce(&mut egui::Ui, usize),
) -> bool {
    let mut open = true;

    let builder = egui::UiBuilder::new()
        .id(id)
        .layout(egui::Layout::top_down(egui::Align::Center).with_cross_justify(true));

    ui.scope_builder(builder, |ui| {
        ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
            add_page_content(ui, *i);
        });

        ui.separator();

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
            if *i < page_count - 1 {
                if allow_next && ui.button(t!("next")).clicked() {
                    *i += 1;
                }
            } else if ui.button(t!("done")).clicked() {
                open = false;
            }
            if *i > 0 && ui.button(t!("previous")).clicked() {
                *i -= 1;
            }
        });
    });

    open
}

pub(crate) fn async_request_ui_content<T: Send + Sync + 'static>(
    ui: &mut egui::Ui,
    request: Arc<AsyncRequest<T>>,
    add_contents: impl FnOnce(&mut egui::Ui, &T),
) {
    let Some(result) = &**request.result.load() else {
        if !request.running() {
            request.call();
        }
        ui.centered_and_justified(|ui| {
            ui.label(t!("loading_label"));
        });
        return;
    };

    match result {
        Ok(v) => add_contents(ui, v),
        Err(e) => {
            let rect = ui.available_rect_before_wrap();

            let text_style = egui::TextStyle::Body;
            let text_font = ui.style().text_styles.get(&text_style).cloned().unwrap_or_default();
            let text_color = ui.visuals().text_color();

            let mut text_job = egui::text::LayoutJob::simple(e.to_string(), text_font, text_color, rect.width());
            text_job.halign = egui::Align::Center;
            let text_galley = ui.painter().layout_job(text_job.clone());
            let text_height = text_galley.size().y;

            let btn_text = t!("retry");
            let btn_style = egui::TextStyle::Button;
            let btn_font = ui.style().text_styles.get(&btn_style).cloned().unwrap_or_default();
            let btn_job = egui::text::LayoutJob::simple(btn_text.to_string(), btn_font, text_color, f32::INFINITY);
            let btn_galley = ui.painter().layout_job(btn_job);
            let btn_padding = ui.style().spacing.button_padding;
            let btn_height = btn_galley.size().y + btn_padding.y * 2.0;

            let spacing = ui.spacing().item_spacing.y;
            let total_height = text_height + spacing + btn_height;

            let center_y = rect.center().y;
            let top_y = (center_y - total_height / 2.0).max(rect.top());

            let content_rect =
                egui::Rect::from_min_size(egui::pos2(rect.left(), top_y), egui::vec2(rect.width(), total_height));

            let builder = egui::UiBuilder::new().max_rect(content_rect);
            ui.scope_builder(builder, |ui| {
                ui.vertical_centered(|ui| {
                    ui.label(text_job);
                    if ui.button(btn_text).clicked() {
                        request.call();
                    }
                });
            });
        }
    }
}

pub(crate) fn save_and_reload_config(config: crate::core::hachimi::Config) {
    use std::thread;

    use rust_i18n::t;

    use crate::core::Hachimi;

    use super::Gui;

    let notif = match Hachimi::instance().save_and_reload_config(config) {
        Ok(_) => t!("notification.config_saved").into_owned(),
        Err(e) => e.to_string(),
    };

    thread::spawn(move || {
        Gui::instance()
            .expect("unexpected failure")
            .lock()
            .expect("lock poisoned")
            .show_notification(&notif);
    });
}
