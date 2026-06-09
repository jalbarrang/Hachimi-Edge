//! Training tab: cap warning, turn suggestion, career summary.

use hachimi_plugin_sdk::egui;

use crate::build_profile::Objective;
use crate::cm_model;
use crate::memory_reader;
use crate::planner;
use crate::recommend;

use super::stats_grid::StatRow;

pub(super) fn draw(
    ui: &mut egui::Ui,
    snap: &memory_reader::CareerSnapshot,
    stats: &[StatRow; 5],
    rec: &[recommend::FacilityScore; 5],
    any_capped: bool,
    pctx: &planner::PlannerContext,
    sctx: &recommend::ScoringContext,
) {
    if any_capped {
        ui.small("\u{26a0} target/cap reached — further training wasted");
    }

    // `Off` hides the whole scoring + recommendation system (no suggestion, no
    // CM status) — the cap warning above is factual stat state, so it stays.
    if sctx.objective == Objective::Off {
        return;
    }

    let race_encouraged = recommend::scenario_encourages_racing(snap.scenario_command_base);
    let objective = recommend::effective_objective(sctx);
    let fallback = recommend::cm_fallback_active(sctx);

    match planner::plan_suggestion(
        rec,
        snap.failure_rates,
        race_encouraged,
        pctx,
        &recommend::params(),
        &planner::params(),
    ) {
        recommend::TurnSuggestion::Train(best) => {
            let tag = objective_tag(objective, fallback);
            ui.small(format!(
                "\u{2605} best {}: {} — projected score {}",
                tag, stats[best].0, rec[best].score
            ));
        }
        recommend::TurnSuggestion::Rest => {
            ui.colored_label(egui::Color32::from_rgb(120, 200, 255), "\u{1f4a4} Rest");
        }
        recommend::TurnSuggestion::Race => {
            ui.colored_label(egui::Color32::from_rgb(255, 200, 50), "\u{1f3c1} Race");
        }
    }

    // CM status line (survival / speed-to-cap / power-knee), only under a CM
    // objective with a course set.
    if objective != Objective::Rank {
        draw_cm_status(ui, snap, sctx);
    }
}

/// Short caption tag for the active objective, flagging the Rank fallback.
fn objective_tag(objective: Objective, fallback: bool) -> &'static str {
    if fallback {
        "(Rank — no CM course)"
    } else {
        match objective {
            Objective::Cm => "(CM)",
            // `Off` never reaches here (the footer returns early); Rank is the
            // default caption.
            Objective::Off | Objective::Rank => "(Rank)",
        }
    }
}

/// Compact, color-coded CM status: stamina (have vs survival need), speed-to-soft
/// cap, and power-to-knee. Shown only when a CM course is configured.
fn draw_cm_status(ui: &mut egui::Ui, snap: &memory_reader::CareerSnapshot, sctx: &recommend::ScoringContext) {
    let Some(course) = sctx.course else {
        return;
    };
    let cond = sctx.ground_condition;
    let need = cm_model::effective_stamina_need(
        course,
        sctx.strategy,
        snap.guts.max(1) as f64,
        snap.speed.max(1) as f64,
        sctx.aptitudes.distance_grade,
        cond,
        sctx.recovery_heal_bp,
    )
    .round() as i32;
    // Raise the displayed power knee by the (negative) ground power penalty, so
    // the target reflects the chosen track condition / surface.
    let knee =
        (cm_model::power_knee(course) - cm_model::ground_power_modifier(course.surface, cond) as f64).round() as i32;

    let green = egui::Color32::from_rgb(120, 220, 120);
    let red = egui::Color32::from_rgb(235, 120, 120);
    let amber = egui::Color32::from_rgb(235, 200, 90);

    ui.horizontal_wrapped(|ui| {
        // Stamina survival.
        let (sc, smark) = if snap.stamina >= need {
            (green, "\u{2714}")
        } else {
            (red, "\u{2717}")
        };
        ui.colored_label(sc, format!("Stamina {}/{} {}", snap.stamina, need, smark));
        ui.label("\u{2022}");
        // Speed toward the soft cap (raised by the ground speed penalty on heavy).
        let speed_cap = cm_model::SOFT_CAP - cm_model::ground_speed_modifier(course.surface, cond);
        let speed_col = if snap.speed >= speed_cap { green } else { amber };
        ui.colored_label(speed_col, format!("Speed {}/{}", snap.speed, speed_cap));
        ui.label("\u{2022}");
        // Power toward the knee.
        let power_col = if snap.power >= knee { green } else { amber };
        ui.colored_label(power_col, format!("Power {}/{}", snap.power, knee));
    });
}
