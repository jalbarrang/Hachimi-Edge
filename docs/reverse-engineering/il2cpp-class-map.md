# IL2CPP Class Map

Confirmed classes, methods, fields, and properties extracted from the game's `global-metadata.dat` (IL2CPP metadata v31). All names are in the `Gallop` namespace unless noted.

## Metadata Analysis Details

- **Source**: `UmamusumePrettyDerby_Data/il2cpp_data/Metadata/global-metadata.dat`
- **Metadata version**: 31
- **IL2CPP version**: 31
- **File size**: ~29 MB
- **Method**: Direct string table parsing (PE loader failed due to DllMain dependency on Unity runtime)

## Career Mode Controllers

### `SingleModeMainViewController`
Primary view controller for career mode.

**Methods:**
| Name | Args | Confirmed |
|------|------|-----------|
| `OnClickTraining` | 1 | ✅ metadata |
| `OnClickSelect` | 1 | ✅ metadata |
| `OnClickStart` | 0 | ✅ metadata |
| `OnClickRace` | ? | ✅ metadata |
| `OnSelectCommand` | 2 | Likely |
| `OnDecide` | ? | ✅ metadata |

**Properties:**
- `get_SelectedTrainingCommandId` / `set_SelectedTrainingCommandId`
- `get_TrainingCommandId`
- `get_SeaTrainingCommandId`
- `get_SingleModeCharaId`
- `get_SingleModeScenario`
- `get_TrainingView`
- `get_TrainingController`
- `get_TrainingMenu`
- `get_TrainingButton`
- `get_TrainingButtonRoot`
- `get_TrainingFooter`
- `get_TrainingNum`
- `get_TrainingRank`
- `get_TrainingStatus`
- `get_TrainingTipsModel`
- `get_TrainingTurnInfo`
- `get_IsInTraining`
- `get_IsPlayingTrainingCutt`
- `get_IsPlayingOrWillPlayTrainingCutt`
- `get_IsSingleMode`
- `get_IsSingleModeChara`
- `get_SingleModeFlashRoot`
- `get_SingleModeFooter`
- `get_SingleModeHeader`

**Fields:**
- `_commandId`, `_commandType`, `_currentCommandId`
- `_trainingCommandId`, `selectedCommandId`
- `_disableCommandIdList`
- `_singleModeCharaData`, `_singleModeData`
- `_singleModeCharacterData`, `_singleModeCharaImage`
- `_trainingView`, `_trainingController`
- `_trainingButton`, `_homeTrainingButton`
- `_trainingLevelDic`
- `_trainingPartnerInfoArray`
- `_trainingCuttController`
- `_bestTraining`
- `_finalTrainingRank`, `_finalTrainingRankArray`
- `_singleModePrevSelectedButtonType`
- `_singleModeLogGroupScroll`, `_singleModeLogItemPool`
- `_singleModeNextButton`

## Training UI Classes

### `TrainingView`
| Member | Type | Confirmed |
|--------|------|-----------|
| `OnDecide` | method | ✅ |
| `get_SelectedTrainingCommandId` | property | ✅ |
| `get_TrainingCommandId` | property | ✅ |
| `_currentTrainingInfo` | field | ✅ |
| `_previewTrainingInfo` | field | ✅ |
| `_trainingButton` | field | ✅ |
| `_trainingBg` | field | ✅ |
| `_trainingBgObject` | field | ✅ |

### `TrainingSelectDecide`
| Member | Type | Confirmed |
|--------|------|-----------|
| `OnDecide` | method | ✅ |

### `TrainingController`
| Member | Type | Confirmed |
|--------|------|-----------|
| `OnDecide` | method | ✅ |
| `get_TrainingLevel` | property | ✅ |
| `get_TrainingRank` | property | ✅ |
| `get_TrainingStatus` | property | ✅ |
| `get_IsInTraining` | property | ✅ |
| `get_TrainingHorse` | property | ✅ |
| `get_TrainingHorseList` | property | ✅ |
| `get_TrainingCutStatus` | property | ✅ |
| `get_TrainingHighSpeedType` | property | ✅ |
| `get_TrainingEventType` | property | ✅ |

### `TrainingMain`
| Member | Type | Confirmed |
|--------|------|-----------|
| `OnDecide` | method | ✅ |

### `TrainingMenu`
| Member | Type | Confirmed |
|--------|------|-----------|
| `_trainingButton` | field | ✅ |
| `_trainingCountText` | field | ✅ |
| `_trainingLevelText` | field | ✅ |
| `_trainingNameText` | field | ✅ |
| `_trainingNameShadow` | field | ✅ |
| `_trainingIconImage` | field | ✅ |
| `_trainingIconBase` | field | ✅ |
| `_trainingMaxIcon` | field | ✅ |

### `TrainingButton`
| Member | Type | Confirmed |
|--------|------|-----------|
| `_trainingIconImage` | field | ✅ |
| `_trainingIconBase` | field | ✅ |
| `_trainingLevelTitle` | field | ✅ |
| `_trainingLevelText` | field | ✅ |
| `_trainingButtonFlash` | field | ✅ |

### `TrainingParamChangeA2U`
Text display for training stat gain captions. Already hooked by Hachimi for localization.

### `TrainingParamChangePlate`
Plate/log text for training stat changes. Already hooked by Hachimi for localization.

### `TrainingParamChangeUI`
Overlay for stat change visualization.

### `TrainingParamChangeSupportMemberA2U`
Support member stat change display.

### `TrainingDefine` / `TrainingDefineExtensions`
Constants and utility methods for the training system.

### `TrainingCuttController` / `TrainingCuttData`
Training cutscene playback controller and data.

### `TrainingEnvParam` / `TrainingEnvParamHelper`
Training environment parameters (visuals, effects).

### `TrainingModelController`
3D model management during training scenes.

### `TrainingFootSmokeController`
Foot smoke particle effects during training.

## Race Telemetry Classes

> Source: Trainers-Legend-G cross-reference. See [trainers-legend-g-crossref.md](trainers-legend-g-crossref.md).

### `HorseRaceInfo`
Real-time race statistics for each horse during a race.

| Property | Return Type | Confirmed |
|----------|-------------|----------|
| `get_RaceBaseSpeed` | float | ✅ TLG |
| `get_MinSpeed` | float | ✅ TLG |
| `get_StartDashSpeedThreshold` | float | ✅ TLG |
| `get_IsOverRun` | bool | ✅ TLG |
| `GetHp` | float | ✅ TLG |
| `GetMaxHp` | float | ✅ TLG |
| `GetHpPer` | float | ✅ TLG |
| `get_NearHorseCount` | int | ⚠️ TLG (commented out) |
| `get_CongestionTime` | float | ⚠️ TLG (commented out) |
| `get_RawSpeed` / `get_BaseSpeed` / `get_Speed` | int/int/float | ✅ TLG |
| `get_RawStamina` / `get_BaseStamina` / `get_Stamina` | int/float/float | ✅ TLG |
| `get_RawPow` / `get_BasePow` / `get_Pow` | int/float/float | ✅ TLG |
| `get_RawGuts` / `get_BaseGuts` / `get_Guts` | int/float/float | ✅ TLG |
| `get_RawWiz` / `get_BaseWiz` / `get_Wiz` | int/float/float | ✅ TLG |
| `get_IsStartDash` | bool | ✅ TLG |
| `get_MoveDistance` | float | ✅ TLG |

### `HorseRaceInfoReplay`
Replay/recorded race data.

| Member | Type | Args | Confirmed |
|--------|------|------|----------|
| `.ctor` | method | 2 | ✅ TLG (hooked) |
| `get_RunMotionSpeed` | property | 0 | ✅ TLG (hooked) |
| `get_RunMotionRate` | property | 0 | ✅ TLG |
| `get_RaceMotion` | property | 0 | ✅ TLG |
| `get_IsLastSpurt` | property | 0 | ✅ TLG |
| `get_LastSpurtStartDistance` | property | 0 | ✅ TLG |
| `get_FinishOrder` | property | 0 | ✅ TLG |

### `HorseData`
Per-horse identification data.

| Member | Args | Confirmed |
|--------|------|----------|
| `get_GateNo` | 0 | ✅ TLG |
| `get_charaName` | 0 | ✅ TLG |
| `get_TrainerName` | 0 | ✅ TLG |
| `InitTrainerName` | 0 | ✅ TLG |
| `get_SingleModeTeamRank` | 0 | ✅ TLG |

## Skill System Classes

> Source: Trainers-Legend-G cross-reference.

### `SkillManager`
| Member | Args | Confirmed |
|--------|------|----------|
| `GetSkill` | ? | ✅ TLG |
| `AddUsedSkillId` | 1 | ✅ TLG (hooked) |

### `SkillBase`
| Member | Type | Confirmed |
|--------|------|----------|
| `get_Level` | property | ✅ TLG |
| `get_Details` | property | ✅ TLG |
| `get_SkillMaster` | property | ✅ TLG |

### `StandaloneSimulator.SkillDetail`

> Note: Namespace is `StandaloneSimulator`, not `Gallop`.

| Member | Type | Confirmed |
|--------|------|----------|
| `get_Abilities` | property | ✅ TLG |
| `get_SkillEffectName` | property | ✅ TLG |
| `Activate` | method | ⚠️ TLG (commented out) |
| `get_DefaultCoolDownTime` | property | ⚠️ TLG (commented out) |

## Character Build Classes

> Source: Trainers-Legend-G cross-reference.

### `CharacterBuildInfo`

| Member | Args | Parameters | Confirmed |
|--------|------|------------|----------|
| `.ctor` | 11 | `charaId, dressId, controllerType, headId, zekken, mobId, backDancerColorId, isUseDressDataHeadModelSubId, audienceId, motionDressId, isEnableModelCache` | ✅ TLG |
| `.ctor` | 14 | `cardId, charaId, dressId, controllerType, headId, zekken, mobId, backDancerColorId, overrideClothCategory, isUseDressDataHeadModelSubId, audienceId, motionDressId, isEnableModelCache, charaDressColorSetId` | ✅ TLG |
| `Rebuild` | 0 | — | ✅ TLG (hooked) |

### `EditableCharacterBuildInfo`

| Member | Args | Parameters | Confirmed |
|--------|------|------------|----------|
| `.ctor` | 11 | `cardId, charaId, dressId, controllerType, zekken, mobId, backDancerColorId, headId, isUseDressDataHeadModelSubId, isEnableModelCache, chara_dress_color_set_id` | ✅ TLG |

### `SingleModeSceneController` (confirmed signature)

| Member | Args | Parameters | Confirmed |
|--------|------|------------|----------|
| `CreateModel` | 3 | `cardId: int, dressId: int, addVoiceCue: bool` | ✅ TLG (hooked) |

## UmaControllerType Enum

> Source: Trainers-Legend-G. Used as the `controllerType` parameter in `CharacterBuildInfo` constructors.

| Name | Value |
|------|-------|
| Default | 0x0 |
| Race | 0x1 |
| Training | 0x2 |
| EventTimeline | 0x3 |
| Live | 0x4 |
| LiveTheater | 0x5 |
| HomeStand | 0x6 |
| HomeTalk | 0x7 |
| HomeWalk | 0x8 |
| CutIn | 0x9 |
| TrainingTop | 0xa |
| SingleRace | 0xb |
| Simple | 0xc |
| Mini | 0xd |
| Paddock | 0xe |
| Champions | 0xf |

## Physics/Spring Classes

> Source: Trainers-Legend-G cross-reference.

### `CySpringParamDataElement`
Bone spring simulation parameters.

| Field | Type | Confirmed |
|-------|------|----------|
| `_boneName` | string | ✅ TLG |
| `_stiffnessForce` | float | ✅ TLG |
| `_dragForce` | float | ✅ TLG |
| `_gravity` | Vector3 | ✅ TLG |
| `_childElements` | array | ✅ TLG |
| `_verticalWindRateSlow` | float | ✅ TLG |
| `_collisionRadius` | float | ✅ TLG |
| `_needEnvCollision` | bool | ✅ TLG |
| `_horizontalWindRateSlow` | float | ✅ TLG |
| `_verticalWindRateFast` | float | ✅ TLG |
| `_horizontalWindRateFast` | float | ✅ TLG |
| `_isLimit` | bool | ✅ TLG |
| `_MoveSpringApplyRate` | float | ✅ TLG |

### `CySpringParamDataChildElement`
Same fields as parent minus `_childElements`, `_collisionRadius`, `_needEnvCollision`, `_isLimit`, `_MoveSpringApplyRate`.

## Miscellaneous Classes (TLG-sourced)

### `Gallop.Certification`
| Method | Args | Confirmed |
|--------|------|----------|
| `get_dmmViewerId` | 0 | ✅ TLG |

### `Gallop.GallopUtil` (additional)
| Method | Args | Confirmed |
|--------|------|----------|
| `GetUserName` | 0 | ✅ TLG |

### `Gallop.GameDefine`
| Method | Args | Confirmed |
|--------|------|----------|
| `get_ApplicationServerUrl` | 0 | ✅ TLG (hooked) |

## Data Model Classes

### `WorkSingleModeData`
Working copy of career state during gameplay.

### `WorkSingleModeHomeInfo`
Working copy of home screen data including available commands.

### `WorkSingleModeCharaData`
Working copy of character data during career.

**Methods (confirmed via TLG):**
| Method | Args | Confirmed |
|--------|------|----------|
| `GetRaceDressId` | 1 (`isApplyDressChange: bool`) | ✅ TLG (hooked) |

### `WorkSingleModeRaceData`
Working copy of race data.

### `WorkSingleModeScenarioFree`
Working copy of Free scenario data.

### `WorkSingleModeScenarioTeamRace`
Working copy of Team Race scenario data.

### `WorkTrainingChallengeData`
Working copy of Training Challenge data.

## Master Data (Database) Classes

These correspond to SQLite tables in `master.mdb`:

| Class/Table | Purpose | Indexed Queries |
|-------------|---------|-----------------|
| `masterSingleModeTraining` | Training facility definitions | by `commandId`, `commandType`, `commandId+commandLevel` |
| `masterSingleModeTrainingEffect` | Training effect definitions | by `commandId+resultState` |
| `masterSingleModeTrainingSe` | Training sound effects | by `sheetId` |
| `masterSingleModeTurn` | Turn definitions per scenario | by `turnSetId` |
| `masterSingleModeProgram` | Race program schedule | by `month` |
| `masterSingleModeRaceGroup` | Race groupings | by `raceGroupId`, `raceProgramId` |
| `masterSingleModeEvaluation` | Friendship thresholds | by `charaId` |
| `masterSingleModeCharaEffect` | Character effects | — |
| `masterSingleModeCharaGrade` | Character grade data | — |
| `masterSingleModeSkillNeedPoint` | Skill point costs | — |
| `masterSingleModeRoute` | Career route definitions | by `scenarioId`, `scenarioId+charaId` |
| `masterSingleModeRouteRace` | Route race definitions | by `raceSetId` |
| `masterSingleModeScenario` | Scenario definitions | — |
| `masterSingleModeDifficultyData` | Difficulty settings | by `difficultyId+difficultyIndex` |
| `masterSingleModeNpc` | NPC definitions | by `npcGroupId` |
| `masterSingleModeOuting` | Outing definitions | by `commandGroupId` |
| `masterSingleModeFanCount` | Fan count thresholds | by `fanSetId` |
| `masterSingleModeHintGain` | Hint gain data | by `hintId` |
| `masterSingleModeMessage` | In-game messages | — |
| `masterSingleModeRewardSet` | Reward definitions | by `rewardSetId` |
| `masterSingleModeStoryData` | Story data | by `storyId`, `cardId`, `cardCharaId`, etc. |
| `masterTrainingCuttCharaData` | Training cutscene chars | by `commandId+subId` |
| `masterTrainingCuttData` | Training cutscene data | by `commandId+subId` |

## Network Request/Response Classes

### Confirmed Request Types (Base Career)
| Class | Purpose |
|-------|---------|
| `SingleModeStartRequest` | Start career |
| `SingleModeCheckEventRequest` | Check turn events |
| `SingleModeExecCommandRequest` | Execute training/action |
| `SingleModeGainSkillsRequest` | Learn skills |
| `SingleModeRaceEntryRequest` | Enter a race |
| `SingleModeRaceStartRequest` | Start a race |
| `SingleModeRaceEndRequest` | End a race |
| `SingleModeRaceOutRequest` | Exit race |
| `SingleModeRaceReserveRequest` | Reserve a race |
| `SingleModeMultiRaceReserveRequest` | Reserve multi race |
| `SingleModeFinishRequest` | End career |
| `SingleModeContinueRequest` | Continue after failure |
| `SingleModeLoadRequest` | Load saved career |
| `SingleModeChangeRunningStyleRequest` | Change running style |
| `SingleModeChangeShortCutRequest` | Change shortcut |
| `SingleModeMinigameEndRequest` | End minigame |
| `SingleModeGetChoiceRewardRequest` | Get choice reward |
| `SingleModeSaveRaceResultRequest` | Save race result |

Each has a corresponding `*Response` and `*Task` class.

### Free Scenario Extensions
`SingleModeFree*` — Includes all base types plus:
- `SingleModeFreeChoiceRewardRequest`
- `SingleModeFreeMultiItemExchangeRequest` / `MultiItemExchange2Request`
- `SingleModeFreeMultiItemUseRequest`
- `SingleModeFreeRaceAnalyzeRequest`

### Team Scenario Extensions
`SingleModeTeam*` — Includes all base types plus:
- `SingleModeTeamOpponentListRequest`
- `SingleModeTeamRaceAnalyzeRequest`
- `SingleModeTeamTeamEditRequest`
- `SingleModeTeamTeamRaceAnalyzeRequest`
- `SingleModeTeamTeamRaceStartRequest` / `EndRequest` / `OutRequest`
- `SingleModeTeamSaveTeamEditFlagRequest`

## Classes Already Hooked by Hachimi Edge

The tables below list hooks relevant to career/training. For the full list of hooked modules, see `src/il2cpp/hook/umamusume/mod.rs` which initializes **34+ additional modules** covering graphics, story, camera, race, UI, and text rendering.

### Career / Training Hooks

| Class | Method | Purpose | Notes |
|-------|--------|---------|-------|
| `TrainingParamChangeA2U` | `GetCaptionText` | Training caption localization | Strips template filters |
| `TrainingParamChangePlate` | `PlayTypeWriteJp` (JP) / `PlayTypeWrite` (non-JP) | Training plate text | JP variant: 2 args (`message`, `skip_add_system_log`); non-JP: 1 arg (`message`) |
| `SingleModeUtils` | `GetMonthTextByTurn` | Month text formatting | Template context exposes `month` and `half` filters |
| `MasterSingleModeTurn.SingleModeTurn` | `get_Month`, `get_Half` | Turn calendar field accessors | These are field accessors on the nested `SingleModeTurn` class, not method hooks on `MasterSingleModeTurn` itself |
| `PartsSingleModeSkillListItem` | `UpdateItem`, `SetupOnClickSkillButton` | Skill list rendering | `UpdateItem` has region-specific overloads: JP takes 4 args, non-JP takes 2 args |
| `PartsSingleModeSkillLearningListItem` | `UpdateCurrent` | Skill learning text | |
| `PartsSingleModeChoiceRewardTextElementViewModel` | `GetParameterValueText` | Choice reward text | |

### General Hooks (also active during career)

| Class | Method | Purpose |
|-------|--------|--------|
| `Localize` | `Get` | General text localization |
| `LibNative.Sqlite3.Connection` | `Query`, `PreparedQuery` | SQLite query interception |
| `LibNative.Sqlite3.Query` | `GetText`, `Dispose` | SQLite text replacement |
| `LibNative.Sqlite3.PreparedQuery` | `BindInt` | SQLite parameter binding |

### Other Hooked Modules (non-exhaustive)

These are initialized in `src/il2cpp/hook/umamusume/mod.rs` and provide hooks for graphics, story, camera, race, and UI:

`ButtonCommon`, `CameraController`, `CameraData`, `CharacterNoteTopView`, `CharacterNoteTopViewController`, `CySpringController`, `DialogCommon`, `DialogCommonBase`, `DialogManager`, `DialogObject`, `DialogRaceOrientation`, `FlashActionPlayer`, `GallopUtil`, `GameSystem`, `GraphicSettings`, `ImageCommon`, `JikkyoDisplay`, `LiveTheaterCharaSelect`, `LiveTheaterViewController`, `LiveUtil`, `LowResolutionCamera`, `LyricsController`, `MasterDataUtil`, `MasterMissionData`, `NowLoading`, `PartsCommonHeaderTitle`, `PartsRaceAnalyzeRaceEventListItem`, `PaymentUtility`, `RaceInfo`, `RaceUtil`, `SaveDataManager`, `SceneManager`, `Screen`, `SingleModeStartResultCharaViewer`, `StoryChoiceController`, `StoryParamChangeEffect`, `StoryRaceTextAsset`, `StoryTimelineBlockData`, `StoryTimelineCharaTrackData`, `StoryTimelineClipData`, `StoryTimelineController`, `StoryTimelineData`, `StoryTimelineTextClipData`, `StoryTimelineTrackData`, `StoryViewController`, `StoryViewTextControllerLandscape`, `StoryViewTextControllerSingleMode`, `TextCommon`, `TextDotData`, `TextFontManager`, `TextFormat`, `TextFrame`, `TextId`, `TextMeshProUguiCommon`, `TimeUtil`, `TweenAnimationTimelineComponent`, `TweenAnimationTimelineData`, `TweenAnimationTimelineSheetData`, `UIManager`, `ViewControllerBase`, `WebViewDefine`, `WebViewManager`

## Appendix: Complete Training-Related String List

The following strings were extracted from the metadata string table matching training/career patterns. This is a subset of ~6,700 matches; see the full dump for complete results.

Key class names:
```
SingleModeMainViewController
TrainingView
TrainingController
TrainingMain
TrainingMenu
TrainingButton
TrainingTop
TrainingSelectDecide
TrainingParamChangeA2U
TrainingParamChangePlate
TrainingParamChangeUI
TrainingParamChangeSupportMemberA2U
TrainingDefine
TrainingCuttController
TrainingCuttData
TrainingEnvParam
TrainingModelController
WorkSingleModeData
WorkSingleModeHomeInfo
WorkSingleModeCharaData
```

Key method/property names:
```
OnClickTraining
OnDecide
OnClickSelect
OnClickStart
OnClickRace
get_SelectedTrainingCommandId
set_SelectedTrainingCommandId
get_TrainingCommandId
get_TrainingLevel
get_TrainingRank
get_IsInTraining
selectedCommandId
selectedTraining
```
