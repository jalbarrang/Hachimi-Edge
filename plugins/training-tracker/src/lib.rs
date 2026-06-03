//! Hachimi Training Tracker Plugin
//!
//! Tracks how many times each training facility (Speed, Stamina, Power, Guts,
//! Wisdom) has been visited during a career run, and displays the counts in
//! Hachimi's in-game overlay.

#![allow(function_casts_as_integer)]

#[macro_use]
extern crate hachimi_plugin_abi;

mod bond_progress;
mod class_dump;
mod config;
mod deck_bonuses;
mod diagnostics;
mod eval_data;
mod evaluation;
mod gametora_data;
mod hooks;
mod memory_reader;
mod overlay_cache;
mod rank_table;
mod recommend;
mod shop_hooks;
mod skill_shop;
mod skill_shop_prefs;
mod stat_targets;
mod tabs;
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

    config::load();
    ui::register_ui();

    // Warm the GameTora catalog off-thread (a 2MB+ parse) so it's ready for
    // tracker features, and surface load diagnostics in the log.
    std::thread::spawn(|| {
        if gametora_data::is_available() {
            hlog_info!(target: "training-tracker", "GameTora catalog ready");
        } else {
            hlog_warn!(
                target: "training-tracker",
                "GameTora catalog unavailable (no cache yet, or host too old)"
            );
        }
    });

    let events = hooks::subscribe_events();
    if shop_hooks::try_install_shop_hooks() {
        hlog_info!(target: "training-tracker", "Skill shop visibility hooks installed");
    }

    if !events {
        hlog_warn!(
            target: "training-tracker",
            "Host does not advertise EVENTS; shutdown teardown of IL2CPP hooks is unavailable."
        );
    }
    hlog_info!(target: "training-tracker", "Training Tracker ready");
    sdk.show_notification("Training Tracker loaded!");

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
