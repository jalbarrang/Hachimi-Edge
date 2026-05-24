//! Skill shop reconstruction from game memory.
//!
//! Reads `_skillTipsList` from `WorkSingleModeCharaData` and resolves
//! skill names and costs via the game's own master data accessor chain:
//!
//! ```text
//! MasterDataManager (singleton)
//!   → get_masterSkillData() → MasterSkillData
//!     → GetListWithGroupIdOrderByIdAsc(group_id) → List<SkillData>
//!       → each: .Id, .Rarity, get_Name(), .GradeValue
//!   → get_masterSingleModeSkillNeedPoint() → MasterSingleModeSkillNeedPoint
//!     → Get(skill_id) → SingleModeSkillNeedPoint
//!       → .NeedSkillPoint
//! ```
//!
//! Results are cached and only refreshed on explicit user request (Refresh button).

use std::ffi::c_void;
use std::sync::{Mutex, OnceLock};

use crate::memory_reader;
use crate::vtable::vt;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// A skill available in the shop, resolved from tips + master data.
#[derive(Debug, Clone)]
pub struct SkillShopEntry {
    #[allow(dead_code)] // Available for future use (e.g. skill detail lookup)
    pub skill_id: i32,
    pub group_id: i32,
    pub rarity: i32,
    pub hint_level: i32,
    pub name: String,
    pub base_cost: i32,
    pub is_learned: bool,
}

// ---------------------------------------------------------------------------
// Cache
// ---------------------------------------------------------------------------

static SHOP_CACHE: Mutex<Vec<SkillShopEntry>> = Mutex::new(Vec::new());

pub fn get_cached() -> Vec<SkillShopEntry> {
    SHOP_CACHE.lock().ok().map(|g| g.clone()).unwrap_or_default()
}

pub fn refresh() {
    let entries = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(read_skill_shop)) {
        Ok(v) => v,
        Err(_) => { hlog_error!("skill shop refresh PANICKED"); Vec::new() }
    };
    if let Ok(mut guard) = SHOP_CACHE.lock() {
        *guard = entries;
    }
}

// ---------------------------------------------------------------------------
// Resolved IL2CPP pointers (all from one-time resolution)
// ---------------------------------------------------------------------------

struct Resolved {
    // MasterDataManager (singleton)
    mdm_klass: *mut c_void,
    m_get_master_skill_data: *const c_void,      // → MasterSkillData
    m_get_skill_need_point: *const c_void,        // → MasterSingleModeSkillNeedPoint

    // MasterSkillData
    m_msd_get_list_by_group: *const c_void,       // GetListWithGroupIdOrderByIdAsc(int)

    // MasterSingleModeSkillNeedPoint
    m_snp_get: *const c_void,                     // Get(int) → SingleModeSkillNeedPoint

    // MasterSkillData.SkillData fields/methods
    f_sd_id: *mut c_void,
    f_sd_rarity: *mut c_void,
    f_sd_grade_value: *mut c_void,
    m_sd_get_name: *const c_void,

    // SingleModeSkillNeedPoint fields
    f_snp_need_skill_point: *mut c_void,

    // SkillTips backing fields
    f_tips_group_id: *mut c_void,
    f_tips_rarity: *mut c_void,
    f_tips_level: *mut c_void,

    // WorkSingleModeCharaData skill point
    f_skill_point: *mut c_void,
}

unsafe impl Send for Resolved {}
unsafe impl Sync for Resolved {}

static RESOLVED: OnceLock<Resolved> = OnceLock::new();

// ---------------------------------------------------------------------------
// IL2CPP helpers
// ---------------------------------------------------------------------------

#[inline]
unsafe fn mptr(mi: *const c_void) -> usize { unsafe { *(mi as *const usize) } }

#[inline]
unsafe fn call_obj(this: *mut c_void, mi: *const c_void) -> *mut c_void {
    let f: extern "C" fn(*mut c_void, *const c_void) -> *mut c_void = unsafe { std::mem::transmute(mptr(mi)) };
    f(this, mi)
}

#[inline]
unsafe fn call_i32(this: *mut c_void, mi: *const c_void) -> i32 {
    let f: extern "C" fn(*mut c_void, *const c_void) -> i32 = unsafe { std::mem::transmute(mptr(mi)) };
    f(this, mi)
}

#[inline]
unsafe fn call_obj_i32(this: *mut c_void, mi: *const c_void, arg: i32) -> *mut c_void {
    let f: extern "C" fn(*mut c_void, i32, *const c_void) -> *mut c_void = unsafe { std::mem::transmute(mptr(mi)) };
    f(this, arg, mi)
}

unsafe fn read_string(s: *mut c_void) -> Option<String> {
    unsafe {
        if s.is_null() { return None; }
        let len = *(s.byte_add(0x10) as *const i32);
        if len <= 0 || len > 4096 { return None; }
        String::from_utf16(std::slice::from_raw_parts(s.byte_add(0x14) as *const u16, len as usize)).ok()
    }
}

unsafe fn read_field_i32(obj: *mut c_void, field: *mut c_void) -> i32 {
    let mut v: i32 = 0;
    unsafe { (vt().il2cpp_get_field_value)(obj.cast(), field.cast(), &mut v as *mut _ as *mut c_void) };
    v
}

unsafe fn decrypt_obscured_int(obj: *mut c_void, field: *mut c_void) -> i32 {
    let mut buf = [0u8; 16];
    unsafe { (vt().il2cpp_get_field_value)(obj.cast(), field.cast(), buf.as_mut_ptr() as *mut c_void) };
    let key = i32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]);
    let val = i32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]);
    val ^ key
}

// ---------------------------------------------------------------------------
// Resolution
// ---------------------------------------------------------------------------

fn ensure_resolved() -> bool {
    if RESOLVED.get().is_some() { return true; }
    match try_resolve() {
        Ok(r) => { let _ = RESOLVED.set(r); true }
        Err(e) => { hlog_error!("Skill shop resolution failed: {}", e); false }
    }
}

macro_rules! resolve {
    (class $img:expr, $ns:literal, $name:literal) => {{
        let k = unsafe { (vt().il2cpp_get_class)($img, concat!($ns, "\0").as_ptr().cast(), concat!($name, "\0").as_ptr().cast()) };
        if k.is_null() { return Err(concat!($name, " not found")); }
        k
    }};
    (nested $parent:expr, $name:literal) => {{
        let k = unsafe { (vt().il2cpp_find_nested_class)($parent, concat!($name, "\0").as_ptr().cast()) };
        if k.is_null() { return Err(concat!("nested ", $name, " not found")); }
        k
    }};
    (method $klass:expr, $name:literal, $args:expr) => {{
        let m = unsafe { (vt().il2cpp_get_method)($klass, concat!($name, "\0").as_ptr().cast(), $args) };
        if m.is_null() { return Err(concat!($name, " method not found")); }
        m as *const c_void
    }};
    (field $klass:expr, $name:literal) => {{
        let f = unsafe { (vt().il2cpp_get_field_from_name)($klass, concat!($name, "\0").as_ptr().cast()) };
        if f.is_null() { return Err(concat!($name, " field not found")); }
        f as *mut c_void
    }};
    (field_opt $klass:expr, $name:literal) => {{
        unsafe { (vt().il2cpp_get_field_from_name)($klass, concat!($name, "\0").as_ptr().cast()) as *mut c_void }
    }};
}

fn try_resolve() -> Result<Resolved, &'static str> {
    let img = unsafe { (vt().il2cpp_get_assembly_image)(b"umamusume.dll\0".as_ptr().cast()) };
    if img.is_null() { return Err("umamusume.dll not found"); }

    // MasterDataManager (singleton hub)
    let mdm = resolve!(class img, "Gallop", "MasterDataManager");
    let m_get_msd = resolve!(method mdm, "get_masterSkillData", 0);
    let m_get_snp = resolve!(method mdm, "get_masterSingleModeSkillNeedPoint", 0);

    // MasterSkillData
    let msd_klass = resolve!(class img, "Gallop", "MasterSkillData");
    let m_msd_get_list = resolve!(method msd_klass, "GetListWithGroupIdOrderByIdAsc", 1);

    // MasterSkillData.SkillData (nested)
    let sd_klass = resolve!(nested msd_klass, "SkillData");
    let f_sd_id = resolve!(field sd_klass, "Id");
    let f_sd_rarity = resolve!(field sd_klass, "Rarity");
    let f_sd_grade = resolve!(field sd_klass, "GradeValue");
    let m_sd_name = resolve!(method sd_klass, "get_Name", 0);

    // MasterSingleModeSkillNeedPoint
    let snp_klass = resolve!(class img, "Gallop", "MasterSingleModeSkillNeedPoint");
    let m_snp_get = resolve!(method snp_klass, "Get", 1);
    let snp_row = resolve!(nested snp_klass, "SingleModeSkillNeedPoint");
    let f_snp_cost = resolve!(field snp_row, "NeedSkillPoint");

    // SkillTips
    let wsmcd = resolve!(class img, "Gallop", "WorkSingleModeCharaData");
    let tips = resolve!(nested wsmcd, "SkillTips");
    let f_gid = resolve!(field tips, "<GroupId>k__BackingField");
    let f_rar = resolve!(field tips, "<Rarity>k__BackingField");
    let f_lvl = resolve!(field tips, "<Level>k__BackingField");

    // SkillPoint
    let f_sp = resolve!(field_opt wsmcd, "<SkillPoint>k__BackingField");

    hlog_info!("Skill shop: full IL2CPP chain resolved (MasterDataManager → SkillData + NeedPoint)");
    Ok(Resolved {
        mdm_klass: mdm as _,
        m_get_master_skill_data: m_get_msd,
        m_get_skill_need_point: m_get_snp,
        m_msd_get_list_by_group: m_msd_get_list,
        m_snp_get,
        f_sd_id, f_sd_rarity, f_sd_grade_value: f_sd_grade, m_sd_get_name: m_sd_name,
        f_snp_need_skill_point: f_snp_cost,
        f_tips_group_id: f_gid, f_tips_rarity: f_rar, f_tips_level: f_lvl,
        f_skill_point: f_sp,
    })
}

// ---------------------------------------------------------------------------
// Public: read current SP
// ---------------------------------------------------------------------------

pub fn read_skill_points() -> Option<i32> {
    let r = RESOLVED.get()?;
    if r.f_skill_point.is_null() { return None; }
    let chara = memory_reader::get_chara_ptr()?;
    Some(unsafe { decrypt_obscured_int(chara, r.f_skill_point) })
}

// ---------------------------------------------------------------------------
// Core read
// ---------------------------------------------------------------------------

fn read_skill_shop() -> Vec<SkillShopEntry> {
    if !ensure_resolved() { return Vec::new(); }
    let r = match RESOLVED.get() { Some(r) => r, None => return Vec::new() };

    let chara = match memory_reader::get_chara_ptr() { Some(c) => c, None => return Vec::new() };

    // Get MasterDataManager singleton
    let mdm = unsafe { (vt().il2cpp_get_singleton_like_instance)(r.mdm_klass.cast()) };
    if mdm.is_null() { hlog_warn!("MasterDataManager singleton is null"); return Vec::new(); }
    let mdm = mdm as *mut c_void;

    // Get master data table instances
    let msd = unsafe { call_obj(mdm, r.m_get_master_skill_data) };
    let snp = unsafe { call_obj(mdm, r.m_get_skill_need_point) };
    if msd.is_null() { hlog_warn!("MasterSkillData is null"); return Vec::new(); }

    // Learned IDs
    let learned = memory_reader::read_acquired_skills();
    let learned_ids: Vec<i32> = learned.iter().map(|s| s.master_id).collect();

    // Read tips
    let (list_ptr, count, m_get_item) = match unsafe { memory_reader::read_list_field(chara, b"_skillTipsList\0") } {
        Some(v) => v,
        None => return Vec::new(),
    };
    if count <= 0 || count > 500 { return Vec::new(); }

    let mut entries = Vec::new();

    for i in 0..count {
        let item = unsafe { call_obj_i32(list_ptr, m_get_item, i) };
        if item.is_null() { continue; }

        let group_id = unsafe { decrypt_obscured_int(item, r.f_tips_group_id) };
        let tip_rarity = unsafe { decrypt_obscured_int(item, r.f_tips_rarity) };
        let level = unsafe { decrypt_obscured_int(item, r.f_tips_level) };

        // Expand group → concrete skills via MasterSkillData
        let skill_list = unsafe { call_obj_i32(msd, r.m_msd_get_list_by_group, group_id) };
        if skill_list.is_null() { continue; }

        let list_klass = unsafe { *(skill_list as *const *mut c_void) };
        let m_cnt = unsafe { (vt().il2cpp_get_method)(list_klass, b"get_Count\0".as_ptr().cast(), 0) };
        let m_itm = unsafe { (vt().il2cpp_get_method)(list_klass, b"get_Item\0".as_ptr().cast(), 1) };
        if m_cnt.is_null() || m_itm.is_null() { continue; }

        let sk_count = unsafe { call_i32(skill_list, m_cnt as _) };

        for j in 0..sk_count.min(20) {
            let sd = unsafe { call_obj_i32(skill_list, m_itm as _, j) };
            if sd.is_null() { continue; }

            let rarity = unsafe { read_field_i32(sd, r.f_sd_rarity) };
            if rarity != tip_rarity { continue; }

            let skill_id = unsafe { read_field_i32(sd, r.f_sd_id) };
            let name = unsafe { read_string(call_obj(sd, r.m_sd_get_name)) }.unwrap_or_default();
            let _grade = unsafe { read_field_i32(sd, r.f_sd_grade_value) };

            // Look up cost
            let base_cost = if !snp.is_null() {
                let row = unsafe { call_obj_i32(snp, r.m_snp_get, skill_id) };
                if !row.is_null() {
                    unsafe { read_field_i32(row, r.f_snp_need_skill_point) }
                } else { 0 }
            } else { 0 };

            let is_learned = learned_ids.contains(&skill_id);

            entries.push(SkillShopEntry {
                skill_id, group_id, rarity, hint_level: level,
                name, base_cost, is_learned,
            });
        }
    }

    // Sort: gold first, then by name
    entries.sort_by(|a, b| b.rarity.cmp(&a.rarity).then(a.name.cmp(&b.name)));
    entries
}

// ---------------------------------------------------------------------------
// Display helpers
// ---------------------------------------------------------------------------

pub fn discount_pct(hint_level: i32, has_kiremono: bool) -> i32 {
    let base = match hint_level {
        0 => 0, 1 => 10, 2 => 20, 3 => 30, 4 => 35, _ => 40,
    };
    base + if has_kiremono { 10 } else { 0 }
}

pub fn rarity_label(rarity: i32) -> &'static str {
    match rarity {
        1 => "\u{26aa}",  // ⚪
        2 => "\u{1f31f}", // 🌟
        _ => "?",
    }
}
