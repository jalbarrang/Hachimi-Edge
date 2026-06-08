# Smart Training Recommendation

How the training-tracker plugin scores each training facility per turn and flags
the best one (`HachimiRedux-9ge`), and how that scorer was extended from a single
**Rank (評価点)** heuristic into a configurable, **Champions-Meeting-aware**
recommender with a multi-turn planning layer.

Builds on the Tier-1 data features (stat-cap warning `36o`, failure rate `6cy`,
stat-gain preview `dsz`) and the validated evaluation curve
([career-evaluation.md](career-evaluation.md)).

---

## Overview

The recommender is a stack of **pure, deterministic, render-thread-safe** modules.
Each turn the UI assembles live data + the user's build profile, runs the scorer,
then the multi-turn planner, and renders the result:

```text
CareerSnapshot ┐
build_profile  ├─▶ recommend::score_facilities  ─▶ planner::adjust_scores ─▶ ★ + caption
course_data    ┘   (objective-aware per-turn)      (energy/bond/phase layer)
```

| Module | Role |
|--------|------|
| `recommend.rs` | Per-turn objective scoring (Rank / CM / Hybrid) + failure-risk EV + Rest/Race fallback |
| `cm_model.rs` | Closed-form CM race-utility: marginal value of one stat point (survival floor, soft cap, power knee) |
| `course_data.rs` | Per-course params (distance/surface/set-status) loaded from `course_params.json` |
| `build_profile.rs` | The build target: objective + per-stat targets/weights + course/strategy + presets |
| `planner.rs` | Multi-turn layer: energy/rest option-value, bond/rainbow lookahead, career-phase weighting |
| `evaluation.rs` | The 評価点 stat curve (`stat_score`), used by the Rank objective |

---

## Objectives

The active build profile (`build_profile::Objective`) selects what the scorer
optimizes for:

- **Rank (評価点)** — the original, validated model. Scores by **projected
  evaluation-point gain**, which the nonlinear `stat_score` curve makes "smart":
  a point at 1100 Speed is worth far more than at 200, so it pushes your strongest
  stats — exactly how the game's final career score works.
- **CM (race power)** — scores by **marginal race-winning utility** toward the CM
  build target (`cm_model`), *not* raw rank. This rewards the stats that actually
  win the race for your course + strategy, and stops over-investing in stats that
  only inflate 評価点.
- **Hybrid(b)** — a normalized blend `b·CM + (1−b)·Rank` (`b` ∈ 0..1).

### Per-turn model (`recommend::score_facilities`)

Pure function, indexed by facility slot `[Speed, Stamina, Power, Guts, Wit]`:

```text
delta = objective_delta(objective, current, gains, ctx)     # 評価点-equivalent units
p     = max(0, failure_rate) / 100
score = delta × (1 − p)                                      # expected value of the gains
if failure_rate > RISK_THRESHOLD_PCT (25):                  # extra downside penalty
    loss  = Σ_trained_stat [ stat_score(cur) − stat_score(cur − 5) ] + MOOD_DROP_PENALTY
    score −= p × loss
best  = argmax(score) over facilities with live data         # ★ in the Training tab
```

`objective_delta` dispatches on the objective:

- **Rank** → `Σ_stat [ stat_score(min(cur+gain, ceiling)) − stat_score(cur) ]`,
  ceiling = `effective_threshold(target, cap)`.
- **CM** → `Σ_stat cm_model::stat_marginal_value(stat, current, course, strategy,
  apt) × useful_gain × stat_weight`, where `useful_gain` is still capped at the
  manual target/cap ceiling (gains past it earn nothing), then scaled by
  `CM_EVAL_SCALE` into 評価点-equivalent units.
- **Hybrid(b)** → `b·CM + (1−b)·Rank` (both already in the same unit).

**Unit normalization.** CM marginal value is in "uutil" (utility units;
≈`1000 × m/s of effective race speed per point`). `CM_EVAL_SCALE` (≈4) converts a
facility's summed uutil into 評価点-equivalent points so that (a) the failure/risk
EV model — denominated in eval points — applies uniformly to every objective, and
(b) the Hybrid blend mixes comparable quantities. Tests assert *shape* (knees,
ordering, monotonicity), not these magnitudes.

**Graceful fallback.** If a CM/Hybrid objective is selected but no target course is
available (`course_data` missing the id, or none configured), `effective_objective`
degrades to **Rank** and `cm_fallback_active` flags it for the UI — the scorer
never panics for want of course data.

---

## CM race-utility model (`cm_model.rs`)

Closed-form, self-contained (no IL2CPP, no cross-repo dependency) answers to:
*given a target CM course + strategy + the trainee's current stats, what is the
race value of one more point of stat X?* Ported from the Torena/uma-sim race engine
(`../uma-sim/packages/uma-sim-primitives`, itself a port of umasim) and grounded in
the community meta (gametora race-mechanics, uma.guide). Parity with the reference
engine is asserted where exact anchors exist (`max_hp`, `base_speed`, set-status
multiplier, strategy coefficients).

`stat_marginal_value` is **threshold-aware** — it bakes in the non-linearities that
make a CM build different from a rank build:

| Stat | Behaviour |
|------|-----------|
| **Speed** | Last-spurt derivative (exact); the principled backbone. Halves above the 1200 soft cap. |
| **Stamina** | High **below** the survival floor, ~0 above (smooth knee). Crossing the floor unlocks the full last spurt, so deficient stamina dominates. |
| **Power** | Acceleration derivative, tapering past a course-tuned **knee** (longer/dirt courses want more power). |
| **Guts** | Minor: small last-spurt + HP-saving term, slightly larger for short/front. |
| **Wit** | Gentle, diminishing, **never zero** (no soft cap — skill-proc consistency). |

The **stamina survival threshold** is the dominant CM non-linearity: a conservative
closed-form estimate of the stamina needed to sustain a full max last-spurt for the
course/strategy, plus a distance-scaled **rush buffer** (掛かり reserve). Below it
the trainee gasses out and cannot spurt; above it, extra stamina is mostly wasted.

---

## Build profiles & presets (`build_profile.rs`)

A **profile** is the single source of truth for *what to aim at*: objective,
`per_stat_target[5]` (0 ⇒ use the live game cap), `stat_weights[5]`, `strategy`,
`target_course_id`, and notes. `cm_model` says *how much each point is worth*; the
profile says *where you're headed*. `stat_targets.rs` is a thin façade over the
active profile's targets so the existing cap/target warning keeps working.

Seven curated **presets** encode veteran wisdom per distance bucket × strategy
(uma.guide / gametora meta): Speed capped at 1200, Stamina scaled to the survival
floor (front styles need more), Power ~850–1000, Wit high, Guts low. Users can load
a preset, edit any field, and **save/rename** custom profiles. Everything persists
in `training_config.json` (every field `#[serde(default)]` for clean migration; the
legacy flat `stat_targets` field migrates into the default profile).

### Course-data pipeline (`course_data.rs` + `tools/course-data`)

The CM math needs per-course parameters (distance, surface, set-status stat
thresholds). These are generated offline by `cargo run -p course-data` from
master.mdb (`race_course_set` + `race_course_set_status`) into
`assets/course_params.json` (107 courses), shipped as a sidecar next to the plugin
DLL so it can be refreshed per game version without rebuilding. `course_data` loads
it lazily and exposes `course_params(id)`, plus `course_label(id)` /
`all_courses()` for the picker (racetrack name decoded from the `id / 100` track
prefix: 101 Sapporo … 110 Kokura, 111 Ooi).

---

## Multi-turn planning layer (`planner.rs`)

The per-turn scorer is greedy. The planner lifts it toward the *trajectory* to the
build target with three terms, all faded by a single **influence** scalar
(`aggressiveness × depth/CAP`) so `lookahead_depth == 0` reproduces the greedy
result exactly. No game simulation — a discounted closed-form heuristic, not an
N-turn rollout, to stay within the render-thread budget.

1. **Energy / rest option-value** — when HP is low (and failures cascade), resting
   preserves the turns you have left. Surfaced as a fatigue discount on training
   value plus a **rest override** on the turn suggestion (flips `Train → Rest`, or
   `Race` on race-encouraged scenarios). Scaled by failure trajectory, motivation
   (low mood adds rest pull), and turns remaining. Uses live `hp/max_hp/motivation`.
2. **Bond / rainbow lookahead** — training a support that is *near* the friendship
   (rainbow) threshold now pays off over future turns. Modelled as an
   early-career-weighted per-facility uplift (`near_rainbow_pressure`). The
   per-facility pressure is read **live**: `memory_reader::command_info` walks each
   facility's `TurnInfo.TrainingHorseList`, reads every present non-guest support's
   bond gauge (`TrainingHorse.GetEvaluation().get_Value()`), maps each through
   `near_rainbow_pressure`, and combines them as a soft-OR. It degrades to greedy
   when no partner is near the threshold.
3. **Career-phase weighting** — late in the career, facilities that close a
   distance-from-target gap are boosted (`stat_deficits`), shifting the plan from
   bond-building early to stat-maxing late.

Knobs (`PlannerParams`: depth / aggressiveness / energy-floor %) are user-tunable in
the L1 settings page and persisted. Conservative defaults keep the greedy pick at
full HP largely intact.

---

## Rest vs. Race fallback

When **every** facility with live data exceeds `ALL_RISKY_PCT` (30%) failure,
training is a bad turn. `turn_suggestion` returns **Rest**, or **Race** on scenarios
that reward racing (`scenario_encourages_racing`, keyed on the Speed-slot training
command base: 101 URA, 601 Unity Cup, 1101 Trackblazer; `RACE_ENCOURAGED_BASES =
[1101]`). The planner's energy override can additionally trigger Rest/Race at low HP
even when not every facility is risky. This objective-agnostic fallback is reused
unchanged across all objectives.

---

## UI

**Settings (L1 menu, `ui/menu.rs`)**
- **Build Profile**: objective selector (Rank | CM | Hybrid + blend slider), CM
  course picker + strategy, live **survival advisory** (`Stamina ≈ N for max spurt
  + rush buffer • Speed soft cap • Power knee`), per-stat **targets + weights**
  editor, preset chooser, and save/rename of custom profiles.
- **Smart Recommendation**: risk/penalty sliders (`RISK_THRESHOLD_PCT`,
  `ALL_RISKY_PCT`, `MOOD_DROP_PENALTY`, `FAILURE_STAT_LOSS`).
- **Multi-turn Planning**: lookahead depth / aggressiveness / energy-floor sliders.

**Overlay (Training tab, `ui/training/`)**
- A `★N` / `N` score row under the fail% row (green ★ on the single best facility),
  and a caption naming the **active objective** — `★ best (CM): Power — …`, or
  `(Rank — no CM course)` when CM falls back.
- A compact, color-coded **CM status line** (only under a CM objective): stamina
  *have vs survival need*, speed-to-soft-cap, power-to-knee.

## Tunable constants

| Const | Where | Meaning |
|-------|-------|---------|
| `RISK_THRESHOLD_PCT` (25) | `recommend` | Failure % above which the downside penalty applies |
| `FAILURE_STAT_LOSS` (5) | `recommend` | Stat points lost on a failed training |
| `MOOD_DROP_PENALTY` (30) | `recommend` | Eval-pt cost of the mood-level drop on failure (estimate) |
| `CM_EVAL_SCALE` (≈4) | `recommend` | uutil → 評価点-equivalent normalization |
| `CAREER_TURNS_TOTAL` (78) | `planner` | Approx career length, for relative phase weighting |
| `RAINBOW_BOND_THRESHOLD` (80) | `planner` | Bond value that unlocks friendship training |
| `MAX_LOOKAHEAD_DEPTH` (4) | `planner` | Hard cap on the lookahead knob |

## Known limitations

- **SP excluded.** Skill points granted by training are not modelled directly (Wit
  still scores via its stat gains / no-soft-cap term). SP→skill→eval is indirect.
- **Shallow lookahead.** The planner is a discounted closed-form heuristic, not a
  true multi-turn search.

## Status

Gate-green (build + clippy `-D warnings` + fmt + full lib test suite, including
recommend/cm_model/planner/course_data shape tests). **In-game verification
pending** for the CM objective — confirm the ★ lands sensibly, the survival line
reads right, and presets load.
