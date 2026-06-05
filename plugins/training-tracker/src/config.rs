//! Unified on-disk plugin config (`hachimi/training_config.json`, next to
//! Hachimi's own `config.json`).
//!
//! This module owns the persisted struct and the load/persist path. Each feature
//! module keeps its own in-memory state and exposes a getter/setter; `config`
//! bridges those to disk so there is a single source of truth for the file format:
//! - [`crate::stat_targets`] — per-stat training targets
//! - [`crate::tabs`] — enabled overlay tabs
//! - [`crate::recommend`] — smart-recommendation tuning params
//!
//! Back-compat: every field is `#[serde(default)]`, so older configs (and configs
//! written by older plugin versions) load fine with sensible defaults.

use serde::{Deserialize, Serialize};

use crate::{overlay_prefs, recommend, stat_targets, tabs};

#[derive(Debug, Serialize, Deserialize)]
struct PersistedConfig {
    #[serde(default)]
    stat_targets: [i32; 5],
    #[serde(default = "default_enabled_tabs")]
    enabled_tabs: u8,
    #[serde(default)]
    recommend: recommend::RecommendParams,
    #[serde(default = "overlay_prefs::default_zoom")]
    overlay_zoom: f32,
}

impl Default for PersistedConfig {
    fn default() -> Self {
        Self {
            stat_targets: [0; 5],
            enabled_tabs: default_enabled_tabs(),
            recommend: recommend::RecommendParams::default(),
            overlay_zoom: overlay_prefs::default_zoom(),
        }
    }
}

fn default_enabled_tabs() -> u8 {
    tabs::ALL_ENABLED_MASK
}

fn config_path() -> std::path::PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|dir| dir.join("hachimi").join("training_config.json")))
        .unwrap_or_else(|| std::path::PathBuf::from("training_config.json"))
}

/// Load persisted config into the feature modules. Call once on plugin init.
/// A missing or invalid file leaves every module at its defaults.
pub fn load() {
    let path = config_path();
    let cfg = match std::fs::read(&path) {
        Ok(bytes) => match serde_json::from_slice::<PersistedConfig>(&bytes) {
            Ok(cfg) => cfg,
            Err(e) => {
                hlog_warn!(target: "training-tracker", "training config parse failed: {e}");
                return;
            }
        },
        Err(_) => return,
    };

    stat_targets::set_targets(cfg.stat_targets);
    tabs::set_enabled_mask(cfg.enabled_tabs);
    recommend::set_params(cfg.recommend);
    overlay_prefs::set_zoom(cfg.overlay_zoom);
    hlog_info!(
        target: "training-tracker",
        "config loaded: targets={:?} tabs={:#06b}",
        stat_targets::targets(),
        tabs::enabled_mask()
    );
}

/// Gather the current state from every feature module and write it to disk.
/// Call when the user commits a settings edit.
pub fn persist() {
    let cfg = PersistedConfig {
        stat_targets: stat_targets::targets(),
        enabled_tabs: tabs::enabled_mask(),
        recommend: recommend::params(),
        overlay_zoom: overlay_prefs::zoom(),
    };
    let Ok(bytes) = serde_json::to_vec_pretty(&cfg) else {
        return;
    };
    let path = config_path();
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Err(e) = std::fs::write(&path, bytes) {
        hlog_warn!(target: "training-tracker", "config persist failed: {e}");
    }
}
