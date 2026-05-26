//! Service object for resolving and loading skill packs dynamically.

use crate::registry::detector::{DetectedFramework, PackDetector};
use crate::registry::manifest::RegistryManifest;
use crate::registry::resolver::{LoadedPack, RegistryResolver};
use crate::registry::source::SkillSourceResolver;
use crate::registry::tile::TileManifest;
use std::collections::BTreeSet;
use std::path::PathBuf;

/// Service object responsible for resolving and loading skill packs and registries.
pub struct PackResolverService<'a> {
    source_resolver: &'a SkillSourceResolver,
}

impl<'a> PackResolverService<'a> {
    /// Creates a new `PackResolverService` with a reference to a `SkillSourceResolver`.
    ///
    /// # Arguments
    ///
    /// * `source_resolver` - A reference to the [`SkillSourceResolver`] used to resolve remote packs.
    #[must_use]
    pub const fn new(source_resolver: &'a SkillSourceResolver) -> Self {
        Self { source_resolver }
    }

    /// Resolves active packs from the manifest and custom configuration, returning a compiled `RegistryResolver`.
    ///
    /// This will automatically resolve remote repositories if needed, load the tile manifest configurations, and merge
    /// local directory overrides.
    ///
    /// # Arguments
    ///
    /// * `manifest` - The registry manifest containing pack definitions.
    /// * `explicit_packs` - An optional slice of pack names to load. If None, it auto-detects from the environment.
    /// * `local_registries` - An optional slice of local registry directory paths to register.
    ///
    /// # Returns
    ///
    /// Returns a [`RegistryResolver`] on success.
    ///
    /// # Errors
    ///
    /// Returns an error if remote pack resolution fails, file operations fail, or manifests are malformed.
    pub async fn resolve(
        &self,
        manifest: &RegistryManifest,
        explicit_packs: Option<&[String]>,
        local_registries: Option<&[PathBuf]>,
    ) -> Result<RegistryResolver, anyhow::Error> {
        let mut active_pack_names = BTreeSet::new();

        // 1. Gather packs marked as always_loaded
        for (name, pack_def) in &manifest.packs {
            if pack_def.always_loaded.unwrap_or(false) {
                active_pack_names.insert(name.clone());
            }
        }

        // 2. Add explicit packs or perform framework auto-detection
        if let Some(explicit) = explicit_packs {
            for p in explicit {
                active_pack_names.insert(p.clone());
            }
        } else {
            let detected = PackDetector::detect();
            if detected.is_empty() {
                println!(
                    "No framework detected in Gemfile. Loading default stack: {:?}",
                    manifest.default_stack
                );
                for p in &manifest.default_stack {
                    active_pack_names.insert(p.clone());
                }
            } else {
                println!("Auto-detected frameworks: {detected:?}");
                for framework in detected {
                    match framework {
                        DetectedFramework::Rails => {
                            active_pack_names.insert("rails".to_string());
                        }
                        DetectedFramework::Hanami => {
                            active_pack_names.insert("hanami".to_string());
                        }
                    }
                }
            }
        }

        let mut loaded_packs = Vec::new();

        // 3. Resolve and load defined packs
        for name in active_pack_names {
            let pack_def = manifest
                .packs
                .get(&name)
                .ok_or_else(|| anyhow::anyhow!("Pack '{name}' not defined in registry manifest"))?;

            println!(
                "Resolving pack '{name}' from source '{}'...",
                pack_def.source
            );
            let base_path = self.source_resolver.resolve(&pack_def.source).await?;
            let tile_path = base_path.join(&pack_def.tile);
            let tile_content = std::fs::read_to_string(&tile_path).map_err(|e| {
                anyhow::anyhow!(
                    "Failed to read tile manifest for pack '{name}' at {}: {e}",
                    tile_path.display()
                )
            })?;
            let tile: TileManifest = serde_json::from_str(&tile_content)?;

            let priority = match name.as_str() {
                "rails" | "hanami" => 10,
                "core" => 20,
                _ => 30,
            };

            loaded_packs.push(LoadedPack {
                name,
                tile,
                base_path,
                priority,
            });
        }

        // 4. Resolve and load local registries
        if let Some(local_paths) = local_registries {
            for (i, path) in local_paths.iter().enumerate() {
                let tile_path = path.join("tile.json");
                println!("Loading local registry from: {}", tile_path.display());
                let tile_content = std::fs::read_to_string(&tile_path).map_err(|e| {
                    anyhow::anyhow!(
                        "Failed to read local registry tile manifest at {}: {e}",
                        tile_path.display()
                    )
                })?;
                let tile: TileManifest = serde_json::from_str(&tile_content)?;

                loaded_packs.push(LoadedPack {
                    name: format!("local_{i}"),
                    tile,
                    base_path: path.clone(),
                    priority: 0, // Highest priority
                });
            }
        }

        Ok(RegistryResolver::new(loaded_packs))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::manifest::PackDefinition;
    use std::collections::HashMap;

    struct MockGitRunner;

    #[async_trait::async_trait]
    impl crate::registry::git_runner::GitRunner for MockGitRunner {
        async fn clone_repo(
            &self,
            _url: &str,
            dest: &std::path::Path,
        ) -> Result<(), anyhow::Error> {
            std::fs::create_dir_all(dest)?;
            std::fs::write(
                dest.join("tile.json"),
                r#"{
                        "name": "core",
                        "version": "1.0.0",
                        "skills": {}
                    }"#,
            )?;
            Ok(())
        }

        async fn pull_repo(&self, _path: &std::path::Path) -> Result<(), anyhow::Error> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_pack_resolver_service_error_on_missing_pack() -> Result<(), anyhow::Error> {
        let manifest = RegistryManifest {
            version: "1.0.0".to_string(),
            packs: HashMap::new(),
            default_stack: vec![],
            context_providers: None,
        };

        let temp_dir = std::env::temp_dir();
        let source_resolver = SkillSourceResolver::new(temp_dir);
        let service = PackResolverService::new(&source_resolver);

        let res = service
            .resolve(&manifest, Some(&["missing_pack".to_string()]), None)
            .await;
        assert!(res.is_err());
        assert_eq!(
            res.err().expect("expected an error").to_string(),
            "Pack 'missing_pack' not defined in registry manifest"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_pack_resolver_always_loaded() -> Result<(), anyhow::Error> {
        let mut packs = HashMap::new();
        packs.insert(
            "core".to_string(),
            PackDefinition {
                source: "dummy/core".to_string(),
                tile: "tile.json".to_string(),
                always_loaded: Some(true),
                depends_on: None,
            },
        );

        let manifest = RegistryManifest {
            version: "1.0.0".to_string(),
            packs,
            default_stack: vec![],
            context_providers: None,
        };

        let temp_dir = std::env::temp_dir().join("test_pack_resolver_always_loaded");
        if temp_dir.exists() {
            std::fs::remove_dir_all(&temp_dir)?;
        }
        std::fs::create_dir_all(&temp_dir)?;

        let source_resolver =
            SkillSourceResolver::with_git_runner(temp_dir.clone(), Box::new(MockGitRunner));
        let service = PackResolverService::new(&source_resolver);

        let resolver = service.resolve(&manifest, None, None).await?;
        assert_eq!(resolver.active_packs().len(), 1);
        assert_eq!(resolver.active_packs()[0].name, "core");

        std::fs::remove_dir_all(&temp_dir)?;
        Ok(())
    }
}
