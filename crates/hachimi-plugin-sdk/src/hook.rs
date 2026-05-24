//! Interceptor / hook helpers.

use std::ffi::c_void;

use hachimi_plugin_abi::{vt, Hachimi, Interceptor};

use crate::Sdk;

impl Sdk {
    #[must_use]
    pub fn hachimi_instance(&self) -> *const Hachimi {
        // SAFETY: Host returns valid singleton for process lifetime.
        unsafe { (vt().hachimi_instance)() }
    }

    #[must_use]
    pub fn interceptor(&self) -> *const Interceptor {
        let hachimi = self.hachimi_instance();
        // SAFETY: Host singleton valid after init.
        unsafe { (vt().hachimi_get_interceptor)(hachimi) }
    }

    /// Install a detour hook; returns trampoline address or null on failure.
    pub fn hook(&self, orig_addr: *mut c_void, hook_addr: *mut c_void) -> Option<*mut c_void> {
        let interceptor = self.interceptor();
        // SAFETY: Addresses are valid code pointers from il2cpp resolution.
        let tramp = unsafe { (vt().interceptor_hook)(interceptor, orig_addr, hook_addr) };
        if tramp.is_null() {
            None
        } else {
            Some(tramp)
        }
    }

    #[must_use]
    pub fn trampoline_addr(&self, hook_addr: *mut c_void) -> Option<*mut c_void> {
        let interceptor = self.interceptor();
        // SAFETY: Hook was installed via same interceptor.
        let addr = unsafe { (vt().interceptor_get_trampoline_addr)(interceptor, hook_addr) };
        if addr.is_null() {
            None
        } else {
            Some(addr)
        }
    }
}
