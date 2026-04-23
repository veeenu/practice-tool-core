//! Common utilities for building xtasks.

// #![deny(missing_docs)]

use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

pub mod codegen;
mod dist;
mod file_install;
pub mod params;

use anyhow::{bail, Result};
pub use dist::Distribution;
pub use file_install::FileInstall;
use sysinfo::System;

/// Points to the current project's root directory (e.g. the one with the
/// topmost `Cargo.toml`).
pub fn project_root() -> PathBuf {
    Path::new(&env::var("CARGO_MANIFEST_DIR").unwrap()).ancestors().nth(1).unwrap().to_path_buf()
}

/// Points to a path in `target`.
///
/// On Windows, it is the normal directory for the build target (e.g.
/// `target/release`). On Linux, it is the target directory under the
/// `x86_64-pc-windows-msvc` target.
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
/// On Linux, it figures out the pertinent environment variables from the
/// running process and runs `proton` manually.
pub fn steam_command<P: AsRef<Path>>(child_cmd: P, appid: u32, exe_name: &str) -> Result<Command> {
    let project_root = project_root();

    if cfg!(windows) {
        let mut cmd = Command::new(child_cmd.as_ref());
        cmd.current_dir(project_root);

        return Ok(cmd);
    }

    let sys = System::new_all();
    let Some((proton_path, compat_data_path, compat_client_install_path)) =
        sys.processes().iter().find_map(|(_, process)| {
            if !process.name().to_str()?.contains(exe_name) {
                return None;
            }

            if process.exe()?.ends_with("files/bin/wine64_preloader") {
                return None;
            }

            let proton_path = process.exe()?.parent()?.parent()?.parent()?.join("proton");
            let proton_path = match proton_path.strip_prefix("/run/host").ok() {
                Some(p) => Path::new("/").join(p),
                None => proton_path,
            };

            let compat_data_path = process.environ().iter().find_map(|env| {
                env.to_string_lossy().strip_prefix("STEAM_COMPAT_DATA_PATH=").map(|s| s.to_owned())
            });

            let compat_client_install_path = process.environ().iter().find_map(|env| {
                env.to_string_lossy()
                    .strip_prefix("STEAM_COMPAT_CLIENT_INSTALL_PATH=")
                    .map(|s| s.to_owned())
            });

            Some((proton_path, compat_data_path?, compat_client_install_path?))
        })
    else {
        bail!("Couldn't find running process");
    };

    println!("proton_path={proton_path:?}");
    println!("compat_data_path={compat_data_path:?}");
    println!("compat_client_install_path={compat_client_install_path:?}");

    let child_cmd = child_cmd.as_ref().strip_prefix(&project_root).unwrap().with_extension("exe");

    let mut cmd = Command::new(proton_path);

    cmd.env("SteamAppId", appid.to_string())
        .env("STEAM_COMPAT_DATA_PATH", compat_data_path)
        .env("STEAM_COMPAT_CLIENT_INSTALL_PATH", compat_client_install_path)
        .arg("runinprefix")
        .arg(child_cmd);

    Ok(cmd)
}
