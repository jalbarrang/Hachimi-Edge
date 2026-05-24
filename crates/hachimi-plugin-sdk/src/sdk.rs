//! Global `Sdk` instance holding the vtable and negotiated API version.

use std::ffi::{c_void, CString};
use std::sync::OnceLock;

use hachimi_plugin_abi::{set_vtable, vt, InitResult, Vtable};

use crate::ApiVersion;

static SDK: OnceLock<Sdk> = OnceLock::new();

/// Errors returned when plugin initialization fails.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum InitError {
    NullVtable,
    AlreadyInitialized,
}

/// Plugin runtime handle: vtable access and version-aware helpers.
pub struct Sdk {
    version: ApiVersion,
}

impl Sdk {
    /// Install the host vtable and API version. Call once from `hachimi_init`.
    ///
    /// # Safety
    /// `vtable_ptr` must point to a valid `Vtable` for the process lifetime.
    pub unsafe fn init(vtable_ptr: *const Vtable, version: i32) -> Result<(), InitError> {
        if vtable_ptr.is_null() {
            return Err(InitError::NullVtable);
        }
        // SAFETY: Caller guarantees a valid vtable pointer for process lifetime.
        unsafe {
            set_vtable(vtable_ptr);
        }
        let sdk = Self {
            version: ApiVersion::new(version),
        };
        SDK.set(sdk).map_err(|_| InitError::AlreadyInitialized)
    }

    /// Global SDK instance after successful [`Self::init`].
    ///
    /// # Panics
    /// If `init` was not called.
    #[must_use]
    pub fn get() -> &'static Self {
        SDK.get().expect("Sdk::init not called")
    }

    /// Global SDK if initialization completed.
    #[must_use]
    pub fn try_get() -> Option<&'static Self> {
        SDK.get()
    }

    #[must_use]
    pub fn version(&self) -> ApiVersion {
        self.version
    }

    #[must_use]
    pub fn vtable(&self) -> &'static Vtable {
        vt()
    }

    /// Log at info level with the given target.
    pub fn log_info(&self, target: &str, message: &str) {
        let Ok(msg_c) = CString::new(message) else {
            return;
        };
        let Ok(target_c) = CString::new(target) else {
            return;
        };
        // SAFETY: Host vtable `log` slot is valid after init.
        unsafe {
            (vt().log)(hachimi_plugin_abi::log_level::INFO, target_c.as_ptr(), msg_c.as_ptr());
        }
    }

    /// Show a host notification toast.
    pub fn show_notification(&self, message: &str) -> bool {
        let Ok(msg_c) = CString::new(message) else {
            return false;
        };
        // SAFETY: Host vtable `gui_show_notification` slot is valid after init.
        unsafe { (vt().gui_show_notification)(msg_c.as_ptr()) }
    }

    /// Render small text in an egui `Ui` pointer passed from the host.
    pub fn gui_small(&self, ui: *mut c_void, text: &str) -> bool {
        let Ok(text_c) = CString::new(text) else {
            return false;
        };
        // SAFETY: `ui` is a valid egui Ui pointer from the host callback.
        unsafe { (vt().gui_ui_small)(ui, text_c.as_ptr()) }
    }

    /// Render a heading in an egui `Ui` pointer.
    pub fn gui_heading(&self, ui: *mut c_void, text: &str) -> bool {
        let Ok(text_c) = CString::new(text) else {
            return false;
        };
        // SAFETY: `ui` is a valid egui Ui pointer from the host callback.
        unsafe { (vt().gui_ui_heading)(ui, text_c.as_ptr()) }
    }

    /// Render a label in an egui `Ui` pointer.
    pub fn gui_label(&self, ui: *mut c_void, text: &str) -> bool {
        let Ok(text_c) = CString::new(text) else {
            return false;
        };
        // SAFETY: `ui` is a valid egui Ui pointer from the host callback.
        unsafe { (vt().gui_ui_label)(ui, text_c.as_ptr()) }
    }

    /// Register a menu section callback.
    pub fn register_menu_section(
        &self,
        callback: hachimi_plugin_abi::GuiMenuSectionCallback,
        userdata: *mut c_void,
    ) -> bool {
        // SAFETY: Callback lifetime managed by plugin; host stores until unload.
        unsafe { (vt().gui_register_menu_section)(Some(callback), userdata) }
    }

    /// Register an overlay when the host API supports it (v3+).
    pub fn register_overlay(
        &self,
        id: &str,
        callback: hachimi_plugin_abi::GuiMenuSectionCallback,
        userdata: *mut c_void,
    ) -> bool {
        if !self.version.supports_overlay() {
            return false;
        }
        let Ok(id_c) = CString::new(id) else {
            return false;
        };
        // SAFETY: Callback lifetime managed by plugin.
        unsafe { (vt().gui_register_overlay)(id_c.as_ptr(), Some(callback), userdata) }
    }
}

/// Convert init result to raw `i32` for the C entry point.
#[must_use]
pub const fn init_result_to_i32(result: InitResult) -> i32 {
    result as i32
}
