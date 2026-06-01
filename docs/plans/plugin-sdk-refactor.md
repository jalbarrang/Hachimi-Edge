# Plugin SDK Domain-Driven Refactor Plan

**Issue**: HachimiRedux-1bq  
**Status**: Planning complete — ready for phased implementation  
**Date**: 2026-05-23

---

## 1. Current State Audit

### Code Map

| Location | What lives there | Lines | Problem |
|----------|-----------------|-------|---------|
| `apps/hachimi/src/core/plugin_api.rs` | Flat vtable (53 fn pointers), VERSION=3, `Plugin` struct, all FFI wrapper fns | ~580 | Monolithic — hook API, UI API, platform API, logging all in one `#[repr(C)]` struct |
| `apps/hachimi/src/core/gui.rs` | 5 plugin statics (`PLUGIN_MENU_ITEMS`, `PLUGIN_MENU_SECTIONS`, `PLUGIN_MENU_ICONS`, `PLUGIN_NOTIFICATIONS`, `PLUGIN_OVERLAYS`), 8 pub registration fns, rendering in `run_menu()` + `run_overlays()`, `is_empty()` checks overlays | ~200 lines of plugin code in a 3100-line GUI module | Plugin state management mixed into GUI module |
| `apps/hachimi/src/windows/main.rs` | `load_libraries()` — `LoadLibraryW` + `GetProcAddress("hachimi_init")` | ~30 | Platform-specific loading, no shared trait |
| `src/android/plugin_loader.rs` | `load_libraries()` — `dlopen` + `dlsym`, autoscan `libhachimi_*.so` | ~110 | Same — platform-specific, no shared interface |
| `apps/hachimi/src/core/hachimi.rs` | `plugins: Mutex<Vec<Plugin>>`, init loop in `hooking_finished()` | ~15 | Thin, but Plugin struct is too basic (no deinit, no metadata) |
| `plugins/training-tracker/src/vtable.rs` | Complete mirror of the flat vtable | ~170 | Must be manually kept in sync — no generated bindings |

### Vtable Structure (current)

```
Vtable (53 fields, #[repr(C)]):
  ├── Core (2):        hachimi_instance, hachimi_get_interceptor
  ├── Hook API (4):    interceptor_hook/hook_vtable/get_trampoline/unhook
  ├── IL2CPP API (23): resolve_symbol, get_class, get_method, fields, threads, arrays...
  ├── Logging (1):     log
  ├── GUI Menu (6):    register_menu_item/section, show_notification, register_icon variants
  ├── GUI Widgets (11): heading, label, button, checkbox, text_edit, horizontal, grid...
  ├── Android DEX (4): dex_load/unload/call_static_noargs/call_static_string
  └── Overlay (1):     gui_register_overlay (v3+)
```

**Version gating**: Linear integer (VERSION=3). Plugins check `version >= N` to decide which tail fields are safe to access. This is fragile — adding fields in the middle breaks ABI.

### Cross-cutting Concerns

1. **`is_empty()` gating**: Render hooks in both `apps/hachimi/src/windows/gui_impl/render_hook.rs:86` and `src/android/gui_impl/render_hook.rs:59` skip the entire egui pass when `gui.is_empty()` returns true. Plugin overlays must keep `is_empty()` returning false — the recent overlay fix (HachimiRedux-1vp) added `PLUGIN_OVERLAYS` to this check.

2. **Thread safety**: All plugin statics use `Lazy<Mutex<...>>`. Registration fns are called from the plugin's `hachimi_init` (on the DllMain/attach thread). Rendering fns are called from the render thread. Current pattern: snapshot via `get_plugin_*()` (clone under lock), render from snapshot.

3. **Panic boundaries**: Plugin callbacks are wrapped in `catch_unwind(AssertUnwindSafe(...))` at call sites in `run_menu()`. Overlay callbacks in `run_overlays()` are NOT panic-caught — this is a bug to fix during refactor.

---

## 2. Target Architecture

### Module Structure

```
apps/hachimi/src/core/
  ├── plugin/
  │   ├── mod.rs           — re-exports, PluginManager, plugin init orchestration
  │   ├── api.rs           — C ABI vtable (Vtable struct, VERSION, FFI wrappers)
  │   ├── types.rs         — Plugin, InitResult, HachimiInitFn, callback type aliases
  │   ├── overlay.rs       — PluginOverlay, PLUGIN_OVERLAYS, registration + snapshot fns
  │   ├── menu.rs          — PluginMenuItem, PluginMenuSection, PluginMenuIcon, PLUGIN_MENU_*, registration fns
  │   └── notification.rs  — PLUGIN_NOTIFICATIONS, enqueue/drain fns
  ├── gui.rs               — Gui struct, rendering only (calls into plugin::overlay, plugin::menu)
  └── ...
```

### Domain Boundaries

| Domain | Owns | Exposed to `gui.rs` |
|--------|------|---------------------|
| **`plugin::api`** | Vtable struct, VERSION, all `unsafe extern "C" fn` wrappers | Nothing — api.rs calls into other plugin submodules |
| **`plugin::types`** | `Plugin`, `InitResult`, `HachimiInitFn`, callback type aliases | `Plugin` used by hachimi.rs and platform loaders |
| **`plugin::overlay`** | `PLUGIN_OVERLAYS`, `PluginOverlay`, `register_plugin_overlay()`, `get_plugin_overlays()`, `has_plugin_overlays()` | `get_plugin_overlays()` for rendering, `has_plugin_overlays()` for `is_empty()` |
| **`plugin::menu`** | `PLUGIN_MENU_ITEMS`, `PLUGIN_MENU_SECTIONS`, `PLUGIN_MENU_ICONS`, all menu registration fns | `get_plugin_menu_items()`, `get_plugin_menu_sections()`, `get_plugin_menu_icon()` |
| **`plugin::notification`** | `PLUGIN_NOTIFICATIONS`, `enqueue_plugin_notification()`, `drain_plugin_notifications()` | `drain_plugin_notifications()` |
| **`gui.rs`** | Rendering, `Gui` struct, `is_empty()`, `run_overlays()`, `run_menu()` plugin sections | Calls plugin module read-only accessors |

### Key Design Decisions

1. **Vtable stays flat for now** — The existing 53-field `#[repr(C)]` struct is the shipped ABI. Restructuring it into sub-vtables would break every existing plugin. The refactor reorganizes the *Rust side* (module structure, state management) without changing the C ABI wire format.

2. **Sub-vtable design deferred to v4** — When VERSION bumps to 4, we can append a `sub_vtables: *const SubVtables` pointer at the end. Existing v3 plugins won't access it. New plugins get domain-separated sub-tables. This is a future phase.

3. **Overlay positioning stays host-controlled** — Overlays render at a fixed anchor (`RIGHT_TOP`). Plugin-controlled positioning is a v4+ feature that requires extending the overlay registration API.

4. **Mediated widget API continues** — Exposing raw `egui::Context` would tie the ABI to egui's Rust layout. The mediated `gui_ui_*` functions remain the stable interface.

5. **Panic boundaries added to overlays** — `run_overlays()` currently lacks `catch_unwind`. The refactor adds it, matching the pattern in `run_menu()`.

---

## 3. Migration Plan — 5 Phases

Each phase is a single PR that builds, passes all 81 tests, and maintains zero clippy warnings.

### Phase 1: Extract plugin types (`plugin::types`)
**Risk**: Low  
**Changes**:
- Create `apps/hachimi/src/core/plugin/mod.rs` and `apps/hachimi/src/core/plugin/types.rs`
- Move `Plugin`, `InitResult`, `HachimiInitFn`, callback type aliases from `plugin_api.rs` → `plugin/types.rs`
- `plugin/mod.rs` re-exports everything publicly
- Update `apps/hachimi/src/core/mod.rs`: replace `pub mod plugin_api` with `pub mod plugin`
- Update 3 import sites (`hachimi.rs`, `windows/main.rs`, `android/plugin_loader.rs`) from `plugin_api::Plugin` → `plugin::Plugin`
- `plugin_api.rs` → `plugin/api.rs` (rename + move)

**Acceptance criteria**:
- `cargo build --target x86_64-pc-windows-msvc` succeeds
- `cargo clippy` zero warnings
- `cargo test` all 81 pass
- No changes to `#[repr(C)] Vtable` layout — byte-for-byte identical ABI

### Phase 2: Extract plugin overlay state (`plugin::overlay`)
**Risk**: Medium — touches `is_empty()` and render hook gating  
**Changes**:
- Create `apps/hachimi/src/core/plugin/overlay.rs`
- Move from `gui.rs`: `PluginOverlay` struct, `PLUGIN_OVERLAYS` static, `register_plugin_overlay()`, `get_plugin_overlays()`
- Add `has_plugin_overlays() -> bool` (replaces inline `PLUGIN_OVERLAYS.lock().map_or(true, |o| o.is_empty())` in `is_empty()`)
- Add `catch_unwind` to overlay callback invocation in `run_overlays()`
- `gui.rs` imports from `plugin::overlay` instead of owning the state
- `plugin/api.rs` calls `plugin::overlay::register_plugin_overlay()` (already does via `gui::register_plugin_overlay`, just redirect)

**Acceptance criteria**:
- `is_empty()` behavior identical (test: register overlay → `is_empty()` returns false)
- Overlay rendering unchanged
- Panic in overlay callback caught and logged, not propagated to render hook

### Phase 3: Extract plugin menu state (`plugin::menu`)
**Risk**: Medium — largest extraction, many functions  
**Changes**:
- Create `apps/hachimi/src/core/plugin/menu.rs`
- Move from `gui.rs`: `PluginMenuItem`, `PluginMenuSection`, `PluginMenuIcon`, `PLUGIN_MENU_ITEMS`, `PLUGIN_MENU_SECTIONS`, `PLUGIN_MENU_ICONS`, all registration fns, all getter fns
- Move callback type aliases `PluginMenuCallback`, `PluginMenuSectionCallback` to `plugin/types.rs` (shared by menu + overlay)
- `gui.rs` `run_menu()` calls `plugin::menu::get_*()` — rendering stays in gui.rs
- `plugin/api.rs` FFI wrappers call `plugin::menu::*` instead of `gui::*`

**Acceptance criteria**:
- Menu plugin items render identically
- Plugin section with icon works
- Zero compile warnings

### Phase 4: Extract plugin notifications (`plugin::notification`)
**Risk**: Low  
**Changes**:
- Create `apps/hachimi/src/core/plugin/notification.rs`
- Move from `gui.rs`: `PLUGIN_NOTIFICATIONS`, `enqueue_plugin_notification()`, `drain_plugin_notifications()`
- `gui.rs` `run_menu()` calls `plugin::notification::drain_plugin_notifications()`
- `plugin/api.rs` `gui_show_notification` wrapper calls `plugin::notification::enqueue()`

**Acceptance criteria**:
- Plugin notifications still appear
- Zero warnings

### Phase 5: Documentation + cleanup
**Risk**: Low  
**Changes**:
- Update `docs/architecture.md` with new module structure
- Update `docs/patterns.md` with plugin domain pattern
- Add module-level doc comments to each `plugin/*.rs` file
- Update `plugins/training-tracker/` README if needed (vtable.rs stays identical — no ABI change)
- Create follow-up issue for v4 sub-vtable design

**Acceptance criteria**:
- Docs match code
- `cargo doc` builds cleanly
- Follow-up issue created for sub-vtable design

---

## 4. Risk Analysis

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| ABI break — vtable layout changes | Low (plan explicitly forbids it) | Critical — breaks all existing plugins | Phase 1 acceptance: byte-identical vtable. CI can add a vtable size assertion test. |
| `is_empty()` regression — overlays not gating render hook | Medium (Phase 2 moves the check) | High — render hook skips frames, overlays invisible | Add unit test: register overlay → `is_empty()` returns false |
| Deadlock — lock ordering change | Low (all locks are independent Lazy<Mutex>) | High | No lock ordering changes in plan — each static is independently locked, no nested locks |
| Plugin callback panic propagation | Already a bug (overlays lack `catch_unwind`) | Medium — render thread panic = crash | Fixed in Phase 2 |
| Import path breakage in platform modules | Low | Low — only 3 import sites, compile error caught immediately | Mechanical update, no shim needed |

---

## 5. Vtable Size Assertion (add in Phase 1)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn vtable_size_is_stable() {
        // 53 function pointers × 8 bytes (64-bit) = 424 bytes
        assert_eq!(
            std::mem::size_of::<Vtable>(),
            53 * std::mem::size_of::<usize>(),
            "Vtable size changed — this breaks plugin ABI!"
        );
    }
    
    #[test]
    fn vtable_is_copy() {
        // Vtable must be Copy for C ABI compatibility
        fn assert_copy<T: Copy>() {}
        assert_copy::<Vtable>();
    }
}
```

---

## 6. What This Plan Does NOT Do

- **Does not change the C ABI** — The `#[repr(C)] Vtable` struct keeps all 53 fields in the same order. Existing plugins work unchanged.
- **Does not introduce sub-vtables** — That's a v4 design decision requiring a new issue.
- **Does not add new plugin capabilities** — No config API, no event bus, no plugin-controlled overlay positioning. Those are separate issues.
- **Does not touch platform loading code** — `windows/main.rs::load_libraries()` and `android/plugin_loader.rs::load_libraries()` stay in their platform modules. A shared `PluginLoader` trait is a separate follow-up.

---

## 7. Dependency Graph

```
Phase 1 (types + module structure)
  ↓
Phase 2 (overlay) ←──── can start after Phase 1
  ↓
Phase 3 (menu)    ←──── can start after Phase 1 (parallel with Phase 2 if careful)
  ↓
Phase 4 (notification) ← after Phase 3 (shares run_menu changes)
  ↓
Phase 5 (docs + cleanup) ← after all above
```

Phases 2 and 3 could theoretically be parallelized (they extract from different statics in gui.rs), but sequential is safer to avoid merge conflicts in gui.rs imports.

---

## 8. Estimated Effort

| Phase | Estimated effort | Files touched |
|-------|-----------------|---------------|
| Phase 1 | ~1 hour | 6 files (new: 2, modified: 4) |
| Phase 2 | ~1 hour | 4 files (new: 1, modified: 3) |
| Phase 3 | ~1.5 hours | 4 files (new: 1, modified: 3) |
| Phase 4 | ~30 min | 4 files (new: 1, modified: 3) |
| Phase 5 | ~1 hour | 4-5 files (docs + cleanup) |
| **Total** | **~5 hours** | |
