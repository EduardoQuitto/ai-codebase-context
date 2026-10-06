//! Typed errors: user-facing reason + internal diagnostic context.
//!
//! Policy (ROADMAP §56, Fase 2):
//! - normal failures print a short message + reason, no stack trace;
//! - `--verbose` / `RUST_LOG=debug` may add diagnostics via `tracing`;
//! - one file failing must not kill a whole index run (handled by callers).

use miette::Diagnostic;
use thiserror::Error;

/// Unified error for the foundation crates.
#[derive(Debug, Error, Diagnostic)]
pub enum CoreError {
    #[error("configuration error: {0}")]
    Config(#[from] ConfigError),

    #[error("project discovery failed")]
    Discovery(#[from] DiscoveryError),

    #[error("security policy violation: {0}")]
    Security(String),

    #[error("I/O error: {0}")]
    Io(String),

    #[error("feature not yet implemented: {0}")]
    NotImplemented(&'static str),
}

/// Configuration errors (Fase 3 contract).
#[derive(Debug, Error, Diagnostic)]
pub enum ConfigError {
    #[error("config file not found: {path}")]
    NotFound { path: String },

    #[error("config file could not be parsed: {path}")]
    Parse {
        path: String,
        #[source]
        source: toml::de::Error,
    },

    #[error("unsupported config version {found}; supported version is {supported}")]
    UnsupportedVersion { found: u32, supported: u32 },

    #[error("invalid config value in `{field}`: {reason}")]
    InvalidValue { field: &'static str, reason: String },
}

/// Project root discovery errors.
#[derive(Debug, Error, Diagnostic)]
pub enum DiscoveryError {
    #[error("repository root could not be determined")]
    RootNotFound {
        #[help]
        help: &'static str,
    },

    #[error("path does not exist or is not accessible: {path}")]
    Inaccessible { path: String },
}

/// Security classification errors (conservative; see `security` module).
#[derive(Debug, Error, Diagnostic)]
pub enum SecurityError {
    #[error("path escapes project root: {path}")]
    PathTraversal { path: String },
}

impl CoreError {
    /// Short user-facing reason (stable, no internals/secrets).
    #[must_use]
    pub fn user_message(&self) -> String {
        self.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_message_does_not_panic() {
        let e = CoreError::NotImplemented("mcp server (Fase 19)");
        assert!(e.user_message().contains("not yet implemented"));
    }
}
