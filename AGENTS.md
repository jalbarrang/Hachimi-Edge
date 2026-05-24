# Agent Instructions

## Code Conventions

- Always run clippy and rust-fmt after writing code.
- Test the code generated and avoid regressions.
- The lesser the code the better maintainability.
    - If a file starts to get bigger, decompose it.

## Hard Rules

- **Never launch the game.** Do not run `steam://rungameid`, start executables, or invoke any command that launches the game process. Deployment (copying DLLs) is fine; running the game is the user's job.
- **Never kill game processes.** Do not use `taskkill` or equivalent on game processes.
- **Never modify the backup DLL** at `cri_mana_vpx.dll.backup` in the game directory.

## Extra Documentation

Read these on demand — don't load everything upfront.

| Topic | Doc |
|-------|-----|
| What this project is | [docs/overview.md](docs/overview.md) |
| Module layout, platform split, plugin API | [docs/architecture.md](docs/architecture.md) |
| Build commands, deployment, config | [docs/build-and-deployment.md](docs/build-and-deployment.md) |
| Render hook gating, IL2CPP hooks, unsafe, egui overlays | [docs/patterns.md](docs/patterns.md) |
| Log file location and usage | [docs/logging.md](docs/logging.md) |
| Beads issue tracker usage | [docs/issue-tracker.md](docs/issue-tracker.md) |
| IL2CPP class maps, training system, network protocol, TLG cross-ref | [docs/reverse-engineering/](docs/reverse-engineering/README.md) |
