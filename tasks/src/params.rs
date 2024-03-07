use std::collections::BTreeMap;
use std::env;
use std::ffi::OsStr;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::Command;

use anyhow::{bail, Context, Result};
use regex::Regex;

use crate::project_root;

pub fn codegen_param_names<P1: AsRef<Path>, P2: AsRef<Path>>(
    paramdex_path: P1,
    dest_path: P2,
) -> Result<()> {
    let mut data: BTreeMap<String, BTreeMap<usize, String>> = BTreeMap::new();

    let files_with_content = project_root()
        .join(paramdex_path)
        .read_dir()?
        .flat_map(|entry| {
            entry.map(|entry| entry.path()).map(|path| {
                if path.is_file() && Some("txt") == path.extension().and_then(OsStr::to_str) {
                    Some(path)
                } else {
                    None
                }
            })
        })
        .flatten();

    let r = Regex::new(r"^(\d+)\s+(.+)").unwrap();

    for path in files_with_content {
        let stem = path.file_stem().unwrap().to_string_lossy().to_string();

        let data_contents: BTreeMap<_, _> = BufReader::new(File::open(path)?)
            .lines()
            .filter_map(|line| {
                let line = line.ok()?;
                let cap = r.captures(&line)?;

                let id: usize = cap[1].parse().ok()?;
                let name: String = cap[2].to_string();
                Some((id, name))
            })
            .collect();

        data.insert(stem, data_contents);
    }

    serde_json::to_writer_pretty(File::create(project_root().join(dest_path))?, &data)?;

    Ok(())
}

pub fn checkout_paramdex() -> Result<()> {
    let git = env::var("GIT").unwrap_or_else(|_| "git".to_string());

    if project_root().join("target/Paramdex").exists() {
        let status = Command::new(&git)
            .current_dir(project_root().join("target/Paramdex"))
            .args(["fetch"])
            .status()
            .context("git")?;

        if !status.success() {
            bail!("git fetch failed");
        }

        let status = Command::new(&git)
            .current_dir(project_root().join("target/Paramdex"))
            .args(["pull"])
            .status()
            .context("git")?;

        if !status.success() {
            bail!("git pull failed");
        }
    } else {
        let status = Command::new(&git)
            .current_dir(project_root().join("target"))
            .args(["clone", "https://github.com/soulsmods/Paramdex.git"])
            .status()
            .context("git")?;

        if !status.success() {
            bail!("git clone failed");
        }
    }

    Ok(())
}
