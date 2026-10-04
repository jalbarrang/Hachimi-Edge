use crate::{
    windows::free_camera::{self, CameraScene},
    il2cpp::{
        ext::StringExt,
        hook::UnityEngine_CoreModule::Transform,
        symbols::{find_nested_class, get_field_from_name, get_field_value, get_method_addr, get_static_field_value},
        types::*,
    },
};
use once_cell::sync::Lazy;
use std::sync::atomic::{AtomicI32, Ordering};

use super::{RaceModelController};

static RACE_DISABLED_HEADS: free_camera::DisabledHeadStore = Lazy::new(free_camera::new_disabled_head_store);

static mut CUR_PHASE_FIELD: *mut FieldInfo = std::ptr::null_mut();
/// `RaceViewBase.Phase.Race` (resolved from the enum, not hardcoded).
static RACE_PHASE_VALUE: AtomicI32 = AtomicI32::new(i32::MIN);
static CURRENT_PHASE: AtomicI32 = AtomicI32::new(i32::MIN);

/// Whether the race is actually running (`Phase.Race`), as opposed to gate-in,
/// intro, or the result screen. The free camera engages only then.
pub fn is_race_phase() -> bool {
    let race = RACE_PHASE_VALUE.load(Ordering::Acquire);
    race != i32::MIN && CURRENT_PHASE.load(Ordering::Acquire) == race
}

static mut GET_MODEL_CONTROLLER_ADDR: usize = 0;
impl_addr_wrapper_fn!(GetModelController, GET_MODEL_CONTROLLER_ADDR, *mut Il2CppObject, this: *mut Il2CppObject, index: i32);

pub fn restore_race_disabled_heads(current_index: i32, force_all: bool) {
    free_camera::restore_disabled_heads(&RACE_DISABLED_HEADS, current_index, force_all);
}

type LateUpdateViewFn = extern "C" fn(this: *mut Il2CppObject);
extern "C" fn LateUpdateView(this: *mut Il2CppObject) {
    // Track the view phase so `RaceCameraManager` can gate the free camera on it.
    unsafe {
        if !CUR_PHASE_FIELD.is_null() {
            let phase: i32 = get_field_value(this, CUR_PHASE_FIELD);
            CURRENT_PHASE.store(phase, Ordering::Release);
            if !is_race_phase() {
                // Also clear the scene here: `RaceCameraManager::AlterLateUpdate` stops
                // running on the result screen, and leaving the scene set would keep the
                // free camera's FOV override active there.
                free_camera::end_scene(CameraScene::Race);
            }
        }
    }
    let first_person = free_camera::is_race_first_person();
    let head_selfie = free_camera::is_race_head_selfie();
    if first_person || head_selfie {
        let index = free_camera::race_model_index();
        let model_controller = GetModelController(this, index);
        if !model_controller.is_null() && RaceModelController::attach_transform_ready() {
            let empty = "".to_il2cpp_string();
            let eye_left = RaceModelController::GetPrefabAttachTransform(model_controller, 0x7, empty);
            let eye_right = RaceModelController::GetPrefabAttachTransform(model_controller, 0x8, empty);
            if !eye_left.is_null() && !eye_right.is_null() {
                let mut pos_left = Vector3_t::default();
                let mut pos_right = Vector3_t::default();
                let mut rot_left = Quaternion_t::default();
                let mut rot_right = Quaternion_t::default();

                Transform::get_position_Injected(eye_left, &mut pos_left);
                Transform::get_position_Injected(eye_right, &mut pos_right);
                Transform::get_rotation_Injected(eye_left, &mut rot_left);
                Transform::get_rotation_Injected(eye_right, &mut rot_right);

                let pos = Vector3_t {
                    x: (pos_left.x + pos_right.x) * 0.5,
                    y: (pos_left.y + pos_right.y) * 0.5,
                    z: (pos_left.z + pos_right.z) * 0.5,
                };
                let rot = free_camera::slerp_quaternion(rot_left, rot_right, 0.5);
                if first_person {
                    free_camera::update_first_person(CameraScene::Race, pos, rot, None);
                    free_camera::hide_head_parts(&RACE_DISABLED_HEADS, model_controller, index);
                    restore_race_disabled_heads(index, false);
                }
                else {
                    free_camera::update_race_head_follow(pos, rot);
                    restore_race_disabled_heads(0, true);
                }
            }
        }
    }
    else {
        restore_race_disabled_heads(0, true);
    }

    get_orig_fn!(LateUpdateView, LateUpdateViewFn)(this);
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, "Gallop", RaceViewBase);

    let LateUpdateView_addr = get_method_addr(RaceViewBase, c"LateUpdateView", 0);
    new_hook!(LateUpdateView_addr, LateUpdateView);

    unsafe {
        GET_MODEL_CONTROLLER_ADDR = get_method_addr(RaceViewBase, c"GetModelController", 1);
        CUR_PHASE_FIELD = get_field_from_name(RaceViewBase, c"_curPhase");
    }

    // Resolve `Phase.Race` from the nested enum rather than assuming its ordinal.
    if let Ok(phase_class) = find_nested_class(RaceViewBase, c"Phase") {
        let race_field = get_field_from_name(phase_class, c"Race");
        if !race_field.is_null() {
            let value: i32 = get_static_field_value(race_field);
            RACE_PHASE_VALUE.store(value, Ordering::Release);
        }
        else {
            warn!("RaceViewBase.Phase.Race not found; free camera phase gating disabled");
        }
    }
    else {
        warn!("RaceViewBase.Phase not found; free camera phase gating disabled");
    }
}
