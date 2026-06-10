# Updating game data (skills, umas, support cards)

When the global (EN) server of the Honse game ships an update, two distinct data
sources need refreshing. Run every command from the repo root.

## Sources

| Source | Provides | Tools |
| --- | --- | --- |
| `master.mdb` (game CDN) | skill grades/roles, course params | `fetch-master-db`, `skill-grades`, `course-data` |
| GameTora | skill / uma (character-card) / support-card listings, effects, events | `gametora-sync` |

> Note: the **uma** and **support card** listings do NOT come from `master.mdb`;
> they come from GameTora. The `master.mdb` only feeds `skill_grades.json` and
> `course_params.json`.

## Full process

```bash
# 1. Download a fresh master.mdb from the global server.
#    Resolves the version from uma.moe/api/ver, walks the CDN manifest chain and
#    writes db/master.mdb.
cargo run -p fetch-master-db
#    (optional) pin version / platform:
#    cargo run -p fetch-master-db -- <RESOURCE_VERSION> --out db --platform Windows

# 2. Regenerate the master.mdb-derived assets.
cargo run -p skill-grades   # -> plugins/training-tracker/assets/skill_grades.json
cargo run -p course-data    # -> plugins/training-tracker/assets/course_params.json

# 3. Publish those assets for hosted download (data/ + data/manifest.json).
cargo run -p tracker-data-manifest

# 4. Refresh the GameTora catalogs (skills, umas, support cards, events…).
#    Writes data/gametora/ + data/gametora/manifest.json.
cargo run -p gametora-sync
```

## After

Commit the generated files:

- `db/master.mdb` (if you version it)
- `plugins/training-tracker/assets/skill_grades.json`
- `plugins/training-tracker/assets/course_params.json`
- `data/skill_grades.json`, `data/course_params.json`, `data/manifest.json`
- `data/gametora/*.json` + `data/gametora/manifest.json`

Clients download the committed `data/...` files from the repo's raw GitHub URL;
they are never embedded in any binary nor attached to a release.

## Notes

- `fetch-master-db`, `skill-grades` and `course-data` need the `master.mdb` on
  disk, so they **don't run in CI** — the maintainer runs them manually.
- `gametora-sync` fails fast (non-zero exit) on any fetch/parse error so a
  partial/inconsistent dataset is never committed.
