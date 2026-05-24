# Developing Training Tracker

Build from the **repo root** (workspace member):

```bash
cargo build -p hachimi-training-tracker
cargo build --release -p hachimi-training-tracker
cargo clippy -p hachimi-training-tracker -- -D warnings
```

Output: `target/release/hachimi_training_tracker.dll` (Windows).

## Dependencies

- `hachimi-plugin-abi` — `Vtable`, `vt()`, `hlog_*` macros
- `hachimi-plugin-sdk` — `Sdk::init`, `ApiVersion`, safe GUI/IL2CPP/hook helpers

Copy the release DLL to the game directory and list it in `hachimi/config.json` → `load_libraries`.
