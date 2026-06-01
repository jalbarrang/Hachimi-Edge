//! Shared egui widget kit for the host UI.
//!
//! egui ships only `Checkbox`/`RadioButton`, so the on/off **toggle switch**
//! used throughout the Control Center is the one custom widget we maintain here.
//! `section_header` keeps section titles visually consistent across L1 tabs.

/// Canonical egui toggle-switch widget (adapted from the egui demo).
/// Flips `*on` when clicked and animates the knob between states.
pub(crate) fn toggle_ui(ui: &mut egui::Ui, on: &mut bool) -> egui::Response {
    let desired_size = ui.spacing().interact_size.y * egui::vec2(2.0, 1.0);
    let (rect, mut response) = ui.allocate_exact_size(desired_size, egui::Sense::click());

    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    response.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Checkbox, ui.is_enabled(), *on, ""));

    if ui.is_rect_visible(rect) {
        let how_on = ui.ctx().animate_bool(response.id, *on);
        let visuals = ui.style().interact_selectable(&response, *on);
        let rect = rect.expand(visuals.expansion);
        let radius = 0.5 * rect.height();
        ui.painter().rect(
            rect,
            radius,
            visuals.bg_fill,
            visuals.bg_stroke,
            egui::epaint::StrokeKind::Inside,
        );
        let circle_x = egui::lerp((rect.left() + radius)..=(rect.right() - radius), how_on);
        let center = egui::pos2(circle_x, rect.center().y);
        ui.painter()
            .circle(center, 0.75 * radius, visuals.bg_fill, visuals.fg_stroke);
    }

    response
}

/// A labelled toggle row: `[label .................... (switch)]`.
/// Returns the switch [`egui::Response`] so callers can react to `.changed()`.
pub(crate) fn toggle_row(ui: &mut egui::Ui, label: impl Into<egui::WidgetText>, on: &mut bool) -> egui::Response {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| toggle_ui(ui, on))
            .inner
    })
    .inner
}

/// Consistent section header used across the Control Center tabs.
pub(crate) fn section_header(ui: &mut egui::Ui, text: impl Into<egui::RichText>) {
    ui.add_space(4.0);
    ui.heading(text.into());
    ui.separator();
}
