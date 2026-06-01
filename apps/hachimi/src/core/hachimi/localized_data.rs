//! Localized data loading: translation dictionaries, plural rules, and the
//! `localized_data/config.json` schema.

use std::{
    fs,
    path::{Path, PathBuf},
};

use fnv::FnvHashMap;
use serde::{de::DeserializeOwned, Deserialize};
use textwrap::wrap_algorithms::Penalties;

use crate::core::{plurals, Error};

use super::{default_serde_instance, AssetInfo, AssetMetadata, Config, OsOption};

#[derive(Default)]
pub struct LocalizedData {
    pub config: LocalizedDataConfig,
    path: Option<PathBuf>,

    pub localize_dict: FnvHashMap<String, String>,
    pub hashed_dict: FnvHashMap<u64, String>,
    pub text_data_dict: FnvHashMap<i32, FnvHashMap<i32, String>>, // {"category": {"index": "text"}}
    pub character_system_text_dict: FnvHashMap<i32, FnvHashMap<i32, String>>, // {"character_id": {"voice_id": "text"}}
    pub race_jikkyo_comment_dict: FnvHashMap<i32, String>,        // {"id": "text"}
    pub race_jikkyo_message_dict: FnvHashMap<i32, String>,        // {"id": "text"}
    assets_path: Option<PathBuf>,

    pub plural_form: plurals::Resolver,
    pub ordinal_form: plurals::Resolver,

    pub wrapper_penalties: Penalties,
}

impl LocalizedData {
    pub(super) fn new(config: &Config, data_dir: &Path) -> Result<LocalizedData, Error> {
        if config.disable_translations {
            return Ok(LocalizedData::default());
        }

        let path: Option<PathBuf>;
        let config: LocalizedDataConfig = if let Some(ld_dir) = &config.localized_data_dir {
            let ld_path = Path::new(data_dir).join(ld_dir);

            let ld_config_path = ld_path.join("config.json");
            path = Some(ld_path);

            if fs::metadata(&ld_config_path).is_ok() {
                let json = fs::read_to_string(&ld_config_path)?;
                serde_json::from_str(&json)?
            } else {
                warn!("Localized data config not found");
                LocalizedDataConfig::default()
            }
        } else {
            path = None;
            LocalizedDataConfig::default()
        };

        let plural_form = Self::parse_plural_form_or_default(&config.plural_form)?;
        let ordinal_form = Self::parse_plural_form_or_default(&config.ordinal_form)?;

        let wrapper_penalties = Self::parse_wrap_penalties_or_default(&config.wrapper_penalties);

        Ok(LocalizedData {
            localize_dict: Self::load_dict_static(&path, config.localize_dict.as_ref()).unwrap_or_default(),
            hashed_dict: Self::load_dict_static(&path, config.hashed_dict.as_ref()).unwrap_or_default(),
            text_data_dict: Self::load_dict_static(&path, config.text_data_dict.as_ref()).unwrap_or_default(),
            character_system_text_dict: Self::load_dict_static(&path, config.character_system_text_dict.as_ref())
                .unwrap_or_default(),
            race_jikkyo_comment_dict: Self::load_dict_static(&path, config.race_jikkyo_comment_dict.as_ref())
                .unwrap_or_default(),
            race_jikkyo_message_dict: Self::load_dict_static(&path, config.race_jikkyo_message_dict.as_ref())
                .unwrap_or_default(),
            assets_path: path
                .as_ref()
                .map(|p| config.assets_dir.as_ref().map(|dir| p.join(dir)))
                .unwrap_or_default(),

            plural_form,
            ordinal_form,

            wrapper_penalties,

            config,
            path,
        })
    }

    fn load_dict_static_ex<T: DeserializeOwned, P: AsRef<Path>>(
        ld_path_opt: &Option<PathBuf>,
        rel_path_opt: Option<P>,
        silent_fs_error: bool,
    ) -> Option<T> {
        let ld_path = ld_path_opt.as_ref()?;
        let rel_path = rel_path_opt?;

        let path = ld_path.join(rel_path);
        let json = match fs::read_to_string(&path) {
            Ok(v) => v,
            Err(e) => {
                if !silent_fs_error {
                    error!("Failed to read '{}': {}", path.display(), e);
                }
                return None;
            }
        };

        let dict = match serde_json::from_str::<T>(&json) {
            Ok(v) => v,
            Err(e) => {
                error!("Failed to parse '{}': {}", path.display(), e);
                return None;
            }
        };

        Some(dict)
    }

    fn load_dict_static<T: DeserializeOwned, P: AsRef<Path>>(
        ld_path_opt: &Option<PathBuf>,
        rel_path_opt: Option<P>,
    ) -> Option<T> {
        Self::load_dict_static_ex(ld_path_opt, rel_path_opt, false)
    }

    pub fn load_dict<T: DeserializeOwned, P: AsRef<Path>>(&self, rel_path_opt: Option<P>) -> Option<T> {
        Self::load_dict_static(&self.path, rel_path_opt)
    }

    pub fn load_assets_dict<T: DeserializeOwned, P: AsRef<Path>>(&self, rel_path_opt: Option<P>) -> Option<T> {
        Self::load_dict_static_ex(&self.assets_path, rel_path_opt, true)
    }

    fn parse_plural_form_or_default(opt: &Option<String>) -> Result<plurals::Resolver, Error> {
        if let Some(plural_form) = opt {
            Ok(plurals::Resolver::Expr(plurals::Ast::parse(plural_form)?))
        } else {
            Ok(plurals::Resolver::Function(|_| 0))
        }
    }

    fn parse_wrap_penalties_or_default(opt: &Option<PenaltiesConfig>) -> Penalties {
        let Some(cfg) = opt else { return Penalties::new() };
        Penalties {
            nline_penalty: cfg.nline_penalty,
            overflow_penalty: cfg.overflow_penalty,
            short_last_line_fraction: cfg.short_last_line_fraction,
            short_last_line_penalty: cfg.short_last_line_penalty,
            hyphen_penalty: cfg.hyphen_penalty,
        }
    }

    pub fn get_assets_path<P: AsRef<Path>>(&self, rel_path: P) -> Option<PathBuf> {
        self.assets_path.as_ref().map(|p| p.join(rel_path))
    }

    pub fn get_data_path<P: AsRef<Path>>(&self, rel_path: P) -> Option<PathBuf> {
        self.path.as_ref().map(|p| p.join(rel_path))
    }

    pub fn load_asset_metadata<P: AsRef<Path>>(&self, rel_path: P) -> AssetMetadata {
        let mut path = rel_path.as_ref().to_owned();
        path.set_extension("json");
        self.load_assets_dict::<AssetInfo<()>, _>(Some(path))
            .unwrap_or_default()
            .metadata()
    }

    pub fn load_asset_info<P: AsRef<Path>, T: DeserializeOwned>(&self, rel_path: P) -> AssetInfo<T> {
        let mut path = rel_path.as_ref().to_owned();
        path.set_extension("json");
        self.load_assets_dict(Some(path)).unwrap_or_default()
    }
}

#[derive(Deserialize, Clone)]
pub struct LocalizedDataConfig {
    pub localize_dict: Option<String>,
    pub hashed_dict: Option<String>,
    pub text_data_dict: Option<String>,
    pub character_system_text_dict: Option<String>,
    pub race_jikkyo_comment_dict: Option<String>,
    pub race_jikkyo_message_dict: Option<String>,
    pub assets_dir: Option<String>,
    #[serde(default)]
    pub extra_asset_bundle: OsOption<String>,
    pub replacement_font_name: Option<String>,

    pub plural_form: Option<String>,
    pub ordinal_form: Option<String>,
    #[serde(default)]
    pub ordinal_types: Vec<String>,
    #[serde(default)]
    pub months: Vec<String>,
    pub month_text_format: Option<String>,

    #[serde(default)]
    pub use_text_wrapper: bool,
    // Predefined line widths are counts of cjk characters.
    // 1 cjk char = 2 columns, so setting this value to 2 replicates the default behaviour.
    pub line_width_multiplier: Option<f32>,
    #[serde(default)]
    pub systext_cue_lines: FnvHashMap<String, i32>,
    pub wrapper_penalties: Option<PenaltiesConfig>,

    #[serde(default)]
    pub auto_adjust_story_clip_length: bool,
    pub story_line_count_offset: Option<i32>,
    pub text_frame_line_spacing_multiplier: Option<f32>,
    pub text_frame_font_size_multiplier: Option<f32>,
    #[serde(default)]
    pub skill_formatting: SkillFormatting,
    #[serde(default)]
    pub text_common_allow_overflow: bool,
    #[serde(default)]
    pub now_loading_comic_title_ellipsis: bool,

    #[serde(default)]
    pub remove_ruby: bool,
    pub character_note_top_gallery_button: Option<UITextConfig>,
    pub character_note_top_talk_gallery_button: Option<UITextConfig>,

    pub news_url: Option<String>,

    // RESERVED
    #[serde(default)]
    pub _debug: i32,
}

#[derive(Deserialize, Clone)]
pub struct UITextConfig {
    pub text: Option<String>,
    pub font_size: Option<i32>,
    pub line_spacing: Option<f32>,
}

impl Default for LocalizedDataConfig {
    fn default() -> Self {
        default_serde_instance().expect("default instance")
    }
}

#[derive(Deserialize, Clone)]
pub struct PenaltiesConfig {
    nline_penalty: usize,
    overflow_penalty: usize,
    short_last_line_fraction: usize,
    short_last_line_penalty: usize,
    hyphen_penalty: usize,
}

#[derive(Deserialize, Clone)]
pub struct SkillFormatting {
    #[serde(default = "SkillFormatting::default_length")]
    pub name_length: i32,
    #[serde(default = "SkillFormatting::default_length")]
    pub desc_length: i32,
    #[serde(default = "SkillFormatting::default_lines")]
    pub name_short_lines: i32,

    #[serde(default = "SkillFormatting::default_mult")]
    pub name_short_mult: f32,
    #[serde(default = "SkillFormatting::default_mult")]
    pub name_sp_mult: f32,
}
impl SkillFormatting {
    fn default_length() -> i32 {
        18
    }
    fn default_lines() -> i32 {
        1
    }
    fn default_mult() -> f32 {
        1.0
    }
}

impl Default for SkillFormatting {
    fn default() -> Self {
        SkillFormatting {
            name_length: 13,
            desc_length: 18,
            name_short_lines: 1,
            name_short_mult: 1.0,
            name_sp_mult: 1.0,
        }
    }
}
