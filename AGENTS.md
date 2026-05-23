# Agent Guidelines — Hachimi Edge

## What This Project Is

Hachimi Edge is a game enhancement mod for **Uma Musume: Pretty Derby** (Steam). It works by DLL proxying (`cri_mana_vpx.dll`) and hooking IL2CPP methods at runtime. It has a plugin system where external DLLs (cdylib) interact via a C ABI vtable.

## Hard Rules

- **Never launch the game.** Do not run `steam://rungameid`, start executables, or invoke any command that launches the game process. Deployment (copying DLLs) is fine; running the game is the user's job.
- **Never kill game processes.** Do not use `taskkill` or equivalent on game processes.
- **Never modify the backup DLL** at `cri_mana_vpx.dll.backup` in the game directory.

## Architecture

- **Core** (`src/core/`): Platform-agnostic — GUI (egui), plugin API, IL2CPP interceptor, game logic hooks
- **Windows** (`src/windows/`): DX11 render hook, window hook, DLL proxy, Steam integration
- **Android** (`src/android/`): Parallel platform impl — changes to render hook logic often need mirroring here
- **Plugins** (`plugins/`): External cdylib crates loaded at runtime via `load_libraries` in config.json
- **Plugin API** (`src/core/plugin_api.rs`): Flat C ABI vtable struct. **Field order is ABI** — new functions must be appended at the end only. Version field gates access to newer entries.

## Build & Deploy

- **Core**: `cargo build --release` → `target/release/hachimi.dll`
- **Plugin**: `cargo build --release` from `plugins/training-tracker/` → `plugins/training-tracker/target/release/hachimi_training_tracker.dll`
- **Deploy core**: Copy `target/release/hachimi.dll` as `C:/Program Files (x86)/Steam/steamapps/common/UmamusumePrettyDerby/cri_mana_vpx.dll`
- **Deploy plugin**: Copy plugin DLL to the game directory root
- **Config**: `hachimi/config.json` in the game directory. `menu_open_key: 68` (D key). Plugins listed in `load_libraries`.

## Key Patterns

- **Render hook gating**: `gui.is_empty()` in `src/windows/gui_impl/render_hook.rs` (and Android equivalent) controls whether the entire egui pass runs. Anything that should render must make `is_empty()` return `false`.
- **IL2CPP hooks**: Use `usize` for all pointer-typed arguments in hook signatures (not `i32`). IL2CPP object pointers are 64-bit on Windows.
- **Unsafe code**: This codebase is heavily `unsafe` (IL2CPP FFI, raw pointers, transmute). Be precise with pointer types and ABI.
- **egui overlays**: Use `egui::Area` with `interactable(false)` so overlays don't capture game input.

## Logs

- `hachimi.log` in the game directory when `enable_file_logging: true`
- Check logs after game runs to verify hooks installed, plugins loaded, overlays registered

## Research Docs

- `docs/reverse-engineering/` — IL2CPP class maps, training system architecture, network protocol, TLG cross-reference
- These inform hook targets and plugin development

## Active Issues (beads)

- `Hachimi-Edge-1vp` (P0): Overlay rendering fix — built & deployed, awaiting user verification
- `Hachimi-Edge-1bq` (P1): Plugin SDK refactor into domain-driven architecture
- `Hachimi-Edge-5ht` (P1): Rust code quality tooling (clippy, strict lints, CI)
