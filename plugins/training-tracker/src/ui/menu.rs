//! L1 menu page (Plugins tab section).

use std::sync::atomic::Ordering;

use hachimi_plugin_sdk::{egui, Sdk};

use crate::build_profile::{self, Objective};
use crate::class_dump;
use crate::cm_model::{self, Strategy};
use crate::config;
use crate::course_data;
use crate::gametora_data;
use crate::memory_reader;
use crate::overlay_cache;
use crate::planner;
use crate::recommend;
use crate::tabs;

use super::constants::OVERLAY_ID;

/// Page title — h2 (theme heading size).
fn heading_h2(ui: &mut egui::Ui, text: impl Into<egui::RichText>) {
    ui.heading(text);
}

/// Section title — h3 (between body and heading).
fn heading_h3(ui: &mut egui::Ui, text: impl Into<egui::RichText>) {
    let style = ui.style();
    let heading_size = egui::TextStyle::Heading.resolve(style).size;
    let body_size = egui::TextStyle::Body.resolve(style).size;
    let size = body_size + (heading_size - body_size) * 0.55;
    ui.label(text.into().size(size).strong());
}

pub(super) fn draw(ui: &mut egui::Ui) {
    let sdk = Sdk::get();

    heading_h2(ui, "\u{1f3cb} Training Tracker");
    ui.add_space(8.0);

    draw_tracking_controls(ui);

    ui.add_space(12.0);
    ui.separator();
    ui.add_space(8.0);
    draw_build_profile(ui);

    ui.add_space(12.0);
    ui.separator();
    ui.add_space(8.0);
    draw_recommendation(ui);

    ui.add_space(12.0);
    ui.separator();
    ui.add_space(8.0);
    draw_multiturn(ui);

    ui.add_space(12.0);
    ui.separator();
    ui.add_space(8.0);
    draw_tab_visibility(ui);

    ui.add_space(12.0);
    ui.separator();
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        if ui.button("\u{1f4ca} Show Training Overlay").clicked() {
            if sdk.overlay_set_visible(OVERLAY_ID, true) {
                sdk.show_notification("Training overlay shown");
            } else {
                hlog_warn!(target: "training-tracker", "Host declined overlay_set_visible");
            }
        }
        if ui.button("\u{1f4cb} Dump All IL2CPP Classes").clicked() {
            class_dump::dump_all_classes();
            sdk.show_notification("Class dump complete — see il2cpp_classes.txt");
        }
    });
}

/// Draw start/stop button and brief status in the menu.
fn draw_tracking_controls(ui: &mut egui::Ui) {
    let sdk = Sdk::get();
    let tracking = memory_reader::TRACKING.load(Ordering::Relaxed);

    if !tracking {
        if ui.button("\u{25b6} Start Memory Tracking").clicked() {
            match memory_reader::start_tracking() {
                Ok(()) => sdk.show_notification("Memory tracking started!"),
                Err(e) => {
                    sdk.show_notification(&format!("Failed: {}", e));
                    hlog_error!("start_tracking failed: {}", e);
                    false
                }
            };
        }
        ui.small("Reads stats directly from game memory via IL2CPP");
        return;
    }

    if ui.button("\u{23f9} Stop Memory Tracking").clicked() {
        memory_reader::stop_tracking();
        sdk.show_notification("Memory tracking stopped");
        return;
    }

    overlay_cache::maybe_request_refresh();
    let status = match overlay_cache::snapshot() {
        Some(snap) if snap.is_playing => format!(
            "\u{2705} Tracking • Turn {} • Total {}",
            snap.current_turn, snap.total_stats
        ),
        Some(_) => "\u{23f8} No active career".to_owned(),
        None => "\u{26a0} Waiting for data…".to_owned(),
    };
    ui.small(status);
}

/// Overlay tab visibility toggles. At least one tab must stay enabled, so the
/// last remaining tab's checkbox is disabled to prevent hiding every tab.
fn draw_tab_visibility(ui: &mut egui::Ui) {
    heading_h3(ui, "\u{1f5c2} Overlay Tabs");
    ui.small("Choose which tabs appear in the overlay");
    ui.add_space(4.0);
    let last_one = tabs::enabled_count() <= 1;
    for (tab, label) in tabs::Tab::ALL {
        let mut on = tabs::is_enabled(tab);
        let lock = last_one && on; // can't disable the only remaining tab
        let resp = ui.add_enabled(!lock, egui::Checkbox::new(&mut on, label));
        if resp.changed() {
            tabs::set_enabled(tab, on);
            config::persist();
        }
        if lock {
            resp.on_hover_text("At least one tab must stay enabled");
        }
    }
}

/// Smart-recommendation tuning. Sliders for how cautious the per-turn suggestion
/// is; values persist on release and a button restores the defaults.
fn draw_recommendation(ui: &mut egui::Ui) {
    heading_h3(ui, "\u{1f9e0} Smart Recommendation");
    ui.small("Tune how cautious the per-turn suggestion is");
    ui.add_space(4.0);
    let mut p = recommend::params();
    let mut changed = false;
    let mut commit = false;
    egui::Grid::new("tt_recommend")
        .num_columns(2)
        .spacing([8.0, 4.0])
        .show(ui, |ui| {
            rec_row(
                ui,
                "Risk penalty threshold",
                "%",
                &mut p.risk_threshold_pct,
                0..=100,
                &mut changed,
                &mut commit,
            );
            rec_row(
                ui,
                "Rest-all threshold",
                "%",
                &mut p.all_risky_pct,
                0..=100,
                &mut changed,
                &mut commit,
            );
            rec_row(
                ui,
                "Failure penalty weight",
                " pts",
                &mut p.mood_drop_penalty,
                0..=500,
                &mut changed,
                &mut commit,
            );
            rec_row(
                ui,
                "Failure stat loss",
                "",
                &mut p.failure_stat_loss,
                0..=100,
                &mut changed,
                &mut commit,
            );
        });
    ui.add_space(4.0);
    if changed {
        recommend::set_params(p);
    }
    if commit {
        config::persist();
    }
    if ui.small_button("Reset to defaults").clicked() {
        recommend::set_params(recommend::RecommendParams::default());
        config::persist();
    }
}

/// One labelled `DragValue` row for the recommendation grid. Sets `changed` while
/// editing and `commit` when the edit is finished (drag stop / focus lost).
fn rec_row(
    ui: &mut egui::Ui,
    label: &str,
    suffix: &str,
    value: &mut i32,
    range: std::ops::RangeInclusive<i32>,
    changed: &mut bool,
    commit: &mut bool,
) {
    ui.label(label);
    let mut drag = egui::DragValue::new(value).range(range);
    if !suffix.is_empty() {
        drag = drag.suffix(suffix);
    }
    let resp = ui.add(drag);
    *changed |= resp.changed();
    *commit |= resp.drag_stopped() || resp.lost_focus();
    ui.end_row();
}

/// Human label for an objective.
fn objective_label(obj: Objective) -> &'static str {
    match obj {
        Objective::Off => "Off",
        Objective::Rank => "Rank (評価点)",
        Objective::Cm => "CM (race power)",
    }
}

/// Build-profile editor: objective + CM target (course/strategy) + presets +
/// per-stat targets & weights. The single source of truth the scorer reads.
fn draw_build_profile(ui: &mut egui::Ui) {
    heading_h3(ui, "\u{1f3af} Build Profile");
    let mut prof = build_profile::active();
    ui.small(format!("Active: {}", prof.name));
    ui.add_space(4.0);

    // `picked` = discrete combo/button choices (persist immediately); `changed` =
    // live drag edits (persist on release via `commit`).
    let mut picked = false;
    let mut changed = false;
    let mut commit = false;

    // --- Objective / strategy / race course on one aligned grid ---
    let grouped = course_data::courses_by_track();
    let first_track = grouped.keys().next().copied();
    // Track is derived from the chosen course (fallback to the first track).
    let mut track = if prof.target_course_id > 0 {
        prof.target_course_id / 100
    } else {
        first_track.unwrap_or(0)
    };
    if let Some(ft) = first_track {
        if !grouped.contains_key(&track) {
            track = ft;
        }
    }
    let course_desc = if prof.target_course_id > 0 {
        course_data::course_label(prof.target_course_id).unwrap_or_else(|| "— none —".to_owned())
    } else {
        "— none —".to_owned()
    };

    egui::Grid::new("tt_profile_controls")
        .num_columns(4)
        .spacing([8.0, 6.0])
        .show(ui, |ui| {
            // Row 1: objective + running style.
            ui.label("Objective");
            egui::ComboBox::from_id_salt("tt_objective")
                .width(150.0)
                .selected_text(objective_label(prof.objective))
                .show_ui(ui, |ui| {
                    for obj in [Objective::Off, Objective::Rank, Objective::Cm] {
                        picked |= ui
                            .selectable_value(&mut prof.objective, obj, objective_label(obj))
                            .changed();
                    }
                });
            ui.label("Strategy");
            egui::ComboBox::from_id_salt("tt_strategy")
                .width(150.0)
                .selected_text(prof.strategy.label())
                .show_ui(ui, |ui| {
                    for s in Strategy::ALL {
                        picked |= ui.selectable_value(&mut prof.strategy, s, s.label()).changed();
                    }
                });
            ui.end_row();

            // Row 2: track + course + ground (only when course data is loaded).
            if grouped.is_empty() {
                ui.label("Track");
                ui.weak("\u{26a0} course data unavailable (run the course-data tool / deploy assets)");
                ui.end_row();
            } else {
                ui.label("Track");
                egui::ComboBox::from_id_salt("tt_track")
                    .width(150.0)
                    .selected_text(course_data::track_name(track))
                    .show_ui(ui, |ui| {
                        for &t in grouped.keys() {
                            if ui.selectable_label(t == track, course_data::track_name(t)).clicked() && t != track {
                                if let Some(first) = grouped.get(&t).and_then(|v| v.first()) {
                                    prof.target_course_id = first.0;
                                    picked = true;
                                }
                            }
                        }
                    });
                egui::ComboBox::from_id_salt("tt_course")
                    .width(190.0)
                    .height(320.0)
                    .selected_text(course_desc)
                    .show_ui(ui, |ui| {
                        for (id, label) in grouped.get(&track).into_iter().flatten() {
                            picked |= ui.selectable_value(&mut prof.target_course_id, *id, label).changed();
                        }
                    });
                ui.horizontal(|ui| {
                    ui.label("Ground");
                    egui::ComboBox::from_id_salt("tt_ground")
                        .width(110.0)
                        .selected_text(prof.ground_condition.label())
                        .show_ui(ui, |ui| {
                            for c in cm_model::GroundCondition::ALL {
                                picked |= ui.selectable_value(&mut prof.ground_condition, c, c.label()).changed();
                            }
                        });
                });
                ui.end_row();
            }
        });

    // --- Recovery skills + survival advisory (only under the CM objective) ---
    if prof.objective == Objective::Cm {
        draw_recovery_picker(ui, &mut prof, &mut picked);
        draw_survival_advisory(ui, &prof);
    }

    ui.add_space(6.0);

    // --- Per-stat targets + weights ---
    ui.small("Targets (0 = game cap) • Weights bias CM scoring per stat");
    egui::Grid::new("tt_profile_stats")
        .num_columns(build_profile::STAT_LABELS.len() + 1)
        .striped(true)
        .spacing([8.0, 4.0])
        .show(ui, |ui| {
            ui.label("");
            for name in build_profile::STAT_LABELS.iter() {
                ui.label(*name);
            }
            ui.end_row();

            ui.strong("Target");
            for value in &mut prof.per_stat_target {
                let resp = ui.add(
                    egui::DragValue::new(value)
                        .speed(10.0)
                        .range(0..=build_profile::MAX_TARGET),
                );
                changed |= resp.changed();
                commit |= resp.drag_stopped() || resp.lost_focus();
            }
            ui.end_row();

            ui.strong("Weight");
            for w in &mut prof.stat_weights {
                let resp = ui.add(egui::DragValue::new(w).speed(0.05).range(0.0..=5.0).max_decimals(2));
                changed |= resp.changed();
                commit |= resp.drag_stopped() || resp.lost_focus();
            }
            ui.end_row();
        });

    if changed || picked {
        build_profile::set_active(prof);
    }
    // Persist on drag release (commit) or any discrete pick; never every drag frame.
    if commit || picked {
        config::persist();
    }
}

/// Persistent text filter for the recovery-skill picker.
static RECOVERY_FILTER: std::sync::Mutex<String> = std::sync::Mutex::new(String::new());

/// Searchable multi-select of the recovery skills the player plans to run. Their
/// summed heal (basis points of max HP) lowers the stamina the recommender
/// expects you to train (see `cm_model::effective_stamina_need`).
fn draw_recovery_picker(ui: &mut egui::Ui, prof: &mut build_profile::BuildProfile, picked: &mut bool) {
    let skills = gametora_data::recovery_skills();
    if skills.is_empty() {
        return; // catalog unavailable — advisory still works with 0 recovery
    }
    let count = prof.recovery_skill_ids.len();
    egui::CollapsingHeader::new(format!("\u{1fa79} Planned recoveries ({count})"))
        .id_salt("tt_recovery_picker")
        .show(ui, |ui| {
            let mut filter = RECOVERY_FILTER.lock().map(|g| g.clone()).unwrap_or_default();
            if ui
                .add(
                    egui::TextEdit::singleline(&mut filter)
                        .hint_text("filter by name")
                        .desired_width(220.0),
                )
                .changed()
            {
                if let Ok(mut g) = RECOVERY_FILTER.lock() {
                    *g = filter.clone();
                }
            }
            let needle = filter.trim().to_lowercase();
            egui::ScrollArea::vertical().max_height(170.0).show(ui, |ui| {
                for rs in &skills {
                    if !needle.is_empty() && !rs.name.to_lowercase().contains(&needle) {
                        continue;
                    }
                    let mut on = prof.recovery_skill_ids.contains(&rs.id);
                    let label = format!(
                        "{} \u{2014} {:.1}% ({} bp)",
                        rs.name,
                        rs.heal_bp as f32 / 100.0,
                        rs.heal_bp
                    );
                    if ui.checkbox(&mut on, label).changed() {
                        if on {
                            if !prof.recovery_skill_ids.contains(&rs.id) {
                                prof.recovery_skill_ids.push(rs.id);
                            }
                        } else {
                            prof.recovery_skill_ids.retain(|&id| id != rs.id);
                        }
                        *picked = true;
                    }
                }
            });
        });
}

/// Live stamina/speed/power advisory from the `cm_model` survival math, using the
/// current career stats when available (else the profile's own targets).
fn draw_survival_advisory(ui: &mut egui::Ui, prof: &build_profile::BuildProfile) {
    let Some(course) = course_data::course_params(prof.target_course_id) else {
        ui.small("\u{2139} pick a CM course to see the stamina survival target");
        return;
    };
    let snap = overlay_cache::snapshot().filter(|s| s.is_playing);
    // Prefer the build's *target* speed/guts so the requirement reflects the
    // intended end-state (and visibly drops as the Guts target rises) rather than
    // the career-start worst case; fall back to the live stat, then to 1.
    let stat_for = |idx: usize, live: Option<i32>| -> f64 {
        let target = prof.per_stat_target[idx];
        if target > 0 {
            target as f64
        } else {
            live.filter(|&v| v > 0).unwrap_or(1) as f64
        }
    };
    let speed = stat_for(0, snap.as_ref().map(|s| s.speed));
    let guts = stat_for(3, snap.as_ref().map(|s| s.guts));
    let apt = snap
        .as_ref()
        .map(|s| recommend::cm_aptitudes_for_course(&s.aptitudes, course))
        .unwrap_or_default();
    let cond = prof.ground_condition;
    let heal_bp = gametora_data::recovery_heal_bp_total(&prof.recovery_skill_ids) as f64;
    let raw = cm_model::stamina_survival_threshold(course, prof.strategy, guts, speed, apt.distance_grade, cond);
    let need = cm_model::effective_stamina_need(course, prof.strategy, guts, speed, apt.distance_grade, cond, heal_bp);
    // Soft/heavy/dirt lowers the effective speed/power, so the *raw* targets the
    // player should aim for shift up by the (negative) ground penalty.
    let speed_cap = cm_model::SOFT_CAP - cm_model::ground_speed_modifier(course.surface, cond);
    let knee = cm_model::power_knee(course) - cm_model::ground_power_modifier(course.surface, cond) as f64;
    let saved = (raw - need).round() as i32;
    let recovery_note = if saved > 0 {
        format!(" (−{saved} from {} recoveries)", prof.recovery_skill_ids.len())
    } else {
        String::new()
    };
    ui.small(format!(
        "\u{1f3c1} Stamina need ≈ {}{} (max spurt + rush buffer; lower with Guts) • Speed soft cap {} • Power knee ≈ {}",
        need.round() as i32,
        recovery_note,
        speed_cap,
        knee.round() as i32,
    ));
}

/// Multi-turn planner knobs (energy / bonds / career-phase lookahead).
fn draw_multiturn(ui: &mut egui::Ui) {
    heading_h3(ui, "\u{1f52e} Multi-turn Planning");
    ui.small("Lookahead beyond this turn (0 depth = greedy, single-turn)");
    ui.add_space(4.0);
    let mut pp = planner::params();
    let mut changed = false;
    let mut commit = false;

    let r =
        ui.add(egui::Slider::new(&mut pp.lookahead_depth, 0..=planner::MAX_LOOKAHEAD_DEPTH).text("Lookahead depth"));
    changed |= r.changed();
    commit |= r.drag_stopped() || r.lost_focus();

    let r = ui.add(egui::Slider::new(&mut pp.lookahead_aggressiveness, 0.0..=2.0).text("Aggressiveness"));
    changed |= r.changed();
    commit |= r.drag_stopped() || r.lost_focus();

    let r = ui.add(
        egui::Slider::new(&mut pp.energy_floor_pct, 0..=100)
            .text("Energy floor %")
            .suffix("%"),
    );
    changed |= r.changed();
    commit |= r.drag_stopped() || r.lost_focus();

    if changed {
        planner::set_params(pp);
    }
    if commit {
        config::persist();
    }
    if ui.small_button("Reset to defaults").clicked() {
        planner::set_params(planner::PlannerParams::default());
        config::persist();
    }
}
