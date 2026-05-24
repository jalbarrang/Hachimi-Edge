//! GUI rendering via the Hachimi plugin menu system.
//!
//! Registers a menu section and an overlay that display:
//! - Live career stats read directly from game memory (memory-read mode)
//! - Training facility visit counts from hooks (complementary)

use std::ffi::{c_void, CString};
use std::panic::{self, AssertUnwindSafe};
use std::sync::atomic::Ordering;

use hachimi_plugin_abi::vt;
use hachimi_plugin_sdk::Sdk;

use crate::memory_reader;
use crate::skill_shop;
use crate::tracker::{Facility, TRACKER};

/// Overlay font size in pixels. Stored as f32 bits in an AtomicU32.
static OVERLAY_FONT_SIZE: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

/// Default overlay font size.
const DEFAULT_FONT_SIZE: f32 = 11.0;

fn get_font_size() -> f32 {
    let bits = OVERLAY_FONT_SIZE.load(Ordering::Relaxed);
    if bits == 0 {
        DEFAULT_FONT_SIZE
    } else {
        f32::from_bits(bits)
    }
}

fn set_font_size(size: f32) {
    let clamped = size.clamp(6.0, 30.0);
    OVERLAY_FONT_SIZE.store(clamped.to_bits(), Ordering::Relaxed);
}

/// The overlay ID used during registration — must match for show/hide calls.
const OVERLAY_ID: &std::ffi::CStr = c"training_tracker_overlay";

/// Register the plugin's UI components with the Hachimi GUI.
pub fn register_ui() {
    let sdk = Sdk::get();
    let version = sdk.version();

    sdk.register_menu_section(draw_menu_section, std::ptr::null_mut());

    if sdk.register_overlay("training_tracker_overlay", draw_overlay, std::ptr::null_mut()) {
        hlog_info!(target: "training-tracker", "UI registered (menu + overlay)");
    } else {
        hlog_info!(
            target: "training-tracker",
            "UI registered (menu only, host API v{} < 3)",
            version.raw()
        );
    }
}

// ===========================================================================
// Menu section (drawn inside Hachimi menu panel)
// ===========================================================================

extern "C" fn draw_menu_section(ui: *mut c_void, _userdata: *mut c_void) {
    if panic::catch_unwind(AssertUnwindSafe(|| draw_menu_section_inner(ui))).is_err() {
        hlog_error!("draw_menu_section PANICKED");
        let vt = vt();
        // SAFETY: IL2CPP FFI call; host vtable and resolved symbols are valid for process lifetime.
        unsafe {
            (vt.gui_ui_colored_label)(ui, 255, 70, 70, 255, c"[Training Tracker: menu render error]".as_ptr());
        }
    }
}

fn draw_menu_section_inner(ui: *mut c_void) {
    let vt = vt();
    let api_version = Sdk::get().version();

    // SAFETY: IL2CPP FFI call; host vtable and resolved symbols are valid for process lifetime.
    unsafe {
        let heading = c"\u{1f3cb} Training Tracker";
        (vt.gui_ui_heading)(ui, heading.as_ptr());

        // --- Memory-read tracking controls ---
        draw_tracking_controls(ui);

        // --- Hook counts status + reset ---
        draw_hook_status(ui);

        // --- Show overlay button ---
        if api_version.supports_overlay_visibility() {
            let show_overlay = c"\u{1f4ca} Show Training Overlay";
            if (vt.gui_ui_button)(ui, show_overlay.as_ptr()) {
                (vt.gui_overlay_set_visible)(OVERLAY_ID.as_ptr(), true);
                (vt.gui_show_notification)(c"Training overlay shown".as_ptr());
            }
        }

        // --- Overlay font size ---
        if api_version.supports_font_size() {
            let label = c"Overlay font size (px):";
            (vt.gui_ui_small)(ui, label.as_ptr());

            let mut font_buf = [0u8; 8];
            let s = format!("{:.0}", get_font_size());
            let bytes = s.as_bytes();
            let len = bytes.len().min(7);
            font_buf[..len].copy_from_slice(&bytes[..len]);
            font_buf[len] = 0;
            if (vt.gui_ui_text_edit_singleline)(ui, font_buf.as_mut_ptr() as *mut std::ffi::c_char, font_buf.len()) {
                let end = font_buf.iter().position(|b| *b == 0).unwrap_or(font_buf.len());
                if let Ok(s) = std::str::from_utf8(&font_buf[..end]) {
                    if let Ok(v) = s.trim().parse::<f32>() {
                        set_font_size(v);
                    }
                }
            }
        }

        // --- Diagnostic dumps ---
        let dump = c"Dump IL2CPP Diagnostics";
        if (vt.gui_ui_small_button)(ui, dump.as_ptr()) {
            crate::diagnostics::run_diagnostics();
            (vt.gui_show_notification)(c"Diagnostics dumped to log".as_ptr());
        }
        let dump_skills = c"Dump Skill Classes";
        if (vt.gui_ui_small_button)(ui, dump_skills.as_ptr()) {
            crate::diagnostics::dump_skill_classes();
            (vt.gui_show_notification)(c"Skill class diagnostics dumped to log".as_ptr());
        }

        (vt.gui_ui_separator)(ui);
    }
}

/// Draw start/stop button and brief status in the menu.
fn draw_tracking_controls(ui: *mut c_void) {
    let vt = vt();
    let tracking = memory_reader::TRACKING.load(Ordering::Relaxed);

    if !tracking {
        let btn = c"\u{25b6} Start Memory Tracking";
        // SAFETY: Plugin FFI interop with Hachimi vtable
        if unsafe { (vt.gui_ui_button)(ui, btn.as_ptr()) } {
            match memory_reader::start_tracking() {
                // SAFETY: IL2CPP FFI call; host vtable and resolved symbols are valid for process lifetime.
                Ok(()) => unsafe {
                    (vt.gui_show_notification)(c"Memory tracking started!".as_ptr());
                },
                Err(e) => {
                    let msg = CString::new(format!("Failed: {}", e)).unwrap_or_default();
                    // SAFETY: IL2CPP FFI call; host vtable and resolved symbols are valid for process lifetime.
                    unsafe { (vt.gui_show_notification)(msg.as_ptr()) };
                    hlog_error!("start_tracking failed: {}", e);
                }
            }
        }
        let hint = c"Reads stats directly from game memory via IL2CPP";
        // SAFETY: IL2CPP FFI call; host vtable and resolved symbols are valid for process lifetime.
        unsafe { (vt.gui_ui_small)(ui, hint.as_ptr()) };
        return;
    }

    let btn = c"\u{23f9} Stop Memory Tracking";
    // SAFETY: Plugin FFI interop with Hachimi vtable
    if unsafe { (vt.gui_ui_button)(ui, btn.as_ptr()) } {
        memory_reader::stop_tracking();
        // SAFETY: IL2CPP FFI call; host vtable and resolved symbols are valid for process lifetime.
        unsafe { (vt.gui_show_notification)(c"Memory tracking stopped".as_ptr()) };
        return;
    }

    // Brief status — detailed stats are in the overlay
    let status = match memory_reader::read_snapshot() {
        Some(snap) if snap.is_playing => CString::new(format!(
            "\u{2705} Tracking • Turn {} • Total {}",
            snap.current_turn, snap.total_stats
        ))
        .unwrap_or_default(),
        Some(_) => c"\u{23f8} No active career".to_owned(),
        None => c"\u{26a0} Singleton unavailable".to_owned(),
    };
    // SAFETY: IL2CPP FFI call; host vtable and resolved symbols are valid for process lifetime.
    unsafe { (vt.gui_ui_small)(ui, status.as_ptr()) };
}

/// Draw a compact hook-counts status line with a reset button.
fn draw_hook_status(ui: *mut c_void) {
    let vt = vt();

    let tracker = match TRACKER.lock() {
        Ok(t) => t,
        Err(_) => return,
    };

    let total = tracker.total();
    if !tracker.active && total == 0 {
        return;
    }
    drop(tracker);

    let status = CString::new(format!("Hook events: {} trainings recorded", total)).unwrap_or_default();
    // SAFETY: Plugin FFI interop with Hachimi vtable
    unsafe { (vt.gui_ui_small)(ui, status.as_ptr()) };

    let reset = c"Reset Counts";
    // SAFETY: Plugin FFI interop with Hachimi vtable
    if unsafe { (vt.gui_ui_small_button)(ui, reset.as_ptr()) } {
        if let Ok(mut t) = TRACKER.lock() {
            t.counts = [0; 5];
            // SAFETY: IL2CPP FFI call; host vtable and resolved symbols are valid for process lifetime.
            unsafe { (vt.gui_show_notification)(c"Training counts reset!".as_ptr()) };
        }
    }
}

// ===========================================================================
// Overlay (always-on-screen HUD)
// ===========================================================================

extern "C" fn draw_overlay(ui: *mut c_void, _userdata: *mut c_void) {
    if panic::catch_unwind(AssertUnwindSafe(|| draw_overlay_inner(ui))).is_err() {
        hlog_error!("draw_overlay PANICKED");
        let vt = vt();
        // SAFETY: IL2CPP FFI call; host vtable and resolved symbols are valid for process lifetime.
        unsafe {
            (vt.gui_ui_colored_label)(ui, 255, 70, 70, 255, c"[overlay render error]".as_ptr());
        }
    }
}

fn draw_overlay_inner(ui: *mut c_void) {
    let vt = vt();

    let tracking = memory_reader::TRACKING.load(Ordering::Relaxed);

    // SAFETY: IL2CPP FFI call; host vtable and resolved symbols are valid for process lifetime.
    unsafe {
        (vt.gui_ui_set_font_size)(ui, get_font_size());
    }

    // SAFETY: IL2CPP FFI call; host vtable and resolved symbols are valid for process lifetime.
    unsafe {
        (vt.gui_ui_set_min_width)(ui, 300.0);
    }

    if tracking {
        draw_overlay_memory(ui);
    } else {
        draw_overlay_hooks(ui);
    }
}

/// Overlay: memory-read live stats.
fn draw_overlay_memory(ui: *mut c_void) {
    let vt = vt();

    let snap = match memory_reader::read_snapshot() {
        Some(s) if s.is_playing => s,
        Some(_) => {
            // SAFETY: IL2CPP FFI call; host vtable and resolved symbols are valid for process lifetime.
            unsafe {
                (vt.gui_ui_small)(ui, c"\u{1f3cb} No active career".as_ptr());
            }
            return;
        }
        None => return, // Singleton unavailable, show nothing
    };

    // SAFETY: Reading field or calling method on non-null IL2CPP object pointer.
    unsafe {
        // Header with turn
        let header = CString::new(format!(
            "\u{1f3cb} Turn {} \u{2022} Month {}",
            snap.current_turn, snap.month
        ))
        .unwrap_or_default();
        (vt.gui_ui_small)(ui, header.as_ptr());

        // Stats: compact rows with levels
        let lv = &snap.training_levels;
        let row1 = CString::new(format!(
            "Speed {:>4}(L{})  Stamina {:>4}(L{})  Power {:>4}(L{})",
            snap.speed, lv[0], snap.stamina, lv[1], snap.power, lv[2]
        ))
        .unwrap_or_default();
        (vt.gui_ui_small)(ui, row1.as_ptr());

        let row2 = CString::new(format!(
            "Guts {:>4}(L{})  Wit {:>4}(L{})  Total {:>4}",
            snap.guts, lv[3], snap.wiz, lv[4], snap.total_stats
        ))
        .unwrap_or_default();
        (vt.gui_ui_small)(ui, row2.as_ptr());

        // Energy + Mood
        let (mr, mg, mb) = memory_reader::motivation_color(snap.motivation);
        let energy_mood = CString::new(format!(
            "Energy {}/{}  Mood: {}",
            snap.hp,
            snap.max_hp,
            memory_reader::mood_label(snap.motivation)
        ))
        .unwrap_or_default();
        (vt.gui_ui_colored_label)(ui, mr, mg, mb, 255, energy_mood.as_ptr());

        // Fans + Races (SP omitted until ObscuredInt decryption is implemented)
        let extra = CString::new(format!(
            "Fans {}  Races {}/{}W",
            format_number(snap.fan_count),
            snap.total_races,
            snap.win_count
        ))
        .unwrap_or_default();
        (vt.gui_ui_small)(ui, extra.as_ptr());

        // Collapsible panels (requires API v6)
        let api_version = Sdk::get().version();
        if api_version.supports_collapsing() {
            (vt.gui_ui_collapsing)(
                ui,
                c"\u{1f4d6} Skills".as_ptr(),
                false,
                Some(draw_skills_panel),
                std::ptr::null_mut(),
            );
            (vt.gui_ui_collapsing)(
                ui,
                c"\u{1f91d} Bonds".as_ptr(),
                false,
                Some(draw_bonds_panel),
                std::ptr::null_mut(),
            );
            (vt.gui_ui_collapsing)(
                ui,
                c"\u{1f6d2} Skill Shop".as_ptr(),
                false,
                Some(draw_skill_shop_panel),
                std::ptr::null_mut(),
            );
        }
    }
}

/// Draw the skills panel inside a collapsing header.
extern "C" fn draw_skills_panel(ui: *mut c_void, _userdata: *mut c_void) {
    if panic::catch_unwind(AssertUnwindSafe(|| draw_skills_panel_inner(ui))).is_err() {
        hlog_error!("draw_skills_panel PANICKED");
    }
}

fn draw_skills_panel_inner(ui: *mut c_void) {
    let vt = vt();
    let skills = memory_reader::read_acquired_skills();

    if skills.is_empty() {
        // SAFETY: IL2CPP FFI call; host vtable and resolved symbols are valid for process lifetime.
        unsafe { (vt.gui_ui_small)(ui, c"No skills acquired yet".as_ptr()) };
        return;
    }

    for skill in &skills {
        let label = if skill.name.is_empty() {
            CString::new(format!("Lv.{} \u{2022} Skill #{}", skill.level, skill.master_id)).unwrap_or_default()
        } else {
            CString::new(format!("Lv.{} \u{2022} {}", skill.level, skill.name)).unwrap_or_default()
        };
        // SAFETY: IL2CPP FFI call; host vtable and resolved symbols are valid for process lifetime.
        unsafe { (vt.gui_ui_small)(ui, label.as_ptr()) };
    }

    let count = CString::new(format!("{} skills", skills.len())).unwrap_or_default();
    // SAFETY: IL2CPP FFI call; host vtable and resolved symbols are valid for process lifetime.
    unsafe { (vt.gui_ui_colored_label)(ui, 150, 150, 150, 255, count.as_ptr()) };
}

/// Draw the bonds/friendship panel inside a collapsing header.
extern "C" fn draw_bonds_panel(ui: *mut c_void, _userdata: *mut c_void) {
    if panic::catch_unwind(AssertUnwindSafe(|| draw_bonds_panel_inner(ui))).is_err() {
        hlog_error!("draw_bonds_panel PANICKED");
    }
}

fn draw_bonds_panel_inner(ui: *mut c_void) {
    let vt = vt();
    let evals = memory_reader::read_evaluations();

    if evals.is_empty() {
        // SAFETY: IL2CPP FFI call; host vtable and resolved symbols are valid for process lifetime.
        unsafe { (vt.gui_ui_small)(ui, c"No bond data available".as_ptr()) };
        return;
    }

    for eval in &evals {
        // Filter out NPCs that aren't present in this career
        if !eval.is_appear {
            continue;
        }

        let (r, g, b) = bond_color(eval.value);
        let name = if eval.name.is_empty() {
            format!("#{}", eval.target_id)
        } else {
            eval.name.clone()
        };
        let label = CString::new(format!("{} - {}/100", name, eval.value)).unwrap_or_default();
        // SAFETY: IL2CPP FFI call; host vtable and resolved symbols are valid for process lifetime.
        unsafe { (vt.gui_ui_colored_label)(ui, r, g, b, 255, label.as_ptr()) };
    }
}

/// Draw refresh button + SP display in a horizontal row.
extern "C" fn draw_skill_shop_header(ui: *mut c_void, _userdata: *mut c_void) {
    let vt = vt();
    // SAFETY: IL2CPP FFI call; host vtable and resolved symbols are valid for process lifetime.
    if unsafe { (vt.gui_ui_small_button)(ui, c"\u{1f504} Refresh".as_ptr()) } {
        skill_shop::refresh();
    }
    if let Some(sp) = skill_shop::read_skill_points() {
        let label = CString::new(format!("SP: {}", sp)).unwrap_or_default();
        // SAFETY: IL2CPP FFI call; host vtable and resolved symbols are valid for process lifetime.
        unsafe { (vt.gui_ui_small)(ui, label.as_ptr()) };
    }
}

/// Draw the skill shop panel inside a collapsing header.
extern "C" fn draw_skill_shop_panel(ui: *mut c_void, _userdata: *mut c_void) {
    if panic::catch_unwind(AssertUnwindSafe(|| draw_skill_shop_panel_inner(ui))).is_err() {
        hlog_error!("draw_skill_shop_panel PANICKED");
    }
}

fn draw_skill_shop_panel_inner(ui: *mut c_void) {
    let vt = vt();

    // Refresh button + current SP
    // SAFETY: IL2CPP FFI call; host vtable and resolved symbols are valid for process lifetime.
    unsafe {
        (vt.gui_ui_horizontal)(ui, Some(draw_skill_shop_header), std::ptr::null_mut());
    }

    let entries = skill_shop::get_cached();

    if entries.is_empty() {
        return;
    }

    for entry in &entries {
        if entry.is_learned {
            continue;
        }

        let icon = skill_shop::rarity_label(entry.rarity);
        let discount = skill_shop::discount_pct(entry.hint_level, false);
        let (r, g, b) = if entry.rarity >= 2 {
            (255, 200, 50)
        } else {
            (220, 220, 220)
        };

        let name = if entry.name.is_empty() {
            format!("#{}", entry.group_id)
        } else {
            entry.name.clone()
        };

        let cost_str = if entry.base_cost > 0 {
            let discounted = skill_shop::discounted_cost(entry.base_cost, entry.hint_level, false);
            format!(" {}pt", discounted)
        } else {
            String::new()
        };

        let label = if discount > 0 {
            CString::new(format!("{} {} (-{}%{})", icon, name, discount, cost_str)).unwrap_or_default()
        } else {
            CString::new(format!(
                "{} {}{}",
                icon,
                name,
                if cost_str.is_empty() {
                    String::new()
                } else {
                    format!(" ({})", cost_str.trim())
                }
            ))
            .unwrap_or_default()
        };
        // SAFETY: IL2CPP FFI call; host vtable and resolved symbols are valid for process lifetime.
        unsafe { (vt.gui_ui_colored_label)(ui, r, g, b, 255, label.as_ptr()) };
    }
}

/// Color for bond/friendship value: blue → green → orange → gold (max).
pub fn bond_color(value: i32) -> (u8, u8, u8) {
    if value >= 100 {
        (255, 200, 50) // Gold - maxed
    } else if value >= 80 {
        (255, 160, 40) // Orange - high
    } else if value >= 40 {
        (100, 220, 100) // Green - medium
    } else {
        (100, 150, 255) // Blue - low
    }
}

/// Overlay: hook-based training counts (fallback when not memory-tracking).
fn draw_overlay_hooks(ui: *mut c_void) {
    let vt = vt();

    let tracker = match TRACKER.lock() {
        Ok(t) => t,
        Err(_) => return,
    };

    if !tracker.active && tracker.total() == 0 {
        return; // Don't show overlay until first training
    }

    let counts = tracker.counts;
    let total = tracker.total();
    drop(tracker);

    // SAFETY: IL2CPP FFI call; host vtable and resolved symbols are valid for process lifetime.
    unsafe {
        let heading = c"\u{1f3cb} Training";
        (vt.gui_ui_small)(ui, heading.as_ptr());

        for facility in Facility::ALL {
            let count = counts[facility as usize];
            let (r, g, b) = facility_color(facility);
            let text = CString::new(format!("{}: {}", facility.name(), count)).unwrap_or_default();
            (vt.gui_ui_colored_label)(ui, r, g, b, 255, text.as_ptr());
        }

        let total_text = CString::new(format!("Total: {}", total)).unwrap_or_default();
        (vt.gui_ui_small)(ui, total_text.as_ptr());
    }
}

// ===========================================================================
// Helpers
// ===========================================================================

/// Color per facility (matching common game UI colors).
pub(crate) fn facility_color(facility: Facility) -> (u8, u8, u8) {
    match facility {
        Facility::Speed => (70, 130, 255),   // Blue
        Facility::Stamina => (255, 70, 70),  // Red
        Facility::Power => (255, 140, 40),   // Orange
        Facility::Guts => (255, 130, 180),   // Pink
        Facility::Wisdom => (100, 220, 100), // Green
    }
}

/// Format a number with comma separators.
pub(crate) fn format_number(n: i32) -> String {
    if n < 0 {
        return format!("-{}", format_number(-n));
    }
    let s = n.to_string();
    let mut result = String::with_capacity(s.len() + s.len() / 3);
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- bond_color ----

    #[test]
    fn bond_color_thresholds() {
        // Maxed (gold)
        assert_eq!(bond_color(100), (255, 200, 50));
        assert_eq!(bond_color(150), (255, 200, 50));
        // High (orange)
        assert_eq!(bond_color(80), (255, 160, 40));
        assert_eq!(bond_color(99), (255, 160, 40));
        // Medium (green)
        assert_eq!(bond_color(40), (100, 220, 100));
        assert_eq!(bond_color(79), (100, 220, 100));
        // Low (blue)
        assert_eq!(bond_color(0), (100, 150, 255));
        assert_eq!(bond_color(39), (100, 150, 255));
    }

    // ---- format_number ----

    #[test]
    fn format_number_basic() {
        assert_eq!(format_number(0), "0");
        assert_eq!(format_number(999), "999");
        assert_eq!(format_number(1000), "1,000");
        assert_eq!(format_number(1234567), "1,234,567");
    }

    #[test]
    fn format_number_negative() {
        assert_eq!(format_number(-1000), "-1,000");
        assert_eq!(format_number(-42), "-42");
    }

    // ---- facility_color ----

    #[test]
    fn facility_colors_distinct() {
        let colors: Vec<_> = Facility::ALL.iter().map(|f| facility_color(*f)).collect();
        // All 5 should be different
        for i in 0..colors.len() {
            for j in (i + 1)..colors.len() {
                assert_ne!(
                    colors[i],
                    colors[j],
                    "Facilities {:?} and {:?} share color",
                    Facility::ALL[i],
                    Facility::ALL[j]
                );
            }
        }
    }

    // ---- motivation helpers (from memory_reader, tested here for convenience) ----

    #[test]
    fn mood_labels() {
        assert!(memory_reader::mood_label(5).contains("Great"));
        assert!(memory_reader::mood_label(3).contains("Normal"));
        assert!(memory_reader::mood_label(1).contains("Terrible"));
        assert_eq!(memory_reader::mood_label(0), "???");
    }

    #[test]
    fn motivation_colors_distinct() {
        let colors: Vec<_> = (1..=5).map(memory_reader::motivation_color).collect();
        for i in 0..colors.len() {
            for j in (i + 1)..colors.len() {
                assert_ne!(colors[i], colors[j]);
            }
        }
    }
}
