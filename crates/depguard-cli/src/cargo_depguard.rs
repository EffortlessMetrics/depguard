//! `cargo depguard` wrapper.
//!
//! Cargo discovers subcommands via executables named `cargo-<name>`.
//! This wrapper forwards all arguments to the sibling `depguard` binary.

#![forbid(unsafe_code)]

use anyhow::Context;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() -> anyhow::Result<()> {
    let current_exe = std::env::current_exe().context("resolve current executable path")?;
    let depguard_exe = sibling_depguard_path(&current_exe);

    let status = if depguard_exe.exists() {
        Command::new(depguard_exe)
            .args(std::env::args().skip(1))
            .status()
            .context("run depguard")?
    } else {
        Command::new("depguard")
            .args(std::env::args().skip(1))
            .status()
            .context("run depguard from PATH")?
    };

    match status.code() {
        Some(code) => std::process::exit(code),
        None => anyhow::bail!("depguard process terminated by signal"),
    }
}

fn sibling_depguard_path(current_exe: &Path) -> PathBuf {
    let depguard_file = if cfg!(windows) {
        "depguard.exe"
    } else {
        "depguard"
    };
    current_exe.with_file_name(depguard_file)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sibling_depguard_path_uses_platform_name() {
        let input = if cfg!(windows) {
            PathBuf::from(r"C:\tools\cargo-depguard.exe")
        } else {
            PathBuf::from("/usr/local/bin/cargo-depguard")
        };
        let output = sibling_depguard_path(&input);
        if cfg!(windows) {
            assert!(output.ends_with("depguard.exe"));
        } else {
            assert!(output.ends_with("depguard"));
        }
    }
}
