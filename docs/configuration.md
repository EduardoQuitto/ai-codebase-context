# Configuration (`.context.toml`)

> Status: contract frozen for the foundation; values below match `AppConfig`. Unknown future keys are out of scope until their phase lands.

Configuration is part of the product API and treated as a stable, versioned contract.

## File location

Project root, e.g. `./.context.toml`. Created by `aicc init`. Discovery walks up from `--path` or cwd.

## Minimal example

```toml
version = 1

[project]
name = "example"

[index]
include_docs = true
include_tests = true
max_file_bytes = 524288
extra_ignores = []

[security]
exclude_secrets = true
extra_excludes = []

[retrieval]
semantic = false
default_limit = 10

[context]
default_budget = 30000
```

## Rules

- Missing file → safe defaults (local-only, `semantic = false`, secrets excluded).
- Invalid TOML → clear error, non-zero exit.
- `version != 1` → controlled failure (no silent migration).
- Unknown fields are currently **ignored** (forward-compatible default).
  The Fase 3 completion will define the warn-vs-error policy.
- `default_limit` must be 1–1000; `default_budget` must be 1–1_000_000.
- Config is declarative data only; it is never executed.

## Security (Fase 5)

`[security]` is wired to the scanner via `ScanOptions::from_app_config`.
`ScanOptions::from_index_config` keeps the old index-only behavior with secure
defaults (`exclude_secrets = true`, `extra_excludes = []`).

- `exclude_secrets` (default `true`):
  - `true`: path-based secrets (`.env`, `*.pem`, …) are excluded before any
    content is read; remaining candidates get a single bounded read
    (≤ `max_file_bytes`) backing the binary check and the high-confidence
    content detector (PEM markers, selected provider tokens, DB URLs with
    `user:password@`, credential assignments). Hits are excluded as
    `Sensitive` (path) or `SecretContent` (content) with static reasons only.
  - `false`: explicit opt-out. Automatic secret exclusion (path + content) is
    disabled, so credential-bearing content may be admitted. This
    significantly reduces protection. Binary, generated, oversized and
    `extra_excludes` policy still apply.
- `extra_excludes` (default `[]`): hard path policy using the same
  gitignore-style globs as `index.extra_ignores`, e.g.
  `["**/private/**", "**/*.local", "config/production.toml"]`.
  - Validated at scan start; invalid patterns fail fast
    (`InvalidExcludePattern`), never silently ignored.
  - `!` negation patterns are rejected: a whitelist must never reopen a hard
    exclusion.
  - Evaluated before `.gitignore` reinclusions and without reading content;
    reported as `SecurityExclude`, distinct from `ExtraIgnore` (soft,
    `.gitignore`-reopenable) and from `Sensitive`/`SecretContent`.

See ADR [0005](decisions/0005-config-toml.md) for the TOML rationale.
See [privacy.md](privacy.md) for processing, opt-out and limitations.
