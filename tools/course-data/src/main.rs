//! course-data generator (maintainer tool).
//!
//! Builds the training-tracker CM resource `course_params.json` from master.mdb:
//! `race_course_set` gives distance, surface (ground), turn, finish times, and the
//! course's set-status id; `race_course_set_status` gives the set-status stat
//! thresholds (`target_status_N`).
//!
//! Output: `{ "<course_id>": { "distance", "surface", "turn", "thresholds",
//! "finish_time_min", "finish_time_max" } }` sorted by id. The shape mirrors the
//! plugin's `cm_model::CourseParams` so `crate::course_data` can deserialize it
//! directly (Surface/StatKind use their serde variant names: "Turf"/"Dirt",
//! "Speed"/"Stamina"/"Power"/"Guts"/"Wit").
//!
//! Same pattern as the `skill-grades` tool — reads master.mdb directly via
//! rusqlite (fetch one with `fetch-master-db`). No uma-sim dependency.
//!
//! Usage: `course-data [--master db/master.mdb] [--out <path>]`

use std::collections::{BTreeMap, HashMap};
use std::process::ExitCode;

use serde::Serialize;

const DEFAULT_MASTER: &str = "db/master.mdb";
const DEFAULT_OUT: &str = "plugins/training-tracker/assets/course_params.json";

/// One course's emitted parameters. Field names + enum string values must match
/// the plugin's `cm_model::CourseParams` serde representation.
#[derive(Serialize)]
struct CourseOut {
    distance: f64,
    surface: &'static str,
    turn: i64,
    thresholds: Vec<&'static str>,
    finish_time_min: f64,
    finish_time_max: f64,
}

/// master.mdb `ground` code → `cm_model::Surface` serde name.
fn surface_name(ground: i64) -> Option<&'static str> {
    match ground {
        1 => Some("Turf"),
        2 => Some("Dirt"),
        _ => None,
    }
}

/// master.mdb `target_status` code → `cm_model::StatKind` serde name.
fn stat_name(code: i64) -> Option<&'static str> {
    match code {
        1 => Some("Speed"),
        2 => Some("Stamina"),
        3 => Some("Power"),
        4 => Some("Guts"),
        5 => Some("Wit"),
        _ => None,
    }
}

fn run(master: &str, out: &str) -> Result<(), String> {
    let db = rusqlite::Connection::open_with_flags(master, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|e| format!("opening {master}: {e}"))?;

    // Set-status id → threshold stat codes (target_status_2 == 0 means "none").
    let mut thresholds: HashMap<i64, Vec<i64>> = HashMap::new();
    {
        let mut stmt = db
            .prepare("SELECT course_set_status_id, target_status_1, target_status_2 FROM race_course_set_status")
            .map_err(|e| format!("set-status query: {e}"))?;
        let rows = stmt
            .query_map([], |r| {
                Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?, r.get::<_, i64>(2)?))
            })
            .map_err(|e| format!("set-status query: {e}"))?;
        for row in rows {
            let (id, t1, t2) = row.map_err(|e| format!("set-status row: {e}"))?;
            let mut list = Vec::new();
            if t1 != 0 {
                list.push(t1);
            }
            if t2 != 0 {
                list.push(t2);
            }
            thresholds.insert(id, list);
        }
    }
    eprintln!("Loaded {} course set-status rows from master.mdb", thresholds.len());

    let mut out_map: BTreeMap<i64, CourseOut> = BTreeMap::new();
    let mut skipped = 0usize;
    {
        let mut stmt = db
            .prepare(
                "SELECT id, distance, ground, turn, course_set_status_id, finish_time_min, finish_time_max \
                 FROM race_course_set",
            )
            .map_err(|e| format!("course query: {e}"))?;
        let rows = stmt
            .query_map([], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, f64>(1)?,
                    r.get::<_, i64>(2)?,
                    r.get::<_, i64>(3)?,
                    r.get::<_, i64>(4)?,
                    r.get::<_, f64>(5)?,
                    r.get::<_, f64>(6)?,
                ))
            })
            .map_err(|e| format!("course query: {e}"))?;
        for row in rows {
            let (id, distance, ground, turn, set_status_id, ft_min, ft_max) =
                row.map_err(|e| format!("course row: {e}"))?;
            let Some(surface) = surface_name(ground) else {
                skipped += 1;
                continue;
            };
            let thresholds = thresholds
                .get(&set_status_id)
                .map(|codes| codes.iter().filter_map(|&c| stat_name(c)).collect())
                .unwrap_or_default();
            out_map.insert(
                id,
                CourseOut {
                    distance,
                    surface,
                    turn,
                    thresholds,
                    finish_time_min: ft_min,
                    finish_time_max: ft_max,
                },
            );
        }
    }

    let json = serde_json::to_string_pretty(&out_map).map_err(|e| format!("serialize: {e}"))?;
    if let Some(parent) = std::path::Path::new(out).parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("mkdir {}: {e}", parent.display()))?;
    }
    std::fs::write(out, &json).map_err(|e| format!("writing {out}: {e}"))?;

    eprintln!(
        "Wrote {} courses ({skipped} skipped: unknown surface) -> {}",
        out_map.len(),
        out
    );
    if out_map.is_empty() {
        return Err("no courses written (empty master.mdb or schema mismatch)".to_owned());
    }
    Ok(())
}

fn main() -> ExitCode {
    let mut master = DEFAULT_MASTER.to_owned();
    let mut out = DEFAULT_OUT.to_owned();

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--master" | "-m" => master = args.next().unwrap_or(master),
            "--out" | "-o" => out = args.next().unwrap_or(out),
            "--help" | "-h" => {
                eprintln!("usage: course-data [--master db/master.mdb] [--out <path>]");
                return ExitCode::SUCCESS;
            }
            other => {
                eprintln!("error: unexpected argument '{other}'");
                return ExitCode::FAILURE;
            }
        }
    }

    match run(&master, &out) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
