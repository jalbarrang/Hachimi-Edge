use crate::{
    windows::free_camera::{self, CameraScene},
    il2cpp::{
        symbols::{get_method_addr, get_method_addr_any},
        types::*,
    },
};

static mut GET_PREFAB_ATTACH_TRANSFORM_ADDR: usize = 0;
impl_addr_wrapper_fn!(
    GetPrefabAttachTransform,
    GET_PREFAB_ATTACH_TRANSFORM_ADDR,
    *mut Il2CppObject,
    this: *mut Il2CppObject,
    part: i32,
    name: *mut Il2CppString
);

/// Whether the attach-transform lookup resolved. The wrapper above has no null check,
/// so callers must ask before using it.
pub fn attach_transform_ready() -> bool {
    unsafe { GET_PREFAB_ATTACH_TRANSFORM_ADDR != 0 }
}

type RaceUpdateCameraDistanceBlendRateFn = extern "C" fn(
    this: *mut Il2CppObject,
    p1: *mut Il2CppObject,
    p2: *mut Il2CppObject,
    p3: *mut Il2CppObject,
);
extern "C" fn RaceModelController_UpdateCameraDistanceBlendRate(
    this: *mut Il2CppObject,
    p1: *mut Il2CppObject,
    p2: *mut Il2CppObject,
    p3: *mut Il2CppObject,
) {
    if free_camera::is_scene_enabled(CameraScene::Race) {
        return;
    }
    get_orig_fn!(RaceModelController_UpdateCameraDistanceBlendRate, RaceUpdateCameraDistanceBlendRateFn)(
        this,
        p1,
        p2,
        p3,
    );
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, "Gallop", RaceModelController);

    let RaceModelController_UpdateCameraDistanceBlendRate_addr =
        get_method_addr(RaceModelController, c"UpdateCameraDistanceBlendRate", 3);    new_hook!(
        RaceModelController_UpdateCameraDistanceBlendRate_addr,
        RaceModelController_UpdateCameraDistanceBlendRate
    );

    unsafe {
        // The attach-transform method's signature differs by build: one
        // `CharaAttachTransform` argument on Global, a `(part, name)` pair on JP. Try
        // both shapes rather than branching on region, so this keeps working if either
        // build changes again. The caller passes both arguments, so the extra one is
        // ignored by whichever shape resolves.
        GET_PREFAB_ATTACH_TRANSFORM_ADDR =
            get_method_addr_any(RaceModelController, c"GetPrefabAttachTransform", &[1, 2]);
    }
}
