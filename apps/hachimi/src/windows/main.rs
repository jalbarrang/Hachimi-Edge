use std::os::raw::{c_ulong, c_void};

use widestring::U16CString;
use windows::{
    core::PCWSTR,
    Win32::{
        Foundation::{HMODULE, TRUE},
        System::LibraryLoader::LoadLibraryW,
    },
};

use crate::{
    core::{plugin::Plugin, Hachimi},
    windows::utils,
};

use super::{hook, wnd_hook};

const DLL_PROCESS_ATTACH: c_ulong = 1;
const DLL_PROCESS_DETACH: c_ulong = 0;

pub fn load_libraries() -> Vec<Plugin> {
    let mut plugins = Vec::new();
    for name in Hachimi::instance().config.load().windows.load_libraries.iter() {
        if let Some(plugin) = load_plugin_library(name, plugins.len() as u32 + 1) {
            plugins.push(plugin);
        }
    }
    plugins
}

/// Load a single library by name and build its [`Plugin`] (without calling init).
/// Returns `None` if the library can't be loaded or fails the compatibility gate.
fn load_plugin_library(name: &str, id: u32) -> Option<Plugin> {
    let Ok(name_cstr) = U16CString::from_str(name) else {
        warn!("Invalid library name: {}", name);
        return None;
    };
    // SAFETY: FFI / raw pointer operation required by IL2CPP interop
    let res = unsafe { LoadLibraryW(PCWSTR(name_cstr.as_ptr())) };

    let Ok(handle) = res else {
        warn!("Failed to load library: {}", name);
        return None;
    };
    if handle.is_invalid() {
        warn!("Failed to load library: {}", name);
        return None;
    }
    info!("Loaded library: {}", name);

    let hachimi_init_addr = utils::get_proc_address(handle, c"hachimi_init");
    if hachimi_init_addr == 0 {
        return None;
    }
    let Some(_caps) = plugin_is_compatible(name, handle) else {
        crate::core::utils::notify_error(format!(
            "Plugin '{}' is incompatible with this Hachimi build and was not loaded (see log)",
            name
        ));
        return None;
    };

    Some(Plugin {
        name: name.to_owned(),
        id,
        // SAFETY: Transmute required for IL2CPP type conversion
        init_fn: unsafe { std::mem::transmute(hachimi_init_addr) },
    })
}

/// Disconnect a plugin by name: dispatch `SHUTDOWN` and drop its GUI/event
/// registrations, then forget it. Returns `false` if it was not loaded.
///
/// The plugin's DLL is **deliberately kept mapped** — the host cannot track the
/// IL2CPP hooks a plugin installs, so `FreeLibrary`-ing it while the game still
/// holds trampolines into its code would crash on the next call. A pure disconnect
/// (stop driving the plugin via GUI/events) is the most we can safely do at
/// runtime; fully removing a plugin requires restarting the game. Call only from
/// host context — never from inside the target plugin's own callback.
pub fn unload_plugin(name: &str) -> bool {
    let hachimi = Hachimi::instance();
    let mut plugins = hachimi.plugins.lock().expect("lock poisoned");
    let Some(pos) = plugins.iter().position(|p| p.name == name) else {
        warn!("unload_plugin: '{}' not loaded", name);
        return false;
    };
    let plugin = plugins.remove(pos);
    drop(plugins);
    crate::core::plugin::teardown_owner(plugin.id);
    info!("Disconnected plugin '{}' (GUI/events torn down); DLL kept mapped", name);
    true
}

/// Read a plugin's `hachimi_plugin_manifest` and decide whether it is safe to load.
///
/// A missing manifest means a pre-v9 plugin built against an incompatible vtable
/// layout, so it is refused. A present manifest must match the host abi exactly and
/// not require a newer host than we provide. On success returns the plugin's
/// declared `requested_caps`.
fn plugin_is_compatible(name: &str, handle: HMODULE) -> Option<u64> {
    use hachimi_plugin_abi::{PluginManifestFn, API_VERSION};

    let addr = utils::get_proc_address(handle, c"hachimi_plugin_manifest");
    if addr == 0 {
        error!(
            "Plugin '{}' has no manifest (pre-v9 plugin); refusing to load against host abi v{}",
            name, API_VERSION
        );
        return None;
    }

    // SAFETY: symbol resolved from the loaded plugin; signature matches the ABI.
    let manifest = unsafe {
        let f: PluginManifestFn = std::mem::transmute(addr);
        let ptr = f();
        if ptr.is_null() {
            error!("Plugin '{}' returned a null manifest; refusing to load", name);
            return None;
        }
        &*ptr
    };

    // SAFETY: manifest name/version are 'static NUL-terminated C strings.
    let (pname, pver) = unsafe {
        (
            std::ffi::CStr::from_ptr(manifest.name).to_string_lossy(),
            std::ffi::CStr::from_ptr(manifest.version).to_string_lossy(),
        )
    };
    info!(
        "Plugin '{}' = {} v{} (built against abi v{}, needs host >= v{})",
        name, pname, pver, manifest.abi_version, manifest.min_host_api
    );

    match manifest_compatibility(manifest.abi_version, manifest.min_host_api, API_VERSION) {
        Compatibility::Ok => Some(manifest.requested_caps),
        Compatibility::AbiMismatch => {
            error!(
                "Plugin '{}' built against abi v{} but host is abi v{}; refusing to load",
                pname, manifest.abi_version, API_VERSION
            );
            None
        }
        Compatibility::HostTooOld => {
            error!(
                "Plugin '{}' requires host api >= v{} but host is v{}; refusing to load",
                pname, manifest.min_host_api, API_VERSION
            );
            None
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Compatibility {
    Ok,
    /// Plugin built against a different abi layout than the host.
    AbiMismatch,
    /// Plugin needs a newer host than this one.
    HostTooOld,
}

/// Pure manifest compatibility decision. The abi must match exactly (the layout is
/// not forward/backward compatible across `API_VERSION` bumps) and the host must be
/// at least the plugin's required minimum.
fn manifest_compatibility(plugin_abi: i32, plugin_min_host_api: i32, host_api: i32) -> Compatibility {
    if plugin_abi != host_api {
        Compatibility::AbiMismatch
    } else if plugin_min_host_api > host_api {
        Compatibility::HostTooOld
    } else {
        Compatibility::Ok
    }
}

pub static mut DLL_HMODULE: HMODULE = HMODULE(0 as _);

#[no_mangle]
#[allow(non_snake_case)]
pub extern "C" fn DllMain(hmodule: HMODULE, call_reason: c_ulong, _reserved: *mut c_void) -> bool {
    if call_reason == DLL_PROCESS_ATTACH {
        // SAFETY: FFI / raw pointer operation required by IL2CPP interop
        unsafe {
            DLL_HMODULE = hmodule;
        }
        if !Hachimi::init() {
            return TRUE.into();
        }

        let hachimi = Hachimi::instance();
        *hachimi.plugins.lock().expect("lock poisoned") = load_libraries();

        hook::init();
        info!("Attach completed");
    } else if call_reason == DLL_PROCESS_DETACH && Hachimi::is_initialized() {
        crate::core::plugin::events::dispatch_shutdown();
        wnd_hook::uninit();

        info!("Unhooking everything");
        Hachimi::instance().interceptor.unhook_all();
    }
    TRUE.into()
}

#[cfg(test)]
mod tests {
    use super::{manifest_compatibility, Compatibility};

    #[test]
    fn exact_match_is_ok() {
        assert_eq!(manifest_compatibility(9, 9, 9), Compatibility::Ok);
        assert_eq!(manifest_compatibility(9, 8, 9), Compatibility::Ok);
    }

    #[test]
    fn abi_mismatch_rejected() {
        // Older plugin abi.
        assert_eq!(manifest_compatibility(8, 8, 9), Compatibility::AbiMismatch);
        // Newer plugin abi.
        assert_eq!(manifest_compatibility(10, 9, 9), Compatibility::AbiMismatch);
    }

    #[test]
    fn host_too_old_rejected() {
        // abi matches but the plugin demands a newer host minimum.
        assert_eq!(manifest_compatibility(9, 10, 9), Compatibility::HostTooOld);
    }
}
