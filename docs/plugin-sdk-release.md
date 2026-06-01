# Plugin SDK Release & Versioning

The plugin SDK crates are **not published to crates.io**. They are distributed via
**git tags** on this repository, so third-party plugins pin a SDK version with a Cargo
`git` + `tag` dependency. All three crates carry `publish = false` to block accidental
registry pushes.

Crates in the SDK surface:

- `hachimi-plugin-abi` — raw C ABI types, event/capability/log constants, `hlog_*` macros.
- `hachimi-plugin-sdk` — safe `Sdk` wrapper + `egui` re-export (the crate plugins depend on).
- `hachimi-plugin-macros` — the `#[hachimi_plugin]` proc-macro (pulled in via the SDK).

## Tag scheme

SDK releases use a dedicated tag prefix, **decoupled** from the host mod's `v0.25.x`
release tags:

```
sdk-vMAJOR.MINOR.PATCH      e.g. sdk-v0.1.0
```

- Bump **PATCH** for backwards-compatible SDK wrapper fixes (no ABI change).
- Bump **MINOR** when the host API version (`hachimi_plugin_abi::API_VERSION`) gains
  new, additive vtable slots/events (older plugins still load).
- Bump **MAJOR** for breaking ABI changes (vtable layout changes, removed slots).

The three crate `version` fields move together (they are released as a set).

## Release ritual

1. Land the SDK changes on the default branch.
2. Bump `version` in lockstep in all three crate manifests:
   - `crates/hachimi-plugin-abi/Cargo.toml`
   - `crates/hachimi-plugin-sdk/Cargo.toml`
   - `crates/hachimi-plugin-macros/Cargo.toml`
   (Keep the intra-SDK `version =` requirements next to their `path =` deps in sync.)
3. If the C ABI changed, bump `API_VERSION` in `crates/hachimi-plugin-abi/src/version.rs`.
4. Commit, then tag and push:
   ```sh
   git tag sdk-v0.1.0
   git push origin sdk-v0.1.0
   ```
5. (Optional) Cut a matching GitHub release pointing at the tag with notes on the
   minimum compatible host build.

## Consuming the SDK (downstream plugins)

Plugins depend on the crates by git tag. `hachimi-plugin-sdk` is required;
`hachimi-plugin-abi` is needed directly only for the `hlog_*` macros and the
event/capability constants:

```toml
[dependencies]
hachimi-plugin-sdk = { git = "https://github.com/jalbarrang/Hachimi-Edge", tag = "sdk-v0.1.0" }
hachimi-plugin-abi = { git = "https://github.com/jalbarrang/Hachimi-Edge", tag = "sdk-v0.1.0" }
```

Cargo resolves the sibling `path` dependencies inside the git checkout automatically, so
no extra `[patch]` entries are required.

### egui version must match the host

`hachimi-plugin-sdk` re-exports `egui` and plugins MUST draw through
`hachimi_plugin_sdk::egui`. The host pins egui to a specific git revision at release
build time (see `.github/workflows/create_release.yml`). A plugin built against a
mismatched egui can break the shared-`Ui` FFI. When releasing an SDK tag, document the
egui version/revision it is built against and keep the SDK's `egui` dependency aligned
with the host.
