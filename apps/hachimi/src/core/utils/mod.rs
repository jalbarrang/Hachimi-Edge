//! General-purpose helpers for the core crate, grouped by domain:
//! - [`text`]: markup-aware wrapping/fitting/truncation and string measurement
//! - [`fs`]: filesystem/path helpers and game data paths
//! - [`image`]: minimal PNG loading
//! - [`math`]: numeric scaling helpers
//!
//! The submodules are re-exported flatly so existing `utils::*` call sites keep working.

mod fs;
mod image;
mod math;
mod text;

pub use fs::*;
pub use image::*;
pub use math::*;
pub use text::*;

use std::sync::Mutex;

use fnv::FnvHashMap;
use once_cell::sync::Lazy;

use crate::{
    core::Gui,
    il2cpp::{
        ext::Il2CppStringExt,
        hook::umamusume::{Localize, TextId},
        symbols::Thread,
        types::Il2CppObject,
    },
};

#[repr(transparent)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct SendPtr(pub *mut Il2CppObject);

// SAFETY: IL2CPP object pointers are safe to send across threads as the runtime manages their lifecycle
unsafe impl Send for SendPtr {}
// SAFETY: IL2CPP object pointers are safe to share across threads as the runtime manages their lifecycle
unsafe impl Sync for SendPtr {}

static LOCALIZE_ID_CACHE: Lazy<Mutex<FnvHashMap<String, i32>>> = Lazy::new(|| Mutex::new(FnvHashMap::default()));

pub fn get_localized_string(id_name: &str) -> String {
    let check_cache = |name: &str| -> Option<String> {
        let cache = LOCALIZE_ID_CACHE.lock().expect("lock poisoned");
        if let Some(&id) = cache.get(name) {
            let ptr = Localize::Get(id);
            if !ptr.is_null() {
                // SAFETY: FFI / raw pointer operation required by IL2CPP interop
                return Some(unsafe { (*ptr).as_utf16str() }.to_string());
            }
            return Some(name.to_owned());
        }
        None
    };

    if let Some(result) = check_cache(id_name) {
        return result;
    }

    let id_name_owned = id_name.to_owned();
    static PENDING_NAME: Mutex<Option<String>> = Mutex::new(None);
    *PENDING_NAME.lock().expect("lock poisoned") = Some(id_name_owned);

    Thread::main_thread().schedule(|| {
        if let Some(name) = PENDING_NAME.lock().expect("lock poisoned").take() {
            let val = TextId::from_name(&name);
            LOCALIZE_ID_CACHE.lock().expect("lock poisoned").insert(name, val);
        }
    });

    check_cache(id_name).unwrap_or_else(|| id_name.to_owned())
}

pub fn print_json_entry(key: &str, value: &str) {
    info!(
        "{}: {},",
        serde_json::to_string(key).expect("valid UTF-8"),
        serde_json::to_string(value).expect("valid UTF-8")
    );
}

pub fn notify_error(message: impl AsRef<str>) {
    let s = message.as_ref();
    error!("{}", s);
    if let Some(mutex) = Gui::instance() {
        mutex.lock().expect("lock poisoned").show_notification(s);
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_methods)]
mod tests {
    use super::*;

    #[test]
    fn send_ptr_is_send_sync() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}
        assert_send::<SendPtr>();
        assert_sync::<SendPtr>();
    }
}
