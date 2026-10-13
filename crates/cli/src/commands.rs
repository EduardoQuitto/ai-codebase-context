//! Command dispatch (thin; domain logic lives in library crates).

use crate::cli::{Cli, Commands};
use crate::output::{print_diag, print_human, print_json};
use aicc_core::branding::{BINARY_NAME, CONFIG_FILENAME, PRODUCT_DISPLAY_NAME};
use aicc_core::config::AppConfig;
use aicc_core::discovery::find_root;
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum CliError {
    #[error("{0}")]
    Message(String),
}

/// Resolve the effective project root: explicit flag, else cwd discovery.
fn resolve_root(start: Option<PathBuf>) -> Result<PathBuf, CliError> {
    let start = start.unwrap_or_else(|| PathBuf::from("."));
    find_root(&start).map_err(|e| CliError::Message(e.to_string()))
}

#[derive(Debug, Serialize)]
struct StatusPayload {
    product: String,
    binary: String,
    version: String,
    root: String,
    config: AppConfig,
    index: IndexState,
}

#[derive(Debug, Serialize)]
struct IndexState {
    present: bool,
    db_path: String,
    note: String,
}

pub fn dispatch(cli: Cli) -> Result<(), CliError> {
    let global_path = cli.path.clone();
    let as_json = cli.json;
    match cli.command {
        Commands::Init(args) => cmd_init(args.path.or(global_path), args.force, as_json),
        Commands::Status(args) => cmd_status(args.path.or(global_path), as_json),
        Commands::Config(args) => cmd_config(args.path.or(global_path), as_json),
        Commands::Version => cmd_version(as_json),
        Commands::Analyze(_) => not_yet("analyze", "Fase 7-8 (parser + graph)"),
        Commands::Index(_) => not_yet("index", "Fase 10 (SQLite persistence)"),
        Commands::Search(_) => not_yet("search", "Fase 11 (FTS5 lexical search)"),
        Commands::Context(_) => not_yet("context", "Fase 12 (context compiler)"),
        Commands::Map(_) => not_yet("map", "Fase 8 (structural map)"),
        Commands::Mcp(args) => {
            if args.transport != "stdio" {
                return Err(CliError::Message(String::from(
                    "unsupported MCP transport (planned: stdio only)",
                )));
            }
            not_yet("mcp", "Fase 19 (read-only STDIO server)")
        }
    }
}

fn cmd_init(path: Option<PathBuf>, force: bool, as_json: bool) -> Result<(), CliError> {
    let start = path.unwrap_or_else(|| PathBuf::from("."));
    // `find_root` already covers .git/manifest markers; fall back to `start`
    // so `init` can bootstrap a config in a fresh directory.
    let root = find_root(&start).unwrap_or(start);
    let cfg_path = root.join(CONFIG_FILENAME);

    if cfg_path.exists() && !force {
        if as_json {
            print_json(&serde_json::json!({"initialized": false, "path": cfg_path}));
        } else {
            print_human(&format!("Configuration already exists: {}", cfg_path.display()));
            print_diag("Run with --force to overwrite.");
        }
        return Ok(());
    }

    let cfg = AppConfig::default();
    std::fs::write(&cfg_path, cfg.to_toml_string())
        .map_err(|e| CliError::Message(format!("could not write {}: {e}", cfg_path.display())))?;

    if as_json {
        print_json(&serde_json::json!({"initialized": true, "path": cfg_path}));
    } else {
        print_human(&format!("Initialized configuration: {}", cfg_path.display()));
        print_diag("Next: run `aicc status` to verify discovery and defaults.");
    }
    Ok(())
}

fn cmd_status(path: Option<PathBuf>, as_json: bool) -> Result<(), CliError> {
    let root = resolve_root(path)?;
    let cfg = AppConfig::load_from_dir(&root)
        .map_err(|e| CliError::Message(format!("invalid configuration: {e}")))?;
    // Foundation: index file presence check only (no SQLite yet).
    let repo_id = repo_id_for(&root);
    let db_path = aicc_core::platform::index_db_path(&repo_id);
    let present = db_path.exists();
    let payload = StatusPayload {
        product: PRODUCT_DISPLAY_NAME.to_string(),
        binary: BINARY_NAME.to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        root: root.display().to_string(),
        config: cfg,
        index: IndexState {
            present,
            db_path: db_path.display().to_string(),
            note: if present {
                String::from("index file present (full validation in Fase 10)")
            } else {
                String::from("not indexed yet (planned: `aicc index`, Fase 10)")
            },
        },
    };
    if as_json {
        print_json(&payload);
    } else {
        print_human(&format!("Root: {}", payload.root));
        print_human(&format!(
            "Index: {}",
            if payload.index.present { "present" } else { "not present" }
        ));
        print_diag(&format!("Details: {}", payload.index.note));
    }
    Ok(())
}

fn cmd_config(path: Option<PathBuf>, as_json: bool) -> Result<(), CliError> {
    let root = resolve_root(path)?;
    let cfg = AppConfig::load_from_dir(&root)
        .map_err(|e| CliError::Message(format!("invalid configuration: {e}")))?;
    if as_json {
        print_json(&cfg);
    } else {
        print_human(&format!("Config source: {}/{}", root.display(), CONFIG_FILENAME));
        print_human(&cfg.to_toml_string());
    }
    Ok(())
}

fn cmd_version(as_json: bool) -> Result<(), CliError> {
    if as_json {
        print_json(&serde_json::json!({
            "binary": BINARY_NAME,
            "version": env!("CARGO_PKG_VERSION"),
            "index_schema": aicc_core::INDEX_SCHEMA_VERSION,
            "config_schema": aicc_core::CONFIG_SCHEMA_VERSION,
            "msrv": "1.97.0",
        }));
    } else {
        print_human(&format!(
            "{} {} (index schema v{}, config schema v{})",
            BINARY_NAME,
            env!("CARGO_PKG_VERSION"),
            aicc_core::INDEX_SCHEMA_VERSION,
            aicc_core::CONFIG_SCHEMA_VERSION,
        ));
    }
    Ok(())
}

/// Honest skeleton: never fake success and never echo user input.
/// Only the command name, its roadmap phase and the docs pointer are shown.
fn not_yet(command: &'static str, phase: &'static str) -> Result<(), CliError> {
    Err(CliError::Message(format!(
        "`{command}` is not yet implemented in this foundation build.\nPlanned: {phase}. See ROADMAP.md."
    )))
}

/// Provisional stable repo id: FNV-1a 64 over the canonical root path.
/// Deterministic across runs (unlike `DefaultHasher`); replaced by the
/// canonical identity (path + git id) in Fase 10.
fn repo_id_for(root: &Path) -> String {
    let canonical = std::fs::canonicalize(root)
        .unwrap_or_else(|_| root.to_path_buf())
        .to_string_lossy()
        .into_owned();
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in canonical.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{ContextArgs, SearchArgs};

    #[test]
    fn repo_id_is_stable() {
        let a = repo_id_for(Path::new("."));
        let b = repo_id_for(Path::new("."));
        assert_eq!(a, b);
        assert_eq!(a.len(), 16);
    }

    #[test]
    fn not_yet_hides_user_input() {
        let canary = "CANARY-QUERY-9f8e7d6c5b4a";
        let err = not_yet("search", "Fase 11 (FTS5 lexical search)").unwrap_err().to_string();
        assert!(err.contains("not yet implemented"));
        assert!(err.contains("search"));
        assert!(!err.contains(canary), "canary must never appear: {err}");
        assert!(!err.contains("Received:"), "detail echo must be gone: {err}");
    }

    #[test]
    fn search_dispatch_does_not_echo_query() {
        let canary = "CANARY-SEARCH-0123456789abcdef";
        let cli = Cli {
            path: None,
            json: false,
            verbose: false,
            command: Commands::Search(SearchArgs {
                query: canary.to_string(),
                limit: None,
                language: None,
                path: None,
            }),
        };
        let err = dispatch(cli).unwrap_err().to_string();
        assert!(!err.contains(canary), "query canary leaked: {err}");
        assert!(err.contains("not yet implemented"));
    }

    #[test]
    fn context_dispatch_does_not_echo_task() {
        let canary = "CANARY-TASK-fedcba9876543210";
        let cli = Cli {
            path: None,
            json: true,
            verbose: false,
            command: Commands::Context(ContextArgs {
                task: canary.to_string(),
                budget: None,
                path: None,
            }),
        };
        let err = dispatch(cli).unwrap_err().to_string();
        assert!(!err.contains(canary), "task canary leaked: {err}");
        assert!(err.contains("not yet implemented"));
    }
}
