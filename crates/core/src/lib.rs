//! `aicc-core`: shared contracts for the local codebase context layer.
//!
//! Current (foundation) scope:
//! - branding (provisional, easily renamable)
//! - version / schema constants
//! - typed errors
//! - config schema (TOML, `.context.toml`, versioned)
//! - project root discovery
//! - conceptual data model (minimal, non-persistent yet)
//! - security classification (conservative defaults)
//! - platform paths (user cache dir, index outside repo)
//!
//! Planned (see ROADMAP.md, Milestones B+):
//! SQLite persistence, Tree-sitter parsing, FTS5, embeddings, MCP.

pub mod branding;
pub mod config;
pub mod discovery;
pub mod error;
pub mod model;
pub mod platform;
pub mod security;

pub use branding::{BINARY_NAME, CONFIG_FILENAME, PRODUCT_DISPLAY_NAME};
pub use config::AppConfig;
pub use error::{ConfigError, CoreError, DiscoveryError, SecurityError};

/// Index schema version for the future SQLite store.
/// Bumped only on incompatible changes; old indexes must be rebuilt.
pub const INDEX_SCHEMA_VERSION: u32 = 1;

/// Config schema version stored in `.context.toml` as `version = 1`.
pub const CONFIG_SCHEMA_VERSION: u32 = 1;

/// Crate version (mirrors workspace version).
pub const CORE_VERSION: &str = env!("CARGO_PKG_VERSION");
