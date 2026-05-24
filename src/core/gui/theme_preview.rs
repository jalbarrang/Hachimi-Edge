use std::sync::Mutex;

use crate::core::hachimi;

static PENDING_THEME: Mutex<Option<hachimi::Config>> = Mutex::new(None);

pub fn enqueue_theme_preview(config: hachimi::Config) {
    if let Ok(mut lock) = PENDING_THEME.lock() {
        *lock = Some(config);
    }
}

pub(crate) fn take_pending_theme() -> Option<hachimi::Config> {
    PENDING_THEME.lock().ok().and_then(|mut lock| lock.take())
}
