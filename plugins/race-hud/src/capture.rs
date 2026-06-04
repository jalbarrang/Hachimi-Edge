//! Race data capture (IL2CPP hooks, all on the game thread).
//!
//! - `Gallop.RaceInfo.get_RaceTrackId` → grab + decode `<SimDataBase64>` (frames).
//! - `Gallop.RaceManager.get_ElapsedTime()` is read by the race UI every frame, so
//!   we hook it as a per-frame trigger. It returns 0 (not the playback clock), so
//!   for the actual time we call `get_AccumulateTimeSinceStart(this)` and sample
//!   the decoded frames on a ~500ms cadence.
//!
//! The overlay never touches IL2CPP; it only reads the snapshot in `state`.

use std::ffi::c_void;
use std::panic::{self, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use hachimi_plugin_abi::{FieldInfo, Il2CppImage, Il2CppObject, MethodInfo};
use hachimi_plugin_sdk::Sdk;

const SIMDATA_FIELD: &str = "<SimDataBase64>k__BackingField";
const SAMPLE_CADENCE: Duration = Duration::from_millis(500);

// ── RaceInfo SimData hook ──
static ORIG_GET_RACE_TRACK_ID: AtomicUsize = AtomicUsize::new(0);
static SIMDATA_FIELD_INFO: AtomicUsize = AtomicUsize::new(0);
static RACE_INFO_HOOK_ADDR: AtomicUsize = AtomicUsize::new(0);

// ── Runner names (resolved, not hooked) ──
static RACE_HORSE_FN: AtomicUsize = AtomicUsize::new(0);
static RACE_HORSE_MI: AtomicUsize = AtomicUsize::new(0);
static CHARANAME_FN: AtomicUsize = AtomicUsize::new(0);
static CHARANAME_MI: AtomicUsize = AtomicUsize::new(0);

// ── RaceManager live hook ──
static ORIG_GET_ELAPSED: AtomicUsize = AtomicUsize::new(0);
static LIVE_HOOK_ADDR: AtomicUsize = AtomicUsize::new(0);
// Resolved (unhooked) `get_AccumulateTimeSinceStart` used to read the advancing time.
static ACCUM_FN_ADDR: AtomicUsize = AtomicUsize::new(0);
static ACCUM_METHOD_INFO: AtomicUsize = AtomicUsize::new(0);
static LAST_SAMPLE: Mutex<Option<Instant>> = Mutex::new(None);
static FIRST_SAMPLE_LOGGED: AtomicBool = AtomicBool::new(false);

type GetRaceTrackIdFn = extern "C" fn(this: *mut Il2CppObject, method: *const MethodInfo) -> i32;
type GetSingleFn = extern "C" fn(this: *mut Il2CppObject, method: *const MethodInfo) -> f32;
type GetObjFn = extern "C" fn(this: *mut Il2CppObject, method: *const MethodInfo) -> *mut c_void;

// ─────────────────────────── RaceInfo / SimData ───────────────────────────

extern "C" fn get_race_track_id_hook(this: *mut Il2CppObject, method: *const MethodInfo) -> i32 {
    let orig = ORIG_GET_RACE_TRACK_ID.load(Ordering::Acquire);
    let ret = if orig != 0 {
        // SAFETY: trampoline address produced by the host interceptor for this signature.
        let orig_fn: GetRaceTrackIdFn = unsafe { std::mem::transmute(orig) };
        orig_fn(this, method)
    } else {
        0
    };

    let _ = panic::catch_unwind(AssertUnwindSafe(|| capture_simdata(this)));
    ret
}

fn capture_simdata(race_info: *mut Il2CppObject) {
    if race_info.is_null() {
        return;
    }
    let field = SIMDATA_FIELD_INFO.load(Ordering::Acquire) as *mut FieldInfo;
    if field.is_null() {
        return;
    }

    // Reference-type field: the field value is an `Il2CppString*`.
    let mut simdata_str: *mut c_void = std::ptr::null_mut();
    // SAFETY: `race_info` is a live RaceInfo passed by the game; `field` was
    // resolved from RaceInfo's class; out buffer matches a pointer-sized value.
    unsafe {
        Sdk::get().get_field_value(race_info, field, std::ptr::from_mut(&mut simdata_str).cast());
    }

    let len = il2cpp_string_len(simdata_str);
    if !crate::state::is_new_signature(race_info as usize, len) {
        return;
    }

    let decoded = read_il2cpp_string(simdata_str).and_then(|b64| match crate::sim::decode_full(&b64) {
        Ok(d) => Some(d),
        Err(e) => {
            hlog_warn!(target: "race-hud", "SimData decode failed: {}", e);
            None
        }
    });

    match &decoded {
        Some(d) => hlog_info!(
            target: "race-hud",
            "Race captured @ {:#x}: {} runners, {} frames, ~{:.0}m (v{})",
            race_info as usize,
            d.summary.horse_num,
            d.summary.frame_count,
            d.summary.race_length_m,
            d.summary.version
        ),
        None => hlog_info!(
            target: "race-hud",
            "Race captured @ {:#x}; SimDataBase64 len={} (decode unavailable)",
            race_info as usize,
            len
        ),
    }

    let count = decoded.as_ref().map_or(0, |d| d.summary.horse_num.max(0) as usize);
    let names = read_chara_names(race_info, count);

    crate::state::set_decoded(race_info as usize, len, decoded, names);
    *LAST_SAMPLE.lock().expect("race-hud sample lock poisoned") = None;
    FIRST_SAMPLE_LOGGED.store(false, Ordering::Release);
}

/// Read the `charaName` of each runner via `RaceInfo.get_RaceHorse()` (HorseData[]
/// in horse-index order). Returns an empty vec if accessors are unavailable.
fn read_chara_names(race_info: *mut Il2CppObject, count: usize) -> Vec<String> {
    let arr_fn = RACE_HORSE_FN.load(Ordering::Acquire);
    let name_fn = CHARANAME_FN.load(Ordering::Acquire);
    if arr_fn == 0 || name_fn == 0 || count == 0 {
        return Vec::new();
    }

    // SAFETY: resolved 0-arg getter on the live RaceInfo; returns a HorseData[].
    let get_race_horse: GetObjFn = unsafe { std::mem::transmute(arr_fn) };
    let arr = get_race_horse(race_info, RACE_HORSE_MI.load(Ordering::Acquire) as *const MethodInfo);
    if arr.is_null() {
        return Vec::new();
    }

    // SZ array layout (64-bit): [klass][monitor][bounds][usize max_length][elems...]
    // → max_length at byte 24, element pointers start at byte 32.
    // SAFETY: `arr` is a live Il2CppArray of references.
    let max_len = unsafe { *(arr.cast::<u8>().add(24).cast::<usize>()) };
    let n = count.min(max_len);
    // SAFETY: SZ-array element pointers begin at the fixed 32-byte header offset.
    let base = unsafe { arr.cast::<u8>().add(32).cast::<*mut c_void>() };

    // SAFETY: resolved 0-arg `HorseData.get_charaName` returning an Il2CppString.
    let get_chara_name: GetObjFn = unsafe { std::mem::transmute(name_fn) };
    let name_mi = CHARANAME_MI.load(Ordering::Acquire) as *const MethodInfo;

    let mut names = Vec::with_capacity(n);
    for i in 0..n {
        // SAFETY: `i < max_len`; each element is an 8-byte object pointer.
        let horse = unsafe { *base.add(i) };
        let name = if horse.is_null() {
            String::new()
        } else {
            let s = get_chara_name(horse, name_mi);
            read_il2cpp_string(s).unwrap_or_default()
        };
        names.push(name);
    }
    names
}

/// Read the length field of an `Il2CppString`. Layout on 64-bit:
/// `[klass ptr][monitor ptr][i32 length][u16 chars...]` → length at byte 16.
fn il2cpp_string_len(s: *mut c_void) -> i32 {
    if s.is_null() {
        return 0;
    }
    // SAFETY: non-null Il2CppString pointer; length lives at the fixed header offset.
    unsafe { *(s.cast::<u8>().add(16).cast::<i32>()) }
}

/// Read an `Il2CppString` into a Rust `String` (UTF-16 chars start at byte 20).
fn read_il2cpp_string(s: *mut c_void) -> Option<String> {
    if s.is_null() {
        return None;
    }
    let len = il2cpp_string_len(s);
    if !(1..=8_000_000).contains(&len) {
        return None;
    }
    // SAFETY: non-null Il2CppString; `len` chars of UTF-16 live at the fixed offset.
    let chars = unsafe { std::slice::from_raw_parts(s.cast::<u8>().add(20).cast::<u16>(), len as usize) };
    String::from_utf16(chars).ok()
}

// ─────────────────────────── RaceManager / live ───────────────────────────

extern "C" fn get_elapsed_hook(this: *mut Il2CppObject, method: *const MethodInfo) -> f32 {
    let orig = ORIG_GET_ELAPSED.load(Ordering::Acquire);
    if orig == 0 {
        return 0.0;
    }
    // SAFETY: trampoline address produced by the host interceptor for this signature.
    let orig_fn: GetSingleFn = unsafe { std::mem::transmute(orig) };
    let value = orig_fn(this, method);

    let _ = panic::catch_unwind(AssertUnwindSafe(|| {
        // ElapsedTime stays 0; read the advancing cumulative time instead.
        let t = race_time(this).unwrap_or(value);
        sample_if_due(t);
    }));
    value
}

/// Read `RaceManager.get_AccumulateTimeSinceStart(this)` (resolved, not hooked).
fn race_time(race_manager: *mut Il2CppObject) -> Option<f32> {
    let addr = ACCUM_FN_ADDR.load(Ordering::Acquire);
    if addr == 0 {
        return None;
    }
    let mi = ACCUM_METHOD_INFO.load(Ordering::Acquire) as *const MethodInfo;
    // SAFETY: addr is the compiled `get_AccumulateTimeSinceStart` (Single, 0 args);
    // `race_manager` is the live RaceManager passed into the ElapsedTime hook.
    let f: GetSingleFn = unsafe { std::mem::transmute(addr) };
    let t = f(race_manager, mi);
    t.is_finite().then_some(t)
}

fn sample_if_due(elapsed: f32) {
    if !elapsed.is_finite() || elapsed < 0.0 {
        return;
    }

    // Throttle to the configured cadence.
    {
        let now = Instant::now();
        let mut last = LAST_SAMPLE.lock().expect("race-hud sample lock poisoned");
        match *last {
            Some(prev) if now.duration_since(prev) < SAMPLE_CADENCE => return,
            _ => *last = Some(now),
        }
    }

    if !FIRST_SAMPLE_LOGGED.swap(true, Ordering::AcqRel) {
        hlog_info!(target: "race-hud", "Live feed sampling started (t={:.2}s)", elapsed);
    }
    crate::state::sample_live(elapsed);
}

// ─────────────────────────────── install ───────────────────────────────

/// Resolve classes, install both hooks. Returns `true` if the SimData hook is up
/// (the live hook is best-effort and logged separately).
pub fn install() -> bool {
    let sdk = Sdk::get();
    if !sdk.has_capability(hachimi_plugin_abi::capability::IL2CPP) {
        hlog_warn!(target: "race-hud", "Host does not advertise IL2CPP capability");
        return false;
    }

    let Some(image) = sdk.get_assembly_image("umamusume.dll") else {
        hlog_warn!(target: "race-hud", "umamusume.dll image not found");
        return false;
    };

    let ok = install_race_info(sdk, image);
    install_race_manager(sdk, image);
    ok
}

fn install_race_info(sdk: &Sdk, image: *const Il2CppImage) -> bool {
    let Some(class) = sdk.get_class(image, "Gallop", "RaceInfo") else {
        hlog_warn!(target: "race-hud", "Gallop.RaceInfo class not found");
        return false;
    };

    match sdk.get_field_from_name(class, SIMDATA_FIELD) {
        Some(field) => SIMDATA_FIELD_INFO.store(field as usize, Ordering::Release),
        None => {
            hlog_warn!(target: "race-hud", "RaceInfo SimData field not found");
            return false;
        }
    }

    // Runner-name accessors (best-effort; live feed still works without names).
    if let Some(addr) = sdk.get_method_addr(class, "get_RaceHorse", 0) {
        RACE_HORSE_FN.store(addr as usize, Ordering::Release);
    }
    if let Some(mi) = sdk.get_method(class, "get_RaceHorse", 0) {
        RACE_HORSE_MI.store(mi as usize, Ordering::Release);
    }
    if let Some(hd) = sdk.get_class(image, "Gallop", "HorseData") {
        if let Some(addr) = sdk.get_method_addr(hd, "get_charaName", 0) {
            CHARANAME_FN.store(addr as usize, Ordering::Release);
        }
        if let Some(mi) = sdk.get_method(hd, "get_charaName", 0) {
            CHARANAME_MI.store(mi as usize, Ordering::Release);
        }
    }

    let Some(method_addr) = sdk.get_method_addr(class, "get_RaceTrackId", 0) else {
        hlog_warn!(target: "race-hud", "RaceInfo.get_RaceTrackId not found");
        return false;
    };

    let hook_addr = get_race_track_id_hook as *mut c_void;
    match sdk.hook(method_addr, hook_addr) {
        Some(tramp) => {
            ORIG_GET_RACE_TRACK_ID.store(tramp as usize, Ordering::Release);
            RACE_INFO_HOOK_ADDR.store(hook_addr as usize, Ordering::Release);
            hlog_info!(target: "race-hud", "Hooked Gallop.RaceInfo.get_RaceTrackId");
            true
        }
        None => {
            hlog_warn!(target: "race-hud", "Failed to hook RaceInfo.get_RaceTrackId");
            false
        }
    }
}

fn install_race_manager(sdk: &Sdk, image: *const Il2CppImage) {
    let Some(class) = sdk.get_class(image, "Gallop", "RaceManager") else {
        hlog_warn!(target: "race-hud", "Gallop.RaceManager class not found; live feed disabled");
        return;
    };

    // Time source: AccumulateTimeSinceStart getter (advances; the setter is never
    // called but the backing field is updated each frame).
    if let Some(addr) = sdk.get_method_addr(class, "get_AccumulateTimeSinceStart", 0) {
        ACCUM_FN_ADDR.store(addr as usize, Ordering::Release);
    } else {
        hlog_warn!(target: "race-hud", "RaceManager.get_AccumulateTimeSinceStart not found; falling back to ElapsedTime");
    }
    if let Some(mi) = sdk.get_method(class, "get_AccumulateTimeSinceStart", 0) {
        ACCUM_METHOD_INFO.store(mi as usize, Ordering::Release);
    }

    // Per-frame trigger: the race UI reads ElapsedTime every frame (0-arg → no ABI guesswork).
    let Some(method_addr) = sdk.get_method_addr(class, "get_ElapsedTime", 0) else {
        hlog_warn!(target: "race-hud", "RaceManager.get_ElapsedTime not found; live feed disabled");
        return;
    };

    let hook_addr = get_elapsed_hook as *mut c_void;
    match sdk.hook(method_addr, hook_addr) {
        Some(tramp) => {
            ORIG_GET_ELAPSED.store(tramp as usize, Ordering::Release);
            LIVE_HOOK_ADDR.store(hook_addr as usize, Ordering::Release);
            hlog_info!(target: "race-hud", "Hooked Gallop.RaceManager.get_ElapsedTime (live feed)");
        }
        None => {
            hlog_warn!(target: "race-hud", "Failed to hook RaceManager.get_ElapsedTime; live feed disabled");
        }
    }
}

/// Remove both hooks (SHUTDOWN handler, UNLOADABLE contract).
pub fn uninstall() {
    let sdk = Sdk::get();

    let ri = RACE_INFO_HOOK_ADDR.swap(0, Ordering::AcqRel);
    if ri != 0 {
        sdk.unhook(ri as *mut c_void);
    }
    ORIG_GET_RACE_TRACK_ID.store(0, Ordering::Release);

    let live = LIVE_HOOK_ADDR.swap(0, Ordering::AcqRel);
    if live != 0 {
        sdk.unhook(live as *mut c_void);
    }
    ORIG_GET_ELAPSED.store(0, Ordering::Release);

    crate::state::clear_all();
    hlog_info!(target: "race-hud", "Hooks removed");
}
