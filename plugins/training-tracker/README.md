# Hachimi Training Tracker Plugin

A [Hachimi Edge](../../README.md) plugin that tracks how many times each training facility has been visited during a career run, displaying live counts in the in-game overlay.

![Plugin Type: Hachimi Native Plugin](https://img.shields.io/badge/type-hachimi_plugin-blue)

## Features

- **Live training counter** — Tracks Speed, Stamina, Power, Guts, and Wisdom facility visits
- **In-game overlay** — Displays a color-coded table with visit counts and visual bars in Hachimi's menu
- **Reset button** — Clear counts mid-run if needed
- **Multi-scenario support** — Handles command IDs from URA, UAF, Onsen, and other career scenarios
- **Auto-detection** — Attempts to find and hook the training method at runtime by name

## Building

```bash
cd plugins/training-tracker

# Windows (produces hachimi_training_tracker.dll)
cargo build --release

# Android (requires NDK, produces libhachimi_training_tracker.so)
cargo ndk -t arm64-v8a build --release
```

## Installation

### Windows
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

### Android
1. Build the plugin (see above)
2. Place `libhachimi_training_tracker.so` beside `libmain.so`, or name it `libhachimi_training_tracker.so` for auto-discovery
3. Alternatively, add to config:
   ```json
   {
     "android": {
       "load_libraries": ["libhachimi_training_tracker.so"]
     }
   }
   ```

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

1. Run [Il2CppDumper](https://github.com/Perfare/Il2CppDumper) on your game's `GameAssembly.dll` or `libil2cpp.so`
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

Same as [Hachimi Edge](../../LICENSE) — GNU GPLv3.
