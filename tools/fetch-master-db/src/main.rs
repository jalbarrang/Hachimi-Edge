//! master.mdb fetcher (CI/maintainer tool).
//!
//! Resolves the current resource version from uma.moe, then walks the Umamusume
//! CDN manifest chain (root -> platform -> master -> master.mdb), decompressing
//! LZ4 and parsing the BSV manifests, and writes `db/master.mdb`.
//!
//! Port of uma-sim's `scripts/master-data/fetch-master-db.ts`.
//!
//! Usage: `fetch-master-db [RESOURCE_VERSION] [--out DIR] [--platform Windows]`
//! Default version is resolved from `https://uma.moe/api/ver`.

mod bsv;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use sha1::{Digest, Sha1};

const BASE_URL: &str = "https://assets-umamusume-en.akamaized.net";
const VERSION_URL: &str = "https://uma.moe/api/ver";
const USER_AGENT: &str = "UnityPlayer/2022.3.46f1 (UnityWebRequest/1.0, libcurl/8.5.0-DEV)";
const BASE32_ALPHABET: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
const DEFAULT_OUT: &str = "db";
const DEFAULT_PLATFORM: &str = "Windows";

struct Entry {
    name: String,
    size: u64,
    checksum: u64,
}

impl Entry {
    fn hname(&self) -> String {
        calc_hname(self.checksum, self.size, self.name.as_bytes())
    }
}

/// base32(SHA1( u64_BE(checksum) ++ u64_BE(size) ++ name )).
fn calc_hname(checksum: u64, size: u64, name: &[u8]) -> String {
    let mut hasher = Sha1::new();
    hasher.update(checksum.to_be_bytes());
    hasher.update(size.to_be_bytes());
    hasher.update(name);
    base32(&hasher.finalize())
}

fn base32(bytes: &[u8]) -> String {
    let mut out = String::new();
    let mut value: u32 = 0;
    let mut bits = 0u32;
    for &b in bytes {
        value = (value << 8) | u32::from(b);
        bits += 8;
        while bits >= 5 {
            out.push(BASE32_ALPHABET[((value >> (bits - 5)) & 0x1F) as usize] as char);
            bits -= 5;
        }
    }
    if bits > 0 {
        out.push(BASE32_ALPHABET[((value << (5 - bits)) & 0x1F) as usize] as char);
    }
    out
}

fn agent() -> ureq::Agent {
    ureq::Agent::new_with_defaults()
}

fn download(agent: &ureq::Agent, url: &str) -> Result<Vec<u8>, String> {
    let mut reader = agent
        .get(url)
        .header("User-Agent", USER_AGENT)
        .header("Accept", "*/*")
        .header("Accept-Encoding", "identity")
        .call()
        .map_err(|e| format!("GET {url} failed: {e}"))?
        .into_body()
        .into_reader();
    let mut buf = Vec::new();
    std::io::Read::read_to_end(&mut reader, &mut buf).map_err(|e| format!("reading {url}: {e}"))?;
    Ok(buf)
}

fn resolve_version(agent: &ureq::Agent, explicit: Option<&str>) -> Result<String, String> {
    if let Some(v) = explicit {
        if !v.trim().is_empty() {
            return Ok(v.trim().to_owned());
        }
    }
    let body = agent
        .get(VERSION_URL)
        .header("Accept", "application/json")
        .call()
        .map_err(|e| format!("GET {VERSION_URL}: {e}"))?
        .into_body()
        .read_to_string()
        .map_err(|e| format!("reading version: {e}"))?;
    let json: serde_json::Value = serde_json::from_str(&body).map_err(|e| format!("parsing version json: {e}"))?;
    // Response is either { resource_version, ... } or { current: { resource_version } }.
    let rv = json
        .get("resource_version")
        .or_else(|| json.get("current").and_then(|c| c.get("resource_version")))
        .and_then(|v| v.as_str())
        .ok_or("resource_version missing from uma.moe response")?;
    Ok(rv.to_owned())
}

fn manifest_url(hname: &str) -> String {
    format!("{BASE_URL}/dl/vertical/resources/Manifest/{}/{}", &hname[0..2], hname)
}

fn generic_url(hname: &str) -> String {
    format!("{BASE_URL}/dl/vertical/resources/Generic/{}/{}", &hname[0..2], hname)
}

/// Root rows: (platform, size, checksum). Hname is computed from the platform name.
fn parse_root(data: &[u8]) -> Result<Vec<Entry>, String> {
    Ok(bsv::parse_anonymous(data)?
        .into_iter()
        .filter(|row| row.len() >= 3)
        .filter_map(|row| {
            Some(Entry {
                name: row[0].as_str()?.to_owned(),
                size: row[1].as_u64()?,
                checksum: row[2].as_u64()?,
            })
        })
        .collect())
}

/// Content rows: prefer (name, row[4]=size, row[5]=checksum); fall back to (name, row[1], row[2]).
fn parse_content(data: &[u8]) -> Result<Vec<Entry>, String> {
    Ok(bsv::parse_anonymous(data)?
        .into_iter()
        .filter_map(|row| {
            let name = row.first()?.as_str()?.to_owned();
            if row.len() >= 7 {
                Some(Entry {
                    name,
                    size: row[4].as_u64()?,
                    checksum: row[5].as_u64()?,
                })
            } else if row.len() >= 3 {
                Some(Entry {
                    name,
                    size: row[1].as_u64()?,
                    checksum: row[2].as_u64()?,
                })
            } else {
                None
            }
        })
        .collect())
}

fn run(version: Option<&str>, out_dir: &Path, platform: &str) -> Result<PathBuf, String> {
    let agent = agent();
    let resource_version = resolve_version(&agent, version)?;
    eprintln!("Resource version: {resource_version}");
    std::fs::create_dir_all(out_dir).map_err(|e| format!("mkdir {}: {e}", out_dir.display()))?;

    // STEP 1: root manifest (always LZ4).
    let root_url = format!("{BASE_URL}/dl/vertical/{resource_version}/manifests/manifestdat/root.manifest.bsv.lz4");
    eprintln!("[1/4] root manifest: {root_url}");
    let root = bsv::decompress_lz4(&download(&agent, &root_url)?)?;
    let platform_entry = parse_root(&root)?
        .into_iter()
        .find(|e| e.name.eq_ignore_ascii_case(platform))
        .ok_or_else(|| format!("platform '{platform}' not found in root manifest"))?;

    // STEP 2: platform manifest (LZ4 only if framed).
    eprintln!("[2/4] platform manifest ({platform})");
    let mut platform_data = download(&agent, &manifest_url(&platform_entry.hname()))?;
    if bsv::is_lz4_frame(&platform_data) || looks_block_compressed(&platform_data) {
        platform_data = bsv::decompress_lz4(&platform_data)?;
    }
    let master_entry = parse_content(&platform_data)?
        .into_iter()
        .find(|e| e.name.eq_ignore_ascii_case("master"))
        .ok_or("'master' entry not found in platform manifest")?;

    // STEP 3: master manifest (LZ4 only if framed).
    eprintln!("[3/4] master manifest");
    let mut master_manifest = download(&agent, &manifest_url(&master_entry.hname()))?;
    if bsv::is_lz4_frame(&master_manifest) || looks_block_compressed(&master_manifest) {
        master_manifest = bsv::decompress_lz4(&master_manifest)?;
    }
    let mdb_entry = parse_content(&master_manifest)?
        .into_iter()
        .find(|e| e.name.to_lowercase().contains("master.mdb"))
        .ok_or("'master.mdb' entry not found in master manifest")?;

    // STEP 4: master.mdb (always LZ4).
    eprintln!("[4/4] master.mdb ({} bytes compressed target)", mdb_entry.size);
    let mdb = bsv::decompress_lz4(&download(&agent, &generic_url(&mdb_entry.hname()))?)?;
    let out_path = out_dir.join("master.mdb");
    std::fs::write(&out_path, &mdb).map_err(|e| format!("writing {}: {e}", out_path.display()))?;
    eprintln!(
        "Wrote {} ({:.2} MB)",
        out_path.display(),
        mdb.len() as f64 / (1024.0 * 1024.0)
    );
    Ok(out_path)
}

/// Heuristic for the size-prefixed raw-block path: BSV data starts with 0xBF, so
/// if byte 0 isn't the BSV magic the manifest is (block-)compressed.
fn looks_block_compressed(data: &[u8]) -> bool {
    !data.is_empty() && data[0] != 0xBF
}

fn main() -> ExitCode {
    let mut version: Option<String> = None;
    let mut out_dir = PathBuf::from(DEFAULT_OUT);
    let mut platform = DEFAULT_PLATFORM.to_owned();

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--out" | "-o" => out_dir = PathBuf::from(args.next().unwrap_or_default()),
            "--platform" | "-p" => platform = args.next().unwrap_or_else(|| DEFAULT_PLATFORM.to_owned()),
            "--help" | "-h" => {
                eprintln!("usage: fetch-master-db [RESOURCE_VERSION] [--out DIR] [--platform Windows]");
                return ExitCode::SUCCESS;
            }
            v if !v.starts_with('-') && version.is_none() => version = Some(v.to_owned()),
            other => {
                eprintln!("error: unexpected argument '{other}'");
                return ExitCode::FAILURE;
            }
        }
    }

    match run(version.as_deref(), &out_dir, &platform) {
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
