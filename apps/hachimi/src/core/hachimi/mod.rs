//! The `Hachimi` application singleton: lifecycle, hooking, and shared state.
//!
//! Domain types live in submodules and are re-exported flatly so existing
//! `hachimi::Config` / `hachimi::Language` / etc. call sites keep working.

mod assets;
mod config;
mod language;
mod localized_data;

pub use assets::{AssetInfo, AssetMetadata};
pub use config::{Config, OsOption};
pub use language::Language;
pub use localized_data::{LocalizedData, LocalizedDataConfig, PenaltiesConfig, SkillFormatting, UITextConfig};

use arc_swap::ArcSwap;
use once_cell::sync::OnceCell;
use serde::Deserialize;
use std::{
    fs,
    path::{Path, PathBuf},
    process,
    sync::{
        atomic::{self, AtomicBool, AtomicI32},
        Arc, Mutex,
    },
};

use crate::{
    core::{plugin::Plugin, updater},
    gui_impl, hachimi_impl,
    il2cpp::{
        self,
        hook::umamusume::GameSystem,
        sql::{CharacterData, SkillInfo},
    },
};

use super::{
    game::{Game, Region},
    hosted_data, ipc, template, template_filters, tl_repo, utils, Error, Interceptor,
};

pub const REPO_PATH: &str = "jalbarrang/hachimi-redux";
pub const GITHUB_API: &str = "https://api.github.com/repos";
pub const CODEBERG_API: &str = "https://codeberg.org/api/v1/repos";
pub const WEBSITE_URL: &str = "https://hachimi.noccu.art";

pub static CONFIG_LOAD_ERROR: AtomicBool = AtomicBool::new(false);

pub struct Hachimi {
    // Hooking stuff
    pub interceptor: Interceptor,
    pub hooking_finished: AtomicBool,
    pub plugins: Mutex<Vec<Plugin>>,

    // Localized data
    pub localized_data: ArcSwap<LocalizedData>,
    pub tl_updater: Arc<tl_repo::Updater>,
    /// GameTora catalog snapshots sync.
    pub gametora_updater: Arc<hosted_data::Updater>,
    /// Training-tracker generated resources (skill_grades / course_params) sync.
    pub tracker_updater: Arc<hosted_data::Updater>,

    // Character data
    pub chara_data: ArcSwap<CharacterData>,
    // Untranslated skill info
    pub skill_info: ArcSwap<SkillInfo>,

    // Shared properties
    pub game: Game,
    pub config: ArcSwap<Config>,
    pub template_parser: template::Parser,

    /// -1 = default
    pub target_fps: AtomicI32,

    #[cfg(target_os = "windows")]
    pub vsync_count: AtomicI32,

    #[cfg(target_os = "windows")]
    pub window_always_on_top: AtomicBool,

    #[cfg(target_os = "windows")]
    pub discord_rpc: AtomicBool,

    pub updater: Arc<updater::Updater>,
}

static INSTANCE: OnceCell<Arc<Hachimi>> = OnceCell::new();

impl Hachimi {
    pub fn init() -> bool {
        if INSTANCE.get().is_some() {
            warn!("Hachimi should be initialized only once");
            return true;
        }

        let instance = match Self::new() {
            Ok(v) => v,
            Err(e) => {
                super::log::init(false, false); // early init to log error
                error!("Init failed: {}", e);
                return false;
            }
        };

        let config = instance.config.load();
        if config.disable_gui_once {
            let mut config = config.as_ref().clone();
            config.disable_gui_once = false;
            _ = instance.save_config(&config);

            config.disable_gui = true;
            instance.config.store(Arc::new(config));
        }

        super::log::init(config.debug_mode, config.enable_file_logging);

        info!("Hachimi {}", env!("HACHIMI_DISPLAY_VERSION"));
        info!("Game region: {}", instance.game.region);
        instance.load_localized_data();

        INSTANCE.set(Arc::new(instance)).is_ok()
    }

    pub fn instance() -> Arc<Hachimi> {
        INSTANCE
            .get()
            .unwrap_or_else(|| {
                error!("FATAL: Attempted to get Hachimi instance before initialization");
                process::exit(1);
            })
            .clone()
    }

    pub fn is_initialized() -> bool {
        INSTANCE.get().is_some()
    }

    fn new() -> Result<Hachimi, Error> {
        let game = Game::init();
        let config = Self::load_config(&game.data_dir, &game.region)?;

        config.language.set_locale();

        Ok(Hachimi {
            interceptor: Interceptor::default(),
            hooking_finished: AtomicBool::new(false),
            plugins: Mutex::default(),

            // Don't load localized data initially since it might fail, logging the error is not possible here
            localized_data: ArcSwap::default(),
            tl_updater: Arc::default(),
            gametora_updater: Arc::new(hosted_data::Updater::new(&hosted_data::GAMETORA)),
            tracker_updater: Arc::new(hosted_data::Updater::new(&hosted_data::TRACKER)),

            // Same with these
            chara_data: ArcSwap::default(),
            skill_info: ArcSwap::default(),

            game,
            template_parser: template::Parser::new(&template_filters::LIST),

            target_fps: AtomicI32::new(config.target_fps.unwrap_or(-1)),

            #[cfg(target_os = "windows")]
            vsync_count: AtomicI32::new(config.windows.vsync_count),

            #[cfg(target_os = "windows")]
            window_always_on_top: AtomicBool::new(config.windows.window_always_on_top),

            #[cfg(target_os = "windows")]
            discord_rpc: AtomicBool::new(config.windows.discord_rpc),

            updater: Arc::default(),

            config: ArcSwap::new(Arc::new(config)),
        })
    }

    // region param is unused?
    fn load_config(data_dir: &Path, _region: &Region) -> Result<Config, Error> {
        let config_path = data_dir.join("config.json");
        if fs::metadata(&config_path).is_ok() {
            let json = fs::read_to_string(&config_path)?;
            match serde_json::from_str::<Config>(&json) {
                Ok(config) => Ok(config),
                Err(e) => {
                    eprintln!("Failed to parse config: {}", e);
                    CONFIG_LOAD_ERROR.store(true, std::sync::atomic::Ordering::Release);
                    Ok(Config::default())
                }
            }
        } else {
            Ok(Config::default())
        }
    }

    /// Push runtime-tweakable config values to their atomic mirrors and re-apply them
    /// to the running game, so saving/reloading config affects the live game (not just
    /// the on-disk file). Mirrors what the old quick Graphics toggles did.
    pub fn apply_runtime_config(&self, config: &Config) {
        use atomic::Ordering;

        self.target_fps
            .store(config.target_fps.unwrap_or(-1), Ordering::Relaxed);
        il2cpp::symbols::Thread::main_thread().schedule(|| {
            crate::il2cpp::hook::UnityEngine_CoreModule::Application::set_targetFrameRate(30);
        });

        #[cfg(target_os = "windows")]
        {
            use crate::il2cpp::hook::UnityEngine_CoreModule::QualitySettings;
            use crate::windows::{discord, utils::set_window_topmost, wnd_hook};

            self.vsync_count.store(config.windows.vsync_count, Ordering::Relaxed);
            il2cpp::symbols::Thread::main_thread().schedule(|| {
                QualitySettings::set_vSyncCount(1);
            });

            self.window_always_on_top
                .store(config.windows.window_always_on_top, Ordering::Relaxed);
            il2cpp::symbols::Thread::main_thread().schedule(|| {
                let topmost = Hachimi::instance().window_always_on_top.load(Ordering::Relaxed);
                // SAFETY: FFI / Win32 call required to toggle the window's topmost state
                unsafe {
                    _ = set_window_topmost(wnd_hook::get_target_hwnd(), topmost);
                }
            });

            let rpc = config.windows.discord_rpc;
            self.discord_rpc.store(rpc, Ordering::Relaxed);
            if let Err(e) = if rpc { discord::start_rpc() } else { discord::stop_rpc() } {
                error!("{}", e);
            }
        }
    }

    pub fn reload_config(&self) {
        let new_config = match Self::load_config(&self.game.data_dir, &self.game.region) {
            Ok(v) => v,
            Err(e) => {
                error!("Failed to reload config: {}", e);
                return;
            }
        };

        new_config.language.set_locale();
        self.apply_runtime_config(&new_config);
        self.config.store(Arc::new(new_config));
        super::plugin::events::dispatch_config_reload();
    }

    pub fn save_config(&self, config: &Config) -> Result<(), Error> {
        fs::create_dir_all(&self.game.data_dir)?;
        let config_path = self.get_data_path("config.json");
        utils::write_json_file(config, &config_path)?;

        Ok(())
    }

    pub fn save_and_reload_config(&self, config: Config) -> Result<(), Error> {
        self.save_config(&config)?;

        config.language.set_locale();
        self.apply_runtime_config(&config);
        self.config.store(Arc::new(config));
        super::plugin::events::dispatch_config_reload();
        Ok(())
    }

    pub fn load_localized_data(&self) {
        if self.tl_updater.progress().is_some() {
            warn!("Update in progress, not loading localized data");
            return;
        }
        let new_data = match LocalizedData::new(&self.config.load(), &self.game.data_dir) {
            Ok(v) => v,
            Err(e) => {
                error!("Failed to load localized data: {}", e);
                return;
            }
        };
        self.localized_data.store(Arc::new(new_data));
    }

    pub fn init_character_data(&self) {
        if self.chara_data.load().chara_ids.is_empty() {
            let data = CharacterData::load_from_db();
            self.chara_data.store(Arc::new(data));
            info!("Character database loaded successfully.");
        }
    }

    pub fn init_skill_info(&self) {
        if self.skill_info.load().skill_names.is_empty() {
            let data = SkillInfo::load_from_db();
            self.skill_info.store(Arc::new(data));
            info!("Skill info loaded successfully.");
        }
    }

    pub fn on_dlopen(&self, filename: &str, handle: usize) -> bool {
        // Prevent double initialization
        if self.hooking_finished.load(atomic::Ordering::Relaxed) {
            return false;
        }

        if hachimi_impl::is_il2cpp_lib(filename) {
            info!("Got il2cpp handle");
            il2cpp::symbols::set_handle(handle);
            false
        } else if hachimi_impl::is_criware_lib(filename) {
            self.on_hooking_finished();
            true
        } else {
            false
        }
    }

    pub fn on_hooking_finished(&self) {
        self.hooking_finished.store(true, atomic::Ordering::Relaxed);

        info!("GameAssembly finished loading");
        il2cpp::symbols::init();
        il2cpp::hook::init();

        // By the time it finished hooking the game will have already finished initializing
        GameSystem::on_game_initialized();

        let config = self.config.load();
        if !config.disable_gui {
            gui_impl::init();
        }

        if config.enable_ipc {
            ipc::start_http(config.ipc_listen_all);
        }

        hachimi_impl::on_hooking_finished(self);

        for plugin in self.plugins.lock().expect("lock poisoned").iter() {
            info!("Initializing plugin: {}", plugin.name);
            let res = plugin.init();
            if !res.is_ok() {
                info!("Plugin init failed");
            }
        }
    }

    pub fn get_data_path<P: AsRef<Path>>(&self, rel_path: P) -> PathBuf {
        self.game.data_dir.join(rel_path)
    }

    pub fn run_auto_update_check(&self) {
        if !self.config.load().disable_auto_update_check {
            // Check for hachimi updates first, then translations
            // Don't auto check for tl updates if it's not up to date
            self.updater.clone().check_for_updates(|new_update| {
                let hachimi = Hachimi::instance();
                if !new_update && !hachimi.config.load().translator_mode {
                    hachimi.tl_updater.clone().check_for_updates(false);
                }
            });
        }
        // Independent of the hachimi/translation update flow: refresh the hosted
        // data sets (each gated by its own config flag inside `sync`).
        if !self.config.load().disable_auto_update_check {
            self.gametora_updater.clone().sync(false);
            self.tracker_updater.clone().sync(false);
        }
    }
}

/// Builds a `T` from an empty serde map, i.e. using only `#[serde(default)]` field
/// defaults. Shared by `Config` and `LocalizedDataConfig` default impls.
pub(crate) fn default_serde_instance<'a, T: Deserialize<'a>>() -> Option<T> {
    let empty_data = std::iter::empty::<((), ())>();
    let empty_deserializer = serde::de::value::MapDeserializer::<_, serde::de::value::Error>::new(empty_data);
    T::deserialize(empty_deserializer).ok()
}
