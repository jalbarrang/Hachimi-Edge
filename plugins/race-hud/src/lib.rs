//! Race HUD Plugin
//!
//! Surfaces a live per-runner heads-up display during races. It captures the race
//! SimData and decodes it; the live per-runner feed is built on top of the decoded
//! frames.

#![allow(function_casts_as_integer)]

#[macro_use]
extern crate hachimi_plugin_abi;

mod capture;
mod sim;
mod state;
mod tick;
mod ui;

use hachimi_plugin_sdk::{hachimi_plugin, Sdk};

/// Plugin entry point. The macro generates the C exports consumed by Hachimi.
#[hachimi_plugin(name = "race-hud", caps = hachimi_plugin_sdk::capability::UNLOADABLE)]
fn init(sdk: &Sdk) -> Result<(), &'static str> {
    hlog_info!(
        target: "race-hud",
        "Race HUD v{} initializing (host API v{})",
        env!("CARGO_PKG_VERSION"),
        sdk.version().raw()
    );

    state::init();
    ui::register_ui();

    if !tick::subscribe_events() {
        hlog_warn!(
            target: "race-hud",
            "Host does not advertise EVENTS; cadence + shutdown teardown unavailable"
        );
    }

    if !capture::install() {
        hlog_warn!(
            target: "race-hud",
            "SimData capture hook not installed; overlay will stay idle"
        );
    }

    hlog_info!(target: "race-hud", "Race HUD ready");
    sdk.show_notification("Race HUD loaded");

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
            "race-hud must advertise UNLOADABLE so it can unhook on unload"
        );
    }
}
