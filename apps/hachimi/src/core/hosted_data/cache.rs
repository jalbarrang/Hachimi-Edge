//! On-disk cache manifest tracking the last-synced hash per snapshot filename.
//!
//! The set of files to fetch is driven entirely by the hosted `manifest.json`;
//! the runtime just mirrors whatever filenames that manifest lists, after
//! sanitizing them. The cache-manifest filename is per-set ([`super::DataSet`]).

use fnv::FnvHashMap;
use serde::{Deserialize, Serialize};

/// Persisted record of the last successful sync: content hash per filename.
#[derive(Serialize, Deserialize, Default)]
pub(super) struct CacheManifest {
    #[serde(default)]
    pub(super) synced_at: String,
    #[serde(default)]
    pub(super) files: FnvHashMap<String, String>,
}

/// Reject filenames that could escape the cache dir or nest into subdirs.
/// Hosted snapshots are always flat (e.g. `skills.json`).
pub(super) fn is_safe_filename(name: &str) -> bool {
    !name.is_empty()
        && name != "."
        && name != ".."
        && !name.contains('/')
        && !name.contains('\\')
        && !std::path::Path::new(name).is_absolute()
}
