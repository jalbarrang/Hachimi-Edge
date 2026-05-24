//! Hachimi Training Tracker Plugin
//!
//! Tracks how many times each training facility (Speed, Stamina, Power, Guts,
//! Wisdom) has been visited during a career run, and displays the counts in
//! Hachimi's in-game overlay.

#![allow(function_casts_as_integer)]

#[macro_use]
extern crate hachimi_plugin_abi;

mod diagnostics;
mod hooks;
mod memory_reader;
mod skill_shop;
mod tracker;
mod ui;

use std::ffi::c_void;

use hachimi_plugin_abi::{InitResult, Vtable};
use hachimi_plugin_sdk::{init_result_to_i32, Sdk};

/// Plugin entry point called by Hachimi after core hooking is complete.
///
/// # Safety
/// Called by the host with a valid vtable pointer.
#[no_mangle]
pub extern "C" fn hachimi_init(vtable_ptr: *const c_void, version: i32) -> i32 {
    // SAFETY: Host passes a valid vtable pointer during plugin load.
    match unsafe { Sdk::init(vtable_ptr as *const Vtable, version) } {
        Ok(()) => init_inner(version),
        Err(_) => init_result_to_i32(InitResult::Error),
    }
}

fn init_inner(version: i32) -> i32 {
    hlog_info!(
        target: "training-tracker",
        "Training Tracker plugin v{} initializing (host API v{})",
        env!("CARGO_PKG_VERSION"),
        version
    );

    ui::register_ui();

    let hooked = hooks::try_install_hooks();

    let sdk = Sdk::get();
    if hooked {
        hlog_info!(target: "training-tracker", "Training Tracker ready — hooks installed");
        sdk.show_notification("Training Tracker loaded!");
    } else {
        hlog_warn!(
            target: "training-tracker",
            "Training Tracker loaded without hooks. The UI is registered \
             but training won't be tracked automatically. See the log for \
             details on which methods were tried."
        );
        sdk.show_notification("Training Tracker loaded (no hooks - see log)");
    }

    init_result_to_i32(InitResult::Ok)
}
