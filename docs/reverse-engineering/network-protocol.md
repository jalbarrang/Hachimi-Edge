# Network Protocol — Career Mode

## Overview

The game communicates with Cygames servers using **MessagePack**-encoded requests and responses over HTTPS. Each career action (training, race entry, skill learning, etc.) follows a request → response pattern. The protocol has been reverse-engineered by the [UmamusumeResponseAnalyzer](https://github.com/UmamusumeResponseAnalyzer/UmamusumeResponseAnalyzer) project.

Intercepting these messages requires either:
- A network proxy (like [ura-core](https://github.com/UmamusumeResponseAnalyzer/ura-core)) that captures and forwards packets
- IL2CPP hooking of the serialization/deserialization layer

## Wire Format

> Source: Confirmed by Trainers-Legend-G `request_conv.cpp`. See [trainers-legend-g-crossref.md](trainers-legend-g-crossref.md).

Requests and responses are LZ4-compressed, then framed:

```
[4 bytes: uint32 header_length (normally 166)] [header_length bytes: header] [msgpack payload]
```

- Compression uses `LZ4_compress_default_ext` / `LZ4_decompress_safe_ext` from `libnative.dll`
- The msgpack payload starts at byte offset `4 + header_length`
- TLG hooks the LZ4 layer to intercept before compression (requests) and after decompression (responses)

### Known API Endpoints

```
https://api-umamusume.cygames.jp/umamusume/note/trainer_note
https://api-umamusume.cygames.jp/umamusume/card/get_release_card_array
https://api-umamusume.cygames.jp/umamusume/note/get_new_chara_data
```

## Key Request/Response Pairs

### Career Lifecycle

| Request | Response | When |
|---------|----------|------|
| `SingleModeStartRequest` | `SingleModeStartResponse` | Career begins |
| `SingleModeCheckEventRequest` | `SingleModeCheckEventResponse` | Each turn start |
| `SingleModeExecCommandRequest` | `SingleModeExecCommandResponse` | Player executes an action |
| `SingleModeGainSkillsRequest` | `SingleModeGainSkillsResponse` | Skill learning |
| `SingleModeRaceEntryRequest` | `SingleModeRaceEntryResponse` | Race entry |
| `SingleModeRaceStartRequest` | `SingleModeRaceStartResponse` | Race begins |
| `SingleModeRaceEndRequest` | `SingleModeRaceEndResponse` | Race ends |
| `SingleModeFinishRequest` | `SingleModeFinishResponse` | Career ends |
| `SingleModeContinueRequest` | `SingleModeContinueResponse` | Continue after failure |
| `SingleModeLoadRequest` | `SingleModeLoadResponse` | Resume saved career |

### Scenario-Specific Variants

Each career scenario has its own set of request/response types prefixed with the scenario name:

| Scenario | Prefix | Example |
|----------|--------|---------|
| Free (base) | `SingleModeFree` | `SingleModeFreeExecCommandRequest` |
| Team Race | `SingleModeTeam` | `SingleModeTeamExecCommandRequest` |

Confirmed complete list of scenario-specific tasks:
- `SingleModeFree*` — Free scenario (base game + variants)
- `SingleModeTeam*` — Team race scenario

## Core Data Structures

### `SingleModeExecCommandRequest`

Sent when the player chooses a training/action:

```
{
    command_type: int      // 1=training, 3=outing, 4=rest, 7=race
    command_id: int        // Facility/action ID (e.g., 101=Speed)
    command_group_id: int  // Group ID (scenario-specific)
    select_id: int         // Selection ID for choices
    current_turn: int      // Current turn number
    current_vital: int     // Current stamina/HP
}
```

### `SingleModeCheckEventResponse.CommonResponse`

The main turn-state payload returned each turn:

```
{
    chara_info: SingleModeChara              // Character stats & state
    not_up_parameter_info: NotUpParameterInfo // Stats at cap
    not_down_parameter_info: NotDownParameterInfo // Stats at floor
    home_info: SingleModeHomeInfo            // Available commands
    command_result: SingleModeCommandResult   // Result of last command
    unchecked_event_array: SingleModeEventInfo[]  // Pending events
    event_effected_factor_array: SuccessionEffectedFactor[]  // Inheritance effects
    race_condition_array: SingleModeRaceCondition[]  // Available races
    race_start_info: SingleRaceStartInfo     // Race start data
    
    // Scenario-specific data sets (only one populated per career):
    team_data_set: SingleModeTeamDataSet
    free_data_set: SingleModeFreeDataSet
    live_data_set: SingleModeTeamDataSet     // Note: reuses TeamDataSet type
    venus_data_set: SingleModeVenusDataSet
    arc_data_set: SingleModeArcDataSet
    sport_data_set: SingleModeSportDataSet
    cook_data_set: SingleModeCookDataSet
    mecha_data_set: SingleModeMechaDataSet
    legend_data_set: SingleModeLegendDataSet
    pioneer_data_set: SingleModePioneerDataSet
    onsen_data_set: SingleModeOnsenDataSet
    breeders_data_set: SingleModeBreedersDataSet
    
    select_index: int?  // Selected choice index
}
```

### `SingleModeChara`

Full character state each turn:

```
{
    // Identity
    single_mode_chara_id: int
    card_id: int
    scenario_id: int
    turn: int
    
    // Base stats
    speed: int, stamina: int, power: int, wiz: int, guts: int  // wiz = Wisdom
    vital: int, max_vital: int
    motivation: int  // Mood (1=worst, 5=best)
    
    // Stat caps
    max_speed: int, max_stamina: int, max_power: int, max_wiz: int, max_guts: int
    default_max_speed: int, default_max_stamina: int, ...
    
    // Skills
    skill_array: SkillData[]
    skill_tips_array: SkillTips[]
    skill_point: int
    
    // Support cards
    support_card_array: SingleModeSupportCard[]
    
    // Training levels (facility hit proxy)
    training_level_info_array: TrainingLevelInfo[]
    
    // Aptitudes
    proper_distance_short: int, proper_distance_mile: int, ...
    proper_running_style_nige: int, ...
    proper_ground_turf: int, proper_ground_dirt: int
    
    // Status effects
    chara_effect_id_array: int[]
    nickname_id_array: int[]
    
    // Misc
    fans: int, rarity: int, talent_level: int
    state: int, playing_state: int
    evaluation_info_array: EvaluationInfo[]
    guest_outing_info_array: GuestOutingInfo[]
    skill_upgrade_info_array: SkillUpgradeInfo[]
}
```

### `SingleModeHomeInfo`

Available actions for the current turn:

```
{
    command_info_array: SingleModeCommandInfo[]  // Available commands
    disable_command_id_array: int[]              // Disabled commands
    race_entry_restriction: int                  // Race entry restrictions
    available_continue_num: int                  // Continues remaining
    free_continue_time: long                     // Free continue timestamp
    shortened_race_state: int                    // Race shortening state
}
```

### `SingleModeCommandInfo`

Per-command details (one entry per available training/action):

```
{
    command_type: int                              // 1=training
    command_id: int                                // Facility ID
    is_enable: int                                 // Available?
    training_partner_array: int[]                  // Support cards present
    tips_event_partner_array: int[]                // Hint event partners
    params_inc_dec_info_array: SingleModeParamsIncDecInfo[]  // Stat preview
    failure_rate: int                              // Failure chance (%)
}
```

### `SingleModeCommandResult`

Result of the last executed command:

```
{
    command_id: int      // Which command was executed
    sub_id: int          // Sub-command ID
    result_state: int    // Success/failure state
}
```

## Interception Points

For a Hachimi plugin, the most practical interception points are:

1. **IL2CPP hook on the UI method** that sends `SingleModeExecCommandRequest` — gives `command_type` and `command_id` before the request is sent
2. **IL2CPP hook on the response handler** for `SingleModeCheckEventResponse` — gives full turn state including `training_level_info_array`
3. **Network-level interception** via Hachimi's IPC system or a separate proxy

## Related Projects

- [UmamusumeResponseAnalyzer](https://github.com/UmamusumeResponseAnalyzer/UmamusumeResponseAnalyzer) — C# tool that deserializes and analyzes these responses
- [ura-core](https://github.com/UmamusumeResponseAnalyzer/ura-core) — C++ hook that captures game packets and forwards them to a local HTTP server
- [EXNOA-CarrotJuicer](https://github.com/CNA-Bld/EXNOA-CarrotJuicer) — Alternative packet capture tool
