# Contributing

Thanks for considering a contribution. This project aims to become a local-first context layer for coding agents; the foundation phase values **small, correct, documented** changes over large features.

## Vision (one paragraph)

We build `repository → context`: secure scanning, structural parsing, persistent local index, hybrid retrieval, budgeted context packages, consumed via CLI / JSON / MCP. See `ROADMAP.md` for phases and `ARCHITECTURE.md` for Current vs Planned.

## Setup

Requirements: Rust stable ≥ 1.97 (2024 edition), Git.

```sh
cargo --version
cargo build
cargo test
```

No Node, Python or database required to build or run the binary.

## Quality gates (must pass)

```sh
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

- Keep modules small and single-responsibility; CLI stays thin.
- Add unit tests next to the behavior; add integration tests under `tests/` for public flows.
- Update docs when public behavior changes; add an ADR under `docs/decisions/` for structural decisions.
- Never commit secrets, `.env` contents, private paths or model downloads.

## Commits

Conventional, short scope prefix encouraged:

```text
feat(cli): add status --json payload
fix(scanner): skip .git on Windows junctions
docs: clarify budget estimator heuristic
test(core): cover unknown config version
```

## Issues / PRs

- Bug reports: OS, version (`aicc version`), repro steps, expected vs observed, redacted logs (no secrets).
- Feature requests: problem, proposal, benefit, alternatives, impact on core vs adapter.
- PRs: link the roadmap phase, describe tests, note docs/ADR updates. CI must be green.

## Security

Do not open public issues for vulnerabilities. See `SECURITY.md`.
