//! `aicc-retrieval`: hybrid retrieval engine (Fase 11-14 foundation).
//!
//! Foundation: query/result contracts + deterministic budget-agnostic
//! scoring inputs. FTS5, embeddings and graph expansion arrive later.
//! Every result must be able to explain *why* it was selected.

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RetrievalError {
    #[error("retrieval failed: {0}")]
    Engine(String),
    #[error("index not ready (run `aicc index` first)")]
    IndexNotReady,
}

/// A retrieval intent (lexical today; semantic/structural later).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Query {
    pub text: String,
    pub limit: usize,
    pub language: Option<String>,
}

impl Query {
    #[must_use]
    pub fn new(text: impl Into<String>, limit: usize) -> Self {
        Self { text: text.into(), limit: limit.clamp(1, 1000), language: None }
    }
}

/// Why a candidate was selected (auditability, ROADMAP Fase 14).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MatchReason {
    LexicalMatch,
    SymbolMatch,
    ReferencedBy(String),
    NearEntryPoint,
    PathMatch,
}

/// A ranked candidate with provenance. Scores are normalized 0.0–1.0.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RetrievalResult {
    pub rel_path: String,
    pub start_line: u32,
    pub end_line: u32,
    pub score: f32,
    pub reasons: Vec<MatchReason>,
}

impl RetrievalResult {
    #[must_use]
    pub fn provenance(&self) -> String {
        format!("{}:{}-{}", self.rel_path, self.start_line, self.end_line)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provenance_format() {
        let r = RetrievalResult {
            rel_path: "src/auth.ts".to_string(),
            start_line: 42,
            end_line: 118,
            score: 0.87,
            reasons: vec![MatchReason::LexicalMatch],
        };
        assert_eq!(r.provenance(), "src/auth.ts:42-118");
    }
}
