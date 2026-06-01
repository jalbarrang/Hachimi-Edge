# Key Patterns

- **Render hook gating**: `gui.is_empty()` in `apps/hachimi/src/windows/gui_impl/render_hook.rs` controls whether the entire egui pass runs. Anything that should render must make `is_empty()` return `false`.
- **IL2CPP hooks**: Use `usize` for all pointer-typed arguments in hook signatures (not `i32`). IL2CPP object pointers are 64-bit on Windows.
- **Unsafe code**: This codebase is heavily `unsafe` (IL2CPP FFI, raw pointers, transmute). Be precise with pointer types and ABI.
- **egui overlays**: Use `egui::Area` with `interactable(false)` so overlays don't capture game input.
- **Plugin domain state**: Plugin-owned state lives in `apps/hachimi/src/core/plugin/` submodules (`overlay`, `menu`, `notification`); the GUI reads it through `pub(crate)` getters instead of owning it, and plugin callbacks are wrapped in `catch_unwind` at render call sites.
