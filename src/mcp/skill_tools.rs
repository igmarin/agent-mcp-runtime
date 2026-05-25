//! MCP tools for working with skill packs, skills, and agents.

#![allow(
    clippy::unnecessary_literal_bound,
    clippy::format_push_string,
    clippy::if_not_else,
    clippy::uninlined_format_args
)]

use crate::registry::resolver::RegistryResolver;
use crate::registry::tool::Tool;
use async_trait::async_trait;
use std::sync::Arc;

/// Helper function to parse a name out of JSON input or raw string input.
fn parse_name_input(input: &str) -> String {
    let trimmed = input.trim();
    if let Ok(val) = serde_json::from_str::<serde_json::Value>(trimmed) {
        if let Some(obj) = val.as_object() {
            if let Some(name_val) = obj
                .get("name")
                .or_else(|| obj.get("skill_name"))
                .or_else(|| obj.get("agent_name"))
            {
                if let Some(name_str) = name_val.as_str() {
                    return name_str.to_string();
                }
            }
        }
    }
    trimmed.to_string()
}

/// Tool for listing all available skills in active packs.
pub struct ListSkillsTool {
    /// Reference to the shared registry resolver.
    pub resolver: Arc<RegistryResolver>,
}

#[async_trait]
impl Tool for ListSkillsTool {
    fn name(&self) -> &str {
        "list_skills"
    }

    fn description(&self) -> &str {
        "Returns all available skills with name, description, and source pack. Use to discover what skills are available."
    }

    async fn call(&self, _input: &str) -> Result<String, anyhow::Error> {
        let skills = self.resolver.list_skills();
        let formatted = serde_json::to_string_pretty(&skills)?;
        Ok(formatted)
    }
}

/// Tool for fetching and loading the instructions of a specific skill.
pub struct UseSkillTool {
    /// Reference to the shared registry resolver.
    pub resolver: Arc<RegistryResolver>,
}

#[async_trait]
impl Tool for UseSkillTool {
    fn name(&self) -> &str {
        "use_skill"
    }

    fn description(&self) -> &str {
        "Loads the full SKILL.md content for a skill by name. Returns the complete instructions. Input: skill name (string) or JSON {\"name\": \"skill_name\"}."
    }

    async fn call(&self, input: &str) -> Result<String, anyhow::Error> {
        let skill_name = parse_name_input(input);
        if skill_name.is_empty() {
            anyhow::bail!("Skill name input is empty.");
        }

        // Print deprecation warning to stderr if needed
        if let Some(warning) = self.resolver.check_deprecated(&skill_name) {
            eprintln!("⚠ DEPRECATED: {warning}");
        }

        if let Some(skill) = self.resolver.resolve_skill(&skill_name) {
            Ok(skill.content)
        } else {
            anyhow::bail!("Skill '{}' not found.", skill_name)
        }
    }
}

/// Tool for listing all available agents in active packs.
pub struct ListAgentsTool {
    /// Reference to the shared registry resolver.
    pub resolver: Arc<RegistryResolver>,
}

#[async_trait]
impl Tool for ListAgentsTool {
    fn name(&self) -> &str {
        "list_agents"
    }

    fn description(&self) -> &str {
        "Returns all available agents (orchestrated workflows) with name, description, and source pack."
    }

    async fn call(&self, _input: &str) -> Result<String, anyhow::Error> {
        let agents = self.resolver.list_agents();
        let formatted = serde_json::to_string_pretty(&agents)?;
        Ok(formatted)
    }
}

/// Tool for fetching and loading an agent's instructions along with its dependency catalog.
pub struct UseAgentTool {
    /// Reference to the shared registry resolver.
    pub resolver: Arc<RegistryResolver>,
}

#[async_trait]
impl Tool for UseAgentTool {
    fn name(&self) -> &str {
        "use_agent"
    }

    fn description(&self) -> &str {
        "Loads an agent's SKILL.md and resolves its full dependency tree. Returns agent instructions + all referenced skill summaries. Input: agent name (string) or JSON {\"name\": \"agent_name\"}."
    }

    async fn call(&self, input: &str) -> Result<String, anyhow::Error> {
        let agent_name = parse_name_input(input);
        if agent_name.is_empty() {
            anyhow::bail!("Agent name input is empty.");
        }

        if let Some(agent) = self.resolver.resolve_agent(&agent_name) {
            let mut response = format!(
                "# Agent: {} (Pack: {})\n\n## Instructions\n{}\n",
                agent.name, agent.pack, agent.content
            );

            if let Some(deps) = self.resolver.get_agent_dependencies(&agent_name) {
                if !deps.is_empty() {
                    response.push_str("\n## Skill Dependencies\n");
                    response.push_str(
                        "The following skills are available and referenced by this agent:\n",
                    );

                    let all_skills = self.resolver.list_skills();
                    for dep in deps {
                        if let Some(skill_summary) = all_skills.iter().find(|s| s.name == dep) {
                            response.push_str(&format!(
                                "- **{}**: {} (from pack {})\n",
                                skill_summary.name, skill_summary.description, skill_summary.pack
                            ));
                        } else {
                            response.push_str(&format!(
                                "- **{}**: [Missing/Unresolved Dependency]\n",
                                dep
                            ));
                        }
                    }
                }
            }

            Ok(response)
        } else {
            anyhow::bail!("Agent '{}' not found.", agent_name)
        }
    }
}

/// Tool for listing active packs, their properties, and validating dependencies.
pub struct ListPacksTool {
    /// Reference to the shared registry resolver.
    pub resolver: Arc<RegistryResolver>,
}

#[async_trait]
impl Tool for ListPacksTool {
    fn name(&self) -> &str {
        "list_packs"
    }

    fn description(&self) -> &str {
        "Returns loaded packs, their sources, and dependency status."
    }

    async fn call(&self, _input: &str) -> Result<String, anyhow::Error> {
        let packs = self.resolver.active_packs();
        let mut response = String::new();
        response.push_str("### Active Packs\n\n");
        response.push_str("| Pack Name | Version | Source | Priority | Base Path |\n");
        response.push_str("|---|---|---|---|---|\n");

        for pack in packs {
            response.push_str(&format!(
                "| {} | {} | {} | {} | {} |\n",
                pack.name,
                pack.tile.version,
                pack.tile.summary.as_deref().unwrap_or(""),
                pack.priority,
                pack.base_path.to_string_lossy()
            ));
        }

        let warnings = self.resolver.validate_dependencies();
        if !warnings.is_empty() {
            response.push_str("\n### Dependency Warnings\n");
            for warning in warnings {
                response.push_str(&format!("- ⚠ {}\n", warning));
            }
        } else {
            response.push_str(
                "\n### Dependencies Status\n- All pack dependencies are fully satisfied.\n",
            );
        }

        Ok(response)
    }
}
