//! Tile manifest types representing a single pack's skills, agents, and metadata.

use serde::Deserialize;
use std::collections::HashMap;

/// The tile manifest describing a pack's catalog of skills and agents.
#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
pub struct TileManifest {
    /// Unique name of the pack, e.g., "ruby-core-skills".
    pub name: String,
    /// Version of the pack.
    pub version: String,
    /// Optional description or summary of the pack.
    pub summary: Option<String>,
    /// Optional dependencies of this pack on other packs.
    pub depends_on: Option<Vec<String>>,
    /// Map of skill names to their metadata entry.
    pub skills: HashMap<String, SkillEntry>,
    /// Map of agent names to their metadata entry.
    pub agents: Option<HashMap<String, AgentEntry>>,
    /// Optional mapping of deprecated skill names to redirect entries.
    pub deprecated_skills: Option<HashMap<String, DeprecatedEntry>>,
}

/// Metadata entry for a specific skill in a pack.
#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
pub struct SkillEntry {
    /// Relative path to the skill markdown file (e.g. "skills/write-code.md").
    pub path: String,
    /// Optional override description. If missing, frontmatter is used.
    pub description: Option<String>,
    /// Optional classification tags.
    pub tags: Option<Vec<String>>,
}

/// Metadata entry for an agent/workflow in a pack.
#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
pub struct AgentEntry {
    /// Relative path to the agent markdown file.
    pub path: String,
    /// Optional override description.
    pub description: Option<String>,
    /// Skills this agent depends on / references.
    pub depends_on: Option<Vec<String>>,
}

/// Deprecation redirection entry.
#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
pub struct DeprecatedEntry {
    /// The name of the skill/agent this has moved to.
    pub moved_to: String,
    /// Informational message to print explaining the deprecation.
    pub message: String,
    /// Optional future version in which this alias will be completely removed.
    pub removed_in: Option<String>,
}
