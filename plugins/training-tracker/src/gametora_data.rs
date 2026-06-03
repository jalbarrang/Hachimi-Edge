//! GameTora data catalog access.
//!
//! Reads the JSON snapshots the host caches under its data dir (`gametora/`,
//! downloaded by `core::gametora_data`). The directory is resolved at runtime via
//! the host `host_data_path` service (host API v10+, capability `DATA_PATHS`).
//!
//! Snapshots are stored verbatim in GameTora's upstream shape (uma-sim ADR-0002),
//! so the structs below model only the fields the plugin needs and ignore the
//! rest. Skills/support-cards/character-cards are typed; the irregular training
//! event trees and the encoded reward/name dictionaries are exposed as raw JSON.
//!
//! Everything degrades gracefully: a missing host capability, missing directory,
//! or missing/malformed file yields an empty catalog (logged once), never a panic.
//!
//! The catalog is a public API consumed incrementally by tracker features; until
//! every accessor has a call site, unused entries are allowed here.
#![allow(dead_code)]

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::OnceLock;

use hachimi_plugin_sdk::Sdk;
use serde::Deserialize;
use serde_json::Value;

/// GameTora stores skill-id arrays with mixed number/string entries
/// (e.g. `[200162, "201352"]`). Accept both and drop non-numeric values.
fn de_flexible_id_vec<'de, D>(d: D) -> Result<Vec<i64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum IdOrStr {
        Int(i64),
        Str(String),
    }
    let raw: Vec<IdOrStr> = Vec::deserialize(d)?;
    Ok(raw
        .into_iter()
        .filter_map(|x| match x {
            IdOrStr::Int(i) => Some(i),
            IdOrStr::Str(s) => s.parse::<i64>().ok(),
        })
        .collect())
}

// ── Typed entities ──────────────────────────────────────────────────────────

/// A skill entry (`skills.json`). `condition_groups` / `loc` are kept raw because
/// their shape varies and is only meaningful to a full simulator.
#[derive(Debug, Clone, Deserialize)]
pub struct Skill {
    pub id: i64,
    #[serde(default)]
    pub name_en: Option<String>,
    #[serde(default)]
    pub jpname: Option<String>,
    #[serde(default)]
    pub rarity: Option<i64>,
    #[serde(default)]
    pub iconid: Option<i64>,
    /// Skill type tags, e.g. `["nac"]`.
    #[serde(default)]
    pub r#type: Vec<String>,
    /// Inline condition/effect groups (JP top-level; Global overrides under `loc`).
    #[serde(default)]
    pub condition_groups: Value,
    /// Server-specific overrides (`loc.en` = Global).
    #[serde(default)]
    pub loc: Value,
}

/// A support card entry (`support-cards.json`).
#[derive(Debug, Clone, Deserialize)]
pub struct SupportCard {
    pub support_id: i64,
    #[serde(default)]
    pub char_id: Option<i64>,
    #[serde(default)]
    pub char_name: Option<String>,
    /// Rarity index (1=R, 2=SR, 3=SSR).
    #[serde(default)]
    pub rarity: Option<i64>,
    /// Card type, e.g. `"group"`, `"guts"`, `"speed"`.
    #[serde(default)]
    pub r#type: Option<String>,
    #[serde(default)]
    pub title_en: Option<String>,
    #[serde(default)]
    pub title_ja: Option<String>,
    #[serde(default)]
    pub url_name: Option<String>,
    /// Skill ids granted via this card's training events.
    #[serde(default, deserialize_with = "de_flexible_id_vec")]
    pub event_skills: Vec<i64>,
    /// Hint-skill payload (raw; nested by hint group).
    #[serde(default)]
    pub hints: Value,
    /// Per-level effect table (raw matrix).
    #[serde(default)]
    pub effects: Value,
}

/// A character (trainee) card entry (`character-cards.json`) — an outfit/costume.
#[derive(Debug, Clone, Deserialize)]
pub struct CharacterCard {
    pub card_id: i64,
    #[serde(default)]
    pub char_id: Option<i64>,
    /// Costume id (outfit).
    #[serde(default)]
    pub costume: Option<i64>,
    #[serde(default)]
    pub name_en: Option<String>,
    #[serde(default)]
    pub name_jp: Option<String>,
    #[serde(default)]
    pub rarity: Option<i64>,
    /// Global outfit title, when localized.
    #[serde(default)]
    pub title_en_gl: Option<String>,
    #[serde(default)]
    pub title_jp: Option<String>,
    #[serde(default, deserialize_with = "de_flexible_id_vec")]
    pub skills_unique: Vec<i64>,
    #[serde(default, deserialize_with = "de_flexible_id_vec")]
    pub skills_innate: Vec<i64>,
    #[serde(default, deserialize_with = "de_flexible_id_vec")]
    pub skills_event: Vec<i64>,
    #[serde(default, deserialize_with = "de_flexible_id_vec")]
    pub skills_awakening: Vec<i64>,
}

/// Which support-card rarity bucket a training-event file covers.
#[derive(Debug, Clone, Copy)]
pub enum EventKind {
    Ssr,
    Sr,
    Shared,
    Friend,
    Group,
}

impl EventKind {
    fn file(self) -> &'static str {
        match self {
            EventKind::Ssr => "training-events-ssr.json",
            EventKind::Sr => "training-events-sr.json",
            EventKind::Shared => "training-events-shared.json",
            EventKind::Friend => "training-events-friend.json",
            EventKind::Group => "training-events-group.json",
        }
    }
}

// ── Catalog (lazy, cached) ──────────────────────────────────────────────────

#[derive(Default)]
struct Catalog {
    skills: HashMap<i64, Skill>,
    support_cards: HashMap<i64, SupportCard>,
    character_cards: HashMap<i64, CharacterCard>,
}

static CATALOG: OnceLock<Catalog> = OnceLock::new();

/// Absolute path to the host's cached `gametora/` directory, if the host exposes it.
fn data_dir() -> Option<PathBuf> {
    Sdk::try_get().and_then(|sdk| sdk.gametora_data_dir())
}

/// Read + parse a snapshot file relative to the cache dir. `None` on any failure.
fn load_file<T: for<'de> Deserialize<'de>>(file: &str) -> Option<T> {
    let path = data_dir()?.join(file);
    let bytes = match std::fs::read(&path) {
        Ok(b) => b,
        Err(e) => {
            hlog_warn!(target: "training-tracker", "gametora_data: {} unavailable ({e})", path.display());
            return None;
        }
    };
    match serde_json::from_slice::<T>(&bytes) {
        Ok(v) => Some(v),
        Err(e) => {
            hlog_warn!(target: "training-tracker", "gametora_data: failed to parse {} ({e})", file);
            None
        }
    }
}

fn index_by<T: for<'de> Deserialize<'de>, F: Fn(&T) -> i64>(file: &str, key: F) -> HashMap<i64, T> {
    let items: Vec<T> = load_file(file).unwrap_or_default();
    items.into_iter().map(|it| (key(&it), it)).collect()
}

fn catalog() -> &'static Catalog {
    CATALOG.get_or_init(|| {
        if data_dir().is_none() {
            hlog_warn!(
                target: "training-tracker",
                "gametora_data: host does not expose a data path (API < v10 or capability missing); catalog empty"
            );
            return Catalog::default();
        }
        let catalog = Catalog {
            skills: index_by("skills.json", |s: &Skill| s.id),
            support_cards: index_by("support-cards.json", |c: &SupportCard| c.support_id),
            character_cards: index_by("character-cards.json", |c: &CharacterCard| c.card_id),
        };
        hlog_info!(
            target: "training-tracker",
            "gametora_data: loaded {} skills, {} support cards, {} character cards",
            catalog.skills.len(),
            catalog.support_cards.len(),
            catalog.character_cards.len()
        );
        catalog
    })
}

// ── Public accessors ────────────────────────────────────────────────────────

/// Look up a support card by its `support_card_id` (the id uma.moe reports).
#[must_use]
pub fn support_card(id: i64) -> Option<&'static SupportCard> {
    catalog().support_cards.get(&id)
}

/// Look up a skill by id.
#[must_use]
pub fn skill(id: i64) -> Option<&'static Skill> {
    catalog().skills.get(&id)
}

/// Look up a character (outfit) card by `card_id`.
#[must_use]
pub fn character_card(card_id: i64) -> Option<&'static CharacterCard> {
    catalog().character_cards.get(&card_id)
}

/// Whether any catalog data was loaded (i.e. the host cache is present).
#[must_use]
pub fn is_available() -> bool {
    let c = catalog();
    !c.skills.is_empty() || !c.support_cards.is_empty() || !c.character_cards.is_empty()
}

/// Raw training-event tree for a rarity bucket (GameTora's nested array form).
/// Returned uncached since these are large and rarely needed.
#[must_use]
pub fn training_events(kind: EventKind) -> Option<Value> {
    load_file(kind.file())
}

/// Raw reward/name dictionary by snapshot filename (e.g. `evrew.json`,
/// `te-names-en.json`, `te-names-ja.json`). Encoded upstream; consumer decodes.
#[must_use]
pub fn raw_dict(file: &str) -> Option<Value> {
    load_file(file)
}
