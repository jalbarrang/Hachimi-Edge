# Single Mode (Career) Architecture

## Overview

Single Mode is the core career/training gameplay loop. The player raises a character over a fixed number of turns, choosing training facilities, races, rest, or outings each turn. The game processes each turn as a client-server round-trip using MessagePack-encoded requests/responses.

## Lifecycle

```
┌─────────────────┐
│  SingleModeStart │  ← Player selects character, support cards, inheritance
└────────┬────────┘
         │
         ▼
┌─────────────────────────┐
│  SingleModeCheckEvent    │  ← Server returns turn state: available commands,
│  (each turn)             │     events, support card positions, stat preview
└────────┬────────────────┘
         │
         ▼
┌─────────────────────────┐
│  Player chooses action   │  ← Training / Race / Rest / Outing / Skill Learn
│  (UI interaction)        │
└────────┬────────────────┘
         │
         ▼
┌─────────────────────────┐
│  SingleModeExecCommand   │  ← Client sends command_type + command_id
│  (request → response)    │     Server returns stat changes, events, etc.
└────────┬────────────────┘
         │
         ▼
┌─────────────────────────┐
│  SingleModeCheckEvent    │  ← Next turn begins
│  (repeat until final)    │
└────────┬────────────────┘
         │
         ▼
┌─────────────────────────┐
│  SingleModeFinish        │  ← Career ends, results screen
└─────────────────────────┘
```

## Key Controller Classes

### `SingleModeMainViewController`

The primary view controller for career mode. Confirmed methods (runtime-verified 2026-05-23):

| Method | Args | Purpose | Runtime status |
|--------|------|---------|----------------|
| `OnClickTraining` | 0 | Opens training view (no command_id) | ✅ verified |
| `OnClickTrainingMenu` | 1 | Player taps a specific training facility button | ✅ verified, hooked |
| `SendCommandAsync` | 6 | Submits command (arg1=command_id) | ✅ verified, hooked |
| `CommonSendCommandAsync` | 2 | Simpler command sender | ✅ verified, hooked |
| `OnClickRace` | 1 | Player selects a race | ✅ verified |
| `OnClickHospital` | 0 | Player selects rest | ✅ verified |
| `OnClickOuting` | 0 | Player selects outing | ✅ verified |

> **Note (2026-05-23):** Earlier docs listed `OnClickTraining` as taking 1 arg (command_id). Runtime shows it takes 0 args — it just opens the training view. The actual command_id flows through `SendCommandAsync(6)` where arg1 is the command_id (e.g., 106 = Wisdom). Field probes found 0/41 expected fields on this class; state is likely accessed via property getters, not direct fields.

### `TrainingSelectDecide`

Handles the training selection confirmation step:

| Method | Args | Purpose |
|--------|------|---------|
| `OnDecide` | 1 | Confirm training selection |

> **⚠️ Runtime status (2026-05-23):** Class NOT FOUND at runtime under `Gallop` in `umamusume.dll`. Present in metadata strings but may be a nested class or different assembly.

### `TrainingView`

Renders the training facility UI:

| Method | Purpose |
|--------|---------|
| `OnDecide` | Training confirmed from the view layer |
| `get_SelectedTrainingCommandId` | Returns the currently selected command_id |
| `get_TrainingCommandId` | Returns the active training command_id |

> **⚠️ Runtime status (2026-05-23):** Class NOT FOUND at runtime. Same as above.

### `TrainingController`

Manages training logic and state:

| Method | Purpose |
|--------|---------|
| `OnDecide` | Training decision processing |

> **⚠️ Runtime status (2026-05-23):** Class NOT FOUND at runtime. Same as above.

### `TrainingMain`

Top-level training orchestrator:

| Method | Purpose |
|--------|---------|
| `OnDecide` | Training decision processing |

> **⚠️ Runtime status (2026-05-23):** Class NOT FOUND at runtime. Same as above.

### Other Confirmed Classes

| Class | Purpose | Runtime (2026-05-23) |
|-------|---------|---------------------|
| `TrainingMenu` | Training facility menu UI | ⚠️ NOT FOUND |
| `TrainingButton` | Individual training button widget | ⚠️ NOT FOUND |
| `TrainingTop` | Training screen top-level layout | ⚠️ NOT FOUND |
| `WorkDataManager` | **Central singleton** for all game data | ✅ FOUND, LIVE singleton, 48 fields, 49 methods |
| `WorkSingleModeData` | Working copy of career state | ✅ FOUND via `WorkDataManager.get_SingleMode()`, 32 fields, 179 methods |
| `WorkSingleModeCharaData` | Working copy of character data | ✅ FOUND via `WorkSingleModeData.get_Character()`, 73 fields, 131 methods |
| `WorkSingleModeHomeInfo` | Working copy of home screen data | ✅ FOUND via `WorkSingleModeData.get_HomeInfo()`, 12 fields, 13 methods |

> **Note (2026-05-23):** Many training UI classes (`TrainingView`, `TrainingController`, `TrainingMenu`, `TrainingButton`, etc.) appear in metadata strings but are NOT FOUND as `Gallop::ClassName` in `umamusume.dll` at runtime. They may be nested classes, in a different namespace, or in a different assembly. The `Work*` data classes are the reliable runtime targets.

## Confirmed Fields & Properties

These fields are present on career mode objects:

| Field/Property | Type | Purpose |
|----------------|------|---------|
| `selectedCommandId` | `int` | Currently selected command ID |
| `selectedTraining` | object | Currently selected training info |
| `_commandId` | `int` | Internal command ID backing field |
| `_commandType` | `int` | Internal command type backing field |
| `_currentCommandId` | `int` | Currently active command ID |
| `_trainingCommandId` | `int` | Training-specific command ID |
| `_disableCommandIdList` | `List<int>` | Commands that are disabled this turn |
| `_trainingLevelDic` | `Dictionary` | Training level per facility |
| `_trainingPartnerInfoArray` | `Array` | Support cards at each facility |
| `_currentTrainingInfo` | object | Info about the current training |
| `_previewTrainingInfo` | object | Preview info for hovering |

## Career Scenarios

Each career scenario (URA, Grand Masters, UAF, Cook, etc.) extends the base flow with scenario-specific data sets and command IDs. The scenario type is tracked on `SingleModeChara.scenario_id`.

| Scenario | Data Set Class | Training Command IDs |
|----------|---------------|---------------------|
| URA (base) | (base) | 101, 105, 102, 103, 106 |
| Aoharu | (base) | 601, 602, 603, 604, 605 |
| Make a New Track | `SingleModeArcDataSet` | 1101, 1102, 1103, 1104, 1105 |
| Grand Masters (Venus) | `SingleModeVenusDataSet` | (uses base IDs) |
| UAF (Sport) | `SingleModeSportDataSet` | 2101–2105, 2201–2205, 2301–2305 (3 sub-types × 5 facilities) |
| Cook | `SingleModeCookDataSet` | varies by scenario |
| Mecha | `SingleModeMechaDataSet` | varies by scenario |
| Legend | `SingleModeLegendDataSet` | varies by scenario |
| Pioneer | `SingleModePioneerDataSet` | varies by scenario |
| Onsen | `SingleModeOnsenDataSet` | 901, 902, 906 (only 3 of 5 facilities confirmed) |
| Breeders | `SingleModeBreedersDataSet` | varies by scenario |

> **Note:** Command IDs are sparse, not contiguous ranges. For example, URA uses 101 (Speed), 105 (Stamina), 102 (Power), 103 (Guts), 106 (Wisdom). See [training-system.md](training-system.md) for the complete mapping.

## Memory-Read Access Chain (Confirmed 2026-05-24)

The preferred approach for reading career state is direct memory access via the IL2CPP singleton chain. This avoids hook-counting and gives a snapshot of the current state on demand.

```
WorkDataManager (LIVE singleton, confirmed at 0x217cea33a80)
  │
  ├── get_SingleMode() → WorkSingleModeData (32 fields, 179 methods)
  │     │
  │     ├── get_IsPlaying() → bool  (check before reading anything)
  │     ├── GetCurrentTurn() → int
  │     ├── GetFinalTurn() → int
  │     ├── GetRemainTurnNum() → int
  │     ├── get_Month() → int
  │     ├── get_State() → SingleModeDefine.State
  │     ├── get_SelectedTrainingCommandId() → ObscuredInt
  │     ├── get_TotalRaceCount() → int
  │     ├── get_WinCount() → int
  │     │
  │     ├── get_Character() → WorkSingleModeCharaData (73 fields, 131 methods)
  │     │     ├── get_Speed() → int       ┐
  │     │     ├── get_Stamina() → int      │ Core stats
  │     │     ├── get_Power() → int        │ (decrypted from ObscuredInt)
  │     │     ├── get_Guts() → int         │
  │     │     ├── get_Wiz() → int          ┘
  │     │     ├── get_Hp() → int           (current vital)
  │     │     ├── get_MaxHp() → int        (max vital)
  │     │     ├── get_SkillPoint() → ObscuredInt
  │     │     ├── get_Motivation() → RaceDefine.Motivation (enum)
  │     │     ├── get_FanCount() → int
  │     │     ├── GetAllTotalParameterValue() → int (sum of 5 stats)
  │     │     ├── GetTrainingLevel(commandId) → int
  │     │     └── get_CharaGrade() → CharaGradeType
  │     │
  │     └── get_HomeInfo() → WorkSingleModeHomeInfo (12 fields, 13 methods)
  │           ├── get_TurnInfoListDic() → Dictionary<CommandType, List<TurnInfo>>
  │           └── get_DisableCommandIdList() → List<ObscuredInt>
  │
  └── (48 other Work*Data getters for non-career game state)
```

> **Key insight (2026-05-24):** Backing fields use `ObscuredInt` (CodeStage anti-cheat encryption), but the C# property getters decrypt and return plain `System.Int32`. Always call getters via `il2cpp_get_method` + method invoke — never read encrypted fields directly.

> **Classes that DON'T exist:** `SingleModeManager`, `SingleModeWorkDataManager`, `SingleModeContext`, `SingleModeDataManager` — all NOT FOUND at runtime.

### Acquired Skills Access Chain (Confirmed 2026-05-24)

```
WorkSingleModeCharaData._acquiredSkillList : List<WorkSkillData.AcquiredSkill>
  │
  ├── element[i] : WorkSkillData.AcquiredSkill
  │     │  (0 own fields — all inherited from SkillDataBase)
  │     │
  │     ├── get_MasterId() → int       (skill_id, from SkillDataBase._masterId)
  │     ├── get_Level() → int          (skill level, from SkillDataBase._level)
  │     └── get_MasterData() → MasterSkillData.SkillData
  │           ├── .Id, .Rarity, .GroupId, .SkillCategory, ...
  │           ├── get_Name() → string (localized)
  │           └── get_Remarks() → string (description)
  │
  Inheritance: AcquiredSkill → SkillDataBase → System.Object
```

> **Resolution note:** `AcquiredSkill` cannot be found via `il2cpp_find_class("Gallop", "AcquiredSkill")`. It is `Gallop.WorkSkillData.AcquiredSkill` (nested). Use `il2cpp_find_nested_class(WorkSkillData, "AcquiredSkill")` or read the klass pointer from a live list element.

### Friendship / Bond Access Chain (Confirmed 2026-05-24)

```
WorkSingleModeCharaData  (73 fields, 131 methods)
  │
  ├── _evaluationList : List<WorkSingleModeCharaData.Evaluation>  (assumed field name)
  │     │
  │     ├── element[i] : WorkSingleModeCharaData.Evaluation
  │     │     ├── get_TargetId() → int       (support card ID)
  │     │     ├── get_Value() → int          (friendship gauge, 0–100+)
  │     │     ├── get_IsOuting() → bool      (outing occurred)
  │     │     ├── get_StoryStep() → int      (support story progress)
  │     │     ├── get_IsAppear() → bool      (character appeared this run)
  │     │     ├── get_GuestCharaId() → int   (guest character, if any)
  │     │     ├── get_InterestState() → InterestState  (team race interest)
  │     │     ├── get_SoulEventState() → bool (soul event active)
  │     │     └── get_SoulThresholdId() → int (soul threshold)
  │     │
  │     └── Server source: evaluation_info_array in SingleModeCheckEventResponse
  │
  └── MasterSingleModeEvaluation (master DB)
        ├── Get(id) → SingleModeEvaluation
        └── GetListWithCharaIdOrderByIdAsc(charaId) → List<SingleModeEvaluation>
```

> **Resolution note:** The friendship class is `WorkSingleModeCharaData.Evaluation` (nested). It cannot be found as `EvaluationInfo`, `SingleModeEvaluation`, or `WorkSingleModeEvaluationInfo`. Use `il2cpp_find_nested_class(WorkSingleModeCharaData, "Evaluation")`.

## Data Flow (Server Responses)

```
Server Response (MessagePack)
    │
    ▼
SingleModeCheckEventResponse.CommonResponse
    ├── chara_info: SingleModeChara
    │       ├── speed, stamina, power, wiz (Wisdom), guts, vital
    │       ├── training_level_info_array: TrainingLevelInfo[]
    │       ├── skill_array → WorkSkillData.AcquiredSkill (via Convert)
    │       │     └── each: { skill_id, level } → SkillDataBase._masterId, ._level
    │       ├── skill_tips_array → WorkSingleModeCharaData.SkillTips
    │       │     └── each: { group_id, rarity, level }
    │       ├── evaluation_info_array → WorkSingleModeCharaData.Evaluation
    │       │     └── each: { target_id, value, is_outing, story_step, ... }
    │       └── support_card_array
    ├── home_info: SingleModeHomeInfo
    │       ├── command_info_array: SingleModeCommandInfo[]
    │       └── disable_command_id_array: int[]
    ├── command_result: SingleModeCommandResult
    └── [scenario]_data_set: scenario-specific data
```
