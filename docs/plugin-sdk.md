# Plugin SDK Guide

Hachimi plugins are native shared libraries (`.dll` on Windows, `.so` on Android) loaded at runtime. They interact with the host through a C ABI vtable — a struct of function pointers passed during initialization. Any language that can produce a `cdylib` and call C functions can be a plugin.

## Quick Start

### 1. Create a cdylib crate

```toml
# Cargo.toml
[lib]
crate-type = ["cdylib"]
```

### 2. Export `hachimi_init`

```rust
use std::ffi::c_void;

#[no_mangle]
pub extern "C" fn hachimi_init(vtable_ptr: *const c_void, version: i32) -> i32 {
    if vtable_ptr.is_null() {
        return 0; // Error
    }

    // Store the vtable pointer for later use (see "Vtable Access" below)
    // SAFETY: Host guarantees vtable_ptr is valid for the process lifetime.
    unsafe { set_vtable(vtable_ptr as *const Vtable); }

    // Register UI, install hooks, etc.
    // ...

    1 // Ok
}
```

**Parameters:**
- `vtable_ptr` — Pointer to the host's `Vtable` struct. Valid for the entire process lifetime.
- `version` — Host API version (currently `3`). Use this to gate access to newer vtable fields.

**Return value:** `0` = error, `1` = ok.

### 3. Configure loading

Add your DLL name to the game's `hachimi/config.json`:

```json
{
  "windows": {
    "load_libraries": ["my_plugin.dll"]
  }
}
```

On Android, use the `android.load_libraries` array, or name your `.so` with the `libhachimi_` prefix for auto-discovery.

### 4. Deploy

Copy the built DLL to the game directory root (same folder as `config.json`).

---

## The Vtable

The vtable is a `#[repr(C)]` struct of function pointers. **Field order is ABI** — it must never be reordered. New fields are appended at the end and gated by the `version` parameter.

Your plugin must mirror the struct layout exactly. See `plugins/training-tracker/src/vtable.rs` for a complete reference copy.

### Vtable domains

The 53 function pointers are logically grouped:

| Domain | Fields | Purpose |
|--------|--------|---------|
| Core | `hachimi_instance`, `hachimi_get_interceptor` | Get host instance and interceptor |
| Hook API | `interceptor_hook`, `interceptor_hook_vtable`, `interceptor_get_trampoline_addr`, `interceptor_unhook` | Install/remove function hooks |
| IL2CPP API | 23 fields (`il2cpp_*`) | Resolve classes, methods, fields; create objects; manage threads |
| Logging | `log` | Log messages through the host logger |
| GUI — Registration | `gui_register_menu_item`, `gui_register_menu_section`, `gui_show_notification`, `gui_register_menu_item_icon`, `gui_register_menu_section_with_icon` | Register menu entries and send notifications |
| GUI — Widgets | `gui_ui_heading`, `gui_ui_label`, `gui_ui_small`, `gui_ui_separator`, `gui_ui_button`, `gui_ui_small_button`, `gui_ui_checkbox`, `gui_ui_text_edit_singleline`, `gui_ui_horizontal`, `gui_ui_grid`, `gui_ui_end_row`, `gui_ui_colored_label` | Draw UI inside callbacks |
| Android DEX | `android_dex_load`, `android_dex_unload`, `android_dex_call_static_noargs`, `android_dex_call_static_string` | Load/call Java code (v2+, no-op on Windows) |
| Overlay | `gui_register_overlay` | Register always-visible overlays (v3+) |

### Version gating

```rust
// Only use overlay API if host supports it
if api_version >= 3 {
    (vt.gui_register_overlay)(id.as_ptr(), Some(draw_fn), std::ptr::null_mut());
}
```

Fields added in version N are only safe to access when `version >= N`. Accessing fields beyond the version boundary is undefined behavior.

---

## GUI: Menu Sections

Menu sections draw inside Hachimi's side panel when the user opens the menu.

```rust
extern "C" fn draw_section(ui: *mut c_void, _userdata: *mut c_void) {
    let vt = get_vtable();
    unsafe {
        (vt.gui_ui_heading)(ui, c"My Plugin".as_ptr());
        if (vt.gui_ui_button)(ui, c"Click me".as_ptr()) {
            (vt.gui_show_notification)(c"Button clicked!".as_ptr());
        }
        (vt.gui_ui_separator)(ui);
    }
}

// In hachimi_init:
(vt.gui_register_menu_section)(Some(draw_section), std::ptr::null_mut());
```

The `ui` pointer is an opaque handle to an `egui::Ui`. Do not dereference it — pass it to `gui_ui_*` functions only.

### With title and icon

```rust
(vt.gui_register_menu_section_with_icon)(
    c"My Plugin".as_ptr(),      // title
    c"bytes://my-icon".as_ptr(), // icon URI (or null for auto)
    icon_bytes.as_ptr(),         // PNG data
    icon_bytes.len(),
    Some(draw_section),
    std::ptr::null_mut(),
);
```

---

## GUI: Overlays (v3+)

Overlays are always-visible HUD elements rendered every frame, even when the menu is closed. They render anchored to the top-right corner and are non-interactive (they don't capture game input).

```rust
extern "C" fn draw_overlay(ui: *mut c_void, _userdata: *mut c_void) {
    let vt = get_vtable();
    unsafe {
        (vt.gui_ui_small)(ui, c"HP: 100".as_ptr());
    }
}

// In hachimi_init (check version first):
if version >= 3 {
    (vt.gui_register_overlay)(
        c"my_overlay_id".as_ptr(),
        Some(draw_overlay),
        std::ptr::null_mut(),
    );
}
```

**Important:** Registering an overlay keeps the render hook active. The host's `is_empty()` check accounts for plugin overlays — your overlay will render even when nothing else is on screen.

---

## Hooking IL2CPP Methods

The hook API lets you intercept game functions at runtime.

```rust
// 1. Resolve the method address
let image = (vt.il2cpp_get_assembly_image)(c"umamusume.dll".as_ptr());
let klass = (vt.il2cpp_get_class)(image, c"Gallop".as_ptr(), c"SomeClass".as_ptr());
let addr = (vt.il2cpp_get_method_addr)(klass, c"SomeMethod".as_ptr(), 2);

// 2. Install the hook
let hachimi = (vt.hachimi_instance)();
let interceptor = (vt.hachimi_get_interceptor)(hachimi);
let trampoline = (vt.interceptor_hook)(interceptor, addr, my_hook as *mut c_void);

// 3. Store the trampoline to call the original
static mut ORIG: *mut c_void = std::ptr::null_mut();
ORIG = trampoline;

// 4. Your hook function (must match the original's calling convention)
extern "C" fn my_hook(this: *mut c_void, arg1: usize, arg2: usize) {
    // Your logic here...

    // Call the original
    unsafe {
        let orig: extern "C" fn(*mut c_void, usize, usize) = std::mem::transmute(ORIG);
        orig(this, arg1, arg2);
    }
}
```

**Use `usize` for all pointer-typed arguments** — IL2CPP object pointers are 64-bit on Windows. Using `i32` will truncate them.

---

## Logging

```rust
// Log levels: 1=Error, 2=Warn, 3=Info, 4=Debug, 5=Trace
(vt.log)(3, c"my-plugin".as_ptr(), c"Hello from plugin!".as_ptr());
```

Logs go to the same file as Hachimi's own logs. A convenience macro pattern:

```rust
macro_rules! hlog {
    ($level:expr, $($arg:tt)*) => {{
        let msg = std::ffi::CString::new(format!($($arg)*)).unwrap_or_default();
        unsafe { (vt().log)($level, c"my-plugin".as_ptr(), msg.as_ptr()); }
    }};
}
```

---

## Notifications

Show a toast notification on screen:

```rust
(vt.gui_show_notification)(c"Plugin loaded successfully!".as_ptr());
```

Notifications are queued and displayed on the next render frame. Safe to call from any thread.

---

## Userdata Pattern

All callback registration functions accept a `*mut c_void` userdata pointer. This is passed back to your callback on every invocation. Use it to avoid global state:

```rust
struct MyState { counter: u32 }

let state = Box::into_raw(Box::new(MyState { counter: 0 }));
(vt.gui_register_menu_section)(Some(draw_fn), state as *mut c_void);

extern "C" fn draw_fn(ui: *mut c_void, userdata: *mut c_void) {
    let state = unsafe { &mut *(userdata as *mut MyState) };
    state.counter += 1;
    // ...
}
```

**You own the memory.** The host will not free it. If the plugin can be unloaded, arrange cleanup accordingly.

---

## Thread Safety

- **Registration functions** (`gui_register_*`, `gui_show_notification`) are safe to call from any thread. They acquire internal locks.
- **Widget functions** (`gui_ui_*`) must only be called from inside a callback — they operate on the render thread's `egui::Ui`.
- **Hook/IL2CPP functions** can be called from any thread once `hachimi_init` returns.
- **Callbacks** are invoked on the render thread. Keep them fast — blocking stalls the frame.

---

## Panic Safety

The host wraps plugin callbacks in `catch_unwind`. A panic in your callback will be caught and logged, not crash the game. However, panicking across FFI is undefined behavior in Rust — mark your callbacks `extern "C"` and avoid panicking in them.

---

## Reference Implementation

See `plugins/training-tracker/` for a complete working plugin that demonstrates:
- Vtable mirroring (`src/vtable.rs`)
- Menu section + overlay registration (`src/ui.rs`)
- IL2CPP hook installation with fallback candidates (`src/hooks.rs`)
- Logging macros (`vtable.rs` bottom)
- State management with `Mutex` (`src/tracker.rs`)

---

## Host Module Structure

For contributors working on the host side, plugin SDK code lives in `src/core/plugin/`:

| File | Owns |
|------|------|
| `api.rs` | C ABI vtable struct, VERSION, all FFI wrapper functions |
| `types.rs` | `Plugin`, `InitResult`, callback type aliases |
| `overlay.rs` | Overlay registration state, `has_plugin_overlays()` for render hook gating |
| `menu.rs` | Menu item/section/icon registration state |
| `notification.rs` | Notification queue |
| `mod.rs` | Re-exports public surface |

GUI rendering code stays in `src/core/gui.rs` — it reads plugin state through `pub(crate)` getters but does not own it.
