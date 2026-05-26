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

impl ProjectContext {
    /// Merges another context's data into this one, overwriting any field if the other is `Some`.
    pub fn merge(&mut self, other: Self) {
        let Self {
            schema,
            routes,
            controllers,
            models,
            config,
            gems,
            tests,
        } = other;

        if let Some(s) = schema {
            self.schema = Some(s);
        }
        if let Some(r) = routes {
            self.routes = Some(r);
        }
        if let Some(c) = controllers {
            self.controllers = Some(c);
        }
        if let Some(m) = models {
            self.models = Some(m);
        }
        if let Some(cfg) = config {
            self.config = Some(cfg);
        }
        if let Some(g) = gems {
            self.gems = Some(g);
        }
        if let Some(t) = tests {
            self.tests = Some(t);
        }
    }

    /// Updates the matching field in the context with the given content.
    ///
    /// Accepts either the canonical field names (e.g. `"schema"`, `"routes"`) or the legacy
    /// Ruby/Rails tool names (e.g. `"rails_get_schema"`, `"rails_get_routes"`).
    pub fn update_field(&mut self, field_or_tool: &str, content: String) {
        match field_or_tool {
            "schema" | "rails_get_schema" => self.schema = Some(content),
            "routes" | "rails_get_routes" => self.routes = Some(content),
            "controllers" | "rails_get_controllers" => self.controllers = Some(content),
            "models" | "rails_get_model_details" => self.models = Some(content),
            "config" | "rails_get_config" => self.config = Some(content),
            "gems" | "rails_get_gems" => self.gems = Some(content),
            "tests" | "rails_get_test_info" => self.tests = Some(content),
            _ => {}
        }
    }
}
