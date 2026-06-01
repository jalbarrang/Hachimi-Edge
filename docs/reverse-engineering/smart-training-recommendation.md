# Smart Training Recommendation

How the training-tracker plugin scores each training facility per turn and flags
the best one (`HachimiRedux-9ge`). Builds on the Tier-1 data features (stat-cap
warning `36o`, failure rate `6cy`, stat-gain preview `dsz`).

Implemented in `plugins/training-tracker/src/recommend.rs` (pure scoring + tests),
fed by `CareerSnapshot.per_stat_gains` / `failure_rates` / `stat_caps`, rendered in
`ui.rs::draw_training_tab`.

---

## Why a heuristic (not a read)

The game does not expose a "best training" recommendation. We synthesise one from
data we already read live, and from the validated 評価点 (evaluation-point) stat
curve (`crate::evaluation::stat_score`, exact to the point — see
[career-evaluation.md](career-evaluation.md)). Scoring by **projected 評価点 gain**
(rather than raw stat points) is what makes it "smart": the eval curve is nonlinear,
so a point at 1100 Speed is worth far more than a point at 200, and the recommender
follows that automatically.

## Model

Pure function `score_facilities(&Inputs) -> [FacilityScore; 5]`, indexed by facility
slot `[Speed, Stamina, Power, Guts, Wisdom]`.

```text
eval_delta = Σ_stat [ stat_score(min(cur + gain, ceiling)) − stat_score(cur) ]
   ceiling  = stat_targets::effective_threshold(target[s], cap[s])   (0 ⇒ no clamp)
p     = max(0, failure_rate) / 100
score = eval_delta × (1 − p)                          # expected value of the gains
if failure_rate > RISK_THRESHOLD_PCT (25):            # extra downside penalty
    loss  = Σ_trained_stat [ stat_score(cur) − stat_score(cur − 5) ] + MOOD_DROP_PENALTY
    score −= p × loss
best = argmax(score) over facilities with live data   # ★ in the Training tab
```

What the model captures:
- **Nonlinear eval curve** — high stats earn more, so it favours pushing your
  strongest stats (matching how the game's final score actually works).
- **Caps & targets** — gains past `min(target, cap)` add no eval, so a capped /
  target-hit stat sinks that facility. Ties directly into `36o`.
- **Failure risk** — base expected value discounts by failure %, and above 25% an
  extra penalty models the real cost of a failed training (loses ~5 stat points on
  the trained stats + a motivation-level drop).
- **Support cards implicitly** — the live `per_stat_gains` already bake in which
  support cards are on each facility (more/again rainbow cards ⇒ bigger gains), so
  no separate friendship term is needed for v1.

## Tunable constants (`recommend.rs`)

| Const | Value | Meaning |
|-------|-------|---------|
| `RISK_THRESHOLD_PCT` | 25 | Failure % above which the downside penalty applies |
| `FAILURE_STAT_LOSS` | 5 | Stat points lost on a failed training (per trained stat) |
| `MOOD_DROP_PENALTY` | 30 | Eval-pt cost of the mood-level drop on failure (rough guess) |

`MOOD_DROP_PENALTY` is a deliberate estimate — refine once we can compare against
real careers. Tests assert *shape* (monotonicity, clamping), not these magnitudes.

## Rest vs. Race fallback

When **every** facility with live data exceeds `ALL_RISKY_PCT` (30%) failure,
training is a bad turn. `turn_suggestion` then returns:
- **Rest** (recover energy) on most scenarios, or
- **Race** on scenarios that reward racing (`scenario_encourages_racing`, keyed on
  `get_ScenarioId` via `RACE_ENCOURAGED_SCENARIOS`).

The scenario is identified by its **Speed-slot training command id**
(`scenario_command_base`): 101 = URA Finale, 601 = Unity Cup (JP Aoharu), 1101 =
Trackblazer (Make a New Track). `RACE_ENCOURAGED_BASES = [1101]` — Trackblazer
rewards racing, so it suggests Race; URA and Unity Cup suggest Rest. Keying on the
directly-observed command set (rather than `get_ScenarioId`, whose values we have
not mapped) means no extra measurement was needed. `get_ScenarioId` is still read
and logged (`Command info: scenario_id=N …`) for cross-reference.

## Display

Training tab, a `★N` / `N` score row under the fail% row (green ★ on the single
best facility, weak grey otherwise, `—` when no live data), plus a caption naming
the best facility and its projected score.

## Known limitations (v1)

- **Greedy / per-turn.** It maximises this turn's risk-adjusted eval gain. It does
  **not** value building bonds early for later rainbow payoff, nor manage energy
  across turns, nor plan around target races. A future version could add a
  friendship-threshold term and an energy model.
- **SP excluded.** Skill points granted by training are ignored for v1 (Wit still
  scores via its stat gains). SP→skill→eval is indirect; revisit if Wit feels
  undervalued.

## Status

Gate-green (build + clippy `-D warnings` + fmt + 53 tests, incl. 5 recommend shape
tests). **In-game verification pending** — needs a live turn to confirm the ★ lands
on a sensible facility and the scores look reasonable.
