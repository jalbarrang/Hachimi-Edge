//! Shared dark-theme tokens for the host egui UI.
//!
//! egui has no CSS cascade, so components should read semantic tokens from here
//! instead of hard-coding colors. Tokens are intentionally derived from the
//! persisted config colors so existing config files and the theme editor keep
//! working.

use crate::core::hachimi;

#[allow(dead_code)]
#[derive(Clone, Copy)]
pub(crate) struct ThemeTokens {
    pub window: egui::Color32,
    pub panel: egui::Color32,
    pub surface: egui::Color32,
    pub surface_hi: egui::Color32,
    pub line: egui::Color32,
    pub text: egui::Color32,
    pub text_dim: egui::Color32,
    pub text_faint: egui::Color32,
    pub accent: egui::Color32,
    pub accent_2: egui::Color32,
    pub accent_ink: egui::Color32,
    pub warn: egui::Color32,
    pub crit: egui::Color32,
    pub kakari: egui::Color32,
    pub velocity: egui::Color32,
    pub star: egui::Color32,
    pub card_radius: egui::CornerRadius,
    pub small_radius: egui::CornerRadius,
    pub pill_radius: egui::CornerRadius,
}

impl ThemeTokens {
    pub(crate) fn from_config(config: &hachimi::Config) -> Self {
        let accent = config.ui_accent_color;
        Self {
            window: config.ui_window_fill,
            panel: config.ui_panel_fill,
            surface: egui::Color32::from_rgba_premultiplied(42, 47, 51, config.ui_panel_fill.a()),
            surface_hi: egui::Color32::from_rgba_premultiplied(50, 56, 64, config.ui_panel_fill.a()),
            line: egui::Color32::from_rgba_premultiplied(58, 64, 70, 210),
            text: config.ui_text_color,
            text_dim: egui::Color32::from_rgb(154, 163, 171),
            text_faint: egui::Color32::from_rgb(107, 116, 124),
            accent,
            accent_2: accent.linear_multiply(0.72),
            accent_ink: egui::Color32::from_rgb(18, 31, 10),
            warn: egui::Color32::from_rgb(217, 167, 43),
            crit: egui::Color32::from_rgb(214, 81, 81),
            kakari: egui::Color32::from_rgb(255, 140, 46),
            velocity: egui::Color32::from_rgb(70, 194, 232),
            star: egui::Color32::from_rgb(255, 203, 61),
            card_radius: config.ui_window_rounding.into(),
            small_radius: 8.0.into(),
            pill_radius: 255.0.into(),
        }
    }

    pub(crate) fn from_ui(ui: &egui::Ui) -> Self {
        Self::from_style(ui.style())
    }

    pub(crate) fn from_style(style: &egui::Style) -> Self {
        let visuals = &style.visuals;
        let accent = visuals.widgets.active.bg_fill;
        Self {
            window: visuals.window_fill,
            panel: visuals.panel_fill,
            surface: visuals.widgets.inactive.weak_bg_fill,
            surface_hi: visuals.widgets.hovered.weak_bg_fill,
            line: visuals.window_stroke.color,
            text: visuals
                .override_text_color
                .unwrap_or(visuals.widgets.noninteractive.fg_stroke.color),
            text_dim: visuals.widgets.inactive.fg_stroke.color,
            text_faint: visuals.widgets.noninteractive.fg_stroke.color.linear_multiply(0.65),
            accent,
            accent_2: accent.linear_multiply(0.72),
            accent_ink: egui::Color32::from_rgb(18, 31, 10),
            warn: egui::Color32::from_rgb(217, 167, 43),
            crit: egui::Color32::from_rgb(214, 81, 81),
            kakari: egui::Color32::from_rgb(255, 140, 46),
            velocity: egui::Color32::from_rgb(70, 194, 232),
            star: egui::Color32::from_rgb(255, 203, 61),
            card_radius: visuals.window_corner_radius,
            small_radius: 8.0.into(),
            pill_radius: 255.0.into(),
        }
    }
}

pub(crate) fn apply_style(style: &mut egui::Style, config: &hachimi::Config) {
    let tokens = ThemeTokens::from_config(config);

    style.spacing.button_padding = egui::vec2(10.0, 6.0);
    style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    style.spacing.window_margin = egui::Margin::symmetric(14, 12);
    style.spacing.menu_margin = egui::Margin::symmetric(8, 8);
    style.interaction.selectable_labels = false;

    style.visuals.window_fill = tokens.window;
    style.visuals.panel_fill = tokens.panel;
    style.visuals.extreme_bg_color = config.ui_extreme_bg_color;
    style.visuals.window_corner_radius = tokens.card_radius;
    style.visuals.window_stroke = egui::Stroke::new(1.0, tokens.line);

    style.visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, tokens.text);

    style.visuals.widgets.inactive.weak_bg_fill = tokens.surface;
    style.visuals.widgets.inactive.bg_fill = tokens.surface;
    style.visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, tokens.line);
    style.visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, tokens.text_dim);
    style.visuals.widgets.inactive.corner_radius = tokens.small_radius;

    style.visuals.widgets.hovered.weak_bg_fill = tokens.surface_hi;
    style.visuals.widgets.hovered.bg_fill = tokens.surface_hi;
    style.visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, tokens.accent.linear_multiply(0.75));
    style.visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, tokens.text);
    style.visuals.widgets.hovered.corner_radius = tokens.small_radius;

    style.visuals.widgets.active.weak_bg_fill = tokens.accent_2;
    style.visuals.widgets.active.bg_fill = tokens.accent;
    style.visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0, tokens.accent);
    style.visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, tokens.accent_ink);
    style.visuals.widgets.active.corner_radius = tokens.small_radius;

    style.visuals.widgets.open = style.visuals.widgets.hovered;
    style.visuals.selection.bg_fill = tokens.accent.linear_multiply(0.42);
    style.visuals.selection.stroke = egui::Stroke::new(1.0, tokens.accent);
    style.visuals.override_text_color = Some(tokens.text);
    style.visuals.hyperlink_color = tokens.velocity;
}
