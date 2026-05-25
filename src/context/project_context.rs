//! Unified project context model.

use serde::{Deserialize, Serialize};

/// Unified context data containing information fetched from external providers.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct ProjectContext {
    /// Schema of the database/ActiveRecord models.
    pub schema: Option<String>,
    /// Configured routes and paths.
    pub routes: Option<String>,
    /// Controller definitions and methods.
    pub controllers: Option<String>,
    /// Model details (associations, semantic tiers).
    pub models: Option<String>,
    /// Config files or overall conventions.
    pub config: Option<String>,
    /// Gemfile/library dependencies.
    pub gems: Option<String>,
    /// Test suite configuration and statuses.
    pub tests: Option<String>,
}
