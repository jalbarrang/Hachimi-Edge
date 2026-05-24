//! Shared Rust-side plugin SDK types.
//! Defines plugin metadata, init result values, and callback signatures used by the ABI.
//! These types are referenced by `api` and by plugin loading code.

pub use hachimi_plugin_abi::{
    GuiMenuCallback, GuiMenuSectionCallback, GuiUiCallback, HachimiInitFn, InitResult, Vtable,
};

pub struct Plugin {
    pub name: String,
    pub init_fn: HachimiInitFn,
}

impl Plugin {
    pub fn init(&self) -> InitResult {
        super::api::init_plugin(self.init_fn)
    }
}
