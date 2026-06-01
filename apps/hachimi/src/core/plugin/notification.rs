//! Plugin notification queue shared with the GUI layer.
//! Plugins may enqueue messages from any thread through a `Lazy<Mutex<_>>` buffer.
//! The render thread drains the queue in batches before displaying notifications.

use std::sync::Mutex;

use once_cell::sync::Lazy;

static PLUGIN_NOTIFICATIONS: Lazy<Mutex<Vec<String>>> = Lazy::new(|| Mutex::new(Vec::new()));

pub fn enqueue(message: String) {
    PLUGIN_NOTIFICATIONS.lock().expect("lock poisoned").push(message);
}

pub(crate) fn drain() -> Vec<String> {
    let mut notifications = PLUGIN_NOTIFICATIONS.lock().expect("lock poisoned");
    std::mem::take(&mut *notifications)
}
