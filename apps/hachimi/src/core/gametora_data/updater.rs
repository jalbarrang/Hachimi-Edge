//! GameTora data sync orchestration: hash-diff the hosted manifest against the
//! local cache, download only changed snapshots, then persist the cache manifest.

use std::{
    fs,
    sync::{Arc, Mutex},
};

use rust_i18n::t;

use crate::core::{gui::NotificationGuard, utils, Error, Gui, Hachimi};

use super::{
    cache::{is_safe_filename, CacheManifest, CACHE_FILENAME},
    client::{self, DEFAULT_DATA_URL},
    DATA_SUBDIR,
};

const LOG_TARGET: &str = "gametora_data";

#[derive(Default)]
pub struct Updater {
    sync_mutex: Mutex<()>,
}

impl Updater {
    /// Spawn a background sync. Safe to call repeatedly; concurrent runs are
    /// skipped via the internal mutex.
    ///
    /// When `notify` is set (manual trigger), progress/result is surfaced through
    /// the GUI; the automatic launch sync passes `false` to stay silent unless it
    /// errors.
    pub fn sync(self: Arc<Self>, notify: bool) {
        std::thread::Builder::new()
            .name("gametora_data_sync".into())
            .spawn(move || {
                if let Err(e) = self.sync_internal(notify) {
                    warn!(target: LOG_TARGET, "GameTora data sync failed: {}", e);
                    if notify {
                        Self::notify(&t!("notification.gametora_sync_failed", reason = e.to_string()));
                    }
                }
            })
            .expect("Failed to spawn GameTora sync thread");
    }

    fn notify(message: &str) {
        if let Some(mutex) = Gui::instance() {
            mutex.lock().expect("lock poisoned").show_notification(message);
        }
    }

    /// Persistent "syncing" indicator held while snapshots download; auto-closes
    /// when the returned guard drops (download finished, or errored out).
    fn show_loading() -> Option<NotificationGuard> {
        Gui::instance().map(|mutex| {
            let id = mutex
                .lock()
                .expect("lock poisoned")
                .show_persistent_notification(&t!("notification.gametora_syncing"));
            NotificationGuard(id)
        })
    }

    fn sync_internal(&self, notify: bool) -> Result<(), Error> {
        // Prevent overlapping syncs.
        let Ok(_guard) = self.sync_mutex.try_lock() else {
            return Ok(());
        };

        let hachimi = Hachimi::instance();
        let config = hachimi.config.load();
        if config.disable_gametora_data {
            debug!(target: LOG_TARGET, "GameTora data sync disabled by config");
            return Ok(());
        }
        let base = config.gametora_data_url.as_deref().unwrap_or(DEFAULT_DATA_URL);

        let data_dir = hachimi.get_data_path(DATA_SUBDIR);
        let cache_path = data_dir.join(CACHE_FILENAME);

        let mut cache: CacheManifest = if fs::metadata(&cache_path).is_ok() {
            serde_json::from_str(&fs::read_to_string(&cache_path)?).unwrap_or_default()
        } else {
            CacheManifest::default()
        };

        info!(target: LOG_TARGET, "Checking GameTora data manifest...");
        let manifest = client::load_manifest(base)?;

        // Decide which files need a (re)download from the hosted manifest.
        let mut pending = Vec::new();
        for (file, remote_hash) in manifest.files.iter() {
            if !is_safe_filename(file) {
                warn!(target: LOG_TARGET, "Skipping unsafe filename '{}' from manifest", file);
                continue;
            }
            let out_path = data_dir.join(file);
            let cached_hash = cache.files.get(file);
            let needs_fetch = cached_hash.is_none_or(|h| h != remote_hash) || !out_path.is_file();
            if needs_fetch {
                pending.push((file.clone(), remote_hash.clone()));
            }
        }

        if pending.is_empty() {
            info!(target: LOG_TARGET, "GameTora data already up to date");
            if notify {
                Self::notify(&t!("notification.gametora_up_to_date"));
            }
            return Ok(());
        }

        fs::create_dir_all(&data_dir)?;
        info!(target: LOG_TARGET, "Syncing {} GameTora snapshot(s)...", pending.len());

        let mut updated = 0usize;
        {
            // Loading indicator visible only while snapshots are downloading.
            let _loading = Self::show_loading();
            for (file, remote_hash) in pending {
                match client::fetch_snapshot(base, &file) {
                    Ok(text) => {
                        fs::write(data_dir.join(&file), text)?;
                        cache.files.insert(file.clone(), remote_hash);
                        updated += 1;
                        debug!(target: LOG_TARGET, "Wrote {}", file);
                    }
                    Err(e) => {
                        // Non-fatal: keep the old cache entry so this file is retried.
                        warn!(target: LOG_TARGET, "Failed to fetch '{}': {}", file, e);
                    }
                }
            }
        }

        if updated > 0 {
            cache.synced_at = chrono::Utc::now().to_rfc3339();
            utils::write_json_file(&cache, &cache_path)?;
            info!(target: LOG_TARGET, "GameTora data sync complete ({} updated)", updated);
        }
        if notify {
            Self::notify(&t!("notification.gametora_sync_complete", count = updated));
        }

        Ok(())
    }
}
