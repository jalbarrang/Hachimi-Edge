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

use std::collections::HashMap;
use std::ffi::c_void;
use std::sync::Mutex;

use hachimi_plugin_sdk::Sdk;

use super::il2cpp::{
    call_i32, call_obj, call_obj_with_i32, dict_try_get_obj, read_obscured_int_field, resolve_obj_method,
};

/// `Gallop.SingleModeDefine.CommandType.Training`.
const COMMAND_TYPE_TRAINING: i32 = 1;
/// `Gallop.SingleModeDefine.ParameterType` values for the 5 main stats (Speed..Wiz).
const STAT_PARAM_TYPES: [i32; 5] = [1, 2, 3, 4, 5];

/// One training facility's live preview (failure rate + stat gains).
#[derive(Debug, Clone, Copy, Default)]
pub struct CommandInfo {
    pub command_id: i32,
    pub failure_rate: i32,
    /// Total stat gain summed over the 5 main stats.
    pub stat_gain: i32,
    /// Per-stat gain [Speed, Stamina, Power, Guts, Wisdom].
    pub per_stat: [i32; 5],
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
        // Displayed preview = base Value + client-computed BonusValue (support cards,
        // scenario amplifiers). `BonusParamIncDecInfoDic` is logged separately to
        // confirm it is not an additional (double-counted) term — see issue 23x.
        let main = read_param_dict(ti, "ParamIncDecInfoDic");
        let bonus2 = read_param_dict(ti, "BonusParamIncDecInfoDic");
        let per_stat: [i32; 5] = std::array::from_fn(|s| main[s].0 + main[s].1);
        let stat_gain = per_stat.iter().sum();
        log_breakdown_on_change(command_id, &main, &bonus2);
        CommandInfo {
            command_id,
            failure_rate,
            stat_gain,
            per_stat,
        }
    }
}

/// Read per-stat `(Value, BonusValue)` for the 5 main stats from a `TurnInfo` dict
/// field (`ParamIncDecInfoDic` or `BonusParamIncDecInfoDic`). Missing stat → (0, 0).
unsafe fn read_param_dict(ti: *mut c_void, field_name: &str) -> [(i32, i32); 5] {
    let mut out = [(0i32, 0i32); 5];
    let sdk = Sdk::get();
    // SAFETY: IL2CPP object header — klass pointer at offset 0.
    let klass = unsafe { *(ti as *const *mut c_void) };
    let Some(field) = sdk.get_field_from_name(klass.cast(), field_name) else {
        return out;
    };
    let mut dict: *mut c_void = std::ptr::null_mut();
    // SAFETY: IL2CPP object and field from resolved metadata.
    unsafe {
        sdk.get_field_value(ti.cast(), field, &mut dict as *mut _ as *mut c_void);
    }
    if dict.is_null() {
        return out;
    }
    // SAFETY: `dict` is a non-null IL2CPP Dictionary object.
    let Some(m_try) = (unsafe { resolve_obj_method(dict, "TryGetValue", 2) }) else {
        return out;
    };
    for (i, &pt) in STAT_PARAM_TYPES.iter().enumerate() {
        // SAFETY: TryGetValue with a value-type key; null when the stat is absent.
        let info = unsafe { dict_try_get_obj(dict, m_try, pt) };
        if !info.is_null() {
            // SAFETY: `info` is a non-null ParamsIncDecInfo object.
            out[i] = unsafe { read_param_values(info) };
        }
    }
    out
}

/// Read `(Value, BonusValue)` (both ObscuredInt) from a ParamsIncDecInfo object.
unsafe fn read_param_values(info: *mut c_void) -> (i32, i32) {
    let sdk = Sdk::get();
    // SAFETY: IL2CPP object header — klass pointer at offset 0.
    let klass = unsafe { *(info as *const *mut c_void) };
    let read = |name: &str| {
        sdk.get_field_from_name(klass.cast(), name)
            // SAFETY: ObscuredInt field on a valid ParamsIncDecInfo object.
            .map(|f| unsafe { read_obscured_int_field(info, f.cast()) })
            .unwrap_or(0)
    };
    (read("Value"), read("BonusValue"))
}

/// Diagnostic (23x): log a facility's gain breakdown whenever it CHANGES, so the
/// amplifier-active turn is captured without spamming the ~2s refresh. Deduped per
/// command id. Temporary — remove once the bonus source is settled.
fn log_breakdown_on_change(command_id: i32, main: &[(i32, i32); 5], bonus2: &[(i32, i32); 5]) {
    static LAST: Mutex<Option<HashMap<i32, [i32; 15]>>> = Mutex::new(None);
    let base: [i32; 5] = std::array::from_fn(|s| main[s].0);
    let bonus: [i32; 5] = std::array::from_fn(|s| main[s].1);
    let b2: [i32; 5] = std::array::from_fn(|s| bonus2[s].0 + bonus2[s].1);
    let mut sig = [0i32; 15];
    sig[..5].copy_from_slice(&base);
    sig[5..10].copy_from_slice(&bonus);
    sig[10..].copy_from_slice(&b2);

    if let Ok(mut guard) = LAST.lock() {
        let map = guard.get_or_insert_with(HashMap::new);
        if map.get(&command_id) == Some(&sig) {
            return; // unchanged since last refresh
        }
        map.insert(command_id, sig);
    }
    hlog_info!(
        "Gain breakdown cmd={}: base={:?} bonus={:?} bonusDic={:?} (shown=base+bonus)",
        command_id,
        base,
        bonus,
        b2
    );
}
