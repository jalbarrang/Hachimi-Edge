# Quality of Life Enhancement Opportunities

Feasibility analysis of potential game enhancements based on confirmed IL2CPP classes, Hachimi plugin capabilities, and community research.

## Tier 1 — High Feasibility (Data Accessible, UI Available)

### 1. Training Facility Hit Counter ✅ (In Progress)
**Status**: Plugin built (`plugins/training-tracker/`)

Show how many times each training facility has been visited during a career run.

- **Hook**: Candidate targets include `SingleModeMainViewController.OnClickTraining(commandId)` or `TrainingSelectDecide.OnDecide(commandId)` (confirmed in metadata; exact signatures need runtime verification)
- **Display**: Hachimi menu section with color-coded grid
- **Data**: `command_id` from hook argument, mapped to facility index

---

### 2. Training Stat Gain Preview Enhancement
Show the **exact stat numbers** being gained at each facility in a more readable format, potentially as a persistent overlay rather than only on hover.

- **Hook**: Read `command_info_array` from `SingleModeHomeInfo` each turn
- **Data**: `SingleModeCommandInfo.params_inc_dec_info_array` gives exact stat deltas per facility
- **Display**: Overlay on training buttons or a summary panel
- **Feasibility**: ⭐⭐⭐⭐⭐ — Data is already in the server response, accessible via IL2CPP field reads

---

### 3. Support Card Friendship Tracker
Display current friendship values for all support cards at a glance, with indicators for who's close to friendship training threshold (80+).

- **Hook**: Read `evaluation_info_array` from `SingleModeChara` (contains friendship per partner)
- **Data**: Each `EvaluationInfo` has `target_id` and `evaluation` (friendship value)
- **Display**: Color-coded list (red < 60, yellow 60-79, green 80+)
- **Feasibility**: ⭐⭐⭐⭐⭐ — Data is in `chara_info` every turn, friendship is already tracked by UmamusumeResponseAnalyzer

---

### 4. Turn Counter / Calendar Enhancement
Show a clearer calendar view of the career with current turn, remaining turns, upcoming mandatory races, and scenario milestones.

- **Hook**: Already partially hooked by Hachimi (`SingleModeUtils.GetMonthTextByTurn`)
- **Data**: `SingleModeChara.turn`, `MasterSingleModeTurn.SingleModeTurn` for month/half mapping, `route_race_id_array` for mandatory races
- **Display**: Enhanced month display or a mini calendar panel
- **Feasibility**: ⭐⭐⭐⭐⭐ — Turn data is readily available

---

### 5. Failure Rate Display
Always show the training failure rate for each facility, not just on hover. Highlight dangerous training (high failure chance).

- **Hook**: Read `failure_rate` from `SingleModeCommandInfo` per facility
- **Data**: `command_info_array[i].failure_rate` — integer percentage
- **Display**: Badge on training buttons or summary panel
- **Feasibility**: ⭐⭐⭐⭐⭐ — Available in `home_info.command_info_array`

---

### 6. Stat Cap Warning
Alert the player when a stat is about to hit its cap, making further training in that stat wasteful.

- **Hook**: Read `SingleModeChara` stats vs max stats each turn
- **Data**: Compare `speed` vs `max_speed`, `stamina` vs `max_stamina`, etc.
- **Display**: Warning icon on facilities that would push a stat past cap, or notification
- **Feasibility**: ⭐⭐⭐⭐⭐ — Both current and max values are on `SingleModeChara`

---

## Tier 2 — Medium Feasibility (Requires Some RE Work)

### 7. Smart Training Recommendation
Suggest the optimal training facility each turn based on stat needs, support card positions, friendship gains, and failure rate.

- **Data needed**: All of Tier 1 data combined — stat gains, partners, friendship, failure rate, stat caps
- **Logic**: Score each facility based on weighted factors
- **Display**: Highlight or rank training buttons
- **Feasibility**: ⭐⭐⭐⭐ — Data is available; the challenge is defining good heuristics
- **Reference**: UmamusumeResponseAnalyzer already does similar analysis in its AI mode

---

### 8. Support Card Position Tracker
Show where each support card has appeared across turns, helping predict future positions and plan friendship training.

- **Hook**: Read `training_partner_array` from each `SingleModeCommandInfo` every turn
- **Data**: Track support card deck position → facility mapping across turns (entries are deck slot IDs 1–6 for player cards, >1000 for NPCs)
- **Display**: History table or heatmap
- **Feasibility**: ⭐⭐⭐⭐ — Requires accumulating data across turns (state management)

---

### 9. Race Schedule Planner
Show all available and upcoming races with their rewards, goals, and conflicts with training. Highlight which races the character has good aptitude for.

- **Data**: `race_condition_array` from `SingleModeCheckEventResponse`, `SingleModeRaceCondition` struct, aptitude data from `SingleModeChara`
- **Hook**: Read race data each turn
- **Display**: Scrollable race list with aptitude indicators
- **Feasibility**: ⭐⭐⭐⭐ — Race data is in the response, aptitude data is on the character

---

### 10. Skill Point Budget Tracker
Track total skill points earned and spent, show remaining budget, and estimate if the player can afford key target skills.

- **Data**: `skill_point` on `SingleModeChara`, `skill_array` for learned skills, `masterSingleModeSkillNeedPoint` for costs
- **Hook**: Read each turn, track delta
- **Display**: Running total with projections
- **Feasibility**: ⭐⭐⭐⭐ — Skill point data is accessible; cost tables need SQLite query hooking

---

### 11. Character Effect Status Display
Show active character effects (buffs/debuffs) in a readable format instead of requiring the player to navigate menus.

- **Data**: `chara_effect_id_array` on `SingleModeChara`, mapped to `masterSingleModeCharaEffect` for names/descriptions
- **Display**: Icon bar or status panel
- **Feasibility**: ⭐⭐⭐⭐ — Effect IDs are available; need to resolve names from master data

---

### 12. Motivation (Mood) History
Track mood changes across the career to help the player understand what's causing drops and plan rest/outing timing.

- **Data**: `SingleModeChara.motivation` (1-5 scale)
- **Hook**: Record each turn
- **Display**: Graph or timeline
- **Feasibility**: ⭐⭐⭐⭐ — Simple integer tracking per turn

---

## Tier 3 — Lower Feasibility (Significant RE Required)

### 13. Event Choice Optimizer
For branching events, show the expected outcomes of each choice (which stats change, what buffs are gained).

- **Data**: Requires parsing `SingleModeEventInfo` and cross-referencing with master data tables (likely `masterSingleModeEventConclusion` and `masterSingleModeEventChoiceReward` — names inferred from metadata patterns, not individually verified)
- **Challenge**: Event outcomes vary by context; need full event database cross-referencing
- **Reference**: GameTora's Training Event Helper does this externally
- **Feasibility**: ⭐⭐⭐ — Complex data relationships; better done with an external database

---

### 14. Inheritance Factor Viewer
Show detailed information about inheritance factors from parent characters before and during the career.

- **Data**: `FactorInfo`, `FactorExtendInfo`, `SuccessionEffectedFactor` (class names confirmed in UmamusumeResponseAnalyzer protocol definitions; presence in IL2CPP metadata not individually verified)
- **Challenge**: Factor activation is probabilistic and depends on multiple conditions
- **Feasibility**: ⭐⭐⭐ — Data structures likely exist but complex to display meaningfully

---

### 15. Live Race Stat Overlay
During races, show the character's real-time stats, skill activations, and positioning data.

- **Data**: Race simulation happens server-side; `RaceHorseData` and `RaceHorseDataRaceResult` contain results
- **Challenge**: Real-time race state isn't exposed to the client in a hookable way
- **Feasibility**: ⭐⭐ — Would require deep hooks into the race rendering system

---

### 16. Auto-Training Mode
Automatically select the optimal training each turn based on a configurable strategy.

- **Challenge**: Requires sending `SingleModeExecCommandRequest` programmatically, which modifies gameplay behavior
- **Risk**: Very likely to trigger anti-cheat or cause bans
- **Feasibility**: ⭐⭐ — Technically possible but high risk, and against the spirit of the game
- **Note**: OCR-based tools like [UmaTrainerTools](https://github.com/suchxs/UmaTrainerTools) already do this externally

---

## Implementation Priority

Recommended order based on player impact and development effort:

| Priority | Enhancement | Effort | Impact |
|----------|-------------|--------|--------|
| 1 | Training Facility Hit Counter | Done ✅ | Medium |
| 2 | Failure Rate Display | Low | High |
| 3 | Support Card Friendship Tracker | Low | High |
| 4 | Stat Cap Warning | Low | High |
| 5 | Training Stat Gain Preview | Medium | High |
| 6 | Turn Counter / Calendar | Low | Medium |
| 7 | Motivation History | Low | Medium |
| 8 | Character Effect Display | Medium | Medium |
| 9 | Smart Training Recommendation | High | Very High |
| 10 | Skill Point Budget Tracker | Medium | Medium |
| 11 | Support Card Position Tracker | Medium | Medium |
| 12 | Race Schedule Planner | Medium | Medium |

## Technical Notes

### Data Access Pattern

Most enhancements follow the same pattern:

1. Hook a method that fires each turn (e.g., `SingleModeCheckEventResponse` handler)
2. Read fields from the `SingleModeChara` and `SingleModeHomeInfo` objects via IL2CPP field access
3. Store the data in plugin-local state
4. Render via `gui_register_menu_section` or overlay

### Shared Infrastructure

Several enhancements share common needs:

- **Per-turn data capture**: A single hook that reads the full turn state could feed multiple features
- **Master data lookup**: Resolving IDs to names requires SQLite queries or cached dictionaries
- **State persistence**: Tracking data across turns requires in-memory state; tracking across careers requires file I/O

A **multi-feature plugin** that captures turn state once and feeds it to multiple display modules would be more efficient than separate plugins for each enhancement.

### Existing Hooks to Leverage

Hachimi already hooks several useful methods that plugins can piggyback on:

- `TrainingParamChangePlate.PlayTypeWrite` — Fires after training, gives access to the stat change text
- `SingleModeUtils.GetMonthTextByTurn` — Fires each turn, gives turn/month context
- `Localize.Get` — Text localization; can be used to intercept any UI string
- SQLite query hooks — Can intercept master data lookups
