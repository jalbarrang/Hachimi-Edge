use std::ptr::null_mut;

use crate::il2cpp::{
    symbols::{get_field_from_name, get_field_value, set_field_value},
    types::*,
};

static mut CLIPLENGTH_FIELD: *mut FieldInfo = null_mut();
pub fn get_ClipLength(this: *mut Il2CppObject) -> i32 {
    // SAFETY: FFI / raw pointer operation required by IL2CPP interop
    get_field_value(this, unsafe { CLIPLENGTH_FIELD })
}

pub fn set_ClipLength(this: *mut Il2CppObject, value: i32) {
    // SAFETY: FFI / raw pointer operation required by IL2CPP interop
    set_field_value(this, unsafe { CLIPLENGTH_FIELD }, &value);
}

static mut STARTFRAME_FIELD: *mut FieldInfo = null_mut();
pub fn get_StartFrame(this: *mut Il2CppObject) -> i32 {
    // SAFETY: FFI / raw pointer operation required by IL2CPP interop
    get_field_value(this, unsafe { STARTFRAME_FIELD })
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, StoryTimelineClipData);

    // SAFETY: FFI / raw pointer operation required by IL2CPP interop
    unsafe {
        CLIPLENGTH_FIELD = get_field_from_name(StoryTimelineClipData, c"ClipLength");
        STARTFRAME_FIELD = get_field_from_name(StoryTimelineClipData, c"StartFrame");
    }
}
