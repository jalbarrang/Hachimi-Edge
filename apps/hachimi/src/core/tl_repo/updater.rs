//! Translation updater: update detection, user prompts, and run orchestration.
//! The heavy download workers live in [`super::download`].

use std::{
    collections::HashSet,
    fs,
    path::Path,
    sync::{Arc, Mutex},
};

use arc_swap::ArcSwap;
use fnv::FnvHashMap;
use rust_i18n::t;
use size::Size;

use crate::core::{
    gui::{NotificationGuard, SimpleYesNoDialog},
    hachimi::LocalizedData,
    http, utils, Error, Gui, Hachimi,
};

use super::repo::{RepoCache, RepoFile, RepoIndex, REPO_CACHE_FILENAME};

const LOCALIZED_DATA_DIR: &str = "localized_data";
const REPO_EXCLUDES_FILENAME: &str = "excludes.txt";

const INCREMENTAL_UPDATE_LIMIT_GITHUB: usize = 55;
const INCREMENTAL_UPDATE_LIMIT_GITLAB: usize = 250;
const INCREMENTAL_SIZE_RATIO_THRESHOLD: f64 = 0.8;
const ZIP_SIZE_WARNING_RATIO: f64 = 1.2; // Warn if ZIP is 1.2x larger than changes

#[derive(Clone)]
pub(super) struct UpdateInfo {
    pub(super) base_url: String,
    pub(super) zip_url: String,
    pub(super) zip_dir: String,
    pub(super) files: Vec<RepoFile>, // only contains files needed for update
    pub(super) is_new_repo: bool,
    pub(super) cached_files: FnvHashMap<String, String>, // from repo cache
    pub(super) size: usize,
    // New fields for better user communication, idk why it complains about these never being read
    #[allow(dead_code)]
    pub(super) update_size: usize, // Size of changed files only
    #[allow(dead_code)]
    pub(super) total_size: usize, // Total size of all files (for ZIP downloads)
    pub(super) will_use_zip: bool,   // Whether ZIP download will be used
    pub(super) modifies_atlas: bool, // Whether file updates include atlases
}

#[derive(Default, Clone)]
pub struct UpdateProgress {
    pub current: usize,
    pub total: usize,
}

impl UpdateProgress {
    pub fn new(current: usize, total: usize) -> UpdateProgress {
        UpdateProgress { current, total }
    }
}

#[derive(Default)]
pub struct Updater {
    update_check_mutex: Mutex<()>,
    pub(super) new_update: ArcSwap<Option<UpdateInfo>>,
    pub(super) progress: ArcSwap<Option<UpdateProgress>>,
}

impl Updater {
    pub fn check_for_updates(self: Arc<Self>, pedantic: bool) {
        std::thread::spawn(move || {
            if let Err(e) = self.check_for_updates_internal(pedantic) {
                if let Some(mutex) = Gui::instance() {
                    mutex
                        .lock()
                        .expect("lock poisoned")
                        .show_notification(&format!("{}", e));
                }
                info!("{}", e);
            }
        });
    }

    fn is_github_hosted(url: &str) -> bool {
        url.contains("github.com") || url.contains("githubusercontent.com") || url.contains("github.io")
    }

    fn is_gitlab_hosted(url: &str) -> bool {
        url.contains("gitlab.com") || url.contains("gitlab.io")
    }

    fn should_use_zip_download(file_count: usize, update_size: usize, total_size: usize, base_url: &str) -> bool {
        // if it's on GitHub and the update has > 55 files, use ZIP to avoid 403 errors
        if Self::is_github_hosted(base_url) && file_count > INCREMENTAL_UPDATE_LIMIT_GITHUB {
            return true;
        }

        // for GitLab, 250 file limit is a safe safe buffer below the raw endpoint cap of 300
        if Self::is_gitlab_hosted(base_url) && file_count > INCREMENTAL_UPDATE_LIMIT_GITLAB {
            return true;
        }

        // as long as the update is less than 80% of the total size of the repo, keep it incremental
        if (update_size as f64) < (total_size as f64 * INCREMENTAL_SIZE_RATIO_THRESHOLD) {
            return false;
        }

        // if the update >80% of the repo size, just grab the ZIP
        true
    }

    fn check_for_updates_internal(&self, pedantic: bool) -> Result<(), Error> {
        // Prevent multiple update checks running at the same time
        let Ok(_guard) = self.update_check_mutex.try_lock() else {
            return Ok(());
        };

        let hachimi = Hachimi::instance();
        let config = hachimi.config.load();
        let Some(index_url) = &config.translation_repo_index else {
            return Ok(());
        };
        let ld_dir_path = config.localized_data_dir.as_ref().map(|p| hachimi.get_data_path(p));

        let checking_notif_id = Gui::instance().map(|mutex| {
            mutex
                .lock()
                .expect("unexpected failure")
                .show_persistent_notification(&t!("notification.checking_for_tl_updates"))
        });
        let _guard = checking_notif_id.map(NotificationGuard);

        let index: RepoIndex = http::get_json(index_url)?;

        let cache_path = hachimi.get_data_path(REPO_CACHE_FILENAME);
        let repo_cache = if fs::metadata(&cache_path).is_ok() {
            let json = fs::read_to_string(&cache_path)?;
            serde_json::from_str(&json)?
        } else {
            RepoCache::default()
        };

        let excludes_path = hachimi.get_data_path(REPO_EXCLUDES_FILENAME);
        let excludes: HashSet<String> = if excludes_path.exists() {
            fs::read_to_string(&excludes_path)
                .unwrap_or_default()
                .lines()
                .map(|l| l.trim().replace("\\", "/")) // normalize to match repo format
                .filter(|l| !l.is_empty())
                .collect()
        } else {
            HashSet::new()
        };

        let is_new_repo = index.base_url != repo_cache.base_url;
        let mut modifies_atlas = false;
        let mut update_files: Vec<RepoFile> = Vec::new();
        let mut update_size: usize = 0;
        let mut total_size: usize = 0;
        for file in index.files.iter() {
            if file.path.contains("..") || Path::new(&file.path).has_root() {
                warn!("File path '{}' sanitized", file.path);
                continue;
            }

            let path = ld_dir_path.as_ref().map(|p| p.join(&file.path));
            let exists = path.as_ref().is_some_and(|p| p.is_file());

            let updated = if is_new_repo {
                // redownload every single file because the directory will be deleted
                true
            } else if !pedantic && exists && excludes.contains(&file.path) {
                // skip excluded file unless pedantic update or the file doesn't exist in the system
                false
            } else if let Some(hash) = repo_cache.files.get(&file.path) {
                // lazy auto update, cached hash and repo hash matches. ignored during pedantic
                if !pedantic && config.lazy_translation_updates && hash == &file.hash {
                    false
                } else if let Some(path) = path {
                    // get path or force download if path is invalid
                    // file doesn't exist -> download
                    if !exists
                        || hash != &file.hash
                        || fs::metadata(&path).map_or(true, |m| m.len() as usize != file.size)
                    {
                        true
                    } else if pedantic {
                        // full blake3 integrity check if user requested pedantic update
                        !file.verify_integrity(&path)
                    } else {
                        false // everything matches -> skip
                    }
                } else {
                    true // path invalid -> download
                }
            } else {
                // file doesn't exist in cache at all -> download it
                true
            };

            if updated {
                update_files.push(file.clone());
                update_size += file.size;
                if file.path.contains("/atlas/") {
                    modifies_atlas = true;
                }
            }
            total_size += file.size;
        }

        if !update_files.is_empty() {
            // Determine download strategy
            let will_use_zip =
                Self::should_use_zip_download(update_files.len(), update_size, total_size, &index.base_url);

            // Calculate actual download size
            let actual_download_size = if will_use_zip { total_size } else { update_size };

            // Store update info with all relevant sizes
            self.new_update.store(Arc::new(Some(UpdateInfo {
                is_new_repo,
                base_url: index.base_url,
                zip_url: index.zip_url,
                zip_dir: index.zip_dir,
                files: update_files,
                cached_files: repo_cache.files,
                size: actual_download_size,
                update_size,
                total_size,
                will_use_zip,
                modifies_atlas,
            })));

            if let Some(mutex) = Gui::instance() {
                // Determine the dialog message based on download strategy
                let dialog_message = if will_use_zip && update_size > 0 {
                    let size_ratio = total_size as f64 / update_size.max(1) as f64;

                    if size_ratio >= ZIP_SIZE_WARNING_RATIO {
                        // Warn user about larger ZIP download
                        debug!(
                            "ZIP download warning: changed={} MB, total={} MB, ratio={:.2}x",
                            update_size / (1024 * 1024),
                            total_size / (1024 * 1024),
                            size_ratio
                        );

                        t!(
                            "tl_update_dialog.content_zip_warning",
                            changed_size = Size::from_bytes(update_size),
                            download_size = Size::from_bytes(total_size)
                        )
                    } else {
                        // ZIP is being used but size difference is not significant
                        t!(
                            "tl_update_dialog.content",
                            size = Size::from_bytes(actual_download_size)
                        )
                    }
                } else {
                    // Incremental update or no warning needed
                    t!(
                        "tl_update_dialog.content",
                        size = Size::from_bytes(actual_download_size)
                    )
                };

                mutex
                    .lock()
                    .expect("lock poisoned")
                    .show_window(Box::new(SimpleYesNoDialog::new(
                        &t!("tl_update_dialog.title"),
                        &dialog_message,
                        |ok| {
                            if !ok {
                                return;
                            }
                            Hachimi::instance().tl_updater.clone().run();
                        },
                    )));
            }
        } else if let Some(mutex) = Gui::instance() {
            mutex
                .lock()
                .expect("unexpected failure")
                .show_notification(&t!("notification.no_tl_updates"));
        }

        Ok(())
    }

    pub fn run(self: Arc<Self>) {
        std::thread::Builder::new()
            .name("tl_repo_updater".into())
            .stack_size(8 * 1024 * 1024) // increase stack size to 8MB to prevent 0xc0000409 (Stack Buffer Overrun) during single-threaded downloads
            .spawn(move || {
                if let Err(e) = self.clone().run_internal() {
                    error!("{}", e);
                    self.progress.store(Arc::new(None));
                    if let Some(mutex) = Gui::instance() {
                        mutex.lock().expect("lock poisoned").show_notification(&t!("notification.update_failed", reason = e.to_string()));
                    }
                }
            })
            .expect("Failed to spawn updater thread");
    }

    pub(super) fn create_dir(path: &Path, override_exists: bool) -> Result<(), Error> {
        if override_exists {
            // rm -rf
            if let Ok(meta) = fs::metadata(path) {
                if meta.is_dir() {
                    fs::remove_dir_all(path)?;
                }
            }
        }

        // mkdir -p
        fs::create_dir_all(path)?;
        Ok(())
    }

    fn run_internal(self: Arc<Self>) -> Result<(), Error> {
        let Some(update_info) = (**self.new_update.load()).clone() else {
            return Ok(());
        };
        self.new_update.store(Arc::new(None));

        self.progress
            .store(Arc::new(Some(UpdateProgress::new(0, update_info.size))));
        if let Some(mutex) = Gui::instance() {
            mutex.lock().expect("lock poisoned").update_progress_visible = true;
        }

        // Empty the localized data so files couldnt be accessed while update is in progress
        let hachimi = Hachimi::instance();
        hachimi.localized_data.store(Arc::new(LocalizedData::default()));

        let localized_data_dir = hachimi.get_data_path(LOCALIZED_DATA_DIR);

        if update_info.is_new_repo {
            Self::create_dir(&localized_data_dir, true)?;
        } else {
            Self::create_dir(&localized_data_dir, false)?;
        }

        // Download the files - use the pre-determined strategy
        let cached_files = Arc::new(Mutex::new(update_info.cached_files.clone()));
        let error_count = if update_info.will_use_zip {
            self.clone()
                .download_zip(&update_info, &localized_data_dir, cached_files.clone())
        } else {
            self.clone()
                .download_incremental(&update_info, &localized_data_dir, cached_files.clone())
        }?;

        // Modify the config if needed
        let config = hachimi.config.load();
        if config.localized_data_dir.is_none() {
            let mut new_config = (**config).clone();
            new_config.localized_data_dir = Some(LOCALIZED_DATA_DIR.to_owned());
            hachimi.save_and_reload_config(new_config)?;
        }
        if config.apply_atlas_workaround && (update_info.modifies_atlas || update_info.will_use_zip) {
            let mut new_config = (**config).clone();
            new_config.apply_atlas_workaround = false;
            hachimi.save_and_reload_config(new_config)?;
            if let Some(gui_mutex) = Gui::instance() {
                gui_mutex
                    .lock()
                    .expect("unexpected failure")
                    .show_notification(&t!("notification.atlas_workaround_reset"));
            }
        }

        // Drop the download state
        self.progress.store(Arc::new(None));

        // Reload the localized data
        hachimi.load_localized_data();

        // Save the repo cache (done last so if any of the previous fails, the entire update would be voided)
        let repo_cache = RepoCache {
            base_url: update_info.base_url,
            files: cached_files.lock().expect("lock poisoned").clone(),
        };
        let cache_path = hachimi.get_data_path(REPO_CACHE_FILENAME);
        utils::write_json_file(&repo_cache, &cache_path)?;

        if let Some(mutex) = Gui::instance() {
            let mut gui = mutex.lock().expect("lock poisoned");
            gui.show_notification(&t!("notification.update_completed"));
            if error_count > 0 {
                gui.show_notification(&t!("notification.errors_during_update", count = error_count));
            }
        }
        Ok(())
    }

    pub fn progress(&self) -> Option<UpdateProgress> {
        (**self.progress.load()).clone()
    }
}
