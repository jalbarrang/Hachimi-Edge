# Architecture

- **Core** (`apps/hachimi/src/core/`): Platform-agnostic — GUI (egui), plugin API, IL2CPP interceptor, game logic hooks
- **Windows** (`apps/hachimi/src/windows/`): DX11 render hook, window hook, DLL proxy, Steam integration
- **Plugins** (`plugins/`): External cdylib crates loaded at runtime via `load_libraries` in config.json
- **Plugin API** (`apps/hachimi/src/core/plugin/`): Host-side FFI implementations in `api.rs`; wire types live in **`crates/hachimi-plugin-abi`** (`Vtable`, `API_VERSION = 9`, 42 slots). Plugins depend on `hachimi-plugin-abi` (required) and `hachimi-plugin-sdk` (recommended wrappers, re-exports the `#[hachimi_plugin]` macro from `hachimi-plugin-macros` and `egui`). Host depends on **abi only**, not sdk. Breaking the ABI bumps `API_VERSION`; the abi-layout test pins `VTABLE_SLOT_COUNT`.

### Why three plugin crates (`abi` / `sdk` / `macros`)

The "host" is the **`hachimi`** crate (`apps/hachimi`) itself — the `cdylib` injected into the game. It builds the `Vtable` in `apps/hachimi/src/core/plugin/api.rs` and hands it to each plugin's `hachimi_init`. Plugins are **separate cdylibs** loaded at runtime. So the host DLL and each plugin DLL are *independently compiled binaries that talk only through raw pointers and `#[repr(C)]` layout* — an FFI boundary, even though both may be written in this repo.

- **`hachimi-plugin-macros` is separate because Rust requires it.** A `proc-macro = true` crate can export *only* procedural macros, nothing else, so `#[hachimi_plugin]` physically cannot live alongside the `Vtable` or `Sdk`. Not optional.
- **`abi` is the shared contract — a C header written in Rust.** It is the single source of truth for the wire layout (`Vtable` field order, `API_VERSION`, `PluginManifest`, `event`/`capability` consts, `#[repr(C)]` payloads) that *both* sides must agree on byte-for-byte. The `size_of::<Vtable>() == 42 * size_of::<usize>()` layout test lives here. Having one host doesn't remove this need: there are still **two separately-built binaries across an FFI boundary**, and something has to pin the bytes. `abi` is deliberately tiny and dependency-light so both sides can compile against it cheaply.
- **`sdk` is the plugin-side ergonomics** (safe wrappers like `Sdk::hook/unhook/on`, `ui_from_ptr`, the `egui` re-export for version lockstep, and a re-export of the macro). It pulls in heavier deps (egui and its tree).

The dependency graph is asymmetric and points *up* at the small contract; neither end depends on the other:

```
            hachimi-plugin-abi   (shared header: Vtable, version, consts, payloads)
              ▲                ▲
   depends on │                │ depends on (re-exports abi + macro + egui)
   ┌──────────┴───────┐   ┌────┴───────────────┐
   │ HOST = hachimi   │FFI│ SDK → plugin cdylib │
   │ implements vtable│◄─►│ consumes vtable     │
   └──────────────────┘   └────────────────────┘
```

If the wire types lived in the host instead, every plugin would have to depend on the **entire host crate** (egui, render/window hooks, IL2CPP, Steam) just to learn the shape of a struct — an inverted, enormous dependency on a `cdylib` that isn't meant to be consumed as a library. The split keeps the contract minimal and stable while letting the host avoid the plugin-author conveniences it doesn't need.

### Plugin model (v9)

- **Entry points**: plugins export `hachimi_init(vtable, version) -> i32` and `hachimi_plugin_manifest() -> *const PluginManifest`. The `#[hachimi_plugin]` attribute generates both from a single `fn(&Sdk) -> Result<(), E: Display>`. The loader (`apps/hachimi/src/windows/main.rs`) reads the manifest for introspection/validation before init.
- **GUI = shared `egui::Ui`**: the host hands plugin menu/overlay/section callbacks a real `&mut egui::Ui` (as `*mut c_void`); plugins cast it via `hachimi_plugin_sdk::ui_from_ptr` and draw egui directly. There are **no per-widget vtable slots**. This relies on egui version lockstep — the SDK re-exports `egui`, and the whole repo is one workspace, so versions match.
- **Two-layer UI (L1 Control Center / L2 floating HUD)** — `apps/hachimi/src/core/gui/`:
  - **L1 Control Center** (`menu.rs` + `tabs/`): a hotkey-toggled `egui::Modal` (toggle key = `windows.menu_open_key`) with fixed top tabs **Settings · Plugins · Overlay · About**. *Settings* is the migrated Hachimi config UI; *Plugins* is a sub-nav that renders each registered **page** (menu section → selectable chip + body); *Overlay* manages L2 panels (per-panel show/hide, global lock, opacity, reset position); *About* is the inlined about content. The modal captures all input and dims the game while open. Replaced the old left `SidePanel`.
  - **L2 floating HUD** (`overlays.rs` + registry `apps/hachimi/src/core/plugin/overlay.rs`): each registered **panel** is a draggable `egui::Window` (`title_bar(false)`) that toggles between a compact **badge** (collapsed) and a full **panel** (expanded, with header → collapse/hide buttons). A **global lock** makes panels non-interactive (`interactable(false)`) → click-through; **opacity** is applied to the frame + content. Panel position, collapsed and visible state, plus the global lock/opacity persist to `overlay_state.json` (`PanelState`/`OverlayUiState`).
  - **Input/z-order contract** (`frame.rs` + `apps/hachimi/src/windows/wnd_hook.rs`): L1 open → `IS_CONSUMING_INPUT` true → full capture. L1 closed + overlays present → the wnd hook feeds mouse messages to egui for hover/drag tracking and **swallows** them only when `L2_WANTS_POINTER` is set (cursor over an unlocked panel; computed each frame from `ctx.is_pointer_over_area()`), otherwise the click falls through to the game. Locked panels keep `L2_WANTS_POINTER` false → click-through.
  - **Shared widgets** (`widgets.rs`): the canonical `toggle_ui` switch (egui ships only `Checkbox`/`RadioButton`) plus `toggle_row`/`section_header` helpers, used across L1 tabs and the host's dogfooded sections.
  - **SDK wrappers** (`crates/hachimi-plugin-sdk`): `Sdk::register_page`/`register_page_with_icon` (L1) and `Sdk::register_panel` (L2) are thin, clearly-named wrappers over the **existing** `gui_register_menu_section[_with_icon]` / `gui_register_overlay` slots — **no ABI break** (`API_VERSION` 9, 42 slots). `register_menu_section`/`register_overlay` remain as the lower-level names. The host dogfoods the same registries.
- **Events** (`apps/hachimi/src/core/plugin/events.rs`): plugins subscribe via `Sdk::on(event_id, cb, userdata) -> handle` over the `host_subscribe` slot. Event ids live in `hachimi_plugin_abi::event` and are **append-only — adding one needs no new vtable slot and no `API_VERSION` bump** (the abi-layout test stays 9/42). Current events:
  - `FRAME` — per frame, from `Gui::run_overlays`. Null `data`.
  - `CONFIG_RELOAD` — from `Hachimi::reload_config`/`save_and_reload_config`. Null `data`.
  - `SHUTDOWN` — process detach (`DllMain`) **and** per-plugin unload. Null `data`. A plugin that installed IL2CPP hooks MUST unhook here before its DLL can be freed.
  - `VIEW_CHANGE` — from `SceneManager::ChangeViewCommon`. `data` → `ViewChangeEvent { view_id }`.
  - `SPLASH_SHOWN` — fired once when the splash view first appears. Null `data`.
  - `TRAINING_COMMAND` — from the host hook on `SingleModeMainViewController.SendCommandAsync(6)` (arg1 = `command_id`). `data` → `TrainingCommandEvent { command_id }`. Per the IL2CPP dump this method returns `System.Collections.IEnumerator` (a coroutine kickoff), so the hook **must forward that return value** — a `void` hook leaves garbage in the return register and the game crashes in `StartCoroutine`.
  - `CAREER_START`/`CAREER_END` — from `apps/hachimi/src/core/plugin/career.rs`, which re-checks the `WorkDataManager → get_SingleMode() → get_IsPlaying()` chain on each `VIEW_CHANGE` (career boundaries always coincide with a view transition; lazy-resolved, no per-frame cost) and emits `IsPlaying` transitions. Null `data`.

  Events carrying `data` point it at the matching `#[repr(C)]` payload struct in the abi crate, valid for the callback duration only. Callbacks are snapshotted under a short lock and each wrapped in `catch_unwind`. Events let plugins observe game lifecycle without re-implementing their own IL2CPP hooks — `training-tracker` subscribes to `TRAINING_COMMAND`/`CAREER_*` instead of hooking `SendCommandAsync` itself.
- **Capabilities**: `Sdk::capabilities()/has_capability()` (the `host_capabilities` slot) expose a bitset (`GUI`/`OVERLAY`/`EVENTS`/`IL2CPP`) for graceful feature detection, independent of the monotonic `API_VERSION`. `capability::UNLOADABLE` is a *plugin-declared* flag set in the manifest `requested_caps`.
- **Registration handles & ownership**: `gui_register_*` slots return a non-zero `u64` handle (0 = failure); `Sdk::unregister(handle)` (the `gui_unregister` slot) removes a menu item/section/overlay. Handles come from a shared counter in `apps/hachimi/src/core/plugin/mod.rs`. Every registration and event subscription is tagged with an owning plugin id via an `OwnerScope` guard set during init and host→plugin callbacks, so the host can reclaim a plugin's callbacks on unload.
- **Unload / reload** (`apps/hachimi/src/windows/main.rs`): `unload_plugin(name)` runs `teardown_owner` (dispatches `SHUTDOWN` to that plugin's subscriptions, then drops its GUI/event registrations). It only `FreeLibrary`s plugins that opted in with `capability::UNLOADABLE`; others are merely disconnected (DLL kept mapped, since the host can't remove a plugin's IL2CPP hooks). `reload_plugin` requires `UNLOADABLE` (a fresh DLL mapping is needed to reset plugin statics). `core::plugin::reload_all()` is wired to a Danger Zone GUI button. **Native cdylib hot-reload is fundamentally limited**: a plugin must unhook all IL2CPP hooks in its `SHUTDOWN` handler or freeing it will crash on the next game call into freed code.
