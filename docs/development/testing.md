# Testing strategy

Layers (ROADMAP §55):

- **Unit**: config parsing/validation, discovery, classification, language detection, graph navigation, budget selection, schema gates. Live next to each crate (`cargo test -p <crate>`).
- **Integration** (`tests/`): binary contract via `assert_cmd` — help, version, init/status/config flows, `--json` stdout purity (logs on stderr only).
- **Fixtures** (`tests/fixtures/`): small deterministic repos (source + docs + ignored + binary + secret + large file) asserting scanner inclusion/exclusion.
- **E2E / snapshots / benches**: skeletons now (`benches/` compiles, snapshot dir reserved); real FTS/hybrid/retrieval benchmarks arrive with Fase 23–24 and must be reproducible.
- **Fase 5 (security)**: synthetic `FAKE…TESTONLY` positives (PEM, provider tokens, DB URLs, assignments, late secret), placeholder/public-id negatives, `exclude_secrets` true/false, hard `extra_excludes` vs `.gitignore` whitelist, canary absence in JSON/warnings/`Debug`.
- **Fase 7 (structural parsing)**: synthetic `tests/fixtures/parser/` per language (`sample.ts` with functions/generator/class/interface/type/enum/const/arrow/imports/named+default+star exports, `sample.tsx` for TSX grammar selection, `sample.js`/`sample.jsx`, `sample.py` with decorated/multiline code, `broken.py` + `partial.rs` for error recovery); hierarchy/positions/IDs, duplicate scopes, comments-vs-code, empty files, `ERROR`/`MISSING` diagnostics without code text, secret-like import redaction, Go-unsupported, JSON without raw content. No Fase 5/6 test removed.
- **Fase 6 (language + metadata)**: full priority/second-wave registry incl. `.cxx`/`.hh`/`.hxx`, case-insensitivity, `.h`→C fallback, `Unknown` semantics, `Language::id()` == JSON serialization (`has_structural_parser()` was false then; Fase 7 flipped the four implemented languages); metadata size/hash (`blake3-v1`)/line-count rules (empty 0, unterminated last line, CRLF), admitted-vs-excluded coherence, no invented content metadata, no raw content in JSON/`Debug`.

Gates: `cargo fmt --check`, `cargo check`, `cargo clippy -- -D warnings`, `cargo test --workspace` — all green on ubuntu/windows/macos.
