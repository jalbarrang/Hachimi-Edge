//! Ergonomic Hachimi plugin SDK — safe wrappers around [`hachimi_plugin_abi`].

pub use hachimi_plugin_abi::*;

mod gui;
mod hook;
mod il2cpp;
mod sdk;
mod version;

pub use sdk::{init_result_to_i32, InitError, Sdk};
pub use version::ApiVersion;
