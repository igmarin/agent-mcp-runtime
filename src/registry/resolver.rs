//! Registry resolver module for composing and resolving skills and agents.

#![allow(clippy::must_use_candidate, clippy::option_if_let_else)]

use crate::registry::tile::{DeprecatedEntry, TileManifest};
use std::collections::HashMap;
use std::path::PathBuf;

/// A loaded pack representing a registered repository.
#[derive(Debug, Clone)]
pub struct LoadedPack {
    /// Name of the pack.
    pub name: String,
    /// Deserialized tile manifest.
    pub tile: TileManifest,
    /// Local filesystem path where the pack is located.
    pub base_path: PathBuf,
    /// Priority level (lower value is higher priority).
    pub priority: usize,
}

/// A resolved skill or agent, containing its content and metadata.
#[derive(Debug, Clone)]
pub struct ResolvedSkill {
    /// Name of the resolved skill/agent.
    pub name: String,
    /// Pack from which it was resolved.
    pub pack: String,
    /// Absolute filesystem path to the markdown file.
    pub path: PathBuf,
    /// Complete text content of the markdown file.
    pub content: String,
}

/// Summary of a skill or agent for catalogs.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SkillSummary {
    /// Unique name of the skill/agent.
    pub name: String,
    /// Source pack name.
    pub pack: String,
    /// Human-readable description.
    pub description: String,
}

fn is_descendant(base: &std::path::Path, path: &std::path::Path) -> bool {
    if let (Ok(base_canon), Ok(path_canon)) =
        (std::fs::canonicalize(base), std::fs::canonicalize(path))
    {
        path_canon.starts_with(base_canon)
    } else {
        false
    }
}

/// Core resolver that aggregates active packs and resolves queries.
pub struct RegistryResolver {
    active_packs: Vec<LoadedPack>,
    deprecated_index: HashMap<String, DeprecatedEntry>,
}

impl RegistryResolver {
    /// Builds a new `RegistryResolver` from a list of loaded packs.
    #[must_use]
    pub fn new(mut active_packs: Vec<LoadedPack>) -> Self {
        // Sort active packs by priority ascending (highest priority first)
        active_packs.sort_by_key(|p| p.priority);

        let mut deprecated_index = HashMap::new();
        // Gather deprecated skills in reverse order so higher priority overwrites
        let mut sorted_reverse = active_packs.clone();
        sorted_reverse.sort_by_key(|p| std::cmp::Reverse(p.priority));

        for pack in sorted_reverse {
            if let Some(ref deprecated) = pack.tile.deprecated_skills {
                for (old_name, entry) in deprecated {
                    deprecated_index.insert(old_name.clone(), entry.clone());
                }
            }
        }

        Self {
            active_packs,
            deprecated_index,
        }
    }

    /// Resolves a skill by name, handling priority tiers and deprecation redirects.
    pub fn resolve_skill(&self, name: &str) -> Option<ResolvedSkill> {
        // Handle deprecation redirects transparently
        let target_name = if let Some(dep) = self.deprecated_index.get(name) {
            &dep.moved_to
        } else {
            name
        };

        for pack in &self.active_packs {
            if let Some(entry) = pack.tile.skills.get(target_name) {
                let file_path = pack.base_path.join(&entry.path);
                if is_descendant(&pack.base_path, &file_path) {
                    if let Ok(content) = std::fs::read_to_string(&file_path) {
                        return Some(ResolvedSkill {
                            name: target_name.to_string(),
                            pack: pack.name.clone(),
                            path: file_path,
                            content,
                        });
                    }
                }
            }
        }
        None
    }

    /// Resolves an agent by name, handling priority tiers.
    pub fn resolve_agent(&self, name: &str) -> Option<ResolvedSkill> {
        for pack in &self.active_packs {
            if let Some(ref agents) = pack.tile.agents {
                if let Some(entry) = agents.get(name) {
                    let file_path = pack.base_path.join(&entry.path);
                    if is_descendant(&pack.base_path, &file_path) {
                        if let Ok(content) = std::fs::read_to_string(&file_path) {
                            return Some(ResolvedSkill {
                                name: name.to_string(),
                                pack: pack.name.clone(),
                                path: file_path,
                                content,
                            });
                        }
                    }
                }
            }
        }
        None
    }

    /// Returns a list of all unique skills across active packs, deduplicated by priority.
    pub fn list_skills(&self) -> Vec<SkillSummary> {
        let mut skill_map: HashMap<String, SkillSummary> = HashMap::new();

        // Iterate in reverse priority order (lowest priority first)
        // so higher priority overwrites them in the map.
        let mut sorted_reverse = self.active_packs.clone();
        sorted_reverse.sort_by_key(|p| std::cmp::Reverse(p.priority));

        for pack in sorted_reverse {
            for (skill_name, entry) in &pack.tile.skills {
                let description = entry
                    .description
                    .clone()
                    .unwrap_or_else(|| "No description provided.".to_string());
                skill_map.insert(
                    skill_name.clone(),
                    SkillSummary {
                        name: skill_name.clone(),
                        pack: pack.name.clone(),
                        description,
                    },
                );
            }
        }

        let mut skills: Vec<SkillSummary> = skill_map.into_values().collect();
        skills.sort_by(|a, b| a.name.cmp(&b.name));
        skills
    }

    /// Returns a list of all unique agents across active packs, deduplicated by priority.
    pub fn list_agents(&self) -> Vec<SkillSummary> {
        let mut agent_map: HashMap<String, SkillSummary> = HashMap::new();

        let mut sorted_reverse = self.active_packs.clone();
        sorted_reverse.sort_by_key(|p| std::cmp::Reverse(p.priority));

        for pack in sorted_reverse {
            if let Some(ref agents) = pack.tile.agents {
                for (agent_name, entry) in agents {
                    let description = entry
                        .description
                        .clone()
                        .unwrap_or_else(|| "No description provided.".to_string());
                    agent_map.insert(
                        agent_name.clone(),
                        SkillSummary {
                            name: agent_name.clone(),
                            pack: pack.name.clone(),
                            description,
                        },
                    );
                }
            }
        }

        let mut agents: Vec<SkillSummary> = agent_map.into_values().collect();
        agents.sort_by(|a, b| a.name.cmp(&b.name));
        agents
    }

    /// Validates that all pack dependencies (`depends_on`) are satisfied among active packs.
    ///
    /// Returns a list of missing dependency warning strings.
    pub fn validate_dependencies(&self) -> Vec<String> {
        let mut warnings = Vec::new();
        let loaded_names: std::collections::HashSet<&str> =
            self.active_packs.iter().map(|p| p.name.as_str()).collect();

        for pack in &self.active_packs {
            if let Some(ref deps) = pack.tile.depends_on {
                for dep in deps {
                    if !loaded_names.contains(dep.as_str()) {
                        warnings.push(format!(
                            "Pack '{}' depends on '{}', which is not loaded.",
                            pack.name, dep
                        ));
                    }
                }
            }
        }
        warnings
    }

    /// Check if a skill name is deprecated, returning the warning message if so.
    pub fn check_deprecated(&self, name: &str) -> Option<String> {
        self.deprecated_index.get(name).map(|dep| {
            if let Some(ref ver) = dep.removed_in {
                format!(
                    "Skill '{}' is deprecated: {}. It will be removed in version {}.",
                    name, dep.message, ver
                )
            } else {
                format!("Skill '{}' is deprecated: {}.", name, dep.message)
            }
        })
    }

    /// Direct access to loaded packs (useful for tool lists/status).
    pub fn active_packs(&self) -> &[LoadedPack] {
        &self.active_packs
    }

    /// Gets dependency list for a specific agent.
    pub fn get_agent_dependencies(&self, name: &str) -> Option<Vec<String>> {
        for pack in &self.active_packs {
            if let Some(ref agents) = pack.tile.agents {
                if let Some(entry) = agents.get(name) {
                    return entry.depends_on.clone();
                }
            }
        }
        None
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::uninlined_format_args,
    clippy::similar_names
)]
mod tests {
    use super::*;
    use crate::mcp::skill_tools::{
        ListAgentsTool, ListPacksTool, ListSkillsTool, UseAgentTool, UseSkillTool,
    };
    use crate::registry::tile::{AgentEntry, DeprecatedEntry, SkillEntry};
    use crate::registry::tool::Tool;
    use std::sync::Arc;

    struct TempPack {
        dir: PathBuf,
    }

    impl TempPack {
        fn new(name: &str) -> Self {
            let mut dir = std::env::temp_dir();
            let unique = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            dir.push(format!("agent_mcp_test_{}_{}", name, unique));
            std::fs::create_dir_all(&dir).unwrap();
            Self { dir }
        }

        fn write_file(&self, path: &str, content: &str) {
            let full_path = self.dir.join(path);
            if let Some(parent) = full_path.parent() {
                std::fs::create_dir_all(parent).unwrap();
            }
            std::fs::write(&full_path, content).unwrap();
        }
    }

    impl Drop for TempPack {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }

    fn create_dummy_manifest(name: &str) -> TileManifest {
        let mut skills = HashMap::new();
        skills.insert(
            "test-skill".to_string(),
            SkillEntry {
                path: "skills/test_skill.md".to_string(),
                description: Some(format!("Test skill for {}", name)),
                tags: None,
            },
        );

        let mut agents = HashMap::new();
        agents.insert(
            "test-agent".to_string(),
            AgentEntry {
                path: "agents/test_agent.md".to_string(),
                description: Some(format!("Test agent for {}", name)),
                depends_on: Some(vec!["test-skill".to_string()]),
            },
        );

        TileManifest {
            name: name.to_string(),
            version: "1.0.0".to_string(),
            summary: Some(format!("Summary of {}", name)),
            depends_on: None,
            skills,
            agents: Some(agents),
            deprecated_skills: None,
        }
    }

    #[test]
    fn test_framework_pack_wins_over_core() {
        let core_pack = TempPack::new("core");
        core_pack.write_file("skills/test_skill.md", "core content");
        let core_manifest = create_dummy_manifest("core");

        let rails_pack = TempPack::new("rails");
        rails_pack.write_file("skills/test_skill.md", "rails content");
        let rails_manifest = create_dummy_manifest("rails");

        let resolver = RegistryResolver::new(vec![
            LoadedPack {
                name: "core".to_string(),
                tile: core_manifest,
                base_path: core_pack.dir.clone(),
                priority: 20,
            },
            LoadedPack {
                name: "rails".to_string(),
                tile: rails_manifest,
                base_path: rails_pack.dir.clone(),
                priority: 10,
            },
        ]);

        let resolved = resolver.resolve_skill("test-skill").unwrap();
        assert_eq!(resolved.content, "rails content");
        assert_eq!(resolved.pack, "rails");
    }

    #[test]
    fn test_local_registry_wins_over_framework() {
        let local_pack = TempPack::new("local");
        local_pack.write_file("skills/test_skill.md", "local content");
        let local_manifest = create_dummy_manifest("local");

        let rails_pack = TempPack::new("rails");
        rails_pack.write_file("skills/test_skill.md", "rails content");
        let rails_manifest = create_dummy_manifest("rails");

        let resolver = RegistryResolver::new(vec![
            LoadedPack {
                name: "local".to_string(),
                tile: local_manifest,
                base_path: local_pack.dir.clone(),
                priority: 0,
            },
            LoadedPack {
                name: "rails".to_string(),
                tile: rails_manifest,
                base_path: rails_pack.dir.clone(),
                priority: 10,
            },
        ]);

        let resolved = resolver.resolve_skill("test-skill").unwrap();
        assert_eq!(resolved.content, "local content");
        assert_eq!(resolved.pack, "local");
    }

    #[test]
    fn test_warns_when_depends_on_not_satisfied() {
        let mut rails_manifest = create_dummy_manifest("rails");
        rails_manifest.depends_on = Some(vec!["core".to_string()]);

        let resolver = RegistryResolver::new(vec![LoadedPack {
            name: "rails".to_string(),
            tile: rails_manifest,
            base_path: PathBuf::from("/dummy"),
            priority: 10,
        }]);

        let warnings = resolver.validate_dependencies();
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("depends on 'core', which is not loaded"));
    }

    #[test]
    fn test_passes_when_all_deps_loaded() {
        let mut rails_manifest = create_dummy_manifest("rails");
        rails_manifest.depends_on = Some(vec!["core".to_string()]);
        let core_manifest = create_dummy_manifest("core");

        let resolver = RegistryResolver::new(vec![
            LoadedPack {
                name: "rails".to_string(),
                tile: rails_manifest,
                base_path: PathBuf::from("/dummy1"),
                priority: 10,
            },
            LoadedPack {
                name: "core".to_string(),
                tile: core_manifest,
                base_path: PathBuf::from("/dummy2"),
                priority: 20,
            },
        ]);

        let warnings = resolver.validate_dependencies();
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_deprecated_skill_resolves_with_warning() {
        let pack = TempPack::new("pack");
        pack.write_file("skills/new_skill.md", "new content");

        let mut tile = create_dummy_manifest("pack");
        tile.skills.insert(
            "new-skill".to_string(),
            SkillEntry {
                path: "skills/new_skill.md".to_string(),
                description: Some("New skill".to_string()),
                tags: None,
            },
        );
        let mut deprecated = HashMap::new();
        deprecated.insert(
            "old-skill".to_string(),
            DeprecatedEntry {
                moved_to: "new-skill".to_string(),
                message: "Use new-skill instead".to_string(),
                removed_in: Some("v2.0.0".to_string()),
            },
        );
        tile.deprecated_skills = Some(deprecated);

        let resolver = RegistryResolver::new(vec![LoadedPack {
            name: "pack".to_string(),
            tile,
            base_path: pack.dir.clone(),
            priority: 10,
        }]);

        assert!(resolver.check_deprecated("old-skill").is_some());
        assert!(resolver.check_deprecated("new-skill").is_none());

        let resolved = resolver.resolve_skill("old-skill").unwrap();
        assert_eq!(resolved.content, "new content");
        assert_eq!(resolved.name, "new-skill");
    }

    #[tokio::test]
    async fn test_mcp_tools_skill_and_agent() -> Result<(), anyhow::Error> {
        let pack = TempPack::new("mcp_pack");
        pack.write_file("skills/test_skill.md", "skill instructions");
        pack.write_file("agents/test_agent.md", "agent workflow");

        let tile = create_dummy_manifest("mcp_pack");
        let resolver = Arc::new(RegistryResolver::new(vec![LoadedPack {
            name: "mcp_pack".to_string(),
            tile,
            base_path: pack.dir.clone(),
            priority: 10,
        }]));

        let list_skills_tool = ListSkillsTool {
            resolver: Arc::clone(&resolver),
        };
        let skills_list = list_skills_tool.call("").await?;
        assert!(skills_list.contains("test-skill"));

        let use_skill_tool = UseSkillTool {
            resolver: Arc::clone(&resolver),
        };
        let skill_content = use_skill_tool.call("test-skill").await?;
        assert_eq!(skill_content, "skill instructions");

        let list_agents_tool = ListAgentsTool {
            resolver: Arc::clone(&resolver),
        };
        let agents_list = list_agents_tool.call("").await?;
        assert!(agents_list.contains("test-agent"));

        let use_agent_tool = UseAgentTool {
            resolver: Arc::clone(&resolver),
        };
        let agent_content = use_agent_tool.call("test-agent").await?;
        assert!(agent_content.contains("agent workflow"));
        assert!(agent_content.contains("test-skill")); // dependency catalog list

        let list_packs_tool = ListPacksTool {
            resolver: Arc::clone(&resolver),
        };
        let packs_list = list_packs_tool.call("").await?;
        assert!(packs_list.contains("mcp_pack"));

        Ok(())
    }
}
