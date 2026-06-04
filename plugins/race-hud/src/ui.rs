//! Minimal race overlays: HP/velocity cards + standalone timer.

use std::ffi::c_void;
use std::panic::{self, AssertUnwindSafe};

use hachimi_plugin_sdk::{egui, ui_from_ptr, Sdk};

use crate::state::{RunnerRow, UiState};

const OVERLAY_ID: &str = "race_hud";
const TIMER_OVERLAY_ID: &str = "race_hud_timer";
const SOLO_WIDTH: f32 = 210.0;
const TEAM_WIDTH: f32 = 236.0;
const TIMER_WIDTH: f32 = 112.0;
const MAX_VELOCITY_MPS: f32 = 22.0;

/// Register the race-hud overlay panels with Hachimi's GUI.
pub fn register_ui() {
    let sdk = Sdk::get();
    register_panel(sdk, OVERLAY_ID, draw_overlay);
    register_panel(sdk, TIMER_OVERLAY_ID, draw_timer_overlay);
}

fn register_panel(sdk: &Sdk, id: &str, callback: extern "C" fn(*mut c_void, *mut c_void)) {
    let handle = sdk.register_panel(id, callback, std::ptr::null_mut());
    if handle == 0 {
        hlog_warn!(target: "race-hud", "Overlay panel registration declined: {id}");
    } else {
        hlog_info!(target: "race-hud", "Overlay panel registered: {id} ({handle})");
    }
}

extern "C" fn draw_overlay(ui: *mut c_void, _userdata: *mut c_void) {
    // SAFETY: host passes its live `&mut egui::Ui` for this callback.
    let ui = unsafe { ui_from_ptr(ui) };
    if panic::catch_unwind(AssertUnwindSafe(|| draw_overlay_inner(ui))).is_err() {
        hlog_error!(target: "race-hud", "draw_overlay panicked");
    }
}

extern "C" fn draw_timer_overlay(ui: *mut c_void, _userdata: *mut c_void) {
    // SAFETY: host passes its live `&mut egui::Ui` for this callback.
    let ui = unsafe { ui_from_ptr(ui) };
    if panic::catch_unwind(AssertUnwindSafe(|| draw_timer_inner(ui))).is_err() {
        hlog_error!(target: "race-hud", "draw_timer_overlay panicked");
    }
}

fn draw_overlay_inner(ui: &mut egui::Ui) {
    let st = crate::state::ui_state();

    if let Some(live) = &st.live {
        let rows: Vec<_> = live.rows.iter().take(3).collect();
        if rows.len() <= 1 {
            ui.set_min_width(SOLO_WIDTH);
        } else {
            ui.set_min_width(TEAM_WIDTH);
        }
        draw_live_cards(ui, &rows);
        draw_watch_picker(ui, &st);
    } else {
        ui.set_min_width(SOLO_WIDTH);
        draw_facts(ui, &st);
    }
}

fn draw_timer_inner(ui: &mut egui::Ui) {
    let st = crate::state::ui_state();
    ui.set_min_width(TIMER_WIDTH);
    if let Some(live) = &st.live {
        draw_timer(ui, Some(live.elapsed));
    } else {
        draw_timer(ui, None);
    }
}

fn draw_live_cards(ui: &mut egui::Ui, rows: &[&RunnerRow]) {
    if rows.is_empty() {
        draw_idle(ui, "Waiting for runners…", "Live frame has no rows.");
        return;
    }

    let max_hp = rows.iter().map(|r| r.hp).max().unwrap_or(1).max(1);
    if rows.len() == 1 {
        draw_solo_card(ui, rows[0], max_hp);
    } else {
        draw_team_card(ui, rows, max_hp);
    }
}

fn draw_solo_card(ui: &mut egui::Ui, row: &RunnerRow, max_hp: u16) {
    panel_frame(ui).show(ui, |ui| {
        metric_row(ui, "HP", row.hp, hp_ratio(row.hp, max_hp), hp_color(ui, row.hp, max_hp));
        ui.add_space(8.0);
        velocity_row(ui, row.speed);
    });
}

fn draw_team_card(ui: &mut egui::Ui, rows: &[&RunnerRow], max_hp: u16) {
    panel_frame(ui).show(ui, |ui| {
        for (i, row) in rows.iter().enumerate() {
            if i > 0 {
                ui.painter().hline(
                    ui.available_rect_before_wrap().x_range(),
                    ui.cursor().top() - 4.0,
                    egui::Stroke::new(1.0, line_color(ui)),
                );
                ui.add_space(8.0);
            }
            team_row(ui, i + 1, row, max_hp);
        }
    });
}

fn metric_row(ui: &mut egui::Ui, label: &str, value: u16, ratio: f32, color: egui::Color32) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(label).small().strong().color(faint_text(ui)));
        draw_bar(
            ui,
            ratio,
            color,
            egui::vec2((ui.available_width() - 66.0).max(24.0), 6.0),
        );
        ui.monospace(value.to_string());
    });
}

fn velocity_row(ui: &mut egui::Ui, speed_raw: u16) {
    let velocity = velocity_mps(speed_raw);
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("VEL").small().strong().color(faint_text(ui)));
        draw_bar(
            ui,
            (velocity / MAX_VELOCITY_MPS).clamp(0.0, 1.0),
            velocity_color(),
            egui::vec2((ui.available_width() - 58.0).max(24.0), 6.0),
        );
        ui.monospace(format!("{velocity:.1}"));
    });
}

fn team_row(ui: &mut egui::Ui, index: usize, row: &RunnerRow, max_hp: u16) {
    ui.horizontal(|ui| {
        ui.monospace(index.to_string());
        draw_bar(
            ui,
            hp_ratio(row.hp, max_hp),
            hp_color(ui, row.hp, max_hp),
            egui::vec2((ui.available_width() - 54.0).max(24.0), 5.0),
        );
        ui.monospace(format!("{:.1}", velocity_mps(row.speed)));
    });
}

fn draw_timer(ui: &mut egui::Ui, elapsed: Option<f32>) {
    let fill = surface_color(ui);
    let line = line_color(ui);
    let text = ui
        .visuals()
        .override_text_color
        .unwrap_or(ui.visuals().strong_text_color());
    let live = elapsed.is_some();
    let label = elapsed.map(format_elapsed).unwrap_or_else(|| "--:--.-".to_owned());

    let desired = egui::vec2(TIMER_WIDTH, 30.0);
    let (rect, _) = ui.allocate_exact_size(desired, egui::Sense::hover());
    if ui.is_rect_visible(rect) {
        ui.painter().rect_filled(rect, 15.0, fill);
        ui.painter().rect_stroke(
            rect,
            15.0,
            egui::Stroke::new(1.0, line),
            egui::epaint::StrokeKind::Inside,
        );
        if live {
            ui.painter()
                .circle_filled(egui::pos2(rect.left() + 13.0, rect.center().y), 3.5, crit_color());
        }
        ui.painter().text(
            rect.center() + egui::vec2(if live { 6.0 } else { 0.0 }, 0.0),
            egui::Align2::CENTER_CENTER,
            label,
            egui::FontId::monospace(16.0),
            text,
        );
    }
}

fn draw_bar(ui: &mut egui::Ui, ratio: f32, color: egui::Color32, size: egui::Vec2) {
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    if ui.is_rect_visible(rect) {
        ui.painter()
            .rect_filled(rect, rect.height() / 2.0, ui.visuals().extreme_bg_color);
        let fill = egui::Rect::from_min_max(rect.min, egui::pos2(rect.left() + rect.width() * ratio, rect.bottom()));
        ui.painter().rect_filled(fill, rect.height() / 2.0, color);
    }
}

/// Collapsible source selector. This is temporary: until the plugin can identify
/// the player's actual team, the watch list is the solo/team source.
fn draw_watch_picker(ui: &mut egui::Ui, st: &UiState) {
    if st.watch.is_empty() {
        return;
    }

    ui.add_space(6.0);
    egui::CollapsingHeader::new("Source")
        .default_open(false)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                if ui.small_button("All").clicked() {
                    crate::state::set_all_watched(true);
                }
                if ui.small_button("None").clicked() {
                    crate::state::set_all_watched(false);
                }
            });
            for entry in &st.watch {
                let mut checked = entry.watched;
                let label = format!("Runner {}", entry.post);
                if ui.checkbox(&mut checked, label).changed() {
                    crate::state::toggle_watch(entry.post);
                }
            }
        });
}

fn draw_facts(ui: &mut egui::Ui, st: &UiState) {
    match st.summary {
        Some(s) => {
            draw_idle(
                ui,
                "Race decoded",
                &format!(
                    "{} runners · {} captures · waiting for start",
                    s.horse_num, st.capture_count
                ),
            );
        }
        None if st.captured => draw_idle(ui, "Decode failed", "Captured SimData was not recognized."),
        None => draw_idle(ui, "Waiting for race…", "Enter a race to capture SimData."),
    }
}

fn draw_idle(ui: &mut egui::Ui, title: &str, hint: &str) {
    panel_frame(ui).show(ui, |ui| {
        ui.vertical_centered(|ui| {
            ui.label(egui::RichText::new(title).strong());
            ui.label(egui::RichText::new(hint).small().color(faint_text(ui)));
        });
    });
}

fn panel_frame(ui: &egui::Ui) -> egui::Frame {
    egui::Frame::new()
        .fill(surface_color(ui))
        .stroke(egui::Stroke::new(1.0, line_color(ui)))
        .corner_radius(10.0)
        .inner_margin(egui::Margin::symmetric(12, 10))
}

fn hp_ratio(hp: u16, max_hp: u16) -> f32 {
    (f32::from(hp) / f32::from(max_hp.max(1))).clamp(0.0, 1.0)
}

fn hp_color(ui: &egui::Ui, hp: u16, max_hp: u16) -> egui::Color32 {
    let ratio = hp_ratio(hp, max_hp);
    if ratio <= 0.2 {
        crit_color()
    } else if ratio <= 0.4 {
        egui::Color32::from_rgb(217, 167, 43)
    } else {
        ui.visuals().widgets.active.bg_fill
    }
}

fn velocity_mps(speed_raw: u16) -> f32 {
    f32::from(speed_raw) / 100.0
}

fn format_elapsed(seconds: f32) -> String {
    let seconds = seconds.max(0.0);
    let minutes = (seconds / 60.0).floor() as u32;
    let seconds = seconds - minutes as f32 * 60.0;
    format!("{minutes}:{seconds:04.1}")
}

fn surface_color(ui: &egui::Ui) -> egui::Color32 {
    ui.visuals().widgets.inactive.weak_bg_fill.linear_multiply(0.92)
}

fn line_color(ui: &egui::Ui) -> egui::Color32 {
    ui.visuals().window_stroke.color
}

fn faint_text(ui: &egui::Ui) -> egui::Color32 {
    ui.visuals().widgets.inactive.fg_stroke.color.linear_multiply(0.75)
}

fn velocity_color() -> egui::Color32 {
    egui::Color32::from_rgb(70, 194, 232)
}

fn crit_color() -> egui::Color32 {
    egui::Color32::from_rgb(214, 81, 81)
}
