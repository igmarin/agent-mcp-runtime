//! Frontmatter parser module for extracting skill metadata from Markdown files.

use serde::{Deserialize, Serialize};

/// Skill metadata structure holding name, version, and description of the skill.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillMetadata {
    /// Unique name of the skill.
    pub name: String,
    /// Semantic version of the skill.
    pub version: String,
    /// Human-readable description of what the skill does.
    pub description: String,
}

/// Parser for markdown frontmatter metadata.
pub struct FrontmatterParser;

impl FrontmatterParser {
    /// Parses frontmatter content from the given markdown string.
    ///
    /// # Errors
    ///
    /// Returns an error if the frontmatter is missing, invalid, or lacks required fields.
    pub fn parse(content: &str) -> Result<SkillMetadata, anyhow::Error> {
        let trimmed = content.trim_start();
        if !trimmed.starts_with("---") {
            anyhow::bail!("Missing frontmatter starting delimiter '---'");
        }
        let rest = &trimmed[3..];
        let end_index = rest
            .find("---")
            .ok_or_else(|| anyhow::anyhow!("Missing frontmatter ending delimiter '---'"))?;
        let yaml_content = &rest[..end_index];
        let metadata: SkillMetadata = serde_yaml::from_str(yaml_content)?;
        Ok(metadata)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_successfully_parses_valid_frontmatter() -> Result<(), anyhow::Error> {
        let markdown_input = r#"---
name: generate-api-collection
version: 1.0.0
description: Use when creating REST API endpoints.
---
# Actual content down here..."#;

        let metadata = FrontmatterParser::parse(markdown_input)?;

        // Asserting success and expected structural equality
        assert_eq!(metadata.name, "generate-api-collection");
        assert_eq!(metadata.version, "1.0.0");
        Ok(())
    }

    #[test]
    fn test_returns_error_on_missing_fields() {
        let invalid_input = r#"---
name: incomplete-skill
---"#;
        let result = FrontmatterParser::parse(invalid_input);
        assert!(result.is_err());
    }
}
