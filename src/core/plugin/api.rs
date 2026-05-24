//! C ABI surface for the plugin SDK.
//! `Vtable` is passed to plugins during init and is version-gated by `VERSION`.
//! Field order is part of the ABI: append new entries only at the end.
//! The functions in this module are the host-side FFI wrappers behind that table.

use std::ffi::{c_char, c_void, CStr};

use egui::Align;
use hachimi_plugin_abi::{
    FieldInfo, Hachimi, Il2CppArray, Il2CppClass, Il2CppImage, Il2CppMethodPointer, Il2CppObject, Il2CppThread,
    Il2CppTypeEnum, Interceptor, MethodInfo, Vtable, API_VERSION,
};
use hachimi_plugin_abi::{GuiMenuCallback, GuiMenuSectionCallback, GuiUiCallback, InitResult};
use once_cell::sync::OnceCell;

use crate::{
    core::{Hachimi as HostHachimi, Interceptor as HostInterceptor},
    il2cpp::{
        self,
        types::{
            il2cpp_array_size_t, FieldInfo as HostFieldInfo, Il2CppClass as HostIl2CppClass,
            Il2CppImage as HostIl2CppImage, Il2CppObject as HostIl2CppObject, Il2CppThread as HostIl2CppThread,
            Il2CppTypeEnum as HostIl2CppTypeEnum,
        },
    },
};

use super::{menu, types::HachimiInitFn};

static PLUGIN_VTABLE: OnceCell<Vtable> = OnceCell::new();

unsafe extern "C" fn hachimi_instance() -> *const Hachimi {
    HostHachimi::instance().as_ref() as *const HostHachimi as *const Hachimi
}

unsafe extern "C" fn hachimi_get_interceptor(this: *const Hachimi) -> *const Interceptor {
    // SAFETY: FFI / raw pointer operation required by IL2CPP interop
    unsafe {
        let this = this as *const HostHachimi;
        &(*this).interceptor as *const HostInterceptor as *const Interceptor
    }
}

unsafe extern "C" fn interceptor_hook(
    this: *const Interceptor,
    orig_addr: *mut c_void,
    hook_addr: *mut c_void,
) -> *mut c_void {
    // SAFETY: FFI / raw pointer operation required by IL2CPP interop
    unsafe {
        let this = this as *const HostInterceptor;
        (*this)
            .hook(orig_addr as _, hook_addr as _)
            .inspect_err(|e| error!("{}", e))
            .unwrap_or(0) as _
    }
}

unsafe extern "C" fn interceptor_hook_vtable(
    this: *const Interceptor,
    vtable: *mut *mut c_void,
    vtable_index: usize,
    hook_addr: *mut c_void,
) -> *mut c_void {
    // SAFETY: FFI / raw pointer operation required by IL2CPP interop
    unsafe {
        let this = this as *const HostInterceptor;
        (*this)
            .hook_vtable(vtable as _, vtable_index as _, hook_addr as _)
            .inspect_err(|e| error!("{}", e))
            .unwrap_or(0) as _
    }
}

unsafe extern "C" fn interceptor_get_trampoline_addr(this: *const Interceptor, hook_addr: *mut c_void) -> *mut c_void {
    // SAFETY: FFI / raw pointer operation required by IL2CPP interop
    unsafe {
        let this = this as *const HostInterceptor;
        (*this).get_trampoline_addr(hook_addr as _) as _
    }
}

unsafe extern "C" fn interceptor_unhook(this: *const Interceptor, hook_addr: *mut c_void) -> *mut c_void {
    // SAFETY: FFI / raw pointer operation required by IL2CPP interop
    unsafe {
        let this = this as *const HostInterceptor;
        if let Some(handle) = (*this).unhook(hook_addr as _) {
            handle.orig_addr as _
        } else {
            0 as _
        }
    }
}

unsafe extern "C" fn il2cpp_resolve_symbol(name: *const c_char) -> *mut c_void {
    // SAFETY: FFI / raw pointer operation required by IL2CPP interop
    unsafe {
        let Ok(name) = CStr::from_ptr(name).to_str() else {
            return 0 as _;
        };
        il2cpp::symbols::dlsym(name) as _
    }
}

unsafe extern "C" fn il2cpp_get_assembly_image(assembly_name: *const c_char) -> *const Il2CppImage {
    // SAFETY: FFI / raw pointer operation required by IL2CPP interop
    unsafe {
        il2cpp::symbols::get_assembly_image(CStr::from_ptr(assembly_name))
            .inspect_err(|e| error!("{}", e))
            .map_or(std::ptr::null(), |p| p as *const Il2CppImage)
    }
}

unsafe extern "C" fn il2cpp_get_class(
    image: *const Il2CppImage,
    namespace: *const c_char,
    class_name: *const c_char,
) -> *mut Il2CppClass {
    // SAFETY: FFI / raw pointer operation required by IL2CPP interop
    unsafe {
        let image = image as *const HostIl2CppImage;
        il2cpp::symbols::get_class(image, CStr::from_ptr(namespace), CStr::from_ptr(class_name))
            .inspect_err(|e| error!("{}", e))
            .map_or(std::ptr::null_mut(), |p| p as *mut Il2CppClass)
    }
}

unsafe extern "C" fn il2cpp_get_method(
    class: *mut Il2CppClass,
    name: *const c_char,
    args_count: i32,
) -> *const MethodInfo {
    // SAFETY: FFI / raw pointer operation required by IL2CPP interop
    unsafe {
        let class = class as *mut HostIl2CppClass;
        il2cpp::symbols::get_method(class, CStr::from_ptr(name), args_count)
            .inspect_err(|e| error!("{}", e))
            .map_or(std::ptr::null(), |p| p as *const MethodInfo)
    }
}

unsafe extern "C" fn il2cpp_get_method_overload(
    class: *mut Il2CppClass,
    name: *const c_char,
    params: *const Il2CppTypeEnum,
    param_count: usize,
) -> *const MethodInfo {
    // SAFETY: FFI / raw pointer operation required by IL2CPP interop
    unsafe {
        let class = class as *mut HostIl2CppClass;
        let name = CStr::from_ptr(name).to_string_lossy();
        let params = std::slice::from_raw_parts(params as *const HostIl2CppTypeEnum, param_count);
        il2cpp::symbols::get_method_overload(class, &name, params)
            .inspect_err(|e| error!("{}", e))
            .map_or(std::ptr::null(), |p| p as *const MethodInfo)
    }
}

unsafe extern "C" fn il2cpp_get_method_addr(
    class: *mut Il2CppClass,
    name: *const c_char,
    args_count: i32,
) -> *mut c_void {
    // SAFETY: FFI / raw pointer operation required by IL2CPP interop
    unsafe {
        let class = class as *mut HostIl2CppClass;
        il2cpp::symbols::get_method_addr(class, CStr::from_ptr(name), args_count) as _
    }
}

unsafe extern "C" fn il2cpp_get_method_overload_addr(
    class: *mut Il2CppClass,
    name: *const c_char,
    params: *const Il2CppTypeEnum,
    param_count: usize,
) -> *mut c_void {
    // SAFETY: FFI / raw pointer operation required by IL2CPP interop
    unsafe {
        let class = class as *mut HostIl2CppClass;
        let name = CStr::from_ptr(name).to_string_lossy();
        let params = std::slice::from_raw_parts(params as *const HostIl2CppTypeEnum, param_count);
        il2cpp::symbols::get_method_overload_addr(class, &name, params) as _
    }
}

unsafe extern "C" fn il2cpp_get_method_cached(
    class: *mut Il2CppClass,
    name: *const c_char,
    args_count: i32,
) -> *const MethodInfo {
    // SAFETY: FFI / raw pointer operation required by IL2CPP interop
    unsafe {
        let class = class as *mut HostIl2CppClass;
        il2cpp::symbols::get_method_cached(class, CStr::from_ptr(name), args_count)
            .inspect_err(|e| error!("{}", e))
            .map_or(std::ptr::null(), |p| p as *const MethodInfo)
    }
}

unsafe extern "C" fn il2cpp_get_method_addr_cached(
    class: *mut Il2CppClass,
    name: *const c_char,
    args_count: i32,
) -> *mut c_void {
    // SAFETY: FFI / raw pointer operation required by IL2CPP interop
    unsafe {
        let class = class as *mut HostIl2CppClass;
        il2cpp::symbols::get_method_addr_cached(class, CStr::from_ptr(name), args_count) as _
    }
}

unsafe extern "C" fn il2cpp_find_nested_class(class: *mut Il2CppClass, name: *const c_char) -> *mut Il2CppClass {
    // SAFETY: FFI / raw pointer operation required by IL2CPP interop
    unsafe {
        let class = class as *mut HostIl2CppClass;
        il2cpp::symbols::find_nested_class(class, CStr::from_ptr(name))
            .inspect_err(|e| error!("{}", e))
            .map_or(std::ptr::null_mut(), |p| p as *mut Il2CppClass)
    }
}

unsafe extern "C" fn il2cpp_resolve_icall(name: *const c_char) -> Il2CppMethodPointer {
    il2cpp::api::il2cpp_resolve_icall(name) as Il2CppMethodPointer
}

unsafe extern "C" fn il2cpp_class_get_methods(klass: *mut Il2CppClass, iter: *mut *mut c_void) -> *const MethodInfo {
    let klass = klass as *mut HostIl2CppClass;
    il2cpp::api::il2cpp_class_get_methods(klass, iter) as *const MethodInfo
}

unsafe extern "C" fn il2cpp_get_field_from_name(class: *mut Il2CppClass, name: *const c_char) -> *mut FieldInfo {
    let class = class as *mut HostIl2CppClass;
    il2cpp::api::il2cpp_class_get_field_from_name(class, name) as *mut FieldInfo
}

unsafe extern "C" fn il2cpp_get_field_value(obj: *mut Il2CppObject, field: *mut FieldInfo, out_value: *mut c_void) {
    let obj = obj as *mut HostIl2CppObject;
    let field = field as *mut HostFieldInfo;
    il2cpp::api::il2cpp_field_get_value(obj, field, out_value)
}

unsafe extern "C" fn il2cpp_set_field_value(obj: *mut Il2CppObject, field: *mut FieldInfo, value: *const c_void) {
    let obj = obj as *mut HostIl2CppObject;
    let field = field as *mut HostFieldInfo;
    il2cpp::api::il2cpp_field_set_value(obj, field, value as _)
}

unsafe extern "C" fn il2cpp_get_static_field_value(field: *mut FieldInfo, out_value: *mut c_void) {
    let field = field as *mut HostFieldInfo;
    il2cpp::api::il2cpp_field_static_get_value(field, out_value)
}

unsafe extern "C" fn il2cpp_set_static_field_value(field: *mut FieldInfo, value: *const c_void) {
    let field = field as *mut HostFieldInfo;
    il2cpp::api::il2cpp_field_static_set_value(field, value as _)
}

unsafe extern "C" fn il2cpp_object_new(klass: *const Il2CppClass) -> *mut Il2CppObject {
    let klass = klass as *const HostIl2CppClass;
    il2cpp::api::il2cpp_object_new(klass) as *mut Il2CppObject
}

unsafe extern "C" fn il2cpp_unbox(obj: *mut Il2CppObject) -> *mut c_void {
    let obj = obj as *mut HostIl2CppObject;
    il2cpp::api::il2cpp_object_unbox(obj)
}

unsafe extern "C" fn il2cpp_get_main_thread() -> *mut Il2CppThread {
    il2cpp::symbols::Thread::main_thread().as_raw() as *mut Il2CppThread
}

unsafe extern "C" fn il2cpp_get_attached_threads(out_size: *mut usize) -> *mut *mut Il2CppThread {
    il2cpp::api::il2cpp_thread_get_all_attached_threads(out_size) as *mut *mut Il2CppThread
}

unsafe extern "C" fn il2cpp_schedule_on_thread(thread: *mut Il2CppThread, callback: unsafe extern "C" fn()) {
    // SAFETY: FFI / raw pointer operation required by IL2CPP interop
    unsafe {
        let thread = thread as *mut HostIl2CppThread;
        il2cpp::symbols::Thread::from_raw(thread).schedule(std::mem::transmute(callback));
    }
}

unsafe extern "C" fn il2cpp_create_array(
    element_type: *mut Il2CppClass,
    length: il2cpp_array_size_t,
) -> *mut Il2CppArray {
    let element_type = element_type as *mut HostIl2CppClass;
    il2cpp::api::il2cpp_array_new(element_type, length) as *mut Il2CppArray
}

unsafe extern "C" fn il2cpp_get_singleton_like_instance(class: *mut Il2CppClass) -> *mut Il2CppObject {
    let class = class as *mut HostIl2CppClass;
    il2cpp::symbols::SingletonLike::new(class).map_or(std::ptr::null_mut(), |s| s.instance() as *mut Il2CppObject)
}

unsafe extern "C" fn log(level: i32, target: *const c_char, message: *const c_char) {
    // SAFETY: FFI / raw pointer operation required by IL2CPP interop
    unsafe {
        let target = CStr::from_ptr(target).to_string_lossy();
        let message = CStr::from_ptr(message).to_string_lossy();
        let level = match level {
            1 => log::Level::Error,
            2 => log::Level::Warn,
            3 => log::Level::Info,
            4 => log::Level::Debug,
            5 => log::Level::Trace,

            _ => log::Level::Info,
        };
        log!(target: &target, level, "{}", message);
    }
}

unsafe extern "C" fn gui_register_menu_item(
    label: *const c_char,
    callback: Option<GuiMenuCallback>,
    userdata: *mut c_void,
) -> bool {
    // SAFETY: FFI / raw pointer operation required by IL2CPP interop
    unsafe {
        if label.is_null() {
            return false;
        }
        let Ok(label) = CStr::from_ptr(label).to_str() else {
            return false;
        };
        menu::register_plugin_menu_item(label.to_owned(), callback, userdata);
        true
    }
}

unsafe extern "C" fn gui_register_menu_section(
    callback: Option<GuiMenuSectionCallback>,
    userdata: *mut c_void,
) -> bool {
    let Some(callback) = callback else {
        return false;
    };
    menu::register_plugin_menu_section(callback, userdata);
    true
}

unsafe extern "C" fn gui_show_notification(message: *const c_char) -> bool {
    // SAFETY: FFI / raw pointer operation required by IL2CPP interop
    unsafe {
        if message.is_null() {
            return false;
        }
        let Ok(message) = CStr::from_ptr(message).to_str() else {
            return false;
        };
        super::notification::enqueue(message.to_owned());
        true
    }
}

unsafe fn ui_from_ptr<'a>(ui: *mut c_void) -> Option<&'a mut egui::Ui> {
    // SAFETY: FFI / raw pointer operation required by IL2CPP interop
    unsafe {
        if ui.is_null() {
            return None;
        }
        Some(&mut *(ui as *mut egui::Ui))
    }
}

unsafe fn cstr_or_empty(ptr: *const c_char) -> &'static str {
    // SAFETY: FFI / raw pointer operation required by IL2CPP interop
    unsafe {
        if ptr.is_null() {
            return "";
        }
        CStr::from_ptr(ptr).to_str().unwrap_or("")
    }
}

unsafe extern "C" fn gui_ui_heading(ui: *mut c_void, text: *const c_char) -> bool {
    // SAFETY: FFI / raw pointer operation required by IL2CPP interop
    unsafe {
        let Some(ui) = ui_from_ptr(ui) else {
            return false;
        };
        ui.heading(cstr_or_empty(text));
        true
    }
}

unsafe extern "C" fn gui_ui_label(ui: *mut c_void, text: *const c_char) -> bool {
    // SAFETY: FFI / raw pointer operation required by IL2CPP interop
    unsafe {
        let Some(ui) = ui_from_ptr(ui) else {
            return false;
        };
        ui.label(cstr_or_empty(text));
        true
    }
}

unsafe extern "C" fn gui_ui_small(ui: *mut c_void, text: *const c_char) -> bool {
    // SAFETY: FFI / raw pointer operation required by IL2CPP interop
    unsafe {
        let Some(ui) = ui_from_ptr(ui) else {
            return false;
        };
        ui.small(cstr_or_empty(text));
        true
    }
}

unsafe extern "C" fn gui_ui_separator(ui: *mut c_void) -> bool {
    // SAFETY: FFI / raw pointer operation required by IL2CPP interop
    unsafe {
        let Some(ui) = ui_from_ptr(ui) else {
            return false;
        };
        ui.separator();
        true
    }
}

unsafe extern "C" fn gui_ui_button(ui: *mut c_void, text: *const c_char) -> bool {
    // SAFETY: FFI / raw pointer operation required by IL2CPP interop
    unsafe {
        let Some(ui) = ui_from_ptr(ui) else {
            return false;
        };
        ui.button(cstr_or_empty(text)).clicked()
    }
}

unsafe extern "C" fn gui_ui_small_button(ui: *mut c_void, text: *const c_char) -> bool {
    // SAFETY: FFI / raw pointer operation required by IL2CPP interop
    unsafe {
        let Some(ui) = ui_from_ptr(ui) else {
            return false;
        };
        ui.small_button(cstr_or_empty(text)).clicked()
    }
}

unsafe extern "C" fn gui_ui_checkbox(ui: *mut c_void, text: *const c_char, value: *mut bool) -> bool {
    // SAFETY: FFI / raw pointer operation required by IL2CPP interop
    unsafe {
        let Some(ui) = ui_from_ptr(ui) else {
            return false;
        };
        if value.is_null() {
            return false;
        }
        let mut current = *value;
        let changed = ui.checkbox(&mut current, cstr_or_empty(text)).changed();
        if changed {
            *value = current;
        }
        changed
    }
}

unsafe extern "C" fn gui_ui_text_edit_singleline(ui: *mut c_void, buffer: *mut c_char, buffer_len: usize) -> bool {
    // SAFETY: FFI / raw pointer operation required by IL2CPP interop
    unsafe {
        let Some(ui) = ui_from_ptr(ui) else {
            return false;
        };
        if buffer.is_null() || buffer_len == 0 {
            return false;
        }

        let bytes = std::slice::from_raw_parts_mut(buffer as *mut u8, buffer_len);
        let end = bytes.iter().position(|b| *b == 0).unwrap_or(buffer_len);

        let id = ui.make_persistent_id(buffer as usize);
        let mut value = ui
            .memory(|mem| mem.data.get_temp::<String>(id))
            .unwrap_or_else(|| String::from_utf8_lossy(&bytes[..end]).into_owned());
        let original_value = value.clone();

        let response = ui.add(egui::TextEdit::singleline(&mut value).id(id).desired_width(80.0));
        #[cfg(target_os = "android")]
        gui::handle_android_keyboard(&response, &mut value);

        if response.gained_focus() {
            response.scroll_to_me(Some(Align::Center));
        }

        ui.memory_mut(|mem| mem.data.insert_temp(id, value.clone()));

        let changed = value != original_value;
        if changed {
            bytes.fill(0);
            let src = value.as_bytes();
            let copy_len = src.len().min(buffer_len.saturating_sub(1));
            bytes[..copy_len].copy_from_slice(&src[..copy_len]);
        }

        changed
    }
}

unsafe extern "C" fn gui_ui_horizontal(
    ui: *mut c_void,
    callback: Option<GuiUiCallback>,
    userdata: *mut c_void,
) -> bool {
    // SAFETY: FFI / raw pointer operation required by IL2CPP interop
    unsafe {
        let Some(ui) = ui_from_ptr(ui) else {
            return false;
        };
        let Some(callback) = callback else {
            return false;
        };
        ui.horizontal(|ui| {
            callback(ui as *mut _ as *mut c_void, userdata);
        });
        true
    }
}

unsafe extern "C" fn gui_ui_grid(
    ui: *mut c_void,
    id: *const c_char,
    columns: usize,
    spacing_x: f32,
    spacing_y: f32,
    callback: Option<GuiUiCallback>,
    userdata: *mut c_void,
) -> bool {
    // SAFETY: FFI / raw pointer operation required by IL2CPP interop
    unsafe {
        let Some(ui) = ui_from_ptr(ui) else {
            return false;
        };
        let Some(callback) = callback else {
            return false;
        };
        let id = cstr_or_empty(id);
        egui::Grid::new(id)
            .num_columns(columns)
            .spacing([spacing_x, spacing_y])
            .show(ui, |ui| {
                callback(ui as *mut _ as *mut c_void, userdata);
            });
        true
    }
}

unsafe extern "C" fn gui_ui_end_row(ui: *mut c_void) -> bool {
    // SAFETY: FFI / raw pointer operation required by IL2CPP interop
    unsafe {
        let Some(ui) = ui_from_ptr(ui) else {
            return false;
        };
        ui.end_row();
        true
    }
}

unsafe extern "C" fn gui_ui_colored_label(ui: *mut c_void, r: u8, g: u8, b: u8, a: u8, text: *const c_char) -> bool {
    // SAFETY: FFI / raw pointer operation required by IL2CPP interop
    unsafe {
        let Some(ui) = ui_from_ptr(ui) else {
            return false;
        };
        ui.colored_label(egui::Color32::from_rgba_unmultiplied(r, g, b, a), cstr_or_empty(text));
        true
    }
}

unsafe extern "C" fn gui_register_menu_item_icon(
    label: *const c_char,
    icon_uri: *const c_char,
    icon_ptr: *const u8,
    icon_len: usize,
) -> bool {
    // SAFETY: FFI / raw pointer operation required by IL2CPP interop
    unsafe {
        if label.is_null() || icon_ptr.is_null() || icon_len == 0 {
            return false;
        }
        let Ok(label) = CStr::from_ptr(label).to_str() else {
            return false;
        };
        let uri = if icon_uri.is_null() {
            format!("bytes://plugin-icon/{}.png", label)
        } else {
            let Ok(uri) = CStr::from_ptr(icon_uri).to_str() else {
                return false;
            };
            uri.to_owned()
        };
        let bytes = std::slice::from_raw_parts(icon_ptr, icon_len);
        menu::register_plugin_menu_icon(label.to_owned(), uri, bytes.to_vec())
    }
}

unsafe extern "C" fn gui_register_menu_section_with_icon(
    title: *const c_char,
    icon_uri: *const c_char,
    icon_ptr: *const u8,
    icon_len: usize,
    callback: Option<GuiMenuSectionCallback>,
    userdata: *mut c_void,
) -> bool {
    // SAFETY: FFI / raw pointer operation required by IL2CPP interop
    unsafe {
        let Some(callback) = callback else {
            return false;
        };
        if title.is_null() || icon_ptr.is_null() || icon_len == 0 {
            return false;
        }
        let Ok(title) = CStr::from_ptr(title).to_str() else {
            return false;
        };
        let uri = if icon_uri.is_null() {
            format!("bytes://plugin-section/{}.png", title)
        } else {
            let Ok(uri) = CStr::from_ptr(icon_uri).to_str() else {
                return false;
            };
            uri.to_owned()
        };
        let bytes = std::slice::from_raw_parts(icon_ptr, icon_len);
        menu::register_plugin_menu_section_with_icon(title.to_owned(), uri, bytes.to_vec(), callback, userdata)
    }
}

unsafe extern "C" fn gui_register_overlay(
    id: *const c_char,
    callback: Option<GuiMenuSectionCallback>,
    userdata: *mut c_void,
) -> bool {
    // SAFETY: FFI / raw pointer operation required by IL2CPP interop
    unsafe {
        let Some(callback) = callback else {
            return false;
        };
        if id.is_null() {
            return false;
        }
        let Ok(id) = CStr::from_ptr(id).to_str() else {
            return false;
        };
        super::overlay::register_plugin_overlay(id.to_owned(), callback, userdata);
        true
    }
}

unsafe extern "C" fn gui_ui_set_min_width(ui: *mut c_void, width: f32) -> bool {
    // SAFETY: FFI / raw pointer operation required by IL2CPP interop
    unsafe {
        let Some(ui) = ui_from_ptr(ui) else {
            return false;
        };
        ui.set_min_width(width);
        true
    }
}

unsafe extern "C" fn gui_ui_set_font_size(ui: *mut c_void, size: f32) -> bool {
    // SAFETY: FFI / raw pointer operation required by IL2CPP interop
    unsafe {
        let Some(ui) = ui_from_ptr(ui) else {
            return false;
        };
        ui.style_mut().override_font_id = Some(egui::FontId::proportional(size));
        true
    }
}

unsafe extern "C" fn gui_ui_collapsing(
    ui: *mut c_void,
    heading: *const c_char,
    default_open: bool,
    callback: Option<GuiUiCallback>,
    userdata: *mut c_void,
) -> bool {
    // SAFETY: FFI / raw pointer operation required by IL2CPP interop
    unsafe {
        let Some(ui) = ui_from_ptr(ui) else {
            return false;
        };
        let Some(callback) = callback else {
            return false;
        };
        egui::CollapsingHeader::new(cstr_or_empty(heading))
            .default_open(default_open)
            .show(ui, |ui| {
                callback(ui as *mut _ as *mut c_void, userdata);
            });
        true
    }
}

unsafe extern "C" fn gui_overlay_set_visible(id: *const c_char, visible: bool) -> bool {
    // SAFETY: FFI / raw pointer operation required by IL2CPP interop
    unsafe {
        if id.is_null() {
            return false;
        }
        let Ok(id) = CStr::from_ptr(id).to_str() else {
            return false;
        };
        super::overlay::set_overlay_visible(id, visible);
        true
    }
}

#[cfg(target_os = "android")]
unsafe extern "C" fn android_dex_load(dex_ptr: *const u8, dex_len: usize, class_name: *const c_char) -> u64 {
    crate::android::dex_bridge::dex_load(dex_ptr, dex_len, class_name)
}

#[cfg(not(target_os = "android"))]
unsafe extern "C" fn android_dex_load(_dex_ptr: *const u8, _dex_len: usize, _class_name: *const c_char) -> u64 {
    0
}

#[cfg(target_os = "android")]
unsafe extern "C" fn android_dex_unload(handle: u64) -> bool {
    crate::android::dex_bridge::dex_unload(handle)
}

#[cfg(not(target_os = "android"))]
unsafe extern "C" fn android_dex_unload(_handle: u64) -> bool {
    false
}

#[cfg(target_os = "android")]
unsafe extern "C" fn android_dex_call_static_noargs(handle: u64, method: *const c_char, sig: *const c_char) -> bool {
    let method = CStr::from_ptr(method);
    let sig = CStr::from_ptr(sig);
    crate::android::dex_bridge::call_static_noargs(handle, method, sig)
}

#[cfg(not(target_os = "android"))]
unsafe extern "C" fn android_dex_call_static_noargs(_handle: u64, _method: *const c_char, _sig: *const c_char) -> bool {
    false
}

#[cfg(target_os = "android")]
unsafe extern "C" fn android_dex_call_static_string(
    handle: u64,
    method: *const c_char,
    sig: *const c_char,
    arg: *const c_char,
) -> bool {
    let method = CStr::from_ptr(method);
    let sig = CStr::from_ptr(sig);
    let arg = CStr::from_ptr(arg);
    crate::android::dex_bridge::call_static_string(handle, method, sig, arg)
}

#[cfg(not(target_os = "android"))]
unsafe extern "C" fn android_dex_call_static_string(
    _handle: u64,
    _method: *const c_char,
    _sig: *const c_char,
    _arg: *const c_char,
) -> bool {
    false
}

fn build_host_vtable() -> Vtable {
    Vtable {
        hachimi_instance,
        hachimi_get_interceptor,
        interceptor_hook,
        interceptor_hook_vtable,
        interceptor_get_trampoline_addr,
        interceptor_unhook,
        il2cpp_resolve_symbol,
        il2cpp_get_assembly_image,
        il2cpp_get_class,
        il2cpp_get_method,
        il2cpp_get_method_overload,
        il2cpp_get_method_addr,
        il2cpp_get_method_overload_addr,
        il2cpp_get_method_cached,
        il2cpp_get_method_addr_cached,
        il2cpp_find_nested_class,
        il2cpp_resolve_icall,
        il2cpp_class_get_methods,
        il2cpp_get_field_from_name,
        il2cpp_get_field_value,
        il2cpp_set_field_value,
        il2cpp_get_static_field_value,
        il2cpp_set_static_field_value,
        il2cpp_object_new,
        il2cpp_unbox,
        il2cpp_get_main_thread,
        il2cpp_get_attached_threads,
        il2cpp_schedule_on_thread,
        il2cpp_create_array,
        il2cpp_get_singleton_like_instance,
        log,
        gui_register_menu_item,
        gui_register_menu_section,
        gui_show_notification,
        gui_ui_heading,
        gui_ui_label,
        gui_ui_small,
        gui_ui_separator,
        gui_ui_button,
        gui_ui_small_button,
        gui_ui_checkbox,
        gui_ui_text_edit_singleline,
        gui_ui_horizontal,
        gui_ui_grid,
        gui_ui_end_row,
        gui_ui_colored_label,
        gui_register_menu_item_icon,
        gui_register_menu_section_with_icon,
        android_dex_load,
        android_dex_unload,
        android_dex_call_static_noargs,
        android_dex_call_static_string,
        gui_register_overlay,
        gui_ui_set_min_width,
        gui_overlay_set_visible,
        gui_ui_set_font_size,
        gui_ui_collapsing,
    }
}

pub(crate) fn init_plugin(init_fn: HachimiInitFn) -> InitResult {
    let vtable = PLUGIN_VTABLE.get_or_init(build_host_vtable);
    init_fn(vtable as *const Vtable, API_VERSION)
}
