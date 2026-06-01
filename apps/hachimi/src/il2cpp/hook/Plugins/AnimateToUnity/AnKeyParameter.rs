use crate::il2cpp::{
    symbols::{get_field_from_name, get_field_object_value},
    types::*,
};

static mut _KEY_LIST_FIELD: *mut FieldInfo = 0 as _;
pub fn get__keyList(this: *mut Il2CppObject) -> *mut Il2CppObject {
    // SAFETY: FFI / raw pointer operation required by IL2CPP interop
    get_field_object_value(this, unsafe { _KEY_LIST_FIELD })
}

pub fn init(Plugins: *const Il2CppImage) {
    get_class_or_return!(Plugins, AnimateToUnity, AnKeyParameter);

    // SAFETY: FFI / raw pointer operation required by IL2CPP interop
    unsafe {
        _KEY_LIST_FIELD = get_field_from_name(AnKeyParameter, c"_keyList");
    }
}
