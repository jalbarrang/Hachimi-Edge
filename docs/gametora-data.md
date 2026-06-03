# GameTora data pipeline

Game catalog data — **skills**, **support cards** (incl. hint skills + training/
chain events), **character cards** (umas with outfits), plus `support_effects` and
the reward/event dictionaries — extracted from GameTora with a local tool,
committed to this repo, and downloaded by clients from the repo's raw GitHub URL.

> **End users never hit GameTora.** A maintainer extracts the catalog once and
> commits it; every client pulls the committed copy (CDN-backed by GitHub/Fastly).
> This avoids a distributed load spike on GameTora from thousands of plugin installs.

## Flow

```
GameTora manifest ──► tools/gametora-sync (local) ──► data/gametora/*.json + manifest.json ──► git commit
                                                          │
                          client core::gametora_data ◄────┘  (raw.githubusercontent.com)
                                  │
                                  ▼
                       <game data dir>/gametora/*.json ──► training-tracker plugin
```

1. **Extract (local).** A maintainer runs `tools/gametora-sync`, which reads
   GameTora's content-hash manifest (`gametora.com/data/manifests/umamusume.json`),
   downloads the in-scope keyed snapshots (`{key}.{hash}.json`), validates them, and
   writes `data/gametora/<file>.json`. It owns the GameTora **key → filename** mapping.
2. **Publish (commit).** It also recomputes `data/gametora/manifest.json` — our own
   `{ generated_at, source, files: { filename: blake3 } }`. The maintainer commits
   `data/gametora/` to `main`.
3. **Download (client).** On launch, `Hachimi::run_auto_update_check` kicks
   `gametora_updater.sync(false)` (gated by `disable_auto_update_check` /
   `disable_gametora_data`). It fetches our hosted `manifest.json`, diffs each
   `filename → hash` against the local cache, and downloads only changed files.
   A manual **"Sync GameTora data"** button (About tab) calls `sync(true)`.
4. **Cache.** Snapshots are stored **verbatim** (uma-sim ADR-0002, zero transform)
   under `<game data dir>/gametora/`, with `.gametora_cache.json` tracking the
   last-synced hash per filename. Per-file download failures are non-fatal and
   retried next launch.
5. **Consume (plugin).** training-tracker resolves the cache dir via the host
   `host_data_path` service (API v10, capability `DATA_PATHS`) and reads the JSON.
   See `plugins/training-tracker/src/gametora_data.rs`.

## Hosted layout (`data/gametora/`)

| File | Contents |
| --- | --- |
| `manifest.json` | `{ generated_at, source, files: { filename: blake3 } }` |
| `skills.json` | skill catalog (conditions/effects, `loc` overrides) |
| `character-cards.json` | trainee cards (umas + outfits) |
| `support-cards.json` | support cards (incl. `event_skills`, `hints`) |
| `support-effects.json` | support effect tables |
| `training-events-{ssr,sr,shared,friend,group}.json` | training/chain event trees |
| `evrew.json`, `te-names-en.json`, `te-names-ja.json` | reward / event-name dictionaries |

## Refreshing the data

When GameTora updates (new game version / cards / skills), refresh and commit:

```bash
cargo run -p gametora-sync -- data/gametora
git add data/gametora && git commit -m "chore(data): sync GameTora snapshots"
```

Idempotent: only files whose content changed are rewritten; `manifest.json` always
refreshes its hashes + timestamp. Once pushed to `main`, clients pick up the change
on their next sync.

## Code map

| Location | Role |
| --- | --- |
| `tools/gametora-sync` | local extractor: GameTora → `data/gametora/*.json` + `manifest.json` |
| `apps/hachimi/src/core/gametora_data/client.rs` | hosted manifest + per-file fetch |
| `apps/hachimi/src/core/gametora_data/cache.rs` | cache manifest + filename sanitization |
| `apps/hachimi/src/core/gametora_data/updater.rs` | hash-diff orchestration + GUI notifications |
| `apps/hachimi/src/core/hachimi/config.rs` | `disable_gametora_data`, `gametora_data_url` override |
| `crates/hachimi-plugin-abi` | `host_data_path` slot, `DATA_PATHS` cap, `GAMETORA_DATA_SUBDIR` |
| `crates/hachimi-plugin-sdk/src/sdk.rs` | `Sdk::host_data_path` / `gametora_data_dir` |
| `plugins/training-tracker/src/gametora_data.rs` | typed loaders + raw event/dict access |

## Config

| Key | Default | Purpose |
| --- | --- | --- |
| `disable_gametora_data` | `false` | Turn the catalog sync off entirely |
| `gametora_data_url` | hosted repo URL | Override the download base (dev/testing) |

## Notes

- **No deploy step.** The client cache is runtime-managed in the data dir; unlike
  `skill_grades.json`, nothing is copied at deploy time.
- **Typed vs raw.** Skills, support cards, and character cards are typed in the
  plugin (unknown fields ignored). The irregular training-event trees and encoded
  `evrew` / `te_names` dictionaries are exposed as raw JSON.
- **`loc.en` / `groupId`.** As-is storage means Global overrides and skill-family
  grouping are resolved by the consumer at load time (mirrors uma-sim
  `SkillService`); not yet implemented in the plugin.
- **History churn.** The committed `skills.json` is ~2 MB and changes on game
  updates; this is an accepted tradeoff for simple raw-GitHub URLs. Could move to
  an orphan branch or release asset later if it becomes a problem.
