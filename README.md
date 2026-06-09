<img align="left" width="80" height="80" src="apps/hachimi/assets/icon.png">

# HachimiRedux

English | [简体中文](README-zh_cn.md) | [繁體中文](README-zh_tw.md)

[![Discord server](https://dcbadge.limes.pink/api/server/https://discord.gg/YjBgmuqqYr)](https://discord.gg/YjBgmuqqYr)

Game enhancement and translation mod for UM:PD. HachimiRedux is a fork of Hachimi with an in-game training tracker plugin and a reworked native plugin SDK.

<img height="400" src="apps/hachimi/assets/screenshot-2.png">

# ⚠️ Please don't link to this repo or Hachimi's website
We understand that you want to help people install Hachimi and have a better experience playing the game. However, this project is inherently against the game's TOS and The Game Developer most definitely wants it gone if they were ever to learn about it.

While sharing in your self-managed chat services and through private messaging is fine, we humbly ask that you refrain from sharing links to this project on public facing sites, or to any of the tools involved.

Or share them and ruin it for the dozens of Hachimi users. It's up to you.

### If you're going to share it anyways
Do what you must, but we would respectfully request that you try to label the game as "UM:PD" or "The Honse Game" instead of the actual name of the game, to avoid search engine parsing.

# ⚠️ Incompatible with upstream Hachimi plugins
This fork ships its own native plugin API (host API v9). **Plugins built for upstream Hachimi are not compatible with HachimiRedux**, and the training tracker plugin distributed here will not load on upstream Hachimi. Prefer DLLs built from this repository, and use them together. Mixing builds can fail to load or crash the game.

## Legacy plugin compatibility (opt-in, limited)
Manifest-less, legacy-ABI plugins (e.g. upstream Hachimi data-dumpers) can be loaded through an **opt-in compatibility path**. List the DLL under a `legacy_libraries` allowlist in `config.json`, in addition to `load_libraries`:

```json
{
  "windows": {
    "load_libraries": ["some_legacy_plugin.dll"],
    "legacy_libraries": ["some_legacy_plugin.dll"]
  }
}
```

A legacy plugin only needs to export `hachimi_init`; the host skips its usual manifest/ABI check and loads it on trust. This support is **limited and unsupported**:

- The plugin must rely **only on the stable vtable prefix** of the host API. Anything beyond it is undefined behaviour and can crash the game.
- The host **cannot validate, track, or unload** a legacy plugin or its IL2CPP hooks. The DLL stays mapped for the lifetime of the process.
- A warning is logged whenever a plugin loads via this path.
- Entries in `legacy_libraries` must also appear in `load_libraries`.

When in doubt, rebuild the plugin against this repository (host API v9) instead of relying on the legacy path.

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

HachimiRedux is a fork built on the work of:

- [Hachimi](https://github.com/Hachimi-Hachimi/Hachimi) — the original project this is based on.
- [Hachimi Edge](https://github.com/kairusds/Hachimi-Edge) — the Windows/Steam-focused fork HachimiRedux continues from.

These projects have in turn been the basis for Hachimi's development; without them, Hachimi would never have existed in its current form:

- [Trainers' Legend G](https://github.com/MinamiChiwa/Trainers-Legend-G)
- [umamusume-localify-android](https://github.com/Kimjio/umamusume-localify-android)
- [umamusume-localify](https://github.com/GEEKiDoS/umamusume-localify)
- [Carotenify](https://github.com/KevinVG207/Uma-Carotenify)
- [umamusu-translate](https://github.com/noccu/umamusu-translate)
- [frida-il2cpp-bridge](https://github.com/vfsfitvnm/frida-il2cpp-bridge)

# License
[GNU GPLv3](LICENSE)
