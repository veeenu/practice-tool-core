use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};
use zip::write::FileOptions;
use zip::{CompressionMethod, ZipWriter};

use crate::{cargo_command, dist_path, project_root, target_path};

/// Builder object for artifacts distribution.
pub struct Distribution {
    zip_file: PathBuf,
    artifacts: Vec<(PathBuf, String)>,
    files: Vec<(PathBuf, String)>,
}

impl Distribution {
    /// Constructor. The object will be created in `target/dist/<zip_file>`.
    pub fn new<P: AsRef<Path>>(zip_file: P) -> Self {
        Self {
            zip_file: dist_path().join(zip_file),
            artifacts: Vec::new(),
            files: Vec::new(),
        }
    }

    /// Add an artifact (e.g. a compiled resource).
    pub fn with_artifact<P: AsRef<Path>, S: Into<String>>(mut self, path: P, entry: S) -> Self {
        self.artifacts
            .push((target_path("release").join(path), entry.into()));
        self
    }

    /// Add a regular file (e.g. a configuration file or a README).
    pub fn with_file<P: AsRef<Path>, S: Into<String>>(mut self, path: P, entry: S) -> Self {
        self.files.push((project_root().join(path), entry.into()));
        self
    }

    /// Build the distribution.
    pub fn build(self, args: &[&str]) -> Result<()> {
        let mut command = cargo_command("build");
        command.args(args).env("CARGO_XTASK_DIST", "true");

        let status = command.status().map_err(|e| anyhow!("cargo: {}", e))?;

        if !status.success() {
            bail!("cargo build failed");
        }

        fs::remove_dir_all(dist_path()).ok();
        fs::create_dir_all(dist_path())?;

        let mut zip = ZipWriter::new(File::create(self.zip_file)?);
        let file_options = FileOptions::default().compression_method(CompressionMethod::Deflated);

        let mut buf: Vec<u8> = Vec::new();

        for (src, dst) in self.artifacts.into_iter().chain(self.files) {
            File::open(&src)
                .context("Couldn't open file")?
                .read_to_end(&mut buf)
                .context("Couldn't read file")?;

            zip.start_file(&dst, file_options)
                .context("Couldn't start zip file")?;
            zip.write_all(&buf).context("Couldn't write zip")?;

            buf.clear();
        }

        Ok(())
    }
}
