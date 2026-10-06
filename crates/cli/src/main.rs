//! `aicc` — local-first codebase context layer (foundation CLI).
//!
//! Thin entry point: init observability, parse CLI, dispatch, report errors
//! as friendly diagnostics (no stack traces for normal failures).

mod cli;
mod commands;
mod output;

use clap::Parser as _;
use cli::Cli;

fn main() -> miette::Result<()> {
    let cli = Cli::parse();
    init_tracing(cli.verbose);

    if let Err(err) = commands::dispatch(cli) {
        // User-facing: short message on stderr; details via RUST_LOG=debug.
        eprintln!("Error: {err}");
        std::process::exit(1);
    }
    Ok(())
}

/// Initialize `tracing` writing to stderr only, so `--json` stdout stays pure.
fn init_tracing(verbose: bool) {
    use tracing_subscriber::{EnvFilter, fmt};
    let filter = if verbose {
        String::from("debug")
    } else {
        std::env::var("RUST_LOG").unwrap_or_else(|_| String::from("warn"))
    };
    let _ = fmt()
        .with_env_filter(EnvFilter::try_new(filter).unwrap_or_else(|_| EnvFilter::new("warn")))
        .with_writer(std::io::stderr)
        .try_init();
}
