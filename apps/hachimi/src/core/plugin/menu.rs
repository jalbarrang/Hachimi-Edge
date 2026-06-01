//! Plugin menu registration and shared menu state.
//! Stores plugin menu items, custom sections, and optional icon payloads.
//! State is guarded by `Lazy<Mutex<_>>` so plugins can register entries safely across threads.
//! GUI consumers read cloned snapshots and lookup helpers instead of owning this state.

use std::{
    collections::HashMap,
    ffi::c_void,
    sync::{Arc, Mutex},
};

use once_cell::sync::Lazy;

use super::types::{GuiMenuCallback, GuiMenuSectionCallback};

#[derive(Clone)]
pub(crate) struct PluginMenuItem {
    pub(crate) handle: u64,
    pub(crate) owner: u32,
    pub(crate) label: String,
    pub(crate) callback: Option<GuiMenuCallback>,
    pub(crate) userdata: usize,
}

#[derive(Clone)]
pub(crate) struct PluginMenuIcon {
    pub(crate) uri: String,
    pub(crate) bytes: Arc<[u8]>,
}

#[derive(Clone)]
pub(crate) struct PluginMenuSection {
    pub(crate) handle: u64,
    pub(crate) owner: u32,
    pub(crate) title: Option<String>,
    pub(crate) icon: Option<PluginMenuIcon>,
    pub(crate) callback: GuiMenuSectionCallback,
    pub(crate) userdata: usize,
}

pub(crate) static PLUGIN_MENU_ITEMS: Lazy<Mutex<Vec<PluginMenuItem>>> = Lazy::new(|| Mutex::new(Vec::new()));
pub(crate) static PLUGIN_MENU_SECTIONS: Lazy<Mutex<Vec<PluginMenuSection>>> = Lazy::new(|| Mutex::new(Vec::new()));
pub(crate) static PLUGIN_MENU_ICONS: Lazy<Mutex<HashMap<String, PluginMenuIcon>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

pub fn register_plugin_menu_item(label: String, callback: Option<GuiMenuCallback>, userdata: *mut c_void) -> u64 {
    let handle = super::next_handle();
    PLUGIN_MENU_ITEMS.lock().expect("lock poisoned").push(PluginMenuItem {
        handle,
        owner: super::current_owner(),
        label,
        callback,
        userdata: userdata as usize,
    });
    handle
}

pub fn register_plugin_menu_section(callback: GuiMenuSectionCallback, userdata: *mut c_void) -> u64 {
    let handle = super::next_handle();
    PLUGIN_MENU_SECTIONS
        .lock()
        .expect("lock poisoned")
        .push(PluginMenuSection {
            handle,
            owner: super::current_owner(),
            title: None,
            icon: None,
            callback,
            userdata: userdata as usize,
        });
    handle
}

pub fn register_plugin_menu_section_with_icon(
    title: String,
    uri: String,
    bytes: Vec<u8>,
    callback: GuiMenuSectionCallback,
    userdata: *mut c_void,
) -> u64 {
    if title.is_empty() || uri.is_empty() || bytes.is_empty() {
        return 0;
    }
    let handle = super::next_handle();
    PLUGIN_MENU_SECTIONS
        .lock()
        .expect("lock poisoned")
        .push(PluginMenuSection {
            handle,
            owner: super::current_owner(),
            title: Some(title),
            icon: Some(PluginMenuIcon {
                uri,
                bytes: bytes.into(),
            }),
            callback,
            userdata: userdata as usize,
        });
    handle
}

/// Remove all menu items and sections owned by `owner`.
pub(crate) fn remove_by_owner(owner: u32) {
    PLUGIN_MENU_ITEMS
        .lock()
        .expect("lock poisoned")
        .retain(|i| i.owner != owner);
    PLUGIN_MENU_SECTIONS
        .lock()
        .expect("lock poisoned")
        .retain(|s| s.owner != owner);
}

/// Remove a menu item or section by handle. Returns whether anything was removed.
pub(crate) fn remove_by_handle(handle: u64) -> bool {
    let mut items = PLUGIN_MENU_ITEMS.lock().expect("lock poisoned");
    let before = items.len();
    items.retain(|i| i.handle != handle);
    let mut removed = items.len() != before;
    drop(items);

    let mut sections = PLUGIN_MENU_SECTIONS.lock().expect("lock poisoned");
    let before = sections.len();
    sections.retain(|s| s.handle != handle);
    removed |= sections.len() != before;
    removed
}

pub fn register_plugin_menu_icon(label: String, uri: String, bytes: Vec<u8>) -> bool {
    if label.is_empty() || uri.is_empty() || bytes.is_empty() {
        return false;
    }
    PLUGIN_MENU_ICONS.lock().expect("lock poisoned").insert(
        label,
        PluginMenuIcon {
            uri,
            bytes: bytes.into(),
        },
    );
    true
}

pub(crate) fn get_plugin_menu_items() -> Vec<PluginMenuItem> {
    PLUGIN_MENU_ITEMS.lock().expect("lock poisoned").clone()
}

pub(crate) fn get_plugin_menu_sections() -> Vec<PluginMenuSection> {
    PLUGIN_MENU_SECTIONS.lock().expect("lock poisoned").clone()
}

pub(crate) fn get_plugin_menu_icon(label: &str) -> Option<PluginMenuIcon> {
    PLUGIN_MENU_ICONS.lock().expect("lock poisoned").get(label).cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remove_by_owner_scopes_items_and_sections() {
        let _guard = super::super::TEST_LOCK.lock().expect("lock poisoned");
        PLUGIN_MENU_ITEMS.lock().expect("lock poisoned").clear();
        PLUGIN_MENU_SECTIONS.lock().expect("lock poisoned").clear();

        {
            let _s = super::super::OwnerScope::enter(3);
            let _ = register_plugin_menu_item("x".to_owned(), None, std::ptr::null_mut());
        }
        {
            let _s = super::super::OwnerScope::enter(4);
            let _ = register_plugin_menu_item("y".to_owned(), None, std::ptr::null_mut());
        }

        remove_by_owner(3);
        let items = get_plugin_menu_items();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].owner, 4);

        PLUGIN_MENU_ITEMS.lock().expect("lock poisoned").clear();
    }
}
