use std::ffi::CStr;

use crate::il2cpp::{
    symbols::{get_field_object_value, FieldsIter},
    types::*,
};

// Special adaptation for the fields in this class because there are SO MANY OF THEM
static mut MOTION_TRACK_DATA_FIELDS: Vec<*mut FieldInfo> = Vec::new();
pub fn motion_track_data_values(this: *mut Il2CppObject) -> impl Iterator<Item = *mut Il2CppObject> {
    // SAFETY: FFI / raw pointer operation required by IL2CPP interop
    unsafe {
        MOTION_TRACK_DATA_FIELDS
            .iter()
            .map(move |f| get_field_object_value(this, *f))
    }
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, StoryTimelineCharaTrackData);

    // SAFETY: FFI / raw pointer operation required by IL2CPP interop
    unsafe {
        MOTION_TRACK_DATA_FIELDS = FieldsIter::new(StoryTimelineCharaTrackData)
            .filter(|f| {
                CStr::from_ptr((**f).name)
                    .to_str()
                    .is_ok_and(|s| s.ends_with("MotionTrackData"))
            })
            .collect();
    }
}
