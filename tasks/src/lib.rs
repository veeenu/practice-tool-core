//! Common utilities for building xtasks.

#![deny(missing_docs)]

use std::{
    env,
    path::{Path, PathBuf},
    process::Command,
};

mod dist;
mod file_install;

use anyhow::{Context, Result};
pub use dist::Distribution;
pub use file_install::FileInstall;

/// Points to the current project's root directory (e.g. the one with the topmost `Cargo.toml`).
pub fn project_root() -> PathBuf {
    Path::new(&env::var("CARGO_MANIFEST_DIR").unwrap()).ancestors().nth(1).unwrap().to_path_buf()
}

/// Points to a path in `target`.
///
/// On Windows, it is the normal directory for the build target (e.g. `target/release`).
/// On Linux, it is the target directory under the `x86_64-pc-windows-msvc` target.
pub fn target_path(target: &str) -> PathBuf {
    let mut target_path = project_root().join("target");
    if cfg!(not(windows)) {
        target_path = target_path.join("x86_64-pc-windows-msvc");
    }

    target_path.join(target)
}

/// Points to the distribution directory (`target/dist`).
pub fn dist_path() -> PathBuf {
    project_root().join("target/dist")
}

/// Run a `cargo` command against the MSVC target.
///
/// On Windows, it runs plain `cargo`.
/// On Linux, it passes the command through `xwin` and adds the MSVC target.
pub fn cargo_command(cmd: &'static str) -> Command {
    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());

    let mut command = Command::new(cargo);
    command.current_dir(project_root());
    if cfg!(windows) {
        command.arg(cmd);
    } else {
        command.args(["xwin", cmd, "--target", "x86_64-pc-windows-msvc"]);
    }
    command
}

/// Run a command in the appropriate Steam environment.
///
/// On Windows, it constructs a command and runs it in the project directory.
/// On Linux, it wraps the command in `protontricks-launch`.
pub fn steam_command<P: AsRef<Path>>(child_cmd: P, appid: u32) -> Result<Command> {
    let project_root = project_root();

    if cfg!(windows) {
        let mut cmd = Command::new(child_cmd.as_ref());
        cmd.current_dir(project_root);

        return Ok(cmd);
    }

    let child_cmd = child_cmd.as_ref().strip_prefix(&project_root).unwrap().with_extension("exe");

    Command::new("protontricks-launch")
        .arg("-h")
        .output()
        .context("Could not run `protontricks-launch`: {e}")?;

    let mut cmd = Command::new("protontricks-launch");

    cmd.current_dir(project_root).arg("--appid").arg(appid.to_string()).arg(child_cmd);

    Ok(cmd)
}
