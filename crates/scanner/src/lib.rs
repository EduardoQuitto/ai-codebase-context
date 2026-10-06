//! `aicc-scanner`: secure file discovery (Fase 4 foundation).
//!
//! Current: std-only walker with conservative defaults, binary + size
//! guards, no code execution. `.gitignore` semantics via the `ignore` crate
//! arrive in Fase 4 full implementation (see ROADMAP).
//!
//! Future: `ignore` crate, generated-code heuristics, exclusion reasons in
//! diagnostics mode.

use aicc_core::security::{Classification, classify_path};
use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ScanError {
    #[error("scan root is not accessible: {0}")]
    Inaccessible(String),
    #[error("I/O error while scanning: {0}")]
    Io(String),
}

/// Why a path was skipped (diagnostics only, never secret contents).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkipReason {
    DefaultIgnore,
    TooLarge,
    Binary,
    Sensitive,
    ExtraIgnore,
}

/// A candidate file found by the scanner.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScannedEntry {
    pub rel_path: String,
    pub size_bytes: u64,
    pub classification: Classification,
}

/// Options for a scan (subset of `AppConfig::index`).
#[derive(Debug, Clone)]
pub struct ScanOptions {
    pub max_file_bytes: u64,
    pub extra_ignores: Vec<String>,
}

impl Default for ScanOptions {
    fn default() -> Self {
        Self { max_file_bytes: 512 * 1024, extra_ignores: Vec::new() }
    }
}

/// Default-ignored directory names (ROADMAP Fase 4). Configurable later.
pub const DEFAULT_IGNORED_DIRS: &[&str] = &[
    ".git",
    "node_modules",
    "target",
    "dist",
    "build",
    "out",
    "coverage",
    ".cache",
    "venv",
    ".venv",
    "__pycache__",
];

/// Walk `root` and return included candidates + skip list (for `--verbose`).
/// Single-threaded foundation; parallelism is a Fase 23 concern.
pub fn scan(
    root: &Path,
    options: &ScanOptions,
) -> Result<(Vec<ScannedEntry>, Vec<(String, SkipReason)>), ScanError> {
    if !root.is_dir() {
        return Err(ScanError::Inaccessible(root.display().to_string()));
    }

    let mut included = Vec::new();
    let mut skipped = Vec::new();
    let mut stack = vec![root.to_path_buf()];

    while let Some(dir) = stack.pop() {
        let entries = std::fs::read_dir(&dir).map_err(|e| ScanError::Io(e.to_string()))?;
        for entry in entries.flatten() {
            let path = entry.path();
            let rel = path.strip_prefix(root).unwrap_or(&path).to_string_lossy().replace('\\', "/");

            if path.is_dir() {
                let name = entry.file_name().to_string_lossy().into_owned();
                if DEFAULT_IGNORED_DIRS.contains(&name.as_str()) {
                    skipped.push((rel, SkipReason::DefaultIgnore));
                    continue;
                }
                if options.extra_ignores.iter().any(|g| glob_match(g, &rel)) {
                    skipped.push((rel, SkipReason::ExtraIgnore));
                    continue;
                }
                stack.push(path);
                continue;
            }

            if !path.is_file() {
                continue;
            }

            if options.extra_ignores.iter().any(|g| glob_match(g, &rel)) {
                skipped.push((rel, SkipReason::ExtraIgnore));
                continue;
            }

            let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
            let classification = classify_path(&path);

            match classification {
                Classification::Sensitive => {
                    skipped.push((rel, SkipReason::Sensitive));
                    continue;
                }
                Classification::Binary => {
                    skipped.push((rel, SkipReason::Binary));
                    continue;
                }
                Classification::Generated | Classification::Allowed => {}
            }

            if size > options.max_file_bytes {
                skipped.push((rel, SkipReason::TooLarge));
                continue;
            }

            included.push(ScannedEntry { rel_path: rel, size_bytes: size, classification });
        }
    }

    included.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
    Ok((included, skipped))
}

/// Minimal glob (`*` only) for `extra_ignores` foundation. Full glob via
/// `ignore` crate in Fase 4.
fn glob_match(pattern: &str, path: &str) -> bool {
    if pattern.contains('*') {
        let parts: Vec<&str> = pattern.split('*').collect();
        let mut rest = path;
        for (i, part) in parts.iter().enumerate() {
            if part.is_empty() {
                continue;
            }
            match rest.find(part) {
                Some(idx) => {
                    if i == 0 && idx != 0 && !pattern.starts_with('*') {
                        return false;
                    }
                    rest = &rest[idx + part.len()..];
                }
                None => return false,
            }
        }
        let last = parts.last().copied().unwrap_or("");
        pattern.ends_with('*') || path.ends_with(last)
    } else {
        path.contains(pattern)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn fixture_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("aicc-scan-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("src")).unwrap();
        fs::create_dir_all(dir.join("node_modules")).unwrap();
        fs::write(dir.join("src/main.rs"), "fn main() {}").unwrap();
        fs::write(dir.join("node_modules/dep.js"), "x").unwrap();
        fs::write(dir.join(".env"), "SECRET=1").unwrap();
        fs::write(dir.join("a.png"), [0u8, 1, 2]).unwrap();
        dir
    }

    #[test]
    fn skips_defaults_and_sensitive() {
        let dir = fixture_dir();
        let (included, skipped) = scan(&dir, &ScanOptions::default()).unwrap();
        let paths: Vec<_> = included.iter().map(|e| e.rel_path.as_str()).collect();
        assert!(paths.contains(&"src/main.rs"));
        assert!(!paths.iter().any(|p| p.contains("node_modules")));
        assert!(!paths.iter().any(|p| p.contains(".env")));
        assert!(!paths.iter().any(|p| p.ends_with(".png")));
        assert!(!skipped.is_empty());
        let _ = fs::remove_dir_all(&dir);
    }
}
