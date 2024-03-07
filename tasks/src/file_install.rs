use std::path::{Path, PathBuf};
use std::{env, fs};

use anyhow::{Context, Result};

/// Builder object for installing files to the game directory.
pub struct FileInstall {
    files: Vec<(PathBuf, String)>,
}

impl FileInstall {
    /// Create a new DLL install builder.
    pub fn new() -> Self {
        Self { files: Vec::new() }
    }

    /// Add a file. `path` can be absolute or relative to the project root, `entry` is
    /// relative to the game's directory.
    pub fn with_file<P: AsRef<Path>, S: Into<String>>(mut self, path: P, entry: S) -> Self {
        self.files.push((path.as_ref().to_path_buf(), entry.into()));
        self
    }

    /// Install files. The environment variable designed by `env_var` must point
    /// to the game's directory.
    pub fn install(self, env_var: &str) -> Result<()> {
        let path = env::var(env_var).context("environment variable")?;
        let path = Path::new(&path);

        for (src, dst) in self.files {
            fs::copy(src, path.join(dst)).context("copy")?;
        }

        Ok(())
    }

    /// Uninstall files. The environment variable designed by `env_var` must point
    /// to the game's directory.
    pub fn uninstall(self, env_var: &str) -> Result<()> {
        let path = env::var(env_var).context("environment variable")?;
        let path = Path::new(&path);

        for (_, dst) in self.files {
            fs::remove_file(path.join(dst)).context("remove")?;
        }

        Ok(())
    }
}

impl Default for FileInstall {
    fn default() -> Self {
        Self::new()
    }
}
