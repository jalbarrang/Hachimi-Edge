# Build & Deploy

## Build

The repo is a Cargo workspace (root + `crates/*` + `plugins/training-tracker`). Build from the repo root:

- **Core**: `cargo build --release -p hachimi` → `target/release/hachimi.dll`
- **Plugin ABI tests**: `cargo test -p hachimi-plugin-abi`
- **Training tracker plugin**: `cargo build --release -p hachimi-training-tracker` → `target/release/hachimi_training_tracker.dll`
- **ABI guardrail** (optional): `scripts/check-plugin-api.sh`

## Deploy

- **Deploy core**: Copy `target/release/hachimi.dll` as `C:/Program Files (x86)/Steam/steamapps/common/UmamusumePrettyDerby/cri_mana_vpx.dll`
- **Deploy plugin**: Copy plugin DLL to the game directory root
- **Config**: `hachimi/config.json` in the game directory. `menu_open_key: 68` (D key). Plugins listed in `load_libraries`.
