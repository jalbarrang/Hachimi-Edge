use crate::il2cpp::types::*;

pub static mut UNITYACTION_CLASS: *mut Il2CppClass = std::ptr::null_mut();

pub fn init(UnityEngine_CoreModule: *const Il2CppImage) {
    get_class_or_return!(UnityEngine_CoreModule, "UnityEngine.Events", UnityAction);

    // SAFETY: FFI / raw pointer operation required by IL2CPP interop
    unsafe {
        UNITYACTION_CLASS = UnityAction;
    }
}
