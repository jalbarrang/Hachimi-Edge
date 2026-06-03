# Career Evaluation (評価点) and Final Training Rank

How the training-tracker plugin computes a trainee's **overall evaluation score**
(評価点, *hyōka-ten*) and its **rank** (G → SS+ → Ultimate → Legend) live during a
career — something the game itself does not expose mid-run.

Implemented in `plugins/training-tracker/src/{evaluation,eval_data,rank_table}.rs`.
Validated to **0.00% error** (exact, to the point) against two real, fully-raced,
mixed-aptitude careers (see [Validation](#validation)).

---

## Why this is computed, not read

There is **no live source** for the overall evaluation. A full field+method dump
(`il2cpp_classes.txt`, 14,832 `umamusume.dll` classes) confirms:

- `WorkSingleModeCharaData` (the live trainee model) exposes only `_evaluationList`
  (per-support **bond** values, 0–100) and `SkillPoint`. No overall score/rank field.
- The only `Int32 get_EvaluationValue(0 args)` in the entire game is on
  `SingleModeMainViewTrainingHorseIcon` — and that is the **support-horse bond
  gauge** (0–100), *not* the trainee's overall rating. (Verified in-game: it returned
  values like `charaId=1068 value=100` and `charaId=9002 value=64` for partner horses,
  never the trainee.)
- `RankScore` / `EvaluationPoint` exist only on **post-career** data
  (`WorkTrainedCharaData`) and team-stadium/team-race data — never the live single-mode
  chara.

The game computes the score at **career result** in native code. The in-career
Character Details screen shows stats/aptitudes/skills but **no score or rank**.
So a live rank must be **predicted** from the same inputs the game uses.

---

## The formula

```text
total = Σ stat_score(stat)          // 5 stats
      + Σ skill_score(skill)        // all non-unique acquired skills
      + unique_bonus

skill_score = round(gradeValue × aptitude_multiplier)
unique_bonus = uniqueSkillLevel × (170 if star ≥ 3 else 120)
```

Race wins/placements do **not** contribute (they drive *fans*, a separate quantity).
This was proven: two careers with 34–35 wins out of 35 races match to the point using
only `stat + skill + unique`.

### Stat score

Each of the 5 stats (clamped 0–2500) maps to evaluation points via the reconstructed
"umakonga" per-point curve (ported in `evaluation.rs::build_stat_table`). Three block
ranges with increasing per-point rates; the cumulative value is `round(raw / 10)`.
The port reproduces real careers exactly (see validation), so despite being a
reconstruction it is treated as ground truth.

### Skill score

Each non-unique acquired skill contributes `round(gradeValue × multiplier)`:

- **`gradeValue`** — the skill's base evaluation points. Authoritative, extracted from
  the game's `master.mdb` (`MasterSkillData.SkillData.GradeValue`).
- **multiplier** — depends on the skill's *aptitude role* and the trainee's grade for
  the matching aptitude line:

  | Aptitude grade | Bucket    | Multiplier |
  |----------------|-----------|-----------:|
  | S, A           | good      | 1.1        |
  | B, C           | average   | 0.9        |
  | D, E, F        | bad       | 0.8        |
  | G              | terrible  | 0.7        |
  | none / no role | base      | 1.0        |

  A skill's **role** (e.g. `front`, `mile`, `turf`) selects which aptitude line to read.
  Role → aptitude line:

  | Role keys                          | Aptitude line                                  |
  |------------------------------------|------------------------------------------------|
  | `turf`, `dirt`                     | surface                                        |
  | `sprint`, `mile`, `medium`, `long` | distance (Short/Mile/Middle/Long)              |
  | `front`, `pace`, `late`, `end`     | running style (Nige/Senko/Sashi/Oikomi)        |

  **Compound roles** (e.g. `sprint/front`): take the best multiplier *per category*
  (surface / distance / style), then multiply across categories.

Roles come from UmaTools' `affinity_role` annotation (the one piece not in `master.mdb`).

### Unique bonus

The trainee's own unique skill is **not** scored via its `gradeValue`. Instead:
`unique_bonus = uniqueSkillLevel × (170 if star ≥ 3 else 120)`.

- `uniqueSkillLevel` is read from the **acquired unique skill's level** (the resource
  flags uniques with `"u":1`, i.e. rarity ≥ 3). `get_TalentLevel()` is *not* used — it
  returns `ObscuredInt` (encrypted), which the plugin does not decrypt.
- `star` is the card rarity (1–5), from `get_CardRarityData().Rarity`.

---

## Rank mapping

The total maps to a badge tier via the threshold ladder in `rank_table.rs`
(`RANK_BADGES`, 298 entries, G → SS+ → Ultimate UG…US9 → Legend LG…LS24), sourced from
UmaTools' `RATING_BADGE_MINIMA`. `rank_label(value)` returns the highest tier whose
`min ≤ value`.

The labels match the game's `GameDefine.FinalTrainingRank` enum order, confirmed in the
dump:

```text
None, G, GPlus, F, FPlus, E, EPlus, D, DPlus, C, CPlus, B, BPlus,
A, APlus, S, SPlus, SS, SSPlus, UG, UG1…UG9, UF…, … LS24
```

`SingleModeDefine.GetTotalRank(value) -> FinalTrainingRank` is the game's own
value→rank mapper (static, master-data backed). We reproduce it with the table instead
of calling it, but the two agree.

---

## IL2CPP sources

**Live reads** (on `WorkSingleModeCharaData`, resolved by name — see
[il2cpp-signatures.md](il2cpp-signatures.md)):

| Datum            | Method / field                                              | Return       |
|------------------|-------------------------------------------------------------|--------------|
| Stats            | `get_Speed/Stamina/Power/Guts/Wiz`                          | `Int32`      |
| Aptitudes (×10)  | `get_ProperDistanceShort/Mile/Middle/Long`, `…RunningStyleNige/Senko/Sashi/Oikomi`, `…GroundTurf/Dirt` | `RaceDefine.ProperGrade` (Null=0, G=1 … S=8) |
| Star             | `get_CardRarityData()` → read `Rarity` field                | `Int32`      |
| Skills           | `_acquiredSkillList` → `get_MasterId` / `get_Level`         | `Int32`      |

**Bundled resource** (`gradeValue` + role + unique flag): see below.

---

## The skill-grade resource

`plugins/training-tracker/assets/skill_grades.json` — keyed by skill id:

```json
{ "200151": { "g": 174 }, "201242": { "g": 217, "r": "front" }, "100201": { "g": 340, "u": 1 } }
```

- `g` = `gradeValue` (base points)
- `r` = role key (lowercased; `"a/b"` for compound), omitted when none
- `u` = `1` for trainee uniques (rarity ≥ 3), omitted otherwise

Loaded **once at runtime** by `eval_data.rs` from the directory next to the plugin DLL
(the game folder). It is a **sidecar file, not bundled in the DLL** — so it can be
refreshed per game version without rebuilding.

### Regenerating (per game update)

```sh
# 1. fetch the latest global master.mdb -> db/master.mdb (gitignored)
cargo run -p fetch-master-db
# 2. join master.mdb + UmaTools CSV -> plugins/training-tracker/assets/skill_grades.json
cargo run -p skill-grades
```

Both are Rust workspace tools (no Node / uma-sim dependency). `skill-grades` joins
two sources by skill id ↔ name:

1. `grade_value` (+ rarity, English name via `text_data` category 47) per skill —
   read directly from the game's `master.mdb` with rusqlite.
2. `affinity_role` per skill — fetched from UmaTools'
   [`assets/uma_skills.csv`](https://github.com/daftuyda/UmaTools) (override with `--csv`).

`fetch-master-db` ports the Honse game CDN manifest chain (resolves the resource
version from `uma.moe/api/ver`); see [docs/gametora-data.md](../gametora-data.md)
for the sibling data pipeline.

The deploy script (`scripts/deploy-windows.ps1 -Build`) copies the JSON to the game
folder alongside the DLLs.

---

## Validation

Two real, fully-raced careers with mixed aptitudes (`veterans/*.json`), comparing the
computed total to the game's displayed 評価点:

| Runner       | Star | Unique Lv | Races/Wins | Computed | Game   | Error |
|--------------|-----:|----------:|------------|---------:|-------:|------:|
| Seiun Sky    | 4    | 5         | 35 / 34    | 18,535   | 18,535 | 0     |
| Mejiro Ryan  | 5    | 6         | 35 / 35    | 17,527   | 17,527 | 0     |

Both **exact**. (An initial −49 on Seiun Sky was a data-entry typo in the test JSON —
`Front Runner Corners ○` g=217 vs the real `◎` g=262 — not a formula error.)

These fixtures live in `veterans/*.json` and are enforced by the
`evaluation::tests::validated_runners_match_exactly` unit test, which runs each one
through `compute_with` against the real `skill_grades.json` resource and asserts an exact
match. Add a new `veterans/*.json` (stats, star, `uniqueLevel`, per-line `aptitudes`,
`skills`, and ground-truth `evaluationScore`) to extend coverage.

The maxed-win careers matching to the point also confirm **race results do not feed the
evaluation score**.

### Residual error

The only way to be wrong is **resource coverage**: a skill not present in
`skill_grades.json` (e.g. a newly added skill) is skipped → the total undercounts.
The fix is regenerating the resource from an updated `master.mdb`. The formula itself is
exact for covered skills.

---

## Surface

`CareerSnapshot.evaluation_value: Option<i32>` is computed on the main-thread refresh
(`overlay_cache::refresh_cache_cb`) and rendered in the overlay's Training tab as
`Rank: <label> • <points>` (`—` when the resource is missing).
