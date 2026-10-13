//! `aicc-scanner`: secure file discovery (Fase 5-6).
//!
//! Traversal is built on the [`ignore`](https://docs.rs/ignore) crate:
//!
//! - `.gitignore` files at the scan root and in nested directories are
//!   honored with per-file attribution, so every exclusion keeps its reason.
//!   Deeper `.gitignore` files override shallower ones (git semantics).
//! - `.ignore` files, global gitignores and `.git/info/exclude` are NOT
//!   honored: results must not depend on the machine running the scan.
//! - Hidden files ARE traversed (dotfiles may hold config or docs).
//! - Symlinks are never followed and their targets are never read.
//! - The walk is single-threaded and results are sorted: scans are
//!   deterministic for the same working tree.
//!
//! Security (Fase 5) is defense-in-depth, local-only and bounded:
//! - path-based secrets (`.env`, `*.pem`…) are excluded *before* any content
//!   is read whenever `exclude_secrets` is enabled;
//! - remaining candidates are read once, bounded by `max_file_bytes`, and the
//!   same buffer backs the binary (NUL) check, the high-confidence secret
//!   detector and the Fase 6 metadata (hash + line count); oversized files
//!   are never read fully;
//! - `security.extra_excludes` is a hard path policy evaluated before
//!   `.gitignore` reinclusions and without reading content;
//! - findings are static reasons only: values, spans and snippets never enter
//!   reports, warnings, logs or serialized output.
//!
//! Metadata (Fase 6): admitted files carry language (extension registry),
//! normalized extension, deterministic BLAKE3 content hash and line count,
//! all derived from the same validated buffer — no second read, no raw
//! content stored. Excluded files carry only their relative path and static
//! reason: no hash, line count or measured size is invented for files that
//! were never read. Pruned directories never yield simulated descendants.
//!
//! The scanner never executes repository code and never logs file contents:
//! diagnostics carry relative paths, sizes and reasons only.
//!
//! Exclusion precedence for files (first match wins):
//! `Symlink > SecurityExclude > GitIgnore > ExtraIgnore > Generated >
//! DocsExcluded > TestsExcluded > Sensitive > TooLarge > Binary > SecretContent`.
//! `Sensitive` (path) and `SecretContent` (bounded content inspection) apply
//! only when `exclude_secrets` is enabled. I/O failures are reported as
//! warnings and the file is skipped (`Unreadable`) without aborting the scan.
//! Only default-ignored directories (`DefaultIgnore`) are pruned during
//! traversal; `extra_ignores` never prunes so nested `.gitignore`
//! reinclusions keep their documented precedence.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use aicc_parser::{DetectionSource, detect_language};
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use ignore::{DirEntry, WalkBuilder};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use aicc_core::config::{AppConfig, IndexConfig};
use aicc_core::security::{Classification, classify_path, contains_secret, is_generated_file_name};

/// Content-hash algorithm identifier (versioned so future indexing never
/// depends on an implicit choice). Stored hashes look like
/// `blake3-v1:<64 lowercase hex>`.
pub const HASH_ALGORITHM_ID: &str = "blake3-v1";

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

    /// A `security.extra_excludes` pattern is invalid or uses a denied
    /// negation (`!`) that would weaken the hard exclusion policy.
    #[error("invalid security exclude pattern {pattern:?}: {reason}")]
    InvalidExcludePattern { pattern: String, reason: String },
}

/// Why a path was excluded from the default context candidates.
/// Every variant is a fact about policy, never file contents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkipReason {
    /// Pruned product default directory (`node_modules`, `target`, …).
    DefaultIgnore,
    /// Hard user policy from `security.extra_excludes`. Beats `.gitignore`
    /// reinclusions; decided without reading content.
    SecurityExclude,
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
    /// Potential secret by name or extension. Excluded before content is read
    /// whenever `exclude_secrets` is enabled.
    Sensitive,
    /// Larger than `max_file_bytes`. Decided from metadata (plus a bounded
    /// growth check at read time), never by reading fully.
    TooLarge,
    /// Binary by extension or NUL-byte probe over the bounded buffer.
    Binary,
    /// High-confidence secret pattern in bounded candidate content.
    /// Static reason only: no pattern name, value or snippet is stored.
    SecretContent,
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

/// A candidate file admitted by the scanner, enriched with Fase 6 metadata.
///
/// All content-derived fields (`content_hash`, `line_count`, measured
/// `size_bytes`) describe the exact validated buffer that passed the Fase 5
/// binary and secret checks — never a second read, never raw content.
/// `language` is the stable [`aicc_parser::Language::id`]; `Unknown` files
/// stay eligible for metadata and future textual indexing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScannedEntry {
    /// Path relative to the scan root, always with `/` separators.
    pub rel_path: String,
    /// Measured content size in bytes (length of the validated buffer).
    pub size_bytes: u64,
    /// Name-based classification at scan time.
    pub classification: Classification,
    /// Normalized lowercase extension (`None` when absent, e.g. `Makefile`).
    pub extension: Option<String>,
    /// Detected language id (e.g. `rust`, `typescript`, `unknown`).
    pub language: String,
    /// Where the language assignment came from (Fase 6: always `extension`).
    pub detection_source: DetectionSource,
    /// Deterministic content hash (`blake3-v1:<hex>`) of the validated bytes.
    pub content_hash: String,
    /// Deterministic line count over the validated bytes (see
    /// [`count_lines`]).
    pub line_count: u64,
    /// Real structural parser availability. Always `false` in Fase 6
    /// (Tree-sitter belongs to Fase 7); distinct from the v1 support plan.
    pub parser_available: bool,
}

/// Options for a scan (subset of `AppConfig::index` + `AppConfig::security`).
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
    /// When true (default), path-based secrets are excluded early and bounded
    /// candidate content is inspected for high-confidence secret patterns.
    /// When false, automatic secret exclusion is disabled (documented opt-out);
    /// binary/generated/size/`extra_excludes` policy still applies.
    pub exclude_secrets: bool,
    /// Hard path policy from `security.extra_excludes`. Evaluated before
    /// `.gitignore` reinclusions and without reading content. `!` negations
    /// are rejected at build time.
    pub extra_excludes: Vec<String>,
}

impl Default for ScanOptions {
    fn default() -> Self {
        Self {
            max_file_bytes: 512 * 1024,
            extra_ignores: Vec::new(),
            include_docs: true,
            include_tests: true,
            exclude_secrets: true,
            extra_excludes: Vec::new(),
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
            exclude_secrets: true,
            extra_excludes: Vec::new(),
        }
    }

    /// Build scanner options from the full application configuration,
    /// wiring both `index` and `security` sections. Prefer this over
    /// [`ScanOptions::from_index_config`] for real scans.
    #[must_use]
    pub fn from_app_config(cfg: &AppConfig) -> Self {
        Self {
            max_file_bytes: cfg.index.max_file_bytes,
            extra_ignores: cfg.index.extra_ignores.clone(),
            include_docs: cfg.index.include_docs,
            include_tests: cfg.index.include_tests,
            exclude_secrets: cfg.security.exclude_secrets,
            extra_excludes: cfg.security.extra_excludes.clone(),
        }
    }
}

/// Walk `root` and classify every file.
///
/// Returns included candidates plus, for diagnostics, the skip list and any
/// warnings. A single unreadable file never aborts the scan; only an
/// unreachable root or an invalid `extra_ignores`/`extra_excludes` pattern is
/// fatal. Reasons carry static policy facts only, never secret values.
pub fn scan(root: &Path, options: &ScanOptions) -> Result<ScanReport, ScanError> {
    if !root.is_dir() {
        return Err(ScanError::Inaccessible(root.display().to_string()));
    }
    // Canonicalize so walker paths, prefix stripping and matcher bases share
    // one absolute form (also resolves symlinked temp directories).
    let root_abs = std::fs::canonicalize(root)
        .map_err(|err| ScanError::Io(format!("could not resolve {}: {err}", root.display())))?;

    let extra = build_extra_matcher(&root_abs, &options.extra_ignores)?;
    let excludes = build_exclude_matcher(&root_abs, &options.extra_excludes)?;

    // Directory pruning happens inside the walker: only default-ignored
    // directories are recorded once and never descended into. `extra_ignores`
    // must NOT prune: a pruned directory would hide nested `.gitignore`
    // reinclusions that outrank `ExtraIgnore` by documented precedence.
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
        // Hard user policy first: `security.extra_excludes` beats `.gitignore`
        // reinclusions and is decided without reading content.
        if excludes.matched_path_or_any_parents(entry.path(), false).is_ignore() {
            skipped.push((rel, SkipReason::SecurityExclude));
            continue;
        }
        // Documented precedence: GitIgnore > ExtraIgnore. A nested `.gitignore`
        // whitelist (`!keep.ts`) explicitly re-includes and therefore skips the
        // `ExtraIgnore` check; absence of any git match falls through to it.
        match layers.git_verdict(entry.path(), false) {
            Some(GitVerdict::Ignore) => {
                skipped.push((rel, SkipReason::GitIgnore));
                continue;
            }
            Some(GitVerdict::Whitelist) => {}
            None => {
                if layers.extra_is_ignored(entry.path(), false) {
                    skipped.push((rel, SkipReason::ExtraIgnore));
                    continue;
                }
            }
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
        // Early path gate (no content read) when secret exclusion is enabled.
        if options.exclude_secrets {
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
        } else {
            // Opt-out: only the binary *extension* gate still applies early;
            // path-based `Sensitive` and content inspection are disabled.
            if matches!(classify_path(entry.path()), Classification::Binary) {
                skipped.push((rel, SkipReason::Binary));
                continue;
            }
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
        // Single bounded read backs both the binary probe and, for candidates,
        // the secret detector. `limit+1` exposes growth between metadata and
        // read so a grown file stays `TooLarge` instead of being read fully.
        let bounded = match read_bounded(entry.path(), options.max_file_bytes) {
            Ok(buf) => buf,
            Err(err) => {
                warnings.push(ScanWarning {
                    rel_path: Some(rel.clone()),
                    message: format!("could not read file for content inspection: {err}"),
                });
                skipped.push((rel, SkipReason::Unreadable));
                continue;
            }
        };
        if (bounded.len() as u64) > options.max_file_bytes {
            skipped.push((rel, SkipReason::TooLarge));
            continue;
        }
        if bounded.contains(&0) {
            skipped.push((rel, SkipReason::Binary));
            continue;
        }
        if options.exclude_secrets {
            // Lossy text view for detection only; the buffer itself is dropped
            // after this scope and never stored, logged or serialized.
            let text = String::from_utf8_lossy(&bounded);
            if contains_secret(&text) {
                skipped.push((rel, SkipReason::SecretContent));
                continue;
            }
        }
        // Fase 6 metadata from the SAME validated buffer: no second read, no
        // raw bytes stored. Secret-excluded files never reach this point, so
        // no hash/line count of sensitive content enters the admitted set.
        let language = detect_language(entry.path());
        included.push(ScannedEntry {
            rel_path: rel,
            size_bytes: bounded.len() as u64,
            classification: Classification::Allowed,
            extension: normalized_extension(entry.path()),
            language: language.id().to_string(),
            detection_source: DetectionSource::Extension,
            content_hash: content_hash_for(&bounded),
            line_count: count_lines(&bounded),
            parser_available: language.has_structural_parser(),
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

/// Compile `security.extra_excludes` as a hard path policy. `!` negations are
/// rejected: a whitelist must never reopen a hard exclusion.
fn build_exclude_matcher(root: &Path, patterns: &[String]) -> Result<Gitignore, ScanError> {
    let mut builder = GitignoreBuilder::new(root);
    builder.allow_unclosed_class(false);
    for pattern in patterns {
        let trimmed = pattern.trim_start();
        if trimmed.starts_with('!') {
            return Err(ScanError::InvalidExcludePattern {
                pattern: pattern.clone(),
                reason: String::from("negation patterns are not allowed for security excludes"),
            });
        }
        builder.add_line(None, pattern).map_err(|err| ScanError::InvalidExcludePattern {
            pattern: pattern.clone(),
            reason: err.to_string(),
        })?;
    }
    builder.build().map_err(|err| ScanError::Io(format!("could not build exclude matcher: {err}")))
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

/// Explicit `.gitignore` outcome for precedence handling.
/// `Ignore` is an explicit exclusion, `Whitelist` an explicit `!` re-inclusion;
/// absence of both means no `.gitignore` rule matched.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GitVerdict {
    Ignore,
    Whitelist,
}

impl GitLayers {
    fn new(root: PathBuf, extra: Gitignore) -> Self {
        Self { root, extra, cache: HashMap::new(), warnings: Vec::new() }
    }

    /// Fold verdicts shallow-to-deep; the deepest non-trivial verdict wins.
    /// Returns `None` when no `.gitignore` layer matches, so callers can
    /// distinguish exclusion, explicit re-inclusion and absence.
    fn git_verdict(&mut self, path: &Path, is_dir: bool) -> Option<GitVerdict> {
        let mut verdict: Option<GitVerdict> = None;
        for dir in ancestor_dirs(&self.root, path) {
            if let Some(matcher) = self.layer_for(&dir) {
                let matched = matcher.matched_path_or_any_parents(path, is_dir);
                if matched.is_ignore() {
                    verdict = Some(GitVerdict::Ignore);
                } else if matched.is_whitelist() {
                    verdict = Some(GitVerdict::Whitelist);
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

/// Bounded content read for binary + secret inspection: at most
/// `limit + 1` bytes. The extra byte exposes growth between the metadata
/// check and the read so oversized files stay `TooLarge`. Never reads
/// unbounded content; oversized files are never read fully for detection.
fn read_bounded(path: &Path, limit: u64) -> Result<Vec<u8>, std::io::Error> {
    use std::io::Read as _;
    let file = std::fs::File::open(path)?;
    let mut buf = Vec::new();
    // `limit` fits the documented 1_000_000 budget cap; saturating keeps the
    // `u64 -> u64` arithmetic total without silent truncation on huge configs.
    let capped = limit.saturating_add(1);
    file.take(capped).read_to_end(&mut buf)?;
    Ok(buf)
}

/// Normalized lowercase extension (`None` when the file has none, e.g.
/// `Makefile` or dotfiles like `.gitignore`). Consistent with
/// [`detect_language`]: same `Path::extension` view, lowercased once.
#[must_use]
pub fn normalized_extension(path: &Path) -> Option<String> {
    path.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase())
}

/// Deterministic line count over raw bytes (no UTF-8 assumption, no panic):
/// - empty content: 0;
/// - otherwise newlines (`\n`) plus one for a final unterminated line;
/// - trailing `\n` adds no extra empty line;
/// - CRLF is consistent (each `\r\n` holds one `\n`); lone `\r` is not a break.
#[must_use]
pub fn count_lines(bytes: &[u8]) -> u64 {
    if bytes.is_empty() {
        return 0;
    }
    let newlines = bytes.iter().filter(|b| **b == b'\n').count() as u64;
    if bytes.ends_with(b"\n") { newlines } else { newlines + 1 }
}

/// Deterministic content hash of the validated bytes: `blake3-v1:<hex>`.
/// BLAKE3 is a consolidated, dependency-free hash for code content (no
/// external service, no hand-rolled crypto); the `v1` tag keeps future
/// indexing independent of this implicit choice.
#[must_use]
pub fn content_hash_for(bytes: &[u8]) -> String {
    format!("{HASH_ALGORITHM_ID}:{}", blake3::hash(bytes).to_hex())
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

    #[test]
    fn nested_gitignore_whitelist_beats_extra_ignore_dir() {
        // Regression: `extra_ignores` hits a directory, but a nested
        // `.gitignore` explicitly re-includes `keep.ts`. Documented precedence
        // (`GitIgnore > ExtraIgnore`) requires `keep.ts` to stay included
        // while siblings remain `ExtraIgnore`. No directory pruning may hide
        // the nested rules.
        let dir = std::env::temp_dir().join(format!("aicc-extra-git-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("over")).unwrap();
        std::fs::write(dir.join("over/.gitignore"), "!keep.ts\n").unwrap();
        std::fs::write(dir.join("over/keep.ts"), "console.log(1);\n").unwrap();
        std::fs::write(dir.join("over/drop.ts"), "console.log(2);\n").unwrap();

        let options =
            ScanOptions { extra_ignores: vec![String::from("over/")], ..ScanOptions::default() };
        let report = scan(&dir, &options).expect("scan must succeed");
        let included = included_set(&report);
        let skipped = skipped_map(&report);
        assert!(
            included.contains(&"over/keep.ts"),
            "nested .gitignore whitelist must beat extra_ignores: {report:?}"
        );
        assert_eq!(
            skipped.get("over/drop.ts"),
            Some(&SkipReason::ExtraIgnore),
            "siblings without reinclusion stay excluded: {report:?}"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    // ---- Fase 5: positives (synthetic, clearly fake) ----

    #[test]
    fn content_private_key_in_common_file_is_excluded() {
        let report = scan_fixture("secrets-repo", &ScanOptions::default());
        assert_eq!(
            skipped_map(&report).get("notes.txt"),
            Some(&SkipReason::SecretContent),
            "PEM marker in notes.txt must exclude: {report:?}"
        );
    }

    #[test]
    fn content_provider_token_is_excluded() {
        let report = scan_fixture("secrets-repo", &ScanOptions::default());
        assert_eq!(
            skipped_map(&report).get("src/token.ts"),
            Some(&SkipReason::SecretContent),
            "ghp_ token must exclude: {report:?}"
        );
    }

    #[test]
    fn content_database_url_is_excluded() {
        let report = scan_fixture("secrets-repo", &ScanOptions::default());
        assert_eq!(
            skipped_map(&report).get("src/db.ts"),
            Some(&SkipReason::SecretContent),
            "postgres URL with creds must exclude: {report:?}"
        );
    }

    #[test]
    fn content_credential_assignment_is_excluded() {
        let report = scan_fixture("secrets-repo", &ScanOptions::default());
        assert_eq!(
            skipped_map(&report).get("src/config.js"),
            Some(&SkipReason::SecretContent),
            "api_key assignment must exclude: {report:?}"
        );
    }

    #[test]
    fn content_secret_after_8k_is_excluded() {
        // Proves inspection is not limited to the initial binary probe.
        let report = scan_fixture("secrets-repo", &ScanOptions::default());
        assert_eq!(
            skipped_map(&report).get("large_note.txt"),
            Some(&SkipReason::SecretContent),
            "late secret must exclude: {report:?}"
        );
    }

    #[test]
    fn path_env_is_excluded_early() {
        let report = scan_fixture("secrets-repo", &ScanOptions::default());
        assert_eq!(skipped_map(&report).get(".env"), Some(&SkipReason::Sensitive));
        assert_eq!(skipped_map(&report).get("deploy.pem"), Some(&SkipReason::Sensitive));
    }

    // ---- Fase 5: negatives (no false positives) ----

    #[test]
    fn plain_code_is_included() {
        let report = scan_fixture("secrets-repo", &ScanOptions::default());
        assert!(included_set(&report).contains(&"src/main.rs"));
    }

    #[test]
    fn suspicious_name_without_secret_value_is_included() {
        let report = scan_fixture("secrets-repo", &ScanOptions::default());
        assert!(
            included_set(&report).contains(&"src/empty.js"),
            "empty/short values must not flag: {report:?}"
        );
    }

    #[test]
    fn placeholders_are_included() {
        let report = scan_fixture("secrets-repo", &ScanOptions::default());
        assert!(
            included_set(&report).contains(&"src/placeholder.js"),
            "placeholders must not flag: {report:?}"
        );
        assert!(
            included_set(&report).contains(&"docs/example.md"),
            "illustrative docs must not flag: {report:?}"
        );
    }

    #[test]
    fn public_id_alone_is_included() {
        let report = scan_fixture("secrets-repo", &ScanOptions::default());
        assert!(
            included_set(&report).contains(&"src/public_id.txt"),
            "AKIA public id alone must not flag: {report:?}"
        );
    }

    #[test]
    fn binary_and_generated_keep_their_reasons() {
        let report = scan_fixture("secrets-repo", &ScanOptions::default());
        let skipped = skipped_map(&report);
        assert_eq!(skipped.get("assets/logo.png"), Some(&SkipReason::Binary));
        assert_eq!(skipped.get("package-lock.json"), Some(&SkipReason::Generated));
    }

    // ---- Fase 5: policy ----

    #[test]
    fn exclude_secrets_true_and_false_semantics() {
        let dir = std::env::temp_dir().join(format!("aicc-excl-bool-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(".env"), "FAKE=1\n").unwrap();
        std::fs::write(
            dir.join("app.ts"),
            "const api_key = \"aB3dE5fG7hJ9kL2mN4pQ6rS8tU0vW2xY4\";\n",
        )
        .unwrap();
        std::fs::write(dir.join("plain.ts"), "export const x = 1;\n").unwrap();

        let on = ScanOptions { exclude_secrets: true, ..ScanOptions::default() };
        let report = scan(&dir, &on).expect("scan must succeed");
        let skipped = skipped_map(&report);
        assert_eq!(skipped.get(".env"), Some(&SkipReason::Sensitive));
        assert_eq!(skipped.get("app.ts"), Some(&SkipReason::SecretContent));
        assert!(included_set(&report).contains(&"plain.ts"));

        let off = ScanOptions { exclude_secrets: false, ..ScanOptions::default() };
        let report = scan(&dir, &off).expect("scan must succeed");
        let included = included_set(&report);
        // Opt-out admits credential content (documented reduction).
        assert!(included.contains(&".env"), "opt-out must admit .env: {report:?}");
        assert!(included.contains(&"app.ts"), "opt-out must admit secret content: {report:?}");

        // Binary/generated/size policy still applies with opt-out.
        std::fs::write(dir.join("bin.png"), "fake").unwrap();
        std::fs::write(dir.join("package-lock.json"), "{}").unwrap();
        let report = scan(&dir, &off).expect("scan must succeed");
        assert_eq!(skipped_map(&report).get("bin.png"), Some(&SkipReason::Binary));
        assert_eq!(skipped_map(&report).get("package-lock.json"), Some(&SkipReason::Generated));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn extra_excludes_is_hard_policy() {
        let dir = std::env::temp_dir().join(format!("aicc-extra-excl-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("private")).unwrap();
        std::fs::write(dir.join("private/data.ts"), "export const x = 1;\n").unwrap();
        std::fs::write(dir.join("keep.ts"), "export const y = 2;\n").unwrap();

        let options = ScanOptions {
            extra_excludes: vec![String::from("**/private/**")],
            ..ScanOptions::default()
        };
        let report = scan(&dir, &options).expect("scan must succeed");
        assert_eq!(skipped_map(&report).get("private/data.ts"), Some(&SkipReason::SecurityExclude));
        assert!(included_set(&report).contains(&"keep.ts"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn extra_excludes_beats_gitignore_whitelist() {
        let dir = std::env::temp_dir().join(format!("aicc-excl-git-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("locked")).unwrap();
        std::fs::write(dir.join("locked/.gitignore"), "!keep.ts\n").unwrap();
        std::fs::write(dir.join("locked/keep.ts"), "export const x = 1;\n").unwrap();

        let options =
            ScanOptions { extra_excludes: vec![String::from("locked/")], ..ScanOptions::default() };
        let report = scan(&dir, &options).expect("scan must succeed");
        assert_eq!(
            skipped_map(&report).get("locked/keep.ts"),
            Some(&SkipReason::SecurityExclude),
            "hard policy must beat whitelist: {report:?}"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn extra_excludes_path_wins_without_reading_content() {
        // Same file would be SecretContent, but hard path policy applies first.
        let dir = std::env::temp_dir().join(format!("aicc-excl-first-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("private")).unwrap();
        std::fs::write(
            dir.join("private/keys.ts"),
            "const api_key = \"aB3dE5fG7hJ9kL2mN4pQ6rS8tU0vW2xY4\";\n",
        )
        .unwrap();
        let options = ScanOptions {
            extra_excludes: vec![String::from("**/private/**")],
            ..ScanOptions::default()
        };
        let report = scan(&dir, &options).expect("scan must succeed");
        assert_eq!(skipped_map(&report).get("private/keys.ts"), Some(&SkipReason::SecurityExclude));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn invalid_extra_excludes_fails_fast() {
        let err = scan(
            &fixture("basic-repo"),
            &ScanOptions {
                extra_excludes: vec![String::from("[unclosed")],
                ..ScanOptions::default()
            },
        )
        .expect_err("invalid glob must fail");
        assert!(matches!(err, ScanError::InvalidExcludePattern { .. }));

        let err = scan(
            &fixture("basic-repo"),
            &ScanOptions {
                extra_excludes: vec![String::from("!keep.ts")],
                ..ScanOptions::default()
            },
        )
        .expect_err("negation must be rejected");
        assert!(matches!(err, ScanError::InvalidExcludePattern { .. }));
    }

    #[test]
    fn oversized_file_is_excluded_before_full_read() {
        let dir = std::env::temp_dir().join(format!("aicc-too-large-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        // 200 bytes with a secret at the end; limit 100 forces TooLarge first.
        let mut body = "a".repeat(150);
        body.push_str("\napi_key = \"aB3dE5fG7hJ9kL2mN4pQ6rS8tU0vW2xY4\"\n");
        std::fs::write(dir.join("big.ts"), &body).unwrap();
        let options = ScanOptions { max_file_bytes: 100, ..ScanOptions::default() };
        let report = scan(&dir, &options).expect("scan must succeed");
        assert_eq!(skipped_map(&report).get("big.ts"), Some(&SkipReason::TooLarge));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn secret_canary_never_appears_in_report() {
        let canary = "CANARY827364aB3dE5fG7hJ9kL2mN4pQ6rS8tU0";
        let dir = std::env::temp_dir().join(format!("aicc-canary-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("app.ts"), format!("const api_key = \"{canary}\";\n")).unwrap();

        let report = scan(&dir, &ScanOptions::default()).expect("scan must succeed");
        assert_eq!(skipped_map(&report).get("app.ts"), Some(&SkipReason::SecretContent));
        let json = serde_json::to_string(&report).expect("report serializes");
        assert!(!json.contains(canary), "serialized report leaked canary");
        for w in &report.warnings {
            assert!(!w.message.contains(canary));
            assert!(w.rel_path.as_deref().is_none_or(|p| !p.contains(canary)));
        }
        for (rel, _) in &report.skipped {
            assert!(!rel.contains(canary));
        }

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn secrets_scan_is_deterministic() {
        let first = scan_fixture("secrets-repo", &ScanOptions::default());
        let second = scan_fixture("secrets-repo", &ScanOptions::default());
        assert_eq!(first, second);
    }

    #[test]
    fn options_come_from_full_app_config() {
        let mut config = aicc_core::config::AppConfig::default();
        config.index.max_file_bytes = 4321;
        config.index.extra_ignores = vec![String::from("gen/**")];
        config.security.exclude_secrets = false;
        config.security.extra_excludes = vec![String::from("**/private/**")];
        let options = ScanOptions::from_app_config(&config);
        assert_eq!(options.max_file_bytes, 4321);
        assert_eq!(options.extra_ignores, vec![String::from("gen/**")]);
        assert!(!options.exclude_secrets);
        assert_eq!(options.extra_excludes, vec![String::from("**/private/**")]);
    }

    // ---- Fase 6: metadata helpers ----

    #[test]
    fn line_counts_follow_documented_rule() {
        assert_eq!(count_lines(b""), 0);
        assert_eq!(count_lines(b"a"), 1);
        assert_eq!(count_lines(b"a\n"), 1);
        assert_eq!(count_lines(b"a\nb"), 2);
        assert_eq!(count_lines(b"a\nb\n"), 2);
        assert_eq!(count_lines(b"a\r\nb\r\n"), 2);
        assert_eq!(count_lines(b"a\r\nb"), 2);
        // Non-UTF8 never panics; lone CR is not a break.
        assert_eq!(count_lines(b"\xff\n\xfe"), 2);
        assert_eq!(count_lines(b"a\rb"), 1);
    }

    #[test]
    fn content_hash_is_versioned_and_deterministic() {
        let a = content_hash_for(b"hello");
        let b = content_hash_for(b"hello");
        let c = content_hash_for(b"world");
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert!(a.starts_with("blake3-v1:"), "{a}");
        assert_eq!(a.len(), "blake3-v1:".len() + 64);
        assert!(a["blake3-v1:".len()..].chars().all(|ch| ch.is_ascii_hexdigit()));
        assert_eq!(HASH_ALGORITHM_ID, "blake3-v1");
    }

    #[test]
    fn extension_normalization_is_consistent() {
        use std::path::PathBuf;
        assert_eq!(normalized_extension(&PathBuf::from("a.RS")), Some(String::from("rs")));
        assert_eq!(normalized_extension(&PathBuf::from("Makefile")), None);
        assert_eq!(normalized_extension(&PathBuf::from(".gitignore")), None);
    }

    // ---- Fase 6: metadata integration ----

    fn entry_by_rel<'a>(report: &'a ScanReport, rel: &str) -> &'a ScannedEntry {
        report.included.iter().find(|e| e.rel_path == rel).expect("admitted entry")
    }

    #[test]
    fn metadata_extension_present_and_absent() {
        let dir = std::env::temp_dir().join(format!("aicc-meta-ext-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.rs"), "fn ok() {}\n").unwrap();
        std::fs::write(dir.join("Makefile"), "all:\n\techo hi\n").unwrap();

        let report = scan(&dir, &ScanOptions::default()).expect("scan must succeed");
        assert_eq!(entry_by_rel(&report, "a.rs").extension, Some(String::from("rs")));
        assert_eq!(entry_by_rel(&report, "Makefile").extension, None);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn metadata_size_matches_measured_content() {
        let dir = std::env::temp_dir().join(format!("aicc-meta-size-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let body = "fn ok() {}\n// plain\n";
        std::fs::write(dir.join("a.rs"), body).unwrap();

        let report = scan(&dir, &ScanOptions::default()).expect("scan must succeed");
        let entry = entry_by_rel(&report, "a.rs");
        assert_eq!(entry.size_bytes, body.len() as u64);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn metadata_language_and_source() {
        let dir = std::env::temp_dir().join(format!("aicc-meta-lang-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.rs"), "fn ok() {}\n").unwrap();
        std::fs::write(dir.join("b.TS"), "export const x = 1;\n").unwrap();
        std::fs::write(dir.join("Makefile"), "all:\n\techo hi\n").unwrap();

        let report = scan(&dir, &ScanOptions::default()).expect("scan must succeed");
        let rs = entry_by_rel(&report, "a.rs");
        assert_eq!(rs.language, "rust");
        assert_eq!(rs.detection_source.id(), "extension");
        let ts = entry_by_rel(&report, "b.TS");
        assert_eq!(ts.language, "typescript");
        let mk = entry_by_rel(&report, "Makefile");
        assert_eq!(mk.language, "unknown");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn metadata_line_counts_cover_empty_no_trailing_lf_and_crlf() {
        let dir = std::env::temp_dir().join(format!("aicc-meta-lines-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("empty.rs"), "").unwrap();
        std::fs::write(dir.join("unterminated.rs"), "fn a() {}").unwrap();
        std::fs::write(dir.join("crlf.rs"), "fn a() {}\r\nfn b() {}\r\n").unwrap();

        let report = scan(&dir, &ScanOptions::default()).expect("scan must succeed");
        assert_eq!(entry_by_rel(&report, "empty.rs").line_count, 0);
        assert_eq!(entry_by_rel(&report, "unterminated.rs").line_count, 1);
        assert_eq!(entry_by_rel(&report, "crlf.rs").line_count, 2);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn metadata_parser_unavailable_in_fase_6() {
        let report = scan_fixture("basic-repo", &ScanOptions::default());
        assert!(!report.included.is_empty());
        for entry in &report.included {
            assert!(!entry.parser_available, "{} must report no parser", entry.rel_path);
        }
    }

    #[test]
    fn metadata_admitted_state_is_coherent() {
        let report = scan_fixture("basic-repo", &ScanOptions::default());
        let skipped = skipped_map(&report);
        for entry in &report.included {
            assert!(!entry.language.is_empty(), "{}", entry.rel_path);
            assert!(entry.content_hash.starts_with("blake3-v1:"), "{}", entry.rel_path);
            assert!(!skipped.contains_key(entry.rel_path.as_str()));
        }
    }

    #[test]
    fn metadata_excluded_files_have_no_invented_content() {
        // Secret, oversized and path-excluded files never yield admitted
        // entries, hence no hash/line count of unread content exists.
        let dir = std::env::temp_dir().join(format!("aicc-meta-excl-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(".env"), "FAKE=1\n").unwrap();
        std::fs::write(
            dir.join("secret.ts"),
            "const api_key = \"FAKEaB3dE5fG7hJ9kL2mN4pQ6TESTONLY\";\n",
        )
        .unwrap();
        std::fs::write(dir.join("big.ts"), "a".repeat(200)).unwrap();
        let options = ScanOptions { max_file_bytes: 100, ..ScanOptions::default() };
        let report = scan(&dir, &options).expect("scan must succeed");
        let skipped = skipped_map(&report);
        assert_eq!(skipped.get(".env"), Some(&SkipReason::Sensitive));
        assert_eq!(skipped.get("secret.ts"), Some(&SkipReason::SecretContent));
        assert_eq!(skipped.get("big.ts"), Some(&SkipReason::TooLarge));
        assert!(report.included.is_empty());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn metadata_hash_of_secret_never_enters_admitted_set() {
        let dir = std::env::temp_dir().join(format!("aicc-meta-hash-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let secret = "const api_key = \"FAKEaB3dE5fG7hJ9kL2mN4pQ6TESTONLY\";\n";
        std::fs::write(dir.join("secret.ts"), secret).unwrap();
        std::fs::write(dir.join("plain.ts"), "export const x = 1;\n").unwrap();

        let report = scan(&dir, &ScanOptions::default()).expect("scan must succeed");
        let secret_hash = content_hash_for(secret.as_bytes());
        assert!(!report.included.iter().any(|e| e.content_hash == secret_hash));
        assert!(included_set(&report).contains(&"plain.ts"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn metadata_json_and_debug_hide_raw_content() {
        let dir = std::env::temp_dir().join(format!("aicc-meta-json-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let unique = "purple-monkey-dishwasher-plain-text-line-827364";
        std::fs::write(dir.join("note.txt"), format!("{unique}\nsecond line\n")).unwrap();

        let report = scan(&dir, &ScanOptions::default()).expect("scan must succeed");
        assert!(included_set(&report).contains(&"note.txt"));
        let json = serde_json::to_string(&report).expect("report serializes");
        assert!(json.contains("note.txt"));
        assert!(!json.contains(unique), "raw content leaked into JSON");
        let debug = format!("{report:?}");
        assert!(!debug.contains(unique), "raw content leaked into Debug");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn metadata_debug_hides_secret_canary() {
        let canary = "CANARYMETA827364aB3dE5fG7hJ9kL2mN4pQ6rS8";
        let dir = std::env::temp_dir().join(format!("aicc-meta-canary-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("app.ts"), format!("const api_key = \"{canary}\";\n")).unwrap();

        let report = scan(&dir, &ScanOptions::default()).expect("scan must succeed");
        assert_eq!(skipped_map(&report).get("app.ts"), Some(&SkipReason::SecretContent));
        let json = serde_json::to_string(&report).expect("report serializes");
        assert!(!json.contains(canary));
        let debug = format!("{report:?}");
        assert!(!debug.contains(canary), "canary leaked into Debug");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn metadata_scan_is_deterministic() {
        let dir = std::env::temp_dir().join(format!("aicc-meta-determ-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(dir.join("src/a.rs"), "fn a() {}\n").unwrap();
        std::fs::write(dir.join("src/b.py"), "x = 1\n").unwrap();

        let first = scan(&dir, &ScanOptions::default()).expect("scan must succeed");
        let second = scan(&dir, &ScanOptions::default()).expect("scan must succeed");
        assert_eq!(first, second);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn metadata_exclude_secrets_false_keeps_opt_out() {
        // Opt-out admits the secret file WITH content-derived metadata; the
        // Fase 5 contract is unchanged, only explicitly disabled.
        let dir = std::env::temp_dir().join(format!("aicc-meta-optout-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let secret = "const api_key = \"FAKEaB3dE5fG7hJ9kL2mN4pQ6TESTONLY\";\n";
        std::fs::write(dir.join("secret.ts"), secret).unwrap();

        let on = scan(&dir, &ScanOptions::default()).expect("scan must succeed");
        assert_eq!(skipped_map(&on).get("secret.ts"), Some(&SkipReason::SecretContent));

        let off = scan(&dir, &ScanOptions { exclude_secrets: false, ..ScanOptions::default() })
            .expect("scan must succeed");
        let entry = entry_by_rel(&off, "secret.ts");
        assert_eq!(entry.content_hash, content_hash_for(secret.as_bytes()));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
