# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/) after 1.0.
Before 1.0, `0.x` releases may evolve APIs (config/JSON versioned separately).

## [Unreleased]

### Added

- Fase 4 secure scanner: `ignore`-crate traversal with `.gitignore` layers,
  default/extra pruning, binary probe, size limits, docs/tests gates,
  per-file warnings and typed skip reasons (`ScanReport`).

- Cargo workspace foundation (`core`, `scanner`, `parser`, `graph`, `index`, `retrieval`, `context`, `mcp`, `cli`).
- CLI contract: `init`, `analyze`, `index`, `search`, `context`, `map`, `status`, `config`, `mcp`, `version` (+ `--json`, `--path`, `--verbose`).
- Versioned TOML configuration (`.context.toml`, `version = 1`) with safe defaults.
- Project root discovery (`.git` / `.context.toml` / manifests).
- Conservative security classification baseline + platform cache-dir paths.
- Docs: README, ARCHITECTURE (Current vs Planned), CONTRIBUTING, SECURITY, docs/ + ADRs 0001–0006.
- CI (fmt/check/test/clippy on ubuntu/windows/macos) + release skeleton.
- Examples, fixtures, integration-test harness, benchmark skeleton.

### Known limitations

- `analyze`, `index`, `search`, `context`, `map`, `mcp` are honest skeletons (owning roadmap phases reported at runtime).
- No SQLite/FTS5, Tree-sitter or embeddings yet (planned seams only).
