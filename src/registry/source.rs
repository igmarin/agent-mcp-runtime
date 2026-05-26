//! Git repository caching and source resolution for skill packs.

use crate::registry::git_runner::{DefaultGitRunner, GitRunner};
use std::path::PathBuf;

/// Resolves remote git skill pack sources by cloning or pulling them into a local cache directory.
pub struct SkillSourceResolver {
    cache_dir: PathBuf,
    git: Box<dyn GitRunner>,
}

impl SkillSourceResolver {
    /// Creates a new `SkillSourceResolver` with the given cache directory and default git runner.
    #[must_use]
    pub fn new(cache_dir: PathBuf) -> Self {
        Self {
            cache_dir,
            git: Box::new(DefaultGitRunner),
        }
    }

    /// Creates a new `SkillSourceResolver` with a custom [`GitRunner`].
    #[must_use]
    pub fn with_git_runner(cache_dir: PathBuf, git: Box<dyn GitRunner>) -> Self {
        Self { cache_dir, git }
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

    fn compute_cache_key(source: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        source.hash(&mut hasher);
        let hash_val = hasher.finish();

        let sanitized: String = source
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '_' })
            .collect();
        format!("{sanitized}_{hash_val:x}")
    }

    /// Resolves a remote source (e.g. "igmarin/ruby-core-skills") to a local path.
    ///
    /// Clones the repository if not cached, or runs `git pull` if it already exists.
    ///
    /// # Errors
    ///
    /// Returns an error if the git command execution fails.
    pub async fn resolve(&self, source: &str) -> Result<PathBuf, anyhow::Error> {
        let cache_key = Self::compute_cache_key(source);
        let cache_path = self.cache_dir.join(cache_key);

        if cache_path.exists() {
            if let Err(e) = self.git.pull_repo(&cache_path).await {
                anyhow::bail!("git pull failed for pack: {source}: {e}");
            }
        } else {
            if let Some(parent) = cache_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let clone_url = format!("https://github.com/{source}.git");
            if let Err(e) = self.git.clone_repo(&clone_url, &cache_path).await {
                if cache_path.exists() {
                    let _ = std::fs::remove_dir_all(&cache_path);
                }
                anyhow::bail!("git clone failed for pack: {source}: {e}");
            }
        }
        Ok(cache_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::git_runner::GitRunner;
    use async_trait::async_trait;
    use std::path::Path;
    use std::sync::Mutex;

    struct MockGitRunner {
        clone_called: Mutex<Vec<(String, PathBuf)>>,
        pull_called: Mutex<Vec<PathBuf>>,
        fail_clone: bool,
    }

    #[async_trait]
    impl GitRunner for MockGitRunner {
        async fn clone_repo(&self, url: &str, dest: &Path) -> Result<(), anyhow::Error> {
            if self.fail_clone {
                anyhow::bail!("mock clone error");
            }
            self.clone_called
                .lock()
                .map_err(|_| anyhow::anyhow!("mutex lock error"))?
                .push((url.to_string(), dest.to_path_buf()));
            std::fs::create_dir_all(dest)?;
            Ok(())
        }

        async fn pull_repo(&self, path: &Path) -> Result<(), anyhow::Error> {
            self.pull_called
                .lock()
                .map_err(|_| anyhow::anyhow!("mutex lock error"))?
                .push(path.to_path_buf());
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_source_resolver_clones_when_missing() -> Result<(), anyhow::Error> {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| anyhow::anyhow!("{e}"))?
            .as_nanos();
        let temp_dir = std::env::temp_dir().join(format!("test_source_resolver_{unique}"));
        if temp_dir.exists() {
            std::fs::remove_dir_all(&temp_dir)?;
        }
        std::fs::create_dir_all(&temp_dir)?;

        let git = MockGitRunner {
            clone_called: Mutex::new(Vec::new()),
            pull_called: Mutex::new(Vec::new()),
            fail_clone: false,
        };

        let resolver = SkillSourceResolver::with_git_runner(temp_dir.clone(), Box::new(git));
        let path = resolver.resolve("igmarin/test-pack").await?;

        assert!(path.starts_with(&temp_dir));
        assert!(path.exists());

        std::fs::remove_dir_all(&temp_dir)?;
        Ok(())
    }
}
