//! Publishes the training-tracker generated resources for **hosted download**.
//!
//! Copies `plugins/training-tracker/assets/{skill_grades,course_params}.json`
//! into `data/` and (re)writes `data/manifest.json` (`filename -> blake3`), the
//! same manifest contract the host's `hosted_data` sync consumes for the GameTora
//! set. Clients then download these committed files from the repo's raw GitHub
//! URL — the JSONs are never embedded in any binary nor attached to a release.
//!
//! **Run manually** by the maintainer after regenerating the assets (the assets
//! themselves come from master.mdb via `tools/skill-grades` / `tools/course-data`,
//! which need the DB and so cannot run in CI):
//!
//! ```text
//! cargo run -p tracker-data-manifest
//! ```
//!
//! Then commit the updated `data/` files + `data/manifest.json`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Serialize;

/// Asset files published for download (relative to the assets dir + `data/`).
const FILES: &[&str] = &["skill_grades.json", "course_params.json"];
const MANIFEST_FILE: &str = "manifest.json";

/// Our published manifest: `{ generated_at, source, files: { filename: blake3 } }`.
/// Mirrors `tools/gametora-sync`'s contract so one client reads both.
#[derive(Serialize)]
struct HostedManifest {
    generated_at: String,
    source: String,
    files: BTreeMap<String, String>,
}

/// Repo root: this crate lives at `tools/tracker-data-manifest`, so two up.
fn repo_root() -> PathBuf {
    let here = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    here.canonicalize().unwrap_or(here)
}

fn main() -> Result<(), String> {
    let root = repo_root();
    let assets = root.join("plugins/training-tracker/assets");
    let out_dir = root.join("data");
    std::fs::create_dir_all(&out_dir).map_err(|e| format!("mkdir {}: {e}", out_dir.display()))?;

    let mut files = BTreeMap::new();
    for name in FILES {
        let src = assets.join(name);
        let bytes = std::fs::read(&src).map_err(|e| format!("reading {}: {e}", src.display()))?;
        // Validate JSON before publishing so a corrupt asset never lands in `data/`.
        serde_json::from_slice::<serde::de::IgnoredAny>(&bytes)
            .map_err(|e| format!("{name} is not valid JSON: {e}"))?;
        let dst = out_dir.join(name);
        std::fs::write(&dst, &bytes).map_err(|e| format!("writing {}: {e}", dst.display()))?;
        files.insert((*name).to_owned(), blake3::hash(&bytes).to_hex().to_string());
        println!("published {name} ({} bytes)", bytes.len());
    }

    let manifest = HostedManifest {
        generated_at: chrono::Utc::now().to_rfc3339(),
        source: "training-tracker assets (master.mdb)".to_owned(),
        files,
    };
    let json = serde_json::to_string_pretty(&manifest).map_err(|e| format!("serializing manifest: {e}"))?;
    let manifest_path = out_dir.join(MANIFEST_FILE);
    std::fs::write(&manifest_path, format!("{json}\n")).map_err(|e| format!("writing manifest: {e}"))?;
    println!("wrote {} ({} files)", manifest_path.display(), manifest.files.len());
    Ok(())
}
