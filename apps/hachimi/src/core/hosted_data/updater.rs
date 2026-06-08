//! Hosted-data sync orchestration: hash-diff a [`DataSet`]'s hosted manifest
//! against the local cache, download only changed snapshots, then persist the
//! cache manifest. Generic over the set.

use std::{
    fs,
    sync::{Arc, Mutex},
};

use crate::core::{gui::NotificationGuard, utils, Error, Gui, Hachimi};

use super::{
    cache::{is_safe_filename, CacheManifest},
    client, DataSet,
};

pub struct Updater {
    set: &'static DataSet,
    sync_mutex: Mutex<()>,
}

impl Updater {
    /// An updater bound to a hosted [`DataSet`].
    pub fn new(set: &'static DataSet) -> Self {
        Self {
            set,
            sync_mutex: Mutex::new(()),
        }
    }

    /// Spawn a background sync. Safe to call repeatedly; concurrent runs are
    /// skipped via the internal mutex.
    ///
    /// When `notify` is set (manual trigger), progress/result is surfaced through
    /// the GUI; the automatic launch sync passes `false` to stay silent unless it
    /// errors.
    pub fn sync(self: Arc<Self>, notify: bool) {
        let log_target = self.set.log_target;
        std::thread::Builder::new()
            .name(format!("{log_target}_sync"))
            .spawn(move || {
                if let Err(e) = self.sync_internal(notify) {
                    warn!(target: log_target, "data sync failed: {}", e);
                    if notify {
                        Self::notify(&(self.set.msg_failed)(&e.to_string()));
                    }
                }
            })
            .expect("Failed to spawn hosted-data sync thread");
    }

    fn notify(message: &str) {
        if let Some(mutex) = Gui::instance() {
            mutex.lock().expect("lock poisoned").show_notification(message);
        }
    }

    /// Persistent "syncing" indicator held while snapshots download; auto-closes
    /// when the returned guard drops (download finished, or errored out).
    fn show_loading(&self) -> Option<NotificationGuard> {
        let msg = (self.set.msg_syncing)();
        Gui::instance().map(|mutex| {
            let id = mutex.lock().expect("lock poisoned").show_persistent_notification(&msg);
            NotificationGuard(id)
        })
    }

    fn sync_internal(&self, notify: bool) -> Result<(), Error> {
        // Prevent overlapping syncs.
        let Ok(_guard) = self.sync_mutex.try_lock() else {
            return Ok(());
        };
        let set = self.set;
        let log_target = set.log_target;

        let hachimi = Hachimi::instance();
        let config = hachimi.config.load();
        if (set.is_disabled)(&config) {
            debug!(target: log_target, "data sync disabled by config");
            return Ok(());
        }
        let url_override = (set.url_override)(&config);
        let base = url_override.as_deref().unwrap_or(set.default_url);

        let data_dir = hachimi.get_data_path(set.subdir);
        let cache_path = data_dir.join(set.cache_filename);

        let mut cache: CacheManifest = if fs::metadata(&cache_path).is_ok() {
            serde_json::from_str(&fs::read_to_string(&cache_path)?).unwrap_or_default()
        } else {
            CacheManifest::default()
        };

        info!(target: log_target, "Checking hosted-data manifest...");
        let manifest = client::load_manifest(base)?;

        // Decide which files need a (re)download from the hosted manifest.
        let mut pending = Vec::new();
        for (file, remote_hash) in manifest.files.iter() {
            if !is_safe_filename(file) {
                warn!(target: log_target, "Skipping unsafe filename '{}' from manifest", file);
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
            info!(target: log_target, "hosted data already up to date");
            if notify {
                Self::notify(&(set.msg_up_to_date)());
            }
            return Ok(());
        }

        fs::create_dir_all(&data_dir)?;
        info!(target: log_target, "Syncing {} snapshot(s)...", pending.len());

        let mut updated = 0usize;
        {
            // Loading indicator visible only while snapshots are downloading.
            let _loading = self.show_loading();
            for (file, remote_hash) in pending {
                match client::fetch_snapshot(base, &file) {
                    Ok(text) => {
                        fs::write(data_dir.join(&file), text)?;
                        cache.files.insert(file.clone(), remote_hash);
                        updated += 1;
                        debug!(target: log_target, "Wrote {}", file);
                    }
                    Err(e) => {
                        // Non-fatal: keep the old cache entry so this file is retried.
                        warn!(target: log_target, "Failed to fetch '{}': {}", file, e);
                    }
                }
            }
        }

        if updated > 0 {
            cache.synced_at = chrono::Utc::now().to_rfc3339();
            utils::write_json_file(&cache, &cache_path)?;
            info!(target: log_target, "hosted data sync complete ({} updated)", updated);
        }
        if notify {
            Self::notify(&(set.msg_complete)(updated));
        }

        Ok(())
    }
}
