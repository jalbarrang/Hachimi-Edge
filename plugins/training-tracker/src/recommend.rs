//! Smart training recommendation: score each facility by its projected 評価点
//! (evaluation-point) gain this turn, failure-adjusted and clamped at the player's
//! per-stat targets/caps. Pure logic — no IL2CPP, safe on the render thread.
//!
//! Model (agreed with the user):
//! ```text
//! eval_delta = Σ_stat [ stat_score(min(cur + gain, ceiling)) − stat_score(cur) ]
//!   ceiling = effective_threshold(target, cap)   (0 ⇒ no clamp)
//! p = failure_rate / 100
//! score = eval_delta × (1 − p)                         # EV of the gains
//! if failure_rate > RISK_THRESHOLD_PCT:                # extra risk penalty
//!     loss = eval_cost_of_losing 5 pts on the trained stats + MOOD_DROP_PENALTY
//!     score −= p × loss
//! ```
//!
//! Known v1 limitation: greedy per-turn. It does not value building bonds early for
//! later rainbow payoff, nor cross-turn energy management.

use crate::evaluation::stat_score;
use crate::stat_targets;

/// Failure % above which the extra downside penalty applies.
const RISK_THRESHOLD_PCT: i32 = 25;
/// If EVERY available facility's failure % exceeds this, training is a bad turn —
/// suggest resting (or racing on race-encouraged scenarios) instead.
const ALL_RISKY_PCT: i32 = 30;

/// Scenario training-set bases (Speed-slot command id) where racing is the better
/// fallback when all trainings are too risky. Trackblazer (Make a New Track, base
/// 1101) rewards racing; URA (101) and Unity Cup (601) do not.
const RACE_ENCOURAGED_BASES: &[i32] = &[1101];

/// Whether the active scenario (identified by its Speed-slot command base) rewards
/// racing enough to prefer it over resting when every facility is too risky.
#[must_use]
pub fn scenario_encourages_racing(scenario_command_base: i32) -> bool {
    RACE_ENCOURAGED_BASES.contains(&scenario_command_base)
}

/// The overall suggestion for the turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurnSuggestion {
    /// Train the given facility slot (the best by projected score).
    Train(usize),
    /// All facilities are too risky — rest to recover energy.
    Rest,
    /// All facilities are too risky, but this scenario rewards racing.
    Race,
}

/// Decide the turn suggestion: if every facility with live data exceeds
/// [`ALL_RISKY_PCT`] failure, suggest Rest (or Race when `race_encouraged`);
/// otherwise train the best-scoring facility.
#[must_use]
pub fn turn_suggestion(scores: &[FacilityScore; 5], failure_rates: [i32; 5], race_encouraged: bool) -> TurnSuggestion {
    let known: Vec<usize> = (0..5).filter(|&i| scores[i].known).collect();
    let all_risky = !known.is_empty() && known.iter().all(|&i| failure_rates[i] > ALL_RISKY_PCT);
    if all_risky {
        return if race_encouraged {
            TurnSuggestion::Race
        } else {
            TurnSuggestion::Rest
        };
    }
    match scores.iter().position(|f| f.is_best) {
        Some(i) => TurnSuggestion::Train(i),
        None => TurnSuggestion::Rest,
    }
}
/// Stat points lost on a failed training (applied to the trained stats).
const FAILURE_STAT_LOSS: i32 = 5;
/// Eval-point cost of one motivation-level drop on failure. A rough, tunable guess
/// (failures lower mood, hurting all future gains) — refine against real careers.
const MOOD_DROP_PENALTY: i32 = 30;

/// Per-facility recommendation result.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct FacilityScore {
    /// Projected 評価点 gain this turn after failure/risk adjustment (can be negative).
    pub score: i32,
    /// `true` for the single best facility (highest score among known facilities).
    pub is_best: bool,
    /// Whether live command info was available for this facility this turn.
    pub known: bool,
}

/// Inputs for the recommendation, all per facility-slot [Speed, Stamina, Power,
/// Guts, Wisdom] (outer index = facility, matching the snapshot layout).
pub struct Inputs<'a> {
    /// Current stat values [Speed, Stamina, Power, Guts, Wisdom].
    pub current: [i32; 5],
    /// Per-facility, per-stat gain (facility × stat).
    pub per_stat_gains: &'a [[i32; 5]; 5],
    /// Per-stat caps [Speed..Wisdom]; `0` ⇒ unknown.
    pub caps: [i32; 5],
    /// Per-stat targets [Speed..Wisdom]; `0` ⇒ use cap.
    pub targets: [i32; 5],
    /// Per-facility failure %; `< 0` ⇒ unknown (treated as 0% for scoring).
    pub failure_rates: [i32; 5],
}

/// Score all five facilities and flag the best. Facilities with no live gain data
/// (all-zero row) are marked `known: false` and excluded from the best pick.
#[must_use]
pub fn score_facilities(input: &Inputs) -> [FacilityScore; 5] {
    let mut out = [FacilityScore::default(); 5];
    let mut best: Option<(usize, i32)> = None;

    for (i, slot) in out.iter_mut().enumerate() {
        let gains = input.per_stat_gains[i];
        let known = gains.iter().any(|&g| g != 0);
        let fail = input.failure_rates[i].max(0);
        let score = facility_score(input.current, gains, input.caps, input.targets, fail);
        *slot = FacilityScore {
            score,
            is_best: false,
            known,
        };
        if known && best.is_none_or(|(_, bs)| score > bs) {
            best = Some((i, score));
        }
    }

    if let Some((b, _)) = best {
        out[b].is_best = true;
    }
    out
}

/// Score a single facility: failure-adjusted projected 評価点 delta.
fn facility_score(current: [i32; 5], gains: [i32; 5], caps: [i32; 5], targets: [i32; 5], fail_pct: i32) -> i32 {
    let eval_delta = projected_eval_delta(current, gains, caps, targets);
    let p = fail_pct as f32 / 100.0;
    let mut score = eval_delta as f32 * (1.0 - p);

    if fail_pct > RISK_THRESHOLD_PCT {
        score -= p * failure_loss(current, gains) as f32;
    }
    score.round() as i32
}

/// Projected 評価点 gain from a facility's per-stat gains, clamping useful gains at
/// `min(target, cap)` (gains past the ceiling earn no evaluation).
fn projected_eval_delta(current: [i32; 5], gains: [i32; 5], caps: [i32; 5], targets: [i32; 5]) -> i32 {
    let mut delta = 0;
    for s in 0..5 {
        if gains[s] == 0 {
            continue;
        }
        let ceiling = stat_targets::effective_threshold(targets[s], caps[s]);
        let raised = current[s] + gains[s];
        let capped = if ceiling > 0 { raised.min(ceiling) } else { raised };
        delta += stat_score(capped) - stat_score(current[s]);
    }
    delta
}

/// Eval-point cost of a failed training: losing `FAILURE_STAT_LOSS` on each stat the
/// facility would have raised, plus the mood-drop penalty.
fn failure_loss(current: [i32; 5], gains: [i32; 5]) -> i32 {
    let mut loss = MOOD_DROP_PENALTY;
    for s in 0..5 {
        if gains[s] == 0 {
            continue;
        }
        let dropped = (current[s] - FAILURE_STAT_LOSS).max(0);
        loss += stat_score(current[s]) - stat_score(dropped);
    }
    loss
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one_facility(slot: usize, gains: [i32; 5]) -> [[i32; 5]; 5] {
        let mut m = [[0i32; 5]; 5];
        m[slot] = gains;
        m
    }

    #[test]
    fn unknown_facilities_are_not_best() {
        let input = Inputs {
            current: [100; 5],
            per_stat_gains: &[[0i32; 5]; 5],
            caps: [0; 5],
            targets: [0; 5],
            failure_rates: [-1; 5],
        };
        let out = score_facilities(&input);
        assert!(out.iter().all(|f| !f.known));
        assert!(out.iter().all(|f| !f.is_best));
    }

    #[test]
    fn best_is_highest_score() {
        // Speed facility gains 20 on Speed; Guts facility gains 20 on Guts but at 40% fail.
        let mut gains = [[0i32; 5]; 5];
        gains[0] = [20, 0, 0, 0, 0];
        gains[3] = [0, 0, 0, 20, 0];
        let input = Inputs {
            current: [300; 5],
            per_stat_gains: &gains,
            caps: [0; 5],
            targets: [0; 5],
            failure_rates: [0, -1, -1, 40, -1],
        };
        let out = score_facilities(&input);
        assert!(out[0].is_best, "safe Speed should beat risky Guts");
        assert!(out[0].score > out[3].score);
        assert!(out[3].known);
    }

    #[test]
    fn higher_failure_lowers_score() {
        let gains = one_facility(0, [20, 0, 0, 0, 0]);
        let mk = |fail: i32| {
            let input = Inputs {
                current: [300; 5],
                per_stat_gains: &gains,
                caps: [0; 5],
                targets: [0; 5],
                failure_rates: [fail, -1, -1, -1, -1],
            };
            score_facilities(&input)[0].score
        };
        assert!(mk(0) > mk(20));
        assert!(mk(20) > mk(50)); // risk penalty kicks in above 25%
    }

    #[test]
    fn gains_past_ceiling_earn_nothing() {
        // Stat at its target already → projected delta is ~0 regardless of gain.
        let gains = one_facility(0, [50, 0, 0, 0, 0]);
        let at_target = Inputs {
            current: [1200, 0, 0, 0, 0],
            per_stat_gains: &gains,
            caps: [1200, 0, 0, 0, 0],
            targets: [1200, 0, 0, 0, 0],
            failure_rates: [0, -1, -1, -1, -1],
        };
        assert_eq!(score_facilities(&at_target)[0].score, 0);

        // Same gain with headroom scores positive.
        let with_room = Inputs {
            current: [800, 0, 0, 0, 0],
            per_stat_gains: &gains,
            caps: [1200, 0, 0, 0, 0],
            targets: [0, 0, 0, 0, 0],
            failure_rates: [0, -1, -1, -1, -1],
        };
        assert!(score_facilities(&with_room)[0].score > 0);
    }

    #[test]
    fn suggest_rest_when_all_risky() {
        let mut gains = [[0i32; 5]; 5];
        for (i, g) in gains.iter_mut().enumerate() {
            g[i] = 10; // every facility raises one stat
        }
        let input = Inputs {
            current: [300; 5],
            per_stat_gains: &gains,
            caps: [0; 5],
            targets: [0; 5],
            failure_rates: [35, 40, 31, 50, 33], // all > 30%
        };
        let scores = score_facilities(&input);
        assert_eq!(
            turn_suggestion(&scores, input.failure_rates, false),
            TurnSuggestion::Rest
        );
        assert_eq!(
            turn_suggestion(&scores, input.failure_rates, true),
            TurnSuggestion::Race
        );
    }

    #[test]
    fn suggest_train_when_one_is_safe() {
        let mut gains = [[0i32; 5]; 5];
        for (i, g) in gains.iter_mut().enumerate() {
            g[i] = 10;
        }
        let input = Inputs {
            current: [300; 5],
            per_stat_gains: &gains,
            caps: [0; 5],
            targets: [0; 5],
            failure_rates: [35, 40, 5, 50, 33], // Power is safe
        };
        let scores = score_facilities(&input);
        assert_eq!(
            turn_suggestion(&scores, input.failure_rates, false),
            TurnSuggestion::Train(2)
        );
    }

    #[test]
    fn no_data_suggests_rest() {
        let input = Inputs {
            current: [300; 5],
            per_stat_gains: &[[0i32; 5]; 5],
            caps: [0; 5],
            targets: [0; 5],
            failure_rates: [-1; 5],
        };
        let scores = score_facilities(&input);
        assert_eq!(
            turn_suggestion(&scores, input.failure_rates, false),
            TurnSuggestion::Rest
        );
    }

    #[test]
    fn target_clamps_before_cap() {
        // Target 900 below cap 1200: gain past 900 is wasted even though under cap.
        let gains = one_facility(0, [40, 0, 0, 0, 0]);
        let capped_at_target = projected_eval_delta([880, 0, 0, 0, 0], gains[0], [1200, 0, 0, 0, 0], [900, 0, 0, 0, 0]);
        let capped_at_cap = projected_eval_delta([880, 0, 0, 0, 0], gains[0], [1200, 0, 0, 0, 0], [0, 0, 0, 0, 0]);
        assert!(capped_at_target < capped_at_cap);
    }
}
