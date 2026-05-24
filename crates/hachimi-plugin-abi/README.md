# hachimi-plugin-abi

Stable C ABI types for Hachimi plugins: `Vtable` (57 slots), `API_VERSION = 7`, `set_vtable` / `vt()`, and `hlog_*` macros.

Use this crate alone for minimal plugins, or add `hachimi-plugin-sdk` for safe wrappers.

```toml
[dependencies]
hachimi-plugin-abi = { path = "../../crates/hachimi-plugin-abi" }
```

```rust
#[macro_use]
extern crate hachimi_plugin_abi;

use hachimi_plugin_abi::{set_vtable, InitResult, Vtable};

#[no_mangle]
pub extern "C" fn hachimi_init(vtable_ptr: *const Vtable, version: i32) -> i32 {
    if vtable_ptr.is_null() {
        return InitResult::Error as i32;
    }
    // SAFETY: Host provides a valid vtable for process lifetime.
    unsafe { set_vtable(vtable_ptr) };
    hlog_info!("loaded (API v{})", version);
    InitResult::Ok as i32
}
```
