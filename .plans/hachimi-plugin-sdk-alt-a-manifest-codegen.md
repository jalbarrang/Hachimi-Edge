# Alternative A — Manifest-driven codegen (`plugin-api` schema)

**Codename**: `manifest-codegen`  
**Status**: Alternative to [.plans/hachimi-plugin-sdk.md](hachimi-plugin-sdk.md)  
**Date**: 2026-05-23

Phases are sequential checkpoints. After each: `cargo build`, `cargo test`, `cargo clippy`.

---

## 1. Idea

Instead of hand-maintaining `Vtable` in Rust in two places (host + plugin), maintain a **single machine-readable manifest** that generates:

- `crates/hachimi-plugin-sdk/src/abi.generated.rs` — `#[repr(C)] Vtable`, opaque types, `API_VERSION`
- `crates/hachimi-plugin-sdk/src/version.generated.rs` — `ApiVersion::supports_*()` from per-slot `since: N` metadata
- Optional: a markdown fragment for `docs/reverse-engineering/hachimi-plugin-surface.md`

Host `api.rs` still implements `extern "C"` functions manually, but **cannot drift** from the schema: CI fails if `Vtable::VALUE` initializer field count ≠ manifest slot count.

---

## 2. Manifest shape (example)

`tools/plugin-api/manifest.toml`:

```toml
api_version = 7

[[slot]]
name = "hachimi_instance"
since = 1
ret = "*const Hachimi"
args = []

[[slot]]
name = "gui_register_overlay"
since = 3
ret = "bool"
args = ["*const c_char", "Option<GuiMenuSectionCallback>", "*mut c_void"]
# ...
```

`build.rs` in `hachimi-plugin-sdk` reads manifest, emits Rust. Host adds a **dev-dependency** or `xtask check-plugin-api` that parses the same manifest and asserts host `api.rs` contains each `name` as a function (grep/regex check, not full codegen of host).

---

## 3. Goals

1. **Drift is impossible** for struct layout and slot order — generated `Vtable`.
2. **Version gates derived** from `since` on each slot — no magic `>= 5` in plugins.
3. **Docs stay in sync** — optional generated table in plugin surface doc.
4. Same runtime ABI as today — no wire change.

## 4. Non-goals

- Generating host `extern "C"` bodies (still hand-written against il2cpp/egui).
- Proc-macros for `hachimi_init`.
- crates.io publish.

---

## 5. Layout

```
tools/plugin-api/manifest.toml
crates/hachimi-plugin-sdk/
  build.rs
  src/
    lib.rs          # mod abi; include!(abi.generated.rs) pattern
    sdk.rs          # safe wrappers (same as baseline plan)
    init.rs
```

Workspace + host path-dep same as baseline.

---

## 6. Phases

| Phase | Work |
|-------|------|
| 0 | Workspace; manifest with all 57 slots transcribed from `api.rs`; codegen emits `Vtable` + tests |
| 1 | Host imports generated types; `xtask`/test: manifest names ⊆ host fn exports |
| 2 | Plugin drops `vtable.rs`; uses SDK |
| 3 | Generated `ApiVersion` + `Sdk` |
| 4 | Safe wrappers (manual in `sdk.rs`, not generated) |
| 5 | Docs; optional `cargo xtask gen-plugin-docs` |

**Extra phase cost**: ~4–6 h to build manifest + `build.rs` + CI check vs baseline.

---

## 7. Tradeoffs

| Pros | Cons |
|------|------|
| Best long-term ABI discipline | Highest upfront complexity |
| Adding slot 58 = edit manifest + host fn + bump version — checklist is explicit | Manifest syntax is another language to learn |
| Auto-generated version helpers | Host impls still manual — manifest can lie if check is weak |
| Scales to many plugins | Overkill for one plugin today |

---

## 8. When to pick this

Choose **A** if you expect **3+ plugins** or frequent API additions and want CI to catch layout mistakes before runtime.

---

## 9. Effort

~18–24 h total (baseline ~12–18 h + codegen infrastructure).
