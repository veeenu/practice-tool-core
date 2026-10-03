//! Checks GitHub for new releases of a tool.

use std::time::Duration;

pub use semver::Version;
use tracing::info;

pub enum Update {
    Available { url: String, notes: String },
    UpToDate,
    Error(String),
}

impl Update {
    /// Checks whether the latest release of the GitHub `repo` (e.g.
    /// `"veeenu/darksoulsiii-practice-tool"`) is newer than `current`.
    pub fn check(repo: &str, current: Version) -> Self {
        info!("Checking for updates...");
        Self::fetch(repo, current).unwrap_or_else(Update::Error)
    }

    fn fetch(repo: &str, current_version: Version) -> Result<Self, String> {
        #[derive(serde::Deserialize)]
        struct GithubRelease {
            tag_name: String,
            html_url: String,
            body: String,
        }

        // Fail fast when the network is unreachable instead of waiting for the
        // default timeout.
        let release = ureq::AgentBuilder::new()
            .timeout(Duration::from_secs(5))
            .build()
            .get(&format!("https://api.github.com/repos/{repo}/releases/latest"))
            .call()
            .map_err(|e| e.to_string())?
            .into_json::<GithubRelease>()
            .map_err(|e| e.to_string())?;

        let version = Version::parse(&release.tag_name).map_err(|e| e.to_string())?;

        if version <= current_version {
            return Ok(Update::UpToDate);
        }

        let notes = match release.body.find("## What's Changed") {
            Some(i) => release.body[..i].trim(),
            None => &release.body,
        };
        let notes = format!(
            "A new version of the practice tool is available!\n\nLatest version:    \
             {version}\nInstalled version: {current_version}\n\nRelease notes:\n{notes}\n",
        );

        Ok(Update::Available { url: release.html_url, notes })
    }
}
