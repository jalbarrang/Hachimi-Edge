use std::sync::atomic::AtomicU64;

use crate::{
    windows::free_camera,
    il2cpp::{
        symbols::get_method_addr,
        types::*,
    },
};

// The race view plays its own cut-ins from these routines, separate from the skill
// reserve (`RaceSkillCutInReserveCreator`) and the event camera (`PlayEventCamera`).
// While the race free camera owns the camera, skip them so the view does not jump away.
type VoidFn = extern "C" fn(this: *mut Il2CppObject);

static LAST_LOG_MS: AtomicU64 = AtomicU64::new(0);

fn suppressed(name: &str) {
    if free_camera::throttle_1s(&LAST_LOG_MS) {
        info!("RaceCamera: suppressed RaceViewReplay.{name}");
    }
}

extern "C" fn FirstCutin(this: *mut Il2CppObject) {
    if free_camera::should_suppress_race_cutins() {
        suppressed("FirstCutin");
        return;
    }
    get_orig_fn!(FirstCutin, VoidFn)(this);
}

extern "C" fn SecondCutin(this: *mut Il2CppObject) {
    if free_camera::should_suppress_race_cutins() {
        suppressed("SecondCutin");
        return;
    }
    get_orig_fn!(SecondCutin, VoidFn)(this);
}

extern "C" fn PlayerCutin(this: *mut Il2CppObject) {
    if free_camera::should_suppress_race_cutins() {
        suppressed("PlayerCutin");
        return;
    }
    get_orig_fn!(PlayerCutin, VoidFn)(this);
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, "Gallop", RaceViewReplay);

    let first = get_method_addr(RaceViewReplay, c"FirstCutin", 0);
    new_hook!(first, FirstCutin);

    let second = get_method_addr(RaceViewReplay, c"SecondCutin", 0);
    new_hook!(second, SecondCutin);

    let player = get_method_addr(RaceViewReplay, c"PlayerCutin", 0);
    new_hook!(player, PlayerCutin);
}
