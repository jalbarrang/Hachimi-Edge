//! Direct memory reader for career state via IL2CPP singleton chain.
//!
//! Reads character stats, turn info, and career state by walking:
//! ```text
//! WorkDataManager (singleton)
//!   → get_SingleMode() → WorkSingleModeData
//!     → get_Character() → WorkSingleModeCharaData
//!       → get_Speed/Stamina/Power/Guts/Wiz/Hp/MaxHp/FanCount/...()
//! ```
//!
//! All property getters return decrypted values (bypassing ObscuredInt).

use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

use hachimi_plugin_abi::vt;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Snapshot of career state read from game memory.
#[derive(Debug, Clone, Default)]
pub struct CareerSnapshot {
    pub is_playing: bool,
    pub current_turn: i32,
    pub month: i32,

    // Core stats (decrypted from ObscuredInt by the C# getters)
    pub speed: i32,
    pub stamina: i32,
    pub power: i32,
    pub guts: i32,
    pub wiz: i32,
    pub total_stats: i32,

    pub hp: i32,
    pub max_hp: i32,
    pub motivation: i32, // RaceDefine.Motivation enum (1-5)
    pub fan_count: i32,
    // NOTE: get_SkillPoint returns ObscuredInt (struct), not i32.
    // Needs special decryption handling — skipped for now.
    #[allow(dead_code)]
    pub skill_point: i32,

    pub total_races: i32,
    pub win_count: i32,

    /// Training facility levels [Speed, Stamina, Power, Guts, Wisdom].
    /// Read via `GetTrainingLevel(commandId)`. 0 means not available.
    pub training_levels: [i32; 5],
}

/// Whether the memory reader is actively tracking.
pub static TRACKING: AtomicBool = AtomicBool::new(false);

// ---------------------------------------------------------------------------
// Internal: resolved IL2CPP method chain
// ---------------------------------------------------------------------------

/// IL2CPP MethodInfo starts with the method_pointer at offset 0.
/// We read it to get the callable function pointer.
#[inline]
unsafe fn method_ptr(method_info: *const c_void) -> usize {
    // SAFETY: Reading field or calling method on non-null IL2CPP object pointer.
    unsafe { *(method_info as *const usize) }
}

/// Call an instance method that returns `*mut c_void` (an IL2CPP object).
#[inline]
unsafe fn call_obj(this: *mut c_void, mi: *const c_void) -> *mut c_void {
    // SAFETY: Transmuting IL2CPP MethodInfo pointer to callable function pointer.
    let fp: extern "C" fn(*mut c_void, *const c_void) -> *mut c_void = unsafe { std::mem::transmute(method_ptr(mi)) };
    fp(this, mi)
}

/// Call an instance method that returns `i32`.
#[inline]
unsafe fn call_i32(this: *mut c_void, mi: *const c_void) -> i32 {
    // SAFETY: Transmuting IL2CPP MethodInfo pointer to callable function pointer.
    let fp: extern "C" fn(*mut c_void, *const c_void) -> i32 = unsafe { std::mem::transmute(method_ptr(mi)) };
    fp(this, mi)
}

/// Call an instance method that returns `bool` (IL2CPP uses u8).
#[inline]
unsafe fn call_bool(this: *mut c_void, mi: *const c_void) -> bool {
    // SAFETY: Transmuting IL2CPP MethodInfo pointer to callable function pointer.
    let fp: extern "C" fn(*mut c_void, *const c_void) -> u8 = unsafe { std::mem::transmute(method_ptr(mi)) };
    fp(this, mi) != 0
}

/// Call an instance method that takes one `i32` arg and returns `i32`.
/// IL2CPP calling convention: `fn(this, arg1, method_info) -> i32`.
#[inline]
unsafe fn call_i32_with_i32(this: *mut c_void, mi: *const c_void, arg: i32) -> i32 {
    // SAFETY: Transmuting IL2CPP MethodInfo pointer to callable function pointer.
    let fp: extern "C" fn(*mut c_void, i32, *const c_void) -> i32 = unsafe { std::mem::transmute(method_ptr(mi)) };
    fp(this, arg, mi)
}

/// All resolved MethodInfo pointers for the singleton chain.
struct ResolvedChain {
    wdm_klass: *mut c_void,

    // WorkDataManager → WorkSingleModeData
    m_get_single_mode: *const c_void,

    // WorkSingleModeData getters
    m_get_is_playing: *const c_void,
    m_get_character: *const c_void,
    m_get_current_turn: *const c_void,
    m_get_month: *const c_void,
    m_get_total_races: *const c_void,
    m_get_win_count: *const c_void,

    // WorkSingleModeCharaData getters
    m_get_speed: *const c_void,
    m_get_stamina: *const c_void,
    m_get_power: *const c_void,
    m_get_guts: *const c_void,
    m_get_wiz: *const c_void,
    m_get_all_total: *const c_void,
    m_get_hp: *const c_void,
    m_get_max_hp: *const c_void,
    m_get_motivation: *const c_void,
    m_get_fan_count: *const c_void,
    m_get_training_level: *const c_void, // GetTrainingLevel(1 arg: commandId)
    #[allow(dead_code)]
    m_get_scenario_id: *const c_void, // get_ScenarioId(0 args), reserved for scenario detection
}

// SAFETY: IL2CPP class/method pointers are stable for process lifetime.
unsafe impl Send for ResolvedChain {}
// SAFETY: IL2CPP pointers are stable for process lifetime.
unsafe impl Sync for ResolvedChain {}

static CHAIN: OnceLock<ResolvedChain> = OnceLock::new();

// ---------------------------------------------------------------------------
// Resolution helpers
// ---------------------------------------------------------------------------

fn resolve_class(
    image: *const c_void,
    ns: &std::ffi::CStr,
    name: &std::ffi::CStr,
) -> Result<*mut c_void, &'static str> {
    let vt = vt();
    // SAFETY: IL2CPP FFI call; assembly image and class names are valid C strings.
    let klass = unsafe { (vt.il2cpp_get_class)(image.cast_mut(), ns.as_ptr(), name.as_ptr()) };
    if klass.is_null() {
        let label = name.to_str().unwrap_or("?");
        hlog_error!("Class not found: {}", label);
        return Err("IL2CPP class not found");
    }
    Ok(klass)
}

fn resolve_method(klass: *mut c_void, name: &std::ffi::CStr, args: i32) -> Result<*const c_void, &'static str> {
    let vt = vt();
    // SAFETY: IL2CPP FFI call; class pointer resolved above, method name is a valid C string.
    let mi = unsafe { (vt.il2cpp_get_method)(klass.cast(), name.as_ptr(), args) };
    if mi.is_null() {
        let label = name.to_str().unwrap_or("?");
        hlog_error!("Method not found: {} (args={})", label, args);
        return Err("IL2CPP method not found");
    }
    Ok(mi)
}

fn try_resolve() -> Result<ResolvedChain, &'static str> {
    let vt = vt();

    hlog_info!("try_resolve: resolving IL2CPP assembly...");
    // SAFETY: IL2CPP FFI call; host vtable and resolved symbols are valid for process lifetime.
    let image = unsafe { (vt.il2cpp_get_assembly_image)(c"umamusume.dll".as_ptr()) };
    if image.is_null() {
        hlog_error!("try_resolve: umamusume.dll assembly not found");
        return Err("Assembly umamusume.dll not found");
    }
    // Resolve classes
    hlog_info!("try_resolve: resolving classes...");
    let wdm = resolve_class(image, c"Gallop", c"WorkDataManager")?;
    let wsmd = resolve_class(image, c"Gallop", c"WorkSingleModeData")?;
    let wsmcd = resolve_class(image, c"Gallop", c"WorkSingleModeCharaData")?;

    hlog_info!(
        "Resolved classes: WorkDataManager={:?} WorkSingleModeData={:?} WorkSingleModeCharaData={:?}",
        wdm,
        wsmd,
        wsmcd
    );

    hlog_info!("try_resolve: resolving methods...");

    // Resolve methods
    let chain = ResolvedChain {
        wdm_klass: wdm,
        m_get_single_mode: resolve_method(wdm, c"get_SingleMode", 0)?,

        m_get_is_playing: resolve_method(wsmd, c"get_IsPlaying", 0)?,
        m_get_character: resolve_method(wsmd, c"get_Character", 0)?,
        m_get_current_turn: resolve_method(wsmd, c"GetCurrentTurn", 0)?,

        m_get_month: resolve_method(wsmd, c"get_Month", 0)?,
        m_get_total_races: resolve_method(wsmd, c"get_TotalRaceCount", 0)?,
        m_get_win_count: resolve_method(wsmd, c"get_WinCount", 0)?,

        m_get_speed: resolve_method(wsmcd, c"get_Speed", 0)?,
        m_get_stamina: resolve_method(wsmcd, c"get_Stamina", 0)?,
        m_get_power: resolve_method(wsmcd, c"get_Power", 0)?,
        m_get_guts: resolve_method(wsmcd, c"get_Guts", 0)?,
        m_get_wiz: resolve_method(wsmcd, c"get_Wiz", 0)?,
        m_get_all_total: resolve_method(wsmcd, c"GetAllTotalParameterValue", 0)?,
        m_get_hp: resolve_method(wsmcd, c"get_Hp", 0)?,
        m_get_max_hp: resolve_method(wsmcd, c"get_MaxHp", 0)?,
        m_get_motivation: resolve_method(wsmcd, c"get_Motivation", 0)?,
        m_get_fan_count: resolve_method(wsmcd, c"get_FanCount", 0)?,
        m_get_training_level: resolve_method(wsmcd, c"GetTrainingLevel", 1)?,
        m_get_scenario_id: resolve_method(wsmcd, c"get_ScenarioId", 0)?,
    };

    hlog_info!("All 21 methods resolved for memory-read chain");
    Ok(chain)
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Attempt to resolve the IL2CPP method chain and begin tracking.
/// Call from a UI button click.
pub fn start_tracking() -> Result<(), &'static str> {
    // Resolve chain if not already done
    if CHAIN.get().is_none() {
        let chain = try_resolve()?;
        let _ = CHAIN.set(chain); // ignore if race
    }
    TRACKING.store(true, Ordering::Relaxed);
    hlog_info!("Memory-read tracking STARTED");
    Ok(())
}

/// Stop tracking (overlay goes away, no more reads).
pub fn stop_tracking() {
    TRACKING.store(false, Ordering::Relaxed);
    hlog_info!("Memory-read tracking STOPPED");
}

/// Read a snapshot of the current career state from game memory.
/// Returns `None` if the chain isn't resolved or the singleton is unavailable.
pub fn read_snapshot() -> Option<CareerSnapshot> {
    // Catch panics from bad IL2CPP pointers so they don't take down the render thread.
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(read_snapshot_inner)) {
        Ok(result) => result,
        Err(_) => {
            hlog_error!("read_snapshot PANICKED — IL2CPP call likely hit a bad pointer");
            None
        }
    }
}

fn read_snapshot_inner() -> Option<CareerSnapshot> {
    let chain = CHAIN.get()?;
    let vt = vt();

    // Step 1: Get the WorkDataManager singleton
    hlog_trace!("snapshot: step 1 — get singleton");
    // SAFETY: IL2CPP FFI call; host vtable and resolved symbols are valid for process lifetime.
    let singleton = unsafe { (vt.il2cpp_get_singleton_like_instance)(chain.wdm_klass.cast()) };
    if singleton.is_null() {
        return None;
    }
    // Step 2: WorkDataManager → WorkSingleModeData
    hlog_trace!("snapshot: step 2 — get_SingleMode (singleton={:?})", singleton);
    // SAFETY: Reading field or calling method on non-null IL2CPP object pointer.
    let wsmd = unsafe { call_obj(singleton, chain.m_get_single_mode) };
    if wsmd.is_null() {
        return Some(CareerSnapshot::default());
    }

    // Step 3: Check if a career is active
    hlog_trace!("snapshot: step 3 — get_IsPlaying (wsmd={:?})", wsmd);
    // SAFETY: Reading field or calling method on non-null IL2CPP object pointer.
    let is_playing = unsafe { call_bool(wsmd, chain.m_get_is_playing) };

    if !is_playing {
        return Some(CareerSnapshot {
            is_playing: false,
            ..Default::default()
        });
    }

    // Step 4: Read turn/career info from WorkSingleModeData
    // NOTE: Only simple `get_` property accessors are safe here.
    // `GetFinalTurn`/`GetRemainTurnNum` do master-data lookups and crash
    // when called from the render thread.
    hlog_trace!("snapshot: step 4 — turn/career info");
    // SAFETY: Reading field or calling method on non-null IL2CPP object pointer.
    let month = unsafe { call_i32(wsmd, chain.m_get_month) };
    // SAFETY: Reading field or calling method on non-null IL2CPP object pointer.
    let current_turn = unsafe { call_i32(wsmd, chain.m_get_current_turn) };
    // SAFETY: Reading field or calling method on non-null IL2CPP object pointer.
    let total_races = unsafe { call_i32(wsmd, chain.m_get_total_races) };
    // SAFETY: Reading field or calling method on non-null IL2CPP object pointer.
    let win_count = unsafe { call_i32(wsmd, chain.m_get_win_count) };

    // Step 5: WorkSingleModeData → WorkSingleModeCharaData
    hlog_trace!("snapshot: step 5 — get_Character");
    // SAFETY: Reading field or calling method on non-null IL2CPP object pointer.
    let chara = unsafe { call_obj(wsmd, chain.m_get_character) };
    if chara.is_null() {
        hlog_warn!("read_snapshot: get_Character returned null");
        return Some(CareerSnapshot {
            is_playing: true,
            current_turn,
            month,
            total_races,
            win_count,
            ..Default::default()
        });
    }

    // Step 6: Read all stats from WorkSingleModeCharaData
    hlog_trace!("snapshot: step 6 — stats (chara={:?})", chara);
    // SAFETY: Reading field or calling method on non-null IL2CPP object pointer.
    let speed = unsafe { call_i32(chara, chain.m_get_speed) };
    // SAFETY: Reading field or calling method on non-null IL2CPP object pointer.
    let stamina = unsafe { call_i32(chara, chain.m_get_stamina) };
    // SAFETY: Reading field or calling method on non-null IL2CPP object pointer.
    let power = unsafe { call_i32(chara, chain.m_get_power) };
    // SAFETY: Reading field or calling method on non-null IL2CPP object pointer.
    let guts = unsafe { call_i32(chara, chain.m_get_guts) };
    // SAFETY: Reading field or calling method on non-null IL2CPP object pointer.
    let wiz = unsafe { call_i32(chara, chain.m_get_wiz) };
    // SAFETY: Reading field or calling method on non-null IL2CPP object pointer.
    let total_stats = unsafe { call_i32(chara, chain.m_get_all_total) };
    hlog_trace!("snapshot: step 6b — hp/motivation/fans");
    // SAFETY: Reading field or calling method on non-null IL2CPP object pointer.
    let hp = unsafe { call_i32(chara, chain.m_get_hp) };
    // SAFETY: Reading field or calling method on non-null IL2CPP object pointer.
    let max_hp = unsafe { call_i32(chara, chain.m_get_max_hp) };
    // SAFETY: Reading field or calling method on non-null IL2CPP object pointer.
    let motivation = unsafe { call_i32(chara, chain.m_get_motivation) };
    // SAFETY: Reading field or calling method on non-null IL2CPP object pointer.
    let fan_count = unsafe { call_i32(chara, chain.m_get_fan_count) };

    // Step 7: Read training levels per facility
    hlog_trace!("snapshot: step 7 — training levels");
    let training_levels = read_training_levels(chara, chain);

    hlog_trace!("snapshot: complete (turn={}, total={})", current_turn, total_stats);
    Some(CareerSnapshot {
        is_playing: true,
        current_turn,
        month,
        speed,
        stamina,
        power,
        guts,
        wiz,
        total_stats,
        hp,
        max_hp,
        motivation,
        fan_count,
        skill_point: 0, // ObscuredInt — needs decryption, not yet implemented
        total_races,
        win_count,
        training_levels,
    })
}

// ---------------------------------------------------------------------------
// Training level detection
// ---------------------------------------------------------------------------

/// Known command ID sets per scenario: [Speed, Stamina, Power, Guts, Wisdom].
const COMMAND_ID_SETS: &[[i32; 5]] = &[
    [101, 105, 102, 103, 106],      // URA / base
    [601, 602, 603, 604, 605],      // Aoharu
    [1101, 1102, 1103, 1104, 1105], // Make a New Track (Arc)
    [2101, 2102, 2103, 2104, 2105], // UAF type A
    [2201, 2202, 2203, 2204, 2205], // UAF type B
    [2301, 2302, 2303, 2304, 2305], // UAF type C
    [901, 902, 903, 904, 906],      // Onsen (partially confirmed)
];

/// Read training levels for all 5 facilities.
/// Auto-detects the correct command ID set by probing known sets.
/// Returns [0; 5] if anything goes wrong.
fn read_training_levels(chara: *mut c_void, chain: &ResolvedChain) -> [i32; 5] {
    // First, verify the _trainingLevelDic field exists and is non-null.
    // If the dictionary isn't initialized, calling GetTrainingLevel would crash.
    let vt = vt();
    hlog_trace!("training_levels: checking _trainingLevelDic field");
    // SAFETY: IL2CPP FFI call; host vtable and resolved symbols are valid for process lifetime.
    let field = unsafe {
        (vt.il2cpp_get_field_from_name)(
            // We need the chara object's class. We can get it from the object header.
            // IL2CPP objects have klass at offset 0.
            *(chara as *const *mut c_void), // object->klass
            c"_trainingLevelDic".as_ptr(),
        )
    };
    if field.is_null() {
        hlog_trace!("training_levels: _trainingLevelDic field not found");
        return [0; 5];
    }

    // Read the field value (it's an object reference = pointer)
    let mut dict_ptr: *mut c_void = std::ptr::null_mut();
    // SAFETY: IL2CPP FFI call; host vtable and resolved symbols are valid for process lifetime.
    unsafe {
        (vt.il2cpp_get_field_value)(chara.cast(), field.cast(), &mut dict_ptr as *mut _ as *mut c_void);
    }
    if dict_ptr.is_null() {
        hlog_trace!("training_levels: dictionary is null, skipping");
        return [0; 5];
    }

    hlog_trace!("training_levels: probing command ID sets (dict={:?})", dict_ptr);
    for set in COMMAND_ID_SETS {
        let mut levels = [0i32; 5];
        let mut any_positive = false;

        for (i, &cmd_id) in set.iter().enumerate() {
            // SAFETY: Reading field or calling method on non-null IL2CPP object pointer.
            let level = unsafe { call_i32_with_i32(chara, chain.m_get_training_level, cmd_id) };
            levels[i] = level;
            if level > 0 {
                any_positive = true;
            }
        }

        if any_positive {
            static LEVELS_LOGGED: AtomicBool = AtomicBool::new(false);
            if !LEVELS_LOGGED.swap(true, Ordering::Relaxed) {
                hlog_info!("Training levels matched set {:?} → {:?}", set, levels);
            }
            return levels;
        }
    }

    hlog_trace!("training_levels: no matching command ID set found");
    [0; 5]
}

// ---------------------------------------------------------------------------
// IL2CPP string reading
// ---------------------------------------------------------------------------

/// Read an IL2CPP `System.String` object and convert to a Rust `String`.
/// IL2CppString layout (64-bit):
///   offset 0x00: Il2CppObject header (klass + monitor = 16 bytes)
///   offset 0x10: int32 length (in UTF-16 code units)
///   offset 0x14: char16_t[] chars (UTF-16 data)
unsafe fn read_il2cpp_string(str_obj: *mut c_void) -> Option<String> {
    // SAFETY: Reading field or calling method on non-null IL2CPP object pointer.
    unsafe {
        if str_obj.is_null() {
            return None;
        }
        let len = *(str_obj.byte_add(0x10) as *const i32);
        if len <= 0 || len > 4096 {
            return None;
        }
        let chars = str_obj.byte_add(0x14) as *const u16;
        let slice = std::slice::from_raw_parts(chars, len as usize);
        String::from_utf16(slice).ok()
    }
}

/// Call an instance method that takes one `i32` arg and returns `*mut c_void`.
#[inline]
unsafe fn call_obj_with_i32(this: *mut c_void, mi: *const c_void, arg: i32) -> *mut c_void {
    let fp: extern "C" fn(*mut c_void, i32, *const c_void) -> *mut c_void =
        // SAFETY: Transmuting IL2CPP MethodInfo pointer to callable function pointer.
        unsafe { std::mem::transmute(method_ptr(mi)) };
    fp(this, arg, mi)
}

// ---------------------------------------------------------------------------
// Generic IL2CPP List helpers
// ---------------------------------------------------------------------------

/// Read an IL2CPP `List<T>` field from an object.
/// Returns (list_ptr, count, get_Item method) or None.
pub unsafe fn read_list_field(
    obj: *mut c_void,
    field_name: &std::ffi::CStr,
) -> Option<(*mut c_void, i32, *const c_void)> {
    let vt = vt();
    // SAFETY: Reading Il2CppClass header from non-null object pointer.
    let obj_klass = unsafe { *(obj as *const *mut c_void) };
    // SAFETY: IL2CPP FFI call; field name is a valid C string.
    let field = unsafe { (vt.il2cpp_get_field_from_name)(obj_klass, field_name.as_ptr()) };
    if field.is_null() {
        return None;
    }

    let mut list_ptr: *mut c_void = std::ptr::null_mut();
    // SAFETY: IL2CPP FFI call; host vtable and resolved symbols are valid for process lifetime.
    unsafe {
        (vt.il2cpp_get_field_value)(obj.cast(), field.cast(), &mut list_ptr as *mut _ as *mut c_void);
    }
    if list_ptr.is_null() {
        return None;
    }

    // SAFETY: IL2CPP FFI call; host vtable and resolved symbols are valid for process lifetime.
    let list_klass = unsafe { *(list_ptr as *const *mut c_void) };
    // SAFETY: IL2CPP FFI call; host vtable and resolved symbols are valid for process lifetime.
    let m_count = unsafe { (vt.il2cpp_get_method)(list_klass, c"get_Count".as_ptr(), 0) };
    // SAFETY: IL2CPP FFI call; host vtable and resolved symbols are valid for process lifetime.
    let m_item = unsafe { (vt.il2cpp_get_method)(list_klass, c"get_Item".as_ptr(), 1) };
    if m_count.is_null() || m_item.is_null() {
        return None;
    }

    // SAFETY: Reading field or calling method on non-null IL2CPP object pointer.
    let count = unsafe { call_i32(list_ptr, m_count) };
    Some((list_ptr, count, m_item))
}

// ---------------------------------------------------------------------------
// Skill list reading
// ---------------------------------------------------------------------------

/// A single acquired skill read from game memory.
#[derive(Debug, Clone)]
pub struct AcquiredSkillInfo {
    pub master_id: i32,
    pub level: i32,
    pub name: String,
}

/// Read the acquired skill list from the chara object.
/// Returns (list_ptr, count) for diagnostics, or None.
pub fn read_acquired_skill_list() -> Option<(*mut c_void, i32)> {
    let chara = get_chara_ptr()?;
    // SAFETY: Reading field or calling method on non-null IL2CPP object pointer.
    unsafe {
        let (list_ptr, count, _) = read_list_field(chara, c"_acquiredSkillList")?;
        Some((list_ptr, count))
    }
}

/// Read all acquired skills with names.
pub fn read_acquired_skills() -> Vec<AcquiredSkillInfo> {
    // SAFETY: Reading field or calling method on non-null IL2CPP object pointer.
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| unsafe { read_acquired_skills_inner() })) {
        Ok(v) => v,
        Err(_) => {
            hlog_error!("read_acquired_skills PANICKED");
            Vec::new()
        }
    }
}

unsafe fn read_acquired_skills_inner() -> Vec<AcquiredSkillInfo> {
    let chara = match get_chara_ptr() {
        Some(c) => c,
        None => return Vec::new(),
    };

    // SAFETY: Reading field or calling method on non-null IL2CPP object pointer.
    let (list_ptr, count, m_get_item) = match unsafe { read_list_field(chara, c"_acquiredSkillList") } {
        Some(v) => v,
        None => return Vec::new(),
    };

    if count <= 0 || count > 200 {
        return Vec::new();
    }

    let vt = vt();
    let mut skills = Vec::with_capacity(count as usize);
    let mut m_master_id: *const c_void = std::ptr::null();
    let mut m_level: *const c_void = std::ptr::null();
    let mut m_master_data: *const c_void = std::ptr::null();
    let mut m_name: *const c_void = std::ptr::null();
    let mut methods_resolved = false;

    for i in 0..count {
        // SAFETY: Reading field or calling method on non-null IL2CPP object pointer.
        let item = unsafe { call_obj_with_i32(list_ptr, m_get_item, i) };
        if item.is_null() {
            continue;
        }

        // Resolve methods on first element (inherited from SkillDataBase)
        if !methods_resolved {
            methods_resolved = true;
            // SAFETY: IL2CPP FFI call; host vtable and resolved symbols are valid for process lifetime.
            let klass = unsafe { *(item as *const *mut c_void) };

            // SAFETY: IL2CPP FFI call; host vtable and resolved symbols are valid for process lifetime.
            m_master_id = unsafe { (vt.il2cpp_get_method)(klass, c"get_MasterId".as_ptr(), 0) } as _;
            // SAFETY: IL2CPP FFI call; host vtable and resolved symbols are valid for process lifetime.
            m_level = unsafe { (vt.il2cpp_get_method)(klass, c"get_Level".as_ptr(), 0) } as _;
            // SAFETY: IL2CPP FFI call; host vtable and resolved symbols are valid for process lifetime.
            m_master_data = unsafe { (vt.il2cpp_get_method)(klass, c"get_MasterData".as_ptr(), 0) } as _;

            if m_master_id.is_null() || m_level.is_null() {
                hlog_warn!("SkillDataBase methods not found (get_MasterId/get_Level)");
                return Vec::new();
            }

            static LOGGED: AtomicBool = AtomicBool::new(false);
            if !LOGGED.swap(true, Ordering::Relaxed) {
                hlog_info!(
                    "AcquiredSkill: resolved get_MasterId={} get_Level={} get_MasterData={}",
                    !m_master_id.is_null(),
                    !m_level.is_null(),
                    !m_master_data.is_null()
                );
            }
        }

        // SAFETY: Reading field or calling method on non-null IL2CPP object pointer.
        let master_id = unsafe { call_i32(item, m_master_id) };
        // SAFETY: Reading field or calling method on non-null IL2CPP object pointer.
        let level = unsafe { call_i32(item, m_level) };

        // Try to get the name via get_MasterData() -> get_Name()
        let name = if !m_master_data.is_null() {
            // SAFETY: Reading field or calling method on non-null IL2CPP object pointer.
            let master_obj = unsafe { call_obj(item, m_master_data) };
            if !master_obj.is_null() {
                if m_name.is_null() {
                    // SAFETY: IL2CPP FFI call; host vtable and resolved symbols are valid for process lifetime.
                    let master_klass = unsafe { *(master_obj as *const *mut c_void) };
                    // SAFETY: IL2CPP FFI call; host vtable and resolved symbols are valid for process lifetime.
                    m_name = unsafe { (vt.il2cpp_get_method)(master_klass, c"get_Name".as_ptr(), 0) } as _;
                }
                if !m_name.is_null() {
                    // SAFETY: IL2CPP FFI call; host vtable and resolved symbols are valid for process lifetime.
                    let str_obj = unsafe { call_obj(master_obj, m_name) };
                    // SAFETY: IL2CPP FFI call; host vtable and resolved symbols are valid for process lifetime.
                    unsafe { read_il2cpp_string(str_obj) }.unwrap_or_default()
                } else {
                    String::new()
                }
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        skills.push(AcquiredSkillInfo { master_id, level, name });
    }

    skills
}

// ---------------------------------------------------------------------------
// Friendship / Evaluation reading
// ---------------------------------------------------------------------------

/// A single support card's friendship/bond value.
#[derive(Debug, Clone)]
pub struct EvaluationInfo {
    pub target_id: i32,  // support card chara ID
    pub value: i32,      // friendship/bond value (0-100+)
    pub is_appear: bool, // whether the character is present in this career
    pub name: String,    // resolved character name
}

/// Read the evaluation (friendship) list from the chara object.
pub fn read_evaluations() -> Vec<EvaluationInfo> {
    // SAFETY: Reading field or calling method on non-null IL2CPP object pointer.
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| unsafe { read_evaluations_inner() })) {
        Ok(v) => v,
        Err(_) => {
            hlog_error!("read_evaluations PANICKED");
            Vec::new()
        }
    }
}

unsafe fn read_evaluations_inner() -> Vec<EvaluationInfo> {
    let chara = match get_chara_ptr() {
        Some(c) => c,
        None => return Vec::new(),
    };

    // Try known field names for the evaluation list
    let field_names = [c"_evaluationList", c"_evaluationInfoList", c"_evaluations"];

    let mut list_data = None;
    for name in &field_names {
        // SAFETY: Reading field or calling method on non-null IL2CPP object pointer.
        if let Some(data) = unsafe { read_list_field(chara, name) } {
            list_data = Some(data);
            break;
        }
    }

    let (list_ptr, count, m_get_item) = match list_data {
        Some(v) => v,
        None => return Vec::new(),
    };

    if count <= 0 || count > 50 {
        return Vec::new();
    }

    let vt = vt();
    let mut evals = Vec::with_capacity(count as usize);
    let mut m_target_id: *const c_void = std::ptr::null();
    let mut m_value: *const c_void = std::ptr::null();
    let mut m_is_appear: *const c_void = std::ptr::null();
    let mut m_get_chara_name: *const c_void = std::ptr::null();
    let mut methods_resolved = false;

    for i in 0..count {
        // SAFETY: Reading field or calling method on non-null IL2CPP object pointer.
        let item = unsafe { call_obj_with_i32(list_ptr, m_get_item, i) };
        if item.is_null() {
            continue;
        }

        if !methods_resolved {
            methods_resolved = true;
            // SAFETY: IL2CPP FFI call; host vtable and resolved symbols are valid for process lifetime.
            let klass = unsafe { *(item as *const *mut c_void) };

            // SAFETY: IL2CPP FFI call; host vtable and resolved symbols are valid for process lifetime.
            m_target_id = unsafe { (vt.il2cpp_get_method)(klass, c"get_TargetId".as_ptr(), 0) } as _;
            // SAFETY: IL2CPP FFI call; host vtable and resolved symbols are valid for process lifetime.
            m_value = unsafe { (vt.il2cpp_get_method)(klass, c"get_Value".as_ptr(), 0) } as _;
            // SAFETY: IL2CPP FFI call; host vtable and resolved symbols are valid for process lifetime.
            m_is_appear = unsafe { (vt.il2cpp_get_method)(klass, c"get_IsAppear".as_ptr(), 0) } as _;

            if m_target_id.is_null() || m_value.is_null() {
                hlog_warn!("Evaluation methods not found (get_TargetId/get_Value)");
                return Vec::new();
            }

            // Resolve MasterDataUtil.GetCharaNameByCharaId for name lookup
            // SAFETY: IL2CPP FFI calls for class/method resolution
            unsafe {
                let image = (vt.il2cpp_get_assembly_image)(c"umamusume.dll".as_ptr());
                if !image.is_null() {
                    let mdu = (vt.il2cpp_get_class)(image, c"Gallop".as_ptr(), c"MasterDataUtil".as_ptr());
                    if !mdu.is_null() {
                        m_get_chara_name = (vt.il2cpp_get_method)(mdu, c"GetCharaNameByCharaId".as_ptr(), 1) as _;
                    }
                }
            }

            static LOGGED: AtomicBool = AtomicBool::new(false);
            if !LOGGED.swap(true, Ordering::Relaxed) {
                hlog_info!(
                    "Evaluation: resolved get_TargetId + get_Value + get_IsAppear={} + GetCharaName={}",
                    !m_is_appear.is_null(),
                    !m_get_chara_name.is_null()
                );
            }
        }

        // SAFETY: Reading field or calling method on non-null IL2CPP object pointer.
        let target_id = unsafe { call_i32(item, m_target_id) };
        // SAFETY: Reading field or calling method on non-null IL2CPP object pointer.
        let value = unsafe { call_i32(item, m_value) };

        let is_appear = if !m_is_appear.is_null() {
            // SAFETY: Reading field or calling method on non-null IL2CPP object pointer.
            unsafe { call_bool(item, m_is_appear) }
        } else {
            true // assume present if we can't check
        };

        // Resolve name via MasterDataUtil.GetCharaNameByCharaId (static)
        let name = if !m_get_chara_name.is_null() {
            // SAFETY: IL2CPP static method call
            let str_obj = unsafe {
                let fp: extern "C" fn(i32, *const c_void) -> *mut c_void =
                    std::mem::transmute(method_ptr(m_get_chara_name));
                fp(target_id, m_get_chara_name)
            };
            // SAFETY: IL2CPP FFI call; host vtable and resolved symbols are valid for process lifetime.
            unsafe { read_il2cpp_string(str_obj) }.unwrap_or_default()
        } else {
            String::new()
        };

        evals.push(EvaluationInfo {
            target_id,
            value,
            is_appear,
            name,
        });
    }

    evals
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// Get the chara object pointer (WorkSingleModeCharaData) if available.
pub fn get_chara_ptr() -> Option<*mut c_void> {
    let chain = CHAIN.get()?;
    let vt = vt();

    // SAFETY: IL2CPP FFI call; host vtable and resolved symbols are valid for process lifetime.
    let singleton = unsafe { (vt.il2cpp_get_singleton_like_instance)(chain.wdm_klass.cast()) };
    if singleton.is_null() {
        return None;
    }

    // SAFETY: Reading field or calling method on non-null IL2CPP object pointer.
    let wsmd = unsafe { call_obj(singleton, chain.m_get_single_mode) };
    if wsmd.is_null() {
        return None;
    }

    // SAFETY: Reading field or calling method on non-null IL2CPP object pointer.
    let is_playing = unsafe { call_bool(wsmd, chain.m_get_is_playing) };
    if !is_playing {
        return None;
    }

    // SAFETY: Reading field or calling method on non-null IL2CPP object pointer.
    let chara = unsafe { call_obj(wsmd, chain.m_get_character) };
    if chara.is_null() {
        return None;
    }

    Some(chara)
}

/// Map motivation enum value to display string.
pub fn mood_label(m: i32) -> &'static str {
    match m {
        5 => "\u{2b06}\u{2b06} Great",    // ⬆⬆
        4 => "\u{2b06} Good",             // ⬆
        3 => "\u{27a1} Normal",           // ➡
        2 => "\u{2b07} Bad",              // ⬇
        1 => "\u{2b07}\u{2b07} Terrible", // ⬇⬇
        _ => "???",
    }
}

/// Map motivation to color (r, g, b).
pub fn motivation_color(m: i32) -> (u8, u8, u8) {
    match m {
        5 => (255, 200, 50),  // Gold
        4 => (100, 220, 100), // Green
        3 => (200, 200, 200), // Gray
        2 => (100, 150, 255), // Blue
        1 => (255, 70, 70),   // Red
        _ => (200, 200, 200),
    }
}
