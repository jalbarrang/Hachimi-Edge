//! Single Mode (career) start/end detection, driven by view changes.
//!
//! There is no verified host hook that fires exactly at career boundaries, but
//! every boundary coincides with a view transition: entering Single Mode flips
//! `IsPlaying` true, the results/home transition flips it false. So instead of
//! polling every frame we re-check the runtime-confirmed `WorkDataManager →
//! get_SingleMode() → get_IsPlaying()` chain (see
//! docs/reverse-engineering/single-mode-architecture.md) once per `VIEW_CHANGE`
//! and translate `IsPlaying` transitions into [`event::CAREER_START`] /
//! [`event::CAREER_END`].
//!
//! This needs no knowledge of the specific Single Mode `ViewId`: any view change
//! re-syncs the flag, and careers contain many intra-run transitions, so a missed
//! edge self-corrects on the next screen change. Resolution is lazy and cached; if
//! the career classes aren't loaded yet the check is a cheap no-op and retries on
//! the next view change.

use std::sync::Mutex;

use crate::il2cpp::{
    symbols::{get_assembly_image, get_class, get_method, SingletonLike},
    types::{Il2CppObject, MethodInfo},
};

use super::events;

/// Cached IL2CPP handles for the career-state chain.
struct Resolved {
    wdm_class: *mut crate::il2cpp::types::Il2CppClass,
    get_single_mode: *const MethodInfo,
    get_is_playing: *const MethodInfo,
}

// SAFETY: IL2CPP class/method pointers are stable for the process lifetime.
unsafe impl Send for Resolved {}

struct State {
    resolved: Option<Resolved>,
    last_playing: bool,
}

static STATE: Mutex<State> = Mutex::new(State {
    resolved: None,
    last_playing: false,
});

fn resolve() -> Option<Resolved> {
    let image = get_assembly_image(c"umamusume.dll").ok()?;
    let wdm_class = get_class(image, c"Gallop", c"WorkDataManager").ok()?;
    let wsmd_class = get_class(image, c"Gallop", c"WorkSingleModeData").ok()?;
    let get_single_mode = get_method(wdm_class, c"get_SingleMode", 0).ok()?;
    let get_is_playing = get_method(wsmd_class, c"get_IsPlaying", 0).ok()?;
    Some(Resolved {
        wdm_class,
        get_single_mode,
        get_is_playing,
    })
}

/// Invoke a 0-arg IL2CPP instance method returning an object pointer.
unsafe fn call_obj(this: *mut Il2CppObject, mi: *const MethodInfo) -> *mut Il2CppObject {
    // SAFETY: `mi` is a resolved MethodInfo; methodPointer is the callable entry.
    let fp: extern "C" fn(*mut Il2CppObject, *const MethodInfo) -> *mut Il2CppObject =
        unsafe { std::mem::transmute((*mi).methodPointer) };
    fp(this, mi)
}

/// Invoke a 0-arg IL2CPP instance method returning `bool` (IL2CPP `u8`).
unsafe fn call_bool(this: *mut Il2CppObject, mi: *const MethodInfo) -> bool {
    // SAFETY: `mi` is a resolved MethodInfo; methodPointer is the callable entry.
    let fp: extern "C" fn(*mut Il2CppObject, *const MethodInfo) -> u8 =
        unsafe { std::mem::transmute((*mi).methodPointer) };
    fp(this, mi) != 0
}

/// Read the current `IsPlaying` flag via the resolved chain, or `false` if the
/// singleton/career data isn't available yet.
fn is_playing(r: &Resolved) -> bool {
    let Some(singleton) = SingletonLike::new(r.wdm_class) else {
        return false;
    };
    let instance = singleton.instance();
    if instance.is_null() {
        return false;
    }
    // SAFETY: resolved chain + live singleton; getters are 0-arg and pointer-safe.
    unsafe {
        let wsmd = call_obj(instance, r.get_single_mode);
        if wsmd.is_null() {
            return false;
        }
        call_bool(wsmd, r.get_is_playing)
    }
}

/// Called on every `VIEW_CHANGE`. Re-checks `IsPlaying` and emits career start/end
/// events on transitions. Lazily resolved, panic-free, no per-frame cost.
pub fn on_view_change() {
    let mut state = STATE.lock().expect("lock poisoned");

    if state.resolved.is_none() {
        state.resolved = resolve();
    }
    let Some(resolved) = state.resolved.as_ref() else {
        return;
    };

    let playing = is_playing(resolved);
    if playing == state.last_playing {
        return;
    }
    state.last_playing = playing;
    // Release the lock before dispatching so callbacks can't deadlock us.
    drop(state);

    if playing {
        events::dispatch_career_start();
    } else {
        events::dispatch_career_end();
    }
}
