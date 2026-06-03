//! Stable C ABI for Hachimi plugins: `Vtable`, opaque FFI types, init helpers, and logging macros.
//!
//! Field order in `Vtable` is part of the wire ABI — append new slots only at the end and bump
//! [`API_VERSION`](version::API_VERSION).

#![allow(clippy::too_many_lines)] // Vtable is intentionally one struct

mod init;
mod log;
mod version;

use std::ffi::{c_char, c_void};

pub use init::{set_vtable, try_vt, vt};
pub use log::log_level;
pub use version::{API_VERSION, VTABLE_SLOT_COUNT};

// Opaque host types — plugins only hold pointers.
pub type Hachimi = c_void;
pub type Interceptor = c_void;
pub type Il2CppImage = c_void;
pub type Il2CppClass = c_void;
pub type Il2CppObject = c_void;
pub type Il2CppArray = c_void;
pub type Il2CppThread = c_void;
pub type MethodInfo = c_void;
pub type FieldInfo = c_void;
pub type Il2CppTypeEnum = i32;
pub type Il2CppMethodPointer = usize;

pub type GuiMenuCallback = extern "C" fn(userdata: *mut c_void);
pub type GuiMenuSectionCallback = extern "C" fn(ui: *mut c_void, userdata: *mut c_void);
pub type GuiUiCallback = extern "C" fn(ui: *mut c_void, userdata: *mut c_void);

/// Host → plugin event callback. `data` is event-specific (null for most events).
pub type PluginEventFn = extern "C" fn(event_id: u32, data: *const c_void, userdata: *mut c_void);

/// Plugin-exported metadata function: `hachimi_plugin_manifest() -> *const PluginManifest`.
pub type PluginManifestFn = extern "C" fn() -> *const PluginManifest;

/// Host→plugin event ids for [`Vtable::host_subscribe`].
///
/// Event ids are append-only and **do not** require an [`API_VERSION`] bump or a
/// new vtable slot — adding an event is purely additive. Events whose `data` is
/// non-null point at the matching `#[repr(C)]` payload struct documented below.
pub mod event {
    /// Fired once per rendered frame on the render thread. `data` is null.
    pub const FRAME: u32 = 1;
    /// Fired after the host reloads its config. `data` is null.
    pub const CONFIG_RELOAD: u32 = 2;
    /// Fired before the host unloads (process detach), or before a single plugin is
    /// unloaded. `data` is null. A plugin that installed IL2CPP hooks MUST unhook
    /// them here, otherwise unloading its DLL is unsafe.
    pub const SHUTDOWN: u32 = 3;
    /// Fired when the game changes view/scene. `data` → [`super::ViewChangeEvent`].
    pub const VIEW_CHANGE: u32 = 4;
    /// Fired when a Single Mode (career) run becomes active. `data` is null.
    pub const CAREER_START: u32 = 5;
    /// Fired when a Single Mode (career) run ends. `data` is null.
    pub const CAREER_END: u32 = 6;
    /// Fired when the player submits a training command. `data` → [`super::TrainingCommandEvent`].
    pub const TRAINING_COMMAND: u32 = 7;
    /// Fired once when the splash screen is first shown (game ready). `data` is null.
    pub const SPLASH_SHOWN: u32 = 8;
}

/// Payload for [`event::VIEW_CHANGE`]. `data` points at one of these for the
/// duration of the callback only — copy out what you need, don't retain the pointer.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ViewChangeEvent {
    /// The game's next view id (`Gallop.ViewId`). `1` is the splash view.
    pub view_id: i32,
}

/// Payload for [`event::TRAINING_COMMAND`]. Valid for the callback duration only.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct TrainingCommandEvent {
    /// The submitted Single Mode command id (e.g. `106` = Wisdom). Scenario-dependent.
    pub command_id: i32,
}

/// Host capability bitflags returned by [`Vtable::host_capabilities`], plus
/// plugin-declared flags set in [`PluginManifest::requested_caps`].
pub mod capability {
    // Host-provided capabilities (queried via `host_capabilities`).
    pub const GUI: u64 = 1 << 0;
    pub const OVERLAY: u64 = 1 << 1;
    pub const EVENTS: u64 = 1 << 2;
    pub const IL2CPP: u64 = 1 << 3;
    /// Host can resolve paths under the game data dir (see [`Vtable::host_data_path`]).
    pub const DATA_PATHS: u64 = 1 << 4;

    // Plugin-declared flags (set in the manifest `requested_caps`).
    /// The plugin promises it can be unloaded/reloaded at runtime: it removes every
    /// IL2CPP hook it installed in its `SHUTDOWN` handler so the host can safely
    /// `FreeLibrary` it. Without this flag the host only disconnects the plugin's
    /// GUI/event callbacks and keeps the DLL mapped (it never force-unmaps code the
    /// game may still call into).
    pub const UNLOADABLE: u64 = 1 << 8;
}

/// Plugin metadata read by the host before/at init for introspection and validation.
/// `name` and `version` are NUL-terminated, `'static`, UTF-8 C strings.
#[repr(C)]
pub struct PluginManifest {
    /// `API_VERSION` the plugin was built against.
    pub abi_version: i32,
    /// Minimum host API version the plugin requires.
    pub min_host_api: i32,
    /// Capability bits the plugin intends to use (see [`capability`]).
    pub requested_caps: u64,
    pub name: *const c_char,
    pub version: *const c_char,
}

// SAFETY: the pointers reference 'static C strings; the manifest is read-only.
unsafe impl Sync for PluginManifest {}

#[repr(i32)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum InitResult {
    Error = 0,
    Ok = 1,
}

impl InitResult {
    #[must_use]
    pub const fn is_ok(self) -> bool {
        matches!(self, Self::Ok)
    }
}

pub type HachimiInitFn = extern "C" fn(vtable: *const Vtable, version: i32) -> InitResult;

/// Flat function-pointer table passed from host to plugin at init.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Vtable {
    pub hachimi_instance: unsafe extern "C" fn() -> *const Hachimi,
    pub hachimi_get_interceptor: unsafe extern "C" fn(this: *const Hachimi) -> *const Interceptor,

    pub interceptor_hook:
        unsafe extern "C" fn(this: *const Interceptor, orig_addr: *mut c_void, hook_addr: *mut c_void) -> *mut c_void,
    pub interceptor_hook_vtable: unsafe extern "C" fn(
        this: *const Interceptor,
        vtable: *mut *mut c_void,
        vtable_index: usize,
        hook_addr: *mut c_void,
    ) -> *mut c_void,
    pub interceptor_get_trampoline_addr:
        unsafe extern "C" fn(this: *const Interceptor, hook_addr: *mut c_void) -> *mut c_void,
    pub interceptor_unhook: unsafe extern "C" fn(this: *const Interceptor, hook_addr: *mut c_void) -> *mut c_void,

    pub il2cpp_resolve_symbol: unsafe extern "C" fn(name: *const c_char) -> *mut c_void,
    pub il2cpp_get_assembly_image: unsafe extern "C" fn(assembly_name: *const c_char) -> *const Il2CppImage,
    pub il2cpp_get_class: unsafe extern "C" fn(
        image: *const Il2CppImage,
        namespace: *const c_char,
        class_name: *const c_char,
    ) -> *mut Il2CppClass,
    pub il2cpp_get_method:
        unsafe extern "C" fn(class: *mut Il2CppClass, name: *const c_char, args_count: i32) -> *const MethodInfo,
    pub il2cpp_get_method_overload: unsafe extern "C" fn(
        class: *mut Il2CppClass,
        name: *const c_char,
        params: *const Il2CppTypeEnum,
        param_count: usize,
    ) -> *const MethodInfo,
    pub il2cpp_get_method_addr:
        unsafe extern "C" fn(class: *mut Il2CppClass, name: *const c_char, args_count: i32) -> *mut c_void,
    pub il2cpp_get_method_overload_addr: unsafe extern "C" fn(
        class: *mut Il2CppClass,
        name: *const c_char,
        params: *const Il2CppTypeEnum,
        param_count: usize,
    ) -> *mut c_void,
    pub il2cpp_get_method_cached:
        unsafe extern "C" fn(class: *mut Il2CppClass, name: *const c_char, args_count: i32) -> *const MethodInfo,
    pub il2cpp_get_method_addr_cached:
        unsafe extern "C" fn(class: *mut Il2CppClass, name: *const c_char, args_count: i32) -> *mut c_void,
    pub il2cpp_find_nested_class:
        unsafe extern "C" fn(class: *mut Il2CppClass, name: *const c_char) -> *mut Il2CppClass,
    pub il2cpp_resolve_icall: unsafe extern "C" fn(name: *const c_char) -> Il2CppMethodPointer,
    pub il2cpp_class_get_methods:
        unsafe extern "C" fn(klass: *mut Il2CppClass, iter: *mut *mut c_void) -> *const MethodInfo,
    pub il2cpp_get_field_from_name:
        unsafe extern "C" fn(class: *mut Il2CppClass, name: *const c_char) -> *mut FieldInfo,
    pub il2cpp_get_field_value:
        unsafe extern "C" fn(obj: *mut Il2CppObject, field: *mut FieldInfo, out_value: *mut c_void),
    pub il2cpp_set_field_value:
        unsafe extern "C" fn(obj: *mut Il2CppObject, field: *mut FieldInfo, value: *const c_void),
    pub il2cpp_get_static_field_value: unsafe extern "C" fn(field: *mut FieldInfo, out_value: *mut c_void),
    pub il2cpp_set_static_field_value: unsafe extern "C" fn(field: *mut FieldInfo, value: *const c_void),
    pub il2cpp_object_new: unsafe extern "C" fn(klass: *const Il2CppClass) -> *mut Il2CppObject,
    pub il2cpp_unbox: unsafe extern "C" fn(obj: *mut Il2CppObject) -> *mut c_void,
    pub il2cpp_get_main_thread: unsafe extern "C" fn() -> *mut Il2CppThread,
    pub il2cpp_get_attached_threads: unsafe extern "C" fn(out_size: *mut usize) -> *mut *mut Il2CppThread,
    pub il2cpp_schedule_on_thread: unsafe extern "C" fn(thread: *mut Il2CppThread, callback: unsafe extern "C" fn()),
    pub il2cpp_create_array: unsafe extern "C" fn(element_type: *mut Il2CppClass, length: usize) -> *mut Il2CppArray,
    pub il2cpp_get_singleton_like_instance: unsafe extern "C" fn(class: *mut Il2CppClass) -> *mut Il2CppObject,

    pub log: unsafe extern "C" fn(level: i32, target: *const c_char, message: *const c_char),

    // Host services
    /// Capability bitflags (see [`capability`]).
    pub host_capabilities: unsafe extern "C" fn() -> u64,
    /// Subscribe to a host event. Returns a non-zero subscription handle, or 0 on failure.
    pub host_subscribe: unsafe extern "C" fn(event_id: u32, callback: PluginEventFn, userdata: *mut c_void) -> u64,
    /// Remove a subscription previously returned by `host_subscribe`.
    pub host_unsubscribe: unsafe extern "C" fn(handle: u64),

    // GUI registration. Plugins draw with the shared `egui::Ui` handed to their
    // callbacks (cast via the SDK); there are no per-widget slots.
    /// Returns a non-zero registration handle, or 0 on failure.
    pub gui_register_menu_item:
        unsafe extern "C" fn(label: *const c_char, callback: Option<GuiMenuCallback>, userdata: *mut c_void) -> u64,
    /// Returns a non-zero registration handle, or 0 on failure.
    pub gui_register_menu_section:
        unsafe extern "C" fn(callback: Option<GuiMenuSectionCallback>, userdata: *mut c_void) -> u64,
    pub gui_register_menu_item_icon: unsafe extern "C" fn(
        label: *const c_char,
        icon_uri: *const c_char,
        icon_ptr: *const u8,
        icon_len: usize,
    ) -> bool,
    /// Returns a non-zero registration handle, or 0 on failure.
    pub gui_register_menu_section_with_icon: unsafe extern "C" fn(
        title: *const c_char,
        icon_uri: *const c_char,
        icon_ptr: *const u8,
        icon_len: usize,
        callback: Option<GuiMenuSectionCallback>,
        userdata: *mut c_void,
    ) -> u64,
    /// Returns a non-zero registration handle, or 0 on failure.
    pub gui_register_overlay:
        unsafe extern "C" fn(id: *const c_char, callback: Option<GuiMenuSectionCallback>, userdata: *mut c_void) -> u64,
    /// Remove any registration (menu item/section/overlay) by its handle.
    pub gui_unregister: unsafe extern "C" fn(handle: u64) -> bool,
    pub gui_show_notification: unsafe extern "C" fn(message: *const c_char) -> bool,
    pub gui_overlay_set_visible: unsafe extern "C" fn(id: *const c_char, visible: bool) -> bool,

    // ── Data paths (API v10) ──
    /// Resolve `rel` against the game **data** directory and write the absolute
    /// path (UTF-8) into `out_buf`, NUL-terminated when there is room.
    ///
    /// Returns the number of bytes the full path requires, excluding the NUL
    /// terminator. If that value is `>= buf_len`, the path was truncated; the
    /// caller should retry with a buffer of at least `returned + 1` bytes. Pass
    /// a null `out_buf` (or `buf_len == 0`) to query the required length only.
    /// Returns `0` on error (null/invalid `rel`).
    pub host_data_path: unsafe extern "C" fn(rel: *const c_char, out_buf: *mut c_char, buf_len: usize) -> usize,
}

/// Subdirectory (under the game data dir) where the host caches GameTora data
/// snapshots. Shared so host and plugins resolve the same location.
pub const GAMETORA_DATA_SUBDIR: &str = "gametora";
