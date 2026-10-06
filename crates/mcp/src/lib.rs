//! `aicc-mcp`: MCP adapter seam (Fase 19 planned).
//!
//! Foundation: tool names + read-only guard. No transport, no shell, no
//! writes. The server implementation arrives in Milestone D behind the
//! internal Context API (ROADMAP Fase 18).

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum McpError {
    #[error("mcp server is not yet implemented (planned: Fase 19)")]
    NotImplemented,
    #[error("denied: MCP surface is read-only; `{0}` is not allowed")]
    Denied(String),
}

/// Planned read-only tools (names frozen early for docs stability).
pub const PLANNED_TOOLS: &[&str] = &[
    "search_codebase",
    "get_file_context",
    "get_symbol",
    "get_project_map",
    "get_context",
    "get_index_status",
];

/// Guard: only planned read-only tools may ever be exposed.
#[must_use]
pub fn is_read_only_tool(name: &str) -> bool {
    PLANNED_TOOLS.contains(&name)
}

/// Planned `get_index_status` payload shape (forward-compatible stub).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexStatus {
    pub indexed: bool,
    pub files: u64,
    pub chunks: u64,
    pub symbols: u64,
    pub semantic_enabled: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_planned_tools_allowed() {
        assert!(is_read_only_tool("search_codebase"));
        assert!(!is_read_only_tool("exec_shell"));
        assert!(!is_read_only_tool("write_file"));
    }
}
