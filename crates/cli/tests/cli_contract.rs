//! CLI contract tests (Fase 2): help, version, init/status/config flows,
//! `--json` stdout purity, and honest skeletons for future commands.

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::tempdir;

fn aicc() -> Command {
    Command::cargo_bin("aicc").expect("binary `aicc` must build")
}

#[test]
fn help_lists_planned_commands() {
    aicc()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("init"))
        .stdout(predicate::str::contains("search"))
        .stdout(predicate::str::contains("mcp"));
}

#[test]
fn version_flag_works() {
    aicc().arg("--version").assert().success().stdout(predicate::str::contains("aicc"));
}

#[test]
fn version_subcommand_json_shape() {
    let out = aicc().args(["--json", "version"]).assert().success().get_output().stdout.clone();
    let v: serde_json::Value = serde_json::from_slice(&out).expect("valid JSON on stdout");
    assert_eq!(v["binary"], "aicc");
    assert!(v["version"].is_string());
}

#[test]
fn init_creates_config_and_status_reads_it() {
    let dir = tempdir().unwrap();
    // Make it look like a project root.
    fs::write(dir.path().join("package.json"), r#"{"name":"t"}"#).unwrap();

    aicc().args(["init", "--path"]).arg(dir.path()).assert().success();

    assert!(dir.path().join(".context.toml").exists());

    aicc()
        .args(["status", "--path"])
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Root:"));
}

#[test]
fn future_commands_are_honest_not_fake() {
    for cmd in ["analyze", "index", "search", "context", "map", "mcp"] {
        let args: Vec<&str> = match cmd {
            "search" => vec![cmd, "auth"],
            "context" => vec![cmd, "add login"],
            _ => vec![cmd],
        };
        aicc()
            .args(&args)
            .assert()
            .failure()
            .stderr(predicate::str::contains("not yet implemented"));
    }
}

#[test]
fn json_stdout_stays_pure() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("Cargo.toml"), "[package]\nname=\"t\"\n").unwrap();

    let output = aicc()
        .args(["--json", "status", "--path"])
        .arg(dir.path())
        .assert()
        .success()
        .get_output()
        .clone();

    // stdout must be exactly one JSON document.
    let v: serde_json::Value = serde_json::from_slice(&output.stdout).expect("stdout is pure JSON");
    assert!(v.get("root").is_some());
    // diagnostics (if any) live on stderr, never mixed into stdout.
    let stdout_text = String::from_utf8_lossy(&output.stdout);
    assert!(stdout_text.trim_start().starts_with('{'));
}
