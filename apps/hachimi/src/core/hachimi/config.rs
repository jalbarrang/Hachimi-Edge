//! User configuration schema (`config.json`) and its defaults.

use fnv::FnvHashSet;
use serde::{Deserialize, Serialize};

use crate::{hachimi_impl, il2cpp::hook::umamusume::CySpringController::SpringUpdateMode};

use super::{default_serde_instance, Language};

#[derive(Deserialize, Serialize, Clone)]
pub struct Config {
    #[serde(default)]
    pub debug_mode: bool,
    #[serde(default)]
    pub enable_file_logging: bool,
    #[serde(default)]
    pub apply_atlas_workaround: bool,
    #[serde(default)]
    pub translator_mode: bool,
    #[serde(default)]
    pub disable_gui: bool,
    #[serde(default)]
    pub disable_gui_once: bool,
    pub localized_data_dir: Option<String>,
    pub target_fps: Option<i32>,
    #[serde(default = "Config::default_open_browser_url")]
    pub open_browser_url: String,
    #[serde(default = "Config::default_virtual_res_mult")]
    pub virtual_res_mult: f32,
    pub translation_repo_index: Option<String>,
    #[serde(default)]
    pub skip_first_time_setup: bool,
    #[serde(default)]
    pub lazy_translation_updates: bool,
    #[serde(default)]
    pub disable_auto_update_check: bool,
    #[serde(default)]
    pub disable_translations: bool,
    #[serde(default = "Config::default_gui_scale")]
    pub gui_scale: f32,
    #[serde(default = "Config::default_ui_scale")]
    pub ui_scale: f32,
    #[serde(default = "Config::default_render_scale")]
    pub render_scale: f32,
    #[serde(default)]
    pub msaa: crate::il2cpp::hook::umamusume::GraphicSettings::MsaaQuality,
    #[serde(default)]
    pub aniso_level: crate::il2cpp::hook::UnityEngine_CoreModule::Texture::AnisoLevel,
    #[serde(default)]
    pub shadow_resolution: crate::il2cpp::hook::umamusume::CameraData::ShadowResolution,
    #[serde(default)]
    pub graphics_quality: crate::il2cpp::hook::umamusume::GraphicSettings::GraphicsQuality,
    #[serde(default = "Config::default_story_choice_auto_select_delay")]
    pub story_choice_auto_select_delay: f32,
    #[serde(default = "Config::default_story_tcps_multiplier")]
    pub story_tcps_multiplier: f32,
    #[serde(default)]
    pub enable_ipc: bool,
    #[serde(default)]
    pub ipc_listen_all: bool,
    #[serde(default)]
    pub force_allow_dynamic_camera: bool,
    #[serde(default)]
    pub live_theater_allow_same_chara: bool,
    #[serde(default = "Config::default_live_vocals_swap")]
    pub live_vocals_swap: [i32; 6],
    #[serde(default)]
    pub skill_info_dialog: bool,
    #[serde(default)]
    pub homescreen_bgseason: crate::il2cpp::hook::umamusume::TimeUtil::BgSeason,
    pub sugoi_url: Option<String>,
    #[serde(default)]
    pub auto_translate_stories: bool,
    #[serde(default)]
    pub auto_translate_localize: bool,
    #[serde(default)]
    pub disable_skill_name_translation: bool,
    #[serde(default)]
    pub hide_ingame_ui_hotkey: bool,
    #[serde(default)]
    pub language: Language,
    #[serde(default = "Config::default_meta_index_url")]
    pub meta_index_url: String,
    #[serde(default)]
    pub ipv4_only: bool,
    pub physics_update_mode: Option<SpringUpdateMode>,
    #[serde(default = "Config::default_ui_animation_scale")]
    pub ui_animation_scale: f32,
    #[serde(default)]
    pub disabled_hooks: FnvHashSet<String>,

    // theme settings
    #[serde(default = "Config::default_ui_accent")]
    pub ui_accent_color: egui::Color32,
    #[serde(default = "Config::default_window_fill")]
    pub ui_window_fill: egui::Color32,
    #[serde(default = "Config::default_panel_fill")]
    pub ui_panel_fill: egui::Color32,
    #[serde(default = "Config::default_extreme_bg")]
    pub ui_extreme_bg_color: egui::Color32,
    #[serde(default = "Config::default_text_color")]
    pub ui_text_color: egui::Color32,
    #[serde(default = "Config::default_window_rounding")]
    pub ui_window_rounding: f32,

    #[cfg(target_os = "windows")]
    #[serde(flatten)]
    pub windows: hachimi_impl::Config,
}

impl Config {
    fn default_open_browser_url() -> String {
        "https://www.google.com/".to_owned()
    }
    fn default_virtual_res_mult() -> f32 {
        1.0
    }
    fn default_ui_scale() -> f32 {
        1.0
    }
    fn default_render_scale() -> f32 {
        1.0
    }
    fn default_gui_scale() -> f32 {
        1.0
    }
    fn default_story_choice_auto_select_delay() -> f32 {
        1.2
    }
    fn default_story_tcps_multiplier() -> f32 {
        3.0
    }
    fn default_meta_index_url() -> String {
        "https://gitlab.com/umatl/hachimi-meta/-/raw/main/meta.json".to_owned()
    }
    fn default_ui_animation_scale() -> f32 {
        1.0
    }
    fn default_live_vocals_swap() -> [i32; 6] {
        [0; 6]
    }
    pub fn default_ui_accent() -> egui::Color32 {
        egui::Color32::from_rgb(100, 150, 240)
    }
    pub fn default_window_fill() -> egui::Color32 {
        egui::Color32::from_rgba_premultiplied(27, 27, 27, 220)
    }
    pub fn default_panel_fill() -> egui::Color32 {
        egui::Color32::from_rgba_premultiplied(27, 27, 27, 220)
    }
    pub fn default_extreme_bg() -> egui::Color32 {
        egui::Color32::from_rgb(15, 15, 15)
    }
    pub fn default_text_color() -> egui::Color32 {
        egui::Color32::from_gray(170)
    }
    pub fn default_window_rounding() -> f32 {
        10.0
    }
}

impl Default for Config {
    fn default() -> Self {
        default_serde_instance().expect("default instance")
    }
}

#[derive(Deserialize, Default, Clone)]
pub struct OsOption<T> {
    #[cfg(target_os = "windows")]
    windows: Option<T>,
}

impl<T> OsOption<T> {
    pub fn as_ref(&self) -> Option<&T> {
        #[cfg(target_os = "windows")]
        return self.windows.as_ref();
    }
}
