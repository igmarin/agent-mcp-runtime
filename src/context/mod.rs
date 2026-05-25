//! Context provider module for agent-mcp-runtime.

pub mod mcp_provider;
pub mod project_context;

use crate::context::mcp_provider::McpContextProvider;
use crate::context::project_context::ProjectContext;
use crate::registry::manifest::RegistryManifest;
use crate::registry::manifest::ContextProviderDefinition;
use std::time::Duration;

/// Registry of configured external context providers.
#[derive(Default)]
pub struct ContextProviderRegistry {
    providers: Vec<McpContextProvider>,
    client: reqwest::Client,
}

impl ContextProviderRegistry {
    /// Creates a new `ContextProviderRegistry` from the registry manifest.
    #[must_use]
    pub fn from_manifest(manifest: &RegistryManifest) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        let providers = manifest
            .context_providers
            .as_ref()
            .map(|cp_map| {
                // Collect and sort by provider name to ensure deterministic merge order
                let mut sorted_providers: Vec<(&String, &ContextProviderDefinition)> = cp_map
                    .iter()
                    .filter(|(_, cp_def)| cp_def.r#type == "mcp")
                    .collect();
                sorted_providers.sort_by_key(|(name, _)| *name);

                sorted_providers
                    .into_iter()
                    .filter_map(|(name, cp_def)| {
                        match McpContextProvider::from_definition(name, cp_def) {
                            Ok(p) => Some(p),
                            Err(e) => {
                                println!("Warning: Failed to load context provider '{name}': {e}");
                                None
                            }
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();

        Self { providers, client }
    }

    /// Queries all configured context providers and returns a merged `ProjectContext`.
    ///
    /// # Errors
    ///
    /// Returns an error if any non-optional provider fails.
    pub async fn query_all(&self) -> Result<ProjectContext, anyhow::Error> {
        let mut merged = ProjectContext::default();
        for provider in &self.providers {
            match provider.query(&self.client).await {
                Ok(ctx) => merged.merge(ctx),
                Err(e) => {
                    if provider.optional {
                        println!("Warning: optional context provider failed: {e}");
                    } else {
                        return Err(e);
                    }
                }
            }
        }
        Ok(merged)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::manifest::RegistryManifest;

    #[test]
    fn test_from_manifest_empty() {
        let raw = r#"{
            "version": "1.0.0",
            "packs": {},
            "default_stack": []
        }"#;
        let manifest: RegistryManifest = serde_json::from_str(raw).expect("valid json");
        let registry = ContextProviderRegistry::from_manifest(&manifest);
        assert_eq!(registry.providers.len(), 0);
    }

    #[test]
    fn test_from_manifest_with_providers() {
        let raw = r#"{
            "version": "1.0.0",
            "packs": {},
            "default_stack": [],
            "context_providers": {
                "rails-ai-bridge": {
                    "type": "mcp",
                    "endpoint": "http://localhost:3100",
                    "optional": true,
                    "tools": ["rails_get_schema"]
                }
            }
        }"#;
        let manifest: RegistryManifest = serde_json::from_str(raw).expect("valid json");
        let registry = ContextProviderRegistry::from_manifest(&manifest);
        assert_eq!(registry.providers.len(), 1);
        assert_eq!(
            registry.providers[0].endpoint.as_str(),
            "http://localhost:3100/mcp"
        );
        assert!(registry.providers[0].optional);
        assert_eq!(
            registry.providers[0].tools,
            vec!["rails_get_schema".to_string()]
        );
    }

    #[test]
    fn test_from_manifest_deterministic_sorting() {
        let raw = r#"{
            "version": "1.0.0",
            "packs": {},
            "default_stack": [],
            "context_providers": {
                "z_provider": {
                    "type": "mcp",
                    "endpoint": "http://localhost:3200",
                    "optional": true
                },
                "a_provider": {
                    "type": "mcp",
                    "endpoint": "http://localhost:3100",
                    "optional": true
                }
            }
        }"#;
        let manifest: RegistryManifest = serde_json::from_str(raw).expect("valid json");
        let registry = ContextProviderRegistry::from_manifest(&manifest);
        assert_eq!(registry.providers.len(), 2);
        // "a_provider" must come before "z_provider" due to sorting
        assert_eq!(
            registry.providers[0].endpoint.as_str(),
            "http://localhost:3100/mcp"
        );
        assert_eq!(
            registry.providers[1].endpoint.as_str(),
            "http://localhost:3200/mcp"
        );
    }
}
