use crate::il2cpp::{
    hook::umamusume::TextFrame,
    symbols::{create_delegate, get_method_addr, GCHandle},
    types::*,
};

use super::{AsyncOperation, Object};

type UnloadUnusedAssetsFn = extern "C" fn() -> *mut Il2CppObject;
extern "C" fn UnloadUnusedAssets() -> *mut Il2CppObject {
    let res = get_orig_fn!(UnloadUnusedAssets, UnloadUnusedAssetsFn)();
    // SAFETY: FFI / raw pointer operation required by IL2CPP interop
    let delegate = create_delegate(unsafe { AsyncOperation::ACTION_ASYNCOPERATION_CLASS }, 1, || {
        TextFrame::PROCESSED
            .lock()
            .expect("lock poisoned")
            .retain(retain_object_gc_handle);
    })
    .expect("unexpected failure");
    AsyncOperation::add_completed(res, delegate);

    res
}

fn retain_object_gc_handle(_ptr: &usize, gc_handle: &mut GCHandle) -> bool {
    let obj = gc_handle.target();
    if obj.is_null() {
        return false;
    }
    Object::IsNativeObjectAlive(obj)
}

pub fn init(UnityEngine_CoreModule: *const Il2CppImage) {
    get_class_or_return!(UnityEngine_CoreModule, UnityEngine, Resources);

    let UnloadUnusedAssets_addr = get_method_addr(Resources, c"UnloadUnusedAssets", 0);

    new_hook!(UnloadUnusedAssets_addr, UnloadUnusedAssets);
}
