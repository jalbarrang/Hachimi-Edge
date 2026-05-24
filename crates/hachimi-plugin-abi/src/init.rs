//! Runtime vtable pointer set during `hachimi_init`.

use crate::Vtable;

static mut VTABLE: *const Vtable = std::ptr::null();

/// Store the host vtable pointer. Must be called once from `hachimi_init` before any other API use.
///
/// # Safety
/// `vt` must point to a valid `Vtable` for the process lifetime.
pub unsafe fn set_vtable(vt: *const Vtable) {
    // SAFETY: Called once from hachimi_init before concurrent plugin code runs.
    unsafe {
        VTABLE = vt;
    }
}

/// Access the vtable installed by the host.
///
/// # Panics
/// In debug builds, if `set_vtable` was not called.
#[inline]
pub fn vt() -> &'static Vtable {
    // SAFETY: Plugin reads the vtable pointer set during init.
    unsafe {
        debug_assert!(
            !VTABLE.is_null(),
            "vtable not initialized — call set_vtable from hachimi_init"
        );
        &*VTABLE
    }
}

/// Access the vtable if initialization has completed.
#[inline]
pub fn try_vt() -> Option<&'static Vtable> {
    // SAFETY: Null check before dereference.
    unsafe {
        if VTABLE.is_null() {
            None
        } else {
            Some(&*VTABLE)
        }
    }
}
