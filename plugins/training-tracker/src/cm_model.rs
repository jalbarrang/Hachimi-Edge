//! Closed-form Champions Meeting (CM) race-utility model.
//!
//! Pure functions that answer: *given a target CM course + strategy + the
//! trainee's current stats, what is the race value of one more point in stat X?*
//! This is the foundation the CM-objective scorer builds on — it deliberately
//! replaces the 評価点 (rank) curve (`crate::evaluation::stat_score`), which
//! rewards raw stat magnitude for ranking rather than race-winning power.
//!
//! The formulas are ported from the Torena/uma-sim race engine
//! (`../uma-sim/packages/uma-sim-primitives`, itself a port of umasim) and
//! grounded in the community meta (gametora race-mechanics, uma.guide). They are
//! closed-form and self-contained: no IL2CPP, no cross-repo dependency, so the
//! shipped DLL stays standalone. Parity with the reference engine is asserted in
//! the tests where exact anchors exist.
//!
//! ## Marginal-value unit
//!
//! [`stat_marginal_value`] returns **uutil** ("utility units"): approximately
//! `1000 × (m/s contribution to effective race speed per +1 stat point)`. Speed
//! is the principled backbone (its last-spurt derivative is exact); the other
//! stats are scaled into the same unit through documented heuristic coefficients
//! (anchored, then tuned later). Only *internal consistency* matters — the scorer
//! sums these across a facility's per-stat gains, so the relative magnitudes are
//! what count, not the absolute scale.

// ---------------------------------------------------------------------------
// Value objects
// ---------------------------------------------------------------------------

use serde::{Deserialize, Serialize};

/// Race surface. Discriminants match the master.mdb `ground` column
/// (1 = Turf, 2 = Dirt); the `cm-course-data` tool fills these.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Surface {
    Turf,
    Dirt,
}

/// Running style. Discriminant follows the game / uma-sim convention
/// (FrontRunner = 1 … Runaway = 5) so HP-coefficient lookups index directly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Strategy {
    /// 逃げ (nige).
    FrontRunner,
    /// 先行 (senko).
    PaceChaser,
    /// 差し (sashi).
    LateSurger,
    /// 追込 (oikomi).
    EndCloser,
    /// 大逃げ (oonige).
    Runaway,
}

impl Strategy {
    /// All running styles, in game order, for UI pickers.
    pub const ALL: [Strategy; 5] = [
        Strategy::FrontRunner,
        Strategy::PaceChaser,
        Strategy::LateSurger,
        Strategy::EndCloser,
        Strategy::Runaway,
    ];

    /// Short English label (with the JP romanization) for the UI.
    pub fn label(self) -> &'static str {
        match self {
            Strategy::FrontRunner => "Front (nige)",
            Strategy::PaceChaser => "Pace (senko)",
            Strategy::LateSurger => "Late (sashi)",
            Strategy::EndCloser => "End (oikomi)",
            Strategy::Runaway => "Runaway (oonige)",
        }
    }

    /// 1-based discriminant matching uma-sim's `Strategy as usize`.
    pub fn discriminant(self) -> usize {
        match self {
            Strategy::FrontRunner => 1,
            Strategy::PaceChaser => 2,
            Strategy::LateSurger => 3,
            Strategy::EndCloser => 4,
            Strategy::Runaway => 5,
        }
    }

    /// Stamina→HP conversion coefficient (`HP_STRATEGY_COEFFICIENT`).
    /// Late Surger / End Closer convert best; Pace Chaser worst.
    pub fn hp_coef(self) -> f64 {
        // [_, nige .95, senko .89, sashi 1.0, oikomi .995, oonige .86]
        const HP_STRATEGY_COEFFICIENT: [f64; 6] = [0.0, 0.95, 0.89, 1.0, 0.995, 0.86];
        HP_STRATEGY_COEFFICIENT[self.discriminant()]
    }

    /// Speed strategy×phase coefficient (uma-sim `speed::STRATEGY_PHASE_COEFFICIENT`).
    /// `phase_col`: 0 = early, 1 = mid, 2 = late/last-spurt.
    fn speed_phase_coef(self, phase_col: usize) -> f64 {
        const SPEED_STRATEGY_PHASE: [[f64; 3]; 6] = [
            [0.0, 0.0, 0.0],
            [1.0, 0.98, 0.962],
            [0.978, 0.991, 0.975],
            [0.938, 0.998, 0.994],
            [0.931, 1.0, 1.0],
            [1.063, 0.962, 0.95],
        ];
        SPEED_STRATEGY_PHASE[self.discriminant()][phase_col.min(2)]
    }
}

/// The five core stats, in the plugin's canonical order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StatKind {
    Speed,
    Stamina,
    Power,
    Guts,
    Wit,
}

impl StatKind {
    /// Index into a `[_; 5]` stat array ([Speed, Stamina, Power, Guts, Wit]).
    pub fn index(self) -> usize {
        match self {
            StatKind::Speed => 0,
            StatKind::Stamina => 1,
            StatKind::Power => 2,
            StatKind::Guts => 3,
            StatKind::Wit => 4,
        }
    }
}

/// Per-course parameters the CM math needs. Owned here (the canonical shape);
/// the `cm-course-data` maintainer tool fills these from master.mdb. Fields the
/// model does not read (turn, finish times) are carried for the data layer / UI.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CourseParams {
    /// Course distance in meters.
    pub distance: f64,
    /// Turf or dirt.
    pub surface: Surface,
    /// Track orientation (master.mdb `turn`); unused by the math, kept for UI.
    pub turn: i32,
    /// Course "set status" stat thresholds: crossing each by multiples of 300
    /// (up to 900) grants a +5% speed bonus per step. Serialized as `thresholds`
    /// to match the `course-data` tool's asset.
    #[serde(rename = "thresholds", default)]
    pub set_status_thresholds: Vec<StatKind>,
    /// Reference finish-time window (master.mdb), carried for the data layer.
    #[serde(default)]
    pub finish_time_min: f64,
    /// Reference finish-time window (master.mdb), carried for the data layer.
    #[serde(default)]
    pub finish_time_max: f64,
}

/// Race aptitude grades relevant to the CM math, using the game's `ProperGrade`
/// convention: `Null = 0`, `G = 1` … `S = 8`.
#[derive(Debug, Clone, Copy, Default)]
pub struct Aptitudes {
    /// Distance aptitude for the target course (affects top spurt speed + accel).
    pub distance_grade: i32,
    /// Surface aptitude for the target course (affects acceleration).
    pub surface_grade: i32,
}

// ---------------------------------------------------------------------------
// Proficiency tables (uma-sim `course::coefficients`), indexed S=0 … G=7
// ---------------------------------------------------------------------------

/// Map a `ProperGrade` (Null=0, G=1 … S=8) to a proficiency-table index
/// (S=0 … G=7). Unknown/Null falls back to A (neutral, index 1).
fn apt_index(grade: i32) -> usize {
    if grade <= 0 {
        1 // treat "no data" as A (1.0×) rather than punishing it
    } else {
        (8 - grade).clamp(0, 7) as usize
    }
}

/// Distance proficiency multiplier for **target speed** (speed table).
fn speed_distance_prof(grade: i32) -> f64 {
    const T: [f64; 8] = [1.05, 1.0, 0.9, 0.8, 0.6, 0.4, 0.2, 0.1];
    T[apt_index(grade)]
}

/// Distance proficiency multiplier for **acceleration** (accel table).
fn accel_distance_prof(grade: i32) -> f64 {
    const T: [f64; 8] = [1.0, 1.0, 1.0, 1.0, 1.0, 0.6, 0.5, 0.4];
    T[apt_index(grade)]
}

/// Surface (ground-type) proficiency multiplier for **acceleration**.
fn ground_surface_prof(grade: i32) -> f64 {
    const T: [f64; 8] = [1.05, 1.0, 0.9, 0.8, 0.7, 0.5, 0.3, 0.1];
    T[apt_index(grade)]
}

// ---------------------------------------------------------------------------
// Core scalar formulas (ports)
// ---------------------------------------------------------------------------

/// Course base speed: `20 − (distance − 2000) / 1000`.
pub fn base_speed(distance: f64) -> f64 {
    20.0 - (distance - 2000.0) / 1000.0
}

/// Maximum HP at race start: `0.8 × strategy_coef × stamina + distance`.
pub fn max_hp(stamina: f64, strategy: Strategy, distance: f64) -> f64 {
    0.8 * strategy.hp_coef() * stamina + distance
}

/// Spurt-phase HP-burn modifier from Guts: `1 + 200 / sqrt(600 × guts)`.
/// Higher Guts ⇒ lower modifier ⇒ less HP burned during the last spurt.
pub fn guts_modifier(guts: f64) -> f64 {
    1.0 + 200.0 / (600.0 * guts.max(1.0)).sqrt()
}

/// Firm-ground HP consumption coefficient for a surface (planning baseline:
/// CM weather varies, but firm is the reference; wetter ground only raises burn).
fn ground_consumption_coef(_surface: Surface) -> f64 {
    1.0
}

/// HP consumed per second at `velocity` (port of `calculate_hp_per_second` with
/// `status_modifier = 1`). The Guts modifier applies only in the spurt phase.
fn hp_per_second(velocity: f64, base_speed: f64, ground_coef: f64, guts_mod: f64, in_spurt: bool) -> f64 {
    let guts = if in_spurt { guts_mod } else { 1.0 };
    (20.0 * (velocity - base_speed + 12.0).powi(2) / 144.0) * ground_coef * guts
}

/// Mid-race target speed (no Speed-stat term in phases 0/1): `base × strat_coef`.
fn mid_target_speed(strategy: Strategy, base_speed: f64) -> f64 {
    base_speed * strategy.speed_phase_coef(1)
}

/// Maximum last-spurt speed (port of `calculate_last_spurt_speed`). Depends on
/// Speed (twice: phase-2 target + spurt term), Guts, distance aptitude, strategy.
pub fn last_spurt_speed(speed: f64, guts: f64, strategy: Strategy, distance_grade: i32, base_speed: f64) -> f64 {
    let prof = speed_distance_prof(distance_grade);
    let phase2_target = base_speed * strategy.speed_phase_coef(2) + (500.0 * speed).sqrt() * prof * 0.002;
    let mut result = (phase2_target + 0.01 * base_speed) * 1.05 + (500.0 * speed).sqrt() * prof * 0.002;
    result += (450.0 * guts.max(1.0)).powf(0.597) * 0.0001;
    result
}

// ---------------------------------------------------------------------------
// Soft-cap / overcap and course set-status thresholds
// ---------------------------------------------------------------------------

/// Stat value at which the in-race soft cap kicks in (points above count half).
pub const SOFT_CAP: i32 = 1200;

/// Power "enough" knee centre for a course: longer / dirt courses want more Power
/// before the marginal value tapers. Exposed so the UI can show a Power target.
pub fn power_knee(course: &CourseParams) -> f64 {
    POWER_KNEE_BASE
        + if course.surface == Surface::Dirt { 100.0 } else { 0.0 }
        + ((course.distance - 2000.0) / 400.0).clamp(-100.0, 200.0)
}

/// In-race effective stat value: points above 1200 count half (`adjust_overcap`).
pub fn effective_in_race_value(stat: f64) -> f64 {
    if stat > 1200.0 {
        1200.0 + ((stat - 1200.0) / 2.0).floor()
    } else {
        stat
    }
}

/// Course set-status speed multiplier (port of `speed_modifier`): each threshold
/// stat contributes `(1 + floor(min(stat, 901) / 300.01)) × 0.05`, averaged over
/// the threshold list. Returns `1.0` when the course has no thresholds.
pub fn speed_set_status_multiplier(stats: [i32; 5], course: &CourseParams) -> f64 {
    let list = &course.set_status_thresholds;
    if list.is_empty() {
        return 1.0;
    }
    let sum: f64 = list
        .iter()
        .map(|k| {
            let v = (stats[k.index()] as f64).min(901.0);
            (1.0 + (v / 300.01).floor()) * 0.05
        })
        .sum();
    1.0 + sum / list.len() as f64
}

// ---------------------------------------------------------------------------
// Stamina survival threshold
// ---------------------------------------------------------------------------

/// Rush HP buffer expressed in *stamina points*: ≈45 (short) … 180 (long),
/// linear in distance. Rushing (掛かり) raises consumption ~1.6× for a window;
/// this reserves stamina so a rush does not break the spurt.
pub fn rush_buffer_stamina(distance: f64) -> f64 {
    let t = ((distance - 1200.0) / (3000.0 - 1200.0)).clamp(0.0, 1.0);
    45.0 + t * (180.0 - 45.0)
}

/// Stamina needed to sustain a **full max last-spurt** for this course/strategy,
/// including the rush buffer. This is the dominant CM non-linearity: below it the
/// trainee gasses out and cannot spurt; above it, extra stamina is mostly wasted.
///
/// Heuristic closed-form: estimate total HP burned over the race as
/// `non-spurt portion (0 → 2/3 distance at mid speed) + spurt portion
/// (final 1/3 at max spurt speed, Guts modifier on)`, then solve
/// `max_hp(stamina) ≥ total_hp` for stamina and add the rush buffer. Recovery
/// skills are not modelled (the player equips those separately), so this is a
/// deliberately conservative survival floor.
pub fn stamina_survival_threshold(
    course: &CourseParams,
    strategy: Strategy,
    guts: f64,
    speed: f64,
    distance_grade: i32,
) -> f64 {
    let distance = course.distance;
    let bs = base_speed(distance);
    let ground = ground_consumption_coef(course.surface);
    let g_mod = guts_modifier(guts);

    let mid_speed = mid_target_speed(strategy, bs);
    let spurt_speed = last_spurt_speed(speed, guts, strategy, distance_grade, bs);

    // Non-spurt portion: start → 2/3 of the course at mid target speed (Guts off).
    let nonspurt_len = distance * 2.0 / 3.0;
    let hp_nonspurt = hp_per_second(mid_speed, bs, ground, g_mod, false) * (nonspurt_len / mid_speed);

    // Spurt portion: final third at max spurt speed (Guts on).
    let spurt_len = distance / 3.0;
    let hp_spurt = hp_per_second(spurt_speed, bs, ground, g_mod, true) * (spurt_len / spurt_speed);

    let total_hp = hp_nonspurt + hp_spurt;
    // Solve 0.8 * hp_coef * stamina + distance >= total_hp.
    let stamina = (total_hp - distance) / (0.8 * strategy.hp_coef());
    stamina.max(0.0) + rush_buffer_stamina(distance)
}

// ---------------------------------------------------------------------------
// Marginal stat value (the scoring backbone)
// ---------------------------------------------------------------------------

/// Heuristic anchors that scale the non-speed stats into the speed-derived
/// uutil unit. Tuned later against real builds; tests assert *shape* only.
const SPEED_DERIV_UUTIL: f64 = 1000.0; // m/s → uutil
/// How strongly being below the stamina survival floor is valued (stamina that
/// unlocks the spurt is mandatory, so it dominates while deficient).
const STAMINA_UNLOCK_UUTIL: f64 = 9.0;
/// Smoothing width (stamina points) of the survival knee.
const STAMINA_KNEE_WIDTH: f64 = 120.0;
/// Converts a power-driven acceleration gain into an approximate m/s-equivalent.
const POWER_ACCEL_TO_SPEED: f64 = 0.9;
/// Power "enough" knee center before the surface/distance adjustment.
const POWER_KNEE_BASE: f64 = 900.0;
/// Gentle, never-zero Wit value (skill-proc consistency; no soft cap).
const WIT_UUTIL: f64 = 1.4;
/// Baseline Guts value (minor: small last-spurt + HP-saving contribution).
const GUTS_UUTIL: f64 = 1.1;

/// Smooth 0→1 logistic ramp.
fn logistic(x: f64) -> f64 {
    1.0 / (1.0 + (-x).exp())
}

/// The race-value (uutil) of **one more point** of `stat`, given current stats,
/// the target course, strategy and aptitudes. Threshold-aware: it bakes in the
/// stamina survival floor, the 1200 soft-cap, the power "enough" knee, and the
/// Wit "no soft-cap" behaviour. See the module-level note for the unit.
pub fn stat_marginal_value(
    stat: StatKind,
    current: [i32; 5],
    course: &CourseParams,
    strategy: Strategy,
    apt: Aptitudes,
) -> f64 {
    let speed = current[StatKind::Speed.index()] as f64;
    let stamina = current[StatKind::Stamina.index()] as f64;
    let power = current[StatKind::Power.index()] as f64;
    let guts = current[StatKind::Guts.index()] as f64;
    let wit = current[StatKind::Wit.index()] as f64;

    // Overcap halves marginal in-race value above 1200.
    let overcap = |v: f64| if v >= 1200.0 { 0.5 } else { 1.0 };

    match stat {
        StatKind::Speed => {
            // d(last_spurt_speed)/d(speed): the Speed term appears in the phase-2
            // target (×1.05) and again in the spurt term ⇒ factor 2.05.
            let prof = speed_distance_prof(apt.distance_grade);
            let s_eff = speed.max(1.0);
            let dv = 2.05 * 0.002 * prof * (500.0_f64).sqrt() / (2.0 * s_eff.sqrt());
            dv * SPEED_DERIV_UUTIL * overcap(speed)
        }
        StatKind::Stamina => {
            // High below the survival floor, ~0 above (smooth knee). Crossing the
            // floor unlocks the full spurt, so deficient stamina dominates.
            let floor = stamina_survival_threshold(course, strategy, guts, speed, apt.distance_grade);
            let deficit = floor - stamina;
            STAMINA_UNLOCK_UUTIL
                * SPEED_DERIV_UUTIL
                * 0.001
                * logistic(deficit / STAMINA_KNEE_WIDTH)
                * if stamina >= 1200.0 { 0.5 } else { 1.0 }
        }
        StatKind::Power => {
            // Acceleration derivative, ramped down past a course-tuned knee.
            let strat = strategy.speed_phase_coef(2).max(0.5); // proxy weight
            let g_prof = ground_surface_prof(apt.surface_grade);
            let d_prof = accel_distance_prof(apt.distance_grade);
            let p_eff = power.max(1.0);
            // accel ∝ (500·power)^0.5 ; derivative ∝ 0.5·sqrt(500)/sqrt(power).
            let d_accel = 0.5 * (500.0_f64).sqrt() / p_eff.sqrt() * strat * g_prof * d_prof;
            // Knee: longer / dirt courses want more power before tapering.
            let knee = power_knee(course);
            let knee_factor = 1.0 - logistic((power - knee) / 150.0) * 0.8;
            d_accel * POWER_ACCEL_TO_SPEED * SPEED_DERIV_UUTIL * knee_factor * overcap(power)
        }
        StatKind::Guts => {
            // Minor: last-spurt term + HP saving. Small bump for short / front.
            let g_eff = guts.max(1.0);
            let short = if course.distance <= 1600.0 { 1.4 } else { 1.0 };
            let front = matches!(strategy, Strategy::FrontRunner | Strategy::Runaway);
            let style = if front { 1.3 } else { 1.0 };
            GUTS_UUTIL * short * style / g_eff.sqrt() * 10.0 * overcap(guts)
        }
        StatKind::Wit => {
            // Gentle, diminishing, never zero (no soft cap per uma.guide). Style
            // aptitude scales wit's in-race usefulness, but proc rate is unaffected.
            let w_eff = wit.max(1.0);
            WIT_UUTIL * 30.0 / w_eff.sqrt()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn course(distance: f64, surface: Surface, thresholds: Vec<StatKind>) -> CourseParams {
        CourseParams {
            distance,
            surface,
            turn: 1,
            set_status_thresholds: thresholds,
            finish_time_min: 0.0,
            finish_time_max: 0.0,
        }
    }

    // ---- parity anchors against uma-sim ----

    #[test]
    fn max_hp_parity() {
        // LateSurger 1200 stamina @ 2400 = 0.8*1.0*1200 + 2400 = 3360.
        assert_eq!(max_hp(1200.0, Strategy::LateSurger, 2400.0), 3360.0);
        // Runaway coef 0.86: 0.8*0.86*1000 + 1600 = 688 + 1600 = 2288.
        assert_eq!(max_hp(1000.0, Strategy::Runaway, 1600.0), 2288.0);
    }

    #[test]
    fn hp_strategy_coefficients_match() {
        assert_eq!(Strategy::FrontRunner.hp_coef(), 0.95);
        assert_eq!(Strategy::PaceChaser.hp_coef(), 0.89);
        assert_eq!(Strategy::LateSurger.hp_coef(), 1.0);
        assert_eq!(Strategy::EndCloser.hp_coef(), 0.995);
        assert_eq!(Strategy::Runaway.hp_coef(), 0.86);
    }

    #[test]
    fn base_speed_parity() {
        assert_eq!(base_speed(2000.0), 20.0);
        assert_eq!(base_speed(2400.0), 19.6);
        assert_eq!(base_speed(1600.0), 20.4);
    }

    #[test]
    fn overcap_halves_above_1200() {
        assert_eq!(effective_in_race_value(1000.0), 1000.0);
        assert_eq!(effective_in_race_value(1300.0), 1250.0); // 1200 + floor(100/2)
        assert_eq!(effective_in_race_value(1500.0), 1350.0); // 1200 + 150
    }

    #[test]
    fn set_status_multiplier_parity() {
        // One Speed threshold at 900 → (1 + floor(900/300.01))*0.05 = (1+2)*0.05 = 0.15.
        let c = course(2000.0, Surface::Turf, vec![StatKind::Speed]);
        let m = speed_set_status_multiplier([900, 0, 0, 0, 0], &c);
        assert!((m - 1.15).abs() < 1e-9);
        // No thresholds → 1.0.
        let c0 = course(2000.0, Surface::Turf, vec![]);
        assert_eq!(speed_set_status_multiplier([900, 900, 900, 900, 900], &c0), 1.0);
    }

    #[test]
    fn guts_modifier_decreases_with_guts() {
        assert!(guts_modifier(1200.0) < guts_modifier(400.0));
        assert!(guts_modifier(400.0) > 1.0);
    }

    // ---- survival-threshold shape ----

    #[test]
    fn survival_threshold_grows_with_distance() {
        let short = course(1600.0, Surface::Turf, vec![]);
        let long = course(2400.0, Surface::Turf, vec![]);
        let t_short = stamina_survival_threshold(&short, Strategy::LateSurger, 400.0, 1000.0, 7);
        let t_long = stamina_survival_threshold(&long, Strategy::LateSurger, 400.0, 1000.0, 7);
        assert!(
            t_long > t_short,
            "longer course needs more stamina ({t_short} -> {t_long})"
        );
        // Sanity: a 2400m survival floor lands in a plausible CM range.
        assert!((300.0..1400.0).contains(&t_long), "implausible threshold {t_long}");
    }

    #[test]
    fn survival_threshold_lower_for_better_hp_strategy() {
        let c = course(2400.0, Surface::Turf, vec![]);
        let sashi = stamina_survival_threshold(&c, Strategy::LateSurger, 400.0, 1000.0, 7); // coef 1.0
        let senko = stamina_survival_threshold(&c, Strategy::PaceChaser, 400.0, 1000.0, 7); // coef 0.89
                                                                                            // Worse conversion (senko) needs MORE stamina for the same HP.
        assert!(senko > sashi);
    }

    // ---- marginal-value shape ----

    #[test]
    fn speed_marginal_drops_past_1200() {
        let c = course(2000.0, Surface::Turf, vec![]);
        let below = stat_marginal_value(
            StatKind::Speed,
            [900, 600, 600, 400, 600],
            &c,
            Strategy::PaceChaser,
            Aptitudes {
                distance_grade: 7,
                surface_grade: 7,
            },
        );
        let above = stat_marginal_value(
            StatKind::Speed,
            [1300, 600, 600, 400, 600],
            &c,
            Strategy::PaceChaser,
            Aptitudes {
                distance_grade: 7,
                surface_grade: 7,
            },
        );
        assert!(
            below > above,
            "speed value should fall past the soft cap ({below} vs {above})"
        );
    }

    #[test]
    fn stamina_marginal_high_below_floor_low_above() {
        let c = course(2400.0, Surface::Turf, vec![]);
        let apt = Aptitudes {
            distance_grade: 7,
            surface_grade: 7,
        };
        let floor = stamina_survival_threshold(&c, Strategy::LateSurger, 400.0, 1100.0, 7);
        let low_stam = (floor - 250.0).max(50.0) as i32;
        let high_stam = (floor + 350.0) as i32;
        let deficient = stat_marginal_value(
            StatKind::Stamina,
            [1100, low_stam, 800, 400, 600],
            &c,
            Strategy::LateSurger,
            apt,
        );
        let satisfied = stat_marginal_value(
            StatKind::Stamina,
            [1100, high_stam, 800, 400, 600],
            &c,
            Strategy::LateSurger,
            apt,
        );
        assert!(
            deficient > satisfied * 3.0,
            "stamina must dominate below the floor ({deficient} vs {satisfied})"
        );
    }

    #[test]
    fn power_marginal_ramps_down_past_knee() {
        let c = course(2000.0, Surface::Turf, vec![]);
        let apt = Aptitudes {
            distance_grade: 7,
            surface_grade: 7,
        };
        let low = stat_marginal_value(
            StatKind::Power,
            [1100, 700, 500, 400, 600],
            &c,
            Strategy::PaceChaser,
            apt,
        );
        let high = stat_marginal_value(
            StatKind::Power,
            [1100, 700, 1150, 400, 600],
            &c,
            Strategy::PaceChaser,
            apt,
        );
        assert!(low > high, "power value should taper past the knee ({low} vs {high})");
    }

    #[test]
    fn wit_marginal_positive_with_no_hard_cap() {
        let c = course(2000.0, Surface::Turf, vec![]);
        let apt = Aptitudes {
            distance_grade: 7,
            surface_grade: 7,
        };
        let mid = stat_marginal_value(StatKind::Wit, [1100, 700, 800, 400, 800], &c, Strategy::PaceChaser, apt);
        let high = stat_marginal_value(
            StatKind::Wit,
            [1100, 700, 800, 400, 1400],
            &c,
            Strategy::PaceChaser,
            apt,
        );
        assert!(mid > 0.0 && high > 0.0, "wit always has positive value");
        assert!(mid > high, "wit has diminishing (not zero) returns");
    }

    #[test]
    fn guts_is_minor_relative_to_speed_and_stamina() {
        let c = course(2400.0, Surface::Turf, vec![]);
        let apt = Aptitudes {
            distance_grade: 7,
            surface_grade: 7,
        };
        let cur = [1000, 500, 800, 400, 600];
        let guts = stat_marginal_value(StatKind::Guts, cur, &c, Strategy::LateSurger, apt);
        let speed = stat_marginal_value(StatKind::Speed, cur, &c, Strategy::LateSurger, apt);
        assert!(guts > 0.0);
        assert!(guts < speed, "guts should be a minor stat vs speed ({guts} vs {speed})");
    }
}
