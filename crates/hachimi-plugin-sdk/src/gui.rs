//! GUI helpers built on the plugin vtable.

use std::ffi::{c_void, CString};

use hachimi_plugin_abi::vt;

use crate::Sdk;

impl Sdk {
    pub fn gui_separator(&self, ui: *mut c_void) -> bool {
        // SAFETY: `ui` from host overlay/menu callback.
        unsafe { (vt().gui_ui_separator)(ui) }
    }

    pub fn gui_button(&self, ui: *mut c_void, text: &str) -> bool {
        let Ok(text_c) = CString::new(text) else {
            return false;
        };
        // SAFETY: `ui` from host callback.
        unsafe { (vt().gui_ui_button)(ui, text_c.as_ptr()) }
    }

    pub fn gui_checkbox(&self, ui: *mut c_void, text: &str, value: &mut bool) -> bool {
        let Ok(text_c) = CString::new(text) else {
            return false;
        };
        // SAFETY: `ui` from host callback; `value` lives for callback duration.
        unsafe { (vt().gui_ui_checkbox)(ui, text_c.as_ptr(), value) }
    }

    pub fn gui_set_min_width(&self, ui: *mut c_void, width: f32) -> bool {
        if !self.version().supports_min_width() {
            return false;
        }
        // SAFETY: `ui` from host callback.
        unsafe { (vt().gui_ui_set_min_width)(ui, width) }
    }

    pub fn gui_set_font_size(&self, ui: *mut c_void, size: f32) -> bool {
        if !self.version().supports_font_size() {
            return false;
        }
        // SAFETY: `ui` from host callback.
        unsafe { (vt().gui_ui_set_font_size)(ui, size) }
    }

    pub fn overlay_set_visible(&self, id: &str, visible: bool) -> bool {
        if !self.version().supports_overlay_visibility() {
            return false;
        }
        let Ok(id_c) = CString::new(id) else {
            return false;
        };
        // SAFETY: Overlay id registered earlier with same string.
        unsafe { (vt().gui_overlay_set_visible)(id_c.as_ptr(), visible) }
    }
}
