//! GUI rendering via the Hachimi plugin menu system.
//!
//! Registers a menu section and an overlay that display:
//! - Live career stats read directly from game memory (memory-read mode)
//! - Training facility visit counts from hooks (complementary)

use std::ffi::{c_void, CString};
use std::panic::{self, AssertUnwindSafe};
use std::sync::atomic::{AtomicI32, Ordering};

use crate::memory_reader;
use crate::tracker::{Facility, TRACKER};
use crate::vtable::vt;

/// Host plugin API version captured during registration.
static API_VERSION: AtomicI32 = AtomicI32::new(0);

/// Overlay font size in pixels. Stored as f32 bits in an AtomicU32.
static OVERLAY_FONT_SIZE: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

/// Default overlay font size.
const DEFAULT_FONT_SIZE: f32 = 11.0;

fn get_font_size() -> f32 {
    let bits = OVERLAY_FONT_SIZE.load(Ordering::Relaxed);
    if bits == 0 { DEFAULT_FONT_SIZE } else { f32::from_bits(bits) }
}

fn set_font_size(size: f32) {
    let clamped = size.clamp(6.0, 30.0);
    OVERLAY_FONT_SIZE.store(clamped.to_bits(), Ordering::Relaxed);
}

/// The overlay ID used during registration — must match for show/hide calls.
const OVERLAY_ID: &std::ffi::CStr = c"training_tracker_overlay";

/// Register the plugin's UI components with the Hachimi GUI.
pub fn register_ui(api_version: i32) {
    API_VERSION.store(api_version, Ordering::Relaxed);

    let vt = vt();
    unsafe {
        (vt.gui_register_menu_section)(Some(draw_menu_section), std::ptr::null_mut());

        if api_version >= 3 {
            (vt.gui_register_overlay)(OVERLAY_ID.as_ptr(), Some(draw_overlay), std::ptr::null_mut());
            hlog_info!("UI registered (menu + overlay)");
        } else {
            hlog_info!("UI registered (menu only, host API v{} < 3)", api_version);
        }
    }
}

// ===========================================================================
// Menu section (drawn inside Hachimi menu panel)
// ===========================================================================

extern "C" fn draw_menu_section(ui: *mut c_void, _userdata: *mut c_void) {
    if let Err(_) = panic::catch_unwind(AssertUnwindSafe(|| draw_menu_section_inner(ui))) {
        hlog_error!("draw_menu_section PANICKED");
        let vt = vt();
        unsafe { (vt.gui_ui_colored_label)(ui, 255, 70, 70, 255, c"[Training Tracker: menu render error]".as_ptr()); }
    }
}

fn draw_menu_section_inner(ui: *mut c_void) {
    let vt = vt();
    let api_version = API_VERSION.load(Ordering::Relaxed);

    unsafe {
        let heading = c"\u{1f3cb} Training Tracker";
        (vt.gui_ui_heading)(ui, heading.as_ptr());

        // --- Memory-read tracking controls ---
        draw_tracking_controls(ui);

        // --- Hook counts status + reset ---
        draw_hook_status(ui);

        // --- Show overlay button ---
        if api_version >= 5 {
            let show_overlay = c"\u{1f4ca} Show Training Overlay";
            if (vt.gui_ui_button)(ui, show_overlay.as_ptr()) {
                (vt.gui_overlay_set_visible)(OVERLAY_ID.as_ptr(), true);
                (vt.gui_show_notification)(c"Training overlay shown".as_ptr());
            }
        }

        // --- Overlay font size ---
        if api_version >= 7 {
            let label = c"Overlay font size (px):";
            (vt.gui_ui_small)(ui, label.as_ptr());

            static mut FONT_BUF: [u8; 8] = [0; 8];
            // Initialize buffer from current value on first use
            if FONT_BUF[0] == 0 {
                let s = format!("{:.0}", get_font_size());
                let bytes = s.as_bytes();
                let len = bytes.len().min(7);
                FONT_BUF[..len].copy_from_slice(&bytes[..len]);
                FONT_BUF[len] = 0;
            }
            if (vt.gui_ui_text_edit_singleline)(
                ui,
                FONT_BUF.as_mut_ptr() as *mut std::ffi::c_char,
                FONT_BUF.len(),
            ) {
                let end = FONT_BUF.iter().position(|b| *b == 0).unwrap_or(FONT_BUF.len());
                if let Ok(s) = std::str::from_utf8(&FONT_BUF[..end]) {
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
                Ok(()) => unsafe {
                    (vt.gui_show_notification)(c"Memory tracking started!".as_ptr());
                },
                Err(e) => {
                    let msg = CString::new(format!("Failed: {}", e)).unwrap_or_default();
                    unsafe { (vt.gui_show_notification)(msg.as_ptr()) };
                    hlog_error!("start_tracking failed: {}", e);
                }
            }
        }
        let hint = c"Reads stats directly from game memory via IL2CPP";
        unsafe { (vt.gui_ui_small)(ui, hint.as_ptr()) };
        return;
    }

    let btn = c"\u{23f9} Stop Memory Tracking";
    // SAFETY: Plugin FFI interop with Hachimi vtable
    if unsafe { (vt.gui_ui_button)(ui, btn.as_ptr()) } {
        memory_reader::stop_tracking();
        unsafe { (vt.gui_show_notification)(c"Memory tracking stopped".as_ptr()) };
        return;
    }

    // Brief status — detailed stats are in the overlay
    let status = match memory_reader::read_snapshot() {
        Some(snap) if snap.is_playing => {
            CString::new(format!(
                "\u{2705} Tracking • Turn {} • Total {}",
                snap.current_turn, snap.total_stats
            )).unwrap_or_default()
        }
        Some(_) => c"\u{23f8} No active career".to_owned(),
        None => c"\u{26a0} Singleton unavailable".to_owned(),
    };
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
            unsafe { (vt.gui_show_notification)(c"Training counts reset!".as_ptr()) };
        }
    }
}

// ===========================================================================
// Overlay (always-on-screen HUD)
// ===========================================================================

extern "C" fn draw_overlay(ui: *mut c_void, _userdata: *mut c_void) {
    if let Err(_) = panic::catch_unwind(AssertUnwindSafe(|| draw_overlay_inner(ui))) {
        hlog_error!("draw_overlay PANICKED");
        let vt = vt();
        unsafe { (vt.gui_ui_colored_label)(ui, 255, 70, 70, 255, c"[overlay render error]".as_ptr()); }
    }
}

fn draw_overlay_inner(ui: *mut c_void) {
    let vt = vt();
    let tracking = memory_reader::TRACKING.load(Ordering::Relaxed);
    let api_version = API_VERSION.load(Ordering::Relaxed);

    if api_version >= 7 {
        unsafe { (vt.gui_ui_set_font_size)(ui, get_font_size()); }
    }
    if api_version >= 4 {
        unsafe { (vt.gui_ui_set_min_width)(ui, 280.0); }
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
            unsafe {
                (vt.gui_ui_small)(ui, c"\u{1f3cb} No active career".as_ptr());
            }
            return;
        }
        None => return, // Singleton unavailable, show nothing
    };

    unsafe {
        // Header with turn
        let header = CString::new(format!(
            "\u{1f3cb} Turn {} \u{2022} Month {}", snap.current_turn, snap.month
        )).unwrap_or_default();
        (vt.gui_ui_small)(ui, header.as_ptr());

        // Stats: compact rows with levels
        let lv = &snap.training_levels;
        let row1 = CString::new(format!(
            "Speed {:>4}(L{})  Stamina {:>4}(L{})  Power {:>4}(L{})",
            snap.speed, lv[0], snap.stamina, lv[1], snap.power, lv[2]
        )).unwrap_or_default();
        (vt.gui_ui_small)(ui, row1.as_ptr());

        let row2 = CString::new(format!(
            "Guts {:>4}(L{})  Wit {:>4}(L{})  Total {:>4}",
            snap.guts, lv[3], snap.wiz, lv[4], snap.total_stats
        )).unwrap_or_default();
        (vt.gui_ui_small)(ui, row2.as_ptr());

        // Energy + Mood
        let (mr, mg, mb) = memory_reader::motivation_color(snap.motivation);
        let energy_mood = CString::new(format!(
            "Energy {}/{}  Mood: {}",
            snap.hp, snap.max_hp,
            memory_reader::mood_label(snap.motivation)
        )).unwrap_or_default();
        (vt.gui_ui_colored_label)(ui, mr, mg, mb, 255, energy_mood.as_ptr());

        // Fans + Races (SP omitted until ObscuredInt decryption is implemented)
        let extra = CString::new(format!(
            "Fans {}  Races {}/{}W",
            format_number(snap.fan_count), snap.total_races, snap.win_count
        )).unwrap_or_default();
        (vt.gui_ui_small)(ui, extra.as_ptr());

        // Collapsible skills panel (requires API v6)
        let api_version = API_VERSION.load(Ordering::Relaxed);
        if api_version >= 6 {
            (vt.gui_ui_collapsing)(
                ui,
                c"\u{1f4d6} Skills".as_ptr(),
                false,
                Some(draw_skills_panel),
                std::ptr::null_mut(),
            );
        }
    }
}

/// Draw the skills panel inside a collapsing header.
extern "C" fn draw_skills_panel(ui: *mut c_void, _userdata: *mut c_void) {
    if let Err(_) = panic::catch_unwind(AssertUnwindSafe(|| draw_skills_panel_inner(ui))) {
        hlog_error!("draw_skills_panel PANICKED");
    }
}

fn draw_skills_panel_inner(ui: *mut c_void) {
    let vt = vt();
    // TODO: Read _acquiredSkillList from game memory once AcquiredSkill class is introspected.
    // For now, show a placeholder with skill point count.
    let msg = c"Skill list not yet available — run 'Dump Skill Classes' from the menu to introspect AcquiredSkill";
    unsafe { (vt.gui_ui_small)(ui, msg.as_ptr()) };
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
fn facility_color(facility: Facility) -> (u8, u8, u8) {
    match facility {
        Facility::Speed => (70, 130, 255),   // Blue
        Facility::Stamina => (255, 70, 70),  // Red
        Facility::Power => (255, 140, 40),   // Orange
        Facility::Guts => (255, 130, 180),   // Pink
        Facility::Wisdom => (100, 220, 100), // Green
    }
}

/// Format a number with comma separators.
fn format_number(n: i32) -> String {
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
