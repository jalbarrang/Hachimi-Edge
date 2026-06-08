# Hachimi Training Tracker Plugin

A [HachimiRedux](../../README.md) plugin that reads live career state from game
memory (IL2CPP) and renders a training-decision overlay: per-facility stat gains,
failure rates, a self-computed career evaluation (評価点), bonds, a skill shop
advisor, and a **smart per-turn training recommendation**.

![Plugin Type: Hachimi Native Plugin](https://img.shields.io/badge/type-hachimi_plugin-blue)

## Features

- **Live career snapshot** — Stats, caps, energy/mood, aptitudes, failure rates, and per-stat training gains read directly from game memory
- **Smart recommendation** — Scores each facility per turn and flags the best (★), with a Rest/Race fallback when every option is too risky
- **Selectable objective** — Optimize for career **Rank (評価点)**, **Champions Meeting race power (CM)**, or a **Hybrid** blend
- **CM race model** — Threshold-aware marginal-value scoring (stamina survival floor, 1200 soft cap, power knee) for a chosen course + running style
- **Build profiles & presets** — Curated CM presets plus a manual editor (per-stat targets + weights, course/strategy), saveable as custom profiles
- **Multi-turn planning** — Optional lookahead layer valuing energy/rest, early bond-building, and late-career stat-maxing
- **Self-computed evaluation** — Reproduces the 評価点 stat curve (validated against real careers)
- **Bonds & skill shop** — Support-card bond/event-chain progress and a skill-purchase advisor
- **Multi-scenario support** — Command IDs from URA, Unity Cup, Trackblazer, UAF, Onsen, and other scenarios
- **Configurable overlay** — Toggle tabs, tune recommendation/planner knobs; settings persist to `training_config.json`

## Building

```bash
cd plugins/training-tracker

# Windows (produces hachimi_training_tracker.dll)
cargo build --release
```

## Installation

1. Build the plugin (see above)
2. Copy `target/release/hachimi_training_tracker.dll` next to the game executable
3. Add to Hachimi config (`hachimi.json`):
   ```json
   {
     "windows": {
       "load_libraries": ["hachimi_training_tracker.dll"]
     }
   }
   ```

The plugin's data resources (`skill_grades.json` for the evaluation estimate,
`course_params.json` for the CM model) are **not** bundled with the DLL: the host
downloads them into the game data dir on launch (the `hosted_data` sync, like the
GameTora catalog) from the repo's published `data/`. For local development,
`scripts/deploy-windows.ps1` drops them next to the DLL as a fallback.

## How It Works

1. On plugin init, the UI section is registered with Hachimi's menu system
2. The plugin attempts to resolve training-related IL2CPP methods by name:
   - `Gallop.SingleModeViewController.OnSelectCommand`
   - `Gallop.SingleModeMainViewController.OnSelectCommand`
   - `Gallop.SingleModeViewController.OnClickTraining`
   - `Gallop.SingleModeViewController.ExecCommand`
   - `Gallop.SingleModeTrainingView.OnClickCommandButton`
3. The first method found gets hooked; when the player selects a training facility, the `command_id` is mapped to a facility and counted
4. The overlay shows live counts with colored bars

## Updating Hook Targets

The hook candidates in `src/hooks.rs` are educated guesses based on community research (particularly [UmamusumeResponseAnalyzer](https://github.com/UmamusumeResponseAnalyzer/UmamusumeResponseAnalyzer)). If none resolve:

1. Run [Il2CppDumper](https://github.com/Perfare/Il2CppDumper) on your game's `GameAssembly.dll`
2. Search the `dump.cs` output for training-related classes in the `Gallop` namespace
3. Look for methods that take a `command_id` or `command_type` parameter
4. Update the `candidates` array in `src/hooks.rs`

### Key search terms for the dump:
```
SingleMode
Training
ExecCommand
CommandType
CommandId
OnClickTraining
```

## Command ID Reference

| command_id | Facility | Scenario |
|-----------|----------|----------|
| 101, 601, 1101 | Speed | URA/base |
| 105, 602, 1102 | Stamina | URA/base |
| 102, 603, 1103 | Power | URA/base |
| 103, 604, 1104 | Guts | URA/base |
| 106, 605, 1105 | Wisdom | URA/base |
| 2101, 2201, 2301 | Speed | UAF/Sport |
| 2102, 2202, 2302 | Stamina | UAF/Sport |
| 2103, 2203, 2303 | Power | UAF/Sport |
| 2104, 2204, 2304 | Guts | UAF/Sport |
| 2105, 2205, 2305 | Wisdom | UAF/Sport |
| 901 | Speed | Onsen |
| 902 | Power | Onsen |
| 906 | Wisdom | Onsen |

## License

Same as [HachimiRedux](../../LICENSE) — GNU GPLv3.
