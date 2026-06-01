# Training Command Preview (Failure Rate + Stat Gain)

How the training-tracker plugin reads, **live and per-turn**, each training
facility's **failure rate** (%) and **total stat gain** — the numbers the game
only surfaces on hover. Powers the Training-tab columns for issues
`HachimiRedux-6cy` (Failure Rate Display) and `HachimiRedux-dsz` (Stat Gain
Preview).

Implemented in `plugins/training-tracker/src/memory_reader/command_info.rs`
(reader) + `snapshot.rs` (alignment + `CareerSnapshot.failure_rates/stat_gains`)
+ `ui.rs` (render).

---

## Live data path

The authoritative live source is the **working data** (not the server-response
proto). The proto `SingleModeHomeInfo` / `SingleModeCommandInfo` (plain arrays,
`failure_rate: Int32`, `params_inc_dec_info_array`) has **no stable live
reference** — after the server response is applied it is GC'd. The persistent
copy lives on the working-data objects:

```text
WorkSingleModeData
  .get_HomeInfo()            -> WorkSingleModeHomeInfo
    .get_TurnInfoListDic()   -> Dictionary<SingleModeDefine.CommandType,
                                            List<WorkSingleModeData.TurnInfo>>
      [CommandType.Training=1] -> List<TurnInfo>           // one per facility
        .get_CommandId()            -> TrainingCommandId   // facility command id
        .get_TrainingFailureRate()  -> Int32 (PLAIN %)     // 6cy
        .ParamIncDecInfoDic         -> Dictionary<ParameterType, ParamsIncDecInfo>
          [Speed=1 .. Wiz=5].Value (ObscuredInt)           // dsz, per-stat delta
```

### Verified signatures (against `il2cpp_classes.txt`)

| Member | Class | Returns | Args |
|--------|-------|---------|------|
| `get_HomeInfo` | `Gallop.WorkSingleModeData` | `WorkSingleModeHomeInfo` | 0 |
| `get_TurnInfoListDic` | `Gallop.WorkSingleModeHomeInfo` | `Dictionary<CommandType, List<TurnInfo>>` | 0 |
| `get_CommandId` | `WorkSingleModeData.TurnInfo` | `TrainingDefine.TrainingCommandId` | 0 |
| `get_TrainingFailureRate` | `WorkSingleModeData.TurnInfo` | `System.Int32` | 0 |
| `ParamIncDecInfoDic` (field) | `WorkSingleModeData.TurnInfo` | `Dictionary<ParameterType, ParamsIncDecInfo>` | — |
| `Value` (field) | `WorkSingleModeData.ParamsIncDecInfo` | `ObscuredInt` | — |

### Enum values

- `SingleModeDefine.CommandType`: `None=0, Training=1, EatMeal=2, Outing=3,
  RaceEntry=4, Camp=5, Holiday=6, Hospital=7`.
- `SingleModeDefine.ParameterType`: `None=0, Speed=1, Stamina=2, Power=3, Guts=4,
  Wiz=5, Hp=6, Motivation=7, SkillPoint=8`.

---

## Key decisions

- **No hooks needed.** Everything is reachable by reading the working data on the
  ~2s main-thread refresh (`overlay_cache::refresh_cache_cb`), folded into
  `read_snapshot`. Master-data-touching calls would crash on the render thread,
  but these getters do not — they were verified safe on the main thread.
- **Methods resolved from each object's runtime klass** (`resolve_obj_method`,
  reading the klass pointer at object offset 0) rather than pre-resolving the
  nested classes `WorkSingleModeData.TurnInfo` / `ParamsIncDecInfo`. This avoids
  fragile nested-class metadata lookups; the same pattern `read_list_field` uses.
- **Dictionary access via `TryGetValue(key, out)`**, never `get_Item` — the latter
  throws `KeyNotFoundException` on a missing key (e.g. a facility that grants no
  Stamina has no Stamina entry). `dict_try_get_obj` passes the enum key as `i32`
  and uses an over-sized out buffer so a small value-type `V` cannot corrupt the
  stack; only the first word (the object pointer) is read. `ParamsIncDecInfo` is a
  reference type, so this returns its object pointer directly.
- **`failure_rate` is a plain `Int32`** (the `<TrainingFailureRate>k__BackingField`
  is not obscured) — read directly, no XOR. The per-stat `Value` *is* an
  `ObscuredInt` and is decrypted via `read_obscured_int_field`.
- **Per-stat gain** = `ParamIncDecInfoDic[stat].Value` (base) **+
  `BonusParamIncDecInfoDic[stat].Value`** (bonus). Confirmed in-game on an Aoharu
  amplifier turn: `ParamsIncDecInfo.BonusValue` is **always 0**; the real preview
  bonus (support cards **and** scenario amplifiers — e.g. Aoharu spirit burst,
  Track Blazer tools) lives in the **separate `BonusParamIncDecInfoDic`**, which is
  zero on non-amplified turns and populates when a bonus is active. Summing
  `base + bonus` is therefore **scenario-agnostic** — no scenario detection needed.
  The reader sums all four components so any future non-zero `BonusValue` is still
  counted. See issue `HachimiRedux-23x`.
- **Total stat gain** = sum of per-stat gains over the 5 main stats (Speed..Wiz).
  Skill points / motivation / Hp deltas are intentionally excluded from the
  headline number (they are not "stats"); they can be surfaced separately later.

---

## Facility → stat spread (confirmed in-game)

Each facility raises several stats (plus Skill Points, excluded from the gain total).
Confirmed against live `base` values, array order `[Speed, Stamina, Power, Guts, Wit]`:

| Facility (cmd) | Stats raised |
|----------------|--------------|
| Speed (101)    | Speed + Power |
| Stamina (105)  | Stamina + Guts |
| Power (102)    | Power + Stamina |
| Guts (103)     | Guts + Power + Speed |
| Wit (106)      | Wit + Speed (and a large Skill-Point gain) |

## Facility alignment

`TurnInfo`s come back in list order; each carries its `command_id`. The pure
function `align_command_infos` (snapshot.rs, unit-tested) maps each onto a facility
slot `[Speed, Stamina, Power, Guts, Wisdom]` by matching `command_id` against the
known `COMMAND_ID_SETS` (URA `101/105/102/103/106`, Aoharu `601..605`, etc.).
Failure defaults to `-1` (unknown → UI shows `—`); gain defaults to `0`.

---

## Display

Training tab, under the stat-value row:
- **Gain row** — `+N` in light blue per facility (the dsz headline).
- **Failure row** — `N%` color-scaled green<20 / yellow<40 / orange<60 / red≥60
  (`failure_rate_color`, unit-tested).

---

## Status

**Verified in-game** (Classic Year, turn 3, URA set). Training tab showed correct
per-facility values: gains Speed `+18` / Stamina `+14` / Power `+10` / Guts `+14`
/ Wit `+9`, failure `0%` across all L1 facilities (86 energy) — aligned to the
right columns, color-scaled as designed. Gate-green (build + clippy `-D warnings`
+ fmt + 48 tests). Issues `6cy` + `dsz` closed.
