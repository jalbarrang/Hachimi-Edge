//! Shared Rust-side plugin SDK types.
//! Defines plugin metadata, init result values, and callback signatures used by the ABI.
//! These types are referenced by `api` and by plugin loading code.

use std::ffi::c_void;

use super::api::{init_plugin, Vtable};

pub type HachimiInitFn = extern "C" fn(vtable: *const Vtable, version: i32) -> InitResult;
pub type GuiMenuCallback = extern "C" fn(userdata: *mut c_void);
pub type GuiMenuSectionCallback = extern "C" fn(ui: *mut c_void, userdata: *mut c_void);
pub type GuiUiCallback = extern "C" fn(ui: *mut c_void, userdata: *mut c_void);

#[repr(i32)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum InitResult {
    Error,
    Ok,
}

impl InitResult {
    pub fn is_ok(&self) -> bool {
        matches!(self, Self::Ok)
    }
}

pub struct Plugin {
    pub name: String,
    pub init_fn: HachimiInitFn,
}

impl Plugin {
    pub fn init(&self) -> InitResult {
        init_plugin(self.init_fn)
    }
}
