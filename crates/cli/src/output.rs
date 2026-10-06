//! Output helpers: stdout = result, stderr = diagnostics.
//!
//! Contract (Fase 2): when `--json` is set, stdout contains ONLY the JSON
//! payload. Human logs, progress and warnings always go to stderr.

use serde::Serialize;

/// Print a JSON payload to stdout (single document, pretty in verbose TTY is
/// intentionally avoided: stable machine output).
pub fn print_json<T: Serialize>(value: &T) {
    match serde_json::to_string_pretty(value) {
        Ok(text) => println!("{text}"),
        Err(err) => {
            eprintln!("Error: failed to serialize JSON output: {err}");
            std::process::exit(1);
        }
    }
}

/// Human line to stdout (normal result channel).
pub fn print_human(line: &str) {
    println!("{line}");
}

/// Diagnostic line to stderr (never pollutes `--json` stdout).
pub fn print_diag(line: &str) {
    eprintln!("{line}");
}
