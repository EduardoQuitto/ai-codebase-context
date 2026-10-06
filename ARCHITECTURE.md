# ARCHITECTURE

> Source of intent: `ROADMAP.md`. This file describes what exists **now** vs what is **planned**, so the foundation can evolve without rewrites.

## Problem architecture solves

Transform a heterogeneous working tree into relevant, structured, safe, queryable context for humans and agents — without sending the repo to a cloud, without requiring an LLM, and without executing repo code.

## Current architecture (foundation, Milestone A)

```text
aicc (clap CLI, human default / --json on stdout, diagnostics on stderr)
 ├── init / status / config / version   (real, minimal)
 └── analyze / index / search / context / map / mcp  (honest skeletons)
         ↓
aicc-core
 ├── branding (provisional `aicc`, easily renamable)
 ├── config (`.context.toml`, version = 1, safe defaults)
 ├── discovery (walk up to .git / .context.toml / manifests)
 ├── model (Repository, IndexedFile, Chunk, Symbol, Relationship — names frozen)
 ├── security (name/extension conservative classification)
 └── platform (user cache dir → index.sqlite3 path)
         ↓
aicc-scanner (std-only walker, default ignores, binary/size guards)
aicc-parser  (extension registry + LanguageParser trait seam)
aicc-graph   (directed multigraph, typed edges + confidence)
aicc-index   (schema gate + path resolution; no SQLite yet)
aicc-retrieval (Query / RetrievalResult + MatchReason contracts)
aicc-context (ContextRequest, ContextTier, char-based estimator, budget select)
aicc-mcp     (planned tool names + read-only guard; no transport yet)
```

### Dependency rules

- `cli → {core, scanner, parser, graph, index, retrieval, context, mcp}` (currently cli uses `core` only; others wired as phases land).
- Domain crates depend on `core`, never on `cli`.
- No cycles. No MCP/shell/exec in the core. No provider SDKs anywhere in the core.

## Planned architecture (Milestones B–D, abridged)

```text
Repository → Discovery → Secure Scanner (ignore crate) → Language Detection
 → Tree-sitter Parsing → Symbols/Relations → Chunk Builder → SQLite + FTS5 (+ optional local embeddings)
 → Retrieval (lexical + semantic + structural, hybrid weights, MMR diversity)
 → Context Compiler (budget, tiers, dedup, provenance)
 → CLI / Markdown / JSON / MCP (STDIO, read-only)
```

Key seams already reserved:

| Concern | Now | Later | Decision |
|---|---|---|---|
| Traversal + gitignore | std walker | `ignore` crate | ROADMAP §4.7 |
| Parsing | extension registry + trait | Tree-sitter grammars | ADR 0004 |
| Storage | path + schema gate | SQLite + FTS5, versioned migrations | ADR 0003 |
| Config | TOML `version=1` | same, migratable | ADR 0005 |
| Embeddings | `semantic=false` default | `fastembed` local, optional | ROADMAP §4.9 |
| MCP | tool-name guard | STDIO read-only server | ROADMAP §4.11 |
| Language | Rust 2024, MSRV 1.97 | same, tested in CI | ADR 0002 |

## Data flow (steady state, future)

```text
scan → fingerprints → parse changed → upsert chunks/symbols/relations
 → FTS5 + vectors → hybrid score → budget select → package + provenance → adapter
```

## Security & privacy boundaries

```text
filesystem → index → retrieval → adapter → external agent
```

- Each arrow re-applies policy (index-time exclusion ≠ export-time re-check bypass).
- Secrets never enter default context; diagnostics print references, not contents.
- MCP exposes read-only tools only; no shell, no writes (enforced by `is_read_only_tool`).

## Cross-platform

Only `std::path`, `std::fs` and the `directories` crate. No shell-outs, no hardcoded separators, no Unix-only deps. CI covers `ubuntu / windows / macos` on x64; ARM64 joins at distribution (Fase 27).

## Evolution notes

- New languages: detector + grammar + extractor + fixtures + benchmark (no core rewrite).
- New storage: implement behind `aicc-index` API; retrieval untouched.
- New adapters: convert `Context API → consumer format`; never duplicate ranking.
