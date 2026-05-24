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

    pub gui_register_menu_item:
        unsafe extern "C" fn(label: *const c_char, callback: Option<GuiMenuCallback>, userdata: *mut c_void) -> bool,
    pub gui_register_menu_section:
        unsafe extern "C" fn(callback: Option<GuiMenuSectionCallback>, userdata: *mut c_void) -> bool,
    pub gui_show_notification: unsafe extern "C" fn(message: *const c_char) -> bool,
    pub gui_ui_heading: unsafe extern "C" fn(ui: *mut c_void, text: *const c_char) -> bool,
    pub gui_ui_label: unsafe extern "C" fn(ui: *mut c_void, text: *const c_char) -> bool,
    pub gui_ui_small: unsafe extern "C" fn(ui: *mut c_void, text: *const c_char) -> bool,
    pub gui_ui_separator: unsafe extern "C" fn(ui: *mut c_void) -> bool,
    pub gui_ui_button: unsafe extern "C" fn(ui: *mut c_void, text: *const c_char) -> bool,
    pub gui_ui_small_button: unsafe extern "C" fn(ui: *mut c_void, text: *const c_char) -> bool,
    pub gui_ui_checkbox: unsafe extern "C" fn(ui: *mut c_void, text: *const c_char, value: *mut bool) -> bool,
    pub gui_ui_text_edit_singleline:
        unsafe extern "C" fn(ui: *mut c_void, buffer: *mut c_char, buffer_len: usize) -> bool,
    pub gui_ui_horizontal:
        unsafe extern "C" fn(ui: *mut c_void, callback: Option<GuiUiCallback>, userdata: *mut c_void) -> bool,
    pub gui_ui_grid: unsafe extern "C" fn(
        ui: *mut c_void,
        id: *const c_char,
        columns: usize,
        spacing_x: f32,
        spacing_y: f32,
        callback: Option<GuiUiCallback>,
        userdata: *mut c_void,
    ) -> bool,
    pub gui_ui_end_row: unsafe extern "C" fn(ui: *mut c_void) -> bool,
    pub gui_ui_colored_label:
        unsafe extern "C" fn(ui: *mut c_void, r: u8, g: u8, b: u8, a: u8, text: *const c_char) -> bool,
    pub gui_register_menu_item_icon: unsafe extern "C" fn(
        label: *const c_char,
        icon_uri: *const c_char,
        icon_ptr: *const u8,
        icon_len: usize,
    ) -> bool,
    pub gui_register_menu_section_with_icon: unsafe extern "C" fn(
        title: *const c_char,
        icon_uri: *const c_char,
        icon_ptr: *const u8,
        icon_len: usize,
        callback: Option<GuiMenuSectionCallback>,
        userdata: *mut c_void,
    ) -> bool,

    pub android_dex_load: unsafe extern "C" fn(dex_ptr: *const u8, dex_len: usize, class_name: *const c_char) -> u64,
    pub android_dex_unload: unsafe extern "C" fn(handle: u64) -> bool,
    pub android_dex_call_static_noargs:
        unsafe extern "C" fn(handle: u64, method: *const c_char, sig: *const c_char) -> bool,
    pub android_dex_call_static_string:
        unsafe extern "C" fn(handle: u64, method: *const c_char, sig: *const c_char, arg: *const c_char) -> bool,

    pub gui_register_overlay: unsafe extern "C" fn(
        id: *const c_char,
        callback: Option<GuiMenuSectionCallback>,
        userdata: *mut c_void,
    ) -> bool,

    pub gui_ui_set_min_width: unsafe extern "C" fn(ui: *mut c_void, width: f32) -> bool,

    pub gui_overlay_set_visible: unsafe extern "C" fn(id: *const c_char, visible: bool) -> bool,

    pub gui_ui_set_font_size: unsafe extern "C" fn(ui: *mut c_void, size: f32) -> bool,

    pub gui_ui_collapsing: unsafe extern "C" fn(
        ui: *mut c_void,
        heading: *const c_char,
        default_open: bool,
        callback: Option<GuiUiCallback>,
        userdata: *mut c_void,
    ) -> bool,
}
