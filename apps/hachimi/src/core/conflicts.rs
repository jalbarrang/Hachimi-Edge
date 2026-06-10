//! Detection of other game mods / DLL injectors that commonly conflict with
//! HachimiRedux.
//!
//! Stacking multiple injectors in the game directory (each hooking D3D / IL2CPP)
//! is the single most common cause of "the game crashes on launch" reports. This
//! module classifies the files found in the game directory so the host can warn
//! the user instead of crashing silently.
//!
//! This logic is intentionally platform-agnostic and unit-tested. The Windows
//! glue (enumerating the game directory, logging, surfacing a notification) lives
//! under `crate::windows`.
//!
//! NOTE: the installer keeps its own copy of the conflict list
//! (`apps/installer/src/installer.rs`). Keep the two in sync.

use std::path::Path;
use std::sync::OnceLock;

/// Summary of conflicts found by the startup scan, stashed so the GUI can surface
/// it as a notification once it initializes (the scan runs before the GUI exists).
static STARTUP_SUMMARY: OnceLock<Option<String>> = OnceLock::new();

/// Named third-party overlays / mods known to conflict. Lowercase basenames.
const KNOWN_OVERLAYS: &[&str] = &["horseact.dll", "heaven_overlay.dll", "heaven_version.dll"];

/// Generic proxy-loader DLL names. These are legitimate Windows system DLLs that
/// live in System32 — a *local copy in the game directory* is almost always a
/// third-party injector hijacking the game's import resolution. Lowercase.
const PROXY_LOADERS: &[&str] = &[
    "version.dll",
    "winhttp.dll",
    "dxgi.dll",
    "d3d11.dll",
    "d3d9.dll",
    "dinput8.dll",
    "xinput1_3.dll",
    "xinput1_4.dll",
    "opengl32.dll",
    "dsound.dll",
];

/// Files that belong to HachimiRedux or the game itself and must never be
/// flagged. Lowercase basenames.
const SAFE_FILES: &[&str] = &[
    "cri_mana_vpx.dll",
    "hachimi_training_tracker.dll",
    "unityplayer.dll",
    "gameassembly.dll",
    "baselib.dll",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictKind {
    /// A named third-party overlay/mod.
    KnownOverlay,
    /// A generic proxy-loader DLL present locally in the game directory.
    ProxyLoader,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetectedConflict {
    pub file_name: String,
    pub kind: ConflictKind,
}

/// Classify the given file names (basenames of files found in the game
/// directory) into the conflicts HachimiRedux knows about. Matching is
/// case-insensitive; duplicates are collapsed.
pub fn classify_conflicts(file_names: impl IntoIterator<Item = String>) -> Vec<DetectedConflict> {
    let mut out: Vec<DetectedConflict> = Vec::new();
    for name in file_names {
        let lower = name.to_ascii_lowercase();
        if SAFE_FILES.contains(&lower.as_str()) {
            continue;
        }
        let kind = if KNOWN_OVERLAYS.contains(&lower.as_str()) {
            ConflictKind::KnownOverlay
        } else if PROXY_LOADERS.contains(&lower.as_str()) {
            ConflictKind::ProxyLoader
        } else {
            continue;
        };
        if out.iter().any(|c| c.file_name.eq_ignore_ascii_case(&name)) {
            continue;
        }
        out.push(DetectedConflict { file_name: name, kind });
    }
    out
}

/// Whether a single file name (basename) is a known conflict. Used by the plugin
/// loader to enrich its "this isn't a HachimiRedux plugin" message.
pub fn is_known_conflict(file_name: &str) -> bool {
    let lower = file_name.to_ascii_lowercase();
    KNOWN_OVERLAYS.contains(&lower.as_str()) || PROXY_LOADERS.contains(&lower.as_str())
}

/// Enumerate the basenames of files directly inside `dir` and classify them.
/// Returns an empty vec if the directory cannot be read.
pub fn scan_dir(dir: &Path) -> Vec<DetectedConflict> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let names = entries
        .filter_map(Result::ok)
        .filter_map(|e| e.file_name().into_string().ok());
    classify_conflicts(names)
}

/// Run the one-time startup conflict scan over the game directory: log a warning
/// summary and stash it so the GUI can show it as a notification later. Safe to
/// call before the GUI exists. Idempotent (only the first call records anything).
pub fn run_startup_scan(game_dir: &Path) {
    let conflicts = scan_dir(game_dir);
    let summary = conflict_summary(&conflicts);
    if let Some(text) = &summary {
        warn!("{}", text);
    }
    let _ = STARTUP_SUMMARY.set(summary);
}

/// The summary recorded by [`run_startup_scan`], if any conflicts were found.
pub fn startup_summary() -> Option<String> {
    STARTUP_SUMMARY.get().cloned().flatten()
}

/// Build a human-readable, multi-line summary of detected conflicts suitable for
/// both the log and an in-game notification. Returns `None` when there are none.
pub fn conflict_summary(conflicts: &[DetectedConflict]) -> Option<String> {
    if conflicts.is_empty() {
        return None;
    }

    let mut names: Vec<&str> = conflicts.iter().map(|c| c.file_name.as_str()).collect();
    names.sort_unstable();

    let mut msg = String::from(
        "Other game mods / DLL injectors were found next to HachimiRedux. Stacking injectors \
         commonly crashes the game. If you have issues, keep only HachimiRedux (cri_mana_vpx.dll \
         and its plugins) and remove these:\n",
    );
    for name in names {
        msg.push_str("  - ");
        msg.push_str(name);
        msg.push('\n');
    }
    Some(msg)
}

#[cfg(test)]
#[allow(clippy::disallowed_methods)]
mod tests {
    use super::*;

    #[test]
    fn flags_named_overlays_and_proxies() {
        let files = vec![
            "cri_mana_vpx.dll".to_string(),
            "hachimi_training_tracker.dll".to_string(),
            "horseACT.dll".to_string(),
            "heaven_overlay.dll".to_string(),
            "version.dll".to_string(),
            "winhttp.dll".to_string(),
            "GameAssembly.dll".to_string(),
            "skill_grades.json".to_string(),
        ];
        let conflicts = classify_conflicts(files);
        let names: Vec<&str> = conflicts.iter().map(|c| c.file_name.as_str()).collect();

        assert!(names.contains(&"horseACT.dll"));
        assert!(names.contains(&"heaven_overlay.dll"));
        assert!(names.contains(&"version.dll"));
        assert!(names.contains(&"winhttp.dll"));
        // Our own DLLs and the game's are never flagged.
        assert!(!names.contains(&"cri_mana_vpx.dll"));
        assert!(!names.contains(&"hachimi_training_tracker.dll"));
        assert!(!names.contains(&"GameAssembly.dll"));
    }

    #[test]
    fn classifies_kind_correctly() {
        let conflicts = classify_conflicts(["horseACT.dll".to_string(), "dxgi.dll".to_string()]);
        let overlay = conflicts.iter().find(|c| c.file_name == "horseACT.dll").unwrap();
        let proxy = conflicts.iter().find(|c| c.file_name == "dxgi.dll").unwrap();
        assert_eq!(overlay.kind, ConflictKind::KnownOverlay);
        assert_eq!(proxy.kind, ConflictKind::ProxyLoader);
    }

    #[test]
    fn dedupes_case_insensitively() {
        let conflicts = classify_conflicts(["Version.dll".to_string(), "version.dll".to_string()]);
        assert_eq!(conflicts.len(), 1);
    }

    #[test]
    fn empty_summary_when_clean() {
        let conflicts = classify_conflicts(["cri_mana_vpx.dll".to_string()]);
        assert!(conflict_summary(&conflicts).is_none());
    }

    #[test]
    fn summary_lists_each_conflict() {
        let conflicts = classify_conflicts(["horseACT.dll".to_string(), "version.dll".to_string()]);
        let summary = conflict_summary(&conflicts).unwrap();
        assert!(summary.contains("horseACT.dll"));
        assert!(summary.contains("version.dll"));
    }

    #[test]
    fn is_known_conflict_matches_list() {
        assert!(is_known_conflict("horseACT.dll"));
        assert!(is_known_conflict("WINHTTP.DLL"));
        assert!(!is_known_conflict("cri_mana_vpx.dll"));
        assert!(!is_known_conflict("hachimi_training_tracker.dll"));
    }
}
