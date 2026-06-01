use crate::il2cpp::{
    symbols::{get_method_addr, SingletonLike},
    types::*,
};

static mut CLASS: *mut Il2CppClass = 0 as _;
pub fn class() -> *mut Il2CppClass {
    // SAFETY: FFI / raw pointer operation required by IL2CPP interop
    unsafe { CLASS }
}

pub fn instance() -> *mut Il2CppObject {
    let Some(singleton) = SingletonLike::new(class()) else {
        return 0 as _;
    };
    singleton.instance()
}

static mut GET_SAVELOADER_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_SaveLoader, GET_SAVELOADER_ADDR, *mut Il2CppObject, this: *mut Il2CppObject);

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, SaveDataManager);

    // SAFETY: FFI / raw pointer operation required by IL2CPP interop
    unsafe {
        CLASS = SaveDataManager;
        GET_SAVELOADER_ADDR = get_method_addr(SaveDataManager, c"get_SaveLoader", 0);
    }
}
