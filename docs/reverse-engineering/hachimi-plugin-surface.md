# Hachimi Plugin API Surface

## Overview

Hachimi Edge exposes a native C ABI plugin system. Plugins are dynamic libraries (`.dll` on Windows, `.so` on Android) that export a single entry point. The host passes a vtable of function pointers that give the plugin access to hooking, IL2CPP introspection, GUI rendering, and logging.

## Plugin Lifecycle

```
1. Host discovers plugin DLL/SO
   ├── Windows: config.windows.load_libraries or manual copy
   └── Android: config.android.load_libraries or auto-scan libhachimi_*.so

2. Host calls LoadLibrary / dlopen

3. Host resolves export: "hachimi_init"

4. Host completes core hooking (IL2CPP, GUI, IPC)

5. Host calls hachimi_init(vtable, version)
   ├── Plugin stores vtable pointer
   ├── Plugin registers hooks
   ├── Plugin registers UI
   └── Plugin returns InitResult::Ok (1) or InitResult::Error (0)

6. Runtime: plugin hooks fire, UI callbacks render each frame

7. Process exit: host calls unhook_all (no plugin deinit callback)
```

## Entry Point

```c
// Must be exported with C linkage
extern "C" int hachimi_init(const Vtable* vtable, int version);
// Returns 1 for success, 0 for error
// Current API version: 2
```

## Vtable Capabilities

The vtable contains **52 function pointer slots** total, organized into the following groups.

### Host Access (2 functions)
| Function | Signature | Purpose |
|----------|-----------|---------|
| `hachimi_instance` | `() → *const Hachimi` | Get the global Hachimi instance |
| `hachimi_get_interceptor` | `(hachimi) → *const Interceptor` | Get the interceptor from a Hachimi instance |

### Hooking (4 functions)
| Function | Signature | Purpose |
|----------|-----------|---------|
| `interceptor_hook` | `(interceptor, orig_addr, hook_addr) → trampoline` | Hook a function by address |
| `interceptor_hook_vtable` | `(interceptor, vtable, index, hook_addr) → trampoline` | Hook a vtable entry |
| `interceptor_get_trampoline_addr` | `(interceptor, hook_addr) → trampoline` | Get original function pointer |
| `interceptor_unhook` | `(interceptor, hook_addr) → orig_addr` | Remove a hook |

**Backend**: MinHook on Windows, Dobby on Android.

### IL2CPP Introspection (19 functions)
| Function | Purpose |
|----------|---------|
| `il2cpp_resolve_symbol` | dlsym for IL2CPP symbols |
| `il2cpp_get_assembly_image` | Get assembly image by name (e.g., `"umamusume.dll"`) |
| `il2cpp_get_class` | Get class by namespace + name |
| `il2cpp_get_method` | Get method by name + arg count |
| `il2cpp_get_method_overload` | Get method by name + parameter types |
| `il2cpp_get_method_addr` | Get method code address |
| `il2cpp_get_method_overload_addr` | Get overloaded method code address |
| `il2cpp_get_method_cached` | Get method (cached lookup) |
| `il2cpp_get_method_addr_cached` | Get method address (cached) |
| `il2cpp_find_nested_class` | Find nested/inner class |
| `il2cpp_resolve_icall` | Resolve internal call |
| `il2cpp_class_get_methods` | Iterate all methods on a class |
| `il2cpp_get_field_from_name` | Get field by name |
| `il2cpp_get_field_value` | Read instance field |
| `il2cpp_set_field_value` | Write instance field |
| `il2cpp_get_static_field_value` | Read static field |
| `il2cpp_set_static_field_value` | Write static field |
| `il2cpp_object_new` | Allocate new IL2CPP object |
| `il2cpp_unbox` | Unbox value type |

### Threading (3 functions)
| Function | Purpose |
|----------|---------|
| `il2cpp_get_main_thread` | Get the main (UI) thread |
| `il2cpp_get_attached_threads` | Get all attached threads |
| `il2cpp_schedule_on_thread` | Schedule a callback on a specific thread |

### Object Creation (2 functions)
| Function | Purpose |
|----------|---------|
| `il2cpp_create_array` | Create a new IL2CPP array |
| `il2cpp_get_singleton_like_instance` | Get singleton instance of a class |

### Logging (1 function)
| Function | Levels |
|----------|--------|
| `log(level, target, message)` | 1=Error, 2=Warn, 3=Info, 4=Debug, 5=Trace |

### GUI (17 functions)

#### Menu Registration (5 functions)
| Function | Purpose |
|----------|---------|
| `gui_register_menu_item` | Add a clickable item to the Plugins section |
| `gui_register_menu_section` | Add a custom-drawn section to the menu |
| `gui_show_notification` | Push a toast notification |
| `gui_register_menu_item_icon` | Add a menu item with a PNG icon |
| `gui_register_menu_section_with_icon` | Add a section with a PNG icon and title |

> **Note:** In the vtable, `gui_show_notification` comes before the icon variants. The ordering above matches the actual vtable layout.

#### Widget Helpers (12 functions, used inside section callbacks)
| Function | Purpose |
|----------|---------|
| `gui_ui_heading` | Draw heading text |
| `gui_ui_label` | Draw label text |
| `gui_ui_small` | Draw small text |
| `gui_ui_separator` | Draw horizontal separator |
| `gui_ui_button` | Draw button, returns true if clicked |
| `gui_ui_small_button` | Draw small button |
| `gui_ui_checkbox` | Draw checkbox |
| `gui_ui_text_edit_singleline` | Draw text input field |
| `gui_ui_horizontal` | Horizontal layout container |
| `gui_ui_grid` | Grid layout container |
| `gui_ui_end_row` | End a grid row |
| `gui_ui_colored_label` | Draw colored text (RGBA) |

The GUI is built on **egui** (rendered via egui_glow on Android, egui-directx11 on Windows).

### Android DEX Helpers (4 functions, v2+)
| Function | Purpose |
|----------|---------|
| `android_dex_load` | Load a DEX blob and get a class handle |
| `android_dex_unload` | Unload a DEX class |
| `android_dex_call_static_noargs` | Call a static method with no arguments |
| `android_dex_call_static_string` | Call a static method with a string argument |

No-ops on Windows.

## Plugin Patterns

### Typical Hook Installation

```rust
unsafe {
    let image = (vt.il2cpp_get_assembly_image)(b"umamusume.dll\0".as_ptr() as _);
    let klass = (vt.il2cpp_get_class)(image, b"Gallop\0".as_ptr() as _, b"ClassName\0".as_ptr() as _);
    let addr = (vt.il2cpp_get_method_addr)(klass, b"MethodName\0".as_ptr() as _, arg_count);
    
    let hachimi = (vt.hachimi_instance)();
    let interceptor = (vt.hachimi_get_interceptor)(hachimi);
    let trampoline = (vt.interceptor_hook)(interceptor, addr, my_hook as *mut c_void);
    // Store trampoline to call original
}
```

### Reading a Field

```rust
unsafe {
    let field = (vt.il2cpp_get_field_from_name)(klass, b"_fieldName\0".as_ptr() as _);
    let mut value: i32 = 0;
    (vt.il2cpp_get_field_value)(obj, field, &mut value as *mut _ as _);
}
```

### Drawing Custom UI

```rust
extern "C" fn my_section(ui: *mut c_void, _userdata: *mut c_void) {
    unsafe {
        (vt.gui_ui_heading)(ui, c"My Plugin".as_ptr());
        (vt.gui_ui_label)(ui, c"Status: Active".as_ptr());
        if (vt.gui_ui_button)(ui, c"Do Thing".as_ptr()) {
            // button was clicked
        }
        (vt.gui_ui_separator)(ui);
    }
}
```

## Limitations

- **No plugin deinit callback** — Plugins can't clean up on unload
- **No custom window API** — Plugins draw inside the host menu, not their own windows
- **No async/timer API** — Plugins must manage their own threading
- **Vtable is additive-only** — New functions are appended; old ones can't be removed (version check recommended)
- **No inter-plugin communication** — Plugins don't know about each other
- **egui widgets are host-mediated** — Plugins get an opaque `ui` pointer, not direct egui access
