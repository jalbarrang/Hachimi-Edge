//! Runtime IL2CPP class/field diagnostics.
//! Dumps field and method info for career-related classes to the log.

use std::ffi::{c_void, CStr};

use crate::vtable::vt;

/// Minimal FieldInfo layout for reading field names and types.
#[repr(C)]
struct FieldInfoCompat {
    name: *const std::ffi::c_char,
    type_: *const c_void, // Il2CppType*
}

/// Minimal MethodInfo layout (same as in hooks.rs).
#[repr(C)]
struct MethodInfoCompat {
    method_pointer: usize,
    virtual_method_pointer: usize,
    invoker_method: usize,
    name: *const std::ffi::c_char,
    klass: *mut c_void,
    return_type: *const c_void,
    parameters: *mut c_void,
    _union1: usize,
    _union2: usize,
    token: u32,
    flags: u16,
    iflags: u16,
    slot: u16,
    parameters_count: u8,
}

/// Resolved IL2CPP runtime functions for type introspection.
struct TypeIntrospection {
    type_get_name: unsafe extern "C" fn(type_: *const c_void) -> *mut std::ffi::c_char,
    class_get_name: unsafe extern "C" fn(klass: *mut c_void) -> *const std::ffi::c_char,
    class_get_fields: unsafe extern "C" fn(klass: *mut c_void, iter: *mut *mut c_void) -> *mut c_void,
    il2cpp_free: unsafe extern "C" fn(ptr: *mut c_void),
}

impl TypeIntrospection {
    /// Resolve IL2CPP runtime functions via il2cpp_resolve_symbol.
    /// Returns None if any symbol fails to resolve.
    fn resolve() -> Option<Self> {
        let vt = vt();
        unsafe {
            let type_get_name = (vt.il2cpp_resolve_symbol)(b"il2cpp_type_get_name\0".as_ptr().cast());
            let class_get_name = (vt.il2cpp_resolve_symbol)(b"il2cpp_class_get_name\0".as_ptr().cast());
            let class_get_fields = (vt.il2cpp_resolve_symbol)(b"il2cpp_class_get_fields\0".as_ptr().cast());

            if type_get_name.is_null() || class_get_name.is_null() || class_get_fields.is_null() {
                hlog_warn!("Failed to resolve type introspection symbols: type_get_name={:?} class_get_name={:?} class_get_fields={:?}",
                    type_get_name, class_get_name, class_get_fields);
                return None;
            }

            let free_fn = (vt.il2cpp_resolve_symbol)(b"il2cpp_free\0".as_ptr().cast());
            if free_fn.is_null() {
                hlog_warn!("Failed to resolve il2cpp_free");
                return None;
            }

            Some(Self {
                type_get_name: std::mem::transmute(type_get_name),
                class_get_name: std::mem::transmute(class_get_name),
                class_get_fields: std::mem::transmute(class_get_fields),
                il2cpp_free: std::mem::transmute(free_fn),
            })
        }
    }

    /// Get the name of an Il2CppType. Returns "?" on failure.
    fn type_name(&self, type_ptr: *const c_void) -> String {
        if type_ptr.is_null() {
            return "void".to_string();
        }
        unsafe {
            let name_ptr = (self.type_get_name)(type_ptr);
            if name_ptr.is_null() {
                return "?".to_string();
            }
            let name = CStr::from_ptr(name_ptr).to_str().unwrap_or("?").to_string();
            // il2cpp_type_get_name returns an allocated string, free it
            (self.il2cpp_free)(name_ptr as *mut c_void);
            name
        }
    }
}

/// Classes to probe for career/training state.
const PROBE_CLASSES: &[(&[u8], &[u8], &[u8])] = &[
    // (assembly, namespace, class)
    (b"umamusume.dll\0", b"Gallop\0", b"SingleModeMainViewController\0"),
    (b"umamusume.dll\0", b"Gallop\0", b"TrainingView\0"),
    (b"umamusume.dll\0", b"Gallop\0", b"TrainingController\0"),
    (b"umamusume.dll\0", b"Gallop\0", b"SingleModeChara\0"),
    (b"umamusume.dll\0", b"Gallop\0", b"SingleModeHomeInfo\0"),
    (b"umamusume.dll\0", b"Gallop\0", b"SingleModeCommandInfo\0"),
    (b"umamusume.dll\0", b"Gallop\0", b"TrainingLevelInfo\0"),
    (b"umamusume.dll\0", b"Gallop\0", b"WorkSingleModeData\0"),
    (b"umamusume.dll\0", b"Gallop\0", b"WorkSingleModeCharaData\0"),
    (b"umamusume.dll\0", b"Gallop\0", b"WorkSingleModeHomeInfo\0"),
    // Manager / controller singletons that might hold WorkSingleMode* references
    (b"umamusume.dll\0", b"Gallop\0", b"SingleModeSceneController\0"),
    (b"umamusume.dll\0", b"Gallop\0", b"GameSystem\0"),
    (b"umamusume.dll\0", b"Gallop\0", b"GameManager\0"),
    (b"umamusume.dll\0", b"Gallop\0", b"SingleModeManager\0"),
    (b"umamusume.dll\0", b"Gallop\0", b"SingleModeWorkDataManager\0"),
    (b"umamusume.dll\0", b"Gallop\0", b"WorkDataManager\0"),
    (b"umamusume.dll\0", b"Gallop\0", b"WorkManager\0"),
    (b"umamusume.dll\0", b"Gallop\0", b"ViewManager\0"),
    (b"umamusume.dll\0", b"Gallop\0", b"SceneManager\0"),
    (b"umamusume.dll\0", b"Gallop\0", b"UIManager\0"),
    (b"umamusume.dll\0", b"Gallop\0", b"MasterDataManager\0"),
    (b"umamusume.dll\0", b"Gallop\0", b"SingleModeGameSystem\0"),
    (b"umamusume.dll\0", b"Gallop\0", b"SingleModeContext\0"),
    (b"umamusume.dll\0", b"Gallop\0", b"SingleModeDataManager\0"),
];

/// Known field names worth probing.
/// The plugin API does not expose field iteration, so this is a targeted probe.
const PROBE_FIELD_NAMES: &[&[u8]] = &[
    // SingleModeMainViewController likely fields
    b"_instance\0",
    b"_commandId\0",
    b"_commandType\0",
    b"_currentCommandId\0",
    b"_trainingCommandId\0",
    b"_selectedCommandId\0",
    b"_singleModeData\0",
    b"_singleModeCharaData\0",
    b"_trainingLevelDic\0",
    b"_trainingPartnerInfoArray\0",
    b"_turnInfo\0",
    b"_currentTurn\0",
    b"_turn\0",
    // SingleModeChara / HomeInfo / CommandInfo fields
    b"turn\0",
    b"command_id\0",
    b"command_type\0",
    b"level\0",
    b"training_level_info_array\0",
    b"command_info_array\0",
    b"disable_command_id_array\0",
    b"training_partner_array\0",
    b"failure_rate\0",
    b"chara_id\0",
    b"scenario_id\0",
    b"speed\0",
    b"stamina\0",
    b"power\0",
    b"guts\0",
    b"wiz\0",
    b"skill_point\0",
    b"vital\0",
    b"max_vital\0",
    b"motivation\0",
    b"fans\0",
    b"is_playing\0",
    // Manager fields that might hold WorkSingleMode* references
    b"_data\0",
    b"_workData\0",
    b"_singleModeWorkData\0",
    b"_workSingleModeData\0",
    b"_singleModeData\0",
    b"_charaData\0",
    b"_workCharaData\0",
    b"_homeInfo\0",
    b"_workHomeInfo\0",
    b"_mainViewController\0",
    b"_controller\0",
    b"_viewController\0",
    b"_model\0",
    b"_context\0",
    b"_currentData\0",
    // Common property backing fields
    b"<SelectedTrainingCommandId>k__BackingField\0",
    b"<TrainingCommandId>k__BackingField\0",
    b"<IsInTraining>k__BackingField\0",
    b"<TrainingStatus>k__BackingField\0",
    b"<TrainingLevel>k__BackingField\0",
    b"<TrainingRank>k__BackingField\0",
    b"<Instance>k__BackingField\0",
    b"<Data>k__BackingField\0",
    b"<CurrentData>k__BackingField\0",
    b"<WorkData>k__BackingField\0",
    b"<Character>k__BackingField\0",
    b"<HomeInfo>k__BackingField\0",
];

/// Dump all methods on a class to the log, including return type names.
fn dump_methods(class_label: &str, klass: *mut c_void, introspect: Option<&TypeIntrospection>) {
    let vt = vt();
    let mut iter: *mut c_void = std::ptr::null_mut();
    let mut count = 0u32;

    hlog_info!("  Methods on {}:", class_label);
    loop {
        // SAFETY: Plugin FFI interop with Hachimi vtable
        let method = unsafe { (vt.il2cpp_class_get_methods)(klass.cast(), &mut iter) };
        if method.is_null() {
            break;
        }

        // SAFETY: MethodInfoCompat matches the leading MethodInfo fields used here.
        unsafe {
            let mi = &*(method as *const MethodInfoCompat);
            if !mi.name.is_null() {
                if let Ok(name) = CStr::from_ptr(mi.name).to_str() {
                    let ret_type = introspect
                        .map(|i| i.type_name(mi.return_type))
                        .unwrap_or_else(|| "?".to_string());
                    hlog_info!("    method: {} {}({} args)", ret_type, name, mi.parameters_count);
                }
            }
        }

        count += 1;
        if count > 300 {
            hlog_warn!("    ... truncated at 300 methods");
            break;
        }
    }

    hlog_info!("  {} total methods on {}", count, class_label);
}

/// Dump ALL fields on a class via il2cpp_class_get_fields iteration.
fn dump_all_fields(class_label: &str, klass: *mut c_void, introspect: &TypeIntrospection) {
    let mut iter: *mut c_void = std::ptr::null_mut();
    let mut count = 0u32;

    hlog_info!("  All fields on {}:", class_label);
    loop {
        let field = unsafe { (introspect.class_get_fields)(klass, &mut iter) };
        if field.is_null() {
            break;
        }

        unsafe {
            let fi = &*(field as *const FieldInfoCompat);
            let field_name = if fi.name.is_null() {
                "?"
            } else {
                CStr::from_ptr(fi.name).to_str().unwrap_or("?")
            };
            let type_name = introspect.type_name(fi.type_);
            hlog_info!("    field: {} {} (FieldInfo={:?})", type_name, field_name, field);
        }

        count += 1;
        if count > 200 {
            hlog_warn!("    ... truncated at 200 fields");
            break;
        }
    }

    hlog_info!("  {} total fields on {}", count, class_label);
}

fn field_name_from_info(field: *mut c_void) -> &'static str {
    // SAFETY: FieldInfoCompat matches the leading FieldInfo field used here.
    unsafe {
        let fi = &*(field as *const FieldInfoCompat);
        if fi.name.is_null() {
            return "?";
        }
        CStr::from_ptr(fi.name).to_str().unwrap_or("?")
    }
}

fn probe_fields(class_label: &str, klass: *mut c_void) {
    let vt = vt();
    let mut found = 0u32;

    hlog_info!("  Probing fields on {}:", class_label);
    for name_bytes in PROBE_FIELD_NAMES {
        // SAFETY: Plugin FFI interop with Hachimi vtable
        let field = unsafe { (vt.il2cpp_get_field_from_name)(klass.cast(), name_bytes.as_ptr().cast()) };
        if !field.is_null() {
            let field_name = field_name_from_info(field.cast());
            hlog_info!("    ✓ field found: {} (FieldInfo={:?})", field_name, field);
            found += 1;
        }
    }

    hlog_info!(
        "  {}/{} probe fields found on {}",
        found,
        PROBE_FIELD_NAMES.len(),
        class_label
    );
}

/// Check if a class has a singleton-like `_instance` static field and try to get the instance.
fn probe_singleton(class_label: &str, klass: *mut c_void) {
    let vt = vt();
    // SAFETY: Plugin FFI interop with Hachimi vtable
    let instance = unsafe { (vt.il2cpp_get_singleton_like_instance)(klass.cast()) };
    if !instance.is_null() {
        hlog_info!("  ★ {} has LIVE singleton instance at {:?}", class_label, instance);
    } else {
        hlog_info!("  {} — no singleton instance (null or no _instance field)", class_label);
    }
}

/// Deep-dive dump for a specific class: full field iteration + methods with return types.
fn deep_dive_class(label: &str, klass: *mut c_void, introspect: &TypeIntrospection) {
    hlog_info!("╔══ DEEP DIVE: {} (klass={:?}) ══╗", label, klass);
    probe_singleton(label, klass);
    dump_all_fields(label, klass, introspect);
    dump_methods(label, klass, Some(introspect));
    hlog_info!("╚══ END DEEP DIVE: {} ══╝", label);
}

/// Classes that get the full deep-dive treatment (field iteration + return types).
const DEEP_DIVE_CLASSES: &[(&[u8], &[u8], &[u8])] = &[
    (b"umamusume.dll\0", b"Gallop\0", b"WorkDataManager\0"),
    (b"umamusume.dll\0", b"Gallop\0", b"WorkSingleModeData\0"),
    (b"umamusume.dll\0", b"Gallop\0", b"WorkSingleModeCharaData\0"),
    (b"umamusume.dll\0", b"Gallop\0", b"WorkSingleModeHomeInfo\0"),
    (b"umamusume.dll\0", b"Gallop\0", b"SingleModeManager\0"),
    (b"umamusume.dll\0", b"Gallop\0", b"SingleModeWorkDataManager\0"),
    (b"umamusume.dll\0", b"Gallop\0", b"SingleModeContext\0"),
    (b"umamusume.dll\0", b"Gallop\0", b"SingleModeDataManager\0"),
    (b"umamusume.dll\0", b"Gallop\0", b"AcquiredSkill\0"),
    (b"umamusume.dll\0", b"Gallop\0", b"SkillTips\0"),
    (b"umamusume.dll\0", b"Gallop\0", b"SkillData\0"),
    (b"umamusume.dll\0", b"Gallop\0", b"SkillManager\0"),
    (b"umamusume.dll\0", b"Gallop\0", b"SkillBase\0"),
];

/// Run the full diagnostic dump. Call from a menu button click.
pub fn run_diagnostics() {
    let vt = vt();

    hlog_info!("=== TRAINING TRACKER DIAGNOSTICS START ===");

    // Resolve type introspection functions for enhanced dumps
    let introspect = TypeIntrospection::resolve();
    if introspect.is_some() {
        hlog_info!("Type introspection: AVAILABLE (will show return types and all fields)");
    } else {
        hlog_warn!("Type introspection: UNAVAILABLE (fallback to basic dump)");
    }

    // Phase 1: Deep-dive on key classes (full field iteration + return types)
    if let Some(ref intro) = introspect {
        hlog_info!("\n=== PHASE 1: DEEP DIVE ON KEY CLASSES ===");
        for &(asm, ns, class) in DEEP_DIVE_CLASSES {
            let class_name = CStr::from_bytes_with_nul(class)
                .map(|c| c.to_str().unwrap_or("?"))
                .unwrap_or("?");

            let image = unsafe { (vt.il2cpp_get_assembly_image)(asm.as_ptr().cast()) };
            if image.is_null() { continue; }

            let klass = unsafe { (vt.il2cpp_get_class)(image, ns.as_ptr().cast(), class.as_ptr().cast()) };
            if klass.is_null() {
                hlog_info!("[DEEP] {} — class NOT FOUND", class_name);
                continue;
            }

            deep_dive_class(class_name, klass.cast(), intro);
        }
    }

    // Phase 2: Broad scan of all probe classes (lighter: singleton + probe fields + basic methods)
    hlog_info!("\n=== PHASE 2: BROAD CLASS SCAN ===");
    for &(asm, ns, class) in PROBE_CLASSES {
        let class_name = CStr::from_bytes_with_nul(class)
            .map(|c| c.to_str().unwrap_or("?"))
            .unwrap_or("?");

        let image = unsafe { (vt.il2cpp_get_assembly_image)(asm.as_ptr().cast()) };
        if image.is_null() {
            hlog_warn!("Assembly not found for {}", class_name);
            continue;
        }

        let klass = unsafe { (vt.il2cpp_get_class)(image, ns.as_ptr().cast(), class.as_ptr().cast()) };
        if klass.is_null() {
            hlog_info!("[{}] — class NOT FOUND", class_name);
            continue;
        }

        hlog_info!("[{}] — class FOUND at {:?}", class_name, klass);
        probe_singleton(class_name, klass.cast());
        probe_fields(class_name, klass.cast());
        dump_methods(class_name, klass.cast(), introspect.as_ref());
        hlog_info!("---");
    }

    hlog_info!("=== TRAINING TRACKER DIAGNOSTICS END ===");
}

/// Focused dump of skill-related classes for wiring up the skills panel.
const SKILL_CLASSES: &[(&[u8], &[u8], &[u8])] = &[
    (b"umamusume.dll\0", b"Gallop\0", b"AcquiredSkill\0"),
    (b"umamusume.dll\0", b"Gallop\0", b"SkillTips\0"),
    (b"umamusume.dll\0", b"Gallop\0", b"SkillData\0"),
    (b"umamusume.dll\0", b"Gallop\0", b"SkillBase\0"),
    (b"umamusume.dll\0", b"Gallop\0", b"SkillManager\0"),
    (b"umamusume.dll\0", b"Gallop\0", b"SingleModeAcquiredSkill\0"),
    (b"umamusume.dll\0", b"Gallop\0", b"WorkSingleModeSkillData\0"),
    (b"umamusume.dll\0", b"Gallop\0", b"MasterSkillData\0"),
    (b"umamusume.dll\0", b"Gallop\0", b"SkillDataManager\0"),
    (b"umamusume.dll\0", b"Gallop\0", b"WorkSkillData\0"),
    // Friendship / bond / evaluation classes
    (b"umamusume.dll\0", b"Gallop\0", b"EvaluationInfo\0"),
    (b"umamusume.dll\0", b"Gallop\0", b"WorkSingleModeEvaluationInfo\0"),
    (b"umamusume.dll\0", b"Gallop\0", b"SingleModeEvaluationInfo\0"),
    (b"umamusume.dll\0", b"Gallop\0", b"WorkSupportCardData\0"),
    (b"umamusume.dll\0", b"Gallop\0", b"SingleModeSupportCard\0"),
    (b"umamusume.dll\0", b"Gallop\0", b"WorkSingleModeSupportCard\0"),
    (b"umamusume.dll\0", b"Gallop\0", b"SingleModeEvaluation\0"),
    (b"umamusume.dll\0", b"Gallop\0", b"MasterSingleModeEvaluation\0"),
    (b"umamusume.dll\0", b"Gallop\0", b"TrainingPartnerInfo\0"),
    // Skill cost / master data utilities
    (b"umamusume.dll\0", b"Gallop\0", b"MasterDataUtil\0"),
    (b"umamusume.dll\0", b"Gallop\0", b"MasterSingleModeSkillNeedPoint\0"),
    // Master data instance holders
    (b"umamusume.dll\0", b"Gallop\0", b"MasterDataManager\0"),
    (b"umamusume.dll\0", b"Gallop\0", b"MasterHolder\0"),
    (b"umamusume.dll\0", b"Gallop\0", b"MasterBanker\0"),
    (b"umamusume.dll\0", b"Gallop\0", b"MasterSingleModeDatabase\0"),
    (b"umamusume.dll\0", b"Gallop\0", b"MasterCardDatabase\0"),
];

pub fn dump_skill_classes() {
    let vt = vt();
    hlog_info!("=== SKILL CLASS DIAGNOSTICS START ===");

    let introspect = TypeIntrospection::resolve();
    if introspect.is_none() {
        hlog_warn!("Type introspection unavailable — field types won't be shown");
    }

    for &(asm, ns, class) in SKILL_CLASSES {
        let class_name = CStr::from_bytes_with_nul(class)
            .map(|c| c.to_str().unwrap_or("?"))
            .unwrap_or("?");

        let image = unsafe { (vt.il2cpp_get_assembly_image)(asm.as_ptr().cast()) };
        if image.is_null() { continue; }

        let klass = unsafe { (vt.il2cpp_get_class)(image, ns.as_ptr().cast(), class.as_ptr().cast()) };
        if klass.is_null() {
            hlog_info!("[SKILL] {} — NOT FOUND", class_name);
            continue;
        }

        if let Some(ref intro) = introspect {
            deep_dive_class(class_name, klass.cast(), intro);
        } else {
            hlog_info!("[SKILL] {} — FOUND at {:?}", class_name, klass);
            dump_methods(class_name, klass.cast(), None);
        }
    }

    // Phase 2: Introspect the actual AcquiredSkill class from a live list element
    hlog_info!("\n=== LIVE ACQUIRED SKILL INTROSPECTION ===");
    if let Some((list_ptr, count)) = crate::memory_reader::read_acquired_skill_list() {
        hlog_info!("_acquiredSkillList: {} items (list={:?})", count, list_ptr);

        if count > 0 {
            unsafe {
                // Get the list's inflated class and call get_Item(0)
                let list_klass = *(list_ptr as *const *mut c_void);
                let m_get_item =
                    (vt.il2cpp_get_method)(list_klass.cast(), b"get_Item\0".as_ptr().cast(), 1);
                if !m_get_item.is_null() {
                    // call_obj_with_i32 equivalent inline
                    let mi = m_get_item as *const c_void;
                    let fp: extern "C" fn(*mut c_void, i32, *const c_void) -> *mut c_void =
                        std::mem::transmute(*(mi as *const usize));
                    let first_item = fp(list_ptr, 0, mi);

                    if !first_item.is_null() {
                        let item_klass = *(first_item as *const *mut c_void);
                        hlog_info!("First AcquiredSkill element: obj={:?} klass={:?}", first_item, item_klass);

                        if let Some(ref intro) = introspect {
                            let class_name_ptr = (intro.class_get_name)(item_klass);
                            let class_name = if !class_name_ptr.is_null() {
                                CStr::from_ptr(class_name_ptr).to_str().unwrap_or("?")
                            } else {
                                "?"
                            };
                            hlog_info!("AcquiredSkill runtime class name: {}", class_name);
                            deep_dive_class(class_name, item_klass, intro);

                            // Walk parent class chain to find inherited fields
                            let il2cpp_class_get_parent = (vt.il2cpp_resolve_symbol)(
                                b"il2cpp_class_get_parent\0".as_ptr().cast(),
                            );
                            if !il2cpp_class_get_parent.is_null() {
                                let get_parent: extern "C" fn(*mut c_void) -> *mut c_void =
                                    std::mem::transmute(il2cpp_class_get_parent);
                                let mut klass = item_klass;
                                for depth in 0..5 {
                                    let parent = get_parent(klass);
                                    if parent.is_null() || parent == klass {
                                        break;
                                    }
                                    let pname_ptr = (intro.class_get_name)(parent);
                                    let pname = if !pname_ptr.is_null() {
                                        CStr::from_ptr(pname_ptr).to_str().unwrap_or("?")
                                    } else {
                                        "?"
                                    };
                                    hlog_info!("  Parent[{}]: {} (klass={:?})", depth, pname, parent);
                                    deep_dive_class(&format!("parent::{}", pname), parent, intro);
                                    klass = parent;
                                }
                            }
                        } else {
                            dump_methods("AcquiredSkill(runtime)", item_klass, None);
                        }
                    } else {
                        hlog_info!("get_Item(0) returned null");
                    }
                } else {
                    hlog_warn!("get_Item not found on list class");
                }
            }
        }
    } else {
        hlog_info!("No acquired skill list available (not in a career or tracking not started)");
    }

    // Phase 3: Probe for nested classes that weren't found as top-level
    hlog_info!("\n=== NESTED CLASS PROBE ===");
    let image = unsafe { (vt.il2cpp_get_assembly_image)(b"umamusume.dll\0".as_ptr().cast()) };
    if !image.is_null() {
        // AcquiredSkill / SkillTips might be nested inside these parent classes
        let parent_candidates: &[(&[u8], &str)] = &[
            (b"WorkSingleModeCharaData\0", "WorkSingleModeCharaData"),
            (b"SingleModeChara\0", "SingleModeChara"),
            (b"WorkSingleModeData\0", "WorkSingleModeData"),
            (b"SingleModeHomeInfo\0", "SingleModeHomeInfo"),
            (b"MasterSkillData\0", "MasterSkillData"),
            (b"WorkSkillData\0", "WorkSkillData"),
            (b"WorkSingleModeHomeInfo\0", "WorkSingleModeHomeInfo"),
            (b"WorkSupportCardData\0", "WorkSupportCardData"),
            (b"MasterSingleModeSkillNeedPoint\0", "MasterSingleModeSkillNeedPoint"),
        ];
        let nested_names: &[(&[u8], &str)] = &[
            (b"AcquiredSkill\0", "AcquiredSkill"),
            (b"SkillTips\0", "SkillTips"),
            (b"SkillData\0", "SkillData"),
            (b"Skill\0", "Skill"),
            // Friendship / evaluation nested classes
            (b"EvaluationInfo\0", "EvaluationInfo"),
            (b"Evaluation\0", "Evaluation"),
            (b"SupportCard\0", "SupportCard"),
            (b"TrainingPartner\0", "TrainingPartner"),
            (b"SingleModeSkillNeedPoint\0", "SingleModeSkillNeedPoint"),
        ];

        for &(parent_bytes, parent_label) in parent_candidates {
            let parent_klass = unsafe {
                (vt.il2cpp_get_class)(image, b"Gallop\0".as_ptr().cast(), parent_bytes.as_ptr().cast())
            };
            if parent_klass.is_null() {
                continue;
            }

            for &(nested_bytes, nested_label) in nested_names {
                let nested = unsafe {
                    (vt.il2cpp_find_nested_class)(parent_klass, nested_bytes.as_ptr().cast())
                };
                if !nested.is_null() {
                    hlog_info!("  \u{2705} FOUND nested: {}.{} at {:?}", parent_label, nested_label, nested);
                    if let Some(ref intro) = introspect {
                        let label = format!("{}.{}", parent_label, nested_label);
                        deep_dive_class(&label, nested.cast(), intro);
                    }
                }
            }
        }
    }

    hlog_info!("=== SKILL CLASS DIAGNOSTICS END ===");
}
