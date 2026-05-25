//! Context provider module for agent-mcp-runtime.

pub mod mcp_provider;
pub mod project_context;

use crate::context::mcp_provider::McpContextProvider;
use crate::context::project_context::ProjectContext;
use crate::registry::manifest::RegistryManifest;

/// Registry of configured external context providers.
#[derive(Default)]
pub struct ContextProviderRegistry {
    providers: Vec<McpContextProvider>,
}

impl ContextProviderRegistry {
    /// Creates a new `ContextProviderRegistry` from the registry manifest.
    #[must_use]
    pub fn from_manifest(manifest: &RegistryManifest) -> Self {
        let providers = manifest
            .context_providers
            .as_ref()
            .map(|cp_map| {
                cp_map
                    .iter()
                    .filter(|(_, cp_def)| cp_def.r#type == "mcp")
                    .map(|(name, cp_def)| {
                        let optional = cp_def.optional.unwrap_or(true);
                        let tools = cp_def.tools.clone().unwrap_or_else(|| vec![
                            "rails_get_schema".to_string(),
                            "rails_get_routes".to_string(),
                            "rails_get_controllers".to_string(),
                            "rails_get_model_details".to_string(),
                            "rails_get_config".to_string(),
                            "rails_get_gems".to_string(),
                            "rails_get_test_info".to_string(),
                        ]);
                        println!("Registered context provider '{name}' (endpoint: {})", cp_def.endpoint);
                        McpContextProvider::new(cp_def.endpoint.clone(), optional, tools)
                    })
                    .collect()
            })
            .unwrap_or_default();

        Self { providers }
    }

    /// Queries all configured context providers and returns a merged `ProjectContext`.
    pub async fn query_all(&self) -> ProjectContext {
        let mut merged = ProjectContext::default();
        for provider in &self.providers {
            if let Ok(ctx) = provider.query().await {
                merged.merge(ctx);
            }
        }
        merged
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
        assert_eq!(registry.providers[0].endpoint, "http://localhost:3100");
        assert!(registry.providers[0].optional);
        assert_eq!(registry.providers[0].tools, vec!["rails_get_schema".to_string()]);
    }
}
