# Build & Deploy

## Build

The repo is a Cargo workspace (root + `crates/*` + `plugins/training-tracker` + `installer`). Build from the repo root:

- **Core**: `cargo build --release -p hachimi` → `target/release/hachimi.dll`
- **Plugin ABI tests**: `cargo test -p hachimi-plugin-abi`
- **Training tracker plugin**: `cargo build --release -p hachimi-training-tracker` → `target/release/hachimi_training_tracker.dll`
- **Installer** (Windows): vendored MIT fork in `installer/`. It's kept out of
  `default-members`, so build it explicitly and only after staging the binaries it
  embeds (`hachimi.dll`, `cellar.dll`, `FunnyHoney.exe`, and — with the
  `training_tracker` feature — `hachimi_training_tracker.dll` + `skill_grades.json`)
  into `installer/`:
  `cargo build --release -p hachimi_installer --features compress_bin,training_tracker`
  → `target/release/hachimi_installer.exe`. The release workflow does this staging
  automatically; those embedded files are gitignored.

The plugin ABI is guarded automatically: the host's `build_host_vtable` is a `Vtable`
struct literal, so any slot mismatch is a compile error, and `abi_layout.rs`
(`cargo test -p hachimi-plugin-abi`) pins `API_VERSION`, vtable size, and `Copy`-ness.

## Deploy

### Windows (script)

From the repo root (builds optionally with `-Build`):

```powershell
.\scripts\deploy-windows.ps1 -Build
```

Override the game folder:

```powershell
$env:HACHIMI_GAME_DIR = "D:\path\to\UmamusumePrettyDerby"
.\scripts\deploy-windows.ps1 -Build
```

The script copies `hachimi.dll` → `cri_mana_vpx.dll` and `hachimi_training_tracker.dll` into the game directory. It never modifies `cri_mana_vpx.dll.backup`.

### Manual

- **Deploy core**: Copy `target/release/hachimi.dll` as `C:/Program Files (x86)/Steam/steamapps/common/UmamusumePrettyDerby/cri_mana_vpx.dll`
- **Deploy plugin**: Copy `target/release/hachimi_training_tracker.dll` to the game directory root
- **Config**: `config.json` in the game data directory. `menu_open_key: 68` (D key). Plugins listed in `windows.load_libraries`.
