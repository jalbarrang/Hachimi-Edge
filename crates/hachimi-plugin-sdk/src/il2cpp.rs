//! IL2CPP vtable helpers.

use std::ffi::{c_char, c_void, CString};

use hachimi_plugin_abi::{vt, Il2CppClass, Il2CppImage, Il2CppObject, MethodInfo};

use crate::Sdk;

impl Sdk {
    pub fn resolve_symbol(&self, name: &str) -> Option<*mut std::ffi::c_void> {
        let Ok(name_c) = CString::new(name) else {
            return None;
        };
        // SAFETY: Symbol name valid for host il2cpp resolver.
        let ptr = unsafe { (vt().il2cpp_resolve_symbol)(name_c.as_ptr()) };
        if ptr.is_null() {
            None
        } else {
            Some(ptr)
        }
    }

    pub fn get_assembly_image(&self, assembly: &str) -> Option<*const Il2CppImage> {
        let Ok(name_c) = CString::new(assembly) else {
            return None;
        };
        // SAFETY: Assembly name valid C string.
        let ptr = unsafe { (vt().il2cpp_get_assembly_image)(name_c.as_ptr()) };
        if ptr.is_null() {
            None
        } else {
            Some(ptr)
        }
    }

    pub fn get_class(&self, image: *const Il2CppImage, namespace: &str, class_name: &str) -> Option<*mut Il2CppClass> {
        let Ok(ns_c) = CString::new(namespace) else {
            return None;
        };
        let Ok(class_c) = CString::new(class_name) else {
            return None;
        };
        // SAFETY: Image pointer from prior host resolution.
        let ptr = unsafe { (vt().il2cpp_get_class)(image, ns_c.as_ptr(), class_c.as_ptr()) };
        if ptr.is_null() {
            None
        } else {
            Some(ptr)
        }
    }

    pub fn get_method(&self, class: *mut Il2CppClass, name: &str, args_count: i32) -> Option<*const MethodInfo> {
        let Ok(name_c) = CString::new(name) else {
            return None;
        };
        // SAFETY: Class pointer from host il2cpp.
        let ptr = unsafe { (vt().il2cpp_get_method)(class, name_c.as_ptr(), args_count) };
        if ptr.is_null() {
            None
        } else {
            Some(ptr)
        }
    }

    pub fn get_method_addr(
        &self,
        class: *mut Il2CppClass,
        name: &str,
        args_count: i32,
    ) -> Option<*mut std::ffi::c_void> {
        let Ok(name_c) = CString::new(name) else {
            return None;
        };
        // SAFETY: Class pointer from host il2cpp.
        let ptr = unsafe { (vt().il2cpp_get_method_addr)(class, name_c.as_ptr(), args_count) };
        if ptr.is_null() {
            None
        } else {
            Some(ptr)
        }
    }

    pub fn get_singleton(&self, class: *mut Il2CppClass) -> Option<*mut Il2CppObject> {
        // SAFETY: Class pointer valid.
        let ptr = unsafe { (vt().il2cpp_get_singleton_like_instance)(class) };
        if ptr.is_null() {
            None
        } else {
            Some(ptr)
        }
    }

    /// Resolve a symbol exported from `libil2cpp` / `GameAssembly`.
    pub fn dlsym(&self, name: &str) -> Option<*mut std::ffi::c_void> {
        self.resolve_symbol(name)
    }

    /// Read a field value into `out_value` (size must match field type).
    ///
    /// # Safety
    /// `obj`, `field`, and `out_value` must be valid IL2CPP pointers/sizes.
    pub unsafe fn get_field_value(
        &self,
        obj: *mut Il2CppObject,
        field: *mut hachimi_plugin_abi::FieldInfo,
        out_value: *mut std::ffi::c_void,
    ) {
        // SAFETY: Caller guarantees IL2CPP object/field validity.
        unsafe {
            (vt().il2cpp_get_field_value)(obj, field, out_value);
        }
    }

    /// Resolve `il2cpp_class_get_methods` and invoke with a C string name from the host.
    pub fn class_get_methods(&self, klass: *mut Il2CppClass, iter: *mut *mut std::ffi::c_void) -> *const MethodInfo {
        // SAFETY: Iterator pointer follows il2cpp convention.
        unsafe { (vt().il2cpp_class_get_methods)(klass, iter) }
    }

    /// Free a string returned by il2cpp introspection (when host exposes `il2cpp_free` via resolve_symbol).
    pub fn free_il2cpp_string(&self, ptr: *mut c_char) {
        if ptr.is_null() {
            return;
        }
        if let Some(free_fn) = self.resolve_symbol("il2cpp_free") {
            type Il2CppFree = unsafe extern "C" fn(*mut c_void);
            // SAFETY: `il2cpp_free` signature; pointer from il2cpp allocator APIs.
            let free_fn: Il2CppFree = unsafe { std::mem::transmute(free_fn) };
            // SAFETY: Pointer from il2cpp allocator APIs.
            unsafe {
                free_fn(ptr as *mut _);
            }
        }
    }
}
