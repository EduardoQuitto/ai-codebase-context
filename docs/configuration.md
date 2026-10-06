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

See ADR [0005](decisions/0005-config-toml.md) for the TOML rationale.
