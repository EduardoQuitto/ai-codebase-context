//! Project root discovery (Fase 3).
//!
//! Walks up from `start` until a known marker is found.
//! `.git` is treated as a strong signal; `.context.toml` and well-known
//! manifests are also accepted. No I/O outside the traversal, no execution.

use crate::branding::CONFIG_FILENAME;
use crate::error::DiscoveryError;
use std::path::{Path, PathBuf};

/// Marker files/dirs that identify a project root.
/// `.git` is the strong signal; `.context.toml` and well-known manifests
/// are also accepted. (`*.sln` is matched by extension in `is_project_root`.)
const MARKERS: &[&str] = &[
    CONFIG_FILENAME,
    ".git",
    "Cargo.toml",
    "package.json",
    "pyproject.toml",
    "go.mod",
    "pom.xml",
    "build.gradle",
    "composer.json",
    "Gemfile",
];

/// Find the project root starting at `start` (file or dir).
pub fn find_root(start: &Path) -> Result<PathBuf, DiscoveryError> {
    let mut current = if start.is_file() {
        start
            .parent()
            .map(Path::to_path_buf)
            .ok_or(DiscoveryError::RootNotFound { help: "Invalid start path." })?
    } else if start.exists() {
        canonicalize_lossy(start)
    } else {
        return Err(DiscoveryError::Inaccessible { path: start.display().to_string() });
    };

    loop {
        if is_project_root(&current) {
            return Ok(current);
        }
        match current.parent() {
            Some(parent) => {
                let parent = parent.to_path_buf();
                // Stop at filesystem root.
                if parent == current {
                    break;
                }
                current = parent;
            }
            None => break,
        }
    }

    Err(DiscoveryError::RootNotFound { help: "No supported project marker was found." })
}

/// Check whether `dir` looks like a project root.
pub fn is_project_root(dir: &Path) -> bool {
    // Cheap checks first.
    for marker in MARKERS {
        if dir.join(marker).exists() {
            return true;
        }
    }
    // .NET solution files: any *.sln.
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            if entry.path().extension().is_some_and(|e| e == "sln") {
                return true;
            }
        }
    }
    false
}

/// Best-effort canonicalization (falls back to the original path).
fn canonicalize_lossy(p: &Path) -> PathBuf {
    std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_cargo_root_in_fixtures() {
        // `crates/core` itself lives under a Cargo workspace in dev.
        let here = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let root = find_root(&here).expect("should find a Cargo.toml ancestor in dev");
        assert!(root.join("Cargo.toml").exists() || is_project_root(&root));
    }

    #[test]
    fn inaccessible_path_errors() {
        let err = find_root(&PathBuf::from("/definitely/not/here/xyz-123")).unwrap_err();
        assert!(matches!(err, DiscoveryError::Inaccessible { .. }));
    }

    #[test]
    fn markers_list_covers_toml_config() {
        assert!(MARKERS.contains(&CONFIG_FILENAME));
    }
}
