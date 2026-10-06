# ADR 0003 — SQLite (+ FTS5) for local persistence

- Status: accepted (implementation in Fase 10–11)
- Date: 2026-10-06

## Context

Need portable, serverless, structured + full-text persistence for metadata, chunks, symbols, relations and (later) vectors.

## Decision

Embedded SQLite with FTS5 for lexical search. Vectors stored locally alongside metadata (no external vector DB in MVP; ANN/HNSW only after benchmarks demand it). Index lives in the user cache dir, keyed by stable repo identity. Schema versioned; mismatches require rebuild, never silent migration.

## Consequences

- No server, no network, easy backup/cleanup (`index.sqlite3` under cache dir).
- Storage layer isolated so backends can change without rewriting retrieval.
