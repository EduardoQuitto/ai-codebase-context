//! `aicc-context`: context compiler v1 contracts (Fase 12 foundation).
//!
//! Foundation: deterministic char-based size estimator + budget selection
//! helpers. Full retrieval + tiered compression arrive in Fase 12-13.

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ContextError {
    #[error("budget must be > 0")]
    InvalidBudget,
    #[error("context build failed: {0}")]
    Build(String),
}

/// Detail tier (Fase 13). Foundation only models the enum + labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContextTier {
    /// Path only.
    Path,
    /// Structural summary / metadata.
    Summary,
    /// Symbols and signatures.
    Symbols,
    /// Relevant snippet.
    Snippet,
    /// Full source.
    Full,
}

impl ContextTier {
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Path => "path",
            Self::Summary => "summary",
            Self::Symbols => "symbols",
            Self::Snippet => "snippet",
            Self::Full => "full",
        }
    }
}

/// A request to build context for a task/question.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextRequest {
    pub task: String,
    pub budget: usize,
    pub tier: ContextTier,
}

impl ContextRequest {
    pub fn new(task: impl Into<String>, budget: usize) -> Result<Self, ContextError> {
        if budget == 0 {
            return Err(ContextError::InvalidBudget);
        }
        Ok(Self { task: task.into(), budget, tier: ContextTier::Snippet })
    }
}

/// Deterministic v1 size estimator: ~4 chars per token (documented heuristic).
/// Real tokenizers arrive per-adapter later; this estimator is stable and
/// offline, suitable for budget enforcement tests.
#[must_use]
pub fn estimate_tokens(text: &str) -> usize {
    (text.len() / 4).max(1)
}

/// Select items greedily by score while respecting `budget`.
/// Returns selected indices + a flag indicating budget exhaustion.
pub fn select_within_budget(sizes: &[usize], scores: &[f32], budget: usize) -> (Vec<usize>, bool) {
    let mut order: Vec<usize> = (0..sizes.len()).collect();
    order.sort_by(|&a, &b| {
        scores
            .get(b)
            .unwrap_or(&0.0)
            .partial_cmp(scores.get(a).unwrap_or(&0.0))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let mut selected = Vec::new();
    let mut used = 0usize;
    let mut exhausted = false;
    for i in order {
        let s = sizes.get(i).copied().unwrap_or(0);
        if used + s <= budget {
            selected.push(i);
            used += s;
        } else {
            exhausted = true;
        }
    }
    selected.sort_unstable();
    (selected, exhausted)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn estimator_is_deterministic() {
        assert_eq!(estimate_tokens("abcd"), 1);
        assert_eq!(estimate_tokens(&"x".repeat(400)), 100);
    }

    #[test]
    fn budget_selection_respects_limit() {
        let (sel, exhausted) = select_within_budget(&[10, 10, 10], &[0.9, 0.5, 0.1], 20);
        assert_eq!(sel, vec![0, 1]);
        assert!(exhausted);
    }
}
