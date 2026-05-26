//! Git command runner trait for test mockability.

use async_trait::async_trait;
use std::path::Path;
use tokio::process::Command;

/// Trait encapsulating raw Git subprocess commands.
#[async_trait]
pub trait GitRunner: Send + Sync {
    /// Clones a remote repository to a local directory path.
    ///
    /// # Errors
    ///
    /// Returns an error if the git clone command execution fails or returns non-zero.
    async fn clone_repo(&self, url: &str, dest: &Path) -> Result<(), anyhow::Error>;

    /// Pulls latest changes inside a local repository directory.
    ///
    /// # Errors
    ///
    /// Returns an error if the git pull command execution fails or returns non-zero.
    async fn pull_repo(&self, path: &Path) -> Result<(), anyhow::Error>;
}

/// Default implementation of [`GitRunner`] that spawns a real git subprocess.
pub struct DefaultGitRunner;

#[async_trait]
impl GitRunner for DefaultGitRunner {
    async fn clone_repo(&self, url: &str, dest: &Path) -> Result<(), anyhow::Error> {
        let dest_str = dest
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid destination path string"))?;

        let status = Command::new("git")
            .args(["clone", url, dest_str])
            .status()
            .await?;

        if !status.success() {
            anyhow::bail!("git clone failed");
        }
        Ok(())
    }

    async fn pull_repo(&self, path: &Path) -> Result<(), anyhow::Error> {
        let status = Command::new("git")
            .arg("pull")
            .current_dir(path)
            .status()
            .await?;

        if !status.success() {
            anyhow::bail!("git pull failed");
        }
        Ok(())
    }
}
