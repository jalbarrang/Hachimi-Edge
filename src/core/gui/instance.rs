use std::sync::{atomic, Mutex};
use std::time::Instant;

use rust_i18n::t;

use crate::core::Hachimi;

use super::tween::{Easing, TweenInOutWithDelay};
use super::window::FirstTimeSetupWindow;
use super::{Gui, INSTANCE};

macro_rules! add_font {
    ($fonts:expr, $family_fonts:expr, $filename:literal) => {
        $fonts.font_data.insert(
            $filename.to_owned(),
            egui::FontData::from_static(include_bytes!(concat!("../../../assets/fonts/", $filename))).into(),
        );
        $family_fonts.push($filename.to_owned());
    };
}

impl Gui {
    pub fn apply_theme(ctx: &egui::Context, style: &mut egui::Style, config: &crate::core::hachimi::Config) {
        let mut visuals = egui::Visuals::dark();

        visuals.window_fill = config.ui_window_fill;
        visuals.panel_fill = config.ui_panel_fill;
        visuals.extreme_bg_color = config.ui_extreme_bg_color;
        visuals.window_corner_radius = config.ui_window_rounding.into();

        visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, config.ui_text_color);

        visuals.widgets.active.bg_fill = config.ui_accent_color;
        visuals.widgets.hovered.bg_fill = config.ui_accent_color.linear_multiply(0.8);
        visuals.selection.bg_fill = config.ui_accent_color.linear_multiply(0.5);

        visuals.override_text_color = Some(config.ui_text_color);

        style.visuals = visuals.clone();
        ctx.set_visuals(visuals);
    }

    pub fn instance_or_init(#[cfg_attr(target_os = "windows", allow(unused))] open_key_id: &str) -> &Mutex<Gui> {
        if let Some(instance) = INSTANCE.get() {
            return instance;
        }

        let hachimi = Hachimi::instance();
        let config = (**Hachimi::instance().config.load()).clone();

        let context = egui::Context::default();
        egui_extras::install_image_loaders(&context);

        context.set_fonts(Self::get_font_definitions());

        let mut style = egui::Style::default();
        style.spacing.button_padding = egui::Vec2::new(8.0, 5.0);
        style.interaction.selectable_labels = false;

        Self::apply_theme(&context, &mut style, &config);

        context.set_style(style.clone());

        let default_style = style.clone();

        let mut fps_value = hachimi.target_fps.load(atomic::Ordering::Relaxed);
        if fps_value == -1 {
            fps_value = 30;
        }

        let mut windows: Vec<super::window::BoxedWindow> = Vec::new();
        if !config.skip_first_time_setup {
            windows.push(Box::new(FirstTimeSetupWindow::new()));
        }

        let now = Instant::now();
        let instance = Gui {
            context,
            config,
            input: egui::RawInput::default(),
            gui_scale: 1.0,
            finalized_scale: 1.0,
            default_style,
            start_time: now,
            prev_main_axis_size: 1,
            last_fps_update: now,
            tmp_frame_count: 0,
            fps_text: "FPS: 0".to_string(),
            last_focused: None,
            #[cfg(target_os = "android")]
            ime_cooldown: None,

            show_menu: false,

            splash_visible: true,
            splash_tween: TweenInOutWithDelay::new(0.8, 3.0, Easing::OutQuad),
            splash_sub_str: {
                #[cfg(target_os = "windows")]
                {
                    let key_label =
                        crate::windows::utils::vk_to_display_label(hachimi.config.load().windows.menu_open_key);
                    t!("splash_sub", open_key_str = key_label).into_owned()
                }
                #[cfg(not(target_os = "windows"))]
                {
                    t!("splash_sub", open_key_str = t!(open_key_id)).into_owned()
                }
            },

            menu_visible: false,
            menu_anim_time: None,
            menu_fps_value: fps_value,

            #[cfg(target_os = "windows")]
            menu_vsync_value: hachimi.vsync_count.load(atomic::Ordering::Relaxed),

            update_progress_visible: false,

            notifications: Vec::new(),
            next_notification_id: 0,
            windows,
        };

        // SAFETY: FFI / raw pointer operation required by IL2CPP interop
        unsafe {
            INSTANCE.set(Mutex::new(instance)).unwrap_unchecked();

            hachimi.run_auto_update_check();

            INSTANCE.get().unwrap_unchecked()
        }
    }

    pub fn instance() -> Option<&'static Mutex<Gui>> {
        INSTANCE.get()
    }

    fn get_font_definitions() -> egui::FontDefinitions {
        let mut fonts = egui::FontDefinitions::default();
        let proportional_fonts = fonts
            .families
            .get_mut(&egui::FontFamily::Proportional)
            .expect("unexpected failure");

        add_font!(fonts, proportional_fonts, "Inter_24pt-Regular.ttf");
        add_font!(fonts, proportional_fonts, "AlibabaPuHuiTi-3-45-Light.otf");
        add_font!(fonts, proportional_fonts, "FontAwesome.otf");

        fonts
    }
}
