# Plugin SDK plans — comparison & vote summary

**Date**: 2026-05-23  
**Voters**: 3 agents (maintainer, plugin author, ABI/FFI engineer)

| Plan | File | One-line |
|------|------|----------|
| **Baseline** | [hachimi-plugin-sdk.md](hachimi-plugin-sdk.md) | Single `hachimi-plugin-sdk` crate: ABI + phased `Sdk` / safe wrappers |
| **Alt A** | [hachimi-plugin-sdk-alt-a-manifest-codegen.md](hachimi-plugin-sdk-alt-a-manifest-codegen.md) | `manifest.toml` + `build.rs` generates `Vtable` and version gates |
| **Alt B** | [hachimi-plugin-sdk-alt-b-split-abi-sdk.md](hachimi-plugin-sdk-alt-b-split-abi-sdk.md) | `hachimi-plugin-abi` (required) + `hachimi-plugin-sdk` (optional ergonomics) |
| **Alt C** | [hachimi-plugin-sdk-alt-c-abi-only-defer-sdk.md](hachimi-plugin-sdk-alt-c-abi-only-defer-sdk.md) | ABI crate only; delete mirror; defer all wrappers |

---

## Agent rankings

| Voter | #1 | #2 | #3 | #4 |
|-------|----|----|----|-----|
| Maintainer | Baseline | Alt B | Alt C | Alt A |
| Plugin author | Baseline | Alt A | Alt B | Alt C |
| ABI engineer | Alt A | Alt B | Alt C | Baseline |

**Explicit recommendations**: Baseline ×2, Alt A ×1 (ABI engineer: “A or B if deferring codegen”).

---

## Borda score (3 pts / 2 / 1 / 0 for ranks 1–4)

| Plan | Score |
|------|-------|
| Baseline | **6** |
| Alt B | **6** (tie) |
| Alt A | **5** |
| Alt C | **2** |

---

## Synthesized recommendation → **implemented in consolidated plan**

**Canonical doc**: [hachimi-plugin-sdk-consolidated.md](hachimi-plugin-sdk-consolidated.md)

Hybrid adopted: **Alt B** (`hachimi-plugin-abi`, Phases 0–2) + **Baseline** (`hachimi-plugin-sdk`, Phases 3–4). Alt A deferred per §12 triggers; Alt C rejected except timeboxed emergency.

---

## When to read which plan

- Need **fastest** mirror fix → Alt C, then upgrade to B.
- Need **strongest** CI/ABI → Alt A.
- Need **incremental** deps → Alt B.
- Need **one crate, full path** → Baseline.
