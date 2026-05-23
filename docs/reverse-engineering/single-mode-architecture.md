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

The primary view controller for career mode. Confirmed methods:

| Method | Args | Purpose |
|--------|------|---------|
| `OnClickTraining` | 1 (command_id) | Player taps a training facility |
| `OnClickSelect` | 1 | Player selects an option |
| `OnClickStart` | 0 | Player confirms/starts action |
| `OnClickRace` | 0 or 1 | Player selects a race |
| `OnSelectCommand` | 2 (type, id) | Generic command selection |

### `TrainingSelectDecide`

Handles the training selection confirmation step:

| Method | Args | Purpose |
|--------|------|---------|
| `OnDecide` | 1 | Confirm training selection |

### `TrainingView`

Renders the training facility UI:

| Method | Purpose |
|--------|---------|
| `OnDecide` | Training confirmed from the view layer |
| `get_SelectedTrainingCommandId` | Returns the currently selected command_id |
| `get_TrainingCommandId` | Returns the active training command_id |

### `TrainingController`

Manages training logic and state:

| Method | Purpose |
|--------|---------|
| `OnDecide` | Training decision processing |

### `TrainingMain`

Top-level training orchestrator:

| Method | Purpose |
|--------|---------|
| `OnDecide` | Training decision processing |

### Other Confirmed Classes

| Class | Purpose |
|-------|---------|
| `TrainingMenu` | Training facility menu UI |
| `TrainingButton` | Individual training button widget |
| `TrainingTop` | Training screen top-level layout |
| `WorkSingleModeData` | Working copy of career state |
| `WorkSingleModeHomeInfo` | Working copy of home screen data |

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

## Data Flow

```
Server Response (MessagePack)
    │
    ▼
SingleModeCheckEventResponse.CommonResponse
    ├── chara_info: SingleModeChara
    │       ├── speed, stamina, power, wiz (Wisdom), guts, vital
    │       ├── training_level_info_array: TrainingLevelInfo[]
    │       ├── skill_array, skill_tips_array
    │       └── support_card_array
    ├── home_info: SingleModeHomeInfo
    │       ├── command_info_array: SingleModeCommandInfo[]
    │       └── disable_command_id_array: int[]
    ├── command_result: SingleModeCommandResult
    └── [scenario]_data_set: scenario-specific data
```
