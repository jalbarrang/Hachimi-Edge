//! Translation repository formats: meta index, repo index, file entries, and
//! the on-disk hash cache.

use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
};

use fnv::FnvHashMap;
use serde::{Deserialize, Serialize};

use crate::core::{game::Region, http::AsyncRequest, Hachimi};

pub(super) const REPO_CACHE_FILENAME: &str = ".tl_repo_cache";

#[derive(Deserialize)]
pub struct RepoInfo {
    pub name: String,
    pub index: String,
    pub short_desc: Option<String>,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub region: Region,
}

impl RepoInfo {
    pub fn is_recommended(&self, current_lang_str: &str) -> bool {
        let Some(repo_tag) = self.language.as_deref() else {
            return false;
        };
        let repo_tag = repo_tag.to_lowercase();
        let target = current_lang_str.to_lowercase();

        if repo_tag == target || repo_tag.starts_with(&target) {
            return true;
        }

        let sys = sys_locale::get_locale().as_deref().unwrap_or("en").to_lowercase();
        repo_tag.starts_with(&sys) || sys.starts_with(&repo_tag)
    }
}

pub fn new_meta_index_request() -> AsyncRequest<Vec<RepoInfo>> {
    let meta_index_url = &Hachimi::instance().config.load().meta_index_url;

    let req = ureq::http::Request::builder()
        .uri(meta_index_url)
        .method("GET")
        .body(ureq::Body::builder().reader(std::io::empty()))
        .expect("Failed to build meta index request");

    AsyncRequest::with_json_response(req)
}

#[derive(Deserialize)]
pub(super) struct RepoIndex {
    pub(super) base_url: String,
    pub(super) zip_url: String,
    pub(super) zip_dir: String,
    pub(super) files: Vec<RepoFile>,
}

#[derive(Deserialize, Clone)]
pub(super) struct RepoFile {
    pub(super) path: String,
    pub(super) hash: String,
    pub(super) size: usize,
}

impl RepoFile {
    pub(super) fn get_fs_path(&self, root_dir: &Path) -> PathBuf {
        // Modern Windows versions support forward slashes anyways but it doesn't hurt to do something so trivial
        #[cfg(target_os = "windows")]
        return root_dir.join(self.path.replace("/", "\\"));

        #[cfg(not(target_os = "windows"))]
        return root_dir.join(&self.path);
    }
    pub(super) fn verify_integrity(&self, full_path: &Path) -> bool {
        let Ok(mut file) = fs::File::open(full_path) else {
            return false;
        };
        let mut hasher = blake3::Hasher::new();
        let mut buffer = vec![0u8; 8192];

        while let Ok(n) = file.read(&mut buffer) {
            if n == 0 {
                break;
            }
            hasher.update(&buffer[..n]);
        }

        hasher.finalize().to_hex().as_str() == self.hash
    }
}

#[derive(Serialize, Deserialize, Default)]
pub(super) struct RepoCache {
    pub(super) base_url: String,
    pub(super) files: FnvHashMap<String, String>, // path: hash
}
