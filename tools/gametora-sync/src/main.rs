//! GameTora data extractor (CI tool).
//!
//! Fetches GameTora's content-hash manifest, downloads the in-scope catalog
//! snapshots, and writes them — plus our own `manifest.json` (filename ->
//! blake3) — into the output directory (default `data/gametora`). Clients then
//! download these committed files from the repo's raw GitHub URL instead of
//! hitting GameTora directly.
//!
//! Usage: `gametora-sync [OUT_DIR]`  (default `data/gametora`)
//!
//! Fails fast (non-zero exit) on any fetch/parse error so CI never commits a
//! partial/inconsistent dataset.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use serde::Serialize;

const GAMETORA_BASE: &str = "https://gametora.com";
const USER_AGENT: &str = concat!(
    "hachimi-redux-gametora-sync/",
    env!("CARGO_PKG_VERSION"),
    " (+https://github.com/jalbarrang/hachimi-redux)"
);
const DEFAULT_OUT_DIR: &str = "data/gametora";
const MANIFEST_FILE: &str = "manifest.json";

/// In-scope catalog: GameTora manifest key -> output filename. Slashes in keys are
/// flattened to `-` for the on-disk name. Mirrors uma-sim `SYNC_TARGETS`.
const TARGETS: &[(&str, &str)] = &[
    ("skills", "skills.json"),
    ("character-cards", "character-cards.json"),
    ("support-cards", "support-cards.json"),
    ("support_effects", "support-effects.json"),
    ("training_events/ssr", "training-events-ssr.json"),
    ("training_events/sr", "training-events-sr.json"),
    ("training_events/shared", "training-events-shared.json"),
    ("training_events/friend", "training-events-friend.json"),
    ("training_events/group", "training-events-group.json"),
    ("dict/evrew", "evrew.json"),
    ("dict/te_names_en", "te-names-en.json"),
    ("dict/te_names_ja", "te-names-ja.json"),
];

/// Our published manifest: `{ generated_at, source, files: { filename: blake3 } }`.
#[derive(Serialize)]
struct HostedManifest {
    generated_at: String,
    source: String,
    files: BTreeMap<String, String>,
}

fn agent() -> ureq::Agent {
    ureq::Agent::new_with_defaults()
}

fn fetch_string(agent: &ureq::Agent, url: &str) -> Result<String, String> {
    agent
        .get(url)
        .header("User-Agent", USER_AGENT)
        .call()
        .map_err(|e| format!("GET {url} failed: {e}"))?
        .into_body()
        .read_to_string()
        .map_err(|e| format!("reading {url} failed: {e}"))
}

fn run(out_dir: &Path) -> Result<(), String> {
    let agent = agent();

    // 1. GameTora manifest: { key: hash }.
    let manifest_url = format!("{GAMETORA_BASE}/data/manifests/umamusume.json");
    let manifest: BTreeMap<String, String> = serde_json::from_str(&fetch_string(&agent, &manifest_url)?)
        .map_err(|e| format!("parsing GameTora manifest: {e}"))?;
    println!("GameTora manifest: {} entries", manifest.len());

    std::fs::create_dir_all(out_dir).map_err(|e| format!("mkdir {}: {e}", out_dir.display()))?;

    let mut files = BTreeMap::new();
    let mut changed = 0usize;

    // 2. Download each in-scope snapshot (fail fast on any error).
    for (key, file) in TARGETS {
        let hash = manifest
            .get(*key)
            .ok_or_else(|| format!("GameTora manifest missing key '{key}'"))?;
        let url = format!("{GAMETORA_BASE}/data/umamusume/{key}.{hash}.json");
        let text = fetch_string(&agent, &url)?;

        // Validate it parses as JSON before persisting.
        serde_json::from_str::<serde::de::IgnoredAny>(&text).map_err(|e| format!("'{key}' is not valid JSON: {e}"))?;

        let out_path = out_dir.join(file);
        let unchanged = std::fs::read(&out_path)
            .map(|old| old == text.as_bytes())
            .unwrap_or(false);
        if unchanged {
            println!("  [skip]  {file}");
        } else {
            std::fs::write(&out_path, &text).map_err(|e| format!("writing {}: {e}", out_path.display()))?;
            changed += 1;
            println!("  [write] {file}");
        }

        files.insert((*file).to_owned(), blake3::hash(text.as_bytes()).to_hex().to_string());
    }

    // 3. Write our manifest (always, so hashes/timestamp stay current).
    let hosted = HostedManifest {
        generated_at: chrono::Utc::now().to_rfc3339(),
        source: "gametora.com".to_owned(),
        files,
    };
    let manifest_path = out_dir.join(MANIFEST_FILE);
    let json = serde_json::to_string_pretty(&hosted).map_err(|e| format!("serializing manifest: {e}"))?;
    std::fs::write(&manifest_path, format!("{json}\n")).map_err(|e| format!("writing manifest: {e}"))?;

    println!(
        "Done: {} snapshot(s) changed, {} file(s) in manifest -> {}",
        changed,
        hosted.files.len(),
        out_dir.display()
    );
    Ok(())
}

fn main() -> ExitCode {
    let out_dir = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_OUT_DIR));

    match run(&out_dir) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
