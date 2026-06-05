//! Shared egui widget kit for the host UI.
//!
//! The kit is a dark reinterpretation of the Honse game's UI language: pill
//! controls, bright accent section banners, compact cards, and cockpit-friendly
//! mono numbers for telemetry-like values.

use super::theme::ThemeTokens;

#[derive(Clone, Copy)]
pub(crate) enum PillButtonKind {
    Primary,
    Secondary,
    Ghost,
    Danger,
}

pub(crate) fn card_frame(ui: &egui::Ui) -> egui::Frame {
    let tokens = ThemeTokens::from_ui(ui);
    egui::Frame::new()
        .fill(tokens.surface)
        .stroke(egui::Stroke::new(1.0, tokens.line))
        .corner_radius(tokens.card_radius)
        .inner_margin(egui::Margin::symmetric(12, 10))
}

pub(crate) fn pill_button(ui: &mut egui::Ui, text: impl Into<String>, kind: PillButtonKind) -> egui::Response {
    let tokens = ThemeTokens::from_ui(ui);
    let (fill, stroke, text_color) = match kind {
        PillButtonKind::Primary => (
            tokens.accent,
            egui::Stroke::new(1.0, tokens.accent_2),
            tokens.accent_ink,
        ),
        PillButtonKind::Secondary => (tokens.surface, egui::Stroke::new(1.0, tokens.line), tokens.text),
        PillButtonKind::Ghost => (
            egui::Color32::TRANSPARENT,
            egui::Stroke::new(1.0, tokens.line),
            tokens.text_dim,
        ),
        PillButtonKind::Danger => (
            tokens.crit,
            egui::Stroke::new(1.0, tokens.crit.linear_multiply(0.8)),
            egui::Color32::WHITE,
        ),
    };

    ui.add(
        egui::Button::new(egui::RichText::new(text.into()).color(text_color).strong())
            .fill(fill)
            .stroke(stroke)
            .corner_radius(tokens.pill_radius)
            .min_size(egui::vec2(0.0, 26.0)),
    )
}

pub(crate) fn primary_button(ui: &mut egui::Ui, text: impl Into<String>) -> egui::Response {
    pill_button(ui, text, PillButtonKind::Primary)
}

pub(crate) fn secondary_button(ui: &mut egui::Ui, text: impl Into<String>) -> egui::Response {
    pill_button(ui, text, PillButtonKind::Secondary)
}

pub(crate) fn ghost_button(ui: &mut egui::Ui, text: impl Into<String>) -> egui::Response {
    pill_button(ui, text, PillButtonKind::Ghost)
}

pub(crate) fn danger_button(ui: &mut egui::Ui, text: impl Into<String>) -> egui::Response {
    pill_button(ui, text, PillButtonKind::Danger)
}

/// Accent section banner. Used in place of the old heading + separator.
pub(crate) fn section_banner(ui: &mut egui::Ui, text: impl Into<String>) -> egui::Response {
    let tokens = ThemeTokens::from_ui(ui);
    let desired = egui::vec2(ui.available_width(), 28.0);
    let (rect, response) = ui.allocate_exact_size(desired, egui::Sense::hover());

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        painter.rect_filled(rect, tokens.pill_radius, tokens.accent_2);
        painter.rect_filled(
            rect.shrink2(egui::vec2(0.0, rect.height() * 0.42)),
            tokens.pill_radius,
            tokens.accent,
        );

        let galley = ui.painter().layout_no_wrap(
            text.into(),
            egui::TextStyle::Button.resolve(ui.style()),
            tokens.accent_ink,
        );
        let text_pos = rect.center() - galley.size() / 2.0;
        painter.galley(text_pos, galley, tokens.accent_ink);
    }

    response
}

/// Consistent section header used across the Control Center tabs.
pub(crate) fn section_header(ui: &mut egui::Ui, text: impl Into<String>) {
    ui.add_space(6.0);
    section_banner(ui, text);
    ui.add_space(6.0);
}

pub(crate) fn category_tag(ui: &mut egui::Ui, text: impl Into<String>) -> egui::Response {
    let tokens = ThemeTokens::from_ui(ui);
    let text = text.into();
    let galley = ui
        .painter()
        .layout_no_wrap(text, egui::TextStyle::Small.resolve(ui.style()), tokens.text);
    let desired = galley.size() + egui::vec2(18.0, 8.0);
    let (rect, response) = ui.allocate_exact_size(desired, egui::Sense::hover());
    if ui.is_rect_visible(rect) {
        ui.painter()
            .rect_filled(rect, tokens.small_radius, tokens.surface.linear_multiply(0.95));
        ui.painter().rect_stroke(
            rect,
            tokens.small_radius,
            egui::Stroke::new(1.0, tokens.line),
            egui::epaint::StrokeKind::Inside,
        );
        ui.painter().line_segment(
            [
                rect.left_top() + egui::vec2(3.0, 4.0),
                rect.left_bottom() + egui::vec2(3.0, -4.0),
            ],
            egui::Stroke::new(2.0, tokens.accent),
        );
        ui.painter().galley(
            rect.center() - galley.size() / 2.0 + egui::vec2(2.0, 0.0),
            galley,
            tokens.text,
        );
    }
    response
}

#[allow(dead_code)]
pub(crate) fn stat_chip(ui: &mut egui::Ui, label: &str, value: impl ToString, delta: Option<&str>) -> egui::Response {
    let tokens = ThemeTokens::from_ui(ui);
    let desired = egui::vec2(72.0, 48.0);
    let (rect, response) = ui.allocate_exact_size(desired, egui::Sense::hover());
    if ui.is_rect_visible(rect) {
        ui.painter().rect_filled(rect, tokens.small_radius, tokens.surface);
        ui.painter().rect_stroke(
            rect,
            tokens.small_radius,
            egui::Stroke::new(1.0, tokens.line),
            egui::epaint::StrokeKind::Inside,
        );
        let label_galley = ui.painter().layout_no_wrap(
            label.to_owned(),
            egui::TextStyle::Small.resolve(ui.style()),
            tokens.text_dim,
        );
        let value_galley = ui.painter().layout_no_wrap(
            value.to_string(),
            egui::TextStyle::Button.resolve(ui.style()),
            tokens.text,
        );
        ui.painter().galley(
            egui::pos2(rect.center().x - label_galley.size().x / 2.0, rect.top() + 6.0),
            label_galley,
            tokens.text_dim,
        );
        ui.painter().galley(
            egui::pos2(rect.center().x - value_galley.size().x / 2.0, rect.top() + 20.0),
            value_galley,
            tokens.text,
        );
        if let Some(delta) = delta {
            let delta_galley = ui.painter().layout_no_wrap(
                delta.to_owned(),
                egui::TextStyle::Small.resolve(ui.style()),
                tokens.accent,
            );
            ui.painter().galley(
                egui::pos2(rect.center().x - delta_galley.size().x / 2.0, rect.bottom() - 14.0),
                delta_galley,
                tokens.accent,
            );
        }
    }
    response
}

#[allow(dead_code)]
pub(crate) fn icon_tile(
    ui: &mut egui::Ui,
    icon: impl Into<String>,
    label: impl Into<String>,
    badge: Option<&str>,
) -> egui::Response {
    let tokens = ThemeTokens::from_ui(ui);
    let desired = egui::vec2(160.0, 54.0);
    let (rect, response) = ui.allocate_exact_size(desired, egui::Sense::click());
    let fill = if response.hovered() {
        tokens.surface_hi
    } else {
        tokens.surface
    };

    if ui.is_rect_visible(rect) {
        ui.painter().rect_filled(rect, tokens.card_radius, fill);
        ui.painter().rect_stroke(
            rect,
            tokens.card_radius,
            egui::Stroke::new(1.0, if response.hovered() { tokens.accent } else { tokens.line }),
            egui::epaint::StrokeKind::Inside,
        );

        let icon_rect = egui::Rect::from_min_size(rect.min + egui::vec2(10.0, 8.0), egui::vec2(38.0, 38.0));
        ui.painter()
            .rect_filled(icon_rect, tokens.small_radius, tokens.surface_hi);
        ui.painter().rect_stroke(
            icon_rect,
            tokens.small_radius,
            egui::Stroke::new(1.0, tokens.line),
            egui::epaint::StrokeKind::Inside,
        );
        let icon_galley =
            ui.painter()
                .layout_no_wrap(icon.into(), egui::TextStyle::Button.resolve(ui.style()), tokens.text);
        ui.painter()
            .galley(icon_rect.center() - icon_galley.size() / 2.0, icon_galley, tokens.text);

        let label_galley = egui::WidgetText::from(label.into()).into_galley(
            ui,
            Some(egui::TextWrapMode::Wrap),
            rect.width() - 58.0,
            egui::TextStyle::Button,
        );
        ui.painter().galley(
            egui::pos2(rect.left() + 58.0, rect.center().y - label_galley.size().y / 2.0),
            label_galley,
            tokens.text,
        );

        if let Some(badge) = badge {
            let badge_rect = egui::Rect::from_min_size(rect.min + egui::vec2(6.0, -5.0), egui::vec2(34.0, 14.0));
            ui.painter().rect_filled(badge_rect, 4.0, tokens.crit);
            ui.painter().text(
                badge_rect.center(),
                egui::Align2::CENTER_CENTER,
                badge,
                egui::TextStyle::Small.resolve(ui.style()),
                egui::Color32::WHITE,
            );
        }
    }

    response
}

/// Canonical egui toggle-switch widget (adapted from the egui demo).
/// Flips `*on` when clicked and animates the knob between states.
pub(crate) fn toggle_ui(ui: &mut egui::Ui, on: &mut bool) -> egui::Response {
    let tokens = ThemeTokens::from_ui(ui);
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
        let fill = if *on {
            tokens.accent
        } else if response.hovered() {
            tokens.surface_hi
        } else {
            tokens.surface
        };
        ui.painter().rect(
            rect,
            radius,
            fill,
            egui::Stroke::new(1.0, if *on { tokens.accent_2 } else { tokens.line }),
            egui::epaint::StrokeKind::Inside,
        );
        let circle_x = egui::lerp((rect.left() + radius)..=(rect.right() - radius), how_on);
        let center = egui::pos2(circle_x, rect.center().y);
        ui.painter().circle_filled(center, 0.72 * radius, tokens.text);
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

pub(crate) fn empty_state(ui: &mut egui::Ui, text: impl Into<String>) {
    let tokens = ThemeTokens::from_ui(ui);
    card_frame(ui).show(ui, |ui| {
        ui.vertical_centered(|ui| {
            ui.label(egui::RichText::new(text.into()).color(tokens.text_dim));
        });
    });
}
