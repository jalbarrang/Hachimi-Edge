use crate::il2cpp::{
    api::{il2cpp_class_get_type, il2cpp_type_get_object},
    types::*,
};

static mut TYPE_OBJECT: *mut Il2CppObject = 0 as _;
pub fn type_object() -> *mut Il2CppObject {
    // SAFETY: FFI / raw pointer operation required by IL2CPP interop
    unsafe { TYPE_OBJECT }
}

pub fn init(UnityEngine_TextRenderingModule: *const Il2CppImage) {
    get_class_or_return!(UnityEngine_TextRenderingModule, UnityEngine, Font);

    // SAFETY: FFI / raw pointer operation required by IL2CPP interop
    unsafe {
        TYPE_OBJECT = il2cpp_type_get_object(il2cpp_class_get_type(Font));
    }
}
