# Architecture

- **Core** (`src/core/`): Platform-agnostic — GUI (egui), plugin API, IL2CPP interceptor, game logic hooks
- **Windows** (`src/windows/`): DX11 render hook, window hook, DLL proxy, Steam integration
- **Plugins** (`plugins/`): External cdylib crates loaded at runtime via `load_libraries` in config.json
- **Plugin API** (`src/core/plugin/`): Host-side FFI implementations in `api.rs`; wire types live in **`crates/hachimi-plugin-abi`** (`Vtable`, `API_VERSION = 8`, 53 slots). Plugins depend on `hachimi-plugin-abi` (required) and `hachimi-plugin-sdk` (recommended wrappers). Host depends on **abi only**, not sdk. **Field order is ABI** — append new vtable functions at the end and bump `API_VERSION`.
