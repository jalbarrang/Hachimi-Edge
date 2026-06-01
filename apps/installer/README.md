# HachimiRedux Installer for Steam
Simple installer for [HachimiRedux](https://github.com/jalbarrang/Hachimi-Edge), adapted for use with the Steam version of Umamusume: Pretty Derby (JP). Built against the latest DLLs from the [HachimiRedux](https://github.com/jalbarrang/Hachimi-Edge) repository.

This is a fork of the upstream Hachimi installer, extended to optionally bundle the HachimiRedux Training Tracker plugin.

# Usage
The installer supports both GUI and CLI/Unattended mode. To start in GUI mode, just launch the application without any arguments.

## CLI
- Usage: `hachimi_installer.exe [OPTIONS] <SUBCOMMAND>`
- Subcommands:
    - install
    - uninstall
- Options:
    - `--target <filename or path>`: Specifies the install target, relative to the install dir. If it's an absolute path, the install dir will be ignored.
    - `--explicit-target <filename>`: Explicitly specifies the specific target name, regardless of the target's path. This option influences the install method that will be used.
    - `--install-dir <path>`: Specifies the install directory.
    - `--sleep <milliseconds>`: Duration to sleep before starting the install process.
    - `--prompt-for-game-exit`: When enabled, the installer will display a dialog prompting the user to close the game if it is running. The dialog will continue to display until the user closes the game, or cancel the install process.
    - `--pre-install`: Also run pre-install checks. Ignored when uninstalling.
    - `--post-install`: Also run post-install tasks. Ignored when uninstalling.
    - `--with-training-tracker`: Also install the bundled Training Tracker plugin (drops `hachimi_training_tracker.dll` + `skill_grades.json` and enables it in `config.json`). Requires a build with the `training_tracker` feature. Ignored when uninstalling; uninstall always removes the plugin.
    - `--launch-game`: Launch the game after the operation finishes successfully.
    - `--`: Arguments separator; any arguments put after it will be passed onto the game when using `--launch-game`.

In GUI mode an **"Install Training Tracker plugin"** checkbox (ticked by default in `training_tracker` builds) controls the same behavior.

# Building
Put `hachimi.dll` in the root directory, build as any other rust application. For the
Training Tracker feature, also place `hachimi_training_tracker.dll` and `skill_grades.json`
in the root directory.

- **MSRV:** v1.77
- Features:
    - `compress_bin`: Compress the embedded binaries using zstd and decompress them during installation.
    - `training_tracker`: Embed and offer the optional Training Tracker plugin (requires the two extra files above).

# Vendored into HachimiRedux
This is a vendored fork of the upstream Hachimi installer, living as a workspace
member (`installer/`) of the HachimiRedux monorepo. It is built and released as part
of the repo's `create_release.yml` workflow — there is no separate installer repo,
tag, or release pipeline to coordinate. Bump `version` in `Cargo.toml` when the
installer changes meaningfully; it ships with the next HachimiRedux release.

# License
[MIT](LICENSE)

Upstream: forked from [teiosteppa/Installer](https://github.com/teiosteppa/Installer).
