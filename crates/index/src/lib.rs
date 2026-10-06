//! `aicc-index`: persistent local index (Fase 10 foundation).
//!
//! Foundation: schema version gate + index path resolution. SQLite/FTS5
//! tables arrive in Milestone B (see ADR 0003). No network, no external DB.

use aicc_core::platform::index_db_path;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum IndexError {
    #[error("index schema mismatch: found {found}, supported {supported}")]
    SchemaMismatch { found: u32, supported: u32 },
    #[error("index storage error: {0}")]
    Storage(String),
    #[error("index not found for repository `{0}` (run `aicc index`)")]
    NotFound(String),
}

/// Metadata about a local index (stored in SQLite `metadata` table later).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexMetadata {
    pub repo_id: String,
    pub schema_version: u32,
    pub db_path: PathBuf,
}

impl IndexMetadata {
    #[must_use]
    pub fn for_repo(repo_id: &str) -> Self {
        Self {
            repo_id: repo_id.to_string(),
            schema_version: aicc_core::INDEX_SCHEMA_VERSION,
            db_path: index_db_path(repo_id),
        }
    }

    /// Validate a stored schema version (detect + report, never migrate silently).
    pub fn check_schema(found: u32) -> Result<(), IndexError> {
        let supported = aicc_core::INDEX_SCHEMA_VERSION;
        if found != supported {
            return Err(IndexError::SchemaMismatch { found, supported });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_gate_rejects_mismatch() {
        assert!(IndexMetadata::check_schema(aicc_core::INDEX_SCHEMA_VERSION).is_ok());
        assert!(IndexMetadata::check_schema(999).is_err());
    }
}
