# Architecture overview

> Companion to `ARCHITECTURE.md` (root). This page is the navigable version; the root file remains the normative summary.

## Current (foundation)

CLI (`aicc-cli`) over `aicc-core` (branding, config, discovery, model, security, platform). Sibling crates (`scanner`, `parser`, `graph`, `index`, `retrieval`, `context`, `mcp`) expose tested contracts with future-backed seams.

Dependency direction: `cli → domain → core`. No cycles, no provider SDKs in core, no shell/exec.

## Planned (Milestones B–D)

Secure walk (`ignore` crate) → Tree-sitter symbols → chunk builder → SQLite + FTS5 (+ optional `fastembed`) → hybrid retrieval with auditability → budgeted context compiler → CLI / JSON / Markdown / read-only MCP (STDIO).

## Invariants

1. Local-first; core works without network, GPU or API key.
2. Evidence vs inference vs generation stay distinguishable; provenance required.
3. One file failing never kills an index run.
4. Index version gates rebuilds; no silent schema migration.
