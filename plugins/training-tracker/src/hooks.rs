//! IL2CPP hooks for intercepting training commands.
//!
//! Strategy:
//! We hook the method that the game calls when the player executes a training
//! command during career mode. The game processes training via a network
//! request/response cycle, but locally the UI calls a method to submit the
//! chosen command. We intercept at the point where `command_id` is known.
//!
//! The exact method to hook depends on the game version. This module tries
//! to resolve methods by name at runtime using the Hachimi vtable's IL2CPP
//! helpers. If resolution fails, the plugin still loads — it just won't track.
//!
//! ## Hook targets (in priority order):
//!
//! 1. **`SingleModeViewController.OnSelectCommand(int commandType, int commandId)`**
//!    — If this exists, it fires when the player taps a training button.
//!
//! 2. **`SingleModeMainViewController.OnClickTraining(int commandId)`**
//!    — Alternative name for the same concept.
//!
//! 3. **Fallback: `TrainingParamChangePlate.PlayTypeWrite`**
//!    — Already hooked by Hachimi for text. We can piggyback on the fact that
//!    this fires after a training completes, but it doesn't carry command_id
//!    directly.
//!
//! Because exact signatures depend on the game version, this module is designed
//! to be **updated** once you do an IL2CPP dump of your specific build.
//!
//! ## Cross-reference note (Trainers-Legend-G)
//!
//! TLG (136 IL2CPP hooks) does NOT hook any training-command methods. Their
//! SingleMode hooks are limited to model replacement:
//!   - `SingleModeStartResultCharaViewer.SetupImageEffect(0)`
//!   - `SingleModeSceneController.CreateModel(3)` — signature: (cardId, dressId, addVoiceCue)
//!   - `WorkSingleModeCharaData.GetRaceDressId(1)`
//!
//! This confirms our hook candidates are novel — no existing open-source mod
//! intercepts training commands. The `UmaControllerType` enum from TLG shows
//! Training=0x2 and TrainingTop=0xa as distinct controller modes.

use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::tracker::{Facility, TRACKER};
use crate::vtable::vt;

static HOOKS_INSTALLED: AtomicBool = AtomicBool::new(false);

// ---- Trampoline storage ----
// Each hook target gets its own trampoline slot.
static mut ORIG_ON_CLICK_TRAINING_MENU: *mut c_void = std::ptr::null_mut();
static mut ORIG_COMMON_SEND_COMMAND: *mut c_void = std::ptr::null_mut();
static mut ORIG_SEND_COMMAND_ASYNC: *mut c_void = std::ptr::null_mut();
static mut ORIG_ON_CLICK_TRAINING: *mut c_void = std::ptr::null_mut();

/// Hook for OnClickTrainingMenu(1) — arg is an IL2CPP object pointer.
extern "C" fn hook_on_click_training(this: *mut c_void, arg1: *mut c_void) {
    hlog_info!("[OnClickTrainingMenu] this={:?}, arg1={:?}", this, arg1);

    // SAFETY: Plugin FFI interop with Hachimi vtable
    unsafe {
        if !ORIG_ON_CLICK_TRAINING_MENU.is_null() {
            let orig: extern "C" fn(*mut c_void, *mut c_void) = std::mem::transmute(ORIG_ON_CLICK_TRAINING_MENU);
            orig(this, arg1);
        }
    }
}

/// Hook for CommonSendCommandAsync(2) — args could be objects or ints.
/// Use pointer-sized args to be safe.
extern "C" fn hook_on_select_command(this: *mut c_void, arg1: usize, arg2: usize) {
    hlog_info!("[CommonSendCommandAsync] arg1=0x{:x}, arg2=0x{:x}", arg1, arg2);

    // If values are small enough to be ints, log that interpretation too
    if arg1 < 10000 && arg2 < 10000 {
        hlog_info!("  As ints: arg1={}, arg2={}", arg1, arg2);
    }

    // SAFETY: Plugin FFI interop with Hachimi vtable
    unsafe {
        if !ORIG_COMMON_SEND_COMMAND.is_null() {
            let orig: extern "C" fn(*mut c_void, usize, usize) = std::mem::transmute(ORIG_COMMON_SEND_COMMAND);
            orig(this, arg1, arg2);
        }
    }
}

/// Hook for SendCommandAsync(6).
/// Confirmed arg layout (2026-05-23 runtime analysis):
///   arg1 = command_id (int, e.g. 106 = Wisdom)
///   arg2 = 0 (possibly command_group_id)
///   arg3 = 0 (possibly select_id)
///   arg4 = pointer (callback/continuation object)
///   arg5 = pointer (callback/continuation object)
///   arg6 = 0
extern "C" fn hook_send_command_async(
    this: *mut c_void,
    command_id: usize,
    a2: usize,
    a3: usize,
    a4: usize,
    a5: usize,
    a6: usize,
) {
    // command_id is in the int range — safe to cast
    let cid = command_id as i32;
    hlog_info!("[SendCommandAsync] command_id={}", cid);

    if let Some(facility) = Facility::from_command_id(cid) {
        if let Ok(mut tracker) = TRACKER.lock() {
            tracker.active = true;
            tracker.record_training(facility);
            hlog_info!(
                "Training recorded: {} (command_id={}, total={})",
                facility.name(),
                cid,
                tracker.counts[facility as usize]
            );
        }
    }

    // SAFETY: Plugin FFI interop with Hachimi vtable
    unsafe {
        if !ORIG_SEND_COMMAND_ASYNC.is_null() {
            let orig: extern "C" fn(*mut c_void, usize, usize, usize, usize, usize, usize) =
                std::mem::transmute(ORIG_SEND_COMMAND_ASYNC);
            orig(this, command_id, a2, a3, a4, a5, a6);
        }
    }
}

/// Hook for OnClickTraining(0) — no args, just logs entry into training view.
extern "C" fn hook_on_click_training_no_args(this: *mut c_void) {
    hlog_info!("[OnClickTraining] training view opened");

    // SAFETY: Plugin FFI interop with Hachimi vtable
    unsafe {
        if !ORIG_ON_CLICK_TRAINING.is_null() {
            let orig: extern "C" fn(*mut c_void) = std::mem::transmute(ORIG_ON_CLICK_TRAINING);
            orig(this);
        }
    }
}

/// Minimal MethodInfo layout matching IL2CPP v31 (64-bit).
/// Only the fields we need for diagnostics.
#[repr(C)]
struct MethodInfoCompat {
    method_pointer: usize,
    virtual_method_pointer: usize,
    invoker_method: usize,
    name: *const std::ffi::c_char,
    klass: *mut c_void,
    return_type: *const c_void,
    parameters: *mut c_void,
    _union1: usize,
    _union2: usize,
    token: u32,
    flags: u16,
    iflags: u16,
    slot: u16,
    parameters_count: u8,
}

/// Dump all method names on a class for diagnostics.
fn dump_class_methods(class_name: &str, klass: *mut c_void) {
    let vt = vt();
    let mut iter: *mut c_void = std::ptr::null_mut();
    let mut count = 0u32;
    hlog_info!("Enumerating methods on {}:", class_name);
    loop {
        // SAFETY: Plugin FFI interop with Hachimi vtable
        let method = unsafe { (vt.il2cpp_class_get_methods)(klass as _, &mut iter as *mut *mut c_void) };
        if method.is_null() {
            break;
        }
        // SAFETY: Plugin FFI interop with Hachimi vtable
        unsafe {
            let mi = &*(method as *const MethodInfoCompat);
            if !mi.name.is_null() {
                let name = std::ffi::CStr::from_ptr(mi.name);
                if let Ok(s) = name.to_str() {
                    // Filter to training/command/click/select/decide related
                    let sl = s.to_ascii_lowercase();
                    if sl.contains("train")
                        || sl.contains("command")
                        || sl.contains("click")
                        || sl.contains("select")
                        || sl.contains("decide")
                        || sl.contains("exec")
                        || sl.contains("start")
                        || sl.contains("home")
                    {
                        hlog_info!("  {}::{} (args={})", class_name, s, mi.parameters_count);
                    }
                }
            }
        }
        count += 1;
        if count > 500 {
            break;
        }
    }
    hlog_info!("  {} total methods on {}", count, class_name);
}

/// Attempt to install hooks by resolving IL2CPP methods at runtime.
///
/// This tries several known class/method combinations. The first one that
/// resolves successfully gets hooked.
///
/// Returns `true` if at least one hook was installed.
pub fn try_install_hooks() -> bool {
    if HOOKS_INSTALLED.load(Ordering::Relaxed) {
        return true;
    }

    let vt = vt();

    // List of (assembly, namespace, class, method, arg_count, hook_fn) to try.
    // These are educated guesses based on community research. Update after
    // running Il2CppDumper on your game version.
    // Candidates derived from Il2CppDumper metadata analysis of the actual game.
    // Classes/methods confirmed present in the global-metadata.dat:
    //   - SingleModeMainViewController (class)
    //   - OnClickTraining (method)
    //   - OnDecide (method)
    //   - TrainingSelectDecide (class)
    //   - TrainingView, TrainingController, TrainingMain (classes)
    //   - get_SelectedTrainingCommandId, get_TrainingCommandId (properties)
    let candidates: &[(
        &[u8],         // assembly name (null-terminated)
        &[u8],         // namespace
        &[u8],         // class name
        &[u8],         // method name
        i32,           // arg count
        *const c_void, // hook function pointer
    )] = &[
        // Candidate 1: OnClickTrainingMenu(1) — fires when player taps a specific
        // training facility button. The arg is likely the menu index or command_id.
        // Discovered via runtime method enumeration 2026-05-23.
        (
            b"umamusume.dll\0",
            b"Gallop\0",
            b"SingleModeMainViewController\0",
            b"OnClickTrainingMenu\0",
            1,
            hook_on_click_training as *const c_void,
        ),
        // Candidate 2: CommonSendCommandAsync(2) — simpler command sender,
        // likely (commandType, commandId) or similar.
        (
            b"umamusume.dll\0",
            b"Gallop\0",
            b"SingleModeMainViewController\0",
            b"CommonSendCommandAsync\0",
            2,
            hook_on_select_command as *const c_void,
        ),
        // Candidate 3: SendCommandAsync(6) — full command submission with all params.
        // We hook this to log all 6 args and identify which carries command_id.
        (
            b"umamusume.dll\0",
            b"Gallop\0",
            b"SingleModeMainViewController\0",
            b"SendCommandAsync\0",
            6,
            hook_send_command_async as *const c_void,
        ),
        // Candidate 4: OnClickTraining(0) — no-arg, opens the training view.
        // May not carry command_id but confirms training flow entry.
        (
            b"umamusume.dll\0",
            b"Gallop\0",
            b"SingleModeMainViewController\0",
            b"OnClickTraining\0",
            0,
            hook_on_click_training_no_args as *const c_void,
        ),
    ];

    let mut installed_count = 0u32;

    // --- Diagnostic: enumerate methods on key classes ---
    // SAFETY: Plugin FFI interop with Hachimi vtable
    unsafe {
        let image = (vt.il2cpp_get_assembly_image)(c"umamusume.dll".as_ptr().cast());
        if !image.is_null() {
            // Probe SingleModeMainViewController
            let klass = (vt.il2cpp_get_class)(
                image,
                c"Gallop".as_ptr().cast(),
                c"SingleModeMainViewController".as_ptr().cast(),
            );
            if !klass.is_null() {
                dump_class_methods("SingleModeMainViewController", klass as _);
            } else {
                hlog_warn!("SingleModeMainViewController class not found!");
            }

            // Probe some other training-related classes
            for probe_class in [
                &b"TrainingView\0"[..],
                b"TrainingController\0",
                b"TrainingSelectDecide\0",
                b"TrainingMain\0",
                b"TrainingMenu\0",
                b"SingleModeViewController\0",
                b"SingleModeSceneController\0",
            ] {
                let k = (vt.il2cpp_get_class)(image, c"Gallop".as_ptr().cast(), probe_class.as_ptr().cast());
                let name = std::str::from_utf8(&probe_class[..probe_class.len() - 1]).unwrap_or("?");
                if !k.is_null() {
                    dump_class_methods(name, k as _);
                } else {
                    hlog_debug!("  Class {} not found", name);
                }
            }
        }
    }

    for (asm, ns, class, method, args, hook_fn) in candidates {
        hlog_info!(
            "Trying hook: {}::{}::{} (args={})",
            std::str::from_utf8(&asm[..asm.len() - 1]).unwrap_or("?"),
            std::str::from_utf8(&class[..class.len() - 1]).unwrap_or("?"),
            std::str::from_utf8(&method[..method.len() - 1]).unwrap_or("?"),
            args,
        );

        // SAFETY: Plugin FFI interop with Hachimi vtable
        unsafe {
            let image = (vt.il2cpp_get_assembly_image)(asm.as_ptr() as _);
            if image.is_null() {
                hlog_warn!("  Assembly not found, skipping");
                continue;
            }

            let klass = (vt.il2cpp_get_class)(image, ns.as_ptr() as _, class.as_ptr() as _);
            if klass.is_null() {
                hlog_warn!("  Class not found, skipping");
                continue;
            }

            let addr = (vt.il2cpp_get_method_addr)(klass, method.as_ptr() as _, *args);
            if addr.is_null() {
                hlog_warn!("  Method not found, skipping");
                continue;
            }

            hlog_info!("  Found at {:?}, installing hook...", addr);

            let hachimi = (vt.hachimi_instance)();
            let interceptor = (vt.hachimi_get_interceptor)(hachimi);
            let trampoline = (vt.interceptor_hook)(interceptor, addr, *hook_fn as *mut c_void);

            if !trampoline.is_null() {
                // Route trampoline to the correct storage based on the hook fn
                let hook_ptr = *hook_fn as usize;
                if hook_ptr == hook_on_click_training as usize {
                    ORIG_ON_CLICK_TRAINING_MENU = trampoline;
                } else if hook_ptr == hook_on_select_command as usize {
                    ORIG_COMMON_SEND_COMMAND = trampoline;
                } else if hook_ptr == hook_send_command_async as usize {
                    ORIG_SEND_COMMAND_ASYNC = trampoline;
                } else if hook_ptr == hook_on_click_training_no_args as usize {
                    ORIG_ON_CLICK_TRAINING = trampoline;
                }
                installed_count += 1;
                HOOKS_INSTALLED.store(true, Ordering::Relaxed);
                hlog_info!("  ✓ Hook installed successfully!");
                // Continue to install ALL hooks, not just the first
            } else {
                hlog_error!("  ✗ Hook installation failed");
            }
        }
    }

    if installed_count == 0 {
        hlog_warn!(
            "No hook candidates found. The plugin will still load but won't \
             track automatically."
        );
    } else {
        hlog_info!("{} hooks installed for diagnostic capture", installed_count);
    }

    installed_count > 0
}
