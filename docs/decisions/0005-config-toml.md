# ADR 0005 — TOML project configuration

- Status: accepted
- Date: 2026-10-06
- Phase: Fase 3

## Context

Need a predictable, versioned project config for indexer/retrieval policy. YAML was considered for familiarity; TOML fits the Rust ecosystem with less ambiguity.

## Decision

TOML file `.context.toml` with required `version = 1`. Missing file → safe defaults; invalid → clear error; unknown version → controlled failure. Declarative data only, never executed.

## Consequences

- Example schema frozen in `docs/configuration.md` and `AppConfig`; changes require version bump + migration note.
- Unknown-field policy defined at implementation (warn vs error) without breaking the version gate.
