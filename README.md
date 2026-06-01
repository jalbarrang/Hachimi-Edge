<img align="left" width="80" height="80" src="apps/hachimi/assets/icon.png">

# HachimiRedux

English | [简体中文](README-zh_cn.md) | [繁體中文](README-zh_tw.md)

[![Discord server](https://dcbadge.limes.pink/api/server/https://discord.gg/YjBgmuqqYr)](https://discord.gg/YjBgmuqqYr)

Game enhancement and translation mod for UM:PD. HachimiRedux is a fork of Hachimi with an in-game training tracker plugin and a reworked native plugin SDK.

<img height="400" src="apps/hachimi/assets/screenshot.jpg">

# ⚠️ Please don't link to this repo or Hachimi's website
We understand that you want to help people install Hachimi and have a better experience playing the game. However, this project is inherently against the game's TOS and The Game Developer most definitely wants it gone if they were ever to learn about it.

While sharing in your self-managed chat services and through private messaging is fine, we humbly ask that you refrain from sharing links to this project on public facing sites, or to any of the tools involved.

Or share them and ruin it for the dozens of Hachimi users. It's up to you.

### If you're going to share it anyways
Do what you must, but we would respectfully request that you try to label the game as "UM:PD" or "The Honse Game" instead of the actual name of the game, to avoid search engine parsing.

# ⚠️ Incompatible with upstream Hachimi plugins
This fork ships its own native plugin API (host API v9). **Plugins built for upstream Hachimi will not load with HachimiRedux**, and the training tracker plugin distributed here will not load on upstream Hachimi. Only use the DLLs built from this repository together. Mixing builds can fail to load or crash the game.

# Features
- **High quality translations:** Hachimi comes with advanced translation features that help translations feel more natural (plural forms, ordinal numbers, etc.) and prevent introducing jank to the UI. It also supports translating most in-game components; no manual assets patching needed!

    Supported components:
    - UI text
    - master.mdb (skill name, skill desc, etc.)
    - Race story
    - Main story/Home dialog
    - Lyrics
    - Texture replacement
    - Sprite atlas replacement

    Additionally, Hachimi does not provide translation features for only a single language; it has been designed to be fully configurable for any language.

- **Easy setup:** Just plug and play. All setup is done within the game itself, no external application needed.
- **Translation auto update:** Built-in translation updater lets you play the game as normal while it updates, and reloads it in-game when it's done, no restart needed!
- **Built-in GUI:** Comes with a config editor so you can modify settings without even exiting the game!
- **Graphics settings:** You can adjust the game's graphics settings to make full use of your device's specs, such as FPS unlocking and resolution scaling.
- **Windows only:** Built specifically for the Windows (Steam) version of the game. **HachimiRedux does not support Android by choice** — it focuses solely on the Windows client, and there are no plans to add or maintain an Android build.

# Installation

HachimiRedux is the core mod (loaded as `cri_mana_vpx.dll`); the **Training Tracker** is an optional plugin DLL loaded by the core mod. Both are built from this repository and must come from the same build.

The game directory is the Steam install folder, e.g.
`C:\Program Files (x86)\Steam\steamapps\common\UmamusumePrettyDerby`.

## Build from source

This repo is a Cargo workspace. From the repo root:

```sh
# Core mod
cargo build --release -p hachimi                    # -> target/release/hachimi.dll
# Training Tracker plugin
cargo build --release -p hachimi-training-tracker   # -> target/release/hachimi_training_tracker.dll
```

## Install HachimiRedux (core)

The game loads the mod through the renderer DLL `cri_mana_vpx.dll`.

1. In the game directory, back up the original `cri_mana_vpx.dll` to `cri_mana_vpx.dll.backup` (do this once — never overwrite the backup afterwards).
2. Copy `target/release/hachimi.dll` into the game directory and rename it to `cri_mana_vpx.dll`.
3. Launch the game. Press the menu key (default **D**, keycode `68`) to open the in-game UI.

Mod settings live in `config.json` inside the game data directory; everything else is configured from the in-game GUI.

## Install the Training Tracker plugin

Plugins are native DLLs the core mod loads at startup from the game directory root.

1. Install the HachimiRedux core first (above).
2. Copy `target/release/hachimi_training_tracker.dll` into the game directory root (the same folder as `cri_mana_vpx.dll` / `config.json`).
3. Add the DLL to the `load_libraries` list in `config.json`:

   ```json
   {
     "windows": {
       "load_libraries": ["hachimi_training_tracker.dll"]
     }
   }
   ```
4. Launch the game. The tracker appears as a page in the Plugins tab and as a floating overlay panel. See [docs/plugin-sdk.md](docs/plugin-sdk.md) for how plugins work.

## Automated deploy (Windows, from source)

From the repo root, the helper script builds and copies both DLLs into the game directory:

```powershell
.\scripts\deploy-windows.ps1 -Build
```

Override the game folder if it is not at the default Steam path:

```powershell
$env:HACHIMI_GAME_DIR = "D:\path\to\UmamusumePrettyDerby"
.\scripts\deploy-windows.ps1 -Build
```

The script copies `hachimi.dll` → `cri_mana_vpx.dll` and the training tracker DLL into the game directory, and never modifies `cri_mana_vpx.dll.backup`.

# Special thanks
These projects have been the basis for Hachimi's development; without them, Hachimi would never have existed in its current form:

- [Trainers' Legend G](https://github.com/MinamiChiwa/Trainers-Legend-G)
- [umamusume-localify-android](https://github.com/Kimjio/umamusume-localify-android)
- [umamusume-localify](https://github.com/GEEKiDoS/umamusume-localify)
- [Carotenify](https://github.com/KevinVG207/Uma-Carotenify)
- [umamusu-translate](https://github.com/noccu/umamusu-translate)
- [frida-il2cpp-bridge](https://github.com/vfsfitvnm/frida-il2cpp-bridge)

# License
[GNU GPLv3](LICENSE)

# Copyright and Fair Use Notice

Uma Musume: Pretty Derby, its characters, names, artwork, game assets, and related trademarks are the property of
**Cygames, Inc.** and their respective rights holders.

This project is an independent, fan-made enhancement and translation tool. It is not affiliated with, endorsed by, or
sponsored by Cygames, Inc.

Any referenced game data, terminology, or limited derivative material is used for commentary, research, education,
and interoperability purposes. This repository is intended to fall under applicable **fair use / fair dealing**
principles and equivalent exceptions under relevant copyright laws.
