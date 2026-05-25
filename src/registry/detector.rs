//! Gemfile framework auto-detector module.

#![allow(clippy::must_use_candidate, clippy::option_if_let_else)]

use std::path::Path;

/// Frameworks that can be automatically detected in a Ruby project.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetectedFramework {
    /// Ruby on Rails framework.
    Rails,
    /// Hanami framework.
    Hanami,
}

/// Utility for detecting frameworks in the local project workspace.
pub struct PackDetector;

impl PackDetector {
    /// Detects frameworks by checking the `Gemfile` in the current working directory.
    #[must_use]
    pub fn detect() -> Vec<DetectedFramework> {
        Self::detect_in_path(Path::new("."))
    }

    /// Detects frameworks by checking the `Gemfile` in a given base directory path.
    pub fn detect_in_path(base_path: &Path) -> Vec<DetectedFramework> {
        let gemfile_path = base_path.join("Gemfile");
        if let Ok(content) = std::fs::read_to_string(&gemfile_path) {
            Self::detect_from_content(&content)
        } else {
            Vec::new()
        }
    }

    /// Pure function that parses `Gemfile` file contents to detect frameworks.
    pub fn detect_from_content(content: &str) -> Vec<DetectedFramework> {
        let mut detected = Vec::new();
        let mut rails_found = false;
        let mut hanami_found = false;

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with('#') {
                continue;
            }
            if !rails_found
                && (trimmed.contains("gem 'rails'")
                    || trimmed.contains("gem \"rails\"")
                    || trimmed.contains("gem 'rails',")
                    || trimmed.contains("gem \"rails\","))
            {
                rails_found = true;
                detected.push(DetectedFramework::Rails);
            }
            if !hanami_found
                && (trimmed.contains("gem 'hanami'")
                    || trimmed.contains("gem \"hanami\"")
                    || trimmed.contains("gem 'hanami',")
                    || trimmed.contains("gem \"hanami\","))
            {
                hanami_found = true;
                detected.push(DetectedFramework::Hanami);
            }
        }
        detected
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_rails() {
        let content = "source 'https://rubygems.org'\ngem 'rails', '~> 7.0'\n";
        assert_eq!(
            PackDetector::detect_from_content(content),
            vec![DetectedFramework::Rails]
        );
    }

    #[test]
    fn test_detect_hanami() {
        let content = "source 'https://rubygems.org'\ngem \"hanami\", \"~> 2.0\"\n";
        assert_eq!(
            PackDetector::detect_from_content(content),
            vec![DetectedFramework::Hanami]
        );
    }

    #[test]
    fn test_detect_both() {
        let content = "gem 'rails'\ngem 'hanami'";
        assert_eq!(
            PackDetector::detect_from_content(content),
            vec![DetectedFramework::Rails, DetectedFramework::Hanami]
        );
    }

    #[test]
    fn test_detect_none() {
        let content = "gem 'rspec'\ngem 'rake'";
        assert!(PackDetector::detect_from_content(content).is_empty());
    }

    #[test]
    fn test_detect_commented_gems() {
        let content = "# gem 'rails'\n#gem 'hanami'\ngem 'rails'";
        assert_eq!(
            PackDetector::detect_from_content(content),
            vec![DetectedFramework::Rails]
        );
    }
}
