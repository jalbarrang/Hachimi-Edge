//! Global `Sdk` instance holding the vtable and negotiated API version.

use std::ffi::{c_void, CString};
use std::sync::OnceLock;

use hachimi_plugin_abi::{log_level, set_vtable, vt, GuiMenuSectionCallback, InitResult, PluginEventFn, Vtable};

use crate::ApiVersion;

static SDK: OnceLock<Sdk> = OnceLock::new();

/// Errors returned when plugin initialization fails.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum InitError {
    NullVtable,
    AlreadyInitialized,
    /// Host `version` is below the plugin's required minimum.
    HostApiTooOld {
        required: i32,
        actual: i32,
    },
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
        // SAFETY: Caller guarantees a valid vtable pointer for process lifetime.
        unsafe { Self::init_min(vtable_ptr, version, i32::MIN) }
    }

    /// Install the vtable after verifying the host API is new enough.
    ///
    /// # Safety
    /// `vtable_ptr` must point to a valid `Vtable` for the process lifetime.
    pub unsafe fn init_min(vtable_ptr: *const Vtable, version: i32, min_host_api: i32) -> Result<(), InitError> {
        if vtable_ptr.is_null() {
            return Err(InitError::NullVtable);
        }
        if version < min_host_api {
            return Err(InitError::HostApiTooOld {
                required: min_host_api,
                actual: version,
            });
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

    // ── Logging ──

    fn log(&self, level: i32, target: &str, message: &str) {
        let (Ok(msg_c), Ok(target_c)) = (CString::new(message), CString::new(target)) else {
            return;
        };
        // SAFETY: host vtable `log` slot is valid after init.
        unsafe { (vt().log)(level, target_c.as_ptr(), msg_c.as_ptr()) }
    }

    pub fn log_info(&self, target: &str, message: &str) {
        self.log(log_level::INFO, target, message);
    }

    pub fn log_warn(&self, target: &str, message: &str) {
        self.log(log_level::WARN, target, message);
    }

    pub fn log_error(&self, target: &str, message: &str) {
        self.log(log_level::ERROR, target, message);
    }

    // ── Capabilities ──

    /// Bitset of host capabilities (see [`hachimi_plugin_abi::capability`]).
    #[must_use]
    pub fn capabilities(&self) -> u64 {
        // SAFETY: host vtable slot is valid after init.
        unsafe { (vt().host_capabilities)() }
    }

    /// Whether the host advertises a given capability bit.
    #[must_use]
    pub fn has_capability(&self, cap: u64) -> bool {
        self.capabilities() & cap != 0
    }

    // ── Events ──

    /// Subscribe to a host event (see [`hachimi_plugin_abi::event`]).
    /// Returns a non-zero handle, or 0 on failure. Keep it to later [`Self::off`].
    pub fn on(&self, event_id: u32, callback: PluginEventFn, userdata: *mut c_void) -> u64 {
        // SAFETY: callback lifetime is managed by the plugin until unsubscribe.
        unsafe { (vt().host_subscribe)(event_id, callback, userdata) }
    }

    /// Remove an event subscription previously returned by [`Self::on`].
    pub fn off(&self, handle: u64) {
        // SAFETY: handle was issued by `host_subscribe`.
        unsafe { (vt().host_unsubscribe)(handle) }
    }

    // ── GUI registration ──
    //
    // The host UI is a two-layer system:
    //   * **L1 — Control Center**: a hotkey-toggled modal with a Plugins tab. Each
    //     registered *page* becomes a selectable sub-nav entry there.
    //   * **L2 — floating HUD**: draggable, collapsible *panels* drawn over the game,
    //     with a global lock (click-through) and persisted positions.
    //
    // `register_page` / `register_panel` are the preferred names; the callback
    // receives a host `Ui` pointer to cast with [`crate::ui_from_ptr`].

    /// Register an **L1 page** (Plugins tab). Returns a handle (0 = fail).
    pub fn register_page(&self, callback: GuiMenuSectionCallback, userdata: *mut c_void) -> u64 {
        self.register_menu_section(callback, userdata)
    }

    /// Register an **L1 page** with a title + icon shown in the Plugins sub-nav.
    /// `icon_bytes` should be PNG data. Returns a handle (0 = fail).
    pub fn register_page_with_icon(
        &self,
        title: &str,
        icon_uri: &str,
        icon_bytes: &[u8],
        callback: GuiMenuSectionCallback,
        userdata: *mut c_void,
    ) -> u64 {
        let (Ok(title_c), Ok(uri_c)) = (CString::new(title), CString::new(icon_uri)) else {
            return 0;
        };
        // SAFETY: pointers are valid for the duration of the call; host copies the data.
        unsafe {
            (vt().gui_register_menu_section_with_icon)(
                title_c.as_ptr(),
                uri_c.as_ptr(),
                icon_bytes.as_ptr(),
                icon_bytes.len(),
                Some(callback),
                userdata,
            )
        }
    }

    /// Register an **L2 panel** (floating HUD), identified by `id`. Returns a handle (0 = fail).
    pub fn register_panel(&self, id: &str, callback: GuiMenuSectionCallback, userdata: *mut c_void) -> u64 {
        self.register_overlay(id, callback, userdata)
    }

    /// Register a menu section (lower-level name for [`Self::register_page`]).
    /// The callback receives a host `Ui` pointer; cast it with [`crate::ui_from_ptr`]
    /// and draw egui directly. Returns a handle (0 = fail).
    pub fn register_menu_section(&self, callback: GuiMenuSectionCallback, userdata: *mut c_void) -> u64 {
        // SAFETY: callback lifetime managed by plugin; host stores until unregister/unload.
        unsafe { (vt().gui_register_menu_section)(Some(callback), userdata) }
    }

    /// Register an overlay window (lower-level name for [`Self::register_panel`]).
    /// Returns a handle (0 = fail).
    pub fn register_overlay(&self, id: &str, callback: GuiMenuSectionCallback, userdata: *mut c_void) -> u64 {
        let Ok(id_c) = CString::new(id) else {
            return 0;
        };
        // SAFETY: callback lifetime managed by plugin.
        unsafe { (vt().gui_register_overlay)(id_c.as_ptr(), Some(callback), userdata) }
    }

    /// Remove a menu item/section/overlay registration by its handle.
    pub fn unregister(&self, handle: u64) -> bool {
        // SAFETY: handle was issued by a `gui_register_*` slot.
        unsafe { (vt().gui_unregister)(handle) }
    }

    // ── Data paths (host API v10) ──

    /// Resolve `rel` against the game **data** directory via the host.
    ///
    /// Returns `None` if the host is too old to provide [`capability::DATA_PATHS`],
    /// or if `rel` is rejected (absolute / contains `..`). The returned path is not
    /// guaranteed to exist on disk.
    #[must_use]
    pub fn host_data_path(&self, rel: &str) -> Option<std::path::PathBuf> {
        if !self.has_capability(crate::capability::DATA_PATHS) {
            return None;
        }
        let rel_c = CString::new(rel).ok()?;
        // First call: query the required length.
        // SAFETY: host vtable slot valid after init; null out_buf is allowed.
        let needed = unsafe { (vt().host_data_path)(rel_c.as_ptr(), std::ptr::null_mut(), 0) };
        if needed == 0 {
            return None;
        }
        let mut buf = vec![0u8; needed + 1];
        // SAFETY: buf has space for `needed` bytes plus the NUL terminator.
        let written = unsafe { (vt().host_data_path)(rel_c.as_ptr(), buf.as_mut_ptr() as *mut _, buf.len()) };
        if written == 0 || written > needed {
            return None;
        }
        buf.truncate(written);
        Some(std::path::PathBuf::from(String::from_utf8(buf).ok()?))
    }

    /// Absolute path to the host's cached GameTora data directory, if available.
    #[must_use]
    pub fn gametora_data_dir(&self) -> Option<std::path::PathBuf> {
        self.host_data_path(hachimi_plugin_abi::GAMETORA_DATA_SUBDIR)
    }
}

/// Convert init result to raw `i32` for the C entry point.
#[must_use]
pub const fn init_result_to_i32(result: InitResult) -> i32 {
    result as i32
}
