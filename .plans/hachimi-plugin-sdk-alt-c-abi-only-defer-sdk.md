# Alternative C — ABI-only crate; defer `Sdk` and safe wrappers

**Codename**: `abi-only-minimal`  
**Status**: Alternative to [.plans/hachimi-plugin-sdk.md](hachimi-plugin-sdk.md)  
**Date**: 2026-05-23

Phases are sequential checkpoints. After each: `cargo build`, `cargo test`, `cargo clippy`.

---

## 1. Idea

Ship the **smallest possible fix** for the actual bug: duplicated `Vtable` and manual sync.

- One crate: `hachimi-plugin-abi` (or name it `hachimi-plugin-sdk` but **only** contains ABI + `vt()` + `hlog!` — no `Sdk`, no `gui_small`, no domain modules).
- Host + training-tracker depend on it; delete `vtable.rs`.
- **Stop there.** Keep ~138 `(vt.slot)(...)` call sites unchanged.
- Document follow-up issue: “plugin Sdk ergonomics” when pain justifies it.

Optionally move **version constants** into abi crate as plain `pub const OVERLAY_API: i32 = 3` — no `ApiVersion` struct yet.

---

## 2. Goals

1. Single `Vtable` definition — **done in ~1 day**.
2. Zero behavior change in plugin logic — mechanical path/import swap.
3. No new abstractions to learn before shipping.
4. Workspace for future sdk crate without building it now.

## 3. Explicit deferrals

- `Sdk` struct and `Sdk::get()`
- Safe `gui_small(&str)` wrappers
- `il2cpp::resolve_class` helpers
- Proc-macro `#[hachimi_plugin]`
- Manifest codegen

---

## 4. Layout

```
crates/hachimi-plugin-abi/   # or hachimi-plugin-sdk with minimal lib.rs
  src/
    lib.rs      # Vtable, types, set_vtable, vt, hlog macros
    version.rs  # pub const API_VERSION = 7; pub const MIN_FOR_OVERLAY: i32 = 3; ...
```

```
plugins/training-tracker/
  # no vtable.rs
  # still: (vt().gui_ui_small)(ui, cstr.as_ptr()) everywhere
```

---

## 5. Phases

| Phase | Work |
|-------|------|
| 0 | Workspace + abi crate + layout tests |
| 1 | Host imports abi types; opaque signatures + casts |
| 2 | Plugin path dep; delete `vtable.rs`; mechanical `crate::vtable` → `hachimi_plugin_abi` |
| 3 | Docs + stale `hachimi-plugin-surface.md` version fix |
| **Stop** | |

Optional phase 4 (later): add `hachimi-plugin-sdk` crate per baseline plan — **separate effort**, not part of this alternative.

---

## 6. Tradeoffs

| Pros | Cons |
|------|------|
| Lowest risk, smallest diff | Plugin code still ugly (`unsafe` + CString at every call) |
| Fastest time to “never drift again” | Version checks stay ad-hoc in `ui.rs` |
| No over-engineering | Second migration later if you add Sdk wrappers |
| Easy to review | Doesn’t address ergonomics pain training-tracker already has |

---

## 7. When to pick this

Choose **C** if the priority is **unblock correctness now** and you’re fine punting DX until training-tracker (or a second plugin) hurts enough to justify baseline phases 3–4.

---

## 8. Effort

~6–10 h total (phases 0–3 only).

---

## 9. Upgrade path

```
C (abi-only) ──► B (add sdk crate on top of abi) ──► optional A (manifest codegen on abi)
```

C is not a dead end; it’s the first slice of B without the second crate.
