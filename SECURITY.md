# Security Policy

This tool reads potentially sensitive code. Security is structural, not a follow-up.

## Scope

- Local file scanning, parsing (static only), indexing, retrieval, export, MCP.
- Threats we design against: secret leakage into context/logs, path traversal outside the project root, malicious repos (no code execution), overly broad MCP permissions.

## Guarantees (foundation)

- Static analysis only: the tool never executes repo code to understand it.
- Conservative defaults: `.env`, keys, certs, credential files are excluded from default context and sanitized in diagnostics.
- Local-first: no telemetry, no mandatory network, no silent uploads. An external LLM is only ever an explicit, optional consumer.
- Index outside the repo (user cache dir) so derived data is never committed accidentally.
- Future MCP is read-only: no shell execution, no file writes (see `crates/mcp` guard).

## Non-goals / non-claims

- `local-first` and `privacy-first` are architecture principles, not legal/compliance certifications. No compliance claims are made.
- Secret detection is heuristic (names/extensions/patterns); users remain responsible for reviewing exports before sharing.

## Reporting a vulnerability

Email: **SECURITY-CONTACT-TBD** — replace with a monitored address before public launch.

Please include: version (`aicc version`), OS/arch, repro (redacted, no secrets), impact assessment. We aim to acknowledge within 72h and will coordinate disclosure.

Do not include real credentials, customer code or private repo contents in reports.

## Handling secrets in issues/PRs

Redact tokens, keys and paths. Use `aicc status` / `RUST_LOG=debug` output only after removing sensitive lines.
