use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use crate::{
    windows::free_camera::{self, CameraScene},
    il2cpp::{
        hook::UnityEngine_CoreModule::{Camera, Transform},
        symbols::{get_field_from_name, get_method_addr, get_method_overload_addr, set_field_value},
        types::*,
    },
};

use super::RaceViewBase;

const FULL_RECT: Rect_t = Rect_t { x: 0.0, y: 0.0, width: 1.0, height: 1.0 };
const HIDDEN_RECT: Rect_t = Rect_t { x: 0.0, y: 0.0, width: 0.0, height: 0.0 };

// RaceCameraManager exposes the race cameras as properties. `CurrentCamera` is the one the
// game renders this frame; mid-race it can be `CurrentMainMultiCamera` while `MainCamera`
// shows the wide caster view.
static mut GET_CURRENT_CAMERA_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_CurrentCamera, GET_CURRENT_CAMERA_ADDR, *mut Il2CppObject, this: *mut Il2CppObject);

static mut GET_MAIN_CAMERA_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_MainCamera, GET_MAIN_CAMERA_ADDR, *mut Il2CppObject, this: *mut Il2CppObject);

static mut GET_CURRENT_MAIN_MULTI_CAMERA_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_CurrentMainMultiCamera, GET_CURRENT_MAIN_MULTI_CAMERA_ADDR, *mut Il2CppObject, this: *mut Il2CppObject);

static mut GET_EVENT_CAMERA_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_EventCamera, GET_EVENT_CAMERA_ADDR, *mut Il2CppObject, this: *mut Il2CppObject);

/// `RaceCameraEventBase._unPlayable`. Forcing it true is the game's own "do not play this
/// event camera" switch, which also keeps the cut-in FSM from taking the camera.
static mut EVENT_CAMERA_UNPLAYABLE_FIELD: *mut FieldInfo = std::ptr::null_mut();

static CUTINS_SUPPRESSED: AtomicBool = AtomicBool::new(false);
static LAST_STATE_LOG_MS: AtomicU64 = AtomicU64::new(0);
static LAST_MULTI_LOG_MS: AtomicU64 = AtomicU64::new(0);
static LAST_MODE_LOG_MS: AtomicU64 = AtomicU64::new(0);
static LAST_EVENT_LOG_MS: AtomicU64 = AtomicU64::new(0);

type NoArgsFn = extern "C" fn(this: *mut Il2CppObject);
extern "C" fn RaceCameraManager_AlterLateUpdate(this: *mut Il2CppObject) {
    // Only drive the race camera while the race is actually running. Gate-in, the intro
    // and the result screen keep the game's cameras, which is also what stops the free
    // camera FOV leaking onto those screens.
    if RaceViewBase::is_race_phase() {
        free_camera::set_race_active();
    }
    else {
        free_camera::end_scene(CameraScene::Race);
    }
    free_camera::tick();

    let active = free_camera::is_scene_enabled(CameraScene::Race);
    Transform::set_update_race_camera(active);
    get_orig_fn!(RaceCameraManager_AlterLateUpdate, NoArgsFn)(this);
    Transform::set_update_race_camera(false);

    if active {
        // The game may have switched to its split-screen "multi camera" this frame. It
        // drives that by resizing the two race cameras, so put the camera we own back to
        // full screen and hide the other one instead of skipping the camera update (which
        // left a black, texture-less panel on screen).
        enforce_race_camera_view(this);
    }

    sync_cutin_suppression(this, free_camera::should_suppress_race_cutins());
    log_race_camera_state(this, active);
}

/// Makes the active race camera fill the viewport and hides the other on-screen race
/// camera, so the game's mid-race split-screen collapses onto the free camera.
fn enforce_race_camera_view(this: *mut Il2CppObject) {
    let current = get_CurrentCamera(this);
    let main = get_MainCamera(this);
    let multi = get_CurrentMainMultiCamera(this);

    let owned = if !current.is_null() { current } else { main };
    if owned.is_null() {
        return;
    }

    Camera::set_rect(owned, FULL_RECT);

    for other in [main, multi] {
        if other.is_null() || other == owned {
            continue;
        }
        // Cameras with a target texture feed an off-screen display (racetrack monitor,
        // cut-in texture); resizing those blanks the feed without removing the split.
        if Camera::renders_off_screen(other) {
            continue;
        }
        Camera::set_rect(other, HIDDEN_RECT);
    }
}

/// Flips the game's own "event camera unplayable" flag so the race does not hand the view
/// to a cut-in while the free camera owns it. Only written when the desired state changes,
/// and cleared again once the free camera releases the race camera.
fn sync_cutin_suppression(this: *mut Il2CppObject, suppress: bool) {
    if CUTINS_SUPPRESSED.load(Ordering::Relaxed) == suppress {
        return;
    }
    let unplayable_field = unsafe { EVENT_CAMERA_UNPLAYABLE_FIELD };
    if unplayable_field.is_null() {
        CUTINS_SUPPRESSED.store(suppress, Ordering::Relaxed);
        return;
    }

    let event_camera = get_EventCamera(this);
    if event_camera.is_null() {
        return;
    }

    set_field_value(event_camera, unplayable_field, &suppress);
    CUTINS_SUPPRESSED.store(suppress, Ordering::Relaxed);
    info!("RaceCameraManager: EventCamera._unPlayable = {suppress}");
}

fn camera_rect(camera: *mut Il2CppObject) -> Option<Rect_t> {
    Camera::get_rect(camera)
}

fn log_race_camera_state(this: *mut Il2CppObject, active: bool) {
    if !free_camera::throttle_1s(&LAST_STATE_LOG_MS) {
        return;
    }

    let current = get_CurrentCamera(this);
    let main = get_MainCamera(this);
    let multi = get_CurrentMainMultiCamera(this);
    info!(
        "RaceCamera: active={active} race_phase={} suppress={} current={:p}{:?} main={:p}{:?} multi={:p}{:?}",
        RaceViewBase::is_race_phase(),
        free_camera::should_suppress_race_cutins(),
        current, camera_rect(current),
        main, camera_rect(main),
        multi, camera_rect(multi),
    );
}

type RaceChangeCameraModeFn = extern "C" fn(this: *mut Il2CppObject, mode: i32, is_skip: bool);
extern "C" fn RaceCameraManager_ChangeCameraMode(this: *mut Il2CppObject, mode: i32, is_skip: bool) {
    let suppress = free_camera::should_suppress_race_cutins();
    if free_camera::throttle_1s(&LAST_MODE_LOG_MS) {
        info!("RaceCamera: ChangeCameraMode mode={mode} skip={is_skip} suppress={suppress}");
    }
    if suppress {
        return;
    }
    get_orig_fn!(RaceCameraManager_ChangeCameraMode, RaceChangeCameraModeFn)(this, mode, is_skip);
}

// public void UpdateMultiCamera() { }
//
// Drives the mid-race split-screen (caster) camera. It still runs so the game's camera
// state stays consistent; the split itself is removed by `enforce_race_camera_view`,
// which restores the free camera's full-screen viewport afterwards.
extern "C" fn RaceCameraManager_UpdateMultiCamera(this: *mut Il2CppObject) {
    if free_camera::throttle_1s(&LAST_MULTI_LOG_MS) {
        info!(
            "RaceCamera: UpdateMultiCamera suppress={} active={}",
            free_camera::should_suppress_race_cutins(),
            free_camera::is_scene_enabled(CameraScene::Race),
        );
    }
    get_orig_fn!(RaceCameraManager_UpdateMultiCamera, NoArgsFn)(this);
}

// public bool PlayEventCamera(int targetHorseIndex, int rivalHorseIndex, int cameraId, bool isForceInPlaying = False, bool isForceUnPlayableArea = False) { }
//
// Note: the second parameter is an `int`, not an `int[]` -- the old type list used
// `SZARRAY`, so `get_method_overload_addr` returned NULL and this hook never installed.
type RacePlayEventCameraFn = extern "C" fn(
    this: *mut Il2CppObject,
    targetHorseIndex: i32,
    rivalHorseIndex: i32,
    cameraId: i32,
    isForceInPlaying: bool,
    isForceUnPlayableArea: bool,
) -> bool;
extern "C" fn RaceCameraManager_PlayEventCamera(
    this: *mut Il2CppObject,
    targetHorseIndex: i32,
    rivalHorseIndex: i32,
    cameraId: i32,
    isForceInPlaying: bool,
    isForceUnPlayableArea: bool,
) -> bool {
    let suppress = free_camera::should_suppress_race_cutins();
    if free_camera::throttle_1s(&LAST_EVENT_LOG_MS) || suppress {
        info!(
            "RaceCamera: PlayEventCamera target={targetHorseIndex} rival={rivalHorseIndex} camera={cameraId} force={isForceInPlaying} unplayable={isForceUnPlayableArea} suppress={suppress}"
        );
    }
    if suppress {
        return false;
    }
    get_orig_fn!(RaceCameraManager_PlayEventCamera, RacePlayEventCameraFn)(this, targetHorseIndex, rivalHorseIndex, cameraId, isForceInPlaying, isForceUnPlayableArea)
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, "Gallop", RaceCameraManager);

    unsafe {
        GET_CURRENT_CAMERA_ADDR = get_method_addr(RaceCameraManager, c"get_CurrentCamera", 0);
        GET_MAIN_CAMERA_ADDR = get_method_addr(RaceCameraManager, c"get_MainCamera", 0);
        GET_CURRENT_MAIN_MULTI_CAMERA_ADDR =
            get_method_addr(RaceCameraManager, c"get_CurrentMainMultiCamera", 0);
        GET_EVENT_CAMERA_ADDR = get_method_addr(RaceCameraManager, c"get_EventCamera", 0);
    }

    if let Ok(event_camera_class) = crate::il2cpp::symbols::get_class(
        umamusume, c"Gallop", c"RaceCameraEventBase",
    ) {
        unsafe {
            EVENT_CAMERA_UNPLAYABLE_FIELD = get_field_from_name(event_camera_class, c"_unPlayable");
        }
    }

    let RaceCameraManager_AlterLateUpdate_addr = get_method_addr(RaceCameraManager, c"AlterLateUpdate", 0);
    new_hook!(RaceCameraManager_AlterLateUpdate_addr, RaceCameraManager_AlterLateUpdate);

    let RaceCameraManager_ChangeCameraMode_addr = get_method_addr(RaceCameraManager, c"ChangeCameraMode", 2);
    new_hook!(RaceCameraManager_ChangeCameraMode_addr, RaceCameraManager_ChangeCameraMode);

    let RaceCameraManager_UpdateMultiCamera_addr = get_method_addr(RaceCameraManager, c"UpdateMultiCamera", 0);
    new_hook!(RaceCameraManager_UpdateMultiCamera_addr, RaceCameraManager_UpdateMultiCamera);

    let RaceCameraManager_PlayEventCamera_addr = get_method_overload_addr(
        RaceCameraManager,
        "PlayEventCamera",
        &[
            Il2CppTypeEnum_IL2CPP_TYPE_I4, // int targetHorseIndex
            Il2CppTypeEnum_IL2CPP_TYPE_I4, // int rivalHorseIndex
            Il2CppTypeEnum_IL2CPP_TYPE_I4, // int cameraId
            Il2CppTypeEnum_IL2CPP_TYPE_BOOLEAN, // bool isForceInPlaying
            Il2CppTypeEnum_IL2CPP_TYPE_BOOLEAN // bool isForceUnPlayableArea
        ]
    );
    if RaceCameraManager_PlayEventCamera_addr == 0 {
        warn!("RaceCameraManager.PlayEventCamera not found; race cut-ins may still play");
    }
    else {
        new_hook!(RaceCameraManager_PlayEventCamera_addr, RaceCameraManager_PlayEventCamera);
    }
}
