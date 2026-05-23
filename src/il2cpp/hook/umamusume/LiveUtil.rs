use crate::{
    core::Hachimi,
    il2cpp::{symbols::get_method_addr, types::*},
};

type GetSingCharaIdListFn = extern "C" fn(
    songId: i32,
    songPartNumber: i32,
    allCharaIdArray: *mut Il2CppArray,
    vocalCharaIdArray: *mut Il2CppArray,
    shuffledCharaDataList: *mut Il2CppObject,
) -> *mut Il2CppObject;
extern "C" fn GetSingCharaIdList(
    songId: i32,
    songPartNumber: i32,
    allCharaIdArray: *mut Il2CppArray,
    vocalCharaIdArray: *mut Il2CppArray,
    shuffledCharaDataList: *mut Il2CppObject,
) -> *mut Il2CppObject {
    let chara_vo_ids = &Hachimi::instance().config.load().live_vocals_swap;

    if songId > 0 {
        // SAFETY: FFI / raw pointer operation required by IL2CPP interop
        unsafe {
            if !vocalCharaIdArray.is_null() {
                let len = (*vocalCharaIdArray).max_length;
                let data_ptr = vocalCharaIdArray.add(1) as *mut i32;

                for (i, &id) in chara_vo_ids.iter().enumerate().take(len) {
                    if id != 0 {
                        *data_ptr.add(i) = id;
                    }
                }
            }

            if !allCharaIdArray.is_null() {
                let len = (*allCharaIdArray).max_length;
                let data_ptr = allCharaIdArray.add(1) as *mut i32;

                for (i, &new_id) in chara_vo_ids.iter().enumerate().take(len) {
                    if new_id != 0 {
                        *data_ptr.add(i) = new_id;
                    }
                }
            }
        }
    }

    get_orig_fn!(GetSingCharaIdList, GetSingCharaIdListFn)(
        songId,
        songPartNumber,
        allCharaIdArray,
        vocalCharaIdArray,
        shuffledCharaDataList,
    )
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, "Gallop", LiveUtil);

    let GetSingCharaIdList_addr = get_method_addr(LiveUtil, c"GetSingCharaIdList", 5);
    new_hook!(GetSingCharaIdList_addr, GetSingCharaIdList);
}
