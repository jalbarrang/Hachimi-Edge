//! Unified cache for overlay career data.
//!
//! All IL2CPP reads run on the Unity main thread on a ~2s cadence (or immediately
//! when tracking starts). The render thread only clones from [`CACHE`].

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering as AtomicOrdering};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use hachimi_plugin_sdk::Sdk;

use crate::deck_bonuses;
use crate::memory_reader::{self, AcquiredSkillInfo, CareerSnapshot, EvaluationInfo, FiredEvent};
use crate::skill_shop::{self, SkillShopEntry};

/// Auto-refresh interval while memory tracking is on (milliseconds).
pub const AUTO_REFRESH_INTERVAL_MS: u64 = 2000;

#[derive(Default)]
struct OverlayCache {
    snapshot: Option<CareerSnapshot>,
    skills: Vec<AcquiredSkillInfo>,
    evaluations: Vec<EvaluationInfo>,
    skill_shop: Vec<SkillShopEntry>,
    skill_points: Option<i32>,
    /// Equipped `(deck slot, support_card_id)` map, captured once per career.
    support_ids: Vec<(i32, i32)>,
}

static CACHE: Mutex<OverlayCache> = Mutex::new(OverlayCache {
    snapshot: None,
    skills: Vec::new(),
    evaluations: Vec::new(),
    skill_shop: Vec::new(),
    skill_points: None,
    support_ids: Vec::new(),
});
static PENDING: AtomicBool = AtomicBool::new(false);
static LAST_REFRESH_MS: AtomicU64 = AtomicU64::new(0);
static SHUTTING_DOWN: AtomicBool = AtomicBool::new(false);

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Whether an auto-refresh should be requested (pure logic for tests).
#[must_use]
pub fn should_auto_refresh(elapsed_ms: u64, interval_ms: u64) -> bool {
    elapsed_ms >= interval_ms
}

fn elapsed_since_last_refresh_ms() -> u64 {
    let last = LAST_REFRESH_MS.load(AtomicOrdering::Relaxed);
    if last == 0 {
        return u64::MAX;
    }
    now_ms().saturating_sub(last)
}

fn schedule_refresh() {
    if SHUTTING_DOWN.load(AtomicOrdering::Acquire) {
        return;
    }
    if PENDING.swap(true, AtomicOrdering::AcqRel) {
        return;
    }
    Sdk::get().schedule_on_main_thread(refresh_cache_cb);
}

extern "C" fn refresh_cache_cb() {
    if SHUTTING_DOWN.load(AtomicOrdering::Acquire) {
        PENDING.store(false, AtomicOrdering::Release);
        return;
    }
    let mut snapshot = memory_reader::read_snapshot();
    let skills = memory_reader::read_acquired_skills();
    let evaluations = memory_reader::read_evaluations();
    let skill_points = skill_shop::read_skill_points();
    let skill_shop = skill_shop::read_skill_shop();

    let is_playing = snapshot.as_ref().is_some_and(|s| s.is_playing);
    // Equipped support-card ids: re-read every refresh (pure ObscuredInt field reads,
    // no Convert). Cheap, and avoids stale deck mapping when the game keeps SingleMode
    // "playing" across a career -> new-career transition.
    let support_ids = if is_playing {
        memory_reader::read_equipped_support_ids()
    } else {
        Vec::new()
    };
    // Deck change (new career / reshuffled deck) invalidates per-career progress and
    // the once-per-career deck-bonus capture. Detect by comparing to the prior deck.
    if is_playing {
        let prev = CACHE.lock().ok().map(|g| g.support_ids.clone()).unwrap_or_default();
        // Require both non-empty so a transient empty read can't wipe progress mid-career.
        if !prev.is_empty() && !support_ids.is_empty() && prev != support_ids {
            crate::bond_progress::clear();
            deck_bonuses::clear(); // re-captured below via try_capture
            EVAL_DIAG_LOGGED.store(false, AtomicOrdering::Relaxed);
        }
    }
    // Fired-event history: re-read each refresh (read-only; grows over the career).
    let fired_events = if is_playing {
        memory_reader::read_fired_events()
    } else {
        Vec::new()
    };
    // Accumulate observed events into per-career progress (auto counter).
    if is_playing {
        crate::bond_progress::observe(&support_ids, &fired_events);
    } else {
        crate::bond_progress::clear();
    }
    if is_playing {
        if let Some(chara) = memory_reader::get_chara_ptr() {
            deck_bonuses::try_capture(chara);
        }
        // Self-computed evaluation estimate from stats + skills + aptitudes.
        if let Some(s) = snapshot.as_mut() {
            let stats = [s.speed, s.stamina, s.power, s.guts, s.wiz];
            s.evaluation_value = crate::evaluation::compute(stats, &s.aptitudes, s.star, &skills);
        }
        log_career_diagnostic(&evaluations, &support_ids, &fired_events);
    } else {
        deck_bonuses::clear();
        EVAL_DIAG_LOGGED.store(false, AtomicOrdering::Relaxed);
    }

    if let Ok(mut guard) = CACHE.lock() {
        guard.snapshot = snapshot;
        guard.skills = skills;
        guard.evaluations = evaluations;
        guard.skill_shop = skill_shop;
        guard.skill_points = skill_points;
        guard.support_ids = support_ids;
    }

    LAST_REFRESH_MS.store(now_ms(), AtomicOrdering::Relaxed);
    PENDING.store(false, AtomicOrdering::Release);
}

/// One-shot per career: dump the (safe, already-read) evaluation rows so the
/// `target_id` (deck slot 1–6 / guest) ↔ `story_step` relationship can be correlated
/// against a known deck. Evaluation-only — touches no support-card/deck memory.
static EVAL_DIAG_LOGGED: AtomicBool = AtomicBool::new(false);

fn log_career_diagnostic(evaluations: &[EvaluationInfo], support_ids: &[(i32, i32)], fired: &[FiredEvent]) {
    if evaluations.is_empty() || EVAL_DIAG_LOGGED.swap(true, AtomicOrdering::Relaxed) {
        return;
    }
    hlog_info!(target: "training-tracker", "Eval diagnostic ({} rows):", evaluations.len());
    for e in evaluations {
        hlog_info!(
            target: "training-tracker",
            "  target_id={} value={} story_step={} guest_chara_id={} is_appear={} name={:?}",
            e.target_id, e.value, e.story_step, e.guest_chara_id, e.is_appear, e.name
        );
    }
    // Probe the master evaluation table to learn target_id -> chara_id mapping.
    let target_ids: Vec<i32> = evaluations.iter().map(|e| e.target_id).collect();
    memory_reader::probe_eval_master(&target_ids);

    // Fired-event history sample (to compare ids against catalog chain keys).
    let ev_ids: std::collections::HashSet<i32> = fired.iter().map(|e| e.event_id).collect();
    let st_ids: std::collections::HashSet<i32> = fired.iter().map(|e| e.story_id).collect();
    hlog_info!(target: "training-tracker", "Fired events: {} total", fired.len());
    for e in fired.iter().take(12) {
        hlog_info!(target: "training-tracker", "  event_id={} story_id={}", e.event_id, e.story_id);
    }

    hlog_info!(target: "training-tracker", "Deck map ({} slots):", support_ids.len());
    for (slot, support_id) in support_ids {
        let name = crate::gametora_data::support_card_name(*support_id as i64).unwrap_or("?");
        let max = crate::gametora_data::max_chain_steps(*support_id as i64);
        let keys = crate::gametora_data::chain_event_keys(*support_id as i64);
        let matched = keys
            .iter()
            .filter(|(eid, sid)| {
                (*eid != 0 && ev_ids.contains(&(*eid as i32))) || (*sid != 0 && st_ids.contains(&(*sid as i32)))
            })
            .count();
        let sample: Vec<(i64, i64)> = keys.iter().take(3).copied().collect();
        hlog_info!(
            target: "training-tracker",
            "  slot={} support_id={} name={:?} max={:?} chain_keys={} matched={} keys_sample={:?}",
            slot, support_id, name, max, keys.len(), matched, sample
        );
    }
}

/// Throttled auto-refresh (call from render thread each overlay frame).
pub fn maybe_request_refresh() {
    if !memory_reader::TRACKING.load(AtomicOrdering::Relaxed) {
        return;
    }
    if !should_auto_refresh(elapsed_since_last_refresh_ms(), AUTO_REFRESH_INTERVAL_MS) {
        return;
    }
    schedule_refresh();
}

/// Immediate refresh when tracking starts — bypasses interval, still coalesced.
pub fn request_refresh_immediate() {
    if !memory_reader::TRACKING.load(AtomicOrdering::Relaxed) {
        return;
    }
    schedule_refresh();
}

pub fn snapshot() -> Option<CareerSnapshot> {
    CACHE.lock().ok().and_then(|g| g.snapshot.clone())
}

pub fn skills() -> Vec<AcquiredSkillInfo> {
    CACHE.lock().ok().map(|g| g.skills.clone()).unwrap_or_default()
}

pub fn evaluations() -> Vec<EvaluationInfo> {
    CACHE.lock().ok().map(|g| g.evaluations.clone()).unwrap_or_default()
}

/// Equipped `(deck slot, support_card_id)` pairs for the active career.
pub fn equipped_support_ids() -> Vec<(i32, i32)> {
    CACHE.lock().ok().map(|g| g.support_ids.clone()).unwrap_or_default()
}

pub fn skill_shop() -> Vec<SkillShopEntry> {
    CACHE.lock().ok().map(|g| g.skill_shop.clone()).unwrap_or_default()
}

pub fn skill_points() -> Option<i32> {
    CACHE.lock().ok().and_then(|g| g.skill_points)
}

/// Stop scheduling refreshes and bail out of any in-flight main-thread callback.
/// Call from the plugin `SHUTDOWN` handler before the host frees the DLL.
pub fn shutdown() {
    SHUTTING_DOWN.store(true, AtomicOrdering::Release);
    PENDING.store(false, AtomicOrdering::Release);
    LAST_REFRESH_MS.store(0, AtomicOrdering::Release);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_auto_refresh_respects_interval() {
        assert!(!should_auto_refresh(0, AUTO_REFRESH_INTERVAL_MS));
        assert!(!should_auto_refresh(1999, AUTO_REFRESH_INTERVAL_MS));
        assert!(should_auto_refresh(2000, AUTO_REFRESH_INTERVAL_MS));
        assert!(should_auto_refresh(3000, AUTO_REFRESH_INTERVAL_MS));
    }

    #[test]
    fn shutdown_blocks_refresh_scheduling() {
        shutdown();
        schedule_refresh();
        assert!(!PENDING.load(AtomicOrdering::Relaxed));
    }
}
