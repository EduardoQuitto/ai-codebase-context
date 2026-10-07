//! `aicc-scanner`: secure file discovery (Fase 4).
//!
//! Traversal is built on the [`ignore`](https://docs.rs/ignore) crate:
//!
//! - `.gitignore` files at the scan root and in nested directories are
//!   honored with per-file attribution, so every exclusion keeps its reason.
//!   Deeper `.gitignore` files override shallower ones (git semantics).
//! - `.ignore` files, global gitignores and `.git/info/exclude` are NOT
//!   honored: results must not depend on the machine running the scan.
//! - Hidden files ARE traversed (dotfiles may hold config or docs); secrets
//!   are still excluded by the security policy before any content is read.
//! - Symlinks are never followed and their targets are never read.
//! - The walk is single-threaded and results are sorted: scans are
//!   deterministic for the same working tree.
//!
//! The scanner never executes repository code and never logs file contents:
//! diagnostics carry relative paths, sizes and reasons only.
//!
//! Exclusion precedence for files (first match wins):
//! `GitIgnore > ExtraIgnore > Generated > DocsExcluded > TestsExcluded >
//! Sensitive > TooLarge > Binary`.
//! Symlinks are rejected before everything (`Symlink`); I/O failures are
//! reported as warnings and the file is skipped (`Unreadable`) without
//! aborting the scan. Directory pruning (`DefaultIgnore`, `ExtraIgnore`)
//! happens during traversal, before file-level evaluation.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use ignore::gitignore::{Gitignore, GitignoreBuilder};
use ignore::{DirEntry, WalkBuilder};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use aicc_core::config::IndexConfig;
use aicc_core::security::{Classification, classify_path, is_generated_file_name};

/// Maximum bytes read from a file for the binary sniffing probe.
/// Files are never loaded fully just to decide inclusion.
const SNIFF_BYTES: u64 = 8 * 1024;

/// Default-ignored directory names (ROADMAP Fase 4; additional directories
/// can be pruned via `extra_ignores`).
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

/// Scanner failures: unreachable roots and invalid user patterns are fatal;
/// per-file problems become warnings and never abort the scan.
#[derive(Debug, Error)]
pub enum ScanError {
    /// Scan root does not exist or is not a directory.
    #[error("scan root is not accessible: {0}")]
    Inaccessible(String),

    /// Unexpected I/O failure (resolving the root, building matchers).
    #[error("I/O error while scanning: {0}")]
    Io(String),

    /// An `extra_ignores` pattern is not a valid gitignore glob.
    /// Fails fast with the offending pattern instead of silently ignoring it.
    #[error("invalid extra ignore pattern {pattern:?}: {reason}")]
    InvalidIgnorePattern { pattern: String, reason: String },
}

/// Why a path was excluded from the default context candidates.
/// Every variant is a fact about policy, never file contents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkipReason {
    /// Pruned product default directory (`node_modules`, `target`, …).
    DefaultIgnore,
    /// Matched a `.gitignore` rule (root or nested).
    GitIgnore,
    /// Matched an `extra_ignores` pattern from configuration.
    ExtraIgnore,
    /// Derived artifact (lockfiles excluded; see docs): minified bundles and
    /// explicit generated markers.
    Generated,
    /// Documentation file while `include_docs` is disabled.
    DocsExcluded,
    /// Test file while `include_tests` is disabled.
    TestsExcluded,
    /// Potential secret by name or extension. Contents are never read.
    Sensitive,
    /// Larger than `max_file_bytes`. Decided from metadata, never by reading.
    TooLarge,
    /// Binary by extension or NUL-byte probe.
    Binary,
    /// A symlink: never followed and never read (symlink policy).
    Symlink,
    /// Could not be examined (metadata/read failure). Always paired with a
    /// [`ScanWarning`]; the scan continues with the remaining files.
    Unreadable,
}

/// A skipped path paired with its exclusion reason.
pub type SkippedEntry = (String, SkipReason);

/// A non-fatal problem found during traversal (unreadable file, broken
/// `.gitignore`, walker error). Carries paths and static messages only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScanWarning {
    /// Relative path involved, when attributable to one.
    pub rel_path: Option<String>,
    /// Static description; never contains file contents.
    pub message: String,
}

/// The complete, deterministic outcome of a scan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScanReport {
    /// Files admitted as default context candidates, sorted by path.
    pub included: Vec<ScannedEntry>,
    /// Excluded paths with reasons, sorted by path.
    pub skipped: Vec<SkippedEntry>,
    /// Non-fatal problems; empty on a clean tree.
    pub warnings: Vec<ScanWarning>,
}

/// A candidate file admitted by the scanner.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScannedEntry {
    /// Path relative to the scan root, always with `/` separators.
    pub rel_path: String,
    /// File size in bytes (from metadata).
    pub size_bytes: u64,
    /// Name-based classification at scan time.
    pub classification: Classification,
}

/// Options for a scan (subset of `AppConfig::index`, Fase 3 contract).
#[derive(Debug, Clone)]
pub struct ScanOptions {
    /// Files larger than this are skipped (`TooLarge`).
    pub max_file_bytes: u64,
    /// Additional gitignore-style patterns (also prune directories).
    pub extra_ignores: Vec<String>,
    /// When false, documentation paths are skipped (`DocsExcluded`).
    pub include_docs: bool,
    /// When false, test paths are skipped (`TestsExcluded`).
    pub include_tests: bool,
}

impl Default for ScanOptions {
    fn default() -> Self {
        Self {
            max_file_bytes: 512 * 1024,
            extra_ignores: Vec::new(),
            include_docs: true,
            include_tests: true,
        }
    }
}

impl ScanOptions {
    /// Build scanner options from project configuration, so `extra_ignores`
    /// and `max_file_bytes` (plus the docs/tests gates) always reach the
    /// scanner instead of existing as inert config values.
    #[must_use]
    pub fn from_index_config(cfg: &IndexConfig) -> Self {
        Self {
            max_file_bytes: cfg.max_file_bytes,
            extra_ignores: cfg.extra_ignores.clone(),
            include_docs: cfg.include_docs,
            include_tests: cfg.include_tests,
        }
    }
}

/// Walk `root` and classify every file.
///
/// Returns included candidates plus, for diagnostics, the skip list and any
/// warnings. A single unreadable file never aborts the scan; only an
/// unreachable root or an invalid `extra_ignores` pattern is fatal.
pub fn scan(root: &Path, options: &ScanOptions) -> Result<ScanReport, ScanError> {
    if !root.is_dir() {
        return Err(ScanError::Inaccessible(root.display().to_string()));
    }
    // Canonicalize so walker paths, prefix stripping and matcher bases share
    // one absolute form (also resolves symlinked temp directories).
    let root_abs = std::fs::canonicalize(root)
        .map_err(|err| ScanError::Io(format!("could not resolve {}: {err}", root.display())))?;

    let extra = build_extra_matcher(&root_abs, &options.extra_ignores)?;

    // Directory pruning happens inside the walker: default and extra-ignored
    // directories are recorded once and never descended into.
    let pruned: Arc<Mutex<Vec<SkippedEntry>>> = Arc::new(Mutex::new(Vec::new()));
    let mut builder = WalkBuilder::new(&root_abs);
    builder
        .hidden(false)
        .git_ignore(false)
        .ignore(false)
        .git_global(false)
        .git_exclude(false)
        .follow_links(false)
        .filter_entry({
            let root = root_abs.clone();
            let extra = extra.clone();
            let pruned = Arc::clone(&pruned);
            move |entry: &DirEntry| {
                if entry.depth() == 0 {
                    return true;
                }
                if !entry.file_type().is_some_and(|t| t.is_dir()) {
                    return true;
                }
                let rel = rel_of(&root, entry.path());
                let name = entry.file_name().to_string_lossy().into_owned();
                if DEFAULT_IGNORED_DIRS.contains(&name.as_str()) {
                    record_skip(&pruned, rel, SkipReason::DefaultIgnore);
                    return false;
                }
                if extra.matched_path_or_any_parents(entry.path(), true).is_ignore() {
                    record_skip(&pruned, rel, SkipReason::ExtraIgnore);
                    return false;
                }
                true
            }
        });

    let mut layers = GitLayers::new(root_abs.clone(), extra);
    let mut included: Vec<ScannedEntry> = Vec::new();
    let mut skipped: Vec<SkippedEntry> = Vec::new();
    let mut warnings: Vec<ScanWarning> = Vec::new();

    for item in builder.build() {
        let entry = match item {
            Ok(entry) => entry,
            Err(err) => {
                warnings.push(ScanWarning {
                    rel_path: None,
                    message: format!("traversal error: {err}"),
                });
                continue;
            }
        };
        if entry.depth() == 0 {
            continue;
        }
        let rel = rel_of(&root_abs, entry.path());
        if let Some(err) = entry.error() {
            warnings.push(ScanWarning {
                rel_path: Some(rel.clone()),
                message: format!("ignore-file error: {err}"),
            });
        }
        if entry.path_is_symlink() {
            skipped.push((rel, SkipReason::Symlink));
            continue;
        }
        if entry.file_type().is_some_and(|t| t.is_dir()) {
            continue;
        }
        if layers.is_ignored(entry.path(), false) {
            skipped.push((rel, SkipReason::GitIgnore));
            continue;
        }
        if layers.extra_is_ignored(entry.path(), false) {
            skipped.push((rel, SkipReason::ExtraIgnore));
            continue;
        }
        if is_generated_file_name(entry.path()) {
            skipped.push((rel, SkipReason::Generated));
            continue;
        }
        if !options.include_docs && is_doc_path(&rel) {
            skipped.push((rel, SkipReason::DocsExcluded));
            continue;
        }
        if !options.include_tests && is_test_path(&rel) {
            skipped.push((rel, SkipReason::TestsExcluded));
            continue;
        }
        match classify_path(entry.path()) {
            Classification::Sensitive => {
                skipped.push((rel, SkipReason::Sensitive));
                continue;
            }
            Classification::Binary => {
                skipped.push((rel, SkipReason::Binary));
                continue;
            }
            Classification::Allowed | Classification::Generated => {}
        }
        let size = match entry.metadata() {
            Ok(metadata) => metadata.len(),
            Err(err) => {
                warnings.push(ScanWarning {
                    rel_path: Some(rel.clone()),
                    message: format!("could not read metadata: {err}"),
                });
                skipped.push((rel, SkipReason::Unreadable));
                continue;
            }
        };
        if size > options.max_file_bytes {
            skipped.push((rel, SkipReason::TooLarge));
            continue;
        }
        match contains_nul_byte(entry.path()) {
            Ok(true) => {
                skipped.push((rel, SkipReason::Binary));
                continue;
            }
            Ok(false) => {}
            Err(err) => {
                warnings.push(ScanWarning {
                    rel_path: Some(rel.clone()),
                    message: format!("could not read file for binary probe: {err}"),
                });
                skipped.push((rel, SkipReason::Unreadable));
                continue;
            }
        }
        included.push(ScannedEntry {
            rel_path: rel,
            size_bytes: size,
            classification: Classification::Allowed,
        });
    }

    skipped.extend(pruned_lock(&pruned));
    warnings.extend(layers.take_warnings());
    included.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
    skipped.sort_by(|a, b| a.0.cmp(&b.0));
    warnings.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
    Ok(ScanReport { included, skipped, warnings })
}

/// Compile `extra_ignores` with strict gitignore semantics: user typos must
/// fail loudly instead of silently matching literally.
fn build_extra_matcher(root: &Path, patterns: &[String]) -> Result<Gitignore, ScanError> {
    let mut builder = GitignoreBuilder::new(root);
    builder.allow_unclosed_class(false);
    for pattern in patterns {
        builder.add_line(None, pattern).map_err(|err| ScanError::InvalidIgnorePattern {
            pattern: pattern.clone(),
            reason: err.to_string(),
        })?;
    }
    builder.build().map_err(|err| ScanError::Io(format!("could not build ignore matcher: {err}")))
}

/// Stacked `.gitignore` evaluation: the root's file plus one layer per nested
/// directory. Deeper layers override shallower ones; `!` whitelists
/// re-include. Only files at or below the scan root are considered, so
/// results never depend on parent directories or machine-global git config.
struct GitLayers {
    root: PathBuf,
    extra: Gitignore,
    cache: HashMap<PathBuf, Option<Gitignore>>,
    warnings: Vec<ScanWarning>,
}

impl GitLayers {
    fn new(root: PathBuf, extra: Gitignore) -> Self {
        Self { root, extra, cache: HashMap::new(), warnings: Vec::new() }
    }

    /// Fold verdicts shallow-to-deep; the deepest non-trivial verdict wins.
    fn is_ignored(&mut self, path: &Path, is_dir: bool) -> bool {
        let mut verdict = false;
        for dir in ancestor_dirs(&self.root, path) {
            if let Some(matcher) = self.layer_for(&dir) {
                let matched = matcher.matched_path_or_any_parents(path, is_dir);
                if matched.is_ignore() {
                    verdict = true;
                } else if matched.is_whitelist() {
                    verdict = false;
                }
            }
        }
        verdict
    }

    fn extra_is_ignored(&self, path: &Path, is_dir: bool) -> bool {
        self.extra.matched_path_or_any_parents(path, is_dir).is_ignore()
    }

    fn take_warnings(&mut self) -> Vec<ScanWarning> {
        std::mem::take(&mut self.warnings)
    }

    fn layer_for(&mut self, dir: &Path) -> Option<&Gitignore> {
        if !self.cache.contains_key(dir) {
            let parsed = Self::parse_layer(&self.root, dir, &mut self.warnings);
            self.cache.insert(dir.to_path_buf(), parsed);
        }
        self.cache.get(dir).and_then(|layer| layer.as_ref())
    }

    fn parse_layer(root: &Path, dir: &Path, warnings: &mut Vec<ScanWarning>) -> Option<Gitignore> {
        let file = dir.join(".gitignore");
        if !file.is_file() {
            return None;
        }
        let mut builder = GitignoreBuilder::new(dir);
        if let Some(err) = builder.add(&file) {
            warnings.push(ScanWarning {
                rel_path: Some(rel_of(root, &file)),
                message: format!("could not read .gitignore: {err}"),
            });
            return None;
        }
        match builder.build() {
            Ok(matcher) if !matcher.is_empty() => Some(matcher),
            Ok(_) => None,
            Err(err) => {
                warnings.push(ScanWarning {
                    rel_path: Some(rel_of(root, &file)),
                    message: format!("could not parse .gitignore: {err}"),
                });
                None
            }
        }
    }
}

/// Directories from the scan root down to (and including) `path`'s parent.
fn ancestor_dirs(root: &Path, path: &Path) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    let mut current = path.parent();
    while let Some(dir) = current {
        if dir == root {
            dirs.push(root.to_path_buf());
            break;
        }
        if !dir.starts_with(root) {
            break;
        }
        dirs.push(dir.to_path_buf());
        current = dir.parent();
    }
    dirs.reverse();
    dirs
}

/// Logical relative path with `/` separators on every platform.
/// Falls back to the full path (normalized) if prefix stripping fails,
// which cannot happen for walker-produced entries.
fn rel_of(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .map(|rel| rel.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|_| path.to_string_lossy().replace('\\', "/"))
}

fn record_skip(target: &Mutex<Vec<SkippedEntry>>, rel: String, reason: SkipReason) {
    target.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).push((rel, reason));
}

fn pruned_lock(target: &Mutex<Vec<SkippedEntry>>) -> Vec<SkippedEntry> {
    target.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).clone()
}

/// Bounded NUL-byte probe: reads at most `SNIFF_BYTES` from the start.
/// Extensionless binaries are caught here; known text stays untouched.
fn contains_nul_byte(path: &Path) -> Result<bool, std::io::Error> {
    use std::io::Read as _;
    let file = std::fs::File::open(path)?;
    let mut buf = Vec::new();
    file.take(SNIFF_BYTES).read_to_end(&mut buf)?;
    Ok(buf.contains(&0))
}

/// Documentation paths by extension, well-known filename or directory.
/// Conservative and documented; case-insensitive.
fn is_doc_path(rel: &str) -> bool {
    const DOC_EXTENSIONS: &[&str] = &["md", "markdown", "rst", "adoc", "txt"];
    const DOC_BASENAMES: &[&str] =
        &["readme", "changelog", "changes", "license", "licence", "notice", "authors"];
    const DOC_DIRS: &[&str] = &["docs", "doc", "documentation", "wiki"];

    let lower = rel.to_ascii_lowercase();
    let filename = lower.rsplit('/').next().unwrap_or(&lower);
    if DOC_EXTENSIONS.iter().any(|ext| filename.ends_with(&format!(".{ext}"))) {
        return true;
    }
    if DOC_BASENAMES.iter().any(|base| {
        filename == *base
            || filename.starts_with(&format!("{base}."))
            || filename.starts_with(&format!("{base}_"))
    }) {
        return true;
    }
    lower.split('/').next().is_some_and(|first| DOC_DIRS.contains(&first))
}

/// Test paths by directory, filename convention or well-known helper name.
/// Conservative and documented; case-insensitive.
fn is_test_path(rel: &str) -> bool {
    const TEST_DIRS: &[&str] = &["test", "tests", "__tests__", "spec", "specs"];

    let lower = rel.to_ascii_lowercase();
    if lower.split('/').any(|component| TEST_DIRS.contains(&component)) {
        return true;
    }
    let filename = lower.rsplit('/').next().unwrap_or(&lower);
    if filename == "conftest.py" {
        return true;
    }
    let stem = filename.rsplit_once('.').map(|(stem, _)| stem).unwrap_or(filename);
    stem.starts_with("test_")
        || stem.ends_with("_test")
        || stem.ends_with("_spec")
        || filename.contains(".test.")
        || filename.contains(".spec.")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// Shared project fixture (committed; some files are force-added because
    /// the fixture's own `.gitignore` hides them from git — see tests/README).
    fn fixture(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures").join(name)
    }

    fn scan_fixture(name: &str, options: &ScanOptions) -> ScanReport {
        scan(&fixture(name), options).expect("fixture scan must succeed")
    }

    fn included_set(report: &ScanReport) -> Vec<&str> {
        report.included.iter().map(|entry| entry.rel_path.as_str()).collect()
    }

    fn skipped_map(report: &ScanReport) -> HashMap<&str, SkipReason> {
        report.skipped.iter().map(|(rel, reason)| (rel.as_str(), *reason)).collect()
    }

    #[test]
    fn normal_files_are_included() {
        let report = scan_fixture("basic-repo", &ScanOptions::default());
        let included = included_set(&report);
        for expected in [
            ".context.toml",
            ".gitignore",
            "package.json",
            "extra-me.txt",
            "keep.log",
            "docs/notes.md",
            "src/hello.ts",
            "tests/test_example.py",
            "sub/visible.txt",
            "sub/.gitignore",
            "assets/data.txt",
        ] {
            assert!(included.contains(&expected), "missing candidate: {expected}");
        }
    }

    #[test]
    fn default_ignored_dirs_are_pruned() {
        let report = scan_fixture("basic-repo", &ScanOptions::default());
        let skipped = skipped_map(&report);
        assert_eq!(skipped.get("node_modules"), Some(&SkipReason::DefaultIgnore));
        // Pruned contents never surface, neither as candidates nor as skips.
        assert!(!included_set(&report).iter().any(|path| path.starts_with("node_modules/")));
        assert!(!skipped.contains_key("node_modules/dep/index.js"));
    }

    #[test]
    fn gitignore_is_respected_with_reasons() {
        let report = scan_fixture("basic-repo", &ScanOptions::default());
        let skipped = skipped_map(&report);
        assert_eq!(skipped.get("ignored-by-git.txt"), Some(&SkipReason::GitIgnore));
        assert_eq!(skipped.get("debug.log"), Some(&SkipReason::GitIgnore));
        assert_eq!(skipped.get("build-git/out.js"), Some(&SkipReason::GitIgnore));
        assert_eq!(skipped.get("sub/inner-ignored.txt"), Some(&SkipReason::GitIgnore));
        // `!keep.log` whitelists back a file otherwise ignored by `*.log`.
        assert!(included_set(&report).contains(&"keep.log"));
    }

    #[test]
    fn extra_ignores_are_respected() {
        let options = ScanOptions {
            extra_ignores: vec![String::from("extra-*.txt")],
            ..ScanOptions::default()
        };
        let report = scan_fixture("basic-repo", &options);
        assert_eq!(skipped_map(&report).get("extra-me.txt"), Some(&SkipReason::ExtraIgnore));
        // Without the option the same file is a normal candidate.
        let plain = scan_fixture("basic-repo", &ScanOptions::default());
        assert!(included_set(&plain).contains(&"extra-me.txt"));
    }

    #[test]
    fn binaries_are_excluded() {
        let report = scan_fixture("basic-repo", &ScanOptions::default());
        let skipped = skipped_map(&report);
        assert_eq!(skipped.get("assets/logo.png"), Some(&SkipReason::Binary));
        // No known binary extension: caught by the NUL-byte probe.
        assert_eq!(skipped.get("assets/blob.bin"), Some(&SkipReason::Binary));
        assert!(included_set(&report).contains(&"assets/data.txt"));
    }

    #[test]
    fn sensitive_files_are_excluded_without_being_read() {
        let report = scan_fixture("basic-repo", &ScanOptions::default());
        assert_eq!(skipped_map(&report).get(".env"), Some(&SkipReason::Sensitive));

        let secrets = scan_fixture("secrets-repo", &ScanOptions::default());
        let skipped = skipped_map(&secrets);
        assert_eq!(skipped.get(".env"), Some(&SkipReason::Sensitive));
        assert_eq!(skipped.get("deploy.pem"), Some(&SkipReason::Sensitive));
        assert!(included_set(&secrets).contains(&"src/main.rs"));
    }

    #[test]
    fn oversized_files_are_excluded() {
        let plain = scan_fixture("basic-repo", &ScanOptions::default());
        let size = plain
            .included
            .iter()
            .find(|entry| entry.rel_path == "src/hello.ts")
            .map(|entry| entry.size_bytes)
            .expect("hello.ts must be included by default");
        let limit = size.checked_sub(1).expect("fixture must be non-empty");
        let report = scan_fixture(
            "basic-repo",
            &ScanOptions { max_file_bytes: limit, ..ScanOptions::default() },
        );
        assert_eq!(skipped_map(&report).get("src/hello.ts"), Some(&SkipReason::TooLarge));
    }

    #[test]
    fn generated_files_are_excluded() {
        let report = scan_fixture("basic-repo", &ScanOptions::default());
        assert_eq!(skipped_map(&report).get("package-lock.json"), Some(&SkipReason::Generated));
    }

    #[test]
    fn docs_and_tests_gates_have_effect() {
        let plain = scan_fixture("basic-repo", &ScanOptions::default());
        assert!(included_set(&plain).contains(&"docs/notes.md"));
        assert!(included_set(&plain).contains(&"tests/test_example.py"));

        let no_docs = scan_fixture(
            "basic-repo",
            &ScanOptions { include_docs: false, ..ScanOptions::default() },
        );
        assert_eq!(skipped_map(&no_docs).get("docs/notes.md"), Some(&SkipReason::DocsExcluded));

        let no_tests = scan_fixture(
            "basic-repo",
            &ScanOptions { include_tests: false, ..ScanOptions::default() },
        );
        assert_eq!(
            skipped_map(&no_tests).get("tests/test_example.py"),
            Some(&SkipReason::TestsExcluded)
        );
    }

    #[test]
    fn exclusion_precedence_is_documented_order() {
        // Git wins over an identical extra pattern.
        let report = scan_fixture(
            "basic-repo",
            &ScanOptions { extra_ignores: vec![String::from("*.log")], ..ScanOptions::default() },
        );
        assert_eq!(skipped_map(&report).get("debug.log"), Some(&SkipReason::GitIgnore));

        // An explicit extra pattern wins over the sensitive classification.
        let report = scan_fixture(
            "basic-repo",
            &ScanOptions { extra_ignores: vec![String::from(".env")], ..ScanOptions::default() },
        );
        assert_eq!(skipped_map(&report).get(".env"), Some(&SkipReason::ExtraIgnore));
    }

    #[test]
    fn rel_paths_are_normalized() {
        for name in ["basic-repo", "secrets-repo"] {
            let report = scan_fixture(name, &ScanOptions::default());
            for entry in &report.included {
                assert!(!entry.rel_path.contains('\\'), "backslash: {}", entry.rel_path);
                assert!(!entry.rel_path.starts_with("./"), "dot prefix: {}", entry.rel_path);
                assert!(!entry.rel_path.starts_with('/'), "absolute: {}", entry.rel_path);
            }
            for (rel, _) in &report.skipped {
                assert!(!rel.contains('\\'), "backslash: {rel}");
            }
        }
    }

    #[test]
    fn scan_is_deterministic() {
        let first = scan_fixture("basic-repo", &ScanOptions::default());
        let second = scan_fixture("basic-repo", &ScanOptions::default());
        assert_eq!(first, second);
    }

    #[test]
    fn full_default_scan_matches_expected_candidates() {
        let report = scan_fixture("basic-repo", &ScanOptions::default());
        let mut included = included_set(&report);
        included.sort_unstable();
        assert_eq!(
            included,
            vec![
                ".context.toml",
                ".gitignore",
                "assets/data.txt",
                "docs/notes.md",
                "extra-me.txt",
                "keep.log",
                "package.json",
                "src/hello.ts",
                "sub/.gitignore",
                "sub/visible.txt",
                "tests/test_example.py",
            ]
        );
        assert!(report.warnings.is_empty(), "clean fixture must not warn: {:?}", report.warnings);
    }

    #[test]
    fn missing_root_is_fatal() {
        let err = scan(&PathBuf::from("definitely-missing-dir-xyz"), &ScanOptions::default())
            .expect_err("missing root must fail");
        assert!(matches!(err, ScanError::Inaccessible(_)));
    }

    #[test]
    fn invalid_extra_pattern_fails_fast() {
        let err = scan(
            &fixture("basic-repo"),
            &ScanOptions {
                extra_ignores: vec![String::from("[unclosed")],
                ..ScanOptions::default()
            },
        )
        .expect_err("invalid glob must fail");
        assert!(matches!(err, ScanError::InvalidIgnorePattern { .. }));
    }

    #[test]
    fn options_come_from_index_config() {
        let mut config = aicc_core::config::AppConfig::default();
        config.index.max_file_bytes = 1234;
        config.index.extra_ignores = vec![String::from("gen/**")];
        config.index.include_docs = false;
        config.index.include_tests = false;
        let options = ScanOptions::from_index_config(&config.index);
        assert_eq!(options.max_file_bytes, 1234);
        assert_eq!(options.extra_ignores, vec![String::from("gen/**")]);
        assert!(!options.include_docs);
        assert!(!options.include_tests);
    }

    #[test]
    #[cfg(unix)]
    fn unreadable_file_warns_without_killing_scan() {
        use std::os::unix::fs::PermissionsExt as _;

        let dir = std::env::temp_dir().join(format!("aicc-scan-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("ok.txt"), "fine").unwrap();
        let locked = dir.join("locked.txt");
        std::fs::write(&locked, "cannot read this").unwrap();
        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).unwrap();

        let report =
            scan(&dir, &ScanOptions::default()).expect("scan must survive unreadable file");
        let skipped = skipped_map(&report);
        assert_eq!(skipped.get("locked.txt"), Some(&SkipReason::Unreadable));
        assert!(included_set(&report).contains(&"ok.txt"));
        assert!(!report.warnings.is_empty());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    #[cfg(unix)]
    fn symlinks_are_never_followed() {
        let dir = std::env::temp_dir().join(format!("aicc-link-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("real")).unwrap();
        std::fs::write(dir.join("real/inner.txt"), "content").unwrap();
        std::os::unix::fs::symlink(dir.join("real/inner.txt"), dir.join("link.txt")).unwrap();
        std::os::unix::fs::symlink(dir.join("real"), dir.join("linked-dir")).unwrap();

        let report = scan(&dir, &ScanOptions::default()).expect("scan must succeed");
        let skipped = skipped_map(&report);
        assert_eq!(skipped.get("link.txt"), Some(&SkipReason::Symlink));
        assert_eq!(skipped.get("linked-dir"), Some(&SkipReason::Symlink));
        // The real content appears exactly once, via its true path.
        assert_eq!(
            included_set(&report).iter().filter(|path| path.ends_with("inner.txt")).count(),
            1
        );

        let _ = std::fs::remove_dir_all(&dir);
    }
}
