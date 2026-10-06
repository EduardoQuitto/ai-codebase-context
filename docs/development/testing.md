# Testing strategy

Layers (ROADMAP §55):

- **Unit**: config parsing/validation, discovery, classification, language detection, graph navigation, budget selection, schema gates. Live next to each crate (`cargo test -p <crate>`).
- **Integration** (`tests/`): binary contract via `assert_cmd` — help, version, init/status/config flows, `--json` stdout purity (logs on stderr only).
- **Fixtures** (`tests/fixtures/`): small deterministic repos (source + docs + ignored + binary + secret + large file) asserting scanner inclusion/exclusion.
- **E2E / snapshots / benches**: skeletons now (`benches/` compiles, snapshot dir reserved); real FTS/hybrid/retrieval benchmarks arrive with Fase 23–24 and must be reproducible.

Gates: `cargo fmt --check`, `cargo check`, `cargo clippy -- -D warnings`, `cargo test --workspace` — all green on ubuntu/windows/macos.
