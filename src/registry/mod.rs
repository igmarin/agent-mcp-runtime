//! Registry module for managing agent tools and skills.

pub mod detector;
pub mod manifest;
pub mod pack_resolver;
pub mod parser;
pub mod resolver;
pub mod source;
pub mod tile;
pub mod tool;

pub use pack_resolver::PackResolverService;
