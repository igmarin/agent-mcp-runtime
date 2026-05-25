//! Registry manifest types for loading and parsing the pack registry configurations.

use serde::Deserialize;
use std::collections::HashMap;

/// The root manifest describing the available packs and defaults.
#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
pub struct RegistryManifest {
    /// Schema/manifest version.
    pub version: String,
    /// Map of pack identifiers to their definitions.
    pub packs: HashMap<String, PackDefinition>,
    /// Default stack of pack identifiers to load when no framework is detected.
    pub default_stack: Vec<String>,
    /// Optional context providers.
    pub context_providers: Option<HashMap<String, ContextProviderDefinition>>,
}

/// Description of a single pack repository and its dependencies.
#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
pub struct PackDefinition {
    /// Remote git source repository, e.g. "igmarin/ruby-core-skills".
    pub source: String,
    /// Path to the pack's tile manifest file, usually "tile.json".
    pub tile: String,
    /// Whether this pack is always loaded (e.g. core).
    pub always_loaded: Option<bool>,
    /// Packs that this pack depends on.
    pub depends_on: Option<Vec<String>>,
}

/// Description of a context provider service.
#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
pub struct ContextProviderDefinition {
    /// Provider type, e.g. "mcp".
    pub r#type: String,
    /// Provider HTTP endpoint base URL.
    pub endpoint: String,
    /// Whether this provider is optional.
    pub optional: Option<bool>,
    /// List of tool names requested from the provider.
    pub tools: Option<Vec<String>>,
}

