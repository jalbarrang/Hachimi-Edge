//! C ABI surface for the plugin SDK.
//! `Vtable` is passed to plugins during init and is version-gated by `API_VERSION`.
//! The functions in this module are the host-side FFI wrappers behind that table.
//! Plugins draw GUI with the shared `egui::Ui` handed to their callbacks.

use std::ffi::{c_char, c_void, CStr};

use hachimi_plugin_abi::{capability, GuiMenuCallback, GuiMenuSectionCallback, InitResult, PluginEventFn};
use hachimi_plugin_abi::{
    FieldInfo, Hachimi, Il2CppArray, Il2CppClass, Il2CppImage, Il2CppMethodPointer, Il2CppObject, Il2CppThread,
    Il2CppTypeEnum, Interceptor, MethodInfo, Vtable, API_VERSION,
};
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

// ── Host services ──

unsafe extern "C" fn host_capabilities() -> u64 {
    capability::GUI | capability::OVERLAY | capability::EVENTS | capability::IL2CPP | capability::DATA_PATHS
}

unsafe extern "C" fn host_data_path(rel: *const c_char, out_buf: *mut c_char, buf_len: usize) -> usize {
    // SAFETY: FFI / raw pointer operation; caller-provided buffer is written within bounds.
    unsafe {
        if rel.is_null() {
            return 0;
        }
        let Ok(rel) = CStr::from_ptr(rel).to_str() else {
            return 0;
        };
        // Reject absolute/escaping paths: the service only exposes the data dir subtree.
        let rel_path = std::path::Path::new(rel);
        if rel_path.has_root() || rel.split(['/', '\\']).any(|c| c == "..") {
            return 0;
        }
        let path = HostHachimi::instance().get_data_path(rel);
        let s = path.to_string_lossy();
        let bytes = s.as_bytes();
        let needed = bytes.len();
        if !out_buf.is_null() && buf_len > 0 {
            let copy = needed.min(buf_len - 1);
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), out_buf as *mut u8, copy);
            *out_buf.add(copy) = 0; // NUL terminator
        }
        needed
    }
}

unsafe extern "C" fn host_subscribe(event_id: u32, callback: PluginEventFn, userdata: *mut c_void) -> u64 {
    super::events::subscribe(event_id, callback, userdata)
}

unsafe extern "C" fn host_view_name(view_id: i32) -> *const c_char {
    crate::core::scene_views::view_name_cstr(view_id).map_or(std::ptr::null(), CStr::as_ptr)
}

unsafe extern "C" fn host_unsubscribe(handle: u64) {
    super::events::unsubscribe(handle);
}

unsafe extern "C" fn gui_unregister(handle: u64) -> bool {
    super::unregister(handle)
}

// ── GUI registration ──

unsafe extern "C" fn gui_register_menu_item(
    label: *const c_char,
    callback: Option<GuiMenuCallback>,
    userdata: *mut c_void,
) -> u64 {
    // SAFETY: FFI / raw pointer operation required by IL2CPP interop
    unsafe {
        if label.is_null() {
            return 0;
        }
        let Ok(label) = CStr::from_ptr(label).to_str() else {
            return 0;
        };
        menu::register_plugin_menu_item(label.to_owned(), callback, userdata)
    }
}

unsafe extern "C" fn gui_register_menu_section(callback: Option<GuiMenuSectionCallback>, userdata: *mut c_void) -> u64 {
    let Some(callback) = callback else {
        return 0;
    };
    menu::register_plugin_menu_section(callback, userdata)
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
) -> u64 {
    // SAFETY: FFI / raw pointer operation required by IL2CPP interop
    unsafe {
        let Some(callback) = callback else {
            return 0;
        };
        if title.is_null() || icon_ptr.is_null() || icon_len == 0 {
            return 0;
        }
        let Ok(title) = CStr::from_ptr(title).to_str() else {
            return 0;
        };
        let uri = if icon_uri.is_null() {
            format!("bytes://plugin-section/{}.png", title)
        } else {
            let Ok(uri) = CStr::from_ptr(icon_uri).to_str() else {
                return 0;
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
) -> u64 {
    // SAFETY: FFI / raw pointer operation required by IL2CPP interop
    unsafe {
        let Some(callback) = callback else {
            return 0;
        };
        if id.is_null() {
            return 0;
        }
        let Ok(id) = CStr::from_ptr(id).to_str() else {
            return 0;
        };
        super::overlay::register_plugin_overlay(id.to_owned(), callback, userdata)
    }
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
        host_capabilities,
        host_subscribe,
        host_unsubscribe,
        gui_register_menu_item,
        gui_register_menu_section,
        gui_register_menu_item_icon,
        gui_register_menu_section_with_icon,
        gui_register_overlay,
        gui_unregister,
        gui_show_notification,
        gui_overlay_set_visible,
        host_data_path,
        host_view_name,
    }
}

pub(crate) fn init_plugin(init_fn: HachimiInitFn) -> InitResult {
    let vtable = PLUGIN_VTABLE.get_or_init(build_host_vtable);
    init_fn(vtable as *const Vtable, API_VERSION)
}
