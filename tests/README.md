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

## Fase 5 adversarial fixtures (`secrets-repo/`, all synthetic)

Positives (must be `SecretContent`, or `Sensitive` for early path exclusion):

- `notes.txt`: common name carrying `-----BEGIN RSA PRIVATE KEY-----`.
- `src/token.ts`: `ghp_FAKE…TESTONLY` provider token shape.
- `src/db.ts`: `postgres://appuser:…@…` DB URL with embedded credentials.
- `src/config.js`: `api_key = "aB3d…"` non-placeholder assignment.
- `large_note.txt`: ~9 KiB padding then an assignment, proving inspection is
  not limited to the first 8 KiB.
- `.env`, `deploy.pem`: early path exclusion (`Sensitive`).

Negatives (must stay included with default options):

- `src/main.rs`: plain code.
- `src/empty.js`: suspicious names with empty/short values.
- `src/placeholder.js` + `docs/example.md`: `changeme`/`replace_me`/
  `your_api_key`/`example`/`placeholder` values.
- `src/public_id.txt`: lone `AKIA…EXAMPLE` public identifier (not a secret).
- `assets/logo.png` (`Binary`) and `package-lock.json` (`Generated`) keep
  their own reasons.

Policy cases live in scanner unit tests (temp dirs): `exclude_secrets`
true/false, `extra_excludes` hard policy vs `.gitignore` whitelist, invalid
patterns, `TooLarge` before full read, symlink non-following, canary
sanitization (`CANARY…` never in serialized JSON/warnings), determinism.
