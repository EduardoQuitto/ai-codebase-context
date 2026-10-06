# Shared test fixtures

Deterministic mini-repos for scanner / parser / retrieval tests.
Runnable integration tests live in `crates/cli/tests/` (cargo-discoverable);
this directory holds the *data* so every crate tests against the same layout.

- `basic-repo/`: source + docs + `node_modules/` + binary + `.env` (fake secret) + large file.
  Scanner must include only the allowed sources.
- `secrets-repo/`: fake credential patterns for Fase 5 classification tests.
