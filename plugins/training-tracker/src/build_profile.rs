//! Build-target **profiles**: the desired end-state stat shape + objective the
//! recommender aims for. Sourced from curated **community presets** and a
//! **manual editor**. This replaces the old flat per-stat targets as the single
//! source of truth (`crate::stat_targets` is now a thin façade over the active
//! profile's `per_stat_target`).
//!
//! - The closed-form [`crate::cm_model`] supplies *threshold-aware marginal
//!   value* (survival floor, 1200 soft-cap, power knee). A profile says *what to
//!   aim at* (objective, targets, weights, course/strategy); the model says *how
//!   much each point is worth getting there*.
//! - Presets encode veteran wisdom per distance/strategy (uma.guide / gametora
//!   meta). Manual edits let power users override any field.
//!
//! In-memory state only; persistence lives in [`crate::config`]. Every persisted
//! field is `#[serde(default)]` so older configs migrate cleanly.

use std::sync::{Mutex, OnceLock};

use serde::{Deserialize, Serialize};

use crate::cm_model::Strategy;

/// Stat order: [Speed, Stamina, Power, Guts, Wit].
pub const STAT_LABELS: [&str; 5] = ["Speed", "Stamina", "Power", "Guts", "Wit"];

/// Upper bound for a per-stat target (matches the highest reachable stat cap).
pub const MAX_TARGET: i32 = 2000;

/// Which scoring objective the recommender optimizes for.
///
/// - `Rank` — the validated 評価点 (career-rank) model (legacy default).
/// - `Cm` — Champions Meeting race-utility (threshold-aware, via `cm_model`).
/// - `Hybrid(w)` — blend with `w` = CM weight in `0.0..=1.0`
///   (`0` ≡ Rank, `1` ≡ Cm).
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub enum Objective {
    /// Preserve the shipped behaviour until the user opts into CM.
    #[default]
    Rank,
    Cm,
    Hybrid(f32),
}

impl Objective {
    /// CM blend weight in `0.0..=1.0` (`Rank` ⇒ 0, `Cm` ⇒ 1, `Hybrid(w)` ⇒ w).
    pub fn cm_weight(self) -> f32 {
        match self {
            Objective::Rank => 0.0,
            Objective::Cm => 1.0,
            Objective::Hybrid(w) => w.clamp(0.0, 1.0),
        }
    }
}

/// A complete build target: objective + the stat shape and race context to aim
/// at. Switching profiles switches all of these together.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BuildProfile {
    /// Human-readable label (preset name or a user's custom name).
    pub name: String,
    /// Scoring objective.
    #[serde(default)]
    pub objective: Objective,
    /// Per-stat targets [Speed, Stamina, Power, Guts, Wit]; `0` ⇒ use the live
    /// game cap (keeps the old `stat_targets` semantics).
    #[serde(default)]
    pub per_stat_target: [i32; 5],
    /// Per-stat scoring weights (secondary tuning on top of the marginal model).
    #[serde(default = "default_weights")]
    pub stat_weights: [f32; 5],
    /// Intended running style for the target CM race (drives HP conversion etc.).
    #[serde(default = "default_strategy")]
    pub strategy: Strategy,
    /// Target CM course id (key into `crate::course_data`); `0` ⇒ none chosen.
    #[serde(default)]
    pub target_course_id: i32,
    /// Stamina rush-buffer override in points; `0` ⇒ auto (distance-derived).
    #[serde(default)]
    pub rush_buffer: i32,
    /// Free-form notes (preset provenance / user reminders).
    #[serde(default)]
    pub notes: String,
}

fn default_weights() -> [f32; 5] {
    [1.0; 5]
}

fn default_strategy() -> Strategy {
    Strategy::LateSurger
}

impl Default for BuildProfile {
    fn default() -> Self {
        // The default profile preserves shipped behaviour: Rank objective, no
        // targets (fall back to game caps), neutral weights.
        Self {
            name: "Default".to_owned(),
            objective: Objective::Rank,
            per_stat_target: [0; 5],
            stat_weights: default_weights(),
            strategy: default_strategy(),
            target_course_id: 0,
            rush_buffer: 0,
            notes: String::new(),
        }
    }
}

impl BuildProfile {
    /// Clamp every field to a sane range (targets `0..=MAX_TARGET`, weights
    /// `0..=5`, rush buffer `0..=600`, Hybrid weight `0..=1`).
    pub fn clamped(mut self) -> Self {
        for t in &mut self.per_stat_target {
            *t = (*t).clamp(0, MAX_TARGET);
        }
        for w in &mut self.stat_weights {
            *w = w.clamp(0.0, 5.0);
        }
        if let Objective::Hybrid(w) = self.objective {
            self.objective = Objective::Hybrid(w.clamp(0.0, 1.0));
        }
        self.rush_buffer = self.rush_buffer.clamp(0, 600);
        self
    }
}

// ---------------------------------------------------------------------------
// Community presets (curated veteran wisdom, in-code so no extra asset to ship)
// ---------------------------------------------------------------------------

/// Curated CM build presets keyed by distance bucket × strategy. Targets follow
/// the meta (uma.guide / gametora): Speed capped at 1200, Stamina scaled to the
/// survival floor for the distance/strategy (front styles need more), Power
/// ~850-1000, Wit high, Guts low. Weights nudge priority; the `cm_model`
/// marginal curve does the heavy lifting. Provenance lives in each `notes`.
pub fn presets() -> Vec<BuildProfile> {
    // [Speed, Stamina, Power, Guts, Wit]
    let mk = |name: &str, strategy: Strategy, t: [i32; 5], w: [f32; 5], notes: &str| BuildProfile {
        name: name.to_owned(),
        objective: Objective::Cm,
        per_stat_target: t,
        stat_weights: w,
        strategy,
        target_course_id: 0,
        rush_buffer: 0,
        notes: notes.to_owned(),
    };
    vec![
        mk(
            "Sprint ≤1400 • Front",
            Strategy::FrontRunner,
            [1200, 450, 1000, 350, 800],
            [1.0, 0.9, 1.0, 0.4, 0.8],
            "Short turf: cap Speed, heavy Power for the break, modest Stamina.",
        ),
        mk(
            "Mile ≤1800 • Pace",
            Strategy::PaceChaser,
            [1200, 600, 900, 350, 950],
            [1.0, 1.0, 0.9, 0.4, 0.9],
            "Mile: capped Speed + Power, Wit high for procs/position.",
        ),
        mk(
            "Mid 2000 • Late",
            Strategy::LateSurger,
            [1200, 800, 900, 350, 1000],
            [1.0, 1.0, 0.9, 0.4, 1.0],
            "Tokyo-2000 archetype: survival Stamina, max Wit, cap Speed.",
        ),
        mk(
            "Mid 2200-2400 • Late",
            Strategy::LateSurger,
            [1200, 950, 900, 400, 950],
            [1.0, 1.0, 0.9, 0.4, 0.95],
            "Longer mid: more Stamina for the spurt, still cap Speed/Power.",
        ),
        mk(
            "Long 2400+ • Late",
            Strategy::LateSurger,
            [1200, 1050, 850, 400, 900],
            [1.0, 1.0, 0.85, 0.45, 0.9],
            "Long: Stamina-first survival, Speed capped, Power eased.",
        ),
        mk(
            "Long 3000+ • End",
            Strategy::EndCloser,
            [1200, 1200, 800, 450, 850],
            [1.0, 1.0, 0.8, 0.5, 0.85],
            "Stayer: near-max Stamina, best HP conversion (oikomi), late kick.",
        ),
        mk(
            "Mid 2000 • Front",
            Strategy::FrontRunner,
            [1200, 900, 950, 400, 850],
            [1.0, 1.0, 0.95, 0.45, 0.85],
            "Front needs extra Stamina/Power to hold the lead + survive a rush.",
        ),
    ]
}

// ---------------------------------------------------------------------------
// Live state (active profile + saved custom profiles)
// ---------------------------------------------------------------------------

#[derive(Default)]
struct ProfileState {
    active: BuildProfile,
    saved: Vec<BuildProfile>,
}

fn state() -> &'static Mutex<ProfileState> {
    static S: OnceLock<Mutex<ProfileState>> = OnceLock::new();
    S.get_or_init(|| Mutex::new(ProfileState::default()))
}

/// The active build profile (cloned).
pub fn active() -> BuildProfile {
    state().lock().map(|s| s.active.clone()).unwrap_or_default()
}

/// Replace the active profile (clamped to sane ranges).
pub fn set_active(profile: BuildProfile) {
    if let Ok(mut s) = state().lock() {
        s.active = profile.clamped();
    }
}

/// The active objective.
pub fn objective() -> Objective {
    active().objective
}

/// The active profile's per-stat targets (`0` ⇒ use game cap).
pub fn per_stat_target() -> [i32; 5] {
    active().per_stat_target
}

/// Set the active profile's per-stat targets (clamped to `0..=MAX_TARGET`).
pub fn set_per_stat_target(targets: [i32; 5]) {
    if let Ok(mut s) = state().lock() {
        for (slot, v) in s.active.per_stat_target.iter_mut().zip(targets) {
            *slot = v.clamp(0, MAX_TARGET);
        }
    }
}

/// All saved custom profiles (cloned).
pub fn saved() -> Vec<BuildProfile> {
    state().lock().map(|s| s.saved.clone()).unwrap_or_default()
}

/// Replace the saved-profile list (clamped).
pub fn set_saved(profiles: Vec<BuildProfile>) {
    if let Ok(mut s) = state().lock() {
        s.saved = profiles.into_iter().map(BuildProfile::clamped).collect();
    }
}

/// Save the current active profile under `name` into the saved list (replacing
/// any existing entry with the same name).
pub fn save_active_as(name: &str) {
    if let Ok(mut s) = state().lock() {
        let mut p = s.active.clone();
        p.name = name.to_owned();
        s.saved.retain(|q| q.name != name);
        s.saved.push(p);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn objective_cm_weight_blends() {
        assert_eq!(Objective::Rank.cm_weight(), 0.0);
        assert_eq!(Objective::Cm.cm_weight(), 1.0);
        assert_eq!(Objective::Hybrid(0.3).cm_weight(), 0.3);
        assert_eq!(Objective::Hybrid(2.0).cm_weight(), 1.0); // clamped
    }

    #[test]
    fn default_profile_preserves_rank_behaviour() {
        let p = BuildProfile::default();
        assert_eq!(p.objective, Objective::Rank);
        assert_eq!(p.per_stat_target, [0; 5]);
        assert_eq!(p.stat_weights, [1.0; 5]);
    }

    #[test]
    fn clamped_bounds_every_field() {
        let p = BuildProfile {
            per_stat_target: [5000, -10, 1200, 0, 600],
            stat_weights: [9.0, -1.0, 1.0, 1.0, 1.0],
            objective: Objective::Hybrid(3.0),
            rush_buffer: 9000,
            ..Default::default()
        }
        .clamped();
        assert_eq!(p.per_stat_target[0], MAX_TARGET); // clamped down
        assert_eq!(p.per_stat_target[1], 0); // clamped up from negative
        assert_eq!(p.stat_weights[0], 5.0);
        assert_eq!(p.stat_weights[1], 0.0);
        assert_eq!(p.objective, Objective::Hybrid(1.0));
        assert_eq!(p.rush_buffer, 600);
    }

    #[test]
    fn presets_are_cm_with_capped_speed_and_growing_stamina() {
        let ps = presets();
        assert!(ps.len() >= 5);
        for p in &ps {
            assert_eq!(p.objective, Objective::Cm, "{} should be a CM preset", p.name);
            assert_eq!(p.per_stat_target[0], 1200, "{} should cap Speed", p.name);
            assert!(!p.notes.is_empty(), "{} should document provenance", p.name);
        }
        // Stamina target rises from sprint to stayer.
        let sprint = ps.iter().find(|p| p.name.contains("Sprint")).expect("sprint preset");
        let stayer = ps.iter().find(|p| p.name.contains("3000")).expect("stayer preset");
        assert!(stayer.per_stat_target[1] > sprint.per_stat_target[1]);
    }

    #[test]
    fn active_state_round_trips() {
        let p = BuildProfile {
            name: "Test".to_owned(),
            objective: Objective::Cm,
            per_stat_target: [1200, 800, 900, 300, 1000],
            ..Default::default()
        };
        set_active(p);
        assert_eq!(active().objective, Objective::Cm);
        assert_eq!(per_stat_target(), [1200, 800, 900, 300, 1000]);

        set_per_stat_target([1100, 700, 850, 250, 950]);
        assert_eq!(per_stat_target(), [1100, 700, 850, 250, 950]);

        save_active_as("Saved A");
        assert!(saved().iter().any(|q| q.name == "Saved A"));

        // Reset for other tests sharing global state.
        set_active(BuildProfile::default());
        set_saved(Vec::new());
    }
}
