//! Plugin API version and vtable slot count.

/// Current plugin API version passed to `hachimi_init` alongside the vtable pointer.
///
/// v9 redesign: removed per-widget GUI slots (plugins now draw with the shared
/// `egui::Ui`), added `host_subscribe`/`host_unsubscribe`/`host_capabilities`/
/// `gui_unregister`, registration slots return handles, and plugins export a
/// `hachimi_plugin_manifest`.
///
/// v10: added `host_data_path` (resolve paths under the game data dir) and the
/// `capability::DATA_PATHS` bit, enabling plugins to locate host-cached data
/// such as the GameTora snapshots.
pub const API_VERSION: i32 = 10;

/// Number of function pointers in `Vtable`.
pub const VTABLE_SLOT_COUNT: usize = 43;
