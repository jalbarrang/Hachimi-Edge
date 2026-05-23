# Reverse Engineering Research

Documentation of internal game structures, IL2CPP class hierarchies, and modding surface area for UM:PD. This research supports both Hachimi Edge core development and plugin authoring.

## Contents

| Document | Description |
|----------|-------------|
| [single-mode-architecture.md](single-mode-architecture.md) | Career/Single Mode internal classes, lifecycle, and data flow |
| [training-system.md](training-system.md) | Training facilities, command IDs, stat gains, and facility levels |
| [network-protocol.md](network-protocol.md) | MessagePack request/response structures for career mode |
| [il2cpp-class-map.md](il2cpp-class-map.md) | Confirmed IL2CPP classes, methods, and fields from metadata analysis |
| [hachimi-plugin-surface.md](hachimi-plugin-surface.md) | What the Hachimi plugin API exposes and how to use it for mods |
| [qol-opportunities.md](qol-opportunities.md) | Quality of life enhancement ideas with feasibility analysis |
| [trainers-legend-g-crossref.md](trainers-legend-g-crossref.md) | Cross-reference analysis of Trainers-Legend-G mod (136 hooks, race telemetry, IL2CPP patterns) |

## Methodology

Research was conducted through:

1. **Static metadata analysis** — Parsing `global-metadata.dat` (IL2CPP metadata v31) to extract all class, method, field, and property names without needing to execute the game binary
2. **Existing mod analysis** — Studying [Hachimi Edge](https://github.com/Hachimi-Hachimi/Hachimi), [Trainers-Legend-G](https://github.com/MinamiChiwa/Trainers-Legend-G) (136 IL2CPP hooks, see [cross-reference](trainers-legend-g-crossref.md)), and [UmamusumeResponseAnalyzer](https://github.com/UmamusumeResponseAnalyzer/UmamusumeResponseAnalyzer) source code
3. **Community protocol research** — Analyzing the MessagePack data structures from UmamusumeResponseAnalyzer's `Gallop/` namespace
4. **Hook point identification** — Mapping Hachimi's existing IL2CPP hooks to understand what's already intercepted and what gaps remain

## Tools Used

- **Il2CppDumper** (v6.7.46) — Metadata extraction (PE loader path failed; used direct metadata parsing)
- **Node.js metadata parser** — Custom string table extraction from `global-metadata.dat`
- **UmamusumeResponseAnalyzer source** — C# MessagePack deserialization classes as ground truth for protocol structures

## Important Notes

- Method offsets are **not included** because the PE loader couldn't process `GameAssembly.dll` standalone. Offsets change per game update anyway — runtime resolution by name via Hachimi's vtable is the preferred approach.
- All namespace references are in the `Gallop` namespace unless noted otherwise.
- Game version at time of analysis: Steam/Global build (metadata v31, IL2CPP v31).
