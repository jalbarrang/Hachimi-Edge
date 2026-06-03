//! GameTora data sync: download skill / support-card / character-card / event
//! catalog snapshots directly from GameTora's content-hash manifest and cache
//! them under the game data dir for the training-tracker plugin to consume.
//!
//! This is a Rust port of uma-sim's `gametora-client.ts` + `sync-gametora.ts`:
//! - manifest: `GET https://gametora.com/data/manifests/umamusume.json` -> `{key: hash}`
//! - data file: `https://gametora.com/data/umamusume/{key}.{hash}.json`
//!
//! Snapshots are stored **as-is** (uma-sim ADR-0002): no transformation at fetch
//! time. Only keys whose manifest hash changed (or whose snapshot is missing) are
//! re-downloaded; a local cache manifest tracks the last-synced hashes.
//!
//! Submodules are re-exported flatly so call sites use `gametora_data::*`.

mod cache;
mod client;
mod updater;

pub use updater::Updater;

/// Subdirectory under the game data dir where snapshots are cached. Kept in sync
/// with the plugin ABI's `GAMETORA_DATA_SUBDIR` so plugins resolve the same path.
pub const DATA_SUBDIR: &str = hachimi_plugin_abi::GAMETORA_DATA_SUBDIR;
