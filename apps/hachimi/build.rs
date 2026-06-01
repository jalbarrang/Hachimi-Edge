// Build scripts are expected to panic on failure — unwrap() is appropriate here.
#![allow(clippy::disallowed_methods)]

use std::process::{Command, Output};

fn setup_windows_build() {
    // Link proxy export defs
    let absolute_path = std::fs::canonicalize("src/windows/proxy/exports.def").unwrap();
    if std::env::var("CARGO_CFG_TARGET_ENV").unwrap() == "msvc" {
        println!("cargo:rustc-cdylib-link-arg=/DEF:{}", absolute_path.display());
    } else {
        // I have to remove the '/DEF:' every time I cross compile on linux, so might as well do this
        println!("cargo:rustc-cdylib-link-arg={}", absolute_path.display());
    }

    // Generate and link version information
    let res = tauri_winres::WindowsResource::new();
    res.compile().unwrap();
}

fn command_output_to_string(output: Output) -> String {
    String::from_utf8(output.stdout).expect("valid utf-8 from command output")
}

fn execute_command(command: &mut Command) -> Option<Output> {
    let output = command.output().ok()?;
    if !output.status.success() {
        return None;
    }
    Some(output)
}

fn setup_version_env() {
    let mut version_str = "v".to_owned() + env!("CARGO_PKG_VERSION");

    if execute_command(Command::new("git").args(["--version"])).is_some() {
        let commit_output = execute_command(Command::new("git").args(["rev-parse", "--short", "HEAD"]));
        if let Some(output) = commit_output {
            version_str.push('-');
            let output_str = command_output_to_string(output);
            version_str.push_str(&output_str[..output_str.len() - 1]); // remove \n
        } else {
            println!("cargo:warning=Failed to retrieve git commit hash");
        }

        let status_output = execute_command(Command::new("git").args(["status", "--porcelain"]));
        if let Some(output) = status_output {
            if !output.stdout.is_empty() && std::env::var("HACHIMI_IGNORE_DIRTY").is_err() {
                version_str.push_str("-dirty");
            }
        } else {
            println!("cargo:warning=Failed to retrieve git repo status");
        }

        let gitdir_output = execute_command(Command::new("git").args(["rev-parse", "--git-dir"]));
        if let Some(output) = gitdir_output {
            println!("cargo:rerun-if-changed={}", command_output_to_string(output));
        } else {
            println!("cargo:warning=Failed to retrieve git directory");
        }
    } else {
        println!("cargo:warning=Failed to execute git. Is git installed?");
    }

    println!("cargo:rustc-env=HACHIMI_DISPLAY_VERSION={}", version_str);
}

fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap();
    if target_os == "windows" {
        setup_windows_build();
    }

    setup_version_env();
}
