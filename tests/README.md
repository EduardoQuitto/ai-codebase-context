# Shared test fixtures

Deterministic mini-repos for scanner / parser / retrieval tests.
Runnable integration tests live in `crates/cli/tests/` (cargo-discoverable);
this directory holds the *data* so every crate tests against the same layout.

- `basic-repo/`: source + docs + `node_modules/` + binary + `.env` (fake secret) + large file.
  Scanner must include only the allowed sources.
- `secrets-repo/`: fake credential patterns for Fase 5 classification tests.

## Force-added fixtures

Some fixture files match the fixture's own `.gitignore` (`*.log`,
`/ignored-by-git.txt`, `build-git/`, nested `inner-ignored.txt`) and were
added with `git add -f`. This is intentional: git ignores untracked files
matching those rules, but the scanner tests need them on disk to prove
`GitIgnore` attribution. Once tracked, git keeps tracking them.
All secrets in fixtures are fake (`FAKE_*`, documented in each file).
