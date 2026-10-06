//! Conceptual data model (ROADMAP §5, Fase 6-9).
//!
//! Minimal foundation types only. Persistence (SQLite), embeddings and
//! full symbol graphs arrive in later milestones. Naming is intentionally
//! aligned with the roadmap so later phases don't require renames.

use serde::{Deserialize, Serialize};

/// A repository under analysis (working-tree state, not a specific commit).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Repository {
    /// Stable local identity (derived from canonical path + git id when available).
    pub id: String,
    /// Canonicalized root path (as discovered).
    pub root: String,
    /// Index schema version that produced the current index (0 = not indexed).
    pub index_schema_version: u32,
}

/// A file or indexable object (metadata only, no content here).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexedFile {
    /// Path relative to repo root, always with `/` separators.
    pub rel_path: String,
    /// Detected language id (e.g. `typescript`), or `None` for unknown.
    pub language: Option<String>,
    /// File size in bytes.
    pub size_bytes: u64,
    /// Content hash (hex, e.g. SHA-256 or blake; algorithm versioned separately).
    pub content_hash: String,
    /// Inclusion status after ignore + security policy.
    pub status: FileStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileStatus {
    Included,
    Ignored,
    ExcludedByPolicy,
    TooLarge,
    Binary,
}

/// A retrievable unit of content (Fase 9). Content lives in the index, not here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Chunk {
    pub id: String,
    pub file_rel_path: String,
    pub start_line: u32,
    pub end_line: u32,
    pub kind: ChunkKind,
    /// Deterministic size estimate (chars-based v1; tokenizer later).
    pub size_estimate: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChunkKind {
    FileHeader,
    Symbol,
    Section,
    TextWindow,
    Configuration,
    Documentation,
}

/// A structural symbol (function, class, interface, …). Positions are 1-based.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Symbol {
    pub id: String,
    pub name: String,
    pub kind: SymbolKind,
    pub file_rel_path: String,
    pub start_line: u32,
    pub end_line: u32,
    pub parent: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SymbolKind {
    Function,
    Method,
    Class,
    Struct,
    Interface,
    Enum,
    Type,
    Constant,
    Module,
    Unknown,
}

/// A typed structural relation (Fase 8). Confidence is explicit; never invent edges.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Relationship {
    pub from: String,
    pub to: String,
    pub kind: RelationKind,
    pub confidence: Confidence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelationKind {
    Imports,
    Exports,
    Contains,
    References,
    Calls,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Confidence {
    High,
    Medium,
    Low,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_serializes_to_json() {
        let f = IndexedFile {
            rel_path: "src/main.rs".to_string(),
            language: Some("rust".to_string()),
            size_bytes: 42,
            content_hash: "abc".to_string(),
            status: FileStatus::Included,
        };
        let v = serde_json::to_value(&f).unwrap();
        assert_eq!(v["rel_path"], "src/main.rs");
    }
}
