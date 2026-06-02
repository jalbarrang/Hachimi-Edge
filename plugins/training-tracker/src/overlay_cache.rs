//! Unified cache for overlay career data.
//!
//! All IL2CPP reads run on the Unity main thread on a ~2s cadence (or immediately
//! when tracking starts). The render thread only clones from [`CACHE`].

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering as AtomicOrdering};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use hachimi_plugin_sdk::Sdk;

use crate::deck_bonuses;
use crate::memory_reader::{self, AcquiredSkillInfo, CareerSnapshot, EvaluationInfo};
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
}

static CACHE: Mutex<OverlayCache> = Mutex::new(OverlayCache {
    snapshot: None,
    skills: Vec::new(),
    evaluations: Vec::new(),
    skill_shop: Vec::new(),
    skill_points: None,
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

    // Deck bonuses: capture once when career starts, clear when it ends.
    let is_playing = snapshot.as_ref().is_some_and(|s| s.is_playing);
    if is_playing {
        if let Some(chara) = memory_reader::get_chara_ptr() {
            deck_bonuses::try_capture(chara);
        }
        // Self-computed evaluation estimate from stats + skills + aptitudes.
        if let Some(s) = snapshot.as_mut() {
            let stats = [s.speed, s.stamina, s.power, s.guts, s.wiz];
            s.evaluation_value = crate::evaluation::compute(stats, &s.aptitudes, s.star, &skills);
        }
    } else {
        deck_bonuses::clear();
    }

    if let Ok(mut guard) = CACHE.lock() {
        guard.snapshot = snapshot;
        guard.skills = skills;
        guard.evaluations = evaluations;
        guard.skill_shop = skill_shop;
        guard.skill_points = skill_points;
    }

    LAST_REFRESH_MS.store(now_ms(), AtomicOrdering::Relaxed);
    PENDING.store(false, AtomicOrdering::Release);
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
