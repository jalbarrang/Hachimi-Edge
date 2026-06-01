use std::sync::atomic::{self, Ordering};

use egui_scale::EguiScale;
use rust_i18n::t;

use crate::core::plugin::overlay;
use crate::core::Hachimi;

use super::theme_preview::take_pending_theme;
use super::window::BoxedWindow;
use super::{Gui, IS_CONSUMING_INPUT, PIXELS_PER_POINT_RATIO};

impl Gui {
    pub fn set_screen_size(&mut self, width: i32, height: i32) {
        let is_landscape = width > height;
        let main_axis_size = if is_landscape { height } else { width.min(height) };

        let orientation_scale = {
            let orientation_ratio = if is_landscape {
                height as f32 / width as f32
            } else {
                1.0
            };
            if is_landscape {
                orientation_ratio * Hachimi::instance().config.load().windows.gui_landscape_ratio
            } else {
                1.0
            }
        };

        let pixels_per_point = main_axis_size as f32 * PIXELS_PER_POINT_RATIO * orientation_scale;
        self.context.set_pixels_per_point(pixels_per_point);

        self.input.screen_rect = Some(egui::Rect {
            min: egui::Pos2::default(),
            max: egui::Pos2::new(
                width as f32 / self.context.pixels_per_point(),
                height as f32 / self.context.pixels_per_point(),
            ),
        });

        self.prev_main_axis_size = main_axis_size;
    }

    fn take_input(&mut self) -> egui::RawInput {
        self.input.time = Some(self.start_time.elapsed().as_secs_f64());
        self.input.take()
    }

    fn update_fps(&mut self) {
        let delta = self.last_fps_update.elapsed().as_secs_f64();
        if delta > 0.5 {
            let fps = (self.tmp_frame_count as f64 * (0.5 / delta) * 2.0).round();
            self.fps_text = t!("menu.fps_text", fps = fps).into_owned();
            self.tmp_frame_count = 1;
            self.last_fps_update = std::time::Instant::now();
        } else {
            self.tmp_frame_count += 1;
        }
    }

    pub fn run(&mut self) -> egui::FullOutput {
        if let Some(config) = take_pending_theme() {
            self.config = config.clone();
            Self::apply_theme(&self.context, &mut self.default_style, &config);

            let mut style = self.default_style.clone();
            style.scale(self.gui_scale);
            self.context.set_style(style)
        }

        self.update_fps();
        let input = self.take_input();

        let live_scale = Hachimi::instance().config.load().gui_scale;
        if self.gui_scale != live_scale {
            self.gui_scale = live_scale;
            if !self.context.is_using_pointer() {
                self.finalized_scale = live_scale;
            }

            let mut style = self.default_style.clone();
            if live_scale != 1.0 {
                style.scale(live_scale);
            }
            self.context.set_style(style);
        }

        self.context.data_mut(|d| {
            d.insert_temp(egui::Id::new("gui_scale"), live_scale);
            d.insert_temp(egui::Id::new("gui_scale_salt"), self.finalized_scale);
        });

        let mut style = self.default_style.clone();
        if live_scale != 1.0 {
            style.scale(live_scale);
        }
        self.context.set_style(style);

        self.context.begin_pass(input);

        if self.menu_visible {
            self.run_menu();
        }
        if self.update_progress_visible {
            self.run_update_progress();
        }

        self.run_windows();
        self.run_notifications();
        self.run_overlays();

        if self.splash_visible {
            self.run_splash();
        }
        if crate::core::hachimi::CONFIG_LOAD_ERROR.swap(false, Ordering::AcqRel) {
            self.show_notification(&t!("notification.config_error"));
        }

        #[cfg(target_os = "windows")]
        {
            use crate::il2cpp::{hook::UnityEngine_InputLegacyModule::Input::set_imeCompositionMode, symbols::Thread};

            let focused = self.context.memory(egui::Memory::focused);
            let wants_kb = self.context.wants_keyboard_input();

            if focused != self.last_focused {
                if wants_kb {
                    Thread::main_thread().schedule(|| {
                        set_imeCompositionMode(1);
                    });
                } else if self.last_focused.is_some() {
                    Thread::main_thread().schedule(|| {
                        set_imeCompositionMode(0);
                    });
                }
            }
            self.last_focused = focused;
        }

        self.set_consuming_input(self.is_consuming_input());

        // L2 gate: while the L1 modal is closed, the cursor is "over a panel" when it
        // hovers an egui area and overlays aren't globally locked. Used by the wnd hook
        // to swallow mouse input for panels but let clicks fall through to the game.
        let l2_wants = !self.menu_visible && !overlay::is_locked() && self.context.is_pointer_over_area();
        super::L2_WANTS_POINTER.store(l2_wants, Ordering::Relaxed);

        self.context.end_pass()
    }

    pub(crate) fn run_windows(&mut self) {
        self.windows.retain_mut(|w| w.run(&self.context));
    }

    pub fn is_empty(&self) -> bool {
        !self.splash_visible
            && !self.menu_visible
            && !self.update_progress_visible
            && self.notifications.is_empty()
            && self.windows.is_empty()
            && !overlay::has_plugin_overlays()
    }

    pub fn is_consuming_input(&self) -> bool {
        self.menu_visible || !self.windows.is_empty()
    }

    pub fn is_consuming_input_atomic() -> bool {
        IS_CONSUMING_INPUT.load(atomic::Ordering::Relaxed)
    }

    /// Whether the cursor is over an interactable L2 overlay panel (L1 closed).
    pub fn l2_wants_pointer_atomic() -> bool {
        super::L2_WANTS_POINTER.load(atomic::Ordering::Relaxed)
    }

    pub fn set_consuming_input(&mut self, val: bool) {
        if !self.windows.is_empty() && !val {
            self.windows.clear();
        }

        self.menu_visible = val;
        if !val {
            self.show_menu = false;
        }
        IS_CONSUMING_INPUT.store(val, atomic::Ordering::Relaxed);
    }

    pub fn toggle_menu(&mut self) {
        self.show_menu = !self.show_menu;
        if self.show_menu {
            self.menu_visible = true;
        } else {
            self.menu_anim_time = None;
        }
    }

    pub fn show_window(&mut self, window: BoxedWindow) {
        self.windows.push(window);
    }
}
