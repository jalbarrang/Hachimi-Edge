//! Hosted-data HTTP client: our published manifest + per-file snapshot fetch.
//!
//! Points at the repo's committed `data/gametora/` (served via raw GitHub, CDN-
//! backed) rather than GameTora directly, so end users never hit GameTora. The
//! upstream extraction runs in CI (`tools/gametora-sync`).

use fnv::FnvHashMap;
use serde::Deserialize;

use crate::core::{http, Error};

/// Default hosted base URL (no trailing slash). Overridable via
/// `config.gametora_data_url` for development/testing.
pub(super) const DEFAULT_DATA_URL: &str =
    "https://raw.githubusercontent.com/jalbarrang/hachimi-redux/main/data/gametora";

const USER_AGENT: &str = concat!(
    "hachimi-redux/",
    env!("CARGO_PKG_VERSION"),
    " (+https://github.com/jalbarrang/hachimi-redux; gametora-data)"
);

/// Our published manifest (`manifest.json`): `filename -> content hash`.
/// `generated_at` / `source` are ignored at runtime.
#[derive(Deserialize)]
pub(super) struct HostedManifest {
    #[serde(default)]
    pub(super) files: FnvHashMap<String, String>,
}

fn agent() -> ureq::Agent {
    ureq::Agent::new_with_config(http::ureq_config())
}

fn fetch_string(url: &str) -> Result<String, Error> {
    let res = agent().get(url).header("User-Agent", USER_AGENT).call()?;
    Ok(res.into_body().read_to_string()?)
}

/// Download the hosted `manifest.json` from `base`.
pub(super) fn load_manifest(base: &str) -> Result<HostedManifest, Error> {
    let url = format!("{}/manifest.json", base.trim_end_matches('/'));
    Ok(serde_json::from_str(&fetch_string(&url)?)?)
}

/// Download a single snapshot file (raw JSON text, stored verbatim).
pub(super) fn fetch_snapshot(base: &str, file: &str) -> Result<String, Error> {
    let url = format!("{}/{}", base.trim_end_matches('/'), file);
    let text = fetch_string(&url)?;
    // Validate JSON before persisting so a truncated/HTML error never lands in cache.
    serde_json::from_str::<serde::de::IgnoredAny>(&text)?;
    Ok(text)
}
