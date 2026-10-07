# AI Codebase Context (provisional name)

> Turn any codebase into AI-ready context — locally.

**Status: foundation in development.** This repository currently contains the engineering foundation (workspace, CLI contract, config, docs, CI). Retrieval, indexing and MCP arrive in later roadmap phases. Nothing here claims to work before it does.

## Problem

Coding agents get too little context (miss the implementation), too much context (cost + noise), or the wrong context (plausible but wrong answers).

## Solution

A local-first, provider-agnostic context layer:

```text
repository → discovery → secure scan → parse → index → retrieve → budgeted context → CLI / JSON / MCP
```

- Local-first: indexing, search and export work offline; no API key required.
- Private: conservative secret handling; index lives in the user cache dir, not in the repo.
- Composable: same core feeds human CLI, JSON for scripts, Markdown for LLMs, and a future read-only MCP server.
- Honest: provenance (path + lines) and selection reasons travel with every result.

## Quick start (foundation)

```sh
cargo build
./target/debug/aicc --help
./target/debug/aicc init
./target/debug/aicc status
./target/debug/aicc config
```

`analyze`, `index`, `search`, `context`, `map` and `mcp` exist as documented skeletons and clearly report their owning roadmap phase instead of faking success.

## Privacy

- No telemetry. No mandatory network. No code execution during analysis.
- `.env`, keys, certs and similar are excluded from default context and sanitized in logs.
- See [SECURITY.md](SECURITY.md) for the privacy and disclosure policy.

## Architecture (summary)

```text
CLI (crates/cli)
 ↓
core (types, config `.context.toml`, discovery, security, platform)
 ↓
scanner → parser → graph → index → retrieval → context → mcp
```

- One workspace, many small crates; modularity first, no premature micro-crates beyond the roadmap list.
- CLI is an interface over the core, never the core itself.
- Storage (SQLite + FTS5), parsing (Tree-sitter) and embeddings (fastembed) are planned seams, not current dependencies.

See [ARCHITECTURE.md](ARCHITECTURE.md) for Current vs Planned.

## Roadmap

See [ROADMAP.md](ROADMAP.md). Milestones:

- A Foundation (Fases 0–4) ← **we are here**
- B Code Intelligence Core (5–10)
- C Retrieval Core (11–16)
- D Agent Integration (17–20)
- E Production Quality (21–28)
- F Public Launch (29–35)
- G Ecosystem (36–44, post-1.0)

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). Rust 2024, stable, MSRV 1.97.0. `cargo fmt`, `cargo clippy`, `cargo test` must pass.

## License

MIT — see [LICENSE](LICENSE).
