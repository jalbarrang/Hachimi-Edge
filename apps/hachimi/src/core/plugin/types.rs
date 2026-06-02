//! Shared Rust-side plugin SDK types.
//! Defines plugin metadata, init result values, and callback signatures used by the ABI.
//! These types are referenced by `api` and by plugin loading code.

pub use hachimi_plugin_abi::{
    GuiMenuCallback, GuiMenuSectionCallback, GuiUiCallback, HachimiInitFn, InitResult, Vtable,
};

pub struct Plugin {
    pub name: String,
    /// Non-zero owner id used to attribute this plugin's registrations and event
    /// subscriptions (see [`super::OwnerScope`]).
    pub id: u32,
    pub init_fn: HachimiInitFn,
}

impl Plugin {
    pub fn init(&self) -> InitResult {
        // Attribute everything the plugin registers during init to its owner id.
        let _scope = super::OwnerScope::enter(self.id);
        super::api::init_plugin(self.init_fn)
    }
}
