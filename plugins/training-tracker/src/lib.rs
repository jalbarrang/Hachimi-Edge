//! Hachimi Training Tracker Plugin
//!
//! Tracks how many times each training facility (Speed, Stamina, Power, Guts,
//! Wisdom) has been visited during a career run, and displays the counts in
//! Hachimi's in-game overlay.
//!
//! ## How it works
//!
//! 1. On `hachimi_init`, the plugin stores the host vtable and registers UI.
//! 2. It attempts to hook the IL2CPP method that fires when the player selects
//!    a training command. The exact method name varies by game version.
//! 3. Each time a training is selected, the hook increments the counter for
//!    that facility.
//! 4. A menu section in the Hachimi overlay shows a live table of counts.
//!
//! ## Updating hook targets
//!
//! If the automatic method resolution fails (you'll see warnings in the log),
//! you need to:
//! 1. Run Il2CppDumper on your `GameAssembly.dll` / `libil2cpp.so`
//! 2. Search the dump for training-related methods in the `Gallop` namespace
//! 3. Update the candidates list in `hooks.rs`

#![allow(function_casts_as_integer)] // Plugin hooks cast function pointers

#[macro_use]
mod vtable;
mod diagnostics;
mod hooks;
mod memory_reader;
mod skill_shop;
mod tracker;
mod ui;

use std::ffi::c_void;

/// Plugin entry point called by Hachimi after core hooking is complete.
///
/// # Safety
/// Called by the host with a valid vtable pointer.
#[no_mangle]
pub extern "C" fn hachimi_init(vtable_ptr: *const c_void, version: i32) -> i32 {
    if vtable_ptr.is_null() {
        return 0; // InitResult::Error
    }

    // Store the vtable for use by all modules
    // SAFETY: Plugin FFI interop with Hachimi vtable
    unsafe {
        vtable::set_vtable(vtable_ptr as *const vtable::Vtable);
    }

    hlog_info!(
        "Training Tracker plugin v{} initializing (host API v{})",
        env!("CARGO_PKG_VERSION"),
        version
    );

    // Register the UI section (always works, even without hooks)
    ui::register_ui(version);

    // Try to install IL2CPP hooks
    let hooked = hooks::try_install_hooks();

    if hooked {
        hlog_info!("Training Tracker ready — hooks installed");
    } else {
        hlog_warn!(
            "Training Tracker loaded without hooks. The UI is registered \
             but training won't be tracked automatically. See the log for \
             details on which methods were tried."
        );
    }

    // Show a notification to the user
    // SAFETY: Plugin FFI interop with Hachimi vtable
    unsafe {
        let msg = if hooked {
            c"Training Tracker loaded!"
        } else {
            c"Training Tracker loaded (no hooks - see log)"
        };
        (vtable::vt().gui_show_notification)(msg.as_ptr());
    }

    1 // InitResult::Ok
}
