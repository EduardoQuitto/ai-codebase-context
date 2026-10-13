//! `aicc-parser`: language detection + structural parsing (Fase 6-7).
//!
//! Fase 6: static extension registry + per-file metadata inputs. Detection is
//! deterministic, case-insensitive and total (never panics): unknown or
//! missing extensions yield [`Language::Unknown`].
//! The [`LanguageParser`] trait reserves the Tree-sitter seam (ADR 0004)
//! without pulling tree-sitter yet (Fase 7).

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

/// Where a language assignment came from. Fase 6 has a single source: the
/// static extension registry. Kept as an enum (not a string) so future
/// sources (shebang, content sniffing) are additive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DetectionSource {
    Extension,
}

impl DetectionSource {
    /// Stable id for filters and JSON (matches serialized form).
    #[must_use]
    pub fn id(self) -> &'static str {
        match self {
            Self::Extension => "extension",
        }
    }
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
    /// Planning only: use [`Language::has_structural_parser`] to ask what is
    /// actually implemented.
    #[must_use]
    pub fn has_v1_structural_support(self) -> bool {
        matches!(self, Self::TypeScript | Self::JavaScript | Self::Python | Self::Rust)
    }

    /// Whether a structural parser is really implemented and available.
    /// Always `false` in Fase 6: Tree-sitter grammars belong to Fase 7.
    /// Kept separate from [`Language::has_v1_structural_support`] (the plan)
    /// so callers cannot mistake the roadmap for an implementation.
    #[must_use]
    pub fn has_structural_parser(self) -> bool {
        let _ = self;
        false
    }
}

/// Detect language from file extension (case-insensitive).
///
/// Total function: missing or unknown extensions yield [`Language::Unknown`]
/// instead of panicking. `.h` is ambiguous between C and C++; it falls back
/// to [`Language::C`] (historical choice) — the distinction is heuristic, not
/// perfect. Languages without structural support remain fully eligible for
/// metadata and future textual indexing.
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
        // `.h` fallback is C by convention (see docs); C++-only headers use
        // the unambiguous aliases below.
        "c" | "h" => Language::C,
        "cpp" | "cc" | "cxx" | "hpp" | "hh" | "hxx" => Language::Cpp,
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
        assert_eq!(detect_language(&PathBuf::from("a.tsx")), Language::TypeScript);
        assert_eq!(detect_language(&PathBuf::from("a.mts")), Language::TypeScript);
        assert_eq!(detect_language(&PathBuf::from("a.cts")), Language::TypeScript);
        assert_eq!(detect_language(&PathBuf::from("a.js")), Language::JavaScript);
        assert_eq!(detect_language(&PathBuf::from("a.jsx")), Language::JavaScript);
        assert_eq!(detect_language(&PathBuf::from("a.mjs")), Language::JavaScript);
        assert_eq!(detect_language(&PathBuf::from("a.cjs")), Language::JavaScript);
        assert_eq!(detect_language(&PathBuf::from("a.py")), Language::Python);
        assert_eq!(detect_language(&PathBuf::from("a.rs")), Language::Rust);
    }

    #[test]
    fn detects_second_wave_languages() {
        assert_eq!(detect_language(&PathBuf::from("a.go")), Language::Go);
        assert_eq!(detect_language(&PathBuf::from("A.java")), Language::Java);
        assert_eq!(detect_language(&PathBuf::from("a.cs")), Language::CSharp);
        assert_eq!(detect_language(&PathBuf::from("a.c")), Language::C);
        assert_eq!(detect_language(&PathBuf::from("a.cpp")), Language::Cpp);
        assert_eq!(detect_language(&PathBuf::from("a.cc")), Language::Cpp);
        assert_eq!(detect_language(&PathBuf::from("a.hpp")), Language::Cpp);
    }

    #[test]
    fn detects_cpp_aliases() {
        for name in ["a.cxx", "a.hh", "a.hxx"] {
            assert_eq!(detect_language(&PathBuf::from(name)), Language::Cpp, "{name}");
        }
    }

    #[test]
    fn h_falls_back_to_c_and_is_documented() {
        // Ambiguous header: kept as C by convention, not a perfect split.
        assert_eq!(detect_language(&PathBuf::from("a.h")), Language::C);
    }

    #[test]
    fn detects_textual_formats() {
        assert_eq!(detect_language(&PathBuf::from("a.md")), Language::Markdown);
        assert_eq!(detect_language(&PathBuf::from("a.markdown")), Language::Markdown);
        assert_eq!(detect_language(&PathBuf::from("a.json")), Language::Json);
        assert_eq!(detect_language(&PathBuf::from("a.toml")), Language::Toml);
        assert_eq!(detect_language(&PathBuf::from("a.yaml")), Language::Yaml);
        assert_eq!(detect_language(&PathBuf::from("a.yml")), Language::Yaml);
    }

    #[test]
    fn detection_is_case_insensitive() {
        assert_eq!(detect_language(&PathBuf::from("A.TS")), Language::TypeScript);
        assert_eq!(detect_language(&PathBuf::from("B.Py")), Language::Python);
        assert_eq!(detect_language(&PathBuf::from("C.RS")), Language::Rust);
        assert_eq!(detect_language(&PathBuf::from("D.CPP")), Language::Cpp);
        assert_eq!(detect_language(&PathBuf::from("E.MD")), Language::Markdown);
    }

    #[test]
    fn unknown_for_missing_extension() {
        assert_eq!(detect_language(&PathBuf::from("Makefile")), Language::Unknown);
    }

    #[test]
    fn unknown_for_unknown_extension_and_special_names() {
        for name in ["a.zzz9", "Dockerfile", ".gitignore", "archive.tar.gz9"] {
            assert_eq!(detect_language(&PathBuf::from(name)), Language::Unknown, "{name}");
        }
    }

    #[test]
    fn language_ids_match_serialization() {
        for lang in [
            Language::TypeScript,
            Language::JavaScript,
            Language::Python,
            Language::Rust,
            Language::Go,
            Language::Java,
            Language::CSharp,
            Language::C,
            Language::Cpp,
            Language::Ruby,
            Language::Php,
            Language::Markdown,
            Language::Json,
            Language::Toml,
            Language::Yaml,
            Language::Unknown,
        ] {
            let json = serde_json::to_value(lang).expect("serializable");
            assert_eq!(json, serde_json::Value::String(lang.id().to_string()), "{lang:?}");
        }
        let source = serde_json::to_value(DetectionSource::Extension).expect("serializable");
        assert_eq!(source, serde_json::Value::String("extension".to_string()));
        assert_eq!(DetectionSource::Extension.id(), "extension");
    }

    #[test]
    fn structural_parser_is_unavailable_in_fase_6() {
        for lang in [
            Language::TypeScript,
            Language::JavaScript,
            Language::Python,
            Language::Rust,
            Language::Go,
            Language::Unknown,
        ] {
            assert!(!lang.has_structural_parser(), "{lang:?} must report no parser yet");
        }
        // The plan still names the v1 set; it is not an implementation claim.
        assert!(Language::Rust.has_v1_structural_support());
        assert!(!Language::Go.has_v1_structural_support());
    }
}
