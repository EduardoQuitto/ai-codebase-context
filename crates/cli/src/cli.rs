//! CLI contract (ROADMAP Fase 2 + §7).
//!
//! - human output by default; `--json` => structured stdout, logs on stderr;
//! - invalid usage => legible error + non-zero exit, no stack trace;
//! - future commands exist as honest skeletons (never fake success).

use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

/// Local-first codebase context layer (foundation).
#[derive(Debug, Parser)]
#[command(
    name = "aicc",
    version,
    about = "Turn any codebase into AI-ready context — locally.",
    long_about = "Local-first codebase context layer for coding agents.\nFoundation build: CLI contract + safe defaults. See ROADMAP.md.",
    propagate_version = true
)]
pub struct Cli {
    /// Project path (defaults to current directory; discovery walks up).
    #[arg(global = true, long, value_name = "DIR")]
    pub path: Option<PathBuf>,

    /// Machine-readable JSON on stdout (diagnostics stay on stderr).
    #[arg(global = true, long)]
    pub json: bool,

    /// Verbose diagnostics (debug logs on stderr).
    #[arg(global = true, long, short = 'v')]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Initialize `.context.toml` in the project root.
    Init(InitArgs),
    /// Analyze the repository and show an overview (planned: Fase 6-8).
    Analyze(CommonArgs),
    /// Build or update the persistent local index (planned: Fase 10).
    Index(IndexArgs),
    /// Search files, chunks and symbols (planned: Fase 11).
    Search(SearchArgs),
    /// Build a budgeted context package for a task (planned: Fase 12).
    Context(ContextArgs),
    /// Show the structural project map (planned: Fase 8).
    Map(CommonArgs),
    /// Show index + configuration status.
    Status(CommonArgs),
    /// Show and diagnose the loaded configuration.
    Config(CommonArgs),
    /// Start the local read-only MCP server (planned: Fase 19).
    Mcp(McpArgs),
    /// Show version and build information.
    Version,
}

#[derive(Debug, Args)]
pub struct CommonArgs {
    /// Project path override (same as global --path).
    #[arg(long, value_name = "DIR")]
    pub path: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct InitArgs {
    #[arg(long, value_name = "DIR")]
    pub path: Option<PathBuf>,
    /// Overwrite an existing `.context.toml`.
    #[arg(long)]
    pub force: bool,
}

#[derive(Debug, Args)]
pub struct IndexArgs {
    #[arg(long, value_name = "DIR")]
    pub path: Option<PathBuf>,
    /// Full rebuild even if fingerprints match (future).
    #[arg(long)]
    pub rebuild: bool,
}

#[derive(Debug, Args)]
pub struct SearchArgs {
    /// Query text.
    pub query: String,
    /// Max results (overrides config default_limit).
    #[arg(long)]
    pub limit: Option<usize>,
    /// Filter by language id (e.g. rust, typescript).
    #[arg(long)]
    pub language: Option<String>,
    /// Project path override (same as global --path).
    #[arg(long, value_name = "DIR")]
    pub path: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct ContextArgs {
    /// Task or question to build context for.
    pub task: String,
    /// Context budget (estimate; overrides config default_budget).
    #[arg(long)]
    pub budget: Option<usize>,
    /// Project path override (same as global --path).
    #[arg(long, value_name = "DIR")]
    pub path: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct McpArgs {
    /// Transport (only `stdio` is planned for v1).
    #[arg(long, default_value = "stdio")]
    pub transport: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_global_flags() {
        let cli = Cli::try_parse_from(["aicc", "--json", "status"]).unwrap();
        assert!(cli.json);
    }
}
