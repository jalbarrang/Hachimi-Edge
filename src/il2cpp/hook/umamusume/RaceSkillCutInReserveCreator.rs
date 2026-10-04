use std::sync::atomic::AtomicU64;

use crate::{
    windows::free_camera,
    il2cpp::{
        symbols::get_method_addr,
        types::*,
    },
};

// The race asks `FindReserve` before entering the cut-in state, so answering `false` is
// equivalent to having an empty reserve list. That covers reserves built at race load,
// unlike `AddCutInInfo` which never fires in practice. While the free camera owns the
// race camera, cut-ins would steal it, so they are suppressed here.
type FindReserveFn = extern "C" fn(this: *mut Il2CppObject, horse_index: i32) -> bool;

static LAST_LOG_MS: AtomicU64 = AtomicU64::new(0);

extern "C" fn FindReserve(this: *mut Il2CppObject, horse_index: i32) -> bool {
    let suppress = free_camera::should_suppress_race_cutins();
    if suppress && free_camera::throttle_1s(&LAST_LOG_MS) {
        info!("RaceCamera: suppressed FindReserve (horse_index={horse_index})");
    }
    if suppress {
        return false;
    }
    get_orig_fn!(FindReserve, FindReserveFn)(this, horse_index)
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, "Gallop", RaceSkillCutInReserveCreator);

    let addr = get_method_addr(RaceSkillCutInReserveCreator, c"FindReserve", 1);
    if addr == 0 {
        warn!("RaceSkillCutInReserveCreator.FindReserve not found; race cut-ins may still play");
        return;
    }
    new_hook!(addr, FindReserve);
}
