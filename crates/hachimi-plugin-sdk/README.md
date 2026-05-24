# hachimi-plugin-sdk

Recommended plugin SDK: re-exports `hachimi-plugin-abi`, plus `Sdk`, `ApiVersion`, and safe wrappers (`gui`, `il2cpp`, `hook`).

```toml
[dependencies]
hachimi-plugin-abi = { path = "../../crates/hachimi-plugin-abi" }
hachimi-plugin-sdk = { path = "../../crates/hachimi-plugin-sdk" }
```

```rust
#[macro_use]
extern crate hachimi_plugin_abi;

use hachimi_plugin_abi::{InitResult, Vtable};
use hachimi_plugin_sdk::{init_result_to_i32, Sdk};

#[no_mangle]
pub extern "C" fn hachimi_init(vtable_ptr: *const std::ffi::c_void, version: i32) -> i32 {
    // SAFETY: Host passes a valid vtable at load.
    match unsafe { Sdk::init(vtable_ptr as *const Vtable, version) } {
        Ok(()) => {
            Sdk::get().show_notification("My plugin loaded");
            init_result_to_i32(InitResult::Ok)
        }
        Err(_) => init_result_to_i32(InitResult::Error),
    }
}
```

Version gates: `Sdk::get().version().supports_overlay()` (v3+), `supports_overlay_visibility()` (v5+), `supports_collapsing()` (v6+), `supports_font_size()` (v7+).
