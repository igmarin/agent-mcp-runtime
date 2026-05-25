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
        if other.schema.is_some() {
            self.schema = other.schema;
        }
        if other.routes.is_some() {
            self.routes = other.routes;
        }
        if other.controllers.is_some() {
            self.controllers = other.controllers;
        }
        if other.models.is_some() {
            self.models = other.models;
        }
        if other.config.is_some() {
            self.config = other.config;
        }
        if other.gems.is_some() {
            self.gems = other.gems;
        }
        if other.tests.is_some() {
            self.tests = other.tests;
        }
    }
}
