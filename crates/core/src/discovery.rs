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

/// Manifest markers used as the lowest-priority tier in [`find_root`].
/// Mirrors `MARKERS` minus `.git` and the config filename; `*.sln` is
/// additionally accepted via [`has_manifest_marker`].
const MANIFEST_MARKERS: &[&str] = &[
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
///
/// Deterministic tiered discovery (nearest wins within each tier):
/// 1. nearest ancestor containing `.git` (dir or file, e.g. worktrees),
/// 2. otherwise nearest ancestor containing `.context.toml`,
/// 3. otherwise nearest ancestor with a known project manifest
///    (including `*.sln`).
///
/// Walks up to the filesystem root. Never executes git, scripts or repo code.
pub fn find_root(start: &Path) -> Result<PathBuf, DiscoveryError> {
    let start_dir = if start.is_file() {
        let parent = start
            .parent()
            .map(Path::to_path_buf)
            .ok_or(DiscoveryError::RootNotFound { help: "Invalid start path." })?;
        canonicalize_lossy(&parent)
    } else if start.exists() {
        canonicalize_lossy(start)
    } else {
        return Err(DiscoveryError::Inaccessible { path: start.display().to_string() });
    };

    let ancestors = ancestors_inclusive(&start_dir);

    // Tier 1: nearest `.git` (deterministic for nested git repos).
    if let Some(hit) = ancestors.iter().find(|dir| has_git_marker(dir)) {
        return Ok(hit.clone());
    }
    // Tier 2: nearest `.context.toml`.
    if let Some(hit) = ancestors.iter().find(|dir| has_config_marker(dir)) {
        return Ok(hit.clone());
    }
    // Tier 3: nearest known manifest (preserves `.sln` support).
    if let Some(hit) = ancestors.iter().find(|dir| has_manifest_marker(dir)) {
        return Ok(hit.clone());
    }

    Err(DiscoveryError::RootNotFound { help: "No supported project marker was found." })
}

/// Ancestors from `start` (inclusive) up to the filesystem root.
fn ancestors_inclusive(start: &Path) -> Vec<PathBuf> {
    let mut chain = Vec::new();
    let mut current = start.to_path_buf();
    loop {
        chain.push(current.clone());
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
    chain
}

/// `.git` as directory or file (worktrees store a `gitdir:` pointer file).
/// Uses `symlink_metadata` so a `.git` file/dir is accepted regardless of
/// type; no git command is ever executed.
fn has_git_marker(dir: &Path) -> bool {
    std::fs::symlink_metadata(dir.join(".git")).is_ok()
}

/// `.context.toml` marker.
fn has_config_marker(dir: &Path) -> bool {
    dir.join(CONFIG_FILENAME).exists()
}

/// Known project manifests, including `*.sln` solutions.
fn has_manifest_marker(dir: &Path) -> bool {
    for marker in MANIFEST_MARKERS {
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

    fn canonical(p: &Path) -> PathBuf {
        std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf())
    }

    #[test]
    fn prefers_repo_root_over_nested_package_manifest() {
        // Essential regression: `repo/crates/core` has its own Cargo.toml but
        // discovery from inside must return the repo root carrying `.git`.
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path().join("repo");
        let inner = root.join("crates").join("core");
        std::fs::create_dir_all(&inner).unwrap();
        std::fs::create_dir_all(root.join(".git")).unwrap();
        std::fs::write(root.join("Cargo.toml"), "[workspace]\n").unwrap();
        std::fs::write(inner.join("Cargo.toml"), "[package]\nname=\"core\"\n").unwrap();

        let found = find_root(&inner).expect("must find repo root");
        assert_eq!(found, canonical(&root));
    }

    #[test]
    fn git_file_marker_covers_worktrees() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path().join("wt-root");
        let inner = root.join("crates").join("core");
        std::fs::create_dir_all(&inner).unwrap();
        // Worktrees keep `.git` as a file pointer, not a directory.
        std::fs::write(root.join(".git"), "gitdir: /elsewhere/worktrees/wt\n").unwrap();
        std::fs::write(root.join("Cargo.toml"), "[workspace]\n").unwrap();
        std::fs::write(inner.join("Cargo.toml"), "[package]\nname=\"core\"\n").unwrap();

        let found = find_root(&inner).expect("worktree .git file must win");
        assert_eq!(found, canonical(&root));
    }

    #[test]
    fn context_toml_beats_nearer_manifest_without_git() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path().join("proj");
        let inner = root.join("crates").join("core");
        std::fs::create_dir_all(&inner).unwrap();
        std::fs::write(root.join(CONFIG_FILENAME), "version = 1\n").unwrap();
        std::fs::write(inner.join("Cargo.toml"), "[package]\nname=\"core\"\n").unwrap();

        let found = find_root(&inner).expect("must find .context.toml ancestor");
        assert_eq!(found, canonical(&root));
    }

    #[test]
    fn manifest_fallback_returns_nearest_without_git_or_config() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path().join("proj");
        let inner = root.join("crates").join("core");
        std::fs::create_dir_all(&inner).unwrap();
        std::fs::write(root.join("package.json"), "{\"name\":\"root\"}").unwrap();
        std::fs::write(inner.join("Cargo.toml"), "[package]\nname=\"core\"\n").unwrap();

        let found = find_root(&inner).expect("must fall back to nearest manifest");
        assert_eq!(found, canonical(&inner));
    }

    #[test]
    fn nested_git_prefers_nearest() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let outer = tmp.path().join("outer");
        let nested = outer.join("nested");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::create_dir_all(outer.join(".git")).unwrap();
        std::fs::create_dir_all(nested.join(".git")).unwrap();

        let found = find_root(&nested).expect("must prefer nearest .git");
        assert_eq!(found, canonical(&nested));
    }

    #[test]
    fn sln_manifest_is_recognized() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path().join("dotnet-proj");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("app.sln"), "sln\n").unwrap();

        let found = find_root(&root).expect("*.sln must be a manifest marker");
        assert_eq!(found, canonical(&root));
    }
}
