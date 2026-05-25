//! Git repository caching and source resolution for skill packs.

#![allow(clippy::must_use_candidate, clippy::missing_const_for_fn)]

use std::path::PathBuf;
use tokio::process::Command;

/// Resolves remote git skill pack sources by cloning or pulling them into a local cache directory.
pub struct SkillSourceResolver {
    cache_dir: PathBuf,
}

impl SkillSourceResolver {
    /// Creates a new `SkillSourceResolver` with the given cache directory.
    pub fn new(cache_dir: PathBuf) -> Self {
        Self { cache_dir }
    }

    /// Resolves the default cache directory, checking `AGENT_MCP_CACHE_DIR` then `HOME` / `USERPROFILE`.
    ///
    /// # Errors
    ///
    /// Returns an error if no appropriate environment variables are defined.
    pub fn default_cache_dir() -> Result<PathBuf, anyhow::Error> {
        if let Ok(override_path) = std::env::var("AGENT_MCP_CACHE_DIR") {
            return Ok(PathBuf::from(override_path));
        }
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .map_err(|_| anyhow::anyhow!("Could not find HOME or USERPROFILE environment variables to determine default cache directory"))?;
        Ok(PathBuf::from(home).join(".agent-mcp-runtime").join("cache"))
    }

    /// Resolves a remote source (e.g. "igmarin/ruby-core-skills") to a local path.
    ///
    /// Clones the repository if not cached, or runs `git pull` if it already exists.
    ///
    /// # Errors
    ///
    /// Returns an error if the git subprocess execution fails.
    pub async fn resolve(&self, source: &str) -> Result<PathBuf, anyhow::Error> {
        let clean_source = source.replace('/', "_");
        let cache_path = self.cache_dir.join(clean_source);

        if cache_path.exists() {
            let status = Command::new("git")
                .arg("pull")
                .current_dir(&cache_path)
                .status()
                .await?;
            if !status.success() {
                anyhow::bail!("git pull failed for pack: {source}");
            }
        } else {
            if let Some(parent) = cache_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let clone_url = format!("https://github.com/{source}.git");
            let status = Command::new("git")
                .args([
                    "clone",
                    &clone_url,
                    cache_path
                        .to_str()
                        .ok_or_else(|| anyhow::anyhow!("Invalid cache path string"))?,
                ])
                .status()
                .await?;
            if !status.success() {
                anyhow::bail!("git clone failed for pack: {source}");
            }
        }
        Ok(cache_path)
    }
}
