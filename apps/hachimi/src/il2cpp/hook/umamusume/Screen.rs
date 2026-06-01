use crate::il2cpp::{symbols::get_method_addr, types::*};

#[cfg(target_os = "windows")]
type GetWidthFn = extern "C" fn() -> i32;
#[cfg(target_os = "windows")]
extern "C" fn get_Width() -> i32 {
    if let Some((width, _)) = crate::windows::utils::get_scaling_res() {
        return width;
    }

    get_orig_fn!(get_Width, GetWidthFn)()
}

#[cfg(target_os = "windows")]
pub fn get_Width_orig() -> i32 {
    get_orig_fn!(get_Width, GetWidthFn)()
}

#[cfg(target_os = "windows")]
type GetHeightFn = extern "C" fn() -> i32;
#[cfg(target_os = "windows")]
extern "C" fn get_Height() -> i32 {
    if let Some((_, height)) = crate::windows::utils::get_scaling_res() {
        return height;
    }

    get_orig_fn!(get_Height, GetHeightFn)()
}

#[cfg(target_os = "windows")]
pub fn get_Height_orig() -> i32 {
    get_orig_fn!(get_Height, GetHeightFn)()
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, Screen);

    #[cfg(target_os = "windows")]
    {
        let get_Width_addr = get_method_addr(Screen, c"get_Width", 0);
        let get_Height_addr = get_method_addr(Screen, c"get_Height", 0);

        new_hook!(get_Width_addr, get_Width);
        new_hook!(get_Height_addr, get_Height);
    }
}
