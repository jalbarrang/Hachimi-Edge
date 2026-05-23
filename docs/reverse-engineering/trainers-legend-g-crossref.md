# Trainers-Legend-G Cross-Reference

Cross-reference analysis of [Trainers-Legend-G](https://github.com/MinamiChiwa/Trainers-Legend-G) (TLG), a C++ DLL-injection mod for the DMM/Windows version of Uma Musume. TLG is primarily a localization, character model replacement, and visual enhancement tool — **not** a training tracker.

**Analysis date:** 2026-05-23  
**TLG version:** latest main branch (shallow clone)

## Executive Summary

- TLG has **~136 active IL2CPP hooks** plus 3 native trampolines
- **No training tracking or command ID mapping** exists in TLG — their SingleMode hooks are limited to character model replacement and image effects
- TLG's biggest value to us is **race telemetry classes** (`HorseRaceInfo`, `HorseRaceInfoReplay`), the **`UmaControllerType` enum**, **`CharacterBuildInfo` constructor signatures**, and **msgpack framing details**
- TLG confirms our IL2CPP class map is accurate and fills in gaps for race/live/physics classes
- TLG's IL2CPP resolution approach (direct `GetProcAddress` on `GameAssembly.dll`) matches Hachimi's vtable-mediated equivalent

## Classes/Methods TLG Uses That We Hadn't Documented

### `Gallop.HorseRaceInfo` (race stats — NEW)

Provides real-time race telemetry. TLG reads these via function pointers (not hooks) for their race info display.

| Property | Return Type | Args | Notes |
|----------|-------------|------|-------|
| `get_RaceBaseSpeed` | float | 0 | |
| `get_MinSpeed` | float | 0 | |
| `get_StartDashSpeedThreshold` | float | 0 | |
| `get_IsOverRun` | bool | 0 | |
| `GetHp` | float | 0 | Current HP |
| `GetMaxHp` | float | 0 | Max HP |
| `GetHpPer` | float | 0 | HP percentage |
| `get_NearHorseCount` | int | 0 | Commented out in TLG |
| `get_CongestionTime` | float | 0 | Commented out in TLG |
| `get_RawSpeed` | int | 0 | Base stat before modifiers |
| `get_BaseSpeed` | int | 0 | |
| `get_Speed` | float | 0 | Effective race speed |
| `get_RawStamina` | int | 0 | |
| `get_BaseStamina` | float | 0 | |
| `get_Stamina` | float | 0 | |
| `get_RawPow` | int | 0 | |
| `get_BasePow` | float | 0 | |
| `get_Pow` | float | 0 | |
| `get_RawGuts` | int | 0 | |
| `get_BaseGuts` | float | 0 | |
| `get_Guts` | float | 0 | |
| `get_RawWiz` | int | 0 | |
| `get_BaseWiz` | float | 0 | |
| `get_Wiz` | float | 0 | |
| `get_IsStartDash` | bool | 0 | |
| `get_MoveDistance` | float | 0 | |

### `Gallop.HorseRaceInfoReplay` (NEW)

| Member | Type | Args |
|--------|------|------|
| `.ctor` | method | 2 |
| `get_RunMotionSpeed` | property | 0 |
| `get_RunMotionRate` | property | 0 |
| `get_RaceMotion` | property | 0 |
| `get_IsLastSpurt` | property | 0 |
| `get_LastSpurtStartDistance` | property | 0 |
| `get_FinishOrder` | property | 0 |

### `Gallop.HorseData` (partially new)

| Member | Args | Status |
|--------|------|--------|
| `get_GateNo` | 0 | NEW |
| `get_charaName` | 0 | NEW |
| `get_TrainerName` | 0 | NEW |
| `InitTrainerName` | 0 | NEW |
| `get_SingleModeTeamRank` | 0 | Already had |

### `Gallop.SkillManager` (NEW)

| Member | Args |
|--------|------|
| `GetSkill` | ? |
| `AddUsedSkillId` | 1 (hooked) |

### `Gallop.SkillBase` (NEW)

| Member | Type |
|--------|------|
| `get_Level` | property |
| `get_Details` | property |
| `get_SkillMaster` | property |

### `StandaloneSimulator.SkillDetail` (NEW)

| Member | Type |
|--------|------|
| `get_Abilities` | property |
| `get_SkillEffectName` | property |
| `Activate` | method (commented out in TLG) |
| `get_DefaultCoolDownTime` | property (commented out in TLG) |

### `Gallop.CySpringParamDataElement` (fields — NEW)

Physics bone simulation parameters. All are fields:

| Field | Type |
|-------|------|
| `_boneName` | string |
| `_stiffnessForce` | float |
| `_dragForce` | float |
| `_gravity` | Vector3 |
| `_childElements` | array |
| `_verticalWindRateSlow` | float |
| `_collisionRadius` | float |
| `_needEnvCollision` | bool |
| `_horizontalWindRateSlow` | float |
| `_verticalWindRateFast` | float |
| `_horizontalWindRateFast` | float |
| `_isLimit` | bool |
| `_MoveSpringApplyRate` | float |

`Gallop.CySpringParamDataChildElement` has the same field set minus `_childElements`, `_collisionRadius`, `_needEnvCollision`, `_isLimit`, and `_MoveSpringApplyRate`.

### `Gallop.CharacterBuildInfo` (constructor signatures — NEW)

| Constructor | Args | Parameters |
|-------------|------|------------|
| `.ctor` | 11 | `charaId, dressId, controllerType, headId, zekken, mobId, backDancerColorId, isUseDressDataHeadModelSubId, audienceId, motionDressId, isEnableModelCache` |
| `.ctor` | 14 | `cardId, charaId, dressId, controllerType, headId, zekken, mobId, backDancerColorId, overrideClothCategory, isUseDressDataHeadModelSubId, audienceId, motionDressId, isEnableModelCache, charaDressColorSetId` |

### `Gallop.EditableCharacterBuildInfo` (NEW)

| Constructor | Args | Parameters |
|-------------|------|------------|
| `.ctor` | 11 | `cardId, charaId, dressId, controllerType, zekken, mobId, backDancerColorId, headId, isUseDressDataHeadModelSubId, isEnableModelCache, chara_dress_color_set_id` |

### `Gallop.SingleModeSceneController` (confirmed signature)

| Method | Args | Parameters |
|--------|------|------------|
| `CreateModel` | 3 | `cardId: int, dressId: int, addVoiceCue: bool` |

### `UmaControllerType` Enum (NEW)

```
Default      = 0x0
Race         = 0x1
Training     = 0x2
EventTimeline = 0x3
Live         = 0x4
LiveTheater  = 0x5
HomeStand    = 0x6
HomeTalk     = 0x7
HomeWalk     = 0x8
CutIn        = 0x9
TrainingTop  = 0xa
SingleRace   = 0xb
Simple       = 0xc
Mini         = 0xd
Paddock      = 0xe
Champions    = 0xf
```

This enum is passed as the `controllerType` parameter to `CharacterBuildInfo` constructors.

### `Gallop.Certification` (NEW)

| Method | Args |
|--------|------|
| `get_dmmViewerId` | 0 |

### `Gallop.GallopUtil` (additional methods — NEW)

| Method | Args |
|--------|------|
| `GetUserName` | 0 |

## Network Protocol Findings

### Msgpack Framing (confirmed + detail)

TLG's `parse_request_pack` confirms the request/response framing:

```
[4 bytes: offset value (expected 166)] [offset bytes: header] [msgpack payload]
```

- The 4-byte prefix is a uint32 containing the header length (normally `166`)
- The actual msgpack data starts at byte `4 + offset`
- Compression: LZ4 via `libnative.dll` exports `LZ4_compress_default_ext` and `LZ4_decompress_safe_ext`

TLG intercepts at the LZ4 layer (before compression for requests, after decompression for responses), which gives access to the raw msgpack without dealing with HTTP/TLS.

### API Endpoints Referenced

```
https://api-umamusume.cygames.jp/umamusume/note/trainer_note
https://api-umamusume.cygames.jp/umamusume/card/get_release_card_array
https://api-umamusume.cygames.jp/umamusume/note/get_new_chara_data
```

## TLG's SingleMode Hooks (Limited)

TLG only hooks 3 SingleMode-related methods, all for **character model replacement**, not training tracking:

| Class | Method | Args | Purpose |
|-------|--------|------|---------|
| `SingleModeStartResultCharaViewer` | `SetupImageEffect` | 0 | Image effects during results |
| `SingleModeSceneController` | `CreateModel` | 3 | Character model loading |
| `WorkSingleModeCharaData` | `GetRaceDressId` | 1 | Race dress ID lookup |

Additionally, `HorseData.get_SingleModeTeamRank(0)` is resolved as a function pointer but **not hooked**.

**Conclusion for our plugin:** TLG does NOT hook any training-command methods (`OnClickTraining`, `OnDecide`, `OnSelectCommand`, etc.) and has no training facility tracking. Our hook candidates in `hooks.rs` remain the correct targets — TLG simply doesn't do what we're trying to do.

## TLG's Race Telemetry Display

TLG implements a real-time race info overlay using ImGui that displays:

- Gate position, character name, trainer name
- Speed, HP, HP%, max HP
- RaceBaseSpeed, MinSpeed, StartDashSpeedThreshold
- Raw/Base/Effective values for all 5 stats
- IsOverRun, IsStartDash, IsLastSpurt, FinishOrder
- Skill activation tracking (via `SkillManager.AddUsedSkillId` hook)
- NearHorseCount, CongestionTime (commented out)

This is the closest TLG gets to "stat display" — it's race-time only, not training mode.

## TLG's Live Scene Controls

Extensive live concert visualization with editable parameters:

- DOF (depth of field)
- Post-film effects (3 variants)
- Light projection
- Radial blur
- Exposure / Vortex
- Character foot lights
- Global lighting
- Camera position/rotation/FOV

All implemented via hooks on `Gallop.Live.Cutt.LiveTimelineControl` methods.

## Architecture Comparison: TLG vs Hachimi Plugin

| Aspect | TLG | Hachimi Plugin |
|--------|-----|----------------|
| Language | C++ | Rust |
| Injection | DLL proxy (version.dll) | Plugin API (cdylib) |
| IL2CPP resolution | Direct GetProcAddress | Via Hachimi vtable |
| Hooking library | MinHook | Hachimi interceptor (MinHook backend) |
| GUI | ImGui (DX11) | egui (host-mediated) |
| Method resolution | `class_from_name` + `get_method_from_name` | Same, via vtable wrappers |
| Feature scope | Localization, visual mods, camera, race display | Training tracking |

## Files Examined

| File | Contents |
|------|----------|
| `src/hook.cpp` | Main hook file (~5600 lines), all 136 IL2CPP hook targets |
| `src/il2cpp/il2cpp_symbols.cpp/.hpp` | IL2CPP resolution API, struct definitions |
| `src/requestConvert/request_conv.cpp/.hpp` | Msgpack request interception |
| `src/eventHelper/eventHelper.cpp/.hpp` | Event choice/effect lookup |
| `src/umadb/umadb.cpp/.hpp` | Master DB queries, asset resolution |
| `src/umagui/guiShowData.cpp/.hpp` | Race/live UI data structures |
| `src/umagui/umaguiMain.cpp/.hpp` | ImGui render loop |
| `src/umagui/liveGUI.cpp/.hpp` | Live scene controls |
| `src/umaHook/liveHook.cpp/.hpp` | Live scene hook targets |
| `src/main.cpp` | Initialization, config loading |
| `src/stdinclude.hpp` | Common definitions |
| `src/camera/camera.cpp/.hpp` | Camera math/state |
| `src/local/local.cpp/.hpp` | Translation DB |
