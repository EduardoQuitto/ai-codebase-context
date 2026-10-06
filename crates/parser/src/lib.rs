//! `aicc-parser`: language detection + structural parsing (Fase 6-7).
//!
//! Foundation: extension-based language registry + file metadata. The
//! `LanguageParser` trait reserves the Tree-sitter seam (ADR 0004) without
//! pulling tree-sitter yet.

use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("unsupported language for: {0}")]
    Unsupported(String),
    #[error("parser failed for {path}: {reason}")]
    Failed { path: String, reason: String },
}

/// Language identifier (stable ids used in index + filters).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    TypeScript,
    JavaScript,
    Python,
    Rust,
    Go,
    Java,
    CSharp,
    C,
    Cpp,
    Ruby,
    Php,
    Markdown,
    Json,
    Toml,
    Yaml,
    Unknown,
}

impl Language {
    #[must_use]
    pub fn id(self) -> &'static str {
        match self {
            Self::TypeScript => "typescript",
            Self::JavaScript => "javascript",
            Self::Python => "python",
            Self::Rust => "rust",
            Self::Go => "go",
            Self::Java => "java",
            Self::CSharp => "csharp",
            Self::C => "c",
            Self::Cpp => "cpp",
            Self::Ruby => "ruby",
            Self::Php => "php",
            Self::Markdown => "markdown",
            Self::Json => "json",
            Self::Toml => "toml",
            Self::Yaml => "yaml",
            Self::Unknown => "unknown",
        }
    }

    /// Whether structural parsing is planned for v1 (ROADMAP Fase 7/35).
    #[must_use]
    pub fn has_v1_structural_support(self) -> bool {
        matches!(self, Self::TypeScript | Self::JavaScript | Self::Python | Self::Rust)
    }
}

/// Detect language from file extension (case-insensitive).
#[must_use]
pub fn detect_language(path: &Path) -> Language {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
    match ext.as_str() {
        "ts" | "tsx" | "mts" | "cts" => Language::TypeScript,
        "js" | "jsx" | "mjs" | "cjs" => Language::JavaScript,
        "py" => Language::Python,
        "rs" => Language::Rust,
        "go" => Language::Go,
        "java" => Language::Java,
        "cs" => Language::CSharp,
        "c" | "h" => Language::C,
        "cpp" | "cc" | "hpp" => Language::Cpp,
        "rb" => Language::Ruby,
        "php" => Language::Php,
        "md" | "markdown" => Language::Markdown,
        "json" => Language::Json,
        "toml" => Language::Toml,
        "yaml" | "yml" => Language::Yaml,
        _ => Language::Unknown,
    }
}

/// Future structural parser seam. Implementors must tolerate broken code
/// (Tree-sitter error recovery) and never execute repo code.
pub trait LanguageParser: std::fmt::Debug {
    fn language(&self) -> Language;
    fn parse_symbols(
        &self,
        _rel_path: &str,
        _content: &str,
    ) -> Result<Vec<aicc_core::model::Symbol>, ParseError> {
        Err(ParseError::Unsupported(self.language().id().to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn detects_priority_languages() {
        assert_eq!(detect_language(&PathBuf::from("a.ts")), Language::TypeScript);
        assert_eq!(detect_language(&PathBuf::from("a.py")), Language::Python);
        assert_eq!(detect_language(&PathBuf::from("a.rs")), Language::Rust);
    }

    #[test]
    fn unknown_for_missing_extension() {
        assert_eq!(detect_language(&PathBuf::from("Makefile")), Language::Unknown);
    }
}
