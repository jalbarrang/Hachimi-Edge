use crate::il2cpp::{api::il2cpp_resolve_icall, types::*};

#[cfg(target_os = "windows")]
use crate::il2cpp::symbols::get_method_addr;
#[cfg(target_os = "windows")]
use crate::windows::free_camera::{self, CameraScene};

// The normalised viewport of a camera. The race free camera needs these to keep itself
// full-screen when the game splits the view between two race cameras mid-race.
#[cfg(target_os = "windows")]
static mut GET_RECT_ADDR: usize = 0;
#[cfg(target_os = "windows")]
static mut SET_RECT_ADDR: usize = 0;
#[cfg(target_os = "windows")]
static mut GET_TARGET_TEXTURE_ADDR: usize = 0;

#[cfg(target_os = "windows")]
type CameraRectFn = extern "C" fn(this: *mut Il2CppObject, value: *mut Rect_t);

/// Writes `Camera.rect`. A zero-sized rect makes the camera render nothing.
#[cfg(target_os = "windows")]
pub fn set_rect(camera: *mut Il2CppObject, rect: Rect_t) {
    unsafe {
        if camera.is_null() || SET_RECT_ADDR == 0 {
            return;
        }
        let set: CameraRectFn = std::mem::transmute(SET_RECT_ADDR);
        let mut rect = rect;
        set(camera, &mut rect);
    }
}

#[cfg(target_os = "windows")]
pub fn get_rect(camera: *mut Il2CppObject) -> Option<Rect_t> {
    unsafe {
        if camera.is_null() || GET_RECT_ADDR == 0 {
            return None;
        }
        let get: CameraRectFn = std::mem::transmute(GET_RECT_ADDR);
        let mut rect = Rect_t { x: 0.0, y: 0.0, width: 0.0, height: 0.0 };
        get(camera, &mut rect);
        Some(rect)
    }
}

/// Whether the camera renders into a `RenderTexture` instead of the display. Those are
/// off-screen feeds (the racetrack monitor, the cut-in texture) and must not be resized.
#[cfg(target_os = "windows")]
pub fn renders_off_screen(camera: *mut Il2CppObject) -> bool {
    unsafe {
        if camera.is_null() || GET_TARGET_TEXTURE_ADDR == 0 {
            return false;
        }
        let get: extern "C" fn(this: *mut Il2CppObject) -> *mut Il2CppObject =
            std::mem::transmute(GET_TARGET_TEXTURE_ADDR);
        !get(camera).is_null()
    }
}
#[cfg(target_os = "windows")]
type CameraGetFloatFn = extern "C" fn(this: *mut Il2CppObject) -> f32;
#[cfg(target_os = "windows")]
type CameraSetFloatFn = extern "C" fn(this: *mut Il2CppObject, value: f32);

static mut SET_FIELD_OF_VIEW_ADDR: usize = 0;
impl_addr_wrapper_fn!(set_fieldOfView, SET_FIELD_OF_VIEW_ADDR, (), this: *mut Il2CppObject, value: f32);

#[cfg(target_os = "windows")]
fn should_override_near_clip() -> bool {
    free_camera::is_scene_enabled(CameraScene::Live) ||
        free_camera::is_scene_enabled(CameraScene::Race)
}

#[cfg(target_os = "windows")]
extern "C" fn Camera_get_fieldOfView(this: *mut Il2CppObject) -> f32 {
    let scene = free_camera::scene();
    if let Some(fov) = free_camera::fov_for_scene(scene) {
        return fov;
    }
    get_orig_fn!(Camera_get_fieldOfView, CameraGetFloatFn)(this)
}

#[cfg(target_os = "windows")]
extern "C" fn Camera_set_nearClipPlane(this: *mut Il2CppObject, mut value: f32) {
    if should_override_near_clip() {
        value = 0.001;
    }
    get_orig_fn!(Camera_set_nearClipPlane, CameraSetFloatFn)(this, value);
}

#[cfg(target_os = "windows")]
extern "C" fn Camera_get_nearClipPlane(this: *mut Il2CppObject) -> f32 {
    if should_override_near_clip() {
        return 0.001;
    }
    get_orig_fn!(Camera_get_nearClipPlane, CameraGetFloatFn)(this)
}

#[cfg(target_os = "windows")]
extern "C" fn Camera_set_farClipPlane(this: *mut Il2CppObject, mut value: f32) {
    if free_camera::is_scene_enabled(CameraScene::Live) || free_camera::is_scene_enabled(CameraScene::Race) {
        value = 2500.0;
    }
    get_orig_fn!(Camera_set_farClipPlane, CameraSetFloatFn)(this, value);
}

#[cfg(target_os = "windows")]
extern "C" fn Camera_get_farClipPlane(this: *mut Il2CppObject) -> f32 {
    if free_camera::is_scene_enabled(CameraScene::Live) || free_camera::is_scene_enabled(CameraScene::Race) {
        return 2500.0;
    }
    get_orig_fn!(Camera_get_farClipPlane, CameraGetFloatFn)(this)
}

pub fn init(_UnityEngine_CoreModule: *const Il2CppImage) {
    unsafe {
        SET_FIELD_OF_VIEW_ADDR = il2cpp_resolve_icall(c"UnityEngine.Camera::set_fieldOfView(System.Single)".as_ptr());
    }

    #[cfg(target_os = "windows")]
    {
        unsafe {
            GET_RECT_ADDR = il2cpp_resolve_icall(c"UnityEngine.Camera::get_rect_Injected(UnityEngine.Rect&)".as_ptr());
            SET_RECT_ADDR = il2cpp_resolve_icall(c"UnityEngine.Camera::set_rect_Injected(UnityEngine.Rect&)".as_ptr());
            GET_TARGET_TEXTURE_ADDR = il2cpp_resolve_icall(c"UnityEngine.Camera::get_targetTexture()".as_ptr());
        }

        if unsafe { GET_TARGET_TEXTURE_ADDR } == 0 {
            // Some builds only expose it as a normal method on the class.
            if let Ok(camera_class) = crate::il2cpp::symbols::get_class(
                _UnityEngine_CoreModule, c"UnityEngine", c"Camera",
            ) {
                unsafe {
                    GET_TARGET_TEXTURE_ADDR =
                        get_method_addr(camera_class, c"get_targetTexture", 0);
                }
            }
        }

        let get_fieldOfView_addr = il2cpp_resolve_icall(c"UnityEngine.Camera::get_fieldOfView()".as_ptr());
        new_hook!(get_fieldOfView_addr, Camera_get_fieldOfView);

        let set_nearClipPlane_addr = il2cpp_resolve_icall(c"UnityEngine.Camera::set_nearClipPlane(System.Single)".as_ptr());
        new_hook!(set_nearClipPlane_addr, Camera_set_nearClipPlane);

        let get_nearClipPlane_addr = il2cpp_resolve_icall(c"UnityEngine.Camera::get_nearClipPlane()".as_ptr());
        new_hook!(get_nearClipPlane_addr, Camera_get_nearClipPlane);

        let set_farClipPlane_addr = il2cpp_resolve_icall(c"UnityEngine.Camera::set_farClipPlane(System.Single)".as_ptr());
        new_hook!(set_farClipPlane_addr, Camera_set_farClipPlane);

        let get_farClipPlane_addr = il2cpp_resolve_icall(c"UnityEngine.Camera::get_farClipPlane()".as_ptr());
        new_hook!(get_farClipPlane_addr, Camera_get_farClipPlane);
    }
}
