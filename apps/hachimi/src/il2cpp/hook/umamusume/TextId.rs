use std::ptr::null_mut;

use crate::il2cpp::{
    api::{il2cpp_class_get_type, il2cpp_type_get_object},
    ext::StringExt,
    hook::mscorlib::Enum,
    symbols::IEnumerable,
    types::*,
};

static mut TEXTID_TYPE_OBJECT: *mut Il2CppObject = null_mut();

pub fn get_name(value: i32) -> *const Il2CppString {
    // SAFETY: FFI / raw pointer operation required by IL2CPP interop
    let text_id = Enum::ToObject(unsafe { TEXTID_TYPE_OBJECT }, value);
    Enum::ToString(text_id)
}

// this is named like a constructor to pretend that i32 = TextId
// because that's how it's represented in il2cpp
pub fn from_name(name: &str) -> i32 {
    // SAFETY: FFI / raw pointer operation required by IL2CPP interop
    let text_id = Enum::Parse(unsafe { TEXTID_TYPE_OBJECT }, name.to_il2cpp_string());
    Enum::ToUInt64(text_id) as i32
}

pub fn get_values() -> IEnumerable {
    // SAFETY: FFI / raw pointer operation required by IL2CPP interop
    Enum::GetValues(unsafe { TEXTID_TYPE_OBJECT })
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, TextId);

    // SAFETY: FFI / raw pointer operation required by IL2CPP interop
    unsafe {
        TEXTID_TYPE_OBJECT = il2cpp_type_get_object(il2cpp_class_get_type(TextId));
    }
}
