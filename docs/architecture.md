# Architecture

- **Core** (`src/core/`): Platform-agnostic — GUI (egui), plugin API, IL2CPP interceptor, game logic hooks
- **Windows** (`src/windows/`): DX11 render hook, window hook, DLL proxy, Steam integration
- **Android** (`src/android/`): Parallel platform impl — changes to render hook logic often need mirroring here
- **Plugins** (`plugins/`): External cdylib crates loaded at runtime via `load_libraries` in config.json
- **Plugin API** (`src/core/plugin/`): Plugin SDK domain module split by responsibility — `mod.rs` re-exports, `api.rs` owns the flat C ABI vtable and FFI wrappers, `types.rs` defines shared plugin types, and `overlay.rs`, `menu.rs`, and `notification.rs` own plugin GUI state. **Field order is ABI** — new vtable functions must be appended at the end only. Version field gates access to newer entries.
