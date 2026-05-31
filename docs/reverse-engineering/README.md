# Reverse Engineering Research

Documentation of internal game structures, IL2CPP class hierarchies, and modding surface area for UM:PD. This research supports both Hachimi Edge core development and plugin authoring.

## Contents

| Document | Description |
|----------|-------------|
| [single-mode-architecture.md](single-mode-architecture.md) | Career/Single Mode internal classes, lifecycle, and data flow |
| [training-system.md](training-system.md) | Training facilities, command IDs, stat gains, and facility levels |
| [career-evaluation.md](career-evaluation.md) | Live 評価点 (evaluation score) + Final Training Rank: formula, IL2CPP sources, skill-grade resource, validation |
| [network-protocol.md](network-protocol.md) | MessagePack request/response structures for career mode |
| [il2cpp-class-map.md](il2cpp-class-map.md) | Confirmed IL2CPP classes, methods, and fields from metadata analysis |
| [il2cpp-signatures.md](il2cpp-signatures.md) | How to verify hook signatures (return type + arg count) against the trimmed `umamusume.dll` method dump in [il2cpp/](il2cpp/umamusume-methods.txt); the return-type/coroutine pitfall |
| [hachimi-plugin-surface.md](hachimi-plugin-surface.md) | What the Hachimi plugin API exposes and how to use it for mods |
| [qol-opportunities.md](qol-opportunities.md) | Quality of life enhancement ideas with feasibility analysis |
| [trainers-legend-g-crossref.md](trainers-legend-g-crossref.md) | Cross-reference analysis of Trainers-Legend-G mod (136 hooks, race telemetry, IL2CPP patterns) |

## Methodology

Research was conducted through:

1. **Static metadata analysis** — Parsing `global-metadata.dat` (IL2CPP metadata v31) to extract all class, method, field, and property names without needing to execute the game binary
2. **Runtime full class dump (2026-05-25)** — The training tracker plugin can enumerate ALL IL2CPP classes at runtime via `il2cpp_domain_get_assemblies` → `il2cpp_image_get_class` → fields/methods. Outputs to `il2cpp_classes.txt` (~21 MB, 105 assemblies, 14,832 classes in `umamusume.dll` alone). Triggered by a menu button. Includes declaring-type chain for nested classes (e.g. `MasterSkillData.SkillData`).
3. **Existing mod analysis** — Studying [Hachimi Edge](https://github.com/Hachimi-Hachimi/Hachimi), [Trainers-Legend-G](https://github.com/MinamiChiwa/Trainers-Legend-G) (136 IL2CPP hooks, see [cross-reference](trainers-legend-g-crossref.md)), and [UmamusumeResponseAnalyzer](https://github.com/UmamusumeResponseAnalyzer/UmamusumeResponseAnalyzer) source code
4. **Community protocol research** — Analyzing the MessagePack data structures from UmamusumeResponseAnalyzer's `Gallop/` namespace
5. **Hook point identification** — Mapping Hachimi's existing IL2CPP hooks to understand what's already intercepted and what gaps remain

## Tools Used

- **Il2CppDumper** (v6.7.46) — Metadata extraction (PE loader path failed; used direct metadata parsing)
- **Node.js metadata parser** — Custom string table extraction from `global-metadata.dat`
- **Runtime class dump** (training tracker plugin) — Full IL2CPP class/field/method enumeration via `il2cpp_domain_get_assemblies` API. Output: `il2cpp_classes.txt` next to game executable. Includes nested class qualified names.
- **UmamusumeResponseAnalyzer source** — C# MessagePack deserialization classes as ground truth for protocol structures

## Important Notes

- Method offsets are **not included** because the PE loader couldn't process `GameAssembly.dll` standalone. Offsets change per game update anyway — runtime resolution by name via Hachimi's vtable is the preferred approach.
- All namespace references are in the `Gallop` namespace unless noted otherwise.
- Game version at time of analysis: Steam/Global build (metadata v31, IL2CPP v31).
