//! skill-grades generator (maintainer tool).
//!
//! Builds the training-tracker evaluation resource `skill_grades.json` by joining:
//!   1. master.mdb `skill_data` (grade_value + rarity) + `text_data` (English skill
//!      names, category 47), read via rusqlite.
//!   2. UmaTools `uma_skills.csv` `affinity_role` per skill, matched by name.
//!
//! Output: `{ "<id>": { "g": <grade>, "r"?: "<role>", "u"?: 1 } }` (sorted by id).
//!
//! Rust replacement for the old `scripts/gen-skill-grades.mjs` (Node) — no uma-sim
//! dependency: it reads master.mdb directly (fetch one with `fetch-master-db`).
//!
//! Usage: `skill-grades [--master db/master.mdb] [--out <path>] [--csv <path-or-url>]`

use std::collections::{BTreeMap, HashMap};
use std::process::ExitCode;

use serde::Serialize;

const DEFAULT_MASTER: &str = "db/master.mdb";
const DEFAULT_OUT: &str = "plugins/training-tracker/assets/skill_grades.json";
const DEFAULT_CSV: &str = "https://raw.githubusercontent.com/daftuyda/UmaTools/main/assets/uma_skills.csv";

/// One skill's evaluation inputs (matches the plugin's `SkillGrade`).
#[derive(Serialize)]
struct Grade {
    g: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    r: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    u: Option<u8>,
}

/// Build `name(lowercased) -> affinity_role(lowercased)` from the UmaTools CSV,
/// indexing all three name columns (English name/localized + JP alias).
fn load_roles(csv_bytes: &[u8]) -> Result<HashMap<String, String>, String> {
    let mut rdr = csv::ReaderBuilder::new().flexible(true).from_reader(csv_bytes);
    let headers = rdr.headers().map_err(|e| format!("CSV header: {e}"))?.clone();
    let col = |name: &str| headers.iter().position(|h| h == name);
    let (name_i, alias_i, loc_i, role_i) = (
        col("name"),
        col("alias_name"),
        col("localized_name"),
        col("affinity_role").ok_or("CSV missing 'affinity_role' column")?,
    );

    let mut roles = HashMap::new();
    for rec in rdr.records() {
        let rec = rec.map_err(|e| format!("CSV row: {e}"))?;
        let role = rec.get(role_i).unwrap_or("").trim();
        if role.is_empty() {
            continue;
        }
        for idx in [name_i, alias_i, loc_i].into_iter().flatten() {
            let n = rec.get(idx).unwrap_or("").trim();
            if !n.is_empty() {
                roles.insert(n.to_lowercase(), role.to_lowercase());
            }
        }
    }
    Ok(roles)
}

fn load_csv(source: &str) -> Result<Vec<u8>, String> {
    if source.starts_with("http://") || source.starts_with("https://") {
        let mut reader = ureq::Agent::new_with_defaults()
            .get(source)
            .call()
            .map_err(|e| format!("GET {source}: {e}"))?
            .into_body()
            .into_reader();
        let mut buf = Vec::new();
        std::io::Read::read_to_end(&mut reader, &mut buf).map_err(|e| format!("reading CSV: {e}"))?;
        Ok(buf)
    } else {
        std::fs::read(source).map_err(|e| format!("reading {source}: {e}"))
    }
}

fn run(master: &str, out: &str, csv_src: &str) -> Result<(), String> {
    let roles = load_roles(&load_csv(csv_src)?)?;
    eprintln!("Loaded {} role mappings from CSV", roles.len());

    let db = rusqlite::Connection::open_with_flags(master, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|e| format!("opening {master}: {e}"))?;

    // English skill names: text_data category 47, [index] = skill id.
    let mut names: HashMap<i64, String> = HashMap::new();
    {
        let mut stmt = db
            .prepare("SELECT \"index\", text FROM text_data WHERE category = 47")
            .map_err(|e| format!("name query: {e}"))?;
        let rows = stmt
            .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))
            .map_err(|e| format!("name query: {e}"))?;
        for row in rows {
            let (id, name) = row.map_err(|e| format!("name row: {e}"))?;
            names.insert(id, name);
        }
    }
    eprintln!("Loaded {} skill names from master.mdb", names.len());

    let mut out_map: BTreeMap<i64, Grade> = BTreeMap::new();
    let mut with_role = 0usize;
    let mut uniques = 0usize;
    {
        let mut stmt = db
            .prepare("SELECT id, rarity, grade_value FROM skill_data")
            .map_err(|e| format!("skill query: {e}"))?;
        let rows = stmt
            .query_map([], |r| {
                Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?, r.get::<_, i64>(2)?))
            })
            .map_err(|e| format!("skill query: {e}"))?;
        for row in rows {
            let (id, rarity, grade_value) = row.map_err(|e| format!("skill row: {e}"))?;
            let role = names
                .get(&id)
                .and_then(|n| roles.get(&n.trim().to_lowercase()))
                .cloned();
            if role.is_some() {
                with_role += 1;
            }
            let u = if rarity >= 3 {
                uniques += 1;
                Some(1)
            } else {
                None
            };
            out_map.insert(
                id,
                Grade {
                    g: grade_value,
                    r: role,
                    u,
                },
            );
        }
    }

    let json = serde_json::to_string(&out_map).map_err(|e| format!("serialize: {e}"))?;
    if let Some(parent) = std::path::Path::new(out).parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("mkdir {}: {e}", parent.display()))?;
    }
    std::fs::write(out, &json).map_err(|e| format!("writing {out}: {e}"))?;

    eprintln!(
        "Wrote {} skills ({} with role, {} unique) -> {}",
        out_map.len(),
        with_role,
        uniques,
        out
    );
    Ok(())
}

fn main() -> ExitCode {
    let mut master = DEFAULT_MASTER.to_owned();
    let mut out = DEFAULT_OUT.to_owned();
    let mut csv_src = DEFAULT_CSV.to_owned();

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--master" | "-m" => master = args.next().unwrap_or(master),
            "--out" | "-o" => out = args.next().unwrap_or(out),
            "--csv" | "-c" => csv_src = args.next().unwrap_or(csv_src),
            "--help" | "-h" => {
                eprintln!("usage: skill-grades [--master db/master.mdb] [--out <path>] [--csv <path-or-url>]");
                return ExitCode::SUCCESS;
            }
            other => {
                eprintln!("error: unexpected argument '{other}'");
                return ExitCode::FAILURE;
            }
        }
    }

    match run(&master, &out, &csv_src) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
