//! Per-turn training command preview read live from the working data.
//!
//! Path (all IL2CPP getters return decrypted values):
//! ```text
//! WorkSingleModeData.get_HomeInfo() -> WorkSingleModeHomeInfo
//!   .get_TurnInfoListDic() -> Dictionary<CommandType, List<TurnInfo>>
//!     [Training] -> List<WorkSingleModeData.TurnInfo>
//!       .get_CommandId()           -> facility command id
//!       .get_TrainingFailureRate() -> failure % (plain Int32)
//!       .ParamIncDecInfoDic        -> Dictionary<ParameterType, ParamsIncDecInfo>
//!         [Speed..Wiz].Value (ObscuredInt) -> per-stat gain
//! ```
//!
//! All methods are resolved from each object's runtime klass to avoid resolving
//! nested IL2CPP classes up front. Reads run on the Unity main thread only.

use std::ffi::c_void;

use hachimi_plugin_sdk::Sdk;

use super::il2cpp::{
    call_i32, call_obj, call_obj_with_i32, dict_try_get_obj, read_obscured_int_field, resolve_obj_method,
};

/// `Gallop.SingleModeDefine.CommandType.Training`.
const COMMAND_TYPE_TRAINING: i32 = 1;
/// `Gallop.SingleModeDefine.ParameterType` values for the 5 main stats (Speed..Wiz).
const STAT_PARAM_TYPES: [i32; 5] = [1, 2, 3, 4, 5];

/// One training facility's live preview (failure rate + total stat gain).
#[derive(Debug, Clone, Copy, Default)]
pub struct CommandInfo {
    pub command_id: i32,
    pub failure_rate: i32,
    pub stat_gain: i32,
}

/// Read every training-facility command info for the current turn.
/// `wsmd` is the `WorkSingleModeData` object pointer. Returns empty on failure.
pub(super) fn read_command_infos(wsmd: *mut c_void) -> Vec<CommandInfo> {
    // SAFETY: `wsmd` is a valid non-null IL2CPP object from the resolved chain.
    unsafe { read_command_infos_inner(wsmd) }.unwrap_or_default()
}

unsafe fn read_command_infos_inner(wsmd: *mut c_void) -> Option<Vec<CommandInfo>> {
    if wsmd.is_null() {
        return None;
    }
    // SAFETY: each step calls/reads on a non-null IL2CPP object verified below.
    unsafe {
        let m_home = resolve_obj_method(wsmd, "get_HomeInfo", 0)?;
        let home = call_obj(wsmd, m_home);
        let m_dic = resolve_obj_method(home, "get_TurnInfoListDic", 0)?;
        let dict = call_obj(home, m_dic);
        let m_try = resolve_obj_method(dict, "TryGetValue", 2)?;
        let list = dict_try_get_obj(dict, m_try, COMMAND_TYPE_TRAINING);
        if list.is_null() {
            return None;
        }
        let m_count = resolve_obj_method(list, "get_Count", 0)?;
        let m_item = resolve_obj_method(list, "get_Item", 1)?;
        let count = call_i32(list, m_count);
        if !(0..=64).contains(&count) {
            return None;
        }
        let mut out = Vec::with_capacity(count as usize);
        for i in 0..count {
            let ti = call_obj_with_i32(list, m_item, i);
            if ti.is_null() {
                continue;
            }
            out.push(read_turn_info(ti));
        }
        Some(out)
    }
}

/// Read a single `TurnInfo`: command id, failure rate, and total stat gain.
unsafe fn read_turn_info(ti: *mut c_void) -> CommandInfo {
    // SAFETY: `ti` is a non-null IL2CPP TurnInfo object.
    unsafe {
        let command_id = resolve_obj_method(ti, "get_CommandId", 0)
            .map(|m| call_i32(ti, m))
            .unwrap_or(0);
        let failure_rate = resolve_obj_method(ti, "get_TrainingFailureRate", 0)
            .map(|m| call_i32(ti, m))
            .unwrap_or(0);
        let stat_gain = read_total_stat_gain(ti);
        CommandInfo {
            command_id,
            failure_rate,
            stat_gain,
        }
    }
}

/// Sum the base `Value` over the 5 main stats in `ParamIncDecInfoDic`.
unsafe fn read_total_stat_gain(ti: *mut c_void) -> i32 {
    let sdk = Sdk::get();
    // SAFETY: IL2CPP object header — klass pointer at offset 0.
    let klass = unsafe { *(ti as *const *mut c_void) };
    let Some(field) = sdk.get_field_from_name(klass.cast(), "ParamIncDecInfoDic") else {
        return 0;
    };
    let mut dict: *mut c_void = std::ptr::null_mut();
    // SAFETY: IL2CPP object and field from resolved metadata.
    unsafe {
        sdk.get_field_value(ti.cast(), field, &mut dict as *mut _ as *mut c_void);
    }
    if dict.is_null() {
        return 0;
    }
    // SAFETY: `dict` is a non-null IL2CPP Dictionary object.
    let Some(m_try) = (unsafe { resolve_obj_method(dict, "TryGetValue", 2) }) else {
        return 0;
    };
    let mut total = 0;
    for &pt in &STAT_PARAM_TYPES {
        // SAFETY: TryGetValue with a value-type key; null when the stat is absent.
        let info = unsafe { dict_try_get_obj(dict, m_try, pt) };
        if !info.is_null() {
            // SAFETY: `info` is a non-null ParamsIncDecInfo object.
            total += unsafe { read_param_value(info) };
        }
    }
    total
}

/// Read `ParamsIncDecInfo.Value` (an ObscuredInt) and decrypt it.
unsafe fn read_param_value(info: *mut c_void) -> i32 {
    let sdk = Sdk::get();
    // SAFETY: IL2CPP object header — klass pointer at offset 0.
    let klass = unsafe { *(info as *const *mut c_void) };
    let Some(field) = sdk.get_field_from_name(klass.cast(), "Value") else {
        return 0;
    };
    // SAFETY: ObscuredInt field on a valid ParamsIncDecInfo object.
    unsafe { read_obscured_int_field(info, field.cast()) }
}
