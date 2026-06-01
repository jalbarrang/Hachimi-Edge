//! Translation repository handling: repo/meta formats, update detection, and
//! parallel download workers.
//!
//! Submodules are re-exported flatly so existing `tl_repo::*` call sites keep
//! working:
//! - `repo` — meta index / repo index / file formats + `new_meta_index_request`
//! - `updater` — `Updater` orchestration + `UpdateProgress`
//! - `download` — incremental and ZIP download workers

mod download;
mod repo;
mod updater;

pub use repo::{new_meta_index_request, RepoInfo};
pub use updater::{UpdateProgress, Updater};
