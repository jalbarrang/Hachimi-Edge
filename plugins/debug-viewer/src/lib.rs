//! Debug Viewer Plugin (development only)
//!
//! Records game view transitions and is intended to grow into a live feed of
//! debug values during development. Off by default and never bundled with the
//! installer — enable it manually via `windows.load_libraries`.

#[macro_use]
extern crate hachimi_plugin_abi;

mod hooks;
mod state;
mod ui;

use hachimi_plugin_sdk::{hachimi_plugin, Sdk};

/// Plugin entry point. The macro generates the C exports consumed by Hachimi.
#[hachimi_plugin(name = "debug-viewer", caps = hachimi_plugin_sdk::capability::UNLOADABLE)]
fn init(sdk: &Sdk) -> Result<(), &'static str> {
    hlog_info!(
        target: "debug-viewer",
        "Debug Viewer v{} initializing (host API v{})",
        env!("CARGO_PKG_VERSION"),
        sdk.version().raw()
    );

    state::init();
    ui::register_ui();

    if !hooks::subscribe_events() {
        hlog_warn!(
            target: "debug-viewer",
            "Host does not advertise EVENTS; view-transition recording unavailable"
        );
    }

    hlog_info!(target: "debug-viewer", "Debug Viewer ready");
    sdk.show_notification("Debug Viewer loaded");

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
            "debug-viewer must advertise UNLOADABLE so the host can tear down callbacks"
        );
    }
}
