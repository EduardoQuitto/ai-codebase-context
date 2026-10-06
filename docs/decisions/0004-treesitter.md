# ADR 0004 — Tree-sitter for structural parsing

- Status: accepted (implementation in Fase 7)
- Date: 2026-10-06

## Context

Need incremental, multi-language structural parsing that tolerates broken code and never executes repo code.

## Decision

Tree-sitter behind a `LanguageParser` trait (language detected → grammar available → confidence + symbols/relations or safe textual fallback). v1 structural priority: TypeScript, JavaScript, Python, Rust. Precision over relation quantity; uncertain edges are omitted, not invented.

## Consequences

- Core never assumes Tree-sitter is the only possible parser (abstraction survives backend swaps).
- Unknown/unsupported languages still index textually.
