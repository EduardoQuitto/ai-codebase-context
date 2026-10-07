//! Conservative content classification (Fase 5 foundation).
//!
//! Rules:
//! - never execute repo code to classify it;
//! - secrets are excluded from default context and sanitized in diagnostics;
//! - classification is name/extension/pattern based (no "AI security" in MVP).

use serde::{Deserialize, Serialize};
use std::path::Path;

/// Classification outcome for a path (before content sniffing).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Classification {
    Allowed,
    Sensitive,
    Binary,
    Generated,
}

const SENSITIVE_FILENAMES: &[&str] = &[
    ".env",
    ".env.local",
    ".env.production",
    "id_rsa",
    "id_ed25519",
    ".pem",
    "secrets.json",
    "credentials.json",
];

const SENSITIVE_EXTENSIONS: &[&str] = &["pem", "key", "p12", "pfx", "jks"];

const BINARY_EXTENSIONS: &[&str] = &[
    "exe", "dll", "so", "dylib", "bin", "o", "a", "class", "jar", "war", "png", "jpg", "jpeg",
    "gif", "webp", "ico", "mp4", "mp3", "avi", "mov", "zip", "tar", "gz", "7z", "rar", "pdf",
    "woff", "woff2", "ttf", "otf", "eot", "sqlite", "db",
];

/// Dependency lockfiles that describe the project without being authored source.
const GENERATED_FILENAMES: &[&str] = &[
    "cargo.lock",
    "package-lock.json",
    "yarn.lock",
    "pnpm-lock.yaml",
    "poetry.lock",
    "pipfile.lock",
    "composer.lock",
    "gemfile.lock",
    "go.sum",
];

/// Conservative generated-code heuristic (Fase 4: name-based only).
///
/// Matches dependency lockfiles, minified web bundles and explicit
/// `.generated.` markers. Never reads file contents and never executes
/// anything; deeper generated-code analysis belongs to later phases.
#[must_use]
pub fn is_generated_file_name(path: &Path) -> bool {
    let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_ascii_lowercase();
    if GENERATED_FILENAMES.contains(&filename.as_str()) {
        return true;
    }
    filename.ends_with(".min.js")
        || filename.ends_with(".min.css")
        || filename.contains(".generated.")
}
/// Classify by filename/extension. Content sniffing (high-confidence secret
/// patterns) is a Fase 5 task; this is the safe baseline.
#[must_use]
pub fn classify_path(path: &Path) -> Classification {
    let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_ascii_lowercase();

    if SENSITIVE_FILENAMES.iter().any(|s| filename == *s || filename.starts_with(".env.")) {
        return Classification::Sensitive;
    }

    let ext = path.extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase);

    if ext.as_deref().is_some_and(|e| SENSITIVE_EXTENSIONS.contains(&e)) {
        return Classification::Sensitive;
    }

    // Generated before binary: derived artifacts are noise even when they
    // carry a "code-like" extension, and secrets already won above.
    // Any `*.lock` counts as generated (preserves the previous contract).
    if ext.as_deref() == Some("lock") || is_generated_file_name(path) {
        return Classification::Generated;
    }

    if ext.as_deref().is_some_and(|e| BINARY_EXTENSIONS.contains(&e)) {
        return Classification::Binary;
    }

    // Directories like `target/` are handled by the scanner, not here.
    Classification::Allowed
}

/// Sanitize a path for diagnostics: never print secret *contents*, only the
/// fact that a path was excluded.
#[must_use]
pub fn sanitize_for_log(rel_path: &str, classification: Classification) -> String {
    match classification {
        Classification::Sensitive => format!("{rel_path} [excluded: sensitive]"),
        _ => rel_path.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn env_is_sensitive() {
        assert_eq!(classify_path(&PathBuf::from(".env")), Classification::Sensitive);
        assert_eq!(classify_path(&PathBuf::from(".env.local")), Classification::Sensitive);
    }

    #[test]
    fn pem_is_sensitive() {
        assert_eq!(classify_path(&PathBuf::from("cert/server.pem")), Classification::Sensitive);
    }

    #[test]
    fn png_is_binary() {
        assert_eq!(classify_path(&PathBuf::from("a.png")), Classification::Binary);
    }

    #[test]
    fn normal_source_is_allowed() {
        assert_eq!(classify_path(&PathBuf::from("src/main.rs")), Classification::Allowed);
    }

    #[test]
    fn generated_names_are_detected() {
        assert!(is_generated_file_name(&PathBuf::from("package-lock.json")));
        assert!(is_generated_file_name(&PathBuf::from("Cargo.lock")));
        assert!(is_generated_file_name(&PathBuf::from("dist/bundle.min.js")));
        assert!(is_generated_file_name(&PathBuf::from("src/types.generated.ts")));
        assert!(!is_generated_file_name(&PathBuf::from("src/main.rs")));
        assert!(!is_generated_file_name(&PathBuf::from(".env")));
    }

    #[test]
    fn generated_classification_wins_over_binary_but_not_secrets() {
        assert_eq!(classify_path(&PathBuf::from("package-lock.json")), Classification::Generated);
        assert_eq!(classify_path(&PathBuf::from("app.min.js")), Classification::Generated);
        // Privacy first: a sensitive name is never downgraded to generated.
        assert_eq!(classify_path(&PathBuf::from(".env.generated")), Classification::Sensitive);
    }
}
