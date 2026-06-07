//! Training tab: stat columns, gains, failure rates, recommendations.

use hachimi_plugin_sdk::egui;

use crate::memory_reader;
use crate::recommend;
use crate::stat_targets;

use super::super::util::{cap_level, failure_rate_color, stat_rank_color, CapLevel};

/// Per-stat display row: name, value, training level, effective cap threshold.
pub(super) type StatRow = (&'static str, i32, i32, i32);

pub(super) fn build_stats(snap: &memory_reader::CareerSnapshot) -> [StatRow; 5] {
    let lv = &snap.training_levels;
    let caps = &snap.stat_caps;
    let tgt = stat_targets::targets();
    let thr = |i: usize, cap: i32| stat_targets::effective_threshold(tgt[i], cap);
    [
        ("Speed", snap.speed, lv[0], thr(0, caps[0])),
        ("Stamina", snap.stamina, lv[1], thr(1, caps[1])),
        ("Power", snap.power, lv[2], thr(2, caps[2])),
        ("Guts", snap.guts, lv[3], thr(3, caps[3])),
        ("Wit", snap.wiz, lv[4], thr(4, caps[4])),
    ]
}

pub(super) fn score_facilities(snap: &memory_reader::CareerSnapshot) -> [recommend::FacilityScore; 5] {
    let caps = snap.stat_caps;
    recommend::score_facilities(
        &recommend::Inputs {
            current: [snap.speed, snap.stamina, snap.power, snap.guts, snap.wiz],
            per_stat_gains: &snap.per_stat_gains,
            caps,
            targets: stat_targets::targets(),
            failure_rates: snap.failure_rates,
        },
        &recommend::params(),
    )
}

pub(super) fn draw(
    ui: &mut egui::Ui,
    snap: &memory_reader::CareerSnapshot,
    stats: &[StatRow; 5],
    rec: &[recommend::FacilityScore; 5],
) -> bool {
    let mut any_capped = false;
    egui::Grid::new("tt_stats")
        .num_columns(stats.len() + 1)
        .striped(true)
        .show(ui, |ui| {
            // Top-left corner is blank; stat names act as the column header.
            ui.label("");
            for (name, _, level, _) in stats {
                ui.label(format!("{} (L{})", name, level));
            }
            ui.end_row();

            ui.strong("Stat");
            for (_, value, _, cap) in stats {
                // Color is keyed off the stat's letter rank; the cap warning
                // (⚠) is preserved as a marker when the stat is at its cap.
                let color = stat_rank_color(*value);
                let text = match cap_level(*value, *cap) {
                    CapLevel::AtCap => {
                        any_capped = true;
                        format!("{}\u{26a0}", value)
                    }
                    CapLevel::Near | CapLevel::Normal => value.to_string(),
                };
                ui.colored_label(color, text);
            }
            ui.end_row();

            // Single: gain to the trained (own) stat only.
            ui.strong("Single");
            for (i, _) in stats.iter().enumerate() {
                let single = snap.per_stat_gains[i][i];
                if single > 0 {
                    ui.colored_label(egui::Color32::from_rgb(120, 200, 255), format!("+{}", single));
                } else {
                    ui.weak("—");
                }
            }
            ui.end_row();

            // Total: sum of all stat gains from that facility.
            ui.strong("Total");
            for gain in &snap.stat_gains {
                if *gain > 0 {
                    ui.colored_label(egui::Color32::from_rgb(120, 200, 255), format!("+{}", gain));
                } else {
                    ui.weak("—");
                }
            }
            ui.end_row();

            ui.strong("Failure");
            for fail in &snap.failure_rates {
                if *fail >= 0 {
                    let (r, g, b) = failure_rate_color(*fail);
                    ui.colored_label(egui::Color32::from_rgb(r, g, b), format!("{}%", fail));
                } else {
                    ui.weak("—");
                }
            }
            ui.end_row();

            ui.strong("Score");
            for fs in rec {
                if fs.known {
                    if fs.is_best {
                        ui.colored_label(egui::Color32::from_rgb(120, 220, 120), format!("\u{2605}{}", fs.score));
                    } else {
                        ui.weak(fs.score.to_string());
                    }
                } else {
                    ui.weak("—");
                }
            }
            ui.end_row();
        });
    any_capped
}
