//! Platform paths (Fase 10 foundation).
//!
//! The index lives in the *user cache dir*, outside the repository, so it
//! never pollutes Git and is never committed by accident.

use std::path::PathBuf;

/// Project cache directory for `repo_id` (e.g. `~/.cache/aicc/<repo_id>`).
/// Falls back to `.aicc-cache` under the system temp dir when no home exists.
#[must_use]
pub fn cache_dir_for_repo(repo_id: &str) -> PathBuf {
    let base = directories::ProjectDirs::from("", "", "aicc")
        .map(|d| d.cache_dir().to_path_buf())
        .unwrap_or_else(|| std::env::temp_dir().join("aicc-cache"));
    base.join(sanitize_repo_id(repo_id))
}

/// SQLite index file path for a repository.
#[must_use]
pub fn index_db_path(repo_id: &str) -> PathBuf {
    cache_dir_for_repo(repo_id).join("index.sqlite3")
}

fn sanitize_repo_id(id: &str) -> String {
    id.chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_path_is_sqlite() {
        let p = index_db_path("demo");
        assert_eq!(p.file_name().unwrap(), "index.sqlite3");
    }

    #[test]
    fn repo_id_is_sanitized() {
        assert_eq!(sanitize_repo_id("a/b:c"), "a_b_c");
    }
}
