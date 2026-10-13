# ADR 0004 — Tree-sitter for structural parsing

- Status: accepted (implemented in Fase 7 for TypeScript, JavaScript, Python, Rust)
- Date: 2026-10-06
- Implemented: 2026-10-13

## Context

Need incremental, multi-language structural parsing that tolerates broken code and never executes repo code.

## Decision

Tree-sitter behind a `LanguageParser` trait (language detected → grammar available → confidence + symbols/relations or safe textual fallback). v1 structural priority: TypeScript, JavaScript, Python, Rust. Precision over relation quantity; uncertain edges are omitted, not invented.

## Consequences

- Core never assumes Tree-sitter is the only possible parser (abstraction survives backend swaps).
- Unknown/unsupported languages still index textually.

## Fase 7 integration notes

- Runtime `tree-sitter 0.26` + compiled-in grammars `tree-sitter-typescript 0.23`
  (both `LANGUAGE_TYPESCRIPT` and `LANGUAGE_TSX`), `tree-sitter-javascript 0.25`,
  `tree-sitter-python 0.25`, `tree-sitter-rust 0.24`. All grammars share the
  `tree-sitter-language 0.1` ABI, which the 0.26 runtime loads; an ABI mismatch
  would surface as typed `ParseError::Grammar`, never a panic or fake success.
  MSRV stays 1.97 (runtime MSRV is 1.77; grammar crates are 2021-edition).
- `.tsx` selects the TSX grammar via path-aware `parse_content`; `parser_for`
  defaults TypeScript to the non-TSX grammar. Grammars ship inside the binary
  (cargo `cc` build); nothing is downloaded at execution time, no subprocesses,
  no network.
- No Tree-sitter dependency exists outside `aicc-parser`; scanner/graph/index
  stay decoupled. No graph edges are produced (Fase 8 owns relations).
