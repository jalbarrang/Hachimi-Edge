# Build & Deploy

## Build

The repo is a Cargo workspace with a virtual root manifest and dedicated folders:
`apps/*` (deployables: `apps/hachimi`, `apps/installer`), `crates/*` (libraries),
`plugins/*` (plugins). Build from the repo root:

- **Core**: `cargo build --release -p hachimi` → `target/release/hachimi.dll`
- **Plugin ABI tests**: `cargo test -p hachimi-plugin-abi`
- **Training tracker plugin**: `cargo build --release -p hachimi-training-tracker` → `target/release/hachimi_training_tracker.dll`
- **Installer** (Windows): vendored MIT fork in `apps/installer/`. It's kept out of
  `default-members`, so build it explicitly and only after staging the binaries it
  embeds (`hachimi.dll`, `cellar.dll`, `FunnyHoney.exe`, and — with the
  `training_tracker` feature — `hachimi_training_tracker.dll` + `skill_grades.json`)
  into `apps/installer/`:
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

Plugin-only deploy (skip the core proxy):

```powershell
.\scripts\deploy-windows.ps1 -PluginOnly -Build
```

Hot-swap while the game is running (unload → copy → reload via IPC; requires `enable_ipc: true` in config.json). Do **not** also click **Reload plugins** afterward — the script already reloads the plugin.

```powershell
.\scripts\deploy-windows.ps1 -PluginOnly -HotSwap -Build
```

After updating the host (for IPC unload/reload support), deploy core once and restart the game before relying on hot-swap.

### Manual

- **Deploy core**: Copy `target/release/hachimi.dll` as `C:/Program Files (x86)/Steam/steamapps/common/UmamusumePrettyDerby/cri_mana_vpx.dll`
- **Deploy plugin**: Copy `target/release/hachimi_training_tracker.dll` to the game directory root
- **Config**: `config.json` in the game data directory. `menu_open_key: 68` (D key). Plugins listed in `windows.load_libraries`.

## Versioning & Releases

The release tag is derived from the `[package] version` in `apps/hachimi/Cargo.toml`
(see `.github/workflows/create_release.yml`). The release itself is **manual**: you
trigger the `Create Release` workflow via `workflow_dispatch` on GitHub.

### Bumping the version (script)

`scripts/bump-version.sh` computes the next version from conventional-commit history
since the last `v*` tag and writes it into `apps/hachimi/Cargo.toml` (also refreshing
`Cargo.lock`). It uses standard semver — breaking→major, `feat`→minor, `fix`/other→patch
(configured in `cliff.toml`).

Prerequisites (one-time):

```bash
cargo install git-cliff
cargo install cargo-edit
```

Run it from anywhere in the repo:

```bash
./scripts/bump-version.sh
```

The script only edits files — it does **not** commit, tag, or push. After running:

1. Review the diff and commit (e.g. `chore(release): vX.Y.Z`), then push to `main`.
2. Trigger the `Create Release` workflow on GitHub (it reads the version from
   `main`, tags `vX.Y.Z`, builds, and publishes).

If only non-bumping commits exist since the last tag, the script reports that no
bump is warranted and makes no changes.
