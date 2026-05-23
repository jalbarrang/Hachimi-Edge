use std::{
    collections::HashMap,
    ffi::c_void,
    sync::{Arc, Mutex},
};

use once_cell::sync::Lazy;

use super::types::{GuiMenuCallback, GuiMenuSectionCallback};

#[derive(Clone)]
pub(crate) struct PluginMenuItem {
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
    pub(crate) title: Option<String>,
    pub(crate) icon: Option<PluginMenuIcon>,
    pub(crate) callback: GuiMenuSectionCallback,
    pub(crate) userdata: usize,
}

pub(crate) static PLUGIN_MENU_ITEMS: Lazy<Mutex<Vec<PluginMenuItem>>> = Lazy::new(|| Mutex::new(Vec::new()));
pub(crate) static PLUGIN_MENU_SECTIONS: Lazy<Mutex<Vec<PluginMenuSection>>> = Lazy::new(|| Mutex::new(Vec::new()));
pub(crate) static PLUGIN_MENU_ICONS: Lazy<Mutex<HashMap<String, PluginMenuIcon>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

pub fn register_plugin_menu_item(label: String, callback: Option<GuiMenuCallback>, userdata: *mut c_void) {
    PLUGIN_MENU_ITEMS.lock().expect("lock poisoned").push(PluginMenuItem {
        label,
        callback,
        userdata: userdata as usize,
    });
}

pub fn register_plugin_menu_section(callback: GuiMenuSectionCallback, userdata: *mut c_void) {
    PLUGIN_MENU_SECTIONS
        .lock()
        .expect("lock poisoned")
        .push(PluginMenuSection {
            title: None,
            icon: None,
            callback,
            userdata: userdata as usize,
        });
}

pub fn register_plugin_menu_section_with_icon(
    title: String,
    uri: String,
    bytes: Vec<u8>,
    callback: GuiMenuSectionCallback,
    userdata: *mut c_void,
) -> bool {
    if title.is_empty() || uri.is_empty() || bytes.is_empty() {
        return false;
    }
    PLUGIN_MENU_SECTIONS
        .lock()
        .expect("lock poisoned")
        .push(PluginMenuSection {
            title: Some(title),
            icon: Some(PluginMenuIcon {
                uri,
                bytes: bytes.into(),
            }),
            callback,
            userdata: userdata as usize,
        });
    true
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
