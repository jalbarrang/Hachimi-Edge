# Plugin SDK Guide

Hachimi plugins are native shared libraries (`.dll`) loaded at runtime. They talk to the host through a C ABI vtable, but you almost never touch it directly: the **`hachimi-plugin-sdk`** crate wraps the vtable in a safe `Sdk` handle, and the **`#[hachimi_plugin]`** attribute macro generates the C entry points for you.

The [`training-tracker`](../plugins/training-tracker/) plugin is the reference implementation and is used as the example throughout this guide.

## Architecture

Three crates make up the plugin stack:

| Crate | Role |
|-------|------|
| `hachimi-plugin-abi` | Raw `#[repr(C)]` vtable, `PluginManifest`, event/capability/log constants, `hlog_*` macros. The ABI contract — never reorder fields. |
| `hachimi-plugin-sdk` | Safe `Sdk` wrapper around the vtable, the `#[hachimi_plugin]` macro re-export, and the shared `egui` re-export. **This is what plugins depend on.** |
| `hachimi-plugin-macros` | The proc-macro implementing `#[hachimi_plugin]`. Pulled in transitively via the SDK. |

The current host API version is **9** (`hachimi_plugin_abi::API_VERSION`).

---

## Quick Start

### 1. Create a cdylib crate

```toml
# Cargo.toml
[lib]
# `cdylib` produces the loadable DLL; `lib` lets other crates/tests reuse the code.
crate-type = ["cdylib", "lib"]

[dependencies]
# Distributed via git tags (not crates.io). Pin a SDK release tag.
hachimi-plugin-sdk = { git = "https://github.com/kairusds/Hachimi-Edge", tag = "sdk-v0.1.0" }
# Needed directly only for the hlog_* macros + event/capability constants.
hachimi-plugin-abi = { git = "https://github.com/kairusds/Hachimi-Edge", tag = "sdk-v0.1.0" }
```

> **In-tree plugins** (inside this workspace, like `training-tracker`) use `path =`
> dependencies instead — see `plugins/training-tracker/Cargo.toml`. The git form above
> is for external plugins. See [plugin-sdk-release.md](plugin-sdk-release.md) for the
> tag scheme and the egui-version-match requirement.

### 2. Write your init function

The `#[hachimi_plugin]` macro generates both `hachimi_init` and `hachimi_plugin_manifest` C exports from a single Rust function. Your function takes `&Sdk` and returns `Result<(), E>` where `E: Display`.

```rust
#[macro_use]
extern crate hachimi_plugin_abi; // brings the hlog_* macros into scope

use hachimi_plugin_sdk::{hachimi_plugin, Sdk};

#[hachimi_plugin(name = "training-tracker", caps = hachimi_plugin_sdk::capability::UNLOADABLE)]
fn init(sdk: &Sdk) -> Result<(), &'static str> {
    hlog_info!(
        target: "training-tracker",
        "Training Tracker v{} initializing (host API v{})",
        env!("CARGO_PKG_VERSION"),
        sdk.version().raw()
    );

    ui::register_ui();          // register GUI pages / overlay panels
    hooks::subscribe_events();  // subscribe to host lifecycle events

    sdk.show_notification("Training Tracker loaded!");
    Ok(())
}
```

When `init` returns `Err`, the macro logs the error and reports failure to the host automatically. By the time `init` runs, `Sdk::get()` is already valid — use it anywhere in your plugin.

#### Macro arguments

All optional:

| Arg | Default | Meaning |
|-----|---------|---------|
| `name` | `CARGO_PKG_NAME` | Plugin name in the manifest/logs. |
| `version` | `CARGO_PKG_VERSION` | Plugin version string. |
| `min_api` | SDK's `API_VERSION` | Minimum host API required. The macro refuses to init if the host is older. |
| `caps` | `0` | Plugin-declared capability flags, e.g. `capability::UNLOADABLE`. |

### 3. Configure loading

Add your DLL to the game's `config.json`:

```json
{
  "windows": {
    "load_libraries": ["hachimi_training_tracker.dll"]
  }
}
```

### 4. Deploy

Copy the built DLL to the game directory root (same folder as `config.json` and `cri_mana_vpx.dll`). See [build-and-deployment.md](build-and-deployment.md) or use `scripts/deploy-windows.ps1`.

---

## The `Sdk` Handle

`Sdk` is a global, lazily-initialized handle installed by the macro before your `init` runs. Get it anywhere with:

```rust
let sdk = Sdk::get();          // panics if init hasn't run
let sdk = Sdk::try_get();      // Option, for code that may run pre-init
```

It exposes safe methods grouped by domain (full list in `crates/hachimi-plugin-sdk/src/`):

| Module | Methods |
|--------|---------|
| `sdk.rs` | version/capabilities, events (`on`/`off`), logging, GUI registration (`register_page`, `register_panel`, `unregister`) |
| `gui.rs` | `show_notification`, `overlay_set_visible`, `ui_from_ptr` |
| `hook.rs` | `hook`, `unhook`, `trampoline_addr`, `interceptor` |
| `il2cpp.rs` | `get_assembly_image`, `get_class`, `get_method_addr`, `get_field_from_name`, `schedule_on_main_thread`, … |

### Version & capability gating

Check once at init, not per call:

```rust
if sdk.version().at_least(9) { /* use v9 features */ }

if sdk.has_capability(hachimi_plugin_abi::capability::EVENTS) {
    // safe to subscribe to host events
}
```

Host capability bits: `GUI`, `OVERLAY`, `EVENTS`, `IL2CPP`.

---

## GUI

With host API v9 the host hands plugins the **real `egui::Ui`** inside callbacks — there are no per-widget vtable slots. You draw with `egui` directly. Plugins MUST use the SDK's re-exported `egui` (`hachimi_plugin_sdk::egui`) so the version matches the host exactly.

The UI is a two-layer system:

- **L1 — page**: a selectable entry in the Control Center's Plugins tab. Register with `register_page` / `register_page_with_icon`.
- **L2 — panel**: a draggable, collapsible floating HUD overlay drawn over the game. Register with `register_panel` (needs a stable string id).

```rust
use std::ffi::c_void;
use std::panic::{self, AssertUnwindSafe};
use hachimi_plugin_sdk::{egui, ui_from_ptr, Sdk};

pub fn register_ui() {
    let sdk = Sdk::get();
    sdk.register_page(draw_menu_section, std::ptr::null_mut());
    sdk.register_panel("training_tracker_overlay", draw_overlay, std::ptr::null_mut());
}

extern "C" fn draw_menu_section(ui: *mut c_void, _userdata: *mut c_void) {
    // SAFETY: host passes its live `&mut egui::Ui` for this callback only.
    let ui = unsafe { ui_from_ptr(ui) };
    // Wrap in catch_unwind: panicking across FFI is UB.
    let _ = panic::catch_unwind(AssertUnwindSafe(|| {
        ui.heading("Training Tracker");
        if ui.button("Reset").clicked() {
            Sdk::get().show_notification("Reset!");
        }
    }));
}

extern "C" fn draw_overlay(ui: *mut c_void, _userdata: *mut c_void) {
    let ui = unsafe { ui_from_ptr(ui) };
    let _ = panic::catch_unwind(AssertUnwindSafe(|| {
        ui.small("Speed: 3  Stamina: 1");
    }));
}
```

Each `register_*` returns a non-zero handle (0 = failure). Keep it to call `sdk.unregister(handle)` later, and `sdk.overlay_set_visible(id, bool)` to toggle a panel.

### Page with title + icon

```rust
sdk.register_page_with_icon(
    "Training Tracker",          // title in the Plugins sub-nav
    "bytes://tracker-icon",      // icon URI
    icon_png_bytes,              // PNG data (host copies it)
    draw_menu_section,
    std::ptr::null_mut(),
);
```

### Notifications

```rust
sdk.show_notification("Plugin loaded successfully!");
```

Queued and shown on the next render frame. Safe from any thread.

---

## Events

Subscribe to host lifecycle events (requires the `EVENTS` capability). Event ids are append-only and don't require an API bump.

```rust
use std::ffi::c_void;
use hachimi_plugin_abi::{capability, event};
use hachimi_plugin_sdk::Sdk;

pub fn subscribe_events() -> bool {
    let sdk = Sdk::get();
    if !sdk.has_capability(capability::EVENTS) {
        hlog_warn!("Host does not advertise EVENTS");
        return false;
    }
    sdk.on(event::SHUTDOWN, on_shutdown, std::ptr::null_mut());
    true
}

extern "C" fn on_shutdown(_event_id: u32, _data: *const c_void, _userdata: *mut c_void) {
    // Remove every IL2CPP hook we installed so the host can FreeLibrary us.
    crate::shop_hooks::uninstall_shop_hooks();
}
```

`on` returns a handle; call `sdk.off(handle)` to unsubscribe.

Available events: `FRAME`, `CONFIG_RELOAD`, `SHUTDOWN`, `VIEW_CHANGE` (→ `ViewChangeEvent`), `CAREER_START`, `CAREER_END`, `TRAINING_COMMAND` (→ `TrainingCommandEvent`), `SPLASH_SHOWN`. When `data` is non-null it points at the matching `#[repr(C)]` payload — copy what you need; don't retain the pointer past the callback.

---

## Hooking IL2CPP Methods

Resolve method addresses through the SDK, then install a detour.

```rust
let sdk = Sdk::get();

// 1. Resolve the method address
let image = sdk.get_assembly_image("umamusume.dll")?;
let klass = sdk.get_class(image, "Gallop", "SomeClass")?;
let addr  = sdk.get_method_addr(klass, "SomeMethod", 2)?; // 2 = arg count

// 2. Install the hook -> returns the trampoline to call the original
let trampoline = sdk.hook(addr, my_hook as *mut c_void)?;

// 3. Store the trampoline (e.g. in an AtomicPtr / Mutex)
ORIG.store(trampoline, Ordering::SeqCst);

// 4. Hook function — must match the original's calling convention
extern "C" fn my_hook(this: usize, arg1: usize, arg2: usize) {
    // ... your logic ...
    let orig = ORIG.load(Ordering::SeqCst);
    let orig: extern "C" fn(usize, usize, usize) = unsafe { std::mem::transmute(orig) };
    orig(this, arg1, arg2);
}
```

**Use `usize` for all pointer-typed arguments** — IL2CPP object pointers are 64-bit on Windows; `i32` truncates them.

Remove hooks with `sdk.unhook(my_hook as *mut c_void)`. If your plugin declares `capability::UNLOADABLE`, you **must** unhook every hook from your `SHUTDOWN` handler so the host can safely `FreeLibrary` the DLL. Verify hook signatures (return type + arg count) before detouring — see [reverse-engineering/il2cpp-signatures.md](reverse-engineering/il2cpp-signatures.md).

To run code on Unity's main thread, use `sdk.schedule_on_main_thread(callback)` with an `extern "C" fn()`.

---

## Logging

Use the `hlog_*` macros from `hachimi-plugin-abi` (bring them in with `#[macro_use] extern crate hachimi_plugin_abi;`). They route through the host logger into the same file as Hachimi's own logs.

```rust
hlog_info!(target: "training-tracker", "Ready (host API v{})", sdk.version().raw());
hlog_warn!("Something looks off");        // default target "plugin"
hlog_error!(target: "training-tracker", "Hook failed");
```

Levels: `hlog_error!` / `hlog_warn!` / `hlog_info!` / `hlog_debug!` / `hlog_trace!`. You can also call `sdk.log_info/warn/error(target, msg)` directly.

---

## Userdata Pattern

All callback registration functions accept a `*mut c_void` userdata pointer passed back on every invocation. Use it to avoid global state:

```rust
struct MyState { counter: u32 }

let state = Box::into_raw(Box::new(MyState { counter: 0 }));
sdk.register_page(draw_fn, state as *mut c_void);

extern "C" fn draw_fn(ui: *mut c_void, userdata: *mut c_void) {
    let state = unsafe { &mut *(userdata as *mut MyState) };
    state.counter += 1;
}
```

**You own the memory** — the host never frees it. Many plugins (including the training tracker) instead keep state in `Mutex`/`OnceLock` statics and pass `null_mut()`.

---

## Thread Safety

- **Registration & notifications** (`register_*`, `show_notification`, `on`/`off`) — safe from any thread; they take internal locks.
- **GUI drawing** (`ui_from_ptr` + egui calls) — only inside a callback; the `Ui` belongs to the render thread and is valid for that invocation only.
- **Hook / IL2CPP methods** — safe from any thread once `init` returns.
- **Callbacks** run on the render thread — keep them fast; blocking stalls the frame.

---

## Panic Safety

Panicking across an FFI boundary is undefined behavior. Mark callbacks `extern "C"` and wrap their bodies in `std::panic::catch_unwind(AssertUnwindSafe(...))`, as the training tracker does. Log and swallow the panic rather than letting it cross back into the host.

---

## Reference Implementation

[`plugins/training-tracker/`](../plugins/training-tracker/) is a complete working plugin demonstrating:

- `#[hachimi_plugin]` entry point with `UNLOADABLE` capability (`src/lib.rs`)
- L1 page + L2 overlay panel registration and egui drawing (`src/ui/`)
- `SHUTDOWN` event subscription and hook teardown (`src/hooks.rs`)
- IL2CPP method resolution and detour hooks (`src/shop_hooks.rs`)
- Reading career state directly from game memory (`src/memory_reader/`)
- State management with statics (`src/ui/state.rs`, `src/memory_reader/`)

---

## Host Module Structure

For contributors working on the host side, plugin loader/host code lives in `src/core/plugin/`:

| File | Owns |
|------|------|
| `api.rs` | C ABI vtable struct, `API_VERSION`, all FFI wrapper functions |
| `types.rs` | `Plugin`, `InitResult`, manifest handling, callback type aliases |
| `overlay.rs` | Overlay/panel registration state, render-hook gating |
| `menu.rs` | Page/section/icon registration state |
| `notification.rs` | Notification queue |
| `events.rs` | Host event dispatch to subscribed plugins |
| `career.rs` | Career lifecycle event emission |
| `mod.rs` | Re-exports public surface |

GUI rendering stays in `src/core/gui.rs` — it reads plugin state through `pub(crate)` getters but does not own it.
