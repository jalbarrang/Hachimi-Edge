//! GUI rendering via the Hachimi plugin menu system.
//!
//! With API v9 the host hands plugins the real `egui::Ui`, so we draw with egui
//! directly. Registers a menu section and an overlay that display:
//! - Live career stats read directly from game memory (memory-read mode)

use std::ffi::c_void;
use std::panic::{self, AssertUnwindSafe};
use std::sync::atomic::Ordering;

use hachimi_plugin_sdk::{egui, ui_from_ptr, Sdk};

use crate::class_dump;
use crate::memory_reader;
use crate::overlay_cache;
use crate::rank_table;
use crate::recommend;
use crate::skill_shop;
use crate::skill_shop_prefs::{cycle_sort_mode, prefs, set_prefs, sort_mode_label, DistanceFilter, StyleFilter};
use crate::stat_targets;

/// Default overlay font size.
const OVERLAY_FONT_SIZE: f32 = 12.0;
const OVERLAY_MIN_WIDTH: f32 = 340.0;
/// Max height for scrollable list tabs (Skills/Bonds/Shop), in points.
const LIST_MAX_HEIGHT: f32 = 220.0;

/// The overlay ID used during registration — must match for show/hide calls.
const OVERLAY_ID: &str = "training_tracker_overlay";

/// In-panel tabs for the overlay (selection is in-memory only; resets on reload).
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum Tab {
    Training = 0,
    Skills = 1,
    Bonds = 2,
    Shop = 3,
    Scenario = 4,
}

static SELECTED_TAB: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);

fn selected_tab() -> Tab {
    match SELECTED_TAB.load(Ordering::Relaxed) {
        1 => Tab::Skills,
        2 => Tab::Bonds,
        3 => Tab::Shop,
        4 => Tab::Scenario,
        _ => Tab::Training,
    }
}

fn set_selected_tab(tab: Tab) {
    SELECTED_TAB.store(tab as u8, Ordering::Relaxed);
}

/// Register the plugin's UI components with the Hachimi GUI.
pub fn register_ui() {
    let sdk = Sdk::get();

    // L1 page (Plugins tab) + L2 panel (floating HUD).
    sdk.register_page(draw_menu_section, std::ptr::null_mut());

    if sdk.register_panel(OVERLAY_ID, draw_overlay, std::ptr::null_mut()) != 0 {
        hlog_info!(target: "training-tracker", "UI registered (L1 page + L2 panel)");
    } else {
        hlog_warn!(
            target: "training-tracker",
            "L1 page registered; L2 panel registration declined by host"
        );
    }
}

// ===========================================================================
// Menu section (drawn inside Hachimi menu panel)
// ===========================================================================

extern "C" fn draw_menu_section(ui: *mut c_void, _userdata: *mut c_void) {
    // SAFETY: host passes its live `&mut egui::Ui` for this callback.
    let ui = unsafe { ui_from_ptr(ui) };
    if panic::catch_unwind(AssertUnwindSafe(|| draw_menu_section_inner(ui))).is_err() {
        hlog_error!("draw_menu_section PANICKED");
    }
}

fn draw_menu_section_inner(ui: &mut egui::Ui) {
    let sdk = Sdk::get();

    ui.heading("\u{1f3cb} Training Tracker");

    draw_tracking_controls(ui);
    draw_stat_targets(ui);

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

/// Per-stat target editor. 0 = use the game cap; a positive value warns earlier.
fn draw_stat_targets(ui: &mut egui::Ui) {
    ui.separator();
    ui.small("\u{1f3af} Stat targets (0 = game cap)");
    let mut t = stat_targets::targets();
    let mut changed = false;
    let mut commit = false;
    egui::Grid::new("tt_targets")
        .num_columns(2)
        .striped(true)
        .show(ui, |ui| {
            for (i, name) in stat_targets::LABELS.iter().enumerate() {
                ui.label(*name);
                let resp = ui.add(
                    egui::DragValue::new(&mut t[i])
                        .speed(10.0)
                        .range(0..=stat_targets::MAX_TARGET),
                );
                changed |= resp.changed();
                // Persist only when the edit settles (not every drag tick).
                commit |= resp.drag_stopped() || resp.lost_focus();
                ui.end_row();
            }
        });
    if changed {
        stat_targets::set_targets(t);
    }
    if commit {
        stat_targets::persist();
    }
    if ui.small_button("Clear targets").clicked() {
        stat_targets::set_targets([0; 5]);
        stat_targets::persist();
    }
}

// ===========================================================================
// Overlay (always-on-screen HUD)
// ===========================================================================

extern "C" fn draw_overlay(ui: *mut c_void, _userdata: *mut c_void) {
    // SAFETY: host passes its live `&mut egui::Ui` for this callback.
    let ui = unsafe { ui_from_ptr(ui) };
    if panic::catch_unwind(AssertUnwindSafe(|| draw_overlay_inner(ui))).is_err() {
        hlog_error!("draw_overlay PANICKED");
    }
}

fn draw_overlay_inner(ui: &mut egui::Ui) {
    let tracking = memory_reader::TRACKING.load(Ordering::Relaxed);

    ui.style_mut().override_font_id = Some(egui::FontId::proportional(OVERLAY_FONT_SIZE));
    ui.set_min_width(OVERLAY_MIN_WIDTH);

    // Start/Stop memory tracking, just above the tabs.
    draw_overlay_tracking_toggle(ui, tracking);

    if !tracking {
        draw_start_hint(ui);
        return;
    }

    ui.separator();
    draw_tab_bar(ui);
    ui.separator();

    match selected_tab() {
        Tab::Training => draw_training_tab(ui),
        Tab::Skills => draw_skills_tab(ui),
        Tab::Bonds => draw_bonds_tab(ui),
        Tab::Shop => draw_skill_shop_tab(ui),
        Tab::Scenario => draw_scenario_tab(ui),
    }
}

/// Hint shown when memory tracking is off.
fn draw_start_hint(ui: &mut egui::Ui) {
    ui.small("\u{1f3cb} Training Tracker");
    ui.small("Memory tracking is off — press Start Tracking above.");
}

/// Horizontal tab bar (text labels).
fn draw_tab_bar(ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        for (tab, label) in [
            (Tab::Training, "Training"),
            (Tab::Skills, "Skills"),
            (Tab::Bonds, "Bonds"),
            (Tab::Shop, "Shop"),
            (Tab::Scenario, "Scenario"),
        ] {
            if ui.selectable_label(selected_tab() == tab, label).clicked() {
                set_selected_tab(tab);
            }
        }
    });
}

/// Resolve the live career snapshot, drawing a placeholder when unavailable.
fn current_snapshot(ui: &mut egui::Ui) -> Option<memory_reader::CareerSnapshot> {
    overlay_cache::maybe_request_refresh();
    match overlay_cache::snapshot() {
        Some(s) if s.is_playing => Some(s),
        Some(_) => {
            ui.small("\u{1f3cb} No active career");
            None
        }
        None => {
            ui.small("\u{1f3cb} Loading career data…");
            None
        }
    }
}

/// Training tab: overview + stat tables (egui::Grid).
fn draw_training_tab(ui: &mut egui::Ui) {
    let Some(snap) = current_snapshot(ui) else {
        return;
    };

    egui::Grid::new("tt_overview")
        .num_columns(2)
        .striped(true)
        .show(ui, |ui| {
            ui.label("Turn");
            ui.label(format!("{} \u{2022} Month {}", snap.current_turn, snap.month));
            ui.end_row();

            let (mr, mg, mb) = memory_reader::motivation_color(snap.motivation);
            ui.label("Energy");
            ui.colored_label(
                egui::Color32::from_rgb(mr, mg, mb),
                format!(
                    "{}/{}  {}",
                    snap.hp,
                    snap.max_hp,
                    memory_reader::mood_label(snap.motivation)
                ),
            );
            ui.end_row();
        });

    ui.add_space(4.0);

    let lv = &snap.training_levels;
    let caps = &snap.stat_caps;
    let tgt = stat_targets::targets();
    let thr = |i: usize, cap: i32| stat_targets::effective_threshold(tgt[i], cap);
    let stats = [
        ("Speed", snap.speed, lv[0], thr(0, caps[0])),
        ("Stamina", snap.stamina, lv[1], thr(1, caps[1])),
        ("Power", snap.power, lv[2], thr(2, caps[2])),
        ("Guts", snap.guts, lv[3], thr(3, caps[3])),
        ("Wit", snap.wiz, lv[4], thr(4, caps[4])),
    ];
    // Smart recommendation: projected 評価点 gain per facility (pure, render-safe).
    let rec = recommend::score_facilities(&recommend::Inputs {
        current: [snap.speed, snap.stamina, snap.power, snap.guts, snap.wiz],
        per_stat_gains: &snap.per_stat_gains,
        caps: *caps,
        targets: tgt,
        failure_rates: snap.failure_rates,
    });
    let mut any_capped = false;
    egui::Grid::new("tt_stats")
        .num_columns(stats.len())
        .striped(true)
        .show(ui, |ui| {
            for (name, _, level, _) in &stats {
                ui.label(format!("{} (L{})", name, level));
            }
            ui.end_row();
            for (_, value, _, cap) in &stats {
                match cap_level(*value, *cap) {
                    CapLevel::AtCap => {
                        any_capped = true;
                        ui.colored_label(egui::Color32::from_rgb(255, 80, 80), format!("{}\u{26a0}", value));
                    }
                    CapLevel::Near => {
                        ui.colored_label(egui::Color32::from_rgb(255, 200, 50), value.to_string());
                    }
                    CapLevel::Normal => {
                        ui.strong(value.to_string());
                    }
                };
            }
            ui.end_row();

            // Total stat gain per facility (dsz) — the headline preview number.
            let gains = &snap.stat_gains;
            for gain in gains {
                if *gain > 0 {
                    ui.colored_label(egui::Color32::from_rgb(120, 200, 255), format!("+{}", gain));
                } else {
                    ui.weak("—");
                }
            }
            ui.end_row();

            // Failure rate per facility (6cy) — color-scaled.
            let fails = &snap.failure_rates;
            for fail in fails {
                if *fail >= 0 {
                    let (r, g, b) = failure_rate_color(*fail);
                    ui.colored_label(egui::Color32::from_rgb(r, g, b), format!("{}%", fail));
                } else {
                    ui.weak("—");
                }
            }
            ui.end_row();

            // Smart recommendation (9ge): projected 評価点 score per facility, ★ best.
            for fs in &rec {
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
    if any_capped {
        ui.small("\u{26a0} target/cap reached — further training wasted");
    }

    let race_encouraged = recommend::scenario_encourages_racing(snap.scenario_command_base);

    match recommend::turn_suggestion(&rec, snap.failure_rates, race_encouraged) {
        recommend::TurnSuggestion::Train(best) => {
            ui.small(format!(
                "\u{2605} best: {} — projected score {}",
                stats[best].0, rec[best].score
            ));
        }
        recommend::TurnSuggestion::Rest => {
            ui.colored_label(egui::Color32::from_rgb(120, 200, 255), "\u{1f4a4} Rest");
        }
        recommend::TurnSuggestion::Race => {
            ui.colored_label(egui::Color32::from_rgb(255, 200, 50), "\u{1f3c1} Race");
        }
    }

    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.strong(format!("Total {}", snap.total_stats));
        ui.separator();
        ui.label(rank_text(&snap));
    });
    ui.small(format!(
        "Fans {}  Races {}/{}W",
        format_number(snap.fan_count),
        snap.total_races,
        snap.win_count
    ));
}

/// Format the Rank cell from the self-computed evaluation: "A • 12,345" when known,
/// otherwise the em-dash placeholder (skill-grade resource missing). The value is
/// exact for skills present in the resource — validated to the point against real
/// careers (see docs/reverse-engineering/career-evaluation.md).
fn rank_text(snap: &memory_reader::CareerSnapshot) -> String {
    match snap.evaluation_value {
        Some(value) => format!(
            "Rank: {} \u{2022} {}",
            rank_table::rank_label(value),
            format_number(value)
        ),
        None => "Rank: \u{2014}".to_owned(),
    }
}

/// Skills tab: acquired-skills list (scrollable).
fn draw_skills_tab(ui: &mut egui::Ui) {
    overlay_cache::maybe_request_refresh();
    egui::ScrollArea::vertical()
        .max_height(LIST_MAX_HEIGHT)
        .auto_shrink([false, true])
        .show(ui, draw_skills_panel);
}

/// Bonds tab: bond names + progress (scrollable).
fn draw_bonds_tab(ui: &mut egui::Ui) {
    overlay_cache::maybe_request_refresh();
    egui::ScrollArea::vertical()
        .max_height(LIST_MAX_HEIGHT)
        .auto_shrink([false, true])
        .show(ui, draw_bonds_panel);
}

/// Skill Shop tab: SP + filters + purchasable list (scrollable).
fn draw_skill_shop_tab(ui: &mut egui::Ui) {
    overlay_cache::maybe_request_refresh();
    if overlay_cache::snapshot().is_none() {
        ui.small("Loading shop data…");
        return;
    }

    if let Some(sp) = overlay_cache::skill_points() {
        ui.strong(format!("SP: {}", sp));
    }
    draw_skill_shop_controls(ui);
    ui.separator();
    egui::ScrollArea::vertical()
        .max_height(LIST_MAX_HEIGHT)
        .auto_shrink([false, true])
        .show(ui, draw_skill_shop_list);
}

/// Scenario tab: scenario-specific readout (currently the Trackblazer RaceCoin shop).
fn draw_scenario_tab(ui: &mut egui::Ui) {
    let Some(snap) = current_snapshot(ui) else {
        return;
    };
    match snap.scenario_state {
        Some(memory_reader::ScenarioState::Trackblazer(shop)) => draw_trackblazer_shop(ui, &shop),
        None => {
            ui.small("No scenario-specific data for this run.");
            ui.small("(Supported: Trackblazer / Make a New Track.)");
        }
    }
}

/// Render the Trackblazer RaceCoin shop: coins + current lineup.
fn draw_trackblazer_shop(ui: &mut egui::Ui, shop: &memory_reader::TrackblazerShop) {
    ui.horizontal(|ui| {
        ui.strong(format!("\u{1f3c5} RaceCoins: {}", shop.coins));
        if shop.sale_value > 0 {
            ui.colored_label(
                egui::Color32::from_rgb(220, 120, 60),
                format!("Sale {}%", shop.sale_value),
            );
        }
        if shop.win_points > 0 {
            ui.small(format!("WinPt: {}", shop.win_points));
        }
    });
    ui.separator();
    if shop.items.is_empty() {
        ui.small("Shop lineup unavailable (open the shop in-game first).");
        return;
    }
    egui::ScrollArea::vertical()
        .max_height(LIST_MAX_HEIGHT)
        .auto_shrink([false, true])
        .show(ui, |ui| {
            egui::Grid::new("trackblazer_shop_grid")
                .num_columns(5)
                .striped(true)
                .spacing([12.0, 4.0])
                .show(ui, |ui| {
                    ui.strong("Item");
                    ui.strong("Effect");
                    ui.strong("Price");
                    ui.strong("Avail");
                    ui.strong("Worth");
                    ui.end_row();
                    for item in &shop.items {
                        draw_shop_item_row(ui, item);
                    }
                });

            draw_owned_items(ui, &shop.owned);
        });
}

/// Owned-inventory section, shown below the shop lineup.
fn draw_owned_items(ui: &mut egui::Ui, owned: &[memory_reader::TrackblazerOwnedItem]) {
    if owned.is_empty() {
        return;
    }
    ui.add_space(8.0);
    ui.separator();
    ui.strong("Owned items");
    egui::Grid::new("trackblazer_owned_grid")
        .num_columns(3)
        .striped(true)
        .spacing([12.0, 4.0])
        .show(ui, |ui| {
            for o in owned {
                if o.name.is_empty() {
                    ui.small(format!("#{}", o.item_id));
                } else {
                    ui.label(&o.name);
                }
                if o.effect.is_empty() {
                    ui.small("—");
                } else {
                    ui.small(&o.effect);
                }
                ui.label(format!("\u{00d7}{}", o.count));
                ui.end_row();
            }
        });
}

/// One shop lineup row in the Item | Effect | Price | Worth grid.
/// Affordable rows use a brighter price; sold-out rows are dimmed.
fn draw_shop_item_row(ui: &mut egui::Ui, item: &memory_reader::TrackblazerShopItem) {
    let dim = egui::Color32::from_rgb(140, 140, 140);

    // Name (fallback to #id when unresolved).
    if item.name.is_empty() {
        ui.small(format!("#{}", item.item_id));
    } else {
        ui.label(&item.name);
    }

    // Effect description.
    if item.effect.is_empty() {
        ui.small("—");
    } else {
        ui.label(&item.effect);
    }

    // Price (strike-through original when discounted).
    ui.horizontal(|ui| {
        let price_color = if item.sold_out() {
            dim
        } else if item.discounted() {
            egui::Color32::from_rgb(220, 120, 60)
        } else {
            egui::Color32::from_rgb(230, 200, 90)
        };
        ui.colored_label(price_color, format!("{} \u{1fa99}", item.coin_num));
        if item.discounted() {
            ui.colored_label(
                dim,
                egui::RichText::new(format!("{}", item.original_coin_num)).strikethrough(),
            );
        }
    });

    // Availability (turns left in the shop).
    if item.turns_left > 0 {
        ui.colored_label(
            egui::Color32::from_rgb(220, 120, 60),
            format!("{} turn(s)", item.turns_left),
        );
    } else {
        ui.small("—");
    }

    // Worth tier badge (curated).
    match item.worth {
        Some(w) => ui.colored_label(worth_color(w), w.label()),
        None => ui.small("—"),
    };
    ui.end_row();
}

/// Colour for an editorial buy-priority tier.
fn worth_color(w: memory_reader::Worth) -> egui::Color32 {
    match w {
        memory_reader::Worth::MustBuy => egui::Color32::from_rgb(110, 200, 110),
        memory_reader::Worth::Situational => egui::Color32::from_rgb(230, 200, 90),
        memory_reader::Worth::Optional => egui::Color32::from_rgb(120, 170, 220),
        memory_reader::Worth::Skip => egui::Color32::from_rgb(150, 150, 150),
    }
}

/// Draw the skills panel inside a collapsing header.
fn draw_skills_panel(ui: &mut egui::Ui) {
    let skills = overlay_cache::skills();

    if skills.is_empty() {
        ui.small("No skills acquired yet");
        return;
    }

    for skill in &skills {
        let label = if skill.name.is_empty() {
            format!("Lv.{} \u{2022} Skill #{}", skill.level, skill.master_id)
        } else {
            format!("Lv.{} \u{2022} {}", skill.level, skill.name)
        };
        ui.small(label);
    }

    ui.colored_label(
        egui::Color32::from_rgb(150, 150, 150),
        format!("{} skills", skills.len()),
    );
}

/// Draw the bonds/friendship panel inside a collapsing header.
fn draw_bonds_panel(ui: &mut egui::Ui) {
    let evals = overlay_cache::evaluations();

    if evals.is_empty() {
        ui.small("No bond data available");
        return;
    }

    for eval in &evals {
        if !eval.is_appear {
            continue;
        }

        let (r, g, b) = bond_color(eval.value);
        let name = if eval.name.is_empty() {
            format!("#{}", eval.target_id)
        } else {
            eval.name.clone()
        };
        ui.colored_label(
            egui::Color32::from_rgb(r, g, b),
            format!("{} - {}/100", name, eval.value),
        );
    }
}

/// Skill shop purchasable list (rendered inside the Shop tab's scroll area).
fn draw_skill_shop_list(ui: &mut egui::Ui) {
    let entries = skill_shop::prepare_entries_for_display(overlay_cache::skill_shop(), &prefs());
    if entries.is_empty() {
        ui.small("No shop skills match filters");
        return;
    }

    for entry in &entries {
        let icon = skill_shop::rarity_label(entry.rarity);
        let discount = skill_shop::discount_pct(entry.hint_level, false);
        let color = if entry.rarity >= 2 {
            egui::Color32::from_rgb(255, 200, 50)
        } else {
            egui::Color32::from_rgb(220, 220, 220)
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

        let prefix = if !entry.has_hint { "[full] " } else { "" };

        let label = if discount > 0 {
            format!("{}{} {} (-{}%{})", prefix, icon, name, discount, cost_str)
        } else if cost_str.is_empty() {
            format!("{}{} {}", prefix, icon, name)
        } else {
            format!("{}{} {} ({})", prefix, icon, name, cost_str.trim())
        };
        ui.colored_label(color, label);
    }
}

fn draw_skill_shop_controls(ui: &mut egui::Ui) {
    let p = prefs();

    if ui
        .small_button(format!("Sort: {}", sort_mode_label(p.sort_mode)))
        .clicked()
    {
        cycle_sort_mode();
    }

    ui.small("Style:");
    for &(label, filter) in StyleFilter::LABELS {
        let selected = p.style_filter == filter;
        if ui
            .small_button(format!("{}{}", if selected { "*" } else { "" }, label))
            .clicked()
        {
            set_prefs(|prefs| prefs.style_filter = filter);
        }
    }

    ui.small("Dist:");
    for &(label, filter) in DistanceFilter::LABELS {
        let selected = p.distance_filter == filter;
        if ui
            .small_button(format!("{}{}", if selected { "*" } else { "" }, label))
            .clicked()
        {
            set_prefs(|prefs| prefs.distance_filter = filter);
        }
    }

    let mut show_hintless = p.show_hintless;
    if ui.checkbox(&mut show_hintless, "Show full-price (no hint)").changed() {
        set_prefs(|prefs| prefs.show_hintless = show_hintless);
    }
    if show_hintless {
        ui.small("Open the in-game skill shop once to capture purchasable rows.");
    }
}

/// Proximity of a stat to its cap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CapLevel {
    Normal,
    Near,
    AtCap,
}

/// Classify a stat value against its cap. Unknown cap (`<= 0`) is always `Normal`.
/// `Near` triggers at ≥ 90% of cap; `AtCap` at ≥ cap.
fn cap_level(value: i32, cap: i32) -> CapLevel {
    if cap <= 0 {
        return CapLevel::Normal;
    }
    if value >= cap {
        CapLevel::AtCap
    } else if value * 100 >= cap * 90 {
        CapLevel::Near
    } else {
        CapLevel::Normal
    }
}

/// Color for a training failure rate %: green (safe) → yellow → orange → red.
fn failure_rate_color(pct: i32) -> (u8, u8, u8) {
    if pct >= 60 {
        (255, 80, 80) // red - dangerous
    } else if pct >= 40 {
        (255, 140, 50) // orange
    } else if pct >= 20 {
        (255, 200, 50) // yellow - caution
    } else {
        (120, 220, 120) // green - safe
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

// ===========================================================================
// Helpers
// ===========================================================================

/// Compact Start/Stop memory-tracking button for the overlay (above the tabs).
fn draw_overlay_tracking_toggle(ui: &mut egui::Ui, tracking: bool) {
    let sdk = Sdk::get();
    if tracking {
        if ui.button("\u{23f9} Stop Tracking").clicked() {
            memory_reader::stop_tracking();
            sdk.show_notification("Memory tracking stopped");
        }
    } else if ui.button("\u{25b6} Start Tracking").clicked() {
        match memory_reader::start_tracking() {
            Ok(()) => {
                sdk.show_notification("Memory tracking started!");
            }
            Err(e) => {
                sdk.show_notification(&format!("Failed: {}", e));
                hlog_error!("start_tracking failed: {}", e);
            }
        }
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
    fn cap_level_thresholds() {
        // Unknown cap → always normal.
        assert_eq!(cap_level(1200, 0), CapLevel::Normal);
        assert_eq!(cap_level(0, 0), CapLevel::Normal);
        // Below 90%.
        assert_eq!(cap_level(1000, 1200), CapLevel::Normal); // 83%
        assert_eq!(cap_level(1079, 1200), CapLevel::Normal); // 89.9%
                                                             // Near: 90%..<100%.
        assert_eq!(cap_level(1080, 1200), CapLevel::Near); // exactly 90%
        assert_eq!(cap_level(1199, 1200), CapLevel::Near);
        // At/over cap.
        assert_eq!(cap_level(1200, 1200), CapLevel::AtCap);
        assert_eq!(cap_level(1300, 1200), CapLevel::AtCap);
    }

    #[test]
    fn failure_rate_color_thresholds() {
        assert_eq!(failure_rate_color(0), (120, 220, 120)); // green
        assert_eq!(failure_rate_color(19), (120, 220, 120));
        assert_eq!(failure_rate_color(20), (255, 200, 50)); // yellow
        assert_eq!(failure_rate_color(39), (255, 200, 50));
        assert_eq!(failure_rate_color(40), (255, 140, 50)); // orange
        assert_eq!(failure_rate_color(59), (255, 140, 50));
        assert_eq!(failure_rate_color(60), (255, 80, 80)); // red
        assert_eq!(failure_rate_color(100), (255, 80, 80));
    }

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
