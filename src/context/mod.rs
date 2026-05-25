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
    /// Creates a new ContextProviderRegistry from the registry manifest.
    pub fn from_manifest(manifest: &RegistryManifest) -> Self {
        let mut providers = Vec::new();
        if let Some(ref cp_map) = manifest.context_providers {
            for (name, cp_def) in cp_map {
                if cp_def.r#type == "mcp" {
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
                    providers.push(McpContextProvider::new(cp_def.endpoint.clone(), optional, tools));
                }
            }
        }
        Self { providers }
    }

    /// Queries all configured context providers and returns a merged ProjectContext.
    pub async fn query_all(&self) -> ProjectContext {
        let mut merged = ProjectContext::default();
        for provider in &self.providers {
            if let Ok(ctx) = provider.query().await {
                if let Some(ref s) = ctx.schema {
                    merged.schema = Some(s.clone());
                }
                if let Some(ref r) = ctx.routes {
                    merged.routes = Some(r.clone());
                }
                if let Some(ref c) = ctx.controllers {
                    merged.controllers = Some(c.clone());
                }
                if let Some(ref m) = ctx.models {
                    merged.models = Some(m.clone());
                }
                if let Some(ref cfg) = ctx.config {
                    merged.config = Some(cfg.clone());
                }
                if let Some(ref g) = ctx.gems {
                    merged.gems = Some(g.clone());
                }
                if let Some(ref t) = ctx.tests {
                    merged.tests = Some(t.clone());
                }
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
        let manifest: RegistryManifest = serde_json::from_str(raw).unwrap();
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
        let manifest: RegistryManifest = serde_json::from_str(raw).unwrap();
        let registry = ContextProviderRegistry::from_manifest(&manifest);
        assert_eq!(registry.providers.len(), 1);
        assert_eq!(registry.providers[0].endpoint, "http://localhost:3100");
        assert_eq!(registry.providers[0].optional, true);
        assert_eq!(registry.providers[0].tools, vec!["rails_get_schema".to_string()]);
    }
}

