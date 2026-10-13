//! Conservative content classification (Fase 5).
//!
//! Rules:
//! - never execute repo code to classify it;
//! - secrets are excluded from default context and sanitized in diagnostics;
//! - path-based classification excludes early without reading when possible;
//! - content inspection is bounded, local-only and never returns secret values.
//!
//! Fase 5 adds high-confidence *content* patterns on top of the Fase 4
//! name/extension baseline. The detector is intentionally small, deterministic
//! and dependency-free (plain string scans, no regex crate, no network, no AI).
//! It is a defense-in-depth layer, not a leak-proof guarantee: absence of a
//! finding does NOT prove a file is secret-free.

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

/// High-confidence secret family found in file *content*.
///
/// Carries no payload on purpose: callers learn *which family* matched, never
/// the matched value, span or surrounding text. All variants are fieldless so
/// `Debug`/serialization cannot leak captures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecretKind {
    /// `-----BEGIN ... PRIVATE KEY-----` PEM block marker.
    PrivateKey,
    /// Recognizable provider token shape (`ghp_…`, `xoxb-…`, `sk_live_…`…).
    ProviderToken,
    /// Database URL with embedded `user:password@` userinfo.
    DatabaseCredential,
    /// Explicit `name = "value"` credential assignment with a non-placeholder
    /// value meeting length/composition criteria.
    CredentialAssignment,
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
/// patterns, see [`contains_secret`]) runs later in the scanner on bounded
/// candidate content; this is the early path-only gate.
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

// ---------------------------------------------------------------------------
// Fase 5 content detector (initial high-confidence selection, not exhaustive).
//
// Intent and limits:
// - PEM markers: generic `-----BEGIN … PRIVATE KEY-----` plus known variants
//   (RSA/EC/DSA/OPENSSH). A marker alone is high-confidence.
// - Provider tokens: only shapes that are secrets by themselves. `AKIA…`
//   (AWS key id, a public identifier) alone is NOT flagged; AWS-style secrets
//   must arrive as assignments (see below), preserving the pair requirement.
// - Database URLs: known schemes with non-empty `user:password@` userinfo.
// - Assignments: suspicious name AND quoted value with length/composition
//   criteria AND explicit placeholder filtering. Name alone never flags.
// Placeholders such as `changeme`, `replace_me`, `your_api_key`, `example`,
// `placeholder` (and equivalents) suppress assignment/DB findings.
// Long random-looking strings without a known shape are NOT flagged: no
// entropy-only detection, no length-only rule.
// ---------------------------------------------------------------------------

/// Placeholder fragments (lowercased) that suppress assignment/DB findings.
/// `fake`/`testonly` are intentionally NOT here: synthetic fixtures use
/// `FAKE…TESTONLY` markers and must still be detected.
const PLACEHOLDER_FRAGMENTS: &[&str] = &[
    "changeme",
    "change_me",
    "replace_me",
    "replaceme",
    "your_api_key",
    "your_key",
    "your_token",
    "your_secret",
    "your_password",
    "example",
    "sample",
    "placeholder",
    "dummy",
    "password123",
    "test123",
    "123456",
    "xxx",
    "***",
    "todo",
    "fixme",
];

/// Suspicious assignment names (lowercased substrings). A match alone never
/// flags; the quoted value must also meet [`value_looks_secret`].
const SUSPICIOUS_ASSIGNMENT_NAMES: &[&str] = &[
    "api_key",
    "apikey",
    "api-key",
    "api_secret",
    "access_token",
    "auth_token",
    "client_secret",
    "secret_key",
    "secret_access_key",
    "aws_secret",
    "private_key",
    "app_secret",
];

/// GitHub-style token prefixes (secret by themselves, conservative length).
const GITHUB_PREFIXES: &[&str] = &["ghp_", "gho_", "ghu_", "ghs_", "ghr_"];
/// Slack-style token prefixes.
const SLACK_PREFIXES: &[&str] = &["xoxb-", "xoxp-", "xoxa-", "xoxr-", "xoxs-"];
/// Stripe-style live/test secret prefixes.
const STRIPE_PREFIXES: &[&str] = &["sk_live_", "sk_test_", "rk_live_", "rk_test_"];

/// Database URL schemes (lowercase) considered for embedded credentials.
const DB_SCHEMES: &[&str] = &[
    "postgres://",
    "postgresql://",
    "mysql://",
    "mongodb://",
    "mongodb+srv://",
    "redis://",
    "rediss://",
];

/// Returns the matched secret family, or `None` when no high-confidence
/// pattern is present. Never returns values, spans or context snippets.
#[must_use]
pub fn find_secret_kind(content: &str) -> Option<SecretKind> {
    if has_private_key_marker(content) {
        return Some(SecretKind::PrivateKey);
    }
    if has_provider_token(content) {
        return Some(SecretKind::ProviderToken);
    }
    if has_database_credential(content) {
        return Some(SecretKind::DatabaseCredential);
    }
    if has_credential_assignment(content) {
        return Some(SecretKind::CredentialAssignment);
    }
    None
}

/// Convenience wrapper over [`find_secret_kind`].
#[must_use]
pub fn contains_secret(content: &str) -> bool {
    find_secret_kind(content).is_some()
}

fn has_private_key_marker(content: &str) -> bool {
    content.contains("-----BEGIN") && content.contains("PRIVATE KEY-----")
}

fn has_provider_token(content: &str) -> bool {
    for prefix in GITHUB_PREFIXES {
        if has_tail_alnum(content, prefix, 20) {
            return true;
        }
    }
    for prefix in SLACK_PREFIXES {
        if has_tail_token_chars(content, prefix, 10) {
            return true;
        }
    }
    for prefix in STRIPE_PREFIXES {
        if has_tail_alnum(content, prefix, 16) {
            return true;
        }
    }
    false
}

/// Checks `prefix` followed by at least `min` consecutive ASCII alphanumerics.
fn has_tail_alnum(content: &str, prefix: &str, min: usize) -> bool {
    has_tail_with(content, prefix, min, |b| b.is_ascii_alphanumeric())
}

/// Checks `prefix` followed by at least `min` `[A-Za-z0-9-]` chars.
fn has_tail_token_chars(content: &str, prefix: &str, min: usize) -> bool {
    has_tail_with(content, prefix, min, |b| b.is_ascii_alphanumeric() || b == b'-')
}

fn has_tail_with(content: &str, prefix: &str, min: usize, allowed: fn(u8) -> bool) -> bool {
    let bytes = content.as_bytes();
    let needle = prefix.as_bytes();
    if needle.is_empty() || bytes.len() < needle.len() {
        return false;
    }
    let mut start = 0;
    while start + needle.len() <= bytes.len() {
        let Some(offset) = find_subslice(&bytes[start..], needle) else {
            break;
        };
        let tail = start + offset + needle.len();
        let mut len = 0;
        while tail + len < bytes.len() && allowed(bytes[tail + len]) {
            len += 1;
        }
        if len >= min {
            return true;
        }
        start = tail.max(start + 1);
    }
    false
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    haystack.windows(needle.len()).position(|w| w == needle)
}

fn has_database_credential(content: &str) -> bool {
    // Case-insensitive scheme search on a lowercased copy; only existence is
    // reported, never the userinfo itself.
    let lower = content.to_ascii_lowercase();
    for scheme in DB_SCHEMES {
        let mut search_from = 0;
        while let Some(rel) = lower[search_from..].find(scheme) {
            let abs = search_from + rel;
            let after = abs + scheme.len();
            // Token ends at whitespace or typical delimiters.
            let rest = &lower[after..];
            let token_end = rest
                .find(|c: char| c.is_whitespace() || matches!(c, '"' | '\'' | '<' | '>' | '`'))
                .map(|i| after + i)
                .unwrap_or(lower.len());
            if after < token_end {
                let token = &lower[after..token_end];
                // Userinfo is the part before the first '@' inside the token.
                if let Some(at) = token.find('@') {
                    let userinfo = &token[..at];
                    if let Some(colon) = userinfo.find(':') {
                        let user = &userinfo[..colon];
                        let password = &userinfo[colon + 1..];
                        if !user.is_empty()
                            && password.len() >= 3
                            && !password.contains('/')
                            && !is_placeholder(password)
                        {
                            return true;
                        }
                    }
                }
            }
            search_from = after.max(abs + 1);
            if search_from >= lower.len() {
                break;
            }
        }
    }
    false
}

fn has_credential_assignment(content: &str) -> bool {
    for line in content.lines() {
        if line.len() > 4096 {
            // Skip pathological single-line blobs; other detectors still apply.
            continue;
        }
        let lower = line.to_ascii_lowercase();
        if !SUSPICIOUS_ASSIGNMENT_NAMES.iter().any(|n| lower.contains(n)) {
            continue;
        }
        if let Some(value) = extract_quoted_value_after_assign(line, &lower)
            && value_looks_secret(&value)
        {
            return true;
        }
    }
    false
}

/// Finds the first single/double-quoted value after `=`/`:` that follows the
/// suspicious name. Returns the raw value (unquoted); callers never log it.
fn extract_quoted_value_after_assign(line: &str, lower: &str) -> Option<String> {
    let name_end = SUSPICIOUS_ASSIGNMENT_NAMES
        .iter()
        .filter_map(|n| lower.find(n).map(|pos| pos + n.len()))
        .min()?;
    let bytes = line.as_bytes();
    // Separator `=` or `:` after the name.
    let mut sep = None;
    for (i, b) in bytes.iter().enumerate().skip(name_end) {
        if *b == b'=' || *b == b':' {
            sep = Some(i);
            break;
        }
        // Stop at end of a short preamble; names are always near the separator.
        if i - name_end > 64 {
            break;
        }
    }
    let sep = sep?;
    // Opening quote after the separator.
    let mut open = None;
    let mut quote = 0u8;
    for (i, b) in bytes.iter().enumerate().skip(sep + 1) {
        if *b == b'"' || *b == b'\'' {
            open = Some(i);
            quote = *b;
            break;
        }
        if i - sep > 64 {
            break;
        }
    }
    let open = open?;
    // Closing matching quote (bounded to avoid pathological scans).
    let limit = (open + 1 + 512).min(bytes.len());
    for (i, b) in bytes.iter().enumerate().take(limit).skip(open + 1) {
        if *b == quote {
            return Some(line[open + 1..i].to_string());
        }
    }
    None
}

/// Value criteria: long enough, letter+digit mix, no whitespace, not a
/// placeholder. Name alone is never sufficient.
fn value_looks_secret(value: &str) -> bool {
    if value.len() < 12 || value.len() > 512 {
        return false;
    }
    if value.chars().any(char::is_whitespace) {
        return false;
    }
    let has_letter = value.bytes().any(|b| b.is_ascii_alphabetic());
    let has_digit = value.bytes().any(|b| b.is_ascii_digit());
    if !(has_letter && has_digit) {
        return false;
    }
    !is_placeholder(value)
}

fn is_placeholder(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    PLACEHOLDER_FRAGMENTS.iter().any(|p| lower.contains(p))
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

    // ---- Fase 5 content detector: positives ----

    #[test]
    fn private_key_marker_is_detected() {
        let body = "header\n-----BEGIN RSA PRIVATE KEY-----\nFAKEBODYTESTONLY\n-----END RSA PRIVATE KEY-----\n";
        assert_eq!(find_secret_kind(body), Some(SecretKind::PrivateKey));
        let generic = "-----BEGIN PRIVATE KEY-----\nAAAABBBB\n-----END PRIVATE KEY-----";
        assert_eq!(find_secret_kind(generic), Some(SecretKind::PrivateKey));
    }

    #[test]
    fn provider_tokens_are_detected() {
        let gh = "token = \"ghp_FAKE0123456789abcdefghijTESTONLY\"";
        assert_eq!(find_secret_kind(gh), Some(SecretKind::ProviderToken));
        let slack = "xoxb-FAKE12345678901234567890TESTONLY";
        assert_eq!(find_secret_kind(slack), Some(SecretKind::ProviderToken));
        let stripe = "key=sk_live_FAKE1234567890123456";
        assert_eq!(find_secret_kind(stripe), Some(SecretKind::ProviderToken));
    }

    #[test]
    fn database_url_with_credentials_is_detected() {
        let url = "postgres://appuser:FAKEs3cr3tP4ssw0rdTESTONLY@db.internal:5432/appdb";
        assert_eq!(find_secret_kind(url), Some(SecretKind::DatabaseCredential));
    }

    #[test]
    fn credential_assignment_with_strong_value_is_detected() {
        let line = "api_key = \"aB3dE5fG7hJ9kL2mN4pQ6rS8tU0vW2xY4\"";
        assert_eq!(find_secret_kind(line), Some(SecretKind::CredentialAssignment));
    }

    // ---- Fase 5 content detector: negatives ----

    #[test]
    fn plain_code_is_not_flagged() {
        assert_eq!(find_secret_kind("fn ok() {}\nlet x = 1;\n"), None);
        assert!(!contains_secret("hello world, no secrets here\n"));
    }

    #[test]
    fn suspicious_name_without_value_is_not_flagged() {
        assert_eq!(find_secret_kind("const api_key = \"\";\n"), None);
        assert_eq!(find_secret_kind("const api_key = \"test\";\n"), None);
        assert_eq!(find_secret_kind("token = \"none\";\n"), None);
    }

    #[test]
    fn placeholders_are_not_flagged() {
        for value in [
            "changeme",
            "replace_me_123",
            "your_api_key_here_abc123",
            "example-key-abc123",
            "placeholder_abc123",
        ] {
            let line = format!("api_key = \"{value}\"");
            assert_eq!(find_secret_kind(&line), None, "placeholder leaked: {value}");
        }
        // DB password placeholder is also suppressed.
        let url = "postgres://app:changeme123@db.internal/db";
        assert_eq!(find_secret_kind(url), None);
    }

    #[test]
    fn public_key_id_alone_is_not_a_secret() {
        // AKIA… is a public identifier, not a secret by itself.
        assert_eq!(find_secret_kind("AKIAIOSFODNN7EXAMPLE\n"), None);
    }

    #[test]
    fn secret_kind_carries_no_payload() {
        // Static finding only: Debug/serialization must not echo values.
        let kind = SecretKind::ProviderToken;
        let debug = format!("{kind:?}");
        assert!(!debug.contains("ghp_"));
        let json = serde_json::to_string(&kind).expect("serializable");
        assert!(json.contains("ProviderToken"));
        assert!(!json.contains("ghp_"));
    }
}
