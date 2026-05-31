//! Hachimi Training Tracker Plugin
//!
//! Tracks how many times each training facility (Speed, Stamina, Power, Guts,
//! Wisdom) has been visited during a career run, and displays the counts in
//! Hachimi's in-game overlay.

#![allow(function_casts_as_integer)]

#[macro_use]
extern crate hachimi_plugin_abi;

mod class_dump;
mod deck_bonuses;
mod diagnostics;
mod eval_data;
mod evaluation;
mod hooks;
mod memory_reader;
mod overlay_cache;
mod rank_table;
mod recommend;
mod shop_hooks;
mod skill_shop;
mod skill_shop_prefs;
mod stat_targets;
mod tracker;
mod ui;

use hachimi_plugin_sdk::{hachimi_plugin, Sdk};

/// Plugin entry point. The macro generates the `hachimi_init` and
/// `hachimi_plugin_manifest` C exports; `min_api`/version come from the SDK and Cargo.
#[hachimi_plugin(name = "training-tracker", caps = hachimi_plugin_sdk::capability::UNLOADABLE)]
fn init(sdk: &Sdk) -> Result<(), &'static str> {
    hlog_info!(
        target: "training-tracker",
        "Training Tracker plugin v{} initializing (host API v{})",
        env!("CARGO_PKG_VERSION"),
        sdk.version().raw()
    );

    stat_targets::load();
    ui::register_ui();

    let tracking = hooks::subscribe_events();
    if shop_hooks::try_install_shop_hooks() {
        hlog_info!(target: "training-tracker", "Skill shop visibility hooks installed");
    }

    if tracking {
        hlog_info!(target: "training-tracker", "Training Tracker ready — subscribed to host events");
        sdk.show_notification("Training Tracker loaded!");
    } else {
        hlog_warn!(
            target: "training-tracker",
            "Training Tracker loaded but training-command events are unavailable. \
             The UI is registered but training won't be tracked automatically."
        );
        sdk.show_notification("Training Tracker loaded (no tracking - see log)");
    }

    Ok(())
}

#[cfg(test)]
mod manifest_tests {
    #[test]
    fn manifest_declares_unloadable() {
        // SAFETY: the generated manifest is a 'static read-only struct.
        let manifest = unsafe { &*crate::hachimi_plugin_manifest() };
        assert_ne!(
            manifest.requested_caps & hachimi_plugin_sdk::capability::UNLOADABLE,
            0,
            "training-tracker must advertise UNLOADABLE so the host can hot-reload it"
        );
    }
}
