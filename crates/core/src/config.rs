//! Versioned project configuration (`.context.toml`, TOML).
//!
//! ROADMAP Fase 3 + ADR 0005:
//! - TOML, not YAML;
//! - `version = 1` required;
//! - missing file => safe defaults;
//! - invalid file => clear error;
//! - unknown version => controlled failure;
//! - never execute config as code.

use serde::{Deserialize, Serialize};

use crate::error::ConfigError;
use crate::{CONFIG_SCHEMA_VERSION, branding::CONFIG_FILENAME};
use std::path::{Path, PathBuf};

/// Root configuration object. Mirrors the example in ROADMAP Fase 3.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppConfig {
    /// Schema version. Must equal `CONFIG_SCHEMA_VERSION`.
    pub version: u32,

    #[serde(default)]
    pub project: ProjectConfig,
    #[serde(default)]
    pub index: IndexConfig,
    #[serde(default)]
    pub security: SecurityConfig,
    #[serde(default)]
    pub retrieval: RetrievalConfig,
    #[serde(default)]
    pub context: ContextConfig,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectConfig {
    #[serde(default)]
    pub name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexConfig {
    #[serde(default = "default_true")]
    pub include_docs: bool,
    #[serde(default = "default_true")]
    pub include_tests: bool,
    /// Extra ignore globs on top of defaults + .gitignore (future: `ignore` crate).
    #[serde(default)]
    pub extra_ignores: Vec<String>,
    /// Max file size in bytes before truncation/exclusion (default 512 KiB).
    #[serde(default = "default_max_file_bytes")]
    pub max_file_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecurityConfig {
    #[serde(default = "default_true")]
    pub exclude_secrets: bool,
    #[serde(default)]
    pub extra_excludes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetrievalConfig {
    #[serde(default)]
    pub semantic: bool,
    #[serde(default = "default_limit")]
    pub default_limit: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextConfig {
    #[serde(default = "default_budget")]
    pub default_budget: usize,
}

const fn default_true() -> bool {
    true
}
const fn default_limit() -> usize {
    10
}
const fn default_budget() -> usize {
    30_000
}
const fn default_max_file_bytes() -> u64 {
    512 * 1024
}

impl Default for IndexConfig {
    fn default() -> Self {
        Self {
            include_docs: true,
            include_tests: true,
            extra_ignores: Vec::new(),
            max_file_bytes: default_max_file_bytes(),
        }
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self { exclude_secrets: true, extra_excludes: Vec::new() }
    }
}

impl Default for RetrievalConfig {
    fn default() -> Self {
        Self { semantic: false, default_limit: default_limit() }
    }
}

impl Default for ContextConfig {
    fn default() -> Self {
        Self { default_budget: default_budget() }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            version: CONFIG_SCHEMA_VERSION,
            project: ProjectConfig::default(),
            index: IndexConfig::default(),
            security: SecurityConfig::default(),
            retrieval: RetrievalConfig::default(),
            context: ContextConfig::default(),
        }
    }
}

impl AppConfig {
    /// Load `.context.toml` from `dir`, or return safe defaults when absent.
    pub fn load_from_dir(dir: &Path) -> Result<Self, ConfigError> {
        let path = dir.join(CONFIG_FILENAME);
        Self::load_from_file(&path)
    }

    /// Load an explicit file path; missing file => defaults (Fase 3 criterion).
    pub fn load_from_file(path: &Path) -> Result<Self, ConfigError> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = std::fs::read_to_string(path)
            .map_err(|_| ConfigError::NotFound { path: display_path(path) })?;
        Self::parse(&text, path)
    }

    /// Parse TOML text with version gate.
    pub fn parse(text: &str, origin: &Path) -> Result<Self, ConfigError> {
        let cfg: Self = toml::from_str(text)
            .map_err(|source| ConfigError::Parse { path: display_path(origin), source })?;
        cfg.validate()?;
        Ok(cfg)
    }

    /// Validate invariants (version gate + sane limits).
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.version != CONFIG_SCHEMA_VERSION {
            return Err(ConfigError::UnsupportedVersion {
                found: self.version,
                supported: CONFIG_SCHEMA_VERSION,
            });
        }
        if self.retrieval.default_limit == 0 || self.retrieval.default_limit > 1000 {
            return Err(ConfigError::InvalidValue {
                field: "retrieval.default_limit",
                reason: "must be between 1 and 1000".to_string(),
            });
        }
        if self.context.default_budget == 0 || self.context.default_budget > 1_000_000 {
            return Err(ConfigError::InvalidValue {
                field: "context.default_budget",
                reason: "must be between 1 and 1_000_000".to_string(),
            });
        }
        Ok(())
    }

    /// Serialize back to TOML (for `init`).
    pub fn to_toml_string(&self) -> String {
        toml::to_string_pretty(self).unwrap_or_else(|_| String::new())
    }
}

fn display_path(p: &Path) -> String {
    p.display().to_string()
}

/// Locate the config file path for a project root (helper for CLI).
#[must_use]
pub fn config_path_for_root(root: &Path) -> PathBuf {
    root.join(CONFIG_FILENAME)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn example_toml() -> &'static str {
        r#"
version = 1

[project]
name = "example"

[index]
include_docs = true
include_tests = true

[security]
exclude_secrets = true

[retrieval]
semantic = false
default_limit = 10

[context]
default_budget = 30000
"#
    }

    #[test]
    fn parses_roadmap_example() {
        let cfg = AppConfig::parse(example_toml(), &PathBuf::from(".context.toml")).unwrap();
        assert_eq!(cfg.version, 1);
        assert_eq!(cfg.project.name.as_deref(), Some("example"));
        assert!(!cfg.retrieval.semantic);
    }

    #[test]
    fn missing_file_gives_defaults() {
        let cfg =
            AppConfig::load_from_file(&PathBuf::from("definitely-missing-.context.toml")).unwrap();
        assert_eq!(cfg, AppConfig::default());
    }

    #[test]
    fn rejects_unknown_version() {
        let bad = "version = 99\n";
        let err = AppConfig::parse(bad, &PathBuf::from("x")).unwrap_err();
        assert!(matches!(err, ConfigError::UnsupportedVersion { found: 99, .. }));
    }

    #[test]
    fn rejects_invalid_limit() {
        let mut cfg = AppConfig::default();
        cfg.retrieval.default_limit = 0;
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn defaults_are_safe() {
        let cfg = AppConfig::default();
        assert!(cfg.security.exclude_secrets);
        assert!(!cfg.retrieval.semantic);
    }
}
