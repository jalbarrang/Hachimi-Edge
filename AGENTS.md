# Agent Instructions

## Tech stack

- Rust 2021, stable, Cargo workspace (root `Cargo.toml`).
- **Platform:** Windows Steam the Honse game only (no Android).
- **Core:** `cargo build --release -p hachimi` → `hachimi.dll` → game dir as `cri_mana_vpx.dll`.
- **Plugins:** `cargo build --release -p hachimi-training-tracker` → game dir; `config.json` → `windows.load_libraries`.
- **UI / hooks:** egui 0.33, `egui-directx11`, minhook, pelite, `windows` crate.
- **Workspace:** default members exclude `apps/installer` (use `-p hachimi_installer` after artifact staging).
- **CI commands:** `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --lib` (no game process). Run fmt/clippy in `plugins/training-tracker/` when editing that crate.

## Project facts

- Fork of Hachimi; Windows/Steam translation & enhancement mod for the Honse game.
- Plugin host API **v9** (`hachimi_plugin_abi::API_VERSION`). Upstream Hachimi plugins do not load; use DLLs built from this repo only.
- Default game dir: `C:\Program Files (x86)\Steam\steamapps\common\UmamusumePrettyDerby`.
- Stock `cri_mana_vpx.dll` → back up once as `cri_mana_vpx.dll.backup`; mod replaces `cri_mana_vpx.dll`.
- Settings: `config.json` in the game **data** directory (not the install folder).
- Deploy: `.\scripts\deploy-windows.ps1 -Build`; `$env:HACHIMI_GAME_DIR` overrides game path.
- License: GPL-3.0-or-later (`apps/installer`: MIT).

## Hard Rules

- **Never launch the game** (`steam://rungameid`, game executables, etc.). Copying DLLs is allowed.
- **Never kill game processes** (`taskkill`, etc.).
- **Never modify** `cri_mana_vpx.dll.backup`.
- **Naming:** Never write the game's real, spelled-out name in prose, docs, comments, or commit messages. Always call it **"the Honse game"** (to avoid search-engine parsing). This does NOT apply to load-bearing identifiers that must match the game/OS: the `umamusume.dll` assembly name, `UmamusumePrettyDerby*` folder/exe/window-class names, CDN/API URLs, and the Rust `umamusume` module / IL2CPP class names — leave those exactly as-is.
